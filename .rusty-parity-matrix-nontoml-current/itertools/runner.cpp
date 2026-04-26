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
namespace adaptors { namespace coalesce_tests { struct DedupEq; } }
namespace adaptors { namespace coalesce_tests { struct NoCount; } }
namespace adaptors { namespace coalesce_tests { struct WithCount; } }
namespace adaptors { namespace coalesce_tests { template<typename DP> struct DedupPred2CoalescePred; } }
namespace adaptors { namespace coalesce_tests { template<typename DP> struct DedupPredWithCount2CoalescePred; } }
namespace adaptors { namespace coalesce_tests { template<typename I, typename F, typename C> struct CoalesceBy; } }
namespace adaptors { namespace map { template<typename F> struct MapSpecialCaseFnOk; } }
namespace adaptors { namespace map { template<typename I, typename F> struct MapSpecialCase; } }
namespace adaptors { namespace map { template<typename U> struct MapSpecialCaseFnInto; } }
namespace adaptors { namespace multi_product { template<typename I> struct MultiProduct; } }
namespace adaptors { namespace multi_product { template<typename I> struct MultiProductInner; } }
namespace adaptors { namespace multi_product { template<typename I> struct MultiProductIter; } }
namespace adaptors { template<typename I, typename F> struct Batching; }
namespace adaptors { template<typename I, typename F> struct FilterMapOk; }
namespace adaptors { template<typename I, typename F> struct FilterOk; }
namespace adaptors { template<typename I, typename F> struct Positions; }
namespace adaptors { template<typename I, typename F> struct TakeWhileRef; }
namespace adaptors { template<typename I, typename F> struct Update; }
namespace adaptors { template<typename I, typename J> struct Interleave; }
namespace adaptors { template<typename I, typename J> struct InterleaveShortest; }
namespace adaptors { template<typename I, typename J> struct Product; }
namespace adaptors { template<typename I, typename T> struct TupleCombinations; }
namespace adaptors { template<typename I> struct PutBack; }
namespace adaptors { template<typename I> struct Tuple10Combination; }
namespace adaptors { template<typename I> struct Tuple11Combination; }
namespace adaptors { template<typename I> struct Tuple12Combination; }
namespace adaptors { template<typename I> struct Tuple1Combination; }
namespace adaptors { template<typename I> struct Tuple2Combination; }
namespace adaptors { template<typename I> struct Tuple3Combination; }
namespace adaptors { template<typename I> struct Tuple4Combination; }
namespace adaptors { template<typename I> struct Tuple5Combination; }
namespace adaptors { template<typename I> struct Tuple6Combination; }
namespace adaptors { template<typename I> struct Tuple7Combination; }
namespace adaptors { template<typename I> struct Tuple8Combination; }
namespace adaptors { template<typename I> struct Tuple9Combination; }
namespace adaptors { template<typename I> struct WhileSome; }
namespace combinations { template<typename I, typename Idx> struct CombinationsGeneric; }
namespace combinations_with_replacement { template<typename I> struct CombinationsWithReplacement; }
namespace cons_tuples_impl { struct ConsTuplesFn; }
namespace diff { template<typename I, typename J> struct Diff; }
namespace duplicates_impl { namespace private_ { struct ById; } }
namespace duplicates_impl { namespace private_ { template<typename F> struct ByFn; } }
namespace duplicates_impl { namespace private_ { template<typename I, typename Key, typename F> struct DuplicatesBy; } }
namespace duplicates_impl { namespace private_ { template<typename K, typename V> struct KeyValue; } }
namespace duplicates_impl { namespace private_ { template<typename Key, typename F> struct Meta; } }
namespace duplicates_impl { namespace private_ { template<typename V> struct JustValue; } }
namespace either_or_both { template<typename A, typename B> struct EitherOrBoth; }
namespace exactly_one_err { template<typename I> struct ExactlyOneError; }
namespace flatten_ok { template<typename I, typename T, typename E> struct FlattenOk; }
namespace format { template<typename I, typename F> struct FormatWith; }
namespace format { template<typename I> struct Format; }
namespace groupbylazy { struct ChunkIndex; }
namespace groupbylazy { template<typename I> struct Chunk; }
namespace groupbylazy { template<typename I> struct Chunks; }
namespace groupbylazy { template<typename I> struct IntoChunks; }
namespace groupbylazy { template<typename K, typename I, typename F> struct ChunkBy; }
namespace groupbylazy { template<typename K, typename I, typename F> struct Group; }
namespace groupbylazy { template<typename K, typename I, typename F> struct GroupInner; }
namespace groupbylazy { template<typename K, typename I, typename F> struct Groups; }
namespace grouping_map { template<typename F> struct GroupingMapFn; }
namespace grouping_map { template<typename I> struct GroupingMap; }
namespace intersperse_tests { template<typename I, typename ElemF> struct IntersperseWith; }
namespace intersperse_tests { template<typename Item> struct IntersperseElementSimple; }
namespace iterator { template<typename L, typename R> struct IterEither; }
namespace kmerge_impl { struct KMergeByLt; }
namespace kmerge_impl { template<typename I, typename F> struct KMergeBy; }
namespace kmerge_impl { template<typename I> struct HeadTail; }
namespace lazy_buffer { template<typename I> struct LazyBuffer; }
namespace merge_join { struct MergeLte; }
namespace merge_join { template<typename F, typename T> struct MergeFuncLR; }
namespace merge_join { template<typename I, typename J, typename F> struct MergeBy; }
namespace minmax { template<typename T> struct MinMaxResult; }
namespace multipeek_impl { template<typename I> struct MultiPeek; }
namespace next_array { template<typename T, size_t N> struct ArrayBuilder; }
namespace pad_tail { template<typename I, typename F> struct PadUsing; }
namespace peek_nth { template<typename I> struct PeekNth; }
namespace peeking_take_while { template<typename I, typename F> struct PeekingTakeWhile; }
namespace permutations { struct PermutationState; }
namespace permutations { template<typename I> struct Permutations; }
namespace powerset { template<typename I> struct Powerset; }
namespace process_results_impl { template<typename I, typename E> struct ProcessResults; }
namespace put_back_n_impl { template<typename I> struct PutBackN; }
namespace rciter_impl { template<typename I> struct RcIter; }
namespace repeatn { template<typename A> struct RepeatN; }
namespace sources { template<typename St, typename F> struct Iterate; }
namespace sources { template<typename St, typename F> struct Unfold; }
namespace take_while_inclusive { template<typename I, typename F> struct TakeWhileInclusive; }
namespace tee { template<typename A, typename I> struct TeeBuffer; }
namespace tee { template<typename I> struct Tee; }
namespace tuple_impl { template<typename I, typename T> struct CircularTupleWindows; }
namespace tuple_impl { template<typename I, typename T> struct TupleWindows; }
namespace tuple_impl { template<typename I, typename T> struct Tuples; }
namespace tuple_impl { template<typename T> struct TupleBuffer; }
namespace unique_impl { template<typename I, typename V, typename F> struct UniqueBy; }
namespace unique_impl { template<typename I> struct Unique; }
namespace with_position { enum class Position; }
namespace with_position { template<typename I> struct WithPosition; }
namespace zip_eq_impl { template<typename I, typename J> struct ZipEq; }
namespace zip_longest { template<typename T, typename U> struct ZipLongest; }
namespace ziptuple { template<typename T> struct Zip; }
template<typename L, typename R> struct Either;
template<typename T> struct FoldWhile;

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
void _unsized_ref_propagation();

// Extension trait free-function forward declarations
namespace rusty_ext {
}
namespace into_either {
}



namespace fmt = rusty::fmt;






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
        return [&]() -> rusty::fmt::Result { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.fmt(f); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.fmt(f); } return [&]() -> rusty::fmt::Result { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename T, typename A>
    void extend(T iter) {
        {
            auto&& _m = (*this);
            std::visit(overloaded {
                [&](::Either_Left<L, R>& _v) {
                    auto&& inner = rusty::detail::deref_if_pointer(_v._0);
                    inner.extend(std::move(iter));
                },
                [&](::Either_Right<L, R>& _v) {
                    auto&& inner = rusty::detail::deref_if_pointer(_v._0);
                    inner.extend(std::move(iter));
                },
            }, _m);
        }
    }
    // Rust-only dependent associated type alias skipped in constrained mode: Item
    auto next() {
        return [&]() { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.next(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.next(); } rusty::intrinsics::unreachable(); }();
    }
    std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
        return [&]() -> std::tuple<size_t, rusty::Option<size_t>> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::size_hint(inner); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::size_hint(inner); } return [&]() -> std::tuple<size_t, rusty::Option<size_t>> { rusty::intrinsics::unreachable(); }(); }();
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
        return [&]() { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.nth(std::move(n)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.nth(std::move(n)); } rusty::intrinsics::unreachable(); }();
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
        return [&]() -> bool { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.all(std::move(f)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.all(std::move(f)); } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename F>
    bool any(F f) {
        return [&]() -> bool { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.any(std::move(f)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.any(std::move(f)); } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename P>
    auto find(P predicate) {
        return [&]() { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::str_runtime::find(inner, std::move(predicate)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::str_runtime::find(inner, std::move(predicate)); } rusty::intrinsics::unreachable(); }();
    }
    template<typename B, typename F>
    rusty::Option<B> find_map(F f) {
        return [&]() -> rusty::Option<B> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.find_map(std::move(f)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.find_map(std::move(f)); } return [&]() -> rusty::Option<B> { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename P>
    rusty::Option<size_t> position(P predicate) {
        return [&]() -> rusty::Option<size_t> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.position(std::move(predicate)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.position(std::move(predicate)); } return [&]() -> rusty::Option<size_t> { rusty::intrinsics::unreachable(); }(); }();
    }
    auto next_back() {
        return [&]() { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.next_back(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.next_back(); } rusty::intrinsics::unreachable(); }();
    }
    auto nth_back(size_t n) {
        return [&]() { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.nth_back(std::move(n)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.nth_back(std::move(n)); } rusty::intrinsics::unreachable(); }();
    }
    template<typename Acc, typename G>
    Acc rfold(Acc init, G f) {
        return [&]() -> Acc { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.rfold(std::move(init), std::move(f)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.rfold(std::move(init), std::move(f)); } return [&]() -> Acc { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename P>
    auto rfind(P predicate) {
        return [&]() { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.rfind(std::move(predicate)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.rfind(std::move(predicate)); } rusty::intrinsics::unreachable(); }();
    }
    size_t len() const {
        return [&]() -> size_t { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::len(inner); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::len(inner); } return [&]() -> size_t { rusty::intrinsics::unreachable(); }(); }();
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
        return [&]() -> Either<const L&, const R&> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<const L&, const R&>(Either<const L&, const R&>{Either_Left<const L&, const R&>{inner}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<const L&, const R&>(Either<const L&, const R&>{Either_Right<const L&, const R&>{inner}}); } return [&]() -> Either<const L&, const R&> { rusty::intrinsics::unreachable(); }(); }();
    }
    Either<L&, R&> as_mut() {
        return [&]() -> Either<L&, R&> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<L&, R&>(Either<L&, R&>{Either_Left<L&, R&>{inner}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<L&, R&>(Either<L&, R&>{Either_Right<L&, R&>{inner}}); } return [&]() -> Either<L&, R&> { rusty::intrinsics::unreachable(); }(); }();
    }
    Either<rusty::pin::Pin<const L&>, rusty::pin::Pin<const R&>> as_pin_ref() {
        // @unsafe
        {
            return [&]() -> Either<rusty::pin::Pin<const L&>, rusty::pin::Pin<const R&>> { auto&& _m = rusty::pin::get_ref(std::move((*this))); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<rusty::pin::Pin<const L&>, rusty::pin::Pin<const R&>>(Either<rusty::pin::Pin<const L&>, rusty::pin::Pin<const R&>>{Either_Left<rusty::pin::Pin<const L&>, rusty::pin::Pin<const R&>>{rusty::pin::new_unchecked(inner)}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<rusty::pin::Pin<const L&>, rusty::pin::Pin<const R&>>(Either<rusty::pin::Pin<const L&>, rusty::pin::Pin<const R&>>{Either_Right<rusty::pin::Pin<const L&>, rusty::pin::Pin<const R&>>{rusty::pin::new_unchecked(inner)}}); } return [&]() -> Either<rusty::pin::Pin<const L&>, rusty::pin::Pin<const R&>> { rusty::intrinsics::unreachable(); }(); }();
        }
    }
    Either<rusty::pin::Pin<L&>, rusty::pin::Pin<R&>> as_pin_mut() {
        // @unsafe
        {
            return [&]() -> Either<rusty::pin::Pin<L&>, rusty::pin::Pin<R&>> { auto&& _m = rusty::pin::get_unchecked_mut(std::move((*this))); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<rusty::pin::Pin<L&>, rusty::pin::Pin<R&>>(Either<rusty::pin::Pin<L&>, rusty::pin::Pin<R&>>{Either_Left<rusty::pin::Pin<L&>, rusty::pin::Pin<R&>>{rusty::pin::new_unchecked(inner)}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<rusty::pin::Pin<L&>, rusty::pin::Pin<R&>>(Either<rusty::pin::Pin<L&>, rusty::pin::Pin<R&>>{Either_Right<rusty::pin::Pin<L&>, rusty::pin::Pin<R&>>{rusty::pin::new_unchecked(inner)}}); } return [&]() -> Either<rusty::pin::Pin<L&>, rusty::pin::Pin<R&>> { rusty::intrinsics::unreachable(); }(); }();
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
        return [&]() -> T { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::from_into<T>(inner); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::from_into<T>(inner); } return [&]() -> T { rusty::intrinsics::unreachable(); }(); }();
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
        return [&]() -> Either<L, R> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<L, R>(Either<L, R>{Either_Left<L, R>{rusty::clone(inner)}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<L, R>(Either<L, R>{Either_Right<L, R>{rusty::clone(inner)}}); } return [&]() -> Either<L, R> { rusty::intrinsics::unreachable(); }(); }();
    }
    Either<L, R> copied() {
        return [&]() -> Either<L, R> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<L, R>(Either<L, R>{Either_Left<L, R>{rusty::detail::deref_if_pointer_like(inner)}}); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return Either<L, R>(Either<L, R>{Either_Right<L, R>{rusty::detail::deref_if_pointer_like(inner)}}); } return [&]() -> Either<L, R> { rusty::intrinsics::unreachable(); }(); }();
    }
    static Either<L, R> from(rusty::Result<R, L> r) {
        return [&]() -> Either<L, R> { auto&& _m = r; if (_m.is_err()) { auto&& _mv0 = _m.unwrap_err(); auto&& e = rusty::detail::deref_if_pointer(_mv0); return Either<L, R>(Either<L, R>{Either_Left<L, R>{std::move(e)}}); } if (_m.is_ok()) { auto&& _mv1 = _m.unwrap(); auto&& o = rusty::detail::deref_if_pointer(_mv1); return Either<L, R>(Either<L, R>{Either_Right<L, R>{std::move(o)}}); } return [&]() -> Either<L, R> { rusty::intrinsics::unreachable(); }(); }();
    }
    // Rust-only dependent associated type alias skipped in constrained mode: Output
    auto poll(rusty::Context& cx) {
        return [&]() { auto&& _m = this->as_pin_mut(); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.poll(cx); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.poll(cx); } rusty::intrinsics::unreachable(); }();
    }
    template<typename Target>
    const Target& as_ref() const {
        return [&]() -> const Target& { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.as_ref(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.as_ref(); } return [&]() -> const Target& { rusty::intrinsics::unreachable(); }(); }();
    }
    template<typename Target>
    Target& as_mut() {
        return [&]() -> Target& { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.as_mut(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.as_mut(); } return [&]() -> Target& { rusty::intrinsics::unreachable(); }(); }();
    }
    // Rust-only dependent associated type alias skipped in constrained mode: Target
    decltype(auto) operator*() const {
        return [&]() { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::deref_ref(rusty::deref_ref(inner)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::deref_ref(rusty::deref_ref(inner)); } rusty::intrinsics::unreachable(); }();
    }
    decltype(auto) operator*() {
        return [&]() { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::deref_ref(inner); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::deref_ref(inner); } rusty::intrinsics::unreachable(); }();
    }
    rusty::fmt::Result write_str(std::string_view s) {
        return [&]() -> rusty::fmt::Result { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.write_str(std::string_view(s)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.write_str(std::string_view(s)); } return [&]() -> rusty::fmt::Result { rusty::intrinsics::unreachable(); }(); }();
    }
    rusty::fmt::Result write_char(char32_t c) {
        return [&]() -> rusty::fmt::Result { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.write_char(std::move(c)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return inner.write_char(std::move(c)); } return [&]() -> rusty::fmt::Result { rusty::intrinsics::unreachable(); }(); }();
    }
    rusty::fmt::Result write_fmt(rusty::fmt::Arguments args) {
        return [&]() -> rusty::fmt::Result { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::write_fmt(inner, std::move(args)); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::write_fmt(inner, std::move(args)); } return [&]() -> rusty::fmt::Result { rusty::intrinsics::unreachable(); }(); }();
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
            return [&]() -> std::tuple<size_t, rusty::Option<size_t>> { auto&& _m = this->inner; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::size_hint(inner); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& inner = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return rusty::size_hint(inner); } return [&]() -> std::tuple<size_t, rusty::Option<size_t>> { rusty::intrinsics::unreachable(); }(); }();
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

// Extension trait From lowered to rusty_ext:: free functions
namespace rusty_ext {
    // Rust-only extension method skipped (no receiver): from

}



// ── from itertools.cppm ──


namespace free_mod {}
namespace intersperse_tests {}

namespace rusty_module_aliases {
namespace free = free_mod;
namespace intersperse = intersperse_tests;
} // namespace rusty_module_aliases
using namespace rusty_module_aliases;

template<typename T>
struct FoldWhile;
namespace duplicates_impl {
    namespace private_ {
        template<typename Key, typename F>
        struct Meta;
        template<typename I, typename Key, typename F>
        struct DuplicatesBy;
        struct ById;
        template<typename F>
        struct ByFn;
        template<typename K, typename V>
        struct KeyValue;
        template<typename V>
        struct JustValue;
    }
    template<typename I, typename V, typename F>
    using DuplicatesBy = ::duplicates_impl::private_::DuplicatesBy<I, V, ::duplicates_impl::private_::ByFn<F>>;
    template<typename I>
    using Duplicates = ::duplicates_impl::private_::DuplicatesBy<I, rusty::detail::associated_item_t<I>, ::duplicates_impl::private_::ById>;
    template<typename I, typename Key, typename F>
    DuplicatesBy<I, Key, F> duplicates_by(I iter, F f);
    template<typename I>
    Duplicates<I> duplicates(I iter);
}
namespace groupbylazy {
    struct ChunkIndex;
    template<typename K, typename I, typename F>
    struct GroupInner;
    template<typename K, typename I, typename F>
    struct ChunkBy;
    template<typename K, typename I, typename F>
    struct Groups;
    template<typename K, typename I, typename F>
    struct Group;
    template<typename I>
    struct IntoChunks;
    template<typename I>
    struct Chunks;
    template<typename I>
    struct Chunk;
    template<typename K, typename I, typename F>
    using GroupBy = ChunkBy<K, I, F>;
    template<typename K, typename J, typename F>
    ChunkBy<K, typename J::IntoIter, F> new_(J iter, F f);
    template<typename J>
    IntoChunks<typename J::IntoIter> new_chunks(J iter, size_t size);
}
namespace either_or_both {
    template<typename A, typename B>
    struct EitherOrBoth;
}
namespace format {
    template<typename I, typename F>
    struct FormatWith;
    template<typename I>
    struct Format;
    template<typename I, typename F>
    FormatWith<I, F> new_format(I iter, std::string_view separator, F f);
    template<typename I>
    Format<I> new_format_default(I iter, std::string_view separator);
}
namespace minmax {
    template<typename T>
    struct MinMaxResult;
    template<typename I, typename K, typename F, typename L>
    MinMaxResult<rusty::detail::associated_item_t<I>> minmax_impl(I it, F key_for, L lt);
}
namespace next_array {
    template<typename T, size_t N>
    struct ArrayBuilder;
    namespace test {
        using ::next_array::ArrayBuilder;
        void zero_len_take();
        void zero_len_push();
        void push_4();
        void tracked_drop();
    }
    template<typename T>
    std::span<T> slice_assume_init_mut(std::span<rusty::MaybeUninit<T>> slice);
    template<typename I, size_t N>
    rusty::Option<std::array<rusty::detail::associated_item_t<I>, rusty::sanitize_array_capacity<N>()>> next_array(I& it);
}
namespace peeking_take_while {
    template<typename I, typename F>
    struct PeekingTakeWhile;
    template<typename I, typename F>
    PeekingTakeWhile<I, F> peeking_take_while(I& iter, F f);
}
namespace process_results_impl {
    template<typename I, typename E>
    struct ProcessResults;
    template<typename I, typename F, typename T, typename E, typename R>
    rusty::Result<R, E> process_results(I iterable, F processor);
}
namespace rciter_impl {
    template<typename I>
    struct RcIter;
    template<typename I>
    RcIter<typename I::IntoIter> rciter(I iterable);
}
namespace repeatn {
    template<typename A>
    struct RepeatN;
    template<typename A>
    RepeatN<A> repeat_n(A element, size_t n);
}
namespace sources {
    template<typename St, typename F>
    struct Unfold;
    template<typename St, typename F>
    struct Iterate;
    template<typename A, typename St, typename F>
    Unfold<St, F> unfold(St initial_state, F f);
    template<typename St, typename F>
    Iterate<St, F> iterate(St initial_value, F f);
}
namespace take_while_inclusive {
    template<typename I, typename F>
    struct TakeWhileInclusive;
}
namespace unique_impl {
    template<typename I, typename V, typename F>
    struct UniqueBy;
    template<typename I>
    struct Unique;
    template<typename I, typename V, typename F>
    UniqueBy<I, V, F> unique_by(I iter, F f);
    template<typename I, typename K>
    size_t count_new_keys(rusty::HashMap<K, std::tuple<>> used, I iterable);
    template<typename I>
    Unique<I> unique(I iter);
}
namespace with_position {
    enum class Position;
    constexpr Position Position_First();
    constexpr Position Position_Middle();
    constexpr Position Position_Last();
    constexpr Position Position_Only();
    Position clone(const Position& self_);
    bool eq(const Position& self_, const Position& other);
    void assert_receiver_is_total_eq(const Position& self_);
    template<typename I>
    struct WithPosition;
    template<typename I>
    WithPosition<I> with_position(I iter);
}
namespace size_hint {
    using SizeHint = std::tuple<size_t, rusty::Option<size_t>>;
    SizeHint add(const auto& a, const auto& b);
    SizeHint add_scalar(const auto& sh, size_t x);
    SizeHint sub_scalar(const auto& sh, size_t x);
    SizeHint mul(const auto& a, const auto& b);
    SizeHint mul_scalar(const auto& sh, size_t x);
    SizeHint max(const auto& a, const auto& b);
    SizeHint min(const auto& a, const auto& b);
    void mul_size_hints();
}
namespace impl_macros {
}
namespace concat_impl {
    template<typename I>
    rusty::detail::associated_item_t<I> concat(I iterable);
}
namespace extrema_set {
    template<typename I, typename K, typename F, typename Compare>
    rusty::Vec<rusty::detail::associated_item_t<I>> min_set_impl(I it, F key_for, Compare compare);
    template<typename I, typename K, typename F, typename Compare>
    rusty::Vec<rusty::detail::associated_item_t<I>> max_set_impl(I it, F key_for, Compare compare);
}
namespace group_map {
    template<typename I, typename K, typename V>
    rusty::HashMap<K, rusty::Vec<V>> into_group_map(I iter);
    template<typename I, typename K, typename V, typename F>
    rusty::HashMap<K, rusty::Vec<V>> into_group_map_by(I iter, F f);
}
namespace iter_index {
    namespace private_iter_index {
    }
    template<typename I, typename R>
    typename R::Output get(I iter, R index);
}
namespace k_smallest {
    template<typename I, typename F>
    rusty::Vec<rusty::detail::associated_item_t<I>> k_smallest_general(I iter, size_t k, F comparator);
    template<typename I, typename F>
    rusty::Vec<rusty::detail::associated_item_t<I>> k_smallest_relaxed_general(I iter, size_t k, F comparator);
}
namespace unziptuple {
    template<typename I>
    auto multiunzip(I i);
}
namespace adaptors {
    template<typename I, typename J>
    struct Interleave;
    template<typename I, typename J>
    struct InterleaveShortest;
    template<typename I>
    struct PutBack;
    template<typename I, typename J>
    struct Product;
    template<typename I, typename F>
    struct Batching;
    template<typename I, typename F>
    struct TakeWhileRef;
    template<typename I>
    struct WhileSome;
    template<typename I, typename T>
    struct TupleCombinations;
    template<typename I>
    struct Tuple1Combination;
    template<typename I>
    struct Tuple2Combination;
    template<typename I>
    struct Tuple3Combination;
    template<typename I>
    struct Tuple4Combination;
    template<typename I>
    struct Tuple5Combination;
    template<typename I>
    struct Tuple6Combination;
    template<typename I>
    struct Tuple7Combination;
    template<typename I>
    struct Tuple8Combination;
    template<typename I>
    struct Tuple9Combination;
    template<typename I>
    struct Tuple10Combination;
    template<typename I>
    struct Tuple11Combination;
    template<typename I>
    struct Tuple12Combination;
    template<typename I, typename F>
    struct FilterOk;
    template<typename I, typename F>
    struct FilterMapOk;
    template<typename I, typename F>
    struct Positions;
    template<typename I, typename F>
    struct Update;
    namespace coalesce_tests {
        template<typename I, typename F, typename C>
        struct CoalesceBy;
        struct NoCount;
        struct WithCount;
        template<typename DP>
        struct DedupPred2CoalescePred;
        struct DedupEq;
        template<typename DP>
        struct DedupPredWithCount2CoalescePred;
        template<typename I, typename F>
        using Coalesce = CoalesceBy<I, F, NoCount>;
        template<typename I, typename Pred>
        using DedupBy = CoalesceBy<I, DedupPred2CoalescePred<Pred>, NoCount>;
        template<typename I>
        using Dedup = DedupBy<I, DedupEq>;
        template<typename I, typename Pred>
        using DedupByWithCount = CoalesceBy<I, DedupPredWithCount2CoalescePred<Pred>, WithCount>;
        template<typename I>
        using DedupWithCount = DedupByWithCount<I, DedupEq>;
        template<typename I, typename F>
        Coalesce<I, F> coalesce(I iter, F f);
        template<typename I, typename Pred>
        DedupBy<I, Pred> dedup_by(I iter, Pred dedup_pred);
        template<typename I>
        Dedup<I> dedup(I iter);
        template<typename I, typename Pred>
        DedupByWithCount<I, Pred> dedup_by_with_count(I iter, Pred dedup_pred);
        template<typename I>
        DedupWithCount<I> dedup_with_count(I iter);
    }
    namespace map {
        template<typename I, typename F>
        struct MapSpecialCase;
        template<typename F>
        struct MapSpecialCaseFnOk;
        template<typename U>
        struct MapSpecialCaseFnInto;
        template<typename I, typename F>
        using MapOk = MapSpecialCase<I, MapSpecialCaseFnOk<F>>;
        template<typename I, typename R>
        using MapInto = MapSpecialCase<I, MapSpecialCaseFnInto<R>>;
        template<typename I, typename F, typename T, typename U, typename E>
        MapOk<I, F> map_ok(I iter, F f);
        template<typename I, typename R>
        MapInto<I, R> map_into(I iter);
    }
    namespace multi_product {
        template<typename I>
        struct MultiProductIter;
        template<typename I>
        struct MultiProductInner;
        template<typename I>
        struct MultiProduct;
    }
    template<typename I, typename J>
    Interleave<typename I::IntoIter, typename J::IntoIter> interleave(I i, J j);
    template<typename I, typename J>
    InterleaveShortest<I, J> interleave_shortest(I i, J j);
    template<typename I>
    PutBack<typename I::IntoIter> put_back(I iterable);
    template<typename I, typename J>
    Product<I, J> cartesian_product(I i, J j);
    template<typename I, typename F>
    Batching<I, F> batching(I iter, F f);
    template<typename I, typename F>
    TakeWhileRef<I, F> take_while_ref(I& iter, F f);
    template<typename I>
    WhileSome<I> while_some(I iter);
    template<typename T, typename I>
    TupleCombinations<I, T> tuple_combinations(I iter);
    rusty::Option<size_t> checked_binomial(size_t n, size_t k);
    void test_checked_binomial();
    template<typename I, typename F, typename T, typename E>
    FilterOk<I, F> filter_ok(I iter, F f);
    template<typename T, typename E>
    rusty::Option<rusty::Result<T, E>> transpose_result(rusty::Result<rusty::Option<T>, E> result);
    template<typename I, typename F, typename T, typename U, typename E>
    FilterMapOk<I, F> filter_map_ok(I iter, F f);
    template<typename I, typename F>
    Positions<I, F> positions(I iter, F f);
    template<typename I, typename F>
    Update<I, F> update(I iter, F f);
}
namespace intersperse_tests {
    template<typename Item>
    struct IntersperseElementSimple;
    template<typename I, typename ElemF>
    struct IntersperseWith;
    template<typename I>
    using Intersperse = decltype(std::declval<I>().intersperse_with(std::declval<IntersperseElementSimple<rusty::detail::associated_item_t<I>>>()));
    template<typename I>
    decltype(std::declval<I>().intersperse(std::declval<typename I::Item>())) intersperse(I iter, rusty::detail::associated_item_t<I> elt);
    template<typename I, typename ElemF>
    decltype(std::declval<I>().intersperse_with(std::declval<ElemF>())) intersperse_with(I iter, ElemF elt);
}
namespace kmerge_impl {
    template<typename I>
    struct HeadTail;
    struct KMergeByLt;
    template<typename I, typename F>
    struct KMergeBy;
    template<typename I>
    using KMerge = KMergeBy<I, KMergeByLt>;
    template<typename T, typename S>
    void heapify(std::span<T> data, S less_than);
    template<typename T, typename S>
    void sift_down(std::span<T> heap, size_t index, S less_than);
}
namespace exactly_one_err {
    template<typename I>
    struct ExactlyOneError;
}
namespace flatten_ok {
    template<typename I, typename T, typename E>
    struct FlattenOk;
    template<typename I, typename T, typename E>
    FlattenOk<I, T, E> flatten_ok(I iter);
}
namespace lazy_buffer {
    template<typename I>
    struct LazyBuffer;
}
namespace multipeek_impl {
    template<typename I>
    struct MultiPeek;
    template<typename I>
    MultiPeek<typename I::IntoIter> multipeek(I iterable);
}
namespace pad_tail {
    template<typename I, typename F>
    struct PadUsing;
    template<typename I, typename F>
    PadUsing<I, F> pad_using(I iter, size_t min, F filler);
}
namespace peek_nth {
    template<typename I>
    struct PeekNth;
    template<typename I>
    PeekNth<typename I::IntoIter> peek_nth(I iterable);
}
namespace put_back_n_impl {
    template<typename I>
    struct PutBackN;
    template<typename I>
    PutBackN<typename I::IntoIter> put_back_n(I iterable);
}
namespace tee {
    template<typename A, typename I>
    struct TeeBuffer;
    template<typename I>
    struct Tee;
    template<typename I>
    std::tuple<Tee<I>, Tee<I>> new_(I iter);
}
namespace tuple_impl {
    template<typename T>
    struct TupleBuffer;
    template<typename I, typename T>
    struct Tuples;
    template<typename I, typename T>
    struct TupleWindows;
    template<typename I, typename T>
    struct CircularTupleWindows;
    template<typename I, typename T>
    Tuples<I, T> tuples(I iter);
    rusty::Option<size_t> add_then_div(size_t n, size_t a, size_t d);
    template<typename I, typename T>
    TupleWindows<I, T> tuple_windows(I iter);
    template<typename I, typename T>
    CircularTupleWindows<I, T> circular_tuple_windows(I iter);
}
namespace zip_eq_impl {
    template<typename I, typename J>
    struct ZipEq;
    template<typename I, typename J>
    ZipEq<typename I::IntoIter, typename J::IntoIter> zip_eq(I i, J j);
}
namespace zip_longest {
    template<typename T, typename U>
    struct ZipLongest;
    using either_or_both::EitherOrBoth;
    template<typename T, typename U>
    ZipLongest<T, U> zip_longest(T a, U b);
}
namespace ziptuple {
    template<typename T>
    struct Zip;
    template<typename T, typename U>
    Zip<T> multizip(U t);
}
namespace cons_tuples_impl {
    struct ConsTuplesFn;
    template<typename I>
    using ConsTuples = ::adaptors::map::MapSpecialCase<I, ConsTuplesFn>;
    template<typename I>
    ConsTuples<typename I::IntoIter> cons_tuples(I iterable);
}
namespace grouping_map {
    template<typename F>
    struct GroupingMapFn;
    template<typename I>
    struct GroupingMap;
    using minmax::MinMaxResult;
    template<typename I, typename F>
    using MapForGrouping = ::adaptors::map::MapSpecialCase<I, GroupingMapFn<F>>;
    template<typename I, typename F>
    using GroupingMapBy = GroupingMap<MapForGrouping<I, F>>;
    template<typename K, typename I, typename F>
    MapForGrouping<I, F> new_map_for_grouping(I iter, F key_mapper);
    template<typename I, typename K, typename V>
    GroupingMap<I> new_(I iter);
}
namespace combinations {
    template<typename I, typename Idx>
    struct CombinationsGeneric;
    using lazy_buffer::LazyBuffer;
    template<typename I>
    using Combinations = CombinationsGeneric<I, rusty::Vec<size_t>>;
    template<typename I>
    void __rusty_alias_Combinations_reset(auto& self_, size_t k);
    template<typename I, size_t K>
    using ArrayCombinations = CombinationsGeneric<I, std::array<size_t, rusty::sanitize_array_capacity<K>()>>;
    template<typename I>
    Combinations<I> combinations(I iter, size_t k);
    template<typename I, size_t K>
    ArrayCombinations<I, K> array_combinations(I iter);
    rusty::Option<size_t> remaining_for(size_t n, bool first, std::span<const size_t> indices);
}
namespace merge_join {
    struct MergeLte;
    template<typename I, typename J, typename F>
    struct MergeBy;
    template<typename F, typename T>
    struct MergeFuncLR;
    using adaptors::PutBack;
    using either_or_both::EitherOrBoth;
    template<typename I, typename J>
    using Merge = MergeBy<I, J, MergeLte>;
    template<typename I, typename J, typename F>
    using MergeJoinBy = MergeBy<I, J, MergeFuncLR<F, typename F::T>>;
    template<typename I, typename J>
    Merge<typename I::IntoIter, typename J::IntoIter> merge(I i, J j);
    template<typename I, typename J, typename F>
    MergeBy<typename I::IntoIter, typename J::IntoIter, F> merge_by_new(I a, J b, F cmp);
    template<typename I, typename J, typename F, typename T>
    MergeJoinBy<typename I::IntoIter, typename J::IntoIter, F> merge_join_by(I left, J right, F cmp_fn);
}
namespace combinations_with_replacement {
    template<typename I>
    struct CombinationsWithReplacement;
    using lazy_buffer::LazyBuffer;
    template<typename I>
    CombinationsWithReplacement<I> combinations_with_replacement(I iter, size_t k);
    rusty::Option<size_t> remaining_for(size_t n, bool first, std::span<const size_t> indices);
}
namespace permutations {
    struct PermutationState;
    template<typename I>
    struct Permutations;
    using lazy_buffer::LazyBuffer;
    template<typename I>
    Permutations<I> permutations(I iter, size_t k);
    bool advance(std::span<size_t> indices, std::span<size_t> cycles);
}
namespace traits {
}
namespace powerset {
    template<typename I>
    struct Powerset;
    template<typename I>
    Powerset<I> powerset(I src);
    rusty::Option<size_t> remaining_for(size_t n, size_t k);
}
namespace free_mod {
    template<typename T>
    using VecIntoIter = decltype(rusty::iter(std::declval<rusty::Vec<T>>()));
    template<typename I>
    decltype(std::declval<typename I::IntoIter>().intersperse(std::declval<typename I::IntoIter::Item>())) intersperse(I iterable, rusty::detail::associated_item_t<I> element);
    template<typename I, typename F>
    decltype(std::declval<typename I::IntoIter>().intersperse_with(std::declval<F>())) intersperse_with(I iterable, F element);
    template<typename I>
    decltype(std::declval<typename I::IntoIter>().enumerate()) enumerate(I iterable);
    template<typename I>
    decltype(std::declval<typename I::IntoIter>().rev()) rev(I iterable);
    template<typename I, typename J>
    decltype(rusty::zip(std::declval<typename I::IntoIter>(), std::declval<typename J::IntoIter>())) zip(I i, J j);
    template<typename I, typename J>
    decltype(std::declval<typename I::IntoIter>().chain(std::declval<typename J::IntoIter>())) chain(I i, J j);
    template<typename I, typename T>
    decltype(std::declval<typename I::IntoIter>().cloned()) cloned(I iterable);
    template<typename I, typename B, typename F>
    B fold(I iterable, B init, F f);
    template<typename I, typename F>
    bool all(I iterable, F f);
    template<typename I, typename F>
    bool any(I iterable, F f);
    template<typename I>
    rusty::Option<rusty::detail::associated_item_t<I>> max(I iterable);
    template<typename I>
    rusty::Option<rusty::detail::associated_item_t<I>> min(I iterable);
    template<typename I>
    rusty::String join(I iterable, std::string_view sep);
    template<typename I>
    VecIntoIter<rusty::detail::associated_item_t<I>> sorted(I iterable);
    template<typename I>
    VecIntoIter<rusty::detail::associated_item_t<I>> sorted_unstable(I iterable);
}
namespace structs {
    using adaptors::Batching;
    using adaptors::FilterMapOk;
    using adaptors::FilterOk;
    using adaptors::Interleave;
    using adaptors::InterleaveShortest;
    using adaptors::Positions;
    using adaptors::Product;
    using adaptors::PutBack;
    using adaptors::TakeWhileRef;
    using adaptors::TupleCombinations;
    using adaptors::Update;
    using adaptors::WhileSome;
    using combinations_with_replacement::CombinationsWithReplacement;
    using exactly_one_err::ExactlyOneError;
    using flatten_ok::FlattenOk;
    using format::Format;
    using format::FormatWith;
    using groupbylazy::Chunk;
    using groupbylazy::ChunkBy;
    using groupbylazy::Chunks;
    using groupbylazy::Group;
    using groupbylazy::Groups;
    using groupbylazy::IntoChunks;
    using grouping_map::GroupingMap;
    using kmerge_impl::KMergeBy;
    using merge_join::MergeBy;
    using multipeek_impl::MultiPeek;
    using pad_tail::PadUsing;
    using peek_nth::PeekNth;
    using peeking_take_while::PeekingTakeWhile;
    using permutations::Permutations;
    using powerset::Powerset;
    using process_results_impl::ProcessResults;
    using put_back_n_impl::PutBackN;
    using rciter_impl::RcIter;
    using repeatn::RepeatN;
    using sources::Iterate;
    using sources::Unfold;
    using take_while_inclusive::TakeWhileInclusive;
    using tee::Tee;
    using tuple_impl::CircularTupleWindows;
    using tuple_impl::TupleBuffer;
    using tuple_impl::TupleWindows;
    using tuple_impl::Tuples;
    using unique_impl::Unique;
    using unique_impl::UniqueBy;
    using with_position::WithPosition;
    using zip_eq_impl::ZipEq;
    using zip_longest::ZipLongest;
    using ziptuple::Zip;
}
namespace diff {
    template<typename I, typename J>
    struct Diff;
    template<typename I, typename J, typename F>
    rusty::Option<Diff<typename I::IntoIter, typename J::IntoIter>> diff_with(I i, J j, F is_equal);
}
using diff::Diff;
using minmax::MinMaxResult;
using with_position::Position;
using either_or_both::EitherOrBoth;
template<typename T>
using VecDequeIntoIter = decltype(rusty::iter(std::declval<rusty::VecDeque<T>>()));
template<typename T>
using VecIntoIter = decltype(rusty::iter(std::declval<rusty::Vec<T>>()));
template<typename I, typename J>
bool equal(I a, J b);
template<typename I, typename J>
void assert_equal(I a, J b);
template<typename A, typename I, typename F>
size_t partition(I iter, F pred);

namespace adaptors {
    namespace coalesce_tests {
        // Extension trait free-function forward declarations
        namespace rusty_ext {
            template<typename F, typename Item, typename T>
            rusty::Result<T, std::tuple<T, T>> coalesce_pair(F& self_, T t, Item item);

            template<typename T, typename F>
            bool dedup_pair(F& self_, const T& a, const T& b);

        }

    }
}

namespace combinations {
    // Extension trait free-function forward declarations
    namespace rusty_ext {
        template<typename I, typename T>
        rusty::Vec<T> extract_item(const rusty::Vec<size_t>& self_, const ::lazy_buffer::LazyBuffer<I>& pool);

    }

}

namespace either_or_both {
    // Extension trait free-function forward declarations
    namespace rusty_ext {
    }
}

namespace groupbylazy {
    // Extension trait free-function forward declarations
    namespace rusty_ext {
        template<typename A, typename K, typename F>
        K call_mut(F& self_, A arg);

    }

}

namespace intersperse_tests {
    // Extension trait free-function forward declarations
    namespace rusty_ext {
        template<typename Item, typename F>
        Item generate(F& self_);

    }

}

namespace iter_index {
    // Extension trait free-function forward declarations
    namespace rusty_ext {
        template<typename I>
        decltype(rusty::skip(std::declval<decltype(rusty::take(std::declval<I>(), std::declval<size_t>()))>(), std::declval<size_t>())) index(rusty::range<size_t> self_, I iter);

        template<typename I>
        decltype(rusty::take(std::declval<decltype(rusty::skip(std::declval<I>(), std::declval<size_t>()))>(), std::declval<size_t>())) index(rusty::range_inclusive<size_t> self_, I iter);

        template<typename I>
        decltype(rusty::take(std::declval<I>(), std::declval<size_t>())) index(rusty::range_to<size_t> self_, I iter);

        template<typename I>
        decltype(rusty::take(std::declval<I>(), std::declval<size_t>())) index(rusty::range_to_inclusive<size_t> self_, I iter);

        template<typename I>
        decltype(rusty::skip(std::declval<I>(), std::declval<size_t>())) index(rusty::range_from<size_t> self_, I iter);

        template<typename I>
        I index(rusty::range_full self_, I iter);

    }

}

namespace iter_index {
    namespace private_iter_index {
    }
}

namespace kmerge_impl {
    // Extension trait free-function forward declarations
    namespace rusty_ext {
        template<typename T, typename F>
        bool kmerge_pred(F& self_, const T& a, const T& b);

    }

}

namespace merge_join {
    // Extension trait free-function forward declarations
    namespace rusty_ext {
        template<typename T, typename F>
        std::tuple<rusty::Option<rusty::Either<T, T>>, T> merge(F& self_, T left, T right);

    }

}

namespace peeking_take_while {
    // Extension trait free-function forward declarations
    namespace rusty_ext {
        template<typename F, typename I>
        rusty::Option<typename decltype(std::declval<I>().peekable())::Item> peeking_next(decltype(std::declval<I>().peekable())& self_, F accept);

        template<typename F, typename T>
        rusty::Option<typename rusty::slice_iter::Iter<const T>::Item> peeking_next(rusty::slice_iter::Iter<const T>& self_, F accept);

        template<typename F, typename T>
        rusty::Option<typename rusty::empty_iter<T>::Item> peeking_next(rusty::empty_iter<T>& self_, F accept);

        template<typename F, typename I>
        rusty::Option<typename decltype(std::declval<I>().rev())::Item> peeking_next(decltype(std::declval<I>().rev())& self_, F accept);

    }

}

namespace tuple_impl {
}

namespace unziptuple {
    // Extension trait free-function forward declarations
    namespace rusty_ext {
        template<typename IT>
        std::tuple<> multiunzip(IT self_);

        template<typename IT, typename FromA>
        std::tuple<FromA> multiunzip(IT self_);

        template<typename IT, typename FromA, typename FromB>
        std::tuple<FromA, FromB> multiunzip(IT self_);

        template<typename IT, typename FromA, typename FromB, typename FromC>
        std::tuple<FromA, FromB, FromC> multiunzip(IT self_);

        template<typename IT, typename FromA, typename FromB, typename FromC, typename FromD>
        std::tuple<FromA, FromB, FromC, FromD> multiunzip(IT self_);

        template<typename IT, typename FromA, typename FromB, typename FromC, typename FromD, typename FromE>
        std::tuple<FromA, FromB, FromC, FromD, FromE> multiunzip(IT self_);

        template<typename IT, typename FromA, typename FromB, typename FromC, typename FromD, typename FromE, typename FromF>
        std::tuple<FromA, FromB, FromC, FromD, FromE, FromF> multiunzip(IT self_);

        template<typename IT, typename FromA, typename FromB, typename FromC, typename FromD, typename FromE, typename FromF, typename FromG>
        std::tuple<FromA, FromB, FromC, FromD, FromE, FromF, FromG> multiunzip(IT self_);

        template<typename IT, typename FromA, typename FromB, typename FromC, typename FromD, typename FromE, typename FromF, typename FromG, typename FromH>
        std::tuple<FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH> multiunzip(IT self_);

        template<typename IT, typename FromA, typename FromB, typename FromC, typename FromD, typename FromE, typename FromF, typename FromG, typename FromH, typename FromI>
        std::tuple<FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH, FromI> multiunzip(IT self_);

        template<typename IT, typename FromA, typename FromB, typename FromC, typename FromD, typename FromE, typename FromF, typename FromG, typename FromH, typename FromI, typename FromJ>
        std::tuple<FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH, FromI, FromJ> multiunzip(IT self_);

        template<typename IT, typename FromA, typename FromB, typename FromC, typename FromD, typename FromE, typename FromF, typename FromG, typename FromH, typename FromI, typename FromJ, typename FromK>
        std::tuple<FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH, FromI, FromJ, FromK> multiunzip(IT self_);

        template<typename IT, typename FromA, typename FromB, typename FromC, typename FromD, typename FromE, typename FromF, typename FromG, typename FromH, typename FromI, typename FromJ, typename FromK, typename FromL>
        std::tuple<FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH, FromI, FromJ, FromK, FromL> multiunzip(IT self_);

    }

}



using ::rusty::VecDeque;
using ::rusty::String;
using ::rusty::Vec;

// Rust-only unresolved import: using ::Either;


using ::rusty::cmp::Ordering;

using ::rusty::HashMap;

using ::rusty::HashSet;

namespace fmt = rusty::fmt;







namespace impl_macros {
}


namespace either_or_both {

    template<typename A, typename B>
    struct EitherOrBoth;


    // Rust-only namespace import skipped for type path: using namespace either_or_both::EitherOrBoth;

    // Rust-only unresolved import: using ::Either;

    // Algebraic data type
    template<typename A, typename B>
    struct EitherOrBoth_Both {
        A _0;
        B _1;
    };
    template<typename A, typename B>
    struct EitherOrBoth_Left {
        A _0;
    };
    template<typename A, typename B>
    struct EitherOrBoth_Right {
        B _0;
    };
    template<typename A, typename B>
    EitherOrBoth_Both<A, B> Both(A _0, B _1);
    template<typename A, typename B>
    EitherOrBoth_Left<A, B> Left(A _0);
    template<typename A, typename B>
    EitherOrBoth_Right<A, B> Right(B _0);
    template<typename A, typename B>
    struct EitherOrBoth : std::variant<EitherOrBoth_Both<A, B>, EitherOrBoth_Left<A, B>, EitherOrBoth_Right<A, B>> {
        using variant = std::variant<EitherOrBoth_Both<A, B>, EitherOrBoth_Left<A, B>, EitherOrBoth_Right<A, B>>;
        using variant::variant;
        static EitherOrBoth<A, B> Both(A _0, B _1) { return EitherOrBoth<A, B>{EitherOrBoth_Both<A, B>{std::forward<decltype(_0)>(_0), std::forward<decltype(_1)>(_1)}}; }
        static EitherOrBoth<A, B> Left(A _0) { return EitherOrBoth<A, B>{EitherOrBoth_Left<A, B>{std::forward<decltype(_0)>(_0)}}; }
        static EitherOrBoth<A, B> Right(B _0) { return EitherOrBoth<A, B>{EitherOrBoth_Right<A, B>{std::forward<decltype(_0)>(_0)}}; }


        EitherOrBoth<A, B> clone() const {
            return [&]() -> EitherOrBoth<A, B> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& __self_1 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return EitherOrBoth<A, B>{EitherOrBoth_Both<A, B>{rusty::clone(__self_0), rusty::clone(__self_1)}}; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return EitherOrBoth<A, B>(EitherOrBoth<A, B>{EitherOrBoth_Left<A, B>{rusty::clone(__self_0)}}); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return EitherOrBoth<A, B>(EitherOrBoth<A, B>{EitherOrBoth_Right<A, B>{rusty::clone(__self_0)}}); } return [&]() -> EitherOrBoth<A, B> { rusty::intrinsics::unreachable(); }(); }();
        }
        bool operator==(const EitherOrBoth<A, B>& other) const {
            const auto __self_discr = rusty::intrinsics::discriminant_value((*this));
            const auto __arg1_discr = rusty::intrinsics::discriminant_value(other);
            return (__self_discr == __arg1_discr) && [&]() -> bool { auto&& _m0 = (*this); auto&& _m1 = other; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0))._0); auto&& __self_1 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0))._1); auto&& __arg1_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))._0); auto&& __arg1_1 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))._1); return (__self_0 == __arg1_0) && (__self_1 == __arg1_1); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0))._0); auto&& __arg1_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))._0); return __self_0 == __arg1_0; } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0))._0); auto&& __arg1_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))._0); return __self_0 == __arg1_0; } if (true) { return [&]() -> bool { rusty::intrinsics::unreachable(); }(); } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
        }
        void assert_receiver_is_total_eq() const {
        }
        template<typename __H>
        void hash(__H& state) const {
            const auto __self_discr = rusty::intrinsics::discriminant_value((*this));
            rusty::hash::hash(__self_discr, state);
            {
                auto&& _m = (*this);
                std::visit(overloaded {
                    [&](const EitherOrBoth_Both<A, A>& _v) {
                        auto&& __self_0 = rusty::detail::deref_if_pointer(_v._0);
                        auto&& __self_1 = rusty::detail::deref_if_pointer(_v._1);
                        rusty::hash::hash(__self_0, state);
                        rusty::hash::hash(__self_1, state);
                    },
                    [&](const EitherOrBoth_Left<A, A>& _v) {
                        auto&& __self_0 = rusty::detail::deref_if_pointer(_v._0);
                        rusty::hash::hash(__self_0, state);
                    },
                    [&](const EitherOrBoth_Right<A, A>& _v) {
                        auto&& __self_0 = rusty::detail::deref_if_pointer(_v._0);
                        rusty::hash::hash(__self_0, state);
                    },
                }, _m);
            }
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return [&]() -> rusty::fmt::Result { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& __self_1 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return std::conditional_t<true, rusty::fmt::Formatter, A>::debug_tuple_field2_finish(f, "Both", __self_0, __self_1); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return std::conditional_t<true, rusty::fmt::Formatter, A>::debug_tuple_field1_finish(f, "Left", __self_0); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return std::conditional_t<true, rusty::fmt::Formatter, A>::debug_tuple_field1_finish(f, "Right", __self_0); } return [&]() -> rusty::fmt::Result { rusty::intrinsics::unreachable(); }(); }();
        }
        bool has_left() const {
            return (*this).as_ref().left().is_some();
        }
        bool has_right() const {
            return (*this).as_ref().right().is_some();
        }
        bool is_left() const {
            return [&]() -> bool { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return true; } if (true) { return false; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
        }
        bool is_right() const {
            return [&]() -> bool { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return true; } if (true) { return false; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
        }
        bool is_both() const {
            return (*this).as_ref().both().is_some();
        }
        rusty::Option<A> left() {
            return [&]() { auto&& _m = (*this); return std::visit(overloaded { [&](auto&&) -> rusty::Option<A> { return [&]() -> rusty::Option<A> { rusty::intrinsics::unreachable(); }(); }, [&](auto&&) -> rusty::Option<A> { return rusty::Option<A>(rusty::None); } }, std::move(_m)); }();
        }
        rusty::Option<B> right() {
            return [&]() { auto&& _m = (*this); return std::visit(overloaded { [&](auto&&) -> rusty::Option<B> { return [&]() -> rusty::Option<B> { rusty::intrinsics::unreachable(); }(); }, [&](auto&&) -> rusty::Option<B> { return rusty::Option<B>(rusty::None); } }, std::move(_m)); }();
        }
        std::tuple<rusty::Option<A>, rusty::Option<B>> left_and_right() {
            return this->map_any(rusty::Some, rusty::Some).or_default();
        }
        rusty::Option<A> just_left() {
            return [&]() -> rusty::Option<A> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& left = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::Option<A>(left); } if (true) { return rusty::Option<A>(rusty::None); } return [&]() -> rusty::Option<A> { rusty::intrinsics::unreachable(); }(); }();
        }
        rusty::Option<B> just_right() {
            return [&]() -> rusty::Option<B> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& right = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::Option<B>(right); } if (true) { return rusty::Option<B>(rusty::None); } return [&]() -> rusty::Option<B> { rusty::intrinsics::unreachable(); }(); }();
        }
        rusty::Option<std::tuple<A, B>> both() {
            return [&]() -> rusty::Option<std::tuple<A, B>> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& a = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& b = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return rusty::Option<std::tuple<A, B>>(std::make_tuple(a, b)); } if (true) { return rusty::Option<std::tuple<A, B>>(rusty::None); } return [&]() -> rusty::Option<std::tuple<A, B>> { rusty::intrinsics::unreachable(); }(); }();
        }
        A into_left() {
            return [&]() { auto&& _m = (*this); return std::visit(overloaded { [&](auto&&) -> A { return [&]() -> A { rusty::intrinsics::unreachable(); }(); }, [&](EitherOrBoth_Right<A, B>&& _v) -> A { auto&& b = rusty::detail::deref_if_pointer(_v._0); return rusty::from_into<A>(b); } }, std::move(_m)); }();
        }
        B into_right() {
            return [&]() { auto&& _m = (*this); return std::visit(overloaded { [&](auto&&) -> B { return [&]() -> B { rusty::intrinsics::unreachable(); }(); }, [&](EitherOrBoth_Left<A, B>&& _v) -> B { auto&& a = rusty::detail::deref_if_pointer(_v._0); return rusty::from_into<B>(a); } }, std::move(_m)); }();
        }
        EitherOrBoth<const A&, const B&> as_ref() const {
            return [&]() -> EitherOrBoth<const A&, const B&> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& left = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return EitherOrBoth<const A&, const B&>(EitherOrBoth<const A&, const B&>{EitherOrBoth_Left<const A&, const B&>{left}}); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& right = std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return EitherOrBoth<const A&, const B&>(EitherOrBoth<const A&, const B&>{EitherOrBoth_Right<const A&, const B&>{right}}); } if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& left = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; const auto& right = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1; return EitherOrBoth<const A&, const B&>{EitherOrBoth_Both<const A&, const B&>{left, right}}; } return [&]() -> EitherOrBoth<const A&, const B&> { rusty::intrinsics::unreachable(); }(); }();
        }
        EitherOrBoth<A&, B&> as_mut() {
            return [&]() -> EitherOrBoth<A&, B&> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& left = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return EitherOrBoth<A&, B&>(EitherOrBoth<A&, B&>{EitherOrBoth_Left<A&, B&>{left}}); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& right = std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return EitherOrBoth<A&, B&>(EitherOrBoth<A&, B&>{EitherOrBoth_Right<A&, B&>{right}}); } if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& left = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; auto& right = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1; return EitherOrBoth<A&, B&>{EitherOrBoth_Both<A&, B&>{left, right}}; } return [&]() -> EitherOrBoth<A&, B&> { rusty::intrinsics::unreachable(); }(); }();
        }
        auto as_deref() const {
            return [&]() -> EitherOrBoth<const typename A::Target&, const typename B::Target&> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& left = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return EitherOrBoth<const typename A::Target&, const typename B::Target&>(EitherOrBoth<const typename A::Target&, const typename B::Target&>{EitherOrBoth_Left<const typename A::Target&, const typename B::Target&>{left}}); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& right = std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return EitherOrBoth<const typename A::Target&, const typename B::Target&>(EitherOrBoth<const typename A::Target&, const typename B::Target&>{EitherOrBoth_Right<const typename A::Target&, const typename B::Target&>{right}}); } if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { const auto& left = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; const auto& right = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1; return EitherOrBoth<const typename A::Target&, const typename B::Target&>{EitherOrBoth_Both<const typename A::Target&, const typename B::Target&>{left, right}}; } return [&]() -> EitherOrBoth<const typename A::Target&, const typename B::Target&> { rusty::intrinsics::unreachable(); }(); }();
        }
        auto as_deref_mut() {
            return [&]() -> EitherOrBoth<typename A::Target&, typename B::Target&> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& left = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return EitherOrBoth<typename A::Target&, typename B::Target&>(EitherOrBoth<typename A::Target&, typename B::Target&>{EitherOrBoth_Left<typename A::Target&, typename B::Target&>{left}}); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& right = std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; return EitherOrBoth<typename A::Target&, typename B::Target&>(EitherOrBoth<typename A::Target&, typename B::Target&>{EitherOrBoth_Right<typename A::Target&, typename B::Target&>{right}}); } if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto& left = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0; auto& right = std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1; return EitherOrBoth<typename A::Target&, typename B::Target&>{EitherOrBoth_Both<typename A::Target&, typename B::Target&>{left, right}}; } return [&]() -> EitherOrBoth<typename A::Target&, typename B::Target&> { rusty::intrinsics::unreachable(); }(); }();
        }
        EitherOrBoth<B, A> flip() {
            return [&]() -> EitherOrBoth<B, A> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& a = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return EitherOrBoth<B, A>(EitherOrBoth<B, A>{EitherOrBoth_Right<B, A>{a}}); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& b = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return EitherOrBoth<B, A>(EitherOrBoth<B, A>{EitherOrBoth_Left<B, A>{b}}); } if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& a = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& b = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return EitherOrBoth<B, A>{EitherOrBoth_Both<B, A>{b, a}}; } return [&]() -> EitherOrBoth<B, A> { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename F>
        auto map_left(F f) {
            using M = std::remove_cvref_t<std::invoke_result_t<F&, A>>;
            return [&]() -> EitherOrBoth<M, B> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& a = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& b = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return EitherOrBoth<M, B>{EitherOrBoth_Both<M, B>{f(a), b}}; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& a = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return EitherOrBoth<M, B>(EitherOrBoth<M, B>{EitherOrBoth_Left<M, B>{f(a)}}); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& b = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return EitherOrBoth<M, B>(EitherOrBoth<M, B>{EitherOrBoth_Right<M, B>{b}}); } return [&]() -> EitherOrBoth<M, B> { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename F>
        auto map_right(F f) {
            using M = std::remove_cvref_t<std::invoke_result_t<F&, B>>;
            return [&]() -> EitherOrBoth<A, M> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& a = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return EitherOrBoth<A, M>(EitherOrBoth<A, M>{EitherOrBoth_Left<A, M>{a}}); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& b = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return EitherOrBoth<A, M>(EitherOrBoth<A, M>{EitherOrBoth_Right<A, M>{f(b)}}); } if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& a = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& b = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return EitherOrBoth<A, M>{EitherOrBoth_Both<A, M>{a, f(b)}}; } return [&]() -> EitherOrBoth<A, M> { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename F, typename G>
        auto map_any(F f, G g) {
            using L = std::remove_cvref_t<std::invoke_result_t<F&, A>>;
            using R = std::remove_cvref_t<std::invoke_result_t<G&, B>>;
            return [&]() -> EitherOrBoth<L, R> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& a = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return EitherOrBoth<L, R>(EitherOrBoth<L, R>{EitherOrBoth_Left<L, R>{f(a)}}); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& b = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return EitherOrBoth<L, R>(EitherOrBoth<L, R>{EitherOrBoth_Right<L, R>{g(b)}}); } if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& a = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& b = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return EitherOrBoth<L, R>{EitherOrBoth_Both<L, R>{f(a), g(b)}}; } return [&]() -> EitherOrBoth<L, R> { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename F, typename L>
        EitherOrBoth<L, B> left_and_then(F f) {
            return [&]() { auto&& _m = (*this); return std::visit(overloaded { [&](auto&&) -> EitherOrBoth<L, B> { return [&]() -> EitherOrBoth<L, B> { rusty::intrinsics::unreachable(); }(); }, [&](EitherOrBoth_Right<A, B>&& _v) -> EitherOrBoth<L, B> { auto&& b = rusty::detail::deref_if_pointer(_v._0); return EitherOrBoth<L, B>(EitherOrBoth<L, B>{EitherOrBoth_Right<L, B>{b}}); } }, std::move(_m)); }();
        }
        template<typename F, typename R>
        EitherOrBoth<A, R> right_and_then(F f) {
            return [&]() { auto&& _m = (*this); return std::visit(overloaded { [&](EitherOrBoth_Left<A, B>&& _v) -> EitherOrBoth<A, R> { auto&& a = rusty::detail::deref_if_pointer(_v._0); return EitherOrBoth<A, R>(EitherOrBoth<A, R>{EitherOrBoth_Left<A, R>{a}}); }, [&](auto&&) -> EitherOrBoth<A, R> { return [&]() -> EitherOrBoth<A, R> { rusty::intrinsics::unreachable(); }(); } }, std::move(_m)); }();
        }
        std::tuple<A, B> or_(A l, B r) {
            return [&]() -> std::tuple<A, B> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner_l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return std::make_tuple(inner_l, std::move(r)); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner_r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return std::make_tuple(std::move(l), inner_r); } if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner_l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& inner_r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return std::make_tuple(inner_l, inner_r); } return [&]() -> std::tuple<A, B> { rusty::intrinsics::unreachable(); }(); }();
        }
        std::tuple<A, B> or_default() {
            return [&]() -> std::tuple<A, B> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return std::make_tuple(l, B::default_()); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return std::make_tuple(A::default_(), r); } if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return std::make_tuple(l, r); } return [&]() -> std::tuple<A, B> { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename L, typename R>
        std::tuple<A, B> or_else(L l, R r) {
            return [&]() -> std::tuple<A, B> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner_l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return std::make_tuple(inner_l, r()); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner_r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return std::make_tuple(l(), inner_r); } if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& inner_l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& inner_r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return std::make_tuple(inner_l, inner_r); } return [&]() -> std::tuple<A, B> { rusty::intrinsics::unreachable(); }(); }();
        }
        A& left_or_insert(A val) {
            return this->left_or_insert_with([&]() { return val; });
        }
        B& right_or_insert(B val) {
            return this->right_or_insert_with([&]() { return val; });
        }
        template<typename F>
        A& left_or_insert_with(F f) {
            return [&]() { auto&& _m = (*this); return std::visit(overloaded { [&](const auto&) -> A& { return [&]() -> A& { rusty::intrinsics::unreachable(); }(); }, [&](EitherOrBoth_Right<A, B>& _v) -> A& {  return this->insert_left(f()); } }, _m); }();
        }
        template<typename F>
        B& right_or_insert_with(F f) {
            return [&]() { auto&& _m = (*this); return std::visit(overloaded { [&](const auto&) -> B& { return [&]() -> B& { rusty::intrinsics::unreachable(); }(); }, [&](EitherOrBoth_Left<A, B>& _v) -> B& {  return this->insert_right(f()); } }, _m); }();
        }
        A& insert_left(A val) {
            return [&]() { auto&& _m = (*this); return std::visit(overloaded { [&](const auto&) -> A& { return [&]() -> A& { rusty::intrinsics::unreachable(); }(); }, [&](EitherOrBoth_Right<A, B>& _v) -> A& { auto&& right = rusty::detail::deref_if_pointer(_v._0); return [&]() -> A& { // @unsafe
{
    auto right = rusty::ptr::read(&right);
    rusty::ptr::write(&(*this), std::move(Both(std::move(val), right)));
}
return [&]() -> A& { auto&& _iflet = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_iflet))>>>(rusty::detail::deref_if_pointer(_iflet))) { auto&& left = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_iflet))>>>(rusty::detail::deref_if_pointer(_iflet))._0);
return left; } return [&]() -> A& { rusty::intrinsics::unreachable(); }(); }(); }(); } }, _m); }();
        }
        B& insert_right(B val) {
            return [&]() { auto&& _m = (*this); return std::visit(overloaded { [&](const auto&) -> B& { return [&]() -> B& { rusty::intrinsics::unreachable(); }(); }, [&](EitherOrBoth_Left<A, B>& _v) -> B& { auto&& left = rusty::detail::deref_if_pointer(_v._0); return [&]() -> B& { // @unsafe
{
    auto left = rusty::ptr::read(&left);
    rusty::ptr::write(&(*this), std::move(Both(left, std::move(val))));
}
return [&]() -> B& { auto&& _iflet = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_iflet))>>>(rusty::detail::deref_if_pointer(_iflet))) { auto&& right = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_iflet))>>>(rusty::detail::deref_if_pointer(_iflet))._1);
return right; } return [&]() -> B& { rusty::intrinsics::unreachable(); }(); }(); }(); } }, _m); }();
        }
        std::tuple<A&, B&> insert_both(A left, B right) {
            (*this) = Both(std::move(left), std::move(right));
            if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer((*this)))>>>(rusty::detail::deref_if_pointer((*this)))) {
                auto&& left = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer((*this)))>>>(rusty::detail::deref_if_pointer((*this)))._0);
                auto&& right = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer((*this)))>>>(rusty::detail::deref_if_pointer((*this)))._1);
                return std::tuple<A&, B&>{left, right};
            } else {
                // @unsafe
                {
                    return [&]() -> std::tuple<A&, B&> { rusty::intrinsics::unreachable(); }();
                }
            }
        }
        template<typename F, typename T>
        T reduce(F f) {
            return [&]() -> T { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& a = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return a; } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& b = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return b; } if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& a = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& b = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return f(a, b); } return [&]() -> T { rusty::intrinsics::unreachable(); }(); }();
        }
        static EitherOrBoth<A, B> from(rusty::Either<A, B> either) {
            return [&]() -> EitherOrBoth<A, B> { auto&& _m = either; if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return EitherOrBoth<A, B>(EitherOrBoth<A, B>{EitherOrBoth_Left<A, B>{l}}); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return EitherOrBoth<A, B>(EitherOrBoth<A, B>{EitherOrBoth_Right<A, B>{l}}); } return [&]() -> EitherOrBoth<A, B> { rusty::intrinsics::unreachable(); }(); }();
        }
    };
    template<typename A, typename B>
    EitherOrBoth_Both<A, B> Both(A _0, B _1) { return EitherOrBoth_Both<A, B>{std::forward<A>(_0), std::forward<B>(_1)};  }
    template<typename A, typename B>
    EitherOrBoth_Left<A, B> Left(A _0) { return EitherOrBoth_Left<A, B>{std::forward<A>(_0)};  }
    template<typename A, typename B>
    EitherOrBoth_Right<A, B> Right(B _0) { return EitherOrBoth_Right<A, B>{std::forward<B>(_0)};  }

    // Extension trait From lowered to rusty_ext:: free functions
    namespace rusty_ext {
        // Rust-only extension method skipped (no receiver): from

    }


}

using concat_impl::concat;

using cons_tuples_impl::cons_tuples;

using diff::diff_with;

using diff::Diff;

// Rust-only unresolved function import (forward decl unavailable): using kmerge_impl::kmerge_by;

using minmax::MinMaxResult;


using process_results_impl::process_results;

using repeatn::repeat_n;

using sources::iterate;
using sources::unfold;

namespace structs {}
using namespace ::structs;

using unziptuple::multiunzip;

using with_position::Position;

using ziptuple::multizip;

namespace duplicates_impl {
    namespace private_ {}

    namespace private_ {
        template<typename Key, typename F>
        struct Meta;
        template<typename I, typename Key, typename F>
        struct DuplicatesBy;
        struct ById;
        template<typename F>
        struct ByFn;
        template<typename K, typename V>
        struct KeyValue;
        template<typename V>
        struct JustValue;
    }
    template<typename I, typename Key, typename F>
    DuplicatesBy<I, Key, F> duplicates_by(I iter, F f);
    template<typename I>
    Duplicates<I> duplicates(I iter);


    namespace private_ {

        template<typename Key, typename F>
        struct Meta;
        template<typename I, typename Key, typename F>
        struct DuplicatesBy;
        struct ById;
        template<typename F>
        struct ByFn;
        template<typename K, typename V>
        struct KeyValue;
        template<typename V>
        struct JustValue;

        using ::rusty::HashMap;

        namespace fmt = rusty::fmt;


        template<typename Key, typename F>
        struct Meta {
            rusty::HashMap<Key, bool> used;
            size_t pending;
            F key_method;

            Meta<Key, F> clone() const {
                return Meta<Key, F>{.used = rusty::clone(this->used), .pending = rusty::clone(this->pending), .key_method = rusty::clone(this->key_method)};
            }
            template<typename I>
            rusty::Option<I> filter(I item) {
                const auto kv = this->key_method.make(std::move(item));
                return [&]() -> rusty::Option<I> { auto&& _m = this->used.get_mut(kv.key_ref()); if (_m.is_none()) { return [&]() -> rusty::Option<I> { this->used.insert(kv.key(), false);
[&]() { static_cast<void>(this->pending += 1); return std::make_tuple(); }();
return rusty::Option<I>(rusty::None); }(); } if (_m.is_some()) { auto&& _mv1 = std::as_const(_m).unwrap(); if (_mv1 == true) { return rusty::Option<I>(rusty::None); } } if (_m.is_some()) { auto&& _mv2 = _m.unwrap(); auto&& produced = rusty::detail::deref_if_pointer(_mv2); return [&]() -> rusty::Option<I> { rusty::detail::deref_if_pointer_like(produced) = true;
[&]() { static_cast<void>(this->pending -= 1); return std::make_tuple(); }();
return rusty::Option<I>(kv.value()); }(); } return [&]() -> rusty::Option<I> { rusty::intrinsics::unreachable(); }(); }();
            }
        };

        template<typename I, typename Key, typename F>
        struct DuplicatesBy {
            // Rust-only dependent associated type alias skipped in constrained mode: Item
            I iter;
            Meta<Key, F> meta;

            DuplicatesBy<I, Key, F> clone() const {
                return DuplicatesBy<I, Key, F>{.iter = rusty::clone(this->iter), .meta = rusty::clone(this->meta)};
            }
            template<typename V>
            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
                return f.debug_struct("DuplicatesBy").field("iter", &this->iter).field("meta.used", &this->meta.used).finish();
            }
            static DuplicatesBy<I, Key, F> new_(I iter, F key_method) {
                return DuplicatesBy<I, Key, F>{.iter = std::move(iter), .meta = Meta<Key, F>{.used = rusty::HashMap<Key, bool>(), .pending = static_cast<size_t>(0), .key_method = std::move(key_method)}};
            }
            // Rust-only dependent associated type alias skipped in constrained mode: Item
            auto next() {
                auto&& _let_pat = (*this);
                auto&& iter = rusty::detail::deref_if_pointer(_let_pat.iter);
                auto&& meta = rusty::detail::deref_if_pointer(_let_pat.meta);
                return iter.find_map([&](auto&& v) { return meta.filter(std::move(v)); });
            }
            std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
                auto [_tuple_ignore0, hi] = rusty::detail::deref_if_pointer_like(rusty::size_hint(this->iter));
                auto hi_shadow1 = hi.map([&](auto&& hi) {
if (hi <= this->meta.pending) {
    return hi;
} else {
    return this->meta.pending + (((hi - this->meta.pending)) / 2);
}
});
                return std::make_tuple(static_cast<size_t>(0), std::move(hi_shadow1));
            }
            auto next_back() {
                auto&& _let_pat = (*this);
                auto&& iter = rusty::detail::deref_if_pointer(_let_pat.iter);
                auto&& meta = rusty::detail::deref_if_pointer(_let_pat.meta);
                return iter.rev().find_map([&](auto&& v) { return meta.filter(std::move(v)); });
            }
        };

        // Rust-only trait KeyMethod (Proxy facade emission skipped in module mode)

        /// Apply the identity function to elements before checking them for equality.
        struct ById {
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Container

            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
            ById clone() const;
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Container
            template<typename V>
            auto make(V v);
        };

        /// Apply a user-supplied function to elements before checking them for equality.
        template<typename F>
        struct ByFn {
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Container
            F _0;

            ByFn<F> clone() const {
                return ByFn(rusty::clone(this->_0));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
                return f.debug_struct("ByFn").finish();
            }
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Container
            template<typename K, typename V>
            auto make(V v) {
                return KeyValue<K, V>::KeyValue((this->_0)(v), std::move(v));
            }
        };

        // Rust-only trait KeyXorValue (Proxy facade emission skipped in module mode)

        template<typename K, typename V>
        struct KeyValue {
            K _0;
            V _1;

            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
                return std::conditional_t<true, rusty::fmt::Formatter, K>::debug_tuple_field2_finish(f, "KeyValue", &this->_0, &this->_1);
            }
            const K& key_ref() const {
                return this->_0;
            }
            K key() {
                return this->_0;
            }
            V value() {
                return this->_1;
            }
        };

        template<typename V>
        struct JustValue {
            V _0;

            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
                return std::conditional_t<true, rusty::fmt::Formatter, V>::debug_tuple_field1_finish(f, "JustValue", &this->_0);
            }
            const V& key_ref() const {
                return this->_0;
            }
            V key() {
                return this->_0;
            }
            V value() {
                return this->_0;
            }
        };

    }



}

using either_or_both::EitherOrBoth;
namespace either_or_both {}
using namespace ::either_or_both;

namespace format {

    template<typename I, typename F>
    struct FormatWith;
    template<typename I>
    struct Format;
    template<typename I, typename F>
    FormatWith<I, F> new_format(I iter, std::string_view separator, F f);
    template<typename I>
    Format<I> new_format_default(I iter, std::string_view separator);

    using ::rusty::Cell;

    namespace fmt = rusty::fmt;

    /// Format all iterator elements lazily, separated by `sep`.
    ///
    /// The format value can only be formatted once, after that the iterator is
    /// exhausted.
    ///
    /// See [`.format_with()`](crate::Itertools::format_with) for more information.
    template<typename I, typename F>
    struct FormatWith {
        std::string_view sep;
        /// `FormatWith` uses interior mutability because `Display::fmt` takes `&self`.
        rusty::Cell<rusty::Option<std::tuple<I, F>>> inner;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            auto [iter, format_shadow1] = rusty::detail::deref_if_pointer_like([&]() { auto&& _m = this->inner.take(); if (_m.is_some()) { return _m.unwrap(); } if (_m.is_none()) { return [&]() { rusty::panic::begin_panic("FormatWith: was already formatted once"); }(); } rusty::intrinsics::unreachable(); }());
            if (auto&& _iflet_scrutinee = iter.next(); rusty::detail::option_has_value(_iflet_scrutinee)) {
                auto&& _iflet_take = _iflet_scrutinee;
                auto fst = rusty::detail::option_take_value(_iflet_take);
                RUSTY_TRY(format_shadow1(std::move(fst), rusty::addr_of_temp([&](rusty::fmt::DisplayRef disp) { return disp.fmt(f); })));
                RUSTY_TRY(iter.try_for_each([&](auto&& elt) {
if (!rusty::is_empty(this->sep)) {
    RUSTY_TRY(f.write_str(this->sep));
}
return format_shadow1(std::move(elt), rusty::addr_of_temp([&](rusty::fmt::DisplayRef disp) { return disp.fmt(f); }));
}));
            }
            return rusty::fmt::Result::Ok(std::make_tuple());
        }
        struct PutBackOnDrop {
            const FormatWith<I, F>& into;
            rusty::Option<std::tuple<I, F>> inner;
            PutBackOnDrop(const FormatWith<I, F>& into_init, rusty::Option<std::tuple<I, F>> inner_init) : into(into_init), inner(std::move(inner_init)) {}
            PutBackOnDrop(const PutBackOnDrop&) = default;
            PutBackOnDrop(PutBackOnDrop&& other) noexcept : into(other.into), inner(std::move(other.inner)) {
                if (rusty::mem::consume_forgotten_address(&other)) {
                    this->rusty_mark_forgotten();
                    other.rusty_mark_forgotten();
                } else {
                    other.rusty_mark_forgotten();
                }
            }
            PutBackOnDrop& operator=(const PutBackOnDrop&) = default;
            PutBackOnDrop& operator=(PutBackOnDrop&& other) noexcept {
                if (this == &other) {
                    return *this;
                }
                this->~PutBackOnDrop();
                new (this) PutBackOnDrop(std::move(other));
                return *this;
            }
            void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


            ~PutBackOnDrop() noexcept(false) {
                if (rusty::mem::consume_forgotten_address(this)) { return; }
                this->into.inner.set(this->inner.take());
            }
        };
        FormatWith<I, F> clone() const {
            const auto pbod = PutBackOnDrop((*this), this->inner.take());
            return FormatWith<I, F>{.sep = std::string_view(this->sep), .inner = rusty::Cell<rusty::Option<std::tuple<I, F>>>::new_(rusty::clone(pbod.inner))};
        }
    };

    /// Format all iterator elements lazily, separated by `sep`.
    ///
    /// The format value can only be formatted once, after that the iterator is
    /// exhausted.
    ///
    /// See [`.format()`](crate::Itertools::format)
    /// for more information.
    template<typename I>
    struct Format {
        std::string_view sep;
        /// `Format` uses interior mutability because `Display::fmt` takes `&self`.
        rusty::Cell<rusty::Option<I>> inner;

        rusty::fmt::Result format(rusty::fmt::Formatter& f, rusty::SafeFn<rusty::fmt::Result(const rusty::detail::associated_item_t<I>&, rusty::fmt::Formatter&)> cb) const {
            auto iter = [&]() { auto&& _m = this->inner.take(); if (_m.is_some()) { return _m.unwrap(); } if (_m.is_none()) { return [&]() { rusty::panic::begin_panic("Format: was already formatted once"); }(); } rusty::intrinsics::unreachable(); }();
            if (auto&& _iflet_scrutinee = iter.next(); rusty::detail::option_has_value(_iflet_scrutinee)) {
                auto&& _iflet_take = _iflet_scrutinee;
                auto fst = rusty::detail::option_take_value(_iflet_take);
                RUSTY_TRY(cb(fst, f));
                RUSTY_TRY(iter.try_for_each([&](auto&& elt) {
if (!rusty::is_empty(this->sep)) {
    RUSTY_TRY(f.write_str(this->sep));
}
return cb(elt, f);
}));
            }
            return rusty::fmt::Result::Ok(std::make_tuple());
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return this->format(f, [](auto&& _f, auto&&... _args) -> decltype(auto) { return _f.fmt(std::forward<decltype(_args)>(_args)...); });
        }
        struct PutBackOnDrop {
            const Format<I>& into;
            rusty::Option<I> inner;
            PutBackOnDrop(const Format<I>& into_init, rusty::Option<I> inner_init) : into(into_init), inner(std::move(inner_init)) {}
            PutBackOnDrop(const PutBackOnDrop&) = default;
            PutBackOnDrop(PutBackOnDrop&& other) noexcept : into(other.into), inner(std::move(other.inner)) {
                if (rusty::mem::consume_forgotten_address(&other)) {
                    this->rusty_mark_forgotten();
                    other.rusty_mark_forgotten();
                } else {
                    other.rusty_mark_forgotten();
                }
            }
            PutBackOnDrop& operator=(const PutBackOnDrop&) = default;
            PutBackOnDrop& operator=(PutBackOnDrop&& other) noexcept {
                if (this == &other) {
                    return *this;
                }
                this->~PutBackOnDrop();
                new (this) PutBackOnDrop(std::move(other));
                return *this;
            }
            void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


            ~PutBackOnDrop() noexcept(false) {
                if (rusty::mem::consume_forgotten_address(this)) { return; }
                this->into.inner.set(this->inner.take());
            }
        };
        Format<I> clone() const {
            const auto pbod = PutBackOnDrop((*this), this->inner.take());
            return Format<I>{.sep = std::string_view(this->sep), .inner = rusty::Cell<rusty::Option<I>>::new_(rusty::clone(pbod.inner))};
        }
    };

}

namespace free_mod {}
using namespace free_mod;

namespace groupbylazy {

    struct ChunkIndex;
    template<typename K, typename I, typename F>
    struct GroupInner;
    template<typename K, typename I, typename F>
    struct ChunkBy;
    template<typename K, typename I, typename F>
    struct Groups;
    template<typename K, typename I, typename F>
    struct Group;
    template<typename I>
    struct IntoChunks;
    template<typename I>
    struct Chunks;
    template<typename I>
    struct Chunk;
    template<typename K, typename J, typename F>
    ChunkBy<K, typename J::IntoIter, F> new_(J iter, F f);
    template<typename J>
    IntoChunks<typename J::IntoIter> new_chunks(J iter, size_t size);

    using ::rusty::Vec;

    using ::rusty::Cell;
    using ::rusty::RefCell;


    /// `ChunkIndex` acts like the grouping key function for `IntoChunks`
    struct ChunkIndex {
        using Key = size_t;
        size_t size;
        size_t index;
        size_t key;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
        ChunkIndex clone() const;
        static ChunkIndex new_(size_t size);
        template<typename A>
        size_t call_mut(A _arg);
    };

    template<typename K, typename I, typename F>
    struct GroupInner {
        F key;
        I iter;
        rusty::Option<K> current_key;
        rusty::Option<rusty::detail::associated_item_t<I>> current_elt;
        /// flag set if iterator is exhausted
        bool done;
        /// Index of group we are currently buffering or visiting
        size_t top_group;
        /// Least index for which we still have elements buffered
        size_t oldest_buffered_group;
        /// Group index for `buffer[0]` -- the slots
        /// `bottom_group..oldest_buffered_group` are unused and will be erased when
        /// that range is large enough.
        size_t bottom_group;
        /// Buffered groups, from `bottom_group` (index 0) to `top_group`.
        rusty::Vec<decltype(rusty::iter(std::declval<rusty::Vec<rusty::detail::associated_item_t<I>>>()))> buffer;
        /// index of last group iter that was dropped,
        /// `usize::MAX` initially when no group was dropped
        size_t dropped_group;

        GroupInner<K, I, F> clone() const {
            return GroupInner<K, I, F>{.key = rusty::clone(this->key), .iter = rusty::clone(this->iter), .current_key = rusty::clone(this->current_key), .current_elt = rusty::clone(this->current_elt), .done = rusty::clone(this->done), .top_group = rusty::clone(this->top_group), .oldest_buffered_group = rusty::clone(this->oldest_buffered_group), .bottom_group = rusty::clone(this->bottom_group), .buffer = rusty::clone(this->buffer), .dropped_group = rusty::clone(this->dropped_group)};
        }
        auto step(size_t client) {
            if (client < this->oldest_buffered_group) {
                return rusty::Option<rusty::detail::associated_item_t<I>>(rusty::None);
            } else if ((client < this->top_group) || (((client == this->top_group) && (rusty::len(this->buffer) > (this->top_group - this->bottom_group))))) {
                return this->lookup_buffer(std::move(client));
            } else if (this->done) {
                return rusty::Option<rusty::detail::associated_item_t<I>>(rusty::None);
            } else if (this->top_group == client) {
                return this->step_current();
            } else {
                return this->step_buffering(std::move(client));
            }
        }
        auto lookup_buffer(size_t client) {
            const auto bufidx = client - this->bottom_group;
            if (client < this->oldest_buffered_group) {
                return rusty::Option<rusty::detail::associated_item_t<I>>(rusty::None);
            }
            auto elt = rusty::get_mut(this->buffer, std::move(bufidx)).and_then([&](auto&& queue) { return queue.next(); });
            if (elt.is_none() && (client == this->oldest_buffered_group)) {
                [&]() { static_cast<void>(this->oldest_buffered_group += 1); return std::make_tuple(); }();
                while (rusty::get(this->buffer, this->oldest_buffered_group - this->bottom_group).map_or(false, [&](auto&& buf) { return rusty::len(buf) == 0; })) {
                    [&]() { static_cast<void>(this->oldest_buffered_group += 1); return std::make_tuple(); }();
                }
                const auto nclear = this->oldest_buffered_group - this->bottom_group;
                if ((nclear > 0) && (nclear >= (rusty::len(this->buffer) / 2))) {
                    auto i = 0;
                    this->buffer.retain([&](auto&& buf) {
[&]() { static_cast<void>(i += 1); return std::make_tuple(); }();
if (true) {
    if (!((rusty::len(buf) == 0) || (i > nclear))) {
        rusty::panicking::panic("assertion failed: buf.len() == 0 || i > nclear");
    }
}
return i > nclear;
});
                    this->bottom_group = this->oldest_buffered_group;
                }
            }
            return elt;
        }
        auto next_element() {
            if (true) {
                if (!!this->done) {
                    [&]() -> rusty::Option<rusty::detail::associated_item_t<I>> { rusty::panicking::panic("assertion failed: !self.done"); }();
                }
            }
            return [&]() -> rusty::Option<rusty::detail::associated_item_t<I>> { auto&& _m = this->iter.next(); if (_m.is_none()) { return [&]() -> rusty::Option<rusty::detail::associated_item_t<I>> { this->done = true;
return rusty::Option<rusty::detail::associated_item_t<I>>(rusty::None); }(); } if (true) { const auto& otherwise = _m; return otherwise; } return [&]() -> rusty::Option<rusty::detail::associated_item_t<I>> { rusty::intrinsics::unreachable(); }(); }();
        }
        auto step_buffering(size_t client) {
            if (true) {
                if (!((this->top_group + 1) == client)) {
                    [&]() -> rusty::Option<rusty::detail::associated_item_t<I>> { rusty::panicking::panic("assertion failed: self.top_group + 1 == client"); }();
                }
            }
            auto group = rusty::Vec<rusty::detail::associated_item_t<I>>::new_();
            if (auto&& _iflet_scrutinee = this->current_elt.take(); _iflet_scrutinee.is_some()) {
                decltype(auto) elt = _iflet_scrutinee.unwrap();
                if (this->top_group != this->dropped_group) {
                    group.push(std::move(elt));
                }
            }
            rusty::Option<rusty::detail::associated_item_t<I>> first_elt = rusty::Option<rusty::detail::associated_item_t<I>>(rusty::None);
            while (true) {
                auto&& _whilelet = this->next_element();
                if (!(_whilelet.is_some())) { break; }
                auto elt = _whilelet.unwrap();
                auto key = ([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::groupbylazy::rusty_ext::call_mut(std::forward<decltype(__self)>(__self), elt); }) { return ::groupbylazy::rusty_ext::call_mut(std::forward<decltype(__self)>(__self), elt); } else { return ::groupbylazy::rusty_ext::call_mut(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self)), elt); } })(this->key);
                {
                    auto&& _m = this->current_key.take();
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_none()) {
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_some()) {
                            auto&& _mv1 = _m.unwrap();
                            auto&& old_key = rusty::detail::deref_if_pointer(_mv1);
                            if (old_key != key) {
                                this->current_key = rusty::Option<K>(std::move(key));
                                first_elt = rusty::Option<rusty::detail::associated_item_t<I>>(std::move(elt));
                                break;
                            }
                            _m_matched = true;
                        }
                    }
                }
                this->current_key = rusty::Option<K>(std::move(key));
                if (this->top_group != this->dropped_group) {
                    group.push(std::move(elt));
                }
            }
            if (this->top_group != this->dropped_group) {
                this->push_next_group(std::move(group));
            }
            if (first_elt.is_some()) {
                [&]() { static_cast<void>(this->top_group += 1); return std::make_tuple(); }();
                if (true) {
                    if (!(this->top_group == client)) {
                        [&]() -> rusty::Option<rusty::detail::associated_item_t<I>> { rusty::panicking::panic("assertion failed: self.top_group == client"); }();
                    }
                }
            }
            return first_elt;
        }
        void push_next_group(rusty::Vec<rusty::detail::associated_item_t<I>> group) {
            while ((this->top_group - this->bottom_group) > rusty::len(this->buffer)) {
                if (rusty::is_empty(this->buffer)) {
                    [&]() { static_cast<void>(this->bottom_group += 1); return std::make_tuple(); }();
                    [&]() { static_cast<void>(this->oldest_buffered_group += 1); return std::make_tuple(); }();
                } else {
                    this->buffer.push(rusty::iter(rusty::Vec<rusty::detail::associated_item_t<I>>::new_()));
                }
            }
            this->buffer.push(rusty::iter(std::move(group)));
            if (true) {
                if (!(((this->top_group + 1) - this->bottom_group) == rusty::len(this->buffer))) {
                    rusty::panicking::panic("assertion failed: self.top_group + 1 - self.bottom_group == self.buffer.len()");
                }
            }
        }
        auto step_current() {
            if (true) {
                if (!!this->done) {
                    [&]() -> rusty::Option<rusty::detail::associated_item_t<I>> { rusty::panicking::panic("assertion failed: !self.done"); }();
                }
            }
            if (true) {
                auto elt = this->current_elt.take();
                return elt;
            }
            return [&]() -> rusty::Option<rusty::detail::associated_item_t<I>> { auto&& _m = this->next_element(); if (_m.is_none()) { return rusty::Option<rusty::detail::associated_item_t<I>>(rusty::None); } if (_m.is_some()) { auto&& _mv1 = _m.unwrap(); auto&& elt = rusty::detail::deref_if_pointer(_mv1); return [&]() -> rusty::Option<rusty::detail::associated_item_t<I>> { auto key = ([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::groupbylazy::rusty_ext::call_mut(std::forward<decltype(__self)>(__self), elt); }) { return ::groupbylazy::rusty_ext::call_mut(std::forward<decltype(__self)>(__self), elt); } else { return ::groupbylazy::rusty_ext::call_mut(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self)), elt); } })(this->key);
{
    auto&& _m = this->current_key.take();
    bool _m_matched = false;
    if (!_m_matched) {
        if (_m.is_none()) {
            _m_matched = true;
        }
    }
    if (!_m_matched) {
        if (_m.is_some()) {
            auto&& _mv1 = _m.unwrap();
            auto&& old_key = rusty::detail::deref_if_pointer(_mv1);
            if (old_key != key) {
                this->current_key = rusty::Option<K>(std::move(key));
                this->current_elt = rusty::Option<rusty::detail::associated_item_t<I>>(std::move(elt));
                [&]() { static_cast<void>(this->top_group += 1); return std::make_tuple(); }();
                return rusty::Option<rusty::detail::associated_item_t<I>>(rusty::None);
            }
            _m_matched = true;
        }
    }
}
this->current_key = rusty::Option<K>(std::move(key));
return rusty::Option<rusty::detail::associated_item_t<I>>(std::move(elt)); }(); } return [&]() -> rusty::Option<rusty::detail::associated_item_t<I>> { rusty::intrinsics::unreachable(); }(); }();
        }
        K group_key(size_t client) {
            if (true) {
                if (!!this->done) {
                    [&]() -> K { rusty::panicking::panic("assertion failed: !self.done"); }();
                }
            }
            if (true) {
                if (!(client == this->top_group)) {
                    [&]() -> K { rusty::panicking::panic("assertion failed: client == self.top_group"); }();
                }
            }
            if (true) {
                if (!this->current_key.is_some()) {
                    [&]() -> K { rusty::panicking::panic("assertion failed: self.current_key.is_some()"); }();
                }
            }
            if (true) {
                if (!this->current_elt.is_none()) {
                    [&]() -> K { rusty::panicking::panic("assertion failed: self.current_elt.is_none()"); }();
                }
            }
            auto old_key = this->current_key.take().unwrap();
            if (auto&& _iflet_scrutinee = this->next_element(); _iflet_scrutinee.is_some()) {
                decltype(auto) elt = _iflet_scrutinee.unwrap();
                auto key = ([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::groupbylazy::rusty_ext::call_mut(std::forward<decltype(__self)>(__self), elt); }) { return ::groupbylazy::rusty_ext::call_mut(std::forward<decltype(__self)>(__self), elt); } else { return ::groupbylazy::rusty_ext::call_mut(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self)), elt); } })(this->key);
                if (old_key != key) {
                    [&]() { static_cast<void>(this->top_group += 1); return std::make_tuple(); }();
                }
                this->current_key = rusty::Option<K>(std::move(key));
                this->current_elt = rusty::Option<rusty::detail::associated_item_t<I>>(std::move(elt));
            }
            return old_key;
        }
        void drop_group(size_t client) {
            if ((this->dropped_group == ~static_cast<int32_t>(0)) || (client > this->dropped_group)) {
                this->dropped_group = std::move(client);
            }
        }
    };


    /// `ChunkBy` is the storage for the lazy grouping operation.
    ///
    /// If the groups are consumed in their original order, or if each
    /// group is dropped without keeping it around, then `ChunkBy` uses
    /// no allocations. It needs allocations only if several group iterators
    /// are alive at the same time.
    ///
    /// This type implements [`IntoIterator`] (it is **not** an iterator
    /// itself), because the group iterators need to borrow from this
    /// value. It should be stored in a local variable or temporary and
    /// iterated.
    ///
    /// See [`.chunk_by()`](crate::Itertools::chunk_by) for more information.
    template<typename K, typename I, typename F>
    struct ChunkBy {
        rusty::RefCell<GroupInner<K, I, F>> inner;
        rusty::Cell<size_t> index;

        auto step(size_t client) const {
            return this->inner.borrow_mut().step(std::move(client));
        }
        void drop_group(size_t client) const {
            this->inner.borrow_mut().drop_group(std::move(client));
        }
    };

    /// An iterator that yields the Group iterators.
    ///
    /// Iterator element type is `(K, Group)`:
    /// the group's key `K` and the group's iterator.
    ///
    /// See [`.chunk_by()`](crate::Itertools::chunk_by) for more information.
    template<typename K, typename I, typename F>
    struct Groups {
        using Item = std::tuple<K, Group<K, I, F>>;
        const ChunkBy<K, I, F>& parent;

        rusty::Option<Item> next() {
            auto index = this->parent.index.get();
            this->parent.index.set(index + 1);
            auto& inner = this->parent.inner.borrow_mut();
            return inner.step(std::move(index)).map([&](auto&& elt) -> Item {
auto key = inner.group_key(std::move(index));
return std::make_tuple(std::move(key), Group<K, I, F>(this->parent, std::move(index), rusty::Option<rusty::detail::associated_item_t<I>>(std::move(elt))));
});
        }
    };

    /// An iterator for the elements in a single group.
    ///
    /// Iterator element type is `I::Item`.
    template<typename K, typename I, typename F>
    struct Group {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        const ChunkBy<K, I, F>& parent;
        size_t index;
        rusty::Option<rusty::detail::associated_item_t<I>> first;
        Group(const ChunkBy<K, I, F>& parent_init, size_t index_init, rusty::Option<rusty::detail::associated_item_t<I>> first_init) : parent(parent_init), index(std::move(index_init)), first(std::move(first_init)) {}
        Group(const Group&) = default;
        Group(Group&& other) noexcept : parent(other.parent), index(std::move(other.index)), first(std::move(other.first)) {
            if (rusty::mem::consume_forgotten_address(&other)) {
                this->rusty_mark_forgotten();
                other.rusty_mark_forgotten();
            } else {
                other.rusty_mark_forgotten();
            }
        }
        Group& operator=(const Group&) = default;
        Group& operator=(Group&& other) noexcept {
            if (this == &other) {
                return *this;
            }
            this->~Group();
            new (this) Group(std::move(other));
            return *this;
        }
        void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


        ~Group() noexcept(false) {
            if (rusty::mem::consume_forgotten_address(this)) { return; }
            this->parent.drop_group(this->index);
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            if (true) {
                auto elt = this->first.take();
                return elt;
            }
            return this->parent.step(this->index);
        }
    };

    /// `ChunkLazy` is the storage for a lazy chunking operation.
    ///
    /// `IntoChunks` behaves just like `ChunkBy`: it is iterable, and
    /// it only buffers if several chunk iterators are alive at the same time.
    ///
    /// This type implements [`IntoIterator`] (it is **not** an iterator
    /// itself), because the chunk iterators need to borrow from this
    /// value. It should be stored in a local variable or temporary and
    /// iterated.
    ///
    /// Iterator element type is `Chunk`, each chunk's iterator.
    ///
    /// See [`.chunks()`](crate::Itertools::chunks) for more information.
    template<typename I>
    struct IntoChunks {
        rusty::RefCell<GroupInner<size_t, I, ChunkIndex>> inner;
        rusty::Cell<size_t> index;

        IntoChunks<I> clone() const {
            return IntoChunks<I>{.inner = rusty::clone(this->inner), .index = rusty::clone(this->index)};
        }
        auto step(size_t client) const {
            return this->inner.borrow_mut().step(std::move(client));
        }
        void drop_group(size_t client) const {
            this->inner.borrow_mut().drop_group(std::move(client));
        }
    };

    /// An iterator that yields the Chunk iterators.
    ///
    /// Iterator element type is `Chunk`.
    ///
    /// See [`.chunks()`](crate::Itertools::chunks) for more information.
    template<typename I>
    struct Chunks {
        using Item = Chunk<I>;
        const IntoChunks<I>& parent;

        Chunks<I> clone() const {
            return Chunks<I>{.parent = rusty::clone(this->parent)};
        }
        rusty::Option<Item> next() {
            auto index = this->parent.index.get();
            this->parent.index.set(index + 1);
            auto& inner = this->parent.inner.borrow_mut();
            return inner.step(std::move(index)).map([&](auto&& elt) -> Item { return Chunk<I>(this->parent, std::move(index), rusty::Option<rusty::detail::associated_item_t<I>>(std::move(elt))); });
        }
    };

    /// An iterator for the elements in a single chunk.
    ///
    /// Iterator element type is `I::Item`.
    template<typename I>
    struct Chunk {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        const IntoChunks<I>& parent;
        size_t index;
        rusty::Option<rusty::detail::associated_item_t<I>> first;
        Chunk(const IntoChunks<I>& parent_init, size_t index_init, rusty::Option<rusty::detail::associated_item_t<I>> first_init) : parent(parent_init), index(std::move(index_init)), first(std::move(first_init)) {}
        Chunk(const Chunk&) = default;
        Chunk(Chunk&& other) noexcept : parent(other.parent), index(std::move(other.index)), first(std::move(other.first)) {
            if (rusty::mem::consume_forgotten_address(&other)) {
                this->rusty_mark_forgotten();
                other.rusty_mark_forgotten();
            } else {
                other.rusty_mark_forgotten();
            }
        }
        Chunk& operator=(const Chunk&) = default;
        Chunk& operator=(Chunk&& other) noexcept {
            if (this == &other) {
                return *this;
            }
            this->~Chunk();
            new (this) Chunk(std::move(other));
            return *this;
        }
        void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


        ~Chunk() noexcept(false) {
            if (rusty::mem::consume_forgotten_address(this)) { return; }
            this->parent.drop_group(this->index);
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            if (true) {
                auto elt = this->first.take();
                return elt;
            }
            return this->parent.step(this->index);
        }
    };

}

namespace iter_index {
    namespace private_iter_index {}

    namespace private_iter_index {
    }
    template<typename I, typename R>
    typename R::Output get(I iter, R index);



    namespace private_iter_index {



    }


}

namespace minmax {

    template<typename T>
    struct MinMaxResult;
    template<typename I, typename K, typename F, typename L>
    MinMaxResult<rusty::detail::associated_item_t<I>> minmax_impl(I it, F key_for, L lt);

    // Algebraic data type
    template<typename T>
    struct MinMaxResult_NoElements {};
    template<typename T>
    struct MinMaxResult_OneElement {
        T _0;
    };
    template<typename T>
    struct MinMaxResult_MinMax {
        T _0;
        T _1;
    };
    template<typename T>
    MinMaxResult_NoElements<T> NoElements();
    template<typename T>
    MinMaxResult_OneElement<T> OneElement(T _0);
    template<typename T>
    MinMaxResult_MinMax<T> MinMax(T _0, T _1);
    template<typename T>
    struct MinMaxResult : std::variant<MinMaxResult_NoElements<T>, MinMaxResult_OneElement<T>, MinMaxResult_MinMax<T>> {
        using variant = std::variant<MinMaxResult_NoElements<T>, MinMaxResult_OneElement<T>, MinMaxResult_MinMax<T>>;
        using variant::variant;
        static MinMaxResult<T> NoElements() { return MinMaxResult<T>{MinMaxResult_NoElements<T>{}}; }
        static MinMaxResult<T> OneElement(T _0) { return MinMaxResult<T>{MinMaxResult_OneElement<T>{std::forward<decltype(_0)>(_0)}}; }
        static MinMaxResult<T> MinMax(T _0, T _1) { return MinMaxResult<T>{MinMaxResult_MinMax<T>{std::forward<decltype(_0)>(_0), std::forward<decltype(_1)>(_1)}}; }


        MinMaxResult<T> clone() const {
            return [&]() -> MinMaxResult<T> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return MinMaxResult<T>{MinMaxResult_NoElements<T>{}}; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return MinMaxResult<T>{MinMaxResult_OneElement<T>{rusty::clone(__self_0)}}; } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& __self_1 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return MinMaxResult<T>{MinMaxResult_MinMax<T>{rusty::clone(__self_0), rusty::clone(__self_1)}}; } return [&]() -> MinMaxResult<T> { rusty::intrinsics::unreachable(); }(); }();
        }
        bool operator==(const MinMaxResult<T>& other) const {
            const auto __self_discr = rusty::intrinsics::discriminant_value((*this));
            const auto __arg1_discr = rusty::intrinsics::discriminant_value(other);
            return (__self_discr == __arg1_discr) && [&]() -> bool { auto&& _m0 = (*this); auto&& _m1 = other; if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0))._0); auto&& __arg1_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))._0); return __self_0 == __arg1_0; } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0))._0); auto&& __self_1 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0))._1); auto&& __arg1_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))._0); auto&& __arg1_1 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))._1); return (__self_0 == __arg1_0) && (__self_1 == __arg1_1); } if (true) { return true; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
        }
        void assert_receiver_is_total_eq() const {
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return [&]() -> rusty::fmt::Result { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return std::conditional_t<true, rusty::fmt::Formatter, T>::write_str(f, "NoElements"); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return std::conditional_t<true, rusty::fmt::Formatter, T>::debug_tuple_field1_finish(f, "OneElement", __self_0); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& __self_1 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return std::conditional_t<true, rusty::fmt::Formatter, T>::debug_tuple_field2_finish(f, "MinMax", __self_0, __self_1); } return [&]() -> rusty::fmt::Result { rusty::intrinsics::unreachable(); }(); }();
        }
        rusty::Option<std::tuple<T, T>> into_option() {
            return [&]() -> rusty::Option<std::tuple<T, T>> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return rusty::Option<std::tuple<T, T>>(rusty::None); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& x = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return rusty::Option<std::tuple<T, T>>(std::make_tuple(rusty::clone(x), x)); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& x = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& y = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return rusty::Option<std::tuple<T, T>>(std::make_tuple(x, y)); } return [&]() -> rusty::Option<std::tuple<T, T>> { rusty::intrinsics::unreachable(); }(); }();
        }
    };
    template<typename T>
    MinMaxResult_NoElements<T> NoElements() { return MinMaxResult_NoElements<T>{};  }
    template<typename T>
    MinMaxResult_OneElement<T> OneElement(T _0) { return MinMaxResult_OneElement<T>{std::forward<T>(_0)};  }
    template<typename T>
    MinMaxResult_MinMax<T> MinMax(T _0, T _1) { return MinMaxResult_MinMax<T>{std::forward<T>(_0), std::forward<T>(_1)};  }

    /// Implementation guts for `minmax` and `minmax_by_key`.
    template<typename I, typename K, typename F, typename L>
    MinMaxResult<rusty::detail::associated_item_t<I>> minmax_impl(I it, F key_for, L lt) {
        auto [min, max, min_key, max_key] = rusty::detail::deref_if_pointer_like([&]() { auto&& _m = it.next(); if (_m.is_none()) { return MinMaxResult<rusty::detail::associated_item_t<I>>{MinMaxResult_NoElements<rusty::detail::associated_item_t<I>>{}}; } if (_m.is_some()) { auto&& _mv1 = _m.unwrap(); auto&& x = rusty::detail::deref_if_pointer(_mv1); return ({ auto&& _m = it.next(); std::optional<std::remove_cvref_t<decltype(([&]() -> decltype(auto) { auto _mv = _m.unwrap();
auto&& y = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
return ([&]() { auto xk = key_for(x);
auto yk = key_for(y);
return (!lt(y, x, yk, xk) ? std::make_tuple(std::move(x), y, std::move(xk), std::move(yk)) : std::make_tuple(y, std::move(x), std::move(yk), std::move(xk))); }()); })())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& y = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move([&]() { auto xk = key_for(x);
auto yk = key_for(y);
return (!lt(y, x, yk, xk) ? std::make_tuple(std::move(x), y, std::move(xk), std::move(yk)) : std::make_tuple(y, std::move(x), std::move(yk), std::move(xk))); }())); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return MinMaxResult<rusty::detail::associated_item_t<I>>{MinMaxResult_OneElement<rusty::detail::associated_item_t<I>>{std::move(x)}}; } std::move(_match_value).value(); }); } rusty::intrinsics::unreachable(); }());
        while (true) {
            auto&& _m = it.next();
            std::optional<std::remove_cvref_t<decltype((_m.unwrap()))>> _match_value;
            {
                if (_m.is_none()) {
                    break;
                }
                if (!(_m.is_some())) { rusty::intrinsics::unreachable(); }
                auto _mv = _m.unwrap();
                auto&& x = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
                _match_value.emplace(x);
            }
            const auto first = std::move(_match_value).value();
            auto&& _m_shadow1 = it.next();
            std::optional<std::remove_cvref_t<decltype((_m_shadow1.unwrap()))>> _match_value_shadow1;
            {
                if (_m_shadow1.is_none()) {
                    const auto first_key = key_for(first);
                    if (lt(first, min, first_key, min_key)) {
                        min = std::move(first);
                    } else if (!lt(first, max, first_key, max_key)) {
                        max = std::move(first);
                    }
                    break;
                }
                if (!(_m_shadow1.is_some())) { rusty::intrinsics::unreachable(); }
                auto _mv_shadow1 = _m_shadow1.unwrap();
                auto&& x = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv_shadow1));
                _match_value_shadow1.emplace(x);
            }
            const auto second = std::move(_match_value_shadow1).value();
            const auto first_key_shadow1 = key_for(first);
            const auto second_key = key_for(second);
            if (!lt(second, first, second_key, first_key_shadow1)) {
                if (lt(first, min, first_key_shadow1, min_key)) {
                    min = std::move(first);
                    min_key = std::move(first_key_shadow1);
                }
                if (!lt(second, max, second_key, max_key)) {
                    max = std::move(second);
                    max_key = std::move(second_key);
                }
            } else {
                if (lt(second, min, second_key, min_key)) {
                    min = std::move(second);
                    min_key = std::move(second_key);
                }
                if (!lt(first, max, first_key_shadow1, max_key)) {
                    max = std::move(first);
                    max_key = std::move(first_key_shadow1);
                }
            }
        }
        return MinMaxResult<rusty::detail::associated_item_t<I>>{MinMaxResult_MinMax<rusty::detail::associated_item_t<I>>{std::move(min), std::move(max)}};
    }

}

namespace next_array {
    namespace test {}

    template<typename T, size_t N>
    struct ArrayBuilder;
    namespace test {
        using ::next_array::ArrayBuilder;
        void zero_len_take();
        void zero_len_push();
        void push_4();
        void tracked_drop();
    }
    template<typename T>
    std::span<T> slice_assume_init_mut(std::span<rusty::MaybeUninit<T>> slice);
    template<typename I, size_t N>
    rusty::Option<std::array<rusty::detail::associated_item_t<I>, rusty::sanitize_array_capacity<N>()>> next_array(I& it);

    namespace mem = rusty::mem;
    using ::rusty::MaybeUninit;

    /// An array of at most `N` elements.
    template<typename T, size_t N>
    struct ArrayBuilder {
        /// The (possibly uninitialized) elements of the `ArrayBuilder`.
        ///
        /// # Safety
        ///
        /// The elements of `arr[..len]` are valid `T`s.
        std::array<rusty::MaybeUninit<T>, rusty::sanitize_array_capacity<N>()> arr;
        /// The number of leading elements of `arr` that are valid `T`s, len <= N.
        size_t len;
        ArrayBuilder(std::array<rusty::MaybeUninit<T>, rusty::sanitize_array_capacity<N>()> arr_init, size_t len_init) : arr(std::move(arr_init)), len(std::move(len_init)) {}
        ArrayBuilder(const ArrayBuilder&) = default;
        ArrayBuilder(ArrayBuilder&& other) noexcept : arr(std::move(other.arr)), len(std::move(other.len)) {
            if (rusty::mem::consume_forgotten_address(&other)) {
                this->rusty_mark_forgotten();
                other.rusty_mark_forgotten();
            } else {
                other.rusty_mark_forgotten();
            }
        }
        ArrayBuilder& operator=(const ArrayBuilder&) = default;
        ArrayBuilder& operator=(ArrayBuilder&& other) noexcept {
            if (this == &other) {
                return *this;
            }
            this->~ArrayBuilder();
            new (this) ArrayBuilder(std::move(other));
            return *this;
        }
        void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


        static ArrayBuilder<T, N> new_() {
            return ArrayBuilder<T, N>(rusty::map([](auto _seed) { std::array<std::tuple<>, rusty::sanitize_array_capacity<N>()> _repeat{}; _repeat.fill(_seed); return _repeat; }(std::make_tuple()), [&](auto _closure_wild0) -> rusty::MaybeUninit<T> { return rusty::MaybeUninit<T>::uninit(); }), static_cast<size_t>(0));
        }
        void push(T value) {
            auto& place = this->arr.at(this->len);
            place = MaybeUninit<T>::new_(std::move(value));
            [&]() { static_cast<void>(this->len += 1); return std::make_tuple(); }();
        }
        rusty::Option<std::array<T, rusty::sanitize_array_capacity<N>()>> take() {
            if (this->len == N) {
                this->len = static_cast<size_t>(0);
                auto arr = rusty::mem::replace(this->arr, rusty::map([](auto _seed) { std::array<std::tuple<>, rusty::sanitize_array_capacity<N>()> _repeat{}; _repeat.fill(_seed); return _repeat; }(std::make_tuple()), [&](auto _closure_wild0) -> rusty::MaybeUninit<T> { return rusty::MaybeUninit<T>::uninit(); }));
                return rusty::Option<std::array<T, rusty::sanitize_array_capacity<N>()>>(rusty::map(arr, [&](auto&& v) {
// @unsafe
{
    return v.assume_init();
}
}));
            } else {
                return rusty::Option<std::array<T, rusty::sanitize_array_capacity<N>()>>(rusty::None);
            }
        }
        std::span<T> as_mut() {
            std::span<rusty::MaybeUninit<T>> valid = rusty::slice_to(this->arr, this->len);
            // @unsafe
            {
                return slice_assume_init_mut(valid);
            }
        }
        ~ArrayBuilder() noexcept(false) {
            if (rusty::mem::consume_forgotten_address(this)) { return; }
            // @unsafe
            {
                rusty::ptr::drop_in_place(this->as_mut());
            }
        }
    };

    namespace test {

        using ::next_array::ArrayBuilder;
        void zero_len_take();
        void zero_len_push();
        void push_4();
        void tracked_drop();

        using ::next_array::ArrayBuilder;


        // Rust-only libtest metadata const skipped: zero_len_take (marker: next_array::test::zero_len_take, should_panic: no)


        // Rust-only libtest metadata const skipped: zero_len_push (marker: next_array::test::zero_len_push, should_panic: yes)


        // Rust-only libtest metadata const skipped: push_4 (marker: next_array::test::push_4, should_panic: no)


        // Rust-only libtest metadata const skipped: tracked_drop (marker: next_array::test::tracked_drop, should_panic: no)

    }

}

namespace peeking_take_while {

    template<typename I, typename F>
    struct PeekingTakeWhile;
    template<typename I, typename F>
    PeekingTakeWhile<I, F> peeking_take_while(I& iter, F f);

    using adaptors::PutBack;

    using put_back_n_impl::PutBackN;

    using repeatn::RepeatN;



    /// An iterator adaptor that takes items while a closure returns `true`.
    ///
    /// See [`.peeking_take_while()`](crate::Itertools::peeking_take_while)
    /// for more information.
    template<typename I, typename F>
    struct PeekingTakeWhile {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        I& iter;
        F f;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("PeekingTakeWhile").field("iter", &this->iter).finish();
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            return ([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::peeking_take_while::rusty_ext::peeking_next(std::forward<decltype(__self)>(__self), &this->f); }) { return ::peeking_take_while::rusty_ext::peeking_next(std::forward<decltype(__self)>(__self), &this->f); } else { return ::peeking_take_while::rusty_ext::peeking_next(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self)), &this->f); } })(this->iter);
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return std::make_tuple(static_cast<size_t>(0), rusty::size_hint(this->iter)._1);
        }
        template<typename G>
        auto peeking_next(G g) {
            auto& f = this->f;
            return ([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::peeking_take_while::rusty_ext::peeking_next(std::forward<decltype(__self)>(__self), [&](auto&& r) { return f(std::move(r)) && g(std::move(r)); }); }) { return ::peeking_take_while::rusty_ext::peeking_next(std::forward<decltype(__self)>(__self), [&](auto&& r) { return f(std::move(r)) && g(std::move(r)); }); } else { return ::peeking_take_while::rusty_ext::peeking_next(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self)), [&](auto&& r) { return f(std::move(r)) && g(std::move(r)); }); } })(this->iter);
        }
    };

}

namespace process_results_impl {

    template<typename I, typename E>
    struct ProcessResults;
    template<typename I, typename F, typename T, typename E, typename R>
    rusty::Result<R, E> process_results(I iterable, F processor);

    /// An iterator that produces only the `T` values as long as the
    /// inner iterator produces `Ok(T)`.
    ///
    /// Used by [`process_results`](crate::process_results), see its docs
    /// for more information.
    template<typename I, typename E>
    struct ProcessResults {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        rusty::Result<std::tuple<>, E>& error;
        I iter;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field2_finish(f, "ProcessResults", "error", &this->error, "iter", &this->iter);
        }
        template<typename T>
        rusty::Option<T> next_body(rusty::Option<rusty::Result<T, E>> item) {
            return [&]() -> rusty::Option<T> { auto&& _m = item; if (_m.is_some()) { auto&& _mv0 = std::as_const(_m).unwrap(); if (rusty::detail::deref_if_pointer(_mv0).is_ok()) { auto&& x = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_mv0)).unwrap()); return rusty::Option<T>(x); } } if (_m.is_some()) { auto&& _mv1 = std::as_const(_m).unwrap(); if (rusty::detail::deref_if_pointer(_mv1).is_err()) { auto&& e = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_mv1)).unwrap_err()); return [&]() -> rusty::Option<T> { this->error = rusty::Err(e);
return rusty::Option<T>(rusty::None); }(); } } if (_m.is_none()) { return rusty::Option<T>(rusty::None); } return [&]() -> rusty::Option<T> { rusty::intrinsics::unreachable(); }(); }();
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename T>
        auto next() {
            auto item = this->iter.next();
            return this->next_body(std::move(item));
        }
        template<typename T>
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return std::make_tuple(static_cast<size_t>(0), rusty::size_hint(this->iter)._1);
        }
        template<typename B, typename F, typename T>
        B fold(B init, F f) {
            const auto error = this->error;
            return rusty::try_fold(this->iter, std::move(init), [&](auto&& acc, auto&& opt) { return [&]() { auto&& _m = opt; if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& x = rusty::detail::deref_if_pointer(_mv0); return rusty::Ok(f(std::move(acc), std::move(x))); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& e = rusty::detail::deref_if_pointer(_mv1); return [&]() { rusty::detail::deref_if_pointer_like(error) = rusty::Err(std::move(e));
return rusty::Err(std::move(acc)); }(); } rusty::intrinsics::unreachable(); }(); }).unwrap_or_else([&](auto&& e) { return e; });
        }
        template<typename T>
        auto next_back() {
            auto item = this->iter.next_back();
            return this->next_body(std::move(item));
        }
        template<typename B, typename F, typename T>
        B rfold(B init, F f) {
            const auto error = this->error;
            return this->iter.try_rfold(std::move(init), [&](auto&& acc, auto&& opt) { return [&]() { auto&& _m = opt; if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& x = rusty::detail::deref_if_pointer(_mv0); return rusty::Ok(f(std::move(acc), std::move(x))); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& e = rusty::detail::deref_if_pointer(_mv1); return [&]() { rusty::detail::deref_if_pointer_like(error) = rusty::Err(std::move(e));
return rusty::Err(std::move(acc)); }(); } rusty::intrinsics::unreachable(); }(); }).unwrap_or_else([&](auto&& e) { return e; });
        }
    };

}

namespace rciter_impl {

    template<typename I>
    struct RcIter;
    template<typename I>
    RcIter<typename I::IntoIter> rciter(I iterable);

    using ::rusty::Rc;

    using ::rusty::RefCell;


    /// A wrapper for `Rc<RefCell<I>>`, that implements the `Iterator` trait.
    template<typename I>
    struct RcIter {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        /// The boxed iterator.
        rusty::Rc<rusty::RefCell<I>> rciter;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field1_finish(f, "RcIter", "rciter", &this->rciter);
        }
        RcIter<I> clone() const {
            return RcIter<I>{.rciter = rusty::clone(this->rciter)};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A>
        auto next() {
            return this->rciter.borrow_mut().next();
        }
        template<typename A>
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return std::make_tuple(static_cast<size_t>(0), rusty::size_hint(this->rciter.borrow())._1);
        }
        auto next_back() {
            return this->rciter.borrow_mut().next_back();
        }
    };

}

namespace repeatn {

    template<typename A>
    struct RepeatN;
    template<typename A>
    RepeatN<A> repeat_n(A element, size_t n);


    /// An iterator that produces *n* repetitions of an element.
    ///
    /// See [`repeat_n()`](crate::repeat_n) for more information.
    template<typename A>
    struct RepeatN {
        using Item = A;
        rusty::Option<A> elt;
        size_t n;

        template<typename F, typename T>
        rusty::Option<Item> peeking_next(F accept) {
            using namespace peeking_take_while;
            const auto r = RUSTY_TRY_OPT(this->elt.as_ref());
            if (!accept(std::move(r))) {
                return rusty::Option<A>(rusty::None);
            }
            return this->next();
        }
        RepeatN<A> clone() const {
            using namespace peeking_take_while;
            return RepeatN<A>{.elt = rusty::clone(this->elt), .n = rusty::clone(this->n)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            using namespace peeking_take_while;
            return std::conditional_t<true, rusty::fmt::Formatter, A>::debug_struct_field2_finish(f, "RepeatN", "elt", &this->elt, "n", &this->n);
        }
        rusty::Option<Item> next() {
            using namespace peeking_take_while;
            if (this->n > 1) {
                [&]() { static_cast<void>(this->n -= 1); return std::make_tuple(); }();
                return this->elt.as_ref().cloned();
            } else {
                this->n = static_cast<size_t>(0);
                return this->elt.take();
            }
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            using namespace peeking_take_while;
            return std::make_tuple(this->n, rusty::Option<size_t>(this->n));
        }
        template<typename B, typename F>
        B fold(B init, F f) {
            using namespace peeking_take_while;
            return [&]() -> B { auto&& _m = (*this); if (rusty::detail::deref_if_pointer(_m.elt).is_some()) { auto&& elt = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m.elt)).unwrap()); auto&& n = rusty::detail::deref_if_pointer(_m.n); return [&]() -> B { if (true) {
    if (!(n > 0)) {
        [&]() -> B { rusty::panicking::panic("assertion failed: n > 0"); }();
    }
}
init = rusty::fold(rusty::map((rusty::range(1, n)), [&](auto _closure_wild0) { return rusty::clone(elt); }), std::move(init), &f);
return f(std::move(init), elt); }(); } if (true) { return init; } return [&]() -> B { rusty::intrinsics::unreachable(); }(); }();
        }
        rusty::Option<Item> next_back() {
            using namespace peeking_take_while;
            return this->next();
        }
        template<typename B, typename F>
        B rfold(B init, F f) {
            using namespace peeking_take_while;
            return rusty::fold((*this), std::move(init), std::move(f));
        }
    };

}

namespace size_hint {

    SizeHint add(const auto& a, const auto& b);
    SizeHint add_scalar(const auto& sh, size_t x);
    SizeHint sub_scalar(const auto& sh, size_t x);
    SizeHint mul(const auto& a, const auto& b);
    SizeHint mul_scalar(const auto& sh, size_t x);
    SizeHint max(const auto& a, const auto& b);
    SizeHint min(const auto& a, const auto& b);
    void mul_size_hints();

    namespace cmp = rusty::cmp;



    // Rust-only libtest metadata const skipped: mul_size_hints (marker: size_hint::mul_size_hints, should_panic: no)

    /// Add `SizeHint` correctly.
    SizeHint add(const auto& a, const auto& b) {
        auto min_shadow1 = rusty::saturating_add(std::get<0>(a), rusty::detail::deref_if_pointer(std::move(std::get<0>(b))));
        auto max_shadow1 = [&]() -> rusty::Option<size_t> { auto&& _m0 = std::get<1>(a); auto&& _m1 = std::get<1>(b); if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some()) { auto&& x = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& y = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); return [&]() { auto&& _checked_lhs = x; return rusty::checked_add(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(y)); }(); } if (true) { return rusty::Option<size_t>(rusty::None); } return [&]() -> rusty::Option<size_t> { rusty::intrinsics::unreachable(); }(); }();
        return std::make_tuple(std::move(min_shadow1), std::move(max_shadow1));
    }

    /// Add `x` correctly to a `SizeHint`.
    SizeHint add_scalar(const auto& sh, size_t x) {
        auto [low, hi] = rusty::detail::deref_if_pointer_like(sh);
        low = rusty::saturating_add(low, rusty::detail::deref_if_pointer(std::move(x)));
        hi = hi.and_then([&](auto&& elt) { return [&]() { auto&& _checked_lhs = elt; return rusty::checked_add(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(std::move(x))); }(); });
        return std::make_tuple(std::move(low), std::move(hi));
    }

    /// Subtract `x` correctly from a `SizeHint`.
    SizeHint sub_scalar(const auto& sh, size_t x) {
        auto [low, hi] = rusty::detail::deref_if_pointer_like(sh);
        low = rusty::saturating_sub(low, rusty::detail::deref_if_pointer(std::move(x)));
        hi = hi.map([&](auto&& elt) -> size_t { return rusty::saturating_sub(elt, rusty::detail::deref_if_pointer(std::move(x))); });
        return std::make_tuple(std::move(low), std::move(hi));
    }

    /// Multiply `SizeHint` correctly
    SizeHint mul(const auto& a, const auto& b) {
        auto low = rusty::saturating_mul(std::get<0>(a), rusty::detail::deref_if_pointer(std::move(std::get<0>(b))));
        auto hi = [&]() -> rusty::Option<size_t> { auto&& _m0 = std::get<1>(a); auto&& _m1 = std::get<1>(b); if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some()) { auto&& x = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& y = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); return [&]() { auto&& _checked_lhs = x; return rusty::checked_mul(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(y)); }(); } if (((rusty::detail::deref_if_pointer(_m0).is_some() && std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap() == 0) && _m1.is_none() || _m0.is_none() && (rusty::detail::deref_if_pointer(_m1).is_some() && std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap() == 0))) { return rusty::Option<size_t>(static_cast<size_t>(0)); } if (true) { return rusty::Option<size_t>(rusty::None); } return [&]() -> rusty::Option<size_t> { rusty::intrinsics::unreachable(); }(); }();
        return std::make_tuple(std::move(low), std::move(hi));
    }

    /// Multiply `x` correctly with a `SizeHint`.
    SizeHint mul_scalar(const auto& sh, size_t x) {
        auto [low, hi] = rusty::detail::deref_if_pointer_like(sh);
        low = rusty::saturating_mul(low, rusty::detail::deref_if_pointer(std::move(x)));
        hi = hi.and_then([&](auto&& elt) { return [&]() { auto&& _checked_lhs = elt; return rusty::checked_mul(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(std::move(x))); }(); });
        return std::make_tuple(std::move(low), std::move(hi));
    }

    /// Return the maximum
    SizeHint max(const auto& a, const auto& b) {
        auto [a_lower, a_upper] = rusty::detail::deref_if_pointer_like(a);
        auto [b_lower, b_upper] = rusty::detail::deref_if_pointer_like(b);
        auto lower = rusty::cmp::max(std::move(a_lower), std::move(b_lower));
        auto upper = [&]() -> rusty::Option<size_t> { auto&& _m0 = a_upper; auto&& _m1 = b_upper; if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some()) { auto&& x = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& y = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); return rusty::Option<size_t>(rusty::cmp::max(x, y)); } if (true) { return rusty::Option<size_t>(rusty::None); } return [&]() -> rusty::Option<size_t> { rusty::intrinsics::unreachable(); }(); }();
        return std::make_tuple(std::move(lower), std::move(upper));
    }

    /// Return the minimum
    SizeHint min(const auto& a, const auto& b) {
        auto [a_lower, a_upper] = rusty::detail::deref_if_pointer_like(a);
        auto [b_lower, b_upper] = rusty::detail::deref_if_pointer_like(b);
        auto lower = rusty::cmp::min(std::move(a_lower), std::move(b_lower));
        auto upper = [&]() { auto&& _m0 = a_upper; auto&& _m1 = b_upper; if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some()) { auto&& u1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& u2 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); return rusty::Some(rusty::cmp::min(u1, u2)); } if (true) { return a_upper.or_(std::move(b_upper)); } rusty::intrinsics::unreachable(); }();
        return std::make_tuple(std::move(lower), std::move(upper));
    }

    void mul_size_hints() {
        {
            auto&& _m0_tmp = mul(std::make_tuple(static_cast<size_t>(3), rusty::Option<size_t>(static_cast<size_t>(4))), std::make_tuple(static_cast<size_t>(3), rusty::Option<size_t>(static_cast<size_t>(4))));
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = std::make_tuple(static_cast<size_t>(9), rusty::Option<size_t>(static_cast<size_t>(16)));
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
            auto&& _m0_tmp = mul(std::make_tuple(static_cast<size_t>(3), rusty::Option<size_t>(static_cast<size_t>(4))), std::make_tuple(std::numeric_limits<size_t>::max(), rusty::Option<size_t>(rusty::None)));
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = std::make_tuple(std::numeric_limits<size_t>::max(), rusty::Option<size_t>(rusty::None));
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
            auto&& _m0_tmp = mul(std::make_tuple(static_cast<size_t>(3), rusty::Option<size_t>(rusty::None)), std::make_tuple(static_cast<size_t>(0), rusty::Option<size_t>(static_cast<size_t>(0))));
            auto _m0 = &_m0_tmp;
            auto&& _m1_tmp = std::make_tuple(static_cast<size_t>(0), rusty::Option<size_t>(static_cast<size_t>(0)));
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

namespace adaptors {
    namespace coalesce_tests {}
    namespace map {}
    namespace multi_product {}

    template<typename I, typename J>
    struct Interleave;
    template<typename I, typename J>
    struct InterleaveShortest;
    template<typename I>
    struct PutBack;
    template<typename I, typename J>
    struct Product;
    template<typename I, typename F>
    struct Batching;
    template<typename I, typename F>
    struct TakeWhileRef;
    template<typename I>
    struct WhileSome;
    template<typename I, typename T>
    struct TupleCombinations;
    template<typename I>
    struct Tuple1Combination;
    template<typename I>
    struct Tuple2Combination;
    template<typename I>
    struct Tuple3Combination;
    template<typename I>
    struct Tuple4Combination;
    template<typename I>
    struct Tuple5Combination;
    template<typename I>
    struct Tuple6Combination;
    template<typename I>
    struct Tuple7Combination;
    template<typename I>
    struct Tuple8Combination;
    template<typename I>
    struct Tuple9Combination;
    template<typename I>
    struct Tuple10Combination;
    template<typename I>
    struct Tuple11Combination;
    template<typename I>
    struct Tuple12Combination;
    template<typename I, typename F>
    struct FilterOk;
    template<typename I, typename F>
    struct FilterMapOk;
    template<typename I, typename F>
    struct Positions;
    template<typename I, typename F>
    struct Update;
    namespace coalesce_tests {
        template<typename I, typename F, typename C>
        struct CoalesceBy;
        struct NoCount;
        struct WithCount;
        template<typename DP>
        struct DedupPred2CoalescePred;
        struct DedupEq;
        template<typename DP>
        struct DedupPredWithCount2CoalescePred;
        template<typename I, typename F>
        Coalesce<I, F> coalesce(I iter, F f);
        template<typename I, typename Pred>
        DedupBy<I, Pred> dedup_by(I iter, Pred dedup_pred);
        template<typename I>
        Dedup<I> dedup(I iter);
        template<typename I, typename Pred>
        DedupByWithCount<I, Pred> dedup_by_with_count(I iter, Pred dedup_pred);
        template<typename I>
        DedupWithCount<I> dedup_with_count(I iter);
    }
    namespace map {
        template<typename I, typename F>
        struct MapSpecialCase;
        template<typename F>
        struct MapSpecialCaseFnOk;
        template<typename U>
        struct MapSpecialCaseFnInto;
        template<typename I, typename F, typename T, typename U, typename E>
        MapOk<I, F> map_ok(I iter, F f);
        template<typename I, typename R>
        MapInto<I, R> map_into(I iter);
    }
    namespace multi_product {
        template<typename I>
        struct MultiProductIter;
        template<typename I>
        struct MultiProductInner;
        template<typename I>
        struct MultiProduct;
        template<typename H>
        MultiProduct<typename rusty::detail::associated_item_t<H>::IntoIter> multi_cartesian_product(H iters);
    }
    template<typename I, typename J>
    Interleave<typename I::IntoIter, typename J::IntoIter> interleave(I i, J j);
    template<typename I, typename J>
    InterleaveShortest<I, J> interleave_shortest(I i, J j);
    template<typename I>
    PutBack<typename I::IntoIter> put_back(I iterable);
    template<typename I, typename J>
    Product<I, J> cartesian_product(I i, J j);
    template<typename I, typename F>
    Batching<I, F> batching(I iter, F f);
    template<typename I, typename F>
    TakeWhileRef<I, F> take_while_ref(I& iter, F f);
    template<typename I>
    WhileSome<I> while_some(I iter);
    template<typename T, typename I>
    TupleCombinations<I, T> tuple_combinations(I iter);
    rusty::Option<size_t> checked_binomial(size_t n, size_t k);
    void test_checked_binomial();
    template<typename I, typename F, typename T, typename E>
    FilterOk<I, F> filter_ok(I iter, F f);
    template<typename T, typename E>
    rusty::Option<rusty::Result<T, E>> transpose_result(rusty::Result<rusty::Option<T>, E> result);
    template<typename I, typename F, typename T, typename U, typename E>
    FilterMapOk<I, F> filter_map_ok(I iter, F f);
    template<typename I, typename F>
    Positions<I, F> positions(I iter, F f);
    template<typename I, typename F>
    Update<I, F> update(I iter, F f);

    namespace size_hint = ::size_hint;
    using size_hint::SizeHint;

    namespace coalesce_tests {

        template<typename I, typename F, typename C>
        struct CoalesceBy;
        struct NoCount;
        struct WithCount;
        template<typename DP>
        struct DedupPred2CoalescePred;
        struct DedupEq;
        template<typename DP>
        struct DedupPredWithCount2CoalescePred;
        template<typename I, typename F>
        Coalesce<I, F> coalesce(I iter, F f);
        template<typename I, typename Pred>
        DedupBy<I, Pred> dedup_by(I iter, Pred dedup_pred);
        template<typename I>
        Dedup<I> dedup(I iter);
        template<typename I, typename Pred>
        DedupByWithCount<I, Pred> dedup_by_with_count(I iter, Pred dedup_pred);
        template<typename I>
        DedupWithCount<I> dedup_with_count(I iter);

        namespace fmt = rusty::fmt;


        namespace size_hint = ::size_hint;

        template<typename I, typename F, typename C>
        struct CoalesceBy {
            // Rust-only dependent associated type alias skipped in constrained mode: Item
            I iter;
            /// `last` is `None` while no item have been taken out of `iter` (at definition).
            /// Then `last` will be `Some(Some(item))` until `iter` is exhausted,
            /// in which case `last` will be `Some(None)`.
            rusty::Option<rusty::Option<typename C::CItem>> last;
            F f;

            CoalesceBy<I, F, C> clone() const {
                return CoalesceBy<I, F, C>{.iter = rusty::clone(this->iter), .last = rusty::clone(this->last), .f = rusty::clone(this->f)};
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
                return f.debug_struct("CoalesceBy").field("iter", &this->iter).field("last", &this->last).finish();
            }
            // Rust-only dependent associated type alias skipped in constrained mode: Item
            auto next() {
                auto&& _let_pat = (*this);
                auto&& iter = rusty::detail::deref_if_pointer(_let_pat.iter);
                auto&& last = rusty::detail::deref_if_pointer(_let_pat.last);
                auto&& f = rusty::detail::deref_if_pointer(_let_pat.f);
                const auto init = RUSTY_TRY_OPT([&]() { auto&& _m = last; if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& elt = rusty::detail::deref_if_pointer(_mv0); return elt.take(); } if (_m.is_none()) { return [&]() { rusty::detail::deref_if_pointer_like(last) = rusty::Some(rusty::None);
return iter.next().map(C::new_); }(); } rusty::intrinsics::unreachable(); }());
                return rusty::Some(rusty::try_fold(iter, std::move(init), [&](auto&& accum, auto&& next) { return [&]() { auto&& _m = ([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::adaptors::coalesce_tests::rusty_ext::coalesce_pair(std::forward<decltype(__self)>(__self), std::move(accum), std::move(next)); }) { return ::adaptors::coalesce_tests::rusty_ext::coalesce_pair(std::forward<decltype(__self)>(__self), std::move(accum), std::move(next)); } else { return ::adaptors::coalesce_tests::rusty_ext::coalesce_pair(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self)), std::move(accum), std::move(next)); } })(f); if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& joined = rusty::detail::deref_if_pointer(_mv0); return rusty::Ok(std::move(joined)); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& last_ = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_mv1))); auto&& next_ = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_mv1))); return [&]() { rusty::detail::deref_if_pointer_like(last) = rusty::Some(rusty::Some(std::move(next_)));
return rusty::Err(std::move(last_)); }(); } rusty::intrinsics::unreachable(); }(); }).unwrap_or_else([&](auto&& x) { return x; }));
            }
            std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
                auto [low, hi] = rusty::detail::deref_if_pointer_like(size_hint::add_scalar(rusty::size_hint(this->iter), static_cast<size_t>([&]() -> bool { auto&& _m = this->last; if (_m.is_some()) { auto&& _mv0 = std::as_const(_m).unwrap(); if (rusty::detail::deref_if_pointer(_mv0).is_some()) { return true; } } if (true) { return false; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }())));
                return std::make_tuple(static_cast<size_t>((low > 0)), std::move(hi));
            }
            template<typename Acc, typename FnAcc>
            Acc fold(Acc acc, FnAcc fn_acc) {
                auto&& _let_pat = (*this);
                auto iter = rusty::detail::deref_if_pointer(_let_pat.iter);
                auto&& last = rusty::detail::deref_if_pointer(_let_pat.last);
                auto f = rusty::detail::deref_if_pointer(_let_pat.f);
                if (auto&& _iflet_scrutinee = last.unwrap_or_else([&]() { return iter.next().map(C::new_); }); _iflet_scrutinee.is_some()) {
                    decltype(auto) last = _iflet_scrutinee.unwrap();
                    auto [last_shadow1, acc_shadow1] = rusty::detail::deref_if_pointer_like(rusty::fold(iter, std::make_tuple(std::move(last), std::move(acc)), [&](auto&& _destruct_param0, auto&& elt) {
auto&& last = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& acc = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
return [&]() { auto&& _m = ([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::adaptors::coalesce_tests::rusty_ext::coalesce_pair(std::forward<decltype(__self)>(__self), std::move(last), std::move(elt)); }) { return ::adaptors::coalesce_tests::rusty_ext::coalesce_pair(std::forward<decltype(__self)>(__self), std::move(last), std::move(elt)); } else { return ::adaptors::coalesce_tests::rusty_ext::coalesce_pair(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self)), std::move(last), std::move(elt)); } })(f); if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& joined = rusty::detail::deref_if_pointer(_mv0); return std::make_tuple(std::move(joined), std::move(acc)); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& last_ = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_mv1))); auto&& next_ = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_mv1))); return std::make_tuple(std::move(next_), fn_acc(std::move(acc), std::move(last_))); } rusty::intrinsics::unreachable(); }();
}));
                    return fn_acc(std::move(acc_shadow1), std::move(last_shadow1));
                } else {
                    return acc;
                }
            }
        };


        struct NoCount {
            // Rust-only associated type alias with unbound generic skipped in constrained mode: CItem

            // Rust-only associated type alias with unbound generic skipped in constrained mode: CItem
            template<typename T>
            static T new_(T t);
        };

        struct WithCount {
            // Rust-only associated type alias with unbound generic skipped in constrained mode: CItem

            // Rust-only associated type alias with unbound generic skipped in constrained mode: CItem
            template<typename T>
            static std::tuple<size_t, T> new_(T t);
        };

        // Rust-only trait CountItem (Proxy facade emission skipped in module mode)



        template<typename DP>
        struct DedupPred2CoalescePred {
            DP _0;

            DedupPred2CoalescePred<DP> clone() const {
                return DedupPred2CoalescePred(rusty::clone(this->_0));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
                return f.debug_struct("DedupPred2CoalescePred").finish();
            }
            template<typename T>
            rusty::Result<T, std::tuple<T, T>> coalesce_pair(T t, T item) {
                if (([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::adaptors::coalesce_tests::rusty_ext::dedup_pair(std::forward<decltype(__self)>(__self), t, item); }) { return ::adaptors::coalesce_tests::rusty_ext::dedup_pair(std::forward<decltype(__self)>(__self), t, item); } else { return ::adaptors::coalesce_tests::rusty_ext::dedup_pair(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self)), t, item); } })(this->_0)) {
                    return rusty::Result<T, std::tuple<T, T>>::Ok(std::move(t));
                } else {
                    return rusty::Result<T, std::tuple<T, T>>::Err(std::make_tuple(std::move(t), std::move(item)));
                }
            }
        };


        struct DedupEq {

            DedupEq clone() const;
            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
            template<typename T>
            bool dedup_pair(const T& a, const T& b);
        };



        template<typename DP>
        struct DedupPredWithCount2CoalescePred {
            DP _0;

            DedupPredWithCount2CoalescePred<DP> clone() const {
                return DedupPredWithCount2CoalescePred(rusty::clone(this->_0));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
                return std::conditional_t<true, rusty::fmt::Formatter, DP>::debug_tuple_field1_finish(f, "DedupPredWithCount2CoalescePred", &this->_0);
            }
            template<typename T>
            rusty::Result<std::tuple<size_t, T>, std::tuple<std::tuple<size_t, T>, std::tuple<size_t, T>>> coalesce_pair(std::tuple<size_t, T> _arg1, T item) {
                auto&& c = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)));
                auto&& t = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_arg1)));
                if (([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::adaptors::coalesce_tests::rusty_ext::dedup_pair(std::forward<decltype(__self)>(__self), rusty::detail::deref_if_pointer_like(t), item); }) { return ::adaptors::coalesce_tests::rusty_ext::dedup_pair(std::forward<decltype(__self)>(__self), rusty::detail::deref_if_pointer_like(t), item); } else { return ::adaptors::coalesce_tests::rusty_ext::dedup_pair(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self)), rusty::detail::deref_if_pointer_like(t), item); } })(this->_0)) {
                    return rusty::Result<std::tuple<size_t, T>, std::tuple<std::tuple<size_t, T>, std::tuple<size_t, T>>>::Ok(std::make_tuple(c + 1, std::move(t)));
                } else {
                    return rusty::Result<std::tuple<size_t, T>, std::tuple<std::tuple<size_t, T>, std::tuple<size_t, T>>>::Err(std::make_tuple(std::make_tuple(std::move(c), std::move(t)), std::make_tuple(static_cast<size_t>(1), std::move(item))));
                }
            }
        };


    }

    namespace map {

        template<typename I, typename F>
        struct MapSpecialCase;
        template<typename F>
        struct MapSpecialCaseFnOk;
        template<typename U>
        struct MapSpecialCaseFnInto;
        template<typename I, typename F, typename T, typename U, typename E>
        MapOk<I, F> map_ok(I iter, F f);
        template<typename I, typename R>
        MapInto<I, R> map_into(I iter);


        using ::rusty::PhantomData;

        template<typename I, typename F>
        struct MapSpecialCase {
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
            I iter;
            F f;

            MapSpecialCase<I, F> clone() const {
                return MapSpecialCase<I, F>{.iter = rusty::clone(this->iter), .f = rusty::clone(this->f)};
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
                return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field2_finish(f, "MapSpecialCase", "iter", &this->iter, "f", &this->f);
            }
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
            template<typename R>
            auto next() {
                return this->iter.next().map([&](auto&& i) -> typename R::Out { return this->f.call(std::move(i)); });
            }
            template<typename R>
            std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
                return rusty::size_hint(this->iter);
            }
            template<typename Acc, typename Fold, typename R>
            Acc fold(Acc init, Fold fold_f) {
                auto f = std::move(this->f);
                return rusty::fold(this->iter, std::move(init), [=, f = std::move(f), fold_f = std::move(fold_f)](auto&& acc, auto&& v) mutable { return fold_f(std::move(acc), f.call(std::move(v))); });
            }
            template<typename C, typename R>
            C collect() {
                auto f = std::move(this->f);
                return C::from_iter(this->iter.map([=, f = std::move(f)](auto&& v) mutable { return f.call(std::move(v)); }));
            }
            template<typename R>
            auto next_back() {
                return this->iter.next_back().map([&](auto&& i) -> typename R::Out { return this->f.call(std::move(i)); });
            }
        };

        // Rust-only trait MapSpecialCaseFn (Proxy facade emission skipped in module mode)


        template<typename F>
        struct MapSpecialCaseFnOk {
            // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
            F _0;

            // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
            template<typename T, typename U, typename E>
            auto call(rusty::Result<T, E> t) {
                return t.map([&](auto&& v) { return this->_0(std::move(v)); });
            }
            MapSpecialCaseFnOk<F> clone() const {
                return MapSpecialCaseFnOk(rusty::clone(this->_0));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
                return f.debug_struct("MapSpecialCaseFnOk").finish();
            }
        };


        template<typename U>
        struct MapSpecialCaseFnInto {
            using Out = U;
            rusty::PhantomData<U> _0;

            template<typename T>
            Out call(T t) {
                return rusty::from_into<Out>(std::move(t));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
                return f.debug_struct("MapSpecialCaseFnInto").field("0", &this->_0).finish();
            }
            MapSpecialCaseFnInto<U> clone() const {
                return MapSpecialCaseFnInto(rusty::PhantomData<U>{});
            }
        };

    }

    namespace multi_product {

        template<typename I>
        struct MultiProductIter;
        template<typename I>
        struct MultiProductInner;
        template<typename I>
        struct MultiProduct;
        template<typename H>
        MultiProduct<typename rusty::detail::associated_item_t<H>::IntoIter> multi_cartesian_product(H iters);

        template<typename... Ts> using State = rusty::Option<Ts...>;
        // Rust-only constructor alias import: using ProductEnded = Option::None;
        // Rust-only constructor alias import: using ProductInProgress = Option::Some;

        template<typename... Ts> using CurrentItems = rusty::Option<Ts...>;
        // Rust-only constructor alias import: using NotYetPopulated = Option::None;
        // Rust-only constructor alias import: using Populated = Option::Some;

        using ::rusty::Vec;

        namespace size_hint = ::size_hint;

        /// Holds the state of a single iterator within a `MultiProduct`.
        template<typename I>
        struct MultiProductIter {
            I iter;
            I iter_orig;

            MultiProductIter<I> clone() const {
                return MultiProductIter<I>{.iter = rusty::clone(this->iter), .iter_orig = rusty::clone(this->iter_orig)};
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
                return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field2_finish(f, "MultiProductIter", "iter", &this->iter, "iter_orig", &this->iter_orig);
            }
            static MultiProductIter<I> new_(I iter) {
                return MultiProductIter<I>{.iter = rusty::clone(iter), .iter_orig = std::move(iter)};
            }
        };

        /// Internals for `MultiProduct`.
        template<typename I>
        struct MultiProductInner {
            /// Holds the iterators.
            rusty::Vec<MultiProductIter<I>> iters;
            /// Not populated at the beginning then it holds the current item of each iterator.
            rusty::Option<rusty::Vec<rusty::detail::associated_item_t<I>>> cur;

            MultiProductInner<I> clone() const {
                return MultiProductInner<I>{.iters = rusty::clone(this->iters), .cur = rusty::clone(this->cur)};
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
                return f.debug_struct("MultiProductInner").field("iters", &this->iters).field("cur", &this->cur).finish();
            }
        };

        /// An iterator adaptor that iterates over the cartesian product of
        /// multiple iterators of type `I`.
        ///
        /// An iterator element type is `Vec<I::Item>`.
        ///
        /// See [`.multi_cartesian_product()`](crate::Itertools::multi_cartesian_product)
        /// for more information.
        template<typename I>
        struct MultiProduct {
            // Rust-only dependent associated type alias skipped in constrained mode: Item
            rusty::Option<MultiProductInner<I>> _0;

            MultiProduct<I> clone() const {
                return MultiProduct(rusty::clone(this->_0));
            }
            rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
                return f.debug_struct("MultiProduct").field("0", &this->_0).finish();
            }
            // Rust-only dependent associated type alias skipped in constrained mode: Item
            auto next() {
                const auto inner = RUSTY_TRY_OPT(this->_0.as_mut());
                return [&]() { auto&& _m = &inner.cur; if (_m.is_some()) { auto&& _mv0 = std::as_const(_m).unwrap(); auto&& values = rusty::detail::deref_if_pointer(_mv0); return [&]() { if (true) {
    if (!!rusty::is_empty(inner.iters)) {
        [&]() -> rusty::Option<rusty::Vec<rusty::detail::associated_item_t<I>>> { rusty::panicking::panic("assertion failed: !inner.iters.is_empty()"); }();
    }
}
for (auto&& _for_item : rusty::for_in(rusty::rev(rusty::zip(rusty::iter_mut(inner.iters), rusty::iter_mut(values))))) {
    auto&& iter = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_for_item)));
    auto&& item = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_for_item)));
    if (auto&& _iflet_scrutinee = iter.iter.next(); _iflet_scrutinee.is_some()) {
        decltype(auto) new_ = _iflet_scrutinee.unwrap();
        rusty::detail::deref_if_pointer_like(item) = std::move(new_);
        return rusty::Some(rusty::clone(values));
    } else {
        iter.iter = rusty::clone(iter.iter_orig);
        rusty::detail::deref_if_pointer_like(item) = iter.iter.next().unwrap();
    }
}
this->_0 = rusty::None;
return rusty::None; }(); } if (_m.is_none()) { return [&]() { auto next = rusty::collect_range(rusty::map(rusty::iter_mut(inner.iters), [&](auto&& i) { return i.iter.next(); }));
if (next.is_none() || rusty::is_empty(inner.iters)) {
    this->_0 = rusty::None;
} else {
    inner.cur.clone_from(next);
}
return next; }(); } rusty::intrinsics::unreachable(); }();
            }
            size_t count() {
                return [&]() -> size_t { auto&& _m = this->_0; if (_m.is_none()) { return static_cast<size_t>(0); } if (_m.is_some()) { auto&& _mv1 = std::as_const(_m).unwrap(); if (_mv1.cur.is_none()) { auto&& iters = rusty::detail::deref_if_pointer(_mv1.iters); return rusty::try_fold(rusty::map(rusty::iter(iters), [&](auto&& iter) { return iter.iter_orig.count(); }), 1, [&](auto&& product, auto&& count) {
if (count == 0) {
    return decltype(rusty::Some(product * count))(rusty::None);
} else {
    return rusty::Some(product * count);
}
}).unwrap_or(rusty::default_value<size_t>()); } } if (_m.is_some()) { auto&& _mv2 = std::as_const(_m).unwrap(); if (rusty::detail::deref_if_pointer(_mv2.cur).is_some()) { auto&& iters = rusty::detail::deref_if_pointer(_mv2.iters); return rusty::fold(rusty::iter(iters), static_cast<size_t>(0), [&](auto&& acc, auto&& iter) {
if (acc != 0) {
    [&]() { static_cast<void>(acc *= iter.iter_orig.count()); return std::make_tuple(); }();
}
return acc + iter.iter.count();
}); } } return [&]() -> size_t { rusty::intrinsics::unreachable(); }(); }();
            }
            std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
                return [&]() -> std::tuple<size_t, rusty::Option<size_t>> { auto&& _m = &this->_0; if (_m.is_none()) { return std::make_tuple(static_cast<size_t>(0), rusty::Option<size_t>(static_cast<size_t>(0))); } if (_m.is_some()) { auto&& _mv1 = std::as_const(_m).unwrap(); if (_mv1.cur.is_none()) { auto&& iters = rusty::detail::deref_if_pointer(_mv1.iters); return rusty::fold(rusty::map(rusty::iter(iters), [&](auto&& iter) { return rusty::size_hint(iter.iter_orig); }), std::make_tuple(static_cast<size_t>(1), rusty::Option<size_t>(static_cast<size_t>(1))), size_hint::mul); } } if (_m.is_some()) { auto&& _mv2 = std::as_const(_m).unwrap(); if (rusty::detail::deref_if_pointer(_mv2.cur).is_some()) { auto&& iters = rusty::detail::deref_if_pointer(_mv2.iters); return [&]() -> std::tuple<size_t, rusty::Option<size_t>> { auto&& _iflet = rusty::slice_full(iters); if (rusty::len(rusty::detail::deref_if_pointer(_iflet)) >= 1) { auto&& first = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_iflet)[0]);
auto&& tail = rusty::detail::deref_if_pointer(rusty::slice(rusty::detail::deref_if_pointer(_iflet), 1, rusty::len(rusty::detail::deref_if_pointer(_iflet))));
return rusty::fold(rusty::iter(tail), rusty::size_hint(first.iter), [&](auto&& sh, auto&& iter) {
sh = size_hint::mul(std::move(sh), rusty::size_hint(iter.iter_orig));
return size_hint::add(std::move(sh), rusty::size_hint(iter.iter));
}); } return [&]() -> std::tuple<size_t, rusty::Option<size_t>> { rusty::panicking::panic("internal error: entered unreachable code"); }(); }(); } } return [&]() -> std::tuple<size_t, rusty::Option<size_t>> { rusty::intrinsics::unreachable(); }(); }();
            }
            auto last() {
                auto&& _let_pat = RUSTY_TRY_OPT(this->_0);
                auto&& iters = rusty::detail::deref_if_pointer(_let_pat.iters);
                auto&& cur = rusty::detail::deref_if_pointer(_let_pat.cur);
                if (rusty::detail::deref_if_pointer(cur).is_some()) {
                    auto&& values = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(cur)).unwrap());
                    auto count = rusty::len(iters);
                    auto last = rusty::collect_range(rusty::map(rusty::zip(rusty::iter(std::move(iters)), std::move(values)), [&](auto&& _destruct_param0) {
auto&& i = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& value = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
return i.iter.last().unwrap_or_else([&]() {
[&]() { static_cast<void>(count -= 1); return std::make_tuple(); }();
return value;
});
}));
                    if (count == 0) {
                        return rusty::None;
                    } else {
                        return rusty::Some(std::move(last));
                    }
                } else {
                    return rusty::Option<rusty::Vec<rusty::detail::associated_item_t<I>>>::from_iter(rusty::map(rusty::iter(std::move(iters)), [&](auto&& i) { return i.iter.last(); }));
                }
            }
        };

    }

    using namespace ::adaptors::coalesce_tests;

    using ::adaptors::map::map_into;
    using ::adaptors::map::map_ok;
    using ::adaptors::map::MapInto;
    using ::adaptors::map::MapOk;

    using namespace ::adaptors::multi_product;

    namespace fmt = rusty::fmt;


    using ::rusty::PhantomData;

    /// An iterator adaptor that alternates elements from two iterators until both
    /// run out.
    ///
    /// This iterator is *fused*.
    ///
    /// See [`.interleave()`](crate::Itertools::interleave) for more information.
    template<typename I, typename J>
    struct Interleave {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        decltype(std::declval<I>().fuse()) i;
        decltype(std::declval<J>().fuse()) j;
        bool next_coming_from_j;

        Interleave<I, J> clone() const {
            return Interleave<I, J>{.i = rusty::clone(this->i), .j = rusty::clone(this->j), .next_coming_from_j = rusty::clone(this->next_coming_from_j)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field3_finish(f, "Interleave", "i", &this->i, "j", &this->j, "next_coming_from_j", &this->next_coming_from_j);
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            this->next_coming_from_j = !this->next_coming_from_j;
            if (this->next_coming_from_j) {
                return [&]() { auto&& _m = this->i.next(); if (_m.is_none()) { return this->j.next(); } if (true) { const auto& r = _m; return r; } rusty::intrinsics::unreachable(); }();
            } else {
                return [&]() { auto&& _m = this->j.next(); if (_m.is_none()) { return this->i.next(); } if (true) { const auto& r = _m; return r; } rusty::intrinsics::unreachable(); }();
            }
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return size_hint::add(rusty::size_hint(this->i), rusty::size_hint(this->j));
        }
        template<typename B, typename F>
        B fold(B init, F f) {
            auto&& _let_pat = (*this);
            auto i = rusty::detail::deref_if_pointer(_let_pat.i);
            auto j = rusty::detail::deref_if_pointer(_let_pat.j);
            auto&& next_coming_from_j = rusty::detail::deref_if_pointer(_let_pat.next_coming_from_j);
            if (next_coming_from_j) {
                {
                    auto&& _m = j.next();
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_some()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& y = rusty::detail::deref_if_pointer(_mv0);
                            init = f(std::move(init), y);
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_none()) {
                            return rusty::fold(i, std::move(init), std::move(f));
                            _m_matched = true;
                        }
                    }
                }
            }
            const auto res = rusty::try_fold(i, std::move(init), [&](auto&& acc, auto&& x) {
acc = f(std::move(acc), std::move(x));
return [&]() { auto&& _m = j.next(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& y = rusty::detail::deref_if_pointer(_mv0); return rusty::Ok(f(std::move(acc), std::move(y))); } if (_m.is_none()) { return rusty::Err(std::move(acc)); } rusty::intrinsics::unreachable(); }();
});
            return [&]() -> B { auto&& _m = res; if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& acc = rusty::detail::deref_if_pointer(_mv0); return rusty::fold(j, std::move(acc), std::move(f)); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& acc = rusty::detail::deref_if_pointer(_mv1); return rusty::fold(i, std::move(acc), std::move(f)); } return [&]() -> B { rusty::intrinsics::unreachable(); }(); }();
        }
    };

    /// An iterator adaptor that alternates elements from the two iterators until
    /// one of them runs out.
    ///
    /// This iterator is *fused*.
    ///
    /// See [`.interleave_shortest()`](crate::Itertools::interleave_shortest)
    /// for more information.
    template<typename I, typename J>
    struct InterleaveShortest {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        I i;
        J j;
        bool next_coming_from_j;

        InterleaveShortest<I, J> clone() const {
            return InterleaveShortest<I, J>{.i = rusty::clone(this->i), .j = rusty::clone(this->j), .next_coming_from_j = rusty::clone(this->next_coming_from_j)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field3_finish(f, "InterleaveShortest", "i", &this->i, "j", &this->j, "next_coming_from_j", &this->next_coming_from_j);
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            auto e = (this->next_coming_from_j ? this->j.next() : this->i.next());
            if (e.is_some()) {
                this->next_coming_from_j = !this->next_coming_from_j;
            }
            return e;
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto [curr_hint, next_hint] = rusty::detail::deref_if_pointer_like([&]() { auto i_hint = rusty::size_hint(this->i);
auto j_hint = rusty::size_hint(this->j);
return (this->next_coming_from_j ? std::make_tuple(std::move(j_hint), std::move(i_hint)) : std::make_tuple(std::move(i_hint), std::move(j_hint))); }());
            auto [curr_lower, curr_upper] = rusty::detail::deref_if_pointer_like(curr_hint);
            auto [next_lower, next_upper] = rusty::detail::deref_if_pointer_like(next_hint);
            auto [combined_lower, combined_upper] = rusty::detail::deref_if_pointer_like(size_hint::mul_scalar(size_hint::min(std::move(curr_hint), std::move(next_hint)), static_cast<size_t>(2)));
            auto lower = (curr_lower > next_lower ? combined_lower + 1 : combined_lower);
            auto upper = [&]() { const auto extra_elem = [&]() -> bool { auto&& _m0 = curr_upper; auto&& _m1 = next_upper; if (_m1.is_none()) { return false; } if (_m0.is_none() && rusty::detail::deref_if_pointer(_m1).is_some()) { return true; } if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some()) { auto&& curr_max = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& next_max = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); return curr_max > next_max; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
return (extra_elem ? combined_upper.and_then([&](auto&& x) { return [&]() { auto&& _checked_lhs = x; return rusty::checked_add(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(1)); }(); }) : combined_upper); }();
            return std::make_tuple(std::move(lower), std::move(upper));
        }
        template<typename B, typename F>
        B fold(B init, F f) {
            auto&& _let_pat = (*this);
            auto i = rusty::detail::deref_if_pointer(_let_pat.i);
            auto j = rusty::detail::deref_if_pointer(_let_pat.j);
            auto&& next_coming_from_j = rusty::detail::deref_if_pointer(_let_pat.next_coming_from_j);
            if (next_coming_from_j) {
                {
                    auto&& _m = j.next();
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_some()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& y = rusty::detail::deref_if_pointer(_mv0);
                            init = f(std::move(init), y);
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_none()) {
                            return init;
                            _m_matched = true;
                        }
                    }
                }
            }
            const auto res = rusty::try_fold(i, std::move(init), [&](auto&& acc, auto&& x) {
acc = f(std::move(acc), std::move(x));
return [&]() { auto&& _m = j.next(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& y = rusty::detail::deref_if_pointer(_mv0); return rusty::Ok(f(std::move(acc), std::move(y))); } if (_m.is_none()) { return rusty::Err(std::move(acc)); } rusty::intrinsics::unreachable(); }();
});
            return [&]() -> B { auto&& _m = res; if (_m.is_ok()) { return _m.unwrap(); } if (_m.is_err()) { return _m.unwrap_err(); } return [&]() -> B { rusty::intrinsics::unreachable(); }(); }();
        }
    };

    /// An iterator adaptor that allows putting back a single
    /// item to the front of the iterator.
    ///
    /// Iterator element type is `I::Item`.
    template<typename I>
    struct PutBack {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        rusty::Option<rusty::detail::associated_item_t<I>> top;
        I iter;

        PutBack<I> clone() const {
            using namespace peeking_take_while;
            return PutBack<I>{.top = rusty::clone(this->top), .iter = rusty::clone(this->iter)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            using namespace peeking_take_while;
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field2_finish(f, "PutBack", "top", &this->top, "iter", &this->iter);
        }
        PutBack<I> with_value(rusty::detail::associated_item_t<I> value) {
            using namespace peeking_take_while;
            this->put_back(std::move(value));
            return std::move((*this));
        }
        auto into_parts() {
            using namespace peeking_take_while;
            auto&& _let_pat = (*this);
            auto&& top = rusty::detail::deref_if_pointer(_let_pat.top);
            auto&& iter = rusty::detail::deref_if_pointer(_let_pat.iter);
            return std::make_tuple(std::move(top), std::move(iter));
        }
        auto put_back(rusty::detail::associated_item_t<I> x) {
            using namespace peeking_take_while;
            return this->top.replace(std::move(x));
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            using namespace peeking_take_while;
            return [&]() { auto&& _m = this->top; if (_m.is_none()) { return this->iter.next(); } if (true) { const auto& some = _m; return some.take(); } rusty::intrinsics::unreachable(); }();
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            using namespace peeking_take_while;
            return size_hint::add_scalar(rusty::size_hint(this->iter), static_cast<size_t>(this->top.is_some()));
        }
        size_t count() {
            using namespace peeking_take_while;
            return this->iter.count() + ((static_cast<size_t>(this->top.is_some())));
        }
        auto last() {
            using namespace peeking_take_while;
            return this->iter.last().or_(std::move(this->top));
        }
        auto nth(size_t n) {
            using namespace peeking_take_while;
            return [&]() { auto&& _m = this->top; if (_m.is_none()) { return this->iter.nth(std::move(n)); } if (true) { const auto& some = _m; return [&]() {
if (n == static_cast<size_t>(0)) {
return some.take();
} else {
rusty::detail::deref_if_pointer_like(some) = rusty::None;
return this->iter.nth(n - 1);
}
}(); } rusty::intrinsics::unreachable(); }();
        }
        template<typename G>
        bool all(G f) {
            using namespace peeking_take_while;
            if (auto&& _iflet_scrutinee = this->top.take(); _iflet_scrutinee.is_some()) {
                decltype(auto) elt = _iflet_scrutinee.unwrap();
                if (!f(std::move(elt))) {
                    return false;
                }
            }
            return this->iter.all(std::move(f));
        }
        template<typename Acc, typename G>
        Acc fold(Acc init, G f) {
            using namespace peeking_take_while;
            auto accum = std::move(init);
            if (auto&& _iflet_scrutinee = this->top.take(); _iflet_scrutinee.is_some()) {
                decltype(auto) elt = _iflet_scrutinee.unwrap();
                accum = f(std::move(accum), std::move(elt));
            }
            return rusty::fold(this->iter, std::move(accum), std::move(f));
        }
        template<typename F>
        auto peeking_next(F accept) {
            using namespace peeking_take_while;
            if (auto&& _iflet_scrutinee = this->next(); _iflet_scrutinee.is_some()) {
                decltype(auto) r = _iflet_scrutinee.unwrap();
                if (!accept(r)) {
                    this->put_back(std::move(r));
                    return rusty::None;
                }
                return rusty::Some(std::move(r));
            } else {
                return rusty::None;
            }
        }
    };

    /// An iterator adaptor that iterates over the cartesian product of
    /// the element sets of two iterators `I` and `J`.
    ///
    /// Iterator element type is `(I::Item, J::Item)`.
    ///
    /// See [`.cartesian_product()`](crate::Itertools::cartesian_product) for more information.
    template<typename I, typename J>
    struct Product {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        I a;
        /// `a_cur` is `None` while no item have been taken out of `a` (at definition).
        /// Then `a_cur` will be `Some(Some(item))` until `a` is exhausted,
        /// in which case `a_cur` will be `Some(None)`.
        rusty::Option<rusty::Option<rusty::detail::associated_item_t<I>>> a_cur;
        J b;
        J b_orig;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field4_finish(f, "Product", "a", &this->a, "a_cur", &this->a_cur, "b", &this->b, "b_orig", &this->b_orig);
        }
        Product<I, J> clone() const {
            return Product<I, J>{.a = rusty::clone(this->a), .a_cur = rusty::clone(this->a_cur), .b = rusty::clone(this->b), .b_orig = rusty::clone(this->b_orig)};
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            auto&& _let_pat = (*this);
            auto&& a = rusty::detail::deref_if_pointer(_let_pat.a);
            auto&& a_cur = rusty::detail::deref_if_pointer(_let_pat.a_cur);
            auto&& b = rusty::detail::deref_if_pointer(_let_pat.b);
            auto&& b_orig = rusty::detail::deref_if_pointer(_let_pat.b_orig);
            auto elt_b = [&]() { auto&& _m = b.next(); if (_m.is_none()) { return [&]() { rusty::detail::deref_if_pointer_like(b) = rusty::clone(b_orig);
return ({ auto&& _m = b.next(); std::optional<std::remove_cvref_t<decltype(([&]() -> decltype(auto) { auto _mv = _m.unwrap();
auto&& x = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
return ([&]() { rusty::detail::deref_if_pointer_like(a_cur) = rusty::Some(a.next());
return x; }()); })())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& x = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move([&]() { rusty::detail::deref_if_pointer_like(a_cur) = rusty::Some(a.next());
return x; }())); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); }); }(); } if (_m.is_some()) { return _m.unwrap(); } rusty::intrinsics::unreachable(); }();
            return a_cur.get_or_insert_with([&]() { return a.next(); }).as_ref().map([&](auto&& a) -> std::tuple<rusty::detail::associated_item_t<I>, rusty::detail::associated_item_t<J>> { return std::make_tuple(rusty::clone(a), std::move(elt_b)); });
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto sh = size_hint::mul(rusty::size_hint(this->a), rusty::size_hint(this->b_orig));
            if ([&]() -> bool { auto&& _m = this->a_cur; if (_m.is_some()) { auto&& _mv0 = std::as_const(_m).unwrap(); if (rusty::detail::deref_if_pointer(_mv0).is_some()) { return true; } } if (true) { return false; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }()) {
                sh = size_hint::add(std::move(sh), rusty::size_hint(this->b));
            }
            return sh;
        }
        template<typename Acc, typename G>
        Acc fold(Acc accum, G f) {
            auto&& _let_pat = (*this);
            auto a = rusty::detail::deref_if_pointer(_let_pat.a);
            auto&& a_cur = rusty::detail::deref_if_pointer(_let_pat.a_cur);
            auto b = rusty::detail::deref_if_pointer(_let_pat.b);
            auto&& b_orig = rusty::detail::deref_if_pointer(_let_pat.b_orig);
            if (auto&& _iflet_scrutinee = a_cur.unwrap_or_else([&]() { return a.next(); }); _iflet_scrutinee.is_some()) {
                decltype(auto) elt_a = _iflet_scrutinee.unwrap();
                while (true) {
                    accum = rusty::fold(b, std::move(accum), [&](auto&& acc, auto&& elt) { return f(std::move(acc), std::make_tuple(rusty::clone(elt_a), std::move(elt))); });
                    if (auto&& _iflet_scrutinee = a.next(); rusty::detail::option_has_value(_iflet_scrutinee)) {
                        auto&& _iflet_take = _iflet_scrutinee;
                        auto next_elt_a = rusty::detail::option_take_value(_iflet_take);
                        b = rusty::clone(b_orig);
                        elt_a = std::move(next_elt_a);
                    } else {
                        break;
                    }
                }
            }
            return accum;
        }
    };

    /// A “meta iterator adaptor”. Its closure receives a reference to the iterator
    /// and may pick off as many elements as it likes, to produce the next iterator element.
    ///
    /// Iterator element type is `X` if the return type of `F` is `Option<X>`.
    ///
    /// See [`.batching()`](crate::Itertools::batching) for more information.
    template<typename I, typename F>
    struct Batching {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        F f;
        I iter;

        Batching<I, F> clone() const {
            return Batching<I, F>{.f = rusty::clone(this->f), .iter = rusty::clone(this->iter)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("Batching").field("iter", &this->iter).finish();
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename B>
        auto next() {
            return (this->f)(&this->iter);
        }
    };

    /// An iterator adaptor that borrows from a `Clone`-able iterator
    /// to only pick off elements while the predicate returns `true`.
    ///
    /// See [`.take_while_ref()`](crate::Itertools::take_while_ref) for more information.
    template<typename I, typename F>
    struct TakeWhileRef {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        I& iter;
        F f;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("TakeWhileRef").field("iter", &this->iter).finish();
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            const auto old = rusty::clone(this->iter);
            return [&]() { auto&& _m = this->iter.next(); if (_m.is_none()) { return rusty::None; } if (_m.is_some()) { auto&& _mv1 = _m.unwrap(); auto&& elt = rusty::detail::deref_if_pointer(_mv1); return [&]() {
if ((this->f)(elt)) {
return rusty::Some(std::move(elt));
} else {
this->iter = std::move(old);
return rusty::None;
}
}(); } rusty::intrinsics::unreachable(); }();
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return std::make_tuple(static_cast<size_t>(0), rusty::size_hint(this->iter)._1);
        }
    };

    /// An iterator adaptor that filters `Option<A>` iterator elements
    /// and produces `A`. Stops on the first `None` encountered.
    ///
    /// See [`.while_some()`](crate::Itertools::while_some) for more information.
    template<typename I>
    struct WhileSome {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        I iter;

        WhileSome<I> clone() const {
            return WhileSome<I>{.iter = rusty::clone(this->iter)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field1_finish(f, "WhileSome", "iter", &this->iter);
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A>
        auto next() {
            return [&]() { auto&& _m = this->iter.next(); if ((_m.is_none() || (rusty::detail::deref_if_pointer(_m).is_some() && std::as_const(rusty::detail::deref_if_pointer(_m)).unwrap().is_none()))) { return rusty::None; } if (_m.is_some()) { return _m.unwrap(); } rusty::intrinsics::unreachable(); }();
        }
        template<typename A>
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return std::make_tuple(static_cast<size_t>(0), rusty::size_hint(this->iter)._1);
        }
        template<typename B, typename F, typename A>
        B fold(B acc, F f) {
            const auto res = rusty::try_fold(this->iter, std::move(acc), [&](auto&& acc, auto&& item) { return [&]() { auto&& _m = item; if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& item = rusty::detail::deref_if_pointer(_mv0); return rusty::Ok(f(std::move(acc), std::move(item))); } if (_m.is_none()) { return rusty::Err(std::move(acc)); } rusty::intrinsics::unreachable(); }(); });
            return [&]() -> B { auto&& _m = res; if (_m.is_ok()) { return _m.unwrap(); } if (_m.is_err()) { return _m.unwrap_err(); } return [&]() -> B { rusty::intrinsics::unreachable(); }(); }();
        }
    };

    /// An iterator to iterate through all combinations in a `Clone`-able iterator that produces tuples
    /// of a specific size.
    ///
    /// See [`.tuple_combinations()`](crate::Itertools::tuple_combinations) for more
    /// information.
    template<typename I, typename T>
    struct TupleCombinations {
        using Item = T;
        typename T::Combination iter;
        rusty::PhantomData<I> _mi;

        TupleCombinations<I, T> clone() const {
            return TupleCombinations<I, T>{.iter = rusty::clone(this->iter), ._mi = rusty::clone(this->_mi)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field2_finish(f, "TupleCombinations", "iter", &this->iter, "_mi", &this->_mi);
        }
        rusty::Option<Item> next() {
            return this->iter.next();
        }
        size_hint::SizeHint size_hint() const {
            return rusty::size_hint(this->iter);
        }
        size_t count() {
            return this->iter.count();
        }
        template<typename B, typename F>
        B fold(B init, F f) {
            return rusty::fold(this->iter, std::move(init), std::move(f));
        }
    };

    // Rust-only trait HasCombination (Proxy facade emission skipped in module mode)

    template<typename I>
    struct Tuple1Combination {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        I iter;

        Tuple1Combination<I> clone() const {
            return Tuple1Combination<I>{.iter = rusty::clone(this->iter)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field1_finish(f, "Tuple1Combination", "iter", &this->iter);
        }
        static Tuple1Combination<I> from(I iter) {
            return Tuple1Combination<I>{.iter = std::move(iter)};
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            return this->iter.next().map([&](auto&& x) -> std::tuple<rusty::detail::associated_item_t<I>> { return std::make_tuple(std::move(x)); });
        }
        size_hint::SizeHint size_hint() const {
            return rusty::size_hint(this->iter);
        }
        size_t count() {
            return this->iter.count();
        }
        template<typename B, typename F>
        B fold(B init, F f) {
            return rusty::fold(this->iter.map([&](auto&& x) { return std::make_tuple(std::move(x)); }), std::move(init), std::move(f));
        }
    };

    template<typename I>
    struct Tuple2Combination {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        rusty::Option<rusty::detail::associated_item_t<I>> item;
        I iter;
        Tuple1Combination<I> c;

        Tuple2Combination<I> clone() const {
            return Tuple2Combination<I>{.item = rusty::clone(this->item), .iter = rusty::clone(this->iter), .c = rusty::clone(this->c)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field3_finish(f, "Tuple2Combination", "item", &this->item, "iter", &this->iter, "c", &this->c);
        }
        static Tuple2Combination<I> from(I iter) {
            return Tuple2Combination<I>{.item = iter.next(), .iter = rusty::clone(iter), .c = rusty::from_into<Tuple1Combination<I>>(std::move(iter))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A>
        auto next() {
            if (auto&& _iflet_scrutinee = this->c.next(); _iflet_scrutinee.is_some()) {
                auto&& _iflet_payload = _iflet_scrutinee.unwrap();
                auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto z = rusty::clone(this->item).unwrap();
                return rusty::Some(std::make_tuple(std::move(z), std::move(a)));
            } else {
                this->item = this->iter.next();
                return rusty::clone(this->item).and_then([&](auto&& z) {
this->c = rusty::from_into<Tuple1Combination<I>>(rusty::clone(this->iter));
return this->c.next().map([&](auto&& _destruct_param0) {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(std::move(z), std::move(a));
});
});
            }
        }
        template<typename A>
        size_hint::SizeHint size_hint() const {
            constexpr size_t K = 1 + ((1 + 0));
            auto [n_min, n_max] = rusty::detail::deref_if_pointer_like(rusty::size_hint(this->iter));
            n_min = checked_binomial(std::move(n_min), K).unwrap_or(std::numeric_limits<size_t>::max());
            n_max = n_max.and_then([&](auto&& n) { return checked_binomial(std::move(n), K); });
            return size_hint::add(rusty::size_hint(this->c), std::make_tuple(std::move(n_min), std::move(n_max)));
        }
        template<typename A>
        size_t count() {
            constexpr size_t K = 1 + ((1 + 0));
            auto n = this->iter.count();
            return checked_binomial(std::move(n), K).unwrap() + this->c.count();
        }
        template<typename B, typename F, typename A>
        B fold(B init, F f) {
            using CurrTuple [[maybe_unused]] = std::tuple<A, A>;
            using PrevTuple [[maybe_unused]] = std::tuple<A>;
            const auto map_fn = [](const auto& z) {
                return [=, z = std::move(z)](auto&& _destruct_param0) mutable -> CurrTuple {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(rusty::clone(z), std::move(a));
};
            };
            auto&& _let_pat = (*this);
            auto&& c = rusty::detail::deref_if_pointer(_let_pat.c);
            auto&& item = rusty::detail::deref_if_pointer(_let_pat.item);
            auto iter = rusty::detail::deref_if_pointer(_let_pat.iter);
            if (auto&& _iflet_scrutinee = item.as_ref(); _iflet_scrutinee.is_some()) {
                decltype(auto) z = _iflet_scrutinee.unwrap();
                init = rusty::fold(c.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            while (true) {
                auto&& _whilelet = iter.next();
                if (!(rusty::detail::option_has_value(_whilelet))) { break; }
                auto z = rusty::detail::option_take_value(_whilelet);
                Tuple1Combination<I> c_shadow1 = rusty::from_into<Tuple1Combination<I>>(rusty::clone(iter));
                init = rusty::fold(c_shadow1.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            return init;
        }
    };

    template<typename I>
    struct Tuple3Combination {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        rusty::Option<rusty::detail::associated_item_t<I>> item;
        I iter;
        Tuple2Combination<I> c;

        Tuple3Combination<I> clone() const {
            return Tuple3Combination<I>{.item = rusty::clone(this->item), .iter = rusty::clone(this->iter), .c = rusty::clone(this->c)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field3_finish(f, "Tuple3Combination", "item", &this->item, "iter", &this->iter, "c", &this->c);
        }
        static Tuple3Combination<I> from(I iter) {
            return Tuple3Combination<I>{.item = iter.next(), .iter = rusty::clone(iter), .c = rusty::from_into<Tuple2Combination<I>>(std::move(iter))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A>
        auto next() {
            if (auto&& _iflet_scrutinee = this->c.next(); _iflet_scrutinee.is_some()) {
                auto&& _iflet_payload = _iflet_scrutinee.unwrap();
                auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto z = rusty::clone(this->item).unwrap();
                return rusty::Some(std::make_tuple(std::move(z), std::move(a), std::move(b)));
            } else {
                this->item = this->iter.next();
                return rusty::clone(this->item).and_then([&](auto&& z) {
this->c = rusty::from_into<Tuple2Combination<I>>(rusty::clone(this->iter));
return this->c.next().map([&](auto&& _destruct_param0) {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(std::move(z), std::move(a), std::move(b));
});
});
            }
        }
        template<typename A>
        size_hint::SizeHint size_hint() const {
            constexpr size_t K = 1 + ((1 + ((1 + 0))));
            auto [n_min, n_max] = rusty::detail::deref_if_pointer_like(rusty::size_hint(this->iter));
            n_min = checked_binomial(std::move(n_min), K).unwrap_or(std::numeric_limits<size_t>::max());
            n_max = n_max.and_then([&](auto&& n) { return checked_binomial(std::move(n), K); });
            return size_hint::add(rusty::size_hint(this->c), std::make_tuple(std::move(n_min), std::move(n_max)));
        }
        template<typename A>
        size_t count() {
            constexpr size_t K = 1 + ((1 + ((1 + 0))));
            auto n = this->iter.count();
            return checked_binomial(std::move(n), K).unwrap() + this->c.count();
        }
        template<typename B, typename F, typename A>
        B fold(B init, F f) {
            using CurrTuple [[maybe_unused]] = std::tuple<A, A, A>;
            using PrevTuple [[maybe_unused]] = std::tuple<A, A>;
            const auto map_fn = [](const auto& z) {
                return [=, z = std::move(z)](auto&& _destruct_param0) mutable -> CurrTuple {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(rusty::clone(z), std::move(a), std::move(b));
};
            };
            auto&& _let_pat = (*this);
            auto&& c = rusty::detail::deref_if_pointer(_let_pat.c);
            auto&& item = rusty::detail::deref_if_pointer(_let_pat.item);
            auto iter = rusty::detail::deref_if_pointer(_let_pat.iter);
            if (auto&& _iflet_scrutinee = item.as_ref(); _iflet_scrutinee.is_some()) {
                decltype(auto) z = _iflet_scrutinee.unwrap();
                init = rusty::fold(c.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            while (true) {
                auto&& _whilelet = iter.next();
                if (!(rusty::detail::option_has_value(_whilelet))) { break; }
                auto z = rusty::detail::option_take_value(_whilelet);
                Tuple2Combination<I> c_shadow1 = rusty::from_into<Tuple2Combination<I>>(rusty::clone(iter));
                init = rusty::fold(c_shadow1.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            return init;
        }
    };

    template<typename I>
    struct Tuple4Combination {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        rusty::Option<rusty::detail::associated_item_t<I>> item;
        I iter;
        Tuple3Combination<I> c;

        Tuple4Combination<I> clone() const {
            return Tuple4Combination<I>{.item = rusty::clone(this->item), .iter = rusty::clone(this->iter), .c = rusty::clone(this->c)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field3_finish(f, "Tuple4Combination", "item", &this->item, "iter", &this->iter, "c", &this->c);
        }
        static Tuple4Combination<I> from(I iter) {
            return Tuple4Combination<I>{.item = iter.next(), .iter = rusty::clone(iter), .c = rusty::from_into<Tuple3Combination<I>>(std::move(iter))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A>
        auto next() {
            if (auto&& _iflet_scrutinee = this->c.next(); _iflet_scrutinee.is_some()) {
                auto&& _iflet_payload = _iflet_scrutinee.unwrap();
                auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto z = rusty::clone(this->item).unwrap();
                return rusty::Some(std::make_tuple(std::move(z), std::move(a), std::move(b), std::move(c)));
            } else {
                this->item = this->iter.next();
                return rusty::clone(this->item).and_then([&](auto&& z) {
this->c = rusty::from_into<Tuple3Combination<I>>(rusty::clone(this->iter));
return this->c.next().map([&](auto&& _destruct_param0) {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(std::move(z), std::move(a), std::move(b), std::move(c));
});
});
            }
        }
        template<typename A>
        size_hint::SizeHint size_hint() const {
            constexpr size_t K = 1 + ((1 + ((1 + ((1 + 0))))));
            auto [n_min, n_max] = rusty::detail::deref_if_pointer_like(rusty::size_hint(this->iter));
            n_min = checked_binomial(std::move(n_min), K).unwrap_or(std::numeric_limits<size_t>::max());
            n_max = n_max.and_then([&](auto&& n) { return checked_binomial(std::move(n), K); });
            return size_hint::add(rusty::size_hint(this->c), std::make_tuple(std::move(n_min), std::move(n_max)));
        }
        template<typename A>
        size_t count() {
            constexpr size_t K = 1 + ((1 + ((1 + ((1 + 0))))));
            auto n = this->iter.count();
            return checked_binomial(std::move(n), K).unwrap() + this->c.count();
        }
        template<typename B, typename F, typename A>
        B fold(B init, F f) {
            using CurrTuple [[maybe_unused]] = std::tuple<A, A, A, A>;
            using PrevTuple [[maybe_unused]] = std::tuple<A, A, A>;
            const auto map_fn = [](const auto& z) {
                return [=, z = std::move(z)](auto&& _destruct_param0) mutable -> CurrTuple {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(rusty::clone(z), std::move(a), std::move(b), std::move(c));
};
            };
            auto&& _let_pat = (*this);
            auto&& c = rusty::detail::deref_if_pointer(_let_pat.c);
            auto&& item = rusty::detail::deref_if_pointer(_let_pat.item);
            auto iter = rusty::detail::deref_if_pointer(_let_pat.iter);
            if (auto&& _iflet_scrutinee = item.as_ref(); _iflet_scrutinee.is_some()) {
                decltype(auto) z = _iflet_scrutinee.unwrap();
                init = rusty::fold(c.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            while (true) {
                auto&& _whilelet = iter.next();
                if (!(rusty::detail::option_has_value(_whilelet))) { break; }
                auto z = rusty::detail::option_take_value(_whilelet);
                Tuple3Combination<I> c_shadow1 = rusty::from_into<Tuple3Combination<I>>(rusty::clone(iter));
                init = rusty::fold(c_shadow1.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            return init;
        }
    };

    template<typename I>
    struct Tuple5Combination {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        rusty::Option<rusty::detail::associated_item_t<I>> item;
        I iter;
        Tuple4Combination<I> c;

        Tuple5Combination<I> clone() const {
            return Tuple5Combination<I>{.item = rusty::clone(this->item), .iter = rusty::clone(this->iter), .c = rusty::clone(this->c)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field3_finish(f, "Tuple5Combination", "item", &this->item, "iter", &this->iter, "c", &this->c);
        }
        static Tuple5Combination<I> from(I iter) {
            return Tuple5Combination<I>{.item = iter.next(), .iter = rusty::clone(iter), .c = rusty::from_into<Tuple4Combination<I>>(std::move(iter))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A>
        auto next() {
            if (auto&& _iflet_scrutinee = this->c.next(); _iflet_scrutinee.is_some()) {
                auto&& _iflet_payload = _iflet_scrutinee.unwrap();
                auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto z = rusty::clone(this->item).unwrap();
                return rusty::Some(std::make_tuple(std::move(z), std::move(a), std::move(b), std::move(c), std::move(d)));
            } else {
                this->item = this->iter.next();
                return rusty::clone(this->item).and_then([&](auto&& z) {
this->c = rusty::from_into<Tuple4Combination<I>>(rusty::clone(this->iter));
return this->c.next().map([&](auto&& _destruct_param0) {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(std::move(z), std::move(a), std::move(b), std::move(c), std::move(d));
});
});
            }
        }
        template<typename A>
        size_hint::SizeHint size_hint() const {
            constexpr size_t K = 1 + ((1 + ((1 + ((1 + ((1 + 0))))))));
            auto [n_min, n_max] = rusty::detail::deref_if_pointer_like(rusty::size_hint(this->iter));
            n_min = checked_binomial(std::move(n_min), K).unwrap_or(std::numeric_limits<size_t>::max());
            n_max = n_max.and_then([&](auto&& n) { return checked_binomial(std::move(n), K); });
            return size_hint::add(rusty::size_hint(this->c), std::make_tuple(std::move(n_min), std::move(n_max)));
        }
        template<typename A>
        size_t count() {
            constexpr size_t K = 1 + ((1 + ((1 + ((1 + ((1 + 0))))))));
            auto n = this->iter.count();
            return checked_binomial(std::move(n), K).unwrap() + this->c.count();
        }
        template<typename B, typename F, typename A>
        B fold(B init, F f) {
            using CurrTuple [[maybe_unused]] = std::tuple<A, A, A, A, A>;
            using PrevTuple [[maybe_unused]] = std::tuple<A, A, A, A>;
            const auto map_fn = [](const auto& z) {
                return [=, z = std::move(z)](auto&& _destruct_param0) mutable -> CurrTuple {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(rusty::clone(z), std::move(a), std::move(b), std::move(c), std::move(d));
};
            };
            auto&& _let_pat = (*this);
            auto&& c = rusty::detail::deref_if_pointer(_let_pat.c);
            auto&& item = rusty::detail::deref_if_pointer(_let_pat.item);
            auto iter = rusty::detail::deref_if_pointer(_let_pat.iter);
            if (auto&& _iflet_scrutinee = item.as_ref(); _iflet_scrutinee.is_some()) {
                decltype(auto) z = _iflet_scrutinee.unwrap();
                init = rusty::fold(c.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            while (true) {
                auto&& _whilelet = iter.next();
                if (!(rusty::detail::option_has_value(_whilelet))) { break; }
                auto z = rusty::detail::option_take_value(_whilelet);
                Tuple4Combination<I> c_shadow1 = rusty::from_into<Tuple4Combination<I>>(rusty::clone(iter));
                init = rusty::fold(c_shadow1.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            return init;
        }
    };

    template<typename I>
    struct Tuple6Combination {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        rusty::Option<rusty::detail::associated_item_t<I>> item;
        I iter;
        Tuple5Combination<I> c;

        Tuple6Combination<I> clone() const {
            return Tuple6Combination<I>{.item = rusty::clone(this->item), .iter = rusty::clone(this->iter), .c = rusty::clone(this->c)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field3_finish(f, "Tuple6Combination", "item", &this->item, "iter", &this->iter, "c", &this->c);
        }
        static Tuple6Combination<I> from(I iter) {
            return Tuple6Combination<I>{.item = iter.next(), .iter = rusty::clone(iter), .c = rusty::from_into<Tuple5Combination<I>>(std::move(iter))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A>
        auto next() {
            if (auto&& _iflet_scrutinee = this->c.next(); _iflet_scrutinee.is_some()) {
                auto&& _iflet_payload = _iflet_scrutinee.unwrap();
                auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto z = rusty::clone(this->item).unwrap();
                return rusty::Some(std::make_tuple(std::move(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e)));
            } else {
                this->item = this->iter.next();
                return rusty::clone(this->item).and_then([&](auto&& z) {
this->c = rusty::from_into<Tuple5Combination<I>>(rusty::clone(this->iter));
return this->c.next().map([&](auto&& _destruct_param0) {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(std::move(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e));
});
});
            }
        }
        template<typename A>
        size_hint::SizeHint size_hint() const {
            constexpr size_t K = 1 + ((1 + ((1 + ((1 + ((1 + ((1 + 0))))))))));
            auto [n_min, n_max] = rusty::detail::deref_if_pointer_like(rusty::size_hint(this->iter));
            n_min = checked_binomial(std::move(n_min), K).unwrap_or(std::numeric_limits<size_t>::max());
            n_max = n_max.and_then([&](auto&& n) { return checked_binomial(std::move(n), K); });
            return size_hint::add(rusty::size_hint(this->c), std::make_tuple(std::move(n_min), std::move(n_max)));
        }
        template<typename A>
        size_t count() {
            constexpr size_t K = 1 + ((1 + ((1 + ((1 + ((1 + ((1 + 0))))))))));
            auto n = this->iter.count();
            return checked_binomial(std::move(n), K).unwrap() + this->c.count();
        }
        template<typename B, typename F, typename A>
        B fold(B init, F f) {
            using CurrTuple [[maybe_unused]] = std::tuple<A, A, A, A, A, A>;
            using PrevTuple [[maybe_unused]] = std::tuple<A, A, A, A, A>;
            const auto map_fn = [](const auto& z) {
                return [=, z = std::move(z)](auto&& _destruct_param0) mutable -> CurrTuple {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(rusty::clone(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e));
};
            };
            auto&& _let_pat = (*this);
            auto&& c = rusty::detail::deref_if_pointer(_let_pat.c);
            auto&& item = rusty::detail::deref_if_pointer(_let_pat.item);
            auto iter = rusty::detail::deref_if_pointer(_let_pat.iter);
            if (auto&& _iflet_scrutinee = item.as_ref(); _iflet_scrutinee.is_some()) {
                decltype(auto) z = _iflet_scrutinee.unwrap();
                init = rusty::fold(c.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            while (true) {
                auto&& _whilelet = iter.next();
                if (!(rusty::detail::option_has_value(_whilelet))) { break; }
                auto z = rusty::detail::option_take_value(_whilelet);
                Tuple5Combination<I> c_shadow1 = rusty::from_into<Tuple5Combination<I>>(rusty::clone(iter));
                init = rusty::fold(c_shadow1.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            return init;
        }
    };

    template<typename I>
    struct Tuple7Combination {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        rusty::Option<rusty::detail::associated_item_t<I>> item;
        I iter;
        Tuple6Combination<I> c;

        Tuple7Combination<I> clone() const {
            return Tuple7Combination<I>{.item = rusty::clone(this->item), .iter = rusty::clone(this->iter), .c = rusty::clone(this->c)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field3_finish(f, "Tuple7Combination", "item", &this->item, "iter", &this->iter, "c", &this->c);
        }
        static Tuple7Combination<I> from(I iter) {
            return Tuple7Combination<I>{.item = iter.next(), .iter = rusty::clone(iter), .c = rusty::from_into<Tuple6Combination<I>>(std::move(iter))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A>
        auto next() {
            if (auto&& _iflet_scrutinee = this->c.next(); _iflet_scrutinee.is_some()) {
                auto&& _iflet_payload = _iflet_scrutinee.unwrap();
                auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& f = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto z = rusty::clone(this->item).unwrap();
                return rusty::Some(std::make_tuple(std::move(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e), std::move(f)));
            } else {
                this->item = this->iter.next();
                return rusty::clone(this->item).and_then([&](auto&& z) {
this->c = rusty::from_into<Tuple6Combination<I>>(rusty::clone(this->iter));
return this->c.next().map([&](auto&& _destruct_param0) {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& f = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(std::move(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e), std::move(f));
});
});
            }
        }
        template<typename A>
        size_hint::SizeHint size_hint() const {
            constexpr size_t K = 1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + 0))))))))))));
            auto [n_min, n_max] = rusty::detail::deref_if_pointer_like(rusty::size_hint(this->iter));
            n_min = checked_binomial(std::move(n_min), K).unwrap_or(std::numeric_limits<size_t>::max());
            n_max = n_max.and_then([&](auto&& n) { return checked_binomial(std::move(n), K); });
            return size_hint::add(rusty::size_hint(this->c), std::make_tuple(std::move(n_min), std::move(n_max)));
        }
        template<typename A>
        size_t count() {
            constexpr size_t K = 1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + 0))))))))))));
            auto n = this->iter.count();
            return checked_binomial(std::move(n), K).unwrap() + this->c.count();
        }
        template<typename B, typename F, typename A>
        B fold(B init, F f) {
            using CurrTuple [[maybe_unused]] = std::tuple<A, A, A, A, A, A, A>;
            using PrevTuple [[maybe_unused]] = std::tuple<A, A, A, A, A, A>;
            const auto map_fn = [](const auto& z) {
                return [=, z = std::move(z)](auto&& _destruct_param0) mutable -> CurrTuple {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& f = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(rusty::clone(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e), std::move(f));
};
            };
            auto&& _let_pat = (*this);
            auto&& c = rusty::detail::deref_if_pointer(_let_pat.c);
            auto&& item = rusty::detail::deref_if_pointer(_let_pat.item);
            auto iter = rusty::detail::deref_if_pointer(_let_pat.iter);
            if (auto&& _iflet_scrutinee = item.as_ref(); _iflet_scrutinee.is_some()) {
                decltype(auto) z = _iflet_scrutinee.unwrap();
                init = rusty::fold(c.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            while (true) {
                auto&& _whilelet = iter.next();
                if (!(rusty::detail::option_has_value(_whilelet))) { break; }
                auto z = rusty::detail::option_take_value(_whilelet);
                Tuple6Combination<I> c_shadow1 = rusty::from_into<Tuple6Combination<I>>(rusty::clone(iter));
                init = rusty::fold(c_shadow1.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            return init;
        }
    };

    template<typename I>
    struct Tuple8Combination {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        rusty::Option<rusty::detail::associated_item_t<I>> item;
        I iter;
        Tuple7Combination<I> c;

        Tuple8Combination<I> clone() const {
            return Tuple8Combination<I>{.item = rusty::clone(this->item), .iter = rusty::clone(this->iter), .c = rusty::clone(this->c)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field3_finish(f, "Tuple8Combination", "item", &this->item, "iter", &this->iter, "c", &this->c);
        }
        static Tuple8Combination<I> from(I iter) {
            return Tuple8Combination<I>{.item = iter.next(), .iter = rusty::clone(iter), .c = rusty::from_into<Tuple7Combination<I>>(std::move(iter))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A>
        auto next() {
            if (auto&& _iflet_scrutinee = this->c.next(); _iflet_scrutinee.is_some()) {
                auto&& _iflet_payload = _iflet_scrutinee.unwrap();
                auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& f = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& g = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto z = rusty::clone(this->item).unwrap();
                return rusty::Some(std::make_tuple(std::move(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e), std::move(f), std::move(g)));
            } else {
                this->item = this->iter.next();
                return rusty::clone(this->item).and_then([&](auto&& z) {
this->c = rusty::from_into<Tuple7Combination<I>>(rusty::clone(this->iter));
return this->c.next().map([&](auto&& _destruct_param0) {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& f = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& g = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(std::move(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e), std::move(f), std::move(g));
});
});
            }
        }
        template<typename A>
        size_hint::SizeHint size_hint() const {
            constexpr size_t K = 1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + 0))))))))))))));
            auto [n_min, n_max] = rusty::detail::deref_if_pointer_like(rusty::size_hint(this->iter));
            n_min = checked_binomial(std::move(n_min), K).unwrap_or(std::numeric_limits<size_t>::max());
            n_max = n_max.and_then([&](auto&& n) { return checked_binomial(std::move(n), K); });
            return size_hint::add(rusty::size_hint(this->c), std::make_tuple(std::move(n_min), std::move(n_max)));
        }
        template<typename A>
        size_t count() {
            constexpr size_t K = 1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + 0))))))))))))));
            auto n = this->iter.count();
            return checked_binomial(std::move(n), K).unwrap() + this->c.count();
        }
        template<typename B, typename F, typename A>
        B fold(B init, F f) {
            using CurrTuple [[maybe_unused]] = std::tuple<A, A, A, A, A, A, A, A>;
            using PrevTuple [[maybe_unused]] = std::tuple<A, A, A, A, A, A, A>;
            const auto map_fn = [](const auto& z) {
                return [=, z = std::move(z)](auto&& _destruct_param0) mutable -> CurrTuple {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& f = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& g = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(rusty::clone(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e), std::move(f), std::move(g));
};
            };
            auto&& _let_pat = (*this);
            auto&& c = rusty::detail::deref_if_pointer(_let_pat.c);
            auto&& item = rusty::detail::deref_if_pointer(_let_pat.item);
            auto iter = rusty::detail::deref_if_pointer(_let_pat.iter);
            if (auto&& _iflet_scrutinee = item.as_ref(); _iflet_scrutinee.is_some()) {
                decltype(auto) z = _iflet_scrutinee.unwrap();
                init = rusty::fold(c.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            while (true) {
                auto&& _whilelet = iter.next();
                if (!(rusty::detail::option_has_value(_whilelet))) { break; }
                auto z = rusty::detail::option_take_value(_whilelet);
                Tuple7Combination<I> c_shadow1 = rusty::from_into<Tuple7Combination<I>>(rusty::clone(iter));
                init = rusty::fold(c_shadow1.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            return init;
        }
    };

    template<typename I>
    struct Tuple9Combination {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        rusty::Option<rusty::detail::associated_item_t<I>> item;
        I iter;
        Tuple8Combination<I> c;

        Tuple9Combination<I> clone() const {
            return Tuple9Combination<I>{.item = rusty::clone(this->item), .iter = rusty::clone(this->iter), .c = rusty::clone(this->c)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field3_finish(f, "Tuple9Combination", "item", &this->item, "iter", &this->iter, "c", &this->c);
        }
        static Tuple9Combination<I> from(I iter) {
            return Tuple9Combination<I>{.item = iter.next(), .iter = rusty::clone(iter), .c = rusty::from_into<Tuple8Combination<I>>(std::move(iter))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A>
        auto next() {
            if (auto&& _iflet_scrutinee = this->c.next(); _iflet_scrutinee.is_some()) {
                auto&& _iflet_payload = _iflet_scrutinee.unwrap();
                auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& f = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& g = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& h = rusty::detail::deref_if_pointer(std::get<7>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto z = rusty::clone(this->item).unwrap();
                return rusty::Some(std::make_tuple(std::move(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e), std::move(f), std::move(g), std::move(h)));
            } else {
                this->item = this->iter.next();
                return rusty::clone(this->item).and_then([&](auto&& z) {
this->c = rusty::from_into<Tuple8Combination<I>>(rusty::clone(this->iter));
return this->c.next().map([&](auto&& _destruct_param0) {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& f = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& g = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& h = rusty::detail::deref_if_pointer(std::get<7>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(std::move(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e), std::move(f), std::move(g), std::move(h));
});
});
            }
        }
        template<typename A>
        size_hint::SizeHint size_hint() const {
            constexpr size_t K = 1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + 0))))))))))))))));
            auto [n_min, n_max] = rusty::detail::deref_if_pointer_like(rusty::size_hint(this->iter));
            n_min = checked_binomial(std::move(n_min), K).unwrap_or(std::numeric_limits<size_t>::max());
            n_max = n_max.and_then([&](auto&& n) { return checked_binomial(std::move(n), K); });
            return size_hint::add(rusty::size_hint(this->c), std::make_tuple(std::move(n_min), std::move(n_max)));
        }
        template<typename A>
        size_t count() {
            constexpr size_t K = 1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + 0))))))))))))))));
            auto n = this->iter.count();
            return checked_binomial(std::move(n), K).unwrap() + this->c.count();
        }
        template<typename B, typename F, typename A>
        B fold(B init, F f) {
            using CurrTuple [[maybe_unused]] = std::tuple<A, A, A, A, A, A, A, A, A>;
            using PrevTuple [[maybe_unused]] = std::tuple<A, A, A, A, A, A, A, A>;
            const auto map_fn = [](const auto& z) {
                return [=, z = std::move(z)](auto&& _destruct_param0) mutable -> CurrTuple {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& f = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& g = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& h = rusty::detail::deref_if_pointer(std::get<7>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(rusty::clone(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e), std::move(f), std::move(g), std::move(h));
};
            };
            auto&& _let_pat = (*this);
            auto&& c = rusty::detail::deref_if_pointer(_let_pat.c);
            auto&& item = rusty::detail::deref_if_pointer(_let_pat.item);
            auto iter = rusty::detail::deref_if_pointer(_let_pat.iter);
            if (auto&& _iflet_scrutinee = item.as_ref(); _iflet_scrutinee.is_some()) {
                decltype(auto) z = _iflet_scrutinee.unwrap();
                init = rusty::fold(c.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            while (true) {
                auto&& _whilelet = iter.next();
                if (!(rusty::detail::option_has_value(_whilelet))) { break; }
                auto z = rusty::detail::option_take_value(_whilelet);
                Tuple8Combination<I> c_shadow1 = rusty::from_into<Tuple8Combination<I>>(rusty::clone(iter));
                init = rusty::fold(c_shadow1.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            return init;
        }
    };

    template<typename I>
    struct Tuple10Combination {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        rusty::Option<rusty::detail::associated_item_t<I>> item;
        I iter;
        Tuple9Combination<I> c;

        Tuple10Combination<I> clone() const {
            return Tuple10Combination<I>{.item = rusty::clone(this->item), .iter = rusty::clone(this->iter), .c = rusty::clone(this->c)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field3_finish(f, "Tuple10Combination", "item", &this->item, "iter", &this->iter, "c", &this->c);
        }
        static Tuple10Combination<I> from(I iter) {
            return Tuple10Combination<I>{.item = iter.next(), .iter = rusty::clone(iter), .c = rusty::from_into<Tuple9Combination<I>>(std::move(iter))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A>
        auto next() {
            if (auto&& _iflet_scrutinee = this->c.next(); _iflet_scrutinee.is_some()) {
                auto&& _iflet_payload = _iflet_scrutinee.unwrap();
                auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& f = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& g = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& h = rusty::detail::deref_if_pointer(std::get<7>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& i = rusty::detail::deref_if_pointer(std::get<8>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto z = rusty::clone(this->item).unwrap();
                return rusty::Some(std::make_tuple(std::move(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e), std::move(f), std::move(g), std::move(h), std::move(i)));
            } else {
                this->item = this->iter.next();
                return rusty::clone(this->item).and_then([&](auto&& z) {
this->c = rusty::from_into<Tuple9Combination<I>>(rusty::clone(this->iter));
return this->c.next().map([&](auto&& _destruct_param0) {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& f = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& g = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& h = rusty::detail::deref_if_pointer(std::get<7>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& i = rusty::detail::deref_if_pointer(std::get<8>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(std::move(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e), std::move(f), std::move(g), std::move(h), std::move(i));
});
});
            }
        }
        template<typename A>
        size_hint::SizeHint size_hint() const {
            constexpr size_t K = 1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + 0))))))))))))))))));
            auto [n_min, n_max] = rusty::detail::deref_if_pointer_like(rusty::size_hint(this->iter));
            n_min = checked_binomial(std::move(n_min), K).unwrap_or(std::numeric_limits<size_t>::max());
            n_max = n_max.and_then([&](auto&& n) { return checked_binomial(std::move(n), K); });
            return size_hint::add(rusty::size_hint(this->c), std::make_tuple(std::move(n_min), std::move(n_max)));
        }
        template<typename A>
        size_t count() {
            constexpr size_t K = 1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + 0))))))))))))))))));
            auto n = this->iter.count();
            return checked_binomial(std::move(n), K).unwrap() + this->c.count();
        }
        template<typename B, typename F, typename A>
        B fold(B init, F f) {
            using CurrTuple [[maybe_unused]] = std::tuple<A, A, A, A, A, A, A, A, A, A>;
            using PrevTuple [[maybe_unused]] = std::tuple<A, A, A, A, A, A, A, A, A>;
            const auto map_fn = [](const auto& z) {
                return [=, z = std::move(z)](auto&& _destruct_param0) mutable -> CurrTuple {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& f = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& g = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& h = rusty::detail::deref_if_pointer(std::get<7>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& i = rusty::detail::deref_if_pointer(std::get<8>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(rusty::clone(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e), std::move(f), std::move(g), std::move(h), std::move(i));
};
            };
            auto&& _let_pat = (*this);
            auto&& c = rusty::detail::deref_if_pointer(_let_pat.c);
            auto&& item = rusty::detail::deref_if_pointer(_let_pat.item);
            auto iter = rusty::detail::deref_if_pointer(_let_pat.iter);
            if (auto&& _iflet_scrutinee = item.as_ref(); _iflet_scrutinee.is_some()) {
                decltype(auto) z = _iflet_scrutinee.unwrap();
                init = rusty::fold(c.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            while (true) {
                auto&& _whilelet = iter.next();
                if (!(rusty::detail::option_has_value(_whilelet))) { break; }
                auto z = rusty::detail::option_take_value(_whilelet);
                Tuple9Combination<I> c_shadow1 = rusty::from_into<Tuple9Combination<I>>(rusty::clone(iter));
                init = rusty::fold(c_shadow1.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            return init;
        }
    };

    template<typename I>
    struct Tuple11Combination {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        rusty::Option<rusty::detail::associated_item_t<I>> item;
        I iter;
        Tuple10Combination<I> c;

        Tuple11Combination<I> clone() const {
            return Tuple11Combination<I>{.item = rusty::clone(this->item), .iter = rusty::clone(this->iter), .c = rusty::clone(this->c)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field3_finish(f, "Tuple11Combination", "item", &this->item, "iter", &this->iter, "c", &this->c);
        }
        static Tuple11Combination<I> from(I iter) {
            return Tuple11Combination<I>{.item = iter.next(), .iter = rusty::clone(iter), .c = rusty::from_into<Tuple10Combination<I>>(std::move(iter))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A>
        auto next() {
            if (auto&& _iflet_scrutinee = this->c.next(); _iflet_scrutinee.is_some()) {
                auto&& _iflet_payload = _iflet_scrutinee.unwrap();
                auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& f = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& g = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& h = rusty::detail::deref_if_pointer(std::get<7>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& i = rusty::detail::deref_if_pointer(std::get<8>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& j = rusty::detail::deref_if_pointer(std::get<9>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto z = rusty::clone(this->item).unwrap();
                return rusty::Some(std::make_tuple(std::move(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e), std::move(f), std::move(g), std::move(h), std::move(i), std::move(j)));
            } else {
                this->item = this->iter.next();
                return rusty::clone(this->item).and_then([&](auto&& z) {
this->c = rusty::from_into<Tuple10Combination<I>>(rusty::clone(this->iter));
return this->c.next().map([&](auto&& _destruct_param0) {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& f = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& g = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& h = rusty::detail::deref_if_pointer(std::get<7>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& i = rusty::detail::deref_if_pointer(std::get<8>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& j = rusty::detail::deref_if_pointer(std::get<9>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(std::move(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e), std::move(f), std::move(g), std::move(h), std::move(i), std::move(j));
});
});
            }
        }
        template<typename A>
        size_hint::SizeHint size_hint() const {
            constexpr size_t K = 1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + 0))))))))))))))))))));
            auto [n_min, n_max] = rusty::detail::deref_if_pointer_like(rusty::size_hint(this->iter));
            n_min = checked_binomial(std::move(n_min), K).unwrap_or(std::numeric_limits<size_t>::max());
            n_max = n_max.and_then([&](auto&& n) { return checked_binomial(std::move(n), K); });
            return size_hint::add(rusty::size_hint(this->c), std::make_tuple(std::move(n_min), std::move(n_max)));
        }
        template<typename A>
        size_t count() {
            constexpr size_t K = 1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + 0))))))))))))))))))));
            auto n = this->iter.count();
            return checked_binomial(std::move(n), K).unwrap() + this->c.count();
        }
        template<typename B, typename F, typename A>
        B fold(B init, F f) {
            using CurrTuple [[maybe_unused]] = std::tuple<A, A, A, A, A, A, A, A, A, A, A>;
            using PrevTuple [[maybe_unused]] = std::tuple<A, A, A, A, A, A, A, A, A, A>;
            const auto map_fn = [](const auto& z) {
                return [=, z = std::move(z)](auto&& _destruct_param0) mutable -> CurrTuple {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& f = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& g = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& h = rusty::detail::deref_if_pointer(std::get<7>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& i = rusty::detail::deref_if_pointer(std::get<8>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& j = rusty::detail::deref_if_pointer(std::get<9>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(rusty::clone(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e), std::move(f), std::move(g), std::move(h), std::move(i), std::move(j));
};
            };
            auto&& _let_pat = (*this);
            auto&& c = rusty::detail::deref_if_pointer(_let_pat.c);
            auto&& item = rusty::detail::deref_if_pointer(_let_pat.item);
            auto iter = rusty::detail::deref_if_pointer(_let_pat.iter);
            if (auto&& _iflet_scrutinee = item.as_ref(); _iflet_scrutinee.is_some()) {
                decltype(auto) z = _iflet_scrutinee.unwrap();
                init = rusty::fold(c.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            while (true) {
                auto&& _whilelet = iter.next();
                if (!(rusty::detail::option_has_value(_whilelet))) { break; }
                auto z = rusty::detail::option_take_value(_whilelet);
                Tuple10Combination<I> c_shadow1 = rusty::from_into<Tuple10Combination<I>>(rusty::clone(iter));
                init = rusty::fold(c_shadow1.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            return init;
        }
    };

    template<typename I>
    struct Tuple12Combination {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        rusty::Option<rusty::detail::associated_item_t<I>> item;
        I iter;
        Tuple11Combination<I> c;

        Tuple12Combination<I> clone() const {
            return Tuple12Combination<I>{.item = rusty::clone(this->item), .iter = rusty::clone(this->iter), .c = rusty::clone(this->c)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field3_finish(f, "Tuple12Combination", "item", &this->item, "iter", &this->iter, "c", &this->c);
        }
        static Tuple12Combination<I> from(I iter) {
            return Tuple12Combination<I>{.item = iter.next(), .iter = rusty::clone(iter), .c = rusty::from_into<Tuple11Combination<I>>(std::move(iter))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A>
        auto next() {
            if (auto&& _iflet_scrutinee = this->c.next(); _iflet_scrutinee.is_some()) {
                auto&& _iflet_payload = _iflet_scrutinee.unwrap();
                auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& f = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& g = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& h = rusty::detail::deref_if_pointer(std::get<7>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& i = rusty::detail::deref_if_pointer(std::get<8>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& j = rusty::detail::deref_if_pointer(std::get<9>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto&& k = rusty::detail::deref_if_pointer(std::get<10>(rusty::detail::deref_if_pointer(_iflet_payload)));
                auto z = rusty::clone(this->item).unwrap();
                return rusty::Some(std::make_tuple(std::move(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e), std::move(f), std::move(g), std::move(h), std::move(i), std::move(j), std::move(k)));
            } else {
                this->item = this->iter.next();
                return rusty::clone(this->item).and_then([&](auto&& z) {
this->c = rusty::from_into<Tuple11Combination<I>>(rusty::clone(this->iter));
return this->c.next().map([&](auto&& _destruct_param0) {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& f = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& g = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& h = rusty::detail::deref_if_pointer(std::get<7>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& i = rusty::detail::deref_if_pointer(std::get<8>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& j = rusty::detail::deref_if_pointer(std::get<9>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& k = rusty::detail::deref_if_pointer(std::get<10>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(std::move(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e), std::move(f), std::move(g), std::move(h), std::move(i), std::move(j), std::move(k));
});
});
            }
        }
        template<typename A>
        size_hint::SizeHint size_hint() const {
            constexpr size_t K = 1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + 0))))))))))))))))))))));
            auto [n_min, n_max] = rusty::detail::deref_if_pointer_like(rusty::size_hint(this->iter));
            n_min = checked_binomial(std::move(n_min), K).unwrap_or(std::numeric_limits<size_t>::max());
            n_max = n_max.and_then([&](auto&& n) { return checked_binomial(std::move(n), K); });
            return size_hint::add(rusty::size_hint(this->c), std::make_tuple(std::move(n_min), std::move(n_max)));
        }
        template<typename A>
        size_t count() {
            constexpr size_t K = 1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + ((1 + 0))))))))))))))))))))));
            auto n = this->iter.count();
            return checked_binomial(std::move(n), K).unwrap() + this->c.count();
        }
        template<typename B, typename F, typename A>
        B fold(B init, F f) {
            using CurrTuple [[maybe_unused]] = std::tuple<A, A, A, A, A, A, A, A, A, A, A, A>;
            using PrevTuple [[maybe_unused]] = std::tuple<A, A, A, A, A, A, A, A, A, A, A>;
            const auto map_fn = [](const auto& z) {
                return [=, z = std::move(z)](auto&& _destruct_param0) mutable -> CurrTuple {
auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& c = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& d = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& e = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& f = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& g = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& h = rusty::detail::deref_if_pointer(std::get<7>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& i = rusty::detail::deref_if_pointer(std::get<8>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& j = rusty::detail::deref_if_pointer(std::get<9>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& k = rusty::detail::deref_if_pointer(std::get<10>(rusty::detail::deref_if_pointer(_destruct_param0)));
return std::make_tuple(rusty::clone(z), std::move(a), std::move(b), std::move(c), std::move(d), std::move(e), std::move(f), std::move(g), std::move(h), std::move(i), std::move(j), std::move(k));
};
            };
            auto&& _let_pat = (*this);
            auto&& c = rusty::detail::deref_if_pointer(_let_pat.c);
            auto&& item = rusty::detail::deref_if_pointer(_let_pat.item);
            auto iter = rusty::detail::deref_if_pointer(_let_pat.iter);
            if (auto&& _iflet_scrutinee = item.as_ref(); _iflet_scrutinee.is_some()) {
                decltype(auto) z = _iflet_scrutinee.unwrap();
                init = rusty::fold(c.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            while (true) {
                auto&& _whilelet = iter.next();
                if (!(rusty::detail::option_has_value(_whilelet))) { break; }
                auto z = rusty::detail::option_take_value(_whilelet);
                Tuple11Combination<I> c_shadow1 = rusty::from_into<Tuple11Combination<I>>(rusty::clone(iter));
                init = rusty::fold(c_shadow1.map(map_fn(rusty::detail::deref_if_pointer_like(z))), std::move(init), &f);
            }
            return init;
        }
    };


    // Rust-only libtest metadata const skipped: test_checked_binomial (marker: adaptors::test_checked_binomial, should_panic: no)

    /// An iterator adapter to filter values within a nested `Result::Ok`.
    ///
    /// See [`.filter_ok()`](crate::Itertools::filter_ok) for more information.
    template<typename I, typename F>
    struct FilterOk {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        I iter;
        F f;

        FilterOk<I, F> clone() const {
            return FilterOk<I, F>{.iter = rusty::clone(this->iter), .f = rusty::clone(this->f)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("FilterOk").field("iter", &this->iter).finish();
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename T, typename E>
        auto next() {
            auto& f = this->f;
            return this->iter.find([&](auto&& res) { return [&]() { auto&& _m = res; if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& t = rusty::detail::deref_if_pointer(_mv0); return f(std::move(t)); } if (true) { return true; } rusty::intrinsics::unreachable(); }(); });
        }
        template<typename T, typename E>
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return std::make_tuple(static_cast<size_t>(0), rusty::size_hint(this->iter)._1);
        }
        template<typename Acc, typename Fold, typename T, typename E>
        Acc fold(Acc init, Fold fold_f) {
            auto f = std::move(this->f);
            return rusty::fold(this->iter.filter([&](auto&& v) { return v.as_ref().map(f).unwrap_or(true); }), std::move(init), std::move(fold_f));
        }
        template<typename C, typename T, typename E>
        C collect() {
            auto f = std::move(this->f);
            return C::from_iter(this->iter.filter([&](auto&& v) { return v.as_ref().map(f).unwrap_or(true); }));
        }
        template<typename T, typename E>
        auto next_back() {
            auto& f = this->f;
            return this->iter.rfind([&](auto&& res) { return [&]() { auto&& _m = res; if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& t = rusty::detail::deref_if_pointer(_mv0); return f(std::move(t)); } if (true) { return true; } rusty::intrinsics::unreachable(); }(); });
        }
        template<typename Acc, typename Fold, typename T, typename E>
        Acc rfold(Acc init, Fold fold_f) {
            auto f = std::move(this->f);
            return this->iter.filter([&](auto&& v) { return v.as_ref().map(f).unwrap_or(true); }).rfold(std::move(init), std::move(fold_f));
        }
    };

    /// An iterator adapter to filter and apply a transformation on values within a nested `Result::Ok`.
    ///
    /// See [`.filter_map_ok()`](crate::Itertools::filter_map_ok) for more information.
    template<typename I, typename F>
    struct FilterMapOk {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        I iter;
        F f;

        FilterMapOk<I, F> clone() const {
            return FilterMapOk<I, F>{.iter = rusty::clone(this->iter), .f = rusty::clone(this->f)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("FilterMapOk").field("iter", &this->iter).finish();
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename T, typename U, typename E>
        auto next() {
            auto& f = this->f;
            return this->iter.find_map([&](auto&& res) { return [&]() { auto&& _m = res; if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& t = rusty::detail::deref_if_pointer(_mv0); return f(std::move(t)).map(rusty::Ok); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& e = rusty::detail::deref_if_pointer(_mv1); return rusty::Some(rusty::Err(std::move(e))); } rusty::intrinsics::unreachable(); }(); });
        }
        template<typename T, typename U, typename E>
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return std::make_tuple(static_cast<size_t>(0), rusty::size_hint(this->iter)._1);
        }
        template<typename Acc, typename Fold, typename T, typename U, typename E>
        Acc fold(Acc init, Fold fold_f) {
            auto f = std::move(this->f);
            return rusty::fold(rusty::filter_map(this->iter, [&](auto&& v) { return transpose_result(v.map(f)); }), std::move(init), std::move(fold_f));
        }
        template<typename C, typename T, typename U, typename E>
        C collect() {
            auto f = std::move(this->f);
            return C::from_iter(rusty::filter_map(this->iter, [&](auto&& v) { return transpose_result(v.map(f)); }));
        }
        template<typename T, typename U, typename E>
        auto next_back() {
            auto& f = this->f;
            return this->iter.by_ref().rev().find_map([&](auto&& res) { return [&]() { auto&& _m = res; if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& t = rusty::detail::deref_if_pointer(_mv0); return f(std::move(t)).map(rusty::Ok); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& e = rusty::detail::deref_if_pointer(_mv1); return rusty::Some(rusty::Err(std::move(e))); } rusty::intrinsics::unreachable(); }(); });
        }
        template<typename Acc, typename Fold, typename T, typename U, typename E>
        Acc rfold(Acc init, Fold fold_f) {
            auto f = std::move(this->f);
            return rusty::filter_map(this->iter, [&](auto&& v) { return transpose_result(v.map(f)); }).rfold(std::move(init), std::move(fold_f));
        }
    };

    /// An iterator adapter to get the positions of each element that matches a predicate.
    ///
    /// See [`.positions()`](crate::Itertools::positions) for more information.
    template<typename I, typename F>
    struct Positions {
        using Item = std::conditional_t<true, size_t, I>;
        decltype(rusty::enumerate(std::declval<I>())) iter;
        F f;

        Positions<I, F> clone() const {
            return Positions<I, F>{.iter = rusty::clone(this->iter), .f = rusty::clone(this->f)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("Positions").field("iter", &this->iter).finish();
        }
        rusty::Option<Item> next() {
            auto& f = this->f;
            return this->iter.find_map([&](auto&& _destruct_param0) {
auto&& count = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
return rusty::then_some(f(std::move(val)), std::move(count));
});
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return std::make_tuple(static_cast<size_t>(0), rusty::size_hint(this->iter)._1);
        }
        template<typename B, typename G>
        B fold(B init, G func) {
            auto f = std::move(this->f);
            return rusty::fold(this->iter, std::move(init), [&](auto&& acc, auto&& _destruct_param1) {
auto&& count = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param1)));
auto&& val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param1)));
if (f(std::move(val))) {
    acc = func(std::move(acc), std::move(count));
}
return acc;
});
        }
        rusty::Option<Item> next_back() {
            auto& f = this->f;
            return this->iter.by_ref().rev().find_map([&](auto&& _destruct_param0) {
auto&& count = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
return rusty::then_some(f(std::move(val)), std::move(count));
});
        }
        template<typename B, typename G>
        B rfold(B init, G func) {
            auto f = std::move(this->f);
            return this->iter.rfold(std::move(init), [&](auto&& acc, auto&& _destruct_param1) {
auto&& count = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param1)));
auto&& val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param1)));
if (f(std::move(val))) {
    acc = func(std::move(acc), std::move(count));
}
return acc;
});
        }
    };

    /// An iterator adapter to apply a mutating function to each element before yielding it.
    ///
    /// See [`.update()`](crate::Itertools::update) for more information.
    template<typename I, typename F>
    struct Update {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        I iter;
        F f;

        Update<I, F> clone() const {
            return Update<I, F>{.iter = rusty::clone(this->iter), .f = rusty::clone(this->f)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("Update").field("iter", &this->iter).finish();
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            if (auto&& _iflet_scrutinee = this->iter.next(); _iflet_scrutinee.is_some()) {
                decltype(auto) v = _iflet_scrutinee.unwrap();
                (this->f)(v);
                return rusty::Some(std::move(v));
            } else {
                return rusty::None;
            }
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return rusty::size_hint(this->iter);
        }
        template<typename Acc, typename G>
        Acc fold(Acc init, G g) {
            auto f = std::move(this->f);
            return rusty::fold(this->iter, std::move(init), [=, f = std::move(f), g = std::move(g)](auto&& acc, auto&& v) mutable {
f(v);
return g(std::move(acc), std::move(v));
});
        }
        template<typename C>
        C collect() {
            auto f = std::move(this->f);
            return C::from_iter(this->iter.map([=, f = std::move(f)](auto&& v) mutable {
f(v);
return v;
}));
        }
        auto next_back() {
            if (auto&& _iflet_scrutinee = this->iter.next_back(); _iflet_scrutinee.is_some()) {
                decltype(auto) v = _iflet_scrutinee.unwrap();
                (this->f)(v);
                return rusty::Some(std::move(v));
            } else {
                return rusty::None;
            }
        }
    };

}

namespace cons_tuples_impl {

    struct ConsTuplesFn;
    template<typename I>
    ConsTuples<typename I::IntoIter> cons_tuples(I iterable);

    using adaptors::map::MapSpecialCase;

    struct ConsTuplesFn {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out

        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        template<typename K, typename L, typename X>
        auto call(std::tuple<std::tuple<K, L>, X> _arg1);
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        template<typename J, typename K, typename L, typename X>
        auto call(std::tuple<std::tuple<J, K, L>, X> _arg1);
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        template<typename I, typename J, typename K, typename L, typename X>
        auto call(std::tuple<std::tuple<I, J, K, L>, X> _arg1);
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        template<typename H, typename I, typename J, typename K, typename L, typename X>
        auto call(std::tuple<std::tuple<H, I, J, K, L>, X> _arg1);
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        template<typename G, typename H, typename I, typename J, typename K, typename L, typename X>
        auto call(std::tuple<std::tuple<G, H, I, J, K, L>, X> _arg1);
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        template<typename F, typename G, typename H, typename I, typename J, typename K, typename L, typename X>
        auto call(std::tuple<std::tuple<F, G, H, I, J, K, L>, X> _arg1);
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        template<typename E, typename F, typename G, typename H, typename I, typename J, typename K, typename L, typename X>
        auto call(std::tuple<std::tuple<E, F, G, H, I, J, K, L>, X> _arg1);
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        template<typename D, typename E, typename F, typename G, typename H, typename I, typename J, typename K, typename L, typename X>
        auto call(std::tuple<std::tuple<D, E, F, G, H, I, J, K, L>, X> _arg1);
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        template<typename C, typename D, typename E, typename F, typename G, typename H, typename I, typename J, typename K, typename L, typename X>
        auto call(std::tuple<std::tuple<C, D, E, F, G, H, I, J, K, L>, X> _arg1);
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        template<typename B, typename C, typename D, typename E, typename F, typename G, typename H, typename I, typename J, typename K, typename L, typename X>
        auto call(std::tuple<std::tuple<B, C, D, E, F, G, H, I, J, K, L>, X> _arg1);
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
        ConsTuplesFn clone() const;
    };


}

namespace exactly_one_err {

    template<typename I>
    struct ExactlyOneError;


    using ::rusty::fmt::Formatter;
    using FmtResult = rusty::fmt::Result;


    // Rust-only unresolved import: using ::Either;

    namespace size_hint = ::size_hint;

    /// Iterator returned for the error case of `Itertools::exactly_one()`
    /// This iterator yields exactly the same elements as the input iterator.
    ///
    /// During the execution of `exactly_one` the iterator must be mutated.  This wrapper
    /// effectively "restores" the state of the input iterator when it's handed back.
    ///
    /// This is very similar to `PutBackN` except this iterator only supports 0-2 elements and does not
    /// use a `Vec`.
    template<typename I>
    struct ExactlyOneError {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        rusty::Option<rusty::Either<std::array<rusty::detail::associated_item_t<I>, 2>, rusty::detail::associated_item_t<I>>> first_two;
        I inner;

        ExactlyOneError<I> clone() const {
            return ExactlyOneError<I>{.first_two = rusty::clone(this->first_two), .inner = rusty::clone(this->inner)};
        }
        static ExactlyOneError<I> new_(rusty::Option<rusty::Either<std::array<rusty::detail::associated_item_t<I>, 2>, rusty::detail::associated_item_t<I>>> first_two, I inner) {
            return ExactlyOneError<I>{.first_two = std::move(first_two), .inner = std::move(inner)};
        }
        size_t additional_len() const {
            return [&]() -> size_t { auto&& _m = this->first_two; if (_m.is_some()) { auto&& _mv0 = std::as_const(_m).unwrap(); if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv0))>>>(rusty::detail::deref_if_pointer(_mv0))) { return static_cast<size_t>(2); } } if (_m.is_some()) { auto&& _mv1 = std::as_const(_m).unwrap(); if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv1))>>>(rusty::detail::deref_if_pointer(_mv1))) { return static_cast<size_t>(1); } } if (_m.is_none()) { return static_cast<size_t>(0); } return [&]() -> size_t { rusty::intrinsics::unreachable(); }(); }();
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            return [&]() { auto&& _m = this->first_two.take(); if (_m.is_some()) { auto&& _mv0 = std::as_const(_m).unwrap(); if ((std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv0))>>>(rusty::detail::deref_if_pointer(_mv0)) && rusty::len(rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv0))>>>(rusty::detail::deref_if_pointer(_mv0))._0)) == 2)) { auto&& first = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv0))>>>(rusty::detail::deref_if_pointer(_mv0))._0)[0]); auto&& second = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv0))>>>(rusty::detail::deref_if_pointer(_mv0))._0)[1]); return [&]() { this->first_two = rusty::Option<rusty::Either<std::array<rusty::detail::associated_item_t<I>, 2>, rusty::detail::associated_item_t<I>>>(rusty::either::Right<std::array<rusty::detail::associated_item_t<I>, 2>, rusty::detail::associated_item_t<I>>(rusty::detail::associated_item_t<I>(second)));
return rusty::Some(first); }(); } } if (_m.is_some()) { auto&& _mv1 = std::as_const(_m).unwrap(); if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv1))>>>(rusty::detail::deref_if_pointer(_mv1))) { auto&& second = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv1))>>>(rusty::detail::deref_if_pointer(_mv1))._0); return rusty::Some(second); } } if (_m.is_none()) { return this->inner.next(); } rusty::intrinsics::unreachable(); }();
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return size_hint::add_scalar(rusty::size_hint(this->inner), this->additional_len());
        }
        template<typename B, typename F>
        B fold(B init, F f) {
            {
                auto&& _m = this->first_two;
                bool _m_matched = false;
                if (!_m_matched) {
                    if (_m.is_some()) {
                        auto&& _mv0 = std::as_const(_m).unwrap();
                        auto&& first = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv0))>>>(rusty::detail::deref_if_pointer(_mv0))._0)[0]);
                        auto&& second = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv0))>>>(rusty::detail::deref_if_pointer(_mv0))._0)[1]);
                        if ((std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv0))>>>(rusty::detail::deref_if_pointer(_mv0)) && rusty::len(rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv0))>>>(rusty::detail::deref_if_pointer(_mv0))._0)) == 2)) {
                            init = f(std::move(init), first);
                            init = f(std::move(init), second);
                            _m_matched = true;
                        }
                    }
                }
                if (!_m_matched) {
                    if (_m.is_some()) {
                        auto&& _mv1 = std::as_const(_m).unwrap();
                        auto&& second = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv1))>>>(rusty::detail::deref_if_pointer(_mv1))._0);
                        if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv1))>>>(rusty::detail::deref_if_pointer(_mv1))) {
                            init = f(std::move(init), second);
                            _m_matched = true;
                        }
                    }
                }
                if (!_m_matched) {
                    if (_m.is_none()) {
                        _m_matched = true;
                    }
                }
            }
            return rusty::fold(this->inner, std::move(init), std::move(f));
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            const auto additional = this->additional_len();
            if (additional > 0) {
                return rusty::write_fmt(f, std::string("got at least 2 elements when exactly one was expected"));
            } else {
                return rusty::write_fmt(f, std::string("got zero elements when exactly one was expected"));
            }
        }
    };

}

namespace flatten_ok {

    template<typename I, typename T, typename E>
    struct FlattenOk;
    template<typename I, typename T, typename E>
    FlattenOk<I, T, E> flatten_ok(I iter);

    namespace size_hint = ::size_hint;

    namespace fmt = rusty::fmt;

    /// An iterator adaptor that flattens `Result::Ok` values and
    /// allows `Result::Err` values through unchanged.
    ///
    /// See [`.flatten_ok()`](crate::Itertools::flatten_ok) for more information.
    template<typename I, typename T, typename E>
    struct FlattenOk {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        I iter;
        rusty::Option<typename T::IntoIter> inner_front;
        rusty::Option<typename T::IntoIter> inner_back;

        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            while (true) {
                if (this->inner_front.is_some()) {
                    decltype(auto) inner = this->inner_front.unwrap();
                    if (auto&& _iflet_scrutinee = inner.next(); rusty::detail::option_has_value(_iflet_scrutinee)) {
                        auto&& _iflet_take = _iflet_scrutinee;
                        auto item = rusty::detail::option_take_value(_iflet_take);
                        return rusty::Some(rusty::Result<rusty::detail::associated_item_t<T>, E>::Ok(std::move(item)));
                    }
                    this->inner_front = rusty::Option<typename T::IntoIter>(rusty::None);
                }
                {
                    auto&& _m = this->iter.next();
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_some()) {
                            auto&& _mv0 = std::as_const(_m).unwrap();
                            auto&& ok = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_mv0)).unwrap());
                            if (rusty::detail::deref_if_pointer(_mv0).is_ok()) {
                                this->inner_front = rusty::Option<typename T::IntoIter>(rusty::iter(ok));
                                _m_matched = true;
                            }
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_some()) {
                            auto&& _mv1 = std::as_const(_m).unwrap();
                            auto&& e = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_mv1)).unwrap_err());
                            if (rusty::detail::deref_if_pointer(_mv1).is_err()) {
                                return rusty::Some(rusty::Result<rusty::detail::associated_item_t<T>, E>::Err(e));
                                _m_matched = true;
                            }
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_none()) {
                            if (this->inner_back.is_some()) {
                                decltype(auto) inner = this->inner_back.unwrap();
                                if (auto&& _iflet_scrutinee = inner.next(); rusty::detail::option_has_value(_iflet_scrutinee)) {
                                    auto&& _iflet_take = _iflet_scrutinee;
                                    auto item = rusty::detail::option_take_value(_iflet_take);
                                    return rusty::Some(rusty::Result<rusty::detail::associated_item_t<T>, E>::Ok(std::move(item)));
                                }
                                this->inner_back = rusty::Option<typename T::IntoIter>(rusty::None);
                            } else {
                                return rusty::None;
                            }
                            _m_matched = true;
                        }
                    }
                }
            }
        }
        template<typename B, typename F>
        B fold(B init, F f) {
            auto acc = [&]() { auto&& _m = this->inner_front; if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& x = rusty::detail::deref_if_pointer(_mv0); return rusty::fold(x, std::move(init), [&](auto&& a, auto&& o) { return f(std::move(a), rusty::Ok(std::move(o))); }); } if (_m.is_none()) { return init; } rusty::intrinsics::unreachable(); }();
            acc = rusty::fold(this->iter, std::move(acc), [&](auto&& acc, auto&& x) { return [&]() { auto&& _m = x; if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& it = rusty::detail::deref_if_pointer(_mv0); return rusty::fold(rusty::iter(std::move(it)), std::move(acc), [&](auto&& a, auto&& o) { return f(std::move(a), rusty::Ok(std::move(o))); }); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& e = rusty::detail::deref_if_pointer(_mv1); return f(std::move(acc), rusty::Err(std::move(e))); } rusty::intrinsics::unreachable(); }(); });
            return [&]() -> B { auto&& _m = this->inner_back; if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& x = rusty::detail::deref_if_pointer(_mv0); return rusty::fold(x, std::move(acc), [&](auto&& a, auto&& o) { return f(std::move(a), rusty::Ok(std::move(o))); }); } if (_m.is_none()) { return acc; } return [&]() -> B { rusty::intrinsics::unreachable(); }(); }();
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            const auto inner_hint = [&](const rusty::Option<typename T::IntoIter>& inner) {
return inner.as_ref().map([](const auto& _v) { return rusty::size_hint(_v); }).unwrap_or(std::make_tuple(0, rusty::Option<int32_t>(0)));
};
            auto inner_front = inner_hint(&this->inner_front);
            auto inner_back = inner_hint(&this->inner_back);
            auto outer = [&]() { auto&& _m = rusty::size_hint(this->iter); return std::visit(overloaded { [&](auto&&) { return rusty::intrinsics::unreachable(); }, [&](auto&&) { return std::make_tuple(0, rusty::None); } }, std::move(_m)); }();
            return size_hint::add(size_hint::add(std::move(inner_front), std::move(inner_back)), std::move(outer));
        }
        auto next_back() {
            while (true) {
                if (this->inner_back.is_some()) {
                    decltype(auto) inner = this->inner_back.unwrap();
                    if (auto&& _iflet_scrutinee = inner.next_back(); rusty::detail::option_has_value(_iflet_scrutinee)) {
                        auto&& _iflet_take = _iflet_scrutinee;
                        auto item = rusty::detail::option_take_value(_iflet_take);
                        return rusty::Some(rusty::Result<rusty::detail::associated_item_t<T>, E>::Ok(std::move(item)));
                    }
                    this->inner_back = rusty::Option<typename T::IntoIter>(rusty::None);
                }
                {
                    auto&& _m = this->iter.next_back();
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_some()) {
                            auto&& _mv0 = std::as_const(_m).unwrap();
                            auto&& ok = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_mv0)).unwrap());
                            if (rusty::detail::deref_if_pointer(_mv0).is_ok()) {
                                this->inner_back = rusty::Option<typename T::IntoIter>(rusty::iter(ok));
                                _m_matched = true;
                            }
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_some()) {
                            auto&& _mv1 = std::as_const(_m).unwrap();
                            auto&& e = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_mv1)).unwrap_err());
                            if (rusty::detail::deref_if_pointer(_mv1).is_err()) {
                                return rusty::Some(rusty::Result<rusty::detail::associated_item_t<T>, E>::Err(e));
                                _m_matched = true;
                            }
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_none()) {
                            if (this->inner_front.is_some()) {
                                decltype(auto) inner = this->inner_front.unwrap();
                                if (auto&& _iflet_scrutinee = inner.next_back(); rusty::detail::option_has_value(_iflet_scrutinee)) {
                                    auto&& _iflet_take = _iflet_scrutinee;
                                    auto item = rusty::detail::option_take_value(_iflet_take);
                                    return rusty::Some(rusty::Result<rusty::detail::associated_item_t<T>, E>::Ok(std::move(item)));
                                }
                                this->inner_front = rusty::Option<typename T::IntoIter>(rusty::None);
                            } else {
                                return rusty::None;
                            }
                            _m_matched = true;
                        }
                    }
                }
            }
        }
        template<typename B, typename F>
        B rfold(B init, F f) {
            auto acc = [&]() { auto&& _m = this->inner_back; if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& x = rusty::detail::deref_if_pointer(_mv0); return x.rfold(std::move(init), [&](auto&& a, auto&& o) { return f(std::move(a), rusty::Ok(std::move(o))); }); } if (_m.is_none()) { return init; } rusty::intrinsics::unreachable(); }();
            acc = this->iter.rfold(std::move(acc), [&](auto&& acc, auto&& x) { return [&]() { auto&& _m = x; if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& it = rusty::detail::deref_if_pointer(_mv0); return rusty::iter(std::move(it)).rfold(std::move(acc), [&](auto&& a, auto&& o) { return f(std::move(a), rusty::Ok(std::move(o))); }); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& e = rusty::detail::deref_if_pointer(_mv1); return f(std::move(acc), rusty::Err(std::move(e))); } rusty::intrinsics::unreachable(); }(); });
            return [&]() -> B { auto&& _m = this->inner_front; if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& x = rusty::detail::deref_if_pointer(_mv0); return x.rfold(std::move(acc), [&](auto&& a, auto&& o) { return f(std::move(a), rusty::Ok(std::move(o))); }); } if (_m.is_none()) { return acc; } return [&]() -> B { rusty::intrinsics::unreachable(); }(); }();
        }
        FlattenOk<I, T, E> clone() const {
            return FlattenOk<I, T, E>{.iter = rusty::clone(this->iter), .inner_front = rusty::clone(this->inner_front), .inner_back = rusty::clone(this->inner_back)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("FlattenOk").field("iter", &this->iter).field("inner_front", &this->inner_front).field("inner_back", &this->inner_back).finish();
        }
    };

}

namespace grouping_map {

    template<typename F>
    struct GroupingMapFn;
    template<typename I>
    struct GroupingMap;
    using minmax::MinMaxResult;
    template<typename K, typename I, typename F>
    MapForGrouping<I, F> new_map_for_grouping(I iter, F key_mapper);
    template<typename I, typename K, typename V>
    GroupingMap<I> new_(I iter);

    using adaptors::map::MapSpecialCase;
    using minmax::MinMaxResult;
    using namespace minmax;

    using ::rusty::cmp::Ordering;

    using ::rusty::HashMap;





    template<typename F>
    struct GroupingMapFn {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        F _0;

        GroupingMapFn<F> clone() const {
            return GroupingMapFn(rusty::clone(this->_0));
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("GroupingMapFn").finish();
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Out
        template<typename V, typename K>
        auto call(V v) {
            return std::make_tuple((this->_0)(v), std::move(v));
        }
    };


    /// `GroupingMap` is an intermediate struct for efficient group-and-fold operations.
    /// It groups elements by their key and at the same time fold each group
    /// using some aggregating operation.
    ///
    /// No method on this struct performs temporary allocations.
    template<typename I>
    struct GroupingMap {
        I iter;

        GroupingMap<I> clone() const {
            return GroupingMap<I>{.iter = rusty::clone(this->iter)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field1_finish(f, "GroupingMap", "iter", &this->iter);
        }
        template<typename FO, typename R, typename K, typename V>
        rusty::HashMap<K, R> aggregate(FO operation) {
            auto destination_map = HashMap<K, R>::new_();
            this->iter.for_each([&](auto&& _destruct_param0) {
auto&& key = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
const auto acc = destination_map.remove(key);
if (auto&& _iflet_scrutinee = operation(std::move(acc), key, std::move(val)); _iflet_scrutinee.is_some()) {
    decltype(auto) op_res = _iflet_scrutinee.unwrap();
    destination_map.insert(std::move(key), std::move(op_res));
}
});
            return destination_map;
        }
        template<typename FI, typename FO, typename K, typename V>
        auto fold_with(FI init, FO operation) {
            using R = std::remove_cvref_t<std::invoke_result_t<FI&, const K&, const V&>>;
            return this->aggregate([&](auto&& acc, auto&& key, auto&& val) {
const auto acc_shadow1 = acc.unwrap_or_else([&]() { return init(std::move(key), val); });
return rusty::Some(operation(std::move(acc_shadow1), std::move(key), std::move(val)));
});
        }
        template<typename FO, typename R, typename K, typename V>
        rusty::HashMap<K, R> fold(R init, FO operation) {
            return this->fold_with([&](auto _closure_wild0, auto _closure_wild1) { return rusty::clone(init); }, std::move(operation));
        }
        template<typename FO, typename K, typename V>
        rusty::HashMap<K, V> reduce(FO operation) {
            return this->aggregate([&](auto&& acc, auto&& key, auto&& val) {
return rusty::Some([&]() { auto&& _m = acc; if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& acc = rusty::detail::deref_if_pointer(_mv0); return operation(std::move(acc), std::move(key), std::move(val)); } if (_m.is_none()) { return val; } rusty::intrinsics::unreachable(); }());
});
        }
        template<typename FO, typename K, typename V>
        rusty::HashMap<K, V> fold_first(FO operation) {
            return this->reduce(std::move(operation));
        }
        template<typename C, typename K, typename V>
        rusty::HashMap<K, C> collect() {
            auto destination_map = HashMap<K, C>::new_();
            this->iter.for_each([&](auto&& _destruct_param0) {
auto&& key = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
destination_map.entry(std::move(key)).extend(rusty::Some(std::move(val)));
});
            return destination_map;
        }
        template<typename K, typename V>
        rusty::HashMap<K, V> max() {
            return this->max_by([&](auto _closure_wild0, auto&& v1, auto&& v2) { return V::cmp(std::move(v1), std::move(v2)); });
        }
        template<typename F, typename K, typename V>
        rusty::HashMap<K, V> max_by(F compare) {
            return this->reduce([&](auto&& acc, auto&& key, auto&& val) { return [&]() { auto&& _m = compare(std::move(key), acc, val); if (_m == Ordering::Less || _m == Ordering::Equal) return val;
if (_m == Ordering::Greater) return acc; }(); });
        }
        template<typename F, typename CK, typename K, typename V>
        rusty::HashMap<K, V> max_by_key(F f) {
            return this->max_by([&](auto&& key, auto&& v1, auto&& v2) { return rusty::cmp::cmp(f(std::move(key), std::move(v1)), f(std::move(key), std::move(v2))); });
        }
        template<typename K, typename V>
        rusty::HashMap<K, V> min() {
            return this->min_by([&](auto _closure_wild0, auto&& v1, auto&& v2) { return V::cmp(std::move(v1), std::move(v2)); });
        }
        template<typename F, typename K, typename V>
        rusty::HashMap<K, V> min_by(F compare) {
            return this->reduce([&](auto&& acc, auto&& key, auto&& val) { return [&]() { auto&& _m = compare(std::move(key), acc, val); if (_m == Ordering::Less || _m == Ordering::Equal) return acc;
if (_m == Ordering::Greater) return val; }(); });
        }
        template<typename F, typename CK, typename K, typename V>
        rusty::HashMap<K, V> min_by_key(F f) {
            return this->min_by([&](auto&& key, auto&& v1, auto&& v2) { return rusty::cmp::cmp(f(std::move(key), std::move(v1)), f(std::move(key), std::move(v2))); });
        }
        template<typename K, typename V>
        rusty::HashMap<K, ::minmax::MinMaxResult<V>> minmax() {
            return this->minmax_by([&](auto _closure_wild0, auto&& v1, auto&& v2) { return V::cmp(std::move(v1), std::move(v2)); });
        }
        template<typename F, typename K, typename V>
        rusty::HashMap<K, ::minmax::MinMaxResult<V>> minmax_by(F compare) {
            return this->aggregate([&](auto&& acc, auto&& key, auto&& val) {
return rusty::Option<::minmax::MinMaxResult<I>>([&]() -> ::minmax::MinMaxResult<I> { auto&& _m = acc; if (_m.is_some()) { auto&& _mv0 = std::as_const(_m).unwrap(); if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv0))>>>(rusty::detail::deref_if_pointer(_mv0))) { auto&& e = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv0))>>>(rusty::detail::deref_if_pointer(_mv0))._0); return (compare(std::move(key), val, e) == Ordering::Less ? MinMaxResult_MinMax<I>{std::move(val), e} : MinMaxResult_MinMax<I>{e, std::move(val)}); } } if (_m.is_some()) { auto&& _mv1 = std::as_const(_m).unwrap(); if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv1))>>>(rusty::detail::deref_if_pointer(_mv1))) { auto&& min = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv1))>>>(rusty::detail::deref_if_pointer(_mv1))._0); auto&& max = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv1))>>>(rusty::detail::deref_if_pointer(_mv1))._1); return (compare(std::move(key), val, min) == Ordering::Less ? MinMaxResult_MinMax<I>{std::move(val), max} : (compare(std::move(key), val, max) != Ordering::Less ? MinMaxResult_MinMax<I>{min, std::move(val)} : MinMaxResult_MinMax<I>{min, max})); } } if (_m.is_none()) { return MinMaxResult_OneElement<I>{std::move(val)}; } if (_m.is_some()) { auto&& _mv3 = std::as_const(_m).unwrap(); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv3))>>>(rusty::detail::deref_if_pointer(_mv3))) { rusty::panicking::panic("internal error: entered unreachable code"); } } return [&]() -> ::minmax::MinMaxResult<I> { rusty::intrinsics::unreachable(); }(); }());
});
        }
        template<typename F, typename CK, typename K, typename V>
        rusty::HashMap<K, ::minmax::MinMaxResult<V>> minmax_by_key(F f) {
            return this->minmax_by([&](auto&& key, auto&& v1, auto&& v2) { return rusty::cmp::cmp(f(std::move(key), std::move(v1)), f(std::move(key), std::move(v2))); });
        }
        template<typename K, typename V>
        rusty::HashMap<K, V> sum() {
            return this->reduce([&](auto&& acc, auto _closure_wild1, auto&& val) { return acc + val; });
        }
        template<typename K, typename V>
        rusty::HashMap<K, V> product() {
            return this->reduce([&](auto&& acc, auto _closure_wild1, auto&& val) { return acc * val; });
        }
    };

}

namespace intersperse_tests {

    template<typename Item>
    struct IntersperseElementSimple;
    template<typename I, typename ElemF>
    struct IntersperseWith;
    template<typename I>
    decltype(std::declval<I>().intersperse(std::declval<typename I::Item>())) intersperse(I iter, rusty::detail::associated_item_t<I> elt);
    template<typename I, typename ElemF>
    decltype(std::declval<I>().intersperse_with(std::declval<ElemF>())) intersperse_with(I iter, ElemF elt);

    namespace size_hint = ::size_hint;



    template<typename Item>
    struct IntersperseElementSimple {
        Item _0;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, Item>::debug_tuple_field1_finish(f, "IntersperseElementSimple", &this->_0);
        }
        IntersperseElementSimple<Item> clone() const {
            return IntersperseElementSimple(rusty::clone(this->_0));
        }
        Item generate() {
            return rusty::clone(this->_0);
        }
    };


    /// An iterator adaptor to insert a particular value created by a function
    /// between each element of the adapted iterator.
    ///
    /// Iterator element type is `I::Item`
    ///
    /// This iterator is *fused*.
    ///
    /// See [`.intersperse_with()`](crate::Itertools::intersperse_with) for more information.
    template<typename I, typename ElemF>
    struct IntersperseWith {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        ElemF element;
        decltype(std::declval<I>().fuse()) iter;
        /// `peek` is None while no item have been taken out of `iter` (at definition).
        /// Then `peek` will alternatively be `Some(None)` and `Some(Some(item))`,
        /// where `None` indicates it's time to generate from `element` (unless `iter` is empty).
        rusty::Option<rusty::Option<rusty::detail::associated_item_t<I>>> peek;

        IntersperseWith<I, ElemF> clone() const {
            return IntersperseWith<I, ElemF>{.element = rusty::clone(this->element), .iter = rusty::clone(this->iter), .peek = rusty::clone(this->peek)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field3_finish(f, "IntersperseWith", "element", &this->element, "iter", &this->iter, "peek", &this->peek);
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            auto&& _let_pat = (*this);
            auto&& element = rusty::detail::deref_if_pointer(_let_pat.element);
            auto&& iter = rusty::detail::deref_if_pointer(_let_pat.iter);
            auto&& peek = rusty::detail::deref_if_pointer(_let_pat.peek);
            return [&]() { auto&& _m = peek; if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& item = rusty::detail::deref_if_pointer(_mv0); return item.take(); } if (_m.is_some()) { auto&& _mv1 = std::as_const(_m).unwrap(); if (_mv1.is_none()) { return [&]() { auto&& _m = iter.next(); if (true) { const auto& new_ = _m; return [&]() { rusty::detail::deref_if_pointer_like(peek) = rusty::Some(new_);
return rusty::Some(([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::intersperse_tests::rusty_ext::generate(std::forward<decltype(__self)>(__self)); }) { return ::intersperse_tests::rusty_ext::generate(std::forward<decltype(__self)>(__self)); } else { return ::intersperse_tests::rusty_ext::generate(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self))); } })(element)); }(); } if (_m.is_none()) { return rusty::None; } rusty::intrinsics::unreachable(); }(); } } if (_m.is_none()) { return [&]() { rusty::detail::deref_if_pointer_like(peek) = rusty::Some(rusty::None);
return iter.next(); }(); } rusty::intrinsics::unreachable(); }();
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto sh = rusty::size_hint(this->iter);
            sh = size_hint::add(std::move(sh), std::move(sh));
            return [&]() -> std::tuple<size_t, rusty::Option<size_t>> { auto&& _m = this->peek; if (_m.is_some()) { auto&& _mv0 = std::as_const(_m).unwrap(); if (rusty::detail::deref_if_pointer(_mv0).is_some()) { return size_hint::add_scalar(std::move(sh), static_cast<size_t>(1)); } } if (_m.is_some()) { auto&& _mv1 = std::as_const(_m).unwrap(); if (_mv1.is_none()) { return sh; } } if (_m.is_none()) { return size_hint::sub_scalar(std::move(sh), static_cast<size_t>(1)); } return [&]() -> std::tuple<size_t, rusty::Option<size_t>> { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename B, typename F>
        B fold(B init, F f) {
            auto&& _let_pat = (*this);
            auto element = rusty::detail::deref_if_pointer(_let_pat.element);
            auto iter = rusty::detail::deref_if_pointer(_let_pat.iter);
            auto&& peek = rusty::detail::deref_if_pointer(_let_pat.peek);
            auto accum = std::move(init);
            if (auto&& _iflet_scrutinee = peek.unwrap_or_else([&]() { return iter.next(); }); _iflet_scrutinee.is_some()) {
                decltype(auto) x = _iflet_scrutinee.unwrap();
                accum = f(std::move(accum), std::move(x));
            }
            return rusty::fold(iter, std::move(accum), [&](auto&& accum, auto&& x) {
const auto accum_shadow1 = f(std::move(accum), ([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::intersperse_tests::rusty_ext::generate(std::forward<decltype(__self)>(__self)); }) { return ::intersperse_tests::rusty_ext::generate(std::forward<decltype(__self)>(__self)); } else { return ::intersperse_tests::rusty_ext::generate(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self))); } })(element));
return f(std::move(accum_shadow1), std::move(x));
});
        }
    };

}

namespace kmerge_impl {

    template<typename I>
    struct HeadTail;
    struct KMergeByLt;
    template<typename I, typename F>
    struct KMergeBy;
    template<typename T, typename S>
    void heapify(std::span<T> data, S less_than);
    template<typename T, typename S>
    void sift_down(std::span<T> heap, size_t index, S less_than);
    template<typename I>
    KMerge<typename rusty::detail::associated_item_t<I>::IntoIter> kmerge(I iterable);
    template<typename I, typename F>
    KMergeBy<typename rusty::detail::associated_item_t<I>::IntoIter, F> kmerge_by(I iterable, F less_than);

    namespace size_hint = ::size_hint;

    using ::rusty::Vec;

    namespace fmt = rusty::fmt;


    using ::rusty::mem::replace;

    /// Head element and Tail iterator pair
    ///
    /// `PartialEq`, `Eq`, `PartialOrd` and `Ord` are implemented by comparing sequences based on
    /// first items (which are guaranteed to exist).
    ///
    /// The meanings of `PartialOrd` and `Ord` are reversed so as to turn the heap used in
    /// `KMerge` into a min-heap.
    template<typename I>
    struct HeadTail {
        rusty::detail::associated_item_t<I> head;
        I tail;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field2_finish(f, "HeadTail", "head", &this->head, "tail", &this->tail);
        }
        static rusty::Option<HeadTail<I>> new_(I it) {
            auto head = it.next();
            return head.map([&](auto&& h) -> HeadTail<I> { return HeadTail<I>{.head = std::move(h), .tail = std::move(it)}; });
        }
        auto next() {
            if (auto&& _iflet_scrutinee = this->tail.next(); _iflet_scrutinee.is_some()) {
                decltype(auto) next = _iflet_scrutinee.unwrap();
                return rusty::Option<rusty::detail::associated_item_t<I>>(replace(&this->head, std::move(next)));
            } else {
                return rusty::Option<rusty::detail::associated_item_t<I>>(rusty::None);
            }
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return size_hint::add_scalar(rusty::size_hint(this->tail), static_cast<size_t>(1));
        }
        HeadTail<I> clone() const {
            return HeadTail<I>{.head = rusty::clone(this->head), .tail = rusty::clone(this->tail)};
        }
    };



    struct KMergeByLt {

        KMergeByLt clone() const;
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
        template<typename T>
        bool kmerge_pred(const T& a, const T& b);
    };

    /// An iterator adaptor that merges an abitrary number of base iterators
    /// according to an ordering function.
    ///
    /// Iterator element type is `I::Item`.
    ///
    /// See [`.kmerge_by()`](crate::Itertools::kmerge_by) for more
    /// information.
    template<typename I, typename F>
    struct KMergeBy {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        rusty::Vec<HeadTail<I>> heap;
        F less_than;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("KMergeBy").field("heap", &this->heap).finish();
        }
        KMergeBy<I, F> clone() const {
            return KMergeBy<I, F>{.heap = rusty::clone(this->heap), .less_than = rusty::clone(this->less_than)};
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            if (rusty::is_empty(this->heap)) {
                return rusty::None;
            }
            auto result = [&]() { auto&& _iflet = this->heap[0].next(); return (_iflet.is_some() ? ([&]() { auto next = _iflet.unwrap(); return next; }()) : this->heap.swap_remove(0).head); }();
            auto& less_than = this->less_than;
            sift_down(this->heap, static_cast<size_t>(0), [&](auto&& a, auto&& b) {
return ([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::kmerge_impl::rusty_ext::kmerge_pred(std::forward<decltype(__self)>(__self), rusty::detail::deref_if_pointer_like(a.head), rusty::detail::deref_if_pointer_like(b.head)); }) { return ::kmerge_impl::rusty_ext::kmerge_pred(std::forward<decltype(__self)>(__self), rusty::detail::deref_if_pointer_like(a.head), rusty::detail::deref_if_pointer_like(b.head)); } else { return ::kmerge_impl::rusty_ext::kmerge_pred(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self)), rusty::detail::deref_if_pointer_like(a.head), rusty::detail::deref_if_pointer_like(b.head)); } })(less_than);
});
            return rusty::Some(std::move(result));
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return rusty::map(rusty::iter(this->heap), [&](auto&& i) { return rusty::size_hint(i); }).reduce(size_hint::add).unwrap_or(std::make_tuple(0, rusty::Option<int32_t>(0)));
        }
    };

}

namespace lazy_buffer {

    template<typename I>
    struct LazyBuffer;

    using ::rusty::Vec;



    namespace size_hint = ::size_hint;
    using size_hint::SizeHint;

    template<typename I>
    struct LazyBuffer {
        decltype(std::declval<I>().fuse()) it;
        rusty::Vec<rusty::detail::associated_item_t<I>> buffer;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field2_finish(f, "LazyBuffer", "it", &this->it, "buffer", &this->buffer);
        }
        LazyBuffer<I> clone() const {
            return LazyBuffer<I>(rusty::clone(this->it), rusty::clone(this->buffer));
        }
        static LazyBuffer<I> new_(I it) {
            return LazyBuffer<I>(it.fuse(), rusty::Vec<rusty::detail::associated_item_t<I>>::new_());
        }
        size_t len() const {
            return rusty::len(this->buffer);
        }
        size_hint::SizeHint size_hint() const {
            return size_hint::add_scalar(rusty::size_hint(this->it), rusty::len((*this)));
        }
        size_t count() {
            return rusty::len((*this)) + this->it.count();
        }
        bool get_next() {
            if (auto&& _iflet_scrutinee = this->it.next(); _iflet_scrutinee.is_some()) {
                decltype(auto) x = _iflet_scrutinee.unwrap();
                this->buffer.push(std::move(x));
                return true;
            } else {
                return false;
            }
        }
        void prefill(size_t len) {
            const auto buffer_len = rusty::len(this->buffer);
            if (len > buffer_len) {
                const auto delta = len - buffer_len;
                this->buffer.extend(this->it.by_ref().take(std::move(delta)));
            }
        }
        auto get_at(std::span<const size_t> indices) const {
            return rusty::Vec<rusty::detail::associated_item_t<I>>::from_iter(rusty::map(rusty::iter(indices), [&](auto&& i) { return rusty::clone(this->buffer[i]); }));
        }
        template<size_t K>
        auto get_array(std::array<size_t, rusty::sanitize_array_capacity<K>()> indices) const {
            return rusty::map(indices, [&](auto&& i) -> rusty::detail::associated_item_t<I> { return rusty::clone(this->buffer[i]); });
        }
        template<typename J>
        decltype(auto) operator[](J index) const {
            return ([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::iter_index::rusty_ext::index(std::forward<decltype(__self)>(__self), std::move(index)); }) { return ::iter_index::rusty_ext::index(std::forward<decltype(__self)>(__self), std::move(index)); } else { return ::iter_index::rusty_ext::index(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self)), std::move(index)); } })(this->buffer);
        }
    };

}

namespace combinations {

    template<typename I, typename Idx>
    struct CombinationsGeneric;
    using lazy_buffer::LazyBuffer;
    template<typename I>
    Combinations<I> combinations(I iter, size_t k);
    template<typename I, size_t K>
    ArrayCombinations<I, K> array_combinations(I iter);
    rusty::Option<size_t> remaining_for(size_t n, bool first, std::span<const size_t> indices);

    using ::std::array;


    namespace fmt = rusty::fmt;


    using lazy_buffer::LazyBuffer;

    using ::rusty::Vec;

    using adaptors::checked_binomial;



    /// An iterator to iterate through all the `k`-length combinations in an iterator.
    ///
    /// See [`.combinations()`](crate::Itertools::combinations) and [`.array_combinations()`](crate::Itertools::array_combinations) for more information.
    template<typename I, typename Idx>
    struct CombinationsGeneric {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        Idx indices;
        lazy_buffer::LazyBuffer<I> pool;
        bool first;

        CombinationsGeneric<I, Idx> clone() const {
            return CombinationsGeneric<I, Idx>{.indices = rusty::clone(this->indices), .pool = rusty::clone(this->pool), .first = rusty::clone(this->first)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("Combinations").field("indices", &this->indices).field("pool", &this->pool).field("first", &this->first).finish();
        }
        static CombinationsGeneric<I, Idx> new_(I iter, Idx indices) {
            return CombinationsGeneric<I, Idx>{.indices = std::move(indices), .pool = lazy_buffer::LazyBuffer<I>::new_(std::move(iter)), .first = true};
        }
        size_t k() const {
            return rusty::len(this->indices);
        }
        size_t n() const {
            return rusty::len(this->pool);
        }
        const lazy_buffer::LazyBuffer<I>& src() const {
            return this->pool;
        }
        std::tuple<size_t, size_t> n_and_count() {
            auto&& _let_pat = (*this);
            auto&& indices = rusty::detail::deref_if_pointer(_let_pat.indices);
            auto&& pool = rusty::detail::deref_if_pointer(_let_pat.pool);
            auto&& first = rusty::detail::deref_if_pointer(_let_pat.first);
            auto n = pool.count();
            return std::make_tuple(std::move(n), remaining_for(std::move(n), std::move(first), indices.borrow()).unwrap());
        }
        bool init() {
            this->pool.prefill(this->k());
            auto done = this->k() > this->n();
            if (!done) {
                this->first = false;
            }
            return done;
        }
        bool increment_indices() {
            auto& indices = this->indices.borrow_mut();
            if (rusty::is_empty(indices)) {
                return true;
            }
            size_t i = rusty::len(indices) - 1;
            if (indices[i] == (rusty::len(this->pool) - 1)) {
                this->pool.get_next();
            }
            while (indices[i] == ((i + rusty::len(this->pool)) - rusty::len(indices))) {
                if (i > 0) {
                    [&]() { static_cast<void>(i -= 1); return std::make_tuple(); }();
                } else {
                    return true;
                }
            }
            [&]() { static_cast<void>(indices[i] += 1); return std::make_tuple(); }();
            for (auto&& j : rusty::for_in(rusty::range(i + 1, rusty::len(indices)))) {
                indices[j] = indices[j - 1] + 1;
            }
            return false;
        }
        auto try_nth(size_t n) {
            const auto done = (this->first ? this->init() : this->increment_indices());
            if (done) {
                return rusty::Result<rusty::detail::associated_item_t<CombinationsGeneric<I, Idx>>, size_t>::Err(static_cast<size_t>(0));
            }
            for (auto&& i : rusty::for_in(rusty::range(0, n))) {
                if (this->increment_indices()) {
                    return rusty::Result<rusty::detail::associated_item_t<CombinationsGeneric<I, Idx>>, size_t>::Err(i + 1);
                }
            }
            return rusty::Result<rusty::detail::associated_item_t<CombinationsGeneric<I, Idx>>, size_t>::Ok(([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::combinations::rusty_ext::extract_item(std::forward<decltype(__self)>(__self), this->pool); }) { return ::combinations::rusty_ext::extract_item(std::forward<decltype(__self)>(__self), this->pool); } else { return ::combinations::rusty_ext::extract_item(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self)), this->pool); } })(this->indices));
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            const auto done = (this->first ? this->init() : this->increment_indices());
            if (done) {
                return rusty::None;
            }
            return rusty::Some(([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::combinations::rusty_ext::extract_item(std::forward<decltype(__self)>(__self), this->pool); }) { return ::combinations::rusty_ext::extract_item(std::forward<decltype(__self)>(__self), this->pool); } else { return ::combinations::rusty_ext::extract_item(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self)), this->pool); } })(this->indices));
        }
        auto nth(size_t n) {
            return this->try_nth(std::move(n)).ok();
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto [low, upp] = rusty::detail::deref_if_pointer_like(this->pool.size_hint());
            low = remaining_for(std::move(low), this->first, this->indices.borrow()).unwrap_or(std::numeric_limits<size_t>::max());
            upp = upp.and_then([&](auto&& upp) { return remaining_for(std::move(upp), this->first, this->indices.borrow()); });
            return std::make_tuple(std::move(low), std::move(upp));
        }
        size_t count() {
            return std::get<1>(this->n_and_count());
        }
    };


    template<typename I>
    inline void __rusty_alias_Combinations_reset(auto& self_, size_t k) {
        self_.first = true;
        if (k < rusty::len(self_.indices)) {
            self_.indices.truncate(std::move(k));
            for (auto&& i : rusty::for_in(rusty::range(0, k))) {
                self_.indices[i] = std::move(i);
            }
        } else {
            for (auto&& i : rusty::for_in(rusty::range(0, rusty::len(self_.indices)))) {
                self_.indices[i] = std::move(i);
            }
            self_.indices.extend(rusty::range(rusty::len(self_.indices), k));
            self_.pool.prefill(std::move(k));
        }
    }


}

namespace combinations_with_replacement {

    template<typename I>
    struct CombinationsWithReplacement;
    using lazy_buffer::LazyBuffer;
    template<typename I>
    CombinationsWithReplacement<I> combinations_with_replacement(I iter, size_t k);
    rusty::Option<size_t> remaining_for(size_t n, bool first, std::span<const size_t> indices);

    using ::rusty::Box;

    using ::rusty::Vec;

    namespace fmt = rusty::fmt;


    using lazy_buffer::LazyBuffer;

    using adaptors::checked_binomial;

    /// An iterator to iterate through all the `n`-length combinations in an iterator, with replacement.
    ///
    /// See [`.combinations_with_replacement()`](crate::Itertools::combinations_with_replacement)
    /// for more information.
    template<typename I>
    struct CombinationsWithReplacement {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        rusty::Box<std::span<size_t>> indices;
        lazy_buffer::LazyBuffer<I> pool;
        bool first;

        CombinationsWithReplacement<I> clone() const {
            return CombinationsWithReplacement<I>{.indices = rusty::clone(this->indices), .pool = rusty::clone(this->pool), .first = rusty::clone(this->first)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("CombinationsWithReplacement").field("indices", &this->indices).field("pool", &this->pool).field("first", &this->first).finish();
        }
        bool increment_indices() {
            this->pool.get_next();
            rusty::Option<std::tuple<size_t, int32_t>> increment = rusty::Option<std::tuple<size_t, int32_t>>(rusty::None);
            for (auto&& _for_item : rusty::for_in(rusty::rev(rusty::enumerate(rusty::iter(this->indices))))) {
                auto&& i = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_for_item)));
                auto&& indices_int = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_for_item)));
                if (rusty::detail::deref_if_pointer_like(indices_int) < (rusty::len(this->pool) - 1)) {
                    increment = rusty::Option<std::tuple<size_t, int32_t>>(std::make_tuple(std::move(i), indices_int + 1));
                    break;
                }
            }
            return [&]() -> bool { auto&& _m = increment; if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& increment_from = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_mv0))); auto&& increment_value = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_mv0))); return [&]() -> bool { rusty::fill(rusty::slice_from(this->indices, increment_from), std::move(increment_value));
return false; }(); } if (_m.is_none()) { return true; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            if (this->first) {
                if (!(rusty::is_empty(this->indices) || this->pool.get_next())) {
                    return rusty::None;
                }
                this->first = false;
            } else if (this->increment_indices()) {
                return rusty::None;
            }
            return rusty::Some(this->pool.get_at(this->indices));
        }
        auto nth(size_t n) {
            if (this->first) {
                if (!(rusty::is_empty(this->indices) || this->pool.get_next())) {
                    return rusty::None;
                }
                this->first = false;
            } else if (this->increment_indices()) {
                return rusty::None;
            }
            for (auto&& _ : rusty::for_in(rusty::range(0, n))) {
                if (this->increment_indices()) {
                    return rusty::None;
                }
            }
            return rusty::Some(this->pool.get_at(this->indices));
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto [low, upp] = rusty::detail::deref_if_pointer_like(this->pool.size_hint());
            low = remaining_for(std::move(low), this->first, this->indices).unwrap_or(std::numeric_limits<size_t>::max());
            upp = upp.and_then([&](auto&& upp) { return remaining_for(std::move(upp), this->first, this->indices); });
            return std::make_tuple(std::move(low), std::move(upp));
        }
        size_t count() {
            auto&& _let_pat = (*this);
            auto&& indices = rusty::detail::deref_if_pointer(_let_pat.indices);
            auto&& pool = rusty::detail::deref_if_pointer(_let_pat.pool);
            auto&& first = rusty::detail::deref_if_pointer(_let_pat.first);
            auto n = pool.count();
            return remaining_for(std::move(n), std::move(first), indices).unwrap();
        }
    };

}

namespace merge_join {

    struct MergeLte;
    template<typename I, typename J, typename F>
    struct MergeBy;
    template<typename F, typename T>
    struct MergeFuncLR;
    using adaptors::PutBack;
    using either_or_both::EitherOrBoth;
    template<typename I, typename J>
    Merge<typename I::IntoIter, typename J::IntoIter> merge(I i, J j);
    template<typename I, typename J, typename F>
    MergeBy<typename I::IntoIter, typename J::IntoIter, F> merge_by_new(I a, J b, F cmp);
    template<typename I, typename J, typename F, typename T>
    MergeJoinBy<typename I::IntoIter, typename J::IntoIter, F> merge_join_by(I left, J right, F cmp_fn);

    using ::rusty::cmp::Ordering;

    namespace fmt = rusty::fmt;


    using ::rusty::PhantomData;

    // Rust-only unresolved import: using ::Either;

    using adaptors::put_back;
    using adaptors::PutBack;

    using either_or_both::EitherOrBoth;
    using namespace either_or_both;

    namespace size_hint = ::size_hint;
    using size_hint::SizeHint;

    struct MergeLte {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: MergeResult

        MergeLte clone() const;
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
        // Rust-only associated type alias with unbound generic skipped in constrained mode: MergeResult
        template<typename T>
        static auto left(T left);
        template<typename T>
        static auto right(T right);
        template<typename T>
        auto merge(T left, T right);
        template<typename T>
        static size_hint::SizeHint size_hint(size_hint::SizeHint left, size_hint::SizeHint right);
    };


    /// An iterator adaptor that merges the two base iterators in ascending order.
    /// If both base iterators are sorted (ascending), the result is sorted.
    ///
    /// Iterator element type is `I::Item`.
    ///
    /// See [`.merge_by()`](crate::Itertools::merge_by) for more information.
    template<typename I, typename J, typename F>
    struct MergeBy {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        adaptors::PutBack<decltype(std::declval<I>().fuse())> left;
        adaptors::PutBack<decltype(std::declval<J>().fuse())> right;
        F cmp_fn;

        MergeBy<I, J, F> clone() const {
            return MergeBy<I, J, F>{.left = rusty::clone(this->left), .right = rusty::clone(this->right), .cmp_fn = rusty::clone(this->cmp_fn)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("MergeBy").field("left", &this->left).field("right", &this->right).finish();
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            return [&]() { auto&& _m0 = this->left.next(); auto&& _m1 = this->right.next(); if (_m0.is_none() && _m1.is_none()) { return rusty::None; } if (rusty::detail::deref_if_pointer(_m0).is_some() && _m1.is_none()) { auto&& left = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); return rusty::Some(F::left(left)); } if (_m0.is_none() && rusty::detail::deref_if_pointer(_m1).is_some()) { auto&& right = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); return rusty::Some(F::right(right)); } if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some()) { auto&& left = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& right = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); return [&]() { auto [not_next, next] = rusty::detail::deref_if_pointer_like(([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::merge_join::rusty_ext::merge(std::forward<decltype(__self)>(__self), left, right); }) { return ::merge_join::rusty_ext::merge(std::forward<decltype(__self)>(__self), left, right); } else { return ::merge_join::rusty_ext::merge(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self)), left, right); } })(this->cmp_fn));
{
    auto&& _m = not_next;
    bool _m_matched = false;
    if (!_m_matched) {
        if (_m.is_some()) {
            auto&& _mv0 = std::as_const(_m).unwrap();
            auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv0))>>>(rusty::detail::deref_if_pointer(_mv0))._0);
            if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv0))>>>(rusty::detail::deref_if_pointer(_mv0))) {
                this->left.put_back(l);
                _m_matched = true;
            }
        }
    }
    if (!_m_matched) {
        if (_m.is_some()) {
            auto&& _mv1 = std::as_const(_m).unwrap();
            auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv1))>>>(rusty::detail::deref_if_pointer(_mv1))._0);
            if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv1))>>>(rusty::detail::deref_if_pointer(_mv1))) {
                this->right.put_back(r);
                _m_matched = true;
            }
        }
    }
    if (!_m_matched) {
        if (_m.is_none()) {
            _m_matched = true;
        }
    }
}
return rusty::Option<typename F::MergeResult>(std::move(next)); }(); } rusty::intrinsics::unreachable(); }();
        }
        template<typename B, typename G>
        B fold(B init, G f) {
            auto acc = std::move(init);
            auto left = this->left.next();
            auto right = this->right.next();
            while (true) {
                {
                    auto&& _m0 = left;
                    auto&& _m1 = right;
                    auto _m_tuple = std::forward_as_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched && ((rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple))).is_some() && rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple))).is_some()))) {
                        auto&& l = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)))).unwrap());
                        auto&& r = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)))).unwrap());
                        {
                            auto&& _m = ([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::merge_join::rusty_ext::merge(std::forward<decltype(__self)>(__self), l, r); }) { return ::merge_join::rusty_ext::merge(std::forward<decltype(__self)>(__self), l, r); } else { return ::merge_join::rusty_ext::merge(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self)), l, r); } })(this->cmp_fn);
                            std::visit(overloaded {
                                // TODO: unhandled match pattern
                                [&](const auto&) {},
                                // TODO: unhandled match pattern
                                [&](const auto&) {},
                                // TODO: unhandled match pattern
                                [&](const auto&) {},
                            }, _m);
                        }
                        _m_matched = true;
                    }
                    if (!_m_matched && ((rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple))).is_some() && std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)).is_none()))) {
                        auto&& l = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)))).unwrap());
                        this->left.put_back(l);
                        acc = rusty::fold(this->left, std::move(acc), [&](auto&& acc, auto&& x) { return f(std::move(acc), F::left(std::move(x))); });
                        break;
                        _m_matched = true;
                    }
                    if (!_m_matched && ((std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)).is_none() && rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple))).is_some()))) {
                        auto&& r = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)))).unwrap());
                        this->right.put_back(r);
                        acc = rusty::fold(this->right, std::move(acc), [&](auto&& acc, auto&& x) { return f(std::move(acc), F::right(std::move(x))); });
                        break;
                        _m_matched = true;
                    }
                    if (!_m_matched && ((std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)).is_none() && std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)).is_none()))) {
                        break;
                        _m_matched = true;
                    }
                }
            }
            return acc;
        }
        size_hint::SizeHint size_hint() const {
            return F::size_hint(rusty::size_hint(this->left), rusty::size_hint(this->right));
        }
        auto nth(size_t n) {
            while (true) {
                if (n == static_cast<size_t>(0)) {
                    return this->next();
                }
                [&]() { static_cast<void>(n -= 1); return std::make_tuple(); }();
                {
                    auto&& _m0 = this->left.next();
                    auto&& _m1 = this->right.next();
                    auto _m_tuple = std::forward_as_tuple(_m0, _m1);
                    bool _m_matched = false;
                    if (!_m_matched && ((std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)).is_none() && std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)).is_none()))) {
                        return rusty::None;
                        _m_matched = true;
                    }
                    if (!_m_matched && ((rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple))).is_some() && std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)).is_none()))) {
                        auto&& _left = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)))).unwrap());
                        return this->left.nth(std::move(n)).map(F::left);
                        _m_matched = true;
                    }
                    if (!_m_matched && ((std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)).is_none() && rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple))).is_some()))) {
                        auto&& _right = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)))).unwrap());
                        return this->right.nth(std::move(n)).map(F::right);
                        _m_matched = true;
                    }
                    if (!_m_matched && ((rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple))).is_some() && rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple))).is_some()))) {
                        auto&& left = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)))).unwrap());
                        auto&& right = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)))).unwrap());
                        auto [not_next, _tuple_ignore1] = rusty::detail::deref_if_pointer_like(([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::merge_join::rusty_ext::merge(std::forward<decltype(__self)>(__self), left, right); }) { return ::merge_join::rusty_ext::merge(std::forward<decltype(__self)>(__self), left, right); } else { return ::merge_join::rusty_ext::merge(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self)), left, right); } })(this->cmp_fn));
                        {
                            auto&& _m = not_next;
                            bool _m_matched = false;
                            if (!_m_matched) {
                                if (_m.is_some()) {
                                    auto&& _mv0 = std::as_const(_m).unwrap();
                                    auto&& l = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv0))>>>(rusty::detail::deref_if_pointer(_mv0))._0);
                                    if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv0))>>>(rusty::detail::deref_if_pointer(_mv0))) {
                                        this->left.put_back(l);
                                        _m_matched = true;
                                    }
                                }
                            }
                            if (!_m_matched) {
                                if (_m.is_some()) {
                                    auto&& _mv1 = std::as_const(_m).unwrap();
                                    auto&& r = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv1))>>>(rusty::detail::deref_if_pointer(_mv1))._0);
                                    if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_mv1))>>>(rusty::detail::deref_if_pointer(_mv1))) {
                                        this->right.put_back(r);
                                        _m_matched = true;
                                    }
                                }
                            }
                            if (!_m_matched) {
                                if (_m.is_none()) {
                                    _m_matched = true;
                                }
                            }
                        }
                        _m_matched = true;
                    }
                }
            }
        }
    };


    template<typename F, typename T>
    struct MergeFuncLR {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: MergeResult
        // Rust-only associated type alias with unbound generic skipped in constrained mode: MergeResult
        F _0;
        rusty::PhantomData<T> _1;

        MergeFuncLR<F, T> clone() const {
            return MergeFuncLR(rusty::clone(this->_0), rusty::clone(this->_1));
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, F>::debug_tuple_field2_finish(f, "MergeFuncLR", &this->_0, &this->_1);
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: MergeResult
        template<typename L, typename R>
        static auto left(L left) {
            return Left<L, L>(L(std::move(left)));
        }
        template<typename L, typename R>
        static auto right(R right) {
            return Right<L, L>(L(std::move(right)));
        }
        template<typename L, typename R>
        auto merge(L left, R right) {
            return [&]() { auto&& _m = this->_0(left, right); if (_m == Ordering::Equal) return std::make_tuple(rusty::None, either_or_both::EitherOrBoth_Both<F, F>{std::move(left), std::move(right)});
if (_m == Ordering::Less) return std::make_tuple(rusty::Some(rusty::either::Right<L, R>(R(std::move(right)))), Left<L, L>(L(std::move(left))));
if (_m == Ordering::Greater) return std::make_tuple(rusty::Some(rusty::either::Left<L, R>(L(std::move(left)))), Right<L, L>(L(std::move(right)))); }();
        }
        template<typename L, typename R>
        static size_hint::SizeHint size_hint(size_hint::SizeHint left, size_hint::SizeHint right) {
            auto [a_lower, a_upper] = rusty::detail::deref_if_pointer_like(left);
            auto [b_lower, b_upper] = rusty::detail::deref_if_pointer_like(right);
            auto lower = rusty::cmp::max(std::move(a_lower), std::move(b_lower));
            auto upper = [&]() -> rusty::Option<size_t> { auto&& _m0 = a_upper; auto&& _m1 = b_upper; if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some()) { auto&& x = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& y = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); return [&]() { auto&& _checked_lhs = x; return rusty::checked_add(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(y)); }(); } if (true) { return rusty::Option<size_t>(rusty::None); } return [&]() -> rusty::Option<size_t> { rusty::intrinsics::unreachable(); }(); }();
            return std::make_tuple(std::move(lower), std::move(upper));
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: MergeResult
    };



}

namespace multipeek_impl {

    template<typename I>
    struct MultiPeek;
    template<typename I>
    MultiPeek<typename I::IntoIter> multipeek(I iterable);

    namespace size_hint = ::size_hint;


    using ::rusty::VecDeque;


    /// See [`multipeek()`] for more information.
    template<typename I>
    struct MultiPeek {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        decltype(std::declval<I>().fuse()) iter;
        rusty::VecDeque<rusty::detail::associated_item_t<I>> buf;
        size_t index;

        MultiPeek<I> clone() const {
            return MultiPeek<I>{.iter = rusty::clone(this->iter), .buf = rusty::clone(this->buf), .index = rusty::clone(this->index)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field3_finish(f, "MultiPeek", "iter", &this->iter, "buf", &this->buf, "index", &this->index);
        }
        void reset_peek() {
            this->index = static_cast<size_t>(0);
        }
        auto peek() {
            auto ret = (this->index < rusty::len(this->buf) ? rusty::SomeRef(this->buf[this->index]) : ({ auto&& _m = this->iter.next(); std::optional<std::remove_cvref_t<decltype(([&]() -> decltype(auto) { auto _mv = _m.unwrap();
auto&& x = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
return ([&]() { this->buf.push_back(x);
return rusty::SomeRef(this->buf[this->index]); }()); })())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& x = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move([&]() { this->buf.push_back(x);
return rusty::SomeRef(this->buf[this->index]); }())); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::Option<const rusty::detail::associated_item_t<I>&>(rusty::None); } std::move(_match_value).value(); }));
            [&]() { static_cast<void>(this->index += 1); return std::make_tuple(); }();
            return ret;
        }
        template<typename F>
        auto peeking_next(F accept) {
            if (rusty::is_empty(this->buf)) {
                if (auto&& _iflet_scrutinee = this->peek(); _iflet_scrutinee.is_some()) {
                    decltype(auto) r = _iflet_scrutinee.unwrap();
                    if (!accept(std::move(r))) {
                        return rusty::None;
                    }
                }
            } else if (auto&& _iflet_scrutinee = this->buf.front(); _iflet_scrutinee.is_some()) {
                decltype(auto) r = _iflet_scrutinee.unwrap();
                if (!accept(std::move(r))) {
                    return rusty::None;
                }
            }
            return this->next();
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            this->index = static_cast<size_t>(0);
            return this->buf.pop_front().or_else([&]() { return this->iter.next(); });
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return size_hint::add_scalar(rusty::size_hint(this->iter), rusty::len(this->buf));
        }
        template<typename B, typename F>
        B fold(B init, F f) {
            init = rusty::fold(this->buf.into_iter(), std::move(init), &f);
            return rusty::fold(this->iter, std::move(init), std::move(f));
        }
    };

}

namespace pad_tail {

    template<typename I, typename F>
    struct PadUsing;
    template<typename I, typename F>
    PadUsing<I, F> pad_using(I iter, size_t min, F filler);

    namespace size_hint = ::size_hint;


    /// An iterator adaptor that pads a sequence to a minimum length by filling
    /// missing elements using a closure.
    ///
    /// Iterator element type is `I::Item`.
    ///
    /// See [`.pad_using()`](crate::Itertools::pad_using) for more information.
    template<typename I, typename F>
    struct PadUsing {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        decltype(std::declval<I>().fuse()) iter;
        size_t min;
        size_t pos;
        F filler;

        PadUsing<I, F> clone() const {
            return PadUsing<I, F>{.iter = rusty::clone(this->iter), .min = rusty::clone(this->min), .pos = rusty::clone(this->pos), .filler = rusty::clone(this->filler)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("PadUsing").field("iter", &this->iter).field("min", &this->min).field("pos", &this->pos).finish();
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            return [&]() { auto&& _m = this->iter.next(); if (_m.is_none()) { return [&]() {
if (this->pos < this->min) {
auto e = rusty::Some((this->filler)(this->pos));
[&]() { static_cast<void>(this->pos += 1); return std::make_tuple(); }();
return e;
} else {
return rusty::None;
}
}(); } if (true) { const auto& e = _m; return [&]() { [&]() { static_cast<void>(this->pos += 1); return std::make_tuple(); }();
return e; }(); } rusty::intrinsics::unreachable(); }();
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto tail = rusty::saturating_sub(this->min, rusty::detail::deref_if_pointer(this->pos));
            return size_hint::max(rusty::size_hint(this->iter), std::make_tuple(std::move(tail), rusty::Option<size_t>(std::move(tail))));
        }
        template<typename B, typename G>
        B fold(B init, G f) {
            auto pos = std::move(this->pos);
            init = rusty::fold(this->iter, std::move(init), [&](auto&& acc, auto&& item) {
[&]() { static_cast<void>(pos += 1); return std::make_tuple(); }();
return f(std::move(acc), std::move(item));
});
            return rusty::fold(rusty::map((rusty::range(pos, this->min)), std::move(this->filler)), std::move(init), std::move(f));
        }
        auto next_back() {
            if (this->min == static_cast<size_t>(0)) {
                return this->iter.next_back();
            } else if (rusty::len(this->iter) >= this->min) {
                [&]() { static_cast<void>(this->min -= 1); return std::make_tuple(); }();
                return this->iter.next_back();
            } else {
                [&]() { static_cast<void>(this->min -= 1); return std::make_tuple(); }();
                return rusty::Some((this->filler)(this->min));
            }
        }
        template<typename B, typename G>
        B rfold(B init, G f) {
            init = rusty::map((rusty::range(rusty::len(this->iter), this->min)), std::move(this->filler)).rfold(std::move(init), f);
            return this->iter.rfold(std::move(init), std::move(f));
        }
    };

}

namespace peek_nth {

    template<typename I>
    struct PeekNth;
    template<typename I>
    PeekNth<typename I::IntoIter> peek_nth(I iterable);

    namespace size_hint = ::size_hint;


    using ::rusty::VecDeque;


    /// See [`peek_nth()`] for more information.
    template<typename I>
    struct PeekNth {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        decltype(std::declval<I>().fuse()) iter;
        rusty::VecDeque<rusty::detail::associated_item_t<I>> buf;

        PeekNth<I> clone() const {
            return PeekNth<I>{.iter = rusty::clone(this->iter), .buf = rusty::clone(this->buf)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field2_finish(f, "PeekNth", "iter", &this->iter, "buf", &this->buf);
        }
        auto peek() {
            return this->peek_nth(static_cast<size_t>(0));
        }
        auto peek_mut() {
            return this->peek_nth_mut(static_cast<size_t>(0));
        }
        auto peek_nth(size_t n) {
            const auto unbuffered_items = rusty::saturating_sub((n + 1), rusty::detail::deref_if_pointer(rusty::len(this->buf)));
            this->buf.extend(this->iter.by_ref().take(std::move(unbuffered_items)));
            return rusty::get(this->buf, std::move(n));
        }
        auto peek_nth_mut(size_t n) {
            const auto unbuffered_items = rusty::saturating_sub((n + 1), rusty::detail::deref_if_pointer(rusty::len(this->buf)));
            this->buf.extend(this->iter.by_ref().take(std::move(unbuffered_items)));
            return rusty::get_mut(this->buf, std::move(n));
        }
        auto next_if(const auto& func) {
            return [&]() -> rusty::Option<rusty::detail::associated_item_t<I>> { auto&& _m = this->next(); if (_m.is_some()) { auto&& _mv0 = std::as_const(_m).unwrap(); auto&& item = rusty::detail::deref_if_pointer(_mv0); if (func(item)) { return rusty::Option<rusty::detail::associated_item_t<I>>(item); } } if (_m.is_some()) { auto&& _mv1 = _m.unwrap(); auto&& item = rusty::detail::deref_if_pointer(_mv1); return [&]() -> rusty::Option<rusty::detail::associated_item_t<I>> { this->buf.push_front(std::move(item));
return rusty::Option<rusty::detail::associated_item_t<I>>(rusty::None); }(); } if (true) { return rusty::Option<rusty::detail::associated_item_t<I>>(rusty::None); } return [&]() -> rusty::Option<rusty::detail::associated_item_t<I>> { rusty::intrinsics::unreachable(); }(); }();
        }
        template<typename T>
        auto next_if_eq(const T& expected) {
            return this->next_if([&](auto&& next) -> bool { return next == expected; });
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            return this->buf.pop_front().or_else([&]() { return this->iter.next(); });
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return size_hint::add_scalar(rusty::size_hint(this->iter), rusty::len(this->buf));
        }
        template<typename B, typename F>
        B fold(B init, F f) {
            init = rusty::fold(this->buf.into_iter(), std::move(init), &f);
            return rusty::fold(this->iter, std::move(init), std::move(f));
        }
        template<typename F>
        auto peeking_next(F accept) {
            RUSTY_TRY_OPT(this->peek().filter([&](auto&& item) { return accept(std::move(item)); }));
            return this->next();
        }
    };

}

namespace permutations {

    struct PermutationState;
    template<typename I>
    struct Permutations;
    using lazy_buffer::LazyBuffer;
    template<typename I>
    Permutations<I> permutations(I iter, size_t k);
    bool advance(std::span<size_t> indices, std::span<size_t> cycles);

    using ::rusty::Box;

    using ::rusty::Vec;

    namespace fmt = rusty::fmt;



    using lazy_buffer::LazyBuffer;

    namespace size_hint = ::size_hint;
    using size_hint::SizeHint;

    // Algebraic data type
    struct PermutationState_Start {
        size_t k;
    };
    struct PermutationState_Buffered {
        size_t k;
        size_t min_n;
    };
    struct PermutationState_Loaded {
        rusty::Box<std::span<size_t>> indices;
        rusty::Box<std::span<size_t>> cycles;
    };
    struct PermutationState_End {};
    PermutationState_Start Start(size_t k);
    PermutationState_Buffered Buffered(size_t k, size_t min_n);
    PermutationState_Loaded Loaded(rusty::Box<std::span<size_t>> indices, rusty::Box<std::span<size_t>> cycles);
    PermutationState_End End();
    struct PermutationState : std::variant<PermutationState_Start, PermutationState_Buffered, PermutationState_Loaded, PermutationState_End> {
        using variant = std::variant<PermutationState_Start, PermutationState_Buffered, PermutationState_Loaded, PermutationState_End>;
        using variant::variant;
        static PermutationState Start(size_t k) { return PermutationState{PermutationState_Start{.k = std::forward<decltype(k)>(k)}}; }
        static PermutationState Buffered(size_t k, size_t min_n) { return PermutationState{PermutationState_Buffered{.k = std::forward<decltype(k)>(k), .min_n = std::forward<decltype(min_n)>(min_n)}}; }
        static PermutationState Loaded(rusty::Box<std::span<size_t>> indices, rusty::Box<std::span<size_t>> cycles) { return PermutationState{PermutationState_Loaded{.indices = std::forward<decltype(indices)>(indices), .cycles = std::forward<decltype(cycles)>(cycles)}}; }
        static PermutationState End() { return PermutationState{PermutationState_End{}}; }


        PermutationState clone() const;
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const;
        size_hint::SizeHint size_hint_for(size_t n) const;
    };
    PermutationState_Start Start(size_t k) { return PermutationState_Start{.k = std::forward<size_t>(k)};  }
    PermutationState_Buffered Buffered(size_t k, size_t min_n) { return PermutationState_Buffered{.k = std::forward<size_t>(k), .min_n = std::forward<size_t>(min_n)};  }
    PermutationState_Loaded Loaded(rusty::Box<std::span<size_t>> indices, rusty::Box<std::span<size_t>> cycles) { return PermutationState_Loaded{.indices = std::forward<rusty::Box<std::span<size_t>>>(indices), .cycles = std::forward<rusty::Box<std::span<size_t>>>(cycles)};  }
    PermutationState_End End() { return PermutationState_End{};  }

    /// An iterator adaptor that iterates through all the `k`-permutations of the
    /// elements from an iterator.
    ///
    /// See [`.permutations()`](crate::Itertools::permutations) for
    /// more information.
    template<typename I>
    struct Permutations {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        lazy_buffer::LazyBuffer<I> vals;
        PermutationState state;

        Permutations<I> clone() const {
            return Permutations<I>{.vals = rusty::clone(this->vals), .state = rusty::clone(this->state)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("Permutations").field("vals", &this->vals).field("state", &this->state).finish();
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            auto&& _let_pat = (*this);
            auto&& vals = rusty::detail::deref_if_pointer(_let_pat.vals);
            auto&& state = rusty::detail::deref_if_pointer(_let_pat.state);
            return [&]() { auto&& _m = state; if ((std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m) && std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).k == 0)) { return [&]() { rusty::detail::deref_if_pointer_like(state) = PermutationState_End{};
return rusty::Some(rusty::Vec<rusty::detail::associated_item_t<I>>::new_()); }(); } if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& k = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m)).k); return [&]() { vals.prefill(k);
if (rusty::len(vals) != k) {
    rusty::detail::deref_if_pointer_like(state) = PermutationState_End{};
    return rusty::None;
}
rusty::detail::deref_if_pointer_like(state) = PermutationState_Buffered{.k = k, .min_n = k};
return rusty::Some(rusty::slice(vals, 0, k).to_vec()); }(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m)) { const auto& k = std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).k; auto&& min_n = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).min_n); return [&]() {
if (vals.get_next()) {
auto item = rusty::collect_range(rusty::map(rusty::chain((rusty::range(0, rusty::detail::deref_if_pointer_like(k) - 1)), rusty::once(rusty::detail::deref_if_pointer_like(min_n))), [&](auto&& i) { return rusty::clone(vals[i]); }));
[&]() { static_cast<void>(rusty::detail::deref_if_pointer_like(min_n) += 1); return std::make_tuple(); }();
return rusty::Some(std::move(item));
} else {
const auto n = rusty::detail::deref_if_pointer_like(min_n);
const auto prev_iteration_count = (n - rusty::detail::deref_if_pointer_like(k)) + 1;
auto indices = rusty::collect_range((rusty::range(0, n)));
auto cycles = rusty::collect_range((rusty::range(n - k, n)).rev());
return rusty::intrinsics::unreachable();
auto item = vals.get_at(rusty::slice(indices, 0, rusty::detail::deref_if_pointer_like(k)));
rusty::detail::deref_if_pointer_like(state) = PermutationState_Loaded{.indices = std::move(indices), .cycles = std::move(cycles)};
return rusty::Some(std::move(item));
}
}(); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m)) { auto&& indices = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).indices); auto&& cycles = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).cycles); return [&]() { if (advance(indices, cycles)) {
    rusty::detail::deref_if_pointer_like(state) = PermutationState_End{};
    return rusty::None;
}
auto k = rusty::len(cycles);
return rusty::Some(vals.get_at(rusty::slice(indices, 0, k))); }(); } if (std::holds_alternative<std::variant_alternative_t<3, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return rusty::None; } rusty::intrinsics::unreachable(); }();
        }
        size_t count() {
            auto&& _let_pat = (*this);
            auto&& vals = rusty::detail::deref_if_pointer(_let_pat.vals);
            auto&& state = rusty::detail::deref_if_pointer(_let_pat.state);
            auto n = vals.count();
            return state.size_hint_for(std::move(n))._1.unwrap();
        }
        size_hint::SizeHint size_hint() const {
            auto [low, upp] = rusty::detail::deref_if_pointer_like(this->vals.size_hint());
            low = std::get<0>(this->state.size_hint_for(std::move(low)));
            upp = upp.and_then([&](auto&& n) { return std::get<1>(this->state.size_hint_for(std::move(n))); });
            return std::make_tuple(std::move(low), std::move(upp));
        }
    };

}

namespace powerset {

    template<typename I>
    struct Powerset;
    template<typename I>
    Powerset<I> powerset(I src);
    rusty::Option<size_t> remaining_for(size_t n, size_t k);

    using ::rusty::Vec;

    namespace fmt = rusty::fmt;


    using combinations::combinations;
    using combinations::Combinations;

    using adaptors::checked_binomial;

    namespace size_hint = ::size_hint;
    using size_hint::SizeHint;

    /// An iterator to iterate through the powerset of the elements from an iterator.
    ///
    /// See [`.powerset()`](crate::Itertools::powerset) for more
    /// information.
    template<typename I>
    struct Powerset {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        combinations::Combinations<I> combs;

        Powerset<I> clone() const {
            return Powerset<I>{.combs = rusty::clone(this->combs)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("Powerset").field("combs", &this->combs).finish();
        }
        bool increment_k() {
            if ((this->combs.k() < this->combs.n()) || (this->combs.k() == 0)) {
                rusty::reset(this->combs, this->combs.k() + 1);
                return true;
            } else {
                return false;
            }
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            if (auto&& _iflet_scrutinee = this->combs.next(); _iflet_scrutinee.is_some()) {
                decltype(auto) elt = _iflet_scrutinee.unwrap();
                return rusty::Some(std::move(elt));
            } else if (this->increment_k()) {
                return this->combs.next();
            } else {
                return rusty::None;
            }
        }
        auto nth(size_t n) {
            while (true) {
                {
                    auto&& _m = this->combs.try_nth(std::move(n));
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_ok()) {
                            auto&& _mv0 = _m.unwrap();
                            auto&& item = rusty::detail::deref_if_pointer(_mv0);
                            return rusty::Some(item);
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (_m.is_err()) {
                            auto&& _mv1 = _m.unwrap_err();
                            auto&& steps = rusty::detail::deref_if_pointer(_mv1);
                            if (!this->increment_k()) {
                                return rusty::None;
                            }
                            [&]() { static_cast<void>(n -= steps); return std::make_tuple(); }();
                            _m_matched = true;
                        }
                    }
                }
            }
        }
        size_hint::SizeHint size_hint() const {
            auto k = this->combs.k();
            auto [n_min, n_max] = rusty::detail::deref_if_pointer_like(rusty::size_hint(this->combs.src()));
            auto low = remaining_for(std::move(n_min), std::move(k)).unwrap_or(std::numeric_limits<size_t>::max());
            auto upp = n_max.and_then([&](auto&& n) { return remaining_for(std::move(n), std::move(k)); });
            return size_hint::add(rusty::size_hint(this->combs), std::make_tuple(std::move(low), std::move(upp)));
        }
        size_t count() {
            auto k = this->combs.k();
            auto [n, combs_count] = rusty::detail::deref_if_pointer_like(this->combs.n_and_count());
            return combs_count + remaining_for(std::move(n), std::move(k)).unwrap();
        }
        template<typename B, typename F>
        B fold(B init, F f) {
            auto it = std::move(this->combs);
            if (it.k() == 0) {
                init = rusty::fold(it.by_ref(), std::move(init), &f);
                rusty::reset(it, 1);
            }
            init = rusty::fold(it.by_ref(), std::move(init), &f);
            for (auto&& k : rusty::for_in(rusty::range_inclusive(it.k() + 1, it.n()))) {
                rusty::reset(it, k);
                init = rusty::fold(it.by_ref(), std::move(init), &f);
            }
            return init;
        }
    };

}

namespace put_back_n_impl {

    template<typename I>
    struct PutBackN;
    template<typename I>
    PutBackN<typename I::IntoIter> put_back_n(I iterable);

    using ::rusty::Vec;

    namespace size_hint = ::size_hint;

    /// An iterator adaptor that allows putting multiple
    /// items in front of the iterator.
    ///
    /// Iterator element type is `I::Item`.
    template<typename I>
    struct PutBackN {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        rusty::Vec<rusty::detail::associated_item_t<I>> top;
        I iter;

        template<typename F>
        auto peeking_next(F accept) {
            using namespace peeking_take_while;
            if (auto&& _iflet_scrutinee = this->next(); _iflet_scrutinee.is_some()) {
                decltype(auto) r = _iflet_scrutinee.unwrap();
                if (!accept(r)) {
                    this->put_back(std::move(r));
                    return rusty::None;
                }
                return rusty::Some(std::move(r));
            } else {
                return rusty::None;
            }
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            using namespace peeking_take_while;
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field2_finish(f, "PutBackN", "top", &this->top, "iter", &this->iter);
        }
        PutBackN<I> clone() const {
            using namespace peeking_take_while;
            return PutBackN<I>{.top = rusty::clone(this->top), .iter = rusty::clone(this->iter)};
        }
        void put_back(rusty::detail::associated_item_t<I> x) {
            using namespace peeking_take_while;
            this->top.push(std::move(x));
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            using namespace peeking_take_while;
            return this->top.pop().or_else([&]() { return this->iter.next(); });
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            using namespace peeking_take_while;
            return size_hint::add_scalar(rusty::size_hint(this->iter), rusty::len(this->top));
        }
        template<typename B, typename F>
        B fold(B init, F f) {
            using namespace peeking_take_while;
            init = rusty::iter(std::move(this->top)).rfold(std::move(init), f);
            return rusty::fold(this->iter, std::move(init), std::move(f));
        }
    };

}

namespace sources {

    template<typename St, typename F>
    struct Unfold;
    template<typename St, typename F>
    struct Iterate;
    template<typename A, typename St, typename F>
    Unfold<St, F> unfold(St initial_state, F f);
    template<typename St, typename F>
    Iterate<St, F> iterate(St initial_value, F f);

    namespace fmt = rusty::fmt;

    namespace mem = rusty::mem;

    /// See [`unfold`](crate::unfold) for more information.
    template<typename St, typename F>
    struct Unfold {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        F f;
        /// Internal state that will be passed to the closure on the next iteration
        St state;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("Unfold").field("state", &this->state).finish();
        }
        Unfold<St, F> clone() const {
            return Unfold<St, F>{.f = rusty::clone(this->f), .state = rusty::clone(this->state)};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A>
        auto next() {
            return (this->f)(&this->state);
        }
    };

    /// An iterator that infinitely applies function to value and yields results.
    ///
    /// This `struct` is created by the [`iterate()`](crate::iterate) function.
    /// See its documentation for more.
    template<typename St, typename F>
    struct Iterate {
        using Item = St;
        St state;
        F f;

        Iterate<St, F> clone() const {
            return Iterate<St, F>{.state = rusty::clone(this->state), .f = rusty::clone(this->f)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("Iterate").field("state", &this->state).finish();
        }
        rusty::Option<Item> next() {
            const auto next_state = (this->f)(&this->state);
            return rusty::Option<St>(rusty::mem::replace(this->state, std::move(next_state)));
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return std::make_tuple(std::numeric_limits<size_t>::max(), rusty::Option<size_t>(rusty::None));
        }
    };

}

namespace take_while_inclusive {

    template<typename I, typename F>
    struct TakeWhileInclusive;


    namespace fmt = rusty::fmt;

    /// An iterator adaptor that consumes elements while the given predicate is
    /// `true`, including the element for which the predicate first returned
    /// `false`.
    ///
    /// See [`.take_while_inclusive()`](crate::Itertools::take_while_inclusive)
    /// for more information.
    template<typename I, typename F>
    struct TakeWhileInclusive {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        I iter;
        F predicate;
        bool done;

        TakeWhileInclusive<I, F> clone() const {
            return TakeWhileInclusive<I, F>{.iter = rusty::clone(this->iter), .predicate = rusty::clone(this->predicate), .done = rusty::clone(this->done)};
        }
        static TakeWhileInclusive<I, F> new_(I iter, F predicate) {
            return TakeWhileInclusive<I, F>{.iter = std::move(iter), .predicate = std::move(predicate), .done = false};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("TakeWhileInclusive").field("iter", &this->iter).field("done", &this->done).finish();
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            if (this->done) {
                return rusty::None;
            } else {
                return this->iter.next().map([&](auto&& item) -> rusty::detail::associated_item_t<I> {
if (!(this->predicate)(item)) {
    this->done = true;
}
return item;
});
            }
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            if (this->done) {
                return std::make_tuple(static_cast<size_t>(0), rusty::Option<size_t>(static_cast<size_t>(0)));
            } else {
                return std::make_tuple(static_cast<size_t>(0), rusty::size_hint(this->iter)._1);
            }
        }
        template<typename B, typename Fold>
        B fold(B init, Fold f) {
            if (this->done) {
                return init;
            } else {
                auto& predicate = this->predicate;
                return rusty::try_fold(this->iter, std::move(init), [&](auto&& acc, auto&& item) {
const auto is_ok = predicate(item);
acc = f(std::move(acc), std::move(item));
if (is_ok) {
    return rusty::Ok(std::move(acc));
} else {
    return rusty::Err(std::move(acc));
}
}).unwrap_or_else([&](auto&& err) { return err; });
            }
        }
    };

}

namespace tee {

    template<typename A, typename I>
    struct TeeBuffer;
    template<typename I>
    struct Tee;
    template<typename I>
    std::tuple<Tee<I>, Tee<I>> new_(I iter);

    namespace size_hint = ::size_hint;

    using ::rusty::VecDeque;

    using ::rusty::Rc;

    using ::rusty::RefCell;

    /// Common buffer object for the two tee halves
    template<typename A, typename I>
    struct TeeBuffer {
        rusty::VecDeque<A> backlog;
        I iter;
        /// The owner field indicates which id should read from the backlog
        bool owner;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, A>::debug_struct_field3_finish(f, "TeeBuffer", "backlog", &this->backlog, "iter", &this->iter, "owner", &this->owner);
        }
    };

    /// One half of an iterator pair where both return the same elements.
    ///
    /// See [`.tee()`](crate::Itertools::tee) for more information.
    template<typename I>
    struct Tee {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        rusty::Rc<rusty::RefCell<TeeBuffer<rusty::detail::associated_item_t<I>, I>>> rcbuffer;
        bool id;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field2_finish(f, "Tee", "rcbuffer", &this->rcbuffer, "id", &this->id);
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            auto& buffer = this->rcbuffer.borrow_mut();
            if (buffer.owner == this->id) {
                {
                    auto&& _m = buffer.backlog.pop_front();
                    bool _m_matched = false;
                    if (!_m_matched) {
                        if (_m.is_none()) {
                            _m_matched = true;
                        }
                    }
                    if (!_m_matched) {
                        if (true) {
                            const auto& some_elt = _m;
                            return some_elt;
                            _m_matched = true;
                        }
                    }
                }
            }
            return [&]() { auto&& _m = buffer.iter.next(); if (_m.is_none()) { return rusty::None; } if (_m.is_some()) { auto&& _mv1 = _m.unwrap(); auto&& elt = rusty::detail::deref_if_pointer(_mv1); return [&]() { buffer.backlog.push_back(rusty::clone(elt));
buffer.owner = !this->id;
return rusty::Some(std::move(elt)); }(); } rusty::intrinsics::unreachable(); }();
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto& buffer = this->rcbuffer.borrow();
            auto sh = rusty::size_hint(buffer.iter);
            if (buffer.owner == this->id) {
                auto log_len = rusty::len(buffer.backlog);
                return size_hint::add_scalar(std::move(sh), std::move(log_len));
            } else {
                return sh;
            }
        }
    };

}

namespace tuple_impl {

    template<typename T>
    struct TupleBuffer;
    template<typename I, typename T>
    struct Tuples;
    template<typename I, typename T>
    struct TupleWindows;
    template<typename I, typename T>
    struct CircularTupleWindows;
    template<typename I, typename T>
    Tuples<I, T> tuples(I iter);
    rusty::Option<size_t> add_then_div(size_t n, size_t a, size_t d);
    template<typename I, typename T>
    TupleWindows<I, T> tuple_windows(I iter);
    template<typename I, typename T>
    CircularTupleWindows<I, T> circular_tuple_windows(I iter);




    namespace size_hint = ::size_hint;


    /// An iterator over a incomplete tuple.
    ///
    /// See [`.tuples()`](crate::Itertools::tuples) and
    /// [`Tuples::into_buffer()`].
    template<typename T>
    struct TupleBuffer {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        size_t cur;
        typename T::Buffer buf;

        TupleBuffer<T> clone() const {
            return TupleBuffer<T>{.cur = rusty::clone(this->cur), .buf = rusty::clone(this->buf)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, T>::debug_struct_field2_finish(f, "TupleBuffer", "cur", &this->cur, "buf", &this->buf);
        }
        static TupleBuffer<T> new_(typename T::Buffer buf) {
            return TupleBuffer<T>{.cur = static_cast<size_t>(0), .buf = std::move(buf)};
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            auto& s = this->buf.as_mut();
            if (auto&& _iflet_scrutinee = rusty::get_mut(s, this->cur); _iflet_scrutinee.is_some()) {
                decltype(auto) item = _iflet_scrutinee.unwrap();
                [&]() { static_cast<void>(this->cur += 1); return std::make_tuple(); }();
                return item.take();
            } else {
                return rusty::None;
            }
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            const auto buffer = rusty::slice_from(this->buf.as_ref(), this->cur);
            auto len = (rusty::is_empty(buffer) ? 0 : rusty::iter(buffer).position([&](auto&& x) { return x.is_none(); }).unwrap_or(rusty::len(buffer)));
            return std::make_tuple(std::move(len), rusty::Option<size_t>(std::move(len)));
        }
    };

    /// An iterator that groups the items in tuples of a specific size.
    ///
    /// See [`.tuples()`](crate::Itertools::tuples) for more information.
    template<typename I, typename T>
    struct Tuples {
        using Item = T;
        decltype(std::declval<I>().fuse()) iter;
        typename T::Buffer buf;

        Tuples<I, T> clone() const {
            return Tuples<I, T>{.iter = rusty::clone(this->iter), .buf = rusty::clone(this->buf)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field2_finish(f, "Tuples", "iter", &this->iter, "buf", &this->buf);
        }
        rusty::Option<Item> next() {
            return this->iter.collect_from_iter(this->buf);
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto buffered = this->buf.buffer_len();
            auto [unbuffered_lo, unbuffered_hi] = rusty::detail::deref_if_pointer_like(rusty::size_hint(this->iter));
            auto total_lo = add_then_div(std::move(unbuffered_lo), std::move(buffered), T::num_items()).unwrap_or(std::numeric_limits<size_t>::max());
            auto total_hi = unbuffered_hi.and_then([&](auto&& hi) { return add_then_div(std::move(hi), std::move(buffered), T::num_items()); });
            return std::make_tuple(std::move(total_lo), std::move(total_hi));
        }
        TupleBuffer<T> into_buffer() {
            return TupleBuffer<T>::new_(std::move(this->buf));
        }
    };

    /// An iterator over all contiguous windows that produces tuples of a specific size.
    ///
    /// See [`.tuple_windows()`](crate::Itertools::tuple_windows) for more
    /// information.
    template<typename I, typename T>
    struct TupleWindows {
        using Item = T;
        I iter;
        rusty::Option<T> last;

        TupleWindows<I, T> clone() const {
            return TupleWindows<I, T>{.iter = rusty::clone(this->iter), .last = rusty::clone(this->last)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field2_finish(f, "TupleWindows", "iter", &this->iter, "last", &this->last);
        }
        rusty::Option<Item> next() {
            if (T::num_items() == 1) {
                return this->iter.collect_from_iter_no_buf();
            }
            if (auto&& _iflet_scrutinee = this->iter.next(); _iflet_scrutinee.is_some()) {
                decltype(auto) new_ = _iflet_scrutinee.unwrap();
                if (this->last.is_some()) {
                    decltype(auto) last = this->last.unwrap();
                    last.left_shift_push(std::move(new_));
                    return rusty::Option<T>(rusty::clone(last));
                } else {
                    auto iter = rusty::chain(once(std::move(new_)), &this->iter);
                    this->last = T::collect_from_iter_no_buf(std::move(iter));
                    return rusty::clone(this->last);
                }
            } else {
                return rusty::Option<T>(rusty::None);
            }
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto sh = rusty::size_hint(this->iter);
            if (this->last.is_none()) {
                sh = size_hint::sub_scalar(std::move(sh), T::num_items() - 1);
            }
            return sh;
        }
    };

    /// An iterator over all windows, wrapping back to the first elements when the
    /// window would otherwise exceed the length of the iterator, producing tuples
    /// of a specific size.
    ///
    /// See [`.circular_tuple_windows()`](crate::Itertools::circular_tuple_windows) for more
    /// information.
    template<typename I, typename T>
    struct CircularTupleWindows {
        using Item = T;
        TupleWindows<decltype(std::declval<I>().cycle()), T> iter;
        size_t len;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field2_finish(f, "CircularTupleWindows", "iter", &this->iter, "len", &this->len);
        }
        CircularTupleWindows<I, T> clone() const {
            return CircularTupleWindows<I, T>{.iter = rusty::clone(this->iter), .len = rusty::clone(this->len)};
        }
        rusty::Option<Item> next() {
            if (this->len != static_cast<size_t>(0)) {
                [&]() { static_cast<void>(this->len -= 1); return std::make_tuple(); }();
                return this->iter.next();
            } else {
                return rusty::Option<T>(rusty::None);
            }
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return std::make_tuple(this->len, rusty::Option<size_t>(this->len));
        }
    };

    // Rust-only trait TupleCollect (Proxy facade emission skipped in module mode)

}

namespace traits {



}

namespace unique_impl {

    template<typename I, typename V, typename F>
    struct UniqueBy;
    template<typename I>
    struct Unique;
    template<typename I, typename V, typename F>
    UniqueBy<I, V, F> unique_by(I iter, F f);
    template<typename I, typename K>
    size_t count_new_keys(rusty::HashMap<K, std::tuple<>> used, I iterable);
    template<typename I>
    Unique<I> unique(I iter);


    using ::rusty::HashMap;

    namespace fmt = rusty::fmt;



    /// An iterator adapter to filter out duplicate elements.
    ///
    /// See [`.unique_by()`](crate::Itertools::unique) for more information.
    template<typename I, typename V, typename F>
    struct UniqueBy {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        I iter;
        rusty::HashMap<V, std::tuple<>> used;
        F f;

        UniqueBy<I, V, F> clone() const {
            return UniqueBy<I, V, F>{.iter = rusty::clone(this->iter), .used = rusty::clone(this->used), .f = rusty::clone(this->f)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("UniqueBy").field("iter", &this->iter).field("used", &this->used).finish();
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            auto&& _let_pat = (*this);
            auto&& iter = rusty::detail::deref_if_pointer(_let_pat.iter);
            auto&& used = rusty::detail::deref_if_pointer(_let_pat.used);
            auto&& f = rusty::detail::deref_if_pointer(_let_pat.f);
            return iter.find([&](auto&& v) { return used.insert(f(std::move(v)), std::make_tuple()).is_none(); });
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto [low, hi] = rusty::detail::deref_if_pointer_like(rusty::size_hint(this->iter));
            return std::make_tuple(static_cast<size_t>(((low > 0) && rusty::is_empty(this->used))), std::move(hi));
        }
        size_t count() {
            auto key_f = std::move(this->f);
            return count_new_keys(std::move(this->used), this->iter.map([=, key_f = std::move(key_f)](auto&& elt) mutable { return key_f(elt); }));
        }
        auto next_back() {
            auto&& _let_pat = (*this);
            auto&& iter = rusty::detail::deref_if_pointer(_let_pat.iter);
            auto&& used = rusty::detail::deref_if_pointer(_let_pat.used);
            auto&& f = rusty::detail::deref_if_pointer(_let_pat.f);
            return iter.rfind([&](auto&& v) { return used.insert(f(std::move(v)), std::make_tuple()).is_none(); });
        }
    };

    /// An iterator adapter to filter out duplicate elements.
    ///
    /// See [`.unique()`](crate::Itertools::unique) for more information.
    template<typename I>
    struct Unique {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        UniqueBy<I, rusty::detail::associated_item_t<I>, std::tuple<>> iter;

        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            auto&& _let_pat = &this->iter;
            auto&& iter = rusty::detail::deref_if_pointer(_let_pat.iter);
            auto&& used = rusty::detail::deref_if_pointer(_let_pat.used);
            return iter.find_map([&](auto&& v) {
if (auto&& _iflet_scrutinee = rusty::detail::make_entry_probe(used, std::move(v)); rusty::detail::deref_if_pointer(_iflet_scrutinee).is_vacant()) {
    auto&& entry = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_iflet_scrutinee)).vacant_entry());
    auto elt = rusty::clone(entry.key());
    entry.insert(std::make_tuple());
    return rusty::Some(std::move(elt));
}
return rusty::None;
});
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto [low, hi] = rusty::detail::deref_if_pointer_like(rusty::size_hint(this->iter.iter));
            return std::make_tuple(static_cast<size_t>(((low > 0) && rusty::is_empty(this->iter.used))), std::move(hi));
        }
        size_t count() {
            return count_new_keys(std::move(this->iter.used), std::move(this->iter.iter));
        }
        auto next_back() {
            auto&& _let_pat = &this->iter;
            auto&& iter = rusty::detail::deref_if_pointer(_let_pat.iter);
            auto&& used = rusty::detail::deref_if_pointer(_let_pat.used);
            return iter.rev().find_map([&](auto&& v) {
if (auto&& _iflet_scrutinee = rusty::detail::make_entry_probe(used, std::move(v)); rusty::detail::deref_if_pointer(_iflet_scrutinee).is_vacant()) {
    auto&& entry = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_iflet_scrutinee)).vacant_entry());
    auto elt = rusty::clone(entry.key());
    entry.insert(std::make_tuple());
    return rusty::Some(std::move(elt));
}
return rusty::None;
});
        }
        Unique<I> clone() const {
            return Unique<I>{.iter = rusty::clone(this->iter)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("Unique").field("iter", &this->iter).finish();
        }
    };

}

namespace unziptuple {

    template<typename I>
    auto multiunzip(I i);


    /// Converts an iterator of tuples into a tuple of containers.
    ///
    /// `multiunzip()` consumes an entire iterator of n-ary tuples, producing `n` collections, one for each
    /// column.
    ///
    /// This function is, in some sense, the opposite of [`multizip`].
    ///
    /// ```
    /// use itertools::multiunzip;
    ///
    /// let inputs = vec![(1, 2, 3), (4, 5, 6), (7, 8, 9)];
    ///
    /// let (a, b, c): (Vec<_>, Vec<_>, Vec<_>) = multiunzip(inputs);
    ///
    /// assert_eq!(a, vec![1, 4, 7]);
    /// assert_eq!(b, vec![2, 5, 8]);
    /// assert_eq!(c, vec![3, 6, 9]);
    /// ```
    ///
    /// [`multizip`]: crate::multizip
    template<typename I>
    auto multiunzip(I i) {
        return ([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::unziptuple::rusty_ext::multiunzip(std::forward<decltype(__self)>(__self)); }) { return ::unziptuple::rusty_ext::multiunzip(std::forward<decltype(__self)>(__self)); } else { return ::unziptuple::rusty_ext::multiunzip(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self))); } })(rusty::iter(std::move(i)));
    }

    // Extension trait MultiUnzip lowered to rusty_ext:: free functions
    namespace rusty_ext {
        template<typename IT>
        std::tuple<> multiunzip(IT self_) {
            using Self = std::remove_reference_t<decltype(self_)>;
            auto res = std::make_tuple();
            static_cast<void>(&res);
            rusty::fold(self_, std::make_tuple(), [&](auto&& _destruct_param0, auto&& _destruct_param1) {
static_cast<void>(_destruct_param0);
static_cast<void>(_destruct_param1);
});
            return res;
        }

        template<typename IT, typename FromA>
        std::tuple<FromA> multiunzip(IT self_) {
            using Self = std::remove_reference_t<decltype(self_)>;
            auto res = std::make_tuple(FromA::default_());
            auto [FromA_shadow1] = rusty::detail::deref_if_pointer_like(&res);
            rusty::fold(self_, std::make_tuple(), [&](auto&& _destruct_param0, auto&& _destruct_param1) {
static_cast<void>(_destruct_param0);
auto&& [A] = _destruct_param1;
FromA_shadow1.extend(rusty::once(A));
});
            return res;
        }

        template<typename IT, typename FromA, typename FromB>
        std::tuple<FromA, FromB> multiunzip(IT self_) {
            using Self = std::remove_reference_t<decltype(self_)>;
            auto res = std::make_tuple(FromA::default_(), FromB::default_());
            auto [FromA_shadow1, FromB_shadow1] = rusty::detail::deref_if_pointer_like(&res);
            rusty::fold(self_, std::make_tuple(), [&](auto&& _destruct_param0, auto&& _destruct_param1) {
static_cast<void>(_destruct_param0);
auto&& [A, B] = _destruct_param1;
FromA_shadow1.extend(rusty::once(A));
FromB_shadow1.extend(rusty::once(B));
});
            return res;
        }

        template<typename IT, typename FromA, typename FromB, typename FromC>
        std::tuple<FromA, FromB, FromC> multiunzip(IT self_) {
            using Self = std::remove_reference_t<decltype(self_)>;
            auto res = std::make_tuple(FromA::default_(), FromB::default_(), FromC::default_());
            auto [FromA_shadow1, FromB_shadow1, FromC_shadow1] = rusty::detail::deref_if_pointer_like(&res);
            rusty::fold(self_, std::make_tuple(), [&](auto&& _destruct_param0, auto&& _destruct_param1) {
static_cast<void>(_destruct_param0);
auto&& [A, B, C] = _destruct_param1;
FromA_shadow1.extend(rusty::once(A));
FromB_shadow1.extend(rusty::once(B));
FromC_shadow1.extend(rusty::once(C));
});
            return res;
        }

        template<typename IT, typename FromA, typename FromB, typename FromC, typename FromD>
        std::tuple<FromA, FromB, FromC, FromD> multiunzip(IT self_) {
            using Self = std::remove_reference_t<decltype(self_)>;
            auto res = std::make_tuple(FromA::default_(), FromB::default_(), FromC::default_(), FromD::default_());
            auto [FromA_shadow1, FromB_shadow1, FromC_shadow1, FromD_shadow1] = rusty::detail::deref_if_pointer_like(&res);
            rusty::fold(self_, std::make_tuple(), [&](auto&& _destruct_param0, auto&& _destruct_param1) {
static_cast<void>(_destruct_param0);
auto&& [A, B, C, D] = _destruct_param1;
FromA_shadow1.extend(rusty::once(A));
FromB_shadow1.extend(rusty::once(B));
FromC_shadow1.extend(rusty::once(C));
FromD_shadow1.extend(rusty::once(D));
});
            return res;
        }

        template<typename IT, typename FromA, typename FromB, typename FromC, typename FromD, typename FromE>
        std::tuple<FromA, FromB, FromC, FromD, FromE> multiunzip(IT self_) {
            using Self = std::remove_reference_t<decltype(self_)>;
            auto res = std::make_tuple(FromA::default_(), FromB::default_(), FromC::default_(), FromD::default_(), FromE::default_());
            auto [FromA_shadow1, FromB_shadow1, FromC_shadow1, FromD_shadow1, FromE_shadow1] = rusty::detail::deref_if_pointer_like(&res);
            rusty::fold(self_, std::make_tuple(), [&](auto&& _destruct_param0, auto&& _destruct_param1) {
static_cast<void>(_destruct_param0);
auto&& [A, B, C, D, E] = _destruct_param1;
FromA_shadow1.extend(rusty::once(A));
FromB_shadow1.extend(rusty::once(B));
FromC_shadow1.extend(rusty::once(C));
FromD_shadow1.extend(rusty::once(D));
FromE_shadow1.extend(rusty::once(E));
});
            return res;
        }

        template<typename IT, typename FromA, typename FromB, typename FromC, typename FromD, typename FromE, typename FromF>
        std::tuple<FromA, FromB, FromC, FromD, FromE, FromF> multiunzip(IT self_) {
            using Self = std::remove_reference_t<decltype(self_)>;
            auto res = std::make_tuple(FromA::default_(), FromB::default_(), FromC::default_(), FromD::default_(), FromE::default_(), FromF::default_());
            auto [FromA_shadow1, FromB_shadow1, FromC_shadow1, FromD_shadow1, FromE_shadow1, FromF_shadow1] = rusty::detail::deref_if_pointer_like(&res);
            rusty::fold(self_, std::make_tuple(), [&](auto&& _destruct_param0, auto&& _destruct_param1) {
static_cast<void>(_destruct_param0);
auto&& [A, B, C, D, E, F] = _destruct_param1;
FromA_shadow1.extend(rusty::once(A));
FromB_shadow1.extend(rusty::once(B));
FromC_shadow1.extend(rusty::once(C));
FromD_shadow1.extend(rusty::once(D));
FromE_shadow1.extend(rusty::once(E));
FromF_shadow1.extend(rusty::once(F));
});
            return res;
        }

        template<typename IT, typename FromA, typename FromB, typename FromC, typename FromD, typename FromE, typename FromF, typename FromG>
        std::tuple<FromA, FromB, FromC, FromD, FromE, FromF, FromG> multiunzip(IT self_) {
            using Self = std::remove_reference_t<decltype(self_)>;
            auto res = std::make_tuple(FromA::default_(), FromB::default_(), FromC::default_(), FromD::default_(), FromE::default_(), FromF::default_(), FromG::default_());
            auto [FromA_shadow1, FromB_shadow1, FromC_shadow1, FromD_shadow1, FromE_shadow1, FromF_shadow1, FromG_shadow1] = rusty::detail::deref_if_pointer_like(&res);
            rusty::fold(self_, std::make_tuple(), [&](auto&& _destruct_param0, auto&& _destruct_param1) {
static_cast<void>(_destruct_param0);
auto&& [A, B, C, D, E, F, G] = _destruct_param1;
FromA_shadow1.extend(rusty::once(A));
FromB_shadow1.extend(rusty::once(B));
FromC_shadow1.extend(rusty::once(C));
FromD_shadow1.extend(rusty::once(D));
FromE_shadow1.extend(rusty::once(E));
FromF_shadow1.extend(rusty::once(F));
FromG_shadow1.extend(rusty::once(G));
});
            return res;
        }

        template<typename IT, typename FromA, typename FromB, typename FromC, typename FromD, typename FromE, typename FromF, typename FromG, typename FromH>
        std::tuple<FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH> multiunzip(IT self_) {
            using Self = std::remove_reference_t<decltype(self_)>;
            auto res = std::make_tuple(FromA::default_(), FromB::default_(), FromC::default_(), FromD::default_(), FromE::default_(), FromF::default_(), FromG::default_(), FromH::default_());
            auto [FromA_shadow1, FromB_shadow1, FromC_shadow1, FromD_shadow1, FromE_shadow1, FromF_shadow1, FromG_shadow1, FromH_shadow1] = rusty::detail::deref_if_pointer_like(&res);
            rusty::fold(self_, std::make_tuple(), [&](auto&& _destruct_param0, auto&& _destruct_param1) {
static_cast<void>(_destruct_param0);
auto&& [A, B, C, D, E, F, G, H] = _destruct_param1;
FromA_shadow1.extend(rusty::once(A));
FromB_shadow1.extend(rusty::once(B));
FromC_shadow1.extend(rusty::once(C));
FromD_shadow1.extend(rusty::once(D));
FromE_shadow1.extend(rusty::once(E));
FromF_shadow1.extend(rusty::once(F));
FromG_shadow1.extend(rusty::once(G));
FromH_shadow1.extend(rusty::once(H));
});
            return res;
        }

        template<typename IT, typename FromA, typename FromB, typename FromC, typename FromD, typename FromE, typename FromF, typename FromG, typename FromH, typename FromI>
        std::tuple<FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH, FromI> multiunzip(IT self_) {
            using Self = std::remove_reference_t<decltype(self_)>;
            auto res = std::make_tuple(FromA::default_(), FromB::default_(), FromC::default_(), FromD::default_(), FromE::default_(), FromF::default_(), FromG::default_(), FromH::default_(), FromI::default_());
            auto [FromA_shadow1, FromB_shadow1, FromC_shadow1, FromD_shadow1, FromE_shadow1, FromF_shadow1, FromG_shadow1, FromH_shadow1, FromI_shadow1] = rusty::detail::deref_if_pointer_like(&res);
            rusty::fold(self_, std::make_tuple(), [&](auto&& _destruct_param0, auto&& _destruct_param1) {
static_cast<void>(_destruct_param0);
auto&& [A, B, C, D, E, F, G, H, I] = _destruct_param1;
FromA_shadow1.extend(rusty::once(A));
FromB_shadow1.extend(rusty::once(B));
FromC_shadow1.extend(rusty::once(C));
FromD_shadow1.extend(rusty::once(D));
FromE_shadow1.extend(rusty::once(E));
FromF_shadow1.extend(rusty::once(F));
FromG_shadow1.extend(rusty::once(G));
FromH_shadow1.extend(rusty::once(H));
FromI_shadow1.extend(rusty::once(I));
});
            return res;
        }

        template<typename IT, typename FromA, typename FromB, typename FromC, typename FromD, typename FromE, typename FromF, typename FromG, typename FromH, typename FromI, typename FromJ>
        std::tuple<FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH, FromI, FromJ> multiunzip(IT self_) {
            using Self = std::remove_reference_t<decltype(self_)>;
            auto res = std::make_tuple(FromA::default_(), FromB::default_(), FromC::default_(), FromD::default_(), FromE::default_(), FromF::default_(), FromG::default_(), FromH::default_(), FromI::default_(), FromJ::default_());
            auto [FromA_shadow1, FromB_shadow1, FromC_shadow1, FromD_shadow1, FromE_shadow1, FromF_shadow1, FromG_shadow1, FromH_shadow1, FromI_shadow1, FromJ_shadow1] = rusty::detail::deref_if_pointer_like(&res);
            rusty::fold(self_, std::make_tuple(), [&](auto&& _destruct_param0, auto&& _destruct_param1) {
static_cast<void>(_destruct_param0);
auto&& [A, B, C, D, E, F, G, H, I, J] = _destruct_param1;
FromA_shadow1.extend(rusty::once(A));
FromB_shadow1.extend(rusty::once(B));
FromC_shadow1.extend(rusty::once(C));
FromD_shadow1.extend(rusty::once(D));
FromE_shadow1.extend(rusty::once(E));
FromF_shadow1.extend(rusty::once(F));
FromG_shadow1.extend(rusty::once(G));
FromH_shadow1.extend(rusty::once(H));
FromI_shadow1.extend(rusty::once(I));
FromJ_shadow1.extend(rusty::once(J));
});
            return res;
        }

        template<typename IT, typename FromA, typename FromB, typename FromC, typename FromD, typename FromE, typename FromF, typename FromG, typename FromH, typename FromI, typename FromJ, typename FromK>
        std::tuple<FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH, FromI, FromJ, FromK> multiunzip(IT self_) {
            using Self = std::remove_reference_t<decltype(self_)>;
            auto res = std::make_tuple(FromA::default_(), FromB::default_(), FromC::default_(), FromD::default_(), FromE::default_(), FromF::default_(), FromG::default_(), FromH::default_(), FromI::default_(), FromJ::default_(), FromK::default_());
            auto [FromA_shadow1, FromB_shadow1, FromC_shadow1, FromD_shadow1, FromE_shadow1, FromF_shadow1, FromG_shadow1, FromH_shadow1, FromI_shadow1, FromJ_shadow1, FromK_shadow1] = rusty::detail::deref_if_pointer_like(&res);
            rusty::fold(self_, std::make_tuple(), [&](auto&& _destruct_param0, auto&& _destruct_param1) {
static_cast<void>(_destruct_param0);
auto&& [A, B, C, D, E, F, G, H, I, J, K] = _destruct_param1;
FromA_shadow1.extend(rusty::once(A));
FromB_shadow1.extend(rusty::once(B));
FromC_shadow1.extend(rusty::once(C));
FromD_shadow1.extend(rusty::once(D));
FromE_shadow1.extend(rusty::once(E));
FromF_shadow1.extend(rusty::once(F));
FromG_shadow1.extend(rusty::once(G));
FromH_shadow1.extend(rusty::once(H));
FromI_shadow1.extend(rusty::once(I));
FromJ_shadow1.extend(rusty::once(J));
FromK_shadow1.extend(rusty::once(K));
});
            return res;
        }

        template<typename IT, typename FromA, typename FromB, typename FromC, typename FromD, typename FromE, typename FromF, typename FromG, typename FromH, typename FromI, typename FromJ, typename FromK, typename FromL>
        std::tuple<FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH, FromI, FromJ, FromK, FromL> multiunzip(IT self_) {
            using Self = std::remove_reference_t<decltype(self_)>;
            auto res = std::make_tuple(FromA::default_(), FromB::default_(), FromC::default_(), FromD::default_(), FromE::default_(), FromF::default_(), FromG::default_(), FromH::default_(), FromI::default_(), FromJ::default_(), FromK::default_(), FromL::default_());
            auto [FromA_shadow1, FromB_shadow1, FromC_shadow1, FromD_shadow1, FromE_shadow1, FromF_shadow1, FromG_shadow1, FromH_shadow1, FromI_shadow1, FromJ_shadow1, FromK_shadow1, FromL_shadow1] = rusty::detail::deref_if_pointer_like(&res);
            rusty::fold(self_, std::make_tuple(), [&](auto&& _destruct_param0, auto&& _destruct_param1) {
static_cast<void>(_destruct_param0);
auto&& [A, B, C, D, E, F, G, H, I, J, K, L] = _destruct_param1;
FromA_shadow1.extend(rusty::once(A));
FromB_shadow1.extend(rusty::once(B));
FromC_shadow1.extend(rusty::once(C));
FromD_shadow1.extend(rusty::once(D));
FromE_shadow1.extend(rusty::once(E));
FromF_shadow1.extend(rusty::once(F));
FromG_shadow1.extend(rusty::once(G));
FromH_shadow1.extend(rusty::once(H));
FromI_shadow1.extend(rusty::once(I));
FromJ_shadow1.extend(rusty::once(J));
FromK_shadow1.extend(rusty::once(K));
FromL_shadow1.extend(rusty::once(L));
});
            return res;
        }

    }


}

namespace with_position {

    enum class Position;
    constexpr Position Position_First();
    constexpr Position Position_Middle();
    constexpr Position Position_Last();
    constexpr Position Position_Only();
    Position clone(const Position& self_);
    bool eq(const Position& self_, const Position& other);
    void assert_receiver_is_total_eq(const Position& self_);
    template<typename I>
    struct WithPosition;
    template<typename I>
    WithPosition<I> with_position(I iter);

    enum class Position {
        First,
    Middle,
    Last,
    Only
    };
    inline constexpr Position Position_First() { return Position::First; }
    inline constexpr Position Position_Middle() { return Position::Middle; }
    inline constexpr Position Position_Last() { return Position::Last; }
    inline constexpr Position Position_Only() { return Position::Only; }
    inline Position clone(const Position& self_) {
        return self_;
    }
    inline bool eq(const Position& self_, const Position& other) {
        const auto __self_discr = rusty::intrinsics::discriminant_value(self_);
        const auto __arg1_discr = rusty::intrinsics::discriminant_value(other);
        return __self_discr == __arg1_discr;
    }
    inline void assert_receiver_is_total_eq(const Position& self_) {
    }

    namespace fmt = rusty::fmt;


    /// An iterator adaptor that wraps each element in an [`Position`].
    ///
    /// Iterator element type is `(Position, I::Item)`.
    ///
    /// See [`.with_position()`](crate::Itertools::with_position) for more information.
    template<typename I>
    struct WithPosition {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        bool handled_first;
        decltype(std::declval<decltype(std::declval<I>().fuse())>().peekable()) peekable;

        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return f.debug_struct("WithPosition").field("handled_first", &this->handled_first).field("peekable", &this->peekable).finish();
        }
        WithPosition<I> clone() const {
            return WithPosition<I>{.handled_first = rusty::clone(this->handled_first), .peekable = rusty::clone(this->peekable)};
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            return [&]() { auto&& _m = this->peekable.next(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& item = rusty::detail::deref_if_pointer(_mv0); return [&]() {
if (!this->handled_first) {
this->handled_first = true;
return [&]() { auto&& _m = this->peekable.peek(); if (_m.is_some()) { return rusty::Some(std::make_tuple(with_position::Position_First(), std::move(item))); } if (_m.is_none()) { return rusty::Some(std::make_tuple(with_position::Position_Only(), std::move(item))); } rusty::intrinsics::unreachable(); }();
} else {
return [&]() { auto&& _m = this->peekable.peek(); if (_m.is_some()) { return rusty::Some(std::make_tuple(with_position::Position_Middle(), std::move(item))); } if (_m.is_none()) { return rusty::Some(std::make_tuple(with_position::Position_Last(), std::move(item))); } rusty::intrinsics::unreachable(); }();
}
}(); } if (_m.is_none()) { return rusty::None; } rusty::intrinsics::unreachable(); }();
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return rusty::size_hint(this->peekable);
        }
        template<typename B, typename F>
        B fold(B init, F f) {
            if (auto&& _iflet_scrutinee = this->peekable.next(); _iflet_scrutinee.is_some()) {
                decltype(auto) head = _iflet_scrutinee.unwrap();
                if (!this->handled_first) {
                    {
                        auto&& _m = this->peekable.next();
                        bool _m_matched = false;
                        if (!_m_matched) {
                            if (_m.is_some()) {
                                auto&& _mv0 = _m.unwrap();
                                auto&& second = rusty::detail::deref_if_pointer(_mv0);
                                auto first = rusty::mem::replace(head, second);
                                init = f(std::move(init), std::make_tuple(with_position::Position_First(), std::move(first)));
                                _m_matched = true;
                            }
                        }
                        if (!_m_matched) {
                            if (_m.is_none()) {
                                return f(std::move(init), std::make_tuple(with_position::Position_Only(), std::move(head)));
                                _m_matched = true;
                            }
                        }
                    }
                }
                init = rusty::fold(this->peekable, std::move(init), [&](auto&& acc, auto&& item) {
rusty::mem::swap(head, item);
return f(std::move(acc), std::make_tuple(with_position::Position_Middle(), std::move(item)));
});
                init = f(std::move(init), std::make_tuple(with_position::Position_Last(), std::move(head)));
            }
            return init;
        }
    };

}

namespace zip_eq_impl {

    template<typename I, typename J>
    struct ZipEq;
    template<typename I, typename J>
    ZipEq<typename I::IntoIter, typename J::IntoIter> zip_eq(I i, J j);

    namespace size_hint = ::size_hint;

    /// An iterator which iterates two other iterators simultaneously
    /// and panic if they have different lengths.
    ///
    /// See [`.zip_eq()`](crate::Itertools::zip_eq) for more information.
    template<typename I, typename J>
    struct ZipEq {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        I a;
        J b;

        ZipEq<I, J> clone() const {
            return ZipEq<I, J>{.a = rusty::clone(this->a), .b = rusty::clone(this->b)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, I>::debug_struct_field2_finish(f, "ZipEq", "a", &this->a, "b", &this->b);
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            {
                auto&& _m0 = this->a.next();
                auto&& _m1 = this->b.next();
                auto _m_tuple = std::forward_as_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched && ((std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)).is_none() && std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)).is_none()))) {
                    rusty::None;
                    _m_matched = true;
                }
                if (!_m_matched && ((rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple))).is_some() && rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple))).is_some()))) {
                    auto&& a = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)))).unwrap());
                    auto&& b = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)))).unwrap());
                    rusty::Some(std::make_tuple(a, b));
                    _m_matched = true;
                }
                if (!_m_matched && (((std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)).is_none() && rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple))).is_some()) || (rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple))).is_some() && std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)).is_none())))) {
                    rusty::panic::begin_panic("itertools: .zip_eq() reached end of one iterator before the other");
                    _m_matched = true;
                }
            }
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return size_hint::min(rusty::size_hint(this->a), rusty::size_hint(this->b));
        }
    };

}

namespace zip_longest {

    template<typename T, typename U>
    struct ZipLongest;
    using either_or_both::EitherOrBoth;
    template<typename T, typename U>
    ZipLongest<T, U> zip_longest(T a, U b);

    namespace size_hint = ::size_hint;



    using either_or_both::EitherOrBoth;
    using namespace either_or_both;

    /// An iterator which iterates two other iterators simultaneously
    /// and wraps the elements in [`EitherOrBoth`].
    ///
    /// This iterator is *fused*.
    ///
    /// See [`.zip_longest()`](crate::Itertools::zip_longest) for more information.
    template<typename T, typename U>
    struct ZipLongest {
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        decltype(std::declval<T>().fuse()) a;
        decltype(std::declval<U>().fuse()) b;

        ZipLongest<T, U> clone() const {
            return ZipLongest<T, U>{.a = rusty::clone(this->a), .b = rusty::clone(this->b)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, T>::debug_struct_field2_finish(f, "ZipLongest", "a", &this->a, "b", &this->b);
        }
        // Rust-only dependent associated type alias skipped in constrained mode: Item
        auto next() {
            return [&]() { auto&& _m0 = this->a.next(); auto&& _m1 = this->b.next(); if (_m0.is_none() && _m1.is_none()) { return rusty::None; } if (rusty::detail::deref_if_pointer(_m0).is_some() && _m1.is_none()) { auto&& a = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); return rusty::Option<either_or_both::EitherOrBoth<T, T>>(Left<T, T>(T(a))); } if (_m0.is_none() && rusty::detail::deref_if_pointer(_m1).is_some()) { auto&& b = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); return rusty::Option<either_or_both::EitherOrBoth<T, T>>(Right<T, T>(T(b))); } if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some()) { auto&& a = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& b = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); return rusty::Option<either_or_both::EitherOrBoth<T, T>>(either_or_both::EitherOrBoth_Both<T, T>{a, b}); } rusty::intrinsics::unreachable(); }();
        }
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            return size_hint::max(rusty::size_hint(this->a), rusty::size_hint(this->b));
        }
        template<typename B, typename F>
        B fold(B init, F f) {
            auto&& _let_pat = (*this);
            auto a = rusty::detail::deref_if_pointer(_let_pat.a);
            auto b = rusty::detail::deref_if_pointer(_let_pat.b);
            const auto res = rusty::try_fold(a, std::move(init), [&](auto&& init, auto&& a) { return [&]() { auto&& _m = b.next(); if (_m.is_some()) { auto&& _mv0 = _m.unwrap(); auto&& b = rusty::detail::deref_if_pointer(_mv0); return rusty::Ok(f(std::move(init), either_or_both::EitherOrBoth_Both<T, T>{std::move(a), std::move(b)})); } if (_m.is_none()) { return rusty::Err(f(std::move(init), Left<B, B>(B(std::move(a))))); } rusty::intrinsics::unreachable(); }(); });
            return [&]() -> B { auto&& _m = res; if (_m.is_ok()) { auto&& _mv0 = _m.unwrap(); auto&& acc = rusty::detail::deref_if_pointer(_mv0); return rusty::fold(b.map([](auto&& _v) { return either_or_both::EitherOrBoth_Right<T, T>{std::forward<decltype(_v)>(_v)}; }), std::move(acc), std::move(f)); } if (_m.is_err()) { auto&& _mv1 = _m.unwrap_err(); auto&& acc = rusty::detail::deref_if_pointer(_mv1); return rusty::fold(a.map([](auto&& _v) { return either_or_both::EitherOrBoth_Left<T, T>{std::forward<decltype(_v)>(_v)}; }), std::move(acc), std::move(f)); } return [&]() -> B { rusty::intrinsics::unreachable(); }(); }();
        }
        auto next_back() {
            return [&]() { auto&& _m = rusty::cmp::cmp(rusty::len(this->a), rusty::len(this->b)); { const auto& Equal = _m; return [&]() { auto&& _m0 = this->a.next_back(); auto&& _m1 = this->b.next_back(); if (_m0.is_none() && _m1.is_none()) { return rusty::None; } if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some()) { auto&& a = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& b = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); return rusty::Option<either_or_both::EitherOrBoth<T, T>>(either_or_both::EitherOrBoth_Both<T, T>{a, b}); } if (rusty::detail::deref_if_pointer(_m0).is_some() && _m1.is_none()) { auto&& a = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); return rusty::Option<either_or_both::EitherOrBoth<T, T>>(Left<T, T>(T(a))); } if (_m0.is_none() && rusty::detail::deref_if_pointer(_m1).is_some()) { auto&& b = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); return rusty::Option<either_or_both::EitherOrBoth<T, T>>(Right<T, T>(T(b))); } rusty::intrinsics::unreachable(); }();  }
{ const auto& Greater = _m; return this->a.next_back().map([](auto&& _v) { return either_or_both::EitherOrBoth_Left<T, T>{std::forward<decltype(_v)>(_v)}; });  }
{ const auto& Less = _m; return this->b.next_back().map([](auto&& _v) { return either_or_both::EitherOrBoth_Right<T, T>{std::forward<decltype(_v)>(_v)}; });  } }();
        }
        template<typename B, typename F>
        B rfold(B init, F f) {
            auto&& _let_pat = (*this);
            auto a = rusty::detail::deref_if_pointer(_let_pat.a);
            auto b = rusty::detail::deref_if_pointer(_let_pat.b);
            const auto a_len = rusty::len(a);
            const auto b_len = rusty::len(b);
            {
                auto&& _m = rusty::cmp::cmp(a_len, b_len);
                bool _m_matched = false;
                if (!_m_matched) {
                    if (true) {
                        const auto& Equal = _m;
                        _m_matched = true;
                    }
                }
                if (!_m_matched) {
                    if (true) {
                        const auto& Greater = _m;
                        init = rusty::fold(rusty::map(a.by_ref().rev().take(a_len - b_len), [](auto&& _v) { return Left<B, B>(std::forward<decltype(_v)>(_v)); }), std::move(init), &f);
                        _m_matched = true;
                    }
                }
                if (!_m_matched) {
                    if (true) {
                        const auto& Less = _m;
                        init = rusty::fold(rusty::map(b.by_ref().rev().take(b_len - a_len), [](auto&& _v) { return Right<B, B>(std::forward<decltype(_v)>(_v)); }), std::move(init), &f);
                        _m_matched = true;
                    }
                }
            }
            return a.rfold(std::move(init), [&](auto&& acc, auto&& item_a) {
return f(std::move(acc), either_or_both::EitherOrBoth_Both<T, T>{std::move(item_a), b.next_back().unwrap()});
});
        }
    };

}

namespace ziptuple {

    template<typename T>
    struct Zip;
    template<typename T, typename U>
    Zip<T> multizip(U t);

    namespace size_hint = ::size_hint;

    /// See [`multizip`] for more information.
    template<typename T>
    struct Zip {
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        T t;

        Zip<T> clone() const {
            return Zip<T>{.t = rusty::clone(this->t)};
        }
        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return std::conditional_t<true, rusty::fmt::Formatter, T>::debug_struct_field1_finish(f, "Zip", "t", &this->t);
        }
        template<typename A>
        static Zip<T> from(std::tuple<A> t) {
            auto [A_shadow1] = rusty::detail::deref_if_pointer_like(t);
            return Zip<T>{.t = std::make_tuple(rusty::iter(A_shadow1))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A>
        auto next() {
            auto [A_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto A_shadow2 = ({ auto&& _m = A_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            return rusty::Some(std::make_tuple(A_shadow2));
        }
        template<typename A>
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto sh = std::make_tuple(std::numeric_limits<size_t>::max(), rusty::None);
            auto [A_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto sh_shadow1 = size_hint::min(rusty::size_hint(A_shadow1), std::move(sh));
            return sh_shadow1;
        }
        template<typename A>
        auto next_back() {
            auto [A_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            const auto size = rusty::detail::deref_if_pointer_like(rusty::iter(std::array{rusty::len(A_shadow1)}).min().unwrap());
            if (rusty::len(A_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(A_shadow1) - size))) {
                    A_shadow1.next_back();
                }
            }
            return [&]() { auto&& _m0 = A_shadow1.next_back(); if (rusty::detail::deref_if_pointer(_m0).is_some()) { auto&& A_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); return rusty::Some(std::make_tuple(A_shadow1)); } if (true) { return rusty::None; } rusty::intrinsics::unreachable(); }();
        }
        template<typename A, typename B>
        static Zip<T> from(std::tuple<A, B> t) {
            auto [A_shadow1, B_shadow1] = rusty::detail::deref_if_pointer_like(t);
            return Zip<T>{.t = std::make_tuple(rusty::iter(A_shadow1), rusty::iter(B_shadow1))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A, typename B>
        auto next() {
            auto [A_shadow1, B_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto A_shadow2 = ({ auto&& _m = A_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto B_shadow2 = ({ auto&& _m = B_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            return rusty::Some(std::make_tuple(A_shadow2, B_shadow2));
        }
        template<typename A, typename B>
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto sh = std::make_tuple(std::numeric_limits<size_t>::max(), rusty::None);
            auto [A_shadow1, B_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto sh_shadow1 = size_hint::min(rusty::size_hint(A_shadow1), std::move(sh));
            auto sh_shadow2 = size_hint::min(rusty::size_hint(B_shadow1), std::move(sh_shadow1));
            return sh_shadow2;
        }
        template<typename A, typename B>
        auto next_back() {
            auto [A_shadow1, B_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            const auto size = rusty::detail::deref_if_pointer_like(rusty::iter(std::array{rusty::len(A_shadow1), rusty::len(B_shadow1)}).min().unwrap());
            if (rusty::len(A_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(A_shadow1) - size))) {
                    A_shadow1.next_back();
                }
            }
            if (rusty::len(B_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(B_shadow1) - size))) {
                    B_shadow1.next_back();
                }
            }
            return [&]() { auto&& _m0 = A_shadow1.next_back(); auto&& _m1 = B_shadow1.next_back(); if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some()) { auto&& A_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& B_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); return rusty::Some(std::make_tuple(A_shadow1, B_shadow1)); } if (true) { return rusty::None; } rusty::intrinsics::unreachable(); }();
        }
        template<typename A, typename B, typename C>
        static Zip<T> from(std::tuple<A, B, C> t) {
            auto [A_shadow1, B_shadow1, C_shadow1] = rusty::detail::deref_if_pointer_like(t);
            return Zip<T>{.t = std::make_tuple(rusty::iter(A_shadow1), rusty::iter(B_shadow1), rusty::iter(C_shadow1))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A, typename B, typename C>
        auto next() {
            auto [A_shadow1, B_shadow1, C_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto A_shadow2 = ({ auto&& _m = A_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto B_shadow2 = ({ auto&& _m = B_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto C_shadow2 = ({ auto&& _m = C_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            return rusty::Some(std::make_tuple(A_shadow2, B_shadow2, C_shadow2));
        }
        template<typename A, typename B, typename C>
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto sh = std::make_tuple(std::numeric_limits<size_t>::max(), rusty::None);
            auto [A_shadow1, B_shadow1, C_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto sh_shadow1 = size_hint::min(rusty::size_hint(A_shadow1), std::move(sh));
            auto sh_shadow2 = size_hint::min(rusty::size_hint(B_shadow1), std::move(sh_shadow1));
            auto sh_shadow3 = size_hint::min(rusty::size_hint(C_shadow1), std::move(sh_shadow2));
            return sh_shadow3;
        }
        template<typename A, typename B, typename C>
        auto next_back() {
            auto [A_shadow1, B_shadow1, C_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            const auto size = rusty::detail::deref_if_pointer_like(rusty::iter(std::array{rusty::len(A_shadow1), rusty::len(B_shadow1), rusty::len(C_shadow1)}).min().unwrap());
            if (rusty::len(A_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(A_shadow1) - size))) {
                    A_shadow1.next_back();
                }
            }
            if (rusty::len(B_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(B_shadow1) - size))) {
                    B_shadow1.next_back();
                }
            }
            if (rusty::len(C_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(C_shadow1) - size))) {
                    C_shadow1.next_back();
                }
            }
            return [&]() { auto&& _m0 = A_shadow1.next_back(); auto&& _m1 = B_shadow1.next_back(); auto&& _m2 = C_shadow1.next_back(); if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some() && rusty::detail::deref_if_pointer(_m2).is_some()) { auto&& A_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& B_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); auto&& C_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m2)).unwrap()); return rusty::Some(std::make_tuple(A_shadow1, B_shadow1, C_shadow1)); } if (true) { return rusty::None; } rusty::intrinsics::unreachable(); }();
        }
        template<typename A, typename B, typename C, typename D>
        static Zip<T> from(std::tuple<A, B, C, D> t) {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1] = rusty::detail::deref_if_pointer_like(t);
            return Zip<T>{.t = std::make_tuple(rusty::iter(A_shadow1), rusty::iter(B_shadow1), rusty::iter(C_shadow1), rusty::iter(D_shadow1))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A, typename B, typename C, typename D>
        auto next() {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto A_shadow2 = ({ auto&& _m = A_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto B_shadow2 = ({ auto&& _m = B_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto C_shadow2 = ({ auto&& _m = C_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto D_shadow2 = ({ auto&& _m = D_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            return rusty::Some(std::make_tuple(A_shadow2, B_shadow2, C_shadow2, D_shadow2));
        }
        template<typename A, typename B, typename C, typename D>
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto sh = std::make_tuple(std::numeric_limits<size_t>::max(), rusty::None);
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto sh_shadow1 = size_hint::min(rusty::size_hint(A_shadow1), std::move(sh));
            auto sh_shadow2 = size_hint::min(rusty::size_hint(B_shadow1), std::move(sh_shadow1));
            auto sh_shadow3 = size_hint::min(rusty::size_hint(C_shadow1), std::move(sh_shadow2));
            auto sh_shadow4 = size_hint::min(rusty::size_hint(D_shadow1), std::move(sh_shadow3));
            return sh_shadow4;
        }
        template<typename A, typename B, typename C, typename D>
        auto next_back() {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            const auto size = rusty::detail::deref_if_pointer_like(rusty::iter(std::array{rusty::len(A_shadow1), rusty::len(B_shadow1), rusty::len(C_shadow1), rusty::len(D_shadow1)}).min().unwrap());
            if (rusty::len(A_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(A_shadow1) - size))) {
                    A_shadow1.next_back();
                }
            }
            if (rusty::len(B_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(B_shadow1) - size))) {
                    B_shadow1.next_back();
                }
            }
            if (rusty::len(C_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(C_shadow1) - size))) {
                    C_shadow1.next_back();
                }
            }
            if (rusty::len(D_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(D_shadow1) - size))) {
                    D_shadow1.next_back();
                }
            }
            return [&]() { auto&& _m0 = A_shadow1.next_back(); auto&& _m1 = B_shadow1.next_back(); auto&& _m2 = C_shadow1.next_back(); auto&& _m3 = D_shadow1.next_back(); if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some() && rusty::detail::deref_if_pointer(_m2).is_some() && rusty::detail::deref_if_pointer(_m3).is_some()) { auto&& A_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& B_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); auto&& C_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m2)).unwrap()); auto&& D_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m3)).unwrap()); return rusty::Some(std::make_tuple(A_shadow1, B_shadow1, C_shadow1, D_shadow1)); } if (true) { return rusty::None; } rusty::intrinsics::unreachable(); }();
        }
        template<typename A, typename B, typename C, typename D, typename E>
        static Zip<T> from(std::tuple<A, B, C, D, E> t) {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1] = rusty::detail::deref_if_pointer_like(t);
            return Zip<T>{.t = std::make_tuple(rusty::iter(A_shadow1), rusty::iter(B_shadow1), rusty::iter(C_shadow1), rusty::iter(D_shadow1), rusty::iter(E_shadow1))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A, typename B, typename C, typename D, typename E>
        auto next() {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto A_shadow2 = ({ auto&& _m = A_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto B_shadow2 = ({ auto&& _m = B_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto C_shadow2 = ({ auto&& _m = C_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto D_shadow2 = ({ auto&& _m = D_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto E_shadow2 = ({ auto&& _m = E_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            return rusty::Some(std::make_tuple(A_shadow2, B_shadow2, C_shadow2, D_shadow2, E_shadow2));
        }
        template<typename A, typename B, typename C, typename D, typename E>
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto sh = std::make_tuple(std::numeric_limits<size_t>::max(), rusty::None);
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto sh_shadow1 = size_hint::min(rusty::size_hint(A_shadow1), std::move(sh));
            auto sh_shadow2 = size_hint::min(rusty::size_hint(B_shadow1), std::move(sh_shadow1));
            auto sh_shadow3 = size_hint::min(rusty::size_hint(C_shadow1), std::move(sh_shadow2));
            auto sh_shadow4 = size_hint::min(rusty::size_hint(D_shadow1), std::move(sh_shadow3));
            auto sh_shadow5 = size_hint::min(rusty::size_hint(E_shadow1), std::move(sh_shadow4));
            return sh_shadow5;
        }
        template<typename A, typename B, typename C, typename D, typename E>
        auto next_back() {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            const auto size = rusty::detail::deref_if_pointer_like(rusty::iter(std::array{rusty::len(A_shadow1), rusty::len(B_shadow1), rusty::len(C_shadow1), rusty::len(D_shadow1), rusty::len(E_shadow1)}).min().unwrap());
            if (rusty::len(A_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(A_shadow1) - size))) {
                    A_shadow1.next_back();
                }
            }
            if (rusty::len(B_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(B_shadow1) - size))) {
                    B_shadow1.next_back();
                }
            }
            if (rusty::len(C_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(C_shadow1) - size))) {
                    C_shadow1.next_back();
                }
            }
            if (rusty::len(D_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(D_shadow1) - size))) {
                    D_shadow1.next_back();
                }
            }
            if (rusty::len(E_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(E_shadow1) - size))) {
                    E_shadow1.next_back();
                }
            }
            return [&]() { auto&& _m0 = A_shadow1.next_back(); auto&& _m1 = B_shadow1.next_back(); auto&& _m2 = C_shadow1.next_back(); auto&& _m3 = D_shadow1.next_back(); auto&& _m4 = E_shadow1.next_back(); if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some() && rusty::detail::deref_if_pointer(_m2).is_some() && rusty::detail::deref_if_pointer(_m3).is_some() && rusty::detail::deref_if_pointer(_m4).is_some()) { auto&& A_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& B_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); auto&& C_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m2)).unwrap()); auto&& D_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m3)).unwrap()); auto&& E_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m4)).unwrap()); return rusty::Some(std::make_tuple(A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1)); } if (true) { return rusty::None; } rusty::intrinsics::unreachable(); }();
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F>
        static Zip<T> from(std::tuple<A, B, C, D, E, F> t) {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1] = rusty::detail::deref_if_pointer_like(t);
            return Zip<T>{.t = std::make_tuple(rusty::iter(A_shadow1), rusty::iter(B_shadow1), rusty::iter(C_shadow1), rusty::iter(D_shadow1), rusty::iter(E_shadow1), rusty::iter(F_shadow1))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A, typename B, typename C, typename D, typename E, typename F>
        auto next() {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto A_shadow2 = ({ auto&& _m = A_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto B_shadow2 = ({ auto&& _m = B_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto C_shadow2 = ({ auto&& _m = C_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto D_shadow2 = ({ auto&& _m = D_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto E_shadow2 = ({ auto&& _m = E_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto F_shadow2 = ({ auto&& _m = F_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            return rusty::Some(std::make_tuple(A_shadow2, B_shadow2, C_shadow2, D_shadow2, E_shadow2, F_shadow2));
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F>
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto sh = std::make_tuple(std::numeric_limits<size_t>::max(), rusty::None);
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto sh_shadow1 = size_hint::min(rusty::size_hint(A_shadow1), std::move(sh));
            auto sh_shadow2 = size_hint::min(rusty::size_hint(B_shadow1), std::move(sh_shadow1));
            auto sh_shadow3 = size_hint::min(rusty::size_hint(C_shadow1), std::move(sh_shadow2));
            auto sh_shadow4 = size_hint::min(rusty::size_hint(D_shadow1), std::move(sh_shadow3));
            auto sh_shadow5 = size_hint::min(rusty::size_hint(E_shadow1), std::move(sh_shadow4));
            auto sh_shadow6 = size_hint::min(rusty::size_hint(F_shadow1), std::move(sh_shadow5));
            return sh_shadow6;
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F>
        auto next_back() {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            const auto size = rusty::detail::deref_if_pointer_like(rusty::iter(std::array{rusty::len(A_shadow1), rusty::len(B_shadow1), rusty::len(C_shadow1), rusty::len(D_shadow1), rusty::len(E_shadow1), rusty::len(F_shadow1)}).min().unwrap());
            if (rusty::len(A_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(A_shadow1) - size))) {
                    A_shadow1.next_back();
                }
            }
            if (rusty::len(B_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(B_shadow1) - size))) {
                    B_shadow1.next_back();
                }
            }
            if (rusty::len(C_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(C_shadow1) - size))) {
                    C_shadow1.next_back();
                }
            }
            if (rusty::len(D_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(D_shadow1) - size))) {
                    D_shadow1.next_back();
                }
            }
            if (rusty::len(E_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(E_shadow1) - size))) {
                    E_shadow1.next_back();
                }
            }
            if (rusty::len(F_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(F_shadow1) - size))) {
                    F_shadow1.next_back();
                }
            }
            return [&]() { auto&& _m0 = A_shadow1.next_back(); auto&& _m1 = B_shadow1.next_back(); auto&& _m2 = C_shadow1.next_back(); auto&& _m3 = D_shadow1.next_back(); auto&& _m4 = E_shadow1.next_back(); auto&& _m5 = F_shadow1.next_back(); if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some() && rusty::detail::deref_if_pointer(_m2).is_some() && rusty::detail::deref_if_pointer(_m3).is_some() && rusty::detail::deref_if_pointer(_m4).is_some() && rusty::detail::deref_if_pointer(_m5).is_some()) { auto&& A_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& B_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); auto&& C_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m2)).unwrap()); auto&& D_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m3)).unwrap()); auto&& E_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m4)).unwrap()); auto&& F_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m5)).unwrap()); return rusty::Some(std::make_tuple(A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1)); } if (true) { return rusty::None; } rusty::intrinsics::unreachable(); }();
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G>
        static Zip<T> from(std::tuple<A, B, C, D, E, F, G> t) {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1] = rusty::detail::deref_if_pointer_like(t);
            return Zip<T>{.t = std::make_tuple(rusty::iter(A_shadow1), rusty::iter(B_shadow1), rusty::iter(C_shadow1), rusty::iter(D_shadow1), rusty::iter(E_shadow1), rusty::iter(F_shadow1), rusty::iter(G_shadow1))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G>
        auto next() {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto A_shadow2 = ({ auto&& _m = A_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto B_shadow2 = ({ auto&& _m = B_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto C_shadow2 = ({ auto&& _m = C_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto D_shadow2 = ({ auto&& _m = D_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto E_shadow2 = ({ auto&& _m = E_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto F_shadow2 = ({ auto&& _m = F_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto G_shadow2 = ({ auto&& _m = G_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            return rusty::Some(std::make_tuple(A_shadow2, B_shadow2, C_shadow2, D_shadow2, E_shadow2, F_shadow2, G_shadow2));
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G>
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto sh = std::make_tuple(std::numeric_limits<size_t>::max(), rusty::None);
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto sh_shadow1 = size_hint::min(rusty::size_hint(A_shadow1), std::move(sh));
            auto sh_shadow2 = size_hint::min(rusty::size_hint(B_shadow1), std::move(sh_shadow1));
            auto sh_shadow3 = size_hint::min(rusty::size_hint(C_shadow1), std::move(sh_shadow2));
            auto sh_shadow4 = size_hint::min(rusty::size_hint(D_shadow1), std::move(sh_shadow3));
            auto sh_shadow5 = size_hint::min(rusty::size_hint(E_shadow1), std::move(sh_shadow4));
            auto sh_shadow6 = size_hint::min(rusty::size_hint(F_shadow1), std::move(sh_shadow5));
            auto sh_shadow7 = size_hint::min(rusty::size_hint(G_shadow1), std::move(sh_shadow6));
            return sh_shadow7;
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G>
        auto next_back() {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            const auto size = rusty::detail::deref_if_pointer_like(rusty::iter(std::array{rusty::len(A_shadow1), rusty::len(B_shadow1), rusty::len(C_shadow1), rusty::len(D_shadow1), rusty::len(E_shadow1), rusty::len(F_shadow1), rusty::len(G_shadow1)}).min().unwrap());
            if (rusty::len(A_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(A_shadow1) - size))) {
                    A_shadow1.next_back();
                }
            }
            if (rusty::len(B_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(B_shadow1) - size))) {
                    B_shadow1.next_back();
                }
            }
            if (rusty::len(C_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(C_shadow1) - size))) {
                    C_shadow1.next_back();
                }
            }
            if (rusty::len(D_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(D_shadow1) - size))) {
                    D_shadow1.next_back();
                }
            }
            if (rusty::len(E_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(E_shadow1) - size))) {
                    E_shadow1.next_back();
                }
            }
            if (rusty::len(F_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(F_shadow1) - size))) {
                    F_shadow1.next_back();
                }
            }
            if (rusty::len(G_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(G_shadow1) - size))) {
                    G_shadow1.next_back();
                }
            }
            return [&]() { auto&& _m0 = A_shadow1.next_back(); auto&& _m1 = B_shadow1.next_back(); auto&& _m2 = C_shadow1.next_back(); auto&& _m3 = D_shadow1.next_back(); auto&& _m4 = E_shadow1.next_back(); auto&& _m5 = F_shadow1.next_back(); auto&& _m6 = G_shadow1.next_back(); if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some() && rusty::detail::deref_if_pointer(_m2).is_some() && rusty::detail::deref_if_pointer(_m3).is_some() && rusty::detail::deref_if_pointer(_m4).is_some() && rusty::detail::deref_if_pointer(_m5).is_some() && rusty::detail::deref_if_pointer(_m6).is_some()) { auto&& A_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& B_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); auto&& C_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m2)).unwrap()); auto&& D_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m3)).unwrap()); auto&& E_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m4)).unwrap()); auto&& F_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m5)).unwrap()); auto&& G_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m6)).unwrap()); return rusty::Some(std::make_tuple(A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1)); } if (true) { return rusty::None; } rusty::intrinsics::unreachable(); }();
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H>
        static Zip<T> from(std::tuple<A, B, C, D, E, F, G, H> t) {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1] = rusty::detail::deref_if_pointer_like(t);
            return Zip<T>{.t = std::make_tuple(rusty::iter(A_shadow1), rusty::iter(B_shadow1), rusty::iter(C_shadow1), rusty::iter(D_shadow1), rusty::iter(E_shadow1), rusty::iter(F_shadow1), rusty::iter(G_shadow1), rusty::iter(H_shadow1))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H>
        auto next() {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto A_shadow2 = ({ auto&& _m = A_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto B_shadow2 = ({ auto&& _m = B_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto C_shadow2 = ({ auto&& _m = C_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto D_shadow2 = ({ auto&& _m = D_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto E_shadow2 = ({ auto&& _m = E_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto F_shadow2 = ({ auto&& _m = F_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto G_shadow2 = ({ auto&& _m = G_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto H_shadow2 = ({ auto&& _m = H_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            return rusty::Some(std::make_tuple(A_shadow2, B_shadow2, C_shadow2, D_shadow2, E_shadow2, F_shadow2, G_shadow2, H_shadow2));
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H>
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto sh = std::make_tuple(std::numeric_limits<size_t>::max(), rusty::None);
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto sh_shadow1 = size_hint::min(rusty::size_hint(A_shadow1), std::move(sh));
            auto sh_shadow2 = size_hint::min(rusty::size_hint(B_shadow1), std::move(sh_shadow1));
            auto sh_shadow3 = size_hint::min(rusty::size_hint(C_shadow1), std::move(sh_shadow2));
            auto sh_shadow4 = size_hint::min(rusty::size_hint(D_shadow1), std::move(sh_shadow3));
            auto sh_shadow5 = size_hint::min(rusty::size_hint(E_shadow1), std::move(sh_shadow4));
            auto sh_shadow6 = size_hint::min(rusty::size_hint(F_shadow1), std::move(sh_shadow5));
            auto sh_shadow7 = size_hint::min(rusty::size_hint(G_shadow1), std::move(sh_shadow6));
            auto sh_shadow8 = size_hint::min(rusty::size_hint(H_shadow1), std::move(sh_shadow7));
            return sh_shadow8;
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H>
        auto next_back() {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            const auto size = rusty::detail::deref_if_pointer_like(rusty::iter(std::array{rusty::len(A_shadow1), rusty::len(B_shadow1), rusty::len(C_shadow1), rusty::len(D_shadow1), rusty::len(E_shadow1), rusty::len(F_shadow1), rusty::len(G_shadow1), rusty::len(H_shadow1)}).min().unwrap());
            if (rusty::len(A_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(A_shadow1) - size))) {
                    A_shadow1.next_back();
                }
            }
            if (rusty::len(B_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(B_shadow1) - size))) {
                    B_shadow1.next_back();
                }
            }
            if (rusty::len(C_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(C_shadow1) - size))) {
                    C_shadow1.next_back();
                }
            }
            if (rusty::len(D_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(D_shadow1) - size))) {
                    D_shadow1.next_back();
                }
            }
            if (rusty::len(E_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(E_shadow1) - size))) {
                    E_shadow1.next_back();
                }
            }
            if (rusty::len(F_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(F_shadow1) - size))) {
                    F_shadow1.next_back();
                }
            }
            if (rusty::len(G_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(G_shadow1) - size))) {
                    G_shadow1.next_back();
                }
            }
            if (rusty::len(H_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(H_shadow1) - size))) {
                    H_shadow1.next_back();
                }
            }
            return [&]() { auto&& _m0 = A_shadow1.next_back(); auto&& _m1 = B_shadow1.next_back(); auto&& _m2 = C_shadow1.next_back(); auto&& _m3 = D_shadow1.next_back(); auto&& _m4 = E_shadow1.next_back(); auto&& _m5 = F_shadow1.next_back(); auto&& _m6 = G_shadow1.next_back(); auto&& _m7 = H_shadow1.next_back(); if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some() && rusty::detail::deref_if_pointer(_m2).is_some() && rusty::detail::deref_if_pointer(_m3).is_some() && rusty::detail::deref_if_pointer(_m4).is_some() && rusty::detail::deref_if_pointer(_m5).is_some() && rusty::detail::deref_if_pointer(_m6).is_some() && rusty::detail::deref_if_pointer(_m7).is_some()) { auto&& A_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& B_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); auto&& C_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m2)).unwrap()); auto&& D_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m3)).unwrap()); auto&& E_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m4)).unwrap()); auto&& F_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m5)).unwrap()); auto&& G_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m6)).unwrap()); auto&& H_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m7)).unwrap()); return rusty::Some(std::make_tuple(A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1)); } if (true) { return rusty::None; } rusty::intrinsics::unreachable(); }();
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H, typename I>
        static Zip<T> from(std::tuple<A, B, C, D, E, F, G, H, I> t) {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1] = rusty::detail::deref_if_pointer_like(t);
            return Zip<T>{.t = std::make_tuple(rusty::iter(A_shadow1), rusty::iter(B_shadow1), rusty::iter(C_shadow1), rusty::iter(D_shadow1), rusty::iter(E_shadow1), rusty::iter(F_shadow1), rusty::iter(G_shadow1), rusty::iter(H_shadow1), rusty::iter(I_shadow1))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H, typename I>
        auto next() {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto A_shadow2 = ({ auto&& _m = A_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto B_shadow2 = ({ auto&& _m = B_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto C_shadow2 = ({ auto&& _m = C_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto D_shadow2 = ({ auto&& _m = D_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto E_shadow2 = ({ auto&& _m = E_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto F_shadow2 = ({ auto&& _m = F_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto G_shadow2 = ({ auto&& _m = G_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto H_shadow2 = ({ auto&& _m = H_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto I_shadow2 = ({ auto&& _m = I_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            return rusty::Some(std::make_tuple(A_shadow2, B_shadow2, C_shadow2, D_shadow2, E_shadow2, F_shadow2, G_shadow2, H_shadow2, I_shadow2));
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H, typename I>
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto sh = std::make_tuple(std::numeric_limits<size_t>::max(), rusty::None);
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto sh_shadow1 = size_hint::min(rusty::size_hint(A_shadow1), std::move(sh));
            auto sh_shadow2 = size_hint::min(rusty::size_hint(B_shadow1), std::move(sh_shadow1));
            auto sh_shadow3 = size_hint::min(rusty::size_hint(C_shadow1), std::move(sh_shadow2));
            auto sh_shadow4 = size_hint::min(rusty::size_hint(D_shadow1), std::move(sh_shadow3));
            auto sh_shadow5 = size_hint::min(rusty::size_hint(E_shadow1), std::move(sh_shadow4));
            auto sh_shadow6 = size_hint::min(rusty::size_hint(F_shadow1), std::move(sh_shadow5));
            auto sh_shadow7 = size_hint::min(rusty::size_hint(G_shadow1), std::move(sh_shadow6));
            auto sh_shadow8 = size_hint::min(rusty::size_hint(H_shadow1), std::move(sh_shadow7));
            auto sh_shadow9 = size_hint::min(rusty::size_hint(I_shadow1), std::move(sh_shadow8));
            return sh_shadow9;
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H, typename I>
        auto next_back() {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            const auto size = rusty::detail::deref_if_pointer_like(rusty::iter(std::array{rusty::len(A_shadow1), rusty::len(B_shadow1), rusty::len(C_shadow1), rusty::len(D_shadow1), rusty::len(E_shadow1), rusty::len(F_shadow1), rusty::len(G_shadow1), rusty::len(H_shadow1), rusty::len(I_shadow1)}).min().unwrap());
            if (rusty::len(A_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(A_shadow1) - size))) {
                    A_shadow1.next_back();
                }
            }
            if (rusty::len(B_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(B_shadow1) - size))) {
                    B_shadow1.next_back();
                }
            }
            if (rusty::len(C_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(C_shadow1) - size))) {
                    C_shadow1.next_back();
                }
            }
            if (rusty::len(D_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(D_shadow1) - size))) {
                    D_shadow1.next_back();
                }
            }
            if (rusty::len(E_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(E_shadow1) - size))) {
                    E_shadow1.next_back();
                }
            }
            if (rusty::len(F_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(F_shadow1) - size))) {
                    F_shadow1.next_back();
                }
            }
            if (rusty::len(G_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(G_shadow1) - size))) {
                    G_shadow1.next_back();
                }
            }
            if (rusty::len(H_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(H_shadow1) - size))) {
                    H_shadow1.next_back();
                }
            }
            if (rusty::len(I_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(I_shadow1) - size))) {
                    I_shadow1.next_back();
                }
            }
            return [&]() { auto&& _m0 = A_shadow1.next_back(); auto&& _m1 = B_shadow1.next_back(); auto&& _m2 = C_shadow1.next_back(); auto&& _m3 = D_shadow1.next_back(); auto&& _m4 = E_shadow1.next_back(); auto&& _m5 = F_shadow1.next_back(); auto&& _m6 = G_shadow1.next_back(); auto&& _m7 = H_shadow1.next_back(); auto&& _m8 = I_shadow1.next_back(); if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some() && rusty::detail::deref_if_pointer(_m2).is_some() && rusty::detail::deref_if_pointer(_m3).is_some() && rusty::detail::deref_if_pointer(_m4).is_some() && rusty::detail::deref_if_pointer(_m5).is_some() && rusty::detail::deref_if_pointer(_m6).is_some() && rusty::detail::deref_if_pointer(_m7).is_some() && rusty::detail::deref_if_pointer(_m8).is_some()) { auto&& A_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& B_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); auto&& C_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m2)).unwrap()); auto&& D_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m3)).unwrap()); auto&& E_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m4)).unwrap()); auto&& F_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m5)).unwrap()); auto&& G_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m6)).unwrap()); auto&& H_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m7)).unwrap()); auto&& I_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m8)).unwrap()); return rusty::Some(std::make_tuple(A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1)); } if (true) { return rusty::None; } rusty::intrinsics::unreachable(); }();
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H, typename I, typename J>
        static Zip<T> from(std::tuple<A, B, C, D, E, F, G, H, I, J> t) {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1] = rusty::detail::deref_if_pointer_like(t);
            return Zip<T>{.t = std::make_tuple(rusty::iter(A_shadow1), rusty::iter(B_shadow1), rusty::iter(C_shadow1), rusty::iter(D_shadow1), rusty::iter(E_shadow1), rusty::iter(F_shadow1), rusty::iter(G_shadow1), rusty::iter(H_shadow1), rusty::iter(I_shadow1), rusty::iter(J_shadow1))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H, typename I, typename J>
        auto next() {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto A_shadow2 = ({ auto&& _m = A_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto B_shadow2 = ({ auto&& _m = B_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto C_shadow2 = ({ auto&& _m = C_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto D_shadow2 = ({ auto&& _m = D_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto E_shadow2 = ({ auto&& _m = E_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto F_shadow2 = ({ auto&& _m = F_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto G_shadow2 = ({ auto&& _m = G_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto H_shadow2 = ({ auto&& _m = H_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto I_shadow2 = ({ auto&& _m = I_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto J_shadow2 = ({ auto&& _m = J_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            return rusty::Some(std::make_tuple(A_shadow2, B_shadow2, C_shadow2, D_shadow2, E_shadow2, F_shadow2, G_shadow2, H_shadow2, I_shadow2, J_shadow2));
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H, typename I, typename J>
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto sh = std::make_tuple(std::numeric_limits<size_t>::max(), rusty::None);
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto sh_shadow1 = size_hint::min(rusty::size_hint(A_shadow1), std::move(sh));
            auto sh_shadow2 = size_hint::min(rusty::size_hint(B_shadow1), std::move(sh_shadow1));
            auto sh_shadow3 = size_hint::min(rusty::size_hint(C_shadow1), std::move(sh_shadow2));
            auto sh_shadow4 = size_hint::min(rusty::size_hint(D_shadow1), std::move(sh_shadow3));
            auto sh_shadow5 = size_hint::min(rusty::size_hint(E_shadow1), std::move(sh_shadow4));
            auto sh_shadow6 = size_hint::min(rusty::size_hint(F_shadow1), std::move(sh_shadow5));
            auto sh_shadow7 = size_hint::min(rusty::size_hint(G_shadow1), std::move(sh_shadow6));
            auto sh_shadow8 = size_hint::min(rusty::size_hint(H_shadow1), std::move(sh_shadow7));
            auto sh_shadow9 = size_hint::min(rusty::size_hint(I_shadow1), std::move(sh_shadow8));
            auto sh_shadow10 = size_hint::min(rusty::size_hint(J_shadow1), std::move(sh_shadow9));
            return sh_shadow10;
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H, typename I, typename J>
        auto next_back() {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            const auto size = rusty::detail::deref_if_pointer_like(rusty::iter(std::array{rusty::len(A_shadow1), rusty::len(B_shadow1), rusty::len(C_shadow1), rusty::len(D_shadow1), rusty::len(E_shadow1), rusty::len(F_shadow1), rusty::len(G_shadow1), rusty::len(H_shadow1), rusty::len(I_shadow1), rusty::len(J_shadow1)}).min().unwrap());
            if (rusty::len(A_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(A_shadow1) - size))) {
                    A_shadow1.next_back();
                }
            }
            if (rusty::len(B_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(B_shadow1) - size))) {
                    B_shadow1.next_back();
                }
            }
            if (rusty::len(C_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(C_shadow1) - size))) {
                    C_shadow1.next_back();
                }
            }
            if (rusty::len(D_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(D_shadow1) - size))) {
                    D_shadow1.next_back();
                }
            }
            if (rusty::len(E_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(E_shadow1) - size))) {
                    E_shadow1.next_back();
                }
            }
            if (rusty::len(F_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(F_shadow1) - size))) {
                    F_shadow1.next_back();
                }
            }
            if (rusty::len(G_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(G_shadow1) - size))) {
                    G_shadow1.next_back();
                }
            }
            if (rusty::len(H_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(H_shadow1) - size))) {
                    H_shadow1.next_back();
                }
            }
            if (rusty::len(I_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(I_shadow1) - size))) {
                    I_shadow1.next_back();
                }
            }
            if (rusty::len(J_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(J_shadow1) - size))) {
                    J_shadow1.next_back();
                }
            }
            return [&]() { auto&& _m0 = A_shadow1.next_back(); auto&& _m1 = B_shadow1.next_back(); auto&& _m2 = C_shadow1.next_back(); auto&& _m3 = D_shadow1.next_back(); auto&& _m4 = E_shadow1.next_back(); auto&& _m5 = F_shadow1.next_back(); auto&& _m6 = G_shadow1.next_back(); auto&& _m7 = H_shadow1.next_back(); auto&& _m8 = I_shadow1.next_back(); auto&& _m9 = J_shadow1.next_back(); if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some() && rusty::detail::deref_if_pointer(_m2).is_some() && rusty::detail::deref_if_pointer(_m3).is_some() && rusty::detail::deref_if_pointer(_m4).is_some() && rusty::detail::deref_if_pointer(_m5).is_some() && rusty::detail::deref_if_pointer(_m6).is_some() && rusty::detail::deref_if_pointer(_m7).is_some() && rusty::detail::deref_if_pointer(_m8).is_some() && rusty::detail::deref_if_pointer(_m9).is_some()) { auto&& A_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& B_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); auto&& C_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m2)).unwrap()); auto&& D_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m3)).unwrap()); auto&& E_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m4)).unwrap()); auto&& F_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m5)).unwrap()); auto&& G_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m6)).unwrap()); auto&& H_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m7)).unwrap()); auto&& I_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m8)).unwrap()); auto&& J_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m9)).unwrap()); return rusty::Some(std::make_tuple(A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1)); } if (true) { return rusty::None; } rusty::intrinsics::unreachable(); }();
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H, typename I, typename J, typename K>
        static Zip<T> from(std::tuple<A, B, C, D, E, F, G, H, I, J, K> t) {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1, K_shadow1] = rusty::detail::deref_if_pointer_like(t);
            return Zip<T>{.t = std::make_tuple(rusty::iter(A_shadow1), rusty::iter(B_shadow1), rusty::iter(C_shadow1), rusty::iter(D_shadow1), rusty::iter(E_shadow1), rusty::iter(F_shadow1), rusty::iter(G_shadow1), rusty::iter(H_shadow1), rusty::iter(I_shadow1), rusty::iter(J_shadow1), rusty::iter(K_shadow1))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H, typename I, typename J, typename K>
        auto next() {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1, K_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto A_shadow2 = ({ auto&& _m = A_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto B_shadow2 = ({ auto&& _m = B_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto C_shadow2 = ({ auto&& _m = C_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto D_shadow2 = ({ auto&& _m = D_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto E_shadow2 = ({ auto&& _m = E_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto F_shadow2 = ({ auto&& _m = F_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto G_shadow2 = ({ auto&& _m = G_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto H_shadow2 = ({ auto&& _m = H_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto I_shadow2 = ({ auto&& _m = I_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto J_shadow2 = ({ auto&& _m = J_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto K_shadow2 = ({ auto&& _m = K_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            return rusty::Some(std::make_tuple(A_shadow2, B_shadow2, C_shadow2, D_shadow2, E_shadow2, F_shadow2, G_shadow2, H_shadow2, I_shadow2, J_shadow2, K_shadow2));
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H, typename I, typename J, typename K>
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto sh = std::make_tuple(std::numeric_limits<size_t>::max(), rusty::None);
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1, K_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto sh_shadow1 = size_hint::min(rusty::size_hint(A_shadow1), std::move(sh));
            auto sh_shadow2 = size_hint::min(rusty::size_hint(B_shadow1), std::move(sh_shadow1));
            auto sh_shadow3 = size_hint::min(rusty::size_hint(C_shadow1), std::move(sh_shadow2));
            auto sh_shadow4 = size_hint::min(rusty::size_hint(D_shadow1), std::move(sh_shadow3));
            auto sh_shadow5 = size_hint::min(rusty::size_hint(E_shadow1), std::move(sh_shadow4));
            auto sh_shadow6 = size_hint::min(rusty::size_hint(F_shadow1), std::move(sh_shadow5));
            auto sh_shadow7 = size_hint::min(rusty::size_hint(G_shadow1), std::move(sh_shadow6));
            auto sh_shadow8 = size_hint::min(rusty::size_hint(H_shadow1), std::move(sh_shadow7));
            auto sh_shadow9 = size_hint::min(rusty::size_hint(I_shadow1), std::move(sh_shadow8));
            auto sh_shadow10 = size_hint::min(rusty::size_hint(J_shadow1), std::move(sh_shadow9));
            auto sh_shadow11 = size_hint::min(rusty::size_hint(K_shadow1), std::move(sh_shadow10));
            return sh_shadow11;
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H, typename I, typename J, typename K>
        auto next_back() {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1, K_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            const auto size = rusty::detail::deref_if_pointer_like(rusty::iter(std::array{rusty::len(A_shadow1), rusty::len(B_shadow1), rusty::len(C_shadow1), rusty::len(D_shadow1), rusty::len(E_shadow1), rusty::len(F_shadow1), rusty::len(G_shadow1), rusty::len(H_shadow1), rusty::len(I_shadow1), rusty::len(J_shadow1), rusty::len(K_shadow1)}).min().unwrap());
            if (rusty::len(A_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(A_shadow1) - size))) {
                    A_shadow1.next_back();
                }
            }
            if (rusty::len(B_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(B_shadow1) - size))) {
                    B_shadow1.next_back();
                }
            }
            if (rusty::len(C_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(C_shadow1) - size))) {
                    C_shadow1.next_back();
                }
            }
            if (rusty::len(D_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(D_shadow1) - size))) {
                    D_shadow1.next_back();
                }
            }
            if (rusty::len(E_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(E_shadow1) - size))) {
                    E_shadow1.next_back();
                }
            }
            if (rusty::len(F_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(F_shadow1) - size))) {
                    F_shadow1.next_back();
                }
            }
            if (rusty::len(G_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(G_shadow1) - size))) {
                    G_shadow1.next_back();
                }
            }
            if (rusty::len(H_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(H_shadow1) - size))) {
                    H_shadow1.next_back();
                }
            }
            if (rusty::len(I_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(I_shadow1) - size))) {
                    I_shadow1.next_back();
                }
            }
            if (rusty::len(J_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(J_shadow1) - size))) {
                    J_shadow1.next_back();
                }
            }
            if (rusty::len(K_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(K_shadow1) - size))) {
                    K_shadow1.next_back();
                }
            }
            return [&]() { auto&& _m0 = A_shadow1.next_back(); auto&& _m1 = B_shadow1.next_back(); auto&& _m2 = C_shadow1.next_back(); auto&& _m3 = D_shadow1.next_back(); auto&& _m4 = E_shadow1.next_back(); auto&& _m5 = F_shadow1.next_back(); auto&& _m6 = G_shadow1.next_back(); auto&& _m7 = H_shadow1.next_back(); auto&& _m8 = I_shadow1.next_back(); auto&& _m9 = J_shadow1.next_back(); auto&& _m10 = K_shadow1.next_back(); if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some() && rusty::detail::deref_if_pointer(_m2).is_some() && rusty::detail::deref_if_pointer(_m3).is_some() && rusty::detail::deref_if_pointer(_m4).is_some() && rusty::detail::deref_if_pointer(_m5).is_some() && rusty::detail::deref_if_pointer(_m6).is_some() && rusty::detail::deref_if_pointer(_m7).is_some() && rusty::detail::deref_if_pointer(_m8).is_some() && rusty::detail::deref_if_pointer(_m9).is_some() && rusty::detail::deref_if_pointer(_m10).is_some()) { auto&& A_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& B_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); auto&& C_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m2)).unwrap()); auto&& D_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m3)).unwrap()); auto&& E_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m4)).unwrap()); auto&& F_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m5)).unwrap()); auto&& G_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m6)).unwrap()); auto&& H_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m7)).unwrap()); auto&& I_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m8)).unwrap()); auto&& J_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m9)).unwrap()); auto&& K_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m10)).unwrap()); return rusty::Some(std::make_tuple(A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1, K_shadow1)); } if (true) { return rusty::None; } rusty::intrinsics::unreachable(); }();
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H, typename I, typename J, typename K, typename L>
        static Zip<T> from(std::tuple<A, B, C, D, E, F, G, H, I, J, K, L> t) {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1, K_shadow1, L_shadow1] = rusty::detail::deref_if_pointer_like(t);
            return Zip<T>{.t = std::make_tuple(rusty::iter(A_shadow1), rusty::iter(B_shadow1), rusty::iter(C_shadow1), rusty::iter(D_shadow1), rusty::iter(E_shadow1), rusty::iter(F_shadow1), rusty::iter(G_shadow1), rusty::iter(H_shadow1), rusty::iter(I_shadow1), rusty::iter(J_shadow1), rusty::iter(K_shadow1), rusty::iter(L_shadow1))};
        }
        // Rust-only associated type alias with unbound generic skipped in constrained mode: Item
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H, typename I, typename J, typename K, typename L>
        auto next() {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1, K_shadow1, L_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto A_shadow2 = ({ auto&& _m = A_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto B_shadow2 = ({ auto&& _m = B_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto C_shadow2 = ({ auto&& _m = C_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto D_shadow2 = ({ auto&& _m = D_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto E_shadow2 = ({ auto&& _m = E_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto F_shadow2 = ({ auto&& _m = F_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto G_shadow2 = ({ auto&& _m = G_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto H_shadow2 = ({ auto&& _m = H_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto I_shadow2 = ({ auto&& _m = I_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto J_shadow2 = ({ auto&& _m = J_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto K_shadow2 = ({ auto&& _m = K_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            auto L_shadow2 = ({ auto&& _m = L_shadow1.next(); std::optional<std::remove_cvref_t<decltype(_m.unwrap())>> _match_value; if (_m.is_some()) { auto _mv = _m.unwrap();
auto&& elt = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_mv));
_match_value.emplace(std::move(_mv)); } else { if (!(_m.is_none())) { rusty::intrinsics::unreachable(); } return rusty::None; } std::move(_match_value).value(); });
            return rusty::Some(std::make_tuple(A_shadow2, B_shadow2, C_shadow2, D_shadow2, E_shadow2, F_shadow2, G_shadow2, H_shadow2, I_shadow2, J_shadow2, K_shadow2, L_shadow2));
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H, typename I, typename J, typename K, typename L>
        std::tuple<size_t, rusty::Option<size_t>> size_hint() const {
            auto sh = std::make_tuple(std::numeric_limits<size_t>::max(), rusty::None);
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1, K_shadow1, L_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            auto sh_shadow1 = size_hint::min(rusty::size_hint(A_shadow1), std::move(sh));
            auto sh_shadow2 = size_hint::min(rusty::size_hint(B_shadow1), std::move(sh_shadow1));
            auto sh_shadow3 = size_hint::min(rusty::size_hint(C_shadow1), std::move(sh_shadow2));
            auto sh_shadow4 = size_hint::min(rusty::size_hint(D_shadow1), std::move(sh_shadow3));
            auto sh_shadow5 = size_hint::min(rusty::size_hint(E_shadow1), std::move(sh_shadow4));
            auto sh_shadow6 = size_hint::min(rusty::size_hint(F_shadow1), std::move(sh_shadow5));
            auto sh_shadow7 = size_hint::min(rusty::size_hint(G_shadow1), std::move(sh_shadow6));
            auto sh_shadow8 = size_hint::min(rusty::size_hint(H_shadow1), std::move(sh_shadow7));
            auto sh_shadow9 = size_hint::min(rusty::size_hint(I_shadow1), std::move(sh_shadow8));
            auto sh_shadow10 = size_hint::min(rusty::size_hint(J_shadow1), std::move(sh_shadow9));
            auto sh_shadow11 = size_hint::min(rusty::size_hint(K_shadow1), std::move(sh_shadow10));
            auto sh_shadow12 = size_hint::min(rusty::size_hint(L_shadow1), std::move(sh_shadow11));
            return sh_shadow12;
        }
        template<typename A, typename B, typename C, typename D, typename E, typename F, typename G, typename H, typename I, typename J, typename K, typename L>
        auto next_back() {
            auto [A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1, K_shadow1, L_shadow1] = rusty::detail::deref_if_pointer_like(this->t);
            const auto size = rusty::detail::deref_if_pointer_like(rusty::iter(std::array{rusty::len(A_shadow1), rusty::len(B_shadow1), rusty::len(C_shadow1), rusty::len(D_shadow1), rusty::len(E_shadow1), rusty::len(F_shadow1), rusty::len(G_shadow1), rusty::len(H_shadow1), rusty::len(I_shadow1), rusty::len(J_shadow1), rusty::len(K_shadow1), rusty::len(L_shadow1)}).min().unwrap());
            if (rusty::len(A_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(A_shadow1) - size))) {
                    A_shadow1.next_back();
                }
            }
            if (rusty::len(B_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(B_shadow1) - size))) {
                    B_shadow1.next_back();
                }
            }
            if (rusty::len(C_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(C_shadow1) - size))) {
                    C_shadow1.next_back();
                }
            }
            if (rusty::len(D_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(D_shadow1) - size))) {
                    D_shadow1.next_back();
                }
            }
            if (rusty::len(E_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(E_shadow1) - size))) {
                    E_shadow1.next_back();
                }
            }
            if (rusty::len(F_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(F_shadow1) - size))) {
                    F_shadow1.next_back();
                }
            }
            if (rusty::len(G_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(G_shadow1) - size))) {
                    G_shadow1.next_back();
                }
            }
            if (rusty::len(H_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(H_shadow1) - size))) {
                    H_shadow1.next_back();
                }
            }
            if (rusty::len(I_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(I_shadow1) - size))) {
                    I_shadow1.next_back();
                }
            }
            if (rusty::len(J_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(J_shadow1) - size))) {
                    J_shadow1.next_back();
                }
            }
            if (rusty::len(K_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(K_shadow1) - size))) {
                    K_shadow1.next_back();
                }
            }
            if (rusty::len(L_shadow1) != size) {
                for (auto&& _ : rusty::for_in(rusty::range(0, rusty::len(L_shadow1) - size))) {
                    L_shadow1.next_back();
                }
            }
            return [&]() { auto&& _m0 = A_shadow1.next_back(); auto&& _m1 = B_shadow1.next_back(); auto&& _m2 = C_shadow1.next_back(); auto&& _m3 = D_shadow1.next_back(); auto&& _m4 = E_shadow1.next_back(); auto&& _m5 = F_shadow1.next_back(); auto&& _m6 = G_shadow1.next_back(); auto&& _m7 = H_shadow1.next_back(); auto&& _m8 = I_shadow1.next_back(); auto&& _m9 = J_shadow1.next_back(); auto&& _m10 = K_shadow1.next_back(); auto&& _m11 = L_shadow1.next_back(); if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some() && rusty::detail::deref_if_pointer(_m2).is_some() && rusty::detail::deref_if_pointer(_m3).is_some() && rusty::detail::deref_if_pointer(_m4).is_some() && rusty::detail::deref_if_pointer(_m5).is_some() && rusty::detail::deref_if_pointer(_m6).is_some() && rusty::detail::deref_if_pointer(_m7).is_some() && rusty::detail::deref_if_pointer(_m8).is_some() && rusty::detail::deref_if_pointer(_m9).is_some() && rusty::detail::deref_if_pointer(_m10).is_some() && rusty::detail::deref_if_pointer(_m11).is_some()) { auto&& A_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& B_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); auto&& C_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m2)).unwrap()); auto&& D_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m3)).unwrap()); auto&& E_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m4)).unwrap()); auto&& F_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m5)).unwrap()); auto&& G_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m6)).unwrap()); auto&& H_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m7)).unwrap()); auto&& I_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m8)).unwrap()); auto&& J_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m9)).unwrap()); auto&& K_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m10)).unwrap()); auto&& L_shadow1 = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m11)).unwrap()); return rusty::Some(std::make_tuple(A_shadow1, B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1, K_shadow1, L_shadow1)); } if (true) { return rusty::None; } rusty::intrinsics::unreachable(); }();
        }
    };

}

namespace structs {

    using adaptors::Batching;
    using adaptors::FilterMapOk;
    using adaptors::FilterOk;
    using adaptors::Interleave;
    using adaptors::InterleaveShortest;
    using adaptors::Positions;
    using adaptors::Product;
    using adaptors::PutBack;
    using adaptors::TakeWhileRef;
    using adaptors::TupleCombinations;
    using adaptors::Update;
    using adaptors::WhileSome;
    using combinations_with_replacement::CombinationsWithReplacement;
    using exactly_one_err::ExactlyOneError;
    using flatten_ok::FlattenOk;
    using format::Format;
    using format::FormatWith;
    using groupbylazy::Chunk;
    using groupbylazy::ChunkBy;
    using groupbylazy::Chunks;
    using groupbylazy::Group;
    using groupbylazy::Groups;
    using groupbylazy::IntoChunks;
    using grouping_map::GroupingMap;
    using kmerge_impl::KMergeBy;
    using merge_join::MergeBy;
    using multipeek_impl::MultiPeek;
    using pad_tail::PadUsing;
    using peek_nth::PeekNth;
    using peeking_take_while::PeekingTakeWhile;
    using permutations::Permutations;
    using powerset::Powerset;
    using process_results_impl::ProcessResults;
    using put_back_n_impl::PutBackN;
    using rciter_impl::RcIter;
    using repeatn::RepeatN;
    using sources::Iterate;
    using sources::Unfold;
    using take_while_inclusive::TakeWhileInclusive;
    using tee::Tee;
    using tuple_impl::CircularTupleWindows;
    using tuple_impl::TupleBuffer;
    using tuple_impl::TupleWindows;
    using tuple_impl::Tuples;
    using unique_impl::Unique;
    using unique_impl::UniqueBy;
    using with_position::WithPosition;
    using zip_eq_impl::ZipEq;
    using zip_longest::ZipLongest;
    using ziptuple::Zip;

    using adaptors::multi_product::MultiProduct;

    using adaptors::Batching;
    using adaptors::coalesce_tests::Coalesce;
    using adaptors::coalesce_tests::Dedup;
    using adaptors::coalesce_tests::DedupBy;
    using adaptors::coalesce_tests::DedupByWithCount;
    using adaptors::coalesce_tests::DedupWithCount;
    using adaptors::FilterMapOk;
    using adaptors::FilterOk;
    using adaptors::Interleave;
    using adaptors::InterleaveShortest;
    using adaptors::map::MapInto;
    using adaptors::map::MapOk;
    using adaptors::Positions;
    using adaptors::Product;
    using adaptors::PutBack;
    using adaptors::TakeWhileRef;
    using adaptors::TupleCombinations;
    using adaptors::Update;
    using adaptors::WhileSome;

    using combinations::ArrayCombinations;
    using combinations::Combinations;

    using combinations_with_replacement::CombinationsWithReplacement;

    using cons_tuples_impl::ConsTuples;

    using duplicates_impl::Duplicates;
    using duplicates_impl::DuplicatesBy;

    using exactly_one_err::ExactlyOneError;

    using flatten_ok::FlattenOk;

    using format::Format;
    using format::FormatWith;

    using groupbylazy::GroupBy;

    using groupbylazy::Chunk;
    using groupbylazy::ChunkBy;
    using groupbylazy::Chunks;
    using groupbylazy::Group;
    using groupbylazy::Groups;
    using groupbylazy::IntoChunks;

    using grouping_map::GroupingMap;
    using grouping_map::GroupingMapBy;

    using intersperse_tests::Intersperse;
    using intersperse_tests::IntersperseWith;

    using kmerge_impl::KMerge;
    using kmerge_impl::KMergeBy;

    using merge_join::Merge;
    using merge_join::MergeBy;
    using merge_join::MergeJoinBy;

    using multipeek_impl::MultiPeek;

    using pad_tail::PadUsing;

    using peek_nth::PeekNth;

    using peeking_take_while::PeekingTakeWhile;

    using permutations::Permutations;

    using powerset::Powerset;

    using process_results_impl::ProcessResults;

    using put_back_n_impl::PutBackN;

    using rciter_impl::RcIter;

    using repeatn::RepeatN;

    using sources::Iterate;
    using sources::Unfold;

    using take_while_inclusive::TakeWhileInclusive;

    using tee::Tee;

    using tuple_impl::CircularTupleWindows;
    using tuple_impl::TupleBuffer;
    using tuple_impl::TupleWindows;
    using tuple_impl::Tuples;

    using unique_impl::Unique;
    using unique_impl::UniqueBy;

    using with_position::WithPosition;

    using zip_eq_impl::ZipEq;

    using zip_longest::ZipLongest;

    using ziptuple::Zip;

}

namespace free_mod {

    template<typename I>
    decltype(std::declval<typename I::IntoIter>().intersperse(std::declval<typename I::IntoIter::Item>())) intersperse(I iterable, rusty::detail::associated_item_t<I> element);
    template<typename I, typename F>
    decltype(std::declval<typename I::IntoIter>().intersperse_with(std::declval<F>())) intersperse_with(I iterable, F element);
    template<typename I>
    decltype(std::declval<typename I::IntoIter>().enumerate()) enumerate(I iterable);
    template<typename I>
    decltype(std::declval<typename I::IntoIter>().rev()) rev(I iterable);
    template<typename I, typename J>
    decltype(rusty::zip(std::declval<typename I::IntoIter>(), std::declval<typename J::IntoIter>())) zip(I i, J j);
    template<typename I, typename J>
    decltype(std::declval<typename I::IntoIter>().chain(std::declval<typename J::IntoIter>())) chain(I i, J j);
    template<typename I, typename T>
    decltype(std::declval<typename I::IntoIter>().cloned()) cloned(I iterable);
    template<typename I, typename B, typename F>
    B fold(I iterable, B init, F f);
    template<typename I, typename F>
    bool all(I iterable, F f);
    template<typename I, typename F>
    bool any(I iterable, F f);
    template<typename I>
    rusty::Option<rusty::detail::associated_item_t<I>> max(I iterable);
    template<typename I>
    rusty::Option<rusty::detail::associated_item_t<I>> min(I iterable);
    template<typename I>
    rusty::String join(I iterable, std::string_view sep);
    template<typename I>
    VecIntoIter<rusty::detail::associated_item_t<I>> sorted(I iterable);
    template<typename I>
    VecIntoIter<rusty::detail::associated_item_t<I>> sorted_unstable(I iterable);




    using ::rusty::String;

    using intersperse_tests::Intersperse;
    using intersperse_tests::IntersperseWith;


    using adaptors::interleave;
    using adaptors::put_back;

    using kmerge_impl::kmerge;

    using merge_join::merge;
    using merge_join::merge_join_by;

    using multipeek_impl::multipeek;

    using peek_nth::peek_nth;

    using put_back_n_impl::put_back_n;

    using rciter_impl::rciter;

    using zip_eq_impl::zip_eq;

    /// Iterate `iterable` with a particular value inserted between each element.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::intersperse`].
    ///
    /// ```
    /// use itertools::intersperse;
    ///
    /// itertools::assert_equal(intersperse(0..3, 8), vec![0, 8, 1, 8, 2]);
    /// ```
    template<typename I>
    intersperse::Intersperse<typename I::IntoIter> intersperse(I iterable, rusty::detail::associated_item_t<I> element) {
        return (rusty::iter(std::move(iterable))).intersperse(std::move(element));
    }

    /// Iterate `iterable` with a particular value created by a function inserted
    /// between each element.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::intersperse_with`].
    ///
    /// ```
    /// use itertools::intersperse_with;
    ///
    /// let mut i = 10;
    /// itertools::assert_equal(intersperse_with(0..3, || { i -= 1; i }), vec![0, 9, 1, 8, 2]);
    /// assert_eq!(i, 8);
    /// ```
    template<typename I, typename F>
    intersperse::IntersperseWith<typename I::IntoIter, F> intersperse_with(I iterable, F element) {
        return (rusty::iter(std::move(iterable))).intersperse_with(std::move(element));
    }

    /// Iterate `iterable` with a running index.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::enumerate`].
    ///
    /// ```
    /// use itertools::enumerate;
    ///
    /// for (i, elt) in enumerate(&[1, 2, 3]) {
    ///     /* loop body */
    ///     # let _ = (i, elt);
    /// }
    /// ```
    template<typename I>
    decltype(std::declval<typename I::IntoIter>().enumerate()) enumerate(I iterable) {
        return rusty::enumerate(rusty::iter(std::move(iterable)));
    }

    /// Iterate `iterable` in reverse.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::rev`].
    ///
    /// ```
    /// use itertools::rev;
    ///
    /// for elt in rev(&[1, 2, 3]) {
    ///     /* loop body */
    ///     # let _ = elt;
    /// }
    /// ```
    template<typename I>
    decltype(std::declval<typename I::IntoIter>().rev()) rev(I iterable) {
        return rusty::rev(rusty::iter(std::move(iterable)));
    }

    /// Converts the arguments to iterators and zips them.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::zip`].
    ///
    /// ## Example
    ///
    /// ```
    /// use itertools::zip;
    ///
    /// let mut result: Vec<(i32, char)> = Vec::new();
    ///
    /// for (a, b) in zip(&[1, 2, 3, 4, 5], &['a', 'b', 'c']) {
    ///     result.push((*a, *b));
    /// }
    /// assert_eq!(result, vec![(1, 'a'),(2, 'b'),(3, 'c')]);
    /// ```
    template<typename I, typename J>
    decltype(rusty::zip(std::declval<typename I::IntoIter>(), std::declval<typename J::IntoIter>())) zip(I i, J j) {
        return rusty::zip(rusty::iter(std::move(i)), std::move(j));
    }

    /// Takes two iterables and creates a new iterator over both in sequence.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::chain`].
    ///
    /// ## Example
    /// ```
    /// use itertools::chain;
    ///
    /// let mut result:Vec<i32> = Vec::new();
    ///
    /// for element in chain(&[1, 2, 3], &[4]) {
    ///     result.push(*element);
    /// }
    /// assert_eq!(result, vec![1, 2, 3, 4]);
    /// ```
    template<typename I, typename J>
    decltype(std::declval<typename I::IntoIter>().chain(std::declval<typename J::IntoIter>())) chain(I i, J j) {
        return rusty::chain(rusty::iter(std::move(i)), std::move(j));
    }

    /// Create an iterator that clones each element from `&T` to `T`.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::cloned`].
    ///
    /// ```
    /// use itertools::cloned;
    ///
    /// assert_eq!(cloned(b"abc").next(), Some(b'a'));
    /// ```
    template<typename I, typename T>
    decltype(std::declval<typename I::IntoIter>().cloned()) cloned(I iterable) {
        return rusty::iter(std::move(iterable)).cloned();
    }

    /// Perform a fold operation over the iterable.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::fold`].
    ///
    /// ```
    /// use itertools::fold;
    ///
    /// assert_eq!(fold(&[1., 2., 3.], 0., |a, &b| f32::max(a, b)), 3.);
    /// ```
    template<typename I, typename B, typename F>
    B fold(I iterable, B init, F f) {
        return rusty::fold(rusty::iter(std::move(iterable)), std::move(init), std::move(f));
    }

    /// Test whether the predicate holds for all elements in the iterable.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::all`].
    ///
    /// ```
    /// use itertools::all;
    ///
    /// assert!(all(&[1, 2, 3], |elt| *elt > 0));
    /// ```
    template<typename I, typename F>
    bool all(I iterable, F f) {
        return rusty::iter(std::move(iterable)).all(std::move(f));
    }

    /// Test whether the predicate holds for any elements in the iterable.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::any`].
    ///
    /// ```
    /// use itertools::any;
    ///
    /// assert!(any(&[0, -1, 2], |elt| *elt > 0));
    /// ```
    template<typename I, typename F>
    bool any(I iterable, F f) {
        return rusty::iter(std::move(iterable)).any(std::move(f));
    }

    /// Return the maximum value of the iterable.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::max`].
    ///
    /// ```
    /// use itertools::max;
    ///
    /// assert_eq!(max(0..10), Some(9));
    /// ```
    template<typename I>
    rusty::Option<rusty::detail::associated_item_t<I>> max(I iterable) {
        return rusty::iter(std::move(iterable)).max();
    }

    /// Return the minimum value of the iterable.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::min`].
    ///
    /// ```
    /// use itertools::min;
    ///
    /// assert_eq!(min(0..10), Some(0));
    /// ```
    template<typename I>
    rusty::Option<rusty::detail::associated_item_t<I>> min(I iterable) {
        return rusty::iter(std::move(iterable)).min();
    }

    /// Combine all iterator elements into one `String`, separated by `sep`.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::join`].
    ///
    /// ```
    /// use itertools::join;
    ///
    /// assert_eq!(join(&[1, 2, 3], ", "), "1, 2, 3");
    /// ```
    template<typename I>
    rusty::String join(I iterable, std::string_view sep) {
        return rusty::join(rusty::iter(std::move(iterable)), sep);
    }

    /// Sort all iterator elements into a new iterator in ascending order.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::sorted`].
    ///
    /// ```
    /// use itertools::sorted;
    /// use itertools::assert_equal;
    ///
    /// assert_equal(sorted("rust".chars()), "rstu".chars());
    /// ```
    template<typename I>
    VecIntoIter<rusty::detail::associated_item_t<I>> sorted(I iterable) {
        return rusty::iter(std::move(iterable)).sorted();
    }

    /// Sort all iterator elements into a new iterator in ascending order.
    /// This sort is unstable (i.e., may reorder equal elements).
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::sorted_unstable`].
    ///
    /// ```
    /// use itertools::sorted_unstable;
    /// use itertools::assert_equal;
    ///
    /// assert_equal(sorted_unstable("rust".chars()), "rstu".chars());
    /// ```
    template<typename I>
    VecIntoIter<rusty::detail::associated_item_t<I>> sorted_unstable(I iterable) {
        return rusty::iter(std::move(iterable)).sorted_unstable();
    }

}

namespace diff {

    template<typename I, typename J>
    struct Diff;
    template<typename I, typename J, typename F>
    rusty::Option<Diff<typename I::IntoIter, typename J::IntoIter>> diff_with(I i, J j, F is_equal);

    namespace fmt = rusty::fmt;

    using free_mod::put_back;

    using structs::PutBack;

    // Algebraic data type
    template<typename I, typename J>
    struct Diff_FirstMismatch {
        size_t _0;
        structs::PutBack<I> _1;
        structs::PutBack<J> _2;
    };
    template<typename I, typename J>
    struct Diff_Shorter {
        size_t _0;
        structs::PutBack<I> _1;
    };
    template<typename I, typename J>
    struct Diff_Longer {
        size_t _0;
        structs::PutBack<J> _1;
    };
    template<typename I, typename J>
    Diff_FirstMismatch<I, J> FirstMismatch(size_t _0, structs::PutBack<I> _1, structs::PutBack<J> _2);
    template<typename I, typename J>
    Diff_Shorter<I, J> Shorter(size_t _0, structs::PutBack<I> _1);
    template<typename I, typename J>
    Diff_Longer<I, J> Longer(size_t _0, structs::PutBack<J> _1);
    template<typename I, typename J>
    struct Diff : std::variant<Diff_FirstMismatch<I, J>, Diff_Shorter<I, J>, Diff_Longer<I, J>> {
        using variant = std::variant<Diff_FirstMismatch<I, J>, Diff_Shorter<I, J>, Diff_Longer<I, J>>;
        using variant::variant;
        static Diff<I, J> FirstMismatch(size_t _0, structs::PutBack<I> _1, structs::PutBack<J> _2) { return Diff<I, J>{Diff_FirstMismatch<I, J>{std::forward<decltype(_0)>(_0), std::forward<decltype(_1)>(_1), std::forward<decltype(_2)>(_2)}}; }
        static Diff<I, J> Shorter(size_t _0, structs::PutBack<I> _1) { return Diff<I, J>{Diff_Shorter<I, J>{std::forward<decltype(_0)>(_0), std::forward<decltype(_1)>(_1)}}; }
        static Diff<I, J> Longer(size_t _0, structs::PutBack<J> _1) { return Diff<I, J>{Diff_Longer<I, J>{std::forward<decltype(_0)>(_0), std::forward<decltype(_1)>(_1)}}; }


        rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
            return [&]() -> rusty::fmt::Result { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& idx = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& i = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); auto&& j = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._2); return f.debug_tuple("FirstMismatch").field(idx).field(i).field(j).finish(); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& idx = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& i = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return f.debug_tuple("Shorter").field(idx).field(i).finish(); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& idx = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& j = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return f.debug_tuple("Longer").field(idx).field(j).finish(); } return [&]() -> rusty::fmt::Result { rusty::intrinsics::unreachable(); }(); }();
        }
        Diff<I, J> clone() const {
            return [&]() -> Diff<I, J> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& idx = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& i = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); auto&& j = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._2); return Diff<I, J>{Diff_FirstMismatch<I, J>{rusty::detail::deref_if_pointer_like(idx), rusty::clone(i), rusty::clone(j)}}; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& idx = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& i = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return Diff<I, J>{Diff_Shorter<I, J>{rusty::detail::deref_if_pointer_like(idx), rusty::clone(i)}}; } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& idx = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); auto&& j = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._1); return Diff<I, J>{Diff_Longer<I, J>{rusty::detail::deref_if_pointer_like(idx), rusty::clone(j)}}; } return [&]() -> Diff<I, J> { rusty::intrinsics::unreachable(); }(); }();
        }
    };
    template<typename I, typename J>
    Diff_FirstMismatch<I, J> FirstMismatch(size_t _0, structs::PutBack<I> _1, structs::PutBack<J> _2) { return Diff_FirstMismatch<I, J>{std::forward<size_t>(_0), std::forward<structs::PutBack<I>>(_1), std::forward<structs::PutBack<J>>(_2)};  }
    template<typename I, typename J>
    Diff_Shorter<I, J> Shorter(size_t _0, structs::PutBack<I> _1) { return Diff_Shorter<I, J>{std::forward<size_t>(_0), std::forward<structs::PutBack<I>>(_1)};  }
    template<typename I, typename J>
    Diff_Longer<I, J> Longer(size_t _0, structs::PutBack<J> _1) { return Diff_Longer<I, J>{std::forward<size_t>(_0), std::forward<structs::PutBack<J>>(_1)};  }

    /// Compares every element yielded by both `i` and `j` with the given function in lock-step and
    /// returns a [`Diff`] which describes how `j` differs from `i`.
    ///
    /// If the number of elements yielded by `j` is less than the number of elements yielded by `i`,
    /// the number of `j` elements yielded will be returned along with `i`'s remaining elements as
    /// `Diff::Shorter`.
    ///
    /// If the two elements of a step differ, the index of those elements along with the remaining
    /// elements of both `i` and `j` are returned as `Diff::FirstMismatch`.
    ///
    /// If `i` becomes exhausted before `j` becomes exhausted, the number of elements in `i` along with
    /// the remaining `j` elements will be returned as `Diff::Longer`.
    template<typename I, typename J, typename F>
    rusty::Option<Diff<typename I::IntoIter, typename J::IntoIter>> diff_with(I i, J j, F is_equal) {
        auto i_shadow1 = rusty::iter(std::move(i));
        auto j_shadow1 = rusty::iter(std::move(j));
        auto idx = 0;
        while (true) {
            auto&& _whilelet = i_shadow1.next();
            if (!(rusty::detail::option_has_value(_whilelet))) { break; }
            auto i_elem = rusty::detail::option_take_value(_whilelet);
            {
                auto&& _m = j_shadow1.next();
                bool _m_matched = false;
                if (!_m_matched) {
                    if (_m.is_none()) {
                        return rusty::Option<Diff<typename I::IntoIter, typename J::IntoIter>>(Diff<typename I::IntoIter, typename J::IntoIter>{Diff_Shorter<typename I::IntoIter, typename J::IntoIter>{std::move(idx), adaptors::put_back(std::move(i_shadow1)).with_value(std::move(i_elem))}});
                        _m_matched = true;
                    }
                }
                if (!_m_matched) {
                    if (_m.is_some()) {
                        auto&& _mv1 = _m.unwrap();
                        auto&& j_elem = rusty::detail::deref_if_pointer(_mv1);
                        if (!is_equal(i_elem, j_elem)) {
                            auto remaining_i = adaptors::put_back(std::move(i_shadow1)).with_value(std::move(i_elem));
                            auto remaining_j = adaptors::put_back(std::move(j_shadow1)).with_value(j_elem);
                            return rusty::Option<Diff<typename I::IntoIter, typename J::IntoIter>>(Diff<typename I::IntoIter, typename J::IntoIter>{Diff_FirstMismatch<typename I::IntoIter, typename J::IntoIter>{std::move(idx), std::move(remaining_i), std::move(remaining_j)}});
                        }
                        _m_matched = true;
                    }
                }
            }
            [&]() { static_cast<void>(idx += 1); return std::make_tuple(); }();
        }
        return j_shadow1.next().map([&](auto&& j_elem) -> Diff<typename I::IntoIter, typename J::IntoIter> { return Diff<typename I::IntoIter, typename J::IntoIter>{Diff_Longer<typename I::IntoIter, typename J::IntoIter>{std::move(idx), adaptors::put_back(std::move(j_shadow1)).with_value(std::move(j_elem))}}; });
    }

}


// Algebraic data type
template<typename T>
struct FoldWhile_Continue {
    T _0;
};
template<typename T>
struct FoldWhile_Done {
    T _0;
};
template<typename T>
FoldWhile_Continue<T> Continue(T _0);
template<typename T>
FoldWhile_Done<T> Done(T _0);
template<typename T>
struct FoldWhile : std::variant<FoldWhile_Continue<T>, FoldWhile_Done<T>> {
    using variant = std::variant<FoldWhile_Continue<T>, FoldWhile_Done<T>>;
    using variant::variant;
    static FoldWhile<T> Continue(T _0) { return FoldWhile<T>{FoldWhile_Continue<T>{std::forward<decltype(_0)>(_0)}}; }
    static FoldWhile<T> Done(T _0) { return FoldWhile<T>{FoldWhile_Done<T>{std::forward<decltype(_0)>(_0)}}; }


    FoldWhile<T> clone() const {
        return [&]() -> FoldWhile<T> { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return FoldWhile<T>{FoldWhile_Continue<T>{rusty::clone(__self_0)}}; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return FoldWhile<T>{FoldWhile_Done<T>{rusty::clone(__self_0)}}; } return [&]() -> FoldWhile<T> { rusty::intrinsics::unreachable(); }(); }();
    }
    rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {
        return [&]() -> rusty::fmt::Result { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return std::conditional_t<true, rusty::fmt::Formatter, T>::debug_tuple_field1_finish(f, "Continue", __self_0); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))._0); return std::conditional_t<true, rusty::fmt::Formatter, T>::debug_tuple_field1_finish(f, "Done", __self_0); } return [&]() -> rusty::fmt::Result { rusty::intrinsics::unreachable(); }(); }();
    }
    void assert_receiver_is_total_eq() const {
    }
    bool operator==(const FoldWhile<T>& other) const {
        const auto __self_discr = rusty::intrinsics::discriminant_value((*this));
        const auto __arg1_discr = rusty::intrinsics::discriminant_value(other);
        return (__self_discr == __arg1_discr) && [&]() -> bool { auto&& _m0 = (*this); auto&& _m1 = other; if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0))._0); auto&& __arg1_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))._0); return __self_0 == __arg1_0; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0)) && std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m0))>>>(rusty::detail::deref_if_pointer(_m0))._0); auto&& __arg1_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m1))>>>(rusty::detail::deref_if_pointer(_m1))._0); return __self_0 == __arg1_0; } if (true) { return [&]() -> bool { rusty::intrinsics::unreachable(); }(); } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
    }
    T into_inner() {
        return [&]() { auto&& _m = (*this); return std::visit(overloaded { [&](auto&&) -> T { return [&]() -> T { rusty::intrinsics::unreachable(); }(); } }, std::move(_m)); }();
    }
    bool is_done() const {
        return [&]() -> bool { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return false; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return true; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
    }
};
template<typename T>
FoldWhile_Continue<T> Continue(T _0) { return FoldWhile_Continue<T>{std::forward<T>(_0)};  }
template<typename T>
FoldWhile_Done<T> Done(T _0) { return FoldWhile_Done<T>{std::forward<T>(_0)};  }

namespace concat_impl {

    template<typename I>
    rusty::detail::associated_item_t<I> concat(I iterable);

    /// Combine all an iterator's elements into one element by using [`Extend`].
    ///
    /// [`IntoIterator`]-enabled version of [`Itertools::concat`](crate::Itertools::concat).
    ///
    /// This combinator will extend the first item with each of the rest of the
    /// items of the iterator. If the iterator is empty, the default value of
    /// `I::Item` is returned.
    ///
    /// ```rust
    /// use itertools::concat;
    ///
    /// let input = vec![vec![1], vec![2, 3], vec![4, 5, 6]];
    /// assert_eq!(concat(input), vec![1, 2, 3, 4, 5, 6]);
    /// ```
    template<typename I>
    rusty::detail::associated_item_t<I> concat(I iterable) {
        return rusty::iter(std::move(iterable)).reduce([&](auto&& a, auto&& b) {
a.extend(std::move(b));
return a;
}).unwrap_or(rusty::default_value<rusty::detail::associated_item_t<I>>());
    }

}

namespace extrema_set {

    template<typename I, typename K, typename F, typename Compare>
    rusty::Vec<rusty::detail::associated_item_t<I>> min_set_impl(I it, F key_for, Compare compare);
    template<typename I, typename K, typename F, typename Compare>
    rusty::Vec<rusty::detail::associated_item_t<I>> max_set_impl(I it, F key_for, Compare compare);

    using ::rusty::Vec;

    using ::rusty::cmp::Ordering;

    /// Implementation guts for `min_set`, `min_set_by`, and `min_set_by_key`.
    template<typename I, typename K, typename F, typename Compare>
    rusty::Vec<rusty::detail::associated_item_t<I>> min_set_impl(I it, F key_for, Compare compare) {
        return [&]() -> rusty::Vec<rusty::detail::associated_item_t<I>> { auto&& _m = it.next(); if (_m.is_none()) { return rusty::Vec<rusty::detail::associated_item_t<I>>::new_(); } if (_m.is_some()) { auto&& _mv1 = _m.unwrap(); auto&& element = rusty::detail::deref_if_pointer(_mv1); return [&]() -> rusty::Vec<rusty::detail::associated_item_t<I>> { auto current_key = key_for(element);
auto result = rusty::boxed::into_vec(rusty::boxed::box_new(std::array{std::move(element)}));
it.for_each([&](auto&& element) {
const auto key = key_for(element);
switch (compare(element, result[0], key, current_key)) {
case Ordering::Less:
{
    result.clear();
    result.push(std::move(element));
    current_key = std::move(key);
    break;
}
case Ordering::Equal:
{
    result.push(std::move(element));
    break;
}
case Ordering::Greater:
{
    break;
}
}
});
return result; }(); } return [&]() -> rusty::Vec<rusty::detail::associated_item_t<I>> { rusty::intrinsics::unreachable(); }(); }();
    }

    /// Implementation guts for `ax_set`, `max_set_by`, and `max_set_by_key`.
    template<typename I, typename K, typename F, typename Compare>
    rusty::Vec<rusty::detail::associated_item_t<I>> max_set_impl(I it, F key_for, Compare compare) {
        return min_set_impl(std::move(it), std::move(key_for), [&](auto&& it1, auto&& it2, auto&& key1, auto&& key2) {
return compare(std::move(it2), std::move(it1), std::move(key2), std::move(key1));
});
    }

}

namespace group_map {

    template<typename I, typename K, typename V>
    rusty::HashMap<K, rusty::Vec<V>> into_group_map(I iter);
    template<typename I, typename K, typename V, typename F>
    rusty::HashMap<K, rusty::Vec<V>> into_group_map_by(I iter, F f);

    using ::rusty::HashMap;



    /// Return a `HashMap` of keys mapped to a list of their corresponding values.
    ///
    /// See [`.into_group_map()`](crate::Itertools::into_group_map)
    /// for more information.
    template<typename I, typename K, typename V>
    rusty::HashMap<K, rusty::Vec<V>> into_group_map(I iter) {
        auto lookup = HashMap<K, rusty::Vec<V>>::new_();
        iter.for_each([&](auto&& _destruct_param0) {
auto&& key = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param0)));
auto&& val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param0)));
lookup.entry(std::move(key)).push(std::move(val));
});
        return lookup;
    }

    template<typename I, typename K, typename V, typename F>
    rusty::HashMap<K, rusty::Vec<V>> into_group_map_by(I iter, F f) {
        return into_group_map(iter.map([&](auto&& v) { return std::make_tuple(f(v), std::move(v)); }));
    }

}

namespace k_smallest {

    template<typename I, typename F>
    rusty::Vec<rusty::detail::associated_item_t<I>> k_smallest_general(I iter, size_t k, F comparator);
    template<typename I, typename F>
    rusty::Vec<rusty::detail::associated_item_t<I>> k_smallest_relaxed_general(I iter, size_t k, F comparator);

    using ::rusty::Vec;

    using ::rusty::cmp::Ordering;

    /// Consumes a given iterator, returning the minimum elements in **ascending** order.
    template<typename I, typename F>
    rusty::Vec<rusty::detail::associated_item_t<I>> k_smallest_general(I iter, size_t k, F comparator) {
        const auto sift_down = [](auto& heap, auto& is_less_than, size_t origin) {
            const rusty::SafeFn<std::tuple<size_t, size_t>(size_t)> children_of = +[](size_t n) -> std::tuple<size_t, size_t> {
                return std::make_tuple((2 * n) + 1, (2 * n) + 2);
            };
            while (origin < rusty::len(heap)) {
                auto [left_idx, right_idx] = rusty::detail::deref_if_pointer_like(children_of(std::move(origin)));
                if (left_idx >= rusty::len(heap)) {
                    return;
                }
                const auto replacement_idx = ((right_idx < rusty::len(heap)) && is_less_than(&heap[left_idx], &heap[right_idx]) ? right_idx : left_idx);
                if (is_less_than(&heap[origin], &heap[replacement_idx])) {
                    [&]() { auto&& _swap_recv = heap; auto&& _swap_view = _swap_recv; const auto _swap_i = origin; const auto _swap_j = replacement_idx; const auto _swap_len = rusty::len(_swap_view); if (!(_swap_i < _swap_len && _swap_j < _swap_len)) { rusty::panicking::panic("index out of bounds"); } rusty::mem::swap(_swap_view[_swap_i], _swap_view[_swap_j]); }();
                    origin = std::move(replacement_idx);
                } else {
                    return;
                }
            }
        };
        if (k == static_cast<size_t>(0)) {
            iter.last();
            return rusty::Vec<rusty::detail::associated_item_t<I>>::new_();
        }
        if (k == static_cast<size_t>(1)) {
            return rusty::Vec<rusty::detail::associated_item_t<I>>::from_iter(rusty::iter(iter.min_by(std::move(comparator))));
        }
        auto iter_shadow1 = iter.fuse();
        rusty::Vec<rusty::detail::associated_item_t<I>> storage = rusty::Vec<rusty::detail::associated_item_t<I>>::from_iter(iter_shadow1.by_ref().take(std::move(k)));
        auto is_less_than = [=, comparator = std::move(comparator)](const auto& a, const auto& b) mutable { return comparator(a, b) == Ordering::Less; };
        for (auto&& i : rusty::for_in(rusty::rev((rusty::range_inclusive(0, (rusty::len(storage) / 2)))))) {
            sift_down(storage, rusty::detail::deref_if_pointer_like(is_less_than), std::move(i));
        }
        iter_shadow1.for_each([&](auto&& val) {
if (true) {
    {
        auto&& _m0_tmp = rusty::len(storage);
        auto _m0 = &_m0_tmp;
        auto _m1 = &k;
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
if (is_less_than(val, &storage[0])) {
    storage[0] = std::move(val);
    sift_down(storage, rusty::detail::deref_if_pointer_like(is_less_than), static_cast<size_t>(0));
}
});
        std::span<rusty::detail::associated_item_t<I>> heap = rusty::slice_full(storage);
        while (rusty::len(heap) > 1) {
            const auto last_idx = rusty::len(heap) - 1;
            [&]() { auto&& _swap_recv = heap; auto&& _swap_view = _swap_recv; const auto _swap_i = 0; const auto _swap_j = last_idx; const auto _swap_len = rusty::len(_swap_view); if (!(_swap_i < _swap_len && _swap_j < _swap_len)) { rusty::panicking::panic("index out of bounds"); } rusty::mem::swap(_swap_view[_swap_i], _swap_view[_swap_j]); }();
            heap = rusty::slice_to(heap, last_idx);
            sift_down(heap, rusty::detail::deref_if_pointer_like(is_less_than), static_cast<size_t>(0));
        }
        return storage;
    }

    template<typename I, typename F>
    rusty::Vec<rusty::detail::associated_item_t<I>> k_smallest_relaxed_general(I iter, size_t k, F comparator) {
        if (k == static_cast<size_t>(0)) {
            iter.last();
            return rusty::Vec<rusty::detail::associated_item_t<I>>::new_();
        }
        auto iter_shadow1 = iter.fuse();
        auto buf = rusty::collect_range(iter_shadow1.by_ref().take(2 * k));
        if (rusty::len(buf) < k) {
            buf.sort_unstable_by(comparator);
            return buf;
        }
        buf.select_nth_unstable_by(k - 1, comparator);
        buf.truncate(std::move(k));
        iter_shadow1.for_each([&](auto&& val) {
if (comparator(val, buf[k - 1]) != Ordering::Less) {
    return;
}
{
    auto&& _m0_tmp = rusty::len(buf);
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = buf.capacity();
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
buf.push(std::move(val));
if (rusty::len(buf) == (2 * k)) {
    buf.select_nth_unstable_by(k - 1, comparator);
    buf.truncate(std::move(k));
}
});
        buf.sort_unstable_by(comparator);
        buf.truncate(std::move(k));
        return buf;
    }

    template<typename T, typename K, typename F>
    const auto& key_to_cmp(F key) {
        return [=, key = std::move(key)](auto&& a, auto&& b) mutable -> rusty::cmp::Ordering { return rusty::cmp::cmp(key(std::move(a)), key(std::move(b))); };
    }

}

/// Return `true` if both iterables produce equal sequences
/// (elements pairwise equal and sequences of the same length),
/// `false` otherwise.
///
/// [`IntoIterator`] enabled version of [`Iterator::eq`].
///
/// ```
/// assert!(itertools::equal(vec![1, 2, 3], 1..4));
/// assert!(!itertools::equal(&[0, 0], &[0, 0, 0]));
/// ```
template<typename I, typename J>
bool equal(I a, J b) {
    return rusty::iter(std::move(a)).eq(std::move(b));
}

/// Assert that two iterables produce equal sequences, with the same
/// semantics as [`equal(a, b)`](equal).
///
/// **Panics** on assertion failure with a message that shows the
/// two different elements and the iteration index.
///
/// ```should_panic
/// # use itertools::assert_equal;
/// assert_equal("exceed".split('c'), "excess".split('c'));
/// // ^PANIC: panicked at 'Failed assertion Some("eed") == Some("ess") for iteration 1'.
/// ```
template<typename I, typename J>
void assert_equal(I a, J b) {
    auto ia = rusty::iter(std::move(a));
    auto ib = rusty::iter(std::move(b));
    size_t i = static_cast<size_t>(0);
    while (true) {
        {
            auto&& _m0 = ia.next();
            auto&& _m1 = ib.next();
            auto _m_tuple = std::forward_as_tuple(_m0, _m1);
            bool _m_matched = false;
            if (!_m_matched && ((std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)).is_none() && std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)).is_none()))) {
                return;
                _m_matched = true;
            }
            if (!_m_matched && (true)) {
                auto&& a = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                auto&& b = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                const auto equal_shadow1 = [&]() -> bool { auto&& _m0 = &a; auto&& _m1 = &b; if (rusty::detail::deref_if_pointer(_m0).is_some() && rusty::detail::deref_if_pointer(_m1).is_some()) { auto&& a = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m0)).unwrap()); auto&& b = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_m1)).unwrap()); return a == b; } if (true) { return false; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }();
                if (!equal_shadow1) {
                    {
                        rusty::panicking::panic_fmt(std::format("Failed assertion {1} == {2} for iteration {0}", i, rusty::to_debug_string(a), rusty::to_debug_string(b)));
                    }
                }
                [&]() { static_cast<void>(i += 1); return std::make_tuple(); }();
                _m_matched = true;
            }
        }
    }
}

/// Partition a sequence using predicate `pred` so that elements
/// that map to `true` are placed before elements which map to `false`.
///
/// The order within the partitions is arbitrary.
///
/// Return the index of the split point.
///
/// ```
/// use itertools::partition;
///
/// # // use repeated numbers to not promise any ordering
/// let mut data = [7, 1, 1, 7, 1, 1, 7];
/// let split_index = partition(&mut data, |elt| *elt >= 3);
///
/// assert_eq!(data, [7, 7, 7, 1, 1, 1, 1]);
/// assert_eq!(split_index, 3);
/// ```
template<typename A, typename I, typename F>
size_t partition(I iter, F pred) {
    auto split_index = 0;
    auto iter_shadow1 = rusty::iter(std::move(iter));
    while (true) {
        auto&& _whilelet = iter_shadow1.next();
        if (!(rusty::detail::option_has_value(_whilelet))) { break; }
        auto front = rusty::detail::option_take_value(_whilelet);
        if (!pred(std::move(front))) {
            {
                auto&& _m = iter_shadow1.rfind([&](auto&& back) { return pred(std::move(back)); });
                bool _m_matched = false;
                if (!_m_matched) {
                    if (_m.is_some()) {
                        auto&& _mv0 = _m.unwrap();
                        auto&& back = rusty::detail::deref_if_pointer(_mv0);
                        rusty::mem::swap(std::move(front), back);
                        _m_matched = true;
                    }
                }
                if (!_m_matched) {
                    if (_m.is_none()) {
                        break;
                        _m_matched = true;
                    }
                }
            }
        }
        [&]() { static_cast<void>(split_index += 1); return std::make_tuple(); }();
    }
    return split_index;
}

// Rust-only libtest main omitted

namespace duplicates_impl {
    namespace private_ {}

    namespace private_ {

    }

    /// Create a new `DuplicatesBy` iterator.
    template<typename I, typename Key, typename F>
    DuplicatesBy<I, Key, F> duplicates_by(I iter, F f) {
        return DuplicatesBy<I, Key, F>::new_(std::move(iter), ::duplicates_impl::private_::ByFn(std::move(f)));
    }

    /// Create a new `Duplicates` iterator.
    template<typename I>
    Duplicates<I> duplicates(I iter) {
        return Duplicates<I>::new_(std::move(iter), ::duplicates_impl::private_::ById{});
    }

}

namespace format {

    template<typename I, typename F>
    FormatWith<I, F> new_format(I iter, std::string_view separator, F f) {
        return FormatWith<I, F>{.sep = std::string_view(separator), .inner = rusty::Cell<rusty::Option<std::tuple<I, F>>>::new_(rusty::Option<std::tuple<I, F>>(std::make_tuple(std::move(iter), std::move(f))))};
    }

    template<typename I>
    Format<I> new_format_default(I iter, std::string_view separator) {
        return Format<I>{.sep = std::string_view(separator), .inner = rusty::Cell<rusty::Option<I>>::new_(rusty::Option<I>(std::move(iter)))};
    }

}

namespace groupbylazy {

    /// Create a new
    template<typename K, typename J, typename F>
    ChunkBy<K, typename J::IntoIter, F> new_(J iter, F f) {
        return ChunkBy<K, typename J::IntoIter, F>{.inner = rusty::RefCell<GroupInner<K, typename J::IntoIter, F>>::new_(GroupInner<K, typename J::IntoIter, F>{.key = std::move(f), .iter = rusty::iter(std::move(iter)), .current_key = rusty::Option<K>(rusty::None), .current_elt = rusty::Option<typename J::IntoIter::Item>(rusty::None), .done = false, .top_group = static_cast<size_t>(0), .oldest_buffered_group = static_cast<size_t>(0), .bottom_group = static_cast<size_t>(0), .buffer = rusty::Vec<decltype(rusty::iter(std::declval<rusty::Vec<typename J::IntoIter::Item>>()))>::new_(), .dropped_group = ~static_cast<int32_t>(0)}), .index = rusty::Cell<size_t>::new_(static_cast<size_t>(0))};
    }

    /// Create a new
    template<typename J>
    IntoChunks<typename J::IntoIter> new_chunks(J iter, size_t size) {
        return IntoChunks<typename J::IntoIter>{.inner = rusty::RefCell<GroupInner<size_t, typename J::IntoIter, ChunkIndex>>::new_(GroupInner<size_t, typename J::IntoIter, ChunkIndex>{.key = std::conditional_t<true, ChunkIndex, J>::new_(std::move(size)), .iter = rusty::iter(std::move(iter)), .current_key = rusty::Option<size_t>(rusty::None), .current_elt = rusty::Option<typename J::IntoIter::Item>(rusty::None), .done = false, .top_group = static_cast<size_t>(0), .oldest_buffered_group = static_cast<size_t>(0), .bottom_group = static_cast<size_t>(0), .buffer = rusty::Vec<decltype(rusty::iter(std::declval<rusty::Vec<typename J::IntoIter::Item>>()))>::new_(), .dropped_group = ~static_cast<int32_t>(0)}), .index = rusty::Cell<size_t>::new_(static_cast<size_t>(0))};
    }

    // Extension trait KeyFunction lowered to rusty_ext:: free functions
    namespace rusty_ext {
        template<typename A, typename K, typename F>
        K call_mut(F& self_, A arg) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return (self_)(std::move(arg));
        }

    }


}

namespace iter_index {
    namespace private_iter_index {}

    namespace private_iter_index {

        // Extension trait Sealed lowered to rusty_ext:: free functions
        namespace rusty_ext {
        }


    }

    template<typename I, typename R>
    typename R::Output get(I iter, R index) {
        return ([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::iter_index::rusty_ext::index(std::forward<decltype(__self)>(__self), rusty::iter(std::move(iter))); }) { return ::iter_index::rusty_ext::index(std::forward<decltype(__self)>(__self), rusty::iter(std::move(iter))); } else { return ::iter_index::rusty_ext::index(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self)), rusty::iter(std::move(iter))); } })(index);
    }

    // Extension trait IteratorIndex lowered to rusty_ext:: free functions
    namespace rusty_ext {
        template<typename I>
        decltype(rusty::skip(std::declval<decltype(rusty::take(std::declval<I>(), std::declval<size_t>()))>(), std::declval<size_t>())) index(rusty::range<size_t> self_, I iter) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return rusty::skip(iter.take(std::move(self_.end_)), std::move(self_.start));
        }

        template<typename I>
        decltype(rusty::take(std::declval<decltype(rusty::skip(std::declval<I>(), std::declval<size_t>()))>(), std::declval<size_t>())) index(rusty::range_inclusive<size_t> self_, I iter) {
            using Self = std::remove_reference_t<decltype(self_)>;
            const auto length = [&]() {
if (rusty::detail::deref_if_pointer_like(&(self_.end_)) == std::numeric_limits<size_t>::max()) {
{
    auto _m0_tmp = rusty::detail::deref_if_pointer_like(&(self_.start));
    auto _m0 = &_m0_tmp;
    auto&& _m1_tmp = static_cast<int32_t>(0);
    auto _m1 = &_m1_tmp;
    auto _m_tuple = std::make_tuple(_m0, _m1);
    bool _m_matched = false;
    if (!_m_matched) {
        auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
        auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
        if (rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val)) {
            const auto kind = rusty::panicking::AssertKind::Ne;
            rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
        }
        _m_matched = true;
    }
}
return (*(&(self_.end_)) - *(&(self_.start))) + 1;
} else {
return rusty::saturating_sub((*(&(self_.end_)) + 1), rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer_like(&(self_.start))));
}
}();
            return iter.skip(rusty::detail::deref_if_pointer_like(&(self_.start))).take(std::move(length));
        }

        template<typename I>
        decltype(rusty::take(std::declval<I>(), std::declval<size_t>())) index(rusty::range_to<size_t> self_, I iter) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return iter.take(std::move(self_.end));
        }

        template<typename I>
        decltype(rusty::take(std::declval<I>(), std::declval<size_t>())) index(rusty::range_to_inclusive<size_t> self_, I iter) {
            using Self = std::remove_reference_t<decltype(self_)>;
            {
                auto _m0 = &self_.end;
                auto&& _m1_tmp = std::numeric_limits<size_t>::max();
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (rusty::detail::deref_if_pointer_like(left_val) == rusty::detail::deref_if_pointer_like(right_val)) {
                        const auto kind = rusty::panicking::AssertKind::Ne;
                        rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None);
                    }
                    _m_matched = true;
                }
            }
            return iter.take(self_.end + 1);
        }

        template<typename I>
        decltype(rusty::skip(std::declval<I>(), std::declval<size_t>())) index(rusty::range_from<size_t> self_, I iter) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return iter.skip(std::move(self_.start));
        }

        template<typename I>
        I index(rusty::range_full self_, I iter) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return iter;
        }

    }


}

namespace next_array {
    namespace test {}

    namespace test {

        void zero_len_take() {
            auto builder = ArrayBuilder<std::tuple<>, 0>::new_();
            const auto taken = builder.take();
            {
                auto _m0 = &taken;
                auto&& _m1_tmp = rusty::Option<std::array<std::tuple<>, 0>>([](auto _seed) { std::array<std::tuple<>, 0> _repeat{}; _repeat.fill(_seed); return _repeat; }(std::make_tuple()));
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

        void zero_len_push() {
            auto builder = ArrayBuilder<std::tuple<>, 0>::new_();
            builder.push(std::make_tuple());
        }

        void push_4() {
            auto builder = ArrayBuilder<std::tuple<>, 4>::new_();
            {
                auto&& _m0_tmp = builder.take();
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
            builder.push(std::make_tuple());
            {
                auto&& _m0_tmp = builder.take();
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
            builder.push(std::make_tuple());
            {
                auto&& _m0_tmp = builder.take();
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
            builder.push(std::make_tuple());
            {
                auto&& _m0_tmp = builder.take();
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
            builder.push(std::make_tuple());
            {
                auto&& _m0_tmp = builder.take();
                auto _m0 = &_m0_tmp;
                auto&& _m1_tmp = rusty::Option<std::array<std::tuple<>, 4>>([](auto _seed) { std::array<std::tuple<>, 4> _repeat{}; _repeat.fill(_seed); return _repeat; }(std::make_tuple()));
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

        void tracked_drop() {
            using ::rusty::panic::catch_unwind;
            using ::rusty::panic::AssertUnwindSafe;
            using ::rusty::sync::atomic::AtomicU16;
            using ::rusty::sync::atomic::Ordering;
            static rusty::sync::atomic::AtomicU16 DROPPED = AtomicU16::new_(0);
            struct TrackedDrop {
                TrackedDrop() = default;
                TrackedDrop(const TrackedDrop&) = default;
                TrackedDrop(TrackedDrop&& other) noexcept {
                    if (rusty::mem::consume_forgotten_address(&other)) {
                        this->rusty_mark_forgotten();
                        other.rusty_mark_forgotten();
                    } else {
                        other.rusty_mark_forgotten();
                    }
                }
                TrackedDrop& operator=(const TrackedDrop&) = default;
                TrackedDrop& operator=(TrackedDrop&& other) noexcept {
                    if (this == &other) {
                        return *this;
                    }
                    this->~TrackedDrop();
                    new (this) TrackedDrop(std::move(other));
                    return *this;
                }
                void rusty_mark_forgotten() noexcept { rusty::mem::mark_forgotten_address(this); }


                ~TrackedDrop() noexcept(false) {
                    if (rusty::mem::consume_forgotten_address(this)) { return; }
                    DROPPED.fetch_add(1, Ordering::Relaxed);
                }
            };
            // Rust-only nested impl block skipped in local scope
            // Rust-only nested impl block skipped in local scope
            // Rust-only nested impl block skipped in local scope
            // Rust-only nested impl block skipped in local scope
            {
                auto builder = ArrayBuilder<TrackedDrop, 0>::new_();
                {
                    auto&& _m0_tmp = DROPPED.load(Ordering::Relaxed);
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
                rusty::mem::drop(std::move(builder));
                {
                    auto&& _m0_tmp = DROPPED.load(Ordering::Relaxed);
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
            {
                auto builder = ArrayBuilder<TrackedDrop, 2>::new_();
                builder.push(std::move(TrackedDrop{}));
                {
                    auto&& _m0_tmp = builder.take();
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
                    auto&& _m0_tmp = DROPPED.load(Ordering::Relaxed);
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
                rusty::mem::drop(std::move(builder));
                {
                    auto&& _m0_tmp = DROPPED.swap(0, Ordering::Relaxed);
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
                auto builder = ArrayBuilder<TrackedDrop, 2>::new_();
                builder.push(std::move(TrackedDrop{}));
                builder.push(std::move(TrackedDrop{}));
                if (![&]() -> bool { auto&& _m = builder.take(); if (_m.is_some()) { return true; } if (true) { return false; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }()) {
                    rusty::panicking::panic("assertion failed: matches!(builder.take(), Some(_))");
                }
                {
                    auto&& _m0_tmp = DROPPED.swap(0, Ordering::Relaxed);
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
                rusty::mem::drop(std::move(builder));
                {
                    auto&& _m0_tmp = DROPPED.load(Ordering::Relaxed);
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
            {
                auto builder = ArrayBuilder<TrackedDrop, 2>::new_();
                builder.push(std::move(TrackedDrop{}));
                builder.push(std::move(TrackedDrop{}));
                if (!catch_unwind(AssertUnwindSafe([&]() {
builder.push(std::move(TrackedDrop{}));
})).is_err()) {
                    rusty::panicking::panic("assertion failed: catch_unwind(AssertUnwindSafe(|| { builder.push(TrackedDrop); })).is_err()");
                }
                {
                    auto&& _m0_tmp = DROPPED.load(Ordering::Relaxed);
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
                rusty::mem::drop(std::move(builder));
                {
                    auto&& _m0_tmp = DROPPED.swap(0, Ordering::Relaxed);
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
                auto builder = ArrayBuilder<TrackedDrop, 2>::new_();
                builder.push(std::move(TrackedDrop{}));
                builder.push(std::move(TrackedDrop{}));
                if (!catch_unwind(AssertUnwindSafe([&]() {
builder.push(std::move(TrackedDrop{}));
})).is_err()) {
                    rusty::panicking::panic("assertion failed: catch_unwind(AssertUnwindSafe(|| { builder.push(TrackedDrop); })).is_err()");
                }
                {
                    auto&& _m0_tmp = DROPPED.load(Ordering::Relaxed);
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
                if (![&]() -> bool { auto&& _m = builder.take(); if (_m.is_some()) { return true; } if (true) { return false; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }()) {
                    rusty::panicking::panic("assertion failed: matches!(builder.take(), Some(_))");
                }
                {
                    auto&& _m0_tmp = DROPPED.load(Ordering::Relaxed);
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
                builder.push(std::move(TrackedDrop{}));
                builder.push(std::move(TrackedDrop{}));
                if (![&]() -> bool { auto&& _m = builder.take(); if (_m.is_some()) { return true; } if (true) { return false; } return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }()) {
                    rusty::panicking::panic("assertion failed: matches!(builder.take(), Some(_))");
                }
                {
                    auto&& _m0_tmp = DROPPED.swap(0, Ordering::Relaxed);
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

    }

    /// Assuming all the elements are initialized, get a mutable slice to them.
    ///
    /// # Safety
    ///
    /// The caller guarantees that the elements `T` referenced by `slice` are in a
    /// valid state.
    // @unsafe
    template<typename T>
    std::span<T> slice_assume_init_mut(std::span<rusty::MaybeUninit<T>> slice) {
        // @unsafe
        {
            return *(const_cast<std::add_pointer_t<std::span<T>>>(reinterpret_cast<std::add_pointer_t<std::add_const_t<std::span<T>>>>(static_cast<std::add_pointer_t<std::span<rusty::MaybeUninit<T>>>>(&slice))));
        }
    }

    /// Equivalent to `it.next_array()`.
    template<typename I, size_t N>
    rusty::Option<std::array<rusty::detail::associated_item_t<I>, rusty::sanitize_array_capacity<N>()>> next_array(I& it) {
        auto builder = ArrayBuilder<rusty::detail::associated_item_t<I>, N>::new_();
        for (auto&& _ : rusty::for_in(rusty::range(0, N))) {
            builder.push(RUSTY_TRY_OPT(it.next()));
        }
        return builder.take();
    }

}

namespace peeking_take_while {

    /// Create a `PeekingTakeWhile`
    template<typename I, typename F>
    PeekingTakeWhile<I, F> peeking_take_while(I& iter, F f) {
        return PeekingTakeWhile<I, F>{.iter = iter, .f = std::move(f)};
    }

    // Extension trait PeekingNext lowered to rusty_ext:: free functions
    namespace rusty_ext {
        template<typename F, typename I>
        rusty::Option<typename decltype(std::declval<I>().peekable())::Item> peeking_next(decltype(std::declval<I>().peekable())& self_, F accept) {
            using Self = std::remove_reference_t<decltype(self_)>;
            if (auto&& _iflet_scrutinee = self_.peek(); _iflet_scrutinee.is_some()) {
                decltype(auto) r = _iflet_scrutinee.unwrap();
                if (!accept(std::move(r))) {
                    return rusty::None;
                }
            }
            return self_.next();
        }

        template<typename F, typename T>
        rusty::Option<typename rusty::slice_iter::Iter<const T>::Item> peeking_next(rusty::slice_iter::Iter<const T>& self_, F accept) {
            using Self = std::remove_reference_t<decltype(self_)>;
            const auto saved_state = rusty::clone(self_);
            if (auto&& _iflet_scrutinee = self_.next(); _iflet_scrutinee.is_some()) {
                decltype(auto) r = _iflet_scrutinee.unwrap();
                if (!accept(&r)) {
                    self_ = std::move(saved_state);
                } else {
                    return rusty::Option<typename Self::Item>(std::move(r));
                }
            }
            return rusty::None;
        }

        // Rust-only extension method skipped (unsupported self type mapping): peeking_next (::std::str::Chars)

        // Rust-only extension method skipped (unsupported self type mapping): peeking_next (::std::str::CharIndices)

        // Rust-only extension method skipped (unsupported self type mapping): peeking_next (::std::str::Bytes)

        // Rust-only extension method skipped (unsupported self type mapping): peeking_next (::std::option::Iter<T>)

        // Rust-only extension method skipped (unsupported self type mapping): peeking_next (::std::result::Iter<T>)

        template<typename F, typename T>
        rusty::Option<typename rusty::empty_iter<T>::Item> peeking_next(rusty::empty_iter<T>& self_, F accept) {
            using Self = std::remove_reference_t<decltype(self_)>;
            const auto saved_state = rusty::clone(self_);
            if (auto&& _iflet_scrutinee = self_.next(); _iflet_scrutinee.is_some()) {
                decltype(auto) r = _iflet_scrutinee.unwrap();
                if (!accept(&r)) {
                    self_ = std::move(saved_state);
                } else {
                    return rusty::Option<typename Self::Item>(std::move(r));
                }
            }
            return rusty::None;
        }

        // Rust-only extension method skipped (unsupported self type mapping): peeking_next (rusty::collections::linked_list::Iter<T>)

        // Rust-only extension method skipped (unsupported self type mapping): peeking_next (rusty::collections::vec_deque::Iter<T>)

        template<typename F, typename I>
        rusty::Option<typename decltype(std::declval<I>().rev())::Item> peeking_next(decltype(std::declval<I>().rev())& self_, F accept) {
            using Self = std::remove_reference_t<decltype(self_)>;
            const auto saved_state = rusty::clone(self_);
            if (auto&& _iflet_scrutinee = self_.next(); _iflet_scrutinee.is_some()) {
                decltype(auto) r = _iflet_scrutinee.unwrap();
                if (!accept(&r)) {
                    self_ = std::move(saved_state);
                } else {
                    return rusty::Option<typename Self::Item>(std::move(r));
                }
            }
            return rusty::None;
        }

    }


}

namespace process_results_impl {

    /// “Lift” a function of the values of an iterator so that it can process
    /// an iterator of `Result` values instead.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::process_results`].
    template<typename I, typename F, typename T, typename E, typename R>
    rusty::Result<R, E> process_results(I iterable, F processor) {
        auto iter = rusty::iter(std::move(iterable));
        auto error = rusty::Result<std::tuple<>, std::tuple<>>::Ok(std::make_tuple());
        const auto result = processor(ProcessResults<I, std::tuple<>>{.error = error, .iter = std::move(iter)});
        return error.map([&](auto _closure_wild0) { return result; });
    }

}

namespace rciter_impl {

    /// Return an iterator inside a `Rc<RefCell<_>>` wrapper.
    ///
    /// The returned `RcIter` can be cloned, and each clone will refer back to the
    /// same original iterator.
    ///
    /// `RcIter` allows doing interesting things like using `.zip()` on an iterator with
    /// itself, at the cost of runtime borrow checking which may have a performance
    /// penalty.
    ///
    /// Iterator element type is `Self::Item`.
    ///
    /// ```
    /// use itertools::rciter;
    /// use itertools::zip;
    ///
    /// // In this example a range iterator is created and we iterate it using
    /// // three separate handles (two of them given to zip).
    /// // We also use the IntoIterator implementation for `&RcIter`.
    ///
    /// let mut iter = rciter(0..9);
    /// let mut z = zip(&iter, &iter);
    ///
    /// assert_eq!(z.next(), Some((0, 1)));
    /// assert_eq!(z.next(), Some((2, 3)));
    /// assert_eq!(z.next(), Some((4, 5)));
    /// assert_eq!(iter.next(), Some(6));
    /// assert_eq!(z.next(), Some((7, 8)));
    /// assert_eq!(z.next(), None);
    /// ```
    ///
    /// **Panics** in iterator methods if a borrow error is encountered in the
    /// iterator methods. It can only happen if the `RcIter` is reentered in
    /// `.next()`, i.e. if it somehow participates in an “iterator knot”
    /// where it is an adaptor of itself.
    template<typename I>
    RcIter<typename I::IntoIter> rciter(I iterable) {
        return RcIter<I>{.rciter = rusty::Rc<rusty::RefCell<typename I::IntoIter>>::new_(rusty::RefCell<typename I::IntoIter>::new_(rusty::iter(std::move(iterable))))};
    }

}

namespace repeatn {

    /// Create an iterator that produces `n` repetitions of `element`.
    template<typename A>
    RepeatN<A> repeat_n(A element, size_t n) {
        if (n == static_cast<size_t>(0)) {
            return RepeatN<A>{.elt = rusty::Option<A>(rusty::None), .n = std::move(n)};
        } else {
            return RepeatN<A>{.elt = rusty::Option<A>(std::move(element)), .n = std::move(n)};
        }
    }

}

namespace adaptors {
    namespace coalesce_tests {}
    namespace map {}
    namespace multi_product {}

    namespace coalesce_tests {

        /// Create a new `Coalesce`.
        template<typename I, typename F>
        Coalesce<I, F> coalesce(I iter, F f) {
            return Coalesce<I, F>{.last = rusty::None, .iter = std::move(iter), .f = std::move(f)};
        }

        /// Create a new `DedupBy`.
        template<typename I, typename Pred>
        DedupBy<I, Pred> dedup_by(I iter, Pred dedup_pred) {
            return DedupBy<I, Pred>{.last = rusty::None, .iter = std::move(iter), .f = DedupPred2CoalescePred(std::move(dedup_pred))};
        }

        /// Create a new `Dedup`.
        template<typename I>
        Dedup<I> dedup(I iter) {
            return ::adaptors::coalesce_tests::dedup_by(std::move(iter), DedupEq{});
        }

        /// Create a new `DedupByWithCount`.
        template<typename I, typename Pred>
        DedupByWithCount<I, Pred> dedup_by_with_count(I iter, Pred dedup_pred) {
            return DedupByWithCount<I, Pred>{.last = rusty::None, .iter = std::move(iter), .f = DedupPredWithCount2CoalescePred(std::move(dedup_pred))};
        }

        /// Create a new `DedupWithCount`.
        template<typename I>
        DedupWithCount<I> dedup_with_count(I iter) {
            return ::adaptors::coalesce_tests::dedup_by_with_count(std::move(iter), DedupEq{});
        }

        // Extension trait CoalescePredicate lowered to rusty_ext:: free functions
        namespace rusty_ext {
            template<typename F, typename Item, typename T>
            rusty::Result<T, std::tuple<T, T>> coalesce_pair(F& self_, T t, Item item) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return self_(std::move(t), std::move(item));
            }

        }

        // Extension trait DedupPredicate lowered to rusty_ext:: free functions
        namespace rusty_ext {
            template<typename T, typename F>
            bool dedup_pair(F& self_, const T& a, const T& b) {
                using Self = std::remove_reference_t<decltype(self_)>;
                return self_(a, b);
            }

        }


    }

    namespace map {

        /// Create a new `MapOk` iterator.
        template<typename I, typename F, typename T, typename U, typename E>
        MapOk<I, F> map_ok(I iter, F f) {
            return MapSpecialCase<I, MapSpecialCaseFnOk<F>>{.iter = std::move(iter), .f = MapSpecialCaseFnOk(std::move(f))};
        }

        /// Create a new [`MapInto`] iterator.
        template<typename I, typename R>
        MapInto<I, R> map_into(I iter) {
            return MapSpecialCase<I, MapSpecialCaseFnInto<R>>{.iter = std::move(iter), .f = MapSpecialCaseFnInto(rusty::PhantomData<R>{})};
        }

    }

    namespace multi_product {

        /// Create a new cartesian product iterator over an arbitrary number
        /// of iterators of the same type.
        ///
        /// Iterator element is of type `Vec<H::Item::Item>`.
        template<typename H>
        MultiProduct<typename rusty::detail::associated_item_t<H>::IntoIter> multi_cartesian_product(H iters) {
            auto inner = MultiProductInner{.iters = rusty::collect_range(iters.map([&](auto&& i) { return MultiProductIter<std::remove_cvref_t<decltype(rusty::iter(std::move(i)))>>::new_(rusty::iter(std::move(i))); })), .cur = rusty::None};
            return MultiProduct(rusty::Option<MultiProductInner<typename rusty::detail::associated_item_t<H>::IntoIter>>(std::move(inner)));
        }

    }

    /// Create an iterator that interleaves elements in `i` and `j`.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::interleave`](crate::Itertools::interleave).
    template<typename I, typename J>
    Interleave<typename I::IntoIter, typename J::IntoIter> interleave(I i, J j) {
        return Interleave<I, J>{.i = rusty::iter(std::move(i)).fuse(), .j = rusty::iter(std::move(j)).fuse(), .next_coming_from_j = false};
    }

    /// Create a new `InterleaveShortest` iterator.
    template<typename I, typename J>
    InterleaveShortest<I, J> interleave_shortest(I i, J j) {
        return InterleaveShortest<I, J>{.i = std::move(i), .j = std::move(j), .next_coming_from_j = false};
    }

    /// Create an iterator where you can put back a single item
    template<typename I>
    PutBack<typename I::IntoIter> put_back(I iterable) {
        return PutBack<I>{.top = rusty::Option<typename I::IntoIter::Item>(rusty::None), .iter = rusty::iter(std::move(iterable))};
    }

    /// Create a new cartesian product iterator
    ///
    /// Iterator element type is `(I::Item, J::Item)`.
    template<typename I, typename J>
    Product<I, J> cartesian_product(I i, J j) {
        return Product<I, J>{.a = std::move(i), .a_cur = rusty::Option<rusty::Option<rusty::detail::associated_item_t<I>>>(rusty::None), .b = rusty::clone(j), .b_orig = std::move(j)};
    }

    /// Create a new Batching iterator.
    template<typename I, typename F>
    Batching<I, F> batching(I iter, F f) {
        return Batching<I, F>{.f = std::move(f), .iter = std::move(iter)};
    }

    /// Create a new `TakeWhileRef` from a reference to clonable iterator.
    template<typename I, typename F>
    TakeWhileRef<I, F> take_while_ref(I& iter, F f) {
        return TakeWhileRef<I, F>{.iter = iter, .f = std::move(f)};
    }

    /// Create a new `WhileSome<I>`.
    template<typename I>
    WhileSome<I> while_some(I iter) {
        return WhileSome<I>{.iter = std::move(iter)};
    }

    /// Create a new `TupleCombinations` from a clonable iterator.
    template<typename T, typename I>
    TupleCombinations<I, T> tuple_combinations(I iter) {
        return TupleCombinations<I, T>{.iter = T::Combination::from(std::move(iter)), ._mi = rusty::PhantomData<I>{}};
    }

    rusty::Option<size_t> checked_binomial(size_t n, size_t k) {
        if (n < k) {
            return rusty::Option<size_t>(static_cast<size_t>(0));
        }
        k = rusty::min((n - k), std::move(k));
        size_t c = static_cast<size_t>(1);
        for (auto&& i : rusty::for_in(rusty::range_inclusive(1, k))) {
            c = RUSTY_TRY_OPT([&]() { auto&& _checked_lhs = RUSTY_TRY_OPT([&]() { auto&& _checked_lhs = (c / i); return rusty::checked_mul(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(std::move(n))); }()); return rusty::checked_add(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(RUSTY_TRY_OPT([&]() { auto&& _checked_lhs = (c % i); return rusty::checked_mul(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(std::move(n))); }()) / i)); }());
            [&]() { static_cast<void>(n -= 1); return std::make_tuple(); }();
        }
        return rusty::Option<size_t>(std::move(c));
    }

    void test_checked_binomial() {
        constexpr size_t LIMIT = static_cast<size_t>(500);
        auto row = rusty::array_repeat(rusty::Option<size_t>(static_cast<size_t>(0)), LIMIT + 1);
        row[0] = rusty::Option<size_t>(static_cast<size_t>(1));
        for (auto&& n : rusty::for_in(rusty::range_inclusive(0, LIMIT))) {
            for (auto&& k : rusty::for_in(rusty::range_inclusive(0, LIMIT))) {
                {
                    auto _m0 = rusty::as_ref_ptr(row[k]);
                    auto&& _m1_tmp = checked_binomial(std::move(n), std::move(k));
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
            row = rusty::Vec<rusty::Option<size_t>>::from_iter(rusty::chain(rusty::once(rusty::Option<size_t>(static_cast<size_t>(1))), rusty::map((rusty::range_inclusive(1, LIMIT)), [&](auto&& k) { return [&]() { auto&& _checked_lhs = RUSTY_TRY(row[k - 1]); return rusty::checked_add(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(RUSTY_TRY(row[k]))); }(); })));
        }
    }

    /// Create a new `FilterOk` iterator.
    template<typename I, typename F, typename T, typename E>
    FilterOk<I, F> filter_ok(I iter, F f) {
        return FilterOk<I, F>{.iter = std::move(iter), .f = std::move(f)};
    }

    template<typename T, typename E>
    rusty::Option<rusty::Result<T, E>> transpose_result(rusty::Result<rusty::Option<T>, E> result) {
        return [&]() -> rusty::Option<rusty::Result<T, E>> { auto&& _m = result; if (_m.is_ok()) { auto&& _mv0 = std::as_const(_m).unwrap(); if (rusty::detail::deref_if_pointer(_mv0).is_some()) { auto&& v = rusty::detail::deref_if_pointer(std::as_const(rusty::detail::deref_if_pointer(_mv0)).unwrap()); return rusty::Option<rusty::Result<T, E>>(rusty::Result<T, E>::Ok(v)); } } if (_m.is_ok()) { return _m.unwrap(); } if (_m.is_err()) { auto&& _mv2 = _m.unwrap_err(); auto&& e = rusty::detail::deref_if_pointer(_mv2); return rusty::Option<rusty::Result<T, E>>(rusty::Result<T, E>::Err(std::move(e))); } return [&]() -> rusty::Option<rusty::Result<T, E>> { rusty::intrinsics::unreachable(); }(); }();
    }

    /// Create a new `FilterOk` iterator.
    template<typename I, typename F, typename T, typename U, typename E>
    FilterMapOk<I, F> filter_map_ok(I iter, F f) {
        return FilterMapOk<I, F>{.iter = std::move(iter), .f = std::move(f)};
    }

    /// Create a new `Positions` iterator.
    template<typename I, typename F>
    Positions<I, F> positions(I iter, F f) {
        auto iter_shadow1 = iter.enumerate();
        return Positions<I, F>{.iter = std::move(iter_shadow1), .f = std::move(f)};
    }

    /// Create a new `Update` iterator.
    template<typename I, typename F>
    Update<I, F> update(I iter, F f) {
        return Update<I, F>{.iter = std::move(iter), .f = std::move(f)};
    }

}

namespace cons_tuples_impl {

    /// Create an iterator that maps for example iterators of
    /// `((A, B), C)` to `(A, B, C)`.
    template<typename I>
    ConsTuples<typename I::IntoIter> cons_tuples(I iterable) {
        return ConsTuples<I>{.iter = rusty::iter(std::move(iterable)), .f = ConsTuplesFn{}};
    }

}

namespace flatten_ok {

    template<typename I, typename T, typename E>
    FlattenOk<I, T, E> flatten_ok(I iter) {
        return FlattenOk<I, T, E>{.iter = std::move(iter), .inner_front = rusty::Option<typename T::IntoIter>(rusty::None), .inner_back = rusty::Option<typename T::IntoIter>(rusty::None)};
    }

}

namespace grouping_map {

    template<typename K, typename I, typename F>
    MapForGrouping<I, F> new_map_for_grouping(I iter, F key_mapper) {
        return adaptors::map::MapSpecialCase<I, GroupingMapFn<F>>{.iter = std::move(iter), .f = GroupingMapFn(std::move(key_mapper))};
    }

    /// Creates a new `GroupingMap` from `iter`
    template<typename I, typename K, typename V>
    GroupingMap<I> new_(I iter) {
        return GroupingMap<I>{.iter = std::move(iter)};
    }

}

namespace intersperse_tests {

    /// Create a new Intersperse iterator
    template<typename I>
    Intersperse<I> intersperse(I iter, rusty::detail::associated_item_t<I> elt) {
        return ::intersperse_tests::intersperse_with(std::move(iter), IntersperseElementSimple(std::move(elt)));
    }

    /// Create a new `IntersperseWith` iterator
    template<typename I, typename ElemF>
    IntersperseWith<I, ElemF> intersperse_with(I iter, ElemF elt) {
        return IntersperseWith<I, ElemF>{.element = std::move(elt), .iter = iter.fuse(), .peek = rusty::Option<rusty::Option<rusty::detail::associated_item_t<I>>>(rusty::None)};
    }

    // Extension trait IntersperseElement lowered to rusty_ext:: free functions
    namespace rusty_ext {
        template<typename Item, typename F>
        Item generate(F& self_) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return self_();
        }

    }


}

namespace kmerge_impl {

    /// Make `data` a heap (min-heap w.r.t the sorting).
    template<typename T, typename S>
    void heapify(std::span<T> data, S less_than) {
        for (auto&& i : rusty::for_in(rusty::rev((rusty::range(0, rusty::len(data) / 2))))) {
            sift_down(data, rusty::detail::deref_if_pointer_like(i), less_than);
        }
    }

    /// Sift down element at `index` (`heap` is a min-heap wrt the ordering)
    template<typename T, typename S>
    void sift_down(std::span<T> heap, size_t index, S less_than) {
        if (true) {
            if (!(index <= rusty::len(heap))) {
                rusty::panicking::panic("assertion failed: index <= heap.len()");
            }
        }
        auto pos = std::move(index);
        auto child = (2 * pos) + 1;
        while ((child + 1) < rusty::len(heap)) {
            [&]() { static_cast<void>(child += static_cast<size_t>(less_than(heap[child + 1], heap[child]))); return std::make_tuple(); }();
            if (!less_than(heap[child], heap[pos])) {
                return;
            }
            [&]() { auto&& _swap_recv = heap; auto&& _swap_view = _swap_recv; const auto _swap_i = pos; const auto _swap_j = child; const auto _swap_len = rusty::len(_swap_view); if (!(_swap_i < _swap_len && _swap_j < _swap_len)) { rusty::panicking::panic("index out of bounds"); } rusty::mem::swap(_swap_view[_swap_i], _swap_view[_swap_j]); }();
            pos = std::move(child);
            child = (2 * pos) + 1;
        }
        if (((child + 1) == rusty::len(heap)) && less_than(heap[child], heap[pos])) {
            [&]() { auto&& _swap_recv = heap; auto&& _swap_view = _swap_recv; const auto _swap_i = pos; const auto _swap_j = child; const auto _swap_len = rusty::len(_swap_view); if (!(_swap_i < _swap_len && _swap_j < _swap_len)) { rusty::panicking::panic("index out of bounds"); } rusty::mem::swap(_swap_view[_swap_i], _swap_view[_swap_j]); }();
        }
    }

    /// Create an iterator that merges elements of the contained iterators using
    /// the ordering function.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::kmerge`](crate::Itertools::kmerge).
    ///
    /// ```
    /// use itertools::kmerge;
    ///
    /// for elt in kmerge(vec![vec![0, 2, 4], vec![1, 3, 5], vec![6, 7]]) {
    ///     /* loop body */
    ///     # let _ = elt;
    /// }
    /// ```
    template<typename I>
    KMerge<typename rusty::detail::associated_item_t<I>::IntoIter> kmerge(I iterable) {
        return kmerge_by(std::move(iterable), KMergeByLt{});
    }

    /// Create an iterator that merges elements of the contained iterators.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::kmerge_by`](crate::Itertools::kmerge_by).
    template<typename I, typename F>
    KMergeBy<typename rusty::detail::associated_item_t<I>::IntoIter, F> kmerge_by(I iterable, F less_than) {
        auto iter = rusty::iter(std::move(iterable));
        auto [lower, _tuple_ignore1] = rusty::detail::deref_if_pointer_like(rusty::size_hint(iter));
        rusty::Vec<HeadTail<I>> heap = rusty::Vec<HeadTail<I>>::with_capacity(std::move(lower));
        heap.extend(rusty::filter_map(iter, [&](auto&& it) { return HeadTail<std::remove_cvref_t<decltype(rusty::iter(std::move(it)))>>::new_(rusty::iter(std::move(it))); }));
        heapify(heap, [&](auto&& a, auto&& b) { return ([&](auto&& __self) -> decltype(auto) { if constexpr (requires { ::kmerge_impl::rusty_ext::kmerge_pred(std::forward<decltype(__self)>(__self), rusty::detail::deref_if_pointer_like(a.head), rusty::detail::deref_if_pointer_like(b.head)); }) { return ::kmerge_impl::rusty_ext::kmerge_pred(std::forward<decltype(__self)>(__self), rusty::detail::deref_if_pointer_like(a.head), rusty::detail::deref_if_pointer_like(b.head)); } else { return ::kmerge_impl::rusty_ext::kmerge_pred(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self)), rusty::detail::deref_if_pointer_like(a.head), rusty::detail::deref_if_pointer_like(b.head)); } })(less_than); });
        return KMergeBy<I, F>{.heap = std::move(heap), .less_than = std::move(less_than)};
    }

    // Extension trait KMergePredicate lowered to rusty_ext:: free functions
    namespace rusty_ext {
        template<typename T, typename F>
        bool kmerge_pred(F& self_, const T& a, const T& b) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return self_(a, b);
        }

    }


}

namespace combinations {

    /// Create a new `Combinations` from a clonable iterator.
    template<typename I>
    Combinations<I> combinations(I iter, size_t k) {
        return Combinations<I>::new_(std::move(iter), rusty::collect_range((rusty::range(0, k))));
    }

    /// Create a new `ArrayCombinations` from a clonable iterator.
    template<typename I, size_t K>
    ArrayCombinations<I, K> array_combinations(I iter) {
        return ArrayCombinations<I, K>::new_(std::move(iter), rusty::array_from_fn<rusty::sanitize_array_capacity<K>()>([&](auto&& i) { return i; }));
    }

    /// For a given size `n`, return the count of remaining combinations or None if it would overflow.
    rusty::Option<size_t> remaining_for(size_t n, bool first, std::span<const size_t> indices) {
        auto k = rusty::len(indices);
        if (n < k) {
            return rusty::Option<size_t>(static_cast<size_t>(0));
        } else if (first) {
            return adaptors::checked_binomial(std::move(n), std::move(k));
        } else {
            return rusty::try_fold(rusty::enumerate(rusty::iter(indices)), static_cast<size_t>(0), [&](auto&& sum, auto&& _destruct_param1) {
auto&& i = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param1)));
auto&& n0 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param1)));
return [&]() { auto&& _checked_lhs = sum; return rusty::checked_add(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(RUSTY_TRY(adaptors::checked_binomial((n - 1) - rusty::detail::deref_if_pointer_like(n0), k - i)))); }();
});
        }
    }

    // Extension trait PoolIndex lowered to rusty_ext:: free functions
    namespace rusty_ext {
        template<typename I, typename T>
        rusty::Vec<T> extract_item(const rusty::Vec<size_t>& self_, const lazy_buffer::LazyBuffer<I>& pool) {
            using Self = std::remove_reference_t<decltype(self_)>;
            return pool.get_at(self_);
        }

    }


}

namespace combinations_with_replacement {

    /// Create a new `CombinationsWithReplacement` from a clonable iterator.
    template<typename I>
    CombinationsWithReplacement<I> combinations_with_replacement(I iter, size_t k) {
        auto indices = rusty::into_boxed_slice(rusty::array_repeat(0, std::move(k)));
        lazy_buffer::LazyBuffer<I> pool = lazy_buffer::LazyBuffer<I>::new_(std::move(iter));
        return CombinationsWithReplacement<I>{.indices = std::move(indices), .pool = std::move(pool), .first = true};
    }

    /// For a given size `n`, return the count of remaining combinations with replacement or None if it would overflow.
    rusty::Option<size_t> remaining_for(size_t n, bool first, std::span<const size_t> indices) {
        const auto count = [&](size_t n, size_t k) {
std::optional<std::remove_cvref_t<decltype((([&]() { auto&& _checked_lhs = (n - 1); return rusty::checked_add(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(std::move(k))); }()).unwrap()))>> _iflet_value0;
{
    if (n == static_cast<size_t>(0)) {
        _iflet_value0.emplace(rusty::saturating_sub(k, rusty::detail::deref_if_pointer(1)));
    } else { _iflet_value0.emplace(RUSTY_TRY([&]() { auto&& _checked_lhs = (n - 1); return rusty::checked_add(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(std::move(k))); }())); }
}
auto positions_shadow1 = std::move(_iflet_value0).value();
return adaptors::checked_binomial(std::move(positions_shadow1), std::move(k));
};
        auto k = rusty::len(indices);
        if (first) {
            return count(std::move(n), std::move(k));
        } else {
            return rusty::try_fold(rusty::enumerate(rusty::iter(indices)), static_cast<size_t>(0), [&](auto&& sum, auto&& _destruct_param1) {
auto&& i = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param1)));
auto&& n0 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param1)));
return [&]() { auto&& _checked_lhs = sum; return rusty::checked_add(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(RUSTY_TRY(count((n - 1) - rusty::detail::deref_if_pointer_like(n0), k - i)))); }();
});
        }
    }

}

namespace merge_join {

    /// Create an iterator that merges elements in `i` and `j`.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::merge`](crate::Itertools::merge).
    ///
    /// ```
    /// use itertools::merge;
    ///
    /// for elt in merge(&[1, 2, 3], &[2, 3, 4]) {
    ///     /* loop body */
    ///     # let _ = elt;
    /// }
    /// ```
    template<typename I, typename J>
    Merge<typename I::IntoIter, typename J::IntoIter> merge(I i, J j) {
        return merge_by_new(std::move(i), std::move(j), MergeLte{});
    }

    /// Create a `MergeBy` iterator.
    template<typename I, typename J, typename F>
    MergeBy<typename I::IntoIter, typename J::IntoIter, F> merge_by_new(I a, J b, F cmp) {
        return MergeBy<I, J, F>{.left = adaptors::put_back(rusty::iter(std::move(a)).fuse()), .right = adaptors::put_back(rusty::iter(std::move(b)).fuse()), .cmp_fn = std::move(cmp)};
    }

    /// Return an iterator adaptor that merge-joins items from the two base iterators in ascending order.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::merge_join_by`].
    template<typename I, typename J, typename F, typename T>
    MergeJoinBy<typename I::IntoIter, typename J::IntoIter, F> merge_join_by(I left, J right, F cmp_fn) {
        return MergeBy<I, J, F>{.left = adaptors::put_back(rusty::iter(std::move(left)).fuse()), .right = adaptors::put_back(rusty::iter(std::move(right)).fuse()), .cmp_fn = MergeFuncLR(std::move(cmp_fn), rusty::PhantomData<T>{})};
    }

    // Extension trait FuncLR lowered to rusty_ext:: free functions
    namespace rusty_ext {
    }

    // Extension trait OrderingOrBool lowered to rusty_ext:: free functions
    namespace rusty_ext {
        // Rust-only extension method skipped (no receiver): left

        // Rust-only extension method skipped (no receiver): right

        template<typename T, typename F>
        std::tuple<rusty::Option<rusty::Either<T, T>>, T> merge(F& self_, T left, T right) {
            using Self = std::remove_reference_t<decltype(self_)>;
            if (self_(left, right)) {
                return std::make_tuple(rusty::Option<rusty::Either<T, T>>(rusty::either::Right<T, T>(T(std::move(right)))), std::move(left));
            } else {
                return std::make_tuple(rusty::Option<rusty::Either<T, T>>(rusty::either::Left<T, T>(T(std::move(left)))), std::move(right));
            }
        }

        // Rust-only extension method skipped (no receiver): size_hint

    }


}

namespace multipeek_impl {

    /// An iterator adaptor that allows the user to peek at multiple `.next()`
    /// values without advancing the base iterator.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::multipeek`].
    template<typename I>
    MultiPeek<typename I::IntoIter> multipeek(I iterable) {
        return MultiPeek<I>{.iter = rusty::iter(std::move(iterable)).fuse(), .buf = rusty::VecDeque<typename I::IntoIter::Item>::new_(), .index = static_cast<size_t>(0)};
    }

}

namespace pad_tail {

    /// Create a new `PadUsing` iterator.
    template<typename I, typename F>
    PadUsing<I, F> pad_using(I iter, size_t min, F filler) {
        return PadUsing<I, F>{.iter = iter.fuse(), .min = std::move(min), .pos = static_cast<size_t>(0), .filler = std::move(filler)};
    }

}

namespace peek_nth {

    /// A drop-in replacement for [`std::iter::Peekable`] which adds a `peek_nth`
    /// method allowing the user to `peek` at a value several iterations forward
    /// without advancing the base iterator.
    ///
    /// This differs from `multipeek` in that subsequent calls to `peek` or
    /// `peek_nth` will always return the same value until `next` is called
    /// (making `reset_peek` unnecessary).
    template<typename I>
    PeekNth<typename I::IntoIter> peek_nth(I iterable) {
        return PeekNth<I>{.iter = rusty::iter(std::move(iterable)).fuse(), .buf = rusty::VecDeque<typename I::IntoIter::Item>::new_()};
    }

}

namespace permutations {

    template<typename I>
    Permutations<I> permutations(I iter, size_t k) {
        return Permutations<I>{.vals = lazy_buffer::LazyBuffer<I>::new_(std::move(iter)), .state = PermutationState{PermutationState_Start{.k = std::move(k)}}};
    }

    bool advance(std::span<size_t> indices, std::span<size_t> cycles) {
        const auto n = rusty::len(indices);
        const auto k = rusty::len(cycles);
        for (auto&& i : rusty::for_in(rusty::rev((rusty::range(0, k))))) {
            if (cycles[i] == static_cast<size_t>(0)) {
                cycles[i] = (n - i) - 1;
                rusty::rotate_left(rusty::slice_from(indices, i), 1);
            } else {
                const auto swap_index = n - cycles[i];
                [&]() { auto&& _swap_recv = indices; auto&& _swap_view = _swap_recv; const auto _swap_i = i; const auto _swap_j = swap_index; const auto _swap_len = rusty::len(_swap_view); if (!(_swap_i < _swap_len && _swap_j < _swap_len)) { rusty::panicking::panic("index out of bounds"); } rusty::mem::swap(_swap_view[_swap_i], _swap_view[_swap_j]); }();
                [&]() { static_cast<void>(cycles[i] -= 1); return std::make_tuple(); }();
                return false;
            }
        }
        return true;
    }

}

namespace powerset {

    /// Create a new `Powerset` from a clonable iterator.
    template<typename I>
    Powerset<I> powerset(I src) {
        return Powerset<I>{.combs = combinations::combinations(std::move(src), static_cast<size_t>(0))};
    }

    rusty::Option<size_t> remaining_for(size_t n, size_t k) {
        return rusty::try_fold((rusty::range_inclusive(k + 1, n)), static_cast<size_t>(0), [&](auto&& sum, auto&& i) { return [&]() { auto&& _checked_lhs = sum; return rusty::checked_add(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(RUSTY_TRY(adaptors::checked_binomial(std::move(n), std::move(i))))); }(); });
    }

}

namespace put_back_n_impl {

    /// Create an iterator where you can put back multiple values to the front
    /// of the iteration.
    ///
    /// Iterator element type is `I::Item`.
    template<typename I>
    PutBackN<typename I::IntoIter> put_back_n(I iterable) {
        return PutBackN<I>{.top = rusty::Vec<typename I::IntoIter::Item>::new_(), .iter = rusty::iter(std::move(iterable))};
    }

}

namespace sources {

    /// Creates a new unfold source with the specified closure as the "iterator
    /// function" and an initial state to eventually pass to the closure
    ///
    /// `unfold` is a general iterator builder: it has a mutable state value,
    /// and a closure with access to the state that produces the next value.
    ///
    /// This more or less equivalent to a regular struct with an [`Iterator`]
    /// implementation, and is useful for one-off iterators.
    ///
    /// ```
    /// // an iterator that yields sequential Fibonacci numbers,
    /// // and stops at the maximum representable value.
    ///
    /// use itertools::unfold;
    ///
    /// let mut fibonacci = unfold((1u32, 1u32), |(x1, x2)| {
    ///     // Attempt to get the next Fibonacci number
    ///     let next = x1.saturating_add(*x2);
    ///
    ///     // Shift left: ret <- x1 <- x2 <- next
    ///     let ret = *x1;
    ///     *x1 = *x2;
    ///     *x2 = next;
    ///
    ///     // If addition has saturated at the maximum, we are finished
    ///     if ret == *x1 && ret > 1 {
    ///         None
    ///     } else {
    ///         Some(ret)
    ///     }
    /// });
    ///
    /// itertools::assert_equal(fibonacci.by_ref().take(8),
    ///                         vec![1, 1, 2, 3, 5, 8, 13, 21]);
    /// assert_eq!(fibonacci.last(), Some(2_971_215_073))
    /// ```
    template<typename A, typename St, typename F>
    Unfold<St, F> unfold(St initial_state, F f) {
        return Unfold<St, F>{.f = std::move(f), .state = std::move(initial_state)};
    }

    /// Creates a new iterator that infinitely applies function to value and yields results.
    ///
    /// ```
    /// use itertools::iterate;
    ///
    /// itertools::assert_equal(iterate(1, |i| i % 3 + 1).take(5), vec![1, 2, 3, 1, 2]);
    /// ```
    ///
    /// **Panics** if compute the next value does.
    ///
    /// ```should_panic
    /// # use itertools::iterate;
    /// let mut it = iterate(25u32, |x| x - 10).take_while(|&x| x > 10);
    /// assert_eq!(it.next(), Some(25)); // `Iterate` holds 15.
    /// assert_eq!(it.next(), Some(15)); // `Iterate` holds 5.
    /// it.next(); // `5 - 10` overflows.
    /// ```
    ///
    /// You can alternatively use [`core::iter::successors`] as it better describes a finite iterator.
    template<typename St, typename F>
    Iterate<St, F> iterate(St initial_value, F f) {
        return Iterate<St, F>{.state = std::move(initial_value), .f = std::move(f)};
    }

}

namespace tee {

    template<typename I>
    std::tuple<Tee<I>, Tee<I>> new_(I iter) {
        auto buffer = TeeBuffer<rusty::detail::associated_item_t<I>, I>{.backlog = rusty::VecDeque<rusty::detail::associated_item_t<I>>::new_(), .iter = std::move(iter), .owner = false};
        auto t1 = Tee<I>{.rcbuffer = rusty::Rc<rusty::RefCell<TeeBuffer<rusty::detail::associated_item_t<I>, I>>>::new_(rusty::RefCell<TeeBuffer<rusty::detail::associated_item_t<I>, I>>::new_(std::move(buffer))), .id = true};
        auto t2 = Tee<I>{.rcbuffer = rusty::clone(t1.rcbuffer), .id = false};
        return std::make_tuple(std::move(t1), std::move(t2));
    }

}

namespace tuple_impl {

    /// Create a new tuples iterator.
    template<typename I, typename T>
    Tuples<I, T> tuples(I iter) {
        return Tuples<I, T>{.iter = iter.fuse(), .buf = rusty::default_value<typename T::Buffer>()};
    }

    /// `(n + a) / d` avoiding overflow when possible, returns `None` if it overflows.
    rusty::Option<size_t> add_then_div(size_t n, size_t a, size_t d) {
        if (true) {
            {
                auto _m0 = &d;
                auto&& _m1_tmp = static_cast<size_t>(0);
                auto _m1 = &_m1_tmp;
                auto _m_tuple = std::make_tuple(_m0, _m1);
                bool _m_matched = false;
                if (!_m_matched) {
                    auto&& left_val = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m_tuple)));
                    auto&& right_val = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m_tuple)));
                    if (left_val == right_val) {
                        const auto kind = rusty::panicking::AssertKind::Ne;
                        [&]() -> rusty::Option<size_t> { rusty::panicking::assert_failed(std::move(kind), left_val, right_val, rusty::None); }();
                    }
                    _m_matched = true;
                }
            }
        }
        return [&]() { auto&& _checked_lhs = RUSTY_TRY_OPT([&]() { auto&& _checked_lhs = (n / d); return rusty::checked_add(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(a / d)); }()); return rusty::checked_add(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>((((n % d) + (a % d))) / d)); }();
    }

    /// Create a new tuple windows iterator.
    template<typename I, typename T>
    TupleWindows<I, T> tuple_windows(I iter) {
        return TupleWindows<I, T>{.iter = std::move(iter), .last = rusty::Option<T>(rusty::None)};
    }

    template<typename I, typename T>
    CircularTupleWindows<I, T> circular_tuple_windows(I iter) {
        auto len = rusty::len(iter);
        auto iter_shadow1 = tuple_windows(iter.cycle());
        return CircularTupleWindows<I, T>{.iter = std::move(iter_shadow1), .len = std::move(len)};
    }

    // Extension trait HomogeneousTuple lowered to rusty_ext:: free functions
    namespace rusty_ext {
    }


}

namespace unique_impl {

    /// Create a new `UniqueBy` iterator.
    template<typename I, typename V, typename F>
    UniqueBy<I, V, F> unique_by(I iter, F f) {
        return UniqueBy<I, V, F>{.iter = std::move(iter), .used = rusty::HashMap<V, std::tuple<>>(), .f = std::move(f)};
    }

    template<typename I, typename K>
    size_t count_new_keys(rusty::HashMap<K, std::tuple<>> used, I iterable) {
        auto iter = rusty::iter(std::move(iterable));
        const auto current_used = rusty::len(used);
        used.extend(iter.map([&](auto&& key) { return std::make_tuple(std::move(key), std::make_tuple()); }));
        return rusty::len(used) - current_used;
    }

    template<typename I>
    Unique<I> unique(I iter) {
        return Unique<I>{.iter = UniqueBy<I, rusty::detail::associated_item_t<I>, std::tuple<>>{.iter = std::move(iter), .used = rusty::HashMap<rusty::detail::associated_item_t<I>, std::tuple<>>(), .f = std::make_tuple()}};
    }

}

namespace with_position {

    /// Create a new `WithPosition` iterator.
    template<typename I>
    WithPosition<I> with_position(I iter) {
        return WithPosition<I>{.handled_first = false, .peekable = iter.fuse().peekable()};
    }

}

namespace zip_eq_impl {

    /// Zips two iterators but **panics** if they are not of the same length.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::zip_eq`](crate::Itertools::zip_eq).
    ///
    /// ```
    /// use itertools::zip_eq;
    ///
    /// let data = [1, 2, 3, 4, 5];
    /// for (a, b) in zip_eq(&data[..data.len() - 1], &data[1..]) {
    ///     /* loop body */
    ///     # let _ = (a, b);
    /// }
    /// ```
    template<typename I, typename J>
    ZipEq<typename I::IntoIter, typename J::IntoIter> zip_eq(I i, J j) {
        return ZipEq<I, J>{.a = rusty::iter(std::move(i)), .b = rusty::iter(std::move(j))};
    }

}

namespace zip_longest {

    /// Create a new `ZipLongest` iterator.
    template<typename T, typename U>
    ZipLongest<T, U> zip_longest(T a, U b) {
        return ZipLongest<T, U>{.a = a.fuse(), .b = b.fuse()};
    }

}

namespace ziptuple {

    /// An iterator that generalizes `.zip()` and allows running multiple iterators in lockstep.
    ///
    /// The iterator `Zip<(I, J, ..., M)>` is formed from a tuple of iterators (or values that
    /// implement [`IntoIterator`]) and yields elements
    /// until any of the subiterators yields `None`.
    ///
    /// The iterator element type is a tuple like like `(A, B, ..., E)` where `A` to `E` are the
    /// element types of the subiterator.
    ///
    /// **Note:** The result of this function is a value of a named type (`Zip<(I, J,
    /// ..)>` of each component iterator `I, J, ...`) if each component iterator is
    /// nameable.
    ///
    /// Prefer [`izip!()`](crate::izip) over `multizip` for the performance benefits of using the
    /// standard library `.zip()`. Prefer `multizip` if a nameable type is needed.
    ///
    /// ```
    /// use itertools::multizip;
    ///
    /// // iterate over three sequences side-by-side
    /// let mut results = [0, 0, 0, 0];
    /// let inputs = [3, 7, 9, 6];
    ///
    /// for (r, index, input) in multizip((&mut results, 0..10, &inputs)) {
    ///     *r = index * 10 + input;
    /// }
    ///
    /// assert_eq!(results, [0 + 3, 10 + 7, 29, 36]);
    /// ```
    template<typename T, typename U>
    Zip<T> multizip(U t) {
        return Zip<T>::from(std::move(t));
    }

}


namespace duplicates_impl::private_ {
        rusty::fmt::Result ById::fmt(rusty::fmt::Formatter& f) const {
            return f.write_str("ById");
        }
}

namespace duplicates_impl::private_ {
        ById ById::clone() const {
            return ById{};
        }
}

namespace duplicates_impl::private_ {
        template<typename V>
        auto ById::make(V v) {
            return JustValue<V>::JustValue(std::move(v));
        }
}

namespace groupbylazy {
    rusty::fmt::Result ChunkIndex::fmt(rusty::fmt::Formatter& f) const {
        using Key = typename ChunkIndex::Key;
        return rusty::fmt::Formatter::debug_struct_field3_finish(f, "ChunkIndex", "size", &this->size, "index", &this->index, "key", &this->key);
    }
}

namespace groupbylazy {
    ChunkIndex ChunkIndex::clone() const {
        using Key = typename ChunkIndex::Key;
        return ChunkIndex{.size = rusty::clone(this->size), .index = rusty::clone(this->index), .key = rusty::clone(this->key)};
    }
}

namespace groupbylazy {
    ChunkIndex ChunkIndex::new_(size_t size) {
        using Key = typename ChunkIndex::Key;
        return ChunkIndex{.size = std::move(size), .index = static_cast<size_t>(0), .key = static_cast<size_t>(0)};
    }
}

namespace groupbylazy {
    template<typename A>
    size_t ChunkIndex::call_mut(A _arg) {
        using Key = typename ChunkIndex::Key;
        if (this->index == this->size) {
            [&]() { static_cast<void>(this->key += 1); return std::make_tuple(); }();
            this->index = static_cast<size_t>(0);
        }
        [&]() { static_cast<void>(this->index += 1); return std::make_tuple(); }();
        return this->key;
    }
}

namespace adaptors::coalesce_tests {
        template<typename T>
        T NoCount::new_(T t) {
            return t;
        }
}

namespace adaptors::coalesce_tests {
        template<typename T>
        std::tuple<size_t, T> WithCount::new_(T t) {
            return std::make_tuple(static_cast<size_t>(1), std::move(t));
        }
}

namespace adaptors::coalesce_tests {
        DedupEq DedupEq::clone() const {
            return DedupEq{};
        }
}

namespace adaptors::coalesce_tests {
        rusty::fmt::Result DedupEq::fmt(rusty::fmt::Formatter& f) const {
            return f.write_str("DedupEq");
        }
}

namespace adaptors::coalesce_tests {
        template<typename T>
        bool DedupEq::dedup_pair(const T& a, const T& b) {
            return a == b;
        }
}

namespace cons_tuples_impl {
    template<typename K, typename L, typename X>
    auto ConsTuplesFn::call(std::tuple<std::tuple<K, L>, X> _arg1) {
        auto&& K_shadow1 = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& L_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& X_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_arg1)));
        return std::make_tuple(K_shadow1, L_shadow1, X_shadow1);
    }
}

namespace cons_tuples_impl {
    template<typename J, typename K, typename L, typename X>
    auto ConsTuplesFn::call(std::tuple<std::tuple<J, K, L>, X> _arg1) {
        auto&& J_shadow1 = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& K_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& L_shadow1 = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& X_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_arg1)));
        return std::make_tuple(J_shadow1, K_shadow1, L_shadow1, X_shadow1);
    }
}

namespace cons_tuples_impl {
    template<typename I, typename J, typename K, typename L, typename X>
    auto ConsTuplesFn::call(std::tuple<std::tuple<I, J, K, L>, X> _arg1) {
        auto&& I_shadow1 = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& J_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& K_shadow1 = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& L_shadow1 = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& X_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_arg1)));
        return std::make_tuple(I_shadow1, J_shadow1, K_shadow1, L_shadow1, X_shadow1);
    }
}

namespace cons_tuples_impl {
    template<typename H, typename I, typename J, typename K, typename L, typename X>
    auto ConsTuplesFn::call(std::tuple<std::tuple<H, I, J, K, L>, X> _arg1) {
        auto&& H_shadow1 = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& I_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& J_shadow1 = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& K_shadow1 = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& L_shadow1 = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& X_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_arg1)));
        return std::make_tuple(H_shadow1, I_shadow1, J_shadow1, K_shadow1, L_shadow1, X_shadow1);
    }
}

namespace cons_tuples_impl {
    template<typename G, typename H, typename I, typename J, typename K, typename L, typename X>
    auto ConsTuplesFn::call(std::tuple<std::tuple<G, H, I, J, K, L>, X> _arg1) {
        auto&& G_shadow1 = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& H_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& I_shadow1 = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& J_shadow1 = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& K_shadow1 = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& L_shadow1 = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& X_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_arg1)));
        return std::make_tuple(G_shadow1, H_shadow1, I_shadow1, J_shadow1, K_shadow1, L_shadow1, X_shadow1);
    }
}

namespace cons_tuples_impl {
    template<typename F, typename G, typename H, typename I, typename J, typename K, typename L, typename X>
    auto ConsTuplesFn::call(std::tuple<std::tuple<F, G, H, I, J, K, L>, X> _arg1) {
        auto&& F_shadow1 = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& G_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& H_shadow1 = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& I_shadow1 = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& J_shadow1 = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& K_shadow1 = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& L_shadow1 = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& X_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_arg1)));
        return std::make_tuple(F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1, K_shadow1, L_shadow1, X_shadow1);
    }
}

namespace cons_tuples_impl {
    template<typename E, typename F, typename G, typename H, typename I, typename J, typename K, typename L, typename X>
    auto ConsTuplesFn::call(std::tuple<std::tuple<E, F, G, H, I, J, K, L>, X> _arg1) {
        auto&& E_shadow1 = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& F_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& G_shadow1 = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& H_shadow1 = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& I_shadow1 = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& J_shadow1 = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& K_shadow1 = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& L_shadow1 = rusty::detail::deref_if_pointer(std::get<7>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& X_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_arg1)));
        return std::make_tuple(E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1, K_shadow1, L_shadow1, X_shadow1);
    }
}

namespace cons_tuples_impl {
    template<typename D, typename E, typename F, typename G, typename H, typename I, typename J, typename K, typename L, typename X>
    auto ConsTuplesFn::call(std::tuple<std::tuple<D, E, F, G, H, I, J, K, L>, X> _arg1) {
        auto&& D_shadow1 = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& E_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& F_shadow1 = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& G_shadow1 = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& H_shadow1 = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& I_shadow1 = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& J_shadow1 = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& K_shadow1 = rusty::detail::deref_if_pointer(std::get<7>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& L_shadow1 = rusty::detail::deref_if_pointer(std::get<8>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& X_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_arg1)));
        return std::make_tuple(D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1, K_shadow1, L_shadow1, X_shadow1);
    }
}

namespace cons_tuples_impl {
    template<typename C, typename D, typename E, typename F, typename G, typename H, typename I, typename J, typename K, typename L, typename X>
    auto ConsTuplesFn::call(std::tuple<std::tuple<C, D, E, F, G, H, I, J, K, L>, X> _arg1) {
        auto&& C_shadow1 = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& D_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& E_shadow1 = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& F_shadow1 = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& G_shadow1 = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& H_shadow1 = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& I_shadow1 = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& J_shadow1 = rusty::detail::deref_if_pointer(std::get<7>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& K_shadow1 = rusty::detail::deref_if_pointer(std::get<8>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& L_shadow1 = rusty::detail::deref_if_pointer(std::get<9>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& X_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_arg1)));
        return std::make_tuple(C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1, K_shadow1, L_shadow1, X_shadow1);
    }
}

namespace cons_tuples_impl {
    template<typename B, typename C, typename D, typename E, typename F, typename G, typename H, typename I, typename J, typename K, typename L, typename X>
    auto ConsTuplesFn::call(std::tuple<std::tuple<B, C, D, E, F, G, H, I, J, K, L>, X> _arg1) {
        auto&& B_shadow1 = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& C_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& D_shadow1 = rusty::detail::deref_if_pointer(std::get<2>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& E_shadow1 = rusty::detail::deref_if_pointer(std::get<3>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& F_shadow1 = rusty::detail::deref_if_pointer(std::get<4>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& G_shadow1 = rusty::detail::deref_if_pointer(std::get<5>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& H_shadow1 = rusty::detail::deref_if_pointer(std::get<6>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& I_shadow1 = rusty::detail::deref_if_pointer(std::get<7>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& J_shadow1 = rusty::detail::deref_if_pointer(std::get<8>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& K_shadow1 = rusty::detail::deref_if_pointer(std::get<9>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& L_shadow1 = rusty::detail::deref_if_pointer(std::get<10>(rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_arg1)))));
        auto&& X_shadow1 = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_arg1)));
        return std::make_tuple(B_shadow1, C_shadow1, D_shadow1, E_shadow1, F_shadow1, G_shadow1, H_shadow1, I_shadow1, J_shadow1, K_shadow1, L_shadow1, X_shadow1);
    }
}

namespace cons_tuples_impl {
    rusty::fmt::Result ConsTuplesFn::fmt(rusty::fmt::Formatter& f) const {
        return f.write_str("ConsTuplesFn");
    }
}

namespace cons_tuples_impl {
    ConsTuplesFn ConsTuplesFn::clone() const {
        return ConsTuplesFn{};
    }
}

namespace kmerge_impl {
    KMergeByLt KMergeByLt::clone() const {
        return KMergeByLt{};
    }
}

namespace kmerge_impl {
    rusty::fmt::Result KMergeByLt::fmt(rusty::fmt::Formatter& f) const {
        return f.write_str("KMergeByLt");
    }
}

namespace kmerge_impl {
    template<typename T>
    bool KMergeByLt::kmerge_pred(const T& a, const T& b) {
        return a < b;
    }
}

namespace merge_join {
    MergeLte MergeLte::clone() const {
        return MergeLte{};
    }
}

namespace merge_join {
    rusty::fmt::Result MergeLte::fmt(rusty::fmt::Formatter& f) const {
        return f.write_str("MergeLte");
    }
}

namespace merge_join {
    template<typename T>
    auto MergeLte::left(T left) {
        return left;
    }
}

namespace merge_join {
    template<typename T>
    auto MergeLte::right(T right) {
        return right;
    }
}

namespace merge_join {
    template<typename T>
    auto MergeLte::merge(T left, T right) {
        if (left <= right) {
            return std::make_tuple(rusty::Option<rusty::Either<T, T>>(rusty::either::Right<T, T>(T(std::move(right)))), std::move(left));
        } else {
            return std::make_tuple(rusty::Option<rusty::Either<T, T>>(rusty::either::Left<T, T>(T(std::move(left)))), std::move(right));
        }
    }
}

namespace merge_join {
    template<typename T>
    size_hint::SizeHint MergeLte::size_hint(size_hint::SizeHint left, size_hint::SizeHint right) {
        return size_hint::add(std::move(left), std::move(right));
    }
}

namespace permutations {
    PermutationState PermutationState::clone() const {
        return [&]() -> PermutationState { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m)) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).k); return PermutationState{PermutationState_Start{.k = rusty::clone(__self_0)}}; } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m)) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).k); auto&& __self_1 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).min_n); return PermutationState{PermutationState_Buffered{.k = rusty::clone(__self_0), .min_n = rusty::clone(__self_1)}}; } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m)) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).indices); auto&& __self_1 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).cycles); return PermutationState{PermutationState_Loaded{.indices = rusty::clone(__self_0), .cycles = rusty::clone(__self_1)}}; } if (std::holds_alternative<std::variant_alternative_t<3, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return PermutationState{PermutationState_End{}}; } return [&]() -> PermutationState { rusty::intrinsics::unreachable(); }(); }();
    }
}

namespace permutations {
    rusty::fmt::Result PermutationState::fmt(rusty::fmt::Formatter& f) const {
        return [&]() -> rusty::fmt::Result { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m)) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).k); return rusty::fmt::Formatter::debug_struct_field1_finish(f, "Start", "k", __self_0); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m)) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).k); auto&& __self_1 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).min_n); return rusty::fmt::Formatter::debug_struct_field2_finish(f, "Buffered", "k", __self_0, "min_n", __self_1); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m)) { auto&& __self_0 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).indices); auto&& __self_1 = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).cycles); return rusty::fmt::Formatter::debug_struct_field2_finish(f, "Loaded", "indices", __self_0, "cycles", __self_1); } if (std::holds_alternative<std::variant_alternative_t<3, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return f.write_str("End"); } return [&]() -> rusty::fmt::Result { rusty::intrinsics::unreachable(); }(); }();
    }
}

namespace permutations {
    size_hint::SizeHint PermutationState::size_hint_for(size_t n) const {
        const auto at_start = [&](auto&& n, auto&& k) {
if (true) {
    if (!(n >= k)) {
        rusty::panicking::panic("assertion failed: n >= k");
    }
}
auto total = rusty::try_fold((rusty::range_inclusive((n - k) + 1, n)), static_cast<size_t>(1), [&](auto&& acc, auto&& i) { return [&]() { auto&& _checked_lhs = acc; return rusty::checked_mul(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(std::move(i))); }(); });
return std::make_tuple(total.unwrap_or(std::numeric_limits<size_t>::max()), std::move(total));
};
        return [&]() -> size_hint::SizeHint { auto&& _m = (*this); if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m)) { auto&& k = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).k); if (n < k) { return std::make_tuple(static_cast<size_t>(0), rusty::Option<size_t>(static_cast<size_t>(0))); } } if (std::holds_alternative<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m)) { auto&& k = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<0, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).k); return at_start(std::move(n), k); } if (std::holds_alternative<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m)) { auto&& k = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).k); auto&& min_n = rusty::detail::deref_if_pointer(std::get<std::variant_alternative_t<1, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).min_n); return size_hint::sub_scalar(at_start(std::move(n), k), (min_n - k) + 1); } if (std::holds_alternative<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m)) { const auto& indices = std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).indices; const auto& cycles = std::get<std::variant_alternative_t<2, rusty::detail::variant_underlying_type_t<decltype(_m)>>>(_m).cycles; return [&]() -> size_hint::SizeHint { auto count = rusty::try_fold(rusty::enumerate(rusty::iter(cycles)), static_cast<size_t>(0), [&](auto&& acc, auto&& _destruct_param1) {
auto&& i = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_destruct_param1)));
auto&& c = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_destruct_param1))));
return [&]() { auto&& _checked_lhs = acc; return rusty::checked_mul(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(rusty::len(indices) - i)); }().and_then([&](auto&& count) { return [&]() { auto&& _checked_lhs = count; return rusty::checked_add(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>(std::move(c))); }(); });
});
return std::make_tuple(count.unwrap_or(std::numeric_limits<size_t>::max()), std::move(count)); }(); } if (std::holds_alternative<std::variant_alternative_t<3, rusty::detail::variant_underlying_type_t<decltype(rusty::detail::deref_if_pointer(_m))>>>(rusty::detail::deref_if_pointer(_m))) { return std::make_tuple(static_cast<size_t>(0), rusty::Option<size_t>(static_cast<size_t>(0))); } return [&]() -> size_hint::SizeHint { rusty::intrinsics::unreachable(); }(); }();
    }
}

// Extension trait Itertools lowered to rusty_ext:: free functions
namespace rusty_ext {
}



// Runnable wrappers for expanded Rust test bodies
// Rust-only libtest wrapper metadata: marker=next_array::test::zero_len_take should_panic=no
void rusty_test_next_array_test_zero_len_take() {
    next_array::test::zero_len_take();
}
// Rust-only libtest wrapper metadata: marker=next_array::test::zero_len_push should_panic=yes
void rusty_test_next_array_test_zero_len_push() {
    next_array::test::zero_len_push();
}
// Rust-only libtest wrapper metadata: marker=next_array::test::push_4 should_panic=no
void rusty_test_next_array_test_push_4() {
    next_array::test::push_4();
}
// Rust-only libtest wrapper metadata: marker=next_array::test::tracked_drop should_panic=no
void rusty_test_next_array_test_tracked_drop() {
    next_array::test::tracked_drop();
}
// Rust-only libtest wrapper metadata: marker=size_hint::mul_size_hints should_panic=no
void rusty_test_size_hint_mul_size_hints() {
    size_hint::mul_size_hints();
}
// Rust-only libtest wrapper metadata: marker=adaptors::test_checked_binomial should_panic=no
void rusty_test_adaptors_test_checked_binomial() {
    adaptors::test_checked_binomial();
}


// ── Test runner ──
int main(int argc, char** argv) {
    if (argc == 3 && std::string(argv[1]) == "--rusty-single-test") {
        const std::string test_name = argv[2];
        rusty::mem::clear_all_forgotten_addresses();
        try {
            if (test_name == "rusty_test_adaptors_test_checked_binomial") { rusty_test_adaptors_test_checked_binomial(); return 0; }
            if (test_name == "rusty_test_next_array_test_push_4") { rusty_test_next_array_test_push_4(); return 0; }
            if (test_name == "rusty_test_next_array_test_tracked_drop") { rusty_test_next_array_test_tracked_drop(); return 0; }
            if (test_name == "rusty_test_next_array_test_zero_len_push") { rusty_test_next_array_test_zero_len_push(); return 0; }
            if (test_name == "rusty_test_next_array_test_zero_len_take") { rusty_test_next_array_test_zero_len_take(); return 0; }
            if (test_name == "rusty_test_size_hint_mul_size_hints") { rusty_test_size_hint_mul_size_hints(); return 0; }
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
    try { rusty_test_adaptors_test_checked_binomial(); std::cout << "  adaptors_test_checked_binomial PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  adaptors_test_checked_binomial FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  adaptors_test_checked_binomial FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_next_array_test_push_4(); std::cout << "  next_array_test_push_4 PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  next_array_test_push_4 FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  next_array_test_push_4 FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_next_array_test_tracked_drop(); std::cout << "  next_array_test_tracked_drop PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  next_array_test_tracked_drop FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  next_array_test_tracked_drop FAILED (unknown exception)" << std::endl; fail++; }
    {
        const std::string cmd = std::string("\"") + argv[0] + "\" --rusty-single-test rusty_test_next_array_test_zero_len_push";
        const int status = std::system(cmd.c_str());
        if (status != 0) { std::cout << "  next_array_test_zero_len_push PASSED (expected panic)" << std::endl; pass++; }
        else { std::cerr << "  next_array_test_zero_len_push FAILED: expected panic" << std::endl; fail++; }
    }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_next_array_test_zero_len_take(); std::cout << "  next_array_test_zero_len_take PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  next_array_test_zero_len_take FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  next_array_test_zero_len_take FAILED (unknown exception)" << std::endl; fail++; }
    rusty::mem::clear_all_forgotten_addresses();
    try { rusty_test_size_hint_mul_size_hints(); std::cout << "  size_hint_mul_size_hints PASSED" << std::endl; pass++; }
    catch (const std::exception& e) { std::cerr << "  size_hint_mul_size_hints FAILED: " << e.what() << std::endl; fail++; }
    catch (...) { std::cerr << "  size_hint_mul_size_hints FAILED (unknown exception)" << std::endl; fail++; }
    std::cout << std::endl;
    std::cout << "Results: " << pass << " passed, " << fail << " failed" << std::endl;
    return fail > 0 ? 1 : 0;
}
