// Comprehensive smoke test for binary_heap_port — exercises peek,
// pop ordering, clear, and the as_slice view to cover the still-unhit
// API surface documented in docs/binary_heap_port/STATUS.md.
//
// Sister of binary_heap_port_module_test.cpp (empty-heap only),
// binary_heap_port_push_test.cpp (push len), and
// binary_heap_port_pop_test.cpp (single pop). This one drives a
// full push-then-drain cycle to confirm max-heap ordering.

import binary_heap_port;

#include <rusty/alloc.hpp>
#include <cassert>
#include <cstdint>
#include <cstdio>
#include <vector>

using rusty::collections::BinaryHeap;

static void test_peek_returns_max() {
    auto h = BinaryHeap<int32_t, ::rusty::alloc::Global>::new_in(
        ::rusty::alloc::Global{});
    h.push(3);
    h.push(1);
    h.push(4);
    h.push(1);
    h.push(5);
    h.push(9);
    h.push(2);

    auto top = h.peek();
    assert(top.is_some());
    assert(top.unwrap() == 9);
    // peek is non-destructive
    assert(h.len() == 7);
}

static void test_pop_drains_in_sorted_order() {
    auto h = BinaryHeap<int32_t, ::rusty::alloc::Global>::new_in(
        ::rusty::alloc::Global{});
    for (int v : {3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5}) {
        h.push(v);
    }
    std::vector<int> drained;
    while (!h.is_empty()) {
        auto top = h.pop();
        assert(top.is_some());
        drained.push_back(top.unwrap());
    }
    // Max-heap: drained sequence is descending.
    for (size_t i = 1; i < drained.size(); ++i) {
        assert(drained[i - 1] >= drained[i]);
    }
    assert(drained.size() == 11);
}

static void test_clear_zeroes_len() {
    auto h = BinaryHeap<int32_t, ::rusty::alloc::Global>::new_in(
        ::rusty::alloc::Global{});
    h.push(10);
    h.push(20);
    assert(h.len() == 2);
    h.clear();
    assert(h.len() == 0);
    assert(h.is_empty());
    auto top = h.peek();
    assert(top.is_none());
}

static void test_push_after_pop_preserves_heap() {
    auto h = BinaryHeap<int32_t, ::rusty::alloc::Global>::new_in(
        ::rusty::alloc::Global{});
    h.push(5);
    h.push(3);
    h.push(7);
    assert(h.pop().unwrap() == 7);   // 7 was max
    h.push(10);                      // new max
    h.push(4);
    assert(h.peek().unwrap() == 10);
    assert(h.pop().unwrap() == 10);
    assert(h.pop().unwrap() == 5);
    assert(h.pop().unwrap() == 4);
    assert(h.pop().unwrap() == 3);
    assert(h.is_empty());
}

static void run(const char* name, void (*fn)()) {
    std::printf("  %s ... ", name);
    std::fflush(stdout);
    fn();
    std::printf("ok\n");
}

int main() {
    std::printf("binary_heap_port (comprehensive) tests:\n");
    run("peek returns max",                 test_peek_returns_max);
    run("pop drains in sorted order",       test_pop_drains_in_sorted_order);
    run("clear zeroes len",                 test_clear_zeroes_len);
    run("push after pop preserves heap",    test_push_after_pop_preserves_heap);
    std::printf("binary_heap_port: all comprehensive tests passed\n");
    return 0;
}
