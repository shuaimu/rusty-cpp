#pragma once

#include <mutex>
#include "option.hpp"

namespace rusty {

template<typename T>
class Mutex {
private:
    mutable std::mutex mtx_;
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

    // Constructor
    explicit Mutex(T value) : data_(std::move(value)) {}

    // Lock and return guard
    [[nodiscard]] MutexGuard lock() {
        return MutexGuard(std::unique_lock(mtx_), &data_);
    }

    // Lock with const access
    [[nodiscard]] MutexGuard lock() const {
        return MutexGuard(std::unique_lock(mtx_), const_cast<T*>(&data_));
    }

    // Try-lock and return optional guard
    [[nodiscard]] Option<MutexGuard> try_lock() {
        std::unique_lock lock(mtx_, std::try_to_lock);
        if (lock.owns_lock()) {
            return Some(MutexGuard(std::move(lock), &data_));
        }
        return None;
    }

    // Try-lock with const access
    [[nodiscard]] Option<MutexGuard> try_lock() const {
        std::unique_lock lock(mtx_, std::try_to_lock);
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

    ~Mutex() = default;
};

// Helper function
template<typename T>
auto make_mutex(T value) {
    return Mutex<T>(std::move(value));
}

} // namespace rusty
