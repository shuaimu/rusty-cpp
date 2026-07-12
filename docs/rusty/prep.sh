#!/usr/bin/env bash
# Prep the std::collections::hash slice (as crate `rusty`) for cargo-expand +
# transpile. Modeled on docs/alloc/prep.sh: rewrite genuinely-external crate::
# refs to std:: (the transpiler maps std:: to the rusty headers); PRESERVE the
# intra-crate ones (crate::hash::RandomState); stub the sys::random seeding.
set -euo pipefail
SRC="${1:?usage: prep.sh <crate_src_dir>}"

find "$SRC" -name "tests" -type d -exec rm -rf {} + 2>/dev/null || true
find "$SRC" -name "tests.rs" -delete 2>/dev/null || true

# --- external refs -> std:: (transpiler-mapped), like alloc's prep ---
find "$SRC" -name "*.rs" -exec sed -i \
  -e 's|use crate::alloc::|use std::alloc::|g' \
  -e 's|crate::alloc::Allocator|std::alloc::Allocator|g' \
  -e 's|crate::alloc::Global|std::alloc::Global|g' \
  -e 's|use crate::borrow::|use std::borrow::|g' \
  -e 's|use crate::collections::|use std::collections::|g' \
  -e 's|use crate::error::|use std::error::|g' \
  -e 's|use crate::fmt|use std::fmt|g' \
  -e 's|crate::fmt::|std::fmt::|g' \
  -e 's|use crate::iter::|use std::iter::|g' \
  -e 's|use crate::ops::|use std::ops::|g' \
  -e 's|use crate::cell::|use std::cell::|g' \
  -e 's|use core::|use std::|g' \
  -e 's|^const impl|impl|' \
  -e 's|impl const |impl |' \
  {} \;

# --- split std-vs-crate hash imports: BuildHasher/Hash are std::hash,
#     RandomState is OUR crate::hash::random ---
find "$SRC" -name "*.rs" -exec sed -i \
  -e 's|use crate::hash::{BuildHasher, Hash, RandomState};|use std::hash::{BuildHasher, Hash};\nuse crate::hash::RandomState;|' \
  {} \;

RAND="$SRC/hash/random.rs"
if [[ -f "$RAND" ]]; then
  # SipHasher13 is a core-internal type; alias the public (deprecated) SipHasher.
  sed -i 's|use super::{BuildHasher, Hasher, SipHasher13};|use std::hash::{BuildHasher, Hasher};\n#[allow(deprecated)]\nuse std::hash::SipHasher as SipHasher13;|' "$RAND"
  # sys::random seeding -> fixed constants (spike: determinism over DoS resistance)
  sed -i '/use crate::sys::random::hashmap_random_keys;/d' "$RAND"
  # Hasher::write_str is a nightly trait method - drop the override (SipHasher
  # forwarding covers write()).
  python3 - "$RAND" <<'PYEOF'
import sys, pathlib
p = pathlib.Path(sys.argv[1]); s = p.read_text()
old = """    #[inline]
    fn write_str(&mut self, s: &str) {
        self.0.write_str(s);
    }

"""
if old in s:
    s = s.replace(old, "")
    print("  dropped DefaultHasher write_str override")
# RandomState::new: thread_local! + OS seeding -> fixed constants.
start = s.find("    pub fn new() -> RandomState {")
if start >= 0:
    end = s.find("\n    }\n", start)
    assert end > 0
    s = s[:start] + """    pub fn new() -> RandomState {
        RandomState { k0: 0x9e3779b97f4a7c15, k1: 0x6a09e667f3bcc909 }
    }
""" + s[end + len("\n    }\n"):]
    print("  stubbed RandomState::new seeding to fixed constants")
p.write_text(s)
PYEOF
fi

# --- io-cursor slice: import split + block surgery ---
if [[ -d "$SRC/io" ]]; then
  sed -i \
    -e 's|use crate::cmp;|use std::cmp;|' \
    -e 's|use crate::io::{self, BorrowedCursor, ErrorKind, IoSlice, IoSliceMut, SeekFrom};|use crate::io::{self, ErrorKind, SeekFrom};|' \
    "$SRC/io/cursor.rs"
  python3 "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/prep_io.py" "$SRC/io"
fi
echo "prep.sh complete: $SRC"
