// Smoke test for the transpiled BinaryHeap port. Phase B level — proves
// the library links and `BinaryHeap<T, A>::new_in(A)` constructs an
// empty heap. The push/pop/peek paths still trigger instantiation-time
// issues in the transpiled body (`rusty::ptr::read` / `copy_nonoverlapping`
// mismatches, `ManuallyDrop<int>` → `int` conversion gap, `std::swap` on
// the internal `Hole` helper), documented in docs/binary_heap_port/STATUS.md
// as the remaining Phase C work.

import binary_heap_port;

#include <rusty/alloc.hpp>
#include <cassert>
#include <cstdint>
#include <cstdio>

int main() {
    auto h = rusty::port::collections::BinaryHeap<int32_t, ::rusty::alloc::Global>::new_in(
        ::rusty::alloc::Global{});

    // Empty-heap invariants — these don't trigger the push/pop bodies.
    assert(h.len() == 0);
    assert(h.is_empty());
    assert(h.capacity() == 0);

    std::printf("binary_heap_port module smoke (Phase B — empty-heap only): ALL CHECKS PASSED\n");
    return 0;
}
