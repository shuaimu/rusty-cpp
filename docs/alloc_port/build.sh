#!/usr/bin/env bash
# Regenerate + compile the consolidated alloc_port as ONE C++ module.
#
# KEY: use --expand (cargo-expand → one flattened crate module). Per-submodule
# emission creates ILLEGAL C++ module cycles (alloc.vec <-> alloc.collections.
# vec_deque); Rust compiles a crate as ONE unit, so we emit ONE .cppm — no
# inter-module imports, no cycle. Scope: vec+raw_vec+vec_deque (the proven
# Vec<->VecDeque cluster). Widen to full collections once btree's NodeRef<auto>
# inference leak is fixed (it panics --expand today).
#
# Usage: build.sh <work_dir>
set -uo pipefail
W="${1:?usage: build.sh <work_dir>}"
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SRC="$(rustc --print sysroot)/lib/rustlib/src/rust/library/alloc/src"
rm -rf "$W"; mkdir -p "$W/src/collections"
cp -r "$SRC/vec" "$W/src/vec"
cp -r "$SRC/raw_vec" "$W/src/raw_vec"
cp -r "$SRC/collections/vec_deque" "$W/src/collections/vec_deque"
printf 'pub mod vec_deque;\npub use vec_deque::VecDeque;\n' > "$W/src/collections/mod.rs"
cat > "$W/Cargo.toml" <<EOF
[package]
name = "alloc_port"
version = "0.0.1"
edition = "2021"
[lib]
path = "src/lib.rs"
EOF
printf '#![allow(unused)]\npub mod raw_vec;\npub mod collections;\npub mod vec;\n' > "$W/src/lib.rs"
bash "$REPO/docs/alloc_port/prep.sh" "$W/src" >/dev/null
"$REPO/target/release/rusty-cpp-transpiler" --crate "$W/Cargo.toml" --expand --output-dir "$W/out" > "$W/transpile.log" 2>&1
echo "transpile exit=$? ($(tail -1 "$W/transpile.log"))"
CPPM="$W/out/alloc_port.cppm"
[[ -f "$CPPM" ]] || { echo "no single-module output — see $W/transpile.log"; exit 1; }
[[ -f "$REPO/docs/alloc_port/post_transpile_patch.py" ]] && python3 "$REPO/docs/alloc_port/post_transpile_patch.py" "$W/out" >/dev/null 2>&1
# strip circular/self imports (rusty umbrella + the OLD *_port modules this replaces)
sed -i '/^import rusty;$/d; /^import [a-z_]*_port\./d' "$CPPM"
FLAGS="-std=c++23 -DRUSTY_PORTABLE_INTRINSICS=1 -march=native -I$REPO/include -x c++-module"
clang++ $FLAGS --precompile -o "$W/out/alloc_port.pcm" "$CPPM" -ferror-limit=0 2> "$W/compile.err"
echo "compile: $(grep -c 'error:' "$W/compile.err") errors"
grep -hoE "error: .*" "$W/compile.err" | sed -E "s/'[^']*'/'X'/g; s/[0-9]+/N/g" | sort | uniq -c | sort -rn | head -10
