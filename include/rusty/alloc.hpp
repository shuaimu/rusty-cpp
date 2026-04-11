#ifndef RUSTY_ALLOC_HPP
#define RUSTY_ALLOC_HPP

#include <cstddef>
#include <cstdlib>
#include <cstdint>
#include <cstring>
#include <limits>
#include <new>
#include <string>
#include <string_view>
#include <utility>
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

namespace __export {

template<typename T>
constexpr decltype(auto) must_use(T&& value) {
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
