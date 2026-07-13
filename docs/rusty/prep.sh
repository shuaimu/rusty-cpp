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
# ============================================================================
# std::error slice (merged core::error + std::error in src/error.rs).
# Runs AFTER the generic seds above, so markers match post-rewrite text.
# Stubs (each reported by the widening probe):
#   S1  use crate::any::TypeId          -> deleted (no rusty::any::TypeId yet)
#   S2  Error::type_id + mod private    -> deleted (needs TypeId)
#   S3  impl dyn Error downcast trio    -> deleted (needs TypeId + ptr casts)
#   S4  Request/Tagged/Erased machinery -> stub Request + request_* -> None
#       (nightly provide API; pointer-tagging is untranspilable — this is the
#        std::error::Request recursion the alloc boxed probe hit)
#   S5  impl Error for ! / FusedIterator / external-type impls -> deleted
#   S6  crate::backtrace::Backtrace     -> local inert stub (OS/sys boundary)
#   S7  duplicate `use std::fmt::{self, Write}` (std part) -> fmt::Write only
# ============================================================================
ERR="$SRC/error.rs"
if [[ -f "$ERR" ]]; then
  python3 - "$ERR" <<'PYEOF'
import re, sys, pathlib
p = pathlib.Path(sys.argv[1]); s = p.read_text()
def cut(s, start, end, keep_end, label):
    i = s.find(start)
    assert i >= 0, f"start marker missing: {label}"
    j = s.find(end, i)
    assert j > i, f"end marker missing: {label}"
    if not keep_end:
        j += len(end)
    print(f"  cut {label}")
    return s[:i] + s[j:]

# S1: TypeId import
s = s.replace("use crate::any::TypeId;\n", "")
# S2a: trait method type_id (attrs through closing brace)
s = cut(s, "    #[doc(hidden)]\n    #[unstable(\n        feature = \"error_type_id\",",
        "        TypeId::of::<Self>()\n    }\n", False, "Error::type_id")
# S2b: mod private
s = cut(s, "mod private {", "\n}\n", False, "mod private")
# S5a: impl Error for !
s = s.replace('#[unstable(feature = "never_type", issue = "35121")]\nimpl Error for ! {}\n', "")
# S3: the three dyn-Error downcast impl blocks (is/downcast_ref/downcast_mut)
s = cut(s, "// Copied from `any.rs`.", "impl dyn Error {", True, "dyn downcast impls")
# S4: Request pointer-tagging machinery -> inert stubs
s = cut(s, '#[unstable(feature = "error_generic_member_access", issue = "99301")]\npub fn request_value',
        "/// An iterator over an [`Error`] and its sources.", True, "Request machinery")
stub = '''// STUB (prep S4): nightly Request/provide (Tagged/Erased/TypeId pointer
// tagging) is not transpilable; request_* always answer "nothing provided".
pub struct Request<'a>(std::marker::PhantomData<&'a mut &'a ()>);
pub fn request_value<'a, T>(_err: &'a (impl Error + ?Sized)) -> Option<T>
where T: 'static {
    None
}
pub fn request_ref<'a, T>(_err: &'a (impl Error + ?Sized)) -> Option<&'a T>
where T: 'static + ?Sized {
    None
}

'''
i = s.find("/// An iterator over an [`Error`] and its sources.")
s = s[:i] + stub + s[i:]
# S5b: FusedIterator marker impl (inline crate::iter path would not resolve)
s = re.sub(r'#\[unstable\(feature = "error_iter", issue = "58520"\)\]\nimpl<\'a> crate::iter::FusedIterator for Source<\'a> \{\}\n', "", s, count=1)
# S5c: Error impls for external (non-slice) types
s, n = re.subn(r'#\[stable\([^)]*\)\]\nimpl Error for (?:crate|std)::[A-Za-z_:]+ \{\}\n', "", s)
print(f"  dropped {n} external-type Error impls")
# S8: blanket `impl Error for &'a T` — emits an __ufcs_Error deref-forwarding
# shim whose using-decls into Error_ collide with the trait's own
# default-method templates (same template<T>(const T&) signature; Rust
# disambiguates by impl selection, the C++ flattening cannot).
s = cut(s, '#[stable(feature = "error_by_ref", since = "1.51.0")]\nimpl<\'a, T: Error + ?Sized> Error for &\'a T {',
        "\n}\n", False, "blanket impl Error for &T")
# std part: kill the now-self-referential re-exports of the merged module
s = s.replace('#[stable(feature = "rust1", since = "1.0.0")]\npub use std::error::Error;\n', "")
s = s.replace('#[unstable(feature = "error_generic_member_access", issue = "99301")]\npub use std::error::{Request, request_ref, request_value};\n', "")
# S7: second fmt import would collide on `self`
s = s.replace("use std::fmt::{self, Write};", "use std::fmt::Write;")
# S6: Backtrace stub (OS boundary: std::backtrace -> sys, hand-written runtime)
s = s.replace("use crate::backtrace::Backtrace;\n", '''// STUB (prep S6): std::backtrace is OS/sys layer, not transpiled.
#[derive(Debug)]
pub struct Backtrace(());
impl fmt::Display for Backtrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<backtrace unavailable>")
    }
}
''')
p.write_text(s)
print("  error.rs prep complete")
PYEOF
fi

echo "prep.sh complete: $SRC"
