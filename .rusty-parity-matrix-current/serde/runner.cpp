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
namespace de { namespace ignored_any { struct IgnoredAny; } }
namespace de { namespace impls { enum class OsStringKind; } }
namespace de { namespace impls { namespace range { enum class Field; } } }
namespace de { namespace impls { namespace range { template<typename Idx> struct RangeVisitor; } } }
namespace de { namespace impls { namespace range_from { enum class Field; } } }
namespace de { namespace impls { namespace range_from { template<typename Idx> struct RangeFromVisitor; } } }
namespace de { namespace impls { namespace range_to { enum class Field; } } }
namespace de { namespace impls { namespace range_to { template<typename Idx> struct RangeToVisitor; } } }
namespace de { namespace impls { struct BoolVisitor; } }
namespace de { namespace impls { struct BytesVisitor; } }
namespace de { namespace impls { struct CStringVisitor; } }
namespace de { namespace impls { struct CharVisitor; } }
namespace de { namespace impls { struct OsStringVisitor; } }
namespace de { namespace impls { struct PathBufVisitor; } }
namespace de { namespace impls { struct PathVisitor; } }
namespace de { namespace impls { struct StrVisitor; } }
namespace de { namespace impls { struct StringInPlaceVisitor; } }
namespace de { namespace impls { struct StringVisitor; } }
namespace de { namespace impls { struct UnitVisitor; } }
namespace de { namespace impls { template<typename A> struct ArrayInPlaceVisitor; } }
namespace de { namespace impls { template<typename A> struct ArrayVisitor; } }
namespace de { namespace impls { template<typename T> struct FromStrVisitor; } }
namespace de { namespace impls { template<typename T> struct OptionVisitor; } }
namespace de { namespace impls { template<typename T> struct PhantomDataVisitor; } }
namespace de { namespace value { namespace private_ { template<typename A> struct MapAsEnum; } } }
namespace de { namespace value { namespace private_ { template<typename E> struct UnitOnly; } } }
namespace de { namespace value { namespace private_ { template<typename V> struct SeedStructVariant; } } }
namespace de { namespace value { namespace private_ { template<typename V> struct SeedTupleVariant; } } }
namespace de { namespace value { struct Error; } }
namespace de { namespace value { struct ExpectedInMap; } }
namespace de { namespace value { struct ExpectedInSeq; } }
namespace de { namespace value { template<typename A, typename B, typename E> struct PairDeserializer; } }
namespace de { namespace value { template<typename A, typename B, typename E> struct PairVisitor; } }
namespace de { namespace value { template<typename A> struct EnumAccessDeserializer; } }
namespace de { namespace value { template<typename A> struct MapAccessDeserializer; } }
namespace de { namespace value { template<typename A> struct SeqAccessDeserializer; } }
namespace de { namespace value { template<typename E> struct BoolDeserializer; } }
namespace de { namespace value { template<typename E> struct BorrowedBytesDeserializer; } }
namespace de { namespace value { template<typename E> struct BorrowedStrDeserializer; } }
namespace de { namespace value { template<typename E> struct BytesDeserializer; } }
namespace de { namespace value { template<typename E> struct CharDeserializer; } }
namespace de { namespace value { template<typename E> struct CowStrDeserializer; } }
namespace de { namespace value { template<typename E> struct F32Deserializer; } }
namespace de { namespace value { template<typename E> struct F64Deserializer; } }
namespace de { namespace value { template<typename E> struct I128Deserializer; } }
namespace de { namespace value { template<typename E> struct I16Deserializer; } }
namespace de { namespace value { template<typename E> struct I32Deserializer; } }
namespace de { namespace value { template<typename E> struct I64Deserializer; } }
namespace de { namespace value { template<typename E> struct I8Deserializer; } }
namespace de { namespace value { template<typename E> struct IsizeDeserializer; } }
namespace de { namespace value { template<typename E> struct StrDeserializer; } }
namespace de { namespace value { template<typename E> struct StringDeserializer; } }
namespace de { namespace value { template<typename E> struct U128Deserializer; } }
namespace de { namespace value { template<typename E> struct U16Deserializer; } }
namespace de { namespace value { template<typename E> struct U32Deserializer; } }
namespace de { namespace value { template<typename E> struct U64Deserializer; } }
namespace de { namespace value { template<typename E> struct U8Deserializer; } }
namespace de { namespace value { template<typename E> struct UnitDeserializer; } }
namespace de { namespace value { template<typename E> struct UsizeDeserializer; } }
namespace de { namespace value { template<typename I, typename E> struct MapDeserializer; } }
namespace de { namespace value { template<typename I, typename E> struct SeqDeserializer; } }
namespace de { struct OneOf; }
namespace de { struct Unexpected; }
namespace de { struct WithDecimalPoint; }
namespace format { struct Buf; }
namespace private_ { namespace content { struct Content; } }
namespace private_ { namespace de { namespace content { enum class TagContentOtherField; } } }
namespace private_ { namespace de { namespace content { enum class TagOrContentField; } } }
namespace private_ { namespace de { namespace content { struct ContentVisitor; } } }
namespace private_ { namespace de { namespace content { struct ExpectedInMap; } } }
namespace private_ { namespace de { namespace content { struct ExpectedInSeq; } } }
namespace private_ { namespace de { namespace content { struct InternallyTaggedUnitVisitor; } } }
namespace private_ { namespace de { namespace content { struct TagContentOtherFieldVisitor; } } }
namespace private_ { namespace de { namespace content { struct TagOrContentFieldVisitor; } } }
namespace private_ { namespace de { namespace content { struct TagOrContentVisitor; } } }
namespace private_ { namespace de { namespace content { struct TagOrContent_Content; } } }
namespace private_ { namespace de { namespace content { struct TagOrContent_Tag; } } }
namespace private_ { namespace de { namespace content { struct UntaggedUnitVisitor; } } }
namespace private_ { namespace de { namespace content { template<typename E> struct ContentDeserializer; } } }
namespace private_ { namespace de { namespace content { template<typename E> struct ContentRefDeserializer; } } }
namespace private_ { namespace de { namespace content { template<typename E> struct EnumDeserializer; } } }
namespace private_ { namespace de { namespace content { template<typename E> struct EnumRefDeserializer; } } }
namespace private_ { namespace de { namespace content { template<typename E> struct MapDeserializer; } } }
namespace private_ { namespace de { namespace content { template<typename E> struct MapRefDeserializer; } } }
namespace private_ { namespace de { namespace content { template<typename E> struct PairDeserializer; } } }
namespace private_ { namespace de { namespace content { template<typename E> struct PairRefDeserializer; } } }
namespace private_ { namespace de { namespace content { template<typename E> struct PairRefVisitor; } } }
namespace private_ { namespace de { namespace content { template<typename E> struct PairVisitor; } } }
namespace private_ { namespace de { namespace content { template<typename E> struct SeqDeserializer; } } }
namespace private_ { namespace de { namespace content { template<typename E> struct SeqRefDeserializer; } } }
namespace private_ { namespace de { namespace content { template<typename E> struct VariantDeserializer; } } }
namespace private_ { namespace de { namespace content { template<typename E> struct VariantRefDeserializer; } } }
namespace private_ { namespace de { namespace content { template<typename T> struct TaggedContentVisitor; } } }
namespace private_ { namespace de { template<typename E> struct BorrowedStrDeserializer; } }
namespace private_ { namespace de { template<typename E> struct FlatMapAccess; } }
namespace private_ { namespace de { template<typename E> struct FlatMapDeserializer; } }
namespace private_ { namespace de { template<typename E> struct FlatStructAccess; } }
namespace private_ { namespace de { template<typename E> struct StrDeserializer; } }
namespace private_ { namespace de { template<typename F> struct AdjacentlyTaggedEnumVariantSeed; } }
namespace private_ { namespace de { template<typename F> struct AdjacentlyTaggedEnumVariantVisitor; } }
namespace private_ { namespace de { template<typename T> struct Borrowed; } }
namespace private_ { namespace doc { struct Error; } }
namespace private_ { namespace seed { template<typename T> struct InPlaceSeed; } }
namespace private_ { namespace ser { enum class Unsupported; } }
namespace private_ { namespace ser { namespace content { struct Content; } } }
namespace private_ { namespace ser { namespace content { template<typename E> struct ContentSerializer; } } }
namespace private_ { namespace ser { namespace content { template<typename E> struct SerializeMap; } } }
namespace private_ { namespace ser { namespace content { template<typename E> struct SerializeSeq; } } }
namespace private_ { namespace ser { namespace content { template<typename E> struct SerializeStruct; } } }
namespace private_ { namespace ser { namespace content { template<typename E> struct SerializeStructVariant; } } }
namespace private_ { namespace ser { namespace content { template<typename E> struct SerializeTuple; } } }
namespace private_ { namespace ser { namespace content { template<typename E> struct SerializeTupleStruct; } } }
namespace private_ { namespace ser { namespace content { template<typename E> struct SerializeTupleVariant; } } }
namespace private_ { namespace ser { namespace content { template<typename M> struct SerializeStructVariantAsMapValue; } } }
namespace private_ { namespace ser { namespace content { template<typename M> struct SerializeTupleVariantAsMapValue; } } }
namespace private_ { namespace ser { struct AdjacentlyTaggedEnumVariant; } }
namespace private_ { namespace ser { template<typename M> struct FlatMapSerializeMap; } }
namespace private_ { namespace ser { template<typename M> struct FlatMapSerializeStruct; } }
namespace private_ { namespace ser { template<typename M> struct FlatMapSerializeStructVariantAsMapValue; } }
namespace private_ { namespace ser { template<typename M> struct FlatMapSerializeTupleVariantAsMapValue; } }
namespace private_ { namespace ser { template<typename M> struct FlatMapSerializer; } }
namespace private_ { namespace ser { template<typename S> struct TaggedSerializer; } }
namespace private_ { namespace ser { template<typename T> struct CannotSerializeVariant; } }
namespace ser { namespace impossible { enum class Void; } }
namespace ser { namespace impossible { template<typename Ok, typename Error> struct Impossible; } }

// ── from serde_core.cppm ──



#if defined(__GNUC__)
#pragma GCC diagnostic ignored "-Wunused-local-typedefs"
#endif


namespace de {}
namespace de::ignored_any {}
namespace de::impls {}
namespace de::impls::range {}
namespace de::impls::range_from {}
namespace de::impls::range_to {}
namespace de::value {}
namespace de::value::private_ {}
namespace format {}
namespace private_::doc {}
namespace private_::seed {}
namespace ser::fmt {}
namespace ser::impls {}
namespace ser::impossible {}

namespace de {}
namespace private_ {}
namespace ser {}

// UNSUPPORTED: unsupported by-value circular type dependency in scope <crate>: [Content]; cycle path: Content -> Content


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



namespace crate_root {
}
namespace macros {
}
namespace lib {
    namespace core {
    }
}
namespace format {
    struct Buf;
}
namespace ser {
    namespace impossible {
        enum class Void;
        template<typename Ok, typename Error>
        struct Impossible;
    }
    namespace fmt {
    }
    namespace impls {
        extern const std::span<const uint8_t> DEC_DIGITS_LUT;
        size_t format_u8(uint8_t n, std::span<uint8_t> out);
    }
    template<typename I>
    rusty::Option<size_t> iterator_len_hint(const I& iter);
}
namespace de {
    struct OneOf;
    struct WithDecimalPoint;
    struct Unexpected;
    namespace value {
        struct Error;
        template<typename E>
        struct UnitDeserializer;
        template<typename E>
        struct BoolDeserializer;
        template<typename E>
        struct I8Deserializer;
        template<typename E>
        struct I16Deserializer;
        template<typename E>
        struct I32Deserializer;
        template<typename E>
        struct I64Deserializer;
        template<typename E>
        struct I128Deserializer;
        template<typename E>
        struct IsizeDeserializer;
        template<typename E>
        struct U8Deserializer;
        template<typename E>
        struct U16Deserializer;
        template<typename E>
        struct U64Deserializer;
        template<typename E>
        struct U128Deserializer;
        template<typename E>
        struct UsizeDeserializer;
        template<typename E>
        struct F32Deserializer;
        template<typename E>
        struct F64Deserializer;
        template<typename E>
        struct CharDeserializer;
        template<typename E>
        struct U32Deserializer;
        template<typename E>
        struct StrDeserializer;
        template<typename E>
        struct BorrowedStrDeserializer;
        template<typename E>
        struct StringDeserializer;
        template<typename E>
        struct CowStrDeserializer;
        template<typename E>
        struct BytesDeserializer;
        template<typename E>
        struct BorrowedBytesDeserializer;
        template<typename I, typename E>
        struct SeqDeserializer;
        struct ExpectedInSeq;
        template<typename A>
        struct SeqAccessDeserializer;
        template<typename I, typename E>
        struct MapDeserializer;
        template<typename A, typename B, typename E>
        struct PairDeserializer;
        template<typename A, typename B, typename E>
        struct PairVisitor;
        struct ExpectedInMap;
        template<typename A>
        struct MapAccessDeserializer;
        template<typename A>
        struct EnumAccessDeserializer;
        namespace private_ {
            template<typename E>
            struct UnitOnly;
            template<typename A>
            struct MapAsEnum;
            template<typename V>
            struct SeedTupleVariant;
            template<typename V>
            struct SeedStructVariant;
            using ::de::Unexpected;
            template<typename T>
            using First = typename T::First;
            template<typename T>
            using Second = typename T::Second;
            template<typename T, typename E>
            std::tuple<T, UnitOnly<E>> unit_only(T t);
            template<typename A>
            MapAsEnum<A> map_as_enum(A map);
        }
        using ErrorImpl = rusty::Box<rusty::String>;
    }
    namespace ignored_any {
        struct IgnoredAny;
    }
    namespace impls {
        enum class OsStringKind;
        constexpr OsStringKind OsStringKind_Unix();
        constexpr OsStringKind OsStringKind_Windows();
        struct UnitVisitor;
        struct BoolVisitor;
        struct CharVisitor;
        struct StringVisitor;
        struct StringInPlaceVisitor;
        struct StrVisitor;
        struct BytesVisitor;
        struct CStringVisitor;
        template<typename T>
        struct OptionVisitor;
        template<typename T>
        struct PhantomDataVisitor;
        template<typename A>
        struct ArrayVisitor;
        template<typename A>
        struct ArrayInPlaceVisitor;
        struct PathVisitor;
        struct PathBufVisitor;
        struct OsStringVisitor;
        template<typename T>
        struct FromStrVisitor;
        namespace range {
            enum class Field;
            constexpr Field Field_Start();
            constexpr Field Field_End();
            template<typename Idx>
            struct RangeVisitor;
            extern const std::span<const std::string_view> FIELDS;
        }
        namespace range_from {
            enum class Field;
            constexpr Field Field_Start();
            template<typename Idx>
            struct RangeFromVisitor;
            extern const std::span<const std::string_view> FIELDS;
        }
        namespace range_to {
            enum class Field;
            constexpr Field Field_End();
            template<typename Idx>
            struct RangeToVisitor;
            extern const std::span<const std::string_view> FIELDS;
        }
        using ::de::Unexpected;
        template<typename T>
        void nop_reserve(T _seq, size_t _n);
    }
}
namespace private_ {
    namespace content {
        struct Content;
    }
    namespace seed {
        template<typename T>
        struct InPlaceSeed;
    }
    namespace doc {
        struct Error;
    }
    namespace size_hint {
        template<typename I>
        rusty::Option<size_t> from_bounds(const I& iter);
        template<typename Element>
        size_t cautious(rusty::Option<size_t> hint);
        rusty::Option<size_t> helper(std::tuple<size_t, rusty::Option<size_t>> bounds);
    }
    namespace string {
        rusty::Cow from_utf8_lossy(std::span<const uint8_t> bytes);
    }
}
namespace __private {
}
namespace __private228 {
}

namespace de {
    // Extension trait free-function forward declarations
    namespace rusty_ext {
        template<typename D, typename T>
        rusty::Result<T, typename D::Error> deserialize(rusty::PhantomData<T> self_, D deserializer);

        template<typename T>
        rusty::fmt::Result fmt(const T& self_, rusty::fmt::Formatter& formatter);

    }

}

namespace de {
    namespace impls {
        // Extension trait free-function forward declarations
        namespace rusty_ext {
        }
    }
}

namespace de {
    namespace value {
        // Extension trait free-function forward declarations
        namespace rusty_ext {
            template<typename E>
            ::de::value::BoolDeserializer<E> into_deserializer(bool self_);

            template<typename E>
            ::de::value::I8Deserializer<E> into_deserializer(int8_t self_);

            template<typename E>
            ::de::value::I16Deserializer<E> into_deserializer(int16_t self_);

            template<typename E>
            ::de::value::I32Deserializer<E> into_deserializer(int32_t self_);

            template<typename E>
            ::de::value::I64Deserializer<E> into_deserializer(int64_t self_);

            template<typename E>
            ::de::value::I128Deserializer<E> into_deserializer(__int128 self_);

            template<typename E>
            ::de::value::IsizeDeserializer<E> into_deserializer(ptrdiff_t self_);

            template<typename E>
            ::de::value::U8Deserializer<E> into_deserializer(uint8_t self_);

            template<typename E>
            ::de::value::U16Deserializer<E> into_deserializer(uint16_t self_);

            template<typename E>
            ::de::value::U64Deserializer<E> into_deserializer(uint64_t self_);

            template<typename E>
            ::de::value::U128Deserializer<E> into_deserializer(unsigned __int128 self_);

            template<typename E>
            ::de::value::UsizeDeserializer<E> into_deserializer(size_t self_);

            template<typename E>
            ::de::value::F32Deserializer<E> into_deserializer(float self_);

            template<typename E>
            ::de::value::F64Deserializer<E> into_deserializer(double self_);

            template<typename E>
            ::de::value::CharDeserializer<E> into_deserializer(char32_t self_);

            template<typename E>
            ::de::value::U32Deserializer<E> into_deserializer(uint32_t self_);

            template<typename E>
            ::de::value::StringDeserializer<E> into_deserializer(rusty::String self_);

            template<typename E>
            ::de::value::CowStrDeserializer<E> into_deserializer(rusty::Cow self_);

            template<typename T, typename E>
            ::de::value::SeqDeserializer<typename rusty::Vec<T>::IntoIter, E> into_deserializer(rusty::Vec<T> self_);

            template<typename T, typename E>
            ::de::value::SeqDeserializer<typename rusty::BTreeSet<T>::IntoIter, E> into_deserializer(rusty::BTreeSet<T> self_);

            template<typename T, typename S, typename E>
            ::de::value::SeqDeserializer<typename rusty::HashSet<T, S>::IntoIter, E> into_deserializer(rusty::HashSet<T, S> self_);

            template<typename K, typename V, typename E>
            ::de::value::MapDeserializer<typename rusty::BTreeMap<K, V>::IntoIter, E> into_deserializer(rusty::BTreeMap<K, V> self_);

            template<typename K, typename V, typename S, typename E>
            ::de::value::MapDeserializer<typename rusty::HashMap<K, V, S>::IntoIter, E> into_deserializer(rusty::HashMap<K, V, S> self_);

        }

    }
}

namespace ser {
    namespace fmt {
        // Extension trait free-function forward declarations
        namespace rusty_ext {
        }
    }
}

namespace ser {
    namespace impls {
        // Extension trait free-function forward declarations
        namespace rusty_ext {
            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const bool& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const ptrdiff_t& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const int8_t& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const int16_t& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const int32_t& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const __int128& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const size_t& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const uint8_t& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const uint16_t& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const uint32_t& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const unsigned __int128& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const float& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const double& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const char32_t& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const std::string_view& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::String& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::fmt::Arguments& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Option<T>& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::PhantomData<T>& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Vec<T>& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::BTreeSet<T>& self_, S serializer);

            template<typename S, typename T, typename H>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::HashSet<T, H>& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::VecDeque<T>& self_, S serializer);

            template<typename S, typename Idx>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::range<Idx>& self_, S serializer);

            template<typename S, typename Idx>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::range_from<Idx>& self_, S serializer);

            template<typename S, typename Idx>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::range_inclusive<Idx>& self_, S serializer);

            template<typename S, typename Idx>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::range_to<Idx>& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Bound<T>& self_, S serializer);

            template<typename S, typename K, typename V>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::BTreeMap<K, V>& self_, S serializer);

            template<typename S, typename K, typename V, typename H>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::HashMap<K, V, H>& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Box<T>& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Rc<T>& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Arc<T>& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Cow& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Weak<T>& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::Weak<T>& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroI8& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroI16& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroI32& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroI64& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroI128& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroU8& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroU16& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroU32& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroU64& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroU128& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Cell<T>& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::RefCell<T>& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Mutex<T>& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::RwLock<T>& self_, S serializer);

            template<typename S, typename T, typename E>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Result<T, E>& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::time::Duration& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::time::SystemTime& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::net::IpAddr& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::net::Ipv4Addr& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::net::Ipv6Addr& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::net::SocketAddr& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::net::SocketAddrV4& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::net::SocketAddrV6& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::path::PathBuf& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::Wrapping<T>& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::Saturating<T>& self_, S serializer);

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::cmp::Reverse<T>& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::atomic::AtomicBool& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::atomic::AtomicI8& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::atomic::AtomicI16& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::atomic::AtomicI32& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::atomic::AtomicIsize& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::atomic::AtomicU8& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::atomic::AtomicU16& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::atomic::AtomicU32& self_, S serializer);

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::atomic::AtomicUsize& self_, S serializer);

        }

    }
}



namespace crate_root {
}

namespace macros {
}

namespace lib {
    namespace core {}

    namespace core {
    }

    namespace core {

        using namespace ::std;

    }


    namespace num = rusty::num;
    namespace str = rusty::str_runtime;

    namespace cmp = rusty::cmp;
    namespace mem = rusty::mem;

    using ::rusty::Cell;
    using ::rusty::RefCell;


    namespace fmt = rusty::fmt;

    using ::rusty::PhantomData;



    namespace result = rusty;

    using ::rusty::time::Duration;

    using ::rusty::Cow;
    using ::rusty::Cow_Borrowed;
    using ::rusty::Cow_Owned;

    using ::rusty::String;

    using ::rusty::Vec;

    using ::rusty::Box;

    using ::rusty::Rc;
    template<typename... Ts> using RcWeak = rusty::Weak<Ts...>;

    using ::rusty::Arc;
    template<typename... Ts> using ArcWeak = rusty::sync::Weak<Ts...>;

    using ::rusty::BTreeMap;
    using ::rusty::BTreeSet;
    using ::rusty::VecDeque;

    using ::rusty::ffi::CStr;

    using ::rusty::ffi::CString;



    using ::rusty::HashMap;
    using ::rusty::HashSet;

    using ::rusty::ffi::OsStr;
    using ::rusty::ffi::OsString;



    using ::rusty::path::Path;

    using ::rusty::Mutex;
    using ::rusty::RwLock;


    using ::rusty::sync::atomic::Ordering;

    using ::rusty::sync::atomic::AtomicBool;
    using ::rusty::sync::atomic::AtomicI8;
    using ::rusty::sync::atomic::AtomicU8;

    using ::rusty::sync::atomic::AtomicI16;
    using ::rusty::sync::atomic::AtomicU16;

    using ::rusty::sync::atomic::AtomicI32;
    using ::rusty::sync::atomic::AtomicU32;

    using ::rusty::sync::atomic::AtomicI64;
    using ::rusty::sync::atomic::AtomicU64;

    using ::rusty::sync::atomic::AtomicIsize;
    using ::rusty::sync::atomic::AtomicUsize;


}

namespace format {

    struct Buf;

    namespace fmt = rusty::fmt;

    namespace str = rusty::str_runtime;

    struct Buf {
        std::span<uint8_t> bytes;
        size_t offset;

        static Buf new_(std::span<uint8_t> bytes);
        std::string_view as_str() const;
        rusty::fmt::Result write_str(std::string_view s);
    };

}

namespace ser {
    namespace fmt {}
    namespace impls {}
    namespace impossible {}

    namespace impossible {
        enum class Void;
        template<typename Ok, typename Error>
        struct Impossible;
    }
    namespace fmt {
    }
    namespace impls {
        extern const std::span<const uint8_t> DEC_DIGITS_LUT;
        size_t format_u8(uint8_t n, std::span<uint8_t> out);
    }
    template<typename I>
    rusty::Option<size_t> iterator_len_hint(const I& iter);

    using namespace lib;

    namespace impls {

        extern const std::span<const uint8_t> DEC_DIGITS_LUT;
        size_t format_u8(uint8_t n, std::span<uint8_t> out);

        using namespace lib;


        static const auto DEC_DIGITS_LUT_storage = std::array<uint8_t, 200>{{ 0x30, 0x30, 0x30, 0x31, 0x30, 0x32, 0x30, 0x33, 0x30, 0x34, 0x30, 0x35, 0x30, 0x36, 0x30, 0x37, 0x30, 0x38, 0x30, 0x39, 0x31, 0x30, 0x31, 0x31, 0x31, 0x32, 0x31, 0x33, 0x31, 0x34, 0x31, 0x35, 0x31, 0x36, 0x31, 0x37, 0x31, 0x38, 0x31, 0x39, 0x32, 0x30, 0x32, 0x31, 0x32, 0x32, 0x32, 0x33, 0x32, 0x34, 0x32, 0x35, 0x32, 0x36, 0x32, 0x37, 0x32, 0x38, 0x32, 0x39, 0x33, 0x30, 0x33, 0x31, 0x33, 0x32, 0x33, 0x33, 0x33, 0x34, 0x33, 0x35, 0x33, 0x36, 0x33, 0x37, 0x33, 0x38, 0x33, 0x39, 0x34, 0x30, 0x34, 0x31, 0x34, 0x32, 0x34, 0x33, 0x34, 0x34, 0x34, 0x35, 0x34, 0x36, 0x34, 0x37, 0x34, 0x38, 0x34, 0x39, 0x35, 0x30, 0x35, 0x31, 0x35, 0x32, 0x35, 0x33, 0x35, 0x34, 0x35, 0x35, 0x35, 0x36, 0x35, 0x37, 0x35, 0x38, 0x35, 0x39, 0x36, 0x30, 0x36, 0x31, 0x36, 0x32, 0x36, 0x33, 0x36, 0x34, 0x36, 0x35, 0x36, 0x36, 0x36, 0x37, 0x36, 0x38, 0x36, 0x39, 0x37, 0x30, 0x37, 0x31, 0x37, 0x32, 0x37, 0x33, 0x37, 0x34, 0x37, 0x35, 0x37, 0x36, 0x37, 0x37, 0x37, 0x38, 0x37, 0x39, 0x38, 0x30, 0x38, 0x31, 0x38, 0x32, 0x38, 0x33, 0x38, 0x34, 0x38, 0x35, 0x38, 0x36, 0x38, 0x37, 0x38, 0x38, 0x38, 0x39, 0x39, 0x30, 0x39, 0x31, 0x39, 0x32, 0x39, 0x33, 0x39, 0x34, 0x39, 0x35, 0x39, 0x36, 0x39, 0x37, 0x39, 0x38, 0x39, 0x39 }};
        const std::span<const uint8_t> DEC_DIGITS_LUT = DEC_DIGITS_LUT_storage;

    }

    namespace impossible {

        enum class Void;
        template<typename Ok, typename Error>
        struct Impossible;

        enum class Void {
            
        };

        using namespace lib;

        namespace ser = ::ser;

        /// Helper type for implementing a `Serializer` that does not support
        /// serializing one of the compound types.
        ///
        /// This type cannot be instantiated, but implements every one of the traits
        /// corresponding to the [`Serializer`] compound types: [`SerializeSeq`],
        /// [`SerializeTuple`], [`SerializeTupleStruct`], [`SerializeTupleVariant`],
        /// [`SerializeMap`], [`SerializeStruct`], and [`SerializeStructVariant`].
        ///
        /// ```edition2021
        /// # use serde::ser::{Serializer, Impossible};
        /// # use serde_core::__private::doc::Error;
        /// #
        /// # struct MySerializer;
        /// #
        /// impl Serializer for MySerializer {
        ///     type Ok = ();
        ///     type Error = Error;
        ///
        ///     type SerializeSeq = Impossible<(), Error>;
        ///     /* other associated types */
        ///
        ///     /// This data format does not support serializing sequences.
        ///     fn serialize_seq(self,
        ///                      len: Option<usize>)
        ///                      -> Result<Self::SerializeSeq, Error> {
        ///         // Given Impossible cannot be instantiated, the only
        ///         // thing we can do here is to return an error.
        /// #         stringify! {
        ///         Err(...)
        /// #         };
        /// #         unimplemented!()
        ///     }
        ///
        ///     /* other Serializer methods */
        /// #     serde_core::__serialize_unimplemented! {
        /// #         bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str bytes none some
        /// #         unit unit_struct unit_variant newtype_struct newtype_variant
        /// #         tuple tuple_struct tuple_variant map struct struct_variant
        /// #     }
        /// }
        /// ```
        ///
        /// [`Serializer`]: crate::Serializer
        /// [`SerializeSeq`]: crate::ser::SerializeSeq
        /// [`SerializeTuple`]: crate::ser::SerializeTuple
        /// [`SerializeTupleStruct`]: crate::ser::SerializeTupleStruct
        /// [`SerializeTupleVariant`]: crate::ser::SerializeTupleVariant
        /// [`SerializeMap`]: crate::ser::SerializeMap
        /// [`SerializeStruct`]: crate::ser::SerializeStruct
        /// [`SerializeStructVariant`]: crate::ser::SerializeStructVariant
        template<typename Ok, typename Error>
        struct Impossible {
            Void void_;
            rusty::PhantomData<Ok> ok;
            rusty::PhantomData<Error> error;

            template<typename T>
            rusty::Result<std::tuple<>, Error> serialize_element(const T& value) {
                static_cast<void>(value);
                return [&]() -> rusty::Result<std::tuple<>, Error> { rusty::intrinsics::unreachable(); }();
            }
            rusty::Result<Ok, Error> end() {
                return [&]() -> rusty::Result<Ok, Error> { rusty::intrinsics::unreachable(); }();
            }
            template<typename T>
            rusty::Result<std::tuple<>, Error> serialize_field(const T& value) {
                static_cast<void>(value);
                return [&]() -> rusty::Result<std::tuple<>, Error> { rusty::intrinsics::unreachable(); }();
            }
            template<typename T>
            rusty::Result<std::tuple<>, Error> serialize_key(const T& key) {
                static_cast<void>(key);
                return [&]() -> rusty::Result<std::tuple<>, Error> { rusty::intrinsics::unreachable(); }();
            }
            template<typename T>
            rusty::Result<std::tuple<>, Error> serialize_value(const T& value) {
                static_cast<void>(value);
                return [&]() -> rusty::Result<std::tuple<>, Error> { rusty::intrinsics::unreachable(); }();
            }
            template<typename T>
            rusty::Result<std::tuple<>, Error> serialize_field(std::string_view key, const T& value) {
                static_cast<void>(key);
                static_cast<void>(value);
                return [&]() -> rusty::Result<std::tuple<>, Error> { rusty::intrinsics::unreachable(); }();
            }
        };

    }

    namespace fmt {

        using namespace lib;

        using ::ser::impossible::Impossible;

    }

    using ::ser::impossible::Impossible;


    // Rust-only trait Error (Proxy facade emission skipped in module mode)

    // Rust-only trait Serialize (Proxy facade emission skipped in module mode)

    // Module-mode trait fallback for default methods on Serializer
    struct SerializerRuntimeHelper {
        static auto serialize_i128(auto self_, auto v) -> rusty::Result<typename std::remove_reference_t<decltype(self_)>::Ok, typename std::remove_reference_t<decltype(self_)>::Error> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            static_cast<void>(std::move(v));
            return rusty::Result<typename Self_::Ok, typename Self_::Error>::Err(Self_::Error::custom("i128 is not supported"));
        }
        static auto serialize_u128(auto self_, auto v) -> rusty::Result<typename std::remove_reference_t<decltype(self_)>::Ok, typename std::remove_reference_t<decltype(self_)>::Error> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            static_cast<void>(std::move(v));
            return rusty::Result<typename Self_::Ok, typename Self_::Error>::Err(Self_::Error::custom("u128 is not supported"));
        }
        static auto collect_seq(auto self_, auto iter) -> rusty::Result<typename std::remove_reference_t<decltype(self_)>::Ok, typename std::remove_reference_t<decltype(self_)>::Error> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            auto iter_shadow1 = iter.into_iter();
            auto serializer = ({ auto&& _m = self_.serialize_seq(iterator_len_hint(rusty::detail::deref_if_pointer_like(iter_shadow1))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename Self_::Ok, typename Self_::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
            {
                auto&& _m = iter_shadow1.try_for_each([&](auto&& item) { return serializer.serialize_element(rusty::detail::deref_if_pointer_like(item)); });
                bool _m_matched = false;
                if (!_m_matched) {
                    if (_m.is_ok()) {
                        auto&& _mv0 = _m.unwrap();
                        auto&& val = _mv0;
                        val;
                        _m_matched = true;
                    }
                }
                if (!_m_matched) {
                    if (_m.is_err()) {
                        auto&& _mv1 = _m.unwrap_err();
                        auto&& err = _mv1;
                        return rusty::Result<typename Self_::Ok, typename Self_::Error>::Err(std::move(err));
                        _m_matched = true;
                    }
                }
            }
            return serializer.end();
        }
        static auto collect_map(auto self_, auto iter) -> rusty::Result<typename std::remove_reference_t<decltype(self_)>::Ok, typename std::remove_reference_t<decltype(self_)>::Error> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            auto iter_shadow1 = iter.into_iter();
            auto serializer = ({ auto&& _m = self_.serialize_map(iterator_len_hint(rusty::detail::deref_if_pointer_like(iter_shadow1))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename Self_::Ok, typename Self_::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
            {
                auto&& _m = iter_shadow1.try_for_each([&](auto&& _destruct_param0) {
auto&& key = std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0));
auto&& value = std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0));
return serializer.serialize_entry(key, value);
});
                bool _m_matched = false;
                if (!_m_matched) {
                    if (_m.is_ok()) {
                        auto&& _mv0 = _m.unwrap();
                        auto&& val = _mv0;
                        val;
                        _m_matched = true;
                    }
                }
                if (!_m_matched) {
                    if (_m.is_err()) {
                        auto&& _mv1 = _m.unwrap_err();
                        auto&& err = _mv1;
                        return rusty::Result<typename Self_::Ok, typename Self_::Error>::Err(std::move(err));
                        _m_matched = true;
                    }
                }
            }
            return serializer.end();
        }
        static auto collect_str(auto self_, auto value) -> rusty::Result<typename std::remove_reference_t<decltype(self_)>::Ok, typename std::remove_reference_t<decltype(self_)>::Error> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            return self_.serialize_str(rusty::to_string(value));
        }
        static auto is_human_readable(const auto& self_) -> bool {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            return true;
        }
    };

    // Rust-only trait SerializeSeq (Proxy facade emission skipped in module mode)

    // Rust-only trait SerializeTuple (Proxy facade emission skipped in module mode)

    // Rust-only trait SerializeTupleStruct (Proxy facade emission skipped in module mode)

    // Rust-only trait SerializeTupleVariant (Proxy facade emission skipped in module mode)

    // Module-mode trait fallback for default methods on SerializeMap
    struct SerializeMapRuntimeHelper {
        static auto serialize_entry(auto& self_, auto key, auto value) -> rusty::Result<std::tuple<>, typename std::remove_reference_t<decltype(self_)>::Error> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            {
                auto&& _m = self_.serialize_key(rusty::detail::deref_if_pointer_like(key));
                bool _m_matched = false;
                if (!_m_matched) {
                    if (_m.is_ok()) {
                        auto&& _mv0 = _m.unwrap();
                        auto&& val = _mv0;
                        val;
                        _m_matched = true;
                    }
                }
                if (!_m_matched) {
                    if (_m.is_err()) {
                        auto&& _mv1 = _m.unwrap_err();
                        auto&& err = _mv1;
                        return rusty::Result<std::tuple<>, typename Self_::Error>::Err(std::move(err));
                        _m_matched = true;
                    }
                }
            }
            return self_.serialize_value(rusty::detail::deref_if_pointer_like(value));
        }
    };

    // Module-mode trait fallback for default methods on SerializeStruct
    struct SerializeStructRuntimeHelper {
        static auto skip_field(auto& self_, auto key) -> rusty::Result<std::tuple<>, typename std::remove_reference_t<decltype(self_)>::Error> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            static_cast<void>(key);
            return rusty::Result<std::tuple<>, typename Self_::Error>::Ok(std::make_tuple());
        }
    };

    // Module-mode trait fallback for default methods on SerializeStructVariant
    struct SerializeStructVariantRuntimeHelper {
        static auto skip_field(auto& self_, auto key) -> rusty::Result<std::tuple<>, typename std::remove_reference_t<decltype(self_)>::Error> {
            using Self_ = std::remove_reference_t<decltype(self_)>;
            static_cast<void>(key);
            return rusty::Result<std::tuple<>, typename Self_::Error>::Ok(std::make_tuple());
        }
    };

}

namespace private_ {
    namespace content {}
    namespace doc {}
    namespace seed {}
    namespace size_hint {}
    namespace string {}

    namespace content {
        struct Content;
    }
    namespace seed {
        template<typename T>
        struct InPlaceSeed;
    }
    namespace doc {
        struct Error;
    }
    namespace size_hint {
        template<typename I>
        rusty::Option<size_t> from_bounds(const I& iter);
        template<typename Element>
        size_t cautious(rusty::Option<size_t> hint);
        rusty::Option<size_t> helper(std::tuple<size_t, rusty::Option<size_t>> bounds);
    }
    namespace string {
        rusty::Cow from_utf8_lossy(std::span<const uint8_t> bytes);
    }

    using ::rusty::Result;

    namespace content {

        struct Content;

        using namespace lib;

        struct Content;  // forward declaration for recursion
        // Algebraic data type
        struct Content_Bool {
            bool _0;
        };
        struct Content_U8 {
            uint8_t _0;
        };
        struct Content_U16 {
            uint16_t _0;
        };
        struct Content_U32 {
            uint32_t _0;
        };
        struct Content_U64 {
            uint64_t _0;
        };
        struct Content_I8 {
            int8_t _0;
        };
        struct Content_I16 {
            int16_t _0;
        };
        struct Content_I32 {
            int32_t _0;
        };
        struct Content_I64 {
            int64_t _0;
        };
        struct Content_F32 {
            float _0;
        };
        struct Content_F64 {
            double _0;
        };
        struct Content_Char {
            char32_t _0;
        };
        struct Content_String {
            rusty::String _0;
        };
        struct Content_Str {
            std::string_view _0;
        };
        struct Content_ByteBuf {
            rusty::Vec<uint8_t> _0;
        };
        struct Content_Bytes {
            std::span<const uint8_t> _0;
        };
        struct Content_None {};
        struct Content_Some {
            rusty::Box<Content> _0;
        };
        struct Content_Unit {};
        struct Content_Newtype {
            rusty::Box<Content> _0;
        };
        struct Content_Seq {
            rusty::Vec<Content> _0;
        };
        struct Content_Map {
            rusty::Vec<std::tuple<Content, Content>> _0;
        };
        Content_Bool Bool(bool _0);
        Content_U8 U8(uint8_t _0);
        Content_U16 U16(uint16_t _0);
        Content_U32 U32(uint32_t _0);
        Content_U64 U64(uint64_t _0);
        Content_I8 I8(int8_t _0);
        Content_I16 I16(int16_t _0);
        Content_I32 I32(int32_t _0);
        Content_I64 I64(int64_t _0);
        Content_F32 F32(float _0);
        Content_F64 F64(double _0);
        Content_Char Char(char32_t _0);
        Content_String String(rusty::String _0);
        Content_Str Str(std::string_view _0);
        Content_ByteBuf ByteBuf(rusty::Vec<uint8_t> _0);
        Content_Bytes Bytes(std::span<const uint8_t> _0);
        Content_None None();
        Content_Some Some(rusty::Box<Content> _0);
        Content_Unit Unit();
        Content_Newtype Newtype(rusty::Box<Content> _0);
        Content_Seq Seq(rusty::Vec<Content> _0);
        Content_Map Map(rusty::Vec<std::tuple<Content, Content>> _0);
        struct Content : std::variant<Content_Bool, Content_U8, Content_U16, Content_U32, Content_U64, Content_I8, Content_I16, Content_I32, Content_I64, Content_F32, Content_F64, Content_Char, Content_String, Content_Str, Content_ByteBuf, Content_Bytes, Content_None, Content_Some, Content_Unit, Content_Newtype, Content_Seq, Content_Map> {
            using variant = std::variant<Content_Bool, Content_U8, Content_U16, Content_U32, Content_U64, Content_I8, Content_I16, Content_I32, Content_I64, Content_F32, Content_F64, Content_Char, Content_String, Content_Str, Content_ByteBuf, Content_Bytes, Content_None, Content_Some, Content_Unit, Content_Newtype, Content_Seq, Content_Map>;
            using variant::variant;
            static Content Bool(bool _0) { return Content{Content_Bool{std::forward<decltype(_0)>(_0)}}; }
            static Content U8(uint8_t _0) { return Content{Content_U8{std::forward<decltype(_0)>(_0)}}; }
            static Content U16(uint16_t _0) { return Content{Content_U16{std::forward<decltype(_0)>(_0)}}; }
            static Content U32(uint32_t _0) { return Content{Content_U32{std::forward<decltype(_0)>(_0)}}; }
            static Content U64(uint64_t _0) { return Content{Content_U64{std::forward<decltype(_0)>(_0)}}; }
            static Content I8(int8_t _0) { return Content{Content_I8{std::forward<decltype(_0)>(_0)}}; }
            static Content I16(int16_t _0) { return Content{Content_I16{std::forward<decltype(_0)>(_0)}}; }
            static Content I32(int32_t _0) { return Content{Content_I32{std::forward<decltype(_0)>(_0)}}; }
            static Content I64(int64_t _0) { return Content{Content_I64{std::forward<decltype(_0)>(_0)}}; }
            static Content F32(float _0) { return Content{Content_F32{std::forward<decltype(_0)>(_0)}}; }
            static Content F64(double _0) { return Content{Content_F64{std::forward<decltype(_0)>(_0)}}; }
            static Content Char(char32_t _0) { return Content{Content_Char{std::forward<decltype(_0)>(_0)}}; }
            static Content String(rusty::String _0) { return Content{Content_String{std::forward<decltype(_0)>(_0)}}; }
            static Content Str(std::string_view _0) { return Content{Content_Str{std::forward<decltype(_0)>(_0)}}; }
            static Content ByteBuf(rusty::Vec<uint8_t> _0) { return Content{Content_ByteBuf{std::forward<decltype(_0)>(_0)}}; }
            static Content Bytes(std::span<const uint8_t> _0) { return Content{Content_Bytes{std::forward<decltype(_0)>(_0)}}; }
            static Content None() { return Content{Content_None{}}; }
            static Content Some(rusty::Box<Content> _0) { return Content{Content_Some{std::forward<decltype(_0)>(_0)}}; }
            static Content Unit() { return Content{Content_Unit{}}; }
            static Content Newtype(rusty::Box<Content> _0) { return Content{Content_Newtype{std::forward<decltype(_0)>(_0)}}; }
            static Content Seq(rusty::Vec<Content> _0) { return Content{Content_Seq{std::forward<decltype(_0)>(_0)}}; }
            static Content Map(rusty::Vec<std::tuple<Content, Content>> _0) { return Content{Content_Map{std::forward<decltype(_0)>(_0)}}; }

        };
        Content_Bool Bool(bool _0) { return Content_Bool{std::forward<bool>(_0)};  }
        Content_U8 U8(uint8_t _0) { return Content_U8{std::forward<uint8_t>(_0)};  }
        Content_U16 U16(uint16_t _0) { return Content_U16{std::forward<uint16_t>(_0)};  }
        Content_U32 U32(uint32_t _0) { return Content_U32{std::forward<uint32_t>(_0)};  }
        Content_U64 U64(uint64_t _0) { return Content_U64{std::forward<uint64_t>(_0)};  }
        Content_I8 I8(int8_t _0) { return Content_I8{std::forward<int8_t>(_0)};  }
        Content_I16 I16(int16_t _0) { return Content_I16{std::forward<int16_t>(_0)};  }
        Content_I32 I32(int32_t _0) { return Content_I32{std::forward<int32_t>(_0)};  }
        Content_I64 I64(int64_t _0) { return Content_I64{std::forward<int64_t>(_0)};  }
        Content_F32 F32(float _0) { return Content_F32{std::forward<float>(_0)};  }
        Content_F64 F64(double _0) { return Content_F64{std::forward<double>(_0)};  }
        Content_Char Char(char32_t _0) { return Content_Char{std::forward<char32_t>(_0)};  }
        Content_String String(rusty::String _0) { return Content_String{std::forward<rusty::String>(_0)};  }
        Content_Str Str(std::string_view _0) { return Content_Str{std::forward<std::string_view>(_0)};  }
        Content_ByteBuf ByteBuf(rusty::Vec<uint8_t> _0) { return Content_ByteBuf{std::forward<rusty::Vec<uint8_t>>(_0)};  }
        Content_Bytes Bytes(std::span<const uint8_t> _0) { return Content_Bytes{std::forward<std::span<const uint8_t>>(_0)};  }
        Content_None None() { return Content_None{};  }
        Content_Some Some(rusty::Box<Content> _0) { return Content_Some{std::forward<rusty::Box<Content>>(_0)};  }
        Content_Unit Unit() { return Content_Unit{};  }
        Content_Newtype Newtype(rusty::Box<Content> _0) { return Content_Newtype{std::forward<rusty::Box<Content>>(_0)};  }
        Content_Seq Seq(rusty::Vec<Content> _0) { return Content_Seq{std::forward<rusty::Vec<Content>>(_0)};  }
        Content_Map Map(rusty::Vec<std::tuple<Content, Content>> _0) { return Content_Map{std::forward<rusty::Vec<std::tuple<Content, Content>>>(_0)};  }

    }

    namespace seed {

        template<typename T>
        struct InPlaceSeed;


        /// A DeserializeSeed helper for implementing deserialize_in_place Visitors.
        ///
        /// Wraps a mutable reference and calls deserialize_in_place on it.
        template<typename T>
        struct InPlaceSeed {
            using Value = std::conditional_t<true, std::tuple<>, T>;
            T& _0;

            template<typename D>
            rusty::Result<Value, typename D::Error> deserialize(D deserializer) {
                return T::deserialize_in_place(std::move(deserializer), std::move(this->_0));
            }
        };

    }

    namespace doc {

        struct Error;

        using namespace lib;

        namespace ser = ::ser;

        struct Error {

            rusty::fmt::Result fmt(rusty::fmt::Formatter& _arg1) const;
            template<typename T>
            static Error custom(T _arg0);
            std::string_view description() const;
        };

    }

    using ::private_::content::Content;
    namespace private_::content {}
    using namespace private_::content;

    using ::private_::seed::InPlaceSeed;

    namespace size_hint {

        template<typename I>
        rusty::Option<size_t> from_bounds(const I& iter);
        template<typename Element>
        size_t cautious(rusty::Option<size_t> hint);
        rusty::Option<size_t> helper(std::tuple<size_t, rusty::Option<size_t>> bounds);

        using namespace lib;

        template<typename I>
        rusty::Option<size_t> from_bounds(const I& iter) {
            return helper(iter.size_hint());
        }

        template<typename Element>
        size_t cautious(rusty::Option<size_t> hint) {
            constexpr size_t MAX_PREALLOC_BYTES = 1024 * 1024;
            if (rusty::mem::size_of<Element>() == 0) {
                return static_cast<size_t>(0);
            } else {
                return rusty::cmp::min(hint.unwrap_or(0), MAX_PREALLOC_BYTES / rusty::mem::size_of<Element>());
            }
        }

        rusty::Option<size_t> helper(std::tuple<size_t, rusty::Option<size_t>> bounds) {
            return [&]() -> rusty::Option<size_t> { auto&& _m_tuple = bounds; auto&& _m0 = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)); auto&& _m1 = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)); if (_m1.is_some()) { auto&& lower = _m0; auto&& upper = std::as_const(_m1).unwrap(); if (lower == upper) { return rusty::Option<size_t>(upper); } } if (true) { return rusty::Option<size_t>(rusty::None); } return [&]() -> rusty::Option<size_t> { rusty::intrinsics::unreachable(); }(); }();
        }

    }

    namespace string {

        rusty::Cow from_utf8_lossy(std::span<const uint8_t> bytes);

        using namespace lib;

        rusty::Cow from_utf8_lossy(std::span<const uint8_t> bytes) {
            return rusty::Cow_Owned(rusty::String::from_utf8_lossy(bytes));
        }

    }

}



namespace __private {

    namespace doc = ::private_::doc;

    using ::rusty::Result;

}

namespace de {
    namespace ignored_any {}
    namespace impls {}
    namespace value {}

    struct OneOf;
    struct WithDecimalPoint;
    struct Unexpected;
    namespace value {
        struct Error;
        template<typename E>
        struct UnitDeserializer;
        template<typename E>
        struct BoolDeserializer;
        template<typename E>
        struct I8Deserializer;
        template<typename E>
        struct I16Deserializer;
        template<typename E>
        struct I32Deserializer;
        template<typename E>
        struct I64Deserializer;
        template<typename E>
        struct I128Deserializer;
        template<typename E>
        struct IsizeDeserializer;
        template<typename E>
        struct U8Deserializer;
        template<typename E>
        struct U16Deserializer;
        template<typename E>
        struct U64Deserializer;
        template<typename E>
        struct U128Deserializer;
        template<typename E>
        struct UsizeDeserializer;
        template<typename E>
        struct F32Deserializer;
        template<typename E>
        struct F64Deserializer;
        template<typename E>
        struct CharDeserializer;
        template<typename E>
        struct U32Deserializer;
        template<typename E>
        struct StrDeserializer;
        template<typename E>
        struct BorrowedStrDeserializer;
        template<typename E>
        struct StringDeserializer;
        template<typename E>
        struct CowStrDeserializer;
        template<typename E>
        struct BytesDeserializer;
        template<typename E>
        struct BorrowedBytesDeserializer;
        template<typename I, typename E>
        struct SeqDeserializer;
        struct ExpectedInSeq;
        template<typename A>
        struct SeqAccessDeserializer;
        template<typename I, typename E>
        struct MapDeserializer;
        template<typename A, typename B, typename E>
        struct PairDeserializer;
        template<typename A, typename B, typename E>
        struct PairVisitor;
        struct ExpectedInMap;
        template<typename A>
        struct MapAccessDeserializer;
        template<typename A>
        struct EnumAccessDeserializer;
        namespace private_ {
            template<typename E>
            struct UnitOnly;
            template<typename A>
            struct MapAsEnum;
            template<typename V>
            struct SeedTupleVariant;
            template<typename V>
            struct SeedStructVariant;
            using ::de::Unexpected;
            template<typename T, typename E>
            std::tuple<T, UnitOnly<E>> unit_only(T t);
            template<typename A>
            MapAsEnum<A> map_as_enum(A map);
        }
    }
    namespace ignored_any {
        struct IgnoredAny;
    }
    namespace impls {
        enum class OsStringKind;
        constexpr OsStringKind OsStringKind_Unix();
        constexpr OsStringKind OsStringKind_Windows();
        struct UnitVisitor;
        struct BoolVisitor;
        struct CharVisitor;
        struct StringVisitor;
        struct StringInPlaceVisitor;
        struct StrVisitor;
        struct BytesVisitor;
        struct CStringVisitor;
        template<typename T>
        struct OptionVisitor;
        template<typename T>
        struct PhantomDataVisitor;
        template<typename A>
        struct ArrayVisitor;
        template<typename A>
        struct ArrayInPlaceVisitor;
        struct PathVisitor;
        struct PathBufVisitor;
        struct OsStringVisitor;
        template<typename T>
        struct FromStrVisitor;
        namespace range {
            enum class Field;
            constexpr Field Field_Start();
            constexpr Field Field_End();
            template<typename Idx>
            struct RangeVisitor;
            extern const std::span<const std::string_view> FIELDS;
        }
        namespace range_from {
            enum class Field;
            constexpr Field Field_Start();
            template<typename Idx>
            struct RangeFromVisitor;
            extern const std::span<const std::string_view> FIELDS;
        }
        namespace range_to {
            enum class Field;
            constexpr Field Field_End();
            template<typename Idx>
            struct RangeToVisitor;
            extern const std::span<const std::string_view> FIELDS;
        }
        using ::de::Unexpected;
        template<typename T>
        void nop_reserve(T _seq, size_t _n);
    }

    using namespace lib;

    using ::de::ignored_any::IgnoredAny;


    // Rust-only trait Error (Proxy facade emission skipped in module mode)

    /// Used in error messages.
    ///
    /// - expected `a`
    /// - expected `a` or `b`
    /// - expected one of `a`, `b`, `c`
    ///
    /// The slice of names must not be empty.
    struct OneOf {
        std::span<const std::string_view> names;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const;
    };


    // Rust-only trait Deserialize (Proxy facade emission skipped in module mode)



    // Rust-only trait Deserializer (Proxy facade emission skipped in module mode)

    // Rust-only trait Visitor (Proxy facade emission skipped in module mode)

    // Rust-only trait SeqAccess (Proxy facade emission skipped in module mode)

    // Rust-only trait MapAccess (Proxy facade emission skipped in module mode)

    // Rust-only trait EnumAccess (Proxy facade emission skipped in module mode)

    // Rust-only trait VariantAccess (Proxy facade emission skipped in module mode)

    // Rust-only trait IntoDeserializer (Proxy facade emission skipped in module mode)

    struct WithDecimalPoint {
        double _0;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const;
    };

    // Algebraic data type
    struct Unexpected_Bool {
        bool _0;
    };
    struct Unexpected_Unsigned {
        uint64_t _0;
    };
    struct Unexpected_Signed {
        int64_t _0;
    };
    struct Unexpected_Float {
        double _0;
    };
    struct Unexpected_Char {
        char32_t _0;
    };
    struct Unexpected_Str {
        std::string_view _0;
    };
    struct Unexpected_Bytes {
        std::span<const uint8_t> _0;
    };
    struct Unexpected_Unit {};
    struct Unexpected_Option {};
    struct Unexpected_NewtypeStruct {};
    struct Unexpected_Seq {};
    struct Unexpected_Map {};
    struct Unexpected_Enum {};
    struct Unexpected_UnitVariant {};
    struct Unexpected_NewtypeVariant {};
    struct Unexpected_TupleVariant {};
    struct Unexpected_StructVariant {};
    struct Unexpected_Other {
        std::string_view _0;
    };
    Unexpected_Bool Bool(bool _0);
    Unexpected_Unsigned Unsigned(uint64_t _0);
    Unexpected_Signed Signed(int64_t _0);
    Unexpected_Float Float(double _0);
    Unexpected_Char Char(char32_t _0);
    Unexpected_Str Str(std::string_view _0);
    Unexpected_Bytes Bytes(std::span<const uint8_t> _0);
    Unexpected_Unit Unit();
    Unexpected_Option Option();
    Unexpected_NewtypeStruct NewtypeStruct();
    Unexpected_Seq Seq();
    Unexpected_Map Map();
    Unexpected_Enum Enum();
    Unexpected_UnitVariant UnitVariant();
    Unexpected_NewtypeVariant NewtypeVariant();
    Unexpected_TupleVariant TupleVariant();
    Unexpected_StructVariant StructVariant();
    Unexpected_Other Other(std::string_view _0);
    struct Unexpected : std::variant<Unexpected_Bool, Unexpected_Unsigned, Unexpected_Signed, Unexpected_Float, Unexpected_Char, Unexpected_Str, Unexpected_Bytes, Unexpected_Unit, Unexpected_Option, Unexpected_NewtypeStruct, Unexpected_Seq, Unexpected_Map, Unexpected_Enum, Unexpected_UnitVariant, Unexpected_NewtypeVariant, Unexpected_TupleVariant, Unexpected_StructVariant, Unexpected_Other> {
        using variant = std::variant<Unexpected_Bool, Unexpected_Unsigned, Unexpected_Signed, Unexpected_Float, Unexpected_Char, Unexpected_Str, Unexpected_Bytes, Unexpected_Unit, Unexpected_Option, Unexpected_NewtypeStruct, Unexpected_Seq, Unexpected_Map, Unexpected_Enum, Unexpected_UnitVariant, Unexpected_NewtypeVariant, Unexpected_TupleVariant, Unexpected_StructVariant, Unexpected_Other>;
        using variant::variant;
        static Unexpected Bool(bool _0) { return Unexpected{Unexpected_Bool{std::forward<decltype(_0)>(_0)}}; }
        static Unexpected Unsigned(uint64_t _0) { return Unexpected{Unexpected_Unsigned{std::forward<decltype(_0)>(_0)}}; }
        static Unexpected Signed(int64_t _0) { return Unexpected{Unexpected_Signed{std::forward<decltype(_0)>(_0)}}; }
        static Unexpected Float(double _0) { return Unexpected{Unexpected_Float{std::forward<decltype(_0)>(_0)}}; }
        static Unexpected Char(char32_t _0) { return Unexpected{Unexpected_Char{std::forward<decltype(_0)>(_0)}}; }
        static Unexpected Str(std::string_view _0) { return Unexpected{Unexpected_Str{std::forward<decltype(_0)>(_0)}}; }
        static Unexpected Bytes(std::span<const uint8_t> _0) { return Unexpected{Unexpected_Bytes{std::forward<decltype(_0)>(_0)}}; }
        static Unexpected Unit() { return Unexpected{Unexpected_Unit{}}; }
        static Unexpected Option() { return Unexpected{Unexpected_Option{}}; }
        static Unexpected NewtypeStruct() { return Unexpected{Unexpected_NewtypeStruct{}}; }
        static Unexpected Seq() { return Unexpected{Unexpected_Seq{}}; }
        static Unexpected Map() { return Unexpected{Unexpected_Map{}}; }
        static Unexpected Enum() { return Unexpected{Unexpected_Enum{}}; }
        static Unexpected UnitVariant() { return Unexpected{Unexpected_UnitVariant{}}; }
        static Unexpected NewtypeVariant() { return Unexpected{Unexpected_NewtypeVariant{}}; }
        static Unexpected TupleVariant() { return Unexpected{Unexpected_TupleVariant{}}; }
        static Unexpected StructVariant() { return Unexpected{Unexpected_StructVariant{}}; }
        static Unexpected Other(std::string_view _0) { return Unexpected{Unexpected_Other{std::forward<decltype(_0)>(_0)}}; }


        Unexpected clone() const;
        bool operator==(const Unexpected& other) const;
        rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const;
    };
    Unexpected_Bool Bool(bool _0) { return Unexpected_Bool{std::forward<bool>(_0)};  }
    Unexpected_Unsigned Unsigned(uint64_t _0) { return Unexpected_Unsigned{std::forward<uint64_t>(_0)};  }
    Unexpected_Signed Signed(int64_t _0) { return Unexpected_Signed{std::forward<int64_t>(_0)};  }
    Unexpected_Float Float(double _0) { return Unexpected_Float{std::forward<double>(_0)};  }
    Unexpected_Char Char(char32_t _0) { return Unexpected_Char{std::forward<char32_t>(_0)};  }
    Unexpected_Str Str(std::string_view _0) { return Unexpected_Str{std::forward<std::string_view>(_0)};  }
    Unexpected_Bytes Bytes(std::span<const uint8_t> _0) { return Unexpected_Bytes{std::forward<std::span<const uint8_t>>(_0)};  }
    Unexpected_Unit Unit() { return Unexpected_Unit{};  }
    Unexpected_Option Option() { return Unexpected_Option{};  }
    Unexpected_NewtypeStruct NewtypeStruct() { return Unexpected_NewtypeStruct{};  }
    Unexpected_Seq Seq() { return Unexpected_Seq{};  }
    Unexpected_Map Map() { return Unexpected_Map{};  }
    Unexpected_Enum Enum() { return Unexpected_Enum{};  }
    Unexpected_UnitVariant UnitVariant() { return Unexpected_UnitVariant{};  }
    Unexpected_NewtypeVariant NewtypeVariant() { return Unexpected_NewtypeVariant{};  }
    Unexpected_TupleVariant TupleVariant() { return Unexpected_TupleVariant{};  }
    Unexpected_StructVariant StructVariant() { return Unexpected_StructVariant{};  }
    Unexpected_Other Other(std::string_view _0) { return Unexpected_Other{std::forward<std::string_view>(_0)};  }

    namespace value {
        namespace private_ {}

        struct Error;
        template<typename E>
        struct UnitDeserializer;
        template<typename E>
        struct BoolDeserializer;
        template<typename E>
        struct I8Deserializer;
        template<typename E>
        struct I16Deserializer;
        template<typename E>
        struct I32Deserializer;
        template<typename E>
        struct I64Deserializer;
        template<typename E>
        struct I128Deserializer;
        template<typename E>
        struct IsizeDeserializer;
        template<typename E>
        struct U8Deserializer;
        template<typename E>
        struct U16Deserializer;
        template<typename E>
        struct U64Deserializer;
        template<typename E>
        struct U128Deserializer;
        template<typename E>
        struct UsizeDeserializer;
        template<typename E>
        struct F32Deserializer;
        template<typename E>
        struct F64Deserializer;
        template<typename E>
        struct CharDeserializer;
        template<typename E>
        struct U32Deserializer;
        template<typename E>
        struct StrDeserializer;
        template<typename E>
        struct BorrowedStrDeserializer;
        template<typename E>
        struct StringDeserializer;
        template<typename E>
        struct CowStrDeserializer;
        template<typename E>
        struct BytesDeserializer;
        template<typename E>
        struct BorrowedBytesDeserializer;
        template<typename I, typename E>
        struct SeqDeserializer;
        struct ExpectedInSeq;
        template<typename A>
        struct SeqAccessDeserializer;
        template<typename I, typename E>
        struct MapDeserializer;
        template<typename A, typename B, typename E>
        struct PairDeserializer;
        template<typename A, typename B, typename E>
        struct PairVisitor;
        struct ExpectedInMap;
        template<typename A>
        struct MapAccessDeserializer;
        template<typename A>
        struct EnumAccessDeserializer;
        namespace private_ {
            template<typename E>
            struct UnitOnly;
            template<typename A>
            struct MapAsEnum;
            template<typename V>
            struct SeedTupleVariant;
            template<typename V>
            struct SeedStructVariant;
            using ::de::Unexpected;
            template<typename T, typename E>
            std::tuple<T, UnitOnly<E>> unit_only(T t);
            template<typename A>
            MapAsEnum<A> map_as_enum(A map);
        }

        using namespace lib;

        namespace de = ::de;

        namespace size_hint = ::private_::size_hint;

        namespace ser = ::ser;

        using ::de::value::private_::First;
        using ::de::value::private_::Second;

        /// A minimal representation of all possible errors that can occur using the
        /// `IntoDeserializer` trait.
        struct Error {
            ErrorImpl err;

            Error clone() const;
            bool operator==(const Error& other) const;
            template<typename T>
            static Error custom(T msg);
            static Error invalid_type(::de::Unexpected unexp, const auto& exp);
            static Error invalid_value(::de::Unexpected unexp, const auto& exp);
            static Error invalid_length(size_t len, const auto& exp);
            static Error unknown_variant(std::string_view variant, std::span<const std::string_view> expected);
            static Error unknown_field(std::string_view field, std::span<const std::string_view> expected);
            static Error missing_field(std::string_view field);
            static Error duplicate_field(std::string_view field);
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const;
            std::string_view description() const;
        };


        /// A deserializer holding a `()`.
        template<typename E>
        struct UnitDeserializer {
            using Error = E;
            using Deserializer = UnitDeserializer<E>;
            rusty::PhantomData<E> marker;

            UnitDeserializer<E> clone() const {
                return {.marker = rusty::clone(this->marker)};
            }
            static UnitDeserializer<E> new_() {
                return UnitDeserializer<E>{.marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UnitDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_unit();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_option(V visitor) {
                return visitor.visit_none();
            }
            UnitDeserializer<E> into_deserializer() {
                return std::move((*this));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("UnitDeserializer").finish();
            }
        };

        ///A deserializer holding
        ///a `bool`.
        template<typename E>
        struct BoolDeserializer {
            using Error = E;
            using Deserializer = BoolDeserializer<E>;
            bool value;
            rusty::PhantomData<E> marker;

            BoolDeserializer<E> clone() const {
                return {.value = rusty::clone(this->value), .marker = rusty::clone(this->marker)};
            }
            static BoolDeserializer<E> new_(bool value) {
                return BoolDeserializer<E>{.value = std::move(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BoolDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_bool(std::move(this->value));
            }
            BoolDeserializer<E> into_deserializer() {
                return std::move((*this));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("BoolDeserializer").field("value", &this->value).finish();
            }
        };

        ///A deserializer holding
        ///an `i8`.
        template<typename E>
        struct I8Deserializer {
            using Error = E;
            using Deserializer = I8Deserializer<E>;
            int8_t value;
            rusty::PhantomData<E> marker;

            I8Deserializer<E> clone() const {
                return {.value = rusty::clone(this->value), .marker = rusty::clone(this->marker)};
            }
            static I8Deserializer<E> new_(int8_t value) {
                return I8Deserializer<E>{.value = std::move(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I8Deserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_i8(std::move(this->value));
            }
            I8Deserializer<E> into_deserializer() {
                return std::move((*this));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("I8Deserializer").field("value", &this->value).finish();
            }
        };

        ///A deserializer holding
        ///an `i16`.
        template<typename E>
        struct I16Deserializer {
            using Error = E;
            using Deserializer = I16Deserializer<E>;
            int16_t value;
            rusty::PhantomData<E> marker;

            I16Deserializer<E> clone() const {
                return {.value = rusty::clone(this->value), .marker = rusty::clone(this->marker)};
            }
            static I16Deserializer<E> new_(int16_t value) {
                return I16Deserializer<E>{.value = std::move(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I16Deserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_i16(std::move(this->value));
            }
            I16Deserializer<E> into_deserializer() {
                return std::move((*this));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("I16Deserializer").field("value", &this->value).finish();
            }
        };

        ///A deserializer holding
        ///an `i32`.
        template<typename E>
        struct I32Deserializer {
            using Error = E;
            using Deserializer = I32Deserializer<E>;
            int32_t value;
            rusty::PhantomData<E> marker;

            I32Deserializer<E> clone() const {
                return {.value = rusty::clone(this->value), .marker = rusty::clone(this->marker)};
            }
            static I32Deserializer<E> new_(int32_t value) {
                return I32Deserializer<E>{.value = std::move(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I32Deserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_i32(std::move(this->value));
            }
            I32Deserializer<E> into_deserializer() {
                return std::move((*this));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("I32Deserializer").field("value", &this->value).finish();
            }
        };

        ///A deserializer holding
        ///an `i64`.
        template<typename E>
        struct I64Deserializer {
            using Error = E;
            using Deserializer = I64Deserializer<E>;
            int64_t value;
            rusty::PhantomData<E> marker;

            I64Deserializer<E> clone() const {
                return {.value = rusty::clone(this->value), .marker = rusty::clone(this->marker)};
            }
            static I64Deserializer<E> new_(int64_t value) {
                return I64Deserializer<E>{.value = std::move(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I64Deserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_i64(std::move(this->value));
            }
            I64Deserializer<E> into_deserializer() {
                return std::move((*this));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("I64Deserializer").field("value", &this->value).finish();
            }
        };

        ///A deserializer holding
        ///an `i128`.
        template<typename E>
        struct I128Deserializer {
            using Error = E;
            using Deserializer = I128Deserializer<E>;
            __int128 value;
            rusty::PhantomData<E> marker;

            I128Deserializer<E> clone() const {
                return {.value = rusty::clone(this->value), .marker = rusty::clone(this->marker)};
            }
            static I128Deserializer<E> new_(__int128 value) {
                return I128Deserializer<E>{.value = std::move(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename I128Deserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_i128(std::move(this->value));
            }
            I128Deserializer<E> into_deserializer() {
                return std::move((*this));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("I128Deserializer").field("value", &this->value).finish();
            }
        };

        ///A deserializer holding
        ///an `isize`.
        template<typename E>
        struct IsizeDeserializer {
            using Error = E;
            using Deserializer = IsizeDeserializer<E>;
            ptrdiff_t value;
            rusty::PhantomData<E> marker;

            IsizeDeserializer<E> clone() const {
                return {.value = rusty::clone(this->value), .marker = rusty::clone(this->marker)};
            }
            static IsizeDeserializer<E> new_(ptrdiff_t value) {
                return IsizeDeserializer<E>{.value = std::move(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename IsizeDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_i64(static_cast<int64_t>(this->value));
            }
            IsizeDeserializer<E> into_deserializer() {
                return std::move((*this));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("IsizeDeserializer").field("value", &this->value).finish();
            }
        };

        ///A deserializer holding
        ///a `u8`.
        template<typename E>
        struct U8Deserializer {
            using Error = E;
            using Deserializer = U8Deserializer<E>;
            uint8_t value;
            rusty::PhantomData<E> marker;

            U8Deserializer<E> clone() const {
                return {.value = rusty::clone(this->value), .marker = rusty::clone(this->marker)};
            }
            static U8Deserializer<E> new_(uint8_t value) {
                return U8Deserializer<E>{.value = std::move(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U8Deserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_u8(std::move(this->value));
            }
            U8Deserializer<E> into_deserializer() {
                return std::move((*this));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("U8Deserializer").field("value", &this->value).finish();
            }
        };

        ///A deserializer holding
        ///a `u16`.
        template<typename E>
        struct U16Deserializer {
            using Error = E;
            using Deserializer = U16Deserializer<E>;
            uint16_t value;
            rusty::PhantomData<E> marker;

            U16Deserializer<E> clone() const {
                return {.value = rusty::clone(this->value), .marker = rusty::clone(this->marker)};
            }
            static U16Deserializer<E> new_(uint16_t value) {
                return U16Deserializer<E>{.value = std::move(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U16Deserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_u16(std::move(this->value));
            }
            U16Deserializer<E> into_deserializer() {
                return std::move((*this));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("U16Deserializer").field("value", &this->value).finish();
            }
        };

        ///A deserializer holding
        ///a `u64`.
        template<typename E>
        struct U64Deserializer {
            using Error = E;
            using Deserializer = U64Deserializer<E>;
            uint64_t value;
            rusty::PhantomData<E> marker;

            U64Deserializer<E> clone() const {
                return {.value = rusty::clone(this->value), .marker = rusty::clone(this->marker)};
            }
            static U64Deserializer<E> new_(uint64_t value) {
                return U64Deserializer<E>{.value = std::move(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U64Deserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_u64(std::move(this->value));
            }
            U64Deserializer<E> into_deserializer() {
                return std::move((*this));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("U64Deserializer").field("value", &this->value).finish();
            }
        };

        ///A deserializer holding
        ///a `u128`.
        template<typename E>
        struct U128Deserializer {
            using Error = E;
            using Deserializer = U128Deserializer<E>;
            unsigned __int128 value;
            rusty::PhantomData<E> marker;

            U128Deserializer<E> clone() const {
                return {.value = rusty::clone(this->value), .marker = rusty::clone(this->marker)};
            }
            static U128Deserializer<E> new_(unsigned __int128 value) {
                return U128Deserializer<E>{.value = std::move(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U128Deserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_u128(std::move(this->value));
            }
            U128Deserializer<E> into_deserializer() {
                return std::move((*this));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("U128Deserializer").field("value", &this->value).finish();
            }
        };

        ///A deserializer holding
        ///a `usize`.
        template<typename E>
        struct UsizeDeserializer {
            using Error = E;
            using Deserializer = UsizeDeserializer<E>;
            size_t value;
            rusty::PhantomData<E> marker;

            UsizeDeserializer<E> clone() const {
                return {.value = rusty::clone(this->value), .marker = rusty::clone(this->marker)};
            }
            static UsizeDeserializer<E> new_(size_t value) {
                return UsizeDeserializer<E>{.value = std::move(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename UsizeDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_u64(static_cast<uint64_t>(this->value));
            }
            UsizeDeserializer<E> into_deserializer() {
                return std::move((*this));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("UsizeDeserializer").field("value", &this->value).finish();
            }
        };

        ///A deserializer holding
        ///an `f32`.
        template<typename E>
        struct F32Deserializer {
            using Error = E;
            using Deserializer = F32Deserializer<E>;
            float value;
            rusty::PhantomData<E> marker;

            F32Deserializer<E> clone() const {
                return {.value = rusty::clone(this->value), .marker = rusty::clone(this->marker)};
            }
            static F32Deserializer<E> new_(float value) {
                return F32Deserializer<E>{.value = std::move(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F32Deserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_f32(std::move(this->value));
            }
            F32Deserializer<E> into_deserializer() {
                return std::move((*this));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("F32Deserializer").field("value", &this->value).finish();
            }
        };

        ///A deserializer holding
        ///an `f64`.
        template<typename E>
        struct F64Deserializer {
            using Error = E;
            using Deserializer = F64Deserializer<E>;
            double value;
            rusty::PhantomData<E> marker;

            F64Deserializer<E> clone() const {
                return {.value = rusty::clone(this->value), .marker = rusty::clone(this->marker)};
            }
            static F64Deserializer<E> new_(double value) {
                return F64Deserializer<E>{.value = std::move(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename F64Deserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_f64(std::move(this->value));
            }
            F64Deserializer<E> into_deserializer() {
                return std::move((*this));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("F64Deserializer").field("value", &this->value).finish();
            }
        };

        ///A deserializer holding
        ///a `char`.
        template<typename E>
        struct CharDeserializer {
            using Error = E;
            using Deserializer = CharDeserializer<E>;
            char32_t value;
            rusty::PhantomData<E> marker;

            CharDeserializer<E> clone() const {
                return {.value = rusty::clone(this->value), .marker = rusty::clone(this->marker)};
            }
            static CharDeserializer<E> new_(char32_t value) {
                return CharDeserializer<E>{.value = std::move(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CharDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_char(std::move(this->value));
            }
            CharDeserializer<E> into_deserializer() {
                return std::move((*this));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("CharDeserializer").field("value", &this->value).finish();
            }
        };

        /// A deserializer holding a `u32`.
        template<typename E>
        struct U32Deserializer {
            using Error = E;
            using Deserializer = U32Deserializer<E>;
            using Variant = ::de::value::private_::UnitOnly<E>;
            uint32_t value;
            rusty::PhantomData<E> marker;

            U32Deserializer<E> clone() const {
                return {.value = rusty::clone(this->value), .marker = rusty::clone(this->marker)};
            }
            static U32Deserializer<E> new_(uint32_t value) {
                return U32Deserializer<E>{.value = std::move(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename U32Deserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_u32(std::move(this->value));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return visitor.visit_enum(std::move((*this)));
            }
            U32Deserializer<E> into_deserializer() {
                return std::move((*this));
            }
            template<typename T>
            rusty::Result<std::tuple<typename T::Value, Variant>, Error> variant_seed(T seed) {
                return ::de::rusty_ext::deserialize(seed, std::move((*this))).map(private_::unit_only);
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("U32Deserializer").field("value", &this->value).finish();
            }
        };

        /// A deserializer holding a `&str`.
        template<typename E>
        struct StrDeserializer {
            using Error = E;
            using Deserializer = StrDeserializer<E>;
            using Variant = ::de::value::private_::UnitOnly<E>;
            std::string_view value;
            rusty::PhantomData<E> marker;

            StrDeserializer<E> clone() const {
                return {.value = this->value, .marker = rusty::clone(this->marker)};
            }
            static StrDeserializer<E> new_(std::string_view value) {
                return StrDeserializer<E>{.value = std::string_view(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_str(std::move(std::string_view(this->value)));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return visitor.visit_enum(std::move((*this)));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            StrDeserializer<E> into_deserializer() {
                return std::move((*this));
            }
            template<typename T>
            rusty::Result<std::tuple<typename T::Value, Variant>, Error> variant_seed(T seed) {
                return ::de::rusty_ext::deserialize(seed, std::move((*this))).map(private_::unit_only);
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("StrDeserializer").field("value", &this->value).finish();
            }
        };

        /// A deserializer holding a `&str` with a lifetime tied to another
        /// deserializer.
        template<typename E>
        struct BorrowedStrDeserializer {
            using Error = E;
            using Deserializer = BorrowedStrDeserializer<E>;
            using Variant = ::de::value::private_::UnitOnly<E>;
            std::string_view value;
            rusty::PhantomData<E> marker;

            BorrowedStrDeserializer<E> clone() const {
                return {.value = this->value, .marker = rusty::clone(this->marker)};
            }
            static BorrowedStrDeserializer<E> new_(std::string_view value) {
                return BorrowedStrDeserializer<E>{.value = std::string_view(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_borrowed_str(std::move(std::string_view(this->value)));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return visitor.visit_enum(std::move((*this)));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            BorrowedStrDeserializer<E> into_deserializer() {
                return std::move((*this));
            }
            template<typename T>
            rusty::Result<std::tuple<typename T::Value, Variant>, Error> variant_seed(T seed) {
                return ::de::rusty_ext::deserialize(seed, std::move((*this))).map(private_::unit_only);
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("BorrowedStrDeserializer").field("value", &this->value).finish();
            }
        };

        /// A deserializer holding a `String`.
        template<typename E>
        struct StringDeserializer {
            using Error = E;
            using Deserializer = StringDeserializer<E>;
            using Variant = ::de::value::private_::UnitOnly<E>;
            rusty::String value;
            rusty::PhantomData<E> marker;

            StringDeserializer<E> clone() const {
                return StringDeserializer<E>{.value = rusty::clone(this->value), .marker = rusty::PhantomData<E>{}};
            }
            static StringDeserializer<E> new_(rusty::String value) {
                return StringDeserializer<E>{.value = std::move(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_string(std::move(this->value));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return visitor.visit_enum(std::move((*this)));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename StringDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            StringDeserializer<E> into_deserializer() {
                return std::move((*this));
            }
            template<typename T>
            rusty::Result<std::tuple<typename T::Value, Variant>, Error> variant_seed(T seed) {
                return ::de::rusty_ext::deserialize(seed, std::move((*this))).map(private_::unit_only);
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("StringDeserializer").field("value", &this->value).finish();
            }
        };

        /// A deserializer holding a `Cow<str>`.
        template<typename E>
        struct CowStrDeserializer {
            using Error = E;
            using Deserializer = CowStrDeserializer<E>;
            using Variant = ::de::value::private_::UnitOnly<E>;
            rusty::Cow value;
            rusty::PhantomData<E> marker;

            CowStrDeserializer<E> clone() const {
                return CowStrDeserializer<E>{.value = rusty::clone(this->value), .marker = rusty::PhantomData<E>{}};
            }
            static CowStrDeserializer<E> new_(rusty::Cow value) {
                return CowStrDeserializer<E>{.value = std::move(value), .marker = rusty::PhantomData<E>{}};
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return [&]() { auto&& _m = this->value; return std::visit(overloaded { [&](rusty::Cow_Borrowed&& _v) -> rusty::Result<typename V::Value, Error> { auto&& string = _v._0; return visitor.visit_str(std::move(rusty::to_string_view(string))); }, [&](rusty::Cow_Owned&& _v) -> rusty::Result<typename V::Value, Error> { auto&& string = _v._0; return visitor.visit_string(std::move(string)); } }, std::move(_m)); }();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return visitor.visit_enum(std::move((*this)));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename CowStrDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            CowStrDeserializer<E> into_deserializer() {
                return std::move((*this));
            }
            template<typename T>
            rusty::Result<std::tuple<typename T::Value, Variant>, Error> variant_seed(T seed) {
                return ::de::rusty_ext::deserialize(seed, std::move((*this))).map(private_::unit_only);
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("CowStrDeserializer").field("value", &this->value).finish();
            }
        };

        /// A deserializer holding a `&[u8]`. Always calls [`Visitor::visit_bytes`].
        template<typename E>
        struct BytesDeserializer {
            using Error = E;
            using Deserializer = BytesDeserializer<E>;
            std::span<const uint8_t> value;
            rusty::PhantomData<E> marker;

            static BytesDeserializer<E> new_(std::span<const uint8_t> value) {
                return BytesDeserializer<E>{.value = value, .marker = rusty::PhantomData<E>{}};
            }
            BytesDeserializer<E> clone() const {
                return {.value = this->value, .marker = rusty::clone(this->marker)};
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_bytes(std::move(this->value));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BytesDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            BytesDeserializer<E> into_deserializer() {
                return std::move((*this));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("BytesDeserializer").field("value", &this->value).finish();
            }
        };

        /// A deserializer holding a `&[u8]` with a lifetime tied to another
        /// deserializer. Always calls [`Visitor::visit_borrowed_bytes`].
        template<typename E>
        struct BorrowedBytesDeserializer {
            using Error = E;
            using Deserializer = BorrowedBytesDeserializer<E>;
            std::span<const uint8_t> value;
            rusty::PhantomData<E> marker;

            static BorrowedBytesDeserializer<E> new_(std::span<const uint8_t> value) {
                return BorrowedBytesDeserializer<E>{.value = value, .marker = rusty::PhantomData<E>{}};
            }
            BorrowedBytesDeserializer<E> clone() const {
                return {.value = this->value, .marker = rusty::clone(this->marker)};
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_borrowed_bytes(std::move(this->value));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename BorrowedBytesDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            BorrowedBytesDeserializer<E> into_deserializer() {
                return std::move((*this));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("BorrowedBytesDeserializer").field("value", &this->value).finish();
            }
        };

        /// A deserializer that iterates over a sequence.
        template<typename I, typename E>
        struct SeqDeserializer {
            using Error = E;
            using Deserializer = SeqDeserializer<I, E>;
            decltype(std::declval<I>().fuse()) iter;
            size_t count;
            rusty::PhantomData<E> marker;

            SeqDeserializer<I, E> clone() const {
                return SeqDeserializer<I, E>{.iter = rusty::clone(this->iter), .count = rusty::clone(this->count), .marker = rusty::clone(this->marker)};
            }
            static SeqDeserializer<I, E> new_(I iter) {
                return SeqDeserializer<I, E>{.iter = iter.fuse(), .count = static_cast<size_t>(0), .marker = rusty::PhantomData<E>{}};
            }
            rusty::Result<std::tuple<>, E> end() {
                const auto remaining = this->iter.count();
                if (remaining == 0) {
                    return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
                } else {
                    return rusty::Result<std::tuple<>, E>::Err(E::invalid_length(this->count + remaining, rusty::addr_of_temp(ExpectedInSeq(std::move(this->count)))));
                }
            }
            template<typename V, typename T>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                auto v = ({ auto&& _m = visitor.visit_seq((*this)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename V::Value, Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = this->end();
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename V::Value, Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return rusty::Result<typename V::Value, Error>::Ok(std::move(v));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V, typename T>
            ::rusty::Result<typename V::Value, typename SeqDeserializer<I, E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename T>
            SeqDeserializer<I, E> into_deserializer() {
                return std::move((*this));
            }
            template<typename V, typename T>
            rusty::Result<rusty::Option<typename V::Value>, Error> next_element_seed(V seed) {
                return [&]() -> rusty::Result<rusty::Option<typename V::Value>, Error> { auto&& _m = this->iter.next(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& value = _mv0; return [&]() -> rusty::Result<rusty::Option<typename V::Value>, Error> { [&]() { static_cast<void>(this->count += 1); return std::make_tuple(); }();
return ::de::rusty_ext::deserialize(seed, ::de::value::rusty_ext::into_deserializer(std::move(value))).map(rusty::Some); }(); } if (_m.is_none()) { return rusty::Result<rusty::Option<typename V::Value>, Error>::Ok(rusty::Option<typename V::Value>(rusty::None)); } return [&]() -> rusty::Result<rusty::Option<typename V::Value>, Error> { rusty::intrinsics::unreachable(); }(); }();
            }
            template<typename T>
            rusty::Option<size_t> size_hint() const {
                return size_hint::from_bounds(rusty::detail::deref_if_pointer_like(this->iter));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("SeqDeserializer").field("iter", &this->iter).field("count", &this->count).finish();
            }
        };

        struct ExpectedInSeq {
            size_t _0;

            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const;
        };

        /// A deserializer holding a `SeqAccess`.
        template<typename A>
        struct SeqAccessDeserializer {
            // Rust-only dependent associated type alias skipped in constrained mode: Error
            using Deserializer = SeqAccessDeserializer<A>;
            A seq;

            SeqAccessDeserializer<A> clone() const {
                return SeqAccessDeserializer<A>{.seq = rusty::clone(this->seq)};
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
                return std::conditional_t<true, rusty::fmt::Formatter, A>::debug_struct_field1_finish(f, "SeqAccessDeserializer", "seq", &this->seq);
            }
            static SeqAccessDeserializer<A> new_(A seq) {
                return SeqAccessDeserializer<A>{.seq = std::move(seq)};
            }
            // Rust-only dependent associated type alias skipped in constrained mode: Error
            template<typename V>
            rusty::Result<typename V::Value, typename A::Error> deserialize_any(V visitor) {
                return visitor.visit_seq(std::move(this->seq));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename SeqAccessDeserializer<A>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            SeqAccessDeserializer<A> into_deserializer() {
                return std::move((*this));
            }
        };

        /// A deserializer that iterates over a map.
        template<typename I, typename E>
        struct MapDeserializer {
            using Error = E;
            using Deserializer = MapDeserializer<I, E>;
            decltype(std::declval<I>().fuse()) iter;
            rusty::Option<de::value::private_::Second<rusty::detail::associated_item_t<I>>> value;
            size_t count;
            rusty::PhantomData<const std::tuple<>&> lifetime;
            rusty::PhantomData<E> error;

            static MapDeserializer<I, E> new_(I iter) {
                return MapDeserializer<I, E>{.iter = iter.fuse(), .value = rusty::Option<de::value::private_::Second<rusty::detail::associated_item_t<I>>>(rusty::None), .count = static_cast<size_t>(0), .lifetime = rusty::PhantomData<const std::tuple<>&>{}, .error = rusty::PhantomData<E>{}};
            }
            rusty::Result<std::tuple<>, E> end() {
                const auto remaining = this->iter.count();
                if (remaining == 0) {
                    return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
                } else {
                    return rusty::Result<std::tuple<>, E>::Err(E::invalid_length(this->count + remaining, rusty::addr_of_temp(ExpectedInMap(std::move(this->count)))));
                }
            }
            auto next_pair() {
                return [&]() -> rusty::Option<std::tuple<de::value::private_::First<rusty::detail::associated_item_t<I>>, de::value::private_::Second<rusty::detail::associated_item_t<I>>>> { auto&& _m = this->iter.next(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& kv = _mv0; return [&]() -> rusty::Option<std::tuple<de::value::private_::First<rusty::detail::associated_item_t<I>>, de::value::private_::Second<rusty::detail::associated_item_t<I>>>> { [&]() { static_cast<void>(this->count += 1); return std::make_tuple(); }();
return rusty::Option<std::tuple<de::value::private_::First<rusty::detail::associated_item_t<I>>, de::value::private_::Second<rusty::detail::associated_item_t<I>>>>((std::move(kv)).split()); }(); } if (_m.is_none()) { return rusty::Option<std::tuple<de::value::private_::First<rusty::detail::associated_item_t<I>>, de::value::private_::Second<rusty::detail::associated_item_t<I>>>>(rusty::None); } return [&]() -> rusty::Option<std::tuple<de::value::private_::First<rusty::detail::associated_item_t<I>>, de::value::private_::Second<rusty::detail::associated_item_t<I>>>> { rusty::intrinsics::unreachable(); }(); }();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                auto value = ({ auto&& _m = visitor.visit_map((*this)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename V::Value, Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = this->end();
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename V::Value, Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return rusty::Result<typename V::Value, Error>::Ok(std::move(value));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_seq(V visitor) {
                auto value = ({ auto&& _m = visitor.visit_seq((*this)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename V::Value, Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = this->end();
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename V::Value, Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return rusty::Result<typename V::Value, Error>::Ok(std::move(value));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_seq(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapDeserializer<I, E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            MapDeserializer<I, E> into_deserializer() {
                return std::move((*this));
            }
            template<typename T>
            rusty::Result<rusty::Option<typename T::Value>, Error> next_key_seed(T seed) {
                return [&]() -> rusty::Result<rusty::Option<typename T::Value>, Error> { auto&& _m = this->next_pair(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& key = std::get<0>(rusty::detail::deref_if_pointer(_mv0)); auto&& value = std::get<1>(rusty::detail::deref_if_pointer(_mv0)); return [&]() -> rusty::Result<rusty::Option<typename T::Value>, Error> { this->value = rusty::Option<de::value::private_::Second<rusty::detail::associated_item_t<I>>>(std::move(value));
return ::de::rusty_ext::deserialize(seed, ::de::value::rusty_ext::into_deserializer(std::move(key))).map(rusty::Some); }(); } if (_m.is_none()) { return rusty::Result<rusty::Option<typename T::Value>, Error>::Ok(rusty::Option<typename T::Value>(rusty::None)); } return [&]() -> rusty::Result<rusty::Option<typename T::Value>, Error> { rusty::intrinsics::unreachable(); }(); }();
            }
            template<typename T>
            rusty::Result<typename T::Value, Error> next_value_seed(T seed) {
                auto value = this->value.take();
                auto value_shadow1 = value.expect("MapAccess::next_value called before next_key");
                return ::de::rusty_ext::deserialize(seed, ::de::value::rusty_ext::into_deserializer(std::move(value_shadow1)));
            }
            template<typename TK, typename TV>
            rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error> next_entry_seed(TK kseed, TV vseed) {
                return [&]() -> rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error> { auto&& _m = this->next_pair(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& key = std::get<0>(rusty::detail::deref_if_pointer(_mv0)); auto&& value = std::get<1>(rusty::detail::deref_if_pointer(_mv0)); return [&]() -> rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error> { auto key_shadow1 = ({ auto&& _m = ::de::rusty_ext::deserialize(kseed, ::de::value::rusty_ext::into_deserializer(std::move(key))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error>::Err(std::move(err)); } std::move(_match_value).value(); });
auto value_shadow1 = ({ auto&& _m = ::de::rusty_ext::deserialize(vseed, ::de::value::rusty_ext::into_deserializer(std::move(value))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error>::Err(std::move(err)); } std::move(_match_value).value(); });
return rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error>::Ok(rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>(std::make_tuple(std::move(key_shadow1), std::move(value_shadow1)))); }(); } if (_m.is_none()) { return rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error>::Ok(rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>(rusty::None)); } return [&]() -> rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error> { rusty::intrinsics::unreachable(); }(); }();
            }
            rusty::Option<size_t> size_hint() const {
                return size_hint::from_bounds(rusty::detail::deref_if_pointer_like(this->iter));
            }
            template<typename T>
            rusty::Result<rusty::Option<typename T::Value>, Error> next_element_seed(T seed) {
                return [&]() -> rusty::Result<rusty::Option<typename T::Value>, Error> { auto&& _m = this->next_pair(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& k = std::get<0>(rusty::detail::deref_if_pointer(_mv0)); auto&& v = std::get<1>(rusty::detail::deref_if_pointer(_mv0)); return [&]() -> rusty::Result<rusty::Option<typename T::Value>, Error> { auto de_shadow1 = PairDeserializer(std::move(k), std::move(v), rusty::PhantomData<E>{});
return ::de::rusty_ext::deserialize(seed, std::move(de_shadow1)).map(rusty::Some); }(); } if (_m.is_none()) { return rusty::Result<rusty::Option<typename T::Value>, Error>::Ok(rusty::Option<typename T::Value>(rusty::None)); } return [&]() -> rusty::Result<rusty::Option<typename T::Value>, Error> { rusty::intrinsics::unreachable(); }(); }();
            }
            MapDeserializer<I, E> clone() const {
                return MapDeserializer<I, E>{.iter = rusty::clone(this->iter), .value = rusty::clone(this->value), .count = this->count, .lifetime = this->lifetime, .error = this->error};
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return formatter.debug_struct("MapDeserializer").field("iter", &this->iter).field("value", &this->value).field("count", &this->count).finish();
            }
        };

        template<typename A, typename B, typename E>
        struct PairDeserializer {
            using Error = E;
            A _0;
            B _1;
            rusty::PhantomData<E> _2;

            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename PairDeserializer<A, B, E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return this->deserialize_seq(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_seq(V visitor) {
                auto pair_visitor = PairVisitor(rusty::Option<A>(std::move(this->_0)), rusty::Option<B>(std::move(this->_1)), rusty::PhantomData<E>{});
                auto pair = ({ auto&& _m = visitor.visit_seq(pair_visitor); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename V::Value, Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                if (pair_visitor._1.is_none()) {
                    return rusty::Result<typename V::Value, Error>::Ok(std::move(pair));
                } else {
                    const auto remaining = pair_visitor.size_hint().unwrap();
                    return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_length(2, rusty::addr_of_temp(ExpectedInSeq(2 - remaining))));
                }
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_tuple(size_t len, V visitor) {
                if (len == static_cast<size_t>(2)) {
                    return this->deserialize_seq(std::move(visitor));
                } else {
                    return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_length(2, rusty::addr_of_temp(ExpectedInSeq(std::move(len)))));
                }
            }
        };

        template<typename A, typename B, typename E>
        struct PairVisitor {
            using Error = E;
            rusty::Option<A> _0;
            rusty::Option<B> _1;
            rusty::PhantomData<E> _2;

            template<typename T>
            rusty::Result<rusty::Option<typename T::Value>, Error> next_element_seed(T seed) {
                if (auto&& _iflet_scrutinee = this->_0.take(); _iflet_scrutinee.is_some()) {
                    decltype(auto) k = _iflet_scrutinee.unwrap();
                    return ::de::rusty_ext::deserialize(seed, ::de::value::rusty_ext::into_deserializer(std::move(k))).map(rusty::Some);
                } else if (auto&& _iflet_scrutinee = this->_1.take(); _iflet_scrutinee.is_some()) {
                    decltype(auto) v = _iflet_scrutinee.unwrap();
                    return ::de::rusty_ext::deserialize(seed, ::de::value::rusty_ext::into_deserializer(std::move(v))).map(rusty::Some);
                } else {
                    return rusty::Result<rusty::Option<typename T::Value>, Error>::Ok(rusty::Option<typename T::Value>(rusty::None));
                }
            }
            rusty::Option<size_t> size_hint() const {
                if (this->_0.is_some()) {
                    return rusty::Option<size_t>(static_cast<size_t>(2));
                } else if (this->_1.is_some()) {
                    return rusty::Option<size_t>(static_cast<size_t>(1));
                } else {
                    return rusty::Option<size_t>(static_cast<size_t>(0));
                }
            }
        };

        struct ExpectedInMap {
            size_t _0;

            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const;
        };

        /// A deserializer holding a `MapAccess`.
        template<typename A>
        struct MapAccessDeserializer {
            // Rust-only dependent associated type alias skipped in constrained mode: Error
            using Deserializer = MapAccessDeserializer<A>;
            // Rust-only dependent associated type alias skipped in constrained mode: Error
            using Variant = ::de::value::private_::MapAsEnum<A>;
            A map;

            MapAccessDeserializer<A> clone() const {
                return MapAccessDeserializer<A>{.map = rusty::clone(this->map)};
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
                return std::conditional_t<true, rusty::fmt::Formatter, A>::debug_struct_field1_finish(f, "MapAccessDeserializer", "map", &this->map);
            }
            static MapAccessDeserializer<A> new_(A map) {
                return MapAccessDeserializer<A>{.map = std::move(map)};
            }
            // Rust-only dependent associated type alias skipped in constrained mode: Error
            template<typename V>
            rusty::Result<typename V::Value, typename A::Error> deserialize_any(V visitor) {
                return visitor.visit_map(std::move(this->map));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename A::Error> deserialize_enum(std::string_view _name, std::span<const std::string_view> _variants, V visitor) {
                return visitor.visit_enum(std::move((*this)));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename MapAccessDeserializer<A>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            MapAccessDeserializer<A> into_deserializer() {
                return std::move((*this));
            }
            // Rust-only dependent associated type alias skipped in constrained mode: Error
            template<typename T>
            rusty::Result<std::tuple<typename T::Value, Variant>, typename A::Error> variant_seed(T seed) {
                return [&]() -> rusty::Result<std::tuple<typename T::Value, Variant>, typename A::Error> { auto&& _m = ({ auto&& _m = this->map.next_key_seed(std::move(seed)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<std::tuple<typename T::Value, Variant>, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); }); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& key = _mv0; return rusty::Result<std::tuple<typename T::Value, Variant>, typename A::Error>::Ok(std::make_tuple(std::move(key), private_::map_as_enum(std::move(this->map)))); } if (_m.is_none()) { return rusty::Result<std::tuple<typename T::Value, Variant>, typename A::Error>::Err(A::Error::invalid_type(::de::Unexpected_Map{}, rusty::addr_of_temp("enum"))); } return [&]() -> rusty::Result<std::tuple<typename T::Value, Variant>, typename A::Error> { rusty::intrinsics::unreachable(); }(); }();
            }
        };

        /// A deserializer holding an `EnumAccess`.
        template<typename A>
        struct EnumAccessDeserializer {
            // Rust-only dependent associated type alias skipped in constrained mode: Error
            using Deserializer = EnumAccessDeserializer<A>;
            A access;

            EnumAccessDeserializer<A> clone() const {
                return EnumAccessDeserializer<A>{.access = rusty::clone(this->access)};
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
                return std::conditional_t<true, rusty::fmt::Formatter, A>::debug_struct_field1_finish(f, "EnumAccessDeserializer", "access", &this->access);
            }
            static EnumAccessDeserializer<A> new_(A access) {
                return EnumAccessDeserializer<A>{.access = std::move(access)};
            }
            // Rust-only dependent associated type alias skipped in constrained mode: Error
            template<typename V>
            rusty::Result<typename V::Value, typename A::Error> deserialize_any(V visitor) {
                return visitor.visit_enum(std::move(this->access));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            ::rusty::Result<typename V::Value, typename EnumAccessDeserializer<A>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            EnumAccessDeserializer<A> into_deserializer() {
                return std::move((*this));
            }
        };

        namespace private_ {

            template<typename E>
            struct UnitOnly;
            template<typename A>
            struct MapAsEnum;
            template<typename V>
            struct SeedTupleVariant;
            template<typename V>
            struct SeedStructVariant;
            using ::de::Unexpected;
            template<typename T, typename E>
            std::tuple<T, UnitOnly<E>> unit_only(T t);
            template<typename A>
            MapAsEnum<A> map_as_enum(A map);

            using namespace lib;

            namespace de = ::de;
            using ::de::Unexpected;
            using namespace de;

            template<typename E>
            struct UnitOnly {
                using Error = E;
                rusty::PhantomData<E> marker;

                auto unit_variant() {
                    return rusty::Result<std::tuple<>, Error>::Ok(std::make_tuple());
                }
                template<typename T>
                rusty::Result<typename T::Value, Error> newtype_variant_seed(T _seed) {
                    return rusty::Result<typename T::Value, Error>::Err(std::conditional_t<true, Error, T>::invalid_type(de::Unexpected_UnitVariant{}, rusty::addr_of_temp("newtype variant")));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> tuple_variant(size_t _len, V _visitor) {
                    return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_type(de::Unexpected_UnitVariant{}, rusty::addr_of_temp("tuple variant")));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> struct_variant(std::span<const std::string_view> _fields, V _visitor) {
                    return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_type(de::Unexpected_UnitVariant{}, rusty::addr_of_temp("struct variant")));
                }
            };

            template<typename A>
            struct MapAsEnum {
                // Rust-only dependent associated type alias skipped in constrained mode: Error
                A map;

                // Rust-only dependent associated type alias skipped in constrained mode: Error
                rusty::Result<std::tuple<>, typename A::Error> unit_variant() {
                    return this->map.next_value();
                }
                template<typename T>
                rusty::Result<typename T::Value, typename A::Error> newtype_variant_seed(T seed) {
                    return this->map.next_value_seed(std::move(seed));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename A::Error> tuple_variant(size_t len, V visitor) {
                    return this->map.next_value_seed(SeedTupleVariant<V>(std::move(len), std::move(visitor)));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename A::Error> struct_variant(std::span<const std::string_view> _fields, V visitor) {
                    return this->map.next_value_seed(SeedStructVariant<V>(std::move(visitor)));
                }
            };

            template<typename V>
            struct SeedTupleVariant {
                // Rust-only dependent associated type alias skipped in constrained mode: Value
                size_t len;
                V visitor;

                // Rust-only dependent associated type alias skipped in constrained mode: Value
                template<typename D>
                rusty::Result<typename V::Value, typename D::Error> deserialize(D deserializer) {
                    return deserializer.deserialize_tuple(std::move(this->len), std::move(this->visitor));
                }
            };

            template<typename V>
            struct SeedStructVariant {
                // Rust-only dependent associated type alias skipped in constrained mode: Value
                V visitor;

                // Rust-only dependent associated type alias skipped in constrained mode: Value
                template<typename D>
                rusty::Result<typename V::Value, typename D::Error> deserialize(D deserializer) {
                    return deserializer.deserialize_map(std::move(this->visitor));
                }
            };

            // Rust-only trait Pair (Proxy facade emission skipped in module mode)



            template<typename T, typename E>
            std::tuple<T, UnitOnly<E>> unit_only(T t) {
                return std::make_tuple(std::move(t), UnitOnly<E>{.marker = rusty::PhantomData<E>{}});
            }

            template<typename A>
            MapAsEnum<A> map_as_enum(A map) {
                return MapAsEnum<A>{.map = std::move(map)};
            }

        }

        // Extension trait IntoDeserializer lowered to rusty_ext:: free functions
        namespace rusty_ext {
            template<typename E>
            BoolDeserializer<E> into_deserializer(bool self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return BoolDeserializer<E>::new_(std::move(self_));
            }

            template<typename E>
            I8Deserializer<E> into_deserializer(int8_t self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return I8Deserializer<E>::new_(std::move(self_));
            }

            template<typename E>
            I16Deserializer<E> into_deserializer(int16_t self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return I16Deserializer<E>::new_(std::move(self_));
            }

            template<typename E>
            I32Deserializer<E> into_deserializer(int32_t self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return I32Deserializer<E>::new_(std::move(self_));
            }

            template<typename E>
            I64Deserializer<E> into_deserializer(int64_t self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return I64Deserializer<E>::new_(std::move(self_));
            }

            template<typename E>
            I128Deserializer<E> into_deserializer(__int128 self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return I128Deserializer<E>::new_(std::move(self_));
            }

            template<typename E>
            IsizeDeserializer<E> into_deserializer(ptrdiff_t self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return IsizeDeserializer<E>::new_(std::move(self_));
            }

            template<typename E>
            U8Deserializer<E> into_deserializer(uint8_t self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return U8Deserializer<E>::new_(std::move(self_));
            }

            template<typename E>
            U16Deserializer<E> into_deserializer(uint16_t self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return U16Deserializer<E>::new_(std::move(self_));
            }

            template<typename E>
            U64Deserializer<E> into_deserializer(uint64_t self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return U64Deserializer<E>::new_(std::move(self_));
            }

            template<typename E>
            U128Deserializer<E> into_deserializer(unsigned __int128 self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return U128Deserializer<E>::new_(std::move(self_));
            }

            template<typename E>
            UsizeDeserializer<E> into_deserializer(size_t self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return UsizeDeserializer<E>::new_(std::move(self_));
            }

            template<typename E>
            F32Deserializer<E> into_deserializer(float self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return F32Deserializer<E>::new_(std::move(self_));
            }

            template<typename E>
            F64Deserializer<E> into_deserializer(double self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return F64Deserializer<E>::new_(std::move(self_));
            }

            template<typename E>
            CharDeserializer<E> into_deserializer(char32_t self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return CharDeserializer<E>::new_(std::move(self_));
            }

            template<typename E>
            U32Deserializer<E> into_deserializer(uint32_t self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return U32Deserializer<E>::new_(std::move(self_));
            }

            template<typename E>
            StringDeserializer<E> into_deserializer(rusty::String self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return StringDeserializer<E>::new_(std::move(self_));
            }

            template<typename E>
            CowStrDeserializer<E> into_deserializer(rusty::Cow self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return CowStrDeserializer<E>::new_(std::move(self_));
            }

            template<typename T, typename E>
            SeqDeserializer<typename rusty::Vec<T>::IntoIter, E> into_deserializer(rusty::Vec<T> self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return SeqDeserializer<T, E>::new_(rusty::iter(std::move(self_)));
            }

            template<typename T, typename E>
            SeqDeserializer<typename rusty::BTreeSet<T>::IntoIter, E> into_deserializer(rusty::BTreeSet<T> self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return SeqDeserializer<T, E>::new_(self_.into_iter());
            }

            template<typename T, typename S, typename E>
            SeqDeserializer<typename rusty::HashSet<T, S>::IntoIter, E> into_deserializer(rusty::HashSet<T, S> self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return SeqDeserializer<T, E>::new_(self_.into_iter());
            }

            template<typename K, typename V, typename E>
            MapDeserializer<typename rusty::BTreeMap<K, V>::IntoIter, E> into_deserializer(rusty::BTreeMap<K, V> self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return MapDeserializer<K, E>::new_(self_.into_iter());
            }

            template<typename K, typename V, typename S, typename E>
            MapDeserializer<typename rusty::HashMap<K, V, S>::IntoIter, E> into_deserializer(rusty::HashMap<K, V, S> self_) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return MapDeserializer<K, E>::new_(self_.into_iter());
            }

        }


    }

    namespace ignored_any {

        struct IgnoredAny;

        using namespace lib;


        /// An efficient way of discarding data from a deserializer.
        ///
        /// Think of this like `serde_json::Value` in that it can be deserialized from
        /// any type, except that it does not store any information about the data that
        /// gets deserialized.
        ///
        /// ```edition2021
        /// use serde::de::{
        ///     self, Deserialize, DeserializeSeed, Deserializer, IgnoredAny, SeqAccess, Visitor,
        /// };
        /// use std::fmt;
        /// use std::marker::PhantomData;
        ///
        /// /// A seed that can be used to deserialize only the `n`th element of a sequence
        /// /// while efficiently discarding elements of any type before or after index `n`.
        /// ///
        /// /// For example to deserialize only the element at index 3:
        /// ///
        /// /// ```
        /// /// NthElement::new(3).deserialize(deserializer)
        /// /// ```
        /// pub struct NthElement<T> {
        ///     n: usize,
        ///     marker: PhantomData<T>,
        /// }
        ///
        /// impl<T> NthElement<T> {
        ///     pub fn new(n: usize) -> Self {
        ///         NthElement {
        ///             n: n,
        ///             marker: PhantomData,
        ///         }
        ///     }
        /// }
        ///
        /// impl<'de, T> Visitor<'de> for NthElement<T>
        /// where
        ///     T: Deserialize<'de>,
        /// {
        ///     type Value = T;
        ///
        ///     fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        ///         write!(
        ///             formatter,
        ///             "a sequence in which we care about element {}",
        ///             self.n
        ///         )
        ///     }
        ///
        ///     fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        ///     where
        ///         A: SeqAccess<'de>,
        ///     {
        ///         // Skip over the first `n` elements.
        ///         for i in 0..self.n {
        ///             // It is an error if the sequence ends before we get to element `n`.
        ///             if seq.next_element::<IgnoredAny>()?.is_none() {
        ///                 return Err(de::Error::invalid_length(i, &self));
        ///             }
        ///         }
        ///
        ///         // Deserialize the one we care about.
        ///         let nth = match seq.next_element()? {
        ///             Some(nth) => nth,
        ///             None => {
        ///                 return Err(de::Error::invalid_length(self.n, &self));
        ///             }
        ///         };
        ///
        ///         // Skip over any remaining elements in the sequence after `n`.
        ///         while let Some(IgnoredAny) = seq.next_element()? {
        ///             // ignore
        ///         }
        ///
        ///         Ok(nth)
        ///     }
        /// }
        ///
        /// impl<'de, T> DeserializeSeed<'de> for NthElement<T>
        /// where
        ///     T: Deserialize<'de>,
        /// {
        ///     type Value = T;
        ///
        ///     fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        ///     where
        ///         D: Deserializer<'de>,
        ///     {
        ///         deserializer.deserialize_seq(self)
        ///     }
        /// }
        ///
        /// # fn example<'de, D>(deserializer: D) -> Result<(), D::Error>
        /// # where
        /// #     D: Deserializer<'de>,
        /// # {
        /// // Deserialize only the sequence element at index 3 from this deserializer.
        /// // The element at index 3 is required to be a string. Elements before and
        /// // after index 3 are allowed to be of any type.
        /// let s: String = NthElement::new(3).deserialize(deserializer)?;
        /// #     Ok(())
        /// # }
        /// ```
        struct IgnoredAny {
            using Value = IgnoredAny;

            IgnoredAny clone() const;
            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
            static IgnoredAny default_();
            bool operator==(const IgnoredAny& other) const;
            rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const;
            template<typename E>
            rusty::Result<IgnoredAny, E> visit_bool(bool x);
            template<typename E>
            rusty::Result<IgnoredAny, E> visit_i64(int64_t x);
            template<typename E>
            rusty::Result<IgnoredAny, E> visit_i128(__int128 x);
            template<typename E>
            rusty::Result<IgnoredAny, E> visit_u64(uint64_t x);
            template<typename E>
            rusty::Result<IgnoredAny, E> visit_u128(unsigned __int128 x);
            template<typename E>
            rusty::Result<IgnoredAny, E> visit_f64(double x);
            template<typename E>
            rusty::Result<IgnoredAny, E> visit_str(std::string_view s);
            template<typename E>
            rusty::Result<IgnoredAny, E> visit_none();
            template<typename D>
            rusty::Result<IgnoredAny, typename D::Error> visit_some(D deserializer);
            template<typename D>
            rusty::Result<IgnoredAny, typename D::Error> visit_newtype_struct(D deserializer);
            template<typename E>
            rusty::Result<IgnoredAny, E> visit_unit();
            template<typename A>
            rusty::Result<IgnoredAny, typename A::Error> visit_seq(A seq);
            template<typename A>
            rusty::Result<IgnoredAny, typename A::Error> visit_map(A map);
            template<typename E>
            rusty::Result<IgnoredAny, E> visit_bytes(std::span<const uint8_t> bytes);
            template<typename A>
            rusty::Result<IgnoredAny, typename A::Error> visit_enum(A data);
            template<typename D>
            static auto deserialize(D deserializer);
            template<typename D>
            static auto deserialize_in_place(D deserializer, IgnoredAny& place);
        };

    }

    namespace impls {
        namespace range {}
        namespace range_from {}
        namespace range_to {}

        enum class OsStringKind;
        constexpr OsStringKind OsStringKind_Unix();
        constexpr OsStringKind OsStringKind_Windows();
        struct UnitVisitor;
        struct BoolVisitor;
        struct CharVisitor;
        struct StringVisitor;
        struct StringInPlaceVisitor;
        struct StrVisitor;
        struct BytesVisitor;
        struct CStringVisitor;
        template<typename T>
        struct OptionVisitor;
        template<typename T>
        struct PhantomDataVisitor;
        template<typename A>
        struct ArrayVisitor;
        template<typename A>
        struct ArrayInPlaceVisitor;
        struct PathVisitor;
        struct PathBufVisitor;
        struct OsStringVisitor;
        template<typename T>
        struct FromStrVisitor;
        namespace range {
            enum class Field;
            constexpr Field Field_Start();
            constexpr Field Field_End();
            template<typename Idx>
            struct RangeVisitor;
            extern const std::span<const std::string_view> FIELDS;
        }
        namespace range_from {
            enum class Field;
            constexpr Field Field_Start();
            template<typename Idx>
            struct RangeFromVisitor;
            extern const std::span<const std::string_view> FIELDS;
        }
        namespace range_to {
            enum class Field;
            constexpr Field Field_End();
            template<typename Idx>
            struct RangeToVisitor;
            extern const std::span<const std::string_view> FIELDS;
        }
        using ::de::Unexpected;
        template<typename T>
        void nop_reserve(T _seq, size_t _n);

        using namespace lib;

        using ::de::Unexpected;
        using namespace de;

        namespace private_ = ::private_;
        using private_::seed::InPlaceSeed;

        namespace size_hint = ::private_::size_hint;

        enum class OsStringKind {
            Unix,
    Windows
        };
        inline constexpr OsStringKind OsStringKind_Unix() { return OsStringKind::Unix; }
        inline constexpr OsStringKind OsStringKind_Windows() { return OsStringKind::Windows; }

        struct UnitVisitor {
            using Value = std::tuple<>;

            rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const;
            template<typename E>
            rusty::Result<std::tuple<>, E> visit_unit();
        };

        struct BoolVisitor {
            using Value = bool;

            rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const;
            template<typename E>
            rusty::Result<bool, E> visit_bool(bool v);
        };

        struct CharVisitor {
            using Value = char32_t;

            rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const;
            template<typename E>
            rusty::Result<char32_t, E> visit_char(char32_t v);
            template<typename E>
            rusty::Result<char32_t, E> visit_str(std::string_view v);
        };

        struct StringVisitor {
            using Value = rusty::String;

            rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const;
            template<typename E>
            rusty::Result<rusty::String, E> visit_str(std::string_view v);
            template<typename E>
            rusty::Result<rusty::String, E> visit_string(rusty::String v);
            template<typename E>
            rusty::Result<rusty::String, E> visit_bytes(std::span<const uint8_t> v);
            template<typename E>
            rusty::Result<rusty::String, E> visit_byte_buf(rusty::Vec<uint8_t> v);
        };

        struct StringInPlaceVisitor {
            using Value = std::tuple<>;
            rusty::String& _0;

            rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const;
            template<typename E>
            rusty::Result<std::tuple<>, E> visit_str(std::string_view v);
            template<typename E>
            rusty::Result<std::tuple<>, E> visit_string(rusty::String v);
            template<typename E>
            rusty::Result<std::tuple<>, E> visit_bytes(std::span<const uint8_t> v);
            template<typename E>
            rusty::Result<std::tuple<>, E> visit_byte_buf(rusty::Vec<uint8_t> v);
        };

        struct StrVisitor {
            using Value = std::string_view;

            rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const;
            template<typename E>
            rusty::Result<std::string_view, E> visit_borrowed_str(std::string_view v);
            template<typename E>
            rusty::Result<std::string_view, E> visit_borrowed_bytes(std::span<const uint8_t> v);
        };

        struct BytesVisitor {
            using Value = std::span<const uint8_t>;

            rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const;
            template<typename E>
            rusty::Result<std::span<const uint8_t>, E> visit_borrowed_bytes(std::span<const uint8_t> v);
            template<typename E>
            rusty::Result<std::span<const uint8_t>, E> visit_borrowed_str(std::string_view v);
        };

        struct CStringVisitor {
            using Value = rusty::ffi::CString;

            rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const;
            template<typename A>
            rusty::Result<rusty::ffi::CString, typename A::Error> visit_seq(A seq);
            template<typename E>
            rusty::Result<rusty::ffi::CString, E> visit_bytes(std::span<const uint8_t> v);
            template<typename E>
            rusty::Result<rusty::ffi::CString, E> visit_byte_buf(rusty::Vec<uint8_t> v);
            template<typename E>
            rusty::Result<rusty::ffi::CString, E> visit_str(std::string_view v);
            template<typename E>
            rusty::Result<rusty::ffi::CString, E> visit_string(rusty::String v);
        };

        template<typename T>
        struct OptionVisitor {
            using Value = rusty::Option<T>;
            rusty::PhantomData<T> marker;

            rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const {
                return formatter.write_str(std::string_view("option"));
            }
            template<typename E>
            rusty::Result<Value, E> visit_unit() {
                return rusty::Result<Value, E>::Ok(rusty::None);
            }
            template<typename E>
            rusty::Result<Value, E> visit_none() {
                return rusty::Result<Value, E>::Ok(rusty::None);
            }
            template<typename D>
            rusty::Result<Value, typename D::Error> visit_some(D deserializer) {
                return T::deserialize(std::move(deserializer)).map(rusty::Some);
            }
            template<typename D>
            auto __private_visit_untagged_option(D deserializer) {
                return rusty::Result<Value, std::tuple<>>::Ok(T::deserialize(std::move(deserializer)).ok());
            }
        };

        template<typename T>
        struct PhantomDataVisitor {
            using Value = rusty::PhantomData<T>;
            rusty::PhantomData<T> marker;

            rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const {
                return formatter.write_str(std::string_view("unit"));
            }
            template<typename E>
            rusty::Result<Value, E> visit_unit() {
                return rusty::Result<Value, E>::Ok(rusty::PhantomData<std::tuple<>>{});
            }
        };

        template<typename A>
        struct ArrayVisitor {
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            rusty::PhantomData<A> marker;

            static ArrayVisitor<A> new_() {
                return ArrayVisitor<A>{.marker = rusty::PhantomData<A>{}};
            }
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            template<typename T>
            rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const {
                return formatter.write_str(std::string_view("an empty array"));
            }
            template<typename T>
            rusty::Result<std::array<T, 0>, typename A::Error> visit_seq(A _arg1) {
                return rusty::Result<std::array<T, 0>, typename A::Error>::Ok(std::array<int, 0>{});
            }
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Value
        };

        template<typename A>
        struct ArrayInPlaceVisitor {
            using Value = std::conditional_t<true, std::tuple<>, A>;
            A& _0;

            template<typename T>
            rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const {
                return formatter.write_str(std::string_view("an array of length 1"));
            }
            template<typename T>
            rusty::Result<Value, typename A::Error> visit_seq(A seq) {
                rusty::Option<size_t> fail_idx = rusty::Option<size_t>(rusty::None);
                for (auto&& _for_item : rusty::for_in(rusty::enumerate(rusty::iter_mut(rusty::slice_full(this->_0))))) {
                    auto&& idx = std::get<0>(rusty::detail::deref_if_pointer(_for_item));
                    auto&& dest = std::get<1>(rusty::detail::deref_if_pointer(_for_item));
                    if (({ auto&& _m = seq.next_element_seed(::private_::seed::InPlaceSeed(rusty::detail::deref_if_pointer_like(dest))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Value, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); }).is_none()) {
                        fail_idx = rusty::Option<size_t>(std::move(idx));
                        break;
                    }
                }
                if (fail_idx.is_some()) {
                    decltype(auto) idx = fail_idx.unwrap();
                    return rusty::Result<Value, typename A::Error>::Err(A::Error::invalid_length(std::move(idx), (*this)));
                }
                return rusty::Result<Value, typename A::Error>::Ok(std::make_tuple());
            }
        };

        struct PathVisitor {
            using Value = const rusty::path::Path&;

            rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const;
            template<typename E>
            rusty::Result<const rusty::path::Path&, E> visit_borrowed_str(std::string_view v);
            template<typename E>
            rusty::Result<const rusty::path::Path&, E> visit_borrowed_bytes(std::span<const uint8_t> v);
        };

        struct PathBufVisitor {
            using Value = rusty::path::PathBuf;

            rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const;
            template<typename E>
            rusty::Result<rusty::path::PathBuf, E> visit_str(std::string_view v);
            template<typename E>
            rusty::Result<rusty::path::PathBuf, E> visit_string(rusty::String v);
            template<typename E>
            rusty::Result<rusty::path::PathBuf, E> visit_bytes(std::span<const uint8_t> v);
            template<typename E>
            rusty::Result<rusty::path::PathBuf, E> visit_byte_buf(rusty::Vec<uint8_t> v);
        };

        static std::span<const std::string_view> OSSTR_VARIANTS = []() -> std::span<const std::string_view> { static const std::array<std::string_view, 2> _slice_ref_tmp = {std::string_view("Unix"), std::string_view("Windows")}; return std::span<const std::string_view>(_slice_ref_tmp); }();

        struct OsStringVisitor {
            using Value = rusty::ffi::OsString;

            rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const;
            template<typename A>
            rusty::Result<rusty::ffi::OsString, typename A::Error> visit_enum(A data);
        };

        namespace range {

            enum class Field;
            constexpr Field Field_Start();
            constexpr Field Field_End();
            template<typename Idx>
            struct RangeVisitor;
            extern const std::span<const std::string_view> FIELDS;

            enum class Field {
                Start,
    End
            };
            inline constexpr Field Field_Start() { return Field::Start; }
            inline constexpr Field Field_End() { return Field::End; }

            using namespace lib;


            namespace private_ = ::private_;

            const std::span<const std::string_view> FIELDS = []() -> std::span<const std::string_view> { static const std::array<std::string_view, 2> _slice_ref_tmp = {std::string_view("start"), std::string_view("end")}; return std::span<const std::string_view>(_slice_ref_tmp); }();

            template<typename Idx>
            struct RangeVisitor {
                using Value = std::tuple<Idx, Idx>;
                std::string_view expecting_field;
                rusty::PhantomData<Idx> phantom;

                rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const {
                    return formatter.write_str(std::string_view(this->expecting_field));
                }
                template<typename A>
                rusty::Result<Value, typename A::Error> visit_seq(A seq) {
                    Idx start = ({ auto&& _m = ({ auto&& _m = seq.next_element(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Value, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); }); std::optional<Idx> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& value = _mv;
_match_value.emplace(std::move(value)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::Result<Value, typename A::Error>::Err(A::Error::invalid_length(0, (*this))); } std::move(_match_value).value(); });
                    Idx end = ({ auto&& _m = ({ auto&& _m = seq.next_element(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Value, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); }); std::optional<Idx> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& value = _mv;
_match_value.emplace(std::move(value)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::Result<Value, typename A::Error>::Err(A::Error::invalid_length(1, (*this))); } std::move(_match_value).value(); });
                    return rusty::Result<Value, typename A::Error>::Ok(std::make_tuple(std::move(start), std::move(end)));
                }
                template<typename A>
                rusty::Result<Value, typename A::Error> visit_map(A map) {
                    rusty::Option<Idx> start = rusty::Option<Idx>(rusty::None);
                    rusty::Option<Idx> end = rusty::Option<Idx>(rusty::None);
                    while (true) {
                        auto&& _whilelet = ({ auto&& _m = map.next_key(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Value, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                        if (!(_whilelet.is_some())) { break; }
                        auto key = _whilelet.unwrap();
                        switch (key) {
                        case Field::Start:
                        {
                            if (start.is_some()) {
                                return rusty::Result<Value, typename A::Error>::Err(A::Error::duplicate_field("start"));
                            }
                            start = rusty::Option<Idx>(({ auto&& _m = map.next_value(); std::optional<Idx> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Value, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); }));
                            break;
                        }
                        case Field::End:
                        {
                            if (end.is_some()) {
                                return rusty::Result<Value, typename A::Error>::Err(A::Error::duplicate_field("end"));
                            }
                            end = rusty::Option<Idx>(({ auto&& _m = map.next_value(); std::optional<Idx> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Value, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); }));
                            break;
                        }
                        }
                    }
                    auto start_shadow1 = ({ auto&& _m = start; std::optional<Idx> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& start = _mv;
_match_value.emplace(std::move(start)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::Result<Value, typename A::Error>::Err(A::Error::missing_field("start")); } std::move(_match_value).value(); });
                    auto end_shadow1 = ({ auto&& _m = end; std::optional<Idx> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& end = _mv;
_match_value.emplace(std::move(end)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::Result<Value, typename A::Error>::Err(A::Error::missing_field("end")); } std::move(_match_value).value(); });
                    return rusty::Result<Value, typename A::Error>::Ok(std::make_tuple(std::move(start_shadow1), std::move(end_shadow1)));
                }
            };

        }

        namespace range_from {

            enum class Field;
            constexpr Field Field_Start();
            template<typename Idx>
            struct RangeFromVisitor;
            extern const std::span<const std::string_view> FIELDS;

            enum class Field {
                Start
            };
            inline constexpr Field Field_Start() { return Field::Start; }

            using namespace lib;


            namespace private_ = ::private_;

            const std::span<const std::string_view> FIELDS = []() -> std::span<const std::string_view> { static const std::array<std::string_view, 1> _slice_ref_tmp = {std::string_view("start")}; return std::span<const std::string_view>(_slice_ref_tmp); }();

            template<typename Idx>
            struct RangeFromVisitor {
                using Value = Idx;
                std::string_view expecting_field;
                rusty::PhantomData<Idx> phantom;

                rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const {
                    return formatter.write_str(std::string_view(this->expecting_field));
                }
                template<typename A>
                rusty::Result<Value, typename A::Error> visit_seq(A seq) {
                    Idx start = ({ auto&& _m = ({ auto&& _m = seq.next_element(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Value, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); }); std::optional<Idx> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& value = _mv;
_match_value.emplace(std::move(value)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::Result<Value, typename A::Error>::Err(A::Error::invalid_length(0, (*this))); } std::move(_match_value).value(); });
                    return rusty::Result<Value, typename A::Error>::Ok(std::move(start));
                }
                template<typename A>
                rusty::Result<Value, typename A::Error> visit_map(A map) {
                    rusty::Option<Idx> start = rusty::Option<Idx>(rusty::None);
                    while (true) {
                        auto&& _whilelet = ({ auto&& _m = map.next_key(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Value, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                        if (!(_whilelet.is_some())) { break; }
                        auto key = _whilelet.unwrap();
                        switch (key) {
                        case Field::Start:
                        {
                            if (start.is_some()) {
                                return rusty::Result<Value, typename A::Error>::Err(A::Error::duplicate_field("start"));
                            }
                            start = rusty::Option<Idx>(({ auto&& _m = map.next_value(); std::optional<Idx> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Value, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); }));
                            break;
                        }
                        }
                    }
                    auto start_shadow1 = ({ auto&& _m = start; std::optional<Idx> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& start = _mv;
_match_value.emplace(std::move(start)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::Result<Value, typename A::Error>::Err(A::Error::missing_field("start")); } std::move(_match_value).value(); });
                    return rusty::Result<Value, typename A::Error>::Ok(std::move(start_shadow1));
                }
            };

        }

        namespace range_to {

            enum class Field;
            constexpr Field Field_End();
            template<typename Idx>
            struct RangeToVisitor;
            extern const std::span<const std::string_view> FIELDS;

            enum class Field {
                End
            };
            inline constexpr Field Field_End() { return Field::End; }

            using namespace lib;


            namespace private_ = ::private_;

            const std::span<const std::string_view> FIELDS = []() -> std::span<const std::string_view> { static const std::array<std::string_view, 1> _slice_ref_tmp = {std::string_view("end")}; return std::span<const std::string_view>(_slice_ref_tmp); }();

            template<typename Idx>
            struct RangeToVisitor {
                using Value = Idx;
                std::string_view expecting_field;
                rusty::PhantomData<Idx> phantom;

                rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const {
                    return formatter.write_str(std::string_view(this->expecting_field));
                }
                template<typename A>
                rusty::Result<Value, typename A::Error> visit_seq(A seq) {
                    Idx end = ({ auto&& _m = ({ auto&& _m = seq.next_element(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Value, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); }); std::optional<Idx> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& value = _mv;
_match_value.emplace(std::move(value)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::Result<Value, typename A::Error>::Err(A::Error::invalid_length(0, (*this))); } std::move(_match_value).value(); });
                    return rusty::Result<Value, typename A::Error>::Ok(std::move(end));
                }
                template<typename A>
                rusty::Result<Value, typename A::Error> visit_map(A map) {
                    rusty::Option<Idx> end = rusty::Option<Idx>(rusty::None);
                    while (true) {
                        auto&& _whilelet = ({ auto&& _m = map.next_key(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Value, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                        if (!(_whilelet.is_some())) { break; }
                        auto key = _whilelet.unwrap();
                        switch (key) {
                        case Field::End:
                        {
                            if (end.is_some()) {
                                return rusty::Result<Value, typename A::Error>::Err(A::Error::duplicate_field("end"));
                            }
                            end = rusty::Option<Idx>(({ auto&& _m = map.next_value(); std::optional<Idx> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Value, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); }));
                            break;
                        }
                        }
                    }
                    auto end_shadow1 = ({ auto&& _m = end; std::optional<Idx> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& end = _mv;
_match_value.emplace(std::move(end)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::Result<Value, typename A::Error>::Err(A::Error::missing_field("end")); } std::move(_match_value).value(); });
                    return rusty::Result<Value, typename A::Error>::Ok(std::move(end_shadow1));
                }
            };

        }

        template<typename T>
        struct FromStrVisitor {
            using Value = T;
            std::string_view expecting_field;
            rusty::PhantomData<T> ty;

            static FromStrVisitor<T> new_(std::string_view expecting) {
                return FromStrVisitor<T>{.expecting_field = std::string_view(expecting), .ty = rusty::PhantomData<T>{}};
            }
            rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const {
                return formatter.write_str(std::string_view(this->expecting_field));
            }
            template<typename E>
            rusty::Result<Value, E> visit_str(std::string_view s) {
                return Value::from_str(s).map_err([&](auto&& _err) -> E { return (E::custom) (std::forward<decltype(_err)>(_err)); });
            }
        };

        template<typename T>
        void nop_reserve(T _seq, size_t _n) {
        }

        // Extension trait Deserialize lowered to rusty_ext:: free functions
        namespace rusty_ext {
            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize_in_place

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize_in_place

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize_in_place

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize_in_place

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize_in_place

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize_in_place

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize_in_place

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

            // Rust-only extension method skipped (no receiver): deserialize

        }


    }

    // Extension trait DeserializeOwned lowered to rusty_ext:: free functions
    namespace rusty_ext {
    }

    // Extension trait DeserializeSeed lowered to rusty_ext:: free functions
    namespace rusty_ext {
        template<typename D, typename T>
        rusty::Result<T, typename D::Error> deserialize(rusty::PhantomData<T> self_, D deserializer) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return T::deserialize(std::move(deserializer));
        }

    }

    // Extension trait Expected lowered to rusty_ext:: free functions
    namespace rusty_ext {
        template<typename T>
        rusty::fmt::Result fmt(const T& self_, rusty::fmt::Formatter& formatter) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return self_.expecting(rusty::detail::deref_if_pointer_like(formatter));
        }

    }


}

namespace __private228 {

    using namespace ::private_;
    using namespace ::private_::content;

}

namespace ser {
    namespace fmt {}
    namespace impls {}
    namespace impossible {}

    namespace impls {

        size_t format_u8(uint8_t n, std::span<uint8_t> out) {
            if (n >= 100) {
                const auto d1 = static_cast<size_t>((((n % 100)) << 1));
                [&]() { static_cast<void>(n /= 100); return std::make_tuple(); }();
                out[0] = static_cast<uint8_t>(48) + n;
                out[1] = DEC_DIGITS_LUT[d1];
                out[2] = DEC_DIGITS_LUT[d1 + 1];
                return static_cast<size_t>(3);
            } else if (n >= 10) {
                const auto d1 = static_cast<size_t>((n << 1));
                out[0] = DEC_DIGITS_LUT[d1];
                out[1] = DEC_DIGITS_LUT[d1 + 1];
                return static_cast<size_t>(2);
            } else {
                out[0] = static_cast<uint8_t>(48) + n;
                return static_cast<size_t>(1);
            }
        }

        // Extension trait Serialize lowered to rusty_ext:: free functions
        namespace rusty_ext {
            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const bool& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.serialize_bool(self_);
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const ptrdiff_t& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.serialize_i64(static_cast<int64_t>(self_));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const int8_t& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.serialize_i8(self_);
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const int16_t& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.serialize_i16(self_);
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const int32_t& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.serialize_i32(self_);
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const __int128& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.serialize_i128(self_);
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const size_t& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.serialize_u64(static_cast<uint64_t>(self_));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const uint8_t& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.serialize_u8(self_);
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const uint16_t& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.serialize_u16(self_);
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const uint32_t& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.serialize_u32(self_);
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const unsigned __int128& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.serialize_u128(self_);
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const float& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.serialize_f32(self_);
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const double& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.serialize_f64(self_);
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const char32_t& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.serialize_char(self_);
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const std::string_view& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.serialize_str(std::string_view(self_));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::String& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.serialize_str(std::string_view(self_.as_str()));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::fmt::Arguments& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.collect_str(self_);
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Option<T>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return [&]() -> rusty::Result<typename S::Ok, typename S::Error> { auto&& _m = self_; if (_m.is_some()) { auto&& _mv0 = std::as_const(_m).unwrap(); const auto& value = _mv0; return serializer.serialize_some(rusty::detail::deref_if_pointer_like(value)); } if (_m.is_none()) { return serializer.serialize_none(); } return [&]() -> rusty::Result<typename S::Ok, typename S::Error> { rusty::intrinsics::unreachable(); }(); }();
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::PhantomData<T>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.serialize_unit_struct(std::string_view("PhantomData"));
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Vec<T>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.collect_seq(self_);
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::BTreeSet<T>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.collect_seq(self_);
            }

            template<typename S, typename T, typename H>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::HashSet<T, H>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.collect_seq(self_);
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::VecDeque<T>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.collect_seq(self_);
            }

            template<typename S, typename Idx>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::range<Idx>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                auto state = ({ auto&& _m = serializer.serialize_struct(std::string_view("Range"), static_cast<size_t>(2)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = state.serialize_field("start", rusty::detail::deref_if_pointer_like(self_.start));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                {
                    auto&& _m = state.serialize_field("end", rusty::detail::deref_if_pointer_like(self_.end_));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return state.end();
            }

            template<typename S, typename Idx>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::range_from<Idx>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                auto state = ({ auto&& _m = serializer.serialize_struct(std::string_view("RangeFrom"), static_cast<size_t>(1)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = state.serialize_field("start", rusty::detail::deref_if_pointer_like(self_.start));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return state.end();
            }

            template<typename S, typename Idx>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::range_inclusive<Idx>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                auto state = ({ auto&& _m = serializer.serialize_struct(std::string_view("RangeInclusive"), static_cast<size_t>(2)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = state.serialize_field("start", &(self_.start));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                {
                    auto&& _m = state.serialize_field("end", &(self_.end_));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return state.end();
            }

            template<typename S, typename Idx>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::range_to<Idx>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                auto state = ({ auto&& _m = serializer.serialize_struct(std::string_view("RangeTo"), static_cast<size_t>(1)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = state.serialize_field("end", rusty::detail::deref_if_pointer_like(self_.end));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return state.end();
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Bound<T>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return [&]() { auto&& _m = self_; return std::visit(overloaded { [&](const std::variant_alternative_t<0, std::remove_reference_t<decltype(_m)>>&) -> rusty::Result<typename S::Ok, typename S::Error> { return serializer.serialize_unit_variant(std::string_view("Bound"), static_cast<uint32_t>(0), std::string_view("Unbounded")); }, [&](const std::variant_alternative_t<1, std::remove_reference_t<decltype(_m)>>& _v) -> rusty::Result<typename S::Ok, typename S::Error> { const auto& value = _v._0; return serializer.serialize_newtype_variant(std::string_view("Bound"), static_cast<uint32_t>(1), std::string_view("Included"), rusty::detail::deref_if_pointer_like(value)); }, [&](const std::variant_alternative_t<2, std::remove_reference_t<decltype(_m)>>& _v) -> rusty::Result<typename S::Ok, typename S::Error> { const auto& value = _v._0; return serializer.serialize_newtype_variant(std::string_view("Bound"), static_cast<uint32_t>(2), std::string_view("Excluded"), rusty::detail::deref_if_pointer_like(value)); } }, _m); }();
            }

            template<typename S, typename K, typename V>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::BTreeMap<K, V>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.collect_map(self_);
            }

            template<typename S, typename K, typename V, typename H>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::HashMap<K, V, H>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return serializer.collect_map(self_);
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Box<T>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize((*(*(self_))), std::move(serializer));
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Rc<T>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize((rusty::detail::deref_if_pointer_like(*(self_))), std::move(serializer));
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Arc<T>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize((rusty::detail::deref_if_pointer_like(*(self_))), std::move(serializer));
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Cow& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize((rusty::detail::deref_if_pointer_like(self_)), std::move(serializer));
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Weak<T>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.upgrade(), std::move(serializer));
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::Weak<T>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.upgrade(), std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroI8& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.get(), std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroI16& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.get(), std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroI32& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.get(), std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroI64& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.get(), std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroI128& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.get(), std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroU8& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.get(), std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroU16& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.get(), std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroU32& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.get(), std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroU64& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.get(), std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::NonZeroU128& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.get(), std::move(serializer));
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Cell<T>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.get(), std::move(serializer));
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::RefCell<T>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return [&]() -> rusty::Result<typename S::Ok, typename S::Error> { auto&& _m = self_.try_borrow(); if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& value = _mv0; return ::ser::impls::rusty_ext::serialize(value, std::move(serializer)); } if (_m.is_err()) { return rusty::Result<typename S::Ok, typename S::Error>::Err(S::Error::custom("already mutably borrowed")); } return [&]() -> rusty::Result<typename S::Ok, typename S::Error> { rusty::intrinsics::unreachable(); }(); }();
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Mutex<T>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return [&]() -> rusty::Result<typename S::Ok, typename S::Error> { auto&& _m = self_.lock(); if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& locked = _mv0; return ::ser::impls::rusty_ext::serialize(locked, std::move(serializer)); } if (_m.is_err()) { return rusty::Result<typename S::Ok, typename S::Error>::Err(S::Error::custom("lock poison error while serializing")); } return [&]() -> rusty::Result<typename S::Ok, typename S::Error> { rusty::intrinsics::unreachable(); }(); }();
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::RwLock<T>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return [&]() -> rusty::Result<typename S::Ok, typename S::Error> { auto&& _m = self_.read(); if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& locked = _mv0; return ::ser::impls::rusty_ext::serialize(locked, std::move(serializer)); } if (_m.is_err()) { return rusty::Result<typename S::Ok, typename S::Error>::Err(S::Error::custom("lock poison error while serializing")); } return [&]() -> rusty::Result<typename S::Ok, typename S::Error> { rusty::intrinsics::unreachable(); }(); }();
            }

            template<typename S, typename T, typename E>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::Result<T, E>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return [&]() -> rusty::Result<typename S::Ok, typename S::Error> { auto&& _m = self_; if (_m.is_ok()) { auto&& _mv0 = std::as_const(_m).unwrap(); const auto& value = _mv0; return serializer.serialize_newtype_variant(std::string_view("Result"), static_cast<uint32_t>(0), std::string_view("Ok"), rusty::detail::deref_if_pointer_like(value)); } if (_m.is_err()) { auto&& _mv1 = std::as_const(_m).unwrap_err(); const auto& value = _mv1; return serializer.serialize_newtype_variant(std::string_view("Result"), static_cast<uint32_t>(1), std::string_view("Err"), rusty::detail::deref_if_pointer_like(value)); } return [&]() -> rusty::Result<typename S::Ok, typename S::Error> { rusty::intrinsics::unreachable(); }(); }();
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::time::Duration& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                auto state = ({ auto&& _m = serializer.serialize_struct(std::string_view("Duration"), static_cast<size_t>(2)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = state.serialize_field("secs", self_.as_secs());
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                {
                    auto&& _m = state.serialize_field("nanos", self_.subsec_nanos());
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return state.end();
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::time::SystemTime& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                const auto duration_since_epoch = ({ auto&& _m = self_.duration_since(::rusty::time::UNIX_EPOCH); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& duration_since_epoch = _mv;
_match_value.emplace(std::move(duration_since_epoch)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
return rusty::Result<typename S::Ok, typename S::Error>::Err(S::Error::custom("SystemTime must be later than UNIX_EPOCH")); } std::move(_match_value).value(); });
                auto state = ({ auto&& _m = serializer.serialize_struct(std::string_view("SystemTime"), static_cast<size_t>(2)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = state.serialize_field("secs_since_epoch", duration_since_epoch.as_secs());
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                {
                    auto&& _m = state.serialize_field("nanos_since_epoch", duration_since_epoch.subsec_nanos());
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return state.end();
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::net::IpAddr& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                if (serializer.is_human_readable()) {
                    return [&]() { auto&& _m = self_; return std::visit(overloaded { [&](const rusty::net::IpAddr_V4& _v) -> rusty::Result<typename S::Ok, typename S::Error> { const auto& a = _v._0; return ::ser::impls::rusty_ext::serialize(a, std::move(serializer)); }, [&](const rusty::net::IpAddr_V6& _v) -> rusty::Result<typename S::Ok, typename S::Error> { const auto& a = _v._0; return ::ser::impls::rusty_ext::serialize(a, std::move(serializer)); } }, _m); }();
                } else {
                    return [&]() { auto&& _m = self_; return std::visit(overloaded { [&](const rusty::net::IpAddr_V4& _v) -> rusty::Result<typename S::Ok, typename S::Error> { const auto& a = _v._0; return serializer.serialize_newtype_variant(std::string_view("IpAddr"), static_cast<uint32_t>(0), std::string_view("V4"), rusty::detail::deref_if_pointer_like(a)); }, [&](const rusty::net::IpAddr_V6& _v) -> rusty::Result<typename S::Ok, typename S::Error> { const auto& a = _v._0; return serializer.serialize_newtype_variant(std::string_view("IpAddr"), static_cast<uint32_t>(1), std::string_view("V6"), rusty::detail::deref_if_pointer_like(a)); } }, _m); }();
                }
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::net::Ipv4Addr& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                if (serializer.is_human_readable()) {
                    constexpr size_t MAX_LEN = static_cast<size_t>(15);
                    if (true) {
                        {
                            auto _m0 = &MAX_LEN;
                            auto&& _m1_tmp = rusty::len("101.102.103.104");
                            auto _m1 = &_m1_tmp;
                            auto _m_tuple = std::make_tuple(_m0, _m1);
                            bool _m_matched = false;
                            if (!_m_matched) {
                                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                                    const auto kind = rusty::panicking::AssertKind::Eq;
                                    [&]() -> rusty::Result<typename S::Ok, typename S::Error> { rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None); }();
                                }
                                _m_matched = true;
                            }
                        }
                    }
                    auto buf = [](auto _seed) { std::array<std::string_view, rusty::sanitize_array_capacity<MAX_LEN>()> _repeat{}; _repeat.fill(_seed); return _repeat; }(static_cast<uint8_t>(46));
                    auto written = format_u8(self_.octets()[0], buf);
                    for (auto&& oct : rusty::for_in(rusty::iter(rusty::slice_from(self_.octets(), 1)))) {
                        [&]() { static_cast<void>(written += format_u8(std::move(rusty::detail::deref_if_pointer_like(oct)), rusty::slice_from(buf, written + 1)) + 1); return std::make_tuple(); }();
                    }
                    auto buf_shadow1 = rusty::str_runtime::from_utf8_unchecked(rusty::slice_to(buf, written));
                    return serializer.serialize_str(std::string_view(buf_shadow1));
                } else {
                    return ::ser::impls::rusty_ext::serialize(self_.octets(), std::move(serializer));
                }
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::net::Ipv6Addr& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                if (serializer.is_human_readable()) {
                    constexpr size_t MAX_LEN = static_cast<size_t>(39);
                    if (true) {
                        {
                            auto _m0 = &MAX_LEN;
                            auto&& _m1_tmp = rusty::len("1001:1002:1003:1004:1005:1006:1007:1008");
                            auto _m1 = &_m1_tmp;
                            auto _m_tuple = std::make_tuple(_m0, _m1);
                            bool _m_matched = false;
                            if (!_m_matched) {
                                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                                    const auto kind = rusty::panicking::AssertKind::Eq;
                                    [&]() -> rusty::Result<typename S::Ok, typename S::Error> { rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None); }();
                                }
                                _m_matched = true;
                            }
                        }
                    }
                    {
                        auto buffer = [](auto _seed) { std::array<uint8_t, rusty::sanitize_array_capacity<MAX_LEN>()> _repeat{}; _repeat.fill(static_cast<uint8_t>(_seed)); return _repeat; }(static_cast<uint8_t>(0));
                        auto writer = std::conditional_t<true, ::format::Buf, S>::new_(buffer);
                        rusty::io::write_fmt(((&writer)), std::format("{0}", rusty::to_string(self_))).unwrap();
                        return serializer.serialize_str(writer.as_str());
                    }
                } else {
                    return ::ser::impls::rusty_ext::serialize(self_.octets(), std::move(serializer));
                }
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::net::SocketAddr& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                if (serializer.is_human_readable()) {
                    return [&]() { auto&& _m = self_; return std::visit(overloaded { [&](const rusty::net::SocketAddr_V4& _v) -> rusty::Result<typename S::Ok, typename S::Error> { const auto& addr = _v._0; return ::ser::impls::rusty_ext::serialize(addr, std::move(serializer)); }, [&](const rusty::net::SocketAddr_V6& _v) -> rusty::Result<typename S::Ok, typename S::Error> { const auto& addr = _v._0; return ::ser::impls::rusty_ext::serialize(addr, std::move(serializer)); } }, _m); }();
                } else {
                    return [&]() { auto&& _m = self_; return std::visit(overloaded { [&](const rusty::net::SocketAddr_V4& _v) -> rusty::Result<typename S::Ok, typename S::Error> { const auto& addr = _v._0; return serializer.serialize_newtype_variant(std::string_view("SocketAddr"), static_cast<uint32_t>(0), std::string_view("V4"), rusty::detail::deref_if_pointer_like(addr)); }, [&](const rusty::net::SocketAddr_V6& _v) -> rusty::Result<typename S::Ok, typename S::Error> { const auto& addr = _v._0; return serializer.serialize_newtype_variant(std::string_view("SocketAddr"), static_cast<uint32_t>(1), std::string_view("V6"), rusty::detail::deref_if_pointer_like(addr)); } }, _m); }();
                }
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::net::SocketAddrV4& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                if (serializer.is_human_readable()) {
                    constexpr size_t MAX_LEN = static_cast<size_t>(21);
                    if (true) {
                        {
                            auto _m0 = &MAX_LEN;
                            auto&& _m1_tmp = rusty::len("101.102.103.104:65000");
                            auto _m1 = &_m1_tmp;
                            auto _m_tuple = std::make_tuple(_m0, _m1);
                            bool _m_matched = false;
                            if (!_m_matched) {
                                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                                    const auto kind = rusty::panicking::AssertKind::Eq;
                                    [&]() -> rusty::Result<typename S::Ok, typename S::Error> { rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None); }();
                                }
                                _m_matched = true;
                            }
                        }
                    }
                    {
                        auto buffer = [](auto _seed) { std::array<uint8_t, rusty::sanitize_array_capacity<MAX_LEN>()> _repeat{}; _repeat.fill(static_cast<uint8_t>(_seed)); return _repeat; }(static_cast<uint8_t>(0));
                        auto writer = std::conditional_t<true, ::format::Buf, S>::new_(buffer);
                        rusty::io::write_fmt(((&writer)), std::format("{0}", rusty::to_string(self_))).unwrap();
                        return serializer.serialize_str(writer.as_str());
                    }
                } else {
                    return ::ser::impls::rusty_ext::serialize(std::make_tuple(self_.ip(), self_.port()), std::move(serializer));
                }
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::net::SocketAddrV6& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                if (serializer.is_human_readable()) {
                    constexpr size_t MAX_LEN = static_cast<size_t>(58);
                    if (true) {
                        {
                            auto _m0 = &MAX_LEN;
                            auto&& _m1_tmp = rusty::len("[1001:1002:1003:1004:1005:1006:1007:1008%4294967295]:65000");
                            auto _m1 = &_m1_tmp;
                            auto _m_tuple = std::make_tuple(_m0, _m1);
                            bool _m_matched = false;
                            if (!_m_matched) {
                                auto&& left_val = std::get<0>(rusty::detail::deref_if_pointer(_m_tuple));
                                auto&& right_val = std::get<1>(rusty::detail::deref_if_pointer(_m_tuple));
                                if (!(rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val))) {
                                    const auto kind = rusty::panicking::AssertKind::Eq;
                                    [&]() -> rusty::Result<typename S::Ok, typename S::Error> { rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None); }();
                                }
                                _m_matched = true;
                            }
                        }
                    }
                    {
                        auto buffer = [](auto _seed) { std::array<uint8_t, rusty::sanitize_array_capacity<MAX_LEN>()> _repeat{}; _repeat.fill(static_cast<uint8_t>(_seed)); return _repeat; }(static_cast<uint8_t>(0));
                        auto writer = std::conditional_t<true, ::format::Buf, S>::new_(buffer);
                        rusty::io::write_fmt(((&writer)), std::format("{0}", rusty::to_string(self_))).unwrap();
                        return serializer.serialize_str(writer.as_str());
                    }
                } else {
                    return ::ser::impls::rusty_ext::serialize(std::make_tuple(self_.ip(), self_.port()), std::move(serializer));
                }
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::path::PathBuf& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.as_path(), std::move(serializer));
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::Wrapping<T>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_._0, std::move(serializer));
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::num::Saturating<T>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_._0, std::move(serializer));
            }

            template<typename S, typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::cmp::Reverse<T>& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_._0, std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::atomic::AtomicBool& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.load(Ordering::Relaxed), std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::atomic::AtomicI8& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.load(Ordering::Relaxed), std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::atomic::AtomicI16& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.load(Ordering::Relaxed), std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::atomic::AtomicI32& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.load(Ordering::Relaxed), std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::atomic::AtomicIsize& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.load(Ordering::Relaxed), std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::atomic::AtomicU8& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.load(Ordering::Relaxed), std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::atomic::AtomicU16& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.load(Ordering::Relaxed), std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::atomic::AtomicU32& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.load(Ordering::Relaxed), std::move(serializer));
            }

            template<typename S>
            rusty::Result<typename S::Ok, typename S::Error> serialize(const rusty::sync::atomic::AtomicUsize& self_, S serializer) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return ::ser::impls::rusty_ext::serialize(self_.load(Ordering::Relaxed), std::move(serializer));
            }

        }


    }

    namespace impossible {

    }

    namespace fmt {

        // Extension trait Error lowered to rusty_ext:: free functions
        namespace rusty_ext {
            // Rust-only extension method skipped (no receiver): custom

        }


    }

    template<typename I>
    rusty::Option<size_t> iterator_len_hint(const I& iter) {
        return [&]() { auto&& _m = iter.size_hint(); return std::visit(overloaded { [&](auto&&) -> rusty::Option<size_t> { return [&]() -> rusty::Option<size_t> { rusty::intrinsics::unreachable(); }(); }, [&](auto&&) -> rusty::Option<size_t> { return rusty::Option<size_t>(rusty::None); } }, std::move(_m)); }();
    }

}


namespace format {
    Buf Buf::new_(std::span<uint8_t> bytes) {
        return Buf{.bytes = bytes, .offset = static_cast<size_t>(0)};
    }
}

namespace format {
    std::string_view Buf::as_str() const {
        const auto slice = rusty::slice_to(this->bytes, this->offset);
        // @unsafe
        {
            return rusty::str_runtime::from_utf8_unchecked(slice);
        }
    }
}

namespace format {
    rusty::fmt::Result Buf::write_str(std::string_view s) {
        if ((this->offset + rusty::len(s)) > rusty::len(this->bytes)) {
            return rusty::fmt::Result::Err(rusty::fmt::Error{});
        } else {
            rusty::clone_from_slice(rusty::slice(this->bytes, this->offset, this->offset + rusty::len(s)), rusty::as_bytes(s));
            [&]() { static_cast<void>(this->offset += rusty::len(s)); return std::make_tuple(); }();
            return rusty::fmt::Result::Ok(std::make_tuple());
        }
    }
}

namespace private_::doc {
        rusty::fmt::Result Error::fmt(rusty::fmt::Formatter& _arg1) const {
            return [&]() -> rusty::fmt::Result { rusty::panicking::panic("not implemented"); }();
        }
}

namespace private_::doc {
        template<typename T>
        Error Error::custom(T _arg0) {
            return [&]() -> Error { rusty::panicking::panic("not implemented"); }();
        }
}

namespace private_::doc {
        std::string_view Error::description() const {
            return [&]() -> std::string_view { rusty::panicking::panic("not implemented"); }();
        }
}

namespace de {
    rusty::fmt::Result OneOf::fmt(rusty::fmt::Formatter& formatter) const {
        return ({ auto&& _m = rusty::len(this->names); std::optional<rusty::fmt::Result> _match_value; bool _m_matched = false; if (!_m_matched && (_m == 0)) { [&]() -> rusty::fmt::Result { rusty::panicking::panic("explicit panic"); }(); _m_matched = true; } if (!_m_matched && (_m == 1)) { _match_value.emplace(std::move(rusty::write_fmt(formatter, std::format("`{0}`", rusty::to_string(this->names[0]))))); _m_matched = true; } if (!_m_matched && (_m == 2)) { _match_value.emplace(std::move(rusty::write_fmt(formatter, std::format("`{0}` or `{1}`", rusty::to_string(this->names[0]), rusty::to_string(this->names[1]))))); _m_matched = true; } if (!_m_matched) { _match_value.emplace(std::move([&]() -> rusty::fmt::Result { {
    auto&& _m = formatter.write_str(std::string_view("one of "));
    bool _m_matched = false;
    if (!_m_matched) {
        if (_m.is_ok()) {
            auto&& _mv0 = _m.unwrap();
            auto&& val = _mv0;
            val;
            _m_matched = true;
        }
    }
    if (!_m_matched) {
        if (_m.is_err()) {
            auto&& _mv1 = _m.unwrap_err();
            auto&& err = _mv1;
            return rusty::fmt::Result::Err(std::move(err));
            _m_matched = true;
        }
    }
}
for (auto&& _for_item : rusty::for_in(rusty::enumerate(rusty::iter(this->names)))) {
    auto&& i = std::get<0>(rusty::detail::deref_if_pointer(_for_item));
    auto&& alt = std::get<1>(rusty::detail::deref_if_pointer(_for_item));
    if (i > 0) {
        {
            auto&& _m = formatter.write_str(std::string_view(", "));
            bool _m_matched = false;
            if (!_m_matched) {
                if (_m.is_ok()) {
                    auto&& _mv0 = _m.unwrap();
                    auto&& val = _mv0;
                    val;
                    _m_matched = true;
                }
            }
            if (!_m_matched) {
                if (_m.is_err()) {
                    auto&& _mv1 = _m.unwrap_err();
                    auto&& err = _mv1;
                    return rusty::fmt::Result::Err(std::move(err));
                    _m_matched = true;
                }
            }
        }
    }
    {
        auto&& _m = rusty::write_fmt(formatter, std::format("`{0}`", rusty::to_string(alt)));
        bool _m_matched = false;
        if (!_m_matched) {
            if (_m.is_ok()) {
                auto&& _mv0 = _m.unwrap();
                auto&& val = _mv0;
                val;
                _m_matched = true;
            }
        }
        if (!_m_matched) {
            if (_m.is_err()) {
                auto&& _mv1 = _m.unwrap_err();
                auto&& err = _mv1;
                return rusty::fmt::Result::Err(std::move(err));
                _m_matched = true;
            }
        }
    }
}
return rusty::fmt::Result::Ok(std::make_tuple()); }())); _m_matched = true; } if (!_m_matched) { rusty::intrinsics::unreachable(); } std::move(_match_value).value(); });
    }
}

namespace de {
    struct LookForDecimalPoint {
        rusty::fmt::Formatter& formatter;
        bool has_decimal_point;
    };
    rusty::fmt::Result WithDecimalPoint::fmt(rusty::fmt::Formatter& formatter) const {
        if (rusty::is_finite(this->_0)) {
            auto writer = LookForDecimalPoint{.formatter = formatter, .has_decimal_point = false};
            {
                auto&& _m = rusty::write_fmt(writer, std::format("{0}", rusty::to_string(this->_0)));
                bool _m_matched = false;
                if (!_m_matched) {
                    if (_m.is_ok()) {
                        auto&& _mv0 = _m.unwrap();
                        auto&& val = _mv0;
                        val;
                        _m_matched = true;
                    }
                }
                if (!_m_matched) {
                    if (_m.is_err()) {
                        auto&& _mv1 = _m.unwrap_err();
                        auto&& err = _mv1;
                        return rusty::fmt::Result::Err(std::move(err));
                        _m_matched = true;
                    }
                }
            }
            if (!writer.has_decimal_point) {
                {
                    auto&& _m = formatter.write_str(std::string_view(".0"));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::fmt::Result::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
            }
        } else {
            {
                auto&& _m = rusty::write_fmt(formatter, std::format("{0}", rusty::to_string(this->_0)));
                bool _m_matched = false;
                if (!_m_matched) {
                    if (_m.is_ok()) {
                        auto&& _mv0 = _m.unwrap();
                        auto&& val = _mv0;
                        val;
                        _m_matched = true;
                    }
                }
                if (!_m_matched) {
                    if (_m.is_err()) {
                        auto&& _mv1 = _m.unwrap_err();
                        auto&& err = _mv1;
                        return rusty::fmt::Result::Err(std::move(err));
                        _m_matched = true;
                    }
                }
            }
        }
        return rusty::fmt::Result::Ok(std::make_tuple());
    }
}

namespace de {
    Unexpected Unexpected::clone() const {
        return {};
    }
}

namespace de {
    bool Unexpected::operator==(const Unexpected& other) const {
        const auto __self_discr = rusty::intrinsics::discriminant_value((*this));
        const auto __arg1_discr = rusty::intrinsics::discriminant_value(other);
        return (__self_discr == __arg1_discr) && [&]() -> bool { auto&& _m0 = (*this); auto&& _m1 = other; if (std::holds_alternative<Unexpected_Bool>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<Unexpected_Bool>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = std::get<Unexpected_Bool>(rusty::detail::deref_if_pointer(_m0))._0; auto&& __arg1_0 = std::get<Unexpected_Bool>(rusty::detail::deref_if_pointer(_m1))._0; return __self_0 == __arg1_0; } if (std::holds_alternative<Unexpected_Unsigned>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<Unexpected_Unsigned>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = std::get<Unexpected_Unsigned>(rusty::detail::deref_if_pointer(_m0))._0; auto&& __arg1_0 = std::get<Unexpected_Unsigned>(rusty::detail::deref_if_pointer(_m1))._0; return __self_0 == __arg1_0; } if (std::holds_alternative<Unexpected_Signed>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<Unexpected_Signed>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = std::get<Unexpected_Signed>(rusty::detail::deref_if_pointer(_m0))._0; auto&& __arg1_0 = std::get<Unexpected_Signed>(rusty::detail::deref_if_pointer(_m1))._0; return __self_0 == __arg1_0; } if (std::holds_alternative<Unexpected_Float>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<Unexpected_Float>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = std::get<Unexpected_Float>(rusty::detail::deref_if_pointer(_m0))._0; auto&& __arg1_0 = std::get<Unexpected_Float>(rusty::detail::deref_if_pointer(_m1))._0; return __self_0 == __arg1_0; } if (std::holds_alternative<Unexpected_Char>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<Unexpected_Char>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = std::get<Unexpected_Char>(rusty::detail::deref_if_pointer(_m0))._0; auto&& __arg1_0 = std::get<Unexpected_Char>(rusty::detail::deref_if_pointer(_m1))._0; return __self_0 == __arg1_0; } if (std::holds_alternative<Unexpected_Str>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<Unexpected_Str>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = std::get<Unexpected_Str>(rusty::detail::deref_if_pointer(_m0))._0; auto&& __arg1_0 = std::get<Unexpected_Str>(rusty::detail::deref_if_pointer(_m1))._0; return __self_0 == __arg1_0; } if (std::holds_alternative<Unexpected_Bytes>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<Unexpected_Bytes>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = std::get<Unexpected_Bytes>(rusty::detail::deref_if_pointer(_m0))._0; auto&& __arg1_0 = std::get<Unexpected_Bytes>(rusty::detail::deref_if_pointer(_m1))._0; return __self_0 == __arg1_0; } if (std::holds_alternative<Unexpected_Other>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<Unexpected_Other>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = std::get<Unexpected_Other>(rusty::detail::deref_if_pointer(_m0))._0; auto&& __arg1_0 = std::get<Unexpected_Other>(rusty::detail::deref_if_pointer(_m1))._0; return __self_0 == __arg1_0; } if (true) { return true; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
    }
}

namespace de {
    rusty::fmt::Result Unexpected::fmt(rusty::fmt::Formatter& formatter) const {
        // Rust-only namespace import skipped for type path: using namespace de::Unexpected;
        return [&]() { auto&& _m = (*this); return std::visit(overloaded { [&](const Unexpected_Bool& _v) -> rusty::fmt::Result { auto&& b = _v._0; return rusty::write_fmt(formatter, std::format("boolean `{0}`", rusty::to_string(b))); }, [&](const Unexpected_Unsigned& _v) -> rusty::fmt::Result { auto&& i = _v._0; return rusty::write_fmt(formatter, std::format("integer `{0}`", rusty::to_string(i))); }, [&](const Unexpected_Signed& _v) -> rusty::fmt::Result { auto&& i = _v._0; return rusty::write_fmt(formatter, std::format("integer `{0}`", rusty::to_string(i))); }, [&](const Unexpected_Float& _v) -> rusty::fmt::Result { auto&& f = _v._0; return rusty::write_fmt(formatter, std::format("floating point `{0}`", rusty::to_string(WithDecimalPoint(std::move(f))))); }, [&](const Unexpected_Char& _v) -> rusty::fmt::Result { auto&& c = _v._0; return rusty::write_fmt(formatter, std::format("character `{0}`", rusty::to_string(c))); }, [&](const Unexpected_Str& _v) -> rusty::fmt::Result { auto&& s = _v._0; return rusty::write_fmt(formatter, std::format("string {0}", rusty::to_debug_string(s))); }, [&](const Unexpected_Bytes& _v) -> rusty::fmt::Result {  return formatter.write_str(std::string_view("byte array")); }, [&](const Unexpected_Unit&) -> rusty::fmt::Result { return formatter.write_str(std::string_view("unit value")); }, [&](const Unexpected_Option&) -> rusty::fmt::Result { return formatter.write_str(std::string_view("Option value")); }, [&](const Unexpected_NewtypeStruct&) -> rusty::fmt::Result { return formatter.write_str(std::string_view("newtype struct")); }, [&](const Unexpected_Seq&) -> rusty::fmt::Result { return formatter.write_str(std::string_view("sequence")); }, [&](const Unexpected_Map&) -> rusty::fmt::Result { return formatter.write_str(std::string_view("map")); }, [&](const Unexpected_Enum&) -> rusty::fmt::Result { return formatter.write_str(std::string_view("enum")); }, [&](const Unexpected_UnitVariant&) -> rusty::fmt::Result { return formatter.write_str(std::string_view("unit variant")); }, [&](const Unexpected_NewtypeVariant&) -> rusty::fmt::Result { return formatter.write_str(std::string_view("newtype variant")); }, [&](const Unexpected_TupleVariant&) -> rusty::fmt::Result { return formatter.write_str(std::string_view("tuple variant")); }, [&](const Unexpected_StructVariant&) -> rusty::fmt::Result { return formatter.write_str(std::string_view("struct variant")); }, [&](const Unexpected_Other& _v) -> rusty::fmt::Result { auto&& other = _v._0; return formatter.write_str(std::move(rusty::to_string_view(other))); } }, _m); }();
    }
}

namespace de::value {
        Error Error::clone() const {
            return Error(rusty::clone(this->err));
        }
}

namespace de::value {
        bool Error::operator==(const Error& other) const {
            return this->err == other.err;
        }
}

namespace de::value {
        template<typename T>
        Error Error::custom(T msg) {
            return Error(rusty::into_boxed_str(rusty::to_string(msg)));
        }
}

namespace de::value {
        Error Error::invalid_type(::de::Unexpected unexp, const auto& exp) {
            return Error::custom(std::format("invalid type: {0}, expected {1}", rusty::to_string(unexp), rusty::to_string(exp)));
        }
}

namespace de::value {
        Error Error::invalid_value(::de::Unexpected unexp, const auto& exp) {
            return Error::custom(std::format("invalid value: {0}, expected {1}", rusty::to_string(unexp), rusty::to_string(exp)));
        }
}

namespace de::value {
        Error Error::invalid_length(size_t len, const auto& exp) {
            return Error::custom(std::format("invalid length {0}, expected {1}", len, rusty::to_string(exp)));
        }
}

namespace de::value {
        Error Error::unknown_variant(std::string_view variant, std::span<const std::string_view> expected) {
            if (rusty::is_empty(expected)) {
                return Error::custom(std::format("unknown variant `{0}`, there are no variants", rusty::to_string(variant)));
            } else {
                return Error::custom(std::format("unknown variant `{0}`, expected {1}", rusty::to_string(variant), rusty::to_string(::de::OneOf{.names = expected})));
            }
        }
}

namespace de::value {
        Error Error::unknown_field(std::string_view field, std::span<const std::string_view> expected) {
            if (rusty::is_empty(expected)) {
                return Error::custom(std::format("unknown field `{0}`, there are no fields", rusty::to_string(field)));
            } else {
                return Error::custom(std::format("unknown field `{0}`, expected {1}", rusty::to_string(field), rusty::to_string(::de::OneOf{.names = expected})));
            }
        }
}

namespace de::value {
        Error Error::missing_field(std::string_view field) {
            return Error::custom(std::format("missing field `{0}`", rusty::to_string(field)));
        }
}

namespace de::value {
        Error Error::duplicate_field(std::string_view field) {
            return Error::custom(std::format("duplicate field `{0}`", rusty::to_string(field)));
        }
}

namespace de::value {
        rusty::fmt::Result Error::fmt(rusty::fmt::Formatter& formatter) const {
            return formatter.write_str(std::string_view(this->err));
        }
}

namespace de::value {
        std::string_view Error::description() const {
            return std::string_view(this->err);
        }
}

namespace de::value {
        rusty::fmt::Result ExpectedInSeq::fmt(rusty::fmt::Formatter& formatter) const {
            if (this->_0 == 1) {
                return formatter.write_str(std::string_view("1 element in sequence"));
            } else {
                return rusty::write_fmt(formatter, std::format("{0} elements in sequence", rusty::to_string(this->_0)));
            }
        }
}

namespace de::value {
        rusty::fmt::Result ExpectedInMap::fmt(rusty::fmt::Formatter& formatter) const {
            if (this->_0 == 1) {
                return formatter.write_str(std::string_view("1 element in map"));
            } else {
                return rusty::write_fmt(formatter, std::format("{0} elements in map", rusty::to_string(this->_0)));
            }
        }
}

namespace de::ignored_any {
        IgnoredAny IgnoredAny::clone() const {
            return {};
        }
}

namespace de::ignored_any {
        rusty::fmt::Result IgnoredAny::fmt(rusty::fmt::Formatter& f) const {
            return f.write_str("IgnoredAny");
        }
}

namespace de::ignored_any {
        IgnoredAny IgnoredAny::default_() {
            return IgnoredAny{};
        }
}

namespace de::ignored_any {
        bool IgnoredAny::operator==(const IgnoredAny& other) const {
            return true;
        }
}

namespace de::ignored_any {
        rusty::fmt::Result IgnoredAny::expecting(rusty::fmt::Formatter& formatter) const {
            return formatter.write_str(std::string_view("anything at all"));
        }
}

namespace de::ignored_any {
        template<typename E>
        rusty::Result<IgnoredAny, E> IgnoredAny::visit_bool(bool x) {
            static_cast<void>(std::move(x));
            return rusty::Result<IgnoredAny, E>::Ok(IgnoredAny{});
        }
}

namespace de::ignored_any {
        template<typename E>
        rusty::Result<IgnoredAny, E> IgnoredAny::visit_i64(int64_t x) {
            static_cast<void>(std::move(x));
            return rusty::Result<IgnoredAny, E>::Ok(IgnoredAny{});
        }
}

namespace de::ignored_any {
        template<typename E>
        rusty::Result<IgnoredAny, E> IgnoredAny::visit_i128(__int128 x) {
            static_cast<void>(std::move(x));
            return rusty::Result<IgnoredAny, E>::Ok(IgnoredAny{});
        }
}

namespace de::ignored_any {
        template<typename E>
        rusty::Result<IgnoredAny, E> IgnoredAny::visit_u64(uint64_t x) {
            static_cast<void>(std::move(x));
            return rusty::Result<IgnoredAny, E>::Ok(IgnoredAny{});
        }
}

namespace de::ignored_any {
        template<typename E>
        rusty::Result<IgnoredAny, E> IgnoredAny::visit_u128(unsigned __int128 x) {
            static_cast<void>(std::move(x));
            return rusty::Result<IgnoredAny, E>::Ok(IgnoredAny{});
        }
}

namespace de::ignored_any {
        template<typename E>
        rusty::Result<IgnoredAny, E> IgnoredAny::visit_f64(double x) {
            static_cast<void>(std::move(x));
            return rusty::Result<IgnoredAny, E>::Ok(IgnoredAny{});
        }
}

namespace de::ignored_any {
        template<typename E>
        rusty::Result<IgnoredAny, E> IgnoredAny::visit_str(std::string_view s) {
            static_cast<void>(s);
            return rusty::Result<IgnoredAny, E>::Ok(IgnoredAny{});
        }
}

namespace de::ignored_any {
        template<typename E>
        rusty::Result<IgnoredAny, E> IgnoredAny::visit_none() {
            return rusty::Result<IgnoredAny, E>::Ok(IgnoredAny{});
        }
}

namespace de::ignored_any {
        template<typename D>
        rusty::Result<IgnoredAny, typename D::Error> IgnoredAny::visit_some(D deserializer) {
            return std::conditional_t<true, IgnoredAny, D>::deserialize(std::move(deserializer));
        }
}

namespace de::ignored_any {
        template<typename D>
        rusty::Result<IgnoredAny, typename D::Error> IgnoredAny::visit_newtype_struct(D deserializer) {
            return std::conditional_t<true, IgnoredAny, D>::deserialize(std::move(deserializer));
        }
}

namespace de::ignored_any {
        template<typename E>
        rusty::Result<IgnoredAny, E> IgnoredAny::visit_unit() {
            return rusty::Result<IgnoredAny, E>::Ok(IgnoredAny{});
        }
}

namespace de::ignored_any {
        template<typename A>
        rusty::Result<IgnoredAny, typename A::Error> IgnoredAny::visit_seq(A seq) {
            while (true) {
                auto&& _whilelet = ({ auto&& _m = seq.next_element(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<IgnoredAny, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                if (!(_whilelet.is_some())) { break; }
                auto IgnoredAny = _whilelet.unwrap();
            }
            return rusty::Result<IgnoredAny, typename A::Error>::Ok(IgnoredAny{});
        }
}

namespace de::ignored_any {
        template<typename A>
        rusty::Result<IgnoredAny, typename A::Error> IgnoredAny::visit_map(A map) {
            while (true) {
                auto&& _whilelet = ({ auto&& _m = map.next_entry(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<IgnoredAny, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                if (!(_whilelet.is_some())) { break; }
            }
            return rusty::Result<IgnoredAny, typename A::Error>::Ok(IgnoredAny{});
        }
}

namespace de::ignored_any {
        template<typename E>
        rusty::Result<IgnoredAny, E> IgnoredAny::visit_bytes(std::span<const uint8_t> bytes) {
            static_cast<void>(bytes);
            return rusty::Result<IgnoredAny, E>::Ok(IgnoredAny{});
        }
}

namespace de::ignored_any {
        template<typename A>
        rusty::Result<IgnoredAny, typename A::Error> IgnoredAny::visit_enum(A data) {
            return ({ auto&& _m = data.template variant<IgnoredAny>(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<IgnoredAny, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); })._1.newtype_variant();
        }
}

namespace de::ignored_any {
        template<typename D>
        auto IgnoredAny::deserialize(D deserializer) {
            return deserializer.deserialize_ignored_any(IgnoredAny{});
        }
}

namespace de::ignored_any {
        template<typename D>
        auto IgnoredAny::deserialize_in_place(D deserializer, IgnoredAny& place) {
            place = ({ auto&& _m = std::conditional_t<true, IgnoredAny, D>::deserialize(std::move(deserializer)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<std::tuple<>, typename D::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
            return rusty::Result<std::tuple<>, typename D::Error>::Ok(std::make_tuple());
        }
}

namespace de::impls {
        rusty::fmt::Result UnitVisitor::expecting(rusty::fmt::Formatter& formatter) const {
            return formatter.write_str(std::string_view("unit"));
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<std::tuple<>, E> UnitVisitor::visit_unit() {
            return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
        }
}

namespace de::impls {
        rusty::fmt::Result BoolVisitor::expecting(rusty::fmt::Formatter& formatter) const {
            return formatter.write_str(std::string_view("a boolean"));
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<bool, E> BoolVisitor::visit_bool(bool v) {
            return rusty::Result<bool, E>::Ok(std::move(v));
        }
}

namespace de::impls {
        rusty::fmt::Result CharVisitor::expecting(rusty::fmt::Formatter& formatter) const {
            return formatter.write_str(std::string_view("a character"));
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<char32_t, E> CharVisitor::visit_char(char32_t v) {
            return rusty::Result<char32_t, E>::Ok(std::move(v));
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<char32_t, E> CharVisitor::visit_str(std::string_view v) {
            auto iter = rusty::str_runtime::chars(v);
            return [&]() -> rusty::Result<char32_t, E> { auto&& _m0 = iter.next(); auto&& _m1 = iter.next(); if (_m0.is_some() && _m1.is_none()) { auto&& c = std::as_const(_m0).unwrap(); return rusty::Result<char32_t, E>::Ok(std::move(c)); } if (true) { return rusty::Result<char32_t, E>::Err(E::invalid_value(de::Unexpected_Str{v}, (*this))); } return [&]() -> rusty::Result<char32_t, E> { rusty::intrinsics::unreachable(); }(); }();
        }
}

namespace de::impls {
        rusty::fmt::Result StringVisitor::expecting(rusty::fmt::Formatter& formatter) const {
            return formatter.write_str(std::string_view("a string"));
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<rusty::String, E> StringVisitor::visit_str(std::string_view v) {
            return rusty::Result<rusty::String, E>::Ok(rusty::String::from(v));
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<rusty::String, E> StringVisitor::visit_string(rusty::String v) {
            return rusty::Result<rusty::String, E>::Ok(std::move(v));
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<rusty::String, E> StringVisitor::visit_bytes(std::span<const uint8_t> v) {
            return [&]() -> rusty::Result<rusty::String, E> { auto&& _m = rusty::str_runtime::from_utf8(v); if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& s = _mv0; return rusty::Result<rusty::String, E>::Ok(rusty::to_owned(std::move(s))); } if (_m.is_err()) { return rusty::Result<rusty::String, E>::Err(E::invalid_value(de::Unexpected_Bytes{v}, (*this))); } return [&]() -> rusty::Result<rusty::String, E> { rusty::intrinsics::unreachable(); }(); }();
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<rusty::String, E> StringVisitor::visit_byte_buf(rusty::Vec<uint8_t> v) {
            return [&]() -> rusty::Result<rusty::String, E> { auto&& _m = std::conditional_t<true, rusty::String, E>::from_utf8(std::move(v)); if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& s = _mv0; return rusty::Result<rusty::String, E>::Ok(std::move(s)); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& e = _mv1; return rusty::Result<rusty::String, E>::Err(E::invalid_value(de::Unexpected_Bytes{e.into_bytes()}, (*this))); } return [&]() -> rusty::Result<rusty::String, E> { rusty::intrinsics::unreachable(); }(); }();
        }
}

namespace de::impls {
        rusty::fmt::Result StringInPlaceVisitor::expecting(rusty::fmt::Formatter& formatter) const {
            return formatter.write_str(std::string_view("a string"));
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<std::tuple<>, E> StringInPlaceVisitor::visit_str(std::string_view v) {
            this->_0.clear();
            this->_0.push_str(v);
            return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<std::tuple<>, E> StringInPlaceVisitor::visit_string(rusty::String v) {
            this->_0 = v;
            return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<std::tuple<>, E> StringInPlaceVisitor::visit_bytes(std::span<const uint8_t> v) {
            return [&]() -> rusty::Result<std::tuple<>, E> { auto&& _m = rusty::str_runtime::from_utf8(v); if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& s = _mv0; return [&]() -> rusty::Result<std::tuple<>, E> { this->_0.clear();
this->_0.push_str(std::move(s));
return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple()); }(); } if (_m.is_err()) { return rusty::Result<std::tuple<>, E>::Err(E::invalid_value(de::Unexpected_Bytes{v}, (*this))); } return [&]() -> rusty::Result<std::tuple<>, E> { rusty::intrinsics::unreachable(); }(); }();
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<std::tuple<>, E> StringInPlaceVisitor::visit_byte_buf(rusty::Vec<uint8_t> v) {
            return [&]() -> rusty::Result<std::tuple<>, E> { auto&& _m = std::conditional_t<true, rusty::String, E>::from_utf8(std::move(v)); if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& s = _mv0; return [&]() -> rusty::Result<std::tuple<>, E> { this->_0 = s;
return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple()); }(); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& e = _mv1; return rusty::Result<std::tuple<>, E>::Err(E::invalid_value(de::Unexpected_Bytes{e.into_bytes()}, (*this))); } return [&]() -> rusty::Result<std::tuple<>, E> { rusty::intrinsics::unreachable(); }(); }();
        }
}

namespace de::impls {
        rusty::fmt::Result StrVisitor::expecting(rusty::fmt::Formatter& formatter) const {
            return formatter.write_str(std::string_view("a borrowed string"));
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<std::string_view, E> StrVisitor::visit_borrowed_str(std::string_view v) {
            return rusty::Result<std::string_view, E>::Ok(std::string_view(v));
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<std::string_view, E> StrVisitor::visit_borrowed_bytes(std::span<const uint8_t> v) {
            return rusty::str_runtime::from_utf8(v).map_err([&](auto&& _err) -> E { return ([&](auto _closure_wild0) { return E::invalid_value(de::Unexpected_Bytes{v}, (*this)); }) (std::forward<decltype(_err)>(_err)); });
        }
}

namespace de::impls {
        rusty::fmt::Result BytesVisitor::expecting(rusty::fmt::Formatter& formatter) const {
            return formatter.write_str(std::string_view("a borrowed byte array"));
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<std::span<const uint8_t>, E> BytesVisitor::visit_borrowed_bytes(std::span<const uint8_t> v) {
            return rusty::Result<std::span<const uint8_t>, E>::Ok(v);
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<std::span<const uint8_t>, E> BytesVisitor::visit_borrowed_str(std::string_view v) {
            return rusty::Result<std::span<const uint8_t>, E>::Ok(rusty::as_bytes(v));
        }
}

namespace de::impls {
        rusty::fmt::Result CStringVisitor::expecting(rusty::fmt::Formatter& formatter) const {
            return formatter.write_str(std::string_view("byte array"));
        }
}

namespace de::impls {
        template<typename A>
        rusty::Result<rusty::ffi::CString, typename A::Error> CStringVisitor::visit_seq(A seq) {
            const auto capacity = size_hint::cautious<uint8_t>(seq.size_hint());
            auto values = std::conditional_t<true, rusty::Vec<uint8_t>, A>::with_capacity(std::move(capacity));
            while (true) {
                auto&& _whilelet = ({ auto&& _m = seq.next_element(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<rusty::ffi::CString, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                if (!(_whilelet.is_some())) { break; }
                auto value = _whilelet.unwrap();
                values.push(std::move(value));
            }
            return rusty::ffi::cstring_new(std::move(values)).map_err([&](auto&& _err) -> typename A::Error { return (A::Error::custom) (std::forward<decltype(_err)>(_err)); });
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<rusty::ffi::CString, E> CStringVisitor::visit_bytes(std::span<const uint8_t> v) {
            return rusty::ffi::cstring_new(v).map_err([&](auto&& _err) -> E { return (E::custom) (std::forward<decltype(_err)>(_err)); });
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<rusty::ffi::CString, E> CStringVisitor::visit_byte_buf(rusty::Vec<uint8_t> v) {
            return rusty::ffi::cstring_new(std::move(v)).map_err([&](auto&& _err) -> E { return (E::custom) (std::forward<decltype(_err)>(_err)); });
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<rusty::ffi::CString, E> CStringVisitor::visit_str(std::string_view v) {
            return rusty::ffi::cstring_new(v).map_err([&](auto&& _err) -> E { return (E::custom) (std::forward<decltype(_err)>(_err)); });
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<rusty::ffi::CString, E> CStringVisitor::visit_string(rusty::String v) {
            return rusty::ffi::cstring_new(std::move(v)).map_err([&](auto&& _err) -> E { return (E::custom) (std::forward<decltype(_err)>(_err)); });
        }
}

namespace de::impls {
        rusty::fmt::Result PathVisitor::expecting(rusty::fmt::Formatter& formatter) const {
            return formatter.write_str(std::string_view("a borrowed path"));
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<const rusty::path::Path&, E> PathVisitor::visit_borrowed_str(std::string_view v) {
            return rusty::Result<const rusty::path::Path&, E>::Ok(rusty::as_ref_into<const rusty::path::Path&>(v));
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<const rusty::path::Path&, E> PathVisitor::visit_borrowed_bytes(std::span<const uint8_t> v) {
            return rusty::str_runtime::from_utf8(v).map([&](auto&& _v) -> const rusty::path::Path& { return rusty::as_ref_into<const rusty::path::Path&>(std::forward<decltype(_v)>(_v)); }).map_err([&](auto&& _err) -> E { return ([&](auto _closure_wild0) { return E::invalid_value(de::Unexpected_Bytes{v}, (*this)); }) (std::forward<decltype(_err)>(_err)); });
        }
}

namespace de::impls {
        rusty::fmt::Result PathBufVisitor::expecting(rusty::fmt::Formatter& formatter) const {
            return formatter.write_str(std::string_view("path string"));
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<rusty::path::PathBuf, E> PathBufVisitor::visit_str(std::string_view v) {
            return rusty::Result<rusty::path::PathBuf, E>::Ok(rusty::path::PathBuf(v));
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<rusty::path::PathBuf, E> PathBufVisitor::visit_string(rusty::String v) {
            return rusty::Result<rusty::path::PathBuf, E>::Ok(rusty::path::PathBuf(std::move(v)));
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<rusty::path::PathBuf, E> PathBufVisitor::visit_bytes(std::span<const uint8_t> v) {
            return rusty::str_runtime::from_utf8(v).map([&](auto&& _v) -> rusty::path::PathBuf { return rusty::from_into<rusty::path::PathBuf>(std::forward<decltype(_v)>(_v)); }).map_err([&](auto&& _err) -> E { return ([&](auto _closure_wild0) { return E::invalid_value(de::Unexpected_Bytes{v}, (*this)); }) (std::forward<decltype(_err)>(_err)); });
        }
}

namespace de::impls {
        template<typename E>
        rusty::Result<rusty::path::PathBuf, E> PathBufVisitor::visit_byte_buf(rusty::Vec<uint8_t> v) {
            return std::conditional_t<true, rusty::String, E>::from_utf8(std::move(v)).map([&](auto&& _v) -> rusty::path::PathBuf { return rusty::from_into<rusty::path::PathBuf>(std::forward<decltype(_v)>(_v)); }).map_err([&](auto&& _err) -> E { return ([&](auto&& e) { return E::invalid_value(de::Unexpected_Bytes{e.into_bytes()}, (*this)); }) (std::forward<decltype(_err)>(_err)); });
        }
}

namespace de::impls {
        rusty::fmt::Result OsStringVisitor::expecting(rusty::fmt::Formatter& formatter) const {
            return formatter.write_str(std::string_view("os string"));
        }
}

namespace de::impls {
        template<typename A>
        rusty::Result<rusty::ffi::OsString, typename A::Error> OsStringVisitor::visit_enum(A data) {
            return [&]() { auto&& _m = ({ auto&& _m = data.variant(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<rusty::ffi::OsString, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); }); return std::visit(overloaded { [&](auto&&) -> rusty::Result<rusty::ffi::OsString, typename A::Error> { return [&]() -> rusty::Result<rusty::ffi::OsString, typename A::Error> { rusty::intrinsics::unreachable(); }(); }, [&](auto&&) -> rusty::Result<rusty::ffi::OsString, typename A::Error> { return [&]() -> rusty::Result<rusty::ffi::OsString, typename A::Error> { rusty::intrinsics::unreachable(); }(); } }, std::move(_m)); }();
        }
}


// ── from serde.cppm ──

namespace lib {
    namespace core {
    }
}
namespace integer128 {
}
namespace private_ {
    namespace de {
        template<typename T>
        struct Borrowed;
        template<typename E>
        struct StrDeserializer;
        template<typename E>
        struct BorrowedStrDeserializer;
        template<typename E>
        struct FlatMapDeserializer;
        template<typename E>
        struct FlatMapAccess;
        template<typename E>
        struct FlatStructAccess;
        template<typename F>
        struct AdjacentlyTaggedEnumVariantSeed;
        template<typename F>
        struct AdjacentlyTaggedEnumVariantVisitor;
        namespace content {
            enum class TagOrContentField;
            constexpr TagOrContentField TagOrContentField_Tag();
            constexpr TagOrContentField TagOrContentField_Content();
            enum class TagContentOtherField;
            constexpr TagContentOtherField TagContentOtherField_Tag();
            constexpr TagContentOtherField TagContentOtherField_Content();
            constexpr TagContentOtherField TagContentOtherField_Other();
            struct ContentVisitor;
            struct TagOrContent_Tag;
            struct TagOrContent_Content;
            using TagOrContent = std::variant<TagOrContent_Tag, TagOrContent_Content>;
            struct TagOrContentVisitor;
            template<typename T>
            struct TaggedContentVisitor;
            struct TagOrContentFieldVisitor;
            struct TagContentOtherFieldVisitor;
            template<typename E>
            struct ContentDeserializer;
            template<typename E>
            struct SeqDeserializer;
            struct ExpectedInSeq;
            template<typename E>
            struct MapDeserializer;
            template<typename E>
            struct PairDeserializer;
            template<typename E>
            struct PairVisitor;
            struct ExpectedInMap;
            template<typename E>
            struct EnumDeserializer;
            template<typename E>
            struct VariantDeserializer;
            template<typename E>
            struct ContentRefDeserializer;
            template<typename E>
            struct SeqRefDeserializer;
            template<typename E>
            struct MapRefDeserializer;
            template<typename E>
            struct PairRefDeserializer;
            template<typename E>
            struct PairRefVisitor;
            template<typename E>
            struct EnumRefDeserializer;
            template<typename E>
            struct VariantRefDeserializer;
            struct InternallyTaggedUnitVisitor;
            struct UntaggedUnitVisitor;
            rusty::Option<std::string_view> content_as_str(const ::__private228::Content& content);
            ::__private228::Content content_clone(const ::__private228::Content& content);
            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_seq(rusty::Vec<::__private228::Content> content, V visitor);
            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_map(rusty::Vec<std::tuple<::__private228::Content, ::__private228::Content>> content, V visitor);
            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_seq_ref(std::span<const ::__private228::Content> content, V visitor);
            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_map_ref(std::span<const std::tuple<::__private228::Content, ::__private228::Content>> content, V visitor);
        }
        template<typename V, typename E>
        rusty::Result<V, E> missing_field(std::string_view field);
        template<typename D, typename R>
        rusty::Result<R, typename D::Error> borrow_cow_str(D deserializer);
        template<typename D, typename R>
        rusty::Result<R, typename D::Error> borrow_cow_bytes(D deserializer);
        rusty::Option<std::tuple<::__private228::Content, ::__private228::Content>> flat_map_take_entry(rusty::Option<std::tuple<::__private228::Content, ::__private228::Content>>& entry, std::span<const std::string_view> recognized);
    }
    namespace ser {
        enum class Unsupported;
        constexpr Unsupported Unsupported_Boolean();
        constexpr Unsupported Unsupported_Integer();
        constexpr Unsupported Unsupported_Float();
        constexpr Unsupported Unsupported_Char();
        constexpr Unsupported Unsupported_String();
        constexpr Unsupported Unsupported_ByteArray();
        constexpr Unsupported Unsupported_Optional();
        constexpr Unsupported Unsupported_Sequence();
        constexpr Unsupported Unsupported_Tuple();
        constexpr Unsupported Unsupported_TupleStruct();
        template<typename S>
        struct TaggedSerializer;
        template<typename M>
        struct FlatMapSerializer;
        template<typename M>
        struct FlatMapSerializeMap;
        template<typename M>
        struct FlatMapSerializeStruct;
        template<typename M>
        struct FlatMapSerializeTupleVariantAsMapValue;
        template<typename M>
        struct FlatMapSerializeStructVariantAsMapValue;
        struct AdjacentlyTaggedEnumVariant;
        template<typename T>
        struct CannotSerializeVariant;
        namespace content {
            struct Content;
            template<typename M>
            struct SerializeTupleVariantAsMapValue;
            template<typename M>
            struct SerializeStructVariantAsMapValue;
            template<typename E>
            struct ContentSerializer;
            template<typename E>
            struct SerializeSeq;
            template<typename E>
            struct SerializeTuple;
            template<typename E>
            struct SerializeTupleStruct;
            template<typename E>
            struct SerializeTupleVariant;
            template<typename E>
            struct SerializeMap;
            template<typename E>
            struct SerializeStruct;
            template<typename E>
            struct SerializeStructVariant;
        }
        template<typename T>
        const T& constrain(const T& t);
        template<typename S, typename T>
        rusty::Result<typename S::Ok, typename S::Error> serialize_tagged_newtype(S serializer, std::string_view type_ident, std::string_view variant_ident, std::string_view tag, std::string_view variant_name, const T& value);
    }
}
namespace __private228 {
}

namespace private_ {
    namespace de {
        // Extension trait free-function forward declarations
        namespace rusty_ext {
        }
    }
}


namespace lib {
    namespace core {}

    namespace core {
    }

    namespace core {

        using namespace ::std;

    }


    namespace ptr = rusty::ptr;
    namespace str = rusty::str_runtime;





    namespace fmt = rusty::fmt;

    using ::rusty::PhantomData;

    namespace option = rusty;

    namespace result = rusty;

    using ::rusty::Cow;
    using ::rusty::Cow_Borrowed;
    using ::rusty::Cow_Owned;

    using ::rusty::String;

    using ::rusty::Vec;

    using ::rusty::Box;

}

namespace de = ::de;
// Rust-only namespace re-export: using ::forward_to_deserialize_any;
namespace ser = ::ser;
// Rust-only unresolved import: using ::Deserialize;
// Rust-only unresolved import: using ::Deserializer;
// Rust-only unresolved import: using ::Serialize;
// Rust-only unresolved import: using ::Serializer;

namespace private_ {
    namespace de {}
    namespace ser {}

    namespace de {
        template<typename T>
        struct Borrowed;
        template<typename E>
        struct StrDeserializer;
        template<typename E>
        struct BorrowedStrDeserializer;
        template<typename E>
        struct FlatMapDeserializer;
        template<typename E>
        struct FlatMapAccess;
        template<typename E>
        struct FlatStructAccess;
        template<typename F>
        struct AdjacentlyTaggedEnumVariantSeed;
        template<typename F>
        struct AdjacentlyTaggedEnumVariantVisitor;
        namespace content {
            enum class TagOrContentField;
            constexpr TagOrContentField TagOrContentField_Tag();
            constexpr TagOrContentField TagOrContentField_Content();
            enum class TagContentOtherField;
            constexpr TagContentOtherField TagContentOtherField_Tag();
            constexpr TagContentOtherField TagContentOtherField_Content();
            constexpr TagContentOtherField TagContentOtherField_Other();
            struct ContentVisitor;
            struct TagOrContent_Tag;
            struct TagOrContent_Content;
            using TagOrContent = std::variant<TagOrContent_Tag, TagOrContent_Content>;
            struct TagOrContentVisitor;
            template<typename T>
            struct TaggedContentVisitor;
            struct TagOrContentFieldVisitor;
            struct TagContentOtherFieldVisitor;
            template<typename E>
            struct ContentDeserializer;
            template<typename E>
            struct SeqDeserializer;
            struct ExpectedInSeq;
            template<typename E>
            struct MapDeserializer;
            template<typename E>
            struct PairDeserializer;
            template<typename E>
            struct PairVisitor;
            struct ExpectedInMap;
            template<typename E>
            struct EnumDeserializer;
            template<typename E>
            struct VariantDeserializer;
            template<typename E>
            struct ContentRefDeserializer;
            template<typename E>
            struct SeqRefDeserializer;
            template<typename E>
            struct MapRefDeserializer;
            template<typename E>
            struct PairRefDeserializer;
            template<typename E>
            struct PairRefVisitor;
            template<typename E>
            struct EnumRefDeserializer;
            template<typename E>
            struct VariantRefDeserializer;
            struct InternallyTaggedUnitVisitor;
            struct UntaggedUnitVisitor;
            rusty::Option<std::string_view> content_as_str(const ::__private228::Content& content);
            ::__private228::Content content_clone(const ::__private228::Content& content);
            ::de::Unexpected content_unexpected(const ::__private228::Content& content);
            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_seq(rusty::Vec<::__private228::Content> content, V visitor);
            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_map(rusty::Vec<std::tuple<::__private228::Content, ::__private228::Content>> content, V visitor);
            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_seq_ref(std::span<const ::__private228::Content> content, V visitor);
            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_map_ref(std::span<const std::tuple<::__private228::Content, ::__private228::Content>> content, V visitor);
        }
        template<typename V, typename E>
        rusty::Result<V, E> missing_field(std::string_view field);
        template<typename D, typename R>
        rusty::Result<R, typename D::Error> borrow_cow_str(D deserializer);
        template<typename D, typename R>
        rusty::Result<R, typename D::Error> borrow_cow_bytes(D deserializer);
        rusty::Option<std::tuple<::__private228::Content, ::__private228::Content>> flat_map_take_entry(rusty::Option<std::tuple<::__private228::Content, ::__private228::Content>>& entry, std::span<const std::string_view> recognized);
    }
    namespace ser {
        enum class Unsupported;
        constexpr Unsupported Unsupported_Boolean();
        constexpr Unsupported Unsupported_Integer();
        constexpr Unsupported Unsupported_Float();
        constexpr Unsupported Unsupported_Char();
        constexpr Unsupported Unsupported_String();
        constexpr Unsupported Unsupported_ByteArray();
        constexpr Unsupported Unsupported_Optional();
        constexpr Unsupported Unsupported_Sequence();
        constexpr Unsupported Unsupported_Tuple();
        constexpr Unsupported Unsupported_TupleStruct();
        template<typename S>
        struct TaggedSerializer;
        template<typename M>
        struct FlatMapSerializer;
        template<typename M>
        struct FlatMapSerializeMap;
        template<typename M>
        struct FlatMapSerializeStruct;
        template<typename M>
        struct FlatMapSerializeTupleVariantAsMapValue;
        template<typename M>
        struct FlatMapSerializeStructVariantAsMapValue;
        struct AdjacentlyTaggedEnumVariant;
        template<typename T>
        struct CannotSerializeVariant;
        namespace content {
            struct Content;
            template<typename M>
            struct SerializeTupleVariantAsMapValue;
            template<typename M>
            struct SerializeStructVariantAsMapValue;
            template<typename E>
            struct ContentSerializer;
            template<typename E>
            struct SerializeSeq;
            template<typename E>
            struct SerializeTuple;
            template<typename E>
            struct SerializeTupleStruct;
            template<typename E>
            struct SerializeTupleVariant;
            template<typename E>
            struct SerializeMap;
            template<typename E>
            struct SerializeStruct;
            template<typename E>
            struct SerializeStructVariant;
        }
        template<typename T>
        const T& constrain(const T& t);
        template<typename S, typename T>
        rusty::Result<typename S::Ok, typename S::Error> serialize_tagged_newtype(S serializer, std::string_view type_ident, std::string_view variant_ident, std::string_view tag, std::string_view variant_name, const T& value);
    }




    namespace fmt = rusty::fmt;
    using ::rusty::fmt::Formatter;

    using ::rusty::PhantomData;

    using ::rusty::Option;
    using ::rusty::None;
    using ::rusty::Some;

    namespace ptr = rusty::ptr;

    using ::rusty::Result;
    using ::rusty::Err;
    using ::rusty::Ok;

    using ::__private228::string::from_utf8_lossy;

    using ::rusty::Vec;

    namespace de {
        namespace content {}

        template<typename T>
        struct Borrowed;
        template<typename E>
        struct StrDeserializer;
        template<typename E>
        struct BorrowedStrDeserializer;
        template<typename E>
        struct FlatMapDeserializer;
        template<typename E>
        struct FlatMapAccess;
        template<typename E>
        struct FlatStructAccess;
        template<typename F>
        struct AdjacentlyTaggedEnumVariantSeed;
        template<typename F>
        struct AdjacentlyTaggedEnumVariantVisitor;
        namespace content {
            enum class TagOrContentField;
            constexpr TagOrContentField TagOrContentField_Tag();
            constexpr TagOrContentField TagOrContentField_Content();
            enum class TagContentOtherField;
            constexpr TagContentOtherField TagContentOtherField_Tag();
            constexpr TagContentOtherField TagContentOtherField_Content();
            constexpr TagContentOtherField TagContentOtherField_Other();
            struct ContentVisitor;
            struct TagOrContent_Tag;
            struct TagOrContent_Content;
            using TagOrContent = std::variant<TagOrContent_Tag, TagOrContent_Content>;
            struct TagOrContentVisitor;
            template<typename T>
            struct TaggedContentVisitor;
            struct TagOrContentFieldVisitor;
            struct TagContentOtherFieldVisitor;
            template<typename E>
            struct ContentDeserializer;
            template<typename E>
            struct SeqDeserializer;
            struct ExpectedInSeq;
            template<typename E>
            struct MapDeserializer;
            template<typename E>
            struct PairDeserializer;
            template<typename E>
            struct PairVisitor;
            struct ExpectedInMap;
            template<typename E>
            struct EnumDeserializer;
            template<typename E>
            struct VariantDeserializer;
            template<typename E>
            struct ContentRefDeserializer;
            template<typename E>
            struct SeqRefDeserializer;
            template<typename E>
            struct MapRefDeserializer;
            template<typename E>
            struct PairRefDeserializer;
            template<typename E>
            struct PairRefVisitor;
            template<typename E>
            struct EnumRefDeserializer;
            template<typename E>
            struct VariantRefDeserializer;
            struct InternallyTaggedUnitVisitor;
            struct UntaggedUnitVisitor;
            rusty::Option<std::string_view> content_as_str(const ::__private228::Content& content);
            ::__private228::Content content_clone(const ::__private228::Content& content);
            ::de::Unexpected content_unexpected(const ::__private228::Content& content);
            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_seq(rusty::Vec<::__private228::Content> content, V visitor);
            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_map(rusty::Vec<std::tuple<::__private228::Content, ::__private228::Content>> content, V visitor);
            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_seq_ref(std::span<const ::__private228::Content> content, V visitor);
            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_map_ref(std::span<const std::tuple<::__private228::Content, ::__private228::Content>> content, V visitor);
        }
        template<typename V, typename E>
        rusty::Result<V, E> missing_field(std::string_view field);
        template<typename D, typename R>
        rusty::Result<R, typename D::Error> borrow_cow_str(D deserializer);
        template<typename D, typename R>
        rusty::Result<R, typename D::Error> borrow_cow_bytes(D deserializer);
        rusty::Option<std::tuple<::__private228::Content, ::__private228::Content>> flat_map_take_entry(rusty::Option<std::tuple<::__private228::Content, ::__private228::Content>>& entry, std::span<const std::string_view> recognized);

        using namespace lib;

        using ::de::value::BorrowedBytesDeserializer;
        using ::de::value::BytesDeserializer;


        using ::de::Unexpected;

        using ::__private228::InPlaceSeed;

        using ::private_::de::content::content_as_str;
        namespace serde_core_private = ::__private228;
        using serde_core_private::Content;
        using ::private_::de::content::ContentDeserializer;
        using ::private_::de::content::ContentRefDeserializer;
        using ::private_::de::content::ContentVisitor;
        using ::private_::de::content::EnumDeserializer;
        using ::private_::de::content::InternallyTaggedUnitVisitor;
        using ::private_::de::content::TagContentOtherField;
        using ::private_::de::content::TagContentOtherFieldVisitor;
        using ::private_::de::content::TagOrContentField;
        using ::private_::de::content::TagOrContentFieldVisitor;
        using ::private_::de::content::TaggedContentVisitor;
        using ::private_::de::content::UntaggedUnitVisitor;

        namespace content {

            enum class TagOrContentField;
            constexpr TagOrContentField TagOrContentField_Tag();
            constexpr TagOrContentField TagOrContentField_Content();
            enum class TagContentOtherField;
            constexpr TagContentOtherField TagContentOtherField_Tag();
            constexpr TagContentOtherField TagContentOtherField_Content();
            constexpr TagContentOtherField TagContentOtherField_Other();
            struct ContentVisitor;
            struct TagOrContent_Tag;
            struct TagOrContent_Content;
            using TagOrContent = std::variant<TagOrContent_Tag, TagOrContent_Content>;
            struct TagOrContentVisitor;
            template<typename T>
            struct TaggedContentVisitor;
            struct TagOrContentFieldVisitor;
            struct TagContentOtherFieldVisitor;
            template<typename E>
            struct ContentDeserializer;
            template<typename E>
            struct SeqDeserializer;
            struct ExpectedInSeq;
            template<typename E>
            struct MapDeserializer;
            template<typename E>
            struct PairDeserializer;
            template<typename E>
            struct PairVisitor;
            struct ExpectedInMap;
            template<typename E>
            struct EnumDeserializer;
            template<typename E>
            struct VariantDeserializer;
            template<typename E>
            struct ContentRefDeserializer;
            template<typename E>
            struct SeqRefDeserializer;
            template<typename E>
            struct MapRefDeserializer;
            template<typename E>
            struct PairRefDeserializer;
            template<typename E>
            struct PairRefVisitor;
            template<typename E>
            struct EnumRefDeserializer;
            template<typename E>
            struct VariantRefDeserializer;
            struct InternallyTaggedUnitVisitor;
            struct UntaggedUnitVisitor;
            rusty::Option<std::string_view> content_as_str(const ::__private228::Content& content);
            ::__private228::Content content_clone(const ::__private228::Content& content);
            ::de::Unexpected content_unexpected(const ::__private228::Content& content);
            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_seq(rusty::Vec<::__private228::Content> content, V visitor);
            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_map(rusty::Vec<std::tuple<::__private228::Content, ::__private228::Content>> content, V visitor);
            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_seq_ref(std::span<const ::__private228::Content> content, V visitor);
            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_map_ref(std::span<const std::tuple<::__private228::Content, ::__private228::Content>> content, V visitor);

            enum class TagOrContentField {
                Tag,
    Content
            };
            inline constexpr TagOrContentField TagOrContentField_Tag() { return TagOrContentField::Tag; }
            inline constexpr TagOrContentField TagOrContentField_Content() { return TagOrContentField::Content; }

            enum class TagContentOtherField {
                Tag,
    Content,
    Other
            };
            inline constexpr TagContentOtherField TagContentOtherField_Tag() { return TagContentOtherField::Tag; }
            inline constexpr TagContentOtherField TagContentOtherField_Content() { return TagContentOtherField::Content; }
            inline constexpr TagContentOtherField TagContentOtherField_Other() { return TagContentOtherField::Other; }

            using namespace lib;

            // Rust-only unresolved import: using de;
            using ::de::IgnoredAny;
            using ::de::Unexpected;

            namespace size_hint = ::__private228::size_hint;

            using ::__private228::Content;

            struct ContentVisitor {
                using Value = serde_core_private::Content;
                rusty::PhantomData<serde_core_private::Content> value;

                static ContentVisitor new_();
                template<typename D>
                rusty::Result<serde_core_private::Content, typename D::Error> deserialize(D deserializer);
                rusty::fmt::Result expecting(rusty::fmt::Formatter& fmt) const;
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_bool(bool value);
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_i8(int8_t value);
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_i16(int16_t value);
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_i32(int32_t value);
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_i64(int64_t value);
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_u8(uint8_t value);
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_u16(uint16_t value);
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_u32(uint32_t value);
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_u64(uint64_t value);
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_f32(float value);
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_f64(double value);
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_char(char32_t value);
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_str(std::string_view value);
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_borrowed_str(std::string_view value);
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_string(rusty::String value);
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_bytes(std::span<const uint8_t> value);
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_borrowed_bytes(std::span<const uint8_t> value);
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_byte_buf(rusty::Vec<uint8_t> value);
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_unit();
                template<typename F>
                rusty::Result<serde_core_private::Content, F> visit_none();
                template<typename D>
                rusty::Result<serde_core_private::Content, typename D::Error> visit_some(D deserializer);
                template<typename D>
                rusty::Result<serde_core_private::Content, typename D::Error> visit_newtype_struct(D deserializer);
                template<typename V>
                rusty::Result<serde_core_private::Content, typename V::Error> visit_seq(V visitor);
                template<typename V>
                rusty::Result<serde_core_private::Content, typename V::Error> visit_map(V visitor);
                template<typename V>
                rusty::Result<serde_core_private::Content, typename V::Error> visit_enum(V _visitor);
            };

            // Algebraic data type
            struct TagOrContent_Tag {};
            struct TagOrContent_Content {
                serde_core_private::Content _0;
            };
            TagOrContent_Tag Tag();
            using TagOrContent = std::variant<TagOrContent_Tag, TagOrContent_Content>;
            TagOrContent_Tag Tag() { return TagOrContent_Tag{};  }

            /// Serves as a seed for deserializing a key of internally tagged enum.
            /// Cannot capture externally tagged enums, `i128` and `u128`.
            struct TagOrContentVisitor {
                using Value = TagOrContent;
                std::string_view name;
                rusty::PhantomData<TagOrContent> value;

                static TagOrContentVisitor new_(std::string_view name);
                template<typename D>
                rusty::Result<TagOrContent, typename D::Error> deserialize(D deserializer);
                rusty::fmt::Result expecting(rusty::fmt::Formatter& fmt) const;
                template<typename F>
                rusty::Result<TagOrContent, F> visit_bool(bool value);
                template<typename F>
                rusty::Result<TagOrContent, F> visit_i8(int8_t value);
                template<typename F>
                rusty::Result<TagOrContent, F> visit_i16(int16_t value);
                template<typename F>
                rusty::Result<TagOrContent, F> visit_i32(int32_t value);
                template<typename F>
                rusty::Result<TagOrContent, F> visit_i64(int64_t value);
                template<typename F>
                rusty::Result<TagOrContent, F> visit_u8(uint8_t value);
                template<typename F>
                rusty::Result<TagOrContent, F> visit_u16(uint16_t value);
                template<typename F>
                rusty::Result<TagOrContent, F> visit_u32(uint32_t value);
                template<typename F>
                rusty::Result<TagOrContent, F> visit_u64(uint64_t value);
                template<typename F>
                rusty::Result<TagOrContent, F> visit_f32(float value);
                template<typename F>
                rusty::Result<TagOrContent, F> visit_f64(double value);
                template<typename F>
                rusty::Result<TagOrContent, F> visit_char(char32_t value);
                template<typename F>
                rusty::Result<TagOrContent, F> visit_str(std::string_view value);
                template<typename F>
                rusty::Result<TagOrContent, F> visit_borrowed_str(std::string_view value);
                template<typename F>
                rusty::Result<TagOrContent, F> visit_string(rusty::String value);
                template<typename F>
                rusty::Result<TagOrContent, F> visit_bytes(std::span<const uint8_t> value);
                template<typename F>
                rusty::Result<TagOrContent, F> visit_borrowed_bytes(std::span<const uint8_t> value);
                template<typename F>
                rusty::Result<TagOrContent, F> visit_byte_buf(rusty::Vec<uint8_t> value);
                template<typename F>
                rusty::Result<TagOrContent, F> visit_unit();
                template<typename F>
                rusty::Result<TagOrContent, F> visit_none();
                template<typename D>
                rusty::Result<TagOrContent, typename D::Error> visit_some(D deserializer);
                template<typename D>
                rusty::Result<TagOrContent, typename D::Error> visit_newtype_struct(D deserializer);
                template<typename V>
                rusty::Result<TagOrContent, typename V::Error> visit_seq(V visitor);
                template<typename V>
                rusty::Result<TagOrContent, typename V::Error> visit_map(V visitor);
                template<typename V>
                rusty::Result<TagOrContent, typename V::Error> visit_enum(V visitor);
            };

            /// Used by generated code to deserialize an internally tagged enum.
            ///
            /// Captures map or sequence from the original deserializer and searches
            /// a tag in it (in case of sequence, tag is the first element of sequence).
            ///
            /// Not public API.
            template<typename T>
            struct TaggedContentVisitor {
                using Value = std::tuple<T, serde_core_private::Content>;
                std::string_view tag_name;
                std::string_view expecting_field;
                rusty::PhantomData<T> value;

                static TaggedContentVisitor<T> new_(std::string_view name, std::string_view expecting) {
                    return TaggedContentVisitor<T>{.tag_name = std::string_view(name), .expecting_field = std::string_view(expecting), .value = rusty::PhantomData<T>{}};
                }
                rusty::fmt::Result expecting(rusty::fmt::Formatter& fmt) const {
                    return fmt.write_str(this->expecting_field);
                }
                template<typename S>
                rusty::Result<Value, typename S::Error> visit_seq(S seq) {
                    auto tag = ({ auto&& _m = ({ auto&& _m = seq.next_element(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Value, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); }); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& tag = _mv;
_match_value.emplace(std::move(tag)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::Result<Value, typename S::Error>::Err(S::Error::missing_field(std::move(std::string_view(this->tag_name)))); } std::move(_match_value).value(); });
                    auto rest = ::de::value::SeqAccessDeserializer<S>::new_(std::move(seq));
                    return rusty::Result<Value, typename S::Error>::Ok(std::make_tuple(std::move(tag), ({ auto&& _m = ::de::rusty_ext::deserialize(std::conditional_t<true, ContentVisitor, S>::new_(), std::move(rest)); std::optional<Value> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Value, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); })));
                }
                template<typename M>
                rusty::Result<Value, typename M::Error> visit_map(M map) {
                    rusty::Option<T> tag = rusty::Option<T>(rusty::None);
                    auto vec = std::conditional_t<true, rusty::Vec<std::tuple<serde_core_private::Content, serde_core_private::Content>>, M>::with_capacity(size_hint::cautious<std::tuple<serde_core_private::Content, serde_core_private::Content>>(map.size_hint()));
                    while (true) {
                        auto&& _whilelet = ({ auto&& _m = map.next_key_seed(std::conditional_t<true, TagOrContentVisitor, M>::new_(std::move(std::string_view(this->tag_name)))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Value, typename M::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                        if (!(_whilelet.is_some())) { break; }
                        auto k = _whilelet.unwrap();
                        {
                            auto&& _m = k;
                            std::visit(overloaded {
                                [&](const TagOrContent_Tag&) {
                                    if (tag.is_some()) {
                                        return rusty::Result<Value, typename M::Error>::Err(M::Error::duplicate_field(std::move(this->tag_name)));
                                    }
                                    tag = rusty::Option<T>(({ auto&& _m = map.next_value(); std::optional<T> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Value, typename M::Error>::Err(std::move(err)); } std::move(_match_value).value(); }));
                                },
                                [&](const TagOrContent_Content& _v) {
                                    auto&& k = _v._0;
                                    auto v = ({ auto&& _m = map.next_value_seed(std::conditional_t<true, ContentVisitor, M>::new_()); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Value, typename M::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                                    vec.push(std::make_tuple(std::move(k), std::move(v)));
                                },
                            }, _m);
                        }
                    }
                    return [&]() -> rusty::Result<Value, typename M::Error> { auto&& _m = tag; if (_m.is_none()) { return rusty::Result<Value, typename M::Error>::Err(M::Error::missing_field(std::move(std::string_view(this->tag_name)))); } if (_m.is_some()) { auto&& _mv1 = _m.unwrap(); auto&& tag = _mv1; return rusty::Result<Value, typename M::Error>::Ok(std::make_tuple(std::move(tag), std::conditional_t<true, Content, M>::Map(std::move(vec)))); } return [&]() -> rusty::Result<Value, typename M::Error> { rusty::intrinsics::unreachable(); }(); }();
                }
            };

            /// Not public API.
            struct TagOrContentFieldVisitor {
                using Value = TagOrContentField;
                /// Name of the tag field of the adjacently tagged enum
                std::string_view tag;
                /// Name of the content field of the adjacently tagged enum
                std::string_view content;

                template<typename D>
                rusty::Result<TagOrContentField, typename D::Error> deserialize(D deserializer);
                rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const;
                template<typename E>
                rusty::Result<TagOrContentField, E> visit_u64(uint64_t field_index);
                template<typename E>
                rusty::Result<TagOrContentField, E> visit_str(std::string_view field);
                template<typename E>
                rusty::Result<TagOrContentField, E> visit_bytes(std::span<const uint8_t> field);
            };

            /// Not public API.
            struct TagContentOtherFieldVisitor {
                using Value = TagContentOtherField;
                /// Name of the tag field of the adjacently tagged enum
                std::string_view tag;
                /// Name of the content field of the adjacently tagged enum
                std::string_view content;

                template<typename D>
                rusty::Result<TagContentOtherField, typename D::Error> deserialize(D deserializer);
                rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const;
                template<typename E>
                rusty::Result<TagContentOtherField, E> visit_u64(uint64_t field_index);
                template<typename E>
                rusty::Result<TagContentOtherField, E> visit_str(std::string_view field);
                template<typename E>
                rusty::Result<TagContentOtherField, E> visit_bytes(std::span<const uint8_t> field);
            };

            /// Not public API
            template<typename E>
            struct ContentDeserializer {
                using Error = E;
                using Deserializer = ContentDeserializer<E>;
                serde_core_private::Content content;
                rusty::PhantomData<E> err;

                E invalid_type(const auto& exp) {
                    return E::invalid_type(content_unexpected(rusty::detail::deref_if_pointer_like(this->content)), exp);
                }
                template<typename V>
                auto deserialize_integer(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](serde_core_private::Content_U8&& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u8(std::move(v)); }, [&](serde_core_private::Content_U16&& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u16(std::move(v)); }, [&](serde_core_private::Content_U32&& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u32(std::move(v)); }, [&](serde_core_private::Content_U64&& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u64(std::move(v)); }, [&](serde_core_private::Content_I8&& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i8(std::move(v)); }, [&](serde_core_private::Content_I16&& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i16(std::move(v)); }, [&](serde_core_private::Content_I32&& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i32(std::move(v)); }, [&](serde_core_private::Content_I64&& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i64(std::move(v)); }, [&](auto&&) -> rusty::Result<typename V::Value, E> { return rusty::Result<typename V::Value, E>::Err(this->invalid_type(visitor)); } }, std::move(_m)); }();
                }
                template<typename V>
                auto deserialize_float(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](serde_core_private::Content_F32&& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_f32(std::move(v)); }, [&](serde_core_private::Content_F64&& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_f64(std::move(v)); }, [&](serde_core_private::Content_U8&& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u8(std::move(v)); }, [&](serde_core_private::Content_U16&& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u16(std::move(v)); }, [&](serde_core_private::Content_U32&& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u32(std::move(v)); }, [&](serde_core_private::Content_U64&& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u64(std::move(v)); }, [&](serde_core_private::Content_I8&& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i8(std::move(v)); }, [&](serde_core_private::Content_I16&& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i16(std::move(v)); }, [&](serde_core_private::Content_I32&& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i32(std::move(v)); }, [&](serde_core_private::Content_I64&& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i64(std::move(v)); }, [&](auto&&) -> rusty::Result<typename V::Value, E> { return rusty::Result<typename V::Value, E>::Err(this->invalid_type(visitor)); } }, std::move(_m)); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](serde_core_private::Content_Bool&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_bool(std::move(v)); }, [&](serde_core_private::Content_U8&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_u8(std::move(v)); }, [&](serde_core_private::Content_U16&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_u16(std::move(v)); }, [&](serde_core_private::Content_U32&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_u32(std::move(v)); }, [&](serde_core_private::Content_U64&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_u64(std::move(v)); }, [&](serde_core_private::Content_I8&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_i8(std::move(v)); }, [&](serde_core_private::Content_I16&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_i16(std::move(v)); }, [&](serde_core_private::Content_I32&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_i32(std::move(v)); }, [&](serde_core_private::Content_I64&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_i64(std::move(v)); }, [&](serde_core_private::Content_F32&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_f32(std::move(v)); }, [&](serde_core_private::Content_F64&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_f64(std::move(v)); }, [&](serde_core_private::Content_Char&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_char(std::move(v)); }, [&](serde_core_private::Content_String&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_string(std::move(v)); }, [&](serde_core_private::Content_Str&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_borrowed_str(std::move(rusty::to_string_view(v))); }, [&](serde_core_private::Content_ByteBuf&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_byte_buf(std::move(v)); }, [&](serde_core_private::Content_Bytes&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_borrowed_bytes(v); }, [&](serde_core_private::Content_Unit&&) -> rusty::Result<typename V::Value, Error> { return visitor.visit_unit(); }, [&](serde_core_private::Content_None&&) -> rusty::Result<typename V::Value, Error> { return visitor.visit_none(); }, [&](serde_core_private::Content_Some&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_some(ContentDeserializer<E>::new_(std::move(rusty::detail::deref_if_pointer_like(v)))); }, [&](serde_core_private::Content_Newtype&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_newtype_struct(ContentDeserializer<E>::new_(std::move(rusty::detail::deref_if_pointer_like(v)))); }, [&](serde_core_private::Content_Seq&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visit_content_seq(std::move(v), std::move(visitor)); }, [&](serde_core_private::Content_Map&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visit_content_map(std::move(v), std::move(visitor)); } }, std::move(_m)); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_bool(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](serde_core_private::Content_Bool&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_bool(std::move(v)); }, [&](auto&&) -> rusty::Result<typename V::Value, Error> { return rusty::Result<typename V::Value, Error>::Err(this->invalid_type(visitor)); } }, std::move(_m)); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_i8(V visitor) {
                    return this->deserialize_integer(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_i16(V visitor) {
                    return this->deserialize_integer(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_i32(V visitor) {
                    return this->deserialize_integer(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_i64(V visitor) {
                    return this->deserialize_integer(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_u8(V visitor) {
                    return this->deserialize_integer(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_u16(V visitor) {
                    return this->deserialize_integer(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_u32(V visitor) {
                    return this->deserialize_integer(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_u64(V visitor) {
                    return this->deserialize_integer(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_f32(V visitor) {
                    return this->deserialize_float(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_f64(V visitor) {
                    return this->deserialize_float(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_char(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](serde_core_private::Content_Char&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_char(std::move(v)); }, [&](serde_core_private::Content_String&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_string(std::move(v)); }, [&](serde_core_private::Content_Str&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_borrowed_str(std::move(rusty::to_string_view(v))); }, [&](auto&&) -> rusty::Result<typename V::Value, Error> { return rusty::Result<typename V::Value, Error>::Err(this->invalid_type(visitor)); } }, std::move(_m)); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_str(V visitor) {
                    return this->deserialize_string(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_string(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](serde_core_private::Content_String&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_string(std::move(v)); }, [&](serde_core_private::Content_Str&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_borrowed_str(std::move(rusty::to_string_view(v))); }, [&](serde_core_private::Content_ByteBuf&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_byte_buf(std::move(v)); }, [&](serde_core_private::Content_Bytes&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_borrowed_bytes(v); }, [&](auto&&) -> rusty::Result<typename V::Value, Error> { return rusty::Result<typename V::Value, Error>::Err(this->invalid_type(visitor)); } }, std::move(_m)); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_bytes(V visitor) {
                    return this->deserialize_byte_buf(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_byte_buf(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](serde_core_private::Content_String&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_string(std::move(v)); }, [&](serde_core_private::Content_Str&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_borrowed_str(std::move(rusty::to_string_view(v))); }, [&](serde_core_private::Content_ByteBuf&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_byte_buf(std::move(v)); }, [&](serde_core_private::Content_Bytes&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_borrowed_bytes(v); }, [&](serde_core_private::Content_Seq&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visit_content_seq(std::move(v), std::move(visitor)); }, [&](auto&&) -> rusty::Result<typename V::Value, Error> { return rusty::Result<typename V::Value, Error>::Err(this->invalid_type(visitor)); } }, std::move(_m)); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_option(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](serde_core_private::Content_None&&) -> rusty::Result<typename V::Value, Error> { return visitor.visit_none(); }, [&](serde_core_private::Content_Some&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_some(ContentDeserializer<E>::new_(std::move(rusty::detail::deref_if_pointer_like(v)))); }, [&](serde_core_private::Content_Unit&&) -> rusty::Result<typename V::Value, Error> { return visitor.visit_unit(); }, [&](auto&&) -> rusty::Result<typename V::Value, Error> { return visitor.visit_some(std::move((*this))); } }, std::move(_m)); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_unit(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](serde_core_private::Content_Unit&&) -> rusty::Result<typename V::Value, Error> { return visitor.visit_unit(); }, [&](serde_core_private::Content_Map&& _v) -> rusty::Result<typename V::Value, Error> { const auto& v = _v._0; if (rusty::is_empty(v)) return visitor.visit_unit(); return [&]() -> rusty::Result<typename V::Value, Error> { rusty::intrinsics::unreachable(); }(); }, [&](auto&&) -> rusty::Result<typename V::Value, Error> { return rusty::Result<typename V::Value, Error>::Err(this->invalid_type(visitor)); } }, std::move(_m)); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_unit_struct(std::string_view _name, V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](serde_core_private::Content_Map&& _v) -> rusty::Result<typename V::Value, Error> { const auto& v = _v._0; if (rusty::is_empty(v)) return visitor.visit_unit(); return [&]() -> rusty::Result<typename V::Value, Error> { rusty::intrinsics::unreachable(); }(); }, [&](serde_core_private::Content_Seq&& _v) -> rusty::Result<typename V::Value, Error> { const auto& v = _v._0; if (rusty::is_empty(v)) return visitor.visit_unit(); return [&]() -> rusty::Result<typename V::Value, Error> { rusty::intrinsics::unreachable(); }(); }, [&](auto&&) -> rusty::Result<typename V::Value, Error> { return this->deserialize_any(std::move(visitor)); } }, std::move(_m)); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_newtype_struct(std::string_view _name, V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](serde_core_private::Content_Newtype&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_newtype_struct(ContentDeserializer<E>::new_(std::move(rusty::detail::deref_if_pointer_like(v)))); }, [&](auto&&) -> rusty::Result<typename V::Value, Error> { return visitor.visit_newtype_struct(std::move((*this))); } }, std::move(_m)); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_seq(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](serde_core_private::Content_Seq&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visit_content_seq(std::move(v), std::move(visitor)); }, [&](auto&&) -> rusty::Result<typename V::Value, Error> { return rusty::Result<typename V::Value, Error>::Err(this->invalid_type(visitor)); } }, std::move(_m)); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_tuple(size_t _len, V visitor) {
                    return this->deserialize_seq(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_tuple_struct(std::string_view _name, size_t _len, V visitor) {
                    return this->deserialize_seq(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_map(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](serde_core_private::Content_Map&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visit_content_map(std::move(v), std::move(visitor)); }, [&](auto&&) -> rusty::Result<typename V::Value, Error> { return rusty::Result<typename V::Value, Error>::Err(this->invalid_type(visitor)); } }, std::move(_m)); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_struct(std::string_view _name, std::span<const std::string_view> _fields, V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](serde_core_private::Content_Seq&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visit_content_seq(std::move(v), std::move(visitor)); }, [&](serde_core_private::Content_Map&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visit_content_map(std::move(v), std::move(visitor)); }, [&](auto&&) -> rusty::Result<typename V::Value, Error> { return rusty::Result<typename V::Value, Error>::Err(this->invalid_type(visitor)); } }, std::move(_m)); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_enum(std::string_view _name, std::span<const std::string_view> _variants, V visitor) {
                    auto [variant, value] = rusty::detail::deref_if_pointer_like([&]() { auto&& _m = this->content; return std::visit(overloaded { [&](serde_core_private::Content_Map&& _v) { auto&& value = _v._0; return [&]() { auto iter = rusty::iter(std::move(value));
auto [variant, value] = rusty::detail::deref_if_pointer_like(({ auto&& _m = iter.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& v = _mv;
_match_value.emplace(std::move(v)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_value(::de::Unexpected::Map, rusty::addr_of_temp("map with a single key"))); } std::move(_match_value).value(); }));
if (iter.next().is_some()) {
    return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_value(::de::Unexpected::Map, rusty::addr_of_temp("map with a single key")));
}
return std::make_tuple(std::move(variant), rusty::Some(value)); }(); }, [&](auto&&) { return rusty::intrinsics::unreachable(); }, [&](auto&& other) { return (static_cast<void>([&]() { return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_type(content_unexpected(rusty::detail::deref_if_pointer_like(other)), rusty::addr_of_temp("string or map"))); }()), rusty::intrinsics::unreachable()); } }, std::move(_m)); }());
                    return visitor.visit_enum(EnumDeserializer<E>::new_(std::move(variant), std::move(value)));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_identifier(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](serde_core_private::Content_String&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_string(std::move(v)); }, [&](serde_core_private::Content_Str&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_borrowed_str(std::move(rusty::to_string_view(v))); }, [&](serde_core_private::Content_ByteBuf&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_byte_buf(std::move(v)); }, [&](serde_core_private::Content_Bytes&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_borrowed_bytes(v); }, [&](serde_core_private::Content_U8&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_u8(std::move(v)); }, [&](serde_core_private::Content_U64&& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_u64(std::move(v)); }, [&](auto&&) -> rusty::Result<typename V::Value, Error> { return rusty::Result<typename V::Value, Error>::Err(this->invalid_type(visitor)); } }, std::move(_m)); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_ignored_any(V visitor) {
                    rusty::mem::drop(std::move((*this)));
                    return visitor.visit_unit();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> __deserialize_content_v1(V visitor) {
                    static_cast<void>(std::move(visitor));
                    return rusty::Result<typename V::Value, Error>::Ok(std::move(this->content));
                }
                static ContentDeserializer<E> new_(serde_core_private::Content content) {
                    return ContentDeserializer<E>{.content = std::move(content), .err = rusty::PhantomData<E>{}};
                }
                ContentDeserializer<E> into_deserializer() {
                    return std::move((*this));
                }
            };

            template<typename E>
            struct SeqDeserializer {
                using Error = E;
                decltype(rusty::iter(std::declval<rusty::Vec<serde_core_private::Content>>())) iter;
                size_t count;
                rusty::PhantomData<E> marker;

                static SeqDeserializer<E> new_(rusty::Vec<serde_core_private::Content> content) {
                    return SeqDeserializer<E>{.iter = rusty::iter(std::move(content)), .count = static_cast<size_t>(0), .marker = rusty::PhantomData<E>{}};
                }
                rusty::Result<std::tuple<>, E> end() {
                    const auto remaining = rusty::count(this->iter);
                    if (remaining == 0) {
                        return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
                    } else {
                        return rusty::Result<std::tuple<>, E>::Err(E::invalid_length(this->count + remaining, rusty::addr_of_temp(ExpectedInSeq(std::move(this->count)))));
                    }
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                    auto v = ({ auto&& _m = visitor.visit_seq((*this)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename V::Value, Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                    {
                        auto&& _m = this->end();
                        bool _m_matched = false;
                        if (!_m_matched) {
                            if (_m.is_ok()) {
                                auto&& _mv0 = _m.unwrap();
                                auto&& val = _mv0;
                                val;
                                _m_matched = true;
                            }
                        }
                        if (!_m_matched) {
                            if (_m.is_err()) {
                                auto&& _mv1 = _m.unwrap_err();
                                auto&& err = _mv1;
                                return rusty::Result<typename V::Value, Error>::Err(std::move(err));
                                _m_matched = true;
                            }
                        }
                    }
                    return rusty::Result<typename V::Value, Error>::Ok(std::move(v));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_bool(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_i8(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_i16(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_i32(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_i64(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_i128(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_u8(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_u16(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_u32(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_u64(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_u128(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_f32(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_f64(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_char(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_str(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_string(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_bytes(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_option(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_unit(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                    static_cast<void>(name);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                    static_cast<void>(name);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_seq(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                    static_cast<void>(std::move(len));
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                    static_cast<void>(name);
                    static_cast<void>(std::move(len));
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_map(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                    static_cast<void>(name);
                    static_cast<void>(fields);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                    static_cast<void>(name);
                    static_cast<void>(variants);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_identifier(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<rusty::Option<typename V::Value>, Error> next_element_seed(V seed) {
                    return [&]() -> rusty::Result<rusty::Option<typename V::Value>, Error> { auto&& _m = this->iter.next(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& value = _mv0; return [&]() -> rusty::Result<rusty::Option<typename V::Value>, Error> { [&]() { static_cast<void>(this->count += 1); return std::make_tuple(); }();
return ::de::rusty_ext::deserialize(seed, ContentDeserializer<E>::new_(std::move(value))).map(rusty::Some); }(); } if (_m.is_none()) { return rusty::Result<rusty::Option<typename V::Value>, Error>::Ok(rusty::Option<typename V::Value>(rusty::None)); } return [&]() -> rusty::Result<rusty::Option<typename V::Value>, Error> { rusty::intrinsics::unreachable(); }(); }();
                }
                rusty::Option<size_t> size_hint() const {
                    return size_hint::from_bounds(&this->iter);
                }
            };

            struct ExpectedInSeq {
                size_t _0;

                rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const;
            };

            template<typename E>
            struct MapDeserializer {
                using Error = E;
                decltype(rusty::iter(std::declval<rusty::Vec<std::tuple<serde_core_private::Content, serde_core_private::Content>>>())) iter;
                rusty::Option<serde_core_private::Content> value;
                size_t count;
                rusty::PhantomData<E> error;

                static MapDeserializer<E> new_(rusty::Vec<std::tuple<serde_core_private::Content, serde_core_private::Content>> content) {
                    return MapDeserializer<E>{.iter = rusty::iter(std::move(content)), .value = rusty::Option<serde_core_private::Content>(rusty::None), .count = static_cast<size_t>(0), .error = rusty::PhantomData<E>{}};
                }
                rusty::Result<std::tuple<>, E> end() {
                    const auto remaining = rusty::count(this->iter);
                    if (remaining == 0) {
                        return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
                    } else {
                        return rusty::Result<std::tuple<>, E>::Err(E::invalid_length(this->count + remaining, rusty::addr_of_temp(ExpectedInMap(std::move(this->count)))));
                    }
                }
                rusty::Option<std::tuple<serde_core_private::Content, serde_core_private::Content>> next_pair() {
                    return [&]() -> rusty::Option<std::tuple<serde_core_private::Content, serde_core_private::Content>> { auto&& _m = this->iter.next(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& k = std::get<0>(rusty::detail::deref_if_pointer(_mv0)); auto&& v = std::get<1>(rusty::detail::deref_if_pointer(_mv0)); return [&]() -> rusty::Option<std::tuple<serde_core_private::Content, serde_core_private::Content>> { [&]() { static_cast<void>(this->count += 1); return std::make_tuple(); }();
return rusty::Option<std::tuple<serde_core_private::Content, serde_core_private::Content>>(std::make_tuple(std::move(k), std::move(v))); }(); } if (_m.is_none()) { return rusty::Option<std::tuple<serde_core_private::Content, serde_core_private::Content>>(rusty::None); } return [&]() -> rusty::Option<std::tuple<serde_core_private::Content, serde_core_private::Content>> { rusty::intrinsics::unreachable(); }(); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                    auto value = ({ auto&& _m = visitor.visit_map((*this)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename V::Value, Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                    {
                        auto&& _m = this->end();
                        bool _m_matched = false;
                        if (!_m_matched) {
                            if (_m.is_ok()) {
                                auto&& _mv0 = _m.unwrap();
                                auto&& val = _mv0;
                                val;
                                _m_matched = true;
                            }
                        }
                        if (!_m_matched) {
                            if (_m.is_err()) {
                                auto&& _mv1 = _m.unwrap_err();
                                auto&& err = _mv1;
                                return rusty::Result<typename V::Value, Error>::Err(std::move(err));
                                _m_matched = true;
                            }
                        }
                    }
                    return rusty::Result<typename V::Value, Error>::Ok(std::move(value));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_seq(V visitor) {
                    auto value = ({ auto&& _m = visitor.visit_seq((*this)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename V::Value, Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                    {
                        auto&& _m = this->end();
                        bool _m_matched = false;
                        if (!_m_matched) {
                            if (_m.is_ok()) {
                                auto&& _mv0 = _m.unwrap();
                                auto&& val = _mv0;
                                val;
                                _m_matched = true;
                            }
                        }
                        if (!_m_matched) {
                            if (_m.is_err()) {
                                auto&& _mv1 = _m.unwrap_err();
                                auto&& err = _mv1;
                                return rusty::Result<typename V::Value, Error>::Err(std::move(err));
                                _m_matched = true;
                            }
                        }
                    }
                    return rusty::Result<typename V::Value, Error>::Ok(std::move(value));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_tuple(size_t len, V visitor) {
                    static_cast<void>(std::move(len));
                    return this->deserialize_seq(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_bool(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_i8(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_i16(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_i32(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_i64(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_i128(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_u8(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_u16(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_u32(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_u64(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_u128(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_f32(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_f64(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_char(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_str(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_string(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_bytes(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_option(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_unit(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                    static_cast<void>(name);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                    static_cast<void>(name);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                    static_cast<void>(name);
                    static_cast<void>(std::move(len));
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_map(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                    static_cast<void>(name);
                    static_cast<void>(fields);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                    static_cast<void>(name);
                    static_cast<void>(variants);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_identifier(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename T>
                rusty::Result<rusty::Option<typename T::Value>, Error> next_key_seed(T seed) {
                    return [&]() -> rusty::Result<rusty::Option<typename T::Value>, Error> { auto&& _m = this->next_pair(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& key = std::get<0>(rusty::detail::deref_if_pointer(_mv0)); auto&& value = std::get<1>(rusty::detail::deref_if_pointer(_mv0)); return [&]() -> rusty::Result<rusty::Option<typename T::Value>, Error> { this->value = rusty::Option<serde_core_private::Content>(std::move(value));
return ::de::rusty_ext::deserialize(seed, ContentDeserializer<E>::new_(std::move(key))).map(rusty::Some); }(); } if (_m.is_none()) { return rusty::Result<rusty::Option<typename T::Value>, Error>::Ok(rusty::Option<typename T::Value>(rusty::None)); } return [&]() -> rusty::Result<rusty::Option<typename T::Value>, Error> { rusty::intrinsics::unreachable(); }(); }();
                }
                template<typename T>
                rusty::Result<typename T::Value, Error> next_value_seed(T seed) {
                    auto value = this->value.take();
                    auto value_shadow1 = value.expect("MapAccess::next_value called before next_key");
                    return ::de::rusty_ext::deserialize(seed, ContentDeserializer<E>::new_(std::move(value_shadow1)));
                }
                template<typename TK, typename TV>
                rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error> next_entry_seed(TK kseed, TV vseed) {
                    return [&]() -> rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error> { auto&& _m = this->next_pair(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& key = std::get<0>(rusty::detail::deref_if_pointer(_mv0)); auto&& value = std::get<1>(rusty::detail::deref_if_pointer(_mv0)); return [&]() -> rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error> { auto key_shadow1 = ({ auto&& _m = ::de::rusty_ext::deserialize(kseed, ContentDeserializer<E>::new_(std::move(key))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error>::Err(std::move(err)); } std::move(_match_value).value(); });
auto value_shadow1 = ({ auto&& _m = ::de::rusty_ext::deserialize(vseed, ContentDeserializer<E>::new_(std::move(value))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error>::Err(std::move(err)); } std::move(_match_value).value(); });
return rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error>::Ok(rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>(std::make_tuple(std::move(key_shadow1), std::move(value_shadow1)))); }(); } if (_m.is_none()) { return rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error>::Ok(rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>(rusty::None)); } return [&]() -> rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error> { rusty::intrinsics::unreachable(); }(); }();
                }
                rusty::Option<size_t> size_hint() const {
                    return size_hint::from_bounds(&this->iter);
                }
                template<typename T>
                rusty::Result<rusty::Option<typename T::Value>, Error> next_element_seed(T seed) {
                    return [&]() -> rusty::Result<rusty::Option<typename T::Value>, Error> { auto&& _m = this->next_pair(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& k = std::get<0>(rusty::detail::deref_if_pointer(_mv0)); auto&& v = std::get<1>(rusty::detail::deref_if_pointer(_mv0)); return [&]() -> rusty::Result<rusty::Option<typename T::Value>, Error> { auto de = PairDeserializer(std::move(k), std::move(v), rusty::PhantomData<E>{});
return ::de::rusty_ext::deserialize(seed, std::move(de)).map(rusty::Some); }(); } if (_m.is_none()) { return rusty::Result<rusty::Option<typename T::Value>, Error>::Ok(rusty::Option<typename T::Value>(rusty::None)); } return [&]() -> rusty::Result<rusty::Option<typename T::Value>, Error> { rusty::intrinsics::unreachable(); }(); }();
                }
            };

            template<typename E>
            struct PairDeserializer {
                using Error = E;
                serde_core_private::Content _0;
                serde_core_private::Content _1;
                rusty::PhantomData<E> _2;

                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_bool(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_i8(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_i16(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_i32(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_i64(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_i128(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_u8(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_u16(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_u32(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_u64(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_u128(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_f32(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_f64(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_char(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_str(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_string(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_bytes(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_option(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_unit(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                    static_cast<void>(name);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                    static_cast<void>(name);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                    static_cast<void>(name);
                    static_cast<void>(std::move(len));
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_map(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                    static_cast<void>(name);
                    static_cast<void>(fields);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                    static_cast<void>(name);
                    static_cast<void>(variants);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_identifier(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                    return this->deserialize_seq(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_seq(V visitor) {
                    auto pair_visitor = PairVisitor(rusty::Option<serde_core_private::Content>(std::move(this->_0)), rusty::Option<serde_core_private::Content>(std::move(this->_1)), rusty::PhantomData<E>{});
                    auto pair = ({ auto&& _m = visitor.visit_seq(pair_visitor); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename V::Value, Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                    if (pair_visitor._1.is_none()) {
                        return rusty::Result<typename V::Value, Error>::Ok(std::move(pair));
                    } else {
                        const auto remaining = pair_visitor.size_hint().unwrap();
                        return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_length(2, rusty::addr_of_temp(ExpectedInSeq(2 - remaining))));
                    }
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_tuple(size_t len, V visitor) {
                    if (len == static_cast<size_t>(2)) {
                        return this->deserialize_seq(std::move(visitor));
                    } else {
                        return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_length(2, rusty::addr_of_temp(ExpectedInSeq(std::move(len)))));
                    }
                }
            };

            template<typename E>
            struct PairVisitor {
                using Error = E;
                rusty::Option<serde_core_private::Content> _0;
                rusty::Option<serde_core_private::Content> _1;
                rusty::PhantomData<E> _2;

                template<typename T>
                rusty::Result<rusty::Option<typename T::Value>, Error> next_element_seed(T seed) {
                    if (auto&& _iflet_scrutinee = this->_0.take(); _iflet_scrutinee.is_some()) {
                        decltype(auto) k = _iflet_scrutinee.unwrap();
                        return ::de::rusty_ext::deserialize(seed, ContentDeserializer<E>::new_(std::move(k))).map(rusty::Some);
                    } else if (auto&& _iflet_scrutinee = this->_1.take(); _iflet_scrutinee.is_some()) {
                        decltype(auto) v = _iflet_scrutinee.unwrap();
                        return ::de::rusty_ext::deserialize(seed, ContentDeserializer<E>::new_(std::move(v))).map(rusty::Some);
                    } else {
                        return rusty::Result<rusty::Option<typename T::Value>, Error>::Ok(rusty::Option<typename T::Value>(rusty::None));
                    }
                }
                rusty::Option<size_t> size_hint() const {
                    if (this->_0.is_some()) {
                        return rusty::Option<size_t>(static_cast<size_t>(2));
                    } else if (this->_1.is_some()) {
                        return rusty::Option<size_t>(static_cast<size_t>(1));
                    } else {
                        return rusty::Option<size_t>(static_cast<size_t>(0));
                    }
                }
            };

            struct ExpectedInMap {
                size_t _0;

                rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const;
            };

            template<typename E>
            struct EnumDeserializer {
                using Error = E;
                // Rust-only dependent associated type alias skipped in constrained mode: Variant
                serde_core_private::Content variant;
                rusty::Option<serde_core_private::Content> value;
                rusty::PhantomData<E> err;

                static EnumDeserializer<E> new_(serde_core_private::Content variant, rusty::Option<serde_core_private::Content> value) {
                    return EnumDeserializer<E>{.variant = std::move(variant), .value = std::move(value), .err = rusty::PhantomData<E>{}};
                }
                // Rust-only dependent associated type alias skipped in constrained mode: Variant
                template<typename V>
                rusty::Result<std::tuple<typename V::Value, VariantDeserializer<Error>>, E> variant_seed(V seed) {
                    auto visitor = VariantDeserializer<E>(std::move(this->value), rusty::PhantomData<E>{});
                    return ::de::rusty_ext::deserialize(seed, ContentDeserializer<E>::new_(std::move(this->variant))).map([&](auto&& v) -> std::tuple<typename V::Value, VariantDeserializer<Error>> { return std::make_tuple(std::move(v), std::move(visitor)); });
                }
            };

            template<typename E>
            struct VariantDeserializer {
                using Error = E;
                rusty::Option<serde_core_private::Content> value;
                rusty::PhantomData<E> err;

                rusty::Result<std::tuple<>, E> unit_variant() {
                    return [&]() -> rusty::Result<std::tuple<>, E> { auto&& _m = this->value; if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& value = _mv0; return ::de::rusty_ext::deserialize(rusty::PhantomData<std::tuple<>>{}, ContentDeserializer<E>::new_(std::move(value))); } if (_m.is_none()) { return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple()); } return [&]() -> rusty::Result<std::tuple<>, E> { rusty::intrinsics::unreachable(); }(); }();
                }
                template<typename T>
                auto newtype_variant_seed(T seed) {
                    return [&]() -> rusty::Result<typename T::Value, E> { auto&& _m = this->value; if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& value = _mv0; return ::de::rusty_ext::deserialize(seed, ContentDeserializer<E>::new_(std::move(value))); } if (_m.is_none()) { return rusty::Result<typename T::Value, E>::Err(E::invalid_type(::de::Unexpected::UnitVariant, rusty::addr_of_temp("newtype variant"))); } return [&]() -> rusty::Result<typename T::Value, E> { rusty::intrinsics::unreachable(); }(); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> tuple_variant(size_t _len, V visitor) {
                    return [&]() -> rusty::Result<typename V::Value, Error> { auto&& _m = this->value; if (_m.is_some()) { auto&& _mv0 = std::as_const(_m).unwrap(); if (std::holds_alternative<serde_core_private::Content_Seq>(rusty::detail::deref_if_pointer(_mv0))) { auto&& v = std::get<serde_core_private::Content_Seq>(rusty::detail::deref_if_pointer(_mv0))._0; return (SeqDeserializer<E>::new_(std::move(v))).deserialize_any(std::move(visitor)); } } if (_m.is_some()) { auto&& _mv1 = _m.unwrap(); auto&& other = _mv1; return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_type(content_unexpected(rusty::detail::deref_if_pointer_like(other)), rusty::addr_of_temp("tuple variant"))); } if (_m.is_none()) { return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_type(::de::Unexpected::UnitVariant, rusty::addr_of_temp("tuple variant"))); } return [&]() -> rusty::Result<typename V::Value, Error> { rusty::intrinsics::unreachable(); }(); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> struct_variant(std::span<const std::string_view> _fields, V visitor) {
                    return [&]() -> rusty::Result<typename V::Value, Error> { auto&& _m = this->value; if (_m.is_some()) { auto&& _mv0 = std::as_const(_m).unwrap(); if (std::holds_alternative<serde_core_private::Content_Map>(rusty::detail::deref_if_pointer(_mv0))) { auto&& v = std::get<serde_core_private::Content_Map>(rusty::detail::deref_if_pointer(_mv0))._0; return (MapDeserializer<E>::new_(std::move(v))).deserialize_any(std::move(visitor)); } } if (_m.is_some()) { auto&& _mv1 = std::as_const(_m).unwrap(); if (std::holds_alternative<serde_core_private::Content_Seq>(rusty::detail::deref_if_pointer(_mv1))) { auto&& v = std::get<serde_core_private::Content_Seq>(rusty::detail::deref_if_pointer(_mv1))._0; return (SeqDeserializer<E>::new_(std::move(v))).deserialize_any(std::move(visitor)); } } if (_m.is_some()) { auto&& _mv2 = _m.unwrap(); auto&& other = _mv2; return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_type(content_unexpected(rusty::detail::deref_if_pointer_like(other)), rusty::addr_of_temp("struct variant"))); } if (_m.is_none()) { return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_type(::de::Unexpected::UnitVariant, rusty::addr_of_temp("struct variant"))); } return [&]() -> rusty::Result<typename V::Value, Error> { rusty::intrinsics::unreachable(); }(); }();
                }
            };

            /// Not public API.
            template<typename E>
            struct ContentRefDeserializer {
                using Error = E;
                using Deserializer = ContentRefDeserializer<E>;
                const serde_core_private::Content& content;
                rusty::PhantomData<E> err;

                E invalid_type(const auto& exp) {
                    return E::invalid_type(content_unexpected(rusty::detail::deref_if_pointer_like(this->content)), exp);
                }
                template<typename V>
                auto deserialize_integer(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](const serde_core_private::Content_U8& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u8(std::move(v)); }, [&](const serde_core_private::Content_U16& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u16(std::move(v)); }, [&](const serde_core_private::Content_U32& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u32(std::move(v)); }, [&](const serde_core_private::Content_U64& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u64(std::move(v)); }, [&](const serde_core_private::Content_I8& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i8(std::move(v)); }, [&](const serde_core_private::Content_I16& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i16(std::move(v)); }, [&](const serde_core_private::Content_I32& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i32(std::move(v)); }, [&](const serde_core_private::Content_I64& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i64(std::move(v)); }, [&](const auto&) -> rusty::Result<typename V::Value, E> { return rusty::Result<typename V::Value, E>::Err(this->invalid_type(visitor)); } }, _m); }();
                }
                template<typename V>
                auto deserialize_float(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](const serde_core_private::Content_F32& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_f32(std::move(v)); }, [&](const serde_core_private::Content_F64& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_f64(std::move(v)); }, [&](const serde_core_private::Content_U8& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u8(std::move(v)); }, [&](const serde_core_private::Content_U16& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u16(std::move(v)); }, [&](const serde_core_private::Content_U32& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u32(std::move(v)); }, [&](const serde_core_private::Content_U64& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u64(std::move(v)); }, [&](const serde_core_private::Content_I8& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i8(std::move(v)); }, [&](const serde_core_private::Content_I16& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i16(std::move(v)); }, [&](const serde_core_private::Content_I32& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i32(std::move(v)); }, [&](const serde_core_private::Content_I64& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i64(std::move(v)); }, [&](const auto&) -> rusty::Result<typename V::Value, E> { return rusty::Result<typename V::Value, E>::Err(this->invalid_type(visitor)); } }, _m); }();
                }
                template<typename V>
                auto deserialize_any(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](const serde_core_private::Content_Bool& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_bool(std::move(v)); }, [&](const serde_core_private::Content_U8& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u8(std::move(v)); }, [&](const serde_core_private::Content_U16& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u16(std::move(v)); }, [&](const serde_core_private::Content_U32& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u32(std::move(v)); }, [&](const serde_core_private::Content_U64& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_u64(std::move(v)); }, [&](const serde_core_private::Content_I8& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i8(std::move(v)); }, [&](const serde_core_private::Content_I16& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i16(std::move(v)); }, [&](const serde_core_private::Content_I32& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i32(std::move(v)); }, [&](const serde_core_private::Content_I64& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_i64(std::move(v)); }, [&](const serde_core_private::Content_F32& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_f32(std::move(v)); }, [&](const serde_core_private::Content_F64& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_f64(std::move(v)); }, [&](const serde_core_private::Content_Char& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_char(std::move(v)); }, [&](const serde_core_private::Content_String& _v) -> rusty::Result<typename V::Value, E> { const auto& v = _v._0; return visitor.visit_str(std::move(rusty::to_string_view(v))); }, [&](const serde_core_private::Content_Str& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_borrowed_str(std::move(rusty::to_string_view(v))); }, [&](const serde_core_private::Content_ByteBuf& _v) -> rusty::Result<typename V::Value, E> { const auto& v = _v._0; return visitor.visit_bytes(v); }, [&](const serde_core_private::Content_Bytes& _v) -> rusty::Result<typename V::Value, E> { auto&& v = _v._0; return visitor.visit_borrowed_bytes(v); }, [&](const serde_core_private::Content_Unit&) -> rusty::Result<typename V::Value, E> { return visitor.visit_unit(); }, [&](const serde_core_private::Content_None&) -> rusty::Result<typename V::Value, E> { return visitor.visit_none(); }, [&](const serde_core_private::Content_Some& _v) -> rusty::Result<typename V::Value, E> { const auto& v = _v._0; return visitor.visit_some(ContentRefDeserializer<E>::new_(rusty::detail::deref_if_pointer_like(v))); }, [&](const serde_core_private::Content_Newtype& _v) -> rusty::Result<typename V::Value, E> { const auto& v = _v._0; return visitor.visit_newtype_struct(ContentRefDeserializer<E>::new_(rusty::detail::deref_if_pointer_like(v))); }, [&](const serde_core_private::Content_Seq& _v) -> rusty::Result<typename V::Value, E> { const auto& v = _v._0; return visit_content_seq_ref(v, std::move(visitor)); }, [&](const serde_core_private::Content_Map& _v) -> rusty::Result<typename V::Value, E> { const auto& v = _v._0; return visit_content_map_ref(v, std::move(visitor)); } }, _m); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_bool(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](const serde_core_private::Content_Bool& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_bool(std::move(v)); }, [&](const auto&) -> rusty::Result<typename V::Value, Error> { return rusty::Result<typename V::Value, Error>::Err(this->invalid_type(visitor)); } }, _m); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_i8(V visitor) {
                    return this->deserialize_integer(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_i16(V visitor) {
                    return this->deserialize_integer(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_i32(V visitor) {
                    return this->deserialize_integer(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_i64(V visitor) {
                    return this->deserialize_integer(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_u8(V visitor) {
                    return this->deserialize_integer(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_u16(V visitor) {
                    return this->deserialize_integer(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_u32(V visitor) {
                    return this->deserialize_integer(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_u64(V visitor) {
                    return this->deserialize_integer(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_f32(V visitor) {
                    return this->deserialize_float(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_f64(V visitor) {
                    return this->deserialize_float(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_char(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](const serde_core_private::Content_Char& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_char(std::move(v)); }, [&](const serde_core_private::Content_String& _v) -> rusty::Result<typename V::Value, Error> { const auto& v = _v._0; return visitor.visit_str(std::move(rusty::to_string_view(v))); }, [&](const serde_core_private::Content_Str& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_borrowed_str(std::move(rusty::to_string_view(v))); }, [&](const auto&) -> rusty::Result<typename V::Value, Error> { return rusty::Result<typename V::Value, Error>::Err(this->invalid_type(visitor)); } }, _m); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_str(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](const serde_core_private::Content_String& _v) -> rusty::Result<typename V::Value, Error> { const auto& v = _v._0; return visitor.visit_str(std::move(rusty::to_string_view(v))); }, [&](const serde_core_private::Content_Str& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_borrowed_str(std::move(rusty::to_string_view(v))); }, [&](const serde_core_private::Content_ByteBuf& _v) -> rusty::Result<typename V::Value, Error> { const auto& v = _v._0; return visitor.visit_bytes(v); }, [&](const serde_core_private::Content_Bytes& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_borrowed_bytes(v); }, [&](const auto&) -> rusty::Result<typename V::Value, Error> { return rusty::Result<typename V::Value, Error>::Err(this->invalid_type(visitor)); } }, _m); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_string(V visitor) {
                    return this->deserialize_str(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_bytes(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](const serde_core_private::Content_String& _v) -> rusty::Result<typename V::Value, Error> { const auto& v = _v._0; return visitor.visit_str(std::move(rusty::to_string_view(v))); }, [&](const serde_core_private::Content_Str& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_borrowed_str(std::move(rusty::to_string_view(v))); }, [&](const serde_core_private::Content_ByteBuf& _v) -> rusty::Result<typename V::Value, Error> { const auto& v = _v._0; return visitor.visit_bytes(v); }, [&](const serde_core_private::Content_Bytes& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_borrowed_bytes(v); }, [&](const serde_core_private::Content_Seq& _v) -> rusty::Result<typename V::Value, Error> { const auto& v = _v._0; return visit_content_seq_ref(v, std::move(visitor)); }, [&](const auto&) -> rusty::Result<typename V::Value, Error> { return rusty::Result<typename V::Value, Error>::Err(this->invalid_type(visitor)); } }, _m); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_byte_buf(V visitor) {
                    return this->deserialize_bytes(std::move(visitor));
                }
                template<typename V>
                auto deserialize_option(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](const serde_core_private::Content_None&) -> rusty::Result<typename V::Value, E> { return visitor.visit_none(); }, [&](const serde_core_private::Content_Some& _v) -> rusty::Result<typename V::Value, E> { const auto& v = _v._0; return visitor.visit_some(ContentRefDeserializer<E>::new_(rusty::detail::deref_if_pointer_like(v))); }, [&](const serde_core_private::Content_Unit&) -> rusty::Result<typename V::Value, E> { return visitor.visit_unit(); }, [&](const auto&) -> rusty::Result<typename V::Value, E> { return visitor.visit_some(std::move((*this))); } }, _m); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_unit(V visitor) {
                    return ({ auto&& _m = this->content; std::optional<rusty::Result<typename V::Value, Error>> _match_value; bool _m_matched = false; if (!_m_matched && (std::holds_alternative<serde_core_private::Content_Unit>(_m))) { _match_value.emplace(std::move(visitor.visit_unit())); _m_matched = true; } if (!_m_matched) { _match_value.emplace(std::move(rusty::Result<typename V::Value, Error>::Err(this->invalid_type(visitor)))); _m_matched = true; } if (!_m_matched) { rusty::intrinsics::unreachable(); } std::move(_match_value).value(); });
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_unit_struct(std::string_view _name, V visitor) {
                    return this->deserialize_unit(std::move(visitor));
                }
                template<typename V>
                auto deserialize_newtype_struct(std::string_view _name, V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](const serde_core_private::Content_Newtype& _v) -> rusty::Result<typename V::Value, E> { const auto& v = _v._0; return visitor.visit_newtype_struct(ContentRefDeserializer<E>::new_(rusty::detail::deref_if_pointer_like(v))); }, [&](const auto&) -> rusty::Result<typename V::Value, E> { return visitor.visit_newtype_struct(std::move((*this))); } }, _m); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_seq(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](const serde_core_private::Content_Seq& _v) -> rusty::Result<typename V::Value, Error> { const auto& v = _v._0; return visit_content_seq_ref(v, std::move(visitor)); }, [&](const auto&) -> rusty::Result<typename V::Value, Error> { return rusty::Result<typename V::Value, Error>::Err(this->invalid_type(visitor)); } }, _m); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_tuple(size_t _len, V visitor) {
                    return this->deserialize_seq(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_tuple_struct(std::string_view _name, size_t _len, V visitor) {
                    return this->deserialize_seq(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_map(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](const serde_core_private::Content_Map& _v) -> rusty::Result<typename V::Value, Error> { const auto& v = _v._0; return visit_content_map_ref(v, std::move(visitor)); }, [&](const auto&) -> rusty::Result<typename V::Value, Error> { return rusty::Result<typename V::Value, Error>::Err(this->invalid_type(visitor)); } }, _m); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_struct(std::string_view _name, std::span<const std::string_view> _fields, V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](const serde_core_private::Content_Seq& _v) -> rusty::Result<typename V::Value, Error> { const auto& v = _v._0; return visit_content_seq_ref(v, std::move(visitor)); }, [&](const serde_core_private::Content_Map& _v) -> rusty::Result<typename V::Value, Error> { const auto& v = _v._0; return visit_content_map_ref(v, std::move(visitor)); }, [&](const auto&) -> rusty::Result<typename V::Value, Error> { return rusty::Result<typename V::Value, Error>::Err(this->invalid_type(visitor)); } }, _m); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_enum(std::string_view _name, std::span<const std::string_view> _variants, V visitor) {
                    auto [variant, value] = rusty::detail::deref_if_pointer_like([&]() { auto&& _m = this->content; return std::visit(overloaded { [&](const serde_core_private::Content_Map& _v) { const auto& value = _v._0; return [&]() { auto iter = rusty::iter(value);
auto [variant, value] = rusty::detail::deref_if_pointer_like(({ auto&& _m = iter.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& v = _mv;
_match_value.emplace(std::move(v)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_value(::de::Unexpected::Map, rusty::addr_of_temp("map with a single key"))); } std::move(_match_value).value(); }));
if (iter.next().is_some()) {
    return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_value(::de::Unexpected::Map, rusty::addr_of_temp("map with a single key")));
}
return std::make_tuple(std::move(variant), rusty::Some(value)); }(); }, [&](const auto&) { return rusty::intrinsics::unreachable(); }, [&](const auto& other) { return (static_cast<void>([&]() { return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_type(content_unexpected(rusty::detail::deref_if_pointer_like(other)), rusty::addr_of_temp("string or map"))); }()), rusty::intrinsics::unreachable()); } }, _m); }());
                    return visitor.visit_enum(EnumRefDeserializer<E>(std::move(variant), std::move(value), rusty::PhantomData<E>{}));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_identifier(V visitor) {
                    return [&]() { auto&& _m = this->content; return std::visit(overloaded { [&](const serde_core_private::Content_String& _v) -> rusty::Result<typename V::Value, Error> { const auto& v = _v._0; return visitor.visit_str(std::move(rusty::to_string_view(v))); }, [&](const serde_core_private::Content_Str& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_borrowed_str(std::move(rusty::to_string_view(v))); }, [&](const serde_core_private::Content_ByteBuf& _v) -> rusty::Result<typename V::Value, Error> { const auto& v = _v._0; return visitor.visit_bytes(v); }, [&](const serde_core_private::Content_Bytes& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_borrowed_bytes(v); }, [&](const serde_core_private::Content_U8& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_u8(std::move(v)); }, [&](const serde_core_private::Content_U64& _v) -> rusty::Result<typename V::Value, Error> { auto&& v = _v._0; return visitor.visit_u64(std::move(v)); }, [&](const auto&) -> rusty::Result<typename V::Value, Error> { return rusty::Result<typename V::Value, Error>::Err(this->invalid_type(visitor)); } }, _m); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_ignored_any(V visitor) {
                    return visitor.visit_unit();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> __deserialize_content_v1(V visitor) {
                    static_cast<void>(std::move(visitor));
                    return rusty::Result<typename V::Value, Error>::Ok(content_clone(rusty::detail::deref_if_pointer_like(this->content)));
                }
                static ContentRefDeserializer<E> new_(const serde_core_private::Content& content) {
                    return ContentRefDeserializer<E>{.content = content, .err = rusty::PhantomData<E>{}};
                }
                ContentRefDeserializer<E> clone() const {
                    return {.content = this->content, .err = rusty::clone(this->err)};
                }
                ContentRefDeserializer<E> into_deserializer() {
                    return std::move((*this));
                }
            };

            template<typename E>
            struct SeqRefDeserializer {
                using Error = E;
                decltype(rusty::iter(std::declval<std::span<const serde_core_private::Content>>())) iter;
                size_t count;
                rusty::PhantomData<E> marker;

                static SeqRefDeserializer<E> new_(std::span<const serde_core_private::Content> content) {
                    return SeqRefDeserializer<E>{.iter = rusty::iter(content), .count = static_cast<size_t>(0), .marker = rusty::PhantomData<E>{}};
                }
                rusty::Result<std::tuple<>, E> end() {
                    const auto remaining = rusty::count(this->iter);
                    if (remaining == 0) {
                        return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
                    } else {
                        return rusty::Result<std::tuple<>, E>::Err(E::invalid_length(this->count + remaining, rusty::addr_of_temp(ExpectedInSeq(std::move(this->count)))));
                    }
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                    auto v = ({ auto&& _m = visitor.visit_seq((*this)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename V::Value, Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                    {
                        auto&& _m = this->end();
                        bool _m_matched = false;
                        if (!_m_matched) {
                            if (_m.is_ok()) {
                                auto&& _mv0 = _m.unwrap();
                                auto&& val = _mv0;
                                val;
                                _m_matched = true;
                            }
                        }
                        if (!_m_matched) {
                            if (_m.is_err()) {
                                auto&& _mv1 = _m.unwrap_err();
                                auto&& err = _mv1;
                                return rusty::Result<typename V::Value, Error>::Err(std::move(err));
                                _m_matched = true;
                            }
                        }
                    }
                    return rusty::Result<typename V::Value, Error>::Ok(std::move(v));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_bool(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_i8(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_i16(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_i32(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_i64(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_i128(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_u8(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_u16(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_u32(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_u64(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_u128(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_f32(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_f64(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_char(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_str(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_string(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_bytes(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_option(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_unit(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                    static_cast<void>(name);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                    static_cast<void>(name);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_seq(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                    static_cast<void>(std::move(len));
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                    static_cast<void>(name);
                    static_cast<void>(std::move(len));
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_map(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                    static_cast<void>(name);
                    static_cast<void>(fields);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                    static_cast<void>(name);
                    static_cast<void>(variants);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_identifier(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename SeqRefDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<rusty::Option<typename V::Value>, Error> next_element_seed(V seed) {
                    return [&]() -> rusty::Result<rusty::Option<typename V::Value>, Error> { auto&& _m = this->iter.next(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& value = _mv0; return [&]() -> rusty::Result<rusty::Option<typename V::Value>, Error> { [&]() { static_cast<void>(this->count += 1); return std::make_tuple(); }();
return ::de::rusty_ext::deserialize(seed, ContentRefDeserializer<E>::new_(rusty::detail::deref_if_pointer_like(value))).map(rusty::Some); }(); } if (_m.is_none()) { return rusty::Result<rusty::Option<typename V::Value>, Error>::Ok(rusty::Option<typename V::Value>(rusty::None)); } return [&]() -> rusty::Result<rusty::Option<typename V::Value>, Error> { rusty::intrinsics::unreachable(); }(); }();
                }
                rusty::Option<size_t> size_hint() const {
                    return size_hint::from_bounds(&this->iter);
                }
            };

            template<typename E>
            struct MapRefDeserializer {
                using Error = E;
                decltype(rusty::iter(std::declval<std::span<const std::tuple<serde_core_private::Content, serde_core_private::Content>>>())) iter;
                rusty::Option<const serde_core_private::Content&> value;
                size_t count;
                rusty::PhantomData<E> error;

                static MapRefDeserializer<E> new_(std::span<const std::tuple<serde_core_private::Content, serde_core_private::Content>> content) {
                    return MapRefDeserializer<E>{.iter = rusty::iter(content), .value = rusty::Option<const serde_core_private::Content&>(rusty::None), .count = static_cast<size_t>(0), .error = rusty::PhantomData<E>{}};
                }
                rusty::Result<std::tuple<>, E> end() {
                    const auto remaining = rusty::count(this->iter);
                    if (remaining == 0) {
                        return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
                    } else {
                        return rusty::Result<std::tuple<>, E>::Err(E::invalid_length(this->count + remaining, rusty::addr_of_temp(ExpectedInMap(std::move(this->count)))));
                    }
                }
                rusty::Option<std::tuple<const serde_core_private::Content&, const serde_core_private::Content&>> next_pair() {
                    return [&]() -> rusty::Option<std::tuple<const serde_core_private::Content&, const serde_core_private::Content&>> { auto&& _m = this->iter.next(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& k = std::get<0>(rusty::detail::deref_if_pointer(_mv0)); auto&& v = std::get<1>(rusty::detail::deref_if_pointer(_mv0)); return [&]() -> rusty::Option<std::tuple<const serde_core_private::Content&, const serde_core_private::Content&>> { [&]() { static_cast<void>(this->count += 1); return std::make_tuple(); }();
return rusty::Option<std::tuple<const serde_core_private::Content&, const serde_core_private::Content&>>(std::tuple<const serde_core_private::Content&, const serde_core_private::Content&>{k, v}); }(); } if (_m.is_none()) { return rusty::Option<std::tuple<const serde_core_private::Content&, const serde_core_private::Content&>>(rusty::None); } return [&]() -> rusty::Option<std::tuple<const serde_core_private::Content&, const serde_core_private::Content&>> { rusty::intrinsics::unreachable(); }(); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                    auto value = ({ auto&& _m = visitor.visit_map((*this)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename V::Value, Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                    {
                        auto&& _m = this->end();
                        bool _m_matched = false;
                        if (!_m_matched) {
                            if (_m.is_ok()) {
                                auto&& _mv0 = _m.unwrap();
                                auto&& val = _mv0;
                                val;
                                _m_matched = true;
                            }
                        }
                        if (!_m_matched) {
                            if (_m.is_err()) {
                                auto&& _mv1 = _m.unwrap_err();
                                auto&& err = _mv1;
                                return rusty::Result<typename V::Value, Error>::Err(std::move(err));
                                _m_matched = true;
                            }
                        }
                    }
                    return rusty::Result<typename V::Value, Error>::Ok(std::move(value));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_seq(V visitor) {
                    auto value = ({ auto&& _m = visitor.visit_seq((*this)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename V::Value, Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                    {
                        auto&& _m = this->end();
                        bool _m_matched = false;
                        if (!_m_matched) {
                            if (_m.is_ok()) {
                                auto&& _mv0 = _m.unwrap();
                                auto&& val = _mv0;
                                val;
                                _m_matched = true;
                            }
                        }
                        if (!_m_matched) {
                            if (_m.is_err()) {
                                auto&& _mv1 = _m.unwrap_err();
                                auto&& err = _mv1;
                                return rusty::Result<typename V::Value, Error>::Err(std::move(err));
                                _m_matched = true;
                            }
                        }
                    }
                    return rusty::Result<typename V::Value, Error>::Ok(std::move(value));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_tuple(size_t len, V visitor) {
                    static_cast<void>(std::move(len));
                    return this->deserialize_seq(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_bool(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_i8(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_i16(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_i32(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_i64(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_i128(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_u8(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_u16(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_u32(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_u64(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_u128(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_f32(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_f64(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_char(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_str(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_string(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_bytes(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_option(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_unit(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                    static_cast<void>(name);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                    static_cast<void>(name);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                    static_cast<void>(name);
                    static_cast<void>(std::move(len));
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_map(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                    static_cast<void>(name);
                    static_cast<void>(fields);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                    static_cast<void>(name);
                    static_cast<void>(variants);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_identifier(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename MapRefDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename T>
                rusty::Result<rusty::Option<typename T::Value>, Error> next_key_seed(T seed) {
                    return [&]() -> rusty::Result<rusty::Option<typename T::Value>, Error> { auto&& _m = this->next_pair(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& key = std::get<0>(rusty::detail::deref_if_pointer(_mv0)); auto&& value = std::get<1>(rusty::detail::deref_if_pointer(_mv0)); return [&]() -> rusty::Result<rusty::Option<typename T::Value>, Error> { this->value = rusty::Option<const serde_core_private::Content&>(value);
return ::de::rusty_ext::deserialize(seed, ContentRefDeserializer<E>::new_(rusty::detail::deref_if_pointer_like(key))).map(rusty::Some); }(); } if (_m.is_none()) { return rusty::Result<rusty::Option<typename T::Value>, Error>::Ok(rusty::Option<typename T::Value>(rusty::None)); } return [&]() -> rusty::Result<rusty::Option<typename T::Value>, Error> { rusty::intrinsics::unreachable(); }(); }();
                }
                template<typename T>
                rusty::Result<typename T::Value, Error> next_value_seed(T seed) {
                    auto value = this->value.take();
                    auto value_shadow1 = value.expect("MapAccess::next_value called before next_key");
                    return ::de::rusty_ext::deserialize(seed, ContentRefDeserializer<E>::new_(rusty::detail::deref_if_pointer_like(value_shadow1)));
                }
                template<typename TK, typename TV>
                rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error> next_entry_seed(TK kseed, TV vseed) {
                    return [&]() -> rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error> { auto&& _m = this->next_pair(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& key = std::get<0>(rusty::detail::deref_if_pointer(_mv0)); auto&& value = std::get<1>(rusty::detail::deref_if_pointer(_mv0)); return [&]() -> rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error> { auto key_shadow1 = ({ auto&& _m = ::de::rusty_ext::deserialize(kseed, ContentRefDeserializer<E>::new_(rusty::detail::deref_if_pointer_like(key))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error>::Err(std::move(err)); } std::move(_match_value).value(); });
auto value_shadow1 = ({ auto&& _m = ::de::rusty_ext::deserialize(vseed, ContentRefDeserializer<E>::new_(rusty::detail::deref_if_pointer_like(value))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error>::Err(std::move(err)); } std::move(_match_value).value(); });
return rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error>::Ok(rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>(std::make_tuple(std::move(key_shadow1), std::move(value_shadow1)))); }(); } if (_m.is_none()) { return rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error>::Ok(rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>(rusty::None)); } return [&]() -> rusty::Result<rusty::Option<std::tuple<typename TK::Value, typename TV::Value>>, Error> { rusty::intrinsics::unreachable(); }(); }();
                }
                rusty::Option<size_t> size_hint() const {
                    return size_hint::from_bounds(&this->iter);
                }
                template<typename T>
                rusty::Result<rusty::Option<typename T::Value>, Error> next_element_seed(T seed) {
                    return [&]() -> rusty::Result<rusty::Option<typename T::Value>, Error> { auto&& _m = this->next_pair(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& k = std::get<0>(rusty::detail::deref_if_pointer(_mv0)); auto&& v = std::get<1>(rusty::detail::deref_if_pointer(_mv0)); return [&]() -> rusty::Result<rusty::Option<typename T::Value>, Error> { auto de = PairRefDeserializer(rusty::detail::deref_if_pointer_like(k), rusty::detail::deref_if_pointer_like(v), rusty::PhantomData<E>{});
return ::de::rusty_ext::deserialize(seed, std::move(de)).map(rusty::Some); }(); } if (_m.is_none()) { return rusty::Result<rusty::Option<typename T::Value>, Error>::Ok(rusty::Option<typename T::Value>(rusty::None)); } return [&]() -> rusty::Result<rusty::Option<typename T::Value>, Error> { rusty::intrinsics::unreachable(); }(); }();
                }
            };

            template<typename E>
            struct PairRefDeserializer {
                using Error = E;
                const serde_core_private::Content& _0;
                const serde_core_private::Content& _1;
                rusty::PhantomData<E> _2;

                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_bool(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_i8(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_i16(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_i32(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_i64(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_i128(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_u8(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_u16(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_u32(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_u64(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_u128(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_f32(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_f64(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_char(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_str(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_string(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_bytes(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_option(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_unit(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                    static_cast<void>(name);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                    static_cast<void>(name);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                    static_cast<void>(name);
                    static_cast<void>(std::move(len));
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_map(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                    static_cast<void>(name);
                    static_cast<void>(fields);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                    static_cast<void>(name);
                    static_cast<void>(variants);
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_identifier(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, typename PairRefDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                    return this->deserialize_any(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                    return this->deserialize_seq(std::move(visitor));
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_seq(V visitor) {
                    auto pair_visitor = PairRefVisitor(rusty::Option<const serde_core_private::Content&>(this->_0), rusty::Option<const serde_core_private::Content&>(this->_1), rusty::PhantomData<E>{});
                    auto pair = ({ auto&& _m = visitor.visit_seq(pair_visitor); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename V::Value, Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                    if (pair_visitor._1.is_none()) {
                        return rusty::Result<typename V::Value, Error>::Ok(std::move(pair));
                    } else {
                        const auto remaining = pair_visitor.size_hint().unwrap();
                        return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_length(2, rusty::addr_of_temp(ExpectedInSeq(2 - remaining))));
                    }
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> deserialize_tuple(size_t len, V visitor) {
                    if (len == static_cast<size_t>(2)) {
                        return this->deserialize_seq(std::move(visitor));
                    } else {
                        return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_length(2, rusty::addr_of_temp(ExpectedInSeq(std::move(len)))));
                    }
                }
            };

            template<typename E>
            struct PairRefVisitor {
                using Error = E;
                rusty::Option<const serde_core_private::Content&> _0;
                rusty::Option<const serde_core_private::Content&> _1;
                rusty::PhantomData<E> _2;

                template<typename T>
                rusty::Result<rusty::Option<typename T::Value>, Error> next_element_seed(T seed) {
                    if (auto&& _iflet_scrutinee = this->_0.take(); _iflet_scrutinee.is_some()) {
                        decltype(auto) k = _iflet_scrutinee.unwrap();
                        return ::de::rusty_ext::deserialize(seed, ContentRefDeserializer<E>::new_(rusty::detail::deref_if_pointer_like(k))).map(rusty::Some);
                    } else if (auto&& _iflet_scrutinee = this->_1.take(); _iflet_scrutinee.is_some()) {
                        decltype(auto) v = _iflet_scrutinee.unwrap();
                        return ::de::rusty_ext::deserialize(seed, ContentRefDeserializer<E>::new_(rusty::detail::deref_if_pointer_like(v))).map(rusty::Some);
                    } else {
                        return rusty::Result<rusty::Option<typename T::Value>, Error>::Ok(rusty::Option<typename T::Value>(rusty::None));
                    }
                }
                rusty::Option<size_t> size_hint() const {
                    if (this->_0.is_some()) {
                        return rusty::Option<size_t>(static_cast<size_t>(2));
                    } else if (this->_1.is_some()) {
                        return rusty::Option<size_t>(static_cast<size_t>(1));
                    } else {
                        return rusty::Option<size_t>(static_cast<size_t>(0));
                    }
                }
            };

            template<typename E>
            struct EnumRefDeserializer {
                using Error = E;
                // Rust-only dependent associated type alias skipped in constrained mode: Variant
                const serde_core_private::Content& variant;
                rusty::Option<const serde_core_private::Content&> value;
                rusty::PhantomData<E> err;

                // Rust-only dependent associated type alias skipped in constrained mode: Variant
                template<typename V>
                rusty::Result<std::tuple<typename V::Value, VariantRefDeserializer<Error>>, Error> variant_seed(V seed) {
                    auto visitor = VariantRefDeserializer<E>(std::move(this->value), rusty::PhantomData<E>{});
                    return ::de::rusty_ext::deserialize(seed, ContentRefDeserializer<E>::new_(rusty::detail::deref_if_pointer_like(this->variant))).map([&](auto&& v) -> std::tuple<typename V::Value, VariantRefDeserializer<Error>> { return std::make_tuple(std::move(v), std::move(visitor)); });
                }
            };

            template<typename E>
            struct VariantRefDeserializer {
                using Error = E;
                rusty::Option<const serde_core_private::Content&> value;
                rusty::PhantomData<E> err;

                rusty::Result<std::tuple<>, E> unit_variant() {
                    return [&]() -> rusty::Result<std::tuple<>, E> { auto&& _m = this->value; if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& value = _mv0; return ::de::rusty_ext::deserialize(rusty::PhantomData<std::tuple<>>{}, ContentRefDeserializer<E>::new_(rusty::detail::deref_if_pointer_like(value))); } if (_m.is_none()) { return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple()); } return [&]() -> rusty::Result<std::tuple<>, E> { rusty::intrinsics::unreachable(); }(); }();
                }
                template<typename T>
                auto newtype_variant_seed(T seed) {
                    return [&]() -> rusty::Result<typename T::Value, E> { auto&& _m = this->value; if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& value = _mv0; return ::de::rusty_ext::deserialize(seed, ContentRefDeserializer<E>::new_(rusty::detail::deref_if_pointer_like(value))); } if (_m.is_none()) { return rusty::Result<typename T::Value, E>::Err(E::invalid_type(::de::Unexpected::UnitVariant, rusty::addr_of_temp("newtype variant"))); } return [&]() -> rusty::Result<typename T::Value, E> { rusty::intrinsics::unreachable(); }(); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> tuple_variant(size_t _len, V visitor) {
                    return [&]() -> rusty::Result<typename V::Value, Error> { auto&& _m = this->value; if (_m.is_some()) { auto&& _mv0 = std::as_const(_m).unwrap(); if (std::holds_alternative<serde_core_private::Content_Seq>(rusty::detail::deref_if_pointer(_mv0))) { auto&& v = std::get<serde_core_private::Content_Seq>(rusty::detail::deref_if_pointer(_mv0))._0; return visit_content_seq_ref(v, std::move(visitor)); } } if (_m.is_some()) { auto&& _mv1 = _m.unwrap(); auto&& other = _mv1; return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_type(content_unexpected(rusty::detail::deref_if_pointer_like(other)), rusty::addr_of_temp("tuple variant"))); } if (_m.is_none()) { return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_type(::de::Unexpected::UnitVariant, rusty::addr_of_temp("tuple variant"))); } return [&]() -> rusty::Result<typename V::Value, Error> { rusty::intrinsics::unreachable(); }(); }();
                }
                template<typename V>
                rusty::Result<typename V::Value, Error> struct_variant(std::span<const std::string_view> _fields, V visitor) {
                    return [&]() -> rusty::Result<typename V::Value, Error> { auto&& _m = this->value; if (_m.is_some()) { auto&& _mv0 = std::as_const(_m).unwrap(); if (std::holds_alternative<serde_core_private::Content_Map>(rusty::detail::deref_if_pointer(_mv0))) { auto&& v = std::get<serde_core_private::Content_Map>(rusty::detail::deref_if_pointer(_mv0))._0; return visit_content_map_ref(v, std::move(visitor)); } } if (_m.is_some()) { auto&& _mv1 = std::as_const(_m).unwrap(); if (std::holds_alternative<serde_core_private::Content_Seq>(rusty::detail::deref_if_pointer(_mv1))) { auto&& v = std::get<serde_core_private::Content_Seq>(rusty::detail::deref_if_pointer(_mv1))._0; return visit_content_seq_ref(v, std::move(visitor)); } } if (_m.is_some()) { auto&& _mv2 = _m.unwrap(); auto&& other = _mv2; return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_type(content_unexpected(rusty::detail::deref_if_pointer_like(other)), rusty::addr_of_temp("struct variant"))); } if (_m.is_none()) { return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::invalid_type(::de::Unexpected::UnitVariant, rusty::addr_of_temp("struct variant"))); } return [&]() -> rusty::Result<typename V::Value, Error> { rusty::intrinsics::unreachable(); }(); }();
                }
            };

            /// Visitor for deserializing an internally tagged unit variant.
            ///
            /// Not public API.
            struct InternallyTaggedUnitVisitor {
                using Value = std::tuple<>;
                std::string_view type_name;
                std::string_view variant_name;

                static InternallyTaggedUnitVisitor new_(std::string_view type_name, std::string_view variant_name);
                rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const;
                template<typename S>
                auto visit_seq(S _arg1);
                template<typename M>
                auto visit_map(M access);
            };

            /// Visitor for deserializing an untagged unit variant.
            ///
            /// Not public API.
            struct UntaggedUnitVisitor {
                using Value = std::tuple<>;
                std::string_view type_name;
                std::string_view variant_name;

                static UntaggedUnitVisitor new_(std::string_view type_name, std::string_view variant_name);
                rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const;
                template<typename E>
                rusty::Result<std::tuple<>, E> visit_unit();
                template<typename E>
                rusty::Result<std::tuple<>, E> visit_none();
            };

            rusty::Option<std::string_view> content_as_str(const serde_core_private::Content& content) {
                return [&]() { auto&& _m = content; return std::visit(overloaded { [&](const serde_core_private::Content_Str& _v) -> rusty::Option<std::string_view> { auto&& x = _v._0; return rusty::Option<std::string_view>(rusty::to_string_view(x)); }, [&](const serde_core_private::Content_String& _v) -> rusty::Option<std::string_view> { const auto& x = _v._0; return rusty::Option<std::string_view>(rusty::to_string_view(x)); }, [&](const serde_core_private::Content_Bytes& _v) -> rusty::Option<std::string_view> { auto&& x = _v._0; return rusty::str_runtime::from_utf8(std::move(x)).ok(); }, [&](const serde_core_private::Content_ByteBuf& _v) -> rusty::Option<std::string_view> { const auto& x = _v._0; return rusty::str_runtime::from_utf8(std::move(x)).ok(); }, [&](const auto&) -> rusty::Option<std::string_view> { return rusty::Option<std::string_view>(rusty::None); } }, _m); }();
            }

            serde_core_private::Content content_clone(const serde_core_private::Content& content) {
                return [&]() { auto&& _m = content; return std::visit(overloaded { [&](const serde_core_private::Content_Bool& _v) -> serde_core_private::Content { auto&& b = _v._0; return serde_core_private::Content::Bool(rusty::detail::deref_if_pointer_like(b)); }, [&](const serde_core_private::Content_U8& _v) -> serde_core_private::Content { auto&& n = _v._0; return serde_core_private::Content::U8(rusty::detail::deref_if_pointer_like(n)); }, [&](const serde_core_private::Content_U16& _v) -> serde_core_private::Content { auto&& n = _v._0; return serde_core_private::Content::U16(rusty::detail::deref_if_pointer_like(n)); }, [&](const serde_core_private::Content_U32& _v) -> serde_core_private::Content { auto&& n = _v._0; return serde_core_private::Content::U32(rusty::detail::deref_if_pointer_like(n)); }, [&](const serde_core_private::Content_U64& _v) -> serde_core_private::Content { auto&& n = _v._0; return serde_core_private::Content::U64(rusty::detail::deref_if_pointer_like(n)); }, [&](const serde_core_private::Content_I8& _v) -> serde_core_private::Content { auto&& n = _v._0; return serde_core_private::Content::I8(rusty::detail::deref_if_pointer_like(n)); }, [&](const serde_core_private::Content_I16& _v) -> serde_core_private::Content { auto&& n = _v._0; return serde_core_private::Content::I16(rusty::detail::deref_if_pointer_like(n)); }, [&](const serde_core_private::Content_I32& _v) -> serde_core_private::Content { auto&& n = _v._0; return serde_core_private::Content::I32(rusty::detail::deref_if_pointer_like(n)); }, [&](const serde_core_private::Content_I64& _v) -> serde_core_private::Content { auto&& n = _v._0; return serde_core_private::Content::I64(rusty::detail::deref_if_pointer_like(n)); }, [&](const serde_core_private::Content_F32& _v) -> serde_core_private::Content { auto&& f = _v._0; return serde_core_private::Content::F32(rusty::detail::deref_if_pointer_like(f)); }, [&](const serde_core_private::Content_F64& _v) -> serde_core_private::Content { auto&& f = _v._0; return serde_core_private::Content::F64(rusty::detail::deref_if_pointer_like(f)); }, [&](const serde_core_private::Content_Char& _v) -> serde_core_private::Content { auto&& c = _v._0; return serde_core_private::Content::Char(rusty::detail::deref_if_pointer_like(c)); }, [&](const serde_core_private::Content_String& _v) -> serde_core_private::Content { auto&& s = _v._0; return serde_core_private::Content::String(rusty::clone(s)); }, [&](const serde_core_private::Content_Str& _v) -> serde_core_private::Content { auto&& s = _v._0; return serde_core_private::Content::Str(rusty::detail::deref_if_pointer_like(s)); }, [&](const serde_core_private::Content_ByteBuf& _v) -> serde_core_private::Content { auto&& b = _v._0; return serde_core_private::Content::ByteBuf(rusty::clone(b)); }, [&](const serde_core_private::Content_Bytes& _v) -> serde_core_private::Content { auto&& b = _v._0; return serde_core_private::Content::Bytes(std::move(b)); }, [&](const serde_core_private::Content_None&) -> serde_core_private::Content { return serde_core_private::Content::None(); }, [&](const serde_core_private::Content_Some& _v) -> serde_core_private::Content { auto&& content = _v._0; return serde_core_private::Content::Some(rusty::Box<serde_core_private::Content>::new_(content_clone(rusty::detail::deref_if_pointer_like(content)))); }, [&](const serde_core_private::Content_Unit&) -> serde_core_private::Content { return serde_core_private::Content::Unit(); }, [&](const serde_core_private::Content_Newtype& _v) -> serde_core_private::Content { auto&& content = _v._0; return serde_core_private::Content::Newtype(rusty::Box<serde_core_private::Content>::new_(content_clone(rusty::detail::deref_if_pointer_like(content)))); }, [&](const serde_core_private::Content_Seq& _v) -> serde_core_private::Content { auto&& seq = _v._0; return serde_core_private::Content::Seq(rusty::collect_range(rusty::map(rusty::iter(seq), [&](auto&&... _args) -> decltype(auto) { return content_clone(std::forward<decltype(_args)>(_args)...); }))); }, [&](const serde_core_private::Content_Map& _v) -> serde_core_private::Content { auto&& map = _v._0; return serde_core_private::Content::Map(rusty::collect_range(rusty::map(rusty::iter(map), [&](auto&& _destruct_param0) {
auto&& k = std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0));
auto&& v = std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0));
return std::make_tuple(content_clone(rusty::detail::deref_if_pointer_like(k)), content_clone(rusty::detail::deref_if_pointer_like(v)));
}))); } }, _m); }();
            }

            ::de::Unexpected content_unexpected(const serde_core_private::Content& content) {
                return [&]() { auto&& _m = content; return std::visit(overloaded { [&](const serde_core_private::Content_Bool& _v) -> ::de::Unexpected { auto&& b = _v._0; return ::de::Unexpected::Bool(std::move(b)); }, [&](const serde_core_private::Content_U8& _v) -> ::de::Unexpected { auto&& n = _v._0; return ::de::Unexpected::Unsigned(static_cast<uint64_t>(n)); }, [&](const serde_core_private::Content_U16& _v) -> ::de::Unexpected { auto&& n = _v._0; return ::de::Unexpected::Unsigned(static_cast<uint64_t>(n)); }, [&](const serde_core_private::Content_U32& _v) -> ::de::Unexpected { auto&& n = _v._0; return ::de::Unexpected::Unsigned(static_cast<uint64_t>(n)); }, [&](const serde_core_private::Content_U64& _v) -> ::de::Unexpected { auto&& n = _v._0; return ::de::Unexpected::Unsigned(std::move(n)); }, [&](const serde_core_private::Content_I8& _v) -> ::de::Unexpected { auto&& n = _v._0; return ::de::Unexpected::Signed(static_cast<int64_t>(n)); }, [&](const serde_core_private::Content_I16& _v) -> ::de::Unexpected { auto&& n = _v._0; return ::de::Unexpected::Signed(static_cast<int64_t>(n)); }, [&](const serde_core_private::Content_I32& _v) -> ::de::Unexpected { auto&& n = _v._0; return ::de::Unexpected::Signed(static_cast<int64_t>(n)); }, [&](const serde_core_private::Content_I64& _v) -> ::de::Unexpected { auto&& n = _v._0; return ::de::Unexpected::Signed(std::move(n)); }, [&](const serde_core_private::Content_F32& _v) -> ::de::Unexpected { auto&& f = _v._0; return ::de::Unexpected::Float(static_cast<double>(f)); }, [&](const serde_core_private::Content_F64& _v) -> ::de::Unexpected { auto&& f = _v._0; return ::de::Unexpected::Float(std::move(f)); }, [&](const serde_core_private::Content_Char& _v) -> ::de::Unexpected { auto&& c = _v._0; return ::de::Unexpected::Char(std::move(c)); }, [&](const serde_core_private::Content_String& _v) -> ::de::Unexpected { const auto& s = _v._0; return ::de::Unexpected::Str(std::move(s)); }, [&](const serde_core_private::Content_Str& _v) -> ::de::Unexpected { auto&& s = _v._0; return ::de::Unexpected::Str(std::move(s)); }, [&](const serde_core_private::Content_ByteBuf& _v) -> ::de::Unexpected { const auto& b = _v._0; return ::de::Unexpected::Bytes(std::move(b)); }, [&](const serde_core_private::Content_Bytes& _v) -> ::de::Unexpected { auto&& b = _v._0; return ::de::Unexpected::Bytes(std::move(b)); }, [&](const auto&) -> ::de::Unexpected { return [&]() -> ::de::Unexpected { rusty::intrinsics::unreachable(); }(); }, [&](const serde_core_private::Content_Unit&) -> ::de::Unexpected { return ::de::Unexpected::Unit(); }, [&](const serde_core_private::Content_Newtype& _v) -> ::de::Unexpected {  return ::de::Unexpected::NewtypeStruct(); }, [&](const serde_core_private::Content_Seq& _v) -> ::de::Unexpected {  return ::de::Unexpected::Seq(); }, [&](const serde_core_private::Content_Map& _v) -> ::de::Unexpected {  return ::de::Unexpected::Map(); } }, _m); }();
            }

            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_seq(rusty::Vec<serde_core_private::Content> content, V visitor) {
                auto seq_visitor = SeqDeserializer<E>::new_(std::move(content));
                auto value = ({ auto&& _m = visitor.visit_seq(seq_visitor); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename V::Value, E>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = seq_visitor.end();
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename V::Value, E>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return rusty::Result<typename V::Value, E>::Ok(std::move(value));
            }

            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_map(rusty::Vec<std::tuple<serde_core_private::Content, serde_core_private::Content>> content, V visitor) {
                auto map_visitor = MapDeserializer<E>::new_(std::move(content));
                auto value = ({ auto&& _m = visitor.visit_map(map_visitor); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename V::Value, E>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = map_visitor.end();
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename V::Value, E>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return rusty::Result<typename V::Value, E>::Ok(std::move(value));
            }

            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_seq_ref(std::span<const serde_core_private::Content> content, V visitor) {
                auto seq_visitor = SeqRefDeserializer<E>::new_(content);
                auto value = ({ auto&& _m = visitor.visit_seq(seq_visitor); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename V::Value, E>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = seq_visitor.end();
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename V::Value, E>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return rusty::Result<typename V::Value, E>::Ok(std::move(value));
            }

            template<typename V, typename E>
            rusty::Result<typename V::Value, E> visit_content_map_ref(std::span<const std::tuple<serde_core_private::Content, serde_core_private::Content>> content, V visitor) {
                auto map_visitor = MapRefDeserializer<E>::new_(content);
                auto value = ({ auto&& _m = visitor.visit_map(map_visitor); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename V::Value, E>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = map_visitor.end();
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename V::Value, E>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return rusty::Result<typename V::Value, E>::Ok(std::move(value));
            }

        }


        template<typename T>
        struct Borrowed {
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Deserializer
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Deserializer
            const T& _0;

            // Rust-only associated type alias with unbound generic skipped in constrained mode: Deserializer
            template<typename E>
            BorrowedStrDeserializer<E> from() {
                return BorrowedStrDeserializer<E>(std::move(std::string_view(this->_0)), rusty::PhantomData<E>{});
            }
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Deserializer
        };

        template<typename E>
        struct StrDeserializer {
            using Error = E;
            std::string_view value;
            rusty::PhantomData<E> marker;

            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_str(std::move(std::string_view(this->value)));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename StrDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
        };

        template<typename E>
        struct BorrowedStrDeserializer {
            using Error = E;
            std::string_view value;
            rusty::PhantomData<E> marker;

            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return visitor.visit_borrowed_str(std::move(std::string_view(this->value)));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_bool(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_i8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_i16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_i32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_i64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_i128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_u8(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_u16(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_u32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_u64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_u128(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_f32(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_f64(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_char(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_str(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_string(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_bytes(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_byte_buf(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_option(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_unit(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_unit_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_newtype_struct(std::string_view name, V visitor) {
                static_cast<void>(name);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_seq(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_tuple(size_t len, V visitor) {
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_tuple_struct(std::string_view name, size_t len, V visitor) {
                static_cast<void>(name);
                static_cast<void>(std::move(len));
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_map(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_struct(std::string_view name, std::span<const std::string_view> fields, V visitor) {
                static_cast<void>(name);
                static_cast<void>(fields);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                static_cast<void>(name);
                static_cast<void>(variants);
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_identifier(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, typename BorrowedStrDeserializer<E>::Error> deserialize_ignored_any(V visitor) {
                return this->deserialize_any(std::move(visitor));
            }
        };

        template<typename E>
        struct FlatMapDeserializer {
            using Error = E;
            rusty::Vec<rusty::Option<std::tuple<::private_::de::content::Content, ::private_::de::content::Content>>>& _0;
            rusty::PhantomData<E> _1;

            template<typename V>
            static rusty::Result<V, E> deserialize_other() {
                return rusty::Result<V, E>::Err(E::custom("can only flatten structs and maps"));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_any(V visitor) {
                return this->deserialize_map(std::move(visitor));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_enum(std::string_view name, std::span<const std::string_view> variants, V visitor) {
                for (auto&& entry : rusty::for_in(this->_0)) {
                    if (auto&& _iflet_scrutinee = flat_map_take_entry(rusty::detail::deref_if_pointer_like(entry), variants); _iflet_scrutinee.is_some()) {
                        auto&& _iflet_payload = _iflet_scrutinee.unwrap();
                        auto&& key = std::get<0>(rusty::detail::deref_if_pointer(_iflet_payload));
                        auto&& value = std::get<1>(rusty::detail::deref_if_pointer(_iflet_payload));
                        return visitor.visit_enum(EnumDeserializer<E>::new_(std::move(key), rusty::Option<::private_::de::content::Content>(std::move(value))));
                    }
                }
                return rusty::Result<typename V::Value, Error>::Err(std::conditional_t<true, Error, V>::custom(std::format("no variant of enum {0} found in flattened data", rusty::to_string(name))));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_map(V visitor) {
                return visitor.visit_map(FlatMapAccess<E>(rusty::iter(this->_0), rusty::Option<const ::private_::de::content::Content&>(rusty::None), rusty::PhantomData<E>{}));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_struct(std::string_view _arg1, std::span<const std::string_view> fields, V visitor) {
                return visitor.visit_map(FlatStructAccess<E>(rusty::iter_mut(this->_0), rusty::Option<::private_::de::content::Content>(rusty::None), fields, rusty::PhantomData<E>{}));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_newtype_struct(std::string_view _name, V visitor) {
                return visitor.visit_newtype_struct(std::move((*this)));
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_option(V visitor) {
                return [&]() -> rusty::Result<typename V::Value, Error> { auto&& _m = visitor.__private_visit_untagged_option(std::move((*this))); if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& value = _mv0; return rusty::Result<typename V::Value, Error>::Ok(std::move(value)); } if (_m.is_err()) { return FlatMapDeserializer<E>::deserialize_other(); } return [&]() -> rusty::Result<typename V::Value, Error> { rusty::intrinsics::unreachable(); }(); }();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_unit(V visitor) {
                return visitor.visit_unit();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_unit_struct(std::string_view _name, V visitor) {
                return visitor.visit_unit();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_ignored_any(V visitor) {
                return visitor.visit_unit();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_bool(V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_i8(V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_i16(V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_i32(V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_i64(V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_u8(V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_u16(V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_u32(V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_u64(V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_f32(V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_f64(V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_char(V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_str(V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_string(V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_bytes(V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_byte_buf(V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_seq(V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_tuple(size_t _arg1, V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_tuple_struct(std::string_view _arg1, size_t _arg2, V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
            template<typename V>
            rusty::Result<typename V::Value, Error> deserialize_identifier(V _visitor) {
                return FlatMapDeserializer<E>::deserialize_other();
            }
        };

        template<typename E>
        struct FlatMapAccess {
            using Error = E;
            rusty::slice_iter::Iter<const rusty::Option<std::tuple<::private_::de::content::Content, ::private_::de::content::Content>>> iter;
            rusty::Option<const ::private_::de::content::Content&> pending_content;
            rusty::PhantomData<E> _marker;

            template<typename T>
            rusty::Result<rusty::Option<typename T::Value>, Error> next_key_seed(T seed) {
                for (auto&& item : rusty::for_in(rusty::iter(this->iter))) {
                    if (auto&& _iflet_scrutinee = rusty::detail::deref_if_pointer_like(item); (_iflet_scrutinee).is_some()) {
                        auto&& _iflet_payload = _iflet_scrutinee.unwrap();
                        const auto& key = std::get<0>(rusty::detail::deref_if_pointer(_iflet_payload));
                        const auto& content = std::get<1>(rusty::detail::deref_if_pointer(_iflet_payload));
                        this->pending_content = rusty::Option<const ::private_::de::content::Content&>(content);
                        return ::de::rusty_ext::deserialize(seed, ContentRefDeserializer<E>::new_(rusty::detail::deref_if_pointer_like(key))).map(rusty::Some);
                    }
                }
                return rusty::Result<rusty::Option<typename T::Value>, Error>::Ok(rusty::Option<typename T::Value>(rusty::None));
            }
            template<typename T>
            rusty::Result<typename T::Value, Error> next_value_seed(T seed) {
                return [&]() -> rusty::Result<typename T::Value, Error> { auto&& _m = this->pending_content.take(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& value = _mv0; return ::de::rusty_ext::deserialize(seed, ContentRefDeserializer<E>::new_(rusty::detail::deref_if_pointer_like(value))); } if (_m.is_none()) { return rusty::Result<typename T::Value, Error>::Err(std::conditional_t<true, Error, T>::custom("value is missing")); } return [&]() -> rusty::Result<typename T::Value, Error> { rusty::intrinsics::unreachable(); }(); }();
            }
        };

        template<typename E>
        struct FlatStructAccess {
            using Error = E;
            rusty::slice_iter::Iter<rusty::Option<std::tuple<::private_::de::content::Content, ::private_::de::content::Content>>> iter;
            rusty::Option<::private_::de::content::Content> pending_content;
            std::span<const std::string_view> fields;
            rusty::PhantomData<E> _marker;

            template<typename T>
            rusty::Result<rusty::Option<typename T::Value>, Error> next_key_seed(T seed) {
                for (auto&& entry : rusty::for_in(this->iter.by_ref())) {
                    if (auto&& _iflet_scrutinee = flat_map_take_entry(rusty::detail::deref_if_pointer_like(entry), this->fields); _iflet_scrutinee.is_some()) {
                        auto&& _iflet_payload = _iflet_scrutinee.unwrap();
                        auto&& key = std::get<0>(rusty::detail::deref_if_pointer(_iflet_payload));
                        auto&& content = std::get<1>(rusty::detail::deref_if_pointer(_iflet_payload));
                        this->pending_content = rusty::Option<::private_::de::content::Content>(std::move(content));
                        return ::de::rusty_ext::deserialize(seed, ContentDeserializer<E>::new_(std::move(key))).map(rusty::Some);
                    }
                }
                return rusty::Result<rusty::Option<typename T::Value>, Error>::Ok(rusty::Option<typename T::Value>(rusty::None));
            }
            template<typename T>
            rusty::Result<typename T::Value, Error> next_value_seed(T seed) {
                return [&]() -> rusty::Result<typename T::Value, Error> { auto&& _m = this->pending_content.take(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& value = _mv0; return ::de::rusty_ext::deserialize(seed, ContentDeserializer<E>::new_(std::move(value))); } if (_m.is_none()) { return rusty::Result<typename T::Value, Error>::Err(std::conditional_t<true, Error, T>::custom("value is missing")); } return [&]() -> rusty::Result<typename T::Value, Error> { rusty::intrinsics::unreachable(); }(); }();
            }
        };

        template<typename F>
        struct AdjacentlyTaggedEnumVariantSeed {
            using Value = F;
            std::string_view enum_name;
            std::span<const std::string_view> variants;
            rusty::PhantomData<F> fields_enum;

            template<typename D>
            rusty::Result<Value, typename D::Error> deserialize(D deserializer) {
                return deserializer.deserialize_enum(std::move(this->enum_name), std::move(this->variants), AdjacentlyTaggedEnumVariantVisitor<F>(std::move(std::string_view(this->enum_name)), rusty::PhantomData<F>{}));
            }
        };

        template<typename F>
        struct AdjacentlyTaggedEnumVariantVisitor {
            using Value = F;
            std::string_view enum_name;
            rusty::PhantomData<F> fields_enum;

            rusty::fmt::Result expecting(rusty::fmt::Formatter& formatter) const {
                return rusty::write_fmt(formatter, std::format("variant of enum {0}", rusty::to_string(this->enum_name)));
            }
            template<typename A>
            rusty::Result<Value, typename A::Error> visit_enum(A data) {
                auto [variant, variant_access] = rusty::detail::deref_if_pointer_like(({ auto&& _m = data.variant(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Value, typename A::Error>::Err(std::move(err)); } std::move(_match_value).value(); }));
                {
                    auto&& _m = variant_access.unit_variant();
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<Value, typename A::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return rusty::Result<Value, typename A::Error>::Ok(std::move(variant));
            }
        };

        /// If the missing field is of type `Option<T>` then treat is as `None`,
        /// otherwise it is an error.
        template<typename V, typename E>
        rusty::Result<V, E> missing_field(std::string_view field) {
            struct MissingFieldDeserializer {
                std::string_view _0;
                rusty::PhantomData<E> _1;
            };
            // Rust-only nested impl block skipped in local scope
            auto deserializer = MissingFieldDeserializer(field, rusty::PhantomData<std::tuple<>>{});
            return V::deserialize(std::move(deserializer));
        }

        template<typename D, typename R>
        rusty::Result<R, typename D::Error> borrow_cow_str(D deserializer) {
            struct CowStrVisitor {
            };
            // Rust-only nested impl block skipped in local scope
            return deserializer.deserialize_str(CowStrVisitor{}).map([&](auto&& _v) -> R { return rusty::from_into<R>(std::forward<decltype(_v)>(_v)); });
        }

        template<typename D, typename R>
        rusty::Result<R, typename D::Error> borrow_cow_bytes(D deserializer) {
            struct CowBytesVisitor {
            };
            // Rust-only nested impl block skipped in local scope
            return deserializer.deserialize_bytes(CowBytesVisitor{}).map([&](auto&& _v) -> R { return rusty::from_into<R>(std::forward<decltype(_v)>(_v)); });
        }

        /// Claims one key-value pair from a FlatMapDeserializer's field buffer if the
        /// field name matches any of the recognized ones.
        rusty::Option<std::tuple<::private_::de::content::Content, ::private_::de::content::Content>> flat_map_take_entry(rusty::Option<std::tuple<::private_::de::content::Content, ::private_::de::content::Content>>& entry, std::span<const std::string_view> recognized) {
            const auto is_recognized = [&]() { auto&& _m = entry; if (_m.is_none()) { return false; } if (_m.is_some()) { auto&& _mv1 = std::as_const(_m).unwrap(); auto&& k = std::get<0>(rusty::detail::deref_if_pointer(_mv1)); auto&& _v = std::get<1>(rusty::detail::deref_if_pointer(_mv1)); return content::content_as_str(rusty::detail::deref_if_pointer_like(k)).map_or(false, [&](auto&& name) { return [&]() { auto&& _haystack = recognized; auto&& _needle = &name; for (const auto& _item : _haystack) { if constexpr (requires { _item == _needle; }) { if (_item == _needle) return true; } else if constexpr (requires { _needle == _item; }) { if (_needle == _item) return true; } } return false; }(); }); } rusty::intrinsics::unreachable(); }();
            if (is_recognized) {
                return entry.take();
            } else {
                return rusty::Option<std::tuple<::private_::de::content::Content, ::private_::de::content::Content>>(rusty::None);
            }
        }

        // Extension trait IdentifierDeserializer lowered to rusty_ext:: free functions
        namespace rusty_ext {
            // Rust-only extension method skipped (unresolved signature placeholder): from

        }


    }

    namespace ser {
        namespace content {}

        enum class Unsupported;
        constexpr Unsupported Unsupported_Boolean();
        constexpr Unsupported Unsupported_Integer();
        constexpr Unsupported Unsupported_Float();
        constexpr Unsupported Unsupported_Char();
        constexpr Unsupported Unsupported_String();
        constexpr Unsupported Unsupported_ByteArray();
        constexpr Unsupported Unsupported_Optional();
        constexpr Unsupported Unsupported_Sequence();
        constexpr Unsupported Unsupported_Tuple();
        constexpr Unsupported Unsupported_TupleStruct();
        template<typename S>
        struct TaggedSerializer;
        template<typename M>
        struct FlatMapSerializer;
        template<typename M>
        struct FlatMapSerializeMap;
        template<typename M>
        struct FlatMapSerializeStruct;
        template<typename M>
        struct FlatMapSerializeTupleVariantAsMapValue;
        template<typename M>
        struct FlatMapSerializeStructVariantAsMapValue;
        struct AdjacentlyTaggedEnumVariant;
        template<typename T>
        struct CannotSerializeVariant;
        namespace content {
            struct Content;
            template<typename M>
            struct SerializeTupleVariantAsMapValue;
            template<typename M>
            struct SerializeStructVariantAsMapValue;
            template<typename E>
            struct ContentSerializer;
            template<typename E>
            struct SerializeSeq;
            template<typename E>
            struct SerializeTuple;
            template<typename E>
            struct SerializeTupleStruct;
            template<typename E>
            struct SerializeTupleVariant;
            template<typename E>
            struct SerializeMap;
            template<typename E>
            struct SerializeStruct;
            template<typename E>
            struct SerializeStructVariant;
        }
        template<typename T>
        const T& constrain(const T& t);
        template<typename S, typename T>
        rusty::Result<typename S::Ok, typename S::Error> serialize_tagged_newtype(S serializer, std::string_view type_ident, std::string_view variant_ident, std::string_view tag, std::string_view variant_name, const T& value);

        using namespace lib;

        // Rust-only unresolved import: using ser;
        using ::ser::Impossible;

        enum class Unsupported {
            Boolean,
    Integer,
    Float,
    Char,
    String,
    ByteArray,
    Optional,
    Sequence,
    Tuple,
    TupleStruct
        };
        inline constexpr Unsupported Unsupported_Boolean() { return Unsupported::Boolean; }
        inline constexpr Unsupported Unsupported_Integer() { return Unsupported::Integer; }
        inline constexpr Unsupported Unsupported_Float() { return Unsupported::Float; }
        inline constexpr Unsupported Unsupported_Char() { return Unsupported::Char; }
        inline constexpr Unsupported Unsupported_String() { return Unsupported::String; }
        inline constexpr Unsupported Unsupported_ByteArray() { return Unsupported::ByteArray; }
        inline constexpr Unsupported Unsupported_Optional() { return Unsupported::Optional; }
        inline constexpr Unsupported Unsupported_Sequence() { return Unsupported::Sequence; }
        inline constexpr Unsupported Unsupported_Tuple() { return Unsupported::Tuple; }
        inline constexpr Unsupported Unsupported_TupleStruct() { return Unsupported::TupleStruct; }
        inline rusty::fmt::Result rusty_fmt(const Unsupported& self, rusty::fmt::Formatter& formatter) {
            return ({ auto&& _m = self; std::optional<rusty::fmt::Result> _match_value; bool _m_matched = false; if (!_m_matched && (_m == Unsupported::Boolean)) { _match_value.emplace(std::move(formatter.write_str("a boolean"))); _m_matched = true; } if (!_m_matched && (_m == Unsupported::Integer)) { _match_value.emplace(std::move(formatter.write_str("an integer"))); _m_matched = true; } if (!_m_matched && (_m == Unsupported::Float)) { _match_value.emplace(std::move(formatter.write_str("a float"))); _m_matched = true; } if (!_m_matched && (_m == Unsupported::Char)) { _match_value.emplace(std::move(formatter.write_str("a char"))); _m_matched = true; } if (!_m_matched && (_m == Unsupported::String)) { _match_value.emplace(std::move(formatter.write_str("a string"))); _m_matched = true; } if (!_m_matched && (_m == Unsupported::ByteArray)) { _match_value.emplace(std::move(formatter.write_str("a byte array"))); _m_matched = true; } if (!_m_matched && (_m == Unsupported::Optional)) { _match_value.emplace(std::move(formatter.write_str("an optional"))); _m_matched = true; } if (!_m_matched && (_m == Unsupported::Sequence)) { _match_value.emplace(std::move(formatter.write_str("a sequence"))); _m_matched = true; } if (!_m_matched && (_m == Unsupported::Tuple)) { _match_value.emplace(std::move(formatter.write_str("a tuple"))); _m_matched = true; } if (!_m_matched && (_m == Unsupported::TupleStruct)) { _match_value.emplace(std::move(formatter.write_str("a tuple struct"))); _m_matched = true; } if (!_m_matched) { rusty::intrinsics::unreachable(); } std::move(_match_value).value(); });
        }

        using ::private_::ser::content::Content;
        using ::private_::ser::content::ContentSerializer;
        using ::private_::ser::content::SerializeStructVariantAsMapValue;
        using ::private_::ser::content::SerializeTupleVariantAsMapValue;

        template<typename S>
        struct TaggedSerializer {
            // Rust-only dependent associated type alias skipped in constrained mode: Ok
            // Rust-only dependent associated type alias skipped in constrained mode: Error
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeSeq
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeTuple
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeTupleStruct
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeMap
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeStruct
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeTupleVariant
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeStructVariant
            std::string_view type_ident;
            std::string_view variant_ident;
            std::string_view tag;
            std::string_view variant_name;
            S delegate;

            auto bad_type(const auto& what) {
                return S::Error::custom(std::format("cannot serialize tagged newtype variant {0}::{1} containing {2}", rusty::to_string(this->type_ident), rusty::to_string(this->variant_ident), rusty::to_string(what)));
            }
            // Rust-only dependent associated type alias skipped in constrained mode: Ok
            // Rust-only dependent associated type alias skipped in constrained mode: Error
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeSeq
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeTuple
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeTupleStruct
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeMap
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeStruct
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeTupleVariant
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeStructVariant
            rusty::Result<typename S::Ok, typename S::Error> serialize_bool(bool _arg1) {
                return rusty::Result<typename S::Ok, typename S::Error>::Err(this->bad_type(Unsupported_Boolean()));
            }
            rusty::Result<typename S::Ok, typename S::Error> serialize_i8(int8_t _arg1) {
                return rusty::Result<typename S::Ok, typename S::Error>::Err(this->bad_type(Unsupported_Integer()));
            }
            rusty::Result<typename S::Ok, typename S::Error> serialize_i16(int16_t _arg1) {
                return rusty::Result<typename S::Ok, typename S::Error>::Err(this->bad_type(Unsupported_Integer()));
            }
            rusty::Result<typename S::Ok, typename S::Error> serialize_i32(int32_t _arg1) {
                return rusty::Result<typename S::Ok, typename S::Error>::Err(this->bad_type(Unsupported_Integer()));
            }
            rusty::Result<typename S::Ok, typename S::Error> serialize_i64(int64_t _arg1) {
                return rusty::Result<typename S::Ok, typename S::Error>::Err(this->bad_type(Unsupported_Integer()));
            }
            rusty::Result<typename S::Ok, typename S::Error> serialize_u8(uint8_t _arg1) {
                return rusty::Result<typename S::Ok, typename S::Error>::Err(this->bad_type(Unsupported_Integer()));
            }
            rusty::Result<typename S::Ok, typename S::Error> serialize_u16(uint16_t _arg1) {
                return rusty::Result<typename S::Ok, typename S::Error>::Err(this->bad_type(Unsupported_Integer()));
            }
            rusty::Result<typename S::Ok, typename S::Error> serialize_u32(uint32_t _arg1) {
                return rusty::Result<typename S::Ok, typename S::Error>::Err(this->bad_type(Unsupported_Integer()));
            }
            rusty::Result<typename S::Ok, typename S::Error> serialize_u64(uint64_t _arg1) {
                return rusty::Result<typename S::Ok, typename S::Error>::Err(this->bad_type(Unsupported_Integer()));
            }
            rusty::Result<typename S::Ok, typename S::Error> serialize_f32(float _arg1) {
                return rusty::Result<typename S::Ok, typename S::Error>::Err(this->bad_type(Unsupported_Float()));
            }
            rusty::Result<typename S::Ok, typename S::Error> serialize_f64(double _arg1) {
                return rusty::Result<typename S::Ok, typename S::Error>::Err(this->bad_type(Unsupported_Float()));
            }
            rusty::Result<typename S::Ok, typename S::Error> serialize_char(char32_t _arg1) {
                return rusty::Result<typename S::Ok, typename S::Error>::Err(this->bad_type(Unsupported_Char()));
            }
            rusty::Result<typename S::Ok, typename S::Error> serialize_str(std::string_view _arg1) {
                return rusty::Result<typename S::Ok, typename S::Error>::Err(this->bad_type(Unsupported_String()));
            }
            rusty::Result<typename S::Ok, typename S::Error> serialize_bytes(std::span<const uint8_t> _arg1) {
                return rusty::Result<typename S::Ok, typename S::Error>::Err(this->bad_type(Unsupported_ByteArray()));
            }
            rusty::Result<typename S::Ok, typename S::Error> serialize_none() {
                return rusty::Result<typename S::Ok, typename S::Error>::Err(this->bad_type(Unsupported_Optional()));
            }
            template<typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize_some(const T& _arg1) {
                return rusty::Result<typename S::Ok, typename S::Error>::Err(this->bad_type(Unsupported_Optional()));
            }
            rusty::Result<typename S::Ok, typename S::Error> serialize_unit() {
                auto map = ({ auto&& _m = this->delegate.serialize_map(rusty::Option<size_t>(static_cast<size_t>(1))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = map.serialize_entry(rusty::detail::deref_if_pointer_like(this->tag), rusty::detail::deref_if_pointer_like(this->variant_name));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return map.end();
            }
            rusty::Result<typename S::Ok, typename S::Error> serialize_unit_struct(std::string_view _arg1) {
                auto map = ({ auto&& _m = this->delegate.serialize_map(rusty::Option<size_t>(static_cast<size_t>(1))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = map.serialize_entry(rusty::detail::deref_if_pointer_like(this->tag), rusty::detail::deref_if_pointer_like(this->variant_name));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return map.end();
            }
            rusty::Result<typename S::Ok, typename S::Error> serialize_unit_variant(std::string_view _arg1, uint32_t _arg2, std::string_view inner_variant) {
                auto map = ({ auto&& _m = this->delegate.serialize_map(rusty::Option<size_t>(static_cast<size_t>(2))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = map.serialize_entry(rusty::detail::deref_if_pointer_like(this->tag), rusty::detail::deref_if_pointer_like(this->variant_name));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                {
                    auto&& _m = map.serialize_entry(rusty::detail::deref_if_pointer_like(inner_variant), std::make_tuple());
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return map.end();
            }
            template<typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize_newtype_struct(std::string_view _arg1, const T& value) {
                return ::ser::impls::rusty_ext::serialize(value, std::move((*this)));
            }
            template<typename T>
            rusty::Result<typename S::Ok, typename S::Error> serialize_newtype_variant(std::string_view _arg1, uint32_t _arg2, std::string_view inner_variant, const T& inner_value) {
                auto map = ({ auto&& _m = this->delegate.serialize_map(rusty::Option<size_t>(static_cast<size_t>(2))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = map.serialize_entry(rusty::detail::deref_if_pointer_like(this->tag), rusty::detail::deref_if_pointer_like(this->variant_name));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                {
                    auto&& _m = map.serialize_entry(rusty::detail::deref_if_pointer_like(inner_variant), rusty::detail::deref_if_pointer_like(inner_value));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return map.end();
            }
            rusty::Result<ser::Impossible<typename S::Ok, typename S::Error>, typename S::Error> serialize_seq(rusty::Option<size_t> _arg1) {
                return rusty::Result<ser::Impossible<typename S::Ok, typename S::Error>, typename S::Error>::Err(this->bad_type(Unsupported_Sequence()));
            }
            rusty::Result<ser::Impossible<typename S::Ok, typename S::Error>, typename S::Error> serialize_tuple(size_t _arg1) {
                return rusty::Result<ser::Impossible<typename S::Ok, typename S::Error>, typename S::Error>::Err(this->bad_type(Unsupported_Tuple()));
            }
            rusty::Result<ser::Impossible<typename S::Ok, typename S::Error>, typename S::Error> serialize_tuple_struct(std::string_view _arg1, size_t _arg2) {
                return rusty::Result<ser::Impossible<typename S::Ok, typename S::Error>, typename S::Error>::Err(this->bad_type(Unsupported_TupleStruct()));
            }
            rusty::Result<::private_::ser::content::SerializeTupleVariantAsMapValue<typename S::SerializeMap<S>>, typename S::Error> serialize_tuple_variant(std::string_view _arg1, uint32_t _arg2, std::string_view inner_variant, size_t len) {
                auto map = ({ auto&& _m = this->delegate.serialize_map(rusty::Option<size_t>(static_cast<size_t>(2))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<::private_::ser::content::SerializeTupleVariantAsMapValue<typename S::SerializeMap<S>>, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = map.serialize_entry(rusty::detail::deref_if_pointer_like(this->tag), rusty::detail::deref_if_pointer_like(this->variant_name));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<::private_::ser::content::SerializeTupleVariantAsMapValue<typename S::SerializeMap<S>>, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                {
                    auto&& _m = map.serialize_key(rusty::detail::deref_if_pointer_like(inner_variant));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<::private_::ser::content::SerializeTupleVariantAsMapValue<typename S::SerializeMap<S>>, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return rusty::Result<::private_::ser::content::SerializeTupleVariantAsMapValue<typename S::SerializeMap<S>>, typename S::Error>::Ok(SerializeTupleVariantAsMapValue<S>::new_(std::move(map), std::string_view(inner_variant), std::move(len)));
            }
            rusty::Result<typename S::SerializeMap<S>, typename S::Error> serialize_map(rusty::Option<size_t> len) {
                auto map = ({ auto&& _m = this->delegate.serialize_map(len.map([&](auto&& len) -> size_t { return len + 1; })); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename S::SerializeMap<S>, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = map.serialize_entry(rusty::detail::deref_if_pointer_like(this->tag), rusty::detail::deref_if_pointer_like(this->variant_name));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename S::SerializeMap<S>, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return rusty::Result<typename S::SerializeMap<S>, typename S::Error>::Ok(std::move(map));
            }
            rusty::Result<typename S::SerializeStruct<S>, typename S::Error> serialize_struct(std::string_view name, size_t len) {
                auto state = ({ auto&& _m = this->delegate.serialize_struct(std::string_view(name), len + 1); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename S::SerializeStruct<S>, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = state.serialize_field(std::move(this->tag), rusty::detail::deref_if_pointer_like(this->variant_name));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<typename S::SerializeStruct<S>, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return rusty::Result<typename S::SerializeStruct<S>, typename S::Error>::Ok(std::move(state));
            }
            rusty::Result<::private_::ser::content::SerializeStructVariantAsMapValue<typename S::SerializeMap<S>>, typename S::Error> serialize_struct_variant(std::string_view _arg1, uint32_t _arg2, std::string_view inner_variant, size_t len) {
                auto map = ({ auto&& _m = this->delegate.serialize_map(rusty::Option<size_t>(static_cast<size_t>(2))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<::private_::ser::content::SerializeStructVariantAsMapValue<typename S::SerializeMap<S>>, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                {
                    auto&& _m = map.serialize_entry(rusty::detail::deref_if_pointer_like(this->tag), rusty::detail::deref_if_pointer_like(this->variant_name));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<::private_::ser::content::SerializeStructVariantAsMapValue<typename S::SerializeMap<S>>, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                {
                    auto&& _m = map.serialize_key(rusty::detail::deref_if_pointer_like(inner_variant));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<::private_::ser::content::SerializeStructVariantAsMapValue<typename S::SerializeMap<S>>, typename S::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return rusty::Result<::private_::ser::content::SerializeStructVariantAsMapValue<typename S::SerializeMap<S>>, typename S::Error>::Ok(SerializeStructVariantAsMapValue<S>::new_(std::move(map), std::string_view(inner_variant), std::move(len)));
            }
        };

        namespace content {

            struct Content;
            template<typename M>
            struct SerializeTupleVariantAsMapValue;
            template<typename M>
            struct SerializeStructVariantAsMapValue;
            template<typename E>
            struct ContentSerializer;
            template<typename E>
            struct SerializeSeq;
            template<typename E>
            struct SerializeTuple;
            template<typename E>
            struct SerializeTupleStruct;
            template<typename E>
            struct SerializeTupleVariant;
            template<typename E>
            struct SerializeMap;
            template<typename E>
            struct SerializeStruct;
            template<typename E>
            struct SerializeStructVariant;

            using namespace lib;

            // Rust-only unresolved import: using ser;

            struct Content;  // forward declaration for recursion
            // Algebraic data type
            struct Content_Bool {
                bool _0;
            };
            struct Content_U8 {
                uint8_t _0;
            };
            struct Content_U16 {
                uint16_t _0;
            };
            struct Content_U32 {
                uint32_t _0;
            };
            struct Content_U64 {
                uint64_t _0;
            };
            struct Content_I8 {
                int8_t _0;
            };
            struct Content_I16 {
                int16_t _0;
            };
            struct Content_I32 {
                int32_t _0;
            };
            struct Content_I64 {
                int64_t _0;
            };
            struct Content_F32 {
                float _0;
            };
            struct Content_F64 {
                double _0;
            };
            struct Content_Char {
                char32_t _0;
            };
            struct Content_String {
                rusty::String _0;
            };
            struct Content_Bytes {
                rusty::Vec<uint8_t> _0;
            };
            struct Content_None {};
            struct Content_Some {
                rusty::Box<Content> _0;
            };
            struct Content_Unit {};
            struct Content_UnitStruct {
                std::string_view _0;
            };
            struct Content_UnitVariant {
                std::string_view _0;
                uint32_t _1;
                std::string_view _2;
            };
            struct Content_NewtypeStruct {
                std::string_view _0;
                rusty::Box<Content> _1;
            };
            struct Content_NewtypeVariant {
                std::string_view _0;
                uint32_t _1;
                std::string_view _2;
                rusty::Box<Content> _3;
            };
            struct Content_Seq {
                rusty::Vec<Content> _0;
            };
            struct Content_Tuple {
                rusty::Vec<Content> _0;
            };
            struct Content_TupleStruct {
                std::string_view _0;
                rusty::Vec<Content> _1;
            };
            struct Content_TupleVariant {
                std::string_view _0;
                uint32_t _1;
                std::string_view _2;
                rusty::Vec<Content> _3;
            };
            struct Content_Map {
                rusty::Vec<std::tuple<Content, Content>> _0;
            };
            struct Content_Struct {
                std::string_view _0;
                rusty::Vec<std::tuple<std::string_view, Content>> _1;
            };
            struct Content_StructVariant {
                std::string_view _0;
                uint32_t _1;
                std::string_view _2;
                rusty::Vec<std::tuple<std::string_view, Content>> _3;
            };
            Content_Bool Bool(bool _0);
            Content_U8 U8(uint8_t _0);
            Content_U16 U16(uint16_t _0);
            Content_U32 U32(uint32_t _0);
            Content_U64 U64(uint64_t _0);
            Content_I8 I8(int8_t _0);
            Content_I16 I16(int16_t _0);
            Content_I32 I32(int32_t _0);
            Content_I64 I64(int64_t _0);
            Content_F32 F32(float _0);
            Content_F64 F64(double _0);
            Content_Char Char(char32_t _0);
            Content_String String(rusty::String _0);
            Content_Bytes Bytes(rusty::Vec<uint8_t> _0);
            Content_None None();
            Content_Some Some(rusty::Box<Content> _0);
            Content_Unit Unit();
            Content_UnitStruct UnitStruct(std::string_view _0);
            Content_UnitVariant UnitVariant(std::string_view _0, uint32_t _1, std::string_view _2);
            Content_NewtypeStruct NewtypeStruct(std::string_view _0, rusty::Box<Content> _1);
            Content_NewtypeVariant NewtypeVariant(std::string_view _0, uint32_t _1, std::string_view _2, rusty::Box<Content> _3);
            Content_Seq Seq(rusty::Vec<Content> _0);
            Content_Tuple Tuple(rusty::Vec<Content> _0);
            Content_TupleStruct TupleStruct(std::string_view _0, rusty::Vec<Content> _1);
            Content_TupleVariant TupleVariant(std::string_view _0, uint32_t _1, std::string_view _2, rusty::Vec<Content> _3);
            Content_Map Map(rusty::Vec<std::tuple<Content, Content>> _0);
            Content_Struct Struct(std::string_view _0, rusty::Vec<std::tuple<std::string_view, Content>> _1);
            Content_StructVariant StructVariant(std::string_view _0, uint32_t _1, std::string_view _2, rusty::Vec<std::tuple<std::string_view, Content>> _3);
            struct Content : std::variant<Content_Bool, Content_U8, Content_U16, Content_U32, Content_U64, Content_I8, Content_I16, Content_I32, Content_I64, Content_F32, Content_F64, Content_Char, Content_String, Content_Bytes, Content_None, Content_Some, Content_Unit, Content_UnitStruct, Content_UnitVariant, Content_NewtypeStruct, Content_NewtypeVariant, Content_Seq, Content_Tuple, Content_TupleStruct, Content_TupleVariant, Content_Map, Content_Struct, Content_StructVariant> {
                using variant = std::variant<Content_Bool, Content_U8, Content_U16, Content_U32, Content_U64, Content_I8, Content_I16, Content_I32, Content_I64, Content_F32, Content_F64, Content_Char, Content_String, Content_Bytes, Content_None, Content_Some, Content_Unit, Content_UnitStruct, Content_UnitVariant, Content_NewtypeStruct, Content_NewtypeVariant, Content_Seq, Content_Tuple, Content_TupleStruct, Content_TupleVariant, Content_Map, Content_Struct, Content_StructVariant>;
                using variant::variant;
                static Content Bool(bool _0) { return Content{Content_Bool{std::forward<decltype(_0)>(_0)}}; }
                static Content U8(uint8_t _0) { return Content{Content_U8{std::forward<decltype(_0)>(_0)}}; }
                static Content U16(uint16_t _0) { return Content{Content_U16{std::forward<decltype(_0)>(_0)}}; }
                static Content U32(uint32_t _0) { return Content{Content_U32{std::forward<decltype(_0)>(_0)}}; }
                static Content U64(uint64_t _0) { return Content{Content_U64{std::forward<decltype(_0)>(_0)}}; }
                static Content I8(int8_t _0) { return Content{Content_I8{std::forward<decltype(_0)>(_0)}}; }
                static Content I16(int16_t _0) { return Content{Content_I16{std::forward<decltype(_0)>(_0)}}; }
                static Content I32(int32_t _0) { return Content{Content_I32{std::forward<decltype(_0)>(_0)}}; }
                static Content I64(int64_t _0) { return Content{Content_I64{std::forward<decltype(_0)>(_0)}}; }
                static Content F32(float _0) { return Content{Content_F32{std::forward<decltype(_0)>(_0)}}; }
                static Content F64(double _0) { return Content{Content_F64{std::forward<decltype(_0)>(_0)}}; }
                static Content Char(char32_t _0) { return Content{Content_Char{std::forward<decltype(_0)>(_0)}}; }
                static Content String(rusty::String _0) { return Content{Content_String{std::forward<decltype(_0)>(_0)}}; }
                static Content Bytes(rusty::Vec<uint8_t> _0) { return Content{Content_Bytes{std::forward<decltype(_0)>(_0)}}; }
                static Content None() { return Content{Content_None{}}; }
                static Content Some(rusty::Box<Content> _0) { return Content{Content_Some{std::forward<decltype(_0)>(_0)}}; }
                static Content Unit() { return Content{Content_Unit{}}; }
                static Content UnitStruct(std::string_view _0) { return Content{Content_UnitStruct{std::forward<decltype(_0)>(_0)}}; }
                static Content UnitVariant(std::string_view _0, uint32_t _1, std::string_view _2) { return Content{Content_UnitVariant{std::forward<decltype(_0)>(_0), std::forward<decltype(_1)>(_1), std::forward<decltype(_2)>(_2)}}; }
                static Content NewtypeStruct(std::string_view _0, rusty::Box<Content> _1) { return Content{Content_NewtypeStruct{std::forward<decltype(_0)>(_0), std::forward<decltype(_1)>(_1)}}; }
                static Content NewtypeVariant(std::string_view _0, uint32_t _1, std::string_view _2, rusty::Box<Content> _3) { return Content{Content_NewtypeVariant{std::forward<decltype(_0)>(_0), std::forward<decltype(_1)>(_1), std::forward<decltype(_2)>(_2), std::forward<decltype(_3)>(_3)}}; }
                static Content Seq(rusty::Vec<Content> _0) { return Content{Content_Seq{std::forward<decltype(_0)>(_0)}}; }
                static Content Tuple(rusty::Vec<Content> _0) { return Content{Content_Tuple{std::forward<decltype(_0)>(_0)}}; }
                static Content TupleStruct(std::string_view _0, rusty::Vec<Content> _1) { return Content{Content_TupleStruct{std::forward<decltype(_0)>(_0), std::forward<decltype(_1)>(_1)}}; }
                static Content TupleVariant(std::string_view _0, uint32_t _1, std::string_view _2, rusty::Vec<Content> _3) { return Content{Content_TupleVariant{std::forward<decltype(_0)>(_0), std::forward<decltype(_1)>(_1), std::forward<decltype(_2)>(_2), std::forward<decltype(_3)>(_3)}}; }
                static Content Map(rusty::Vec<std::tuple<Content, Content>> _0) { return Content{Content_Map{std::forward<decltype(_0)>(_0)}}; }
                static Content Struct(std::string_view _0, rusty::Vec<std::tuple<std::string_view, Content>> _1) { return Content{Content_Struct{std::forward<decltype(_0)>(_0), std::forward<decltype(_1)>(_1)}}; }
                static Content StructVariant(std::string_view _0, uint32_t _1, std::string_view _2, rusty::Vec<std::tuple<std::string_view, Content>> _3) { return Content{Content_StructVariant{std::forward<decltype(_0)>(_0), std::forward<decltype(_1)>(_1), std::forward<decltype(_2)>(_2), std::forward<decltype(_3)>(_3)}}; }


                template<typename S>
                auto serialize(S serializer) const;
            };
            Content_Bool Bool(bool _0) { return Content_Bool{std::forward<bool>(_0)};  }
            Content_U8 U8(uint8_t _0) { return Content_U8{std::forward<uint8_t>(_0)};  }
            Content_U16 U16(uint16_t _0) { return Content_U16{std::forward<uint16_t>(_0)};  }
            Content_U32 U32(uint32_t _0) { return Content_U32{std::forward<uint32_t>(_0)};  }
            Content_U64 U64(uint64_t _0) { return Content_U64{std::forward<uint64_t>(_0)};  }
            Content_I8 I8(int8_t _0) { return Content_I8{std::forward<int8_t>(_0)};  }
            Content_I16 I16(int16_t _0) { return Content_I16{std::forward<int16_t>(_0)};  }
            Content_I32 I32(int32_t _0) { return Content_I32{std::forward<int32_t>(_0)};  }
            Content_I64 I64(int64_t _0) { return Content_I64{std::forward<int64_t>(_0)};  }
            Content_F32 F32(float _0) { return Content_F32{std::forward<float>(_0)};  }
            Content_F64 F64(double _0) { return Content_F64{std::forward<double>(_0)};  }
            Content_Char Char(char32_t _0) { return Content_Char{std::forward<char32_t>(_0)};  }
            Content_String String(rusty::String _0) { return Content_String{std::forward<rusty::String>(_0)};  }
            Content_Bytes Bytes(rusty::Vec<uint8_t> _0) { return Content_Bytes{std::forward<rusty::Vec<uint8_t>>(_0)};  }
            Content_None None() { return Content_None{};  }
            Content_Some Some(rusty::Box<Content> _0) { return Content_Some{std::forward<rusty::Box<Content>>(_0)};  }
            Content_Unit Unit() { return Content_Unit{};  }
            Content_UnitStruct UnitStruct(std::string_view _0) { return Content_UnitStruct{std::forward<std::string_view>(_0)};  }
            Content_UnitVariant UnitVariant(std::string_view _0, uint32_t _1, std::string_view _2) { return Content_UnitVariant{std::forward<std::string_view>(_0), std::forward<uint32_t>(_1), std::forward<std::string_view>(_2)};  }
            Content_NewtypeStruct NewtypeStruct(std::string_view _0, rusty::Box<Content> _1) { return Content_NewtypeStruct{std::forward<std::string_view>(_0), std::forward<rusty::Box<Content>>(_1)};  }
            Content_NewtypeVariant NewtypeVariant(std::string_view _0, uint32_t _1, std::string_view _2, rusty::Box<Content> _3) { return Content_NewtypeVariant{std::forward<std::string_view>(_0), std::forward<uint32_t>(_1), std::forward<std::string_view>(_2), std::forward<rusty::Box<Content>>(_3)};  }
            Content_Seq Seq(rusty::Vec<Content> _0) { return Content_Seq{std::forward<rusty::Vec<Content>>(_0)};  }
            Content_Tuple Tuple(rusty::Vec<Content> _0) { return Content_Tuple{std::forward<rusty::Vec<Content>>(_0)};  }
            Content_TupleStruct TupleStruct(std::string_view _0, rusty::Vec<Content> _1) { return Content_TupleStruct{std::forward<std::string_view>(_0), std::forward<rusty::Vec<Content>>(_1)};  }
            Content_TupleVariant TupleVariant(std::string_view _0, uint32_t _1, std::string_view _2, rusty::Vec<Content> _3) { return Content_TupleVariant{std::forward<std::string_view>(_0), std::forward<uint32_t>(_1), std::forward<std::string_view>(_2), std::forward<rusty::Vec<Content>>(_3)};  }
            Content_Map Map(rusty::Vec<std::tuple<Content, Content>> _0) { return Content_Map{std::forward<rusty::Vec<std::tuple<Content, Content>>>(_0)};  }
            Content_Struct Struct(std::string_view _0, rusty::Vec<std::tuple<std::string_view, Content>> _1) { return Content_Struct{std::forward<std::string_view>(_0), std::forward<rusty::Vec<std::tuple<std::string_view, Content>>>(_1)};  }
            Content_StructVariant StructVariant(std::string_view _0, uint32_t _1, std::string_view _2, rusty::Vec<std::tuple<std::string_view, Content>> _3) { return Content_StructVariant{std::forward<std::string_view>(_0), std::forward<uint32_t>(_1), std::forward<std::string_view>(_2), std::forward<rusty::Vec<std::tuple<std::string_view, Content>>>(_3)};  }

            template<typename M>
            struct SerializeTupleVariantAsMapValue {
                // Rust-only dependent associated type alias skipped in constrained mode: Ok
                // Rust-only dependent associated type alias skipped in constrained mode: Error
                M map;
                std::string_view name;
                rusty::Vec<Content> fields;

                static SerializeTupleVariantAsMapValue<M> new_(M map, std::string_view name, size_t len) {
                    return SerializeTupleVariantAsMapValue<M>{.map = std::move(map), .name = std::string_view(name), .fields = rusty::Vec<Content>::with_capacity(std::move(len))};
                }
                // Rust-only dependent associated type alias skipped in constrained mode: Ok
                // Rust-only dependent associated type alias skipped in constrained mode: Error
                template<typename T>
                auto serialize_field(const T& value) {
                    auto value_shadow1 = ({ auto&& _m = ::ser::impls::rusty_ext::serialize(value, ContentSerializer<typename M::Error>::new_()); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<std::tuple<>, typename M::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                    this->fields.push(std::move(value_shadow1));
                    return rusty::Result<std::tuple<>, typename M::Error>::Ok(std::make_tuple());
                }
                auto end() {
                    {
                        auto&& _m = this->map.serialize_value(Content_TupleStruct{std::move(this->name), std::move(this->fields)});
                        bool _m_matched = false;
                        if (!_m_matched) {
                            if (_m.is_ok()) {
                                auto&& _mv0 = _m.unwrap();
                                auto&& val = _mv0;
                                val;
                                _m_matched = true;
                            }
                        }
                        if (!_m_matched) {
                            if (_m.is_err()) {
                                auto&& _mv1 = _m.unwrap_err();
                                auto&& err = _mv1;
                                return rusty::Result<typename M::Ok, typename M::Error>::Err(std::move(err));
                                _m_matched = true;
                            }
                        }
                    }
                    return this->map.end();
                }
            };

            template<typename M>
            struct SerializeStructVariantAsMapValue {
                // Rust-only dependent associated type alias skipped in constrained mode: Ok
                // Rust-only dependent associated type alias skipped in constrained mode: Error
                M map;
                std::string_view name;
                rusty::Vec<std::tuple<std::string_view, Content>> fields;

                static SerializeStructVariantAsMapValue<M> new_(M map, std::string_view name, size_t len) {
                    return SerializeStructVariantAsMapValue<M>{.map = std::move(map), .name = std::string_view(name), .fields = rusty::Vec<std::tuple<std::string_view, Content>>::with_capacity(std::move(len))};
                }
                // Rust-only dependent associated type alias skipped in constrained mode: Ok
                // Rust-only dependent associated type alias skipped in constrained mode: Error
                template<typename T>
                auto serialize_field(std::string_view key, const T& value) {
                    auto value_shadow1 = ({ auto&& _m = ::ser::impls::rusty_ext::serialize(value, ContentSerializer<typename M::Error>::new_()); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<std::tuple<>, typename M::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                    this->fields.push(std::tuple<std::string_view, Content>{std::string_view(key), std::move(value_shadow1)});
                    return rusty::Result<std::tuple<>, typename M::Error>::Ok(std::make_tuple());
                }
                auto end() {
                    {
                        auto&& _m = this->map.serialize_value(Content_Struct{std::move(this->name), std::move(this->fields)});
                        bool _m_matched = false;
                        if (!_m_matched) {
                            if (_m.is_ok()) {
                                auto&& _mv0 = _m.unwrap();
                                auto&& val = _mv0;
                                val;
                                _m_matched = true;
                            }
                        }
                        if (!_m_matched) {
                            if (_m.is_err()) {
                                auto&& _mv1 = _m.unwrap_err();
                                auto&& err = _mv1;
                                return rusty::Result<typename M::Ok, typename M::Error>::Err(std::move(err));
                                _m_matched = true;
                            }
                        }
                    }
                    return this->map.end();
                }
            };

            template<typename E>
            struct ContentSerializer {
                using Ok = std::conditional_t<true, Content, E>;
                using Error = E;
                using SerializeSeq = ::private_::ser::content::SerializeSeq<E>;
                using SerializeTuple = ::private_::ser::content::SerializeTuple<E>;
                using SerializeTupleStruct = ::private_::ser::content::SerializeTupleStruct<E>;
                using SerializeTupleVariant = ::private_::ser::content::SerializeTupleVariant<E>;
                using SerializeMap = ::private_::ser::content::SerializeMap<E>;
                using SerializeStruct = ::private_::ser::content::SerializeStruct<E>;
                using SerializeStructVariant = ::private_::ser::content::SerializeStructVariant<E>;
                rusty::PhantomData<E> error;

                static ContentSerializer<E> new_() {
                    return ContentSerializer<E>{.error = rusty::PhantomData<E>{}};
                }
                rusty::Result<Content, E> serialize_bool(bool v) {
                    return rusty::Result<Content, E>::Ok(Content{Content_Bool{std::move(v)}});
                }
                rusty::Result<Content, E> serialize_i8(int8_t v) {
                    return rusty::Result<Content, E>::Ok(Content{Content_I8{std::move(v)}});
                }
                rusty::Result<Content, E> serialize_i16(int16_t v) {
                    return rusty::Result<Content, E>::Ok(Content{Content_I16{std::move(v)}});
                }
                rusty::Result<Content, E> serialize_i32(int32_t v) {
                    return rusty::Result<Content, E>::Ok(Content{Content_I32{std::move(v)}});
                }
                rusty::Result<Content, E> serialize_i64(int64_t v) {
                    return rusty::Result<Content, E>::Ok(Content{Content_I64{std::move(v)}});
                }
                rusty::Result<Content, E> serialize_u8(uint8_t v) {
                    return rusty::Result<Content, E>::Ok(Content{Content_U8{std::move(v)}});
                }
                rusty::Result<Content, E> serialize_u16(uint16_t v) {
                    return rusty::Result<Content, E>::Ok(Content{Content_U16{std::move(v)}});
                }
                rusty::Result<Content, E> serialize_u32(uint32_t v) {
                    return rusty::Result<Content, E>::Ok(Content{Content_U32{std::move(v)}});
                }
                rusty::Result<Content, E> serialize_u64(uint64_t v) {
                    return rusty::Result<Content, E>::Ok(Content{Content_U64{std::move(v)}});
                }
                rusty::Result<Content, E> serialize_f32(float v) {
                    return rusty::Result<Content, E>::Ok(Content{Content_F32{std::move(v)}});
                }
                rusty::Result<Content, E> serialize_f64(double v) {
                    return rusty::Result<Content, E>::Ok(Content{Content_F64{std::move(v)}});
                }
                rusty::Result<Content, E> serialize_char(char32_t v) {
                    return rusty::Result<Content, E>::Ok(Content{Content_Char{std::move(v)}});
                }
                rusty::Result<Content, E> serialize_str(std::string_view value) {
                    return rusty::Result<Content, E>::Ok(Content{Content_String{rusty::String::from(value)}});
                }
                rusty::Result<Content, E> serialize_bytes(std::span<const uint8_t> value) {
                    return rusty::Result<Content, E>::Ok(Content{Content_Bytes{rusty::to_owned(value)}});
                }
                rusty::Result<Content, E> serialize_none() {
                    return rusty::Result<Content, E>::Ok(Content{Content_None{}});
                }
                template<typename T>
                rusty::Result<Content, E> serialize_some(const T& value) {
                    return rusty::Result<Content, E>::Ok(Content{Content_Some{rusty::make_box(({ auto&& _m = ::ser::impls::rusty_ext::serialize(value, std::move((*this))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Content, E>::Err(std::move(err)); } std::move(_match_value).value(); }))}});
                }
                rusty::Result<Content, E> serialize_unit() {
                    return rusty::Result<Content, E>::Ok(Content{Content_Unit{}});
                }
                rusty::Result<Content, E> serialize_unit_struct(std::string_view name) {
                    return rusty::Result<Content, E>::Ok(Content{Content_UnitStruct{name}});
                }
                rusty::Result<Content, E> serialize_unit_variant(std::string_view name, uint32_t variant_index, std::string_view variant) {
                    return rusty::Result<Content, E>::Ok(Content{Content_UnitVariant{name, std::move(variant_index), variant}});
                }
                template<typename T>
                rusty::Result<Content, E> serialize_newtype_struct(std::string_view name, const T& value) {
                    return rusty::Result<Content, E>::Ok(Content{Content_NewtypeStruct{name, rusty::make_box(({ auto&& _m = ::ser::impls::rusty_ext::serialize(value, std::move((*this))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Content, E>::Err(std::move(err)); } std::move(_match_value).value(); }))}});
                }
                template<typename T>
                rusty::Result<Content, E> serialize_newtype_variant(std::string_view name, uint32_t variant_index, std::string_view variant, const T& value) {
                    return rusty::Result<Content, E>::Ok(Content{Content_NewtypeVariant{name, std::move(variant_index), variant, rusty::make_box(({ auto&& _m = ::ser::impls::rusty_ext::serialize(value, std::move((*this))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<Content, E>::Err(std::move(err)); } std::move(_match_value).value(); }))}});
                }
                rusty::Result<SerializeSeq, E> serialize_seq(rusty::Option<size_t> len) {
                    return rusty::Result<SerializeSeq, E>::Ok(SerializeSeq(rusty::Vec<Content>::with_capacity(len.unwrap_or(0)), rusty::PhantomData<E>{}));
                }
                rusty::Result<SerializeTuple, E> serialize_tuple(size_t len) {
                    return rusty::Result<SerializeTuple, E>::Ok(SerializeTuple(rusty::Vec<Content>::with_capacity(std::move(len)), rusty::PhantomData<E>{}));
                }
                rusty::Result<SerializeTupleStruct, E> serialize_tuple_struct(std::string_view name, size_t len) {
                    return rusty::Result<SerializeTupleStruct, E>::Ok(SerializeTupleStruct(std::string_view(name), rusty::Vec<Content>::with_capacity(std::move(len)), rusty::PhantomData<E>{}));
                }
                rusty::Result<SerializeTupleVariant, E> serialize_tuple_variant(std::string_view name, uint32_t variant_index, std::string_view variant, size_t len) {
                    return rusty::Result<SerializeTupleVariant, E>::Ok(SerializeTupleVariant(std::string_view(name), std::move(variant_index), std::string_view(variant), rusty::Vec<Content>::with_capacity(std::move(len)), rusty::PhantomData<E>{}));
                }
                rusty::Result<SerializeMap, E> serialize_map(rusty::Option<size_t> len) {
                    return rusty::Result<SerializeMap, E>::Ok(SerializeMap(rusty::Vec<std::tuple<Content, Content>>::with_capacity(len.unwrap_or(0)), rusty::Option<Content>(rusty::None), rusty::PhantomData<E>{}));
                }
                rusty::Result<SerializeStruct, E> serialize_struct(std::string_view name, size_t len) {
                    return rusty::Result<SerializeStruct, E>::Ok(SerializeStruct(std::string_view(name), rusty::Vec<std::tuple<std::string_view, Content>>::with_capacity(std::move(len)), rusty::PhantomData<E>{}));
                }
                rusty::Result<SerializeStructVariant, E> serialize_struct_variant(std::string_view name, uint32_t variant_index, std::string_view variant, size_t len) {
                    return rusty::Result<SerializeStructVariant, E>::Ok(SerializeStructVariant(std::string_view(name), std::move(variant_index), std::string_view(variant), rusty::Vec<std::tuple<std::string_view, Content>>::with_capacity(std::move(len)), rusty::PhantomData<E>{}));
                }
            };

            template<typename E>
            struct SerializeSeq {
                using Ok = std::conditional_t<true, Content, E>;
                using Error = E;
                rusty::Vec<Content> elements;
                rusty::PhantomData<E> error;

                template<typename T>
                rusty::Result<std::tuple<>, E> serialize_element(const T& value) {
                    auto value_shadow1 = ({ auto&& _m = ::ser::impls::rusty_ext::serialize(value, ContentSerializer<E>::new_()); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<std::tuple<>, E>::Err(std::move(err)); } std::move(_match_value).value(); });
                    this->elements.push(std::move(value_shadow1));
                    return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
                }
                rusty::Result<Content, E> end() {
                    return rusty::Result<Content, E>::Ok(Content{Content_Seq{std::move(this->elements)}});
                }
            };

            template<typename E>
            struct SerializeTuple {
                using Ok = std::conditional_t<true, Content, E>;
                using Error = E;
                rusty::Vec<Content> elements;
                rusty::PhantomData<E> error;

                template<typename T>
                rusty::Result<std::tuple<>, E> serialize_element(const T& value) {
                    auto value_shadow1 = ({ auto&& _m = ::ser::impls::rusty_ext::serialize(value, ContentSerializer<E>::new_()); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<std::tuple<>, E>::Err(std::move(err)); } std::move(_match_value).value(); });
                    this->elements.push(std::move(value_shadow1));
                    return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
                }
                rusty::Result<Content, E> end() {
                    return rusty::Result<Content, E>::Ok(Content{Content_Tuple{std::move(this->elements)}});
                }
            };

            template<typename E>
            struct SerializeTupleStruct {
                using Ok = std::conditional_t<true, Content, E>;
                using Error = E;
                std::string_view name;
                rusty::Vec<Content> fields;
                rusty::PhantomData<E> error;

                template<typename T>
                rusty::Result<std::tuple<>, E> serialize_field(const T& value) {
                    auto value_shadow1 = ({ auto&& _m = ::ser::impls::rusty_ext::serialize(value, ContentSerializer<E>::new_()); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<std::tuple<>, E>::Err(std::move(err)); } std::move(_match_value).value(); });
                    this->fields.push(std::move(value_shadow1));
                    return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
                }
                rusty::Result<Content, E> end() {
                    return rusty::Result<Content, E>::Ok(Content{Content_TupleStruct{std::move(this->name), std::move(this->fields)}});
                }
            };

            template<typename E>
            struct SerializeTupleVariant {
                using Ok = std::conditional_t<true, Content, E>;
                using Error = E;
                std::string_view name;
                uint32_t variant_index;
                std::string_view variant;
                rusty::Vec<Content> fields;
                rusty::PhantomData<E> error;

                template<typename T>
                rusty::Result<std::tuple<>, E> serialize_field(const T& value) {
                    auto value_shadow1 = ({ auto&& _m = ::ser::impls::rusty_ext::serialize(value, ContentSerializer<E>::new_()); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<std::tuple<>, E>::Err(std::move(err)); } std::move(_match_value).value(); });
                    this->fields.push(std::move(value_shadow1));
                    return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
                }
                rusty::Result<Content, E> end() {
                    return rusty::Result<Content, E>::Ok(Content{Content_TupleVariant{std::move(this->name), std::move(this->variant_index), std::move(this->variant), std::move(this->fields)}});
                }
            };

            template<typename E>
            struct SerializeMap {
                using Ok = std::conditional_t<true, Content, E>;
                using Error = E;
                rusty::Vec<std::tuple<Content, Content>> entries;
                rusty::Option<Content> key;
                rusty::PhantomData<E> error;

                template<typename T>
                rusty::Result<std::tuple<>, E> serialize_key(const T& key) {
                    auto key_shadow1 = ({ auto&& _m = ::ser::impls::rusty_ext::serialize(key, ContentSerializer<E>::new_()); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<std::tuple<>, E>::Err(std::move(err)); } std::move(_match_value).value(); });
                    this->key = rusty::Option<Content>(std::move(key_shadow1));
                    return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
                }
                template<typename T>
                rusty::Result<std::tuple<>, E> serialize_value(const T& value) {
                    auto key = this->key.take().expect("serialize_value called before serialize_key");
                    auto value_shadow1 = ({ auto&& _m = ::ser::impls::rusty_ext::serialize(value, ContentSerializer<E>::new_()); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<std::tuple<>, E>::Err(std::move(err)); } std::move(_match_value).value(); });
                    this->entries.push(std::make_tuple(std::move(key), std::move(value_shadow1)));
                    return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
                }
                rusty::Result<Content, E> end() {
                    return rusty::Result<Content, E>::Ok(Content{Content_Map{std::move(this->entries)}});
                }
                template<typename K, typename V>
                rusty::Result<std::tuple<>, E> serialize_entry(const K& key, const V& value) {
                    auto key_shadow1 = ({ auto&& _m = ::ser::impls::rusty_ext::serialize(key, ContentSerializer<E>::new_()); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<std::tuple<>, E>::Err(std::move(err)); } std::move(_match_value).value(); });
                    auto value_shadow1 = ({ auto&& _m = ::ser::impls::rusty_ext::serialize(value, ContentSerializer<E>::new_()); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<std::tuple<>, E>::Err(std::move(err)); } std::move(_match_value).value(); });
                    this->entries.push(std::make_tuple(std::move(key_shadow1), std::move(value_shadow1)));
                    return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
                }
            };

            template<typename E>
            struct SerializeStruct {
                using Ok = std::conditional_t<true, Content, E>;
                using Error = E;
                std::string_view name;
                rusty::Vec<std::tuple<std::string_view, Content>> fields;
                rusty::PhantomData<E> error;

                template<typename T>
                rusty::Result<std::tuple<>, E> serialize_field(std::string_view key, const T& value) {
                    auto value_shadow1 = ({ auto&& _m = ::ser::impls::rusty_ext::serialize(value, ContentSerializer<E>::new_()); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<std::tuple<>, E>::Err(std::move(err)); } std::move(_match_value).value(); });
                    this->fields.push(std::tuple<std::string_view, Content>{std::string_view(key), std::move(value_shadow1)});
                    return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
                }
                rusty::Result<Content, E> end() {
                    return rusty::Result<Content, E>::Ok(Content{Content_Struct{std::move(this->name), std::move(this->fields)}});
                }
            };

            template<typename E>
            struct SerializeStructVariant {
                using Ok = std::conditional_t<true, Content, E>;
                using Error = E;
                std::string_view name;
                uint32_t variant_index;
                std::string_view variant;
                rusty::Vec<std::tuple<std::string_view, Content>> fields;
                rusty::PhantomData<E> error;

                template<typename T>
                rusty::Result<std::tuple<>, E> serialize_field(std::string_view key, const T& value) {
                    auto value_shadow1 = ({ auto&& _m = ::ser::impls::rusty_ext::serialize(value, ContentSerializer<E>::new_()); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<std::tuple<>, E>::Err(std::move(err)); } std::move(_match_value).value(); });
                    this->fields.push(std::tuple<std::string_view, Content>{std::string_view(key), std::move(value_shadow1)});
                    return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
                }
                rusty::Result<Content, E> end() {
                    return rusty::Result<Content, E>::Ok(Content{Content_StructVariant{std::move(this->name), std::move(this->variant_index), std::move(this->variant), std::move(this->fields)}});
                }
            };

        }

        template<typename M>
        struct FlatMapSerializer {
            using Ok = std::conditional_t<true, std::tuple<>, M>;
            // Rust-only dependent associated type alias skipped in constrained mode: Error
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeSeq
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeTuple
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeTupleStruct
            using SerializeMap = FlatMapSerializeMap<M>;
            using SerializeStruct = FlatMapSerializeStruct<M>;
            using SerializeTupleVariant = FlatMapSerializeTupleVariantAsMapValue<M>;
            using SerializeStructVariant = FlatMapSerializeStructVariantAsMapValue<M>;
            M& _0;

            static auto bad_type(const auto& what) {
                return M::Error::custom(std::format("can only flatten structs and maps (got {0})", rusty::to_string(what)));
            }
            // Rust-only dependent associated type alias skipped in constrained mode: Error
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeSeq
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeTuple
            // Rust-only dependent associated type alias skipped in constrained mode: SerializeTupleStruct
            rusty::Result<Ok, typename M::Error> serialize_bool(bool _arg1) {
                return rusty::Result<Ok, typename M::Error>::Err(FlatMapSerializer<M>::bad_type(Unsupported_Boolean()));
            }
            rusty::Result<Ok, typename M::Error> serialize_i8(int8_t _arg1) {
                return rusty::Result<Ok, typename M::Error>::Err(FlatMapSerializer<M>::bad_type(Unsupported_Integer()));
            }
            rusty::Result<Ok, typename M::Error> serialize_i16(int16_t _arg1) {
                return rusty::Result<Ok, typename M::Error>::Err(FlatMapSerializer<M>::bad_type(Unsupported_Integer()));
            }
            rusty::Result<Ok, typename M::Error> serialize_i32(int32_t _arg1) {
                return rusty::Result<Ok, typename M::Error>::Err(FlatMapSerializer<M>::bad_type(Unsupported_Integer()));
            }
            rusty::Result<Ok, typename M::Error> serialize_i64(int64_t _arg1) {
                return rusty::Result<Ok, typename M::Error>::Err(FlatMapSerializer<M>::bad_type(Unsupported_Integer()));
            }
            rusty::Result<Ok, typename M::Error> serialize_u8(uint8_t _arg1) {
                return rusty::Result<Ok, typename M::Error>::Err(FlatMapSerializer<M>::bad_type(Unsupported_Integer()));
            }
            rusty::Result<Ok, typename M::Error> serialize_u16(uint16_t _arg1) {
                return rusty::Result<Ok, typename M::Error>::Err(FlatMapSerializer<M>::bad_type(Unsupported_Integer()));
            }
            rusty::Result<Ok, typename M::Error> serialize_u32(uint32_t _arg1) {
                return rusty::Result<Ok, typename M::Error>::Err(FlatMapSerializer<M>::bad_type(Unsupported_Integer()));
            }
            rusty::Result<Ok, typename M::Error> serialize_u64(uint64_t _arg1) {
                return rusty::Result<Ok, typename M::Error>::Err(FlatMapSerializer<M>::bad_type(Unsupported_Integer()));
            }
            rusty::Result<Ok, typename M::Error> serialize_f32(float _arg1) {
                return rusty::Result<Ok, typename M::Error>::Err(FlatMapSerializer<M>::bad_type(Unsupported_Float()));
            }
            rusty::Result<Ok, typename M::Error> serialize_f64(double _arg1) {
                return rusty::Result<Ok, typename M::Error>::Err(FlatMapSerializer<M>::bad_type(Unsupported_Float()));
            }
            rusty::Result<Ok, typename M::Error> serialize_char(char32_t _arg1) {
                return rusty::Result<Ok, typename M::Error>::Err(FlatMapSerializer<M>::bad_type(Unsupported_Char()));
            }
            rusty::Result<Ok, typename M::Error> serialize_str(std::string_view _arg1) {
                return rusty::Result<Ok, typename M::Error>::Err(FlatMapSerializer<M>::bad_type(Unsupported_String()));
            }
            rusty::Result<Ok, typename M::Error> serialize_bytes(std::span<const uint8_t> _arg1) {
                return rusty::Result<Ok, typename M::Error>::Err(FlatMapSerializer<M>::bad_type(Unsupported_ByteArray()));
            }
            rusty::Result<Ok, typename M::Error> serialize_none() {
                return rusty::Result<Ok, typename M::Error>::Ok(std::make_tuple());
            }
            template<typename T>
            rusty::Result<Ok, typename M::Error> serialize_some(const T& value) {
                return ::ser::impls::rusty_ext::serialize(value, std::move((*this)));
            }
            rusty::Result<Ok, typename M::Error> serialize_unit() {
                return rusty::Result<Ok, typename M::Error>::Ok(std::make_tuple());
            }
            rusty::Result<Ok, typename M::Error> serialize_unit_struct(std::string_view _arg1) {
                return rusty::Result<Ok, typename M::Error>::Ok(std::make_tuple());
            }
            rusty::Result<Ok, typename M::Error> serialize_unit_variant(std::string_view _arg1, uint32_t _arg2, std::string_view variant) {
                return this->_0.serialize_entry(rusty::detail::deref_if_pointer_like(variant), std::make_tuple());
            }
            template<typename T>
            rusty::Result<Ok, typename M::Error> serialize_newtype_struct(std::string_view _arg1, const T& value) {
                return ::ser::impls::rusty_ext::serialize(value, std::move((*this)));
            }
            template<typename T>
            rusty::Result<Ok, typename M::Error> serialize_newtype_variant(std::string_view _arg1, uint32_t _arg2, std::string_view variant, const T& value) {
                return this->_0.serialize_entry(rusty::detail::deref_if_pointer_like(variant), rusty::detail::deref_if_pointer_like(value));
            }
            rusty::Result<ser::Impossible<Ok, typename M::Error>, typename M::Error> serialize_seq(rusty::Option<size_t> _arg1) {
                return rusty::Result<ser::Impossible<Ok, typename M::Error>, typename M::Error>::Err(FlatMapSerializer<M>::bad_type(Unsupported_Sequence()));
            }
            rusty::Result<ser::Impossible<Ok, typename M::Error>, typename M::Error> serialize_tuple(size_t _arg1) {
                return rusty::Result<ser::Impossible<Ok, typename M::Error>, typename M::Error>::Err(FlatMapSerializer<M>::bad_type(Unsupported_Tuple()));
            }
            rusty::Result<ser::Impossible<Ok, typename M::Error>, typename M::Error> serialize_tuple_struct(std::string_view _arg1, size_t _arg2) {
                return rusty::Result<ser::Impossible<Ok, typename M::Error>, typename M::Error>::Err(FlatMapSerializer<M>::bad_type(Unsupported_TupleStruct()));
            }
            rusty::Result<SerializeTupleVariant, typename M::Error> serialize_tuple_variant(std::string_view _arg1, uint32_t _arg2, std::string_view variant, size_t _arg4) {
                {
                    auto&& _m = this->_0.serialize_key(rusty::detail::deref_if_pointer_like(variant));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<SerializeTupleVariant, typename M::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return rusty::Result<SerializeTupleVariant, typename M::Error>::Ok(FlatMapSerializeTupleVariantAsMapValue<std::remove_cvref_t<decltype(std::move(this->_0))>>::new_(std::move(this->_0)));
            }
            rusty::Result<SerializeMap, typename M::Error> serialize_map(rusty::Option<size_t> _arg1) {
                return rusty::Result<SerializeMap, typename M::Error>::Ok(FlatMapSerializeMap(rusty::detail::deref_if_pointer_like(this->_0)));
            }
            rusty::Result<SerializeStruct, typename M::Error> serialize_struct(std::string_view _arg1, size_t _arg2) {
                return rusty::Result<SerializeStruct, typename M::Error>::Ok(FlatMapSerializeStruct(rusty::detail::deref_if_pointer_like(this->_0)));
            }
            rusty::Result<SerializeStructVariant, typename M::Error> serialize_struct_variant(std::string_view _arg1, uint32_t _arg2, std::string_view inner_variant, size_t _arg4) {
                {
                    auto&& _m = this->_0.serialize_key(rusty::detail::deref_if_pointer_like(inner_variant));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<SerializeStructVariant, typename M::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return rusty::Result<SerializeStructVariant, typename M::Error>::Ok(FlatMapSerializeStructVariantAsMapValue<M>::new_(std::move(this->_0), std::string_view(inner_variant)));
            }
        };

        template<typename M>
        struct FlatMapSerializeMap {
            using Ok = std::conditional_t<true, std::tuple<>, M>;
            // Rust-only dependent associated type alias skipped in constrained mode: Error
            M& _0;

            // Rust-only dependent associated type alias skipped in constrained mode: Error
            template<typename T>
            rusty::Result<std::tuple<>, typename M::Error> serialize_key(const T& key) {
                return this->_0.serialize_key(rusty::detail::deref_if_pointer_like(key));
            }
            template<typename T>
            rusty::Result<std::tuple<>, typename M::Error> serialize_value(const T& value) {
                return this->_0.serialize_value(rusty::detail::deref_if_pointer_like(value));
            }
            template<typename K, typename V>
            rusty::Result<std::tuple<>, typename M::Error> serialize_entry(const K& key, const V& value) {
                return this->_0.serialize_entry(rusty::detail::deref_if_pointer_like(key), rusty::detail::deref_if_pointer_like(value));
            }
            rusty::Result<std::tuple<>, typename M::Error> end() {
                return rusty::Result<std::tuple<>, typename M::Error>::Ok(std::make_tuple());
            }
        };

        template<typename M>
        struct FlatMapSerializeStruct {
            using Ok = std::conditional_t<true, std::tuple<>, M>;
            // Rust-only dependent associated type alias skipped in constrained mode: Error
            M& _0;

            // Rust-only dependent associated type alias skipped in constrained mode: Error
            template<typename T>
            rusty::Result<std::tuple<>, typename M::Error> serialize_field(std::string_view key, const T& value) {
                return this->_0.serialize_entry(rusty::detail::deref_if_pointer_like(key), rusty::detail::deref_if_pointer_like(value));
            }
            rusty::Result<std::tuple<>, typename M::Error> end() {
                return rusty::Result<std::tuple<>, typename M::Error>::Ok(std::make_tuple());
            }
        };

        template<typename M>
        struct FlatMapSerializeTupleVariantAsMapValue {
            using Ok = std::conditional_t<true, std::tuple<>, M>;
            // Rust-only dependent associated type alias skipped in constrained mode: Error
            M& map;
            rusty::Vec<::private_::ser::content::Content> fields;

            static FlatMapSerializeTupleVariantAsMapValue<M> new_(M& map) {
                return FlatMapSerializeTupleVariantAsMapValue<M>{.map = map, .fields = rusty::Vec<::private_::ser::content::Content>::new_()};
            }
            // Rust-only dependent associated type alias skipped in constrained mode: Error
            template<typename T>
            rusty::Result<std::tuple<>, typename M::Error> serialize_field(const T& value) {
                auto value_shadow1 = ({ auto&& _m = ::ser::impls::rusty_ext::serialize(value, ContentSerializer<typename M::Error>::new_()); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<std::tuple<>, typename M::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                this->fields.push(std::move(value_shadow1));
                return rusty::Result<std::tuple<>, typename M::Error>::Ok(std::make_tuple());
            }
            rusty::Result<std::tuple<>, typename M::Error> end() {
                {
                    auto&& _m = this->map.serialize_value(::private_::ser::content::Content_Seq{std::move(this->fields)});
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<std::tuple<>, typename M::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return rusty::Result<std::tuple<>, typename M::Error>::Ok(std::make_tuple());
            }
        };

        template<typename M>
        struct FlatMapSerializeStructVariantAsMapValue {
            using Ok = std::conditional_t<true, std::tuple<>, M>;
            // Rust-only dependent associated type alias skipped in constrained mode: Error
            M& map;
            std::string_view name;
            rusty::Vec<std::tuple<std::string_view, ::private_::ser::content::Content>> fields;

            static FlatMapSerializeStructVariantAsMapValue<M> new_(M& map, std::string_view name) {
                return FlatMapSerializeStructVariantAsMapValue<M>{.map = map, .name = std::string_view(name), .fields = rusty::Vec<std::tuple<std::string_view, ::private_::ser::content::Content>>::new_()};
            }
            // Rust-only dependent associated type alias skipped in constrained mode: Error
            template<typename T>
            rusty::Result<std::tuple<>, typename M::Error> serialize_field(std::string_view key, const T& value) {
                auto value_shadow1 = ({ auto&& _m = ::ser::impls::rusty_ext::serialize(value, ContentSerializer<typename M::Error>::new_()); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<std::tuple<>, typename M::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                this->fields.push(std::tuple<std::string_view, ::private_::ser::content::Content>{std::string_view(key), std::move(value_shadow1)});
                return rusty::Result<std::tuple<>, typename M::Error>::Ok(std::make_tuple());
            }
            rusty::Result<std::tuple<>, typename M::Error> end() {
                {
                    auto&& _m = this->map.serialize_value(::private_::ser::content::Content_Struct{std::move(this->name), std::move(this->fields)});
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& val = _mv0;
                            val;
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& err = _mv1;
                            return rusty::Result<std::tuple<>, typename M::Error>::Err(std::move(err));
                            _m_matched = true;
                        }
                    }
                }
                return rusty::Result<std::tuple<>, typename M::Error>::Ok(std::make_tuple());
            }
        };

        struct AdjacentlyTaggedEnumVariant {
            std::string_view enum_name;
            uint32_t variant_index;
            std::string_view variant_name;

            template<typename S>
            auto serialize(S serializer) const;
        };

        template<typename T>
        struct CannotSerializeVariant {
            T _0;

            rusty::fmt::Result fmt(rusty::fmt::Formatter& formatter) const {
                return rusty::write_fmt(formatter, std::format("enum variant cannot be serialized: {0}", rusty::to_debug_string(this->_0)));
            }
        };

        /// Used to check that serde(getter) attributes return the expected type.
        /// Not public API.
        template<typename T>
        const T& constrain(const T& t) {
            return t;
        }

        /// Not public API.
        template<typename S, typename T>
        rusty::Result<typename S::Ok, typename S::Error> serialize_tagged_newtype(S serializer, std::string_view type_ident, std::string_view variant_ident, std::string_view tag, std::string_view variant_name, const T& value) {
            return ::ser::impls::rusty_ext::serialize(value, TaggedSerializer<S>{.type_ident = std::string_view(type_ident), .variant_ident = std::string_view(variant_ident), .tag = std::string_view(tag), .variant_name = std::string_view(variant_name), .delegate = std::move(serializer)});
        }

    }

}

namespace __private228 {

    using namespace ::private_;

}

namespace serde_core_private = ::__private228;

namespace integer128 {
}

// Rust-only libtest main omitted


namespace private_::de::content {
            ContentVisitor ContentVisitor::new_() {
                return ContentVisitor{.value = rusty::PhantomData<serde_core_private::Content>{}};
            }
}

namespace private_::de::content {
            template<typename D>
            rusty::Result<serde_core_private::Content, typename D::Error> ContentVisitor::deserialize(D deserializer) {
                return deserializer.__deserialize_content_v1(std::move((*this)));
            }
}

namespace private_::de::content {
            rusty::fmt::Result ContentVisitor::expecting(rusty::fmt::Formatter& fmt) const {
                return fmt.write_str("any value");
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_bool(bool value) {
                return rusty::Result<serde_core_private::Content, F>::Ok(std::conditional_t<true, Content, F>::Bool(std::move(value)));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_i8(int8_t value) {
                return rusty::Result<serde_core_private::Content, F>::Ok(std::conditional_t<true, Content, F>::I8(std::move(value)));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_i16(int16_t value) {
                return rusty::Result<serde_core_private::Content, F>::Ok(std::conditional_t<true, Content, F>::I16(std::move(value)));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_i32(int32_t value) {
                return rusty::Result<serde_core_private::Content, F>::Ok(std::conditional_t<true, Content, F>::I32(std::move(value)));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_i64(int64_t value) {
                return rusty::Result<serde_core_private::Content, F>::Ok(std::conditional_t<true, Content, F>::I64(std::move(value)));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_u8(uint8_t value) {
                return rusty::Result<serde_core_private::Content, F>::Ok(std::conditional_t<true, Content, F>::U8(std::move(value)));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_u16(uint16_t value) {
                return rusty::Result<serde_core_private::Content, F>::Ok(std::conditional_t<true, Content, F>::U16(std::move(value)));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_u32(uint32_t value) {
                return rusty::Result<serde_core_private::Content, F>::Ok(std::conditional_t<true, Content, F>::U32(std::move(value)));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_u64(uint64_t value) {
                return rusty::Result<serde_core_private::Content, F>::Ok(std::conditional_t<true, Content, F>::U64(std::move(value)));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_f32(float value) {
                return rusty::Result<serde_core_private::Content, F>::Ok(std::conditional_t<true, Content, F>::F32(std::move(value)));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_f64(double value) {
                return rusty::Result<serde_core_private::Content, F>::Ok(std::conditional_t<true, Content, F>::F64(std::move(value)));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_char(char32_t value) {
                return rusty::Result<serde_core_private::Content, F>::Ok(std::conditional_t<true, Content, F>::Char(std::move(value)));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_str(std::string_view value) {
                return rusty::Result<serde_core_private::Content, F>::Ok(std::conditional_t<true, Content, F>::String(rusty::String::from(value)));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_borrowed_str(std::string_view value) {
                return rusty::Result<serde_core_private::Content, F>::Ok(std::conditional_t<true, Content, F>::Str(value));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_string(rusty::String value) {
                return rusty::Result<serde_core_private::Content, F>::Ok(std::conditional_t<true, Content, F>::String(std::move(value)));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_bytes(std::span<const uint8_t> value) {
                return rusty::Result<serde_core_private::Content, F>::Ok(std::conditional_t<true, Content, F>::ByteBuf(rusty::to_vec(value)));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_borrowed_bytes(std::span<const uint8_t> value) {
                return rusty::Result<serde_core_private::Content, F>::Ok(std::conditional_t<true, Content, F>::Bytes(value));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_byte_buf(rusty::Vec<uint8_t> value) {
                return rusty::Result<serde_core_private::Content, F>::Ok(std::conditional_t<true, Content, F>::ByteBuf(std::move(value)));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_unit() {
                return rusty::Result<serde_core_private::Content, F>::Ok(Content::Unit);
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<serde_core_private::Content, F> ContentVisitor::visit_none() {
                return rusty::Result<serde_core_private::Content, F>::Ok(Content::None);
            }
}

namespace private_::de::content {
            template<typename D>
            rusty::Result<serde_core_private::Content, typename D::Error> ContentVisitor::visit_some(D deserializer) {
                const auto v = ({ auto&& _m = ::de::rusty_ext::deserialize(std::conditional_t<true, ContentVisitor, D>::new_(), std::move(deserializer)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<serde_core_private::Content, typename D::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                return rusty::Result<serde_core_private::Content, typename D::Error>::Ok(std::conditional_t<true, Content, D>::Some(rusty::Box<std::remove_cvref_t<decltype((v))>>::new_(std::move(v))));
            }
}

namespace private_::de::content {
            template<typename D>
            rusty::Result<serde_core_private::Content, typename D::Error> ContentVisitor::visit_newtype_struct(D deserializer) {
                const auto v = ({ auto&& _m = ::de::rusty_ext::deserialize(std::conditional_t<true, ContentVisitor, D>::new_(), std::move(deserializer)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<serde_core_private::Content, typename D::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                return rusty::Result<serde_core_private::Content, typename D::Error>::Ok(std::conditional_t<true, Content, D>::Newtype(rusty::Box<std::remove_cvref_t<decltype((v))>>::new_(std::move(v))));
            }
}

namespace private_::de::content {
            template<typename V>
            rusty::Result<serde_core_private::Content, typename V::Error> ContentVisitor::visit_seq(V visitor) {
                auto vec = std::conditional_t<true, rusty::Vec<serde_core_private::Content>, V>::with_capacity(size_hint::cautious<serde_core_private::Content>(visitor.size_hint()));
                while (true) {
                    auto&& _whilelet = ({ auto&& _m = visitor.next_element_seed(std::conditional_t<true, ContentVisitor, V>::new_()); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<serde_core_private::Content, typename V::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                    if (!(_whilelet.is_some())) { break; }
                    auto e = _whilelet.unwrap();
                    vec.push(std::move(e));
                }
                return rusty::Result<serde_core_private::Content, typename V::Error>::Ok(std::conditional_t<true, Content, V>::Seq(std::move(vec)));
            }
}

namespace private_::de::content {
            template<typename V>
            rusty::Result<serde_core_private::Content, typename V::Error> ContentVisitor::visit_map(V visitor) {
                auto vec = std::conditional_t<true, rusty::Vec<std::tuple<serde_core_private::Content, serde_core_private::Content>>, V>::with_capacity(size_hint::cautious<std::tuple<serde_core_private::Content, serde_core_private::Content>>(visitor.size_hint()));
                while (true) {
                    auto&& _whilelet = ({ auto&& _m = visitor.next_entry_seed(std::conditional_t<true, ContentVisitor, V>::new_(), std::conditional_t<true, ContentVisitor, V>::new_()); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<serde_core_private::Content, typename V::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
                    if (!(_whilelet.is_some())) { break; }
                    auto kv = _whilelet.unwrap();
                    vec.push(std::move(kv));
                }
                return rusty::Result<serde_core_private::Content, typename V::Error>::Ok(std::conditional_t<true, Content, V>::Map(std::move(vec)));
            }
}

namespace private_::de::content {
            template<typename V>
            rusty::Result<serde_core_private::Content, typename V::Error> ContentVisitor::visit_enum(V _visitor) {
                return rusty::Result<serde_core_private::Content, typename V::Error>::Err(V::Error::custom("untagged and internally tagged enums do not support enum input"));
            }
}

namespace private_::de::content {
            TagOrContentVisitor TagOrContentVisitor::new_(std::string_view name) {
                return TagOrContentVisitor{.name = std::string_view(name), .value = rusty::PhantomData<TagOrContent>{}};
            }
}

namespace private_::de::content {
            template<typename D>
            rusty::Result<TagOrContent, typename D::Error> TagOrContentVisitor::deserialize(D deserializer) {
                return deserializer.deserialize_any(std::move((*this)));
            }
}

namespace private_::de::content {
            rusty::fmt::Result TagOrContentVisitor::expecting(rusty::fmt::Formatter& fmt) const {
                return rusty::write_fmt(fmt, std::format("a type tag `{0}` or any other value", rusty::to_string(this->name)));
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_bool(bool value) {
                return std::conditional_t<true, ContentVisitor, F>::new_().template visit_bool<F>(std::move(value)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_i8(int8_t value) {
                return std::conditional_t<true, ContentVisitor, F>::new_().template visit_i8<F>(std::move(value)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_i16(int16_t value) {
                return std::conditional_t<true, ContentVisitor, F>::new_().template visit_i16<F>(std::move(value)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_i32(int32_t value) {
                return std::conditional_t<true, ContentVisitor, F>::new_().template visit_i32<F>(std::move(value)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_i64(int64_t value) {
                return std::conditional_t<true, ContentVisitor, F>::new_().template visit_i64<F>(std::move(value)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_u8(uint8_t value) {
                return std::conditional_t<true, ContentVisitor, F>::new_().template visit_u8<F>(std::move(value)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_u16(uint16_t value) {
                return std::conditional_t<true, ContentVisitor, F>::new_().template visit_u16<F>(std::move(value)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_u32(uint32_t value) {
                return std::conditional_t<true, ContentVisitor, F>::new_().template visit_u32<F>(std::move(value)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_u64(uint64_t value) {
                return std::conditional_t<true, ContentVisitor, F>::new_().template visit_u64<F>(std::move(value)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_f32(float value) {
                return std::conditional_t<true, ContentVisitor, F>::new_().template visit_f32<F>(std::move(value)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_f64(double value) {
                return std::conditional_t<true, ContentVisitor, F>::new_().template visit_f64<F>(std::move(value)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_char(char32_t value) {
                return std::conditional_t<true, ContentVisitor, F>::new_().template visit_char<F>(std::move(value)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_str(std::string_view value) {
                if (std::string_view(value) == std::string_view(this->name)) {
                    return rusty::Result<TagOrContent, F>::Ok(TagOrContent_Tag{});
                } else {
                    return std::conditional_t<true, ContentVisitor, F>::new_().template visit_str<F>(std::string_view(value)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
                }
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_borrowed_str(std::string_view value) {
                if (std::string_view(value) == std::string_view(this->name)) {
                    return rusty::Result<TagOrContent, F>::Ok(TagOrContent_Tag{});
                } else {
                    return std::conditional_t<true, ContentVisitor, F>::new_().template visit_borrowed_str<F>(std::string_view(value)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
                }
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_string(rusty::String value) {
                if (std::string_view(value.as_str()) == this->name) {
                    return rusty::Result<TagOrContent, F>::Ok(TagOrContent_Tag{});
                } else {
                    return std::conditional_t<true, ContentVisitor, F>::new_().template visit_string<F>(std::move(value)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
                }
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_bytes(std::span<const uint8_t> value) {
                if (value == rusty::as_bytes(this->name)) {
                    return rusty::Result<TagOrContent, F>::Ok(TagOrContent_Tag{});
                } else {
                    return std::conditional_t<true, ContentVisitor, F>::new_().template visit_bytes<F>(value).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
                }
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_borrowed_bytes(std::span<const uint8_t> value) {
                if (value == rusty::as_bytes(this->name)) {
                    return rusty::Result<TagOrContent, F>::Ok(TagOrContent_Tag{});
                } else {
                    return std::conditional_t<true, ContentVisitor, F>::new_().template visit_borrowed_bytes<F>(value).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
                }
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_byte_buf(rusty::Vec<uint8_t> value) {
                if (value == rusty::as_bytes(this->name)) {
                    return rusty::Result<TagOrContent, F>::Ok(TagOrContent_Tag{});
                } else {
                    return std::conditional_t<true, ContentVisitor, F>::new_().template visit_byte_buf<F>(std::move(value)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
                }
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_unit() {
                return std::conditional_t<true, ContentVisitor, F>::new_().template visit_unit<F>().map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename F>
            rusty::Result<TagOrContent, F> TagOrContentVisitor::visit_none() {
                return std::conditional_t<true, ContentVisitor, F>::new_().template visit_none<F>().map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename D>
            rusty::Result<TagOrContent, typename D::Error> TagOrContentVisitor::visit_some(D deserializer) {
                return std::conditional_t<true, ContentVisitor, D>::new_().visit_some(std::move(deserializer)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename D>
            rusty::Result<TagOrContent, typename D::Error> TagOrContentVisitor::visit_newtype_struct(D deserializer) {
                return std::conditional_t<true, ContentVisitor, D>::new_().visit_newtype_struct(std::move(deserializer)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename V>
            rusty::Result<TagOrContent, typename V::Error> TagOrContentVisitor::visit_seq(V visitor) {
                return std::conditional_t<true, ContentVisitor, V>::new_().visit_seq(std::move(visitor)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename V>
            rusty::Result<TagOrContent, typename V::Error> TagOrContentVisitor::visit_map(V visitor) {
                return std::conditional_t<true, ContentVisitor, V>::new_().visit_map(std::move(visitor)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename V>
            rusty::Result<TagOrContent, typename V::Error> TagOrContentVisitor::visit_enum(V visitor) {
                return std::conditional_t<true, ContentVisitor, V>::new_().visit_enum(std::move(visitor)).map([](auto&& _v) { return TagOrContent_Content{std::forward<decltype(_v)>(_v)}; });
            }
}

namespace private_::de::content {
            template<typename D>
            rusty::Result<TagOrContentField, typename D::Error> TagOrContentFieldVisitor::deserialize(D deserializer) {
                return deserializer.deserialize_identifier(std::move((*this)));
            }
}

namespace private_::de::content {
            rusty::fmt::Result TagOrContentFieldVisitor::expecting(rusty::fmt::Formatter& formatter) const {
                return rusty::write_fmt(formatter, std::format("{0} or {1}", rusty::to_debug_string(this->tag), rusty::to_debug_string(this->content)));
            }
}

namespace private_::de::content {
            template<typename E>
            rusty::Result<TagOrContentField, E> TagOrContentFieldVisitor::visit_u64(uint64_t field_index) {
                return ({ auto&& _m = field_index; std::optional<rusty::Result<TagOrContentField, E>> _match_value; bool _m_matched = false; if (!_m_matched && (_m == 0)) { _match_value.emplace(std::move(rusty::Result<TagOrContentField, E>::Ok(TagOrContentField_Tag()))); _m_matched = true; } if (!_m_matched && (_m == 1)) { _match_value.emplace(std::move(rusty::Result<TagOrContentField, E>::Ok(TagOrContentField_Content()))); _m_matched = true; } if (!_m_matched) { _match_value.emplace(std::move(rusty::Result<TagOrContentField, E>::Err(E::invalid_value(std::conditional_t<true, Unexpected, E>::Unsigned(std::move(field_index)), (*this))))); _m_matched = true; } if (!_m_matched) { rusty::intrinsics::unreachable(); } std::move(_match_value).value(); });
            }
}

namespace private_::de::content {
            template<typename E>
            rusty::Result<TagOrContentField, E> TagOrContentFieldVisitor::visit_str(std::string_view field) {
                if (std::string_view(field) == std::string_view(this->tag)) {
                    return rusty::Result<TagOrContentField, E>::Ok(TagOrContentField_Tag());
                } else if (std::string_view(field) == std::string_view(this->content)) {
                    return rusty::Result<TagOrContentField, E>::Ok(TagOrContentField_Content());
                } else {
                    return rusty::Result<TagOrContentField, E>::Err(E::invalid_value(std::conditional_t<true, Unexpected, E>::Str(field), (*this)));
                }
            }
}

namespace private_::de::content {
            template<typename E>
            rusty::Result<TagOrContentField, E> TagOrContentFieldVisitor::visit_bytes(std::span<const uint8_t> field) {
                if (field == rusty::as_bytes(this->tag)) {
                    return rusty::Result<TagOrContentField, E>::Ok(TagOrContentField_Tag());
                } else if (field == rusty::as_bytes(this->content)) {
                    return rusty::Result<TagOrContentField, E>::Ok(TagOrContentField_Content());
                } else {
                    return rusty::Result<TagOrContentField, E>::Err(E::invalid_value(std::conditional_t<true, Unexpected, E>::Bytes(field), (*this)));
                }
            }
}

namespace private_::de::content {
            template<typename D>
            rusty::Result<TagContentOtherField, typename D::Error> TagContentOtherFieldVisitor::deserialize(D deserializer) {
                return deserializer.deserialize_identifier(std::move((*this)));
            }
}

namespace private_::de::content {
            rusty::fmt::Result TagContentOtherFieldVisitor::expecting(rusty::fmt::Formatter& formatter) const {
                return rusty::write_fmt(formatter, std::format("{0}, {1}, or other ignored fields", rusty::to_debug_string(this->tag), rusty::to_debug_string(this->content)));
            }
}

namespace private_::de::content {
            template<typename E>
            rusty::Result<TagContentOtherField, E> TagContentOtherFieldVisitor::visit_u64(uint64_t field_index) {
                return ({ auto&& _m = field_index; std::optional<rusty::Result<TagContentOtherField, E>> _match_value; bool _m_matched = false; if (!_m_matched && (_m == 0)) { _match_value.emplace(std::move(rusty::Result<TagContentOtherField, E>::Ok(TagContentOtherField_Tag()))); _m_matched = true; } if (!_m_matched && (_m == 1)) { _match_value.emplace(std::move(rusty::Result<TagContentOtherField, E>::Ok(TagContentOtherField_Content()))); _m_matched = true; } if (!_m_matched) { _match_value.emplace(std::move(rusty::Result<TagContentOtherField, E>::Ok(TagContentOtherField_Other()))); _m_matched = true; } if (!_m_matched) { rusty::intrinsics::unreachable(); } std::move(_match_value).value(); });
            }
}

namespace private_::de::content {
            template<typename E>
            rusty::Result<TagContentOtherField, E> TagContentOtherFieldVisitor::visit_str(std::string_view field) {
                return this->template visit_bytes<E>(rusty::as_bytes(field));
            }
}

namespace private_::de::content {
            template<typename E>
            rusty::Result<TagContentOtherField, E> TagContentOtherFieldVisitor::visit_bytes(std::span<const uint8_t> field) {
                if (field == rusty::as_bytes(this->tag)) {
                    return rusty::Result<TagContentOtherField, E>::Ok(TagContentOtherField_Tag());
                } else if (field == rusty::as_bytes(this->content)) {
                    return rusty::Result<TagContentOtherField, E>::Ok(TagContentOtherField_Content());
                } else {
                    return rusty::Result<TagContentOtherField, E>::Ok(TagContentOtherField_Other());
                }
            }
}

namespace private_::de::content {
            rusty::fmt::Result ExpectedInSeq::fmt(rusty::fmt::Formatter& formatter) const {
                if (this->_0 == 1) {
                    return formatter.write_str("1 element in sequence");
                } else {
                    return rusty::write_fmt(formatter, std::format("{0} elements in sequence", rusty::to_string(this->_0)));
                }
            }
}

namespace private_::de::content {
            rusty::fmt::Result ExpectedInMap::fmt(rusty::fmt::Formatter& formatter) const {
                if (this->_0 == 1) {
                    return formatter.write_str("1 element in map");
                } else {
                    return rusty::write_fmt(formatter, std::format("{0} elements in map", rusty::to_string(this->_0)));
                }
            }
}

namespace private_::de::content {
            InternallyTaggedUnitVisitor InternallyTaggedUnitVisitor::new_(std::string_view type_name, std::string_view variant_name) {
                return InternallyTaggedUnitVisitor{.type_name = std::string_view(type_name), .variant_name = std::string_view(variant_name)};
            }
}

namespace private_::de::content {
            rusty::fmt::Result InternallyTaggedUnitVisitor::expecting(rusty::fmt::Formatter& formatter) const {
                return rusty::write_fmt(formatter, std::format("unit variant {0}::{1}", rusty::to_string(this->type_name), rusty::to_string(this->variant_name)));
            }
}

namespace private_::de::content {
            template<typename S>
            auto InternallyTaggedUnitVisitor::visit_seq(S _arg1) {
                return rusty::Result<std::tuple<>, typename S::Error>::Ok(std::make_tuple());
            }
}

namespace private_::de::content {
            template<typename M>
            auto InternallyTaggedUnitVisitor::visit_map(M access) {
                while (({ auto&& _m = access.template next_entry<::de::IgnoredAny, ::de::IgnoredAny>(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<std::tuple<>, typename M::Error>::Err(std::move(err)); } std::move(_match_value).value(); }).is_some()) {
                }
                return rusty::Result<std::tuple<>, typename M::Error>::Ok(std::make_tuple());
            }
}

namespace private_::de::content {
            UntaggedUnitVisitor UntaggedUnitVisitor::new_(std::string_view type_name, std::string_view variant_name) {
                return UntaggedUnitVisitor{.type_name = std::string_view(type_name), .variant_name = std::string_view(variant_name)};
            }
}

namespace private_::de::content {
            rusty::fmt::Result UntaggedUnitVisitor::expecting(rusty::fmt::Formatter& formatter) const {
                return rusty::write_fmt(formatter, std::format("unit variant {0}::{1}", rusty::to_string(this->type_name), rusty::to_string(this->variant_name)));
            }
}

namespace private_::de::content {
            template<typename E>
            rusty::Result<std::tuple<>, E> UntaggedUnitVisitor::visit_unit() {
                return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
            }
}

namespace private_::de::content {
            template<typename E>
            rusty::Result<std::tuple<>, E> UntaggedUnitVisitor::visit_none() {
                return rusty::Result<std::tuple<>, E>::Ok(std::make_tuple());
            }
}

namespace private_::ser::content {
            template<typename S>
            auto Content::serialize(S serializer) const {
                return [&]() { auto&& _m = (*this); return std::visit(overloaded { [&](const Content_Bool& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& b = _v._0; return serializer.serialize_bool(std::move(b)); }, [&](const Content_U8& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& u = _v._0; return serializer.serialize_u8(std::move(u)); }, [&](const Content_U16& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& u = _v._0; return serializer.serialize_u16(std::move(u)); }, [&](const Content_U32& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& u = _v._0; return serializer.serialize_u32(std::move(u)); }, [&](const Content_U64& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& u = _v._0; return serializer.serialize_u64(std::move(u)); }, [&](const Content_I8& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& i = _v._0; return serializer.serialize_i8(std::move(i)); }, [&](const Content_I16& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& i = _v._0; return serializer.serialize_i16(std::move(i)); }, [&](const Content_I32& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& i = _v._0; return serializer.serialize_i32(std::move(i)); }, [&](const Content_I64& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& i = _v._0; return serializer.serialize_i64(std::move(i)); }, [&](const Content_F32& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& f = _v._0; return serializer.serialize_f32(std::move(f)); }, [&](const Content_F64& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& f = _v._0; return serializer.serialize_f64(std::move(f)); }, [&](const Content_Char& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& c = _v._0; return serializer.serialize_char(std::move(c)); }, [&](const Content_String& _v) -> rusty::Result<typename S::Ok, typename S::Error> { const auto& s = _v._0; return serializer.serialize_str(std::move(rusty::to_string_view(s))); }, [&](const Content_Bytes& _v) -> rusty::Result<typename S::Ok, typename S::Error> { const auto& b = _v._0; return serializer.serialize_bytes(b); }, [&](const Content_None&) -> rusty::Result<typename S::Ok, typename S::Error> { return serializer.serialize_none(); }, [&](const Content_Some& _v) -> rusty::Result<typename S::Ok, typename S::Error> { const auto& c = _v._0; return serializer.serialize_some(rusty::detail::deref_if_pointer_like(rusty::detail::deref_if_pointer_like(c))); }, [&](const Content_Unit&) -> rusty::Result<typename S::Ok, typename S::Error> { return serializer.serialize_unit(); }, [&](const Content_UnitStruct& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& n = _v._0; return serializer.serialize_unit_struct(std::move(rusty::to_string_view(n))); }, [&](const Content_UnitVariant& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& n = _v._0;
auto&& i = _v._1;
auto&& v = _v._2; return serializer.serialize_unit_variant(std::move(rusty::to_string_view(n)), std::move(i), std::move(rusty::to_string_view(v))); }, [&](const Content_NewtypeStruct& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& n = _v._0;
const auto& c = _v._1; return serializer.serialize_newtype_struct(std::move(rusty::to_string_view(n)), rusty::detail::deref_if_pointer_like(rusty::detail::deref_if_pointer_like(c))); }, [&](const Content_NewtypeVariant& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& n = _v._0;
auto&& i = _v._1;
auto&& v = _v._2;
const auto& c = _v._3; return serializer.serialize_newtype_variant(std::move(rusty::to_string_view(n)), std::move(i), std::move(rusty::to_string_view(v)), rusty::detail::deref_if_pointer_like(rusty::detail::deref_if_pointer_like(c))); }, [&](const Content_Seq& _v) -> rusty::Result<typename S::Ok, typename S::Error> { const auto& elements = _v._0; return ::ser::impls::rusty_ext::serialize(elements, std::move(serializer)); }, [&](const Content_Tuple& _v) -> rusty::Result<typename S::Ok, typename S::Error> { const auto& elements = _v._0; return [&]() -> rusty::Result<typename S::Ok, typename S::Error> { // Rust-only: using ser::SerializeTuple;
auto tuple = ({ auto&& _m = serializer.serialize_tuple(rusty::len(elements)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
for (auto&& e : rusty::for_in(elements)) {
    {
        auto&& _m = tuple.serialize_element(rusty::detail::deref_if_pointer_like(e));
        bool _m_matched = false;
        if (!_m_matched) {
            if (_m.is_ok()) {
                auto&& _mv0 = _m.unwrap();
                auto&& val = _mv0;
                val;
                _m_matched = true;
            }
        }
        if (!_m_matched) {
            if (_m.is_err()) {
                auto&& _mv1 = _m.unwrap_err();
                auto&& err = _mv1;
                return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                _m_matched = true;
            }
        }
    }
}
return tuple.end(); }(); }, [&](const Content_TupleStruct& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& n = _v._0;
const auto& fields = _v._1; return [&]() -> rusty::Result<typename S::Ok, typename S::Error> { // Rust-only: using ser::SerializeTupleStruct;
auto ts = ({ auto&& _m = serializer.serialize_tuple_struct(std::move(rusty::to_string_view(n)), rusty::len(fields)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
for (auto&& f : rusty::for_in(fields)) {
    {
        auto&& _m = ts.serialize_field(std::move(f));
        bool _m_matched = false;
        if (!_m_matched) {
            if (_m.is_ok()) {
                auto&& _mv0 = _m.unwrap();
                auto&& val = _mv0;
                val;
                _m_matched = true;
            }
        }
        if (!_m_matched) {
            if (_m.is_err()) {
                auto&& _mv1 = _m.unwrap_err();
                auto&& err = _mv1;
                return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                _m_matched = true;
            }
        }
    }
}
return ts.end(); }(); }, [&](const Content_TupleVariant& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& n = _v._0;
auto&& i = _v._1;
auto&& v = _v._2;
const auto& fields = _v._3; return [&]() -> rusty::Result<typename S::Ok, typename S::Error> { // Rust-only: using ser::SerializeTupleVariant;
auto tv = ({ auto&& _m = serializer.serialize_tuple_variant(std::move(rusty::to_string_view(n)), std::move(i), std::move(rusty::to_string_view(v)), rusty::len(fields)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
for (auto&& f : rusty::for_in(fields)) {
    {
        auto&& _m = tv.serialize_field(std::move(f));
        bool _m_matched = false;
        if (!_m_matched) {
            if (_m.is_ok()) {
                auto&& _mv0 = _m.unwrap();
                auto&& val = _mv0;
                val;
                _m_matched = true;
            }
        }
        if (!_m_matched) {
            if (_m.is_err()) {
                auto&& _mv1 = _m.unwrap_err();
                auto&& err = _mv1;
                return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                _m_matched = true;
            }
        }
    }
}
return tv.end(); }(); }, [&](const Content_Map& _v) -> rusty::Result<typename S::Ok, typename S::Error> { const auto& entries = _v._0; return [&]() -> rusty::Result<typename S::Ok, typename S::Error> { // Rust-only: using ser::SerializeMap;
auto map = ({ auto&& _m = serializer.serialize_map(rusty::Option<size_t>(rusty::len(entries))); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
for (auto&& _for_item : rusty::for_in(entries)) {
    auto&& k = std::get<0>(rusty::detail::deref_if_pointer(_for_item));
    auto&& v = std::get<1>(rusty::detail::deref_if_pointer(_for_item));
    {
        auto&& _m = map.serialize_entry(rusty::detail::deref_if_pointer_like(k), rusty::detail::deref_if_pointer_like(v));
        bool _m_matched = false;
        if (!_m_matched) {
            if (_m.is_ok()) {
                auto&& _mv0 = _m.unwrap();
                auto&& val = _mv0;
                val;
                _m_matched = true;
            }
        }
        if (!_m_matched) {
            if (_m.is_err()) {
                auto&& _mv1 = _m.unwrap_err();
                auto&& err = _mv1;
                return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                _m_matched = true;
            }
        }
    }
}
return map.end(); }(); }, [&](const Content_Struct& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& n = _v._0;
const auto& fields = _v._1; return [&]() -> rusty::Result<typename S::Ok, typename S::Error> { // Rust-only: using ser::SerializeStruct;
auto s = ({ auto&& _m = serializer.serialize_struct(std::move(rusty::to_string_view(n)), rusty::len(fields)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
for (auto&& _for_item : rusty::for_in(fields)) {
    auto&& k = std::get<0>(rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_for_item)));
    const auto& v = std::get<1>(rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_for_item)));
    {
        auto&& _m = s.serialize_field(std::move(k), rusty::detail::deref_if_pointer_like(v));
        bool _m_matched = false;
        if (!_m_matched) {
            if (_m.is_ok()) {
                auto&& _mv0 = _m.unwrap();
                auto&& val = _mv0;
                val;
                _m_matched = true;
            }
        }
        if (!_m_matched) {
            if (_m.is_err()) {
                auto&& _mv1 = _m.unwrap_err();
                auto&& err = _mv1;
                return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                _m_matched = true;
            }
        }
    }
}
return s.end(); }(); }, [&](const Content_StructVariant& _v) -> rusty::Result<typename S::Ok, typename S::Error> { auto&& n = _v._0;
auto&& i = _v._1;
auto&& v = _v._2;
const auto& fields = _v._3; return [&]() -> rusty::Result<typename S::Ok, typename S::Error> { // Rust-only: using ser::SerializeStructVariant;
auto sv = ({ auto&& _m = serializer.serialize_struct_variant(std::move(rusty::to_string_view(n)), std::move(i), std::move(rusty::to_string_view(v)), rusty::len(fields)); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_ok()) { auto _mv = _m.unwrap();
auto&& val = _mv;
_match_value.emplace(std::move(val)); } else { if (!(_m.is_err())) { rusty::intrinsics::unreachable(); } auto _mv = _m.unwrap_err();
auto&& err = _mv;
return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err)); } std::move(_match_value).value(); });
for (auto&& _for_item : rusty::for_in(fields)) {
    auto&& k = std::get<0>(rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_for_item)));
    const auto& v = std::get<1>(rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_for_item)));
    {
        auto&& _m = sv.serialize_field(std::move(k), rusty::detail::deref_if_pointer_like(v));
        bool _m_matched = false;
        if (!_m_matched) {
            if (_m.is_ok()) {
                auto&& _mv0 = _m.unwrap();
                auto&& val = _mv0;
                val;
                _m_matched = true;
            }
        }
        if (!_m_matched) {
            if (_m.is_err()) {
                auto&& _mv1 = _m.unwrap_err();
                auto&& err = _mv1;
                return rusty::Result<typename S::Ok, typename S::Error>::Err(std::move(err));
                _m_matched = true;
            }
        }
    }
}
return sv.end(); }(); } }, _m); }();
            }
}

namespace private_::ser {
        template<typename S>
        auto AdjacentlyTaggedEnumVariant::serialize(S serializer) const {
            return serializer.serialize_unit_variant(std::string_view(this->enum_name), this->variant_index, std::string_view(this->variant_name));
        }
}



// ── Compile-validation runner ──
int main() {
    std::cout << "No transpiled test wrappers discovered; compile-validation only." << std::endl;
    return 0;
}
