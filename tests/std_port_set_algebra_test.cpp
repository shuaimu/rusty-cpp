// HashSet set-algebra views: range-for + count, against the transpiled Rust
// std port DIRECTLY (`import std_port;`), not through the `rusty` umbrella.
//
// WHY A SECOND TEST rather than extending hashset_set_algebra_test.cpp:
// that one reaches the same types via `import rusty;`, and post-retarget the
// umbrella path costs >10 MINUTES for this TU (measured; it was ~3.7s before
// rusty::HashSet was retargeted onto std_port). This direct-import TU compiles
// in ~2s, so the FUNCTIONAL coverage lives here where it is cheap to run. The
// umbrella test stays as the reachability guard for the alias itself.
//
// Rust's HashSet::{union,intersection,difference,symmetric_difference} are
// Iterators, so `for x in a.intersection(&b)` is idiomatic. The ported views
// are next()-shaped, so they carry begin()/end() members bridging to
// rusty::rust_range_begin (include/rusty/array.hpp); the members are
// (re)applied on regeneration by docs/rusty/post_transpile_patch.py.
//
// The members must be MEMBERS, not free functions: for
// Intersection<T, S, rusty::alloc::Global> the ADL-associated namespace is
// rusty::alloc (from the allocator argument), NOT rusty, so a free begin()/
// end() in namespace rusty would never be found.
import std_port;

#include <cstdio>
#include <cstdlib>
#include <rusty/rusty.hpp>

// NOT assert(): these test targets are compiled with -DNDEBUG, which would
// make every check below a no-op and the suite vacuously green.
#define CHECK(cond)                                                            \
    do {                                                                       \
        if (!(cond)) {                                                         \
            std::printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond);        \
            std::abort();                                                      \
        }                                                                      \
    } while (0)

template <typename T>
using HSet = ::std_port::collections::hash::set::HashSet<T>;

int main() {
    auto a = HSet<int>::new_();
    auto b = HSet<int>::new_();
    for (int x : {1, 2, 3, 4}) a.insert(x);
    for (int x : {3, 4, 5, 6}) b.insert(x);

    // Sums, not element sequences: std's RandomState is randomly seeded, so
    // iteration order legitimately differs run to run. Sums are order-free.
    int inter = 0;
    for (const auto& v : a.intersection(b)) inter += v;
    CHECK(inter == 3 + 4);

    int uni = 0;
    for (const auto& v : a.union_(b)) uni += v;
    CHECK(uni == 1 + 2 + 3 + 4 + 5 + 6);

    int diff = 0;
    for (const auto& v : a.difference(b)) diff += v;
    CHECK(diff == 1 + 2);

    int rdiff = 0;
    for (const auto& v : b.difference(a)) rdiff += v;
    CHECK(rdiff == 5 + 6);

    int sym = 0;
    for (const auto& v : a.symmetric_difference(b)) sym += v;
    CHECK(sym == 1 + 2 + 5 + 6);

    // count() is next()-based and would keep passing even if range-for
    // regressed, so it is a cross-check on the loops above, not a substitute.
    CHECK(rusty::count(a.intersection(b)) == 2);
    CHECK(rusty::count(a.union_(b)) == 6);
    CHECK(rusty::count(a.difference(b)) == 2);
    CHECK(rusty::count(a.symmetric_difference(b)) == 4);

    // Empty-side views must terminate immediately rather than run off the end.
    auto empty = HSet<int>::new_();
    int e = 0;
    for (const auto& v : empty.intersection(a)) e += v;
    CHECK(e == 0);
    for (const auto& v : empty.difference(a)) e += v;
    CHECK(e == 0);
    int eu = 0;
    for (const auto& v : empty.union_(a)) eu += v;
    CHECK(eu == 1 + 2 + 3 + 4);

    std::printf("std_port set-algebra range-for: all checks passed\n");
    return 0;
}
