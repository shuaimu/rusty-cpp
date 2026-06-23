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
#include "rusty/alloc.hpp"
#include "rusty/fmt.hpp"   // rusty::fmt::Error / Result / trait stubs

namespace rusty {
namespace fmt {
namespace rt {

using rusty::fmt::Error;
using rusty::fmt::Result;

inline Result ok() { return Result::Ok(std::make_tuple()); }

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
};

} // namespace rt
} // namespace fmt
} // namespace rusty

#endif // RUSTY_FMT_RT_HPP
