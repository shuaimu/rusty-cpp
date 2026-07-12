#!/usr/bin/env bash
# Regenerate + compile the transpiled Rust **std** port as C++20 module
# `rusty` (std collides with C++'s std namespace — see the std-family roadmap
# in docs/port_regen/STATUS.md).
#
# First slice: std::collections::hash — HashMap/HashSet over a vendored
# hashbrown 0.16.1 (std's own pin) — plus std::hash (RandomState with
# fixed-seed stub, DefaultHasher over rusty::hash::SipHasher). The hashbrown
# dependency is a LOCAL PATH dep so the transpiler recursively transpiles it
# to a sibling `hashbrown` module (registry deps are not transpiled).
#
# Usage: build.sh <work_dir>
set -uo pipefail
W="${1:?usage: build.sh <work_dir>}"
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SRC="$(rustc --print sysroot)/lib/rustlib/src/rust/library/std/src"
rm -rf "$W"; mkdir -p "$W/src/collections/hash" "$W/src/hash"

# --- std sources (raw; prep.sh rewrites in place) ---
cp "$SRC/collections/hash/map.rs" "$W/src/collections/hash/map.rs"
cp "$SRC/collections/hash/set.rs" "$W/src/collections/hash/set.rs"
cp "$SRC/hash/random.rs" "$W/src/hash/random.rs"
printf 'pub mod map;\npub mod set;\n' > "$W/src/collections/hash/mod.rs"
printf 'pub mod hash;\npub use hash::map::HashMap;\npub use hash::set::HashSet;\n' > "$W/src/collections/mod.rs"
printf 'pub mod random;\npub use random::{DefaultHasher, RandomState};\n' > "$W/src/hash/mod.rs"
printf '#![allow(unused)]\n#![allow(deprecated)]\npub mod collections;\npub mod hash;\n' > "$W/src/lib.rs"

# --- vendored hashbrown 0.16.1 (registry cache, fallback: git tag) ---
HB_CACHE="$(ls -d "$HOME"/.cargo/registry/src/*/hashbrown-0.16.1 2>/dev/null | head -1)"
if [[ -n "$HB_CACHE" ]]; then
  cp -r "$HB_CACHE" "$W/hashbrown"
else
  git clone --depth 1 --branch v0.16.1 https://github.com/rust-lang/hashbrown.git "$W/hashbrown" \
    || { echo "hashbrown 0.16.1 unavailable (no registry cache, clone failed)"; exit 1; }
fi
rm -rf "$W/hashbrown/tests" "$W/hashbrown/benches" "$W/hashbrown/.git" 2>/dev/null || true
# Trimmed manifest: no foldhash/allocator deps; rustc-internal-api gives std's
# RustcEntry surface; [workspace] keeps cargo-expand standalone.
cat > "$W/hashbrown/Cargo.toml" <<EOF
[package]
edition = "2021"
name = "hashbrown"
version = "0.16.1"
[lib]
name = "hashbrown"
path = "src/lib.rs"
[features]
default = ["rustc-internal-api"]
inline-more = []
raw-entry = []
rustc-internal-api = []
[workspace]
EOF

cat > "$W/Cargo.toml" <<EOF
[package]
name = "rusty"
version = "0.0.1"
edition = "2021"
[lib]
path = "src/lib.rs"
[dependencies]
hashbrown = { path = "./hashbrown", default-features = false, features = ["rustc-internal-api"] }
# Empty [workspace] (+ exclude the vendored dep) so cargo-expand treats this
# as standalone even inside the repo tree (same gotcha as docs/alloc).
[workspace]
exclude = ["hashbrown"]
EOF

bash "$REPO/docs/rusty/prep.sh" "$W/src" >/dev/null
TRANSPILER="${RUSTY_CPP_TRANSPILER_BIN:-$REPO/target/release/rusty-cpp-transpiler}"
"$TRANSPILER" --crate "$W/Cargo.toml" --expand --output-dir "$W/out" > "$W/transpile.log" 2>&1
echo "transpile exit=$? ($(tail -1 "$W/transpile.log"))"
[[ -f "$W/out/rusty.cppm" ]] || { echo "no rusty.cppm — see $W/transpile.log"; exit 1; }
[[ -f "$W/out/hashbrown/hashbrown.cppm" ]] || { echo "no hashbrown.cppm (dep not transpiled?)"; exit 1; }

python3 "$REPO/docs/rusty/post_transpile_patch.py" "$W/out" || exit 1

FLAGS="-std=c++23 -DRUSTY_PORTABLE_INTRINSICS=1 -march=native -I$REPO/include -x c++-module"
clang++ $FLAGS --precompile -o "$W/out/hashbrown/hashbrown.pcm" \
  "$W/out/hashbrown/hashbrown.cppm" -ferror-limit=0 2> "$W/hb_compile.err"
echo "hashbrown compile: $(grep -c 'error:' "$W/hb_compile.err") errors"
clang++ $FLAGS --precompile -fmodule-file=hashbrown="$W/out/hashbrown/hashbrown.pcm" \
  -o "$W/out/rusty.pcm" "$W/out/rusty.cppm" -ferror-limit=0 2> "$W/compile.err"
echo "rusty compile: $(grep -c 'error:' "$W/compile.err") errors"
grep -hoE "error: .*" "$W/compile.err" "$W/hb_compile.err" 2>/dev/null \
  | sed -E "s/'[^']*'/'X'/g; s/[0-9]+/N/g" | sort | uniq -c | sort -rn | head -8
