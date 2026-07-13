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
#include <span>
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

        // Rust's `NonNull::cast::<U>().as_non_null_ptr()` chain — the
        // `as_non_null_ptr` is identity on NonNull<T>. For the CastProxy
        // form we need an explicit terminator method that the call-site
        // typing tells us the target U. With CTAD we can't get U here;
        // fall back to T (no-op cast).
        constexpr NonNull<T> as_non_null_ptr() const noexcept {
            return NonNull<T>(ptr_);
        }
    };

public:
    // Rust compares NonNull by ADDRESS (PartialEq/Hash on the pointer value,
    // never through the pointee). The universal unwrap helper
    // (`rusty::detail::deref_if_pointer_like`) must therefore pass NonNull
    // through unchanged: dereferencing it to compare would value-compare the
    // pointees — and for tagged-pointer smallstring reprs (semver's
    // Identifier packs string bytes INTO the pointer value) the "address"
    // isn't dereferenceable at all.
    using rusty_pointer_identity_semantics = void;

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

    // Deref access. Rust's NonNull needs unsafe as_ref()/as_mut(), but the
    // transpiler's universal pointer-unwrapping (`deref_if_pointer_like`,
    // `deref_call`) dispatches through `operator*`/`.get()` like Box/Rc —
    // without these, field access through a NonNull-typed place
    // (`(*owned.ptr).sys`) can't lower.
    // @unsafe
    constexpr T& operator*() const noexcept { return *ptr_; }
    // @unsafe
    constexpr T* operator->() const noexcept { return ptr_; }
    // @unsafe
    constexpr T* get() const noexcept { return ptr_; }

    // Rust `NonNull<T>` pointer-arithmetic / read methods the transpiled
    // stdlib port (vec::IntoIter etc.) calls directly. The port's NonNull
    // omitted them; they are thin wrappers over the raw pointer.
    // A default ctor is needed for `decltype(ptr) x{};` late-init locals
    // (immediately reassigned; the transient null never escapes).
    constexpr NonNull() noexcept : ptr_(nullptr) {}
    constexpr NonNull<T> add(std::size_t n) const noexcept { return NonNull<T>(ptr_ + n); }
    constexpr NonNull<T> sub(std::size_t n) const noexcept { return NonNull<T>(ptr_ - n); }
    constexpr T read() const noexcept { return *ptr_; }
    constexpr std::size_t offset_from_unsigned(NonNull<T> origin) const noexcept {
        return static_cast<std::size_t>(ptr_ - origin.ptr_);
    }
    constexpr std::size_t addr() const noexcept {
        return reinterpret_cast<std::uintptr_t>(ptr_);
    }

    // Rust `NonNull::without_provenance(addr)` — an address-only,
    // never-dereferenced pointer (Weak::new's usize::MAX dangling sentinel).
    // Accepts NonZero-likes (`.get()`) or integrals.
    template<typename AddrLike>
    static NonNull<T> without_provenance(AddrLike addr_like) noexcept {
        std::uintptr_t a;
        if constexpr (requires { addr_like.get(); }) {
            a = static_cast<std::uintptr_t>(addr_like.get());
        } else {
            a = static_cast<std::uintptr_t>(addr_like);
        }
        return NonNull<T>(reinterpret_cast<T*>(a));
    }

    // Rust's `NonNull::from(&mut T)` / `NonNull::from(&T)` converts a
    // borrow into a non-null raw-pointer wrapper. In transpiled C++ the
    // `&mut T` argument arrives as a `T*` (e.g. the return of
    // `Box::leak(b)`), so the most useful spelling here is `from(T*)`,
    // accepting an already non-null pointer.
    // @unsafe
    // Rust `impl From<&T> for NonNull<T>` / `From<&mut T>`.
    static constexpr NonNull<T> from(T& r) noexcept {
        return NonNull<T>{std::addressof(r)};
    }
    static constexpr NonNull<T> from(T* ptr) noexcept {
        return NonNull<T>(ptr);
    }

    // Rust `NonNull::from(&[u8])` builds a fat pointer to the slice DATA
    // (`.cast::<E>()` then yields the data pointer). The `&[u8]` borrow
    // arrives in C++ as a std::span value; capture its data pointer —
    // pointing at the span OBJECT would make a later cast() reinterpret
    // the span header as elements.
    // @unsafe
    template<typename E, std::size_t N>
        requires (std::is_same_v<std::remove_const_t<E>, std::remove_const_t<T>>)
    static constexpr NonNull<T> from(std::span<E, N> s) noexcept {
        return NonNull<T>(const_cast<T*>(s.data()));
    }

    // Overload accepting an existing NonNull<T> — identity, used by
    // Unique::from(NonNull) in raw_vec. Added for vec_port.
    static constexpr NonNull<T> from(NonNull<T> other) noexcept {
        return other;
    }

    // Overload accepting a CastProxy — completes the cast and yields
    // NonNull<T>. Used by `Unique::from(ptr.cast())` patterns where
    // `cast()` returns the proxy. Added for vec_port.
    static constexpr NonNull<T> from(CastProxy proxy) noexcept {
        return NonNull<T>(reinterpret_cast<T*>(proxy.ptr_));
    }

    constexpr T* as_ptr() const noexcept {
        return ptr_;
    }

    constexpr T& as_mut() noexcept {
        return *ptr_;
    }

    // Rust NonNull::as_ref(&self) -> &T — borrow the pointee immutably.
    // Mirrors `as_mut` but for const access.
    // @unsafe — caller asserts the pointer is dereferenceable.
    constexpr const T& as_ref() const noexcept {
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

// Rust's fat slice pointer `NonNull<[T]>` (Allocator::allocate's return):
// pointer + length. Mirrors the thin NonNull surface (cast/as_non_null_ptr)
// while keeping the length reachable (`block.len() != layout.size`).
template<typename T>
class NonNullSlice {
    T* ptr_;
    std::size_t len_;

public:
    using rusty_pointer_identity_semantics = void;

    constexpr NonNullSlice(T* ptr, std::size_t len) noexcept : ptr_(ptr), len_(len) {}
    constexpr explicit NonNullSlice(std::span<T> s) noexcept
        : ptr_(s.data()), len_(s.size()) {}
    // Thin-pointer fallback (length unknown -> 0); keeps producers that
    // only have a data pointer compiling.
    constexpr explicit NonNullSlice(T* ptr) noexcept : ptr_(ptr), len_(0) {}
    constexpr explicit NonNullSlice(NonNull<T> ptr) noexcept
        : ptr_(ptr.as_ptr()), len_(0) {}

    static constexpr NonNullSlice<T> new_unchecked(std::span<T> s) noexcept {
        return NonNullSlice<T>(s);
    }
    static constexpr NonNullSlice<T> new_unchecked(T* ptr) noexcept {
        return NonNullSlice<T>(ptr);
    }

    constexpr std::size_t len() const noexcept { return len_; }
    constexpr std::size_t size() const noexcept { return len_; }
    constexpr T* as_ptr() const noexcept { return ptr_; }
    constexpr T* data() const noexcept { return ptr_; }
    constexpr NonNull<T> as_non_null_ptr() const noexcept { return NonNull<T>(ptr_); }
    constexpr std::span<T> as_span() const noexcept { return std::span<T>(ptr_, len_); }

    constexpr auto cast() const noexcept { return NonNull<T>(ptr_).cast(); }
    template<typename U>
    constexpr NonNull<U> cast() const noexcept {
        return NonNull<U>(reinterpret_cast<U*>(ptr_));
    }

    // Thin-context compatibility: existing consumers typed NonNull<T>.
    constexpr operator NonNull<T>() const noexcept { return NonNull<T>(ptr_); }

    friend constexpr bool operator==(NonNullSlice lhs, NonNullSlice rhs) noexcept {
        return lhs.ptr_ == rhs.ptr_ && lhs.len_ == rhs.len_;
    }
    friend constexpr bool operator!=(NonNullSlice lhs, NonNullSlice rhs) noexcept {
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
using NonNullSlice = ::rusty::NonNullSlice<T>;

inline constexpr std::nullptr_t null_mut() noexcept {
    return nullptr;
}

// Typed form: `core::ptr::null_mut::<T>()` → `T*`. A separate template
// overload, so the bare `null_mut()` above still yields `nullptr_t` for
// untyped callers (this template is not viable without an explicit argument,
// so `null_mut()` is unambiguous).
template<typename T>
inline constexpr T* null_mut() noexcept {
    return nullptr;
}

// `core::ptr::null::<T>()` → `const T*`, with the same bare/typed split.
inline constexpr std::nullptr_t null() noexcept {
    return nullptr;
}

template<typename T>
inline constexpr const T* null() noexcept {
    return nullptr;
}

// `core::ptr::slice_from_raw_parts[_mut](data, len)` build a `*const/*mut [T]`
// raw slice (fat) pointer. This runtime models a raw slice pointer by its data
// pointer alone — the length is carried separately by the caller (and a
// `NonNull<[T]>` is represented as `NonNull<T>`), so the constructors return the
// data pointer. Distinct from `slice::from_raw_parts[_mut]`, which build an
// actual `&[T]`/`std::span<T>` view.
template<typename T>
inline constexpr T* slice_from_raw_parts_mut(T* data, size_t len) noexcept {
    (void)len;
    return data;
}

template<typename T>
inline constexpr const T* slice_from_raw_parts(const T* data, size_t len) noexcept {
    (void)len;
    return data;
}

// `without_provenance(addr)` — construct a pointer from a raw address with
// strict-provenance API (the resulting pointer has no provenance). In C++
// we just reinterpret the address. Used by transpiled core::slice::iter
// for ZST iterators which encode `len` as the "pointer" itself.
template<typename T = void>
inline constexpr const T* without_provenance(std::uintptr_t addr) noexcept {
    return reinterpret_cast<const T*>(addr);
}

template<typename T = void>
inline constexpr T* without_provenance_mut(std::uintptr_t addr) noexcept {
    return reinterpret_cast<T*>(addr);
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

// `<*const T>::cast()` / `<*mut T>::cast()` when the target pointee type is NOT
// determinable at the call site (no turbofish, and the result flows into a
// callee whose signature the transpiler doesn't model — e.g. a C SIMD
// intrinsic `_mm_loadu_si128(const __m128i*)`). Returns a proxy that
// reinterpret-casts to whatever pointer type the surrounding context requires.
// Const-correct: a proxy over a `const T*` only converts to `const U*` (a
// conversion to a mutable `U*` is a compile error, as it should be).
template<typename T>
struct RawCastProxy {
    T* ptr_;
    template<typename U>
    constexpr operator U*() const noexcept {
        return reinterpret_cast<U*>(ptr_);
    }
    // `NonNull::from(x).cast()` flowing into a NonNull<U> parameter
    // (cstr::from_ptr) — the targetless proxy adapts to NonNull too.
    template<typename U>
    constexpr operator NonNull<U>() const noexcept {
        return NonNull<U>(reinterpret_cast<U*>(const_cast<std::remove_const_t<T>*>(ptr_)));
    }
    // Chain parity with NonNull's own CastProxy (`nn.cast().as_non_null_ptr()`)
    // — without a deducible target, falls back to the source pointee.
    constexpr NonNull<std::remove_const_t<T>> as_non_null_ptr() const noexcept {
        return NonNull<std::remove_const_t<T>>(const_cast<std::remove_const_t<T>*>(ptr_));
    }
};

template<typename T>
inline constexpr RawCastProxy<T> cast(T* ptr) noexcept {
    return RawCastProxy<T>{ptr};
}

// NonNull receiver through the same targetless-cast seam (an
// unresolvable receiver type may turn out to be NonNull at C++ time).
template<typename T>
inline constexpr RawCastProxy<T> cast(const NonNull<T>& ptr) noexcept {
    return RawCastProxy<T>{ptr.get()};
}

// Fat slice pointer (`block.cast()` on Allocator::allocate's return): the
// cast drops to the data pointer, same as Rust's NonNull<[T]>::cast.
template<typename T>
inline constexpr RawCastProxy<T> cast(const NonNullSlice<T>& ptr) noexcept {
    return RawCastProxy<T>{ptr.as_ptr()};
}

// `<*const T>::align_offset(align)` — the number of ELEMENTS of T that must be
// added to `ptr` to reach an `align`-aligned address, or SIZE_MAX when that is
// unreachable. `align` must be a power of two. Mirrors core::ptr::align_offset.
template<typename T>
inline std::size_t align_offset(const T* ptr, std::size_t align) noexcept {
    if (align == 0 || (align & (align - 1)) != 0) {
        return static_cast<std::size_t>(-1);
    }
    std::uintptr_t addr = reinterpret_cast<std::uintptr_t>(ptr);
    std::size_t misalign = static_cast<std::size_t>(addr & (align - 1));
    if (misalign == 0) {
        return 0;
    }
    std::size_t byte_offset = align - misalign;
    if (sizeof(T) == 0 || byte_offset % sizeof(T) != 0) {
        return static_cast<std::size_t>(-1);  // alignment unreachable for this T
    }
    return byte_offset / sizeof(T);
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
    // Under the strict null-state convention there is no global
    // forgotten-address table for `dst` to consult — `T`'s own
    // `_rusty_forgotten` flag (set by move ctors and `mem::forget`)
    // is the single source of truth. `ptr::write` matches Rust's
    // `ptr::write` semantics: bit-construct a fresh `T` at `dst`
    // without touching whatever was there.
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
        // Left-shift overlap: process forward, destroy-then-construct each
        // slot. The null-state convention makes destruction safe for
        // moved-from slots (T's destructor sees `_rusty_forgotten == true`
        // and short-circuits), so we no longer need a side-table to skip.
        for (std::size_t i = 0; i < element_count; ++i) {
            T* const dst_i = dst + i;
            T* const src_i = const_cast<T*>(src) + i;
            std::destroy_at(dst_i);
            std::construct_at(dst_i, std::move(*src_i));
        }
    } else {
        // Right-shift overlap: process reverse. Slots beyond the original
        // source window are uninitialized holes — no destroy_at on those.
        for (std::size_t i = element_count; i-- > 0;) {
            T* const dst_i = dst + i;
            T* const src_i = const_cast<T*>(src) + i;
            if (dst_i < src + element_count) {
                std::destroy_at(dst_i);
            }
            std::construct_at(dst_i, std::move(*src_i));
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

/// Rust's `core::ptr::swap(a, b)` — swaps the VALUES `*a` and `*b`.
/// NOT the same as `std::swap(a, b)` (which swaps the pointer values
/// themselves). A previous binary_heap_port patcher rule conflated
/// the two; this overload provides the correct shape so the cppm can
/// be patched back to `rusty::ptr::swap(a, b)`.
template<typename T>
inline void swap(T* a, T* b) {
    using std::swap;
    swap(*a, *b);
}

template<typename T, typename Count>
inline void swap_nonoverlapping(T* x, T* y, Count count) {
    const auto element_count = static_cast<std::size_t>(count);
    if (element_count == 0) {
        return;
    }
    if constexpr (std::is_trivially_copyable_v<T>) {
        auto byte_count = element_count * sizeof(T);
        // Use a small fixed stack buffer chunked swap for large counts.
        constexpr std::size_t CHUNK = 256;
        unsigned char buf[CHUNK];
        auto px = reinterpret_cast<unsigned char*>(x);
        auto py = reinterpret_cast<unsigned char*>(y);
        while (byte_count >= CHUNK) {
            std::memcpy(buf, px, CHUNK);
            std::memcpy(px, py, CHUNK);
            std::memcpy(py, buf, CHUNK);
            px += CHUNK;
            py += CHUNK;
            byte_count -= CHUNK;
        }
        if (byte_count != 0) {
            std::memcpy(buf, px, byte_count);
            std::memcpy(px, py, byte_count);
            std::memcpy(py, buf, byte_count);
        }
    } else {
        for (std::size_t i = 0; i < element_count; ++i) {
            using std::swap;
            swap(x[i], y[i]);
        }
    }
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

// `<ptr>.offset_from(origin)` (and c2rust's `<ptr>.c_offset_from(origin)`):
// the element-count difference between two pointers into the same allocation.
// Rust returns `isize`; both arguments decay to `const T*` so a mut/const
// receiver-vs-origin mix still resolves to a single T.
template<typename T>
inline std::ptrdiff_t offset_from(const T* ptr, const T* origin) noexcept {
    return ptr - origin;
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

// ----- Unique<T>: like NonNull<T> but expresses sole ownership.
// In rustc, `Unique<T>` is a `NonNull<T>` with a `PhantomData<T>` to
// mark it as owning. For the C++ port we alias it directly to
// NonNull — the ownership distinction matters for Rust's borrow
// checker but not for the C++ semantics. (Added for vec_port.)
template<typename T>
using Unique = NonNull<T>;

// ----- Alignment: like rustc's core::ptr::Alignment.
// A power-of-two alignment value with type-driven constructors.
// (Added for vec_port.)
class Alignment {
    std::size_t value_;
public:
    constexpr explicit Alignment(std::size_t v) noexcept : value_(v) {}
    constexpr std::size_t as_usize() const noexcept { return value_; }
    constexpr std::size_t as_nonzero() const noexcept { return value_; }
    template<typename T>
    static constexpr Alignment of() noexcept { return Alignment(alignof(T)); }
    // Rust `Alignment::of_val_raw(ptr)` — the pointee's alignment. The port
    // has no DSTs, so this is just alignof of the (sized) pointee.
    template<typename P>
    static constexpr Alignment of_val_raw(P* /*p*/) noexcept { return Alignment(alignof(P)); }
    constexpr bool operator==(const Alignment& o) const noexcept = default;
};

// Rust `ptr::addr_eq(a, b)` — compare pointers by ADDRESS only (ignores
// pointee types / any would-be metadata; the port has no fat pointers).
template<typename A2, typename B2>
inline bool addr_eq(const A2* a, const B2* b) noexcept {
    return static_cast<const void*>(a) == static_cast<const void*>(b);
}

// Rust `ptr::from_ref(&v)` / `ptr::from_mut(&mut v)` — reference to raw
// pointer. from_ref returns a MUTABLE pointer (Rust yields *const T, but the
// transpiled cast sites re-type it to byte pointers for memcpy SOURCES —
// const_cast here avoids a cast-away-const error at those sites; the
// storage is never written through this path).
template<typename T>
inline T* from_ref(const T& r) noexcept { return const_cast<T*>(&r); }
template<typename T>
inline T* from_mut(T& r) noexcept { return &r; }

// `core::ptr::replace(dst, src)` — atomic-ish swap that returns the
// old value while overwriting `*dst`. For our purposes (cell_port
// uses it in `Cell::replace`), a sequential read-write is fine.
template<typename T>
inline T replace(T* dst, T src) noexcept(std::is_nothrow_move_constructible_v<T> &&
                                          std::is_nothrow_move_assignable_v<T>) {
    T old = std::move(*dst);
    *dst = std::move(src);
    return old;
}

// `core::ptr::eq(p, q)` — pointer-identity comparison. Distinguished
// from `==` only for fat pointers in Rust; for our T* / NonNull<T>
// surface a plain `==` suffices.
template<typename T, typename U>
inline bool eq(const T* a, const U* b) noexcept {
    return static_cast<const void*>(a) == static_cast<const void*>(b);
}

} // namespace ptr

} // namespace rusty

#endif // RUSTY_PTR_HPP
