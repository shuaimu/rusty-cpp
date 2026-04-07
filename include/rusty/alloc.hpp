#ifndef RUSTY_ALLOC_HPP
#define RUSTY_ALLOC_HPP

#include <cstddef>
#include <cstdlib>
#include <cstdint>
#include <new>
#include <string>
#include <string_view>
#include <utility>

namespace rusty::alloc {

struct Layout {
    std::size_t size;
    std::size_t align;

    static constexpr Layout from_size_align_unchecked(
        std::size_t size,
        std::size_t align) noexcept {
        return Layout{size, align};
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
