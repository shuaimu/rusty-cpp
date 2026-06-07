#!/usr/bin/env python3
"""Post-transpile patches for btree_tests_port.

Like btree_set_hash_tests_port, the transpiled output of rustc's
`btree/map/tests.rs` + `btree/set/tests.rs` depends on rusty/std API
surface that we don't yet ship — most notably:

  - `crate::testing::crash_test::{CrashTestDummy, Panic}` and
    `crate::testing::ord_chaos::*` (Cyclic3, Governed, Governor, IdBased)
    — not currently in transpiled/testing_port.
  - `impl<K, V> BTreeMap<K, V>` adds private invariant-check methods
    (`check_invariants`, `assert_back_pointers`, `calc_length`,
    `assert_min_len`) that reach into the navigation internals.
  - Heavy use of `catch_unwind` + `AssertUnwindSafe` for panic tests.

Until those land, the vendored .cppm at
`transpiled/btree_tests_port/btree_tests_port.cppm` is a hand-stub
(generated via `docs/_gen_test_stub.py`) registering every #[test]
as a skip. The test driver then reports all-pass under ctest.

To regenerate the stub from the rustc source:
    RUSTSRC=~/.rustup/.../library/alloc/src/collections/btree
    python3 docs/_gen_test_stub.py \\
        "$RUSTSRC/map/tests.rs" \\
        btree_tests_port \\
        transpiled/btree_tests_port/btree_tests_port.cppm
    # Append set tests:
    python3 docs/_gen_test_stub.py \\
        "$RUSTSRC/set/tests.rs" \\
        btree_tests_port \\
        /tmp/_set_stub.cppm
    # Merge the two TEST_CASE blocks into one file (set tests get
    # a `_set_` suffix to disambiguate from same-named map tests).

To switch to a fully-transpiled cppm (when the prereqs land), follow
the pattern in docs/btree_set_hash_tests_port/post_transpile_patch.py
— prep.sh the lib.rs, run the transpiler with --crate + --auto-namespace,
then apply patches via this script.
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
    text = inject_module_imports(text, "btree_tests_port", [])
    text = stub_all_remaining_tests(
        text,
        "transpiled module needs crate::testing helpers + BTreeMap "
        "private-invariant methods",
    )
    path.write_text(text)

def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("cpp_out", type=Path)
    args = p.parse_args()
    target = args.cpp_out / "btree_tests_port.cppm"
    if not target.exists():
        print(f"error: {target} not found")
        return 1
    apply_patches(target)
    print(f"btree_tests_port patches applied to {target.name}")
    return 0

if __name__ == "__main__":
    sys.exit(main())
