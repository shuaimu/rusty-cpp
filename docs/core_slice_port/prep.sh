#!/usr/bin/env bash
# Pre-process rustc's core/src/slice/*.rs for transpilation.
#
# - `crate::` → `std::` so the transpiler resolves imports against rusty.cppm.
# - `assert_unsafe_precondition!` (multi-line, rustc-internal grammar)
#   → `();` no-op (handled via Python).
# - Various derive_const → derive rewrites.
#
# Run BEFORE collapse.py.
set -euo pipefail
SRC_DIR="${1:?usage: prep.sh <src_dir>}"
[[ -d "$SRC_DIR" ]] || { echo "error: $SRC_DIR is not a directory" >&2; exit 1; }

for f in "$SRC_DIR"/*.rs; do
  sed -i \
    -e 's|\bcrate::|std::|g' \
    -e 's|#\[derive_const(\([^)]*\))\]|#[derive(\1)] // derive_const → derive|g' \
    -e 's|let check_mask = #\[cold\]|let check_mask =|g' \
    -e '/^use std::slice::memchr/d' \
    -e '/^use std::ub_checks/d' \
    -e '/^use std::intrinsics/d' \
    -e 's|#\[rustc_diagnostic_item = "[^"]*"\]||g' \
    -e 's|#\[rustc_const_unstable[^]]*\]||g' \
    -e 's|#\[rustc_allow_const_fn_unstable[^]]*\]||g' \
    -e '/^pub mod sort;/d' \
    -e '/^pub use sort::/d' \
    -e '/^pub use std::slice::sort::/d' \
    -e 's|impl const |impl |g' \
    -e 's|default impl |impl |g' \
    -e 's|default fn |fn |g' \
    -e 's|default unsafe fn |unsafe fn |g' \
    -e 's|\[const\] ||g' \
    -e 's| const Destruct\b||g' \
    -e 's|^const trait |trait |g' \
    -e 's|^const unsafe trait |unsafe trait |g' \
    -e 's|^const pub trait |pub trait |g' \
    -e 's|^pub const trait |pub trait |g' \
    -e 's|^pub const unsafe trait |pub unsafe trait |g' \
    -e 's|\(impl<[^>]*>\) const |\1 |g' \
    -e 's|\(unsafe impl<[^>]*>\) const |\1 |g' \
    -e 's|#\[rustc_intrinsic\]||g' \
    -e 's|#\[rustc_inherit_overflow_checks\]||g' \
    -e 's|#\[rustc_no_implicit_autorefs\]||g' \
    -e 's|#\[rustc_specialization_trait\]||g' \
    -e 's|#\[rustc_unsafe_specialization_marker\]||g' \
    -e 's|#\[rustc_skip_during_method_dispatch[^]]*\]||g' \
    "$f"
done

# Inline iter/macros.rs (referenced via `macro_use` from iter.rs).
# Order is constrained: macro_rules! must be defined before its
# invocation. So macros.rs body comes first. But iter.rs's `use`
# statements need to be at the top of the combined file so collapse's
# split_uses can extract them; otherwise duplicate-import errors leak
# through.
#
# Strategy:
#   1. extract iter.rs's leading `use` lines.
#   2. write iter.rs = [iter uses] + [macros body] + [iter remainder].
if [[ -f "$SRC_DIR/iter/macros.rs" ]] && [[ -f "$SRC_DIR/iter.rs" ]]; then
  python3 - "$SRC_DIR" <<'PY'
import re
import sys
from pathlib import Path

src = Path(sys.argv[1])
iter_text = (src / "iter.rs").read_text()
macros_text = (src / "iter/macros.rs").read_text()

# Walk iter_text head, lifting `use ... ;` (single-line + multi-line braced).
lines = iter_text.splitlines(keepends=True)
uses: list[str] = []
i = 0
n = len(lines)
in_use = False
use_buf = ""
while i < n:
    ln = lines[i]
    stripped = ln.strip()
    if in_use:
        use_buf += ln
        if stripped.endswith(";"):
            uses.append(use_buf)
            use_buf = ""
            in_use = False
        i += 1
        continue
    if not stripped or stripped.startswith("//") or stripped.startswith("/*"):
        i += 1
        continue
    if stripped.startswith("//!"):
        i += 1
        continue
    if stripped.startswith("#"):
        i += 1
        continue
    if stripped.startswith("mod ") and stripped.endswith(";"):
        # Skip `mod X;` declaration (e.g. iter.rs's `mod macros;`).
        i += 1
        continue
    if stripped.startswith("use "):
        if stripped.endswith(";"):
            uses.append(ln)
            i += 1
            continue
        else:
            use_buf = ln
            in_use = True
            i += 1
            continue
    break
remainder = "".join(lines[i:])

(src / "iter.rs").write_text(
    "".join(uses) + "\n" + macros_text + "\n" + remainder
)
PY
fi
rm -rf "$SRC_DIR/iter"

# Drop sort subdirectory entirely — not needed for unblocking str/string.
rm -rf "$SRC_DIR/sort"

# Multi-line macro stripping via Python — handles invocations whose
# arguments span multiple lines / contain nested parens. Each macro is
# stripped to `();` so the surrounding statement sequence stays valid.
python3 - "$SRC_DIR" <<'PY'
import re
import sys
from pathlib import Path

# Macros to strip entirely (rustc-internal, no analogue).
# `assert_unsafe_precondition!` — debug-only safety check.
# `const_eval_select!` — chooses between ct/rt impl, no analogue.
EXPR_MACROS = [
    "assert_unsafe_precondition",
    "const_eval_select",
]

src_dir = Path(sys.argv[1])
for f in src_dir.glob("*.rs"):
    text = f.read_text()
    for mac in EXPR_MACROS:
        out = []
        i = 0
        # Match optional `path::path::` prefix so `ub_checks::assert_unsafe_precondition!`
        # collapses to `();` rather than leaving the prefix behind.
        pat = re.compile(rf"(?:\b\w+::)*\b{mac}!\(")
        while True:
            m = pat.search(text, i)
            if not m:
                out.append(text[i:])
                break
            out.append(text[i : m.start()])
            j = m.end()
            depth = 1
            while j < len(text) and depth > 0:
                ch = text[j]
                if ch == "(":
                    depth += 1
                elif ch == ")":
                    depth -= 1
                j += 1
            if j < len(text) and text[j] == ";":
                j += 1
            out.append("();")
            i = j
        text = "".join(out)
    # impl_fn_for_zst! { ... } takes braces, not parens — strip the whole block.
    out = []
    i = 0
    while True:
        m = re.search(r"\bimpl_fn_for_zst!\s*\{", text[i:])
        if not m:
            out.append(text[i:])
            break
        out.append(text[i : i + m.start()])
        j = i + m.end()
        depth = 1
        while j < len(text) and depth > 0:
            ch = text[j]
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
            j += 1
        out.append("// impl_fn_for_zst! { ... } stripped\n")
        i = j
    text = "".join(out)
    f.write_text(text)
PY

echo "[core_slice_port prep] normalized $SRC_DIR"
