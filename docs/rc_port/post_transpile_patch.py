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
