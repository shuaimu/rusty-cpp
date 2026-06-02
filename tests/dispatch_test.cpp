// Tests for `rusty::deref_call` — the universal auto-deref dispatcher.

#include <rusty/dispatch.hpp>
#include <cassert>
#include <cstdio>

namespace {

// Test scenarios for the dispatcher.

// (1) Direct: receiver has the method, no deref needed.
struct HasIter {
    int iter() const { return 42; }
};

// (2) One-level deref: Vec doesn't have the method, deref to slice.
struct Slice {
    int iter() const { return 99; }
};
struct Vec {
    Slice slice{};
    Slice& operator*() { return slice; }
    const Slice& operator*() const { return slice; }
    // intentionally NO iter() — must walk through *vec to Slice
};

// (3) Two-level deref: Box<Vec> → Vec → Slice → iter.
struct BoxOfVec {
    Vec vec{};
    Vec& operator*() { return vec; }
    const Vec& operator*() const { return vec; }
    // no iter(), no direct slice access
};

// (4) First-match wins: if the wrapper itself has the method, use it
//     (don't walk through to the deref target).
struct CountingBox {
    Slice slice{};
    Slice& operator*() { return slice; }
    int iter() const { return 7;  /* not 99! */ }
};

// (5) Method with arguments.
struct ArgTaker {
    int sum(int a, int b) const { return a + b; }
};
struct WrappedArgTaker {
    ArgTaker inner{};
    ArgTaker& operator*() { return inner; }
    const ArgTaker& operator*() const { return inner; }
};

// (6) Method that returns by reference (lifetime preservation).
struct HasRef {
    int storage{77};
    int& get_ref() { return storage; }
};

static void test_direct_call() {
    HasIter h{};
    auto v = rusty::deref_call(h, [](auto&& r) -> decltype(r.iter()) { return r.iter(); });
    assert(v == 42);
}

static void test_one_level_deref() {
    Vec v{};
    auto x = rusty::deref_call(v, [](auto&& r) -> decltype(r.iter()) { return r.iter(); });
    assert(x == 99);
}

static void test_two_level_deref() {
    BoxOfVec b{};
    auto x = rusty::deref_call(b, [](auto&& r) -> decltype(r.iter()) { return r.iter(); });
    assert(x == 99);  // walked Box → Vec → Slice
}

static void test_first_match_wins() {
    CountingBox cb{};
    auto x = rusty::deref_call(cb, [](auto&& r) -> decltype(r.iter()) { return r.iter(); });
    // Should pick CountingBox::iter (returns 7), NOT walk to Slice (would be 99).
    assert(x == 7);
}

static void test_method_with_args() {
    WrappedArgTaker w{};
    int a = 3, b = 4;
    auto x = rusty::deref_call(w, [&](auto&& r) -> decltype(r.sum(a, b)) {
        return r.sum(a, b);
    });
    assert(x == 7);
}

static void test_reference_return() {
    HasRef h{};
    // Mutate through the returned reference; confirm decltype(auto)
    // preserved reference-ness.
    rusty::deref_call(h, [](auto&& r) -> decltype(r.get_ref()) { return r.get_ref(); }) = 100;
    assert(h.storage == 100);
}

// (7) Chained-call shape — A.B().C() with universal dispatcher.
struct Chain1 {
    int value{};
    Chain1 add(int x) const { return Chain1{value + x}; }
    int finalize() const { return value; }
};

static void test_chained_calls() {
    Chain1 c{10};
    // Mirrors what the transpiler would emit for `c.add(5).finalize()`.
    auto x = rusty::deref_call(
        rusty::deref_call(c, [&](auto&& r) -> decltype(r.add(5)) { return r.add(5); }),
        [&](auto&& r) -> decltype(r.finalize()) { return r.finalize(); }
    );
    assert(x == 15);
}

static void run(const char* name, void (*fn)()) {
    std::printf("  %s ... ", name);
    std::fflush(stdout);
    fn();
    std::printf("ok\n");
}

}  // namespace

int main() {
    std::printf("rusty::deref_call tests:\n");
    run("direct call (no deref)",       test_direct_call);
    run("one-level deref",              test_one_level_deref);
    run("two-level deref",              test_two_level_deref);
    run("first-match wins (no walk)",   test_first_match_wins);
    run("method with arguments",        test_method_with_args);
    run("reference return preserved",   test_reference_return);
    run("chained A.B().C() shape",      test_chained_calls);
    std::printf("rusty::deref_call: all tests passed\n");
    return 0;
}
