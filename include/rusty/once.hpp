#pragma once

#include <mutex>
#include <atomic>

namespace rusty {

// Once - Ensures a piece of code is executed exactly once
// Matches Rust's std::sync::Once behavior
//
// Usage:
//   static Once INIT;
//   static int* global_data = nullptr;
//
//   INIT.call_once([] {
//       global_data = new int(42);
//   });
//
class Once {
private:
    std::once_flag flag_;

public:
    Once() = default;

    // Execute the given function exactly once
    // If multiple threads call call_once() simultaneously,
    // exactly one will execute the function, and the others will wait
    template<typename F>
    void call_once(F&& func) {
        std::call_once(flag_, std::forward<F>(func));
    }

    // Non-copyable, non-movable
    Once(const Once&) = delete;
    Once& operator=(const Once&) = delete;
    Once(Once&&) = delete;
    Once& operator=(Once&&) = delete;

    ~Once() = default;
};

// OnceCell - A cell which can be written to only once
// Similar to Rust's once_cell crate (now std::sync::OnceLock in Rust)
//
// Usage:
//   static OnceCell<int> CELL;
//
//   // First write succeeds
//   CELL.set(42);
//
//   // Subsequent writes are ignored
//   CELL.set(100);  // Does nothing
//
//   // Get value (returns nullptr if not initialized)
//   const int* value = CELL.get();
//
template<typename T>
class OnceCell {
private:
    std::once_flag flag_;
    alignas(T) unsigned char storage_[sizeof(T)];
    std::atomic<bool> initialized_{false};

    T* as_ptr() { return reinterpret_cast<T*>(storage_); }
    const T* as_ptr() const { return reinterpret_cast<const T*>(storage_); }

public:
    OnceCell() = default;

    // Set the value (only succeeds if not already set)
    // Returns true if the value was set, false if already initialized
    bool set(T value) {
        bool success = false;
        std::call_once(flag_, [this, &value, &success]() {
            new (storage_) T(std::move(value));
            initialized_.store(true, std::memory_order_release);
            success = true;
        });
        return success;
    }

    // Get the value if initialized, nullptr otherwise
    const T* get() const {
        if (initialized_.load(std::memory_order_acquire)) {
            return as_ptr();
        }
        return nullptr;
    }

    // Get mutable reference to value if initialized, nullptr otherwise
    T* get_mut() {
        if (initialized_.load(std::memory_order_acquire)) {
            return as_ptr();
        }
        return nullptr;
    }

    // Get or initialize the value
    template<typename F>
    const T& get_or_init(F&& func) {
        std::call_once(flag_, [this, &func]() {
            new (storage_) T(func());
            initialized_.store(true, std::memory_order_release);
        });
        return *as_ptr();
    }

    // Check if the cell is initialized
    bool is_initialized() const {
        return initialized_.load(std::memory_order_acquire);
    }

    // Non-copyable, non-movable
    OnceCell(const OnceCell&) = delete;
    OnceCell& operator=(const OnceCell&) = delete;
    OnceCell(OnceCell&&) = delete;
    OnceCell& operator=(OnceCell&&) = delete;

    ~OnceCell() {
        if (initialized_.load(std::memory_order_acquire)) {
            as_ptr()->~T();
        }
    }
};

} // namespace rusty
