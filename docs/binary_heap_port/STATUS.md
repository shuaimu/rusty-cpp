# BinaryHeap port — ✅ Phase C: push() + pop() both work

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
| 6. Smoke test | ✅ **Seven test files, ~38 assertions, full public API covered.** module / push / pop / comprehensive (peek + drain + clear) / iter (iter + into_iter_sorted) / advanced (drain, drain_sorted, into_vec, from(Vec), into_sorted_vec, append, retain, with_capacity_in) / full-API (new_, default_, with_capacity, from(array), from_iter, from_raw_vec, peek_mut, pop_if, extend, extend_one, into_iter, reserve*, try_reserve*, shrink_to*, clone, clone_from, allocator). |
| 7. Bench | ✅ **Done.** `docs/binary_heap_port/binary_heap_port_bench.cpp` — transpiled vs `std::priority_queue<int>` across PUSH / POP / MIX at N=10,000 × 200 rounds. Transpiled is **35% faster on POP**, within 2-8% on PUSH and MIX (see book §6.1). |

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

| Cluster | Site | Status |
|---|---|---|
| C1 | `Hole::new_` (line 3731) | ✅ Patched — `rusty::ptr::read(data[pos])` → `rusty::ptr::read(&data[pos])`. |
| C2 | `Hole::~Hole` (line 3765) | ✅ Patched — second arg of `copy_nonoverlapping` was `data[pos]` (ref); patched to `&data[pos]`. |
| C3 | `Hole::element` (line 3738) | ✅ Patched — was `return this->elt;` (ManuallyDrop<T> → const T& conversion); patched to `return *this->elt;` (operator* on ManuallyDrop). |
| C4 | Sift down (line 4051) | ✅ Patched — `rusty::mem::swap(&item, &this->data[0])` → `rusty::mem::swap(item, this->data[0])` (refs, not pointers). |
| C5 | Sift down comparator (lines 4112/4113/4122/4143) | ✅ Patched — `rusty::get(hole, idx)` → `hole.get(idx)` (call Hole's own member surface rather than the free-function rusty::get which assumes a slice/Vec). |
| C6 | `rusty::len` over `Hole<int>` | ✅ Resolved as a side-effect of C5 — `hole.get(...)` no longer triggers `slice_full(hole)` instantiation. |

Predicted effort to close Phase C: **half to one day** — each cluster
is small and BTreeMap-port-shaped. Most are `rusty::ptr::*` helper
gaps that the BTreeMap port also hit (and which are already addressed
in btree_port's patcher).

## Advanced API impedances (surfaced 2026-06 — ✅ all four fixed)

A push to extend the test surface beyond push/pop/peek/iter into the
consume / bulk-build / mutation APIs surfaced four fresh instantiation
errors. All four landed inline in the vendored cppm + one runtime
helper added. `tests/binary_heap_port_advanced_test.cpp` now drives
all 8 advanced tests green.

| Tag | Site | Root cause + fix |
|---|---|---|
| D1 | `BinaryHeap::from(Vec)` body (line 4269) | Transpiler emitted `::Vec<T, A>{.data = …}` as the outer wrapper when it should be `BinaryHeap<T, A>{.data = …}` — the arg type leaked into the return-type slot. Sibling `from_raw_vec` at line 4039 is correct. Fix: patched the wrapper type inline. |
| D2 | Sift-down in `into_sorted_vec()` (line 4085) | Original Rust is `ptr::swap(ptr_a, ptr_b)` — swaps the *values* at the two pointers. Patcher item #13 (STATUS.md above) had rewritten `rusty::ptr::swap` → `std::swap`, conflating the two (std::swap swaps pointer *values themselves*) and also tripping on the rvalue 2nd arg. Fix: (a) added `rusty::ptr::swap(T*, T*)` to `include/rusty/ptr.hpp` with the right semantics, (b) reverted the call site to use `rusty::ptr::swap`. |
| D3 | Cross-module `Vec::from_iter` (vec.cppm:5305) | `SpecFromIter<T, Iter>::from_iter` was declaration-only with deduced `auto` return — any use site instantiating it past the declaration hit "function with deduced return type cannot be used before it is defined." Fix: (a) added an out-of-line generic fallback definition (push-loop over `iter.next()`) after Vec is fully visible, (b) moved Vec's forward declaration earlier so SpecFromIter could name `::Vec<T>` as its return type. |
| D4 | `RebuildOnDrop` ctor (line 4206) | The `heap` field was emitted as `::Vec<T, A>&` but the destructor body calls `heap.rebuild_tail(…)` — a BinaryHeap method. Confirms the field should be `BinaryHeap<T, A>&`. The transpiler likely recovered the type from the inner `BinaryHeap::data: Vec` field, missing that the Rust source's RebuildOnDrop holds `&mut BinaryHeap<T, A>`. Fix: patched both the field type and the ctor signature inline. |

A fifth latent bug (D5) surfaced *after* D4 landed: `Vec::retain_mut`
(vec.cppm:4942) emitted `rusty::ptr::drop_in_place(cur)` where `cur:
T&`. The sister site at line 4934 has `&cur` correctly. Patched
inline as `&cur`.

**Coverage**: `tests/binary_heap_port_advanced_test.cpp` now drives 8
APIs end-to-end: `with_capacity_in`, `drain()`, `into_vec`,
`from(Vec)`, `into_sorted_vec`, `drain_sorted`, `append`, `retain`.

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
