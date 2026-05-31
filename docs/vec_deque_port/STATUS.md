# VecDeque port — Phase A1 (transpile clean)

This directory holds the scaffolding for the rustc
`alloc::collections::vec_deque` port — Tier 2 in
[`rusty-std-book.md`](../rusty-std-book.md) §3.3.

## Pipeline summary (per Chapter 0)

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ `library/alloc/src/collections/vec_deque/` (10 .rs files, 5527 LOC excluding `tests.rs`) vendored to `/tmp/vec_deque_port/vec_deque_crate/src/` |
| 2. Preprocessing (`prep.sh`) | ✅ Same idempotent rewrites as the BTreeMap port |
| 3. Transpilation | ✅ **Zero transpiler errors.** Run with `--auto-namespace`. 14 hand-port slots across 3 files. Outputs 10 `.cppm` files (per-rust-module). |
| 4. Post-transpile patching | ⏸️ Not yet attempted. Same pattern as binary_heap_port — will need: `rusty::Vec` → `::Vec` bulk rename, `visit_byte_buf` prelude stub, duplicate `clone` template strip, `using rusty::Vec;` deletion, `import vec_port.vec; import vec_port.vec.into_iter;` additions, `vec::IntoIter` / `vec::Drain` → `::IntoIter` / `::Drain` fixups. |
| 5. Build | ⏸️ Not wired into `CMakeLists.txt` yet. |
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

## Predicted effort to Phase B (compile clean)

Per §2.8: medium port, multi-file but acyclic-ish. The post-transpile
patcher will be the lift, similar in complexity to vec_port's patcher.
Likely **3–5 days** of patcher iteration + hand-ports.

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
