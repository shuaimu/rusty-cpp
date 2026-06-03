// End-to-end smoke test for cell_port (rustc library/core/src/cell.rs).
//
// Exercises Cell<T> (interior mutability via copy semantics) and
// RefCell<T> (runtime-checked aliasing) — the two public types of
// cell.rs — plus the BorrowError / BorrowMutError reachability that
// the original Phase B test covered.

#include <rusty/panic.hpp>

import cell_port;

#include <cassert>
#include <iostream>
#include <sstream>
#include <string>

using rusty::cell::Cell;
using rusty::cell::RefCell;

static void test_cell_new_get_set() {
    auto c = Cell<int>::new_(42);
    assert(c.get() == 42);
    c.set(7);
    assert(c.get() == 7);
}

static void test_cell_replace_returns_old() {
    auto c = Cell<int>::new_(100);
    int old = c.replace(200);
    assert(old == 100);
    assert(c.get() == 200);
}

static void test_cell_swap_exchanges() {
    auto a = Cell<int>::new_(1);
    auto b = Cell<int>::new_(2);
    a.swap(b);
    assert(a.get() == 2);
    assert(b.get() == 1);
}

static void test_cell_into_inner() {
    auto c = Cell<int>::new_(99);
    int v = std::move(c).into_inner();
    assert(v == 99);
}

static void test_refcell_borrow_reads() {
    auto rc = RefCell<int>::new_(50);
    auto guard = rc.borrow();
    // Transpiled Ref<T> doesn't surface operator*; reach through the
    // NonNull<T> field directly.
    assert(*guard.value.as_ptr() == 50);
}

static void test_refcell_borrow_mut_writes() {
    auto rc = RefCell<int>::new_(10);
    {
        auto guard = rc.borrow_mut();
        *guard.value.as_ptr() = 25;
    }
    assert(*rc.borrow().value.as_ptr() == 25);
}

static void test_borrow_error_formattable() {
    rusty::cell::BorrowError be{.location = rusty::panic::Location::caller()};
    std::ostringstream os;
    os << be;
    assert(os.str().find("BorrowError") != std::string::npos);

    rusty::cell::BorrowMutError bme{.location = rusty::panic::Location::caller()};
    std::ostringstream os2;
    os2 << bme;
    assert(os2.str().find("BorrowMutError") != std::string::npos);
}

static void run(const char* name, void (*fn)()) {
    std::printf("  %s ... ", name);
    std::fflush(stdout);
    fn();
    std::printf("ok\n");
}

int main() {
    std::printf("cell_port (transpiled) tests:\n");
    run("Cell::new_ + get + set",        test_cell_new_get_set);
    run("Cell::replace returns old",     test_cell_replace_returns_old);
    run("Cell::swap exchanges values",   test_cell_swap_exchanges);
    run("Cell::into_inner",              test_cell_into_inner);
    run("RefCell::borrow reads",         test_refcell_borrow_reads);
    run("RefCell::borrow_mut writes",    test_refcell_borrow_mut_writes);
    run("BorrowError formattable",       test_borrow_error_formattable);
    std::printf("cell_port: all tests passed\n");
    return 0;
}
