#!/usr/bin/env bash
# Build the transpiled std port (module `rusty` + its hashbrown dep module)
# and compile/link/RUN the runtime assertion test — instantiates HashMap with
# concrete types (the --precompile skips template bodies) and asserts
# behavior. Runtime-validation counterpart of docs/rusty/build.sh.
#
# Usage: runtest.sh <work_dir>
set -uo pipefail
W="${1:?usage: runtest.sh <work_dir>}"
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
bash "$REPO/docs/rusty/build.sh" "$W" | tail -3
[[ -f "$W/out/rusty.pcm" && -f "$W/out/hashbrown/hashbrown.pcm" ]] || { echo "no BMI — build failed"; exit 1; }
FLAGS="-std=c++23 -DRUSTY_PORTABLE_INTRINSICS=1 -march=native -I$REPO/include"
clang++ $FLAGS -c "$W/out/hashbrown/hashbrown.pcm" -o "$W/hashbrown.o" 2>"$W/ho.err" || { echo "hashbrown obj FAIL"; tail -3 "$W/ho.err"; exit 1; }
clang++ $FLAGS -fmodule-file=hashbrown="$W/out/hashbrown/hashbrown.pcm" -c "$W/out/rusty.pcm" -o "$W/rusty.o" 2>"$W/ro.err" || { echo "rusty obj FAIL"; tail -3 "$W/ro.err"; exit 1; }
clang++ $FLAGS -fmodule-file=rusty="$W/out/rusty.pcm" -fmodule-file=hashbrown="$W/out/hashbrown/hashbrown.pcm" \
  -c "$REPO/docs/rusty/test_rusty.cpp" -o "$W/test.o" 2>"$W/tc.err" \
  || { echo "test compile: $(grep -c error: "$W/tc.err") errors"; grep error: "$W/tc.err" | head; exit 1; }
clang++ $FLAGS -o "$W/test_bin" "$W/test.o" "$W/rusty.o" "$W/hashbrown.o" 2>"$W/tl.err" \
  || { echo "link FAIL"; grep -iE "error|undefined" "$W/tl.err" | head; exit 1; }
"$W/test_bin" && echo "=== rusty (std) RUNTIME PASS ==="
