#pragma once

#include <condition_variable>
#include <mutex>
#include <chrono>

namespace rusty {

// Condvar - Condition variable for waiting and notification
// Matches Rust's std::sync::Condvar behavior
//
// Note: Unlike Rust's Condvar which works with Mutex<T>, this implementation
// works directly with std::unique_lock<std::mutex> for simplicity and
// compatibility with standard C++ condition variables.
//
// Usage:
//   std::mutex mtx;
//   Condvar cv;
//   bool ready = false;
//
//   // Thread 1 (waiter)
//   {
//       std::unique_lock lock(mtx);
//       cv.wait(lock, [&]{ return ready; });
//   }
//
//   // Thread 2 (notifier)
//   {
//       std::unique_lock lock(mtx);
//       ready = true;
//       cv.notify_one();
//   }
//
class Condvar {
private:
    std::condition_variable cv_;

public:
    Condvar() = default;

    // Wait on a unique_lock (automatically releases and reacquires lock)
    void wait(std::unique_lock<std::mutex>& lock) {
        cv_.wait(lock);
    }

    // Wait with a predicate (avoids spurious wakeups)
    template<typename Predicate>
    void wait(std::unique_lock<std::mutex>& lock, Predicate pred) {
        cv_.wait(lock, pred);
    }

    // Wait for a duration - returns true if notified, false if timeout
    template<typename Rep, typename Period>
    bool wait_for(
        std::unique_lock<std::mutex>& lock,
        const std::chrono::duration<Rep, Period>& duration
    ) {
        return cv_.wait_for(lock, duration) == std::cv_status::no_timeout;
    }

    // Wait for a duration with predicate - returns value of predicate
    template<typename Rep, typename Period, typename Predicate>
    bool wait_for(
        std::unique_lock<std::mutex>& lock,
        const std::chrono::duration<Rep, Period>& duration,
        Predicate pred
    ) {
        return cv_.wait_for(lock, duration, pred);
    }

    // Wait until a time point - returns true if notified, false if timeout
    template<typename Clock, typename Duration>
    bool wait_until(
        std::unique_lock<std::mutex>& lock,
        const std::chrono::time_point<Clock, Duration>& timeout_time
    ) {
        return cv_.wait_until(lock, timeout_time) == std::cv_status::no_timeout;
    }

    // Wait until a time point with predicate
    template<typename Clock, typename Duration, typename Predicate>
    bool wait_until(
        std::unique_lock<std::mutex>& lock,
        const std::chrono::time_point<Clock, Duration>& timeout_time,
        Predicate pred
    ) {
        return cv_.wait_until(lock, timeout_time, pred);
    }

    // Notify one waiting thread
    void notify_one() {
        cv_.notify_one();
    }

    // Notify all waiting threads
    void notify_all() {
        cv_.notify_all();
    }

    // Non-copyable, non-movable
    Condvar(const Condvar&) = delete;
    Condvar& operator=(const Condvar&) = delete;
    Condvar(Condvar&&) = delete;
    Condvar& operator=(Condvar&&) = delete;

    ~Condvar() = default;
};

} // namespace rusty
