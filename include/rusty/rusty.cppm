module;

// All std + rusty headers go in the GMF (Global Module Fragment), NOT
// the module purview. The hand-written `<rusty/*.hpp>` files
// transitively `#include <shared_mutex>` / `<fstream>` /
// `<netinet/tcp.h>` etc. via the `platform/threading` sub-header
// chain. Leaving them in the old `export { #include <rusty/*.hpp> }`
// module-purview block attached every std type they touched to
// module `rusty`, and importers that also `#include` the same std
// headers in their own GMF (rrr's module units all do) then saw
// duplicate declarations — Clang 21+ rejects with:
//
//   declaration of 'X' in module rusty follows declaration in the
//   global module
//
// Moving the rusty/*.hpp `#include`s into the GMF below pulls the
// whole transitive closure onto global-module attachment, matching
// what importers see in their own GMFs.
//
// The rusty types declared via `namespace rusty { … }` inside those
// headers are reachable in the module purview below as `::rusty::Foo`;
// the `export namespace rusty { using = … }` aliases re-export them
// for `import rusty;` consumers. Most consumers (rrr's module units)
// already `#include <rusty/*.hpp>` directly via their own GMF, so the
// umbrella's value-add is really the transpiled-port re-exports —
// Vec / Rc / BTreeMap / BTreeSet / HashMap / HashSet / VecDeque /
// BinaryHeap / …

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

// Rusty hand-written headers in GMF (not module purview). See block
// comment at the top for the libstdc++ attachment rationale.
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
#include <rusty/array.hpp>
#include <rusty/slice.hpp>
#include <rusty/io.hpp>
#include <rusty/net.hpp>
#include <rusty/process.hpp>
#include <rusty/error.hpp>
#include <rusty/move.hpp>
#include <rusty/dispatch.hpp>
#include <rusty/sync/atomic.hpp>
#include <rusty/sync/mpsc.hpp>
#include <rusty/sys/fs.hpp>
#include <rusty/sys/time.hpp>
#include <rusty/sys/process.hpp>
#include <rusty/sys/env.hpp>
#include <rusty/sys/pthread.hpp>
#include <rusty/os/fd.hpp>
#include <rusty/net/tcp.hpp>
#include <rusty/mutex.hpp>
#include <rusty/rwlock.hpp>
#include <rusty/condvar.hpp>
#include <rusty/barrier.hpp>
#include <rusty/once.hpp>
#include <rusty/thread.hpp>
#include <rusty/async.hpp>

export module rusty;

// C++20 rule: every `export import …;` declaration must appear in the
// module's import section, immediately following the module declaration
// and preceding any other declarations. These re-export the transpiled
// rustc-stdlib ports (`vec_port`, `btree_port`, `rc_port`,
// `hashbrown_port`, …) plus the hand-written `rusty.async` module
// through this umbrella, so consumers can `import rusty;` and get the
// full surface. Previously these lived after the `export { #include … }`
// block below — Clang 21+ rejects that ordering with "imports must
// immediately follow the module declaration".
export import rusty.async;
export import vec_port.vec;
export import btree_port.btree.map;
export import btree_port.btree.set;
export import rc_port;
export import binary_heap_port;  // namespace: rusty::port::collections::binary_heap
export import hashbrown_port.map;
export import hashbrown_port.set;
export import vec_deque_port;
export import cell_port;
export import string_port;
export import arc_port;

// Some runtime templates rely on placement-new/delete lookup in importers.
export using ::operator new;
export using ::operator delete;

// rusty/*.hpp includes were moved into the GMF above. Including them
// inside an `export { … }` block here would attach every std type they
// transitively touch (`std::shared_mutex`, `std::basic_filebuf`, …) to
// module rusty and re-introduce the importer-side attachment conflict.
// `rusty::Box` / `Option` / `Result` / etc. are still reachable for
// importers because every importer #includes the same rusty/*.hpp via
// its own GMF (just like the umbrella's GMF here does).

// `rusty::Executor`, `rusty::Vec`, `rusty::BTreeMap`, `rusty::BTreeSet`
// live in C++20 modules now — the `export import` lines for those have
// been moved up to the import section immediately after the
// `export module rusty;` declaration above, per the C++20 rule that all
// imports precede every other declaration. The using-aliases below now
// just bind those imported symbols into the `rusty::` namespace.

export namespace rusty {

// VecLegacy retired — `rusty::Vec<T,A>` is the transpiled rustc Vec.
template<typename T, typename A = ::rusty::alloc::Global>
using Vec = ::Vec<T, A>;

// Legacy hand-written rusty::Rc retired — `rusty::Rc<T,A>` is now the
// transpiled rustc Rc from `library/alloc/src/rc.rs`. API change:
// use `Rc<T>::new_(value)` instead of constructor / `make(value)`.
template<typename T, typename A = ::rusty::alloc::Global>
using Rc = ::rusty::port::rc::Rc<T, A>;

// Note: no top-level `rusty::Weak` alias — that would collapse the
// distinct Rust types `std::rc::Weak<T>` and `std::sync::Weak<T>`
// under one ambiguous name. Use `rusty::rc::Weak<T, A>` (declared
// in `namespace rusty::rc` below) or `rusty::sync::Weak<T>` (in
// include/rusty/sync/weak.hpp) explicitly.

// rusty::BTreeMap / rusty::BTreeSet alias the transpiled rustc port.
// Note: no Compare parameter — Rust's BTreeMap uses the Ord trait
// directly, mirrored in C++ via operator<. Consumers that need a
// custom comparator should use the deep path explicitly.
template<typename K, typename V, typename A = ::rusty::alloc::Global>
using BTreeMap = ::btree_port::btree::map::BTreeMap<K, V, A>;

template<typename T, typename A = ::rusty::alloc::Global>
using BTreeSet = ::rusty::port::collections::btree::set::BTreeSet<T, A>;

// rusty::port — namespace hierarchy mirroring Rust std's layout
// (port = transpiled-from-rustc). Each transpiled module lives under
// `rusty::port::<section>::<crate_name>::Type` (the deep path), with
// a flat alias `rusty::port::<section>::Type` for the common case.
// E.g. `rusty::port::collections::BinaryHeap` →
// `rusty::port::collections::binary_heap::BinaryHeap`.
namespace port::collections {
    template<typename T, typename A = ::rusty::alloc::Global>
    using BinaryHeap = ::rusty::port::collections::binary_heap::BinaryHeap<T, A>;
    template<typename T, typename A = ::rusty::alloc::Global>
        requires (rusty::alloc::Allocator<A>)
    using VecDeque = ::rusty::port::collections::vec_deque::VecDeque<T, A>;
}

namespace rc {
// `rusty::rc::Weak<T, A>` — the single-threaded weak reference,
// companion to `rusty::Rc`. Mirrors Rust's `std::rc::Weak`.
template<typename T, typename A = ::rusty::alloc::Global>
using Weak = ::rusty::port::rc::Weak<T, A>;
} // namespace rc

// User-facing `rusty::collections::*` aliases. End users write
// `rusty::collections::HashMap<K,V>` / `HashSet<T>` to match Rust's
// `std::collections::*`. HashMap/HashSet come from hashbrown_port,
// now deep-migrated to `rusty::port::collections::hashbrown::*` via
// the `--cxx-namespace` transpiler flag.
namespace collections {
    template<typename K, typename V, typename S = ::rusty::port::collections::hashbrown::DefaultHasher>
    using HashMap = ::rusty::port::collections::hashbrown::HashMap<K, V, S>;
    template<typename T, typename S = ::rusty::port::collections::hashbrown::DefaultHasher>
    using HashSet = ::rusty::port::collections::hashbrown::HashSet<T, S>;
    template<typename K, typename V, typename A = ::rusty::alloc::Global>
    using BTreeMap = ::btree_port::btree::map::BTreeMap<K, V, A>;
    template<typename T, typename A = ::rusty::alloc::Global>
    using BTreeSet = ::rusty::port::collections::btree::set::BTreeSet<T, A>;
}

} // export namespace rusty

export namespace rusty {

// `rusty::forward` / `rusty::exchange` / `rusty::swap` are no longer
// (re-)defined here in the module purview — `<rusty/rusty.hpp>` (now
// in this file's GMF via the `#include <rusty/vec.hpp>` chain, which
// pulls in the umbrella header) already declares them attached to the
// global module. Re-declaring them in module rusty's purview would
// trigger "declaration of 'X' in module rusty follows declaration in
// the global module" attachment conflicts. Importers reach them via
// the global-module attachment chain through their own GMF includes.
using Unit = std::tuple<>;
using StrView = std::string_view;
template<typename T, std::size_t Extent = std::dynamic_extent>
using Span = std::span<T, Extent>;

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

namespace detail {
    template<typename T>
    concept string_view_compatible =
        requires(T&& value) {
            { *std::forward<T>(value) } -> std::convertible_to<std::string_view>;
        } ||
        requires(T&& value) {
            std::forward<T>(value).as_str();
        } ||
        requires(T&& value) {
            std::string_view(std::forward<T>(value));
        };
}

template<typename T>
requires detail::string_view_compatible<T>
std::string_view to_string_view(T&& value) {
    if constexpr (requires(T&& input) { { *std::forward<T>(input) } -> std::convertible_to<std::string_view>; }) {
        return std::string_view(*std::forward<T>(value));
    } else if constexpr (requires(T&& input) { std::forward<T>(input).as_str(); }) {
        auto text = std::forward<T>(value).as_str();
        if constexpr (requires { text.is_some(); text.unwrap(); }) {
            return text.is_some() ? std::string_view(text.unwrap()) : std::string_view();
        } else {
            return std::string_view(text);
        }
    } else {
        return std::string_view(std::forward<T>(value));
    }
}

template<typename T>
requires requires(T& pointee) { to_string_view(pointee); }
std::string_view to_string_view(T* value) {
    return value ? to_string_view(*value) : std::string_view();
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

#if !defined(RUSTY_NO_STD_VECTOR_INTEROP)
template<typename T, typename Alloc>
Vec<T> into_vec(std::vector<T, Alloc> values) {
    Vec<T> out(values.size());
    for (auto& value : values) {
        out.push(std::move(value));
    }
    return out;
}
#endif

template<typename T>
constexpr std::decay_t<T> into_vec(T&& value) {
    return std::forward<T>(value);
}

} // namespace boxed

} // namespace rusty
