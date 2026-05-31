# LinkedList port — Phase A1 (transpile clean)

This directory holds the scaffolding for the rustc
`alloc::collections::linked_list` port — Tier 2 in
[`rusty-std-book.md`](../rusty-std-book.md) §3.3.

## Pipeline summary (per Chapter 0)

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ `library/alloc/src/collections/linked_list.rs` (2255 LOC) vendored to `/tmp/linked_list_port/linked_list_crate/src/lib.rs` |
| 2. Preprocessing (`prep.sh`) | ✅ |
| 3. Transpilation | ✅ **Zero transpiler errors.** Run with `--auto-namespace`. 1 hand-port slot. Single `.cppm` file. |
| 4. Post-transpile patching | ⏸️ Not yet attempted. |
| 5. Build | ⏸️ Not wired into `CMakeLists.txt` yet. |
| 6. Smoke test | ⏸️ |
| 7. Bench | ⏸️ |

## Reproducing the pipeline

```bash
RUSTSRC=$(ls -d ~/.rustup/toolchains/*/lib/rustlib/src/rust/library/alloc/src/collections/ | head -1)
mkdir -p /tmp/linked_list_port/linked_list_crate/src
cp $RUSTSRC/linked_list.rs /tmp/linked_list_port/linked_list_crate/src/lib.rs
cp docs/linked_list_port/Cargo.toml.template /tmp/linked_list_port/linked_list_crate/Cargo.toml

bash docs/linked_list_port/prep.sh /tmp/linked_list_port/linked_list_crate/src/lib.rs

./target/release/rusty-cpp-transpiler \
    --crate /tmp/linked_list_port/linked_list_crate/Cargo.toml \
    --output-dir /tmp/linked_list_port/cpp_out \
    --auto-namespace

cp /tmp/linked_list_port/cpp_out/*.cppm transpiled/linked_list_port/
```

## Predicted effort to Phase B (compile clean)

Per §2.8: small-to-medium port — single file, intrusive doubly-linked
list with raw-pointer plumbing. Cursor API uses unsafe heavily; that's
where the hand-port effort will land. Likely **2–3 days**.

## Dependencies

`alloc::boxed::Box` (heap node allocation). The hand-written
`rusty::Box` should be enough — no `vec_port.vec` dep.

## Net-new functionality

No hand-written `rusty::LinkedList` exists yet — this port adds a new
collection type to the rusty library. The cursor API (`CursorMut::insert_before`,
etc.) is the value-add over C++'s `std::list`.
