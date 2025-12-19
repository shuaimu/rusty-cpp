#pragma once

#include <mutex>
#include "option.hpp"
#include "result.hpp"
#include "unsafe_cell.hpp"

namespace rusty {

// Forward declaration
template<typename T> class Mutex;
template<typename T> class MutexGuard;

// =============================================================================
// PoisonError<T> - Error type for poisoned mutex
// =============================================================================
// In Rust, a mutex becomes "poisoned" if a thread panics while holding the lock.
// C++ doesn't have the same panic semantics, but we provide this for API consistency.
// The error contains the guard, allowing recovery of the data if desired.
//
// @safe
template<typename T>
class PoisonError {
private:
    // Use optional-like storage to allow default construction
    // (needed by Result's internal implementation)
    alignas(MutexGuard<T>) unsigned char guard_storage_[sizeof(MutexGuard<T>)];
    bool has_guard_ = false;

    MutexGuard<T>& guard_ref() {
        return *reinterpret_cast<MutexGuard<T>*>(guard_storage_);
    }
    const MutexGuard<T>& guard_ref() const {
        return *reinterpret_cast<const MutexGuard<T>*>(guard_storage_);
    }

public:
    // Default constructor (required by Result internals, creates empty error)
    PoisonError() : has_guard_(false) {}

    explicit PoisonError(MutexGuard<T>&& guard) : has_guard_(true) {
        new (guard_storage_) MutexGuard<T>(std::move(guard));
    }

    // Move constructor
    PoisonError(PoisonError&& other) noexcept : has_guard_(other.has_guard_) {
        if (has_guard_) {
            new (guard_storage_) MutexGuard<T>(std::move(other.guard_ref()));
        }
    }

    // Move assignment
    PoisonError& operator=(PoisonError&& other) noexcept {
        if (this != &other) {
            if (has_guard_) {
                guard_ref().~MutexGuard<T>();
            }
            has_guard_ = other.has_guard_;
            if (has_guard_) {
                new (guard_storage_) MutexGuard<T>(std::move(other.guard_ref()));
            }
        }
        return *this;
    }

    ~PoisonError() {
        if (has_guard_) {
            guard_ref().~MutexGuard<T>();
        }
    }

    // @safe - Get the guard (allows recovery of potentially inconsistent data)
    MutexGuard<T> into_inner() && {
        return std::move(guard_ref());
    }

    // @safe - Get reference to guard
    MutexGuard<T>& get_ref() { return guard_ref(); }
    const MutexGuard<T>& get_ref() const { return guard_ref(); }

    // Non-copyable
    PoisonError(const PoisonError&) = delete;
    PoisonError& operator=(const PoisonError&) = delete;
};

// Type aliases matching Rust's std::sync
template<typename T>
using LockResult = Result<MutexGuard<T>, PoisonError<T>>;

// @safe - MutexGuard - RAII lock guard for Mutex<T>
// Standalone template class to enable template deduction in Condvar
template<typename T>
class MutexGuard {
private:
    std::unique_lock<std::mutex> lock_;
    T* data_;

    friend class Mutex<T>;
    template<typename U> friend class Mutex;  // Allow all Mutex<U> to create guards

    MutexGuard(std::unique_lock<std::mutex>&& lock, T* data)
        : lock_(std::move(lock)), data_(data) {}

public:
    // @safe - Access to data
    T& operator*() { return *data_; }
    // @safe
    const T& operator*() const { return *data_; }

    // @safe
    T* operator->() { return data_; }
    // @safe
    const T* operator->() const { return data_; }

    // @safe - Get raw pointer
    T* get() { return data_; }
    // @safe
    const T* get() const { return data_; }

    // @safe - Get mutable reference (Rust-like API)
    T& get_mut() { return *data_; }

    // @safe - Consume guard and extract data (like Rust's into_inner)
    T into_inner() && {
        T result = std::move(*data_);
        // Lock will be released by destructor
        return result;
    }

    // Access to underlying lock for Condvar integration
    // @safe - Returns reference to the underlying unique_lock
    std::unique_lock<std::mutex>& underlying_lock() { return lock_; }
    // @safe
    const std::unique_lock<std::mutex>& underlying_lock() const { return lock_; }

    // Non-copyable, movable
    MutexGuard(const MutexGuard&) = delete;
    MutexGuard& operator=(const MutexGuard&) = delete;
    // @safe
    MutexGuard(MutexGuard&&) = default;
    // @safe
    MutexGuard& operator=(MutexGuard&&) = default;

    // @safe - Destructor unlocks automatically
    ~MutexGuard() = default;
};

// @safe - Mutex<T> - Thread-safe interior mutability primitive
// Similar to Rust's std::sync::Mutex<T>
//
// Usage:
//   Mutex<int> counter(0);
//   {
//       auto guard = counter.lock().unwrap();  // Like Rust!
//       *guard += 1;
//   }  // Lock released here
//
template<typename T>
class Mutex {
private:
    UnsafeCell<std::mutex> mtx_;  // UnsafeCell for interior mutability
    T data_;

public:
    // Type alias for the guard type
    using Guard = MutexGuard<T>;

    // @safe - Constructor initializes mutex and data
    explicit Mutex(T value) : data_(std::move(value)) {}

    // @safe - Acquires lock and returns LockResult (has internal @unsafe block)
    // Returns Result<MutexGuard<T>, PoisonError<T>> like Rust's std::sync::Mutex
    // Note: C++ doesn't have poisoning, so this always returns Ok
    [[nodiscard]] LockResult<T> lock() {
        // @unsafe
        {
            auto guard = MutexGuard<T>(std::unique_lock<std::mutex>(*mtx_.get()), &data_);
            return LockResult<T>::Ok(std::move(guard));
        }
    }

    // @safe - Acquires lock with const access (has internal @unsafe block)
    [[nodiscard]] LockResult<T> lock() const {
        // @unsafe
        {
            auto guard = MutexGuard<T>(std::unique_lock<std::mutex>(*mtx_.get()), const_cast<T*>(&data_));
            return LockResult<T>::Ok(std::move(guard));
        }
    }

    // @safe - Attempts to acquire lock without blocking (has internal @unsafe block)
    // Returns Option - None if lock is already held, Some(guard) otherwise
    // Note: Rust uses TryLockResult here, but since we don't have poisoning,
    // Option is sufficient (WouldBlock is the only possible "error")
    [[nodiscard]] Option<MutexGuard<T>> try_lock() {
        // @unsafe
        {
            std::unique_lock<std::mutex> lk(*mtx_.get(), std::try_to_lock);
            if (lk.owns_lock()) {
                return Some(MutexGuard<T>(std::move(lk), &data_));
            }
            return None;
        }
    }

    // @safe - Attempts to acquire lock with const access (has internal @unsafe block)
    [[nodiscard]] Option<MutexGuard<T>> try_lock() const {
        // @unsafe
        {
            std::unique_lock<std::mutex> lk(*mtx_.get(), std::try_to_lock);
            if (lk.owns_lock()) {
                return Some(MutexGuard<T>(std::move(lk), const_cast<T*>(&data_)));
            }
            return None;
        }
    }

    // Mutex is not copyable or movable
    Mutex(const Mutex&) = delete;
    Mutex& operator=(const Mutex&) = delete;
    Mutex(Mutex&&) = delete;
    Mutex& operator=(Mutex&&) = delete;

    // @safe - RAII destructor
    ~Mutex() = default;
};

// @safe - Helper function to create Mutex
template<typename T>
auto make_mutex(T value) {
    return Mutex<T>(std::move(value));
}

} // namespace rusty
