#ifndef RUSTY_WEAK_HPP
#define RUSTY_WEAK_HPP

#include <atomic>
#include <cassert>
#include <cstddef>
#include "option.hpp"

// Weak<T> - Weak reference for Rc<T> and Arc<T>
// Equivalent to Rust's std::rc::Weak and std::sync::Weak
//
// This implementation works with the existing Rc and Arc control blocks
// by storing raw pointers and checking if the strong count is still > 0
// when upgrading. This is a simplified approach that doesn't track weak
// counts separately, but provides the core functionality.

namespace rusty {

// Forward declarations
template<typename T> class Rc;
template<typename T> class Arc;

// Weak reference for Rc (non-thread-safe)
template<typename T>
class RcWeak {
private:
    // We need to store the control block pointer
    // This matches the internal structure of Rc
    typename Rc<T>::ControlBlock* ptr;
    
public:
    friend class Rc<T>;
    
    // Default constructor
    RcWeak() : ptr(nullptr) {}
    
    // Private constructor from control block (for Rc to use)
    explicit RcWeak(typename Rc<T>::ControlBlock* p) : ptr(p) {}
    
    // Copy constructor
    RcWeak(const RcWeak& other) : ptr(other.ptr) {}
    
    // Move constructor
    RcWeak(RcWeak&& other) noexcept : ptr(other.ptr) {
        other.ptr = nullptr;
    }
    
    // Copy assignment
    RcWeak& operator=(const RcWeak& other) {
        ptr = other.ptr;
        return *this;
    }
    
    // Move assignment
    RcWeak& operator=(RcWeak&& other) noexcept {
        ptr = other.ptr;
        other.ptr = nullptr;
        return *this;
    }
    
    // Try to upgrade to strong reference
    Option<Rc<T>> upgrade() const {
        if (ptr && ptr->ref_count > 0) {
            Rc<T> result;
            result.ptr = ptr;
            ++ptr->ref_count;
            return Some(std::move(result));
        }
        return None;
    }
    
    // Check if the value has been dropped
    bool expired() const {
        return !ptr || ptr->ref_count == 0;
    }
    
    // Get strong count (0 if expired)
    size_t strong_count() const {
        return ptr ? ptr->ref_count : 0;
    }
};

// Weak reference for Arc (thread-safe)
template<typename T>
class ArcWeak {
private:
    // We need to store the control block pointer
    // This matches the internal structure of Arc
    typename Arc<T>::ControlBlock* ptr;
    
public:
    friend class Arc<T>;
    
    // Default constructor
    ArcWeak() : ptr(nullptr) {}
    
    // Private constructor from control block (for Arc to use)
    explicit ArcWeak(typename Arc<T>::ControlBlock* p) : ptr(p) {}
    
    // Copy constructor
    ArcWeak(const ArcWeak& other) : ptr(other.ptr) {}
    
    // Move constructor
    ArcWeak(ArcWeak&& other) noexcept : ptr(other.ptr) {
        other.ptr = nullptr;
    }
    
    // Copy assignment
    ArcWeak& operator=(const ArcWeak& other) {
        ptr = other.ptr;
        return *this;
    }
    
    // Move assignment
    ArcWeak& operator=(ArcWeak&& other) noexcept {
        ptr = other.ptr;
        other.ptr = nullptr;
        return *this;
    }
    
    // Try to upgrade to strong reference
    Option<Arc<T>> upgrade() const {
        if (!ptr) return None;
        
        // Try to increment strong count atomically
        size_t count = ptr->ref_count.load(std::memory_order_relaxed);
        while (count != 0) {
            if (ptr->ref_count.compare_exchange_weak(
                    count, count + 1,
                    std::memory_order_relaxed,
                    std::memory_order_relaxed)) {
                Arc<T> result;
                result.ptr = ptr;
                return Some(std::move(result));
            }
        }
        return None;
    }
    
    // Check if the value has been dropped
    bool expired() const {
        return !ptr || ptr->ref_count.load(std::memory_order_relaxed) == 0;
    }
    
    // Get strong count (0 if expired)
    size_t strong_count() const {
        return ptr ? ptr->ref_count.load(std::memory_order_relaxed) : 0;
    }
};

// Type alias for compatibility - use rc::Weak or sync::Weak for new code
template<typename T>
using Weak = RcWeak<T>;  // Default to single-threaded version

// Add downgrade methods to Rc and Arc
template<typename T>
RcWeak<T> downgrade(const Rc<T>& rc) {
    return RcWeak<T>(rc.ptr);
}

template<typename T>
ArcWeak<T> downgrade(const Arc<T>& arc) {
    return ArcWeak<T>(arc.ptr);
}

} // namespace rusty

#endif // RUSTY_WEAK_HPP