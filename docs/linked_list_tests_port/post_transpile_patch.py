#!/usr/bin/env python3
"""Post-transpile patches for linked_list_tests_port.

Source: library/alloctests/tests/linked_list.rs (1 #[test], 21 LOC).
The single test uses `BuildHasherDefault::<DefaultHasher>` which we don't
have in rusty-cpp — stub it as skip.
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
    text = inject_module_imports(text, "linked_list_tests_port", ["linked_list_port"])
    # 1 test, needs std::hash builder support → stub.
    text = stub_all_remaining_tests(text, "needs std::hash builder (rusty-cpp infra gap)")
    path.write_text(text)


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("cpp_out", type=Path)
    args = p.parse_args()
    target = args.cpp_out / "linked_list_tests_port.cppm"
    if not target.exists():
        print(f"error: {target} not found")
        return 1
    apply_patches(target)
    print(f"linked_list_tests_port patches applied to {target.name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
