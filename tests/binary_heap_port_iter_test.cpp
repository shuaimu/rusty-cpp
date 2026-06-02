// Iterator + bulk-construction coverage for binary_heap_port.
// Three impedance fixes unblock iter() / into_iter_sorted():
//   1. `Option<T&>(Option<U*>)` converting ctor in
//      include/rusty/option.hpp
//   2. `begin()`-returning-pointer + `size()` arm in `rusty::iter`
//      (include/rusty/slice.hpp)
//   3. Restored `BinaryHeap<T,A> inner;` field type in wrapper
//      structs (IntoIterSorted/DrainSorted/PeekMut) — an earlier
//      inline patcher had over-rewritten BinaryHeap → ::Vec; the
//      fresh transpile is correct.
// See rusty-std-book §6.10 for the full writeup.

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

static void test_iter_visits_all_elements() {
    auto h = BinaryHeap<int32_t, ::rusty::alloc::Global>::new_in(
        ::rusty::alloc::Global{});
    for (int v : {3, 1, 4, 1, 5}) h.push(v);

    auto it = h.iter();
    int count = 0;
    int sum = 0;
    while (true) {
        auto next = it.next();
        if (next.is_none()) break;
        sum += next.unwrap();
        ++count;
    }
    assert(count == 5);
    assert(sum == 3 + 1 + 4 + 1 + 5);
    // iter() is non-destructive
    assert(h.len() == 5);
}

static void test_into_iter_sorted_descending() {
    auto h = BinaryHeap<int32_t, ::rusty::alloc::Global>::new_in(
        ::rusty::alloc::Global{});
    for (int v : {3, 1, 4, 1, 5, 9, 2}) h.push(v);

    auto sorted = std::move(h).into_iter_sorted();
    int prev = INT32_MAX;
    int count = 0;
    while (true) {
        auto next = sorted.next();
        if (next.is_none()) break;
        int val = next.unwrap();
        assert(val <= prev);  // descending
        prev = val;
        ++count;
    }
    assert(count == 7);
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
    run("iter visits all elements",         test_iter_visits_all_elements);
    run("into_iter_sorted descending",      test_into_iter_sorted_descending);
    std::printf("binary_heap_port: iter tests passed\n");
    return 0;
}
