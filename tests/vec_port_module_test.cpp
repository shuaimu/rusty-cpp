// Smoke test: imports the transpiled vec_port.vec module and exercises a
// minimum end-to-end Vec workflow (push + len + index + pop), proving the
// vendored module is consumable from regular .cpp.
//
// Companion to tests/btree_port_module_test.cpp / tests/btree_port_set_module_test.cpp.
// After the deep-namespace migration the transpiled Vec lives at
// `::rusty::port::vec::Vec<T, A>` (previously `::Vec<T, A>` under flat-export).

import vec_port.vec;

#include <rusty/alloc.hpp>
#include <cassert>
#include <cstdint>
#include <cstdio>

int main() {
    auto v = ::rusty::port::vec::Vec<int32_t, ::rusty::alloc::Global>::new_();
    assert(v.len() == 0);
    assert(v.is_empty());

    v.push(10);
    v.push(20);
    v.push(30);
    assert(v.len() == 3);
    assert(!v.is_empty());

    auto popped = v.pop();
    assert(popped.is_some());
    assert(popped.unwrap() == 30);
    assert(v.len() == 2);

    std::printf("transpiled vec_port module smoke: ALL CHECKS PASSED\n");
    return 0;
}
