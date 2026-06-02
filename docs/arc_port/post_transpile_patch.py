#!/usr/bin/env python3
"""Post-transpile patches for the arc_port C++20 module port.

arc.rs is the atomics-heavy sibling of rc.rs — almost the same shape.
We re-use the rc_port patcher's namespace fixups verbatim. The deeper
blockers (Arc<T, A> two-template-arg vs hand-written rusty::Arc<T>,
Cluster A regression, NonNull::cast<>, missing memory-ordering
helpers) are arc-specific and aren't addressed here. See STATUS.md
for the punch list.

Usage:
    python3 post_transpile_patch.py <cpp_out_dir>
"""

import re
import sys
from pathlib import Path


ARC_FILE = "arc_port.cppm"


def patch_namespace_using_prefix(cpp_out: Path) -> int:
    """Same namespace-fixup as rc_port: borrow/string/Vec/ptr::Alignment/
    mem::MaybeUninit re-qualification."""
    path = cpp_out / ARC_FILE
    if not path.exists():
        return 0
    text = path.read_text()
    original = text

    text = re.sub(r"using ::?std::borrow::", "using rusty::borrow::", text)
    text = re.sub(r"using ::string::", "using rusty::", text)
    text = re.sub(r"^using rusty::Vec;", "using ::Vec;", text, flags=re.MULTILINE)
    text = re.sub(r"(?<![A-Za-z0-9_])rusty::Vec<", "::Vec<", text)
    text = text.replace("std::ptr::Alignment", "rusty::ptr::Alignment")
    text = text.replace("rusty::mem::MaybeUninit", "rusty::MaybeUninit")
    text = re.sub(
        r"(?<![A-Za-z0-9_:])mem::MaybeUninit",
        "rusty::MaybeUninit", text)
    text = re.sub(
        r"^using rusty::borrow::(Cow|ToOwned);$",
        r"// using rusty::borrow::\1; — borrow module not vendored",
        text, flags=re.MULTILINE)

    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_drop_misplaced_module_imports(cpp_out: Path) -> int:
    path = cpp_out / ARC_FILE
    if not path.exists():
        return 0
    text = path.read_text()
    original = text
    text = re.sub(
        r"^import arc_port\.\w+;\s*$",
        "// import arc_port.<sub>; — sub-module not vendored",
        text, flags=re.MULTILINE)
    if text != original:
        path.write_text(text)
        return 1
    return 0


def patch_arc_specific(cpp_out: Path) -> int:
    """Arc-specific rewrites (atop the namespace-prefix patches).

    Mirrors many rc_port rules — see docs/rc_port/post_transpile_patch.py
    for the prose explanations. Each rule below is the arc-side variant.
    """
    path = cpp_out / ARC_FILE
    if not path.exists():
        return 0
    text = path.read_text()
    original = text

    # `using ::rc::is_dangling;` / `using ::Vec;` — these reference
    # things in the global namespace that don't exist. Comment out.
    text = re.sub(
        r"^using ::rc::is_dangling;\s*$",
        "// using ::rc::is_dangling; — rc module not vendored; "
        "arc_port has its own is_dangling helper if needed",
        text, flags=re.MULTILINE)
    text = re.sub(
        r"^using ::Vec;\s*$",
        "// using ::Vec; — Vec comes from vec_port via include path",
        text, flags=re.MULTILINE)

    # `rusty::sync::atomic::Atomic<size_t>` — main moved Atomic into
    # `detail::` namespace; concrete aliases like `AtomicUsize` remain
    # at the public path. Rewrite uses to the detail-qualified name.
    text = re.sub(
        r"rusty::sync::atomic::Atomic<",
        "rusty::sync::atomic::detail::Atomic<",
        text)

    # `rusty::Weak<T, A>` inside this file refers to the arc-side
    # Weak struct that lives in the same TU (arc_port::Weak). The
    # transpiler emitted `rusty::Weak` because Rust's sync.rs uses
    # `Weak` as a bare name (imported at module scope). Re-qualify
    # to `arc_port::Weak`.
    text = re.sub(
        r"(?<![A-Za-z0-9_:])rusty::Weak<",
        "arc_port::Weak<",
        text)

    # `, rusty::alloc::Global)` — Global is a type, not a value.
    # Brace-init the default value.
    text = re.sub(r"\brusty::alloc::Global\)", "rusty::alloc::Global{})", text)
    text = re.sub(r"\brusty::alloc::Global,", "rusty::alloc::Global{},", text)
    text = re.sub(
        r"\brusty::alloc::Global\.allocate",
        "rusty::alloc::Global{}.allocate", text)

    # `, ::cast))` — Rust passed a cast helper function. We don't
    # have one; pass a passthrough lambda.
    text = re.sub(
        r",\s*::cast\)",
        ", [](auto&& __x) { return std::forward<decltype(__x)>(__x); })",
        text)

    # `::data_offset<...>` — free helper in arc_port namespace; drop
    # the `::` prefix so name lookup finds it locally.
    text = re.sub(
        r"(?<![A-Za-z0-9_:])::data_offset<",
        "data_offset<",
        text)

    # `Arc<T, A>::arcinner_layout_for_value_layout(...)` — same
    # pattern as rc_port's `rc_inner_layout_for_value_layout`: free
    # helper, not a member.
    text = re.sub(
        r"\bArc<T,\s*A>::arcinner_layout_for_value_layout\b",
        "arcinner_layout_for_value_layout",
        text)

    # `ptr::slice_from_raw_parts_mut` → `rusty::from_raw_parts_mut`.
    text = re.sub(
        r"(?<![A-Za-z0-9_])ptr::slice_from_raw_parts_mut\b",
        "rusty::from_raw_parts_mut",
        text)

    # `hint::spin_loop()` (bare) → no-op. The standalone `hint`
    # namespace isn't visible here; spin_loop is a perf hint we can
    # safely no-op for correctness.
    text = re.sub(
        r"\bhint::spin_loop\(\);",
        "(void)0;  // patcher: hint::spin_loop() no-oped",
        text)
    # `rusty::hint::assert_unchecked(…)` → `(void)0`.
    text = re.sub(
        r"\brusty::hint::assert_unchecked\([^;]*\);",
        "(void)0;",
        text)

    # `NonNull<u8>::as_non_null_ptr()` — we expose `as_non_null_ptr`
    # only on the CastProxy, not directly. The transpiled call is
    # `recv.as_non_null_ptr()`. Stub via reinterpret-cast.
    text = re.sub(
        r"\.as_non_null_ptr\(\)",
        ".as_ptr()  /* patcher: as_non_null_ptr → as_ptr */",
        text)

    # `(rusty::)?ptr::addr_eq(a, b)` — pointer-address comparison
    # intrinsic. Use plain `==` on cast-to-uintptr values. The emit
    # appears both fully-qualified and via the file-scope
    # `using rusty::ptr` brought into scope.
    text = re.sub(
        r"(?:rusty::)?ptr::addr_eq\(([^,]+),\s*([^)]+)\)",
        r"(reinterpret_cast<std::uintptr_t>(\1) == reinterpret_cast<std::uintptr_t>(\2))",
        text)

    # NOTE: earlier patcher iterations had a self-destructive
    # `Box<auto>::try_new(` ↔ `Box<std::remove_cvref_t<decltype(`
    # rewrite + "revert" pair that corrupted the transpiler's
    # already-correct `Box<decltype(...)>::into_unique(...)` emit.
    # Removed once codegen.rs learned `try_new` / `try_new_in` /
    # `new_in` etc. in the explicit Box arg-inference branch
    # (alongside `new` / `new_` / `make`).
    #
    # `Box<auto>::new_uninit()` (zero-arg) inside `Default::default()` —
    # only one site survives codegen.rs. The surrounding shape is
    # `Box<decltype(Box<auto>::new_uninit())>::write_(Box<auto>::new_uninit(),
    # ArcInner<T>{...})`. Rust deduces T from the second arg; we can
    # do the same by replacing both `Box<auto>` with the concrete
    # `Box<MaybeUninit<ArcInner<T>>>` here.
    text = re.sub(
        r"rusty::Box<std::remove_cvref_t<decltype\(\(rusty::Box<auto>::new_uninit\(\)\)\)>>::write_"
        r"\(rusty::Box<auto>::new_uninit\(\),\s*ArcInner<T>\{",
        "rusty::Box<rusty::MaybeUninit<ArcInner<T>>>::write_("
        "rusty::Box<rusty::MaybeUninit<ArcInner<T>>>::new_uninit(), "
        "ArcInner<T>{",
        text)

    # Stub the orphan `provide(rusty::error::Request&)` method — the
    # Request type isn't in our rusty::error.
    text = re.sub(
        r"void provide\(rusty::error::Request& req\) const \{[^}]*\}",
        "void provide(const auto&) const {}  // patcher: Error::provide stubbed",
        text,
        flags=re.DOTALL)

    # Free-function `from(arc_port::Arc<T, A>)` — same shape as the
    # rc_port `from(VecDeque)` issue: emitted at file scope without
    # template prefix.
    text = re.sub(
        r"^static auto from\(arc_port::Arc<T, A> other\)\s*\{[^}]*?\n\}",
        "// patcher: free-fn from(Arc<T,A>) stubbed (template params not in scope)",
        text,
        flags=re.MULTILINE | re.DOTALL)

    # serde-de prelude visit_byte_buf — same stub shape as rc_port.
    STUBBED_VISIT_BYTE_BUF = (
        "rusty::Result<Value, E> visit_byte_buf(auto&&) "
        "{ return rusty::Result<Value, E>::Err(E{}); }"
    )
    if STUBBED_VISIT_BYTE_BUF not in text:
        text = re.sub(
            r"rusty::Result<Value, E> visit_byte_buf\([^)]+\)\s*\{[^}]*\}",
            STUBBED_VISIT_BYTE_BUF,
            text)

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
        ("arc-specific rewrites", patch_arc_specific),
    ]

    total = 0
    for name, fn in patches:
        n = fn(cpp_out)
        if n:
            print(f"  applied: {name}")
        total += n

    print(f"arc_port patches applied: {total}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
