#!/usr/bin/env python3
"""Post-transpile patches for rc_tests_port (REAL translation).

Transpile + patch flow (transpile WITHOUT --expand; cargo-expand strips
#[test] items). new_cyclic tests need RUSTY_CPP_DUMP_AUTO=1 to let the
Rc<auto> placeholders through (those tests are stubbed below):

    RUSTY_CPP_DUMP_AUTO=1 ./target/release/rusty-cpp-transpiler \
        --crate <tgt>/Cargo.toml --output-dir <tgt>/cpp_out --auto-namespace
    python3 docs/rc_tests_port/post_transpile_patch.py <tgt>/cpp_out
    cp <tgt>/cpp_out/rc_tests_port.cppm transpiled/rc_tests_port/
"""
import argparse, re, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from _test_port_helpers import (  # noqa
    inject_test_runner_include,
    inject_module_imports,
    stub_tests,
)

# Tests requiring Rc<[T]>/Rc<str>/Rc<dyn> — DST support the port does
# not have (port Rc is a thin pointer over a sized T).
DST_TESTS = [
    "uninhabited",
    "slice",
    "trait_object",
    "shared_from_iter_normal",
    "shared_from_iter_trustedlen_normal",
    "shared_from_iter_trustedlen_panic",
    "shared_from_iter_trustedlen_no_fuse",
    "test_into_from_raw_unsized",
    "test_into_from_weak_raw_unsized",
    "test_cowrc_unsized",
    "test_unsized",
    "test_maybe_thin_unsized",
    "test_from_str",
    "test_copy_from_slice",
    "test_clone_from_slice",
    "test_clone_from_slice_panic",
    "test_from_box_str",
    "test_from_box_slice",
    "test_from_box_trait",
    "test_from_box_trait_zero_sized",
    "test_from_vec",
    "test_downcast",
    "test_array_from_slice",
    "test_unique_rc_unsizing_coercion",
]

# Rc::new_cyclic closure-return inference missing (transpiler emits
# Rc<auto>; needs closure-return T back-prop).
NEW_CYCLIC_TESTS = [
    "test_rc_cyclic_with_zero_refs",
    "test_rc_cyclic_with_one_ref",
    "test_rc_cyclic_with_two_ref",
]

# Tests whose Rust bodies define fn-LOCAL `impl` blocks — the
# transpiler skips nested impl blocks in local scope.
LOCAL_IMPL_TESTS = [
    "partial_eq",
    "eq",
    "panic_no_leak",
]

# Port has no std::alloc::System allocator.
ALLOC_TESTS = [
    "test_unique_rc_with_alloc_drops_contents",
]


def apply_patches(path: Path) -> None:
    text = path.read_text()
    text = inject_test_runner_include(text)
    text = inject_module_imports(text, "rc_tests_port", [])

    # Kill the pin-coercion helpers (unsized-coercion signatures the
    # port cannot express).
    out = []
    it = iter(text.splitlines(True))
    for line in it:
        stripped = line.strip()
        if ("pin_rc(" in stripped or "pin_unique_rc(" in stripped) and (
            stripped.startswith("export ") or stripped.startswith("rusty::pin::Pin")
        ):
            out.append("// [rc_tests_port] dropped (pin-coercion helper)\n")
            if stripped.endswith("{"):
                depth = 1
                for body_line in it:
                    depth += body_line.count("{") - body_line.count("}")
                    if depth <= 0:
                        break
            continue
        out.append(line)
    text = "".join(out)

    # Emitted `std::rc::` qualifiers — the port lives in rusty::rc.
    text = text.replace("std::rc::", "rusty::rc::")

    # `Weak::new_()` bare (weak_may_dangle: Rust infers Weak<&'a str>).
    text = text.replace("auto val = Weak::new_();",
                        "auto val = Weak<std::string_view>::new_();", 1)
    # weak_counts: port Weak's counts are MEMBERS — call on the temp.
    text = re.sub(r"Weak::(weak|strong)_count\(Weak<uint64_t>::new_\(\)\)",
                  r"(Weak<uint64_t>::new_()).\1_count()", text)
    # test_strong_count's `b.strong_count()` / `c.strong_count()`: the
    # receivers are Rc, whose counts are STATICS (this_) — spell the
    # static call. Weak receivers (w/w2) keep the member form.
    text = re.sub(r"\b([bc])\.(strong|weak)_count\(\)",
                  r"std::remove_cvref_t<decltype(\1)>::\2_count(\1)", text)
    # is_unique's fn-local generic helper leaked its Rust type param T.
    text = re.sub(r"Rc<T>::(weak|strong)_count\(this_\)",
                  r"std::remove_cvref_t<decltype(this_)>::\1_count(this_)", text)
    # from_raw element type was derived from the POINTER argument.
    text = re.sub(r"Rc<std::remove_cvref_t<decltype\(\(([a-z_0-9]+)\)\)>>::from_raw",
                  r"Rc<std::remove_cvref_t<std::remove_pointer_t<decltype(\1)>>>::from_raw",
                  text)
    # Weak::into_raw is a consuming MEMBER in the port.
    text = re.sub(r"Weak::into_raw\(std::move\(([a-z_0-9]+)\)\)",
                  r"std::move(\1).into_raw()", text)
    # try_unwrap element type was derived from the Rc ITSELF.
    text = re.sub(r"Rc<std::remove_cvref_t<decltype\(\(([a-z_0-9]+)\)\)>>::try_unwrap",
                  r"std::remove_cvref_t<decltype(\1)>::try_unwrap", text)
    text = re.sub(
        r"Weak::from_raw\(std::move\(([a-z_0-9]+)\)\)",
        r"rusty::rc::Weak<std::remove_cvref_t<std::remove_pointer_t<decltype(\1)>>>"
        r"::from_raw(std::move(\1))",
        text)
    # into_from_raw: Ok("hello") must carry the mapped T (string_view),
    # not const char* (the Ok ctor's reference param rejects the decay).
    text = text.replace('rusty::Ok("hello")', 'rusty::Ok(std::string_view("hello"))')
    # Rust `.map(|x| *x)` moves the value OUT (T by value); the emitted
    # decltype(auto) lambda returns a reference, making Result<T&>.
    text = text.replace(
        ".map([&](auto&& x) -> decltype(auto) { return rusty::deref_mut(x); })",
        ".map([&](auto&& x) -> auto { return rusty::deref_mut(x); })")
    # make_mut takes the Rc by lvalue ref, not address-of.
    text = text.replace("::make_mut(&", "::make_mut(")
    # UniqueRc alias spellings without type args.
    text = text.replace("rusty::rc::UniqueRc::new_(", "unique_rc_new(")
    text = re.sub(r"rusty::rc::UniqueRc::downgrade\(([a-z_0-9]+)\)",
                  r"std::remove_cvref_t<decltype(\1)>::downgrade(\1)", text)
    text = re.sub(r"rusty::rc::UniqueRc::into_rc\(std::move\(([a-z_0-9]+)\)\)",
                  r"std::remove_cvref_t<decltype(\1)>::into_rc(std::move(\1))", text)

    # Helper: UniqueRc::new(v) — Rust infers T; supply it from the arg.
    anchor = "namespace rc_tests_port {"
    helper = (
        "\n// UniqueRc::new(v) — Rust infers T; supply it from the argument.\n"
        "template<typename T>\n"
        "static rusty::rc::UniqueRc<T, rusty::alloc::Global> unique_rc_new(T v) {\n"
        "    return rusty::rc::UniqueRc<T, rusty::alloc::Global>::new_(std::move(v));\n"
        "}\n")
    if anchor in text:
        text = text.replace(anchor, anchor + helper, 1)
    else:
        print("warning: namespace anchor missing", file=sys.stderr)

    text = stub_tests(text, ALLOC_TESTS, "std::alloc::System allocator not in port")
    text = stub_tests(text, DST_TESTS, "Rc<[T]>/Rc<str>/Rc<dyn> (DST) not in port")
    text = stub_tests(text, NEW_CYCLIC_TESTS,
                      "Rc::new_cyclic closure-return T inference missing")
    text = stub_tests(text, LOCAL_IMPL_TESTS, "fn-local impl blocks skipped by transpiler")
    path.write_text(text)


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("cpp_out", type=Path)
    args = p.parse_args()
    target = args.cpp_out / "rc_tests_port.cppm"
    if not target.exists():
        print(f"error: {target} not found")
        return 1
    apply_patches(target)
    print(f"rc_tests_port patches applied to {target.name}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
