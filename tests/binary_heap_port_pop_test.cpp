// Phase C — sift-down exercise. Sister of binary_heap_port_push_test.cpp;
// this one drives pop() which is the path that hits clusters C4/C5/C6.

import binary_heap_port;

#include <rusty/alloc.hpp>
#include <cassert>
#include <cstdint>
#include <cstdio>

int main() {
    auto h = rusty::collections::BinaryHeap<int32_t, ::rusty::alloc::Global>::new_in(
        ::rusty::alloc::Global{});

    h.push(3);
    h.push(1);
    h.push(4);
    h.push(1);
    h.push(5);
    assert(h.len() == 5);

    auto top = h.pop();
    assert(top.is_some());
    assert(top.unwrap() == 5);  // max-heap: largest first
    assert(h.len() == 4);

    std::printf("binary_heap_port pop smoke (Phase C, sift-down): OK\n");
    return 0;
}
