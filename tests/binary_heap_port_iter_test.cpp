// Iterator + bulk-construction coverage for binary_heap_port.
// Iter / IntoIterSorted / into_sorted_vec / append all have
// instantiation-time bugs (Cluster C6 in STATUS.md) — see the
// `disabled-tests/` section in this file. as_slice is the only
// path that works today.

import binary_heap_port;

#include <rusty/alloc.hpp>
#include <cassert>
#include <cstdint>
#include <cstdio>

using binary_heap_port::BinaryHeap;

static void test_as_slice_view() {
    auto h = BinaryHeap<int32_t, ::rusty::alloc::Global>::new_in(
        ::rusty::alloc::Global{});
    h.push(7);
    h.push(3);
    h.push(9);
    auto s = h.as_slice();
    assert(s.size() == 3);
    assert(s[0] == 9);  // max-heap: max at index 0
}

static void run(const char* name, void (*fn)()) {
    std::printf("  %s ... ", name);
    std::fflush(stdout);
    fn();
    std::printf("ok\n");
}

int main() {
    std::printf("binary_heap_port (iter + bulk) tests:\n");
    run("as_slice view",                    test_as_slice_view);
    std::printf("binary_heap_port: as_slice test passed\n");
    return 0;
}
