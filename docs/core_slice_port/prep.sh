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
    -e 's|#\[rustc_intrinsic\]||g' \
    -e 's|#\[rustc_inherit_overflow_checks\]||g' \
    -e 's|#\[rustc_no_implicit_autorefs\]||g' \
    -e 's|#\[rustc_specialization_trait\]||g' \
    -e 's|#\[rustc_unsafe_specialization_marker\]||g' \
    -e 's|#\[rustc_skip_during_method_dispatch[^]]*\]||g' \
    "$f"
done

# Inline iter/macros.rs (referenced via `macro_use` from iter.rs).
# After inlining the macros file at the top of iter.rs, transpile sees
# them as already-defined.
if [[ -f "$SRC_DIR/iter/macros.rs" ]]; then
  if [[ -f "$SRC_DIR/iter.rs" ]]; then
    # Prepend macros body (minus its uses) to iter.rs.
    cat "$SRC_DIR/iter/macros.rs" "$SRC_DIR/iter.rs" > "$SRC_DIR/iter.rs.new"
    mv "$SRC_DIR/iter.rs.new" "$SRC_DIR/iter.rs"
  fi
  rm -rf "$SRC_DIR/iter"
fi

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
        while True:
            m = re.search(rf"\b{mac}!\(", text[i:])
            if not m:
                out.append(text[i:])
                break
            out.append(text[i : i + m.start()])
            j = i + m.end()
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
