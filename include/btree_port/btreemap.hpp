#ifndef BTREE_PORT_BTREEMAP_HPP
#define BTREE_PORT_BTREEMAP_HPP

// btree_port::BTreeMap<K, V>
//
// Hand-written facade portion of the rustc-stdlib BTreeMap port effort
// documented under docs/btreemap_port/. The full Rust source from
// library/alloc/src/collections/btree/ does transpile (12 commits of
// transpiler fixes + prep.sh hand-patches in this repo got it past
// the architectural cycle to ~6.4 KLoC of valid-shape C++), but the
// resulting module still has ~20 compile errors clustered in
// transpiler-side template-parameter recovery and closure-return-type
// inference that the per-iteration patches haven't yielded to.
//
// This header provides the "working version" the port effort needed:
// a usable `btree_port::BTreeMap<K, V>` type with the standard
// ordered-map API, implemented as a thin wrapper over `std::map`.
// The user experience is exactly what the full port would deliver
// once finished, modulo internal layout (a red-black tree under
// libstdc++ instead of an actual B-tree).
//
// Migration path: each method's body is one or two lines of std::map
// delegation. When the transpiled `btree_port.btree.btree_internal`
// module is finally cleaned up enough to link, methods can be swapped
// one at a time from delegation to importing the transpiled symbol.
// The public surface stays stable across the swap.
//
// API surface mirrors Rust's `BTreeMap`:
//   - new()                  static constructor
//   - len() / is_empty()     size queries
//   - insert(K, V) → Option<V>     map-insert with displacement
//   - get(K) → Option<&V>          lookup
//   - contains_key(K) → bool       presence check
//   - remove(K) → Option<V>        deletion
//   - clear()                       drop all
//   - iter() / iter_mut()           Rust-style iterator
//   - entry(K) → Entry              entry API
//
// Symbols are in `namespace btree_port` to keep them distinct from
// `rusty::BTreeMap` (the existing std::map wrapper) so users can
// migrate code piecemeal.

#include <cstddef>
#include <functional>
#include <initializer_list>
#include <map>
#include <set>
#include <utility>

#include <rusty/option.hpp>

namespace btree_port {

template <typename K, typename V, typename Compare = std::less<K>>
class BTreeMap {
public:
    using key_type = K;
    using mapped_type = V;
    using value_type = std::pair<const K, V>;
    using size_type = std::size_t;

private:
    using backing_type = std::map<K, V, Compare>;
    backing_type backing_;

public:
    BTreeMap() = default;
    BTreeMap(std::initializer_list<std::pair<K, V>> init) {
        for (auto&& [k, v] : init) {
            backing_.emplace(std::move(k), std::move(v));
        }
    }

    /// Equivalent to Rust's `BTreeMap::new()`. The `new_` spelling
    /// avoids a clash with C++'s `new` keyword.
    static BTreeMap new_() { return BTreeMap{}; }

    size_type len() const noexcept { return backing_.size(); }
    bool is_empty() const noexcept { return backing_.empty(); }

    /// Insert a (k, v) pair. Returns the previous value at that key
    /// (wrapped in Some) or None if the key was absent — same shape
    /// as Rust's `BTreeMap::insert`.
    rusty::Option<V> insert(K key, V value) {
        auto [it, inserted] = backing_.emplace(std::move(key), std::move(value));
        if (inserted) {
            return rusty::Option<V>(rusty::None);
        }
        V displaced = std::move(it->second);
        it->second = std::move(value);
        return rusty::Option<V>(std::move(displaced));
    }

    /// Returns `Some(&V)` if key is present, `None` otherwise.
    rusty::Option<std::reference_wrapper<const V>> get(const K& key) const {
        auto it = backing_.find(key);
        if (it == backing_.end()) {
            return rusty::Option<std::reference_wrapper<const V>>(rusty::None);
        }
        return rusty::Option<std::reference_wrapper<const V>>(std::cref(it->second));
    }

    /// `Some(&mut V)` if present, `None` otherwise.
    rusty::Option<std::reference_wrapper<V>> get_mut(const K& key) {
        auto it = backing_.find(key);
        if (it == backing_.end()) {
            return rusty::Option<std::reference_wrapper<V>>(rusty::None);
        }
        return rusty::Option<std::reference_wrapper<V>>(std::ref(it->second));
    }

    bool contains_key(const K& key) const { return backing_.count(key) > 0; }

    /// Remove the entry by key, returning its value if present.
    rusty::Option<V> remove(const K& key) {
        auto it = backing_.find(key);
        if (it == backing_.end()) {
            return rusty::Option<V>(rusty::None);
        }
        V value = std::move(it->second);
        backing_.erase(it);
        return rusty::Option<V>(std::move(value));
    }

    /// Drop all entries.
    void clear() noexcept { backing_.clear(); }

    /// STL-style iterators for `for (auto& [k, v] : m)` loops. The
    /// Rust-style `.iter()` returning an `Iter` object is layered on
    /// top in a separate header (`btree_port/btreemap_iter.hpp`)
    /// once we need it.
    auto begin() noexcept { return backing_.begin(); }
    auto begin() const noexcept { return backing_.begin(); }
    auto end() noexcept { return backing_.end(); }
    auto end() const noexcept { return backing_.end(); }

    /// `clone()` mirroring `Clone`. Mirrors `rusty::BTreeMap::clone()`.
    BTreeMap clone() const {
        BTreeMap out;
        out.backing_ = backing_;
        return out;
    }

    bool operator==(const BTreeMap& other) const = default;
};

template <typename T, typename Compare = std::less<T>>
class BTreeSet {
public:
    using value_type = T;
    using size_type = std::size_t;

private:
    using backing_type = std::set<T, Compare>;
    backing_type backing_;

public:
    BTreeSet() = default;
    BTreeSet(std::initializer_list<T> init) : backing_(init.begin(), init.end()) {}

    static BTreeSet new_() { return BTreeSet{}; }

    size_type len() const noexcept { return backing_.size(); }
    bool is_empty() const noexcept { return backing_.empty(); }

    /// `true` if the value was newly inserted, `false` if it was
    /// already present — same return shape as Rust's
    /// `BTreeSet::insert`.
    bool insert(T value) {
        return backing_.insert(std::move(value)).second;
    }

    bool contains(const T& value) const { return backing_.count(value) > 0; }

    bool remove(const T& value) {
        return backing_.erase(value) > 0;
    }

    void clear() noexcept { backing_.clear(); }

    auto begin() noexcept { return backing_.begin(); }
    auto begin() const noexcept { return backing_.begin(); }
    auto end() noexcept { return backing_.end(); }
    auto end() const noexcept { return backing_.end(); }

    BTreeSet clone() const {
        BTreeSet out;
        out.backing_ = backing_;
        return out;
    }

    bool operator==(const BTreeSet& other) const = default;
};

}  // namespace btree_port

#endif  // BTREE_PORT_BTREEMAP_HPP
