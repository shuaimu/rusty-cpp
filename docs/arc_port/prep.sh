#!/usr/bin/env bash
# Pre-process a vendored copy of rustc's library/alloc/src/collections/binary_heap/
# for transpilation with rusty-cpp-transpiler. Mirrors docs/btreemap_port/prep.sh.
#
# Usage:
#   RUSTSRC=$(ls -d ~/.rustup/toolchains/*/lib/rustlib/src/rust/library/alloc/src/collections/binary_heap/ | head -1)
#   mkdir -p /tmp/arc_port/binary_heap_crate/src
#   cp $RUSTSRC/mod.rs /tmp/arc_port/binary_heap_crate/src/lib.rs
#   cp docs/arc_port/Cargo.toml.template /tmp/arc_port/binary_heap_crate/Cargo.toml
#   bash docs/arc_port/prep.sh /tmp/arc_port/binary_heap_crate/src/lib.rs
set -euo pipefail

SRC="${1:?usage: prep.sh <lib.rs>}"
if [[ ! -f "$SRC" ]]; then
  echo "error: $SRC is not a file" >&2
  exit 1
fi

sed -i \
  -e 's|use crate::alloc::|use std::alloc::|g' \
  -e 's|crate::alloc::Allocator|std::alloc::Allocator|g' \
  -e 's|crate::alloc::Global|std::alloc::Global|g' \
  -e 's|crate::alloc::Layout|std::alloc::Layout|g' \
  -e 's|crate::alloc::AllocError|std::alloc::AllocError|g' \
  -e 's|use crate::boxed::Box|use alloc::boxed::Box|g' \
  -e 's|crate::boxed::Box|alloc::boxed::Box|g' \
  -e 's|use crate::vec::|use std::vec::|g' \
  -e 's|crate::vec::Vec|std::vec::Vec|g' \
  -e 's|use crate::collections::|use std::collections::|g' \
  -e 's|crate::collections::TryReserveError|std::collections::TryReserveError|g' \
  -e 's|use core::|use std::|g' \
  -e 's|use crate::slice::|use std::slice::|g' \
  "$SRC"
echo "[arc_port prep] normalized $SRC"

# 2026-10 (pinned 1.95.0 sync.rs): `#[cfg(not(no_global_oom_handling))]` on a
# const (MAX_STATIC_INNER_SLICE_ALIGNMENT) is a predicate the transpiler
# cannot preserve on a const item. The cfg is always true for us (we have
# global OOM handling), so dropping the attribute keeps the item.
sed -i '/^\s*#\[cfg(not(no_global_oom_handling))\]\s*$/d' "$SRC"
