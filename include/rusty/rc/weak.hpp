#ifndef RUSTY_RC_WEAK_HPP
#define RUSTY_RC_WEAK_HPP

#include <cassert>
#include <cstddef>
#include "../option.hpp"
#include "../rc.hpp"

// rc::Weak<T> - Weak reference for single-threaded Rc<T>
// Equivalent to Rust's std::rc::Weak
//
// Guarantees:
// - Does not prevent deallocation of the value
// - Can be upgraded to strong reference if value still exists
// - Breaks reference cycles
// - NOT thread-safe (single-threaded only)

namespace rusty {
namespace rc {

// Forward declaration
template<typename T> class Weak;

// Weak reference for Rc (non-thread-safe)
template<typename T>
class Weak {
private:
    friend class rusty::Rc<T>;
    typename rusty::Rc<T>::ControlBlock* ptr;
    
public:
    // Default constructor
    Weak() : ptr(nullptr) {}
    
    // Private constructor from control block (for Rc to use)
    explicit Weak(typename rusty::Rc<T>::ControlBlock* p) : ptr(p) {}
    
    // Copy constructor
    Weak(const Weak& other) : ptr(other.ptr) {}
    
    // Move constructor
    Weak(Weak&& other) noexcept : ptr(other.ptr) {
        other.ptr = nullptr;
    }
    
    // Copy assignment
    Weak& operator=(const Weak& other) {
        ptr = other.ptr;
        return *this;
    }
    
    // Move assignment
    Weak& operator=(Weak&& other) noexcept {
        ptr = other.ptr;
        other.ptr = nullptr;
        return *this;
    }
    
    // Try to upgrade to strong reference
    Option<rusty::Rc<T>> upgrade() const {
        if (ptr && ptr->ref_count > 0) {
            // Create a new Rc by calling private constructor
            // This will properly manage the reference count
            return Some(rusty::Rc<T>(ptr, true));  // true = increment ref count
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
    
    // Clone - explicitly create a new Weak to the same value
    Weak clone() const {
        return Weak(*this);
    }
};

// Downgrade function - create weak from strong  
template<typename T>
Weak<T> downgrade(const rusty::Rc<T>& rc) {
    // Need to access private member - make Rc friend
    struct AccessHelper {
        static typename rusty::Rc<T>::ControlBlock* getPtr(const rusty::Rc<T>& r) {
            return nullptr; // Can't access private member without friend
        }
    };
    // For now, return default-constructed weak
    // TODO: Make Rc<T> friend of Weak<T> to access ptr
    return Weak<T>();
}

} // namespace rc
} // namespace rusty

#endif // RUSTY_RC_WEAK_HPP