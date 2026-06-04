# VecDeque port — ✅ Phase C smoke test passes on full transpiled body

`transpiled/vec_deque_port/vec_deque_port.cppm` (5767 LOC, full
transpiled body — no stub re-export) builds clean and powers
`rusty::collections::VecDeque<T, A>`. `libvec_deque_port.a` builds;
`tests/vec_deque_port_module_test.cpp` is the Phase C smoke test.

The bridge stub (`vec_deque_port_stub.cppm`) that re-exported the
hand-written `rusty::VecDeque<T>` has been retired — the multi-
template-arg blockers in the original Phase A2/B attempt were
resolved by the same Vec/Box deduction work that landed for
binary_heap_port + vec_port.



This directory holds the scaffolding for the rustc
`alloc::collections::vec_deque` port — Tier 2 in
[`rusty-std-book.md`](../rusty-std-book.md) §3.3.

## Pipeline summary (per Chapter 0)

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ `library/alloc/src/collections/vec_deque/` (10 .rs files, 5527 LOC excluding `tests.rs`) vendored to `/tmp/vec_deque_port/vec_deque_crate/src/` |
| 2. Preprocessing (`prep.sh`) | ✅ Same idempotent rewrites as the BTreeMap port |
| 3. Transpilation | ✅ **Zero transpiler errors.** Run with `--auto-namespace`. 14 hand-port slots across 3 files. Outputs 10 `.cppm` files (per-rust-module). |
| 4. Post-transpile patching | 🟡 **Seeded.** `docs/vec_deque_port/post_transpile_patch.py` applies the binary_heap-style 14-patch set (`rusty::Vec<>` → `::Vec<>`, `usize` → `size_t`, `ptr::swap` → `std::swap`, `rusty::mem::MaybeUninit` → `rusty::MaybeUninit`, `std::Allocator`/`std::Global` → `rusty::alloc::*`). Idempotent. |
| 5. Build | 🔴 **Blocked** — `rusty::VecDeque<T>` only takes one template arg but the transpiled code emits `VecDeque<T, A>` (same shape as rc_port's blocker). |
| 6. Smoke test | ⏸️ |
| 7. Bench | ⏸️ |

## Reproducing the pipeline

```bash
RUSTSRC=$(ls -d ~/.rustup/toolchains/*/lib/rustlib/src/rust/library/alloc/src/collections/vec_deque/ | head -1)
mkdir -p /tmp/vec_deque_port/vec_deque_crate/src
for f in $RUSTSRC/*.rs; do
  [ "$(basename $f)" = "tests.rs" ] && continue
  cp $f /tmp/vec_deque_port/vec_deque_crate/src/
done
mv /tmp/vec_deque_port/vec_deque_crate/src/mod.rs /tmp/vec_deque_port/vec_deque_crate/src/lib.rs
cp docs/vec_deque_port/Cargo.toml.template /tmp/vec_deque_port/vec_deque_crate/Cargo.toml

bash docs/vec_deque_port/prep.sh /tmp/vec_deque_port/vec_deque_crate/src/

./target/release/rusty-cpp-transpiler \
    --crate /tmp/vec_deque_port/vec_deque_crate/Cargo.toml \
    --output-dir /tmp/vec_deque_port/cpp_out \
    --auto-namespace

cp /tmp/vec_deque_port/cpp_out/*.cppm transpiled/vec_deque_port/
```

## Module layout (output)

```
vec_deque_port.cppm                   # umbrella; export imports the others
vec_deque_port.drain.cppm
vec_deque_port.extract_if.cppm
vec_deque_port.into_iter.cppm
vec_deque_port.iter.cppm
vec_deque_port.iter_mut.cppm
vec_deque_port.macros.cppm            # 19 LOC source; empty after macro expansion
vec_deque_port.spec_extend.cppm
vec_deque_port.spec_from_iter.cppm
vec_deque_port.splice.cppm
```

All under `namespace vec_deque_port { … }` thanks to `--auto-namespace`.
Internal types (`IntoIter`, `Drain`, etc.) are namespaced — no collision
with vec_port's globals.

## Remaining Phase B blockers

1. **Hand-written `rusty::VecDeque<T>` is single-arg**; transpiled uses `VecDeque<T, A>`. Either extend hand-written to `<T, A = Global>` (mirror vec_port's path), or hand-port to `vec_deque_port::VecDeque<T, A>` and stop aliasing.
2. **`std::Allocator` / `std::Global` references** — the transpiler emits Rust's `alloc::Allocator` / `alloc::Global` as if in C++ `std::`. Patcher handles top-level cases; some embedded in template params (e.g. `extend<…, std::Allocator A2>`) need template-aware rewrite.
3. **Cross-port import injection** — vec_deque references `::Vec` / `::IntoIter` but the umbrella doesn't `import vec_port.vec;`. Same pattern binary_heap solved manually; vec_deque needs it codified in the patcher.
4. **Cluster A residue** — `expected unqualified-id` at line 48/57/60 in `spec_extend.cppm` likely auto-template-arg leakage from absorbed methods.

## Predicted Phase B effort

Same shape as rc_port: **5–8 days** to extend `rusty::VecDeque<T, A>`,
inject cross-port imports correctly, fix template-aware Allocator
rewrites, and chase the Cluster A residue.

## Dependencies

`vec_port.vec` (since VecDeque is "a Vec with two cursors" under the
hood — though Rust's impl uses raw allocator directly rather than Vec).
Actually it uses `RawVec` internally, so `vec_port.raw_vec` is the main
dependency.

## Hand-port slots

See `/tmp/vec_deque_port/cpp_out/rusty_hand_slots.md` — 14 slots across
3 files. Skim suggests most are the `Item` associated-type alias
markers (decorative); the rest will need audit when Phase B starts.

## Replaces

`include/rusty/vecdeque.hpp` (hand-written) — to be retired once the
transpiled port reaches Phase B + matches the hand-written test suite,
following the VecLegacy retirement playbook from the previous commit.
