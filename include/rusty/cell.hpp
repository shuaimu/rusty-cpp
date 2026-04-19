#ifndef RUSTY_CELL_HPP
#define RUSTY_CELL_HPP

#include <utility>
#include <type_traits>
#include "unsafe_cell.hpp"

// Cell<T> - Interior mutability
// Provides interior mutability with Rust-like `Cell` surfaces.
//
// Guarantees:
// - Single-threaded only (not thread-safe)
// - No runtime borrow checking (unlike RefCell)
// - Zero overhead - uses UnsafeCell internally

// @safe
namespace rusty {

// @safe - Cell provides interior mutability for Copy types
template<typename T>
class Cell {
private:
    UnsafeCell<T> value;

public:
    // @safe - Constructors
    Cell() : value() {}
    // @safe
    explicit Cell(T val) : value(std::move(val)) {}
    // @safe - Rust-style constructor path used by transpiled code (`Cell::new(...)`)
    static Cell new_(T val) { return Cell(std::move(val)); }

    // @safe - Get a copy of the value (available only for copyable payloads)
    // @lifetime: (&'a) -> T
    template<typename U = T>
    typename std::enable_if_t<std::is_copy_constructible_v<U>, U>
    get() const {
        // @unsafe
        { return *value.get(); }
    }

    // @safe - Set the value
    // @lifetime: (&'a, T) -> void
    void set(T val) const {
        // @unsafe
        { *value.get() = std::move(val); }
    }

    // @safe - Replace the value and return the old one
    // @lifetime: (&'a, T) -> T
    T replace(T val) const {
        // @unsafe
        {
            T old = std::move(*value.get());
            *value.get() = std::move(val);
            return old;
        }
    }

    // @safe - Swap the values of two cells
    // @lifetime: (&'a, &'a) -> void
    void swap(Cell& other) const {
        // @unsafe
        {
            using std::swap;
            swap(*value.get(), *other.value.get());
        }
    }

    // @safe - Take the value, leaving Default::default() in its place
    // Only available if T has a default constructor
    template<typename U = T>
    typename std::enable_if_t<std::is_default_constructible_v<U>, T>
    take() const {
        return replace(T{});
    }

    // @safe - Clone the cell by copying inner value when copyable.
    template<typename U = T>
    typename std::enable_if_t<std::is_copy_constructible_v<U>, Cell>
    clone() const {
        return Cell(get());
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

    // No copy - copying a Cell would create aliased interior-mutable state.
    Cell(const Cell&) = delete;
    Cell& operator=(const Cell&) = delete;
    // Move is allowed (matches Rust move semantics for Cell<T>).
    Cell(Cell&&) = default;
    Cell& operator=(Cell&&) = default;
};

// @safe - Helper function to create a Cell
template<typename T>
Cell<T> make_cell(T value) {
    return Cell<T>(value);
}

} // namespace rusty

#endif // RUSTY_CELL_HPP
