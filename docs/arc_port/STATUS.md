# Arc port — Phase A1 (transpile clean)

Vendored `library/alloc/src/sync.rs` (4936 LOC) → `transpiled/arc_port/arc_port.cppm`.
Transpiled with `--auto-namespace`: zero errors, 7 hand-port slots. See
[`rusty-std-book.md`](../rusty-std-book.md) §3.4 (Tier 3) for the
rationale.

## Pipeline summary

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ |
| 2. Prep | ✅ |
| 3. Transpile | ✅ Zero errors, 7 hand-slots |
| 4. Patcher | ⏸️ |
| 5. Build | ⏸️ |

## Reproducing

```bash
RUSTSRC=$(ls -d ~/.rustup/toolchains/*/lib/rustlib/src/rust/library/alloc/src/ | head -1)
mkdir -p /tmp/arc_port/arc_crate/src
cp $RUSTSRC/sync.rs /tmp/arc_port/arc_crate/src/lib.rs
cp docs/arc_port/Cargo.toml.template /tmp/arc_port/arc_crate/Cargo.toml
bash docs/arc_port/prep.sh /tmp/arc_port/arc_crate/src/lib.rs
./target/release/rusty-cpp-transpiler --crate /tmp/arc_port/arc_crate/Cargo.toml \
    --output-dir /tmp/arc_port/cpp_out --auto-namespace
cp /tmp/arc_port/cpp_out/*.cppm transpiled/arc_port/
```

## Predicted Phase B effort

Per §2.8: **3-5 days** — single file but atomics-everywhere; memory
ordering matters; ABA-style concerns on `upgrade()`. Hand-written
`rusty::Arc` has known rough edges (see commit `ddee375`); transpiling
could nail down the exact ordering rustc uses, which is the main
value-add over the existing header.
