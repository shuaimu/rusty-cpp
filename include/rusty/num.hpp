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

    // Rust `NonZero*::MAX` (Weak::new's dangling-sentinel address).
    inline static const NonZero<T> MAX = NonZero<T>(std::numeric_limits<T>::max());

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
    constexpr int trailing_zeros() const noexcept {
        using UnsignedT = std::make_unsigned_t<T>;
        return static_cast<int>(std::countr_zero(static_cast<UnsignedT>(value_)));
    }
    constexpr int count_ones() const noexcept {
        using UnsignedT = std::make_unsigned_t<T>;
        return static_cast<int>(std::popcount(static_cast<UnsignedT>(value_)));
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

/// std::num::ParseIntError / ParseFloatError ports — opaque error markers
/// (serde_yaml threads ParseIntError through `from_str_radix` fn pointers
/// without inspecting it).
struct ParseIntError {
    constexpr bool operator==(const ParseIntError&) const noexcept { return true; }
};
struct ParseFloatError {
    constexpr bool operator==(const ParseFloatError&) const noexcept { return true; }
};

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

// Unsigned-counterpart picker that also covers __int128, which
// std::make_unsigned does NOT handle under strict -std=c++23 (libstdc++
// guards the 128-bit specializations behind GNU mode).
template<typename T>
struct wrapping_unsigned {
    using type = std::make_unsigned_t<T>;
};
template<>
struct wrapping_unsigned<__int128> {
    using type = unsigned __int128;
};
template<>
struct wrapping_unsigned<unsigned __int128> {
    using type = unsigned __int128;
};
template<typename T>
using wrapping_unsigned_t = typename wrapping_unsigned<T>::type;

template<typename T>
inline constexpr bool wrapping_integral_v = std::is_integral_v<T>
    || std::is_same_v<T, __int128>
    || std::is_same_v<T, unsigned __int128>;

template<typename T>
requires wrapping_integral_v<T>
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
requires wrapping_integral_v<T>
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
requires wrapping_integral_v<T>
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

// Smallest multiple of `rhs` that is >= self, or None on rhs==0 / overflow.
// Mirrors Rust's checked_next_multiple_of for both signed and unsigned.
template<typename T>
requires(std::is_integral_v<T>)
Option<T> checked_next_multiple_of(T value, T rhs) {
    if (rhs == static_cast<T>(0)) {
        return Option<T>(rusty::None);
    }
    T r = static_cast<T>(value % rhs);
    if (r == static_cast<T>(0)) {
        return Option<T>(value);
    }
    // For signed types Rust rounds toward +inf: if the remainder has the
    // opposite sign of rhs, the next multiple is value + (rhs - r) after
    // normalizing r into rhs's sign domain.
    if constexpr (std::is_signed_v<T>) {
        if ((r < 0) != (rhs < 0)) {
            r = static_cast<T>(r + rhs);
        }
    }
    T delta = static_cast<T>(rhs - r);
    T sum;
    if (__builtin_add_overflow(value, delta, &sum)) {
        return Option<T>(rusty::None);
    }
    return Option<T>(sum);
}

// Integer square root (floor) via integer Newton's method — no FP rounding.
template<typename U>
requires(std::is_integral_v<U> && std::is_unsigned_v<U>)
constexpr U isqrt(U v) {
    if (v < 2) return v;
    U x0 = static_cast<U>(U(1) << ((std::bit_width(v) + 1) / 2));
    U x1 = static_cast<U>((x0 + v / x0) / 2);
    while (x1 < x0) {
        x0 = x1;
        x1 = static_cast<U>((x0 + v / x0) / 2);
    }
    return x0;
}
template<typename T>
requires(std::is_integral_v<T> && std::is_signed_v<T>)
constexpr T isqrt(T v) {
    // Rust panics on negatives for signed isqrt; mirror by clamping to 0.
    if (v < 0) return static_cast<T>(0);
    return static_cast<T>(isqrt<std::make_unsigned_t<T>>(static_cast<std::make_unsigned_t<T>>(v)));
}

template<typename T>
requires std::is_integral_v<T>
Option<T> checked_rem(T a, T b) {
    if (b == 0) return Option<T>(rusty::None);
    if constexpr (std::is_signed_v<T>) {
        if (a == std::numeric_limits<T>::min() && b == static_cast<T>(-1)) {
            return Option<T>(rusty::None);
        }
    }
    return Option<T>(static_cast<T>(a % b));
}

template<typename T>
requires std::is_integral_v<T>
Option<T> checked_neg(T x) {
    if constexpr (std::is_unsigned_v<T>) {
        return x == static_cast<T>(0) ? Option<T>(static_cast<T>(0)) : Option<T>(rusty::None);
    } else {
        return x == std::numeric_limits<T>::min() ? Option<T>(rusty::None)
                                                  : Option<T>(static_cast<T>(-x));
    }
}

template<typename T>
requires std::is_integral_v<T>
Option<T> checked_abs(T x) {
    if constexpr (std::is_unsigned_v<T>) {
        return Option<T>(x);
    } else {
        return x == std::numeric_limits<T>::min() ? Option<T>(rusty::None)
                                                  : Option<T>(static_cast<T>(x < 0 ? -x : x));
    }
}

template<typename T>
requires std::is_integral_v<T>
Option<T> checked_shl(T x, std::uint32_t rhs) {
    return rhs >= static_cast<std::uint32_t>(sizeof(T) * 8)
               ? Option<T>(rusty::None)
               : Option<T>(static_cast<T>(static_cast<std::make_unsigned_t<T>>(x) << rhs));
}

template<typename T>
requires std::is_integral_v<T>
Option<T> checked_shr(T x, std::uint32_t rhs) {
    return rhs >= static_cast<std::uint32_t>(sizeof(T) * 8)
               ? Option<T>(rusty::None)
               : Option<T>(static_cast<T>(x >> rhs));
}

template<typename T>
requires std::is_integral_v<T>
Option<T> checked_rem_euclid(T a, T b) {
    if (b == 0) return Option<T>(rusty::None);
    if constexpr (std::is_signed_v<T>) {
        if (a == std::numeric_limits<T>::min() && b == static_cast<T>(-1)) {
            return Option<T>(rusty::None);
        }
        T r = static_cast<T>(a % b);
        if (r < 0) r = static_cast<T>(r + (b < 0 ? static_cast<T>(-b) : b));
        return Option<T>(r);
    } else {
        return Option<T>(static_cast<T>(a % b));
    }
}

template<typename T>
requires std::is_integral_v<T>
Option<T> checked_div_euclid(T a, T b) {
    if (b == 0) return Option<T>(rusty::None);
    if constexpr (std::is_signed_v<T>) {
        if (a == std::numeric_limits<T>::min() && b == static_cast<T>(-1)) {
            return Option<T>(rusty::None);
        }
        T q = static_cast<T>(a / b);
        if ((a % b) < 0) q = static_cast<T>(q + (b > 0 ? static_cast<T>(-1) : static_cast<T>(1)));
        return Option<T>(q);
    } else {
        return Option<T>(static_cast<T>(a / b));
    }
}

template<typename T>
requires std::is_integral_v<T>
Option<std::uint32_t> checked_ilog2(T x) {
    if (x <= 0) return Option<std::uint32_t>(rusty::None);
    return Option<std::uint32_t>(
        static_cast<std::uint32_t>(std::bit_width(static_cast<std::make_unsigned_t<T>>(x)) - 1));
}

template<typename T>
requires(std::is_integral_v<T> && std::is_signed_v<T>)
Option<T> checked_isqrt(T x) {
    if (x < 0) return Option<T>(rusty::None);
    return Option<T>(isqrt(x));
}

// u{N}::checked_add_signed(self, rhs: i{N}). Wrapping add + overflow detect.
template<typename T, typename S>
requires(std::is_integral_v<T> && std::is_unsigned_v<T> && std::is_integral_v<std::remove_cvref_t<S>>
         && std::is_signed_v<std::remove_cvref_t<S>>)
Option<T> checked_add_signed(T a, S b) {
    T r = static_cast<T>(a + static_cast<T>(b));
    bool overflow = (b >= 0) ? (r < a) : (r > a);
    return overflow ? Option<T>(rusty::None) : Option<T>(r);
}

template<typename T>
requires std::is_integral_v<T>
Option<T> checked_pow(T base, std::uint32_t exp) {
    // Rust's exponentiation-by-squaring guarded by checked_mul.
    T acc = static_cast<T>(1);
    while (exp > 1) {
        if ((exp & 1u) != 0) {
            if (__builtin_mul_overflow(acc, base, &acc)) return Option<T>(rusty::None);
        }
        exp >>= 1;
        if (__builtin_mul_overflow(base, base, &base)) return Option<T>(rusty::None);
    }
    if (exp == 1) {
        if (__builtin_mul_overflow(acc, base, &acc)) return Option<T>(rusty::None);
    }
    return Option<T>(acc);
}

template<typename T>
requires(std::is_integral_v<T> && std::is_signed_v<T>)
constexpr T saturating_abs(T x) {
    return x == std::numeric_limits<T>::min() ? std::numeric_limits<T>::max()
                                              : static_cast<T>(x < 0 ? -x : x);
}

template<typename T>
requires(std::is_integral_v<T> && std::is_signed_v<T>)
constexpr T saturating_neg(T x) {
    return x == std::numeric_limits<T>::min() ? std::numeric_limits<T>::max()
                                              : static_cast<T>(-x);
}

template<typename T>
requires std::is_integral_v<T>
constexpr T saturating_div(T a, T b) {
    if (b == 0) __builtin_trap();  // Rust panics on divide-by-zero
    if constexpr (std::is_signed_v<T>) {
        if (a == std::numeric_limits<T>::min() && b == static_cast<T>(-1)) {
            return std::numeric_limits<T>::max();
        }
    }
    return static_cast<T>(a / b);
}

template<typename T>
constexpr T detail_saturating_mul(T a, T b) {
    T r;
    if (__builtin_mul_overflow(a, b, &r)) {
        if constexpr (std::is_signed_v<T>) {
            return ((a < 0) != (b < 0)) ? std::numeric_limits<T>::min()
                                        : std::numeric_limits<T>::max();
        } else {
            return std::numeric_limits<T>::max();
        }
    }
    return r;
}

template<typename T>
requires std::is_integral_v<T>
constexpr T saturating_pow(T base, std::uint32_t exp) {
    T acc = static_cast<T>(1);
    while (exp > 1) {
        if ((exp & 1u) != 0) acc = detail_saturating_mul<T>(acc, base);
        exp >>= 1;
        base = detail_saturating_mul<T>(base, base);
    }
    return exp == 1 ? detail_saturating_mul<T>(acc, base) : acc;
}

// i{N}::saturating_{add,sub}_unsigned(rhs: u{N}). rhs >= 0, so add only
// overflows high and sub only underflows low.
template<typename T>
requires(std::is_integral_v<T> && std::is_signed_v<T>)
constexpr T saturating_add_unsigned(T a, std::make_unsigned_t<T> b) {
    using UT = std::make_unsigned_t<T>;
    T r = static_cast<T>(static_cast<UT>(a) + b);
    return r < a ? std::numeric_limits<T>::max() : r;
}

template<typename T>
requires(std::is_integral_v<T> && std::is_signed_v<T>)
constexpr T saturating_sub_unsigned(T a, std::make_unsigned_t<T> b) {
    using UT = std::make_unsigned_t<T>;
    T r = static_cast<T>(static_cast<UT>(a) - b);
    return r > a ? std::numeric_limits<T>::min() : r;
}

// Rust wrapping_{add,sub,mul,div,rem}: modular arithmetic in the operand's
// own width. Computed in the unsigned counterpart (well-defined wrap) and
// cast back. div/rem never wrap (only /0, which traps in Rust too).
template<typename T>
requires wrapping_integral_v<T>
constexpr T wrapping_add(T a, T b) {
    using U = wrapping_unsigned_t<T>;
    return static_cast<T>(static_cast<U>(a) + static_cast<U>(b));
}

template<typename T>
requires wrapping_integral_v<T>
constexpr T wrapping_sub(T a, T b) {
    using U = wrapping_unsigned_t<T>;
    return static_cast<T>(static_cast<U>(a) - static_cast<U>(b));
}

template<typename T>
requires wrapping_integral_v<T>
constexpr T wrapping_mul(T a, T b) {
    using U = wrapping_unsigned_t<T>;
    return static_cast<T>(static_cast<U>(a) * static_cast<U>(b));
}

template<typename T>
requires wrapping_integral_v<T>
constexpr T wrapping_div(T a, T b) {
    return static_cast<T>(a / b);
}

template<typename T>
requires wrapping_integral_v<T>
constexpr T wrapping_rem(T a, T b) {
    return static_cast<T>(a % b);
}

template<typename T>
requires std::is_integral_v<T>
constexpr T wrapping_pow(T base, std::uint32_t exp) {
    using U = std::make_unsigned_t<T>;
    U b = static_cast<U>(base);
    U acc = static_cast<U>(1);
    while (exp > 1) {
        if ((exp & 1u) != 0) acc = static_cast<U>(acc * b);
        exp >>= 1;
        b = static_cast<U>(b * b);
    }
    if (exp == 1) acc = static_cast<U>(acc * b);
    return static_cast<T>(acc);
}

template<typename T>
requires(std::is_integral_v<T> && std::is_unsigned_v<T>)
constexpr T wrapping_next_power_of_two(T v) {
    if (v <= static_cast<T>(1)) return static_cast<T>(1);
    const int w = std::bit_width(static_cast<T>(v - 1));
    return w >= static_cast<int>(sizeof(T) * 8) ? static_cast<T>(0)
                                                : static_cast<T>(static_cast<T>(1) << w);
}

// Rust `i*::abs` / `f*::abs`. A member abs() wins (user numeric wrappers);
// unsigned values pass through; signed/floating flip the sign. Routed for
// unknown-typed receivers (closure params), so member preference keeps
// non-primitive receivers on their own method.
template<typename T>
constexpr auto abs(T&& value) {
    using Plain = std::remove_cvref_t<T>;
    if constexpr (requires { std::forward<T>(value).abs(); }) {
        return std::forward<T>(value).abs();
    } else if constexpr (std::is_unsigned_v<Plain>) {
        return Plain(value);
    } else {
        return value < Plain{} ? Plain(-value) : Plain(value);
    }
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

// Rust's integer `div_ceil` (ceiling division). Overflow-safe form:
// quotient plus one when a remainder exists.
template<typename T, typename U>
requires std::is_integral_v<T>
constexpr T div_ceil(T value, U rhs) {
    const T d = static_cast<T>(rhs);
    return static_cast<T>(value / d + ((value % d) != T{0} ? T{1} : T{0}));
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

// 128-bit overloads: std::countl_zero/countr_zero/popcount do not accept
// unsigned __int128; split into 64-bit halves.
constexpr std::uint32_t leading_zeros(unsigned __int128 value) {
    const std::uint64_t hi = static_cast<std::uint64_t>(value >> 64);
    if (hi != 0) {
        return static_cast<std::uint32_t>(std::countl_zero(hi));
    }
    const std::uint64_t lo = static_cast<std::uint64_t>(value);
    return 64u + static_cast<std::uint32_t>(std::countl_zero(lo));
}
constexpr std::uint32_t leading_zeros(__int128 value) {
    return leading_zeros(static_cast<unsigned __int128>(value));
}
constexpr std::uint32_t trailing_zeros(unsigned __int128 value) {
    const std::uint64_t lo = static_cast<std::uint64_t>(value);
    if (lo != 0) {
        return static_cast<std::uint32_t>(std::countr_zero(lo));
    }
    const std::uint64_t hi = static_cast<std::uint64_t>(value >> 64);
    return 64u + static_cast<std::uint32_t>(std::countr_zero(hi));
}
constexpr std::uint32_t trailing_zeros(__int128 value) {
    return trailing_zeros(static_cast<unsigned __int128>(value));
}
constexpr std::uint32_t count_ones(unsigned __int128 value) {
    const std::uint64_t hi = static_cast<std::uint64_t>(value >> 64);
    const std::uint64_t lo = static_cast<std::uint64_t>(value);
    return static_cast<std::uint32_t>(std::popcount(hi) + std::popcount(lo));
}
constexpr std::uint32_t count_ones(__int128 value) {
    return count_ones(static_cast<unsigned __int128>(value));
}
constexpr std::uint32_t count_zeros(unsigned __int128 value) {
    return 128u - count_ones(value);
}
constexpr std::uint32_t count_zeros(__int128 value) {
    return 128u - count_ones(value);
}

// Rust `iN/uN::swap_bytes(self) -> Self` — reverse the byte order.
template<typename T>
requires std::is_integral_v<T>
constexpr T swap_bytes(T value) {
    using U = std::make_unsigned_t<T>;
    return static_cast<T>(std::byteswap(static_cast<U>(value)));
}

// Rust `uN::is_power_of_two(self) -> bool` — exactly one bit set (0 is not a power of two).
template<typename T>
requires std::is_integral_v<T>
constexpr bool is_power_of_two(T value) {
    using U = std::make_unsigned_t<T>;
    return std::has_single_bit(static_cast<U>(value));
}

// Rust `uN::next_power_of_two(self)` — the smallest power of two >= self
// (1 for 0; overflow panics in debug, wraps to 0 via bit_ceil UB-avoidance
// here matching release semantics closely enough for transpiled corpora).
template<typename T>
requires std::is_integral_v<T>
constexpr T next_power_of_two(T value) {
    using U = std::make_unsigned_t<T>;
    return static_cast<T>(std::bit_ceil(static_cast<U>(value)));
}

template<typename T>
requires num::is_nonzero_integral_v<std::remove_cvref_t<T>>
constexpr std::remove_cvref_t<T> wrapping_neg(T value) {
    using Raw = std::remove_cvref_t<T>;
    using Unsigned = wrapping_unsigned_t<Raw>;
    return static_cast<Raw>(Unsigned(0) - static_cast<Unsigned>(value));
}

template<typename T>
requires num::is_nonzero_integral_v<std::remove_cvref_t<T>>
constexpr std::remove_cvref_t<T> wrapping_abs(T value) {
    using Raw = std::remove_cvref_t<T>;
    if constexpr (std::is_signed_v<Raw> || std::is_same_v<Raw, __int128>) {
        return value < static_cast<Raw>(0) ? wrapping_neg(value) : static_cast<Raw>(value);
    } else {
        return static_cast<Raw>(value);
    }
}

// `T::from(x)` on a generic type param: primitives have no static from —
// dispatch the member when present, else convert. Mirrors default_like.
template<typename T, typename V>
constexpr T from_like(V&& v) {
    if constexpr (requires { T::from(std::forward<V>(v)); }) {
        return T::from(std::forward<V>(v));
    } else if constexpr (requires { T::from_(std::forward<V>(v)); }) {
        return T::from_(std::forward<V>(v));
    } else {
        return static_cast<T>(std::forward<V>(v));
    }
}

// Rust float→int `as` cast: SATURATES (out-of-range clamps to the target's
// MIN/MAX, NaN → 0) where a bare static_cast is UB on both. The range bound
// 2^(bits[-1]) is an exact power of two in any float format; every value at
// or above it (or below the signed lower bound) is out of range, and every
// value strictly inside truncates in range (floats in (MIN-1, MIN] truncate
// to MIN). For float→u128 the bound 2^128 overflows to +inf, which still
// classifies correctly: no finite float exceeds u128::MAX. Target MIN/MAX
// are computed bitwise so __int128 works without numeric_limits.
template<typename To, typename From>
requires(
    std::is_floating_point_v<std::remove_cvref_t<From>>
    && num::is_nonzero_integral_v<To>)
constexpr To float_to_int_cast(From value_in) {
    using F = std::remove_cvref_t<From>;
    F value = static_cast<F>(value_in);
    if (value != value) {
        return To(0);
    }
    using U = wrapping_unsigned_t<To>;
    constexpr bool to_signed = std::is_signed_v<To> || std::is_same_v<To, __int128>;
    constexpr To to_max = to_signed ? To(U(~U(0)) >> 1) : To(~U(0));
    constexpr To to_min = to_signed ? To(To(0) - to_max - To(1)) : To(0);
    constexpr int bound_bits =
        to_signed ? int(sizeof(To) * 8) - 1 : int(sizeof(To) * 8);
    F upper = F(1);
    for (int i = 0; i < bound_bits; ++i) {
        upper *= F(2);
    }
    if (value >= upper) {
        return to_max;
    }
    if constexpr (to_signed) {
        if (value < -upper) {
            return to_min;
        }
    } else {
        if (value < F(0)) {
            return To(0);
        }
    }
    return static_cast<To>(value);
}

// Rust `overflowing_neg` — (wrapping_neg(self), overflowed). Signed types
// overflow only at MIN (the one value whose wrapped negation is itself);
// unsigned types overflow for every nonzero value. std::is_signed_v is
// false for __int128 under strict C++23, so it is special-cased.
template<typename T>
requires num::is_nonzero_integral_v<std::remove_cvref_t<T>>
constexpr auto overflowing_neg(T value) {
    using Raw = std::remove_cvref_t<T>;
    Raw wrapped = wrapping_neg(value);
    bool overflowed;
    if constexpr (std::is_signed_v<Raw> || std::is_same_v<Raw, __int128>) {
        overflowed = value != Raw(0) && wrapped == value;
    } else {
        overflowed = value != Raw(0);
    }
    return std::make_tuple(wrapped, overflowed);
}

// Rust `overflowing_abs` — (wrapping_abs(self), self == MIN). Only MIN's
// wrapped absolute value stays negative, so that is the overflow test.
template<typename T>
requires num::is_nonzero_integral_v<std::remove_cvref_t<T>>
constexpr auto overflowing_abs(T value) {
    using Raw = std::remove_cvref_t<T>;
    if constexpr (std::is_signed_v<Raw> || std::is_same_v<Raw, __int128>) {
        Raw wrapped = wrapping_abs(value);
        return std::make_tuple(wrapped, wrapped < Raw(0));
    } else {
        return std::make_tuple(static_cast<Raw>(value), false);
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
constexpr std::remove_cvref_t<T> from_le_bytes(Bytes&& bytes) {
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

// Big-endian byte order: most-significant byte first.
template<typename T, typename Bytes>
requires std::is_integral_v<std::remove_cvref_t<T>>
constexpr std::remove_cvref_t<T> from_be_bytes(Bytes&& bytes) {
    using RawT = std::remove_cvref_t<T>;
    using Unsigned = std::make_unsigned_t<RawT>;
    auto&& view = std::forward<Bytes>(bytes);
    const size_t count = std::size(view);
    if (count < sizeof(RawT)) {
        throw std::out_of_range("from_be_bytes input is too short");
    }
    Unsigned value = 0;
    for (size_t i = 0; i < sizeof(RawT); ++i) {
        value |= (static_cast<Unsigned>(static_cast<uint8_t>(std::data(view)[i]))
                  << ((sizeof(RawT) - 1 - i) * 8));
    }
    return static_cast<RawT>(value);
}

// Native byte order: little- or big-endian per the host (std::endian::native).
template<typename T, typename Bytes>
requires std::is_integral_v<std::remove_cvref_t<T>>
constexpr std::remove_cvref_t<T> from_ne_bytes(Bytes&& bytes) {
    if constexpr (std::endian::native == std::endian::big) {
        return from_be_bytes<T>(std::forward<Bytes>(bytes));
    } else {
        return from_le_bytes<T>(std::forward<Bytes>(bytes));
    }
}

} // namespace rusty

#endif // RUSTY_NUM_HPP
