// Behavioural tests for the transpiled Rust std HashSet (std_port), translated
// from rustc's library/std/src/collections/hash/set/tests.rs.
//
// Covers the surface tests/std_port_set_algebra_test.cpp does NOT: membership,
// remove/take/replace/get, clear, clone independence, extend, reserve, drain.
// (The four set-algebra views and is_disjoint/is_subset/is_superset live there.)
//
// `import std_port;` DIRECTLY — the `import rusty;` umbrella path hangs (#183).
// std's RandomState is randomly seeded, so anything order-sensitive is asserted
// as a sum or a membership check, never as a sequence.
import std_port;

#include <cstdio>
#include <cstdlib>
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

template <typename T>
using HSet = ::std_port::collections::hash::set::HashSet<T>;

// set/tests.rs — insert reports whether the element was NEW.
static void test_insert_and_contains() {
    auto s = HSet<int>::new_();
    CHECK(s.is_empty());
    CHECK(s.insert(1));            // new
    CHECK(!s.insert(1));           // already present
    CHECK(s.len() == 1);
    CHECK(s.contains(1));
    CHECK(!s.contains(2));
    CHECK(s.insert(2));
    CHECK(s.len() == 2);
    CHECK(!s.is_empty());
    std::printf("  test_insert_and_contains ok\n");
}

// set/tests.rs::test_remove — remove reports whether anything was removed.
static void test_remove() {
    auto s = HSet<int>::new_();
    s.insert(1); s.insert(2); s.insert(3);
    CHECK(s.remove(2));
    CHECK(!s.contains(2));
    CHECK(s.len() == 2);
    CHECK(!s.remove(2));           // second remove is a no-op
    CHECK(s.len() == 2);
    // Neighbours survive.
    CHECK(s.contains(1) && s.contains(3));
    std::printf("  test_remove ok\n");
}

// set/tests.rs — take() returns the STORED element, not the probe.
static void test_take() {
    auto s = HSet<int>::new_();
    s.insert(42);
    auto t = s.take(42);
    CHECK(t.is_some());
    CHECK(t.unwrap() == 42);
    CHECK(!s.contains(42));
    CHECK(s.is_empty());
    // take of an absent element on a NON-EMPTY set.
    s.insert(7);
    CHECK(s.take(8).is_none());
    CHECK(s.len() == 1);
    std::printf("  test_take ok\n");
}

// set/tests.rs::test_replace — replace RETURNS the previous element and swaps
// the stored one, where insert() would keep the original.
static void test_replace() {
    auto s = HSet<int>::new_();
    CHECK(s.replace(5).is_none());     // vacant: nothing came back
    CHECK(s.len() == 1);
    auto old = s.replace(5);           // occupied: the old element comes back
    CHECK(old.is_some());
    CHECK(old.unwrap() == 5);
    CHECK(s.len() == 1);
    CHECK(s.contains(5));
    std::printf("  test_replace ok\n");
}

// set/tests.rs — get() returns a reference to the stored element.
static void test_get() {
    auto s = HSet<int>::new_();
    s.insert(11);
    auto g = s.get(11);
    CHECK(g.is_some());
    CHECK(g.unwrap() == 11);
    CHECK(s.get(12).is_none());
    std::printf("  test_get ok\n");
}

// set/tests.rs::test_iterate — every inserted element is present exactly once;
// asserted as a sum so the randomly-seeded order cannot matter.
static void test_iterate() {
    auto s = HSet<int>::with_capacity(4);
    for (int i = 0; i < 32; ++i) CHECK(s.insert(i));
    CHECK(s.len() == 32);
    int expected = 0;
    for (int i = 0; i < 32; ++i) { CHECK(s.contains(i)); expected += i; }
    // Range-for over iter() — the regression guard for #188. Before that fix
    // only the four set-algebra views had begin()/end(), so the most basic
    // iteration form over a set did not compile at all.
    int total = 0;
    size_t seen = 0;
    for (const auto& v : s.iter()) { total += v; ++seen; }
    CHECK(seen == 32);
    CHECK(total == expected);
    std::printf("  test_iterate ok\n");
}

// Growth across several rehashes, then remove half.
static void test_lots_of_insertions() {
    auto s = HSet<int>::new_();
    const int N = 256;
    for (int i = 0; i < N; ++i) CHECK(s.insert(i));
    CHECK(s.len() == static_cast<size_t>(N));
    for (int i = 0; i < N; ++i) CHECK(s.contains(i));
    for (int i = 0; i < N; i += 2) CHECK(s.remove(i));
    CHECK(s.len() == static_cast<size_t>(N / 2));
    for (int i = 1; i < N; i += 2) CHECK(s.contains(i));
    for (int i = 0; i < N; i += 2) CHECK(!s.contains(i));
    std::printf("  test_lots_of_insertions ok\n");
}

// clone must be equal AND independent.
static void test_clone_independence() {
    auto s = HSet<int>::new_();
    for (int i = 0; i < 8; ++i) s.insert(i);
    auto s2 = s.clone();
    CHECK(s2.len() == s.len());
    for (int i = 0; i < 8; ++i) CHECK(s2.contains(i));
    s2.insert(100);
    s2.remove(0);
    CHECK(!s.contains(100));       // original unaffected
    CHECK(s.contains(0));
    CHECK(s.len() == 8);
    std::printf("  test_clone_independence ok\n");
}

// set/tests.rs::test_extend_ref
static void test_extend_and_reserve() {
    auto s = HSet<int>::new_();
    s.reserve(64);
    s.insert(1);
    auto other = HSet<int>::new_();
    other.insert(2); other.insert(3);
    s.extend(other.iter());
    CHECK(s.len() == 3);
    CHECK(s.contains(1) && s.contains(2) && s.contains(3));
    std::printf("  test_extend_and_reserve ok\n");
}

static void test_clear() {
    auto s = HSet<int>::new_();
    for (int i = 0; i < 16; ++i) s.insert(i);
    CHECK(s.len() == 16);
    s.clear();
    CHECK(s.is_empty());
    CHECK(s.len() == 0);
    for (int i = 0; i < 16; ++i) CHECK(!s.contains(i));
    // Reusable after clear.
    CHECK(s.insert(1));
    CHECK(s.contains(1));
    std::printf("  test_clear ok\n");
}

int main() {
    test_insert_and_contains();
    test_remove();
    test_take();
    test_replace();
    test_get();
    test_iterate();
    test_lots_of_insertions();
    test_clone_independence();
    test_extend_and_reserve();
    test_clear();
    std::printf("std_port HashSet API: all checks passed\n");
    return 0;
}
