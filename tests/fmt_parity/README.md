# rusty::fmt parity harness

Rust's `format!` is the oracle for the self-contained `rusty::fmt` runtime
(`include/rusty/fmt.hpp`). This harness guarantees byte-identical output.

## Pieces

- `gen_golden.rs` — a standalone Rust program. For each named case it emits the
  exact bytes of Rust's `format!`, hex-encoded, as `<case_id>|<hex>`.
- `golden.txt` — the **checked-in** fixture produced by `gen_golden.rs`.
- `../rusty_fmt_parity_test.cpp` — reproduces each case through `rusty::fmt` and
  asserts byte-identical output (also flags cases present on one side only).

## Regenerating the golden fixture

Only needed when you add/change cases. Requires a Rust toolchain.

```sh
rustc -O tests/fmt_parity/gen_golden.rs -o /tmp/gen_golden \
    && /tmp/gen_golden > tests/fmt_parity/golden.txt
```

Then add the matching reproduction in `tests/rusty_fmt_parity_test.cpp` (same
`case_id`). The test fails loudly if a case exists on only one side, so drift is
caught immediately.

## Running

Via ctest (the `rusty_fmt_parity_test` target is registered with the golden
path) or directly:

```sh
clang++ -std=c++23 -I include tests/rusty_fmt_parity_test.cpp -o /tmp/fmtp \
    && /tmp/fmtp tests/fmt_parity/golden.txt
```

## Scope by phase

- Phase 0 (now): string Display via `Formatter::pad` — width / precision / fill
  / alignment.
- Later: integers (all bases + flags), `bool`/`char` (Debug escaping), the Debug
  builders, and f32/f64 shortest round-trip (the float cases will also fuzz
  random bit patterns for round-trip correctness, not just match Rust).
