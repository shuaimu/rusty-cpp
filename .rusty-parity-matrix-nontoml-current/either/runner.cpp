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
namespace iterator { template<typename L, typename R> struct IterEither; }
template<typename L, typename R> struct Either;

// ── from either.cppm ──



#if defined(__GNUC__)
#pragma GCC diagnostic ignored "-Wunused-local-typedefs"
#endif


namespace into_either {}
namespace iterator {}


namespace iterator {
template<typename L, typename R>
struct IterEither;
}

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










template<typename L, typename R>
struct Either;
namespace iterator {
    template<typename L, typename R>
    struct IterEither;
}
namespace into_either {
}
using iterator::IterEither;
void basic();
void macros();
void deref();
void iter();
void seek();
void read_write();
void error();
void _unsized_ref_propagation();
void _unsized_std_propagation();

namespace into_either {
}



namespace fmt = rusty::fmt;






namespace io = rusty::io;
using ::rusty::io::SeekFrom;


// Algebraic data type
template<typename L, typename R>
struct Either_Left {
    L _0;
};
template<typename L, typename R>
struct Either_Right {
    R _0;
};
template<typename L, typename R>
Either_Left<L, R> Left(L _0);
template<typename L, typename R>
Either_Right<L, R> Right(R _0);
template<typename L, typename R>
struct Either : std::variant<Either_Left<L, R>, Either_Right<L, R>> {
    using variant = std::variant<Either_Left<L, R>, Either_Right<L, R>>;
    using variant::variant;
    static Either<L, R> Left(L _0) { return Either<L, R>{Either_Left<L, R>{std::forward<decltype(_0)>(_0)}}; }
    static Either<L, R> Right(R _0) { return Either<L, R>{Either_Right<L, R>{std::forward<decltype(_0)>(_0)}}; }


    bool operator==(const Either<L, R>& other) const {
        const auto __self_discr = rusty::intrinsics::discriminant_value((*this));
        const auto __arg1_discr = rusty::intrinsics::discriminant_value(other);
        return (__self_discr == __arg1_discr) && [&]() -> bool { auto&& _m0 = (*this); auto&& _m1 = other; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0))._0); auto&& __arg1_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))._0); return __self_0 == __arg1_0; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0))._0); auto&& __arg1_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))._0); return __self_0 == __arg1_0; } if (true) { return [&]() -> bool { rusty::intrinsics::unreachable(); }(); } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
    }
    void assert_receiver_is_total_eq() const {
    }
    std::partial_ordering operator<=>(const Either<L, R>& other) const {
        return rusty::to_partial_ordering([&]() -> rusty::Option<rusty::cmp::Ordering> {
            const auto __self_discr = rusty::intrinsics::discriminant_value((*this));
            const auto __arg1_discr = rusty::intrinsics::discriminant_value(other);
            return [&]() -> rusty::Option<rusty::cmp::Ordering> { auto&& _m0 = (*this); auto&& _m1 = other; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0))._0); auto&& __arg1_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))._0); return rusty::partial_cmp(__self_0, __arg1_0); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0))._0); auto&& __arg1_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))._0); return rusty::partial_cmp(__self_0, __arg1_0); } if (true) { return rusty::partial_cmp(__self_discr, __arg1_discr); } return [&]() -> rusty::Option<rusty::cmp::Ordering> { rusty::intrinsics::unreachable(); }(); }();
        }());
    }
    rusty::cmp::Ordering cmp(const Either<L, R>& other) const {
        const auto __self_discr = rusty::intrinsics::discriminant_value((*this));
        const auto __arg1_discr = rusty::intrinsics::discriminant_value(other);
        return [&]() { auto&& _m = rusty::cmp::cmp(__self_discr, __arg1_discr); if (_m == ::core::cmp::Ordering::Equal) return [&]() -> rusty::cmp::Ordering { auto&& _m0 = (*this); auto&& _m1 = other; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0))._0); auto&& __arg1_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))._0); return rusty::cmp::cmp(__self_0, __arg1_0); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0))._0); auto&& __arg1_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))._0); return rusty::cmp::cmp(__self_0, __arg1_0); } if (true) { return [&]() -> rusty::cmp::Ordering { rusty::intrinsics::unreachable(); }(); } return [&]() -> rusty::cmp::Ordering { rusty::intrinsics::unreachable(); }(); }();
{ const auto& cmp = _m; return cmp;  } }();
    }
    template<typename __H>
    void hash(__H& state) const {
        const auto __self_discr = rusty::intrinsics::discriminant_value((*this));
        rusty::hash::hash(__self_discr, state);
        {
            auto&& _m = (*this);
            std::visit(overloaded {
                [&](const Either_Left<L, R>& _v) {
                    auto&& __self_0 = rusty::detail::deref_if_pointer(_v._0);
                    rusty::hash::hash(__self_0, state);
                },
                [&](const Either_Right<L, R>& _v) {
                    auto&& __self_0 = rusty::detail::deref_if_pointer(_v._0);
                    rusty::hash::hash(__self_0, state);
                },
            }, _m);
        }
    }
    rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
        return [&]() -> rusty::fmt::Result { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.fmt(f); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.fmt(f); } return [&]() -> rusty::fmt::Result { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename T, typename A>
    void extend(T iter) {
        {
            auto&& _m = (*this);
            std::visit(overloaded {
                [&](::Either_Left<L, R>& _v) {
                    auto& inner = _v._0;
                    inner.extend(std::move(iter));
                },
                [&](::Either_Right<L, R>& _v) {
                    auto& inner = _v._0;
                    inner.extend(std::move(iter));
                },
            }, _m);
        }
    }
    // Rust-only dependent associated type alias skipped in constrained mode: Item
    auto next() {
        return [&]() { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.next(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.next(); } rusty::intrinsics::unreachable(); }();
    }
    std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
        return [&]() -> std::tuple<size_t, rusty::Option<size_t>> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.size_hint(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.size_hint(); } return [&]() -> std::tuple<size_t, rusty::Option<size_t>> { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename Acc, typename G>
    Acc fold(Acc init, G f) {
        return [&]() -> Acc { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::fold(inner, std::move(init), std::move(f)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::fold(inner, std::move(init), std::move(f)); } return [&]() -> Acc { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename F>
    void for_each(F f) {
        {
            auto&& _m = (*this);
            std::visit(overloaded {
                [&](const ::Either_Left<L, R>& _v) {
                    auto&& inner = rusty::detail::deref_if_pointer(_v._0);
                    inner.for_each(std::move(f));
                },
                [&](const ::Either_Right<L, R>& _v) {
                    auto&& inner = rusty::detail::deref_if_pointer(_v._0);
                    inner.for_each(std::move(f));
                },
            }, _m);
        }
    }
    size_t count() {
        return [&]() -> size_t { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.count(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.count(); } return [&]() -> size_t { rusty::intrinsics::unreachable(); }(); }();
    }
    auto last() {
        return [&]() { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.last(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.last(); } rusty::intrinsics::unreachable(); }();
    }
    auto nth(size_t n) {
        return [&]() { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.nth(std::move(n)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.nth(std::move(n)); } rusty::intrinsics::unreachable(); }();
    }
    template<typename B>
    B collect() {
        return [&]() -> B { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return B::from_iter(inner); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return B::from_iter(inner); } return [&]() -> B { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename B, typename F>
    std::tuple<B, B> partition(F f) {
        return [&]() -> std::tuple<B, B> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.partition(std::move(f)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.partition(std::move(f)); } return [&]() -> std::tuple<B, B> { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename F>
    bool all(F f) {
        return [&]() -> bool { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.all(std::move(f)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.all(std::move(f)); } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename F>
    bool any(F f) {
        return [&]() -> bool { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.any(std::move(f)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.any(std::move(f)); } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename P>
    auto find(P predicate) {
        return [&]() { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::str_runtime::find(inner, std::move(predicate)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::str_runtime::find(inner, std::move(predicate)); } rusty::intrinsics::unreachable(); }();
    }
    template<typename B, typename F>
    rusty::Option<B> find_map(F f) {
        return [&]() -> rusty::Option<B> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.find_map(std::move(f)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.find_map(std::move(f)); } return [&]() -> rusty::Option<B> { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename P>
    rusty::Option<size_t> position(P predicate) {
        return [&]() -> rusty::Option<size_t> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.position(std::move(predicate)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.position(std::move(predicate)); } return [&]() -> rusty::Option<size_t> { rusty::intrinsics::unreachable(); }(); }();
    }
    auto next_back() {
        return [&]() { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.next_back(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.next_back(); } rusty::intrinsics::unreachable(); }();
    }
    auto nth_back(size_t n) {
        return [&]() { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.nth_back(std::move(n)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.nth_back(std::move(n)); } rusty::intrinsics::unreachable(); }();
    }
    template<typename Acc, typename G>
    Acc rfold(Acc init, G f) {
        return [&]() -> Acc { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.rfold(std::move(init), std::move(f)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.rfold(std::move(init), std::move(f)); } return [&]() -> Acc { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename P>
    auto rfind(P predicate) {
        return [&]() { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.rfind(std::move(predicate)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.rfind(std::move(predicate)); } rusty::intrinsics::unreachable(); }();
    }
    size_t len() const {
        return [&]() -> size_t { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::len(inner); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::len(inner); } return [&]() -> size_t { rusty::intrinsics::unreachable(); }(); }();
    }
    Either<L, R> clone() const {
        return [&]() -> Either<L, R> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<L, R>(Either<L, R>{Either_Left<L, R>{rusty::clone(inner)}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<L, R>(Either<L, R>{Either_Right<L, R>{rusty::clone(inner)}}); } return [&]() -> Either<L, R> { rusty::intrinsics::unreachable(); }(); }();
    }
    void clone_from(const Either<L, R>& source) {
        {
            auto&& _m0 = (*this);
            auto&& _m1 = source;
            auto _m_tuple = std::forward_as_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched && ((std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple))))>>>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)))) && std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple))))>>>(rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple))))))) {
                auto&& dest = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple))))>>>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple))))._0);
                auto&& source = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple))))>>>(rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple))))._0);
                dest.clone_from(source);
                _m_matched = true;
            }
            if (!_m_matched && ((std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple))))>>>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)))) && std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple))))>>>(rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple))))))) {
                auto&& dest = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple))))>>>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple))))._0);
                auto&& source = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple))))>>>(rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple))))._0);
                dest.clone_from(source);
                _m_matched = true;
            }
            if (!_m_matched && (true)) {
                auto&& dest = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                auto&& source = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                rusty::detail::deref_if_pointer_like(dest) = rusty::clone(source);
                _m_matched = true;
            }
        }
    }
    bool is_left() const {
        return [&]() -> bool { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return true; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return false; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
    }
    bool is_right() const {
        return !this->is_left();
    }
    rusty::Option<L> left() {
        return [&]() -> rusty::Option<L> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::Option<L>(l); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return rusty::Option<L>(rusty::None); } return [&]() -> rusty::Option<L> { rusty::intrinsics::unreachable(); }(); }();
    }
    rusty::Option<R> right() {
        return [&]() -> rusty::Option<R> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return rusty::Option<R>(rusty::None); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::Option<R>(r); } return [&]() -> rusty::Option<R> { rusty::intrinsics::unreachable(); }(); }();
    }
    Either<const L&, const R&> as_ref() const {
        return [&]() -> Either<const L&, const R&> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return Either<const L&, const R&>(Either<const L&, const R&>{Either_Left<const L&, const R&>{inner}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return Either<const L&, const R&>(Either<const L&, const R&>{Either_Right<const L&, const R&>{inner}}); } return [&]() -> Either<const L&, const R&> { rusty::intrinsics::unreachable(); }(); }();
    }
    Either<L&, R&> as_mut() {
        return [&]() -> Either<L&, R&> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return Either<L&, R&>(Either<L&, R&>{Either_Left<L&, R&>{inner}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return Either<L&, R&>(Either<L&, R&>{Either_Right<L&, R&>{inner}}); } return [&]() -> Either<L&, R&> { rusty::intrinsics::unreachable(); }(); }();
    }
    Either<rusty::pin::Pin<const L&>, rusty::pin::Pin<const R&>> as_pin_ref() {
        // @unsafe
        {
            return [&]() -> Either<rusty::pin::Pin<const L&>, rusty::pin::Pin<const R&>> { auto&& _m = rusty::detail::deref_if_pointer_like(rusty::pin::get_ref(std::move((*this)))); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return Either<rusty::pin::Pin<const L&>, rusty::pin::Pin<const R&>>(Either<rusty::pin::Pin<const L&>, rusty::pin::Pin<const R&>>{Either_Left<rusty::pin::Pin<const L&>, rusty::pin::Pin<const R&>>{rusty::pin::new_unchecked(inner)}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return Either<rusty::pin::Pin<const L&>, rusty::pin::Pin<const R&>>(Either<rusty::pin::Pin<const L&>, rusty::pin::Pin<const R&>>{Either_Right<rusty::pin::Pin<const L&>, rusty::pin::Pin<const R&>>{rusty::pin::new_unchecked(inner)}}); } return [&]() -> Either<rusty::pin::Pin<const L&>, rusty::pin::Pin<const R&>> { rusty::intrinsics::unreachable(); }(); }();
        }
    }
    Either<rusty::pin::Pin<L&>, rusty::pin::Pin<R&>> as_pin_mut() {
        // @unsafe
        {
            return [&]() -> Either<rusty::pin::Pin<L&>, rusty::pin::Pin<R&>> { auto&& _m = rusty::detail::deref_if_pointer_like(rusty::pin::get_unchecked_mut(std::move((*this)))); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return Either<rusty::pin::Pin<L&>, rusty::pin::Pin<R&>>(Either<rusty::pin::Pin<L&>, rusty::pin::Pin<R&>>{Either_Left<rusty::pin::Pin<L&>, rusty::pin::Pin<R&>>{rusty::pin::new_unchecked(inner)}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return Either<rusty::pin::Pin<L&>, rusty::pin::Pin<R&>>(Either<rusty::pin::Pin<L&>, rusty::pin::Pin<R&>>{Either_Right<rusty::pin::Pin<L&>, rusty::pin::Pin<R&>>{rusty::pin::new_unchecked(inner)}}); } return [&]() -> Either<rusty::pin::Pin<L&>, rusty::pin::Pin<R&>> { rusty::intrinsics::unreachable(); }(); }();
        }
    }
    Either<R, L> flip() {
        return [&]() -> Either<R, L> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<R, L>(Either<R, L>{Either_Right<R, L>{l}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<R, L>(Either<R, L>{Either_Left<R, L>{r}}); } return [&]() -> Either<R, L> { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename F>
    auto map_left(F f) {
        using M = std::remove_cvref_t<std::invoke_result_t<F&, L>>;
        return [&]() -> Either<M, R> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<M, R>(Either<M, R>{Either_Left<M, R>{f(l)}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<M, R>(Either<M, R>{Either_Right<M, R>{r}}); } return [&]() -> Either<M, R> { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename F>
    auto map_right(F f) {
        using S = std::remove_cvref_t<std::invoke_result_t<F&, R>>;
        return [&]() -> Either<L, S> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<L, S>(Either<L, S>{Either_Left<L, S>{l}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<L, S>(Either<L, S>{Either_Right<L, S>{f(r)}}); } return [&]() -> Either<L, S> { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename F, typename G>
    auto map_either(F f, G g) {
        using M = std::remove_cvref_t<std::invoke_result_t<F&, L>>;
        using S = std::remove_cvref_t<std::invoke_result_t<G&, R>>;
        return [&]() -> Either<M, S> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<M, S>(Either<M, S>{Either_Left<M, S>{f(l)}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<M, S>(Either<M, S>{Either_Right<M, S>{g(r)}}); } return [&]() -> Either<M, S> { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename Ctx, typename F, typename G>
    auto map_either_with(Ctx ctx, F f, G g) {
        using M = std::remove_cvref_t<std::invoke_result_t<F&, Ctx, L>>;
        using S = std::remove_cvref_t<std::invoke_result_t<G&, Ctx, R>>;
        return [&]() -> Either<M, S> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<M, S>(Either<M, S>{Either_Left<M, S>{f(std::move(ctx), l)}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<M, S>(Either<M, S>{Either_Right<M, S>{g(std::move(ctx), r)}}); } return [&]() -> Either<M, S> { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename F, typename G>
    auto either(F f, G g) {
        using T = std::remove_cvref_t<std::invoke_result_t<F&, L>>;
        return [&]() -> T { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return f(l); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return g(r); } return [&]() -> T { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename Ctx, typename F, typename G>
    auto either_with(Ctx ctx, F f, G g) {
        using T = std::remove_cvref_t<std::invoke_result_t<F&, Ctx, L>>;
        return [&]() -> T { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return f(std::move(ctx), l); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return g(std::move(ctx), r); } return [&]() -> T { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename F, typename S>
    Either<S, R> left_and_then(F f) {
        return [&]() -> Either<S, R> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return f(l); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<S, R>(Either<S, R>{Either_Right<S, R>{r}}); } return [&]() -> Either<S, R> { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename F, typename S>
    Either<L, S> right_and_then(F f) {
        return [&]() -> Either<L, S> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<L, S>(Either<L, S>{Either_Left<L, S>{l}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return f(r); } return [&]() -> Either<L, S> { rusty::intrinsics::unreachable(); }(); }();
    }
    auto into_iter() {
        return [&]() -> Either<typename L::IntoIter, typename R::IntoIter> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<typename L::IntoIter, typename R::IntoIter>(Either<typename L::IntoIter, typename R::IntoIter>{Either_Left<typename L::IntoIter, typename R::IntoIter>{rusty::iter(inner)}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<typename L::IntoIter, typename R::IntoIter>(Either<typename L::IntoIter, typename R::IntoIter>{Either_Right<typename L::IntoIter, typename R::IntoIter>{rusty::iter(inner)}}); } return [&]() -> Either<typename L::IntoIter, typename R::IntoIter> { rusty::intrinsics::unreachable(); }(); }();
    }
    auto iter() const {
        return [&]() -> Either<typename L::IntoIter, typename R::IntoIter> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<typename L::IntoIter, typename R::IntoIter>(Either<typename L::IntoIter, typename R::IntoIter>{Either_Left<typename L::IntoIter, typename R::IntoIter>{rusty::iter(inner)}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<typename L::IntoIter, typename R::IntoIter>(Either<typename L::IntoIter, typename R::IntoIter>{Either_Right<typename L::IntoIter, typename R::IntoIter>{rusty::iter(inner)}}); } return [&]() -> Either<typename L::IntoIter, typename R::IntoIter> { rusty::intrinsics::unreachable(); }(); }();
    }
    auto iter_mut() {
        return [&]() -> Either<typename L::IntoIter, typename R::IntoIter> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<typename L::IntoIter, typename R::IntoIter>(Either<typename L::IntoIter, typename R::IntoIter>{Either_Left<typename L::IntoIter, typename R::IntoIter>{rusty::iter(inner)}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<typename L::IntoIter, typename R::IntoIter>(Either<typename L::IntoIter, typename R::IntoIter>{Either_Right<typename L::IntoIter, typename R::IntoIter>{rusty::iter(inner)}}); } return [&]() -> Either<typename L::IntoIter, typename R::IntoIter> { rusty::intrinsics::unreachable(); }(); }();
    }
    auto factor_into_iter() {
        return iterator::IterEither<typename L::IntoIter, typename R::IntoIter>::new_([&]() -> typename L::IntoIter::IntoIter { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return typename L::IntoIter::IntoIter::Left(rusty::iter(inner)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return typename L::IntoIter::IntoIter::Right(rusty::iter(inner)); } return [&]() -> typename L::IntoIter::IntoIter { rusty::intrinsics::unreachable(); }(); }());
    }
    auto factor_iter() const {
        return iterator::IterEither<typename L::IntoIter, typename R::IntoIter>::new_([&]() -> typename L::IntoIter::IntoIter { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return typename L::IntoIter::IntoIter::Left(rusty::iter(inner)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return typename L::IntoIter::IntoIter::Right(rusty::iter(inner)); } return [&]() -> typename L::IntoIter::IntoIter { rusty::intrinsics::unreachable(); }(); }());
    }
    auto factor_iter_mut() {
        return iterator::IterEither<typename L::IntoIter, typename R::IntoIter>::new_([&]() -> typename L::IntoIter::IntoIter { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return typename L::IntoIter::IntoIter::Left(rusty::iter(inner)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return typename L::IntoIter::IntoIter::Right(rusty::iter(inner)); } return [&]() -> typename L::IntoIter::IntoIter { rusty::intrinsics::unreachable(); }(); }());
    }
    L left_or(L other) {
        return [&]() -> L { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return l; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return other; } return [&]() -> L { rusty::intrinsics::unreachable(); }(); }();
    }
    L left_or_default() {
        return [&]() -> L { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return l; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return L::default_(); } return [&]() -> L { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename F>
    L left_or_else(F f) {
        return [&]() -> L { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return l; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return f(r); } return [&]() -> L { rusty::intrinsics::unreachable(); }(); }();
    }
    R right_or(R other) {
        return [&]() -> R { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return other; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return r; } return [&]() -> R { rusty::intrinsics::unreachable(); }(); }();
    }
    R right_or_default() {
        return [&]() -> R { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return R::default_(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return r; } return [&]() -> R { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename F>
    R right_or_else(F f) {
        return [&]() -> R { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return f(l); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return r; } return [&]() -> R { rusty::intrinsics::unreachable(); }(); }();
    }
    L unwrap_left() {
        return [&]() -> L { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return l; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return [&]() -> L { return [&]() -> L { rusty::panicking::panic_fmt(std::format("called `Either::unwrap_left()` on a `Right` value: {0}", rusty::to_debug_string(r))); }(); }(); } return [&]() -> L { rusty::intrinsics::unreachable(); }(); }();
    }
    R unwrap_right() {
        return [&]() -> R { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return r; } if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return [&]() -> R { return [&]() -> R { rusty::panicking::panic_fmt(std::format("called `Either::unwrap_right()` on a `Left` value: {0}", rusty::to_debug_string(l))); }(); }(); } return [&]() -> R { rusty::intrinsics::unreachable(); }(); }();
    }
    L expect_left(std::string_view msg) {
        return [&]() -> L { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return l; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return [&]() -> L { return [&]() -> L { rusty::panicking::panic_fmt(std::format("{0}: {1}", rusty::to_string(msg), rusty::to_debug_string(r))); }(); }(); } return [&]() -> L { rusty::intrinsics::unreachable(); }(); }();
    }
    R expect_right(std::string_view msg) {
        return [&]() -> R { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return r; } if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return [&]() -> R { return [&]() -> R { rusty::panicking::panic_fmt(std::format("{0}: {1}", rusty::to_string(msg), rusty::to_debug_string(l))); }(); }(); } return [&]() -> R { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename T>
    T either_into() {
        return [&]() -> T { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::from_into<T>(l); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::from_into<T>(r); } return [&]() -> T { rusty::intrinsics::unreachable(); }(); }();
    }
    rusty::Option<Either<L, R>> factor_none() {
        return [&]() -> rusty::Option<Either<L, R>> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return l.map([](auto&& _v) { return Either_Left<L, R>{std::forward<decltype(_v)>(_v)}; }); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return r.map([](auto&& _v) { return Either_Right<L, R>{std::forward<decltype(_v)>(_v)}; }); } return [&]() -> rusty::Option<Either<L, R>> { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename E>
    rusty::Result<Either<L, R>, E> factor_err() {
        return [&]() -> rusty::Result<Either<L, R>, E> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return l.map([](auto&& _v) { return Either_Left<L, R>{std::forward<decltype(_v)>(_v)}; }); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return r.map([](auto&& _v) { return Either_Right<L, R>{std::forward<decltype(_v)>(_v)}; }); } return [&]() -> rusty::Result<Either<L, R>, E> { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename T>
    rusty::Result<T, Either<L, R>> factor_ok() {
        return [&]() -> rusty::Result<T, Either<L, R>> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return l.map_err([&](auto&& _err) -> Either<L, R> { return (Either<L, R>::Left) (std::forward<decltype(_err)>(_err)); }); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return r.map_err([&](auto&& _err) -> Either<L, R> { return (Either<L, R>::Right) (std::forward<decltype(_err)>(_err)); }); } return [&]() -> rusty::Result<T, Either<L, R>> { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename T>
    std::tuple<T, Either<L, R>> factor_first() {
        return [&]() -> std::tuple<T, Either<L, R>> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& t = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0))); auto&& l = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0))); return std::make_tuple(t, Either<L, R>{Either_Left<L, R>{l}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& t = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0))); auto&& r = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0))); return std::make_tuple(t, Either<L, R>{Either_Right<L, R>{r}}); } return [&]() -> std::tuple<T, Either<L, R>> { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename T>
    std::tuple<Either<L, R>, T> factor_second() {
        return [&]() -> std::tuple<Either<L, R>, T> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0))); auto&& t = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0))); return std::make_tuple(Either<L, R>{Either_Left<L, R>{l}}, t); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0))); auto&& t = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0))); return std::make_tuple(Either<L, R>{Either_Right<L, R>{r}}, t); } return [&]() -> std::tuple<Either<L, R>, T> { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename T>
    T into_inner() {
        return [&]() -> T { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner; } return [&]() -> T { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename F, typename T>
    auto map(F f) {
        using M = std::remove_cvref_t<std::invoke_result_t<F&, T>>;
        return [&]() -> Either<M, M> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<M, M>(Either<M, M>{Either_Left<M, M>{f(l)}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<M, M>(Either<M, M>{Either_Right<M, M>{f(r)}}); } return [&]() -> Either<M, M> { rusty::intrinsics::unreachable(); }(); }();
    }
    Either<L, R> cloned() {
        return [&]() -> Either<L, R> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<L, R>(Either<L, R>{Either_Left<L, R>{rusty::clone(l)}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<L, R>(Either<L, R>{Either_Right<L, R>{rusty::clone(r)}}); } return [&]() -> Either<L, R> { rusty::intrinsics::unreachable(); }(); }();
    }
    Either<L, R> copied() {
        return [&]() -> Either<L, R> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<L, R>(Either<L, R>{Either_Left<L, R>{rusty::detail::deref_if_pointer_like(l)}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<L, R>(Either<L, R>{Either_Right<L, R>{rusty::detail::deref_if_pointer_like(r)}}); } return [&]() -> Either<L, R> { rusty::intrinsics::unreachable(); }(); }();
    }
    static Either<L, R> from(rusty::Result<R, L> r) {
        return [&]() -> Either<L, R> { auto&& _m = r; if (_m.is_err()) { auto&& _mv0 = _m.unwrap_err(); auto&& e = rusty::detail::deref_if_pointer(_mv0); return Either<L, R>(Either<L, R>{Either_Left<L, R>{std::move(e)}}); } if (_m.is_ok()) { auto&& _mv1 = _m.unwrap(); auto&& o = rusty::detail::deref_if_pointer(_mv1); return Either<L, R>(Either<L, R>{Either_Right<L, R>{std::move(o)}}); } return [&]() -> Either<L, R> { rusty::intrinsics::unreachable(); }(); }();
    }
    rusty::Result<R, L> into() {
        return [&]() -> rusty::Result<R, L> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::Result<R, L>::Err(l); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::Result<R, L>::Ok(r); } return [&]() -> rusty::Result<R, L> { rusty::intrinsics::unreachable(); }(); }();
    }
    // Rust-only dependent associated type alias skipped in constrained mode: Output
    auto poll(rusty::Context& cx) {
        return [&]() { auto&& _m = this->as_pin_mut(); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.poll(cx); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.poll(cx); } rusty::intrinsics::unreachable(); }();
    }
    rusty::io::Result<size_t> read(std::span<uint8_t> buf) {
        return [&]() -> rusty::io::Result<size_t> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::io::read(inner, buf); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::io::read(inner, buf); } return [&]() -> rusty::io::Result<size_t> { rusty::intrinsics::unreachable(); }(); }();
    }
    rusty::io::Result<std::tuple<>> read_exact(std::span<uint8_t> buf) {
        return [&]() -> rusty::io::Result<std::tuple<>> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.read_exact(buf); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.read_exact(buf); } return [&]() -> rusty::io::Result<std::tuple<>> { rusty::intrinsics::unreachable(); }(); }();
    }
    rusty::io::Result<size_t> read_to_end(rusty::Vec<uint8_t>& buf) {
        return [&]() -> rusty::io::Result<size_t> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.read_to_end(buf); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.read_to_end(buf); } return [&]() -> rusty::io::Result<size_t> { rusty::intrinsics::unreachable(); }(); }();
    }
    rusty::io::Result<size_t> read_to_string(rusty::String& buf) {
        return [&]() -> rusty::io::Result<size_t> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.read_to_string(buf); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.read_to_string(buf); } return [&]() -> rusty::io::Result<size_t> { rusty::intrinsics::unreachable(); }(); }();
    }
    rusty::io::Result<uint64_t> seek(rusty::io::SeekFrom pos) {
        return [&]() -> rusty::io::Result<uint64_t> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.seek(std::move(pos)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.seek(std::move(pos)); } return [&]() -> rusty::io::Result<uint64_t> { rusty::intrinsics::unreachable(); }(); }();
    }
    rusty::io::Result<std::span<const uint8_t>> fill_buf() {
        return [&]() -> rusty::io::Result<std::span<const uint8_t>> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.fill_buf(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.fill_buf(); } return [&]() -> rusty::io::Result<std::span<const uint8_t>> { rusty::intrinsics::unreachable(); }(); }();
    }
    void consume(size_t amt) {
        {
            auto&& _m = (*this);
            std::visit(overloaded {
                [&](::Either_Left<L, R>& _v) {
                    auto& inner = _v._0;
                    inner.consume(std::move(amt));
                },
                [&](::Either_Right<L, R>& _v) {
                    auto& inner = _v._0;
                    inner.consume(std::move(amt));
                },
            }, _m);
        }
    }
    rusty::io::Result<size_t> read_until(uint8_t byte, rusty::Vec<uint8_t>& buf) {
        return [&]() -> rusty::io::Result<size_t> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.read_until(std::move(byte), buf); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.read_until(std::move(byte), buf); } return [&]() -> rusty::io::Result<size_t> { rusty::intrinsics::unreachable(); }(); }();
    }
    rusty::io::Result<size_t> read_line(rusty::String& buf) {
        return [&]() -> rusty::io::Result<size_t> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.read_line(buf); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.read_line(buf); } return [&]() -> rusty::io::Result<size_t> { rusty::intrinsics::unreachable(); }(); }();
    }
    rusty::io::Result<size_t> write_(std::span<const uint8_t> buf) {
        return [&]() -> rusty::io::Result<size_t> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::io::write(inner, buf); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::io::write(inner, buf); } return [&]() -> rusty::io::Result<size_t> { rusty::intrinsics::unreachable(); }(); }();
    }
    rusty::io::Result<std::tuple<>> write_all(std::span<const uint8_t> buf) {
        return [&]() -> rusty::io::Result<std::tuple<>> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.write_all(buf); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.write_all(buf); } return [&]() -> rusty::io::Result<std::tuple<>> { rusty::intrinsics::unreachable(); }(); }();
    }
    rusty::io::Result<std::tuple<>> write_fmt(rusty::fmt::Arguments fmt) {
        return [&]() -> rusty::io::Result<std::tuple<>> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::write_fmt(inner, std::move(fmt)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::write_fmt(inner, std::move(fmt)); } return [&]() -> rusty::io::Result<std::tuple<>> { rusty::intrinsics::unreachable(); }(); }();
    }
    rusty::io::Result<std::tuple<>> flush() {
        return [&]() -> rusty::io::Result<std::tuple<>> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.flush(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.flush(); } return [&]() -> rusty::io::Result<std::tuple<>> { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename Target>
    const Target& as_ref() const {
        return [&]() -> const Target& { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.as_ref(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.as_ref(); } return [&]() -> const Target& { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename Target>
    Target& as_mut() {
        return [&]() -> Target& { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.as_mut(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.as_mut(); } return [&]() -> Target& { rusty::intrinsics::unreachable(); }(); }();
    }
    // Rust-only dependent associated type alias skipped in constrained mode: Target
    decltype(auto) operator*() const {
        return [&]() { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::deref_ref(rusty::deref_ref(inner)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::deref_ref(rusty::deref_ref(inner)); } rusty::intrinsics::unreachable(); }();
    }
    decltype(auto) operator*() {
        return [&]() { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::deref_ref(inner); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::deref_ref(inner); } rusty::intrinsics::unreachable(); }();
    }
    rusty::Option<const void*&> source() const {
        return [&]() -> rusty::Option<const void*&> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.source(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.source(); } return [&]() -> rusty::Option<const void*&> { rusty::intrinsics::unreachable(); }(); }();
    }
    std::string_view description() const {
        return [&]() -> std::string_view { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::error::description(inner); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::error::description(inner); } return [&]() -> std::string_view { rusty::intrinsics::unreachable(); }(); }();
    }
    rusty::Option<const void*> cause() const {
        return [&]() -> rusty::Option<const void*> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.cause(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.cause(); } return [&]() -> rusty::Option<const void*> { rusty::intrinsics::unreachable(); }(); }();
    }
};
template<typename L, typename R>
Either_Left<L, R> Left(L _0) { return Either_Left<L, R>{std::forward<L>(_0)};  }
template<typename L, typename R>
Either_Right<L, R> Right(R _0) { return Either_Right<L, R>{std::forward<R>(_0)};  }

namespace iterator {

    template<typename L, typename R>
    struct IterEither;

    // Rust-only unresolved import: using for_both;
    using ::Either;


    /// Iterator that maps left or right iterators to corresponding `Either`-wrapped items.
    ///
    /// This struct is created by the [`Either::factor_into_iter`],
    /// [`factor_iter`][Either::factor_iter],
    /// and [`factor_iter_mut`][Either::factor_iter_mut] methods.
    template<typename L, typename R>
    struct IterEither {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        Either<L, R> inner;

        iterator::IterEither<L, R> clone() const {
            return iterator::IterEither<L, R>{.inner = rusty::clone(this->inner)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, L>::debug_struct_field1_finish(f, "IterEither", "inner", &this->inner);
        }
        static IterEither<L, R> new_(Either<L, R> inner) {
            return IterEither<L, R>{.inner = std::move(inner)};
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            return rusty::Some([&]() { auto&& _m = this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return Either<rusty::detail::associated_item_t<L>, rusty::detail::associated_item_t<R>>::Left(RUSTY_TRY_OPT(inner.next())); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return Either<rusty::detail::associated_item_t<L>, rusty::detail::associated_item_t<R>>::Right(RUSTY_TRY_OPT(inner.next())); } rusty::intrinsics::unreachable(); }());
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return [&]() -> std::tuple<size_t, rusty::Option<size_t>> { auto&& _m = this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.size_hint(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return inner.size_hint(); } return [&]() -> std::tuple<size_t, rusty::Option<size_t>> { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename Acc, typename G>
        Acc fold(Acc init, G f) {
            return [&]() -> Acc { auto&& _m = this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::fold(inner.map([](auto&& _v) { return rusty::either::Left<L, R>(std::forward<decltype(_v)>(_v)); }), std::move(init), std::move(f)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::fold(inner.map([](auto&& _v) { return rusty::either::Right<L, R>(std::forward<decltype(_v)>(_v)); }), std::move(init), std::move(f)); } return [&]() -> Acc { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename F>
        void for_each(F f) {
            {
                auto&& _m = this->inner;
                std::visit(overloaded {
                    [&](const Either_Left<L, R>& _v) {
                        auto&& inner = rusty::detail::deref_if_pointer(_v._0);
                        rusty::for_each(inner.map([](auto&& _v) { return rusty::either::Left<L, R>(std::forward<decltype(_v)>(_v)); }), std::move(f));
                    },
                    [&](const Either_Right<L, R>& _v) {
                        auto&& inner = rusty::detail::deref_if_pointer(_v._0);
                        rusty::for_each(inner.map([](auto&& _v) { return rusty::either::Right<L, R>(std::forward<decltype(_v)>(_v)); }), std::move(f));
                    },
                }, _m);
            }
        }
        size_t count() {
            return [&]() -> size_t { auto&& _m = this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.count(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.count(); } return [&]() -> size_t { rusty::intrinsics::unreachable(); }(); }();
        }
        auto last() {
            return rusty::Some([&]() { auto&& _m = this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<rusty::detail::associated_item_t<L>, rusty::detail::associated_item_t<R>>::Left(RUSTY_TRY_OPT(inner.last())); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<rusty::detail::associated_item_t<L>, rusty::detail::associated_item_t<R>>::Right(RUSTY_TRY_OPT(inner.last())); } rusty::intrinsics::unreachable(); }());
        }
        auto nth(size_t n) {
            return rusty::Some([&]() { auto&& _m = this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return Either<rusty::detail::associated_item_t<L>, rusty::detail::associated_item_t<R>>::Left(RUSTY_TRY_OPT(inner.nth(std::move(n)))); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return Either<rusty::detail::associated_item_t<L>, rusty::detail::associated_item_t<R>>::Right(RUSTY_TRY_OPT(inner.nth(std::move(n)))); } rusty::intrinsics::unreachable(); }());
        }
        template<typename B>
        B collect() {
            return [&]() -> B { auto&& _m = this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return B::from_iter(inner.map([](auto&& _v) { return rusty::either::Left<L, R>(std::forward<decltype(_v)>(_v)); })); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return B::from_iter(inner.map([](auto&& _v) { return rusty::either::Right<L, R>(std::forward<decltype(_v)>(_v)); })); } return [&]() -> B { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename B, typename F>
        std::tuple<B, B> partition(F f) {
            return [&]() -> std::tuple<B, B> { auto&& _m = this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.map([](auto&& _v) { return rusty::either::Left<L, R>(std::forward<decltype(_v)>(_v)); }).partition(std::move(f)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.map([](auto&& _v) { return rusty::either::Right<L, R>(std::forward<decltype(_v)>(_v)); }).partition(std::move(f)); } return [&]() -> std::tuple<B, B> { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename F>
        bool all(F f) {
            return [&]() -> bool { auto&& _m = &this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.map([](auto&& _v) { return rusty::either::Left<L, R>(std::forward<decltype(_v)>(_v)); }).all(std::move(f)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.map([](auto&& _v) { return rusty::either::Right<L, R>(std::forward<decltype(_v)>(_v)); }).all(std::move(f)); } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename F>
        bool any(F f) {
            return [&]() -> bool { auto&& _m = &this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.map([](auto&& _v) { return rusty::either::Left<L, R>(std::forward<decltype(_v)>(_v)); }).any(std::move(f)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.map([](auto&& _v) { return rusty::either::Right<L, R>(std::forward<decltype(_v)>(_v)); }).any(std::move(f)); } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename P>
        auto find(P predicate) {
            return [&]() { auto&& _m = &this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::find(inner.map([](auto&& _v) { return rusty::either::Left<L, R>(std::forward<decltype(_v)>(_v)); }), predicate); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::find(inner.map([](auto&& _v) { return rusty::either::Right<L, R>(std::forward<decltype(_v)>(_v)); }), predicate); } rusty::intrinsics::unreachable(); }();
        }
        template<typename B, typename F>
        rusty::Option<B> find_map(F f) {
            return [&]() -> rusty::Option<B> { auto&& _m = &this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.map([](auto&& _v) { return rusty::either::Left<L, R>(std::forward<decltype(_v)>(_v)); }).find_map(std::move(f)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.map([](auto&& _v) { return rusty::either::Right<L, R>(std::forward<decltype(_v)>(_v)); }).find_map(std::move(f)); } return [&]() -> rusty::Option<B> { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename P>
        rusty::Option<size_t> position(P predicate) {
            return [&]() -> rusty::Option<size_t> { auto&& _m = &this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.map([](auto&& _v) { return rusty::either::Left<L, R>(std::forward<decltype(_v)>(_v)); }).position(std::move(predicate)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.map([](auto&& _v) { return rusty::either::Right<L, R>(std::forward<decltype(_v)>(_v)); }).position(std::move(predicate)); } return [&]() -> rusty::Option<size_t> { rusty::intrinsics::unreachable(); }(); }();
        }
        auto next_back() {
            return rusty::Some([&]() { auto&& _m = this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return Either<rusty::detail::associated_item_t<L>, rusty::detail::associated_item_t<R>>::Left(RUSTY_TRY_OPT(inner.next_back())); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return Either<rusty::detail::associated_item_t<L>, rusty::detail::associated_item_t<R>>::Right(RUSTY_TRY_OPT(inner.next_back())); } rusty::intrinsics::unreachable(); }());
        }
        auto nth_back(size_t n) {
            return rusty::Some([&]() { auto&& _m = this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return Either<rusty::detail::associated_item_t<L>, rusty::detail::associated_item_t<R>>::Left(RUSTY_TRY_OPT(inner.nth_back(std::move(n)))); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return Either<rusty::detail::associated_item_t<L>, rusty::detail::associated_item_t<R>>::Right(RUSTY_TRY_OPT(inner.nth_back(std::move(n)))); } rusty::intrinsics::unreachable(); }());
        }
        template<typename Acc, typename G>
        Acc rfold(Acc init, G f) {
            return [&]() -> Acc { auto&& _m = this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.map([](auto&& _v) { return rusty::either::Left<L, R>(std::forward<decltype(_v)>(_v)); }).rfold(std::move(init), std::move(f)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.map([](auto&& _v) { return rusty::either::Right<L, R>(std::forward<decltype(_v)>(_v)); }).rfold(std::move(init), std::move(f)); } return [&]() -> Acc { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename P>
        auto rfind(P predicate) {
            return [&]() { auto&& _m = &this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.map([](auto&& _v) { return rusty::either::Left<L, R>(std::forward<decltype(_v)>(_v)); }).rfind(std::move(predicate)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.map([](auto&& _v) { return rusty::either::Right<L, R>(std::forward<decltype(_v)>(_v)); }).rfind(std::move(predicate)); } rusty::intrinsics::unreachable(); }();
        }
        size_t len() const {
            return [&]() -> size_t { auto&& _m = this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::len(inner); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::len(inner); } return [&]() -> size_t { rusty::intrinsics::unreachable(); }(); }();
        }
    };

}


namespace into_either {

    using ::Either;


    // Extension trait IntoEither lowered to rusty_ext:: free functions
    namespace rusty_ext {
    }


}


// Rust-only libtest metadata const skipped: basic (marker: basic, should_panic: no)

// Rust-only libtest metadata const skipped: macros (marker: macros, should_panic: no)

// Rust-only libtest metadata const skipped: deref (marker: deref, should_panic: no)

// Rust-only libtest metadata const skipped: iter (marker: iter, should_panic: no)

// Rust-only libtest metadata const skipped: seek (marker: seek, should_panic: no)

// Rust-only libtest metadata const skipped: read_write (marker: read_write, should_panic: no)

// Rust-only libtest metadata const skipped: error (marker: error, should_panic: no)

void basic() {
    Either<int32_t, int32_t> e = Either<int32_t, int32_t>{Either_Left<int32_t, int32_t>{2}};
    const auto r = Either<int32_t, int32_t>{Either_Right<int32_t, int32_t>{2}};
    {
        auto _m0 = &e;
        auto&& _m1_tmp = Either<int32_t, int32_t>(Either<int32_t, int32_t>{Either_Left<int32_t, int32_t>{2}});
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
    e = std::move(r);
    {
        auto _m0 = &e;
        auto&& _m1_tmp = Either<int32_t, int32_t>(Either<int32_t, int32_t>{Either_Right<int32_t, int32_t>{2}});
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
        auto&& _m0_tmp = e.left();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::None;
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
        auto&& _m0_tmp = e.right();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::Option<int32_t>(static_cast<int32_t>(2));
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
        auto&& _m0_tmp = e.as_ref().right();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::Option<const int32_t&>([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<int32_t>(2); return _some_ref_tmp; }());
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
        auto&& _m0_tmp = e.as_mut().right();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::Option<int32_t&>([&]() -> auto& { auto _some_mut_ref_value = (static_cast<int32_t>(2)); thread_local std::optional<int32_t> _some_mut_ref_tmp; _some_mut_ref_tmp.reset(); _some_mut_ref_tmp.emplace(std::move(_some_mut_ref_value)); return *_some_mut_ref_tmp; }());
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

void macros() {
    using ::rusty::String;
    const rusty::SafeFn<Either<uint32_t, uint32_t>()> a = +[]() -> Either<uint32_t, uint32_t> {
        const uint32_t x = ({ auto&& _m = Either<uint32_t, uint32_t>(Right<uint32_t, uint32_t>(static_cast<uint32_t>(static_cast<uint32_t>(1337)))); uint32_t _match_value; if (_m.is_left()) { auto&& _mv = _m.unwrap_left(); auto&& val = rusty::detail::deref_if_pointer(_mv);
_match_value = val; } else { auto&& _mv = _m.unwrap_right(); auto&& err = rusty::detail::deref_if_pointer(_mv);
return Right<uint32_t, uint32_t>(static_cast<uint32_t>(err)); } _match_value; });
        return Either<uint32_t, uint32_t>{Either_Left<uint32_t, uint32_t>{x * 2}};
    };
    const rusty::SafeFn<Either<rusty::String, std::string_view>()> b = +[]() -> Either<rusty::String, std::string_view> {
        return Either<rusty::String, std::string_view>{Either_Right<rusty::String, std::string_view>{({ auto&& _m = Either<std::string_view, std::string_view>(Left<std::string_view, std::string_view>(std::string_view("foo bar"))); std::string_view _match_value; if (_m.is_right()) { auto&& _mv = _m.unwrap_right(); auto&& val = rusty::detail::deref_if_pointer(_mv);
_match_value = val; } else { auto&& _mv = _m.unwrap_left(); auto&& err = rusty::detail::deref_if_pointer(_mv);
return Left<rusty::String, std::string_view>(rusty::String::from(err)); } _match_value; })}};
    };
    {
        auto&& _m0_tmp = a();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = Either<uint32_t, uint32_t>(Either<uint32_t, uint32_t>{Either_Right<uint32_t, uint32_t>{1337}});
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
        auto&& _m0_tmp = b();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = Either<rusty::String, std::string_view>(Either<rusty::String, std::string_view>{Either_Left<rusty::String, std::string_view>{rusty::String::from("foo bar")}});
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

void deref() {
    using ::rusty::String;
    const rusty::SafeFn<void(std::string_view)> is_str = +[](std::string_view _) {
    };
    const Either<rusty::String, std::string_view> value = Either<rusty::String, std::string_view>{Either_Left<rusty::String, std::string_view>{rusty::String::from("test")}};
    is_str(*(value));
}

void iter() {
    const auto x = 3;
    auto iter_shadow1 = [&]() { auto&& _m = x; if (_m == 3) return Either<rusty::range<int32_t>, rusty::range_from<int32_t>>(Either<rusty::range<int32_t>, rusty::range_from<int32_t>>{Either_Left<rusty::range<int32_t>, rusty::range_from<int32_t>>{rusty::range(0, 10)}});
return Either<rusty::range<int32_t>, rusty::range_from<int32_t>>(Either<rusty::range<int32_t>, rusty::range_from<int32_t>>{Either_Right<rusty::range<int32_t>, rusty::range_from<int32_t>>{rusty::range_from(17)}}); }();
    {
        auto&& _m0_tmp = iter_shadow1.next();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::Option<int32_t>(static_cast<int32_t>(0));
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
        auto&& _m0_tmp = iter_shadow1.count();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = static_cast<int32_t>(9);
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

void seek() {
    namespace io = rusty::io;
    const auto use_empty = false;
    auto mockdata = rusty::array_repeat(static_cast<uint8_t>(0), 256);
    for (auto&& i : rusty::for_in(rusty::range(0, 256))) {
        mockdata.at(i) = static_cast<uint8_t>(i);
    }
    auto reader = (use_empty ? Either<decltype((rusty::io::cursor_new(rusty::array_repeat(static_cast<uint8_t>(0), 0)))), decltype((rusty::io::cursor_new(rusty::slice_full(mockdata))))>(Left<decltype((rusty::io::cursor_new(rusty::array_repeat(static_cast<uint8_t>(0), 0)))), decltype((rusty::io::cursor_new(rusty::slice_full(mockdata))))>(decltype((rusty::io::cursor_new(rusty::array_repeat(static_cast<uint8_t>(0), 0))))(rusty::io::cursor_new(rusty::array_repeat(static_cast<uint8_t>(0), 0))))) : Either<decltype((rusty::io::cursor_new(rusty::array_repeat(static_cast<uint8_t>(0), 0)))), decltype((rusty::io::cursor_new(rusty::slice_full(mockdata))))>(Right<decltype((rusty::io::cursor_new(rusty::array_repeat(static_cast<uint8_t>(0), 0)))), decltype((rusty::io::cursor_new(rusty::slice_full(mockdata))))>(decltype((rusty::io::cursor_new(rusty::slice_full(mockdata))))(rusty::io::cursor_new(rusty::slice_full(mockdata))))));
    auto buf = [](auto _seed) { std::array<uint8_t, 16> _repeat{}; _repeat.fill(static_cast<uint8_t>(_seed)); return _repeat; }(static_cast<uint8_t>(0));
    {
        auto&& _m0_tmp = reader.read(rusty::slice_full(buf)).unwrap();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::len(buf);
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
        auto&& _m0_tmp = rusty::slice_full(buf);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::slice_to(mockdata, rusty::len(buf));
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
        auto&& _m0_tmp = reader.read(rusty::slice_full(buf)).unwrap();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::len(buf);
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
        auto&& _m0_tmp = rusty::slice_full(buf);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::slice_to(mockdata, rusty::len(buf));
        auto _m1 = &_m1_tmp;
        auto _m_tuple = std::make_tuple(_m0, _m1);
        bool _m_matched = false;
        if (!_m_matched) {
            auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
            auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
            if (left_val == right_val) {
                const auto kind = rusty::panicking::AssertKind::Ne;
                rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
            }
            _m_matched = true;
        }
    }
    reader.seek(rusty::io::SeekFrom::Start(0)).unwrap();
    {
        auto&& _m0_tmp = reader.read(rusty::slice_full(buf)).unwrap();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::len(buf);
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
        auto&& _m0_tmp = rusty::slice_full(buf);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::slice_to(mockdata, rusty::len(buf));
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

void read_write() {
    namespace io = rusty::io;
    const auto use_stdio = false;
    const auto mockdata = [](auto _seed) { std::array<uint8_t, 256> _repeat{}; _repeat.fill(static_cast<uint8_t>(_seed)); return _repeat; }(static_cast<uint8_t>(255));
    auto reader = (use_stdio ? Either<decltype((rusty::io::stdin_())), decltype((rusty::slice_full(mockdata)))>(Left<decltype((rusty::io::stdin_())), decltype((rusty::slice_full(mockdata)))>(decltype((rusty::io::stdin_()))(rusty::io::stdin_()))) : Either<decltype((rusty::io::stdin_())), decltype((rusty::slice_full(mockdata)))>(Right<decltype((rusty::io::stdin_())), decltype((rusty::slice_full(mockdata)))>(decltype((rusty::slice_full(mockdata)))(rusty::slice_full(mockdata)))));
    auto buf = [](auto _seed) { std::array<uint8_t, 16> _repeat{}; _repeat.fill(static_cast<uint8_t>(_seed)); return _repeat; }(static_cast<uint8_t>(0));
    {
        auto&& _m0_tmp = reader.read(rusty::slice_full(buf)).unwrap();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::len(buf);
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
        auto&& _m0_tmp = rusty::slice_full(buf);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::slice_to(mockdata, rusty::len(buf));
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
    auto mockbuf = rusty::array_repeat(static_cast<uint8_t>(0), 256);
    auto writer = (use_stdio ? Either<decltype((rusty::io::stdout_())), decltype((rusty::slice_full(mockbuf)))>(Left<decltype((rusty::io::stdout_())), decltype((rusty::slice_full(mockbuf)))>(decltype((rusty::io::stdout_()))(rusty::io::stdout_()))) : Either<decltype((rusty::io::stdout_())), decltype((rusty::slice_full(mockbuf)))>(Right<decltype((rusty::io::stdout_())), decltype((rusty::slice_full(mockbuf)))>(decltype((rusty::slice_full(mockbuf)))(rusty::slice_full(mockbuf)))));
    auto buf_shadow1 = [](auto _seed) { std::array<uint8_t, 16> _repeat{}; _repeat.fill(static_cast<uint8_t>(_seed)); return _repeat; }(static_cast<uint8_t>(1));
    {
        auto&& _m0_tmp = writer.write_(rusty::slice_full(buf_shadow1)).unwrap();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::len(buf_shadow1);
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

void error() {
    const auto invalid_utf8 = std::array<uint8_t, 1>{{ 0xff }};
    auto res = [&]() { auto&& _iflet = rusty::str_runtime::from_utf8(std::move(invalid_utf8)); return (_iflet.is_err() ? ([&]() { auto error = _iflet.unwrap_err(); return rusty::Result<decltype((std::make_tuple())), Either<decltype((_iflet.unwrap_err())), decltype((_iflet.unwrap_err()))>>::Err(Left<decltype((_iflet.unwrap_err())), decltype((_iflet.unwrap_err()))>(decltype((_iflet.unwrap_err()))(std::move(error)))); }()) : [&]() { auto&& _iflet = rusty::str_runtime::parse<int32_t>("x"); return (_iflet.is_err() ? ([&]() { auto error = _iflet.unwrap_err(); return rusty::Result<decltype((std::make_tuple())), Either<decltype((_iflet.unwrap_err())), decltype((_iflet.unwrap_err()))>>::Err(Right<decltype((_iflet.unwrap_err())), decltype((_iflet.unwrap_err()))>(decltype((_iflet.unwrap_err()))(std::move(error)))); }()) : rusty::Result<decltype((std::make_tuple())), Either<decltype((_iflet.unwrap_err())), decltype((_iflet.unwrap_err()))>>::Ok(std::make_tuple())); }()); }();
    if (!res.is_err()) {
        rusty::panicking::panic("assertion failed: res.is_err()");
    }
    res.unwrap_err().description();
}

void _unsized_ref_propagation() {
    const rusty::SafeFn<void()> check_array_ref = +[]() {
    };
    const rusty::SafeFn<void()> check_array_mut = +[]() {
    };
    const auto propagate_array_ref = [&]() {
        check_array_ref();
    };
    const auto propagate_array_mut = [&]() {
        check_array_mut();
    };
    {
        const rusty::SafeFn<void()> check_ref = +[]() {
        };
        const auto propagate_ref = [&]() {
            check_ref();
        };
        const rusty::SafeFn<void()> check_mut = +[]() {
        };
        const auto propagate_mut = [&]() {
            check_mut();
        };
    }
}

void _unsized_std_propagation() {
    {
        const rusty::SafeFn<void()> check_ref = +[]() {
        };
        const auto propagate_ref = [&]() {
            check_ref();
        };
        const rusty::SafeFn<void()> check_mut = +[]() {
        };
        const auto propagate_mut = [&]() {
            check_mut();
        };
    }
    {
        const rusty::SafeFn<void()> check_ref = +[]() {
        };
        const auto propagate_ref = [&]() {
            check_ref();
        };
        const rusty::SafeFn<void()> check_mut = +[]() {
        };
        const auto propagate_mut = [&]() {
            check_mut();
        };
    }
    {
        const rusty::SafeFn<void()> check_ref = +[]() {
        };
        const auto propagate_ref = [&]() {
            check_ref();
        };
        const rusty::SafeFn<void()> check_mut = +[]() {
        };
        const auto propagate_mut = [&]() {
            check_mut();
        };
    }
}

// Rust-only libtest main omitted


// Runnable wrappers for expanded Rust test bodies
// Rust-only libtest wrapper metadata: marker=basic should_panic=no
void rusty_test_basic() {
    basic();
}
// Rust-only libtest wrapper metadata: marker=macros should_panic=no
void rusty_test_macros() {
    macros();
}
// Rust-only libtest wrapper metadata: marker=deref should_panic=no
void rusty_test_deref() {
    deref();
}
// Rust-only libtest wrapper metadata: marker=iter should_panic=no
void rusty_test_iter() {
    iter();
}
// Rust-only libtest wrapper metadata: marker=seek should_panic=no
void rusty_test_seek() {
    seek();
}
// Rust-only libtest wrapper metadata: marker=read_write should_panic=no
void rusty_test_read_write() {
    read_write();
}
// Rust-only libtest wrapper metadata: marker=error should_panic=no
void rusty_test_error() {
    error();
}


// ── Test runner ──
int main(int argc, char** argv) {
    if (argc == 3 && std::string(argv[1]) == "--rusty-single-test") {
        const std::string test_name = argv[2];
        rusty::mem::clear_all_forgotten_addresses();
        try {
            if (test_name == "rusty_test_basic") { rusty_test_basic(); return 0; }
            if (test_name == "rusty_test_deref") { rusty_test_deref(); return 0; }
            if (test_name == "rusty_test_error") { rusty_test_error(); return 0; }
            if (test_name == "rusty_test_iter") { rusty_test_iter(); return 0; }
            if (test_name == "rusty_test_macros") { rusty_test_macros(); return 0; }
            if (test_name == "rusty_test_read_write") { rusty_test_read_write(); return 0; }
            if (test_name == "rusty_test_seek") { rusty_test_seek(); return 0; }
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
    try { rusty_test_basic(); std::cout << "  basic PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  basic FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  basic FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_deref(); std::cout << "  deref PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  deref FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  deref FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_error(); std::cout << "  error PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  error FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  error FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_iter(); std::cout << "  iter PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  iter FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  iter FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_macros(); std::cout << "  macros PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  macros FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  macros FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_read_write(); std::cout << "  read_write PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  read_write FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  read_write FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_seek(); std::cout << "  seek PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  seek FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  seek FAILED (unknown exception)" << std::endl; fail++; }
    std::cout << std::endl;
    std::cout << "Results: " << pass << " passed, " << fail << " failed" << std::endl;
    return fail > 0 ? 1 : 0;
}
