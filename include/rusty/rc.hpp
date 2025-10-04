#ifndef RUSTY_RC_HPP
#define RUSTY_RC_HPP

#include <cassert>
#include <cstddef>
#include <utility>

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
namespace rc {
    template<typename T> class Weak;
    template<typename T> Weak<T> downgrade(const Rc<T>&);
}
template<typename T> rc::Weak<T> downgrade(const Rc<T>&);

template<typename T>
class Rc {
private:
    template<typename U>
    friend class rc::Weak;
    template<typename U>
    friend rc::Weak<U> rc::downgrade(const Rc<U>&);
    template<typename U>
    friend rc::Weak<U> downgrade(const Rc<U>&);

    struct ControlBlock {
        T* value;
        size_t strong_count;
        size_t weak_count;

        template<typename... Args>
        ControlBlock(Args&&... args)
            : value(new T(std::forward<Args>(args)...)),
              strong_count(1),
              weak_count(1) {}

        ~ControlBlock() {
            delete value;
        }
    };

    ControlBlock* ptr;

    static void release_weak(ControlBlock* cb) {
        if (cb && --cb->weak_count == 0) {
            delete cb;
        }
    }

    void increment_strong() {
        if (ptr) {
            ++ptr->strong_count;
        }
    }

    void decrement_strong() {
        if (!ptr) {
            return;
        }
        ControlBlock* current = ptr;
        if (--current->strong_count == 0) {
            delete current->value;
            current->value = nullptr;
            release_weak(current);
        }
        ptr = nullptr;
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

    // Placement factory for in-place construction
    // @lifetime: owned
    template<typename... Args>
    static Rc<T> make_in_place(Args&&... args) {
        return Rc<T>(new ControlBlock(std::forward<Args>(args)...));
    }

    // Private constructor from control block
    explicit Rc(ControlBlock* p) : ptr(p) {}

    // Private constructor for weak upgrade (with increment flag)
    Rc(ControlBlock* p, bool increment) : ptr(p) {
        if (increment && ptr) {
            ++ptr->strong_count;
        }
    }

    // Copy constructor - increases reference count
    Rc(const Rc& other) : ptr(other.ptr) {
        increment_strong();
    }

    // Move constructor - no ref count change
    Rc(Rc&& other) noexcept : ptr(other.ptr) {
        other.ptr = nullptr;
    }

    // Copy assignment
    Rc& operator=(const Rc& other) {
        if (this != &other) {
            decrement_strong();
            ptr = other.ptr;
            increment_strong();
        }
        return *this;
    }

    // Move assignment
    Rc& operator=(Rc&& other) noexcept {
        if (this != &other) {
            decrement_strong();
            ptr = other.ptr;
            other.ptr = nullptr;
        }
        return *this;
    }

    // Destructor
    ~Rc() {
        decrement_strong();
    }

    // Dereference - get immutable reference
    // @lifetime: (&'a) -> &'a
    const T& operator*() const {
        assert(ptr != nullptr && ptr->value != nullptr);
        return *ptr->value;
    }

    // Arrow operator - access members
    // @lifetime: (&'a) -> &'a
    const T* operator->() const {
        assert(ptr != nullptr && ptr->value != nullptr);
        return ptr->value;
    }

    // Get raw pointer
    // @lifetime: (&'a) -> &'a
    const T* get() const {
        return (ptr && ptr->value) ? ptr->value : nullptr;
    }

    // Check if Rc contains a value
    bool is_valid() const {
        return ptr != nullptr && ptr->value != nullptr;
    }

    // Explicit bool conversion
    explicit operator bool() const {
        return is_valid();
    }

    // Get current strong reference count
    size_t strong_count() const {
        return ptr ? ptr->strong_count : 0;
    }

    // Get current weak reference count (excluding implicit strong-held weak)
    size_t weak_count() const {
        return ptr ? (ptr->weak_count > 0 ? ptr->weak_count - 1 : 0) : 0;
    }

    // Clone - explicitly create a new Rc to the same value
    Rc clone() const {
        return Rc(*this);
    }

    // Try to get mutable reference if we're the only owner
    // Returns nullptr if there are other references
    // @lifetime: (&'a mut) -> &'a mut
    T* get_mut() {
        if (ptr && ptr->value && ptr->strong_count == 1) {
            return ptr->value;
        }
        return nullptr;
    }

    // Create a new Rc with the same value (deep copy)
    // Requires T to be copyable
    Rc<T> make_unique() const {
        if (ptr && ptr->value) {
            return Rc<T>::new_(*ptr->value);
        }
        return Rc<T>();
    }
};

// Factory function for creating Rc
template<typename T, typename... Args>
// @lifetime: owned
Rc<T> make_rc(Args&&... args) {
    return Rc<T>::new_(T(std::forward<Args>(args)...));
}

// Weak reference support lives in rc/weak.hpp

} // namespace rusty

#endif // RUSTY_RC_HPP
