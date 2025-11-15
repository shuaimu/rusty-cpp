#ifndef RUSTY_UNSAFE_CELL_HPP
#define RUSTY_UNSAFE_CELL_HPP

// UnsafeCell<T> - The core primitive for interior mutability
//
// This is the fundamental building block for all interior mutability in Rust.
// Unlike Cell<T> or RefCell<T>, UnsafeCell<T> provides no safety guarantees.
//
// Guarantees:
// - Single-threaded only (not thread-safe)
// - NO borrow checking (unsafe)
// - Direct mutable access to inner value through const methods
// - Zero overhead - just wraps the value
//
// Safety:
// - You MUST ensure no data races or aliasing violations
// - You MUST NOT create multiple mutable references simultaneously
// - You MUST ensure references don't outlive the UnsafeCell

// @safe
namespace rusty {

template<typename T>
class UnsafeCell {
private:
    T value;

public:
    // Constructors
    UnsafeCell() : value() {}
    explicit UnsafeCell(T val) : value(std::move(val)) {}

    // Factory method (Rust-style)
    static UnsafeCell<T> new_(T value) {
        return UnsafeCell<T>(std::move(value));
    }

    // Get a raw mutable pointer to the inner value
    // This is the ONLY way to access the value
    // @lifetime: (&'a) -> *mut T where return: 'a
    // SAFETY: Caller must ensure:
    // 1. No data races (single-threaded access only)
    // 2. No aliasing violations (don't create multiple mutable refs)
    // 3. Returned pointer doesn't outlive the UnsafeCell
    T* get() const {
        return const_cast<T*>(&value);
    }

    // Get a const raw pointer to the inner value (for reading)
    // @lifetime: (&'a) -> *const T where return: 'a
    const T* get_const() const {
        return &value;
    }

    // Take ownership of the value, leaving default in its place
    // Only available if T has a default constructor
    template<typename U = T>
    typename std::enable_if_t<std::is_default_constructible_v<U>, T>
    take() const {
        T old = *get();
        *get() = T{};
        return old;
    }

    // Replace the value and return the old one
    // @lifetime: (&'a, T) -> T
    T replace(T new_value) const {
        T old = *get();
        *get() = new_value;
        return old;
    }

    // No copy or move - UnsafeCell itself is not copyable/movable
    // (This prevents accidental aliasing of the inner pointer)
    UnsafeCell(const UnsafeCell&) = delete;
    UnsafeCell& operator=(const UnsafeCell&) = delete;
    UnsafeCell(UnsafeCell&&) = delete;
    UnsafeCell& operator=(UnsafeCell&&) = delete;
};

// Helper function to create an UnsafeCell
template<typename T>
UnsafeCell<T> make_unsafe_cell(T value) {
    return UnsafeCell<T>(value);
}

} // namespace rusty

#endif // RUSTY_UNSAFE_CELL_HPP
