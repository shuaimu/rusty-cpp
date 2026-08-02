// Minimal reproduction for #183. NOT wired into CMake on purpose — it CRASHES
// the compiler, so adding it to TEST_SOURCES would break `ninja` for everyone.
//
// Build it by hand (from .rusty-modules-cache/build), reusing the hashset
// test's modmap so the module-file set is guaranteed consistent:
//
//   cd .rusty-modules-cache/build
//   clang++ -I ../../include -std=c++23 -Wall -Wextra -Wpedantic \
//     -O3 -DNDEBUG -march=native \
//     @CMakeFiles/hashset_set_algebra_test.out.dir/tests/hashset_set_algebra_test.cpp.o.modmap \
//     -c ../../docs/umbrella_repro/umbrella_smoke_repro.cpp -o /tmp/smoke.o
//
// (-march=native is REQUIRED: the BMIs are built with it, and omitting it gives
// spurious "compiled with target feature '+64bit'" errors that look like a
// different bug.)
//
// EXPECTED, as of 2026-08-01 at HEAD:
//   clang++: error: clang frontend command failed with exit code 139  (SIGSEGV)
//   ...in ASTContext::getInlineVariableDefinitionKind, reached from
//      CodeGenModule::EmitGlobal -> DeclMustBeEmitted -> GetGVALinkageForVariable
//   i.e. codegen of a module-owned global VARIABLE, AFTER the import completes
//   (the crash reports `current parser token 'int'`, meaning it got to main()).
//
// WITH A PRE-#187 std_port it HANGS instead of crashing (>300s, no output).
// Both are broken; the fix changed which clang bug is hit first.
//
// This file exists because the full hashset TU takes >40 MINUTES to fail, which
// makes bisecting impossible. This one fails in under a second.
import rusty;

int main() { return 0; }
