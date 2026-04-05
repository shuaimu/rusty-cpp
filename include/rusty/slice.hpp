#ifndef RUSTY_SLICE_HPP
#define RUSTY_SLICE_HPP

#include <cstddef>
#include <span>

namespace rusty {

// Raw-pointer slice constructors used by expanded std/core::slice paths.
template<typename T>
auto from_raw_parts(const T* ptr, size_t len) {
    return std::span<const T>(ptr, len);
}

template<typename T>
auto from_raw_parts_mut(T* ptr, size_t len) {
    return std::span<T>(ptr, len);
}

} // namespace rusty

#endif // RUSTY_SLICE_HPP
