module;

// Keep std declarations in the global module to avoid libstdc++ named-module
// attachment conflicts in importer translation units.
#if __has_include(<bits/stdc++.h>)
#include <bits/stdc++.h>
#else
#include <algorithm>
#include <any>
#include <array>
#include <atomic>
#include <bit>
#include <cassert>
#include <charconv>
#include <chrono>
#include <condition_variable>
#include <coroutine>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <exception>
#include <functional>
#include <future>
#include <initializer_list>
#include <iomanip>
#include <ios>
#include <iostream>
#include <iterator>
#include <limits>
#include <map>
#include <memory>
#include <mutex>
#include <new>
#include <numeric>
#include <optional>
#include <queue>
#include <set>
#include <span>
#include <sstream>
#include <stdexcept>
#include <string>
#include <string_view>
#include <thread>
#include <tuple>
#include <type_traits>
#include <typeindex>
#include <unordered_map>
#include <unordered_set>
#include <utility>
#include <variant>
#include <vector>
#endif

export module rusty;

// Some runtime templates rely on placement-new/delete lookup in importers.
export using ::operator new;
export using ::operator delete;

// NOTE:
// This interface intentionally exports a GCC-14-stable subset of rusty headers.
// The full umbrella `rusty/rusty.hpp` currently triggers importer ICEs under
// `-fmodules-ts` for some header combinations.
export {
#include <rusty/box.hpp>
#include <rusty/vec.hpp>
#include <rusty/vecdeque.hpp>
#include <rusty/option.hpp>
#include <rusty/result.hpp>
#include <rusty/marker.hpp>
#include <rusty/ptr.hpp>
#include <rusty/mem.hpp>
#include <rusty/alloc.hpp>
#include <rusty/panic.hpp>
#include <rusty/cell.hpp>
#include <rusty/refcell.hpp>
#include <rusty/fmt.hpp>
#include <rusty/string.hpp>
#include <rusty/fn.hpp>
#include <rusty/function.hpp>
#include <rusty/btreemap.hpp>
#include <rusty/btreeset.hpp>
#include <rusty/array.hpp>
#include <rusty/slice.hpp>
#include <rusty/io.hpp>
#include <rusty/net.hpp>
#include <rusty/process.hpp>
#include <rusty/error.hpp>
#include <rusty/move.hpp>
#include <rusty/sync/atomic.hpp>
#include <rusty/sync/mpsc.hpp>
#include <rusty/mutex.hpp>
#include <rusty/rwlock.hpp>
#include <rusty/condvar.hpp>
#include <rusty/barrier.hpp>
#include <rusty/once.hpp>
#include <rusty/thread.hpp>
#include <rusty/async.hpp>
} // export

export namespace rusty {

template<typename T>
using ResultVoid = Result<T, void>;

template<typename T>
using ResultString = Result<T, const char*>;

template<typename T>
using ResultInt = Result<T, int>;

template<typename T>
Box<T> from_raw(T* ptr) {
    return Box<T>(ptr);
}

template<typename T>
Box<T> box_from_raw(T* ptr) {
    return from_raw(ptr);
}

template<typename T>
using Boxed = Box<T>;

template<typename T>
requires requires { T::default_(); }
auto default_value() {
    return T::default_();
}

template<typename T>
requires (!requires { T::default_(); } && requires { T::empty(); })
auto default_value() {
    return T::empty();
}

template<typename T>
requires (!requires { T::default_(); } && !requires { T::empty(); } && requires { T{}; })
T default_value() {
    return T{};
}

template<std::size_t N>
constexpr std::size_t sanitize_array_capacity() noexcept {
    if constexpr (N == std::numeric_limits<std::size_t>::max()) {
        return 1;
    } else {
        return N;
    }
}

template<typename T>
std::string_view to_string_view(T&& value) {
    if constexpr (requires { { *value } -> std::convertible_to<std::string_view>; }) {
        return std::string_view(*value);
    } else if constexpr (requires { value.as_str(); }) {
        return std::string_view(value.as_str());
    } else {
        return std::string_view(std::forward<T>(value));
    }
}

inline String to_owned(std::string_view value) {
    return String::from(value);
}

inline String to_owned(const char* value) {
    return String::from(value);
}

inline String to_owned(const str& value) {
    return String::from(value.as_str());
}

template<typename T, std::size_t Extent>
Vec<std::remove_const_t<T>> to_owned(std::span<T, Extent> value) {
    using Elem = std::remove_const_t<T>;
    Vec<Elem> out(value.size());
    for (const auto& item : value) {
        out.push(static_cast<Elem>(item));
    }
    return out;
}

template<typename T>
auto to_owned(const T& value) {
    if constexpr (requires { value.clone(); }) {
        return value.clone();
    } else {
        return T(value);
    }
}

namespace boxed {

template<typename T>
constexpr std::decay_t<T> box_new(T&& value) {
    return std::forward<T>(value);
}

template<typename T, std::size_t N>
Vec<T> into_vec(std::array<T, N> values) {
    Vec<T> out(N);
    for (auto& value : values) {
        out.push(std::move(value));
    }
    return out;
}

template<typename T, typename Alloc>
Vec<T> into_vec(std::vector<T, Alloc> values) {
    Vec<T> out(values.size());
    for (auto& value : values) {
        out.push(std::move(value));
    }
    return out;
}

template<typename T>
constexpr std::decay_t<T> into_vec(T&& value) {
    return std::forward<T>(value);
}

} // namespace boxed

} // namespace rusty
