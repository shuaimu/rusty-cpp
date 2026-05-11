#ifndef RUSTY_IO_HPP
#define RUSTY_IO_HPP

// rusty::io — Rust-like I/O types for C++
//
// Provides equivalents of Rust's std::io module:
//   - io::Error          — I/O error type
//   - io::Result<T>      — Result<T, io::Error>
//   - io::Read           — trait for reading bytes
//   - io::Write          — trait for writing bytes
//   - io::Seek           — trait for seeking
//   - io::BufRead        — trait for buffered reading
//   - io::SeekFrom       — seek position enum
//   - io::Cursor<T>      — in-memory cursor over a buffer
//   - io::stdin/stdout/stderr — standard stream handles

#include <cstdint>
#include <cstddef>
#include <cstring>
#include <string>
#include <string_view>
#include <vector>
#include <algorithm>
#include <stdexcept>
#include <iostream>
#include <sstream>
#include <span>
#include <type_traits>
#include <tuple>
#include <utility>
#include <iterator>
#include "rusty/result.hpp"

namespace rusty {
namespace io {

struct Read {};
struct Write {};

// ── Error ──────────────────────────────────────────────

class Error {
public:
    enum class Kind {
        NotFound,
        PermissionDenied,
        ConnectionRefused,
        ConnectionReset,
        ConnectionAborted,
        NotConnected,
        AddrInUse,
        AddrNotAvailable,
        BrokenPipe,
        AlreadyExists,
        WouldBlock,
        InvalidInput,
        InvalidData,
        TimedOut,
        WriteZero,
        Interrupted,
        UnexpectedEof,
        Unsupported,
        OutOfMemory,
        Other,
    };

    Error() : kind_(Kind::Other), message_("") {}

    Error(Kind kind, const std::string& message)
        : kind_(kind), message_(message) {}

    explicit Error(const std::string& message)
        : kind_(Kind::Other), message_(message) {}

    Kind kind() const { return kind_; }
    const std::string& to_string() const { return message_; }
    const char* what() const { return message_.c_str(); }
    Option<const void*&> source() const { return Option<const void*&>{None}; }

private:
    Kind kind_;
    std::string message_;
};

using ErrorKind = Error::Kind;

// ── Result<T> ──────────────────────────────────────────

template<typename T>
class Result {
public:
    static Result ok(T value) { return Result(std::move(value), true); }
    static Result err(Error error) { return Result(std::move(error)); }
    static Result Ok(T value) { return ok(std::move(value)); }
    static Result Err(Error error) { return err(std::move(error)); }

    template<typename U>
    Result(rusty::ok_contextual_value<U> ok) : Result(static_cast<T>(std::move(ok.value)), true) {}

    template<typename U>
    Result(rusty::err_contextual_value<U> err) : Result(Error(std::move(err.error))) {}

    bool is_ok() const { return ok_; }
    bool is_err() const { return !ok_; }

    T& unwrap() {
        if (!ok_) throw std::runtime_error("io::Result::unwrap on Err: " + error_.to_string());
        return value_;
    }

    const T& unwrap() const {
        if (!ok_) throw std::runtime_error("io::Result::unwrap on Err: " + error_.to_string());
        return value_;
    }

    Error& unwrap_err() {
        if (ok_) throw std::runtime_error("io::Result::unwrap_err on Ok");
        return error_;
    }

    bool operator==(const Result& other) const {
        if (ok_ != other.ok_) return false;
        if (ok_) return value_ == other.value_;
        return true;
    }

    template<typename F>
    auto map(F f) -> rusty::Result<decltype(f(std::declval<T>())), Error> {
        using NewT = decltype(f(std::declval<T>()));
        if (ok_) {
            return rusty::Result<NewT, Error>::Ok(f(std::move(value_)));
        }
        return rusty::Result<NewT, Error>::Err(std::move(error_));
    }

    template<typename F>
    auto map_err(F f) -> rusty::Result<T, decltype(f(std::declval<Error>()))> {
        using NewE = decltype(f(std::declval<Error>()));
        if (ok_) {
            return rusty::Result<T, NewE>::Ok(std::move(value_));
        }
        return rusty::Result<T, NewE>::Err(f(std::move(error_)));
    }

private:
    Result(T value, bool) : value_(std::move(value)), error_(""), ok_(true) {}
    Result(Error error) : value_{}, error_(std::move(error)), ok_(false) {}

    T value_;
    Error error_;
    bool ok_;
};

// Specialization for void result
template<>
class Result<void> {
public:
    static Result ok() { return Result(true); }
    static Result err(Error error) { return Result(std::move(error)); }
    static Result Ok() { return ok(); }
    static Result Err(Error error) { return err(std::move(error)); }

    template<typename U>
    Result(rusty::ok_contextual_value<U>) : Result(true) {}

    template<typename U>
    Result(rusty::err_contextual_value<U> err) : Result(Error(std::move(err.error))) {}

    bool is_ok() const { return ok_; }
    bool is_err() const { return !ok_; }

    void unwrap() const {
        if (!ok_) throw std::runtime_error("io::Result::unwrap on Err: " + error_.to_string());
    }

    Error& unwrap_err() {
        if (ok_) throw std::runtime_error("io::Result::unwrap_err on Ok");
        return error_;
    }

    template<typename F>
    auto map(F f) -> rusty::Result<decltype(f()), Error> {
        using NewT = decltype(f());
        if (ok_) {
            return rusty::Result<NewT, Error>::Ok(f());
        }
        return rusty::Result<NewT, Error>::Err(std::move(error_));
    }

    template<typename F>
    auto map_err(F f) -> rusty::Result<void, decltype(f(std::declval<Error>()))> {
        using NewE = decltype(f(std::declval<Error>()));
        if (ok_) {
            return rusty::Result<void, NewE>::Ok();
        }
        return rusty::Result<void, NewE>::Err(f(std::move(error_)));
    }

private:
    Result(bool) : error_(""), ok_(true) {}
    Result(Error error) : error_(std::move(error)), ok_(false) {}

    Error error_;
    bool ok_;
};

template<typename R>
class Bytes {
public:
    using Item = Result<uint8_t>;

    explicit Bytes(R& reader) : reader_(&reader) {}

    Option<Item> next() {
        uint8_t byte = 0;
        std::span<uint8_t> buf(&byte, 1);
        auto read_result = reader_->read(buf);
        if (read_result.is_err()) {
            return Option<Item>(Item::err(read_result.unwrap_err()));
        }
        if (read_result.unwrap() == 0) {
            return None;
        }
        return Option<Item>(Item::ok(byte));
    }

private:
    R* reader_;
};

template<typename R>
Bytes<std::remove_reference_t<R>> bytes(R& reader) {
    return Bytes<std::remove_reference_t<R>>(reader);
}

// ── SeekFrom ───────────────────────────────────────────

class SeekFrom {
public:
    enum Tag { StartTag, EndTag, CurrentTag };

    static SeekFrom Start(uint64_t pos) { return SeekFrom(StartTag, static_cast<int64_t>(pos)); }
    static SeekFrom End(int64_t offset) { return SeekFrom(EndTag, offset); }
    static SeekFrom Current(int64_t offset) { return SeekFrom(CurrentTag, offset); }

    Tag tag() const { return tag_; }
    int64_t offset() const { return offset_; }

private:
    SeekFrom(Tag tag, int64_t offset) : tag_(tag), offset_(offset) {}
    Tag tag_;
    int64_t offset_;
};

// ── Cursor<T> ──────────────────────────────────────────
// In-memory cursor over a byte buffer. Implements Read, Write, Seek.
// T must be a contiguous byte container (vector<uint8_t>, span<uint8_t>, etc.)

template<typename T>
class Cursor {
public:
    explicit Cursor(T inner) : inner_(std::move(inner)), pos_(0) {}

    static Cursor new_(T inner) { return Cursor(std::move(inner)); }

    // Read: copy bytes from cursor position into buf
    Result<size_t> read(std::span<uint8_t> buf) {
        const uint8_t* data = get_data();
        size_t len = get_len();

        if (pos_ >= len) return Result<size_t>::ok(0);

        size_t available = len - pos_;
        size_t to_read = std::min(buf.size(), available);
        std::memcpy(buf.data(), data + pos_, to_read);
        pos_ += to_read;
        return Result<size_t>::ok(to_read);
    }

    // Write: copy bytes from buf into cursor (for mutable buffers)
    Result<size_t> write(std::span<const uint8_t> buf) {
        uint8_t* data = get_mut_data();
        size_t len = get_len();

        if (pos_ >= len) return Result<size_t>::ok(0);

        size_t available = len - pos_;
        size_t to_write = std::min(buf.size(), available);
        std::memcpy(data + pos_, buf.data(), to_write);
        pos_ += to_write;
        return Result<size_t>::ok(to_write);
    }

    // Seek: move cursor position
    Result<uint64_t> seek(SeekFrom from) {
        int64_t new_pos;
        switch (from.tag()) {
            case SeekFrom::StartTag:
                new_pos = from.offset();
                break;
            case SeekFrom::EndTag:
                new_pos = static_cast<int64_t>(get_len()) + from.offset();
                break;
            case SeekFrom::CurrentTag:
                new_pos = static_cast<int64_t>(pos_) + from.offset();
                break;
        }
        if (new_pos < 0) {
            return Result<uint64_t>::err(Error(Error::Kind::InvalidInput, "seek to negative position"));
        }
        pos_ = static_cast<size_t>(new_pos);
        return Result<uint64_t>::ok(static_cast<uint64_t>(pos_));
    }

    // Accessors
    size_t position() const { return pos_; }
    void set_position(size_t pos) { pos_ = pos; }
    const T& get_ref() const { return inner_; }
    T& get_mut() { return inner_; }
    T into_inner() { return std::move(inner_); }

private:
    // Helper to get raw data pointer (works with vector, span, array)
    const uint8_t* get_data() const {
        if constexpr (std::is_same_v<T, std::vector<uint8_t>>) {
            return inner_.data();
        } else if constexpr (std::is_same_v<T, std::span<const uint8_t>>) {
            return inner_.data();
        } else if constexpr (std::is_same_v<T, std::span<uint8_t>>) {
            return inner_.data();
        } else {
            return reinterpret_cast<const uint8_t*>(&inner_[0]);
        }
    }

    uint8_t* get_mut_data() {
        if constexpr (std::is_same_v<T, std::vector<uint8_t>>) {
            return inner_.data();
        } else if constexpr (std::is_same_v<T, std::span<uint8_t>>) {
            return inner_.data();
        } else {
            return reinterpret_cast<uint8_t*>(&inner_[0]);
        }
    }

    size_t get_len() const {
        if constexpr (std::is_same_v<T, std::vector<uint8_t>>) {
            return inner_.size();
        } else if constexpr (std::is_same_v<T, std::span<const uint8_t>> ||
                            std::is_same_v<T, std::span<uint8_t>>) {
            return inner_.size();
        } else {
            return sizeof(T);
        }
    }

    T inner_;
    size_t pos_;
};

template<typename T>
auto cursor_new(T&& inner) {
    using Inner = std::decay_t<T>;
    return Cursor<Inner>::new_(std::forward<T>(inner));
}

// ── Stdin / Stdout / Stderr ────────────────────────────

class Stdin {
public:
    Result<size_t> read(std::span<uint8_t> buf) {
        std::cin.read(reinterpret_cast<char*>(buf.data()), buf.size());
        auto count = std::cin.gcount();
        if (std::cin.bad()) {
            return Result<size_t>::err(Error("stdin read failed"));
        }
        return Result<size_t>::ok(static_cast<size_t>(count));
    }

    Result<std::string> read_line() {
        std::string line;
        if (std::getline(std::cin, line)) {
            return Result<std::string>::ok(std::move(line));
        }
        return Result<std::string>::err(Error("stdin read_line failed"));
    }
};

class Stdout {
public:
    Result<size_t> write(std::span<const uint8_t> buf) {
        std::cout.write(reinterpret_cast<const char*>(buf.data()), buf.size());
        if (std::cout.bad()) {
            return Result<size_t>::err(Error("stdout write failed"));
        }
        return Result<size_t>::ok(buf.size());
    }

    Result<void> flush() {
        std::cout.flush();
        return Result<void>::ok();
    }
};

class Stderr {
public:
    Result<size_t> write(std::span<const uint8_t> buf) {
        std::cerr.write(reinterpret_cast<const char*>(buf.data()), buf.size());
        if (std::cerr.bad()) {
            return Result<size_t>::err(Error("stderr write failed"));
        }
        return Result<size_t>::ok(buf.size());
    }

    Result<void> flush() {
        std::cerr.flush();
        return Result<void>::ok();
    }
};

// Factory functions (like Rust's std::io::stdin())
inline Stdin stdin_() { return Stdin{}; }
inline Stdout stdout_() { return Stdout{}; }
inline Stderr stderr_() { return Stderr{}; }

// Expanded Rust test harnesses may lower formatting calls through `std::io::_print`.
// Keep this as a permissive shim so generated code compiles even when formatting
// arguments are emitted as placeholder comments.
template<typename... Args>
inline void _print(Args&&...) {}

template<typename... Args>
inline void _eprint(Args&&...) {}

namespace detail {

template<typename T>
using remove_cvref_t = std::remove_cv_t<std::remove_reference_t<T>>;

template<typename T>
struct is_integral_span : std::false_type {};

template<typename Elem, std::size_t Extent>
struct is_integral_span<std::span<Elem, Extent>>
    : std::bool_constant<std::is_integral_v<std::remove_const_t<Elem>>> {};

template<typename T>
inline constexpr bool is_integral_span_v = is_integral_span<remove_cvref_t<T>>::value;

template<typename Span>
void advance_dynamic_span(Span& span, std::size_t count) {
    using SpanT = remove_cvref_t<Span>;
    if constexpr (SpanT::extent == std::dynamic_extent) {
        span = span.subspan(count);
    }
}

template<typename Bytes>
std::span<const uint8_t> as_write_bytes(Bytes&& bytes) {
    using std::data;
    using std::size;
    auto* ptr = data(bytes);
    using Elem = std::remove_cv_t<std::remove_pointer_t<decltype(ptr)>>;
    static_assert(sizeof(Elem) == 1, "io::write_all expects a byte-sized buffer");
    return std::span<const uint8_t>(
        reinterpret_cast<const uint8_t*>(ptr),
        static_cast<std::size_t>(size(bytes)));
}

} // namespace detail

// ── io::read/io::write dispatch helpers ───────────────
//
// The transpiler lowers some method-shape IO calls (notably for expanded `for_both!`
// paths) through these helpers so mixed payload branches (e.g. stream on one side,
// span on the other) do not require a uniform member-method surface.

template<typename Reader>
Result<size_t> read(Reader& reader, std::span<uint8_t> buf)
requires requires(Reader& r, std::span<uint8_t> b) { r.read(b); }
{
    return reader.read(buf);
}

template<typename Elem, std::size_t Extent>
Result<size_t> read(std::span<Elem, Extent>& reader, std::span<uint8_t> buf)
requires(std::is_integral_v<std::remove_const_t<Elem>>)
{
    const size_t to_read = std::min(buf.size(), reader.size());
    for (size_t i = 0; i < to_read; ++i) {
        buf[i] = static_cast<uint8_t>(reader[i]);
    }
    detail::advance_dynamic_span(reader, to_read);
    return Result<size_t>::ok(to_read);
}

template<typename Reader>
Result<size_t> read(Reader&, std::span<uint8_t>)
requires(
    !requires(Reader& r, std::span<uint8_t> b) { r.read(b); } &&
    !detail::is_integral_span_v<Reader>)
{
    return Result<size_t>::err(
        Error(Error::Kind::Unsupported, "type does not implement io::read"));
}

template<typename Writer>
Result<size_t> write(Writer& writer, std::span<const uint8_t> buf)
requires(
    requires(Writer& w, std::span<const uint8_t> b) { w.write(b); } ||
    requires(Writer& w, std::span<const uint8_t> b) { w.write_(b); })
{
    if constexpr (requires(Writer& w, std::span<const uint8_t> b) { w.write(b); }) {
        return writer.write(buf);
    } else {
        return writer.write_(buf);
    }
}

template<typename Elem, std::size_t Extent>
Result<size_t> write(std::span<Elem, Extent>& writer, std::span<const uint8_t> buf)
requires(std::is_integral_v<std::remove_const_t<Elem>> && !std::is_const_v<Elem>)
{
    const size_t to_write = std::min(buf.size(), writer.size());
    for (size_t i = 0; i < to_write; ++i) {
        writer[i] = static_cast<Elem>(buf[i]);
    }
    detail::advance_dynamic_span(writer, to_write);
    return Result<size_t>::ok(to_write);
}

template<typename Elem, std::size_t Extent>
Result<size_t> write(std::span<const Elem, Extent>&, std::span<const uint8_t>)
requires(std::is_integral_v<Elem>)
{
    return Result<size_t>::err(
        Error(Error::Kind::Unsupported, "io::write target is read-only span"));
}

template<typename Writer>
Result<size_t> write(Writer&, std::span<const uint8_t>)
requires(
    !requires(Writer& w, std::span<const uint8_t> b) { w.write(b); } &&
    !requires(Writer& w, std::span<const uint8_t> b) { w.write_(b); } &&
    !detail::is_integral_span_v<Writer>)
{
    return Result<size_t>::err(
        Error(Error::Kind::Unsupported, "type does not implement io::write"));
}

template<typename Writer>
Result<size_t> write(Writer* writer, std::span<const uint8_t> buf) {
    if (writer == nullptr) {
        return Result<size_t>::err(Error(Error::Kind::InvalidInput, "io::write null writer"));
    }
    return write(*writer, buf);
}

template<typename Writer, typename Bytes>
Result<std::tuple<>> write_all(Writer& writer, Bytes&& buf) {
    auto bytes = detail::as_write_bytes(std::forward<Bytes>(buf));
    if constexpr (requires(Writer& w, std::span<const uint8_t> b) { w.write_all(b); }) {
        if constexpr (std::is_void_v<decltype(writer.write_all(bytes))>) {
            writer.write_all(bytes);
            return Result<std::tuple<>>::ok(std::make_tuple());
        } else {
            return writer.write_all(bytes);
        }
    }
    std::size_t written = 0;
    while (written < bytes.size()) {
        auto write_result = write(writer, bytes.subspan(written));
        if (write_result.is_err()) {
            return Result<std::tuple<>>::err(write_result.unwrap_err());
        }
        auto n = write_result.unwrap();
        if (n == 0) {
            return Result<std::tuple<>>::err(
                Error(Error::Kind::WriteZero, "failed to write whole buffer"));
        }
        written += n;
    }
    return Result<std::tuple<>>::ok(std::make_tuple());
}

template<typename Writer, typename Bytes>
Result<std::tuple<>> write_all(Writer* writer, Bytes&& buf) {
    if (writer == nullptr) {
        return Result<std::tuple<>>::err(
            Error(Error::Kind::InvalidInput, "io::write_all null writer"));
    }
    return write_all(*writer, std::forward<Bytes>(buf));
}

template<typename Writer, typename FmtArg>
auto write_fmt(Writer& writer, FmtArg&& fmt_arg) {
    if constexpr (requires(Writer& w, FmtArg&& arg) { w.write_fmt(std::forward<FmtArg>(arg)); }) {
        return writer.write_fmt(std::forward<FmtArg>(fmt_arg));
    } else if constexpr (std::is_convertible_v<FmtArg, std::string_view>) {
        const auto view = std::string_view(std::forward<FmtArg>(fmt_arg));
        auto write_result = write(
            writer,
            std::span<const uint8_t>(
                reinterpret_cast<const uint8_t*>(view.data()),
                view.size()));
        if (write_result.is_err()) {
            return rusty::Result<std::tuple<>, Error>::Err(write_result.unwrap_err());
        }
        return rusty::Result<std::tuple<>, Error>::Ok(std::make_tuple());
    } else {
        return rusty::Result<std::tuple<>, Error>::Err(
            Error(Error::Kind::Unsupported, "type does not implement io::write_fmt"));
    }
}

template<typename Writer, typename FmtArg>
auto write_fmt(Writer* writer, FmtArg&& fmt_arg) {
    if (writer == nullptr) {
        return rusty::Result<std::tuple<>, Error>::Err(
            Error(Error::Kind::InvalidInput, "io::write_fmt null writer"));
    }
    return write_fmt(*writer, std::forward<FmtArg>(fmt_arg));
}

// ── copy ───────────────────────────────────────────────

/// Copy all bytes from reader to writer.
template<typename R, typename W>
Result<uint64_t> copy(R& reader, W& writer) {
    uint8_t buf[8192];
    uint64_t total = 0;
    while (true) {
        auto read_result = read(reader, std::span<uint8_t>(buf, sizeof(buf)));
        if (read_result.is_err()) return Result<uint64_t>::err(read_result.unwrap_err());
        size_t n = read_result.unwrap();
        if (n == 0) break;
        auto write_result = write(writer, std::span<const uint8_t>(buf, n));
        if (write_result.is_err()) return Result<uint64_t>::err(write_result.unwrap_err());
        total += n;
    }
    return Result<uint64_t>::ok(total);
}

} // namespace io
} // namespace rusty

#endif // RUSTY_IO_HPP
