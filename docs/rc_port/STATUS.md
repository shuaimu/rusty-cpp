# Rc port — Phase A1 (transpile clean)

Vendored `library/alloc/src/rc.rs` (4565 LOC) → `transpiled/rc_port/rc_port.cppm`.
Transpiled with `--auto-namespace`: zero errors, 4 hand-port slots. See
[`rusty-std-book.md`](../rusty-std-book.md) §3.4 (Tier 3) for the
rationale — there's a working hand-written `rusty::Rc` in `rc.hpp`, so
this port is *opportunistic*: validates the transpiler against
single-threaded refcount + cycle-detection unsafe code rather than
displacing the hand-written version on day 1.

## Pipeline summary

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ |
| 2. Prep | ✅ |
| 3. Transpile | ✅ Zero errors, 4 hand-slots |
| 4. Patcher | ⏸️ Not started |
| 5. Build | ⏸️ |

## Reproducing

```bash
RUSTSRC=$(ls -d ~/.rustup/toolchains/*/lib/rustlib/src/rust/library/alloc/src/ | head -1)
mkdir -p /tmp/rc_port/rc_crate/src
cp $RUSTSRC/rc.rs /tmp/rc_port/rc_crate/src/lib.rs
cp docs/rc_port/Cargo.toml.template /tmp/rc_port/rc_crate/Cargo.toml
bash docs/rc_port/prep.sh /tmp/rc_port/rc_crate/src/lib.rs
./target/release/rusty-cpp-transpiler --crate /tmp/rc_port/rc_crate/Cargo.toml \
    --output-dir /tmp/rc_port/cpp_out --auto-namespace
cp /tmp/rc_port/cpp_out/*.cppm transpiled/rc_port/
```

## Predicted Phase B effort

Per §2.8: **2-3 days** — single file, lots of unsafe pointer-arith +
drop-ordering work; Cluster A (impl-generic method-template params)
is likely to apply.
