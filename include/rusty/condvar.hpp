#pragma once

#include <condition_variable>
#include <mutex>
#include <chrono>
#include "mutex.hpp"

namespace rusty {

// @safe
// Condvar - Condition variable for waiting and notification
// Similar to Rust's std::sync::Condvar
//
// Works with rusty::MutexGuard<T> (preferred) or std::unique_lock<std::mutex>
//
// Usage with Mutex<T> (Rust-like pattern):
//   Mutex<bool> ready(false);
//   Condvar cv;
//
//   // Thread 1 (waiter)
//   {
//       auto guard = ready.lock();
//       cv.wait(guard, [&]{ return *guard; });
//   }
//
//   // Thread 2 (notifier)
//   {
//       auto guard = ready.lock();
//       *guard = true;
//       cv.notify_one();
//   }
//
class Condvar {
private:
    std::condition_variable cv_;

public:
    // @safe - Default constructor
    Condvar() = default;

    // =========================================================================
    // MutexGuard<T> overloads (Rust-like API)
    // =========================================================================

    // @safe - Wait on a MutexGuard (automatically releases and reacquires lock)
    template<typename T>
    void wait(MutexGuard<T>& guard) {
        cv_.wait(guard.underlying_lock());
    }

    // @safe - Wait with a predicate (avoids spurious wakeups)
    template<typename T, typename Predicate>
    void wait(MutexGuard<T>& guard, Predicate pred) {
        cv_.wait(guard.underlying_lock(), pred);
    }

    // @safe - Wait for a duration - returns true if notified, false if timeout
    template<typename T, typename Rep, typename Period>
    bool wait_for(
        MutexGuard<T>& guard,
        const std::chrono::duration<Rep, Period>& duration
    ) {
        return cv_.wait_for(guard.underlying_lock(), duration) == std::cv_status::no_timeout;
    }

    // @safe - Wait for a duration with predicate - returns value of predicate
    template<typename T, typename Rep, typename Period, typename Predicate>
    bool wait_for(
        MutexGuard<T>& guard,
        const std::chrono::duration<Rep, Period>& duration,
        Predicate pred
    ) {
        return cv_.wait_for(guard.underlying_lock(), duration, pred);
    }

    // @safe - Wait until a time point - returns true if notified, false if timeout
    template<typename T, typename Clock, typename Duration>
    bool wait_until(
        MutexGuard<T>& guard,
        const std::chrono::time_point<Clock, Duration>& timeout_time
    ) {
        return cv_.wait_until(guard.underlying_lock(), timeout_time) == std::cv_status::no_timeout;
    }

    // @safe - Wait until a time point with predicate
    template<typename T, typename Clock, typename Duration, typename Predicate>
    bool wait_until(
        MutexGuard<T>& guard,
        const std::chrono::time_point<Clock, Duration>& timeout_time,
        Predicate pred
    ) {
        return cv_.wait_until(guard.underlying_lock(), timeout_time, pred);
    }

    // =========================================================================
    // std::unique_lock overloads (C++ compatibility, kept for backward compat)
    // =========================================================================

    // @safe - Wait on a unique_lock (automatically releases and reacquires lock)
    void wait(std::unique_lock<std::mutex>& lock) {
        cv_.wait(lock);
    }

    // @safe - Wait with a predicate (avoids spurious wakeups)
    template<typename Predicate>
    void wait(std::unique_lock<std::mutex>& lock, Predicate pred) {
        cv_.wait(lock, pred);
    }

    // @safe - Wait for a duration - returns true if notified, false if timeout
    template<typename Rep, typename Period>
    bool wait_for(
        std::unique_lock<std::mutex>& lock,
        const std::chrono::duration<Rep, Period>& duration
    ) {
        return cv_.wait_for(lock, duration) == std::cv_status::no_timeout;
    }

    // @safe - Wait for a duration with predicate - returns value of predicate
    template<typename Rep, typename Period, typename Predicate>
    bool wait_for(
        std::unique_lock<std::mutex>& lock,
        const std::chrono::duration<Rep, Period>& duration,
        Predicate pred
    ) {
        return cv_.wait_for(lock, duration, pred);
    }

    // @safe - Wait until a time point - returns true if notified, false if timeout
    template<typename Clock, typename Duration>
    bool wait_until(
        std::unique_lock<std::mutex>& lock,
        const std::chrono::time_point<Clock, Duration>& timeout_time
    ) {
        return cv_.wait_until(lock, timeout_time) == std::cv_status::no_timeout;
    }

    // @safe - Wait until a time point with predicate
    template<typename Clock, typename Duration, typename Predicate>
    bool wait_until(
        std::unique_lock<std::mutex>& lock,
        const std::chrono::time_point<Clock, Duration>& timeout_time,
        Predicate pred
    ) {
        return cv_.wait_until(lock, timeout_time, pred);
    }

    // =========================================================================
    // Notification methods
    // =========================================================================

    // @safe - Notify one waiting thread
    void notify_one() {
        cv_.notify_one();
    }

    // @safe - Notify all waiting threads
    void notify_all() {
        cv_.notify_all();
    }

    // Non-copyable, non-movable
    Condvar(const Condvar&) = delete;
    Condvar& operator=(const Condvar&) = delete;
    Condvar(Condvar&&) = delete;
    Condvar& operator=(Condvar&&) = delete;

    // @safe - RAII destructor
    ~Condvar() = default;
};

} // namespace rusty
