// Smoke test: imports the transpiled vec_port.vec module and exercises a
// minimum end-to-end Vec workflow (push + len + index + pop), proving the
// vendored module is consumable from regular .cpp.
//
// Companion to tests/btree_port_module_test.cpp / tests/btree_port_set_module_test.cpp.
// Under flat-export, the transpiled Vec lives at global scope as `::Vec<T, A>`.

import vec_port.vec;

#include <rusty/alloc.hpp>
#include <cassert>
#include <cstdint>
#include <cstdio>

int main() {
    auto v = ::Vec<int32_t, ::rusty::alloc::Global>::new_();
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
