#ifndef RUSTY_CELL_HPP
#define RUSTY_CELL_HPP

#include <utility>
#include <type_traits>

// Cell<T> - Interior mutability for Copy types
// Provides interior mutability for types that implement Copy
//
// Guarantees:
// - Single-threaded only (not thread-safe)
// - No runtime borrow checking (unlike RefCell)
// - Only for types that can be copied bitwise
// - Zero overhead - just a mutable field

// @safe
namespace rusty {

template<typename T>
class Cell {
    static_assert(std::is_trivially_copyable_v<T>, 
                  "Cell<T> requires T to be trivially copyable (similar to Rust's Copy trait)");
private:
    mutable T value;
    
public:
    // Constructors
    Cell() : value() {}
    explicit Cell(T val) : value(val) {}
    
    // Get a copy of the value
    // @lifetime: (&'a) -> T
    T get() const {
        return value;
    }
    
    // Set the value
    // @lifetime: (&'a, T) -> void
    void set(T val) const {
        value = val;
    }
    
    // Replace the value and return the old one
    // @lifetime: (&'a, T) -> T
    T replace(T val) const {
        T old = value;
        value = val;
        return old;
    }
    
    // Swap the values of two cells
    // @lifetime: (&'a, &'a) -> void
    void swap(Cell& other) const {
        T temp = value;
        value = other.value;
        other.value = temp;
    }
    
    // Take the value, leaving Default::default() in its place
    // Only available if T has a default constructor
    template<typename U = T>
    typename std::enable_if_t<std::is_default_constructible_v<U>, T>
    take() const {
        return replace(T{});
    }
    
    // Get a raw pointer to the value (unsafe in Rust, but needed for C++ interop)
    // @lifetime: (&'a) -> *mut T where return: 'a
    T* get_mut() const {
        return &value;
    }
    
    // Update the value using a function
    // @lifetime: (&'a, F) -> void
    template<typename F>
    void update(F f) const {
        value = f(value);
    }
    
    // No copy or move - Cell itself is not copyable/movable
    // (though the inner value is)
    Cell(const Cell&) = delete;
    Cell& operator=(const Cell&) = delete;
    Cell(Cell&&) = delete;
    Cell& operator=(Cell&&) = delete;
};

// Helper function to create a Cell
template<typename T>
Cell<T> make_cell(T value) {
    return Cell<T>(value);
}

} // namespace rusty

#endif // RUSTY_CELL_HPP