#ifndef RUSTY_RC_HPP
#define RUSTY_RC_HPP

#include <cassert>
#include <utility>  // for std::move, std::forward
#include <cstddef>  // for size_t

// Rc<T> - Reference Counted pointer (non-atomic)
// Equivalent to Rust's Rc<T>
//
// Guarantees:
// - Single-threaded reference counting
// - Shared ownership within a thread
// - Automatic deallocation when last Rc is dropped
// - Immutable access only (use RefCell for interior mutability)
//
// WARNING: Not thread-safe! Use Arc for multi-threaded scenarios

// @safe
namespace rusty {

// Forward declarations for weak references
template<typename T> class Rc;
template<typename T> class RcWeak;
namespace rc { template<typename T> class Weak; }
template<typename T> RcWeak<T> downgrade(const Rc<T>&);

template<typename T>
class Rc {
private:
    template<typename U>
    friend class RcWeak;
    template<typename U>
    friend class rc::Weak;
    template<typename U>
    friend RcWeak<U> downgrade(const Rc<U>&);
    
    struct ControlBlock {
        T value;
        size_t ref_count;
        
        template<typename... Args>
        ControlBlock(Args&&... args) 
            : value(std::forward<Args>(args)...), ref_count(1) {}
    };
    
    ControlBlock* ptr;
    
    void increment() {
        if (ptr) {
            ++ptr->ref_count;
        }
    }
    
    void decrement() {
        if (ptr) {
            if (--ptr->ref_count == 0) {
                delete ptr;
            }
        }
    }
    
public:
    // Default constructor - creates empty Rc
    Rc() : ptr(nullptr) {}
    
    // Rust-idiomatic factory method - Rc::new()
    // @lifetime: owned
    static Rc<T> new_(T value) {
        return Rc<T>(new ControlBlock(std::move(value)));
    }
    
    // C++-friendly factory method (kept for compatibility)
    // @lifetime: owned
    static Rc<T> make(T value) {
        return Rc<T>(new ControlBlock(std::move(value)));
    }
    
    // Private constructor from control block
    explicit Rc(ControlBlock* p) : ptr(p) {}
    
    // Private constructor for weak upgrade (with increment flag)
    Rc(ControlBlock* p, bool increment) : ptr(p) {
        if (increment && ptr) {
            ++ptr->ref_count;
        }
    }
    
    // Copy constructor - increases reference count
    Rc(const Rc& other) : ptr(other.ptr) {
        increment();
    }
    
    // Move constructor - no ref count change
    Rc(Rc&& other) noexcept : ptr(other.ptr) {
        other.ptr = nullptr;
    }
    
    // Copy assignment
    Rc& operator=(const Rc& other) {
        if (this != &other) {
            decrement();
            ptr = other.ptr;
            increment();
        }
        return *this;
    }
    
    // Move assignment
    Rc& operator=(Rc&& other) noexcept {
        if (this != &other) {
            decrement();
            ptr = other.ptr;
            other.ptr = nullptr;
        }
        return *this;
    }
    
    // Destructor
    ~Rc() {
        decrement();
    }
    
    // Dereference - get immutable reference
    // @lifetime: (&'a) -> &'a
    const T& operator*() const {
        assert(ptr != nullptr);
        return ptr->value;
    }
    
    // Arrow operator - access members
    // @lifetime: (&'a) -> &'a
    const T* operator->() const {
        assert(ptr != nullptr);
        return &ptr->value;
    }
    
    // Get raw pointer
    // @lifetime: (&'a) -> &'a
    const T* get() const {
        return ptr ? &ptr->value : nullptr;
    }
    
    // Check if Rc contains a value
    bool is_valid() const {
        return ptr != nullptr;
    }
    
    // Explicit bool conversion
    explicit operator bool() const {
        return is_valid();
    }
    
    // Get current reference count
    size_t strong_count() const {
        return ptr ? ptr->ref_count : 0;
    }
    
    // Clone - explicitly create a new Rc to the same value
    Rc clone() const {
        return Rc(*this);
    }
    
    // Try to get mutable reference if we're the only owner
    // Returns nullptr if there are other references
    // @lifetime: (&'a mut) -> &'a mut
    T* get_mut() {
        if (ptr && ptr->ref_count == 1) {
            return &ptr->value;
        }
        return nullptr;
    }
    
    // Create a new Rc with the same value (deep copy)
    // Requires T to be copyable
    Rc<T> make_unique() const {
        if (ptr) {
            return Rc<T>::new_(ptr->value);
        }
        return Rc<T>();
    }
};

// Factory function for creating Rc
template<typename T, typename... Args>
// @lifetime: owned
Rc<T> make_rc(Args&&... args) {
    return Rc<T>::make(T(std::forward<Args>(args)...));
}

// Weak reference support moved to weak.hpp and rc/weak.hpp

} // namespace rusty

#endif // RUSTY_RC_HPP