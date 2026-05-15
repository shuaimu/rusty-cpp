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
#include "mem.hpp"
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

    // Rust's `NonNull::from(&mut T)` / `NonNull::from(&T)` converts a
    // borrow into a non-null raw-pointer wrapper. In transpiled C++ the
    // `&mut T` argument arrives as a `T*` (e.g. the return of
    // `Box::leak(b)`), so the most useful spelling here is `from(T*)`,
    // accepting an already non-null pointer.
    // @unsafe
    static constexpr NonNull<T> from(T* ptr) noexcept {
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

inline constexpr std::nullptr_t null_mut() noexcept {
    return nullptr;
}

template<typename T>
inline constexpr T* cast_mut(const T* ptr) noexcept {
    return const_cast<T*>(ptr);
}

template<typename T>
inline constexpr T* cast_mut(const T& value) noexcept {
    return const_cast<T*>(&value);
}

template<typename T>
inline constexpr const T* cast_const(T* ptr) noexcept {
    return ptr;
}

template<typename T>
inline constexpr const T* cast_const(T& value) noexcept {
    return &value;
}

template<typename T>
inline Option<const T&> as_ref(const T* ptr) {
    if (ptr == nullptr) {
        return Option<const T&>(None);
    }
    return Option<const T&>(*ptr);
}

template<typename T>
inline Option<T&> as_mut(T* ptr) {
    if (ptr == nullptr) {
        return Option<T&>(None);
    }
    return Option<T&>(*ptr);
}

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
    // A moved-from value may have left a forgotten-address marker at `dst`.
    // Writing a fresh value into that slot must clear stale marker state so
    // the new object's destructor is not skipped.
    rusty::mem::clear_forgotten_address_range(static_cast<const void*>(dst), sizeof(T));
    std::construct_at(dst, std::forward<U>(value));
}

// Some generated call sites may carry escaped identifier spellings (`write_`)
// when traversing generic path-lowering code paths. Keep a forwarding alias
// so both spellings map to Rust `ptr::write` semantics.
template<typename T, typename U>
inline void write_(T* dst, U&& value) {
    write(dst, std::forward<U>(value));
}

template<typename T>
inline T read_unaligned(const T* src) {
    T value;
    std::memcpy(&value, src, sizeof(T));
    return value;
}

template<typename T, typename U>
inline void write_unaligned(T* dst, U&& value) {
    T tmp = static_cast<T>(std::forward<U>(value));
    std::memcpy(dst, &tmp, sizeof(T));
}

template<typename T, typename U, typename Count>
inline void write_bytes(T* dst, U value, Count count) {
    const auto byte_value = static_cast<unsigned char>(value);
    const auto byte_count = static_cast<std::size_t>(count) * sizeof(T);
    std::memset(static_cast<void*>(dst), byte_value, byte_count);
}

template<typename T, typename Count>
inline void copy(const T* src, T* dst, Count count) {
    const auto element_count = static_cast<std::size_t>(count);
    if (element_count == 0 || src == dst) {
        return;
    }
    if constexpr (std::is_trivially_copyable_v<T>) {
        auto byte_count = element_count * sizeof(T);
        std::memmove(static_cast<void*>(dst), static_cast<const void*>(src), byte_count);
    } else if (dst < src) {
        // Left-shift overlap patterns (`dst` before `src`) move low→high.
        for (std::size_t i = 0; i < element_count; ++i) {
            T* const dst_i = dst + i;
            T* const src_i = const_cast<T*>(src) + i;
            // Destination is currently initialized; destroy it unless a prior
            // move already marked the slot as forgotten.
            if (!rusty::mem::consume_forgotten_address(static_cast<const void*>(dst_i))) {
                std::destroy_at(dst_i);
            }
            std::construct_at(dst_i, std::move(*src_i));
            // Model move-shift semantics for drop-sensitive generated types.
            rusty::mem::mark_forgotten_address(static_cast<const void*>(src_i));
        }
    } else {
        // Right-shift overlap patterns need reverse order. Slots beyond source
        // coverage are newly opened holes and require placement construction.
        for (std::size_t i = element_count; i-- > 0;) {
            T* const dst_i = dst + i;
            T* const src_i = const_cast<T*>(src) + i;
            if (dst_i < src + element_count) {
                // Destination currently holds an object from the source window.
                if (!rusty::mem::consume_forgotten_address(static_cast<const void*>(dst_i))) {
                    std::destroy_at(dst_i);
                }
            } else {
                // Newly opened hole: ensure stale marker state cannot leak to
                // the newly constructed destination object.
                rusty::mem::clear_forgotten_address_range(
                    static_cast<const void*>(dst_i), sizeof(T));
            }
            std::construct_at(dst_i, std::move(*src_i));
            // Rust `ptr::copy` move-shift callers treat this source slot as logically
            // uninitialized after the move. Mark it forgotten so any later destructor
            // path skips running user drop code on moved-from state.
            rusty::mem::mark_forgotten_address(static_cast<const void*>(src_i));
        }
    }
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
    const auto element_count = static_cast<std::size_t>(count);
    if (element_count == 0) {
        return;
    }
    if constexpr (std::is_trivially_copyable_v<T>) {
        auto byte_count = element_count * sizeof(T);
        std::memcpy(static_cast<void*>(dst), static_cast<const void*>(src), byte_count);
    } else {
        for (std::size_t i = 0; i < element_count; ++i) {
            std::construct_at(dst + i, std::move(const_cast<T&>(src[i])));
        }
    }
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
