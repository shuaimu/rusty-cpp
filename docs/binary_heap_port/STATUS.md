# BinaryHeap port — Phase A1 (transpile + early compile)

This directory holds the scaffolding for the rustc
`alloc::collections::binary_heap` port — Tier 2 in
[`rusty-std-book.md`](../rusty-std-book.md) §3.3.

## Pipeline summary (per Chapter 0)

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ `library/alloc/src/collections/binary_heap/mod.rs` (2038 LOC) vendored to `/tmp/binary_heap_port/binary_heap_crate/src/lib.rs` |
| 2. Preprocessing (`prep.sh`) | ✅ Idempotent path rewrites (`crate::alloc::*` → `std::alloc::*`, `crate::vec::*` → `std::vec::*`, etc.) — see `prep.sh` |
| 3. Transpilation | ✅ **Zero transpiler errors.** Run with `--auto-namespace` so internal types (Drain, IntoIter, PeekMut, etc.) land in `namespace binary_heap_port` and don't collide with vec_port's globals. 5 hand-port slots in the slot manifest (4× `Item` assoc-type skips + 1× orphan-impl marker). |
| 4. Post-transpile patching | 🟡 **In progress.** Standard patches applied: `rusty::Vec<…>` → `::Vec<…>`, `visit_byte_buf` rebodied to stub, duplicate `clone` template stripped, `using rusty::Vec;` deleted, `import vec_port.vec; import vec_port.vec.into_iter;` added, `vec::IntoIter` / `vec::Drain` → `::IntoIter` / `::Drain`. |
| 5. Build (compile) | 🟡 **Partial.** First successful transpile compiles past the namespace cycle; remaining compile errors are the BTreeMap-port-style long-tail clusters documented below. |
| 6. Smoke test | ⏸️ Blocked on Stage 5. Library target wired into `CMakeLists.txt` as `binary_heap_port` (clang-only, links vec_port). |
| 7. Bench | ⏸️ Blocked on Stage 6. |

## Reproducing the pipeline

```bash
RUSTSRC=$(ls -d ~/.rustup/toolchains/*/lib/rustlib/src/rust/library/alloc/src/collections/binary_heap/ | head -1)
mkdir -p /tmp/binary_heap_port/binary_heap_crate/src
cp $RUSTSRC/mod.rs /tmp/binary_heap_port/binary_heap_crate/src/lib.rs
cp docs/binary_heap_port/Cargo.toml.template /tmp/binary_heap_port/binary_heap_crate/Cargo.toml

bash docs/binary_heap_port/prep.sh /tmp/binary_heap_port/binary_heap_crate/src/lib.rs

# Transpile with auto-namespace (so internal Drain/IntoIter don't collide with vec_port)
./target/release/rusty-cpp-transpiler \
    --crate /tmp/binary_heap_port/binary_heap_crate/Cargo.toml \
    --output-dir /tmp/binary_heap_port/cpp_out \
    --auto-namespace

cp /tmp/binary_heap_port/cpp_out/binary_heap_port.cppm transpiled/binary_heap_port/
```

The current vendored .cppm has the standard patches hand-applied (no
post_transpile_patch.py yet — TODO when the long-tail stabilises).

## Remaining compile-error clusters (Phase A2)

These are the same shape as the BTreeMap port's Phase A2 clusters (per
rusty-std-book §1.1 / §2.4 / §2.10), just smaller surface:

| Cluster | Example | Origin |
|---|---|---|
| **Cluster A — `const Option<NonZero<usize>>` constexpr** | `static constexpr rusty::Option<rusty::num::NonZero<size_t>> EXPAND_BY = NonZero::new_(1);` | `rusty::Option` isn't a literal type; rustc relies on `NonZeroUsize::new` being `const fn`. Same shape as BTreeMap's `MIN_LEN` dup (§2.4 cluster D). Either lower to a non-`constexpr` `static const` or hand-port the helper. |
| **Cluster B — `std::collections::TryReserveError`** | `auto try_reserve(…) -> rusty::Result<std::tuple<>, std::collections::TryReserveError>` | Transpiler emit didn't map `std::collections::TryReserveError` → `rusty::collections::TryReserveError`. Two sites. Patcher rule. |
| **Cluster C — `rusty::Vec{}` default-construct** | `return BinaryHeap<T>{.data = rusty::Vec{}};` | One site missed by the bulk `rusty::Vec → ::Vec` patch because it's `rusty::Vec` without template args (the empty default-ctor pattern). Patcher rule. |
| **Cluster D — `rusty::ptr::swap`** | `rusty::ptr::swap(elt, …)` | `swap` doesn't exist under `rusty::ptr::`; the rustc source calls `core::ptr::swap`. Need to either add it to `rusty::ptr::` or substitute `std::swap`. |
| **Cluster E — `usize` bare identifier** | `&[T; usize]` or `(usize, usize)` style emit | The transpiler emitted `usize` instead of `size_t` in one position. One-line patcher. |
| **Cluster F — `T` does not refer to a value** | `… BinaryHeap<T> { … T } …` line 4538 | Transpiler emit bug; same shape as BTreeMap's "Cluster A — undeducible impl-generic method-template params" (§2.4 cluster A). |

Slot manifest (`/tmp/binary_heap_port/cpp_out/rusty_hand_slots.md`):

- L3822, L3823, L3852, L3857 — `Rust-only associated type alias with
  unbound generic skipped` for `Item`. These are decorative; the type
  isn't actually used in the C++ output. Mark each as "intentionally
  skipped" once we audit.
- L4535 — `orphan impl: methods for Vec were declared in this file
  but the…`. Marker indicates an `impl` block that the transpiler
  couldn't emit as a real member-function set. Likely the
  `impl<T> Extend for Vec<T>` impl from the heap source; safe to
  delete since Vec already has Extend in vec_port.

## Predicted effort to Phase B (compile clean)

Per §2.8: small port (single file, no submodule cycles, no exotic
unsafe) — **1–2 days** of focused patcher work + hand-ports. Mostly
copy-paste from BTreeMap's `docs/btreemap_port/post_transpile_patch.py`
patcher rules. Probably 4–6 patcher functions: cluster B mapping,
cluster C empty-Vec ctor, cluster D `rusty::ptr::swap` injection,
cluster E `usize`/`size_t` fixup, plus the hand-edit of the 4–5 hand
slots.

## Predicted effort to Phase C (smoke test + bench)

After Phase B compiles clean, smoke test should be near-trivial —
binary heap is small, ops are `push` / `pop` / `peek` / `len`. Bench
will exercise the priority-queue insert/extract paths against Rust's
own `BinaryHeap` and against `std::priority_queue`.

Estimated: **half a day** once Phase B closes.

## Dependencies

`vec_port.vec` (full Vec definition), `vec_port.vec.into_iter` (the
`IntoIter` referenced by `BinaryHeap::into_iter()`). Both already
vendored and clean. **No need to port Drain** — the heap's drain reuses
vec_port's drain via the parent module.
