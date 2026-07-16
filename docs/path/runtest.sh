#!/usr/bin/env bash
# Build the std::path port module, compile/link/run the runtime test. The
# runtime-validation counterpart to build.sh's compile-only precompile: it
# instantiates Path/PathBuf/Components with concrete data and asserts correct
# behavior at run time.
#
# Usage: runtest.sh <work_dir>
set -uo pipefail
W="${1:?usage: runtest.sh <work_dir>}"
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
bash "$REPO/docs/path/build.sh" "$W" | tail -2
CPPM="$W/path.cppm"
[[ -f "$W/path.pcm" ]] || { echo "no BMI — build failed"; exit 1; }
FLAGS="-std=c++23 -DRUSTY_PORTABLE_INTRINSICS=1 -march=native -I$REPO/include"
clang++ $FLAGS -c "$W/path.pcm" -o "$W/path.o" 2>"$W/mo.err" \
  || { echo "module obj FAIL"; tail -5 "$W/mo.err"; exit 1; }
clang++ $FLAGS -fmodule-file=pathmod="$W/path.pcm" -c "$REPO/docs/path/test_path.cpp" \
  -o "$W/test.o" 2>"$W/tc.err" \
  || { echo "test compile: $(grep -c error: "$W/tc.err") errors"; grep error: "$W/tc.err" | head; exit 1; }
clang++ $FLAGS -o "$W/test_bin" "$W/test.o" "$W/path.o" 2>"$W/tl.err" \
  || { echo "link FAIL"; grep -iE "error|undefined" "$W/tl.err" | head; exit 1; }
"$W/test_bin" && echo "=== path RUNTIME PASS ==="
