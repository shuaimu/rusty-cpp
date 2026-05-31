#!/usr/bin/env bash
# Pre-process a vendored copy of rustc's library/alloc/src/collections/vec_deque/
# for transpilation with rusty-cpp-transpiler. Multi-file port mirroring docs/vec_port/.
#
# Usage:
#   bash docs/vec_deque_port/prep.sh <src_dir>
set -euo pipefail
SRC="${1:?usage: prep.sh <src_dir>}"
[[ -d "$SRC" ]] || { echo "error: $SRC is not a directory" >&2; exit 1; }

find "$SRC" -name "tests*" -type d -exec rm -rf {} + 2>/dev/null || true
find "$SRC" -name "tests.rs" -type f -delete 2>/dev/null || true

find "$SRC" -name "*.rs" -exec sed -i \
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
  {} \;
echo "[vec_deque_port prep] normalized $SRC"
