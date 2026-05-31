#!/usr/bin/env python3
"""Post-transpile patches for the cell_port C++20 module port.

Same shape as docs/vec_port/post_transpile_patch.py and
docs/hashbrown_port/post_transpile_patch.py: each patch addresses a
specific cluster of errors documented in STATUS.md. Idempotent —
rerunning detects already-applied patches and skips.

Usage:
    python3 post_transpile_patch.py <cpp_out_dir>
"""

import re
import sys
from pathlib import Path


CELL_FILE = "cell_port.cppm"


def patch_namespace_using_prefix(cpp_out: Path) -> int:
    """Transpiler emits cross-crate namespace imports as bare
    `using ::cmp::Ordering;` etc. These should be qualified with the
    `rusty::` namespace, since that's where the C++ side defines them.
    Same shape applies to fmt/marker/mem/ops/ptr/iter/hash/panic/pin."""
    path = cpp_out / CELL_FILE
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    # The transpiler emits `using ::cmp::X;` for cross-crate imports.
    # Rewrite to `using rusty::cmp::X;` for everything we know about.
    for ns in ("cmp", "fmt", "marker", "mem", "ops", "ptr", "iter",
               "hash", "panic", "pin"):
        text = re.sub(
            rf"^using ::{ns}::",
            f"using rusty::{ns}::",
            text, flags=re.MULTILINE)
        # Also rewrite type references inside declarations (e.g.
        # `const ::panic::Location& foo` → `const rusty::panic::Location&`).
        # Use negative lookbehind so we don't double-prefix
        # `rusty::ns::` (which would yield `rustyrusty::ns::`).
        text = re.sub(
            rf"(?<![A-Za-z0-9_])::{ns}::",
            f"rusty::{ns}::",
            text)
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_borrow_counter_order(cpp_out: Path) -> int:
    """Transpiler emits `using BorrowCounter = ptrdiff_t;` AFTER its
    first use in forward declarations. Move it to the top of the
    module purview (right after `export module cell_port;`)."""
    path = cpp_out / CELL_FILE
    if not path.exists():
        return 0
    text = path.read_text()
    if text.count("using BorrowCounter = ptrdiff_t;") < 1:
        return 0
    # Find the alias line.
    alias = "using BorrowCounter = ptrdiff_t;"
    first_use = text.find("BorrowCounter")
    alias_pos = text.find(alias)
    if alias_pos < 0 or first_use < 0 or alias_pos < first_use:
        # Already in front of first use.
        return 0
    # Remove the alias from its current spot and re-insert immediately
    # after `export module cell_port;`.
    text = text.replace(alias + "\n", "")
    anchor = "export module cell_port;\n"
    anchor_pos = text.find(anchor)
    if anchor_pos < 0:
        return 0
    insert_at = anchor_pos + len(anchor)
    text = text[:insert_at] + "\n" + alias + "\n" + text[insert_at:]
    path.write_text(text)
    return 1


def patch_drop_global_qualifier_on_intra_module_helpers(cpp_out: Path) -> int:
    """Inside cell_port, helpers like `panic_already_borrowed`,
    `panic_already_mutably_borrowed`, `is_writing`, `is_reading` are
    forward-declared at namespace scope (cell_port::). RefCell methods
    call them with a leading `::` (the transpiler emitted them as
    crate-root references). Strip the `::` so the name resolves to the
    enclosing cell_port namespace via unqualified lookup."""
    path = cpp_out / CELL_FILE
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    for name in ("panic_already_borrowed", "panic_already_mutably_borrowed",
                 "is_writing", "is_reading"):
        text = re.sub(rf"::{name}\b", name, text)
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_qualify_bare_ptr_helpers(cpp_out: Path) -> int:
    """The transpiler emits `ptr::replace(...)` / `ptr::eq(...)` etc.
    without a leading `rusty::`. Inside `namespace cell_port` neither
    `ptr` nor `rusty::ptr` is in unqualified scope, so the call fails.
    Rewrite the bare `ptr::FN(` pattern to `rusty::ptr::FN(`."""
    path = cpp_out / CELL_FILE
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    text = re.sub(r"(?<![A-Za-z0-9_:])ptr::(replace|eq|read|write|copy|null|null_mut|drop_in_place|swap|swap_nonoverlapping|addr_of|addr_of_mut)\b",
                  r"rusty::ptr::\1", text)
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_fix_empty_write_macro_stub(cpp_out: Path) -> int:
    """Transpiler emits `auto res = /* write!(f, ...) */;` when it
    can't lower a `write!` macro. The comment block sits where an
    expression should be — clang reads it as `auto res = ;` which is
    a parse error. The next statement always recovers via
    `rusty::write_fmt`, so we can just delete the broken line.

    Idempotent: skip if the broken pattern is already gone."""
    path = cpp_out / CELL_FILE
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    text = re.sub(
        r"^\s*auto res = /\* write!\([^*]*\*/;\s*$",
        "    // write! macro elided — recovered by rusty::write_fmt below",
        text, flags=re.MULTILINE)
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_borrow_field_get_mut_const(cpp_out: Path) -> int:
    """`RefCell::undo_leak()` is a `T& undo_leak()` (non-const) but it
    calls `this->borrow_field.get_mut()`, and `borrow_field` is a
    `Cell<BorrowCounter>` whose `get_mut()` is declared `const` in
    rusty::Cell. The transpiled body does `borrow_field.get_mut() =
    UNUSED;` which fails because the RHS is an rvalue.

    Easy fix: assign through `borrow_field.set(UNUSED)` instead. Same
    semantic effect."""
    path = cpp_out / CELL_FILE
    if not path.exists():
        return 0
    text = path.read_text()
    needle = "this->borrow_field.get_mut() = UNUSED;"
    if needle in text:
        text = text.replace(needle, "this->borrow_field.set(UNUSED);")
        path.write_text(text)
        return 1
    return 0


def patch_drop_assert_coerce_unsized(cpp_out: Path) -> int:
    """`assert_coerce_unsized` is a compile-time check on Rust traits
    we can't represent (CoerceUnsized). Its signature instantiates
    `rusty::UnsafeCell<const int32_t&>` etc. — reference-typed
    UnsafeCell is malformed in our hand-written rusty::UnsafeCell
    (can't form pointers to refs). Rewrite both the forward decl and
    the definition to take no parameters, dropping the
    reference-typed UnsafeCell instantiations entirely. CoerceUnsized
    is a Rust-only trait so the assertion has no runtime meaning."""
    path = cpp_out / CELL_FILE
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    # Forward declaration in the namespace head.
    text = re.sub(
        r"void assert_coerce_unsized\(rusty::UnsafeCell<const int32_t&>[^)]*\);",
        "void assert_coerce_unsized(); "
        "// CoerceUnsized trait-check, signature elided",
        text)
    # Definition (with body block) — replace with empty stub.
    text = re.sub(
        r"void assert_coerce_unsized\(rusty::UnsafeCell<const int32_t&>[^)]*\)\s*\{[^}]*\}",
        "void assert_coerce_unsized() { /* CoerceUnsized stubbed */ }",
        text)
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_unref_option_location(cpp_out: Path) -> int:
    """Cell port stores `Cell<Option<&'static Location>>` in
    BorrowError / RefCell. Our `rusty::Some(x)` helper decays
    references, so it builds an `Option<Location>` value — which
    doesn't convert to the declared `Option<const Location&>` field
    type. Since our `panic::Location` is a one-byte empty marker,
    Option-by-value and Option-by-reference are observationally
    identical. Rewrite the field type to Option-by-value."""
    path = cpp_out / CELL_FILE
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    text = text.replace(
        "rusty::Option<const rusty::panic::Location&>",
        "rusty::Option<rusty::panic::Location>")
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_drop_misplaced_module_imports(cpp_out: Path) -> int:
    """Transpiler emits `import cell_port.lazy;` and `import
    cell_port.once;` partway through the module purview. Two issues:

    1. C++20 requires `import` declarations to appear in the module
       preamble (between `export module` and the first non-import
       declaration). Putting them later is ill-formed.
    2. We don't vendor the lazy/once submodules — only cell.rs is
       transpiled in this port.

    Stub out both imports and the dependent `using lazy::LazyCell;` /
    `using once::OnceCell;` re-exports. The LazyCell/OnceCell types
    aren't used by the BorrowError / RefCell paths the smoke test
    exercises."""
    path = cpp_out / CELL_FILE
    if not path.exists():
        return 0
    text = path.read_text()
    original = text

    # Drop the misplaced imports + their re-exports. Replace with a
    # comment so the line numbers don't shift dramatically.
    text = re.sub(
        r"^import cell_port\.lazy;\s*$",
        "// import cell_port.lazy; — vendor lazy.rs to re-enable",
        text, flags=re.MULTILINE)
    text = re.sub(
        r"^import cell_port\.once;\s*$",
        "// import cell_port.once; — vendor once.rs to re-enable",
        text, flags=re.MULTILINE)
    text = re.sub(
        r"^export using lazy::LazyCell;\s*$",
        "// export using lazy::LazyCell; — lazy submodule not vendored",
        text, flags=re.MULTILINE)
    text = re.sub(
        r"^export using once::OnceCell;\s*$",
        "// export using once::OnceCell; — once submodule not vendored",
        text, flags=re.MULTILINE)

    if text != original:
        path.write_text(text)
        return 1
    return 0


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 1
    cpp_out = Path(sys.argv[1])
    if not cpp_out.exists():
        print(f"error: {cpp_out} does not exist")
        return 1

    patches = [
        ("namespace using prefix", patch_namespace_using_prefix),
        ("BorrowCounter alias order", patch_borrow_counter_order),
        ("drop ::global qualifier on intra-module helpers",
         patch_drop_global_qualifier_on_intra_module_helpers),
        ("qualify bare ptr:: helpers", patch_qualify_bare_ptr_helpers),
        ("fix empty write! macro stub lines",
         patch_fix_empty_write_macro_stub),
        ("borrow_field.get_mut() = UNUSED → .set(UNUSED)",
         patch_borrow_field_get_mut_const),
        ("stub assert_coerce_unsized body",
         patch_drop_assert_coerce_unsized),
        ("Option<&Location> → Option<Location>",
         patch_unref_option_location),
        ("drop misplaced module imports", patch_drop_misplaced_module_imports),
    ]

    total = 0
    for name, fn in patches:
        n = fn(cpp_out)
        if n:
            print(f"  applied: {name}")
        total += n

    print(f"cell_port patches applied: {total}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
