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


def patch_module_qualified_refs(cpp_out: Path) -> int:
    """vec.cppm has `raw_vec::RawVec<T, A>`, `into_iter::IntoIter<T, A>`,
    etc. — submodule-qualified references. C++ modules don't preserve
    the Rust-style submodule namespace; the imported symbols are
    flat. Strip the submodule prefix.

    Affects: raw_vec::, peek_mut::, into_iter::, in_place_drop::,
    spec_from_iter_nested::, spec_from_iter::, spec_from_elem::,
    spec_extend::, set_len_on_drop::, is_zero::, in_place_collect::,
    extract_if::, drain::, splice::, partial_eq::, cow::
    """
    submodules = [
        "raw_vec",
        "peek_mut",
        "into_iter",
        "in_place_drop",
        "in_place_collect",
        "spec_from_iter_nested",
        "spec_from_iter",
        "spec_from_elem",
        "spec_extend",
        "set_len_on_drop",
        "is_zero",
        "extract_if",
        "drain",
        "splice",
        "partial_eq",
        "cow",
    ]
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        for sub in submodules:
            text = text.replace(f"{sub}::", "")
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_strip_orphan_using_decls(cpp_out: Path) -> int:
    """After `patch_module_qualified_refs` strips submodule prefixes,
    `using submodule::Symbol;` declarations become bare `using Symbol;`
    which is invalid. Strip these lines entirely — they're now redundant
    (the symbols are already visible from the import).

    Also handles `export using X;` form.

    Also strip specific known-bad lines like `using std::ub_checks;`
    (ub_checks isn't in C++ std).
    """
    import re
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        # Remove bare `using <ident>;` lines (no namespace qualification).
        text = re.sub(r"^using\s+([A-Za-z_][A-Za-z_0-9]*);\s*\n",
                      "", text, flags=re.MULTILINE)
        # Remove `export using <ident>;` lines.
        text = re.sub(r"^export\s+using\s+([A-Za-z_][A-Za-z_0-9]*);\s*\n",
                      "", text, flags=re.MULTILINE)
        # Strip `using std::ub_checks;` — not in C++ std.
        text = text.replace("using std::ub_checks;\n", "")
        # Strip rusty::Cow / Cow_Borrowed / Cow_Owned usings (Cow not ported).
        text = text.replace("using rusty::Cow;\n", "")
        text = text.replace("using rusty::Cow_Borrowed;\n", "")
        text = text.replace("using rusty::Cow_Owned;\n", "")
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_stub_dropped_iter_types(cpp_out: Path) -> int:
    """Inject forward-declared stubs for IntoIter / Drain / PeekMut
    (etc.) so that vec.cppm parses even though the actual modules
    aren't built. The stubs are empty class templates; any code path
    that actually uses them will fail at instantiation, which is
    fine — vec.cppm's `Vec<T>` core API doesn't construct these
    types at module compile time.
    """
    path = cpp_out / "vec_port.vec.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    if "// vec_port stubs for dropped aux types" in text:
        return 0  # idempotent
    # Find the `export module` line and inject stubs right after it.
    import re
    m = re.search(r"^export module vec_port\.vec;\s*\n", text, re.MULTILINE)
    if not m:
        return 0
    stubs = """
// vec_port stubs for dropped aux types — see Chapter 4 of rusty-std-book.
// These are forward-declared placeholders so vec.cppm parses; any code
// path that actually instantiates them will fail at the use site,
// which is acceptable for Phase A2 (core compile-only milestone).
export template<typename T, typename A = rusty::alloc::Global> class IntoIter;
export template<typename T, typename A = rusty::alloc::Global> class Drain;
export template<typename T, typename A = rusty::alloc::Global> class ExtractIf;
export template<typename T, typename A = rusty::alloc::Global> class Splice;
export template<typename T, typename A = rusty::alloc::Global> class PeekMut;

"""
    text = text[:m.end()] + stubs + text[m.end():]
    path.write_text(text)
    return 1


def patch_strip_ub_checks(cpp_out: Path) -> int:
    """`std::ub_checks::assert_unsafe_precondition!(...)` is a Rust
    nightly intrinsic. Map to a no-op `(void)0` or strip entirely.
    Conservatively replace the call site as a no-op.
    """
    import re
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        # Find `std::ub_checks::assert_unsafe_precondition(...)` (1 paren level).
        text = re.sub(
            r"std::ub_checks::assert_unsafe_precondition\(((?:[^()]|\([^()]*\))*)\)",
            "((void)0)",
            text,
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_strip_vec_cppm_aux_imports(cpp_out: Path) -> int:
    """Strip imports of auxiliary modules that aren't in our build
    (cow, drain, extract_if, in_place_*, into_iter, is_zero,
    partial_eq, peek_mut, spec_*, splice). Each removed import
    leaves symbols undeclared — those are handled by later patches
    (stub injection or commenting out the offending call sites).
    """
    path = cpp_out / "vec_port.vec.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    dropped = [
        "vec_port.vec.cow",
        "vec_port.vec.drain",
        "vec_port.vec.extract_if",
        "vec_port.vec.in_place_collect",
        "vec_port.vec.in_place_drop",
        "vec_port.vec.into_iter",
        "vec_port.vec.is_zero",
        "vec_port.vec.partial_eq",
        "vec_port.vec.peek_mut",
        "vec_port.vec.spec_extend",
        "vec_port.vec.spec_from_elem",
        "vec_port.vec.spec_from_iter",
        "vec_port.vec.spec_from_iter_nested",
        "vec_port.vec.splice",
    ]
    for mod in dropped:
        text = text.replace(f"import {mod};\n", "")
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_hoist_imports_after_module_decl(cpp_out: Path) -> int:
    """C++20 requires `import X;` declarations to immediately follow the
    `export module Y;` declaration. The transpiler interleaves struct
    declarations / namespace setups between `export module` and the
    first `import`. Find all `import` lines after the module
    declaration and hoist them right after it.

    Conservative: only re-orders within the module body; never reorders
    code before `export module`.
    """
    import re
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        # Find the `export module X;` line.
        m = re.search(r"^export module [^\n;]+;\s*\n", text, re.MULTILINE)
        if not m:
            continue
        body_start = m.end()
        prefix = text[:body_start]
        body = text[body_start:]
        # Find all `import X;` lines in the body.
        import_re = re.compile(r"^import\s+[^\n;]+;\s*\n", re.MULTILINE)
        imports = import_re.findall(body)
        if not imports:
            continue
        # Already correctly ordered? Check if every line between body
        # start and the last import is either an import or whitespace.
        last_import_end = 0
        for ie in import_re.finditer(body):
            last_import_end = ie.end()
        between = body[:last_import_end]
        non_import = import_re.sub("", between).strip()
        if not non_import:
            # Already in order.
            continue
        # Strip all imports from body, then re-prepend them.
        body_without = import_re.sub("", body)
        # Build: prefix + imports_block + "\n" + body_without
        imports_block = "".join(imports)
        new_text = prefix + imports_block + "\n" + body_without
        if new_text != text:
            path.write_text(new_text)
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
    keep = {
        "vec_port.raw_vec",
        "vec_port.vec.set_len_on_drop",
        "vec_port.vec",
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
    vec_port.vec.cppm
)

target_sources(vec_port PUBLIC FILE_SET CXX_MODULES FILES
    vec_port.cppm
    vec_port.raw_vec.cppm
    vec_port.vec.set_len_on_drop.cppm
    vec_port.vec.cppm
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
        ("hoist module-internal imports up to after `export module`",
            patch_hoist_imports_after_module_decl),
        ("strip vec.cppm imports of dropped aux modules",
            patch_strip_vec_cppm_aux_imports),
        ("strip submodule:: qualifiers (flat namespace from import)",
            patch_module_qualified_refs),
        ("strip orphan `using X;` decls (no namespace qual)",
            patch_strip_orphan_using_decls),
        ("stub dropped aux types (IntoIter/Drain/etc.)",
            patch_stub_dropped_iter_types),
        ("strip std::ub_checks::assert_unsafe_precondition",
            patch_strip_ub_checks),
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
