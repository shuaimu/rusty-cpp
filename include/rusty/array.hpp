#ifndef RUSTY_ARRAY_HPP
#define RUSTY_ARRAY_HPP

#include <array>
#include <vector>
#include <cstddef>
#include <algorithm>

namespace rusty {

/// Create a vector filled with `count` copies of `value`.
/// Equivalent to Rust's `[value; count]` array repeat syntax.
template<typename T>
std::vector<T> array_repeat(T value, size_t count) {
    return std::vector<T>(count, value);
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
