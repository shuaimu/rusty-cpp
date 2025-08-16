#ifndef RUSTY_BOX_HPP
#define RUSTY_BOX_HPP

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
    Box() : ptr(nullptr) {}
    
    // @lifetime: owned
    explicit Box(T* p) : ptr(p) {}
    
    // Rust-idiomatic factory method - Box::new()
    // @lifetime: owned
    static Box<T> new_(T value) {
        return Box<T>(new T(std::move(value)));
    }
    
    // C++-friendly factory method (kept for compatibility)
    // @lifetime: owned
    static Box<T> make(T value) {
        return Box<T>(new T(std::move(value)));
    }
    
    // No copy constructor - Box cannot be copied
    Box(const Box&) = delete;
    Box& operator=(const Box&) = delete;
    
    // Move constructor - transfers ownership
    // @lifetime: owned
    Box(Box&& other) noexcept : ptr(other.ptr) {
        other.ptr = nullptr;  // Other box becomes empty
    }
    
    // Move assignment - transfers ownership
    // @lifetime: owned
    Box& operator=(Box&& other) noexcept {
        if (this != &other) {
            delete ptr;
            ptr = other.ptr;
            other.ptr = nullptr;
        }
        return *this;
    }
    
    // Destructor - automatic cleanup
    ~Box() {
        delete ptr;
    }
    
    // Dereference - borrow the value
    // @lifetime: (&'a) -> &'a
    T& operator*() {
        // In debug mode, could assert(ptr != nullptr)
        return *ptr;
    }
    
    // @lifetime: (&'a) -> &'a
    const T& operator*() const {
        return *ptr;
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
    
    // Reset with new value
    void reset(T* p = nullptr) {
        delete ptr;
        ptr = p;
    }
};

// Rust-idiomatic factory function
template<typename T, typename... Args>
// @lifetime: owned
Box<T> box(Args&&... args) {
    return Box<T>(new T(std::forward<Args>(args)...));
}

// C++-friendly factory function (kept for compatibility)
template<typename T, typename... Args>
// @lifetime: owned
Box<T> make_box(Args&&... args) {
    return Box<T>(new T(std::forward<Args>(args)...));
}

} // namespace rusty

#endif // RUSTY_BOX_HPP