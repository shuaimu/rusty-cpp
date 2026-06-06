# LinkedList port — ✅ Phase B + Phase C peek complete

`liblinked_list_port.a` builds clean against the fully transpiled
`linked_list_port.cppm` (no `_stub.cppm` re-export). Patcher
codified in `post_transpile_patch.py`. Smoke test (test #37/38)
exercises:
- Phase B: `new_() / is_empty / len / push_back / push_front / pop_front` round-trip
- Phase C: `front() / back()` peek (does not consume; verifies head/tail visibility)

This directory holds the scaffolding for the rustc
`alloc::collections::linked_list` port — Tier 2 in
[`rusty-std-book.md`](../rusty-std-book.md) §3.3 and Chapter 6.3.

## Pipeline summary (per Chapter 0)

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ `library/alloc/src/collections/linked_list.rs` (2255 LOC, single file) vendored |
| 2. Preprocessing (`prep.sh`) | ✅ |
| 3. Transpilation | ✅ **Zero transpiler errors** with `--auto-namespace`. 1 hand-port slot. |
| 4. Post-transpile patching | ✅ Codified in `post_transpile_patch.py`. 9 patches (visit_byte_buf rebody + vec_port imports + Box `<X, const A&>` strip + `from_raw_in` rewrite + `&this->alloc` deref + Global value default-construct + Box arrow-access on `node_shadow1` + Node::into_element undeducible-template strip + node_shadow1 double-move fix in push_*_node). Idempotent. |
| 5. Build (compile) | ✅ **`liblinked_list_port.a` builds clean** (used to be blocked on 13 Cluster A + 2 Cluster B + 1 Cluster C errors; closed via transpiler fixes 8ed539b + 486c10c + 925aa58 + this patcher set). |
| 6. Smoke test | ✅ **Passing.** `tests/linked_list_port_module_test.cpp` exercises `new_() / is_empty / len / push_back / push_front / pop_front` round-trip via the Rust API. Runs under `ctest` (test #37/38, exit 0). |
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
| 7 | `--auto-namespace` wraps in `namespace linked_list_port`, but end users write `rusty::collections::LinkedList<T>` | Rename namespace wrap to `rusty::port::collections::linked_list` and append a `using LinkedList = ::rusty::port::collections::linked_list::LinkedList<T,A>;` alias in `rusty::collections` |

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

## Remaining for Phase D+

- `cursor_*`, `into_iter`, `iter_mut`, `extend`, `split_off`, etc.
  not exercised yet — same Option::map / NonNull deref pattern likely
  to trip on first instantiation.
- Bench against `std::list<int>` not run.

## Resolved bugs

- **front-after-pop state-corruption** (fixed). Root cause: the
  transpiler lowered Rust `match` arms as `auto&& _m = field; if
  (_m.is_some()) { auto&& _mv1 = _m.unwrap(); … }`. Hand-written
  `Option<T>::unwrap()` (non-const lvalue) is destructive — moves
  value out and sets `has_value = false` — so the match arm nuked
  the real field. Rust's match on a Copy-type Option (NonNull is
  Copy) copies the inner value into the arm binding rather than
  consuming the scrutinee.

  Fixed in the transpiler (commit `f91be68`): the
  `runtime_match_scrutinee_borrows_payload` predicate now also
  returns true for `match self.field` and `match local_ref.field`
  shapes (where Rust provably can't move out, so the scrutinee
  type is Copy). The materialized binding emit goes through
  `std::as_const(_m).unwrap()`, which calls the const overload
  (non-destructive, returns `const T&`).

  The earlier patcher rule (rewriting `_m.unwrap()` →
  `_m.as_ref().unwrap()` globally in linked_list_port.cppm) has
  been retired since the transpiler now produces the correct
  emit natively. 5 destructive `_m.unwrap()` sites remain in the
  cppm — all `match foo.take()` / `match foo.method()` with
  temporary scrutinees, where destructive consume is correct.

## Box helper

`include/rusty/box.hpp` gained `Box<T>::into_non_null_with_allocator(box)`
as a generic UFCS-style static helper. The transpiler emits this as
`Box<DT>::into_non_null_with_allocator(box)` where `DT = decltype(box)`
which can be `Box<U>` itself — so the static method has to be generic
over the argument type rather than relying on the enclosing Box's
template parameter. Other ports that use Rust's `Box::method(box)`
UFCS sugar can rely on this same helper.

## CMake target

**Wired** in `CMakeLists.txt` (since commit 0f8fac6, refined here).
Library links against `vec_port` (for `::Vec` and `::IntoIter`).

## Dependencies

- `alloc::boxed::Box` (heap node allocation) — provided by hand-written `rusty::Box`
- `vec_port.vec` (for `::Vec` type visibility)
- `vec_port.vec.into_iter` (for `::IntoIter` type visibility)
