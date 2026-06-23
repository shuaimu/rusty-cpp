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
