#!/usr/bin/env python3
"""Post-transpile patches for the linked_list_port C++20 module port.

Idempotent. The patch set is intentionally minimal compared to the
binary_heap_port 14-patch set: the linked_list emit is cleaner because
it doesn't pull in TryReserveError, NonZero, or ptr::swap. Only two
patches apply:

  1. `visit_byte_buf` in the serde-de prelude takes a
     `rusty::Vec<uint8_t>` argument. The function body sits in the GMF
     (before `export module linked_list_port;`) where module imports
     haven't kicked in, so `rusty::Vec` isn't visible. binary_heap_port
     and rc_port both hit this and stub the body — we do the same.

  2. `using IntoIter = ::IntoIter<T, A>;` inside the LinkedList struct
     references the global `::IntoIter` from vec_port. The transpiler
     emits the type ref but not the cross-module import. Inject
     `import vec_port.vec.into_iter;` (and `vec_port.vec` for
     symmetry) into the module preamble after the `export module`
     line.

Usage:
    python3 post_transpile_patch.py <cpp_out_dir>
"""

import re
import sys
from pathlib import Path


LL_FILE = "linked_list_port.cppm"


STUBBED_VISIT_BYTE_BUF = (
    "rusty::Result<Value, E> visit_byte_buf(auto&&) "
    "{ return rusty::Result<Value, E>::Err(E{}); }"
)


def patch_visit_byte_buf(text: str) -> str:
    """Rebody `visit_byte_buf(rusty::Vec<uint8_t>)` to discard its arg
    and return Err. The serde-de prelude declares this in the GMF
    where module imports haven't kicked in, so `rusty::Vec` is not
    a visible name. Same fix binary_heap_port and rc_port apply.

    Idempotent: bails early if the stub is already in place."""
    if STUBBED_VISIT_BYTE_BUF in text:
        return text
    return re.sub(
        r"rusty::Result<Value, E> visit_byte_buf\([^)]+\)\s*\{[^}]*\}",
        STUBBED_VISIT_BYTE_BUF,
        text,
    )


def patch_box_two_arg_to_one_arg(text: str) -> str:
    """Hand-written `rusty::Box<T>` is single-template-arg, but the
    transpiler emits `rusty::Box<T, const A&>` (with the allocator
    type parameter that Rust's Box has). Strip the `, const A&`
    second template argument so the type names compile against
    the single-arg surface.

    The accompanying `from_raw_in(ptr, &alloc)` call is also
    rewritten to `from_raw(ptr)` since the hand-written Box has
    no allocator-aware constructor (it uses Global by default).
    """
    text = text.replace(", const A&>", ">")
    # `from_raw_in(rusty::as_ptr(VAR), &OBJ.alloc)` or `&OBJ->alloc`.
    # OBJ may contain `->` or `.` (e.g. `this->alloc`, `this->list.alloc`).
    text = re.sub(
        r"from_raw_in\((rusty::as_ptr\([^)]+\)),\s*&[^)]+alloc\)",
        r"from_raw(\1)",
        text,
    )
    # Rust's `Box<T>` auto-derefs on field access (`box.field` reaches
    # into the underlying T). C++ doesn't auto-call `operator->` from
    # `.` syntax, so transpiled `node_shadow1.next` etc. fail to compile.
    # `node_shadow1` is consistently bound from `Box<Node<T>>::from_raw`
    # in linked_list, with fields `next` / `prev` / `element`. Rewrite
    # to arrow access. Narrow to known field names so we don't disturb
    # any unrelated `.something` on the same identifier.
    for field in ("next", "prev", "element"):
        text = re.sub(
            rf"(?<![A-Za-z0-9_])node_shadow1\.{field}(?![A-Za-z0-9_])",
            f"node_shadow1->{field}",
            text,
        )
    return text


def patch_global_value_default_construct(text: str) -> str:
    """`rusty::alloc::Global` used as a value rather than a type.
    The transpiler emits the qualified type path where a default-
    constructed instance is expected (e.g. as a positional argument
    in a struct-literal expansion). Same fix rc_port applies.

    Three call sites:
      - `rusty::alloc::Global)`  — last positional arg
      - `rusty::alloc::Global,`  — middle positional arg
      - `rusty::alloc::Global.`  — method call on type (wrong)
    """
    text = re.sub(
        r"(?<![A-Za-z0-9_:]):?:?rusty::alloc::Global(?=\))",
        "rusty::alloc::Global{}",
        text,
    )
    text = re.sub(
        r"(?<![A-Za-z0-9_:]):?:?rusty::alloc::Global(?=,)",
        "rusty::alloc::Global{}",
        text,
    )
    text = re.sub(
        r"(?<![A-Za-z0-9_:]):?:?rusty::alloc::Global(?=\.)",
        "rusty::alloc::Global{}",
        text,
    )
    return text


def patch_inject_vec_imports(text: str) -> str:
    """Inject `import vec_port.vec;` and `import vec_port.vec.into_iter;`
    after the `export module linked_list_port;` line so that
    `::IntoIter<T, A>` references in LinkedList resolve.

    Idempotent: skips injection if the import line is already present."""
    if "import vec_port.vec.into_iter;" in text:
        return text
    if "export module linked_list_port;" not in text:
        return text
    return text.replace(
        "export module linked_list_port;\n",
        (
            "export module linked_list_port;\n\n"
            "import vec_port.vec;  // patcher-injected for ::Vec\n"
            "import vec_port.vec.into_iter;  // patcher-injected for ::IntoIter\n"
        ),
        1,
    )


def patch_file(path: Path) -> bool:
    """Apply all patches to LL_FILE. Returns True if anything changed."""
    text = path.read_text()
    original = text

    text = patch_visit_byte_buf(text)
    text = patch_inject_vec_imports(text)
    text = patch_box_two_arg_to_one_arg(text)
    text = patch_global_value_default_construct(text)

    if text != original:
        path.write_text(text)
        return True
    return False


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 1
    cpp_out = Path(sys.argv[1])
    if not cpp_out.exists():
        print(f"error: {cpp_out} does not exist")
        return 1

    path = cpp_out / LL_FILE
    if not path.exists():
        print(f"error: {path} does not exist")
        return 1

    changed = patch_file(path)
    if changed:
        print(f"linked_list_port patches applied to {path.name}")
    else:
        print(f"linked_list_port: no patches needed (already clean or idempotent)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
