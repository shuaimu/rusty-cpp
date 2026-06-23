#ifndef RUSTY_FMT_RT_HPP
#define RUSTY_FMT_RT_HPP

// Self-contained (no-std) formatting runtime — Phase 0 scaffolding.
//
// Developed in the `rusty::fmt::rt` namespace, SEPARATE from the live
// `rusty::fmt::Formatter` stub, so building it out has zero impact on existing
// transpiled code until a deliberate cutover (when the full surface — Phase 2's
// Debug builders — is ready, `rt` is promoted to `rusty::fmt` and the
// transpiler is repointed). This keeps each phase additive and matrix-safe.
//
// Constraints (deliberate): the formatting LOGIC pulls in no std runtime
// library — no <format>, <string>, <vector>, <charconv>/to_chars or <ostream>.
// Only freestanding language-support headers and `std::string_view` (the
// transpiler's `&str` ABI, a non-owning view) are used. Output grows through
// `rusty::alloc` (the runtime's own allocator), never std::string/std::vector.
//
// Layering (built bottom-up across phases):
//   Phase 0 (here): Buffer (the sink) + FormatSpec + Formatter skeleton
//                   (write_str/write_char/pad + flag accessors).
//   Phase 1: integer/str/bool/char primitive formatting.
//   Phase 2: the Debug builders (DebugStruct/Tuple/List/Set/Map).
//   Phase 3: f32/f64 shortest round-trip (Ryū).

#include <bit>
#include <cstddef>
#include <cstdint>
#include <string_view>
#include <tuple>
#include <type_traits>
#include "rusty/alloc.hpp"
#include "rusty/fmt.hpp"   // rusty::fmt::Error / Result / trait stubs

namespace rusty {
namespace fmt {
namespace rt {

using rusty::fmt::Error;
using rusty::fmt::Result;

inline Result ok() { return Result::Ok(std::make_tuple()); }

/// Radix for `{:x}` / `{:X}` / `{:o}` / `{:b}` integer formatting.
enum class Base : std::uint8_t { LowerHex, UpperHex, Octal, Binary };

// ---------------------------------------------------------------------------
// Buffer — a growable byte sink backed by rusty::alloc (no std::string/vector).
// ---------------------------------------------------------------------------
class Buffer {
    std::uint8_t* data_ = nullptr;
    std::size_t len_ = 0;
    std::size_t cap_ = 0;
    // Byte buffer — alignment 1 is sufficient for character data.
    static constexpr std::size_t ALIGN = 1;

    void reserve(std::size_t additional) {
        std::size_t needed = len_ + additional;
        if (needed <= cap_) {
            return;
        }
        std::size_t new_cap = cap_ ? cap_ : 16;
        while (new_cap < needed) {
            new_cap *= 2;
        }
        std::uint8_t* new_data =
            data_ ? rusty::alloc::realloc(data_, rusty::alloc::Layout{cap_, ALIGN}, new_cap)
                  : rusty::alloc::alloc(rusty::alloc::Layout{new_cap, ALIGN});
        data_ = new_data;
        cap_ = new_cap;
    }

public:
    Buffer() = default;
    Buffer(const Buffer&) = delete;
    Buffer& operator=(const Buffer&) = delete;
    Buffer(Buffer&& other) noexcept
        : data_(other.data_), len_(other.len_), cap_(other.cap_) {
        other.data_ = nullptr;
        other.len_ = 0;
        other.cap_ = 0;
    }
    ~Buffer() {
        if (data_) {
            rusty::alloc::dealloc(data_, rusty::alloc::Layout{cap_, ALIGN});
        }
    }

    void push_bytes(const char* p, std::size_t n) {
        if (n == 0) {
            return;
        }
        reserve(n);
        for (std::size_t i = 0; i < n; ++i) {
            data_[len_ + i] = static_cast<std::uint8_t>(p[i]);
        }
        len_ += n;
    }
    void push_byte(char c) { push_bytes(&c, 1); }
    void push_str(std::string_view s) { push_bytes(s.data(), s.size()); }

    std::string_view view() const {
        return data_ ? std::string_view(reinterpret_cast<const char*>(data_), len_)
                     : std::string_view();
    }
    std::size_t len() const { return len_; }
    bool is_empty() const { return len_ == 0; }
    void clear() { len_ = 0; }
};

// ---------------------------------------------------------------------------
// FormatSpec — the active format flags (`{:fill<+#0width.prec}`).
// ---------------------------------------------------------------------------
enum class Alignment : std::uint8_t { Left, Right, Center, Unknown };

struct FormatSpec {
    std::size_t width = 0;
    bool has_width = false;
    std::size_t precision = 0;
    bool has_precision = false;
    char fill = ' ';
    Alignment align = Alignment::Unknown;
    bool alternate = false;             // `{:#}`
    bool sign_plus = false;             // `{:+}`
    bool sign_aware_zero_pad = false;   // `{:0}`
};

// Debug-builder classes (defined after the primitives they recurse into).
class DebugStruct;
class DebugTuple;
class DebugList;
class DebugSet;
class DebugMap;

// ---------------------------------------------------------------------------
// Formatter — what transpiled `fmt(Formatter&)` methods will write into (after
// cutover). Phase 0: the sink + spec + write_str/write_char/pad + flag accessors.
// ---------------------------------------------------------------------------
class Formatter {
    Buffer& buf_;
    FormatSpec spec_;

public:
    explicit Formatter(Buffer& buf) : buf_(buf) {}
    Formatter(Buffer& buf, const FormatSpec& spec) : buf_(buf), spec_(spec) {}

    Buffer& buffer() { return buf_; }
    const FormatSpec& spec() const { return spec_; }
    void set_spec(const FormatSpec& spec) { spec_ = spec; }

    Result write_str(std::string_view s) {
        buf_.push_str(s);
        return ok();
    }
    Result write_char(char c) {
        buf_.push_byte(c);
        return ok();
    }

    // Rust `Formatter` flag accessors. `width()`/`precision()` are
    // `Option<usize>` in Rust; we expose has_*/value rather than an Option here
    // to keep the runtime dependency-free (the transpiler maps the Rust calls).
    bool alternate() const { return spec_.alternate; }
    bool sign_plus() const { return spec_.sign_plus; }
    bool sign_aware_zero_pad() const { return spec_.sign_aware_zero_pad; }
    bool has_width() const { return spec_.has_width; }
    std::size_t width_or(std::size_t fallback) const {
        return spec_.has_width ? spec_.width : fallback;
    }
    bool has_precision() const { return spec_.has_precision; }
    std::size_t precision_or(std::size_t fallback) const {
        return spec_.has_precision ? spec_.precision : fallback;
    }
    char fill() const { return spec_.fill; }
    Alignment align() const { return spec_.align; }

    // `Formatter::pad` — apply precision (truncate) then width/fill/align to a
    // pre-rendered string. This is the string-formatting path (Display for
    // `&str`); integer/float paths use `pad_integral` (Phase 1). Width is
    // counted in BYTES here (correct for ASCII); Unicode scalar-count width is
    // a Phase-1 refinement.
    Result pad(std::string_view s) {
        if (spec_.has_precision && s.size() > spec_.precision) {
            s = s.substr(0, spec_.precision);
        }
        if (!spec_.has_width || s.size() >= spec_.width) {
            return write_str(s);
        }
        std::size_t padding = spec_.width - s.size();
        // Default alignment for `pad` (strings) is LEFT.
        Alignment align =
            spec_.align == Alignment::Unknown ? Alignment::Left : spec_.align;
        auto pad_n = [&](std::size_t n) {
            for (std::size_t i = 0; i < n; ++i) {
                buf_.push_byte(spec_.fill);
            }
        };
        switch (align) {
            case Alignment::Left:
                write_str(s);
                pad_n(padding);
                break;
            case Alignment::Right:
                pad_n(padding);
                write_str(s);
                break;
            case Alignment::Center:
                pad_n(padding / 2);
                write_str(s);
                pad_n(padding - padding / 2);
                break;
            case Alignment::Unknown:
                write_str(s);
                pad_n(padding);
                break;
        }
        return ok();
    }

    // `Formatter::pad_integral` — the numeric formatting path (Display/Debug for
    // integers and the radix traits). Lays out `[sign][prefix][precision-zeros]
    // [digits]` and applies width via: sign-aware zero pad (zeros between prefix
    // and digits), else fill/align (default alignment RIGHT). `prefix` (e.g.
    // "0x"/"0o"/"0b") is emitted only when the `#` alternate flag is set;
    // `min_digits` is the integer precision (minimum digit count, zero-padded).
    Result pad_integral(bool is_nonneg, std::string_view prefix,
                        std::string_view digits, std::size_t min_digits = 0) {
        std::string_view sign =
            !is_nonneg ? std::string_view("-")
                       : (spec_.sign_plus ? std::string_view("+") : std::string_view());
        std::string_view pfx = spec_.alternate ? prefix : std::string_view();
        std::size_t prec_zeros = digits.size() < min_digits ? min_digits - digits.size() : 0;

        auto emit_n = [&](std::size_t n, char c) {
            for (std::size_t i = 0; i < n; ++i) {
                buf_.push_byte(c);
            }
        };
        auto emit_content = [&] {
            write_str(sign);
            write_str(pfx);
            emit_n(prec_zeros, '0');
            write_str(digits);
        };

        std::size_t content_width = sign.size() + pfx.size() + prec_zeros + digits.size();
        if (!spec_.has_width || content_width >= spec_.width) {
            emit_content();
            return ok();
        }
        std::size_t padding = spec_.width - content_width;

        if (spec_.sign_aware_zero_pad) {
            // `{:08}` — zeros go AFTER sign/prefix, ignoring fill/align.
            write_str(sign);
            write_str(pfx);
            emit_n(padding, '0');
            emit_n(prec_zeros, '0');
            write_str(digits);
            return ok();
        }

        Alignment align =
            spec_.align == Alignment::Unknown ? Alignment::Right : spec_.align;
        switch (align) {
            case Alignment::Left:
                emit_content();
                emit_n(padding, spec_.fill);
                break;
            case Alignment::Center:
                emit_n(padding / 2, spec_.fill);
                emit_content();
                emit_n(padding - padding / 2, spec_.fill);
                break;
            case Alignment::Right:
            case Alignment::Unknown:
                emit_n(padding, spec_.fill);
                emit_content();
                break;
        }
        return ok();
    }

    // Debug builders (`#[derive(Debug)]` / hand-written Debug impls). Defined
    // out-of-line once the builder classes are complete.
    DebugStruct debug_struct(std::string_view name);
    DebugTuple debug_tuple(std::string_view name);
    DebugList debug_list();
    DebugSet debug_set();
    DebugMap debug_map();
};

namespace detail {

// Render an unsigned magnitude as digits in `radix` into `out` (caller buffer,
// >= 128 bytes covers u128 binary), most-significant first. Returns count.
inline std::size_t to_radix_digits(unsigned __int128 mag, unsigned radix,
                                   bool upper, char* out) {
    static const char lower[] = "0123456789abcdef";
    static const char upper_d[] = "0123456789ABCDEF";
    const char* d = upper ? upper_d : lower;
    char tmp[130];
    std::size_t n = 0;
    if (mag == 0) {
        tmp[n++] = '0';
    }
    while (mag != 0) {
        tmp[n++] = d[static_cast<unsigned>(mag % radix)];
        mag /= radix;
    }
    for (std::size_t i = 0; i < n; ++i) {
        out[i] = tmp[n - 1 - i];
    }
    return n;
}

// UTF-8 encode a Unicode scalar into `out` (>= 4 bytes). Returns byte count.
inline std::size_t utf8_encode(char32_t c, char* out) {
    if (c < 0x80) {
        out[0] = static_cast<char>(c);
        return 1;
    } else if (c < 0x800) {
        out[0] = static_cast<char>(0xC0 | (c >> 6));
        out[1] = static_cast<char>(0x80 | (c & 0x3F));
        return 2;
    } else if (c < 0x10000) {
        out[0] = static_cast<char>(0xE0 | (c >> 12));
        out[1] = static_cast<char>(0x80 | ((c >> 6) & 0x3F));
        out[2] = static_cast<char>(0x80 | (c & 0x3F));
        return 3;
    }
    out[0] = static_cast<char>(0xF0 | (c >> 18));
    out[1] = static_cast<char>(0x80 | ((c >> 12) & 0x3F));
    out[2] = static_cast<char>(0x80 | ((c >> 6) & 0x3F));
    out[3] = static_cast<char>(0x80 | (c & 0x3F));
    return 4;
}

}  // namespace detail

// Integer Display/Debug: signed-aware DECIMAL (sign + magnitude).
template<typename T>
    requires(std::is_integral_v<T> && !std::is_same_v<std::remove_cv_t<T>, bool>)
inline Result fmt_int(Formatter& f, T value) {
    bool nonneg = true;
    unsigned __int128 mag;
    if constexpr (std::is_signed_v<T>) {
        nonneg = value >= 0;
        using U = std::make_unsigned_t<T>;
        U u = static_cast<U>(value);
        mag = nonneg ? static_cast<unsigned __int128>(u)
                     : static_cast<unsigned __int128>(static_cast<U>(static_cast<U>(0) - u));
    } else {
        mag = static_cast<unsigned __int128>(value);
    }
    char buf[130];
    std::size_t n = detail::to_radix_digits(mag, 10, false, buf);
    // Integer formatting IGNORES precision (only str/float honor it).
    return f.pad_integral(nonneg, std::string_view(), std::string_view(buf, n));
}

// Integer radix (`{:x}`/`{:X}`/`{:o}`/`{:b}`): the raw bit pattern, UNSIGNED
// (two's-complement for signed inputs — `format!("{:x}", -5i32) == "fffffffb"`).
template<typename T>
    requires(std::is_integral_v<T> && !std::is_same_v<std::remove_cv_t<T>, bool>)
inline Result fmt_int_radix(Formatter& f, T value, Base base) {
    using U = std::make_unsigned_t<T>;
    unsigned __int128 bits = static_cast<unsigned __int128>(static_cast<U>(value));
    unsigned radix = 16;
    bool upper = false;
    std::string_view prefix = "0x";
    switch (base) {
        case Base::LowerHex: radix = 16; upper = false; prefix = "0x"; break;
        case Base::UpperHex: radix = 16; upper = true;  prefix = "0x"; break;
        case Base::Octal:    radix = 8;  upper = false; prefix = "0o"; break;
        case Base::Binary:   radix = 2;  upper = false; prefix = "0b"; break;
    }
    char buf[130];
    std::size_t n = detail::to_radix_digits(bits, radix, upper, buf);
    // Integer formatting IGNORES precision (only str/float honor it).
    return f.pad_integral(true, prefix, std::string_view(buf, n));
}

// `bool` Display/Debug: "true"/"false", honoring width/precision via pad.
inline Result fmt_bool(Formatter& f, bool value) {
    return f.pad(value ? std::string_view("true") : std::string_view("false"));
}

// `&str` Debug: `"..."` with escaping. ASCII-correct; printable non-ASCII (e.g.
// 'é') passes through. Non-printable non-ASCII (which Rust escapes via Unicode
// tables) is a documented later refinement.
inline Result fmt_str_debug(Formatter& f, std::string_view s) {
    f.write_char('"');
    for (std::size_t i = 0; i < s.size(); ++i) {
        unsigned char c = static_cast<unsigned char>(s[i]);
        switch (c) {
            case '"':  f.write_str("\\\""); break;
            case '\\': f.write_str("\\\\"); break;
            case '\n': f.write_str("\\n"); break;
            case '\r': f.write_str("\\r"); break;
            case '\t': f.write_str("\\t"); break;
            case '\0': f.write_str("\\0"); break;
            default:
                if (c < 0x20 || c == 0x7f) {
                    char hb[3];
                    std::size_t hn = detail::to_radix_digits(c, 16, false, hb);
                    f.write_str("\\u{");
                    f.write_str(std::string_view(hb, hn));
                    f.write_char('}');
                } else {
                    f.write_char(static_cast<char>(c));
                }
        }
    }
    f.write_char('"');
    return ok();
}

// `char` Display: the UTF-8 of the scalar, honoring width via pad.
inline Result fmt_char_display(Formatter& f, char32_t c) {
    char b[4];
    std::size_t n = detail::utf8_encode(c, b);
    return f.pad(std::string_view(b, n));
}

// `char` Debug: `'c'` with escaping (same rules as str Debug, plus `\'`).
inline Result fmt_char_debug(Formatter& f, char32_t c) {
    f.write_char('\'');
    switch (c) {
        case U'\'': f.write_str("\\'"); break;
        case U'\\': f.write_str("\\\\"); break;
        case U'\n': f.write_str("\\n"); break;
        case U'\r': f.write_str("\\r"); break;
        case U'\t': f.write_str("\\t"); break;
        case 0:     f.write_str("\\0"); break;
        default:
            if (c < 0x20 || c == 0x7f) {
                char hb[8];
                std::size_t hn = detail::to_radix_digits(c, 16, false, hb);
                f.write_str("\\u{");
                f.write_str(std::string_view(hb, hn));
                f.write_char('}');
            } else {
                char b[4];
                std::size_t n = detail::utf8_encode(c, b);
                f.write_str(std::string_view(b, n));
            }
    }
    f.write_char('\'');
    return ok();
}

// ---------------------------------------------------------------------------
// Float formatting (Phase 3): shortest round-trip via freestanding Dragon4.
//
// `detail::dragon4::shortest_decimal` is the verified standalone core (Steele &
// White / Burger & Dubois over exact base-2^32 big integers), adapted to the
// no-std constraints: IEEE decode happens in the callers via std::bit_cast
// (<bit>), and the only big-int copy uses a manual limb loop — so no <cstring>.
// It was fuzzed to ~3.7M round-trip samples (0 failures) before integration.
// ---------------------------------------------------------------------------
namespace detail {
namespace dragon4 {

constexpr int BIGINT_LIMBS = 128;  // 128 * 32 = 4096 bits (worst case ~2500)

struct BigInt {
    std::uint32_t limb[BIGINT_LIMBS];  // little-endian base 2^32
    int length;                        // 0 means value == 0
};

inline void big_from_u64(BigInt* b, std::uint64_t v) {
    if (v == 0) { b->length = 0; return; }
    b->limb[0] = static_cast<std::uint32_t>(v & 0xFFFFFFFFu);
    std::uint32_t hi = static_cast<std::uint32_t>(v >> 32);
    if (hi) { b->limb[1] = hi; b->length = 2; } else { b->length = 1; }
}

inline void big_copy(BigInt* dst, const BigInt* src) {
    dst->length = src->length;
    for (int i = 0; i < src->length; ++i) dst->limb[i] = src->limb[i];
}

inline int big_cmp(const BigInt* a, const BigInt* b) {
    if (a->length != b->length) return a->length < b->length ? -1 : 1;
    for (int i = a->length - 1; i >= 0; --i)
        if (a->limb[i] != b->limb[i]) return a->limb[i] < b->limb[i] ? -1 : 1;
    return 0;
}

inline void big_normalize(BigInt* b) {
    while (b->length > 0 && b->limb[b->length - 1] == 0) b->length--;
}

// dst = a + b
inline void big_add(BigInt* dst, const BigInt* a, const BigInt* b) {
    const BigInt* lo = a; const BigInt* hi = b;
    if (lo->length > hi->length) { const BigInt* t = lo; lo = hi; hi = t; }
    std::uint64_t carry = 0; int i = 0;
    for (; i < lo->length; ++i) {
        std::uint64_t s = static_cast<std::uint64_t>(lo->limb[i]) + hi->limb[i] + carry;
        dst->limb[i] = static_cast<std::uint32_t>(s & 0xFFFFFFFFu); carry = s >> 32;
    }
    for (; i < hi->length; ++i) {
        std::uint64_t s = static_cast<std::uint64_t>(hi->limb[i]) + carry;
        dst->limb[i] = static_cast<std::uint32_t>(s & 0xFFFFFFFFu); carry = s >> 32;
    }
    dst->length = hi->length;
    if (carry) { dst->limb[dst->length] = static_cast<std::uint32_t>(carry); dst->length++; }
}

// dst = a - b, requires a >= b
inline void big_sub(BigInt* dst, const BigInt* a, const BigInt* b) {
    std::int64_t borrow = 0; int i = 0;
    for (; i < b->length; ++i) {
        std::int64_t d = static_cast<std::int64_t>(a->limb[i]) - b->limb[i] - borrow;
        if (d < 0) { d += (static_cast<std::int64_t>(1) << 32); borrow = 1; } else borrow = 0;
        dst->limb[i] = static_cast<std::uint32_t>(d);
    }
    for (; i < a->length; ++i) {
        std::int64_t d = static_cast<std::int64_t>(a->limb[i]) - borrow;
        if (d < 0) { d += (static_cast<std::int64_t>(1) << 32); borrow = 1; } else borrow = 0;
        dst->limb[i] = static_cast<std::uint32_t>(d);
    }
    dst->length = a->length; big_normalize(dst);
}

inline void big_mul_small(BigInt* b, std::uint32_t m) {
    if (m == 0 || b->length == 0) { b->length = 0; return; }
    std::uint64_t carry = 0;
    for (int i = 0; i < b->length; ++i) {
        std::uint64_t p = static_cast<std::uint64_t>(b->limb[i]) * m + carry;
        b->limb[i] = static_cast<std::uint32_t>(p & 0xFFFFFFFFu); carry = p >> 32;
    }
    if (carry) { b->limb[b->length] = static_cast<std::uint32_t>(carry); b->length++; }
}

// dst = src << bits ; dst must not alias src
inline void big_shl_to(BigInt* dst, const BigInt* src, int bits) {
    if (src->length == 0) { dst->length = 0; return; }
    int limb_shift = bits / 32, bit_shift = bits % 32;
    for (int i = 0; i < BIGINT_LIMBS; ++i) dst->limb[i] = 0;
    if (bit_shift == 0) {
        for (int i = 0; i < src->length; ++i) dst->limb[i + limb_shift] = src->limb[i];
        dst->length = src->length + limb_shift;
    } else {
        std::uint32_t carry = 0;
        for (int i = 0; i < src->length; ++i) {
            std::uint64_t v = (static_cast<std::uint64_t>(src->limb[i]) << bit_shift) | carry;
            dst->limb[i + limb_shift] = static_cast<std::uint32_t>(v & 0xFFFFFFFFu);
            carry = static_cast<std::uint32_t>(v >> 32);
        }
        dst->length = src->length + limb_shift;
        if (carry) { dst->limb[dst->length] = carry; dst->length++; }
    }
    big_normalize(dst);
}

inline void big_shl(BigInt* b, int bits) {
    if (b->length == 0 || bits == 0) return;
    BigInt tmp; big_shl_to(&tmp, b, bits); big_copy(b, &tmp);
}

inline void big_mul_pow5(BigInt* b, int n) {
    constexpr std::uint32_t pow5_chunk = 1220703125u;  // 5^13
    while (n >= 13) { big_mul_small(b, pow5_chunk); n -= 13; }
    if (n > 0) { std::uint32_t p = 1; for (int i = 0; i < n; ++i) p *= 5u; big_mul_small(b, p); }
}

inline void big_mul_pow10(BigInt* b, int n) {
    big_mul_pow5(b, n);
    BigInt tmp; big_shl_to(&tmp, b, n); big_copy(b, &tmp);
}

// returns floor(num/den), num := num % den (Dragon4 quotient digit is 0..9)
inline std::uint32_t big_divmod(BigInt* num, const BigInt* den) {
    std::uint32_t q = 0;
    while (big_cmp(num, den) >= 0) { big_sub(num, num, den); q++; }
    return q;
}

// --- Extra ops used by fixed-precision (`{:.N}`) digit generation. ----------

// b /= d (schoolbook), returns the remainder. O(limbs), not O(quotient).
inline std::uint32_t big_divmod_u32(BigInt* b, std::uint32_t d) {
    std::uint64_t rem = 0;
    for (int i = b->length - 1; i >= 0; --i) {
        std::uint64_t cur = (rem << 32) | b->limb[i];
        b->limb[i] = static_cast<std::uint32_t>(cur / d);
        rem = cur % d;
    }
    big_normalize(b);
    return static_cast<std::uint32_t>(rem);
}

// Decimal digits of `b`, most-significant first, into `out`; returns the count
// ("0" for zero). Destroys `b`.
inline int big_to_decimal(BigInt* b, char* out) {
    if (b->length == 0) { out[0] = '0'; return 1; }
    int n = 0;
    while (b->length != 0) out[n++] = static_cast<char>('0' + big_divmod_u32(b, 10));
    for (int i = 0, j = n - 1; i < j; ++i, --j) { char t = out[i]; out[i] = out[j]; out[j] = t; }
    return n;
}

inline bool big_test_bit(const BigInt* b, int bit) {
    int limb = bit / 32, off = bit % 32;
    if (limb >= b->length) return false;
    return (b->limb[limb] >> off) & 1u;
}

// Any set bit strictly below position `bit` (the "sticky" bits when rounding).
inline bool big_any_bit_below(const BigInt* b, int bit) {
    int full = bit / 32, off = bit % 32;
    for (int i = 0; i < full && i < b->length; ++i) if (b->limb[i]) return true;
    if (off && full < b->length && (b->limb[full] & ((1u << off) - 1))) return true;
    return false;
}

// dst = src >> bits (floor); dst must not alias src.
inline void big_shr(BigInt* dst, const BigInt* src, int bits) {
    int limb_shift = bits / 32, bit_shift = bits % 32;
    for (int i = 0; i < BIGINT_LIMBS; ++i) dst->limb[i] = 0;
    if (limb_shift >= src->length) { dst->length = 0; return; }
    int newlen = src->length - limb_shift;
    if (bit_shift == 0) {
        for (int i = 0; i < newlen; ++i) dst->limb[i] = src->limb[i + limb_shift];
    } else {
        for (int i = 0; i < newlen; ++i) {
            std::uint64_t lo = static_cast<std::uint64_t>(src->limb[i + limb_shift]) >> bit_shift;
            std::uint64_t hi = (i + limb_shift + 1 < src->length)
                ? (static_cast<std::uint64_t>(src->limb[i + limb_shift + 1]) << (32 - bit_shift))
                : 0;
            dst->limb[i] = static_cast<std::uint32_t>((lo | hi) & 0xFFFFFFFFu);
        }
    }
    dst->length = newlen; big_normalize(dst);
}

inline void big_inc(BigInt* b) {
    std::uint64_t carry = 1;
    for (int i = 0; i < b->length && carry; ++i) {
        std::uint64_t s = static_cast<std::uint64_t>(b->limb[i]) + carry;
        b->limb[i] = static_cast<std::uint32_t>(s & 0xFFFFFFFFu); carry = s >> 32;
    }
    if (carry) { b->limb[b->length] = static_cast<std::uint32_t>(carry); b->length++; }
}

// dst = round(src / 2^shift), round-half-to-even (shift >= 1).
inline void big_shr_round_to_even(BigInt* dst, const BigInt* src, int shift) {
    big_shr(dst, src, shift);                               // floor(src / 2^shift)
    if (big_test_bit(src, shift - 1)) {                     // halfway bit set?
        bool sticky = big_any_bit_below(src, shift - 1);    // > half  -> round up
        bool q_odd = (dst->length > 0) && (dst->limb[0] & 1u);
        if (sticky || q_odd) big_inc(dst);                  // == half -> to even
    }
}

// Round positive (mantissa * 2^exponent) to exactly `prec` fractional decimal
// digits, round-half-to-even, and write the scaled integer round(value*10^prec)
// as MSD-first decimal digits into `out` (caller places the point `prec` from
// the right). Returns the digit count. Only a power-of-two division is needed
// (prec >= 0 sends 5^prec to the numerator), so no bignum/bignum divide.
inline int fixed_positional(std::uint64_t mantissa, int exponent, int prec, char* out) {
    BigInt N; big_from_u64(&N, mantissa);
    big_mul_pow5(&N, prec);                // N = mantissa * 5^prec
    int p2 = exponent + prec;              // residual power of two
    BigInt M;
    if (p2 >= 0) big_shl_to(&M, &N, p2);                // exact integer
    else         big_shr_round_to_even(&M, &N, -p2);    // round(N / 2^(-p2))
    return big_to_decimal(&M, out);
}

// Shortest decimal for a positive finite value given as mantissa * 2^exponent.
// `pow2_boundary` is true iff the value sits on a power-of-two mantissa with an
// asymmetric lower neighbor (its lower gap is half the upper gap). Writes the
// significant digits (no sign/point/trailing zeros) into `digits` (>= 32 bytes),
// sets *E so value == 0.<digits> * 10^E, and returns the digit count n.
inline int shortest_decimal(std::uint64_t mantissa, int exponent,
                            bool pow2_boundary, char* digits, int* E_out) {
    // --- Exact rationals: value = R/S, half-boundary distances Mplus/Mminus ---
    BigInt R, S, Mplus, Mminus;
    if (pow2_boundary) {  // asymmetric: lower gap = half the upper gap
        std::uint64_t f2 = mantissa << 1;
        if (exponent >= 0) {
            big_from_u64(&R, f2); big_shl(&R, exponent); big_mul_small(&R, 2);
            big_from_u64(&S, 4);
            big_from_u64(&Mplus, 1);  big_shl(&Mplus, exponent); big_mul_small(&Mplus, 2);
            big_from_u64(&Mminus, 1); big_shl(&Mminus, exponent);
        } else {
            int negexp = -exponent;
            big_from_u64(&R, f2); big_mul_small(&R, 2);
            big_from_u64(&S, 1);  big_shl(&S, negexp); big_mul_small(&S, 4);
            big_from_u64(&Mplus, 2); big_from_u64(&Mminus, 1);
        }
    } else {  // symmetric boundaries
        if (exponent >= 0) {
            big_from_u64(&R, mantissa); big_shl(&R, exponent); big_mul_small(&R, 2);
            big_from_u64(&S, 2);
            big_from_u64(&Mplus, 1);  big_shl(&Mplus, exponent);
            big_from_u64(&Mminus, 1); big_shl(&Mminus, exponent);
        } else {
            int negexp = -exponent;
            big_from_u64(&R, mantissa); big_mul_small(&R, 2);
            big_from_u64(&S, 1); big_shl(&S, negexp); big_mul_small(&S, 2);
            big_from_u64(&Mplus, 1); big_from_u64(&Mminus, 1);
        }
    }

    // --- Estimate decimal exponent k, then fix up exactly ---
    int mant_bits = 0;
    { std::uint64_t t = mantissa; while (t) { mant_bits++; t >>= 1; } }
    int e2 = (mant_bits - 1) + exponent;                  // ~floor(log2(value))
    int k = static_cast<int>(static_cast<double>(e2) * 0.3010299956639812);  // *log10(2)

    if (k >= 0) big_mul_pow10(&S, k);
    else { big_mul_pow10(&R, -k); big_mul_pow10(&Mplus, -k); big_mul_pow10(&Mminus, -k); }

    BigInt RplusM;
    big_add(&RplusM, &R, &Mplus);
    while (big_cmp(&RplusM, &S) > 0) {                    // ensure value < 1 at this scale
        big_mul_small(&S, 10); k++; big_add(&RplusM, &R, &Mplus);
    }
    {                                                    // ensure first digit is nonzero
        BigInt tenRM; big_copy(&tenRM, &RplusM); big_mul_small(&tenRM, 10);
        while (big_cmp(&tenRM, &S) <= 0) {
            big_mul_small(&R, 10); big_mul_small(&Mplus, 10); big_mul_small(&Mminus, 10);
            k--; big_add(&RplusM, &R, &Mplus);
            big_copy(&tenRM, &RplusM); big_mul_small(&tenRM, 10);
        }
    }
    *E_out = k;  // value = 0.<digits> * 10^k

    // --- Digit generation (round-to-nearest-ties-to-even) ---
    int n = 0; bool low, high;
    for (;;) {
        big_mul_small(&R, 10); big_mul_small(&Mplus, 10); big_mul_small(&Mminus, 10);
        std::uint32_t d = big_divmod(&R, &S);             // d in [0,9]; R := remainder

        low = big_cmp(&R, &Mminus) < 0;
        { BigInt rp; big_add(&rp, &R, &Mplus); high = big_cmp(&rp, &S) > 0; }

        if (low || high) {
            if (low && !high) { /* round down */ }
            else if (high && !low) { d++; }               // round up
            else {                                        // tie: compare 2R vs S
                BigInt twoR; big_copy(&twoR, &R); big_mul_small(&twoR, 2);
                int c = big_cmp(&twoR, &S);
                if (c > 0) d++;                            // > 1/2 -> up
                else if (c < 0) { /* < 1/2 -> down */ }
                else { if (d & 1) d++; }                   // exactly 1/2 -> round to even
            }
            digits[n++] = static_cast<char>('0' + d);
            break;
        }
        digits[n++] = static_cast<char>('0' + d);
        if (n >= 30) break;                               // safety guard (never for f64)
    }
    return n;
}

}  // namespace dragon4
}  // namespace detail

// The four float-formatting "styles" the transpiler maps `{}`/`{:?}`/`{:e}`/
// `{:E}` to (Rust's Display / Debug / LowerExp / UpperExp).
enum class FloatStyle : std::uint8_t { Display, Debug, LowerExp, UpperExp };

// Decoded IEEE form shared by f32/f64 (mantissa * 2^exponent, plus the special
// flags and the asymmetric power-of-two boundary marker).
struct FloatParts {
    bool is_nan = false;
    bool is_inf = false;
    bool is_zero = false;
    bool nonneg = true;
    std::uint64_t mantissa = 0;
    int exponent = 0;
    bool pow2_boundary = false;
};

inline FloatParts decode_f64(double value) {
    std::uint64_t bits = std::bit_cast<std::uint64_t>(value);
    FloatParts p;
    p.nonneg = (bits >> 63) == 0;
    int raw_exp = static_cast<int>((bits >> 52) & 0x7FFu);
    std::uint64_t raw_mant = bits & ((static_cast<std::uint64_t>(1) << 52) - 1);
    if (raw_exp == 0x7FF) { if (raw_mant) p.is_nan = true; else p.is_inf = true; return p; }
    if (raw_exp == 0 && raw_mant == 0) { p.is_zero = true; return p; }
    if (raw_exp == 0) {                                 // subnormal
        p.mantissa = raw_mant; p.exponent = 1 - 1075;   // 2^-1074
    } else {                                            // normal
        p.mantissa = raw_mant | (static_cast<std::uint64_t>(1) << 52);
        p.exponent = raw_exp - 1075;
        p.pow2_boundary = (raw_mant == 0 && raw_exp > 1);
    }
    return p;
}

inline FloatParts decode_f32(float value) {
    std::uint32_t bits = std::bit_cast<std::uint32_t>(value);
    FloatParts p;
    p.nonneg = (bits >> 31) == 0;
    int raw_exp = static_cast<int>((bits >> 23) & 0xFFu);
    std::uint32_t raw_mant = bits & ((1u << 23) - 1);
    if (raw_exp == 0xFF) { if (raw_mant) p.is_nan = true; else p.is_inf = true; return p; }
    if (raw_exp == 0 && raw_mant == 0) { p.is_zero = true; return p; }
    if (raw_exp == 0) {                                 // subnormal
        p.mantissa = raw_mant; p.exponent = 1 - 150;    // 2^-149
    } else {                                            // normal
        p.mantissa = raw_mant | (1u << 23);
        p.exponent = raw_exp - 150;
        p.pow2_boundary = (raw_mant == 0 && raw_exp > 1);
    }
    return p;
}

// Render `parts` through `style`, applying sign / width / fill / zero-pad via
// `pad_integral` (Rust routes floats through the same numeric padding path).
inline Result format_float(Formatter& f, const FloatParts& parts, FloatStyle style) {
    // Non-finite. "inf" keeps the value's sign and honors `+` (`{:+}` -> "+inf").
    // "NaN" NEVER carries a sign — not even under `{:+}` (Rust emits a sign-less
    // NaN part), so format it with sign_plus masked off.
    if (parts.is_nan) {
        FormatSpec s = f.spec();
        s.sign_plus = false;
        Formatter nf(f.buffer(), s);
        return nf.pad_integral(true, std::string_view(), "NaN");
    }
    if (parts.is_inf) return f.pad_integral(parts.nonneg, std::string_view(), "inf");

    const FormatSpec& sp = f.spec();
    bool exp_style = (style == FloatStyle::LowerExp || style == FloatStyle::UpperExp);

    char mag[1400];
    std::size_t mlen = 0;
    auto put = [&](char c) { mag[mlen++] = c; };
    auto put_n = [&](char c, std::size_t k) { for (std::size_t i = 0; i < k; ++i) mag[mlen++] = c; };
    auto put_sv = [&](const char* p, std::size_t k) { for (std::size_t i = 0; i < k; ++i) mag[mlen++] = p[i]; };

    // Fixed precision `{:.N}` (positional). Display and Debug behave identically
    // here — exactly `precision` fractional digits, round-half-to-even, and NO
    // scientific switch (Rust routes both through float_to_decimal_common_exact).
    // The cap guards the fixed-width BigInt against absurd precisions.
    if (sp.has_precision && !exp_style && sp.precision <= 800) {
        int prec = static_cast<int>(sp.precision);
        char md[1300];
        int L = parts.is_zero
                    ? (md[0] = '0', 1)
                    : detail::dragon4::fixed_positional(parts.mantissa, parts.exponent, prec, md);
        if (prec == 0) {                                 // integer, no point
            put_sv(md, static_cast<std::size_t>(L));
        } else if (L <= prec) {                          // 0.<zeros><digits>
            put('0'); put('.');
            put_n('0', static_cast<std::size_t>(prec - L));
            put_sv(md, static_cast<std::size_t>(L));
        } else {                                         // <int>.<frac>
            put_sv(md, static_cast<std::size_t>(L - prec));
            put('.');
            put_sv(md + (L - prec), static_cast<std::size_t>(prec));
        }
        return f.pad_integral(parts.nonneg, std::string_view(), std::string_view(mag, mlen));
    }

    char digits[40];
    int n, E;
    if (parts.is_zero) {
        digits[0] = '0'; n = 1; E = 1;  // 0 == 0.0 * 10^1, no significant digits
    } else {
        n = detail::dragon4::shortest_decimal(parts.mantissa, parts.exponent,
                                              parts.pow2_boundary, digits, &E);
    }

    // Positional vs scientific. Display is always positional; the exp traits are
    // always scientific; Debug flips to scientific when the value's decimal
    // exponent (exp = E-1) is < -4 or >= 16 (Rust's float_to_general_debug).
    // (Scientific WITH precision, `{:.Ne}`, would need round-to-N+1-significant
    // digits — a bignum/bignum divide — and is not yet implemented; such inputs
    // fall through here and emit the shortest scientific form.)
    bool scientific;
    char exp_char = 'e';
    switch (style) {
        case FloatStyle::Display:  scientific = false; break;
        case FloatStyle::LowerExp: scientific = true; exp_char = 'e'; break;
        case FloatStyle::UpperExp: scientific = true; exp_char = 'E'; break;
        case FloatStyle::Debug: {
            int exp = E - 1;
            scientific = (exp < -4 || exp >= 16);
            break;
        }
    }
    // Debug positional always shows a fractional part (whole numbers get ".0").
    bool force_point_zero = (style == FloatStyle::Debug) && !scientific;

    if (scientific) {
        put(digits[0]);
        if (n > 1) { put('.'); put_sv(digits + 1, static_cast<std::size_t>(n - 1)); }
        put(exp_char);
        int exp = E - 1;
        if (exp < 0) { put('-'); exp = -exp; }
        char eb[12];
        std::size_t en = detail::to_radix_digits(static_cast<unsigned __int128>(exp), 10, false, eb);
        put_sv(eb, en);
    } else if (E <= 0) {                                 // 0.<zeros><digits>
        put('0'); put('.');
        put_n('0', static_cast<std::size_t>(-E));
        put_sv(digits, static_cast<std::size_t>(n));
    } else if (E >= n) {                                 // <digits><zeros>[.0]
        put_sv(digits, static_cast<std::size_t>(n));
        put_n('0', static_cast<std::size_t>(E - n));
        if (force_point_zero) { put('.'); put('0'); }
    } else {                                             // <digits[:E]>.<digits[E:]>
        put_sv(digits, static_cast<std::size_t>(E));
        put('.');
        put_sv(digits + E, static_cast<std::size_t>(n - E));
    }

    return f.pad_integral(parts.nonneg, std::string_view(), std::string_view(mag, mlen));
}

inline Result fmt_f64(Formatter& f, double value, FloatStyle style = FloatStyle::Display) {
    return format_float(f, decode_f64(value), style);
}
inline Result fmt_f32(Formatter& f, float value, FloatStyle style = FloatStyle::Display) {
    return format_float(f, decode_f32(value), style);
}

// ---------------------------------------------------------------------------
// Debug dispatch + builders (Phase 2).
// ---------------------------------------------------------------------------

// Format a value's DEBUG representation. Prefers the value's own
// `fmt(Formatter&)` (the transpiled Debug impl); otherwise routes primitives to
// their Debug forms (int -> decimal, bool, &str -> quoted, char -> 'c').
template<typename T>
inline Result debug_value(Formatter& f, const T& value) {
    using U = std::remove_cvref_t<T>;
    if constexpr (requires { value.fmt(f); }) {
        return value.fmt(f);
    } else if constexpr (std::is_same_v<U, bool>) {
        return fmt_bool(f, value);
    } else if constexpr (std::is_same_v<U, char32_t>) {
        return fmt_char_debug(f, value);
    } else if constexpr (std::is_integral_v<U>) {
        return fmt_int(f, value);
    } else if constexpr (std::is_floating_point_v<U>) {
        if constexpr (std::is_same_v<U, float>) return fmt_f32(f, value, FloatStyle::Debug);
        else return fmt_f64(f, static_cast<double>(value), FloatStyle::Debug);
    } else if constexpr (std::is_convertible_v<const U&, std::string_view>) {
        return fmt_str_debug(f, std::string_view(value));
    } else {
        return ok();
    }
}

// Write `content`, prefixing `indent` after every newline (Rust's PadAdapter):
// this is what nests pretty-printed (`{:#?}`) Debug output one level deeper.
inline void write_indented(Formatter& f, std::string_view content,
                           std::string_view indent) {
    bool on_newline = false;
    for (char c : content) {
        if (on_newline) {
            f.write_str(indent);
        }
        f.write_char(c);
        on_newline = (c == '\n');
    }
}

// Format a value's Debug into a fresh buffer (alternate-aware) so it can be
// re-emitted with indentation. Returns the owning Buffer (move).
template<typename T>
inline Buffer debug_to_buffer(bool alternate, const T& value) {
    Buffer tmp;
    FormatSpec spec;
    spec.alternate = alternate;
    Formatter sub(tmp, spec);
    debug_value(sub, value);
    return tmp;
}

inline constexpr std::string_view kIndent = "    ";

class DebugStruct {
    Formatter& f_;
    bool has_fields_ = false;

public:
    DebugStruct(Formatter& f, std::string_view name) : f_(f) { f_.write_str(name); }

    template<typename T>
    DebugStruct& field(std::string_view name, const T& value) {
        if (f_.alternate()) {
            if (!has_fields_) {
                f_.write_str(" {\n");
            }
            Buffer vb = debug_to_buffer(true, value);
            f_.write_str(kIndent);
            f_.write_str(name);
            f_.write_str(": ");
            write_indented(f_, vb.view(), kIndent);
            f_.write_str(",\n");
        } else {
            f_.write_str(has_fields_ ? ", " : " { ");
            f_.write_str(name);
            f_.write_str(": ");
            debug_value(f_, value);
        }
        has_fields_ = true;
        return *this;
    }

    Result finish() {
        if (has_fields_) {
            f_.write_str(f_.alternate() ? std::string_view("}") : std::string_view(" }"));
        }
        return ok();
    }

    Result finish_non_exhaustive() {
        if (f_.alternate()) {
            f_.write_str(has_fields_ ? std::string_view("") : std::string_view(" {\n"));
            f_.write_str(kIndent);
            f_.write_str("..\n}");
        } else {
            f_.write_str(has_fields_ ? std::string_view(", .. }") : std::string_view(" { .. }"));
        }
        return ok();
    }
};

// Shared body for the "sequence" builders (tuple / list / set), parameterized by
// the bracket pair and (for tuple) a name prefix.
class SeqBuilder {
    Formatter& f_;
    char open_;
    char close_;
    bool has_entries_ = false;

public:
    SeqBuilder(Formatter& f, std::string_view name_prefix, char open, char close)
        : f_(f), open_(open), close_(close) {
        f_.write_str(name_prefix);
    }

    template<typename T>
    SeqBuilder& entry(const T& value) {
        if (f_.alternate()) {
            if (!has_entries_) {
                f_.write_char(open_);
                f_.write_char('\n');
            }
            Buffer vb = debug_to_buffer(true, value);
            f_.write_str(kIndent);
            write_indented(f_, vb.view(), kIndent);
            f_.write_str(",\n");
        } else {
            if (!has_entries_) {
                f_.write_char(open_);
            } else {
                f_.write_str(", ");
            }
            debug_value(f_, value);
        }
        has_entries_ = true;
        return *this;
    }

    Result finish() {
        if (!has_entries_) {
            f_.write_char(open_);
        }
        f_.write_char(close_);
        return ok();
    }
};

// Thin typed wrappers so the transpiler's debug_tuple()/list()/set() return
// distinct types (matching Rust's API) over the shared sequence logic.
class DebugTuple : public SeqBuilder {
public:
    DebugTuple(Formatter& f, std::string_view name) : SeqBuilder(f, name, '(', ')') {}
    template<typename T> DebugTuple& field(const T& v) { entry(v); return *this; }
};
class DebugList : public SeqBuilder {
public:
    explicit DebugList(Formatter& f) : SeqBuilder(f, std::string_view(), '[', ']') {}
    template<typename It> DebugList& entries(It begin, It end) {
        for (; begin != end; ++begin) entry(*begin);
        return *this;
    }
};
class DebugSet : public SeqBuilder {
public:
    explicit DebugSet(Formatter& f) : SeqBuilder(f, std::string_view(), '{', '}') {}
    template<typename It> DebugSet& entries(It begin, It end) {
        for (; begin != end; ++begin) entry(*begin);
        return *this;
    }
};

class DebugMap {
    Formatter& f_;
    bool has_entries_ = false;

public:
    explicit DebugMap(Formatter& f) : f_(f) {}

    template<typename K, typename V>
    DebugMap& entry(const K& key, const V& value) {
        if (f_.alternate()) {
            if (!has_entries_) {
                f_.write_str("{\n");
            }
            Buffer kb = debug_to_buffer(true, key);
            Buffer vb = debug_to_buffer(true, value);
            f_.write_str(kIndent);
            write_indented(f_, kb.view(), kIndent);
            f_.write_str(": ");
            write_indented(f_, vb.view(), kIndent);
            f_.write_str(",\n");
        } else {
            f_.write_str(has_entries_ ? ", " : "{");
            debug_value(f_, key);
            f_.write_str(": ");
            debug_value(f_, value);
        }
        has_entries_ = true;
        return *this;
    }

    Result finish() {
        if (!has_entries_) {
            f_.write_char('{');
        }
        f_.write_char('}');
        return ok();
    }
};

inline DebugStruct Formatter::debug_struct(std::string_view name) {
    return DebugStruct(*this, name);
}
inline DebugTuple Formatter::debug_tuple(std::string_view name) {
    return DebugTuple(*this, name);
}
inline DebugList Formatter::debug_list() { return DebugList(*this); }
inline DebugSet Formatter::debug_set() { return DebugSet(*this); }
inline DebugMap Formatter::debug_map() { return DebugMap(*this); }

} // namespace rt
} // namespace fmt
} // namespace rusty

#endif // RUSTY_FMT_RT_HPP
