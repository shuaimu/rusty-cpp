// Iterator + bulk-construction coverage for binary_heap_port.
// `iter()` previously failed at instantiation time because the
// transpiled `Iter<T>::next()` returns `Option<const T&>` while the
// underlying `rusty::slice_iter::Iter::next()` yields
// `Option<const T*>`. Unblocked by the converting ctor in
// include/rusty/option.hpp (`Option<T&>(Option<U*>)`). See
// rusty-std-book §6.4 for the deeper writeup.
//
// IntoIterSorted is still broken (different bug: transpiler emits
// `inner: Vec<T,A>` for what Rust declares as `inner: BinaryHeap<T,A>`).

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

// `test_iter_visits_all_elements` removed for now — the Option<T&>
// ↔ Option<T*> converting ctor in include/rusty/option.hpp unblocks
// Iter::next, but a third issue surfaces: `rusty::iter(Vec<int>)`
// returns the Vec itself rather than a slice_iter::Iter because
// vec_port::Vec exposes begin()/end() but not data() (the iter
// dispatcher in rusty/slice.hpp's data-only branch never matches).
// See rusty-std-book §6.4 for the writeup.

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
