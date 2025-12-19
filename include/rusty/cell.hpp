#ifndef RUSTY_CELL_HPP
#define RUSTY_CELL_HPP

#include <utility>
#include <type_traits>
#include "unsafe_cell.hpp"

// Cell<T> - Interior mutability for Copy types
// Provides interior mutability for types that implement Copy
//
// Guarantees:
// - Single-threaded only (not thread-safe)
// - No runtime borrow checking (unlike RefCell)
// - Only for types that can be copied bitwise
// - Zero overhead - uses UnsafeCell internally

// @safe
namespace rusty {

// @safe - Cell provides interior mutability for Copy types
template<typename T>
class Cell {
    static_assert(std::is_trivially_copyable_v<T>,
                  "Cell<T> requires T to be trivially copyable (similar to Rust's Copy trait)");
private:
    UnsafeCell<T> value;

public:
    // @safe - Constructors
    Cell() : value() {}
    // @safe
    explicit Cell(T val) : value(val) {}

    // @safe - Get a copy of the value
    // @lifetime: (&'a) -> T
    T get() const {
        // @unsafe
        { return *value.get(); }
    }

    // @safe - Set the value
    // @lifetime: (&'a, T) -> void
    void set(T val) const {
        // @unsafe
        { *value.get() = val; }
    }

    // @safe - Replace the value and return the old one
    // @lifetime: (&'a, T) -> T
    T replace(T val) const {
        // @unsafe
        {
            T old = *value.get();
            *value.get() = val;
            return old;
        }
    }

    // @safe - Swap the values of two cells
    // @lifetime: (&'a, &'a) -> void
    void swap(Cell& other) const {
        // @unsafe
        {
            T temp = *value.get();
            *value.get() = *other.value.get();
            *other.value.get() = temp;
        }
    }

    // @safe - Take the value, leaving Default::default() in its place
    // Only available if T has a default constructor
    template<typename U = T>
    typename std::enable_if_t<std::is_default_constructible_v<U>, T>
    take() const {
        return replace(T{});
    }

    // @unsafe - Get a raw pointer to the value (unsafe in Rust, but needed for C++ interop)
    // @lifetime: (&'a) -> *mut T where return: 'a
    T* get_mut() const {
        return value.get();
    }

    // @safe - Update the value using a function
    // @lifetime: (&'a, F) -> void
    template<typename F>
    void update(F f) const {
        // @unsafe
        {
            T* ptr = value.get();
            *ptr = f(*ptr);
        }
    }

    // No copy or move - Cell itself is not copyable/movable
    // (though the inner value is)
    Cell(const Cell&) = delete;
    Cell& operator=(const Cell&) = delete;
    Cell(Cell&&) = delete;
    Cell& operator=(Cell&&) = delete;
};

// @safe - Helper function to create a Cell
template<typename T>
Cell<T> make_cell(T value) {
    return Cell<T>(value);
}

} // namespace rusty

#endif // RUSTY_CELL_HPP