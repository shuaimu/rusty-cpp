#!/usr/bin/env bash
# Pre-process a vendored copy of rustc's library/alloctests/testing/
# helpers for transpilation. Mirrors docs/binary_heap_port/prep.sh.
#
# Usage:
#   RUSTSRC=$(ls -d ~/.rustup/toolchains/*/lib/rustlib/src/rust/library/alloctests/testing/ | head -1)
#   TGT=/tmp/testing_port
#   mkdir -p $TGT/testing_crate/src
#   cp $RUSTSRC/*.rs $TGT/testing_crate/src/
#   cp docs/testing_port/Cargo.toml.template $TGT/testing_crate/Cargo.toml
#   bash docs/testing_port/prep.sh $TGT/testing_crate/src
set -euo pipefail

SRCDIR="${1:?usage: prep.sh <src-dir>}"
if [[ ! -d "$SRCDIR" ]]; then
  echo "error: $SRCDIR is not a directory" >&2
  exit 1
fi

# `mod.rs` declares submodules with `pub(crate)`. Promote to `pub` so
# downstream consumers can reference them.
sed -i \
  -e 's|pub(crate) mod|pub mod|g' \
  "$SRCDIR/mod.rs"

# `lib.rs` is what the transpiler reads. Move mod.rs there.
mv "$SRCDIR/mod.rs" "$SRCDIR/lib.rs"

# Each helper file uses `pub(crate)` on its public items. Promote to
# `pub` so a downstream crate (e.g. binary_heap test port) can import.
for f in "$SRCDIR"/*.rs; do
  sed -i 's|pub(crate)|pub|g' "$f"
done

# `use core::` → `use std::` (matches binary_heap_port convention).
for f in "$SRCDIR"/*.rs; do
  sed -i 's|use core::|use std::|g' "$f"
done

echo "[testing_port prep] normalized $SRCDIR"
