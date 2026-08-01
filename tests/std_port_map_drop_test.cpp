// Drop-accounting and memory-management tests for the transpiled Rust std
// HashMap (std_port), translated from rustc's own
// library/std/src/collections/hash/map/tests.rs.
//
// WHY THIS FILE EXISTS: #178 retargeted rusty::HashMap onto std_port, but the
// shipped default path had NO runtime test constructing a HashMap at all —
// tests/hashbrown_port_map_test.cpp covers the RETIRED direct hashbrown port,
// and the parity matrix's only hash target is a known-fail. The flagship
// container was swapped onto a freshly transpiled implementation validated by
// essentially nothing. This file starts closing that gap, leading with the
// drop/ownership surface where a silent bug would actually live.
//
// `import std_port;` DIRECTLY, not `import rusty;` — the umbrella path
// currently hangs in ASTReader::ReadAST (see task #183). This TU compiles in
// ~2s. See tests/std_port_set_algebra_test.cpp for the same shape.
import std_port;

#include <cstdio>
#include <cstdlib>
#include <vector>
#include <rusty/rusty.hpp>

// NOT assert(): these targets compile with -DNDEBUG, which would make every
// check below a no-op and the suite vacuously green.
#define CHECK(cond)                                                            \
    do {                                                                       \
        if (!(cond)) {                                                          \
            std::printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond);         \
            std::fflush(stdout);  /* abort() does NOT flush; without this the  */ \
            std::abort();         /* failure message is lost when piped.       */                                                       \
        }                                                                       \
    } while (0)

template <typename K, typename V>
using HMap = ::std_port::collections::hash::map::HashMap<K, V>;

// ---------------------------------------------------------------------------
// Rust's tests use a thread-local DROP_VECTOR plus a `Droppable` that bumps
// slot[id] on construction and decrements it on drop; the invariant is that a
// live element reads 1 and a dropped one reads 0. Anything that leaks stays at
// 1 past its scope; anything double-dropped goes negative. That catches both
// directions, which a plain "destructor ran" counter does not.
// ---------------------------------------------------------------------------
static std::vector<int> DROP_VECTOR;

struct Droppable {
    size_t id;
    explicit Droppable(size_t i) : id(i) { DROP_VECTOR[id] += 1; }
    Droppable(const Droppable& o) : id(o.id) { DROP_VECTOR[id] += 1; }
    Droppable& operator=(const Droppable& o) {
        if (this != &o) {
            DROP_VECTOR[id] -= 1;
            id = o.id;
            DROP_VECTOR[id] += 1;
        }
        return *this;
    }
    ~Droppable() { DROP_VECTOR[id] -= 1; }
};

static void reset_drops(size_t n) {
    DROP_VECTOR.assign(n, 0);
}

static bool all_slots(size_t lo, size_t hi, int want) {
    for (size_t i = lo; i < hi; ++i) {
        if (DROP_VECTOR[i] != want) {
            std::printf("  slot[%zu] = %d, wanted %d\n", i, DROP_VECTOR[i], want);
            std::fflush(stdout);
            return false;
        }
    }
    return true;
}

// map/tests.rs::test_drops — insert 100, remove 50, verify accounting at each
// stage, then let the map die and verify EVERY slot returns to 0.
//
// This test FOUND bug #186 (erase paths skipped the husk destructor, one per
// removed element, across every port) and is now its regression guard. Fixed by
// giving rusty::ptr::read relocate semantics for non-trivially-destructible T.
static void test_drops() {
    reset_drops(200);
    {
        auto m = HMap<size_t, Droppable>::new_();
        CHECK(all_slots(0, 200, 0));

        for (size_t i = 0; i < 100; ++i) {
            m.insert(i, Droppable(i + 100));
        }
        CHECK(all_slots(100, 200, 1));

        for (size_t i = 0; i < 50; ++i) {
            auto removed = m.remove(i);
            CHECK(removed.is_some());
            // `removed` owns the value; the vacated slot must NOT still hold one.
            CHECK(DROP_VECTOR[i + 100] == 1);
        }
        CHECK(m.len() == 50);
        CHECK(all_slots(100, 150, 0));
        CHECK(all_slots(150, 200, 1));
    }
    // Nothing may leak (1) and nothing may double-drop (<0).
    CHECK(all_slots(0, 200, 0));
    std::printf("  test_drops ok\n");
}

// map/tests.rs::test_into_iter_drops — DISABLED: HashMap::into_iter() does not
// compile AT ALL on the shipped path, for any element type.
//   into_iter -> RawTable::into_iter_from -> RawTable::into_allocation
//   (hashbrown.cppm:7862 -> 6575 -> 6482)
// into_allocation's emitted IIFE returns `rusty::None` in one branch and
// `rusty::Some(std::make_tuple(...))` in the other with NO explicit return type:
//   error: return type 'Option<...>' must match previous return type 'None_t'
//          when lambda expression has unspecified explicit return type
// The deduction failure is type-independent, so every HashMap<K,V>::into_iter()
// instantiation fails. That is bug #180, and it went unnoticed because nothing
// on the shipped path ever called into_iter. Re-enable once #180 lands.
#if 0
static void test_into_iter_drops() { /* see above */ }
#endif

// map/tests.rs::test_clone — a clone must be independent; dropping one must not
// disturb the other's elements.
static void test_clone_independence() {
    reset_drops(200);
    {
        auto m = HMap<size_t, Droppable>::new_();
        for (size_t i = 0; i < 50; ++i) m.insert(i, Droppable(i + 100));
        CHECK(all_slots(100, 150, 1));
        {
            auto m2 = m.clone();
            CHECK(m2.len() == 50);
            // Each live element now exists twice.
            CHECK(all_slots(100, 150, 2));
        }
        // m2 gone, m intact.
        CHECK(all_slots(100, 150, 1));
        CHECK(m.len() == 50);
    }
    CHECK(all_slots(0, 200, 0));
    std::printf("  test_clone_independence ok\n");
}

// map/tests.rs::test_insert_overwrite — overwriting returns the OLD value and
// must drop exactly one of the two.
static void test_insert_overwrite_drops() {
    reset_drops(200);
    {
        auto m = HMap<size_t, Droppable>::new_();
        m.insert(1, Droppable(10));
        CHECK(DROP_VECTOR[10] == 1);
        {
            auto old = m.insert(1, Droppable(20));
            CHECK(old.is_some());
            CHECK(m.len() == 1);
            // Both alive: 10 held by `old`, 20 held by the map.
            CHECK(DROP_VECTOR[10] == 1);
            CHECK(DROP_VECTOR[20] == 1);
        }
        CHECK(DROP_VECTOR[10] == 0);   // old Option died
        CHECK(DROP_VECTOR[20] == 1);   // replacement still in the map
    }
    CHECK(all_slots(0, 200, 0));
    std::printf("  test_insert_overwrite_drops ok\n");
}

// map/tests.rs::test_lots_of_insertions — the rehash stress test. Growth
// reallocates and moves every element; a bug there shows up as a lost or
// double-dropped value rather than a wrong answer.
static void test_lots_of_insertions_drops() {
    const size_t N = 200;
    reset_drops(N);
    {
        auto m = HMap<size_t, Droppable>::new_();
        for (size_t i = 0; i < N; ++i) {
            m.insert(i, Droppable(i));
            CHECK(m.len() == i + 1);
        }
        // Every element survived however many rehashes happened.
        CHECK(all_slots(0, N, 1));
        for (size_t i = 0; i < N; ++i) CHECK(m.contains_key(i));

        m.clear();
        CHECK(m.len() == 0);
        CHECK(m.is_empty());
        CHECK(all_slots(0, N, 0));
    }
    CHECK(all_slots(0, N, 0));
    std::printf("  test_lots_of_insertions_drops ok\n");
}

// map/tests.rs::test_retain — retain drops exactly the rejected half.
// This was DISABLED because std_port's retain() did not compile at all:
// hashbrown.cppm:7626 emitted `f(std::move(key), std::move(value))` for Rust's
// `let &mut (ref key, ref mut value)`, and a `V&&` will not bind to the `V&` a
// retain predicate takes. That is bug #179, fixed in the emitter (fefdffe0);
// this test came back when the port was regenerated to pick the fix up.
static void test_retain_drops() {
    const size_t N = 100;
    reset_drops(N);
    {
        auto m = HMap<size_t, Droppable>::new_();
        for (size_t i = 0; i < N; ++i) m.insert(i, Droppable(i));
        CHECK(all_slots(0, N, 1));

        m.retain([](const size_t& k, Droppable&) { return k % 2 == 0; });
        CHECK(m.len() == N / 2);
        // Exactly the rejected half is destroyed; the kept half survives.
        for (size_t i = 0; i < N; ++i) {
            CHECK(DROP_VECTOR[i] == (i % 2 == 0 ? 1 : 0));
        }
    }
    CHECK(all_slots(0, N, 0));
    std::printf("  test_retain_drops ok\n");
}

// map/tests.rs::test_reserve_shrink_to_fit — capacity churn must not disturb
// element ownership.
static void test_reserve_drops() {
    const size_t N = 128;
    reset_drops(N);
    {
        auto m = HMap<size_t, Droppable>::new_();
        for (size_t i = 0; i < N; ++i) m.insert(i, Droppable(i));
        CHECK(all_slots(0, N, 1));

        m.reserve(1000);          // grow: moves everything
        CHECK(m.len() == N);
        CHECK(all_slots(0, N, 1));

        // The shrink half of this test needed remove(), which leaks (#186), so
        // it is omitted rather than asserted wrongly. Growth is still covered.
        // Also NOT calling m.shrink_to_fit() — it instantiates RawTable::
        // into_allocation(), whose emitted IIFE returns `rusty::None` in one
        // branch and `rusty::Some(...)` in the other with NO explicit return
        // type, so clang cannot deduce it:
        //   error: return type 'Option<...>' must match previous return type
        //          'None_t' when lambda expression has unspecified return type
        // That is bug #180, still open, and it means HashMap::shrink_to_fit
        // does not compile on the shipped path either. Restore the call here
        // once #180 lands — this is its regression guard.
        CHECK(m.len() == N);
        CHECK(all_slots(0, N, 1));
    }
    CHECK(all_slots(0, N, 0));
    std::printf("  test_reserve_drops ok\n");
}

int main() {
    test_drops();
    test_clone_independence();
    test_insert_overwrite_drops();
    test_lots_of_insertions_drops();
    test_retain_drops();
    test_reserve_drops();
    std::printf("std_port HashMap drop/ownership: all checks passed\n");
    return 0;
}
