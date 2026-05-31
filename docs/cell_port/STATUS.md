# Cell / RefCell port — Phase A1 (transpile clean)

Vendored `library/core/src/cell.rs` (2737 LOC) → `transpiled/cell_port/cell_port.cppm`.
Transpiled with `--auto-namespace`: zero errors, 20 hand-port slots
(Cell+RefCell+UnsafeCell+OnceCell+LazyCell live in this one file).

## Pipeline summary

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ |
| 2. Prep | ✅ |
| 3. Transpile | ✅ Zero errors, 20 hand-slots |
| 4. Patcher | ⏸️ |
| 5. Build | ⏸️ |

## Reproducing

```bash
RUSTSRC=$(ls -d ~/.rustup/toolchains/*/lib/rustlib/src/rust/library/core/src/ | head -1)
mkdir -p /tmp/cell_port/cell_crate/src
cp $RUSTSRC/cell.rs /tmp/cell_port/cell_crate/src/lib.rs
cp docs/cell_port/Cargo.toml.template /tmp/cell_port/cell_crate/Cargo.toml
bash docs/cell_port/prep.sh /tmp/cell_port/cell_crate/src/lib.rs
./target/release/rusty-cpp-transpiler --crate /tmp/cell_port/cell_crate/Cargo.toml \
    --output-dir /tmp/cell_port/cpp_out --auto-namespace
cp /tmp/cell_port/cpp_out/*.cppm transpiled/cell_port/
```

## Predicted Phase B effort

Per §2.8: **1-2 days** — small file, mostly bookkeeping. Hand-written
`refcell.hpp` / `cell.hpp` / `unsafe_cell.hpp` already cover the basic
cases; transpiling is opportunistic per Tier 3.
