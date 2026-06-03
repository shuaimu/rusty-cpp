#!/usr/bin/env python3
"""Post-transpile patches for the binary_heap_port C++20 module port.

Idempotent. Codifies the patches that took the fresh transpiler output
from `docs/binary_heap_port/prep.sh` + transpile to the vendored
`transpiled/binary_heap_port/binary_heap_port.cppm` that builds clean
and passes all five binary_heap_port_*_test.out test files (38 test
cases total).

Groups (numbered to match `docs/binary_heap_port/STATUS.md`):

  Phase A2 patches (originally inline, codified here):
    P1  — strip local `template<typename T> auto clone(...)` that
          conflicts with rusty::clone in <rusty/move.hpp>
    P2  — rebody visit_byte_buf (Vec<uint8_t> not visible in GMF)
    P3  — inject `import vec_port.vec` + `import vec_port.vec.into_iter`
          after `export module binary_heap_port;`
    P4  — `rusty::Vec<…>` → `::Vec<…>` (Vec module-only now)
    P5  — `rusty::Vec{}` → `::Vec<T>{}` (empty-arg form)
    P6  — `vec::IntoIter` / `vec::Drain` → `::IntoIter` / `::Drain`
    P7  — `std::collections::TryReserveError` → `rusty::collections::…`
    P8  — bare `usize` → `size_t`
    P9  — `size_t::BITS` → `std::numeric_limits<size_t>::digits`
    P10 — `constexpr Option<NonZero<size_t>>` → `inline const`
    P11 — bare `NonZero::new_(N)` → fully qualified
    P12 — `using rusty::Vec;` delete

  Phase-C sift-down patches:
    P13 — Hole::new_: `ptr::read(data[pos])` → `ptr::read(&data[pos])`
    P14 — Hole::~Hole: `copy_nonoverlapping(..., data[pos], ...)` 2nd
          arg → `&data[pos]`
    P15 — Hole::element: `return this->elt;` → `return *this->elt;`
    P16 — sift_down: `mem::swap(&item, &this->data[0])` → drop the
          address-of (refs, not pointers)
    P17 — sift-down comparator: `rusty::get(hole, idx)` →
          `hole.get(idx)`

  D-tier (full-API push, surfaced 2026-06):
    P18 — from(Vec) outer wrapper Vec → BinaryHeap
    P19 — sift-down `ptr::swap` revert (the prior patcher had rewritten
          to std::swap; restore — rusty::ptr::swap now exists in
          include/rusty/ptr.hpp with correct semantics)
    P20 — RebuildOnDrop field+ctor Vec<T,A>& → BinaryHeap<T,A>&

  A-tier (full-API push):
    P21 — remove duplicate extend_one(const T&) overload
    P22 — IntoIter alias `::IntoIter<T,A>` → `::binary_heap_port::…`
    P23 — into_iter() body: `rusty::iter(...)` → `std::move(this->data).into_iter()`
    P24 — clone() body: `rusty::clone(this->data)` → `this->data.clone()`

  Hand-port slot:
    P25 — delete the orphan `Methods for Vec` impl block (function
          bodies outside any class — transpiler emit bug)

CROSS-MODULE DEPENDENCY: the binary_heap_port build also requires
vec_port patches that are NOT applied here (each port's patcher
should only own its own cppm). See the vendored
`transpiled/vec_port/vec_port.vec.cppm` for inline patches:
  - SpecFromIter generic out-of-line body (D3)
  - SpecExtend generic spec_extend body (A4)
  - Vec::from_iter / Vec::extend drop `typename I::IntoIter` (A2)
  - Vec::retain_mut `drop_in_place(cur)` → `drop_in_place(&cur)` (D5)
  - Move Vec's forward decl earlier in the file

Usage:
    python3 post_transpile_patch.py <cpp_out_dir>
"""

import re
import sys
from pathlib import Path


BHP_FILE = "binary_heap_port.cppm"


# ---------------------------------------------------------------------------
# P1: strip local `clone` template that conflicts with rusty::clone.
# ---------------------------------------------------------------------------

LOCAL_CLONE_PATTERN = re.compile(
    r"// Clone: dispatches to \.clone\(\) if available, otherwise copy-constructs\.\n"
    r"template<typename T>\n"
    r"auto clone\(const T& value\) \{\n"
    r"if constexpr \(requires \{ value\.clone\(\); \}\) \{\n"
    r"return value\.clone\(\);\n"
    r"\} else \{\n"
    r"return value;\n"
    r"\}\n"
    r"\}\n"
)


def patch_strip_local_clone(text: str) -> str:
    """Remove the prelude's local `clone` template. rusty::clone in
    <rusty/move.hpp> already handles the same dispatch and the two
    declarations conflict (redefinition error)."""
    return LOCAL_CLONE_PATTERN.sub(
        "// Local clone() template removed — rusty::clone in <rusty/move.hpp> handles this.\n",
        text,
    )


# ---------------------------------------------------------------------------
# P2: rebody visit_byte_buf — its arg uses rusty::Vec<uint8_t> which
# isn't visible in the GMF (module imports kick in after `export module`).
# ---------------------------------------------------------------------------

VISIT_BYTE_BUF_STUB = (
    "template<typename E>\n"
    "rusty::Result<Value, E> visit_byte_buf(auto&& value) {\n"
    "(void)value; return rusty::Result<Value, E>::Err(E{});\n"
    "}"
)


def patch_visit_byte_buf(text: str) -> str:
    """Rebody to discard arg + return Err. Idempotent: skips if the stub
    is already present."""
    if VISIT_BYTE_BUF_STUB in text:
        return text
    return re.sub(
        r"template<typename E>\nrusty::Result<Value, E> visit_byte_buf\(rusty::Vec<uint8_t> value\) \{\n"
        r"return rusty::Result<Value, E>::Ok\(rusty::as_u8_slice\(value\)\);\n"
        r"\}",
        VISIT_BYTE_BUF_STUB,
        text,
    )


# ---------------------------------------------------------------------------
# P3: inject `import vec_port.vec` + `import vec_port.vec.into_iter`
# after `export module binary_heap_port;`.
# ---------------------------------------------------------------------------


def patch_inject_vec_imports(text: str) -> str:
    if "import vec_port.vec.into_iter;" in text:
        return text
    if "export module binary_heap_port;" not in text:
        return text
    return text.replace(
        "export module binary_heap_port;\n",
        (
            "export module binary_heap_port;\n\n"
            "import vec_port.vec;  // patcher-injected for ::Vec\n"
            "import vec_port.vec.into_iter;  // patcher-injected for ::IntoIter\n"
        ),
        1,
    )


# ---------------------------------------------------------------------------
# P4 / P5: rusty::Vec namespace rewrites. The Vec module is exported at
# the global namespace (::Vec); the legacy rusty::Vec alias was retired.
# ---------------------------------------------------------------------------


def patch_rusty_vec_namespace(text: str) -> str:
    # P5 must run before P4: `rusty::Vec{}` becomes `::Vec<T>{}` (preserves
    # the empty-default-construct shape), then the bulk `rusty::Vec<` →
    # `::Vec<` catches the rest.
    text = text.replace("rusty::Vec{}", "::Vec<T>{}")
    text = re.sub(r"\brusty::Vec<", "::Vec<", text)
    return text


# ---------------------------------------------------------------------------
# P6: vec_port submodule path leaks.
# ---------------------------------------------------------------------------


def patch_vec_submodule_paths(text: str) -> str:
    text = re.sub(r"\bvec::IntoIter\b", "::IntoIter", text)
    text = re.sub(r"\bvec::Drain\b", "::Drain", text)
    return text


# ---------------------------------------------------------------------------
# P7: TryReserveError lives in rusty::collections, not std::collections.
# ---------------------------------------------------------------------------


def patch_try_reserve_error(text: str) -> str:
    return text.replace(
        "std::collections::TryReserveError",
        "rusty::collections::TryReserveError",
    )


# ---------------------------------------------------------------------------
# P8: bare `usize` identifier → size_t. Carefully avoids usize::MAX etc.
# by also handling P9 next.
# ---------------------------------------------------------------------------


def patch_usize_to_size_t(text: str) -> str:
    return re.sub(r"\busize\b", "size_t", text)


def patch_size_t_bits(text: str) -> str:
    return text.replace("size_t::BITS", "std::numeric_limits<size_t>::digits")


# ---------------------------------------------------------------------------
# P10: `constexpr Option<NonZero<size_t>>` is not a literal type;
# downgrade to `inline const`.
# ---------------------------------------------------------------------------


def patch_constexpr_option_nonzero(text: str) -> str:
    # Fresh emit is `static constexpr rusty::Option<rusty::num::NonZero<size_t>>`.
    # constexpr requires a literal type; Option holding NonZero isn't one.
    # Downgrade to `inline static const`. Match any preceding `static`.
    return re.sub(
        r"(static\s+)?constexpr\s+(const\s+)?rusty::Option<rusty::num::NonZero<size_t>>",
        "inline static const rusty::Option<rusty::num::NonZero<size_t>>",
        text,
    )


# ---------------------------------------------------------------------------
# P11: bare `NonZero::new_(N)` → fully qualified.
# ---------------------------------------------------------------------------


def patch_nonzero_qualified(text: str) -> str:
    return re.sub(
        r"(?<![A-Za-z0-9_:])NonZero::new_\(",
        "rusty::num::NonZero<size_t>::new_(",
        text,
    )


# ---------------------------------------------------------------------------
# P12: `using rusty::Vec;` delete (rusty::Vec is module-only now).
# ---------------------------------------------------------------------------


def patch_strip_using_rusty_vec(text: str) -> str:
    return re.sub(r"\busing rusty::Vec;\n?", "", text)


# ---------------------------------------------------------------------------
# Phase-C sift-down patches P13-P17.
# ---------------------------------------------------------------------------


def patch_hole_ptr_read_takes_address(text: str) -> str:
    """C1: Hole::new_ — `ptr::read(data[…])` needs `&` (data[…] is a
    value, ptr::read needs a pointer). Idempotent via negative
    lookbehind: doesn't re-match if `data[…]` is already preceded by
    `&`."""
    return re.sub(
        r"rusty::ptr::read\((?!&)(data\[[^\]]+\])\)",
        r"rusty::ptr::read(&\1)",
        text,
    )


def patch_hole_dtor_copy_nonoverlapping_address(text: str) -> str:
    """C2: Hole::~Hole — `copy_nonoverlapping(..., this->data[…], 1)`
    2nd arg needs `&`. Idempotent via negative lookbehind."""
    return re.sub(
        r"(rusty::ptr::copy_nonoverlapping\([^;]+?,\s*)(?<![&])this->data\[",
        r"\1&this->data[",
        text,
    )


def patch_hole_element_deref(text: str) -> str:
    """C3: Hole::element returns `T&`; body emitted `return this->elt;`
    where `elt` is `ManuallyDrop<T>`. Deref via operator*."""
    return re.sub(
        r"(rusty::Option<const T&>|const T&|T&)\s+element\(\)\s*const\s*\{\s*return this->elt;\s*\}",
        r"\1 element() const { return *this->elt; }",
        text,
    )


def patch_mem_swap_takes_refs(text: str) -> str:
    """C4: sift_down emitted `mem::swap(&item, &this->data[0])` but
    rusty::mem::swap takes refs, not pointers. Drop the address-of."""
    return re.sub(
        r"rusty::mem::swap\(&item,\s*&this->data\[0\]\)",
        "rusty::mem::swap(item, this->data[0])",
        text,
    )


def patch_rusty_get_to_hole_get(text: str) -> str:
    """C5: sift-down comparator emitted `rusty::get(hole, idx)` but the
    free function rusty::get is for slices/Vecs; call Hole's member
    surface instead. Idempotent: re-runs are no-ops because the rewrite
    target doesn't contain the source pattern."""
    return re.sub(
        r"rusty::get\(hole,\s*([^)]+)\)",
        r"hole.get(\1)",
        text,
    )


# ---------------------------------------------------------------------------
# D-tier patches.
# ---------------------------------------------------------------------------


def patch_from_vec_outer_wrapper(text: str) -> str:
    """D1: BinaryHeap::from(Vec) emitted `auto heap = ::Vec<T,A>{.data
    = ...}` (the arg type), should be `BinaryHeap<T,A>{.data = ...}`.
    Sibling `from_raw_vec` already correct."""
    return text.replace(
        "auto heap = ::Vec<T, A>{.data = std::move(vec)};\n        heap.rebuild();",
        "auto heap = BinaryHeap<T, A>{.data = std::move(vec)};\n        heap.rebuild();",
    )


def patch_revert_std_swap_to_ptr_swap(text: str) -> str:
    """D2: the sift-down hot path wants `rusty::ptr::swap(a, b)` (swaps
    `*a` with `*b`). A prior patcher (item 13) blanket-rewrote
    `rusty::ptr::swap` → `std::swap` (wrong semantics + std::swap also
    rejects the rvalue 2nd arg). Fresh transpiler now emits bare
    `ptr::swap(...)` (no `rusty::` prefix). Match both and normalize
    to `rusty::ptr::swap`."""
    text = re.sub(
        r"std::swap\(ptr_shadow1,\s*rusty::ptr::add\(",
        "rusty::ptr::swap(ptr_shadow1, rusty::ptr::add(",
        text,
    )
    # Bare `ptr::swap(` (after `using` makes ptr in scope) → fully
    # qualified rusty::ptr::swap. Guard with negative lookbehind so we
    # don't double-prefix.
    text = re.sub(
        r"(?<![A-Za-z0-9_:])ptr::swap\(",
        "rusty::ptr::swap(",
        text,
    )
    return text


def patch_rebuild_on_drop_heap_type(text: str) -> str:
    """D4: RebuildOnDrop emitted `::Vec<T,A>& heap;` but the dtor body
    calls `heap.rebuild_tail(...)` — a BinaryHeap method. Fix field +
    ctor."""
    text = text.replace(
        "    ::Vec<T, A>& heap;\n    size_t rebuild_from;",
        "    BinaryHeap<T, A>& heap;\n    size_t rebuild_from;",
    )
    text = text.replace(
        "RebuildOnDrop(::Vec<T, A>& heap_init, size_t rebuild_from_init)",
        "RebuildOnDrop(BinaryHeap<T, A>& heap_init, size_t rebuild_from_init)",
    )
    return text


# ---------------------------------------------------------------------------
# A-tier patches.
# ---------------------------------------------------------------------------

EXTEND_ONE_CONST_REF_PATTERN = re.compile(
    r"\n    void extend_one\(const T& _arg1\) \{\n"
    r"        auto&& item = rusty::detail::deref_if_pointer\("
    r"rusty::detail::deref_if_pointer\(_arg1\)\);\n"
    r"        this->push\(std::move\(item\)\);\n"
    r"    \}\n"
)


def patch_remove_extend_one_const_ref(text: str) -> str:
    """A1: drop the redundant `extend_one(const T&)` overload — for
    rvalue callers it's equally viable to `extend_one(T)` and the call
    is ambiguous."""
    return EXTEND_ONE_CONST_REF_PATTERN.sub("\n", text)


def patch_into_iter_alias(text: str) -> str:
    """A3: `using IntoIter = ::IntoIter<T, A>;` aliased the outer
    `::IntoIter` (Vec's) when the Rust source wants the local
    binary_heap_port::IntoIter (the one with the `.iter` field). The
    local namespace IntoIter gets shadowed by this nested alias, so the
    designated init `IntoIter{.iter=…}` fails."""
    return text.replace(
        "    using IntoIter = ::IntoIter<T, A>;\n",
        "    using IntoIter = ::binary_heap_port::IntoIter<T, A>;\n",
    )


def patch_into_iter_body(text: str) -> str:
    """A5: `into_iter()` body emitted `rusty::iter(std::move(this->data))`
    but rusty::iter is the borrowing CPO. The Rust source is
    `self.data.into_iter()` — consuming. Swap."""
    return text.replace(
        "    IntoIter into_iter() {\n"
        "        return IntoIter{.iter = rusty::iter(std::move(this->data))};\n"
        "    }",
        "    IntoIter into_iter() {\n"
        "        return IntoIter{.iter = std::move(this->data).into_iter()};\n"
        "    }",
    )


def patch_clone_body(text: str) -> str:
    """A6: `clone()` body emitted `rusty::clone(this->data)` but
    rusty::clone is a copy-ctor alias; for Vec (defaulted shallow copy
    ctor) that double-frees. Call Vec's deep `.clone()` directly."""
    return text.replace(
        "    BinaryHeap<T, A> clone() const {\n"
        "        return BinaryHeap<T, A>{.data = rusty::clone(this->data)};\n"
        "    }",
        "    BinaryHeap<T, A> clone() const {\n"
        "        return BinaryHeap<T, A>{.data = this->data.clone()};\n"
        "    }",
    )


# ---------------------------------------------------------------------------
# P25: orphan "Methods for Vec" impl block.
# ---------------------------------------------------------------------------

ORPHAN_VEC_METHODS_HEADING = "// Methods for Vec"


def patch_strip_orphan_vec_methods(text: str) -> str:
    """Transpiler emitted method bodies for `impl<T> Vec<T>` outside any
    enclosing class — function bodies floating at namespace scope.
    Delete the entire block bounded by the heading and the next blank
    line + non-method content. Idempotent: heading-absent → no-op."""
    if ORPHAN_VEC_METHODS_HEADING not in text:
        return text
    # Locate heading and trim through the closing `}` of the last
    # orphan body. Conservatively: find heading, then find the next
    # top-level `}` followed by a blank line.
    start = text.find(ORPHAN_VEC_METHODS_HEADING)
    end = text.find("\n}\n\n", start)
    if end == -1:
        return text
    return text[:start] + text[end + 4 :]


# ---------------------------------------------------------------------------
# Driver.
# ---------------------------------------------------------------------------


def patch_file(path: Path) -> bool:
    text = path.read_text()
    original = text

    # Order: structural / imports first, then bulk renames, then
    # site-specific fixes. Each function is idempotent.
    text = patch_strip_local_clone(text)
    text = patch_inject_vec_imports(text)
    text = patch_visit_byte_buf(text)
    text = patch_strip_using_rusty_vec(text)
    text = patch_strip_orphan_vec_methods(text)

    text = patch_rusty_vec_namespace(text)
    text = patch_vec_submodule_paths(text)
    text = patch_try_reserve_error(text)
    text = patch_usize_to_size_t(text)
    text = patch_size_t_bits(text)
    text = patch_constexpr_option_nonzero(text)
    text = patch_nonzero_qualified(text)

    text = patch_hole_ptr_read_takes_address(text)
    text = patch_hole_dtor_copy_nonoverlapping_address(text)
    text = patch_hole_element_deref(text)
    text = patch_mem_swap_takes_refs(text)
    text = patch_rusty_get_to_hole_get(text)

    text = patch_from_vec_outer_wrapper(text)
    text = patch_revert_std_swap_to_ptr_swap(text)
    text = patch_rebuild_on_drop_heap_type(text)

    text = patch_remove_extend_one_const_ref(text)
    text = patch_into_iter_alias(text)
    text = patch_into_iter_body(text)
    text = patch_clone_body(text)

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

    path = cpp_out / BHP_FILE
    if not path.exists():
        print(f"error: {path} does not exist")
        return 1

    changed = patch_file(path)
    if changed:
        print(f"binary_heap_port patches applied to {path.name}")
    else:
        print(f"binary_heap_port: no patches needed (already clean or idempotent)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
