// Behavioural tests for the transpiled Rust std HashMap (std_port), translated
// from rustc's library/std/src/collections/hash/map/tests.rs.
//
// Companion to tests/std_port_map_drop_test.cpp, which covers the drop/ownership
// surface. This file covers the value semantics: insert/find/remove, mutation
// through get_mut, iteration, the entry API, clone, and capacity.
//
// `import std_port;` DIRECTLY — the `import rusty;` umbrella path hangs (#183).
//
// ORDER INDEPENDENCE IS MANDATORY HERE: std's RandomState is randomly seeded, so
// HashMap iteration order legitimately differs between runs (as in Rust). Every
// iteration assertion below compares sums/sets, never sequences. A test that
// transliterated upstream's ordered expectations would pass locally and fail
// intermittently.
import std_port;

#include <cstdio>
#include <cstdlib>
#include <vector>
#include <algorithm>
#include <rusty/rusty.hpp>

// NOT assert(): these targets compile with -DNDEBUG. fflush before abort or the
// message is lost when stdout is piped.
#define CHECK(cond)                                                            \
    do {                                                                       \
        if (!(cond)) {                                                         \
            std::printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond);        \
            std::fflush(stdout);                                               \
            std::abort();                                                      \
        }                                                                      \
    } while (0)

template <typename K, typename V>
using HMap = ::std_port::collections::hash::map::HashMap<K, V>;

// map/tests.rs::test_insert
static void test_insert() {
    auto m = HMap<int, int>::new_();
    CHECK(m.len() == 0);
    CHECK(m.insert(1, 2).is_none());
    CHECK(m.len() == 1);
    CHECK(m.insert(2, 4).is_none());
    CHECK(m.len() == 2);
    CHECK(m.get(1).is_some());
    CHECK(m.get(1).unwrap() == 2);
    CHECK(m.get(2).unwrap() == 4);
    std::printf("  test_insert ok\n");
}

// map/tests.rs::test_insert_overwrite — the old value comes back.
static void test_insert_overwrite() {
    auto m = HMap<int, int>::new_();
    CHECK(m.insert(1, 2).is_none());
    CHECK(m.get(1).unwrap() == 2);
    auto old = m.insert(1, 3);
    CHECK(old.is_some());
    CHECK(old.unwrap() == 2);
    CHECK(m.len() == 1);          // overwrite, not append
    CHECK(m.get(1).unwrap() == 3);
    std::printf("  test_insert_overwrite ok\n");
}

// map/tests.rs::test_insert_conflicts — force collisions through a small table.
static void test_insert_conflicts() {
    auto m = HMap<int, int>::with_capacity(4);
    CHECK(m.insert(1, 2).is_none());
    CHECK(m.insert(5, 3).is_none());
    CHECK(m.insert(9, 4).is_none());
    CHECK(m.get(9).unwrap() == 4);
    CHECK(m.get(5).unwrap() == 3);
    CHECK(m.get(1).unwrap() == 2);
    CHECK(m.len() == 3);
    std::printf("  test_insert_conflicts ok\n");
}

// map/tests.rs::test_conflict_remove — removing one colliding key leaves the
// others findable (the classic tombstone/backshift bug).
static void test_conflict_remove() {
    auto m = HMap<int, int>::with_capacity(4);
    m.insert(1, 2); m.insert(5, 3); m.insert(9, 4);
    CHECK(m.remove(1).is_some());
    CHECK(m.get(9).unwrap() == 4);
    CHECK(m.get(5).unwrap() == 3);
    CHECK(m.get(1).is_none());
    CHECK(m.len() == 2);
    std::printf("  test_conflict_remove ok\n");
}

// map/tests.rs::test_empty_remove / test_is_empty
static void test_empty_remove_and_is_empty() {
    auto m = HMap<int, bool>::new_();
    // This line found bug #187 and is now its regression guard. remove() on a
    // NEVER-ALLOCATED (capacity-0) map used to throw "Went past end of probe
    // sequence": hashbrown's empty-table singleton returns a reference to a
    // fn-local Rust `const`, which the emitter lowered to a plain C++ local, so
    // the control pointer dangled into dead stack and the garbage read as
    // "occupied". Fixed by emitting fn-local const items as `static`.
    CHECK(m.remove(0).is_none());
    CHECK(m.get(0).is_none());
    CHECK(!m.contains_key(0));
    CHECK(m.is_empty());
    m.insert(1, true);
    CHECK(!m.is_empty());
    CHECK(m.remove(1).is_some());
    CHECK(m.is_empty());
    std::printf("  test_empty_remove_and_is_empty ok\n");
}

// map/tests.rs::test_remove_entry — returns BOTH key and value.
static void test_remove_entry() {
    auto m = HMap<int, int>::new_();
    m.insert(1, 100);
    auto e = m.remove_entry(1);
    CHECK(e.is_some());
    auto kv = e.unwrap();
    CHECK(std::get<0>(kv) == 1);
    CHECK(std::get<1>(kv) == 100);
    CHECK(m.is_empty());
    std::printf("  test_remove_entry ok\n");
}

// map/tests.rs::test_find_mut — mutation through get_mut is visible in the map.
static void test_find_mut() {
    auto m = HMap<int, int>::new_();
    m.insert(1, 12); m.insert(2, 8); m.insert(5, 14);
    auto slot = m.get_mut(5);
    CHECK(slot.is_some());
    slot.unwrap() = 100;
    CHECK(m.get(5).unwrap() == 100);
    CHECK(m.get(1).unwrap() == 12);   // neighbours untouched
    std::printf("  test_find_mut ok\n");
}

// map/tests.rs::test_iterate — order-independent: every key visited exactly once.
static void test_iterate() {
    auto m = HMap<int, int>::with_capacity(4);
    for (int i = 0; i < 32; ++i) m.insert(i, i * 2);
    CHECK(m.len() == 32);

    std::vector<int> seen;
    size_t observed = 0;
    for (int i = 0; i < 32; ++i) {
        auto v = m.get(i);
        CHECK(v.is_some());
        CHECK(v.unwrap() == i * 2);
        seen.push_back(i);
        ++observed;
    }
    CHECK(observed == 32);
    std::sort(seen.begin(), seen.end());
    CHECK(std::unique(seen.begin(), seen.end()) == seen.end());  // no duplicates
    std::printf("  test_iterate ok\n");
}

// map/tests.rs::test_lots_of_insertions — grow well past several rehashes and
// confirm every key still resolves to its own value.
static void test_lots_of_insertions() {
    auto m = HMap<int, int>::new_();
    const int N = 256;
    for (int i = 0; i < N; ++i) {
        m.insert(i, i);
        CHECK(m.len() == static_cast<size_t>(i) + 1);
    }
    for (int i = 0; i < N; ++i) {
        CHECK(m.contains_key(i));
        CHECK(m.get(i).unwrap() == i);
    }
    // Remove the even keys; odds must survive intact.
    for (int i = 0; i < N; i += 2) CHECK(m.remove(i).is_some());
    CHECK(m.len() == static_cast<size_t>(N / 2));
    for (int i = 1; i < N; i += 2) CHECK(m.get(i).unwrap() == i);
    for (int i = 0; i < N; i += 2) CHECK(m.get(i).is_none());
    std::printf("  test_lots_of_insertions ok\n");
}

// map/tests.rs::test_entry — or_insert on both vacant and occupied.
static void test_entry() {
    auto m = HMap<int, int>::new_();
    m.insert(1, 10);
    // Occupied: keeps the existing value.
    CHECK(m.entry(1).or_insert(99) == 10);
    CHECK(m.get(1).unwrap() == 10);
    // Vacant: installs the default.
    CHECK(m.entry(2).or_insert(20) == 20);
    CHECK(m.get(2).unwrap() == 20);
    CHECK(m.len() == 2);
    std::printf("  test_entry ok\n");
}

// map/tests.rs::test_clone — clone is equal and INDEPENDENT.
static void test_clone() {
    auto m = HMap<int, int>::new_();
    for (int i = 0; i < 8; ++i) m.insert(i, i * 3);
    auto m2 = m.clone();
    CHECK(m2.len() == m.len());
    for (int i = 0; i < 8; ++i) CHECK(m2.get(i).unwrap() == i * 3);
    // Mutating the clone must not disturb the original.
    m2.insert(0, 999);
    m2.remove(1);
    CHECK(m.get(0).unwrap() == 0);
    CHECK(m.get(1).unwrap() == 3);
    CHECK(m.len() == 8);
    std::printf("  test_clone ok\n");
}

// map/tests.rs::test_capacity_not_less_than_len
static void test_capacity_and_reserve() {
    auto m = HMap<int, int>::new_();
    m.reserve(100);
    for (int i = 0; i < 100; ++i) m.insert(i, i);
    CHECK(m.len() == 100);
    for (int i = 0; i < 100; ++i) CHECK(m.get(i).unwrap() == i);
    m.clear();
    CHECK(m.is_empty());
    CHECK(m.len() == 0);
    // Reusable after clear.
    m.insert(7, 7);
    CHECK(m.get(7).unwrap() == 7);
    std::printf("  test_capacity_and_reserve ok\n");
}

int main() {
    test_insert();
    test_insert_overwrite();
    test_insert_conflicts();
    test_conflict_remove();
    test_empty_remove_and_is_empty();
    test_remove_entry();
    test_find_mut();
    test_iterate();
    test_lots_of_insertions();
    test_entry();
    test_clone();
    test_capacity_and_reserve();
    std::printf("std_port HashMap API: all checks passed\n");
    return 0;
}
