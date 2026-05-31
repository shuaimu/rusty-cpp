# BinaryHeap port — Phase B reached (library + empty-heap smoke test)

This directory holds the scaffolding for the rustc
`alloc::collections::binary_heap` port — Tier 2 in
[`rusty-std-book.md`](../rusty-std-book.md) §3.3 and Chapter 6.1.

## Pipeline summary (per Chapter 0)

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ `library/alloc/src/collections/binary_heap/mod.rs` (2038 LOC, single file) vendored |
| 2. Preprocessing (`prep.sh`) | ✅ Idempotent path rewrites |
| 3. Transpilation | ✅ **Zero transpiler errors** with `--auto-namespace`. 5 hand-port slots. |
| 4. Post-transpile patching | ✅ All six Phase A2 clusters fixed (see "Patches applied" below). Patches still inline in the vendored .cppm; a `post_transpile_patch.py` would codify these for re-transpile. |
| 5. Build (compile) | ✅ **`libbinary_heap_port.a` builds clean.** |
| 6. Smoke test | 🟡 **Partial.** `tests/binary_heap_port_module_test.cpp` proves `BinaryHeap<int32_t, Global>::new_in()` works + empty-heap invariants. push/pop/peek bodies hit instantiation-time issues — see "Remaining for Phase C". |
| 7. Bench | ⏸️ Blocked on full Phase C. |

## Patches applied (Phase A2)

These were all sed/perl one-liners on the vendored .cppm. They should
be codified into `post_transpile_patch.py` before any re-transpile:

| # | Issue | Patch |
|---|---|---|
| 1 | `rusty::Vec<…>` references no longer header-visible after VecLegacy retirement | `s/rusty::Vec</::Vec</g` |
| 2 | Dead `visit_byte_buf(rusty::Vec<uint8_t>)` prelude required complete Vec | rebody to `(auto&& value) { (void)value; return Err(E{}); }` |
| 3 | Duplicate `template<typename T> auto clone(...)` in prelude (collides with rusty::clone) | delete |
| 4 | `using rusty::Vec;` (rusty::Vec module-only now) | delete |
| 5 | binary_heap_port references `::Vec` and `::IntoIter` (vec_port globals) | add `import vec_port.vec; import vec_port.vec.into_iter;` |
| 6 | `vec::IntoIter<T,A>` / `vec::Drain<T,A>` (transpiler emitted `vec::` for vec_port — wrong) | `s/\bvec::IntoIter/::IntoIter/g`, same for Drain |
| 7 | `std::collections::TryReserveError` not in std | `s/std::collections::TryReserveError/rusty::collections::TryReserveError/g` |
| 8 | `rusty::Vec{}` empty default-ctor (missed by bulk Vec rename — no template args) | `s/rusty::Vec{}/::Vec<T>{}/g` |
| 9 | Bare `usize` identifier | `s/\busize\b/size_t/g` |
| 10 | But `usize::BITS` shouldn't become `size_t::BITS` | `s/size_t::BITS/std::numeric_limits<size_t>::digits/g` |
| 11 | `constexpr Option<NonZero<size_t>>` not literal type | drop `constexpr`, add `inline` for ODR |
| 12 | Bare `NonZero::new_(1)` needs template args | `s/= NonZero::new_(/= rusty::num::NonZero<size_t>::new_(/g` |
| 13 | `rusty::ptr::swap` / `ptr::swap` not implemented | `s/\b(rusty::)?ptr::swap\b/std::swap/g` |
| 14 | Orphan "Methods for Vec" impl block (transpiler emit bug — function body outside any class) | delete entirely |

## Remaining for Phase C (full push/pop/peek instantiation)

Errors visible at smoke-test instantiation time (template body not
evaluated until used with concrete types):

| Cluster | Site | Issue |
|---|---|---|
| C1 | `Hole::on_drop` (around line 3731) | `rusty::ptr::read` overload doesn't match the `const T*` passed |
| C2 | Hole-rebuild path (around line 3765) | `rusty::ptr::copy_nonoverlapping` doesn't accept the (T*, T*, size_t) signature the transpiled call uses |
| C3 | `Hole::pos`'s return type | the transpiler emitted `const ManuallyDrop<int>` for a field that should decay to `const int&` |
| C4 | Sift up/down (around line 4047) | `std::swap` not callable on `MaybeUninit<T>` slots; need `std::swap(*p, *q)` after manual deref |
| C5 | `peek()` (around line 4138) | `Option<const Hole<int>&>` operator overload (binary expr) — likely a `&&` on Options that the transpiler emitted as if `Option` had it (it doesn't) |
| C6 | `rusty::len` over `Hole<int>` | `rusty::len` SFINAE checks for `.len()` / `.size()` / `into_iter()` — `Hole` has neither; the call site is probably dead-code in our context |

Predicted effort to close Phase C: **half to one day** — each cluster
is small and BTreeMap-port-shaped. Most are `rusty::ptr::*` helper
gaps that the BTreeMap port also hit (and which are already addressed
in btree_port's patcher).

## Reproducing

```bash
RUSTSRC=$(ls -d ~/.rustup/toolchains/*/lib/rustlib/src/rust/library/alloc/src/collections/binary_heap/ | head -1)
mkdir -p /tmp/binary_heap_port/binary_heap_crate/src
cp $RUSTSRC/mod.rs /tmp/binary_heap_port/binary_heap_crate/src/lib.rs
cp docs/binary_heap_port/Cargo.toml.template /tmp/binary_heap_port/binary_heap_crate/Cargo.toml

bash docs/binary_heap_port/prep.sh /tmp/binary_heap_port/binary_heap_crate/src/lib.rs

./target/release/rusty-cpp-transpiler \
    --crate /tmp/binary_heap_port/binary_heap_crate/Cargo.toml \
    --output-dir /tmp/binary_heap_port/cpp_out \
    --auto-namespace

cp /tmp/binary_heap_port/cpp_out/binary_heap_port.cppm transpiled/binary_heap_port/
```

Then apply the 14 patches listed above. (TODO: codify in
`post_transpile_patch.py`.)

## Dependencies

- `vec_port.vec` (full Vec)
- `vec_port.vec.into_iter` (for BinaryHeap::into_iter return type)
