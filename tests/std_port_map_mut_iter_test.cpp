// Mutation-through-iterator tests for the transpiled Rust std HashMap
// (std_port), translated from rustc's map/tests.rs (test_values_mut,
// test_iter_mut_len, test_mut_size_hint).
//
// WHY THIS FAMILY SPECIFICALLY: mutating through an iterator is the class that
// was already PROVEN broken here. retain() and ExtractIf::next both lowered
// Rust's `ref mut` bindings to by-value copies (#179), so the user's mutation
// landed on a temporary and never reached the table — a silent wrong answer,
// not a compile error, in ExtractIf's case. values_mut()/iter_mut() are the
// remaining members of that family and had no coverage at all.
//
// The assertions therefore always READ BACK through the map after mutating, not
// through the iterator: a mutation applied to a copy would still look correct
// if you only re-read the copy.
//
// `import std_port;` DIRECTLY, not via the `import rusty;` umbrella: these tests
// target the std port itself, so they should not depend on the umbrella's
// re-export aliases. (The umbrella used to hang here — #183, a forked std_port
// BMI from a CMake flag delta — but that is fixed; this is now a choice, not a
// workaround.)
import std_port;

#include <cstdio>
#include <cstdlib>
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

// map/tests.rs::test_values_mut — mutate every value through values_mut(), then
// verify by re-reading THE MAP.
static void test_values_mut_writes_reach_the_map() {
    auto m = HMap<int, int>::new_();
    for (int i = 1; i <= 5; ++i) m.insert(i, i);

    auto it = m.values_mut();
    size_t touched = 0;
    for (auto v = it.next(); v.is_some(); v = it.next()) {
        v.unwrap() *= 10;
        ++touched;
    }
    CHECK(touched == 5);

    // Read back through the map — this is the assertion that a by-value
    // iterator would fail.
    for (int i = 1; i <= 5; ++i) CHECK(m.get(i).unwrap() == i * 10);
    CHECK(m.len() == 5);
    std::printf("  test_values_mut_writes_reach_the_map ok\n");
}

// Same, via range-for (available since #188 gave ValuesMut begin()/end()).
static void test_values_mut_range_for() {
    auto m = HMap<int, int>::new_();
    for (int i = 1; i <= 5; ++i) m.insert(i, i);
    size_t touched = 0;
    for (auto& v : m.values_mut()) { v += 100; ++touched; }
    CHECK(touched == 5);
    for (int i = 1; i <= 5; ++i) CHECK(m.get(i).unwrap() == i + 100);
    std::printf("  test_values_mut_range_for ok\n");
}

// map/tests.rs::test_iter_mut_len — iter_mut yields (const K&, V&); the value
// half must alias the table, the key half must not be mutable.
static void test_iter_mut_writes_reach_the_map() {
    auto m = HMap<int, int>::new_();
    for (int i = 1; i <= 6; ++i) m.insert(i, i);

    auto it = m.iter_mut();
    size_t touched = 0;
    int key_sum = 0;
    for (auto kv = it.next(); kv.is_some(); kv = it.next()) {
        auto pair = kv.unwrap();
        key_sum += std::get<0>(pair);
        std::get<1>(pair) = std::get<0>(pair) * 7;
        ++touched;
    }
    CHECK(touched == 6);
    CHECK(key_sum == 1 + 2 + 3 + 4 + 5 + 6);

    for (int i = 1; i <= 6; ++i) CHECK(m.get(i).unwrap() == i * 7);
    std::printf("  test_iter_mut_writes_reach_the_map ok\n");
}

// A mutation that must survive a subsequent rehash: mutate, then grow the map
// well past its capacity and confirm the mutated values moved with it.
static void test_mutations_survive_rehash() {
    auto m = HMap<int, int>::with_capacity(4);
    for (int i = 0; i < 8; ++i) m.insert(i, i);
    for (auto& v : m.values_mut()) v = -v;
    for (int i = 0; i < 8; ++i) CHECK(m.get(i).unwrap() == -i);

    // Force several growths.
    for (int i = 8; i < 256; ++i) m.insert(i, i);
    CHECK(m.len() == 256);
    // The pre-rehash mutations must have been carried across, unchanged.
    for (int i = 0; i < 8; ++i) CHECK(m.get(i).unwrap() == -i);
    for (int i = 8; i < 256; ++i) CHECK(m.get(i).unwrap() == i);
    std::printf("  test_mutations_survive_rehash ok\n");
}

// get_mut is the single-element form of the same aliasing question, and is the
// one shape here that was already known good — kept as a control, so a failure
// in the bulk forms above can be attributed to the iterator rather than to the
// table's element storage.
static void test_get_mut_control() {
    auto m = HMap<int, int>::new_();
    m.insert(1, 1);
    m.get_mut(1).unwrap() = 42;
    CHECK(m.get(1).unwrap() == 42);
    std::printf("  test_get_mut_control ok\n");
}

int main() {
    test_get_mut_control();
    test_values_mut_writes_reach_the_map();
    test_values_mut_range_for();
    test_iter_mut_writes_reach_the_map();
    test_mutations_survive_rehash();
    std::printf("std_port HashMap mutation-through-iterator: all checks passed\n");
    return 0;
}
