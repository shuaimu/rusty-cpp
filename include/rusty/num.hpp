#ifndef RUSTY_NUM_HPP
#define RUSTY_NUM_HPP

#include <bit>
#include <array>
#include <cstddef>
#include <cstdint>
#include <cmath>
#include <limits>
#include <span>
#include <stdexcept>
#include <string_view>
#include <tuple>
#include <type_traits>
#include <utility>
#include <variant>

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
inline constexpr bool is_rust_integral_v =
    is_nonzero_integral_v<std::remove_cv_t<std::remove_reference_t<T>>>;

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

struct FpCategory_Nan {};
struct FpCategory_Infinite {};
struct FpCategory_Zero {};
struct FpCategory_Subnormal {};
struct FpCategory_Normal {};
using FpCategory = std::variant<
    FpCategory_Nan,
    FpCategory_Infinite,
    FpCategory_Zero,
    FpCategory_Subnormal,
    FpCategory_Normal>;

} // namespace rusty::num

// Checked arithmetic helpers (Rust integer methods returning Option<T>)
// Note: Option<T> must be defined before including this header (included via rusty.hpp)
namespace rusty {

template<typename T>
struct float_traits;

template<>
struct float_traits<float> {
    using SigType = std::uint32_t;

    static constexpr std::int32_t NUM_BITS = 32;
    static constexpr std::int32_t MANTISSA_DIGITS = 24;
    static constexpr std::int32_t NUM_SIG_BITS = MANTISSA_DIGITS - 1;
    static constexpr std::int32_t NUM_EXP_BITS = NUM_BITS - NUM_SIG_BITS - 1;
    static constexpr std::int32_t EXP_MASK = (static_cast<std::int32_t>(1) << NUM_EXP_BITS) - 1;
    static constexpr std::int32_t EXP_BIAS = (static_cast<std::int32_t>(1) << (NUM_EXP_BITS - 1)) - 1;
    static constexpr std::int32_t EXP_OFFSET = EXP_BIAS + NUM_SIG_BITS;
    static constexpr std::int32_t MIN_10_EXP = -37;
    static constexpr std::int32_t MAX_10_EXP = 38;
    static constexpr std::uint32_t MAX_DIGITS10 = 9;
    static constexpr SigType IMPLICIT_BIT = static_cast<SigType>(1) << NUM_SIG_BITS;

    static constexpr SigType to_bits(float value) noexcept {
        return std::bit_cast<SigType>(value);
    }

    static constexpr bool is_negative(SigType bits) noexcept {
        return (bits >> (NUM_BITS - 1)) != static_cast<SigType>(0);
    }

    static constexpr SigType get_sig(SigType bits) noexcept {
        return bits & (IMPLICIT_BIT - static_cast<SigType>(1));
    }

    static constexpr std::int64_t get_exp(SigType bits) noexcept {
        return static_cast<std::int64_t>((bits << 1u) >> (NUM_SIG_BITS + 1));
    }
};

template<>
struct float_traits<double> {
    using SigType = std::uint64_t;

    static constexpr std::int32_t NUM_BITS = 64;
    static constexpr std::int32_t MANTISSA_DIGITS = 53;
    static constexpr std::int32_t NUM_SIG_BITS = MANTISSA_DIGITS - 1;
    static constexpr std::int32_t NUM_EXP_BITS = NUM_BITS - NUM_SIG_BITS - 1;
    static constexpr std::int32_t EXP_MASK = (static_cast<std::int32_t>(1) << NUM_EXP_BITS) - 1;
    static constexpr std::int32_t EXP_BIAS = (static_cast<std::int32_t>(1) << (NUM_EXP_BITS - 1)) - 1;
    static constexpr std::int32_t EXP_OFFSET = EXP_BIAS + NUM_SIG_BITS;
    static constexpr std::int32_t MIN_10_EXP = -307;
    static constexpr std::int32_t MAX_10_EXP = 308;
    static constexpr std::uint32_t MAX_DIGITS10 = 17;
    static constexpr SigType IMPLICIT_BIT = static_cast<SigType>(1) << NUM_SIG_BITS;

    static constexpr SigType to_bits(double value) noexcept {
        return std::bit_cast<SigType>(value);
    }

    static constexpr bool is_negative(SigType bits) noexcept {
        return (bits >> (NUM_BITS - 1)) != static_cast<SigType>(0);
    }

    static constexpr SigType get_sig(SigType bits) noexcept {
        return bits & (IMPLICIT_BIT - static_cast<SigType>(1));
    }

    static constexpr std::int64_t get_exp(SigType bits) noexcept {
        return static_cast<std::int64_t>((bits << 1u) >> (NUM_SIG_BITS + 1));
    }
};

template<typename T>
constexpr auto float_to_bits(T&& value) noexcept {
    using Float = std::remove_cvref_t<T>;
    return float_traits<Float>::to_bits(static_cast<Float>(value));
}

template<typename T>
requires std::is_floating_point_v<std::remove_cvref_t<T>>
inline num::FpCategory classify_float(T value) {
    switch (std::fpclassify(static_cast<std::remove_cvref_t<T>>(value))) {
    case FP_NAN:
        return num::FpCategory{num::FpCategory_Nan{}};
    case FP_INFINITE:
        return num::FpCategory{num::FpCategory_Infinite{}};
    case FP_ZERO:
        return num::FpCategory{num::FpCategory_Zero{}};
    case FP_SUBNORMAL:
        return num::FpCategory{num::FpCategory_Subnormal{}};
    default:
        return num::FpCategory{num::FpCategory_Normal{}};
    }
}

template<typename T>
requires std::is_floating_point_v<std::remove_cvref_t<T>>
inline bool is_finite(T value) {
    return std::isfinite(static_cast<std::remove_cvref_t<T>>(value));
}

template<typename T>
requires std::is_floating_point_v<std::remove_cvref_t<T>>
inline bool is_nan(T value) {
    return std::isnan(static_cast<std::remove_cvref_t<T>>(value));
}

template<typename T>
requires std::is_floating_point_v<std::remove_cvref_t<T>>
inline bool is_infinite(T value) {
    return std::isinf(static_cast<std::remove_cvref_t<T>>(value));
}

template<typename T>
requires std::is_floating_point_v<std::remove_cvref_t<T>>
inline bool is_sign_negative(T value) {
    return std::signbit(static_cast<std::remove_cvref_t<T>>(value));
}

template<typename T>
requires std::is_floating_point_v<std::remove_cvref_t<T>>
inline bool is_sign_positive(T value) {
    return !std::signbit(static_cast<std::remove_cvref_t<T>>(value));
}

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

template<typename Target, typename Source>
requires(num::is_rust_integral_v<Target> && num::is_rust_integral_v<Source>)
Result<Target, std::tuple<>> try_from(Source value) {
    using Src = std::remove_cvref_t<Source>;
    auto fail = []() { return Result<Target, std::tuple<>>::Err(std::tuple<>{}); };

    if constexpr (std::is_signed_v<Src>) {
        const __int128 signed_value = static_cast<__int128>(value);
        if constexpr (std::is_signed_v<Target>) {
            const __int128 min_target = static_cast<__int128>(std::numeric_limits<Target>::min());
            const __int128 max_target = static_cast<__int128>(std::numeric_limits<Target>::max());
            if (signed_value < min_target || signed_value > max_target) {
                return fail();
            }
        } else {
            if (signed_value < 0) {
                return fail();
            }
            const unsigned __int128 unsigned_value = static_cast<unsigned __int128>(signed_value);
            const unsigned __int128 max_target =
                static_cast<unsigned __int128>(std::numeric_limits<Target>::max());
            if (unsigned_value > max_target) {
                return fail();
            }
        }
    } else {
        const unsigned __int128 unsigned_value = static_cast<unsigned __int128>(value);
        if constexpr (std::is_signed_v<Target>) {
            const unsigned __int128 max_target =
                static_cast<unsigned __int128>(std::numeric_limits<Target>::max());
            if (unsigned_value > max_target) {
                return fail();
            }
        } else {
            const unsigned __int128 max_target =
                static_cast<unsigned __int128>(std::numeric_limits<Target>::max());
            if (unsigned_value > max_target) {
                return fail();
            }
        }
    }

    return Result<Target, std::tuple<>>::Ok(static_cast<Target>(value));
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

template<typename T>
requires std::is_integral_v<T>
constexpr std::uint32_t leading_zeros(T value) {
    using U = std::make_unsigned_t<T>;
    return static_cast<std::uint32_t>(std::countl_zero(static_cast<U>(value)));
}

template<typename T>
constexpr std::uint32_t leading_zeros(const num::NonZero<T>& value) {
    return leading_zeros(value.get());
}

template<typename T>
requires std::is_integral_v<T>
constexpr std::uint32_t trailing_zeros(T value) {
    using U = std::make_unsigned_t<T>;
    return static_cast<std::uint32_t>(std::countr_zero(static_cast<U>(value)));
}

template<typename T>
constexpr std::uint32_t trailing_zeros(const num::NonZero<T>& value) {
    return trailing_zeros(value.get());
}

template<typename T>
requires std::is_integral_v<T>
constexpr std::uint32_t count_ones(T value) {
    using U = std::make_unsigned_t<T>;
    return static_cast<std::uint32_t>(std::popcount(static_cast<U>(value)));
}

template<typename T>
constexpr std::uint32_t count_ones(const num::NonZero<T>& value) {
    return count_ones(value.get());
}

template<typename T>
requires std::is_integral_v<T>
constexpr std::uint32_t count_zeros(T value) {
    return static_cast<std::uint32_t>(sizeof(T) * 8) - count_ones(value);
}

template<typename T>
constexpr std::uint32_t count_zeros(const num::NonZero<T>& value) {
    return count_zeros(value.get());
}

template<typename T>
requires num::is_nonzero_integral_v<std::remove_cvref_t<T>>
constexpr std::remove_cvref_t<T> wrapping_neg(T value) {
    using Raw = std::remove_cvref_t<T>;
    using Unsigned = std::make_unsigned_t<Raw>;
    return static_cast<Raw>(Unsigned(0) - static_cast<Unsigned>(value));
}

template<typename T>
requires num::is_nonzero_integral_v<std::remove_cvref_t<T>>
constexpr std::remove_cvref_t<T> wrapping_abs(T value) {
    using Raw = std::remove_cvref_t<T>;
    if constexpr (std::is_signed_v<Raw>) {
        return value < static_cast<Raw>(0) ? wrapping_neg(value) : static_cast<Raw>(value);
    } else {
        return static_cast<Raw>(value);
    }
}

template<typename T>
requires std::is_integral_v<T>
constexpr T byte_swap(T value) {
    using U = std::make_unsigned_t<T>;
    U u = static_cast<U>(value);
    if constexpr (sizeof(U) == 1) {
        return value;
    } else if constexpr (sizeof(U) == 2) {
        u = static_cast<U>(__builtin_bswap16(static_cast<std::uint16_t>(u)));
    } else if constexpr (sizeof(U) == 4) {
        u = static_cast<U>(__builtin_bswap32(static_cast<std::uint32_t>(u)));
    } else if constexpr (sizeof(U) == 8) {
        u = static_cast<U>(__builtin_bswap64(static_cast<std::uint64_t>(u)));
    } else {
        U swapped = 0;
        for (std::size_t i = 0; i < sizeof(U); ++i) {
            swapped = static_cast<U>((swapped << 8) | (u & static_cast<U>(0xff)));
            u = static_cast<U>(u >> 8);
        }
        u = swapped;
    }
    return static_cast<T>(u);
}

template<typename T>
requires std::is_integral_v<T>
constexpr T to_le(T value) {
    if constexpr (std::endian::native == std::endian::little) {
        return value;
    } else {
        return byte_swap(value);
    }
}

template<typename T>
requires std::is_integral_v<T>
constexpr T from_le(T value) {
    return to_le(value);
}

template<typename T>
requires std::is_integral_v<T>
constexpr T to_be(T value) {
    if constexpr (std::endian::native == std::endian::big) {
        return value;
    } else {
        return byte_swap(value);
    }
}

template<typename T>
requires std::is_integral_v<T>
constexpr T from_be(T value) {
    return to_be(value);
}

template<typename T, typename Input, typename Radix>
requires(
    num::is_nonzero_integral_v<std::remove_cvref_t<T>>
    && num::is_nonzero_integral_v<std::remove_cvref_t<Radix>>)
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

template<typename T, typename Bytes>
requires std::is_integral_v<std::remove_cvref_t<T>>
std::remove_cvref_t<T> from_le_bytes(Bytes&& bytes) {
    using RawT = std::remove_cvref_t<T>;
    using Unsigned = std::make_unsigned_t<RawT>;
    auto&& view = std::forward<Bytes>(bytes);
    const size_t count = std::size(view);
    if (count < sizeof(RawT)) {
        throw std::out_of_range("from_le_bytes input is too short");
    }
    Unsigned value = 0;
    for (size_t i = 0; i < sizeof(RawT); ++i) {
        value |= (static_cast<Unsigned>(static_cast<uint8_t>(std::data(view)[i])) << (i * 8));
    }
    return static_cast<RawT>(value);
}

} // namespace rusty

#endif // RUSTY_NUM_HPP
