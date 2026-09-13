#ifndef RUSTY_ASYNC_HPP
#define RUSTY_ASYNC_HPP

// Rust-like async runtime on C++20 coroutines — header subset.
// Implements a pollable state machine model matching Rust's Future trait.
//
// Architecture:
//   - Poll<T>: Ready/Pending result (like Rust's Poll enum)
//   - Waker/Context: Notification mechanism for IO readiness
//   - Task<T>: Lazy coroutine future with poll() method
//   - Executor: Event loop that drives tasks to completion
//
// Key design: initial_suspend = suspend_always → lazy semantics (like Rust)
//
// MIGRATION NOTE: `Executor` lives in module `rusty.async` (file
// include/rusty/async.cppm) because its task storage uses vec_port::Vec
// — a C++20 module that headers cannot `import`. The non-Executor types
// stay here so transpiled prelude code (vec_port/btree_port/...) which
// references `rusty::Poll<T>` / `rusty::Context` keeps compiling.

#include <coroutine>
#include <functional>
#include <memory>
#include <optional>
#include <tuple>
#include <type_traits>
#include <thread>
#include <utility>

namespace rusty {

// A pending poll has no live payload, including for non-default-constructible T.
template<typename T>
struct PollRef {
    T* value;
    bool is_ready() const { return value != nullptr; }
    bool is_pending() const { return value == nullptr; }
    T& unwrap() const { if (!value) std::terminate(); return *value; }
    T& unwrap_mut() const { return unwrap(); }
};

template<typename T>
struct Poll {
    bool ready;
    union { T value; };

    Poll(bool active, T v) : ready(active) {
        if (ready) std::construct_at(&value, std::move(v));
    }
    Poll(const Poll& other) requires std::is_copy_constructible_v<T>
        : ready(other.ready) {
        if (ready) std::construct_at(&value, other.value);
    }
    Poll(Poll&& other) noexcept(std::is_nothrow_move_constructible_v<T>)
        : ready(other.ready) {
        if (ready) std::construct_at(&value, std::move(other.value));
    }
    Poll& operator=(const Poll& other) requires std::is_copy_constructible_v<T> {
        if (this != &other) {
            reset();
            if (other.ready) {
                std::construct_at(&value, other.value);
                ready = true;
            }
        }
        return *this;
    }
    Poll& operator=(Poll&& other) noexcept(std::is_nothrow_move_constructible_v<T>) {
        if (this != &other) {
            reset();
            if (other.ready) {
                std::construct_at(&value, std::move(other.value));
                ready = true;
            }
        }
        return *this;
    }
    ~Poll() { reset(); }
    static Poll ready_with(T v) { return Poll{true, std::move(v)}; }
    static Poll pending() { return Poll{}; }
    bool is_ready() const { return ready; }
    bool is_pending() const { return !ready; }
    T unwrap() {
        if (!ready) std::terminate();
        T result = std::move(value);
        reset();
        return result;
    }
    const T& unwrap() const { if (!ready) std::terminate(); return value; }
    T& unwrap_mut() { if (!ready) std::terminate(); return value; }
    PollRef<T> as_mut() & { return {ready ? &value : nullptr}; }
    PollRef<const T> as_ref() const & { return {ready ? &value : nullptr}; }
private:
    Poll() : ready(false) {}
    void reset() { if (ready) { std::destroy_at(&value); ready = false; } }
};

template<>
struct Poll<void> {
    bool ready;
    static Poll ready_with() { return Poll{true}; }
    static Poll pending() { return Poll{false}; }
    bool is_ready() const { return ready; }
    bool is_pending() const { return !ready; }
    std::tuple<> unwrap() const { if (!ready) std::terminate(); return {}; }
    std::tuple<> unwrap_mut() const { return unwrap(); }
    Poll as_mut() const { return *this; }
    Poll as_ref() const { return *this; }
};

struct Waker {
    std::function<void()> wake_fn;
    std::function<void(const std::function<void()>&)> wake_by_ref_fn{};
    template<typename F>
    static Waker from_callable(F&& callback) {
        return Waker{std::forward<F>(callback)};
    }
    template<typename ArcLike>
    static Waker from_arc(ArcLike arc) {
        using Target = std::remove_cvref_t<decltype(*arc)>;
        struct OwnedWake {
            ArcLike owner;
            void operator()() { Target::wake(std::move(owner)); }
            void by_ref() const {
                if constexpr (requires { Target::wake_by_ref(owner); }) {
                    Target::wake_by_ref(owner);
                } else {
                    Target::wake(owner.clone());
                }
            }
        };
        return Waker{
            OwnedWake{std::move(arc)},
            [](const std::function<void()>& callback) {
                callback.template target<OwnedWake>()->by_ref();
            }
        };
    }
    Waker clone() const { return *this; }
    void wake() const { if (wake_fn) wake_fn(); }
    void wake_by_ref() const {
        if (wake_by_ref_fn) wake_by_ref_fn(wake_fn);
        else wake();
    }
};

struct Context {
    const Waker* waker;
};

inline thread_local Context* current_context_tls = nullptr;
inline Context* current_context() { return current_context_tls; }

namespace async_detail {
// Each coroutine owns the wake context borrowed by its suspended awaiters.
// The child link identifies the suspended leaf that the next poll must resume.
struct TaskContext {
    Waker owned_waker{};
    Context owned_context{&owned_waker};
    Context* current_ctx = &owned_context;
    TaskContext* parent = nullptr;
    TaskContext* child = nullptr;
    Context* continuation_context = nullptr;
    std::coroutine_handle<> continuation{};
    std::coroutine_handle<> self{};

    void refresh(const Context& cx) {
        owned_waker = cx.waker ? *cx.waker : Waker{};
    }
};
inline thread_local TaskContext* current_task_tls = nullptr;

struct ContextScope {
    Context* previous_context = current_context_tls;
    TaskContext* previous_task = current_task_tls;
    ~ContextScope() {
        current_context_tls = previous_context;
        current_task_tls = previous_task;
    }
};

inline void resume(TaskContext& root, const Context& cx) {
    ContextScope restore;
    auto* leaf = &root;
    while (true) {
        leaf->refresh(cx);
        if (!leaf->child) break;
        leaf = leaf->child;
    }
    current_task_tls = leaf;
    current_context_tls = leaf->current_ctx;
    leaf->self.resume();
}

inline std::coroutine_handle<> start_await(TaskContext& task,
                                          std::coroutine_handle<> caller) {
    task.continuation = caller;
    task.continuation_context = current_context_tls;
    task.parent = current_task_tls;
    if (task.parent) task.parent->child = &task;
    if (current_context_tls) task.refresh(*current_context_tls);
    current_task_tls = &task;
    current_context_tls = task.current_ctx;
    return task.self;
}

struct FinalAwaiter {
    bool await_ready() noexcept { return false; }
    template<typename Promise>
    std::coroutine_handle<> await_suspend(std::coroutine_handle<Promise> h) noexcept {
        auto& task = h.promise();
        if (task.parent) task.parent->child = nullptr;
        current_task_tls = task.parent;
        current_context_tls = task.parent ? task.parent->current_ctx
                                         : task.continuation_context;
        return task.continuation ? task.continuation : std::noop_coroutine();
    }
    void await_resume() noexcept {}
};
} // namespace async_detail

template<typename T>
class Task {
public:
    struct promise_type : async_detail::TaskContext {
        std::optional<T> result;
        Task get_return_object() {
            auto handle = std::coroutine_handle<promise_type>::from_promise(*this);
            self = handle;
            return Task{handle};
        }
        std::suspend_always initial_suspend() { return {}; }
        async_detail::FinalAwaiter final_suspend() noexcept { return {}; }
        void return_value(T value) { result.emplace(std::move(value)); }
        void unhandled_exception() { std::terminate(); }
        T take_result() {
            if (!result) std::terminate();
            T value = std::move(*result);
            result.reset();
            return value;
        }
    };

    Poll<T> poll(Context& cx) {
        if (!handle_) std::terminate();
        if (!handle_.done()) async_detail::resume(handle_.promise(), cx);
        if (handle_.done()) return Poll<T>::ready_with(handle_.promise().take_result());
        return Poll<T>::pending();
    }
    bool await_ready() const { return handle_ && handle_.done(); }
    std::coroutine_handle<> await_suspend(std::coroutine_handle<> caller) {
        if (!handle_) std::terminate();
        return async_detail::start_await(handle_.promise(), caller);
    }
    T await_resume() { return handle_.promise().take_result(); }

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

template<>
class Task<void> {
public:
    struct promise_type : async_detail::TaskContext {
        Task get_return_object() {
            auto handle = std::coroutine_handle<promise_type>::from_promise(*this);
            self = handle;
            return Task{handle};
        }
        std::suspend_always initial_suspend() { return {}; }
        async_detail::FinalAwaiter final_suspend() noexcept { return {}; }
        void return_void() {}
        void unhandled_exception() { std::terminate(); }
    };
    Poll<void> poll(Context& cx) {
        if (!handle_) std::terminate();
        if (!handle_.done()) async_detail::resume(handle_.promise(), cx);
        return handle_.done() ? Poll<void>::ready_with() : Poll<void>::pending();
    }
    bool await_ready() const { return handle_ && handle_.done(); }
    std::coroutine_handle<> await_suspend(std::coroutine_handle<> caller) {
        if (!handle_) std::terminate();
        return async_detail::start_await(handle_.promise(), caller);
    }
    void await_resume() {}
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

namespace future {
struct Pending {
    template<typename T> operator Poll<T>() const { return Poll<T>::pending(); }
};
template<typename T>
Task<T> pin(Task<T> task) { return task; }

// The coroutine frame keeps the concrete future at one address across polls.
template<typename T, typename F>
Task<T> pin(F future) {
    while (true) {
        auto result = future.poll(*current_context());
        if (result.is_ready()) {
            if constexpr (std::is_void_v<T>) co_return;
            else co_return result.unwrap();
        }
        co_await std::suspend_always{};
    }
}

template<typename PollType> struct poll_output;
template<typename T> struct poll_output<Poll<T>> { using type = T; };

// Derive Output from the concrete pollable when Rust inferred Box::pin's
// type through a generic call. A caller's generic parameter names are not
// necessarily in scope where that expression is emitted.
template<typename F>
auto pin(F future) -> Task<typename poll_output<decltype(
    std::declval<F&>().poll(std::declval<Context&>()))>::type> {
    using Output = typename poll_output<decltype(
        std::declval<F&>().poll(std::declval<Context&>()))>::type;
    return pin<Output, F>(std::move(future));
}
} // namespace future

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

// NOTE: class Executor lives in the C++20 module `rusty.async` (file
// include/rusty/async.cppm) so its task storage can use vec_port::Vec.
// Header consumers can't access Executor without `import rusty.async;`.

} // namespace rusty

#endif // RUSTY_ASYNC_HPP
