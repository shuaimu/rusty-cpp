// Driver for rustc library/alloc/src/collections/btree/{map,set}/tests.rs.
//
// Currently every TEST_CASE registers a skip — the transpiled module
// depends on `crate::testing::{crash_test, ord_chaos}` helpers and on
// internal BTreeMap invariant-check methods that we don't yet ship.
// See docs/btree_tests_port/post_transpile_patch.py for the un-stub
// pipeline. As prereqs land, un-stub tests one cluster at a time.
import btree_tests_port;
#include <rusty/test_runner.hpp>
int main() { return ::rusty_test_runner::run_all(); }
