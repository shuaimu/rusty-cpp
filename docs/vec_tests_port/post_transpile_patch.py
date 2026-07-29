#!/usr/bin/env python3
"""Post-transpile patches for the vec (alloctests vec.rs) suite.

The suite binds the ALLOC consolidated module's `vec::Vec` (the vendored
vec_port stays on its own smoke path). Started minimal — rules grow from
compile-error triage exactly like the vec_deque patcher did.

Usage: post_transpile_patch.py <cpp_out_dir>
"""
import re
import sys
from pathlib import Path

HELPERS = """
// The tests exercise the ALLOC consolidated module's Vec.
template<typename T, typename A = rusty::alloc::Global>
using VecT = ::vec::Vec<T, A>;
"""

# Tests stubbed with a cited reason (printed at runtime, counted ok).
SKIP_TESTS: dict[str, str] = {}


def _suite_fixes(text: str) -> str:
    # Import the alloc module instead of the legacy ports.
    if "import alloc;" not in text:
        text = text.replace("import rusty;", "import rusty;\nimport alloc;", 1)
    # Helper block after the namespace opens.
    marker = "namespace vec_tests_port {"
    if marker in text and "using VecT = " not in text:
        text = text.replace(marker, marker + "\n" + HELPERS, 1)
    for name, reason in SKIP_TESTS.items():
        pat = re.compile(
            r'(TEST_CASE\("' + re.escape(name) + r'"\) \{)(.*?)(\n\})',
            re.DOTALL)
        text = pat.sub(
            lambda m: m.group(1)
            + f'\n    rusty::io::println_str("[port] SKIP {name}: {reason}");'
            + "\n}",
            text, count=1)
    return text


def run(cpp_out: Path) -> None:
    for path in sorted(cpp_out.glob("*.cppm")):
        o = path.read_text()
        t = _suite_fixes(o)
        if t != o:
            path.write_text(t)
            print(f"vec_tests_port patches applied to {path.name}")


if __name__ == "__main__":
    run(Path(sys.argv[1]))
