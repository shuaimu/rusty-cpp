#!/usr/bin/env python3
"""Hoist fn-local structs/enums/impls/statics to module level.

The transpiler skips impl blocks in local scope ("Rust-only nested impl
block skipped") and emits derives as friend functions — ILLEGAL inside
C++ local classes. Hoisting to module level (with per-site _L<line>
renames, mirroring prep_expand's counted-drop convention) makes both
problems disappear and preserves Drop-counter semantics.

Usage: prep_hoist.py <lib.rs>   (rewrites in place; idempotent via marker)
"""
import re
import sys
from pathlib import Path

MARKER = "// PREP_HOIST_APPLIED"

ITEM_HEAD = re.compile(
    r"^(\s*)((?:#\[[^\]]*\]\s*)*)"
    r"(pub\s+)?(struct|enum|impl|static)\b")


def brace_or_semi_span(text: str, start: int) -> int:
    """End index (exclusive) of an item starting at `start`: through the
    matching close brace of its first brace block, or the first `;` that
    sits OUTSIDE all nesting (tuple/unit structs, statics — a `;` inside
    `[u64; 0]` or `(..)` must not end the item)."""
    i = start
    n = len(text)
    brace = paren = brack = 0
    while i < n:
        c = text[i]
        if c == "{":
            brace += 1
        elif c == "}":
            brace -= 1
            if brace == 0:
                # struct S { .. }  /  impl .. { .. }
                return i + 1
        elif c == "(":
            paren += 1
        elif c == ")":
            paren -= 1
        elif c == "[":
            brack += 1
        elif c == "]":
            brack -= 1
        elif c == ";" and brace == 0 and paren == 0 and brack == 0:
            return i + 1
        i += 1
    return n


def hoist(path: Path) -> None:
    text = path.read_text()
    if MARKER in text:
        print("prep_hoist: already applied")
        return
    lines = text.splitlines(True)

    # Pass 1: top-level fn spans (line index ranges, 0-based inclusive).
    fn_spans = []
    depth = 0
    fn_start = None
    for i, line in enumerate(lines):
        if depth == 0 and fn_start is None and re.match(r"\s*(?:pub\s+)?fn\s+\w+", line):
            fn_start = i
        depth += line.count("{") - line.count("}")
        if fn_start is not None and depth == 0 and "{" in "".join(lines[fn_start:i + 1]):
            fn_spans.append((fn_start, i))
            fn_start = None

    # Pass 2: find local items per fn.
    per_fn_items = []  # (fn_start, fn_end, [(abs_start, abs_end, names)])
    offsets = [0]
    for l in lines:
        offsets.append(offsets[-1] + len(l))
    for (fs, fe) in fn_spans:
        items = []
        i = fs + 1
        while i <= fe:
            line = lines[i]
            m = ITEM_HEAD.match(line)
            if m and not line.strip().startswith("//"):
                # Pull immediately-preceding attribute lines (#[derive(..)])
                # into the item so they hoist together.
                attr_i = i
                while attr_i - 1 > fs and lines[attr_i - 1].strip().startswith("#["):
                    attr_i -= 1
                item_start = offsets[attr_i] + (
                    len(m.group(1)) if attr_i == i else
                    len(lines[attr_i]) - len(lines[attr_i].lstrip()))
                item_end = brace_or_semi_span(text, item_start)
                item_text = text[item_start:item_end]
                names = []
                nm = re.search(r"\b(?:struct|enum)\s+([A-Za-z_0-9]+)", item_text)
                if nm and m.group(4) in ("struct", "enum"):
                    names.append(nm.group(1))
                if m.group(4) == "static":
                    nm = re.search(r"\bstatic\s+(?:mut\s+)?([A-Za-z_0-9]+)", item_text)
                    if nm:
                        names.append(nm.group(1))
                items.append((item_start, item_end, names))
                # skip past the item's lines
                while i <= fe and offsets[i + 1] <= item_end:
                    i += 1
                i += 1
                continue
            i += 1
        if items:
            per_fn_items.append((fs, fe, items))

    if not per_fn_items:
        print("prep_hoist: nothing to hoist")
        path.write_text(MARKER + "\n" + text)
        return

    # Duplicate names across fns get per-fn _L<line> suffixes applied to
    # the ENTIRE fn span (body + items) before extraction.
    from collections import Counter
    name_counts = Counter(
        n for _, _, items in per_fn_items for *_r, names in items for n in names)
    dups = {n for n, c in name_counts.items() if c > 1}

    # Build the new file back-to-front so offsets stay valid.
    hoisted_texts = []
    new_text = text
    for (fs, fe, items) in reversed(per_fn_items):
        span_start = offsets[fs]
        span_end = offsets[fe + 1]
        fn_text = new_text[span_start:span_end]
        rel_items = [(s - span_start, e - span_start, names) for s, e, names in items]
        # Extract items back-to-front within the fn.
        for (s, e, names) in sorted(rel_items, reverse=True):
            item_text = fn_text[s:e]
            hoisted_texts.append((fs + 1, item_text, names))
            fn_text = fn_text[:s] + fn_text[e:]
        # Per-fn rename of duplicated names (applies to remaining body).
        for n in {n for *_r, names in rel_items for n in names if n in dups}:
            fn_text = re.sub(rf"\b{n}\b", f"{n}_L{fs + 1}", fn_text)
        new_text = new_text[:span_start] + fn_text + new_text[span_end:]

    # Rename inside hoisted items too, and emit in source order.
    final = []
    for (fn_line, item_text, names) in reversed(hoisted_texts):
        for n in names:
            if n in dups:
                item_text = re.sub(rf"\b{n}\b", f"{n}_L{fn_line}", item_text)
        # impl blocks referencing a renamed struct: handled because impls
        # were extracted from the SAME fn AFTER the body rename? No — impls
        # are extracted before renaming. Apply this fn's dup renames here.
        final.append((fn_line, item_text))
    # Apply dup renames to impl blocks (which carry no declared name).
    fn_dup_names = {}
    for (fs, fe, items) in per_fn_items:
        fn_dup_names[fs + 1] = {n for *_r, names in items for n in names if n in dups}
    final2 = []
    for fn_line, item_text in final:
        for n in fn_dup_names.get(fn_line, ()):  # rename refs in impls/statics
            item_text = re.sub(rf"\b{n}\b", f"{n}_L{fn_line}", item_text)
        final2.append(item_text)

    insert_at = new_text.find("\n#[test]")
    if insert_at < 0:
        insert_at = new_text.find("\nfn ")
    hoist_block = "\n" + "\n".join(final2) + "\n"
    new_text = new_text[:insert_at] + hoist_block + new_text[insert_at:]
    path.write_text(MARKER + "\n" + new_text)
    print(f"prep_hoist: hoisted {len(final2)} item(s); dup-renamed: {sorted(dups) or 'none'}")


if __name__ == "__main__":
    hoist(Path(sys.argv[1]))
