#!/usr/bin/env bash
# Pre-process a vendored copy of rustc's library/alloc/src/collections/btree/
# for transpilation with rusty-cpp-transpiler.
#
# Why: the stdlib source uses several crate-internal paths that don't
# resolve when only the `btree` subtree is vendored — there is no
# enclosing `crate::alloc::*` or `crate::boxed::*` to look up. We
# rewrite those to the equivalent public std::* paths so the
# transpiler's existing std-mapping table can route them through to
# rusty::* without a separate crate-resolution pass.
#
# Usage: bash prep.sh <btree_dir>
# where <btree_dir> is a copy of the stdlib btree/ source.
#
# Idempotent — safe to re-run.
set -euo pipefail

BTREE_DIR="${1:?usage: prep.sh <btree_dir>}"

if [[ ! -d "$BTREE_DIR" ]]; then
  echo "error: $BTREE_DIR is not a directory" >&2
  exit 1
fi

# Strip the rustc-internal tests/ subdirectories — they don't transpile
# (they depend on rand and other test-only crates).
find "$BTREE_DIR" -name "tests*" -type d -exec rm -rf {} + 2>/dev/null || true

# crate::alloc::* → std::alloc::* (Allocator, Global, Layout, AllocError)
# The btree code uses `crate::alloc::*` to reach the alloc crate's
# internal alloc module, but when only the btree subtree is vendored
# there is no enclosing alloc crate. Re-route to `std::alloc::*` so
# the transpiler's std-mapping table picks them up.
find "$BTREE_DIR" -name "*.rs" -exec sed -i \
  -e 's|use crate::alloc::|use std::alloc::|g' \
  -e 's|crate::alloc::Allocator|std::alloc::Allocator|g' \
  -e 's|crate::alloc::Global|std::alloc::Global|g' \
  -e 's|crate::alloc::Layout|std::alloc::Layout|g' \
  -e 's|crate::alloc::AllocError|std::alloc::AllocError|g' \
  {} \;

# crate::boxed::Box → alloc::boxed::Box (same reasoning — there's no
# `crate::boxed` because the alloc crate's boxed module isn't
# vendored alongside btree).
find "$BTREE_DIR" -name "*.rs" -exec sed -i \
  -e 's|use crate::boxed::Box|use alloc::boxed::Box|g' \
  {} \;

# crate::vec::Vec → alloc::vec::Vec (same reasoning).
find "$BTREE_DIR" -name "*.rs" -exec sed -i \
  -e 's|use crate::vec::Vec|use alloc::vec::Vec|g' \
  {} \;

echo "Port-prep complete for $BTREE_DIR"
