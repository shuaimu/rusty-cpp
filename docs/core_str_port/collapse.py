#!/usr/bin/env python3
"""Collapse vendored core::str submodules into a single lib.rs.

Rust source tree for `library/core/src/str/` is a parent `mod.rs` plus
eight submodule files. Each Rust module would become its own `.cppm`
partition, but they reference each other heavily (`super::Chars`,
`super::Pattern`, etc.) — which forms an import cycle that C++20
modules forbid.

This pre-processing step flattens the Rust module tree: every
submodule body is hoisted into `lib.rs` (the renamed `mod.rs`),
`super::X` becomes `X`, `pub(super)` becomes `pub(crate)`, and the
submodule files are deleted. The transpiler then emits a single
`core_str_port.cppm`.

Run AFTER `prep.sh` (which normalizes external use-paths).

Usage:
    python3 collapse.py <src_dir>
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# Order matters: declarations (count/error/validations) before users
# (iter/traits/pattern/lossy/converts). Iter is referenced by many
# others so put it after low-level deps.
SUBMODULES = [
    "error",
    "validations",
    "count",
    "converts",
    "traits",
    "pattern",
    "iter",
    "lossy",
]


def split_uses(text: str) -> tuple[list[str], str]:
    """Pull `use ...;` lines from the leading header of `text`.

    Returns (use_lines, rest). Stops at the first non-blank/non-use/non-attr
    line, so doc-comments and `#![...]` crate-level attrs are kept with
    `rest`.
    """
    lines = text.splitlines(keepends=True)
    uses: list[str] = []
    rest_start = 0
    in_use = False
    use_buf = ""
    for i, ln in enumerate(lines):
        stripped = ln.strip()
        if in_use:
            use_buf += ln
            if stripped.endswith(";"):
                uses.append(use_buf.rstrip())
                use_buf = ""
                in_use = False
            rest_start = i + 1
            continue
        if not stripped:
            rest_start = i + 1
            continue
        if (
            stripped.startswith("//")
            or stripped.startswith("/*")
            or stripped.startswith("*")
            or stripped.startswith("#!")
            or stripped.startswith("#[allow")
            or stripped.startswith("#[cfg")
        ):
            rest_start = i + 1
            continue
        if stripped.startswith("use "):
            if stripped.endswith(";"):
                uses.append(stripped)
            else:
                in_use = True
                use_buf = ln
            rest_start = i + 1
            continue
        break
    return uses, "".join(lines[rest_start:])


def normalize_paths(text: str) -> str:
    """Rewrite intra-str paths so they resolve at crate root."""
    text = re.sub(r"\bsuper::", "", text)
    text = re.sub(r"\bself::", "", text)
    text = text.replace("pub(super)", "pub(crate)")
    return text


def strip_crate_attrs(text: str) -> str:
    """Drop multi-line `#![...]` crate-level attrs from a submodule
    body (they become noise once the submodule is inlined into the
    crate root, and the multi-line form trips up split_uses).
    Uses bracket-counting to handle nested `(...)` inside the attr.
    """
    out = []
    i = 0
    while i < len(text):
        if text[i:i + 3] == "#![":
            j = i + 3
            depth = 1  # one open bracket already
            # Walk through finding matching `]`, respecting nested
            # `(...)` and `[...]`.
            paren_depth = 0
            while j < len(text):
                ch = text[j]
                if ch == "(":
                    paren_depth += 1
                elif ch == ")":
                    paren_depth -= 1
                elif ch == "[" and paren_depth == 0:
                    depth += 1
                elif ch == "]" and paren_depth == 0:
                    depth -= 1
                    if depth == 0:
                        j += 1
                        break
                j += 1
            # Skip the entire attr.
            i = j
            # Also consume trailing newline.
            if i < len(text) and text[i] == "\n":
                i += 1
            continue
        out.append(text[i])
        i += 1
    return "".join(out)


def filter_uses(uses: list[str]) -> list[str]:
    """Drop intra-module imports.

    Three patterns we strip:
      1. `use super::...` / `use self::...` — direct intra-module.
      2. `use std::str::...` / `use std::ascii::Char` — after prep.sh
         rewrites, paths pointing at our own siblings would conflict.
      3. `use crate::str::...` — same; if any leaked through prep.
    """
    kept = []
    for u in uses:
        if re.search(r"\b(super|self)::", u):
            continue
        if re.search(r"\bstd::str::(?!FromStr)", u):
            # `std::str::FromStr` is the actual stdlib trait, keep it.
            continue
        if re.search(r"\bcrate::str::", u):
            continue
        kept.append(u)
    return kept


def main(src_dir: Path) -> int:
    lib_path = src_dir / "lib.rs"
    if not lib_path.exists():
        print(f"error: {lib_path} not found", file=sys.stderr)
        return 1

    sub_uses_combined: list[str] = []
    sub_bodies: dict[str, str] = {}

    for name in SUBMODULES:
        sub_path = src_dir / f"{name}.rs"
        if not sub_path.exists():
            continue
        raw = sub_path.read_text()
        raw = strip_crate_attrs(raw)  # drop `#![unstable(...)]` etc.
        uses, body = split_uses(raw)
        uses = filter_uses(uses)
        uses = [normalize_paths(u) for u in uses]
        body = normalize_paths(body)
        sub_uses_combined.extend(uses)
        sub_bodies[name] = body
        sub_path.unlink()

    lib_raw = lib_path.read_text()

    mod_pattern = re.compile(
        r"(?:^[ \t]*#\[(?:cfg|macro_use|allow)[^\]]*\]\s*\n)*"
        r"^[ \t]*(?:pub )?mod (?:" + "|".join(SUBMODULES + ["tests"]) + r")\s*;\s*\n",
        flags=re.MULTILINE,
    )
    lib_raw = mod_pattern.sub("", lib_raw)

    use_pattern = re.compile(
        r"(?:^[ \t]*#\[[^\]]*\]\s*\n)*"
        r"^[ \t]*(?:pub )?use (?:self::)?(?:"
        + "|".join(SUBMODULES)
        + r")::[^;]+;\s*\n",
        flags=re.MULTILINE,
    )
    lib_raw = use_pattern.sub("", lib_raw)

    lib_uses, lib_body = split_uses(lib_raw)
    lib_uses = filter_uses(lib_uses)
    lib_body = normalize_paths(lib_body)

    # Dedup uses by extracting the leaf symbol(s) and dropping later
    # imports of the same symbol — collapse-then-merge can produce two
    # `use std::fmt::{... Write ...};` lines from different submodules
    # that import overlapping subsets.
    seen_leafs: set[str] = set()
    deduped: list[str] = []
    for u in sorted(set(lib_uses + sub_uses_combined)):
        # Extract braced symbols `use path::{A, B as C, D};`
        leafs = re.findall(r"\b([A-Z][A-Za-z0-9_]*)(?:\s+as\s+\w+)?\s*[,}]", u)
        # And the tail symbol for `use path::Foo;` form.
        m = re.match(r"\s*(?:pub\s+)?use\s+[\w:]+::([A-Z][A-Za-z0-9_]*)\s*(?:as\s+\w+)?\s*;", u)
        if m:
            leafs.append(m.group(1))
        # If ALL its leafs are already seen, drop the whole use.
        new_leafs = [s for s in leafs if s not in seen_leafs]
        if leafs and not new_leafs:
            continue
        for s in leafs:
            seen_leafs.add(s)
        deduped.append(u)
    all_uses = deduped

    out: list[str] = []
    out.append("//! Collapsed core::str module — single-file flattening of\n")
    out.append("//! `library/core/src/str/*.rs`. See docs/core_str_port/collapse.py.\n")
    out.append("\n")
    out.append("\n".join(all_uses))
    out.append("\n\n")

    out.append("// ===== body of original mod.rs =====\n")
    out.append(lib_body.rstrip())
    out.append("\n\n")

    for name in SUBMODULES:
        if name not in sub_bodies:
            continue
        out.append(f"// ===== collapsed from {name}.rs =====\n")
        out.append(sub_bodies[name].rstrip())
        out.append("\n\n")

    lib_path.write_text("".join(out))
    print(f"[collapse] flattened {len(sub_bodies)} submodules into {lib_path}")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        sys.exit(2)
    sys.exit(main(Path(sys.argv[1])))
