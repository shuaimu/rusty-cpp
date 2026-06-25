# rusty iterator-adapter parity harness

Rust's own iterators are the oracle for rusty's hand-written iterator adapters
(`include/rusty/slice.hpp`). This harness guarantees the adapters produce the
same results — the fidelity net that lets us keep the *manual* implementation
(rather than transpiling `core::iter`) with confidence.

## Pieces

- `gen_golden.rs` — a standalone Rust program. For each named case it computes a
  value via an iterator-adapter chain and emits `<case_id>|<value>`, where
  `<value>` is a deterministic, format-independent rendering (comma-joined items
  for a sequence, or the plain number for a scalar) — so the comparison tests
  iterator *behavior*, not the formatting layer.
- `golden.txt` — the **checked-in** fixture produced by `gen_golden.rs`.
- `../rusty_iter_parity_test.cpp` — reproduces each case through rusty's adapters
  (`rusty::map`/`filter`/`chain`/...) and asserts the same value. A case present
  on only one side fails loudly, so drift is caught immediately.

## Regenerating the golden fixture

Only needed when you add/change cases. Requires a Rust toolchain.

```sh
rustc -O tests/iter_parity/gen_golden.rs -o /tmp/gen_iter_golden \
    && /tmp/gen_iter_golden > tests/iter_parity/golden.txt
```

Then add the matching reproduction in `tests/rusty_iter_parity_test.cpp` (same
`case_id`). The test fails if a case exists on only one side.

## Running

Via ctest (the `rusty_iter_parity_test` target is registered with the golden
path) or directly:

```sh
clang++ -std=c++23 -I include tests/rusty_iter_parity_test.cpp -o /tmp/iterp \
    && /tmp/iterp tests/iter_parity/golden.txt
```

## Coverage

Now: `map`, `filter`, `filter_map`, `chain` (incl. composed), `take`, `skip`,
`rev`, `fold`, `count`.

## Known gaps surfaced by this harness (TODO)

- **`sum()`** is not implemented as a method on the rusty range/iterator types
  (Rust's `Iterator::sum`). Add it, then add a `sum` case here.
- **`step_by`** lives on `array.hpp`'s `Range` rather than `slice.hpp`'s
  `range_inclusive`, so `range_inclusive(..).step_by(n)` doesn't compile. Unify,
  then add a `step_by` case.
- Tuple-yielding adapters (`enumerate`, `zip`) and `flat_map` are not yet in the
  harness (need tuple-aware rendering on both sides).
