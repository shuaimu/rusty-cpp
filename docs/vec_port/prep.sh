#!/usr/bin/env bash
# Pre-process a vendored copy of rustc's library/alloc/src/{vec,raw_vec}/
# for transpilation with rusty-cpp-transpiler.
#
# Why: like the BTreeMap port, the Vec source uses several crate-
# internal paths that don't resolve when only the vec/ + raw_vec/
# subtrees are vendored. We rewrite those to the equivalent public
# std::* paths so the transpiler's existing std-mapping table can
# route them through to rusty::* without a separate crate-resolution
# pass.
#
# Usage:
#   bash prep.sh <vec_dir> <raw_vec_dir>
#
# Where <vec_dir> is a copy of library/alloc/src/vec/ and
# <raw_vec_dir> is a copy of library/alloc/src/raw_vec/.
#
# Idempotent — safe to re-run.
set -euo pipefail

VEC_DIR="${1:?usage: prep.sh <vec_dir> <raw_vec_dir>}"
RAW_VEC_DIR="${2:?usage: prep.sh <vec_dir> <raw_vec_dir>}"

if [[ ! -d "$VEC_DIR" ]]; then
  echo "error: $VEC_DIR is not a directory" >&2
  exit 1
fi
if [[ ! -d "$RAW_VEC_DIR" ]]; then
  echo "error: $RAW_VEC_DIR is not a directory" >&2
  exit 1
fi

# Strip the rustc-internal tests — they don't transpile (they depend
# on rand and other test-only crates).
find "$VEC_DIR" "$RAW_VEC_DIR" -name "tests*" -type d -exec rm -rf {} + 2>/dev/null || true
find "$VEC_DIR" "$RAW_VEC_DIR" -name "tests.rs" -type f -delete 2>/dev/null || true

# crate::alloc::* → std::alloc::* (Allocator, Global, Layout, AllocError,
# handle_alloc_error)
find "$VEC_DIR" "$RAW_VEC_DIR" -name "*.rs" -exec sed -i \
  -e 's|use crate::alloc::|use std::alloc::|g' \
  -e 's|crate::alloc::Allocator|std::alloc::Allocator|g' \
  -e 's|crate::alloc::Global|std::alloc::Global|g' \
  -e 's|crate::alloc::Layout|std::alloc::Layout|g' \
  -e 's|crate::alloc::AllocError|std::alloc::AllocError|g' \
  -e 's|crate::alloc::handle_alloc_error|std::alloc::handle_alloc_error|g' \
  {} \;

# crate::boxed::Box → alloc::boxed::Box
find "$VEC_DIR" "$RAW_VEC_DIR" -name "*.rs" -exec sed -i \
  -e 's|use crate::boxed::Box|use alloc::boxed::Box|g' \
  -e 's|crate::boxed::Box|alloc::boxed::Box|g' \
  {} \;

# crate::raw_vec::RawVec → super::raw_vec::RawVec (sibling reference;
# both vec and raw_vec are vendored as sibling modules in the crate)
find "$VEC_DIR" -name "*.rs" -exec sed -i \
  -e 's|use crate::raw_vec::|use super::raw_vec::|g' \
  -e 's|crate::raw_vec::|super::raw_vec::|g' \
  {} \;

# crate::collections::TryReserveError → std::collections::TryReserveError
# (TryReserveError is in core::collections::TryReserveError via re-export)
find "$VEC_DIR" "$RAW_VEC_DIR" -name "*.rs" -exec sed -i \
  -e 's|use crate::collections::TryReserveError|use std::collections::TryReserveError|g' \
  -e 's|crate::collections::TryReserveError|std::collections::TryReserveError|g' \
  -e 's|use crate::collections::TryReserveErrorKind|use std::collections::TryReserveErrorKind|g' \
  -e 's|crate::collections::TryReserveErrorKind|std::collections::TryReserveErrorKind|g' \
  {} \;

# crate::collections::VecDeque → alloc::collections::VecDeque
# (only used in cow.rs / extract_if.rs for From conversions; deferred
# until VecDeque is also ported. The transpiler should treat this as
# external for now.)
find "$VEC_DIR" -name "*.rs" -exec sed -i \
  -e 's|use crate::collections::VecDeque|use alloc::collections::VecDeque|g' \
  {} \;

# crate::borrow::{Cow, ToOwned} → std::borrow::{Cow, ToOwned}
find "$VEC_DIR" -name "*.rs" -exec sed -i \
  -e 's|use crate::borrow::|use std::borrow::|g' \
  -e 's|crate::borrow::Cow|std::borrow::Cow|g' \
  -e 's|crate::borrow::ToOwned|std::borrow::ToOwned|g' \
  {} \;

# crate::fmt → std::fmt (formatting impls in vec/mod.rs Debug etc.)
find "$VEC_DIR" "$RAW_VEC_DIR" -name "*.rs" -exec sed -i \
  -e 's|use crate::fmt|use std::fmt|g' \
  -e 's|crate::fmt::|std::fmt::|g' \
  {} \;

# Strip Drop-implementation deny lints (rustc-internal lint config)
find "$VEC_DIR" "$RAW_VEC_DIR" -name "*.rs" -exec sed -i \
  -e '/^#!\[allow_internal_unstable/d' \
  -e '/^#!\[deny(unsafe_op_in_unsafe_fn)\]/d' \
  -e '/^#!\[stable(/d' \
  -e '/^#!\[unstable(/d' \
  {} \;

# --- Strip unstable Rust features the transpiler's syn parser can't handle ---

# Unstable RFC 3762: `const impl<T, A: [const] Allocator + [const] Destruct>`
# (conditionally-const trait bounds). syn 2.x doesn't parse the `[const]`
# bracket form. Strip:
#   1) Leading `const ` on `const impl ...`
#   2) `[const] ` prefix on trait bounds (Allocator, Destruct, etc.)
#   3) ` + [const] Destruct` (the Destruct trait is rustc-internal; we drop
#      the whole conjunct since C++ doesn't model conditional Drop)
#
# Affects: vec/mod.rs L905, raw_vec/mod.rs L169 + L428.
find "$VEC_DIR" "$RAW_VEC_DIR" -name "*.rs" -exec sed -i \
  -e 's|^const impl|impl|' \
  -e 's| + \[const\] Destruct||g' \
  -e 's|\[const\] ||g' \
  {} \;

# Strip module-level cfg gates that are test-only or feature-gated and
# would otherwise confuse the transpiler (it sees the cfg as part of
# the parse tree but can't evaluate the predicate).
# (Conservative — only remove patterns we've seen blocking specifically.)

echo "prep.sh complete: vec_dir=$VEC_DIR raw_vec_dir=$RAW_VEC_DIR"
echo "(idempotent — safe to re-run)"
