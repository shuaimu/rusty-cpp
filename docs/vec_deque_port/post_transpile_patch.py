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

        # Drop imports of submodules we exclude from the reduced-scope
        # build (see CMakeLists.txt vec_deque_port note). The dropped
        # submodules pull in iterator-adapter types we don't vendor yet.
        # Only applies to the main `vec_deque_port.cppm` file.
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

        if text != original:
            path.write_text(text)
            total_changes += 1
    return total_changes


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
