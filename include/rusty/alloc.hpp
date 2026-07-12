#ifndef RUSTY_ALLOC_HPP
#define RUSTY_ALLOC_HPP

#include <concepts>
#include <cstddef>
#include <cstdlib>
#include <cstdint>
#include <cstring>
#include <limits>
#include <new>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>
#include <rusty/ptr.hpp>
#include <rusty/result.hpp>

namespace rusty::alloc {

struct LayoutErr {};
using LayoutError = LayoutErr;

struct Layout {
    std::size_t size;
    std::size_t align;

    // `core::alloc::Layout` derives `PartialEq`/`Eq`; mirror that so derived
    // equality of types holding a Layout field compiles.
    constexpr bool operator==(const Layout& other) const noexcept {
        return size == other.size && align == other.align;
    }

    // Rust-style accessor mirroring `core::alloc::Layout::alignment()`.
    // (`.size()` and `.align()` are mapped via post_transpile_patch.py
    // strip-parens, since they conflict with field names.)
    // Added for vec_port.
    constexpr rusty::ptr::Alignment alignment() const noexcept {
        return rusty::ptr::Alignment(align);
    }

    // Rust `Layout::for_value_raw(ptr)` — layout of the (sized) pointee.
    // The port has no DSTs, so this equals Layout::of the pointee type.
    template<typename P>
    static constexpr Layout for_value_raw(P* /*p*/) noexcept {
        using V = std::remove_cv_t<P>;
        return Layout{sizeof(V), alignof(V)};
    }

    // Rust `Layout::padding_needed_for(align)` — bytes to round `size` up to
    // the next multiple of `align`.
    template<typename Al>
    constexpr std::size_t padding_needed_for(Al align_like) const noexcept {
        std::size_t a;
        if constexpr (requires { align_like.as_usize(); }) {
            a = align_like.as_usize();
        } else {
            a = static_cast<std::size_t>(align_like);
        }
        return (a - (size % a)) % a;
    }

    static constexpr Layout from_size_align_unchecked(
        std::size_t size,
        std::size_t align) noexcept {
        return Layout{size, align};
    }

    static rusty::Result<Layout, LayoutErr> from_size_align(
        std::size_t size,
        std::size_t align) {
        const bool align_is_power_of_two = align != 0 && ((align & (align - 1)) == 0);
        if (!align_is_power_of_two) {
            return rusty::Result<Layout, LayoutErr>::Err(LayoutErr{});
        }
        if (size > (std::numeric_limits<std::size_t>::max() - (align - 1))) {
            return rusty::Result<Layout, LayoutErr>::Err(LayoutErr{});
        }
        return rusty::Result<Layout, LayoutErr>::Ok(Layout{size, align});
    }

    // Rust's Layout::new::<T>() — `new` is a C++ keyword so the transpiler
    // renames it to `new_`. Keep `for_value<T>` as a readable alias.
    template<typename T>
    static constexpr Layout new_() noexcept {
        return Layout{sizeof(T), alignof(T)};
    }

    template<typename T>
    static constexpr Layout for_value() noexcept {
        return Layout{sizeof(T), alignof(T)};
    }

    // Rust's Layout::array::<T>(n) — does not check for overflow here; callers
    // mirror Rust's behaviour by handling allocation failure downstream.
    template<typename T>
    static constexpr Layout array(std::size_t n) noexcept {
        return Layout{sizeof(T) * n, alignof(T)};
    }

    // Padding needed to round `self.size` up to the next multiple of `align`.
    // Mirrors Rust's `Layout::padding_needed_for`.
    constexpr std::size_t padding_needed_for(std::size_t to_align) const noexcept {
        const std::size_t mask = to_align - 1;
        return (to_align - (size & mask)) & mask;
    }

    // Round `self.size` up to the next multiple of `self.align`. Mirrors
    // Rust's `Layout::pad_to_align`.
    constexpr Layout pad_to_align() const noexcept {
        return Layout{size + padding_needed_for(align), align};
    }

    // Mirrors Rust's `Layout::extend(self, next) -> Result<(Layout, usize), LayoutErr>`.
    // Returns the combined layout and the byte-offset of `next` inside the
    // combined layout. Lossy on alignment overflow: returns Err in that case.
    rusty::Result<std::tuple<Layout, std::size_t>, LayoutErr>
    extend(Layout next) const {
        const std::size_t new_align = align > next.align ? align : next.align;
        const std::size_t pad = padding_needed_for(next.align);
        // Overflow checks mirror Rust's behaviour.
        if (size > std::numeric_limits<std::size_t>::max() - pad) {
            return rusty::Result<std::tuple<Layout, std::size_t>, LayoutErr>::Err(LayoutErr{});
        }
        const std::size_t offset = size + pad;
        if (offset > std::numeric_limits<std::size_t>::max() - next.size) {
            return rusty::Result<std::tuple<Layout, std::size_t>, LayoutErr>::Err(LayoutErr{});
        }
        const std::size_t new_size = offset + next.size;
        return rusty::Result<std::tuple<Layout, std::size_t>, LayoutErr>::Ok(
            std::make_tuple(Layout{new_size, new_align}, offset));
    }

    // Mirrors Rust's `Layout::repeat(self, n)`. Returns the layout for an
    // array of `n` copies of `self` (with internal padding) plus the byte
    // stride between copies.
    rusty::Result<std::tuple<Layout, std::size_t>, LayoutErr>
    repeat(std::size_t n) const {
        const std::size_t padded = pad_to_align().size;
        if (padded != 0
            && n != 0
            && padded > std::numeric_limits<std::size_t>::max() / n) {
            return rusty::Result<std::tuple<Layout, std::size_t>, LayoutErr>::Err(LayoutErr{});
        }
        return rusty::Result<std::tuple<Layout, std::size_t>, LayoutErr>::Ok(
            std::make_tuple(Layout{padded * n, align}, padded));
    }

    // Mirrors unstable Rust's `Layout::repeat_packed(self, n) -> Result<Layout, LayoutError>`.
    // Returns the layout for `n` copies of `self` packed back-to-back
    // with no internal padding. Added for vec_port.
    rusty::Result<Layout, LayoutErr>
    repeat_packed(std::size_t n) const {
        if (size != 0 && n != 0
            && size > std::numeric_limits<std::size_t>::max() / n) {
            return rusty::Result<Layout, LayoutErr>::Err(LayoutErr{});
        }
        return rusty::Result<Layout, LayoutErr>::Ok(Layout{size * n, align});
    }

    // Mirrors Rust's `Layout::dangling`. Returns a dangling-but-aligned
    // raw byte pointer — never deallocate this pointer.
    rusty::NonNull<std::uint8_t> dangling() const noexcept {
        return rusty::NonNull<std::uint8_t>::new_unchecked(
            reinterpret_cast<std::uint8_t*>(align));
    }
};

// AllocError mirrors core::alloc::AllocError. Originally zero-sized
// in rustc, but the vec_port emits aggregate-init with `.layout` and
// `.non_exhaustive` fields per the TryReserveErrorKind::AllocError
// variant shape. Added the fields with defaults so old `AllocError{}`
// call sites still work.
struct AllocError {
    Layout layout{0, 1};                      // failed layout (defaults are harmless)
    std::tuple<> non_exhaustive{};            // rustc ABI-stability marker
};

inline std::uint8_t* alloc(Layout layout) {
    void* memory = nullptr;
    if (layout.align <= alignof(std::max_align_t)) {
        memory = std::malloc(layout.size);
    } else {
        memory = ::operator new(layout.size, std::align_val_t(layout.align), std::nothrow);
    }
    return static_cast<std::uint8_t*>(memory);
}

inline void dealloc(std::uint8_t* ptr, Layout layout) noexcept {
    if (ptr == nullptr) {
        return;
    }
    if (layout.align <= alignof(std::max_align_t)) {
        std::free(ptr);
    } else {
        ::operator delete(ptr, std::align_val_t(layout.align));
    }
}

inline std::uint8_t* realloc(
    std::uint8_t* ptr,
    Layout old_layout,
    std::size_t new_size) noexcept {
    if (old_layout.align <= alignof(std::max_align_t)) {
        return static_cast<std::uint8_t*>(std::realloc(ptr, new_size));
    }

    void* memory = ::operator new(new_size, std::align_val_t(old_layout.align), std::nothrow);
    if (memory == nullptr) {
        return nullptr;
    }

    if (ptr != nullptr) {
        const std::size_t bytes_to_copy =
            old_layout.size < new_size ? old_layout.size : new_size;
        std::memcpy(memory, ptr, bytes_to_copy);
        ::operator delete(ptr, std::align_val_t(old_layout.align));
    }

    return static_cast<std::uint8_t*>(memory);
}

[[noreturn]] inline void handle_alloc_error(Layout) {
    throw std::bad_alloc();
}

// Allocator concept — faithful mirror of Rust's `unsafe trait Allocator`.
// Required methods are `allocate` and `deallocate`. Rust's trait also
// provides default `allocate_zeroed`, `grow`, `grow_zeroed`, and `shrink`
// methods on top of those two; here we expose them as non-member helpers
// `rusty::alloc::allocate_zeroed_via(a, layout)`, `..::grow_via(...)`,
// `..::shrink_via(...)` so concept-satisfying types don't have to spell
// these themselves. `Global` provides direct `allocate_zeroed`/`grow`/
// `shrink` methods for convenience.
template<typename A>
concept Allocator = requires(A const& ca,
                             rusty::NonNull<std::uint8_t> p,
                             Layout l) {
    { ca.allocate(l) } -> std::same_as<rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>>;
    { ca.deallocate(p, l) };
};

// Default `allocate_zeroed` body usable by any Allocator. Mirrors Rust's
// `Allocator::allocate_zeroed` default: allocate, then memset.
template<typename A>
inline rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>
allocate_zeroed_via(const A& a, Layout layout) {
    auto result = a.allocate(layout);
    if (result.is_ok()) {
        // `unwrap` consumes; we want to keep the result. Copy NonNull (it is
        // trivially copyable, so this is cheap) and memset through the copy.
        // Re-construct an Ok with the same pointer so we hand back exactly
        // what `allocate` returned.
        auto p = result.unwrap();
        std::memset(p.as_ptr(), 0, layout.size);
        return rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>::Ok(p);
    }
    return result;
}

// Default `grow` body. Mirrors Rust's `Allocator::grow` default: allocate
// the new size, copy bytes from the old buffer, deallocate the old buffer.
// Preconditions (unchecked): new_layout.size >= old_layout.size, alignments
// match. Callers are expected to honour Rust's unsafe contract.
template<typename A>
inline rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>
grow_via(const A& a,
         rusty::NonNull<std::uint8_t> ptr,
         Layout old_layout,
         Layout new_layout) {
    auto result = a.allocate(new_layout);
    if (result.is_err()) {
        return result;
    }
    auto p = result.unwrap();
    std::memcpy(p.as_ptr(), ptr.as_ptr(), old_layout.size);
    a.deallocate(ptr, old_layout);
    return rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>::Ok(p);
}

// Default `grow_zeroed` body: like `grow_via`, then zero the tail. Mirrors
// Rust's `Allocator::grow_zeroed` default.
template<typename A>
inline rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>
grow_zeroed_via(const A& a,
                rusty::NonNull<std::uint8_t> ptr,
                Layout old_layout,
                Layout new_layout) {
    auto result = grow_via(a, ptr, old_layout, new_layout);
    if (result.is_ok() && new_layout.size > old_layout.size) {
        auto p = result.unwrap();
        std::memset(p.as_ptr() + old_layout.size, 0, new_layout.size - old_layout.size);
        return rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>::Ok(p);
    }
    return result;
}

// Default `shrink` body. Mirrors Rust's `Allocator::shrink` default:
// allocate the new (smaller) size, copy the kept prefix, deallocate the old.
// Precondition (unchecked): new_layout.size <= old_layout.size.
template<typename A>
inline rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>
shrink_via(const A& a,
           rusty::NonNull<std::uint8_t> ptr,
           Layout old_layout,
           Layout new_layout) {
    auto result = a.allocate(new_layout);
    if (result.is_err()) {
        return result;
    }
    auto p = result.unwrap();
    std::memcpy(p.as_ptr(), ptr.as_ptr(), new_layout.size);
    a.deallocate(ptr, old_layout);
    return rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>::Ok(p);
}

// Global — the default system allocator, satisfies Allocator.
struct Global {
    constexpr Global() noexcept = default;
    constexpr Global(const Global&) noexcept = default;
    constexpr Global(Global&&) noexcept = default;
    constexpr Global& operator=(const Global&) noexcept = default;
    constexpr Global& operator=(Global&&) noexcept = default;

    static constexpr Global default_() noexcept { return Global{}; }

    rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>
    allocate(Layout layout) const {
        // Rust's Allocator contract for ZSTs: return a dangling-but-aligned
        // pointer, never call the underlying allocator. Matching that here
        // makes `Box<()>` / `VecLegacy<()>` work as expected and avoids passing
        // `malloc(0)` (whose behaviour is implementation-defined).
        if (layout.size == 0) {
            return rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>::Ok(
                layout.dangling());
        }
        std::uint8_t* mem = ::rusty::alloc::alloc(layout);
        if (mem == nullptr) {
            return rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>::Err(AllocError{});
        }
        return rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>::Ok(
            rusty::NonNull<std::uint8_t>::new_unchecked(mem));
    }

    rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>
    allocate_zeroed(Layout layout) const {
        if (layout.size == 0) {
            return rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>::Ok(
                layout.dangling());
        }
        std::uint8_t* mem = ::rusty::alloc::alloc(layout);
        if (mem == nullptr) {
            return rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>::Err(AllocError{});
        }
        std::memset(mem, 0, layout.size);
        return rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>::Ok(
            rusty::NonNull<std::uint8_t>::new_unchecked(mem));
    }

    void deallocate(rusty::NonNull<std::uint8_t> ptr, Layout layout) const noexcept {
        // Mirror the ZST path on the deallocate side — `ptr` came from
        // `Layout::dangling()` and was never malloc'd, so do nothing.
        if (layout.size == 0) {
            return;
        }
        ::rusty::alloc::dealloc(ptr.as_ptr(), layout);
    }

    rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>
    grow(rusty::NonNull<std::uint8_t> ptr, Layout old_layout, Layout new_layout) const {
        return grow_via(*this, ptr, old_layout, new_layout);
    }

    rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>
    grow_zeroed(rusty::NonNull<std::uint8_t> ptr, Layout old_layout, Layout new_layout) const {
        return grow_zeroed_via(*this, ptr, old_layout, new_layout);
    }

    rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>
    shrink(rusty::NonNull<std::uint8_t> ptr, Layout old_layout, Layout new_layout) const {
        return shrink_via(*this, ptr, old_layout, new_layout);
    }
};

inline constexpr bool operator==(const Global&, const Global&) noexcept { return true; }
inline constexpr bool operator!=(const Global&, const Global&) noexcept { return false; }

static_assert(Allocator<Global>, "rusty::alloc::Global must satisfy the Allocator concept");

namespace __export {

template<typename T>
constexpr std::remove_cvref_t<T> must_use(T&& value) {
    return std::forward<T>(value);
}

} // namespace __export

namespace fmt {

template<typename T>
inline std::string format(T&& fmt_like) {
    return std::string(std::forward<T>(fmt_like));
}

} // namespace fmt

} // namespace rusty::alloc

#endif // RUSTY_ALLOC_HPP
