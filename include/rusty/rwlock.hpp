#pragma once

#include "option.hpp"
#include "platform/threading.hpp"
#include "unsafe_cell.hpp"

namespace rusty {

// @unsafe - Interior mutability primitive for thread synchronization
// RwLock - Read-Write lock allowing multiple readers or single writer
// Matches Rust's std::sync::RwLock behavior
template<typename T>
class RwLock {
private:
    UnsafeCell<platform::threading::shared_mutex> mtx_;
    T data_;

public:
    // ReadGuard - RAII read guard (shared lock)
    class ReadGuard {
    private:
        platform::threading::shared_lock<platform::threading::shared_mutex> lock_;
        const T* data_;

        friend class RwLock;

        ReadGuard(platform::threading::shared_lock<platform::threading::shared_mutex>&& lock, const T* data)
            : lock_(std::move(lock)), data_(data) {}

    public:
        // Access to data (read-only)
        const T& operator*() const { return *data_; }
        const T* operator->() const { return data_; }
        const T* get() const { return data_; }

        // Non-copyable, movable
        ReadGuard(const ReadGuard&) = delete;
        ReadGuard& operator=(const ReadGuard&) = delete;
        ReadGuard(ReadGuard&&) = default;
        ReadGuard& operator=(ReadGuard&&) = default;

        ~ReadGuard() = default;
    };

    // WriteGuard - RAII write guard (exclusive lock)
    class WriteGuard {
    private:
        platform::threading::unique_lock<platform::threading::shared_mutex> lock_;
        T* data_;

        friend class RwLock;

        WriteGuard(platform::threading::unique_lock<platform::threading::shared_mutex>&& lock, T* data)
            : lock_(std::move(lock)), data_(data) {}

    public:
        // Access to data (read-write)
        T& operator*() { return *data_; }
        const T& operator*() const { return *data_; }

        T* operator->() { return data_; }
        const T* operator->() const { return data_; }

        T* get() { return data_; }
        const T* get() const { return data_; }

        // Get mutable reference (Rust-like API)
        T& get_mut() { return *data_; }

        // Consume guard and extract data
        T into_inner() && {
            T result = std::move(*data_);
            return result;
        }

        // Non-copyable, movable
        WriteGuard(const WriteGuard&) = delete;
        WriteGuard& operator=(const WriteGuard&) = delete;
        WriteGuard(WriteGuard&&) = default;
        WriteGuard& operator=(WriteGuard&&) = default;

        ~WriteGuard() = default;
    };

    // Constructor
    explicit RwLock(T value) : data_(std::move(value)) {}

    // Acquire read lock (shared)
    [[nodiscard]] ReadGuard read() const {
        return ReadGuard(platform::threading::shared_lock<platform::threading::shared_mutex>(*mtx_.get()), &data_);
    }

    // Try to acquire read lock (non-blocking)
    [[nodiscard]] Option<ReadGuard> try_read() const {
        platform::threading::shared_lock<platform::threading::shared_mutex> lock(*mtx_.get(), platform::threading::try_to_lock);
        if (lock.owns_lock()) {
            return Some(ReadGuard(std::move(lock), &data_));
        }
        return None;
    }

    // Acquire write lock (exclusive)
    [[nodiscard]] WriteGuard write() {
        return WriteGuard(platform::threading::unique_lock<platform::threading::shared_mutex>(*mtx_.get()), &data_);
    }

    // Acquire write lock (exclusive) - const version
    [[nodiscard]] WriteGuard write() const {
        return WriteGuard(platform::threading::unique_lock<platform::threading::shared_mutex>(*mtx_.get()), const_cast<T*>(&data_));
    }

    // Try to acquire write lock (non-blocking)
    [[nodiscard]] Option<WriteGuard> try_write() {
        platform::threading::unique_lock<platform::threading::shared_mutex> lock(*mtx_.get(), platform::threading::try_to_lock);
        if (lock.owns_lock()) {
            return Some(WriteGuard(std::move(lock), &data_));
        }
        return None;
    }

    // Try to acquire write lock (const version)
    [[nodiscard]] Option<WriteGuard> try_write() const {
        platform::threading::unique_lock<platform::threading::shared_mutex> lock(*mtx_.get(), platform::threading::try_to_lock);
        if (lock.owns_lock()) {
            return Some(WriteGuard(std::move(lock), const_cast<T*>(&data_)));
        }
        return None;
    }

    // RwLock is not copyable or movable
    RwLock(const RwLock&) = delete;
    RwLock& operator=(const RwLock&) = delete;
    RwLock(RwLock&&) = delete;
    RwLock& operator=(RwLock&&) = delete;

    ~RwLock() = default;
};

// Helper function
template<typename T>
auto make_rwlock(T value) {
    return RwLock<T>(std::move(value));
}

} // namespace rusty
