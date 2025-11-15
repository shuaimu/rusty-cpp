#pragma once

#include <mutex>
#include "option.hpp"
#include "unsafe_cell.hpp"

namespace rusty {

// @unsafe - Interior mutability primitive for thread synchronization
template<typename T>
class Mutex {
private:
    UnsafeCell<std::mutex> mtx_;
    T data_;

public:
    // MutexGuard - RAII lock guard
    class MutexGuard {
    private:
        std::unique_lock<std::mutex> lock_;
        T* data_;

        friend class Mutex;

        MutexGuard(std::unique_lock<std::mutex>&& lock, T* data)
            : lock_(std::move(lock)), data_(data) {}

    public:
        // Access to data
        T& operator*() { return *data_; }
        const T& operator*() const { return *data_; }

        T* operator->() { return data_; }
        const T* operator->() const { return data_; }

        // Get raw pointer
        T* get() { return data_; }
        const T* get() const { return data_; }

        // Get mutable reference (Rust-like API)
        T& get_mut() { return *data_; }

        // Consume guard and extract data (like Rust's into_inner)
        T into_inner() && {
            T result = std::move(*data_);
            // Lock will be released by destructor
            return result;
        }

        // Non-copyable, movable
        MutexGuard(const MutexGuard&) = delete;
        MutexGuard& operator=(const MutexGuard&) = delete;
        MutexGuard(MutexGuard&&) = default;
        MutexGuard& operator=(MutexGuard&&) = default;

        // Destructor unlocks automatically
        ~MutexGuard() = default;
    };

    // @safe - Constructor initializes mutex and data
    explicit Mutex(T value) : data_(std::move(value)) {}

    // @safe - Acquires lock and returns RAII guard
    [[nodiscard]] MutexGuard lock() {
        return MutexGuard(std::unique_lock(*mtx_.get()), &data_);
    }

    // @safe - Acquires lock with const access
    [[nodiscard]] MutexGuard lock() const {
        return MutexGuard(std::unique_lock(*mtx_.get()), const_cast<T*>(&data_));
    }

    // @safe - Attempts to acquire lock without blocking
    [[nodiscard]] Option<MutexGuard> try_lock() {
        std::unique_lock lock(*mtx_.get(), std::try_to_lock);
        if (lock.owns_lock()) {
            return Some(MutexGuard(std::move(lock), &data_));
        }
        return None;
    }

    // @safe - Attempts to acquire lock with const access
    [[nodiscard]] Option<MutexGuard> try_lock() const {
        std::unique_lock lock(*mtx_.get(), std::try_to_lock);
        if (lock.owns_lock()) {
            return Some(MutexGuard(std::move(lock), const_cast<T*>(&data_)));
        }
        return None;
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
