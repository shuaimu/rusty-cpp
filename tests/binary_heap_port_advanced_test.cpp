// Extends binary_heap_port coverage past push/pop/peek/iter into the
// consume / bulk-build / mutation API surface. Sister to:
//   - binary_heap_port_module_test (empty-heap invariants)
//   - binary_heap_port_push/pop_test (single ops)
//   - binary_heap_port_comprehensive_test (peek + drain ordering)
//   - binary_heap_port_iter_test (.iter(), .into_iter_sorted())
//
// Each test below is a focused single-API exercise so a compile failure
// pinpoints the impedance source (rather than a giant fixture that
// masks which arm broke).
//
// NEW IMPEDANCES surfaced by this file's first instantiation pass
// (documented in docs/binary_heap_port/STATUS.md §"Advanced API
// impedances"). Tests that depend on these are guarded behind
// `#if BHP_ADV_*` macros — flipped on once the respective patch lands.
//
//   D1. `into_vec()` body emits `::Vec<T, A>{.data = std::move(vec)}`
//       — designated init of non-aggregate Vec.
//   D2. Sift-down emits `std::swap(ptr_shadow1, rusty::ptr::add(ptr,
//       end))` — std::swap rejects an rvalue 2nd arg.
//   D3. `Vec::from_iter<binary_heap_port::Iter<int>>` referenced
//       before its deduced-return-type definition is visible across
//       the module boundary.
//   D4. `RebuildOnDrop<T, A>(heap_ref, len)` constructor signature
//       mismatch (used by append + retain).

import binary_heap_port;
import vec_port.vec;

#include <rusty/alloc.hpp>
#include <cassert>
#include <cstdint>
#include <cstdio>
#include <vector>

using rusty::port::collections::BinaryHeap;

static auto make_heap(std::initializer_list<int> values) {
    auto h = BinaryHeap<int32_t, ::rusty::alloc::Global>::new_in(
        ::rusty::alloc::Global{});
    for (int v : values) h.push(v);
    return h;
}

static void test_with_capacity_in_preallocates() {
    auto h = BinaryHeap<int32_t, ::rusty::alloc::Global>::with_capacity_in(
        16, ::rusty::alloc::Global{});
    assert(h.is_empty());
    assert(h.len() == 0);
    assert(h.capacity() >= 16);
    for (int i = 0; i < 16; ++i) h.push(i);
    assert(h.len() == 16);
    assert(h.peek().unwrap() == 15);
}

static void test_drain_yields_all_elements() {
    auto h = make_heap({3, 1, 4, 1, 5});
    auto d = h.drain();
    int count = 0;
    int sum = 0;
    while (true) {
        auto next = d.next();
        if (next.is_none()) break;
        sum += next.unwrap();
        ++count;
    }
    assert(count == 5);
    assert(sum == 3 + 1 + 4 + 1 + 5);
    // drain() leaves the heap empty.
    assert(h.is_empty());
}

// -- Blocked tests (turn ON once the impedance lands) --

// D1 fixed: transpiler emit bug at line 4269 — outer wrapper was
// `::Vec<T,A>` instead of `BinaryHeap<T,A>`. Patched inline in the
// vendored cppm. (Tests `from(Vec)`-using surface; `into_vec` does not
// itself trip this — D1 is misnamed but the patch unblocks several
// tests anyway.)
static void test_into_vec_consumes() {
    auto h = make_heap({7, 3, 5});
    auto v = std::move(h).into_vec();
    assert(v.len() == 3);
    int sum = 0;
    for (size_t i = 0; i < v.len(); ++i) sum += v[i];
    assert(sum == 15);
}

// D2/D3 fixed: ptr::swap restored in rusty/ptr.hpp + cppm call site
// patched (D2); SpecFromIter::from_iter out-of-line definition added
// in vec_port.vec.cppm (D3). Unblocks from(Vec), into_sorted_vec, and
// drain_sorted.
static void test_from_vec_bulk_builds() {
    auto v = ::Vec<int32_t, ::rusty::alloc::Global>::new_in(
        ::rusty::alloc::Global{});
    for (int x : {3, 1, 4, 1, 5, 9, 2, 6}) v.push(x);
    auto h = BinaryHeap<int32_t, ::rusty::alloc::Global>::from(std::move(v));
    assert(h.len() == 8);
    assert(h.peek().unwrap() == 9);
}

static void test_into_sorted_vec_ascending() {
    auto h = make_heap({3, 1, 4, 1, 5, 9, 2, 6});
    auto sorted = std::move(h).into_sorted_vec();
    assert(sorted.len() == 8);
    for (size_t i = 1; i < sorted.len(); ++i) {
        assert(sorted[i - 1] <= sorted[i]);
    }
    assert(sorted[0] == 1);
    assert(sorted[sorted.len() - 1] == 9);
}

static void test_drain_sorted_descending() {
    auto h = make_heap({3, 1, 4, 1, 5, 9, 2});
    auto d = h.drain_sorted();
    int prev = INT32_MAX;
    int count = 0;
    while (true) {
        auto next = d.next();
        if (next.is_none()) break;
        int val = next.unwrap();
        assert(val <= prev);
        prev = val;
        ++count;
    }
    assert(count == 7);
    assert(h.is_empty());
}

// D4 fixed: RebuildOnDrop field was `Vec<T,A>&` but the dtor body
// calls heap.rebuild_tail() — a BinaryHeap method. Field + ctor
// patched to `BinaryHeap<T,A>&`.
static void test_append_merges_heaps() {
    auto a = make_heap({1, 5, 3});
    auto b = make_heap({10, 2, 7});
    a.append(b);
    assert(a.len() == 6);
    assert(b.is_empty());
    assert(a.peek().unwrap() == 10);
}

static void test_retain_filters_in_place() {
    auto h = make_heap({1, 2, 3, 4, 5, 6, 7, 8, 9, 10});
    h.retain([](const int& x) { return x % 2 == 0; });
    assert(h.len() == 5);
    assert(h.peek().unwrap() == 10);
}

static void run(const char* name, void (*fn)()) {
    std::printf("  %s ... ", name);
    std::fflush(stdout);
    fn();
    std::printf("ok\n");
}

int main() {
    std::printf("binary_heap_port (advanced) tests:\n");
    run("with_capacity_in preallocates",  test_with_capacity_in_preallocates);
    run("drain yields all",               test_drain_yields_all_elements);

    run("into_vec consumes",              test_into_vec_consumes);
    run("from(Vec) bulk-builds heap",     test_from_vec_bulk_builds);
    run("into_sorted_vec ascending",      test_into_sorted_vec_ascending);
    run("drain_sorted descending",        test_drain_sorted_descending);
    run("append merges heaps",            test_append_merges_heaps);
    run("retain filters in place",        test_retain_filters_in_place);

    std::printf("binary_heap_port: advanced tests passed\n");
    return 0;
}
