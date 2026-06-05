#!/usr/bin/env python3
"""Collapse vendored core::slice submodules into a single lib.rs.

Rust source tree for `library/core/src/slice/` is a parent `mod.rs`
plus eight submodule files. Each Rust module would become its own
`.cppm` partition, but they reference each other heavily (e.g.
`super::SliceIndex`, `super::Iter`) — which forms an import cycle
that C++20 modules forbid.

This pre-processing step flattens the Rust module tree: every
submodule body is hoisted into `lib.rs` (the renamed `mod.rs`),
`super::X` becomes `X`, `pub(super)` becomes `pub(crate)`, and the
submodule files are deleted. The transpiler then emits a single
`core_slice_port.cppm`.

Run AFTER `prep.sh` (which normalizes external use-paths).

Usage:
    python3 collapse.py <src_dir>
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# Order matters: declarations (specialize/memchr/raw) before users
# (cmp/index/iter/ascii). Iter is referenced by many others so put it
# after low-level deps.
SUBMODULES = [
    "specialize",
    "memchr",
    "raw",
    "rotate",
    "cmp",
    "index",
    "iter",
    "ascii",
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
    """Rewrite intra-slice paths so they resolve at crate root."""
    text = re.sub(r"\bsuper::", "", text)
    text = re.sub(r"\bself::", "", text)
    text = text.replace("pub(super)", "pub(crate)")
    # Strip inner doc comments — only valid at start of module / crate
    # root; once a submodule body is inlined mid-file these break
    # rustc.
    text = re.sub(r"^[ \t]*//!.*\n", "", text, flags=re.MULTILINE)
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

    Patterns we strip:
      1. `use super::...` / `use self::...` — direct intra-module.
      2. `use std::slice::...` — after prep.sh rewrites, paths pointing
         at our own siblings (cmp/index/iter/...) would conflict with
         the collapsed bodies.
      3. `use crate::slice::...` — same; if any leaked through prep.
    """
    kept = []
    for u in uses:
        if re.search(r"\b(super|self)::", u):
            continue
        if re.search(r"\bstd::slice::", u):
            continue
        if re.search(r"\bcrate::slice::", u):
            continue
        kept.append(u)
    return kept


def parse_use(use_text: str) -> tuple[str, list[tuple[str, str | None]] | None]:
    """Parse a `use [pub] PATH;` statement.

    Returns (path_prefix, leafs_or_None). For a single-symbol form
    (`use std::cmp::Ordering;`) leafs_or_None is a single-element list
    `[("Ordering", None)]` with path_prefix `use std::cmp::`. For a
    braced form (`use std::fmt::{Formatter, Write as W};`) it's
    `[("Formatter", None), ("Write", "W")]` with path_prefix
    `use std::fmt::`.

    Returns (use_text, None) when we can't parse the shape cleanly —
    caller should keep the use as-is.
    """
    # Strip trailing `;` + whitespace and leading whitespace.
    text = use_text.strip()
    if not text.endswith(";"):
        return use_text, None
    body = text[:-1].strip()
    # Pull off optional `pub` (or `pub(crate)`).
    pub_match = re.match(r"^(pub(?:\([^)]+\))?\s+)?use\s+", body)
    if not pub_match:
        return use_text, None
    pub_prefix = pub_match.group(1) or ""
    rest = body[pub_match.end():]

    # Two forms:
    #   PATH::{A, B as C, ...}
    #   PATH::Leaf [as Alias]
    # PATH always uses `::`.
    if rest.endswith("}"):
        # Braced form. Find the matching open brace.
        open_idx = rest.rfind("{")
        if open_idx == -1:
            return use_text, None
        path = rest[:open_idx].rstrip()  # e.g. `std::fmt::`
        if not path.endswith("::"):
            return use_text, None
        inner = rest[open_idx + 1 : -1]
        # Split inner by top-level commas (no `{...}` nesting in core::str).
        leaf_strs = [s.strip() for s in inner.split(",") if s.strip()]
        leafs: list[tuple[str, str | None]] = []
        for ls in leaf_strs:
            m = re.match(r"^([\w:]+)(?:\s+as\s+(\w+))?$", ls)
            if not m:
                # Self-import (`self`) or nested braces — bail out.
                return use_text, None
            leafs.append((m.group(1), m.group(2)))
        # path here is `use std::fmt::`; recombine with pub_prefix.
        return pub_prefix + "use " + path, leafs
    else:
        # Single-symbol form: `PATH::Leaf [as Alias]`.
        m = re.match(r"^(.+?::)([\w]+)(?:\s+as\s+(\w+))?$", rest)
        if not m:
            return use_text, None
        path = m.group(1)
        leaf = m.group(2)
        alias = m.group(3)
        return pub_prefix + "use " + path, [(leaf, alias)]


def _leaf_key(leaf: tuple[str, str | None], path_tail: str | None = None) -> str:
    """Dedup key — the bound name (alias if present, else symbol).

    Special case: `use path::Last::{self, ...};` binds the LAST segment
    of the path as the symbol, not `self`. Caller passes `path_tail`
    (the last `::Foo` segment of the use's path prefix) so we resolve
    `self` to it.
    """
    if leaf[1] is not None:
        return leaf[1]
    if leaf[0] == "self" and path_tail is not None:
        return path_tail
    return leaf[0]


def _format_leaf(leaf: tuple[str, str | None]) -> str:
    if leaf[1] is None:
        return leaf[0]
    return f"{leaf[0]} as {leaf[1]}"


def dedup_uses(uses: list[str]) -> list[str]:
    """Dedup use-statements by tracked bound names.

    Multi-leaf braced uses are REWRITTEN to drop leafs whose bound
    names are already seen. Single-leaf uses whose bound name is
    already seen are dropped entirely.

    Use-statements we can't parse are kept as-is — better to fail
    later at rustc than to mangle them silently.
    """
    seen: set[str] = set()
    out: list[str] = []
    for u in uses:
        prefix, leafs = parse_use(u)
        if leafs is None:
            for sym in re.findall(r"\b([A-Z][A-Za-z0-9_]*)\b", u):
                seen.add(sym)
            out.append(u)
            continue
        # Path tail (last `::Foo` of the prefix) — used when a brace
        # contains `self`, which binds the path tail rather than literal
        # "self". e.g. `use std::cmp::Ordering::{self, Equal};` binds
        # `Ordering`, `Equal`, `Greater`, `Less`.
        path_tail = None
        m_tail = re.search(r"::([\w]+)::$", prefix)
        if m_tail:
            path_tail = m_tail.group(1)
        kept = [l for l in leafs if _leaf_key(l, path_tail) not in seen]
        if not kept:
            continue
        for l in kept:
            seen.add(_leaf_key(l, path_tail))
        if len(kept) == 1 and prefix.endswith("::"):
            out.append(f"{prefix}{_format_leaf(kept[0])};")
        else:
            inner = ", ".join(_format_leaf(l) for l in kept)
            out.append(f"{prefix}{{{inner}}};")
    return out


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

    all_uses = dedup_uses(sorted(set(lib_uses + sub_uses_combined)))

    out: list[str] = []
    out.append("//! Collapsed core::slice module — single-file flattening of\n")
    out.append("//! `library/core/src/slice/*.rs`. See docs/core_slice_port/collapse.py.\n")
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
