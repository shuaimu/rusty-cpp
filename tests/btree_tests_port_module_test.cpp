// Driver for rustc library/alloc/src/collections/btree/{map,set}/tests.rs.
//
// All test bodies are REAL translations living in
// btree_tests_port_unstubbed.cpp (separate TU — see that file for why:
// in-module-purview BTreeMap instantiation bug). The legacy SKIP-stub
// module (transpiled/btree_tests_port) was retired 2026-07 once the
// translated suite went green with live asserts; the stub-only test
// names not yet translated (CursorMut API ×12, extract_if height
// matrix, retain/extend_ref/clone_from/recovery/leak-chaos) are
// tracked in docs/btree_tests_port/STATUS.md.
#include <rusty/test_runner.hpp>
int main() { return ::rusty_test_runner::run_all(); }
