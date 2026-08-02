// Regression guard for #183: `import rusty;` must compile, and compile FAST.
//
// NOT wired into CMake — it exists so the umbrella can be smoke-tested by hand
// without paying for a full test TU. Build it from .rusty-modules-cache/build,
// reusing the hashset test's modmap so the module-file set is consistent:
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
// EXPECTED: exit 0, no diagnostics, well under a second.
//
// ── WHAT THIS CAUGHT (#183, fixed 2026-08-01) ───────────────────────────────
// This used to SIGSEGV clang 22.1.8 in ASTContext::getInlineVariableDefinitionKind
// (via CodeGenModule::EmitGlobal -> DeclMustBeEmitted), or hang for 40+ minutes
// in ASTReader::PassInterestingDeclsToConsumer — nondeterministically, from the
// same inputs.
//
// The cause was NOT in this file's C++, nor in clang's handling of any construct
// we emit. `rusty.pcm` was built against ONE std_port BMI while every consumer
// resolved `std_port` to a DIFFERENT file:
//
//   rusty's modmap:    std_port = CMakeFiles/std_port@synth_3.dir/<hash>.bmi (39.6 MB)
//   consumers' modmap: std_port = CMakeFiles/std_port.dir/std_port.pcm       (29.9 MB)
//
// Clang had to merge two complete copies of the same module; the reported decl
// (std::dynamic_extent) was collateral, just the first module-owned inline
// variable codegen happened to touch.
//
// CMake forks a module's BMI into CMakeFiles/<target>@synth_N.dir/*.bmi when the
// producing target's compile options differ from its consumer's. std_port and
// std_port_hashbrown were the only module targets whose flags didn't match the
// rest (they lacked -O3 -DNDEBUG and carried a -DRUSTY_PORTABLE_INTRINSICS=1
// that std_port never actually reads). Aligning them in CMakeLists.txt removed
// the second BMI.
//
// THE GENERAL RULE, which this file exists to keep honest: every C++20 module
// target in this project must carry byte-identical compile options. A flag delta
// on any module target silently forks its BMI, and the failure surfaces far away
// as a compiler hang or crash with no reference to the flag that caused it.
import rusty;

int main() { return 0; }
