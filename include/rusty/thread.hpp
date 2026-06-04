#pragma once

#include <concepts>
#include <chrono>
#include <utility>
#include <atomic>
#include "platform/threading.hpp"
#include "result.hpp"
#include "option.hpp"
#include "traits.hpp"

// rusty::thread — Rust-style threading primitives.
//
// Implementation note: this header is deliberately std-light. The
// historical "blocker 4" of the rusty::Arc retire was a
// libstdc++14 + clang19 + C++20 modules friction where std-templated
// classes (std::thread::_State_impl, std::future::_Task_state, etc.)
// generated derived destructors whose noexcept-spec was "more lax
// than base version" when instantiated with transpiled-module types.
//
// This header replaces the std types that participated in that bug
// AND the std utility types we'd otherwise carry along with them:
//   - std::function<void()>     → detail::TypeErasedClosure
//   - std::shared_ptr<State>    → detail::SharedState<State>
//   - std::optional<T>          → rusty::Option<T>
//   - std::packaged_task/future → hand-rolled JoinState + mutex/cv
//   - std::exception_ptr        → DROPPED (Rust threads have no
//                                 exceptions; if the user function
//                                 throws, std::terminate fires at the
//                                 std::thread boundary — matching
//                                 Rust's panic=abort default).
//
// The std types kept are intrinsics-grade (std::atomic) and pure
// metaprogramming (std::forward / std::invoke / std::is_void_v). The
// Scope class uses a hand-rolled intrusive list rather than
// std::vector. There is no exception handling in this header: a
// throwing user function will terminate the process (Rust panic-abort
// semantics).

namespace rusty::thread {

// Error variants from `JoinHandle<T>::join()`. Mechanical errors only —
// Rust threads have no exception payload to capture.
enum class JoinError {
    AlreadyJoined,   // join() was already called on this handle
    NotJoinable,     // thread was detached or moved-from
};

namespace detail {

struct ParkToken {
    platform::threading::mutex mutex;
    platform::threading::condition_variable cv;
    bool notified = false;
};

// ──────────────────────────────────────────────────────────────────────
// SharedState<S> — minimal hand-rolled atomic-refcount shared pointer.
// Replaces std::shared_ptr<S>. S must have a `std::atomic<size_t>
// refcount` member that starts at 1 (we don't allocate the control
// block separately; it's intrusive). Stays in headers without dragging
// in <memory>.
// ──────────────────────────────────────────────────────────────────────
template<typename S>
class SharedState {
    S* ptr_ = nullptr;

    explicit SharedState(S* p) noexcept : ptr_(p) {}

public:
    SharedState() noexcept = default;

    template<typename... Args>
    static SharedState make(Args&&... args) {
        return SharedState(new S(std::forward<Args>(args)...));
    }

    SharedState(const SharedState& o) noexcept : ptr_(o.ptr_) {
        if (ptr_) ptr_->refcount.fetch_add(1, std::memory_order_relaxed);
    }
    SharedState(SharedState&& o) noexcept : ptr_(o.ptr_) { o.ptr_ = nullptr; }
    SharedState& operator=(const SharedState& o) noexcept {
        if (this != &o) {
            release();
            ptr_ = o.ptr_;
            if (ptr_) ptr_->refcount.fetch_add(1, std::memory_order_relaxed);
        }
        return *this;
    }
    SharedState& operator=(SharedState&& o) noexcept {
        if (this != &o) {
            release();
            ptr_ = o.ptr_;
            o.ptr_ = nullptr;
        }
        return *this;
    }
    ~SharedState() noexcept { release(); }

    S* operator->() const noexcept { return ptr_; }
    S& operator*() const noexcept { return *ptr_; }
    S* get() const noexcept { return ptr_; }
    explicit operator bool() const noexcept { return ptr_ != nullptr; }

private:
    void release() noexcept {
        if (ptr_ && ptr_->refcount.fetch_sub(1, std::memory_order_acq_rel) == 1) {
            std::atomic_thread_fence(std::memory_order_acquire);
            delete ptr_;
        }
    }
};

// Thread-local park token state. Uses SharedState rather than std::shared_ptr.
struct ParkTokenShared {
    std::atomic<size_t> refcount{1};
    ParkToken inner;
};

inline SharedState<ParkTokenShared> current_park_token() {
    thread_local SharedState<ParkTokenShared> token =
        SharedState<ParkTokenShared>::make();
    return token;
}

// ──────────────────────────────────────────────────────────────────────
// JoinState<T> — result-holder for a spawned thread. Replaces
// `std::shared_future<T>` / `std::packaged_task<T>`. Uses rusty::Option
// instead of std::optional for the value slot.
// ──────────────────────────────────────────────────────────────────────
template<typename T>
struct JoinState {
    std::atomic<size_t> refcount{1};
    platform::threading::mutex mutex;
    platform::threading::condition_variable cv;
    bool finished = false;
    rusty::Option<T> value;
};

template<>
struct JoinState<void> {
    std::atomic<size_t> refcount{1};
    platform::threading::mutex mutex;
    platform::threading::condition_variable cv;
    bool finished = false;
};

// ──────────────────────────────────────────────────────────────────────
// TypeErasedClosure — replaces std::function<void()> for the spawn
// body. Plain function-pointer dispatch (no virtual hierarchy), so
// std::thread's `_State_impl<TypeErasedClosure>` lands in a stable
// noexcept-matching destructor shape and avoids the libstdc++14 +
// modules friction that fires for arbitrary user-lambda payload types.
// ──────────────────────────────────────────────────────────────────────
class TypeErasedClosure {
    void* state_ = nullptr;
    void (*invoke_)(void*) = nullptr;
    void (*destroy_)(void*) noexcept = nullptr;

public:
    TypeErasedClosure() noexcept = default;

    template<typename F>
        requires (!std::is_same_v<std::decay_t<F>, TypeErasedClosure>)
    explicit TypeErasedClosure(F&& f) {
        using FD = std::decay_t<F>;
        state_ = new FD(std::forward<F>(f));
        invoke_ = +[](void* p) { (*static_cast<FD*>(p))(); };
        destroy_ = +[](void* p) noexcept {
            try { delete static_cast<FD*>(p); } catch (...) { /* swallow */ }
        };
    }

    TypeErasedClosure(TypeErasedClosure&& o) noexcept
        : state_(o.state_), invoke_(o.invoke_), destroy_(o.destroy_) {
        o.state_ = nullptr; o.invoke_ = nullptr; o.destroy_ = nullptr;
    }

    TypeErasedClosure& operator=(TypeErasedClosure&& o) noexcept {
        if (this != &o) {
            if (state_) destroy_(state_);
            state_ = o.state_; invoke_ = o.invoke_; destroy_ = o.destroy_;
            o.state_ = nullptr; o.invoke_ = nullptr; o.destroy_ = nullptr;
        }
        return *this;
    }

    TypeErasedClosure(const TypeErasedClosure&) = delete;
    TypeErasedClosure& operator=(const TypeErasedClosure&) = delete;

    ~TypeErasedClosure() noexcept { if (state_) destroy_(state_); }

    void operator()() { if (invoke_) invoke_(state_); }
};

// Run user function and route result into the shared state, then mark
// finished + notify. Hoisted into a helper so spawn() and Scope::spawn()
// share the same body. No try/catch: a throwing user function will
// propagate out of the thread body, std::thread will call
// std::terminate (Rust panic-abort semantics).
template<typename T, typename F>
inline void run_into_state(SharedState<JoinState<T>>& state, F&& body) {
    if constexpr (std::is_void_v<T>) {
        std::forward<F>(body)();
    } else {
        state->value = rusty::Option<T>(std::forward<F>(body)());
    }
    {
        platform::threading::lock_guard<platform::threading::mutex> lock(state->mutex);
        state->finished = true;
    }
    state->cv.notify_all();
}

} // namespace detail

/// Opaque thread identifier.
/// Maps Rust's std::thread::ThreadId.
/// Copyable, comparable, hashable. Wraps backend-native thread identifier.
class ThreadId {
public:
    platform::threading::thread_id inner_{};

    ThreadId() = default;
    explicit ThreadId(platform::threading::thread_id id) : inner_(id) {}

    bool operator==(const ThreadId& other) const {
        return platform::threading::thread_id_equal(inner_, other.inner_);
    }
    bool operator!=(const ThreadId& other) const { return !(*this == other); }
    bool operator<(const ThreadId& other) const  {
        return platform::threading::thread_id_less(inner_, other.inner_);
    }

    platform::threading::thread_id as_native() const { return inner_; }
};

/// Get the current thread's ID.
/// Maps Rust's std::thread::current().id().
inline ThreadId current_id() {
    return ThreadId{platform::threading::current_thread_id()};
}

class Thread {
private:
    detail::SharedState<detail::ParkTokenShared> token_;
    ThreadId id_;

public:
    Thread()
        : token_(detail::current_park_token()),
          id_(platform::threading::current_thread_id()) {}

    static Thread current() {
        return Thread();
    }

    /// Get this thread's ID. Maps Rust's Thread::id().
    ThreadId id() const { return id_; }

    void unpark() const {
        if (!token_) {
            return;
        }
        auto& inner = token_->inner;
        platform::threading::lock_guard<platform::threading::mutex> lock(inner.mutex);
        inner.notified = true;
        inner.cv.notify_one();
    }
};

inline Thread current() {
    return Thread::current();
}

inline void park() {
    auto token = detail::current_park_token();
    auto& inner = token->inner;
    platform::threading::unique_lock<platform::threading::mutex> lock(inner.mutex);
    inner.cv.wait(lock, [&]() { return inner.notified; });
    inner.notified = false;
}

inline void yield_now() {
    platform::threading::yield();
}

// ============================================================================
// JoinHandle - Rust-style: detaches on drop if not joined
// ============================================================================

template<typename T>
class JoinHandle {
private:
    mutable platform::threading::thread thread_;
    detail::SharedState<detail::JoinState<T>> state_;
    mutable bool joined_ = false;

public:
    JoinHandle(platform::threading::thread&& t,
               detail::SharedState<detail::JoinState<T>> s)
        : thread_(std::move(t))
        , state_(std::move(s))
    {}

    // Block until thread completes and return a Rust-style Result.
    rusty::Result<T, JoinError> join() const {
        if (joined_) {
            return rusty::Result<T, JoinError>::Err(JoinError::AlreadyJoined);
        }
        if (!thread_.joinable()) {
            return rusty::Result<T, JoinError>::Err(JoinError::NotJoinable);
        }

        thread_.join();
        joined_ = true;

        // After thread.join() returns the thread function has completed,
        // so state_ is fully populated. No additional sync needed.
        if constexpr (std::is_void_v<T>) {
            return rusty::Result<T, JoinError>::Ok();
        } else {
            return rusty::Result<T, JoinError>::Ok(state_->value.unwrap());
        }
    }

    // Explicitly detach the thread. No-op if already joined or not
    // joinable (matches Rust's drop-detaches semantics — never throws).
    void detach() {
        if (!joined_ && thread_.joinable()) {
            thread_.detach();
        }
    }

    // Check if thread has finished (non-blocking)
    [[nodiscard]] bool is_finished() const {
        platform::threading::lock_guard<platform::threading::mutex> lock(state_->mutex);
        return state_->finished;
    }

    // Check if thread is joinable
    [[nodiscard]] bool joinable() const {
        return thread_.joinable() && !joined_;
    }

    // Non-copyable, movable
    JoinHandle(const JoinHandle&) = delete;
    JoinHandle& operator=(const JoinHandle&) = delete;
    JoinHandle(JoinHandle&&) = default;
    JoinHandle& operator=(JoinHandle&&) = default;

    // Destructor: detach if not joined (RUST SEMANTICS)
    ~JoinHandle() {
        if (thread_.joinable() && !joined_) {
            thread_.detach();  // Detach, don't block
        }
    }
};

// ============================================================================
// spawn() - Launch thread with Send checking
// ============================================================================

template<typename F, typename... Args>
    requires (Send<std::decay_t<Args>> && ...) &&
             std::invocable<F, Args...>
auto spawn(F&& func, Args&&... args) -> JoinHandle<std::invoke_result_t<F, Args...>> {
    using ReturnType = std::invoke_result_t<F, Args...>;

    auto state = detail::SharedState<detail::JoinState<ReturnType>>::make();
    auto thread_state = state;  // bumps refcount for the spawned thread.

    // Type-erase the body via TypeErasedClosure BEFORE handing it to
    // platform::threading::thread (= std::thread). This pins
    // std::thread::_State_impl<F> to F = TypeErasedClosure (a plain
    // function-pointer wrapper with noexcept destructor), avoiding the
    // libstdc++14 + clang19 + C++20-modules destructor-noexcept bug
    // that fires for arbitrary user lambda types that capture
    // transpiled module values.
    detail::TypeErasedClosure body{
        [thread_state = std::move(thread_state),
         func = std::forward<F>(func),
         ...args = std::forward<Args>(args)]() mutable {
            auto s = thread_state;
            detail::run_into_state<ReturnType>(s, [&]() {
                return std::invoke(func, std::move(args)...);
            });
        }
    };

    platform::threading::thread thread(std::move(body));

    return JoinHandle<ReturnType>(std::move(thread), std::move(state));
}

// ============================================================================
// Scope - Helper class for scoped threads
// ============================================================================

class Scope {
private:
    // Intrusive singly-linked list of scoped thread states. Replaces
    // std::vector<std::shared_ptr<ScopedThreadBase>>. Each node is heap-
    // allocated and held by both Scope (which joins on destruction) and
    // ScopedJoinHandle (which calls join() directly) via SharedState.
    struct ScopedThreadBase {
        std::atomic<size_t> refcount{1};
        ScopedThreadBase* next_in_scope = nullptr;
        virtual ~ScopedThreadBase() = default;
        virtual void join_if_needed() = 0;
    };

    template<typename T>
    struct ScopedThreadState final : ScopedThreadBase {
        platform::threading::thread thread_;
        detail::SharedState<detail::JoinState<T>> state_;
        bool joined_ = false;

        ScopedThreadState(platform::threading::thread&& t,
                          detail::SharedState<detail::JoinState<T>> s)
            : thread_(std::move(t))
            , state_(std::move(s))
        {}

        rusty::Result<T, JoinError> join() {
            if (joined_) {
                return rusty::Result<T, JoinError>::Err(JoinError::AlreadyJoined);
            }
            if (!thread_.joinable()) {
                return rusty::Result<T, JoinError>::Err(JoinError::NotJoinable);
            }
            thread_.join();
            joined_ = true;
            if constexpr (std::is_void_v<T>) {
                return rusty::Result<T, JoinError>::Ok();
            } else {
                return rusty::Result<T, JoinError>::Ok(state_->value.unwrap());
            }
        }

        void join_if_needed() override {
            if (thread_.joinable() && !joined_) {
                thread_.join();
                joined_ = true;
            }
        }
    };

    ScopedThreadBase* head_ = nullptr;

public:
    template<typename T>
    class ScopedJoinHandle {
    private:
        detail::SharedState<ScopedThreadState<T>> state_;

    public:
        explicit ScopedJoinHandle(detail::SharedState<ScopedThreadState<T>> state)
            : state_(std::move(state))
        {}

        rusty::Result<T, JoinError> join() const {
            if (!state_) {
                return rusty::Result<T, JoinError>::Err(JoinError::NotJoinable);
            }
            return state_->join();
        }
    };

    // Spawn thread within scope - NO Send requirement (lifetime guaranteed)
    template<typename Fn, typename... Args>
        requires std::invocable<Fn, Args...>
    auto spawn(Fn&& fn, Args&&... args) -> ScopedJoinHandle<std::invoke_result_t<Fn, Args...>> {
        using ReturnType = std::invoke_result_t<Fn, Args...>;
        auto inner_state = detail::SharedState<detail::JoinState<ReturnType>>::make();
        auto thread_state = inner_state;

        detail::TypeErasedClosure body{
            [thread_state = std::move(thread_state),
             fn = std::forward<Fn>(fn),
             ...args = std::forward<Args>(args)]() mutable {
                auto s = thread_state;
                detail::run_into_state<ReturnType>(s, [&]() {
                    return std::invoke(fn, std::forward<Args>(args)...);
                });
            }};
        platform::threading::thread t(std::move(body));

        auto state = detail::SharedState<ScopedThreadState<ReturnType>>::make(
            std::move(t), std::move(inner_state));
        // Link into Scope's intrusive list. Bump refcount because the
        // Scope holds a non-SharedState raw pointer (it'll iterate and
        // not own destruction — destruction goes via SharedState).
        ScopedThreadState<ReturnType>* raw = state.get();
        raw->refcount.fetch_add(1, std::memory_order_relaxed);
        raw->next_in_scope = head_;
        head_ = raw;

        return ScopedJoinHandle<ReturnType>(std::move(state));
    }

    // Destructor joins all threads (blocks until all complete)
    ~Scope() {
        // Walk the intrusive list, join each, then release the Scope's
        // refcount.
        ScopedThreadBase* cur = head_;
        while (cur) {
            ScopedThreadBase* next = cur->next_in_scope;
            cur->join_if_needed();
            // Drop the refcount we took at spawn(). Mirrors the
            // SharedState::release() shape but inline since we hold
            // a raw pointer here.
            if (cur->refcount.fetch_sub(1, std::memory_order_acq_rel) == 1) {
                std::atomic_thread_fence(std::memory_order_acquire);
                delete cur;
            }
            cur = next;
        }
        head_ = nullptr;
    }

    Scope() = default;
    Scope(const Scope&) = delete;
    Scope(Scope&&) = delete;
    Scope& operator=(const Scope&) = delete;
    Scope& operator=(Scope&&) = delete;
};

// ============================================================================
// scope() - Scoped threads with guaranteed joining
// ============================================================================

template<typename F>
    requires std::invocable<F, Scope&>
void scope(F&& func) {
    Scope s;
    func(s);
    // s destructor blocks until all threads complete
}

/// Sleep the current thread for the specified duration.
/// Maps Rust's std::thread::sleep(Duration).
template<typename Rep, typename Period>
inline void sleep(const std::chrono::duration<Rep, Period>& duration) {
    platform::threading::sleep_for(duration);
}

template<typename DurationLike>
    requires requires(const DurationLike& d) { d.inner; }
inline void sleep(const DurationLike& duration) {
    platform::threading::sleep_for(duration.inner);
}

/// Convenience overload accepting a raw seconds count.
inline void sleep(unsigned long secs) {
    platform::threading::sleep_for(std::chrono::seconds(secs));
}

} // namespace rusty::thread

namespace std {
template <>
struct hash<rusty::thread::ThreadId> {
    size_t operator()(const rusty::thread::ThreadId& id) const noexcept {
        return rusty::platform::threading::thread_id_hash(id.inner_);
    }
};
} // namespace std
