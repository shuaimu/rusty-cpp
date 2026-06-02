// Iterator + bulk-construction coverage for binary_heap_port.
// Two impedance fixes unblock iter(): the `Option<T&>(Option<U*>)`
// converting ctor in include/rusty/option.hpp and the new
// `begin()`-returning-pointer + `size()` arm in `rusty::iter`
// (include/rusty/slice.hpp). See rusty-std-book §6.10 for the
// full writeup.
//
// IntoIterSorted is still broken (separate transpiler bug:
// `inner: BinaryHeap<T,A>` emitted as `inner: Vec<T,A>`).

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
    std::printf("binary_heap_port: iter tests passed\n");
    return 0;
}
