// Driver for rustc tests/vec.rs — REAL translation, 151 tests, run against
// the consolidated `alloc` module's vec::Vec.
//
// Deliberately does NOT `import vec_tests_port;`. The test bodies register
// themselves through static initializers in that module's object, so the
// driver only needs the runner. Importing it here made clang 22.1.8 SEGFAULT
// in ASTDeclReader::VisitFunctionDecl while loading the suite/alloc BMIs;
// not reading those BMIs from this TU sidesteps the bug entirely. The suite
// library is linked WHOLE_ARCHIVE (see CMakeLists.txt) so the registrations
// are not dropped as unreferenced. Same shape as the btree driver.
#include <rusty/test_runner.hpp>
int main() { return ::rusty_test_runner::run_all(); }
