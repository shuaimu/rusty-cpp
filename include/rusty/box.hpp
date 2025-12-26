#ifndef RUSTY_BOX_HPP
#define RUSTY_BOX_HPP

#include <type_traits>  // for std::enable_if, std::is_convertible, std::is_same
#include <utility>  // for std::move, std::forward

// Box<T> - A smart pointer for heap-allocated values with single ownership
// Equivalent to Rust's Box<T>
//
// Guarantees:
// - Single ownership (no copying)
// - Automatic deallocation when Box goes out of scope
// - Move semantics only
// - Null state after move

// @safe
namespace rusty {

template<typename T>
class Box {
private:
    T* ptr;
    
public:
    // Constructors
    // No default constructor - Box must always own a value (non-nullable)
    Box() = delete;

    // @lifetime: owned
    explicit Box(T* p) : ptr(p) {}

    // Factory method - Box::make()
    // @lifetime: owned
    static Box<T> make(T value) {
        // @unsafe
        {
            // new and std::move are unsafe operations
            return Box<T>(new T(std::move(value)));
        }
    }
    
    // No copy constructor - Box cannot be copied
    Box(const Box&) = delete;
    Box& operator=(const Box&) = delete;
    
    // Move constructor - transfers ownership
    // @lifetime: owned
    Box(Box&& other) noexcept : ptr(other.ptr) {
        other.ptr = nullptr;  // Other box becomes empty
    }

    // Converting move constructor - allows Box<Derived> to convert to Box<Base>
    // Only enabled when U* is convertible to T* (i.e., U derives from T)
    // @lifetime: owned
    template<typename U, typename = typename std::enable_if<
        std::is_convertible<U*, T*>::value && !std::is_same<U, T>::value>::type>
    Box(Box<U>&& other) noexcept : ptr(other.release()) {}

    // Move assignment - transfers ownership
    // @lifetime: owned
    Box& operator=(Box&& other) noexcept {
        // @unsafe
        {
            if (this != &other) {
                delete ptr;
                ptr = other.ptr;
                other.ptr = nullptr;
            }
            return *this;
        }
    }

    // Converting move assignment - allows Box<Derived> to assign to Box<Base>
    // @lifetime: owned
    template<typename U, typename = typename std::enable_if<
        std::is_convertible<U*, T*>::value && !std::is_same<U, T>::value>::type>
    Box& operator=(Box<U>&& other) noexcept {
        // @unsafe
        {
            delete ptr;
            ptr = other.release();
            return *this;
        }
    }

    // Destructor - automatic cleanup
    ~Box() {
        // @unsafe
        {
            delete ptr;
        }
    }
    
    // Dereference - borrow the value
    // @lifetime: (&'a) -> &'a
    T& operator*() {
        // @unsafe
        {
            // Pointer dereference is unsafe, but Box guarantees ptr is valid
            return *ptr;
        }
    }

    // @lifetime: (&'a) -> &'a
    const T& operator*() const {
        // @unsafe
        {
            return *ptr;
        }
    }

    // Arrow operator - access members
    // @lifetime: (&'a) -> &'a
    T* operator->() {
        return ptr;
    }

    // @lifetime: (&'a) -> &'a
    const T* operator->() const {
        return ptr;
    }
    
    // Check if box contains a value
    bool is_valid() const {
        return ptr != nullptr;
    }
    
    // Explicit bool conversion
    explicit operator bool() const {
        return is_valid();
    }
    
    // Take ownership of the raw pointer (Rust: Box::into_raw)
    // After this, the Box is empty and caller is responsible for deletion
    // @lifetime: owned
    T* into_raw() {
        T* temp = ptr;
        ptr = nullptr;
        return temp;
    }
    
    // C++-style alias for into_raw
    // @lifetime: owned
    T* release() {
        return into_raw();
    }
    
    // Get raw pointer without transferring ownership
    // @lifetime: (&'a) -> &'a
    T* get() const {
        return ptr;
    }

    // Note: No reset() method - Box is non-nullable like Rust's Box<T>
    // To replace the value, use assignment: box = Box::make(new_value)
    // To destroy, let it go out of scope or use std::move
};

// Factory function following C++ make_* convention
template<typename T, typename... Args>
// @lifetime: owned
Box<T> make_box(Args&&... args) {
    // @unsafe
    {
        // new and std::forward are unsafe operations
        return Box<T>(new T(std::forward<Args>(args)...));
    }
}

} // namespace rusty

#endif // RUSTY_BOX_HPP