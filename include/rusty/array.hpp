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
#include <rusty/vec.hpp>

namespace rusty {

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
