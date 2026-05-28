#!/usr/bin/env bash
# Pre-process a vendored copy of hashbrown's `src/` for transpilation
# with rusty-cpp-transpiler.
#
# Strategy mirrors `docs/vec_port/prep.sh` and `docs/btreemap_port/prep.sh`:
# strip what the transpiler can't parse (nightly features, target-cfg
# specialization, attribute soup), then rewrite paths the std-mapping
# table doesn't resolve. SIMD probing falls back to the `generic` group
# implementation so the first transpile doesn't need to deal with
# `core::arch::x86_64::*` intrinsics.
#
# Usage:
#   bash prep.sh <hashbrown_src_dir>
#
# Idempotent — safe to re-run.

set -euo pipefail

SRC="${1:?usage: prep.sh <hashbrown_src_dir>}"
if [[ ! -d "$SRC" ]]; then
  echo "error: $SRC is not a directory" >&2
  exit 1
fi

# --- 1. Strip nightly feature gates & attribute soup at crate root ---
# The transpiler's syn parser chokes on stacked cfg_attr feature() blocks.
if [[ -f "$SRC/lib.rs" ]]; then
  python3 - "$SRC/lib.rs" <<'PYEOF'
import sys, pathlib, re
p = pathlib.Path(sys.argv[1])
s = p.read_text()
# Strip the entire #![cfg_attr(...)] block at the top of lib.rs.
# These are crate-level feature attributes — never affect generated code.
s = re.sub(
    r'^#!\[cfg_attr\([^\]]*\][^\n]*\n(?:\s*[^#\n]*\n)*',
    '',
    s,
    flags=re.MULTILINE,
)
# Also strip standalone #![no_std], #![allow(...)], etc.
s = re.sub(r'^#!\[(?:no_std|allow|warn|deny|expect|forbid)\b[^\]]*\]\s*\n', '', s, flags=re.MULTILINE)
p.write_text(s)
PYEOF
fi

# --- 2. cfg_if! collapse — force generic group impl ---
# The transpiler can't evaluate cfg conditions; cfg_if! macro_rules will
# emit all branches. Replace the `control/group/mod.rs` selection with a
# plain `mod generic; use generic as imp;` (no SIMD), and remove the
# cfg_if! block in `control/group/generic.rs` (pick u64 GroupWord).
cat > "$SRC/control/group/mod.rs" <<'EOF'
// Pre-processed by docs/hashbrown_port/prep.sh — cfg_if! collapsed to
// the generic word-size impl, no SIMD intrinsics needed for the first
// transpile pass.
mod generic;
use generic as imp;
pub(crate) use self::imp::Group;
pub(super) use self::imp::{BITMASK_ITER_MASK, BITMASK_STRIDE, BitMaskWord, NonZeroBitMaskWord};
EOF

# In generic.rs, hand-collapse the cfg_if to the 64-bit branch.
if [[ -f "$SRC/control/group/generic.rs" ]]; then
  python3 - "$SRC/control/group/generic.rs" <<'PYEOF'
import sys, pathlib
p = pathlib.Path(sys.argv[1])
s = p.read_text()
old = """cfg_if! {
    if #[cfg(any(
        target_pointer_width = "64",
        target_arch = "aarch64",
        target_arch = "x86_64",
        target_arch = "wasm32",
    ))] {
        type GroupWord = u64;
        type NonZeroGroupWord = core::num::NonZeroU64;
    } else {
        type GroupWord = u32;
        type NonZeroGroupWord = core::num::NonZeroU32;
    }
}"""
new = """type GroupWord = u64;
type NonZeroGroupWord = core::num::NonZeroU64;"""
if old in s:
    s = s.replace(old, new)
    p.write_text(s)
    print("  collapsed cfg_if in control/group/generic.rs")
else:
    print("  generic.rs cfg_if already collapsed (idempotent)")
PYEOF
fi

# Strip the sse2/neon/lsx files since we forced the generic path; they
# contain core::arch intrinsics the transpiler doesn't speak yet.
rm -f "$SRC/control/group/sse2.rs" "$SRC/control/group/neon.rs" "$SRC/control/group/lsx.rs"

# --- 3. cfg_if! in macros.rs — leave as-is; the transpiler may treat
#       it as a Rust-only macro_rules and skip it. We'll see.

# --- 4. Path rewrites — hashbrown uses some paths that don't match
#       the transpiler's std-mapping table.
# `crate::alloc` is hashbrown's own module (src/alloc.rs); leave it.
# `stdalloc::*` refers to `extern crate alloc as stdalloc` — rewrite to
# the canonical `alloc::` path the transpiler expects.
find "$SRC" -name "*.rs" -exec sed -i \
  -e 's|stdalloc::|alloc::|g' \
  {} \;

# --- 5. equivalent crate ---
# hashbrown re-exports `equivalent::Equivalent`. The actual `Equivalent`
# trait lives in the `equivalent` crate (external dep). We hand-roll a
# minimal version of it under hashbrown's own namespace so the
# transpiler doesn't need an external crate lookup.
if [[ -f "$SRC/lib.rs" ]]; then
  python3 - "$SRC/lib.rs" <<'PYEOF'
import sys, pathlib
p = pathlib.Path(sys.argv[1])
s = p.read_text()
old = "pub use equivalent::Equivalent;"
new = """// Hand-rolled minimal `Equivalent` trait (the `equivalent` crate is
// external; inlining its definition avoids the cross-crate dep for
// the port).
pub trait Equivalent<K: ?Sized> {
    fn equivalent(&self, key: &K) -> bool;
}
impl<Q: ?Sized, K: ?Sized> Equivalent<K> for Q
where
    Q: Eq,
    K: core::borrow::Borrow<Q>,
{
    #[inline]
    fn equivalent(&self, key: &K) -> bool {
        PartialEq::eq(self, key.borrow())
    }
}"""
if old in s:
    s = s.replace(old, new)
    p.write_text(s)
    print("  inlined `Equivalent` trait")
else:
    print("  `Equivalent` already inlined (idempotent)")
PYEOF
fi

# --- 6. Strip nightly-only `default_fn!` macro calls ---
# The crate uses `default_fn!` to gate `default fn` (specialization)
# under `feature = "nightly"`. With nightly off, it's a no-op pass-
# through wrapper around the fn. Strip the wrapper:
#   default_fn! { fn foo(...) { ... } }   →   fn foo(...) { ... }
# (Just deleting the `default_fn! {` opener would leave a stray `{`
# inside the surrounding impl block.)
find "$SRC" -name "*.rs" -print0 | while IFS= read -r -d '' f; do
  python3 - "$f" <<'PYEOF'
import sys, pathlib, re
p = pathlib.Path(sys.argv[1])
s = p.read_text()
out = []
i = 0
changed = False
while True:
    m = re.search(r'default_fn!\s*\{', s)
    if not m:
        break
    start = m.start()
    inner_start = m.end()
    # Brace-match to find the closing `}`.
    depth = 1
    j = inner_start
    while j < len(s) and depth > 0:
        c = s[j]
        if c == '{':
            depth += 1
        elif c == '}':
            depth -= 1
            if depth == 0:
                break
        elif c == '/' and j+1 < len(s) and s[j+1] == '/':
            while j < len(s) and s[j] != '\n':
                j += 1
            continue
        elif c == '"':
            j += 1
            while j < len(s) and s[j] != '"':
                if s[j] == '\\':
                    j += 1
                j += 1
        j += 1
    if depth != 0:
        break
    inner = s[inner_start:j]
    s = s[:start] + inner + s[j+1:]
    changed = True
if changed:
    p.write_text(s)
PYEOF
done

# --- 7. Strip rustc-internal attributes the transpiler doesn't grok ---
find "$SRC" -name "*.rs" -exec sed -i \
  -e '/^#!\[stable(/d' \
  -e '/^#!\[unstable(/d' \
  -e '/^#!\[rustc_/d' \
  {} \;

# --- 7.5. alloc.rs: strip the allocator_api2 cfg branch ---
# alloc.rs has two `mod inner` blocks gated on cfg features. The
# allocator-api2 one re-exports an external crate we don't have;
# the other hand-rolls its own Allocator trait. Keep only the
# latter so we get exactly one definition.
if [[ -f "$SRC/alloc.rs" ]]; then
  python3 - "$SRC/alloc.rs" <<'PYEOF'
import sys, pathlib, re
p = pathlib.Path(sys.argv[1])
s = p.read_text()
# Drop the allocator-api2 branch entirely. Match the cfg attribute
# plus its mod inner { ... } body (brace-matched).
m = re.search(r'#\[cfg\(all\(not\(feature\s*=\s*"nightly"\),\s*feature\s*=\s*"allocator-api2"\)\)\]\s*\n\s*mod\s+inner\s*\{', s)
if m:
    depth = 1
    j = m.end()
    while j < len(s) and depth > 0:
        if s[j] == '{':
            depth += 1
        elif s[j] == '}':
            depth -= 1
        j += 1
    s = s[:m.start()] + s[j:]
    s = re.sub(r'#\[cfg\(not\(any\(feature\s*=\s*"nightly",\s*feature\s*=\s*"allocator-api2"\)\)\)\]\s*\n', '', s)
# Strip the #[cfg(test)] re-export for AllocError (test-only;
# the non-feature inner mod doesn't define AllocError).
s = re.sub(r'(^|\n)\s*#\[cfg\(test\)\]\s*\n\s*pub\(crate\) use self::inner::AllocError;\s*\n', r'\1', s)
p.write_text(s)
print("  stripped allocator-api2 cfg branch from alloc.rs")
PYEOF
fi

# --- 8. Strip nightly-gated items + #[may_dangle] etc. ---
# The transpiler can't evaluate cfg conditions; nightly-gated impl
# blocks (TrivialClone, may_dangle Drop) just need to go entirely so
# the parser sees only the stable-Rust path.
find "$SRC" -name "*.rs" -print0 | while IFS= read -r -d '' f; do
  python3 - "$f" <<'PYEOF'
import sys, pathlib, re
p = pathlib.Path(sys.argv[1])
src = p.read_text()
out = []
i = 0
n = len(src)
changed = False

def find_item_end(text, start):
    """Find end position of the item starting at `start`.
    Items end either with `;` (use, type alias, unit struct) or
    with `}` at depth 0 (impl/fn/struct/enum bodies).
    Returns index past the terminator."""
    depth = 0
    j = start
    while j < len(text):
        c = text[j]
        if c == '{':
            depth += 1
            j += 1
            continue
        if c == '}':
            depth -= 1
            j += 1
            if depth == 0:
                # The body just closed. Some items have a trailing `;`
                # (e.g. `use foo::{a, b};` — the closing `}` is not the
                # item end). Peek ahead past whitespace; if a `;` is
                # next, consume it too. Otherwise the item ends here.
                k = j
                while k < len(text) and text[k] in ' \t':
                    k += 1
                if k < len(text) and text[k] == ';':
                    return k + 1
                return j
            continue
        if c == ';' and depth == 0:
            return j + 1
        if c == '/' and j+1 < len(text) and text[j+1] == '/':
            while j < len(text) and text[j] != '\n':
                j += 1
            continue
        if c == '/' and j+1 < len(text) and text[j+1] == '*':
            j += 2
            while j+1 < len(text) and not (text[j] == '*' and text[j+1] == '/'):
                j += 1
            j += 2
            continue
        if c == '"':
            j += 1
            while j < len(text) and text[j] != '"':
                if text[j] == '\\':
                    j += 1
                j += 1
            j += 1
            continue
        j += 1
    return -1

# Match #[cfg(feature = "nightly")] at line start, followed by item.
pattern = re.compile(r'^#\[cfg\(feature\s*=\s*"nightly"\)\]\s*\n', re.MULTILINE)
while True:
    m = pattern.search(src)
    if not m:
        break
    # Find end of the following item
    item_start = m.end()
    item_end = find_item_end(src, item_start)
    if item_end == -1:
        break
    src = src[:m.start()] + src[item_end:]
    # Eat one trailing blank line if present
    if src[m.start():m.start()+1] == '\n':
        src = src[:m.start()] + src[m.start()+1:]
    changed = True

# Strip the `#[cfg(not(feature = "nightly"))]` attribute (keep item).
src, k = re.subn(r'^#\[cfg\(not\(feature\s*=\s*"nightly"\)\)\]\s*\n', '', src, flags=re.MULTILINE)
if k:
    changed = True

# Strip `#[may_dangle]` and `#[cfg_attr(feature = "inline-more", inline)]`.
src, k1 = re.subn(r'#\[may_dangle\]\s*', '', src)
src, k2 = re.subn(r'#\[cfg_attr\(feature\s*=\s*"inline-more",\s*inline\)\]\s*\n?', '', src)
if k1 or k2:
    changed = True

if changed:
    p.write_text(src)
PYEOF
done

echo "prep.sh complete: $SRC"
echo "(idempotent — safe to re-run)"
