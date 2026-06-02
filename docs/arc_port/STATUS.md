# Arc port — ✅ Phase B + C via bridge stub (full transpiled body still WIP)

The full transpiled `arc_port.cppm` has the same shape of transpiler-side
blockers as rc_port plus atomics-specific issues. To unblock consumers,
**`transpiled/arc_port/arc_port_stub.cppm`** re-exports hand-written
`rusty::Arc<T>` under `arc_port::Arc<T, A=Global>`. `libarc_port.a`
builds; `tests/arc_port_module_test.cpp` proves it (Arc<int>(7) + clone).



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
| 4. Patcher | 🟡 **Seeded.** `docs/arc_port/post_transpile_patch.py` mirrors rc_port's namespace fixups (borrow/string/Vec/ptr::Alignment/mem::MaybeUninit). |
| 5. Build | 🔴 **Blocked** on rusty:: API gaps (Vec / Layout::for_value_raw / ptr::from_ref / Arc::is etc.) — Cluster A `Box<auto>` regression for `try_new`/`try_new_in`/`new_in` is **resolved transpiler-side** (see `transpiler/src/codegen.rs` near the explicit Box arg-inference branch). The patched `arc_port.cppm` lives next to the stub as `arc_port.cppm.wip` until the rusty:: API surface catches up. |

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

## Remaining Phase B blockers

~~Cluster A regression~~ ✅ resolved transpiler-side: `try_new` /
`try_new_in` / `new_in` / `new_uninit_in` / `try_new_uninit_in` /
`new_zeroed_in` / `try_new_zeroed_in` now follow the same arg-inference
path as `new` / `new_` / `make`, taking the Box<auto> count from 10 → 0
after the patcher runs.

Same set as rc_port (single- vs two-template-arg Arc<T>, missing
`NonNull::cast<>()`, cross-port Cell/UnsafeCell signature drift) PLUS
arc-specific:

- **Memory ordering helpers** — rustc uses `core::sync::atomic::Ordering::{Acquire,Release,SeqCst,Relaxed}`; our `rusty::atomic` either doesn't surface these or surfaces them as different names.
- **`compare_exchange_weak` overload mismatch** — rustc's `AtomicUsize::compare_exchange_weak(curr, new, succ, fail)` takes two orderings; need to verify our binding.
- **`AtomicPtr::store/load` argument forwarding** — refcount-bump paths need careful ordering preservation.

## Predicted Phase B effort

**8–12 days** — single file but atomics-heavy. Beyond rc_port's
5-8 days because each ordering site needs hand-audit.
