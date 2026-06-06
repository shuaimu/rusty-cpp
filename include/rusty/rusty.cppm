module;

// Umbrella module for the rusty library. The design:
//
//  * The hand-written `<rusty/*.hpp>` headers attach to the global
//    module via the GMF (Global Module Fragment) below. Importers
//    `#include <rusty/*.hpp>` in their own GMFs for the types and
//    utility helpers; this module re-exports nothing from those —
//    it just brings them into the umbrella's TU so its in-purview
//    aliases (Vec, Rc, BTreeMap, …) can name `::rusty::alloc::Global`
//    and other GMF-visible decls.
//
//  * The transpiled rustc-stdlib ports (`vec_port`, `rc_port`,
//    `btree_port`, `hashbrown_port`, …) live in real C++20 module
//    purviews. Consumers reach them through `import rusty;` thanks to
//    the `export import …;` block immediately after the module
//    declaration.
//
//  * The aliases inside `export namespace rusty { … }` below bind
//    each transpiled-port type to its expected `rusty::…` name so
//    callers can keep writing `rusty::Rc<T>` / `rusty::BTreeMap<K,V>`
//    after the legacy hand-written types retired.
//
// We do NOT include `<rusty/*.hpp>` inside an `export { … }` block in
// module purview — that would attach every std type the rusty headers
// touch (`std::shared_mutex`, `std::basic_filebuf`, …) to module rusty
// and conflict with importers' own GMFs (Clang 21+: "declaration of
// 'X' in module rusty follows declaration in the global module").

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

// Rusty hand-written headers in GMF (not module purview). Importers
// include these themselves; we pull them in here only so the
// in-purview aliases below can name `::rusty::alloc::Global`, etc.
#include <rusty/rusty.hpp>

export module rusty;

// C++20: every `export import …;` declaration must appear in the
// import section, immediately following the module declaration and
// preceding any other declarations. These re-export the transpiled
// rustc-stdlib ports plus the hand-written `rusty.async` module so
// `import rusty;` exposes the full surface.
export import rusty.async;
export import vec_port.vec;
export import btree_port.btree.map;  // namespace: btree_port::btree::map
export import btree_port.btree.set;  // namespace: btree_port::btree::set
export import rc_port;
export import binary_heap_port;  // exports rusty::collections::BinaryHeap alias
export import hashbrown_port.hasher;  // exports rusty::port::collections::hashbrown::DefaultHasher
export import hashbrown_port.map;
export import hashbrown_port.set;
export import vec_deque_port;  // exports rusty::collections::VecDeque alias
export import linked_list_port;  // exports rusty::collections::LinkedList alias
export import cell_port;
export import string_port;
export import arc_port;

// Some runtime templates rely on placement-new/delete lookup in importers.
export using ::operator new;
export using ::operator delete;

// `rusty::Vec`, `rusty::Rc`, `rusty::BTreeMap`, … aliases. These bind
// the transpiled-port types into the `rusty::` namespace so consumers
// write `rusty::Vec<T>` / `rusty::Rc<T>` / `rusty::BTreeMap<K,V>` and
// match Rust's `std::vec::Vec` / `std::rc::Rc` / `std::collections::*`
// naming.
//
// Helper templates (`rusty::default_value`, `rusty::to_string_view`,
// `rusty::to_owned`, `rusty::sanitize_array_capacity`, …) are NOT
// re-declared here — they live in `<rusty/rusty.hpp>` (GMF-attached
// via the include above) and importers reach them through their own
// `#include <rusty/*.hpp>` chain.
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
//
// The deep-namespace migration (`--cxx-namespace` transpile flag)
// moved btree's transpiled output from `rusty::port::collections::btree::*`
// to its module-name root `btree_port::btree::*`. Aliases below point
// at the new path.
template<typename K, typename V, typename A = ::rusty::alloc::Global>
using BTreeMap = ::btree_port::btree::map::BTreeMap<K, V, A>;

template<typename T, typename A = ::rusty::alloc::Global>
using BTreeSet = ::btree_port::btree::set::BTreeSet<T, A>;

// rusty::HashMap / rusty::HashSet alias the transpiled rustc hashbrown
// port. These match Rust's `std::collections::HashMap` / `HashSet` at
// the top level. The `rusty::collections::HashMap` / `HashSet` aliases
// (declared below) are kept for code that prefers the namespaced form.
template<typename K, typename V,
         typename S = ::rusty::port::collections::hashbrown::DefaultHasher>
using HashMap = ::rusty::port::collections::hashbrown::HashMap<K, V, S>;

template<typename T,
         typename S = ::rusty::port::collections::hashbrown::DefaultHasher>
using HashSet = ::rusty::port::collections::hashbrown::HashSet<T, S>;

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

// `rusty::rc::Weak<T, A>` and `rusty::rc::Rc<T, A>` aliases live in
// rc_port.cppm itself (`export namespace rusty::rc { using Rc = …; }`)
// and reach importers via the `export import rc_port;` above.

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
    using BTreeSet = ::btree_port::btree::set::BTreeSet<T, A>;
    // BinaryHeap / VecDeque / LinkedList — the transpiled ports each
    // emit their own `export namespace rusty::collections { using X = …; }`
    // alias (see binary_heap_port.cppm, vec_deque_port.cppm,
    // linked_list_port.cppm). The `export import <port>;` lines above
    // bring those aliases into scope here; re-declaring them at this
    // scope would conflict (clang: "declaration of 'X' in module rusty
    // follows declaration in module <port>").
}

} // export namespace rusty
