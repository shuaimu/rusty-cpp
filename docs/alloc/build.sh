#!/usr/bin/env bash
# Regenerate + compile the consolidated alloc as ONE C++ module.
#
# KEY: use --expand (cargo-expand → one flattened crate module). Per-submodule
# emission creates ILLEGAL C++ module cycles (alloc.vec <-> alloc.collections.
# vec_deque); Rust compiles a crate as ONE unit, so we emit ONE .cppm — no
# inter-module imports, no cycle. Scope: vec+raw_vec+vec_deque+binary_heap+
# linked_list+borrow. Still out: btree/rc/sync/boxed/string — gated on the
# Box<auto>::new_uninit / NonNull<auto> lateral-inference transpiler fix
# (rc 49 / arc 60 / btree 88 errors once unblocked) and, for boxed/string,
# builtin-type self-definition mapping suppression (see STATUS.md).
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
cp -r "$SRC/collections/binary_heap" "$W/src/collections/binary_heap"
cp "$SRC/collections/linked_list.rs" "$W/src/collections/linked_list.rs"
cp -r "$SRC/collections/btree" "$W/src/collections/btree"
# boxed fold: opt-in while the mixed Box-binding fallout is open — crate-Box
# declared returns leak `::boxed::Box` into consumer modules' inferred
# lambda/return annotations while expr owners keep the runtime rusty::Box
# spelling (the #53 type-tail-keyed registry family). Module itself compiles
# clean under the flag; consumer instantiation from the test TU does not yet.
if [[ "${RUSTY_ALLOC_WITH_BOXED:-0}" == "1" ]]; then
  mkdir -p "$W/src/boxed"
  cp "$SRC/boxed.rs" "$W/src/boxed.rs"
  cp "$SRC/boxed/convert.rs" "$W/src/boxed/convert.rs"
  cp "$SRC/boxed/iter.rs" "$W/src/boxed/iter.rs"
fi
cp "$SRC/borrow.rs" "$W/src/borrow.rs"
cp "$SRC/rc.rs" "$W/src/rc.rs"
cp "$SRC/sync.rs" "$W/src/sync.rs"
printf 'pub mod vec_deque;\npub use vec_deque::VecDeque;\npub mod binary_heap;\npub use binary_heap::BinaryHeap;\npub mod linked_list;\npub use linked_list::LinkedList;\nmod btree;\npub mod btree_map {\n    pub use super::btree::map::*;\n}\npub mod btree_set {\n    pub use super::btree::set::*;\n}\npub use btree_map::BTreeMap;\npub use btree_set::BTreeSet;\n' > "$W/src/collections/mod.rs"
cat > "$W/Cargo.toml" <<EOF
[package]
name = "alloc"
version = "0.0.1"
edition = "2021"
[lib]
path = "src/lib.rs"
# Empty [workspace] so cargo-expand treats this as a standalone crate even
# when the work dir is INSIDE the repo (e.g. .rusty-parity-matrix/alloc);
# otherwise cargo believes it belongs to the repo workspace and `cargo expand`
# aborts, forcing the per-submodule fallback (illegal C++ module cycle).
[workspace]
EOF
if [[ "${RUSTY_ALLOC_WITH_BOXED:-0}" == "1" ]]; then
  printf '#![allow(unused)]\npub mod raw_vec;\npub mod boxed;\npub mod borrow;\npub mod rc;\npub mod sync;\npub mod collections;\npub mod vec;\n' > "$W/src/lib.rs"
else
  printf '#![allow(unused)]\npub mod raw_vec;\npub mod borrow;\npub mod rc;\npub mod sync;\npub mod collections;\npub mod vec;\n' > "$W/src/lib.rs"
fi
bash "$REPO/docs/alloc/prep.sh" "$W/src" >/dev/null
TRANSPILER="${RUSTY_CPP_TRANSPILER_BIN:-$REPO/target/release/rusty-cpp-transpiler}"
"$TRANSPILER" --crate "$W/Cargo.toml" --expand --output-dir "$W/out" > "$W/transpile.log" 2>&1
echo "transpile exit=$? ($(tail -1 "$W/transpile.log"))"
CPPM="$W/out/alloc.cppm"
[[ -f "$CPPM" ]] || { echo "no single-module output — see $W/transpile.log"; exit 1; }
[[ -f "$REPO/docs/alloc/post_transpile_patch.py" ]] && python3 "$REPO/docs/alloc/post_transpile_patch.py" "$W/out" >/dev/null 2>&1
# strip circular/self imports (rusty umbrella + the OLD *_port modules this replaces)
sed -i '/^import rusty;$/d; /^import [a-z_]*_port\./d' "$CPPM"
FLAGS="-std=c++23 -DRUSTY_PORTABLE_INTRINSICS=1 -march=native -I$REPO/include -x c++-module"
clang++ $FLAGS --precompile -o "$W/out/alloc.pcm" "$CPPM" -ferror-limit=0 2> "$W/compile.err"
echo "compile: $(grep -c 'error:' "$W/compile.err") errors"
grep -hoE "error: .*" "$W/compile.err" | sed -E "s/'[^']*'/'X'/g; s/[0-9]+/N/g" | sort | uniq -c | sort -rn | head -10
