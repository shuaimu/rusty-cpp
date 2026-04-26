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
namespace array_string { template<size_t CAP> struct ArrayString; }
namespace arrayvec { template<typename T, size_t CAP> struct ArrayVec; }
namespace arrayvec { template<typename T, size_t CAP> struct Drain; }
namespace arrayvec { template<typename T, size_t CAP> struct IntoIter; }
namespace arrayvec { template<typename T, typename Data, typename F> struct ScopeExitGuard; }
namespace char_ { struct EncodeUtf8Error; }
namespace errors { template<typename T> struct CapacityError; }
namespace utils { template<typename T, size_t N> struct MakeMaybeUninit; }

// ── from arrayvec.cppm ──



#if defined(__GNUC__)
#pragma GCC diagnostic ignored "-Wunused-local-typedefs"
#endif


namespace array_string {}
namespace arrayvec {}
namespace errors {}
namespace utils {}

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


namespace char_ {
    struct EncodeUtf8Error;
    constexpr uint8_t TAG_CONT = static_cast<uint8_t>(128);
    constexpr uint8_t TAG_TWO_B = static_cast<uint8_t>(192);
    constexpr uint8_t TAG_THREE_B = static_cast<uint8_t>(224);
    constexpr uint8_t TAG_FOUR_B = static_cast<uint8_t>(240);
    constexpr uint32_t MAX_ONE_B = static_cast<uint32_t>(128);
    constexpr uint32_t MAX_TWO_B = static_cast<uint32_t>(2048);
    constexpr uint32_t MAX_THREE_B = static_cast<uint32_t>(65536);
    rusty::Result<size_t, EncodeUtf8Error> encode_utf8(char32_t ch, uint8_t* ptr, size_t len);
    void test_encode_utf8();
    void test_encode_utf8_oob();
}
namespace errors {
    template<typename T>
    struct CapacityError;
    constexpr std::string_view CAPERROR = std::string_view("insufficient capacity");
}
namespace utils {
    template<typename T, size_t N>
    struct MakeMaybeUninit;
}
namespace array_string {
    template<size_t CAP>
    struct ArrayString;
    using errors::CapacityError;
    using utils::MakeMaybeUninit;
}
namespace arrayvec_impl {
    using errors::CapacityError;
}
namespace arrayvec {
    template<typename T, size_t CAP>
    struct ArrayVec;
    template<typename T, size_t CAP>
    struct IntoIter;
    template<typename T, size_t CAP>
    struct Drain;
    template<typename T, typename Data, typename F>
    struct ScopeExitGuard;
    using errors::CapacityError;
    using utils::MakeMaybeUninit;
    void extend_panic();
    template<typename T>
    std::add_pointer_t<T> raw_ptr_add(std::add_pointer_t<T> ptr, size_t offset);
}
using array_string::ArrayString;
using errors::CapacityError;
using arrayvec::ArrayVec;
using arrayvec::IntoIter;
using arrayvec::Drain;
using LenUint = uint32_t;

namespace array_string {
    // Extension trait free-function forward declarations
    namespace rusty_ext {
        template<size_t CAP>
        bool eq(const std::string_view& self_, const ::array_string::ArrayString<CAP>& rhs);

        template<size_t CAP>
        rusty::Option<core::cmp::Ordering> partial_cmp(const std::string_view& self_, const ::array_string::ArrayString<CAP>& rhs);

        template<size_t CAP>
        bool lt(const std::string_view& self_, const ::array_string::ArrayString<CAP>& rhs);

        template<size_t CAP>
        bool le(const std::string_view& self_, const ::array_string::ArrayString<CAP>& rhs);

        template<size_t CAP>
        bool gt(const std::string_view& self_, const ::array_string::ArrayString<CAP>& rhs);

        template<size_t CAP>
        bool ge(const std::string_view& self_, const ::array_string::ArrayString<CAP>& rhs);

    }

}




namespace arrayvec_impl {

    using errors::CapacityError;

    namespace ptr = rusty::ptr;


    using errors::CapacityError;

    // Module-mode trait fallback for default methods on ArrayVecImpl
    struct ArrayVecImplRuntimeHelper {
        static auto as_slice(const auto& self_) -> std::span<const typename std::remove_reference_t<decltype(self_)>::Item> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            const auto len = rusty::len(self_);
            // @unsafe
            {
                return rusty::from_raw_parts(rusty::as_ptr(self_), std::move(len));
            }
        }
        static auto as_mut_slice(auto& self_) -> std::span<typename std::remove_reference_t<decltype(self_)>::Item> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            const auto len = rusty::len(self_);
            // @unsafe
            {
                return rusty::from_raw_parts_mut(rusty::as_mut_ptr(self_), std::move(len));
            }
        }
        static auto push(auto& self_, auto element) -> void {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            self_.try_push(std::move(element)).unwrap();
        }
        static auto try_push(auto& self_, auto element) -> rusty::Result<std::tuple<>, ::errors::CapacityError<typename std::remove_reference_t<decltype(self_)>::Item>> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            if (rusty::len(self_) < rusty::clone(Self_::CAPACITY)) {
                // @unsafe
                {
                    self_.push_unchecked(std::move(element));
                }
                return rusty::Result<std::tuple<>, ::errors::CapacityError<typename Self_::Item>>::Ok(std::make_tuple());
            } else {
                return rusty::Result<std::tuple<>, ::errors::CapacityError<typename Self_::Item>>::Err(::errors::CapacityError<typename Self_::Item>::new_(std::move(element)));
            }
        }
        static auto push_unchecked(auto& self_, auto element) -> void {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            const auto len = rusty::len(self_);
            if (true) {
                if (!(len < rusty::clone(Self_::CAPACITY))) {
                    rusty::panicking::panic("assertion failed: len < Self::CAPACITY");
                }
            }
            rusty::ptr::write(rusty::ptr::add(rusty::as_mut_ptr(self_), std::move(len)), std::move(element));
            self_.set_len(len + 1);
        }
        static auto pop(auto& self_) -> rusty::Option<typename std::remove_reference_t<decltype(self_)>::Item> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            if (rusty::len(self_) == 0) {
                return rusty::None;
            }
            // @unsafe
            {
                auto new_len = rusty::len(self_) - 1;
                self_.set_len(std::move(new_len));
                return rusty::Some(rusty::ptr::read(rusty::ptr::add(rusty::as_ptr(self_), std::move(new_len))));
            }
        }
        static auto clear(auto& self_) -> void {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            self_.truncate(static_cast<size_t>(0));
        }
        static auto truncate(auto& self_, auto new_len) -> void {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            // @unsafe
            {
                const auto len = rusty::len(self_);
                if (new_len < len) {
                    self_.set_len(std::move(new_len));
                    auto tail = rusty::from_raw_parts_mut(rusty::ptr::add(rusty::as_mut_ptr(self_), std::move(new_len)), len - new_len);
                    rusty::ptr::drop_in_place(std::move(tail));
                }
            }
        }
    };

}

namespace char_ {

    struct EncodeUtf8Error;
    rusty::Result<size_t, EncodeUtf8Error> encode_utf8(char32_t ch, uint8_t* ptr, size_t len);
    void test_encode_utf8();
    void test_encode_utf8_oob();








    /// Placeholder
    struct EncodeUtf8Error {
    };


    // Rust-only libtest metadata const skipped: test_encode_utf8 (marker: char::test_encode_utf8, should_panic: no)


    // Rust-only libtest metadata const skipped: test_encode_utf8_oob (marker: char::test_encode_utf8_oob, should_panic: no)

}

namespace errors {

    template<typename T>
    struct CapacityError;

    namespace fmt = rusty::fmt;



    /// Error value indicating insufficient capacity
    template<typename T = std::tuple<>>
    struct CapacityError {
        T element_field;

        CapacityError<T> clone() const {
            return CapacityError<T>(rusty::clone(this->element_field));
        }
        void assert_receiver_is_total_eq() const {
        }
        rusty::cmp::Ordering cmp(const CapacityError<T>& other) const {
            return rusty::cmp::cmp(this->element_field, other.element_field);
        }
        bool operator==(const CapacityError<T>& other) const {
            return this->element_field == other.element_field;
        }
        std::partial_ordering operator<=>(const CapacityError<T>& other) const {
            return rusty::to_partial_ordering([&]() -> rusty::Option<rusty::cmp::Ordering> {
                return rusty::partial_cmp(this->element_field, other.element_field);
            }());
        }
        static CapacityError<T> new_(T element) {
            return CapacityError<T>(std::move(element));
        }
        T element() {
            return this->element_field;
        }
        CapacityError<std::tuple<>> simplify() {
            return CapacityError<std::tuple<>>(std::make_tuple());
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return rusty::write_fmt(f, std::format("{0}", rusty::to_string(CAPERROR)));
        }
    };


}

namespace utils {

    template<typename T, size_t N>
    struct MakeMaybeUninit;

    using ::rusty::PhantomData;

    using ::rusty::MaybeUninit;

    template<typename T, size_t N>
    struct MakeMaybeUninit {
        rusty::PhantomData<rusty::SafeFn<T()>> _0;

        static inline const rusty::MaybeUninit<T> VALUE = rusty::MaybeUninit<T>::uninit();
        static inline const std::array<rusty::MaybeUninit<T>, rusty::sanitize_array_capacity<N>()> ARRAY = [](auto _seed) { std::array<rusty::MaybeUninit<T>, rusty::sanitize_array_capacity<N>()> _repeat{}; _repeat.fill(_seed); return _repeat; }(rusty::clone(MakeMaybeUninit<T, N>::VALUE));
    };

}

namespace arrayvec {

    template<typename T, size_t CAP>
    struct ArrayVec;
    template<typename T, size_t CAP>
    struct IntoIter;
    template<typename T, size_t CAP>
    struct Drain;
    template<typename T, typename Data, typename F>
    struct ScopeExitGuard;
    using errors::CapacityError;
    using utils::MakeMaybeUninit;
    void extend_panic();
    template<typename T>
    std::add_pointer_t<T> raw_ptr_add(std::add_pointer_t<T> ptr, size_t offset);

    namespace cmp = rusty::cmp;


    namespace mem = rusty::mem;


    namespace ptr = rusty::ptr;




    namespace fmt = rusty::fmt;

    namespace io = rusty::io;

    using ::rusty::mem::ManuallyDrop;

    using ::rusty::MaybeUninit;

    using ::LenUint;

    using errors::CapacityError;

    using arrayvec_impl::ArrayVecImplRuntimeHelper;

    using utils::MakeMaybeUninit;

    /// A vector with a fixed capacity.
    ///
    /// The `ArrayVec` is a vector backed by a fixed size array. It keeps track of
    /// the number of initialized elements. The `ArrayVec<T, CAP>` is parameterized
    /// by `T` for the element type and `CAP` for the maximum capacity.
    ///
    /// `CAP` is of type `usize` but is range limited to `u32::MAX`; attempting to create larger
    /// arrayvecs with larger capacity will panic.
    ///
    /// The vector is a contiguous value (storing the elements inline) that you can store directly on
    /// the stack if needed.
    ///
    /// It offers a simple API but also dereferences to a slice, so that the full slice API is
    /// available. The ArrayVec can be converted into a by value iterator.
    template<typename T, size_t CAP>
    struct ArrayVec {
        using Item = T;
        using Target = std::span<const T>;
        using Error = std::conditional_t<true, errors::CapacityError<std::tuple<>>, T>;
        using IntoIter = ::arrayvec::IntoIter<T, CAP>;
        LenUint len_field;
        std::array<rusty::MaybeUninit<T>, rusty::sanitize_array_capacity<CAP>()> xs;
        ArrayVec(LenUint len_init, std::array<rusty::MaybeUninit<T>, rusty::sanitize_array_capacity<CAP>()> xs_init) : len_field(std::move(len_init)), xs(std::move(xs_init)) {}
        ArrayVec(const ArrayVec&) = default;
        ArrayVec(ArrayVec&& other) noexcept : len_field(std::move(other.len_field)), xs(std::move(other.xs)) {
            if (rusty::mem::consume_forgotten_address(&other)) {
                this->rusty_mark_forgotten();
                other.rusty_mark_forgotten();
            } else {
                other.rusty_mark_forgotten();
            }
        }
        ArrayVec& operator=(const ArrayVec&) = default;
        ArrayVec& operator=(ArrayVec&& other) noexcept {
            if (this == &other) {
                return *this;
            }
            this->~ArrayVec();
            new (this) ArrayVec(std::move(other));
            return *this;
        }
        void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


        ~ArrayVec() noexcept(false) {
            if (rusty::mem::consume_forgotten_address(this)) { return; }
            this->clear();
        }
        static constexpr size_t CAPACITY = CAP;
        static ArrayVec<T, CAP> new_() {
            if (rusty::mem::size_of<size_t>() > rusty::mem::size_of<LenUint>()) {
                if (CAP > (static_cast<size_t>(std::numeric_limits<LenUint>::max()))) {
                    {
                        rusty::panic::begin_panic("ArrayVec: largest supported capacity is u32::MAX");
                    }
                }
            }
            // @unsafe
            {
                return ArrayVec<T, CAP>(0, rusty::MaybeUninit<std::array<rusty::MaybeUninit<T>, rusty::sanitize_array_capacity<CAP>()>>::uninit().assume_init());
            }
        }
        static ArrayVec<T, CAP> new_const() {
            if (rusty::mem::size_of<size_t>() > rusty::mem::size_of<LenUint>()) {
                if (CAP > (static_cast<size_t>(std::numeric_limits<LenUint>::max()))) {
                    [&]() -> ArrayVec<T, CAP> { rusty::panicking::panic("index out of bounds"); }();
                }
            }
            return ArrayVec<T, CAP>(0, rusty::clone(MakeMaybeUninit<T, CAP>::ARRAY));
        }
        size_t len() const {
            return static_cast<size_t>(this->len_field);
        }
        bool is_empty() const {
            return rusty::len((*this)) == static_cast<size_t>(0);
        }
        size_t capacity() const {
            return CAP;
        }
        bool is_full() const {
            return rusty::len((*this)) == this->capacity();
        }
        size_t remaining_capacity() const {
            return this->capacity() - rusty::len((*this));
        }
        void push(T element) {
            std::conditional_t<true, ::arrayvec_impl::ArrayVecImplRuntimeHelper, T>::push((*this), std::move(element));
        }
        rusty::Result<std::tuple<>, errors::CapacityError<T>> try_push(T element) {
            return std::conditional_t<true, ::arrayvec_impl::ArrayVecImplRuntimeHelper, T>::try_push((*this), std::move(element));
        }
        void push_unchecked(T element) {
            std::conditional_t<true, ::arrayvec_impl::ArrayVecImplRuntimeHelper, T>::push_unchecked((*this), std::move(element));
        }
        void truncate(size_t new_len) {
            std::conditional_t<true, ::arrayvec_impl::ArrayVecImplRuntimeHelper, T>::truncate((*this), std::move(new_len));
        }
        void clear() {
            std::conditional_t<true, ::arrayvec_impl::ArrayVecImplRuntimeHelper, T>::clear((*this));
        }
        std::add_pointer_t<T> get_unchecked_ptr(size_t index) {
            return rusty::ptr::add(reinterpret_cast<std::add_pointer_t<T>>(rusty::as_mut_ptr((*this))), std::move(index));
        }
        void insert(size_t index, T element) {
            this->try_insert(std::move(index), std::move(element)).unwrap();
        }
        rusty::Result<std::tuple<>, errors::CapacityError<T>> try_insert(size_t index, T element) {
            if (index > rusty::len((*this))) {
                {
                    [&]() -> rusty::Result<std::tuple<>, errors::CapacityError<T>> { rusty::panicking::panic_fmt(std::format("ArrayVec::try_insert: index {0} is out of bounds in vector of length {1}", index, rusty::len((*this)))); }();
                }
            }
            if (rusty::len((*this)) == this->capacity()) {
                return rusty::Result<std::tuple<>, errors::CapacityError<T>>::Err(errors::CapacityError<T>::new_(std::move(element)));
            }
            const auto len = rusty::len((*this));
            // @unsafe
            {
                {
                    auto* p = this->get_unchecked_ptr(std::move(index));
                    rusty::ptr::copy(std::move(p), rusty::ptr::offset(p, 1), len - index);
                    rusty::ptr::write(std::move(p), std::move(element));
                }
                this->set_len(len + 1);
            }
            return rusty::Result<std::tuple<>, errors::CapacityError<T>>::Ok(std::make_tuple());
        }
        rusty::Option<T> pop() {
            return std::conditional_t<true, ::arrayvec_impl::ArrayVecImplRuntimeHelper, T>::pop((*this));
        }
        T swap_remove(size_t index) {
            return this->swap_pop(std::move(index)).unwrap_or_else([&]() {
{
    [&]() -> T { rusty::panicking::panic_fmt(std::format("ArrayVec::swap_remove: index {0} is out of bounds in vector of length {1}", index, rusty::len((*this)))); }();
}
});
        }
        rusty::Option<T> swap_pop(size_t index) {
            const auto len = rusty::len((*this));
            if (index >= len) {
                return rusty::Option<T>(rusty::None);
            }
            [&]() { auto&& _swap_recv = (*this); auto&& _swap_view = _swap_recv; const auto _swap_i = index; const auto _swap_j = len - 1; const auto _swap_len = rusty::len(_swap_view); if (!(_swap_i < _swap_len && _swap_j < _swap_len)) { rusty::panicking::panic("index out of bounds"); } rusty::mem::swap(_swap_view[_swap_i], _swap_view[_swap_j]); }();
            return this->pop();
        }
        T remove(size_t index) {
            return this->pop_at(std::move(index)).unwrap_or_else([&]() {
{
    [&]() -> T { rusty::panicking::panic_fmt(std::format("ArrayVec::remove: index {0} is out of bounds in vector of length {1}", index, rusty::len((*this)))); }();
}
});
        }
        rusty::Option<T> pop_at(size_t index) {
            if (index >= rusty::len((*this))) {
                return rusty::Option<T>(rusty::None);
            } else {
                return this->drain(rusty::range(index, index + 1)).next();
            }
        }
        struct BackshiftOnDrop {
            ArrayVec<T, CAP>& v;
            size_t processed_len;
            size_t deleted_cnt;
            size_t original_len;
            BackshiftOnDrop(ArrayVec<T, CAP>& v_init, size_t processed_len_init, size_t deleted_cnt_init, size_t original_len_init) : v(v_init), processed_len(std::move(processed_len_init)), deleted_cnt(std::move(deleted_cnt_init)), original_len(std::move(original_len_init)) {}
            BackshiftOnDrop(const BackshiftOnDrop&) = default;
            BackshiftOnDrop(BackshiftOnDrop&& other) noexcept : v(other.v), processed_len(std::move(other.processed_len)), deleted_cnt(std::move(other.deleted_cnt)), original_len(std::move(other.original_len)) {
                if (rusty::mem::consume_forgotten_address(&other)) {
                    this->rusty_mark_forgotten();
                    other.rusty_mark_forgotten();
                } else {
                    other.rusty_mark_forgotten();
                }
            }
            BackshiftOnDrop& operator=(const BackshiftOnDrop&) = default;
            BackshiftOnDrop& operator=(BackshiftOnDrop&& other) noexcept {
                if (this == &other) {
                    return *this;
                }
                this->~BackshiftOnDrop();
                new (this) BackshiftOnDrop(std::move(other));
                return *this;
            }
            void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


            ~BackshiftOnDrop() noexcept(false) {
                if (rusty::mem::consume_forgotten_address(this)) { return; }
                if (this->deleted_cnt > 0) {
                    // @unsafe
                    {
                        rusty::ptr::copy(rusty::ptr::add(rusty::as_ptr(this->v), this->processed_len), rusty::ptr::add(rusty::as_mut_ptr(this->v), this->processed_len - this->deleted_cnt), this->original_len - this->processed_len);
                    }
                }
                // @unsafe
                {
                    this->v.set_len(this->original_len - this->deleted_cnt);
                }
            }
        };
        template<typename F>
        void retain(F f) {
            const auto process_one = [](bool DELETED, auto& f, auto& g) -> bool {
                const auto cur = rusty::ptr::add(reinterpret_cast<std::add_pointer_t<T>>(rusty::as_mut_ptr(g.v)), g.processed_len);
                if (!f(*cur)) {
                    [&]() { static_cast<void>(g.processed_len += 1); return std::make_tuple(); }();
                    [&]() { static_cast<void>(g.deleted_cnt += 1); return std::make_tuple(); }();
                    // @unsafe
                    {
                        rusty::ptr::drop_in_place(std::move(cur));
                    }
                    return false;
                }
                if (DELETED) {
                    // @unsafe
                    {
                        const auto hole_slot = rusty::ptr::sub(cur, g.deleted_cnt);
                        rusty::ptr::copy_nonoverlapping(std::move(cur), std::move(hole_slot), 1);
                    }
                }
                [&]() { static_cast<void>(g.processed_len += 1); return std::make_tuple(); }();
                return true;
            };
            auto original_len = rusty::len((*this));
            // @unsafe
            {
                this->set_len(static_cast<size_t>(0));
            }
            auto g = BackshiftOnDrop((*this), static_cast<size_t>(0), static_cast<size_t>(0), std::move(original_len));
            while (g.processed_len != original_len) {
                if (!process_one(false, f, g)) {
                    break;
                }
            }
            while (g.processed_len != original_len) {
                process_one(true, f, g);
            }
            rusty::mem::drop(std::move(g));
        }
        void set_len(size_t length) {
            if (true) {
                if (!(length <= this->capacity())) {
                    rusty::panicking::panic("assertion failed: length <= self.capacity()");
                }
            }
            this->len_field = static_cast<LenUint>(length);
        }
        auto try_extend_from_slice(std::span<const T> other) -> rusty::Result<std::tuple<>, errors::CapacityError<std::tuple<>>> {
            if (this->remaining_capacity() < rusty::len(other)) {
                return rusty::Result<std::tuple<>, errors::CapacityError<std::tuple<>>>::Err(errors::CapacityError<std::tuple<>>::new_(std::make_tuple()));
            }
            auto self_len = rusty::len((*this));
            auto other_len = rusty::len(other);
            // @unsafe
            {
                const auto dst = this->get_unchecked_ptr(std::move(self_len));
                rusty::ptr::copy_nonoverlapping(rusty::as_ptr(other), std::move(dst), std::move(other_len));
                this->set_len(self_len + other_len);
            }
            return rusty::Result<std::tuple<>, errors::CapacityError<std::tuple<>>>::Ok(std::make_tuple());
        }
        template<typename R>
        Drain<T, CAP> drain(R range) {
            const auto len = rusty::len((*this));
            auto start = [&]() -> size_t { auto&& _m = range.start_bound(); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return 0; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& i = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0)); return i; } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& i = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0)); return rusty::saturating_add(i, rusty::detail::deref_if_pointer(1)); } rusty::intrinsics::unreachable(); }();
            auto end = [&]() -> size_t { auto&& _m = range.end_bound(); if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& j = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0)); return j; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& j = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0)); return rusty::saturating_add(j, rusty::detail::deref_if_pointer(1)); } if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return len; } rusty::intrinsics::unreachable(); }();
            return this->drain_range(std::move(start), std::move(end));
        }
        Drain<T, CAP> drain_range(size_t start, size_t end) {
            const auto len = rusty::len((*this));
            const auto range_slice_backing = rusty::slice((*this), start, end);
            const auto* range_slice = &range_slice_backing;
            this->len_field = static_cast<LenUint>(start);
            // @unsafe
            {
                return Drain<T, CAP>(std::move(end), len - end, rusty::iter((*range_slice)), &(*this));
            }
        }
        rusty::Result<std::array<T, rusty::sanitize_array_capacity<CAP>()>, ArrayVec<T, CAP>> into_inner() {
            if (rusty::len((*this)) < this->capacity()) {
                return rusty::Result<std::array<T, rusty::sanitize_array_capacity<CAP>()>, ArrayVec<T, CAP>>::Err(std::move((*this)));
            } else {
                // @unsafe
                {
                    return rusty::Result<std::array<T, rusty::sanitize_array_capacity<CAP>()>, ArrayVec<T, CAP>>::Ok(this->into_inner_unchecked());
                }
            }
        }
        std::array<T, rusty::sanitize_array_capacity<CAP>()> into_inner_unchecked() {
            if (true) {
                {
                    auto&& _m0_tmp = rusty::len((*this));
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = this->capacity();
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(left_val == right_val)) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            [&]() -> std::array<T, rusty::sanitize_array_capacity<CAP>()> { rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None); }();
                        }
                        _m_matched = true;
                    }
                }
            }
            const auto self_ = rusty::mem::manually_drop_new(std::move((*this)));
            auto array = rusty::ptr::read(reinterpret_cast<std::add_pointer_t<std::add_const_t<std::array<T, rusty::sanitize_array_capacity<CAP>()>>>>((*self_).as_ptr()));
            return array;
        }
        ArrayVec<T, CAP> take() {
            return rusty::mem::replace((*this), ArrayVec<T, CAP>::new_());
        }
        std::span<const T> as_slice() const {
            return std::conditional_t<true, ::arrayvec_impl::ArrayVecImplRuntimeHelper, T>::as_slice((*this));
        }
        std::span<T> as_mut_slice() {
            return std::conditional_t<true, ::arrayvec_impl::ArrayVecImplRuntimeHelper, T>::as_mut_slice((*this));
        }
        std::add_pointer_t<std::add_const_t<Item>> as_ptr() const {
            return reinterpret_cast<std::add_pointer_t<std::add_const_t<Item>>>(rusty::as_ptr(this->xs));
        }
        std::add_pointer_t<Item> as_mut_ptr() {
            return const_cast<std::add_pointer_t<Item>>(reinterpret_cast<std::add_pointer_t<std::add_const_t<Item>>>(rusty::as_mut_ptr(this->xs)));
        }
        std::span<const T> operator*() const {
            return rusty::as_slice((*this));
        }
        std::span<T> operator*() {
            return rusty::as_mut_slice((*this));
        }
        static ArrayVec<T, CAP> from(std::array<T, rusty::sanitize_array_capacity<CAP>()> array) {
            auto array_shadow1 = rusty::mem::manually_drop_new(std::move(array));
            auto vec = ArrayVec<T, CAP>::new_();
            // @unsafe
            {
                rusty::ptr::copy_nonoverlapping(((reinterpret_cast<std::add_pointer_t<std::add_const_t<std::array<rusty::MaybeUninit<T>, rusty::sanitize_array_capacity<CAP>()>>>>(static_cast<std::add_pointer_t<std::add_const_t<std::array<T, rusty::sanitize_array_capacity<CAP>()>>>>(rusty::addr_of_temp(*(array_shadow1)))))), static_cast<std::add_pointer_t<std::array<rusty::MaybeUninit<T>, rusty::sanitize_array_capacity<CAP>()>>>(&vec.xs), 1);
                vec.set_len(CAP);
            }
            return vec;
        }
        static rusty::Result<ArrayVec<T, CAP>, Error> try_from(std::span<const T> slice) {
            if (rusty::clone(ArrayVec<T, CAP>::CAPACITY) < rusty::len(slice)) {
                return rusty::Result<ArrayVec<T, CAP>, Error>::Err(CapacityError<std::tuple<>>::new_(std::make_tuple()));
            } else {
                auto array = ArrayVec<T, CAP>::new_();
                array.extend_from_slice(slice);
                return rusty::Result<ArrayVec<T, CAP>, Error>::Ok(std::move(array));
            }
        }
        IntoIter into_iter() {
            return IntoIter(static_cast<size_t>(0), std::move((*this)));
        }
        template<typename I>
        void extend(I iter) {
            // @unsafe
            {
                this->template extend_from_iter<std::remove_cvref_t<decltype((std::move(iter)))>, true>(std::move(iter));
            }
        }
        template<typename I, bool CHECK>
        void extend_from_iter(I iterable) {
            auto take = this->capacity() - rusty::len((*this));
            auto len = rusty::len((*this));
            auto ptr_shadow1 = raw_ptr_add<std::remove_pointer_t<std::remove_cvref_t<decltype((reinterpret_cast<std::add_pointer_t<T>>(rusty::as_mut_ptr((*this)))))>>>(reinterpret_cast<std::add_pointer_t<T>>(rusty::as_mut_ptr((*this))), std::move(len));
            const auto end_ptr = raw_ptr_add<std::remove_pointer_t<std::remove_cvref_t<decltype((std::move(ptr_shadow1)))>>>(std::move(ptr_shadow1), std::move(take));
            auto guard = ScopeExitGuard(&this->len_field, std::move(len), [=](auto&& _closure_ref_param0, auto&& self_len) mutable {
auto len = rusty::detail::deref_if_pointer_like(_closure_ref_param0);
rusty::detail::deref_if_pointer_like(rusty::deref_mut(self_len)) = static_cast<LenUint>(len);
});
            auto iter = rusty::iter(std::move(iterable));
            while (true) {
                if (auto&& _iflet_scrutinee = iter.next(); rusty::detail::option_has_value(_iflet_scrutinee)) {
                    auto&& _iflet_take = _iflet_scrutinee;
                    auto elt = rusty::detail::option_take_value(_iflet_take);
                    if ((ptr_shadow1 == end_ptr) && CHECK) {
                        extend_panic();
                    }
                    if (true) {
                        {
                            auto _m0 = &ptr_shadow1;
                            auto _m1 = &end_ptr;
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
                    }
                    if (rusty::mem::size_of<T>() != 0) {
                        rusty::ptr::write(ptr_shadow1, std::move(elt));
                    }
                    ptr_shadow1 = raw_ptr_add<std::remove_pointer_t<std::remove_cvref_t<decltype((std::move(ptr_shadow1)))>>>(std::move(ptr_shadow1), static_cast<size_t>(1));
                    [&]() { static_cast<void>(guard.data += 1); return std::make_tuple(); }();
                } else {
                    return;
                }
            }
        }
        void extend_from_slice(std::span<const T> slice) {
            const auto take = this->capacity() - rusty::len((*this));
            if (true) {
                if (!(rusty::len(slice) <= take)) {
                    rusty::panicking::panic("assertion failed: slice.len() <= take");
                }
            }
            // @unsafe
            {
                const auto slice_shadow1 = (take < rusty::len(slice) ? rusty::slice_to(slice, take) : slice);
                this->template extend_from_iter<std::remove_cvref_t<decltype((rusty::iter(slice_shadow1).cloned()))>, false>(rusty::iter(slice_shadow1).cloned());
            }
        }
        template<typename I>
        static ArrayVec<T, CAP> from_iter(I iter) {
            auto array = ArrayVec<T, CAP>::new_();
            array.extend(std::move(iter));
            return array;
        }
        ArrayVec<T, CAP> clone() const {
            return ArrayVec<T, CAP>::from_iter(rusty::iter((*this)).cloned());
        }
        void clone_from(const ArrayVec<T, CAP>& rhs) {
            auto prefix = rusty::cmp::min(rusty::len((*this)), rusty::len(rhs));
            rusty::clone_from_slice(rusty::slice_to((*this), prefix), rusty::slice_to(rhs, prefix));
            if (prefix < rusty::len((*this))) {
                this->truncate(std::move(prefix));
            } else {
                auto rhs_elems = rusty::slice_from(rhs, rusty::len((*this)));
                this->extend_from_slice(rhs_elems);
            }
        }
        template<typename H>
        void hash(H& state) const {
            rusty::hash::hash(rusty::detail::deref_if_pointer_like((*this)), state);
        }
        bool operator==(const ArrayVec<T, CAP>& other) const {
            return rusty::detail::deref_if_pointer_like((*this)) == rusty::detail::deref_if_pointer_like(other);
        }
        bool operator==(std::span<const T> other) const {
            return rusty::detail::deref_if_pointer_like((*this)) == other;
        }
        std::span<const T> borrow() const {
            return this->operator*();
        }
        std::span<T> borrow_mut() {
            return this->operator*();
        }
        std::span<const T> as_ref() const {
            return this->operator*();
        }
        std::span<T> as_mut() {
            return this->operator*();
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return ((rusty::detail::deref_if_pointer_like((*this)))).fmt(f);
        }
        static ArrayVec<T, CAP> default_() {
            return ArrayVec<T, CAP>::new_();
        }
        std::partial_ordering operator<=>(const ArrayVec<T, CAP>& other) const {
            return rusty::to_partial_ordering([&]() -> rusty::Option<cmp::Ordering> {
                return rusty::partial_cmp(((rusty::detail::deref_if_pointer_like((*this)))), other);
            }());
        }
        cmp::Ordering cmp(const ArrayVec<T, CAP>& other) const {
            return rusty::cmp::cmp(((rusty::detail::deref_if_pointer_like((*this)))), other);
        }
        rusty::io::Result<size_t> write_(std::span<const uint8_t> data) {
            auto len = rusty::cmp::min(this->remaining_capacity(), rusty::len(data));
            const auto _result = this->try_extend_from_slice(rusty::slice_to(data, len));
            if (true) {
                if (!_result.is_ok()) {
                    [&]() -> rusty::io::Result<size_t> { rusty::panicking::panic("assertion failed: _result.is_ok()"); }();
                }
            }
            return rusty::io::Result<size_t>::ok(std::move(len));
        }
        rusty::io::Result<std::tuple<>> flush() {
            return rusty::io::Result<std::tuple<>>::ok(std::make_tuple());
        }
    };

    /// By-value iterator for `ArrayVec`.
    template<typename T, size_t CAP>
    struct IntoIter {
        using Item = T;
        size_t index;
        ArrayVec<T, CAP> v;
        IntoIter(size_t index_init, ArrayVec<T, CAP> v_init) : index(std::move(index_init)), v(std::move(v_init)) {}
        IntoIter(const IntoIter&) = default;
        IntoIter(IntoIter&& other) noexcept : index(std::move(other.index)), v(std::move(other.v)) {
            if (rusty::mem::consume_forgotten_address(&other)) {
                this->rusty_mark_forgotten();
                other.rusty_mark_forgotten();
            } else {
                other.rusty_mark_forgotten();
            }
        }
        IntoIter& operator=(const IntoIter&) = default;
        IntoIter& operator=(IntoIter&& other) noexcept {
            if (this == &other) {
                return *this;
            }
            this->~IntoIter();
            new (this) IntoIter(std::move(other));
            return *this;
        }
        void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


        std::span<const T> as_slice() const {
            return rusty::slice_from(this->v, this->index);
        }
        std::span<T> as_mut_slice() {
            return rusty::slice_from(this->v, this->index);
        }
        rusty::Option<Item> next() {
            if (this->index == rusty::len(this->v)) {
                return rusty::Option<T>(rusty::None);
            } else {
                // @unsafe
                {
                    auto index = this->index;
                    this->index = index + 1;
                    return rusty::Option<T>(rusty::ptr::read(this->v.get_unchecked_ptr(std::move(index))));
                }
            }
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto len = rusty::len(this->v) - this->index;
            return std::make_tuple(std::move(len), rusty::Option<size_t>(std::move(len)));
        }
        rusty::Option<Item> next_back() {
            if (this->index == rusty::len(this->v)) {
                return rusty::Option<T>(rusty::None);
            } else {
                // @unsafe
                {
                    auto new_len = rusty::len(this->v) - 1;
                    this->v.set_len(std::move(new_len));
                    return rusty::Option<T>(rusty::ptr::read(this->v.get_unchecked_ptr(std::move(new_len))));
                }
            }
        }
        ~IntoIter() noexcept(false) {
            if (rusty::mem::consume_forgotten_address(this)) { return; }
            auto index = this->index;
            const auto len = rusty::len(this->v);
            // @unsafe
            {
                this->v.set_len(static_cast<size_t>(0));
                auto elements = rusty::from_raw_parts_mut(this->v.get_unchecked_ptr(std::move(index)), len - index);
                rusty::ptr::drop_in_place(std::move(elements));
            }
        }
        IntoIter<T, CAP> clone() const {
            auto v = ArrayVec<T, CAP>::new_();
            v.extend_from_slice(rusty::slice_from(this->v, this->index));
            return v.into_iter();
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_list().entries(rusty::slice_from(this->v, this->index)).finish();
        }
    };

    /// A draining iterator for `ArrayVec`.
    template<typename T, size_t CAP>
    struct Drain {
        using Item = T;
        /// Index of tail to preserve
        size_t tail_start;
        /// Length of tail
        size_t tail_len;
        /// Current remaining range to remove
        rusty::slice_iter::Iter<const T> iter;
        std::add_pointer_t<ArrayVec<T, CAP>> vec;
        Drain(size_t tail_start_init, size_t tail_len_init, rusty::slice_iter::Iter<const T> iter_init, std::add_pointer_t<ArrayVec<T, CAP>> vec_init) : tail_start(std::move(tail_start_init)), tail_len(std::move(tail_len_init)), iter(std::move(iter_init)), vec(std::move(vec_init)) {}
        Drain(const Drain&) = default;
        Drain(Drain&& other) noexcept : tail_start(std::move(other.tail_start)), tail_len(std::move(other.tail_len)), iter(std::move(other.iter)), vec(std::move(other.vec)) {
            if (rusty::mem::consume_forgotten_address(&other)) {
                this->rusty_mark_forgotten();
                other.rusty_mark_forgotten();
            } else {
                other.rusty_mark_forgotten();
            }
        }
        Drain& operator=(const Drain&) = default;
        Drain& operator=(Drain&& other) noexcept {
            if (this == &other) {
                return *this;
            }
            this->~Drain();
            new (this) Drain(std::move(other));
            return *this;
        }
        void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


        rusty::Option<Item> next() {
            return this->iter.next().map([&](auto&& elt) -> Item { return rusty::ptr::read(elt); });
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return this->iter.size_hint();
        }
        rusty::Option<Item> next_back() {
            return this->iter.next_back().map([&](auto&& elt) -> Item { return rusty::ptr::read(elt); });
        }
        ~Drain() noexcept(false) {
            if (rusty::mem::consume_forgotten_address(this)) { return; }
            while (true) {
                auto&& _whilelet = this->next();
                if (!(_whilelet.is_some())) { break; }
            }
            if (this->tail_len > 0) {
                // @unsafe
                {
                    auto& source_vec = *this->vec;
                    const auto start = rusty::len(source_vec);
                    const auto tail = this->tail_start;
                    const auto ptr_shadow1 = reinterpret_cast<std::add_pointer_t<T>>(rusty::as_mut_ptr(source_vec));
                    rusty::ptr::copy(rusty::ptr::add(ptr_shadow1, std::move(tail)), rusty::ptr::add(ptr_shadow1, std::move(start)), this->tail_len);
                    source_vec.set_len(start + this->tail_len);
                }
            }
        }
    };

    template<typename T, typename Data, typename F>
    struct ScopeExitGuard {
        T value;
        Data data;
        F f;
        ScopeExitGuard(T value_init, Data data_init, F f_init) : value(std::move(value_init)), data(std::move(data_init)), f(std::move(f_init)) {}
        ScopeExitGuard(const ScopeExitGuard&) = default;
        ScopeExitGuard(ScopeExitGuard&& other) noexcept : value(std::move(other.value)), data(std::move(other.data)), f(std::move(other.f)) {
            if (rusty::mem::consume_forgotten_address(&other)) {
                this->rusty_mark_forgotten();
                other.rusty_mark_forgotten();
            } else {
                other.rusty_mark_forgotten();
            }
        }
        ScopeExitGuard& operator=(const ScopeExitGuard&) = default;
        ScopeExitGuard& operator=(ScopeExitGuard&& other) noexcept {
            if (this == &other) {
                return *this;
            }
            this->~ScopeExitGuard();
            new (this) ScopeExitGuard(std::move(other));
            return *this;
        }
        void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


        ~ScopeExitGuard() noexcept(false) {
            if (rusty::mem::consume_forgotten_address(this)) { return; }
            (this->f)(&this->data, &this->value);
        }
    };

}

namespace array_string {

    template<size_t CAP>
    struct ArrayString;
    using errors::CapacityError;
    using utils::MakeMaybeUninit;


    namespace cmp = rusty::cmp;


    namespace fmt = rusty::fmt;


    using ::rusty::MaybeUninit;


    using ::rusty::path::Path;

    namespace ptr = rusty::ptr;


    namespace str = rusty::str_runtime;


    using ::rusty::str_runtime::Utf8Error;

    using errors::CapacityError;

    using ::LenUint;

    using char_::encode_utf8;

    using utils::MakeMaybeUninit;

    /// A string with a fixed capacity.
    ///
    /// The `ArrayString` is a string backed by a fixed size array. It keeps track
    /// of its length, and is parameterized by `CAP` for the maximum capacity.
    ///
    /// `CAP` is of type `usize` but is range limited to `u32::MAX`; attempting to create larger
    /// arrayvecs with larger capacity will panic.
    ///
    /// The string is a contiguous value that you can store directly on the stack
    /// if needed.
    template<size_t CAP>
    struct ArrayString {
        using Target = std::string_view;
        using Err = ::errors::CapacityError<std::tuple<>>;
        using Error = ::errors::CapacityError<std::string_view>;
        LenUint len_field;
        std::array<rusty::MaybeUninit<uint8_t>, rusty::sanitize_array_capacity<CAP>()> xs;

        static ArrayString<CAP> default_() {
            return ArrayString<CAP>::new_();
        }
        static ArrayString<CAP> new_() {
            if (rusty::mem::size_of<size_t>() > rusty::mem::size_of<LenUint>()) {
                if (CAP > (static_cast<size_t>(std::numeric_limits<LenUint>::max()))) {
                    {
                        rusty::panic::begin_panic("ArrayVec: largest supported capacity is u32::MAX");
                    }
                }
            }
            // @unsafe
            {
                return ArrayString<CAP>(0, rusty::MaybeUninit<std::array<rusty::MaybeUninit<uint8_t>, rusty::sanitize_array_capacity<CAP>()>>::uninit().assume_init());
            }
        }
        static ArrayString<CAP> new_const() {
            if (rusty::mem::size_of<size_t>() > rusty::mem::size_of<LenUint>()) {
                if (CAP > (static_cast<size_t>(std::numeric_limits<LenUint>::max()))) {
                    [&]() -> ArrayString<CAP> { rusty::panicking::panic("index out of bounds"); }();
                }
            }
            return ArrayString<CAP>(0, rusty::clone(MakeMaybeUninit<uint8_t, CAP>::ARRAY));
        }
        size_t len() const {
            return static_cast<size_t>(this->len_field);
        }
        bool is_empty() const {
            return rusty::len((*this)) == static_cast<size_t>(0);
        }
        static rusty::Result<ArrayString<CAP>, ::errors::CapacityError<std::string_view>> from(std::string_view s) {
            auto arraystr = ArrayString<CAP>::new_();
            RUSTY_TRY_INTO(arraystr.try_push_str(std::string_view(s)), rusty::Result<ArrayString<CAP>, ::errors::CapacityError<std::string_view>>);
            return rusty::Result<ArrayString<CAP>, ::errors::CapacityError<std::string_view>>::Ok(std::move(arraystr));
        }
        static rusty::Result<ArrayString<CAP>, rusty::str_runtime::Utf8Error> from_byte_string(const std::array<uint8_t, rusty::sanitize_array_capacity<CAP>()>& b) {
            const auto len = rusty::len(RUSTY_TRY_INTO(rusty::str_runtime::from_utf8(b), rusty::Result<ArrayString<CAP>, rusty::str_runtime::Utf8Error>));
            if (true) {
                {
                    auto _m0 = &len;
                    auto&& _m1_tmp = CAP;
                    auto _m1 = &_m1_tmp;
                    auto _m_tuple = std::make_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched) {
                        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                            const auto kind = rusty::panicking::AssertKind::Eq;
                            [&]() -> rusty::Result<ArrayString<CAP>, rusty::str_runtime::Utf8Error> { rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None); }();
                        }
                        _m_matched = true;
                    }
                }
            }
            auto vec = ArrayString<CAP>::new_();
            // @unsafe
            {
                rusty::ptr::copy_nonoverlapping(((reinterpret_cast<const std::array<rusty::MaybeUninit<uint8_t>, rusty::sanitize_array_capacity<CAP>()>*>(static_cast<const std::array<uint8_t, rusty::sanitize_array_capacity<CAP>()>*>(&b)))), static_cast<std::array<rusty::MaybeUninit<uint8_t>, rusty::sanitize_array_capacity<CAP>()>*>(&vec.xs), 1);
                vec.set_len(CAP);
            }
            return rusty::Result<ArrayString<CAP>, rusty::str_runtime::Utf8Error>::Ok(std::move(vec));
        }
        static ArrayString<CAP> zero_filled() {
            if (rusty::mem::size_of<size_t>() > rusty::mem::size_of<LenUint>()) {
                if (CAP > (static_cast<size_t>(std::numeric_limits<LenUint>::max()))) {
                    {
                        rusty::panic::begin_panic("ArrayVec: largest supported capacity is u32::MAX");
                    }
                }
            }
            // @unsafe
            {
                return ArrayString<CAP>(static_cast<LenUint>(CAP), rusty::MaybeUninit<std::array<rusty::MaybeUninit<uint8_t>, rusty::sanitize_array_capacity<CAP>()>>::zeroed().assume_init());
            }
        }
        size_t capacity() const {
            return CAP;
        }
        bool is_full() const {
            return rusty::len((*this)) == this->capacity();
        }
        size_t remaining_capacity() const {
            return this->capacity() - rusty::len((*this));
        }
        void push(char32_t c) {
            this->try_push(std::move(c)).unwrap();
        }
        rusty::Result<std::tuple<>, ::errors::CapacityError<char32_t>> try_push(char32_t c) {
            const auto len = rusty::len((*this));
            // @unsafe
            {
                const auto ptr_shadow1 = rusty::ptr::add(reinterpret_cast<uint8_t*>(rusty::as_mut_ptr((*this))), std::move(len));
                auto remaining_cap = this->capacity() - len;
                return [&]() -> rusty::Result<std::tuple<>, ::errors::CapacityError<char32_t>> { auto&& _m = char_::encode_utf8(std::move(c), std::move(ptr_shadow1), std::move(remaining_cap)); if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& n = rusty::detail::deref_if_pointer(_mv0); return [&]() -> rusty::Result<std::tuple<>, ::errors::CapacityError<char32_t>> { this->set_len(len + n);
return rusty::Result<std::tuple<>, ::errors::CapacityError<char32_t>>::Ok(std::make_tuple()); }(); } if (_m.is_err()) { return rusty::Result<std::tuple<>, ::errors::CapacityError<char32_t>>::Err(::errors::CapacityError<char32_t>::new_(std::move(c))); } return [&]() -> rusty::Result<std::tuple<>, ::errors::CapacityError<char32_t>> { rusty::intrinsics::unreachable(); }(); }();
            }
        }
        void push_str(std::string_view s) {
            this->try_push_str(std::string_view(s)).unwrap();
        }
        rusty::Result<std::tuple<>, ::errors::CapacityError<std::string_view>> try_push_str(std::string_view s) {
            if (rusty::len(s) > (this->capacity() - rusty::len((*this)))) {
                return rusty::Result<std::tuple<>, ::errors::CapacityError<std::string_view>>::Err(::errors::CapacityError<std::string_view>::new_(std::string_view(s)));
            }
            // @unsafe
            {
                const auto dst = rusty::ptr::add(reinterpret_cast<uint8_t*>(rusty::as_mut_ptr((*this))), rusty::len((*this)));
                const auto src = reinterpret_cast<const uint8_t*>(rusty::as_ptr(s));
                rusty::ptr::copy_nonoverlapping(std::move(src), std::move(dst), rusty::len(s));
                auto newl = rusty::len((*this)) + rusty::len(s);
                this->set_len(std::move(newl));
            }
            return rusty::Result<std::tuple<>, ::errors::CapacityError<std::string_view>>::Ok(std::make_tuple());
        }
        rusty::Option<char32_t> pop() {
            auto ch = ({ auto&& _m = rusty::rev(rusty::str_runtime::chars((*this))).next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& ch = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::Option<char32_t>(rusty::None); } std::move(_match_value).value(); });
            auto new_len = rusty::len((*this)) - rusty::char_runtime::len_utf8(ch);
            // @unsafe
            {
                this->set_len(std::move(new_len));
            }
            return rusty::Option<char32_t>(std::move(ch));
        }
        void truncate(size_t new_len) {
            if (new_len <= rusty::len((*this))) {
                if (!rusty::str_runtime::is_char_boundary((*this), std::move(new_len))) {
                    rusty::panicking::panic("assertion failed: self.is_char_boundary(new_len)");
                }
                // @unsafe
                {
                    this->set_len(std::move(new_len));
                }
            }
        }
        char32_t remove(size_t idx) {
            auto ch = [&]() { auto&& _m = rusty::str_runtime::chars(rusty::slice_from((*this), idx)).next(); if (_m.is_some()) { return _m.unwrap(); } if (_m.is_none()) { return [&]() { rusty::panic::begin_panic("cannot remove a char from the end of a string"); }(); } rusty::intrinsics::unreachable(); }();
            const auto next = idx + rusty::char_runtime::len_utf8(ch);
            const auto len = rusty::len((*this));
            const auto ptr_shadow1 = rusty::as_mut_ptr((*this));
            // @unsafe
            {
                rusty::ptr::copy(rusty::ptr::add(ptr_shadow1, std::move(next)), rusty::ptr::add(ptr_shadow1, std::move(idx)), len - next);
                this->set_len(len - ((next - idx)));
            }
            return ch;
        }
        void clear() {
            // @unsafe
            {
                this->set_len(static_cast<size_t>(0));
            }
        }
        void set_len(size_t length) {
            if (true) {
                if (!(length <= this->capacity())) {
                    rusty::panicking::panic("assertion failed: length <= self.capacity()");
                }
            }
            this->len_field = static_cast<LenUint>(length);
        }
        std::string_view as_str() const {
            return this->operator*();
        }
        std::string_view as_mut_str() {
            return this->operator*();
        }
        const uint8_t* as_ptr() const {
            return reinterpret_cast<const uint8_t*>(rusty::as_ptr(this->xs));
        }
        uint8_t* as_mut_ptr() {
            return const_cast<uint8_t*>(reinterpret_cast<const uint8_t*>(rusty::as_mut_ptr(this->xs)));
        }
        std::string_view operator*() const {
            // @unsafe
            {
                const auto sl = rusty::from_raw_parts(rusty::as_ptr((*this)), rusty::len((*this)));
                return rusty::str_runtime::from_utf8_unchecked(std::move(sl));
            }
        }
        std::string_view operator*() {
            // @unsafe
            {
                const auto len = rusty::len((*this));
                auto sl = rusty::from_raw_parts_mut(rusty::as_mut_ptr((*this)), std::move(len));
                return rusty::str_runtime::from_utf8_unchecked_mut(std::move(sl));
            }
        }
        bool operator==(const ArrayString<CAP>& rhs) const {
            return rusty::detail::deref_if_pointer_like((*this)) == rusty::detail::deref_if_pointer_like(rhs);
        }
        bool operator==(std::string_view rhs) const {
            return rusty::detail::deref_if_pointer_like((*this)) == rhs;
        }
        template<typename H>
        void hash(H& h) const {
            rusty::hash::hash(((rusty::detail::deref_if_pointer_like((*this)))), h);
        }
        std::string_view borrow() const {
            return this->operator*();
        }
        std::string_view borrow_mut() {
            return this->operator*();
        }
        std::string_view as_ref() const {
            return this->operator*();
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return ((rusty::detail::deref_if_pointer_like((*this)))).fmt(f);
        }
        rusty::fmt::Result write_char(char32_t c) {
            return this->try_push(std::move(c)).map_err([&](auto _closure_wild0) { return rusty::fmt::Error{}; });
        }
        rusty::fmt::Result write_str(std::string_view s) {
            return this->try_push_str(std::string_view(s)).map_err([&](auto _closure_wild0) { return rusty::fmt::Error{}; });
        }
        ArrayString<CAP> clone() const {
            return {.len_field = rusty::clone(this->len_field), .xs = rusty::clone(this->xs)};
        }
        void clone_from(const ArrayString<CAP>& rhs) {
            this->clear();
            this->try_push_str(rusty::to_string_view(rhs)).ok();
        }
        std::partial_ordering operator<=>(const ArrayString<CAP>& rhs) const {
            return rusty::to_partial_ordering([&]() -> rusty::Option<cmp::Ordering> {
                return rusty::partial_cmp(((rusty::detail::deref_if_pointer_like((*this)))), rusty::detail::deref_if_pointer_like(rhs));
            }());
        }
        std::partial_ordering operator<=>(std::string_view rhs) const {
            return rusty::to_partial_ordering([&]() -> rusty::Option<cmp::Ordering> {
                return rusty::partial_cmp(((rusty::detail::deref_if_pointer_like((*this)))), rhs);
            }());
        }
        cmp::Ordering cmp(const ArrayString<CAP>& rhs) const {
            return rusty::cmp::cmp(((rusty::detail::deref_if_pointer_like((*this)))), rusty::detail::deref_if_pointer_like(rhs));
        }
        static rusty::Result<ArrayString<CAP>, Err> from_str(std::string_view s) {
            return ArrayString<CAP>::from(s).map_err([&](auto&& _err) -> Err { return ([&](auto&& _err) { return std::forward<decltype(_err)>(_err).simplify(); }) (std::forward<decltype(_err)>(_err)); });
        }
        static rusty::Result<ArrayString<CAP>, Error> try_from(std::string_view f) {
            auto v = ArrayString<CAP>::new_();
            RUSTY_TRY_INTO(v.try_push_str(std::string_view(f)), rusty::Result<ArrayString<CAP>, Error>);
            return rusty::Result<ArrayString<CAP>, Error>::Ok(std::move(v));
        }
        static rusty::Result<ArrayString<CAP>, Error> try_from(rusty::fmt::Arguments f) {
            auto v = ArrayString<CAP>::new_();
            RUSTY_TRY_INTO(rusty::write_fmt(v, std::move(f)).map_err([&](auto&& e) { return CapacityError<std::tuple<>>::new_(std::move(e)); }), rusty::Result<ArrayString<CAP>, Error>);
            return rusty::Result<ArrayString<CAP>, Error>::Ok(std::move(v));
        }
    };

    // Extension trait PartialEq lowered to rusty_ext:: free functions
    namespace rusty_ext {
        template<size_t CAP>
        bool eq(const std::string_view& self_, const ArrayString<CAP>& rhs) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return self_ == rusty::detail::deref_if_pointer_like(rhs);
        }

    }

    // Extension trait PartialOrd lowered to rusty_ext:: free functions
    namespace rusty_ext {
        template<size_t CAP>
        rusty::Option<cmp::Ordering> partial_cmp(const std::string_view& self_, const ArrayString<CAP>& rhs) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return rusty::partial_cmp(self_, rusty::detail::deref_if_pointer_like(rhs));
        }

        template<size_t CAP>
        bool lt(const std::string_view& self_, const ArrayString<CAP>& rhs) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return self_ < rusty::detail::deref_if_pointer_like(rhs);
        }

        template<size_t CAP>
        bool le(const std::string_view& self_, const ArrayString<CAP>& rhs) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return self_ <= rusty::detail::deref_if_pointer_like(rhs);
        }

        template<size_t CAP>
        bool gt(const std::string_view& self_, const ArrayString<CAP>& rhs) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return self_ > rusty::detail::deref_if_pointer_like(rhs);
        }

        template<size_t CAP>
        bool ge(const std::string_view& self_, const ArrayString<CAP>& rhs) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return self_ >= rusty::detail::deref_if_pointer_like(rhs);
        }

    }


}

using array_string::ArrayString;

using errors::CapacityError;

using arrayvec::ArrayVec;
using arrayvec::IntoIter;
using arrayvec::Drain;

// Rust-only libtest main omitted

namespace char_ {

    /// Encode a char into buf using UTF-8.
    ///
    /// On success, return the byte length of the encoding (1, 2, 3 or 4).<br>
    /// On error, return `EncodeUtf8Error` if the buffer was too short for the char.
    ///
    /// Safety: `ptr` must be writable for `len` bytes.
    // @unsafe
    rusty::Result<size_t, EncodeUtf8Error> encode_utf8(char32_t ch, uint8_t* ptr, size_t len) {
        const auto code = static_cast<uint32_t>(ch);
        if ((code < MAX_ONE_B) && (len >= 1)) {
            rusty::ptr::write(rusty::ptr::add(ptr, 0), std::move(static_cast<uint8_t>(code)));
            return rusty::Result<size_t, EncodeUtf8Error>::Ok(static_cast<size_t>(1));
        } else if ((code < MAX_TWO_B) && (len >= 2)) {
            rusty::ptr::write(rusty::ptr::add(ptr, 0), std::move((static_cast<uint8_t>(((code >> 6) & 31))) | TAG_TWO_B));
            rusty::ptr::write(rusty::ptr::add(ptr, 1), std::move((static_cast<uint8_t>((code & 63))) | TAG_CONT));
            return rusty::Result<size_t, EncodeUtf8Error>::Ok(static_cast<size_t>(2));
        } else if ((code < MAX_THREE_B) && (len >= 3)) {
            rusty::ptr::write(rusty::ptr::add(ptr, 0), std::move((static_cast<uint8_t>(((code >> 12) & 15))) | TAG_THREE_B));
            rusty::ptr::write(rusty::ptr::add(ptr, 1), std::move((static_cast<uint8_t>(((code >> 6) & 63))) | TAG_CONT));
            rusty::ptr::write(rusty::ptr::add(ptr, 2), std::move((static_cast<uint8_t>((code & 63))) | TAG_CONT));
            return rusty::Result<size_t, EncodeUtf8Error>::Ok(static_cast<size_t>(3));
        } else if (len >= 4) {
            rusty::ptr::write(rusty::ptr::add(ptr, 0), std::move((static_cast<uint8_t>(((code >> 18) & 7))) | TAG_FOUR_B));
            rusty::ptr::write(rusty::ptr::add(ptr, 1), std::move((static_cast<uint8_t>(((code >> 12) & 63))) | TAG_CONT));
            rusty::ptr::write(rusty::ptr::add(ptr, 2), std::move((static_cast<uint8_t>(((code >> 6) & 63))) | TAG_CONT));
            rusty::ptr::write(rusty::ptr::add(ptr, 3), std::move((static_cast<uint8_t>((code & 63))) | TAG_CONT));
            return rusty::Result<size_t, EncodeUtf8Error>::Ok(static_cast<size_t>(4));
        }
        return rusty::Result<size_t, EncodeUtf8Error>::Err(EncodeUtf8Error{});
    }

    void test_encode_utf8() {
        auto data = rusty::array_repeat(static_cast<uint8_t>(0), 16);
        for (auto&& codepoint : rusty::for_in(rusty::range_inclusive(0, (static_cast<uint32_t>(static_cast<char32_t>(0x10FFFF)))))) {
            if (auto&& _iflet_scrutinee = rusty::char_runtime::from_u32(std::move(codepoint)); _iflet_scrutinee.is_some()) {
                decltype(auto) ch = _iflet_scrutinee.unwrap();
                for (auto&& elt : rusty::for_in(rusty::iter(data))) {
                    rusty::detail::deref_if_pointer_like(elt) = 0;
                }
                const auto ptr_shadow1 = rusty::as_mut_ptr(data);
                auto len = rusty::len(data);
                // @unsafe
                {
                    const auto res = encode_utf8(std::move(ch), std::move(ptr_shadow1), std::move(len)).ok().unwrap();
                    {
                        auto _m0 = &res;
                        auto&& _m1_tmp = rusty::char_runtime::len_utf8(ch);
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
                const auto string = rusty::str_runtime::from_utf8(data).unwrap();
                {
                    auto&& _m0_tmp = rusty::str_runtime::chars(string).next();
                    auto _m0 = &_m0_tmp;
                    auto&& _m1_tmp = rusty::Some(std::move(ch));
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

    void test_encode_utf8_oob() {
        auto data = rusty::array_repeat(static_cast<uint8_t>(0), 16);
        const auto chars = std::array{U'a', U'\u03B1', U'\uFFFD', U'\U00010348'};
        for (auto&& _for_item : rusty::for_in(rusty::zip((rusty::range_inclusive(1, 4)), chars))) {
            auto&& len = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_for_item)));
            auto&& ch = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_for_item))));
            {
                auto&& _m0_tmp = len;
                auto _m0 = &_m0_tmp;
                auto&& _m1_tmp = rusty::char_runtime::len_utf8(ch);
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::Some(std::format("Len of ch={0}", rusty::to_string(ch))));
                    }
                    _m_matched = true;
                }
            }
            const auto ptr_shadow1 = rusty::as_mut_ptr(data);
            // @unsafe
            {
                if (![&]() -> bool { auto&& _m = encode_utf8(std::move(ch), std::move(ptr_shadow1), len - 1); if (_m.is_err()) { return true; } if (true) { return false; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }()) {
                    rusty::panicking::panic("assertion failed: matches::matches!(encode_utf8(ch, ptr, len - 1), Err(_))");
                }
                if (![&]() -> bool { auto&& _m = encode_utf8(std::move(ch), std::move(ptr_shadow1), std::move(len)); if (_m.is_ok()) { return true; } if (true) { return false; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }()) {
                    rusty::panicking::panic("assertion failed: matches::matches!(encode_utf8(ch, ptr, len), Ok(_))");
                }
            }
        }
    }

}

namespace arrayvec {

    void extend_panic() {
        {
            rusty::panic::begin_panic("ArrayVec: capacity exceeded in extend/from_iter");
        }
    }

    /// Rawptr add but uses arithmetic distance for ZST
    // @unsafe
    template<typename T>
    std::add_pointer_t<T> raw_ptr_add(std::add_pointer_t<T> ptr, size_t offset) {
        if (rusty::mem::size_of<T>() == 0) {
            return reinterpret_cast<std::add_pointer_t<T>>(rusty::ptr::add(reinterpret_cast<uint8_t*>(ptr), std::move(offset)));
        } else {
            return rusty::ptr::add(ptr, std::move(offset));
        }
    }

}


// Runnable wrappers for expanded Rust test bodies
// Rust-only libtest wrapper metadata: marker=char::test_encode_utf8 should_panic=no
void rusty_test_char_test_encode_utf8() {
    char_::test_encode_utf8();
}
// Rust-only libtest wrapper metadata: marker=char::test_encode_utf8_oob should_panic=no
void rusty_test_char_test_encode_utf8_oob() {
    char_::test_encode_utf8_oob();
}

// ── from tests.cppm ──




















































void test_simple();
void test_capacity_left();
void test_extend_from_slice();
void test_extend_from_slice_error();
void test_try_from_slice_error();
void test_u16_index();
void test_iter();
void test_drop();
void test_drop_panics();
void test_extend();
void test_extend_capacity_panic_1();
void test_extend_capacity_panic_2();
void test_is_send_sync();
void test_compact_size();
void test_still_works_with_option_arrayvec();
void test_drain();
void test_drain_range_inclusive();
void test_drain_range_inclusive_oob();
void test_retain();
void test_drain_oob();
void test_drop_panic();
void test_drop_panic_into_iter();
void test_insert();
void test_into_inner_1();
void test_into_inner_2();
void test_into_inner_3();
void test_take();
void test_write();
void array_clone_from();
void test_string();
void test_string_from();
void test_string_parse_from_str();
void test_string_from_bytes();
void test_string_clone();
void test_string_push();
void test_insert_at_length();
void test_insert_out_of_bounds();
void test_drop_in_insert();
void test_pop_at();
void test_sizes();
void test_default();
void test_extend_zst();
void test_try_from_argument();
void allow_max_capacity_arrayvec_type();
void deny_max_capacity_arrayvec_value();
void deny_max_capacity_arrayvec_value_const();
void test_arrayvec_const_constructible();
void test_arraystring_const_constructible();
void test_arraystring_zero_filled_has_some_sanity_checks();


// Rust-only unresolved import: using crate::ArrayVec;

// Rust-only unresolved import: using crate::ArrayString;

namespace mem = rusty::mem;

// Rust-only unresolved import: using crate::CapacityError;

using ::rusty::HashMap;

// Rust-only libtest metadata const skipped: test_simple (marker: test_simple, should_panic: no)

// Rust-only libtest metadata const skipped: test_capacity_left (marker: test_capacity_left, should_panic: no)

// Rust-only libtest metadata const skipped: test_extend_from_slice (marker: test_extend_from_slice, should_panic: no)

// Rust-only libtest metadata const skipped: test_extend_from_slice_error (marker: test_extend_from_slice_error, should_panic: no)

// Rust-only libtest metadata const skipped: test_try_from_slice_error (marker: test_try_from_slice_error, should_panic: no)

// Rust-only libtest metadata const skipped: test_u16_index (marker: test_u16_index, should_panic: no)

// Rust-only libtest metadata const skipped: test_iter (marker: test_iter, should_panic: no)

// Rust-only libtest metadata const skipped: test_drop (marker: test_drop, should_panic: no)

// Rust-only libtest metadata const skipped: test_drop_panics (marker: test_drop_panics, should_panic: no)

// Rust-only libtest metadata const skipped: test_extend (marker: test_extend, should_panic: no)

// Rust-only libtest metadata const skipped: test_extend_capacity_panic_1 (marker: test_extend_capacity_panic_1, should_panic: yes)

// Rust-only libtest metadata const skipped: test_extend_capacity_panic_2 (marker: test_extend_capacity_panic_2, should_panic: yes)

// Rust-only libtest metadata const skipped: test_is_send_sync (marker: test_is_send_sync, should_panic: no)

// Rust-only libtest metadata const skipped: test_compact_size (marker: test_compact_size, should_panic: no)

// Rust-only libtest metadata const skipped: test_still_works_with_option_arrayvec (marker: test_still_works_with_option_arrayvec, should_panic: no)

// Rust-only libtest metadata const skipped: test_drain (marker: test_drain, should_panic: no)

// Rust-only libtest metadata const skipped: test_drain_range_inclusive (marker: test_drain_range_inclusive, should_panic: no)

// Rust-only libtest metadata const skipped: test_drain_range_inclusive_oob (marker: test_drain_range_inclusive_oob, should_panic: yes)

// Rust-only libtest metadata const skipped: test_retain (marker: test_retain, should_panic: no)

// Rust-only libtest metadata const skipped: test_drain_oob (marker: test_drain_oob, should_panic: yes)

// Rust-only libtest metadata const skipped: test_drop_panic (marker: test_drop_panic, should_panic: yes)

// Rust-only libtest metadata const skipped: test_drop_panic_into_iter (marker: test_drop_panic_into_iter, should_panic: yes)

// Rust-only libtest metadata const skipped: test_insert (marker: test_insert, should_panic: no)

// Rust-only libtest metadata const skipped: test_into_inner_1 (marker: test_into_inner_1, should_panic: no)

// Rust-only libtest metadata const skipped: test_into_inner_2 (marker: test_into_inner_2, should_panic: no)

// Rust-only libtest metadata const skipped: test_into_inner_3 (marker: test_into_inner_3, should_panic: no)

// Rust-only libtest metadata const skipped: test_take (marker: test_take, should_panic: no)

// Rust-only libtest metadata const skipped: test_write (marker: test_write, should_panic: no)

// Rust-only libtest metadata const skipped: array_clone_from (marker: array_clone_from, should_panic: no)

// Rust-only libtest metadata const skipped: test_string (marker: test_string, should_panic: no)

// Rust-only libtest metadata const skipped: test_string_from (marker: test_string_from, should_panic: no)

// Rust-only libtest metadata const skipped: test_string_parse_from_str (marker: test_string_parse_from_str, should_panic: no)

// Rust-only libtest metadata const skipped: test_string_from_bytes (marker: test_string_from_bytes, should_panic: no)

// Rust-only libtest metadata const skipped: test_string_clone (marker: test_string_clone, should_panic: no)

// Rust-only libtest metadata const skipped: test_string_push (marker: test_string_push, should_panic: no)

// Rust-only libtest metadata const skipped: test_insert_at_length (marker: test_insert_at_length, should_panic: no)

// Rust-only libtest metadata const skipped: test_insert_out_of_bounds (marker: test_insert_out_of_bounds, should_panic: yes)

// Rust-only libtest metadata const skipped: test_drop_in_insert (marker: test_drop_in_insert, should_panic: no)

// Rust-only libtest metadata const skipped: test_pop_at (marker: test_pop_at, should_panic: no)

// Rust-only libtest metadata const skipped: test_sizes (marker: test_sizes, should_panic: no)

// Rust-only libtest metadata const skipped: test_default (marker: test_default, should_panic: no)

// Rust-only libtest metadata const skipped: test_extend_zst (marker: test_extend_zst, should_panic: no)

// Rust-only libtest metadata const skipped: test_try_from_argument (marker: test_try_from_argument, should_panic: no)

// Rust-only libtest metadata const skipped: allow_max_capacity_arrayvec_type (marker: allow_max_capacity_arrayvec_type, should_panic: no)

// Rust-only libtest metadata const skipped: deny_max_capacity_arrayvec_value (marker: deny_max_capacity_arrayvec_value, should_panic: yes)

// Rust-only libtest metadata const skipped: deny_max_capacity_arrayvec_value_const (marker: deny_max_capacity_arrayvec_value_const, should_panic: yes)

// Rust-only libtest metadata const skipped: test_arrayvec_const_constructible (marker: test_arrayvec_const_constructible, should_panic: no)

// Rust-only libtest metadata const skipped: test_arraystring_const_constructible (marker: test_arraystring_const_constructible, should_panic: no)

// Rust-only libtest metadata const skipped: test_arraystring_zero_filled_has_some_sanity_checks (marker: test_arraystring_zero_filled_has_some_sanity_checks, should_panic: no)

void test_simple() {
    ArrayVec<rusty::Vec<int32_t>, 3> vec = ArrayVec<rusty::Vec<int32_t>, 3>::new_();
    vec.push(rusty::boxed::into_vec(rusty::boxed::box_new(std::array{static_cast<int32_t>(1), static_cast<int32_t>(2), static_cast<int32_t>(3), static_cast<int32_t>(4)})));
    vec.push(rusty::boxed::into_vec(rusty::boxed::box_new(std::array{static_cast<int32_t>(10)})));
    vec.push(rusty::boxed::into_vec(rusty::boxed::box_new(std::array{-1, static_cast<int32_t>(13), -2})));
    for (auto&& elt : rusty::for_in(rusty::iter(vec))) {
        {
            auto&& _m0_tmp = rusty::fold(rusty::iter(elt), static_cast<int32_t>(0), rusty::ops::add_fn);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(10);
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
    const auto sum_len = rusty::fold(rusty::map(vec.into_iter(), [&](auto&& x) { return rusty::len(x); }), 0, rusty::ops::add_fn);
    {
        auto _m0 = &sum_len;
        auto&& _m1_tmp = static_cast<int32_t>(8);
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

void test_capacity_left() {
    ArrayVec<size_t, 4> vec = ArrayVec<size_t, 4>::new_();
    {
        auto&& _m0_tmp = vec.remaining_capacity();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = static_cast<int32_t>(4);
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
    vec.push(static_cast<size_t>(1));
    {
        auto&& _m0_tmp = vec.remaining_capacity();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = static_cast<int32_t>(3);
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
    vec.push(static_cast<size_t>(2));
    {
        auto&& _m0_tmp = vec.remaining_capacity();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = static_cast<int32_t>(2);
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
    vec.push(static_cast<size_t>(3));
    {
        auto&& _m0_tmp = vec.remaining_capacity();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = static_cast<int32_t>(1);
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
    vec.push(static_cast<size_t>(4));
    {
        auto&& _m0_tmp = vec.remaining_capacity();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = static_cast<int32_t>(0);
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

void test_extend_from_slice() {
    ArrayVec<size_t, 10> vec = ArrayVec<size_t, 10>::new_();
    vec.try_extend_from_slice([&]() -> std::span<const size_t> { static const std::array<size_t, 3> _slice_ref_tmp = {static_cast<size_t>(1), static_cast<size_t>(2), static_cast<size_t>(3)}; return std::span<const size_t>(_slice_ref_tmp); }()).unwrap();
    {
        auto&& _m0_tmp = rusty::len(vec);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = static_cast<int32_t>(3);
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
        auto&& _m0_tmp = rusty::slice_full(vec);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<size_t, 3>{static_cast<size_t>(1), static_cast<size_t>(2), static_cast<size_t>(3)};
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
        auto&& _m0_tmp = vec.pop();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::Option<int32_t>(static_cast<int32_t>(3));
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
        auto&& _m0_tmp = rusty::slice_full(vec);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<size_t, 2>{static_cast<size_t>(1), static_cast<size_t>(2)};
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

void test_extend_from_slice_error() {
    ArrayVec<size_t, 10> vec = ArrayVec<size_t, 10>::new_();
    vec.try_extend_from_slice([&]() -> std::span<const size_t> { static const std::array<size_t, 3> _slice_ref_tmp = {static_cast<size_t>(1), static_cast<size_t>(2), static_cast<size_t>(3)}; return std::span<const size_t>(_slice_ref_tmp); }()).unwrap();
    const auto res = vec.try_extend_from_slice([&]() -> std::span<const size_t> { static const auto _slice_ref_tmp = rusty::array_repeat(static_cast<size_t>(0), 8); return std::span<const size_t>(_slice_ref_tmp); }());
    {
        auto&& _m = res;
        bool _m_matched = false;
        if (!_m_matched) {
            if (_m.is_err()) {
                _m_matched = true;
            }
        }
        if (!_m_matched) {
            if (true) {
                const auto& e = _m;
                rusty::panicking::panic_fmt(std::format("assertion failed: `{0}` does not match `{1}`", rusty::to_debug_string(e), rusty::to_string("Err(_)")));
                _m_matched = true;
            }
        }
    }
    ArrayVec<size_t, 0> vec_shadow1 = ArrayVec<size_t, 0>::new_();
    const auto res_shadow1 = vec_shadow1.try_extend_from_slice([&]() -> std::span<const size_t> { static const auto _slice_ref_tmp = rusty::array_repeat(static_cast<size_t>(0), 1); return std::span<const size_t>(_slice_ref_tmp); }());
    {
        auto&& _m = res_shadow1;
        bool _m_matched = false;
        if (!_m_matched) {
            if (_m.is_err()) {
                _m_matched = true;
            }
        }
        if (!_m_matched) {
            if (true) {
                const auto& e = _m;
                rusty::panicking::panic_fmt(std::format("assertion failed: `{0}` does not match `{1}`", rusty::to_debug_string(e), rusty::to_string("Err(_)")));
                _m_matched = true;
            }
        }
    }
}

void test_try_from_slice_error() {
    // Rust-only unresolved import: using crate::ArrayVec;
    const auto res = ArrayVec<int32_t, 2>::try_from(std::array{1, 2, 3});
    {
        auto&& _m = res;
        bool _m_matched = false;
        if (!_m_matched) {
            if (_m.is_err()) {
                _m_matched = true;
            }
        }
        if (!_m_matched) {
            if (true) {
                const auto& e = _m;
                rusty::panicking::panic_fmt(std::format("assertion failed: `{0}` does not match `{1}`", rusty::to_debug_string(e), rusty::to_string("Err(_)")));
                _m_matched = true;
            }
        }
    }
}

void test_u16_index() {
    constexpr size_t N = static_cast<size_t>(4096);
    ArrayVec<uint8_t, N> vec = ArrayVec<uint8_t, N>::new_();
    for (auto&& _ : rusty::for_in(rusty::range(0, N))) {
        if (!vec.try_push(static_cast<uint8_t>(1)).is_ok()) {
            rusty::panicking::panic("assertion failed: vec.try_push(1u8).is_ok()");
        }
    }
    if (!vec.try_push(static_cast<uint8_t>(0)).is_err()) {
        rusty::panicking::panic("assertion failed: vec.try_push(0).is_err()");
    }
    {
        auto&& _m0_tmp = rusty::len(vec);
        auto _m0 = &_m0_tmp;
        auto _m1 = &N;
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

void test_iter() {
    auto iter = ArrayVec<int32_t, 3>::from(std::array{1, 2, 3}).into_iter();
    {
        auto&& _m0_tmp = iter.size_hint();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::make_tuple(3, rusty::Option<int32_t>(3));
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
        auto&& _m0_tmp = iter.next_back();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::Option<int32_t>(static_cast<int32_t>(3));
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
        auto&& _m0_tmp = iter.next();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::Option<int32_t>(static_cast<int32_t>(1));
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
        auto&& _m0_tmp = iter.next_back();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::Option<int32_t>(static_cast<int32_t>(2));
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
        auto&& _m0_tmp = iter.size_hint();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::make_tuple(0, rusty::Option<int32_t>(0));
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
        auto&& _m0_tmp = iter.next_back();
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
}

void test_drop() {
    using ::rusty::Cell;
    struct Bump {
        const rusty::Cell<int32_t>& _0;
        Bump(const rusty::Cell<int32_t>& _0_init) : _0(_0_init) {}
        Bump(const Bump&) = default;
        Bump(Bump&& other) noexcept : _0(other._0) {
            if (rusty::mem::consume_forgotten_address(&other)) {
                this->rusty_mark_forgotten();
                other.rusty_mark_forgotten();
            } else {
                other.rusty_mark_forgotten();
            }
        }
        Bump& operator=(const Bump&) = default;
        Bump& operator=(Bump&& other) noexcept {
            if (this == &other) {
                return *this;
            }
            this->~Bump();
            new (this) Bump(std::move(other));
            return *this;
        }
        void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


        ~Bump() noexcept(false) {
            if (rusty::mem::consume_forgotten_address(this)) { return; }
            const auto n = this->_0.get();
            this->_0.set(n + 1);
        }
    };
    const auto& flag = Cell<int32_t>::new_(static_cast<int32_t>(0));
    // Rust-only nested impl block skipped in local scope
    // Rust-only nested impl block skipped in local scope
    {
        auto array = ArrayVec<Bump, 128>::new_();
        array.push(Bump(flag));
        array.push(Bump(flag));
    }
    {
        auto&& _m0_tmp = flag.get();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = static_cast<int32_t>(2);
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
    flag.set(0);
    {
        auto array = ArrayVec<rusty::Vec<Bump>, 3>::new_();
        array.push(rusty::boxed::into_vec(rusty::boxed::box_new(std::array{Bump(flag)})));
        array.push(rusty::boxed::into_vec(rusty::boxed::box_new(std::array{Bump(flag), Bump(flag)})));
        array.push(rusty::Vec<Bump>::new_());
        auto push4 = array.try_push(rusty::boxed::into_vec(rusty::boxed::box_new(std::array{Bump(flag)})));
        {
            auto&& _m0_tmp = flag.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
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
        rusty::mem::drop(std::move(push4));
        {
            auto&& _m0_tmp = flag.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
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
        rusty::mem::drop(array.pop());
        {
            auto&& _m0_tmp = flag.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
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
        rusty::mem::drop(array.pop());
        {
            auto&& _m0_tmp = flag.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(3);
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
    {
        auto&& _m0_tmp = flag.get();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = static_cast<int32_t>(4);
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
    flag.set(0);
    {
        auto array = ArrayVec<Bump, 3>::new_();
        array.push(Bump(flag));
        array.push(Bump(flag));
        array.push(Bump(flag));
        auto inner = array.into_inner();
        if (!inner.is_ok()) {
            rusty::panicking::panic("assertion failed: inner.is_ok()");
        }
        {
            auto&& _m0_tmp = flag.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
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
        rusty::mem::drop(std::move(inner));
        {
            auto&& _m0_tmp = flag.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(3);
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
    flag.set(0);
    {
        auto array1 = ArrayVec<Bump, 3>::new_();
        array1.push(Bump(flag));
        array1.push(Bump(flag));
        array1.push(Bump(flag));
        auto array2 = array1.take();
        {
            auto&& _m0_tmp = flag.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
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
        rusty::mem::drop(std::move(array1));
        {
            auto&& _m0_tmp = flag.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
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
        rusty::mem::drop(std::move(array2));
        {
            auto&& _m0_tmp = flag.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(3);
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
    flag.set(0);
    {
        auto array = ArrayVec<Bump, 3>::new_();
        array.push(Bump(flag));
        array.push(Bump(flag));
        array.push(Bump(flag));
        auto iter = array.into_iter();
        {
            auto&& _m0_tmp = flag.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
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
        iter.next();
        {
            auto&& _m0_tmp = flag.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
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
        auto clone = rusty::clone(iter);
        {
            auto&& _m0_tmp = flag.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
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
        rusty::mem::drop(std::move(clone));
        {
            auto&& _m0_tmp = flag.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(3);
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
        rusty::mem::drop(std::move(iter));
        {
            auto&& _m0_tmp = flag.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(5);
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

void test_drop_panics() {
    using ::rusty::Cell;
    using ::rusty::panic::catch_unwind;
    using ::rusty::panic::AssertUnwindSafe;
    struct Bump {
        const rusty::Cell<int32_t>& _0;
        Bump(const rusty::Cell<int32_t>& _0_init) : _0(_0_init) {}
        Bump(const Bump&) = default;
        Bump(Bump&& other) noexcept : _0(other._0) {
            if (rusty::mem::consume_forgotten_address(&other)) {
                this->rusty_mark_forgotten();
                other.rusty_mark_forgotten();
            } else {
                other.rusty_mark_forgotten();
            }
        }
        Bump& operator=(const Bump&) = default;
        Bump& operator=(Bump&& other) noexcept {
            if (this == &other) {
                return *this;
            }
            this->~Bump();
            new (this) Bump(std::move(other));
            return *this;
        }
        void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


        ~Bump() noexcept(false) {
            if (rusty::mem::consume_forgotten_address(this)) { return; }
            const auto n = this->_0.get();
            this->_0.set(n + 1);
            if (n == 0) {
                {
                    rusty::panic::begin_panic("Panic in Bump's drop");
                }
            }
        }
    };
    const auto& flag = Cell<int32_t>::new_(static_cast<int32_t>(0));
    // Rust-only nested impl block skipped in local scope
    flag.set(0);
    {
        auto array = rusty::boxed::into_vec(rusty::boxed::box_new(std::array{Bump(flag), Bump(flag)}));
        const auto res = catch_unwind(AssertUnwindSafe([&]() {
rusty::mem::drop(std::move(array));
}));
        if (!res.is_err()) {
            rusty::panicking::panic("assertion failed: res.is_err()");
        }
    }
    if (flag.get() != 2) {
        {
            rusty::io::_print(std::string("test_drop_panics: skip, this version of Rust doesn't continue in drop_in_place\n"));
        }
        return;
    }
    flag.set(0);
    {
        auto array = ArrayVec<Bump, 128>::new_();
        array.push(Bump(flag));
        array.push(Bump(flag));
        array.push(Bump(flag));
        const auto res = catch_unwind(AssertUnwindSafe([&]() {
rusty::mem::drop(std::move(array));
}));
        if (!res.is_err()) {
            rusty::panicking::panic("assertion failed: res.is_err()");
        }
    }
    {
        auto&& _m0_tmp = flag.get();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = static_cast<int32_t>(3);
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
    flag.set(0);
    {
        auto array = ArrayVec<Bump, 16>::new_();
        array.push(Bump(flag));
        array.push(Bump(flag));
        array.push(Bump(flag));
        array.push(Bump(flag));
        array.push(Bump(flag));
        const auto i = 2;
        const auto tail_len = rusty::len(array) - i;
        const auto res = catch_unwind(AssertUnwindSafe([&]() {
array.truncate(std::move(i));
}));
        if (!res.is_err()) {
            rusty::panicking::panic("assertion failed: res.is_err()");
        }
        {
            auto&& _m0_tmp = flag.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(tail_len);
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

void test_extend() {
    auto range = rusty::range(0, 10);
    auto array = ArrayVec<int32_t, 5>::from_iter(rusty::take(range, 5));
    {
        auto&& _m0_tmp = rusty::slice_full(array);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array{0, 1, 2, 3, 4};
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
        auto&& _m0_tmp = range.next();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::Option<int32_t>(static_cast<int32_t>(5));
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
    array.extend(rusty::take(range, 0));
    {
        auto&& _m0_tmp = range.next();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::Option<int32_t>(static_cast<int32_t>(6));
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
    auto array_shadow1 = ArrayVec<int32_t, 10>::from_iter((rusty::range(0, 3)));
    {
        auto&& _m0_tmp = rusty::slice_full(array_shadow1);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array{0, 1, 2};
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
    array_shadow1.extend(rusty::range(3, 5));
    {
        auto&& _m0_tmp = rusty::slice_full(array_shadow1);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array{0, 1, 2, 3, 4};
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

void test_extend_capacity_panic_1() {
    auto range = rusty::range(0, 10);
    static_cast<void>(ArrayVec<int32_t, 5>::from_iter(range));
}

void test_extend_capacity_panic_2() {
    auto range = rusty::range(0, 10);
    auto array = ArrayVec<int32_t, 5>::from_iter(rusty::take(range, 5));
    {
        auto&& _m0_tmp = rusty::slice_full(array);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array{0, 1, 2, 3, 4};
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
        auto&& _m0_tmp = range.next();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::Option<int32_t>(static_cast<int32_t>(5));
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
    array.extend(rusty::take(range, 1));
}

void test_is_send_sync() {
    const auto data = ArrayVec<rusty::Vec<int32_t>, 5>::new_();
    static_cast<const void*>(&data);
    static_cast<const void*>(&data);
}

void test_compact_size() {
    using ByteArray [[maybe_unused]] = ArrayVec<uint8_t, 4>;
    using EmptyArray [[maybe_unused]] = ArrayVec<uint8_t, 0>;
    using QuadArray [[maybe_unused]] = ArrayVec<uint32_t, 3>;
    {
        rusty::io::_print(std::format("{0}\n", rusty::to_string(rusty::mem::size_of<ByteArray>())));
    }
    if (!(rusty::mem::size_of<ByteArray>() <= (2 * rusty::mem::size_of<uint32_t>()))) {
        rusty::panicking::panic("assertion failed: mem::size_of::<ByteArray>() <= 2 * mem::size_of::<u32>()");
    }
    {
        rusty::io::_print(std::format("{0}\n", rusty::to_string(rusty::mem::size_of<EmptyArray>())));
    }
    if (!(rusty::mem::size_of<EmptyArray>() <= rusty::mem::size_of<uint32_t>())) {
        rusty::panicking::panic("assertion failed: mem::size_of::<EmptyArray>() <= mem::size_of::<u32>()");
    }
    {
        rusty::io::_print(std::format("{0}\n", rusty::to_string(rusty::mem::size_of<QuadArray>())));
    }
    if (!(rusty::mem::size_of<QuadArray>() <= ((4 * 4) + rusty::mem::size_of<uint32_t>()))) {
        rusty::panicking::panic("assertion failed: mem::size_of::<QuadArray>() <= 4 * 4 + mem::size_of::<u32>()");
    }
}

void test_still_works_with_option_arrayvec() {
    using RefArray [[maybe_unused]] = ArrayVec<const int32_t&, 2>;
    const auto array = rusty::Some(RefArray::new_());
    if (!array.is_some()) {
        rusty::panicking::panic("assertion failed: array.is_some()");
    }
    {
        rusty::io::_print(std::format("{0}\n", rusty::to_debug_string(array)));
    }
}

void test_drain() {
    auto v = ArrayVec<int32_t, 8>::from([](auto _seed) { std::array<int32_t, 8> _repeat{}; _repeat.fill(static_cast<int32_t>(_seed)); return _repeat; }(0));
    v.pop();
    v.drain(rusty::range(0, 7));
    {
        auto&& _m0_tmp = rusty::slice_full(v);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<int32_t, 0>{};
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
    v.extend(rusty::range(0, 8));
    v.drain(rusty::range(1, 4));
    {
        auto&& _m0_tmp = rusty::slice_full(v);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<int32_t, 5>{static_cast<int32_t>(0), static_cast<int32_t>(4), static_cast<int32_t>(5), static_cast<int32_t>(6), static_cast<int32_t>(7)};
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
    const auto u = ArrayVec<int32_t, 3>::from_iter(rusty::rev(v.drain(rusty::range(1, 4))));
    {
        auto&& _m0_tmp = rusty::slice_full(u);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array{6, 5, 4};
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
        auto&& _m0_tmp = rusty::slice_full(v);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<int32_t, 2>{static_cast<int32_t>(0), static_cast<int32_t>(7)};
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
    v.drain(rusty::range_full());
    {
        auto&& _m0_tmp = rusty::slice_full(v);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<int32_t, 0>{};
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

void test_drain_range_inclusive() {
    auto v = ArrayVec<int32_t, 8>::from([](auto _seed) { std::array<int32_t, 8> _repeat{}; _repeat.fill(static_cast<int32_t>(_seed)); return _repeat; }(0));
    v.drain(rusty::range_inclusive(0, 7));
    {
        auto&& _m0_tmp = rusty::slice_full(v);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<int32_t, 0>{};
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
    v.extend(rusty::range(0, 8));
    v.drain(rusty::range_inclusive(1, 4));
    {
        auto&& _m0_tmp = rusty::slice_full(v);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<int32_t, 4>{static_cast<int32_t>(0), static_cast<int32_t>(5), static_cast<int32_t>(6), static_cast<int32_t>(7)};
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
    const auto u = ArrayVec<int32_t, 3>::from_iter(rusty::rev(v.drain(rusty::range_inclusive(1, 2))));
    {
        auto&& _m0_tmp = rusty::slice_full(u);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array{6, 5};
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
        auto&& _m0_tmp = rusty::slice_full(v);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<int32_t, 2>{static_cast<int32_t>(0), static_cast<int32_t>(7)};
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
    v.drain(rusty::range_full());
    {
        auto&& _m0_tmp = rusty::slice_full(v);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<int32_t, 0>{};
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

void test_drain_range_inclusive_oob() {
    auto v = ArrayVec<int32_t, 0>::from([](auto _seed) { std::array<int32_t, 0> _repeat{}; _repeat.fill(static_cast<int32_t>(_seed)); return _repeat; }(0));
    v.drain(rusty::range_inclusive(0, 0));
}

void test_retain() {
    auto v = ArrayVec<int32_t, 8>::from([](auto _seed) { std::array<int32_t, 8> _repeat{}; _repeat.fill(static_cast<int32_t>(_seed)); return _repeat; }(0));
    for (auto&& _for_item : rusty::for_in(rusty::enumerate(rusty::iter_mut(v)))) {
        auto&& i = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_for_item)));
        auto&& elt = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_for_item)));
        rusty::detail::deref_if_pointer_like(elt) = std::move(i);
    }
    v.retain([&](auto _closure_wild0) { return true; });
    {
        auto&& _m0_tmp = rusty::slice_full(v);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<int32_t, 8>{static_cast<int32_t>(0), static_cast<int32_t>(1), static_cast<int32_t>(2), static_cast<int32_t>(3), static_cast<int32_t>(4), static_cast<int32_t>(5), static_cast<int32_t>(6), static_cast<int32_t>(7)};
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
    v.retain([&](auto&& elt) {
[&]() { static_cast<void>(rusty::deref_mut(elt) /= 2); return std::make_tuple(); }();
return (rusty::deref_mut(elt) % 2) == static_cast<int32_t>(0);
});
    {
        auto&& _m0_tmp = rusty::slice_full(v);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<int32_t, 4>{static_cast<int32_t>(0), static_cast<int32_t>(0), static_cast<int32_t>(2), static_cast<int32_t>(2)};
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
    v.retain([&](auto _closure_wild0) { return false; });
    {
        auto&& _m0_tmp = rusty::slice_full(v);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<int32_t, 0>{};
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

void test_drain_oob() {
    auto v = ArrayVec<int32_t, 8>::from([](auto _seed) { std::array<int32_t, 8> _repeat{}; _repeat.fill(static_cast<int32_t>(_seed)); return _repeat; }(0));
    v.pop();
    v.drain(rusty::range(0, 8));
}

void test_drop_panic() {
    struct DropPanic {
        DropPanic() = default;
        DropPanic(const DropPanic&) = default;
        DropPanic(DropPanic&& other) noexcept {
            if (rusty::mem::consume_forgotten_address(&other)) {
                this->rusty_mark_forgotten();
                other.rusty_mark_forgotten();
            } else {
                other.rusty_mark_forgotten();
            }
        }
        DropPanic& operator=(const DropPanic&) = default;
        DropPanic& operator=(DropPanic&& other) noexcept {
            if (this == &other) {
                return *this;
            }
            this->~DropPanic();
            new (this) DropPanic(std::move(other));
            return *this;
        }
        void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


        ~DropPanic() noexcept(false) {
            if (rusty::mem::consume_forgotten_address(this)) { return; }
            {
                rusty::panic::begin_panic("drop");
            }
        }
    };
    // Rust-only nested impl block skipped in local scope
    auto array = ArrayVec<DropPanic, 1>::new_();
    array.push(std::move(DropPanic{}));
}

void test_drop_panic_into_iter() {
    struct DropPanic {
        DropPanic() = default;
        DropPanic(const DropPanic&) = default;
        DropPanic(DropPanic&& other) noexcept {
            if (rusty::mem::consume_forgotten_address(&other)) {
                this->rusty_mark_forgotten();
                other.rusty_mark_forgotten();
            } else {
                other.rusty_mark_forgotten();
            }
        }
        DropPanic& operator=(const DropPanic&) = default;
        DropPanic& operator=(DropPanic&& other) noexcept {
            if (this == &other) {
                return *this;
            }
            this->~DropPanic();
            new (this) DropPanic(std::move(other));
            return *this;
        }
        void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


        ~DropPanic() noexcept(false) {
            if (rusty::mem::consume_forgotten_address(this)) { return; }
            {
                rusty::panic::begin_panic("drop");
            }
        }
    };
    // Rust-only nested impl block skipped in local scope
    auto array = ArrayVec<DropPanic, 1>::new_();
    array.push(std::move(DropPanic{}));
    array.into_iter();
}

void test_insert() {
    auto v = ArrayVec<int32_t, 0>::from(std::array<int, 0>{});
    {
        auto&& _m = v.try_push(static_cast<int32_t>(1));
        bool _m_matched = false;
        if (!_m_matched) {
            if (_m.is_err()) {
                _m_matched = true;
            }
        }
        if (!_m_matched) {
            if (true) {
                const auto& e = _m;
                rusty::panicking::panic_fmt(std::format("assertion failed: `{0}` does not match `{1}`", rusty::to_debug_string(e), rusty::to_string("Err(_)")));
                _m_matched = true;
            }
        }
    }
    auto v_shadow1 = ArrayVec<int32_t, 3>::new_();
    v_shadow1.insert(0, static_cast<int32_t>(0));
    v_shadow1.insert(1, static_cast<int32_t>(1));
    {
        auto&& _m0_tmp = rusty::slice_full(v_shadow1);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<int32_t, 2>{static_cast<int32_t>(0), static_cast<int32_t>(1)};
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
    v_shadow1.insert(2, static_cast<int32_t>(2));
    {
        auto&& _m0_tmp = rusty::slice_full(v_shadow1);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<int32_t, 3>{static_cast<int32_t>(0), static_cast<int32_t>(1), static_cast<int32_t>(2)};
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
    const auto ret2 = v_shadow1.try_insert(1, static_cast<int32_t>(9));
    {
        auto&& _m0_tmp = rusty::slice_full(v_shadow1);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<int32_t, 3>{static_cast<int32_t>(0), static_cast<int32_t>(1), static_cast<int32_t>(2)};
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
        auto&& _m = ret2;
        bool _m_matched = false;
        if (!_m_matched) {
            if (_m.is_err()) {
                _m_matched = true;
            }
        }
        if (!_m_matched) {
            if (true) {
                const auto& e = _m;
                rusty::panicking::panic_fmt(std::format("assertion failed: `{0}` does not match `{1}`", rusty::to_debug_string(e), rusty::to_string("Err(_)")));
                _m_matched = true;
            }
        }
    }
    auto v_shadow2 = ArrayVec<int32_t, 1>::from(std::array{2});
    {
        auto&& _m = v_shadow2.try_insert(0, static_cast<int32_t>(1));
        bool _m_matched = false;
        if (!_m_matched) {
            if (_m.is_err()) {
                _m_matched = true;
            }
        }
        if (!_m_matched) {
            if (true) {
                const auto& e = _m;
                rusty::panicking::panic_fmt(std::format("assertion failed: `{0}` does not match `{1}`", rusty::to_debug_string(e), rusty::to_string("Err(CapacityError { .. })")));
                _m_matched = true;
            }
        }
    }
    {
        auto&& _m = v_shadow2.try_insert(1, static_cast<int32_t>(1));
        bool _m_matched = false;
        if (!_m_matched) {
            if (_m.is_err()) {
                _m_matched = true;
            }
        }
        if (!_m_matched) {
            if (true) {
                const auto& e = _m;
                rusty::panicking::panic_fmt(std::format("assertion failed: `{0}` does not match `{1}`", rusty::to_debug_string(e), rusty::to_string("Err(CapacityError { .. })")));
                _m_matched = true;
            }
        }
    }
}

void test_into_inner_1() {
    auto v = ArrayVec<int32_t, 2>::from(std::array{1, 2});
    v.pop();
    auto u = rusty::clone(v);
    {
        auto&& _m0_tmp = v.into_inner();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = [&]() { using _ResultCtorCtx = std::remove_cvref_t<decltype((v.into_inner()))>; return _ResultCtorCtx::Err(std::move(u)); }();
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

void test_into_inner_2() {
    auto v = ArrayVec<rusty::String, 4>::new_();
    v.push(rusty::String::from("a"));
    v.push(rusty::String::from("b"));
    v.push(rusty::String::from("c"));
    v.push(rusty::String::from("d"));
    {
        auto&& _m0_tmp = v.into_inner().unwrap();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<rusty::String, 4>{rusty::String::from("a"), rusty::String::from("b"), rusty::String::from("c"), rusty::String::from("d")};
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

void test_into_inner_3() {
    auto v = ArrayVec<int32_t, 4>::new_();
    v.extend(rusty::range_inclusive(1, 4));
    {
        auto&& _m0_tmp = v.into_inner().unwrap();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<int32_t, 4>{static_cast<int32_t>(1), static_cast<int32_t>(2), static_cast<int32_t>(3), static_cast<int32_t>(4)};
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

void test_take() {
    auto v1 = ArrayVec<int32_t, 4>::new_();
    v1.extend(rusty::range_inclusive(1, 4));
    auto v2 = v1.take();
    if (!v1.into_inner().is_err()) {
        rusty::panicking::panic("assertion failed: v1.into_inner().is_err()");
    }
    {
        auto&& _m0_tmp = v2.into_inner().unwrap();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array{1, 2, 3, 4};
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

void test_write() {
    auto v = ArrayVec<uint8_t, 8>::new_();
    rusty::io::write_fmt(((&v)), std::string("\x01\x02\x03")).unwrap();
    {
        auto&& _m0_tmp = rusty::slice_full(v);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<uint8_t, 3>{static_cast<uint8_t>(1), static_cast<uint8_t>(2), static_cast<uint8_t>(3)};
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
    const auto r = v.write_(rusty::slice_full(rusty::array_repeat(static_cast<uint8_t>(9), 16))).unwrap();
    {
        auto _m0 = &r;
        auto&& _m1_tmp = static_cast<int32_t>(5);
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
        auto&& _m0_tmp = rusty::slice_full(v);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<uint8_t, 8>{static_cast<uint8_t>(1), static_cast<uint8_t>(2), static_cast<uint8_t>(3), static_cast<uint8_t>(9), static_cast<uint8_t>(9), static_cast<uint8_t>(9), static_cast<uint8_t>(9), static_cast<uint8_t>(9)};
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

void array_clone_from() {
    auto v = ArrayVec<rusty::Vec<int32_t>, 4>::new_();
    v.push(rusty::boxed::into_vec(rusty::boxed::box_new(std::array{static_cast<int32_t>(1), static_cast<int32_t>(2)})));
    v.push(rusty::boxed::into_vec(rusty::boxed::box_new(std::array{static_cast<int32_t>(3), static_cast<int32_t>(4), static_cast<int32_t>(5)})));
    v.push(rusty::boxed::into_vec(rusty::boxed::box_new(std::array{static_cast<int32_t>(6)})));
    const auto reference = rusty::to_vec(v);
    auto u = ArrayVec<rusty::Vec<int32_t>, 4>::new_();
    u.clone_from(v);
    {
        auto&& _m0_tmp = rusty::slice_full(u);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::slice_full(reference);
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
    auto t = ArrayVec<rusty::Vec<int32_t>, 4>::new_();
    t.push(rusty::boxed::into_vec(rusty::boxed::box_new(std::array{static_cast<int32_t>(97)})));
    t.push(rusty::Vec<int32_t>::new_());
    t.push(rusty::boxed::into_vec(rusty::boxed::box_new(std::array{static_cast<int32_t>(5), static_cast<int32_t>(6), static_cast<int32_t>(2)})));
    t.push(rusty::boxed::into_vec(rusty::boxed::box_new(std::array{static_cast<int32_t>(2)})));
    t.clone_from(v);
    {
        auto&& _m0_tmp = rusty::slice_full(t);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::slice_full(reference);
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
    t.clear();
    t.clone_from(v);
    {
        auto&& _m0_tmp = rusty::slice_full(t);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::slice_full(reference);
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

void test_string() {
    const auto text = std::string_view("hello world");
    auto s = ArrayString<16>::new_();
    s.try_push_str(text).unwrap();
    {
        auto _m0 = &s;
        auto _m1 = &text;
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
        auto&& _m0_tmp = std::string_view(text);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::string_view(s.as_str());
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
    auto map = rusty::HashMap<ArrayString<16>, int32_t>();
    map.insert(std::move(s), 1);
    {
        auto _m0 = rusty::as_ref_ptr(map[text]);
        auto&& _m1_tmp = static_cast<int32_t>(1);
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
    auto t = ArrayString<2>::new_();
    if (!t.try_push_str(text).is_err()) {
        rusty::panicking::panic("assertion failed: t.try_push_str(text).is_err()");
    }
    {
        auto _m0 = &t;
        auto&& _m1_tmp = std::string_view("");
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
    t.push_str("ab");
    std::string_view tmut = std::string_view(t.as_str());
    {
        auto&& _m0_tmp = std::string_view(tmut);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::string_view("ab");
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
    const auto t_shadow1 = [&]() {
auto t_shadow1 = ArrayString<2>::new_();
RUSTY_TRY(t_shadow1.try_push_str(text));
return rusty::Result<decltype((std::make_tuple())), std::remove_cvref_t<decltype(std::declval<std::remove_cvref_t<decltype((t.try_push_str(text)))>>().unwrap_err())>>::Ok(std::make_tuple());
}();
    if (!t_shadow1.is_err()) {
        rusty::panicking::panic("assertion failed: t.is_err()");
    }
}

void test_string_from() {
    const auto text = std::string_view("hello world");
    const auto u = ArrayString<11>::from(text).unwrap();
    {
        auto&& _m0_tmp = rusty::to_string_view(u);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::string_view(text);
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
        auto&& _m0_tmp = rusty::len(u);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::len(text);
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

void test_string_parse_from_str() {
    const auto text = std::string_view("hello world");
    const ArrayString<11> u = ArrayString<11>::from_str(rusty::to_string_view(text)).unwrap();
    {
        auto _m0 = &u;
        auto _m1 = &text;
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
        auto&& _m0_tmp = rusty::len(u);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::len(text);
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

void test_string_from_bytes() {
    const auto text = std::string_view("hello world");
    const auto u = ArrayString<11>::from_byte_string(std::array<uint8_t, 11>{{ 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64 }}).unwrap();
    {
        auto&& _m0_tmp = rusty::to_string_view(u);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::string_view(text);
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
        auto&& _m0_tmp = rusty::len(u);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::len(text);
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

void test_string_clone() {
    const auto text = std::string_view("hi");
    auto s = ArrayString<4>::new_();
    s.push_str("abcd");
    const auto t = ArrayString<4>::from(text).unwrap();
    s.clone_from(t);
    {
        auto _m0 = &t;
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

void test_string_push() {
    const auto text = std::string_view("abcαβγ");
    auto s = ArrayString<8>::new_();
    for (auto&& c : rusty::for_in(rusty::str_runtime::chars(text))) {
        if (auto&& _iflet_scrutinee = s.try_push(std::move(c)); _iflet_scrutinee.is_err()) {
            auto&& _iflet_payload = _iflet_scrutinee.unwrap_err();
            break;
        }
    }
    {
        auto&& _m0_tmp = std::string_view("abcαβ");
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::string_view(s.as_str());
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
    s.push(U'x');
    {
        auto&& _m0_tmp = std::string_view("abcαβx");
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::string_view(s.as_str());
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
    if (!s.try_push(U'x').is_err()) {
        rusty::panicking::panic("assertion failed: s.try_push('x').is_err()");
    }
}

void test_insert_at_length() {
    auto v = ArrayVec<rusty::String, 8>::new_();
    const auto result1 = v.try_insert(0, rusty::String::from("a"));
    const auto result2 = v.try_insert(1, rusty::String::from("b"));
    if (!(result1.is_ok() && result2.is_ok())) {
        rusty::panicking::panic("assertion failed: result1.is_ok() && result2.is_ok()");
    }
    {
        auto&& _m0_tmp = rusty::slice_full(v);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<rusty::String, 2>{rusty::String::from("a"), rusty::String::from("b")};
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

void test_insert_out_of_bounds() {
    auto v = ArrayVec<rusty::String, 8>::new_();
    static_cast<void>(v.try_insert(1, rusty::String::from("test")));
}

void test_drop_in_insert() {
    using ::rusty::Cell;
    struct Bump {
        const rusty::Cell<int32_t>& _0;
        Bump(const rusty::Cell<int32_t>& _0_init) : _0(_0_init) {}
        Bump(const Bump&) = default;
        Bump(Bump&& other) noexcept : _0(other._0) {
            if (rusty::mem::consume_forgotten_address(&other)) {
                this->rusty_mark_forgotten();
                other.rusty_mark_forgotten();
            } else {
                other.rusty_mark_forgotten();
            }
        }
        Bump& operator=(const Bump&) = default;
        Bump& operator=(Bump&& other) noexcept {
            if (this == &other) {
                return *this;
            }
            this->~Bump();
            new (this) Bump(std::move(other));
            return *this;
        }
        void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


        ~Bump() noexcept(false) {
            if (rusty::mem::consume_forgotten_address(this)) { return; }
            const auto n = this->_0.get();
            this->_0.set(n + 1);
        }
    };
    const auto& flag = Cell<int32_t>::new_(static_cast<int32_t>(0));
    // Rust-only nested impl block skipped in local scope
    flag.set(0);
    {
        auto array = ArrayVec<Bump, 2>::new_();
        array.push(Bump(flag));
        array.insert(0, Bump(flag));
        {
            auto&& _m0_tmp = flag.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
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
        auto ret = array.try_insert(1, Bump(flag));
        {
            auto&& _m0_tmp = flag.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
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
            auto&& _m = ret;
            bool _m_matched = false;
            if (!_m_matched) {
                if (_m.is_err()) {
                    _m_matched = true;
                }
            }
            if (!_m_matched) {
                if (true) {
                    const auto& e = _m;
                    rusty::panicking::panic_fmt(std::format("assertion failed: `{0}` does not match `{1}`", rusty::to_debug_string(e), rusty::to_string("Err(_)")));
                    _m_matched = true;
                }
            }
        }
        rusty::mem::drop(std::move(ret));
        {
            auto&& _m0_tmp = flag.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
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
    {
        auto&& _m0_tmp = flag.get();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = static_cast<int32_t>(3);
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

void test_pop_at() {
    auto v = ArrayVec<rusty::String, 4>::new_();
    const auto s = [&](auto&&... _args) -> decltype(auto) { return rusty::String::from(std::forward<decltype(_args)>(_args)...); };
    v.push(s("a"));
    v.push(s("b"));
    v.push(s("c"));
    v.push(s("d"));
    {
        auto&& _m0_tmp = v.pop_at(4);
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
        auto&& _m0_tmp = v.pop_at(1);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::Some(s("b"));
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
        auto&& _m0_tmp = v.pop_at(1);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::Some(s("c"));
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
        auto&& _m0_tmp = v.pop_at(2);
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
        auto&& _m0_tmp = rusty::slice_full(v);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<rusty::String, 2>{rusty::String::from("a"), rusty::String::from("d")};
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

void test_sizes() {
    const auto v = ArrayVec<uint8_t, 1 << 16>::from([](auto _seed) { std::array<uint8_t, 1 << 16> _repeat{}; _repeat.fill(static_cast<uint8_t>(_seed)); return _repeat; }(static_cast<uint8_t>(0)));
    {
        auto&& _m0_tmp = rusty::array_repeat(static_cast<uint8_t>(0), rusty::len(v));
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::slice_full(v);
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

void test_default() {
    const ArrayString<4> s = rusty::default_value<ArrayString<4>>();
    const ArrayVec<rusty::net::TcpStream, 4> v = rusty::default_value<ArrayVec<rusty::net::TcpStream, 4>>();
    {
        auto&& _m0_tmp = rusty::len(s);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = static_cast<int32_t>(0);
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
        auto&& _m0_tmp = rusty::len(v);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = static_cast<int32_t>(0);
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

void test_extend_zst() {
    struct Z {
    };
    auto range = rusty::range(0, 10);
    // Rust-only nested impl block skipped in local scope
    // Rust-only nested impl block skipped in local scope
    // Rust-only nested impl block skipped in local scope
    // Rust-only nested impl block skipped in local scope
    // Rust-only nested impl block skipped in local scope
    // Rust-only nested impl block skipped in local scope
    auto array = ArrayVec<Z, 5>::from_iter(rusty::map(rusty::take(range, 5), [&](auto _closure_wild0) { return Z{}; }));
    {
        auto&& _m0_tmp = rusty::slice_full(array);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::array_repeat(Z{}, 5);
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
        auto&& _m0_tmp = range.next();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::Option<int32_t>(static_cast<int32_t>(5));
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
    array.extend(rusty::map(rusty::take(range, 0), [&](auto _closure_wild0) { return Z{}; }));
    {
        auto&& _m0_tmp = range.next();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::Option<int32_t>(static_cast<int32_t>(6));
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
    auto array_shadow1 = ArrayVec<Z, 10>::from_iter(rusty::map((rusty::range(0, 3)), [&](auto _closure_wild0) { return Z{}; }));
    {
        auto&& _m0_tmp = rusty::slice_full(array_shadow1);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::array_repeat(Z{}, 3);
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
    array_shadow1.extend(rusty::map((rusty::range(3, 5)), [&](auto _closure_wild0) { return Z{}; }));
    {
        auto&& _m0_tmp = rusty::slice_full(array_shadow1);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = rusty::array_repeat(Z{}, 5);
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
        auto&& _m0_tmp = rusty::len(array_shadow1);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = static_cast<int32_t>(5);
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

void test_try_from_argument() {
    const auto v = ArrayString<16>::try_from(std::format("Hello {0}", 123)).unwrap();
    {
        auto&& _m0_tmp = rusty::to_string_view(v);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::string_view("Hello 123");
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

void allow_max_capacity_arrayvec_type() {
    std::optional<ArrayVec<std::tuple<>, std::numeric_limits<size_t>::max()>> _v;
}

void deny_max_capacity_arrayvec_value() {
    if (rusty::mem::size_of<size_t>() <= rusty::mem::size_of<uint32_t>()) {
        {
            rusty::panic::begin_panic("This test does not work on this platform. 'largest supported capacity'");
        }
    }
    const ArrayVec<std::tuple<>, std::numeric_limits<size_t>::max()> _v = ArrayVec<std::tuple<>, std::numeric_limits<size_t>::max()>::new_();
}

void deny_max_capacity_arrayvec_value_const() {
    if (rusty::mem::size_of<size_t>() <= rusty::mem::size_of<uint32_t>()) {
        {
            rusty::panic::begin_panic("This test does not work on this platform. 'index out of bounds'");
        }
    }
    const ArrayVec<std::tuple<>, std::numeric_limits<size_t>::max()> _v = ArrayVec<std::tuple<>, std::numeric_limits<size_t>::max()>::new_const();
}

void test_arrayvec_const_constructible() {
    const auto OF_U8 = []() -> ArrayVec<rusty::Vec<uint8_t>, 10> {
        return ArrayVec<rusty::Vec<uint8_t>, 10>::new_const();
    };
    auto var = OF_U8();
    if (!rusty::is_empty(var)) {
        rusty::panicking::panic("assertion failed: var.is_empty()");
    }
    {
        auto _m0 = &var;
        auto&& _m1_tmp = ArrayVec<rusty::Vec<uint8_t>, 10>::new_();
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
    var.push(rusty::boxed::into_vec(rusty::boxed::box_new(std::array{static_cast<uint8_t>(3), static_cast<uint8_t>(5), static_cast<uint8_t>(8)})));
    {
        auto&& _m0_tmp = rusty::slice_full(var);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::array<rusty::Vec<uint8_t>, 1>{rusty::boxed::into_vec(rusty::boxed::box_new(std::array{static_cast<uint8_t>(3), static_cast<uint8_t>(5), static_cast<uint8_t>(8)}))};
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

void test_arraystring_const_constructible() {
    const auto AS = []() -> ArrayString<10> {
        return ArrayString<10>::new_const();
    };
    auto var = AS();
    if (!rusty::is_empty(var)) {
        rusty::panicking::panic("assertion failed: var.is_empty()");
    }
    {
        auto _m0 = &var;
        auto&& _m1_tmp = ArrayString<10>::new_();
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
    var.push_str("hello");
    {
        auto _m0 = &var;
        auto _m1_tmp = std::string_view("hello");
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

void test_arraystring_zero_filled_has_some_sanity_checks() {
    const auto string = ArrayString<4>::zero_filled();
    {
        auto&& _m0_tmp = string.as_str();
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = std::string_view(std::string_view("\0\0\0\0", 4));
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
        auto&& _m0_tmp = rusty::len(string);
        auto _m0 = &_m0_tmp;
        auto&& _m1_tmp = static_cast<int32_t>(4);
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
// Rust-only libtest wrapper metadata: marker=test_simple should_panic=no
void rusty_test_test_simple() {
    test_simple();
}
// Rust-only libtest wrapper metadata: marker=test_capacity_left should_panic=no
void rusty_test_test_capacity_left() {
    test_capacity_left();
}
// Rust-only libtest wrapper metadata: marker=test_extend_from_slice should_panic=no
void rusty_test_test_extend_from_slice() {
    test_extend_from_slice();
}
// Rust-only libtest wrapper metadata: marker=test_extend_from_slice_error should_panic=no
void rusty_test_test_extend_from_slice_error() {
    test_extend_from_slice_error();
}
// Rust-only libtest wrapper metadata: marker=test_try_from_slice_error should_panic=no
void rusty_test_test_try_from_slice_error() {
    test_try_from_slice_error();
}
// Rust-only libtest wrapper metadata: marker=test_u16_index should_panic=no
void rusty_test_test_u16_index() {
    test_u16_index();
}
// Rust-only libtest wrapper metadata: marker=test_iter should_panic=no
void rusty_test_test_iter() {
    test_iter();
}
// Rust-only libtest wrapper metadata: marker=test_drop should_panic=no
void rusty_test_test_drop() {
    test_drop();
}
// Rust-only libtest wrapper metadata: marker=test_drop_panics should_panic=no
void rusty_test_test_drop_panics() {
    test_drop_panics();
}
// Rust-only libtest wrapper metadata: marker=test_extend should_panic=no
void rusty_test_test_extend() {
    test_extend();
}
// Rust-only libtest wrapper metadata: marker=test_extend_capacity_panic_1 should_panic=yes
void rusty_test_test_extend_capacity_panic_1() {
    test_extend_capacity_panic_1();
}
// Rust-only libtest wrapper metadata: marker=test_extend_capacity_panic_2 should_panic=yes
void rusty_test_test_extend_capacity_panic_2() {
    test_extend_capacity_panic_2();
}
// Rust-only libtest wrapper metadata: marker=test_is_send_sync should_panic=no
void rusty_test_test_is_send_sync() {
    test_is_send_sync();
}
// Rust-only libtest wrapper metadata: marker=test_compact_size should_panic=no
void rusty_test_test_compact_size() {
    test_compact_size();
}
// Rust-only libtest wrapper metadata: marker=test_still_works_with_option_arrayvec should_panic=no
void rusty_test_test_still_works_with_option_arrayvec() {
    test_still_works_with_option_arrayvec();
}
// Rust-only libtest wrapper metadata: marker=test_drain should_panic=no
void rusty_test_test_drain() {
    test_drain();
}
// Rust-only libtest wrapper metadata: marker=test_drain_range_inclusive should_panic=no
void rusty_test_test_drain_range_inclusive() {
    test_drain_range_inclusive();
}
// Rust-only libtest wrapper metadata: marker=test_drain_range_inclusive_oob should_panic=yes
void rusty_test_test_drain_range_inclusive_oob() {
    test_drain_range_inclusive_oob();
}
// Rust-only libtest wrapper metadata: marker=test_retain should_panic=no
void rusty_test_test_retain() {
    test_retain();
}
// Rust-only libtest wrapper metadata: marker=test_drain_oob should_panic=yes
void rusty_test_test_drain_oob() {
    test_drain_oob();
}
// Rust-only libtest wrapper metadata: marker=test_drop_panic should_panic=yes
void rusty_test_test_drop_panic() {
    test_drop_panic();
}
// Rust-only libtest wrapper metadata: marker=test_drop_panic_into_iter should_panic=yes
void rusty_test_test_drop_panic_into_iter() {
    test_drop_panic_into_iter();
}
// Rust-only libtest wrapper metadata: marker=test_insert should_panic=no
void rusty_test_test_insert() {
    test_insert();
}
// Rust-only libtest wrapper metadata: marker=test_into_inner_1 should_panic=no
void rusty_test_test_into_inner_1() {
    test_into_inner_1();
}
// Rust-only libtest wrapper metadata: marker=test_into_inner_2 should_panic=no
void rusty_test_test_into_inner_2() {
    test_into_inner_2();
}
// Rust-only libtest wrapper metadata: marker=test_into_inner_3 should_panic=no
void rusty_test_test_into_inner_3() {
    test_into_inner_3();
}
// Rust-only libtest wrapper metadata: marker=test_take should_panic=no
void rusty_test_test_take() {
    test_take();
}
// Rust-only libtest wrapper metadata: marker=test_write should_panic=no
void rusty_test_test_write() {
    test_write();
}
// Rust-only libtest wrapper metadata: marker=array_clone_from should_panic=no
void rusty_test_array_clone_from() {
    array_clone_from();
}
// Rust-only libtest wrapper metadata: marker=test_string should_panic=no
void rusty_test_test_string() {
    test_string();
}
// Rust-only libtest wrapper metadata: marker=test_string_from should_panic=no
void rusty_test_test_string_from() {
    test_string_from();
}
// Rust-only libtest wrapper metadata: marker=test_string_parse_from_str should_panic=no
void rusty_test_test_string_parse_from_str() {
    test_string_parse_from_str();
}
// Rust-only libtest wrapper metadata: marker=test_string_from_bytes should_panic=no
void rusty_test_test_string_from_bytes() {
    test_string_from_bytes();
}
// Rust-only libtest wrapper metadata: marker=test_string_clone should_panic=no
void rusty_test_test_string_clone() {
    test_string_clone();
}
// Rust-only libtest wrapper metadata: marker=test_string_push should_panic=no
void rusty_test_test_string_push() {
    test_string_push();
}
// Rust-only libtest wrapper metadata: marker=test_insert_at_length should_panic=no
void rusty_test_test_insert_at_length() {
    test_insert_at_length();
}
// Rust-only libtest wrapper metadata: marker=test_insert_out_of_bounds should_panic=yes
void rusty_test_test_insert_out_of_bounds() {
    test_insert_out_of_bounds();
}
// Rust-only libtest wrapper metadata: marker=test_drop_in_insert should_panic=no
void rusty_test_test_drop_in_insert() {
    test_drop_in_insert();
}
// Rust-only libtest wrapper metadata: marker=test_pop_at should_panic=no
void rusty_test_test_pop_at() {
    test_pop_at();
}
// Rust-only libtest wrapper metadata: marker=test_sizes should_panic=no
void rusty_test_test_sizes() {
    test_sizes();
}
// Rust-only libtest wrapper metadata: marker=test_default should_panic=no
void rusty_test_test_default() {
    test_default();
}
// Rust-only libtest wrapper metadata: marker=test_extend_zst should_panic=no
void rusty_test_test_extend_zst() {
    test_extend_zst();
}
// Rust-only libtest wrapper metadata: marker=test_try_from_argument should_panic=no
void rusty_test_test_try_from_argument() {
    test_try_from_argument();
}
// Rust-only libtest wrapper metadata: marker=allow_max_capacity_arrayvec_type should_panic=no
void rusty_test_allow_max_capacity_arrayvec_type() {
    allow_max_capacity_arrayvec_type();
}
// Rust-only libtest wrapper metadata: marker=deny_max_capacity_arrayvec_value should_panic=yes
void rusty_test_deny_max_capacity_arrayvec_value() {
    deny_max_capacity_arrayvec_value();
}
// Rust-only libtest wrapper metadata: marker=deny_max_capacity_arrayvec_value_const should_panic=yes
void rusty_test_deny_max_capacity_arrayvec_value_const() {
    deny_max_capacity_arrayvec_value_const();
}
// Rust-only libtest wrapper metadata: marker=test_arrayvec_const_constructible should_panic=no
void rusty_test_test_arrayvec_const_constructible() {
    test_arrayvec_const_constructible();
}
// Rust-only libtest wrapper metadata: marker=test_arraystring_const_constructible should_panic=no
void rusty_test_test_arraystring_const_constructible() {
    test_arraystring_const_constructible();
}
// Rust-only libtest wrapper metadata: marker=test_arraystring_zero_filled_has_some_sanity_checks should_panic=no
void rusty_test_test_arraystring_zero_filled_has_some_sanity_checks() {
    test_arraystring_zero_filled_has_some_sanity_checks();
}


// ── Test runner ──
int main(int argc, char** argv) {
    if (argc == 3 && std::string(argv[1]) == "--rusty-single-test") {
        const std::string test_name = argv[2];
        rusty::mem::clear_all_forgotten_addresses();
        try {
            if (test_name == "rusty_test_allow_max_capacity_arrayvec_type") { rusty_test_allow_max_capacity_arrayvec_type(); return 0; }
            if (test_name == "rusty_test_array_clone_from") { rusty_test_array_clone_from(); return 0; }
            if (test_name == "rusty_test_char_test_encode_utf8") { rusty_test_char_test_encode_utf8(); return 0; }
            if (test_name == "rusty_test_char_test_encode_utf8_oob") { rusty_test_char_test_encode_utf8_oob(); return 0; }
            if (test_name == "rusty_test_deny_max_capacity_arrayvec_value") { rusty_test_deny_max_capacity_arrayvec_value(); return 0; }
            if (test_name == "rusty_test_deny_max_capacity_arrayvec_value_const") { rusty_test_deny_max_capacity_arrayvec_value_const(); return 0; }
            if (test_name == "rusty_test_test_arraystring_const_constructible") { rusty_test_test_arraystring_const_constructible(); return 0; }
            if (test_name == "rusty_test_test_arraystring_zero_filled_has_some_sanity_checks") { rusty_test_test_arraystring_zero_filled_has_some_sanity_checks(); return 0; }
            if (test_name == "rusty_test_test_arrayvec_const_constructible") { rusty_test_test_arrayvec_const_constructible(); return 0; }
            if (test_name == "rusty_test_test_capacity_left") { rusty_test_test_capacity_left(); return 0; }
            if (test_name == "rusty_test_test_compact_size") { rusty_test_test_compact_size(); return 0; }
            if (test_name == "rusty_test_test_default") { rusty_test_test_default(); return 0; }
            if (test_name == "rusty_test_test_drain") { rusty_test_test_drain(); return 0; }
            if (test_name == "rusty_test_test_drain_oob") { rusty_test_test_drain_oob(); return 0; }
            if (test_name == "rusty_test_test_drain_range_inclusive") { rusty_test_test_drain_range_inclusive(); return 0; }
            if (test_name == "rusty_test_test_drain_range_inclusive_oob") { rusty_test_test_drain_range_inclusive_oob(); return 0; }
            if (test_name == "rusty_test_test_drop") { rusty_test_test_drop(); return 0; }
            if (test_name == "rusty_test_test_drop_in_insert") { rusty_test_test_drop_in_insert(); return 0; }
            if (test_name == "rusty_test_test_drop_panic") { rusty_test_test_drop_panic(); return 0; }
            if (test_name == "rusty_test_test_drop_panic_into_iter") { rusty_test_test_drop_panic_into_iter(); return 0; }
            if (test_name == "rusty_test_test_drop_panics") { rusty_test_test_drop_panics(); return 0; }
            if (test_name == "rusty_test_test_extend") { rusty_test_test_extend(); return 0; }
            if (test_name == "rusty_test_test_extend_capacity_panic_1") { rusty_test_test_extend_capacity_panic_1(); return 0; }
            if (test_name == "rusty_test_test_extend_capacity_panic_2") { rusty_test_test_extend_capacity_panic_2(); return 0; }
            if (test_name == "rusty_test_test_extend_from_slice") { rusty_test_test_extend_from_slice(); return 0; }
            if (test_name == "rusty_test_test_extend_from_slice_error") { rusty_test_test_extend_from_slice_error(); return 0; }
            if (test_name == "rusty_test_test_extend_zst") { rusty_test_test_extend_zst(); return 0; }
            if (test_name == "rusty_test_test_insert") { rusty_test_test_insert(); return 0; }
            if (test_name == "rusty_test_test_insert_at_length") { rusty_test_test_insert_at_length(); return 0; }
            if (test_name == "rusty_test_test_insert_out_of_bounds") { rusty_test_test_insert_out_of_bounds(); return 0; }
            if (test_name == "rusty_test_test_into_inner_1") { rusty_test_test_into_inner_1(); return 0; }
            if (test_name == "rusty_test_test_into_inner_2") { rusty_test_test_into_inner_2(); return 0; }
            if (test_name == "rusty_test_test_into_inner_3") { rusty_test_test_into_inner_3(); return 0; }
            if (test_name == "rusty_test_test_is_send_sync") { rusty_test_test_is_send_sync(); return 0; }
            if (test_name == "rusty_test_test_iter") { rusty_test_test_iter(); return 0; }
            if (test_name == "rusty_test_test_pop_at") { rusty_test_test_pop_at(); return 0; }
            if (test_name == "rusty_test_test_retain") { rusty_test_test_retain(); return 0; }
            if (test_name == "rusty_test_test_simple") { rusty_test_test_simple(); return 0; }
            if (test_name == "rusty_test_test_sizes") { rusty_test_test_sizes(); return 0; }
            if (test_name == "rusty_test_test_still_works_with_option_arrayvec") { rusty_test_test_still_works_with_option_arrayvec(); return 0; }
            if (test_name == "rusty_test_test_string") { rusty_test_test_string(); return 0; }
            if (test_name == "rusty_test_test_string_clone") { rusty_test_test_string_clone(); return 0; }
            if (test_name == "rusty_test_test_string_from") { rusty_test_test_string_from(); return 0; }
            if (test_name == "rusty_test_test_string_from_bytes") { rusty_test_test_string_from_bytes(); return 0; }
            if (test_name == "rusty_test_test_string_parse_from_str") { rusty_test_test_string_parse_from_str(); return 0; }
            if (test_name == "rusty_test_test_string_push") { rusty_test_test_string_push(); return 0; }
            if (test_name == "rusty_test_test_take") { rusty_test_test_take(); return 0; }
            if (test_name == "rusty_test_test_try_from_argument") { rusty_test_test_try_from_argument(); return 0; }
            if (test_name == "rusty_test_test_try_from_slice_error") { rusty_test_test_try_from_slice_error(); return 0; }
            if (test_name == "rusty_test_test_u16_index") { rusty_test_test_u16_index(); return 0; }
            if (test_name == "rusty_test_test_write") { rusty_test_test_write(); return 0; }
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
    try { rusty_test_allow_max_capacity_arrayvec_type(); std::cout << "  allow_max_capacity_arrayvec_type PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  allow_max_capacity_arrayvec_type FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  allow_max_capacity_arrayvec_type FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_array_clone_from(); std::cout << "  array_clone_from PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  array_clone_from FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  array_clone_from FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_char_test_encode_utf8(); std::cout << "  char_test_encode_utf8 PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  char_test_encode_utf8 FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  char_test_encode_utf8 FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_char_test_encode_utf8_oob(); std::cout << "  char_test_encode_utf8_oob PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  char_test_encode_utf8_oob FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  char_test_encode_utf8_oob FAILED (unknown exception)" << std::endl; fail++; }
    {
        const std::string cmd = std::string("\"") + argv[0] + "\" --rusty-single-test rusty_test_deny_max_capacity_arrayvec_value";
        const int status = std::system(cmd.c_str());
        if (status != 0) { std::cout << "  deny_max_capacity_arrayvec_value PASSED (expected panic)" << std::endl; pass++; }
        else { std::cerr << "  deny_max_capacity_arrayvec_value FAILED: expected panic" << std::endl; fail++; }
    }
    {
        const std::string cmd = std::string("\"") + argv[0] + "\" --rusty-single-test rusty_test_deny_max_capacity_arrayvec_value_const";
        const int status = std::system(cmd.c_str());
        if (status != 0) { std::cout << "  deny_max_capacity_arrayvec_value_const PASSED (expected panic)" << std::endl; pass++; }
        else { std::cerr << "  deny_max_capacity_arrayvec_value_const FAILED: expected panic" << std::endl; fail++; }
    }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_arraystring_const_constructible(); std::cout << "  test_arraystring_const_constructible PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_arraystring_const_constructible FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_arraystring_const_constructible FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_arraystring_zero_filled_has_some_sanity_checks(); std::cout << "  test_arraystring_zero_filled_has_some_sanity_checks PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_arraystring_zero_filled_has_some_sanity_checks FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_arraystring_zero_filled_has_some_sanity_checks FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_arrayvec_const_constructible(); std::cout << "  test_arrayvec_const_constructible PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_arrayvec_const_constructible FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_arrayvec_const_constructible FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_capacity_left(); std::cout << "  test_capacity_left PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_capacity_left FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_capacity_left FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_compact_size(); std::cout << "  test_compact_size PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_compact_size FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_compact_size FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_default(); std::cout << "  test_default PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_default FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_default FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_drain(); std::cout << "  test_drain PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_drain FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_drain FAILED (unknown exception)" << std::endl; fail++; }
    {
        const std::string cmd = std::string("\"") + argv[0] + "\" --rusty-single-test rusty_test_test_drain_oob";
        const int status = std::system(cmd.c_str());
        if (status != 0) { std::cout << "  test_drain_oob PASSED (expected panic)" << std::endl; pass++; }
        else { std::cerr << "  test_drain_oob FAILED: expected panic" << std::endl; fail++; }
    }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_drain_range_inclusive(); std::cout << "  test_drain_range_inclusive PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_drain_range_inclusive FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_drain_range_inclusive FAILED (unknown exception)" << std::endl; fail++; }
    {
        const std::string cmd = std::string("\"") + argv[0] + "\" --rusty-single-test rusty_test_test_drain_range_inclusive_oob";
        const int status = std::system(cmd.c_str());
        if (status != 0) { std::cout << "  test_drain_range_inclusive_oob PASSED (expected panic)" << std::endl; pass++; }
        else { std::cerr << "  test_drain_range_inclusive_oob FAILED: expected panic" << std::endl; fail++; }
    }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_drop(); std::cout << "  test_drop PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_drop FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_drop FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_drop_in_insert(); std::cout << "  test_drop_in_insert PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_drop_in_insert FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_drop_in_insert FAILED (unknown exception)" << std::endl; fail++; }
    {
        const std::string cmd = std::string("\"") + argv[0] + "\" --rusty-single-test rusty_test_test_drop_panic";
        const int status = std::system(cmd.c_str());
        if (status != 0) { std::cout << "  test_drop_panic PASSED (expected panic)" << std::endl; pass++; }
        else { std::cerr << "  test_drop_panic FAILED: expected panic" << std::endl; fail++; }
    }
    {
        const std::string cmd = std::string("\"") + argv[0] + "\" --rusty-single-test rusty_test_test_drop_panic_into_iter";
        const int status = std::system(cmd.c_str());
        if (status != 0) { std::cout << "  test_drop_panic_into_iter PASSED (expected panic)" << std::endl; pass++; }
        else { std::cerr << "  test_drop_panic_into_iter FAILED: expected panic" << std::endl; fail++; }
    }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_drop_panics(); std::cout << "  test_drop_panics PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_drop_panics FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_drop_panics FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_extend(); std::cout << "  test_extend PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_extend FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_extend FAILED (unknown exception)" << std::endl; fail++; }
    {
        const std::string cmd = std::string("\"") + argv[0] + "\" --rusty-single-test rusty_test_test_extend_capacity_panic_1";
        const int status = std::system(cmd.c_str());
        if (status != 0) { std::cout << "  test_extend_capacity_panic_1 PASSED (expected panic)" << std::endl; pass++; }
        else { std::cerr << "  test_extend_capacity_panic_1 FAILED: expected panic" << std::endl; fail++; }
    }
    {
        const std::string cmd = std::string("\"") + argv[0] + "\" --rusty-single-test rusty_test_test_extend_capacity_panic_2";
        const int status = std::system(cmd.c_str());
        if (status != 0) { std::cout << "  test_extend_capacity_panic_2 PASSED (expected panic)" << std::endl; pass++; }
        else { std::cerr << "  test_extend_capacity_panic_2 FAILED: expected panic" << std::endl; fail++; }
    }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_extend_from_slice(); std::cout << "  test_extend_from_slice PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_extend_from_slice FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_extend_from_slice FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_extend_from_slice_error(); std::cout << "  test_extend_from_slice_error PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_extend_from_slice_error FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_extend_from_slice_error FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_extend_zst(); std::cout << "  test_extend_zst PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_extend_zst FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_extend_zst FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_insert(); std::cout << "  test_insert PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_insert FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_insert FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_insert_at_length(); std::cout << "  test_insert_at_length PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_insert_at_length FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_insert_at_length FAILED (unknown exception)" << std::endl; fail++; }
    {
        const std::string cmd = std::string("\"") + argv[0] + "\" --rusty-single-test rusty_test_test_insert_out_of_bounds";
        const int status = std::system(cmd.c_str());
        if (status != 0) { std::cout << "  test_insert_out_of_bounds PASSED (expected panic)" << std::endl; pass++; }
        else { std::cerr << "  test_insert_out_of_bounds FAILED: expected panic" << std::endl; fail++; }
    }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_into_inner_1(); std::cout << "  test_into_inner_1 PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_into_inner_1 FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_into_inner_1 FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_into_inner_2(); std::cout << "  test_into_inner_2 PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_into_inner_2 FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_into_inner_2 FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_into_inner_3(); std::cout << "  test_into_inner_3 PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_into_inner_3 FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_into_inner_3 FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_is_send_sync(); std::cout << "  test_is_send_sync PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_is_send_sync FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_is_send_sync FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_iter(); std::cout << "  test_iter PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_iter FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_iter FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_pop_at(); std::cout << "  test_pop_at PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_pop_at FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_pop_at FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_retain(); std::cout << "  test_retain PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_retain FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_retain FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_simple(); std::cout << "  test_simple PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_simple FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_simple FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_sizes(); std::cout << "  test_sizes PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_sizes FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_sizes FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_still_works_with_option_arrayvec(); std::cout << "  test_still_works_with_option_arrayvec PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_still_works_with_option_arrayvec FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_still_works_with_option_arrayvec FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_string(); std::cout << "  test_string PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_string FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_string FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_string_clone(); std::cout << "  test_string_clone PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_string_clone FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_string_clone FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_string_from(); std::cout << "  test_string_from PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_string_from FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_string_from FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_string_from_bytes(); std::cout << "  test_string_from_bytes PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_string_from_bytes FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_string_from_bytes FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_string_parse_from_str(); std::cout << "  test_string_parse_from_str PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_string_parse_from_str FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_string_parse_from_str FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_string_push(); std::cout << "  test_string_push PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_string_push FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_string_push FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_take(); std::cout << "  test_take PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_take FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_take FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_try_from_argument(); std::cout << "  test_try_from_argument PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_try_from_argument FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_try_from_argument FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_try_from_slice_error(); std::cout << "  test_try_from_slice_error PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_try_from_slice_error FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_try_from_slice_error FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_u16_index(); std::cout << "  test_u16_index PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_u16_index FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_u16_index FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_test_write(); std::cout << "  test_write PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  test_write FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  test_write FAILED (unknown exception)" << std::endl; fail++; }
    std::cout << std::endl;
    std::cout << "Results: " << pass << " passed, " << fail << " failed" << std::endl;
    return fail > 0 ? 1 : 0;
}
