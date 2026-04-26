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
enum class Op;
namespace error { enum class Position; }
namespace error { struct ErrorKind_Empty; }
namespace error { struct ErrorKind_EmptySegment; }
namespace error { struct ErrorKind_ExcessiveComparators; }
namespace error { struct ErrorKind_ExpectedCommaFound; }
namespace error { struct ErrorKind_IllegalCharacter; }
namespace error { struct ErrorKind_LeadingZero; }
namespace error { struct ErrorKind_Overflow; }
namespace error { struct ErrorKind_UnexpectedAfterWildcard; }
namespace error { struct ErrorKind_UnexpectedChar; }
namespace error { struct ErrorKind_UnexpectedCharAfter; }
namespace error { struct ErrorKind_UnexpectedEnd; }
namespace error { struct ErrorKind_WildcardNotTheOnlyComparator; }
namespace error { struct QuotedChar; }
namespace identifier { struct Identifier; }
namespace parse { struct Error; }
struct BuildMetadata;
struct Comparator;
struct Prerelease;
struct Version;
struct VersionReq;

// ── from semver.cppm ──



#if defined(__GNUC__)
#pragma GCC diagnostic ignored "-Wunused-local-typedefs"
#endif


namespace display {}
namespace error {}
namespace identifier {}
namespace impls {}
namespace parse {}

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
std::string join(const Range& range, Sep&& sep) {
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



enum class Op;
constexpr Op Op_Exact();
constexpr Op Op_Greater();
constexpr Op Op_GreaterEq();
constexpr Op Op_Less();
constexpr Op Op_LessEq();
constexpr Op Op_Tilde();
constexpr Op Op_Caret();
constexpr Op Op_Wildcard();
Op clone(const Op& self_);
void assert_receiver_is_total_eq(const Op& self_);
bool eq(const Op& self_, const Op& other);
template<typename __H>
void hash(const Op& self_, __H& state);
struct Prerelease;
struct Comparator;
struct VersionReq;
struct BuildMetadata;
struct Version;
namespace identifier {
    struct Identifier;
    extern const size_t PTR_BYTES;
    extern const size_t TAIL_BYTES;
    rusty::ptr::NonNull<uint8_t> ptr_to_repr(uint8_t* original);
    const uint8_t* repr_to_ptr(rusty::ptr::NonNull<uint8_t> modified);
    uint8_t* repr_to_ptr_mut(rusty::ptr::NonNull<uint8_t> repr);
    rusty::num::NonZeroUsize inline_len(const Identifier& repr);
    std::string_view inline_as_str(const Identifier& repr);
    rusty::num::NonZeroUsize decode_len(const uint8_t* ptr);
    std::string_view ptr_as_str(const rusty::ptr::NonNull<uint8_t>& repr);
    size_t bytes_for_varint(rusty::num::NonZeroUsize len);
}
namespace backport {
}
namespace eval {
    bool matches_req(const ::VersionReq& req, const ::Version& ver);
    bool matches_comparator(const ::Comparator& cmp, const ::Version& ver);
    bool matches_impl(const ::Comparator& cmp, const ::Version& ver);
    bool matches_exact(const ::Comparator& cmp, const ::Version& ver);
    bool matches_greater(const ::Comparator& cmp, const ::Version& ver);
    bool matches_less(const ::Comparator& cmp, const ::Version& ver);
    bool matches_tilde(const ::Comparator& cmp, const ::Version& ver);
    bool matches_caret(const ::Comparator& cmp, const ::Version& ver);
    bool pre_is_compatible(const ::Comparator& cmp, const ::Version& ver);
}
namespace display {
    rusty::fmt::Result pad(rusty::fmt::Formatter& formatter, const auto& do_display, const auto& do_len);
    size_t digits(uint64_t val);
}
namespace impls {
    using identifier::Identifier;
}
namespace error {
    enum class Position;
    constexpr Position Position_Major();
    constexpr Position Position_Minor();
    constexpr Position Position_Patch();
    constexpr Position Position_Pre();
    constexpr Position Position_Build();
    Position clone(const Position& self_);
    void assert_receiver_is_total_eq(const Position& self_);
    bool eq(const Position& self_, const Position& other);
    struct ErrorKind_Empty;
    struct ErrorKind_UnexpectedEnd;
    struct ErrorKind_UnexpectedChar;
    struct ErrorKind_UnexpectedCharAfter;
    struct ErrorKind_ExpectedCommaFound;
    struct ErrorKind_LeadingZero;
    struct ErrorKind_Overflow;
    struct ErrorKind_EmptySegment;
    struct ErrorKind_IllegalCharacter;
    struct ErrorKind_WildcardNotTheOnlyComparator;
    struct ErrorKind_UnexpectedAfterWildcard;
    struct ErrorKind_ExcessiveComparators;
    using ErrorKind = std::variant<ErrorKind_Empty, ErrorKind_UnexpectedEnd, ErrorKind_UnexpectedChar, ErrorKind_UnexpectedCharAfter, ErrorKind_ExpectedCommaFound, ErrorKind_LeadingZero, ErrorKind_Overflow, ErrorKind_EmptySegment, ErrorKind_IllegalCharacter, ErrorKind_WildcardNotTheOnlyComparator, ErrorKind_UnexpectedAfterWildcard, ErrorKind_ExcessiveComparators>;
    struct QuotedChar;
    using parse::Error;
}
namespace parse {
    struct Error;
    using error::ErrorKind;
    using error::Position;
    using identifier::Identifier;
    rusty::Result<std::tuple<uint64_t, std::string_view>, Error> numeric_identifier(std::string_view input, ::error::Position pos);
    rusty::Option<std::tuple<char32_t, std::string_view>> wildcard(std::string_view input);
    rusty::Result<std::string_view, Error> dot(std::string_view input, ::error::Position pos);
    rusty::Result<std::tuple<::Prerelease, std::string_view>, Error> prerelease_identifier(std::string_view input);
    rusty::Result<std::tuple<::BuildMetadata, std::string_view>, Error> build_identifier(std::string_view input);
    rusty::Result<std::tuple<std::string_view, std::string_view>, Error> identifier(std::string_view input, ::error::Position pos);
    std::tuple<::Op, std::string_view> op(std::string_view input);
    rusty::Result<std::tuple<::Comparator, ::error::Position, std::string_view>, Error> comparator(std::string_view input);
    rusty::Result<size_t, Error> version_req(std::string_view input, rusty::Vec<::Comparator>& out, size_t depth);
}
using identifier::Identifier;
using parse::Error;

enum class Op {
    Exact,
    Greater,
    GreaterEq,
    Less,
    LessEq,
    Tilde,
    Caret,
    Wildcard
};
inline constexpr Op Op_Exact() { return Op::Exact; }
inline constexpr Op Op_Greater() { return Op::Greater; }
inline constexpr Op Op_GreaterEq() { return Op::GreaterEq; }
inline constexpr Op Op_Less() { return Op::Less; }
inline constexpr Op Op_LessEq() { return Op::LessEq; }
inline constexpr Op Op_Tilde() { return Op::Tilde; }
inline constexpr Op Op_Caret() { return Op::Caret; }
inline constexpr Op Op_Wildcard() { return Op::Wildcard; }
inline constexpr auto Op_DEFAULT = Op_Caret();
inline Op clone(const Op& self_) {
    return self_;
}
inline void assert_receiver_is_total_eq(const Op& self_) {
}
inline bool eq(const Op& self_, const Op& other) {
    const auto __self_discr = rusty::intrinsics::discriminant_value(self_);
    const auto __arg1_discr = rusty::intrinsics::discriminant_value(other);
    return __self_discr == __arg1_discr;
}
template<typename __H>
inline void hash(const Op& self_, __H& state) {
    const auto __self_discr = rusty::intrinsics::discriminant_value(self_);
    rusty::hash::hash(__self_discr, state);
}


namespace backport {

    using ::rusty::Vec;

}

namespace display {

    rusty::fmt::Result pad(rusty::fmt::Formatter& formatter, const auto& do_display, const auto& do_len);
    size_t digits(uint64_t val);

    using ::BuildMetadata;
    using ::Comparator;
    using ::Op;
    using ::Prerelease;
    using ::Version;
    using ::VersionReq;

    namespace fmt = rusty::fmt;
    using ::rusty::fmt::Alignment;

    rusty::fmt::Result pad(rusty::fmt::Formatter& formatter, const auto& do_display, const auto& do_len) {
        const auto min_width = ({ auto&& _m = formatter.width(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& min_width = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(min_width)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return do_display(formatter); } std::move(_match_value).value(); });
        const auto len = do_len();
        if (len >= min_width) {
            return do_display(formatter);
        }
        const auto default_align = Alignment::Left;
        const auto align = formatter.align().unwrap_or(std::move(default_align));
        auto padding = min_width - len;
        auto [pre_pad, post_pad] = rusty::detail::deref_if_pointer_like(({ auto&& _m = align; std::optional<std::tuple<int32_t, int32_t>> _match_value; bool _m_matched = false; if (!_m_matched && (_m == Alignment::Left)) { _match_value.emplace(std::move(std::make_tuple(static_cast<std::remove_cvref_t<decltype((padding / 2))>>(0), std::move(padding)))); _m_matched = true; } if (!_m_matched && (_m == Alignment::Right)) { _match_value.emplace(std::move(std::make_tuple(std::move(padding), static_cast<std::remove_cvref_t<decltype((((padding + 1)) / 2))>>(0)))); _m_matched = true; } if (!_m_matched && (_m == Alignment::Center)) { _match_value.emplace(std::move(std::make_tuple(padding / 2, ((padding + 1)) / 2))); _m_matched = true; } if (!_m_matched) { rusty::intrinsics::unreachable(); } std::move(_match_value).value(); }));
        const auto fill = formatter.fill();
        for (auto&& _ : rusty::for_in(rusty::range(0, pre_pad))) {
            RUSTY_TRY(formatter.write_char(std::move(fill)));
        }
        RUSTY_TRY(do_display(formatter));
        for (auto&& _ : rusty::for_in(rusty::range(0, post_pad))) {
            RUSTY_TRY(formatter.write_char(std::move(fill)));
        }
        return rusty::fmt::Result::Ok(std::make_tuple());
    }

    size_t digits(uint64_t val) {
        if (val < 10) {
            return static_cast<size_t>(1);
        } else {
            return 1 + digits(val / 10);
        }
    }

}

namespace error {

    enum class Position;
    constexpr Position Position_Major();
    constexpr Position Position_Minor();
    constexpr Position Position_Patch();
    constexpr Position Position_Pre();
    constexpr Position Position_Build();
    Position clone(const Position& self_);
    void assert_receiver_is_total_eq(const Position& self_);
    bool eq(const Position& self_, const Position& other);
    struct ErrorKind_Empty;
    struct ErrorKind_UnexpectedEnd;
    struct ErrorKind_UnexpectedChar;
    struct ErrorKind_UnexpectedCharAfter;
    struct ErrorKind_ExpectedCommaFound;
    struct ErrorKind_LeadingZero;
    struct ErrorKind_Overflow;
    struct ErrorKind_EmptySegment;
    struct ErrorKind_IllegalCharacter;
    struct ErrorKind_WildcardNotTheOnlyComparator;
    struct ErrorKind_UnexpectedAfterWildcard;
    struct ErrorKind_ExcessiveComparators;
    using ErrorKind = std::variant<ErrorKind_Empty, ErrorKind_UnexpectedEnd, ErrorKind_UnexpectedChar, ErrorKind_UnexpectedCharAfter, ErrorKind_ExpectedCommaFound, ErrorKind_LeadingZero, ErrorKind_Overflow, ErrorKind_EmptySegment, ErrorKind_IllegalCharacter, ErrorKind_WildcardNotTheOnlyComparator, ErrorKind_UnexpectedAfterWildcard, ErrorKind_ExcessiveComparators>;
    struct QuotedChar;
    using parse::Error;

    enum class Position {
        Major,
    Minor,
    Patch,
    Pre,
    Build
    };
    inline constexpr Position Position_Major() { return Position::Major; }
    inline constexpr Position Position_Minor() { return Position::Minor; }
    inline constexpr Position Position_Patch() { return Position::Patch; }
    inline constexpr Position Position_Pre() { return Position::Pre; }
    inline constexpr Position Position_Build() { return Position::Build; }
    inline Position clone(const Position& self_) {
        return self_;
    }
    inline void assert_receiver_is_total_eq(const Position& self_) {
    }
    inline bool eq(const Position& self_, const Position& other) {
        const auto __self_discr = rusty::intrinsics::discriminant_value(self_);
        const auto __arg1_discr = rusty::intrinsics::discriminant_value(other);
        return __self_discr == __arg1_discr;
    }
    inline rusty::fmt::Result rusty_fmt(const Position& self, rusty::fmt::Formatter& formatter) {
        return formatter.write_str(({ auto&& _m = self; std::optional<std::string_view> _match_value; bool _m_matched = false; if (!_m_matched && (_m == Position::Major)) { _match_value.emplace(std::move("major version number")); _m_matched = true; } if (!_m_matched && (_m == Position::Minor)) { _match_value.emplace(std::move("minor version number")); _m_matched = true; } if (!_m_matched && (_m == Position::Patch)) { _match_value.emplace(std::move("patch version number")); _m_matched = true; } if (!_m_matched && (_m == Position::Pre)) { _match_value.emplace(std::move("pre-release identifier")); _m_matched = true; } if (!_m_matched && (_m == Position::Build)) { _match_value.emplace(std::move("build metadata")); _m_matched = true; } if (!_m_matched) { rusty::intrinsics::unreachable(); } std::move(_match_value).value(); }));
    }

    using parse::Error;

    namespace fmt = rusty::fmt;

    // Algebraic data type
    struct ErrorKind_Empty {};
    struct ErrorKind_UnexpectedEnd {
        Position _0;
    };
    struct ErrorKind_UnexpectedChar {
        Position _0;
        char32_t _1;
    };
    struct ErrorKind_UnexpectedCharAfter {
        Position _0;
        char32_t _1;
    };
    struct ErrorKind_ExpectedCommaFound {
        Position _0;
        char32_t _1;
    };
    struct ErrorKind_LeadingZero {
        Position _0;
    };
    struct ErrorKind_Overflow {
        Position _0;
    };
    struct ErrorKind_EmptySegment {
        Position _0;
    };
    struct ErrorKind_IllegalCharacter {
        Position _0;
    };
    struct ErrorKind_WildcardNotTheOnlyComparator {
        char32_t _0;
    };
    struct ErrorKind_UnexpectedAfterWildcard {};
    struct ErrorKind_ExcessiveComparators {};
    ErrorKind_Empty Empty();
    ErrorKind_UnexpectedEnd UnexpectedEnd(Position _0);
    ErrorKind_UnexpectedChar UnexpectedChar(Position _0, char32_t _1);
    ErrorKind_UnexpectedCharAfter UnexpectedCharAfter(Position _0, char32_t _1);
    ErrorKind_ExpectedCommaFound ExpectedCommaFound(Position _0, char32_t _1);
    ErrorKind_LeadingZero LeadingZero(Position _0);
    ErrorKind_Overflow Overflow(Position _0);
    ErrorKind_EmptySegment EmptySegment(Position _0);
    ErrorKind_IllegalCharacter IllegalCharacter(Position _0);
    ErrorKind_WildcardNotTheOnlyComparator WildcardNotTheOnlyComparator(char32_t _0);
    ErrorKind_UnexpectedAfterWildcard UnexpectedAfterWildcard();
    ErrorKind_ExcessiveComparators ExcessiveComparators();
    using ErrorKind = std::variant<ErrorKind_Empty, ErrorKind_UnexpectedEnd, ErrorKind_UnexpectedChar, ErrorKind_UnexpectedCharAfter, ErrorKind_ExpectedCommaFound, ErrorKind_LeadingZero, ErrorKind_Overflow, ErrorKind_EmptySegment, ErrorKind_IllegalCharacter, ErrorKind_WildcardNotTheOnlyComparator, ErrorKind_UnexpectedAfterWildcard, ErrorKind_ExcessiveComparators>;
    ErrorKind_Empty Empty() { return ErrorKind_Empty{};  }
    ErrorKind_UnexpectedEnd UnexpectedEnd(Position _0) { return ErrorKind_UnexpectedEnd{std::forward<Position>(_0)};  }
    ErrorKind_UnexpectedChar UnexpectedChar(Position _0, char32_t _1) { return ErrorKind_UnexpectedChar{std::forward<Position>(_0), std::forward<char32_t>(_1)};  }
    ErrorKind_UnexpectedCharAfter UnexpectedCharAfter(Position _0, char32_t _1) { return ErrorKind_UnexpectedCharAfter{std::forward<Position>(_0), std::forward<char32_t>(_1)};  }
    ErrorKind_ExpectedCommaFound ExpectedCommaFound(Position _0, char32_t _1) { return ErrorKind_ExpectedCommaFound{std::forward<Position>(_0), std::forward<char32_t>(_1)};  }
    ErrorKind_LeadingZero LeadingZero(Position _0) { return ErrorKind_LeadingZero{std::forward<Position>(_0)};  }
    ErrorKind_Overflow Overflow(Position _0) { return ErrorKind_Overflow{std::forward<Position>(_0)};  }
    ErrorKind_EmptySegment EmptySegment(Position _0) { return ErrorKind_EmptySegment{std::forward<Position>(_0)};  }
    ErrorKind_IllegalCharacter IllegalCharacter(Position _0) { return ErrorKind_IllegalCharacter{std::forward<Position>(_0)};  }
    ErrorKind_WildcardNotTheOnlyComparator WildcardNotTheOnlyComparator(char32_t _0) { return ErrorKind_WildcardNotTheOnlyComparator{std::forward<char32_t>(_0)};  }
    ErrorKind_UnexpectedAfterWildcard UnexpectedAfterWildcard() { return ErrorKind_UnexpectedAfterWildcard{};  }
    ErrorKind_ExcessiveComparators ExcessiveComparators() { return ErrorKind_ExcessiveComparators{};  }

    struct QuotedChar {
        char32_t _0;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const;
    };

}

namespace identifier {

    struct Identifier;
    extern const size_t PTR_BYTES;
    extern const size_t TAIL_BYTES;
    rusty::ptr::NonNull<uint8_t> ptr_to_repr(uint8_t* original);
    const uint8_t* repr_to_ptr(rusty::ptr::NonNull<uint8_t> modified);
    uint8_t* repr_to_ptr_mut(rusty::ptr::NonNull<uint8_t> repr);
    rusty::num::NonZeroUsize inline_len(const Identifier& repr);
    std::string_view inline_as_str(const Identifier& repr);
    rusty::num::NonZeroUsize decode_len(const uint8_t* ptr);
    std::string_view ptr_as_str(const rusty::ptr::NonNull<uint8_t>& repr);
    size_t bytes_for_varint(rusty::num::NonZeroUsize len);

    using ::rusty::alloc::alloc;
    using ::rusty::alloc::dealloc;
    using ::rusty::alloc::handle_alloc_error;
    using ::rusty::alloc::Layout;


    namespace mem = rusty::mem;

    using ::rusty::num::NonZeroU64;
    using ::rusty::num::NonZeroUsize;

    namespace ptr = rusty::ptr;
    using ::rusty::ptr::NonNull;


    namespace str = rusty::str_runtime;


    constexpr size_t PTR_BYTES = rusty::mem::size_of<rusty::ptr::NonNull<uint8_t>>();

    constexpr size_t TAIL_BYTES = (8 * (static_cast<size_t>((PTR_BYTES < 8)))) - (PTR_BYTES * (static_cast<size_t>((PTR_BYTES < 8))));

    struct Identifier {
        rusty::ptr::NonNull<uint8_t> head;
        std::array<uint8_t, rusty::sanitize_array_capacity<TAIL_BYTES>()> tail;
        Identifier(rusty::ptr::NonNull<uint8_t> head_init, std::array<uint8_t, rusty::sanitize_array_capacity<TAIL_BYTES>()> tail_init) : head(std::move(head_init)), tail(std::move(tail_init)) {}
        Identifier(const Identifier&) = default;
        Identifier(Identifier&& other) noexcept : head(std::move(other.head)), tail(std::move(other.tail)) {
            if (rusty::mem::consume_forgotten_address(&other)) {
                this->rusty_mark_forgotten();
                other.rusty_mark_forgotten();
            } else {
                other.rusty_mark_forgotten();
            }
        }
        Identifier& operator=(const Identifier&) = default;
        Identifier& operator=(Identifier&& other) noexcept {
            if (this == &other) {
                return *this;
            }
            this->~Identifier();
            new (this) Identifier(std::move(other));
            return *this;
        }
        void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


        static Identifier empty();
        static Identifier new_unchecked(std::string_view string);
        bool is_empty() const;
        bool is_inline() const;
        bool is_empty_or_inline() const;
        std::string_view as_str() const;
        bool ptr_eq(const Identifier& rhs) const;
        Identifier clone() const;
        ~Identifier() noexcept(false);
        bool operator==(const Identifier& rhs) const;
        static Identifier default_();
        template<typename H>
        void hash(H& hasher) const;
    };

}

namespace impls {

    using identifier::Identifier;

    using namespace backport;

    using identifier::Identifier;

    using ::BuildMetadata;
    using ::Comparator;
    using ::Prerelease;
    using ::VersionReq;

    using ::rusty::cmp::Ordering;




}

namespace parse {

    struct Error;
    using error::ErrorKind;
    using error::Position;
    using identifier::Identifier;
    rusty::Result<std::tuple<uint64_t, std::string_view>, Error> numeric_identifier(std::string_view input, ::error::Position pos);
    rusty::Option<std::tuple<char32_t, std::string_view>> wildcard(std::string_view input);
    rusty::Result<std::string_view, Error> dot(std::string_view input, ::error::Position pos);
    rusty::Result<std::tuple<::Prerelease, std::string_view>, Error> prerelease_identifier(std::string_view input);
    rusty::Result<std::tuple<::BuildMetadata, std::string_view>, Error> build_identifier(std::string_view input);
    rusty::Result<std::tuple<std::string_view, std::string_view>, Error> identifier(std::string_view input, ::error::Position pos);
    std::tuple<::Op, std::string_view> op(std::string_view input);
    rusty::Result<std::tuple<::Comparator, ::error::Position, std::string_view>, Error> comparator(std::string_view input);
    rusty::Result<size_t, Error> version_req(std::string_view input, rusty::Vec<::Comparator>& out, size_t depth);

    using namespace backport;

    using error::ErrorKind;
    using namespace error;
    using error::Position;

    using identifier::Identifier;

    using ::BuildMetadata;
    using ::Comparator;
    using ::Op;
    using ::Prerelease;
    using ::Version;
    using ::VersionReq;


    /// Error parsing a SemVer version or version requirement.
    ///
    /// # Example
    ///
    /// ```
    /// use semver::Version;
    ///
    /// fn main() {
    ///     let err = Version::parse("1.q.r").unwrap_err();
    ///
    ///     // "unexpected character 'q' while parsing minor version number"
    ///     eprintln!("{}", err);
    /// }
    /// ```
    struct Error {
        error::ErrorKind kind;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const;
        static Error new_(error::ErrorKind kind);
    };

}

using identifier::Identifier;

using ::rusty::cmp::Ordering;


namespace backport {}
using namespace ::backport;

using parse::Error;

/// Optional pre-release identifier on a version string. This comes after `-` in
/// a SemVer version, like `1.0.0-alpha.1`
///
/// # Examples
///
/// Some real world pre-release idioms drawn from crates.io:
///
/// - **[mio]** <code>0.7.0-<b>alpha.1</b></code> &mdash; the most common style
///   for numbering pre-releases.
///
/// - **[pest]** <code>1.0.0-<b>beta.8</b></code>,&ensp;<code>1.0.0-<b>rc.0</b></code>
///   &mdash; this crate makes a distinction between betas and release
///   candidates.
///
/// - **[sassers]** <code>0.11.0-<b>shitshow</b></code> &mdash; ???.
///
/// - **[atomic-utils]** <code>0.0.0-<b>reserved</b></code> &mdash; a squatted
///   crate name.
///
/// [mio]: https://crates.io/crates/mio
/// [pest]: https://crates.io/crates/pest
/// [atomic-utils]: https://crates.io/crates/atomic-utils
/// [sassers]: https://crates.io/crates/sassers
///
/// *Tip:* Be aware that if you are planning to number your own pre-releases,
/// you should prefer to separate the numeric part from any non-numeric
/// identifiers by using a dot in between. That is, prefer pre-releases
/// `alpha.1`, `alpha.2`, etc rather than `alpha1`, `alpha2` etc. The SemVer
/// spec's rule for pre-release precedence has special treatment of numeric
/// components in the pre-release string, but only if there are no non-digit
/// characters in the same dot-separated component. So you'd have `alpha.2` &lt;
/// `alpha.11` as intended, but `alpha11` &lt; `alpha2`.
///
/// # Syntax
///
/// Pre-release strings are a series of dot separated identifiers immediately
/// following the patch version. Identifiers must comprise only ASCII
/// alphanumerics and hyphens: `0-9`, `A-Z`, `a-z`, `-`. Identifiers must not be
/// empty. Numeric identifiers must not include leading zeros.
///
/// # Total ordering
///
/// Pre-releases have a total order defined by the SemVer spec. It uses
/// lexicographic ordering of dot-separated components. Identifiers consisting
/// of only digits are compared numerically. Otherwise, identifiers are compared
/// in ASCII sort order. Any numeric identifier is always less than any
/// non-numeric identifier.
///
/// Example:&ensp;`alpha`&ensp;&lt;&ensp;`alpha.85`&ensp;&lt;&ensp;`alpha.90`&ensp;&lt;&ensp;`alpha.200`&ensp;&lt;&ensp;`alpha.0a`&ensp;&lt;&ensp;`alpha.1a0`&ensp;&lt;&ensp;`alpha.a`&ensp;&lt;&ensp;`beta`
struct Prerelease {
    using Target = std::string_view;
    using Err = Error;
    Identifier identifier;

    rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const;
    std::string_view operator*() const;
    std::partial_ordering operator<=>(const Prerelease& rhs) const;
    rusty::cmp::Ordering cmp(const Prerelease& rhs) const;
    static rusty::Result<Prerelease, Error> from_str(std::string_view text);
    static Prerelease default_();
    Prerelease clone() const;
    void assert_receiver_is_total_eq() const;
    bool operator==(const Prerelease& other) const;
    template<typename __H>
    void hash(__H& state) const;
    static const Prerelease EMPTY;
    static rusty::Result<Prerelease, Error> new_(std::string_view text);
    std::string_view as_str() const;
    bool is_empty() const;
};
inline const Prerelease Prerelease::EMPTY = Prerelease(Identifier::empty());

/// A pair of comparison operator and partial version, such as `>=1.2`. Forms
/// one piece of a VersionReq.
struct Comparator {
    using Err = Error;
    Op op;
    uint64_t major;
    rusty::Option<uint64_t> minor;
    /// Patch is only allowed if minor is Some.
    rusty::Option<uint64_t> patch;
    /// Non-empty pre-release is only allowed if patch is Some.
    Prerelease pre;

    rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const;
    static rusty::Result<Comparator, Error> from_str(std::string_view text);
    Comparator clone() const;
    void assert_receiver_is_total_eq() const;
    bool operator==(const Comparator& other) const;
    template<typename __H>
    void hash(__H& state) const;
    static rusty::Result<Comparator, Error> parse(std::string_view text);
    bool matches(const Version& version) const;
};

/// **SemVer version requirement** describing the intersection of some version
/// comparators, such as `>=1.2.3, <1.8`.
///
/// # Syntax
///
/// - Either `*` (meaning "any"), or one or more comma-separated comparators.
///
/// - A [`Comparator`] is an operator ([`Op`]) and a partial version, separated
///   by optional whitespace. For example `>=1.0.0` or `>=1.0`.
///
/// - Build metadata is syntactically permitted on the partial versions, but is
///   completely ignored, as it's never relevant to whether any comparator
///   matches a particular version.
///
/// - Whitespace is permitted around commas and around operators. Whitespace is
///   not permitted within a partial version, i.e. anywhere between the major
///   version number and its minor, patch, pre-release, or build metadata.
struct VersionReq {
    using Err = Error;
    rusty::Vec<Comparator> comparators;

    rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const;
    template<typename I>
    static VersionReq from_iter(I iter);
    static rusty::Result<VersionReq, Error> from_str(std::string_view text);
    VersionReq clone() const;
    void assert_receiver_is_total_eq() const;
    bool operator==(const VersionReq& other) const;
    template<typename __H>
    void hash(__H& state) const;
    static const VersionReq STAR;
    static rusty::Result<VersionReq, Error> parse(std::string_view text);
    bool matches(const Version& version) const;
    static VersionReq default_();
};
inline const VersionReq VersionReq::STAR = VersionReq(rusty::Vec<Comparator>::new_());

/// Optional build metadata identifier. This comes after `+` in a SemVer
/// version, as in `0.8.1+zstd.1.5.0`.
///
/// # Examples
///
/// Some real world build metadata idioms drawn from crates.io:
///
/// - **[libgit2-sys]** <code>0.12.20+<b>1.1.0</b></code> &mdash; for this
///   crate, the build metadata indicates the version of the C libgit2 library
///   that the Rust crate is built against.
///
/// - **[mashup]** <code>0.1.13+<b>deprecated</b></code> &mdash; just the word
///   "deprecated" for a crate that has been superseded by another. Eventually
///   people will take notice of this in Cargo's build output where it lists the
///   crates being compiled.
///
/// - **[google-bigquery2]** <code>2.0.4+<b>20210327</b></code> &mdash; this
///   library is automatically generated from an official API schema, and the
///   build metadata indicates the date on which that schema was last captured.
///
/// - **[fbthrift-git]** <code>0.0.6+<b>c7fcc0e</b></code> &mdash; this crate is
///   published from snapshots of a big company monorepo. In monorepo
///   development, there is no concept of versions, and all downstream code is
///   just updated atomically in the same commit that breaking changes to a
///   library are landed. Therefore for crates.io purposes, every published
///   version must be assumed to be incompatible with the previous. The build
///   metadata provides the source control hash of the snapshotted code.
///
/// [libgit2-sys]: https://crates.io/crates/libgit2-sys
/// [mashup]: https://crates.io/crates/mashup
/// [google-bigquery2]: https://crates.io/crates/google-bigquery2
/// [fbthrift-git]: https://crates.io/crates/fbthrift-git
///
/// # Syntax
///
/// Build metadata is a series of dot separated identifiers immediately
/// following the patch or pre-release version. Identifiers must comprise only
/// ASCII alphanumerics and hyphens: `0-9`, `A-Z`, `a-z`, `-`. Identifiers must
/// not be empty. Leading zeros *are* allowed, unlike any other place in the
/// SemVer grammar.
///
/// # Total ordering
///
/// Build metadata is ignored in evaluating `VersionReq`; it plays no role in
/// whether a `Version` matches any one of the comparison operators.
///
/// However for comparing build metadatas among one another, they do have a
/// total order which is determined by lexicographic ordering of dot-separated
/// components. Identifiers consisting of only digits are compared numerically.
/// Otherwise, identifiers are compared in ASCII sort order. Any numeric
/// identifier is always less than any non-numeric identifier.
///
/// Example:&ensp;`demo`&ensp;&lt;&ensp;`demo.85`&ensp;&lt;&ensp;`demo.90`&ensp;&lt;&ensp;`demo.090`&ensp;&lt;&ensp;`demo.200`&ensp;&lt;&ensp;`demo.1a0`&ensp;&lt;&ensp;`demo.a`&ensp;&lt;&ensp;`memo`
struct BuildMetadata {
    using Target = std::string_view;
    using Err = Error;
    Identifier identifier;

    rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const;
    std::string_view operator*() const;
    std::partial_ordering operator<=>(const BuildMetadata& rhs) const;
    rusty::cmp::Ordering cmp(const BuildMetadata& rhs) const;
    static rusty::Result<BuildMetadata, Error> from_str(std::string_view text);
    static BuildMetadata default_();
    BuildMetadata clone() const;
    void assert_receiver_is_total_eq() const;
    bool operator==(const BuildMetadata& other) const;
    template<typename __H>
    void hash(__H& state) const;
    static const BuildMetadata EMPTY;
    static rusty::Result<BuildMetadata, Error> new_(std::string_view text);
    std::string_view as_str() const;
    bool is_empty() const;
};
inline const BuildMetadata BuildMetadata::EMPTY = BuildMetadata(Identifier::empty());

/// **SemVer version** as defined by <https://semver.org>.
///
/// # Syntax
///
/// - The major, minor, and patch numbers may be any integer 0 through u64::MAX.
///   When representing a SemVer version as a string, each number is written as
///   a base 10 integer. For example, `1.0.119`.
///
/// - Leading zeros are forbidden in those positions. For example `1.01.00` is
///   invalid as a SemVer version.
///
/// - The pre-release identifier, if present, must conform to the syntax
///   documented for [`Prerelease`].
///
/// - The build metadata, if present, must conform to the syntax documented for
///   [`BuildMetadata`].
///
/// - Whitespace is not allowed anywhere in the version.
///
/// # Total ordering
///
/// Given any two SemVer versions, one is less than, greater than, or equal to
/// the other. Versions may be compared against one another using Rust's usual
/// comparison operators.
///
/// - The major, minor, and patch number are compared numerically from left to
///   right, lexicographically ordered as a 3-tuple of integers. So for example
///   version `1.5.0` is less than version `1.19.0`, despite the fact that
///   "1.19.0" &lt; "1.5.0" as ASCIIbetically compared strings and 1.19 &lt; 1.5
///   as real numbers.
///
/// - When major, minor, and patch are equal, a pre-release version is
///   considered less than the ordinary release:&ensp;version `1.0.0-alpha.1` is
///   less than version `1.0.0`.
///
/// - Two pre-releases of the same major, minor, patch are compared by
///   lexicographic ordering of dot-separated components of the pre-release
///   string.
///
///   - Identifiers consisting of only digits are compared
///     numerically:&ensp;`1.0.0-pre.8` is less than `1.0.0-pre.12`.
///
///   - Identifiers that contain a letter or hyphen are compared in ASCII sort
///     order:&ensp;`1.0.0-pre12` is less than `1.0.0-pre8`.
///
///   - Any numeric identifier is always less than any non-numeric
///     identifier:&ensp;`1.0.0-pre.1` is less than `1.0.0-pre.x`.
///
/// Example:&ensp;`1.0.0-alpha`&ensp;&lt;&ensp;`1.0.0-alpha.1`&ensp;&lt;&ensp;`1.0.0-alpha.beta`&ensp;&lt;&ensp;`1.0.0-beta`&ensp;&lt;&ensp;`1.0.0-beta.2`&ensp;&lt;&ensp;`1.0.0-beta.11`&ensp;&lt;&ensp;`1.0.0-rc.1`&ensp;&lt;&ensp;`1.0.0`
struct Version {
    using Err = Error;
    uint64_t major;
    uint64_t minor;
    uint64_t patch;
    Prerelease pre;
    BuildMetadata build;

    rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const;
    static rusty::Result<Version, Error> from_str(std::string_view text);
    Version clone() const;
    void assert_receiver_is_total_eq() const;
    bool operator==(const Version& other) const;
    rusty::cmp::Ordering cmp(const Version& other) const;
    std::partial_ordering operator<=>(const Version& other) const;
    template<typename __H>
    void hash(__H& state) const;
    static Version new_(uint64_t major, uint64_t minor, uint64_t patch);
    static rusty::Result<Version, Error> parse(std::string_view text);
    rusty::cmp::Ordering cmp_precedence(const Version& other) const;
};

namespace eval {

    bool matches_req(const ::VersionReq& req, const ::Version& ver);
    bool matches_comparator(const ::Comparator& cmp, const ::Version& ver);
    bool matches_impl(const ::Comparator& cmp, const ::Version& ver);
    bool matches_exact(const ::Comparator& cmp, const ::Version& ver);
    bool matches_greater(const ::Comparator& cmp, const ::Version& ver);
    bool matches_less(const ::Comparator& cmp, const ::Version& ver);
    bool matches_tilde(const ::Comparator& cmp, const ::Version& ver);
    bool matches_caret(const ::Comparator& cmp, const ::Version& ver);
    bool pre_is_compatible(const ::Comparator& cmp, const ::Version& ver);

    using ::Comparator;
    using ::Op;
    using ::Version;
    using ::VersionReq;

    bool matches_req(const VersionReq& req, const Version& ver) {
        for (auto&& cmp : rusty::for_in(rusty::iter(req.comparators))) {
            if (!matches_impl(cmp, ver)) {
                return false;
            }
        }
        if (rusty::is_empty(ver.pre)) {
            return true;
        }
        for (auto&& cmp : rusty::for_in(rusty::iter(req.comparators))) {
            if (pre_is_compatible(cmp, ver)) {
                return true;
            }
        }
        return false;
    }

    bool matches_comparator(const Comparator& cmp, const Version& ver) {
        return matches_impl(cmp, ver) && ((rusty::is_empty(ver.pre) || pre_is_compatible(cmp, ver)));
    }

    bool matches_impl(const Comparator& cmp, const Version& ver) {
        return ({ auto&& _m = cmp.op; std::optional<bool> _match_value; bool _m_matched = false; if (!_m_matched && (_m == Op::Exact || _m == Op::Wildcard)) { _match_value.emplace(std::move(matches_exact(cmp, ver))); _m_matched = true; } if (!_m_matched && (_m == Op::Greater)) { _match_value.emplace(std::move(matches_greater(cmp, ver))); _m_matched = true; } if (!_m_matched && (_m == Op::GreaterEq)) { _match_value.emplace(std::move(matches_exact(cmp, ver) || matches_greater(cmp, ver))); _m_matched = true; } if (!_m_matched && (_m == Op::Less)) { _match_value.emplace(std::move(matches_less(cmp, ver))); _m_matched = true; } if (!_m_matched && (_m == Op::LessEq)) { _match_value.emplace(std::move(matches_exact(cmp, ver) || matches_less(cmp, ver))); _m_matched = true; } if (!_m_matched && (_m == Op::Tilde)) { _match_value.emplace(std::move(matches_tilde(cmp, ver))); _m_matched = true; } if (!_m_matched && (_m == Op::Caret)) { _match_value.emplace(std::move(matches_caret(cmp, ver))); _m_matched = true; } if (!_m_matched) { rusty::intrinsics::unreachable(); } std::move(_match_value).value(); });
    }

    bool matches_exact(const Comparator& cmp, const Version& ver) {
        if (ver.major != cmp.major) {
            return false;
        }
        if (cmp.minor.is_some()) {
            decltype(auto) minor = cmp.minor.unwrap();
            if (ver.minor != minor) {
                return false;
            }
        }
        if (cmp.patch.is_some()) {
            decltype(auto) patch = cmp.patch.unwrap();
            if (ver.patch != patch) {
                return false;
            }
        }
        return ver.pre == cmp.pre;
    }

    bool matches_greater(const Comparator& cmp, const Version& ver) {
        if (ver.major != cmp.major) {
            return ver.major > cmp.major;
        }
        {
            auto&& _m = cmp.minor;
            bool _m_matched = false;
            if (!_m_matched) {
                if (_m.is_none()) {
                    return false;
                    _m_matched = true;
                }
            }
            if (!_m_matched) {
                if (_m.is_some()) {
                    auto&& _mv1 = _m.unwrap();
                    auto&& minor = rusty::detail::deref_if_pointer(_mv1);
                    if (ver.minor != minor) {
                        return ver.minor > minor;
                    }
                    _m_matched = true;
                }
            }
        }
        {
            auto&& _m = cmp.patch;
            bool _m_matched = false;
            if (!_m_matched) {
                if (_m.is_none()) {
                    return false;
                    _m_matched = true;
                }
            }
            if (!_m_matched) {
                if (_m.is_some()) {
                    auto&& _mv1 = _m.unwrap();
                    auto&& patch = rusty::detail::deref_if_pointer(_mv1);
                    if (ver.patch != patch) {
                        return ver.patch > patch;
                    }
                    _m_matched = true;
                }
            }
        }
        return ver.pre > cmp.pre;
    }

    bool matches_less(const Comparator& cmp, const Version& ver) {
        if (ver.major != cmp.major) {
            return ver.major < cmp.major;
        }
        {
            auto&& _m = cmp.minor;
            bool _m_matched = false;
            if (!_m_matched) {
                if (_m.is_none()) {
                    return false;
                    _m_matched = true;
                }
            }
            if (!_m_matched) {
                if (_m.is_some()) {
                    auto&& _mv1 = _m.unwrap();
                    auto&& minor = rusty::detail::deref_if_pointer(_mv1);
                    if (ver.minor != minor) {
                        return ver.minor < minor;
                    }
                    _m_matched = true;
                }
            }
        }
        {
            auto&& _m = cmp.patch;
            bool _m_matched = false;
            if (!_m_matched) {
                if (_m.is_none()) {
                    return false;
                    _m_matched = true;
                }
            }
            if (!_m_matched) {
                if (_m.is_some()) {
                    auto&& _mv1 = _m.unwrap();
                    auto&& patch = rusty::detail::deref_if_pointer(_mv1);
                    if (ver.patch != patch) {
                        return ver.patch < patch;
                    }
                    _m_matched = true;
                }
            }
        }
        return ver.pre < cmp.pre;
    }

    bool matches_tilde(const Comparator& cmp, const Version& ver) {
        if (ver.major != cmp.major) {
            return false;
        }
        if (cmp.minor.is_some()) {
            decltype(auto) minor = cmp.minor.unwrap();
            if (ver.minor != minor) {
                return false;
            }
        }
        if (cmp.patch.is_some()) {
            decltype(auto) patch = cmp.patch.unwrap();
            if (ver.patch != patch) {
                return ver.patch > patch;
            }
        }
        return ver.pre >= cmp.pre;
    }

    bool matches_caret(const Comparator& cmp, const Version& ver) {
        if (ver.major != cmp.major) {
            return false;
        }
        const auto minor = ({ auto&& _m = cmp.minor; std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& minor = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(minor)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return true; } std::move(_match_value).value(); });
        const auto patch = ({ auto&& _m = cmp.patch; std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& patch = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(patch)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } if (cmp.major > 0) { return ver.minor >= minor; } else { return ver.minor == minor; } } std::move(_match_value).value(); });
        if (cmp.major > 0) {
            if (ver.minor != minor) {
                return ver.minor > minor;
            } else if (ver.patch != patch) {
                return ver.patch > patch;
            }
        } else if (minor > 0) {
            if (ver.minor != minor) {
                return false;
            } else if (ver.patch != patch) {
                return ver.patch > patch;
            }
        } else if ((ver.minor != minor) || (ver.patch != patch)) {
            return false;
        }
        return ver.pre >= cmp.pre;
    }

    bool pre_is_compatible(const Comparator& cmp, const Version& ver) {
        return (((cmp.major == ver.major) && (cmp.minor == rusty::Option<uint64_t>(ver.minor))) && (cmp.patch == rusty::Option<uint64_t>(ver.patch))) && !rusty::is_empty(cmp.pre);
    }

}

// Rust-only libtest main omitted

namespace identifier {

    rusty::ptr::NonNull<uint8_t> ptr_to_repr(uint8_t* original) {
        auto modified = std::rotr(static_cast<size_t>(((static_cast<size_t>(reinterpret_cast<std::uintptr_t>(original))) | 1)), 1);
        const auto diff = (static_cast<size_t>(modified) - static_cast<size_t>(static_cast<size_t>(reinterpret_cast<std::uintptr_t>(original))));
        auto modified_shadow1 = rusty::ptr::add(original, std::move(diff));
        // @unsafe
        {
            return rusty::ptr::NonNull<uint8_t>::new_unchecked(std::move(modified_shadow1));
        }
    }

    const uint8_t* repr_to_ptr(rusty::ptr::NonNull<uint8_t> modified) {
        const auto modified_shadow1 = const_cast<uint8_t*>(reinterpret_cast<const uint8_t*>(rusty::as_ptr(modified)));
        const auto original = ((static_cast<size_t>(reinterpret_cast<std::uintptr_t>(modified_shadow1)))) << 1;
        const auto diff = (static_cast<size_t>(original) - static_cast<size_t>(static_cast<size_t>(reinterpret_cast<std::uintptr_t>(modified_shadow1))));
        return rusty::ptr::add(modified_shadow1, std::move(diff));
    }

    uint8_t* repr_to_ptr_mut(rusty::ptr::NonNull<uint8_t> repr) {
        return const_cast<uint8_t*>(reinterpret_cast<const uint8_t*>(repr_to_ptr(std::move(repr))));
    }

    // @unsafe
    rusty::num::NonZeroUsize inline_len(const Identifier& repr) {
        const auto repr_shadow1 = rusty::ptr::read(reinterpret_cast<const rusty::num::NonZeroU64*>(static_cast<const Identifier*>(&repr)));
        const auto zero_bits_on_string_end = repr_shadow1.leading_zeros();
        auto nonzero_bytes = 8 - ((static_cast<size_t>(zero_bits_on_string_end)) / 8);
        // @unsafe
        {
            return NonZeroUsize::new_unchecked(std::move(nonzero_bytes));
        }
    }

    // @unsafe
    std::string_view inline_as_str(const Identifier& repr) {
        const auto ptr_shadow1 = reinterpret_cast<const uint8_t*>(static_cast<const Identifier*>(&repr));
        const auto len = inline_len(repr).get();
        const auto slice = rusty::from_raw_parts(std::move(ptr_shadow1), std::move(len));
        // @unsafe
        {
            return rusty::str_runtime::from_utf8_unchecked(std::move(slice));
        }
    }

    // @unsafe
    rusty::num::NonZeroUsize decode_len(const uint8_t* ptr) {
        auto [first, second] = rusty::ptr::read(reinterpret_cast<const std::array<uint8_t, 2>*>(ptr));
        if (second < 128) {
            // @unsafe
            {
                return NonZeroUsize::new_unchecked(static_cast<size_t>((first & 127)));
            }
        } else {
            const rusty::SafeFn<rusty::num::NonZeroUsize(const uint8_t*)> decode_len_cold = +[](const uint8_t* ptr) -> rusty::num::NonZeroUsize {
                auto len = 0;
                auto shift = 0;
                while (true) {
                    const auto byte = *ptr;
                    if (byte < 128) {
                        return NonZeroUsize::new_unchecked(std::move(len));
                    }
                    ptr = rusty::ptr::add(ptr, 1);
                    [&]() { static_cast<void>(len += ((static_cast<size_t>((byte & 127)))) << shift); return std::make_tuple(); }();
                    [&]() { static_cast<void>(shift += 7); return std::make_tuple(); }();
                }
            };
            return decode_len_cold(std::move(ptr));
        }
    }

    // @unsafe
    std::string_view ptr_as_str(const rusty::ptr::NonNull<uint8_t>& repr) {
        const auto ptr_shadow1 = repr_to_ptr(repr);
        auto len = decode_len(std::move(ptr_shadow1));
        const auto header = bytes_for_varint(std::move(len));
        const auto slice = rusty::from_raw_parts(rusty::ptr::add(ptr_shadow1, std::move(header)), len.get());
        // @unsafe
        {
            return rusty::str_runtime::from_utf8_unchecked(std::move(slice));
        }
    }

    size_t bytes_for_varint(rusty::num::NonZeroUsize len) {
        const auto usize_bits = rusty::mem::size_of<size_t>() * 8;
        const auto len_bits = usize_bits - (static_cast<size_t>(len.leading_zeros()));
        return ((len_bits + 6)) / 7;
    }

}

namespace parse {

    rusty::Result<std::tuple<uint64_t, std::string_view>, Error> numeric_identifier(std::string_view input, error::Position pos) {
        auto len = 0;
        auto value = static_cast<uint64_t>(0);
        while (true) {
            auto&& _whilelet = rusty::get(rusty::as_bytes(input), std::move(len));
            if (!(_whilelet.is_some())) { break; }
            auto&& _whilelet_payload = _whilelet.unwrap();
            auto&& digit = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_whilelet_payload));
            if ((digit < static_cast<uint8_t>(48)) || (digit > static_cast<uint8_t>(57))) {
                break;
            }
            if ((value == 0) && (len > 0)) {
                return rusty::Result<std::tuple<uint64_t, std::string_view>, Error>::Err(Error::new_(error::ErrorKind{error::ErrorKind_LeadingZero{std::move(pos)}}));
            }
            {
                auto&& _m = [&]() { auto&& _checked_lhs = value; return rusty::checked_mul(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(10)); }().and_then([&](auto&& value) { return [&]() { auto&& _checked_lhs = value; return rusty::checked_add(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(static_cast<uint64_t>((digit - static_cast<uint8_t>(48))))); }(); });
                bool _m_matched = false;
                if (!_m_matched) {
                    if (_m.is_some()) {
                        auto&& _mv0 = _m.unwrap();
                        auto&& sum = rusty::detail::deref_if_pointer(_mv0);
                        value = sum;
                        _m_matched = true;
                    }
                }
                if (!_m_matched) {
                    if (_m.is_none()) {
                        return rusty::Result<std::tuple<uint64_t, std::string_view>, Error>::Err(Error::new_(error::ErrorKind{error::ErrorKind_Overflow{std::move(pos)}}));
                        _m_matched = true;
                    }
                }
            }
            [&]() { static_cast<void>(len += 1); return std::make_tuple(); }();
        }
        if (len > 0) {
            return rusty::Result<std::tuple<uint64_t, std::string_view>, Error>::Ok(std::tuple<uint64_t, std::string_view>{std::move(value), rusty::slice_from(std::string_view(input), len)});
        } else if (auto&& _iflet_scrutinee = rusty::str_runtime::chars(rusty::slice_from(input, len)).next(); _iflet_scrutinee.is_some()) {
            decltype(auto) unexpected = _iflet_scrutinee.unwrap();
            return rusty::Result<std::tuple<uint64_t, std::string_view>, Error>::Err(Error::new_(error::ErrorKind{error::ErrorKind_UnexpectedChar{std::move(pos), std::move(unexpected)}}));
        } else {
            return rusty::Result<std::tuple<uint64_t, std::string_view>, Error>::Err(Error::new_(error::ErrorKind{error::ErrorKind_UnexpectedEnd{std::move(pos)}}));
        }
    }

    rusty::Option<std::tuple<char32_t, std::string_view>> wildcard(std::string_view input) {
        if (auto&& _iflet_scrutinee = rusty::str_runtime::strip_prefix(input, U'*'); _iflet_scrutinee.is_some()) {
            decltype(auto) rest = _iflet_scrutinee.unwrap();
            return rusty::Option<std::tuple<char32_t, std::string_view>>(std::tuple<char32_t, std::string_view>{U'*', rusty::to_string_view(rest)});
        } else if (auto&& _iflet_scrutinee = rusty::str_runtime::strip_prefix(input, U'x'); _iflet_scrutinee.is_some()) {
            decltype(auto) rest = _iflet_scrutinee.unwrap();
            return rusty::Option<std::tuple<char32_t, std::string_view>>(std::tuple<char32_t, std::string_view>{U'x', rusty::to_string_view(rest)});
        } else if (auto&& _iflet_scrutinee = rusty::str_runtime::strip_prefix(input, U'X'); _iflet_scrutinee.is_some()) {
            decltype(auto) rest = _iflet_scrutinee.unwrap();
            return rusty::Option<std::tuple<char32_t, std::string_view>>(std::tuple<char32_t, std::string_view>{U'X', rusty::to_string_view(rest)});
        } else {
            return rusty::Option<std::tuple<char32_t, std::string_view>>(rusty::None);
        }
    }

    rusty::Result<std::string_view, Error> dot(std::string_view input, error::Position pos) {
        if (auto&& _iflet_scrutinee = rusty::str_runtime::strip_prefix(input, U'.'); _iflet_scrutinee.is_some()) {
            decltype(auto) rest = _iflet_scrutinee.unwrap();
            return rusty::Result<std::string_view, Error>::Ok(rusty::to_string_view(rest));
        } else if (auto&& _iflet_scrutinee = rusty::str_runtime::chars(input).next(); _iflet_scrutinee.is_some()) {
            decltype(auto) unexpected = _iflet_scrutinee.unwrap();
            return rusty::Result<std::string_view, Error>::Err(Error::new_(error::ErrorKind{error::ErrorKind_UnexpectedCharAfter{std::move(pos), std::move(unexpected)}}));
        } else {
            return rusty::Result<std::string_view, Error>::Err(Error::new_(error::ErrorKind{error::ErrorKind_UnexpectedEnd{std::move(pos)}}));
        }
    }

    rusty::Result<std::tuple<Prerelease, std::string_view>, Error> prerelease_identifier(std::string_view input) {
        auto _tuple_destructure = rusty::detail::deref_if_pointer_like(RUSTY_TRY_INTO(identifier(std::string_view(input), error::Position_Pre()), rusty::Result<std::tuple<Prerelease, std::string_view>, Error>));
        auto&& string = std::get<0>(rusty::detail::deref_if_pointer(_tuple_destructure));
        auto&& rest = std::get<1>(rusty::detail::deref_if_pointer(_tuple_destructure));
        auto identifier_shadow1 = Identifier::new_unchecked(std::string_view(string));
        return rusty::Result<std::tuple<Prerelease, std::string_view>, Error>::Ok(std::tuple<Prerelease, std::string_view>{Prerelease(std::move(identifier_shadow1)), std::string_view(rest)});
    }

    rusty::Result<std::tuple<BuildMetadata, std::string_view>, Error> build_identifier(std::string_view input) {
        auto _tuple_destructure = rusty::detail::deref_if_pointer_like(RUSTY_TRY_INTO(identifier(std::string_view(input), error::Position_Build()), rusty::Result<std::tuple<BuildMetadata, std::string_view>, Error>));
        auto&& string = std::get<0>(rusty::detail::deref_if_pointer(_tuple_destructure));
        auto&& rest = std::get<1>(rusty::detail::deref_if_pointer(_tuple_destructure));
        auto identifier_shadow1 = Identifier::new_unchecked(std::string_view(string));
        return rusty::Result<std::tuple<BuildMetadata, std::string_view>, Error>::Ok(std::tuple<BuildMetadata, std::string_view>{BuildMetadata(std::move(identifier_shadow1)), std::string_view(rest)});
    }

    rusty::Result<std::tuple<std::string_view, std::string_view>, Error> identifier(std::string_view input, error::Position pos) {
        auto accumulated_len = 0;
        int32_t segment_len = static_cast<int32_t>(0);
        auto segment_has_nondigit = false;
        while (true) {
            {
                auto&& _m = rusty::get(rusty::as_bytes(input), accumulated_len + segment_len);
                bool _m_matched = false;
                if (!_m_matched) {
                    bool _m_or_match0 = false;
                    if (_m.is_some()) { auto&& _m_orv0 = std::as_const(_m).unwrap(); _m_or_match0 = ((_m_orv0 >= static_cast<uint8_t>(65) && _m_orv0 <= static_cast<uint8_t>(90)) || (_m_orv0 >= static_cast<uint8_t>(97) && _m_orv0 <= static_cast<uint8_t>(122)) || _m_orv0 == static_cast<uint8_t>(45)); }
                    if (_m_or_match0) {
                        [&]() { static_cast<void>(segment_len += 1); return std::make_tuple(); }();
                        segment_has_nondigit = true;
                        _m_matched = true;
                    }
                }
                if (!_m_matched) {
                    if (_m.is_some()) {
                        auto&& _mv1 = std::as_const(_m).unwrap();
                        if ((_mv1 >= static_cast<uint8_t>(48) && _mv1 <= static_cast<uint8_t>(57))) {
                            [&]() { static_cast<void>(segment_len += 1); return std::make_tuple(); }();
                            _m_matched = true;
                        }
                    }
                }
                if (!_m_matched) {
                    if (true) {
                        const auto& boundary = _m;
                        if (segment_len == static_cast<int32_t>(0)) {
                            if ((accumulated_len == 0) && (boundary != rusty::SomeRef([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<uint8_t>(46); return _some_ref_tmp; }()))) {
                                return rusty::Result<std::tuple<std::string_view, std::string_view>, Error>::Ok(std::tuple<std::string_view, std::string_view>{std::string_view(""), std::string_view(input)});
                            } else {
                                return rusty::Result<std::tuple<std::string_view, std::string_view>, Error>::Err(Error::new_(error::ErrorKind{error::ErrorKind_EmptySegment{std::move(pos)}}));
                            }
                        }
                        if ((((pos == error::Position_Pre()) && (segment_len > 1)) && !segment_has_nondigit) && rusty::starts_with(rusty::slice_from(input, accumulated_len), U'0')) {
                            return rusty::Result<std::tuple<std::string_view, std::string_view>, Error>::Err(Error::new_(error::ErrorKind{error::ErrorKind_LeadingZero{std::move(pos)}}));
                        }
                        [&]() { static_cast<void>(accumulated_len += segment_len); return std::make_tuple(); }();
                        if (boundary == rusty::SomeRef([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<uint8_t>(46); return _some_ref_tmp; }())) {
                            [&]() { static_cast<void>(accumulated_len += 1); return std::make_tuple(); }();
                            segment_len = static_cast<int32_t>(0);
                            segment_has_nondigit = false;
                        } else {
                            return rusty::Result<std::tuple<std::string_view, std::string_view>, Error>::Ok(rusty::split_at(input, std::move(accumulated_len)));
                        }
                        _m_matched = true;
                    }
                }
            }
        }
    }

    std::tuple<Op, std::string_view> op(std::string_view input) {
        const auto bytes = rusty::as_bytes(input);
        if (rusty::first(bytes) == rusty::SomeRef([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<uint8_t>(61); return _some_ref_tmp; }())) {
            return std::tuple<Op, std::string_view>{Op_Exact(), rusty::slice_from(std::string_view(input), 1)};
        } else if (rusty::first(bytes) == rusty::SomeRef([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<uint8_t>(62); return _some_ref_tmp; }())) {
            if (rusty::get(bytes, 1) == rusty::SomeRef([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<uint8_t>(61); return _some_ref_tmp; }())) {
                return std::tuple<Op, std::string_view>{Op_GreaterEq(), rusty::slice_from(std::string_view(input), 2)};
            } else {
                return std::tuple<Op, std::string_view>{Op_Greater(), rusty::slice_from(std::string_view(input), 1)};
            }
        } else if (rusty::first(bytes) == rusty::SomeRef([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<uint8_t>(60); return _some_ref_tmp; }())) {
            if (rusty::get(bytes, 1) == rusty::SomeRef([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<uint8_t>(61); return _some_ref_tmp; }())) {
                return std::tuple<Op, std::string_view>{Op_LessEq(), rusty::slice_from(std::string_view(input), 2)};
            } else {
                return std::tuple<Op, std::string_view>{Op_Less(), rusty::slice_from(std::string_view(input), 1)};
            }
        } else if (rusty::first(bytes) == rusty::SomeRef([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<uint8_t>(126); return _some_ref_tmp; }())) {
            return std::tuple<Op, std::string_view>{Op_Tilde(), rusty::slice_from(std::string_view(input), 1)};
        } else if (rusty::first(bytes) == rusty::SomeRef([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<uint8_t>(94); return _some_ref_tmp; }())) {
            return std::tuple<Op, std::string_view>{Op_Caret(), rusty::slice_from(std::string_view(input), 1)};
        } else {
            return std::tuple<Op, std::string_view>{rusty::clone(rusty::clone(Op_DEFAULT)), std::string_view(input)};
        }
    }

    rusty::Result<std::tuple<Comparator, error::Position, std::string_view>, Error> comparator(std::string_view input) {
        auto _tuple_destructure = rusty::detail::deref_if_pointer_like(op(std::string_view(input)));
        auto op_shadow1 = std::get<0>(rusty::detail::deref_if_pointer(_tuple_destructure));
        auto&& text = std::get<1>(rusty::detail::deref_if_pointer(_tuple_destructure));
        const auto default_op = rusty::len(input) == rusty::len(text);
        auto text_shadow1 = rusty::str_runtime::trim_start_matches(text, U' ');
        auto pos = error::Position_Major();
        auto _tuple_destructure_1 = rusty::detail::deref_if_pointer_like(RUSTY_TRY_INTO(numeric_identifier(std::move(rusty::to_string_view(text_shadow1)), std::move(pos)), rusty::Result<std::tuple<Comparator, error::Position, std::string_view>, Error>));
        auto major = std::get<0>(rusty::detail::deref_if_pointer(_tuple_destructure_1));
        auto&& text_shadow2 = std::get<1>(rusty::detail::deref_if_pointer(_tuple_destructure_1));
        auto has_wildcard = false;
        std::tuple<rusty::Option<uint64_t>, std::string_view> _iflet_result2 = std::make_tuple(rusty::Option<uint64_t>(rusty::None), std::move(rusty::to_string_view(text_shadow2)));
        {
            auto&& _iflet_scrutinee = rusty::str_runtime::strip_prefix(text_shadow2, U'.');
            if (_iflet_scrutinee.is_some()) {
                auto text = _iflet_scrutinee.unwrap();
                pos = error::Position_Minor();
                { auto&& _iflet_s = wildcard(std::move(rusty::to_string_view(text)));
                if (_iflet_s.is_some()) {
                    auto&& _iflet_payload = _iflet_s.unwrap();
                    auto&& text = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_iflet_payload)));
                    has_wildcard = true;
                    if (default_op) {
                        op_shadow1 = Op_Wildcard();
                    }
                    _iflet_result2 = std::tuple<rusty::Option<uint64_t>, std::string_view>{rusty::Option<uint64_t>(rusty::None), rusty::to_string_view(text)};
                } else {
                    auto _tuple_destructure = rusty::detail::deref_if_pointer_like(RUSTY_TRY_INTO(numeric_identifier(std::move(rusty::to_string_view(text)), std::move(pos)), rusty::Result<std::tuple<Comparator, error::Position, std::string_view>, Error>));
                    auto minor = std::get<0>(rusty::detail::deref_if_pointer(_tuple_destructure));
                    auto&& text_shadow1 = std::get<1>(rusty::detail::deref_if_pointer(_tuple_destructure));
                    _iflet_result2 = std::tuple<rusty::Option<uint64_t>, std::string_view>{rusty::Option<uint64_t>(std::move(minor)), std::string_view(text_shadow1)};
                }}
            }
        }
        auto [minor, text_shadow3] = std::move(_iflet_result2);
        std::tuple<rusty::Option<uint64_t>, std::string_view> _iflet_result3 = std::make_tuple(rusty::Option<uint64_t>(rusty::None), std::move(rusty::to_string_view(text_shadow3)));
        {
            auto&& _iflet_scrutinee = rusty::str_runtime::strip_prefix(text_shadow3, U'.');
            if (_iflet_scrutinee.is_some()) {
                auto text = _iflet_scrutinee.unwrap();
                pos = error::Position_Patch();
                { auto&& _iflet_s = wildcard(std::move(rusty::to_string_view(text)));
                if (_iflet_s.is_some()) {
                    auto&& _iflet_payload = _iflet_s.unwrap();
                    auto&& text = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_iflet_payload)));
                    if (default_op) {
                        op_shadow1 = Op_Wildcard();
                    }
                    _iflet_result3 = std::tuple<rusty::Option<uint64_t>, std::string_view>{rusty::Option<uint64_t>(rusty::None), rusty::to_string_view(text)};
                } else {
                    { if (has_wildcard) {
                        return rusty::Result<std::tuple<Comparator, error::Position, std::string_view>, Error>::Err(Error::new_(error::ErrorKind{error::ErrorKind_UnexpectedAfterWildcard{}}));
                    } else {
                        auto _tuple_destructure = rusty::detail::deref_if_pointer_like(RUSTY_TRY_INTO(numeric_identifier(std::move(rusty::to_string_view(text)), std::move(pos)), rusty::Result<std::tuple<Comparator, error::Position, std::string_view>, Error>));
                        auto patch = std::get<0>(rusty::detail::deref_if_pointer(_tuple_destructure));
                        auto&& text_shadow1 = std::get<1>(rusty::detail::deref_if_pointer(_tuple_destructure));
                        _iflet_result3 = std::tuple<rusty::Option<uint64_t>, std::string_view>{rusty::Option<uint64_t>(std::move(patch)), std::string_view(text_shadow1)};
                    }}
                }}
            }
        }
        auto [patch, text_shadow4] = std::move(_iflet_result3);
        auto _iflet_result4 = std::make_tuple(rusty::clone(rusty::clone(Prerelease::EMPTY)), std::move(text_shadow4));
        {
            if (patch.is_some() && rusty::starts_with(text_shadow4, U'-')) {
                pos = error::Position_Pre();
                auto text = rusty::slice_from(text_shadow4, 1);
                auto _tuple_destructure = rusty::detail::deref_if_pointer_like(RUSTY_TRY_INTO(prerelease_identifier(std::move(rusty::to_string_view(text))), rusty::Result<std::tuple<Comparator, error::Position, std::string_view>, Error>));
                auto pre = std::get<0>(rusty::detail::deref_if_pointer(_tuple_destructure));
                auto&& text_shadow1 = std::get<1>(rusty::detail::deref_if_pointer(_tuple_destructure));
                if (rusty::is_empty(pre)) {
                    return rusty::Result<std::tuple<Comparator, error::Position, std::string_view>, Error>::Err(Error::new_(error::ErrorKind{error::ErrorKind_EmptySegment{std::move(pos)}}));
                }
                _iflet_result4 = std::make_tuple(std::move(pre), text_shadow1);
            }
        }
        auto [pre, text_shadow5] = std::move(_iflet_result4);
        auto text_shadow6 = text_shadow5;
        {
            if (patch.is_some() && rusty::starts_with(text_shadow5, U'+')) {
                pos = error::Position_Build();
                auto text = rusty::slice_from(text_shadow5, 1);
                auto _tuple_destructure = rusty::detail::deref_if_pointer_like(RUSTY_TRY_INTO(build_identifier(std::move(rusty::to_string_view(text))), rusty::Result<std::tuple<Comparator, error::Position, std::string_view>, Error>));
                auto build = std::get<0>(rusty::detail::deref_if_pointer(_tuple_destructure));
                auto&& text_shadow1 = std::get<1>(rusty::detail::deref_if_pointer(_tuple_destructure));
                if (rusty::is_empty(build)) {
                    return rusty::Result<std::tuple<Comparator, error::Position, std::string_view>, Error>::Err(Error::new_(error::ErrorKind{error::ErrorKind_EmptySegment{std::move(pos)}}));
                }
                text_shadow6 = text_shadow1;
            }
        }
        auto text_shadow7 = rusty::str_runtime::trim_start_matches(text_shadow6, U' ');
        auto comparator_shadow1 = Comparator(std::move(op_shadow1), std::move(major), std::move(minor), std::move(patch), std::move(pre));
        return rusty::Result<std::tuple<Comparator, error::Position, std::string_view>, Error>::Ok(std::tuple<Comparator, error::Position, std::string_view>{std::move(comparator_shadow1), std::move(pos), rusty::to_string_view(text_shadow7)});
    }

    rusty::Result<size_t, Error> version_req(std::string_view input, rusty::Vec<Comparator>& out, size_t depth) {
        constexpr size_t MAX_COMPARATORS = static_cast<size_t>(32);
        auto _tuple_destructure = rusty::detail::deref_if_pointer_like(({ auto&& _m = comparator(std::string_view(input)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& success = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(success)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto error_shadow1 = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
if (auto&& _iflet_scrutinee = wildcard(std::string_view(input)); _iflet_scrutinee.is_some()) {
    auto&& _iflet_payload = _iflet_scrutinee.unwrap();
    auto&& ch = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_iflet_payload)));
    auto rest = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_iflet_payload)));
    rest = rusty::str_runtime::trim_start_matches(rest, U' ');
    if (rusty::is_empty(rest) || rusty::starts_with(rest, U',')) {
        error_shadow1.kind = error::ErrorKind_WildcardNotTheOnlyComparator{std::move(ch)};
    }
} return rusty::Result<size_t, Error>::Err(error_shadow1); } std::move(_match_value).value(); }));
        auto comparator_shadow1 = std::get<0>(rusty::detail::deref_if_pointer(_tuple_destructure));
        auto pos = std::get<1>(rusty::detail::deref_if_pointer(_tuple_destructure));
        auto&& text = std::get<2>(rusty::detail::deref_if_pointer(_tuple_destructure));
        if (rusty::is_empty(text)) {
            out.reserve_exact(depth + 1);
            // @unsafe
            {
                rusty::ptr::write(rusty::ptr::add(rusty::as_mut_ptr(out), std::move(depth)), std::move(comparator_shadow1));
            }
            return rusty::Result<size_t, Error>::Ok(depth + 1);
        }
        decltype(rusty::str_runtime::trim_start_matches(text, U' ')) text_shadow1 {};
        {
            auto&& _iflet_scrutinee = rusty::str_runtime::strip_prefix(text, U',');
            if (_iflet_scrutinee.is_some()) {
                auto text_shadow2 = _iflet_scrutinee.unwrap();
                text_shadow1 = rusty::str_runtime::trim_start_matches(text_shadow2, U' ');
            } else {
                auto unexpected = rusty::str_runtime::chars(text).next().unwrap();
                return rusty::Result<size_t, Error>::Err(Error::new_(error::ErrorKind{error::ErrorKind_ExpectedCommaFound{std::move(pos), std::move(unexpected)}}));
            }
        }
        if ((depth + 1) == MAX_COMPARATORS) {
            return rusty::Result<size_t, Error>::Err(Error::new_(error::ErrorKind{error::ErrorKind_ExcessiveComparators{}}));
        }
        auto len = RUSTY_TRY_INTO(version_req(std::move(rusty::to_string_view(text_shadow1)), out, depth + 1), rusty::Result<size_t, Error>);
        // @unsafe
        {
            rusty::ptr::write(rusty::ptr::add(rusty::as_mut_ptr(out), std::move(depth)), std::move(comparator_shadow1));
        }
        return rusty::Result<size_t, Error>::Ok(std::move(len));
    }

}


namespace error {
    rusty::fmt::Result QuotedChar::fmt(rusty::fmt::Formatter& formatter) const {
        if (this->_0 == U'\0') {
            return formatter.write_str("'\\0'");
        } else {
            return rusty::write_fmt(formatter, std::format("{0}", rusty::to_debug_string(this->_0)));
        }
    }
}

namespace identifier {
    Identifier Identifier::empty() {
        using namespace impls;
        const rusty::ptr::NonNull<uint8_t> HEAD = rusty::ptr::NonNull<uint8_t>::new_unchecked(reinterpret_cast<uint8_t*>(static_cast<std::uintptr_t>(~static_cast<int32_t>(0))));
        return Identifier(rusty::clone(HEAD), [](auto _seed) { std::array<uint8_t, rusty::sanitize_array_capacity<TAIL_BYTES>()> _repeat{}; _repeat.fill(static_cast<uint8_t>(_seed)); return _repeat; }(~static_cast<int32_t>(0)));
    }
}

namespace identifier {
    Identifier Identifier::new_unchecked(std::string_view string) {
        using namespace impls;
        auto len = rusty::len(string);
        if (true) {
            if (!(len <= (static_cast<size_t>(std::numeric_limits<ptrdiff_t>::max())))) {
                [&]() -> Identifier { rusty::panicking::panic("assertion failed: len <= isize::MAX as usize"); }();
            }
        }
        return ({ auto&& _m = static_cast<uint64_t>(len); std::optional<Identifier> _match_value; bool _m_matched = false; if (!_m_matched && (_m == 0)) { _match_value.emplace(std::move(Identifier::empty())); _m_matched = true; } if (!_m_matched && ((_m >= 1 && _m <= 8))) { _match_value.emplace(std::move([&]() -> Identifier { auto bytes = [](auto _seed) { std::array<uint8_t, rusty::mem::size_of<Identifier>()> _repeat{}; _repeat.fill(static_cast<uint8_t>(_seed)); return _repeat; }(static_cast<uint8_t>(0));
// @unsafe
{
    rusty::ptr::copy_nonoverlapping(rusty::as_ptr(string), rusty::as_mut_ptr(bytes), std::move(len));
}
return mem::transmute<std::array<uint8_t, rusty::mem::size_of<Identifier>()>, Identifier>(std::move(bytes)); }())); _m_matched = true; } if (!_m_matched && ((_m >= 9 && _m <= 72057594037927935))) { _match_value.emplace(std::move([&]() -> Identifier { const auto size = bytes_for_varint(NonZeroUsize::new_unchecked(std::move(len))) + len;
const auto align = 2;
if (rusty::mem::size_of<size_t>() < 8) {
    const auto max_alloc = (std::numeric_limits<size_t>::max() / 2) - align;
    if (!(size <= max_alloc)) {
        [&]() -> Identifier { rusty::panicking::panic("assertion failed: size <= max_alloc"); }();
    }
}
const auto layout = Layout::from_size_align_unchecked(std::move(size), std::move(align));
const auto ptr_shadow1 = alloc(std::move(layout));
if ((ptr_shadow1 == nullptr)) {
    handle_alloc_error(std::move(layout));
}
auto write_ = std::move(ptr_shadow1);
auto varint_remaining = std::move(len);
while (varint_remaining > 0) {
    // @unsafe
    {
        rusty::ptr::write(std::move(write_), std::move((static_cast<uint8_t>(varint_remaining)) | 128));
    }
    [&]() { static_cast<void>(varint_remaining >>= 7); return std::make_tuple(); }();
    write_ = rusty::ptr::add(write_, 1);
}
// @unsafe
{
    rusty::ptr::copy_nonoverlapping(rusty::as_ptr(string), std::move(write_), std::move(len));
}
return Identifier(ptr_to_repr(std::move(ptr_shadow1)), [](auto _seed) { std::array<uint8_t, rusty::sanitize_array_capacity<TAIL_BYTES>()> _repeat{}; _repeat.fill(static_cast<uint8_t>(_seed)); return _repeat; }(static_cast<uint8_t>(0))); }())); _m_matched = true; } if (!_m_matched && ((_m >= 72057594037927936 && _m <= 18446744073709551615))) { {
    rusty::panicking::unreachable_display("please refrain from storing >64 petabytes of text in semver version");
} _m_matched = true; } if (!_m_matched) { rusty::intrinsics::unreachable(); } std::move(_match_value).value(); });
    }
}

namespace identifier {
    bool Identifier::is_empty() const {
        using namespace impls;
        const auto empty = Identifier::empty();
        auto is_empty = (this->head == empty.head) && (this->tail == empty.tail);
        rusty::mem::forget(std::move(empty));
        return is_empty;
    }
}

namespace identifier {
    bool Identifier::is_inline() const {
        using namespace impls;
        return ((static_cast<size_t>(reinterpret_cast<std::uintptr_t>(rusty::as_ptr(this->head)))) >> (((PTR_BYTES * 8) - 1))) == static_cast<size_t>(0);
    }
}

namespace identifier {
    bool Identifier::is_empty_or_inline() const {
        using namespace impls;
        return rusty::is_empty((*this)) || this->is_inline();
    }
}

namespace identifier {
    std::string_view Identifier::as_str() const {
        using namespace impls;
        if (rusty::is_empty((*this))) {
            return std::string_view("");
        } else if (this->is_inline()) {
            // @unsafe
            {
                return inline_as_str((*this));
            }
        } else {
            // @unsafe
            {
                return ptr_as_str(this->head);
            }
        }
    }
}

namespace identifier {
    bool Identifier::ptr_eq(const Identifier& rhs) const {
        using namespace impls;
        return (this->head == rhs.head) && (this->tail == rhs.tail);
    }
}

namespace identifier {
    Identifier Identifier::clone() const {
        using namespace impls;
        if (this->is_empty_or_inline()) {
            return Identifier(this->head, this->tail);
        } else {
            const auto ptr_shadow1 = repr_to_ptr(this->head);
            auto len = decode_len(std::move(ptr_shadow1));
            auto size = bytes_for_varint(std::move(len)) + len.get();
            const auto align = 2;
            const auto layout = Layout::from_size_align_unchecked(std::move(size), std::move(align));
            const auto clone = alloc(std::move(layout));
            if ((clone == nullptr)) {
                handle_alloc_error(std::move(layout));
            }
            // @unsafe
            {
                rusty::ptr::copy_nonoverlapping(std::move(ptr_shadow1), std::move(clone), std::move(size));
            }
            return Identifier(ptr_to_repr(std::move(clone)), [](auto _seed) { std::array<uint8_t, rusty::sanitize_array_capacity<TAIL_BYTES>()> _repeat{}; _repeat.fill(static_cast<uint8_t>(_seed)); return _repeat; }(static_cast<uint8_t>(0)));
        }
    }
}

namespace identifier {
    Identifier::~Identifier() noexcept(false) {
        if (rusty::mem::consume_forgotten_address(this)) { return; }
        using namespace impls;
        if (this->is_empty_or_inline()) {
            return;
        }
        auto ptr_shadow1 = repr_to_ptr_mut(this->head);
        auto len = decode_len(std::move(ptr_shadow1));
        const auto size = bytes_for_varint(std::move(len)) + len.get();
        const auto align = 2;
        const auto layout = Layout::from_size_align_unchecked(std::move(size), std::move(align));
        // @unsafe
        {
            dealloc(std::move(ptr_shadow1), std::move(layout));
        }
    }
}

namespace identifier {
    bool Identifier::operator==(const Identifier& rhs) const {
        using namespace impls;
        if (this->ptr_eq(rhs)) {
            return true;
        } else if (this->is_empty_or_inline() || rhs.is_empty_or_inline()) {
            return false;
        } else {
            // @unsafe
            {
                return ptr_as_str(this->head) == ptr_as_str(rhs.head);
            }
        }
    }
}

namespace identifier {
    Identifier Identifier::default_() {
        using namespace impls;
        return Identifier::empty();
    }
}

namespace identifier {
    template<typename H>
    void Identifier::hash(H& hasher) const {
        using namespace impls;
        rusty::hash::hash(this->as_str(), hasher);
    }
}

namespace parse {
    rusty::fmt::Result Error::fmt(rusty::fmt::Formatter& formatter) const {
        using namespace error;
        return [&]() -> rusty::fmt::Result { auto&& _m = &this->kind; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return formatter.write_str("empty string, expected a semver version"); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& pos = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::write_fmt(formatter, std::format("unexpected end of input while parsing {0}", rusty::to_string(pos))); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& pos = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& ch = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return rusty::write_fmt(formatter, std::format("unexpected character {0} while parsing {1}", rusty::to_string(::error::QuotedChar(std::move(rusty::detail::deref_if_pointer_like(ch)))), rusty::to_string(pos))); } if (std::holds_alternative<std::variant_alternative_t<3, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& pos = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<3, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& ch = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<3, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return rusty::write_fmt(formatter, std::format("unexpected character {0} after {1}", rusty::to_string(::error::QuotedChar(std::move(rusty::detail::deref_if_pointer_like(ch)))), rusty::to_string(pos))); } if (std::holds_alternative<std::variant_alternative_t<4, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& pos = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<4, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& ch = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<4, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return rusty::write_fmt(formatter, std::format("expected comma after {0}, found {1}", rusty::to_string(pos), rusty::to_string(::error::QuotedChar(std::move(rusty::detail::deref_if_pointer_like(ch)))))); } if (std::holds_alternative<std::variant_alternative_t<5, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& pos = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<5, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::write_fmt(formatter, std::format("invalid leading zero in {0}", rusty::to_string(pos))); } if (std::holds_alternative<std::variant_alternative_t<6, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& pos = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<6, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::write_fmt(formatter, std::format("value of {0} exceeds u64::MAX", rusty::to_string(pos))); } if (std::holds_alternative<std::variant_alternative_t<7, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& pos = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<7, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::write_fmt(formatter, std::format("empty identifier segment in {0}", rusty::to_string(pos))); } if (std::holds_alternative<std::variant_alternative_t<8, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& pos = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<8, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::write_fmt(formatter, std::format("unexpected character in {0}", rusty::to_string(pos))); } if (std::holds_alternative<std::variant_alternative_t<9, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& ch = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<9, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::write_fmt(formatter, std::format("wildcard req ({0}) must be the only comparator in the version req", rusty::to_string(ch))); } if (std::holds_alternative<std::variant_alternative_t<10, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return formatter.write_str("unexpected character after wildcard in version req"); } if (std::holds_alternative<std::variant_alternative_t<11, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return formatter.write_str("excessive number of version comparators"); } return [&]() -> rusty::fmt::Result { rusty::intrinsics::unreachable(); }(); }();
    }
}

namespace parse {
    Error Error::new_(error::ErrorKind kind) {
        using namespace error;
        return Error{.kind = std::move(kind)};
    }
}

rusty::fmt::Result Prerelease::fmt(rusty::fmt::Formatter& formatter) const {
    using Err = typename Prerelease::Err;
    using Target = typename Prerelease::Target;
    using namespace display;
    using namespace parse;
    using namespace impls;
    return formatter.write_str(this->as_str());
}

std::string_view Prerelease::operator*() const {
    using Err = typename Prerelease::Err;
    using Target = typename Prerelease::Target;
    using namespace display;
    using namespace parse;
    using namespace impls;
    return this->identifier.as_str();
}

std::partial_ordering Prerelease::operator<=>(const Prerelease& rhs) const {
    using Err = typename Prerelease::Err;
    using Target = typename Prerelease::Target;
    return rusty::to_partial_ordering([&]() -> rusty::Option<rusty::cmp::Ordering> {
        using namespace display;
        using namespace parse;
        using namespace impls;
        return rusty::Option<rusty::cmp::Ordering>(rusty::cmp::cmp((*this), rhs));
    }());
}

rusty::cmp::Ordering Prerelease::cmp(const Prerelease& rhs) const {
    using Err = typename Prerelease::Err;
    using Target = typename Prerelease::Target;
    using namespace display;
    using namespace parse;
    using namespace impls;
    if (this->identifier.ptr_eq(rhs.identifier)) {
        return Ordering::Equal;
    }
    switch (rusty::is_empty((*this))) {
    case true:
    {
        return Ordering::Greater;
        break;
    }
    case false:
    {
        if (rusty::is_empty(rhs)) {
            return Ordering::Less;
        }
        break;
    }
    }
    const auto lhs = rusty::str_runtime::split(this->as_str(), U'.');
    auto rhs_shadow1 = rusty::str_runtime::split(rhs.as_str(), U'.');
    auto _for_iter = lhs;
    for (auto&& lhs : rusty::for_in(_for_iter)) {
        const auto rhs_shadow2 = ({ auto&& _m = rhs_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& rhs_shadow1 = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(rhs_shadow1)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return Ordering::Greater; } std::move(_match_value).value(); });
        const auto string_cmp = [&]() { return rusty::cmp::cmp(std::move(lhs), std::move(rhs_shadow2)); };
        const auto is_ascii_digit = [&](uint8_t b) { return rusty::is_ascii_digit(b); };
        auto ordering = [&]() { auto&& _m0 = rusty::all(rusty::as_bytes(lhs), std::move(is_ascii_digit)); auto&& _m1 = rusty::all(rusty::as_bytes(rhs_shadow2), std::move(is_ascii_digit)); if (_m0 == true && _m1 == true) { return rusty::cmp::then_with(rusty::cmp::cmp(rusty::len(lhs), rusty::len(rhs_shadow2)), std::move(string_cmp)); } if (_m0 == true && _m1 == false) { return Ordering::Less; } if (_m0 == false && _m1 == true) { return Ordering::Greater; } if (_m0 == false && _m1 == false) { return string_cmp(); } rusty::intrinsics::unreachable(); }();
        if (ordering != Ordering::Equal) {
            return ordering;
        }
    }
    if (rhs_shadow1.next().is_none()) {
        return Ordering::Equal;
    } else {
        return Ordering::Less;
    }
}

rusty::Result<Prerelease, Error> Prerelease::from_str(std::string_view text) {
    using Err = typename Prerelease::Err;
    using Target = typename Prerelease::Target;
    using namespace display;
    using namespace parse;
    using namespace impls;
    auto [pre, rest] = rusty::detail::deref_if_pointer_like(RUSTY_TRY_INTO(parse::prerelease_identifier(std::string_view(text)), rusty::Result<Prerelease, Error>));
    if (!rusty::is_empty(rest)) {
        return rusty::Result<Prerelease, Error>::Err(Error::new_(ErrorKind{ErrorKind_IllegalCharacter{Position_Pre()}}));
    }
    return rusty::Result<Prerelease, Error>::Ok(std::move(pre));
}

Prerelease Prerelease::default_() {
    using Err = typename Prerelease::Err;
    using Target = typename Prerelease::Target;
    using namespace display;
    using namespace parse;
    using namespace impls;
    return Prerelease(rusty::default_value<Identifier>());
}

Prerelease Prerelease::clone() const {
    using Err = typename Prerelease::Err;
    using Target = typename Prerelease::Target;
    using namespace display;
    using namespace parse;
    using namespace impls;
    return Prerelease(rusty::clone(this->identifier));
}

void Prerelease::assert_receiver_is_total_eq() const {
    using Err = typename Prerelease::Err;
    using Target = typename Prerelease::Target;
    using namespace display;
    using namespace parse;
    using namespace impls;
}

bool Prerelease::operator==(const Prerelease& other) const {
    using Err = typename Prerelease::Err;
    using Target = typename Prerelease::Target;
    using namespace display;
    using namespace parse;
    using namespace impls;
    return this->identifier == other.identifier;
}

template<typename __H>
void Prerelease::hash(__H& state) const {
    using Err = typename Prerelease::Err;
    using Target = typename Prerelease::Target;
    using namespace display;
    using namespace parse;
    using namespace impls;
    rusty::hash::hash(this->identifier, state);
}

rusty::Result<Prerelease, Error> Prerelease::new_(std::string_view text) {
    using Err = typename Prerelease::Err;
    using Target = typename Prerelease::Target;
    using namespace display;
    using namespace parse;
    using namespace impls;
    return Prerelease::from_str(rusty::to_string_view(text));
}

std::string_view Prerelease::as_str() const {
    using Err = typename Prerelease::Err;
    using Target = typename Prerelease::Target;
    using namespace display;
    using namespace parse;
    using namespace impls;
    return this->identifier.as_str();
}

bool Prerelease::is_empty() const {
    using Err = typename Prerelease::Err;
    using Target = typename Prerelease::Target;
    using namespace display;
    using namespace parse;
    using namespace impls;
    return rusty::is_empty(this->identifier);
}

rusty::fmt::Result Comparator::fmt(rusty::fmt::Formatter& formatter) const {
    using Err = typename Comparator::Err;
    using namespace display;
    using namespace parse;
    const auto op_shadow1 = ({ auto&& _m = this->op; std::optional<std::string_view> _match_value; bool _m_matched = false; if (!_m_matched && (_m == Op::Exact)) { _match_value.emplace(std::move(std::string_view("="))); _m_matched = true; } if (!_m_matched && (_m == Op::Greater)) { _match_value.emplace(std::move(std::string_view(">"))); _m_matched = true; } if (!_m_matched && (_m == Op::GreaterEq)) { _match_value.emplace(std::move(std::string_view(">="))); _m_matched = true; } if (!_m_matched && (_m == Op::Less)) { _match_value.emplace(std::move(std::string_view("<"))); _m_matched = true; } if (!_m_matched && (_m == Op::LessEq)) { _match_value.emplace(std::move(std::string_view("<="))); _m_matched = true; } if (!_m_matched && (_m == Op::Tilde)) { _match_value.emplace(std::move(std::string_view("~"))); _m_matched = true; } if (!_m_matched && (_m == Op::Caret)) { _match_value.emplace(std::move(std::string_view("^"))); _m_matched = true; } if (!_m_matched && (_m == Op::Wildcard)) { _match_value.emplace(std::move(std::string_view(""))); _m_matched = true; } if (!_m_matched) { rusty::intrinsics::unreachable(); } std::move(_match_value).value(); });
    RUSTY_TRY(formatter.write_str(op_shadow1));
    RUSTY_TRY(rusty::write_fmt(formatter, std::format("{0}", this->major)));
    if (this->minor.is_some()) {
        decltype(auto) minor = this->minor.unwrap();
        RUSTY_TRY(rusty::write_fmt(formatter, std::format(".{0}", rusty::to_string(minor))));
        if (this->patch.is_some()) {
            decltype(auto) patch = this->patch.unwrap();
            RUSTY_TRY(rusty::write_fmt(formatter, std::format(".{0}", rusty::to_string(patch))));
            if (!rusty::is_empty(this->pre)) {
                RUSTY_TRY(rusty::write_fmt(formatter, std::format("-{0}", rusty::to_string(this->pre))));
            }
        } else if (this->op == Op_Wildcard()) {
            RUSTY_TRY(formatter.write_str(".*"));
        }
    } else if (this->op == Op_Wildcard()) {
        RUSTY_TRY(formatter.write_str(".*"));
    }
    return rusty::fmt::Result::Ok(std::make_tuple());
}

rusty::Result<Comparator, Error> Comparator::from_str(std::string_view text) {
    using Err = typename Comparator::Err;
    using namespace display;
    using namespace parse;
    auto text_shadow1 = rusty::str_runtime::trim_start_matches(text, U' ');
    auto [comparator_shadow1, pos, rest] = rusty::detail::deref_if_pointer_like(RUSTY_TRY_INTO(comparator(std::move(rusty::to_string_view(text_shadow1))), rusty::Result<Comparator, Error>));
    if (!rusty::is_empty(rest)) {
        auto unexpected = rusty::str_runtime::chars(rest).next().unwrap();
        return rusty::Result<Comparator, Error>::Err(Error::new_(ErrorKind{ErrorKind_UnexpectedCharAfter{std::move(pos), std::move(unexpected)}}));
    }
    return rusty::Result<Comparator, Error>::Ok(std::move(comparator_shadow1));
}

Comparator Comparator::clone() const {
    using Err = typename Comparator::Err;
    using namespace display;
    using namespace parse;
    return Comparator(rusty::clone(this->op), rusty::clone(this->major), rusty::clone(this->minor), rusty::clone(this->patch), rusty::clone(this->pre));
}

void Comparator::assert_receiver_is_total_eq() const {
    using Err = typename Comparator::Err;
    using namespace display;
    using namespace parse;
}

bool Comparator::operator==(const Comparator& other) const {
    using Err = typename Comparator::Err;
    using namespace display;
    using namespace parse;
    return ((((this->major == other.major) && (this->op == other.op)) && (this->minor == other.minor)) && (this->patch == other.patch)) && (this->pre == other.pre);
}

template<typename __H>
void Comparator::hash(__H& state) const {
    using Err = typename Comparator::Err;
    using namespace display;
    using namespace parse;
    rusty::hash::hash(this->op, state);
    rusty::hash::hash(this->major, state);
    rusty::hash::hash(this->minor, state);
    rusty::hash::hash(this->patch, state);
    rusty::hash::hash(this->pre, state);
}

rusty::Result<Comparator, Error> Comparator::parse(std::string_view text) {
    using Err = typename Comparator::Err;
    using namespace display;
    using namespace parse;
    return Comparator::from_str(rusty::to_string_view(text));
}

bool Comparator::matches(const Version& version) const {
    using Err = typename Comparator::Err;
    using namespace display;
    using namespace parse;
    return eval::matches_comparator((*this), version);
}

rusty::fmt::Result VersionReq::fmt(rusty::fmt::Formatter& formatter) const {
    using Err = typename VersionReq::Err;
    using namespace parse;
    using namespace display;
    using namespace impls;
    if (rusty::is_empty(this->comparators)) {
        return formatter.write_str("*");
    }
    for (auto&& _for_item : rusty::for_in(rusty::enumerate(rusty::iter(this->comparators)))) {
        auto&& i = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_for_item)));
        auto&& comparator = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_for_item)));
        if (i > 0) {
            RUSTY_TRY(formatter.write_str(", "));
        }
        RUSTY_TRY(rusty::write_fmt(formatter, std::format("{0}", rusty::to_string(comparator))));
    }
    return rusty::fmt::Result::Ok(std::make_tuple());
}

template<typename I>
VersionReq VersionReq::from_iter(I iter) {
    using Err = typename VersionReq::Err;
    using namespace parse;
    using namespace display;
    using namespace impls;
    auto comparators = rusty::collect_range(std::move(iter));
    return VersionReq(std::move(comparators));
}

rusty::Result<VersionReq, Error> VersionReq::from_str(std::string_view text) {
    using Err = typename VersionReq::Err;
    using namespace parse;
    using namespace display;
    using namespace impls;
    auto text_shadow1 = rusty::str_runtime::trim_start_matches(text, U' ');
    if (auto&& _iflet_scrutinee = parse::wildcard(std::move(rusty::to_string_view(text_shadow1))); _iflet_scrutinee.is_some()) {
        auto&& _iflet_payload = _iflet_scrutinee.unwrap();
        auto&& ch = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_iflet_payload)));
        auto&& text_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_iflet_payload)));
        const auto rest = rusty::str_runtime::trim_start_matches(text_shadow1, U' ');
        if (rusty::is_empty(rest)) {
            return rusty::Result<VersionReq, Error>::Ok(rusty::clone(rusty::clone(VersionReq::STAR)));
        } else if (rusty::starts_with(rest, U',')) {
            return rusty::Result<VersionReq, Error>::Err(Error::new_(ErrorKind{ErrorKind_WildcardNotTheOnlyComparator{std::move(ch)}}));
        } else {
            return rusty::Result<VersionReq, Error>::Err(Error::new_(ErrorKind{ErrorKind_UnexpectedAfterWildcard{}}));
        }
    }
    auto depth = 0;
    auto comparators = rusty::Vec<Comparator>::new_();
    const auto len = RUSTY_TRY_INTO(parse::version_req(std::move(rusty::to_string_view(text_shadow1)), comparators, std::move(depth)), rusty::Result<VersionReq, Error>);
    // @unsafe
    {
        comparators.set_len(std::move(len));
    }
    return rusty::Result<VersionReq, Error>::Ok(VersionReq(std::move(comparators)));
}

VersionReq VersionReq::clone() const {
    using Err = typename VersionReq::Err;
    using namespace parse;
    using namespace display;
    using namespace impls;
    return VersionReq(rusty::clone(this->comparators));
}

void VersionReq::assert_receiver_is_total_eq() const {
    using Err = typename VersionReq::Err;
    using namespace parse;
    using namespace display;
    using namespace impls;
}

bool VersionReq::operator==(const VersionReq& other) const {
    using Err = typename VersionReq::Err;
    using namespace parse;
    using namespace display;
    using namespace impls;
    return this->comparators == other.comparators;
}

template<typename __H>
void VersionReq::hash(__H& state) const {
    using Err = typename VersionReq::Err;
    using namespace parse;
    using namespace display;
    using namespace impls;
    rusty::hash::hash(this->comparators, state);
}

rusty::Result<VersionReq, Error> VersionReq::parse(std::string_view text) {
    using Err = typename VersionReq::Err;
    using namespace parse;
    using namespace display;
    using namespace impls;
    return VersionReq::from_str(rusty::to_string_view(text));
}

bool VersionReq::matches(const Version& version) const {
    using Err = typename VersionReq::Err;
    using namespace parse;
    using namespace display;
    using namespace impls;
    return eval::matches_req((*this), version);
}

VersionReq VersionReq::default_() {
    using Err = typename VersionReq::Err;
    using namespace parse;
    using namespace display;
    using namespace impls;
    return rusty::clone(VersionReq::STAR);
}

rusty::fmt::Result BuildMetadata::fmt(rusty::fmt::Formatter& formatter) const {
    using Err = typename BuildMetadata::Err;
    using Target = typename BuildMetadata::Target;
    using namespace parse;
    using namespace impls;
    using namespace display;
    return formatter.write_str(this->as_str());
}

std::string_view BuildMetadata::operator*() const {
    using Err = typename BuildMetadata::Err;
    using Target = typename BuildMetadata::Target;
    using namespace parse;
    using namespace impls;
    using namespace display;
    return this->identifier.as_str();
}

std::partial_ordering BuildMetadata::operator<=>(const BuildMetadata& rhs) const {
    using Err = typename BuildMetadata::Err;
    using Target = typename BuildMetadata::Target;
    return rusty::to_partial_ordering([&]() -> rusty::Option<rusty::cmp::Ordering> {
        using namespace parse;
        using namespace impls;
        using namespace display;
        return rusty::Option<rusty::cmp::Ordering>(rusty::cmp::cmp((*this), rhs));
    }());
}

rusty::cmp::Ordering BuildMetadata::cmp(const BuildMetadata& rhs) const {
    using Err = typename BuildMetadata::Err;
    using Target = typename BuildMetadata::Target;
    using namespace parse;
    using namespace impls;
    using namespace display;
    if (this->identifier.ptr_eq(rhs.identifier)) {
        return Ordering::Equal;
    }
    const auto lhs = rusty::str_runtime::split(this->as_str(), U'.');
    auto rhs_shadow1 = rusty::str_runtime::split(rhs.as_str(), U'.');
    auto _for_iter = lhs;
    for (auto&& lhs : rusty::for_in(_for_iter)) {
        const auto rhs_shadow2 = ({ auto&& _m = rhs_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& rhs_shadow1 = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(rhs_shadow1)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return Ordering::Greater; } std::move(_match_value).value(); });
        const auto is_ascii_digit = [&](uint8_t b) { return rusty::is_ascii_digit(b); };
        auto ordering = [&]() { auto&& _m0 = rusty::all(rusty::as_bytes(lhs), std::move(is_ascii_digit)); auto&& _m1 = rusty::all(rusty::as_bytes(rhs_shadow2), std::move(is_ascii_digit)); if (_m0 == true && _m1 == true) { return [&]() { const auto lhval = rusty::str_runtime::trim_start_matches(lhs, U'0');
const auto rhval = rusty::str_runtime::trim_start_matches(rhs_shadow2, U'0');
return rusty::cmp::then_with(rusty::cmp::then_with(rusty::cmp::cmp(rusty::len(lhval), rusty::len(rhval)), [&]() { return rusty::cmp::cmp(std::move(lhval), std::move(rhval)); }), [&]() { return rusty::cmp::cmp(rusty::len(lhs), rusty::len(rhs_shadow2)); }); }(); } if (_m0 == true && _m1 == false) { return Ordering::Less; } if (_m0 == false && _m1 == true) { return Ordering::Greater; } if (_m0 == false && _m1 == false) { return rusty::cmp::cmp(std::move(lhs), std::move(rhs_shadow2)); } rusty::intrinsics::unreachable(); }();
        if (ordering != Ordering::Equal) {
            return ordering;
        }
    }
    if (rhs_shadow1.next().is_none()) {
        return Ordering::Equal;
    } else {
        return Ordering::Less;
    }
}

rusty::Result<BuildMetadata, Error> BuildMetadata::from_str(std::string_view text) {
    using Err = typename BuildMetadata::Err;
    using Target = typename BuildMetadata::Target;
    using namespace parse;
    using namespace impls;
    using namespace display;
    auto [build, rest] = rusty::detail::deref_if_pointer_like(RUSTY_TRY_INTO(parse::build_identifier(std::string_view(text)), rusty::Result<BuildMetadata, Error>));
    if (!rusty::is_empty(rest)) {
        return rusty::Result<BuildMetadata, Error>::Err(Error::new_(ErrorKind{ErrorKind_IllegalCharacter{Position_Build()}}));
    }
    return rusty::Result<BuildMetadata, Error>::Ok(std::move(build));
}

BuildMetadata BuildMetadata::default_() {
    using Err = typename BuildMetadata::Err;
    using Target = typename BuildMetadata::Target;
    using namespace parse;
    using namespace impls;
    using namespace display;
    return BuildMetadata(rusty::default_value<Identifier>());
}

BuildMetadata BuildMetadata::clone() const {
    using Err = typename BuildMetadata::Err;
    using Target = typename BuildMetadata::Target;
    using namespace parse;
    using namespace impls;
    using namespace display;
    return BuildMetadata(rusty::clone(this->identifier));
}

void BuildMetadata::assert_receiver_is_total_eq() const {
    using Err = typename BuildMetadata::Err;
    using Target = typename BuildMetadata::Target;
    using namespace parse;
    using namespace impls;
    using namespace display;
}

bool BuildMetadata::operator==(const BuildMetadata& other) const {
    using Err = typename BuildMetadata::Err;
    using Target = typename BuildMetadata::Target;
    using namespace parse;
    using namespace impls;
    using namespace display;
    return this->identifier == other.identifier;
}

template<typename __H>
void BuildMetadata::hash(__H& state) const {
    using Err = typename BuildMetadata::Err;
    using Target = typename BuildMetadata::Target;
    using namespace parse;
    using namespace impls;
    using namespace display;
    rusty::hash::hash(this->identifier, state);
}

rusty::Result<BuildMetadata, Error> BuildMetadata::new_(std::string_view text) {
    using Err = typename BuildMetadata::Err;
    using Target = typename BuildMetadata::Target;
    using namespace parse;
    using namespace impls;
    using namespace display;
    return BuildMetadata::from_str(rusty::to_string_view(text));
}

std::string_view BuildMetadata::as_str() const {
    using Err = typename BuildMetadata::Err;
    using Target = typename BuildMetadata::Target;
    using namespace parse;
    using namespace impls;
    using namespace display;
    return this->identifier.as_str();
}

bool BuildMetadata::is_empty() const {
    using Err = typename BuildMetadata::Err;
    using Target = typename BuildMetadata::Target;
    using namespace parse;
    using namespace impls;
    using namespace display;
    return rusty::is_empty(this->identifier);
}

rusty::fmt::Result Version::fmt(rusty::fmt::Formatter& formatter) const {
    using Err = typename Version::Err;
    using namespace display;
    using namespace parse;
    const auto do_display = [&](rusty::fmt::Formatter& formatter) -> rusty::fmt::Result {
RUSTY_TRY(rusty::write_fmt(formatter, std::format("{0}.{1}.{2}", this->major, this->minor, this->patch)));
if (!rusty::is_empty(this->pre)) {
    RUSTY_TRY(rusty::write_fmt(formatter, std::format("-{0}", rusty::to_string(this->pre))));
}
if (!rusty::is_empty(this->build)) {
    RUSTY_TRY(rusty::write_fmt(formatter, std::format("+{0}", rusty::to_string(this->build))));
}
return rusty::fmt::Result::Ok(std::make_tuple());
};
    const auto do_len = [&]() -> size_t {
return (((((((display::digits(this->major) + 1) + display::digits(this->minor)) + 1) + display::digits(this->patch)) + (static_cast<size_t>(!rusty::is_empty(this->pre)))) + rusty::len(this->pre)) + (static_cast<size_t>(!rusty::is_empty(this->build)))) + rusty::len(this->build);
};
    return display::pad(formatter, std::move(do_display), std::move(do_len));
}

rusty::Result<Version, Error> Version::from_str(std::string_view text) {
    using Err = typename Version::Err;
    using namespace display;
    using namespace parse;
    if (rusty::is_empty(text)) {
        return rusty::Result<Version, Error>::Err(Error::new_(ErrorKind{ErrorKind_Empty{}}));
    }
    auto pos = Position_Major();
    auto [major, text_shadow1] = rusty::detail::deref_if_pointer_like(RUSTY_TRY_INTO(parse::numeric_identifier(std::move(rusty::to_string_view(text)), std::move(pos)), rusty::Result<Version, Error>));
    auto text_shadow2 = RUSTY_TRY_INTO(parse::dot(std::move(rusty::to_string_view(text_shadow1)), std::move(pos)), rusty::Result<Version, Error>);
    pos = Position_Minor();
    auto [minor, text_shadow3] = rusty::detail::deref_if_pointer_like(RUSTY_TRY_INTO(parse::numeric_identifier(std::move(rusty::to_string_view(text_shadow2)), std::move(pos)), rusty::Result<Version, Error>));
    auto text_shadow4 = RUSTY_TRY_INTO(parse::dot(std::move(rusty::to_string_view(text_shadow3)), std::move(pos)), rusty::Result<Version, Error>);
    pos = Position_Patch();
    auto [patch, text_shadow5] = rusty::detail::deref_if_pointer_like(RUSTY_TRY_INTO(parse::numeric_identifier(std::move(rusty::to_string_view(text_shadow4)), std::move(pos)), rusty::Result<Version, Error>));
    if (rusty::is_empty(text_shadow5)) {
        return rusty::Result<Version, Error>::Ok(Version::new_(std::move(major), std::move(minor), std::move(patch)));
    }
    auto _iflet_result0 = std::make_tuple(rusty::clone(rusty::clone(Prerelease::EMPTY)), std::move(text_shadow5));
    {
        auto&& _iflet_scrutinee = rusty::str_runtime::strip_prefix(text_shadow5, U'-');
        if (_iflet_scrutinee.is_some()) {
            auto text_shadow1 = _iflet_scrutinee.unwrap();
            pos = Position_Pre();
            auto [pre, text_shadow2] = rusty::detail::deref_if_pointer_like(RUSTY_TRY_INTO(parse::prerelease_identifier(std::move(rusty::to_string_view(text_shadow1))), rusty::Result<Version, Error>));
            if (rusty::is_empty(pre)) {
                return rusty::Result<Version, Error>::Err(Error::new_(ErrorKind{ErrorKind_EmptySegment{std::move(pos)}}));
            }
            _iflet_result0 = std::make_tuple(std::move(pre), std::move(text_shadow2));
        }
    }
    auto [pre, text_shadow6] = std::move(_iflet_result0);
    auto _iflet_result1 = std::make_tuple(rusty::clone(rusty::clone(BuildMetadata::EMPTY)), std::move(text_shadow6));
    {
        auto&& _iflet_scrutinee = rusty::str_runtime::strip_prefix(text_shadow6, U'+');
        if (_iflet_scrutinee.is_some()) {
            auto text_shadow1 = _iflet_scrutinee.unwrap();
            pos = Position_Build();
            auto [build, text_shadow2] = rusty::detail::deref_if_pointer_like(RUSTY_TRY_INTO(parse::build_identifier(std::move(rusty::to_string_view(text_shadow1))), rusty::Result<Version, Error>));
            if (rusty::is_empty(build)) {
                return rusty::Result<Version, Error>::Err(Error::new_(ErrorKind{ErrorKind_EmptySegment{std::move(pos)}}));
            }
            _iflet_result1 = std::make_tuple(std::move(build), std::move(text_shadow2));
        }
    }
    auto [build, text_shadow7] = std::move(_iflet_result1);
    if (auto&& _iflet_scrutinee = rusty::str_runtime::chars(text_shadow7).next(); _iflet_scrutinee.is_some()) {
        decltype(auto) unexpected = _iflet_scrutinee.unwrap();
        return rusty::Result<Version, Error>::Err(Error::new_(ErrorKind{ErrorKind_UnexpectedCharAfter{std::move(pos), std::move(unexpected)}}));
    }
    return rusty::Result<Version, Error>::Ok(Version(std::move(major), std::move(minor), std::move(patch), std::move(pre), std::move(build)));
}

Version Version::clone() const {
    using Err = typename Version::Err;
    using namespace display;
    using namespace parse;
    return Version(rusty::clone(this->major), rusty::clone(this->minor), rusty::clone(this->patch), rusty::clone(this->pre), rusty::clone(this->build));
}

void Version::assert_receiver_is_total_eq() const {
    using Err = typename Version::Err;
    using namespace display;
    using namespace parse;
}

bool Version::operator==(const Version& other) const {
    using Err = typename Version::Err;
    using namespace display;
    using namespace parse;
    return ((((this->major == other.major) && (this->minor == other.minor)) && (this->patch == other.patch)) && (this->pre == other.pre)) && (this->build == other.build);
}

rusty::cmp::Ordering Version::cmp(const Version& other) const {
    using Err = typename Version::Err;
    using namespace display;
    using namespace parse;
    return ({ auto&& _m = rusty::cmp::cmp(this->major, other.major); std::optional<rusty::cmp::Ordering> _match_value; bool _m_matched = false; if (!_m_matched && (_m == ::core::cmp::Ordering::Equal)) { _match_value.emplace(std::move(({ auto&& _m = rusty::cmp::cmp(this->minor, other.minor); std::optional<rusty::cmp::Ordering> _match_value; bool _m_matched = false; if (!_m_matched && (_m == ::core::cmp::Ordering::Equal)) { _match_value.emplace(std::move(({ auto&& _m = rusty::cmp::cmp(this->patch, other.patch); std::optional<rusty::cmp::Ordering> _match_value; bool _m_matched = false; if (!_m_matched && (_m == ::core::cmp::Ordering::Equal)) { _match_value.emplace(std::move(({ auto&& _m = rusty::cmp::cmp(this->pre, other.pre); std::optional<rusty::cmp::Ordering> _match_value; bool _m_matched = false; if (!_m_matched && (_m == ::core::cmp::Ordering::Equal)) { _match_value.emplace(std::move(rusty::cmp::cmp(this->build, other.build))); _m_matched = true; } if (!_m_matched) { const auto& cmp = _m; _match_value.emplace(std::move(cmp)); _m_matched = true; } if (!_m_matched) { rusty::intrinsics::unreachable(); } std::move(_match_value).value(); }))); _m_matched = true; } if (!_m_matched) { const auto& cmp = _m; _match_value.emplace(std::move(cmp)); _m_matched = true; } if (!_m_matched) { rusty::intrinsics::unreachable(); } std::move(_match_value).value(); }))); _m_matched = true; } if (!_m_matched) { const auto& cmp = _m; _match_value.emplace(std::move(cmp)); _m_matched = true; } if (!_m_matched) { rusty::intrinsics::unreachable(); } std::move(_match_value).value(); }))); _m_matched = true; } if (!_m_matched) { const auto& cmp = _m; _match_value.emplace(std::move(cmp)); _m_matched = true; } if (!_m_matched) { rusty::intrinsics::unreachable(); } std::move(_match_value).value(); });
}

std::partial_ordering Version::operator<=>(const Version& other) const {
    using Err = typename Version::Err;
    return rusty::to_partial_ordering([&]() -> rusty::Option<rusty::cmp::Ordering> {
        using namespace display;
        using namespace parse;
        return [&]() -> rusty::Option<rusty::cmp::Ordering> { auto&& _m = rusty::partial_cmp(this->major, other.major); if (_m.is_some()) { auto&& _mv0 = std::as_const(_m).unwrap(); if (_mv0 == ::core::cmp::Ordering::Equal) { return [&]() -> rusty::Option<rusty::cmp::Ordering> { auto&& _m = rusty::partial_cmp(this->minor, other.minor); if (_m.is_some()) { auto&& _mv0 = std::as_const(_m).unwrap(); if (_mv0 == ::core::cmp::Ordering::Equal) { return [&]() -> rusty::Option<rusty::cmp::Ordering> { auto&& _m = rusty::partial_cmp(this->patch, other.patch); if (_m.is_some()) { auto&& _mv0 = std::as_const(_m).unwrap(); if (_mv0 == ::core::cmp::Ordering::Equal) { return [&]() -> rusty::Option<rusty::cmp::Ordering> { auto&& _m = rusty::partial_cmp(this->pre, other.pre); if (_m.is_some()) { auto&& _mv0 = std::as_const(_m).unwrap(); if (_mv0 == ::core::cmp::Ordering::Equal) { return rusty::partial_cmp(this->build, other.build); } } if (true) { const auto& cmp = _m; return cmp; } return [&]() -> rusty::Option<rusty::cmp::Ordering> { rusty::intrinsics::unreachable(); }(); }(); } } if (true) { const auto& cmp = _m; return cmp; } return [&]() -> rusty::Option<rusty::cmp::Ordering> { rusty::intrinsics::unreachable(); }(); }(); } } if (true) { const auto& cmp = _m; return cmp; } return [&]() -> rusty::Option<rusty::cmp::Ordering> { rusty::intrinsics::unreachable(); }(); }(); } } if (true) { const auto& cmp = _m; return cmp; } return [&]() -> rusty::Option<rusty::cmp::Ordering> { rusty::intrinsics::unreachable(); }(); }();
    }());
}

template<typename __H>
void Version::hash(__H& state) const {
    using Err = typename Version::Err;
    using namespace display;
    using namespace parse;
    rusty::hash::hash(this->major, state);
    rusty::hash::hash(this->minor, state);
    rusty::hash::hash(this->patch, state);
    rusty::hash::hash(this->pre, state);
    rusty::hash::hash(this->build, state);
}

Version Version::new_(uint64_t major, uint64_t minor, uint64_t patch) {
    using Err = typename Version::Err;
    using namespace display;
    using namespace parse;
    return Version(std::move(major), std::move(minor), std::move(patch), rusty::clone(rusty::clone(Prerelease::EMPTY)), rusty::clone(rusty::clone(BuildMetadata::EMPTY)));
}

rusty::Result<Version, Error> Version::parse(std::string_view text) {
    using Err = typename Version::Err;
    using namespace display;
    using namespace parse;
    return Version::from_str(rusty::to_string_view(text));
}

rusty::cmp::Ordering Version::cmp_precedence(const Version& other) const {
    using Err = typename Version::Err;
    using namespace display;
    using namespace parse;
    return rusty::cmp::cmp(std::make_tuple(this->major, this->minor, this->patch, &this->pre), std::make_tuple(other.major, other.minor, other.patch, &other.pre));
}


// ── from test_autotrait.cppm ──


template<typename T>
void assert_send_sync();
void test();


// Rust-only libtest metadata const skipped: test (marker: test, should_panic: no)

template<typename T>
void assert_send_sync() {
}

void test() {
    ::assert_send_sync<BuildMetadata>();
    ::assert_send_sync<Comparator>();
    ::assert_send_sync<Error>();
    ::assert_send_sync<Prerelease>();
    ::assert_send_sync<Version>();
    ::assert_send_sync<VersionReq>();
    ::assert_send_sync<Op>();
}

// Rust-only libtest main omitted


// Runnable wrappers for expanded Rust test bodies
// Rust-only libtest wrapper metadata: marker=test should_panic=no
void rusty_test_test() {
    test();
}

// ── from test_identifier.cppm ──




namespace util {
    Version version(std::string_view text);
    Error version_err(std::string_view text);
    VersionReq req(std::string_view text);
    Error req_err(std::string_view text);
    Comparator comparator(std::string_view text);
    Error comparator_err(std::string_view text);
    Prerelease prerelease(std::string_view text);
    Error prerelease_err(std::string_view text);
    BuildMetadata build_metadata(std::string_view text);
    void assert_to_string(const auto& value, std::string_view expected);
}
void test_new();
void test_eq();
void test_prerelease();


namespace util {}
using namespace ::util;

// Rust-only unresolved import: using crate::Prerelease;

// Rust-only libtest metadata const skipped: test_new (marker: test_new, should_panic: no)

// Rust-only libtest metadata const skipped: test_eq (marker: test_eq, should_panic: no)

// Rust-only libtest metadata const skipped: test_prerelease (marker: test_prerelease, should_panic: no)

namespace util {

    Version version(std::string_view text);
    Error version_err(std::string_view text);
    VersionReq req(std::string_view text);
    Error req_err(std::string_view text);
    Comparator comparator(std::string_view text);
    Error comparator_err(std::string_view text);
    Prerelease prerelease(std::string_view text);
    Error prerelease_err(std::string_view text);
    BuildMetadata build_metadata(std::string_view text);
    void assert_to_string(const auto& value, std::string_view expected);

    // Rust-only unresolved import: using crate::BuildMetadata;
    // Rust-only unresolved import: using crate::Comparator;
    // Rust-only unresolved import: using crate::Error;
    // Rust-only unresolved import: using crate::Prerelease;
    // Rust-only unresolved import: using crate::Version;
    // Rust-only unresolved import: using crate::VersionReq;


    Version version(std::string_view text) {
        return Version::parse(text).unwrap();
    }

    Error version_err(std::string_view text) {
        return Version::parse(text).unwrap_err();
    }

    VersionReq req(std::string_view text) {
        return VersionReq::parse(text).unwrap();
    }

    Error req_err(std::string_view text) {
        return VersionReq::parse(text).unwrap_err();
    }

    Comparator comparator(std::string_view text) {
        return Comparator::parse(text).unwrap();
    }

    Error comparator_err(std::string_view text) {
        return Comparator::parse(text).unwrap_err();
    }

    Prerelease prerelease(std::string_view text) {
        return Prerelease::new_(text).unwrap();
    }

    Error prerelease_err(std::string_view text) {
        return Prerelease::new_(text).unwrap_err();
    }

    BuildMetadata build_metadata(std::string_view text) {
        return BuildMetadata::new_(text).unwrap();
    }

    void assert_to_string(const auto& value, std::string_view expected) {
        {
            auto&& _m0_tmp = rusty::to_string(value);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = std::string_view(expected);
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

}

void test_new() {
    const rusty::SafeFn<void(Prerelease, std::string_view)> test = +[](Prerelease identifier, std::string_view expected) {
        {
            auto&& _m0_tmp = rusty::is_empty(identifier);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::is_empty(expected);
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
            auto&& _m0_tmp = rusty::len(identifier);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::len(expected);
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
            auto&& _m0_tmp = identifier.as_str();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = std::string_view(expected);
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
            auto _m0 = &identifier;
            auto _m1 = &identifier;
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
            auto _m0 = &identifier;
            auto&& _m1_tmp = rusty::clone(identifier);
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
    };
    auto string = rusty::String::new_();
    const auto limit = (false ? static_cast<int32_t>(40) : static_cast<int32_t>(280));
    for (auto&& _ : rusty::for_in(rusty::range(0, limit))) {
        test(util::prerelease(std::move(rusty::to_string_view(string))), std::move(rusty::to_string_view(string)));
        string.push(U'1');
    }
    if (!false) {
        const auto& string_shadow1 = string.repeat(20000);
        test(util::prerelease(rusty::to_string_view(string_shadow1)), rusty::to_string_view(string_shadow1));
    }
}

void test_eq() {
    {
        auto&& _m0_tmp = util::prerelease(std::string_view("-"));
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = util::prerelease(std::string_view("-"));
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
        auto&& _m0_tmp = util::prerelease(std::string_view("a"));
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = util::prerelease(std::string_view("aa"));
        auto _m1 = &_m1_tmp;
        auto _m_tuple = std::make_tuple(_m0, _m1);
        bool _m_matched = false;
        if (!_m_matched) {
            auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
            auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
            if (rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val)) {
                const auto kind = rusty::panicking::AssertKind::Ne;
                rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
            }
            _m_matched = true;
        }
    }
    {
        auto&& _m0_tmp = util::prerelease(std::string_view("aa"));
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = util::prerelease(std::string_view("a"));
        auto _m1 = &_m1_tmp;
        auto _m_tuple = std::make_tuple(_m0, _m1);
        bool _m_matched = false;
        if (!_m_matched) {
            auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
            auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
            if (rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val)) {
                const auto kind = rusty::panicking::AssertKind::Ne;
                rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
            }
            _m_matched = true;
        }
    }
    {
        auto&& _m0_tmp = util::prerelease(std::string_view("aaaaaaaaa"));
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = util::prerelease(std::string_view("a"));
        auto _m1 = &_m1_tmp;
        auto _m_tuple = std::make_tuple(_m0, _m1);
        bool _m_matched = false;
        if (!_m_matched) {
            auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
            auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
            if (rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val)) {
                const auto kind = rusty::panicking::AssertKind::Ne;
                rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
            }
            _m_matched = true;
        }
    }
    {
        auto&& _m0_tmp = util::prerelease(std::string_view("a"));
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = util::prerelease(std::string_view("aaaaaaaaa"));
        auto _m1 = &_m1_tmp;
        auto _m_tuple = std::make_tuple(_m0, _m1);
        bool _m_matched = false;
        if (!_m_matched) {
            auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
            auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
            if (rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val)) {
                const auto kind = rusty::panicking::AssertKind::Ne;
                rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
            }
            _m_matched = true;
        }
    }
    {
        auto&& _m0_tmp = util::prerelease(std::string_view("aaaaaaaaa"));
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = util::prerelease(std::string_view("bbbbbbbbb"));
        auto _m1 = &_m1_tmp;
        auto _m_tuple = std::make_tuple(_m0, _m1);
        bool _m_matched = false;
        if (!_m_matched) {
            auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
            auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
            if (rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val)) {
                const auto kind = rusty::panicking::AssertKind::Ne;
                rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
            }
            _m_matched = true;
        }
    }
    {
        auto&& _m0_tmp = util::build_metadata(std::string_view("1"));
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = util::build_metadata(std::string_view("001"));
        auto _m1 = &_m1_tmp;
        auto _m_tuple = std::make_tuple(_m0, _m1);
        bool _m_matched = false;
        if (!_m_matched) {
            auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
            auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
            if (rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val)) {
                const auto kind = rusty::panicking::AssertKind::Ne;
                rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
            }
            _m_matched = true;
        }
    }
}

void test_prerelease() {
    const auto err = util::prerelease_err(std::string_view("1.b\0", 4));
    util::assert_to_string(std::move(err), std::string_view("unexpected character in pre-release identifier"));
}

// Rust-only libtest main omitted


// Runnable wrappers for expanded Rust test bodies
// Rust-only libtest wrapper metadata: marker=test_new should_panic=no
void rusty_test_test_new() {
    test_new();
}
// Rust-only libtest wrapper metadata: marker=test_eq should_panic=no
void rusty_test_test_eq() {
    test_eq();
}
// Rust-only libtest wrapper metadata: marker=test_prerelease should_panic=no
void rusty_test_test_prerelease() {
    test_prerelease();
}

// ── from test_version.cppm ──











void test_parse();
void test_eq();
void test_ne();
void test_display();
void test_lt();
void test_le();
void test_gt();
void test_ge();
void test_spec_order();
void test_align();


namespace util {}
using namespace ::util;

// Rust-only unresolved import: using crate::BuildMetadata;
// Rust-only unresolved import: using crate::Prerelease;
// Rust-only unresolved import: using crate::Version;

// Rust-only libtest metadata const skipped: test_parse (marker: test_parse, should_panic: no)

// Rust-only libtest metadata const skipped: test_eq (marker: test_eq, should_panic: no)

// Rust-only libtest metadata const skipped: test_ne (marker: test_ne, should_panic: no)

// Rust-only libtest metadata const skipped: test_display (marker: test_display, should_panic: no)

// Rust-only libtest metadata const skipped: test_lt (marker: test_lt, should_panic: no)

// Rust-only libtest metadata const skipped: test_le (marker: test_le, should_panic: no)

// Rust-only libtest metadata const skipped: test_gt (marker: test_gt, should_panic: no)

// Rust-only libtest metadata const skipped: test_ge (marker: test_ge, should_panic: no)

// Rust-only libtest metadata const skipped: test_spec_order (marker: test_spec_order, should_panic: no)

// Rust-only libtest metadata const skipped: test_align (marker: test_align, should_panic: no)


void test_parse() {
    const auto err = util::version_err(std::string_view(""));
    util::assert_to_string(std::move(err), std::string_view("empty string, expected a semver version"));
    const auto err_shadow1 = util::version_err(std::string_view("  "));
    util::assert_to_string(std::move(err_shadow1), std::string_view("unexpected character ' ' while parsing major version number"));
    const auto err_shadow2 = util::version_err(std::string_view("1"));
    util::assert_to_string(std::move(err_shadow2), std::string_view("unexpected end of input while parsing major version number"));
    const auto err_shadow3 = util::version_err(std::string_view("1.2"));
    util::assert_to_string(std::move(err_shadow3), std::string_view("unexpected end of input while parsing minor version number"));
    const auto err_shadow4 = util::version_err(std::string_view("1.2.3-"));
    util::assert_to_string(std::move(err_shadow4), std::string_view("empty identifier segment in pre-release identifier"));
    const auto err_shadow5 = util::version_err(std::string_view("a.b.c"));
    util::assert_to_string(std::move(err_shadow5), std::string_view("unexpected character 'a' while parsing major version number"));
    const auto err_shadow6 = util::version_err(std::string_view("1.2.3 abc"));
    util::assert_to_string(std::move(err_shadow6), std::string_view("unexpected character ' ' after patch version number"));
    const auto err_shadow7 = util::version_err(std::string_view("1.2.3-01"));
    util::assert_to_string(std::move(err_shadow7), std::string_view("invalid leading zero in pre-release identifier"));
    const auto err_shadow8 = util::version_err(std::string_view("1.2.3++"));
    util::assert_to_string(std::move(err_shadow8), std::string_view("empty identifier segment in build metadata"));
    const auto err_shadow9 = util::version_err(std::string_view("07"));
    util::assert_to_string(std::move(err_shadow9), std::string_view("invalid leading zero in major version number"));
    const auto err_shadow10 = util::version_err(std::string_view("111111111111111111111.0.0"));
    util::assert_to_string(std::move(err_shadow10), std::string_view("value of major version number exceeds u64::MAX"));
    const auto err_shadow11 = util::version_err(std::string_view("8\0", 2));
    util::assert_to_string(std::move(err_shadow11), std::string_view("unexpected character '\\0' after major version number"));
    const auto parsed = util::version(std::string_view("1.2.3"));
    const auto expected = Version::new_(1, 2, 3);
    {
        auto _m0 = &parsed;
        auto _m1 = &expected;
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
    const auto expected_shadow1 = Version{.major = 1, .minor = 2, .patch = 3, .pre = rusty::clone(rusty::clone(Prerelease::EMPTY)), .build = rusty::clone(rusty::clone(BuildMetadata::EMPTY))};
    {
        auto _m0 = &parsed;
        auto _m1 = &expected_shadow1;
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
    const auto parsed_shadow1 = util::version(std::string_view("1.2.3-alpha1"));
    const auto expected_shadow2 = Version{.major = 1, .minor = 2, .patch = 3, .pre = util::prerelease(std::string_view("alpha1")), .build = rusty::clone(rusty::clone(BuildMetadata::EMPTY))};
    {
        auto _m0 = &parsed_shadow1;
        auto _m1 = &expected_shadow2;
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
    const auto parsed_shadow2 = util::version(std::string_view("1.2.3+build5"));
    const auto expected_shadow3 = Version{.major = 1, .minor = 2, .patch = 3, .pre = rusty::clone(rusty::clone(Prerelease::EMPTY)), .build = util::build_metadata(std::string_view("build5"))};
    {
        auto _m0 = &parsed_shadow2;
        auto _m1 = &expected_shadow3;
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
    const auto parsed_shadow3 = util::version(std::string_view("1.2.3+5build"));
    const auto expected_shadow4 = Version{.major = 1, .minor = 2, .patch = 3, .pre = rusty::clone(rusty::clone(Prerelease::EMPTY)), .build = util::build_metadata(std::string_view("5build"))};
    {
        auto _m0 = &parsed_shadow3;
        auto _m1 = &expected_shadow4;
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
    const auto parsed_shadow4 = util::version(std::string_view("1.2.3-alpha1+build5"));
    const auto expected_shadow5 = Version{.major = 1, .minor = 2, .patch = 3, .pre = util::prerelease(std::string_view("alpha1")), .build = util::build_metadata(std::string_view("build5"))};
    {
        auto _m0 = &parsed_shadow4;
        auto _m1 = &expected_shadow5;
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
    const auto parsed_shadow5 = util::version(std::string_view("1.2.3-1.alpha1.9+build5.7.3aedf"));
    const auto expected_shadow6 = Version{.major = 1, .minor = 2, .patch = 3, .pre = util::prerelease(std::string_view("1.alpha1.9")), .build = util::build_metadata(std::string_view("build5.7.3aedf"))};
    {
        auto _m0 = &parsed_shadow5;
        auto _m1 = &expected_shadow6;
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
    const auto parsed_shadow6 = util::version(std::string_view("1.2.3-0a.alpha1.9+05build.7.3aedf"));
    const auto expected_shadow7 = Version{.major = 1, .minor = 2, .patch = 3, .pre = util::prerelease(std::string_view("0a.alpha1.9")), .build = util::build_metadata(std::string_view("05build.7.3aedf"))};
    {
        auto _m0 = &parsed_shadow6;
        auto _m1 = &expected_shadow7;
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
    const auto parsed_shadow7 = util::version(std::string_view("0.4.0-beta.1+0851523"));
    const auto expected_shadow8 = Version{.major = 0, .minor = 4, .patch = 0, .pre = util::prerelease(std::string_view("beta.1")), .build = util::build_metadata(std::string_view("0851523"))};
    {
        auto _m0 = &parsed_shadow7;
        auto _m1 = &expected_shadow8;
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
    const auto parsed_shadow8 = util::version(std::string_view("1.1.0-beta-10"));
    const auto expected_shadow9 = Version{.major = 1, .minor = 1, .patch = 0, .pre = util::prerelease(std::string_view("beta-10")), .build = rusty::clone(rusty::clone(BuildMetadata::EMPTY))};
    {
        auto _m0 = &parsed_shadow8;
        auto _m1 = &expected_shadow9;
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


void test_ne() {
    {
        auto&& _m0_tmp = util::version(std::string_view("0.0.0"));
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = util::version(std::string_view("0.0.1"));
        auto _m1 = &_m1_tmp;
        auto _m_tuple = std::make_tuple(_m0, _m1);
        bool _m_matched = false;
        if (!_m_matched) {
            auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
            auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
            if (rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val)) {
                const auto kind = rusty::panicking::AssertKind::Ne;
                rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
            }
            _m_matched = true;
        }
    }
    {
        auto&& _m0_tmp = util::version(std::string_view("0.0.0"));
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = util::version(std::string_view("0.1.0"));
        auto _m1 = &_m1_tmp;
        auto _m_tuple = std::make_tuple(_m0, _m1);
        bool _m_matched = false;
        if (!_m_matched) {
            auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
            auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
            if (rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val)) {
                const auto kind = rusty::panicking::AssertKind::Ne;
                rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
            }
            _m_matched = true;
        }
    }
    {
        auto&& _m0_tmp = util::version(std::string_view("0.0.0"));
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = util::version(std::string_view("1.0.0"));
        auto _m1 = &_m1_tmp;
        auto _m_tuple = std::make_tuple(_m0, _m1);
        bool _m_matched = false;
        if (!_m_matched) {
            auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
            auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
            if (rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val)) {
                const auto kind = rusty::panicking::AssertKind::Ne;
                rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
            }
            _m_matched = true;
        }
    }
    {
        auto&& _m0_tmp = util::version(std::string_view("1.2.3-alpha"));
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = util::version(std::string_view("1.2.3-beta"));
        auto _m1 = &_m1_tmp;
        auto _m_tuple = std::make_tuple(_m0, _m1);
        bool _m_matched = false;
        if (!_m_matched) {
            auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
            auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
            if (rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val)) {
                const auto kind = rusty::panicking::AssertKind::Ne;
                rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
            }
            _m_matched = true;
        }
    }
    {
        auto&& _m0_tmp = util::version(std::string_view("1.2.3+23"));
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = util::version(std::string_view("1.2.3+42"));
        auto _m1 = &_m1_tmp;
        auto _m_tuple = std::make_tuple(_m0, _m1);
        bool _m_matched = false;
        if (!_m_matched) {
            auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
            auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
            if (rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val)) {
                const auto kind = rusty::panicking::AssertKind::Ne;
                rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
            }
            _m_matched = true;
        }
    }
}

void test_display() {
    util::assert_to_string(util::version(std::string_view("1.2.3")), std::string_view("1.2.3"));
    util::assert_to_string(util::version(std::string_view("1.2.3-alpha1")), std::string_view("1.2.3-alpha1"));
    util::assert_to_string(util::version(std::string_view("1.2.3+build.42")), std::string_view("1.2.3+build.42"));
    util::assert_to_string(util::version(std::string_view("1.2.3-alpha1+42")), std::string_view("1.2.3-alpha1+42"));
}

void test_lt() {
    if (!(util::version(std::string_view("0.0.0")) < util::version(std::string_view("1.2.3-alpha2")))) {
        rusty::panicking::panic("assertion failed: version(\"0.0.0\") < version(\"1.2.3-alpha2\")");
    }
    if (!(util::version(std::string_view("1.0.0")) < util::version(std::string_view("1.2.3-alpha2")))) {
        rusty::panicking::panic("assertion failed: version(\"1.0.0\") < version(\"1.2.3-alpha2\")");
    }
    if (!(util::version(std::string_view("1.2.0")) < util::version(std::string_view("1.2.3-alpha2")))) {
        rusty::panicking::panic("assertion failed: version(\"1.2.0\") < version(\"1.2.3-alpha2\")");
    }
    if (!(util::version(std::string_view("1.2.3-alpha1")) < util::version(std::string_view("1.2.3")))) {
        rusty::panicking::panic("assertion failed: version(\"1.2.3-alpha1\") < version(\"1.2.3\")");
    }
    if (!(util::version(std::string_view("1.2.3-alpha1")) < util::version(std::string_view("1.2.3-alpha2")))) {
        rusty::panicking::panic("assertion failed: version(\"1.2.3-alpha1\") < version(\"1.2.3-alpha2\")");
    }
    if (!!(util::version(std::string_view("1.2.3-alpha2")) < util::version(std::string_view("1.2.3-alpha2")))) {
        rusty::panicking::panic("assertion failed: !(version(\"1.2.3-alpha2\") < version(\"1.2.3-alpha2\"))");
    }
    if (!(util::version(std::string_view("1.2.3+23")) < util::version(std::string_view("1.2.3+42")))) {
        rusty::panicking::panic("assertion failed: version(\"1.2.3+23\") < version(\"1.2.3+42\")");
    }
}

void test_le() {
    if (!(util::version(std::string_view("0.0.0")) <= util::version(std::string_view("1.2.3-alpha2")))) {
        rusty::panicking::panic("assertion failed: version(\"0.0.0\") <= version(\"1.2.3-alpha2\")");
    }
    if (!(util::version(std::string_view("1.0.0")) <= util::version(std::string_view("1.2.3-alpha2")))) {
        rusty::panicking::panic("assertion failed: version(\"1.0.0\") <= version(\"1.2.3-alpha2\")");
    }
    if (!(util::version(std::string_view("1.2.0")) <= util::version(std::string_view("1.2.3-alpha2")))) {
        rusty::panicking::panic("assertion failed: version(\"1.2.0\") <= version(\"1.2.3-alpha2\")");
    }
    if (!(util::version(std::string_view("1.2.3-alpha1")) <= util::version(std::string_view("1.2.3-alpha2")))) {
        rusty::panicking::panic("assertion failed: version(\"1.2.3-alpha1\") <= version(\"1.2.3-alpha2\")");
    }
    if (!(util::version(std::string_view("1.2.3-alpha2")) <= util::version(std::string_view("1.2.3-alpha2")))) {
        rusty::panicking::panic("assertion failed: version(\"1.2.3-alpha2\") <= version(\"1.2.3-alpha2\")");
    }
    if (!(util::version(std::string_view("1.2.3+23")) <= util::version(std::string_view("1.2.3+42")))) {
        rusty::panicking::panic("assertion failed: version(\"1.2.3+23\") <= version(\"1.2.3+42\")");
    }
}

void test_gt() {
    if (!(util::version(std::string_view("1.2.3-alpha2")) > util::version(std::string_view("0.0.0")))) {
        rusty::panicking::panic("assertion failed: version(\"1.2.3-alpha2\") > version(\"0.0.0\")");
    }
    if (!(util::version(std::string_view("1.2.3-alpha2")) > util::version(std::string_view("1.0.0")))) {
        rusty::panicking::panic("assertion failed: version(\"1.2.3-alpha2\") > version(\"1.0.0\")");
    }
    if (!(util::version(std::string_view("1.2.3-alpha2")) > util::version(std::string_view("1.2.0")))) {
        rusty::panicking::panic("assertion failed: version(\"1.2.3-alpha2\") > version(\"1.2.0\")");
    }
    if (!(util::version(std::string_view("1.2.3-alpha2")) > util::version(std::string_view("1.2.3-alpha1")))) {
        rusty::panicking::panic("assertion failed: version(\"1.2.3-alpha2\") > version(\"1.2.3-alpha1\")");
    }
    if (!(util::version(std::string_view("1.2.3")) > util::version(std::string_view("1.2.3-alpha2")))) {
        rusty::panicking::panic("assertion failed: version(\"1.2.3\") > version(\"1.2.3-alpha2\")");
    }
    if (!!(util::version(std::string_view("1.2.3-alpha2")) > util::version(std::string_view("1.2.3-alpha2")))) {
        rusty::panicking::panic("assertion failed: !(version(\"1.2.3-alpha2\") > version(\"1.2.3-alpha2\"))");
    }
    if (!!(util::version(std::string_view("1.2.3+23")) > util::version(std::string_view("1.2.3+42")))) {
        rusty::panicking::panic("assertion failed: !(version(\"1.2.3+23\") > version(\"1.2.3+42\"))");
    }
}

void test_ge() {
    if (!(util::version(std::string_view("1.2.3-alpha2")) >= util::version(std::string_view("0.0.0")))) {
        rusty::panicking::panic("assertion failed: version(\"1.2.3-alpha2\") >= version(\"0.0.0\")");
    }
    if (!(util::version(std::string_view("1.2.3-alpha2")) >= util::version(std::string_view("1.0.0")))) {
        rusty::panicking::panic("assertion failed: version(\"1.2.3-alpha2\") >= version(\"1.0.0\")");
    }
    if (!(util::version(std::string_view("1.2.3-alpha2")) >= util::version(std::string_view("1.2.0")))) {
        rusty::panicking::panic("assertion failed: version(\"1.2.3-alpha2\") >= version(\"1.2.0\")");
    }
    if (!(util::version(std::string_view("1.2.3-alpha2")) >= util::version(std::string_view("1.2.3-alpha1")))) {
        rusty::panicking::panic("assertion failed: version(\"1.2.3-alpha2\") >= version(\"1.2.3-alpha1\")");
    }
    if (!(util::version(std::string_view("1.2.3-alpha2")) >= util::version(std::string_view("1.2.3-alpha2")))) {
        rusty::panicking::panic("assertion failed: version(\"1.2.3-alpha2\") >= version(\"1.2.3-alpha2\")");
    }
    if (!!(util::version(std::string_view("1.2.3+23")) >= util::version(std::string_view("1.2.3+42")))) {
        rusty::panicking::panic("assertion failed: !(version(\"1.2.3+23\") >= version(\"1.2.3+42\"))");
    }
}

void test_spec_order() {
    const auto vs = std::array{"1.0.0-alpha", "1.0.0-alpha.1", "1.0.0-alpha.beta", "1.0.0-beta", "1.0.0-beta.2", "1.0.0-beta.11", "1.0.0-rc.1", "1.0.0"};
    auto i = 1;
    while (i < rusty::len(vs)) {
        const auto a = util::version(vs[i - 1]);
        const auto b = util::version(vs[i]);
        if (!(a < b)) {
            {
                rusty::panicking::panic_fmt(std::format("nope {0} < {1}", rusty::to_debug_string(a), rusty::to_debug_string(b)));
            }
        }
        [&]() { static_cast<void>(i += 1); return std::make_tuple(); }();
    }
}

void test_align() {
    const auto version_shadow1 = version(std::string_view("1.2.3-rc1"));
    {
        auto&& _m0_tmp = std::string_view("1.2.3-rc1           ");
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0:20}", rusty::to_string(version_shadow1))));
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
        auto&& _m0_tmp = std::string_view("*****1.2.3-rc1******");
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0:*^20}", rusty::to_string(version_shadow1))));
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
        auto&& _m0_tmp = std::string_view("           1.2.3-rc1");
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0:>20}", rusty::to_string(version_shadow1))));
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

// Rust-only libtest main omitted


// Runnable wrappers for expanded Rust test bodies
// Rust-only libtest wrapper metadata: marker=test_parse should_panic=no
void rusty_test_test_parse() {
    test_parse();
}
// Rust-only libtest wrapper metadata: marker=test_eq should_panic=no
// Rust-only libtest wrapper metadata: marker=test_ne should_panic=no
void rusty_test_test_ne() {
    test_ne();
}
// Rust-only libtest wrapper metadata: marker=test_display should_panic=no
void rusty_test_test_display() {
    test_display();
}
// Rust-only libtest wrapper metadata: marker=test_lt should_panic=no
void rusty_test_test_lt() {
    test_lt();
}
// Rust-only libtest wrapper metadata: marker=test_le should_panic=no
void rusty_test_test_le() {
    test_le();
}
// Rust-only libtest wrapper metadata: marker=test_gt should_panic=no
void rusty_test_test_gt() {
    test_gt();
}
// Rust-only libtest wrapper metadata: marker=test_ge should_panic=no
void rusty_test_test_ge() {
    test_ge();
}
// Rust-only libtest wrapper metadata: marker=test_spec_order should_panic=no
void rusty_test_test_spec_order() {
    test_spec_order();
}
// Rust-only libtest wrapper metadata: marker=test_align should_panic=no
void rusty_test_test_align() {
    test_align();
}

// ── from test_version_req.cppm ──





















void assert_match_all(const VersionReq& req, std::span<const std::string_view> versions);
void assert_match_none(const VersionReq& req, std::span<const std::string_view> versions);
void test_basic();
void test_default();
void test_exact();
void test_greater_than();
void test_less_than();
void test_multiple();
void test_whitespace_delimited_comparator_sets();
void test_tilde();
void test_caret();
void test_wildcard();
void test_logical_or();
void test_any();
void test_pre();
void test_parse();
void test_comparator_parse();
void test_cargo3202();
void test_digit_after_wildcard();
void test_eq_hash();
void test_leading_digit_in_pre_and_build();
void test_wildcard_and_another();


namespace util {}
using namespace ::util;

// Rust-only unresolved import: using std::collections::hash_map::DefaultHasher;


// Rust-only unresolved import: using crate::VersionReq;

// Rust-only libtest metadata const skipped: test_basic (marker: test_basic, should_panic: no)

// Rust-only libtest metadata const skipped: test_default (marker: test_default, should_panic: no)

// Rust-only libtest metadata const skipped: test_exact (marker: test_exact, should_panic: no)

// Rust-only libtest metadata const skipped: test_greater_than (marker: test_greater_than, should_panic: no)

// Rust-only libtest metadata const skipped: test_less_than (marker: test_less_than, should_panic: no)

// Rust-only libtest metadata const skipped: test_multiple (marker: test_multiple, should_panic: no)

// Rust-only libtest metadata const skipped: test_whitespace_delimited_comparator_sets (marker: test_whitespace_delimited_comparator_sets, should_panic: no)

// Rust-only libtest metadata const skipped: test_tilde (marker: test_tilde, should_panic: no)

// Rust-only libtest metadata const skipped: test_caret (marker: test_caret, should_panic: no)

// Rust-only libtest metadata const skipped: test_wildcard (marker: test_wildcard, should_panic: no)

// Rust-only libtest metadata const skipped: test_logical_or (marker: test_logical_or, should_panic: no)

// Rust-only libtest metadata const skipped: test_any (marker: test_any, should_panic: no)

// Rust-only libtest metadata const skipped: test_pre (marker: test_pre, should_panic: no)

// Rust-only libtest metadata const skipped: test_parse (marker: test_parse, should_panic: no)

// Rust-only libtest metadata const skipped: test_comparator_parse (marker: test_comparator_parse, should_panic: no)

// Rust-only libtest metadata const skipped: test_cargo3202 (marker: test_cargo3202, should_panic: no)

// Rust-only libtest metadata const skipped: test_digit_after_wildcard (marker: test_digit_after_wildcard, should_panic: no)

// Rust-only libtest metadata const skipped: test_eq_hash (marker: test_eq_hash, should_panic: no)

// Rust-only libtest metadata const skipped: test_leading_digit_in_pre_and_build (marker: test_leading_digit_in_pre_and_build, should_panic: no)

// Rust-only libtest metadata const skipped: test_wildcard_and_another (marker: test_wildcard_and_another, should_panic: no)


void assert_match_all(const VersionReq& req, std::span<const std::string_view> versions) {
    for (auto&& string : rusty::for_in(rusty::iter(versions))) {
        const auto parsed = util::version(rusty::to_string_view(string));
        if (!req.matches(parsed)) {
            {
                rusty::panicking::panic_fmt(std::format("did not match {0}", rusty::to_string(string)));
            }
        }
    }
}

void assert_match_none(const VersionReq& req, std::span<const std::string_view> versions) {
    for (auto&& string : rusty::for_in(rusty::iter(versions))) {
        const auto parsed = util::version(rusty::to_string_view(string));
        if (!!req.matches(parsed)) {
            {
                rusty::panicking::panic_fmt(std::format("matched {0}", rusty::to_string(string)));
            }
        }
    }
}

void test_basic() {
    const auto& r = util::req(std::string_view("1.0.0"));
    util::assert_to_string(r, std::string_view("^1.0.0"));
    ::assert_match_all(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("1.0.0"), std::string_view("1.1.0"), std::string_view("1.0.1")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 5> _slice_ref_tmp = {std::string_view("0.9.9"), std::string_view("0.10.0"), std::string_view("0.1.0"), std::string_view("1.0.0-pre"), std::string_view("1.0.1-pre")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
}

void test_default() {
    const auto& r = VersionReq::default_();
    {
        auto _m0 = &r;
        auto&& _m1_tmp = rusty::clone(VersionReq::STAR);
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

void test_exact() {
    const auto& r = util::req(std::string_view("=1.0.0"));
    util::assert_to_string(r, std::string_view("=1.0.0"));
    ::assert_match_all(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 1> _slice_ref_tmp = {std::string_view("1.0.0")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 5> _slice_ref_tmp = {std::string_view("1.0.1"), std::string_view("0.9.9"), std::string_view("0.10.0"), std::string_view("0.1.0"), std::string_view("1.0.0-pre")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow1 = util::req(std::string_view("=0.9.0"));
    util::assert_to_string(r_shadow1, std::string_view("=0.9.0"));
    ::assert_match_all(r_shadow1, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 1> _slice_ref_tmp = {std::string_view("0.9.0")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow1, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 4> _slice_ref_tmp = {std::string_view("0.9.1"), std::string_view("1.9.0"), std::string_view("0.0.9"), std::string_view("0.9.0-pre")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow2 = util::req(std::string_view("=0.0.2"));
    util::assert_to_string(r_shadow2, std::string_view("=0.0.2"));
    ::assert_match_all(r_shadow2, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 1> _slice_ref_tmp = {std::string_view("0.0.2")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow2, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("0.0.1"), std::string_view("0.0.3"), std::string_view("0.0.2-pre")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow3 = util::req(std::string_view("=0.1.0-beta2.a"));
    util::assert_to_string(r_shadow3, std::string_view("=0.1.0-beta2.a"));
    ::assert_match_all(r_shadow3, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 1> _slice_ref_tmp = {std::string_view("0.1.0-beta2.a")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow3, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 4> _slice_ref_tmp = {std::string_view("0.9.1"), std::string_view("0.1.0"), std::string_view("0.1.1-beta2.a"), std::string_view("0.1.0-beta2")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow4 = util::req(std::string_view("=0.1.0+meta"));
    util::assert_to_string(r_shadow4, std::string_view("=0.1.0"));
    ::assert_match_all(r_shadow4, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("0.1.0"), std::string_view("0.1.0+meta"), std::string_view("0.1.0+any")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
}

void test_greater_than() {
    const auto& r = util::req(std::string_view(">= 1.0.0"));
    util::assert_to_string(r, std::string_view(">=1.0.0"));
    ::assert_match_all(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 2> _slice_ref_tmp = {std::string_view("1.0.0"), std::string_view("2.0.0")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 4> _slice_ref_tmp = {std::string_view("0.1.0"), std::string_view("0.0.1"), std::string_view("1.0.0-pre"), std::string_view("2.0.0-pre")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow1 = util::req(std::string_view(">= 2.1.0-alpha2"));
    util::assert_to_string(r_shadow1, std::string_view(">=2.1.0-alpha2"));
    ::assert_match_all(r_shadow1, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 4> _slice_ref_tmp = {std::string_view("2.1.0-alpha2"), std::string_view("2.1.0-alpha3"), std::string_view("2.1.0"), std::string_view("3.0.0")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow1, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 4> _slice_ref_tmp = {std::string_view("2.0.0"), std::string_view("2.1.0-alpha1"), std::string_view("2.0.0-alpha2"), std::string_view("3.0.0-alpha2")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
}

void test_less_than() {
    const auto& r = util::req(std::string_view("< 1.0.0"));
    util::assert_to_string(r, std::string_view("<1.0.0"));
    ::assert_match_all(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 2> _slice_ref_tmp = {std::string_view("0.1.0"), std::string_view("0.0.1")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 4> _slice_ref_tmp = {std::string_view("1.0.0"), std::string_view("1.0.0-beta"), std::string_view("1.0.1"), std::string_view("0.9.9-alpha")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow1 = util::req(std::string_view("<= 2.1.0-alpha2"));
    ::assert_match_all(r_shadow1, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 4> _slice_ref_tmp = {std::string_view("2.1.0-alpha2"), std::string_view("2.1.0-alpha1"), std::string_view("2.0.0"), std::string_view("1.0.0")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow1, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 4> _slice_ref_tmp = {std::string_view("2.1.0"), std::string_view("2.2.0-alpha1"), std::string_view("2.0.0-alpha2"), std::string_view("1.0.0-alpha2")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow2 = util::req(std::string_view(">1.0.0-alpha, <1.0.0"));
    ::assert_match_all(r_shadow2, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 1> _slice_ref_tmp = {std::string_view("1.0.0-beta")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow3 = util::req(std::string_view(">1.0.0-alpha, <1.0"));
    ::assert_match_none(r_shadow3, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 1> _slice_ref_tmp = {std::string_view("1.0.0-beta")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow4 = util::req(std::string_view(">1.0.0-alpha, <1"));
    ::assert_match_none(r_shadow4, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 1> _slice_ref_tmp = {std::string_view("1.0.0-beta")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
}

void test_multiple() {
    const auto& r = util::req(std::string_view("> 0.0.9, <= 2.5.3"));
    util::assert_to_string(r, std::string_view(">0.0.9, <=2.5.3"));
    ::assert_match_all(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("0.0.10"), std::string_view("1.0.0"), std::string_view("2.5.3")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 2> _slice_ref_tmp = {std::string_view("0.0.8"), std::string_view("2.5.4")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow1 = util::req(std::string_view("0.3.0, 0.4.0"));
    util::assert_to_string(r_shadow1, std::string_view("^0.3.0, ^0.4.0"));
    ::assert_match_none(r_shadow1, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("0.0.8"), std::string_view("0.3.0"), std::string_view("0.4.0")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow2 = util::req(std::string_view("<= 0.2.0, >= 0.5.0"));
    util::assert_to_string(r_shadow2, std::string_view("<=0.2.0, >=0.5.0"));
    ::assert_match_none(r_shadow2, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("0.0.8"), std::string_view("0.3.0"), std::string_view("0.5.1")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow3 = util::req(std::string_view("0.1.0, 0.1.4, 0.1.6"));
    util::assert_to_string(r_shadow3, std::string_view("^0.1.0, ^0.1.4, ^0.1.6"));
    ::assert_match_all(r_shadow3, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 2> _slice_ref_tmp = {std::string_view("0.1.6"), std::string_view("0.1.9")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow3, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("0.1.0"), std::string_view("0.1.4"), std::string_view("0.2.0")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto err = util::req_err(std::string_view("> 0.1.0,"));
    util::assert_to_string(std::move(err), std::string_view("unexpected end of input while parsing major version number"));
    const auto err_shadow1 = util::req_err(std::string_view("> 0.3.0, ,"));
    util::assert_to_string(std::move(err_shadow1), std::string_view("unexpected character ',' while parsing major version number"));
    const auto& r_shadow4 = util::req(std::string_view(">=0.5.1-alpha3, <0.6"));
    util::assert_to_string(r_shadow4, std::string_view(">=0.5.1-alpha3, <0.6"));
    ::assert_match_all(r_shadow4, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 5> _slice_ref_tmp = {std::string_view("0.5.1-alpha3"), std::string_view("0.5.1-alpha4"), std::string_view("0.5.1-beta"), std::string_view("0.5.1"), std::string_view("0.5.5")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow4, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 4> _slice_ref_tmp = {std::string_view("0.5.1-alpha1"), std::string_view("0.5.2-alpha3"), std::string_view("0.5.5-pre"), std::string_view("0.5.0-pre")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow4, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 2> _slice_ref_tmp = {std::string_view("0.6.0"), std::string_view("0.6.0-pre")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto err_shadow2 = util::req_err(std::string_view("1.2.3 - 2.3.4"));
    util::assert_to_string(std::move(err_shadow2), std::string_view("expected comma after patch version number, found '-'"));
    const auto err_shadow3 = util::req_err(std::string_view(">1, >2, >3, >4, >5, >6, >7, >8, >9, >10, >11, >12, >13, >14, >15, >16, >17, >18, >19, >20, >21, >22, >23, >24, >25, >26, >27, >28, >29, >30, >31, >32, >33"));
    util::assert_to_string(std::move(err_shadow3), std::string_view("excessive number of version comparators"));
}

void test_whitespace_delimited_comparator_sets() {
    const auto err = util::req_err(std::string_view("> 0.0.9 <= 2.5.3"));
    util::assert_to_string(std::move(err), std::string_view("expected comma after patch version number, found '<'"));
}

void test_tilde() {
    const auto& r = util::req(std::string_view("~1"));
    ::assert_match_all(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("1.0.0"), std::string_view("1.0.1"), std::string_view("1.1.1")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("0.9.1"), std::string_view("2.9.0"), std::string_view("0.0.9")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow1 = util::req(std::string_view("~1.2"));
    ::assert_match_all(r_shadow1, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 2> _slice_ref_tmp = {std::string_view("1.2.0"), std::string_view("1.2.1")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow1, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("1.1.1"), std::string_view("1.3.0"), std::string_view("0.0.9")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow2 = util::req(std::string_view("~1.2.2"));
    ::assert_match_all(r_shadow2, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 2> _slice_ref_tmp = {std::string_view("1.2.2"), std::string_view("1.2.4")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow2, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 5> _slice_ref_tmp = {std::string_view("1.2.1"), std::string_view("1.9.0"), std::string_view("1.0.9"), std::string_view("2.0.1"), std::string_view("0.1.3")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow3 = util::req(std::string_view("~1.2.3-beta.2"));
    ::assert_match_all(r_shadow3, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 4> _slice_ref_tmp = {std::string_view("1.2.3"), std::string_view("1.2.4"), std::string_view("1.2.3-beta.2"), std::string_view("1.2.3-beta.4")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow3, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 4> _slice_ref_tmp = {std::string_view("1.3.3"), std::string_view("1.1.4"), std::string_view("1.2.3-beta.1"), std::string_view("1.2.4-beta.2")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
}

void test_caret() {
    const auto& r = util::req(std::string_view("^1"));
    ::assert_match_all(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 4> _slice_ref_tmp = {std::string_view("1.1.2"), std::string_view("1.1.0"), std::string_view("1.2.1"), std::string_view("1.0.1")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("0.9.1"), std::string_view("2.9.0"), std::string_view("0.1.4")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("1.0.0-beta1"), std::string_view("0.1.0-alpha"), std::string_view("1.0.1-pre")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow1 = util::req(std::string_view("^1.1"));
    ::assert_match_all(r_shadow1, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("1.1.2"), std::string_view("1.1.0"), std::string_view("1.2.1")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow1, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 4> _slice_ref_tmp = {std::string_view("0.9.1"), std::string_view("2.9.0"), std::string_view("1.0.1"), std::string_view("0.1.4")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow2 = util::req(std::string_view("^1.1.2"));
    ::assert_match_all(r_shadow2, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("1.1.2"), std::string_view("1.1.4"), std::string_view("1.2.1")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow2, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 4> _slice_ref_tmp = {std::string_view("0.9.1"), std::string_view("2.9.0"), std::string_view("1.1.1"), std::string_view("0.0.1")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow2, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("1.1.2-alpha1"), std::string_view("1.1.3-alpha1"), std::string_view("2.9.0-alpha1")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow3 = util::req(std::string_view("^0.1.2"));
    ::assert_match_all(r_shadow3, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 2> _slice_ref_tmp = {std::string_view("0.1.2"), std::string_view("0.1.4")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow3, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 4> _slice_ref_tmp = {std::string_view("0.9.1"), std::string_view("2.9.0"), std::string_view("1.1.1"), std::string_view("0.0.1")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow3, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("0.1.2-beta"), std::string_view("0.1.3-alpha"), std::string_view("0.2.0-pre")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow4 = util::req(std::string_view("^0.5.1-alpha3"));
    ::assert_match_all(r_shadow4, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 5> _slice_ref_tmp = {std::string_view("0.5.1-alpha3"), std::string_view("0.5.1-alpha4"), std::string_view("0.5.1-beta"), std::string_view("0.5.1"), std::string_view("0.5.5")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow4, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 5> _slice_ref_tmp = {std::string_view("0.5.1-alpha1"), std::string_view("0.5.2-alpha3"), std::string_view("0.5.5-pre"), std::string_view("0.5.0-pre"), std::string_view("0.6.0")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow5 = util::req(std::string_view("^0.0.2"));
    ::assert_match_all(r_shadow5, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 1> _slice_ref_tmp = {std::string_view("0.0.2")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow5, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 5> _slice_ref_tmp = {std::string_view("0.9.1"), std::string_view("2.9.0"), std::string_view("1.1.1"), std::string_view("0.0.1"), std::string_view("0.1.4")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow6 = util::req(std::string_view("^0.0"));
    ::assert_match_all(r_shadow6, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 2> _slice_ref_tmp = {std::string_view("0.0.2"), std::string_view("0.0.0")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow6, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 4> _slice_ref_tmp = {std::string_view("0.9.1"), std::string_view("2.9.0"), std::string_view("1.1.1"), std::string_view("0.1.4")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow7 = util::req(std::string_view("^0"));
    ::assert_match_all(r_shadow7, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("0.9.1"), std::string_view("0.0.2"), std::string_view("0.0.0")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow7, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 2> _slice_ref_tmp = {std::string_view("2.9.0"), std::string_view("1.1.1")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow8 = util::req(std::string_view("^1.4.2-beta.5"));
    ::assert_match_all(r_shadow8, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 5> _slice_ref_tmp = {std::string_view("1.4.2"), std::string_view("1.4.3"), std::string_view("1.4.2-beta.5"), std::string_view("1.4.2-beta.6"), std::string_view("1.4.2-c")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow8, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 5> _slice_ref_tmp = {std::string_view("0.9.9"), std::string_view("2.0.0"), std::string_view("1.4.2-alpha"), std::string_view("1.4.2-beta.4"), std::string_view("1.4.3-beta.5")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
}

void test_wildcard() {
    const auto err = util::req_err(std::string_view(""));
    util::assert_to_string(std::move(err), std::string_view("unexpected end of input while parsing major version number"));
    const auto& r = util::req(std::string_view("*"));
    ::assert_match_all(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 5> _slice_ref_tmp = {std::string_view("0.9.1"), std::string_view("2.9.0"), std::string_view("0.0.9"), std::string_view("1.0.1"), std::string_view("1.1.1")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 1> _slice_ref_tmp = {std::string_view("1.0.0-pre")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    for (auto&& s : rusty::for_in(rusty::iter(std::array{"x", "X"}))) {
        {
            auto _m0 = &r;
            auto&& _m1_tmp = util::req(rusty::to_string_view(s));
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
    const auto& r_shadow1 = util::req(std::string_view("1.*"));
    ::assert_match_all(r_shadow1, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 4> _slice_ref_tmp = {std::string_view("1.2.0"), std::string_view("1.2.1"), std::string_view("1.1.1"), std::string_view("1.3.0")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow1, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 2> _slice_ref_tmp = {std::string_view("0.0.9"), std::string_view("1.2.0-pre")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    for (auto&& s : rusty::for_in(rusty::iter(std::array{"1.x", "1.X", "1.*.*"}))) {
        {
            auto _m0 = &r_shadow1;
            auto&& _m1_tmp = util::req(rusty::to_string_view(s));
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
    const auto& r_shadow2 = util::req(std::string_view("1.2.*"));
    ::assert_match_all(r_shadow2, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("1.2.0"), std::string_view("1.2.2"), std::string_view("1.2.4")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    ::assert_match_none(r_shadow2, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 5> _slice_ref_tmp = {std::string_view("1.9.0"), std::string_view("1.0.9"), std::string_view("2.0.1"), std::string_view("0.1.3"), std::string_view("1.2.2-pre")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    for (auto&& s : rusty::for_in(rusty::iter(std::array{"1.2.x", "1.2.X"}))) {
        {
            auto _m0 = &r_shadow2;
            auto&& _m1_tmp = util::req(rusty::to_string_view(s));
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

void test_logical_or() {
    const auto err = util::req_err(std::string_view("=1.2.3 || =2.3.4"));
    util::assert_to_string(std::move(err), std::string_view("expected comma after patch version number, found '|'"));
    const auto err_shadow1 = util::req_err(std::string_view("1.1 || =1.2.3"));
    util::assert_to_string(std::move(err_shadow1), std::string_view("expected comma after minor version number, found '|'"));
    const auto err_shadow2 = util::req_err(std::string_view("6.* || 8.* || >= 10.*"));
    util::assert_to_string(std::move(err_shadow2), std::string_view("expected comma after minor version number, found '|'"));
}

void test_any() {
    const auto& r = rusty::clone(VersionReq::STAR);
    ::assert_match_all(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 3> _slice_ref_tmp = {std::string_view("0.0.1"), std::string_view("0.1.0"), std::string_view("1.0.0")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
}

void test_pre() {
    const auto& r = util::req(std::string_view("=2.1.1-really.0"));
    ::assert_match_all(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 1> _slice_ref_tmp = {std::string_view("2.1.1-really.0")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
}


void test_comparator_parse() {
    const auto parsed = util::comparator(std::string_view("1.2.3-alpha"));
    util::assert_to_string(std::move(parsed), std::string_view("^1.2.3-alpha"));
    const auto parsed_shadow1 = util::comparator(std::string_view("2.X"));
    util::assert_to_string(std::move(parsed_shadow1), std::string_view("2.*"));
    const auto parsed_shadow2 = util::comparator(std::string_view("2"));
    util::assert_to_string(std::move(parsed_shadow2), std::string_view("^2"));
    const auto parsed_shadow3 = util::comparator(std::string_view("2.x.x"));
    util::assert_to_string(std::move(parsed_shadow3), std::string_view("2.*"));
    const auto err = util::comparator_err(std::string_view("1.2.3-01"));
    util::assert_to_string(std::move(err), std::string_view("invalid leading zero in pre-release identifier"));
    const auto err_shadow1 = util::comparator_err(std::string_view("1.2.3+4."));
    util::assert_to_string(std::move(err_shadow1), std::string_view("empty identifier segment in build metadata"));
    const auto err_shadow2 = util::comparator_err(std::string_view(">"));
    util::assert_to_string(std::move(err_shadow2), std::string_view("unexpected end of input while parsing major version number"));
    const auto err_shadow3 = util::comparator_err(std::string_view("1."));
    util::assert_to_string(std::move(err_shadow3), std::string_view("unexpected end of input while parsing minor version number"));
    const auto err_shadow4 = util::comparator_err(std::string_view("1.*."));
    util::assert_to_string(std::move(err_shadow4), std::string_view("unexpected character after wildcard in version req"));
    const auto err_shadow5 = util::comparator_err(std::string_view("1.2.3+4ÿ"));
    util::assert_to_string(std::move(err_shadow5), std::string_view("unexpected character 'ÿ' after build metadata"));
}

void test_cargo3202() {
    const auto& r = util::req(std::string_view("0.*.*"));
    util::assert_to_string(r, std::string_view("0.*"));
    ::assert_match_all(r, [&]() -> std::span<const std::string_view> { static const std::array<std::string_view, 1> _slice_ref_tmp = {std::string_view("0.5.0")}; return std::span<const std::string_view>(_slice_ref_tmp); }());
    const auto& r_shadow1 = util::req(std::string_view("0.0.*"));
    util::assert_to_string(r_shadow1, std::string_view("0.0.*"));
}

void test_digit_after_wildcard() {
    const auto err = util::req_err(std::string_view("*.1"));
    util::assert_to_string(std::move(err), std::string_view("unexpected character after wildcard in version req"));
    const auto err_shadow1 = util::req_err(std::string_view("1.*.1"));
    util::assert_to_string(std::move(err_shadow1), std::string_view("unexpected character after wildcard in version req"));
    const auto err_shadow2 = util::req_err(std::string_view(">=1.*.1"));
    util::assert_to_string(std::move(err_shadow2), std::string_view("unexpected character after wildcard in version req"));
}

void test_eq_hash() {
    const auto calculate_hash = [](const auto& value) -> uint64_t {
        auto hasher = DefaultHasher::new_();
        rusty::hash::hash(value, hasher);
        return hasher.finish();
    };
    if (!(util::req(std::string_view("^1")) == util::req(std::string_view("^1")))) {
        rusty::panicking::panic("assertion failed: req(\"^1\") == req(\"^1\")");
    }
    if (!(calculate_hash(util::req(std::string_view("^1"))) == calculate_hash(util::req(std::string_view("^1"))))) {
        rusty::panicking::panic("assertion failed: calculate_hash(req(\"^1\")) == calculate_hash(req(\"^1\"))");
    }
    if (!(util::req(std::string_view("^1")) != util::req(std::string_view("^2")))) {
        rusty::panicking::panic("assertion failed: req(\"^1\") != req(\"^2\")");
    }
}

void test_leading_digit_in_pre_and_build() {
    for (auto&& op : rusty::for_in(rusty::iter(std::array{"=", ">", ">=", "<", "<=", "~", "^"}))) {
        util::req(rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0} 1.2.3-1a", rusty::to_string(op)))));
        util::req(rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0} 1.2.3+1a", rusty::to_string(op)))));
        util::req(rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0} 1.2.3-01a", rusty::to_string(op)))));
        util::req(rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0} 1.2.3+01", rusty::to_string(op)))));
        util::req(rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0} 1.2.3-1+1", rusty::to_string(op)))));
        util::req(rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0} 1.2.3-1-1+1-1-1", rusty::to_string(op)))));
        util::req(rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0} 1.2.3-1a+1a", rusty::to_string(op)))));
        util::req(rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0} 1.2.3-1a-1a+1a-1a-1a", rusty::to_string(op)))));
    }
}

void test_wildcard_and_another() {
    const auto err = util::req_err(std::string_view("*, 0.20.0-any"));
    util::assert_to_string(std::move(err), std::string_view("wildcard req (*) must be the only comparator in the version req"));
    const auto err_shadow1 = util::req_err(std::string_view("0.20.0-any, *"));
    util::assert_to_string(std::move(err_shadow1), std::string_view("wildcard req (*) must be the only comparator in the version req"));
    const auto err_shadow2 = util::req_err(std::string_view("0.20.0-any, *, 1.0"));
    util::assert_to_string(std::move(err_shadow2), std::string_view("wildcard req (*) must be the only comparator in the version req"));
}

// Rust-only libtest main omitted


// Runnable wrappers for expanded Rust test bodies
// Rust-only libtest wrapper metadata: marker=test_basic should_panic=no
void rusty_test_test_basic() {
    test_basic();
}
// Rust-only libtest wrapper metadata: marker=test_default should_panic=no
void rusty_test_test_default() {
    test_default();
}
// Rust-only libtest wrapper metadata: marker=test_exact should_panic=no
void rusty_test_test_exact() {
    test_exact();
}
// Rust-only libtest wrapper metadata: marker=test_greater_than should_panic=no
void rusty_test_test_greater_than() {
    test_greater_than();
}
// Rust-only libtest wrapper metadata: marker=test_less_than should_panic=no
void rusty_test_test_less_than() {
    test_less_than();
}
// Rust-only libtest wrapper metadata: marker=test_multiple should_panic=no
void rusty_test_test_multiple() {
    test_multiple();
}
// Rust-only libtest wrapper metadata: marker=test_whitespace_delimited_comparator_sets should_panic=no
void rusty_test_test_whitespace_delimited_comparator_sets() {
    test_whitespace_delimited_comparator_sets();
}
// Rust-only libtest wrapper metadata: marker=test_tilde should_panic=no
void rusty_test_test_tilde() {
    test_tilde();
}
// Rust-only libtest wrapper metadata: marker=test_caret should_panic=no
void rusty_test_test_caret() {
    test_caret();
}
// Rust-only libtest wrapper metadata: marker=test_wildcard should_panic=no
void rusty_test_test_wildcard() {
    test_wildcard();
}
// Rust-only libtest wrapper metadata: marker=test_logical_or should_panic=no
void rusty_test_test_logical_or() {
    test_logical_or();
}
// Rust-only libtest wrapper metadata: marker=test_any should_panic=no
void rusty_test_test_any() {
    test_any();
}
// Rust-only libtest wrapper metadata: marker=test_pre should_panic=no
void rusty_test_test_pre() {
    test_pre();
}
// Rust-only libtest wrapper metadata: marker=test_parse should_panic=no
// Rust-only libtest wrapper metadata: marker=test_comparator_parse should_panic=no
void rusty_test_test_comparator_parse() {
    test_comparator_parse();
}
// Rust-only libtest wrapper metadata: marker=test_cargo3202 should_panic=no
void rusty_test_test_cargo3202() {
    test_cargo3202();
}
// Rust-only libtest wrapper metadata: marker=test_digit_after_wildcard should_panic=no
void rusty_test_test_digit_after_wildcard() {
    test_digit_after_wildcard();
}
// Rust-only libtest wrapper metadata: marker=test_eq_hash should_panic=no
void rusty_test_test_eq_hash() {
    test_eq_hash();
}
// Rust-only libtest wrapper metadata: marker=test_leading_digit_in_pre_and_build should_panic=no
void rusty_test_test_leading_digit_in_pre_and_build() {
    test_leading_digit_in_pre_and_build();
}
// Rust-only libtest wrapper metadata: marker=test_wildcard_and_another should_panic=no
void rusty_test_test_wildcard_and_another() {
    test_wildcard_and_another();
}


// ── Test runner ──
int main(int argc, char** argv) {
    if (argc == 3 && std::string(argv[1]) == "--rusty-single-test") {
        const std::string test_name = argv[2];
        rusty::mem::clear_all_forgotten_addresses();
        try {
            if (test_name == "rusty_test_test") { rusty_test_test(); return 0; }
            if (test_name == "rusty_test_test_align") { rusty_test_test_align(); return 0; }
            if (test_name == "rusty_test_test_any") { rusty_test_test_any(); return 0; }
            if (test_name == "rusty_test_test_basic") { rusty_test_test_basic(); return 0; }
            if (test_name == "rusty_test_test_caret") { rusty_test_test_caret(); return 0; }
            if (test_name == "rusty_test_test_cargo3202") { rusty_test_test_cargo3202(); return 0; }
            if (test_name == "rusty_test_test_comparator_parse") { rusty_test_test_comparator_parse(); return 0; }
            if (test_name == "rusty_test_test_default") { rusty_test_test_default(); return 0; }
            if (test_name == "rusty_test_test_digit_after_wildcard") { rusty_test_test_digit_after_wildcard(); return 0; }
            if (test_name == "rusty_test_test_display") { rusty_test_test_display(); return 0; }
            if (test_name == "rusty_test_test_eq") { rusty_test_test_eq(); return 0; }
            if (test_name == "rusty_test_test_eq_hash") { rusty_test_test_eq_hash(); return 0; }
            if (test_name == "rusty_test_test_exact") { rusty_test_test_exact(); return 0; }
            if (test_name == "rusty_test_test_ge") { rusty_test_test_ge(); return 0; }
            if (test_name == "rusty_test_test_greater_than") { rusty_test_test_greater_than(); return 0; }
            if (test_name == "rusty_test_test_gt") { rusty_test_test_gt(); return 0; }
            if (test_name == "rusty_test_test_le") { rusty_test_test_le(); return 0; }
            if (test_name == "rusty_test_test_leading_digit_in_pre_and_build") { rusty_test_test_leading_digit_in_pre_and_build(); return 0; }
            if (test_name == "rusty_test_test_less_than") { rusty_test_test_less_than(); return 0; }
            if (test_name == "rusty_test_test_logical_or") { rusty_test_test_logical_or(); return 0; }
            if (test_name == "rusty_test_test_lt") { rusty_test_test_lt(); return 0; }
            if (test_name == "rusty_test_test_multiple") { rusty_test_test_multiple(); return 0; }
            if (test_name == "rusty_test_test_ne") { rusty_test_test_ne(); return 0; }
            if (test_name == "rusty_test_test_new") { rusty_test_test_new(); return 0; }
            if (test_name == "rusty_test_test_parse") { rusty_test_test_parse(); return 0; }
            if (test_name == "rusty_test_test_pre") { rusty_test_test_pre(); return 0; }
            if (test_name == "rusty_test_test_prerelease") { rusty_test_test_prerelease(); return 0; }
            if (test_name == "rusty_test_test_spec_order") { rusty_test_test_spec_order(); return 0; }
            if (test_name == "rusty_test_test_tilde") { rusty_test_test_tilde(); return 0; }
            if (test_name == "rusty_test_test_whitespace_delimited_comparator_sets") { rusty_test_test_whitespace_delimited_comparator_sets(); return 0; }
            if (test_name == "rusty_test_test_wildcard") { rusty_test_test_wildcard(); return 0; }
            if (test_name == "rusty_test_test_wildcard_and_another") { rusty_test_test_wildcard_and_another(); return 0; }
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
    try { rusty_test_test(); std::cout << "  test PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_align(); std::cout << "  test_align PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_align FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_align FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_any(); std::cout << "  test_any PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_any FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_any FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_basic(); std::cout << "  test_basic PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_basic FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_basic FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_caret(); std::cout << "  test_caret PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_caret FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_caret FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_cargo3202(); std::cout << "  test_cargo3202 PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_cargo3202 FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_cargo3202 FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_comparator_parse(); std::cout << "  test_comparator_parse PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_comparator_parse FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_comparator_parse FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_default(); std::cout << "  test_default PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_default FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_default FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_digit_after_wildcard(); std::cout << "  test_digit_after_wildcard PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_digit_after_wildcard FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_digit_after_wildcard FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_display(); std::cout << "  test_display PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_display FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_display FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_eq(); std::cout << "  test_eq PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_eq FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_eq FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_eq_hash(); std::cout << "  test_eq_hash PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_eq_hash FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_eq_hash FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_exact(); std::cout << "  test_exact PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_exact FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_exact FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_ge(); std::cout << "  test_ge PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_ge FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_ge FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_greater_than(); std::cout << "  test_greater_than PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_greater_than FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_greater_than FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_gt(); std::cout << "  test_gt PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_gt FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_gt FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_le(); std::cout << "  test_le PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_le FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_le FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_leading_digit_in_pre_and_build(); std::cout << "  test_leading_digit_in_pre_and_build PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_leading_digit_in_pre_and_build FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_leading_digit_in_pre_and_build FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_less_than(); std::cout << "  test_less_than PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_less_than FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_less_than FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_logical_or(); std::cout << "  test_logical_or PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_logical_or FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_logical_or FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_lt(); std::cout << "  test_lt PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_lt FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_lt FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_multiple(); std::cout << "  test_multiple PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_multiple FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_multiple FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_ne(); std::cout << "  test_ne PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_ne FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_ne FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_new(); std::cout << "  test_new PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_new FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_new FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_parse(); std::cout << "  test_parse PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_parse FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_parse FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_pre(); std::cout << "  test_pre PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_pre FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_pre FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_prerelease(); std::cout << "  test_prerelease PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_prerelease FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_prerelease FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_spec_order(); std::cout << "  test_spec_order PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_spec_order FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_spec_order FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_tilde(); std::cout << "  test_tilde PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_tilde FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_tilde FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_whitespace_delimited_comparator_sets(); std::cout << "  test_whitespace_delimited_comparator_sets PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_whitespace_delimited_comparator_sets FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_whitespace_delimited_comparator_sets FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_wildcard(); std::cout << "  test_wildcard PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_wildcard FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_wildcard FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_wildcard_and_another(); std::cout << "  test_wildcard_and_another PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_wildcard_and_another FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_wildcard_and_another FAILED (unknown exception)" << std::endl; fail++; }
    std::cout << std::endl;
    std::cout << "Results: " << pass << " passed, " << fail << " failed" << std::endl;
    return fail > 0 ? 1 : 0;
}
