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
#include <limits>
#include <span>
#include <stdexcept>
#include <rusty/vec.hpp>

// GCC/libstdc++ C++23 does not provide span equality operators.
// Keep a narrow value-comparison overload so transpiled Rust slice assertions compile.
template<typename L, std::size_t LExtent, typename R, std::size_t RExtent>
constexpr bool operator==(std::span<L, LExtent> lhs, std::span<R, RExtent> rhs) {
    return lhs.size() == rhs.size() && std::equal(lhs.begin(), lhs.end(), rhs.begin());
}

namespace rusty {

namespace detail {
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
    using Elem = std::decay_t<decltype(*std::begin(range_like))>;
    Vec<Elem> out = Vec<Elem>::new_();
    for (auto&& item : range_like) {
        out.push(std::forward<decltype(item)>(item));
    }
    return out;
}

/// Unified length helper for transpiled `.len()` calls.
/// Supports rusty types (`.len()`), STL/span (`.size()`), and native arrays.
template<typename Container>
size_t len(const Container& container) {
    if constexpr (requires { container.len(); }) {
        return static_cast<size_t>(container.len());
    } else if constexpr (requires { container.size(); }) {
        return static_cast<size_t>(container.size());
    } else {
        return static_cast<size_t>(std::size(container));
    }
}

/// Slice helpers used by transpiled Rust range-index expressions.
/// Examples:
/// - `x[..]` -> `slice_full(x)`
/// - `x[..n]` -> `slice_to(x, n)`
/// - `x[a..b]` -> `slice(x, a, b)`
template<typename Container>
auto slice_full(Container& container) {
    using Elem = std::remove_reference_t<decltype(*std::data(container))>;
    return std::span<Elem>(std::data(container), std::size(container));
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
    return span.subspan(start_index);
}

template<typename Container, typename Start, typename End>
auto slice(Container& container, Start start, End end) {
    auto span = slice_full(container);
    const size_t start_index = detail::checked_index(start);
    const size_t end_index = detail::checked_index(end);
    detail::validate_slice_bounds(span, start_index, end_index);
    return span.subspan(start_index, end_index - start_index);
}

template<typename Container, typename Start, typename End>
auto slice_inclusive(Container& container, Start start, End end) {
    const size_t end_index = detail::checked_index(end);
    return slice(container, start, end_index + 1);
}

/// Iterable range [start, end) — equivalent to Rust's `start..end`.
template<typename T>
class range {
public:
    range(T start, T end) : start_(start), end_(end) {}

    struct iterator {
        T current;
        T operator*() const { return current; }
        iterator& operator++() { ++current; return *this; }
        bool operator!=(const iterator& other) const { return current != other.current; }
    };

    iterator begin() const { return {start_}; }
    iterator end() const { return {end_}; }

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

/// Inclusive range [start, end] — equivalent to Rust's `start..=end`.
template<typename T>
class range_inclusive {
public:
    range_inclusive(T start, T end) : start_(start), end_(end) {}

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

private:
    T start_, end_;
};

/// Open range from start — equivalent to Rust's `start..`.
template<typename T>
struct range_from {
    T start;

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
};

/// Full range — equivalent to Rust's `..`.
struct range_full {};

/// Range to inclusive — equivalent to Rust's `..=end`.
template<typename T>
struct range_to_inclusive {
    T end;
};

} // namespace rusty

#endif // RUSTY_ARRAY_HPP
