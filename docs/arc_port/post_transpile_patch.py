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

    print(f"arc_port patches applied: {total}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
