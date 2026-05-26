#!/usr/bin/env python3
"""Post-transpile patches for the vec_port C++20 module port.

Each patch addresses a specific cluster of errors documented in
docs/rusty-std-book.md Chapter 4. As transpiler fixes land that
make a patch redundant, mark its entry with `# RETIRED:` and leave
the function in place for one re-transpile cycle as a guard against
regression.

Usage:
    python3 post_transpile_patch.py /tmp/vec_port/cpp_out/

Idempotent: rerunning detects already-applied patches and skips.
"""

import re
import sys
from pathlib import Path


def patch_set_len_on_drop_copy_assign(cpp_out: Path) -> int:
    """Cluster V-D: SetLenOnDrop has `size_t& len` (reference field) so
    its copy assignment operator can't be `= default` (implicitly
    deleted). Rust types don't have implicit copy-assign; emit
    `= delete` instead.

    Also: copy ctor `= default` on a struct with reference field is
    fine (binds the reference identically), but copy-assign isn't —
    references can't rebind.
    """
    path = cpp_out / "vec_port.vec.set_len_on_drop.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    old = "    SetLenOnDrop& operator=(const SetLenOnDrop&) = default;"
    new = "    SetLenOnDrop& operator=(const SetLenOnDrop&) = delete;"
    if old in text:
        text = text.replace(old, new)
        path.write_text(text)
        return 1
    if new in text:
        return 0  # already applied
    return 0


def patch_is_zero_free_fn_const(cpp_out: Path) -> int:
    """Cluster V-F (related to V-D): is_zero.cppm emits free functions
    with `const` qualifier (`bool is_zero() const` outside a class).
    Strip the trailing `const` for free-function emissions.
    """
    path = cpp_out / "vec_port.vec.is_zero.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    # Match `bool fn_name(...) const {` at module/file scope only —
    # the `const` qualifier is invalid there.
    # Pattern is conservative: only strip when the line starts at
    # column 0 (no indent, so it's free-fn scope).
    text = re.sub(
        r"^(bool [A-Za-z_][A-Za-z_0-9]*\([^)]*\))\s+const(\s*\{)",
        r"\1\2",
        text,
        flags=re.MULTILINE,
    )
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_std_collections_to_rusty(cpp_out: Path) -> int:
    """Cluster V-B: `std::collections::TryReserveError` doesn't exist
    in C++ std. Map to `rusty::collections::TryReserveError` (defined
    in include/rusty/collections.hpp). Also strip the namespace-using
    Rust glob `using namespace std::collections::TryReserveErrorKind;`
    which can't apply to enum-class members in C++ (use scoped names
    instead).
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        text = text.replace(
            "std::collections::TryReserveError",
            "rusty::collections::TryReserveError",
        )
        text = text.replace(
            "std::collections::TryReserveErrorKind",
            "rusty::collections::TryReserveErrorKind",
        )
        # The `using namespace ...::TryReserveErrorKind;` would import
        # CapacityOverflow + AllocError as unqualified. After the
        # rusty:: switch above, that becomes
        # `using namespace rusty::collections::TryReserveErrorKind;`
        # which doesn't work for an enum class. Replace with the
        # explicit prefix at call sites in raw_vec source via prep.sh;
        # here, just delete the using line as a no-op.
        text = text.replace(
            "// Rust-only: using namespace rusty::collections::TryReserveErrorKind;\n",
            "",
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_std_ptr_to_rusty(cpp_out: Path) -> int:
    """Cluster V-C: `std::ptr::Unique<T>` / `std::ptr::Alignment` don't
    exist. Map to `rusty::ptr::Unique<T>` / `rusty::ptr::Alignment`
    (added to include/rusty/ptr.hpp for this port).
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        text = text.replace("std::ptr::Unique", "rusty::ptr::Unique")
        text = text.replace("std::ptr::Alignment", "rusty::ptr::Alignment")
        text = text.replace("std::ptr::NonNull", "rusty::ptr::NonNull")
        # also `std::ptr::slice_from_raw_parts_mut` is a free function
        text = text.replace(
            "std::ptr::slice_from_raw_parts_mut",
            "rusty::ptr::from_raw_parts_mut",
        )
        # Bare `ptr::slice_from_raw_parts_mut` (without `std::` prefix,
        # emitted when the source had `use core::ptr;` and called `ptr::X`).
        # Maps to `rusty::from_raw_parts_mut` (top-level, NOT `rusty::ptr::`).
        text = text.replace(
            "ptr::slice_from_raw_parts_mut",
            "rusty::from_raw_parts_mut",
        )
        text = text.replace(
            "rusty::ptr::from_raw_parts_mut",
            "rusty::from_raw_parts_mut",
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_cap_as_inner(cpp_out: Path) -> int:
    """After prep.sh's `niche_types::UsizeNoHighBit` → `usize` strip,
    `Cap` is a plain `size_t`. But the source calls `.as_inner()` on
    Cap values (an inherent method of UsizeNoHighBit). Strip the
    method call so we get the underlying value directly.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        text = text.replace(".as_inner()", "")
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_bare_capacity_overflow(cpp_out: Path) -> int:
    """In rustc, `use std::collections::TryReserveErrorKind::*;` brings
    `CapacityOverflow` into scope as a bare identifier. We can't do
    that for an enum class in C++. Replace bare `CapacityOverflow` (as
    used in `Err(CapacityOverflow)` / `.ok_or(CapacityOverflow)`) with
    the fully-qualified name.

    Caveat: the bare token also appears in `capacity_overflow()` (lower-
    case function name), which we MUST NOT rewrite. The pattern below
    is conservative: only match `CapacityOverflow` with a non-name
    character on each side and NOT immediately followed by `(` (i.e.,
    not the function-call form).
    """
    import re
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        # Replace bare `CapacityOverflow` only when used as an
        # enumerator (not as a function name capacity_overflow which
        # is lowercase, so no conflict; just guard against the type
        # ctor form `CapacityOverflow(...)` which is a function call).
        # Match: `CapacityOverflow` not followed by `(`
        text = re.sub(
            r"\bCapacityOverflow\b(?!\s*\()",
            "rusty::collections::TryReserveErrorKind::CapacityOverflow",
            text,
        )
        # `CapacityOverflow(CapacityOverflow)` is from rustc using a
        # tuple-variant ctor — rewrite to plain enumerator inside the
        # outer ctor call: `Err::Kind(Kind::CapacityOverflow)`.
        text = text.replace(
            "CapacityOverflow(rusty::collections::TryReserveErrorKind::CapacityOverflow)",
            "rusty::collections::TryReserveErrorKind::CapacityOverflow",
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_cap_alias_order(cpp_out: Path) -> int:
    """The emitted raw_vec.cppm has:
        constexpr Cap ZERO_CAP = static_cast<size_t>(0);
        using Cap = size_t;
    But `Cap` is used before its alias is declared. Swap the order.
    """
    path = cpp_out / "vec_port.raw_vec.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    old = "constexpr Cap ZERO_CAP = static_cast<size_t>(0);\nusing Cap = size_t;"
    new = "using Cap = size_t;\nconstexpr Cap ZERO_CAP = static_cast<size_t>(0);"
    if old in text:
        text = text.replace(old, new)
        path.write_text(text)
        return 1
    return 0


def patch_global_unit_struct_value(cpp_out: Path) -> int:
    """In Rust, `Global` is a unit struct usable both as a type and a
    value (`Global` is shorthand for `Global {}`). The transpiler
    emits the bare name; C++ needs `Global{}` at value positions.

    Targeted patch: just the literal pattern `new_in(rusty::alloc::Global)`
    which is the call site shape that breaks. (Earlier broader regex
    was too aggressive — broke template-arg sites.)
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        # Exact patterns observed in raw_vec.cppm:
        text = text.replace(
            "new_in(rusty::alloc::Global)",
            "new_in(rusty::alloc::Global{})",
        )
        # Cluster of similar call patterns where Global is the last arg:
        text = text.replace(
            ", rusty::alloc::Global)",
            ", rusty::alloc::Global{})",
        )
        text = text.replace(
            ", rusty::alloc::Global,",
            ", rusty::alloc::Global{},",
        )
        text = text.replace(
            "(rusty::alloc::Global,",
            "(rusty::alloc::Global{},",
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_unchecked_arith_intrinsics(cpp_out: Path) -> int:
    """`usize::unchecked_mul(a, b)` is a Rust nightly intrinsic — replace
    method-call form `expr.unchecked_mul(arg)` with `(expr * arg)`.

    Loses overflow-UB semantics, but for our use sites (vec capacity
    arithmetic) overflow is checked upstream.
    """
    import re
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        # `.unchecked_mul(arg)` → ` * arg`. Allow one level of nested
        # parens to catch forms like `.unchecked_mul(std::move(cap))`.
        text = re.sub(
            r"\.unchecked_mul\(((?:[^()]|\([^()]*\))+)\)",
            r" * (\1)",
            text,
        )
        text = re.sub(
            r"\.unchecked_add\(((?:[^()]|\([^()]*\))+)\)",
            r" + (\1)",
            text,
        )
        text = re.sub(
            r"\.unchecked_sub\(((?:[^()]|\([^()]*\))+)\)",
            r" - (\1)",
            text,
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_without_provenance_mut(cpp_out: Path) -> int:
    """`ptr::without_provenance_mut(addr)` is a nightly fn that
    constructs a raw pointer from a usize address. C++ equivalent is
    just `reinterpret_cast<T*>(addr)`. For now, map to a small inline
    helper or just cast.

    Used in raw_vec to create a dangling-aligned pointer for
    zero-capacity vecs. Replace `ptr::without_provenance_mut(addr)`
    with `reinterpret_cast<uint8_t*>(addr)`.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        text = text.replace(
            "ptr::without_provenance_mut(",
            "reinterpret_cast<uint8_t*>(",
        )
        text = text.replace(
            "rusty::ptr::without_provenance_mut(",
            "reinterpret_cast<uint8_t*>(",
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_layout_size_align_paren(cpp_out: Path) -> int:
    """`Layout.size()` and `Layout.align()` in rustc are methods, but our
    `rusty::alloc::Layout` has `size` and `align` as fields (can't make
    them methods due to name conflict with field). Strip the parens
    from `.size()` and `.align()` calls on Layout objects.

    Conservative: only strip when `()` is followed by space or `;` `,`
    `)` `.` `<` `=` (i.e., not when chained with another method call).
    """
    import re
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        # `.size()` followed by a "stop" character → just `.size`
        text = re.sub(r"\.size\(\)(\s*[;,)\.<=])", r".size\1", text)
        text = re.sub(r"\.align\(\)(\s*[;,)\.<=])", r".align\1", text)
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_top_level_import_subset(cpp_out: Path) -> int:
    """Match the reduced CMakeLists.txt: strip top-level
    `export import vec_port.X` lines for modules not in the build.
    """
    path = cpp_out / "vec_port.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    # Keep only raw_vec and set_len_on_drop imports.
    keep = {
        "vec_port.raw_vec",
        "vec_port.vec.set_len_on_drop",
    }
    out_lines = []
    for line in text.splitlines(keepends=True):
        stripped = line.strip()
        if stripped.startswith("export import vec_port."):
            mod = stripped[len("export import "):].rstrip(";").strip()
            if mod not in keep:
                continue
        out_lines.append(line)
    text = "".join(out_lines)
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_layout_size_align_targeted(cpp_out: Path) -> int:
    """Targeted replacements for `Layout.size()` / `Layout.align()` at
    the specific known sites where they cause errors. Conservative
    list maintained by hand; expanded as new sites are surfaced.

    The general regex form is too aggressive — it would strip `.size()`
    from std::span and other valid call sites.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        # raw_vec layout_array:
        #   `elem_layout.size == elem_layout.pad_to_align().size()`
        text = text.replace(
            "elem_layout.pad_to_align().size()",
            "elem_layout.pad_to_align().size",
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_bare_unique_template_args(cpp_out: Path) -> int:
    """The transpiler emits `rusty::ptr::Unique::new_unchecked(...)` with
    no template arg in raw_vec::new_in (after our prep.sh rewrite). The
    type at that site is uint8_t. Targeted insertion of `<uint8_t>`.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        text = text.replace(
            "rusty::ptr::Unique::new_unchecked(",
            "rusty::ptr::Unique<uint8_t>::new_unchecked(",
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_handle_error_function(cpp_out: Path) -> int:
    """The transpiler's emit of raw_vec::handle_error mixes 4 separate
    bugs into a single line: `[[noreturn]]` in trailing-return-type
    position, variant-as-pattern declaration `const auto& X::Y =
    _m;`, `.kind()` method call when `kind` is a field, and an
    unreachable!() return-from-IIFE that needs the explicit form.
    Hand-stub the function body.
    """
    import re
    path = cpp_out / "vec_port.raw_vec.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    # Find the line that starts with `[[noreturn]] void handle_error(` and
    # spans through the matching `; }` (the body).
    # Source pattern (single-line):
    #   [[noreturn]] void handle_error(<args>) {
    #     return [&]() -> [[noreturn]] void { ... }();
    #   }
    # Replace the body with a simple throw.
    pattern = re.compile(
        r"\[\[noreturn\]\] void handle_error\(([^)]+)\) \{[\s\S]*?\n\}",
        re.MULTILINE,
    )
    replacement = (
        "[[noreturn]] void handle_error(\\1) {\n"
        "    if (e.kind == rusty::collections::TryReserveErrorKind::CapacityOverflow) {\n"
        "        ::capacity_overflow();\n"
        "    }\n"
        "    // AllocError branch — abort for now; full impl would call handle_alloc_error.\n"
        "    rusty::intrinsics::abort();\n"
        "}"
    )
    m = pattern.search(text)
    if m and "rusty::intrinsics::abort" not in m.group(0):
        text = pattern.sub(replacement, text, count=1)
        path.write_text(text)
        return 1
    return 0


def patch_hint_assert_unchecked(cpp_out: Path) -> int:
    """`core::hint::assert_unchecked(cond)` is a Rust intrinsic that
    tells the compiler `cond` is true. Map to `__builtin_assume(cond)`
    on clang/gcc (no-op otherwise).
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        text = text.replace("hint::assert_unchecked(", "__builtin_assume(")
        # Sometimes emitted with full path
        text = text.replace("core::hint::assert_unchecked(", "__builtin_assume(")
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_trim_cmakelists(cpp_out: Path) -> int:
    """Reduce CMakeLists.txt to core modules only — drop auxiliary
    modules (cow, extract_if, in_place_*, peek_mut, splice, spec_*,
    partial_eq) that have their own deeper cluster issues (V-A...V-E)
    and aren't needed for a Phase C smoke test.
    """
    path = cpp_out / "CMakeLists.txt"
    if not path.exists():
        return 0
    text = path.read_text()
    if "# REDUCED-SCOPE BUILD" in text:
        return 0  # already trimmed
    # Auxiliary modules deferred until their cluster-specific issues
    # are addressed. Current core: raw_vec + set_len_on_drop +
    # (eventually) vec. `into_iter` and `drain` need their own
    # Phase A2 work; deferred.
    reduced = """# Auto-generated by rusty-cpp-transpiler, then trimmed by
# docs/vec_port/post_transpile_patch.py.
# REDUCED-SCOPE BUILD — core 3 modules only. Auxiliary modules
# (cow, extract_if, in_place_*, peek_mut, splice, spec_*, partial_eq,
# is_zero, into_iter, drain) are not built until their cluster-
# specific issues are addressed (see rusty-std-book Chapter 4).

cmake_minimum_required(VERSION 3.28)
project(vec_port VERSION 0.0.1 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(vec_port
    vec_port.cppm
    vec_port.raw_vec.cppm
    vec_port.vec.set_len_on_drop.cppm
)

target_sources(vec_port PUBLIC FILE_SET CXX_MODULES FILES
    vec_port.cppm
    vec_port.raw_vec.cppm
    vec_port.vec.set_len_on_drop.cppm
)
"""
    path.write_text(reduced)
    return 1


def main(cpp_out: Path):
    patches = [
        ("set_len_on_drop copy-assign", patch_set_len_on_drop_copy_assign),
        ("is_zero free-fn const qualifier", patch_is_zero_free_fn_const),
        ("std::collections → rusty::collections", patch_std_collections_to_rusty),
        ("std::ptr → rusty::ptr", patch_std_ptr_to_rusty),
        ("Cap.as_inner() → ''", patch_cap_as_inner),
        ("Cap alias declaration order", patch_cap_alias_order),
        ("rusty::alloc::Global → Global{}", patch_global_unit_struct_value),
        ("bare CapacityOverflow → fully-qualified", patch_bare_capacity_overflow),
        ("usize::unchecked_mul/add/sub → operators", patch_unchecked_arith_intrinsics),
        ("ptr::without_provenance_mut → reinterpret_cast", patch_without_provenance_mut),
        ("bare Unique → Unique<uint8_t>", patch_bare_unique_template_args),
        ("handle_error body stub (4 mixed emit bugs)", patch_handle_error_function),
        # ("Layout.size()/.align() → field access", patch_layout_size_align_paren),
        # ^ DISABLED: regex too aggressive; hits std::span::size and others.
        #   Need a context-aware rewrite (only on Layout-typed exprs).
        #   Targeted alternative: patch the specific exprs by exact match.
        ("Layout.size()/.align() targeted (specific call sites)",
            patch_layout_size_align_targeted),
        ("strip top-level imports for dropped modules",
            patch_top_level_import_subset),
        ("hint::assert_unchecked → __builtin_assume", patch_hint_assert_unchecked),
        ("trim CMakeLists to core 6", patch_trim_cmakelists),
    ]
    total = 0
    for name, fn in patches:
        n = fn(cpp_out)
        if n:
            print(f"  applied: {name}")
            total += n
        else:
            print(f"  skipped: {name} (already applied or not applicable)")
    print(f"{total} patch(es) applied")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("usage: python3 post_transpile_patch.py <cpp_out_dir>", file=sys.stderr)
        sys.exit(1)
    main(Path(sys.argv[1]))
