#pragma once

#include <thread>
#include <future>
#include <memory>
#include <concepts>
#include <chrono>
#include <condition_variable>
#include <mutex>
#include <stdexcept>
#include <vector>
#include <functional>
#include <exception>
#include "result.hpp"
#include "traits.hpp"

namespace rusty::thread {

namespace detail {
struct ParkToken {
    std::mutex mutex;
    std::condition_variable cv;
    bool notified = false;
};

inline std::shared_ptr<ParkToken> current_park_token() {
    thread_local std::shared_ptr<ParkToken> token = std::make_shared<ParkToken>();
    return token;
}
} // namespace detail

class Thread {
private:
    std::shared_ptr<detail::ParkToken> token_;

    explicit Thread(std::shared_ptr<detail::ParkToken> token)
        : token_(std::move(token)) {}

public:
    Thread()
        : token_(detail::current_park_token()) {}

    static Thread current() {
        return Thread(detail::current_park_token());
    }

    void unpark() const {
        if (!token_) {
            return;
        }
        std::lock_guard<std::mutex> lock(token_->mutex);
        token_->notified = true;
        token_->cv.notify_one();
    }
};

inline Thread current() {
    return Thread::current();
}

inline void park() {
    auto token = detail::current_park_token();
    std::unique_lock<std::mutex> lock(token->mutex);
    token->cv.wait(lock, [&]() { return token->notified; });
    token->notified = false;
}

inline void yield_now() {
    std::this_thread::yield();
}

// ============================================================================
// JoinHandle - Rust-style: detaches on drop if not joined
// ============================================================================

template<typename T>
class JoinHandle {
private:
    mutable std::thread thread_;
    mutable std::shared_future<T> future_;
    mutable bool joined_ = false;

public:
    JoinHandle(std::thread&& t, std::future<T>&& f)
        : thread_(std::move(t))
        , future_(std::move(f).share())
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

        try {
            return rusty::Result<T, std::exception_ptr>::Ok(future_.get());
        } catch (...) {
            return rusty::Result<T, std::exception_ptr>::Err(std::current_exception());
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
        return future_.wait_for(std::chrono::seconds(0)) ==
               std::future_status::ready;
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

// Specialization for void return type
template<>
class JoinHandle<void> {
private:
    mutable std::thread thread_;
    mutable std::shared_future<void> future_;
    mutable bool joined_ = false;

public:
    JoinHandle(std::thread&& t, std::future<void>&& f)
        : thread_(std::move(t))
        , future_(std::move(f).share())
    {}

    rusty::Result<void, std::exception_ptr> join() const {
        if (joined_) {
            return rusty::Result<void, std::exception_ptr>::Err(
                std::make_exception_ptr(std::runtime_error("Thread already joined"))
            );
        }
        if (!thread_.joinable()) {
            return rusty::Result<void, std::exception_ptr>::Err(
                std::make_exception_ptr(std::runtime_error("Thread not joinable"))
            );
        }

        thread_.join();
        joined_ = true;
        try {
            future_.get();
            return rusty::Result<void, std::exception_ptr>::Ok();
        } catch (...) {
            return rusty::Result<void, std::exception_ptr>::Err(std::current_exception());
        }
    }

    void detach() {
        if (joined_) {
            throw std::runtime_error("Thread already joined");
        }
        if (thread_.joinable()) {
            thread_.detach();
        }
    }

    [[nodiscard]] bool is_finished() const {
        return future_.wait_for(std::chrono::seconds(0)) ==
               std::future_status::ready;
    }

    [[nodiscard]] bool joinable() const {
        return thread_.joinable() && !joined_;
    }

    JoinHandle(const JoinHandle&) = delete;
    JoinHandle& operator=(const JoinHandle&) = delete;
    JoinHandle(JoinHandle&&) = default;
    JoinHandle& operator=(JoinHandle&&) = default;

    ~JoinHandle() {
        if (thread_.joinable() && !joined_) {
            thread_.detach();
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

    // Package task with arguments captured
    auto task = std::make_shared<std::packaged_task<ReturnType()>>(
        [func = std::forward<F>(func),
         ...args = std::forward<Args>(args)]() mutable -> ReturnType {
            return std::invoke(func, std::move(args)...);
        }
    );

    auto future = task->get_future();

    // Launch thread with std::thread (not jthread)
    std::thread thread([task = std::move(task)]() {
        (*task)();
    });

    return JoinHandle<ReturnType>(std::move(thread), std::move(future));
}

// ============================================================================
// Scope - Helper class for scoped threads
// ============================================================================

class Scope {
private:
    struct ScopedThread {
        std::thread thread_;

        ScopedThread(std::thread&& t) : thread_(std::move(t)) {}

        // Must join in destructor (scoped threads are NOT detached)
        ~ScopedThread() {
            if (thread_.joinable()) {
                thread_.join();
            }
        }

        ScopedThread(const ScopedThread&) = delete;
        ScopedThread(ScopedThread&&) = default;
    };

    std::vector<ScopedThread> threads_;

public:
    // Spawn thread within scope - NO Send requirement (lifetime guaranteed)
    template<typename Fn, typename... Args>
        requires std::invocable<Fn, Args...>
    void spawn(Fn&& fn, Args&&... args) {
        std::thread t([fn = std::forward<Fn>(fn),
                      ...args = std::forward<Args>(args)]() mutable {
            std::invoke(fn, std::forward<Args>(args)...);
        });

        threads_.emplace_back(std::move(t));
    }

    // Destructor joins all threads (blocks until all complete)
    ~Scope() = default;
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
    std::this_thread::sleep_for(duration);
}

/// Convenience overload accepting a raw seconds count.
inline void sleep(unsigned long secs) {
    std::this_thread::sleep_for(std::chrono::seconds(secs));
}

} // namespace rusty::thread
