#pragma once

#include <thread>
#include <future>
#include <memory>
#include <chrono>
#include <stdexcept>
#include <vector>
#include <functional>
#include "traits.hpp"

namespace rusty::thread {

// ============================================================================
// JoinHandle - Rust-style: detaches on drop if not joined
// ============================================================================

template<typename T>
class JoinHandle {
private:
    std::thread thread_;
    std::shared_future<T> future_;
    bool joined_ = false;

public:
    JoinHandle(std::thread&& t, std::future<T>&& f)
        : thread_(std::move(t))
        , future_(std::move(f).share())
    {}

    // Block until thread completes and return result (consumes handle)
    T join() {
        if (joined_) {
            throw std::runtime_error("Thread already joined");
        }
        if (!thread_.joinable()) {
            throw std::runtime_error("Thread not joinable");
        }

        thread_.join();
        joined_ = true;

        // This will propagate any exception thrown in the thread
        return future_.get();
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
    std::thread thread_;
    std::shared_future<void> future_;
    bool joined_ = false;

public:
    JoinHandle(std::thread&& t, std::future<void>&& f)
        : thread_(std::move(t))
        , future_(std::move(f).share())
    {}

    void join() {
        if (joined_) {
            throw std::runtime_error("Thread already joined");
        }
        if (!thread_.joinable()) {
            throw std::runtime_error("Thread not joinable");
        }

        thread_.join();
        joined_ = true;
        future_.get();  // Propagate exceptions
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

template<typename F, typename... Args,
         typename = std::enable_if_t<(Send<std::decay_t<Args>> && ...) &&
                                      std::is_invocable_v<F, Args...>>>
auto spawn(F&& func, Args&&... args) -> JoinHandle<std::invoke_result_t<F, Args...>> {
    using ReturnType = std::invoke_result_t<F, Args...>;

    // Package task with arguments captured (C++17 compatible)
    auto task = std::make_shared<std::packaged_task<ReturnType()>>(
        [func = std::forward<F>(func),
         args_tuple = std::make_tuple(std::forward<Args>(args)...)]() mutable -> ReturnType {
            return std::apply([&func](auto&&... args) {
                return std::invoke(func, std::forward<decltype(args)>(args)...);
            }, std::move(args_tuple));
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
    template<typename Fn, typename... Args,
             typename = std::enable_if_t<std::is_invocable_v<Fn, Args...>>>
    void spawn(Fn&& fn, Args&&... args) {
        std::thread t([fn = std::forward<Fn>(fn),
                      args_tuple = std::make_tuple(std::forward<Args>(args)...)]() mutable {
            std::apply([&fn](auto&&... args) {
                std::invoke(fn, std::forward<decltype(args)>(args)...);
            }, std::move(args_tuple));
        });

        threads_.emplace_back(std::move(t));
    }

    // Destructor joins all threads (blocks until all complete)
    ~Scope() = default;
};

// ============================================================================
// scope() - Scoped threads with guaranteed joining
// ============================================================================

template<typename F,
         typename = std::enable_if_t<std::is_invocable_v<F, Scope&>>>
void scope(F&& func) {
    Scope s;
    func(s);
    // s destructor blocks until all threads complete
}

} // namespace rusty::thread
