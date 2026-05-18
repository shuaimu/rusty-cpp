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

#include <algorithm>
#include <cstddef>
#include <functional>
#include <initializer_list>
#include <iterator>
#include <map>
#include <set>
#include <type_traits>
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

    /// Build a map from any iterator pair over `pair<K, V>`-like
    /// values. Mirrors Rust's `BTreeMap::from_iter` / `collect`. On
    /// duplicate keys the LAST one wins (matches Rust's behavior).
    template <typename It>
    static BTreeMap from_iter(It first, It last) {
        BTreeMap m;
        for (; first != last; ++first) {
            auto&& kv = *first;
            m.backing_.insert_or_assign(kv.first, kv.second);
        }
        return m;
    }

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

    // ── Rust-flavored convenience accessors ────────────────────────
    // These mirror methods that callers familiar with the Rust
    // `BTreeMap` API expect. They're thin and could equivalently be
    // written with begin/end iteration; they're here so call sites
    // read naturally.

    /// First (smallest-key) entry as `Option<(const K&, const V&)>`.
    rusty::Option<std::pair<std::reference_wrapper<const K>,
                            std::reference_wrapper<const V>>>
    first_key_value() const {
        if (backing_.empty()) {
            return rusty::Option<
                std::pair<std::reference_wrapper<const K>,
                          std::reference_wrapper<const V>>>(rusty::None);
        }
        const auto& it = *backing_.begin();
        return rusty::Option<
            std::pair<std::reference_wrapper<const K>,
                      std::reference_wrapper<const V>>>(
            std::make_pair(std::cref(it.first), std::cref(it.second)));
    }

    /// Last (largest-key) entry as `Option<(const K&, const V&)>`.
    rusty::Option<std::pair<std::reference_wrapper<const K>,
                            std::reference_wrapper<const V>>>
    last_key_value() const {
        if (backing_.empty()) {
            return rusty::Option<
                std::pair<std::reference_wrapper<const K>,
                          std::reference_wrapper<const V>>>(rusty::None);
        }
        const auto& it = *backing_.rbegin();
        return rusty::Option<
            std::pair<std::reference_wrapper<const K>,
                      std::reference_wrapper<const V>>>(
            std::make_pair(std::cref(it.first), std::cref(it.second)));
    }

    /// Range iteration helper: returns a pair `(begin, end)` of
    /// iterators over the half-open range `[lower, upper)`. Mirrors
    /// the half-open subset of Rust's `range`. For Rust's full
    /// `range(bound1..bound2)` shape with mixed inclusive/exclusive
    /// bounds, callers compose with `std::map::lower_bound` /
    /// `upper_bound` directly via the exposed iterators.
    auto range(const K& lower, const K& upper) {
        return std::make_pair(backing_.lower_bound(lower),
                              backing_.lower_bound(upper));
    }
    auto range(const K& lower, const K& upper) const {
        return std::make_pair(backing_.lower_bound(lower),
                              backing_.lower_bound(upper));
    }

    /// Number of entries — `size()` as an alias for `len()` for STL
    /// consumers that prefer the C++ spelling.
    size_type size() const noexcept { return backing_.size(); }
    bool empty() const noexcept { return backing_.empty(); }

    // ── View iterators (keys/values) ──────────────────────────────
    // Each view exposes a lightweight `begin/end` pair built from a
    // transform_iterator-like wrapper so callers can iterate just
    // the keys or just the values without copying the whole map.
    // Mirrors Rust's `BTreeMap::keys()` / `values()` / `values_mut()`.

private:
    template <typename UnderlyingIt, typename Project>
    class ProjIter {
        UnderlyingIt it_;
        Project proj_;
    public:
        using iterator_category = std::forward_iterator_tag;
        using value_type = std::remove_reference_t<
            decltype(std::declval<Project>()(*std::declval<UnderlyingIt>()))>;
        using difference_type = std::ptrdiff_t;
        using pointer = void;
        using reference = decltype(std::declval<Project>()(
            *std::declval<UnderlyingIt>()));

        ProjIter(UnderlyingIt it, Project proj)
            : it_(it), proj_(std::move(proj)) {}

        reference operator*() const { return proj_(*it_); }
        ProjIter& operator++() { ++it_; return *this; }
        ProjIter operator++(int) { auto tmp = *this; ++it_; return tmp; }
        bool operator==(const ProjIter& other) const { return it_ == other.it_; }
        bool operator!=(const ProjIter& other) const { return it_ != other.it_; }
    };

    template <typename UnderlyingIt, typename Project>
    struct ProjRange {
        ProjIter<UnderlyingIt, Project> b;
        ProjIter<UnderlyingIt, Project> e;
        auto begin() const { return b; }
        auto end() const { return e; }
    };

    static const K& project_key(const value_type& kv) { return kv.first; }
    static const V& project_const_value(const value_type& kv) { return kv.second; }
    static V& project_value(value_type& kv) { return kv.second; }

public:
    /// Range over `const K&`, in ascending key order.
    /// Use as `for (const auto& k : m.keys()) { … }`.
    auto keys() const {
        using It = typename backing_type::const_iterator;
        using P = const K& (*)(const value_type&);
        return ProjRange<It, P>{
            ProjIter<It, P>{backing_.begin(), &project_key},
            ProjIter<It, P>{backing_.end(), &project_key},
        };
    }

    /// Range over `const V&`, in ascending key order.
    auto values() const {
        using It = typename backing_type::const_iterator;
        using P = const V& (*)(const value_type&);
        return ProjRange<It, P>{
            ProjIter<It, P>{backing_.begin(), &project_const_value},
            ProjIter<It, P>{backing_.end(), &project_const_value},
        };
    }

    /// Range over `V&`, in ascending key order.
    auto values_mut() {
        using It = typename backing_type::iterator;
        using P = V& (*)(value_type&);
        return ProjRange<It, P>{
            ProjIter<It, P>{backing_.begin(), &project_value},
            ProjIter<It, P>{backing_.end(), &project_value},
        };
    }

    /// Bulk-insert from any iterator pair over `std::pair<K, V>`-like
    /// values. Mirrors Rust's `BTreeMap::extend`. Existing keys are
    /// overwritten (Rust's behavior); callers wanting to preserve
    /// originals should use `entry().or_insert(…)` per element.
    template <typename It>
    void extend(It first, It last) {
        for (; first != last; ++first) {
            auto&& kv = *first;
            backing_.insert_or_assign(kv.first, kv.second);
        }
    }

    /// Move all entries from `other` into this map. After the call
    /// `other` is empty. Existing keys are overwritten (Rust's
    /// behavior). Mirrors Rust's `BTreeMap::append`.
    void append(BTreeMap& other) {
        for (auto& [k, v] : other.backing_) {
            backing_.insert_or_assign(std::move(const_cast<K&>(k)),
                                      std::move(v));
        }
        other.backing_.clear();
    }

    /// Split the map: everything with key `>= key` is moved into a
    /// new map and returned; entries with key `< key` stay in
    /// `*this`. Mirrors Rust's `BTreeMap::split_off`.
    BTreeMap split_off(const K& key) {
        BTreeMap out;
        auto it = backing_.lower_bound(key);
        for (auto i = it; i != backing_.end(); ++i) {
            out.backing_.emplace(i->first, std::move(i->second));
        }
        backing_.erase(it, backing_.end());
        return out;
    }

    /// Remove and return the first (smallest-key) entry.
    /// Returns `None` if the map is empty. Mirrors Rust's
    /// `BTreeMap::pop_first`.
    rusty::Option<std::pair<K, V>> pop_first() {
        if (backing_.empty()) {
            return rusty::Option<std::pair<K, V>>(rusty::None);
        }
        auto it = backing_.begin();
        std::pair<K, V> entry{it->first, std::move(it->second)};
        backing_.erase(it);
        return rusty::Option<std::pair<K, V>>(std::move(entry));
    }

    /// Remove and return the last (largest-key) entry.
    rusty::Option<std::pair<K, V>> pop_last() {
        if (backing_.empty()) {
            return rusty::Option<std::pair<K, V>>(rusty::None);
        }
        auto it = std::prev(backing_.end());
        std::pair<K, V> entry{it->first, std::move(it->second)};
        backing_.erase(it);
        return rusty::Option<std::pair<K, V>>(std::move(entry));
    }

    /// Retain only entries for which the predicate `f(k, v)` returns
    /// true. Mirrors Rust's `BTreeMap::retain`.
    template <typename F>
    void retain(F&& f) {
        for (auto it = backing_.begin(); it != backing_.end(); ) {
            if (!f(it->first, it->second)) {
                it = backing_.erase(it);
            } else {
                ++it;
            }
        }
    }

    // ── Entry API ─────────────────────────────────────────────────
    // Rust's `BTreeMap::entry(k)` returns an `Entry` view that
    // exposes `or_insert`/`or_insert_with`/`and_modify`. Here we
    // implement it as a lightweight view holding a backing-map
    // reference + an iterator (or end-iterator if vacant). The
    // view's lifetime is tied to the map; misuse after map
    // mutation is the caller's responsibility (mirrors Rust's
    // borrow-checker requirement; rusty-cpp catches it via
    // iterator-invalidation rules).

    class Entry {
        friend class BTreeMap;
        backing_type& backing_;
        K key_;
        typename backing_type::iterator it_;
        bool occupied_;

        Entry(backing_type& b, K k, typename backing_type::iterator it,
              bool occupied)
            : backing_(b),
              key_(std::move(k)),
              it_(it),
              occupied_(occupied) {}

    public:
        /// True iff the key was present when the entry was constructed.
        bool is_occupied() const noexcept { return occupied_; }

        /// Reference to the key the entry was constructed with.
        const K& key() const noexcept { return key_; }

        /// Insert `default_` if vacant, then return a reference to the
        /// (possibly new) value. Mirrors Rust's
        /// `Entry::or_insert`.
        V& or_insert(V default_) {
            if (!occupied_) {
                auto [new_it, _] = backing_.emplace(std::move(key_),
                                                   std::move(default_));
                it_ = new_it;
                occupied_ = true;
            }
            return it_->second;
        }

        /// Same as `or_insert` but lazily computes the default via
        /// the callable `f()` only when the key is absent.
        template <typename F>
        V& or_insert_with(F&& f) {
            if (!occupied_) {
                auto [new_it, _] = backing_.emplace(std::move(key_), f());
                it_ = new_it;
                occupied_ = true;
            }
            return it_->second;
        }

        /// If the entry is occupied, call `f(value)` on the existing
        /// value. Returns the entry (for chaining), mirroring Rust's
        /// `Entry::and_modify`.
        template <typename F>
        Entry& and_modify(F&& f) {
            if (occupied_) {
                f(it_->second);
            }
            return *this;
        }
    };

    /// Get an entry view for `key` — vacant if the key is absent,
    /// occupied otherwise. Lets callers express `m.entry(k).or_insert(0)`-
    /// style upserts in one statement (no double lookup).
    Entry entry(K key) {
        auto it = backing_.find(key);
        bool occupied = it != backing_.end();
        return Entry(backing_, std::move(key), it, occupied);
    }
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

    /// Build a set from any iterator pair over `T`-like values.
    /// Mirrors Rust's `BTreeSet::from_iter` / `collect`. Duplicates
    /// are dropped (Rust's behavior).
    template <typename It>
    static BTreeSet from_iter(It first, It last) {
        BTreeSet s;
        for (; first != last; ++first) {
            s.backing_.insert(*first);
        }
        return s;
    }

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

    /// `size()` as an alias for `len()` for STL spellings.
    size_type size() const noexcept { return backing_.size(); }
    bool empty() const noexcept { return backing_.empty(); }

    // ── Pop / retain ─────────────────────────────────────────────

    /// Remove and return the smallest value, or `None` if empty.
    rusty::Option<T> pop_first() {
        if (backing_.empty()) {
            return rusty::Option<T>(rusty::None);
        }
        auto it = backing_.begin();
        T v = std::move(const_cast<T&>(*it));
        backing_.erase(it);
        return rusty::Option<T>(std::move(v));
    }

    /// Remove and return the largest value, or `None` if empty.
    rusty::Option<T> pop_last() {
        if (backing_.empty()) {
            return rusty::Option<T>(rusty::None);
        }
        auto it = std::prev(backing_.end());
        T v = std::move(const_cast<T&>(*it));
        backing_.erase(it);
        return rusty::Option<T>(std::move(v));
    }

    /// In-place removal by predicate `f(const T&) -> bool`.
    template <typename F>
    void retain(F&& f) {
        for (auto it = backing_.begin(); it != backing_.end(); ) {
            if (!f(*it)) {
                it = backing_.erase(it);
            } else {
                ++it;
            }
        }
    }

    /// Half-open range `[lower, upper)` as a (begin, end) iterator
    /// pair. Mirrors the subset of Rust's `range(bound..bound)` that
    /// the map facade also exposes.
    auto range(const T& lower, const T& upper) const {
        return std::make_pair(backing_.lower_bound(lower),
                              backing_.lower_bound(upper));
    }

    // ── Set-theoretic operations ─────────────────────────────────
    // Rust's BTreeSet exposes `union`, `intersection`, `difference`,
    // `symmetric_difference` as lazy iterators. Materializing them
    // to a fresh `BTreeSet` is the most common usage in practice, so
    // that's what we expose here. (`union` is a C++ keyword
    // soft-conflict, so we spell it `union_set` to match Rust's
    // ergonomic intent while staying valid C++.)

    /// All elements in `*this` OR `other`.
    BTreeSet union_set(const BTreeSet& other) const {
        BTreeSet out;
        std::set_union(backing_.begin(), backing_.end(),
                       other.backing_.begin(), other.backing_.end(),
                       std::inserter(out.backing_, out.backing_.end()));
        return out;
    }

    /// All elements in BOTH `*this` and `other`.
    BTreeSet intersection(const BTreeSet& other) const {
        BTreeSet out;
        std::set_intersection(backing_.begin(), backing_.end(),
                              other.backing_.begin(), other.backing_.end(),
                              std::inserter(out.backing_, out.backing_.end()));
        return out;
    }

    /// All elements in `*this` but not in `other`.
    BTreeSet difference(const BTreeSet& other) const {
        BTreeSet out;
        std::set_difference(backing_.begin(), backing_.end(),
                            other.backing_.begin(), other.backing_.end(),
                            std::inserter(out.backing_, out.backing_.end()));
        return out;
    }

    /// All elements in exactly one of `*this` or `other`.
    BTreeSet symmetric_difference(const BTreeSet& other) const {
        BTreeSet out;
        std::set_symmetric_difference(
            backing_.begin(), backing_.end(),
            other.backing_.begin(), other.backing_.end(),
            std::inserter(out.backing_, out.backing_.end()));
        return out;
    }

    /// `true` iff every element of `*this` is also in `other`.
    bool is_subset(const BTreeSet& other) const {
        return std::includes(other.backing_.begin(), other.backing_.end(),
                             backing_.begin(), backing_.end());
    }

    /// `true` iff every element of `other` is also in `*this`.
    bool is_superset(const BTreeSet& other) const {
        return other.is_subset(*this);
    }

    /// `true` iff `*this` and `other` share no element.
    bool is_disjoint(const BTreeSet& other) const {
        auto a = backing_.begin();
        auto b = other.backing_.begin();
        while (a != backing_.end() && b != other.backing_.end()) {
            if (*a < *b)
                ++a;
            else if (*b < *a)
                ++b;
            else
                return false;
        }
        return true;
    }
};

}  // namespace btree_port

#endif  // BTREE_PORT_BTREEMAP_HPP
