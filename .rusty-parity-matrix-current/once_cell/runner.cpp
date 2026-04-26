// Auto-generated parity test runner
#include <cstdint>
#include <cstddef>
#include <limits>
#include <variant>
#include <string>
#include <optional>
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

// Overloaded visitor helper
template<class... Ts> struct overloaded : Ts... { using Ts::operator()...; };
template<class... Ts>
overloaded(Ts...) -> overloaded<Ts...>;

// Hoisted namespace forward declarations
namespace imp { struct Guard; }
namespace imp { struct Waiter; }
namespace imp { template<typename T> struct OnceCell; }
namespace race { namespace once_box { template<typename T> struct OnceBox; } }
namespace race { struct OnceBool; }
namespace race { struct OnceNonZeroUsize; }
namespace race { template<typename T> struct OnceRef; }
namespace race_once_box { struct Heap; }
namespace race_once_box { template<typename T> struct Pebble; }
namespace sync_mod { template<typename T, typename F> struct Lazy; }
namespace sync_mod { template<typename T> struct OnceCell; }
namespace unsync { template<typename T, typename F> struct Lazy; }
namespace unsync { template<typename T> struct OnceCell; }

// ── from once_cell.cppm ──



#if defined(__GNUC__)
#pragma GCC diagnostic ignored "-Wunused-local-typedefs"
#endif


namespace imp {}
namespace imp::tests {}
namespace race {}
namespace race::once_box {}
namespace sync_mod {}
namespace unsync {}

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
static_assert(
dependent_false_v<L, R>,
"rusty::cmp fallback requires direct < support or lexicographically comparable begin/end ranges"
);
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
explicit Cow_Borrowed(std::string_view value) : _0(value) {}
bool operator==(const Cow_Borrowed& other) const { return _0 == other._0; }
};
struct Cow_Owned {
rusty::String _0;
explicit Cow_Owned(rusty::String value) : _0(std::move(value)) {}
bool operator==(const Cow_Owned& other) const { return _0 == other._0; }
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
};
inline SplitIter split(std::string_view s, char32_t delim) {
return SplitIter{s, delim};
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



namespace sync_mod {}

namespace rusty_module_aliases {
namespace sync = sync_mod;
} // namespace rusty_module_aliases
using namespace rusty_module_aliases;

namespace unsync {
    template<typename T>
    struct OnceCell;
    template<typename T, typename F>
    struct Lazy;
}
namespace race {
    struct OnceNonZeroUsize;
    struct OnceBool;
    template<typename T>
    struct OnceRef;
    namespace once_box {
        template<typename T>
        struct OnceBox;
        void _dummy();
    }
}
namespace imp {
    struct Waiter;
    template<typename T>
    struct OnceCell;
    struct Guard;
    constexpr size_t INCOMPLETE = static_cast<size_t>(0);
    constexpr size_t RUNNING = static_cast<size_t>(1);
    constexpr size_t COMPLETE = static_cast<size_t>(2);
    extern Waiter* const INCOMPLETE_PTR;
    extern Waiter* const COMPLETE_PTR;
    constexpr size_t STATE_MASK = static_cast<size_t>(3);
    namespace strict {
        template<typename T>
        size_t addr(std::add_pointer_t<T> ptr);
        template<typename T>
        std::add_pointer_t<T> with_addr(std::add_pointer_t<T> ptr, size_t addr);
        template<typename T>
        std::add_pointer_t<T> map_addr(std::add_pointer_t<T> ptr, const auto& f);
    }
    namespace tests {
        using ::imp::OnceCell;
        void smoke_once();
        void stampede_once();
        void poison_bad();
        void wait_for_force_to_finish();
        void test_size();
    }
    void initialize_or_wait(const rusty::sync::atomic::AtomicPtr<Waiter>& queue, rusty::Option<const std::function<bool()>&> init);
    void wait(const rusty::sync::atomic::AtomicPtr<Waiter>& queue, Waiter* curr_queue);
}
namespace sync_mod {
    template<typename T>
    struct OnceCell;
    template<typename T, typename F>
    struct Lazy;
    template<typename... Ts> using Imp = imp::OnceCell<Ts...>;
    void _dummy();
}


namespace unsync {

    template<typename T>
    struct OnceCell;
    template<typename T, typename F>
    struct Lazy;

    using ::rusty::Cell;
    using ::rusty::UnsafeCell;
    namespace fmt = rusty::fmt;
    namespace mem = rusty::mem;

    /// A cell which can be written to only once. It is not thread safe.
    ///
    /// Unlike [`std::cell::RefCell`], a `OnceCell` provides simple `&`
    /// references to the contents.
    ///
    /// [`std::cell::RefCell`]: https://doc.rust-lang.org/std/cell/struct.RefCell.html
    ///
    /// # Example
    /// ```
    /// use once_cell::unsync::OnceCell;
    ///
    /// let cell = OnceCell::new();
    /// assert!(cell.get().is_none());
    ///
    /// let value: &String = cell.get_or_init(|| {
    ///     "Hello, World!".to_string()
    /// });
    /// assert_eq!(value, "Hello, World!");
    /// assert!(cell.get().is_some());
    /// ```
    template<typename T>
    struct OnceCell {
        rusty::UnsafeCell<rusty::Option<T>> inner;

        static OnceCell<T> default_() {
            return OnceCell<T>::new_();
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return [&]() -> rusty::fmt::Result { auto&& _m = this->get(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& v = _mv0; return f.debug_tuple("OnceCell").field(std::move(v)).finish(); } if (_m.is_none()) { return f.write_str("OnceCell(Uninit)"); } return [&]() -> rusty::fmt::Result { rusty::intrinsics::unreachable(); }(); }();
        }
        OnceCell<T> clone() const {
            return [&]() -> OnceCell<T> { auto&& _m = this->get(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& value = _mv0; return OnceCell<T>::with_value(rusty::clone(value)); } if (_m.is_none()) { return OnceCell<T>::new_(); } return [&]() -> OnceCell<T> { rusty::intrinsics::unreachable(); }(); }();
        }
        void clone_from(const OnceCell<T>& source) {
            {
                auto&& _m0 = this->get_mut();
                auto&& _m1 = source.get();
                auto _m_tuple = std::forward_as_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched && ((std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)).is_some() && std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)).is_some()))) {
                    auto&& this_ = std::as_const(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple))).unwrap();
                    auto&& source = std::as_const(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple))).unwrap();
                    this_.clone_from(source);
                    _m_matched = true;
                }
                if (!_m_matched && (true)) {
                    (*this) = rusty::clone(source);
                    _m_matched = true;
                }
            }
        }
        bool operator==(const OnceCell<T>& other) const {
            return this->get() == other.get();
        }
        static OnceCell<T> from(T value) {
            return OnceCell<T>::with_value(std::move(value));
        }
        static OnceCell<T> new_() {
            return OnceCell<T>(rusty::UnsafeCell<rusty::Option<T>>::new_(rusty::Option<T>(rusty::None)));
        }
        static OnceCell<T> with_value(T value) {
            return OnceCell<T>(rusty::UnsafeCell<rusty::Option<T>>::new_(rusty::Option<T>(std::move(value))));
        }
        rusty::Option<const T&> get() const {
            return (*this->inner.get()).as_ref();
        }
        rusty::Option<T&> get_mut() {
            return (*this->inner.get()).as_mut();
        }
        rusty::Result<std::tuple<>, T> set(T value) const {
            return [&]() -> rusty::Result<std::tuple<>, T> { auto&& _m = this->try_insert(std::move(value)); if (_m.is_ok()) { return rusty::Result<std::tuple<>, T>::Ok(std::make_tuple()); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& value = std::get<1>(rusty::detail::deref_if_pointer(_mv1)); return rusty::Result<std::tuple<>, T>::Err(std::move(value)); } return [&]() -> rusty::Result<std::tuple<>, T> { rusty::intrinsics::unreachable(); }(); }();
        }
        rusty::Result<const T&, std::tuple<const T&, T>> try_insert(T value) const {
            if (auto&& _iflet_scrutinee = this->get(); _iflet_scrutinee.is_some()) {
                decltype(auto) old = _iflet_scrutinee.unwrap();
                return rusty::Result<const T&, std::tuple<const T&, T>>::Err(std::tuple<const T&, T>{old, std::move(value)});
            }
            rusty::Option<T>& slot = *this->inner.get();
            slot = rusty::Option<T>(std::move(value));
            return rusty::Result<const T&, std::tuple<const T&, T>>::Ok(slot.as_ref().unwrap_unchecked());
        }
        template<typename F>
        const T& get_or_init(F f) const {
            enum class Void {
                
            };
            return [&]() -> const T& { auto&& _m = this->get_or_try_init([&]() -> rusty::Result<T, std::tuple<>> { return rusty::Result<T, std::tuple<>>::Ok(f()); }); if (_m.is_ok()) { return _m.unwrap(); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& void_ = _mv1; return [&]() -> const T& { rusty::intrinsics::unreachable(); }(); } return [&]() -> const T& { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename F>
        auto get_or_try_init(F f) const {
            using E = rusty::result_err_t<decltype((f()))>;
            if (auto&& _iflet_scrutinee = this->get(); _iflet_scrutinee.is_some()) {
                decltype(auto) val = _iflet_scrutinee.unwrap();
                return rusty::Result<const T&, E>::Ok(val);
            }
            auto val = RUSTY_TRY_INTO(f(), rusty::Result<const T&, E>);
            if (!this->set(std::move(val)).is_ok()) {
                {
                    [&]() -> rusty::Result<const T&, E> { rusty::panicking::panic_fmt(std::string("reentrant init")); }();
                }
            }
            return rusty::Result<const T&, E>::Ok(this->get().unwrap_unchecked());
        }
        rusty::Option<T> take() {
            return (*this).take().into_inner();
        }
        rusty::Option<T> into_inner() {
            return this->inner.into_inner();
        }
    };

    /// A value which is initialized on the first access.
    ///
    /// # Example
    /// ```
    /// use once_cell::unsync::Lazy;
    ///
    /// let lazy: Lazy<i32> = Lazy::new(|| {
    ///     println!("initializing");
    ///     92
    /// });
    /// println!("ready");
    /// println!("{}", *lazy);
    /// println!("{}", *lazy);
    ///
    /// // Prints:
    /// //   ready
    /// //   initializing
    /// //   92
    /// //   92
    /// ```
    template<typename T, typename F = rusty::SafeFn<T()>>
    struct Lazy {
        using Target = T;
        OnceCell<T> cell;
        rusty::Cell<rusty::Option<F>> init;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("Lazy").field("cell", &this->cell).field("init", rusty::addr_of_temp("..")).finish();
        }
        static Lazy<T, F> new_(F init) {
            return Lazy<T, F>(OnceCell<T>::new_(), rusty::Cell<rusty::Option<F>>::new_(rusty::Option<F>(std::move(init))));
        }
        static rusty::Result<T, F> into_value(Lazy<T, F> this_) {
            auto cell = std::move(this_.cell);
            const auto init = std::move(this_.init);
            return cell.into_inner().ok_or_else([&]() {
return init.take().unwrap_or_else([&]() {
rusty::panicking::panic_fmt(std::string("Lazy instance has previously been poisoned"));
});
});
        }
        static const T& force(const Lazy<T, F>& this_) {
            return this_.cell.get_or_init([&]() { return [&]() { auto&& _m = this_.init.take(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& f = _mv0; return f(); } if (_m.is_none()) { [&]() { rusty::panicking::panic_fmt(std::string("Lazy instance has previously been poisoned")); }(); } rusty::intrinsics::unreachable(); }(); });
        }
        static T& force_mut(Lazy<T, F>& this_) {
            if (this_.cell.get_mut().is_none()) {
                auto value = [&]() { auto&& _m = this_.init.get_mut()->take(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& f = _mv0; return f(); } if (_m.is_none()) { [&]() { [&]() -> T& { rusty::panicking::panic_fmt(std::string("Lazy instance has previously been poisoned")); }(); }(); } rusty::intrinsics::unreachable(); }();
                this_.cell = OnceCell<T>::with_value(std::move(value));
            }
            return this_.cell.get_mut().unwrap_or_else([&]() { rusty::panicking::panic("internal error: entered unreachable code"); });
        }
        static rusty::Option<const T&> get(const Lazy<T, F>& this_) {
            return this_.cell.get();
        }
        static rusty::Option<T&> get_mut(Lazy<T, F>& this_) {
            return this_.cell.get_mut();
        }
        const T& operator*() const {
            return Lazy<T>::force((*this));
        }
        T& operator*() {
            return Lazy<T>::force_mut((*this));
        }
        static Lazy<T> default_() {
            return Lazy<T>::new_([&]() { return rusty::default_value<T>(); });
        }
    };

}

namespace race {
    namespace once_box {}

    struct OnceNonZeroUsize;
    struct OnceBool;
    template<typename T>
    struct OnceRef;
    namespace once_box {
        template<typename T>
        struct OnceBox;
        void _dummy();
    }

    namespace atomic = rusty::sync::atomic;

    using rusty::sync::atomic::AtomicPtr;
    using rusty::sync::atomic::AtomicUsize;
    using rusty::sync::atomic::Ordering;

    using ::rusty::UnsafeCell;

    using ::rusty::PhantomData;

    using ::rusty::num::NonZeroUsize;

    namespace ptr = rusty::ptr;

    /// A thread-safe cell which can be written to only once.
    struct OnceNonZeroUsize {
        rusty::sync::atomic::AtomicUsize inner;

        static OnceNonZeroUsize default_();
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
        static OnceNonZeroUsize new_();
        rusty::Option<rusty::num::NonZeroUsize> get() const;
        rusty::num::NonZeroUsize get_unchecked() const;
        rusty::Result<std::tuple<>, std::tuple<>> set(rusty::num::NonZeroUsize value) const;
        template<typename F>
        rusty::num::NonZeroUsize get_or_init(F f) const;
        template<typename F>
        auto get_or_try_init(F f) const;
        template<typename E>
        rusty::Result<rusty::num::NonZeroUsize, E> init(const auto& f) const;
        rusty::Result<size_t, size_t> compare_exchange(rusty::num::NonZeroUsize val) const;
    };

    /// A thread-safe cell which can be written to only once.
    struct OnceBool {
        OnceNonZeroUsize inner;

        static OnceBool default_();
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
        static OnceBool new_();
        rusty::Option<bool> get() const;
        rusty::Result<std::tuple<>, std::tuple<>> set(bool value) const;
        template<typename F>
        bool get_or_init(F f) const;
        template<typename F>
        auto get_or_try_init(F f) const;
        static bool from_usize(rusty::num::NonZeroUsize value);
        static rusty::num::NonZeroUsize to_usize(bool value);
    };

    /// A thread-safe cell which can be written to only once.
    template<typename T>
    struct OnceRef {
        rusty::sync::atomic::AtomicPtr<T> inner;
        rusty::PhantomData<rusty::UnsafeCell<const T&>> ghost;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return rusty::write_fmt(f, std::format("OnceRef({0})", rusty::to_debug_string(this->inner)));
        }
        static OnceRef<T> default_() {
            return OnceRef<T>::new_();
        }
        static OnceRef<T> new_() {
            return OnceRef<T>{.inner = rusty::sync::atomic::AtomicPtr<T>::new_(rusty::ptr::null_mut()), .ghost = rusty::PhantomData<rusty::UnsafeCell<const T&>>{}};
        }
        rusty::Option<const T&> get() const {
            const auto ptr_shadow1 = this->inner.load(Ordering::Acquire);
            // @unsafe
            {
                return rusty::ptr::as_ref(ptr_shadow1);
            }
        }
        auto set(const T& value) const {
            return [&]() -> rusty::Result<std::tuple<>, std::tuple<>> { auto&& _m = this->compare_exchange(rusty::detail::deref_if_pointer_like(value)); if (_m.is_ok()) { return rusty::Result<std::tuple<>, std::tuple<>>::Ok(std::make_tuple()); } if (_m.is_err()) { return rusty::Result<std::tuple<>, std::tuple<>>::Err(std::make_tuple()); } return [&]() -> rusty::Result<std::tuple<>, std::tuple<>> { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename F>
        const T& get_or_init(F f) const {
            enum class Void {
                
            };
            return [&]() -> const T& { auto&& _m = this->get_or_try_init([&]() -> rusty::Result<const T&, std::tuple<>> { return rusty::Result<const T&, std::tuple<>>::Ok([&]() -> const auto& { auto _result_ref_value = (f()); thread_local std::optional<T> _result_ref_tmp; _result_ref_tmp.reset(); _result_ref_tmp.emplace(std::move(_result_ref_value)); return *_result_ref_tmp; }()); }); if (_m.is_ok()) { return _m.unwrap(); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& void_ = _mv1; return [&]() -> const T& { rusty::intrinsics::unreachable(); }(); } return [&]() -> const T& { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename F>
        auto get_or_try_init(F f) const {
            using E = rusty::result_err_t<decltype((f()))>;
            return [&]() -> rusty::Result<const T&, E> { auto&& _m = this->get(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& val = _mv0; return rusty::Result<const T&, E>::Ok(val); } if (_m.is_none()) { return this->template init<E>(rusty::detail::deref_if_pointer_like(f)); } return [&]() -> rusty::Result<const T&, E> { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename E>
        rusty::Result<const T&, E> init(const auto& f) const {
            const T* value = ({ auto _rusty_try_result = (f()); if (_rusty_try_result.is_err()) { return rusty::Result<const T&, E>::Err(_rusty_try_result.unwrap_err()); } &(_rusty_try_result.unwrap()); });
            if (auto&& _iflet_scrutinee = this->compare_exchange(rusty::detail::deref_if_pointer_like(*value)); _iflet_scrutinee.is_err()) {
                decltype(auto) old = _iflet_scrutinee.unwrap_err();
                value = old;
            }
            return rusty::Result<const T&, E>::Ok(*value);
        }
        rusty::Result<std::tuple<>, std::add_pointer_t<std::add_const_t<T>>> compare_exchange(const T& value) const {
            return this->inner.compare_exchange(rusty::ptr::null_mut(), rusty::ptr::cast_mut(value), Ordering::Release, Ordering::Acquire).map([&](std::add_pointer_t<T> _) -> std::tuple<> { return std::make_tuple(); }).map_err([&](auto&& _err) -> std::add_pointer_t<std::add_const_t<T>> { return (rusty::ptr::cast_const) (std::forward<decltype(_err)>(_err)); });
        }
        static void _dummy() {
        }
    };

    using ::race::once_box::OnceBox;

    namespace once_box {

        template<typename T>
        struct OnceBox;
        void _dummy();

        using ::race::atomic::AtomicPtr;
        using ::race::atomic::Ordering;

        using ::rusty::PhantomData;
        namespace ptr = rusty::ptr;

        using ::rusty::Box;

        /// A thread-safe cell which can be written to only once.
        template<typename T>
        struct OnceBox {
            rusty::sync::atomic::AtomicPtr<T> inner;
            rusty::PhantomData<rusty::Option<rusty::Box<T>>> ghost;
            OnceBox(rusty::sync::atomic::AtomicPtr<T> inner_init, rusty::PhantomData<rusty::Option<rusty::Box<T>>> ghost_init) : inner(std::move(inner_init)), ghost(std::move(ghost_init)) {}
            OnceBox(const OnceBox&) = default;
            OnceBox(OnceBox&& other) noexcept : inner(std::move(other.inner)), ghost(std::move(other.ghost)) {
                if (rusty::mem::consume_forgotten_address(&other)) {
                    this->rusty_mark_forgotten();
                    other.rusty_mark_forgotten();
                } else {
                    other.rusty_mark_forgotten();
                }
            }
            OnceBox& operator=(const OnceBox&) = default;
            OnceBox& operator=(OnceBox&& other) noexcept {
                if (this == &other) {
                    return *this;
                }
                this->~OnceBox();
                new (this) OnceBox(std::move(other));
                return *this;
            }
            void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
                return rusty::write_fmt(f, std::format("OnceBox({0})", rusty::to_debug_string(this->inner.load(Ordering::Relaxed))));
            }
            static OnceBox<T> default_() {
                return OnceBox<T>::new_();
            }
            ~OnceBox() noexcept(false) {
                if (rusty::mem::consume_forgotten_address(this)) { return; }
                const auto ptr_shadow1 = rusty::detail::deref_if_pointer_like(this->inner.get_mut());
                if (!(ptr_shadow1 == nullptr)) {
                    rusty::mem::drop(rusty::Box<std::remove_pointer_t<std::remove_reference_t<decltype((ptr_shadow1))>>>::from_raw(std::move(ptr_shadow1)));
                }
            }
            static OnceBox<T> new_() {
                return OnceBox<T>(rusty::sync::atomic::AtomicPtr<T>::new_(rusty::ptr::null_mut()), rusty::PhantomData<rusty::Option<rusty::Box<T>>>{});
            }
            static OnceBox<T> with_value(rusty::Box<T> value) {
                return OnceBox<T>(rusty::sync::atomic::AtomicPtr<T>::new_((std::move(value)).into_raw()), rusty::PhantomData<rusty::Option<rusty::Box<T>>>{});
            }
            rusty::Option<const T&> get() const {
                const auto ptr_shadow1 = this->inner.load(Ordering::Acquire);
                if ((ptr_shadow1 == nullptr)) {
                    return rusty::Option<const T&>(rusty::None);
                }
                return rusty::Option<const T&>(*ptr_shadow1);
            }
            rusty::Result<std::tuple<>, rusty::Box<T>> set(rusty::Box<T> value) const {
                const auto ptr_shadow1 = (std::move(value)).into_raw();
                const auto exchange = this->inner.compare_exchange(rusty::ptr::null_mut(), std::move(ptr_shadow1), Ordering::Release, Ordering::Acquire);
                if (exchange.is_err()) {
                    auto value_shadow1 = rusty::Box<std::remove_pointer_t<std::remove_reference_t<decltype((ptr_shadow1))>>>::from_raw(std::move(ptr_shadow1));
                    return rusty::Result<std::tuple<>, rusty::Box<T>>::Err(std::move(value_shadow1));
                }
                return rusty::Result<std::tuple<>, rusty::Box<T>>::Ok(std::make_tuple());
            }
            template<typename F>
            const T& get_or_init(F f) const {
                enum class Void {
                    
                };
                return [&]() -> const T& { auto&& _m = this->get_or_try_init([&]() -> rusty::Result<rusty::Box<T>, std::tuple<>> { return rusty::Result<rusty::Box<T>, std::tuple<>>::Ok(f()); }); if (_m.is_ok()) { return _m.unwrap(); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& void_ = _mv1; return [&]() -> const T& { rusty::intrinsics::unreachable(); }(); } return [&]() -> const T& { rusty::intrinsics::unreachable(); }(); }();
            }
            template<typename F>
            auto get_or_try_init(F f) const {
                using E = rusty::result_err_t<decltype((f()))>;
                return [&]() -> rusty::Result<const T&, E> { auto&& _m = this->get(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& val = _mv0; return rusty::Result<const T&, E>::Ok(val); } if (_m.is_none()) { return this->template init<E>(std::move(f)); } return [&]() -> rusty::Result<const T&, E> { rusty::intrinsics::unreachable(); }(); }();
            }
            template<typename E>
            rusty::Result<const T&, E> init(const auto& f) const {
                auto val = RUSTY_TRY_INTO(f(), rusty::Result<const T&, E>);
                auto ptr_shadow1 = (std::move(val)).into_raw();
                const auto exchange = this->inner.compare_exchange(rusty::ptr::null_mut(), std::move(ptr_shadow1), Ordering::Release, Ordering::Acquire);
                if (exchange.is_err()) {
                    decltype(auto) old = exchange.unwrap_err();
                    rusty::mem::drop(rusty::Box<std::remove_pointer_t<std::remove_reference_t<decltype((ptr_shadow1))>>>::from_raw(std::move(ptr_shadow1)));
                    ptr_shadow1 = old;
                }
                return rusty::Result<const T&, E>::Ok(rusty::detail::deref_if_pointer_like(ptr_shadow1));
            }
            OnceBox<T> clone() const {
                return [&]() -> OnceBox<T> { auto&& _m = this->get(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& value = _mv0; return OnceBox<T>::with_value(rusty::Box<T>::new_(rusty::clone(value))); } if (_m.is_none()) { return OnceBox<T>::new_(); } return [&]() -> OnceBox<T> { rusty::intrinsics::unreachable(); }(); }();
            }
        };

        /// ```compile_fail
        /// struct S(*mut ());
        /// unsafe impl Sync for S {}
        ///
        /// fn share<T: Sync>(_: &T) {}
        /// share(&once_cell::race::OnceBox::<S>::new());
        /// ```
        void _dummy() {
        }

    }

}

namespace imp {
    namespace strict {}
    namespace tests {}

    struct Waiter;
    template<typename T>
    struct OnceCell;
    struct Guard;
    extern Waiter* const INCOMPLETE_PTR;
    extern Waiter* const COMPLETE_PTR;
    namespace strict {
        template<typename T>
        size_t addr(std::add_pointer_t<T> ptr);
        template<typename T>
        std::add_pointer_t<T> with_addr(std::add_pointer_t<T> ptr, size_t addr);
        template<typename T>
        std::add_pointer_t<T> map_addr(std::add_pointer_t<T> ptr, const auto& f);
    }
    namespace tests {
        using ::imp::OnceCell;
        void smoke_once();
        void stampede_once();
        void poison_bad();
        void wait_for_force_to_finish();
        void test_size();
    }
    void initialize_or_wait(const rusty::sync::atomic::AtomicPtr<Waiter>& queue, rusty::Option<const std::function<bool()>&> init);
    void wait(const rusty::sync::atomic::AtomicPtr<Waiter>& queue, Waiter* curr_queue);

    using ::rusty::Cell;
    using ::rusty::UnsafeCell;
    using ::rusty::sync::atomic::AtomicBool;
    using ::rusty::sync::atomic::AtomicPtr;
    using ::rusty::sync::atomic::Ordering;
    namespace thread = rusty::thread;
    using ::rusty::thread::Thread;

    /// Representation of a node in the linked list of waiters in the RUNNING state.
    /// A waiters is stored on the stack of the waiting threads.
    struct Waiter {
        rusty::Cell<rusty::Option<rusty::thread::Thread>> thread;
        rusty::Box<rusty::sync::atomic::AtomicBool> signaled;
        Waiter* next;
    };




    Waiter* const INCOMPLETE_PTR = reinterpret_cast<Waiter*>(static_cast<std::uintptr_t>(INCOMPLETE));

    Waiter* const COMPLETE_PTR = reinterpret_cast<Waiter*>(static_cast<std::uintptr_t>(COMPLETE));


    template<typename T>
    struct OnceCell {
        rusty::Box<rusty::sync::atomic::AtomicPtr<Waiter>> queue;
        rusty::UnsafeCell<rusty::Option<T>> value;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            using namespace imp::tests;
            return std::conditional_t<true, rusty::fmt::Formatter, T>::debug_struct_field2_finish(f, "OnceCell", "queue", &this->queue, "value", &this->value);
        }
        static OnceCell<T> new_() {
            using namespace imp::tests;
            return OnceCell<T>{.queue = rusty::make_box(rusty::sync::atomic::AtomicPtr<Waiter>::new_(INCOMPLETE_PTR)), .value = rusty::UnsafeCell<rusty::Option<T>>::new_(rusty::Option<T>(rusty::None))};
        }
        static OnceCell<T> with_value(T value) {
            using namespace imp::tests;
            return OnceCell<T>{.queue = rusty::make_box(rusty::sync::atomic::AtomicPtr<Waiter>::new_(COMPLETE_PTR)), .value = rusty::UnsafeCell<rusty::Option<T>>::new_(rusty::Option<T>(std::move(value)))};
        }
        bool is_initialized() const {
            using namespace imp::tests;
            return this->queue->load(Ordering::Acquire) == COMPLETE_PTR;
        }
        template<typename F>
        auto initialize(F f) const {
            using E = rusty::result_err_t<decltype((f()))>;
            using namespace imp::tests;
            auto f_shadow1 = rusty::Option<F>(std::move(f));
            rusty::Result<std::tuple<>, E> res = rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
            const std::add_pointer_t<rusty::Option<T>> slot = this->value.get();
            initialize_or_wait(rusty::detail::deref_if_pointer_like(this->queue), rusty::Option<const std::function<bool()>&>([&]() -> auto& { auto _some_mut_ref_value = ([&]() -> bool {
const auto f_shadow2 = f_shadow1.take().unwrap_unchecked();
return [&]() -> bool { auto&& _m = f_shadow2(); if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& value = _mv0; return [&]() -> bool { // @unsafe
{
    *slot = rusty::Option<T>(std::move(value));
}
return true; }(); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& err = _mv1; return [&]() -> bool { res = rusty::Result<std::tuple<>, E>::Err(std::move(err));
return false; }(); } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
}); thread_local std::optional<std::function<bool()>> _some_mut_ref_tmp; _some_mut_ref_tmp.reset(); _some_mut_ref_tmp.emplace(std::move(_some_mut_ref_value)); return *_some_mut_ref_tmp; }()));
            return res;
        }
        void wait() const {
            using namespace imp::tests;
            initialize_or_wait(rusty::detail::deref_if_pointer_like(this->queue), rusty::Option<const std::function<bool()>&>(rusty::None));
        }
        const T& get_unchecked() const {
            using namespace imp::tests;
            if (true) {
                if (!this->is_initialized()) {
                    [&]() -> const T& { rusty::panicking::panic("assertion failed: self.is_initialized()"); }();
                }
            }
            const auto& slot = *this->value.get();
            return slot.as_ref().unwrap_unchecked();
        }
        rusty::Option<T&> get_mut() {
            using namespace imp::tests;
            return (*this->value.get()).as_mut();
        }
        rusty::Option<T> into_inner() {
            using namespace imp::tests;
            return this->value.into_inner();
        }
        void init(const auto& f) const {
            using namespace imp::tests;
            enum class Void {
                
            };
            static_cast<void>(this->initialize([&]() { return rusty::Result<T, Void>::Ok(f()); }));
        }
    };

    /// Drains and notifies the queue of waiters on drop.
    struct Guard {
        const rusty::sync::atomic::AtomicPtr<Waiter>& queue;
        Waiter* new_queue;
        Guard(const rusty::sync::atomic::AtomicPtr<Waiter>& queue_init, Waiter* new_queue_init) : queue(queue_init), new_queue(std::move(new_queue_init)) {}
        Guard(const Guard&) = default;
        Guard(Guard&& other) noexcept : queue(other.queue), new_queue(std::move(other.new_queue)) {
            if (rusty::mem::consume_forgotten_address(&other)) {
                this->rusty_mark_forgotten();
                other.rusty_mark_forgotten();
            } else {
                other.rusty_mark_forgotten();
            }
        }
        Guard& operator=(const Guard&) = default;
        Guard& operator=(Guard&& other) noexcept {
            if (this == &other) {
                return *this;
            }
            this->~Guard();
            new (this) Guard(std::move(other));
            return *this;
        }
        void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


        ~Guard() noexcept(false);
    };

    namespace tests {

        using ::imp::OnceCell;
        void smoke_once();
        void stampede_once();
        void poison_bad();
        void wait_for_force_to_finish();
        void test_size();

        namespace panic = rusty::panic;

        using ::rusty::sync::mpsc::channel;
        namespace thread = rusty::thread;

        using ::imp::OnceCell;


        // Rust-only libtest metadata const skipped: smoke_once (marker: imp::tests::smoke_once, should_panic: no)


        // Rust-only libtest metadata const skipped: stampede_once (marker: imp::tests::stampede_once, should_panic: no)


        // Rust-only libtest metadata const skipped: poison_bad (marker: imp::tests::poison_bad, should_panic: no)


        // Rust-only libtest metadata const skipped: wait_for_force_to_finish (marker: imp::tests::wait_for_force_to_finish, should_panic: no)


        // Rust-only libtest metadata const skipped: test_size (marker: imp::tests::test_size, should_panic: no)

    }

    namespace strict {

        template<typename T>
        size_t addr(std::add_pointer_t<T> ptr);
        template<typename T>
        std::add_pointer_t<T> with_addr(std::add_pointer_t<T> ptr, size_t addr);
        template<typename T>
        std::add_pointer_t<T> map_addr(std::add_pointer_t<T> ptr, const auto& f);

    }

}

namespace sync_mod {

    template<typename T>
    struct OnceCell;
    template<typename T, typename F>
    struct Lazy;
    template<typename... Ts> using Imp = imp::OnceCell<Ts...>;
    void _dummy();

    using ::rusty::Cell;
    namespace fmt = rusty::fmt;
    namespace mem = rusty::mem;

    template<typename... Ts> using Imp = imp::OnceCell<Ts...>;

    /// A thread-safe cell which can be written to only once.
    ///
    /// `OnceCell` provides `&` references to the contents without RAII guards.
    ///
    /// Reading a non-`None` value out of `OnceCell` establishes a
    /// happens-before relationship with a corresponding write. For example, if
    /// thread A initializes the cell with `get_or_init(f)`, and thread B
    /// subsequently reads the result of this call, B also observes all the side
    /// effects of `f`.
    ///
    /// # Example
    /// ```
    /// use once_cell::sync::OnceCell;
    ///
    /// static CELL: OnceCell<String> = OnceCell::new();
    /// assert!(CELL.get().is_none());
    ///
    /// std::thread::spawn(|| {
    ///     let value: &String = CELL.get_or_init(|| {
    ///         "Hello, World!".to_string()
    ///     });
    ///     assert_eq!(value, "Hello, World!");
    /// }).join().unwrap();
    ///
    /// let value: Option<&String> = CELL.get();
    /// assert!(value.is_some());
    /// assert_eq!(value.unwrap().as_str(), "Hello, World!");
    /// ```
    template<typename T>
    struct OnceCell {
        rusty::Box<Imp<T>> _0;

        static OnceCell<T> default_() {
            return OnceCell<T>::new_();
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return [&]() -> rusty::fmt::Result { auto&& _m = this->get(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& v = _mv0; return f.debug_tuple("OnceCell").field(std::move(v)).finish(); } if (_m.is_none()) { return f.write_str("OnceCell(Uninit)"); } return [&]() -> rusty::fmt::Result { rusty::intrinsics::unreachable(); }(); }();
        }
        OnceCell<T> clone() const {
            return [&]() -> OnceCell<T> { auto&& _m = this->get(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& value = _mv0; return OnceCell<T>::with_value(rusty::clone(value)); } if (_m.is_none()) { return OnceCell<T>::new_(); } return [&]() -> OnceCell<T> { rusty::intrinsics::unreachable(); }(); }();
        }
        void clone_from(const OnceCell<T>& source) {
            {
                auto&& _m0 = this->get_mut();
                auto&& _m1 = source.get();
                auto _m_tuple = std::forward_as_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched && ((std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)).is_some() && std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)).is_some()))) {
                    auto&& this_ = std::as_const(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple))).unwrap();
                    auto&& source = std::as_const(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple))).unwrap();
                    this_.clone_from(source);
                    _m_matched = true;
                }
                if (!_m_matched && (true)) {
                    (*this) = rusty::clone(source);
                    _m_matched = true;
                }
            }
        }
        static OnceCell<T> from(T value) {
            return OnceCell<T>::with_value(std::move(value));
        }
        bool operator==(const OnceCell<T>& other) const {
            return this->get() == other.get();
        }
        static OnceCell<T> new_() {
            return OnceCell(rusty::make_box(Imp<T>::new_()));
        }
        static OnceCell<T> with_value(T value) {
            return OnceCell(rusty::make_box(Imp<T>::with_value(std::move(value))));
        }
        rusty::Option<const T&> get() const {
            if (this->_0->is_initialized()) {
                return rusty::Option<const T&>(this->get_unchecked());
            } else {
                return rusty::Option<const T&>(rusty::None);
            }
        }
        const T& wait() const {
            if (!this->_0->is_initialized()) {
                this->_0->wait();
            }
            if (true) {
                if (!this->_0->is_initialized()) {
                    [&]() -> const T& { rusty::panicking::panic("assertion failed: self.0.is_initialized()"); }();
                }
            }
            // @unsafe
            {
                return this->get_unchecked();
            }
        }
        rusty::Option<T&> get_mut() {
            return this->_0->get_mut();
        }
        const T& get_unchecked() const {
            return this->_0->get_unchecked();
        }
        rusty::Result<std::tuple<>, T> set(T value) const {
            return [&]() -> rusty::Result<std::tuple<>, T> { auto&& _m = this->try_insert(std::move(value)); if (_m.is_ok()) { return rusty::Result<std::tuple<>, T>::Ok(std::make_tuple()); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& value = std::get<1>(rusty::detail::deref_if_pointer(_mv1)); return rusty::Result<std::tuple<>, T>::Err(std::move(value)); } return [&]() -> rusty::Result<std::tuple<>, T> { rusty::intrinsics::unreachable(); }(); }();
        }
        rusty::Result<const T&, std::tuple<const T&, T>> try_insert(T value) const {
            auto value_shadow1 = rusty::Option<T>(std::move(value));
            auto& res = this->get_or_init([&]() { return value_shadow1.take().unwrap_unchecked(); });
            return [&]() -> rusty::Result<const T&, std::tuple<const T&, T>> { auto&& _m = value_shadow1; if (_m.is_none()) { return rusty::Result<const T&, std::tuple<const T&, T>>::Ok(res); } if (_m.is_some()) { auto&& _mv1 = _m.unwrap(); auto&& value_shadow1 = _mv1; return rusty::Result<const T&, std::tuple<const T&, T>>::Err(std::tuple<const T&, T>{res, std::move(value_shadow1)}); } return [&]() -> rusty::Result<const T&, std::tuple<const T&, T>> { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename F>
        const T& get_or_init(F f) const {
            enum class Void {
                
            };
            return [&]() -> const T& { auto&& _m = this->get_or_try_init([&]() -> rusty::Result<T, std::tuple<>> { return rusty::Result<T, std::tuple<>>::Ok(f()); }); if (_m.is_ok()) { return _m.unwrap(); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& void_ = _mv1; return [&]() -> const T& { rusty::intrinsics::unreachable(); }(); } return [&]() -> const T& { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename F>
        auto get_or_try_init(F f) const {
            using E = rusty::result_err_t<decltype((f()))>;
            if (auto&& _iflet_scrutinee = this->get(); _iflet_scrutinee.is_some()) {
                decltype(auto) value = _iflet_scrutinee.unwrap();
                return rusty::Result<const T&, E>::Ok(value);
            }
            RUSTY_TRY_INTO(this->_0->initialize(std::move(f)), rusty::Result<const T&, E>);
            if (true) {
                if (!this->_0->is_initialized()) {
                    [&]() -> rusty::Result<const T&, E> { rusty::panicking::panic("assertion failed: self.0.is_initialized()"); }();
                }
            }
            return rusty::Result<const T&, E>::Ok(this->get_unchecked());
        }
        rusty::Option<T> take() {
            return (*this).take().into_inner();
        }
        rusty::Option<T> into_inner() {
            return this->_0->into_inner();
        }
    };

    /// A value which is initialized on the first access.
    ///
    /// This type is thread-safe and can be used in statics.
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// use once_cell::sync::Lazy;
    ///
    /// static HASHMAP: Lazy<HashMap<i32, String>> = Lazy::new(|| {
    ///     println!("initializing");
    ///     let mut m = HashMap::new();
    ///     m.insert(13, "Spica".to_string());
    ///     m.insert(74, "Hoyten".to_string());
    ///     m
    /// });
    ///
    /// fn main() {
    ///     println!("ready");
    ///     std::thread::spawn(|| {
    ///         println!("{:?}", HASHMAP.get(&13));
    ///     }).join().unwrap();
    ///     println!("{:?}", HASHMAP.get(&74));
    ///
    ///     // Prints:
    ///     //   ready
    ///     //   initializing
    ///     //   Some("Spica")
    ///     //   Some("Hoyten")
    /// }
    /// ```
    template<typename T, typename F = rusty::SafeFn<T()>>
    struct Lazy {
        using Target = T;
        OnceCell<T> cell;
        rusty::Cell<rusty::Option<F>> init;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("Lazy").field("cell", &this->cell).field("init", rusty::addr_of_temp("..")).finish();
        }
        static Lazy<T, F> new_(F f) {
            return Lazy<T, F>(OnceCell<T>::new_(), rusty::Cell<rusty::Option<F>>::new_(rusty::Option<F>(std::move(f))));
        }
        static rusty::Result<T, F> into_value(Lazy<T, F> this_) {
            auto cell = std::move(this_.cell);
            const auto init = std::move(this_.init);
            return cell.into_inner().ok_or_else([&]() {
return init.take().unwrap_or_else([&]() {
rusty::panicking::panic_fmt(std::string("Lazy instance has previously been poisoned"));
});
});
        }
        static const T& force(const Lazy<T, F>& this_) {
            return this_.cell.get_or_init([&]() { return [&]() { auto&& _m = this_.init.take(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& f = _mv0; return f(); } if (_m.is_none()) { [&]() { rusty::panicking::panic_fmt(std::string("Lazy instance has previously been poisoned")); }(); } rusty::intrinsics::unreachable(); }(); });
        }
        static T& force_mut(Lazy<T, F>& this_) {
            if (this_.cell.get_mut().is_none()) {
                auto value = [&]() { auto&& _m = this_.init.get_mut()->take(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& f = _mv0; return f(); } if (_m.is_none()) { [&]() { [&]() -> T& { rusty::panicking::panic_fmt(std::string("Lazy instance has previously been poisoned")); }(); }(); } rusty::intrinsics::unreachable(); }();
                this_.cell = OnceCell<T>::with_value(std::move(value));
            }
            return this_.cell.get_mut().unwrap_or_else([&]() { rusty::panicking::panic("internal error: entered unreachable code"); });
        }
        static rusty::Option<const T&> get(const Lazy<T, F>& this_) {
            return this_.cell.get();
        }
        static rusty::Option<T&> get_mut(Lazy<T, F>& this_) {
            return this_.cell.get_mut();
        }
        const T& operator*() const {
            return Lazy<T>::force((*this));
        }
        T& operator*() {
            return Lazy<T>::force_mut((*this));
        }
        static Lazy<T> default_() {
            return Lazy<T>::new_([&]() { return rusty::default_value<T>(); });
        }
    };

}

// Rust-only libtest main omitted

namespace imp {
    namespace strict {}
    namespace tests {}

    namespace tests {

        void smoke_once() {
            static imp::OnceCell<std::tuple<>> O = imp::OnceCell<std::tuple<>>::new_();
            auto a = static_cast<int32_t>(0);
            O.init([&]() -> std::tuple<> { return [&]() { static_cast<void>(a += 1); return std::make_tuple(); }(); });
            {
                auto _m0 = &a;
                auto&& _m1_tmp = static_cast<int32_t>(1);
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                    auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                    }
                    _m_matched = true;
                }
            }
            O.init([&]() -> std::tuple<> { return [&]() { static_cast<void>(a += 1); return std::make_tuple(); }(); });
            {
                auto _m0 = &a;
                auto&& _m1_tmp = static_cast<int32_t>(1);
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                    auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                    }
                    _m_matched = true;
                }
            }
        }

        void stampede_once() {
            static imp::OnceCell<std::tuple<>> O = imp::OnceCell<std::tuple<>>::new_();
            static bool RUN = false;
            auto [tx, rx] = rusty::detail::deref_if_pointer_like(channel());
            for (auto&& _ : rusty::for_in(rusty::range(0, 10))) {
                const auto tx_shadow1 = rusty::clone(tx);
                rusty::thread::spawn([=, tx_shadow1 = std::move(tx_shadow1)]() mutable {
for (auto&& _ : rusty::for_in(rusty::range(0, 4))) {
    rusty::thread::yield_now();
}
// @unsafe
{
    O.init([&]() -> std::tuple<> {
if (!!RUN) {
    [&]() -> std::tuple<> { rusty::panicking::panic("assertion failed: !RUN"); }();
}
RUN = true;
return std::make_tuple();
});
    if (!RUN) {
        rusty::panicking::panic("assertion failed: RUN");
    }
}
tx_shadow1.send(std::make_tuple()).unwrap();
});
            }
            // @unsafe
            {
                O.init([&]() -> std::tuple<> {
if (!!RUN) {
    [&]() -> std::tuple<> { rusty::panicking::panic("assertion failed: !RUN"); }();
}
RUN = true;
return std::make_tuple();
});
                if (!RUN) {
                    rusty::panicking::panic("assertion failed: RUN");
                }
            }
            for (auto&& _ : rusty::for_in(rusty::range(0, 10))) {
                rx.recv().unwrap();
            }
        }

        void poison_bad() {
            static imp::OnceCell<std::tuple<>> O = imp::OnceCell<std::tuple<>>::new_();
            const auto t = rusty::panic::catch_unwind([&]() {
O.init([&]() -> std::tuple<> { [&]() -> std::tuple<> { rusty::panicking::panic("explicit panic"); }(); });
});
            if (!t.is_err()) {
                rusty::panicking::panic("assertion failed: t.is_err()");
            }
            auto called = false;
            O.init([&]() -> std::tuple<> {
called = true;
return std::make_tuple();
});
            if (!called) {
                rusty::panicking::panic("assertion failed: called");
            }
            O.init([&]() -> std::tuple<> {
return std::make_tuple();
});
        }

        void wait_for_force_to_finish() {
            static imp::OnceCell<std::tuple<>> O = imp::OnceCell<std::tuple<>>::new_();
            const auto t = rusty::panic::catch_unwind([&]() {
O.init([&]() -> std::tuple<> { [&]() -> std::tuple<> { rusty::panicking::panic("explicit panic"); }(); });
});
            if (!t.is_err()) {
                rusty::panicking::panic("assertion failed: t.is_err()");
            }
            auto [tx1, rx1] = rusty::detail::deref_if_pointer_like(channel());
            auto [tx2, rx2] = rusty::detail::deref_if_pointer_like(channel());
            const auto t1 = rusty::thread::spawn([=, rx2 = std::move(rx2), tx1 = std::move(tx1)]() mutable {
O.init([&]() -> std::tuple<> {
tx1.send(std::make_tuple()).unwrap();
rx2.recv().unwrap();
return std::make_tuple();
});
});
            rx1.recv().unwrap();
            const auto t2 = rusty::thread::spawn([&]() {
auto called = false;
O.init([&]() -> std::tuple<> {
called = true;
return std::make_tuple();
});
if (!!called) {
    return rusty::panicking::panic("assertion failed: !called");
}
});
            tx2.send(std::make_tuple()).unwrap();
            if (!t1.join().is_ok()) {
                rusty::panicking::panic("assertion failed: t1.join().is_ok()");
            }
            if (!t2.join().is_ok()) {
                rusty::panicking::panic("assertion failed: t2.join().is_ok()");
            }
        }

        void test_size() {
            using ::rusty::mem::size_of;
            {
                auto&& _m0_tmp = size_of<imp::OnceCell<uint32_t>>();
                auto _m0 = &_m0_tmp;
                auto&& _m1_tmp = 4 * size_of<uint32_t>();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                    auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                    }
                    _m_matched = true;
                }
            }
        }

    }

    namespace strict {

        template<typename T>
        size_t addr(std::add_pointer_t<T> ptr) {
            // @unsafe
            {
                return rusty::mem::transmute<std::remove_cvref_t<decltype((std::move(ptr)))>, size_t>(std::move(ptr));
            }
        }

        template<typename T>
        std::add_pointer_t<T> with_addr(std::add_pointer_t<T> ptr, size_t addr) {
            const auto self_addr = static_cast<ptrdiff_t>(imp::strict::addr<std::remove_pointer_t<std::remove_cvref_t<decltype((std::move(ptr)))>>>(std::move(ptr)));
            const auto dest_addr = static_cast<ptrdiff_t>(addr);
            const auto offset = (static_cast<size_t>(dest_addr) - static_cast<size_t>(std::move(self_addr)));
            return const_cast<std::add_pointer_t<T>>(reinterpret_cast<std::add_pointer_t<std::add_const_t<T>>>(rusty::ptr::offset(((const_cast<uint8_t*>(reinterpret_cast<const uint8_t*>(ptr)))), std::move(offset))));
        }

        template<typename T>
        std::add_pointer_t<T> map_addr(std::add_pointer_t<T> ptr, const auto& f) {
            return imp::strict::with_addr<std::remove_pointer_t<std::remove_cvref_t<decltype((std::move(ptr)))>>>(std::move(ptr), f(addr<std::remove_pointer_t<std::remove_cvref_t<decltype((std::move(ptr)))>>>(std::move(ptr))));
        }

    }

    void initialize_or_wait(const rusty::sync::atomic::AtomicPtr<Waiter>& queue, rusty::Option<const std::function<bool()>&> init) {
        auto curr_queue = queue.load(Ordering::Acquire);
        while (true) {
            auto curr_state = strict::addr<std::remove_pointer_t<std::remove_cvref_t<decltype((std::move(curr_queue)))>>>(std::move(curr_queue)) & STATE_MASK;
            {
                auto&& _m0 = curr_state;
                auto&& _m1 = init;
                auto _m_tuple = std::forward_as_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched && (std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)) == COMPLETE)) {
                    return;
                    _m_matched = true;
                }
                if (!_m_matched && ((std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)) == INCOMPLETE && std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)).is_some()))) {
                    auto&& init = std::as_const(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple))).unwrap();
                    const auto exchange = queue.compare_exchange(std::move(curr_queue), strict::map_addr<std::remove_pointer_t<std::remove_cvref_t<decltype((std::move(curr_queue)))>>>(std::move(curr_queue), [&](auto&& q) -> size_t { return ((q & ~STATE_MASK)) | RUNNING; }), Ordering::Acquire, Ordering::Acquire);
                    if (exchange.is_err()) {
                        decltype(auto) new_queue = exchange.unwrap_err();
                        curr_queue = new_queue;
                        continue;
                    }
                    auto guard = Guard(queue, INCOMPLETE_PTR);
                    if (init()) {
                        guard.new_queue = COMPLETE_PTR;
                    }
                    return;
                    _m_matched = true;
                }
                if (!_m_matched && (((std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)) == INCOMPLETE && std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)).is_none()) || std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)) == RUNNING))) {
                    wait(rusty::detail::deref_if_pointer_like(queue), std::move(curr_queue));
                    curr_queue = queue.load(Ordering::Acquire);
                    _m_matched = true;
                }
                if (!_m_matched && (true)) {
                    if (true) {
                        if (!false) {
                            rusty::panicking::panic("assertion failed: false");
                        }
                    }
                    _m_matched = true;
                }
            }
        }
    }

    void wait(const rusty::sync::atomic::AtomicPtr<Waiter>& queue, Waiter* curr_queue) {
        const auto curr_state = strict::addr<std::remove_pointer_t<std::remove_cvref_t<decltype((std::move(curr_queue)))>>>(std::move(curr_queue)) & STATE_MASK;
        while (true) {
            auto node = Waiter{.thread = rusty::Cell<rusty::Option<rusty::thread::Thread>>::new_(rusty::Option<rusty::thread::Thread>(rusty::thread::current())), .signaled = rusty::make_box(AtomicBool::new_(false)), .next = strict::map_addr<std::remove_pointer_t<std::remove_cvref_t<decltype((std::move(curr_queue)))>>>(std::move(curr_queue), [&](auto&& q) -> size_t { return q & ~STATE_MASK; })};
            const auto me = const_cast<Waiter*>(reinterpret_cast<const Waiter*>(static_cast<const Waiter*>(&node)));
            const auto exchange = queue.compare_exchange(std::move(curr_queue), strict::map_addr<std::remove_pointer_t<std::remove_cvref_t<decltype((std::move(me)))>>>(std::move(me), [&](auto&& q) -> size_t { return q | curr_state; }), Ordering::Release, Ordering::Relaxed);
            if (exchange.is_err()) {
                decltype(auto) new_queue = exchange.unwrap_err();
                if ((strict::addr<std::remove_pointer_t<std::remove_cvref_t<decltype((std::move(new_queue)))>>>(std::move(new_queue)) & STATE_MASK) != curr_state) {
                    return;
                }
                curr_queue = new_queue;
                continue;
            }
            while (!node.signaled->load(Ordering::Acquire)) {
                rusty::thread::park();
            }
            break;
        }
    }

}

namespace sync_mod {

    /// ```compile_fail
    /// struct S(*mut ());
    /// unsafe impl Sync for S {}
    ///
    /// fn share<T: Sync>(_: &T) {}
    /// share(&once_cell::sync::OnceCell::<S>::new());
    /// ```
    ///
    /// ```compile_fail
    /// struct S(*mut ());
    /// unsafe impl Sync for S {}
    ///
    /// fn share<T: Sync>(_: &T) {}
    /// share(&once_cell::sync::Lazy::<S>::new(|| unimplemented!()));
    /// ```
    void _dummy() {
    }

}


namespace race {
    OnceNonZeroUsize OnceNonZeroUsize::default_() {
        return OnceNonZeroUsize{.inner = rusty::default_value<rusty::sync::atomic::AtomicUsize>()};
    }
}

namespace race {
    rusty::fmt::Result OnceNonZeroUsize::fmt(rusty::fmt::Formatter& f) const {
        return rusty::fmt::Formatter::debug_struct_field1_finish(f, "OnceNonZeroUsize", "inner", &this->inner);
    }
}

namespace race {
    OnceNonZeroUsize OnceNonZeroUsize::new_() {
        return OnceNonZeroUsize{.inner = AtomicUsize::new_(0)};
    }
}

namespace race {
    rusty::Option<rusty::num::NonZeroUsize> OnceNonZeroUsize::get() const {
        auto val = this->inner.load(Ordering::Acquire);
        return NonZeroUsize::new_(std::move(val));
    }
}

namespace race {
    rusty::num::NonZeroUsize OnceNonZeroUsize::get_unchecked() const {
        const rusty::SafeFn<const size_t*(const rusty::sync::atomic::AtomicUsize&)> as_const_ptr = +[](const rusty::sync::atomic::AtomicUsize& r) -> const size_t* {
            using ::rusty::mem::align_of;
            const std::tuple<> _ALIGNMENT_COMPATIBLE = [&]() { if (!((align_of<rusty::sync::atomic::AtomicUsize>() % align_of<size_t>()) == 0)) { [&]() -> const size_t* { rusty::panicking::panic("assertion failed: align_of::<AtomicUsize>() % align_of::<usize>() == 0"); }(); } return std::tuple<>(); }();
            const rusty::sync::atomic::AtomicUsize* p = r;
            return reinterpret_cast<const size_t*>(p);
        };
        const auto p = as_const_ptr(rusty::detail::deref_if_pointer_like(this->inner));
        const auto val = rusty::ptr::read(p);
        // @unsafe
        {
            return rusty::num::NonZeroUsize::new_unchecked(std::move(val));
        }
    }
}

namespace race {
    rusty::Result<std::tuple<>, std::tuple<>> OnceNonZeroUsize::set(rusty::num::NonZeroUsize value) const {
        return [&]() -> rusty::Result<std::tuple<>, std::tuple<>> { auto&& _m = this->compare_exchange(std::move(value)); if (_m.is_ok()) { return rusty::Result<std::tuple<>, std::tuple<>>::Ok(std::make_tuple()); } if (_m.is_err()) { return rusty::Result<std::tuple<>, std::tuple<>>::Err(std::make_tuple()); } return [&]() -> rusty::Result<std::tuple<>, std::tuple<>> { rusty::intrinsics::unreachable(); }(); }();
    }
}

namespace race {
    template<typename F>
    rusty::num::NonZeroUsize OnceNonZeroUsize::get_or_init(F f) const {
        enum class Void {
            
        };
        return [&]() -> rusty::num::NonZeroUsize { auto&& _m = this->get_or_try_init([&]() -> rusty::Result<rusty::num::NonZeroUsize, std::tuple<>> { return rusty::Result<rusty::num::NonZeroUsize, std::tuple<>>::Ok(f()); }); if (_m.is_ok()) { return _m.unwrap(); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& void_ = _mv1; return [&]() -> rusty::num::NonZeroUsize { rusty::intrinsics::unreachable(); }(); } return [&]() -> rusty::num::NonZeroUsize { rusty::intrinsics::unreachable(); }(); }();
    }
}

namespace race {
    template<typename F>
    auto OnceNonZeroUsize::get_or_try_init(F f) const {
        using E = rusty::result_err_t<decltype((f()))>;
        return [&]() -> rusty::Result<rusty::num::NonZeroUsize, E> { auto&& _m = this->get(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& it = _mv0; return rusty::Result<rusty::num::NonZeroUsize, E>::Ok(std::move(it)); } if (_m.is_none()) { return this->template init<E>(std::move(f)); } return [&]() -> rusty::Result<rusty::num::NonZeroUsize, E> { rusty::intrinsics::unreachable(); }(); }();
    }
}

namespace race {
    template<typename E>
    rusty::Result<rusty::num::NonZeroUsize, E> OnceNonZeroUsize::init(const auto& f) const {
        const auto nz = RUSTY_TRY_INTO(f(), rusty::Result<rusty::num::NonZeroUsize, E>);
        auto val = nz.get();
        if (auto&& _iflet_scrutinee = this->compare_exchange(std::move(nz)); _iflet_scrutinee.is_err()) {
            decltype(auto) old = _iflet_scrutinee.unwrap_err();
            val = old;
        }
        return rusty::Result<rusty::num::NonZeroUsize, E>::Ok(std::conditional_t<true, NonZeroUsize, E>::new_unchecked(std::move(val)));
    }
}

namespace race {
    rusty::Result<size_t, size_t> OnceNonZeroUsize::compare_exchange(rusty::num::NonZeroUsize val) const {
        return this->inner.compare_exchange(0, val.get(), Ordering::Release, Ordering::Acquire);
    }
}

namespace race {
    OnceBool OnceBool::default_() {
        return OnceBool{.inner = rusty::default_value<OnceNonZeroUsize>()};
    }
}

namespace race {
    rusty::fmt::Result OnceBool::fmt(rusty::fmt::Formatter& f) const {
        return rusty::fmt::Formatter::debug_struct_field1_finish(f, "OnceBool", "inner", &this->inner);
    }
}

namespace race {
    OnceBool OnceBool::new_() {
        return OnceBool{.inner = OnceNonZeroUsize::new_()};
    }
}

namespace race {
    rusty::Option<bool> OnceBool::get() const {
        return this->inner.get().map(OnceBool::from_usize);
    }
}

namespace race {
    rusty::Result<std::tuple<>, std::tuple<>> OnceBool::set(bool value) const {
        return this->inner.set(OnceBool::to_usize(std::move(value)));
    }
}

namespace race {
    template<typename F>
    bool OnceBool::get_or_init(F f) const {
        return std::conditional_t<true, OnceBool, F>::from_usize(this->inner.get_or_init([&]() -> rusty::num::NonZeroUsize { return std::conditional_t<true, OnceBool, F>::to_usize(f()); }));
    }
}

namespace race {
    template<typename F>
    auto OnceBool::get_or_try_init(F f) const {
        using E = rusty::result_err_t<decltype((f()))>;
        return this->inner.get_or_try_init([&]() -> rusty::Result<rusty::num::NonZeroUsize, std::tuple<>> { return f().map(OnceBool::to_usize); }).map(OnceBool::from_usize);
    }
}

namespace race {
    bool OnceBool::from_usize(rusty::num::NonZeroUsize value) {
        return value.get() == 1;
    }
}

namespace race {
    rusty::num::NonZeroUsize OnceBool::to_usize(bool value) {
        // @unsafe
        {
            return rusty::num::NonZeroUsize::new_unchecked((value ? 1 : 2));
        }
    }
}

namespace imp {
    Guard::~Guard() noexcept(false) {
        if (rusty::mem::consume_forgotten_address(this)) { return; }
        const auto queue = this->queue.swap(this->new_queue, Ordering::AcqRel);
        const auto state = strict::addr<std::remove_pointer_t<std::remove_cvref_t<decltype((std::move(queue)))>>>(std::move(queue)) & STATE_MASK;
        {
            auto _m0 = &state;
            auto&& _m1_tmp = RUNNING;
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        // @unsafe
        {
            auto waiter = strict::map_addr<std::remove_pointer_t<std::remove_cvref_t<decltype((std::move(queue)))>>>(std::move(queue), [&](auto&& q) -> size_t { return q & ~STATE_MASK; });
            while (!(waiter == nullptr)) {
                const auto next = (*waiter).next;
                const auto thread_shadow1 = (*waiter).thread.take().unwrap();
                (*waiter).signaled->store(true, Ordering::Release);
                waiter = next;
                thread_shadow1.unpark();
            }
        }
    }
}


// Runnable wrappers for expanded Rust test bodies
// Rust-only libtest wrapper metadata: marker=imp::tests::smoke_once should_panic=no
void rusty_test_imp_tests_smoke_once() {
    imp::tests::smoke_once();
}
// Rust-only libtest wrapper metadata: marker=imp::tests::stampede_once should_panic=no
void rusty_test_imp_tests_stampede_once() {
    imp::tests::stampede_once();
}
// Rust-only libtest wrapper metadata: marker=imp::tests::poison_bad should_panic=no
void rusty_test_imp_tests_poison_bad() {
    imp::tests::poison_bad();
}
// Rust-only libtest wrapper metadata: marker=imp::tests::wait_for_force_to_finish should_panic=no
void rusty_test_imp_tests_wait_for_force_to_finish() {
    imp::tests::wait_for_force_to_finish();
}
// Rust-only libtest wrapper metadata: marker=imp::tests::test_size should_panic=no
void rusty_test_imp_tests_test_size() {
    imp::tests::test_size();
}

// ── from it.cppm ──

namespace unsync_once_cell {
    void once_cell();
    void once_cell_with_value();
    void once_cell_get_mut();
    void once_cell_drop();
    void once_cell_drop_empty();
    void clone();
    void get_or_try_init();
    void from_impl();
    void partialeq_impl();
    void into_inner();
    void debug_impl();
    void reentrant_init();
    void aliasing_in_get();
    void arrrrrrrrrrrrrrrrrrrrrr();
}
namespace sync_once_cell {
    void once_cell();
    void once_cell_with_value();
    void once_cell_get_mut();
    void once_cell_get_unchecked();
    void once_cell_drop();
    void once_cell_drop_empty();
    void clone();
    void get_or_try_init();
    void wait();
    void wait_panic();
    void get_or_init_stress();
    void from_impl();
    void partialeq_impl();
    void into_inner();
    void debug_impl();
    void reentrant_init();
    void eval_once_macro();
    void once_cell_does_not_leak_partially_constructed_boxes();
    void get_does_not_block();
    void arrrrrrrrrrrrrrrrrrrrrr();
    void once_cell_is_sync_send();
}
namespace unsync_lazy {
    void lazy_new();
    void lazy_deref_mut();
    void lazy_force_mut();
    void lazy_get_mut();
    void lazy_default();
    void lazy_into_value();
    void lazy_poisoning();
    void arrrrrrrrrrrrrrrrrrrrrr();
}
namespace sync_lazy {
    void lazy_new();
    void lazy_deref_mut();
    void lazy_force_mut();
    void lazy_get_mut();
    void lazy_default();
    void static_lazy();
    void static_lazy_via_fn();
    void lazy_into_value();
    void lazy_poisoning();
    void arrrrrrrrrrrrrrrrrrrrrr();
    void lazy_is_sync_send();
}
namespace race {
    void once_non_zero_usize_smoke_test();
    void once_non_zero_usize_set();
    void once_non_zero_usize_first_wins();
    void once_bool_smoke_test();
    void once_bool_set();
    void once_bool_get_or_try_init();
    void once_ref_smoke_test();
    void once_ref_set();
    void get_unchecked();
}
namespace race_once_box {
    struct Heap;
    template<typename T>
    struct Pebble;
    void once_box_smoke_test();
    void once_box_set();
    void once_box_first_wins();
    void once_box_reentrant();
    void once_box_default();
    void onece_box_with_value();
    void onece_box_clone();
}


namespace race_once_box {

    struct Heap;
    template<typename T>
    struct Pebble;
    void once_box_smoke_test();
    void once_box_set();
    void once_box_first_wins();
    void once_box_reentrant();
    void once_box_default();
    void onece_box_with_value();
    void onece_box_clone();

    using ::rusty::Barrier;

    using ::rusty::sync::atomic::AtomicUsize;
    constexpr auto SeqCst = rusty::sync::atomic::Ordering::SeqCst;
    using ::rusty::Arc;

    using ::race::OnceBox;

    struct Heap {
        rusty::Arc<rusty::sync::atomic::AtomicUsize> total_field;

        static Heap default_();
        size_t total() const;
        template<typename T>
        Pebble<T> new_pebble(T val) const;
    };

    template<typename T>
    struct Pebble {
        T val;
        rusty::Arc<rusty::sync::atomic::AtomicUsize> total;
        Pebble(T val_init, rusty::Arc<rusty::sync::atomic::AtomicUsize> total_init) : val(std::move(val_init)), total(std::move(total_init)) {}
        Pebble(const Pebble&) = default;
        Pebble(Pebble&& other) noexcept : val(std::move(other.val)), total(std::move(other.total)) {
            if (rusty::mem::consume_forgotten_address(&other)) {
                this->rusty_mark_forgotten();
                other.rusty_mark_forgotten();
            } else {
                other.rusty_mark_forgotten();
            }
        }
        Pebble& operator=(const Pebble&) = default;
        Pebble& operator=(Pebble&& other) noexcept {
            if (this == &other) {
                return *this;
            }
            this->~Pebble();
            new (this) Pebble(std::move(other));
            return *this;
        }
        void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, T>::debug_struct_field2_finish(f, "Pebble", "val", &this->val, "total", &this->total);
        }
        ~Pebble() noexcept(false) {
            if (rusty::mem::consume_forgotten_address(this)) { return; }
            this->total->fetch_sub(1, rusty::sync::atomic::Ordering::SeqCst);
        }
    };


    // Rust-only libtest metadata const skipped: once_box_smoke_test (marker: race_once_box::once_box_smoke_test, should_panic: no)


    // Rust-only libtest metadata const skipped: once_box_set (marker: race_once_box::once_box_set, should_panic: no)


    // Rust-only libtest metadata const skipped: once_box_first_wins (marker: race_once_box::once_box_first_wins, should_panic: no)


    // Rust-only libtest metadata const skipped: once_box_reentrant (marker: race_once_box::once_box_reentrant, should_panic: no)


    // Rust-only libtest metadata const skipped: once_box_default (marker: race_once_box::once_box_default, should_panic: no)


    // Rust-only libtest metadata const skipped: onece_box_with_value (marker: race_once_box::onece_box_with_value, should_panic: no)


    // Rust-only libtest metadata const skipped: onece_box_clone (marker: race_once_box::onece_box_clone, should_panic: no)

}

namespace unsync_once_cell {

    void once_cell();
    void once_cell_with_value();
    void once_cell_get_mut();
    void once_cell_drop();
    void once_cell_drop_empty();
    void clone();
    void get_or_try_init();
    void from_impl();
    void partialeq_impl();
    void into_inner();
    void debug_impl();
    void reentrant_init();
    void aliasing_in_get();
    void arrrrrrrrrrrrrrrrrrrrrr();

    using ::rusty::Cell;
    using ::rusty::sync::atomic::AtomicUsize;
    constexpr auto SeqCst = rusty::sync::atomic::Ordering::SeqCst;

    using ::unsync::OnceCell;


    // Rust-only libtest metadata const skipped: once_cell (marker: unsync_once_cell::once_cell, should_panic: no)


    // Rust-only libtest metadata const skipped: once_cell_with_value (marker: unsync_once_cell::once_cell_with_value, should_panic: no)


    // Rust-only libtest metadata const skipped: once_cell_get_mut (marker: unsync_once_cell::once_cell_get_mut, should_panic: no)


    // Rust-only libtest metadata const skipped: once_cell_drop (marker: unsync_once_cell::once_cell_drop, should_panic: no)


    // Rust-only libtest metadata const skipped: once_cell_drop_empty (marker: unsync_once_cell::once_cell_drop_empty, should_panic: no)


    // Rust-only libtest metadata const skipped: clone (marker: unsync_once_cell::clone, should_panic: no)


    // Rust-only libtest metadata const skipped: get_or_try_init (marker: unsync_once_cell::get_or_try_init, should_panic: no)


    // Rust-only libtest metadata const skipped: from_impl (marker: unsync_once_cell::from_impl, should_panic: no)


    // Rust-only libtest metadata const skipped: partialeq_impl (marker: unsync_once_cell::partialeq_impl, should_panic: no)


    // Rust-only libtest metadata const skipped: into_inner (marker: unsync_once_cell::into_inner, should_panic: no)


    // Rust-only libtest metadata const skipped: debug_impl (marker: unsync_once_cell::debug_impl, should_panic: no)


    // Rust-only libtest metadata const skipped: reentrant_init (marker: unsync_once_cell::reentrant_init, should_panic: yes)


    // Rust-only libtest metadata const skipped: aliasing_in_get (marker: unsync_once_cell::aliasing_in_get, should_panic: no)


    // Rust-only libtest metadata const skipped: arrrrrrrrrrrrrrrrrrrrrr (marker: unsync_once_cell::arrrrrrrrrrrrrrrrrrrrrr, should_panic: no)

    void once_cell() {
        const auto c = unsync::OnceCell<int32_t>::new_();
        if (!c.get().is_none()) {
            rusty::panicking::panic("assertion failed: c.get().is_none()");
        }
        c.get_or_init([&]() -> int32_t { return static_cast<int32_t>(92); });
        {
            auto&& _m0_tmp = c.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<const int32_t&>([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<int32_t>(92); return _some_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        c.get_or_init([&]() -> int32_t {
[&]() -> int32_t { rusty::panicking::panic_fmt(std::string("Kabom!")); }();
});
        {
            auto&& _m0_tmp = c.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<const int32_t&>([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<int32_t>(92); return _some_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_cell_with_value() {
        const unsync::OnceCell<int32_t> CELL = unsync::OnceCell<int32_t>::with_value(12);
        const auto cell = rusty::clone(CELL);
        {
            auto&& _m0_tmp = cell.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<const int32_t&>([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<int32_t>(12); return _some_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_cell_get_mut() {
        auto c = unsync::OnceCell<int32_t>::new_();
        if (!c.get_mut().is_none()) {
            rusty::panicking::panic("assertion failed: c.get_mut().is_none()");
        }
        c.set(static_cast<int32_t>(90)).unwrap();
        [&]() { static_cast<void>(c.get_mut().unwrap() += 2); return std::make_tuple(); }();
        {
            auto&& _m0_tmp = c.get_mut();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<int32_t&>([&]() -> auto& { auto _some_mut_ref_value = (static_cast<int32_t>(92)); thread_local std::optional<int32_t> _some_mut_ref_tmp; _some_mut_ref_tmp.reset(); _some_mut_ref_tmp.emplace(std::move(_some_mut_ref_value)); return *_some_mut_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_cell_drop() {
        static rusty::sync::atomic::AtomicUsize DROP_CNT = AtomicUsize::new_(0);
        struct Dropper {
            Dropper() = default;
            Dropper(const Dropper&) = default;
            Dropper(Dropper&& other) noexcept {
                if (rusty::mem::consume_forgotten_address(&other)) {
                    this->rusty_mark_forgotten();
                    other.rusty_mark_forgotten();
                } else {
                    other.rusty_mark_forgotten();
                }
            }
            Dropper& operator=(const Dropper&) = default;
            Dropper& operator=(Dropper&& other) noexcept {
                if (this == &other) {
                    return *this;
                }
                this->~Dropper();
                new (this) Dropper(std::move(other));
                return *this;
            }
            void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


            ~Dropper() noexcept(false) {
                if (rusty::mem::consume_forgotten_address(this)) { return; }
                DROP_CNT.fetch_add(1, rusty::sync::atomic::Ordering::SeqCst);
            }
        };
        // Rust-only nested impl block skipped in local scope
        auto x = unsync::OnceCell<Dropper>::new_();
        x.get_or_init([&]() -> Dropper { return Dropper{}; });
        {
            auto&& _m0_tmp = DROP_CNT.load(rusty::sync::atomic::Ordering::SeqCst);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        rusty::mem::drop(std::move(x));
        {
            auto&& _m0_tmp = DROP_CNT.load(rusty::sync::atomic::Ordering::SeqCst);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_cell_drop_empty() {
        auto x = unsync::OnceCell<rusty::String>::new_();
        rusty::mem::drop(std::move(x));
    }

    void clone() {
        const auto s = unsync::OnceCell<rusty::String>::new_();
        const auto c = rusty::clone(s);
        if (!c.get().is_none()) {
            rusty::panicking::panic("assertion failed: c.get().is_none()");
        }
        s.set(rusty::String::from(rusty::to_string("hello"))).unwrap();
        const auto c_shadow1 = rusty::clone(s);
        {
            auto&& _m0_tmp = c_shadow1.get().map([](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.as_str(std::forward<decltype(_args)>(_args)...); });
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Some("hello");
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void get_or_try_init() {
        const unsync::OnceCell<rusty::String> cell = unsync::OnceCell<rusty::String>::new_();
        if (!cell.get().is_none()) {
            rusty::panicking::panic("assertion failed: cell.get().is_none()");
        }
        const auto res = rusty::panic::catch_unwind([&]() {
return cell.get_or_try_init([&]() -> rusty::Result<rusty::String, std::tuple<>> {
return [&]() -> rusty::Result<rusty::String, std::tuple<>> { rusty::panicking::panic("explicit panic"); }();
});
});
        if (!res.is_err()) {
            rusty::panicking::panic("assertion failed: res.is_err()");
        }
        if (!cell.get().is_none()) {
            rusty::panicking::panic("assertion failed: cell.get().is_none()");
        }
        {
            auto&& _m0_tmp = cell.get_or_try_init([&]() -> rusty::Result<rusty::String, std::tuple<>> { return rusty::Result<rusty::String, std::tuple<>>::Err(std::make_tuple()); });
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = [&]() { using _ResultCtorCtx = std::remove_cvref_t<decltype((cell.get_or_try_init([&]() -> rusty::Result<rusty::String, std::tuple<>> { return rusty::Result<rusty::String, std::tuple<>>::Err(std::make_tuple()); })))>; return _ResultCtorCtx::Err(std::make_tuple()); }();
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = cell.get_or_try_init([&]() -> rusty::Result<rusty::String, std::tuple<>> { return rusty::Result<rusty::String, std::tuple<>>::Ok(rusty::String::from(rusty::to_string("hello"))); });
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = [&]() { using _ResultCtorCtx = std::remove_cvref_t<decltype((cell.get_or_try_init([&]() -> rusty::Result<rusty::String, std::tuple<>> { return rusty::Result<rusty::String, std::tuple<>>::Ok(rusty::String::from(rusty::to_string("hello"))); })))>; using _ResultCtorArg = rusty::result_ok_t<_ResultCtorCtx>; if constexpr (std::is_reference_v<_ResultCtorArg>) { using _ResultCtorStorage = std::remove_cvref_t<_ResultCtorArg>; auto _result_ctor_value = (rusty::String::from(rusty::to_string("hello"))); thread_local std::optional<_ResultCtorStorage> _result_ctor_tmp; _result_ctor_tmp.reset(); _result_ctor_tmp.emplace(static_cast<_ResultCtorStorage>(std::move(_result_ctor_value))); return _ResultCtorCtx::Ok(static_cast<_ResultCtorArg>(*_result_ctor_tmp)); } else { return _ResultCtorCtx::Ok(rusty::String::from(rusty::to_string("hello"))); } }();
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = cell.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<const rusty::String&>([&]() -> const auto& { static const auto _some_ref_tmp = rusty::String::from(rusty::to_string("hello")); return _some_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void from_impl() {
        {
            auto&& _m0_tmp = OnceCell<std::string_view>::from("value").get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<const std::string_view&>([&]() -> const auto& { static const auto _some_ref_tmp = std::string_view("value"); return _some_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = OnceCell<std::string_view>::from("foo").get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<const std::string_view&>([&]() -> const auto& { static const auto _some_ref_tmp = std::string_view("bar"); return _some_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val)) {
                    const auto kind = rusty::panicking::AssertKind::Ne;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void partialeq_impl() {
        if (!(unsync::OnceCell<std::string_view>::from("value") == unsync::OnceCell<std::string_view>::from("value"))) {
            rusty::panicking::panic("assertion failed: OnceCell::from(\"value\") == OnceCell::from(\"value\")");
        }
        if (!(unsync::OnceCell<std::string_view>::from("foo") != unsync::OnceCell<std::string_view>::from("bar"))) {
            rusty::panicking::panic("assertion failed: OnceCell::from(\"foo\") != OnceCell::from(\"bar\")");
        }
        if (!(OnceCell<rusty::String>::new_() == unsync::OnceCell<rusty::String>::new_())) {
            rusty::panicking::panic("assertion failed: OnceCell::<String>::new() == OnceCell::new()");
        }
        if (!(unsync::OnceCell<rusty::String>::new_() != unsync::OnceCell<rusty::String>::from(rusty::String::from("value")))) {
            rusty::panicking::panic("assertion failed: OnceCell::<String>::new() != OnceCell::from(\"value\".to_owned())");
        }
    }

    void into_inner() {
        unsync::OnceCell<rusty::String> cell = unsync::OnceCell<rusty::String>::new_();
        {
            auto&& _m0_tmp = cell.into_inner();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::None;
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        auto cell_shadow1 = unsync::OnceCell<rusty::String>::new_();
        cell_shadow1.set(rusty::String::from(rusty::to_string("hello"))).unwrap();
        {
            auto&& _m0_tmp = cell_shadow1.into_inner();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<rusty::String>(rusty::String::from(rusty::to_string("hello")));
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void debug_impl() {
        const auto cell = unsync::OnceCell<rusty::Vec<rusty::String>>::new_();
        {
            auto&& _m0_tmp = rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0}", rusty::to_debug_string_pretty(cell))));
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = std::string_view("OnceCell(Uninit)");
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        cell.set(rusty::boxed::into_vec(rusty::boxed::box_new(std::array{rusty::String::from("hello"), rusty::String::from("world")}))).unwrap();
        {
            auto&& _m0_tmp = rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0}", rusty::to_debug_string_pretty(cell))));
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = std::string_view("OnceCell(\n    [\n        \"hello\",\n        \"world\",\n    ],\n)");
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void reentrant_init() {
        const unsync::OnceCell<rusty::Box<int32_t>> x = unsync::OnceCell<rusty::Box<int32_t>>::new_();
        const rusty::Cell<rusty::Option<const int32_t&>> dangling_ref = rusty::Cell<rusty::Option<const int32_t&>>::new_(rusty::Option<const int32_t&>(rusty::None));
        x.get_or_init([&]() -> rusty::Box<int32_t> {
auto& r = x.get_or_init([&]() -> rusty::Box<int32_t> { return rusty::Box<int32_t>::new_(static_cast<int32_t>(92)); });
dangling_ref.set(rusty::Option<const int32_t&>(r));
return rusty::Box<int32_t>::new_(static_cast<int32_t>(62));
});
        {
            rusty::io::_eprint(std::format("use after free: {0}\n", rusty::to_debug_string(dangling_ref.get()->unwrap())));
        }
    }

    void aliasing_in_get() {
        const auto x = unsync::OnceCell<int32_t>::new_();
        x.set(static_cast<int32_t>(42)).unwrap();
        const auto at_x = x.get().unwrap();
        static_cast<void>(x.set(static_cast<int32_t>(27)));
        {
            rusty::io::_print(std::format("{0}\n", rusty::to_string(at_x)));
        }
    }

    void arrrrrrrrrrrrrrrrrrrrrr() {
        const auto cell = unsync::OnceCell<rusty::String>::new_();
        {
            auto s = rusty::String::new_();
            cell.set(std::move(s)).unwrap();
        }
    }

}

namespace sync_once_cell {

    void once_cell();
    void once_cell_with_value();
    void once_cell_get_mut();
    void once_cell_get_unchecked();
    void once_cell_drop();
    void once_cell_drop_empty();
    void clone();
    void get_or_try_init();
    void wait();
    void wait_panic();
    void get_or_init_stress();
    void from_impl();
    void partialeq_impl();
    void into_inner();
    void debug_impl();
    void reentrant_init();
    void eval_once_macro();
    void once_cell_does_not_leak_partially_constructed_boxes();
    void get_does_not_block();
    void arrrrrrrrrrrrrrrrrrrrrr();
    void once_cell_is_sync_send();

    using ::rusty::sync::atomic::AtomicUsize;
    constexpr auto SeqCst = rusty::sync::atomic::Ordering::SeqCst;
    using ::rusty::thread::scope;

    using ::rusty::Barrier;

    using ::sync_mod::Lazy;
    using ::sync_mod::OnceCell;


    // Rust-only libtest metadata const skipped: once_cell (marker: sync_once_cell::once_cell, should_panic: no)


    // Rust-only libtest metadata const skipped: once_cell_with_value (marker: sync_once_cell::once_cell_with_value, should_panic: no)


    // Rust-only libtest metadata const skipped: once_cell_get_mut (marker: sync_once_cell::once_cell_get_mut, should_panic: no)


    // Rust-only libtest metadata const skipped: once_cell_get_unchecked (marker: sync_once_cell::once_cell_get_unchecked, should_panic: no)


    // Rust-only libtest metadata const skipped: once_cell_drop (marker: sync_once_cell::once_cell_drop, should_panic: no)


    // Rust-only libtest metadata const skipped: once_cell_drop_empty (marker: sync_once_cell::once_cell_drop_empty, should_panic: no)


    // Rust-only libtest metadata const skipped: clone (marker: sync_once_cell::clone, should_panic: no)


    // Rust-only libtest metadata const skipped: get_or_try_init (marker: sync_once_cell::get_or_try_init, should_panic: no)


    // Rust-only libtest metadata const skipped: wait (marker: sync_once_cell::wait, should_panic: no)


    // Rust-only libtest metadata const skipped: wait_panic (marker: sync_once_cell::wait_panic, should_panic: no)


    // Rust-only libtest metadata const skipped: get_or_init_stress (marker: sync_once_cell::get_or_init_stress, should_panic: no)


    // Rust-only libtest metadata const skipped: from_impl (marker: sync_once_cell::from_impl, should_panic: no)


    // Rust-only libtest metadata const skipped: partialeq_impl (marker: sync_once_cell::partialeq_impl, should_panic: no)


    // Rust-only libtest metadata const skipped: into_inner (marker: sync_once_cell::into_inner, should_panic: no)


    // Rust-only libtest metadata const skipped: debug_impl (marker: sync_once_cell::debug_impl, should_panic: no)


    // Rust-only libtest metadata const skipped: reentrant_init (marker: sync_once_cell::reentrant_init, should_panic: no)


    // Rust-only libtest metadata const skipped: eval_once_macro (marker: sync_once_cell::eval_once_macro, should_panic: no)


    // Rust-only libtest metadata const skipped: once_cell_does_not_leak_partially_constructed_boxes (marker: sync_once_cell::once_cell_does_not_leak_partially_constructed_boxes, should_panic: no)


    // Rust-only libtest metadata const skipped: get_does_not_block (marker: sync_once_cell::get_does_not_block, should_panic: no)


    // Rust-only libtest metadata const skipped: arrrrrrrrrrrrrrrrrrrrrr (marker: sync_once_cell::arrrrrrrrrrrrrrrrrrrrrr, should_panic: no)


    // Rust-only libtest metadata const skipped: once_cell_is_sync_send (marker: sync_once_cell::once_cell_is_sync_send, should_panic: no)

    void once_cell() {
        const auto c = sync::OnceCell<int32_t>::new_();
        if (!c.get().is_none()) {
            rusty::panicking::panic("assertion failed: c.get().is_none()");
        }
        scope([&](auto&& s) {
s.spawn([&]() {
c.get_or_init([&]() -> int32_t { return static_cast<int32_t>(92); });
{
    auto&& _m0_tmp = c.get();
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = rusty::Option<const int32_t&>([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<int32_t>(92); return _some_ref_tmp; }());
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
});
});
        c.get_or_init([&]() -> int32_t {
[&]() -> int32_t { rusty::panicking::panic_fmt(std::string("Kabom!")); }();
});
        {
            auto&& _m0_tmp = c.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<const int32_t&>([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<int32_t>(92); return _some_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_cell_with_value() {
        static sync::OnceCell<int32_t> CELL = sync::OnceCell<int32_t>::with_value(12);
        {
            auto&& _m0_tmp = CELL.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<const int32_t&>([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<int32_t>(12); return _some_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_cell_get_mut() {
        auto c = sync::OnceCell<int32_t>::new_();
        if (!c.get_mut().is_none()) {
            rusty::panicking::panic("assertion failed: c.get_mut().is_none()");
        }
        c.set(static_cast<int32_t>(90)).unwrap();
        [&]() { static_cast<void>(c.get_mut().unwrap() += 2); return std::make_tuple(); }();
        {
            auto&& _m0_tmp = c.get_mut();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<int32_t&>([&]() -> auto& { auto _some_mut_ref_value = (static_cast<int32_t>(92)); thread_local std::optional<int32_t> _some_mut_ref_tmp; _some_mut_ref_tmp.reset(); _some_mut_ref_tmp.emplace(std::move(_some_mut_ref_value)); return *_some_mut_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_cell_get_unchecked() {
        const auto c = sync::OnceCell<int32_t>::new_();
        c.set(static_cast<int32_t>(92)).unwrap();
        // @unsafe
        {
            {
                auto&& _m0_tmp = c.get_unchecked();
                auto _m0 = &_m0_tmp;
                auto&& _m1_tmp = static_cast<int32_t>(92);
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                    auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                    if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                        const auto kind = rusty::panicking::AssertKind::Eq;
                        rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                    }
                    _m_matched = true;
                }
            }
        }
    }

    void once_cell_drop() {
        static rusty::sync::atomic::AtomicUsize DROP_CNT = AtomicUsize::new_(0);
        struct Dropper {
            Dropper() = default;
            Dropper(const Dropper&) = default;
            Dropper(Dropper&& other) noexcept {
                if (rusty::mem::consume_forgotten_address(&other)) {
                    this->rusty_mark_forgotten();
                    other.rusty_mark_forgotten();
                } else {
                    other.rusty_mark_forgotten();
                }
            }
            Dropper& operator=(const Dropper&) = default;
            Dropper& operator=(Dropper&& other) noexcept {
                if (this == &other) {
                    return *this;
                }
                this->~Dropper();
                new (this) Dropper(std::move(other));
                return *this;
            }
            void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


            ~Dropper() noexcept(false) {
                if (rusty::mem::consume_forgotten_address(this)) { return; }
                DROP_CNT.fetch_add(1, rusty::sync::atomic::Ordering::SeqCst);
            }
        };
        // Rust-only nested impl block skipped in local scope
        auto x = sync::OnceCell<Dropper>::new_();
        scope([&](auto&& s) {
s.spawn([&]() {
x.get_or_init([&]() -> Dropper { return Dropper{}; });
{
    auto&& _m0_tmp = DROP_CNT.load(rusty::sync::atomic::Ordering::SeqCst);
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = static_cast<int32_t>(0);
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
rusty::mem::drop(std::move(x));
});
});
        {
            auto&& _m0_tmp = DROP_CNT.load(rusty::sync::atomic::Ordering::SeqCst);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_cell_drop_empty() {
        auto x = sync::OnceCell<rusty::String>::new_();
        rusty::mem::drop(std::move(x));
    }

    void clone() {
        const auto s = sync::OnceCell<rusty::String>::new_();
        const auto c = rusty::clone(s);
        if (!c.get().is_none()) {
            rusty::panicking::panic("assertion failed: c.get().is_none()");
        }
        s.set(rusty::String::from(rusty::to_string("hello"))).unwrap();
        const auto c_shadow1 = rusty::clone(s);
        {
            auto&& _m0_tmp = c_shadow1.get().map([](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.as_str(std::forward<decltype(_args)>(_args)...); });
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Some("hello");
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void get_or_try_init() {
        const sync::OnceCell<rusty::String> cell = sync::OnceCell<rusty::String>::new_();
        if (!cell.get().is_none()) {
            rusty::panicking::panic("assertion failed: cell.get().is_none()");
        }
        const auto res = rusty::panic::catch_unwind([&]() {
return cell.get_or_try_init([&]() -> rusty::Result<rusty::String, std::tuple<>> {
return [&]() -> rusty::Result<rusty::String, std::tuple<>> { rusty::panicking::panic("explicit panic"); }();
});
});
        if (!res.is_err()) {
            rusty::panicking::panic("assertion failed: res.is_err()");
        }
        if (!cell.get().is_none()) {
            rusty::panicking::panic("assertion failed: cell.get().is_none()");
        }
        {
            auto&& _m0_tmp = cell.get_or_try_init([&]() -> rusty::Result<rusty::String, std::tuple<>> { return rusty::Result<rusty::String, std::tuple<>>::Err(std::make_tuple()); });
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = [&]() { using _ResultCtorCtx = std::remove_cvref_t<decltype((cell.get_or_try_init([&]() -> rusty::Result<rusty::String, std::tuple<>> { return rusty::Result<rusty::String, std::tuple<>>::Err(std::make_tuple()); })))>; return _ResultCtorCtx::Err(std::make_tuple()); }();
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = cell.get_or_try_init([&]() -> rusty::Result<rusty::String, std::tuple<>> { return rusty::Result<rusty::String, std::tuple<>>::Ok(rusty::String::from(rusty::to_string("hello"))); });
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = [&]() { using _ResultCtorCtx = std::remove_cvref_t<decltype((cell.get_or_try_init([&]() -> rusty::Result<rusty::String, std::tuple<>> { return rusty::Result<rusty::String, std::tuple<>>::Ok(rusty::String::from(rusty::to_string("hello"))); })))>; using _ResultCtorArg = rusty::result_ok_t<_ResultCtorCtx>; if constexpr (std::is_reference_v<_ResultCtorArg>) { using _ResultCtorStorage = std::remove_cvref_t<_ResultCtorArg>; auto _result_ctor_value = (rusty::String::from(rusty::to_string("hello"))); thread_local std::optional<_ResultCtorStorage> _result_ctor_tmp; _result_ctor_tmp.reset(); _result_ctor_tmp.emplace(static_cast<_ResultCtorStorage>(std::move(_result_ctor_value))); return _ResultCtorCtx::Ok(static_cast<_ResultCtorArg>(*_result_ctor_tmp)); } else { return _ResultCtorCtx::Ok(rusty::String::from(rusty::to_string("hello"))); } }();
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = cell.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<const rusty::String&>([&]() -> const auto& { static const auto _some_ref_tmp = rusty::String::from(rusty::to_string("hello")); return _some_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void wait() {
        const sync::OnceCell<rusty::String> cell = sync::OnceCell<rusty::String>::new_();
        scope([&](auto&& s) {
s.spawn([&]() { return cell.set(rusty::String::from(rusty::to_string("hello"))); });
const auto& greeting = cell.wait();
{
    auto _m0 = &greeting;
    auto&& _m1_tmp = "hello";
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
});
    }

    void wait_panic() {
        const sync::OnceCell<rusty::String> cell = sync::OnceCell<rusty::String>::new_();
        scope([&](auto&& s) {
const auto h1 = s.spawn([&]() {
cell.get_or_try_init([&]() -> rusty::Result<rusty::String, std::tuple<>> {
return [&]() -> rusty::Result<rusty::String, std::tuple<>> { rusty::panicking::panic("explicit panic"); }();
}).unwrap();
});
const auto h2 = s.spawn([&]() {
if (!h1.join().is_err()) {
    rusty::panicking::panic("assertion failed: h1.join().is_err()");
}
cell.get_or_try_init([&]() -> rusty::Result<rusty::String, std::tuple<>> {
return rusty::Result<rusty::String, std::tuple<>>::Ok(rusty::String::from(rusty::to_string("hello")));
}).unwrap();
});
const auto& greeting = cell.wait();
{
    auto _m0 = &greeting;
    auto&& _m1_tmp = "hello";
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
if (!h2.join().is_ok()) {
    return rusty::panicking::panic("assertion failed: h2.join().is_ok()");
}
});
    }

    void get_or_init_stress() {
        const auto n_threads = (false ? static_cast<int32_t>(30) : static_cast<int32_t>(1000));
        const auto n_cells = (false ? static_cast<int32_t>(30) : static_cast<int32_t>(1000));
        const auto cells = rusty::collect_range(rusty::repeat_with([&]() { return std::make_tuple(Barrier::new_(std::move(n_threads)), OnceCell<size_t>::new_()); }).take(std::move(n_cells)));
        scope([&](auto&& s) {
for (auto&& t : rusty::for_in(rusty::range(0, n_threads))) {
    const auto& cells_shadow1 = cells;
    s.spawn([=, &cells_shadow1, t = std::move(t)]() mutable {
for (auto&& _for_item : rusty::for_in(rusty::enumerate(rusty::iter(cells_shadow1)))) {
    auto&& i = std::get<0>(rusty::detail::deref_if_pointer(_for_item));
    auto&& b = std::get<0>(rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_for_item))));
    auto&& s = std::get<1>(rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_for_item))));
    b.wait();
    const auto j = ((t % 2) == static_cast<int32_t>(0) ? s.wait() : s.get_or_init([&]() { return i; }));
    {
        auto _m0 = &j;
        auto&& _m1_tmp = i;
        auto _m1 = &_m1_tmp;
        auto _m_tuple = std::make_tuple(_m0, _m1);
        bool _m_matched = false;
        if (!_m_matched) {
            auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
            auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
            if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                const auto kind = rusty::panicking::AssertKind::Eq;
                rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
            }
            _m_matched = true;
        }
    }
}
});
}
});
    }

    void from_impl() {
        {
            auto&& _m0_tmp = OnceCell<std::string_view>::from("value").get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<const std::string_view&>([&]() -> const auto& { static const auto _some_ref_tmp = std::string_view("value"); return _some_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = OnceCell<std::string_view>::from("foo").get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<const std::string_view&>([&]() -> const auto& { static const auto _some_ref_tmp = std::string_view("bar"); return _some_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val)) {
                    const auto kind = rusty::panicking::AssertKind::Ne;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void partialeq_impl() {
        if (!(sync::OnceCell<std::string_view>::from("value") == sync::OnceCell<std::string_view>::from("value"))) {
            rusty::panicking::panic("assertion failed: OnceCell::from(\"value\") == OnceCell::from(\"value\")");
        }
        if (!(sync::OnceCell<std::string_view>::from("foo") != sync::OnceCell<std::string_view>::from("bar"))) {
            rusty::panicking::panic("assertion failed: OnceCell::from(\"foo\") != OnceCell::from(\"bar\")");
        }
        if (!(OnceCell<rusty::String>::new_() == sync::OnceCell<rusty::String>::new_())) {
            rusty::panicking::panic("assertion failed: OnceCell::<String>::new() == OnceCell::new()");
        }
        if (!(sync::OnceCell<rusty::String>::new_() != sync::OnceCell<rusty::String>::from(rusty::String::from("value")))) {
            rusty::panicking::panic("assertion failed: OnceCell::<String>::new() != OnceCell::from(\"value\".to_owned())");
        }
    }

    void into_inner() {
        sync::OnceCell<rusty::String> cell = sync::OnceCell<rusty::String>::new_();
        {
            auto&& _m0_tmp = cell.into_inner();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::None;
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        auto cell_shadow1 = sync::OnceCell<rusty::String>::new_();
        cell_shadow1.set(rusty::String::from(rusty::to_string("hello"))).unwrap();
        {
            auto&& _m0_tmp = cell_shadow1.into_inner();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<rusty::String>(rusty::String::from(rusty::to_string("hello")));
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void debug_impl() {
        const auto cell = sync::OnceCell<rusty::Vec<rusty::String>>::new_();
        {
            auto&& _m0_tmp = rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0}", rusty::to_debug_string_pretty(cell))));
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = std::string_view("OnceCell(Uninit)");
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        cell.set(rusty::boxed::into_vec(rusty::boxed::box_new(std::array{rusty::String::from("hello"), rusty::String::from("world")}))).unwrap();
        {
            auto&& _m0_tmp = rusty::alloc::__export::must_use(rusty::alloc::fmt::format(std::format("{0}", rusty::to_debug_string_pretty(cell))));
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = std::string_view("OnceCell(\n    [\n        \"hello\",\n        \"world\",\n    ],\n)");
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void reentrant_init() {
        struct Guard {
            rusty::process::Child child;
            Guard(rusty::process::Child child_init) : child(std::move(child_init)) {}
            Guard(const Guard&) = default;
            Guard(Guard&& other) noexcept : child(std::move(other.child)) {
                if (rusty::mem::consume_forgotten_address(&other)) {
                    this->rusty_mark_forgotten();
                    other.rusty_mark_forgotten();
                } else {
                    other.rusty_mark_forgotten();
                }
            }
            Guard& operator=(const Guard&) = default;
            Guard& operator=(Guard&& other) noexcept {
                if (this == &other) {
                    return *this;
                }
                this->~Guard();
                new (this) Guard(std::move(other));
                return *this;
            }
            void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


            ~Guard() noexcept(false) {
                if (rusty::mem::consume_forgotten_address(this)) { return; }
                static_cast<void>(this->child.kill());
            }
        };
        const auto examples_dir = [&]() { auto exe = rusty::env::current_exe().unwrap();
exe.pop();
exe.pop();
exe.push("examples");
return exe; }();
        const auto bin = examples_dir.join("reentrant_init_deadlocks").with_extension(rusty::env::consts::EXE_EXTENSION);
        auto guard = Guard(rusty::process::Command::new_(std::move(bin)).spawn().unwrap());
        rusty::thread::sleep(rusty::time::Duration::from_secs(2));
        const auto status = guard.child.try_wait().unwrap();
        if (!status.is_none()) {
            rusty::panicking::panic("assertion failed: status.is_none()");
        }
        // Rust-only nested impl block skipped in local scope
    }

    void eval_once_macro() {
        const rusty::Vec<int32_t>& fib = [&]() -> const rusty::Vec<int32_t>& { static sync::OnceCell<rusty::Vec<int32_t>> ONCE_CELL = sync::OnceCell<rusty::Vec<int32_t>>::new_();
const rusty::SafeFn<rusty::Vec<int32_t>()> init = +[]() -> rusty::Vec<int32_t> {
    auto res = rusty::boxed::into_vec(rusty::boxed::box_new(std::array{static_cast<int32_t>(1), static_cast<int32_t>(1)}));
    for (auto&& i : rusty::for_in(rusty::range(0, 10))) {
        auto next = res[i] + res[i + 1];
        res.push(std::move(next));
    }
    return res;
};
return ONCE_CELL.get_or_init(std::move(init)); }();
        {
            auto _m0 = rusty::as_ref_ptr(fib[5]);
            auto&& _m1_tmp = static_cast<int32_t>(8);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_cell_does_not_leak_partially_constructed_boxes() {
        const std::string_view MSG = std::string_view("Hello, World");
        const auto n_tries = (false ? static_cast<int32_t>(10) : static_cast<int32_t>(100));
        const auto n_readers = 10;
        const auto n_writers = 3;
        for (auto&& _ : rusty::for_in(rusty::range(0, n_tries))) {
            const sync::OnceCell<rusty::String> cell = sync::OnceCell<rusty::String>::new_();
            scope([&](auto&& scope) {
for (auto&& _ : rusty::for_in(rusty::range(0, n_readers))) {
    scope.spawn([&]() {
while (true) {
    if (auto&& _iflet_scrutinee = cell.get(); _iflet_scrutinee.is_some()) {
        decltype(auto) msg = _iflet_scrutinee.unwrap();
        {
            auto&& _m0_tmp = rusty::to_string_view(msg);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = std::string_view(MSG);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        break;
    }
}
});
}
for (auto&& _ : rusty::for_in(rusty::range(0, n_writers))) {
    static_cast<void>(scope.spawn([&]() { return cell.set(rusty::String::from(rusty::clone(MSG))); }));
}
});
        }
    }

    void get_does_not_block() {
        const auto cell = sync::OnceCell<rusty::String>::new_();
        const auto barrier = Barrier::new_(2);
        scope([&](auto&& scope) {
scope.spawn([&]() {
cell.get_or_init([&]() -> rusty::String {
barrier.wait();
barrier.wait();
return rusty::String::from(rusty::to_string("hello"));
});
});
barrier.wait();
{
    auto&& _m0_tmp = cell.get();
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = rusty::None;
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
barrier.wait();
});
        {
            auto&& _m0_tmp = cell.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<const rusty::String&>([&]() -> const auto& { static const auto _some_ref_tmp = rusty::String::from(rusty::to_string("hello")); return _some_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void arrrrrrrrrrrrrrrrrrrrrr() {
        const auto cell = sync::OnceCell<rusty::String>::new_();
        {
            auto s = rusty::String::new_();
            cell.set(std::move(s)).unwrap();
        }
    }

    void once_cell_is_sync_send() {
        const rusty::SafeFn<void()> assert_traits = +[]() {
        };
        assert_traits();
        assert_traits();
    }

}

namespace unsync_lazy {

    void lazy_new();
    void lazy_deref_mut();
    void lazy_force_mut();
    void lazy_get_mut();
    void lazy_default();
    void lazy_into_value();
    void lazy_poisoning();
    void arrrrrrrrrrrrrrrrrrrrrr();

    using ::rusty::Cell;
    using ::rusty::sync::atomic::AtomicUsize;
    constexpr auto SeqCst = rusty::sync::atomic::Ordering::SeqCst;

    using ::unsync::Lazy;


    // Rust-only libtest metadata const skipped: lazy_new (marker: unsync_lazy::lazy_new, should_panic: no)


    // Rust-only libtest metadata const skipped: lazy_deref_mut (marker: unsync_lazy::lazy_deref_mut, should_panic: no)


    // Rust-only libtest metadata const skipped: lazy_force_mut (marker: unsync_lazy::lazy_force_mut, should_panic: no)


    // Rust-only libtest metadata const skipped: lazy_get_mut (marker: unsync_lazy::lazy_get_mut, should_panic: no)


    // Rust-only libtest metadata const skipped: lazy_default (marker: unsync_lazy::lazy_default, should_panic: no)


    // Rust-only libtest metadata const skipped: lazy_into_value (marker: unsync_lazy::lazy_into_value, should_panic: no)


    // Rust-only libtest metadata const skipped: lazy_poisoning (marker: unsync_lazy::lazy_poisoning, should_panic: no)


    // Rust-only libtest metadata const skipped: arrrrrrrrrrrrrrrrrrrrrr (marker: unsync_lazy::arrrrrrrrrrrrrrrrrrrrrr, should_panic: no)

    void lazy_new() {
        const auto called = rusty::Cell<int32_t>::new_(static_cast<int32_t>(0));
        const auto x = unsync::Lazy<int32_t>::new_([&]() -> int32_t {
called.set(called.get() + 1);
return static_cast<int32_t>(92);
});
        {
            auto&& _m0_tmp = called.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        const auto y = *(x) - 30;
        {
            auto _m0 = &y;
            auto&& _m1_tmp = static_cast<int32_t>(62);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = called.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        const auto y_shadow1 = *(x) - 30;
        {
            auto _m0 = &y_shadow1;
            auto&& _m1_tmp = static_cast<int32_t>(62);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = called.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void lazy_deref_mut() {
        const auto called = rusty::Cell<int32_t>::new_(static_cast<int32_t>(0));
        auto x = unsync::Lazy<int32_t>::new_([&]() -> int32_t {
called.set(called.get() + 1);
return static_cast<int32_t>(92);
});
        {
            auto&& _m0_tmp = called.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        const auto y = *(x) - 30;
        {
            auto _m0 = &y;
            auto&& _m1_tmp = static_cast<int32_t>(62);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = called.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        [&]() { static_cast<void>(*(x) /= 2); return std::make_tuple(); }();
        {
            auto&& _m0_tmp = *(x);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(46);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = called.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void lazy_force_mut() {
        const auto called = rusty::Cell<int32_t>::new_(static_cast<int32_t>(0));
        auto x = unsync::Lazy<int32_t>::new_([&]() -> int32_t {
called.set(called.get() + 1);
return static_cast<int32_t>(92);
});
        {
            auto&& _m0_tmp = called.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        auto& v = Lazy<int32_t>::force_mut(x);
        {
            auto&& _m0_tmp = called.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        [&]() { static_cast<void>(v /= 2); return std::make_tuple(); }();
        {
            auto&& _m0_tmp = *(x);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(46);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = called.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void lazy_get_mut() {
        const auto called = rusty::Cell<int32_t>::new_(static_cast<int32_t>(0));
        auto x = unsync::Lazy<uint32_t>::new_([&]() -> int32_t {
called.set(called.get() + 1);
return static_cast<int32_t>(92);
});
        {
            auto&& _m0_tmp = called.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = *(x);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(92);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        uint32_t& mut_ref = Lazy<uint32_t>::get_mut(x).unwrap();
        {
            auto&& _m0_tmp = called.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        [&]() { static_cast<void>(mut_ref /= 2); return std::make_tuple(); }();
        {
            auto&& _m0_tmp = *(x);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(46);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = called.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void lazy_default() {
        static rusty::sync::atomic::AtomicUsize CALLED = AtomicUsize::new_(0);
        struct Foo {
            uint8_t _0;

            static Foo default_() {
                CALLED.fetch_add(1, rusty::sync::atomic::Ordering::SeqCst);
                return Foo(42);
            }
        };
        // Rust-only nested impl block skipped in local scope
        const unsync::Lazy<rusty::Mutex<Foo>> lazy = rusty::default_value<unsync::Lazy<rusty::Mutex<Foo>>>();
        {
            auto&& _m0_tmp = CALLED.load(rusty::sync::atomic::Ordering::SeqCst);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = (*(*lazy).lock().unwrap())._0;
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(42);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = CALLED.load(rusty::sync::atomic::Ordering::SeqCst);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        (*(*lazy).lock().unwrap())._0 = 21;
        {
            auto&& _m0_tmp = (*(*lazy).lock().unwrap())._0;
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(21);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = CALLED.load(rusty::sync::atomic::Ordering::SeqCst);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void lazy_into_value() {
        auto l = unsync::Lazy<int32_t>::new_([&]() -> int32_t { [&]() -> int32_t { rusty::panicking::panic("explicit panic"); }(); });
        if (![&]() -> bool { auto&& _m = Lazy<int32_t>::into_value(std::move(l)); if (_m.is_err()) { return true; } if (true) { return false; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }()) {
            rusty::panicking::panic("assertion failed: matches!(Lazy::into_value(l), Err(_))");
        }
        auto l_shadow1 = unsync::Lazy<int32_t>::new_([&]() -> int32_t {
return static_cast<int32_t>(92);
});
        Lazy<int32_t>::force(l_shadow1);
        if (![&]() -> bool { auto&& _m = Lazy<int32_t>::into_value(std::move(l_shadow1)); if (_m.is_ok()) { auto&& _mv0 = std::as_const(_m).unwrap(); if (_mv0 == 92) { return true; } } if (true) { return false; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }()) {
            rusty::panicking::panic("assertion failed: matches!(Lazy::into_value(l), Ok(92))");
        }
    }

    void lazy_poisoning() {
        const unsync::Lazy<rusty::String> x = unsync::Lazy<rusty::String>::new_([&]() -> rusty::String {
[&]() -> rusty::String { rusty::panicking::panic_fmt(std::string("kaboom")); }();
});
        for (auto&& _ : rusty::for_in(rusty::range(0, 2))) {
            const auto res = rusty::panic::catch_unwind([&]() { return rusty::len((*x)); });
            if (!res.is_err()) {
                rusty::panicking::panic("assertion failed: res.is_err()");
            }
        }
    }

    void arrrrrrrrrrrrrrrrrrrrrr() {
        std::optional<unsync::Lazy<const rusty::String&>> lazy;
        {
            const auto s = rusty::String::new_();
            lazy.emplace(unsync::Lazy<const rusty::String&>::new_([&]() -> const rusty::String& { return s; }));
            [&]() { static_cast<void>(lazy.value()); return std::make_tuple(); }();
        }
    }

}

namespace sync_lazy {

    void lazy_new();
    void lazy_deref_mut();
    void lazy_force_mut();
    void lazy_get_mut();
    void lazy_default();
    void static_lazy();
    void static_lazy_via_fn();
    void lazy_into_value();
    void lazy_poisoning();
    void arrrrrrrrrrrrrrrrrrrrrr();
    void lazy_is_sync_send();

    using ::rusty::Cell;
    using ::rusty::sync::atomic::AtomicUsize;
    constexpr auto SeqCst = rusty::sync::atomic::Ordering::SeqCst;
    using ::rusty::thread::scope;

    using ::sync_mod::Lazy;
    using ::sync_mod::OnceCell;


    // Rust-only libtest metadata const skipped: lazy_new (marker: sync_lazy::lazy_new, should_panic: no)


    // Rust-only libtest metadata const skipped: lazy_deref_mut (marker: sync_lazy::lazy_deref_mut, should_panic: no)


    // Rust-only libtest metadata const skipped: lazy_force_mut (marker: sync_lazy::lazy_force_mut, should_panic: no)


    // Rust-only libtest metadata const skipped: lazy_get_mut (marker: sync_lazy::lazy_get_mut, should_panic: no)


    // Rust-only libtest metadata const skipped: lazy_default (marker: sync_lazy::lazy_default, should_panic: no)


    // Rust-only libtest metadata const skipped: static_lazy (marker: sync_lazy::static_lazy, should_panic: no)


    // Rust-only libtest metadata const skipped: static_lazy_via_fn (marker: sync_lazy::static_lazy_via_fn, should_panic: no)


    // Rust-only libtest metadata const skipped: lazy_into_value (marker: sync_lazy::lazy_into_value, should_panic: no)


    // Rust-only libtest metadata const skipped: lazy_poisoning (marker: sync_lazy::lazy_poisoning, should_panic: no)


    // Rust-only libtest metadata const skipped: arrrrrrrrrrrrrrrrrrrrrr (marker: sync_lazy::arrrrrrrrrrrrrrrrrrrrrr, should_panic: no)


    // Rust-only libtest metadata const skipped: lazy_is_sync_send (marker: sync_lazy::lazy_is_sync_send, should_panic: no)

    void lazy_new() {
        const auto called = AtomicUsize::new_(0);
        const auto x = sync::Lazy<int32_t>::new_([&]() -> int32_t {
called.fetch_add(1, rusty::sync::atomic::Ordering::SeqCst);
return static_cast<int32_t>(92);
});
        {
            auto&& _m0_tmp = called.load(rusty::sync::atomic::Ordering::SeqCst);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        scope([&](auto&& s) {
s.spawn([&]() {
const auto y = *(x) - 30;
{
    auto _m0 = &y;
    auto&& _m1_tmp = static_cast<int32_t>(62);
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
{
    auto&& _m0_tmp = called.load(rusty::sync::atomic::Ordering::SeqCst);
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = static_cast<int32_t>(1);
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
});
});
        const auto y = *(x) - 30;
        {
            auto _m0 = &y;
            auto&& _m1_tmp = static_cast<int32_t>(62);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = called.load(rusty::sync::atomic::Ordering::SeqCst);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void lazy_deref_mut() {
        const auto called = AtomicUsize::new_(0);
        auto x = sync::Lazy<int32_t>::new_([&]() -> int32_t {
called.fetch_add(1, rusty::sync::atomic::Ordering::SeqCst);
return static_cast<int32_t>(92);
});
        {
            auto&& _m0_tmp = called.load(rusty::sync::atomic::Ordering::SeqCst);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        const auto y = *(x) - 30;
        {
            auto _m0 = &y;
            auto&& _m1_tmp = static_cast<int32_t>(62);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = called.load(rusty::sync::atomic::Ordering::SeqCst);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        [&]() { static_cast<void>(*(x) /= 2); return std::make_tuple(); }();
        {
            auto&& _m0_tmp = *(x);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(46);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = called.load(rusty::sync::atomic::Ordering::SeqCst);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void lazy_force_mut() {
        const auto called = rusty::Cell<int32_t>::new_(static_cast<int32_t>(0));
        auto x = sync::Lazy<int32_t>::new_([&]() -> int32_t {
called.set(called.get() + 1);
return static_cast<int32_t>(92);
});
        {
            auto&& _m0_tmp = called.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        auto& v = Lazy<int32_t>::force_mut(x);
        {
            auto&& _m0_tmp = called.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        [&]() { static_cast<void>(v /= 2); return std::make_tuple(); }();
        {
            auto&& _m0_tmp = *(x);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(46);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = called.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void lazy_get_mut() {
        const auto called = rusty::Cell<int32_t>::new_(static_cast<int32_t>(0));
        auto x = sync::Lazy<uint32_t>::new_([&]() -> int32_t {
called.set(called.get() + 1);
return static_cast<int32_t>(92);
});
        {
            auto&& _m0_tmp = called.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = *(x);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(92);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        uint32_t& mut_ref = Lazy<uint32_t>::get_mut(x).unwrap();
        {
            auto&& _m0_tmp = called.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        [&]() { static_cast<void>(mut_ref /= 2); return std::make_tuple(); }();
        {
            auto&& _m0_tmp = *(x);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(46);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = called.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void lazy_default() {
        static rusty::sync::atomic::AtomicUsize CALLED = AtomicUsize::new_(0);
        struct Foo {
            uint8_t _0;

            static Foo default_() {
                CALLED.fetch_add(1, rusty::sync::atomic::Ordering::SeqCst);
                return Foo(42);
            }
        };
        // Rust-only nested impl block skipped in local scope
        const sync::Lazy<rusty::Mutex<Foo>> lazy = rusty::default_value<sync::Lazy<rusty::Mutex<Foo>>>();
        {
            auto&& _m0_tmp = CALLED.load(rusty::sync::atomic::Ordering::SeqCst);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = (*(*lazy).lock().unwrap())._0;
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(42);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = CALLED.load(rusty::sync::atomic::Ordering::SeqCst);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        (*(*lazy).lock().unwrap())._0 = 21;
        {
            auto&& _m0_tmp = (*(*lazy).lock().unwrap())._0;
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(21);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = CALLED.load(rusty::sync::atomic::Ordering::SeqCst);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void static_lazy() {
        static sync::Lazy<rusty::Vec<int32_t>> XS = sync::Lazy<rusty::Vec<int32_t>>::new_([&]() -> rusty::Vec<int32_t> {
auto xs = rusty::Vec<int32_t>::new_();
xs.push(static_cast<int32_t>(1));
xs.push(static_cast<int32_t>(2));
xs.push(static_cast<int32_t>(3));
return xs;
});
        scope([&](auto&& s) {
s.spawn([&]() {
{
    auto&& _m0_tmp = *(XS);
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = rusty::boxed::into_vec(rusty::boxed::box_new(std::array{1, 2, 3}));
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
});
});
        {
            auto&& _m0_tmp = *(XS);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::boxed::into_vec(rusty::boxed::box_new(std::array{1, 2, 3}));
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void static_lazy_via_fn() {
        const rusty::SafeFn<const rusty::Vec<int32_t>&()> xs = +[]() -> const rusty::Vec<int32_t>& {
            static sync::OnceCell<rusty::Vec<int32_t>> XS = sync::OnceCell<rusty::Vec<int32_t>>::new_();
            return XS.get_or_init([&]() -> rusty::Vec<int32_t> {
auto xs_shadow1 = rusty::Vec<int32_t>::new_();
xs_shadow1.push(static_cast<int32_t>(1));
xs_shadow1.push(static_cast<int32_t>(2));
xs_shadow1.push(static_cast<int32_t>(3));
return xs_shadow1;
});
        };
        {
            auto&& _m0_tmp = xs();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::boxed::into_vec(rusty::boxed::box_new(std::array{static_cast<int32_t>(1), static_cast<int32_t>(2), static_cast<int32_t>(3)}));
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void lazy_into_value() {
        auto l = sync::Lazy<int32_t>::new_([&]() -> int32_t { [&]() -> int32_t { rusty::panicking::panic("explicit panic"); }(); });
        if (![&]() -> bool { auto&& _m = Lazy<int32_t>::into_value(std::move(l)); if (_m.is_err()) { return true; } if (true) { return false; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }()) {
            rusty::panicking::panic("assertion failed: matches!(Lazy::into_value(l), Err(_))");
        }
        auto l_shadow1 = sync::Lazy<int32_t>::new_([&]() -> int32_t {
return static_cast<int32_t>(92);
});
        Lazy<int32_t>::force(l_shadow1);
        if (![&]() -> bool { auto&& _m = Lazy<int32_t>::into_value(std::move(l_shadow1)); if (_m.is_ok()) { auto&& _mv0 = std::as_const(_m).unwrap(); if (_mv0 == 92) { return true; } } if (true) { return false; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }()) {
            rusty::panicking::panic("assertion failed: matches!(Lazy::into_value(l), Ok(92))");
        }
    }

    void lazy_poisoning() {
        const sync::Lazy<rusty::String> x = sync::Lazy<rusty::String>::new_([&]() -> rusty::String {
[&]() -> rusty::String { rusty::panicking::panic_fmt(std::string("kaboom")); }();
});
        for (auto&& _ : rusty::for_in(rusty::range(0, 2))) {
            const auto res = rusty::panic::catch_unwind([&]() { return rusty::len((*x)); });
            if (!res.is_err()) {
                rusty::panicking::panic("assertion failed: res.is_err()");
            }
        }
    }

    void arrrrrrrrrrrrrrrrrrrrrr() {
        std::optional<sync::Lazy<const rusty::String&>> lazy;
        {
            const auto s = rusty::String::new_();
            lazy.emplace(sync::Lazy<const rusty::String&>::new_([&]() -> const rusty::String& { return s; }));
            [&]() { static_cast<void>(lazy.value()); return std::make_tuple(); }();
        }
    }

    void lazy_is_sync_send() {
        const rusty::SafeFn<void()> assert_traits = +[]() {
        };
        assert_traits();
    }

}

namespace race {

    void once_non_zero_usize_smoke_test();
    void once_non_zero_usize_set();
    void once_non_zero_usize_first_wins();
    void once_bool_smoke_test();
    void once_bool_set();
    void once_bool_get_or_try_init();
    void once_ref_smoke_test();
    void once_ref_set();
    void get_unchecked();

    using ::rusty::Barrier;

    using ::rusty::num::NonZeroUsize;
    using ::rusty::sync::atomic::AtomicUsize;
    constexpr auto SeqCst = rusty::sync::atomic::Ordering::SeqCst;
    using ::rusty::thread::scope;

    using ::race::OnceBool;
    using ::race::OnceNonZeroUsize;
    using ::race::OnceRef;


    // Rust-only libtest metadata const skipped: once_non_zero_usize_smoke_test (marker: race::once_non_zero_usize_smoke_test, should_panic: no)


    // Rust-only libtest metadata const skipped: once_non_zero_usize_set (marker: race::once_non_zero_usize_set, should_panic: no)


    // Rust-only libtest metadata const skipped: once_non_zero_usize_first_wins (marker: race::once_non_zero_usize_first_wins, should_panic: no)


    // Rust-only libtest metadata const skipped: once_bool_smoke_test (marker: race::once_bool_smoke_test, should_panic: no)


    // Rust-only libtest metadata const skipped: once_bool_set (marker: race::once_bool_set, should_panic: no)


    // Rust-only libtest metadata const skipped: once_bool_get_or_try_init (marker: race::once_bool_get_or_try_init, should_panic: no)


    // Rust-only libtest metadata const skipped: once_ref_smoke_test (marker: race::once_ref_smoke_test, should_panic: no)


    // Rust-only libtest metadata const skipped: once_ref_set (marker: race::once_ref_set, should_panic: no)


    // Rust-only libtest metadata const skipped: get_unchecked (marker: race::get_unchecked, should_panic: no)

    void once_non_zero_usize_smoke_test() {
        const auto cnt = AtomicUsize::new_(0);
        const auto cell = OnceNonZeroUsize::new_();
        auto val = NonZeroUsize::new_(92).unwrap();
        scope([&](auto&& s) {
s.spawn([&]() {
{
    auto&& _m0_tmp = cell.get_or_init([&]() {
cnt.fetch_add(1, rusty::sync::atomic::Ordering::SeqCst);
return val;
});
    auto _m0 = &_m0_tmp;
    auto _m1 = &val;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
{
    auto&& _m0_tmp = cnt.load(rusty::sync::atomic::Ordering::SeqCst);
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = static_cast<int32_t>(1);
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
{
    auto&& _m0_tmp = cell.get_or_init([&]() {
cnt.fetch_add(1, rusty::sync::atomic::Ordering::SeqCst);
return val;
});
    auto _m0 = &_m0_tmp;
    auto _m1 = &val;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
{
    auto&& _m0_tmp = cnt.load(rusty::sync::atomic::Ordering::SeqCst);
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = static_cast<int32_t>(1);
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
});
});
        {
            auto&& _m0_tmp = cell.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Some(std::move(val));
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = cnt.load(rusty::sync::atomic::Ordering::SeqCst);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_non_zero_usize_set() {
        auto val1 = NonZeroUsize::new_(92).unwrap();
        auto val2 = NonZeroUsize::new_(62).unwrap();
        const auto cell = OnceNonZeroUsize::new_();
        if (!cell.set(std::move(val1)).is_ok()) {
            rusty::panicking::panic("assertion failed: cell.set(val1).is_ok()");
        }
        {
            auto&& _m0_tmp = cell.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Some(std::move(val1));
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        if (!cell.set(std::move(val2)).is_err()) {
            rusty::panicking::panic("assertion failed: cell.set(val2).is_err()");
        }
        {
            auto&& _m0_tmp = cell.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Some(std::move(val1));
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_non_zero_usize_first_wins() {
        auto val1 = NonZeroUsize::new_(92).unwrap();
        const auto val2 = NonZeroUsize::new_(62).unwrap();
        const auto cell = OnceNonZeroUsize::new_();
        const auto b1 = Barrier::new_(2);
        const auto b2 = Barrier::new_(2);
        const auto b3 = Barrier::new_(2);
        scope([&](auto&& s) {
s.spawn([&]() {
const auto& r1 = cell.get_or_init([&]() {
b1.wait();
b2.wait();
return val1;
});
{
    auto _m0 = &r1;
    auto _m1 = &val1;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
b3.wait();
});
b1.wait();
s.spawn([&]() {
const auto& r2 = cell.get_or_init([&]() {
b2.wait();
b3.wait();
return val2;
});
{
    auto _m0 = &r2;
    auto _m1 = &val1;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
});
});
        {
            auto&& _m0_tmp = cell.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Some(std::move(val1));
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_bool_smoke_test() {
        const auto cnt = AtomicUsize::new_(0);
        const auto cell = OnceBool::new_();
        scope([&](auto&& s) {
s.spawn([&]() {
{
    auto&& _m0_tmp = cell.get_or_init([&]() {
cnt.fetch_add(1, rusty::sync::atomic::Ordering::SeqCst);
return false;
});
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = false;
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
{
    auto&& _m0_tmp = cnt.load(rusty::sync::atomic::Ordering::SeqCst);
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = static_cast<int32_t>(1);
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
{
    auto&& _m0_tmp = cell.get_or_init([&]() {
cnt.fetch_add(1, rusty::sync::atomic::Ordering::SeqCst);
return false;
});
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = false;
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
{
    auto&& _m0_tmp = cnt.load(rusty::sync::atomic::Ordering::SeqCst);
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = static_cast<int32_t>(1);
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
});
});
        {
            auto&& _m0_tmp = cell.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<bool>(false);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = cnt.load(rusty::sync::atomic::Ordering::SeqCst);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_bool_set() {
        const auto cell = OnceBool::new_();
        if (!cell.set(false).is_ok()) {
            rusty::panicking::panic("assertion failed: cell.set(false).is_ok()");
        }
        {
            auto&& _m0_tmp = cell.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<bool>(false);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        if (!cell.set(true).is_err()) {
            rusty::panicking::panic("assertion failed: cell.set(true).is_err()");
        }
        {
            auto&& _m0_tmp = cell.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<bool>(false);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_bool_get_or_try_init() {
        const auto cell = OnceBool::new_();
        const rusty::Result<bool, std::tuple<>> result1 = cell.get_or_try_init([&]() -> rusty::Result<bool, std::tuple<>> { return rusty::Result<bool, std::tuple<>>::Ok(true); });
        const rusty::Result<bool, std::tuple<>> result2 = cell.get_or_try_init([&]() -> rusty::Result<bool, std::tuple<>> { return rusty::Result<bool, std::tuple<>>::Ok(false); });
        {
            auto _m0 = &result1;
            auto&& _m1_tmp = [&]() { using _ResultCtorCtx = std::remove_cvref_t<decltype((result1))>; return _ResultCtorCtx::Ok(true); }();
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto _m0 = &result2;
            auto&& _m1_tmp = [&]() { using _ResultCtorCtx = std::remove_cvref_t<decltype((result2))>; return _ResultCtorCtx::Ok(true); }();
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        const auto cell_shadow1 = OnceBool::new_();
        const rusty::Result<bool, std::tuple<>> result3 = cell_shadow1.get_or_try_init([&]() -> rusty::Result<bool, std::tuple<>> { return rusty::Result<bool, std::tuple<>>::Err(std::make_tuple()); });
        {
            auto _m0 = &result3;
            auto&& _m1_tmp = [&]() { using _ResultCtorCtx = std::remove_cvref_t<decltype((result3))>; return _ResultCtorCtx::Err(std::make_tuple()); }();
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_ref_smoke_test() {
        const rusty::sync::atomic::AtomicUsize cnt = AtomicUsize::new_(0);
        const race::OnceRef<std::string_view> cell = race::OnceRef<std::string_view>::new_();
        scope([&](auto&& s) {
s.spawn([&]() {
{
    auto&& _m0_tmp = cell.get_or_init([&]() -> std::string_view {
cnt.fetch_add(1, rusty::sync::atomic::Ordering::SeqCst);
return std::string_view("false");
});
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = std::string_view("false");
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
{
    auto&& _m0_tmp = cnt.load(rusty::sync::atomic::Ordering::SeqCst);
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = static_cast<int32_t>(1);
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
{
    auto&& _m0_tmp = cell.get_or_init([&]() -> std::string_view {
cnt.fetch_add(1, rusty::sync::atomic::Ordering::SeqCst);
return std::string_view("false");
});
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = std::string_view("false");
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
{
    auto&& _m0_tmp = cnt.load(rusty::sync::atomic::Ordering::SeqCst);
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = static_cast<int32_t>(1);
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
});
});
        {
            auto&& _m0_tmp = cell.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<const std::string_view&>([&]() -> const auto& { static const auto _some_ref_tmp = std::string_view("false"); return _some_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = cnt.load(rusty::sync::atomic::Ordering::SeqCst);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_ref_set() {
        const race::OnceRef<std::string_view> cell = race::OnceRef<std::string_view>::new_();
        if (!cell.set(std::string_view("false")).is_ok()) {
            rusty::panicking::panic("assertion failed: cell.set(&\"false\").is_ok()");
        }
        {
            auto&& _m0_tmp = cell.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<const std::string_view&>([&]() -> const auto& { static const auto _some_ref_tmp = std::string_view("false"); return _some_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        if (!cell.set(std::string_view("true")).is_err()) {
            rusty::panicking::panic("assertion failed: cell.set(&\"true\").is_err()");
        }
        {
            auto&& _m0_tmp = cell.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<const std::string_view&>([&]() -> const auto& { static const auto _some_ref_tmp = std::string_view("false"); return _some_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void get_unchecked() {
        const auto cell = OnceNonZeroUsize::new_();
        cell.set(NonZeroUsize::new_(92).unwrap()).unwrap();
        const auto value = cell.get_unchecked();
        {
            auto _m0 = &value;
            auto&& _m1_tmp = NonZeroUsize::new_(92).unwrap();
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

}

// Rust-only libtest main omitted

namespace race_once_box {

    void once_box_smoke_test() {
        using ::rusty::thread::scope;
        const auto heap = Heap::default_();
        const auto global_cnt = AtomicUsize::new_(0);
        auto cell = race::OnceBox<Pebble<std::tuple<>>>::new_();
        const auto b = Barrier::new_(128);
        scope([&](auto&& s) {
for (auto&& _ : rusty::for_in(rusty::range(0, 128))) {
    s.spawn([&]() {
const auto local_cnt = AtomicUsize::new_(0);
cell.get_or_init([&]() -> rusty::Box<Pebble<std::tuple<>>> {
global_cnt.fetch_add(1, rusty::sync::atomic::Ordering::SeqCst);
local_cnt.fetch_add(1, rusty::sync::atomic::Ordering::SeqCst);
b.wait();
return rusty::Box<Pebble<std::tuple<>>>::new_(heap.new_pebble(std::make_tuple()));
});
{
    auto&& _m0_tmp = local_cnt.load(rusty::sync::atomic::Ordering::SeqCst);
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = static_cast<int32_t>(1);
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
cell.get_or_init([&]() -> rusty::Box<Pebble<std::tuple<>>> {
global_cnt.fetch_add(1, rusty::sync::atomic::Ordering::SeqCst);
local_cnt.fetch_add(1, rusty::sync::atomic::Ordering::SeqCst);
return rusty::Box<Pebble<std::tuple<>>>::new_(heap.new_pebble(std::make_tuple()));
});
{
    auto&& _m0_tmp = local_cnt.load(rusty::sync::atomic::Ordering::SeqCst);
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = static_cast<int32_t>(1);
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
});
}
});
        if (!cell.get().is_some()) {
            rusty::panicking::panic("assertion failed: cell.get().is_some()");
        }
        if (!(global_cnt.load(rusty::sync::atomic::Ordering::SeqCst) > 10)) {
            rusty::panicking::panic("assertion failed: global_cnt.load(SeqCst) > 10");
        }
        {
            auto&& _m0_tmp = heap.total();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        rusty::mem::drop(std::move(cell));
        {
            auto&& _m0_tmp = heap.total();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_box_set() {
        const auto heap = Heap::default_();
        auto cell = race::OnceBox<Pebble<std::string_view>>::new_();
        if (!cell.get().is_none()) {
            rusty::panicking::panic("assertion failed: cell.get().is_none()");
        }
        if (!cell.set(rusty::Box<Pebble<std::string_view>>::new_(heap.new_pebble(std::string_view("hello")))).is_ok()) {
            rusty::panicking::panic("assertion failed: cell.set(Box::new(heap.new_pebble(\"hello\"))).is_ok()");
        }
        {
            auto&& _m0_tmp = std::string_view(cell.get().unwrap().val);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = std::string_view("hello");
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = heap.total();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        if (!cell.set(rusty::Box<Pebble<std::string_view>>::new_(heap.new_pebble(std::string_view("world")))).is_err()) {
            rusty::panicking::panic("assertion failed: cell.set(Box::new(heap.new_pebble(\"world\"))).is_err()");
        }
        {
            auto&& _m0_tmp = std::string_view(cell.get().unwrap().val);
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = std::string_view("hello");
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = heap.total();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        rusty::mem::drop(std::move(cell));
        {
            auto&& _m0_tmp = heap.total();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = static_cast<int32_t>(0);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_box_first_wins() {
        using ::rusty::thread::scope;
        const auto cell = race::OnceBox<int32_t>::new_();
        auto val1 = 92;
        const auto val2 = 62;
        const auto b1 = Barrier::new_(2);
        const auto b2 = Barrier::new_(2);
        const auto b3 = Barrier::new_(2);
        scope([&](auto&& s) {
s.spawn([&]() {
const auto& r1 = cell.get_or_init([&]() -> rusty::Box<int32_t> {
b1.wait();
b2.wait();
return rusty::Box<int32_t>::new_(std::move(val1));
});
{
    auto&& _m0_tmp = r1;
    auto _m0 = &_m0_tmp;
    auto _m1 = &val1;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
b3.wait();
});
b1.wait();
s.spawn([&]() {
const auto& r2 = cell.get_or_init([&]() -> rusty::Box<int32_t> {
b2.wait();
b3.wait();
return rusty::Box<int32_t>::new_(std::move(val2));
});
{
    auto&& _m0_tmp = r2;
    auto _m0 = &_m0_tmp;
    auto _m1 = &val1;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
        auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
        if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
            const auto kind = rusty::panicking::AssertKind::Eq;
            rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
        }
        _m_matched = true;
    }
}
});
});
        {
            auto&& _m0_tmp = cell.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::SomeRef(val1);
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_box_reentrant() {
        const auto cell = race::OnceBox<rusty::String>::new_();
        const auto& res = cell.get_or_init([&]() -> rusty::Box<rusty::String> {
cell.get_or_init([&]() -> rusty::Box<rusty::String> { return rusty::Box<rusty::String>::new_(rusty::String::from(rusty::to_string("hello"))); });
return rusty::Box<rusty::String>::new_(rusty::String::from(rusty::to_string("world")));
});
        {
            auto _m0 = &res;
            auto&& _m1_tmp = "hello";
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void once_box_default() {
        struct Foo {
        };
        const race::OnceBox<Foo> cell = rusty::default_value<race::OnceBox<Foo>>();
        if (!cell.get().is_none()) {
            rusty::panicking::panic("assertion failed: cell.get().is_none()");
        }
    }

    void onece_box_with_value() {
        const auto cell = race::OnceBox<int32_t>::with_value(rusty::Box<int32_t>::new_(static_cast<int32_t>(92)));
        {
            auto&& _m0_tmp = cell.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<const int32_t&>([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<int32_t>(92); return _some_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

    void onece_box_clone() {
        const auto cell1 = race::OnceBox<int32_t>::new_();
        const auto cell2 = rusty::clone(cell1);
        cell1.set(rusty::Box<int32_t>::new_(static_cast<int32_t>(92))).unwrap();
        const auto cell3 = rusty::clone(cell1);
        {
            auto&& _m0_tmp = cell1.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<const int32_t&>([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<int32_t>(92); return _some_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = cell2.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::None;
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
        {
            auto&& _m0_tmp = cell3.get();
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = rusty::Option<const int32_t&>([&]() -> const auto& { static const auto _some_ref_tmp = static_cast<int32_t>(92); return _some_ref_tmp; }());
            auto _m1 = &_m1_tmp;
            auto _m_tuple = std::make_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched) {
                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                    const auto kind = rusty::panicking::AssertKind::Eq;
                    rusty::panicking::assert_failed(std::move(kind), rusty::detail::deref_if_pointer_like(left_val), rusty::detail::deref_if_pointer_like(right_val), rusty::None);
                }
                _m_matched = true;
            }
        }
    }

}


namespace race_once_box {
    Heap Heap::default_() {
        return Heap{.total_field = rusty::default_value<rusty::Arc<rusty::sync::atomic::AtomicUsize>>()};
    }
}

namespace race_once_box {
    size_t Heap::total() const {
        return this->total_field->load(rusty::sync::atomic::Ordering::SeqCst);
    }
}

namespace race_once_box {
    template<typename T>
    Pebble<T> Heap::new_pebble(T val) const {
        this->total_field->fetch_add(1, rusty::sync::atomic::Ordering::SeqCst);
        return Pebble<T>(std::move(val), rusty::Arc<rusty::sync::atomic::AtomicUsize>::clone(&this->total_field));
    }
}


// Runnable wrappers for expanded Rust test bodies
// Rust-only libtest wrapper metadata: marker=race_once_box::once_box_smoke_test should_panic=no
void rusty_test_race_once_box_once_box_smoke_test() {
    race_once_box::once_box_smoke_test();
}
// Rust-only libtest wrapper metadata: marker=race_once_box::once_box_set should_panic=no
void rusty_test_race_once_box_once_box_set() {
    race_once_box::once_box_set();
}
// Rust-only libtest wrapper metadata: marker=race_once_box::once_box_first_wins should_panic=no
void rusty_test_race_once_box_once_box_first_wins() {
    race_once_box::once_box_first_wins();
}
// Rust-only libtest wrapper metadata: marker=race_once_box::once_box_reentrant should_panic=no
void rusty_test_race_once_box_once_box_reentrant() {
    race_once_box::once_box_reentrant();
}
// Rust-only libtest wrapper metadata: marker=race_once_box::once_box_default should_panic=no
void rusty_test_race_once_box_once_box_default() {
    race_once_box::once_box_default();
}
// Rust-only libtest wrapper metadata: marker=race_once_box::onece_box_with_value should_panic=no
void rusty_test_race_once_box_onece_box_with_value() {
    race_once_box::onece_box_with_value();
}
// Rust-only libtest wrapper metadata: marker=race_once_box::onece_box_clone should_panic=no
void rusty_test_race_once_box_onece_box_clone() {
    race_once_box::onece_box_clone();
}
// Rust-only libtest wrapper metadata: marker=unsync_once_cell::once_cell should_panic=no
void rusty_test_unsync_once_cell_once_cell() {
    unsync_once_cell::once_cell();
}
// Rust-only libtest wrapper metadata: marker=unsync_once_cell::once_cell_with_value should_panic=no
void rusty_test_unsync_once_cell_once_cell_with_value() {
    unsync_once_cell::once_cell_with_value();
}
// Rust-only libtest wrapper metadata: marker=unsync_once_cell::once_cell_get_mut should_panic=no
void rusty_test_unsync_once_cell_once_cell_get_mut() {
    unsync_once_cell::once_cell_get_mut();
}
// Rust-only libtest wrapper metadata: marker=unsync_once_cell::once_cell_drop should_panic=no
void rusty_test_unsync_once_cell_once_cell_drop() {
    unsync_once_cell::once_cell_drop();
}
// Rust-only libtest wrapper metadata: marker=unsync_once_cell::once_cell_drop_empty should_panic=no
void rusty_test_unsync_once_cell_once_cell_drop_empty() {
    unsync_once_cell::once_cell_drop_empty();
}
// Rust-only libtest wrapper metadata: marker=unsync_once_cell::clone should_panic=no
void rusty_test_unsync_once_cell_clone() {
    unsync_once_cell::clone();
}
// Rust-only libtest wrapper metadata: marker=unsync_once_cell::get_or_try_init should_panic=no
void rusty_test_unsync_once_cell_get_or_try_init() {
    unsync_once_cell::get_or_try_init();
}
// Rust-only libtest wrapper metadata: marker=unsync_once_cell::from_impl should_panic=no
void rusty_test_unsync_once_cell_from_impl() {
    unsync_once_cell::from_impl();
}
// Rust-only libtest wrapper metadata: marker=unsync_once_cell::partialeq_impl should_panic=no
void rusty_test_unsync_once_cell_partialeq_impl() {
    unsync_once_cell::partialeq_impl();
}
// Rust-only libtest wrapper metadata: marker=unsync_once_cell::into_inner should_panic=no
void rusty_test_unsync_once_cell_into_inner() {
    unsync_once_cell::into_inner();
}
// Rust-only libtest wrapper metadata: marker=unsync_once_cell::debug_impl should_panic=no
void rusty_test_unsync_once_cell_debug_impl() {
    unsync_once_cell::debug_impl();
}
// Rust-only libtest wrapper metadata: marker=unsync_once_cell::reentrant_init should_panic=yes
void rusty_test_unsync_once_cell_reentrant_init() {
    unsync_once_cell::reentrant_init();
}
// Rust-only libtest wrapper metadata: marker=unsync_once_cell::aliasing_in_get should_panic=no
void rusty_test_unsync_once_cell_aliasing_in_get() {
    unsync_once_cell::aliasing_in_get();
}
// Rust-only libtest wrapper metadata: marker=unsync_once_cell::arrrrrrrrrrrrrrrrrrrrrr should_panic=no
void rusty_test_unsync_once_cell_arrrrrrrrrrrrrrrrrrrrrr() {
    unsync_once_cell::arrrrrrrrrrrrrrrrrrrrrr();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::once_cell should_panic=no
void rusty_test_sync_once_cell_once_cell() {
    sync_once_cell::once_cell();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::once_cell_with_value should_panic=no
void rusty_test_sync_once_cell_once_cell_with_value() {
    sync_once_cell::once_cell_with_value();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::once_cell_get_mut should_panic=no
void rusty_test_sync_once_cell_once_cell_get_mut() {
    sync_once_cell::once_cell_get_mut();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::once_cell_get_unchecked should_panic=no
void rusty_test_sync_once_cell_once_cell_get_unchecked() {
    sync_once_cell::once_cell_get_unchecked();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::once_cell_drop should_panic=no
void rusty_test_sync_once_cell_once_cell_drop() {
    sync_once_cell::once_cell_drop();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::once_cell_drop_empty should_panic=no
void rusty_test_sync_once_cell_once_cell_drop_empty() {
    sync_once_cell::once_cell_drop_empty();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::clone should_panic=no
void rusty_test_sync_once_cell_clone() {
    sync_once_cell::clone();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::get_or_try_init should_panic=no
void rusty_test_sync_once_cell_get_or_try_init() {
    sync_once_cell::get_or_try_init();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::wait should_panic=no
void rusty_test_sync_once_cell_wait() {
    sync_once_cell::wait();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::wait_panic should_panic=no
void rusty_test_sync_once_cell_wait_panic() {
    sync_once_cell::wait_panic();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::get_or_init_stress should_panic=no
void rusty_test_sync_once_cell_get_or_init_stress() {
    sync_once_cell::get_or_init_stress();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::from_impl should_panic=no
void rusty_test_sync_once_cell_from_impl() {
    sync_once_cell::from_impl();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::partialeq_impl should_panic=no
void rusty_test_sync_once_cell_partialeq_impl() {
    sync_once_cell::partialeq_impl();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::into_inner should_panic=no
void rusty_test_sync_once_cell_into_inner() {
    sync_once_cell::into_inner();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::debug_impl should_panic=no
void rusty_test_sync_once_cell_debug_impl() {
    sync_once_cell::debug_impl();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::reentrant_init should_panic=no
void rusty_test_sync_once_cell_reentrant_init() {
    sync_once_cell::reentrant_init();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::eval_once_macro should_panic=no
void rusty_test_sync_once_cell_eval_once_macro() {
    sync_once_cell::eval_once_macro();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::once_cell_does_not_leak_partially_constructed_boxes should_panic=no
void rusty_test_sync_once_cell_once_cell_does_not_leak_partially_constructed_boxes() {
    sync_once_cell::once_cell_does_not_leak_partially_constructed_boxes();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::get_does_not_block should_panic=no
void rusty_test_sync_once_cell_get_does_not_block() {
    sync_once_cell::get_does_not_block();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::arrrrrrrrrrrrrrrrrrrrrr should_panic=no
void rusty_test_sync_once_cell_arrrrrrrrrrrrrrrrrrrrrr() {
    sync_once_cell::arrrrrrrrrrrrrrrrrrrrrr();
}
// Rust-only libtest wrapper metadata: marker=sync_once_cell::once_cell_is_sync_send should_panic=no
void rusty_test_sync_once_cell_once_cell_is_sync_send() {
    sync_once_cell::once_cell_is_sync_send();
}
// Rust-only libtest wrapper metadata: marker=unsync_lazy::lazy_new should_panic=no
void rusty_test_unsync_lazy_lazy_new() {
    unsync_lazy::lazy_new();
}
// Rust-only libtest wrapper metadata: marker=unsync_lazy::lazy_deref_mut should_panic=no
void rusty_test_unsync_lazy_lazy_deref_mut() {
    unsync_lazy::lazy_deref_mut();
}
// Rust-only libtest wrapper metadata: marker=unsync_lazy::lazy_force_mut should_panic=no
void rusty_test_unsync_lazy_lazy_force_mut() {
    unsync_lazy::lazy_force_mut();
}
// Rust-only libtest wrapper metadata: marker=unsync_lazy::lazy_get_mut should_panic=no
void rusty_test_unsync_lazy_lazy_get_mut() {
    unsync_lazy::lazy_get_mut();
}
// Rust-only libtest wrapper metadata: marker=unsync_lazy::lazy_default should_panic=no
void rusty_test_unsync_lazy_lazy_default() {
    unsync_lazy::lazy_default();
}
// Rust-only libtest wrapper metadata: marker=unsync_lazy::lazy_into_value should_panic=no
void rusty_test_unsync_lazy_lazy_into_value() {
    unsync_lazy::lazy_into_value();
}
// Rust-only libtest wrapper metadata: marker=unsync_lazy::lazy_poisoning should_panic=no
void rusty_test_unsync_lazy_lazy_poisoning() {
    unsync_lazy::lazy_poisoning();
}
// Rust-only libtest wrapper metadata: marker=unsync_lazy::arrrrrrrrrrrrrrrrrrrrrr should_panic=no
void rusty_test_unsync_lazy_arrrrrrrrrrrrrrrrrrrrrr() {
    unsync_lazy::arrrrrrrrrrrrrrrrrrrrrr();
}
// Rust-only libtest wrapper metadata: marker=sync_lazy::lazy_new should_panic=no
void rusty_test_sync_lazy_lazy_new() {
    sync_lazy::lazy_new();
}
// Rust-only libtest wrapper metadata: marker=sync_lazy::lazy_deref_mut should_panic=no
void rusty_test_sync_lazy_lazy_deref_mut() {
    sync_lazy::lazy_deref_mut();
}
// Rust-only libtest wrapper metadata: marker=sync_lazy::lazy_force_mut should_panic=no
void rusty_test_sync_lazy_lazy_force_mut() {
    sync_lazy::lazy_force_mut();
}
// Rust-only libtest wrapper metadata: marker=sync_lazy::lazy_get_mut should_panic=no
void rusty_test_sync_lazy_lazy_get_mut() {
    sync_lazy::lazy_get_mut();
}
// Rust-only libtest wrapper metadata: marker=sync_lazy::lazy_default should_panic=no
void rusty_test_sync_lazy_lazy_default() {
    sync_lazy::lazy_default();
}
// Rust-only libtest wrapper metadata: marker=sync_lazy::static_lazy should_panic=no
void rusty_test_sync_lazy_static_lazy() {
    sync_lazy::static_lazy();
}
// Rust-only libtest wrapper metadata: marker=sync_lazy::static_lazy_via_fn should_panic=no
void rusty_test_sync_lazy_static_lazy_via_fn() {
    sync_lazy::static_lazy_via_fn();
}
// Rust-only libtest wrapper metadata: marker=sync_lazy::lazy_into_value should_panic=no
void rusty_test_sync_lazy_lazy_into_value() {
    sync_lazy::lazy_into_value();
}
// Rust-only libtest wrapper metadata: marker=sync_lazy::lazy_poisoning should_panic=no
void rusty_test_sync_lazy_lazy_poisoning() {
    sync_lazy::lazy_poisoning();
}
// Rust-only libtest wrapper metadata: marker=sync_lazy::arrrrrrrrrrrrrrrrrrrrrr should_panic=no
void rusty_test_sync_lazy_arrrrrrrrrrrrrrrrrrrrrr() {
    sync_lazy::arrrrrrrrrrrrrrrrrrrrrr();
}
// Rust-only libtest wrapper metadata: marker=sync_lazy::lazy_is_sync_send should_panic=no
void rusty_test_sync_lazy_lazy_is_sync_send() {
    sync_lazy::lazy_is_sync_send();
}
// Rust-only libtest wrapper metadata: marker=race::once_non_zero_usize_smoke_test should_panic=no
void rusty_test_race_once_non_zero_usize_smoke_test() {
    race::once_non_zero_usize_smoke_test();
}
// Rust-only libtest wrapper metadata: marker=race::once_non_zero_usize_set should_panic=no
void rusty_test_race_once_non_zero_usize_set() {
    race::once_non_zero_usize_set();
}
// Rust-only libtest wrapper metadata: marker=race::once_non_zero_usize_first_wins should_panic=no
void rusty_test_race_once_non_zero_usize_first_wins() {
    race::once_non_zero_usize_first_wins();
}
// Rust-only libtest wrapper metadata: marker=race::once_bool_smoke_test should_panic=no
void rusty_test_race_once_bool_smoke_test() {
    race::once_bool_smoke_test();
}
// Rust-only libtest wrapper metadata: marker=race::once_bool_set should_panic=no
void rusty_test_race_once_bool_set() {
    race::once_bool_set();
}
// Rust-only libtest wrapper metadata: marker=race::once_bool_get_or_try_init should_panic=no
void rusty_test_race_once_bool_get_or_try_init() {
    race::once_bool_get_or_try_init();
}
// Rust-only libtest wrapper metadata: marker=race::once_ref_smoke_test should_panic=no
void rusty_test_race_once_ref_smoke_test() {
    race::once_ref_smoke_test();
}
// Rust-only libtest wrapper metadata: marker=race::once_ref_set should_panic=no
void rusty_test_race_once_ref_set() {
    race::once_ref_set();
}
// Rust-only libtest wrapper metadata: marker=race::get_unchecked should_panic=no
void rusty_test_race_get_unchecked() {
    race::get_unchecked();
}


// ── Test runner ──
int main(int argc, char** argv) {
    if (argc == 3 && std::string(argv[1]) == "--rusty-single-test") {
        const std::string test_name = argv[2];
        rusty::mem::clear_all_forgotten_addresses();
        try {
            if (test_name == "rusty_test_imp_tests_poison_bad") { rusty_test_imp_tests_poison_bad(); return 0; }
            if (test_name == "rusty_test_imp_tests_smoke_once") { rusty_test_imp_tests_smoke_once(); return 0; }
            if (test_name == "rusty_test_imp_tests_stampede_once") { rusty_test_imp_tests_stampede_once(); return 0; }
            if (test_name == "rusty_test_imp_tests_test_size") { rusty_test_imp_tests_test_size(); return 0; }
            if (test_name == "rusty_test_imp_tests_wait_for_force_to_finish") { rusty_test_imp_tests_wait_for_force_to_finish(); return 0; }
            if (test_name == "rusty_test_race_get_unchecked") { rusty_test_race_get_unchecked(); return 0; }
            if (test_name == "rusty_test_race_once_bool_get_or_try_init") { rusty_test_race_once_bool_get_or_try_init(); return 0; }
            if (test_name == "rusty_test_race_once_bool_set") { rusty_test_race_once_bool_set(); return 0; }
            if (test_name == "rusty_test_race_once_bool_smoke_test") { rusty_test_race_once_bool_smoke_test(); return 0; }
            if (test_name == "rusty_test_race_once_box_once_box_default") { rusty_test_race_once_box_once_box_default(); return 0; }
            if (test_name == "rusty_test_race_once_box_once_box_first_wins") { rusty_test_race_once_box_once_box_first_wins(); return 0; }
            if (test_name == "rusty_test_race_once_box_once_box_reentrant") { rusty_test_race_once_box_once_box_reentrant(); return 0; }
            if (test_name == "rusty_test_race_once_box_once_box_set") { rusty_test_race_once_box_once_box_set(); return 0; }
            if (test_name == "rusty_test_race_once_box_once_box_smoke_test") { rusty_test_race_once_box_once_box_smoke_test(); return 0; }
            if (test_name == "rusty_test_race_once_box_onece_box_clone") { rusty_test_race_once_box_onece_box_clone(); return 0; }
            if (test_name == "rusty_test_race_once_box_onece_box_with_value") { rusty_test_race_once_box_onece_box_with_value(); return 0; }
            if (test_name == "rusty_test_race_once_non_zero_usize_first_wins") { rusty_test_race_once_non_zero_usize_first_wins(); return 0; }
            if (test_name == "rusty_test_race_once_non_zero_usize_set") { rusty_test_race_once_non_zero_usize_set(); return 0; }
            if (test_name == "rusty_test_race_once_non_zero_usize_smoke_test") { rusty_test_race_once_non_zero_usize_smoke_test(); return 0; }
            if (test_name == "rusty_test_race_once_ref_set") { rusty_test_race_once_ref_set(); return 0; }
            if (test_name == "rusty_test_race_once_ref_smoke_test") { rusty_test_race_once_ref_smoke_test(); return 0; }
            if (test_name == "rusty_test_sync_lazy_arrrrrrrrrrrrrrrrrrrrrr") { rusty_test_sync_lazy_arrrrrrrrrrrrrrrrrrrrrr(); return 0; }
            if (test_name == "rusty_test_sync_lazy_lazy_default") { rusty_test_sync_lazy_lazy_default(); return 0; }
            if (test_name == "rusty_test_sync_lazy_lazy_deref_mut") { rusty_test_sync_lazy_lazy_deref_mut(); return 0; }
            if (test_name == "rusty_test_sync_lazy_lazy_force_mut") { rusty_test_sync_lazy_lazy_force_mut(); return 0; }
            if (test_name == "rusty_test_sync_lazy_lazy_get_mut") { rusty_test_sync_lazy_lazy_get_mut(); return 0; }
            if (test_name == "rusty_test_sync_lazy_lazy_into_value") { rusty_test_sync_lazy_lazy_into_value(); return 0; }
            if (test_name == "rusty_test_sync_lazy_lazy_is_sync_send") { rusty_test_sync_lazy_lazy_is_sync_send(); return 0; }
            if (test_name == "rusty_test_sync_lazy_lazy_new") { rusty_test_sync_lazy_lazy_new(); return 0; }
            if (test_name == "rusty_test_sync_lazy_lazy_poisoning") { rusty_test_sync_lazy_lazy_poisoning(); return 0; }
            if (test_name == "rusty_test_sync_lazy_static_lazy") { rusty_test_sync_lazy_static_lazy(); return 0; }
            if (test_name == "rusty_test_sync_lazy_static_lazy_via_fn") { rusty_test_sync_lazy_static_lazy_via_fn(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_arrrrrrrrrrrrrrrrrrrrrr") { rusty_test_sync_once_cell_arrrrrrrrrrrrrrrrrrrrrr(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_clone") { rusty_test_sync_once_cell_clone(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_debug_impl") { rusty_test_sync_once_cell_debug_impl(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_eval_once_macro") { rusty_test_sync_once_cell_eval_once_macro(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_from_impl") { rusty_test_sync_once_cell_from_impl(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_get_does_not_block") { rusty_test_sync_once_cell_get_does_not_block(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_get_or_init_stress") { rusty_test_sync_once_cell_get_or_init_stress(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_get_or_try_init") { rusty_test_sync_once_cell_get_or_try_init(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_into_inner") { rusty_test_sync_once_cell_into_inner(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_once_cell") { rusty_test_sync_once_cell_once_cell(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_once_cell_does_not_leak_partially_constructed_boxes") { rusty_test_sync_once_cell_once_cell_does_not_leak_partially_constructed_boxes(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_once_cell_drop") { rusty_test_sync_once_cell_once_cell_drop(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_once_cell_drop_empty") { rusty_test_sync_once_cell_once_cell_drop_empty(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_once_cell_get_mut") { rusty_test_sync_once_cell_once_cell_get_mut(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_once_cell_get_unchecked") { rusty_test_sync_once_cell_once_cell_get_unchecked(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_once_cell_is_sync_send") { rusty_test_sync_once_cell_once_cell_is_sync_send(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_once_cell_with_value") { rusty_test_sync_once_cell_once_cell_with_value(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_partialeq_impl") { rusty_test_sync_once_cell_partialeq_impl(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_reentrant_init") { rusty_test_sync_once_cell_reentrant_init(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_wait") { rusty_test_sync_once_cell_wait(); return 0; }
            if (test_name == "rusty_test_sync_once_cell_wait_panic") { rusty_test_sync_once_cell_wait_panic(); return 0; }
            if (test_name == "rusty_test_unsync_lazy_arrrrrrrrrrrrrrrrrrrrrr") { rusty_test_unsync_lazy_arrrrrrrrrrrrrrrrrrrrrr(); return 0; }
            if (test_name == "rusty_test_unsync_lazy_lazy_default") { rusty_test_unsync_lazy_lazy_default(); return 0; }
            if (test_name == "rusty_test_unsync_lazy_lazy_deref_mut") { rusty_test_unsync_lazy_lazy_deref_mut(); return 0; }
            if (test_name == "rusty_test_unsync_lazy_lazy_force_mut") { rusty_test_unsync_lazy_lazy_force_mut(); return 0; }
            if (test_name == "rusty_test_unsync_lazy_lazy_get_mut") { rusty_test_unsync_lazy_lazy_get_mut(); return 0; }
            if (test_name == "rusty_test_unsync_lazy_lazy_into_value") { rusty_test_unsync_lazy_lazy_into_value(); return 0; }
            if (test_name == "rusty_test_unsync_lazy_lazy_new") { rusty_test_unsync_lazy_lazy_new(); return 0; }
            if (test_name == "rusty_test_unsync_lazy_lazy_poisoning") { rusty_test_unsync_lazy_lazy_poisoning(); return 0; }
            if (test_name == "rusty_test_unsync_once_cell_aliasing_in_get") { rusty_test_unsync_once_cell_aliasing_in_get(); return 0; }
            if (test_name == "rusty_test_unsync_once_cell_arrrrrrrrrrrrrrrrrrrrrr") { rusty_test_unsync_once_cell_arrrrrrrrrrrrrrrrrrrrrr(); return 0; }
            if (test_name == "rusty_test_unsync_once_cell_clone") { rusty_test_unsync_once_cell_clone(); return 0; }
            if (test_name == "rusty_test_unsync_once_cell_debug_impl") { rusty_test_unsync_once_cell_debug_impl(); return 0; }
            if (test_name == "rusty_test_unsync_once_cell_from_impl") { rusty_test_unsync_once_cell_from_impl(); return 0; }
            if (test_name == "rusty_test_unsync_once_cell_get_or_try_init") { rusty_test_unsync_once_cell_get_or_try_init(); return 0; }
            if (test_name == "rusty_test_unsync_once_cell_into_inner") { rusty_test_unsync_once_cell_into_inner(); return 0; }
            if (test_name == "rusty_test_unsync_once_cell_once_cell") { rusty_test_unsync_once_cell_once_cell(); return 0; }
            if (test_name == "rusty_test_unsync_once_cell_once_cell_drop") { rusty_test_unsync_once_cell_once_cell_drop(); return 0; }
            if (test_name == "rusty_test_unsync_once_cell_once_cell_drop_empty") { rusty_test_unsync_once_cell_once_cell_drop_empty(); return 0; }
            if (test_name == "rusty_test_unsync_once_cell_once_cell_get_mut") { rusty_test_unsync_once_cell_once_cell_get_mut(); return 0; }
            if (test_name == "rusty_test_unsync_once_cell_once_cell_with_value") { rusty_test_unsync_once_cell_once_cell_with_value(); return 0; }
            if (test_name == "rusty_test_unsync_once_cell_partialeq_impl") { rusty_test_unsync_once_cell_partialeq_impl(); return 0; }
            if (test_name == "rusty_test_unsync_once_cell_reentrant_init") { rusty_test_unsync_once_cell_reentrant_init(); return 0; }
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
    try { rusty_test_imp_tests_poison_bad(); std::cout << "  imp_tests_poison_bad PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  imp_tests_poison_bad FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  imp_tests_poison_bad FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_imp_tests_smoke_once(); std::cout << "  imp_tests_smoke_once PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  imp_tests_smoke_once FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  imp_tests_smoke_once FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_imp_tests_stampede_once(); std::cout << "  imp_tests_stampede_once PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  imp_tests_stampede_once FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  imp_tests_stampede_once FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_imp_tests_test_size(); std::cout << "  imp_tests_test_size PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  imp_tests_test_size FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  imp_tests_test_size FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_imp_tests_wait_for_force_to_finish(); std::cout << "  imp_tests_wait_for_force_to_finish PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  imp_tests_wait_for_force_to_finish FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  imp_tests_wait_for_force_to_finish FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_race_get_unchecked(); std::cout << "  race_get_unchecked PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  race_get_unchecked FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  race_get_unchecked FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_race_once_bool_get_or_try_init(); std::cout << "  race_once_bool_get_or_try_init PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  race_once_bool_get_or_try_init FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  race_once_bool_get_or_try_init FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_race_once_bool_set(); std::cout << "  race_once_bool_set PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  race_once_bool_set FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  race_once_bool_set FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_race_once_bool_smoke_test(); std::cout << "  race_once_bool_smoke_test PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  race_once_bool_smoke_test FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  race_once_bool_smoke_test FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_race_once_box_once_box_default(); std::cout << "  race_once_box_once_box_default PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  race_once_box_once_box_default FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  race_once_box_once_box_default FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_race_once_box_once_box_first_wins(); std::cout << "  race_once_box_once_box_first_wins PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  race_once_box_once_box_first_wins FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  race_once_box_once_box_first_wins FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_race_once_box_once_box_reentrant(); std::cout << "  race_once_box_once_box_reentrant PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  race_once_box_once_box_reentrant FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  race_once_box_once_box_reentrant FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_race_once_box_once_box_set(); std::cout << "  race_once_box_once_box_set PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  race_once_box_once_box_set FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  race_once_box_once_box_set FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_race_once_box_once_box_smoke_test(); std::cout << "  race_once_box_once_box_smoke_test PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  race_once_box_once_box_smoke_test FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  race_once_box_once_box_smoke_test FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_race_once_box_onece_box_clone(); std::cout << "  race_once_box_onece_box_clone PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  race_once_box_onece_box_clone FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  race_once_box_onece_box_clone FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_race_once_box_onece_box_with_value(); std::cout << "  race_once_box_onece_box_with_value PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  race_once_box_onece_box_with_value FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  race_once_box_onece_box_with_value FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_race_once_non_zero_usize_first_wins(); std::cout << "  race_once_non_zero_usize_first_wins PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  race_once_non_zero_usize_first_wins FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  race_once_non_zero_usize_first_wins FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_race_once_non_zero_usize_set(); std::cout << "  race_once_non_zero_usize_set PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  race_once_non_zero_usize_set FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  race_once_non_zero_usize_set FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_race_once_non_zero_usize_smoke_test(); std::cout << "  race_once_non_zero_usize_smoke_test PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  race_once_non_zero_usize_smoke_test FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  race_once_non_zero_usize_smoke_test FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_race_once_ref_set(); std::cout << "  race_once_ref_set PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  race_once_ref_set FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  race_once_ref_set FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_race_once_ref_smoke_test(); std::cout << "  race_once_ref_smoke_test PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  race_once_ref_smoke_test FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  race_once_ref_smoke_test FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_lazy_arrrrrrrrrrrrrrrrrrrrrr(); std::cout << "  sync_lazy_arrrrrrrrrrrrrrrrrrrrrr PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_lazy_arrrrrrrrrrrrrrrrrrrrrr FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_lazy_arrrrrrrrrrrrrrrrrrrrrr FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_lazy_lazy_default(); std::cout << "  sync_lazy_lazy_default PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_lazy_lazy_default FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_lazy_lazy_default FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_lazy_lazy_deref_mut(); std::cout << "  sync_lazy_lazy_deref_mut PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_lazy_lazy_deref_mut FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_lazy_lazy_deref_mut FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_lazy_lazy_force_mut(); std::cout << "  sync_lazy_lazy_force_mut PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_lazy_lazy_force_mut FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_lazy_lazy_force_mut FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_lazy_lazy_get_mut(); std::cout << "  sync_lazy_lazy_get_mut PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_lazy_lazy_get_mut FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_lazy_lazy_get_mut FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_lazy_lazy_into_value(); std::cout << "  sync_lazy_lazy_into_value PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_lazy_lazy_into_value FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_lazy_lazy_into_value FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_lazy_lazy_is_sync_send(); std::cout << "  sync_lazy_lazy_is_sync_send PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_lazy_lazy_is_sync_send FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_lazy_lazy_is_sync_send FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_lazy_lazy_new(); std::cout << "  sync_lazy_lazy_new PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_lazy_lazy_new FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_lazy_lazy_new FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_lazy_lazy_poisoning(); std::cout << "  sync_lazy_lazy_poisoning PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_lazy_lazy_poisoning FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_lazy_lazy_poisoning FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_lazy_static_lazy(); std::cout << "  sync_lazy_static_lazy PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_lazy_static_lazy FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_lazy_static_lazy FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_lazy_static_lazy_via_fn(); std::cout << "  sync_lazy_static_lazy_via_fn PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_lazy_static_lazy_via_fn FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_lazy_static_lazy_via_fn FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_arrrrrrrrrrrrrrrrrrrrrr(); std::cout << "  sync_once_cell_arrrrrrrrrrrrrrrrrrrrrr PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_arrrrrrrrrrrrrrrrrrrrrr FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_arrrrrrrrrrrrrrrrrrrrrr FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_clone(); std::cout << "  sync_once_cell_clone PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_clone FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_clone FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_debug_impl(); std::cout << "  sync_once_cell_debug_impl PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_debug_impl FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_debug_impl FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_eval_once_macro(); std::cout << "  sync_once_cell_eval_once_macro PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_eval_once_macro FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_eval_once_macro FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_from_impl(); std::cout << "  sync_once_cell_from_impl PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_from_impl FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_from_impl FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_get_does_not_block(); std::cout << "  sync_once_cell_get_does_not_block PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_get_does_not_block FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_get_does_not_block FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_get_or_init_stress(); std::cout << "  sync_once_cell_get_or_init_stress PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_get_or_init_stress FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_get_or_init_stress FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_get_or_try_init(); std::cout << "  sync_once_cell_get_or_try_init PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_get_or_try_init FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_get_or_try_init FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_into_inner(); std::cout << "  sync_once_cell_into_inner PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_into_inner FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_into_inner FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_once_cell(); std::cout << "  sync_once_cell_once_cell PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_once_cell FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_once_cell FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_once_cell_does_not_leak_partially_constructed_boxes(); std::cout << "  sync_once_cell_once_cell_does_not_leak_partially_constructed_boxes PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_once_cell_does_not_leak_partially_constructed_boxes FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_once_cell_does_not_leak_partially_constructed_boxes FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_once_cell_drop(); std::cout << "  sync_once_cell_once_cell_drop PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_once_cell_drop FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_once_cell_drop FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_once_cell_drop_empty(); std::cout << "  sync_once_cell_once_cell_drop_empty PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_once_cell_drop_empty FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_once_cell_drop_empty FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_once_cell_get_mut(); std::cout << "  sync_once_cell_once_cell_get_mut PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_once_cell_get_mut FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_once_cell_get_mut FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_once_cell_get_unchecked(); std::cout << "  sync_once_cell_once_cell_get_unchecked PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_once_cell_get_unchecked FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_once_cell_get_unchecked FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_once_cell_is_sync_send(); std::cout << "  sync_once_cell_once_cell_is_sync_send PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_once_cell_is_sync_send FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_once_cell_is_sync_send FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_once_cell_with_value(); std::cout << "  sync_once_cell_once_cell_with_value PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_once_cell_with_value FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_once_cell_with_value FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_partialeq_impl(); std::cout << "  sync_once_cell_partialeq_impl PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_partialeq_impl FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_partialeq_impl FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_reentrant_init(); std::cout << "  sync_once_cell_reentrant_init PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_reentrant_init FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_reentrant_init FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_wait(); std::cout << "  sync_once_cell_wait PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_wait FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_wait FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_sync_once_cell_wait_panic(); std::cout << "  sync_once_cell_wait_panic PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  sync_once_cell_wait_panic FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  sync_once_cell_wait_panic FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_lazy_arrrrrrrrrrrrrrrrrrrrrr(); std::cout << "  unsync_lazy_arrrrrrrrrrrrrrrrrrrrrr PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_lazy_arrrrrrrrrrrrrrrrrrrrrr FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_lazy_arrrrrrrrrrrrrrrrrrrrrr FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_lazy_lazy_default(); std::cout << "  unsync_lazy_lazy_default PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_lazy_lazy_default FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_lazy_lazy_default FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_lazy_lazy_deref_mut(); std::cout << "  unsync_lazy_lazy_deref_mut PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_lazy_lazy_deref_mut FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_lazy_lazy_deref_mut FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_lazy_lazy_force_mut(); std::cout << "  unsync_lazy_lazy_force_mut PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_lazy_lazy_force_mut FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_lazy_lazy_force_mut FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_lazy_lazy_get_mut(); std::cout << "  unsync_lazy_lazy_get_mut PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_lazy_lazy_get_mut FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_lazy_lazy_get_mut FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_lazy_lazy_into_value(); std::cout << "  unsync_lazy_lazy_into_value PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_lazy_lazy_into_value FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_lazy_lazy_into_value FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_lazy_lazy_new(); std::cout << "  unsync_lazy_lazy_new PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_lazy_lazy_new FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_lazy_lazy_new FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_lazy_lazy_poisoning(); std::cout << "  unsync_lazy_lazy_poisoning PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_lazy_lazy_poisoning FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_lazy_lazy_poisoning FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_once_cell_aliasing_in_get(); std::cout << "  unsync_once_cell_aliasing_in_get PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_once_cell_aliasing_in_get FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_once_cell_aliasing_in_get FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_once_cell_arrrrrrrrrrrrrrrrrrrrrr(); std::cout << "  unsync_once_cell_arrrrrrrrrrrrrrrrrrrrrr PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_once_cell_arrrrrrrrrrrrrrrrrrrrrr FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_once_cell_arrrrrrrrrrrrrrrrrrrrrr FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_once_cell_clone(); std::cout << "  unsync_once_cell_clone PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_once_cell_clone FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_once_cell_clone FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_once_cell_debug_impl(); std::cout << "  unsync_once_cell_debug_impl PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_once_cell_debug_impl FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_once_cell_debug_impl FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_once_cell_from_impl(); std::cout << "  unsync_once_cell_from_impl PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_once_cell_from_impl FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_once_cell_from_impl FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_once_cell_get_or_try_init(); std::cout << "  unsync_once_cell_get_or_try_init PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_once_cell_get_or_try_init FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_once_cell_get_or_try_init FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_once_cell_into_inner(); std::cout << "  unsync_once_cell_into_inner PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_once_cell_into_inner FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_once_cell_into_inner FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_once_cell_once_cell(); std::cout << "  unsync_once_cell_once_cell PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_once_cell_once_cell FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_once_cell_once_cell FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_once_cell_once_cell_drop(); std::cout << "  unsync_once_cell_once_cell_drop PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_once_cell_once_cell_drop FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_once_cell_once_cell_drop FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_once_cell_once_cell_drop_empty(); std::cout << "  unsync_once_cell_once_cell_drop_empty PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_once_cell_once_cell_drop_empty FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_once_cell_once_cell_drop_empty FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_once_cell_once_cell_get_mut(); std::cout << "  unsync_once_cell_once_cell_get_mut PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_once_cell_once_cell_get_mut FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_once_cell_once_cell_get_mut FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_once_cell_once_cell_with_value(); std::cout << "  unsync_once_cell_once_cell_with_value PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_once_cell_once_cell_with_value FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_once_cell_once_cell_with_value FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_unsync_once_cell_partialeq_impl(); std::cout << "  unsync_once_cell_partialeq_impl PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  unsync_once_cell_partialeq_impl FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  unsync_once_cell_partialeq_impl FAILED (unknown exception)" << std::endl; fail++; }
    {
        const std::string cmd = std::string("\"") + argv[0] + "\" --rusty-single-test rusty_test_unsync_once_cell_reentrant_init";
        const int status = std::system(cmd.c_str());
        if (status != 0) { std::cout << "  unsync_once_cell_reentrant_init PASSED (expected panic)" << std::endl; pass++; }
        else { std::cerr << "  unsync_once_cell_reentrant_init FAILED: expected panic" << std::endl; fail++; }
    }
    std::cout << std::endl;
    std::cout << "Results: " << pass << " passed, " << fail << " failed" << std::endl;
    return fail > 0 ? 1 : 0;
}
