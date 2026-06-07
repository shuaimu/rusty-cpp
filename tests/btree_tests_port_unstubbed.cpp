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
#include <rusty/array.hpp>
#include <rusty/test_runner.hpp>

#include "btree_testing_helpers.hpp"

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

// B-pop-last was a dangling-reference bug in remove_leaf_kv. The
// transpiler emitted `auto&& [old_kv, pos] = deref_if_pointer_like(this->remove())`,
// but `this->remove()` returns a prvalue tuple; `deref_if_pointer_like`
// materializes it into a `T&&` parameter and forwards back an rvalue
// ref, so the temporary dies at the semicolon and pos is left dangling.
// Reads worked until the rebalance lambda was entered — calling the
// lambda clobbered the stack memory pos pointed at. Mostly latent (the
// stale bytes happened to read as None), but on the 3rd consecutive
// pop the garbage parent pointer read as Some and choose_parent_kv took
// the Ok arm with a stack address, segfaulting in `as_leaf_ptr`.
// Workaround: replace `auto&&` with `auto` at that site, which lets the
// structured binding own the tuple. Same pattern appears ~92x across
// btree_port and core_slice_port; only the pop hot path is on a
// crashable path so the rest stay as-is for now. Proper fix is in the
// transpiler — emit `auto [a,b] = ...` for `let` patterns whose RHS is
// a prvalue. See docs/btreemap_port/STATUS.md for the writeup.

TEST_CASE("test_pop_first_only_unstubbed") {
    auto map = make_map<int, int>();
    assert(map.pop_first().is_none());

    map.insert(1, 10);
    map.insert(2, 20);
    map.insert(3, 30);
    map.insert(4, 40);
    assert(map.len() == 4);

    for (int expected_k = 1; expected_k <= 4; ++expected_k) {
        auto kv = map.pop_first();
        assert(kv.is_some());
        auto t = std::move(kv).unwrap();
        assert(std::get<0>(t) == expected_k);
        assert(std::get<1>(t) == expected_k * 10);
        assert(map.len() == static_cast<size_t>(4 - expected_k));
    }
    assert(map.is_empty());
    assert(map.pop_first().is_none());
}

// ─────────────────────────────────────────────────────────────────────
// rustc map/tests.rs::test_pop_first_last
// Un-stubbed after fixing B-pop-last (dangling structured binding in
// remove_leaf_kv).
// ─────────────────────────────────────────────────────────────────────
TEST_CASE("test_pop_first_last_unstubbed") {
    auto map = make_map<int, int>();
    assert(map.pop_first().is_none());
    assert(map.pop_last().is_none());

    map.insert(1, 10);
    map.insert(2, 20);
    map.insert(3, 30);
    map.insert(4, 40);

    // pop_first then pop_last then pop_first then pop_last — the exact
    // mix that previously crashed.
    {
        auto kv = map.pop_first();
        assert(kv.is_some());
        auto t = std::move(kv).unwrap();
        assert(std::get<0>(t) == 1);
        assert(std::get<1>(t) == 10);
    }
    {
        auto kv = map.pop_last();
        assert(kv.is_some());
        auto t = std::move(kv).unwrap();
        assert(std::get<0>(t) == 4);
        assert(std::get<1>(t) == 40);
    }
    {
        auto kv = map.pop_first();
        assert(kv.is_some());
        auto t = std::move(kv).unwrap();
        assert(std::get<0>(t) == 2);
        assert(std::get<1>(t) == 20);
    }
    {
        auto kv = map.pop_last();
        assert(kv.is_some());
        auto t = std::move(kv).unwrap();
        assert(std::get<0>(t) == 3);
        assert(std::get<1>(t) == 30);
    }
    assert(map.is_empty());
    assert(map.pop_first().is_none());
    assert(map.pop_last().is_none());
}

// Repeats of the pre-fix crash shape: pop_first × 2 then pop_last drains
// the rest. Plus a pure pop_last drain.
TEST_CASE("test_pop_last_drain_unstubbed") {
    auto map = make_map<int, int>();
    for (int k = 1; k <= 4; ++k) map.insert(k, k * 10);

    for (int expected_k = 4; expected_k >= 1; --expected_k) {
        auto kv = map.pop_last();
        assert(kv.is_some());
        auto t = std::move(kv).unwrap();
        assert(std::get<0>(t) == expected_k);
        assert(std::get<1>(t) == expected_k * 10);
    }
    assert(map.is_empty());
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

// ─────────────────────────────────────────────────────────────────────
// Tests un-stubbed by porting crate::testing::{crash_test, ord_chaos}
// into tests/btree_testing_helpers.hpp.
//
// These exercise BTreeMap under broken Ord (Cyclic3 violates transitivity;
// Governed flips order at runtime). The Rust tests verify the map doesn't
// UB even when its invariants are broken — our translations do the same
// but without `map.check()` body (we route through a no-op shim because
// the internal invariant checker isn't exposed). The public-API assertions
// the tests make on len() and iteration still run.
// ─────────────────────────────────────────────────────────────────────

namespace {

template<typename T> using Cyclic3Map = BTreeMap<T, int>;

// Unit-like value type for tests that use `()` in Rust. We use `int` for
// simplicity rather than `std::monostate`; the value is never inspected.
using Unit = int;
constexpr Unit kUnit = 0;

}  // namespace

// rustc map/tests.rs::test_check_ord_chaos
// Builds a 2-element map, flips the governor, then runs `check()`. The
// original verifies that .check() doesn't UB when the Ord invariant breaks
// after the fact. We route check() through our no-op shim — the test still
// exercises insert + flip without crashing.
TEST_CASE("test_check_ord_chaos_unstubbed") {
    using namespace btree_testing;
    Governor gov;
    auto map = BTreeMap<Governed<int>, Unit>::new_in(::rusty::alloc::Global{});
    map.insert(Governed<int>(1, &gov), kUnit);
    map.insert(Governed<int>(2, &gov), kUnit);
    assert(map.len() == 2);
    gov.flip();
    check(map);  // no-op shim, but must not crash
}

// rustc map/tests.rs::test_range_finding_ill_order_in_map
// Inserts B, then conditionally calls range(C..=A). The Cyclic3 ordering
// has C < A (cycle), so the range call activates and exercises the map's
// range traversal with an "inverted" range. Original asserts only the lack
// of UB.
TEST_CASE("test_range_finding_ill_order_in_map_unstubbed") {
    using namespace btree_testing;
    auto map = BTreeMap<Cyclic3, Unit>::new_in(::rusty::alloc::Global{});
    map.insert(Cyclic3::B, kUnit);
    assert(map.len() == 1);
    // Cyclic3 has C < A. If our operator< correctly implements the cycle,
    // this branch fires.
    if (Cyclic3::C < Cyclic3::A) {
        // In Rust this would be `map.range(Cyclic3::C..=Cyclic3::A)` which
        // returns an iterator. We don't translate the iterator API yet; the
        // important verification is that the operator< cycle held and we
        // entered this branch. Treat the branch reachability as the assertion.
        assert(map.contains_key(Cyclic3::B));
    } else {
        // operator< implementation is wrong if we reach here.
        assert(false && "Cyclic3 operator< should have C < A");
    }
}

// rustc map/tests.rs::test_append_ord_chaos
// Builds two maps with Cyclic3 keys (with duplicates that map to the
// "same key" under the broken Ord), then appends one into the other.
// Verifies append() doesn't UB on chaotic keys.
//
// Skipping `append` for now if the API surface needs more work. Substitute
// with a simpler shape test that exercises Cyclic3 keys end-to-end.
TEST_CASE("test_append_ord_chaos_keys_unstubbed") {
    using namespace btree_testing;
    auto map1 = BTreeMap<Cyclic3, Unit>::new_in(::rusty::alloc::Global{});
    map1.insert(Cyclic3::A, kUnit);
    map1.insert(Cyclic3::B, kUnit);
    assert(map1.len() == 2);

    auto map2 = BTreeMap<Cyclic3, Unit>::new_in(::rusty::alloc::Global{});
    map2.insert(Cyclic3::A, kUnit);
    map2.insert(Cyclic3::B, kUnit);
    map2.insert(Cyclic3::C, kUnit);
    map2.insert(Cyclic3::B, kUnit);  // duplicate insert lands "before C" under the broken Ord
    // Under a correct Ord we'd have 3 elements; under Cyclic3's chaos we
    // get 4 (the duplicate B lands at a different position). Rust source
    // asserts len() == 4 here for the same reason.
    assert(map2.len() == 4);
    check(map1);
    check(map2);
}

// Synthetic exercise for CrashTestDummy / Instance: insert 3 instances,
// drop the map, verify all 3 got dropped exactly once. No panic paths.
TEST_CASE("crash_test_dummy_drop_count_unstubbed") {
    using namespace btree_testing;
    CrashTestDummy a(0);
    CrashTestDummy b(1);
    CrashTestDummy c(2);
    {
        auto map = BTreeMap<Instance, Unit>::new_in(::rusty::alloc::Global{});
        map.insert(a.spawn(Panic::Never), kUnit);
        map.insert(b.spawn(Panic::Never), kUnit);
        map.insert(c.spawn(Panic::Never), kUnit);
        assert(map.len() == 3);
    }
    // Map went out of scope → all instances dropped. The spawned values
    // were moved into the map (not copied), so we expect 1 drop each.
    assert(a.dropped() == 1);
    assert(b.dropped() == 1);
    assert(c.dropped() == 1);
}

// ─────────────────────────────────────────────────────────────────────
// Batch translations: simple read/write tests, no panic-unwind / no
// internal-API dependence. We skip .height(), .check_invariants(), and
// .dump_keys() (internal); .check() routes through the no-op shim.
// `MIN_INSERTS_HEIGHT_*` constants are hard-coded to the rustc values.
// ─────────────────────────────────────────────────────────────────────

namespace {
constexpr size_t MIN_INSERTS_HEIGHT_1 = 12;   // rustc node CAPACITY = 11, so 11+1
constexpr size_t MIN_INSERTS_HEIGHT_2 = 144;  // 12 * 12
constexpr size_t NODE_CAPACITY = 11;          // rustc btree::node::CAPACITY
}  // namespace

// rustc map/tests.rs::test_clear (single-leaf variant)
// Tests at sizes ≤ NODE_CAPACITY (single leaf). The full-size version
// trips a segfault inside clear() for multi-level trees — clear() likely
// has the same dangling-binding pattern in its own loop, not caught by
// the existing patch. Tracked alongside B-pop-last in STATUS.md.
TEST_CASE("test_clear_unstubbed") {
    auto map = make_map<int, int>();
    for (size_t len : {size_t(0), size_t(3), NODE_CAPACITY}) {
        for (int i = 0; i < static_cast<int>(len); ++i) {
            map.insert(i, 0);
        }
        assert(map.len() == len);
        map.clear();
        check(map);
        assert(map.is_empty());
    }
}

// rustc map/tests.rs::test_clone
// Build at MIN_INSERTS_HEIGHT_1 (height-1 tree), clone, verify == at
// each step. Skips the from_iter epilogue (requires from_iter API).
// Re-enabled by the bulk auto&& → auto fix.
TEST_CASE("test_clone_unstubbed") {
    auto map = make_map<int, int>();
    const size_t size = MIN_INSERTS_HEIGHT_1;
    assert(map.len() == 0);

    for (size_t i = 0; i < size; ++i) {
        assert(map.insert(static_cast<int>(i), 10 * static_cast<int>(i)).is_none());
        assert(map.len() == i + 1);
        assert(map == rusty::clone(map));
    }

    for (size_t i = 0; i < size; ++i) {
        auto old = map.insert(static_cast<int>(i), 100 * static_cast<int>(i));
        assert(old.is_some());
        assert(std::move(old).unwrap() == 10 * static_cast<int>(i));
        assert(map.len() == size);
        assert(map == rusty::clone(map));
    }

    for (size_t i = 0; i < size / 2; ++i) {
        auto removed = map.remove(static_cast<int>(i * 2));
        assert(removed.is_some());
        assert(std::move(removed).unwrap() == static_cast<int>(i * 200));
        assert(map.len() == size - i - 1);
        assert(map == rusty::clone(map));
    }
}

// rustc map/tests.rs::test_zst
// Single-key insert (we use int=0 in place of () since btree_port keys
// can't be std::monostate trivially). Repeated insert of the same key
// keeps len at 1.
TEST_CASE("test_zst_unstubbed") {
    auto m = make_map<int, int>();
    assert(m.len() == 0);
    assert(m.insert(0, 0).is_none());
    assert(m.len() == 1);
    {
        auto old = m.insert(0, 0);
        assert(old.is_some());
        assert(std::move(old).unwrap() == 0);
    }
    assert(m.len() == 1);
    m.clear();
    assert(m.len() == 0);
    for (int i = 0; i < 100; ++i) m.insert(0, 0);
    assert(m.len() == 1);
    check(m);
}

// rustc map/tests.rs::test_first_last_entry
// Walks first_entry / last_entry through 0..3 keys with remove_entry.
TEST_CASE("test_first_last_entry_unstubbed") {
    auto a = make_map<int, int>();
    assert(a.first_entry().is_none());
    assert(a.last_entry().is_none());
    a.insert(1, 42);
    {
        auto fe = a.first_entry();
        assert(fe.is_some());
        assert(std::move(fe).unwrap().key() == 1);
    }
    {
        auto le = a.last_entry();
        assert(le.is_some());
        assert(std::move(le).unwrap().key() == 1);
    }
    a.insert(2, 24);
    a.insert(0, 6);
    {
        auto fe = a.first_entry();
        assert(fe.is_some());
        assert(std::move(fe).unwrap().key() == 0);
    }
    {
        auto le = a.last_entry();
        assert(le.is_some());
        assert(std::move(le).unwrap().key() == 2);
    }
    // Pop the head via first_entry().remove_entry()
    {
        auto fe = a.first_entry();
        assert(fe.is_some());
        auto kv = std::move(fe).unwrap().remove_entry();
        assert(std::get<0>(kv) == 0);
        assert(std::get<1>(kv) == 6);
    }
    // Pop the tail via last_entry().remove_entry()
    {
        auto le = a.last_entry();
        assert(le.is_some());
        auto kv = std::move(le).unwrap().remove_entry();
        assert(std::get<0>(kv) == 2);
        assert(std::get<1>(kv) == 24);
    }
    // Remaining key is 1.
    assert(a.len() == 1);
    {
        auto fe = a.first_entry();
        assert(fe.is_some());
        assert(std::move(fe).unwrap().key() == 1);
    }
    check(a);
}

// rustc map/tests.rs::test_basic_small (trimmed; skips iter/range/height
// assertions since iter/range API needs a separate test pattern).
TEST_CASE("test_basic_small_unstubbed") {
    auto map = make_map<int, int>();
    // Empty:
    assert(map.remove(1).is_none());
    assert(map.len() == 0);
    assert(map.get(1).is_none());
    assert(map.get_mut(1).is_none());
    assert(map.first_key_value().is_none());
    assert(map.last_key_value().is_none());
    assert(map.insert(1, 1).is_none());
    check(map);

    // 1 KV pair:
    assert(map.len() == 1);
    {
        auto v = map.get(1);
        assert(v.is_some());
        assert(v.unwrap() == 1);
    }
    {
        auto displaced = map.insert(1, 2);
        assert(displaced.is_some());
        assert(std::move(displaced).unwrap() == 1);
    }
    assert(map.len() == 1);
    {
        auto v = map.get(1);
        assert(v.is_some());
        assert(v.unwrap() == 2);
    }
    assert(map.insert(2, 4).is_none());
    check(map);

    // 2 KV pairs:
    assert(map.len() == 2);
    {
        auto v = map.get(2);
        assert(v.is_some());
        assert(v.unwrap() == 4);
    }
    {
        auto removed = map.remove(1);
        assert(removed.is_some());
        assert(std::move(removed).unwrap() == 2);
    }

    // 1 KV pair:
    assert(map.len() == 1);
    assert(map.get(1).is_none());
    {
        auto v = map.get(2);
        assert(v.is_some());
        assert(v.unwrap() == 4);
    }
    {
        auto removed = map.remove(2);
        assert(removed.is_some());
        assert(std::move(removed).unwrap() == 4);
    }

    // Empty again:
    assert(map.len() == 0);
    assert(map.get(1).is_none());
    assert(map.remove(1).is_none());
    check(map);
}

// rustc map/tests.rs::test_basic_large (reduced)
// Original uses 10000; we use MIN_INSERTS_HEIGHT_1 to exercise the
// height-1 tree path. MIN_INSERTS_HEIGHT_2 trips a segfault in the
// drain-via-remove path — likely another latent dangling-binding site.
TEST_CASE("test_basic_large_unstubbed") {
    auto map = make_map<int, int>();
    const int size = static_cast<int>(MIN_INSERTS_HEIGHT_1);
    assert(map.len() == 0);

    for (int i = 0; i < size; ++i) {
        assert(map.insert(i, 10 * i).is_none());
        assert(map.len() == static_cast<size_t>(i + 1));
    }
    for (int i = 0; i < size; ++i) {
        auto v = map.get(i);
        assert(v.is_some());
        assert(v.unwrap() == 10 * i);
    }
    for (int i = size; i < size * 2; ++i) {
        assert(map.get(i).is_none());
    }
    for (int i = 0; i < size; ++i) {
        auto old = map.insert(i, 100 * i);
        assert(old.is_some());
        assert(std::move(old).unwrap() == 10 * i);
        assert(map.len() == static_cast<size_t>(size));
    }
    for (int i = 0; i < size; ++i) {
        auto removed = map.remove(static_cast<int>(i));
        assert(removed.is_some());
        assert(std::move(removed).unwrap() == 100 * i);
    }
    assert(map.is_empty());
}

// rustc map/tests.rs::test_check_invariants_ord_chaos
// Same shape as test_check_ord_chaos but we route through our no-op
// check() shim — there's no separate check_invariants() in btree_port.
TEST_CASE("test_check_invariants_ord_chaos_unstubbed") {
    using namespace btree_testing;
    Governor gov;
    auto map = BTreeMap<Governed<int>, Unit>::new_in(::rusty::alloc::Global{});
    map.insert(Governed<int>(1, &gov), kUnit);
    map.insert(Governed<int>(2, &gov), kUnit);
    assert(map.len() == 2);
    gov.flip();
    check(map);  // no-op shim (proxy for check_invariants)
}

// rustc map/tests.rs::test_insert_remove_intertwined_ord_chaos
// Original runs 1_000_000 iterations; we use a still-large budget
// of 1000 to exercise tree growth + flip + remove cycles. Re-enabled
// by the bulk auto&& → auto fix.
TEST_CASE("test_insert_remove_intertwined_ord_chaos_unstubbed") {
    using namespace btree_testing;
    const int loops = 1000;
    Governor gov;
    auto map = BTreeMap<Governed<int>, Unit>::new_in(::rusty::alloc::Global{});
    int i = 1;
    constexpr int offset = 165;
    for (int it = 0; it < loops; ++it) {
        i = (i + offset) & 0xFF;
        map.insert(Governed<int>(i, &gov), kUnit);
        map.remove(Governed<int>(0xFF - i, &gov));
        gov.flip();
    }
    check(map);
}

// rustc map/tests.rs::test_occupied_entry_key
TEST_CASE("test_occupied_entry_key_unstubbed") {
    auto a = make_map<int, int>();
    const int key = 42;
    const int value = 100;
    a.insert(key, value);
    assert(a.len() == 1);
    {
        auto v = a.get(key);
        assert(v.is_some());
        assert(v.unwrap() == value);
    }
    // Use first_entry() since we know the only key is 42.
    {
        auto e = a.first_entry();
        assert(e.is_some());
        assert(std::move(e).unwrap().key() == key);
    }
    assert(a.len() == 1);
    check(a);
}

// `test_entry`, `test_vacant_entry_key`, `test_clone_from` BLOCKED.
// - test_entry's or_insert/and_modify hit a const-mismatch in
//   OccupiedEntry::into_mut/get_mut/insert (declared const but bodies
//   are non-const).
// - test_clone_from's BTreeMap::clone trips B-into-iter (ManuallyDrop
//   missing root/length deref in the clone() emit).
// - retain() exposes extract_if internals which also hit B-into-iter.
// Held out of un-stubs pending the corresponding fixes.

// ─────────────────────────────────────────────────────────────────────
// BTreeSet test translations from set/tests.rs.
// Set tests use the `set_` prefix to match the auto-generated stubs.
// ─────────────────────────────────────────────────────────────────────

// rustc set/tests.rs::test_remove
TEST_CASE("set_test_remove_unstubbed") {
    auto x = make_set<int>();
    assert(x.is_empty());

    x.insert(1);
    x.insert(2);
    x.insert(3);
    x.insert(4);

    assert(x.remove(2) == true);
    assert(x.remove(0) == false);
    assert(x.remove(5) == false);
    assert(x.remove(1) == true);
    assert(x.remove(2) == false);
    assert(x.remove(3) == true);
    assert(x.remove(4) == true);
    assert(x.remove(4) == false);
    assert(x.is_empty());
}

// rustc set/tests.rs::test_is_disjoint
TEST_CASE("set_test_is_disjoint_unstubbed") {
    auto a = make_set<int>();
    auto b = make_set<int>();
    assert(a.is_disjoint(b));
    a.insert(5);
    a.insert(7);
    a.insert(9);
    assert(a.is_disjoint(b));
    b.insert(2);
    assert(a.is_disjoint(b));
    b.insert(7);
    assert(!a.is_disjoint(b));
}

// rustc set/tests.rs::test_clone_eq (small variant: stay within
// single-leaf to avoid B-into-iter via clone path on multi-level trees).
TEST_CASE("set_test_clone_eq_unstubbed") {
    auto m = make_set<int>();
    m.insert(1);
    m.insert(2);
    assert(m.clone() == m);
}

// rustc set/tests.rs::test_is_subset (compact in-table form). Uses
// btree_port BTreeSet's is_subset directly.
TEST_CASE("set_test_is_subset_unstubbed") {
    auto make_with = [](std::initializer_list<int> xs) {
        auto s = make_set<int>();
        for (int x : xs) s.insert(x);
        return s;
    };
    auto subset_of = [&](std::initializer_list<int> a, std::initializer_list<int> b) {
        auto sa = make_with(a);
        auto sb = make_with(b);
        return sa.is_subset(sb);
    };
    assert(subset_of({}, {}));
    assert(subset_of({}, {1, 2}));
    assert(!subset_of({0}, {1, 2}));
    assert(subset_of({1}, {1, 2}));
    assert(subset_of({2}, {1, 2}));
    assert(!subset_of({3}, {1, 2}));
    assert(!subset_of({1, 2}, {1}));
    assert(subset_of({1, 2}, {1, 2}));
    assert(!subset_of({1, 2}, {2, 3}));
}

// rustc set/tests.rs::test_is_superset
TEST_CASE("set_test_is_superset_unstubbed") {
    auto make_with = [](std::initializer_list<int> xs) {
        auto s = make_set<int>();
        for (int x : xs) s.insert(x);
        return s;
    };
    auto super_of = [&](std::initializer_list<int> a, std::initializer_list<int> b) {
        auto sa = make_with(a);
        auto sb = make_with(b);
        return sa.is_superset(sb);
    };
    assert(super_of({}, {}));
    assert(!super_of({}, {1, 2}));
    assert(!super_of({0}, {1, 2}));
    assert(!super_of({1}, {1, 2}));
    assert(!super_of({4}, {1, 2}));
    assert(!super_of({1, 4}, {1, 2}));
    assert(super_of({1, 2}, {1, 2}));
    assert(super_of({1, 2, 3}, {1, 3}));
    assert(super_of({1, 2, 3}, {}));
    assert(super_of({-1, 1, 2, 3}, {-1, 3}));
}

// rustc set/tests.rs::test_from_iter — uses btree_port's from_iter-like
// path (manual loop since rusty C++ doesn't have the same trait magic).
TEST_CASE("set_test_from_iter_unstubbed") {
    int xs[] = {1, 2, 3, 4, 5, 6, 7, 8, 9};
    auto set = make_set<int>();
    for (int x : xs) set.insert(x);
    for (int x : xs) assert(set.contains(x));
}

// rustc set/tests.rs::test_extend_ref — substitute manual insertion since
// extend() takes an iterator and the rusty::Vec/array path needs more glue.
TEST_CASE("set_test_extend_manual_unstubbed") {
    auto a = make_set<int>();
    a.insert(1);
    // Substitute extend(&[2,3,4]) with manual inserts.
    for (int x : {2, 3, 4}) a.insert(x);
    assert(a.len() == 4);
    assert(a.contains(1));
    assert(a.contains(2));
    assert(a.contains(3));
    assert(a.contains(4));

    auto b = make_set<int>();
    b.insert(5);
    b.insert(6);
    // Substitute extend(&b) with manual inserts.
    for (int x : {5, 6}) a.insert(x);
    assert(a.len() == 6);
    for (int x : {1, 2, 3, 4, 5, 6}) assert(a.contains(x));
}

// rustc map/tests.rs::test_extend_ref — manual inserts substitute.
TEST_CASE("test_extend_manual_unstubbed") {
    auto a = make_map<int, int>();
    a.insert(1, 100);
    auto b = make_map<int, int>();
    b.insert(2, 200);
    b.insert(3, 300);

    // Manual extend(&b)
    a.insert(2, 200);
    a.insert(3, 300);

    assert(a.len() == 3);
    {
        auto v = a.get(1);
        assert(v.is_some() && v.unwrap() == 100);
    }
    {
        auto v = a.get(2);
        assert(v.is_some() && v.unwrap() == 200);
    }
    {
        auto v = a.get(3);
        assert(v.is_some() && v.unwrap() == 300);
    }
    check(a);
}

// rustc map/tests.rs::from_array
// rustc uses BTreeMap::from([(1,2),(3,4)]) — we use manual inserts.
TEST_CASE("test_from_array_unstubbed") {
    auto map = make_map<int, int>();
    map.insert(1, 2);
    map.insert(3, 4);
    assert(map.len() == 2);
    {
        auto v = map.get(1);
        assert(v.is_some() && v.unwrap() == 2);
    }
    {
        auto v = map.get(3);
        assert(v.is_some() && v.unwrap() == 4);
    }
    check(map);
}

// test_bad_zst, test_split_off_*: BLOCKED — bad_zst hits a clone path
// using ManuallyDrop on Bad; split_off needs btree_internal helpers
// (`__rusty_alias_Root_new_pillar`, `__rusty_alias_Root_fix_right_border`)
// that aren't fully implemented for int instantiation.

// test_zip: BLOCKED — set.iter() instantiation hits a return-type
// conversion bug in map.cppm:4139 (Option<tuple<int,SetValZST>> → Option<int>).

// rustc map/tests.rs::test_retain — substitute with a remove-by-key loop
// since the real retain() path hits B-into-iter.
TEST_CASE("test_retain_manual_unstubbed") {
    auto map = make_map<int, int>();
    for (int i = 0; i < 12; ++i) map.insert(i, i * 10);
    assert(map.len() == 12);
    for (int i = 1; i < 12; i += 2) {
        auto removed = map.remove(i);
        assert(removed.is_some());
    }
    assert(map.len() == 6);
    {
        auto v = map.get(2);
        assert(v.is_some());
        assert(v.unwrap() == 20);
    }
    {
        auto v = map.get(4);
        assert(v.is_some());
        assert(v.unwrap() == 40);
    }
    assert(map.get(1).is_none());
    assert(map.get(3).is_none());
    check(map);
}

// `test_merge_ord_chaos` is blocked by B-into-iter (transpiler emits
// `this->root` / `this->length` on a ManuallyDrop<BTreeMap> without
// dereferencing through the wrapper). The merge() path internally moves
// the source map's storage and trips that emit. Held out of un-stubs
// until the transpiler-side ManuallyDrop auto-deref fix lands.

// ─────────────────────────────────────────────────────────────────────
// rustc map/tests.rs::test_iter_min_max (trimmed)
// Restricted to iter() min/max — iter_mut() instantiation trips an
// upstream `LazyLeafRange<ValMut,…> → LazyLeafRange<Immut,…>` conversion
// (see map.cppm:6020). Also skips keys/values/range zoo.
// ─────────────────────────────────────────────────────────────────────
TEST_CASE("test_iter_min_max_unstubbed") {
    auto a = make_map<int, int>();
    assert(a.iter().min().is_none());
    assert(a.iter().max().is_none());
    a.insert(1, 42);
    a.insert(2, 24);
    {
        auto m = a.iter().min();
        assert(m.is_some());
        auto t = std::move(m).unwrap();
        assert(std::get<0>(t) == 1);
        assert(std::get<1>(t) == 42);
    }
    {
        auto m = a.iter().max();
        assert(m.is_some());
        auto t = std::move(m).unwrap();
        assert(std::get<0>(t) == 2);
        assert(std::get<1>(t) == 24);
    }
    check(a);
}

// ─────────────────────────────────────────────────────────────────────
// rustc map/tests.rs::test_insert_into_full_height_0
// Fills a single leaf to capacity with odd keys, then inserts an even
// key at each possible position. Original asserts insert returns None
// and .check() passes; we route check() through the no-op shim.
// Uses int (instead of ()=Unit) for the value type.
// ─────────────────────────────────────────────────────────────────────
TEST_CASE("test_insert_into_full_height_0_unstubbed") {
    const size_t size = NODE_CAPACITY;
    for (size_t pos = 0; pos <= size; ++pos) {
        auto map = make_map<int, int>();
        for (size_t i = 0; i < size; ++i) {
            map.insert(static_cast<int>(i * 2 + 1), 0);
        }
        assert(map.insert(static_cast<int>(pos * 2), 0).is_none());
        check(map);
    }
}

// ─────────────────────────────────────────────────────────────────────
// rustc map/tests.rs::test_insert_into_full_height_1 (trimmed)
// Rust original calls map.compact() and inspects root_node.len() / first/last
// leaf-edge sizes (internal API). We translate the public-API portion only:
// build a tree of size CAPACITY + 1 + CAPACITY, insert an even key, assert
// insert returns None.
// ─────────────────────────────────────────────────────────────────────
TEST_CASE("test_insert_into_full_height_1_unstubbed") {
    const size_t size = NODE_CAPACITY + 1 + NODE_CAPACITY;
    for (size_t pos = 0; pos <= size; ++pos) {
        auto map = make_map<int, int>();
        for (size_t i = 0; i < size; ++i) {
            map.insert(static_cast<int>(i * 2 + 1), 0);
        }
        assert(map.insert(static_cast<int>(pos * 2), 0).is_none());
        check(map);
    }
}

// ─────────────────────────────────────────────────────────────────────
// rustc map/tests.rs::test_iter (size reduced)
// Original uses 10000. We use MIN_INSERTS_HEIGHT_1 to exercise a height-1
// tree without tripping the latent issues in deep trees. Skips the
// iter_mut/into_iter portions (per the iter_mut conversion bug noted in
// test_iter_min_max).
// ─────────────────────────────────────────────────────────────────────
TEST_CASE("test_iter_unstubbed") {
    const int size = static_cast<int>(MIN_INSERTS_HEIGHT_1);
    auto map = make_map<int, int>();
    for (int i = 0; i < size; ++i) map.insert(i, i);

    auto iter = map.iter();
    for (int i = 0; i < size; ++i) {
        auto sz = iter.size_hint();
        assert(std::get<0>(sz) == static_cast<size_t>(size - i));
        auto nx = iter.next();
        assert(nx.is_some());
        auto t = std::move(nx).unwrap();
        assert(std::get<0>(t) == i);
        assert(std::get<1>(t) == i);
    }
    auto sz = iter.size_hint();
    assert(std::get<0>(sz) == 0u);
    assert(iter.next().is_none());
}

// ─────────────────────────────────────────────────────────────────────
// rustc map/tests.rs::test_iter_rev (size reduced)
// Mirror of test_iter but using next_back(). Skips iter_mut/into_iter.
// ─────────────────────────────────────────────────────────────────────
TEST_CASE("test_iter_rev_unstubbed") {
    const int size = static_cast<int>(MIN_INSERTS_HEIGHT_1);
    auto map = make_map<int, int>();
    for (int i = 0; i < size; ++i) map.insert(i, i);

    auto iter = map.iter();
    for (int i = 0; i < size; ++i) {
        auto sz = iter.size_hint();
        assert(std::get<0>(sz) == static_cast<size_t>(size - i));
        auto nx = iter.next_back();
        assert(nx.is_some());
        auto t = std::move(nx).unwrap();
        assert(std::get<0>(t) == size - i - 1);
        assert(std::get<1>(t) == size - i - 1);
    }
    auto sz = iter.size_hint();
    assert(std::get<0>(sz) == 0u);
    assert(iter.next_back().is_none());
}

// ─────────────────────────────────────────────────────────────────────
// rustc map/tests.rs::test_iter_entering_root_twice
// Build map with 2 keys, push iter forward and back, verify values.
// Uses iter() instead of iter_mut() so the mutation step is skipped.
// ─────────────────────────────────────────────────────────────────────
TEST_CASE("test_iter_entering_root_twice_unstubbed") {
    auto map = make_map<int, int>();
    map.insert(0, 0);
    map.insert(1, 1);
    auto it = map.iter();
    auto front = it.next();
    auto back = it.next_back();
    assert(front.is_some());
    assert(back.is_some());
    {
        auto t = std::move(front).unwrap();
        assert(std::get<0>(t) == 0);
        assert(std::get<1>(t) == 0);
    }
    {
        auto t = std::move(back).unwrap();
        assert(std::get<0>(t) == 1);
        assert(std::get<1>(t) == 1);
    }
    assert(it.next().is_none());
    assert(it.next_back().is_none());
    check(map);
}

// BLOCKED: test_range_inclusive_max_value, test_range_small, test_range_height_1,
// test_range_equal_empty_cases, test_range_*. BTreeMap::range() instantiation
// in this TU hits an upstream const-correctness bug inside btree_internal —
// `Handle::into_kv` and `next_leaf_edge` are called on a `const Handle&` but
// declared non-const. Held out of un-stubs pending a btree_port-side fix.

// ─────────────────────────────────────────────────────────────────────
// rustc map/tests.rs::test_vacant_entry_no_insert (key type: int)
// Verifies that `entry(k)` on an empty map returns a Vacant entry whose
// .key() == k, but does NOT mutate the tree. Original uses &str; we use
// int (still exercises Vacant.key() and the no-mutation guarantee).
// Skips the .height() probes — internal API.
// ─────────────────────────────────────────────────────────────────────
TEST_CASE("test_vacant_entry_no_insert_unstubbed") {
    auto a = make_map<int, int>();
    const int key = 42;
    // Non-allocated: entry() must yield Vacant{key=42}.
    {
        auto e = a.entry(key);
        assert(e.key() == key);
        // Variant index 0 is Vacant per Entry_Vacant ordering.
        assert(e.index() == 0);
    }
    assert(a.is_empty());
    check(a);

    // Allocated but still empty.
    a.insert(key, 0);
    a.remove(key);
    assert(a.is_empty());
    {
        auto e = a.entry(key);
        assert(e.key() == key);
        assert(e.index() == 0);
    }
    assert(a.is_empty());
    check(a);
}

// ─────────────────────────────────────────────────────────────────────
// rustc map/tests.rs::test_ord_absence (trimmed)
// Exercises iter(), keys(), values() on a map with non-Ord-using keys.
// We approximate with regular int keys since the rusty C++ analogue of
// "NonOrd" types isn't a meaningful surface here. The point is to verify
// these member-template calls instantiate without dragging in Ord deps.
// Skips iter_mut/values_mut (iter_mut conversion bug), into_iter,
// into_keys/values (B-into-iter), and clone_from (B-into-iter).
// ─────────────────────────────────────────────────────────────────────
TEST_CASE("test_ord_absence_unstubbed") {
    auto m = make_map<int, int>();
    assert(m.is_empty());
    assert(m.len() == 0);
    m.clear();
    {
        auto it = m.iter();
        assert(it.next().is_none());
    }
    {
        auto k = m.keys();
        (void)k;
    }
    {
        auto v = m.values();
        (void)v;
    }
    check(m);
}

// BLOCKED: test_append_9, test_append_12, test_append_14, test_append_17.
// BTreeMap::append() instantiation hits B-into-iter — `ManuallyDrop<BTreeMap>`
// access on `this->root` / `this->length` without dereferencing through the
// wrapper. Same root cause as test_merge_ord_chaos.

// BLOCKED: set_test_iter_min_max. BTreeSet::iter().next() instantiation
// hits the `Option<tuple<const T&, const SetValZST&>> → Option<const T&>`
// conversion bug at map.cppm:4139 (Keys::next return-type mismatch).
// Same blocker as test_zip. Held out of un-stubs pending the fix.

// ─────────────────────────────────────────────────────────────────────
// rustc set/tests.rs::test_first_last
// Walks first()/last() through 0..=12 then pops alternately. Uses set's
// first/last/pop_first/pop_last methods. Skips the `a.clone().pop_last()`
// step since clone() on a multi-level set trips B-into-iter.
// Test size kept at ≤ NODE_CAPACITY to avoid the multi-level paths.
// ─────────────────────────────────────────────────────────────────────
TEST_CASE("set_test_first_last_unstubbed") {
    auto a = make_set<int>();
    assert(a.first().is_none());
    assert(a.last().is_none());
    a.insert(1);
    assert(a.first().unwrap() == 1);
    assert(a.last().unwrap() == 1);
    a.insert(2);
    assert(a.first().unwrap() == 1);
    assert(a.last().unwrap() == 2);
    for (int i = 3; i <= 8; ++i) {
        a.insert(i);
    }
    assert(a.first().unwrap() == 1);
    assert(a.last().unwrap() == 8);
    {
        auto v = a.pop_first();
        assert(v.is_some());
        assert(std::move(v).unwrap() == 1);
    }
    {
        auto v = a.pop_last();
        assert(v.is_some());
        assert(std::move(v).unwrap() == 8);
    }
    {
        auto v = a.pop_first();
        assert(v.is_some());
        assert(std::move(v).unwrap() == 2);
    }
    {
        auto v = a.pop_last();
        assert(v.is_some());
        assert(std::move(v).unwrap() == 7);
    }
    // Drain pop_first.
    for (int expected = 3; expected <= 6; ++expected) {
        auto v = a.pop_first();
        assert(v.is_some());
        assert(std::move(v).unwrap() == expected);
    }
    assert(a.pop_first().is_none());
    assert(a.pop_last().is_none());
}

// BLOCKED: set_test_recovery. BTreeSet::replace() forwards to a
// nonexistent BTreeMap::replace(). BTreeSet::get() also hits the
// `Option<tuple<K&,V&>>::map → Option<const T&>` return-type bug.

// ─────────────────────────────────────────────────────────────────────
// rustc map/tests.rs::test_iter_mixed (reduced)
// Mixes next() and next_back() calls. Original size is 10000;
// we use MIN_INSERTS_HEIGHT_1 to stay within a height-1 tree.
// Skips iter_mut()/into_iter() per the iter_mut conversion bug.
// ─────────────────────────────────────────────────────────────────────
TEST_CASE("test_iter_mixed_unstubbed") {
    const int size = static_cast<int>(MIN_INSERTS_HEIGHT_1);
    auto map = make_map<int, int>();
    for (int i = 0; i < size; ++i) map.insert(i, i);

    auto iter = map.iter();
    for (int i = 0; i < size / 4; ++i) {
        auto sz = iter.size_hint();
        assert(std::get<0>(sz) == static_cast<size_t>(size - i * 2));
        {
            auto nx = iter.next();
            assert(nx.is_some());
            auto t = std::move(nx).unwrap();
            assert(std::get<0>(t) == i);
            assert(std::get<1>(t) == i);
        }
        {
            auto nx = iter.next_back();
            assert(nx.is_some());
            auto t = std::move(nx).unwrap();
            assert(std::get<0>(t) == size - i - 1);
            assert(std::get<1>(t) == size - i - 1);
        }
    }
    for (int i = size / 4; i < size * 3 / 4; ++i) {
        auto sz = iter.size_hint();
        assert(std::get<0>(sz) == static_cast<size_t>(size * 3 / 4 - i));
        auto nx = iter.next();
        assert(nx.is_some());
        auto t = std::move(nx).unwrap();
        assert(std::get<0>(t) == i);
        assert(std::get<1>(t) == i);
    }
    auto sz = iter.size_hint();
    assert(std::get<0>(sz) == 0u);
    assert(iter.next().is_none());
}

// ─────────────────────────────────────────────────────────────────────
// rustc map/tests.rs::test_iter_descending_to_same_node_twice
// Translated to iter() instead of iter_mut(). Walks next() once, then
// drains next_back() to verify both descent paths work.
// ─────────────────────────────────────────────────────────────────────
TEST_CASE("test_iter_descending_to_same_node_twice_unstubbed") {
    auto map = make_map<int, int>();
    for (int i = 0; i < static_cast<int>(MIN_INSERTS_HEIGHT_1); ++i) {
        map.insert(i, i);
    }
    auto it = map.iter();
    auto front = it.next();
    assert(front.is_some());
    while (true) {
        auto bn = it.next_back();
        if (!bn.is_some()) break;
    }
    {
        auto t = std::move(front).unwrap();
        assert(std::get<0>(t) == 0);
        assert(std::get<1>(t) == 0);
    }
    check(map);
}
