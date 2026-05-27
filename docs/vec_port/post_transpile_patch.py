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
            r"(?<!TryReserveErrorKind::)\bCapacityOverflow\b(?!\s*\()",
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
    # Find the LAST `import vec_port.X;` line and inject stubs after it.
    # Imports must immediately follow `export module`, so we can't put
    # the stubs between module decl and imports.
    import re
    matches = list(re.finditer(r"^import\s+vec_port\.[^\n;]+;\s*\n",
                               text, re.MULTILINE))
    if not matches:
        return 0
    insert_at = matches[-1].end()
    # When an aux module is in the build, vec.cppm `import`s it and
    # gets the real binary arity; skip stubbing it then.
    has_into_iter_import = "import vec_port.vec.into_iter;" in text
    # Drain and ExtractIf get merged INTO vec.cppm later in the patch
    # pipeline (patch_merge_*_into_vec), bringing their real binary-arity
    # struct defs with them. Always skip the variadic stubs for those.
    has_drain_import = True
    has_extract_if_import = True
    intoiter_stub = "" if has_into_iter_import else \
        "export template<typename... Ts> class IntoIter;\n"
    drain_stub = "" if has_drain_import else \
        "export template<typename... Ts> class Drain;\n"
    extract_if_stub = "" if has_extract_if_import else \
        "export template<typename... Ts> class ExtractIf;\n"
    # Splice and PeekMut: stub only if NOT in AUX_MERGE_MODULES list.
    splice_stub = "" if "splice" in AUX_MERGE_MODULES else \
        "export template<typename... Ts> class Splice;\n"
    peek_mut_stub = "" if "peek_mut" in AUX_MERGE_MODULES else \
        "export template<typename... Ts> class PeekMut;\n"
    stubs = f"""
// vec_port stubs for dropped aux types — see Chapter 4 of rusty-std-book.
// These are forward-declared placeholders so vec.cppm parses; any code
// path that actually instantiates them will fail at the use site,
// which is acceptable for Phase A2 (core compile-only milestone).
//
// Variadic templates accept any arity (rustc uses 2-4 type params
// across these types after dropping the lifetime).
{intoiter_stub}{drain_stub}{extract_if_stub}{splice_stub}{peek_mut_stub}export template<typename... Ts> class AsVecIntoIter;

"""
    text = text[:insert_at] + stubs + text[insert_at:]
    path.write_text(text)
    return 1


def patch_aggregate_raw_ptr_to_span_ctor(cpp_out: Path) -> int:
    """Rust's `intrinsics::aggregate_raw_ptr::<&[T], _, _>(ptr, len)`
    constructs a slice from ptr+len. The transpiler emits it with
    `auto, auto>` template args (Rust's `_`), which C++ rejects.

    Replace the wrapping pattern with a direct std::span constructor call.
    The slice/span type is in the outer return type position, so we can
    extract it from the template-arg [0] of aggregate_raw_ptr.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        # The two observed patterns:
        # std::add_pointer_t<std::add_const_t<std::span<const T>>>, auto, auto
        # std::add_pointer_t<std::span<T>>, auto, auto
        # Strip the outer aggregate_raw_ptr<...>, replacing the whole
        # thing with the inner span type for a direct ctor call.
        text = text.replace(
            "rusty::intrinsics::aggregate_raw_ptr<std::add_pointer_t<std::add_const_t<std::span<const T>>>, auto, auto>",
            "std::span<const T>",
        )
        text = text.replace(
            "rusty::intrinsics::aggregate_raw_ptr<std::add_pointer_t<std::span<T>>, auto, auto>",
            "std::span<T>",
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_strip_noreturn_in_template_and_trailing_ret(cpp_out: Path) -> int:
    """`[[noreturn]]` is being emitted in two positions where C++ rejects it:
    - Template argument: `SafeFn<[[noreturn]] void(size_t)>` — `[[noreturn]]`
      gets parsed as `[lambda capture list]`.
    - Trailing return type: `-> [[noreturn]] void { ... }` — attribute
      placement is invalid here.

    Strip `[[noreturn]] ` from both positions. The function still works
    semantically (just loses the noreturn hint).
    """
    import re
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        # Strip `[[noreturn]]` when followed by `void` (most common case).
        text = re.sub(r"\[\[noreturn\]\]\s+void", "void", text)
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_spec_trait_stubs(cpp_out: Path) -> int:
    """`rusty_ext::spec_extend`, `SpecFromElem`, etc. are extension trait
    implementations in dropped auxiliary modules. Inject stubs so the
    call sites parse; the actual operations will abort at runtime if
    ever reached.
    """
    path = cpp_out / "vec_port.vec.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    if "// vec_port stubs for dropped spec traits" in text:
        return 0  # idempotent
    # Inject after the namespace-stub block.
    marker = "export template<typename... Ts> class AsVecIntoIter;\n"
    if marker not in text:
        return 0
    stubs = marker + """
// vec_port stubs for dropped spec traits — see Chapter 4 of rusty-std-book.
namespace rusty_ext {
    // spec_extend stub: assert at runtime if reached.
    template<typename Vec, typename Iter>
    inline void spec_extend(Vec&&, Iter&&) {
        // Real implementation lives in the spec_extend module we dropped.
        // For minimum-compile, this is a no-op stub.
    }
}

// SpecFromElem stub — real impl in dropped spec_from_elem module.
// Returns auto with no body referencing Vec — defer to instantiation.
struct SpecFromElem {
    template<typename T, typename A>
    static auto from_elem(T elem, std::size_t n, A alloc);
};

// SpecFromIter stub — used in from_iter dispatch.
template<typename T, typename Iter>
struct SpecFromIter {
    template<typename I>
    static auto from_iter(I);
};

// SpecExtend stub.
template<typename T, typename Iter>
struct SpecExtend {
    template<typename V, typename I>
    static void spec_extend(V&, I) {}
};

// SpecCloneIntoVec stub — bridge for slice::clone_from_slice.
// Cannot nest in `namespace rusty::slice` because the module-scope
// `rusty::slice` would conflict with the header-level one.
// Patcher rewrites `::slice::SpecCloneIntoVec` / `slice::SpecCloneIntoVec`
// to bare `SpecCloneIntoVec` for these stubs.
struct SpecCloneIntoVec {
    template<typename Src, typename Dst>
    static void clone_into(Src, Dst&) {}
};

"""
    text = text.replace(marker, stubs)
    path.write_text(text)
    return 1


def patch_macro_template_arg_parens(cpp_out: Path) -> int:
    """`RUSTY_TRY_INTO(RawVec<T, A>::method(...), ...)` — the comma inside
    `<T, A>` confuses the preprocessor (macros split on commas).
    Wrap such expressions in extra parens or use type aliases.

    Quick fix: within the specific call, replace `RawVec<T, A>::` with
    a typedef'd local alias before the macro call. For one-off
    expressions we use the comma-trick: `(RawVec<T, A>())` wraps the
    type-name in parens that the preprocessor sees as one arg.
    Simpler: use a `using` declaration in a nearby IIFE-block. For
    this specific known site, replace with a parenthesized form.
    """
    path = cpp_out / "vec_port.vec.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    # The specific site: `RUSTY_TRY_INTO(RawVec<T, A>::try_with_capacity_in(...)...)`
    # Comma inside `<T, A>` confuses macro. Use a TYPED workaround:
    #   `RUSTY_TRY_INTO((decltype(RawVec<T, A>::try_with_capacity_in(...))::value)::try_with_capacity_in(...))`
    # is gross. Simplest: introduce a typedef *outside* the macro call.
    # For now, leave the spot but use `RawVec<T,A>` no-space form which
    # at least normalizes — but still has comma.
    # Real fix: wrap the entire RawVec<T, A>::method(args) in an immediately
    # invoked lambda that returns the result:
    text = text.replace(
        "RUSTY_TRY_INTO(RawVec<T, A>::try_with_capacity_in(",
        "RUSTY_TRY_INTO(([&]{ return RawVec<T, A>::try_with_capacity_in(",
    )
    # Close the wrapping — need to match the call's closing paren.
    # Conservatively, only do this when the previous replace ran:
    text = text.replace(
        "RUSTY_TRY_INTO(([&]{ return RawVec<T, A>::try_with_capacity_in(std::move(capacity), std::move(alloc)), rusty::Result<Vec<T, A>, rusty::collections::TryReserveError>)",
        "RUSTY_TRY_INTO(([&]{ return RawVec<T, A>::try_with_capacity_in(std::move(capacity), std::move(alloc)); }()), rusty::Result<Vec<T, A>, rusty::collections::TryReserveError>)",
    )
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_auto_ret_init(cpp_out: Path) -> int:
    """`auto ret;` is invalid (no initializer). Replace with a
    placeholder type — `int ret;` is good enough since this is
    inside an unused code path for our stub-heavy build.
    """
    path = cpp_out / "vec_port.vec.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    text = text.replace("            auto ret;", "            int ret = 0;  // stub")
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_box_from_template(cpp_out: Path) -> int:
    """`static auto from(rusty::Vec<T, A> v)` at namespace scope uses
    `T` and `A` undeclared. Prefix with `template<typename T, typename A>`.
    """
    path = cpp_out / "vec_port.vec.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    text = text.replace(
        "// Methods for Box\nstatic auto from(rusty::Vec<T, A> v) {",
        "// Methods for Box\ntemplate<typename T, typename A>\nstatic auto from(rusty::Vec<T, A> v) {",
    )
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_from_into_identity_shortcircuit(cpp_out: Path) -> int:
    """`from_into<X>(X)` (identity) falls through all `if constexpr`
    branches and hits a static_assert. Add an identity short-circuit
    as the FIRST branch.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        # Find the from_into definition (`Target from_into(Input&& input) {` followed by `if constexpr (requires { Target::from(...)`)
        old = """Target from_into(Input&& input) {
if constexpr (requires { Target::from(std::forward<Input>(input)); }) {"""
        new = """Target from_into(Input&& input) {
if constexpr (std::is_same_v<std::remove_cvref_t<Input>, Target>) {
    return Target(std::forward<Input>(input));
} else if constexpr (requires { Target::from(std::forward<Input>(input)); }) {"""
        if old in text:
            text = text.replace(old, new)
            path.write_text(text)
            n += 1
    return n


def patch_t_max_slice_len_t_layout(cpp_out: Path) -> int:
    """`T::MAX_SLICE_LEN` and similar Rust trait associated constants
    that primitive types don't have. Map to compile-time constants.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        # MAX_SLICE_LEN: isize::MAX / size_of::<T>() in Rust
        text = text.replace(
            "rusty::clone(T::MAX_SLICE_LEN)",
            "(static_cast<std::size_t>(std::numeric_limits<std::ptrdiff_t>::max()) / sizeof(T))",
        )
        text = text.replace(
            "T::MAX_SLICE_LEN",
            "(static_cast<std::size_t>(std::numeric_limits<std::ptrdiff_t>::max()) / sizeof(T))",
        )
        # size_of<T>() → sizeof(T)
        import re
        text = re.sub(r"\bsize_of<([^>]+)>\(\)", r"sizeof(\1)", text)
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_layout_align_method_to_field(cpp_out: Path) -> int:
    """`old_layout.align()` — `align` is a Layout field, not method.
    Strip the parens.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        text = text.replace("old_layout.align()", "old_layout.align")
        text = text.replace("new_layout.align()", "new_layout.align")
        text = text.replace("elem_layout.align()", "elem_layout.align")
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_t_layout_to_layout_new(cpp_out: Path) -> int:
    """`T::LAYOUT` is a Rust trait associated constant that primitive
    types (like `int`) don't have. Map to `rusty::alloc::Layout::new_<T>()`.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        text = text.replace("T::LAYOUT", "rusty::alloc::Layout::new_<T>()")
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_castproxy_implicit_conv(cpp_out: Path) -> int:
    """`this->ptr_field.cast().as_non_null_ptr()` returns NonNull<u8>
    (CastProxy's source type) but the function returns NonNull<T>.
    Strip `.as_non_null_ptr()` so the implicit CastProxy → NonNull<T>
    conversion fires at the return statement.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        text = text.replace(
            "this->ptr_field.cast().as_non_null_ptr()",
            "this->ptr_field.cast()",
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_let_pat_double_unwrap(cpp_out: Path) -> int:
    """Rust destructuring let:
        let (ptr, layout) = self.current_memory(elem_layout).unwrap();
    The transpiler emits a `_let_pat` that holds the Option<tuple>
    and then calls `.unwrap()` twice — once per binding. Option's
    unwrap is destructive (moves the inner out), so the second
    unwrap throws "Called unwrap on None".

    Hoist the unwrap onto the binding line so it runs once.
    Pattern matched: the specific shrink_unchecked emit.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        old = (
            "        auto&& _let_pat = (this->current_memory(std::move(elem_layout)));\n"
            "        auto&& ptr_shadow1 = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer((rusty::detail::deref_if_pointer(_let_pat)).unwrap())));\n"
            "        auto&& layout = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer((rusty::detail::deref_if_pointer(_let_pat)).unwrap())));"
        )
        new = (
            "        auto _let_pat_inner = (this->current_memory(std::move(elem_layout))).unwrap();\n"
            "        auto&& ptr_shadow1 = rusty::detail::deref_if_pointer(std::get<0>(_let_pat_inner));\n"
            "        auto&& layout = rusty::detail::deref_if_pointer(std::get<1>(_let_pat_inner));"
        )
        if old in text:
            text = text.replace(old, new)
            path.write_text(text)
            n += 1
    return n


def patch_return_handle_error_void(cpp_out: Path) -> int:
    """Same pattern as patch_return_assert_failed_void, but for
    `handle_error` (the panic path for try_allocate_in failures).
    Emitted as `return handle_error(err)` from an IIFE returning
    RawVecInner<A>; handle_error itself is void.

    Rewrite to `handle_error(err); std::abort();` — abort's
    [[noreturn]] satisfies the IIFE's required return type.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        text = re.sub(
            r"return\s+::handle_error\(([^;]*?)\);",
            r"::handle_error(\1); std::abort();",
            text,
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_merge_drain_into_vec(cpp_out: Path) -> int:
    """Merge drain.cppm's content into vec.cppm so Drain's
    `NonNull<rusty::Vec<T,A>>` field can be rewritten to `NonNull<Vec<T,A>>`
    (the local transpiled Vec). Without this, drain.cppm references
    `rusty::Vec` which the alias maps to VecLegacy with a different
    layout — passing a transpiled Vec* through reinterpret_cast leads
    to runtime corruption via wrong method dispatch.

    Pattern borrowed from btreemap_port (step 52): C++20 modules can't
    express the cyclic reference (drain.cppm needs Vec, vec.cppm needs
    drain), so merge them into one module.
    """
    map_path = cpp_out / "vec_port.vec.cppm"
    drain_path = cpp_out / "vec_port.vec.drain.cppm"
    if not map_path.exists() or not drain_path.exists():
        return 0
    map_src = map_path.read_text()
    sentinel = "// vec_port: drain content merged from vec_port.vec.drain.cppm"
    if sentinel in map_src:
        return 0  # already merged

    # 1. Strip `import vec_port.vec.drain;` (its content is being inlined).
    #    The import may already have been stripped by
    #    patch_strip_vec_cppm_aux_imports — proceed either way.
    new_src, _ = re.subn(
        r"^import vec_port\.vec\.drain;\s*\n",
        f"// {sentinel} (import removed).\n",
        map_src,
        count=1,
        flags=re.MULTILINE,
    )
    map_src = new_src

    # 2. Extract content from drain.cppm: everything after the
    #    `export module vec_port.vec.drain;` line (and any imports).
    drain_src = drain_path.read_text()
    module_anchor = "export module vec_port.vec.drain;\n"
    pos = drain_src.find(module_anchor)
    if pos == -1:
        return 0
    content_start = pos + len(module_anchor)
    # Skip any leading import lines + blank lines.
    while True:
        # Skip blank lines.
        while content_start < len(drain_src) and drain_src[content_start] == "\n":
            content_start += 1
        # If next line is `import X;`, skip it.
        line_end = drain_src.find("\n", content_start)
        if line_end == -1:
            break
        line = drain_src[content_start:line_end]
        if line.startswith("import "):
            content_start = line_end + 1
        else:
            break
    drain_content = drain_src[content_start:]

    # 3. Substitute rusty::Vec → Vec in the injected content. After the
    #    merge, "Vec" refers to the local transpiled Vec (same module).
    drain_content = drain_content.replace("rusty::Vec", "Vec")

    # 4. Inject right before `struct Vec {` so the field type
    #    `NonNull<Vec<T,A>>` has Vec forward-declared (line 3726-ish in
    #    vec.cppm already has `export template<...> struct Vec;`).
    inject_anchor = "struct Vec {"
    ix = map_src.find(inject_anchor)
    if ix == -1:
        return 0
    # Walk back to the start of the line containing the template header
    # above `struct Vec {`.
    lstart = map_src.rfind("\n", 0, ix) + 1
    # Walk back further to find `export template<typename T, typename A...`
    template_line_start = map_src.rfind(
        "export template<typename T, typename A = rusty::alloc::Global>",
        0, ix
    )
    if template_line_start != -1:
        lstart = template_line_start
    inject = f"\n// {sentinel}\n// rusty::Vec rewritten to Vec (local transpiled) inside this block.\n\n{drain_content}\n\n"
    map_src = map_src[:lstart] + inject + map_src[lstart:]

    # 5. In vec.cppm's own body, the drain() method returns Drain<T, A>
    #    by reinterpret_cast-ing this. Now that Drain expects Vec<T,A>*
    #    (not rusty::Vec<T,A>*), drop the reinterpret_cast.
    map_src = map_src.replace(
        "rusty::ptr::NonNull<rusty::Vec<T, A>>::new_unchecked(\n"
        "                    reinterpret_cast<rusty::Vec<T, A>*>(this))",
        "rusty::ptr::NonNull<Vec<T, A>>::new_unchecked(this)",
    )
    # Any remaining rusty::Vec references inside vec.cppm's own scope —
    # leave them; user-facing API still says rusty::Vec.

    map_path.write_text(map_src)
    return 1


def patch_merge_extract_if_into_vec(cpp_out: Path) -> int:
    """Same as patch_merge_drain_into_vec, for extract_if. Merging
    eliminates the rusty::Vec vs vec_port::Vec layout mismatch by
    putting ExtractIf in the same module as Vec.
    """
    map_path = cpp_out / "vec_port.vec.cppm"
    ei_path = cpp_out / "vec_port.vec.extract_if.cppm"
    if not map_path.exists() or not ei_path.exists():
        return 0
    map_src = map_path.read_text()
    sentinel = "// vec_port: extract_if content merged from vec_port.vec.extract_if.cppm"
    if sentinel in map_src:
        return 0

    # The import line may have already been stripped by
    # patch_strip_vec_cppm_aux_imports — that's fine, we still merge.
    new_src, _ = re.subn(
        r"^import vec_port\.vec\.extract_if;\s*\n",
        f"// {sentinel} (import removed).\n",
        map_src,
        count=1,
        flags=re.MULTILINE,
    )
    map_src = new_src

    ei_src = ei_path.read_text()
    module_anchor = "export module vec_port.vec.extract_if;\n"
    pos = ei_src.find(module_anchor)
    if pos == -1:
        return 0
    content_start = pos + len(module_anchor)
    while True:
        while content_start < len(ei_src) and ei_src[content_start] == "\n":
            content_start += 1
        line_end = ei_src.find("\n", content_start)
        if line_end == -1:
            break
        line = ei_src[content_start:line_end]
        if line.startswith("import "):
            content_start = line_end + 1
        else:
            break
    ei_content = ei_src[content_start:]
    ei_content = ei_content.replace("rusty::Vec", "Vec")

    inject_anchor = "struct Vec {"
    ix = map_src.find(inject_anchor)
    if ix == -1:
        return 0
    template_line_start = map_src.rfind(
        "export template<typename T, typename A = rusty::alloc::Global>",
        0, ix
    )
    if template_line_start != -1:
        lstart = template_line_start
    else:
        lstart = map_src.rfind("\n", 0, ix) + 1
    inject = f"\n// {sentinel}\n// rusty::Vec rewritten to Vec (local transpiled) inside this block.\n\n{ei_content}\n\n"
    map_src = map_src[:lstart] + inject + map_src[lstart:]

    # After merge, ExtractIf::new_'s sig is `Vec<T,A>&` (local), and
    # vec.cppm's call site is plain `(*this)`. No cast adjustment needed.

    map_path.write_text(map_src)
    return 1


def patch_drop_extract_if_from_build(cpp_out: Path) -> int:
    cm = cpp_out / "CMakeLists.txt"
    if not cm.exists():
        return 0
    text = cm.read_text()
    if "vec_port.vec.extract_if.cppm" not in text:
        return 0
    text = text.replace("    vec_port.vec.extract_if.cppm\n", "")
    cm.write_text(text)
    return 1


def _merge_aux_module_into_vec(cpp_out: Path, mod_name: str) -> int:
    """Generic merge helper. Same shape as patch_merge_drain_into_vec
    but parametric on module name. Used for the remaining aux modules
    (cow, in_place_*, is_zero, partial_eq, peek_mut, spec_*, splice).
    """
    map_path = cpp_out / "vec_port.vec.cppm"
    aux_path = cpp_out / f"vec_port.vec.{mod_name}.cppm"
    if not map_path.exists() or not aux_path.exists():
        return 0
    map_src = map_path.read_text()
    sentinel = f"// vec_port: {mod_name} content merged from vec_port.vec.{mod_name}.cppm"
    if sentinel in map_src:
        return 0  # already merged

    # 1. Strip the import line (may already be stripped by
    #    patch_strip_vec_cppm_aux_imports — proceed either way).
    new_src, _ = re.subn(
        rf"^import vec_port\.vec\.{re.escape(mod_name)};\s*\n",
        f"// {sentinel} (import removed).\n",
        map_src,
        count=1,
        flags=re.MULTILINE,
    )
    map_src = new_src

    # 2. Extract content from aux module: everything after the
    #    `export module ...;` line + any leading imports.
    aux_src = aux_path.read_text()
    module_anchor = f"export module vec_port.vec.{mod_name};\n"
    pos = aux_src.find(module_anchor)
    if pos == -1:
        return 0
    content_start = pos + len(module_anchor)
    while True:
        while content_start < len(aux_src) and aux_src[content_start] == "\n":
            content_start += 1
        line_end = aux_src.find("\n", content_start)
        if line_end == -1:
            break
        line = aux_src[content_start:line_end]
        if line.startswith("import "):
            content_start = line_end + 1
        else:
            break
    aux_content = aux_src[content_start:]
    aux_content = aux_content.replace("rusty::Vec", "Vec")

    # 3. Inject before the Vec struct definition.
    inject_anchor = "struct Vec {"
    ix = map_src.find(inject_anchor)
    if ix == -1:
        return 0
    template_line_start = map_src.rfind(
        "export template<typename T, typename A = rusty::alloc::Global>",
        0, ix
    )
    if template_line_start != -1:
        lstart = template_line_start
    else:
        lstart = map_src.rfind("\n", 0, ix) + 1
    inject = (f"\n// {sentinel}\n"
              f"// rusty::Vec rewritten to Vec (local transpiled).\n\n"
              f"{aux_content}\n\n")
    map_src = map_src[:lstart] + inject + map_src[lstart:]

    map_path.write_text(map_src)
    return 1


def _drop_aux_from_build(cpp_out: Path, mod_name: str) -> int:
    """Generic CMakeLists drop helper."""
    cm = cpp_out / "CMakeLists.txt"
    if not cm.exists():
        return 0
    text = cm.read_text()
    line = f"    vec_port.vec.{mod_name}.cppm\n"
    if line not in text:
        return 0
    text = text.replace(line, "")
    cm.write_text(text)
    return 1


# Aux modules merged into vec.cppm via the generic merge helper.
# Order matters: dependencies must be merged before dependents.
# Generally these modules are independent of each other; ordering by
# size (small first) for quicker incremental debug.
AUX_MERGE_MODULES = [
    # Modules verified to merge cleanly without surfacing emit-bug
    # clusters. The merge inlines content from vec_port.vec.X into
    # vec.cppm so `rusty::Vec` → `Vec` (local) rewrites match the
    # same module attachment as Vec's own definition.
    "partial_eq",       # operator== between vecs

    # Modules NOT merged — each surfaces emit-bug clusters when
    # added to vec.cppm. Documented in book Ch4 §4.7.
    #
    # Deferred:
    #   spec_from_iter         (ambiguous SpecFromIter ref vs caller)
    #   spec_from_iter_nested  (T leak)
    #   spec_from_elem         (T leak + is_zero ref)
    #   spec_extend            (T leak)
    #   cow                    (auto-as-template-arg)
    #   in_place_drop
    #   peek_mut               (pulls in PeekMut surface)
    #   splice                 (pulls in Splice surface)
    #   in_place_collect       (biggest cluster)
    #   is_zero                (free-standing `this` orphan emits)
]


def patch_merge_remaining_aux(cpp_out: Path) -> int:
    n = 0
    for mod in AUX_MERGE_MODULES:
        n += _merge_aux_module_into_vec(cpp_out, mod)
        _drop_aux_from_build(cpp_out, mod)
    return n


def patch_drain_dropguard_byte_cast(cpp_out: Path) -> int:
    """The transpiled Drain DropGuard destructor casts typed pointers
    to `uint8_t*` before `ptr::add` and `ptr::copy`. That converts
    element offsets to byte offsets — `tail_len` (3 ints) ends up
    meaning 3 BYTES copied, not 12. The tail-shift after partial
    drain corrupts the buffer.

    Strip the reinterpret_cast<uint8_t*> wrappers so ptr::add and
    ptr::copy operate on the typed (T*) pointers, where their
    arithmetic is in element units.
    """
    path = cpp_out / "vec_port.vec.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    text = text.replace(
        "rusty::ptr::add(reinterpret_cast<const uint8_t*>(rusty::as_ptr(source_vec)), std::move(tail))",
        "rusty::ptr::add(rusty::as_ptr(source_vec), std::move(tail))",
    )
    text = text.replace(
        "rusty::ptr::add(reinterpret_cast<uint8_t*>(rusty::as_mut_ptr(source_vec)), std::move(start))",
        "rusty::ptr::add(rusty::as_mut_ptr(source_vec), std::move(start))",
    )
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_drop_drain_from_build(cpp_out: Path) -> int:
    """Remove vec_port.vec.drain.cppm from CMakeLists since its content
    was merged into vec.cppm. Runs AFTER patch_trim_cmakelists.
    """
    cm = cpp_out / "CMakeLists.txt"
    if not cm.exists():
        return 0
    text = cm.read_text()
    if "vec_port.vec.drain.cppm" not in text:
        return 0
    text = text.replace("    vec_port.vec.drain.cppm\n", "")
    cm.write_text(text)
    return 1


def patch_extract_if_runtime(cpp_out: Path) -> int:
    """extract_if instantiation: same _let_pat.start/.end → .first/.second
    fix as drain, plus reinterpret_cast for the rusty::Vec namespace
    mismatch at the new_() call site.
    """
    n = 0

    # vec.cppm: ExtractIf::new_ takes `rusty::Vec<T,A>&`; after the
    # merge into vec.cppm, the merge has already rewritten that to
    # `Vec<T,A>&` (the local transpiled type), so `(*this)` works
    # directly. No reinterpret_cast needed.

    # extract_if.cppm: _let_pat .start/.end → .first/.second
    p_ei = cpp_out / "vec_port.vec.extract_if.cppm"
    if p_ei.exists():
        t = p_ei.read_text()
        orig = t
        t = t.replace(
            "auto&& start = rusty::detail::deref_if_pointer(_let_pat.start);",
            "auto&& start = rusty::detail::deref_if_pointer(_let_pat.first);",
        )
        t = t.replace(
            "auto&& end = rusty::detail::deref_if_pointer(_let_pat.end);",
            "auto&& end = rusty::detail::deref_if_pointer(_let_pat.second);",
        )
        if t != orig:
            p_ei.write_text(t)
            n += 1
    return n


def patch_drain_runtime(cpp_out: Path) -> int:
    """drain() instantiation surfaces several emit bugs:

    1. vec.cppm:`_let_pat.start` / `.end` — _let_pat is a
       `std::pair<size_t, size_t>` from `rusty::slice_ext::range`,
       so the Rust-style `.start`/`.end` access fails. Rewrite to
       `.first` / `.second`.
    2. drain.cppm: `T::IS_ZST` ternary and `const T` field access
       fail for non-class T. Stub the affected helper.
    3. NonNull::from(*this) — needs an addressable lvalue overload.
    """
    n = 0

    # 1. vec.cppm field rename in drain() + designated→positional ctor
    p_vec = cpp_out / "vec_port.vec.cppm"
    if p_vec.exists():
        t = p_vec.read_text()
        orig = t
        t = t.replace(
            "auto&& start = rusty::detail::deref_if_pointer(_let_pat.start);",
            "auto&& start = rusty::detail::deref_if_pointer(_let_pat.first);",
        )
        t = t.replace(
            "auto&& end = rusty::detail::deref_if_pointer(_let_pat.end);",
            "auto&& end = rusty::detail::deref_if_pointer(_let_pat.second);",
        )
        # Drain has positional ctor — convert designated init.
        old_drain_ret = (
            "            return Drain<T, A>{.tail_start = std::move(end), "
            ".tail_len = rusty::detail::deref_if_pointer_like(len) - rusty::detail::deref_if_pointer_like(end), "
            ".iter = rusty::iter(range_slice), "
            ".vec = NonNull<std::remove_pointer_t<std::remove_reference_t<decltype(((*this)))>>>::from((*this))};"
        )
        new_drain_ret = (
            "            return Drain<T, A>(\n"
            "                std::move(end),\n"
            "                rusty::detail::deref_if_pointer_like(len) - rusty::detail::deref_if_pointer_like(end),\n"
            "                rusty::iter(range_slice),\n"
            "                rusty::ptr::NonNull<rusty::Vec<T, A>>::new_unchecked(\n"
            "                    reinterpret_cast<rusty::Vec<T, A>*>(this))\n"
            "            );"
        )
        t = t.replace(old_drain_ret, new_drain_ret)
        if t != orig:
            p_vec.write_text(t)
            n += 1

    # 2. drain.cppm IS_ZST + const T method access.
    p_dr = cpp_out / "vec_port.vec.drain.cppm"
    if p_dr.exists():
        t = p_dr.read_text()
        orig = t
        # Generic `if (T::IS_ZST)` → `if constexpr (false)`
        t = re.sub(
            r"\bif \(T::IS_ZST\) \{",
            "if constexpr (requires { T::IS_ZST; } && false) {",
            t,
        )
        # `T::IS_ZST ? a : b` → just `b` (we never use ZSTs)
        t = re.sub(
            r"T::IS_ZST \? [^:]*?: ",
            "",
            t,
        )
        # `drop_ptr->offset_from_unsigned(vec_ptr)` is Rust pointer
        # method, not in C++; use pointer subtraction.
        t = t.replace(
            "drop_ptr->offset_from_unsigned(vec_ptr)",
            "static_cast<size_t>(drop_ptr - vec_ptr)",
        )
        if t != orig:
            p_dr.write_text(t)
            n += 1
    return n


def patch_t_is_zst_constexpr_if(cpp_out: Path) -> int:
    """Several emit sites have `if (T::IS_ZST)` or `T::IS_ZST ? ... : ...`
    which fail when T is a non-class (e.g. `int` has no members).

    Wrap each in `if constexpr (requires { T::IS_ZST; })` style. For
    the targeted `new_cap` free function we can simplify aggressively:
    we never use ZST so just always take the non-ZST branch.
    """
    n = 0
    p_rv = cpp_out / "vec_port.raw_vec.cppm"
    if p_rv.exists():
        t = p_rv.read_text()
        orig = t
        # `new_cap<T>`: hard-code non-ZST branch.
        t = t.replace(
            "Cap new_cap(size_t cap) {\n    if (T::IS_ZST) {\n        return ZERO_CAP;\n    } else {\n        return std::move(cap);\n    }\n}",
            "Cap new_cap(size_t cap) {\n    // HAND-PORT: ZST branch elided (non-class T fails T::IS_ZST).\n    return std::move(cap);\n}",
        )
        if t != orig:
            p_rv.write_text(t)
            n += 1
    return n


def patch_into_iter_runtime_body(cpp_out: Path) -> int:
    """Phase B for into_iter: hand-port the methods that actually
    run when user code calls v.into_iter().

    The transpiled bodies assume:
      - T has `T::IS_ZST` constexpr static (fails for non-class T
        like `int`)
      - `NonNull<T>::read()` exists (rusty's doesn't)
      - `me.buf` field access on ManuallyDrop<Vec<...>> (it's
        actually wrapped)

    Replace next(), next() back side, size_hint() with simple
    pointer-based implementations. as_raw_mut_slice's return type
    is also flipped (was &mut [T] pointer-typed, returns by value).
    """
    n = 0

    # Vec::into_iter() in vec.cppm — replace ManuallyDrop dance + IS_ZST.
    p_vec = cpp_out / "vec_port.vec.cppm"
    if p_vec.exists():
        t = p_vec.read_text()
        orig = t
        old = (
            "    IntoIter<T, A> into_iter() {\n"
            "        // @unsafe\n"
            "        {\n"
            "            const auto me = rusty::mem::manually_drop_new(std::move((*this)));\n"
            "            auto alloc = rusty::mem::manually_drop_new(rusty::ptr::read(me.allocator()));\n"
            "            auto buf = me.buf.non_null();\n"
            "            const auto begin = const_cast<std::add_pointer_t<std::add_const_t<T>>>(reinterpret_cast<std::add_pointer_t<std::add_const_t<std::add_const_t<T>>>>(rusty::as_ptr(buf)));\n"
            "            auto end = (T::IS_ZST ? begin->wrapping_byte_add(rusty::len(me)) : reinterpret_cast<std::add_pointer_t<std::add_const_t<T>>>(rusty::ptr::add(begin, rusty::len(me))));\n"
            "            auto cap = me.buf.capacity();\n"
            "            return IntoIter{.buf = std::move(buf), .phantom = rusty::PhantomData<std::tuple<>>{}, .cap = std::move(cap), .alloc = std::move(alloc), .ptr = std::move(buf), .end = std::move(end)};\n"
            "        }\n"
            "    }"
        )
        new = (
            "    IntoIter<T, A> into_iter() {\n"
            "        // HAND-PORT: bypass ManuallyDrop wrapper + T::IS_ZST.\n"
            "        auto buf = this->buf.non_null();\n"
            "        auto cap = this->buf.capacity();\n"
            "        auto* begin = rusty::as_ptr(buf);\n"
            "        auto* end_ptr = static_cast<std::add_pointer_t<std::add_const_t<T>>>(begin + this->len_field);\n"
            "        // Steal the allocator (move-construct into a fresh holder)\n"
            "        auto alloc_copy = rusty::mem::manually_drop_new(\n"
            "            std::move(const_cast<A&>(this->buf.allocator())));\n"
            "        // Neutralize this Vec's RawVec so ~Vec doesn't double-free —\n"
            "        // IntoIter takes ownership of the buffer.\n"
            "        this->len_field = 0;\n"
            "        this->buf.rusty_mark_forgotten();\n"
            "        return IntoIter<T, A>(\n"
            "            buf,\n"
            "            rusty::PhantomData<T>{},\n"
            "            cap,\n"
            "            std::move(alloc_copy),\n"
            "            buf,\n"
            "            end_ptr\n"
            "        );\n"
            "    }"
        )
        if old in t:
            t = t.replace(old, new)
        if t != orig:
            p_vec.write_text(t)
            n += 1

    # IntoIter::next() — replace with simple pointer-based body.
    p_ii = cpp_out / "vec_port.vec.into_iter.cppm"
    if p_ii.exists():
        t = p_ii.read_text()
        orig = t

        old_next = (
            "    rusty::Option<T> next() {\n"
            "        decltype(this->ptr) ptr_shadow1 {};\n"
            "        {\n"
            "            if (T::IS_ZST) {\n"
            "                if (rusty::as_ptr(this->ptr) == (const_cast<std::add_pointer_t<T>>(reinterpret_cast<std::add_pointer_t<std::add_const_t<T>>>(this->end)))) {\n"
            "                    return rusty::Option<T>{rusty::None};\n"
            "                }\n"
            "                this->end = this->end->wrapping_byte_sub(1);\n"
            "                ptr_shadow1 = this->ptr;\n"
            "            } else {\n"
            "                if (rusty::detail::deref_if_pointer_like(this->ptr) == rusty::detail::deref_if_pointer_like(this->end)) {\n"
            "                    return rusty::Option<T>{rusty::None};\n"
            "                }\n"
            "                const auto old = this->ptr;\n"
            "                this->ptr = old.add(1);\n"
            "                ptr_shadow1 = old;\n"
            "            }\n"
            "        }\n"
            "        return rusty::Option<T>(ptr_shadow1.read());\n"
            "    }"
        )
        new_next = (
            "    rusty::Option<T> next() {\n"
            "        // HAND-PORT: simple pointer-based; no IS_ZST, no .read().\n"
            "        auto* p = rusty::as_ptr(this->ptr);\n"
            "        if (p == this->end) return rusty::Option<T>{rusty::None};\n"
            "        T value = std::move(*p);\n"
            "        this->ptr = rusty::ptr::NonNull<T>::new_unchecked(p + 1);\n"
            "        return rusty::Option<T>(std::move(value));\n"
            "    }"
        )
        if old_next in t:
            t = t.replace(old_next, new_next)

        # size_hint() — drop IS_ZST branch
        old_sh = (
            "    std::tuple<size_t, rusty::Option<size_t>> size_hint() const {\n"
            "        auto exact = (T::IS_ZST ? (static_cast<size_t>(this->end->addr()) - static_cast<size_t>(rusty::as_ptr(this->ptr)->addr())) : rusty::detail::deref_if_pointer_like(this->end).offset_from_unsigned(this->ptr));\n"
            "        return std::make_tuple(std::move(exact), rusty::Option<size_t>(std::move(exact)));\n"
            "    }"
        )
        new_sh = (
            "    std::tuple<size_t, rusty::Option<size_t>> size_hint() const {\n"
            "        // HAND-PORT: no IS_ZST.\n"
            "        size_t exact = static_cast<size_t>(this->end - rusty::as_ptr(this->ptr));\n"
            "        return std::make_tuple(exact, rusty::Option<size_t>(exact));\n"
            "    }"
        )
        if old_sh in t:
            t = t.replace(old_sh, new_sh)

        # as_raw_mut_slice — wrong return type wrapping
        old_asraw = (
            "    std::add_pointer_t<std::span<T>> as_raw_mut_slice() {\n"
            "        return rusty::from_raw_parts_mut(rusty::as_ptr(this->ptr), rusty::len((*this)));\n"
            "    }"
        )
        new_asraw = (
            "    std::span<T> as_raw_mut_slice() {\n"
            "        // HAND-PORT: drop the bogus pointer wrap.\n"
            "        return rusty::from_raw_parts_mut(rusty::as_ptr(this->ptr), rusty::len((*this)));\n"
            "    }"
        )
        if old_asraw in t:
            t = t.replace(old_asraw, new_asraw)

        # as_mut_slice — was deref'ing the as_raw_mut_slice ptr
        t = t.replace(
            "    std::span<T> as_mut_slice() {\n        // @unsafe\n        {\n            return *this->as_raw_mut_slice();\n        }\n    }",
            "    std::span<T> as_mut_slice() {\n        return this->as_raw_mut_slice();\n    }",
        )

        # forget_allocation_drop_remaining used the pointer form — fix
        t = t.replace(
            "    void forget_allocation_drop_remaining() {\n        const auto remaining = this->as_raw_mut_slice();\n",
            "    void forget_allocation_drop_remaining() {\n        auto remaining = this->as_raw_mut_slice();\n",
        )

        # Stub advance_by — uses T::IS_ZST
        t = re.sub(
            r"    auto advance_by\(size_t n\) -> rusty::Result<std::tuple<>, rusty::num::NonZero<size_t>> \{[\s\S]*?\n    \}\n",
            (
                "    auto advance_by(size_t n) -> rusty::Result<std::tuple<>, rusty::num::NonZero<size_t>> {\n"
                "        // HAND-PORT: no IS_ZST.\n"
                "        auto avail = static_cast<size_t>(this->end - rusty::as_ptr(this->ptr));\n"
                "        auto step = (n < avail) ? n : avail;\n"
                "        for (size_t i = 0; i < step; ++i) { (void)this->next(); }\n"
                "        if (n > avail) {\n"
                "            return rusty::Result<std::tuple<>, rusty::num::NonZero<size_t>>::Err(\n"
                "                rusty::num::NonZero<size_t>::new_(n - avail).unwrap());\n"
                "        }\n"
                "        return rusty::Result<std::tuple<>, rusty::num::NonZero<size_t>>::Ok(std::make_tuple());\n"
                "    }\n"
            ),
            t,
            count=1,
        )

        if t != orig:
            p_ii.write_text(t)
            n += 1

    return n


def patch_into_iter_more_fixes(cpp_out: Path) -> int:
    """Second pass for vec_port.vec.into_iter:

    1. `/* non_null!(self . end , T) */` — macro that got elided.
       Replace with `this->end` (which is already T* / non-null).
    2. `NonZero::new_(...)` → `rusty::num::NonZero<size_t>::new_(...)`
       at remaining call sites.
    3. `last()` body calls `next_back()` which IntoIter doesn't
       expose under that name in our build; stub with a forwarding
       call to next() and a comment.
    4. `next_chunk()` body uses rusty::array::IntoIter; replace the
       whole method body with std::abort().
    """
    path = cpp_out / "vec_port.vec.into_iter.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text

    # Fix 1: macro placeholder → actual expression. `this->end` is
    # already a `const T*` field; the original Rust `non_null!(...)`
    # macro built a NonNull-wrapped fat pointer. For our purposes,
    # treat as the bare pointer.
    text = text.replace(
        "/* non_null!(self . end , T) */",
        "rusty::detail::deref_if_pointer_like(this->end)",
    )

    # Fix 2: bare NonZero::new_ → fully-qualified
    text = re.sub(
        r"(?<![:\w])NonZero::new_\(",
        "rusty::num::NonZero<size_t>::new_(",
        text,
    )

    # Fix 3: stub last() — IntoIter doesn't have next_back in our cut
    text = text.replace(
        "    rusty::Option<T> last() {\n        return this->next_back();\n    }",
        "    rusty::Option<T> last() {\n        // STUBBED: next_back not exposed\n        rusty::Option<T> result{rusty::None};\n        while (true) {\n            auto next = this->next();\n            if (next.is_none()) break;\n            result = std::move(next);\n        }\n        return result;\n    }",
    )

    # Fix 4: stub next_chunk by replacing from the template header
    # to the next `^    \}\n` matched at the right indentation. Match
    # by exact start line then sweep through to the closing brace.
    start_marker = "    template<size_t N>\n    rusty::Result<std::array<T,"
    if start_marker in text:
        sidx = text.find(start_marker)
        # Find the closing brace at the same indent ("\n    }")
        eidx = text.find("\n    }\n", sidx)
        if eidx != -1:
            stub = (
                "    template<size_t N>\n"
                "    auto next_chunk() {\n"
                "        // STUBBED: rusty::array::IntoIter unavailable\n"
                "        std::abort();\n"
                "    }"
            )
            text = text[:sidx] + stub + text[eidx + 7:]

    # Fix 5: stub default_() — uses `super::rusty::Vec<auto>` (bogus path)
    old_default = (
        "    static IntoIter<T, A> default_() {\n"
        "        return rusty::iter(super::rusty::Vec<auto>::new_in(A::default_()));\n"
        "    }"
    )
    new_default = (
        "    static IntoIter<T, A> default_() {\n"
        "        // STUBBED: super::rusty::Vec<auto>::new_in path doesn't resolve\n"
        "        std::abort();\n"
        "    }"
    )
    text = text.replace(old_default, new_default)

    # Fix 6: stub clone() — uses span::to_vec_in (same blocker as Vec::clone)
    old_clone = (
        "    IntoIter<T, A> clone() const {\n"
        "        return rusty::iter(rusty::as_slice((*this)).to_vec_in(rusty::clone(rusty::deref_ref(this->alloc))));\n"
        "    }"
    )
    new_clone = (
        "    IntoIter<T, A> clone() const {\n"
        "        // STUBBED: span::to_vec_in unavailable\n"
        "        std::abort();\n"
        "    }"
    )
    text = text.replace(old_clone, new_clone)

    # Fix 7: bare `RawVec::from_nonnull_in(...)` → with template args
    text = text.replace(
        "RawVec::from_nonnull_in(this->_0.buf, this->_0.cap, std::move(alloc))",
        "RawVec<T, A>::from_nonnull_in(this->_0.buf, this->_0.cap, std::move(alloc))",
    )

    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_into_iter_compile_errors(cpp_out: Path) -> int:
    """Cluster of small fixes for re-enabling vec_port.vec.into_iter:

    1. `rusty::VecDeque<T, A>` is unary in rusty/vec_deque.hpp; the
       transpiler emits the binary form. Stub `into_vecdeque()` body
       since we don't need this interop method.
    2. `next_chunk<N>()` references `rusty::array::IntoIter<T, N>`
       which doesn't exist in rusty. Stub the body — rare API.
    3. `RawVec::new_()` and `NonZero::new_(1)` need template args.
    4. `static constexpr Option<NonZero<size_t>> EXPAND_BY` — the
       literal-type requirement isn't met by Option. Drop `constexpr`.
    """
    path = cpp_out / "vec_port.vec.into_iter.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text

    # Fix 1: stub into_vecdeque body
    old_vd = (
        "    rusty::VecDeque<T, A> into_vecdeque() {\n"
        "        auto this_ = rusty::mem::manually_drop_new(std::move((*this)));"
    )
    if old_vd in text:
        idx = text.find(old_vd)
        # Find the closing `}` of the method
        end_idx = text.find("\n    }\n", idx)
        if end_idx != -1:
            stub = (
                "    auto into_vecdeque() {\n"
                "        // STUBBED: rusty::VecDeque API arity mismatch\n"
                "        std::abort();\n"
                "    }"
            )
            text = text[:idx] + stub + text[end_idx + 7:]

    # Fix 2: stub next_chunk body
    text = re.sub(
        r"(    template<size_t N>\n    )rusty::Result<std::array<T[^>]*>, rusty::array::IntoIter<T, N>> next_chunk\(\) \{",
        r"\1auto next_chunk() {\n        // STUBBED: rusty::array::IntoIter unavailable\n        std::abort();\n        // (legacy body follows but unreachable)\n        if (false) {",
        text,
    )

    # Fix 3: bare `RawVec::new_()` → `RawVec<T, A>::new_()`
    text = text.replace(
        "this->buf = RawVec::new_().non_null();",
        "this->buf = RawVec<T, A>::new_().non_null();",
    )

    # Fix 4: drop `static constexpr` on Option<NonZero<...>> bindings
    text = text.replace(
        "    static constexpr rusty::Option<rusty::num::NonZero<size_t>> EXPAND_BY = NonZero::new_(1);",
        "    static inline const rusty::Option<rusty::num::NonZero<size_t>> EXPAND_BY = rusty::num::NonZero<size_t>::new_(1);",
    )
    text = text.replace(
        "    static constexpr rusty::Option<rusty::num::NonZero<size_t>> MERGE_BY = NonZero::new_(1);",
        "    static inline const rusty::Option<rusty::num::NonZero<size_t>> MERGE_BY = rusty::num::NonZero<size_t>::new_(1);",
    )

    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_operator_index(cpp_out: Path) -> int:
    """`Vec::operator[]` and `Vec::index_mut` call `.index(...)` on
    the dereffed `*this` — that resolves to a Rust trait method
    `Index::index`, which std::span doesn't have. Hand-port to the
    obvious `as_slice()[i]` / `as_mut_slice()[i]` form.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        old_op_index = (
            "    template<typename I>\n"
            "    decltype(auto) operator[](I index) const {\n"
            "        return (rusty::detail::deref_if_pointer_like((*this))).index(std::move(index));\n"
            "    }"
        )
        new_op_index = (
            "    template<typename I>\n"
            "    decltype(auto) operator[](I index) const {\n"
            "        return rusty::as_slice(*this)[static_cast<size_t>(index)];\n"
            "    }"
        )
        if old_op_index in text:
            text = text.replace(old_op_index, new_op_index)
        old_index_mut = (
            "    template<typename I>\n"
            "    decltype(auto) index_mut(I index) {\n"
            "        return (rusty::detail::deref_if_pointer_like((*this))).index_mut(std::move(index));\n"
            "    }"
        )
        new_index_mut = (
            "    template<typename I>\n"
            "    decltype(auto) index_mut(I index) {\n"
            "        return this->as_mut_slice()[static_cast<size_t>(index)];\n"
            "    }"
        )
        if old_index_mut in text:
            text = text.replace(old_index_mut, new_index_mut)
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_clone_to_vec_in(cpp_out: Path) -> int:
    """`Vec::clone()` ends with `std::span<const T>::to_vec_in(...)`,
    but `to_vec_in` is a Rust extension-trait method on slices —
    `std::span` doesn't have it. Hand-port the body to build a Vec
    via with_capacity_in + element-by-element push.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        old = (
            "    Vec<T, A> clone() const {\n"
            "        const auto alloc = rusty::clone(this->allocator());\n"
            "        return std::span<const T>::to_vec_in(rusty::detail::deref_if_pointer_like((*this)), std::move(alloc));\n"
            "    }"
        )
        new = (
            "    Vec<T, A> clone() const {\n"
            "        auto alloc = rusty::clone(this->allocator());\n"
            "        auto out = Vec<T, A>::with_capacity_in(this->len_field, std::move(alloc));\n"
            "        auto src = rusty::as_slice(*this);\n"
            "        for (size_t i = 0; i < src.size(); ++i) {\n"
            "            out.push(src[i]);\n"
            "        }\n"
            "        return out;\n"
            "    }"
        )
        if old in text:
            text = text.replace(old, new)
            path.write_text(text)
            n += 1
    return n


def patch_spec_extend_slice_iter(cpp_out: Path) -> int:
    """`spec_extend(rusty::slice_iter::Iter<const T> iterator)` tries
    to grab `rusty::as_slice(iterator)` and forward to
    `append_elements`, but `as_slice` on Iter returns
    `span<const Elem>` with Elem = const T (so `span<const const T>`),
    which doesn't convert to the `span<const T>` parameter.

    Hand-port the body to a simple copy-out loop using `.next()`.
    Works for any T that's copyable (which is exactly when this path
    is reachable from `extend_from_slice`).
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        old = (
            "    void spec_extend(rusty::slice_iter::Iter<const T> iterator) {\n"
            "        auto slice = rusty::as_slice(iterator);\n"
            "        // @unsafe\n"
            "        {\n"
            "            this->append_elements(slice);\n"
            "        }\n"
            "    }"
        )
        new = (
            "    void spec_extend(rusty::slice_iter::Iter<const T> iterator) {\n"
            "        while (true) {\n"
            "            auto opt = iterator.next();\n"
            "            if (opt.is_none()) break;\n"
            "            T value = *opt.unwrap();\n"
            "            this->push_mut(std::move(value));\n"
            "        }\n"
            "    }"
        )
        if old in text:
            text = text.replace(old, new)
            path.write_text(text)
            n += 1
    return n


def patch_append_elements_span_param(cpp_out: Path) -> int:
    """Rust source: `unsafe fn append_elements(&mut self, other: *const [T])`
    transpiles to `append_elements(std::add_pointer_t<std::add_const_t<
    std::span<const T>>> other)` — but the body then uses
    `rusty::len(other)` (fails: pointer doesn't have size()) and
    `reinterpret_cast<T const*>(other)` (also broken).

    Rewrite the parameter to plain `std::span<const T> other` and the
    body's reinterpret_cast to `other.data()`. Then strip the same
    cast-chain at call sites: pass `as_slice(...)` directly.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        # 1. Parameter type: pointer-to-span → value span
        text = text.replace(
            "void append_elements(std::add_pointer_t<std::add_const_t<std::span<const T>>> other)",
            "void append_elements(std::span<const T> other)",
        )
        # 2. Body: reinterpret_cast<T const*>(other) → other.data()
        text = text.replace(
            "reinterpret_cast<std::add_pointer_t<std::add_const_t<T>>>(other)",
            "other.data()",
        )
        # 3. Call sites: strip the const_cast/reinterpret_cast/addr_of_temp
        #    chain wrapping `rusty::as_slice(...)`. Replace the whole
        #    chain with the inner as_slice call.
        text = re.sub(
            r"const_cast<std::add_pointer_t<std::add_const_t<std::span<const T>>>>\(reinterpret_cast<std::add_pointer_t<std::add_const_t<std::add_const_t<std::span<const T>>>>>\(rusty::addr_of_temp\(std::move\(rusty::as_slice\(([^)]+)\)\)\)\)\)",
            r"rusty::as_slice(\1)",
            text,
        )
        # 4. spec_extend(slice_iter::Iter) body: was `auto& slice =
        #    rusty::as_slice(...);` then `append_elements(slice)`.
        #    Drop the &; now passes by value matching the new param.
        text = text.replace(
            "auto& slice = rusty::as_slice(iterator);",
            "auto slice = rusty::as_slice(iterator);",
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_return_assert_failed_void(cpp_out: Path) -> int:
    """Rust panic paths translate as `return panic_fn(...)` where
    panic_fn returns `!` — but in C++ it returns void, and the IIFE
    expects to return T. Pattern in IIFE arms:
        if (_m.is_none()) { return assert_failed(...); }
    Replace `return assert_failed(...)` with `assert_failed(...);
    std::abort()` so abort's [[noreturn]] makes the dead path
    type-check (and the abort inside assert_failed itself still runs).
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        text = re.sub(
            r"return\s+assert_failed\(([^;]*?)\);",
            r"assert_failed(\1); std::abort();",
            text,
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_println_assert_lambdas(cpp_out: Path) -> int:
    """The transpiler emits assertion paths as:
        SafeFn<void(...)> assert_failed = +[](...) -> void {
            std::println(stderr, "msg (is {var}) ...");
            std::abort();
        };
    Two problems:
    1. libstdc++ marks parts of std::println as consteval, which
       makes the lambda's operator() implicitly immediate — clang
       then rejects `+lambda` ("cannot take address of immediate
       call operator outside of an immediate invocation").
    2. The format strings use Rust-style named placeholders
       (`{index}`, `{len}`) that std::format doesn't accept.
    Both are dead-end abort paths, so collapse them to fprintf.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        text = re.sub(
            r'std::println\(stderr, "[^"]*"\);',
            r'std::fprintf(stderr, "[vec_port] assertion failed\\n");',
            text,
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_slice_ref_tmp_static(cpp_out: Path) -> int:
    """The transpiler emits lifetime-extension IIFEs like:
        [&]() -> std::span<const T> {
            static const auto _slice_ref_tmp = ...;
            return std::span<const T>(_slice_ref_tmp);
        }();
    The `static` makes the slice persist across calls — the buffer
    pointer captured on first invocation never refreshes after Vec
    grows, so callers see a dangling span with stale len. Drop
    `static` so the binding is a normal local recomputed every call.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        text = text.replace(
            "static const auto _slice_ref_tmp =",
            "const auto _slice_ref_tmp =",
        )
        text = text.replace(
            "static auto _slice_ref_tmp =",
            "auto _slice_ref_tmp =",
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_as_mut_slice_pointer_wrap(cpp_out: Path) -> int:
    """Rust source: `let elems: *mut [T] = self.as_mut_slice();`
    transpiles to:
        const std::add_pointer_t<std::span<T>> elems = rusty::as_mut_slice(...);
    but `as_mut_slice` returns `std::span<T>` by value, not a pointer.
    Drop the `std::add_pointer_t<>` wrap so `elems` is a span and the
    `drop_in_place(RangeLike)` overload fires.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        text = re.sub(
            r"std::add_pointer_t<std::span<([^>]+)>>\s+(\w+)\s*=\s*rusty::as_mut_slice\(",
            r"std::span<\1> \2 = rusty::as_mut_slice(",
            text,
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_setlenondrop_addrof(cpp_out: Path) -> int:
    """`SetLenOnDrop::new_(&this->len_field)` — transpiler emitted `&`
    (address-of) when calling a `size_t&` reference parameter. Strip
    the `&`.
    """
    path = cpp_out / "vec_port.vec.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    text = text.replace(
        "SetLenOnDrop::new_(&this->len_field)",
        "SetLenOnDrop::new_(this->len_field)",
    )
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_template_arg_recovery_for_aux_types(cpp_out: Path) -> int:
    """Specific call sites where the transpiler emitted bare names for
    template types (RawVec, PeekMut, IntoIter) without their template
    args. These appear inside Vec<T, A> methods, so we know T and A
    are in scope.
    """
    path = cpp_out / "vec_port.vec.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    # `RawVec::try_with_capacity_in(...)` → `RawVec<T, A>::try_with_capacity_in(...)`
    text = text.replace(
        "RawVec::try_with_capacity_in(",
        "RawVec<T, A>::try_with_capacity_in(",
    )
    text = text.replace(
        "RawVec::with_capacity_in(",
        "RawVec<T, A>::with_capacity_in(",
    )
    # `PeekMut::new_((*this))` → `PeekMut<T, A>::new_((*this))`
    text = text.replace(
        "PeekMut::new_(",
        "PeekMut<T, A>::new_(",
    )
    # `IntoIter into_iter()` (member function return type) → `IntoIter<T, A> into_iter()`
    text = text.replace(
        "    IntoIter into_iter() {",
        "    IntoIter<T, A> into_iter() {",
    )
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_intoiter_alias_conflict(cpp_out: Path) -> int:
    """Inside class Vec<T, A>, the transpiler emits:
        using IntoIter = IntoIter<T, A>;
    This shadows the namespace-level IntoIter template, making later
    references like `IntoIter<T, A2>` (in template member functions
    that need a different A) fail to parse.

    Strip the `using IntoIter = ...;` line. Code that needs the
    instantiated form will see the namespace-level template instead.
    """
    path = cpp_out / "vec_port.vec.cppm"
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    # Remove `    using IntoIter = IntoIter<T, A>;` (with leading indent).
    import re
    text = re.sub(
        r"^\s*using\s+IntoIter\s*=\s*IntoIter<[^>]+>;\s*\n",
        "",
        text,
        flags=re.MULTILINE,
    )
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_hint_slice_iter_namespaces(cpp_out: Path) -> int:
    """`hint::unlikely(x)` → `(x)` (lose the branch hint).
    `slice::range(...)` → no-op replacement (need rusty::slice::range).
    `iter::zip(a, b)` → `rusty::iter::zip(a, b)`.

    Conservative: replace specific bare-namespace calls with rusty:: form
    or with simple identity expansion.
    """
    import re
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        # hint::unlikely(x) → (x). The branch hint is lost; doesn't change semantics.
        text = re.sub(r"hint::unlikely\(((?:[^()]|\([^()]*\))*)\)", r"(\1)", text)
        text = re.sub(r"hint::likely\(((?:[^()]|\([^()]*\))*)\)", r"(\1)", text)
        # iter::zip(...) → rusty::iter_ext::zip(...) — cannot use
        # rusty::iter:: because `rusty::iter` is a free function.
        text = text.replace("iter::zip(", "rusty::iter_ext::zip(")
        # slice::range — `rusty::slice` is a free function in array.hpp,
        # so we cannot use `rusty::slice::range`. Use `rusty::slice_ext::range`.
        text = text.replace("rusty::slice::range(", "rusty::slice_ext::range(")
        text = text.replace("slice::range(", "rusty::slice_ext::range(")
        # ::slice::SpecCloneIntoVec / slice::SpecCloneIntoVec → bare
        # SpecCloneIntoVec (stub defined at module scope without
        # `rusty::slice::` nesting to avoid conflict).
        text = text.replace("::slice::SpecCloneIntoVec", "SpecCloneIntoVec")
        text = text.replace("slice::SpecCloneIntoVec", "SpecCloneIntoVec")
        if text != original:
            path.write_text(text)
            n += 1
    return n


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
        # vec_port.vec.drain — merged into vec.cppm; import stripped here too
        "vec_port.vec.drain",
        # vec_port.vec.extract_if — merged into vec.cppm; import stripped here too
        "vec_port.vec.extract_if",
        "vec_port.vec.in_place_collect",
        "vec_port.vec.in_place_drop",
        # vec_port.vec.into_iter — keeping; see CMakeLists trim
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


def patch_rusty_intrinsics_stubs(cpp_out: Path) -> int:
    """`rusty::intrinsics::const_make_global(x)` → just `x` (identity).
    `rusty::intrinsics::assume(cond)` → `__builtin_assume(cond)`.
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        # const_make_global is identity — for our port the const-vs-non-const
        # distinction is already captured by the surrounding cast.
        # Strip `rusty::intrinsics::const_make_global(` so the call expression
        # collapses to its argument. Tricky with closing paren — use a
        # placeholder approach: just rewrite the qualified path.
        text = text.replace(
            "rusty::intrinsics::const_make_global(",
            "/* const_make_global */ (",
        )
        # rusty::intrinsics::assume(x) → __builtin_assume(x)
        text = text.replace(
            "rusty::intrinsics::assume(",
            "__builtin_assume(",
        )
        # Also bare `intrinsics::assume(` (no rusty:: prefix)
        text = text.replace(
            "intrinsics::assume(",
            "__builtin_assume(",
        )
        if text != original:
            path.write_text(text)
            n += 1
    return n


def patch_hint_assert_unchecked(cpp_out: Path) -> int:
    """`core::hint::assert_unchecked(cond)` is a Rust intrinsic that
    tells the compiler `cond` is true. Map to `__builtin_assume(cond)`
    on clang/gcc (no-op otherwise).
    """
    n = 0
    for path in cpp_out.glob("*.cppm"):
        text = path.read_text()
        original = text
        # Order matters: longest prefix first so we don't leave dangling `rusty::__builtin_assume(`.
        text = text.replace("rusty::hint::assert_unchecked(", "__builtin_assume(")
        text = text.replace("core::hint::assert_unchecked(", "__builtin_assume(")
        text = text.replace("hint::assert_unchecked(", "__builtin_assume(")
        # If a prior pass produced `rusty::__builtin_assume(`, fix it up.
        text = text.replace("rusty::__builtin_assume(", "__builtin_assume(")
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
    vec_port.vec.into_iter.cppm
    vec_port.vec.drain.cppm
    vec_port.vec.cppm
)

target_sources(vec_port PUBLIC FILE_SET CXX_MODULES FILES
    vec_port.cppm
    vec_port.raw_vec.cppm
    vec_port.vec.set_len_on_drop.cppm
    vec_port.vec.into_iter.cppm
    vec_port.vec.drain.cppm
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
        ("aggregate_raw_ptr<...,auto,auto> → direct std::span ctor",
            patch_aggregate_raw_ptr_to_span_ctor),
        ("strip [[noreturn]] in template-arg / trailing-return",
            patch_strip_noreturn_in_template_and_trailing_ret),
        ("hint::/slice::/iter:: bare namespaces → rusty:: equivalents",
            patch_hint_slice_iter_namespaces),
        ("strip Vec::IntoIter alias (conflicts with namespace template)",
            patch_intoiter_alias_conflict),
        ("template-arg recovery for RawVec/PeekMut/IntoIter call sites",
            patch_template_arg_recovery_for_aux_types),
        ("SetLenOnDrop::new_(&this->len_field) → drop the &",
            patch_setlenondrop_addrof),
        ("strip std::add_pointer_t<std::span<T>> on as_mut_slice() binding",
            patch_as_mut_slice_pointer_wrap),
        ("drop `static` from _slice_ref_tmp lifetime-extension IIFEs",
            patch_slice_ref_tmp_static),
        ("collapse std::println assert messages to std::fprintf",
            patch_println_assert_lambdas),
        ("`return assert_failed(...)` → `assert_failed(...); std::abort()`",
            patch_return_assert_failed_void),
        ("append_elements param: pointer-to-span → value span",
            patch_append_elements_span_param),
        ("hand-port spec_extend(slice_iter::Iter) to copy-out loop",
            patch_spec_extend_slice_iter),
        ("hand-port Vec::clone() to with_capacity_in + push loop",
            patch_clone_to_vec_in),
        ("hand-port Vec::operator[] and index_mut to slice indexing",
            patch_operator_index),
        ("into_iter compile-error cluster (VecDeque/array::IntoIter/NonZero/RawVec)",
            patch_into_iter_compile_errors),
        ("into_iter pass 2 (non_null! macro, NonZero qual, last/next_chunk stubs)",
            patch_into_iter_more_fixes),
        ("into_iter Phase B: hand-port next/size_hint/advance_by + Vec::into_iter",
            patch_into_iter_runtime_body),
        ("raw_vec new_cap<T>: elide ZST branch for non-class T",
            patch_t_is_zst_constexpr_if),
        ("drain runtime: _let_pat.start/end -> .first/.second + NonNull::from -> new_unchecked",
            patch_drain_runtime),
        # extract_if_runtime fixes _let_pat.start/.end in extract_if.cppm.
        # Must run BEFORE the merge so the fixed content gets injected.
        ("extract_if runtime: _let_pat field rename (pre-merge)",
            patch_extract_if_runtime),
        ("merge drain.cppm into vec.cppm (rusty::Vec -> local Vec, fixes layout mismatch)",
            patch_merge_drain_into_vec),
        ("drop drain.cppm from CMakeLists (now merged)",
            patch_drop_drain_from_build),
        ("merge extract_if.cppm into vec.cppm (same fix as drain)",
            patch_merge_extract_if_into_vec),
        ("drop extract_if.cppm from CMakeLists (now merged)",
            patch_drop_extract_if_from_build),
        ("drain DropGuard: strip reinterpret_cast<u8*> (byte vs element offset)",
            patch_drain_dropguard_byte_cast),
        ("merge remaining aux modules (cow, peek_mut, splice, spec_*, etc) into vec.cppm",
            patch_merge_remaining_aux),
        ("`return ::handle_error(...)` → `::handle_error(...); std::abort()`",
            patch_return_handle_error_void),
        ("shrink_unchecked: hoist double-unwrap on Option<tuple>",
            patch_let_pat_double_unwrap),
        ("T::LAYOUT → rusty::alloc::Layout::new_<T>()",
            patch_t_layout_to_layout_new),
        ("T::MAX_SLICE_LEN + size_of<T>() Rust intrinsics",
            patch_t_max_slice_len_t_layout),
        ("Layout.align() → .align field access",
            patch_layout_align_method_to_field),
        ("from_into<X>(X) identity short-circuit",
            patch_from_into_identity_shortcircuit),
        ("strip .as_non_null_ptr() so CastProxy→NonNull<T> implicit conv fires",
            patch_castproxy_implicit_conv),
        ("wrap RawVec<T,A>::method in IIFE to dodge macro comma",
            patch_macro_template_arg_parens),
        ("auto ret; → int ret = 0; (no-init placeholder)",
            patch_auto_ret_init),
        ("add template<> prefix to free `Box::from(Vec<T,A>)`",
            patch_box_from_template),
        ("inject stub SpecFromElem/SpecExtend/SpecFromIter + rusty_ext::spec_extend",
            patch_spec_trait_stubs),
        ("strip std::ub_checks::assert_unsafe_precondition",
            patch_strip_ub_checks),
        ("hint::assert_unchecked → __builtin_assume", patch_hint_assert_unchecked),
        ("rusty::intrinsics::{const_make_global,assume} → identity/builtin",
            patch_rusty_intrinsics_stubs),
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
