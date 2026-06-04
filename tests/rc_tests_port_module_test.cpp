// Driver for rustc tests/rc.rs.
import rc_tests_port;
#include <rusty/test_runner.hpp>
int main() { return ::rusty_test_runner::run_all(); }
