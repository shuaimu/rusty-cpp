#ifndef RUSTY_SYNC_WEAK_HPP
#define RUSTY_SYNC_WEAK_HPP

#include <atomic>
#include <cassert>
#include <cstddef>
#include "../option.hpp"
#include "../arc.hpp"

// sync::Weak<T> - Weak reference for thread-safe Arc<T>
// Equivalent to Rust's std::sync::Weak
//
// Guarantees:
// - Does not prevent deallocation of the value
// - Can be upgraded to strong reference if value still exists
// - Breaks reference cycles
// - Thread-safe (uses atomic operations)

namespace rusty {
namespace sync {

// Forward declaration
template<typename T> class Weak;

// Weak reference for Arc (thread-safe)
template<typename T>
class Weak {
private:
    friend class rusty::Arc<T>;
    typename rusty::Arc<T>::ControlBlock* ptr;
    
public:
    // Default constructor
    Weak() : ptr(nullptr) {}
    
    // Private constructor from control block (for Arc to use)
    explicit Weak(typename rusty::Arc<T>::ControlBlock* p) : ptr(p) {
        if (ptr) {
            ptr->weak_count.fetch_add(1, std::memory_order_relaxed);
        }
    }
    
    // Copy constructor
    Weak(const Weak& other) : ptr(other.ptr) {
        if (ptr) {
            ptr->weak_count.fetch_add(1, std::memory_order_relaxed);
        }
    }
    
    // Move constructor
    Weak(Weak&& other) noexcept : ptr(other.ptr) {
        other.ptr = nullptr;
    }
    
    // Copy assignment
    Weak& operator=(const Weak& other) {
        if (this != &other) {
            // Release old weak reference
            if (ptr) {
                if (ptr->weak_count.fetch_sub(1, std::memory_order_acq_rel) == 1) {
                    // Last weak reference, check if we should delete control block
                    if (ptr->ref_count.load(std::memory_order_acquire) == 0) {
                        delete ptr;
                    }
                }
            }
            
            // Acquire new weak reference
            ptr = other.ptr;
            if (ptr) {
                ptr->weak_count.fetch_add(1, std::memory_order_relaxed);
            }
        }
        return *this;
    }
    
    // Move assignment
    Weak& operator=(Weak&& other) noexcept {
        if (this != &other) {
            // Release old weak reference
            if (ptr) {
                if (ptr->weak_count.fetch_sub(1, std::memory_order_acq_rel) == 1) {
                    // Last weak reference, check if we should delete control block
                    if (ptr->ref_count.load(std::memory_order_acquire) == 0) {
                        delete ptr;
                    }
                }
            }
            
            // Take ownership from other
            ptr = other.ptr;
            other.ptr = nullptr;
        }
        return *this;
    }
    
    // Destructor
    ~Weak() {
        if (ptr) {
            if (ptr->weak_count.fetch_sub(1, std::memory_order_acq_rel) == 1) {
                // Last weak reference, check if we should delete control block
                if (ptr->ref_count.load(std::memory_order_acquire) == 0) {
                    delete ptr;
                }
            }
        }
    }
    
    // Try to upgrade to strong reference
    Option<rusty::Arc<T>> upgrade() const {
        if (!ptr) {
            return None;
        }
        
        // Try to increment strong count atomically
        size_t old_count = ptr->ref_count.load(std::memory_order_acquire);
        while (old_count > 0) {
            if (ptr->ref_count.compare_exchange_weak(
                old_count, old_count + 1,
                std::memory_order_acquire,
                std::memory_order_relaxed)) {
                // Successfully incremented, create Arc with already-incremented count
                return Some(rusty::Arc<T>(ptr, false));  // false = don't increment again
            }
            // CAS failed, old_count was updated, retry
        }
        
        // ref_count is 0, value has been dropped
        return None;
    }
    
    // Check if the value has been dropped
    bool expired() const {
        return !ptr || ptr->ref_count.load(std::memory_order_acquire) == 0;
    }
    
    // Get strong count (0 if expired)
    size_t strong_count() const {
        return ptr ? ptr->ref_count.load(std::memory_order_acquire) : 0;
    }
    
    // Get weak count (excluding this one)
    size_t weak_count() const {
        return ptr ? ptr->weak_count.load(std::memory_order_acquire) - 1 : 0;
    }
    
    // Clone - explicitly create a new Weak to the same value
    Weak clone() const {
        return Weak(*this);
    }
};

// Downgrade function - create weak from strong
template<typename T>
Weak<T> downgrade(const rusty::Arc<T>& arc) {
    return Weak<T>(arc.ptr);
}

} // namespace sync
} // namespace rusty

#endif // RUSTY_SYNC_WEAK_HPP