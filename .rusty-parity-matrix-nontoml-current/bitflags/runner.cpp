// Auto-generated parity test runner
#include <cstdint>
#include <cstddef>
#include <limits>
#include <variant>
#include <string>
#include <optional>
#include <stdexcept>
#include <iostream>
#include <cassert>
#include <vector>
#include <functional>
#include <span>
#include <cstdlib>
#include <rusty/rusty.hpp>
#include <rusty/io.hpp>
#include <rusty/array.hpp>
#include <rusty/try.hpp>

// Parity shim for external `snapbox` assertions used by upstream Rust tests.
// This keeps Stage D buildable when registry dependencies are not transpiled.
namespace snapbox {
namespace data {
struct Position {
    rusty::path::PathBuf file;
    uint32_t line = 0;
    uint32_t column = 0;
};

struct Inline {
    Position position;
    std::string_view data;

    std::string_view raw() const { return data; }
};
} // namespace data

struct Data {
    std::string payload;

    std::string_view raw() const { return payload; }
};

namespace detail {
template<typename T>
std::string to_owned_string(T&& value) {
    if constexpr (requires { value.raw(); }) {
        return to_owned_string(value.raw());
    } else if constexpr (std::is_convertible_v<T, std::string_view>) {
        return std::string(std::string_view(std::forward<T>(value)));
    } else if constexpr (std::is_convertible_v<T, std::string>) {
        return std::string(std::forward<T>(value));
    } else {
        return std::string("<snapbox-data>");
    }
}
} // namespace detail

struct IntoData {
    template<typename T>
    static Data into_data(T&& value) {
        return Data{detail::to_owned_string(std::forward<T>(value))};
    }
};

namespace assert_ {
inline constexpr bool DEFAULT_ACTION_ENV = false;
} // namespace assert_

class Assert {
public:
    static Assert new_() { return Assert{}; }

    Assert& action_env(bool) { return *this; }

    template<typename Left, typename Right>
    void eq(Left&& left, Right&& right) const {
        const auto lhs = IntoData::into_data(std::forward<Left>(left)).payload;
        const auto rhs = IntoData::into_data(std::forward<Right>(right)).payload;
        if (lhs != rhs) {
            throw std::runtime_error(
                std::string("snapbox::Assert::eq failed\nleft:\n")
                + lhs + "\nright:\n" + rhs
            );
        }
    }
};
} // namespace snapbox

// Overloaded visitor helper
template<class... Ts> struct overloaded : Ts... { using Ts::operator()...; };
template<class... Ts>
overloaded(Ts...) -> overloaded<Ts...>;

// Hoisted namespace forward declarations
namespace iter { template<typename B> struct Iter; }
namespace iter { template<typename B> struct IterNames; }
namespace parser { struct ParseError; }
namespace parser { struct ParseErrorKind; }
namespace tests { struct TestEmpty; }
namespace tests { struct TestExternal; }
namespace tests { struct TestExternalFull; }
namespace tests { struct TestFlags; }
namespace tests { struct TestFlagsInvert; }
namespace tests { struct TestOverlapping; }
namespace tests { struct TestOverlappingFull; }
namespace tests { struct TestUnicode; }
namespace tests { struct TestZero; }
namespace tests { struct TestZeroOne; }
namespace traits { template<typename B> struct Flag; }

// ── from bitflags.cppm ──



#if defined(__GNUC__)
#pragma GCC diagnostic ignored "-Wunused-local-typedefs"
#endif


namespace iter {}
namespace parser {}
namespace tests {}
namespace traits {}

namespace iter {}


namespace rusty {
namespace cmp {
enum class Ordering { Less, Equal, Greater };
template<typename T>
struct Reverse {
T _0;
Reverse() = default;
explicit Reverse(T value) : _0(std::move(value)) {}
bool operator==(const Reverse&) const = default;
bool operator<(const Reverse& other) const {
return other._0 < _0;
}
};
namespace detail {
template<typename... Ts>
inline constexpr bool dependent_false_v = false;
template<typename L, typename R>
bool less_than(const L& lhs, const R& rhs) {
if constexpr (requires { lhs < rhs; }) {
return lhs < rhs;
} else if constexpr (requires {
std::begin(lhs); std::end(lhs); std::begin(rhs); std::end(rhs);
*std::begin(lhs) < *std::begin(rhs);
*std::begin(rhs) < *std::begin(lhs);
}) {
return std::lexicographical_compare(
std::begin(lhs), std::end(lhs),
std::begin(rhs), std::end(rhs),
[](const auto& a, const auto& b) { return a < b; }
);
} else {
// Keep parity builds permissive when no ordering surface is available.
// This degrades ordering semantics to equality, but avoids hard compile
// failures for non-orderable generated helper types.
return false;
}
}
} // namespace detail
// Compare two values and return Ordering (works for primitives and types with </>).
template<typename A, typename B>
Ordering cmp(const A& a, const B& b) {
if constexpr (requires { a.cmp(b); }) {
return a.cmp(b);
} else {
if (detail::less_than(a, b)) return Ordering::Less;
if (detail::less_than(b, a)) return Ordering::Greater;
return Ordering::Equal;
}
}
template<typename F>
Ordering then_with(Ordering ord, F&& f) {
if (ord == Ordering::Equal) {
return std::forward<F>(f)();
}
return ord;
}
template<typename T>
const T& min(const T& lhs, const T& rhs) {
return detail::less_than(rhs, lhs) ? rhs : lhs;
}
template<typename T>
const T& max(const T& lhs, const T& rhs) {
return detail::less_than(lhs, rhs) ? rhs : lhs;
}
}
// Clone: dispatches to .clone() if available, otherwise copy-constructs.
template<typename T>
auto clone(const T& value) {
if constexpr (requires { value.clone(); }) {
return value.clone();
} else {
return value;
}
}
template<typename Iter>
auto size_hint(const Iter& iter) -> decltype(iter.size_hint()) {
return iter.size_hint();
}
template<typename Value>
decltype(auto) left(Value&& value) {
return std::forward<Value>(value).left();
}
template<typename Value>
decltype(auto) right(Value&& value) {
return std::forward<Value>(value).right();
}
template<typename L, typename R>
struct Either_Left { L _0; };
template<typename L, typename R>
struct Either_Right { R _0; };
template<typename L, typename R>
using Either = std::variant<Either_Left<L, R>, Either_Right<L, R>>;
namespace either {
template<typename L, typename R>
Either_Left<L, R> Left(L _0) { return Either_Left<L, R>{std::forward<L>(_0)}; }
template<typename L, typename R>
Either_Right<L, R> Right(R _0) { return Either_Right<L, R>{std::forward<R>(_0)}; }
}
// Display-oriented conversion helper used by format_args lowering.
template<typename T>
std::string to_string(const T& value);
template<typename T>
std::string to_debug_string(const T& value);
template<typename T>
std::string to_debug_string_pretty(const T& value);
template<typename T>
constexpr decltype(auto) format_numeric_arg(T&& value);
// Rust u8::is_ascii_digit() — check if byte is in '0'..='9'.
#ifndef RUSTY_HAS_IS_ASCII_DIGIT
inline bool is_ascii_digit(uint8_t b) { return b >= '0' && b <= '9'; }
#endif
// Rust char/u8::is_ascii_hexdigit() — ASCII-only hex digit test.
#ifndef RUSTY_HAS_IS_ASCII_HEXDIGIT
inline bool is_ascii_hexdigit(char32_t c) {
return (c >= U'0' && c <= U'9')
|| (c >= U'a' && c <= U'f')
|| (c >= U'A' && c <= U'F');
}
#endif
template<typename T>
inline bool is_nan(T value) {
using V = std::remove_cv_t<std::remove_reference_t<T>>;
if constexpr (std::is_floating_point_v<V>) {
return value != value;
} else {
return false;
}
}
template<typename T>
inline bool is_infinite(T value) {
using V = std::remove_cv_t<std::remove_reference_t<T>>;
if constexpr (std::is_floating_point_v<V>) {
const auto inf = std::numeric_limits<V>::infinity();
return value == inf || value == -inf;
} else {
return false;
}
}
template<typename T>
inline bool is_finite(T value) {
return !is_nan(value) && !is_infinite(value);
}
template<typename T>
inline bool is_sign_negative(T value) {
using V = std::remove_cv_t<std::remove_reference_t<T>>;
if constexpr (std::is_same_v<V, float>) {
return (std::bit_cast<uint32_t>(value) & 0x80000000u) != 0;
} else if constexpr (std::is_same_v<V, double>) {
return (std::bit_cast<uint64_t>(value) & 0x8000000000000000ULL) != 0;
} else if constexpr (std::is_floating_point_v<V>) {
return value < static_cast<V>(0)
|| (value == static_cast<V>(0) && (static_cast<V>(1) / value) < static_cast<V>(0));
} else if constexpr (std::is_signed_v<V>) {
return value < static_cast<V>(0);
} else {
return false;
}
}
template<typename T>
inline bool is_sign_positive(T value) {
return !is_sign_negative(value);
}
} // namespace rusty
// DefaultHasher stub — used by expanded #[derive(Hash)] test code.
struct DefaultHasher {
std::size_t state = 14695981039346656037ULL;
static DefaultHasher new_() { return DefaultHasher{}; }
std::size_t finish() const { return state; }
};
namespace rusty {
// Convert Option<Ordering> to std::partial_ordering for C++ spaceship operator
inline std::partial_ordering to_partial_ordering(const rusty::Option<rusty::cmp::Ordering>& opt) {
if (opt.is_none()) return std::partial_ordering::unordered;
switch (static_cast<int>(opt.unwrap())) {
case static_cast<int>(rusty::cmp::Ordering::Less): return std::partial_ordering::less;
case static_cast<int>(rusty::cmp::Ordering::Equal): return std::partial_ordering::equivalent;
case static_cast<int>(rusty::cmp::Ordering::Greater): return std::partial_ordering::greater;
default: return std::partial_ordering::unordered;
}
}
template<typename A, typename B>
auto partial_cmp(const A& a, const B& b) {
if constexpr (requires { a.partial_cmp(b); }) {
return a.partial_cmp(b);
} else {
if (rusty::cmp::detail::less_than(a, b)) return rusty::Option<rusty::cmp::Ordering>(rusty::cmp::Ordering::Less);
if (rusty::cmp::detail::less_than(b, a)) return rusty::Option<rusty::cmp::Ordering>(rusty::cmp::Ordering::Greater);
return rusty::Option<rusty::cmp::Ordering>(rusty::cmp::Ordering::Equal);
}
}
namespace fmt {
// Error and Result are defined in rusty/fmt.hpp
struct Arguments {};
enum class Alignment { Left, Right, Center };
struct DebugList {
template<typename... Args>
DebugList& entries(Args&&...) { return *this; }
Result finish() { return Result::Ok(std::make_tuple()); }
};
struct Formatter {
mutable std::string out_;
std::string str() const { return out_; }
struct DebugTuple {
const Formatter* formatter;
bool first = true;
explicit DebugTuple(const Formatter* formatter_init) : formatter(formatter_init) {}
template<typename Name>
DebugTuple& name(Name&& value) {
if (formatter) {
formatter->append_one(std::forward<Name>(value));
formatter->out_ += "(";
}
return *this;
}
template<typename Arg>
DebugTuple& field(Arg&& arg) {
if (formatter) {
if (!first) { formatter->out_ += ", "; }
first = false;
formatter->out_ += rusty::to_debug_string(std::forward<Arg>(arg));
}
return *this;
}
Result finish() {
if (formatter) {
formatter->out_ += ")";
}
return Result::Ok(std::make_tuple());
}
};
struct DebugStruct {
const Formatter* formatter;
bool first = true;
explicit DebugStruct(const Formatter* formatter_init) : formatter(formatter_init) {}
template<typename Name>
DebugStruct& name(Name&& value) {
if (formatter) {
formatter->append_one(std::forward<Name>(value));
formatter->out_ += "{";
}
return *this;
}
template<typename FieldName, typename Arg>
DebugStruct& field(FieldName&& field_name, Arg&& arg) {
if (formatter) {
if (!first) { formatter->out_ += ", "; }
first = false;
formatter->append_one(std::forward<FieldName>(field_name));
formatter->out_ += ": ";
formatter->out_ += rusty::to_debug_string(std::forward<Arg>(arg));
}
return *this;
}
Result finish() {
if (formatter) {
formatter->out_ += "}";
}
return Result::Ok(std::make_tuple());
}
};
template<typename... Args>
static Result debug_tuple_field1_finish(Args&&...) { return Result::Ok(std::make_tuple()); }
template<typename... Args>
static Result debug_tuple_field2_finish(Args&&...) { return Result::Ok(std::make_tuple()); }
template<typename... Args>
static Result debug_tuple_field3_finish(Args&&...) { return Result::Ok(std::make_tuple()); }
template<typename... Args>
static Result debug_tuple_field4_finish(Args&&...) { return Result::Ok(std::make_tuple()); }
template<typename... Args>
static Result debug_tuple_field5_finish(Args&&...) { return Result::Ok(std::make_tuple()); }
template<typename... Args>
static Result debug_struct_field1_finish(Args&&...) { return Result::Ok(std::make_tuple()); }
template<typename... Args>
static Result debug_struct_field2_finish(Args&&...) { return Result::Ok(std::make_tuple()); }
template<typename... Args>
static Result debug_struct_field3_finish(Args&&...) { return Result::Ok(std::make_tuple()); }
template<typename... Args>
static Result debug_struct_field4_finish(Args&&...) { return Result::Ok(std::make_tuple()); }
template<typename... Args>
static Result debug_struct_field5_finish(Args&&...) { return Result::Ok(std::make_tuple()); }
template<typename... Args>
static Result debug_struct_field6_finish(Args&&...) { return Result::Ok(std::make_tuple()); }
template<typename... Args>
static Result debug_struct_field7_finish(Args&&...) { return Result::Ok(std::make_tuple()); }
template<typename... Args>
static Result debug_struct_field8_finish(Args&&...) { return Result::Ok(std::make_tuple()); }
template<typename... Args>
Result write_fmt(Args&&... args) const { (append_one(std::forward<Args>(args)), ...); return Result::Ok(std::make_tuple()); }
rusty::Option<size_t> width() const { return rusty::Option<size_t>(rusty::None); }
rusty::Option<Alignment> align() const { return rusty::Option<Alignment>(rusty::None); }
char fill() const { return ' '; }
template<typename Ch>
Result write_char(Ch&& ch) const { out_.push_back(static_cast<char>(ch)); return Result::Ok(std::make_tuple()); }
template<typename Str>
Result write_str(Str&& s) const { out_ += rusty::to_string(std::forward<Str>(s)); return Result::Ok(std::make_tuple()); }
template<typename Name>
DebugTuple debug_tuple(Name&& name) const {
DebugTuple tuple(this);
tuple.name(std::forward<Name>(name));
return tuple;
}
template<typename Name>
DebugStruct debug_struct(Name&& name) const {
DebugStruct st(this);
st.name(std::forward<Name>(name));
return st;
}
DebugList debug_list() const { return DebugList{}; }
private:
template<typename Arg>
void append_one(Arg&& arg) const { out_ += rusty::to_string(std::forward<Arg>(arg)); }
};
}
namespace detail {
inline std::string utf8_from_char32(char32_t ch) {
std::string out;
if (ch <= 0x7F) {
out.push_back(static_cast<char>(ch));
} else if (ch <= 0x7FF) {
out.push_back(static_cast<char>(0xC0 | ((ch >> 6) & 0x1F)));
out.push_back(static_cast<char>(0x80 | (ch & 0x3F)));
} else if (ch <= 0xFFFF) {
out.push_back(static_cast<char>(0xE0 | ((ch >> 12) & 0x0F)));
out.push_back(static_cast<char>(0x80 | ((ch >> 6) & 0x3F)));
out.push_back(static_cast<char>(0x80 | (ch & 0x3F)));
} else {
out.push_back(static_cast<char>(0xF0 | ((ch >> 18) & 0x07)));
out.push_back(static_cast<char>(0x80 | ((ch >> 12) & 0x3F)));
out.push_back(static_cast<char>(0x80 | ((ch >> 6) & 0x3F)));
out.push_back(static_cast<char>(0x80 | (ch & 0x3F)));
}
return out;
}
inline std::string escape_debug_string(std::string_view input) {
std::string out;
out.reserve(input.size());
for (char ch : input) {
switch (ch) {
case '\\': out += "\\\\"; break;
case '"': out += "\\\""; break;
case '\n': out += "\\n"; break;
case '\r': out += "\\r"; break;
case '\t': out += "\\t"; break;
default: out.push_back(ch); break;
}
}
return out;
}
inline std::string pretty_debug_string(std::string_view input) {
if (input.find(',') == std::string_view::npos
&& input.find('[') == std::string_view::npos
&& input.find('{') == std::string_view::npos) {
return std::string(input);
}
struct Frame {
char open;
bool saw_value;
bool last_was_comma;
};
auto matching_close = [](char open) {
switch (open) {
case '(': return ')';
case '[': return ']';
case '{': return '}';
default: return '\0';
}
};
auto append_indent = [](std::string& out, std::size_t spaces) {
out.append(spaces, ' ');
};
auto is_ws = [](char c) {
return c == ' ' || c == '\n' || c == '\r' || c == '\t' || c == '\f' || c == '\v';
};

std::string out;
out.reserve(input.size() + 32);
std::vector<Frame> stack;
bool in_string = false;
bool escaped = false;

for (std::size_t i = 0; i < input.size(); ++i) {
const char ch = input[i];
if (in_string) {
out.push_back(ch);
if (escaped) {
escaped = false;
} else if (ch == '\\') {
escaped = true;
} else if (ch == '"') {
in_string = false;
}
continue;
}
if (is_ws(ch)) {
continue;
}
if (ch == '"') {
if (!stack.empty()) {
stack.back().saw_value = true;
stack.back().last_was_comma = false;
}
in_string = true;
out.push_back(ch);
continue;
}
if (ch == '(' || ch == '[' || ch == '{') {
if (!stack.empty()) {
stack.back().saw_value = true;
stack.back().last_was_comma = false;
}
out.push_back(ch);
stack.push_back(Frame{ch, false, false});
std::size_t j = i + 1;
while (j < input.size() && is_ws(input[j])) {
++j;
}
if (j < input.size() && input[j] != matching_close(ch)) {
out.push_back('\n');
append_indent(out, stack.size() * 4);
}
continue;
}
if (!stack.empty() && ch == ',') {
out.push_back(',');
stack.back().last_was_comma = true;
out.push_back('\n');
append_indent(out, stack.size() * 4);
continue;
}
if (!stack.empty() && ch == matching_close(stack.back().open)) {
auto frame = stack.back();
stack.pop_back();
if (frame.saw_value && !frame.last_was_comma) {
out.push_back(',');
}
if (frame.saw_value) {
out.push_back('\n');
append_indent(out, stack.size() * 4);
}
out.push_back(ch);
if (!stack.empty()) {
stack.back().saw_value = true;
stack.back().last_was_comma = false;
}
continue;
}
out.push_back(ch);
if (!stack.empty()) {
stack.back().saw_value = true;
stack.back().last_was_comma = false;
}
}
return out;
}
}
template<typename T>
std::string to_string(const T& value) {
using Value = std::remove_cv_t<std::remove_reference_t<T>>;
if constexpr (requires { value.to_string(); }) {
return value.to_string();
} else if constexpr (std::is_same_v<Value, bool>) {
return value ? "true" : "false";
} else if constexpr (std::is_same_v<Value, std::int8_t> || std::is_same_v<Value, std::uint8_t>) {
return std::to_string(static_cast<int>(value));
} else if constexpr (std::is_same_v<Value, char>
|| std::is_same_v<Value, signed char>
|| std::is_same_v<Value, unsigned char>
|| std::is_same_v<Value, wchar_t>
|| std::is_same_v<Value, char16_t>
|| std::is_same_v<Value, char32_t>) {
return rusty::detail::utf8_from_char32(static_cast<char32_t>(value));
} else if constexpr (std::is_convertible_v<T, std::string_view>) {
return std::string(std::string_view(value));
} else if constexpr (requires { value.as_str(); }) {
return std::string(value.as_str());
} else if constexpr (requires { std::string_view(*value); }) {
return std::string(std::string_view(*value));
} else if constexpr (std::is_pointer_v<Value>) {
if (value == nullptr) {
return "<null>";
}
using Pointee = std::remove_cv_t<std::remove_pointer_t<Value>>;
if constexpr (std::is_void_v<Pointee>) {
return std::format("0x{:x}", static_cast<std::uintptr_t>(reinterpret_cast<std::uintptr_t>(value)));
} else {
return rusty::to_string(*value);
}
} else if constexpr (requires(rusty::fmt::Formatter& f) { rusty_fmt(value, f); }) {
rusty::fmt::Formatter formatter{};
auto result = rusty_fmt(value, formatter);
if (result.is_ok()) {
return formatter.str();
}
return "<fmt-error>";
} else if constexpr (requires { std::to_string(value); }) {
return std::to_string(value);
} else if constexpr (requires(rusty::fmt::Formatter& f) { value.fmt(f); }) {
rusty::fmt::Formatter formatter{};
auto result = value.fmt(formatter);
if (result.is_ok()) {
return formatter.str();
}
return "<fmt-error>";
} else {
return "<unprintable>";
}
}
template<typename Range, typename Sep>
std::string join(const Range& range, Sep&& sep)
requires (
requires { std::begin(range); std::end(range); }
&& !requires { range.join(std::forward<Sep>(sep)); }
) {
const auto delimiter = rusty::to_string(std::forward<Sep>(sep));
std::string out;
bool first = true;
for (const auto& item : range) {
if (!first) {
out += delimiter;
}
first = false;
out += rusty::to_string(item);
}
return out;
}
template<typename T>
std::string to_debug_string(const T& value) {
using Value = std::remove_cv_t<std::remove_reference_t<T>>;
if constexpr (std::is_same_v<Value, std::int8_t> || std::is_same_v<Value, std::uint8_t>) {
return std::to_string(static_cast<int>(value));
} else if constexpr (std::is_same_v<Value, char>
|| std::is_same_v<Value, signed char>
|| std::is_same_v<Value, unsigned char>
|| std::is_same_v<Value, wchar_t>
|| std::is_same_v<Value, char16_t>
|| std::is_same_v<Value, char32_t>) {
const auto ch = static_cast<char32_t>(value);
if (ch == U'\0') {
return "'\\0'";
}
return std::string("'") + rusty::detail::utf8_from_char32(ch) + "'";
} else if constexpr (std::is_convertible_v<T, std::string_view>) {
return std::string("\"")
+ rusty::detail::escape_debug_string(std::string(std::string_view(value)))
+ "\"";
} else if constexpr (requires { value.as_str(); }) {
return std::string("\"")
+ rusty::detail::escape_debug_string(std::string(value.as_str()))
+ "\"";
} else if constexpr (requires { std::begin(value); std::end(value); }) {
std::string out = "[";
bool first = true;
for (const auto& item : value) {
if (!first) {
out += ", ";
}
first = false;
out += rusty::to_debug_string(item);
}
out += "]";
return out;
}
return rusty::to_string(value);
}
template<typename T>
std::string to_debug_string_pretty(const T& value) {
return rusty::detail::pretty_debug_string(rusty::to_debug_string(value));
}
template<typename T>
constexpr decltype(auto) format_numeric_arg(T&& value) {
using Value = std::remove_cv_t<std::remove_reference_t<T>>;
if constexpr (std::is_integral_v<Value> && !std::is_same_v<Value, bool>) {
return std::forward<T>(value);
} else if constexpr (
requires { value._0; }
&& std::is_integral_v<std::remove_cv_t<std::remove_reference_t<decltype(value._0)>>>
&& !std::is_same_v<std::remove_cv_t<std::remove_reference_t<decltype(value._0)>>, bool>
) {
return value._0;
} else if constexpr (
requires { value.bits(); }
&& std::is_integral_v<std::remove_cv_t<std::remove_reference_t<decltype(value.bits())>>>
&& !std::is_same_v<std::remove_cv_t<std::remove_reference_t<decltype(value.bits())>>, bool>
) {
return value.bits();
} else {
return std::forward<T>(value);
}
}
namespace path {
using Path = std::string;
inline const Path& as_ref(std::string_view value) {
thread_local Path _path_ref_tmp;
_path_ref_tmp = std::string(value);
return _path_ref_tmp;
}
}
namespace time {
struct Duration {
std::chrono::nanoseconds inner;
static Duration from_secs(unsigned long secs) { return Duration{std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::seconds(secs))}; }
static Duration from_millis(unsigned long ms) { return Duration{std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::milliseconds(ms))}; }
static Duration from_nanos(unsigned long ns) { return Duration{std::chrono::nanoseconds(ns)}; }
std::uint64_t as_secs() const {
return static_cast<std::uint64_t>(std::chrono::duration_cast<std::chrono::seconds>(inner).count());
}
std::uint32_t subsec_nanos() const {
const auto secs_ns = std::chrono::duration_cast<std::chrono::nanoseconds>(
std::chrono::duration_cast<std::chrono::seconds>(inner));
return static_cast<std::uint32_t>((inner - secs_ns).count());
}
friend bool operator==(const Duration& lhs, const Duration& rhs) { return lhs.inner == rhs.inner; }
friend bool operator!=(const Duration& lhs, const Duration& rhs) { return lhs.inner != rhs.inner; }
friend bool operator<(const Duration& lhs, const Duration& rhs) { return lhs.inner < rhs.inner; }
friend bool operator<=(const Duration& lhs, const Duration& rhs) { return lhs.inner <= rhs.inner; }
friend bool operator>(const Duration& lhs, const Duration& rhs) { return lhs.inner > rhs.inner; }
friend bool operator>=(const Duration& lhs, const Duration& rhs) { return lhs.inner >= rhs.inner; }
template<typename Rep, typename Period>
operator std::chrono::duration<Rep, Period>() const { return std::chrono::duration_cast<std::chrono::duration<Rep, Period>>(inner); }
};
struct Instant {
std::chrono::steady_clock::time_point inner;
static Instant now() { return Instant{std::chrono::steady_clock::now()}; }
Duration duration_since(Instant earlier) const {
return Duration{std::chrono::duration_cast<std::chrono::nanoseconds>(inner - earlier.inner)};
}
};
struct SystemTime {
std::chrono::system_clock::time_point inner;
static SystemTime now() { return SystemTime{std::chrono::system_clock::now()}; }
rusty::Result<Duration, std::tuple<>> duration_since(SystemTime earlier) const {
if (inner >= earlier.inner) {
return rusty::Result<Duration, std::tuple<>>::Ok(
Duration{std::chrono::duration_cast<std::chrono::nanoseconds>(inner - earlier.inner)});
}
return rusty::Result<Duration, std::tuple<>>::Err(std::make_tuple());
}
};
inline const SystemTime UNIX_EPOCH{std::chrono::system_clock::time_point{}};
}
namespace future {
template<typename T>
struct Ready {
using Output = T;
T value;
bool done = false;
Ready into_future() { return std::move(*this); }
Ready new_unchecked() { return std::move(*this); }
Ready& as_mut() { return *this; }
rusty::Poll<T> poll(rusty::Context&) {
done = true;
return rusty::Poll<T>::ready_with(std::move(value));
}
};
template<typename T>
Ready<std::decay_t<T>> ready(T&& value) {
return Ready<std::decay_t<T>>{std::forward<T>(value), false};
}
struct Delay {
using Output = std::tuple<>;
std::chrono::nanoseconds duration{};
bool done = false;
static Delay new_(rusty::time::Duration duration) { return Delay{duration.inner, false}; }
Delay into_future() { return std::move(*this); }
Delay new_unchecked() { return std::move(*this); }
Delay& as_mut() { return *this; }
rusty::Poll<std::tuple<>> poll(rusty::Context&) {
if (!done) {
std::this_thread::sleep_for(duration);
done = true;
}
return rusty::Poll<std::tuple<>>::ready_with(std::tuple<>{});
}
};
}
namespace ffi {
using OsStr = std::string;
using CStr = std::string;
using OsString = std::string;
using CString = std::string;
inline rusty::Result<CString, rusty::String> cstring_new(std::string_view value) {
return rusty::Result<CString, rusty::String>::Ok(std::string(value));
}
inline rusty::Result<CString, rusty::String> cstring_new(rusty::String value) {
return cstring_new(std::string_view(value.as_str()));
}
template<typename Bytes>
rusty::Result<CString, rusty::String> cstring_new(const Bytes& bytes) {
if constexpr (requires { bytes.data(); bytes.size(); }) {
const auto* raw = bytes.data();
const auto len = static_cast<std::size_t>(bytes.size());
const auto* data = reinterpret_cast<const char*>(raw);
return rusty::Result<CString, rusty::String>::Ok(std::string(data, len));
}
return rusty::Result<CString, rusty::String>::Err(rusty::String::from("unsupported CString input"));
}
}
template<typename Target, typename Input>
Target from_into(Input&& input) {
if constexpr (requires { Target::from(std::forward<Input>(input)); }) {
return Target::from(std::forward<Input>(input));
} else if constexpr (requires { Target::new_(std::forward<Input>(input)); }) {
return Target::new_(std::forward<Input>(input));
} else if constexpr (std::is_constructible_v<Target, Input&&>) {
return Target(std::forward<Input>(input));
} else if constexpr (std::is_convertible_v<Input&&, Target>) {
return static_cast<Target>(std::forward<Input>(input));
} else if constexpr (std::is_convertible_v<Input&&, std::string_view>) {
auto view = std::string_view(std::forward<Input>(input));
if constexpr (requires { Target::from(view); }) {
return Target::from(view);
} else if constexpr (requires { Target::new_(view); }) {
return Target::new_(view);
} else if constexpr (std::is_constructible_v<Target, std::string_view>) {
return Target(view);
} else if constexpr (std::is_convertible_v<std::string_view, Target>) {
return static_cast<Target>(view);
} else {
static_assert(!std::is_same_v<Target, Target>, "rusty::from_into: unsupported conversion");
return Target{};
}
} else {
static_assert(!std::is_same_v<Target, Target>, "rusty::from_into: unsupported conversion");
return Target{};
}
}
template<typename Target, typename Input>
Target as_ref_into(Input&& input) {
using RawTarget = std::remove_cv_t<std::remove_reference_t<Target>>;
if constexpr (std::is_same_v<RawTarget, rusty::path::Path>) {
if constexpr (std::is_convertible_v<Input, std::string_view>) {
return static_cast<Target>(rusty::path::as_ref(std::string_view(input)));
} else if constexpr (requires { input.as_str(); }) {
return static_cast<Target>(rusty::path::as_ref(std::string_view(input.as_str())));
}
}
if constexpr (requires { std::forward<Input>(input).as_ref(); }) {
return static_cast<Target>(std::forward<Input>(input).as_ref());
} else {
return static_cast<Target>(std::forward<Input>(input));
}
}
template<typename T>
constexpr T* addr_of_temp(T& value) {
return &value;
}
template<typename T>
const std::remove_cv_t<std::remove_reference_t<T>>* addr_of_temp(T&& value) {
using Stored = std::remove_cv_t<std::remove_reference_t<T>>;
thread_local std::optional<Stored> _addr_of_tmp;
_addr_of_tmp.reset();
_addr_of_tmp.emplace(std::forward<T>(value));
return &*_addr_of_tmp;
}
struct Cow_Borrowed {
std::string_view _0;
Cow_Borrowed() : _0(std::string_view{}) {}
explicit Cow_Borrowed(std::string_view value) : _0(value) {}
bool operator==(const Cow_Borrowed& other) const { return _0 == other._0; }
bool operator<(const Cow_Borrowed& other) const { return _0 < other._0; }
};
struct Cow_Owned {
rusty::String _0;
explicit Cow_Owned(rusty::String value) : _0(std::move(value)) {}
bool operator==(const Cow_Owned& other) const { return _0 == other._0; }
bool operator<(const Cow_Owned& other) const { return _0 < other._0; }
};
using Cow = std::variant<Cow_Borrowed, Cow_Owned>;
inline Cow clone(const Cow& value) {
if (const auto* borrowed = std::get_if<Cow_Borrowed>(&value)) {
return Cow_Borrowed{borrowed->_0};
}
if (const auto* owned = std::get_if<Cow_Owned>(&value)) {
return Cow_Owned{owned->_0.clone()};
}
return Cow_Borrowed{std::string_view{}};
}
inline rusty::String& to_mut(Cow& value) {
if (const auto* borrowed = std::get_if<Cow_Borrowed>(&value)) {
value = Cow_Owned{rusty::String::from(borrowed->_0)};
}
return std::get<Cow_Owned>(value)._0;
}
inline rusty::String into_owned(Cow value) {
if (const auto* borrowed = std::get_if<Cow_Borrowed>(&value)) {
return rusty::String::from(borrowed->_0);
}
if (auto* owned = std::get_if<Cow_Owned>(&value)) {
return std::move(owned->_0);
}
return rusty::String::new_();
}
template<typename T>
decltype(auto) into_owned(T&& value) {
if constexpr (requires { std::forward<T>(value).into_owned(); }) {
return std::forward<T>(value).into_owned();
} else {
return std::forward<T>(value);
}
}
inline std::string_view as_str(const Cow& value) {
if (const auto* borrowed = std::get_if<Cow_Borrowed>(&value)) {
return borrowed->_0;
}
if (const auto* owned = std::get_if<Cow_Owned>(&value)) {
return owned->_0.as_str();
}
return std::string_view{};
}
inline std::string_view to_string_view(const Cow& value) {
return as_str(value);
}
namespace pin {
template<typename T>
using Pin = T;
template<typename T>
constexpr decltype(auto) new_unchecked(T&& value) {
return std::forward<T>(value);
}
template<typename T>
constexpr const T* get_ref(const T& value) {
return &value;
}
template<typename T>
constexpr T* get_unchecked_mut(T& value) {
return &value;
}
}
namespace hash {
template<typename State>
inline void combine(State& state, std::size_t value) {
std::size_t seed;
if constexpr (requires { state.state; }) {
seed = state.state;
} else {
seed = static_cast<std::size_t>(state);
}
seed ^= value + 0x9e3779b97f4a7c15ULL + (seed << 6) + (seed >> 2);
if constexpr (requires { state.state; }) {
state.state = seed;
} else {
state = static_cast<State>(seed);
}
}
template<typename T, typename State>
void hash(const T& value, State& state) {
if constexpr (requires { value.hash(state); }) {
value.hash(state);
} else if constexpr (requires { std::begin(value); std::end(value); }) {
// Hash range-like containers by element value, not object bytes.
// This avoids pointer/address-based drift for owning containers.
for (const auto& item : value) {
hash(item, state);
}
} else if constexpr (requires { std::hash<std::remove_cvref_t<T>>{}(value); }) {
combine(state, std::hash<std::remove_cvref_t<T>>{}(value));
} else {
const auto* bytes = reinterpret_cast<const unsigned char*>(&value);
std::size_t h = 14695981039346656037ULL;
for (std::size_t i = 0; i < sizeof(T); ++i) {
h ^= static_cast<std::size_t>(bytes[i]);
h *= 1099511628211ULL;
}
combine(state, h);
}
}
}
template<typename Writer, typename FmtArg>
rusty::fmt::Result write_fmt(Writer&& writer, FmtArg&& fmt_arg) {
const auto text = rusty::to_string(std::forward<FmtArg>(fmt_arg));
const auto text_view = std::string_view(text);
if constexpr (requires { std::forward<Writer>(writer).write_fmt(text_view); }) {
return std::forward<Writer>(writer).write_fmt(text_view);
} else if constexpr (requires { writer.write_fmt(text_view); }) {
return writer.write_fmt(text_view);
} else if constexpr (requires { std::forward<Writer>(writer).write_str(text_view); }) {
return std::forward<Writer>(writer).write_str(text_view);
} else if constexpr (requires { writer.write_str(text_view); }) {
return writer.write_str(text_view);
} else if constexpr (requires { std::forward<Writer>(writer).formatter.write_str(text_view); }) {
if constexpr (requires { std::forward<Writer>(writer).has_decimal_point; }) {
if (text_view.find('.') != std::string_view::npos) {
std::forward<Writer>(writer).has_decimal_point = true;
}
}
return std::forward<Writer>(writer).formatter.write_str(text_view);
} else if constexpr (requires { writer.formatter.write_str(text_view); }) {
if constexpr (requires { writer.has_decimal_point; }) {
if (text_view.find('.') != std::string_view::npos) {
writer.has_decimal_point = true;
}
}
return writer.formatter.write_str(text_view);
} else {
return rusty::fmt::Result::Err(rusty::fmt::Error{});
}
}
template<typename Value, typename Writer>
rusty::fmt::Result write_hex(const Value& value, Writer&& writer) {
using RawValue = std::remove_cv_t<std::remove_reference_t<Value>>;
if constexpr (!std::is_integral_v<RawValue> || std::is_same_v<RawValue, bool>) {
return rusty::fmt::Result::Err(rusty::fmt::Error{});
} else {
using Unsigned = std::make_unsigned_t<RawValue>;
Unsigned bits = static_cast<Unsigned>(value);
std::string text;
do {
const auto digit = static_cast<unsigned>(bits & static_cast<Unsigned>(0xF));
text.push_back("0123456789abcdef"[digit]);
bits = static_cast<Unsigned>(bits >> 4);
} while (bits != 0);
std::reverse(text.begin(), text.end());
const auto text_view = std::string_view(text);
if constexpr (requires { std::forward<Writer>(writer).write_str(text_view); }) {
return std::forward<Writer>(writer).write_str(text_view);
} else if constexpr (requires { writer.write_str(text_view); }) {
return writer.write_str(text_view);
} else {
return rusty::fmt::Result::Err(rusty::fmt::Error{});
}
}
}
template<typename T, typename Input>
rusty::Result<T, std::tuple<>> parse_hex(const Input& input) {
std::string_view text;
if constexpr (std::is_convertible_v<Input, std::string_view>) {
text = std::string_view(input);
} else if constexpr (requires { input.as_str(); }) {
text = std::string_view(input.as_str());
} else {
return rusty::Result<T, std::tuple<>>::Err(std::make_tuple());
}
if (text.empty()) {
return rusty::Result<T, std::tuple<>>::Err(std::make_tuple());
}
bool negative = false;
std::size_t start = 0;
if (text[0] == '+' || text[0] == '-') {
negative = text[0] == '-';
start = 1;
}
if (start >= text.size()) {
return rusty::Result<T, std::tuple<>>::Err(std::make_tuple());
}
using RawT = std::remove_cv_t<std::remove_reference_t<T>>;
if constexpr (!std::is_integral_v<RawT> || std::is_same_v<RawT, bool>) {
return rusty::Result<T, std::tuple<>>::Err(std::make_tuple());
} else {
using Unsigned = std::make_unsigned_t<RawT>;
Unsigned value = 0;
for (std::size_t i = start; i < text.size(); ++i) {
const char ch = text[i];
unsigned digit = 0;
if (ch >= '0' && ch <= '9') {
digit = static_cast<unsigned>(ch - '0');
} else if (ch >= 'a' && ch <= 'f') {
digit = static_cast<unsigned>(10 + (ch - 'a'));
} else if (ch >= 'A' && ch <= 'F') {
digit = static_cast<unsigned>(10 + (ch - 'A'));
} else {
return rusty::Result<T, std::tuple<>>::Err(std::make_tuple());
}
if (value > (std::numeric_limits<Unsigned>::max() - static_cast<Unsigned>(digit))
/ static_cast<Unsigned>(16)) {
return rusty::Result<T, std::tuple<>>::Err(std::make_tuple());
}
value = static_cast<Unsigned>(value * static_cast<Unsigned>(16)
+ static_cast<Unsigned>(digit));
}
if constexpr (std::is_signed_v<RawT>) {
if (negative) {
const auto max_mag = static_cast<Unsigned>(std::numeric_limits<RawT>::max())
+ static_cast<Unsigned>(1);
if (value > max_mag) {
return rusty::Result<T, std::tuple<>>::Err(std::make_tuple());
}
if (value == max_mag) {
return rusty::Result<T, std::tuple<>>::Ok(std::numeric_limits<RawT>::min());
}
const auto signed_value = static_cast<RawT>(value);
return rusty::Result<T, std::tuple<>>::Ok(static_cast<RawT>(-signed_value));
}
if (value > static_cast<Unsigned>(std::numeric_limits<RawT>::max())) {
return rusty::Result<T, std::tuple<>>::Err(std::make_tuple());
}
return rusty::Result<T, std::tuple<>>::Ok(static_cast<RawT>(value));
} else {
if (negative) {
return rusty::Result<T, std::tuple<>>::Err(std::make_tuple());
}
return rusty::Result<T, std::tuple<>>::Ok(static_cast<RawT>(value));
}
}
}
namespace str_runtime {
using Utf8Error = rusty::String;
inline bool is_valid_utf8(const unsigned char* data, std::size_t len) {
std::size_t i = 0;
while (i < len) {
const auto byte = data[i];
if (byte <= 0x7F) {
++i;
continue;
}
if ((byte >> 5) == 0x6) {
if (i + 1 >= len) return false;
const auto b1 = data[i + 1];
if ((b1 & 0xC0) != 0x80 || byte < 0xC2) return false;
i += 2;
continue;
}
if ((byte >> 4) == 0xE) {
if (i + 2 >= len) return false;
const auto b1 = data[i + 1];
const auto b2 = data[i + 2];
if ((b1 & 0xC0) != 0x80 || (b2 & 0xC0) != 0x80) return false;
if (byte == 0xE0 && b1 < 0xA0) return false;
if (byte == 0xED && b1 >= 0xA0) return false;
i += 3;
continue;
}
if ((byte >> 3) == 0x1E) {
if (i + 3 >= len) return false;
const auto b1 = data[i + 1];
const auto b2 = data[i + 2];
const auto b3 = data[i + 3];
if ((b1 & 0xC0) != 0x80 || (b2 & 0xC0) != 0x80 || (b3 & 0xC0) != 0x80) return false;
if (byte == 0xF0 && b1 < 0x90) return false;
if (byte == 0xF4 && b1 >= 0x90) return false;
if (byte > 0xF4) return false;
i += 4;
continue;
}
return false;
}
return true;
}
template<typename Bytes>
rusty::Result<std::string_view, rusty::String> from_utf8(const Bytes& bytes) {
if constexpr (requires { bytes.data(); bytes.size(); }) {
const auto* raw = bytes.data();
const std::size_t len = static_cast<std::size_t>(bytes.size());
const auto* data = reinterpret_cast<const unsigned char*>(raw);
if (!is_valid_utf8(data, len)) {
return rusty::Result<std::string_view, rusty::String>::Err(rusty::String::from("invalid utf-8"));
}
return rusty::Result<std::string_view, rusty::String>::Ok(
std::string_view(reinterpret_cast<const char*>(raw), len)
);
}
return rusty::Result<std::string_view, rusty::String>::Err(rusty::String::from("unsupported from_utf8 input"));
}
template<typename Bytes>
std::string_view from_utf8_unchecked(Bytes&& bytes) {
if constexpr (requires { bytes.data(); bytes.size(); }) {
const auto* raw = bytes.data();
const std::size_t len = static_cast<std::size_t>(bytes.size());
return std::string_view(reinterpret_cast<const char*>(raw), len);
}
return std::string_view{};
}
template<typename Bytes>
std::string_view from_utf8_unchecked_mut(Bytes&& bytes) {
if constexpr (requires { bytes.data(); bytes.size(); }) {
const auto* raw = bytes.data();
const std::size_t len = static_cast<std::size_t>(bytes.size());
return std::string_view(reinterpret_cast<const char*>(raw), len);
}
return std::string_view{};
}
inline std::u32string decode_utf8(std::string_view text) {
std::u32string out;
const auto* data = reinterpret_cast<const unsigned char*>(text.data());
std::size_t i = 0;
while (i < text.size()) {
const unsigned char c0 = data[i];
if (c0 <= 0x7F) {
out.push_back(static_cast<char32_t>(c0));
++i;
continue;
}
if ((c0 >> 5) == 0x6 && i + 1 < text.size()) {
const unsigned char c1 = data[i + 1];
if ((c1 & 0xC0) == 0x80) {
const auto cp = (static_cast<char32_t>(c0 & 0x1F) << 6)
| static_cast<char32_t>(c1 & 0x3F);
out.push_back(cp);
i += 2;
continue;
}
}
if ((c0 >> 4) == 0xE && i + 2 < text.size()) {
const unsigned char c1 = data[i + 1];
const unsigned char c2 = data[i + 2];
if ((c1 & 0xC0) == 0x80 && (c2 & 0xC0) == 0x80) {
const auto cp = (static_cast<char32_t>(c0 & 0x0F) << 12)
| (static_cast<char32_t>(c1 & 0x3F) << 6)
| static_cast<char32_t>(c2 & 0x3F);
out.push_back(cp);
i += 3;
continue;
}
}
if ((c0 >> 3) == 0x1E && i + 3 < text.size()) {
const unsigned char c1 = data[i + 1];
const unsigned char c2 = data[i + 2];
const unsigned char c3 = data[i + 3];
if ((c1 & 0xC0) == 0x80 && (c2 & 0xC0) == 0x80 && (c3 & 0xC0) == 0x80) {
const auto cp = (static_cast<char32_t>(c0 & 0x07) << 18)
| (static_cast<char32_t>(c1 & 0x3F) << 12)
| (static_cast<char32_t>(c2 & 0x3F) << 6)
| static_cast<char32_t>(c3 & 0x3F);
out.push_back(cp);
i += 4;
continue;
}
}
out.push_back(static_cast<char32_t>(0xFFFD));
++i;
}
return out;
}
struct Chars {
using Item = char32_t;
std::u32string decoded;
std::size_t index = 0;

rusty::Option<char32_t> next() {
if (index >= decoded.size()) {
return rusty::Option<char32_t>(rusty::None);
}
return rusty::Option<char32_t>(decoded[index++]);
}

Chars rev() const {
Chars out = *this;
std::reverse(out.decoded.begin(), out.decoded.end());
out.index = 0;
return out;
}

std::size_t count() const {
return decoded.size() - std::min(index, decoded.size());
}
};
inline Chars chars(std::string_view text) {
return Chars{decode_utf8(text), 0};
}
struct CharIndices {
using Item = std::tuple<std::size_t, char32_t>;
Chars iter;
std::size_t index = 0;

rusty::Option<Item> next() {
auto next_ch = iter.next();
if (next_ch.is_none()) {
return rusty::Option<Item>(rusty::None);
}
auto value = std::make_tuple(index++, next_ch.unwrap());
return rusty::Option<Item>(std::move(value));
}
};
inline CharIndices char_indices(std::string_view text) {
return CharIndices{chars(text), 0};
}
struct Bytes {
using Item = uint8_t;
std::string bytes;
std::size_t index = 0;

rusty::Option<uint8_t> next() {
if (index >= bytes.size()) {
return rusty::Option<uint8_t>(rusty::None);
}
return rusty::Option<uint8_t>(static_cast<uint8_t>(bytes[index++]));
}

Bytes rev() const {
Bytes out = *this;
std::reverse(out.bytes.begin(), out.bytes.end());
out.index = 0;
return out;
}

std::size_t count() const {
return bytes.size() - std::min(index, bytes.size());
}
};
inline Bytes bytes(std::string_view text) {
return Bytes{std::string(text), 0};
}
template<typename S>
auto chars(const S& value) {
if constexpr (requires { value.chars(); }) {
return value.chars();
} else if constexpr (requires { value.as_str(); }) {
return chars(std::string_view(value.as_str()));
} else if constexpr (std::is_convertible_v<S, std::string_view>) {
return chars(std::string_view(value));
} else {
return Chars{};
}
}
inline bool is_char_boundary(std::string_view text, std::size_t idx) {
if (idx > text.size()) return false;
if (idx == 0 || idx == text.size()) return true;
const auto c = static_cast<unsigned char>(text[idx]);
return (c & 0xC0) != 0x80;
}
template<typename S, typename I>
bool is_char_boundary(const S& value, I idx) {
const auto uidx = static_cast<std::size_t>(idx);
if constexpr (requires { value.is_char_boundary(uidx); }) {
return value.is_char_boundary(uidx);
} else if constexpr (requires { value.as_str(); }) {
return is_char_boundary(std::string_view(value.as_str()), uidx);
} else if constexpr (std::is_convertible_v<S, std::string_view>) {
return is_char_boundary(std::string_view(value), uidx);
} else {
return false;
}
}
template<typename T, typename Input>
rusty::Result<T, rusty::String> parse(const Input& input) {
std::string_view text;
if constexpr (std::is_convertible_v<Input, std::string_view>) {
text = std::string_view(input);
} else if constexpr (requires { input.as_str(); }) {
text = std::string_view(input.as_str());
} else {
return rusty::Result<T, rusty::String>::Err(rusty::String::from("unsupported parse input"));
}
if constexpr (std::is_integral_v<T> && !std::is_same_v<T, bool>) {
T value{};
const auto* begin = text.data();
const auto* end = begin + text.size();
const auto [ptr, ec] = std::from_chars(begin, end, value);
if (ec == std::errc() && ptr == end) {
return rusty::Result<T, rusty::String>::Ok(value);
}
return rusty::Result<T, rusty::String>::Err(rusty::String::from("invalid digit found in string"));
}
return rusty::Result<T, rusty::String>::Err(rusty::String::from("unsupported parse target"));
}
inline std::string_view trim(std::string_view s) {
auto start = s.find_first_not_of(" \t\n\r");
if (start == std::string_view::npos) return {};
auto end = s.find_last_not_of(" \t\n\r");
return s.substr(start, end - start + 1);
}
inline std::string_view trim_start_matches(std::string_view s, char32_t ch) {
size_t start = 0;
while (start < s.size() && static_cast<char32_t>(static_cast<unsigned char>(s[start])) == ch) ++start;
return s.substr(start);
}
inline std::string_view trim_end_matches(std::string_view s, char32_t ch) {
size_t end = s.size();
while (end > 0 && static_cast<char32_t>(static_cast<unsigned char>(s[end - 1])) == ch) --end;
return s.substr(0, end);
}
inline rusty::Option<std::string_view> strip_prefix(std::string_view s, std::string_view prefix) {
if (s.starts_with(prefix)) {
return rusty::Option<std::string_view>(s.substr(prefix.size()));
}
return rusty::Option<std::string_view>(rusty::None);
}
inline rusty::Option<std::string_view> strip_prefix(std::string_view s, char32_t ch) {
if (!s.empty() && static_cast<char32_t>(static_cast<unsigned char>(s[0])) == ch) {
return rusty::Option<std::string_view>(s.substr(1));
}
return rusty::Option<std::string_view>(rusty::None);
}
template<std::size_t N>
inline rusty::Option<std::string_view> strip_prefix(std::string_view s, const std::array<char32_t, N>& any_prefix) {
if (s.empty()) {
return rusty::Option<std::string_view>(rusty::None);
}
const auto front = static_cast<char32_t>(static_cast<unsigned char>(s[0]));
for (const auto ch : any_prefix) {
if (front == ch) {
return rusty::Option<std::string_view>(s.substr(1));
}
}
return rusty::Option<std::string_view>(rusty::None);
}
inline rusty::Option<std::size_t> find(std::string_view s, std::string_view needle) {
const auto pos = s.find(needle);
if (pos == std::string_view::npos) {
return rusty::Option<std::size_t>(rusty::None);
}
return rusty::Option<std::size_t>(pos);
}
inline rusty::Option<std::size_t> find(std::string_view s, char32_t ch) {
const auto pos = s.find(static_cast<char>(ch));
if (pos == std::string_view::npos) {
return rusty::Option<std::size_t>(rusty::None);
}
return rusty::Option<std::size_t>(pos);
}
template<std::size_t N>
inline rusty::Option<std::size_t> find(std::string_view s, const std::array<char32_t, N>& any_char) {
for (std::size_t i = 0; i < s.size(); ++i) {
const auto cur = static_cast<char32_t>(static_cast<unsigned char>(s[i]));
for (const auto ch : any_char) {
if (cur == ch) {
return rusty::Option<std::size_t>(i);
}
}
}
return rusty::Option<std::size_t>(rusty::None);
}
struct SplitIter {
std::string_view remaining;
char32_t delim;
bool done = false;
rusty::Option<std::string_view> next() {
if (done) return rusty::Option<std::string_view>(rusty::None);
auto pos = remaining.find(static_cast<char>(delim));
if (pos == std::string_view::npos) {
done = true;
return rusty::Option<std::string_view>(remaining);
}
auto piece = remaining.substr(0, pos);
remaining = remaining.substr(pos + 1);
return rusty::Option<std::string_view>(piece);
}
rusty::Option<std::string_view> nth(std::size_t n) {
for (std::size_t i = 0; i < n; ++i) {
if (next().is_none()) {
return rusty::Option<std::string_view>(rusty::None);
}
}
return next();
}
};
inline SplitIter split(std::string_view s, char32_t delim) {
return SplitIter{s, delim};
}
template<typename S>
inline SplitIter split(const S& value, char32_t delim) {
if constexpr (std::is_convertible_v<S, std::string_view>) {
return split(std::string_view(value), delim);
} else if constexpr (requires { value.as_str(); }) {
return split(std::string_view(value.as_str()), delim);
} else if constexpr (
requires { *value; }
&& (
std::is_convertible_v<decltype(*value), std::string_view>
|| requires { (*value).as_str(); }
)
) {
return split(*value, delim);
} else {
return SplitIter{std::string_view{}, delim, true};
}
}
}
namespace char_runtime {
inline rusty::Option<char32_t> from_u32(uint32_t value) {
if (value > 0x10FFFF || (value >= 0xD800 && value <= 0xDFFF)) {
return rusty::Option<char32_t>(rusty::None);
}
return rusty::Option<char32_t>(static_cast<char32_t>(value));
}
inline std::size_t len_utf8(char32_t ch) {
const auto code = static_cast<uint32_t>(ch);
if (code < 0x80) return 1;
if (code < 0x800) return 2;
if (code < 0x10000) return 3;
return 4;
}
template<typename Buffer>
inline std::string_view encode_utf8(char32_t ch, Buffer& buffer) {
using std::data;
using std::size;
const auto needed = len_utf8(ch);
if (size(buffer) < needed) {
throw std::runtime_error("encode_utf8 buffer too small");
}
auto* out = data(buffer);
const auto code = static_cast<uint32_t>(ch);
if (needed == 1) {
out[0] = static_cast<std::remove_reference_t<decltype(out[0])>>(code);
} else if (needed == 2) {
out[0] = static_cast<std::remove_reference_t<decltype(out[0])>>(0xC0u | (code >> 6));
out[1] = static_cast<std::remove_reference_t<decltype(out[1])>>(0x80u | (code & 0x3Fu));
} else if (needed == 3) {
out[0] = static_cast<std::remove_reference_t<decltype(out[0])>>(0xE0u | (code >> 12));
out[1] = static_cast<std::remove_reference_t<decltype(out[1])>>(0x80u | ((code >> 6) & 0x3Fu));
out[2] = static_cast<std::remove_reference_t<decltype(out[2])>>(0x80u | (code & 0x3Fu));
} else {
out[0] = static_cast<std::remove_reference_t<decltype(out[0])>>(0xF0u | (code >> 18));
out[1] = static_cast<std::remove_reference_t<decltype(out[1])>>(0x80u | ((code >> 12) & 0x3Fu));
out[2] = static_cast<std::remove_reference_t<decltype(out[2])>>(0x80u | ((code >> 6) & 0x3Fu));
out[3] = static_cast<std::remove_reference_t<decltype(out[3])>>(0x80u | (code & 0x3Fu));
}
return std::string_view(reinterpret_cast<const char*>(out), needed);
}
inline bool is_whitespace(char32_t ch) {
const auto code = static_cast<uint32_t>(ch);
if (code == 0x0009 || code == 0x000A || code == 0x000B || code == 0x000C || code == 0x000D || code == 0x0020) {
return true;
}
if (code == 0x0085 || code == 0x00A0 || code == 0x1680 || code == 0x2028 || code == 0x2029 || code == 0x202F || code == 0x205F || code == 0x3000) {
return true;
}
return code >= 0x2000 && code <= 0x200A;
}
}
template<typename T>
bool is_empty(const T& value) {
if constexpr (requires { value.is_empty(); }) {
return value.is_empty();
} else if constexpr (requires { value.empty(); }) {
return value.empty();
} else {
return false;
}
}
template<typename T>
auto deref_ref(const T& value) {
if constexpr (requires { value.as_str(); }) {
return value.as_str();
} else if constexpr (requires { *value; }) {
return *value;
} else {
return value;
}
}
template<typename T>
decltype(auto) deref_mut(T& value) {
if constexpr (requires { *value; }) {
return *value;
} else {
return (value);
}
}
namespace panicking {
enum class AssertKind { Eq, Ne };
template<typename... Args>
[[noreturn]] inline void assert_failed(Args&&...) { throw std::runtime_error("assertion failed"); }
template<typename... Args>
[[noreturn]] inline void panic(Args&&...) { throw std::runtime_error("panic"); }
template<typename... Args>
[[noreturn]] inline void panic_fmt(Args&&...) { throw std::runtime_error("panic"); }
template<typename... Args>
[[noreturn]] inline void unreachable_display(Args&&...) { throw std::runtime_error("unreachable"); }
}
namespace intrinsics {
struct Discriminant {
std::size_t value;
bool operator==(const Discriminant&) const = default;
rusty::cmp::Ordering cmp(const Discriminant& other) const {
if (value < other.value) return rusty::cmp::Ordering::Less;
if (value > other.value) return rusty::cmp::Ordering::Greater;
return rusty::cmp::Ordering::Equal;
}
Option<rusty::cmp::Ordering> partial_cmp(const Discriminant& other) const {
return Option<rusty::cmp::Ordering>(cmp(other));
}
template<typename State>
void hash(State& state) const {
rusty::hash::hash(value, state);
}
};
template<typename V>
Discriminant discriminant_value(const V& value) {
using RawV = std::remove_cv_t<std::remove_reference_t<V>>;
if constexpr (requires { value.index(); }) {
return Discriminant{static_cast<std::size_t>(value.index())};
} else if constexpr (std::is_enum_v<RawV>) {
using Underlying = std::underlying_type_t<RawV>;
return Discriminant{static_cast<std::size_t>(static_cast<Underlying>(value))};
} else if constexpr (std::is_integral_v<RawV>) {
return Discriminant{static_cast<std::size_t>(value)};
} else {
static_assert(!std::is_same_v<RawV, RawV>, "unsupported discriminant_value input type");
}
}
[[noreturn]] inline void unreachable() { throw std::runtime_error("unreachable"); }
}
}

namespace core {
namespace cmp {
using Ordering = ::rusty::cmp::Ordering;
template<typename A, typename B>
constexpr auto min(A&& a, B&& b) {
return (std::forward<B>(b) < std::forward<A>(a)) ? std::forward<B>(b) : std::forward<A>(a);
}
template<typename A, typename B>
constexpr auto max(A&& a, B&& b) {
return (std::forward<A>(a) < std::forward<B>(b)) ? std::forward<B>(b) : std::forward<A>(a);
}
struct PartialOrd {
template<typename A, typename B>
static auto partial_cmp(A&& a, B&& b) {
return std::forward<A>(a).partial_cmp(std::forward<B>(b));
}
};
struct Ord {
template<typename A, typename B>
static auto cmp(A&& a, B&& b) {
return std::forward<A>(a).cmp(std::forward<B>(b));
}
};
}
}


namespace public_ {
}
namespace internal {
}
namespace external {
    namespace __private {
    }
}
namespace parser {
    struct ParseErrorKind;
    struct ParseError;
    template<typename B>
    rusty::Result<std::tuple<>, rusty::fmt::Error> to_writer(const B& flags, auto&& writer);
    template<typename B>
    rusty::Result<B, ParseError> from_str(std::string_view input);
    template<typename B>
    rusty::Result<std::tuple<>, rusty::fmt::Error> to_writer_truncate(const B& flags, auto&& writer);
    template<typename B>
    rusty::Result<B, ParseError> from_str_truncate(std::string_view input);
    template<typename B>
    rusty::Result<std::tuple<>, rusty::fmt::Error> to_writer_strict(const B& flags, auto&& writer);
    template<typename B>
    rusty::Result<B, ParseError> from_str_strict(std::string_view input);
}
namespace iter {
    template<typename B>
    struct IterNames;
    template<typename B>
    struct Iter;
    using traits::Flag;
}
namespace traits {
    template<typename B>
    struct Flag;
    namespace __private {
    }
    using parser::ParseError;
}
namespace tests {
    struct TestFlags;
    struct TestFlagsInvert;
    struct TestZero;
    struct TestZeroOne;
    struct TestUnicode;
    struct TestEmpty;
    struct TestOverlapping;
    struct TestOverlappingFull;
    struct TestExternal;
    struct TestExternalFull;
    namespace flags {
        namespace external {
            void cases();
        }
        void cases();
    }
    namespace eq {
        void cases();
    }
    namespace parser {
        namespace from_str_tests {
            void valid();
            void invalid();
        }
        namespace to_writer_tests {
            void cases();
            template<typename F>
            rusty::String write_(F value);
        }
        namespace from_str_truncate_tests {
            void valid();
        }
        namespace to_writer_truncate_tests {
            void cases();
            template<typename F>
            rusty::String write_(F value);
        }
        namespace from_str_strict_tests {
            void valid();
            void invalid();
        }
        namespace to_writer_strict_tests {
            void cases();
            template<typename F>
            rusty::String write_(F value);
        }
        void roundtrip();
        void roundtrip_truncate();
        void roundtrip_strict();
    }
    namespace extend {
        namespace external {
            void cases();
        }
        void cases();
    }
    namespace fmt {
        void cases();
        template<typename T>
        void case_(T value, std::string_view debug, std::string_view uhex, std::string_view lhex, std::string_view oct, std::string_view bin);
    }
    namespace all {
        void cases();
        template<typename T>
        void case_(typename T::Bits expected, const auto& inherent);
    }
    namespace empty {
        void cases();
        template<typename T>
        void case_(typename T::Bits expected, const auto& inherent);
    }
    namespace from_bits {
        void cases();
        template<typename T>
        void case_(rusty::Option<typename T::Bits> expected, typename T::Bits input, const auto& inherent);
    }
    namespace from_bits_retain {
        void cases();
        template<typename T>
        void case_(typename T::Bits input, const auto& inherent);
    }
    namespace from_bits_truncate {
        void cases();
        template<typename T>
        void case_(typename T::Bits expected, typename T::Bits input, const auto& inherent);
    }
    namespace from_name {
        void cases();
        template<typename T>
        void case_(rusty::Option<typename T::Bits> expected, std::string_view input, const auto& inherent);
    }
    namespace union_ {
        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent);
    }
    namespace symmetric_difference {
        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent_sym_diff, const auto& inherent_toggle);
    }
    namespace remove {
        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent_remove, const auto& inherent_set);
    }
    namespace intersection {
        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent);
    }
    namespace insert {
        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent_insert, const auto& inherent_set);
    }
    namespace is_empty {
        void cases();
        template<typename T>
        void case_(bool expected, T value, const auto& inherent);
    }
    namespace is_all {
        void cases();
        template<typename T>
        void case_(bool expected, T value, const auto& inherent);
    }
    namespace intersects {
        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, bool>> inputs, const auto& inherent);
    }
    namespace iter {
        namespace collect {
            void cases();
        }
        namespace iter {
            void cases();
            template<typename T>
            void case_(std::span<const typename T::Bits> expected, T value, const auto& inherent);
        }
        namespace iter_names {
            void cases();
            template<typename T>
            void case_(std::span<const std::tuple<std::string_view, typename T::Bits>> expected, T value, const auto& inherent);
        }
        void roundtrip();
    }
    namespace difference {
        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent);
    }
    namespace contains {
        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, bool>> inputs, const auto& inherent);
    }
    namespace complement {
        void cases();
        template<typename T>
        void case_(typename T::Bits expected, T value, const auto& inherent);
    }
    namespace bits {
        void cases();
        template<typename T>
        void case_(typename T::Bits expected, T value, const auto& inherent);
    }
}
namespace __private {
}
using traits::Flag;

namespace traits {
    // Extension trait free-function forward declarations
    namespace rusty_ext {
        template<typename W>
        rusty::fmt::Result write_hex(const uint8_t& self_, W writer);

        template<typename W>
        rusty::fmt::Result write_hex(const int8_t& self_, W writer);

        template<typename W>
        rusty::fmt::Result write_hex(const uint16_t& self_, W writer);

        template<typename W>
        rusty::fmt::Result write_hex(const int16_t& self_, W writer);

        template<typename W>
        rusty::fmt::Result write_hex(const uint32_t& self_, W writer);

        template<typename W>
        rusty::fmt::Result write_hex(const int32_t& self_, W writer);

        template<typename W>
        rusty::fmt::Result write_hex(const uint64_t& self_, W writer);

        template<typename W>
        rusty::fmt::Result write_hex(const int64_t& self_, W writer);

        template<typename W>
        rusty::fmt::Result write_hex(const unsigned __int128& self_, W writer);

        template<typename W>
        rusty::fmt::Result write_hex(const __int128& self_, W writer);

    }

}



using traits::Flag;

namespace parser {

    struct ParseErrorKind;
    struct ParseError;
    template<typename B>
    rusty::Result<std::tuple<>, rusty::fmt::Error> to_writer(const B& flags, auto&& writer);
    template<typename B>
    rusty::Result<B, ParseError> from_str(std::string_view input);
    template<typename B>
    rusty::Result<std::tuple<>, rusty::fmt::Error> to_writer_truncate(const B& flags, auto&& writer);
    template<typename B>
    rusty::Result<B, ParseError> from_str_truncate(std::string_view input);
    template<typename B>
    rusty::Result<std::tuple<>, rusty::fmt::Error> to_writer_strict(const B& flags, auto&& writer);
    template<typename B>
    rusty::Result<B, ParseError> from_str_strict(std::string_view input);

    namespace fmt = rusty::fmt;


    // Rust-only trait WriteHex (Proxy facade emission skipped in module mode)

    // Rust-only trait ParseHex (Proxy facade emission skipped in module mode)

    // Algebraic data type
    struct ParseErrorKind_EmptyFlag {};
    struct ParseErrorKind_InvalidNamedFlag {
        std::tuple<> got;
    };
    struct ParseErrorKind_InvalidHexFlag {
        std::tuple<> got;
    };
    ParseErrorKind_EmptyFlag EmptyFlag();
    ParseErrorKind_InvalidNamedFlag InvalidNamedFlag(std::tuple<> got);
    ParseErrorKind_InvalidHexFlag InvalidHexFlag(std::tuple<> got);
    struct ParseErrorKind : std::variant<ParseErrorKind_EmptyFlag, ParseErrorKind_InvalidNamedFlag, ParseErrorKind_InvalidHexFlag> {
        using variant = std::variant<ParseErrorKind_EmptyFlag, ParseErrorKind_InvalidNamedFlag, ParseErrorKind_InvalidHexFlag>;
        using variant::variant;
        static ParseErrorKind EmptyFlag() { return ParseErrorKind{ParseErrorKind_EmptyFlag{}}; }
        static ParseErrorKind InvalidNamedFlag(std::tuple<> got) { return ParseErrorKind{ParseErrorKind_InvalidNamedFlag{.got = std::forward<decltype(got)>(got)}}; }
        static ParseErrorKind InvalidHexFlag(std::tuple<> got) { return ParseErrorKind{ParseErrorKind_InvalidHexFlag{.got = std::forward<decltype(got)>(got)}}; }


        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
    };
    ParseErrorKind_EmptyFlag EmptyFlag() { return ParseErrorKind_EmptyFlag{};  }
    ParseErrorKind_InvalidNamedFlag InvalidNamedFlag(std::tuple<> got) { return ParseErrorKind_InvalidNamedFlag{.got = std::forward<std::tuple<>>(got)};  }
    ParseErrorKind_InvalidHexFlag InvalidHexFlag(std::tuple<> got) { return ParseErrorKind_InvalidHexFlag{.got = std::forward<std::tuple<>>(got)};  }

    /// An error encountered while parsing flags from text.
    struct ParseError {
        ParseErrorKind _0;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
        static ParseError invalid_hex_flag(const auto& flag);
        static ParseError invalid_named_flag(const auto& flag);
        static ParseError empty_flag();
    };

}

namespace public_ {
}

namespace internal {
}

namespace external {
    namespace __private {}

    namespace __private {
    }

    namespace __private {
    }

}

namespace external {}
using namespace ::external;


namespace iter {

    template<typename B>
    struct IterNames;
    template<typename B>
    struct Iter;
    using traits::Flag;

    using traits::Flag;

    ///
    ///An iterator over flags values.
    ///
    ///This iterator only yields flags values for contained, defined, named flags. Any remaining bits
    ///won't be yielded, but can be found with the [`IterNames::remaining`] method.
    ///
    template<typename B>
    struct IterNames {
        using Item = std::tuple<std::string_view, B>;
        std::span<const ::traits::Flag<B>> flags;
        size_t idx;
        B source;
        B remaining_field;

        static IterNames<B> new_(const B& flags) {
            return IterNames<B>{.flags = rusty::clone(rusty::clone(B::FLAGS)), .idx = static_cast<size_t>(0), .source = B::from_bits_retain(flags.bits()), .remaining_field = B::from_bits_retain(flags.bits())};
        }
        static IterNames<B> __private_const_new(std::span<const ::traits::Flag<B>> flags, B source, B remaining) {
            return IterNames<B>{.flags = flags, .idx = static_cast<size_t>(0), .source = std::move(source), .remaining_field = std::move(remaining)};
        }
        const B& remaining() const {
            return this->remaining_field;
        }
        rusty::Option<Item> next() {
            while (true) {
                auto&& _whilelet = rusty::get(this->flags, this->idx);
                if (!(_whilelet.is_some())) { break; }
                auto flag = _whilelet.unwrap();
                if (rusty::is_empty(this->remaining_field)) {
                    return rusty::Option<std::tuple<std::string_view, B>>(rusty::None);
                }
                [&]() { static_cast<void>(this->idx += 1); return std::make_tuple(); }();
                if (rusty::is_empty(flag.name())) {
                    continue;
                }
                auto bits = flag.value().bits();
                if (rusty::contains(this->source, B::from_bits_retain(std::move(bits))) && this->remaining_field.intersects(B::from_bits_retain(std::move(bits)))) {
                    this->remaining_field.remove(B::from_bits_retain(std::move(bits)));
                    return rusty::Option<std::tuple<std::string_view, B>>(std::make_tuple(flag.name(), B::from_bits_retain(std::move(bits))));
                }
            }
            return rusty::Option<std::tuple<std::string_view, B>>(rusty::None);
        }
    };

    ///
    ///An iterator over flags values.
    ///
    ///This iterator will yield flags values for contained, defined flags first, with any remaining bits yielded
    ///as a final flags value.
    ///
    template<typename B>
    struct Iter {
        using Item = B;
        IterNames<B> inner;
        bool done;

        static Iter<B> new_(const B& flags) {
            return Iter<B>{.inner = IterNames<B>::new_(flags), .done = false};
        }
        static Iter<B> __private_const_new(std::span<const ::traits::Flag<B>> flags, B source, B remaining) {
            return Iter<B>{.inner = IterNames<B>::__private_const_new(flags, std::move(source), std::move(remaining)), .done = false};
        }
        rusty::Option<Item> next() {
            return [&]() -> rusty::Option<Item> { auto&& _m = this->inner.next(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& flag = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_mv0))); return rusty::Option<B>(std::move(flag)); } if (_m.is_none()) { if (!this->done) { return [&]() -> rusty::Option<Item> { this->done = true;
return (!rusty::is_empty(this->inner.remaining()) ? rusty::Option<B>(B::from_bits_retain(this->inner.remaining_field.bits())) : rusty::Option<B>(rusty::None)); }(); } } if (_m.is_none()) { return rusty::Option<B>(rusty::None); } return [&]() -> rusty::Option<Item> { rusty::intrinsics::unreachable(); }(); }();
        }
    };

}

namespace traits {
    namespace __private {}

    template<typename B>
    struct Flag;
    namespace __private {
    }
    using parser::ParseError;

    namespace iter = ::iter;
    using parser::ParseError;

    namespace fmt = rusty::fmt;

    ///
    ///A defined flags value that may be named or unnamed.
    ///
    template<typename B>
    struct Flag {
        std::string_view name_field;
        B value_field;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, B>::debug_struct_field2_finish(f, "Flag", "name", &this->name_field, "value", &this->value_field);
        }
        static Flag<B> new_(std::string_view name, B value) {
            return Flag<B>{.name_field = std::string_view(name), .value_field = std::move(value)};
        }
        std::string_view name() const {
            return std::string_view(this->name_field);
        }
        const B& value() const {
            return this->value_field;
        }
        bool is_named() const {
            return !rusty::is_empty(this->name_field);
        }
        bool is_unnamed() const {
            return rusty::is_empty(this->name_field);
        }
    };

    // Module-mode trait fallback for default methods on Flags
    struct FlagsRuntimeHelper {
        // Rust-only trait default method skipped (no receiver): empty
        // Rust-only trait default method skipped (no receiver): all
        // Rust-only trait default method skipped (no receiver): from_bits
        // Rust-only trait default method skipped (no receiver): from_bits_truncate
        // Rust-only trait default method skipped (no receiver): from_name
        static auto iter(const auto& self_) -> ::iter::Iter<std::remove_reference_t<decltype(self_)>> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            return ::iter::Iter<Self_>::new_(self_);
        }
        static auto iter_names(const auto& self_) -> ::iter::IterNames<std::remove_reference_t<decltype(self_)>> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            return ::iter::IterNames<Self_>::new_(self_);
        }
        static auto is_empty(const auto& self_) -> bool {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            return self_.bits() == rusty::clone(0);
        }
        static auto is_all(const auto& self_) -> bool {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            return (Self_::all().bits() | self_.bits()) == self_.bits();
        }
        static auto intersects(const auto& self_, auto other) -> bool {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            return (self_.bits() & other.bits()) != rusty::clone(0);
        }
        static auto contains(const auto& self_, auto other) -> bool {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            return (self_.bits() & other.bits()) == other.bits();
        }
        static auto insert(auto& self_, auto other) -> void {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            self_ = Self_::from_bits_retain(self_.bits()).union_(std::move(other));
        }
        static auto remove(auto& self_, auto other) -> void {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            self_ = Self_::from_bits_retain(self_.bits()).difference(std::move(other));
        }
        static auto toggle(auto& self_, auto other) -> void {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            self_ = Self_::from_bits_retain(self_.bits()).symmetric_difference(std::move(other));
        }
        static auto set(auto& self_, auto other, auto value) -> void {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            if (value) {
                self_.insert(std::move(other));
            } else {
                self_.remove(std::move(other));
            }
        }
        static auto intersection(auto self_, auto other) -> std::remove_reference_t<decltype(self_)> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            return Self_::from_bits_retain(self_.bits() & other.bits());
        }
        static auto union_(auto self_, auto other) -> std::remove_reference_t<decltype(self_)> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            return Self_::from_bits_retain(self_.bits() | other.bits());
        }
        static auto difference(auto self_, auto other) -> std::remove_reference_t<decltype(self_)> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            return Self_::from_bits_retain(self_.bits() & ~other.bits());
        }
        static auto symmetric_difference(auto self_, auto other) -> std::remove_reference_t<decltype(self_)> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            return Self_::from_bits_retain(self_.bits() ^ other.bits());
        }
        static auto complement(auto self_) -> std::remove_reference_t<decltype(self_)> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            return Self_::from_bits_truncate(~self_.bits());
        }
    };



    // Rust-only trait PublicFlags (Proxy facade emission skipped in module mode)



    namespace __private {


    }

    // Extension trait BitFlags lowered to rusty_ext:: free functions
    namespace rusty_ext {
    }

    // Extension trait Bits lowered to rusty_ext:: free functions
    namespace rusty_ext {
    }

    // Extension trait ImplementedByBitFlagsMacro lowered to rusty_ext:: free functions
    namespace rusty_ext {
    }

    // Extension trait ParseHex lowered to rusty_ext:: free functions
    namespace rusty_ext {
        // Rust-only extension method skipped (no receiver): parse_hex

        // Rust-only extension method skipped (no receiver): parse_hex

        // Rust-only extension method skipped (no receiver): parse_hex

        // Rust-only extension method skipped (no receiver): parse_hex

        // Rust-only extension method skipped (no receiver): parse_hex

        // Rust-only extension method skipped (no receiver): parse_hex

        // Rust-only extension method skipped (no receiver): parse_hex

        // Rust-only extension method skipped (no receiver): parse_hex

        // Rust-only extension method skipped (no receiver): parse_hex

        // Rust-only extension method skipped (no receiver): parse_hex

        // Rust-only extension method skipped (no receiver): parse_hex

        // Rust-only extension method skipped (no receiver): parse_hex

    }

    // Extension trait Primitive lowered to rusty_ext:: free functions
    namespace rusty_ext {
    }

    // Extension trait WriteHex lowered to rusty_ext:: free functions
    namespace rusty_ext {
        template<typename W>
        rusty::fmt::Result write_hex(const uint8_t& self_, W writer) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return rusty::write_fmt(writer, std::format("{0:x}", self_));
        }

        template<typename W>
        rusty::fmt::Result write_hex(const int8_t& self_, W writer) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return rusty::write_fmt(writer, std::format("{0:x}", self_));
        }

        template<typename W>
        rusty::fmt::Result write_hex(const uint16_t& self_, W writer) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return rusty::write_fmt(writer, std::format("{0:x}", self_));
        }

        template<typename W>
        rusty::fmt::Result write_hex(const int16_t& self_, W writer) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return rusty::write_fmt(writer, std::format("{0:x}", self_));
        }

        template<typename W>
        rusty::fmt::Result write_hex(const uint32_t& self_, W writer) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return rusty::write_fmt(writer, std::format("{0:x}", self_));
        }

        template<typename W>
        rusty::fmt::Result write_hex(const int32_t& self_, W writer) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return rusty::write_fmt(writer, std::format("{0:x}", self_));
        }

        template<typename W>
        rusty::fmt::Result write_hex(const uint64_t& self_, W writer) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return rusty::write_fmt(writer, std::format("{0:x}", self_));
        }

        template<typename W>
        rusty::fmt::Result write_hex(const int64_t& self_, W writer) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return rusty::write_fmt(writer, std::format("{0:x}", self_));
        }

        template<typename W>
        rusty::fmt::Result write_hex(const unsigned __int128& self_, W writer) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return rusty::write_fmt(writer, std::format("{0:x}", self_));
        }

        template<typename W>
        rusty::fmt::Result write_hex(const __int128& self_, W writer) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return rusty::write_fmt(writer, std::format("{0:x}", self_));
        }

    }


}

namespace __private {

    using namespace ::external::__private;
    using namespace ::traits::__private;

    // Rust-only unresolved import: using core;

}

namespace tests {
    namespace all {}
    namespace bits {}
    namespace complement {}
    namespace contains {}
    namespace difference {}
    namespace empty {}
    namespace eq {}
    namespace extend {}
    namespace flags {}
    namespace fmt {}
    namespace from_bits {}
    namespace from_bits_retain {}
    namespace from_bits_truncate {}
    namespace from_name {}
    namespace insert {}
    namespace intersection {}
    namespace intersects {}
    namespace is_all {}
    namespace is_empty {}
    namespace iter {}
    namespace parser {}
    namespace remove {}
    namespace symmetric_difference {}
    namespace union_ {}

    struct TestFlags;
    struct TestFlagsInvert;
    struct TestZero;
    struct TestZeroOne;
    struct TestUnicode;
    struct TestEmpty;
    struct TestOverlapping;
    struct TestOverlappingFull;
    struct TestExternal;
    struct TestExternalFull;
    namespace flags {
        namespace external {
            void cases();
        }
        void cases();
    }
    namespace eq {
        void cases();
    }
    namespace parser {
        namespace from_str_tests {
            void valid();
            void invalid();
        }
        namespace to_writer_tests {
            void cases();
            template<typename F>
            rusty::String write_(F value);
        }
        namespace from_str_truncate_tests {
            void valid();
        }
        namespace to_writer_truncate_tests {
            void cases();
            template<typename F>
            rusty::String write_(F value);
        }
        namespace from_str_strict_tests {
            void valid();
            void invalid();
        }
        namespace to_writer_strict_tests {
            void cases();
            template<typename F>
            rusty::String write_(F value);
        }
        void roundtrip();
        void roundtrip_truncate();
        void roundtrip_strict();
    }
    namespace extend {
        namespace external {
            void cases();
        }
        void cases();
    }
    namespace fmt {
        void cases();
        template<typename T>
        void case_(T value, std::string_view debug, std::string_view uhex, std::string_view lhex, std::string_view oct, std::string_view bin);
    }
    namespace all {
        void cases();
        template<typename T>
        void case_(typename T::Bits expected, const auto& inherent);
    }
    namespace empty {
        void cases();
        template<typename T>
        void case_(typename T::Bits expected, const auto& inherent);
    }
    namespace from_bits {
        void cases();
        template<typename T>
        void case_(rusty::Option<typename T::Bits> expected, typename T::Bits input, const auto& inherent);
    }
    namespace from_bits_retain {
        void cases();
        template<typename T>
        void case_(typename T::Bits input, const auto& inherent);
    }
    namespace from_bits_truncate {
        void cases();
        template<typename T>
        void case_(typename T::Bits expected, typename T::Bits input, const auto& inherent);
    }
    namespace from_name {
        void cases();
        template<typename T>
        void case_(rusty::Option<typename T::Bits> expected, std::string_view input, const auto& inherent);
    }
    namespace union_ {
        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent);
    }
    namespace symmetric_difference {
        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent_sym_diff, const auto& inherent_toggle);
    }
    namespace remove {
        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent_remove, const auto& inherent_set);
    }
    namespace intersection {
        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent);
    }
    namespace insert {
        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent_insert, const auto& inherent_set);
    }
    namespace is_empty {
        void cases();
        template<typename T>
        void case_(bool expected, T value, const auto& inherent);
    }
    namespace is_all {
        void cases();
        template<typename T>
        void case_(bool expected, T value, const auto& inherent);
    }
    namespace intersects {
        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, bool>> inputs, const auto& inherent);
    }
    namespace iter {
        namespace collect {
            void cases();
        }
        namespace iter {
            void cases();
            template<typename T>
            void case_(std::span<const typename T::Bits> expected, T value, const auto& inherent);
        }
        namespace iter_names {
            void cases();
            template<typename T>
            void case_(std::span<const std::tuple<std::string_view, typename T::Bits>> expected, T value, const auto& inherent);
        }
        void roundtrip();
    }
    namespace difference {
        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent);
    }
    namespace contains {
        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, bool>> inputs, const auto& inherent);
    }
    namespace complement {
        void cases();
        template<typename T>
        void case_(typename T::Bits expected, T value, const auto& inherent);
    }
    namespace bits {
        void cases();
        template<typename T>
        void case_(typename T::Bits expected, T value, const auto& inherent);
    }

    struct TestFlags {
        using Bits = uint8_t;
        using Primitive = uint8_t;
        using Internal = uint8_t;
        using Item = TestFlags;
        using IntoIter = ::iter::Iter<TestFlags>;
        TestFlags::Internal _0;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
        bool operator==(const TestFlags& other) const;
        void assert_receiver_is_total_eq() const;
        std::partial_ordering operator<=>(const TestFlags& other) const;
        rusty::cmp::Ordering cmp(const TestFlags& other) const;
        TestFlags clone() const;
        static const TestFlags A;
        static const TestFlags B;
        static const TestFlags C;
        static const std::span<const ::traits::Flag<TestFlags>> FLAGS;
        static TestFlags empty();
        static TestFlags all();
        static rusty::Option<TestFlags> from_bits(uint8_t bits);
        static TestFlags from_bits_truncate(uint8_t bits);
        static rusty::Option<TestFlags> from_name(std::string_view name);
        TestFlags operator|(const auto& other) const;
        void operator|=(TestFlags other);
        TestFlags operator^(TestFlags other) const;
        void operator^=(TestFlags other);
        TestFlags operator&(TestFlags other) const;
        void operator&=(TestFlags other);
        TestFlags operator-(TestFlags other) const;
        void operator-=(TestFlags other);
        TestFlags operator!() const;
        static const TestFlags ABC;

        // Synthetic bitwise trait methods (from const _ block impls)
        TestFlags::Internal bits() const { return this->_0; }
        static TestFlags from_bits_retain(TestFlags::Internal bits) { return TestFlags{bits}; }
        bool is_empty() const { return this->_0 == 0; }
        bool contains(const TestFlags& other) const { return (this->_0 & other._0) == other._0; }
        bool intersects(const TestFlags& other) const { return (this->_0 & other._0) != 0; }
        TestFlags complement() const { return TestFlags::from_bits_truncate(static_cast<decltype(this->_0)>(~this->_0)); }
        bool is_all() const { return (this->_0 & TestFlags::all()._0) == TestFlags::all()._0; }
        void insert(TestFlags other) { this->_0 |= other._0; }
        void remove(TestFlags other) { this->_0 &= ~other._0; }
        void toggle(TestFlags other) { this->_0 ^= other._0; }
        void set(TestFlags other, bool value) { if (value) { insert(std::move(other)); } else { remove(std::move(other)); } }
        TestFlags intersection(TestFlags other) const { return TestFlags{static_cast<decltype(this->_0)>(this->_0 & other._0)}; }
        TestFlags union_(TestFlags other) const { return TestFlags{static_cast<decltype(this->_0)>(this->_0 | other._0)}; }
        TestFlags difference(TestFlags other) const { return TestFlags{static_cast<decltype(this->_0)>(this->_0 & ~other._0)}; }
        TestFlags symmetric_difference(TestFlags other) const { return TestFlags{static_cast<decltype(this->_0)>(this->_0 ^ other._0)}; }
        rusty::Vec<TestFlags> iter() const { rusty::Vec<TestFlags> result; TestFlags rem = *this; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { result.push(flag); rem.remove(flag); } } if (!rem.is_empty()) { result.push(rem); } return result; }
        auto iter_names() const { struct IterNames { rusty::Vec<std::tuple<std::string_view, TestFlags>> items; TestFlags remaining_; auto begin() const { return items.begin(); } auto end() const { return items.end(); } TestFlags remaining() const { return remaining_; } }; TestFlags rem = *this; rusty::Vec<std::tuple<std::string_view, TestFlags>> v; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { v.push(std::make_tuple(FLAGS[i].name(), flag)); rem.remove(flag); } } return IterNames{std::move(v), rem}; }
        std::string to_string() const { rusty::fmt::Formatter f; f.write_str("TestFlags("); bool first = true; const auto iter = this->iter_names(); for (auto&& [name, _] : rusty::for_in(rusty::iter(iter))) { if (!first) { f.write_str(" | "); } first = false; f.write_str(name); } const auto remaining = iter.remaining(); if (!remaining.is_empty()) { if (!first) { f.write_str(" | "); } f.write_str("0x"); f.write_str(std::format("{0:x}", rusty::format_numeric_arg(remaining))); } else if (first) { f.write_str("0x0"); } f.write_str(")"); return f.str(); }
        template<typename Iter> void extend(Iter&& iter) { for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { this->_0 |= static_cast<decltype(this->_0)>(item._0); } else if constexpr (requires { item.bits(); }) { this->_0 |= static_cast<decltype(this->_0)>(item.bits()); } else { this->_0 |= static_cast<decltype(this->_0)>(item); } } }
        template<typename Iter> static TestFlags from_iter(Iter&& iter) { TestFlags result{}; for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { result._0 |= static_cast<decltype(result._0)>(item._0); } else if constexpr (requires { item.bits(); }) { result._0 |= static_cast<decltype(result._0)>(item.bits()); } else { result._0 |= static_cast<decltype(result._0)>(item); } } return result; }
    };
    inline const TestFlags TestFlags::A = TestFlags::from_bits_retain(1);
    inline const TestFlags TestFlags::B = TestFlags::from_bits_retain(1 << 1);
    inline const TestFlags TestFlags::C = TestFlags::from_bits_retain(1 << 2);
    inline const TestFlags TestFlags::ABC = TestFlags::from_bits_retain((TestFlags::A.bits() | TestFlags::B.bits()) | TestFlags::C.bits());
    inline const std::span<const ::traits::Flag<TestFlags>> TestFlags::FLAGS = []() -> std::span<const ::traits::Flag<TestFlags>> { static const std::array<::traits::Flag<TestFlags>, 4> _slice_ref_tmp = {::traits::Flag<TestFlags>::new_(std::string_view("A"), rusty::clone(rusty::clone(TestFlags::A))), ::traits::Flag<TestFlags>::new_(std::string_view("B"), rusty::clone(rusty::clone(TestFlags::B))), ::traits::Flag<TestFlags>::new_(std::string_view("C"), rusty::clone(rusty::clone(TestFlags::C))), ::traits::Flag<TestFlags>::new_(std::string_view("ABC"), rusty::clone(rusty::clone(TestFlags::ABC)))}; return std::span<const ::traits::Flag<TestFlags>>(_slice_ref_tmp); }();


    struct TestFlagsInvert {
        using Bits = uint8_t;
        using Primitive = uint8_t;
        using Internal = uint8_t;
        using Item = TestFlagsInvert;
        using IntoIter = ::iter::Iter<TestFlagsInvert>;
        TestFlagsInvert::Internal _0;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
        bool operator==(const TestFlagsInvert& other) const;
        void assert_receiver_is_total_eq() const;
        std::partial_ordering operator<=>(const TestFlagsInvert& other) const;
        rusty::cmp::Ordering cmp(const TestFlagsInvert& other) const;
        TestFlagsInvert clone() const;
        static const TestFlagsInvert A;
        static const TestFlagsInvert B;
        static const TestFlagsInvert C;
        static const std::span<const ::traits::Flag<TestFlagsInvert>> FLAGS;
        static TestFlagsInvert empty();
        static TestFlagsInvert all();
        static rusty::Option<TestFlagsInvert> from_bits(uint8_t bits);
        static TestFlagsInvert from_bits_truncate(uint8_t bits);
        static rusty::Option<TestFlagsInvert> from_name(std::string_view name);
        TestFlagsInvert operator|(const auto& other) const;
        void operator|=(TestFlagsInvert other);
        TestFlagsInvert operator^(TestFlagsInvert other) const;
        void operator^=(TestFlagsInvert other);
        TestFlagsInvert operator&(TestFlagsInvert other) const;
        void operator&=(TestFlagsInvert other);
        TestFlagsInvert operator-(TestFlagsInvert other) const;
        void operator-=(TestFlagsInvert other);
        TestFlagsInvert operator!() const;
        static const TestFlagsInvert ABC;

        // Synthetic bitwise trait methods (from const _ block impls)
        TestFlagsInvert::Internal bits() const { return this->_0; }
        static TestFlagsInvert from_bits_retain(TestFlagsInvert::Internal bits) { return TestFlagsInvert{bits}; }
        bool is_empty() const { return this->_0 == 0; }
        bool contains(const TestFlagsInvert& other) const { return (this->_0 & other._0) == other._0; }
        bool intersects(const TestFlagsInvert& other) const { return (this->_0 & other._0) != 0; }
        TestFlagsInvert complement() const { return TestFlagsInvert::from_bits_truncate(static_cast<decltype(this->_0)>(~this->_0)); }
        bool is_all() const { return (this->_0 & TestFlagsInvert::all()._0) == TestFlagsInvert::all()._0; }
        void insert(TestFlagsInvert other) { this->_0 |= other._0; }
        void remove(TestFlagsInvert other) { this->_0 &= ~other._0; }
        void toggle(TestFlagsInvert other) { this->_0 ^= other._0; }
        void set(TestFlagsInvert other, bool value) { if (value) { insert(std::move(other)); } else { remove(std::move(other)); } }
        TestFlagsInvert intersection(TestFlagsInvert other) const { return TestFlagsInvert{static_cast<decltype(this->_0)>(this->_0 & other._0)}; }
        TestFlagsInvert union_(TestFlagsInvert other) const { return TestFlagsInvert{static_cast<decltype(this->_0)>(this->_0 | other._0)}; }
        TestFlagsInvert difference(TestFlagsInvert other) const { return TestFlagsInvert{static_cast<decltype(this->_0)>(this->_0 & ~other._0)}; }
        TestFlagsInvert symmetric_difference(TestFlagsInvert other) const { return TestFlagsInvert{static_cast<decltype(this->_0)>(this->_0 ^ other._0)}; }
        rusty::Vec<TestFlagsInvert> iter() const { rusty::Vec<TestFlagsInvert> result; TestFlagsInvert rem = *this; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { result.push(flag); rem.remove(flag); } } if (!rem.is_empty()) { result.push(rem); } return result; }
        auto iter_names() const { struct IterNames { rusty::Vec<std::tuple<std::string_view, TestFlagsInvert>> items; TestFlagsInvert remaining_; auto begin() const { return items.begin(); } auto end() const { return items.end(); } TestFlagsInvert remaining() const { return remaining_; } }; TestFlagsInvert rem = *this; rusty::Vec<std::tuple<std::string_view, TestFlagsInvert>> v; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { v.push(std::make_tuple(FLAGS[i].name(), flag)); rem.remove(flag); } } return IterNames{std::move(v), rem}; }
        std::string to_string() const { rusty::fmt::Formatter f; f.write_str("TestFlagsInvert("); bool first = true; const auto iter = this->iter_names(); for (auto&& [name, _] : rusty::for_in(rusty::iter(iter))) { if (!first) { f.write_str(" | "); } first = false; f.write_str(name); } const auto remaining = iter.remaining(); if (!remaining.is_empty()) { if (!first) { f.write_str(" | "); } f.write_str("0x"); f.write_str(std::format("{0:x}", rusty::format_numeric_arg(remaining))); } else if (first) { f.write_str("0x0"); } f.write_str(")"); return f.str(); }
        template<typename Iter> void extend(Iter&& iter) { for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { this->_0 |= static_cast<decltype(this->_0)>(item._0); } else if constexpr (requires { item.bits(); }) { this->_0 |= static_cast<decltype(this->_0)>(item.bits()); } else { this->_0 |= static_cast<decltype(this->_0)>(item); } } }
        template<typename Iter> static TestFlagsInvert from_iter(Iter&& iter) { TestFlagsInvert result{}; for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { result._0 |= static_cast<decltype(result._0)>(item._0); } else if constexpr (requires { item.bits(); }) { result._0 |= static_cast<decltype(result._0)>(item.bits()); } else { result._0 |= static_cast<decltype(result._0)>(item); } } return result; }
    };
    inline const TestFlagsInvert TestFlagsInvert::A = TestFlagsInvert::from_bits_retain(1);
    inline const TestFlagsInvert TestFlagsInvert::B = TestFlagsInvert::from_bits_retain(1 << 1);
    inline const TestFlagsInvert TestFlagsInvert::C = TestFlagsInvert::from_bits_retain(1 << 2);
    inline const TestFlagsInvert TestFlagsInvert::ABC = TestFlagsInvert::from_bits_retain((TestFlagsInvert::A.bits() | TestFlagsInvert::B.bits()) | TestFlagsInvert::C.bits());
    inline const std::span<const ::traits::Flag<TestFlagsInvert>> TestFlagsInvert::FLAGS = []() -> std::span<const ::traits::Flag<TestFlagsInvert>> { static const std::array<::traits::Flag<TestFlagsInvert>, 4> _slice_ref_tmp = {::traits::Flag<TestFlagsInvert>::new_(std::string_view("ABC"), rusty::clone(rusty::clone(TestFlagsInvert::ABC))), ::traits::Flag<TestFlagsInvert>::new_(std::string_view("A"), rusty::clone(rusty::clone(TestFlagsInvert::A))), ::traits::Flag<TestFlagsInvert>::new_(std::string_view("B"), rusty::clone(rusty::clone(TestFlagsInvert::B))), ::traits::Flag<TestFlagsInvert>::new_(std::string_view("C"), rusty::clone(rusty::clone(TestFlagsInvert::C)))}; return std::span<const ::traits::Flag<TestFlagsInvert>>(_slice_ref_tmp); }();


    struct TestZero {
        using Bits = uint8_t;
        using Primitive = uint8_t;
        using Internal = uint8_t;
        using Item = TestZero;
        using IntoIter = ::iter::Iter<TestZero>;
        TestZero::Internal _0;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
        bool operator==(const TestZero& other) const;
        void assert_receiver_is_total_eq() const;
        std::partial_ordering operator<=>(const TestZero& other) const;
        rusty::cmp::Ordering cmp(const TestZero& other) const;
        TestZero clone() const;
        static const TestZero ZERO;
        static const std::span<const ::traits::Flag<TestZero>> FLAGS;
        static TestZero empty();
        static TestZero all();
        static rusty::Option<TestZero> from_bits(uint8_t bits);
        static TestZero from_bits_truncate(uint8_t bits);
        static rusty::Option<TestZero> from_name(std::string_view name);
        TestZero operator|(const auto& other) const;
        void operator|=(TestZero other);
        TestZero operator^(TestZero other) const;
        void operator^=(TestZero other);
        TestZero operator&(TestZero other) const;
        void operator&=(TestZero other);
        TestZero operator-(TestZero other) const;
        void operator-=(TestZero other);
        TestZero operator!() const;

        // Synthetic bitwise trait methods (from const _ block impls)
        TestZero::Internal bits() const { return this->_0; }
        static TestZero from_bits_retain(TestZero::Internal bits) { return TestZero{bits}; }
        bool is_empty() const { return this->_0 == 0; }
        bool contains(const TestZero& other) const { return (this->_0 & other._0) == other._0; }
        bool intersects(const TestZero& other) const { return (this->_0 & other._0) != 0; }
        TestZero complement() const { return TestZero::from_bits_truncate(static_cast<decltype(this->_0)>(~this->_0)); }
        bool is_all() const { return (this->_0 & TestZero::all()._0) == TestZero::all()._0; }
        void insert(TestZero other) { this->_0 |= other._0; }
        void remove(TestZero other) { this->_0 &= ~other._0; }
        void toggle(TestZero other) { this->_0 ^= other._0; }
        void set(TestZero other, bool value) { if (value) { insert(std::move(other)); } else { remove(std::move(other)); } }
        TestZero intersection(TestZero other) const { return TestZero{static_cast<decltype(this->_0)>(this->_0 & other._0)}; }
        TestZero union_(TestZero other) const { return TestZero{static_cast<decltype(this->_0)>(this->_0 | other._0)}; }
        TestZero difference(TestZero other) const { return TestZero{static_cast<decltype(this->_0)>(this->_0 & ~other._0)}; }
        TestZero symmetric_difference(TestZero other) const { return TestZero{static_cast<decltype(this->_0)>(this->_0 ^ other._0)}; }
        rusty::Vec<TestZero> iter() const { rusty::Vec<TestZero> result; TestZero rem = *this; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { result.push(flag); rem.remove(flag); } } if (!rem.is_empty()) { result.push(rem); } return result; }
        auto iter_names() const { struct IterNames { rusty::Vec<std::tuple<std::string_view, TestZero>> items; TestZero remaining_; auto begin() const { return items.begin(); } auto end() const { return items.end(); } TestZero remaining() const { return remaining_; } }; TestZero rem = *this; rusty::Vec<std::tuple<std::string_view, TestZero>> v; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { v.push(std::make_tuple(FLAGS[i].name(), flag)); rem.remove(flag); } } return IterNames{std::move(v), rem}; }
        std::string to_string() const { rusty::fmt::Formatter f; f.write_str("TestZero("); bool first = true; const auto iter = this->iter_names(); for (auto&& [name, _] : rusty::for_in(rusty::iter(iter))) { if (!first) { f.write_str(" | "); } first = false; f.write_str(name); } const auto remaining = iter.remaining(); if (!remaining.is_empty()) { if (!first) { f.write_str(" | "); } f.write_str("0x"); f.write_str(std::format("{0:x}", rusty::format_numeric_arg(remaining))); } else if (first) { f.write_str("0x0"); } f.write_str(")"); return f.str(); }
        template<typename Iter> void extend(Iter&& iter) { for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { this->_0 |= static_cast<decltype(this->_0)>(item._0); } else if constexpr (requires { item.bits(); }) { this->_0 |= static_cast<decltype(this->_0)>(item.bits()); } else { this->_0 |= static_cast<decltype(this->_0)>(item); } } }
        template<typename Iter> static TestZero from_iter(Iter&& iter) { TestZero result{}; for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { result._0 |= static_cast<decltype(result._0)>(item._0); } else if constexpr (requires { item.bits(); }) { result._0 |= static_cast<decltype(result._0)>(item.bits()); } else { result._0 |= static_cast<decltype(result._0)>(item); } } return result; }
    };
    inline const TestZero TestZero::ZERO = TestZero::from_bits_retain(0);
    inline const std::span<const ::traits::Flag<TestZero>> TestZero::FLAGS = []() -> std::span<const ::traits::Flag<TestZero>> { static const std::array<::traits::Flag<TestZero>, 1> _slice_ref_tmp = {::traits::Flag<TestZero>::new_(std::string_view("ZERO"), rusty::clone(rusty::clone(TestZero::ZERO)))}; return std::span<const ::traits::Flag<TestZero>>(_slice_ref_tmp); }();


    struct TestZeroOne {
        using Bits = uint8_t;
        using Primitive = uint8_t;
        using Internal = uint8_t;
        using Item = TestZeroOne;
        using IntoIter = ::iter::Iter<TestZeroOne>;
        TestZeroOne::Internal _0;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
        bool operator==(const TestZeroOne& other) const;
        void assert_receiver_is_total_eq() const;
        std::partial_ordering operator<=>(const TestZeroOne& other) const;
        rusty::cmp::Ordering cmp(const TestZeroOne& other) const;
        TestZeroOne clone() const;
        static const TestZeroOne ZERO;
        static const TestZeroOne ONE;
        static const std::span<const ::traits::Flag<TestZeroOne>> FLAGS;
        static TestZeroOne empty();
        static TestZeroOne all();
        static rusty::Option<TestZeroOne> from_bits(uint8_t bits);
        static TestZeroOne from_bits_truncate(uint8_t bits);
        static rusty::Option<TestZeroOne> from_name(std::string_view name);
        TestZeroOne operator|(const auto& other) const;
        void operator|=(TestZeroOne other);
        TestZeroOne operator^(TestZeroOne other) const;
        void operator^=(TestZeroOne other);
        TestZeroOne operator&(TestZeroOne other) const;
        void operator&=(TestZeroOne other);
        TestZeroOne operator-(TestZeroOne other) const;
        void operator-=(TestZeroOne other);
        TestZeroOne operator!() const;

        // Synthetic bitwise trait methods (from const _ block impls)
        TestZeroOne::Internal bits() const { return this->_0; }
        static TestZeroOne from_bits_retain(TestZeroOne::Internal bits) { return TestZeroOne{bits}; }
        bool is_empty() const { return this->_0 == 0; }
        bool contains(const TestZeroOne& other) const { return (this->_0 & other._0) == other._0; }
        bool intersects(const TestZeroOne& other) const { return (this->_0 & other._0) != 0; }
        TestZeroOne complement() const { return TestZeroOne::from_bits_truncate(static_cast<decltype(this->_0)>(~this->_0)); }
        bool is_all() const { return (this->_0 & TestZeroOne::all()._0) == TestZeroOne::all()._0; }
        void insert(TestZeroOne other) { this->_0 |= other._0; }
        void remove(TestZeroOne other) { this->_0 &= ~other._0; }
        void toggle(TestZeroOne other) { this->_0 ^= other._0; }
        void set(TestZeroOne other, bool value) { if (value) { insert(std::move(other)); } else { remove(std::move(other)); } }
        TestZeroOne intersection(TestZeroOne other) const { return TestZeroOne{static_cast<decltype(this->_0)>(this->_0 & other._0)}; }
        TestZeroOne union_(TestZeroOne other) const { return TestZeroOne{static_cast<decltype(this->_0)>(this->_0 | other._0)}; }
        TestZeroOne difference(TestZeroOne other) const { return TestZeroOne{static_cast<decltype(this->_0)>(this->_0 & ~other._0)}; }
        TestZeroOne symmetric_difference(TestZeroOne other) const { return TestZeroOne{static_cast<decltype(this->_0)>(this->_0 ^ other._0)}; }
        rusty::Vec<TestZeroOne> iter() const { rusty::Vec<TestZeroOne> result; TestZeroOne rem = *this; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { result.push(flag); rem.remove(flag); } } if (!rem.is_empty()) { result.push(rem); } return result; }
        auto iter_names() const { struct IterNames { rusty::Vec<std::tuple<std::string_view, TestZeroOne>> items; TestZeroOne remaining_; auto begin() const { return items.begin(); } auto end() const { return items.end(); } TestZeroOne remaining() const { return remaining_; } }; TestZeroOne rem = *this; rusty::Vec<std::tuple<std::string_view, TestZeroOne>> v; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { v.push(std::make_tuple(FLAGS[i].name(), flag)); rem.remove(flag); } } return IterNames{std::move(v), rem}; }
        std::string to_string() const { rusty::fmt::Formatter f; f.write_str("TestZeroOne("); bool first = true; const auto iter = this->iter_names(); for (auto&& [name, _] : rusty::for_in(rusty::iter(iter))) { if (!first) { f.write_str(" | "); } first = false; f.write_str(name); } const auto remaining = iter.remaining(); if (!remaining.is_empty()) { if (!first) { f.write_str(" | "); } f.write_str("0x"); f.write_str(std::format("{0:x}", rusty::format_numeric_arg(remaining))); } else if (first) { f.write_str("0x0"); } f.write_str(")"); return f.str(); }
        template<typename Iter> void extend(Iter&& iter) { for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { this->_0 |= static_cast<decltype(this->_0)>(item._0); } else if constexpr (requires { item.bits(); }) { this->_0 |= static_cast<decltype(this->_0)>(item.bits()); } else { this->_0 |= static_cast<decltype(this->_0)>(item); } } }
        template<typename Iter> static TestZeroOne from_iter(Iter&& iter) { TestZeroOne result{}; for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { result._0 |= static_cast<decltype(result._0)>(item._0); } else if constexpr (requires { item.bits(); }) { result._0 |= static_cast<decltype(result._0)>(item.bits()); } else { result._0 |= static_cast<decltype(result._0)>(item); } } return result; }
    };
    inline const TestZeroOne TestZeroOne::ZERO = TestZeroOne::from_bits_retain(0);
    inline const TestZeroOne TestZeroOne::ONE = TestZeroOne::from_bits_retain(1);
    inline const std::span<const ::traits::Flag<TestZeroOne>> TestZeroOne::FLAGS = []() -> std::span<const ::traits::Flag<TestZeroOne>> { static const std::array<::traits::Flag<TestZeroOne>, 2> _slice_ref_tmp = {::traits::Flag<TestZeroOne>::new_(std::string_view("ZERO"), rusty::clone(rusty::clone(TestZeroOne::ZERO))), ::traits::Flag<TestZeroOne>::new_(std::string_view("ONE"), rusty::clone(rusty::clone(TestZeroOne::ONE)))}; return std::span<const ::traits::Flag<TestZeroOne>>(_slice_ref_tmp); }();


    struct TestUnicode {
        using Bits = uint8_t;
        using Primitive = uint8_t;
        using Internal = uint8_t;
        using Item = TestUnicode;
        using IntoIter = ::iter::Iter<TestUnicode>;
        TestUnicode::Internal _0;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
        bool operator==(const TestUnicode& other) const;
        void assert_receiver_is_total_eq() const;
        std::partial_ordering operator<=>(const TestUnicode& other) const;
        rusty::cmp::Ordering cmp(const TestUnicode& other) const;
        TestUnicode clone() const;
        static const TestUnicode 一;
        static const TestUnicode 二;
        static const std::span<const ::traits::Flag<TestUnicode>> FLAGS;
        static TestUnicode empty();
        static TestUnicode all();
        static rusty::Option<TestUnicode> from_bits(uint8_t bits);
        static TestUnicode from_bits_truncate(uint8_t bits);
        static rusty::Option<TestUnicode> from_name(std::string_view name);
        TestUnicode operator|(const auto& other) const;
        void operator|=(TestUnicode other);
        TestUnicode operator^(TestUnicode other) const;
        void operator^=(TestUnicode other);
        TestUnicode operator&(TestUnicode other) const;
        void operator&=(TestUnicode other);
        TestUnicode operator-(TestUnicode other) const;
        void operator-=(TestUnicode other);
        TestUnicode operator!() const;

        // Synthetic bitwise trait methods (from const _ block impls)
        TestUnicode::Internal bits() const { return this->_0; }
        static TestUnicode from_bits_retain(TestUnicode::Internal bits) { return TestUnicode{bits}; }
        bool is_empty() const { return this->_0 == 0; }
        bool contains(const TestUnicode& other) const { return (this->_0 & other._0) == other._0; }
        bool intersects(const TestUnicode& other) const { return (this->_0 & other._0) != 0; }
        TestUnicode complement() const { return TestUnicode::from_bits_truncate(static_cast<decltype(this->_0)>(~this->_0)); }
        bool is_all() const { return (this->_0 & TestUnicode::all()._0) == TestUnicode::all()._0; }
        void insert(TestUnicode other) { this->_0 |= other._0; }
        void remove(TestUnicode other) { this->_0 &= ~other._0; }
        void toggle(TestUnicode other) { this->_0 ^= other._0; }
        void set(TestUnicode other, bool value) { if (value) { insert(std::move(other)); } else { remove(std::move(other)); } }
        TestUnicode intersection(TestUnicode other) const { return TestUnicode{static_cast<decltype(this->_0)>(this->_0 & other._0)}; }
        TestUnicode union_(TestUnicode other) const { return TestUnicode{static_cast<decltype(this->_0)>(this->_0 | other._0)}; }
        TestUnicode difference(TestUnicode other) const { return TestUnicode{static_cast<decltype(this->_0)>(this->_0 & ~other._0)}; }
        TestUnicode symmetric_difference(TestUnicode other) const { return TestUnicode{static_cast<decltype(this->_0)>(this->_0 ^ other._0)}; }
        rusty::Vec<TestUnicode> iter() const { rusty::Vec<TestUnicode> result; TestUnicode rem = *this; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { result.push(flag); rem.remove(flag); } } if (!rem.is_empty()) { result.push(rem); } return result; }
        auto iter_names() const { struct IterNames { rusty::Vec<std::tuple<std::string_view, TestUnicode>> items; TestUnicode remaining_; auto begin() const { return items.begin(); } auto end() const { return items.end(); } TestUnicode remaining() const { return remaining_; } }; TestUnicode rem = *this; rusty::Vec<std::tuple<std::string_view, TestUnicode>> v; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { v.push(std::make_tuple(FLAGS[i].name(), flag)); rem.remove(flag); } } return IterNames{std::move(v), rem}; }
        std::string to_string() const { rusty::fmt::Formatter f; f.write_str("TestUnicode("); bool first = true; const auto iter = this->iter_names(); for (auto&& [name, _] : rusty::for_in(rusty::iter(iter))) { if (!first) { f.write_str(" | "); } first = false; f.write_str(name); } const auto remaining = iter.remaining(); if (!remaining.is_empty()) { if (!first) { f.write_str(" | "); } f.write_str("0x"); f.write_str(std::format("{0:x}", rusty::format_numeric_arg(remaining))); } else if (first) { f.write_str("0x0"); } f.write_str(")"); return f.str(); }
        template<typename Iter> void extend(Iter&& iter) { for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { this->_0 |= static_cast<decltype(this->_0)>(item._0); } else if constexpr (requires { item.bits(); }) { this->_0 |= static_cast<decltype(this->_0)>(item.bits()); } else { this->_0 |= static_cast<decltype(this->_0)>(item); } } }
        template<typename Iter> static TestUnicode from_iter(Iter&& iter) { TestUnicode result{}; for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { result._0 |= static_cast<decltype(result._0)>(item._0); } else if constexpr (requires { item.bits(); }) { result._0 |= static_cast<decltype(result._0)>(item.bits()); } else { result._0 |= static_cast<decltype(result._0)>(item); } } return result; }
    };
    inline const TestUnicode TestUnicode::一 = TestUnicode::from_bits_retain(1);
    inline const TestUnicode TestUnicode::二 = TestUnicode::from_bits_retain(1 << 1);
    inline const std::span<const ::traits::Flag<TestUnicode>> TestUnicode::FLAGS = []() -> std::span<const ::traits::Flag<TestUnicode>> { static const std::array<::traits::Flag<TestUnicode>, 2> _slice_ref_tmp = {::traits::Flag<TestUnicode>::new_(std::string_view("一"), TestUnicode::一), ::traits::Flag<TestUnicode>::new_(std::string_view("二"), TestUnicode::二)}; return std::span<const ::traits::Flag<TestUnicode>>(_slice_ref_tmp); }();


    struct TestEmpty {
        using Bits = uint8_t;
        using Primitive = uint8_t;
        using Internal = uint8_t;
        using Item = TestEmpty;
        using IntoIter = ::iter::Iter<TestEmpty>;
        TestEmpty::Internal _0;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
        bool operator==(const TestEmpty& other) const;
        void assert_receiver_is_total_eq() const;
        std::partial_ordering operator<=>(const TestEmpty& other) const;
        rusty::cmp::Ordering cmp(const TestEmpty& other) const;
        TestEmpty clone() const;
        static const std::span<const ::traits::Flag<TestEmpty>> FLAGS;
        static TestEmpty empty();
        static TestEmpty all();
        static rusty::Option<TestEmpty> from_bits(uint8_t bits);
        static TestEmpty from_bits_truncate(uint8_t bits);
        static rusty::Option<TestEmpty> from_name(std::string_view name);
        TestEmpty operator|(const auto& other) const;
        void operator|=(TestEmpty other);
        TestEmpty operator^(TestEmpty other) const;
        void operator^=(TestEmpty other);
        TestEmpty operator&(TestEmpty other) const;
        void operator&=(TestEmpty other);
        TestEmpty operator-(TestEmpty other) const;
        void operator-=(TestEmpty other);
        TestEmpty operator!() const;

        // Synthetic bitwise trait methods (from const _ block impls)
        TestEmpty::Internal bits() const { return this->_0; }
        static TestEmpty from_bits_retain(TestEmpty::Internal bits) { return TestEmpty{bits}; }
        bool is_empty() const { return this->_0 == 0; }
        bool contains(const TestEmpty& other) const { return (this->_0 & other._0) == other._0; }
        bool intersects(const TestEmpty& other) const { return (this->_0 & other._0) != 0; }
        TestEmpty complement() const { return TestEmpty::from_bits_truncate(static_cast<decltype(this->_0)>(~this->_0)); }
        bool is_all() const { return (this->_0 & TestEmpty::all()._0) == TestEmpty::all()._0; }
        void insert(TestEmpty other) { this->_0 |= other._0; }
        void remove(TestEmpty other) { this->_0 &= ~other._0; }
        void toggle(TestEmpty other) { this->_0 ^= other._0; }
        void set(TestEmpty other, bool value) { if (value) { insert(std::move(other)); } else { remove(std::move(other)); } }
        TestEmpty intersection(TestEmpty other) const { return TestEmpty{static_cast<decltype(this->_0)>(this->_0 & other._0)}; }
        TestEmpty union_(TestEmpty other) const { return TestEmpty{static_cast<decltype(this->_0)>(this->_0 | other._0)}; }
        TestEmpty difference(TestEmpty other) const { return TestEmpty{static_cast<decltype(this->_0)>(this->_0 & ~other._0)}; }
        TestEmpty symmetric_difference(TestEmpty other) const { return TestEmpty{static_cast<decltype(this->_0)>(this->_0 ^ other._0)}; }
        rusty::Vec<TestEmpty> iter() const { rusty::Vec<TestEmpty> result; TestEmpty rem = *this; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { result.push(flag); rem.remove(flag); } } if (!rem.is_empty()) { result.push(rem); } return result; }
        auto iter_names() const { struct IterNames { rusty::Vec<std::tuple<std::string_view, TestEmpty>> items; TestEmpty remaining_; auto begin() const { return items.begin(); } auto end() const { return items.end(); } TestEmpty remaining() const { return remaining_; } }; TestEmpty rem = *this; rusty::Vec<std::tuple<std::string_view, TestEmpty>> v; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { v.push(std::make_tuple(FLAGS[i].name(), flag)); rem.remove(flag); } } return IterNames{std::move(v), rem}; }
        std::string to_string() const { rusty::fmt::Formatter f; f.write_str("TestEmpty("); bool first = true; const auto iter = this->iter_names(); for (auto&& [name, _] : rusty::for_in(rusty::iter(iter))) { if (!first) { f.write_str(" | "); } first = false; f.write_str(name); } const auto remaining = iter.remaining(); if (!remaining.is_empty()) { if (!first) { f.write_str(" | "); } f.write_str("0x"); f.write_str(std::format("{0:x}", rusty::format_numeric_arg(remaining))); } else if (first) { f.write_str("0x0"); } f.write_str(")"); return f.str(); }
        template<typename Iter> void extend(Iter&& iter) { for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { this->_0 |= static_cast<decltype(this->_0)>(item._0); } else if constexpr (requires { item.bits(); }) { this->_0 |= static_cast<decltype(this->_0)>(item.bits()); } else { this->_0 |= static_cast<decltype(this->_0)>(item); } } }
        template<typename Iter> static TestEmpty from_iter(Iter&& iter) { TestEmpty result{}; for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { result._0 |= static_cast<decltype(result._0)>(item._0); } else if constexpr (requires { item.bits(); }) { result._0 |= static_cast<decltype(result._0)>(item.bits()); } else { result._0 |= static_cast<decltype(result._0)>(item); } } return result; }
    };
    inline const std::span<const ::traits::Flag<TestEmpty>> TestEmpty::FLAGS = []() -> std::span<const ::traits::Flag<TestEmpty>> { static const std::array<::traits::Flag<TestEmpty>, 0> _slice_ref_tmp = {}; return std::span<const ::traits::Flag<TestEmpty>>(_slice_ref_tmp); }();


    struct TestOverlapping {
        using Bits = uint8_t;
        using Primitive = uint8_t;
        using Internal = uint8_t;
        using Item = TestOverlapping;
        using IntoIter = ::iter::Iter<TestOverlapping>;
        TestOverlapping::Internal _0;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
        bool operator==(const TestOverlapping& other) const;
        void assert_receiver_is_total_eq() const;
        std::partial_ordering operator<=>(const TestOverlapping& other) const;
        rusty::cmp::Ordering cmp(const TestOverlapping& other) const;
        TestOverlapping clone() const;
        static const TestOverlapping AB;
        static const TestOverlapping BC;
        static const std::span<const ::traits::Flag<TestOverlapping>> FLAGS;
        static TestOverlapping empty();
        static TestOverlapping all();
        static rusty::Option<TestOverlapping> from_bits(uint8_t bits);
        static TestOverlapping from_bits_truncate(uint8_t bits);
        static rusty::Option<TestOverlapping> from_name(std::string_view name);
        TestOverlapping operator|(const auto& other) const;
        void operator|=(TestOverlapping other);
        TestOverlapping operator^(TestOverlapping other) const;
        void operator^=(TestOverlapping other);
        TestOverlapping operator&(TestOverlapping other) const;
        void operator&=(TestOverlapping other);
        TestOverlapping operator-(TestOverlapping other) const;
        void operator-=(TestOverlapping other);
        TestOverlapping operator!() const;

        // Synthetic bitwise trait methods (from const _ block impls)
        TestOverlapping::Internal bits() const { return this->_0; }
        static TestOverlapping from_bits_retain(TestOverlapping::Internal bits) { return TestOverlapping{bits}; }
        bool is_empty() const { return this->_0 == 0; }
        bool contains(const TestOverlapping& other) const { return (this->_0 & other._0) == other._0; }
        bool intersects(const TestOverlapping& other) const { return (this->_0 & other._0) != 0; }
        TestOverlapping complement() const { return TestOverlapping::from_bits_truncate(static_cast<decltype(this->_0)>(~this->_0)); }
        bool is_all() const { return (this->_0 & TestOverlapping::all()._0) == TestOverlapping::all()._0; }
        void insert(TestOverlapping other) { this->_0 |= other._0; }
        void remove(TestOverlapping other) { this->_0 &= ~other._0; }
        void toggle(TestOverlapping other) { this->_0 ^= other._0; }
        void set(TestOverlapping other, bool value) { if (value) { insert(std::move(other)); } else { remove(std::move(other)); } }
        TestOverlapping intersection(TestOverlapping other) const { return TestOverlapping{static_cast<decltype(this->_0)>(this->_0 & other._0)}; }
        TestOverlapping union_(TestOverlapping other) const { return TestOverlapping{static_cast<decltype(this->_0)>(this->_0 | other._0)}; }
        TestOverlapping difference(TestOverlapping other) const { return TestOverlapping{static_cast<decltype(this->_0)>(this->_0 & ~other._0)}; }
        TestOverlapping symmetric_difference(TestOverlapping other) const { return TestOverlapping{static_cast<decltype(this->_0)>(this->_0 ^ other._0)}; }
        rusty::Vec<TestOverlapping> iter() const { rusty::Vec<TestOverlapping> result; TestOverlapping rem = *this; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { result.push(flag); rem.remove(flag); } } if (!rem.is_empty()) { result.push(rem); } return result; }
        auto iter_names() const { struct IterNames { rusty::Vec<std::tuple<std::string_view, TestOverlapping>> items; TestOverlapping remaining_; auto begin() const { return items.begin(); } auto end() const { return items.end(); } TestOverlapping remaining() const { return remaining_; } }; TestOverlapping rem = *this; rusty::Vec<std::tuple<std::string_view, TestOverlapping>> v; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { v.push(std::make_tuple(FLAGS[i].name(), flag)); rem.remove(flag); } } return IterNames{std::move(v), rem}; }
        std::string to_string() const { rusty::fmt::Formatter f; f.write_str("TestOverlapping("); bool first = true; const auto iter = this->iter_names(); for (auto&& [name, _] : rusty::for_in(rusty::iter(iter))) { if (!first) { f.write_str(" | "); } first = false; f.write_str(name); } const auto remaining = iter.remaining(); if (!remaining.is_empty()) { if (!first) { f.write_str(" | "); } f.write_str("0x"); f.write_str(std::format("{0:x}", rusty::format_numeric_arg(remaining))); } else if (first) { f.write_str("0x0"); } f.write_str(")"); return f.str(); }
        template<typename Iter> void extend(Iter&& iter) { for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { this->_0 |= static_cast<decltype(this->_0)>(item._0); } else if constexpr (requires { item.bits(); }) { this->_0 |= static_cast<decltype(this->_0)>(item.bits()); } else { this->_0 |= static_cast<decltype(this->_0)>(item); } } }
        template<typename Iter> static TestOverlapping from_iter(Iter&& iter) { TestOverlapping result{}; for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { result._0 |= static_cast<decltype(result._0)>(item._0); } else if constexpr (requires { item.bits(); }) { result._0 |= static_cast<decltype(result._0)>(item.bits()); } else { result._0 |= static_cast<decltype(result._0)>(item); } } return result; }
    };
    inline const TestOverlapping TestOverlapping::AB = TestOverlapping::from_bits_retain(1 | ((1 << 1)));
    inline const TestOverlapping TestOverlapping::BC = TestOverlapping::from_bits_retain(((1 << 1)) | ((1 << 2)));
    inline const std::span<const ::traits::Flag<TestOverlapping>> TestOverlapping::FLAGS = []() -> std::span<const ::traits::Flag<TestOverlapping>> { static const std::array<::traits::Flag<TestOverlapping>, 2> _slice_ref_tmp = {::traits::Flag<TestOverlapping>::new_(std::string_view("AB"), rusty::clone(rusty::clone(TestOverlapping::AB))), ::traits::Flag<TestOverlapping>::new_(std::string_view("BC"), rusty::clone(rusty::clone(TestOverlapping::BC)))}; return std::span<const ::traits::Flag<TestOverlapping>>(_slice_ref_tmp); }();


    struct TestOverlappingFull {
        using Bits = uint8_t;
        using Primitive = uint8_t;
        using Internal = uint8_t;
        using Item = TestOverlappingFull;
        using IntoIter = ::iter::Iter<TestOverlappingFull>;
        TestOverlappingFull::Internal _0;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
        bool operator==(const TestOverlappingFull& other) const;
        void assert_receiver_is_total_eq() const;
        std::partial_ordering operator<=>(const TestOverlappingFull& other) const;
        rusty::cmp::Ordering cmp(const TestOverlappingFull& other) const;
        TestOverlappingFull clone() const;
        static const TestOverlappingFull A;
        static const TestOverlappingFull B;
        static const TestOverlappingFull C;
        static const TestOverlappingFull D;
        static const std::span<const ::traits::Flag<TestOverlappingFull>> FLAGS;
        static TestOverlappingFull empty();
        static TestOverlappingFull all();
        static rusty::Option<TestOverlappingFull> from_bits(uint8_t bits);
        static TestOverlappingFull from_bits_truncate(uint8_t bits);
        static rusty::Option<TestOverlappingFull> from_name(std::string_view name);
        TestOverlappingFull operator|(const auto& other) const;
        void operator|=(TestOverlappingFull other);
        TestOverlappingFull operator^(TestOverlappingFull other) const;
        void operator^=(TestOverlappingFull other);
        TestOverlappingFull operator&(TestOverlappingFull other) const;
        void operator&=(TestOverlappingFull other);
        TestOverlappingFull operator-(TestOverlappingFull other) const;
        void operator-=(TestOverlappingFull other);
        TestOverlappingFull operator!() const;

        // Synthetic bitwise trait methods (from const _ block impls)
        TestOverlappingFull::Internal bits() const { return this->_0; }
        static TestOverlappingFull from_bits_retain(TestOverlappingFull::Internal bits) { return TestOverlappingFull{bits}; }
        bool is_empty() const { return this->_0 == 0; }
        bool contains(const TestOverlappingFull& other) const { return (this->_0 & other._0) == other._0; }
        bool intersects(const TestOverlappingFull& other) const { return (this->_0 & other._0) != 0; }
        TestOverlappingFull complement() const { return TestOverlappingFull::from_bits_truncate(static_cast<decltype(this->_0)>(~this->_0)); }
        bool is_all() const { return (this->_0 & TestOverlappingFull::all()._0) == TestOverlappingFull::all()._0; }
        void insert(TestOverlappingFull other) { this->_0 |= other._0; }
        void remove(TestOverlappingFull other) { this->_0 &= ~other._0; }
        void toggle(TestOverlappingFull other) { this->_0 ^= other._0; }
        void set(TestOverlappingFull other, bool value) { if (value) { insert(std::move(other)); } else { remove(std::move(other)); } }
        TestOverlappingFull intersection(TestOverlappingFull other) const { return TestOverlappingFull{static_cast<decltype(this->_0)>(this->_0 & other._0)}; }
        TestOverlappingFull union_(TestOverlappingFull other) const { return TestOverlappingFull{static_cast<decltype(this->_0)>(this->_0 | other._0)}; }
        TestOverlappingFull difference(TestOverlappingFull other) const { return TestOverlappingFull{static_cast<decltype(this->_0)>(this->_0 & ~other._0)}; }
        TestOverlappingFull symmetric_difference(TestOverlappingFull other) const { return TestOverlappingFull{static_cast<decltype(this->_0)>(this->_0 ^ other._0)}; }
        rusty::Vec<TestOverlappingFull> iter() const { rusty::Vec<TestOverlappingFull> result; TestOverlappingFull rem = *this; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { result.push(flag); rem.remove(flag); } } if (!rem.is_empty()) { result.push(rem); } return result; }
        auto iter_names() const { struct IterNames { rusty::Vec<std::tuple<std::string_view, TestOverlappingFull>> items; TestOverlappingFull remaining_; auto begin() const { return items.begin(); } auto end() const { return items.end(); } TestOverlappingFull remaining() const { return remaining_; } }; TestOverlappingFull rem = *this; rusty::Vec<std::tuple<std::string_view, TestOverlappingFull>> v; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { v.push(std::make_tuple(FLAGS[i].name(), flag)); rem.remove(flag); } } return IterNames{std::move(v), rem}; }
        std::string to_string() const { rusty::fmt::Formatter f; f.write_str("TestOverlappingFull("); bool first = true; const auto iter = this->iter_names(); for (auto&& [name, _] : rusty::for_in(rusty::iter(iter))) { if (!first) { f.write_str(" | "); } first = false; f.write_str(name); } const auto remaining = iter.remaining(); if (!remaining.is_empty()) { if (!first) { f.write_str(" | "); } f.write_str("0x"); f.write_str(std::format("{0:x}", rusty::format_numeric_arg(remaining))); } else if (first) { f.write_str("0x0"); } f.write_str(")"); return f.str(); }
        template<typename Iter> void extend(Iter&& iter) { for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { this->_0 |= static_cast<decltype(this->_0)>(item._0); } else if constexpr (requires { item.bits(); }) { this->_0 |= static_cast<decltype(this->_0)>(item.bits()); } else { this->_0 |= static_cast<decltype(this->_0)>(item); } } }
        template<typename Iter> static TestOverlappingFull from_iter(Iter&& iter) { TestOverlappingFull result{}; for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { result._0 |= static_cast<decltype(result._0)>(item._0); } else if constexpr (requires { item.bits(); }) { result._0 |= static_cast<decltype(result._0)>(item.bits()); } else { result._0 |= static_cast<decltype(result._0)>(item); } } return result; }
    };
    inline const TestOverlappingFull TestOverlappingFull::A = TestOverlappingFull::from_bits_retain(1);
    inline const TestOverlappingFull TestOverlappingFull::B = TestOverlappingFull::from_bits_retain(1);
    inline const TestOverlappingFull TestOverlappingFull::C = TestOverlappingFull::from_bits_retain(1);
    inline const TestOverlappingFull TestOverlappingFull::D = TestOverlappingFull::from_bits_retain(1 << 1);
    inline const std::span<const ::traits::Flag<TestOverlappingFull>> TestOverlappingFull::FLAGS = []() -> std::span<const ::traits::Flag<TestOverlappingFull>> { static const std::array<::traits::Flag<TestOverlappingFull>, 4> _slice_ref_tmp = {::traits::Flag<TestOverlappingFull>::new_(std::string_view("A"), rusty::clone(rusty::clone(TestOverlappingFull::A))), ::traits::Flag<TestOverlappingFull>::new_(std::string_view("B"), rusty::clone(rusty::clone(TestOverlappingFull::B))), ::traits::Flag<TestOverlappingFull>::new_(std::string_view("C"), rusty::clone(rusty::clone(TestOverlappingFull::C))), ::traits::Flag<TestOverlappingFull>::new_(std::string_view("D"), rusty::clone(rusty::clone(TestOverlappingFull::D)))}; return std::span<const ::traits::Flag<TestOverlappingFull>>(_slice_ref_tmp); }();


    struct TestExternal {
        using Bits = uint8_t;
        using Primitive = uint8_t;
        using Internal = uint8_t;
        using Item = TestExternal;
        using IntoIter = ::iter::Iter<TestExternal>;
        TestExternal::Internal _0;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
        bool operator==(const TestExternal& other) const;
        void assert_receiver_is_total_eq() const;
        std::partial_ordering operator<=>(const TestExternal& other) const;
        rusty::cmp::Ordering cmp(const TestExternal& other) const;
        TestExternal clone() const;
        static const TestExternal A;
        static const TestExternal B;
        static const TestExternal C;
        static const std::span<const ::traits::Flag<TestExternal>> FLAGS;
        static TestExternal empty();
        static TestExternal all();
        static rusty::Option<TestExternal> from_bits(uint8_t bits);
        static TestExternal from_bits_truncate(uint8_t bits);
        static rusty::Option<TestExternal> from_name(std::string_view name);
        TestExternal operator|(const auto& other) const;
        void operator|=(TestExternal other);
        TestExternal operator^(TestExternal other) const;
        void operator^=(TestExternal other);
        TestExternal operator&(TestExternal other) const;
        void operator&=(TestExternal other);
        TestExternal operator-(TestExternal other) const;
        void operator-=(TestExternal other);
        TestExternal operator!() const;
        static const TestExternal ABC;

        // Synthetic bitwise trait methods (from const _ block impls)
        TestExternal::Internal bits() const { return this->_0; }
        static TestExternal from_bits_retain(TestExternal::Internal bits) { return TestExternal{bits}; }
        bool is_empty() const { return this->_0 == 0; }
        bool contains(const TestExternal& other) const { return (this->_0 & other._0) == other._0; }
        bool intersects(const TestExternal& other) const { return (this->_0 & other._0) != 0; }
        TestExternal complement() const { return TestExternal::from_bits_truncate(static_cast<decltype(this->_0)>(~this->_0)); }
        bool is_all() const { return (this->_0 & TestExternal::all()._0) == TestExternal::all()._0; }
        void insert(TestExternal other) { this->_0 |= other._0; }
        void remove(TestExternal other) { this->_0 &= ~other._0; }
        void toggle(TestExternal other) { this->_0 ^= other._0; }
        void set(TestExternal other, bool value) { if (value) { insert(std::move(other)); } else { remove(std::move(other)); } }
        TestExternal intersection(TestExternal other) const { return TestExternal{static_cast<decltype(this->_0)>(this->_0 & other._0)}; }
        TestExternal union_(TestExternal other) const { return TestExternal{static_cast<decltype(this->_0)>(this->_0 | other._0)}; }
        TestExternal difference(TestExternal other) const { return TestExternal{static_cast<decltype(this->_0)>(this->_0 & ~other._0)}; }
        TestExternal symmetric_difference(TestExternal other) const { return TestExternal{static_cast<decltype(this->_0)>(this->_0 ^ other._0)}; }
        rusty::Vec<TestExternal> iter() const { rusty::Vec<TestExternal> result; TestExternal rem = *this; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { result.push(flag); rem.remove(flag); } } if (!rem.is_empty()) { result.push(rem); } return result; }
        auto iter_names() const { struct IterNames { rusty::Vec<std::tuple<std::string_view, TestExternal>> items; TestExternal remaining_; auto begin() const { return items.begin(); } auto end() const { return items.end(); } TestExternal remaining() const { return remaining_; } }; TestExternal rem = *this; rusty::Vec<std::tuple<std::string_view, TestExternal>> v; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { v.push(std::make_tuple(FLAGS[i].name(), flag)); rem.remove(flag); } } return IterNames{std::move(v), rem}; }
        std::string to_string() const { rusty::fmt::Formatter f; f.write_str("TestExternal("); bool first = true; const auto iter = this->iter_names(); for (auto&& [name, _] : rusty::for_in(rusty::iter(iter))) { if (!first) { f.write_str(" | "); } first = false; f.write_str(name); } const auto remaining = iter.remaining(); if (!remaining.is_empty()) { if (!first) { f.write_str(" | "); } f.write_str("0x"); f.write_str(std::format("{0:x}", rusty::format_numeric_arg(remaining))); } else if (first) { f.write_str("0x0"); } f.write_str(")"); return f.str(); }
        template<typename Iter> void extend(Iter&& iter) { for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { this->_0 |= static_cast<decltype(this->_0)>(item._0); } else if constexpr (requires { item.bits(); }) { this->_0 |= static_cast<decltype(this->_0)>(item.bits()); } else { this->_0 |= static_cast<decltype(this->_0)>(item); } } }
        template<typename Iter> static TestExternal from_iter(Iter&& iter) { TestExternal result{}; for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { result._0 |= static_cast<decltype(result._0)>(item._0); } else if constexpr (requires { item.bits(); }) { result._0 |= static_cast<decltype(result._0)>(item.bits()); } else { result._0 |= static_cast<decltype(result._0)>(item); } } return result; }
    };
    inline const TestExternal TestExternal::A = TestExternal::from_bits_retain(1);
    inline const TestExternal TestExternal::B = TestExternal::from_bits_retain(1 << 1);
    inline const TestExternal TestExternal::C = TestExternal::from_bits_retain(1 << 2);
    inline const TestExternal TestExternal::ABC = TestExternal::from_bits_retain((TestExternal::A.bits() | TestExternal::B.bits()) | TestExternal::C.bits());
    inline const std::span<const ::traits::Flag<TestExternal>> TestExternal::FLAGS = []() -> std::span<const ::traits::Flag<TestExternal>> { static const std::array<::traits::Flag<TestExternal>, 5> _slice_ref_tmp = {::traits::Flag<TestExternal>::new_(std::string_view("A"), rusty::clone(rusty::clone(TestExternal::A))), ::traits::Flag<TestExternal>::new_(std::string_view("B"), rusty::clone(rusty::clone(TestExternal::B))), ::traits::Flag<TestExternal>::new_(std::string_view("C"), rusty::clone(rusty::clone(TestExternal::C))), ::traits::Flag<TestExternal>::new_(std::string_view("ABC"), rusty::clone(rusty::clone(TestExternal::ABC))), ::traits::Flag<TestExternal>::new_(std::string_view(""), TestExternal::from_bits_retain(~static_cast<int32_t>(0)))}; return std::span<const ::traits::Flag<TestExternal>>(_slice_ref_tmp); }();


    struct TestExternalFull {
        using Bits = uint8_t;
        using Primitive = uint8_t;
        using Internal = uint8_t;
        using Item = TestExternalFull;
        using IntoIter = ::iter::Iter<TestExternalFull>;
        TestExternalFull::Internal _0;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
        bool operator==(const TestExternalFull& other) const;
        void assert_receiver_is_total_eq() const;
        std::partial_ordering operator<=>(const TestExternalFull& other) const;
        rusty::cmp::Ordering cmp(const TestExternalFull& other) const;
        TestExternalFull clone() const;
        static const std::span<const ::traits::Flag<TestExternalFull>> FLAGS;
        static TestExternalFull empty();
        static TestExternalFull all();
        static rusty::Option<TestExternalFull> from_bits(uint8_t bits);
        static TestExternalFull from_bits_truncate(uint8_t bits);
        static rusty::Option<TestExternalFull> from_name(std::string_view name);
        TestExternalFull operator|(const auto& other) const;
        void operator|=(TestExternalFull other);
        TestExternalFull operator^(TestExternalFull other) const;
        void operator^=(TestExternalFull other);
        TestExternalFull operator&(TestExternalFull other) const;
        void operator&=(TestExternalFull other);
        TestExternalFull operator-(TestExternalFull other) const;
        void operator-=(TestExternalFull other);
        TestExternalFull operator!() const;

        // Synthetic bitwise trait methods (from const _ block impls)
        TestExternalFull::Internal bits() const { return this->_0; }
        static TestExternalFull from_bits_retain(TestExternalFull::Internal bits) { return TestExternalFull{bits}; }
        bool is_empty() const { return this->_0 == 0; }
        bool contains(const TestExternalFull& other) const { return (this->_0 & other._0) == other._0; }
        bool intersects(const TestExternalFull& other) const { return (this->_0 & other._0) != 0; }
        TestExternalFull complement() const { return TestExternalFull::from_bits_truncate(static_cast<decltype(this->_0)>(~this->_0)); }
        bool is_all() const { return (this->_0 & TestExternalFull::all()._0) == TestExternalFull::all()._0; }
        void insert(TestExternalFull other) { this->_0 |= other._0; }
        void remove(TestExternalFull other) { this->_0 &= ~other._0; }
        void toggle(TestExternalFull other) { this->_0 ^= other._0; }
        void set(TestExternalFull other, bool value) { if (value) { insert(std::move(other)); } else { remove(std::move(other)); } }
        TestExternalFull intersection(TestExternalFull other) const { return TestExternalFull{static_cast<decltype(this->_0)>(this->_0 & other._0)}; }
        TestExternalFull union_(TestExternalFull other) const { return TestExternalFull{static_cast<decltype(this->_0)>(this->_0 | other._0)}; }
        TestExternalFull difference(TestExternalFull other) const { return TestExternalFull{static_cast<decltype(this->_0)>(this->_0 & ~other._0)}; }
        TestExternalFull symmetric_difference(TestExternalFull other) const { return TestExternalFull{static_cast<decltype(this->_0)>(this->_0 ^ other._0)}; }
        rusty::Vec<TestExternalFull> iter() const { rusty::Vec<TestExternalFull> result; TestExternalFull rem = *this; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { result.push(flag); rem.remove(flag); } } if (!rem.is_empty()) { result.push(rem); } return result; }
        auto iter_names() const { struct IterNames { rusty::Vec<std::tuple<std::string_view, TestExternalFull>> items; TestExternalFull remaining_; auto begin() const { return items.begin(); } auto end() const { return items.end(); } TestExternalFull remaining() const { return remaining_; } }; TestExternalFull rem = *this; rusty::Vec<std::tuple<std::string_view, TestExternalFull>> v; for (size_t i = 0; i < FLAGS.size(); i++) { if (FLAGS[i].name().empty()) { continue; } const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) { v.push(std::make_tuple(FLAGS[i].name(), flag)); rem.remove(flag); } } return IterNames{std::move(v), rem}; }
        std::string to_string() const { rusty::fmt::Formatter f; f.write_str("TestExternalFull("); bool first = true; const auto iter = this->iter_names(); for (auto&& [name, _] : rusty::for_in(rusty::iter(iter))) { if (!first) { f.write_str(" | "); } first = false; f.write_str(name); } const auto remaining = iter.remaining(); if (!remaining.is_empty()) { if (!first) { f.write_str(" | "); } f.write_str("0x"); f.write_str(std::format("{0:x}", rusty::format_numeric_arg(remaining))); } else if (first) { f.write_str("0x0"); } f.write_str(")"); return f.str(); }
        template<typename Iter> void extend(Iter&& iter) { for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { this->_0 |= static_cast<decltype(this->_0)>(item._0); } else if constexpr (requires { item.bits(); }) { this->_0 |= static_cast<decltype(this->_0)>(item.bits()); } else { this->_0 |= static_cast<decltype(this->_0)>(item); } } }
        template<typename Iter> static TestExternalFull from_iter(Iter&& iter) { TestExternalFull result{}; for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) { if constexpr (requires { item._0; }) { result._0 |= static_cast<decltype(result._0)>(item._0); } else if constexpr (requires { item.bits(); }) { result._0 |= static_cast<decltype(result._0)>(item.bits()); } else { result._0 |= static_cast<decltype(result._0)>(item); } } return result; }
    };
    inline const std::span<const ::traits::Flag<TestExternalFull>> TestExternalFull::FLAGS = []() -> std::span<const ::traits::Flag<TestExternalFull>> { static const std::array<::traits::Flag<TestExternalFull>, 1> _slice_ref_tmp = {::traits::Flag<TestExternalFull>::new_(std::string_view(""), TestExternalFull::from_bits_retain(~static_cast<int32_t>(0)))}; return std::span<const ::traits::Flag<TestExternalFull>>(_slice_ref_tmp); }();


    namespace all {

        void cases();
        template<typename T>
        void case_(typename T::Bits expected, const auto& inherent);

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::all::cases, should_panic: no)

        void cases() {
            case_<TestFlags>((1 | (1 << 1)) | (1 << 2), TestFlags::all);
            case_<TestZero>(0, TestZero::all);
            case_<TestEmpty>(0, TestEmpty::all);
            case_<TestExternal>(~static_cast<int32_t>(0), TestExternal::all);
        }

        template<typename T>
        void case_(typename T::Bits expected, const auto& inherent) {
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = inherent().bits();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::string("T::all()")));
                    }
                    _m_matched = true;
                }
            }
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = T::all().bits();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::string("Flags::all()")));
                    }
                    _m_matched = true;
                }
            }
        }

    }

    namespace empty {

        void cases();
        template<typename T>
        void case_(typename T::Bits expected, const auto& inherent);

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::empty::cases, should_panic: no)

        void cases() {
            case_<TestFlags>(0, TestFlags::empty);
            case_<TestZero>(0, TestZero::empty);
            case_<TestEmpty>(0, TestEmpty::empty);
            case_<TestExternal>(0, TestExternal::empty);
        }

        template<typename T>
        void case_(typename T::Bits expected, const auto& inherent) {
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = inherent().bits();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::string("T::empty()")));
                    }
                    _m_matched = true;
                }
            }
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = T::empty().bits();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::string("Flags::empty()")));
                    }
                    _m_matched = true;
                }
            }
        }

    }

    namespace flags {
        namespace external {}

        namespace external {
            void cases();
        }
        void cases();

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::flags::cases, should_panic: no)

        namespace external {

            void cases();

            using namespace ::tests::flags;


            // Rust-only libtest metadata const skipped: cases (marker: tests::flags::external::cases, should_panic: no)

            void cases() {
                const auto flags = rusty::collect_range(rusty::map(rusty::iter(TestExternal::FLAGS), [&](auto&& flag) { return std::make_tuple(flag.name(), flag.value().bits()); }));
                {
                    auto&& _m0_tmp = rusty::boxed::into_vec(rusty::boxed::box_new(std::array{std::tuple<std::string_view, uint8_t>{std::string_view("A"), static_cast<uint8_t>(1)}, std::tuple<std::string_view, uint8_t>{std::string_view("B"), 1 << 1}, std::tuple<std::string_view, uint8_t>{std::string_view("C"), 1 << 2}, std::tuple<std::string_view, uint8_t>{std::string_view("ABC"), (1 | (1 << 1)) | (1 << 2)}, std::tuple<std::string_view, uint8_t>{std::string_view(""), ~static_cast<int32_t>(0)}}));
                    auto _m0 = &_m0_tmp;
                    auto _m1 = &flags;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
            }

        }

        void cases() {
            const auto flags = rusty::collect_range(rusty::map(rusty::iter(TestFlags::FLAGS), [&](auto&& flag) { return std::make_tuple(flag.name(), flag.value().bits()); }));
            {
                auto&& _m0_tmp = rusty::boxed::into_vec(rusty::boxed::box_new(std::array{std::tuple<std::string_view, uint8_t>{std::string_view("A"), static_cast<uint8_t>(1)}, std::tuple<std::string_view, uint8_t>{std::string_view("B"), 1 << 1}, std::tuple<std::string_view, uint8_t>{std::string_view("C"), 1 << 2}, std::tuple<std::string_view, uint8_t>{std::string_view("ABC"), (1 | (1 << 1)) | (1 << 2)}}));
                auto _m0 = &_m0_tmp;
                auto _m1 = &flags;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                    }
                    _m_matched = true;
                }
            }
            {
                auto&& _m0_tmp = static_cast<int32_t>(0);
                auto _m0 = &_m0_tmp;
                auto&& _m1_tmp = rusty::count(rusty::iter(TestEmpty::FLAGS));
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                    }
                    _m_matched = true;
                }
            }
        }

    }

    namespace from_bits {

        void cases();
        template<typename T>
        void case_(rusty::Option<typename T::Bits> expected, typename T::Bits input, const auto& inherent);

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::from_bits::cases, should_panic: no)

        void cases() {
            case_<TestFlags>(rusty::Option<TestFlags::Bits>(0), 0, TestFlags::from_bits);
            case_<TestFlags>(rusty::Option<TestFlags::Bits>(1), 1, TestFlags::from_bits);
            case_<TestFlags>(rusty::Option<TestFlags::Bits>((1 | (1 << 1)) | (1 << 2)), (1 | (1 << 1)) | (1 << 2), TestFlags::from_bits);
            case_<TestFlags>(rusty::Option<TestFlags::Bits>(rusty::None), 1 << 3, TestFlags::from_bits);
            case_<TestFlags>(rusty::Option<TestFlags::Bits>(rusty::None), 1 | (1 << 3), TestFlags::from_bits);
            case_<TestOverlapping>(rusty::Option<TestOverlapping::Bits>(1 | (1 << 1)), 1 | (1 << 1), TestOverlapping::from_bits);
            case_<TestOverlapping>(rusty::Option<TestOverlapping::Bits>(1 << 1), 1 << 1, TestOverlapping::from_bits);
            case_<TestExternal>(rusty::Option<TestExternal::Bits>(1 << 5), 1 << 5, TestExternal::from_bits);
        }

        template<typename T>
        void case_(rusty::Option<typename T::Bits> expected, typename T::Bits input, const auto& inherent) {
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = inherent(std::move(input)).map([&](auto&& f) -> typename T::Bits { return f.bits(); });
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("T::from_bits({0})", rusty::to_debug_string(input))));
                    }
                    _m_matched = true;
                }
            }
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = T::from_bits(std::move(input)).map([&](auto&& f) -> typename T::Bits { return f.bits(); });
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::from_bits({0})", rusty::to_debug_string(input))));
                    }
                    _m_matched = true;
                }
            }
        }

    }

    namespace from_bits_retain {

        void cases();
        template<typename T>
        void case_(typename T::Bits input, const auto& inherent);

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::from_bits_retain::cases, should_panic: no)

        void cases() {
            case_<TestFlags>(0, TestFlags::from_bits_retain);
            case_<TestFlags>(1, TestFlags::from_bits_retain);
            case_<TestFlags>((1 | (1 << 1)) | (1 << 2), TestFlags::from_bits_retain);
            case_<TestFlags>(1 << 3, TestFlags::from_bits_retain);
            case_<TestFlags>(1 | (1 << 3), TestFlags::from_bits_retain);
            case_<TestOverlapping>(1 | (1 << 1), TestOverlapping::from_bits_retain);
            case_<TestOverlapping>(1 << 1, TestOverlapping::from_bits_retain);
            case_<TestExternal>(1 << 5, TestExternal::from_bits_retain);
        }

        template<typename T>
        void case_(typename T::Bits input, const auto& inherent) {
            {
                auto _m0 = &input;
                auto&& _m1_tmp = inherent(std::move(input)).bits();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("T::from_bits_retain({0})", rusty::to_debug_string(input))));
                    }
                    _m_matched = true;
                }
            }
            {
                auto _m0 = &input;
                auto&& _m1_tmp = T::from_bits_retain(std::move(input)).bits();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::from_bits_retain({0})", rusty::to_debug_string(input))));
                    }
                    _m_matched = true;
                }
            }
        }

    }

    namespace bits {

        void cases();
        template<typename T>
        void case_(typename T::Bits expected, T value, const auto& inherent);

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::bits::cases, should_panic: no)

        void cases() {
            case_<TestFlags>(0, TestFlags::empty(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.bits(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(1, rusty::clone(rusty::clone(TestFlags::A)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.bits(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>((1 | (1 << 1)) | (1 << 2), rusty::clone(rusty::clone(TestFlags::ABC)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.bits(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(~static_cast<int32_t>(0), TestFlags::from_bits_retain(std::numeric_limits<uint8_t>::max()), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.bits(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(1 << 3, TestFlags::from_bits_retain(1 << 3), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.bits(std::forward<decltype(_args)>(_args)...); });
            case_<TestZero>(1 << 3, TestZero::from_bits_retain(1 << 3), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.bits(std::forward<decltype(_args)>(_args)...); });
            case_<TestEmpty>(1 << 3, TestEmpty::from_bits_retain(1 << 3), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.bits(std::forward<decltype(_args)>(_args)...); });
            case_<TestExternal>((1 << 4) | (1 << 6), TestExternal::from_bits_retain((1 << 4) | (1 << 6)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.bits(std::forward<decltype(_args)>(_args)...); });
        }

        template<typename T>
        void case_(typename T::Bits expected, T value, const auto& inherent) {
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = inherent(value);
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(left_val == right_val)) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::Some(std::format("{0}.bits()", rusty::to_debug_string(value))));
                    }
                    _m_matched = true;
                }
            }
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = value.bits();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::bits({0})", rusty::to_debug_string(value))));
                    }
                    _m_matched = true;
                }
            }
        }

    }

    namespace complement {

        void cases();
        template<typename T>
        void case_(typename T::Bits expected, T value, const auto& inherent);

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::complement::cases, should_panic: no)

        void cases() {
            case_<TestFlags>(0, TestFlags::all(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.complement(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(0, TestFlags::from_bits_retain(~static_cast<int32_t>(0)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.complement(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(1 | (1 << 1), rusty::clone(rusty::clone(TestFlags::C)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.complement(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(1 | (1 << 1), rusty::clone(TestFlags::C) | TestFlags::from_bits_retain(1 << 3), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.complement(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>((1 | (1 << 1)) | (1 << 2), TestFlags::empty(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.complement(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>((1 | (1 << 1)) | (1 << 2), TestFlags::from_bits_retain(1 << 3), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.complement(std::forward<decltype(_args)>(_args)...); });
            case_<TestZero>(0, TestZero::empty(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.complement(std::forward<decltype(_args)>(_args)...); });
            case_<TestEmpty>(0, TestEmpty::empty(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.complement(std::forward<decltype(_args)>(_args)...); });
            case_<TestOverlapping>(1 << 2, rusty::clone(rusty::clone(TestOverlapping::AB)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.complement(std::forward<decltype(_args)>(_args)...); });
            case_<TestExternal>(~static_cast<int32_t>(0), TestExternal::empty(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.complement(std::forward<decltype(_args)>(_args)...); });
        }

        template<typename T>
        void case_(typename T::Bits expected, T value, const auto& inherent) {
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = inherent(std::move(value)).bits();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0}.complement()", rusty::to_debug_string(value))));
                    }
                    _m_matched = true;
                }
            }
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = (std::move(value)).complement().bits();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::complement({0})", rusty::to_debug_string(value))));
                    }
                    _m_matched = true;
                }
            }
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = ((!value)).bits();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("!{0}", rusty::to_debug_string(value))));
                    }
                    _m_matched = true;
                }
            }
        }

    }

    namespace contains {

        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, bool>> inputs, const auto& inherent);

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::contains::cases, should_panic: no)

        void cases() {
            case_<TestFlags>(TestFlags::empty(), [&]() -> std::span<const std::tuple<::tests::TestFlags, bool>> { static const std::array<std::tuple<::tests::TestFlags, bool>, 5> _slice_ref_tmp = {std::make_tuple(TestFlags::empty(), true), std::make_tuple(rusty::clone(rusty::clone(TestFlags::A)), false), std::make_tuple(rusty::clone(rusty::clone(TestFlags::B)), false), std::make_tuple(rusty::clone(rusty::clone(TestFlags::C)), false), std::make_tuple(TestFlags::from_bits_retain(1 << 3), false)}; return std::span<const std::tuple<::tests::TestFlags, bool>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.contains(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(rusty::clone(rusty::clone(TestFlags::A)), [&]() -> std::span<const std::tuple<::tests::TestFlags, bool>> { static const std::array<std::tuple<::tests::TestFlags, bool>, 7> _slice_ref_tmp = {std::make_tuple(TestFlags::empty(), true), std::make_tuple(rusty::clone(rusty::clone(TestFlags::A)), true), std::make_tuple(rusty::clone(rusty::clone(TestFlags::B)), false), std::make_tuple(rusty::clone(rusty::clone(TestFlags::C)), false), std::make_tuple(rusty::clone(rusty::clone(TestFlags::ABC)), false), std::make_tuple(TestFlags::from_bits_retain(1 << 3), false), std::make_tuple(TestFlags::from_bits_retain(1 | ((1 << 3))), false)}; return std::span<const std::tuple<::tests::TestFlags, bool>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.contains(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(rusty::clone(rusty::clone(TestFlags::ABC)), [&]() -> std::span<const std::tuple<::tests::TestFlags, bool>> { static const std::array<std::tuple<::tests::TestFlags, bool>, 6> _slice_ref_tmp = {std::make_tuple(TestFlags::empty(), true), std::make_tuple(rusty::clone(rusty::clone(TestFlags::A)), true), std::make_tuple(rusty::clone(rusty::clone(TestFlags::B)), true), std::make_tuple(rusty::clone(rusty::clone(TestFlags::C)), true), std::make_tuple(rusty::clone(rusty::clone(TestFlags::ABC)), true), std::make_tuple(TestFlags::from_bits_retain(1 << 3), false)}; return std::span<const std::tuple<::tests::TestFlags, bool>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.contains(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(TestFlags::from_bits_retain(1 << 3), [&]() -> std::span<const std::tuple<::tests::TestFlags, bool>> { static const std::array<std::tuple<::tests::TestFlags, bool>, 5> _slice_ref_tmp = {std::make_tuple(TestFlags::empty(), true), std::make_tuple(rusty::clone(rusty::clone(TestFlags::A)), false), std::make_tuple(rusty::clone(rusty::clone(TestFlags::B)), false), std::make_tuple(rusty::clone(rusty::clone(TestFlags::C)), false), std::make_tuple(TestFlags::from_bits_retain(1 << 3), true)}; return std::span<const std::tuple<::tests::TestFlags, bool>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.contains(std::forward<decltype(_args)>(_args)...); });
            case_<TestZero>(rusty::clone(rusty::clone(TestZero::ZERO)), [&]() -> std::span<const std::tuple<::tests::TestZero, bool>> { static const std::array<std::tuple<::tests::TestZero, bool>, 1> _slice_ref_tmp = {std::make_tuple(rusty::clone(rusty::clone(TestZero::ZERO)), true)}; return std::span<const std::tuple<::tests::TestZero, bool>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.contains(std::forward<decltype(_args)>(_args)...); });
            case_<TestOverlapping>(rusty::clone(rusty::clone(TestOverlapping::AB)), [&]() -> std::span<const std::tuple<::tests::TestOverlapping, bool>> { static const std::array<std::tuple<::tests::TestOverlapping, bool>, 3> _slice_ref_tmp = {std::make_tuple(rusty::clone(rusty::clone(TestOverlapping::AB)), true), std::make_tuple(rusty::clone(rusty::clone(TestOverlapping::BC)), false), std::make_tuple(TestOverlapping::from_bits_retain(1 << 1), true)}; return std::span<const std::tuple<::tests::TestOverlapping, bool>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.contains(std::forward<decltype(_args)>(_args)...); });
            case_<TestExternal>(TestExternal::all(), [&]() -> std::span<const std::tuple<::tests::TestExternal, bool>> { static const std::array<std::tuple<::tests::TestExternal, bool>, 4> _slice_ref_tmp = {std::make_tuple(rusty::clone(rusty::clone(TestExternal::A)), true), std::make_tuple(rusty::clone(rusty::clone(TestExternal::B)), true), std::make_tuple(rusty::clone(rusty::clone(TestExternal::C)), true), std::make_tuple(TestExternal::from_bits_retain((1 << 5) | (1 << 7)), true)}; return std::span<const std::tuple<::tests::TestExternal, bool>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.contains(std::forward<decltype(_args)>(_args)...); });
        }

        template<typename T>
        void case_(T value, std::span<const std::tuple<T, bool>> inputs, const auto& inherent) {
            for (auto&& _for_item : rusty::for_in(rusty::iter(inputs))) {
                auto&& input = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_for_item)));
                auto&& expected = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_for_item)));
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = inherent(value, rusty::detail::deref_if_pointer_like(input));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0}.contains({1})", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = value.contains(rusty::detail::deref_if_pointer_like(input));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::contains({0}, {1})", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
            }
        }

    }

    namespace difference {

        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent);

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::difference::cases, should_panic: no)

        void cases() {
            case_<TestFlags>(rusty::clone(TestFlags::A) | rusty::clone(TestFlags::B), [&]() -> std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>> { static const std::array<std::tuple<::tests::TestFlags, TestFlags::Bits>, 3> _slice_ref_tmp = {std::tuple<::tests::TestFlags, TestFlags::Bits>{rusty::clone(rusty::clone(TestFlags::A)), 1 << 1}, std::tuple<::tests::TestFlags, TestFlags::Bits>{rusty::clone(rusty::clone(TestFlags::B)), 1}, std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::from_bits_retain(1 << 3), 1 | (1 << 1)}}; return std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.difference(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(TestFlags::from_bits_retain(1 | (1 << 3)), [&]() -> std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>> { static const std::array<std::tuple<::tests::TestFlags, TestFlags::Bits>, 2> _slice_ref_tmp = {std::tuple<::tests::TestFlags, TestFlags::Bits>{rusty::clone(rusty::clone(TestFlags::A)), 1 << 3}, std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::from_bits_retain(1 << 3), 1}}; return std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.difference(std::forward<decltype(_args)>(_args)...); });
            case_<TestExternal>(TestExternal::from_bits_retain(~static_cast<int32_t>(0)), [&]() -> std::span<const std::tuple<::tests::TestExternal, TestExternal::Bits>> { static const std::array<std::tuple<::tests::TestExternal, TestExternal::Bits>, 1> _slice_ref_tmp = {std::tuple<::tests::TestExternal, TestExternal::Bits>{rusty::clone(rusty::clone(TestExternal::A)), 254}}; return std::span<const std::tuple<::tests::TestExternal, TestExternal::Bits>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.difference(std::forward<decltype(_args)>(_args)...); });
            {
                auto&& _m0_tmp = static_cast<int32_t>(254);
                auto _m0 = &_m0_tmp;
                auto&& _m1_tmp = (TestExternal::from_bits_retain(~static_cast<int32_t>(0)) & !rusty::clone(TestExternal::A)).bits();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(left_val == right_val)) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                    }
                    _m_matched = true;
                }
            }
            {
                auto&& _m0_tmp = static_cast<int32_t>(254);
                auto _m0 = &_m0_tmp;
                auto&& _m1_tmp = (TestFlags::from_bits_retain(~static_cast<int32_t>(0)).difference(rusty::clone(rusty::clone(TestFlags::A)))).bits();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                    }
                    _m_matched = true;
                }
            }
            {
                auto&& _m0_tmp = (1 << 1) | (1 << 2);
                auto _m0 = &_m0_tmp;
                auto&& _m1_tmp = (TestFlags::from_bits_retain(~static_cast<int32_t>(0)) & !rusty::clone(TestFlags::A)).bits();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(left_val == right_val)) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                    }
                    _m_matched = true;
                }
            }
        }

        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent) {
            for (auto&& _for_item : rusty::for_in(rusty::iter(inputs))) {
                auto&& input = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_for_item)));
                auto&& expected = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_for_item)));
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = inherent(std::move(value), rusty::detail::deref_if_pointer_like(input)).bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0}.difference({1})", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = (std::move(value)).difference(rusty::detail::deref_if_pointer_like(input)).bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::difference({0}, {1})", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = (value - rusty::detail::deref_if_pointer_like(input)).bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0} - {1}", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = [&]() { auto value_shadow1 = std::move(value);
[&]() { static_cast<void>(value_shadow1 -= rusty::detail::deref_if_pointer_like(input)); return std::make_tuple(); }();
return value_shadow1; }().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0} -= {1}", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
            }
        }

    }

    namespace eq {

        void cases();

        using namespace tests;


        // Rust-only libtest metadata const skipped: cases (marker: tests::eq::cases, should_panic: no)

        void cases() {
            {
                auto&& _m0_tmp = TestFlags::empty();
                auto _m0 = &_m0_tmp;
                auto&& _m1_tmp = TestFlags::empty();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(left_val == right_val)) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                    }
                    _m_matched = true;
                }
            }
            {
                auto&& _m0_tmp = TestFlags::all();
                auto _m0 = &_m0_tmp;
                auto&& _m1_tmp = TestFlags::all();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(left_val == right_val)) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                    }
                    _m_matched = true;
                }
            }
            if (!(TestFlags::from_bits_retain(static_cast<uint8_t>(1)) < TestFlags::from_bits_retain(static_cast<uint8_t>(2)))) {
                rusty::panicking::panic("assertion failed: TestFlags::from_bits_retain(1) < TestFlags::from_bits_retain(2)");
            }
            if (!(TestFlags::from_bits_retain(static_cast<uint8_t>(2)) > TestFlags::from_bits_retain(static_cast<uint8_t>(1)))) {
                rusty::panicking::panic("assertion failed: TestFlags::from_bits_retain(2) > TestFlags::from_bits_retain(1)");
            }
        }

    }

    namespace extend {
        namespace external {}

        namespace external {
            void cases();
        }
        void cases();

        using namespace tests;


        // Rust-only libtest metadata const skipped: cases (marker: tests::extend::cases, should_panic: no)

        namespace external {

            void cases();

            using namespace ::tests::extend;


            // Rust-only libtest metadata const skipped: cases (marker: tests::extend::external::cases, should_panic: no)

            void cases() {
                auto flags = TestExternal::empty();
                flags.extend(rusty::clone(rusty::clone(TestExternal::A)));
                {
                    auto&& _m0_tmp = rusty::clone(TestExternal::A);
                    auto _m0 = &_m0_tmp;
                    auto _m1 = &flags;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                flags.extend((rusty::clone(TestExternal::A) | rusty::clone(TestExternal::B)) | rusty::clone(TestExternal::C));
                {
                    auto&& _m0_tmp = rusty::clone(TestExternal::ABC);
                    auto _m0 = &_m0_tmp;
                    auto _m1 = &flags;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                flags.extend(TestExternal::from_bits_retain(1 << 5));
                {
                    auto&& _m0_tmp = rusty::clone(TestExternal::ABC) | TestExternal::from_bits_retain(1 << 5);
                    auto _m0 = &_m0_tmp;
                    auto _m1 = &flags;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
            }

        }

        void cases() {
            auto flags = TestFlags::empty();
            flags.extend(rusty::clone(rusty::clone(TestFlags::A)));
            {
                auto&& _m0_tmp = rusty::clone(TestFlags::A);
                auto _m0 = &_m0_tmp;
                auto _m1 = &flags;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                    }
                    _m_matched = true;
                }
            }
            flags.extend((rusty::clone(TestFlags::A) | rusty::clone(TestFlags::B)) | rusty::clone(TestFlags::C));
            {
                auto&& _m0_tmp = rusty::clone(TestFlags::ABC);
                auto _m0 = &_m0_tmp;
                auto _m1 = &flags;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                    }
                    _m_matched = true;
                }
            }
            flags.extend(TestFlags::from_bits_retain(1 << 5));
            {
                auto&& _m0_tmp = rusty::clone(TestFlags::ABC) | TestFlags::from_bits_retain(1 << 5);
                auto _m0 = &_m0_tmp;
                auto _m1 = &flags;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(left_val == right_val)) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                    }
                    _m_matched = true;
                }
            }
        }

    }

    namespace fmt {

        void cases();
        template<typename T>
        void case_(T value, std::string_view debug, std::string_view uhex, std::string_view lhex, std::string_view oct, std::string_view bin);

        using namespace tests;


        // Rust-only libtest metadata const skipped: cases (marker: tests::fmt::cases, should_panic: no)

        void cases() {
            case_(TestFlags::empty(), std::string_view("TestFlags(0x0)"), std::string_view("0"), std::string_view("0"), std::string_view("0"), std::string_view("0"));
            case_(rusty::clone(rusty::clone(TestFlags::A)), std::string_view("TestFlags(A)"), std::string_view("1"), std::string_view("1"), std::string_view("1"), std::string_view("1"));
            case_(TestFlags::all(), std::string_view("TestFlags(A | B | C)"), std::string_view("7"), std::string_view("7"), std::string_view("7"), std::string_view("111"));
            case_(TestFlags::from_bits_retain(1 << 3), std::string_view("TestFlags(0x8)"), std::string_view("8"), std::string_view("8"), std::string_view("10"), std::string_view("1000"));
            case_(rusty::clone(TestFlags::A) | TestFlags::from_bits_retain(1 << 3), std::string_view("TestFlags(A | 0x8)"), std::string_view("9"), std::string_view("9"), std::string_view("11"), std::string_view("1001"));
            case_(rusty::clone(rusty::clone(TestZero::ZERO)), std::string_view("TestZero(0x0)"), std::string_view("0"), std::string_view("0"), std::string_view("0"), std::string_view("0"));
            case_(rusty::clone(TestZero::ZERO) | TestZero::from_bits_retain(static_cast<uint8_t>(1)), std::string_view("TestZero(0x1)"), std::string_view("1"), std::string_view("1"), std::string_view("1"), std::string_view("1"));
            case_(rusty::clone(rusty::clone(TestZeroOne::ONE)), std::string_view("TestZeroOne(ONE)"), std::string_view("1"), std::string_view("1"), std::string_view("1"), std::string_view("1"));
            case_(TestOverlapping::from_bits_retain(1 << 1), std::string_view("TestOverlapping(0x2)"), std::string_view("2"), std::string_view("2"), std::string_view("2"), std::string_view("10"));
            case_(TestExternal::from_bits_retain((1 | (1 << 1)) | (1 << 3)), std::string_view("TestExternal(A | B | 0x8)"), std::string_view("B"), std::string_view("b"), std::string_view("13"), std::string_view("1011"));
            case_(TestExternal::all(), std::string_view("TestExternal(A | B | C | 0xf8)"), std::string_view("FF"), std::string_view("ff"), std::string_view("377"), std::string_view("11111111"));
            case_(TestExternalFull::all(), std::string_view("TestExternalFull(0xff)"), std::string_view("FF"), std::string_view("ff"), std::string_view("377"), std::string_view("11111111"));
        }

        template<typename T>
        void case_(T value, std::string_view debug, std::string_view uhex, std::string_view lhex, std::string_view oct, std::string_view bin) {
            {
                auto&& _m0_tmp = std::string_view(debug);
                auto _m0 = &_m0_tmp;
                auto&& _m1_tmp = rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0}", rusty::to_debug_string(value))));
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                    }
                    _m_matched = true;
                }
            }
            {
                auto&& _m0_tmp = std::string_view(uhex);
                auto _m0 = &_m0_tmp;
                auto&& _m1_tmp = rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0:X}", rusty::format_numeric_arg(value))));
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                    }
                    _m_matched = true;
                }
            }
            {
                auto&& _m0_tmp = std::string_view(lhex);
                auto _m0 = &_m0_tmp;
                auto&& _m1_tmp = rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0:x}", rusty::format_numeric_arg(value))));
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                    }
                    _m_matched = true;
                }
            }
            {
                auto&& _m0_tmp = std::string_view(oct);
                auto _m0 = &_m0_tmp;
                auto&& _m1_tmp = rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0:o}", rusty::format_numeric_arg(value))));
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                    }
                    _m_matched = true;
                }
            }
            {
                auto&& _m0_tmp = std::string_view(bin);
                auto _m0 = &_m0_tmp;
                auto&& _m1_tmp = rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0:b}", rusty::format_numeric_arg(value))));
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                    }
                    _m_matched = true;
                }
            }
        }

    }

    namespace from_bits_truncate {

        void cases();
        template<typename T>
        void case_(typename T::Bits expected, typename T::Bits input, const auto& inherent);

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::from_bits_truncate::cases, should_panic: no)

        void cases() {
            case_<TestFlags>(0, 0, TestFlags::from_bits_truncate);
            case_<TestFlags>(1, 1, TestFlags::from_bits_truncate);
            case_<TestFlags>((1 | (1 << 1)) | (1 << 2), (1 | (1 << 1)) | (1 << 2), TestFlags::from_bits_truncate);
            case_<TestFlags>(0, 1 << 3, TestFlags::from_bits_truncate);
            case_<TestFlags>(1, 1 | (1 << 3), TestFlags::from_bits_truncate);
            case_<TestOverlapping>(1 | (1 << 1), 1 | (1 << 1), TestOverlapping::from_bits_truncate);
            case_<TestOverlapping>(1 << 1, 1 << 1, TestOverlapping::from_bits_truncate);
            case_<TestExternal>(1 << 5, 1 << 5, TestExternal::from_bits_truncate);
        }

        template<typename T>
        void case_(typename T::Bits expected, typename T::Bits input, const auto& inherent) {
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = inherent(std::move(input)).bits();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("T::from_bits_truncate({0})", rusty::to_debug_string(input))));
                    }
                    _m_matched = true;
                }
            }
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = T::from_bits_truncate(std::move(input)).bits();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::from_bits_truncate({0})", rusty::to_debug_string(input))));
                    }
                    _m_matched = true;
                }
            }
        }

    }

    namespace from_name {

        void cases();
        template<typename T>
        void case_(rusty::Option<typename T::Bits> expected, std::string_view input, const auto& inherent);

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::from_name::cases, should_panic: no)

        void cases() {
            case_<TestFlags>(rusty::Option<TestFlags::Bits>(1), std::string_view("A"), TestFlags::from_name);
            case_<TestFlags>(rusty::Option<TestFlags::Bits>(1 << 1), std::string_view("B"), TestFlags::from_name);
            case_<TestFlags>(rusty::Option<TestFlags::Bits>((1 | (1 << 1)) | (1 << 2)), std::string_view("ABC"), TestFlags::from_name);
            case_<TestFlags>(rusty::Option<TestFlags::Bits>(rusty::None), std::string_view(""), TestFlags::from_name);
            case_<TestFlags>(rusty::Option<TestFlags::Bits>(rusty::None), std::string_view("a"), TestFlags::from_name);
            case_<TestFlags>(rusty::Option<TestFlags::Bits>(rusty::None), std::string_view("0x1"), TestFlags::from_name);
            case_<TestFlags>(rusty::Option<TestFlags::Bits>(rusty::None), std::string_view("A | B"), TestFlags::from_name);
            case_<TestZero>(rusty::Option<TestZero::Bits>(0), std::string_view("ZERO"), TestZero::from_name);
            case_<TestUnicode>(rusty::Option<TestUnicode::Bits>(2), std::string_view("二"), TestUnicode::from_name);
            case_<TestExternal>(rusty::Option<TestExternal::Bits>(rusty::None), std::string_view("_"), TestExternal::from_name);
            case_<TestExternal>(rusty::Option<TestExternal::Bits>(rusty::None), std::string_view(""), TestExternal::from_name);
        }

        template<typename T>
        void case_(rusty::Option<typename T::Bits> expected, std::string_view input, const auto& inherent) {
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = inherent(input).map([&](auto&& f) -> typename T::Bits { return f.bits(); });
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("T::from_name({0})", rusty::to_debug_string(input))));
                    }
                    _m_matched = true;
                }
            }
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = T::from_name(input).map([&](auto&& f) -> typename T::Bits { return f.bits(); });
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::from_name({0})", rusty::to_debug_string(input))));
                    }
                    _m_matched = true;
                }
            }
        }

    }

    namespace insert {

        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent_insert, const auto& inherent_set);

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::insert::cases, should_panic: no)

        void cases() {
            case_<TestFlags>(TestFlags::empty(), [&]() -> std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>> { static const std::array<std::tuple<::tests::TestFlags, TestFlags::Bits>, 4> _slice_ref_tmp = {std::tuple<::tests::TestFlags, TestFlags::Bits>{rusty::clone(rusty::clone(TestFlags::A)), 1}, std::tuple<::tests::TestFlags, TestFlags::Bits>{rusty::clone(TestFlags::A) | rusty::clone(TestFlags::B), 1 | (1 << 1)}, std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::empty(), 0}, std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::from_bits_retain(1 << 3), 1 << 3}}; return std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.insert(std::forward<decltype(_args)>(_args)...); }, [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.set(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(rusty::clone(rusty::clone(TestFlags::A)), [&]() -> std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>> { static const std::array<std::tuple<::tests::TestFlags, TestFlags::Bits>, 3> _slice_ref_tmp = {std::tuple<::tests::TestFlags, TestFlags::Bits>{rusty::clone(rusty::clone(TestFlags::A)), 1}, std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::empty(), 1}, std::tuple<::tests::TestFlags, TestFlags::Bits>{rusty::clone(rusty::clone(TestFlags::B)), 1 | (1 << 1)}}; return std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.insert(std::forward<decltype(_args)>(_args)...); }, [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.set(std::forward<decltype(_args)>(_args)...); });
        }

        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent_insert, const auto& inherent_set) {
            for (auto&& _for_item : rusty::for_in(rusty::iter(inputs))) {
                auto&& input = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_for_item)));
                auto&& expected = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_for_item)));
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = [&]() { auto value_shadow1 = std::move(value);
inherent_insert(value_shadow1, rusty::detail::deref_if_pointer_like(input));
return value_shadow1; }().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0}.insert({1})", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = [&]() { auto value_shadow1 = std::move(value);
value_shadow1.insert(rusty::detail::deref_if_pointer_like(input));
return value_shadow1; }().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::insert({0}, {1})", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = [&]() { auto value_shadow1 = std::move(value);
inherent_set(value_shadow1, rusty::detail::deref_if_pointer_like(input), true);
return value_shadow1; }().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0}.set({1}, true)", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = [&]() { auto value_shadow1 = std::move(value);
value_shadow1.set(rusty::detail::deref_if_pointer_like(input), true);
return value_shadow1; }().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::set({0}, {1}, true)", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
            }
        }

    }

    namespace intersection {

        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent);

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::intersection::cases, should_panic: no)

        void cases() {
            case_<TestFlags>(TestFlags::empty(), [&]() -> std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>> { static const std::array<std::tuple<::tests::TestFlags, TestFlags::Bits>, 2> _slice_ref_tmp = {std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::empty(), 0}, std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::all(), 0}}; return std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.intersection(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(TestFlags::all(), [&]() -> std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>> { static const std::array<std::tuple<::tests::TestFlags, TestFlags::Bits>, 3> _slice_ref_tmp = {std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::all(), (1 | (1 << 1)) | (1 << 2)}, std::tuple<::tests::TestFlags, TestFlags::Bits>{rusty::clone(rusty::clone(TestFlags::A)), 1}, std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::from_bits_retain(1 << 3), 0}}; return std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.intersection(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(TestFlags::from_bits_retain(1 << 3), [&]() -> std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>> { static const std::array<std::tuple<::tests::TestFlags, TestFlags::Bits>, 1> _slice_ref_tmp = {std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::from_bits_retain(1 << 3), 1 << 3}}; return std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.intersection(std::forward<decltype(_args)>(_args)...); });
            case_<TestOverlapping>(rusty::clone(rusty::clone(TestOverlapping::AB)), [&]() -> std::span<const std::tuple<::tests::TestOverlapping, TestOverlapping::Bits>> { static const std::array<std::tuple<::tests::TestOverlapping, TestOverlapping::Bits>, 1> _slice_ref_tmp = {std::tuple<::tests::TestOverlapping, TestOverlapping::Bits>{rusty::clone(rusty::clone(TestOverlapping::BC)), 1 << 1}}; return std::span<const std::tuple<::tests::TestOverlapping, TestOverlapping::Bits>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.intersection(std::forward<decltype(_args)>(_args)...); });
        }

        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent) {
            for (auto&& _for_item : rusty::for_in(rusty::iter(inputs))) {
                auto&& input = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_for_item)));
                auto&& expected = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_for_item)));
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = inherent(std::move(value), rusty::detail::deref_if_pointer_like(input)).bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0}.intersection({1})", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = (std::move(value)).intersection(rusty::detail::deref_if_pointer_like(input)).bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::intersection({0}, {1})", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = (value & rusty::detail::deref_if_pointer_like(input)).bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0} & {1}", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = [&]() { auto value_shadow1 = std::move(value);
[&]() { static_cast<void>(value_shadow1 &= rusty::detail::deref_if_pointer_like(input)); return std::make_tuple(); }();
return value_shadow1; }().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0} &= {1}", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
            }
        }

    }

    namespace intersects {

        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, bool>> inputs, const auto& inherent);

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::intersects::cases, should_panic: no)

        void cases() {
            case_<TestFlags>(TestFlags::empty(), [&]() -> std::span<const std::tuple<::tests::TestFlags, bool>> { static const std::array<std::tuple<::tests::TestFlags, bool>, 5> _slice_ref_tmp = {std::make_tuple(TestFlags::empty(), false), std::make_tuple(rusty::clone(rusty::clone(TestFlags::A)), false), std::make_tuple(rusty::clone(rusty::clone(TestFlags::B)), false), std::make_tuple(rusty::clone(rusty::clone(TestFlags::C)), false), std::make_tuple(TestFlags::from_bits_retain(1 << 3), false)}; return std::span<const std::tuple<::tests::TestFlags, bool>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.intersects(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(rusty::clone(rusty::clone(TestFlags::A)), [&]() -> std::span<const std::tuple<::tests::TestFlags, bool>> { static const std::array<std::tuple<::tests::TestFlags, bool>, 7> _slice_ref_tmp = {std::make_tuple(TestFlags::empty(), false), std::make_tuple(rusty::clone(rusty::clone(TestFlags::A)), true), std::make_tuple(rusty::clone(rusty::clone(TestFlags::B)), false), std::make_tuple(rusty::clone(rusty::clone(TestFlags::C)), false), std::make_tuple(rusty::clone(rusty::clone(TestFlags::ABC)), true), std::make_tuple(TestFlags::from_bits_retain(1 << 3), false), std::make_tuple(TestFlags::from_bits_retain(1 | ((1 << 3))), true)}; return std::span<const std::tuple<::tests::TestFlags, bool>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.intersects(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(rusty::clone(rusty::clone(TestFlags::ABC)), [&]() -> std::span<const std::tuple<::tests::TestFlags, bool>> { static const std::array<std::tuple<::tests::TestFlags, bool>, 6> _slice_ref_tmp = {std::make_tuple(TestFlags::empty(), false), std::make_tuple(rusty::clone(rusty::clone(TestFlags::A)), true), std::make_tuple(rusty::clone(rusty::clone(TestFlags::B)), true), std::make_tuple(rusty::clone(rusty::clone(TestFlags::C)), true), std::make_tuple(rusty::clone(rusty::clone(TestFlags::ABC)), true), std::make_tuple(TestFlags::from_bits_retain(1 << 3), false)}; return std::span<const std::tuple<::tests::TestFlags, bool>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.intersects(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(TestFlags::from_bits_retain(1 << 3), [&]() -> std::span<const std::tuple<::tests::TestFlags, bool>> { static const std::array<std::tuple<::tests::TestFlags, bool>, 5> _slice_ref_tmp = {std::make_tuple(TestFlags::empty(), false), std::make_tuple(rusty::clone(rusty::clone(TestFlags::A)), false), std::make_tuple(rusty::clone(rusty::clone(TestFlags::B)), false), std::make_tuple(rusty::clone(rusty::clone(TestFlags::C)), false), std::make_tuple(TestFlags::from_bits_retain(1 << 3), true)}; return std::span<const std::tuple<::tests::TestFlags, bool>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.intersects(std::forward<decltype(_args)>(_args)...); });
            case_<TestOverlapping>(rusty::clone(rusty::clone(TestOverlapping::AB)), [&]() -> std::span<const std::tuple<::tests::TestOverlapping, bool>> { static const std::array<std::tuple<::tests::TestOverlapping, bool>, 3> _slice_ref_tmp = {std::make_tuple(rusty::clone(rusty::clone(TestOverlapping::AB)), true), std::make_tuple(rusty::clone(rusty::clone(TestOverlapping::BC)), true), std::make_tuple(TestOverlapping::from_bits_retain(1 << 1), true)}; return std::span<const std::tuple<::tests::TestOverlapping, bool>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.intersects(std::forward<decltype(_args)>(_args)...); });
        }

        template<typename T>
        void case_(T value, std::span<const std::tuple<T, bool>> inputs, const auto& inherent) {
            for (auto&& _for_item : rusty::for_in(rusty::iter(inputs))) {
                auto&& input = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_for_item)));
                auto&& expected = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_for_item)));
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = inherent(value, rusty::detail::deref_if_pointer_like(input));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0}.intersects({1})", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = value.intersects(rusty::detail::deref_if_pointer_like(input));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::intersects({0}, {1})", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
            }
        }

    }

    namespace is_all {

        void cases();
        template<typename T>
        void case_(bool expected, T value, const auto& inherent);

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::is_all::cases, should_panic: no)

        void cases() {
            case_<TestFlags>(false, TestFlags::empty(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.is_all(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(false, rusty::clone(rusty::clone(TestFlags::A)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.is_all(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(true, rusty::clone(rusty::clone(TestFlags::ABC)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.is_all(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(true, rusty::clone(TestFlags::ABC) | TestFlags::from_bits_retain(1 << 3), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.is_all(std::forward<decltype(_args)>(_args)...); });
            case_<TestZero>(true, TestZero::empty(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.is_all(std::forward<decltype(_args)>(_args)...); });
            case_<TestEmpty>(true, TestEmpty::empty(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.is_all(std::forward<decltype(_args)>(_args)...); });
        }

        template<typename T>
        void case_(bool expected, T value, const auto& inherent) {
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = inherent(value);
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(left_val == right_val)) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::Some(std::format("{0}.is_all()", rusty::to_debug_string(value))));
                    }
                    _m_matched = true;
                }
            }
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = value.is_all();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::is_all({0})", rusty::to_debug_string(value))));
                    }
                    _m_matched = true;
                }
            }
        }

    }

    namespace is_empty {

        void cases();
        template<typename T>
        void case_(bool expected, T value, const auto& inherent);

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::is_empty::cases, should_panic: no)

        void cases() {
            case_<TestFlags>(true, TestFlags::empty(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.is_empty(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(false, rusty::clone(rusty::clone(TestFlags::A)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.is_empty(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(false, rusty::clone(rusty::clone(TestFlags::ABC)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.is_empty(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(false, TestFlags::from_bits_retain(1 << 3), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.is_empty(std::forward<decltype(_args)>(_args)...); });
            case_<TestZero>(true, TestZero::empty(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.is_empty(std::forward<decltype(_args)>(_args)...); });
            case_<TestEmpty>(true, TestEmpty::empty(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.is_empty(std::forward<decltype(_args)>(_args)...); });
        }

        template<typename T>
        void case_(bool expected, T value, const auto& inherent) {
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = inherent(value);
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(left_val == right_val)) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::Some(std::format("{0}.is_empty()", rusty::to_debug_string(value))));
                    }
                    _m_matched = true;
                }
            }
            {
                auto _m0 = &expected;
                auto&& _m1_tmp = value.is_empty();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::is_empty({0})", rusty::to_debug_string(value))));
                    }
                    _m_matched = true;
                }
            }
        }

    }

    namespace iter {
        namespace collect {}
        namespace iter {}
        namespace iter_names {}

        namespace collect {
            void cases();
        }
        namespace iter {
            void cases();
            template<typename T>
            void case_(std::span<const typename T::Bits> expected, T value, const auto& inherent);
        }
        namespace iter_names {
            void cases();
            template<typename T>
            void case_(std::span<const std::tuple<std::string_view, typename T::Bits>> expected, T value, const auto& inherent);
        }
        void roundtrip();

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: roundtrip (marker: tests::iter::roundtrip, should_panic: no)

        namespace collect {

            void cases();

            using namespace ::tests::iter;


            // Rust-only libtest metadata const skipped: cases (marker: tests::iter::collect::cases, should_panic: no)

            void cases() {
                {
                    auto&& _m0_tmp = static_cast<int32_t>(0);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::TestFlags::from_iter(rusty::iter(std::array<int, 0>{})).bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = static_cast<int32_t>(1);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::TestFlags::from_iter(rusty::iter(std::array{rusty::clone(rusty::clone(TestFlags::A))})).bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = (1 | (1 << 1)) | (1 << 2);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::TestFlags::from_iter(rusty::iter(std::array{rusty::clone(rusty::clone(TestFlags::A)), rusty::clone(TestFlags::B) | rusty::clone(TestFlags::C)})).bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = 1 | (1 << 3);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::TestFlags::from_iter(rusty::iter(std::array{TestFlags::from_bits_retain(1 << 3), TestFlags::empty(), rusty::clone(rusty::clone(TestFlags::A))})).bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = (1 << 5) | (1 << 7);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::TestExternal::from_iter(rusty::iter(std::array{TestExternal::empty(), TestExternal::from_bits_retain(1 << 5), TestExternal::from_bits_retain(1 << 7)})).bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
            }

        }

        namespace iter {

            void cases();
            template<typename T>
            void case_(std::span<const typename T::Bits> expected, T value, const auto& inherent);

            using namespace ::tests::iter;


            // Rust-only libtest metadata const skipped: cases (marker: tests::iter::iter::cases, should_panic: no)

            void cases() {
                case_<TestFlags>([&]() -> std::span<const TestFlags::Bits> { static const std::array<TestFlags::Bits, 0> _slice_ref_tmp = {}; return std::span<const TestFlags::Bits>(_slice_ref_tmp); }(), TestFlags::empty(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter(std::forward<decltype(_args)>(_args)...); });
                case_<TestFlags>([&]() -> std::span<const TestFlags::Bits> { static const std::array<TestFlags::Bits, 1> _slice_ref_tmp = {1}; return std::span<const TestFlags::Bits>(_slice_ref_tmp); }(), rusty::clone(rusty::clone(TestFlags::A)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter(std::forward<decltype(_args)>(_args)...); });
                case_<TestFlags>([&]() -> std::span<const TestFlags::Bits> { static const std::array<TestFlags::Bits, 2> _slice_ref_tmp = {1, 1 << 1}; return std::span<const TestFlags::Bits>(_slice_ref_tmp); }(), rusty::clone(TestFlags::A) | rusty::clone(TestFlags::B), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter(std::forward<decltype(_args)>(_args)...); });
                case_<TestFlags>([&]() -> std::span<const TestFlags::Bits> { static const std::array<TestFlags::Bits, 3> _slice_ref_tmp = {1, 1 << 1, 1 << 3}; return std::span<const TestFlags::Bits>(_slice_ref_tmp); }(), (rusty::clone(TestFlags::A) | rusty::clone(TestFlags::B)) | TestFlags::from_bits_retain(1 << 3), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter(std::forward<decltype(_args)>(_args)...); });
                case_<TestFlags>([&]() -> std::span<const TestFlags::Bits> { static const std::array<TestFlags::Bits, 3> _slice_ref_tmp = {1, 1 << 1, 1 << 2}; return std::span<const TestFlags::Bits>(_slice_ref_tmp); }(), rusty::clone(rusty::clone(TestFlags::ABC)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter(std::forward<decltype(_args)>(_args)...); });
                case_<TestFlags>([&]() -> std::span<const TestFlags::Bits> { static const std::array<TestFlags::Bits, 4> _slice_ref_tmp = {1, 1 << 1, 1 << 2, 1 << 3}; return std::span<const TestFlags::Bits>(_slice_ref_tmp); }(), rusty::clone(TestFlags::ABC) | TestFlags::from_bits_retain(1 << 3), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter(std::forward<decltype(_args)>(_args)...); });
                case_<TestFlagsInvert>([&]() -> std::span<const TestFlagsInvert::Bits> { static const std::array<TestFlagsInvert::Bits, 1> _slice_ref_tmp = {(1 | (1 << 1)) | (1 << 2)}; return std::span<const TestFlagsInvert::Bits>(_slice_ref_tmp); }(), rusty::clone(rusty::clone(TestFlagsInvert::ABC)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter(std::forward<decltype(_args)>(_args)...); });
                case_<TestZero>([&]() -> std::span<const TestZero::Bits> { static const std::array<TestZero::Bits, 0> _slice_ref_tmp = {}; return std::span<const TestZero::Bits>(_slice_ref_tmp); }(), rusty::clone(rusty::clone(TestZero::ZERO)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter(std::forward<decltype(_args)>(_args)...); });
                case_<TestExternal>([&]() -> std::span<const TestExternal::Bits> { static const std::array<TestExternal::Bits, 4> _slice_ref_tmp = {1, 1 << 1, 1 << 2, 248}; return std::span<const TestExternal::Bits>(_slice_ref_tmp); }(), TestExternal::all(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter(std::forward<decltype(_args)>(_args)...); });
            }

            template<typename T>
            void case_(std::span<const typename T::Bits> expected, T value, const auto& inherent) {
                {
                    auto _m0 = &expected;
                    auto&& _m1_tmp = rusty::collect_range(rusty::map(inherent(value), [&](auto&& f) { return f.bits(); }));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0}.iter()", rusty::to_debug_string(value))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto _m0 = &expected;
                    auto&& _m1_tmp = rusty::collect_range(rusty::map(value.iter(), [&](auto&& f) { return f.bits(); }));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::iter({0})", rusty::to_debug_string(value))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto _m0 = &expected;
                    auto&& _m1_tmp = rusty::collect_range(rusty::map(rusty::iter(std::move(value)), [&](auto&& f) { return f.bits(); }));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0}.into_iter()", rusty::to_debug_string(value))));
                        }
                        _m_matched = true;
                    }
                }
            }

        }

        namespace iter_names {

            void cases();
            template<typename T>
            void case_(std::span<const std::tuple<std::string_view, typename T::Bits>> expected, T value, const auto& inherent);

            using namespace ::tests::iter;


            // Rust-only libtest metadata const skipped: cases (marker: tests::iter::iter_names::cases, should_panic: no)

            void cases() {
                case_<TestFlags>([&]() -> std::span<const std::tuple<std::string_view, TestFlags::Bits>> { static const std::array<std::tuple<std::string_view, TestFlags::Bits>, 0> _slice_ref_tmp = {}; return std::span<const std::tuple<std::string_view, TestFlags::Bits>>(_slice_ref_tmp); }(), TestFlags::empty(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter_names(std::forward<decltype(_args)>(_args)...); });
                case_<TestFlags>([&]() -> std::span<const std::tuple<std::string_view, TestFlags::Bits>> { static const std::array<std::tuple<std::string_view, TestFlags::Bits>, 1> _slice_ref_tmp = {std::tuple<std::string_view, TestFlags::Bits>{std::string_view("A"), 1}}; return std::span<const std::tuple<std::string_view, TestFlags::Bits>>(_slice_ref_tmp); }(), rusty::clone(rusty::clone(TestFlags::A)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter_names(std::forward<decltype(_args)>(_args)...); });
                case_<TestFlags>([&]() -> std::span<const std::tuple<std::string_view, TestFlags::Bits>> { static const std::array<std::tuple<std::string_view, TestFlags::Bits>, 2> _slice_ref_tmp = {std::tuple<std::string_view, TestFlags::Bits>{std::string_view("A"), 1}, std::tuple<std::string_view, TestFlags::Bits>{std::string_view("B"), 1 << 1}}; return std::span<const std::tuple<std::string_view, TestFlags::Bits>>(_slice_ref_tmp); }(), rusty::clone(TestFlags::A) | rusty::clone(TestFlags::B), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter_names(std::forward<decltype(_args)>(_args)...); });
                case_<TestFlags>([&]() -> std::span<const std::tuple<std::string_view, TestFlags::Bits>> { static const std::array<std::tuple<std::string_view, TestFlags::Bits>, 2> _slice_ref_tmp = {std::tuple<std::string_view, TestFlags::Bits>{std::string_view("A"), 1}, std::tuple<std::string_view, TestFlags::Bits>{std::string_view("B"), 1 << 1}}; return std::span<const std::tuple<std::string_view, TestFlags::Bits>>(_slice_ref_tmp); }(), (rusty::clone(TestFlags::A) | rusty::clone(TestFlags::B)) | TestFlags::from_bits_retain(1 << 3), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter_names(std::forward<decltype(_args)>(_args)...); });
                case_<TestFlags>([&]() -> std::span<const std::tuple<std::string_view, TestFlags::Bits>> { static const std::array<std::tuple<std::string_view, TestFlags::Bits>, 3> _slice_ref_tmp = {std::tuple<std::string_view, TestFlags::Bits>{std::string_view("A"), 1}, std::tuple<std::string_view, TestFlags::Bits>{std::string_view("B"), 1 << 1}, std::tuple<std::string_view, TestFlags::Bits>{std::string_view("C"), 1 << 2}}; return std::span<const std::tuple<std::string_view, TestFlags::Bits>>(_slice_ref_tmp); }(), rusty::clone(rusty::clone(TestFlags::ABC)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter_names(std::forward<decltype(_args)>(_args)...); });
                case_<TestFlags>([&]() -> std::span<const std::tuple<std::string_view, TestFlags::Bits>> { static const std::array<std::tuple<std::string_view, TestFlags::Bits>, 3> _slice_ref_tmp = {std::tuple<std::string_view, TestFlags::Bits>{std::string_view("A"), 1}, std::tuple<std::string_view, TestFlags::Bits>{std::string_view("B"), 1 << 1}, std::tuple<std::string_view, TestFlags::Bits>{std::string_view("C"), 1 << 2}}; return std::span<const std::tuple<std::string_view, TestFlags::Bits>>(_slice_ref_tmp); }(), rusty::clone(TestFlags::ABC) | TestFlags::from_bits_retain(1 << 3), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter_names(std::forward<decltype(_args)>(_args)...); });
                case_<TestFlagsInvert>([&]() -> std::span<const std::tuple<std::string_view, TestFlagsInvert::Bits>> { static const std::array<std::tuple<std::string_view, TestFlagsInvert::Bits>, 1> _slice_ref_tmp = {std::tuple<std::string_view, TestFlagsInvert::Bits>{std::string_view("ABC"), (1 | (1 << 1)) | (1 << 2)}}; return std::span<const std::tuple<std::string_view, TestFlagsInvert::Bits>>(_slice_ref_tmp); }(), rusty::clone(rusty::clone(TestFlagsInvert::ABC)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter_names(std::forward<decltype(_args)>(_args)...); });
                case_<TestZero>([&]() -> std::span<const std::tuple<std::string_view, TestZero::Bits>> { static const std::array<std::tuple<std::string_view, TestZero::Bits>, 0> _slice_ref_tmp = {}; return std::span<const std::tuple<std::string_view, TestZero::Bits>>(_slice_ref_tmp); }(), rusty::clone(rusty::clone(TestZero::ZERO)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter_names(std::forward<decltype(_args)>(_args)...); });
                case_<TestOverlappingFull>([&]() -> std::span<const std::tuple<std::string_view, TestOverlappingFull::Bits>> { static const std::array<std::tuple<std::string_view, TestOverlappingFull::Bits>, 1> _slice_ref_tmp = {std::tuple<std::string_view, TestOverlappingFull::Bits>{std::string_view("A"), 1}}; return std::span<const std::tuple<std::string_view, TestOverlappingFull::Bits>>(_slice_ref_tmp); }(), rusty::clone(rusty::clone(TestOverlappingFull::A)), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter_names(std::forward<decltype(_args)>(_args)...); });
                case_<TestOverlappingFull>([&]() -> std::span<const std::tuple<std::string_view, TestOverlappingFull::Bits>> { static const std::array<std::tuple<std::string_view, TestOverlappingFull::Bits>, 2> _slice_ref_tmp = {std::tuple<std::string_view, TestOverlappingFull::Bits>{std::string_view("A"), 1}, std::tuple<std::string_view, TestOverlappingFull::Bits>{std::string_view("D"), 1 << 1}}; return std::span<const std::tuple<std::string_view, TestOverlappingFull::Bits>>(_slice_ref_tmp); }(), rusty::clone(TestOverlappingFull::A) | rusty::clone(TestOverlappingFull::D), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.iter_names(std::forward<decltype(_args)>(_args)...); });
            }

            template<typename T>
            void case_(std::span<const std::tuple<std::string_view, typename T::Bits>> expected, T value, const auto& inherent) {
                {
                    auto _m0 = &expected;
                    auto&& _m1_tmp = rusty::collect_range(rusty::map(inherent(value), [&](auto&& _destruct_param0) {
auto&& n = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& f = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(std::move(n), f.bits());
}));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0}.iter_names()", rusty::to_debug_string(value))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto _m0 = &expected;
                    auto&& _m1_tmp = rusty::collect_range(rusty::map(value.iter_names(), [&](auto&& _destruct_param0) {
auto&& n = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& f = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(std::move(n), f.bits());
}));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::iter_names({0})", rusty::to_debug_string(value))));
                        }
                        _m_matched = true;
                    }
                }
            }

        }

        void roundtrip() {
            for (auto&& a : rusty::for_in(rusty::range_inclusive(static_cast<uint8_t>(0), 255))) {
                for (auto&& b : rusty::for_in(rusty::range_inclusive(static_cast<uint8_t>(0), 255))) {
                    const auto f = TestFlags::from_bits_retain(a | b);
                    {
                        auto _m0 = &f;
                        auto&& _m1_tmp = ::tests::TestFlags::from_iter(rusty::iter(f));
                        auto _m1 = &_m1_tmp;
                        auto _m_tuple = std::make_tuple(_m0, _m1);
                        bool _m_matched = false;
                        if (!_m_matched) {
                            auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                            auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                            if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                                const auto kind = rusty::panicking::AssertKind::Eq;
                                rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                            }
                            _m_matched = true;
                        }
                    }
                    {
                        auto&& _m0_tmp = TestFlags::from_bits_truncate(f.bits());
                        auto _m0 = &_m0_tmp;
                        auto&& _m1_tmp = ::tests::TestFlags::from_iter(rusty::map(f.iter_names(), [&](auto&& _destruct_param0) {
auto&& f = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
return f;
}));
                        auto _m1 = &_m1_tmp;
                        auto _m_tuple = std::make_tuple(_m0, _m1);
                        bool _m_matched = false;
                        if (!_m_matched) {
                            auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                            auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                            if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                                const auto kind = rusty::panicking::AssertKind::Eq;
                                rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                            }
                            _m_matched = true;
                        }
                    }
                    const auto f_shadow1 = TestExternal::from_bits_retain(a | b);
                    {
                        auto _m0 = &f_shadow1;
                        auto&& _m1_tmp = ::tests::TestExternal::from_iter(rusty::iter(f_shadow1));
                        auto _m1 = &_m1_tmp;
                        auto _m_tuple = std::make_tuple(_m0, _m1);
                        bool _m_matched = false;
                        if (!_m_matched) {
                            auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                            auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                            if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                                const auto kind = rusty::panicking::AssertKind::Eq;
                                rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                            }
                            _m_matched = true;
                        }
                    }
                }
            }
        }

    }

    namespace parser {
        namespace from_str_tests {}
        namespace from_str_strict_tests {}
        namespace from_str_truncate_tests {}
        namespace to_writer_tests {}
        namespace to_writer_strict_tests {}
        namespace to_writer_truncate_tests {}

        namespace from_str_tests {
            void valid();
            void invalid();
        }
        namespace to_writer_tests {
            void cases();
            template<typename F>
            rusty::String write_(F value);
        }
        namespace from_str_truncate_tests {
            void valid();
        }
        namespace to_writer_truncate_tests {
            void cases();
            template<typename F>
            rusty::String write_(F value);
        }
        namespace from_str_strict_tests {
            void valid();
            void invalid();
        }
        namespace to_writer_strict_tests {
            void cases();
            template<typename F>
            rusty::String write_(F value);
        }
        void roundtrip();
        void roundtrip_truncate();
        void roundtrip_strict();

        using namespace tests;

        using namespace ::parser;
        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: roundtrip (marker: tests::parser::roundtrip, should_panic: no)


        // Rust-only libtest metadata const skipped: roundtrip_truncate (marker: tests::parser::roundtrip_truncate, should_panic: no)


        // Rust-only libtest metadata const skipped: roundtrip_strict (marker: tests::parser::roundtrip_strict, should_panic: no)

        namespace from_str_tests {

            void valid();
            void invalid();

            using namespace ::tests::parser;


            // Rust-only libtest metadata const skipped: valid (marker: tests::parser::from_str::valid, should_panic: no)


            // Rust-only libtest metadata const skipped: invalid (marker: tests::parser::from_str::invalid, should_panic: no)

            void valid() {
                {
                    auto&& _m0_tmp = static_cast<int32_t>(0);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str<::tests::TestFlags>(rusty::to_string_view("")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = static_cast<int32_t>(1);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str<::tests::TestFlags>(rusty::to_string_view("A")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = static_cast<int32_t>(1);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str<::tests::TestFlags>(rusty::to_string_view(" A ")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = (1 | (1 << 1)) | (1 << 2);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str<::tests::TestFlags>(rusty::to_string_view("A | B | C")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = (1 | (1 << 1)) | (1 << 2);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str<::tests::TestFlags>(rusty::to_string_view("A\n|\tB\r\n|   C ")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = (1 | (1 << 1)) | (1 << 2);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str<::tests::TestFlags>(rusty::to_string_view("A|B|C")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = 1 << 3;
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str<::tests::TestFlags>(rusty::to_string_view("0x8")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = 1 | (1 << 3);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str<::tests::TestFlags>(rusty::to_string_view("A | 0x8")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = (1 | (1 << 1)) | (1 << 3);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str<::tests::TestFlags>(rusty::to_string_view("0x1 | 0x8 | B")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = 1 | (1 << 1);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str<::tests::TestUnicode>(rusty::to_string_view("一 | 二")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
            }

            void invalid() {
                if (!rusty::starts_with(rusty::to_string(::parser::from_str<::tests::TestFlags>(rusty::to_string_view("a")).unwrap_err()), "unrecognized named flag")) {
                    rusty::panicking::panic("assertion failed: from_str::<TestFlags>(\"a\").unwrap_err().to_string().starts_with(\"unrecognized named flag\")");
                }
                if (!rusty::starts_with(rusty::to_string(::parser::from_str<::tests::TestFlags>(rusty::to_string_view("A & B")).unwrap_err()), "unrecognized named flag")) {
                    rusty::panicking::panic("assertion failed: from_str::<TestFlags>(\"A & B\").unwrap_err().to_string().starts_with(\"unrecognized named flag\")");
                }
                if (!rusty::starts_with(rusty::to_string(::parser::from_str<::tests::TestFlags>(rusty::to_string_view("0xg")).unwrap_err()), "invalid hex flag")) {
                    rusty::panicking::panic("assertion failed: from_str::<TestFlags>(\"0xg\").unwrap_err().to_string().starts_with(\"invalid hex flag\")");
                }
                if (!rusty::starts_with(rusty::to_string(::parser::from_str<::tests::TestFlags>(rusty::to_string_view("0xffffffffffff")).unwrap_err()), "invalid hex flag")) {
                    rusty::panicking::panic("assertion failed: from_str::<TestFlags>(\"0xffffffffffff\").unwrap_err().to_string().starts_with(\"invalid hex flag\")");
                }
            }

        }

        namespace to_writer_tests {

            void cases();
            template<typename F>
            rusty::String write_(F value);

            using namespace ::tests::parser;


            // Rust-only libtest metadata const skipped: cases (marker: tests::parser::to_writer::cases, should_panic: no)

            void cases() {
                {
                    auto&& _m0_tmp = std::string_view("");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_tests::write_(TestFlags::empty());
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("A");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_tests::write_<TestFlags>(rusty::clone(rusty::clone(TestFlags::A)));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("A | B | C");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_tests::write_(TestFlags::all());
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("0x8");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_tests::write_(TestFlags::from_bits_retain(1 << 3));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("A | 0x8");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_tests::write_(rusty::clone(TestFlags::A) | TestFlags::from_bits_retain(1 << 3));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_tests::write_<TestZero>(rusty::clone(rusty::clone(TestZero::ZERO)));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("ABC");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_tests::write_(TestFlagsInvert::all());
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("0x1");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_tests::write_(TestOverlapping::from_bits_retain(static_cast<uint8_t>(1)));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("A");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_tests::write_<TestOverlappingFull>(rusty::clone(rusty::clone(TestOverlappingFull::C)));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("A | D");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_tests::write_(rusty::clone(TestOverlappingFull::C) | rusty::clone(TestOverlappingFull::D));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
            }

            template<typename F>
            rusty::String write_(F value) {
                auto s = std::conditional_t<true, rusty::String, F>::new_();
                ::parser::to_writer(value, s).unwrap();
                return s;
            }

        }

        namespace from_str_truncate_tests {

            void valid();

            using namespace ::tests::parser;


            // Rust-only libtest metadata const skipped: valid (marker: tests::parser::from_str_truncate::valid, should_panic: no)

            void valid() {
                {
                    auto&& _m0_tmp = static_cast<int32_t>(0);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str_truncate<::tests::TestFlags>(std::string_view("")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = static_cast<int32_t>(1);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str_truncate<::tests::TestFlags>(std::string_view("A")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = static_cast<int32_t>(1);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str_truncate<::tests::TestFlags>(std::string_view(" A ")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = (1 | (1 << 1)) | (1 << 2);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str_truncate<::tests::TestFlags>(std::string_view("A | B | C")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = (1 | (1 << 1)) | (1 << 2);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str_truncate<::tests::TestFlags>(std::string_view("A\n|\tB\r\n|   C ")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = (1 | (1 << 1)) | (1 << 2);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str_truncate<::tests::TestFlags>(std::string_view("A|B|C")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = static_cast<int32_t>(0);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str_truncate<::tests::TestFlags>(std::string_view("0x8")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = static_cast<int32_t>(1);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str_truncate<::tests::TestFlags>(std::string_view("A | 0x8")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = 1 | (1 << 1);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str_truncate<::tests::TestFlags>(std::string_view("0x1 | 0x8 | B")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = 1 | (1 << 1);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str_truncate<::tests::TestUnicode>(std::string_view("一 | 二")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
            }

        }

        namespace to_writer_truncate_tests {

            void cases();
            template<typename F>
            rusty::String write_(F value);

            using namespace ::tests::parser;


            // Rust-only libtest metadata const skipped: cases (marker: tests::parser::to_writer_truncate::cases, should_panic: no)

            void cases() {
                {
                    auto&& _m0_tmp = std::string_view("");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_truncate_tests::write_(TestFlags::empty());
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("A");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_truncate_tests::write_<TestFlags>(rusty::clone(rusty::clone(TestFlags::A)));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("A | B | C");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_truncate_tests::write_(TestFlags::all());
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_truncate_tests::write_(TestFlags::from_bits_retain(1 << 3));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("A");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_truncate_tests::write_(rusty::clone(TestFlags::A) | TestFlags::from_bits_retain(1 << 3));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_truncate_tests::write_<TestZero>(rusty::clone(rusty::clone(TestZero::ZERO)));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("ABC");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_truncate_tests::write_(TestFlagsInvert::all());
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("0x1");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_truncate_tests::write_(TestOverlapping::from_bits_retain(static_cast<uint8_t>(1)));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("A");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_truncate_tests::write_<TestOverlappingFull>(rusty::clone(rusty::clone(TestOverlappingFull::C)));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("A | D");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_truncate_tests::write_(rusty::clone(TestOverlappingFull::C) | rusty::clone(TestOverlappingFull::D));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
            }

            template<typename F>
            rusty::String write_(F value) {
                auto s = std::conditional_t<true, rusty::String, F>::new_();
                ::parser::to_writer_truncate(value, s).unwrap();
                return s;
            }

        }

        namespace from_str_strict_tests {

            void valid();
            void invalid();

            using namespace ::tests::parser;


            // Rust-only libtest metadata const skipped: valid (marker: tests::parser::from_str_strict::valid, should_panic: no)


            // Rust-only libtest metadata const skipped: invalid (marker: tests::parser::from_str_strict::invalid, should_panic: no)

            void valid() {
                {
                    auto&& _m0_tmp = static_cast<int32_t>(0);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str_strict<::tests::TestFlags>(std::string_view("")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = static_cast<int32_t>(1);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str_strict<::tests::TestFlags>(std::string_view("A")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = static_cast<int32_t>(1);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str_strict<::tests::TestFlags>(std::string_view(" A ")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = (1 | (1 << 1)) | (1 << 2);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str_strict<::tests::TestFlags>(std::string_view("A | B | C")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = (1 | (1 << 1)) | (1 << 2);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str_strict<::tests::TestFlags>(std::string_view("A\n|\tB\r\n|   C ")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = (1 | (1 << 1)) | (1 << 2);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str_strict<::tests::TestFlags>(std::string_view("A|B|C")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = 1 | (1 << 1);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::parser::from_str_strict<::tests::TestUnicode>(std::string_view("一 | 二")).unwrap().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                        }
                        _m_matched = true;
                    }
                }
            }

            void invalid() {
                if (!rusty::starts_with(rusty::to_string(::parser::from_str_strict<::tests::TestFlags>(std::string_view("a")).unwrap_err()), "unrecognized named flag")) {
                    rusty::panicking::panic("assertion failed: from_str_strict::<TestFlags>(\"a\").unwrap_err().to_string().starts_with(\"unrecognized named flag\")");
                }
                if (!rusty::starts_with(rusty::to_string(::parser::from_str_strict<::tests::TestFlags>(std::string_view("A & B")).unwrap_err()), "unrecognized named flag")) {
                    rusty::panicking::panic("assertion failed: from_str_strict::<TestFlags>(\"A & B\").unwrap_err().to_string().starts_with(\"unrecognized named flag\")");
                }
                if (!rusty::starts_with(rusty::to_string(::parser::from_str_strict<::tests::TestFlags>(std::string_view("0x1")).unwrap_err()), "invalid hex flag")) {
                    rusty::panicking::panic("assertion failed: from_str_strict::<TestFlags>(\"0x1\").unwrap_err().to_string().starts_with(\"invalid hex flag\")");
                }
                if (!rusty::starts_with(rusty::to_string(::parser::from_str_strict<::tests::TestFlags>(std::string_view("0xg")).unwrap_err()), "invalid hex flag")) {
                    rusty::panicking::panic("assertion failed: from_str_strict::<TestFlags>(\"0xg\").unwrap_err().to_string().starts_with(\"invalid hex flag\")");
                }
                if (!rusty::starts_with(rusty::to_string(::parser::from_str_strict<::tests::TestFlags>(std::string_view("0xffffffffffff")).unwrap_err()), "invalid hex flag")) {
                    rusty::panicking::panic("assertion failed: from_str_strict::<TestFlags>(\"0xffffffffffff\").unwrap_err().to_string().starts_with(\"invalid hex flag\")");
                }
            }

        }

        namespace to_writer_strict_tests {

            void cases();
            template<typename F>
            rusty::String write_(F value);

            using namespace ::tests::parser;


            // Rust-only libtest metadata const skipped: cases (marker: tests::parser::to_writer_strict::cases, should_panic: no)

            void cases() {
                {
                    auto&& _m0_tmp = std::string_view("");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_strict_tests::write_(TestFlags::empty());
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("A");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_strict_tests::write_<TestFlags>(rusty::clone(rusty::clone(TestFlags::A)));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("A | B | C");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_strict_tests::write_(TestFlags::all());
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_strict_tests::write_(TestFlags::from_bits_retain(1 << 3));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("A");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_strict_tests::write_(rusty::clone(TestFlags::A) | TestFlags::from_bits_retain(1 << 3));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_strict_tests::write_<TestZero>(rusty::clone(rusty::clone(TestZero::ZERO)));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("ABC");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_strict_tests::write_(TestFlagsInvert::all());
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_strict_tests::write_(TestOverlapping::from_bits_retain(static_cast<uint8_t>(1)));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("A");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_strict_tests::write_<TestOverlappingFull>(rusty::clone(rusty::clone(TestOverlappingFull::C)));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = std::string_view("A | D");
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = ::tests::parser::to_writer_strict_tests::write_(rusty::clone(TestOverlappingFull::C) | rusty::clone(TestOverlappingFull::D));
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                        }
                        _m_matched = true;
                    }
                }
            }

            template<typename F>
            rusty::String write_(F value) {
                auto s = std::conditional_t<true, rusty::String, F>::new_();
                ::parser::to_writer_strict(value, s).unwrap();
                return s;
            }

        }

        void roundtrip() {
            auto s = rusty::String::new_();
            for (auto&& a : rusty::for_in(rusty::range_inclusive(static_cast<uint8_t>(0), 255))) {
                for (auto&& b : rusty::for_in(rusty::range_inclusive(static_cast<uint8_t>(0), 255))) {
                    const auto f = TestFlags::from_bits_retain(a | b);
                    s.clear();
                    to_writer(f, s).unwrap();
                    {
                        auto _m0 = &f;
                        auto&& _m1_tmp = from_str<::tests::TestFlags>(rusty::to_string_view(&s)).unwrap();
                        auto _m1 = &_m1_tmp;
                        auto _m_tuple = std::make_tuple(_m0, _m1);
                        bool _m_matched = false;
                        if (!_m_matched) {
                            auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                            auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                            if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                                const auto kind = rusty::panicking::AssertKind::Eq;
                                rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                            }
                            _m_matched = true;
                        }
                    }
                }
            }
        }

        void roundtrip_truncate() {
            auto s = rusty::String::new_();
            for (auto&& a : rusty::for_in(rusty::range_inclusive(static_cast<uint8_t>(0), 255))) {
                for (auto&& b : rusty::for_in(rusty::range_inclusive(static_cast<uint8_t>(0), 255))) {
                    const auto f = TestFlags::from_bits_retain(a | b);
                    s.clear();
                    to_writer_truncate(f, s).unwrap();
                    {
                        auto&& _m0_tmp = TestFlags::from_bits_truncate(f.bits());
                        auto _m0 = &_m0_tmp;
                        auto&& _m1_tmp = from_str_truncate<::tests::TestFlags>(rusty::to_string_view(s)).unwrap();
                        auto _m1 = &_m1_tmp;
                        auto _m_tuple = std::make_tuple(_m0, _m1);
                        bool _m_matched = false;
                        if (!_m_matched) {
                            auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                            auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                            if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                                const auto kind = rusty::panicking::AssertKind::Eq;
                                rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                            }
                            _m_matched = true;
                        }
                    }
                }
            }
        }

        void roundtrip_strict() {
            auto s = rusty::String::new_();
            for (auto&& a : rusty::for_in(rusty::range_inclusive(static_cast<uint8_t>(0), 255))) {
                for (auto&& b : rusty::for_in(rusty::range_inclusive(static_cast<uint8_t>(0), 255))) {
                    const auto f = TestFlags::from_bits_retain(a | b);
                    s.clear();
                    to_writer_strict(f, s).unwrap();
                    auto strict = TestFlags::empty();
                    for (auto&& _for_item : rusty::for_in(f.iter_names())) {
                        auto&& flag = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_for_item)));
                        [&]() { static_cast<void>(strict |= flag); return std::make_tuple(); }();
                    }
                    const auto f_shadow1 = std::move(strict);
                    if (auto&& _iflet_scrutinee = from_str_strict<::tests::TestFlags>(rusty::to_string_view(s)); _iflet_scrutinee.is_ok()) {
                        decltype(auto) s = _iflet_scrutinee.unwrap();
                        {
                            auto _m0 = &f_shadow1;
                            auto _m1 = &s;
                            auto _m_tuple = std::make_tuple(_m0, _m1);
                            bool _m_matched = false;
                            if (!_m_matched) {
                                auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                                auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                                    const auto kind = rusty::panicking::AssertKind::Eq;
                                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                                }
                                _m_matched = true;
                            }
                        }
                    }
                }
            }
        }

    }

    namespace remove {

        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent_remove, const auto& inherent_set);

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::remove::cases, should_panic: no)

        void cases() {
            case_<TestFlags>(TestFlags::empty(), [&]() -> std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>> { static const std::array<std::tuple<::tests::TestFlags, TestFlags::Bits>, 3> _slice_ref_tmp = {std::tuple<::tests::TestFlags, TestFlags::Bits>{rusty::clone(rusty::clone(TestFlags::A)), 0}, std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::empty(), 0}, std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::from_bits_retain(1 << 3), 0}}; return std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.remove(std::forward<decltype(_args)>(_args)...); }, [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.set(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(rusty::clone(rusty::clone(TestFlags::A)), [&]() -> std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>> { static const std::array<std::tuple<::tests::TestFlags, TestFlags::Bits>, 3> _slice_ref_tmp = {std::tuple<::tests::TestFlags, TestFlags::Bits>{rusty::clone(rusty::clone(TestFlags::A)), 0}, std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::empty(), 1}, std::tuple<::tests::TestFlags, TestFlags::Bits>{rusty::clone(rusty::clone(TestFlags::B)), 1}}; return std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.remove(std::forward<decltype(_args)>(_args)...); }, [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.set(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(rusty::clone(rusty::clone(TestFlags::ABC)), [&]() -> std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>> { static const std::array<std::tuple<::tests::TestFlags, TestFlags::Bits>, 2> _slice_ref_tmp = {std::tuple<::tests::TestFlags, TestFlags::Bits>{rusty::clone(rusty::clone(TestFlags::A)), (1 << 1) | (1 << 2)}, std::tuple<::tests::TestFlags, TestFlags::Bits>{rusty::clone(TestFlags::A) | rusty::clone(TestFlags::C), 1 << 1}}; return std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.remove(std::forward<decltype(_args)>(_args)...); }, [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.set(std::forward<decltype(_args)>(_args)...); });
        }

        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent_remove, const auto& inherent_set) {
            for (auto&& _for_item : rusty::for_in(rusty::iter(inputs))) {
                auto&& input = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_for_item)));
                auto&& expected = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_for_item)));
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = [&]() { auto value_shadow1 = std::move(value);
inherent_remove(value_shadow1, rusty::detail::deref_if_pointer_like(input));
return value_shadow1; }().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0}.remove({1})", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = [&]() { auto value_shadow1 = std::move(value);
value_shadow1.remove(rusty::detail::deref_if_pointer_like(input));
return value_shadow1; }().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::remove({0}, {1})", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = [&]() { auto value_shadow1 = std::move(value);
inherent_set(value_shadow1, rusty::detail::deref_if_pointer_like(input), false);
return value_shadow1; }().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0}.set({1}, false)", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = [&]() { auto value_shadow1 = std::move(value);
value_shadow1.set(rusty::detail::deref_if_pointer_like(input), false);
return value_shadow1; }().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::set({0}, {1}, false)", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
            }
        }

    }

    namespace symmetric_difference {

        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent_sym_diff, const auto& inherent_toggle);

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::symmetric_difference::cases, should_panic: no)

        void cases() {
            case_<TestFlags>(TestFlags::empty(), [&]() -> std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>> { static const std::array<std::tuple<::tests::TestFlags, TestFlags::Bits>, 3> _slice_ref_tmp = {std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::empty(), 0}, std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::all(), (1 | (1 << 1)) | (1 << 2)}, std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::from_bits_retain(1 << 3), 1 << 3}}; return std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.symmetric_difference(std::forward<decltype(_args)>(_args)...); }, [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.toggle(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(rusty::clone(rusty::clone(TestFlags::A)), [&]() -> std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>> { static const std::array<std::tuple<::tests::TestFlags, TestFlags::Bits>, 3> _slice_ref_tmp = {std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::empty(), 1}, std::tuple<::tests::TestFlags, TestFlags::Bits>{rusty::clone(rusty::clone(TestFlags::A)), 0}, std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::all(), (1 << 1) | (1 << 2)}}; return std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.symmetric_difference(std::forward<decltype(_args)>(_args)...); }, [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.toggle(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>((rusty::clone(TestFlags::A) | rusty::clone(TestFlags::B)) | TestFlags::from_bits_retain(1 << 3), [&]() -> std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>> { static const std::array<std::tuple<::tests::TestFlags, TestFlags::Bits>, 2> _slice_ref_tmp = {std::tuple<::tests::TestFlags, TestFlags::Bits>{rusty::clone(rusty::clone(TestFlags::ABC)), (1 << 2) | (1 << 3)}, std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::from_bits_retain(1 << 3), 1 | (1 << 1)}}; return std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.symmetric_difference(std::forward<decltype(_args)>(_args)...); }, [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.toggle(std::forward<decltype(_args)>(_args)...); });
        }

        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent_sym_diff, const auto& inherent_toggle) {
            for (auto&& _for_item : rusty::for_in(rusty::iter(inputs))) {
                auto&& input = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_for_item)));
                auto&& expected = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_for_item)));
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = inherent_sym_diff(std::move(value), rusty::detail::deref_if_pointer_like(input)).bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0}.symmetric_difference({1})", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = (std::move(value)).symmetric_difference(rusty::detail::deref_if_pointer_like(input)).bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::symmetric_difference({0}, {1})", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = (value ^ rusty::detail::deref_if_pointer_like(input)).bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0} ^ {1}", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = [&]() { auto value_shadow1 = std::move(value);
[&]() { static_cast<void>(value_shadow1 ^= rusty::detail::deref_if_pointer_like(input)); return std::make_tuple(); }();
return value_shadow1; }().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0} ^= {1}", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = [&]() { auto value_shadow1 = std::move(value);
inherent_toggle(value_shadow1, rusty::detail::deref_if_pointer_like(input));
return value_shadow1; }().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0}.toggle({1})", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = [&]() { auto value_shadow1 = std::move(value);
value_shadow1.toggle(rusty::detail::deref_if_pointer_like(input));
return value_shadow1; }().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0}.toggle({1})", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
            }
        }

    }

    namespace union_ {

        void cases();
        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent);

        using namespace tests;

        using traits::FlagsRuntimeHelper;


        // Rust-only libtest metadata const skipped: cases (marker: tests::union::cases, should_panic: no)

        void cases() {
            case_<TestFlags>(TestFlags::empty(), [&]() -> std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>> { static const std::array<std::tuple<::tests::TestFlags, TestFlags::Bits>, 4> _slice_ref_tmp = {std::tuple<::tests::TestFlags, TestFlags::Bits>{rusty::clone(rusty::clone(TestFlags::A)), 1}, std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::all(), (1 | (1 << 1)) | (1 << 2)}, std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::empty(), 0}, std::tuple<::tests::TestFlags, TestFlags::Bits>{TestFlags::from_bits_retain(1 << 3), 1 << 3}}; return std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.union_(std::forward<decltype(_args)>(_args)...); });
            case_<TestFlags>(rusty::clone(TestFlags::A) | rusty::clone(TestFlags::C), [&]() -> std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>> { static const std::array<std::tuple<::tests::TestFlags, TestFlags::Bits>, 2> _slice_ref_tmp = {std::tuple<::tests::TestFlags, TestFlags::Bits>{rusty::clone(TestFlags::A) | rusty::clone(TestFlags::B), (1 | (1 << 1)) | (1 << 2)}, std::tuple<::tests::TestFlags, TestFlags::Bits>{rusty::clone(rusty::clone(TestFlags::A)), 1 | (1 << 2)}}; return std::span<const std::tuple<::tests::TestFlags, TestFlags::Bits>>(_slice_ref_tmp); }(), [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.union_(std::forward<decltype(_args)>(_args)...); });
        }

        template<typename T>
        void case_(T value, std::span<const std::tuple<T, typename T::Bits>> inputs, const auto& inherent) {
            for (auto&& _for_item : rusty::for_in(rusty::iter(inputs))) {
                auto&& input = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_for_item)));
                auto&& expected = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_for_item)));
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = inherent(std::move(value), rusty::detail::deref_if_pointer_like(input)).bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0}.union({1})", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = (std::move(value)).union_(rusty::detail::deref_if_pointer_like(input)).bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Flags::union({0}, {1})", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = (value | rusty::detail::deref_if_pointer_like(input)).bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0} | {1}", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
                {
                    auto&& _m0_tmp = rusty::detail::deref_if_pointer_like(expected);
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = [&]() { auto value_shadow1 = std::move(value);
[&]() { static_cast<void>(value_shadow1 |= rusty::detail::deref_if_pointer_like(input)); return std::make_tuple(); }();
return value_shadow1; }().bits();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("{0} |= {1}", rusty::to_debug_string(value), rusty::to_debug_string(input))));
                        }
                        _m_matched = true;
                    }
                }
            }
        }

    }

}

// Rust-only libtest main omitted

namespace parser {

    ///
    ///Write a flags value as text.
    ///
    ///Any bits that aren't part of a contained flag will be formatted as a hex number.
    ///
    template<typename B>
    rusty::Result<std::tuple<>, rusty::fmt::Error> to_writer(const B& flags, auto&& writer) {
        auto first = true;
        auto iter_shadow1 = flags.iter_names();
        for (auto&& _for_item : rusty::for_in(rusty::iter(iter_shadow1))) {
            auto&& name = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_for_item)));
            if (!first) {
                RUSTY_TRY_INTO(writer.write_str(" | "), rusty::Result<std::tuple<>, rusty::fmt::Error>);
            }
            first = false;
            RUSTY_TRY_INTO(writer.write_str(name), rusty::Result<std::tuple<>, rusty::fmt::Error>);
        }
        const auto remaining = iter_shadow1.remaining().bits();
        if (remaining != rusty::clone(0)) {
            if (!first) {
                RUSTY_TRY_INTO(writer.write_str(" | "), rusty::Result<std::tuple<>, rusty::fmt::Error>);
            }
            RUSTY_TRY_INTO(writer.write_str("0x"), rusty::Result<std::tuple<>, rusty::fmt::Error>);
            RUSTY_TRY_INTO(rusty::write_hex(remaining, std::move(writer)), rusty::Result<std::tuple<>, rusty::fmt::Error>);
        }
        return rusty::Result<std::tuple<>, rusty::fmt::Error>::Ok(std::make_tuple());
    }

    ///
    ///Parse a flags value from text.
    ///
    ///This function will fail on any names that don't correspond to defined flags.
    ///Unknown bits will be retained.
    ///
    template<typename B>
    rusty::Result<B, ParseError> from_str(std::string_view input) {
        auto parsed_flags = B::empty();
        if (rusty::is_empty(rusty::str_runtime::trim(input))) {
            return rusty::Result<B, ParseError>::Ok(std::move(parsed_flags));
        }
        for (auto&& flag : rusty::for_in(rusty::str_runtime::split(input, U'|'))) {
            auto flag_shadow1 = rusty::str_runtime::trim(flag);
            if (rusty::is_empty(flag_shadow1)) {
                return rusty::Result<B, ParseError>::Err(std::conditional_t<true, ParseError, B>::empty_flag());
            }
            std::optional<std::remove_cvref_t<decltype(((B::from_name(std::move(flag_shadow1)).ok_or_else([&]() { return std::conditional_t<true, ParseError, B>::invalid_named_flag(std::move(flag_shadow1)); })).unwrap()))>> _iflet_value0;
            {
                auto&& _iflet_scrutinee = rusty::str_runtime::strip_prefix(flag_shadow1, "0x");
                if (_iflet_scrutinee.is_some()) {
                    auto flag = _iflet_scrutinee.unwrap();
                    auto bits = RUSTY_TRY_INTO(rusty::parse_hex<typename B::Bits>(std::move(flag)).map_err([&](auto _closure_wild0) { return std::conditional_t<true, ParseError, B>::invalid_hex_flag(std::move(flag)); }), rusty::Result<B, ParseError>);
                    _iflet_value0.emplace(B::from_bits_retain(std::move(bits)));
                } else { _iflet_value0.emplace(RUSTY_TRY_INTO(B::from_name(std::move(flag_shadow1)).ok_or_else([&]() { return std::conditional_t<true, ParseError, B>::invalid_named_flag(std::move(flag_shadow1)); }), rusty::Result<B, ParseError>)); }
            }
            const auto parsed_flag = std::move(_iflet_value0).value();
            parsed_flags.insert(std::move(parsed_flag));
        }
        return rusty::Result<B, ParseError>::Ok(std::move(parsed_flags));
    }

    ///
    ///Write a flags value as text, ignoring any unknown bits.
    ///
    template<typename B>
    rusty::Result<std::tuple<>, rusty::fmt::Error> to_writer_truncate(const B& flags, auto&& writer) {
        return to_writer(B::from_bits_truncate(flags.bits()), std::move(writer));
    }

    ///
    ///Parse a flags value from text.
    ///
    ///This function will fail on any names that don't correspond to defined flags.
    ///Unknown bits will be ignored.
    ///
    template<typename B>
    rusty::Result<B, ParseError> from_str_truncate(std::string_view input) {
        return rusty::Result<B, ParseError>::Ok(B::from_bits_truncate(RUSTY_TRY_INTO(from_str<B>(rusty::to_string_view(input)), rusty::Result<B, ParseError>).bits()));
    }

    ///
    ///Write only the contained, defined, named flags in a flags value as text.
    ///
    template<typename B>
    rusty::Result<std::tuple<>, rusty::fmt::Error> to_writer_strict(const B& flags, auto&& writer) {
        auto first = true;
        auto iter_shadow1 = flags.iter_names();
        for (auto&& _for_item : rusty::for_in(rusty::iter(iter_shadow1))) {
            auto&& name = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_for_item)));
            if (!first) {
                RUSTY_TRY_INTO(writer.write_str(" | "), rusty::Result<std::tuple<>, rusty::fmt::Error>);
            }
            first = false;
            RUSTY_TRY_INTO(writer.write_str(name), rusty::Result<std::tuple<>, rusty::fmt::Error>);
        }
        return rusty::Result<std::tuple<>, rusty::fmt::Error>::Ok(std::make_tuple());
    }

    ///
    ///Parse a flags value from text.
    ///
    ///This function will fail on any names that don't correspond to defined flags.
    ///This function will fail to parse hex values.
    ///
    template<typename B>
    rusty::Result<B, ParseError> from_str_strict(std::string_view input) {
        auto parsed_flags = B::empty();
        if (rusty::is_empty(rusty::str_runtime::trim(input))) {
            return rusty::Result<B, ParseError>::Ok(std::move(parsed_flags));
        }
        for (auto&& flag : rusty::for_in(rusty::str_runtime::split(input, U'|'))) {
            const auto flag_shadow1 = rusty::str_runtime::trim(flag);
            if (rusty::is_empty(flag_shadow1)) {
                return rusty::Result<B, ParseError>::Err(std::conditional_t<true, ParseError, B>::empty_flag());
            }
            if (rusty::starts_with(flag_shadow1, "0x")) {
                return rusty::Result<B, ParseError>::Err(std::conditional_t<true, ParseError, B>::invalid_hex_flag("unsupported hex flag value"));
            }
            const auto parsed_flag = RUSTY_TRY_INTO(B::from_name(std::move(flag_shadow1)).ok_or_else([&]() { return std::conditional_t<true, ParseError, B>::invalid_named_flag(std::move(flag_shadow1)); }), rusty::Result<B, ParseError>);
            parsed_flags.insert(std::move(parsed_flag));
        }
        return rusty::Result<B, ParseError>::Ok(std::move(parsed_flags));
    }

}


namespace parser {
    rusty::fmt::Result ParseErrorKind::fmt(rusty::fmt::Formatter& f) const {
        return [&]() -> rusty::fmt::Result { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return f.write_str("EmptyFlag"); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m)) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).got); return rusty::fmt::Formatter::debug_struct_field1_finish(f, "InvalidNamedFlag", "got", __self_0); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m)) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).got); return rusty::fmt::Formatter::debug_struct_field1_finish(f, "InvalidHexFlag", "got", __self_0); } return [&]() -> rusty::fmt::Result { rusty::intrinsics::unreachable(); }(); }();
    }
}

namespace parser {
    rusty::fmt::Result ParseError::fmt(rusty::fmt::Formatter& f) const {
        {
            auto&& _m = this->_0;
            std::visit(overloaded {
                [&](const std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(_m)>>& _v) {
                    const auto& got = _v.got;
                    [&]() {
                        const auto _got = got;
                        RUSTY_TRY(rusty::write_fmt(f, std::string("unrecognized named flag")));
                        return decltype(rusty::write_fmt(f, std::string("unrecognized named flag"))){};
                    }();
                },
                [&](const std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(_m)>>& _v) {
                    const auto& got = _v.got;
                    [&]() {
                        const auto _got = got;
                        RUSTY_TRY(rusty::write_fmt(f, std::string("invalid hex flag")));
                        return decltype(rusty::write_fmt(f, std::string("invalid hex flag"))){};
                    }();
                },
                [&](const std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(_m)>>&) {
                    [&]() {
                        RUSTY_TRY(rusty::write_fmt(f, std::string("encountered empty flag")));
                        return decltype(rusty::write_fmt(f, std::string("encountered empty flag"))){};
                    }();
                },
            }, _m);
        }
        return rusty::fmt::Result::Ok(std::make_tuple());
    }
}

namespace parser {
    ParseError ParseError::invalid_hex_flag(const auto& flag) {
        const auto _flag = std::move(flag);
        auto got = std::make_tuple();
        return ParseError(ParseErrorKind{ParseErrorKind_InvalidHexFlag{.got = std::move(got)}});
    }
}

namespace parser {
    ParseError ParseError::invalid_named_flag(const auto& flag) {
        const auto _flag = std::move(flag);
        auto got = std::make_tuple();
        return ParseError(ParseErrorKind{ParseErrorKind_InvalidNamedFlag{.got = std::move(got)}});
    }
}

namespace parser {
    ParseError ParseError::empty_flag() {
        return ParseError(ParseErrorKind{ParseErrorKind_EmptyFlag{}});
    }
}

namespace tests {
    rusty::fmt::Result TestFlags::fmt(rusty::fmt::Formatter& f) const {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        return rusty::fmt::Formatter::debug_tuple_field1_finish(f, "TestFlags", &this->_0);
    }
}

namespace tests {
    bool TestFlags::operator==(const TestFlags& other) const {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        return this->_0 == other._0;
    }
}

namespace tests {
    void TestFlags::assert_receiver_is_total_eq() const {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
    }
}

namespace tests {
    std::partial_ordering TestFlags::operator<=>(const TestFlags& other) const {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        return rusty::to_partial_ordering([&]() -> rusty::Option<rusty::cmp::Ordering> {
            return rusty::partial_cmp(this->_0, other._0);
        }());
    }
}

namespace tests {
    rusty::cmp::Ordering TestFlags::cmp(const TestFlags& other) const {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        return rusty::cmp::cmp(this->_0, other._0);
    }
}

namespace tests {
    TestFlags TestFlags::clone() const {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        return {rusty::clone(this->_0)};
    }
}

namespace tests {

}

namespace tests {

}

namespace tests {
    TestFlags TestFlags::empty() {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        return TestFlags::from_bits_retain(rusty::clone(rusty::clone(0)));
    }
}

namespace tests {
    TestFlags TestFlags::all() {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        auto truncated = rusty::clone(0);
        for (auto&& flag : rusty::for_in(rusty::iter(TestFlags::FLAGS))) {
            truncated = truncated | flag.value().bits();
        }
        return TestFlags::from_bits_retain(std::move(truncated));
    }
}

namespace tests {
    rusty::Option<TestFlags> TestFlags::from_bits(uint8_t bits) {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        auto truncated = TestFlags::from_bits_truncate(std::move(bits));
        if (truncated.bits() == bits) {
            return rusty::Option<TestFlags>(std::move(truncated));
        } else {
            return rusty::Option<TestFlags>(rusty::None);
        }
    }
}

namespace tests {
    TestFlags TestFlags::from_bits_truncate(uint8_t bits) {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        return TestFlags::from_bits_retain(bits & TestFlags::all().bits());
    }
}

namespace tests {
    rusty::Option<TestFlags> TestFlags::from_name(std::string_view name) {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        if (rusty::is_empty(name)) {
            return rusty::Option<TestFlags>(rusty::None);
        }
        for (auto&& flag : rusty::for_in(TestFlags::FLAGS)) {
            if (flag.name() == name) {
                return rusty::Option<TestFlags>(TestFlags::from_bits_retain(flag.value().bits()));
            }
        }
        return rusty::Option<TestFlags>(rusty::None);
    }
}

namespace tests {
    TestFlags TestFlags::operator|(const auto& other) const {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        return TestFlags{static_cast<decltype(this->_0)>(this->_0 | other._0)};
    }
}

namespace tests {
    void TestFlags::operator|=(TestFlags other) {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        this->_0 |= other._0;
    }
}

namespace tests {
    TestFlags TestFlags::operator^(TestFlags other) const {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        return TestFlags{static_cast<decltype(this->_0)>(this->_0 ^ other._0)};
    }
}

namespace tests {
    void TestFlags::operator^=(TestFlags other) {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        this->_0 ^= other._0;
    }
}

namespace tests {
    TestFlags TestFlags::operator&(TestFlags other) const {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        return TestFlags{static_cast<decltype(this->_0)>(this->_0 & other._0)};
    }
}

namespace tests {
    void TestFlags::operator&=(TestFlags other) {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        (*this) = TestFlags::from_bits_retain(this->bits()).intersection(std::move(other));
    }
}

namespace tests {
    TestFlags TestFlags::operator-(TestFlags other) const {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        return TestFlags{static_cast<decltype(this->_0)>(this->_0 & ~other._0)};
    }
}

namespace tests {
    void TestFlags::operator-=(TestFlags other) {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        this->_0 &= ~other._0;
    }
}

namespace tests {
    TestFlags TestFlags::operator!() const {
        using Bits = typename TestFlags::Bits;
        using Internal = typename TestFlags::Internal;
        using IntoIter = typename TestFlags::IntoIter;
        using Item = typename TestFlags::Item;
        using Primitive = typename TestFlags::Primitive;
        return this->complement();
    }
}

namespace tests {
    rusty::fmt::Result TestFlagsInvert::fmt(rusty::fmt::Formatter& f) const {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        return rusty::fmt::Formatter::debug_tuple_field1_finish(f, "TestFlagsInvert", &this->_0);
    }
}

namespace tests {
    bool TestFlagsInvert::operator==(const TestFlagsInvert& other) const {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        return this->_0 == other._0;
    }
}

namespace tests {
    void TestFlagsInvert::assert_receiver_is_total_eq() const {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
    }
}

namespace tests {
    std::partial_ordering TestFlagsInvert::operator<=>(const TestFlagsInvert& other) const {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        return rusty::to_partial_ordering([&]() -> rusty::Option<rusty::cmp::Ordering> {
            return rusty::partial_cmp(this->_0, other._0);
        }());
    }
}

namespace tests {
    rusty::cmp::Ordering TestFlagsInvert::cmp(const TestFlagsInvert& other) const {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        return rusty::cmp::cmp(this->_0, other._0);
    }
}

namespace tests {
    TestFlagsInvert TestFlagsInvert::clone() const {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        return {rusty::clone(this->_0)};
    }
}

namespace tests {

}

namespace tests {

}

namespace tests {
    TestFlagsInvert TestFlagsInvert::empty() {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        return TestFlagsInvert::from_bits_retain(rusty::clone(rusty::clone(0)));
    }
}

namespace tests {
    TestFlagsInvert TestFlagsInvert::all() {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        auto truncated = rusty::clone(0);
        for (auto&& flag : rusty::for_in(rusty::iter(TestFlagsInvert::FLAGS))) {
            truncated = truncated | flag.value().bits();
        }
        return TestFlagsInvert::from_bits_retain(std::move(truncated));
    }
}

namespace tests {
    rusty::Option<TestFlagsInvert> TestFlagsInvert::from_bits(uint8_t bits) {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        auto truncated = TestFlagsInvert::from_bits_truncate(std::move(bits));
        if (truncated.bits() == bits) {
            return rusty::Option<TestFlagsInvert>(std::move(truncated));
        } else {
            return rusty::Option<TestFlagsInvert>(rusty::None);
        }
    }
}

namespace tests {
    TestFlagsInvert TestFlagsInvert::from_bits_truncate(uint8_t bits) {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        return TestFlagsInvert::from_bits_retain(bits & TestFlagsInvert::all().bits());
    }
}

namespace tests {
    rusty::Option<TestFlagsInvert> TestFlagsInvert::from_name(std::string_view name) {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        if (rusty::is_empty(name)) {
            return rusty::Option<TestFlagsInvert>(rusty::None);
        }
        for (auto&& flag : rusty::for_in(TestFlagsInvert::FLAGS)) {
            if (flag.name() == name) {
                return rusty::Option<TestFlagsInvert>(TestFlagsInvert::from_bits_retain(flag.value().bits()));
            }
        }
        return rusty::Option<TestFlagsInvert>(rusty::None);
    }
}

namespace tests {
    TestFlagsInvert TestFlagsInvert::operator|(const auto& other) const {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        return TestFlagsInvert{static_cast<decltype(this->_0)>(this->_0 | other._0)};
    }
}

namespace tests {
    void TestFlagsInvert::operator|=(TestFlagsInvert other) {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        this->_0 |= other._0;
    }
}

namespace tests {
    TestFlagsInvert TestFlagsInvert::operator^(TestFlagsInvert other) const {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        return TestFlagsInvert{static_cast<decltype(this->_0)>(this->_0 ^ other._0)};
    }
}

namespace tests {
    void TestFlagsInvert::operator^=(TestFlagsInvert other) {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        this->_0 ^= other._0;
    }
}

namespace tests {
    TestFlagsInvert TestFlagsInvert::operator&(TestFlagsInvert other) const {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        return TestFlagsInvert{static_cast<decltype(this->_0)>(this->_0 & other._0)};
    }
}

namespace tests {
    void TestFlagsInvert::operator&=(TestFlagsInvert other) {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        (*this) = TestFlagsInvert::from_bits_retain(this->bits()).intersection(std::move(other));
    }
}

namespace tests {
    TestFlagsInvert TestFlagsInvert::operator-(TestFlagsInvert other) const {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        return TestFlagsInvert{static_cast<decltype(this->_0)>(this->_0 & ~other._0)};
    }
}

namespace tests {
    void TestFlagsInvert::operator-=(TestFlagsInvert other) {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        this->_0 &= ~other._0;
    }
}

namespace tests {
    TestFlagsInvert TestFlagsInvert::operator!() const {
        using Bits = typename TestFlagsInvert::Bits;
        using Internal = typename TestFlagsInvert::Internal;
        using IntoIter = typename TestFlagsInvert::IntoIter;
        using Item = typename TestFlagsInvert::Item;
        using Primitive = typename TestFlagsInvert::Primitive;
        return this->complement();
    }
}

namespace tests {
    rusty::fmt::Result TestZero::fmt(rusty::fmt::Formatter& f) const {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        return rusty::fmt::Formatter::debug_tuple_field1_finish(f, "TestZero", &this->_0);
    }
}

namespace tests {
    bool TestZero::operator==(const TestZero& other) const {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        return this->_0 == other._0;
    }
}

namespace tests {
    void TestZero::assert_receiver_is_total_eq() const {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
    }
}

namespace tests {
    std::partial_ordering TestZero::operator<=>(const TestZero& other) const {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        return rusty::to_partial_ordering([&]() -> rusty::Option<rusty::cmp::Ordering> {
            return rusty::partial_cmp(this->_0, other._0);
        }());
    }
}

namespace tests {
    rusty::cmp::Ordering TestZero::cmp(const TestZero& other) const {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        return rusty::cmp::cmp(this->_0, other._0);
    }
}

namespace tests {
    TestZero TestZero::clone() const {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        return {rusty::clone(this->_0)};
    }
}

namespace tests {

}

namespace tests {

}

namespace tests {
    TestZero TestZero::empty() {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        return TestZero::from_bits_retain(rusty::clone(rusty::clone(0)));
    }
}

namespace tests {
    TestZero TestZero::all() {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        auto truncated = rusty::clone(0);
        for (auto&& flag : rusty::for_in(rusty::iter(TestZero::FLAGS))) {
            truncated = truncated | flag.value().bits();
        }
        return TestZero::from_bits_retain(std::move(truncated));
    }
}

namespace tests {
    rusty::Option<TestZero> TestZero::from_bits(uint8_t bits) {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        auto truncated = TestZero::from_bits_truncate(std::move(bits));
        if (truncated.bits() == bits) {
            return rusty::Option<TestZero>(std::move(truncated));
        } else {
            return rusty::Option<TestZero>(rusty::None);
        }
    }
}

namespace tests {
    TestZero TestZero::from_bits_truncate(uint8_t bits) {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        return TestZero::from_bits_retain(bits & TestZero::all().bits());
    }
}

namespace tests {
    rusty::Option<TestZero> TestZero::from_name(std::string_view name) {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        if (rusty::is_empty(name)) {
            return rusty::Option<TestZero>(rusty::None);
        }
        for (auto&& flag : rusty::for_in(TestZero::FLAGS)) {
            if (flag.name() == name) {
                return rusty::Option<TestZero>(TestZero::from_bits_retain(flag.value().bits()));
            }
        }
        return rusty::Option<TestZero>(rusty::None);
    }
}

namespace tests {
    TestZero TestZero::operator|(const auto& other) const {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        return TestZero{static_cast<decltype(this->_0)>(this->_0 | other._0)};
    }
}

namespace tests {
    void TestZero::operator|=(TestZero other) {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        this->_0 |= other._0;
    }
}

namespace tests {
    TestZero TestZero::operator^(TestZero other) const {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        return TestZero{static_cast<decltype(this->_0)>(this->_0 ^ other._0)};
    }
}

namespace tests {
    void TestZero::operator^=(TestZero other) {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        this->_0 ^= other._0;
    }
}

namespace tests {
    TestZero TestZero::operator&(TestZero other) const {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        return TestZero{static_cast<decltype(this->_0)>(this->_0 & other._0)};
    }
}

namespace tests {
    void TestZero::operator&=(TestZero other) {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        (*this) = TestZero::from_bits_retain(this->bits()).intersection(std::move(other));
    }
}

namespace tests {
    TestZero TestZero::operator-(TestZero other) const {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        return TestZero{static_cast<decltype(this->_0)>(this->_0 & ~other._0)};
    }
}

namespace tests {
    void TestZero::operator-=(TestZero other) {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        this->_0 &= ~other._0;
    }
}

namespace tests {
    TestZero TestZero::operator!() const {
        using Bits = typename TestZero::Bits;
        using Internal = typename TestZero::Internal;
        using IntoIter = typename TestZero::IntoIter;
        using Item = typename TestZero::Item;
        using Primitive = typename TestZero::Primitive;
        return this->complement();
    }
}

namespace tests {
    rusty::fmt::Result TestZeroOne::fmt(rusty::fmt::Formatter& f) const {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        return rusty::fmt::Formatter::debug_tuple_field1_finish(f, "TestZeroOne", &this->_0);
    }
}

namespace tests {
    bool TestZeroOne::operator==(const TestZeroOne& other) const {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        return this->_0 == other._0;
    }
}

namespace tests {
    void TestZeroOne::assert_receiver_is_total_eq() const {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
    }
}

namespace tests {
    std::partial_ordering TestZeroOne::operator<=>(const TestZeroOne& other) const {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        return rusty::to_partial_ordering([&]() -> rusty::Option<rusty::cmp::Ordering> {
            return rusty::partial_cmp(this->_0, other._0);
        }());
    }
}

namespace tests {
    rusty::cmp::Ordering TestZeroOne::cmp(const TestZeroOne& other) const {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        return rusty::cmp::cmp(this->_0, other._0);
    }
}

namespace tests {
    TestZeroOne TestZeroOne::clone() const {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        return {rusty::clone(this->_0)};
    }
}

namespace tests {

}

namespace tests {

}

namespace tests {
    TestZeroOne TestZeroOne::empty() {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        return TestZeroOne::from_bits_retain(rusty::clone(rusty::clone(0)));
    }
}

namespace tests {
    TestZeroOne TestZeroOne::all() {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        auto truncated = rusty::clone(0);
        for (auto&& flag : rusty::for_in(rusty::iter(TestZeroOne::FLAGS))) {
            truncated = truncated | flag.value().bits();
        }
        return TestZeroOne::from_bits_retain(std::move(truncated));
    }
}

namespace tests {
    rusty::Option<TestZeroOne> TestZeroOne::from_bits(uint8_t bits) {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        auto truncated = TestZeroOne::from_bits_truncate(std::move(bits));
        if (truncated.bits() == bits) {
            return rusty::Option<TestZeroOne>(std::move(truncated));
        } else {
            return rusty::Option<TestZeroOne>(rusty::None);
        }
    }
}

namespace tests {
    TestZeroOne TestZeroOne::from_bits_truncate(uint8_t bits) {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        return TestZeroOne::from_bits_retain(bits & TestZeroOne::all().bits());
    }
}

namespace tests {
    rusty::Option<TestZeroOne> TestZeroOne::from_name(std::string_view name) {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        if (rusty::is_empty(name)) {
            return rusty::Option<TestZeroOne>(rusty::None);
        }
        for (auto&& flag : rusty::for_in(TestZeroOne::FLAGS)) {
            if (flag.name() == name) {
                return rusty::Option<TestZeroOne>(TestZeroOne::from_bits_retain(flag.value().bits()));
            }
        }
        return rusty::Option<TestZeroOne>(rusty::None);
    }
}

namespace tests {
    TestZeroOne TestZeroOne::operator|(const auto& other) const {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        return TestZeroOne{static_cast<decltype(this->_0)>(this->_0 | other._0)};
    }
}

namespace tests {
    void TestZeroOne::operator|=(TestZeroOne other) {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        this->_0 |= other._0;
    }
}

namespace tests {
    TestZeroOne TestZeroOne::operator^(TestZeroOne other) const {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        return TestZeroOne{static_cast<decltype(this->_0)>(this->_0 ^ other._0)};
    }
}

namespace tests {
    void TestZeroOne::operator^=(TestZeroOne other) {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        this->_0 ^= other._0;
    }
}

namespace tests {
    TestZeroOne TestZeroOne::operator&(TestZeroOne other) const {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        return TestZeroOne{static_cast<decltype(this->_0)>(this->_0 & other._0)};
    }
}

namespace tests {
    void TestZeroOne::operator&=(TestZeroOne other) {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        (*this) = TestZeroOne::from_bits_retain(this->bits()).intersection(std::move(other));
    }
}

namespace tests {
    TestZeroOne TestZeroOne::operator-(TestZeroOne other) const {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        return TestZeroOne{static_cast<decltype(this->_0)>(this->_0 & ~other._0)};
    }
}

namespace tests {
    void TestZeroOne::operator-=(TestZeroOne other) {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        this->_0 &= ~other._0;
    }
}

namespace tests {
    TestZeroOne TestZeroOne::operator!() const {
        using Bits = typename TestZeroOne::Bits;
        using Internal = typename TestZeroOne::Internal;
        using IntoIter = typename TestZeroOne::IntoIter;
        using Item = typename TestZeroOne::Item;
        using Primitive = typename TestZeroOne::Primitive;
        return this->complement();
    }
}

namespace tests {
    rusty::fmt::Result TestUnicode::fmt(rusty::fmt::Formatter& f) const {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        return rusty::fmt::Formatter::debug_tuple_field1_finish(f, "TestUnicode", &this->_0);
    }
}

namespace tests {
    bool TestUnicode::operator==(const TestUnicode& other) const {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        return this->_0 == other._0;
    }
}

namespace tests {
    void TestUnicode::assert_receiver_is_total_eq() const {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
    }
}

namespace tests {
    std::partial_ordering TestUnicode::operator<=>(const TestUnicode& other) const {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        return rusty::to_partial_ordering([&]() -> rusty::Option<rusty::cmp::Ordering> {
            return rusty::partial_cmp(this->_0, other._0);
        }());
    }
}

namespace tests {
    rusty::cmp::Ordering TestUnicode::cmp(const TestUnicode& other) const {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        return rusty::cmp::cmp(this->_0, other._0);
    }
}

namespace tests {
    TestUnicode TestUnicode::clone() const {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        return {rusty::clone(this->_0)};
    }
}

namespace tests {

}

namespace tests {

}

namespace tests {
    TestUnicode TestUnicode::empty() {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        return TestUnicode::from_bits_retain(rusty::clone(rusty::clone(0)));
    }
}

namespace tests {
    TestUnicode TestUnicode::all() {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        auto truncated = rusty::clone(0);
        for (auto&& flag : rusty::for_in(rusty::iter(TestUnicode::FLAGS))) {
            truncated = truncated | flag.value().bits();
        }
        return TestUnicode::from_bits_retain(std::move(truncated));
    }
}

namespace tests {
    rusty::Option<TestUnicode> TestUnicode::from_bits(uint8_t bits) {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        auto truncated = TestUnicode::from_bits_truncate(std::move(bits));
        if (truncated.bits() == bits) {
            return rusty::Option<TestUnicode>(std::move(truncated));
        } else {
            return rusty::Option<TestUnicode>(rusty::None);
        }
    }
}

namespace tests {
    TestUnicode TestUnicode::from_bits_truncate(uint8_t bits) {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        return TestUnicode::from_bits_retain(bits & TestUnicode::all().bits());
    }
}

namespace tests {
    rusty::Option<TestUnicode> TestUnicode::from_name(std::string_view name) {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        if (rusty::is_empty(name)) {
            return rusty::Option<TestUnicode>(rusty::None);
        }
        for (auto&& flag : rusty::for_in(TestUnicode::FLAGS)) {
            if (flag.name() == name) {
                return rusty::Option<TestUnicode>(TestUnicode::from_bits_retain(flag.value().bits()));
            }
        }
        return rusty::Option<TestUnicode>(rusty::None);
    }
}

namespace tests {
    TestUnicode TestUnicode::operator|(const auto& other) const {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        return TestUnicode{static_cast<decltype(this->_0)>(this->_0 | other._0)};
    }
}

namespace tests {
    void TestUnicode::operator|=(TestUnicode other) {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        this->_0 |= other._0;
    }
}

namespace tests {
    TestUnicode TestUnicode::operator^(TestUnicode other) const {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        return TestUnicode{static_cast<decltype(this->_0)>(this->_0 ^ other._0)};
    }
}

namespace tests {
    void TestUnicode::operator^=(TestUnicode other) {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        this->_0 ^= other._0;
    }
}

namespace tests {
    TestUnicode TestUnicode::operator&(TestUnicode other) const {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        return TestUnicode{static_cast<decltype(this->_0)>(this->_0 & other._0)};
    }
}

namespace tests {
    void TestUnicode::operator&=(TestUnicode other) {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        (*this) = TestUnicode::from_bits_retain(this->bits()).intersection(std::move(other));
    }
}

namespace tests {
    TestUnicode TestUnicode::operator-(TestUnicode other) const {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        return TestUnicode{static_cast<decltype(this->_0)>(this->_0 & ~other._0)};
    }
}

namespace tests {
    void TestUnicode::operator-=(TestUnicode other) {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        this->_0 &= ~other._0;
    }
}

namespace tests {
    TestUnicode TestUnicode::operator!() const {
        using Bits = typename TestUnicode::Bits;
        using Internal = typename TestUnicode::Internal;
        using IntoIter = typename TestUnicode::IntoIter;
        using Item = typename TestUnicode::Item;
        using Primitive = typename TestUnicode::Primitive;
        return this->complement();
    }
}

namespace tests {
    rusty::fmt::Result TestEmpty::fmt(rusty::fmt::Formatter& f) const {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        return rusty::fmt::Formatter::debug_tuple_field1_finish(f, "TestEmpty", &this->_0);
    }
}

namespace tests {
    bool TestEmpty::operator==(const TestEmpty& other) const {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        return this->_0 == other._0;
    }
}

namespace tests {
    void TestEmpty::assert_receiver_is_total_eq() const {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
    }
}

namespace tests {
    std::partial_ordering TestEmpty::operator<=>(const TestEmpty& other) const {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        return rusty::to_partial_ordering([&]() -> rusty::Option<rusty::cmp::Ordering> {
            return rusty::partial_cmp(this->_0, other._0);
        }());
    }
}

namespace tests {
    rusty::cmp::Ordering TestEmpty::cmp(const TestEmpty& other) const {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        return rusty::cmp::cmp(this->_0, other._0);
    }
}

namespace tests {
    TestEmpty TestEmpty::clone() const {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        return {rusty::clone(this->_0)};
    }
}

namespace tests {

}

namespace tests {

}

namespace tests {
    TestEmpty TestEmpty::empty() {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        return TestEmpty::from_bits_retain(rusty::clone(rusty::clone(0)));
    }
}

namespace tests {
    TestEmpty TestEmpty::all() {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        auto truncated = rusty::clone(0);
        for (auto&& flag : rusty::for_in(rusty::iter(TestEmpty::FLAGS))) {
            truncated = truncated | flag.value().bits();
        }
        return TestEmpty::from_bits_retain(std::move(truncated));
    }
}

namespace tests {
    rusty::Option<TestEmpty> TestEmpty::from_bits(uint8_t bits) {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        auto truncated = TestEmpty::from_bits_truncate(std::move(bits));
        if (truncated.bits() == bits) {
            return rusty::Option<TestEmpty>(std::move(truncated));
        } else {
            return rusty::Option<TestEmpty>(rusty::None);
        }
    }
}

namespace tests {
    TestEmpty TestEmpty::from_bits_truncate(uint8_t bits) {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        return TestEmpty::from_bits_retain(bits & TestEmpty::all().bits());
    }
}

namespace tests {
    rusty::Option<TestEmpty> TestEmpty::from_name(std::string_view name) {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        if (rusty::is_empty(name)) {
            return rusty::Option<TestEmpty>(rusty::None);
        }
        for (auto&& flag : rusty::for_in(TestEmpty::FLAGS)) {
            if (flag.name() == name) {
                return rusty::Option<TestEmpty>(TestEmpty::from_bits_retain(flag.value().bits()));
            }
        }
        return rusty::Option<TestEmpty>(rusty::None);
    }
}

namespace tests {
    TestEmpty TestEmpty::operator|(const auto& other) const {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        return TestEmpty{static_cast<decltype(this->_0)>(this->_0 | other._0)};
    }
}

namespace tests {
    void TestEmpty::operator|=(TestEmpty other) {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        this->_0 |= other._0;
    }
}

namespace tests {
    TestEmpty TestEmpty::operator^(TestEmpty other) const {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        return TestEmpty{static_cast<decltype(this->_0)>(this->_0 ^ other._0)};
    }
}

namespace tests {
    void TestEmpty::operator^=(TestEmpty other) {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        this->_0 ^= other._0;
    }
}

namespace tests {
    TestEmpty TestEmpty::operator&(TestEmpty other) const {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        return TestEmpty{static_cast<decltype(this->_0)>(this->_0 & other._0)};
    }
}

namespace tests {
    void TestEmpty::operator&=(TestEmpty other) {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        (*this) = TestEmpty::from_bits_retain(this->bits()).intersection(std::move(other));
    }
}

namespace tests {
    TestEmpty TestEmpty::operator-(TestEmpty other) const {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        return TestEmpty{static_cast<decltype(this->_0)>(this->_0 & ~other._0)};
    }
}

namespace tests {
    void TestEmpty::operator-=(TestEmpty other) {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        this->_0 &= ~other._0;
    }
}

namespace tests {
    TestEmpty TestEmpty::operator!() const {
        using Bits = typename TestEmpty::Bits;
        using Internal = typename TestEmpty::Internal;
        using IntoIter = typename TestEmpty::IntoIter;
        using Item = typename TestEmpty::Item;
        using Primitive = typename TestEmpty::Primitive;
        return this->complement();
    }
}

namespace tests {
    rusty::fmt::Result TestOverlapping::fmt(rusty::fmt::Formatter& f) const {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        return rusty::fmt::Formatter::debug_tuple_field1_finish(f, "TestOverlapping", &this->_0);
    }
}

namespace tests {
    bool TestOverlapping::operator==(const TestOverlapping& other) const {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        return this->_0 == other._0;
    }
}

namespace tests {
    void TestOverlapping::assert_receiver_is_total_eq() const {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
    }
}

namespace tests {
    std::partial_ordering TestOverlapping::operator<=>(const TestOverlapping& other) const {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        return rusty::to_partial_ordering([&]() -> rusty::Option<rusty::cmp::Ordering> {
            return rusty::partial_cmp(this->_0, other._0);
        }());
    }
}

namespace tests {
    rusty::cmp::Ordering TestOverlapping::cmp(const TestOverlapping& other) const {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        return rusty::cmp::cmp(this->_0, other._0);
    }
}

namespace tests {
    TestOverlapping TestOverlapping::clone() const {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        return {rusty::clone(this->_0)};
    }
}

namespace tests {

}

namespace tests {

}

namespace tests {
    TestOverlapping TestOverlapping::empty() {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        return TestOverlapping::from_bits_retain(rusty::clone(rusty::clone(0)));
    }
}

namespace tests {
    TestOverlapping TestOverlapping::all() {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        auto truncated = rusty::clone(0);
        for (auto&& flag : rusty::for_in(rusty::iter(TestOverlapping::FLAGS))) {
            truncated = truncated | flag.value().bits();
        }
        return TestOverlapping::from_bits_retain(std::move(truncated));
    }
}

namespace tests {
    rusty::Option<TestOverlapping> TestOverlapping::from_bits(uint8_t bits) {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        auto truncated = TestOverlapping::from_bits_truncate(std::move(bits));
        if (truncated.bits() == bits) {
            return rusty::Option<TestOverlapping>(std::move(truncated));
        } else {
            return rusty::Option<TestOverlapping>(rusty::None);
        }
    }
}

namespace tests {
    TestOverlapping TestOverlapping::from_bits_truncate(uint8_t bits) {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        return TestOverlapping::from_bits_retain(bits & TestOverlapping::all().bits());
    }
}

namespace tests {
    rusty::Option<TestOverlapping> TestOverlapping::from_name(std::string_view name) {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        if (rusty::is_empty(name)) {
            return rusty::Option<TestOverlapping>(rusty::None);
        }
        for (auto&& flag : rusty::for_in(TestOverlapping::FLAGS)) {
            if (flag.name() == name) {
                return rusty::Option<TestOverlapping>(TestOverlapping::from_bits_retain(flag.value().bits()));
            }
        }
        return rusty::Option<TestOverlapping>(rusty::None);
    }
}

namespace tests {
    TestOverlapping TestOverlapping::operator|(const auto& other) const {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        return TestOverlapping{static_cast<decltype(this->_0)>(this->_0 | other._0)};
    }
}

namespace tests {
    void TestOverlapping::operator|=(TestOverlapping other) {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        this->_0 |= other._0;
    }
}

namespace tests {
    TestOverlapping TestOverlapping::operator^(TestOverlapping other) const {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        return TestOverlapping{static_cast<decltype(this->_0)>(this->_0 ^ other._0)};
    }
}

namespace tests {
    void TestOverlapping::operator^=(TestOverlapping other) {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        this->_0 ^= other._0;
    }
}

namespace tests {
    TestOverlapping TestOverlapping::operator&(TestOverlapping other) const {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        return TestOverlapping{static_cast<decltype(this->_0)>(this->_0 & other._0)};
    }
}

namespace tests {
    void TestOverlapping::operator&=(TestOverlapping other) {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        (*this) = TestOverlapping::from_bits_retain(this->bits()).intersection(std::move(other));
    }
}

namespace tests {
    TestOverlapping TestOverlapping::operator-(TestOverlapping other) const {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        return TestOverlapping{static_cast<decltype(this->_0)>(this->_0 & ~other._0)};
    }
}

namespace tests {
    void TestOverlapping::operator-=(TestOverlapping other) {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        this->_0 &= ~other._0;
    }
}

namespace tests {
    TestOverlapping TestOverlapping::operator!() const {
        using Bits = typename TestOverlapping::Bits;
        using Internal = typename TestOverlapping::Internal;
        using IntoIter = typename TestOverlapping::IntoIter;
        using Item = typename TestOverlapping::Item;
        using Primitive = typename TestOverlapping::Primitive;
        return this->complement();
    }
}

namespace tests {
    rusty::fmt::Result TestOverlappingFull::fmt(rusty::fmt::Formatter& f) const {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        return rusty::fmt::Formatter::debug_tuple_field1_finish(f, "TestOverlappingFull", &this->_0);
    }
}

namespace tests {
    bool TestOverlappingFull::operator==(const TestOverlappingFull& other) const {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        return this->_0 == other._0;
    }
}

namespace tests {
    void TestOverlappingFull::assert_receiver_is_total_eq() const {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
    }
}

namespace tests {
    std::partial_ordering TestOverlappingFull::operator<=>(const TestOverlappingFull& other) const {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        return rusty::to_partial_ordering([&]() -> rusty::Option<rusty::cmp::Ordering> {
            return rusty::partial_cmp(this->_0, other._0);
        }());
    }
}

namespace tests {
    rusty::cmp::Ordering TestOverlappingFull::cmp(const TestOverlappingFull& other) const {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        return rusty::cmp::cmp(this->_0, other._0);
    }
}

namespace tests {
    TestOverlappingFull TestOverlappingFull::clone() const {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        return {rusty::clone(this->_0)};
    }
}

namespace tests {

}

namespace tests {

}

namespace tests {
    TestOverlappingFull TestOverlappingFull::empty() {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        return TestOverlappingFull::from_bits_retain(rusty::clone(rusty::clone(0)));
    }
}

namespace tests {
    TestOverlappingFull TestOverlappingFull::all() {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        auto truncated = rusty::clone(0);
        for (auto&& flag : rusty::for_in(rusty::iter(TestOverlappingFull::FLAGS))) {
            truncated = truncated | flag.value().bits();
        }
        return TestOverlappingFull::from_bits_retain(std::move(truncated));
    }
}

namespace tests {
    rusty::Option<TestOverlappingFull> TestOverlappingFull::from_bits(uint8_t bits) {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        auto truncated = TestOverlappingFull::from_bits_truncate(std::move(bits));
        if (truncated.bits() == bits) {
            return rusty::Option<TestOverlappingFull>(std::move(truncated));
        } else {
            return rusty::Option<TestOverlappingFull>(rusty::None);
        }
    }
}

namespace tests {
    TestOverlappingFull TestOverlappingFull::from_bits_truncate(uint8_t bits) {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        return TestOverlappingFull::from_bits_retain(bits & TestOverlappingFull::all().bits());
    }
}

namespace tests {
    rusty::Option<TestOverlappingFull> TestOverlappingFull::from_name(std::string_view name) {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        if (rusty::is_empty(name)) {
            return rusty::Option<TestOverlappingFull>(rusty::None);
        }
        for (auto&& flag : rusty::for_in(TestOverlappingFull::FLAGS)) {
            if (flag.name() == name) {
                return rusty::Option<TestOverlappingFull>(TestOverlappingFull::from_bits_retain(flag.value().bits()));
            }
        }
        return rusty::Option<TestOverlappingFull>(rusty::None);
    }
}

namespace tests {
    TestOverlappingFull TestOverlappingFull::operator|(const auto& other) const {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        return TestOverlappingFull{static_cast<decltype(this->_0)>(this->_0 | other._0)};
    }
}

namespace tests {
    void TestOverlappingFull::operator|=(TestOverlappingFull other) {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        this->_0 |= other._0;
    }
}

namespace tests {
    TestOverlappingFull TestOverlappingFull::operator^(TestOverlappingFull other) const {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        return TestOverlappingFull{static_cast<decltype(this->_0)>(this->_0 ^ other._0)};
    }
}

namespace tests {
    void TestOverlappingFull::operator^=(TestOverlappingFull other) {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        this->_0 ^= other._0;
    }
}

namespace tests {
    TestOverlappingFull TestOverlappingFull::operator&(TestOverlappingFull other) const {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        return TestOverlappingFull{static_cast<decltype(this->_0)>(this->_0 & other._0)};
    }
}

namespace tests {
    void TestOverlappingFull::operator&=(TestOverlappingFull other) {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        (*this) = TestOverlappingFull::from_bits_retain(this->bits()).intersection(std::move(other));
    }
}

namespace tests {
    TestOverlappingFull TestOverlappingFull::operator-(TestOverlappingFull other) const {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        return TestOverlappingFull{static_cast<decltype(this->_0)>(this->_0 & ~other._0)};
    }
}

namespace tests {
    void TestOverlappingFull::operator-=(TestOverlappingFull other) {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        this->_0 &= ~other._0;
    }
}

namespace tests {
    TestOverlappingFull TestOverlappingFull::operator!() const {
        using Bits = typename TestOverlappingFull::Bits;
        using Internal = typename TestOverlappingFull::Internal;
        using IntoIter = typename TestOverlappingFull::IntoIter;
        using Item = typename TestOverlappingFull::Item;
        using Primitive = typename TestOverlappingFull::Primitive;
        return this->complement();
    }
}

namespace tests {
    rusty::fmt::Result TestExternal::fmt(rusty::fmt::Formatter& f) const {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        return rusty::fmt::Formatter::debug_tuple_field1_finish(f, "TestExternal", &this->_0);
    }
}

namespace tests {
    bool TestExternal::operator==(const TestExternal& other) const {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        return this->_0 == other._0;
    }
}

namespace tests {
    void TestExternal::assert_receiver_is_total_eq() const {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
    }
}

namespace tests {
    std::partial_ordering TestExternal::operator<=>(const TestExternal& other) const {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        return rusty::to_partial_ordering([&]() -> rusty::Option<rusty::cmp::Ordering> {
            return rusty::partial_cmp(this->_0, other._0);
        }());
    }
}

namespace tests {
    rusty::cmp::Ordering TestExternal::cmp(const TestExternal& other) const {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        return rusty::cmp::cmp(this->_0, other._0);
    }
}

namespace tests {
    TestExternal TestExternal::clone() const {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        return {rusty::clone(this->_0)};
    }
}

namespace tests {

}

namespace tests {

}

namespace tests {
    TestExternal TestExternal::empty() {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        return TestExternal::from_bits_retain(rusty::clone(rusty::clone(0)));
    }
}

namespace tests {
    TestExternal TestExternal::all() {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        auto truncated = rusty::clone(0);
        for (auto&& flag : rusty::for_in(rusty::iter(TestExternal::FLAGS))) {
            truncated = truncated | flag.value().bits();
        }
        return TestExternal::from_bits_retain(std::move(truncated));
    }
}

namespace tests {
    rusty::Option<TestExternal> TestExternal::from_bits(uint8_t bits) {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        auto truncated = TestExternal::from_bits_truncate(std::move(bits));
        if (truncated.bits() == bits) {
            return rusty::Option<TestExternal>(std::move(truncated));
        } else {
            return rusty::Option<TestExternal>(rusty::None);
        }
    }
}

namespace tests {
    TestExternal TestExternal::from_bits_truncate(uint8_t bits) {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        return TestExternal::from_bits_retain(bits & TestExternal::all().bits());
    }
}

namespace tests {
    rusty::Option<TestExternal> TestExternal::from_name(std::string_view name) {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        if (rusty::is_empty(name)) {
            return rusty::Option<TestExternal>(rusty::None);
        }
        for (auto&& flag : rusty::for_in(TestExternal::FLAGS)) {
            if (flag.name() == name) {
                return rusty::Option<TestExternal>(TestExternal::from_bits_retain(flag.value().bits()));
            }
        }
        return rusty::Option<TestExternal>(rusty::None);
    }
}

namespace tests {
    TestExternal TestExternal::operator|(const auto& other) const {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        return TestExternal{static_cast<decltype(this->_0)>(this->_0 | other._0)};
    }
}

namespace tests {
    void TestExternal::operator|=(TestExternal other) {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        this->_0 |= other._0;
    }
}

namespace tests {
    TestExternal TestExternal::operator^(TestExternal other) const {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        return TestExternal{static_cast<decltype(this->_0)>(this->_0 ^ other._0)};
    }
}

namespace tests {
    void TestExternal::operator^=(TestExternal other) {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        this->_0 ^= other._0;
    }
}

namespace tests {
    TestExternal TestExternal::operator&(TestExternal other) const {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        return TestExternal{static_cast<decltype(this->_0)>(this->_0 & other._0)};
    }
}

namespace tests {
    void TestExternal::operator&=(TestExternal other) {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        (*this) = TestExternal::from_bits_retain(this->bits()).intersection(std::move(other));
    }
}

namespace tests {
    TestExternal TestExternal::operator-(TestExternal other) const {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        return TestExternal{static_cast<decltype(this->_0)>(this->_0 & ~other._0)};
    }
}

namespace tests {
    void TestExternal::operator-=(TestExternal other) {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        this->_0 &= ~other._0;
    }
}

namespace tests {
    TestExternal TestExternal::operator!() const {
        using Bits = typename TestExternal::Bits;
        using Internal = typename TestExternal::Internal;
        using IntoIter = typename TestExternal::IntoIter;
        using Item = typename TestExternal::Item;
        using Primitive = typename TestExternal::Primitive;
        return this->complement();
    }
}

namespace tests {
    rusty::fmt::Result TestExternalFull::fmt(rusty::fmt::Formatter& f) const {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        return rusty::fmt::Formatter::debug_tuple_field1_finish(f, "TestExternalFull", &this->_0);
    }
}

namespace tests {
    bool TestExternalFull::operator==(const TestExternalFull& other) const {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        return this->_0 == other._0;
    }
}

namespace tests {
    void TestExternalFull::assert_receiver_is_total_eq() const {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
    }
}

namespace tests {
    std::partial_ordering TestExternalFull::operator<=>(const TestExternalFull& other) const {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        return rusty::to_partial_ordering([&]() -> rusty::Option<rusty::cmp::Ordering> {
            return rusty::partial_cmp(this->_0, other._0);
        }());
    }
}

namespace tests {
    rusty::cmp::Ordering TestExternalFull::cmp(const TestExternalFull& other) const {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        return rusty::cmp::cmp(this->_0, other._0);
    }
}

namespace tests {
    TestExternalFull TestExternalFull::clone() const {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        return {rusty::clone(this->_0)};
    }
}

namespace tests {

}

namespace tests {

}

namespace tests {
    TestExternalFull TestExternalFull::empty() {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        return TestExternalFull::from_bits_retain(rusty::clone(rusty::clone(0)));
    }
}

namespace tests {
    TestExternalFull TestExternalFull::all() {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        auto truncated = rusty::clone(0);
        for (auto&& flag : rusty::for_in(rusty::iter(TestExternalFull::FLAGS))) {
            truncated = truncated | flag.value().bits();
        }
        return TestExternalFull::from_bits_retain(std::move(truncated));
    }
}

namespace tests {
    rusty::Option<TestExternalFull> TestExternalFull::from_bits(uint8_t bits) {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        auto truncated = TestExternalFull::from_bits_truncate(std::move(bits));
        if (truncated.bits() == bits) {
            return rusty::Option<TestExternalFull>(std::move(truncated));
        } else {
            return rusty::Option<TestExternalFull>(rusty::None);
        }
    }
}

namespace tests {
    TestExternalFull TestExternalFull::from_bits_truncate(uint8_t bits) {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        return TestExternalFull::from_bits_retain(bits & TestExternalFull::all().bits());
    }
}

namespace tests {
    rusty::Option<TestExternalFull> TestExternalFull::from_name(std::string_view name) {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        if (rusty::is_empty(name)) {
            return rusty::Option<TestExternalFull>(rusty::None);
        }
        for (auto&& flag : rusty::for_in(TestExternalFull::FLAGS)) {
            if (flag.name() == name) {
                return rusty::Option<TestExternalFull>(TestExternalFull::from_bits_retain(flag.value().bits()));
            }
        }
        return rusty::Option<TestExternalFull>(rusty::None);
    }
}

namespace tests {
    TestExternalFull TestExternalFull::operator|(const auto& other) const {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        return TestExternalFull{static_cast<decltype(this->_0)>(this->_0 | other._0)};
    }
}

namespace tests {
    void TestExternalFull::operator|=(TestExternalFull other) {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        this->_0 |= other._0;
    }
}

namespace tests {
    TestExternalFull TestExternalFull::operator^(TestExternalFull other) const {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        return TestExternalFull{static_cast<decltype(this->_0)>(this->_0 ^ other._0)};
    }
}

namespace tests {
    void TestExternalFull::operator^=(TestExternalFull other) {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        this->_0 ^= other._0;
    }
}

namespace tests {
    TestExternalFull TestExternalFull::operator&(TestExternalFull other) const {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        return TestExternalFull{static_cast<decltype(this->_0)>(this->_0 & other._0)};
    }
}

namespace tests {
    void TestExternalFull::operator&=(TestExternalFull other) {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        (*this) = TestExternalFull::from_bits_retain(this->bits()).intersection(std::move(other));
    }
}

namespace tests {
    TestExternalFull TestExternalFull::operator-(TestExternalFull other) const {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        return TestExternalFull{static_cast<decltype(this->_0)>(this->_0 & ~other._0)};
    }
}

namespace tests {
    void TestExternalFull::operator-=(TestExternalFull other) {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        this->_0 &= ~other._0;
    }
}

namespace tests {
    TestExternalFull TestExternalFull::operator!() const {
        using Bits = typename TestExternalFull::Bits;
        using Internal = typename TestExternalFull::Internal;
        using IntoIter = typename TestExternalFull::IntoIter;
        using Item = typename TestExternalFull::Item;
        using Primitive = typename TestExternalFull::Primitive;
        return this->complement();
    }
}


// Runnable wrappers for expanded Rust test bodies
// Rust-only libtest wrapper metadata: marker=tests::all::cases should_panic=no
void rusty_test_tests_all_cases() {
    tests::all::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::empty::cases should_panic=no
void rusty_test_tests_empty_cases() {
    tests::empty::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::flags::cases should_panic=no
void rusty_test_tests_flags_cases() {
    tests::flags::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::flags::external::cases should_panic=no
void rusty_test_tests_flags_external_cases() {
    tests::flags::external::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::from_bits::cases should_panic=no
void rusty_test_tests_from_bits_cases() {
    tests::from_bits::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::from_bits_retain::cases should_panic=no
void rusty_test_tests_from_bits_retain_cases() {
    tests::from_bits_retain::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::bits::cases should_panic=no
void rusty_test_tests_bits_cases() {
    tests::bits::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::complement::cases should_panic=no
void rusty_test_tests_complement_cases() {
    tests::complement::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::contains::cases should_panic=no
void rusty_test_tests_contains_cases() {
    tests::contains::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::difference::cases should_panic=no
void rusty_test_tests_difference_cases() {
    tests::difference::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::eq::cases should_panic=no
void rusty_test_tests_eq_cases() {
    tests::eq::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::extend::cases should_panic=no
void rusty_test_tests_extend_cases() {
    tests::extend::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::extend::external::cases should_panic=no
void rusty_test_tests_extend_external_cases() {
    tests::extend::external::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::fmt::cases should_panic=no
void rusty_test_tests_fmt_cases() {
    tests::fmt::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::from_bits_truncate::cases should_panic=no
void rusty_test_tests_from_bits_truncate_cases() {
    tests::from_bits_truncate::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::from_name::cases should_panic=no
void rusty_test_tests_from_name_cases() {
    tests::from_name::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::insert::cases should_panic=no
void rusty_test_tests_insert_cases() {
    tests::insert::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::intersection::cases should_panic=no
void rusty_test_tests_intersection_cases() {
    tests::intersection::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::intersects::cases should_panic=no
void rusty_test_tests_intersects_cases() {
    tests::intersects::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::is_all::cases should_panic=no
void rusty_test_tests_is_all_cases() {
    tests::is_all::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::is_empty::cases should_panic=no
void rusty_test_tests_is_empty_cases() {
    tests::is_empty::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::iter::roundtrip should_panic=no
void rusty_test_tests_iter_roundtrip() {
    tests::iter::roundtrip();
}
// Rust-only libtest wrapper metadata: marker=tests::iter::collect::cases should_panic=no
void rusty_test_tests_iter_collect_cases() {
    tests::iter::collect::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::iter::iter::cases should_panic=no
void rusty_test_tests_iter_iter_cases() {
    tests::iter::iter::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::iter::iter_names::cases should_panic=no
void rusty_test_tests_iter_iter_names_cases() {
    tests::iter::iter_names::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::parser::roundtrip should_panic=no
void rusty_test_tests_parser_roundtrip() {
    tests::parser::roundtrip();
}
// Rust-only libtest wrapper metadata: marker=tests::parser::roundtrip_truncate should_panic=no
void rusty_test_tests_parser_roundtrip_truncate() {
    tests::parser::roundtrip_truncate();
}
// Rust-only libtest wrapper metadata: marker=tests::parser::roundtrip_strict should_panic=no
void rusty_test_tests_parser_roundtrip_strict() {
    tests::parser::roundtrip_strict();
}
// Rust-only libtest wrapper metadata: marker=tests::parser::from_str::valid should_panic=no
void rusty_test_tests_parser_from_str_valid() {
    tests::parser::from_str_tests::valid();
}
// Rust-only libtest wrapper metadata: marker=tests::parser::from_str::invalid should_panic=no
void rusty_test_tests_parser_from_str_invalid() {
    tests::parser::from_str_tests::invalid();
}
// Rust-only libtest wrapper metadata: marker=tests::parser::to_writer::cases should_panic=no
void rusty_test_tests_parser_to_writer_cases() {
    tests::parser::to_writer_tests::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::parser::from_str_truncate::valid should_panic=no
void rusty_test_tests_parser_from_str_truncate_valid() {
    tests::parser::from_str_truncate_tests::valid();
}
// Rust-only libtest wrapper metadata: marker=tests::parser::to_writer_truncate::cases should_panic=no
void rusty_test_tests_parser_to_writer_truncate_cases() {
    tests::parser::to_writer_truncate_tests::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::parser::from_str_strict::valid should_panic=no
void rusty_test_tests_parser_from_str_strict_valid() {
    tests::parser::from_str_strict_tests::valid();
}
// Rust-only libtest wrapper metadata: marker=tests::parser::from_str_strict::invalid should_panic=no
void rusty_test_tests_parser_from_str_strict_invalid() {
    tests::parser::from_str_strict_tests::invalid();
}
// Rust-only libtest wrapper metadata: marker=tests::parser::to_writer_strict::cases should_panic=no
void rusty_test_tests_parser_to_writer_strict_cases() {
    tests::parser::to_writer_strict_tests::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::remove::cases should_panic=no
void rusty_test_tests_remove_cases() {
    tests::remove::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::symmetric_difference::cases should_panic=no
void rusty_test_tests_symmetric_difference_cases() {
    tests::symmetric_difference::cases();
}
// Rust-only libtest wrapper metadata: marker=tests::union::cases should_panic=no
void rusty_test_tests_union_cases() {
    tests::union_::cases();
}


// ── Test runner ──
int main(int argc, char** argv) {
    if (argc == 3 && std::string(argv[1]) == "--rusty-single-test") {
        const std::string test_name = argv[2];
        rusty::mem::clear_all_forgotten_addresses();
        try {
            if (test_name == "rusty_test_tests_all_cases") { rusty_test_tests_all_cases(); return 0; }
            if (test_name == "rusty_test_tests_bits_cases") { rusty_test_tests_bits_cases(); return 0; }
            if (test_name == "rusty_test_tests_complement_cases") { rusty_test_tests_complement_cases(); return 0; }
            if (test_name == "rusty_test_tests_contains_cases") { rusty_test_tests_contains_cases(); return 0; }
            if (test_name == "rusty_test_tests_difference_cases") { rusty_test_tests_difference_cases(); return 0; }
            if (test_name == "rusty_test_tests_empty_cases") { rusty_test_tests_empty_cases(); return 0; }
            if (test_name == "rusty_test_tests_eq_cases") { rusty_test_tests_eq_cases(); return 0; }
            if (test_name == "rusty_test_tests_extend_cases") { rusty_test_tests_extend_cases(); return 0; }
            if (test_name == "rusty_test_tests_extend_external_cases") { rusty_test_tests_extend_external_cases(); return 0; }
            if (test_name == "rusty_test_tests_flags_cases") { rusty_test_tests_flags_cases(); return 0; }
            if (test_name == "rusty_test_tests_flags_external_cases") { rusty_test_tests_flags_external_cases(); return 0; }
            if (test_name == "rusty_test_tests_fmt_cases") { rusty_test_tests_fmt_cases(); return 0; }
            if (test_name == "rusty_test_tests_from_bits_cases") { rusty_test_tests_from_bits_cases(); return 0; }
            if (test_name == "rusty_test_tests_from_bits_retain_cases") { rusty_test_tests_from_bits_retain_cases(); return 0; }
            if (test_name == "rusty_test_tests_from_bits_truncate_cases") { rusty_test_tests_from_bits_truncate_cases(); return 0; }
            if (test_name == "rusty_test_tests_from_name_cases") { rusty_test_tests_from_name_cases(); return 0; }
            if (test_name == "rusty_test_tests_insert_cases") { rusty_test_tests_insert_cases(); return 0; }
            if (test_name == "rusty_test_tests_intersection_cases") { rusty_test_tests_intersection_cases(); return 0; }
            if (test_name == "rusty_test_tests_intersects_cases") { rusty_test_tests_intersects_cases(); return 0; }
            if (test_name == "rusty_test_tests_is_all_cases") { rusty_test_tests_is_all_cases(); return 0; }
            if (test_name == "rusty_test_tests_is_empty_cases") { rusty_test_tests_is_empty_cases(); return 0; }
            if (test_name == "rusty_test_tests_iter_collect_cases") { rusty_test_tests_iter_collect_cases(); return 0; }
            if (test_name == "rusty_test_tests_iter_iter_cases") { rusty_test_tests_iter_iter_cases(); return 0; }
            if (test_name == "rusty_test_tests_iter_iter_names_cases") { rusty_test_tests_iter_iter_names_cases(); return 0; }
            if (test_name == "rusty_test_tests_iter_roundtrip") { rusty_test_tests_iter_roundtrip(); return 0; }
            if (test_name == "rusty_test_tests_parser_from_str_invalid") { rusty_test_tests_parser_from_str_invalid(); return 0; }
            if (test_name == "rusty_test_tests_parser_from_str_strict_invalid") { rusty_test_tests_parser_from_str_strict_invalid(); return 0; }
            if (test_name == "rusty_test_tests_parser_from_str_strict_valid") { rusty_test_tests_parser_from_str_strict_valid(); return 0; }
            if (test_name == "rusty_test_tests_parser_from_str_truncate_valid") { rusty_test_tests_parser_from_str_truncate_valid(); return 0; }
            if (test_name == "rusty_test_tests_parser_from_str_valid") { rusty_test_tests_parser_from_str_valid(); return 0; }
            if (test_name == "rusty_test_tests_parser_roundtrip") { rusty_test_tests_parser_roundtrip(); return 0; }
            if (test_name == "rusty_test_tests_parser_roundtrip_strict") { rusty_test_tests_parser_roundtrip_strict(); return 0; }
            if (test_name == "rusty_test_tests_parser_roundtrip_truncate") { rusty_test_tests_parser_roundtrip_truncate(); return 0; }
            if (test_name == "rusty_test_tests_parser_to_writer_cases") { rusty_test_tests_parser_to_writer_cases(); return 0; }
            if (test_name == "rusty_test_tests_parser_to_writer_strict_cases") { rusty_test_tests_parser_to_writer_strict_cases(); return 0; }
            if (test_name == "rusty_test_tests_parser_to_writer_truncate_cases") { rusty_test_tests_parser_to_writer_truncate_cases(); return 0; }
            if (test_name == "rusty_test_tests_remove_cases") { rusty_test_tests_remove_cases(); return 0; }
            if (test_name == "rusty_test_tests_symmetric_difference_cases") { rusty_test_tests_symmetric_difference_cases(); return 0; }
            if (test_name == "rusty_test_tests_union_cases") { rusty_test_tests_union_cases(); return 0; }
            std::cerr << "Unknown single-test wrapper: " << test_name << std::endl;
            return 64;
        } catch (const std::exception& e) {
            std::cerr << e.what() << std::endl;
            return 101;
        } catch (...) {
            return 102;
        }
    }
    int pass = 0, fail = 0;
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_all_cases(); std::cout << "  tests_all_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_all_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_all_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_bits_cases(); std::cout << "  tests_bits_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_bits_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_bits_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_complement_cases(); std::cout << "  tests_complement_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_complement_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_complement_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_contains_cases(); std::cout << "  tests_contains_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_contains_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_contains_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_difference_cases(); std::cout << "  tests_difference_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_difference_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_difference_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_empty_cases(); std::cout << "  tests_empty_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_empty_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_empty_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_eq_cases(); std::cout << "  tests_eq_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_eq_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_eq_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_extend_cases(); std::cout << "  tests_extend_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_extend_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_extend_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_extend_external_cases(); std::cout << "  tests_extend_external_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_extend_external_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_extend_external_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_flags_cases(); std::cout << "  tests_flags_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_flags_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_flags_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_flags_external_cases(); std::cout << "  tests_flags_external_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_flags_external_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_flags_external_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_fmt_cases(); std::cout << "  tests_fmt_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_fmt_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_fmt_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_from_bits_cases(); std::cout << "  tests_from_bits_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_from_bits_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_from_bits_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_from_bits_retain_cases(); std::cout << "  tests_from_bits_retain_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_from_bits_retain_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_from_bits_retain_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_from_bits_truncate_cases(); std::cout << "  tests_from_bits_truncate_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_from_bits_truncate_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_from_bits_truncate_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_from_name_cases(); std::cout << "  tests_from_name_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_from_name_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_from_name_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_insert_cases(); std::cout << "  tests_insert_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_insert_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_insert_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_intersection_cases(); std::cout << "  tests_intersection_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_intersection_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_intersection_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_intersects_cases(); std::cout << "  tests_intersects_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_intersects_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_intersects_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_is_all_cases(); std::cout << "  tests_is_all_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_is_all_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_is_all_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_is_empty_cases(); std::cout << "  tests_is_empty_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_is_empty_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_is_empty_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_iter_collect_cases(); std::cout << "  tests_iter_collect_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_iter_collect_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_iter_collect_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_iter_iter_cases(); std::cout << "  tests_iter_iter_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_iter_iter_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_iter_iter_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_iter_iter_names_cases(); std::cout << "  tests_iter_iter_names_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_iter_iter_names_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_iter_iter_names_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_iter_roundtrip(); std::cout << "  tests_iter_roundtrip PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_iter_roundtrip FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_iter_roundtrip FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_parser_from_str_invalid(); std::cout << "  tests_parser_from_str_invalid PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_parser_from_str_invalid FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_parser_from_str_invalid FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_parser_from_str_strict_invalid(); std::cout << "  tests_parser_from_str_strict_invalid PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_parser_from_str_strict_invalid FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_parser_from_str_strict_invalid FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_parser_from_str_strict_valid(); std::cout << "  tests_parser_from_str_strict_valid PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_parser_from_str_strict_valid FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_parser_from_str_strict_valid FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_parser_from_str_truncate_valid(); std::cout << "  tests_parser_from_str_truncate_valid PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_parser_from_str_truncate_valid FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_parser_from_str_truncate_valid FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_parser_from_str_valid(); std::cout << "  tests_parser_from_str_valid PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_parser_from_str_valid FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_parser_from_str_valid FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_parser_roundtrip(); std::cout << "  tests_parser_roundtrip PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_parser_roundtrip FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_parser_roundtrip FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_parser_roundtrip_strict(); std::cout << "  tests_parser_roundtrip_strict PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_parser_roundtrip_strict FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_parser_roundtrip_strict FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_parser_roundtrip_truncate(); std::cout << "  tests_parser_roundtrip_truncate PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_parser_roundtrip_truncate FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_parser_roundtrip_truncate FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_parser_to_writer_cases(); std::cout << "  tests_parser_to_writer_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_parser_to_writer_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_parser_to_writer_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_parser_to_writer_strict_cases(); std::cout << "  tests_parser_to_writer_strict_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_parser_to_writer_strict_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_parser_to_writer_strict_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_parser_to_writer_truncate_cases(); std::cout << "  tests_parser_to_writer_truncate_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_parser_to_writer_truncate_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_parser_to_writer_truncate_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_remove_cases(); std::cout << "  tests_remove_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_remove_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_remove_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_symmetric_difference_cases(); std::cout << "  tests_symmetric_difference_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_symmetric_difference_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_symmetric_difference_cases FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_tests_union_cases(); std::cout << "  tests_union_cases PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  tests_union_cases FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  tests_union_cases FAILED (unknown exception)" << std::endl; fail++; }
    std::cout << std::endl;
    std::cout << "Results: " << pass << " passed, " << fail << " failed" << std::endl;
    return fail > 0 ? 1 : 0;
}
