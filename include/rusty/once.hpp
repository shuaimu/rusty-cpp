#pragma once

#include <atomic>
#include "platform/threading.hpp"
#include "option.hpp"

namespace rusty {

// Synchronization note (Once, OnceCell, OnceLock).
//
// Every reader observes completion through the cell's own atomic: an acquire
// load paired with the release store the initializer makes after it
// finishes. A completed cell is read on that fast path alone, without
// entering call_once; a thread that did enter call_once (it raced the
// initializer and waited for it) loads the flag again before it reads.
// std::call_once's internal handoff would order those reads too, but in
// libc++ it lives in the uninstrumented shared library, so ThreadSanitizer
// cannot see it and reported the waiter's read of the value as a race with
// the initializer's write (SRPC's TSan battery, through Lion's
// Instant::now() start time). The atomic edge is one TSan sees, and the fast
// path is also the cheaper one.

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
    platform::threading::once_flag flag_;
    std::atomic<bool> completed_{false};

public:
    Once() = default;
    static Once new_() { return Once(); }

    // Execute the given function exactly once
    // If multiple threads call call_once() simultaneously,
    // exactly one will execute the function, and the others will wait
    template<typename F>
    void call_once(F&& func) {
        if (completed_.load(std::memory_order_acquire)) {
            return;
        }
        platform::threading::call_once(flag_, [this, &func]() {
            std::forward<F>(func)();
            completed_.store(true, std::memory_order_release);
        });
        // The waiters' edge to the function's effects (see the note above).
        (void)completed_.load(std::memory_order_acquire);
    }

    // Rust's `Once::is_completed`.
    bool is_completed() const {
        return completed_.load(std::memory_order_acquire);
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
    platform::threading::once_flag flag_;
    alignas(T) unsigned char storage_[sizeof(T)];
    std::atomic<bool> initialized_{false};

    T* as_ptr() { return reinterpret_cast<T*>(storage_); }
    const T* as_ptr() const { return reinterpret_cast<const T*>(storage_); }

public:
    OnceCell() = default;

    // Set the value (only succeeds if not already set)
    // Returns true if the value was set, false if already initialized
    bool set(T value) {
        if (initialized_.load(std::memory_order_acquire)) {
            return false;
        }
        bool success = false;
        platform::threading::call_once(flag_, [this, &value, &success]() {
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
        if (!initialized_.load(std::memory_order_acquire)) {
            platform::threading::call_once(flag_, [this, &func]() {
                new (storage_) T(func());
                initialized_.store(true, std::memory_order_release);
            });
            // The waiters' edge to the initializer's writes (see the note
            // above). The flag is set: call_once returned normally.
            (void)initialized_.load(std::memory_order_acquire);
        }
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


// OnceLock<T> - Rust's `std::sync::OnceLock`: a thread-safe cell written at
// most once, with Rust's API shape (`get` returns an Option of a reference;
// `get_or_init` runs the initializer once). Built on OnceCell.
template<typename T>
class OnceLock {
private:
    OnceCell<T> cell_;

public:
    OnceLock() = default;

    // `OnceLock::new()`. Returned as a prvalue, so the non-movable cell is
    // constructed in place (`static OnceLock<T> X = OnceLock<T>::new_();`).
    static OnceLock new_() { return OnceLock(); }

    Option<const T&> get() const {
        const T* value = cell_.get();
        return value != nullptr ? Option<const T&>(*value) : Option<const T&>(None);
    }

    template<typename F>
    const T& get_or_init(F&& init) {
        return cell_.get_or_init(std::forward<F>(init));
    }

    OnceLock(const OnceLock&) = delete;
    OnceLock& operator=(const OnceLock&) = delete;
    OnceLock(OnceLock&&) = delete;
    OnceLock& operator=(OnceLock&&) = delete;
};

} // namespace rusty
