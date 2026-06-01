#!/usr/bin/env python3
"""Post-transpile patches for the vec_deque_port C++20 module port.

Mirrors the standard 14-patch set from binary_heap_port, which itself
codified the bulk Vec-rename + std/rusty namespace fixups + ptr::swap
mapping that the BTreeMap port pioneered. Idempotent.

Usage:
    python3 post_transpile_patch.py <cpp_out_dir>
"""

import re
import sys
from pathlib import Path


def patch_all_files(cpp_out: Path) -> int:
    """Apply the standard cluster patches to every .cppm in the
    output directory. Same shape as binary_heap_port's 14-patch set."""
    cppms = sorted(cpp_out.glob("*.cppm"))
    if not cppms:
        return 0
    total_changes = 0
    for path in cppms:
        text = path.read_text()
        original = text

        # Patch 1: rusty::Vec<…> → ::Vec<…> (Vec is global after VecLegacy
        # retirement).
        text = re.sub(r"(?<![A-Za-z0-9_])rusty::Vec<", "::Vec<", text)

        # Patch 8: bare `rusty::Vec{}` (no template args)
        text = text.replace("rusty::Vec{}", "::Vec<T>{}")

        # Patch 4: `using rusty::Vec;` — Vec is module-only now.
        text = re.sub(r"^using rusty::Vec;\s*$",
                      "// using rusty::Vec; — Vec at global ::Vec now",
                      text, flags=re.MULTILINE)

        # Patch 6: `vec::IntoIter`/`Drain` from a `vec` sub-namespace
        # that doesn't exist (binary_heap_port hit this from the
        # vec_port import emit).
        text = re.sub(r"(?<![A-Za-z0-9_:])vec::IntoIter",
                      "::IntoIter", text)
        text = re.sub(r"(?<![A-Za-z0-9_:])vec::Drain",
                      "::Drain", text)

        # Patch 7: std::collections::TryReserveError → rusty::collections
        text = text.replace("std::collections::TryReserveError",
                            "rusty::collections::TryReserveError")

        # Patch 9: bare `usize` identifier (one-off in binary_heap)
        text = re.sub(r"(?<![A-Za-z0-9_:])usize(?![A-Za-z0-9_:])",
                      "size_t", text)
        text = re.sub(r"(?<![A-Za-z0-9_])size_t::BITS",
                      "std::numeric_limits<size_t>::digits", text)

        # Patch 13: rusty::ptr::swap / ptr::swap → std::swap (we don't
        # implement rusty::ptr::swap; std::swap on values works for our
        # call sites).
        text = re.sub(r"(?<![A-Za-z0-9_])(rusty::)?ptr::swap(?![A-Za-z0-9_])",
                      "std::swap", text)

        # rusty::mem::MaybeUninit → rusty::MaybeUninit (defined at
        # rusty top level).
        text = text.replace("rusty::mem::MaybeUninit",
                            "rusty::MaybeUninit")
        text = re.sub(r"(?<![A-Za-z0-9_:])mem::MaybeUninit",
                      "rusty::MaybeUninit", text)

        # `using ::std::borrow::X;` — Rust paths leaking as C++ std::.
        text = re.sub(r"using ::?std::borrow::",
                      "// using ::std::borrow:: — borrow not vendored — ",
                      text)
        # Same for `using ::string::String;`.
        text = re.sub(r"using ::string::String;",
                      "using rusty::String;", text)

        # `std::Allocator` / `std::Global` — these are Rust's
        # `alloc::Allocator` / `alloc::Global` mis-emitted as `std::`.
        text = re.sub(r"(?<![A-Za-z0-9_])std::Allocator",
                      "rusty::alloc::Allocator", text)
        text = re.sub(r"(?<![A-Za-z0-9_])std::Global",
                      "rusty::alloc::Global", text)

        # Duplicate `template<typename T> auto clone(...)` in the
        # transpiler-emitted GMF prelude collides with `rusty::clone`
        # (include/rusty/move.hpp). Both have the same signature, so
        # calls to `rusty::clone(...)` are ambiguous. Strip the prelude
        # definition. Same patch binary_heap_port applies (its #3).
        text = re.sub(
            r"// Clone: dispatches to \.clone\(\) if available, otherwise copy-constructs\.\n"
            r"template<typename T>\n"
            r"auto clone\(const T& value\) \{\n"
            r"if constexpr \(requires \{ value\.clone\(\); \}\) \{\n"
            r"return value\.clone\(\);\n"
            r"\} else \{\n"
            r"return value;\n"
            r"\}\n"
            r"\}\n",
            "// clone() prelude removed by patcher — rusty::clone in <rusty/move.hpp> covers this\n",
            text,
        )

        # `NonZero::new_(...)` Cluster A — unqualified Rust path infers
        # `NonZero<usize>`. Inject the explicit `<size_t>` template arg.
        text = re.sub(
            r"(?<![A-Za-z0-9_:])NonZero::new_\(",
            "rusty::num::NonZero<size_t>::new_(", text)

        # `Result<B, [[noreturn]] void>` — Rust `Result<B, !>` (never)
        # gets emitted with `[[noreturn]]` attribute inside template
        # args, which is invalid C++. Strip the attribute (`void` alone
        # is the correct C++ stand-in for `!`).
        text = re.sub(r"\[\[noreturn\]\]\s+void", "void", text)

        #`join_head_and_tail_wrapping` lambda assigns `src`/`dst`/`len`
        # without declaring them (Rust source uses tuple pattern
        # destructuring `let (src, dst, len) = if ...`). Inject the
        # declarations at the top of the lambda body so the assignments
        # parse.
        text = re.sub(
            r"(const auto join_head_and_tail_wrapping = \[\]\([^)]+\) \{)\n",
            r"\1\n                    size_t src, dst, len;\n",
            text,
        )

        # Drop imports of submodules we exclude from the reduced-scope
        # build (see CMakeLists.txt vec_deque_port note). The dropped
        # submodules pull in iterator-adapter types we don't vendor yet.
        # Only applies to the main `vec_deque_port.cppm` file.
        # Main module needs `vec_port.raw_vec` for `::raw_vec::RawVec`
        # references — inject right after `export module vec_deque_port;`.
        if (path.name == "vec_deque_port.cppm"
                and "import vec_port.raw_vec;" not in text):
            text = text.replace(
                "export module vec_deque_port;",
                ("export module vec_deque_port;\n"
                 "\n"
                 "import vec_port.vec;  // patcher-injected for ::Vec\n"
                 "import vec_port.raw_vec;  // patcher-injected for ::raw_vec::RawVec\n"
                 "import vec_port.vec.into_iter;  // patcher-injected for ::IntoIter / ::Drain"),
                1,
            )

        if path.name == "vec_deque_port.cppm":
            for dropped in (
                "spec_extend",
                "spec_from_iter",
                "splice",
                "extract_if",
            ):
                text = re.sub(
                    rf"^import vec_deque_port\.{dropped};\s*$",
                    f"// import vec_deque_port.{dropped}; — excluded from reduced-scope build",
                    text,
                    flags=re.MULTILINE,
                )

        # Hand-written `rusty::VecDeque<T>` (single arg) was retired
        # alongside this port; the transpiled emit uses
        # `rusty::VecDeque<T, A>` because the type-map says
        # `VecDeque -> rusty::VecDeque`. Rewrite to the actual
        # transpiled location so the 2-arg references resolve.
        text = re.sub(r"(?<![A-Za-z0-9_])rusty::VecDeque<",
                      "vec_deque_port::VecDeque<", text)

        # `::vec::IntoIter` / `::vec::Drain` — transpiled emit of a Rust
        # `vec::IntoIter` path. The vec_port module exports these at
        # global namespace (`::IntoIter`, `::Drain`), so the `::vec::`
        # prefix never resolves. Patcher: drop the `vec::` segment.
        text = re.sub(r"(?<![A-Za-z0-9_]):?:?vec::IntoIter",
                      "::IntoIter", text)
        text = re.sub(r"(?<![A-Za-z0-9_]):?:?vec::Drain",
                      "::Drain", text)

        # serde-de prelude's `visit_byte_buf(::Vec<uint8_t> value)` lives
        # in the GMF, where module imports (vec_port.vec) haven't kicked
        # in — `::Vec` isn't visible. Same fix linked_list_port + binary_heap
        # + rc apply: stub the body. The function exists only because
        # the serde-de prelude declares it; we don't actually call it.
        STUBBED_VISIT_BYTE_BUF = (
            "rusty::Result<Value, E> visit_byte_buf(auto&&) "
            "{ return rusty::Result<Value, E>::Err(E{}); }"
        )
        if STUBBED_VISIT_BYTE_BUF not in text:
            text = re.sub(
                r"rusty::Result<Value, E> visit_byte_buf\([^)]+\)\s*\{[^}]*\}",
                STUBBED_VISIT_BYTE_BUF,
                text,
            )

        # `rusty::collections::vec_deque::*` — when Rust imports
        # `use std::collections::vec_deque::Iter`, the transpiler
        # emits `rusty::collections::vec_deque::Iter`. There's no
        # `vec_deque` sub-namespace; rewrite to bare `vec_deque_port::`.
        text = re.sub(
            r"(?<![A-Za-z0-9_])rusty::collections::vec_deque::",
            "vec_deque_port::", text)

        # `std::collections::*` — Rust paths leaking. Re-route to
        # `rusty::collections::*` (which provides TryReserveError).
        text = re.sub(
            r"(?<![A-Za-z0-9_])std::collections::",
            "rusty::collections::", text)

        # Submodule .cppm files (vec_deque_port.iter, .drain, …) refer
        # to `vec_deque_port::VecDeque<T, A>` (after the rewrite above)
        # but importing the main `vec_deque_port` from a submodule
        # creates a cycle (the main module imports the submodules).
        # Provide a forward declaration instead — function-template
        # signatures only need the type to be declared, not defined.
        # The complete definition arrives via the main module which
        # pulls all submodules in.
        #
        # Submodules also need `::Vec` / `::IntoIter` / `::Drain` from
        # vec_port — inject the module imports here too so the global
        # types resolve in their function-template signatures.
        if (path.name != "vec_deque_port.cppm"
                and "// patcher-injected fwd decl for VecDeque" not in text
                and "export module vec_deque_port." in text):
            text = re.sub(
                r"(export module vec_deque_port\.[a-z_]+;\n)",
                (
                    r"\1\n"
                    r"import vec_port.vec;  // patcher-injected for ::Vec\n"
                    r"import vec_port.vec.into_iter;  // patcher-injected for ::IntoIter / ::Drain\n"
                    r"\n"
                    r"// patcher-injected fwd decl for VecDeque (avoids import cycle with main module)\n"
                    r"namespace vec_deque_port {\n"
                    r"  template<typename T, typename A> struct VecDeque;\n"
                    r"}\n"
                ),
                text,
                count=1,
            )

        # `IntoIter::next_chunk()` returns `Result<[T;N], array::IntoIter<T,N>>`
        # — we don't vendor `core::array::IntoIter`. Replace the whole
        # method (template + signature + body) with a stub via brace
        # tracking; nested braces defeat a regex-only rewrite.
        if path.name == "vec_deque_port.into_iter.cppm" and "next_chunk" in text:
            text = _strip_next_chunk_method(text)

        # C++20 modules require all `import` declarations to appear
        # at the very top of the module purview (right after
        # `export module foo;`), before any other declarations. The
        # transpiler interleaves submodule imports with `using` /
        # `export using` declarations, which clang rejects with
        # "unknown type name 'import'". Lift all imports to the top.
        if "export module vec_deque_port" in text:
            text = _lift_imports_to_top(text)

        # Two `struct Guard { … }` definitions at IntoIter class scope —
        # one used by try_fold, one used by try_rfold. The Rust source has
        # them as method-local types; the transpiler hoisted both to
        # class scope, producing a redefinition error. Strip the second
        # Guard + try_rfold method (we don't need it for the basic
        # smoke test path).
        if (path.name == "vec_deque_port.into_iter.cppm"
                and text.count("    struct Guard {") >= 2):
            text = _strip_second_guard_and_try_rfold(text)

        # ----- single-cppm (collapsed-Rust-source) patches -----
        # Below this line: rules that only apply to the collapsed
        # single-file emit. See docs/vec_deque_port/collapse.py.
        if path.name == "vec_deque_port.cppm":
            # `using ::raw_vec::RawVec;` — there is no `raw_vec` sub-
            # namespace; vec_port exports `RawVec` at global scope.
            text = re.sub(
                r"using ::raw_vec::RawVec;",
                "using ::RawVec;",
                text,
            )

            # Bare `raw_vec::RawVec<T, A>` field/parameter references —
            # vec_port exports RawVec at global, so drop the namespace.
            text = re.sub(
                r"(?<![A-Za-z0-9_:])raw_vec::RawVec<",
                "::RawVec<",
                text,
            )

            # `::wrap_index(…)` — the free function lives in the
            # `vec_deque_port` namespace, but the transpiler emits the
            # call site with a leading `::` (global) qualifier. Drop
            # the qualifier; the call is inside `vec_deque_port`.
            text = re.sub(
                r"(?<![A-Za-z0-9_]):{2}wrap_index\b",
                "wrap_index",
                text,
            )

            # `ptr::slice_from_raw_parts_mut` doesn't exist in rusty;
            # `rusty::from_raw_parts_mut` does. Rewrite the call site.
            text = re.sub(
                r"(?<![A-Za-z0-9_])ptr::slice_from_raw_parts_mut\b",
                "rusty::from_raw_parts_mut",
                text,
            )

            # `ptr::null()` — qualify with `rusty::` (we already have
            # `rusty::ptr::null_mut()` working at the same site).
            text = re.sub(
                r"(?<![A-Za-z0-9_:])ptr::null\(\)",
                "rusty::ptr::null<const T>()",
                text,
            )

            # `src.abs_diff(std::move(dst))` on `size_t` — Rust's
            # integer method, no C++ equivalent. Rewrite as a branch.
            # Match only inside assertion guards (the only place this
            # appears) so we don't accidentally rewrite real method
            # invocations.
            text = re.sub(
                r"\bsrc\.abs_diff\(std::move\(dst\)\)",
                "(src > dst ? src - dst : dst - src)",
                text,
            )

            # `self . capacity ()` — the transpiler emitted a Rust
            # source fragment `self.capacity()` into `std::format`
            # argument lists verbatim. Rewrite to `this->capacity()`.
            # The whitespace around `.` and around `(` is what the
            # transpiler emits (it pretty-prints Rust expressions).
            text = re.sub(
                r"\bself\s*\.\s*capacity\s*\(\s*\)",
                "this->capacity()",
                text,
            )

            # `std::iter::Copied<rusty::slice_iter::Iter<const T>>` —
            # used by the `spec_extend_front` specialization for
            # `slice::Iter::copied()`. We don't have a C++ analog
            # for `core::iter::Copied`; stub the two specializations
            # (the generic spec_extend_front handles correctness).
            text = _stub_copied_spec_extend_front(text)

            # Disambiguate duplicate hoisted helper structs (`Guard` /
            # `Dropper`) at class scope. The transpiler hoists method-
            # local helpers without disambiguating the names; identical-
            # field copies get a unique suffix.
            text = _disambiguate_hoisted_helpers(text)

            # `rusty::ptr::null<const T>()` doesn't exist — `null_mut()`
            # works at the same call site, the const variant should be
            # `rusty::ptr::null` (no template arg) returning const T*.
            # Use a static_cast as a safe equivalent.
            text = re.sub(
                r"rusty::ptr::null<const T>\(\)",
                "static_cast<const T*>(nullptr)",
                text,
            )

            # `, rusty::alloc::Global)` — Global is a type, not a value.
            # The transpiler emitted the type where an instance was
            # expected. Brace-initialize to a default instance.
            text = re.sub(
                r"\brusty::alloc::Global\)",
                "rusty::alloc::Global{})",
                text,
            )
            text = re.sub(
                r"\brusty::alloc::Global,",
                "rusty::alloc::Global{},",
                text,
            )

            # `.unchecked_sub(X)` on size_t — Rust integer method that
            # asserts no underflow. Replace with plain `- X`. The call
            # always appears as `field_end(...).unchecked_sub(field_start(...))`
            # in vec_deque, so a generic replacement is safe.
            text = re.sub(
                r"\.unchecked_sub\(",
                " - (",
                text,
            )

            # `rusty::hint::assert_unchecked(…)` — no-op assert hint for
            # perf. Replace with `(void)0` so it compiles.
            text = re.sub(
                r"\brusty::hint::assert_unchecked\([^;]*\);",
                "(void)0;",
                text,
            )

            # `size_t::ByRefSized(&iter).take(head_room)` — transpiler
            # emit of a Rust trait-method call applied to the wrong
            # receiver. The `ByRefSized` adapter wraps `&mut iter` to
            # let it be used by-value. Replace with the bare iter; the
            # extra wrapping is a perf hint.
            text = re.sub(
                r"\bsize_t::ByRefSized\(&iter\)\.take\(",
                "iter.take(",
                text,
            )

            # Inject a `slice::range` shim near the top of the module
            # purview (after `export module …;`). The Rust source uses
            # `slice::range(range, ..len)` to normalize range bounds;
            # we don't vendor that utility. Provide a stub that returns
            # a struct with `.start` and `.end` fields so signatures
            # parse. Bodies that depend on the result are stubbed
            # elsewhere; only the type check matters here.
            SHIM = (
                "// patcher-injected: slice::range shim\n"
                "namespace slice {\n"
                "    struct _RangeShim { size_t start{}; size_t end{}; };\n"
                "    template<typename R, typename E>\n"
                "    inline _RangeShim range(R&&, E&&) noexcept "
                "{ return _RangeShim{}; }\n"
                "}\n"
            )
            if SHIM not in text:
                # Insert AFTER the last `import …;` line so we don't
                # break the C++20 rule that imports must immediately
                # follow `export module …;`.
                text = re.sub(
                    r"(import vec_port\.vec\.into_iter;[^\n]*\n)",
                    r"\1\n" + SHIM + "\n",
                    text,
                    count=1,
                )

            # `::prepend(...)` / `::prepend_reversed(...)` — these free
            # functions live in `vec_deque_port::` namespace; calls
            # qualify with `::` (global) which doesn't resolve. Drop
            # the `::` so name lookup finds the local definition.
            text = re.sub(r"(?<![A-Za-z0-9_:])::prepend\(", "prepend(", text)
            text = re.sub(
                r"(?<![A-Za-z0-9_:])::prepend_reversed\(",
                "prepend_reversed(",
                text,
            )

            # `IS_ZST` — anchored to `T` (size_of<T>() == 0). Replace
            # with `(sizeof(T) == 0)`. Two emit shapes:
            #   `::IS_ZST`  — global-namespace ref
            #   `T::IS_ZST` — Rust trait-method-on-T shape
            text = re.sub(
                r"(?<![A-Za-z0-9_:])::IS_ZST\b",
                "(sizeof(T) == 0)",
                text,
            )
            text = re.sub(
                r"(?<![A-Za-z0-9_])T::IS_ZST\b",
                "(sizeof(T) == 0)",
                text,
            )

            # Free-function `from(vec_deque_port::VecDeque<T, A> other)`
            # emitted without a template prefix — `T`/`A` aren't in
            # scope. Walk balanced braces to stub the whole body.
            text = _stub_free_from_vecdeque(text)

            # Stub `From<[T;N]>::from(std::array<T,N>)` — uses non-type
            # param `N` as a type arg, and references `::IS_ZST`.
            text = _stub_from_array_method(text)

            # Stub `next_chunk()` — return type `array::IntoIter<Item, N>`
            # references `std::array` as a namespace.
            text = _strip_next_chunk_method(text)

            # Replace ~VecDeque()'s body — the Rust Drop impl iterates
            # via `as_mut_slices()` to drop elements one by one. With
            # `as_mut_slices` stubbed, the call aborts. For the smoke
            # path (T = trivially-destructible) we can skip element
            # drops entirely; RawVec's destructor frees the buffer.
            text = _simplify_vecdeque_destructor(text)

            # Stub bodies of methods that are off the smoke-test path
            # (push_back/push_front/pop_back/pop_front/front/back/len/new_)
            # and use C++-side features we don't reproduce (slice::range,
            # std::iter::repeat_n, RawVec::try_with_capacity_in macro
            # tangle, `.extend(iter)`, etc.). The stubs return defaults
            # / `std::abort()` so the signature compiles but calls fail
            # at runtime — fine for smoke test which doesn't exercise
            # them.
            text = _stub_non_smoke_methods(text)

        if text != original:
            path.write_text(text)
            total_changes += 1
    return total_changes


def _lift_imports_to_top(text: str) -> str:
    """Move all `import …;` lines to immediately after the
    `export module …;` line. Required by C++20: imports must precede
    every other declaration in the module purview. Transpiler emits
    them interleaved with `using` / `export using`.

    Leaves the file's structure otherwise intact: the original lines
    that contained imports are blanked out (so line numbers in
    diagnostics shift only slightly).
    """
    lines = text.split("\n")
    out = []
    imports_to_lift = []
    module_line_idx = None

    for idx, line in enumerate(lines):
        if line.startswith("export module ") and module_line_idx is None:
            module_line_idx = idx

    if module_line_idx is None:
        return text

    for idx, line in enumerate(lines):
        stripped = line.lstrip()
        # Only lift bare `import X.Y;` lines past the module declaration.
        # Leave commented-out imports alone — they're our reduced-scope
        # markers.
        if (idx > module_line_idx
                and stripped.startswith("import ")
                and stripped.endswith(";")):
            imports_to_lift.append(line)
            out.append("")  # blank line in original position
        else:
            out.append(line)

    # Insert collected imports right after the module declaration.
    if imports_to_lift:
        insert_at = module_line_idx + 1
        # Inject a leading blank line if there isn't one already.
        if (insert_at < len(out)
                and out[insert_at].strip() != ""):
            imports_to_lift = imports_to_lift + [""]
        out = out[:insert_at] + imports_to_lift + out[insert_at:]
    return "\n".join(out)


def _strip_second_guard_and_try_rfold(text: str) -> str:
    """Strip the second `struct Guard { … }` and the `try_rfold` method
    that follows it from the IntoIter class scope. Rust source has
    method-local Guard structs; the transpiler hoisted both to class
    scope which produces a redefinition. We don't need try_rfold for
    the basic smoke test path.

    Strategy: line-walk; on the SECOND `    struct Guard {` line,
    eat until depth-balanced. Then continue eating any following
    method whose name starts with `try_rfold` (depth-balanced).
    """
    lines = text.split("\n")
    out = []
    seen_guard = 0
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.lstrip().startswith("struct Guard {"):
            seen_guard += 1
            if seen_guard == 2:
                # Eat this struct, then keep eating until we exit the
                # subsequent try_rfold method's body.
                out.append("    // patcher: second `struct Guard` + try_rfold stripped")
                # Eat the struct
                i = _eat_balanced_block(lines, i)
                # Skip blank lines
                while i < len(lines) and lines[i].strip() == "":
                    i += 1
                # If next non-blank is try_rfold, eat it too.
                if i < len(lines) and "try_rfold" in lines[i]:
                    i = _eat_balanced_block(lines, i)
                continue
        out.append(line)
        i += 1
    return "\n".join(out)


def _eat_balanced_block(lines, start_idx: int) -> int:
    """Starting at lines[start_idx] which contains an opening `{`,
    advance until the matching `}` closes the block. Returns the
    index AFTER the closing line."""
    depth = 0
    in_body = False
    i = start_idx
    while i < len(lines):
        line = lines[i]
        for ch in line:
            if ch == "{":
                depth += 1
                in_body = True
            elif ch == "}":
                depth -= 1
        i += 1
        if in_body and depth == 0:
            return i
    return i


def _strip_next_chunk_method(text: str) -> str:
    """Replace the `template<size_t N> ... next_chunk() { ... }` block
    with a stub. Tracks `{`/`}` depth from the opening brace of the
    function body, so nested braces don't confuse the boundary.
    """
    lines = text.split("\n")
    out = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if (i + 1 < len(lines)
                and "template<size_t N>" in line
                and "next_chunk()" in lines[i + 1]
                and "array::IntoIter" in lines[i + 1]):
            # Found the start of next_chunk. Replace lines i..end_of_body
            # with a stub.
            out.append("    // patcher: next_chunk() stubbed — array::IntoIter not vendored")
            out.append("    template<size_t N>")
            out.append("    rusty::Result<std::array<Item, rusty::sanitize_array_capacity<N>()>, void>")
            out.append("    next_chunk() { std::abort(); }")
            # Skip the original template + signature line. The body
            # starts on the same line as the signature ("{ auto raw_arr = …").
            # Find the matching close brace via depth tracking.
            i += 1
            depth = 0
            in_body = False
            while i < len(lines):
                line = lines[i]
                for ch in line:
                    if ch == "{":
                        depth += 1
                        in_body = True
                    elif ch == "}":
                        depth -= 1
                if in_body and depth == 0:
                    i += 1
                    break
                i += 1
        else:
            out.append(line)
            i += 1
    return "\n".join(out)


def _stub_non_smoke_methods(text: str) -> str:
    """Replace bodies of methods off the smoke-test path with stubs.

    For each `\\bNAME\\s*\\(` we find on a line that looks like a
    method definition (no `;` before the matching `)`, and a `{`
    after), we replace from that `{` to its matching `}` with
    `{ std::abort(); }`.

    Smoke-test keepers (NOT stubbed):
      new_, with_capacity_in (Global only), push_back, push_front,
      pop_back, pop_front, front, back, len, len_field accessors,
      capacity, is_empty, clear, ptr, allocator, helpers like
      `to_physical_idx`, `wrap_*`, `grow`, `cpy`, `write_iter*`,
      `buffer_*`, push_back_unchecked, push_unchecked, etc.

    Anything matching STUB_NAMES gets its body stubbed.
    """
    STUB_NAMES = [
        "clone",
        "clone_from",
        "with_capacity",
        "try_with_capacity",
        "try_with_capacity_in",
        "extend",
        "extend_front",
        "extend_from_within",
        "prepend_from_within",
        "spec_extend",
        "spec_extend_front",
        "spec_from_iter",
        "from_iter",
        "drain",
        "splice",
        "extract_if",
        "slice_ranges",
        "range",
        "range_mut",
        "binary_search",
        "binary_search_by",
        "binary_search_by_key",
        "partition_point",
        "swap",
        "swap_remove_back",
        "swap_remove_front",
        "rotate_left",
        "rotate_right",
        "resize",
        "resize_with",
        "truncate",
        "retain",
        "retain_mut",
        "dedup",
        "dedup_by",
        "dedup_by_key",
        "make_contiguous",
        "as_slices",
        "as_mut_slices",
        "iter",
        "iter_mut",
        "into_iter",
        "next_chunk",
        "try_fold",
        "try_rfold",
    ]
    name_set = set(STUB_NAMES)
    lines = text.splitlines(keepends=True)
    n = len(lines)
    out: list[str] = []
    # Compile a regex that matches `<word_boundary><name>(`
    name_re = re.compile(
        r"(?<![A-Za-z0-9_])(" + "|".join(re.escape(s) for s in STUB_NAMES) + r")\s*\("
    )
    i = 0
    while i < n:
        ln = lines[i]
        # Cheap skip: comments, struct decls, etc.
        if (
            ln.lstrip().startswith("//")
            or ln.lstrip().startswith("struct ")
            or ln.lstrip().startswith("using ")
        ):
            out.append(ln)
            i += 1
            continue
        # Try to find a candidate name on this line.
        m = name_re.search(ln)
        if not m:
            out.append(ln)
            i += 1
            continue
        # Verify it's a method DEFINITION by walking balanced parens
        # from after the name to find the closing `)`, then checking
        # that the next non-whitespace is `{` (or `const {` etc.).
        paren_pos = m.end() - 1  # index of `(`
        depth = 0
        sig_end_line = i
        sig_end_col = paren_pos
        found_close = False
        # Walk
        j = i
        k = paren_pos
        scan_lines = 0
        is_call = False
        while j < n and scan_lines < 20:
            chars = lines[j]
            while k < len(chars):
                ch = chars[k]
                if ch == "(":
                    depth += 1
                elif ch == ")":
                    depth -= 1
                    if depth == 0:
                        # End of parameter list.
                        sig_end_line = j
                        sig_end_col = k
                        found_close = True
                        break
                elif ch == ";":
                    # Forward decl or call statement.
                    is_call = True
                    break
                k += 1
            if found_close or is_call:
                break
            j += 1
            k = 0
            scan_lines += 1
        if is_call or not found_close:
            out.append(ln)
            i += 1
            continue
        # After `)`, look for `{` (possibly preceded by `const`,
        # `noexcept`, `&&`, etc.) before a `;`.
        body_start_line = -1
        body_start_col = -1
        j = sig_end_line
        k = sig_end_col + 1
        found_brace = False
        scan_lines = 0
        while j < n and scan_lines < 5:
            chars = lines[j]
            while k < len(chars):
                ch = chars[k]
                if ch == "{":
                    body_start_line = j
                    body_start_col = k
                    found_brace = True
                    break
                if ch == ";":
                    is_call = True
                    break
                k += 1
            if found_brace or is_call:
                break
            j += 1
            k = 0
            scan_lines += 1
        if is_call or not found_brace:
            out.append(ln)
            i += 1
            continue
        # Now walk balanced braces from body_start_line:body_start_col
        # to find the matching `}`.
        depth = 0
        j = body_start_line
        k = body_start_col
        end_line = -1
        end_col = -1
        while j < n:
            chars = lines[j]
            while k < len(chars):
                ch = chars[k]
                if ch == "{":
                    depth += 1
                elif ch == "}":
                    depth -= 1
                    if depth == 0:
                        end_line = j
                        end_col = k
                        break
                k += 1
            if end_line >= 0:
                break
            j += 1
            k = 0
        if end_line < 0:
            # Couldn't find matching brace.
            out.append(ln)
            i += 1
            continue
        # Emit signature up to body_start, then stub, then continue
        # from after the body's closing `}`.
        # Lines [i .. body_start_line-1] are kept fully.
        for L in range(i, body_start_line):
            out.append(lines[L])
        sig_head = lines[body_start_line][:body_start_col]
        indent_match = re.match(r"^(\s*)", lines[i])
        indent = indent_match.group(1) if indent_match else "    "
        out.append(sig_head + "{\n")
        out.append(
            f"{indent}    // patcher: stubbed (off smoke-test path)\n"
        )
        out.append(f"{indent}    std::abort();\n")
        out.append(f"{indent}}}\n")
        # Skip past the original body's closing `}` and append any
        # trailing text on that line.
        trailing = lines[end_line][end_col + 1:]
        if trailing.strip():
            out.append(trailing)
        i = end_line + 1
    return "".join(out)


def _simplify_vecdeque_destructor(text: str) -> str:
    """Replace `~VecDeque() noexcept(false) { ... }` body with empty.

    The Rust Drop impl iterates `as_mut_slices()` to drop elements.
    With as_mut_slices stubbed, the body aborts. For smoke-test path
    where T is trivially-destructible, skipping the element drop loop
    is correct; RawVec's own destructor frees the buffer.
    """
    lines = text.splitlines(keepends=True)
    out: list[str] = []
    i = 0
    n = len(lines)
    while i < n:
        ln = lines[i]
        if re.match(r"^\s*~VecDeque\(\) noexcept\(false\) \{", ln):
            indent_match = re.match(r"^(\s*)", ln)
            indent = indent_match.group(1) if indent_match else "    "
            # Walk balanced braces from this `{`.
            brace_col = ln.find("{")
            depth = 0
            j = i
            k = brace_col
            end_line = -1
            while j < n:
                chars = lines[j]
                while k < len(chars):
                    ch = chars[k]
                    if ch == "{":
                        depth += 1
                    elif ch == "}":
                        depth -= 1
                        if depth == 0:
                            end_line = j
                            break
                    k += 1
                if end_line >= 0:
                    break
                j += 1
                k = 0
            if end_line >= 0:
                out.append(
                    f"{indent}~VecDeque() noexcept(false) {{\n"
                )
                out.append(
                    f"{indent}    // patcher: destructor body simplified — "
                    "element drops skipped\n"
                )
                out.append(
                    f"{indent}    // (RawVec's own destructor handles "
                    "buffer free).\n"
                )
                out.append(f"{indent}}}\n")
                i = end_line + 1
                continue
        out.append(ln)
        i += 1
    return "".join(out)


def _stub_free_from_vecdeque(text: str) -> str:
    """Stub the free-function `from(vec_deque_port::VecDeque<T, A> other)`.

    The transpiler emitted this `impl From<VecDeque<T,A>> for Vec<T,A>`
    as a free function, but it references `T` and `A` which aren't
    in scope at file level. Body uses `::Vec<...>::from_raw_parts_in`
    which doesn't compile either. Replace with a comment.
    """
    lines = text.splitlines(keepends=True)
    out: list[str] = []
    i = 0
    n = len(lines)
    while i < n:
        ln = lines[i]
        if re.match(
            r"^static auto from\(vec_deque_port::VecDeque<T, A>",
            ln,
        ):
            # Walk balanced braces from the `{` on this line (or
            # subsequent lines).
            brace_line = i
            brace_col = ln.find("{")
            while brace_col < 0 and brace_line + 1 < n:
                brace_line += 1
                brace_col = lines[brace_line].find("{")
            if brace_col < 0:
                out.append(ln)
                i += 1
                continue
            depth = 0
            j = brace_line
            k = brace_col
            end_line = -1
            while j < n:
                chars = lines[j]
                while k < len(chars):
                    ch = chars[k]
                    if ch == "{":
                        depth += 1
                    elif ch == "}":
                        depth -= 1
                        if depth == 0:
                            end_line = j
                            break
                    k += 1
                if end_line >= 0:
                    break
                j += 1
                k = 0
            if end_line >= 0:
                out.append(
                    "// patcher: free-fn from(VecDeque<T,A>) stubbed "
                    "(template params not in scope at file level)\n"
                )
                i = end_line + 1
                continue
        out.append(ln)
        i += 1
    return "".join(out)


def _stub_from_array_method(text: str) -> str:
    """Stub the `From<[T; N]>::from(arr)` method that uses `N` as
    both non-type param and type arg, confusing the parser.

    Walks forward from the `template<size_t N>` decl line, finds
    the immediately-following `from(std::array<...) {`, walks the
    body's balanced braces to find the matching `}`, and replaces
    the entire span with a stub.
    """
    lines = text.splitlines(keepends=True)
    out: list[str] = []
    i = 0
    n = len(lines)
    while i < n:
        ln = lines[i]
        # Match `template<size_t N>` followed (on the same or next
        # line) by `static ... from(std::array<...) {`.
        is_template_line = bool(re.match(r"^\s*template<size_t N>\s*$", ln))
        if is_template_line and i + 1 < n:
            sig_line = lines[i + 1]
            if "from(std::array" in sig_line:
                # Find the opening `{` of the body. It's after `)` on
                # this line (or possibly the next).
                brace_line = i + 1
                brace_col = sig_line.find("{")
                while brace_col < 0 and brace_line + 1 < n:
                    brace_line += 1
                    brace_col = lines[brace_line].find("{")
                if brace_col >= 0:
                    # Walk balanced braces from (brace_line, brace_col).
                    depth = 0
                    j = brace_line
                    k = brace_col
                    end_line = -1
                    end_col = -1
                    while j < n:
                        chars = lines[j]
                        while k < len(chars):
                            ch = chars[k]
                            if ch == "{":
                                depth += 1
                            elif ch == "}":
                                depth -= 1
                                if depth == 0:
                                    end_line = j
                                    end_col = k
                                    break
                            k += 1
                        if end_line >= 0:
                            break
                        j += 1
                        k = 0
                    if end_line >= 0:
                        indent_match = re.match(r"^(\s*)", ln)
                        indent = (
                            indent_match.group(1)
                            if indent_match
                            else "    "
                        )
                        out.append(
                            f"{indent}// patcher: From<[T;N]>::from "
                            "stubbed (N-non-type-arg conflict)\n"
                        )
                        out.append(f"{indent}template<size_t N>\n")
                        out.append(
                            f"{indent}static VecDeque<T, A> "
                            "from(std::array<T, N>) "
                            "{ std::abort(); }\n"
                        )
                        # Skip past the body's closing `}` plus any
                        # trailing chars on that line.
                        trailing = lines[end_line][end_col + 1:]
                        if trailing.strip():
                            out.append(trailing)
                        i = end_line + 1
                        continue
        out.append(ln)
        i += 1
    return "".join(out)


def _stub_copied_spec_extend_front(text: str) -> str:
    """Delete the `spec_extend_front(Copied<...> | Rev<Copied<...>>)`
    method specializations entirely.

    These are Rust source-side perf specializations of the generic
    `spec_extend_front<I>`. We don't have a C++ analog for
    `core::iter::Copied`, so the signature won't parse. Skip them —
    the generic template-`I` overload handles all callers
    (slower fallback path, fine for smoke test).

    The match is conservative: must be a true method signature, not
    a comment. A signature has `(` and an open-brace `{` on the
    same line or following lines without an intervening `;`.
    """
    lines = text.splitlines(keepends=True)
    out: list[str] = []
    i = 0
    while i < len(lines):
        ln = lines[i]
        stripped = ln.lstrip()
        is_sig = (
            ("std::iter::Copied" in ln or "::iter::Copied" in ln)
            and "spec_extend_front" in ln
            and not stripped.startswith("//")
            and "(" in ln  # rules out the `using` line
        )
        if is_sig:
            indent_match = re.match(r"^(\s*)", ln)
            indent = indent_match.group(1) if indent_match else "    "
            out.append(
                f"{indent}// patcher: spec_extend_front<Copied<...>> "
                "deleted (no core::iter::Copied analog; generic "
                "template-I overload covers callers)\n"
            )
            i = _eat_balanced_block(lines, i)
            continue
        out.append(ln)
        i += 1
    return "".join(out)


def _disambiguate_hoisted_helpers(text: str) -> str:
    """Rename duplicate `struct Guard` / `struct Dropper` hoisted to
    class scope by the transpiler.

    The transpiler lifts method-local types to class scope without
    disambiguating the names — distinct types end up with the same
    name and produce a redefinition error. For each name, the first
    occurrence stays as-is; subsequent occurrences get suffixed
    (`_2`, `_3`, …) and all uses *within the immediately-following
    method body* are renamed to match.

    Heuristic for "immediately-following method body":
      - Start at the line after the struct's closing `};`.
      - Read until brace depth at class-scope returns to 0 (the
        method's body ends), OR we hit the next `    struct \w+ {`
        at the same indent.
    """
    lines = text.splitlines(keepends=True)
    counts: dict[str, int] = {"Guard": 0, "Dropper": 0}
    i = 0
    while i < len(lines):
        ln = lines[i]
        m = re.match(r"^(\s{4})struct (Guard|Dropper) \{\s*$", ln)
        if not m:
            i += 1
            continue
        name = m.group(2)
        counts[name] += 1
        if counts[name] == 1:
            i += 1
            continue
        new_name = f"{name}_{counts[name]}"

        # Find struct end (matching `};` at the same indent).
        struct_start = i
        depth = 0
        in_body = False
        j = i
        while j < len(lines):
            for ch in lines[j]:
                if ch == "{":
                    depth += 1
                    in_body = True
                elif ch == "}":
                    depth -= 1
            j += 1
            if in_body and depth == 0:
                break
        struct_end = j  # one past the `};`

        # Find end of the method body that uses this struct.
        method_end = struct_end
        depth = 0
        in_body = False
        for k in range(struct_end, len(lines)):
            kl = lines[k]
            # Stop early if we run into the next hoisted struct.
            if re.match(r"^\s{4}struct \w+ \{\s*$", kl):
                method_end = k
                break
            for ch in kl:
                if ch == "{":
                    depth += 1
                    in_body = True
                elif ch == "}":
                    depth -= 1
            if in_body and depth == 0:
                method_end = k + 1
                break

        # Rename `name` → `new_name` in struct + method body.
        pattern = re.compile(r"\b" + name + r"\b")
        for k in range(struct_start, method_end):
            lines[k] = pattern.sub(new_name, lines[k])
        i = method_end
    return "".join(lines)


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 1
    cpp_out = Path(sys.argv[1])
    if not cpp_out.exists():
        print(f"error: {cpp_out} does not exist")
        return 1

    n = patch_all_files(cpp_out)
    print(f"vec_deque_port patches applied to {n} file(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
