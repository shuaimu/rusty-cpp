// End-to-end smoke test for the transpiled arc_port
// (rustc library/alloc/src/sync.rs → arc_port::Arc / arc_port::Weak).
//
// Replaces the legacy bridge-stub test that exercised the
// hand-written `rusty::Arc<T>::make()` API.
import arc_port;

#include <rusty/rusty.hpp>
#include <cassert>
#include <cstdio>

// Surface the transpiled types via their deep path. The user-facing
// `rusty::sync::Arc/Weak` alias is deferred until the hand-written
// `rusty::Arc` in `include/rusty/arc.hpp` retires in favor of arc_port.
using rusty::port::sync::Arc;
using rusty::port::sync::Weak;

static void test_new_and_strong_count() {
    auto p = Arc<int>::new_(42);
    assert(Arc<int>::strong_count(p) == 1);
    assert(Arc<int>::weak_count(p) == 0);
}

static void test_clone_increments_refcount() {
    auto p = Arc<int>::new_(7);
    assert(Arc<int>::strong_count(p) == 1);
    {
        auto p2 = p.clone();
        assert(Arc<int>::strong_count(p) == 2);
        assert(Arc<int>::strong_count(p2) == 2);
    }
    assert(Arc<int>::strong_count(p) == 1);
}

static void test_multiple_clones() {
    auto p = Arc<int>::new_(99);
    auto p2 = p.clone();
    auto p3 = p.clone();
    auto p4 = p2.clone();
    assert(Arc<int>::strong_count(p) == 4);
}

static void test_move_does_not_change_refcount() {
    auto p = Arc<int>::new_(123);
    assert(Arc<int>::strong_count(p) == 1);
    auto moved = std::move(p);
    assert(Arc<int>::strong_count(moved) == 1);
}

static void test_downgrade_increments_weak_count() {
    auto p = Arc<int>::new_(55);
    assert(Arc<int>::weak_count(p) == 0);
    auto w = Arc<int>::downgrade(p);
    assert(Arc<int>::strong_count(p) == 1);
    assert(Arc<int>::weak_count(p) == 1);
    auto w2 = Arc<int>::downgrade(p);
    assert(Arc<int>::weak_count(p) == 2);
}

static void test_weak_clone() {
    auto p = Arc<int>::new_(88);
    auto w = Arc<int>::downgrade(p);
    auto w2 = w.clone();
    assert(Arc<int>::weak_count(p) == 2);
}

struct Point { int x; int y; int sum() const { return x + y; } };

static void test_make_variadic() {
    // patcher-injected ergonomic shim: variadic construct.
    auto p = Arc<Point>::make(3, 4);
    assert(p->x == 3);
    assert(p->y == 4);
    assert(p->sum() == 7);
}

static void test_operator_arrow_and_star() {
    // patcher-injected `operator->` (and existing `operator*`).
    auto p = Arc<Point>::make(10, 20);
    assert((*p).x == 10);
    assert(p->y == 20);
}

static void run(const char* name, void (*fn)()) {
    std::printf("  %s ... ", name);
    std::fflush(stdout);
    fn();
    std::printf("ok\n");
}

int main() {
    std::printf("arc_port (transpiled) tests:\n");
    run("new_ + strong_count",         test_new_and_strong_count);
    run("clone increments refcount",   test_clone_increments_refcount);
    run("multiple clones",             test_multiple_clones);
    run("move keeps refcount",         test_move_does_not_change_refcount);
    run("downgrade -> weak_count",     test_downgrade_increments_weak_count);
    run("Weak::clone",                 test_weak_clone);
    run("make(args...) variadic",      test_make_variadic);
    run("operator-> and operator*",    test_operator_arrow_and_star);
    std::printf("arc_port: all tests passed\n");
    return 0;
}
