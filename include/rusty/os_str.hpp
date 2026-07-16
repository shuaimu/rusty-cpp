#pragma once
// rusty::ffi::OsStr / OsString + rusty::sys::path — Unix-only runtime backing the
// std::path port. On Unix an OsStr is raw platform bytes (no WTF-8), so
// OsString ~ Vec<u8> and OsStr ~ &[u8]. std::path only ever splits its inner
// bytes at ASCII '/' and '.', so from_encoded_bytes_unchecked is a plain byte
// reinterpret and every split lands on a valid boundary.
//
// This header is included ONLY by the std::path port (docs/path/build.sh injects
// it and seds the emitted `using OsStr = std::string` aliases out of the .cppm);
// nothing else in the tree pulls it, so existing targets are unaffected.
#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <span>
#include <string>
#include <string_view>
#include <tuple>
#include <vector>

#include "option.hpp"

namespace rusty {
namespace ffi {

// The &[u8] view returned by OsStr/OsString::as_encoded_bytes(). Provides the
// byte-slice API std::path's parsing uses (position/rposition/last/split_last/
// rsplitn/iter/==). Non-owning: valid while the source OsStr/OsString lives.
struct OsBytes {
    const std::uint8_t* ptr_ = nullptr;
    std::size_t len_ = 0;

    std::size_t len() const { return len_; }
    bool is_empty() const { return len_ == 0; }
    const std::uint8_t* as_ptr() const { return ptr_; }
    std::uint8_t operator[](std::size_t i) const { return ptr_[i]; }

    OsBytes slice(std::size_t start, std::size_t end) const {
        return OsBytes{ptr_ + start, end - start};
    }
    OsBytes slice_from(std::size_t start) const {
        return OsBytes{ptr_ + start, len_ - start};
    }
    OsBytes slice_to(std::size_t end) const { return OsBytes{ptr_, end}; }

    rusty::Option<std::uint8_t> last() const {
        if (len_ == 0) return rusty::None;
        return rusty::Option<std::uint8_t>(ptr_[len_ - 1]);
    }
    rusty::Option<std::uint8_t> first() const {
        if (len_ == 0) return rusty::None;
        return rusty::Option<std::uint8_t>(ptr_[0]);
    }

    template <typename Pred>
    rusty::Option<std::size_t> position(Pred&& pred) const {
        for (std::size_t i = 0; i < len_; ++i) {
            if (pred(ptr_[i])) return rusty::Option<std::size_t>(i);
        }
        return rusty::None;
    }
    template <typename Pred>
    rusty::Option<std::size_t> rposition(Pred&& pred) const {
        for (std::size_t i = len_; i-- > 0;) {
            if (pred(ptr_[i])) return rusty::Option<std::size_t>(i);
        }
        return rusty::None;
    }

    // split_last() -> Option<(&u8, &[u8])>
    rusty::Option<std::tuple<std::uint8_t, OsBytes>> split_last() const {
        if (len_ == 0) return rusty::None;
        return rusty::Option<std::tuple<std::uint8_t, OsBytes>>(
            std::make_tuple(ptr_[len_ - 1], OsBytes{ptr_, len_ - 1}));
    }

    // rsplitn(n, pred): yields at most n sub-slices, splitting from the end.
    template <typename Pred>
    struct RSplitN {
        OsBytes rem;
        std::size_t n;
        Pred pred;
        bool done = false;
        RSplitN into_iter() const { return *this; }
        rusty::Option<OsBytes> next() {
            if (done) return rusty::None;
            if (n <= 1) {
                done = true;
                return rusty::Option<OsBytes>(rem);
            }
            for (std::size_t i = rem.len_; i-- > 0;) {
                if (pred(rem.ptr_[i])) {
                    OsBytes piece{rem.ptr_ + i + 1, rem.len_ - (i + 1)};
                    rem = OsBytes{rem.ptr_, i};
                    --n;
                    return rusty::Option<OsBytes>(piece);
                }
            }
            done = true;
            return rusty::Option<OsBytes>(rem);
        }
    };
    template <typename Pred>
    RSplitN<std::decay_t<Pred>> rsplitn(std::size_t n, Pred&& pred) const {
        return RSplitN<std::decay_t<Pred>>{*this, n, std::forward<Pred>(pred)};
    }

    // iter(): yields each byte (Rust yields &u8; we yield the value).
    struct Iter {
        const std::uint8_t* cur;
        const std::uint8_t* end;
        Iter into_iter() const { return *this; }
        rusty::Option<std::uint8_t> next() {
            if (cur == end) return rusty::None;
            return rusty::Option<std::uint8_t>(*cur++);
        }
    };
    Iter iter() const { return Iter{ptr_, ptr_ + len_}; }

    std::string_view as_str_view() const {
        return std::string_view(reinterpret_cast<const char*>(ptr_), len_);
    }
    // Rust `&[u8]` also flows as std::span at some seams.
    operator std::span<const std::uint8_t>() const {
        return std::span<const std::uint8_t>(ptr_, len_);
    }

    bool operator==(const OsBytes& o) const {
        return len_ == o.len_ && std::equal(ptr_, ptr_ + len_, o.ptr_);
    }
    template <std::size_t N>
    bool operator==(const std::array<std::uint8_t, N>& a) const {
        return len_ == N && std::equal(ptr_, ptr_ + len_, a.data());
    }
};

// Owned byte buffer. On the value-semantics port both OsStr and OsString own
// their bytes (Path/PathBuf hold them by value); OsStr is the read-only face,
// OsString adds mutation. They interconvert freely.
struct OsString;

struct OsStr {
    std::vector<std::uint8_t> bytes_;

    OsStr() = default;
    explicit OsStr(std::vector<std::uint8_t> b) : bytes_(std::move(b)) {}

    static OsStr from_encoded_bytes_unchecked(OsBytes b) {
        return OsStr(std::vector<std::uint8_t>(b.ptr_, b.ptr_ + b.len_));
    }
    static OsStr from_encoded_bytes_unchecked(std::vector<std::uint8_t> b) {
        return OsStr(std::move(b));
    }
    // Rust `&[u8]` byte slices lower to std::span, so byte sub-slices (from
    // rsplitn/split) arrive as spans, not OsBytes.
    static OsStr from_encoded_bytes_unchecked(std::span<const std::uint8_t> b) {
        return OsStr(std::vector<std::uint8_t>(b.begin(), b.end()));
    }
    static OsStr from_encoded_bytes_unchecked(std::string_view s) {
        return OsStr(std::vector<std::uint8_t>(s.begin(), s.end()));
    }

    static const OsStr& new_(std::string_view s);  // for OsStr::new("literal")

    OsBytes as_encoded_bytes() const { return OsBytes{bytes_.data(), bytes_.size()}; }
    std::size_t len() const { return bytes_.size(); }
    bool is_empty() const { return bytes_.empty(); }

    std::string_view as_str_view() const {
        return std::string_view(reinterpret_cast<const char*>(bytes_.data()), bytes_.size());
    }
    rusty::Option<std::string_view> to_str() const {
        return rusty::Option<std::string_view>(as_str_view());
    }
    std::string to_string_lossy() const { return std::string(as_str_view()); }
    std::string_view display() const { return as_str_view(); }

    OsString to_os_string() const;

    // AsRef<OsStr>/AsRef<Path>: Path is repr(transparent) over OsStr, so a
    // reference to the bytes serves both. Callers that want a Path reinterpret
    // this OsStr& (Path is implicitly constructible from OsStr — see the port's
    // post_transpile_patch).
    const OsStr& as_ref() const { return *this; }

    // Clone helpers (Box<OsStr> / spec-clone internals).
    void clone_into(OsString& target) const;
    template <typename Dst>
    void clone_to_uninit(Dst dst) const {
        *dst = OsStr(bytes_);
    }
    const std::uint8_t* into_raw() const { return bytes_.data(); }

    bool operator==(const OsStr& o) const { return bytes_ == o.bytes_; }
};

struct OsString {
    std::vector<std::uint8_t> bytes_;

    OsString() = default;
    explicit OsString(std::vector<std::uint8_t> b) : bytes_(std::move(b)) {}

    static OsString new_() { return OsString{}; }
    static OsString with_capacity(std::size_t cap) {
        OsString s;
        s.bytes_.reserve(cap);
        return s;
    }
    static OsString from(std::string_view s) {
        return OsString(std::vector<std::uint8_t>(s.begin(), s.end()));
    }
    static OsString from(const OsStr& s) { return OsString(s.bytes_); }

    OsBytes as_encoded_bytes() const { return OsBytes{bytes_.data(), bytes_.size()}; }
    const OsStr& as_os_str() const {
        // OsStr and OsString share layout (a byte vector); expose in place.
        return *reinterpret_cast<const OsStr*>(this);
    }
    std::size_t len() const { return bytes_.size(); }
    bool is_empty() const { return bytes_.empty(); }
    std::size_t capacity() const { return bytes_.capacity(); }

    void push_bytes(const std::uint8_t* p, std::size_t n) { bytes_.insert(bytes_.end(), p, p + n); }
    void push(const OsStr& s) { push_bytes(s.bytes_.data(), s.bytes_.size()); }
    void push(const OsString& s) { push_bytes(s.bytes_.data(), s.bytes_.size()); }
    void push(OsBytes b) { push_bytes(b.ptr_, b.len_); }
    void push(std::string_view s) {
        push_bytes(reinterpret_cast<const std::uint8_t*>(s.data()), s.size());
    }
    void push(const char* s) { push(std::string_view(s)); }
    void push(char c) { bytes_.push_back(static_cast<std::uint8_t>(c)); }

    void truncate(std::size_t new_len) {
        if (new_len < bytes_.size()) bytes_.resize(new_len);
    }
    void clear() { bytes_.clear(); }
    void reserve(std::size_t additional) { bytes_.reserve(bytes_.size() + additional); }
    void reserve_exact(std::size_t additional) { bytes_.reserve(bytes_.size() + additional); }
    void shrink_to(std::size_t min_capacity) {
        if (bytes_.capacity() > min_capacity && bytes_.capacity() > bytes_.size()) {
            bytes_.shrink_to_fit();
        }
    }
    void shrink_to_fit() { bytes_.shrink_to_fit(); }
    void extend_from_slice_unchecked(OsBytes b) { push_bytes(b.ptr_, b.len_); }
    void extend_from_slice_unchecked(const OsStr& s) { push(s); }

    // leak() -> &'static mut OsStr: intentionally leaks the buffer.
    OsStr& leak() { return *new OsStr(std::move(bytes_)); }
    OsStr into_boxed_os_str() { return OsStr(std::move(bytes_)); }

    OsString clone() const { return OsString(bytes_); }
    void clone_from(const OsString& other) { bytes_ = other.bytes_; }
    const OsStr& as_ref() const { return as_os_str(); }
    std::string_view as_str_view() const {
        return std::string_view(reinterpret_cast<const char*>(bytes_.data()), bytes_.size());
    }
    rusty::Option<std::string_view> to_str() const {
        return rusty::Option<std::string_view>(as_str_view());
    }
};

inline OsString OsStr::to_os_string() const { return OsString(bytes_); }
inline void OsStr::clone_into(OsString& target) const { target.bytes_ = bytes_; }

// OsStr::new("literal") — returns a stable reference to a byte buffer.
inline const OsStr& OsStr::new_(std::string_view s) {
    thread_local OsStr tmp;
    tmp = OsStr(std::vector<std::uint8_t>(s.begin(), s.end()));
    return tmp;
}

}  // namespace ffi

namespace sys {
namespace path {

// Unix platform behavior (library/std/src/sys/path/unix.rs).
inline constexpr bool HAS_PREFIXES = false;
inline constexpr char32_t MAIN_SEP = U'/';
inline constexpr std::string_view MAIN_SEP_STR = "/";

inline bool is_sep_byte(std::uint8_t b) { return b == static_cast<std::uint8_t>('/'); }
inline bool is_verbatim_sep(std::uint8_t b) { return b == static_cast<std::uint8_t>('/'); }

// parse_prefix always None on Unix — the Prefix machinery is prep-stripped, so
// this returns a plain optional<none> the caller compares against.
template <typename Prefix>
inline rusty::Option<Prefix> parse_prefix(const rusty::ffi::OsStr&) {
    return rusty::None;
}

}  // namespace path
}  // namespace sys
}  // namespace rusty
