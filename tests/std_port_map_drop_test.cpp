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
// `import std_port;` DIRECTLY, not via the `import rusty;` umbrella: these tests
// target the std port itself, so they should not depend on the umbrella's
// re-export aliases. (The umbrella used to hang in ASTReader::ReadAST — #183, a
// forked std_port BMI from a CMake flag delta — but that is fixed; this is now a
// choice, not a workaround.) See tests/std_port_set_algebra_test.cpp for the
// same shape.
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

// map/tests.rs::test_into_iter_drops — RE-ENABLED: this was disabled because
// HashMap::into_iter() did not compile at all (into_iter -> into_iter_from ->
// into_allocation, whose emitted IIFE mixed `return rusty::None;` and
// `return rusty::Some(..);` with no explicit return type — bug #180). #180
// landed (79400f43: tail-returned if/else locals carry the fn return type) and
// the port was re-vendored with the annotated IIFE; this test is now the
// regression guard for that whole chain.
//
// Consume HALF the iterator, check the yielded elements dropped and the
// unyielded half is still live inside the iterator, then drop the iterator —
// its Drop must release the rest (that is into_allocation's actual job).
static void test_into_iter_drops() {
    reset_drops(200);
    {
        auto m = HMap<size_t, Droppable>::new_();
        for (size_t i = 0; i < 100; ++i) m.insert(i, Droppable(i + 100));
        CHECK(all_slots(100, 200, 1));
        {
            auto it = std::move(m).into_iter();
            for (size_t taken = 0; taken < 50; ++taken) {
                auto kv = it.next();
                CHECK(kv.is_some());
                // kv (Option<tuple<K, Droppable>>) dies at the end of this
                // iteration, dropping the yielded value.
            }
            // RandomState randomizes WHICH 50 were yielded — count, not slots.
            int live = 0;
            for (size_t i = 100; i < 200; ++i) live += DROP_VECTOR[i];
            CHECK(live == 50);
        } // iterator dropped: the unyielded 50 must be released here
        CHECK(all_slots(100, 200, 0));
        // `m` was consumed by into_iter; its scope-exit dtor must be a no-op.
    }
    CHECK(all_slots(100, 200, 0));
    std::printf("  test_into_iter_drops ok\n");
}

// into_keys / into_values — the other two #180-unlocked consuming iterators.
// into_keys must DROP every value even though none is ever yielded;
// into_values the mirror image.
static void test_into_keys_into_values_drop() {
    reset_drops(200);
    {
        auto m = HMap<size_t, Droppable>::new_();
        for (size_t i = 0; i < 20; ++i) m.insert(i, Droppable(i + 100));
        auto ks = std::move(m).into_keys();
        size_t n = 0, ksum = 0;
        for (auto k = ks.next(); k.is_some(); k = ks.next()) {
            ksum += k.unwrap();
            ++n;
        }
        CHECK(n == 20);
        CHECK(ksum == 190);  // 0+1+..+19
    }
    CHECK(all_slots(100, 120, 0));  // values dropped despite never being yielded

    reset_drops(200);
    {
        auto m = HMap<size_t, Droppable>::new_();
        for (size_t i = 0; i < 20; ++i) m.insert(i, Droppable(i + 100));
        auto vs = std::move(m).into_values();
        size_t n = 0;
        for (auto v = vs.next(); v.is_some(); v = vs.next()) ++n;
        CHECK(n == 20);
    }
    CHECK(all_slots(100, 120, 0));
    std::printf("  test_into_keys_into_values_drop ok\n");
}

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

        // The shrink half, RESTORED. It was omitted twice over: remove()
        // leaked a destructor per call (#186/#189, fixed — ptr::copy relocates
        // and Option's move destroys the moved-from payload), and
        // shrink_to_fit did not even compile (it instantiates RawTable::
        // into_allocation, the #180 unannotated-IIFE failure, fixed in
        // 79400f43 and re-vendored). This block is the regression guard for
        // all three.
        for (size_t i = 0; i < N / 2; ++i) CHECK(m.remove(i).is_some());
        CHECK(m.len() == N / 2);
        CHECK(all_slots(0, N / 2, 0));      // removed half fully dropped
        CHECK(all_slots(N / 2, N, 1));      // kept half intact
        m.shrink_to_fit();                  // rebuild at smaller capacity
        CHECK(m.len() == N / 2);
        CHECK(all_slots(0, N / 2, 0));
        CHECK(all_slots(N / 2, N, 1));      // shrink moved, didn't drop/dupe
    }
    CHECK(all_slots(0, N, 0));
    std::printf("  test_reserve_drops ok\n");
}

int main() {
    test_drops();
    test_into_iter_drops();
    test_into_keys_into_values_drop();
    test_clone_independence();
    test_insert_overwrite_drops();
    test_lots_of_insertions_drops();
    test_retain_drops();
    test_reserve_drops();
    std::printf("std_port HashMap drop/ownership: all checks passed\n");
    return 0;
}
