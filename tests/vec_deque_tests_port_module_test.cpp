// Driver for rustc tests/vec_deque.rs — REAL translation, 104 tests, run
// against the consolidated `alloc` module's collections::vec_deque::VecDeque.
// See vec_tests_port_module_test.cpp for why this does not import the suite
// module (clang 22 BMI-deserialization crash) and why the library is linked
// WHOLE_ARCHIVE.
#include <rusty/test_runner.hpp>
int main() { return ::rusty_test_runner::run_all(); }
