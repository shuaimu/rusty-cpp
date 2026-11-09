// End-to-end tests for the transpiled rc_port (rusty::Rc /
// rusty::Weak alias to ::rc_port::Rc / ::rc_port::Weak from
// library/alloc/src/rc.rs).
//
// Covers the API surface that comes through the transpiled body
// without hand-fixing additional code paths: new_, clone,
// strong_count, weak_count, downgrade, upgrade, move semantics,
// destructor refcount cleanup.
//
// Replaces the legacy tests/rusty_rc_test.cpp (which tested the
// hand-written `rusty::Rc<T>` API that we retired).
import rc_port;

#include <rusty/rusty.hpp>
// CMake builds the rc_port tests with -DNDEBUG; the checks below are asserts.
#undef NDEBUG
#include <cassert>
#include <cstdio>

using rusty::rc::Rc;
using rusty::rc::Weak;

static void test_new_and_strong_count() {
    auto p = Rc<int>::new_(42);
    assert(Rc<int>::strong_count(p) == 1);
    assert(Rc<int>::weak_count(p) == 0);
}

static void test_clone_increments_refcount() {
    auto p = Rc<int>::new_(7);
    assert(Rc<int>::strong_count(p) == 1);
    {
        auto p2 = p.clone();
        assert(Rc<int>::strong_count(p) == 2);
        assert(Rc<int>::strong_count(p2) == 2);
    }
    // p2 destructor ran — refcount back to 1
    assert(Rc<int>::strong_count(p) == 1);
}

static void test_multiple_clones() {
    auto p = Rc<int>::new_(99);
    auto p2 = p.clone();
    auto p3 = p.clone();
    auto p4 = p2.clone();
    assert(Rc<int>::strong_count(p) == 4);
}

static void test_move_does_not_change_refcount() {
    auto p = Rc<int>::new_(123);
    assert(Rc<int>::strong_count(p) == 1);
    auto moved = std::move(p);
    assert(Rc<int>::strong_count(moved) == 1);
}

static void test_downgrade_increments_weak_count() {
    auto p = Rc<int>::new_(55);
    assert(Rc<int>::weak_count(p) == 0);
    auto w = Rc<int>::downgrade(p);
    assert(Rc<int>::strong_count(p) == 1);
    assert(Rc<int>::weak_count(p) == 1);
    // Second downgrade.
    auto w2 = Rc<int>::downgrade(p);
    assert(Rc<int>::weak_count(p) == 2);
}

static void test_weak_clone() {
    auto p = Rc<int>::new_(88);
    auto w = Rc<int>::downgrade(p);
    auto w2 = w.clone();
    assert(Rc<int>::weak_count(p) == 2);
}

// A payload that cannot move (deleted move operations, as the transpiler
// lowers a `PhantomPinned` type such as SRPC's `Reactor`) and records its
// destruction. `Rc::make_with` builds it from a factory's prvalue inside the
// allocation; `Rc::make` forwards constructor arguments.
static int pinned_alive = 0;
struct Pinned {
    int value;
    explicit Pinned(int v) : value(v) { ++pinned_alive; }
    Pinned(const Pinned&) = delete;
    Pinned(Pinned&&) = delete;
    Pinned& operator=(const Pinned&) = delete;
    Pinned& operator=(Pinned&&) = delete;
    ~Pinned() { --pinned_alive; }
    static Pinned new_(int v) { return Pinned(v); }
};

static void test_make_with_builds_a_non_movable_payload_in_place() {
    {
        auto p = Rc<Pinned>::make_with([&] { return Pinned::new_(31); });
        assert(p->value == 31);
        assert(pinned_alive == 1);
        auto p2 = p.clone();
        assert(Rc<Pinned>::strong_count(p) == 2);
        auto w = Rc<Pinned>::downgrade(p2);
        assert(Rc<Pinned>::weak_count(p) == 1);
    }
    // Both strong handles and the weak one are gone: dropped and freed once.
    assert(pinned_alive == 0);
}

static void test_make_forwards_constructor_arguments() {
    {
        auto p = Rc<Pinned>::make(47);
        assert(p->value == 47);
        assert(Rc<Pinned>::strong_count(p) == 1);
        assert(pinned_alive == 1);
    }
    assert(pinned_alive == 0);
}

static void run(const char* name, void (*fn)()) {
    std::printf("  %s ... ", name);
    std::fflush(stdout);
    fn();
    std::printf("ok\n");
}

int main() {
    std::printf("rc_port (transpiled) tests:\n");
    run("new_ + strong_count",         test_new_and_strong_count);
    run("clone increments refcount",   test_clone_increments_refcount);
    run("multiple clones",             test_multiple_clones);
    run("move keeps refcount",         test_move_does_not_change_refcount);
    run("downgrade -> weak_count",     test_downgrade_increments_weak_count);
    run("Weak::clone",                 test_weak_clone);
    run("make_with: non-movable payload", test_make_with_builds_a_non_movable_payload_in_place);
    run("make: forwarded ctor args",   test_make_forwards_constructor_arguments);
    std::printf("rc_port: all tests passed\n");
    return 0;
}
