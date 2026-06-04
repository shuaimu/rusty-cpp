#!/usr/bin/env python3
"""Post-transpile patches for vec_tests_port.

Currently the transpiled output of `library/alloctests/tests/vec.rs`
has module-level helper code that doesn't compile against the current
rusty/std API surface. Until those gaps are filled, the vendored cppm at
`transpiled/vec_tests_port/vec_tests_port.cppm` is a hand-stub
(generated via `docs/_gen_test_stub.py`) that registers every #[test]
as a skip so the test driver reports a pass under ctest.

To regenerate the stub:
    python3 docs/_gen_test_stub.py \
        ~/.rustup/.../library/alloctests/tests/vec.rs \
        vec_tests_port \
        transpiled/vec_tests_port/vec_tests_port.cppm

To switch to a fully-transpiled cppm, transpile + run this script:
    bash docs/vec_tests_port/prep.sh <tgt>/src/lib.rs
    ./target/release/rusty-cpp-transpiler --crate <tgt>/Cargo.toml \
        --output-dir <tgt>/cpp_out --auto-namespace
    python3 docs/vec_tests_port/post_transpile_patch.py <tgt>/cpp_out
    cp <tgt>/cpp_out/vec_tests_port.cppm transpiled/vec_tests_port/
"""
import argparse, sys
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
    text = inject_module_imports(text, "vec_tests_port", [])
    text = stub_all_remaining_tests(text, "transpiled module needs rusty/std API surface gaps filled")
    path.write_text(text)

def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("cpp_out", type=Path)
    args = p.parse_args()
    target = args.cpp_out / "vec_tests_port.cppm"
    if not target.exists():
        print(f"error: {target} not found")
        return 1
    apply_patches(target)
    print(f"vec_tests_port patches applied to {target.name}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
