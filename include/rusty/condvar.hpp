#pragma once

#include <condition_variable>
#include <mutex>
#include <chrono>
#include "mutex.hpp"

namespace rusty {

// =============================================================================
// WaitTimeoutResult - Result type for timed waits
// =============================================================================
// Matches Rust's std::sync::WaitTimeoutResult
class WaitTimeoutResult {
private:
    bool timed_out_;

public:
    explicit WaitTimeoutResult(bool timed_out) : timed_out_(timed_out) {}

    // @safe - Returns true if the wait timed out
    bool timed_out() const { return timed_out_; }
};

// @safe
// Condvar - Condition variable for waiting and notification
// Similar to Rust's std::sync::Condvar
//
// API matches Rust:
//   - wait(guard) -> LockResult<MutexGuard<T>>
//   - wait_while(guard, condition) -> LockResult<MutexGuard<T>>
//   - wait_timeout(guard, duration) -> LockResult<(MutexGuard<T>, WaitTimeoutResult)>
//   - wait_timeout_while(guard, duration, condition) -> LockResult<(MutexGuard<T>, bool)>
//
// Usage with Mutex<T> (Rust-like pattern):
//   Mutex<bool> ready(false);
//   Condvar cv;
//
//   // Thread 1 (waiter) - wait_while waits WHILE condition is TRUE
//   {
//       auto guard = ready.lock().unwrap();
//       guard = cv.wait_while(std::move(guard), [](bool& r){ return !r; }).unwrap();
//   }
//
//   // Thread 2 (notifier)
//   {
//       auto guard = ready.lock().unwrap();
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
    // Rust-like API with MutexGuard<T>
    // These methods take the guard by rvalue reference and return it in a Result,
    // matching Rust's ownership semantics.
    // =========================================================================

    // @safe - Wait on a MutexGuard (basic wait, no predicate)
    // Returns LockResult containing the guard after waking up.
    // Note: May wake spuriously - use wait_while for predicate-based waiting.
    template<typename T>
    [[nodiscard]] LockResult<T> wait(MutexGuard<T>&& guard) {
        cv_.wait(guard.underlying_lock());
        return LockResult<T>::Ok(std::move(guard));
    }

    // @safe - Wait WHILE condition is TRUE (Rust semantics)
    // Blocks until condition returns false.
    // This is the Rust naming - "wait while the condition holds"
    template<typename T, typename Condition>
    [[nodiscard]] LockResult<T> wait_while(MutexGuard<T>&& guard, Condition condition) {
        // Rust: waits WHILE condition is TRUE, stops when FALSE
        // C++ std::condition_variable: waits UNTIL predicate is TRUE
        // So we negate: wait until NOT condition
        cv_.wait(guard.underlying_lock(), [&]{ return !condition(*guard); });
        return LockResult<T>::Ok(std::move(guard));
    }

    // @safe - Wait with timeout (no predicate)
    // Returns pair of (guard, WaitTimeoutResult) wrapped in Result
    template<typename T, typename Rep, typename Period>
    [[nodiscard]] Result<std::pair<MutexGuard<T>, WaitTimeoutResult>, PoisonError<T>> wait_timeout(
        MutexGuard<T>&& guard,
        const std::chrono::duration<Rep, Period>& duration
    ) {
        auto status = cv_.wait_for(guard.underlying_lock(), duration);
        bool timed_out = (status == std::cv_status::timeout);
        using ResultType = std::pair<MutexGuard<T>, WaitTimeoutResult>;
        return Result<ResultType, PoisonError<T>>::Ok(
            ResultType(std::move(guard), WaitTimeoutResult(timed_out))
        );
    }

    // @safe - Wait with timeout WHILE condition is TRUE (Rust semantics)
    // Returns pair of (guard, bool) where bool indicates if condition is now false
    template<typename T, typename Rep, typename Period, typename Condition>
    [[nodiscard]] Result<std::pair<MutexGuard<T>, bool>, PoisonError<T>> wait_timeout_while(
        MutexGuard<T>&& guard,
        const std::chrono::duration<Rep, Period>& duration,
        Condition condition
    ) {
        // Rust: waits WHILE condition is TRUE, returns when FALSE or timeout
        // C++ wait_for with pred: waits UNTIL predicate is TRUE, returns pred value
        // So we negate: wait until NOT condition, return whether condition is now false
        bool condition_false = cv_.wait_for(
            guard.underlying_lock(),
            duration,
            [&]{ return !condition(*guard); }
        );
        using ResultType = std::pair<MutexGuard<T>, bool>;
        return Result<ResultType, PoisonError<T>>::Ok(
            ResultType(std::move(guard), condition_false)
        );
    }

    // =========================================================================
    // C++ compatibility API with std::unique_lock
    // These keep the traditional C++ semantics for backward compatibility.
    // =========================================================================

    // @safe - Wait on a unique_lock (C++ style, no return value)
    void wait(std::unique_lock<std::mutex>& lock) {
        cv_.wait(lock);
    }

    // @safe - Wait with predicate (C++ semantics: waits UNTIL pred is TRUE)
    template<typename Predicate>
    void wait(std::unique_lock<std::mutex>& lock, Predicate pred) {
        cv_.wait(lock, pred);
    }

    // @safe - Wait for duration
    template<typename Rep, typename Period>
    bool wait_for(
        std::unique_lock<std::mutex>& lock,
        const std::chrono::duration<Rep, Period>& duration
    ) {
        return cv_.wait_for(lock, duration) == std::cv_status::no_timeout;
    }

    // @safe - Wait for duration with predicate (C++ semantics)
    template<typename Rep, typename Period, typename Predicate>
    bool wait_for(
        std::unique_lock<std::mutex>& lock,
        const std::chrono::duration<Rep, Period>& duration,
        Predicate pred
    ) {
        return cv_.wait_for(lock, duration, pred);
    }

    // @safe - Wait until time point
    template<typename Clock, typename Duration>
    bool wait_until(
        std::unique_lock<std::mutex>& lock,
        const std::chrono::time_point<Clock, Duration>& timeout_time
    ) {
        return cv_.wait_until(lock, timeout_time) == std::cv_status::no_timeout;
    }

    // @safe - Wait until time point with predicate
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
