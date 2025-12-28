#ifndef RUSTY_RC_UNIFIED_HPP
#define RUSTY_RC_UNIFIED_HPP

#include <cassert>
#include <cstddef>
#include <utility>
#include <type_traits>
#include "option.hpp"  // For Option<Rc<T>> return types

// Rc<T> - Reference Counted pointer with polymorphism support (single-threaded)
// Equivalent to Rust's Rc<T> with std::shared_ptr-like type conversions
//
// Supports implicit conversions like std::shared_ptr:
//   Rc<Derived> derived = Rc<Derived>::make(...);
//   Rc<Base> base = derived;  // Implicit upcast
//   base->virtual_method();    // Virtual dispatch works!
//
// Guarantees:
// - Single-threaded reference counting (non-atomic)
// - Shared ownership within a thread
// - Automatic deallocation when last Rc is dropped
// - Polymorphic dispatch through base class pointers
// - Proper cleanup of derived types
//
// WARNING: Not thread-safe! Use Arc for multi-threaded scenarios

// @safe
namespace rusty {

// Forward declarations
template<typename T> class Rc;

namespace rc {
    template<typename T> class Weak;
    template<typename T> Weak<T> downgrade(const Rc<T>&);
}

template<typename T> rc::Weak<T> downgrade(const Rc<T>&);

// Type-erased control block base for polymorphism (shared across all Rc<T>)
struct RcControlBlockBase {
    size_t strong_count;
    size_t weak_count;

    RcControlBlockBase()
        : strong_count(1), weak_count(1) {}

    virtual ~RcControlBlockBase() = default;

    // Pure virtual - derived control block handles destruction
    virtual void destroy_value() = 0;

    // Pure virtual - get value pointer for Weak::upgrade()
    virtual void* get_value_ptr() const = 0;
};

// Typed control block - knows the actual allocated type
template<typename U>
struct RcControlBlock : RcControlBlockBase {
    U* value;

    template<typename... Args>
    RcControlBlock(Args&&... args)
        : RcControlBlockBase(),
          value(new U(std::forward<Args>(args)...)) {}

    ~RcControlBlock() override {
        // Control block destructor - value should already be destroyed
    }

    void destroy_value() override {
        if (value) {
            delete value;
            value = nullptr;
        }
    }

    void* get_value_ptr() const override {
        return static_cast<void*>(value);
    }
};

// @unsafe - Raw pointer operations and manual reference counting
template<typename T>
class Rc {
public:
    // Expose ControlBlock type for Weak<T> compatibility
    using ControlBlock = RcControlBlockBase;

private:
    template<typename U>
    friend class Rc;  // Allow Rc<U> to access Rc<T> private members

    template<typename U>
    friend class rc::Weak;

    template<typename U>
    friend rc::Weak<U> rc::downgrade(const Rc<U>&);

    template<typename U>
    friend rc::Weak<U> downgrade(const Rc<U>&);

    // Rc stores:
    // 1. Pointer to T (which might be Base when Rc<Derived> was converted to Rc<Base>)
    // 2. Type-erased control block (which knows actual allocated type U)
    T* ptr_;                        // Pointer to T (or Base)
    RcControlBlockBase* control_;   // Type-erased control block

    // @unsafe
    static void release_weak(RcControlBlockBase* cb) {
        // @unsafe {
        if (cb && --cb->weak_count == 0) {
            delete cb;
        }
        // }
    }

    // @unsafe
    void increment_strong() {
        // @unsafe {
        if (control_) {
            ++control_->strong_count;
        }
        // }
    }

    // @unsafe
    void decrement_strong() {
        // @unsafe {
        if (!control_) {
            return;
        }
        if (--control_->strong_count == 0) {
            control_->destroy_value();
            ptr_ = nullptr;
            release_weak(control_);
        }
        control_ = nullptr;
        ptr_ = nullptr;
        // }
    }

    // Private constructor from typed control block
    template<typename U>
    explicit Rc(RcControlBlock<U>* cb, U* ptr)
        : ptr_(ptr), control_(cb) {}

    // Aliasing constructor - shares control block but points to different type
    // This is the key to polymorphism support!
    template<typename U>
    Rc(const Rc<U>& other, T* ptr)
        : ptr_(ptr), control_(other.control_) {
        increment_strong();
    }

public:
    // No default constructor - Rc must always own a value (Rust-idiomatic)
    // Use Option<Rc<T>> for nullable Rc
    Rc() = delete;

    // Primary factory method - constructs T with given arguments
    // @safe - Public API is safe, internal allocation is encapsulated
    // @lifetime: owned
    template<typename... Args>
    static Rc<T> make(Args&&... args) {
        // @unsafe - new allocation
        {
            auto* cb = new RcControlBlock<T>(std::forward<Args>(args)...);
            return Rc<T>(cb, cb->value);
        }
    }

    // @safe - Copy constructor increases reference count
    Rc(const Rc& other)
        : ptr_(other.ptr_), control_(other.control_) {
        // @unsafe - reference count manipulation
        { increment_strong(); }
    }

    // @safe - Converting copy constructor allows Rc<Derived> -> Rc<Base>
    // This is what enables polymorphism!
    template<typename U, typename = std::enable_if_t<std::is_convertible<U*, T*>::value>>
    Rc(const Rc<U>& other)
        : ptr_(other.ptr_), control_(other.control_) {
        // @unsafe - reference count manipulation
        { increment_strong(); }
    }

    // Move constructor - no ref count change
    Rc(Rc&& other) noexcept
        : ptr_(other.ptr_), control_(other.control_) {
        other.ptr_ = nullptr;
        other.control_ = nullptr;
    }

    // Converting move constructor - allows Rc<Derived> -> Rc<Base>
    template<typename U, typename = std::enable_if_t<std::is_convertible<U*, T*>::value>>
    Rc(Rc<U>&& other) noexcept
        : ptr_(other.ptr_), control_(other.control_) {
        other.ptr_ = nullptr;
        other.control_ = nullptr;
    }

    // Copy assignment
    // @unsafe
    Rc& operator=(const Rc& other) {
        // @unsafe {
        if (this != &other) {
            decrement_strong();
            ptr_ = other.ptr_;
            control_ = other.control_;
            increment_strong();
        }
        return *this;
        // }
    }

    // Converting copy assignment
    // @unsafe
    template<typename U, typename = std::enable_if_t<std::is_convertible<U*, T*>::value>>
    Rc& operator=(const Rc<U>& other) {
        // @unsafe {
        decrement_strong();
        ptr_ = other.ptr_;
        control_ = other.control_;
        increment_strong();
        return *this;
        // }
    }

    // Move assignment
    // @unsafe
    Rc& operator=(Rc&& other) noexcept {
        // @unsafe {
        if (this != &other) {
            decrement_strong();
            ptr_ = other.ptr_;
            control_ = other.control_;
            other.ptr_ = nullptr;
            other.control_ = nullptr;
        }
        return *this;
        // }
    }

    // Converting move assignment
    // @unsafe
    template<typename U, typename = std::enable_if_t<std::is_convertible<U*, T*>::value>>
    Rc& operator=(Rc<U>&& other) noexcept {
        // @unsafe {
        decrement_strong();
        ptr_ = other.ptr_;
        control_ = other.control_;
        other.ptr_ = nullptr;
        other.control_ = nullptr;
        return *this;
        // }
    }

    // Destructor
    ~Rc() {
        decrement_strong();
    }

    // @safe - Dereference to get immutable reference
    // @lifetime: (&'a) -> &'a
    const T& operator*() const {
        // @unsafe - assert and pointer dereference
        {
            assert(ptr_ != nullptr);
            return *ptr_;
        }
    }

    // @safe - Arrow operator accesses members (supports virtual dispatch!)
    // @lifetime: (&'a) -> &'a
    const T* operator->() const {
        // @unsafe - assert and pointer access
        {
            assert(ptr_ != nullptr);
            return ptr_;
        }
    }

    // @safe - Get raw pointer (const, so safe to return)
    // @lifetime: (&'a) -> &'a
    const T* get() const {
        // @unsafe - raw pointer access
        { return ptr_; }
    }

    // Check if Rc contains a value
    bool is_valid() const {
        return ptr_ != nullptr && control_ != nullptr;
    }

    // Explicit bool conversion
    explicit operator bool() const {
        return is_valid();
    }

    // Get current strong reference count
    size_t strong_count() const {
        return control_ ? control_->strong_count : 0;
    }

    // Get current weak reference count (excluding implicit strong-held weak)
    size_t weak_count() const {
        return control_ ? (control_->weak_count > 0 ? control_->weak_count - 1 : 0) : 0;
    }

    // @safe - Clone explicitly creates a new Rc to the same value
    Rc clone() const {
        return Rc(*this);
    }

    // Try to get mutable reference if we're the only owner
    // Returns nullptr if there are other references
    // @lifetime: (&'a mut) -> &'a mut
    T* get_mut() {
        if (control_ && ptr_ && control_->strong_count == 1) {
            return ptr_;
        }
        return nullptr;
    }

    // Create a new Rc with the same value (deep copy)
    // Requires T to be copyable
    // Returns None if Rc is invalid
    Option<Rc<T>> make_unique() const {
        if (ptr_ && control_) {
            return Some(Rc<T>::make(*ptr_));
        }
        return None;
    }

    // Static cast to derived type - unsafe, like std::static_pointer_cast
    // @unsafe
    template<typename U>
    Rc<U> static_pointer_cast() const {
        // @unsafe {
        U* casted = static_cast<U*>(ptr_);
        return Rc<U>(casted, control_, true);  // Use private constructor with increment
        // }
    }

    // Dynamic cast to derived type - returns None on failure
    // @unsafe
    template<typename U>
    Option<Rc<U>> dynamic_pointer_cast() const {
        // @unsafe {
        U* casted = dynamic_cast<U*>(ptr_);
        if (casted) {
            return Some(Rc<U>(casted, control_, true));  // Use private constructor with increment
        }
        return None;
        // }
    }

    // Private constructor for weak upgrade (with increment flag)
    Rc(T* ptr, RcControlBlockBase* control, bool increment_strong_count)
        : ptr_(ptr), control_(control) {
        if (increment_strong_count && control_) {
            ++control_->strong_count;
        }
    }
};

// @safe - Factory function for creating Rc
template<typename T, typename... Args>
// @lifetime: owned
Rc<T> make_rc(Args&&... args) {
    return Rc<T>::make(std::forward<Args>(args)...);
}

// @safe - Comparison operators for Rc<T> (needed for std::set and std::map)
template<typename T>
bool operator<(const Rc<T>& lhs, const Rc<T>& rhs) {
    return lhs.get() < rhs.get();
}

// @safe
template<typename T>
bool operator==(const Rc<T>& lhs, const Rc<T>& rhs) {
    return lhs.get() == rhs.get();
}

// @safe
template<typename T>
bool operator!=(const Rc<T>& lhs, const Rc<T>& rhs) {
    return !(lhs == rhs);
}

// @safe
template<typename T>
bool operator<=(const Rc<T>& lhs, const Rc<T>& rhs) {
    return !(rhs < lhs);
}

// @safe
template<typename T>
bool operator>(const Rc<T>& lhs, const Rc<T>& rhs) {
    return rhs < lhs;
}

// @safe
template<typename T>
bool operator>=(const Rc<T>& lhs, const Rc<T>& rhs) {
    return !(lhs < rhs);
}

} // namespace rusty

#endif // RUSTY_RC_UNIFIED_HPP
