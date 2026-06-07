// Driver for rustc library/alloc/src/collections/btree/{map,set}/tests.rs.
//
// Stubbed test bodies live in transpiled/btree_tests_port/*.cppm
// (auto-generated from the rust source). Hand-translated bodies that
// actually exercise btree_port live alongside this file in
// btree_tests_port_unstubbed.cpp — see that file for why a separate
// TU is needed (in-module-purview instantiation bug).
import btree_tests_port;
#include <rusty/test_runner.hpp>
int main() { return ::rusty_test_runner::run_all(); }
