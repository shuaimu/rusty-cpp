#!/usr/bin/env python3
"""Post-transpile patches for vec_deque_tests_port (REAL translation).

Pipeline (transpile WITHOUT --expand; cargo-expand strips #[test]):
    # 1. prep: sed alloc/core/crate -> std, then expand
    #    struct_with_counted_drop! at module level:
    python3 docs/vec_deque_tests_port/prep_expand.py <tgt>/src/lib.rs
    # 2. transpile (DUMP_AUTO lets the known-untypable autos through;
    #    those tests are stubbed below):
    RUSTY_CPP_DUMP_AUTO=1 ./target/release/rusty-cpp-transpiler \
        --crate <tgt>/Cargo.toml --output-dir <tgt>/cpp_out --auto-namespace
    python3 docs/vec_deque_tests_port/post_transpile_patch.py <tgt>/cpp_out
    cp <tgt>/cpp_out/vec_deque_tests_port.cppm transpiled/vec_deque_tests_port/

The tests must run against the PORT deque
(rusty::port::collections::vec_deque::VecDeque<T, A>), not the facade
rusty::VecDeque (which lacks as_slices/drain/truncate_front/...).
"""
import argparse, re, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from _test_port_helpers import (  # noqa
    inject_test_runner_include,
    inject_module_imports,
    stub_tests,
)

HELPERS = """
// The tests exercise the PORT deque; the facade rusty::VecDeque lacks
// most of the tested API.
template<typename T, typename A = rusty::alloc::Global>
using VecDequeT = rusty::port::collections::vec_deque::VecDeque<T, A>;
// `VecDeque::from(x)` — Rust infers T; deduce from the source.
template<typename Src>
static auto vd_from(Src&& src) {
    using S = std::remove_cvref_t<Src>;
    if constexpr (requires { typename S::Item; }) {
        return VecDequeT<typename S::Item>::from(std::forward<Src>(src));
    } else {
        return VecDequeT<typename S::value_type>::from(std::forward<Src>(src));
    }
}
"""

# DUMP_AUTO leftovers (untypable locals the transpiler cannot infer).
AUTO_TESTS = []  # filled in as triage identifies them


def apply_patches(path: Path) -> None:
    text = path.read_text()
    text = inject_test_runner_include(text)
    text = inject_module_imports(text, "vec_deque_tests_port", [])

    # Route every deque through the port type.
    text = text.replace("rusty::VecDeque<", "VecDequeT<")
    text = re.sub(r"(?<![:\w])VecDeque::from\(", "vd_from(", text)
    text = re.sub(r"(?<![:\w])VecDeque::with_capacity\(",
                  "VecDequeT<int32_t>::with_capacity(", text)
    # new_() sites: test_resize_keeps_reserved_space_from_item's element
    # is Vec<i32>; the other two are i32.
    text = text.replace(
        "auto d = VecDeque::new_();\n    d.resize(1, std::move(v));",
        "auto d = VecDequeT<rusty::Vec<int32_t>>::new_();\n    d.resize(1, std::move(v));")
    text = re.sub(r"(?<![:\w])VecDeque::new_\(\)", "VecDequeT<int32_t>::new_()", text)

    # Insert helpers after the module namespace opens.
    anchor = "namespace vec_deque_tests_port {"
    if anchor in text:
        text = text.replace(anchor, anchor + "\n" + HELPERS, 1)
    else:
        print("warning: namespace anchor missing", file=sys.stderr)

    if AUTO_TESTS:
        text = stub_tests(text, AUTO_TESTS, "untypable auto (inference gap)")
    path.write_text(text)


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("cpp_out", type=Path)
    args = p.parse_args()
    target = args.cpp_out / "vec_deque_tests_port.cppm"
    if not target.exists():
        print(f"error: {target} not found")
        return 1
    apply_patches(target)
    print(f"vec_deque_tests_port patches applied to {target.name}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
