#!/usr/bin/env python3
"""Post-transpile patches for eq_diff_len_tests_port.

Source: library/alloctests/tests/collections/eq_diff_len.rs (7 #[test],
96 LOC). Tests `PartialEq::eq` short-circuit on size mismatch across
Vec, HashSet, HashMap, BTreeSet, BTreeMap, LinkedList. Uses an `Evil`
type with panicking `eq`; the size-mismatch tests verify `eq` is NEVER
called when sizes differ. The `evil_eq_works` test uses `#[should_panic]`
which the transpiler doesn't translate — stub it.

For the rest, stub for now — they exercise `==` on collections of
custom struct types which requires `operator==` plumbing we don't fully
have yet.
"""
from __future__ import annotations
import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from _test_port_helpers import (  # noqa
    inject_test_runner_include,
    inject_module_imports,
    stub_all_remaining_tests,
)


def apply_patches(path: Path) -> None:
    text = path.read_text()
    text = inject_test_runner_include(text)
    # No module imports needed beyond what the runtime provides for now,
    # since all tests are stubbed.
    text = inject_module_imports(text, "eq_diff_len_tests_port", [])
    text = stub_all_remaining_tests(text, "needs collection `==` plumbing + #[should_panic] runner support")
    path.write_text(text)


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("cpp_out", type=Path)
    args = p.parse_args()
    target = args.cpp_out / "eq_diff_len_tests_port.cppm"
    if not target.exists():
        print(f"error: {target} not found")
        return 1
    apply_patches(target)
    print(f"eq_diff_len_tests_port patches applied to {target.name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
