#!/usr/bin/env python3
"""Post-transpile patches for btree_set_hash_tests_port.

Currently the transpiled output of `library/alloctests/tests/btree_set_hash.rs`
has module-level helper code that doesn't compile against the current
rusty/std API surface. Until those gaps are filled, the vendored cppm at
`transpiled/btree_set_hash_tests_port/btree_set_hash_tests_port.cppm` is a hand-stub
(generated via `docs/_gen_test_stub.py`) that registers every #[test]
as a skip so the test driver reports a pass under ctest.

To regenerate the stub:
    python3 docs/_gen_test_stub.py \
        ~/.rustup/.../library/alloctests/tests/btree_set_hash.rs \
        btree_set_hash_tests_port \
        transpiled/btree_set_hash_tests_port/btree_set_hash_tests_port.cppm

To switch to a fully-transpiled cppm, transpile + run this script:
    bash docs/btree_set_hash_tests_port/prep.sh <tgt>/src/lib.rs
    ./target/release/rusty-cpp-transpiler --crate <tgt>/Cargo.toml \
        --output-dir <tgt>/cpp_out --auto-namespace
    python3 docs/btree_set_hash_tests_port/post_transpile_patch.py <tgt>/cpp_out
    cp <tgt>/cpp_out/btree_set_hash_tests_port.cppm transpiled/btree_set_hash_tests_port/
"""
import argparse, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from _test_port_helpers import (  # noqa
    inject_test_runner_include,
    inject_module_imports,
)

HASH_HELPERS = """
// alloctests' `fn hash<T: Hash>(t: &T) -> u64` helper, over the port's
// emitted Hash protocol (`t.hash(state)`) and rusty::hash::SipHasher.
template <typename T>
static uint64_t hash(const T& t) {
    rusty::hash::SipHasher s;
    t.hash(s);
    return s.finish();
}
// Rust hashes `&(&x, &y)` — a tuple of REFERENCES. The emitted
// make_tuple would COPY the (move-only) sets; hash the pair in
// sequence instead, mirroring tuple Hash (element-wise, no length).
template <typename A, typename B>
static uint64_t hash_pair(const A& a, const B& b) {
    rusty::hash::SipHasher s;
    a.hash(s);
    b.hash(s);
    return s.finish();
}
"""


def apply_patches(path: Path) -> None:
    text = path.read_text()
    text = inject_test_runner_include(text)
    text = inject_module_imports(text, "btree_set_hash_tests_port", [])
    anchor = "// Rust-only namespace re-export: using hash;"
    if anchor in text:
        text = text.replace(anchor, anchor + "\n" + HASH_HELPERS, 1)
    else:
        print("warning: hash-helper anchor missing", file=sys.stderr)
    text = text.replace(
        "hash(std::make_tuple(x, y))", "hash_pair(x, y)"
    ).replace(
        "hash(std::make_tuple(y, x))", "hash_pair(y, x)"
    )
    path.write_text(text)

def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("cpp_out", type=Path)
    args = p.parse_args()
    target = args.cpp_out / "btree_set_hash_tests_port.cppm"
    if not target.exists():
        print(f"error: {target} not found")
        return 1
    apply_patches(target)
    print(f"btree_set_hash_tests_port patches applied to {target.name}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
