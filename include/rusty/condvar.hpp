#pragma once

#include <chrono>
#include <errno.h>
#include <mutex>
#include <pthread.h>
#include <time.h>
#include <utility>
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

// =============================================================================
// Internal helpers
// =============================================================================
namespace condvar_detail {

// @unsafe - Computes an absolute CLOCK_REALTIME deadline `now + delta` for
// pthread_cond_timedwait. pthread_cond_timedwait uses CLOCK_REALTIME by
// default; this matches that.
inline ::timespec realtime_after(std::chrono::nanoseconds delta) {
    if (delta.count() < 0) {
        delta = std::chrono::nanoseconds(0);
    }
    ::timespec ts{};
    // @unsafe { clock_gettime is not borrow-checked }
    { clock_gettime(CLOCK_REALTIME, &ts); }

    auto sec = std::chrono::seconds(ts.tv_sec);
    auto nsec = std::chrono::nanoseconds(ts.tv_nsec);
    auto total = sec + nsec + delta;

    auto out_sec = std::chrono::duration_cast<std::chrono::seconds>(total);
    auto out_nsec = std::chrono::duration_cast<std::chrono::nanoseconds>(
        total - out_sec);

    ::timespec out{};
    out.tv_sec = static_cast<::time_t>(out_sec.count());
    out.tv_nsec = static_cast<long>(out_nsec.count());
    return out;
}

} // namespace condvar_detail

// =============================================================================
// Condvar - Condition variable, Rust-like wrapper around pthread_cond_t
// =============================================================================
//
// Similar to Rust's std::sync::Condvar.
//
// API matches Rust:
//   - wait(guard) -> LockResult<MutexGuard<T>>
//   - wait_while(guard, condition) -> LockResult<MutexGuard<T>>
//   - wait_timeout(guard, duration) -> LockResult<(MutexGuard<T>, WaitTimeoutResult)>
//   - wait_timeout_while(guard, duration, condition) -> LockResult<(MutexGuard<T>, bool)>
//
// Movability: the wrapper is movable. The underlying pthread_cond_t is held
// behind a heap pointer so that, when a Condvar wrapper is moved, the
// pthread_cond_t itself stays at the same address. Any waiters blocked on
// the (heap-allocated) cond stay valid — only the wrapper transfers its
// owning pointer.
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
class Condvar {
private:
    // Heap-allocated pthread_cond_t. Held behind a pointer so the cond stays
    // at a stable address across wrapper moves; null after move-from. All
    // wait/notify methods bridge to pthread_* via this pointer.
    //
    // mutable so const wait/notify methods can pass the pointer to pthread
    // APIs (matches Rust's `&self` receivers on Condvar methods).
    mutable ::pthread_cond_t* cv_;

public:
    // @safe - Allocates and initializes a fresh pthread_cond_t on the heap.
    Condvar() : cv_(nullptr) {
        // @unsafe { pthread_cond_init + new are not borrow-checked }
        {
            cv_ = new ::pthread_cond_t{};
            ::pthread_cond_init(cv_, nullptr);
        }
    }

    // @safe - Destroys and deallocates the owned pthread_cond_t. Moved-from
    // Condvars hold a null pointer and skip cleanup.
    ~Condvar() {
        if (cv_ != nullptr) {
            // @unsafe { pthread_cond_destroy + delete are not borrow-checked }
            {
                ::pthread_cond_destroy(cv_);
                delete cv_;
            }
            cv_ = nullptr;
        }
    }

    // @safe - Move constructor: pointer transfer only. The underlying
    // pthread_cond_t stays at the same heap address, so concurrent waiters
    // remain blocked on a valid object.
    Condvar(Condvar&& other) noexcept : cv_(other.cv_) {
        other.cv_ = nullptr;
    }

    // @safe - Move assignment: destroy current, take other's pointer.
    Condvar& operator=(Condvar&& other) noexcept {
        if (this != &other) {
            if (cv_ != nullptr) {
                // @unsafe { pthread_cond_destroy + delete are not borrow-checked }
                {
                    ::pthread_cond_destroy(cv_);
                    delete cv_;
                }
            }
            cv_ = other.cv_;
            other.cv_ = nullptr;
        }
        return *this;
    }

    // Non-copyable (like Rust's Condvar).
    Condvar(const Condvar&) = delete;
    Condvar& operator=(const Condvar&) = delete;

    // =========================================================================
    // Notification methods
    // =========================================================================

    // @safe - Notify one waiting thread (Rust's notify_one).
    void notify_one() const {
        // @unsafe { pthread_cond_signal is not borrow-checked }
        { ::pthread_cond_signal(cv_); }
    }

    // @safe - Notify all waiting threads (Rust's notify_all).
    void notify_all() const {
        // @unsafe { pthread_cond_broadcast is not borrow-checked }
        { ::pthread_cond_broadcast(cv_); }
    }

    // =========================================================================
    // Rust-like API with MutexGuard<T>
    // These methods take the guard by rvalue reference and return it in a
    // Result, matching Rust's ownership semantics.
    // =========================================================================

    // @safe - Wait on a MutexGuard (basic wait, no predicate).
    // Returns LockResult containing the guard after waking up.
    // Note: May wake spuriously - use wait_while for predicate-based waiting.
    template<typename T>
    [[nodiscard]] LockResult<T> wait(MutexGuard<T>&& guard) const {
        // @unsafe { pthread_cond_wait is not borrow-checked }
        {
            auto& lk = guard.underlying_lock();
            ::pthread_cond_wait(cv_, lk.mutex()->native_handle());
        }
        return LockResult<T>::Ok(std::move(guard));
    }

    // @safe - Wait WHILE condition is TRUE (Rust semantics).
    // Blocks until condition returns false.
    template<typename T, typename Condition>
    [[nodiscard]] LockResult<T> wait_while(MutexGuard<T>&& guard,
                                            Condition condition) const {
        // @unsafe { pthread_cond_wait loop is not borrow-checked }
        {
            auto& lk = guard.underlying_lock();
            auto* mtx = lk.mutex()->native_handle();
            while (condition(*guard)) {
                ::pthread_cond_wait(cv_, mtx);
            }
        }
        return LockResult<T>::Ok(std::move(guard));
    }

    // @safe - Wait with timeout (no predicate).
    // Returns pair of (guard, WaitTimeoutResult) wrapped in Result.
    template<typename T, typename Rep, typename Period>
    [[nodiscard]] Result<std::pair<MutexGuard<T>, WaitTimeoutResult>, PoisonError<T>> wait_timeout(
        MutexGuard<T>&& guard,
        const std::chrono::duration<Rep, Period>& duration
    ) const {
        bool timed_out = false;
        // @unsafe { pthread_cond_timedwait is not borrow-checked }
        {
            auto& lk = guard.underlying_lock();
            const auto abs = condvar_detail::realtime_after(
                std::chrono::duration_cast<std::chrono::nanoseconds>(duration));
            const int rc = ::pthread_cond_timedwait(
                cv_, lk.mutex()->native_handle(), &abs);
            timed_out = (rc == ETIMEDOUT);
        }
        using ResultType = std::pair<MutexGuard<T>, WaitTimeoutResult>;
        return Result<ResultType, PoisonError<T>>::Ok(
            ResultType(std::move(guard), WaitTimeoutResult(timed_out))
        );
    }

    // @safe - Wait with timeout WHILE condition is TRUE (Rust semantics).
    // Returns pair of (guard, bool) where bool indicates if condition is now
    // false (true => condition became false, false => timed out with
    // condition still true).
    template<typename T, typename Rep, typename Period, typename Condition>
    [[nodiscard]] Result<std::pair<MutexGuard<T>, bool>, PoisonError<T>> wait_timeout_while(
        MutexGuard<T>&& guard,
        const std::chrono::duration<Rep, Period>& duration,
        Condition condition
    ) const {
        bool condition_now_false = false;
        // @unsafe { pthread_cond_timedwait loop is not borrow-checked }
        {
            auto& lk = guard.underlying_lock();
            auto* mtx = lk.mutex()->native_handle();
            const auto abs = condvar_detail::realtime_after(
                std::chrono::duration_cast<std::chrono::nanoseconds>(duration));
            while (condition(*guard)) {
                const int rc = ::pthread_cond_timedwait(cv_, mtx, &abs);
                if (rc == ETIMEDOUT) {
                    break;
                }
            }
            condition_now_false = !condition(*guard);
        }
        using ResultType = std::pair<MutexGuard<T>, bool>;
        return Result<ResultType, PoisonError<T>>::Ok(
            ResultType(std::move(guard), condition_now_false)
        );
    }

    // =========================================================================
    // C++ compatibility API with platform::threading::unique_lock
    // These keep the traditional C++ semantics for backward compatibility.
    // =========================================================================

    // @safe - Wait on a unique_lock (C++ style, no return value).
    void wait(platform::threading::unique_lock<platform::threading::mutex>& lock) const {
        // @unsafe { pthread_cond_wait is not borrow-checked }
        { ::pthread_cond_wait(cv_, lock.mutex()->native_handle()); }
    }

    // @safe - Wait with predicate (C++ semantics: waits UNTIL pred is TRUE).
    template<typename Predicate>
    void wait(platform::threading::unique_lock<platform::threading::mutex>& lock,
              Predicate pred) const {
        // @unsafe { pthread_cond_wait loop is not borrow-checked }
        {
            auto* mtx = lock.mutex()->native_handle();
            while (!pred()) {
                ::pthread_cond_wait(cv_, mtx);
            }
        }
    }

    // @safe - Wait for duration. Returns true if notified, false if timed out.
    template<typename Rep, typename Period>
    bool wait_for(
        platform::threading::unique_lock<platform::threading::mutex>& lock,
        const std::chrono::duration<Rep, Period>& duration
    ) const {
        // @unsafe { pthread_cond_timedwait is not borrow-checked }
        {
            const auto abs = condvar_detail::realtime_after(
                std::chrono::duration_cast<std::chrono::nanoseconds>(duration));
            const int rc = ::pthread_cond_timedwait(
                cv_, lock.mutex()->native_handle(), &abs);
            return rc != ETIMEDOUT;
        }
    }

    // @safe - Wait for duration with predicate (C++ semantics).
    // Returns final value of pred (true if eventually true, false on timeout).
    template<typename Rep, typename Period, typename Predicate>
    bool wait_for(
        platform::threading::unique_lock<platform::threading::mutex>& lock,
        const std::chrono::duration<Rep, Period>& duration,
        Predicate pred
    ) const {
        // @unsafe { pthread_cond_timedwait loop is not borrow-checked }
        {
            auto* mtx = lock.mutex()->native_handle();
            const auto abs = condvar_detail::realtime_after(
                std::chrono::duration_cast<std::chrono::nanoseconds>(duration));
            while (!pred()) {
                const int rc = ::pthread_cond_timedwait(cv_, mtx, &abs);
                if (rc == ETIMEDOUT) {
                    return pred();
                }
            }
            return true;
        }
    }

    // @safe - Wait until a steady/system clock time point.
    template<typename Clock, typename Duration>
    bool wait_until(
        platform::threading::unique_lock<platform::threading::mutex>& lock,
        const std::chrono::time_point<Clock, Duration>& timeout_time
    ) const {
        auto now = Clock::now();
        if (now >= timeout_time) {
            return false;
        }
        return wait_for(lock, timeout_time - now);
    }

    // @safe - Wait until a steady/system clock time point with predicate.
    template<typename Clock, typename Duration, typename Predicate>
    bool wait_until(
        platform::threading::unique_lock<platform::threading::mutex>& lock,
        const std::chrono::time_point<Clock, Duration>& timeout_time,
        Predicate pred
    ) const {
        while (!pred()) {
            auto now = Clock::now();
            if (now >= timeout_time) {
                return pred();
            }
            if (!wait_for(lock, timeout_time - now)) {
                return pred();
            }
        }
        return true;
    }

#if defined(RUSTY_PLATFORM_BACKEND_POSIX)
    // -------------------------------------------------------------------------
    // POSIX-only legacy overloads for std::mutex / std::unique_lock<std::mutex>.
    //
    // In the non-POSIX backend, platform::threading::mutex IS std::mutex and
    // platform::threading::unique_lock<...> IS std::unique_lock<std::mutex>,
    // so these overloads would collide with the ones above. Only enable them
    // when the platform mutex type is a distinct class.
    //
    // Both bridge to pthread_cond_wait / pthread_cond_timedwait via
    // std::mutex::native_handle(), which on POSIX systems returns
    // pthread_mutex_t* (libstdc++ and libc++).
    // -------------------------------------------------------------------------

    void wait(std::unique_lock<std::mutex>& lock) const {
        // @unsafe { pthread_cond_wait is not borrow-checked }
        { ::pthread_cond_wait(cv_, lock.mutex()->native_handle()); }
    }

    template<typename Predicate>
    void wait(std::unique_lock<std::mutex>& lock, Predicate pred) const {
        // @unsafe { pthread_cond_wait loop is not borrow-checked }
        {
            auto* mtx = lock.mutex()->native_handle();
            while (!pred()) {
                ::pthread_cond_wait(cv_, mtx);
            }
        }
    }

    template<typename Rep, typename Period>
    bool wait_for(
        std::unique_lock<std::mutex>& lock,
        const std::chrono::duration<Rep, Period>& duration
    ) const {
        // @unsafe { pthread_cond_timedwait is not borrow-checked }
        {
            const auto abs = condvar_detail::realtime_after(
                std::chrono::duration_cast<std::chrono::nanoseconds>(duration));
            const int rc = ::pthread_cond_timedwait(
                cv_, lock.mutex()->native_handle(), &abs);
            return rc != ETIMEDOUT;
        }
    }

    template<typename Rep, typename Period, typename Predicate>
    bool wait_for(
        std::unique_lock<std::mutex>& lock,
        const std::chrono::duration<Rep, Period>& duration,
        Predicate pred
    ) const {
        // @unsafe { pthread_cond_timedwait loop is not borrow-checked }
        {
            auto* mtx = lock.mutex()->native_handle();
            const auto abs = condvar_detail::realtime_after(
                std::chrono::duration_cast<std::chrono::nanoseconds>(duration));
            while (!pred()) {
                const int rc = ::pthread_cond_timedwait(cv_, mtx, &abs);
                if (rc == ETIMEDOUT) {
                    return pred();
                }
            }
            return true;
        }
    }

    template<typename Clock, typename Duration>
    bool wait_until(
        std::unique_lock<std::mutex>& lock,
        const std::chrono::time_point<Clock, Duration>& timeout_time
    ) const {
        auto now = Clock::now();
        if (now >= timeout_time) {
            return false;
        }
        return wait_for(lock, timeout_time - now);
    }

    template<typename Clock, typename Duration, typename Predicate>
    bool wait_until(
        std::unique_lock<std::mutex>& lock,
        const std::chrono::time_point<Clock, Duration>& timeout_time,
        Predicate pred
    ) const {
        while (!pred()) {
            auto now = Clock::now();
            if (now >= timeout_time) {
                return pred();
            }
            if (!wait_for(lock, timeout_time - now)) {
                return pred();
            }
        }
        return true;
    }
#endif
};

} // namespace rusty
