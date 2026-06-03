// Smoke test: imports the transpiled btree_port module and exercises a
// minimum end-to-end BTreeMap workflow (insert + get + first/last_key_value),
// proving the vendored module is consumable from regular .cpp.
//
// This test is the BTreeMap equivalent of tests/hashbrown_port_map_test.cpp.
// Under --auto-namespace (STATUS.md Step 88), the transpiled BTreeMap lives
// in `namespace btree_port::btree::map`. Earlier flat-export builds used
// `::BTreeMap` at global scope; this test uses the new namespace path.

import btree_port.btree.map;

#include <rusty/alloc.hpp>
#include <cassert>
#include <cstdint>
#include <cstdio>

int main() {
    using BTreeMap = ::rusty::port::collections::btree::map::BTreeMap<
        int32_t, int32_t, ::rusty::alloc::Global>;
    auto m = BTreeMap::new_in(::rusty::alloc::Global{});

    auto r0 = m.insert(1, 100);
    assert(r0.is_none());

    auto v0 = m.get(1);
    assert(v0.is_some());
    assert(v0.unwrap() == 100);

    auto r1 = m.insert(1, 200);
    assert(r1.is_some());
    assert(r1.unwrap() == 100);

    auto v1 = m.get(1);
    assert(v1.is_some());
    assert(v1.unwrap() == 200);

    auto fkv = m.first_key_value();
    assert(fkv.is_some());
    auto lkv = m.last_key_value();
    assert(lkv.is_some());

    std::printf("transpiled btree_port module smoke: ALL CHECKS PASSED\n");
    return 0;
}
