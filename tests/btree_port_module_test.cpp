// Smoke + multi-node stress test for the transpiled btree_port module.
//
// The smoke half proves the vendored module is consumable from regular .cpp
// (insert + get + first/last_key_value). The stress half exercises the
// MULTI-NODE paths that a single-node tree never reaches: with node CAPACITY
// = 11 (B=6), inserting 200 keys forces root splits and internal-node
// navigation, and removing keys forces node merges / rebalancing.
//
// The removal path was a genuine use-after-free: a `static` slice-ref cached
// the first node's edge span across all merges, corrupting the parent's edge
// array (see the emit_expr.rs slice-ref fix). This test is the regression
// guard; it fails hard if it returns.
//
// NOTE: this TU is built with -DNDEBUG, so `assert` is a no-op — every check
// (and every side-effecting call) is written explicitly, never inside assert.
//
// Under --auto-namespace the transpiled BTreeMap lives in
// `namespace btree_port::btree::map`.

import btree_port.btree.map;

#include <rusty/alloc.hpp>
#include <cstdint>
#include <cstdio>

using BTreeMap =
    ::btree_port::btree::map::BTreeMap<int32_t, int32_t, ::rusty::alloc::Global>;

#define CHECK(cond, ...) do { if (!(cond)) { std::printf(__VA_ARGS__); return 1; } } while (0)

static int smoke() {
    auto m = BTreeMap::new_in(::rusty::alloc::Global{});

    auto r0 = m.insert(1, 100);
    CHECK(r0.is_none(), "smoke: insert(1) not none\n");
    auto v0 = m.get(1);
    CHECK(v0.is_some() && v0.unwrap() == 100, "smoke: get(1) != 100\n");

    auto r1 = m.insert(1, 200);
    CHECK(r1.is_some() && r1.unwrap() == 100, "smoke: reinsert didn't return old\n");
    auto v1 = m.get(1);
    CHECK(v1.is_some() && v1.unwrap() == 200, "smoke: get(1) != 200\n");

    CHECK(m.first_key_value().is_some(), "smoke: first_key_value none\n");
    CHECK(m.last_key_value().is_some(), "smoke: last_key_value none\n");
    return 0;
}

// Insert 0..n, verify every key reads back, remove all even keys (forcing
// merges) and verify the odds survive, then drain the rest to empty.
static int stress(int n) {
    auto m = BTreeMap::new_in(::rusty::alloc::Global{});

    for (int32_t k = 0; k < n; ++k) {
        auto r = m.insert(k, k * 10);
        CHECK(r.is_none(), "stress(%d): dup insert at %d\n", n, k);
    }
    CHECK(m.len() == static_cast<size_t>(n), "stress(%d): len %zu != %d\n", n, m.len(), n);

    for (int32_t k = 0; k < n; ++k) {
        auto v = m.get(k);
        CHECK(v.is_some() && v.unwrap() == k * 10, "stress(%d): get FAIL at %d\n", n, k);
    }

    // remove all even keys -> node merges / rebalancing (the once-broken path)
    for (int32_t k = 0; k < n; k += 2) {
        auto removed = m.remove(k);
        CHECK(removed.is_some() && removed.unwrap() == k * 10, "stress(%d): remove FAIL at %d\n", n, k);
    }
    CHECK(m.len() == static_cast<size_t>(n / 2), "stress(%d): len-after-remove %zu\n", n, m.len());
    for (int32_t k = 0; k < n; ++k) {
        auto v = m.get(k);
        bool want = (k % 2 == 1);
        CHECK(want == v.is_some() && (!want || v.unwrap() == k * 10), "stress(%d): post-remove FAIL at %d\n", n, k);
    }

    // drain the surviving odds -> more merges down to empty
    for (int32_t k = 1; k < n; k += 2) {
        CHECK(m.remove(k).is_some(), "stress(%d): drain FAIL at %d\n", n, k);
    }
    CHECK(m.len() == 0, "stress(%d): drain len %zu != 0\n", n, m.len());
    return 0;
}

int main() {
    if (smoke() != 0) return 1;

    // 200 keys -> a height-2 tree. Also cover sizes just past the split
    // boundary (capacity 11) so a single-level tree is exercised too.
    for (int n : {13, 24, 200}) {
        if (stress(n) != 0) { std::printf("btree stress(%d) FAILED\n", n); return 1; }
        std::printf("btree multi-node stress(%d): OK\n", n);
    }

    std::printf("transpiled btree_port module: ALL CHECKS PASSED\n");
    return 0;
}
