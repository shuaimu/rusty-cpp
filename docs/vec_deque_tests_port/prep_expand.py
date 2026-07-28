#!/usr/bin/env python3
"""Expand `struct_with_counted_drop!` invocations in the vec_deque test
file BEFORE transpiling.

The macro lives in alloctests/testing/macros.rs — the transpiler cannot
expand external macros, and even hand-expanded fn-LOCAL impl blocks are
skipped by the transpiler. So each invocation is hoisted to MODULE level
with per-site unique names (suffix = original line number), and the
enclosing test fn's references are renamed to match.

Shapes handled (all that vec_deque.rs uses):
    struct_with_counted_drop!(Elem, DROPS);
    struct_with_counted_drop!(D(bool), DROPS => |this: &D| if this.0 { panic!("...") } );
    struct_with_counted_drop!(D(u32, bool), DROPS => |this: &D| if this.1 { panic!("...") } );

Usage: prep_expand.py <lib.rs>   (rewrites in place)
"""
import re, sys
from pathlib import Path

INV = re.compile(
    r"^\s*struct_with_counted_drop!\(\s*"
    r"(?P<name>[A-Za-z_0-9]+)\s*(?:\((?P<fields>[^)]*)\))?\s*,\s*"
    r"(?P<counter>[A-Za-z_0-9]+)\s*"
    r"(?:=>\s*\|this: &[A-Za-z_0-9]+\|\s*(?P<stmt>.+?))?\s*\)\s*;\s*$")


def expand(path: Path) -> None:
    lines = path.read_text().splitlines(True)
    hoisted = []
    out = []
    # Track the current fn body: rename struct/counter refs from the
    # invocation line to the end of the enclosing fn (brace depth 0).
    i = 0
    n = len(lines)
    while i < n:
        line = lines[i]
        m = INV.match(line)
        if not m:
            out.append(line)
            i += 1
            continue
        lineno = i + 1
        name, fields, counter, stmt = (
            m.group("name"), m.group("fields"), m.group("counter"), m.group("stmt"))
        uname, ucounter = f"{name}_L{lineno}", f"{counter}_L{lineno}"
        field_decl = f"({fields})" if fields else ""
        drop_body = f"{ucounter}.set({ucounter}.get() + 1);"
        if stmt:
            body = stmt.strip()
            if body.endswith(","):
                body = body[:-1]
            # inline `|this: &D| EXPR` with this -> self
            body = re.sub(r"\bthis\b", "self", body)
            drop_body += f" {body}"
        hoisted.append(
            f"static {ucounter}: std::cell::Cell<u32> = std::cell::Cell::new(0);\n"
            f"#[derive(Clone, Debug, PartialEq)]\n"
            f"struct {uname}{field_decl};\n"
            f"impl Drop for {uname} {{\n"
            f"    fn drop(&mut self) {{ {drop_body} }}\n"
            f"}}\n")
        # Drop the invocation line; rename refs through the fn end.
        # We entered the fn at some earlier `fn ... {` — track depth from
        # here: the invocation sits at depth >= 1; the fn ends when
        # depth returns to 0.
        depth = 0
        for j in range(i):
            depth += lines[j].count("{") - lines[j].count("}")
        i += 1
        while i < n and depth > 0:
            l = lines[i]
            depth += l.count("{") - l.count("}")
            l = re.sub(rf"\b{name}\b", uname, l)
            l = re.sub(rf"\b{counter}\b", ucounter, l)
            out.append(l)
            i += 1
    text = "".join(out)
    # Hoisted defs go right after the leading attrs/uses (before first fn).
    first_fn = text.find("\n#[test]")
    if first_fn < 0:
        first_fn = text.find("\nfn ")
    text = text[:first_fn] + "\n" + "".join(hoisted) + text[first_fn:]
    path.write_text(text)
    print(f"expanded {len(hoisted)} struct_with_counted_drop! invocation(s)")


if __name__ == "__main__":
    expand(Path(sys.argv[1]))
