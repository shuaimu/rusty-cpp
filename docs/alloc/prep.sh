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
  -e 's|use crate::fmt|use std::fmt|g' \
  -e 's|crate::fmt::|std::fmt::|g' \
  -e 's|use core::|use std::|g' \
  -e 's|use crate::slice::|use std::slice::|g' \
  -e 's|use crate::str::|use std::str::|g' \
  -e 's|crate::string::String|std::string::String|g' \
  -e 's|use crate::string::|use std::string::|g' \
  -e 's|^const impl|impl|' \
  {} \;

# raw_vec's Cap = rustc-internal niche-optimized usize → plain usize (drops
# the Option<Cap> niche optimization; functionality preserved).
find "$SRC" -name "*.rs" -exec sed -i \
  -e 's|core::num::niche_types::UsizeNoHighBit|usize|g' \
  -e 's|num::niche_types::UsizeNoHighBit|usize|g' \
  -e 's|unsafe { Cap::new_unchecked(\([^)]*\)) }|\1|g' \
  -e 's|Cap::new_unchecked(\([^)]*\))|\1|g' \
  -e 's|Cap::ZERO|0usize|g' \
  -e 's|const ZERO_CAP: Cap = 0;|const ZERO_CAP: Cap = 0usize;|g' \
  {} \;

RV="$SRC/raw_vec/mod.rs"
if [[ -f "$RV" ]]; then
  # nightly provenance APIs → the older Unique::new_unchecked form.
  sed -i 's|let ptr = Unique::from_non_null(NonNull::without_provenance(align.as_nonzero()));|let ptr = unsafe { Unique::new_unchecked(ptr::without_provenance_mut(align.as_usize())) };|' "$RV"
  # finish_grow's `if let Some((ptr, old_layout))` tuple-destructure → explicit
  # match (the transpiler emits the destructure without binding the components).
  python3 - "$RV" <<'PYEOF'
import sys, pathlib
p = pathlib.Path(sys.argv[1]); s = p.read_text()
old = """        let memory = if let Some((ptr, old_layout)) = unsafe { self.current_memory(elem_layout) } {
            // FIXME(const-hack): switch to `debug_assert_eq`
            debug_assert!(old_layout.align() == new_layout.align());
            unsafe {
                // The allocator checks for alignment equality
                hint::assert_unchecked(old_layout.align() == new_layout.align());
                self.alloc.grow(ptr, old_layout, new_layout)
            }
        } else {
            self.alloc.allocate(new_layout)
        };"""
new = """        let _curmem = unsafe { self.current_memory(elem_layout) };
        let memory = if _curmem.is_some() {
            let _pair = _curmem.unwrap();
            let ptr = _pair.0;
            let old_layout = _pair.1;
            debug_assert!(old_layout.align() == new_layout.align());
            unsafe {
                hint::assert_unchecked(old_layout.align() == new_layout.align());
                self.alloc.grow(ptr, old_layout, new_layout)
            }
        } else {
            self.alloc.allocate(new_layout)
        };"""
if old in s and new not in s:
    p.write_text(s.replace(old, new)); print("  rewrote finish_grow if-let-tuple-destructure")
PYEOF
fi
echo "prep.sh complete: $SRC"
