#pragma once

#include <memory>
#include <concepts>
#include <chrono>
#include <stdexcept>
#include <vector>
#include <functional>
#include <exception>
#include <optional>
#include <utility>
#include "platform/threading.hpp"
#include "result.hpp"
#include "traits.hpp"

// rusty::thread — Rust-style threading primitives.
//
// Implementation note: this header used to depend on `<future>` for
// `std::packaged_task<T>` / `std::shared_future<T>`. That dependency was
// removed because libstdc++14 + clang19 + C++20 modules has a bug in
// `<future>`'s internal `_Task_state` hierarchy that triggers when
// non-trivial types from an imported module appear in `std::async` /
// `std::packaged_task` template arguments (historically blocker 4 of
// the rusty::Arc retire). The replacement is a hand-rolled state holder
// (mutex + condvar + Option<T> / exception_ptr) wired via shared_ptr —
// identical observable behavior, no `<future>` in the include graph.

namespace rusty::thread {

namespace detail {
struct ParkToken {
    platform::threading::mutex mutex;
    platform::threading::condition_variable cv;
    bool notified = false;
};

inline std::shared_ptr<ParkToken> current_park_token() {
    thread_local std::shared_ptr<ParkToken> token = std::make_shared<ParkToken>();
    return token;
}

// Shared state holder for a spawned thread's result. Replaces
// `std::shared_future<T>` / `std::packaged_task<T>` — see top-of-file
// note for the libstdc++14 + modules motivation.
template<typename T>
struct JoinState {
    platform::threading::mutex mutex;
    platform::threading::condition_variable cv;
    bool finished = false;
    std::optional<T> value;
    std::exception_ptr error;
};

template<>
struct JoinState<void> {
    platform::threading::mutex mutex;
    platform::threading::condition_variable cv;
    bool finished = false;
    std::exception_ptr error;
};
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
    std::shared_ptr<detail::ParkToken> token_;
    ThreadId id_;

    explicit Thread(std::shared_ptr<detail::ParkToken> token)
        : token_(std::move(token)), id_(platform::threading::current_thread_id()) {}

public:
    Thread()
        : token_(detail::current_park_token()),
          id_(platform::threading::current_thread_id()) {}

    static Thread current() {
        return Thread(detail::current_park_token());
    }

    /// Get this thread's ID. Maps Rust's Thread::id().
    ThreadId id() const { return id_; }

    void unpark() const {
        if (!token_) {
            return;
        }
        platform::threading::lock_guard<platform::threading::mutex> lock(token_->mutex);
        token_->notified = true;
        token_->cv.notify_one();
    }
};

inline Thread current() {
    return Thread::current();
}

inline void park() {
    auto token = detail::current_park_token();
    platform::threading::unique_lock<platform::threading::mutex> lock(token->mutex);
    token->cv.wait(lock, [&]() { return token->notified; });
    token->notified = false;
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
    std::shared_ptr<detail::JoinState<T>> state_;
    mutable bool joined_ = false;

public:
    JoinHandle(platform::threading::thread&& t,
               std::shared_ptr<detail::JoinState<T>> s)
        : thread_(std::move(t))
        , state_(std::move(s))
    {}

    // Block until thread completes and return a Rust-style Result.
    rusty::Result<T, std::exception_ptr> join() const {
        if (joined_) {
            return rusty::Result<T, std::exception_ptr>::Err(
                std::make_exception_ptr(std::runtime_error("Thread already joined"))
            );
        }
        if (!thread_.joinable()) {
            return rusty::Result<T, std::exception_ptr>::Err(
                std::make_exception_ptr(std::runtime_error("Thread not joinable"))
            );
        }

        thread_.join();
        joined_ = true;

        // After thread.join() returns the thread function has completed,
        // so state_ is fully populated. No additional sync needed.
        if (state_->error) {
            return rusty::Result<T, std::exception_ptr>::Err(state_->error);
        }
        if constexpr (std::is_void_v<T>) {
            return rusty::Result<T, std::exception_ptr>::Ok();
        } else {
            return rusty::Result<T, std::exception_ptr>::Ok(std::move(*state_->value));
        }
    }

    // Explicitly detach the thread (like Rust)
    void detach() {
        if (joined_) {
            throw std::runtime_error("Thread already joined");
        }
        if (thread_.joinable()) {
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

namespace detail {
// Run user function and route result/exception into the shared state,
// then mark finished + notify. Hoisted into a helper so both spawn() and
// Scope::spawn() share the same body.
template<typename T, typename F>
inline void run_into_state(std::shared_ptr<JoinState<T>>& state, F&& body) {
    try {
        if constexpr (std::is_void_v<T>) {
            std::forward<F>(body)();
        } else {
            state->value.emplace(std::forward<F>(body)());
        }
    } catch (...) {
        state->error = std::current_exception();
    }
    {
        platform::threading::lock_guard<platform::threading::mutex> lock(state->mutex);
        state->finished = true;
    }
    state->cv.notify_all();
}
} // namespace detail

template<typename F, typename... Args>
    requires (Send<std::decay_t<Args>> && ...) &&
             std::invocable<F, Args...>
auto spawn(F&& func, Args&&... args) -> JoinHandle<std::invoke_result_t<F, Args...>> {
    using ReturnType = std::invoke_result_t<F, Args...>;

    auto state = std::make_shared<detail::JoinState<ReturnType>>();

    // Type-erase the lambda body via std::function<void()> BEFORE handing
    // it to platform::threading::thread (which on libstdc++ is std::thread).
    // This avoids instantiating std::thread::_State_impl<UserLambdaType>
    // with a transpiled-module type — that instantiation trips a
    // libstdc++14 + clang19 + C++20-modules bug where the derived
    // _State_impl's destructor exception-spec is "more lax than base
    // version". With type-erasure the instantiation is always
    // _State_impl<std::function<void()>>, a stable known-good shape.
    std::function<void()> body =
        [state,
         func = std::forward<F>(func),
         ...args = std::forward<Args>(args)]() mutable {
            auto s = state;
            detail::run_into_state<ReturnType>(s, [&]() {
                return std::invoke(func, std::move(args)...);
            });
        };

    platform::threading::thread thread(std::move(body));

    return JoinHandle<ReturnType>(std::move(thread), std::move(state));
}

// ============================================================================
// Scope - Helper class for scoped threads
// ============================================================================

class Scope {
private:
    struct ScopedThreadBase {
        virtual ~ScopedThreadBase() = default;
        virtual void join_if_needed() = 0;
    };

    template<typename T>
    struct ScopedThreadState final : ScopedThreadBase {
        platform::threading::thread thread_;
        std::shared_ptr<detail::JoinState<T>> state_;
        bool joined_ = false;

        ScopedThreadState(platform::threading::thread&& t,
                          std::shared_ptr<detail::JoinState<T>> s)
            : thread_(std::move(t))
            , state_(std::move(s))
        {}

        rusty::Result<T, std::exception_ptr> join() {
            if (joined_) {
                return rusty::Result<T, std::exception_ptr>::Err(
                    std::make_exception_ptr(std::runtime_error("Thread already joined"))
                );
            }
            if (!thread_.joinable()) {
                return rusty::Result<T, std::exception_ptr>::Err(
                    std::make_exception_ptr(std::runtime_error("Thread not joinable"))
                );
            }
            thread_.join();
            joined_ = true;
            if (state_->error) {
                return rusty::Result<T, std::exception_ptr>::Err(state_->error);
            }
            if constexpr (std::is_void_v<T>) {
                return rusty::Result<T, std::exception_ptr>::Ok();
            } else {
                return rusty::Result<T, std::exception_ptr>::Ok(std::move(*state_->value));
            }
        }

        void join_if_needed() override {
            if (thread_.joinable() && !joined_) {
                thread_.join();
                joined_ = true;
            }
        }
    };

    std::vector<std::shared_ptr<ScopedThreadBase>> threads_;

public:
    template<typename T>
    class ScopedJoinHandle {
    private:
        std::shared_ptr<ScopedThreadState<T>> state_;

    public:
        explicit ScopedJoinHandle(std::shared_ptr<ScopedThreadState<T>> state)
            : state_(std::move(state))
        {}

        rusty::Result<T, std::exception_ptr> join() const {
            if (!state_) {
                return rusty::Result<T, std::exception_ptr>::Err(
                    std::make_exception_ptr(std::runtime_error("Invalid scoped join handle"))
                );
            }
            return state_->join();
        }
    };

    // Spawn thread within scope - NO Send requirement (lifetime guaranteed)
    template<typename Fn, typename... Args>
        requires std::invocable<Fn, Args...>
    auto spawn(Fn&& fn, Args&&... args) -> ScopedJoinHandle<std::invoke_result_t<Fn, Args...>> {
        using ReturnType = std::invoke_result_t<Fn, Args...>;
        auto inner_state = std::make_shared<detail::JoinState<ReturnType>>();
        // Type-erase before reaching std::thread — same libstdc++14
        // + modules workaround as the free `spawn` above.
        std::function<void()> body =
            [inner_state,
             fn = std::forward<Fn>(fn),
             ...args = std::forward<Args>(args)]() mutable {
                auto s = inner_state;
                detail::run_into_state<ReturnType>(s, [&]() {
                    return std::invoke(fn, std::forward<Args>(args)...);
                });
            };
        platform::threading::thread t(std::move(body));
        auto state =
            std::make_shared<ScopedThreadState<ReturnType>>(std::move(t), std::move(inner_state));
        threads_.push_back(state);
        return ScopedJoinHandle<ReturnType>(std::move(state));
    }

    // Destructor joins all threads (blocks until all complete)
    ~Scope() {
        for (auto& state : threads_) {
            if (state) {
                state->join_if_needed();
            }
        }
    }
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
