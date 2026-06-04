// Driver for the transpiled rustc binary_heap.rs test file. Imports the
// `binary_heap_tests_port` module (which contains 34 TEST_CASE blocks
// auto-registered at static-init time) and runs them via the
// rusty::test_runner harness.
//
// 4 of the 34 tests are stubbed-out skips (3 need CrashTestDummy from the
// testing_port helper crate; 1 needs the `rand` crate). The remaining 30
// exercise the transpiled rustc BinaryHeap directly.

import binary_heap_tests_port;

#include <rusty/test_runner.hpp>

int main() {
    return ::rusty_test_runner::run_all();
}
