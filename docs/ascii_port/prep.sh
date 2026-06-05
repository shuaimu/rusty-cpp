#!/usr/bin/env bash
# Pre-process rustc's core/src/ascii/ascii_char.rs for transpilation.
#
# - Normalise `crate::` references to `std::` so the transpiler resolves
#   imports against rusty.cppm.
# - Drop `assert_unsafe_precondition!` (no analogue in our infra).
# - Strip the `into_int_impl!` macro + invocation and the `impl
#   [AsciiChar]` slice-impl block — both use rustc-internal syntax that
#   syn (the parser used by rusty-cpp-transpiler) can't handle.
#   These methods (From<AsciiChar> for u8/u16/.../char and
#   <[AsciiChar]>::as_str / as_bytes) become hand-port responsibilities.
set -euo pipefail
SRC="${1:?usage: prep.sh <lib.rs>}"
[[ -f "$SRC" ]] || { echo "error: $SRC is not a file" >&2; exit 1; }

# Normalise imports + strip unstable attributes syn can't parse.
sed -i \
  -e 's|use crate::mem::transmute;|use std::mem::transmute;|g' \
  -e 's|use crate::{assert_unsafe_precondition, fmt};|use std::fmt;|g' \
  -e 's|#\[derive_const(\([^)]*\))\]|#[derive(\1)] // derive_const → derive (was unstable)|g' \
  "$SRC"

# Strip multi-line `assert_unsafe_precondition!( ... );` invocations.
# These have rustc-internal grammar (`(d: u8 = d) => expr` shape) inside
# them that even valid Rust syntax doesn't parse generically; the macro
# also has no runtime/transpiler analogue. Replace each invocation with
# an empty `();` no-op so the surrounding statement sequence stays valid.
python3 - "$SRC" <<'PY'
import re
import sys
path = sys.argv[1]
text = open(path).read()
out = []
i = 0
while True:
    m = re.search(r"assert_unsafe_precondition!\(", text[i:])
    if not m:
        out.append(text[i:])
        break
    out.append(text[i : i + m.start()])
    j = i + m.end()  # one past `(`
    depth = 1
    while j < len(text) and depth > 0:
        ch = text[j]
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        j += 1
    # Consume trailing `;` if present.
    if j < len(text) and text[j] == ";":
        j += 1
    out.append("();")  # statement-position no-op
    i = j
open(path, "w").write("".join(out))
PY

# Delete `macro_rules! into_int_impl { ... }` definition + invocation.
# The macro spans 14 lines (macro_rules block) plus the invocation line
# plus a blank — we anchor on the macro_rules opener and rstrip through
# the invocation `into_int_impl!(...)`.
python3 - "$SRC" <<'PY'
import re
import sys
path = sys.argv[1]
text = open(path).read()
# Drop the macro_rules! into_int_impl! block + its invocation.
text = re.sub(
    r"macro_rules! into_int_impl \{[\s\S]*?into_int_impl!\([^)]*\);\s*",
    "// into_int_impl! macro + invocation stripped (hand-port slot)\n",
    text,
    count=1,
)
# Drop the `impl [AsciiChar] { ... }` slice-impl block. Anchor on the
# `impl [AsciiChar] {` opener and consume to the matching closing brace.
m = re.search(r"impl \[AsciiChar\] \{", text)
if m:
    start = m.start()
    # Walk braces from the opening `{` after `impl [AsciiChar]`.
    i = m.end() - 1
    depth = 0
    while i < len(text):
        ch = text[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                end = i + 1
                break
        i += 1
    else:
        end = len(text)
    text = (
        text[:start]
        + "// impl [AsciiChar] { ... } stripped (hand-port slot — slice-impl syntax)\n"
        + text[end:]
    )
open(path, "w").write(text)
PY

echo "[ascii_port prep] normalized $SRC"
