#ifndef RUSTY_RC_UNIFIED_HPP
#define RUSTY_RC_UNIFIED_HPP

#include <cassert>
#include <cstddef>
#include <utility>
#include <type_traits>

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

    static void release_weak(RcControlBlockBase* cb) {
        if (cb && --cb->weak_count == 0) {
            delete cb;
        }
    }

    void increment_strong() {
        if (control_) {
            ++control_->strong_count;
        }
    }

    void decrement_strong() {
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
    // Default constructor - creates empty Rc
    Rc() : ptr_(nullptr), control_(nullptr) {}

    // Primary factory method - constructs T with given arguments
    // @lifetime: owned
    template<typename... Args>
    static Rc<T> make(Args&&... args) {
        auto* cb = new RcControlBlock<T>(std::forward<Args>(args)...);
        return Rc<T>(cb, cb->value);
    }

    // Rust-idiomatic factory method - Rc::new()
    // @lifetime: owned
    static Rc<T> new_(T value) {
        auto* cb = new RcControlBlock<T>(std::move(value));
        return Rc<T>(cb, cb->value);
    }

    // Copy constructor - increases reference count
    Rc(const Rc& other)
        : ptr_(other.ptr_), control_(other.control_) {
        increment_strong();
    }

    // Converting copy constructor - allows Rc<Derived> -> Rc<Base>
    // This is what enables polymorphism!
    template<typename U, typename = std::enable_if_t<std::is_convertible<U*, T*>::value>>
    Rc(const Rc<U>& other)
        : ptr_(other.ptr_), control_(other.control_) {
        increment_strong();
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
    Rc& operator=(const Rc& other) {
        if (this != &other) {
            decrement_strong();
            ptr_ = other.ptr_;
            control_ = other.control_;
            increment_strong();
        }
        return *this;
    }

    // Converting copy assignment
    template<typename U, typename = std::enable_if_t<std::is_convertible<U*, T*>::value>>
    Rc& operator=(const Rc<U>& other) {
        decrement_strong();
        ptr_ = other.ptr_;
        control_ = other.control_;
        increment_strong();
        return *this;
    }

    // Move assignment
    Rc& operator=(Rc&& other) noexcept {
        if (this != &other) {
            decrement_strong();
            ptr_ = other.ptr_;
            control_ = other.control_;
            other.ptr_ = nullptr;
            other.control_ = nullptr;
        }
        return *this;
    }

    // Converting move assignment
    template<typename U, typename = std::enable_if_t<std::is_convertible<U*, T*>::value>>
    Rc& operator=(Rc<U>&& other) noexcept {
        decrement_strong();
        ptr_ = other.ptr_;
        control_ = other.control_;
        other.ptr_ = nullptr;
        other.control_ = nullptr;
        return *this;
    }

    // Destructor
    ~Rc() {
        decrement_strong();
    }

    // Dereference - get immutable reference
    // @lifetime: (&'a) -> &'a
    const T& operator*() const {
        assert(ptr_ != nullptr);
        return *ptr_;
    }

    // Arrow operator - access members (supports virtual dispatch!)
    // @lifetime: (&'a) -> &'a
    const T* operator->() const {
        assert(ptr_ != nullptr);
        return ptr_;
    }

    // Get raw pointer
    // @lifetime: (&'a) -> &'a
    const T* get() const {
        return ptr_;
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

    // Clone - explicitly create a new Rc to the same value
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
    Rc<T> make_unique() const {
        if (ptr_ && control_) {
            return Rc<T>::new_(*ptr_);
        }
        return Rc<T>();
    }

    // Static cast to derived type - unsafe, like std::static_pointer_cast
    template<typename U>
    Rc<U> static_pointer_cast() const {
        Rc<U> result;
        result.ptr_ = static_cast<U*>(ptr_);
        result.control_ = control_;
        result.increment_strong();
        return result;
    }

    // Dynamic cast to derived type - returns empty Rc on failure
    template<typename U>
    Rc<U> dynamic_pointer_cast() const {
        U* casted = dynamic_cast<U*>(ptr_);
        if (casted) {
            Rc<U> result;
            result.ptr_ = casted;
            result.control_ = control_;
            result.increment_strong();
            return result;
        }
        return Rc<U>();
    }

    // Private constructor for weak upgrade (with increment flag)
    Rc(T* ptr, RcControlBlockBase* control, bool increment_strong_count)
        : ptr_(ptr), control_(control) {
        if (increment_strong_count && control_) {
            ++control_->strong_count;
        }
    }
};

// Factory function for creating Rc
template<typename T, typename... Args>
// @lifetime: owned
Rc<T> make_rc(Args&&... args) {
    return Rc<T>::make(std::forward<Args>(args)...);
}

} // namespace rusty

#endif // RUSTY_RC_UNIFIED_HPP
