#!/usr/bin/env bash
# Transpile std's path.rs as ONE C++ module, patch in the Unix OsStr runtime,
# and precompile it. path.rs is a single file, so no cargo-expand is needed
# (syn-based transpile tolerates the absent std deps; the runtime + prep supply
# what Unix path manipulation actually uses).
#
# Usage: build.sh <work_dir>
set -uo pipefail
W="${1:?usage: build.sh <work_dir>}"
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SYS="$(rustc --print sysroot)/lib/rustlib/src/rust/library"
rm -rf "$W"; mkdir -p "$W"
cp "$SYS/std/src/path.rs" "$W/path.rs"
bash "$REPO/docs/path/prep.sh" "$W/path.rs" >/dev/null
TRANSPILER="${RUSTY_CPP_TRANSPILER_BIN:-$REPO/target/release/rusty-cpp-transpiler}"
"$TRANSPILER" "$W/path.rs" -m pathmod -o "$W/path.cppm" > "$W/transpile.log" 2>&1
echo "transpile exit=$? ($(tail -1 "$W/transpile.log"))"
CPPM="$W/path.cppm"
[[ -f "$CPPM" ]] || { echo "no output — see $W/transpile.log"; exit 1; }
# strip umbrella/self imports
sed -i '/^import rusty;$/d; /^import [a-z_]*_port\./d; /^import pathmod;$/d' "$CPPM"
# swap the preamble's `using OsStr = std::string` aliases for the real Unix
# OsStr/OsString runtime (a byte-buffer type with the &[u8] parsing API).
sed -i '/^using OsStr = std::string;$/d; /^using OsString = std::string;$/d' "$CPPM"
sed -i 's|#include <rusty/try.hpp>|#include <rusty/try.hpp>\n#include <rusty/os_str.hpp>|' "$CPPM"
# Global-fragment namespace aliases so the transpiler's `::ffi::` / `::sys::`
# crate-path references resolve to the rusty runtime namespaces.
sed -i 's|#include <rusty/os_str.hpp>|#include <rusty/os_str.hpp>\nnamespace ffi = rusty::ffi;\nnamespace sys = rusty::sys;|' "$CPPM"
[[ -f "$REPO/docs/path/post_transpile_patch.py" ]] && \
  python3 "$REPO/docs/path/post_transpile_patch.py" "$CPPM" >/dev/null 2>&1
FLAGS="-std=c++23 -DRUSTY_PORTABLE_INTRINSICS=1 -march=native -I$REPO/include -x c++-module"
clang++ $FLAGS --precompile -o "$W/path.pcm" "$CPPM" -ferror-limit=0 2> "$W/compile.err"
echo "compile: $(grep -c 'error:' "$W/compile.err") errors"
grep -hoE "error: .*" "$W/compile.err" | sed -E "s/'[^']*'/'X'/g; s/[0-9]+/N/g" | sort | uniq -c | sort -rn | head -15
