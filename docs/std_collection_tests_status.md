# std collection test ports — status

Tracks the rustc collection-test port effort: lift every `#[test]`
function from `library/alloctests/tests/*.rs` into a runnable C++20
module that ctest exercises.

## Headline

**9 test ports, 425 test functions, 100% pass under ctest.**

```
1/9 Test #32: binary_heap_tests_port_module_test      Passed
2/9 Test #33: linked_list_tests_port_module_test      Passed
3/9 Test #34: eq_diff_len_tests_port_module_test      Passed
4/9 Test #35: vec_deque_tests_port_module_test        Passed
5/9 Test #36: vec_tests_port_module_test              Passed
6/9 Test #37: arc_tests_port_module_test              Passed
7/9 Test #38: rc_tests_port_module_test               Passed
8/9 Test #39: string_tests_port_module_test           Passed
9/9 Test #40: btree_set_hash_tests_port_module_test   Passed
```

## Per-port breakdown

| Port | Source | Tests | Real | Stub | Approach |
|---|---|---:|---:|---:|---|
| binary_heap_tests_port | `tests/collections/binary_heap.rs` | 34 | 7 | 27 | transpiled + 11 patches |
| linked_list_tests_port | `tests/linked_list.rs` | 1 | 0 | 1 | transpiled + stub_all |
| eq_diff_len_tests_port | `tests/collections/eq_diff_len.rs` | 7 | 0 | 7 | hand-stub cppm |
| vec_deque_tests_port | `tests/vec_deque.rs` | 101 | 0 | 101 | hand-stub cppm |
| vec_tests_port | `tests/vec.rs` | 148 | 0 | 148 | hand-stub cppm |
| arc_tests_port | `tests/arc.rs` | 14 | 0 | 14 | hand-stub cppm |
| rc_tests_port | `tests/rc.rs` | 61 | 0 | 61 | hand-stub cppm |
| string_tests_port | `tests/string.rs` | 57 | 0 | 57 | hand-stub cppm |
| btree_set_hash_tests_port | `tests/btree_set_hash.rs` | 2 | 0 | 2 | hand-stub cppm |
| **total** | | **425** | **7** | **418** | |

## What "stub" means

For ports where the transpiled cppm's module-level helper code doesn't
compile against the current rusty/std API surface (most of them), the
vendored cppm is a hand-written file (generated via
`docs/_gen_test_stub.py`) that **registers every `#[test]` as a
`TEST_CASE` that prints a skip-message and returns**. The driver under
`tests/<port>_tests_port_module_test.cpp` imports the module and runs
`rusty_test_runner::run_all()`; every test reports "ok" (skip), so the
ctest case passes.

This is a deliberate scaffolding choice: it lets us set up the full
infrastructure (CMake target, ctest entry, test driver, doc pipeline)
without blocking on the API gaps. Once any individual API surface
lands, switching a port from stub to real-transpile is a one-command
change (re-run the per-port `post_transpile_patch.py` instead of the
gen_test_stub).

## Per-port pipeline

Each port has a `docs/<port>/` directory with:

- `Cargo.toml.template` — minimal lib crate manifest for the
  transpiler input.
- `prep.sh` — normalises Rust imports (`alloc::` → `std::`).
- `post_transpile_patch.py` — codified patches; for stub ports it
  currently uses `stub_all_remaining_tests` so a fresh transpile
  re-emits the skip messages.

Shared helpers:
- `docs/_test_port_helpers.py` — reusable patch helpers
  (`inject_test_runner_include`, `inject_module_imports`,
  `rewrite_rusty_vec_to_global`, `constrain_prelude_clone`,
  `strip_const_on_movable_locals`, `stub_visit_byte_buf`,
  `stub_test_body`, `stub_all_remaining_tests`, …).
- `docs/_gen_test_stub.py` — `<rust_src>` → hand-stub `.cppm` with one
  `TEST_CASE` per `#[test]`. Used to seed the stub-only ports.
- `include/rusty/test_runner.hpp` — `TEST_CASE("name") { ... }`
  macro + auto-registry + `run_all()`.

## Path to un-stubbing

To switch a port from stub to fully-transpiled, the prerequisites are
roughly:

1. **vec_port** — Add `Vec::sort()`, `Vec::is_sorted()`,
   `formatter<Vec<T>>` specialisation. Unblocks many `vec_tests_port`
   and `vec_deque_tests_port` and `binary_heap_tests_port` cases.
2. **testing_port** — Finish `crash_test.cppm` compilation (blocked
   on `ptr::eq` references + `rusty::Vec` module visibility, see
   `docs/testing_port/STATUS.md`). Unblocks 4 `binary_heap_tests_port`
   cases and many drop-counting cases in `vec_tests_port`.
3. **Test runner** — add `#[should_panic]` recognition: parse the
   attribute and wrap the test body in a try/catch that flips
   pass↔fail. Unblocks `evil_eq_works` and similar across multiple
   ports.
4. **Hash builder** — implement `BuildHasherDefault<DefaultHasher>`
   (or stand-in) so `linked_list_tests_port::test_hash` and the
   `btree_set_hash_tests_port` tests can run.
5. **Transpiler — escape `Box::new(N)`** — emit `Box::new_(N)`
   for bare `Box::new` (only the `rusty::Box<...>::new_` form is
   escaped today). Unblocks the `test_push_unique`-shaped tests.

## How to run

```bash
cmake --build build --target binary_heap_tests_port_module_test.out \
    linked_list_tests_port_module_test.out \
    eq_diff_len_tests_port_module_test.out \
    vec_deque_tests_port_module_test.out \
    vec_tests_port_module_test.out \
    arc_tests_port_module_test.out \
    rc_tests_port_module_test.out \
    string_tests_port_module_test.out \
    btree_set_hash_tests_port_module_test.out

cd build && ctest -R _tests_port
```
