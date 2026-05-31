#!/usr/bin/env python3
"""Post-transpile patches for the rc_port C++20 module port.

Same shape as docs/cell_port/post_transpile_patch.py: each patch
addresses a specific cluster of errors documented in STATUS.md.
Idempotent — rerunning detects already-applied patches and skips.

Usage:
    python3 post_transpile_patch.py <cpp_out_dir>
"""

import re
import sys
from pathlib import Path


RC_FILE = "rc_port.cppm"


def patch_namespace_using_prefix(cpp_out: Path) -> int:
    """Rewrite cross-crate namespace imports that landed at bare or
    ::std::-prefixed paths. The vendored Rust paths `std::borrow::*`,
    `string::*`, `core::ptr::Alignment` need `rusty::` qualification."""
    path = cpp_out / RC_FILE
    if not path.exists():
        return 0
    text = path.read_text()
    original = text

    # `using ::std::borrow::X;` and `using std::borrow::X;` — Rust paths
    # leaking as C++ `std::borrow::` (which doesn't exist).
    text = re.sub(r"using ::?std::borrow::", "using rusty::borrow::", text)

    # `using ::string::X;` — should be `using rusty::X;` since rusty's
    # String is at the top level of the namespace (not in a `string`
    # sub-namespace).
    text = re.sub(r"using ::string::", "using rusty::", text)

    # `using rusty::Vec;` — Vec is at global `::Vec<T,A>` after the
    # VecLegacy retirement, not in the rusty namespace.
    text = re.sub(r"^using rusty::Vec;", "using ::Vec;", text, flags=re.MULTILINE)

    # Same for `rusty::Vec<…>` type references in the body — Vec is now
    # global. Substitute everywhere it appears as a type prefix.
    text = re.sub(r"(?<![A-Za-z0-9_])rusty::Vec<", "::Vec<", text)

    # `std::ptr::Alignment` → `rusty::ptr::Alignment` (transpiler
    # treated `core::ptr` as if it lived in C++ `std::`).
    text = text.replace("std::ptr::Alignment", "rusty::ptr::Alignment")

    # `rusty::mem::MaybeUninit` → `rusty::MaybeUninit` (it's defined
    # at rusty's top level, not in `rusty::mem`). Also handle bare
    # `mem::MaybeUninit` for body sites where the transpiler dropped
    # the rusty:: prefix.
    text = text.replace("rusty::mem::MaybeUninit", "rusty::MaybeUninit")
    text = re.sub(
        r"(?<![A-Za-z0-9_:])mem::MaybeUninit",
        "rusty::MaybeUninit", text)

    # `rusty::borrow::Cow`/`ToOwned` and `rusty::Cow`/`ToOwned`
    # references — we don't vendor `core::borrow` so stub the using
    # decls. They're decorative imports; the actual rc_port surface
    # we care about doesn't depend on Cow conversions.
    text = re.sub(
        r"^using rusty::borrow::(Cow|ToOwned);$",
        r"// using rusty::borrow::\1; — borrow module not vendored",
        text, flags=re.MULTILINE)

    # `rusty::Rc<T, A>` / `rusty::Weak<T, A>` — these qualify to the
    # hand-written single-template-arg rusty::Rc<T> which doesn't
    # accept two args. Inside rc_port, the local two-arg `Rc<T, A>`
    # is the right reference. Drop the `rusty::` prefix.
    text = re.sub(r"(?<![A-Za-z0-9_])rusty::Rc<", "Rc<", text)
    text = re.sub(r"(?<![A-Za-z0-9_])rusty::Weak<", "Weak<", text)

    # `rusty::alloc::Global` used as a value instead of a type. Two
    # cases:
    #   - bare `rusty::alloc::Global)` (passed as an argument) →
    #     `rusty::alloc::Global{})` (default-construct an instance).
    #   - `rusty::alloc::Global.method(...)` (called as if instance) →
    #     `rusty::alloc::Global{}.method(...)`.
    text = re.sub(
        r"(?<![A-Za-z0-9_]):?:?rusty::alloc::Global\)",
        "rusty::alloc::Global{})", text)
    text = re.sub(
        r"(?<![A-Za-z0-9_]):?:?rusty::alloc::Global\.",
        "rusty::alloc::Global{}.", text)

    # Inject `import vec_port.vec;` into the module preamble so `::Vec`
    # references resolve. The transpiler emits cross-port imports
    # inline only when it knows the Cargo dep; for our offline pipeline
    # the patcher has to wire it up. Insert right after the
    # `export module rc_port;` line.
    if "import vec_port.vec;" not in text and "export module rc_port;" in text:
        text = text.replace(
            "export module rc_port;\n",
            "export module rc_port;\n\nimport vec_port.vec;  // patcher-injected for ::Vec\n",
            1)

    # `visit_byte_buf(::Vec<uint8_t>)` in the serde-de prelude lives in
    # the GMF, where module imports haven't kicked in — `Vec` isn't
    # visible. Same approach binary_heap_port took: stub the body.
    # Match the function signature flexibly to survive prefix-rewrite
    # iterations.
    text = re.sub(
        r"rusty::Result<Value, E> visit_byte_buf\([^)]+\)\s*\{[^}]*\}",
        "rusty::Result<Value, E> visit_byte_buf(auto&&) { return rusty::Result<Value, E>::Err(E{}); }",
        text)

    # One-arg `Rc<T>` / `Weak<T>` forward decls — local two-arg
    # template doesn't accept them. Targeted to the rusty_ext slot
    # `Rc<std::span<const T>> to_rc_slice(` shape that the transpiler
    # emits for to_rc_slice / to_arc_slice. Single literal match.
    text = text.replace(
        "Rc<std::span<const T>> to_rc_slice(",
        "Rc<std::span<const T>, rusty::alloc::Global> to_rc_slice(")
    text = text.replace(
        "Weak<std::span<const T>> to_weak_slice(",
        "Weak<std::span<const T>, rusty::alloc::Global> to_weak_slice(")

    # `rusty::ptr::slice_from_raw_parts_mut` and bare `ptr::*` — our
    # header surfaces it as top-level `rusty::from_raw_parts_mut`
    # (not in the `ptr` sub-namespace).
    text = text.replace(
        "rusty::ptr::slice_from_raw_parts_mut",
        "rusty::from_raw_parts_mut")
    text = re.sub(
        r"(?<![A-Za-z0-9_:])ptr::slice_from_raw_parts_mut",
        "rusty::from_raw_parts_mut", text)

    # Cluster A regression: `rusty::Box<auto>::FACTORY(RcInner<T>{...})`
    # — auto in template argument. The constructor argument's type IS
    # the box's value type, so substitute auto with the explicit type.
    # Uniform pattern in rc_port: arg is always RcInner<T> or ManuallyDrop.
    for factory in ("try_new", "new_in", "try_new_in", "write_"):
        text = text.replace(
            f"rusty::Box<auto>::{factory}(RcInner",
            f"rusty::Box<RcInner<T>>::{factory}(RcInner")
    # `Box<auto>::new_uninit()` returns Box<MaybeUninit<T>>.
    text = text.replace(
        "rusty::Box<auto>::new_uninit()",
        "rusty::Box<rusty::MaybeUninit<RcInner<T>>>::new_uninit()")
    # `Box<auto>::write_(MaybeUninit-box, RcInner<T>{})` — the
    # MaybeUninit-box has been substituted above, so Box<auto> here is
    # the MaybeUninit one (write_ is a method on MaybeUninit-box).
    text = text.replace(
        "rusty::Box<auto>::write_(rusty::Box<rusty::MaybeUninit<RcInner<T>>>",
        "rusty::Box<rusty::MaybeUninit<RcInner<T>>>::write_(rusty::Box<rusty::MaybeUninit<RcInner<T>>>")
    # `Box<auto>::into_raw_with_allocator(boxed)` / `into_unique(boxed)`
    # / `allocator(boxed)` — operate on existing Box<RcInner<T>> so
    # template arg is RcInner<T>.
    for op in ("into_raw_with_allocator", "into_unique", "allocator",
               "from_raw_in"):
        text = text.replace(
            f"rusty::Box<auto>::{op}(",
            f"rusty::Box<RcInner<T>>::{op}(")

    # Bare `::cast` (used as a function reference, typical pattern is
    # `.map_err(::cast)` or `.map(::cast)`) — replace with an identity
    # lambda so the call site at least type-checks. Phase B compile
    # only; the runtime path may need different handling for Phase C.
    text = text.replace(
        ", ::cast)",
        ", cast_identity_stub)")

    # `::data_offset<X>(...)` — qualified to global namespace, but the
    # actual decl is in rc_port. Drop the `::` so unqualified lookup
    # finds the in-namespace definition.
    text = text.replace(
        "::data_offset<",
        "data_offset<")

    # `NonNull<auto>::as_ptr(this_.ptr)` — Cluster A. `this_.ptr` is
    # NonNull<RcInner<T>>, so NonNull<auto> should be NonNull<RcInner<T>>.
    text = text.replace(
        "NonNull<auto>::as_ptr(this_.ptr)",
        "NonNull<RcInner<T>>::as_ptr(this_.ptr)")

    # `rusty::ptr::addr_eq` and bare `ptr::addr_eq` — not surfaced;
    # replace with rusty::ptr::eq which we just added.
    text = re.sub(
        r"(?<![A-Za-z0-9_:])(rusty::)?ptr::addr_eq\(",
        "rusty::ptr::eq(", text)

    # `rusty::ptr::from_ref` / `ptr::from_ref` — replace with
    # addr_of_temp which gives `T*` for `const T&`.
    text = re.sub(
        r"(?<![A-Za-z0-9_:])(rusty::)?ptr::from_ref\(",
        "rusty::addr_of_temp(", text)

    # `size_of_val(x)` Rust intrinsic — auto-deduces type via decltype.
    # The transpiled line `auto size_of_val = size_of_val(x)` is invalid
    # (variable shadows the intrinsic). Inject a sizeof fallback.
    text = re.sub(
        r"auto size_of_val = size_of_val\(([^)]+)\)",
        r"auto size_of_val = sizeof(decltype(\1))",
        text)

    # `Rc<T,A>::template is<T>(...)` — Rust trait `.is::<U>()` for
    # downcast checking on Any. We don't surface this method; the
    # call is in a Rust-only `Any` downcast path. Stub with false so
    # the surrounding branch becomes dead.
    text = re.sub(
        r"Rc<T, A>::template is<[^>]+>\([^)]+\)",
        "false /* Rc::is::<U>() trait method stubbed */",
        text)

    # `::rc_inner_layout_for_value_layout(...)` — bare qualifier, the
    # decl is inside Rc<T,A> as static member. Use Rc<T,A>::.
    text = text.replace(
        "::rc_inner_layout_for_value_layout(",
        "Rc<T, A>::rc_inner_layout_for_value_layout(")

    # `Layout::for_value_raw(ptr)` — our `rusty::alloc::Layout` only
    # surfaces `for_value`. Rename: for_value_raw is the unsafe variant
    # that takes a raw pointer; we approximate with for_value(*ptr).
    text = re.sub(
        r"Layout::for_value_raw\(([^)]+)\)",
        r"Layout::for_value(*\1)",
        text)

    # `rusty::num::NonZero<size_t>::MAX` — missing static member.
    # Substitute with the type's max value.
    text = text.replace(
        "rusty::num::NonZero<size_t>::MAX",
        "rusty::num::NonZero<size_t>::new_(SIZE_MAX).unwrap()")
    text = text.replace(
        "rusty::num::NonZero<unsigned long>::MAX",
        "rusty::num::NonZero<unsigned long>::new_(SIZE_MAX).unwrap()")
    # Then inject the helper lambda at the top of the module (just
    # after the namespace open). Idempotent.
    cast_stub = ("namespace { inline constexpr auto cast_identity_stub = "
                 "[](auto&& x) { return std::forward<decltype(x)>(x); }; "
                 "}\n")
    if "cast_identity_stub" in text and cast_stub not in text:
        # Add right after `namespace rc_port {`.
        text = text.replace(
            "namespace rc_port {\n",
            "namespace rc_port {\n" + cast_stub + "\n",
            1)

    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_drop_misplaced_module_imports(cpp_out: Path) -> int:
    """Same shape as cell_port: transpiler emits late `import
    rc_port.lazy/once/etc.;` for submodules we haven't vendored.
    Comment them and the dependent re-exports."""
    path = cpp_out / RC_FILE
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    text = re.sub(
        r"^import rc_port\.\w+;\s*$",
        "// import rc_port.<sub>; — sub-module not vendored",
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
        ("drop misplaced module imports", patch_drop_misplaced_module_imports),
    ]

    total = 0
    for name, fn in patches:
        n = fn(cpp_out)
        if n:
            print(f"  applied: {name}")
        total += n

    print(f"rc_port patches applied: {total}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
