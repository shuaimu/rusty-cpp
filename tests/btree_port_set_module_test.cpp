// Smoke test: imports the transpiled btree_port.btree.set module and exercises
// a minimum end-to-end BTreeSet workflow (insert + contains + len), proving the
// vendored module is consumable from regular .cpp.
//
// Companion to tests/btree_port_module_test.cpp (which exercises BTreeMap).
// Under --auto-namespace, the transpiled BTreeSet lives in
// `namespace btree_port::btree::set`.

import btree_port.btree.set;

#include <rusty/alloc.hpp>
#include <cassert>
#include <cstdint>
#include <cstdio>

int main() {
    using BTreeSet = ::rusty::port::collections::btree::set::BTreeSet<
        int32_t, ::rusty::alloc::Global>;
    auto s = BTreeSet::new_in(::rusty::alloc::Global{});

    // insert returns bool (true if newly inserted).
    assert(s.insert(1));
    assert(s.insert(2));
    assert(!s.insert(1));   // duplicate

    // len + contains
    assert(s.len() == 2);
    assert(s.contains(1));
    assert(s.contains(2));
    assert(!s.contains(99));

    std::printf("transpiled btree_port set module smoke: ALL CHECKS PASSED\n");
    return 0;
}
