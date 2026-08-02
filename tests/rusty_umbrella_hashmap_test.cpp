// Runtime validation of rusty::HashMap reached through the `import rusty;`
// UMBRELLA — the alias declared in include/rusty/rusty.cppm over the transpiled
// Rust std port.
//
// WHY THIS FILE EXISTS. Until now exactly one test imported the umbrella
// (hashset_set_algebra_test.cpp) and it exercised rusty::HashSet only, so
// rusty::HashMap through `import rusty;` had NO coverage at all. That is the
// path that broke twice in a row and both times silently:
//
//   #183  `import rusty;` itself was unusable — clang either SIGSEGV'd in
//         codegen or hung 40+ minutes in ASTReader, because a CMake compile-flag
//         delta forked std_port's BMI so the umbrella and its consumers loaded
//         two different files for one module.
//   #185  the transpiler still routed rusty::HashMap at the retired hashbrown
//         port, whose entry API does not compile (`VacantEntry` has no
//         `into_mut`; `OccupiedEntry::insert` is not const-correct).
//
// The six std_port_*.cpp suites could not have caught either: they all
// `import std_port;` directly, which bypasses the umbrella alias entirely.
// So this file deliberately spells every type as `rusty::…` and never names
// std_port, making the alias itself load-bearing for the test to compile.
import rusty;

#include <cstdio>
#include <cstdlib>
#include <tuple>
#include <rusty/rusty.hpp>

// NOT assert(): this target compiles with -DNDEBUG, which would make every
// check a no-op and the suite vacuously green. fflush before abort or the
// message is lost when stdout is piped.
#define CHECK(cond)                                                            \
    do {                                                                       \
        if (!(cond)) {                                                         \
            std::printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond);        \
            std::fflush(stdout);                                               \
            std::abort();                                                      \
        }                                                                      \
    } while (0)

// The umbrella alias takes <K, V, S = RandomState>; std_port's underlying
// template takes a fourth allocator parameter that the alias leaves defaulted.
// Naming only two arguments here is the whole point — it proves the alias's
// defaults are intact, not just that some HashMap exists somewhere.
static void test_alias_resolves_and_basic_ops() {
    auto m = rusty::HashMap<int, int>::new_();
    CHECK(m.is_empty());
    CHECK(m.insert(1, 10).is_none());
    CHECK(m.insert(2, 20).is_none());
    auto old = m.insert(1, 11);
    CHECK(old.is_some());
    CHECK(old.unwrap() == 10);          // overwrite returns the previous value
    CHECK(m.len() == 2);                // ...and does not append
    CHECK(m.get(1).unwrap() == 11);
    CHECK(m.get(2).unwrap() == 20);
    CHECK(m.get(99).is_none());
    CHECK(m.contains_key(1));
    CHECK(!m.contains_key(99));
    std::printf("  test_alias_resolves_and_basic_ops ok\n");
}

// The entry API is the specific surface that did NOT compile against the
// retired hashbrown port, so it is the sharpest regression guard for #185.
static void test_entry_api_through_umbrella() {
    auto m = rusty::HashMap<int, int>::new_();
    m.insert(1, 10);
    CHECK(m.entry(1).or_insert(99) == 10);   // occupied: existing value wins
    CHECK(m.entry(2).or_insert(20) == 20);   // vacant: default installed
    CHECK(m.len() == 2);

    // or_insert hands back a MUTABLE reference into the table; a copy would
    // silently swallow the write, so read back through the map.
    m.entry(3).or_insert(0) = 300;
    CHECK(m.get(3).unwrap() == 300);
    m.entry(3).or_insert(0) += 7;
    CHECK(m.get(3).unwrap() == 307);

    // and_modify runs only when occupied.
    int calls = 0;
    m.entry(1).and_modify([&](int& v) { ++calls; v += 5; }).or_insert(0);
    CHECK(calls == 1);
    CHECK(m.get(1).unwrap() == 15);
    calls = 0;
    m.entry(42).and_modify([&](int& v) { ++calls; v += 5; }).or_insert(77);
    CHECK(calls == 0);
    CHECK(m.get(42).unwrap() == 77);
    std::printf("  test_entry_api_through_umbrella ok\n");
}

// `rusty::collections::HashMap` is a SECOND alias in rusty.cppm, distinct from
// `rusty::HashMap`. It had no coverage either.
static void test_collections_namespace_alias() {
    auto m = rusty::collections::HashMap<int, int>::new_();
    m.insert(7, 70);
    CHECK(m.get(7).unwrap() == 70);
    CHECK(m.len() == 1);
    auto s = rusty::collections::HashSet<int>::new_();
    s.insert(7);
    CHECK(s.contains(7));
    std::printf("  test_collections_namespace_alias ok\n");
}

// Growth across several rehashes, then removal — catches a table that resolves
// but is wired to the wrong storage.
static void test_growth_and_removal() {
    auto m = rusty::HashMap<int, int>::new_();
    const int N = 256;
    for (int i = 0; i < N; ++i) m.insert(i, i * 3);
    CHECK(m.len() == static_cast<size_t>(N));
    for (int i = 0; i < N; ++i) CHECK(m.get(i).unwrap() == i * 3);
    for (int i = 0; i < N; i += 2) CHECK(m.remove(i).is_some());
    CHECK(m.len() == static_cast<size_t>(N / 2));
    for (int i = 1; i < N; i += 2) CHECK(m.get(i).unwrap() == i * 3);
    for (int i = 0; i < N; i += 2) CHECK(m.get(i).is_none());
    // remove() on a never-allocated map is the #187 regression shape.
    auto empty = rusty::HashMap<int, int>::new_();
    CHECK(empty.remove(0).is_none());
    std::printf("  test_growth_and_removal ok\n");
}

// Iteration through the umbrella. RandomState is randomly seeded, so assert
// sums and counts — never a sequence.
static void test_iteration() {
    auto m = rusty::HashMap<int, int>::new_();
    for (int i = 1; i <= 5; ++i) m.insert(i, i * 10);

    int ksum = 0; size_t kn = 0;
    for (const auto& k : m.keys()) { ksum += k; ++kn; }
    CHECK(kn == 5);
    CHECK(ksum == 1 + 2 + 3 + 4 + 5);

    int vsum = 0; size_t vn = 0;
    for (const auto& v : m.values()) { vsum += v; ++vn; }
    CHECK(vn == 5);
    CHECK(vsum == 150);

    int isum = 0; size_t in = 0;
    for (const auto& kv : m.iter()) { isum += std::get<0>(kv) + std::get<1>(kv); ++in; }
    CHECK(in == 5);
    CHECK(isum == 15 + 150);

    // Mutation through values_mut must reach the table.
    for (auto& v : m.values_mut()) v += 1;
    for (int i = 1; i <= 5; ++i) CHECK(m.get(i).unwrap() == i * 10 + 1);
    std::printf("  test_iteration ok\n");
}

// Both umbrella aliases must coexist in one TU: the umbrella exports HashMap
// and HashSet from the same module, and an earlier shape of the shim (a
// using-declaration rather than an alias template) could collide.
static void test_map_and_set_together() {
    auto m = rusty::HashMap<int, int>::new_();
    auto s = rusty::HashSet<int>::new_();
    for (int i = 0; i < 16; ++i) { m.insert(i, i); s.insert(i); }
    CHECK(m.len() == 16);
    CHECK(s.len() == 16);
    int keys_in_set = 0;
    for (const auto& k : m.keys()) if (s.contains(k)) ++keys_in_set;
    CHECK(keys_in_set == 16);
    std::printf("  test_map_and_set_together ok\n");
}

int main() {
    test_alias_resolves_and_basic_ops();
    test_entry_api_through_umbrella();
    test_collections_namespace_alias();
    test_growth_and_removal();
    test_iteration();
    test_map_and_set_together();
    std::printf("rusty umbrella HashMap: all checks passed\n");
    return 0;
}
