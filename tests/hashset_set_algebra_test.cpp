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
#include <cassert>
#include <cstdio>
#include <rusty/rusty.hpp>

int main() {
    auto a = rusty::HashSet<int>::new_();
    auto b = rusty::HashSet<int>::new_();
    for (int x : {1, 2, 3, 4}) a.insert(x);
    for (int x : {3, 4, 5, 6}) b.insert(x);

    assert(rusty::count(a.union_(b)) == 6);
    assert(rusty::count(a.intersection(b)) == 2);
    assert(rusty::count(a.difference(b)) == 2);
    assert(rusty::count(b.difference(a)) == 2);
    assert(rusty::count(a.symmetric_difference(b)) == 4);

    int inter_sum = 0;
    for (const auto& v : a.intersection(b)) inter_sum += v;
    assert(inter_sum == 3 + 4);

    auto empty = rusty::HashSet<int>::new_();
    assert(rusty::count(empty.union_(a)) == 4);
    assert(rusty::count(a.union_(empty)) == 4);
    assert(rusty::count(empty.intersection(a)) == 0);
    assert(rusty::count(empty.difference(a)) == 0);

    assert(!a.is_disjoint(b));
    auto c = rusty::HashSet<int>::new_();
    c.insert(1); c.insert(2);
    assert(c.is_subset(a) && a.is_superset(c) && !a.is_subset(c));
    auto d = rusty::HashSet<int>::new_();
    d.insert(100);
    assert(a.is_disjoint(d) && empty.is_subset(a) && empty.is_disjoint(a));

    std::printf("hashset set-algebra: all assertions passed\n");
    return 0;
}
