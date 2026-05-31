# LinkedList port — ✅ Phase A2 complete (full transpiled body builds)

`liblinked_list_port.a` builds clean against the fully transpiled
`linked_list_port.cppm` (no `_stub.cppm` re-export). Patcher
codified in `post_transpile_patch.py`. Smoke test does not pass
yet — see "Remaining for Phase B" below.

This directory holds the scaffolding for the rustc
`alloc::collections::linked_list` port — Tier 2 in
[`rusty-std-book.md`](../rusty-std-book.md) §3.3 and Chapter 6.3.

## Pipeline summary (per Chapter 0)

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ `library/alloc/src/collections/linked_list.rs` (2255 LOC, single file) vendored |
| 2. Preprocessing (`prep.sh`) | ✅ |
| 3. Transpilation | ✅ **Zero transpiler errors** with `--auto-namespace`. 1 hand-port slot. |
| 4. Post-transpile patching | ✅ Codified in `post_transpile_patch.py`. 4 patches applied (visit_byte_buf rebody + vec_port imports + Box `<X, const A&>` strip + `from_raw_in` rewrite + Global value default-construct + Box arrow-access on `node_shadow1`). Idempotent. |
| 5. Build (compile) | ✅ **`liblinked_list_port.a` builds clean** (used to be blocked on 13 Cluster A + 2 Cluster B + 1 Cluster C errors; closed via transpiler fixes 8ed539b + 486c10c + 925aa58 + this patcher set). |
| 6. Smoke test | 🔴 **Pending API alignment.** The existing `linked_list_port_module_test.cpp` was written against the std::list-backed stub and calls `front()/back()/size()`. The transpiled LinkedList exposes the Rust API (`front_node()`, `len()`, etc.) and `back()` instantiation hits Option::map template substitution failures. New cluster to peel. |
| 7. Bench | ⏸️ |

## Patches applied (Phase A2)

Codified in `post_transpile_patch.py`. Idempotent.

| # | Issue | Patch |
|---|---|---|
| 1 | `visit_byte_buf(rusty::Vec<uint8_t>)` in the serde-de prelude lives in the GMF — `rusty::Vec` not visible | Rebody to `(auto&&) { return Err(E{}); }` (binary_heap_port shape) |
| 2 | `using IntoIter = ::IntoIter<T, A>;` inside LinkedList struct — global `::Vec` / `::IntoIter` need cross-module visibility | Inject `import vec_port.vec;` and `import vec_port.vec.into_iter;` after `export module linked_list_port;` |
| 3 | `rusty::Box<Node<T>, const A&>` two-arg form — hand-written Box is single-arg | Strip `, const A&` from the second template argument |
| 4 | `Box<T>::from_raw_in(ptr, &alloc)` not defined on hand-written Box | Rewrite to `Box<T>::from_raw(ptr)` (allocator dropped) |
| 5 | `rusty::alloc::Global` used as value (positional arg) instead of type | Rewrite to `rusty::alloc::Global{}` for `Global)` / `Global,` / `Global.` shapes |
| 6 | `node_shadow1.next` / `.prev` / `.element` — Rust auto-deref on Box, C++ requires `->` | Rewrite to `node_shadow1->next` etc. (narrow: only the 3 known Node fields on this specific binding) |

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

python3 docs/linked_list_port/post_transpile_patch.py /tmp/linked_list_port/cpp_out
cp /tmp/linked_list_port/cpp_out/linked_list_port.cppm transpiled/linked_list_port/
```

## Remaining for Phase B (smoke test green)

The library compiles in isolation, but C++ templates aren't
instantiated until used. The existing smoke test calls `back()` which
triggers Option::map instantiation; the lambda body hits
`node.as_ref().element` where `node` is of opaque type. Same shape as
binary_heap_port Phase A2/A3 cluster work — needs investigation.

Two paths to close Phase B:
- (a) Patch the transpiled body to thread types through (binary_heap_port took this route)
- (b) Update the smoke test to exercise the actually-instantiable Rust-shaped API surface (push_back / push_front / pop_front etc.)

Approach (b) is cheaper and gets us a green smoke test sooner; (a)
is the eventual fix but each cluster needs root-causing.

## CMake target

**Wired** in `CMakeLists.txt` (since commit 0f8fac6, refined here).
Library links against `vec_port` (for `::Vec` and `::IntoIter`).

## Dependencies

- `alloc::boxed::Box` (heap node allocation) — provided by hand-written `rusty::Box`
- `vec_port.vec` (for `::Vec` type visibility)
- `vec_port.vec.into_iter` (for `::IntoIter` type visibility)
