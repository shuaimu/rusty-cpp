# LinkedList port — ✅ Phase B + C via bridge stub (full transpiled body blocked on Cluster A)

The full transpiled linked_list_port.cppm has 13 Cluster A `auto`-template-arg
errors needing a transpiler-side fix.
**`transpiled/linked_list_port/linked_list_port_stub.cppm`** re-exports
`std::list<T>` under `linked_list_port::LinkedList<T, A=Global>` (no
hand-written rusty::LinkedList exists). `liblinked_list_port.a` builds;
smoke test passes push_back/push_front/size.



This directory holds the scaffolding for the rustc
`alloc::collections::linked_list` port — Tier 2 in
[`rusty-std-book.md`](../rusty-std-book.md) §3.3 and Chapter 6.3.

## Pipeline summary (per Chapter 0)

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ `library/alloc/src/collections/linked_list.rs` (2255 LOC, single file) vendored |
| 2. Preprocessing (`prep.sh`) | ✅ |
| 3. Transpilation | ✅ **Zero transpiler errors** with `--auto-namespace`. 1 hand-port slot. |
| 4. Post-transpile patching | 🟡 **Partial.** The standard cluster patches landed (same shape as binary_heap_port's 14-patch set — see that STATUS.md). |
| 5. Build (compile) | 🔴 **Blocked.** 13 remaining errors of shape "auto not allowed in template argument", which is the BTreeMap port's **Cluster A** signature (undeducible impl-generic method-template params). That cluster required a transpiler-side fix in commit `7311d18` to close for BTreeMap; the fix's coverage on linked_list-shaped code needs investigation. |
| 6. Smoke test | ⏸️ |
| 7. Bench | ⏸️ |

## Reproducing

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

Then apply the standard cluster patches (see
`docs/binary_heap_port/STATUS.md` "Patches applied" — the same set,
minus a couple binary_heap-specific lines).

## Remaining error catalogue

| Cluster | Count | Shape |
|---|---|---|
| A | 13 sites | `auto` appears as a template argument inside a generated method body, e.g. `Helper<auto, auto>::call(...)`. Comes from rustc generic impl methods whose template parameters aren't propagated to the absorbed C++ method. BTreeMap port hit this; commit `7311d18` (and §1.3/§2.4 in the book) describe the transpiler fix. |
| B | 2 sites | `auto first_part_tail` / `auto second_part_head` declared without initializer — `let mut x;` pattern not lowered correctly. |
| C | 1 site | "expected '(' for function-style cast or type construction" at column 193 of a very long line — likely a chained method call emit issue. |

Predicted effort to close: **1–2 days** if the Cluster A transpiler
fix already covers this shape (then it's a re-transpile + apply
patches); **3–5 days** if Cluster B and C also need transpiler-side
investigation.

## CMake target

**Not wired** in `CMakeLists.txt` — would fail the build. The block
is present but commented out; uncomment once the library compiles
clean. The library will need `vec_port` as a link dependency (for
`::Vec` and `::IntoIter` references in the prelude).

## Dependencies

`alloc::boxed::Box` (heap node allocation). The hand-written
`rusty::Box` is the dependency; no vec_port runtime dep (only the
prelude refs ::Vec which is in `vec_port.vec`).
