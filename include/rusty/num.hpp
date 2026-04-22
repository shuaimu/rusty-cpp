#ifndef RUSTY_NUM_HPP
#define RUSTY_NUM_HPP

#include <bit>
#include <cstddef>
#include <cstdint>
#include <limits>
#include <string_view>
#include <tuple>
#include <type_traits>
#include <utility>

namespace rusty::num {

template<typename T>
struct is_nonzero_integral : std::bool_constant<std::is_integral_v<T>> {};

template<>
struct is_nonzero_integral<__int128> : std::true_type {};

template<>
struct is_nonzero_integral<unsigned __int128> : std::true_type {};

template<typename T>
inline constexpr bool is_nonzero_integral_v = is_nonzero_integral<T>::value;

template<typename T>
class NonZero {
    static_assert(is_nonzero_integral_v<T>, "NonZero<T> requires integral T");

private:
    T value_;

public:
    constexpr explicit NonZero(T value) noexcept : value_(value) {}

    static Option<NonZero<T>> new_(T value) noexcept {
        if (value == static_cast<T>(0)) {
            return Option<NonZero<T>>(rusty::None);
        }
        return Option<NonZero<T>>(NonZero<T>(value));
    }

    static constexpr NonZero<T> new_unchecked(T value) noexcept {
        return NonZero<T>(value);
    }

    constexpr T get() const noexcept {
        return value_;
    }

    constexpr bool operator==(const NonZero& other) const noexcept {
        return value_ == other.value_;
    }
    constexpr bool operator!=(const NonZero& other) const noexcept {
        return value_ != other.value_;
    }
    constexpr auto operator<=>(const NonZero& other) const noexcept {
        return value_ <=> other.value_;
    }

    constexpr int leading_zeros() const noexcept {
        using UnsignedT = std::make_unsigned_t<T>;
        return static_cast<int>(std::countl_zero(static_cast<UnsignedT>(value_)));
    }
};

template<typename T>
struct Wrapping {
    T _0{};

    constexpr Wrapping() = default;
    constexpr explicit Wrapping(T value) : _0(std::move(value)) {}
    constexpr T value() const {
        return _0;
    }
};

template<typename T>
struct Saturating {
    T _0{};

    constexpr Saturating() = default;
    constexpr explicit Saturating(T value) : _0(std::move(value)) {}
    constexpr T value() const {
        return _0;
    }
};

using NonZeroUsize = NonZero<std::size_t>;
using NonZeroU64 = NonZero<std::uint64_t>;
using NonZeroI8 = NonZero<std::int8_t>;
using NonZeroI16 = NonZero<std::int16_t>;
using NonZeroI32 = NonZero<std::int32_t>;
using NonZeroI64 = NonZero<std::int64_t>;
using NonZeroI128 = NonZero<__int128>;
using NonZeroIsize = NonZero<std::ptrdiff_t>;
using NonZeroU8 = NonZero<std::uint8_t>;
using NonZeroU16 = NonZero<std::uint16_t>;
using NonZeroU32 = NonZero<std::uint32_t>;
using NonZeroU128 = NonZero<unsigned __int128>;

} // namespace rusty::num

// Checked arithmetic helpers (Rust integer methods returning Option<T>)
// Note: Option<T> must be defined before including this header (included via rusty.hpp)
namespace rusty {

template<typename T>
requires std::is_integral_v<T>
Option<T> checked_add(T a, T b) {
    T result;
    if (__builtin_add_overflow(a, b, &result)) {
        return Option<T>(rusty::None);
    }
    return Option<T>(result);
}

template<typename A, typename B>
requires(
    std::is_integral_v<std::remove_cvref_t<A>>
    && std::is_integral_v<std::remove_cvref_t<B>>
    && !std::is_same_v<std::remove_cvref_t<A>, std::remove_cvref_t<B>>)
auto checked_add(A a, B b) {
    using Common = std::common_type_t<std::remove_cvref_t<A>, std::remove_cvref_t<B>>;
    return checked_add<Common>(static_cast<Common>(a), static_cast<Common>(b));
}

template<typename T>
requires std::is_integral_v<T>
Option<T> checked_sub(T a, T b) {
    T result;
    if (__builtin_sub_overflow(a, b, &result)) {
        return Option<T>(rusty::None);
    }
    return Option<T>(result);
}

template<typename A, typename B>
requires(
    std::is_integral_v<std::remove_cvref_t<A>>
    && std::is_integral_v<std::remove_cvref_t<B>>
    && !std::is_same_v<std::remove_cvref_t<A>, std::remove_cvref_t<B>>)
auto checked_sub(A a, B b) {
    using Common = std::common_type_t<std::remove_cvref_t<A>, std::remove_cvref_t<B>>;
    return checked_sub<Common>(static_cast<Common>(a), static_cast<Common>(b));
}

template<typename T>
requires std::is_integral_v<T>
Option<T> checked_mul(T a, T b) {
    T result;
    if (__builtin_mul_overflow(a, b, &result)) {
        return Option<T>(rusty::None);
    }
    return Option<T>(result);
}

template<typename A, typename B>
requires(
    std::is_integral_v<std::remove_cvref_t<A>>
    && std::is_integral_v<std::remove_cvref_t<B>>
    && !std::is_same_v<std::remove_cvref_t<A>, std::remove_cvref_t<B>>)
auto checked_mul(A a, B b) {
    using Common = std::common_type_t<std::remove_cvref_t<A>, std::remove_cvref_t<B>>;
    return checked_mul<Common>(static_cast<Common>(a), static_cast<Common>(b));
}

template<typename T>
requires std::is_integral_v<T>
Option<T> checked_div(T a, T b) {
    if (b == 0) {
        return Option<T>(rusty::None);
    }
    if constexpr (std::is_signed_v<T>) {
        if (a == std::numeric_limits<T>::min() && b == static_cast<T>(-1)) {
            return Option<T>(rusty::None);
        }
    }
    return Option<T>(a / b);
}

template<typename A, typename B>
requires(
    std::is_integral_v<std::remove_cvref_t<A>>
    && std::is_integral_v<std::remove_cvref_t<B>>
    && !std::is_same_v<std::remove_cvref_t<A>, std::remove_cvref_t<B>>)
auto checked_div(A a, B b) {
    using Common = std::common_type_t<std::remove_cvref_t<A>, std::remove_cvref_t<B>>;
    return checked_div<Common>(static_cast<Common>(a), static_cast<Common>(b));
}

template<typename T, typename U>
requires(std::is_integral_v<T> && std::is_integral_v<std::remove_cvref_t<U>>)
constexpr T pow(T base, U exp) {
    using exp_unsigned = std::make_unsigned_t<std::remove_cvref_t<U>>;
    exp_unsigned e = static_cast<exp_unsigned>(exp);
    T result = static_cast<T>(1);
    while (e != 0) {
        if ((e & static_cast<exp_unsigned>(1)) != 0) {
            result = static_cast<T>(result * base);
        }
        e = static_cast<exp_unsigned>(e >> 1);
        if (e != 0) {
            base = static_cast<T>(base * base);
        }
    }
    return result;
}

template<typename T>
requires(std::is_integral_v<T> && std::is_unsigned_v<T>)
Option<T> checked_next_power_of_two(T value) {
    if (value <= static_cast<T>(1)) {
        return Option<T>(static_cast<T>(1));
    }
    T current = static_cast<T>(1);
    constexpr T two = static_cast<T>(2);
    while (current < value) {
        if (current > (std::numeric_limits<T>::max() / two)) {
            return Option<T>(rusty::None);
        }
        current = static_cast<T>(current << 1);
    }
    return Option<T>(current);
}

inline Option<std::size_t> checked_next_power_of_two_usize(std::size_t value) {
    return checked_next_power_of_two<std::size_t>(value);
}

template<typename T, typename Input, typename Radix>
requires(
    std::is_integral_v<std::remove_cvref_t<T>>
    && std::is_integral_v<std::remove_cvref_t<Radix>>)
Result<std::remove_cvref_t<T>, std::tuple<>> from_str_radix(Input&& input, Radix radix) {
    using RawT = std::remove_cvref_t<T>;
    using RawRadix = std::remove_cvref_t<Radix>;
    using Unsigned = std::make_unsigned_t<RawT>;

    std::string_view text;
    if constexpr (std::is_convertible_v<Input, std::string_view>) {
        text = std::string_view(std::forward<Input>(input));
    } else if constexpr (requires { input.as_str(); }) {
        text = std::string_view(input.as_str());
    } else {
        return Result<RawT, std::tuple<>>::Err(std::make_tuple());
    }

    const auto base_u64 = static_cast<std::uint64_t>(static_cast<RawRadix>(radix));
    if (base_u64 < 2 || base_u64 > 36) {
        return Result<RawT, std::tuple<>>::Err(std::make_tuple());
    }
    const auto base = static_cast<Unsigned>(base_u64);

    std::size_t idx = 0;
    bool negative = false;
    if (idx < text.size() && (text[idx] == '+' || text[idx] == '-')) {
        negative = (text[idx] == '-');
        idx++;
    }
    if (idx >= text.size()) {
        return Result<RawT, std::tuple<>>::Err(std::make_tuple());
    }

    Unsigned value = 0;
    for (; idx < text.size(); idx++) {
        const char ch = text[idx];
        Unsigned digit = 0;
        if (ch >= '0' && ch <= '9') {
            digit = static_cast<Unsigned>(ch - '0');
        } else if (ch >= 'a' && ch <= 'z') {
            digit = static_cast<Unsigned>(10 + (ch - 'a'));
        } else if (ch >= 'A' && ch <= 'Z') {
            digit = static_cast<Unsigned>(10 + (ch - 'A'));
        } else {
            return Result<RawT, std::tuple<>>::Err(std::make_tuple());
        }
        if (digit >= base) {
            return Result<RawT, std::tuple<>>::Err(std::make_tuple());
        }
        if (value > (std::numeric_limits<Unsigned>::max() - digit) / base) {
            return Result<RawT, std::tuple<>>::Err(std::make_tuple());
        }
        value = static_cast<Unsigned>(value * base + digit);
    }

    if constexpr (std::is_signed_v<RawT>) {
        if (negative) {
            const auto max_mag = static_cast<Unsigned>(std::numeric_limits<RawT>::max())
                + static_cast<Unsigned>(1);
            if (value > max_mag) {
                return Result<RawT, std::tuple<>>::Err(std::make_tuple());
            }
            if (value == max_mag) {
                return Result<RawT, std::tuple<>>::Ok(std::numeric_limits<RawT>::min());
            }
            const auto signed_value = static_cast<RawT>(value);
            return Result<RawT, std::tuple<>>::Ok(static_cast<RawT>(-signed_value));
        }
        if (value > static_cast<Unsigned>(std::numeric_limits<RawT>::max())) {
            return Result<RawT, std::tuple<>>::Err(std::make_tuple());
        }
        return Result<RawT, std::tuple<>>::Ok(static_cast<RawT>(value));
    } else {
        if (negative) {
            return Result<RawT, std::tuple<>>::Err(std::make_tuple());
        }
        if (value > std::numeric_limits<RawT>::max()) {
            return Result<RawT, std::tuple<>>::Err(std::make_tuple());
        }
        return Result<RawT, std::tuple<>>::Ok(static_cast<RawT>(value));
    }
}

} // namespace rusty

#endif // RUSTY_NUM_HPP
