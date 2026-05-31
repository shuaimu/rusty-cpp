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
