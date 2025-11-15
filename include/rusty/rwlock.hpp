#pragma once

#include <shared_mutex>
#include <mutex>
#include "option.hpp"
#include "unsafe_cell.hpp"

namespace rusty {

// RwLock - Read-Write lock allowing multiple readers or single writer
// Matches Rust's std::sync::RwLock behavior
template<typename T>
class RwLock {
private:
    UnsafeCell<std::shared_mutex> mtx_;
    T data_;

public:
    // ReadGuard - RAII read guard (shared lock)
    class ReadGuard {
    private:
        std::shared_lock<std::shared_mutex> lock_;
        const T* data_;

        friend class RwLock;

        ReadGuard(std::shared_lock<std::shared_mutex>&& lock, const T* data)
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
        std::unique_lock<std::shared_mutex> lock_;
        T* data_;

        friend class RwLock;

        WriteGuard(std::unique_lock<std::shared_mutex>&& lock, T* data)
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
        return ReadGuard(std::shared_lock(*mtx_.get()), &data_);
    }

    // Try to acquire read lock (non-blocking)
    [[nodiscard]] Option<ReadGuard> try_read() const {
        std::shared_lock lock(*mtx_.get(), std::try_to_lock);
        if (lock.owns_lock()) {
            return Some(ReadGuard(std::move(lock), &data_));
        }
        return None;
    }

    // Acquire write lock (exclusive)
    [[nodiscard]] WriteGuard write() {
        return WriteGuard(std::unique_lock(*mtx_.get()), &data_);
    }

    // Acquire write lock (exclusive) - const version
    [[nodiscard]] WriteGuard write() const {
        return WriteGuard(std::unique_lock(*mtx_.get()), const_cast<T*>(&data_));
    }

    // Try to acquire write lock (non-blocking)
    [[nodiscard]] Option<WriteGuard> try_write() {
        std::unique_lock lock(*mtx_.get(), std::try_to_lock);
        if (lock.owns_lock()) {
            return Some(WriteGuard(std::move(lock), &data_));
        }
        return None;
    }

    // Try to acquire write lock (const version)
    [[nodiscard]] Option<WriteGuard> try_write() const {
        std::unique_lock lock(*mtx_.get(), std::try_to_lock);
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
