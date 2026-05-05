#ifndef RUSTY_ASYNC_HPP
#define RUSTY_ASYNC_HPP

// Rust-like async runtime on C++20 coroutines.
// Implements a pollable state machine model matching Rust's Future trait.
//
// Architecture:
//   - Poll<T>: Ready/Pending result (like Rust's Poll enum)
//   - Waker/Context: Notification mechanism for IO readiness
//   - Task<T>: Lazy coroutine future with poll() method
//   - Executor: Event loop that drives tasks to completion
//
// Key design: initial_suspend = suspend_always → lazy semantics (like Rust)

#include <coroutine>
#include <functional>
#include <queue>
#include <thread>
#include <utility>
#include <rusty/vec.hpp>

namespace rusty {

// ── Poll<T>: Rust's Poll enum ──────────────────────────────────
template<typename T>
struct Poll {
    bool ready;
    T value;

    static Poll ready_with(T v) { return Poll{true, std::move(v)}; }
    static Poll pending() { return Poll{false, T{}}; }
    bool is_ready() const { return ready; }
    bool is_pending() const { return !ready; }
};

template<>
struct Poll<void> {
    bool ready;
    static Poll ready_with() { return Poll{true}; }
    static Poll pending() { return Poll{false}; }
    bool is_ready() const { return ready; }
    bool is_pending() const { return !ready; }
};

// ── Waker + Context: notification mechanism ────────────────────
struct Waker {
    std::function<void()> wake_fn;
    void wake() const { if (wake_fn) wake_fn(); }
};

struct Context {
    Waker* waker;
};

// Thread-local context pointer set while a Task is being polled.
// Awaiters can use this to register wake-ups instead of directly resuming handles.
inline thread_local Context* current_context_tls = nullptr;

inline Context* current_context() {
    return current_context_tls;
}

// ── Task<T>: lazy coroutine future ─────────────────────────────
template<typename T>
class Task {
public:
    struct promise_type {
        T result{};
        Context* current_ctx = nullptr;
        std::coroutine_handle<> continuation{};

        Task get_return_object() {
            return Task{std::coroutine_handle<promise_type>::from_promise(*this)};
        }

        // KEY: suspend_always makes it LAZY — nothing runs until poll()
        std::suspend_always initial_suspend() { return {}; }
        auto final_suspend() noexcept {
            struct FinalAwaiter {
                bool await_ready() noexcept { return false; }

                void await_suspend(std::coroutine_handle<promise_type> h) noexcept {
                    auto continuation = h.promise().continuation;
                    if (continuation) {
                        continuation.resume();
                    }
                }

                void await_resume() noexcept {}
            };
            return FinalAwaiter{};
        }

        void return_value(T value) { result = std::move(value); }
        void unhandled_exception() { std::terminate(); }
    };

    Poll<T> poll(Context& cx) {
        if (!handle_) {
            return Poll<T>::ready_with(T{});
        }
        if (handle_.done()) {
            return Poll<T>::ready_with(std::move(handle_.promise().result));
        }
        handle_.promise().current_ctx = &cx;
        Context* prev_ctx = current_context_tls;
        current_context_tls = &cx;
        handle_.resume();
        current_context_tls = prev_ctx;
        if (handle_.done()) {
            return Poll<T>::ready_with(std::move(handle_.promise().result));
        }
        return Poll<T>::pending();
    }

    // Awaiter support: makes Task<T> co_await-able
    bool await_ready() const { return handle_.done(); }
    void await_suspend(std::coroutine_handle<> caller) {
        handle_.promise().continuation = caller;
    }
    T await_resume() { return std::move(handle_.promise().result); }

    ~Task() { if (handle_) handle_.destroy(); }
    Task(Task&& o) noexcept : handle_(std::exchange(o.handle_, nullptr)) {}
    Task& operator=(Task&& o) noexcept {
        if (this != &o) {
            if (handle_) handle_.destroy();
            handle_ = std::exchange(o.handle_, nullptr);
        }
        return *this;
    }
    Task(const Task&) = delete;
    Task& operator=(const Task&) = delete;

private:
    explicit Task(std::coroutine_handle<promise_type> h) : handle_(h) {}
    std::coroutine_handle<promise_type> handle_;
};

// Specialization for Task<void>
template<>
class Task<void> {
public:
    struct promise_type {
        Context* current_ctx = nullptr;
        std::coroutine_handle<> continuation{};

        Task get_return_object() {
            return Task{std::coroutine_handle<promise_type>::from_promise(*this)};
        }

        std::suspend_always initial_suspend() { return {}; }
        auto final_suspend() noexcept {
            struct FinalAwaiter {
                bool await_ready() noexcept { return false; }

                void await_suspend(std::coroutine_handle<promise_type> h) noexcept {
                    auto continuation = h.promise().continuation;
                    if (continuation) {
                        continuation.resume();
                    }
                }

                void await_resume() noexcept {}
            };
            return FinalAwaiter{};
        }

        void return_void() {}
        void unhandled_exception() { std::terminate(); }
    };

    Poll<void> poll(Context& cx) {
        if (!handle_ || handle_.done()) {
            return Poll<void>::ready_with();
        }
        handle_.promise().current_ctx = &cx;
        Context* prev_ctx = current_context_tls;
        current_context_tls = &cx;
        handle_.resume();
        current_context_tls = prev_ctx;
        if (handle_.done()) {
            return Poll<void>::ready_with();
        }
        return Poll<void>::pending();
    }

    bool await_ready() const { return handle_.done(); }
    void await_suspend(std::coroutine_handle<> caller) { handle_.promise().continuation = caller; }
    void await_resume() {}

    ~Task() { if (handle_) handle_.destroy(); }
    Task(Task&& o) noexcept : handle_(std::exchange(o.handle_, nullptr)) {}
    Task(const Task&) = delete;

private:
    explicit Task(std::coroutine_handle<promise_type> h) : handle_(h) {}
    std::coroutine_handle<promise_type> handle_;
};

// Block current thread until a poll-based future completes.
// Supports both direct pollables and Rust-expanded shapes that use
// `into_future()` + `new_unchecked()` + `as_mut().poll(...)`.
template<typename FutureLike>
auto block_on(FutureLike&& future_like) {
    auto future = [&]() {
        if constexpr (requires { std::forward<FutureLike>(future_like).into_future(); }) {
            return std::forward<FutureLike>(future_like).into_future();
        } else {
            return std::forward<FutureLike>(future_like);
        }
    }();

    Waker waker{[]() {}};
    Context context{&waker};

    if constexpr (requires { future.new_unchecked(); }) {
        auto pinned = future.new_unchecked();
        while (true) {
            auto polled = [&]() -> decltype(auto) {
                if constexpr (requires { pinned.as_mut().poll(context); }) {
                    return pinned.as_mut().poll(context);
                } else {
                    return pinned.poll(context);
                }
            }();
            if (polled.is_ready()) {
                if constexpr (requires { polled.value; }) {
                    return std::move(polled.value);
                } else {
                    return;
                }
            }
            std::this_thread::yield();
        }
    } else {
        while (true) {
            auto polled = [&]() -> decltype(auto) {
                if constexpr (requires { future.as_mut().poll(context); }) {
                    return future.as_mut().poll(context);
                } else {
                    return future.poll(context);
                }
            }();
            if (polled.is_ready()) {
                if constexpr (requires { polled.value; }) {
                    return std::move(polled.value);
                } else {
                    return;
                }
            }
            std::this_thread::yield();
        }
    }
}

// ── Executor: event loop ───────────────────────────────────────
class Executor {
public:
    void spawn(Task<void> task) {
        tasks_.push(std::move(task));
        ready_queue_.push(tasks_.len() - 1);
    }

    void run() {
        while (!ready_queue_.empty()) {
            auto idx = ready_queue_.front();
            ready_queue_.pop();

            Waker waker{[this, idx]() { ready_queue_.push(idx); }};
            Context cx{&waker};

            auto result = tasks_[idx].poll(cx);
            // If Pending, waker will re-enqueue when IO fires
            // If Ready, task is done
        }
    }

private:
    rusty::Vec<Task<void>> tasks_;
    std::queue<size_t> ready_queue_;
};

} // namespace rusty

#endif // RUSTY_ASYNC_HPP
