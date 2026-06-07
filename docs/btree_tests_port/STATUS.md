# btree_tests_port — un-stub status

Living log of which rustc tests are un-stubbed vs why others aren't.

## Pipeline shape

- `transpiled/btree_tests_port/btree_tests_port.cppm` — 146 auto-generated
  `TEST_CASE("…")` stubs (115 from map/tests.rs, 31 from set/tests.rs
  prefixed with `set_`).
- `tests/btree_tests_port_unstubbed.cpp` — hand-translated test bodies
  that actually exercise btree_port. Lives in a separate TU because of
  the "ManuallyDrop instantiation" bug (see below).
- `tests/btree_tests_port_module_test.cpp` — runner driver.

Un-stubbed tests use `_unstubbed` suffix on their name so the
test-runner registry doesn't collide with the stub of the same Rust
test name. Both run on each invocation.

## Un-stubbed so far

| Rust test | C++ TEST_CASE | Status |
|---|---|---|
| `map/tests.rs::test_get_key_value` | `test_get_key_value_unstubbed` | passing (trimmed: removed `map.remove()` tail — blocked by clear() bug below) |

## Known blockers found while un-stubbing

These each block a small cluster of tests. Each can be fixed
independently in btree_port.

### B-clear: `BTreeMap::clear()` clones ManuallyDrop directly

**Symptom:** `map.cppm:5579` emits
```cpp
rusty::mem::drop(BTreeMap<K, V, A>(
    rusty::mem::replace(this->root, …),
    rusty::mem::replace(this->length, 0),
    rusty::clone(this->alloc),   // ← this->alloc is ManuallyDrop<A>
    …));
```
`rusty::clone<ManuallyDrop<Global>>` triggers a `static_assert` on
`is_copy_constructible_v<ManuallyDrop<Global>>` (ManuallyDrop has a
deleted copy ctor by design).

**Fix:** rewrite the clear() emit (or hand-patch in
post_transpile_patch.py) to unwrap, clone, re-wrap:
```cpp
rusty::mem::manually_drop_new(rusty::clone(*this->alloc))
```

**Tests blocked:** `set_test_clear`, `test_clear` (map), and anything
else that calls `.clear()`. Indirectly blocks `into_keys()`,
`into_values()`, and any other method whose destructor reaches the
same code path.

### B-into-iter: into_keys/into_values destructor reaches B-clear

**Symptom:** `into_keys()` / `into_values()` instantiate the BTreeMap
destructor in a context that also hits the B-clear emit. Same
underlying bug, surfaces differently.

**Tests blocked:** `test_into_keys`, `test_into_values`, anything else
exercising `into_*()` consuming iterators.

### B-purview: in-module-purview instantiation

**Symptom:** TEST_CASE bodies that instantiate BTreeMap inside
`btree_tests_port.cppm`'s module purview hit the B-clear destructor
chain at module-build time, even if the body itself doesn't touch
`.clear()` (because the destructor is implicitly instantiated). The
same body in a separate `.cpp` TU compiles fine if it doesn't
explicitly call `.clear()`.

**Workaround:** un-stubbed test bodies live in
`tests/btree_tests_port_unstubbed.cpp`, not in the module. This is
purely a workaround until B-clear is fixed (then both paths work).

## Roadmap

Fixing B-clear unblocks roughly 20-30 of the 146 tests in one shot
(everything that hits `.clear()`, `into_keys()`, `into_values()`,
`pop_first()`/`pop_last()` destructor paths, etc.).

After that, the remaining blockers are the ones flagged in
`post_transpile_patch.py`'s docstring:
- `crate::testing::{crash_test, ord_chaos}` helpers (not in
  transpiled/testing_port — would need to vendor them).
- Private BTreeMap invariant-check methods (`check_invariants`,
  `assert_back_pointers`, `calc_length`, `assert_min_len`).
- `catch_unwind` / `AssertUnwindSafe` (Rust panic-unwinding has no
  C++ equivalent in this codebase).

Tests that only need basic CRUD + iteration with `_unstubbed` suffix
can keep being added to `btree_tests_port_unstubbed.cpp` without
waiting for any of the above.
