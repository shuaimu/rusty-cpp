#!/usr/bin/env python3
"""Collapse vendored vec_deque submodules into a single lib.rs.

Rust source tree for `library/alloc/src/collections/vec_deque/` is a parent
`mod.rs` plus eight submodule files. When transpiled to C++20 modules each
Rust module becomes its own `.cppm` partition; the umbrella imports the
submodules and the submodules reference `VecDeque` back, which forms an
import cycle that C++20 modules forbid.

This pre-processing step flattens the Rust module tree: every submodule
body is hoisted into `lib.rs` (the renamed `mod.rs`), `super::X` becomes
`X`, `pub(super)` becomes `pub(crate)`, and the submodule files are
deleted. The transpiler then emits a single `vec_deque_port.cppm`.

Run AFTER `prep.sh` (which normalizes external use-paths).

Usage:
    python3 collapse.py <src_dir>
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# Order matters for macros (macro_rules! must be declared before use).
SUBMODULES = [
    "macros",
    "iter",
    "iter_mut",
    "drain",
    "into_iter",
    "extract_if",
    "spec_extend",
    "spec_from_iter",
    "splice",
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
    """Rewrite intra-vec_deque paths so they resolve at crate root."""
    text = re.sub(r"\bsuper::", "", text)
    text = re.sub(r"\bself::", "", text)
    text = text.replace("pub(super)", "pub(crate)")
    return text


def filter_uses(uses: list[str]) -> list[str]:
    """Drop intra-module imports.

    Three patterns we strip:
      1. `use super::...` / `use self::...` — direct intra-module.
      2. `use std::collections::vec_deque::...` — prep.sh rewrote
         `crate::collections::vec_deque::...` into this; the path
         now points back at our own module (vec_deque's siblings)
         which after collapse live at crate root, so the `use` is
         both redundant and conflicts with the local definition.
      3. `use std::vec_deque::...` — same root cause, different
         rewrite shape.
    """
    kept = []
    for u in uses:
        if re.search(r"\b(super|self)::", u):
            continue
        if re.search(r"\bstd::collections::vec_deque::", u):
            continue
        if re.search(r"\bstd::vec_deque::", u):
            continue
        kept.append(u)
    return kept


def main(src_dir: Path) -> int:
    lib_path = src_dir / "lib.rs"
    if not lib_path.exists():
        print(f"error: {lib_path} not found", file=sys.stderr)
        return 1

    # ---- 1. read submodules, hoist uses, normalize paths --------------
    sub_uses_combined: list[str] = []
    sub_bodies: dict[str, str] = {}

    for name in SUBMODULES:
        sub_path = src_dir / f"{name}.rs"
        if not sub_path.exists():
            continue
        raw = sub_path.read_text()
        uses, body = split_uses(raw)
        uses = filter_uses(uses)
        uses = [normalize_paths(u) for u in uses]
        body = normalize_paths(body)
        sub_uses_combined.extend(uses)
        sub_bodies[name] = body
        sub_path.unlink()

    # ---- 2. process lib.rs --------------------------------------------
    lib_raw = lib_path.read_text()

    # Strip `mod X;` declarations for our submodules (plus `mod tests;`).
    # Also strip preceding attribute lines that decorate the mod decl.
    mod_pattern = re.compile(
        r"(?:^[ \t]*#\[(?:cfg|macro_use|allow)[^\]]*\]\s*\n)*"
        r"^[ \t]*mod (?:" + "|".join(SUBMODULES + ["tests"]) + r")\s*;\s*\n",
        flags=re.MULTILINE,
    )
    lib_raw = mod_pattern.sub("", lib_raw)

    # Strip `pub use self::X::...;` / `use self::X::...;` re-exports.
    # These also have optional preceding attrs.
    use_pattern = re.compile(
        r"(?:^[ \t]*#\[[^\]]*\]\s*\n)*"
        r"^[ \t]*(?:pub )?use self::(?:"
        + "|".join(SUBMODULES)
        + r")::[^;]+;\s*\n",
        flags=re.MULTILINE,
    )
    lib_raw = use_pattern.sub("", lib_raw)

    lib_uses, lib_body = split_uses(lib_raw)
    lib_uses = filter_uses(lib_uses)
    lib_body = normalize_paths(lib_body)

    # ---- 3. dedupe + merge uses ---------------------------------------
    all_uses = sorted(set(lib_uses + sub_uses_combined))

    # ---- 4. compose final lib.rs --------------------------------------
    out: list[str] = []
    out.append("//! Collapsed vec_deque module — single-file flattening of\n")
    out.append("//! `library/alloc/src/collections/vec_deque/*.rs`.\n")
    out.append("//! See docs/vec_deque_port/collapse.py for rationale.\n")
    out.append("\n")
    out.append("#![stable(feature = \"rust1\", since = \"1.0.0\")]\n\n")
    out.append("\n".join(all_uses))
    out.append("\n\n")

    # Macros first — `macro_rules!` is order-dependent.
    if "macros" in sub_bodies:
        out.append("// ===== collapsed from macros.rs =====\n")
        out.append(sub_bodies["macros"].rstrip())
        out.append("\n\n")

    out.append("// ===== body of original mod.rs =====\n")
    out.append(lib_body.rstrip())
    out.append("\n\n")

    for name in SUBMODULES:
        if name == "macros":
            continue
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
