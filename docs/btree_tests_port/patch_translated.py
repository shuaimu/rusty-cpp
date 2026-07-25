#!/usr/bin/env python3
"""Wire the transpiler's REAL translation of rustc's btree tests into a
buildable module.

The transpiler now translates rustc's `alloc/src/collections/btree/{map,set}/
tests.rs` essentially in full (124 of 125 tests emit clean C++). What it cannot
supply is the *test environment* those files assume:

  * `crate::testing::{crash_test,ord_chaos,rng}` — prep.sh rewrites these to
    `std::testing::...`, which doesn't exist in C++. We map them onto the C++
    ports we already ship: `btree_testing::` (tests/btree_testing_helpers.hpp)
    and `testing_port::rng` (transpiled/testing_port).
  * `TEST_CASE` — the transpiler emits the macro but not its header.
  * `node::CAPACITY` — a btree-internal constant (11 for our B=6 port).
  * `.check()` / `.check_invariants()` — rustc defines these in an
    `impl BTreeMap` block *inside* tests.rs. C++ can't add members to a class
    from another module, so until they land in btree_port itself they are
    neutralized here (the hand-written suite used the same no-op `check` shim).

Usage:  python3 patch_translated.py <transpiler-output-dir>
"""
import re
import sys
import pathlib

# `import` declarations must precede every other declaration in a module
# purview, so anything we inject has to go *after* the last import line.
GMF_INCLUDES = '#include <rusty/test_runner.hpp>\n#include "btree_testing_helpers.hpp"\n'

# All three rustc testing modules are provided by the one C++ header we ship
# (tests/btree_testing_helpers.hpp). `transpiled/testing_port` also has a
# DeterministicRng, but it is not a CMake target — nothing builds its BMI — so
# `import testing_port.rng` would not resolve.
TESTING_NS = {
    "std::testing::crash_test::": "btree_testing::",
    "std::testing::ord_chaos::": "btree_testing::",
    "std::testing::rng::": "btree_testing::",
}


def patch(path: pathlib.Path) -> dict:
    src = path.read_text()
    applied = {}

    # 1. Test-environment headers into the global module fragment.
    if "test_runner.hpp" not in src:
        src, n = re.subn(r"(?m)^(export module )", GMF_INCLUDES + r"\1", src, count=1)
        applied["gmf_includes"] = n

    # 2. `using std::testing::...;` declarations are meaningless in C++ — the
    #    types come from the headers above. Drop them, then qualify uses.
    src, n = re.subn(r"(?m)^using std::(testing|assert_matches)[^\n]*\n", "", src)
    applied["drop_using"] = n
    total = 0
    for rust_ns, cpp_ns in TESTING_NS.items():
        src, n = re.subn(re.escape(rust_ns), cpp_ns, src)
        total += n
    applied["testing_ns"] = total

    # 3. btree-internal constant: our port's node capacity (B = 6 -> 11).
    src, n = re.subn(
        r"rusty::detail::deref_if_pointer_like\(node::CAPACITY\)|node::CAPACITY",
        "static_cast<size_t>(11)",
        src,
    )
    applied["node_capacity"] = n

    # 4. Internal invariant helpers rustc defines on BTreeMap inside tests.rs.
    #    Neutralize until btree_port grows real check()/check_invariants().
    src, n = re.subn(r"\b(\w+)\.check_invariants\(\)", r"(void)\1", src)
    src, n2 = re.subn(r"\b(\w+)\.check\(\)", r"(void)\1", src)
    applied["check_shim"] = n + n2

    path.write_text(src)
    return applied


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    out = pathlib.Path(sys.argv[1])
    files = sorted(out.glob("*.cppm"))
    if not files:
        print(f"no .cppm files in {out}", file=sys.stderr)
        return 1
    for f in files:
        if "TEST_CASE" not in f.read_text():
            continue
        applied = patch(f)
        summary = ", ".join(f"{k}={v}" for k, v in applied.items() if v)
        print(f"  patched {f.name}: {summary or 'no-op'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
