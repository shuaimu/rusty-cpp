// Hand-translated bodies for rustc btree/{map,set}/tests.rs tests
// that we've moved from "skip" to "real exercise" of btree_port.
//
// Why a separate TU instead of putting these in btree_tests_port.cppm:
// when BTreeMap<int,int> is instantiated inside the module purview
// (i.e. inside the .cppm itself), the destructor path hits a static
// assert — `ManuallyDrop<Global>` ends up being clone()d in the drop
// codegen and `is_copy_constructible_v<ManuallyDrop<Global>>` is
// false. Same instantiation from a regular .cpp that imports the
// module compiles fine (the existing btree_port_module_test.out works
// this way). Until the in-module-purview instantiation bug is fixed,
// translated test bodies live here.
//
// Test name convention: `<rust_test_name>_unstubbed` so registration
// doesn't collide with the corresponding stub in btree_tests_port.cppm.

import btree_port.btree.map;
import btree_port.btree.set;

#include <cassert>
#include <cstdio>
#include <tuple>
#include <utility>
#include <rusty/alloc.hpp>
#include <rusty/test_runner.hpp>

namespace {

template<typename K, typename V>
using BTreeMap = ::btree_port::btree::map::BTreeMap<K, V, ::rusty::alloc::Global>;
template<typename T>
using BTreeSet = ::btree_port::btree::set::BTreeSet<T, ::rusty::alloc::Global>;

template<typename K, typename V> auto make_map() {
    return BTreeMap<K, V>::new_in(::rusty::alloc::Global{});
}
template<typename T> auto make_set() {
    return BTreeSet<T>::new_in(::rusty::alloc::Global{});
}

// `.check()` shim. The original rustc tests define an `impl<K,V> BTreeMap`
// block in tests.rs adding a private `check()` method that walks the
// navigation internals and asserts invariants (back-pointers, calc_length,
// min_len). Those internals aren't exposed by btree_port, so calls to
// `map.check()` in translated tests are routed through this no-op. We
// lose internal-invariant validation but keep all the public-API
// assertions the test itself makes.
template<typename M> inline void check(const M&) {}

} // anonymous

// ─────────────────────────────────────────────────────────────────────
// rustc map/tests.rs::test_get_key_value (trimmed)
// Full Rust source also exercises map.remove + post-remove checks; the
// remove call triggers a stale-codegen bug in btree_port — see
// docs/btree_tests_port/STATUS.md "ManuallyDrop<Global>::clone".
// ─────────────────────────────────────────────────────────────────────
TEST_CASE("test_get_key_value_unstubbed") {
    auto map = make_map<int, int>();

    assert(map.is_empty());

    map.insert(1, 10);
    map.insert(2, 20);
    map.insert(3, 30);

    assert(map.len() == 3);
    {
        auto kv = map.get_key_value(1);
        assert(kv.is_some());
        auto t = std::move(kv).unwrap();
        assert(std::get<0>(t) == 1);
        assert(std::get<1>(t) == 10);
    }
    {
        auto kv = map.get_key_value(3);
        assert(kv.is_some());
        auto t = std::move(kv).unwrap();
        assert(std::get<0>(t) == 3);
        assert(std::get<1>(t) == 30);
    }
    assert(map.get_key_value(4).is_none());
}

// rustc map/tests.rs::test_pop_first_last partially un-stubbed: only
// the pop_first path. pop_last hits B-pop-last — runtime aborts inside
// `last_entry()` / `OccupiedEntry::remove_entry()` chain. `last_key_value`
// works (purely const, no dormant_map). `pop_first` works (calls into
// `first_entry`/`first_leaf_edge`/`right_kv`). The asymmetric failure
// makes it look like a bug in `last_entry` / `last_leaf_edge` /
// `left_kv` / `remove_entry` somewhere on the rightmost-walk path.
// Deferred — needs deeper investigation.
TEST_CASE("test_pop_first_only_unstubbed") {
    auto map = make_map<int, int>();
    assert(map.pop_first().is_none());

    map.insert(1, 10);
    map.insert(2, 20);
    map.insert(3, 30);
    map.insert(4, 40);
    assert(map.len() == 4);

    {
        auto kv = map.pop_first();
        assert(kv.is_some());
        auto t = std::move(kv).unwrap();
        assert(std::get<0>(t) == 1);
        assert(std::get<1>(t) == 10);
        assert(map.len() == 3);
    }
    {
        auto kv = map.pop_first();
        assert(kv.is_some());
        auto t = std::move(kv).unwrap();
        assert(std::get<0>(t) == 2);
        assert(std::get<1>(t) == 20);
        assert(map.len() == 2);
    }
}

// ─────────────────────────────────────────────────────────────────────
// rustc map/tests.rs::test_try_insert
// Re-enabled by fix_btreemap_try_insert_arm_swap patcher rule.
// ─────────────────────────────────────────────────────────────────────
TEST_CASE("test_try_insert_unstubbed") {
    auto map = make_map<int, int>();
    assert(map.is_empty());

    {
        auto r = map.try_insert(1, 10);
        assert(r.is_ok());
        assert(std::move(r).unwrap() == 10);
    }
    {
        auto r = map.try_insert(2, 20);
        assert(r.is_ok());
        assert(std::move(r).unwrap() == 20);
    }
    {
        // Already-occupied → Err with the value we tried to insert.
        auto r = map.try_insert(2, 200);
        assert(r.is_err());
        auto err = std::move(r).unwrap_err();
        assert(err.value == 200);
    }
    // Verify the original (2, 20) entry was not overwritten.
    auto v = map.get(2);
    assert(v.is_some());
    assert(v.unwrap() == 20);
}

// ─────────────────────────────────────────────────────────────────────
// rustc set/tests.rs::test_clear
// Re-enabled by fix_btreemap_clear_manuallydrop patcher rule.
// ─────────────────────────────────────────────────────────────────────
TEST_CASE("set_test_clear_unstubbed") {
    auto x = make_set<int>();
    x.insert(1);
    x.clear();
    assert(x.is_empty());
}

// test_into_keys / test_into_values still BLOCKED. B-clear is fixed
// (which made BTreeMap::clear work), but into_keys/into_values hit a
// SEPARATE ManuallyDrop bug: at map.cppm:5922 the emit accesses
// `this->root` / `this->length` on a `ManuallyDrop<BTreeMap>` directly,
// without dereferencing through the ManuallyDrop wrapper. Needs an
// auto-deref fix in the transpiler's emit-side ManuallyDrop handling.

// ─────────────────────────────────────────────────────────────────────
// Basic smoke test combining insert / contains_key / get / len.
// Closest single-test equivalent of the omitted test_basic_small —
// covers similar surface but without the .check() invariant call.
// Not a 1:1 rustc test translation; included to cover the read path
// while the bigger tests are blocked.
// ─────────────────────────────────────────────────────────────────────
TEST_CASE("smoke_insert_lookup_unstubbed") {
    auto map = make_map<int, int>();
    assert(map.is_empty());
    assert(map.len() == 0);
    assert(!map.contains_key(0));

    // First insert returns None (no displaced value).
    assert(map.insert(1, 100).is_none());
    assert(!map.is_empty());
    assert(map.len() == 1);
    assert(map.contains_key(1));
    assert(!map.contains_key(2));

    // Re-insert returns Some(old).
    {
        auto displaced = map.insert(1, 200);
        assert(displaced.is_some());
        assert(std::move(displaced).unwrap() == 100);
    }
    assert(map.len() == 1);

    // get() returns the current value.
    {
        auto v = map.get(1);
        assert(v.is_some());
        assert(v.unwrap() == 200);
    }
    // get() on absent key returns None.
    assert(map.get(99).is_none());

    // Several more inserts + first/last_key_value.
    map.insert(2, 20);
    map.insert(3, 30);
    map.insert(0, 0);
    assert(map.len() == 4);
    {
        auto first = map.first_key_value();
        assert(first.is_some());
        auto t = std::move(first).unwrap();
        assert(std::get<0>(t) == 0);
        assert(std::get<1>(t) == 0);
    }
    {
        auto last = map.last_key_value();
        assert(last.is_some());
        auto t = std::move(last).unwrap();
        assert(std::get<0>(t) == 3);
        assert(std::get<1>(t) == 30);
    }
}
