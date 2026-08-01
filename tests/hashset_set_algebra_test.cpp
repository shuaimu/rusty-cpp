// Runtime validation of the hashbrown-facade HashSet set algebra
// (union_/intersection/difference/symmetric_difference views + the
// boolean predicates) through the runtime `rusty` module — the surface
// rusty::HashSet aliases and module-mode probes hit.
// Build (against the module cache; run from repo root):
//   cd .rusty-modules-cache && \
//   MODFLAGS=$(for f in pcm/*.pcm; do n=$(basename $f .pcm); \
//     echo -n "-fmodule-file=$n=$PWD/$f "; done) && \
//   clang++ -std=c++23 -DRUSTY_PORTABLE_INTRINSICS=1 -march=native \
//     -I ../include $MODFLAGS ../tests/hashset_set_algebra_test.cpp \
//     -Wl,--start-group build/lib*.a -Wl,--end-group -o /tmp/hs_test && /tmp/hs_test
import rusty;
#include <cstdio>
#include <cstdlib>
#include <rusty/rusty.hpp>

// NOT assert(): CMakeLists.txt compiles this target with -DNDEBUG, which would
// make every check below a no-op and the whole suite vacuously green. Keep the
// checking self-contained so it cannot be switched off by a flag change.
#define CHECK(cond)                                                            \
    do {                                                                       \
        if (!(cond)) {                                                         \
            std::printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond);         \
            std::abort();                                                      \
        }                                                                      \
    } while (0)

int main() {
    auto a = rusty::HashSet<int>::new_();
    auto b = rusty::HashSet<int>::new_();
    for (int x : {1, 2, 3, 4}) a.insert(x);
    for (int x : {3, 4, 5, 6}) b.insert(x);

    CHECK(rusty::count(a.union_(b)) == 6);
    CHECK(rusty::count(a.intersection(b)) == 2);
    CHECK(rusty::count(a.difference(b)) == 2);
    CHECK(rusty::count(b.difference(a)) == 2);
    CHECK(rusty::count(a.symmetric_difference(b)) == 4);

    // Range-for over each lazy view. In Rust these are Iterators, so
    // `for x in a.intersection(&b)` is idiomatic; the ported views are
    // next()-shaped, so they carry begin()/end() bridging to rust_range_begin
    // (added by docs/rusty/post_transpile_patch.py). rusty::count above is
    // next()-based and would keep passing even if range-for regressed, so
    // exercise all four views explicitly.
    int inter_sum = 0;
    for (const auto& v : a.intersection(b)) inter_sum += v;
    CHECK(inter_sum == 3 + 4);

    int union_sum = 0;
    for (const auto& v : a.union_(b)) union_sum += v;
    CHECK(union_sum == 1 + 2 + 3 + 4 + 5 + 6);

    int diff_sum = 0;
    for (const auto& v : a.difference(b)) diff_sum += v;
    CHECK(diff_sum == 1 + 2);

    int symdiff_sum = 0;
    for (const auto& v : a.symmetric_difference(b)) symdiff_sum += v;
    CHECK(symdiff_sum == 1 + 2 + 5 + 6);

    auto empty = rusty::HashSet<int>::new_();
    CHECK(rusty::count(empty.union_(a)) == 4);
    CHECK(rusty::count(a.union_(empty)) == 4);
    CHECK(rusty::count(empty.intersection(a)) == 0);
    CHECK(rusty::count(empty.difference(a)) == 0);

    CHECK(!a.is_disjoint(b));
    auto c = rusty::HashSet<int>::new_();
    c.insert(1); c.insert(2);
    CHECK(c.is_subset(a) && a.is_superset(c) && !a.is_subset(c));
    auto d = rusty::HashSet<int>::new_();
    d.insert(100);
    CHECK(a.is_disjoint(d) && empty.is_subset(a) && empty.is_disjoint(a));

    std::printf("hashset set-algebra: all assertions passed\n");
    return 0;
}
