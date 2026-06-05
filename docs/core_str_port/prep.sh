#!/usr/bin/env bash
# Pre-process rustc's core/src/str/*.rs for transpilation.
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
    -e 's|use crate::|use std::|g' \
    -e 's|crate::ascii::Char|std::ascii::Char|g' \
    -e 's|#\[derive_const(\([^)]*\))\]|#[derive(\1)] // derive_const → derive|g' \
    -e 's|let check_mask = #\[cold\]|let check_mask =|g' \
    "$f"
done

# Multi-line assert_unsafe_precondition! stripping via Python.
python3 - "$SRC_DIR" <<'PY'
import re
import sys
from pathlib import Path

src_dir = Path(sys.argv[1])
for f in src_dir.glob("*.rs"):
    text = f.read_text()
    out = []
    i = 0
    while True:
        m = re.search(r"assert_unsafe_precondition!\(", text[i:])
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
    new_text = "".join(out)
    if new_text != text:
        f.write_text(new_text)
PY

echo "[core_str_port prep] normalized $SRC_DIR"
