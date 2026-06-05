#!/usr/bin/env python3
"""Post-transpile patches for the core_slice_port C++20 module port.

Idempotent. core_slice_port is `library/core/src/slice/*` (15421 LOC
Rust collapsed via prep.sh + collapse.py) → `core_slice_port.cppm`
(8773 LOC C++).

Patches:
  P1 — `std::ops::Bound` → `rusty::Bound` (Bound is at rusty::, not
        rusty::ops::; matches rusty/array.hpp definition).
  P2 — `std::ops::ControlFlow` → `rusty::ops::ControlFlow`.
  P3 — `std::convert::` → `rusty::convert::`.
  P4 — `std::ops::` → `rusty::ops::` (catch-all for remaining
        std::ops:: references; runs AFTER specific Bound/ControlFlow
        rewrites so they take precedence).

Usage: post_transpile_patch.py <cpp_out_dir>
"""

from __future__ import annotations

import sys
from pathlib import Path

SLICE_FILE = "core_slice_port.cppm"


def patch_bound(text: str) -> str:
    # Bound is defined as a free template alias at `rusty::Bound` in
    # array.hpp; it is NOT under rusty::ops::.
    return text.replace("std::ops::Bound", "rusty::Bound")


def patch_control_flow(text: str) -> str:
    return text.replace("std::ops::ControlFlow", "rusty::ops::ControlFlow")


def patch_convert(text: str) -> str:
    return text.replace("std::convert::", "rusty::convert::")


def patch_remaining_std_ops(text: str) -> str:
    # Catch-all after the specific Bound/ControlFlow rewrites above.
    return text.replace("std::ops::", "rusty::ops::")


def patch_std_range(text: str) -> str:
    # The Rust `range::Range`, `range::RangeInclusive` paths leak through
    # as `std::range::Range` etc. They should be rusty::ops::*.
    return text.replace("std::range::", "rusty::ops::")


def patch_std_ptr(text: str) -> str:
    return text.replace("std::ptr::", "rusty::ptr::")


def patch_std_ascii(text: str) -> str:
    return text.replace("std::ascii::", "rusty::ascii::")


def patch_size_of(text: str) -> str:
    # `size_of::<T>()` and `size_of<T>()` are emitted from `mem::size_of`;
    # neither exists in C++; map to `sizeof(T)`.
    import re
    text = re.sub(r"\bsize_of<([^>]+)>\(\)", r"sizeof(\1)", text)
    return text


def patch_strip_orphan_imports(text: str) -> str:
    # `import core_slice_port.index;` etc. — auto-namespace artifacts
    # for submodules that don't exist post-collapse. They appear inside
    # a `module;`/`export module` body where `import` isn't a keyword,
    # so the parser errors. Strip them.
    import re
    return re.sub(r"^import\s+core_slice_port\.\w+;\s*\n",
                  "", text, flags=re.MULTILINE)


def patch_strip_using_simd(text: str) -> str:
    # `using std::simd;` etc. — Rust's portable_simd has no analogue.
    import re
    return re.sub(r"^using std::simd(?:::\w+)?;\s*\n",
                  "", text, flags=re.MULTILINE)


def patch_std_ub_checks(text: str) -> str:
    # Residual `std::ub_checks` reference that prep.sh's macro-strip
    # didn't catch (the use of `ub_checks` outside the macro syntax).
    import re
    return re.sub(r"^using std::ub_checks;\s*\n",
                  "", text, flags=re.MULTILINE)


def patch_file(path: Path) -> bool:
    text = path.read_text()
    original = text
    text = patch_bound(text)
    text = patch_control_flow(text)
    text = patch_convert(text)
    text = patch_remaining_std_ops(text)
    text = patch_std_range(text)
    text = patch_std_ptr(text)
    text = patch_std_ascii(text)
    text = patch_size_of(text)
    text = patch_strip_orphan_imports(text)
    text = patch_strip_using_simd(text)
    text = patch_std_ub_checks(text)
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
    path = cpp_out / SLICE_FILE
    if not path.exists():
        print(f"error: {path} does not exist")
        return 1
    changed = patch_file(path)
    if changed:
        print(f"core_slice_port patches applied to {path.name}")
    else:
        print(f"core_slice_port: no patches needed (already clean or idempotent)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
