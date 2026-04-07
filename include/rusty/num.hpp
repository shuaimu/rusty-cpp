#ifndef RUSTY_NUM_HPP
#define RUSTY_NUM_HPP

#include <bit>
#include <cstddef>
#include <cstdint>
#include <limits>
#include <type_traits>

namespace rusty::num {

template<typename T>
class NonZero {
    static_assert(std::is_integral_v<T>, "NonZero<T> requires integral T");

private:
    T value_;

public:
    constexpr explicit NonZero(T value) noexcept : value_(value) {}

    static constexpr NonZero<T> new_unchecked(T value) noexcept {
        return NonZero<T>(value);
    }

    constexpr T get() const noexcept {
        return value_;
    }

    constexpr int leading_zeros() const noexcept {
        using UnsignedT = std::make_unsigned_t<T>;
        return static_cast<int>(std::countl_zero(static_cast<UnsignedT>(value_)));
    }
};

using NonZeroUsize = NonZero<std::size_t>;
using NonZeroU64 = NonZero<std::uint64_t>;

} // namespace rusty::num

// Checked arithmetic helpers (Rust integer methods returning Option<T>)
// Note: Option<T> must be defined before including this header (included via rusty.hpp)
namespace rusty {

template<typename T>
requires std::is_integral_v<T>
Option<T> checked_add(T a, T b) {
    T result;
    if (__builtin_add_overflow(a, b, &result)) {
        return Option<T>::None();
    }
    return Option<T>::Some(result);
}

template<typename T>
requires std::is_integral_v<T>
Option<T> checked_sub(T a, T b) {
    T result;
    if (__builtin_sub_overflow(a, b, &result)) {
        return Option<T>::None();
    }
    return Option<T>::Some(result);
}

template<typename T>
requires std::is_integral_v<T>
Option<T> checked_mul(T a, T b) {
    T result;
    if (__builtin_mul_overflow(a, b, &result)) {
        return Option<T>::None();
    }
    return Option<T>::Some(result);
}

template<typename T>
requires std::is_integral_v<T>
Option<T> checked_div(T a, T b) {
    if (b == 0) {
        return Option<T>::None();
    }
    if constexpr (std::is_signed_v<T>) {
        if (a == std::numeric_limits<T>::min() && b == static_cast<T>(-1)) {
            return Option<T>::None();
        }
    }
    return Option<T>::Some(a / b);
}

} // namespace rusty

#endif // RUSTY_NUM_HPP
