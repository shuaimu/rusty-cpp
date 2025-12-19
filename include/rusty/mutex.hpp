#pragma once

#include <mutex>
#include "option.hpp"
#include "unsafe_cell.hpp"

namespace rusty {

// Forward declaration
template<typename T> class Mutex;

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
//       auto guard = counter.lock();
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

    // @safe - Acquires lock and returns RAII guard (has internal @unsafe block)
    [[nodiscard]] MutexGuard<T> lock() {
        // @unsafe
        { return MutexGuard<T>(std::unique_lock<std::mutex>(*mtx_.get()), &data_); }
    }

    // @safe - Acquires lock with const access (has internal @unsafe block)
    [[nodiscard]] MutexGuard<T> lock() const {
        // @unsafe
        { return MutexGuard<T>(std::unique_lock<std::mutex>(*mtx_.get()), const_cast<T*>(&data_)); }
    }

    // @safe - Attempts to acquire lock without blocking (has internal @unsafe block)
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
