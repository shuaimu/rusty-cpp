#ifndef RUSTY_ARRAY_HPP
#define RUSTY_ARRAY_HPP

#include <array>
#include <vector>
#include <cstddef>
#include <algorithm>
#include <iterator>
#include <type_traits>
#include <utility>
#include <optional>
#include <cstring>
#include <limits>
#include <span>
#include <stdexcept>
#include <tuple>
#include <variant>
#include <rusty/vec.hpp>
#include <rusty/maybe_uninit.hpp>

// GCC/libstdc++ C++23 does not provide span equality operators.
// Keep a narrow value-comparison overload so transpiled Rust slice assertions compile.
template<typename L, std::size_t LExtent, typename R, std::size_t RExtent>
constexpr bool operator==(std::span<L, LExtent> lhs, std::span<R, RExtent> rhs) {
    return lhs.size() == rhs.size() && std::equal(lhs.begin(), lhs.end(), rhs.begin());
}

template<typename L, std::size_t LExtent, typename R, std::size_t N>
constexpr bool operator==(std::span<L, LExtent> lhs, const std::array<R, N>& rhs) {
    return lhs.size() == rhs.size() && std::equal(lhs.begin(), lhs.end(), rhs.begin());
}

template<typename L, std::size_t N, typename R, std::size_t RExtent>
constexpr bool operator==(const std::array<L, N>& lhs, std::span<R, RExtent> rhs) {
    return rhs == lhs;
}

// Vec/slice assertion scaffolding often compares owned buffers (`std::vector`)
// with borrowed slice views (`std::span`). Mirror Rust slice equality semantics
// for these mixed container/view shapes.
template<typename L, typename Alloc, typename R, std::size_t RExtent>
requires (
    requires(const L& l, const R& r) { l == r; } ||
    requires(const L& l, const R& r) { r == l; } ||
    (std::is_empty_v<std::remove_cv_t<L>> && std::is_empty_v<std::remove_cv_t<R>>))
constexpr bool operator==(const std::vector<L, Alloc>& lhs, std::span<R, RExtent> rhs) {
    if (lhs.size() != rhs.size()) {
        return false;
    }
    if constexpr (requires(const L& l, const R& r) { l == r; }) {
        return std::equal(
            lhs.begin(),
            lhs.end(),
            rhs.begin(),
            [](const L& l, const R& r) { return static_cast<bool>(l == r); });
    } else if constexpr (requires(const L& l, const R& r) { r == l; }) {
        return std::equal(
            lhs.begin(),
            lhs.end(),
            rhs.begin(),
            [](const L& l, const R& r) { return static_cast<bool>(r == l); });
    } else {
        return true;
    }
}

template<typename L, std::size_t LExtent, typename R, typename Alloc>
requires (
    requires(const L& l, const R& r) { l == r; } ||
    requires(const L& l, const R& r) { r == l; } ||
    (std::is_empty_v<std::remove_cv_t<L>> && std::is_empty_v<std::remove_cv_t<R>>))
constexpr bool operator==(std::span<L, LExtent> lhs, const std::vector<R, Alloc>& rhs) {
    return rhs == lhs;
}

// Mixed-element std::array equality for transpiled assertion scaffolding.
// Keep this narrow: only for different element types and only when one-sided
// element equality is well-formed.
template<typename L, std::size_t N, typename R>
requires (
    !std::is_same_v<std::remove_cv_t<L>, std::remove_cv_t<R>> &&
    (requires(const L& l, const R& r) { l == r; } ||
     requires(const L& l, const R& r) { r == l; }))
constexpr bool operator==(const std::array<L, N>& lhs, const std::array<R, N>& rhs) {
    if constexpr (requires(const L& l, const R& r) { l == r; }) {
        return std::equal(
            lhs.begin(),
            lhs.end(),
            rhs.begin(),
            [](const L& l, const R& r) { return static_cast<bool>(l == r); });
    } else {
        return std::equal(
            lhs.begin(),
            lhs.end(),
            rhs.begin(),
            [](const L& l, const R& r) { return static_cast<bool>(r == l); });
    }
}

namespace rusty {

namespace detail {
template<typename T>
inline constexpr bool collect_range_dependent_false_v = false;

template<typename Index>
size_t checked_index(Index idx) {
    if constexpr (std::is_signed_v<Index>) {
        if (idx < 0) {
            throw std::out_of_range("slice index cannot be negative");
        }
    }
    return static_cast<size_t>(idx);
}

template<typename SpanLike>
void validate_slice_bounds(const SpanLike& span, size_t start, size_t end) {
    if (start > end || end > span.size()) {
        throw std::out_of_range("slice range out of bounds");
    }
}

template<typename T, typename = void>
struct has_is_some : std::false_type {};

template<typename T>
struct has_is_some<T, std::void_t<decltype(std::declval<const T&>().is_some())>> : std::true_type {};

template<typename T, typename = void>
struct has_unwrap : std::false_type {};

template<typename T>
struct has_unwrap<T, std::void_t<decltype(std::declval<T&>().unwrap())>> : std::true_type {};

template<typename T, typename = void>
struct has_has_value : std::false_type {};

template<typename T>
struct has_has_value<T, std::void_t<decltype(std::declval<const T&>().has_value())>> : std::true_type {};

template<typename T, typename = void>
struct has_reset : std::false_type {};

template<typename T>
struct has_reset<T, std::void_t<decltype(std::declval<T&>().reset())>> : std::true_type {};

template<typename Opt>
bool option_has_value(const Opt& opt) {
    if constexpr (has_is_some<Opt>::value) {
        return opt.is_some();
    } else if constexpr (has_has_value<Opt>::value) {
        return opt.has_value();
    } else {
        return static_cast<bool>(opt);
    }
}

template<typename Opt>
auto option_take_value(Opt& opt) {
    if constexpr (has_unwrap<Opt>::value) {
        return opt.unwrap();
    } else if constexpr (has_has_value<Opt>::value && has_reset<Opt>::value) {
        auto value = std::move(*opt);
        opt.reset();
        return value;
    } else {
        return std::move(*opt);
    }
}

template<typename Opt>
using option_value_t = std::decay_t<decltype(option_take_value(std::declval<Opt&>()))>;

template<typename Range>
using range_storage_t =
    std::conditional_t<std::is_lvalue_reference_v<Range>, Range, std::decay_t<Range>>;

template<typename T>
struct maybe_uninit_payload {
    using type = void;
};

template<typename U>
struct maybe_uninit_payload<rusty::MaybeUninit<U>> {
    using type = U;
};

template<typename Container>
using container_item_t = typename std::remove_reference_t<Container>::Item;

template<typename Container, typename = void>
struct has_container_item : std::false_type {};

template<typename Container>
struct has_container_item<Container, std::void_t<container_item_t<Container>>> : std::true_type {};

template<typename Ptr>
using raw_ptr_t = std::remove_cv_t<std::remove_reference_t<Ptr>>;

template<typename Container, typename Ptr>
decltype(auto) adapt_as_ptr_result(const Container&, Ptr ptr) {
    using PtrRaw = raw_ptr_t<Ptr>;
    if constexpr (!std::is_pointer_v<PtrRaw>) {
        return ptr;
    } else {
        using Pointee = std::remove_pointer_t<PtrRaw>;
        using MaybeUninitT = std::remove_cv_t<Pointee>;
        using Payload = typename maybe_uninit_payload<MaybeUninitT>::type;
        if constexpr (std::is_void_v<Payload>) {
            return ptr;
        } else if constexpr (has_container_item<Container>::value) {
            using Item = std::remove_reference_t<container_item_t<Container>>;
            if constexpr (std::is_same_v<std::remove_cv_t<Item>, std::remove_cv_t<Payload>>) {
                using ConstItem = std::add_const_t<container_item_t<Container>>;
                return reinterpret_cast<std::add_pointer_t<ConstItem>>(ptr);
            }
            return reinterpret_cast<std::add_pointer_t<std::add_const_t<Payload>>>(ptr);
        } else {
            return reinterpret_cast<std::add_pointer_t<std::add_const_t<Payload>>>(ptr);
        }
    }
}

template<typename Container, typename Ptr>
decltype(auto) adapt_as_mut_ptr_result(Container&, Ptr ptr) {
    using PtrRaw = raw_ptr_t<Ptr>;
    if constexpr (!std::is_pointer_v<PtrRaw>) {
        return ptr;
    } else {
        using Pointee = std::remove_pointer_t<PtrRaw>;
        using MaybeUninitT = std::remove_cv_t<Pointee>;
        using Payload = typename maybe_uninit_payload<MaybeUninitT>::type;
        if constexpr (std::is_void_v<Payload>) {
            return ptr;
        } else if constexpr (has_container_item<Container>::value) {
            using Item = std::remove_reference_t<container_item_t<Container>>;
            if constexpr (std::is_same_v<std::remove_cv_t<Item>, std::remove_cv_t<Payload>>) {
                return reinterpret_cast<std::add_pointer_t<container_item_t<Container>>>(ptr);
            }
            return reinterpret_cast<std::add_pointer_t<Payload>>(ptr);
        } else {
            return reinterpret_cast<std::add_pointer_t<Payload>>(ptr);
        }
    }
}
} // namespace detail

/// Create a vector filled with `count` copies of `value`.
/// Equivalent to Rust's `[value; count]` array repeat syntax.
template<typename T>
std::vector<T> array_repeat(T value, size_t count) {
    return std::vector<T>(count, value);
}

/// Collect any iterable range into rusty::Vec<T>.
/// Used by transpiled Rust range `.collect()` calls.
template<typename Range>
auto collect_range(Range&& range_like) {
    if constexpr (requires(Range&& r) {
        std::begin(r);
        std::end(r);
    }) {
        using Elem = std::decay_t<decltype(*std::begin(range_like))>;
        Vec<Elem> out = Vec<Elem>::new_();
        for (auto&& item : range_like) {
            out.push(std::forward<decltype(item)>(item));
        }
        return out;
    } else if constexpr (requires(Range&& r) { std::forward<Range>(r).into_iter(); }) {
        return collect_range(std::forward<Range>(range_like).into_iter());
    } else if constexpr (requires(Range&& r) { std::forward<Range>(r).next(); }) {
        auto iter = std::forward<Range>(range_like);
        using NextResult = decltype(iter.next());
        static_assert(
            requires(NextResult& next_item) {
                detail::option_has_value(next_item);
                detail::option_take_value(next_item);
            },
            "rusty::collect_range requires next() to return an Option/optional-like value");
        using Elem = std::decay_t<decltype(detail::option_take_value(
            std::declval<NextResult&>()))>;
        Vec<Elem> out = Vec<Elem>::new_();
        while (true) {
            auto item = iter.next();
            if (!detail::option_has_value(item)) {
                break;
            }
            out.push(detail::option_take_value(item));
        }
        return out;
    } else {
        static_assert(
            detail::collect_range_dependent_false_v<Range>,
            "rusty::collect_range requires a range, into_iter(), or Option-like next()");
    }
}

template<typename T>
decltype(auto) as_ptr(const T& value) {
    if constexpr (requires { value.as_ptr(); }) {
        return detail::adapt_as_ptr_result(value, value.as_ptr());
    } else if constexpr (requires { value.data(); }) {
        return detail::adapt_as_ptr_result(value, value.data());
    } else if constexpr (requires { value.begin(); }) {
        if constexpr (std::is_pointer_v<std::remove_reference_t<decltype(value.begin())>>) {
            return detail::adapt_as_ptr_result(value, value.begin());
        } else {
            return &value;
        }
    } else {
        return &value;
    }
}

template<typename T>
decltype(auto) as_mut_ptr(T& value) {
    if constexpr (requires { value.as_mut_ptr(); }) {
        return detail::adapt_as_mut_ptr_result(value, value.as_mut_ptr());
    } else if constexpr (requires { value.data(); }) {
        return detail::adapt_as_mut_ptr_result(value, value.data());
    } else if constexpr (requires { value.begin(); }) {
        if constexpr (std::is_pointer_v<std::remove_reference_t<decltype(value.begin())>>) {
            return detail::adapt_as_mut_ptr_result(value, value.begin());
        } else {
            return &value;
        }
    } else {
        return &value;
    }
}

template<typename Left, typename Right>
auto zip(Left&& left, Right&& right) {
    using LeftElem = std::decay_t<decltype(*std::begin(left))>;
    using RightElem = std::decay_t<decltype(*std::begin(right))>;
    std::vector<std::tuple<LeftElem, RightElem>> out;
    auto left_it = std::begin(left);
    auto left_end = std::end(left);
    auto right_it = std::begin(right);
    auto right_end = std::end(right);
    for (; left_it != left_end && right_it != right_end; ++left_it, ++right_it) {
        out.emplace_back(*left_it, *right_it);
    }
    return out;
}

/// Unified length helper for transpiled `.len()` calls.
/// Supports rusty types (`.len()`), STL/span (`.size()`), and native arrays.
inline size_t len(const char* cstr) {
    return cstr ? std::strlen(cstr) : 0;
}

inline size_t len(char* cstr) {
    return cstr ? std::strlen(cstr) : 0;
}

template<typename Container>
size_t len(const Container& container) {
    if constexpr (requires { container.len(); }) {
        return static_cast<size_t>(container.len());
    } else if constexpr (requires { container.size(); }) {
        return static_cast<size_t>(container.size());
    } else if constexpr (requires { container.as_str(); }) {
        return rusty::len(container.as_str());
    } else if constexpr (requires { std::size(container); }) {
        return static_cast<size_t>(std::size(container));
    } else {
        static_assert(
            detail::collect_range_dependent_false_v<Container>,
            "rusty::len requires len(), size(), as_str(), or std::size-compatible range");
    }
}

template<typename T, typename U>
constexpr auto saturating_add(T lhs, U rhs) {
    using R = std::common_type_t<T, U>;
    static_assert(std::is_integral_v<R>, "rusty::saturating_add requires integral operands");
    const R a = static_cast<R>(lhs);
    const R b = static_cast<R>(rhs);
    if constexpr (std::is_unsigned_v<R>) {
        const R max = std::numeric_limits<R>::max();
        if (a > max - b) {
            return max;
        }
        return static_cast<R>(a + b);
    } else {
        const R max = std::numeric_limits<R>::max();
        const R min = std::numeric_limits<R>::min();
        if (b > 0 && a > max - b) {
            return max;
        }
        if (b < 0 && a < min - b) {
            return min;
        }
        return static_cast<R>(a + b);
    }
}

template<typename T, typename U>
constexpr auto saturating_sub(T lhs, U rhs) {
    using R = std::common_type_t<T, U>;
    static_assert(std::is_integral_v<R>, "rusty::saturating_sub requires integral operands");
    const R a = static_cast<R>(lhs);
    const R b = static_cast<R>(rhs);
    if constexpr (std::is_unsigned_v<R>) {
        if (a < b) {
            return static_cast<R>(0);
        }
        return static_cast<R>(a - b);
    } else {
        const R max = std::numeric_limits<R>::max();
        const R min = std::numeric_limits<R>::min();
        if (b > 0 && a < min + b) {
            return min;
        }
        if (b < 0 && a > max + b) {
            return max;
        }
        return static_cast<R>(a - b);
    }
}

/// Lazy filter_map view for transpiled iterator chains (`iter().filter_map(...)`).
/// The mapping closure is only evaluated while iterating the view.
template<typename Range, typename Func>
class filter_map_view {
    using base_iterator = decltype(std::begin(std::declval<Range&>()));
    using option_type = std::decay_t<decltype(std::declval<Func&>()(*std::declval<base_iterator&>()))>;
    using value_type = detail::option_value_t<option_type>;

public:
    template<typename R, typename F>
    filter_map_view(R&& range, F&& func)
        : range_(std::forward<R>(range)), func_(std::forward<F>(func)) {}

    class iterator {
    public:
        iterator(base_iterator current, base_iterator end, Func* func, bool at_end = false)
            : current_(current), end_(end), func_(func), at_end_(at_end) {
            if (!at_end_) {
                advance();
            }
        }

        const value_type& operator*() const { return *cached_; }

        iterator& operator++() {
            advance();
            return *this;
        }

        bool operator!=(const iterator& other) const {
            return current_ != other.current_ || at_end_ != other.at_end_;
        }

    private:
        void advance() {
            cached_.reset();
            while (current_ != end_) {
                auto mapped = (*func_)(*current_);
                ++current_;
                if (detail::option_has_value(mapped)) {
                    cached_.emplace(detail::option_take_value(mapped));
                    at_end_ = false;
                    return;
                }
            }
            at_end_ = true;
        }

        base_iterator current_;
        base_iterator end_;
        Func* func_;
        std::optional<value_type> cached_;
        bool at_end_;
    };

    iterator begin() { return iterator(std::begin(range_), std::end(range_), &func_); }
    iterator end() { return iterator(std::end(range_), std::end(range_), &func_, true); }
    iterator begin() const { return iterator(std::begin(range_), std::end(range_), &func_); }
    iterator end() const { return iterator(std::end(range_), std::end(range_), &func_, true); }

private:
    Range range_;
    mutable Func func_;
};

template<typename Range, typename Func>
auto filter_map(Range&& range, Func&& func) {
    using StoredRange = detail::range_storage_t<Range&&>;
    using StoredFunc = std::decay_t<Func>;
    return filter_map_view<StoredRange, StoredFunc>(
        std::forward<Range>(range),
        std::forward<Func>(func)
    );
}

/// Slice helpers used by transpiled Rust range-index expressions.
/// Examples:
/// - `x[..]` -> `slice_full(x)`
/// - `x[..n]` -> `slice_to(x, n)`
/// - `x[a..b]` -> `slice(x, a, b)`
template<typename Container>
auto slice_full(Container& container) {
    if constexpr (requires { container.as_mut_slice(); }) {
        return container.as_mut_slice();
    } else if constexpr (requires { container.as_slice(); }) {
        return container.as_slice();
    } else {
        using Elem = std::remove_reference_t<decltype(*rusty::as_mut_ptr(container))>;
        return std::span<Elem>(rusty::as_mut_ptr(container), rusty::len(container));
    }
}

template<typename Container>
auto slice_full(const Container& container) {
    if constexpr (requires { container.as_slice(); }) {
        return container.as_slice();
    } else {
        using Elem = std::remove_reference_t<decltype(*rusty::as_ptr(container))>;
        return std::span<const Elem>(rusty::as_ptr(container), rusty::len(container));
    }
}

// Collect a slice-like container into rusty::Vec by value-cloning elements.
// Used by transpiled Rust `.to_vec()` lowering for slice/array/ArrayVec shapes.
template<typename Container>
auto to_vec(const Container& container) {
    auto span = slice_full(container);
    using Elem = std::remove_cv_t<std::remove_reference_t<decltype(*span.data())>>;
    Vec<Elem> out = Vec<Elem>::new_();
    for (const auto& item : span) {
        if constexpr (requires(const Elem& e) { e.clone(); }) {
            out.push(item.clone());
        } else {
            out.push(item);
        }
    }
    return out;
}

// Clone elements from one slice into another.
// Mirrors Rust `[T]::clone_from_slice` semantics with a size check.
template<typename DstElem, std::size_t DstExtent, typename SrcElem, std::size_t SrcExtent>
void clone_from_slice(std::span<DstElem, DstExtent> dst, std::span<SrcElem, SrcExtent> src) {
    static_assert(!std::is_const_v<DstElem>, "clone_from_slice destination must be mutable");
    if (dst.size() != src.size()) {
        throw std::length_error("clone_from_slice length mismatch");
    }
    for (size_t i = 0; i < dst.size(); ++i) {
        if constexpr (requires(const SrcElem& value) { value.clone(); }) {
            dst[i] = src[i].clone();
        } else {
            dst[i] = src[i];
        }
    }
}

template<typename Container, typename End>
auto slice_to(Container& container, End end) {
    auto span = slice_full(container);
    const size_t end_index = detail::checked_index(end);
    detail::validate_slice_bounds(span, 0, end_index);
    return span.first(end_index);
}

template<typename Container, typename End>
auto slice_to_inclusive(Container& container, End end) {
    const size_t end_index = detail::checked_index(end);
    return slice_to(container, end_index + 1);
}

template<typename Container, typename Start>
auto slice_from(Container& container, Start start) {
    auto span = slice_full(container);
    const size_t start_index = detail::checked_index(start);
    detail::validate_slice_bounds(span, start_index, span.size());
    if constexpr (std::is_same_v<std::remove_cvref_t<Container>, std::string_view>) {
        return container.substr(start_index);
    } else {
        return span.subspan(start_index);
    }
}

template<typename Container, typename Start, typename End>
auto slice(Container& container, Start start, End end) {
    auto span = slice_full(container);
    const size_t start_index = detail::checked_index(start);
    const size_t end_index = detail::checked_index(end);
    detail::validate_slice_bounds(span, start_index, end_index);
    if constexpr (std::is_same_v<std::remove_cvref_t<Container>, std::string_view>) {
        return container.substr(start_index, end_index - start_index);
    } else {
        return span.subspan(start_index, end_index - start_index);
    }
}

template<typename Container, typename Start, typename End>
auto slice_inclusive(Container& container, Start start, End end) {
    const size_t end_index = detail::checked_index(end);
    return slice(container, start, end_index + 1);
}

/// Iterable range [start, end) — equivalent to Rust's `start..end`.
template<typename T>
struct Bound_Unbounded {};

template<typename T>
struct Bound_Included {
    T _0;
};

template<typename T>
struct Bound_Excluded {
    T _0;
};

template<typename T>
using Bound = std::variant<Bound_Unbounded<T>, Bound_Included<T>, Bound_Excluded<T>>;

template<typename T>
class range {
public:
    range(T start, T end) : start_(start), end_(end) {}

    range into_iter() {
        return std::move(*this);
    }

    struct iterator {
        T current;
        T operator*() const { return current; }
        iterator& operator++() { ++current; return *this; }
        bool operator!=(const iterator& other) const { return current != other.current; }
    };

    iterator begin() const { return {start_}; }
    iterator end() const { return {end_}; }

    Bound<T> start_bound() const { return Bound<T>(Bound_Included<T>{start_}); }
    Bound<T> end_bound() const { return Bound<T>(Bound_Excluded<T>{end_}); }

    /// Rust-style iterator protocol helper used by transpiled `.next()` calls.
    std::optional<T> next() {
        if (start_ == end_) {
            return std::nullopt;
        }
        T current = start_;
        ++start_;
        return current;
    }

    /// Rust-style iterator protocol helper used by transpiled `.count()` calls.
    size_t count() const {
        T current = start_;
        size_t n = 0;
        while (current != end_) {
            ++current;
            ++n;
        }
        return n;
    }

private:
    T start_, end_;
};

template<typename A, typename B>
range(A, B) -> range<std::common_type_t<A, B>>;

/// Inclusive range [start, end] — equivalent to Rust's `start..=end`.
template<typename T>
class range_inclusive {
public:
    range_inclusive(T start, T end) : start_(start), end_(end) {}

    range_inclusive into_iter() {
        return std::move(*this);
    }

    struct iterator {
        T current;
        T end;
        bool done;
        T operator*() const { return current; }
        iterator& operator++() { if (current == end) done = true; else ++current; return *this; }
        bool operator!=(const iterator& other) const { return !done; }
    };

    iterator begin() const { return {start_, end_, false}; }
    iterator end() const { return {end_, end_, true}; }

    Bound<T> start_bound() const { return Bound<T>(Bound_Included<T>{start_}); }
    Bound<T> end_bound() const { return Bound<T>(Bound_Included<T>{end_}); }

    /// Rust-style iterator protocol helper used by transpiled `.next()` calls.
    std::optional<T> next() {
        if (done_) {
            return std::nullopt;
        }
        T current = start_;
        if (start_ == end_) {
            done_ = true;
        } else {
            ++start_;
        }
        return current;
    }

    /// Rust-style iterator protocol helper used by transpiled `.count()` calls.
    size_t count() const {
        if (done_) {
            return 0;
        }
        T current = start_;
        size_t n = 0;
        while (true) {
            ++n;
            if (current == end_) {
                break;
            }
            ++current;
        }
        return n;
    }

private:
    T start_, end_;
    bool done_ = false;
};

template<typename A, typename B>
range_inclusive(A, B) -> range_inclusive<std::common_type_t<A, B>>;

/// Open range from start — equivalent to Rust's `start..`.
template<typename T>
struct range_from {
    T start;

    range_from into_iter() {
        return std::move(*this);
    }

    Bound<T> start_bound() const { return Bound<T>(Bound_Included<T>{start}); }
    Bound<T> end_bound() const { return Bound<T>(Bound_Unbounded<T>{}); }

    /// Rust-style iterator protocol helper used by transpiled `.next()` calls.
    std::optional<T> next() {
        T current = start;
        ++start;
        return current;
    }

    /// Rust-style iterator protocol helper used by transpiled `.count()` calls.
    /// `start..` is unbounded, so this mirrors an effectively-infinite count.
    size_t count() const {
        return std::numeric_limits<size_t>::max();
    }
};

/// Range to end — equivalent to Rust's `..end`.
template<typename T>
struct range_to {
    T end;

    Bound<T> start_bound() const { return Bound<T>(Bound_Unbounded<T>{}); }
    Bound<T> end_bound() const { return Bound<T>(Bound_Excluded<T>{end}); }
};

/// Full range — equivalent to Rust's `..`.
struct range_full {
    template<typename T = size_t>
    Bound<T> start_bound() const { return Bound<T>(Bound_Unbounded<T>{}); }
    template<typename T = size_t>
    Bound<T> end_bound() const { return Bound<T>(Bound_Unbounded<T>{}); }
};

/// Range to inclusive — equivalent to Rust's `..=end`.
template<typename T>
struct range_to_inclusive {
    T end;

    Bound<T> start_bound() const { return Bound<T>(Bound_Unbounded<T>{}); }
    Bound<T> end_bound() const { return Bound<T>(Bound_Included<T>{end}); }
};

} // namespace rusty

#endif // RUSTY_ARRAY_HPP
