#!/usr/bin/env bash
# Prep the CONSOLIDATED alloc port: Rust's `alloc` crate (or the container
# subset) as ONE crate, so intra-crate cycles (Vec<->VecDeque, btree->Vec,
# etc.) resolve within one compilation unit instead of being stubbed.
#
# KEY DIFFERENCE from the old per-port preps: PRESERVE the intra-crate refs
# (crate::{vec,raw_vec,collections,...}); rewrite ONLY genuinely-external
# refs (core, the allocator API, fmt) to the rusty headers. The old preps
# severed crate::collections::VecDeque -> alloc::collections::VecDeque
# "until VecDeque is also ported" — that severing forced into_vecdeque's
# abort() stub. Here they're siblings, so it resolves for real.
#
# Usage: prep.sh <crate_src_dir>   (dir already populated with the subtrees)
set -euo pipefail
SRC="${1:?usage: prep.sh <crate_src_dir>}"
[[ -d "$SRC" ]] || { echo "error: $SRC not a directory" >&2; exit 1; }

find "$SRC" -name "tests" -type d -exec rm -rf {} + 2>/dev/null || true
find "$SRC" -name "tests.rs" -delete 2>/dev/null || true

find "$SRC" -name "*.rs" -exec sed -i \
  -e 's|use crate::alloc::|use std::alloc::|g' \
  -e 's|crate::alloc::Allocator|std::alloc::Allocator|g' \
  -e 's|crate::alloc::Global|std::alloc::Global|g' \
  -e 's|crate::alloc::Layout|std::alloc::Layout|g' \
  -e 's|crate::alloc::AllocError|std::alloc::AllocError|g' \
  -e 's|crate::alloc::handle_alloc_error|std::alloc::handle_alloc_error|g' \
  -e 's|use crate::boxed::Box|use alloc::boxed::Box|g' \
  -e 's|crate::boxed::Box|alloc::boxed::Box|g' \
  -e 's|crate::rc::Rc|std::rc::Rc|g' \
  -e 's|use crate::rc::|use std::rc::|g' \
  -e 's|crate::sync::Arc|std::sync::Arc|g' \
  -e 's|use crate::sync::|use std::sync::|g' \
  -e 's|realalloc::collections::TryReserveError|std::collections::TryReserveError|g' \
  -e 's|realalloc::collections::TryReserveErrorKind|std::collections::TryReserveErrorKind|g' \
  -e 's|use crate::borrow::|use std::borrow::|g' \
  -e 's|crate::borrow::Cow|std::borrow::Cow|g' \
  -e 's|crate::borrow::ToOwned|std::borrow::ToOwned|g' \
  -e 's|use crate::fmt|use std::fmt|g' \
  -e 's|crate::fmt::|std::fmt::|g' \
  -e 's|use core::|use std::|g' \
  -e 's|use crate::slice::|use std::slice::|g' \
  -e 's|use crate::str::|use std::str::|g' \
  -e 's|crate::string::String|std::string::String|g' \
  -e 's|use crate::string::|use std::string::|g' \
  -e 's|^const impl|impl|' \
  {} \;
echo "prep.sh complete: $SRC"
