// Phase C smoke test for the transpiled BinaryHeap port — exercises
// push() + len() + is_empty(). Sister to binary_heap_port_module_test.cpp
// (the empty-heap-only Phase B smoke test); this one drives the
// actual push body so we can iterate on the 6 instantiation clusters
// documented in docs/binary_heap_port/STATUS.md.

import binary_heap_port;

#include <rusty/alloc.hpp>
#include <cassert>
#include <cstdint>
#include <cstdio>

int main() {
    auto h = rusty::port::collections::BinaryHeap<int32_t, ::rusty::alloc::Global>::new_in(
        ::rusty::alloc::Global{});

    // Push five scrambled values; max-heap should put the largest on top.
    h.push(3);
    h.push(1);
    h.push(4);
    h.push(1);
    h.push(5);
    assert(h.len() == 5);
    assert(!h.is_empty());

    std::printf("binary_heap_port push smoke (Phase C): OK (len=%zu)\n",
                static_cast<size_t>(h.len()));
    return 0;
}
