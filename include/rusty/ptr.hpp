// rusty/ptr.hpp - Safe pointer types for RustyCpp
//
// In Rust, raw pointers come in two flavors:
//   *const T - pointer to immutable data (safer default)
//   *mut T   - pointer to mutable data (explicit)
//
// This header provides C++ equivalents that are SAFE to use in @safe code:
//   Ptr<T>    - const T* (like *const T) - immutable pointee by default
//   MutPtr<T> - T*       (like *mut T)   - explicit mutable pointee
//
// Usage:
//   int x = 42;
//   Ptr<int> p = &x;       // const int* - cannot modify *p
//   MutPtr<int> mp = &x;   // int* - can modify *mp
//
// Rebindability (controlled by const on the pointer itself):
//   const Ptr<int> cp = &x;     // const int* const - non-rebindable, immutable pointee
//   const MutPtr<int> cmp = &x; // int* const - non-rebindable, mutable pointee
//
// All 4 combinations (matching Rust):
//   Ptr<T>          - rebindable, immutable pointee     (let mut r: &T)
//   const Ptr<T>    - non-rebindable, immutable pointee (let r: &T)
//   MutPtr<T>       - rebindable, mutable pointee       (let mut r: &mut T)
//   const MutPtr<T> - non-rebindable, mutable pointee   (let r: &mut T)
//
// SAFETY: Ptr<T> and MutPtr<T> are SAFE to use in @safe code.
// Raw C++ pointers (T*, const T*) still require @unsafe.

#ifndef RUSTY_PTR_HPP
#define RUSTY_PTR_HPP

#include <cstddef>  // for std::ptrdiff_t

namespace rusty {

// Ptr<T> - pointer to immutable data (like Rust's *const T)
// Default choice for raw pointers - safer because you can't mutate through it
template<typename T>
using Ptr = const T*;

// MutPtr<T> - pointer to mutable data (like Rust's *mut T)
// Use when you need to modify the pointed-to data
template<typename T>
using MutPtr = T*;

// Null pointer constants for explicit null initialization
template<typename T>
constexpr Ptr<T> null_ptr = nullptr;

template<typename T>
constexpr MutPtr<T> null_mut_ptr = nullptr;

// Helper functions for pointer creation
// These make the intent explicit when taking addresses
// Note: These functions are safe to CALL but use internal @unsafe blocks
// because they perform operations that would be unsafe in general code.
// This is safe because:
// - addr_of/addr_of_mut take references, which are guaranteed non-null and valid
// - The address-of on a reference is guaranteed to be a valid pointer

// @safe
template<typename T>
constexpr Ptr<T> addr_of(const T& value) noexcept {
    // @unsafe { address-of on reference parameter is safe - reference guarantees validity }
    return &value;
}

// @safe
template<typename T>
constexpr MutPtr<T> addr_of_mut(T& value) noexcept {
    // @unsafe { address-of on reference parameter is safe - reference guarantees validity }
    return &value;
}

// Conversion helpers

// @unsafe - casting away const is dangerous
template<typename T>
constexpr MutPtr<T> as_mut(Ptr<T> ptr) noexcept {
    return const_cast<MutPtr<T>>(ptr);
}

// @safe - adding const is always safe
template<typename T>
constexpr Ptr<T> as_const(MutPtr<T> ptr) noexcept {
    return ptr;
}

// Pointer arithmetic helpers
// Note: These functions are safe to CALL because they work on Ptr<T>/MutPtr<T>
// which are guaranteed valid. The internal pointer arithmetic is in @unsafe blocks.

// @safe
template<typename T>
constexpr Ptr<T> offset(Ptr<T> ptr, std::ptrdiff_t count) noexcept {
    // @unsafe
    {
        return ptr + count;  // pointer arithmetic - caller guarantees bounds
    }
}

// @safe
template<typename T>
constexpr MutPtr<T> offset_mut(MutPtr<T> ptr, std::ptrdiff_t count) noexcept {
    // @unsafe
    {
        return ptr + count;  // pointer arithmetic - caller guarantees bounds
    }
}

} // namespace rusty

#endif // RUSTY_PTR_HPP
