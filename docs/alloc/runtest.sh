#!/usr/bin/env bash
# Build the consolidated alloc module + compile/link/run the runtime test.
# This is the runtime-validation counterpart to build.sh's compile-only check:
# it INSTANTIATES Vec/VecDeque with concrete types (which the BMI precompile
# skips) and asserts correct behavior at run time.
#
# Usage: runtest.sh <work_dir>
set -uo pipefail
W="${1:?usage: runtest.sh <work_dir>}"
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
bash "$REPO/docs/alloc/build.sh" "$W" | tail -2
CPPM="$W/out/alloc.cppm"
[[ -f "$W/out/alloc.pcm" ]] || { echo "no BMI — build failed"; exit 1; }
FLAGS="-std=c++23 -DRUSTY_PORTABLE_INTRINSICS=1 -march=native -I$REPO/include"
[[ "${RUSTY_ALLOC_WITH_BOXED:-1}" == "1" ]] && FLAGS="$FLAGS -DALLOC_WITH_BOXED"
[[ "${RUSTY_ALLOC_WITH_STRING:-1}" == "1" ]] && FLAGS="$FLAGS -DALLOC_WITH_STRING"
clang++ $FLAGS -c "$W/out/alloc.pcm" -o "$W/alloc.o" 2>"$W/mo.err" || { echo "module obj FAIL"; tail -3 "$W/mo.err"; exit 1; }
clang++ $FLAGS -fmodule-file=alloc="$W/out/alloc.pcm" -c "$REPO/docs/alloc/test_alloc.cpp" -o "$W/test.o" 2>"$W/tc.err" \
  || { echo "test compile: $(grep -c error: "$W/tc.err") errors"; grep error: "$W/tc.err" | head; exit 1; }
clang++ $FLAGS -o "$W/test_bin" "$W/test.o" "$W/alloc.o" 2>"$W/tl.err" \
  || { echo "link FAIL"; grep -iE "error|undefined" "$W/tl.err" | head; exit 1; }
"$W/test_bin" && echo "=== alloc RUNTIME PASS ==="
