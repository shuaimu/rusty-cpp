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
};

// AllocError mirrors core::alloc::AllocError — a zero-sized error type.
struct AllocError {};

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
// Allocators returning a non-null pointer to at least `layout.size` bytes
// aligned to `layout.align` are valid. `deallocate` is `unsafe` in Rust;
// here it is just a non-static member; safety is the caller's responsibility.
template<typename A>
concept Allocator = requires(A const& ca,
                             rusty::NonNull<std::uint8_t> p,
                             Layout l) {
    { ca.allocate(l) } -> std::same_as<rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>>;
    { ca.deallocate(p, l) };
};

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
        std::uint8_t* mem = ::rusty::alloc::alloc(layout);
        if (mem == nullptr) {
            return rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>::Err(AllocError{});
        }
        return rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>::Ok(
            rusty::NonNull<std::uint8_t>::new_unchecked(mem));
    }

    rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>
    allocate_zeroed(Layout layout) const {
        std::uint8_t* mem = ::rusty::alloc::alloc(layout);
        if (mem == nullptr) {
            return rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>::Err(AllocError{});
        }
        std::memset(mem, 0, layout.size);
        return rusty::Result<rusty::NonNull<std::uint8_t>, AllocError>::Ok(
            rusty::NonNull<std::uint8_t>::new_unchecked(mem));
    }

    void deallocate(rusty::NonNull<std::uint8_t> ptr, Layout layout) const noexcept {
        ::rusty::alloc::dealloc(ptr.as_ptr(), layout);
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
