// Smoke test for cell_port — exercises the transpiled C++20 module
// for the Cell / RefCell port (library/core/src/cell.rs).
//
// Phase B/C level: just confirm that the module imports successfully
// and that BorrowError / BorrowMutError are reachable. The full
// RefCell API requires deeper work (cell.hpp's Cell<T> doesn't
// surface the constructors the transpiled body needs).

#include <rusty/panic.hpp>  // rusty::panic::Location for BorrowError init

import cell_port;

#include <cassert>
#include <iostream>
#include <sstream>

int main() {
    // Default-construct via designated init. BorrowError is empty
    // except for the Location reference, which the module exposes
    // through the global rusty::panic::Location::caller() singleton.
    cell_port::BorrowError be{.location = rusty::panic::Location::caller()};
    cell_port::BorrowMutError bme{.location = rusty::panic::Location::caller()};

    // We can format via operator<< (the transpiled module defines one).
    std::ostringstream os;
    os << be;
    assert(os.str().find("BorrowError") != std::string::npos);

    std::ostringstream os2;
    os2 << bme;
    assert(os2.str().find("BorrowMutError") != std::string::npos);

    std::cout << "cell_port smoke test OK: "
              << "BorrowError + BorrowMutError reachable + formattable\n";
    return 0;
}
