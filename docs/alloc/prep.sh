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
# ============================================================================
# boxed slice strips (each reported by the widening probe):
#   X1  mod thin / ThinBox        -> nightly thin-pointer Box (ptr::metadata)
#   X2  Coroutine impls           -> nightly coroutine machinery
#   X3  impl Error for Box<E>     -> error::Request lives in the std (rusty)
#                                    module, not alloc
# ============================================================================
BOXED="$SRC/boxed.rs"
if [[ -f "$BOXED" ]]; then
  # Consumer decouple: linked_list/rc/sync/btree/vec/raw_vec keep binding the
  # RUNTIME rusty::Box (their pre-fold, runtime-green emission). Crate-Box is
  # exercised by the boxed module itself + test_alloc.cpp. Flipping consumers
  # to crate-Box is a later, module-by-module migration.
  grep -rl "use alloc::boxed::Box;" "$SRC" --include="*.rs" | grep -v "/boxed" |     xargs -r sed -i 's/^use alloc::boxed::Box;$//'

  python3 - "$BOXED" "$SRC/boxed/convert.rs" "$SRC/boxed/iter.rs" "$SRC/vec/mod.rs" <<'PYB'
import sys, pathlib, re

def drop_fn(s, name, label):
    """Remove `pub fn NAME...` through its balanced close, including the
    attribute/doc block directly above it."""
    m = re.search(rf"\n(    )pub fn {name}[<(]", s)
    if not m:
        print(f"  (fn {label}: not found, skipped)")
        return s
    start = m.start() + 1
    # walk back over attribute/doc lines
    lines_before = s[:start].split("\n")
    k = len(lines_before) - 1
    while k > 0 and (lines_before[k-1].lstrip().startswith("#[")
                     or lines_before[k-1].lstrip().startswith("///")
                     or lines_before[k-1].lstrip().startswith("//!")
                     or lines_before[k-1].strip() == ""):
        if lines_before[k-1].strip() == "" and not (k > 1 and (lines_before[k-2].lstrip().startswith("#[") or lines_before[k-2].lstrip().startswith("///"))):
            break
        k -= 1
    start = len("\n".join(lines_before[:k])) + (1 if k > 0 else 0)
    depth = 0; seen = False; idx = m.end()
    while idx < len(s):
        c = s[idx]
        if c == "{": depth += 1; seen = True
        elif c == "}":
            depth -= 1
            if seen and depth == 0: break
        idx += 1
    print(f"  drop fn {label}")
    return s[:start] + s[idx+1:]

def drop_impl(s, header_sub, label):
    i = s.find(header_sub)
    if i == -1:
        print(f"  (impl {label}: not found, skipped)")
        return s
    # back up over attributes/docs
    line_start = s.rfind("\n", 0, i) + 1
    k = line_start
    while True:
        prev_end = s.rfind("\n", 0, k - 1) + 1 if k > 0 else 0
        prev = s[prev_end:k]
        if prev.lstrip().startswith("#[") or prev.lstrip().startswith("///"):
            k = prev_end
        else:
            break
    depth = 0; seen = False; idx = i
    while idx < len(s):
        c = s[idx]
        if c == "{": depth += 1; seen = True
        elif c == "}":
            depth -= 1
            if seen and depth == 0: break
        idx += 1
    print(f"  drop impl {label}")
    return s[:k] + s[idx+1:]

boxed = pathlib.Path(sys.argv[1]); s = boxed.read_text()
# X0: dangling turbofish (all bind NonNull<MaybeUninit<T>>)
s = s.replace("NonNull::dangling()", "NonNull::<mem::MaybeUninit<T>>::dangling()")
# X6: de-sugar SizedTypeProperties::LAYOUT + write_via_move intrinsic to plain Rust
s = s.replace("<T as SizedTypeProperties>::LAYOUT", "Layout::new::<T>()")
s = s.replace("core::intrinsics::write_via_move(ptr, x)", "core::ptr::write(ptr, x)")
s = s.replace("core::intrinsics::transmute_unchecked(self)",
              "{ let (raw, alloc) = Box::into_raw_with_allocator(self); Box::from_raw_in(raw as *mut T, alloc) }")
# X7: the transmute-based constructor tails assume Box is pointer-sized; the
# emitted Box is not (it carries _rusty_forgotten). Route through from_raw.
s = s.replace("unsafe { mem::transmute(ptr) }", "unsafe { Box::from_raw(ptr) }")
s = s.replace("unsafe { mem::transmute(box_new_uninit(Layout::new::<T>())) }",
              "unsafe { Box::from_raw(box_new_uninit(Layout::new::<T>()) as *mut mem::MaybeUninit<T>) }")
# X8: new_uninit_in's try_-chain references the stripped try_new_uninit_in;
# allocate directly (global allocation, allocator stored — Global-only tier).
s = s.replace("""        // NOTE: Prefer match over unwrap_or_else since closure sometimes not inlineable.
        // That would make code size bigger.
        match Box::try_new_uninit_in(alloc) {
            Ok(m) => m,
            Err(_) => handle_alloc_error(layout),
        }""",
              """        let ptr = box_new_uninit(layout) as *mut mem::MaybeUninit<T>;
        unsafe { Box::from_raw_in(ptr, alloc) }""")
# X12: Box's Deref/DerefMut are rustc lang-item magic (`&**self` bottoms out
# in the compiler); transpiled naively they self-recurse through operator*.
# Go through the Unique field directly.
s = s.replace("""    fn deref(&self) -> &T {
        &**self
    }""",
              """    fn deref(&self) -> &T {
        unsafe { &*(self.0.as_ptr()) }
    }""")
s = s.replace("""    fn deref_mut(&mut self) -> &mut T {
        &mut **self
    }""",
              """    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *(self.0.as_ptr()) }
    }""")
# X11: into_inner's `*boxed` (consuming deref) collapses to a bare move of
# the Box in emission; desugar to the explicit read+deallocate form.
s = s.replace("""    pub fn into_inner(boxed: Self) -> T {
        *boxed
    }""",
              """    pub fn into_inner(boxed: Self) -> T {
        let (raw, alloc) = Box::into_raw_with_allocator(boxed);
        unsafe {
            let value = ptr::read(raw);
            alloc.deallocate(NonNull::new_unchecked(raw as *mut u8), Layout::new::<T>());
            value
        }
    }""")
# X10: new_in's uninit chain (new_uninit_in + Box<MaybeUninit>::write +
# assume_init) instantiates recursively through the flattened MaybeUninit
# impls; allocate-write-adopt directly, mirroring Box::new.
s = s.replace("""        let mut boxed = Self::new_uninit_in(alloc);
        boxed.write(x);
        unsafe { boxed.assume_init() }""",
              """        let ptr = box_new_uninit(Layout::new::<T>()) as *mut T;
        unsafe { core::ptr::write(ptr, x) };
        unsafe { Box::from_raw_in(ptr, alloc) }""")
# X9: Clone for Box via clone_to_uninit needs per-T CloneToUninit machinery
# the runtime does not model; allocate-and-move the clone instead.
s = s.replace("""        // Pre-allocate memory to allow writing the cloned value directly.
        let mut boxed = Self::new_uninit_in(self.1.clone());
        unsafe {
            (**self).clone_to_uninit(boxed.as_mut_ptr().cast());
            boxed.assume_init()
        }""",
              """        Box::new_in((**self).clone(), self.1.clone())""")
# X1: thin module + re-export (nightly ThinBox / ptr::metadata)
s = s.replace("mod thin;\n", "")
s = s.replace("pub use thin::ThinBox;\n", "")
# X2: nightly Coroutine + Future impls
s = drop_impl(s, "impl<G: ?Sized + Coroutine<R> + Unpin, R, A: Allocator> Coroutine<R> for Box<G, A> {", "Coroutine for Box")
s = drop_impl(s, "impl<G: ?Sized + Coroutine<R>, R, A: Allocator> Coroutine<R> for Pin<Box<G, A>>", "Coroutine for Pin<Box>")
s = drop_impl(s, "impl<F: ?Sized + Future + Unpin, A: Allocator> Future for Box<F, A> {", "Future for Box")
# X3: Error impl (error::Request lives in the std module, not alloc)
s = drop_impl(s, "impl<E: Error> Error for Box<E> {", "Error for Box")
# X4: zeroed/try_new/map family (unstable allocator_api; intrinsics-heavy)
for name in ("map", "new_zeroed", "try_new", "try_new_uninit", "try_new_zeroed",
             "try_new_in", "try_new_uninit_in", "new_zeroed_in", "try_new_zeroed_in",
             "new_zeroed_slice", "try_new_uninit_slice", "try_new_zeroed_slice",
             "new_zeroed_slice_in", "try_new_uninit_slice_in", "try_new_zeroed_slice_in"):
    s = drop_fn(s, name, name)
# X5: rustc-internal vec bridge (depends on stripped Box<[T]>::into_vec)
s = drop_impl(s, "pub fn box_assume_init_into_vec_unsafe<T, const N: usize>(", "box_assume_init_into_vec_unsafe")
boxed.write_text(s)
print("  boxed.rs prep complete")

conv = pathlib.Path(sys.argv[2]); s = conv.read_text()
# dyn-Any downcast machinery + unsized slice conversions + Box<str> pieces
for hdr, label in (
    ("impl<A: Allocator> Box<dyn Any, A> {", "Box<dyn Any> downcast"),
    ("impl<A: Allocator> Box<dyn Any + Send, A> {", "Box<dyn Any+Send> downcast"),
    ("impl<A: Allocator> Box<dyn Any + Send + Sync, A> {", "Box<dyn Any+Send+Sync> downcast"),
    ("impl From<&str> for Box<str> {", "From<&str> collides with From<T> at T=string_view"),
    ("impl From<&mut str> for Box<str> {", "From<&mut str> dup of From<&str>"),
    ("impl<'a> From<&str> for Box<dyn Error + Send + Sync + 'a> {", "From<&str> for Box<dyn Error+S+S>"),
    ("impl<'a> From<&str> for Box<dyn Error + 'a> {", "From<&str> for Box<dyn Error>"),
    ("impl<'a, 'b> From<Cow<'b, str>> for Box<dyn Error + Send + Sync + 'a> {", "From<Cow> for Box<dyn Error+S+S>"),
    ("impl<'a, 'b> From<Cow<'b, str>> for Box<dyn Error + 'a> {", "From<Cow> for Box<dyn Error>"),
    ("impl From<Cow<'_, str>> for Box<str> {", "From<Cow<str>> dup of From<&str>"),
    ("impl<T, const N: usize> TryFrom<Box<[T]>> for Box<[T; N]> {", "TryFrom<Box<[T]>>"),
    ("impl<T, const N: usize> TryFrom<Vec<T>> for Box<[T; N]> {", "TryFrom<Vec>"),
):
    s = drop_impl(s, hdr, label)
conv.write_text(s)
print("  convert.rs prep complete")

it = pathlib.Path(sys.argv[3]); s = it.read_text()
s = s.replace("use std::async_iter::AsyncIterator;\n", "")
for hdr, label in (
    ("impl<S: ?Sized + AsyncIterator + Unpin> AsyncIterator for Box<S> {", "AsyncIterator for Box"),
    ("impl<I, A: Allocator> IntoIterator for Box<[I], A> {", "IntoIterator for Box<[T]> (into_vec)"),
    ("impl FromIterator<char> for Box<str> {", "FromIterator<char> for Box<str>"),
    ("impl<'a> FromIterator<&'a char> for Box<str> {", "FromIterator<&char> for Box<str>"),
    ("impl<'a> FromIterator<&'a str> for Box<str> {", "FromIterator<&str> for Box<str>"),
    ("impl FromIterator<String> for Box<str> {", "FromIterator<String> for Box<str>"),
    ("impl<A: Allocator> FromIterator<Box<str, A>> for Box<str> {", "FromIterator<Box<str>> for Box<str>"),
    ("impl<'a> FromIterator<Cow<'a, str>> for Box<str> {", "FromIterator<Cow<str>> for Box<str>"),
):
    s = drop_impl(s, hdr, label)
it.write_text(s)
print("  iter.rs prep complete")

vm = pathlib.Path(sys.argv[4]); s = vm.read_text()
# slice-box conversion: Box<[T]> has no C++ analog; with crate-Box folded in it
# emits a Vec-typed member into boxed's namespace
s = drop_impl(s, "impl<T, A: Allocator> From<Vec<T, A>> for Box<[T], A> {", "From<Vec> for Box<[T]> (vec/mod.rs)")
vm.write_text(s)
print("  vec/mod.rs boxed-prep complete")
PYB
fi

STRING_RS="$SRC/string.rs"
if [[ -f "$STRING_RS" ]]; then
  python3 - "$STRING_RS" <<'PYS'
import sys, pathlib, re

def drop_fn(s, name, label):
    m = re.search(rf"\n(    )pub fn {name}[<(]", s)
    if not m:
        print(f"  (fn {label}: not found, skipped)")
        return s
    start = m.start() + 1
    lines_before = s[:start].split("\n")
    k = len(lines_before) - 1
    while k > 0 and (lines_before[k-1].lstrip().startswith("#[")
                     or lines_before[k-1].lstrip().startswith("///")
                     or lines_before[k-1].strip() == ""):
        if lines_before[k-1].strip() == "" and not (k > 1 and (lines_before[k-2].lstrip().startswith("#[") or lines_before[k-2].lstrip().startswith("///"))):
            break
        k -= 1
    start = len("\n".join(lines_before[:k])) + (1 if k > 0 else 0)
    depth = 0; seen = False; idx = m.end()
    while idx < len(s):
        c = s[idx]
        if c == "{": depth += 1; seen = True
        elif c == "}":
            depth -= 1
            if seen and depth == 0: break
        idx += 1
    print(f"  drop fn {label}")
    return s[:start] + s[idx+1:]

def drop_impl(s, header_sub, label):
    i = s.find(header_sub)
    if i == -1:
        print(f"  (impl {label}: not found, skipped)")
        return s
    line_start = s.rfind("\n", 0, i) + 1
    k = line_start
    while True:
        prev_end = s.rfind("\n", 0, k - 1) + 1 if k > 0 else 0
        prev = s[prev_end:k]
        if prev.lstrip().startswith("#[") or prev.lstrip().startswith("///"):
            k = prev_end
        else:
            break
    depth = 0; seen = False; idx = i
    while idx < len(s):
        c = s[idx]
        if c == "{": depth += 1; seen = True
        elif c == "}":
            depth -= 1
            if seen and depth == 0: break
        idx += 1
    print(f"  drop impl {label}")
    return s[:k] + s[idx+1:]

p = pathlib.Path(sys.argv[1]); s = p.read_text()
# S0: bare Vec ctors in String's field context — element type u8 undeduced
# (the #74 family); turbofish explicitly.
s = s.replace("String { vec: Vec::new() }", "String { vec: Vec::<u8>::new() }")
s = s.replace("String { vec: Vec::with_capacity(capacity) }",
              "String { vec: Vec::<u8>::with_capacity(capacity) }")
s = s.replace("Ok(String { vec: Vec::try_with_capacity(capacity)? })",
              "Ok(String { vec: Vec::<u8>::try_with_capacity(capacity)? })")
s = s.replace("let mut v = Vec::with_capacity(self.bytes.len());",
              "let mut v = Vec::<u8>::with_capacity(self.bytes.len());")
# S1: Pattern/Searcher machinery — the runtime has no Searcher model.
for name in ("remove_matches", "replace_first", "replace_last"):
    s = drop_fn(s, name, name)
s = drop_impl(s, "impl<'b> Pattern for &'b String {", "Pattern for &String")
s = s.replace("use std::str::pattern::{Pattern, Utf8Pattern};\n", "")
# S2: ascii::Char (nightly) surface
for hdr, label in (
    ("impl FromIterator<core::ascii::Char> for String {", "FromIterator<ascii::Char>"),
    ("impl<'a> FromIterator<&'a core::ascii::Char> for String {", "FromIterator<&ascii::Char>"),
    ("impl Extend<core::ascii::Char> for String {", "Extend<ascii::Char>"),
    ("impl<'a> Extend<&'a core::ascii::Char> for String {", "Extend<&ascii::Char>"),
    ("impl SpecToString for core::ascii::Char {", "SpecToString ascii::Char"),
    ("impl<'a> FromIterator<core::ascii::Char> for Cow<'a, str> {", "FromIterator<ascii::Char> for Cow"),
):
    s = drop_impl(s, hdr, label)
# S3: 128-bit to_string (no C++ __int128 formatting model)
s = s.replace("    i128, u128,\n", "")
# S4: Box<str> iterator impls (Box<str> has no C++ analog)
for hdr, label in (
    ("impl<A: Allocator> FromIterator<Box<str, A>> for String {", "FromIterator<Box<str>>"),
    ("impl<A: Allocator> Extend<Box<str, A>> for String {", "Extend<Box<str>>"),
):
    s = drop_impl(s, hdr, label)
def cut_range(s, start_sub, end_sub, label):
    """Cut [start of the line containing start_sub, incl. leading attrs/docs ..
    start of the line containing end_sub, incl. ITS leading attrs) ."""
    i = s.find(start_sub)
    if i == -1:
        print(f"  (range {label}: start not found, skipped)"); return s
    j = s.find(end_sub, i)
    if j == -1:
        print(f"  (range {label}: end not found, skipped)"); return s
    def back_over_attrs(idx):
        line_start = s.rfind("\n", 0, idx) + 1
        k = line_start
        while True:
            prev_end = s.rfind("\n", 0, k - 1) + 1 if k > 0 else 0
            prev = s[prev_end:k]
            ls = prev.lstrip()
            if ls.startswith("#[") or ls.startswith("///") or ls.startswith("//!") or ls.strip() == "":
                k = prev_end
                if k == 0: break
            else:
                break
        return k
    start = back_over_attrs(i)
    end = back_over_attrs(j)
    print(f"  cut range {label}")
    return s[:start] + s[end:]

# S6: impl From<String> for Vec<u8> creates a FALSE by-value cycle
# (String has a Vec<u8> field; this conversion makes the cycle-breaker box
# String's `vec` field). Strip it — the runtime test does not need it.
s = drop_impl(s, "impl From<String> for Vec<u8> {", "From<String> for Vec<u8>")
# S7: nightly UTF-16 + utf8-lossy + boxed-str surface (as_chunks/align_to/
# utf8_chunks/decode_utf16/from_boxed_utf8_unchecked — unmapped runtime).
for name in ("from_utf8_lossy", "from_utf8_lossy_owned", "from_utf16",
             "from_utf16_lossy", "from_utf16le", "from_utf16le_lossy",
             "from_utf16be", "from_utf16be_lossy", "into_boxed_str"):
    s = drop_fn(s, name, name)

# S5: the entire ToString / SpecToString tower (blanket + all primitive
# specializations + fmt::Arguments) — nightly formatting the runtime already
# provides via rusty::to_string; the emitted _fmt/ilog10/core::mem/Formatter
# machinery is unmappable.
s = cut_range(s, "pub trait ToString {", "impl AsRef<str> for String {",
              "ToString/SpecToString tower")

p.write_text(s)
print("  string.rs prep complete")
PYS
fi

# btree x boxed interplay: with the crate now DEFINING Box, the omitted
# turbofish on node.rs's two Box::new_uninit_in sites no longer resolves
# (strict-auto panic on the A slot) — spell them.
NODE_RS="$SRC/collections/btree/node.rs"
if [[ -f "$NODE_RS" && -f "$SRC/boxed.rs" ]]; then
  sed -i \
    -e 's|let mut leaf = Box::new_uninit_in(alloc);|let mut leaf = Box::<Self, A>::new_uninit_in(alloc);|' \
    -e 's|let mut node = Box::<Self, _>::new_uninit_in(alloc);|let mut node = Box::<Self, A>::new_uninit_in(alloc);|' \
    "$NODE_RS"
fi

echo "prep.sh complete: $SRC"
