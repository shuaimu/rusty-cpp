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
#include <cstring>
#include <memory>
#include <type_traits>
#include <utility>
#include "option.hpp"

namespace rusty {

// Ptr<T> - pointer to immutable data (like Rust's *const T)
// Default choice for raw pointers - safer because you can't mutate through it
template<typename T>
using Ptr = const T*;

// MutPtr<T> - pointer to mutable data (like Rust's *mut T)
// Use when you need to modify the pointed-to data
template<typename T>
using MutPtr = T*;

// NonNull<T> - non-null raw pointer wrapper (Rust std::ptr::NonNull analogue)
template<typename T>
class NonNull {
private:
    T* ptr_;
    struct CastProxy {
        T* ptr_;

        template<typename U>
        constexpr operator NonNull<U>() const noexcept {
            return NonNull<U>(reinterpret_cast<U*>(ptr_));
        }
    };

public:
    constexpr explicit NonNull(T* ptr) noexcept : ptr_(ptr) {}

    static Option<NonNull<T>> new_(T* ptr) noexcept {
        if (ptr == nullptr) {
            return Option<NonNull<T>>(None);
        }
        return Option<NonNull<T>>(NonNull<T>(ptr));
    }

    static constexpr NonNull<T> new_unchecked(T* ptr) noexcept {
        return NonNull<T>(ptr);
    }

    constexpr T* as_ptr() const noexcept {
        return ptr_;
    }

    constexpr T& as_mut() noexcept {
        return *ptr_;
    }

    // Rust `NonNull::cast` supports contextual target inference in chains
    // like `NonNull::new(ptr).unwrap().cast()`. The proxy overload keeps
    // that usage valid while the template overload supports explicit targets.
    constexpr CastProxy cast() const noexcept {
        return CastProxy{ptr_};
    }

    template<typename U>
    constexpr NonNull<U> cast() const noexcept {
        return NonNull<U>(reinterpret_cast<U*>(ptr_));
    }

    friend constexpr bool operator==(NonNull<T> lhs, NonNull<T> rhs) noexcept {
        return lhs.ptr_ == rhs.ptr_;
    }

    friend constexpr bool operator!=(NonNull<T> lhs, NonNull<T> rhs) noexcept {
        return !(lhs == rhs);
    }
};

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

// Minimal Rust std::ptr runtime surface used by transpiled expanded output.
namespace ptr {

template<typename T>
using NonNull = ::rusty::NonNull<T>;

template<typename T>
inline T read(const T* src) {
    // Mirror Rust `ptr::read` move-out semantics even from `*const T`-shaped
    // call sites. This surface is intentionally unsafe: callers must guarantee
    // source validity and single-drop discipline.
    return std::move(*const_cast<T*>(src));
}

template<typename T>
inline T read(T* src) {
    return std::move(*src);
}

template<typename T, typename U>
inline void write(T* dst, U&& value) {
    std::construct_at(dst, std::forward<U>(value));
}

template<typename T, typename Count>
inline void copy(const T* src, T* dst, Count count) {
    auto byte_count = static_cast<std::size_t>(count) * sizeof(T);
    std::memmove(static_cast<void*>(dst), static_cast<const void*>(src), byte_count);
}

template<typename Src, typename Dst, typename Count>
inline void copy(const Src* src, Dst* dst, Count count)
requires (!std::is_same_v<std::remove_cv_t<Src>, std::remove_cv_t<Dst>>)
{
    static_assert(sizeof(Src) == sizeof(Dst), "rusty::ptr::copy requires equal element sizes");
    auto byte_count = static_cast<std::size_t>(count) * sizeof(Src);
    std::memmove(static_cast<void*>(dst), static_cast<const void*>(src), byte_count);
}

template<typename T, typename Count>
inline void copy_nonoverlapping(const T* src, T* dst, Count count) {
    auto byte_count = static_cast<std::size_t>(count) * sizeof(T);
    std::memcpy(static_cast<void*>(dst), static_cast<const void*>(src), byte_count);
}

template<typename Src, typename Dst, typename Count>
inline void copy_nonoverlapping(const Src* src, Dst* dst, Count count)
requires (!std::is_same_v<std::remove_cv_t<Src>, std::remove_cv_t<Dst>>)
{
    static_assert(
        sizeof(Src) == sizeof(Dst),
        "rusty::ptr::copy_nonoverlapping requires equal element sizes");
    auto byte_count = static_cast<std::size_t>(count) * sizeof(Src);
    std::memcpy(static_cast<void*>(dst), static_cast<const void*>(src), byte_count);
}

template<typename T, typename Count>
inline const T* add(const T* ptr, Count count) {
    return ptr + static_cast<std::size_t>(count);
}

template<typename T, typename Count>
inline T* add(T* ptr, Count count) {
    return ptr + static_cast<std::size_t>(count);
}

template<typename T, typename Count>
inline const T* sub(const T* ptr, Count count) {
    return ptr - static_cast<std::size_t>(count);
}

template<typename T, typename Count>
inline T* sub(T* ptr, Count count) {
    return ptr - static_cast<std::size_t>(count);
}

template<typename T, typename Count>
inline const T* offset(const T* ptr, Count count) {
    return ptr + static_cast<std::ptrdiff_t>(count);
}

template<typename T, typename Count>
inline T* offset(T* ptr, Count count) {
    return ptr + static_cast<std::ptrdiff_t>(count);
}

template<typename T>
inline void drop_in_place(T* dst) {
    std::destroy_at(dst);
}

template<typename RangeLike>
inline void drop_in_place(RangeLike&& range)
requires requires(RangeLike r) { r.data(); r.size(); }
{
    auto* data = range.data();
    auto count = static_cast<std::size_t>(range.size());
    std::destroy_n(data, count);
}

} // namespace ptr

} // namespace rusty

#endif // RUSTY_PTR_HPP
