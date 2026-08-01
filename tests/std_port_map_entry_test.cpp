// Entry-API and drain tests for the transpiled Rust std HashMap (std_port),
// translated from rustc's library/std/src/collections/hash/map/tests.rs.
//
// These are the highest-risk untested surfaces on the shipped path: an Entry
// holds a reference INTO the table across a user callback, and drain mutates
// the table while iterating it. Both are where a transpiled hash table is most
// likely to be subtly wrong, and neither had any coverage.
//
// `import std_port;` DIRECTLY — the `import rusty;` umbrella path hangs (#183).
import std_port;

#include <cstdio>
#include <cstdlib>
#include <vector>
#include <algorithm>
#include <tuple>
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

// map/tests.rs::test_entry — or_insert on vacant installs, on occupied keeps.
static void test_entry_or_insert() {
    auto m = HMap<int, int>::new_();
    m.insert(1, 10);
    CHECK(m.entry(1).or_insert(99) == 10);   // occupied: existing wins
    CHECK(m.entry(2).or_insert(20) == 20);   // vacant: default installed
    CHECK(m.len() == 2);
    CHECK(m.get(1).unwrap() == 10);
    CHECK(m.get(2).unwrap() == 20);
    std::printf("  test_entry_or_insert ok\n");
}

// or_insert returns a MUTABLE reference into the table — writing through it
// must be visible in the map. A copy would silently lose the write.
static void test_entry_or_insert_returns_live_ref() {
    auto m = HMap<int, int>::new_();
    m.entry(1).or_insert(5) = 500;
    CHECK(m.get(1).unwrap() == 500);
    m.entry(1).or_insert(0) += 1;            // occupied path, same aliasing
    CHECK(m.get(1).unwrap() == 501);
    std::printf("  test_entry_or_insert_returns_live_ref ok\n");
}

// or_insert_with / or_insert_with_key / or_default
static void test_entry_or_insert_variants() {
    auto m = HMap<int, int>::new_();
    CHECK(m.entry(3).or_insert_with([]() { return 33; }) == 33);
    CHECK(m.get(3).unwrap() == 33);
    // The closure must NOT run when the entry is occupied.
    int calls = 0;
    m.entry(3).or_insert_with([&]() { ++calls; return 99; });
    CHECK(calls == 0);
    CHECK(m.get(3).unwrap() == 33);
    // or_insert_with_key sees the key.
    CHECK(m.entry(7).or_insert_with_key([](const int& k) { return k * 100; }) == 700);
    CHECK(m.get(7).unwrap() == 700);
    // or_default installs a value-initialized V.
    CHECK(m.entry(9).or_default() == 0);
    CHECK(m.get(9).unwrap() == 0);
    std::printf("  test_entry_or_insert_variants ok\n");
}

// map/tests.rs — and_modify runs only for an occupied entry, and its mutation
// must land in the table.
static void test_entry_and_modify() {
    auto m = HMap<int, int>::new_();
    m.insert(1, 10);
    int calls = 0;
    m.entry(1).and_modify([&](int& v) { ++calls; v += 5; }).or_insert(0);
    CHECK(calls == 1);
    CHECK(m.get(1).unwrap() == 15);
    // Vacant: and_modify must NOT run, and or_insert supplies the value.
    calls = 0;
    m.entry(2).and_modify([&](int& v) { ++calls; v += 5; }).or_insert(77);
    CHECK(calls == 0);
    CHECK(m.get(2).unwrap() == 77);
    std::printf("  test_entry_and_modify ok\n");
}

// map/tests.rs::test_occupied_entry_key / test_vacant_entry_key — key() must
// report the probe key WITHOUT disturbing the map.
static void test_entry_key() {
    auto m = HMap<int, int>::new_();
    m.insert(1, 10);
    CHECK(m.entry(1).key() == 1);
    CHECK(m.len() == 1);          // observing an occupied entry inserts nothing
    CHECK(m.entry(2).key() == 2);
    CHECK(m.len() == 1);          // observing a VACANT entry must not insert
    CHECK(!m.contains_key(2));
    std::printf("  test_entry_key ok\n");
}

// map/tests.rs::test_entry_take_doesnt_corrupt — hammer entry() across many
// keys, removing as we go; the table must stay consistent.
static void test_entry_take_doesnt_corrupt() {
    auto m = HMap<int, int>::new_();
    const int N = 64;
    for (int i = 0; i < N; ++i) m.insert(i, i);
    // Touch every key through entry(), mutating in place.
    for (int i = 0; i < N; ++i) m.entry(i).and_modify([](int& v) { v *= 2; }).or_insert(-1);
    for (int i = 0; i < N; ++i) CHECK(m.get(i).unwrap() == i * 2);
    // Remove half, then entry() the removed keys (vacant path) and re-add.
    for (int i = 0; i < N; i += 2) CHECK(m.remove(i).is_some());
    CHECK(m.len() == static_cast<size_t>(N / 2));
    for (int i = 0; i < N; i += 2) m.entry(i).or_insert(i * 3);
    CHECK(m.len() == static_cast<size_t>(N));
    for (int i = 0; i < N; ++i) {
        CHECK(m.get(i).unwrap() == (i % 2 == 0 ? i * 3 : i * 2));
    }
    std::printf("  test_entry_take_doesnt_corrupt ok\n");
}

// map/tests.rs::test_drain — drain yields every element and leaves the map
// empty but reusable. Order-independent: RandomState randomizes it.
static void test_drain() {
    auto m = HMap<int, int>::new_();
    const int N = 32;
    int expected = 0;
    for (int i = 0; i < N; ++i) { m.insert(i, i * 2); expected += i + i * 2; }

    int total = 0;
    size_t seen = 0;
    for (const auto& kv : m.drain()) {
        total += std::get<0>(kv) + std::get<1>(kv);
        ++seen;
    }
    CHECK(seen == static_cast<size_t>(N));
    CHECK(total == expected);
    CHECK(m.is_empty());
    CHECK(m.len() == 0);
    // Reusable after drain.
    m.insert(5, 50);
    CHECK(m.get(5).unwrap() == 50);
    CHECK(m.len() == 1);
    std::printf("  test_drain ok\n");
}

// map/tests.rs::test_trivial_drain — draining an empty map is a no-op, not a
// crash. (The sibling shape of #187, which crashed on a capacity-0 remove.)
static void test_trivial_drain() {
    auto m = HMap<int, int>::new_();
    size_t seen = 0;
    for (const auto& kv : m.drain()) { (void)kv; ++seen; }
    CHECK(seen == 0);
    CHECK(m.is_empty());
    std::printf("  test_trivial_drain ok\n");
}

int main() {
    test_entry_or_insert();
    test_entry_or_insert_returns_live_ref();
    test_entry_or_insert_variants();
    test_entry_and_modify();
    test_entry_key();
    test_entry_take_doesnt_corrupt();
    test_drain();
    test_trivial_drain();
    std::printf("std_port HashMap entry/drain: all checks passed\n");
    return 0;
}
