// Thorough end-to-end exercise of every public BinaryHeap method that
// the other binary_heap_port test files don't already cover. Each test
// is single-API-focused so a compile or assert failure pinpoints the
// site.
//
// Already covered elsewhere (do NOT repeat here):
//   - push / pop / peek / iter / into_iter_sorted        (push/pop/iter tests)
//   - as_slice / len / is_empty / capacity / clear       (comprehensive)
//   - with_capacity_in / drain / drain_sorted / into_vec /
//     from(Vec) / into_sorted_vec / append / retain      (advanced)
//
// New surface exercised here:
//   - constructors: new_, default_, with_capacity, from(array), from_iter, from_raw_vec
//   - inspection: peek_mut, allocator
//   - mutation:   pop_if, extend, extend_one
//   - consume:    into_iter (unsorted)
//   - capacity:   reserve, reserve_exact, try_reserve, try_reserve_exact,
//                 shrink_to_fit, shrink_to
//   - misc:       clone, clone_from

import binary_heap_port;
import vec_port.vec;

#include <rusty/alloc.hpp>
#include <cassert>
#include <cstdint>
#include <cstdio>
#include <vector>

using rusty::collections::BinaryHeap;

using HeapI32 = BinaryHeap<int32_t, ::rusty::alloc::Global>;

static HeapI32 make_heap(std::initializer_list<int> values) {
    auto h = HeapI32::new_in(::rusty::alloc::Global{});
    for (int v : values) h.push(v);
    return h;
}

// ============================================================================
// Constructors
// ============================================================================

static void test_new_creates_empty_heap() {
    auto h = BinaryHeap<int32_t>::new_();
    assert(h.is_empty());
    assert(h.len() == 0);
    assert(h.capacity() == 0);
}

static void test_default_is_empty() {
    auto h = BinaryHeap<int32_t>::default_();
    assert(h.is_empty());
    assert(h.len() == 0);
}

static void test_with_capacity_preallocates() {
    auto h = BinaryHeap<int32_t>::with_capacity(32);
    assert(h.is_empty());
    assert(h.capacity() >= 32);
    for (int i = 0; i < 32; ++i) h.push(i);
    assert(h.len() == 32);
    assert(h.peek().unwrap() == 31);
}

static void test_from_array_bulk_builds() {
    std::array<int32_t, 5> arr = {3, 1, 4, 1, 5};
    auto h = HeapI32::template from<5>(arr);
    assert(h.len() == 5);
    assert(h.peek().unwrap() == 5);
}

static void test_from_iter_bulk_builds() {
    auto src = HeapI32::new_in(::rusty::alloc::Global{});
    for (int v : {2, 7, 1, 8, 2, 8}) src.push(v);
    // Build a fresh heap from src's iterator.
    auto h = BinaryHeap<int32_t>::from_iter(src.drain());
    assert(h.len() == 6);
    assert(h.peek().unwrap() == 8);
    assert(src.is_empty());  // drain consumed src
}

static void test_from_raw_vec_bulk_builds() {
    // from_raw_vec wraps a Vec as the heap's storage WITHOUT
    // re-heapifying — caller responsibility to ensure heap shape.
    // We push then take it back out via into_vec, wrap with from_raw_vec.
    auto h = make_heap({9, 5, 7, 1});
    auto v = std::move(h).into_vec();
    auto h2 = HeapI32::from_raw_vec(std::move(v));
    assert(h2.len() == 4);
    // The values are present; peek may or may not be the max depending
    // on heap shape — into_vec doesn't re-heapify either. Just check
    // membership.
    auto sorted = std::move(h2).into_sorted_vec();
    assert(sorted.len() == 4);
    assert(sorted[0] == 1);
    assert(sorted[3] == 9);
}

// ============================================================================
// Inspection
// ============================================================================

static void test_peek_mut_returns_top() {
    auto h = make_heap({3, 1, 4, 1, 5, 9, 2, 6});
    auto guard = h.peek_mut();
    assert(guard.is_some());
    // PeekMut acts as a guard around the top element. We just confirm
    // is_some — full mutation semantics (changing the top, observing
    // sift-down on drop) are harder to spell portably here.
}

static void test_peek_mut_on_empty_is_none() {
    auto h = HeapI32::new_in(::rusty::alloc::Global{});
    auto guard = h.peek_mut();
    assert(guard.is_none());
}

static void test_allocator_returns_global() {
    auto h = make_heap({1});
    [[maybe_unused]] const auto& alloc = h.allocator();
    // Just confirms the accessor compiles and returns *something*; the
    // Global allocator is stateless so there's no observable property.
}

// ============================================================================
// Mutation
// ============================================================================

static void test_pop_if_pops_when_predicate_true() {
    auto h = make_heap({3, 1, 4, 1, 5, 9, 2});
    auto popped = h.pop_if([](const int& x) { return x > 5; });
    assert(popped.is_some());
    assert(popped.unwrap() == 9);
    assert(h.len() == 6);
}

static void test_pop_if_skips_when_predicate_false() {
    auto h = make_heap({3, 1, 4, 1, 5, 9, 2});
    auto popped = h.pop_if([](const int& x) { return x > 100; });
    assert(popped.is_none());
    assert(h.len() == 7);  // unchanged
    assert(h.peek().unwrap() == 9);  // top still present
}

static void test_pop_if_on_empty_is_none() {
    auto h = HeapI32::new_in(::rusty::alloc::Global{});
    auto popped = h.pop_if([](const int&) { return true; });
    assert(popped.is_none());
}

static void test_extend_with_iter_appends() {
    auto h = make_heap({1, 2, 3});
    auto src = make_heap({10, 20, 30});
    h.extend(src.drain());
    assert(h.len() == 6);
    assert(h.peek().unwrap() == 30);
    // Extended elements present
    auto sorted = std::move(h).into_sorted_vec();
    assert(sorted[0] == 1);
    assert(sorted[5] == 30);
}

static void test_extend_one_pushes_single() {
    auto h = make_heap({1, 2, 3});
    h.extend_one(99);
    assert(h.len() == 4);
    assert(h.peek().unwrap() == 99);
}

// ============================================================================
// Consume
// ============================================================================

static void test_into_iter_unsorted_yields_all() {
    auto h = make_heap({3, 1, 4, 1, 5, 9, 2});
    auto it = std::move(h).into_iter();
    int count = 0;
    int sum = 0;
    while (true) {
        auto next = it.next();
        if (next.is_none()) break;
        sum += next.unwrap();
        ++count;
    }
    assert(count == 7);
    assert(sum == 3 + 1 + 4 + 1 + 5 + 9 + 2);
}

// ============================================================================
// Capacity management
// ============================================================================

static void test_reserve_grows_capacity() {
    auto h = HeapI32::new_in(::rusty::alloc::Global{});
    h.push(1);  // some initial allocation
    const auto before = h.capacity();
    h.reserve(100);
    assert(h.capacity() >= before + 100);
}

static void test_reserve_exact_grows_capacity() {
    auto h = HeapI32::new_in(::rusty::alloc::Global{});
    h.push(1);
    const auto before = h.capacity();
    h.reserve_exact(64);
    assert(h.capacity() >= before + 64);
}

static void test_try_reserve_succeeds_for_modest_size() {
    auto h = HeapI32::new_in(::rusty::alloc::Global{});
    h.push(1);
    auto result = h.try_reserve(64);
    assert(result.is_ok());
    assert(h.capacity() >= 65);
}

static void test_try_reserve_exact_succeeds_for_modest_size() {
    auto h = HeapI32::new_in(::rusty::alloc::Global{});
    h.push(1);
    auto result = h.try_reserve_exact(64);
    assert(result.is_ok());
    assert(h.capacity() >= 65);
}

static void test_shrink_to_fit_reduces_capacity() {
    auto h = BinaryHeap<int32_t>::with_capacity(1024);
    h.push(1);
    h.push(2);
    assert(h.capacity() >= 1024);
    h.shrink_to_fit();
    // After shrink_to_fit, capacity should be at least len() but much
    // smaller than 1024. Exact value is allocator-dependent.
    assert(h.capacity() < 1024);
    assert(h.capacity() >= h.len());
    // Heap still works
    assert(h.peek().unwrap() == 2);
}

static void test_shrink_to_caps_at_target() {
    auto h = BinaryHeap<int32_t>::with_capacity(1024);
    h.push(1);
    h.push(2);
    h.shrink_to(8);
    // capacity is now max(len, 8)-ish; spec: at least max(len, min_capacity).
    assert(h.capacity() >= 2);
    assert(h.capacity() < 1024);
    assert(h.peek().unwrap() == 2);
}

// ============================================================================
// Clone family
// ============================================================================

static void test_clone_produces_independent_copy() {
    auto h = make_heap({3, 1, 4, 1, 5});
    auto h2 = h.clone();
    assert(h2.len() == 5);
    assert(h2.peek().unwrap() == 5);
    // Mutating clone doesn't affect original.
    h2.push(99);
    assert(h2.peek().unwrap() == 99);
    assert(h.peek().unwrap() == 5);
}

static void test_clone_from_assigns_from_source() {
    auto src = make_heap({10, 20, 30});
    auto dst = make_heap({1, 2});
    dst.clone_from(src);
    assert(dst.len() == 3);
    assert(dst.peek().unwrap() == 30);
    // src still intact
    assert(src.len() == 3);
    assert(src.peek().unwrap() == 30);
}

static void run(const char* name, void (*fn)()) {
    std::printf("  %s ... ", name);
    std::fflush(stdout);
    fn();
    std::printf("ok\n");
}

int main() {
    std::printf("binary_heap_port (full-API) tests:\n");

    // Constructors
    run("new_ creates empty",              test_new_creates_empty_heap);
    run("default_ is empty",               test_default_is_empty);
    run("with_capacity preallocates",      test_with_capacity_preallocates);
    run("from(array) bulk-builds",         test_from_array_bulk_builds);
    run("from_iter bulk-builds",           test_from_iter_bulk_builds);
    run("from_raw_vec wraps",              test_from_raw_vec_bulk_builds);

    // Inspection
    run("peek_mut returns top",            test_peek_mut_returns_top);
    run("peek_mut on empty is None",       test_peek_mut_on_empty_is_none);
    run("allocator returns global",        test_allocator_returns_global);

    // Mutation
    run("pop_if pops when true",           test_pop_if_pops_when_predicate_true);
    run("pop_if skips when false",         test_pop_if_skips_when_predicate_false);
    run("pop_if on empty is None",         test_pop_if_on_empty_is_none);
    run("extend appends iterator",         test_extend_with_iter_appends);
    run("extend_one pushes single",        test_extend_one_pushes_single);

    // Consume
    run("into_iter (unsorted) yields all", test_into_iter_unsorted_yields_all);

    // Capacity
    run("reserve grows capacity",          test_reserve_grows_capacity);
    run("reserve_exact grows capacity",    test_reserve_exact_grows_capacity);
    run("try_reserve succeeds",            test_try_reserve_succeeds_for_modest_size);
    run("try_reserve_exact succeeds",      test_try_reserve_exact_succeeds_for_modest_size);
    run("shrink_to_fit reduces",           test_shrink_to_fit_reduces_capacity);
    run("shrink_to caps at target",        test_shrink_to_caps_at_target);

    // Clone
    run("clone is independent",            test_clone_produces_independent_copy);
    run("clone_from assigns",              test_clone_from_assigns_from_source);

    std::printf("binary_heap_port: full-API tests passed\n");
    return 0;
}
