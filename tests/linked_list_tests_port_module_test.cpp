// Driver for the transpiled rustc linked_list.rs test file.
// 1 #[test] (test_hash) — stubbed as skip until rusty-cpp has a hash builder.

import linked_list_tests_port;

#include <rusty/test_runner.hpp>

int main() {
    return ::rusty_test_runner::run_all();
}
