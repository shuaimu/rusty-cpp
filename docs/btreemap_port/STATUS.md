# rustc-stdlib BTreeMap Port — Status

## Modules compile (step 19, expanded step 25)

After [post_transpile_patch.py](post_transpile_patch.py) lands, the
transpiled modules build cleanly to `libbtree_port.a`:

- **g++ -std=c++23** builds `btree_port.btree.btree_internal` (the
  6.4 KLoC merged internal module).
- **clang++ -std=c++23** builds `btree_port.btree.btree_internal`
  AND `btree_port.btree.map.entry` (GCC 14 hits an ICE during
  `RcControlBlockBase` destructor analysis when map.entry is
  included; clang accepts it cleanly).

The CMakeLists conditionally adds map.entry only under clang. Step
19 below documents the initial btree_internal-only build; the
present section captures the broader state after step 25.

`btree_internal.cppm` patch does two things:

1. **Stub 5 methods** that hit transpiler-side template-parameter
   recovery bugs (`from_new_leaf`, `from_new_internal`,
   `push_with_handle`, `deallocating_next`, `deallocating_next_back`).
   Each has its body replaced by
   `throw ::std::runtime_error("…stub…")`. The facade doesn't call
   them; the stubs make the templates parse, and the surrounding
   ~6.4 KLoC of valid-shape C++ stays compilable.

2. **Trim `CMakeLists.txt`** to only build `btree_internal`. The
   `set` / `map` / `*.entry` submodules hit additional, distinct
   transpiler bugs (post-module import ordering, cross-module
   template-arity recovery, orphan-impl misrouting) that are
   tracked separately. Keeping them out of the build target lets
   the proof-of-port (`libbtree_port.a`) link while those bugs
   are fixed independently.

Build also bumped from `CMAKE_CXX_STANDARD 20` to `23` in the
transpiler's CMakeLists generator (`transpiler/src/cmake.rs`) — the
panic!/println! macros lower to `std::println`, a C++23 facility.

State as of step 19:
- Hand-written facade (`include/btree_port/btreemap.hpp`): **10/10
  tests passing**, `g++ -std=c++23`. ✅
- Transpiled `btree_internal` module: **0 errors, 2 warnings**
  (`-Wglobal-module` typedef pragma in global module fragment;
  `-Wdeprecated-declarations` for `std::aligned_storage_t` in
  `rusty::function`). ✅
- Transpiled `map`/`set`/`*.entry`: out of build target, ~10 bug
  classes tracked in the "Open blockers — set/map/entry" section
  below. ⏳

## Open blockers — set/map/entry submodules (added step 19, updated step 21)

These are the next thing to chip at if/when the port resumes. Status is
where each stands after step 21:

- ✅ **Post-module imports not contiguous.** Resolved by
  `patch_entry_imports` in `post_transpile_patch.py` (step 20).
  Collects every `import …;` line in the file, dedups, and re-emits
  them as one contiguous block immediately after `export module`.

- ✅ **Cross-module `as`-rename loses template arity.** Resolved by
  `patch_entry_arities` in `post_transpile_patch.py` (step 20).
  `template<typename T, typename A> using MapOccupiedEntry = …<T, A>`
  rewritten to `template<typename K, typename V, typename A> using
  MapOccupiedEntry = …<K, V, A>`.

- ✅ **`map`/`btree_internal` namespace prefix not resolved.**
  Resolved by `strip_module_namespace_prefixes` in
  `post_transpile_patch.py` (step 20). Strips `<module>::`
  qualifier prefixes since C++20 modules don't put exported symbols
  in a namespace named after the module path.

- ✅ **Forward-decl/definition `requires` clause mismatch.**
  Resolved by `align_requires_clauses` in `post_transpile_patch.py`
  (step 21). Adds the matching `requires (Allocator<A> &&
  copyable<A>)` to algebraic-data-type `struct Entry` definitions
  that inherit from `std::variant<…>` without their forward
  decl's requires clause.

- ⏳ **Orphan-impl methods routed to wrong host.** Partially
  worked around in `post_transpile_patch.py` step 21:
  `remove_setvalzst_methods` wraps misrouted `template<typename T>`
  method clusters in `#if 0 / #endif`. Tightening the orphan-impl
  injector itself (the proper fix) is a separate transpiler-side
  change; the injector landed in commit `1ab0d4d` is matching too
  loosely — `impl VacantEntry<…>` in `map/entry.rs` should not
  absorb into `set::VacantEntry`.

- ⏳ **`NodeRef::new_leaf(…)` emitted without template args.**
  Worked around in step 21 via `stub_nodref_insert_entry` which
  replaces the body of `OccupiedEntry insert_entry(V value)` with
  a throw. The transpiler-side fix is to recover the template
  arguments from the call context (`NodeRef::new_leaf` always
  returns `NodeRef<marker::Mut, K, V, marker::Leaf>` per the
  Rust source).

- ⏳ **GCC 14 ICE on `map.entry` module.** Independent of our
  port: enabling `map.entry.cppm` in the build target triggers a
  GCC 14 internal compiler error (segfault) at the closing brace
  of `struct rusty::RcControlBlockBase`. Reproducer would be useful
  for the GCC bugzilla but it's outside the BTreeMap port scope.
  Workaround: keep `map.entry` out of the build target until GCC
  ICE is resolved, or test with clang's module support.

## Working version delivered (step 13, expanded through step 22)

`btree_port::BTreeMap<K, V>` and `btree_port::BTreeSet<T>` are
usable today via `include/btree_port/btreemap.hpp`. The facade is
a thin wrapper over `std::map`/`std::set` with the Rust-flavored
API:

**BTreeMap<K, V>:**

- Core: `new_()`, `insert` returning displaced `Option<V>`,
  `get`/`get_mut`/`contains_key`/`remove`/`len`/`is_empty`/`clear`/
  `clone`, plus STL-style begin/end for `for (auto& [k, v] : m)`.
- Position queries (step 14): `first_key_value`, `last_key_value`,
  `range`, `size`, `empty`.
- Mutation (step 22): `pop_first`, `pop_last`, `retain(f)`.
- Entry API (step 22): `entry(k).or_insert(v)`,
  `.or_insert_with(f)`, `.and_modify(f)` — the idiomatic Rust
  counter / upsert pattern, one statement, one lookup.
- View iterators (step 23): `keys()`, `values()`, `values_mut()` —
  iterate just the keys or just the values, in ascending order.
- Bulk operations (step 23): `extend(begin, end)`, `append(other)`,
  `split_off(key)`.

**BTreeSet<T>:**

- Core: `new_()`, `insert`, `contains`, `remove`, `len`, `is_empty`,
  `clear`, `clone`, begin/end, `size`/`empty` aliases.
- Mutation (step 24): `pop_first`, `pop_last`, `retain(f)`,
  `range(lower, upper)`.
- Set-theoretic ops (step 24): `union_set(other)`,
  `intersection(other)`, `difference(other)`,
  `symmetric_difference(other)`, `is_subset(other)`,
  `is_superset(other)`, `is_disjoint(other)`.

22-test smoke suite in `tests/btree_port_facade_test.cpp` covers
the full surface above (and the canonical word-count upsert via
`entry().or_insert(0) += 1`). All pass under `g++ -std=c++23`.

The intent is **a stable public API while the transpiled
internals are still being smoothed**. Each method body delegates
to `std::map` in one or two lines, so swapping in a transpiled
implementation as it becomes available is a localized change.

Transpiled internals state: ~6.4 KLoC of valid-shape C++ landed
under `cpp_out/btree_port.btree.btree_internal.cppm` after 12
commits of transpiler fixes and prep.sh hand-patches; ~20
compile errors remain, clustered in transpiler-side
template-parameter recovery (`Cmp`/`NodeType`/`BorrowType` not
declared) and closure-return-type inference under `Fn` trait
bounds. Each is solvable but the per-iteration yield has dropped
to single-digit error reductions, making this a multi-week effort
rather than a multi-session one. The architectural cycle (the
dominant blocker) has been resolved (step 8); what remains is
edge-case cleanup.



Live tracking of the BTreeMap port effort. Updated as compile blockers
surface and get resolved (either via transpiler fixes or hand-patches).

## Goal

Take `library/alloc/src/collections/btree/` from rustc's stdlib and
produce a working `btree_port::*` C++20 module suite that the build
can link against. The discipline is **mostly transpiled, hand-patched
where the transpiler can't easily fix**. Each hand-patch must be
re-applicable (encoded in `prep.sh` here).

## How to reproduce

```bash
# 1. Copy the stdlib btree subtree.
mkdir -p /tmp/btree_port/btree_crate/src/btree
cp -r ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/collections/btree/* \
   /tmp/btree_port/btree_crate/src/btree/

# 2. Apply port-prep hand-patches.
bash docs/btreemap_port/prep.sh /tmp/btree_port/btree_crate/src/btree

# 3. Set up the minimal Cargo crate skeleton (only ever read for
#    Cargo.toml + lib.rs paths; rustc is never invoked).
cat > /tmp/btree_port/btree_crate/Cargo.toml <<EOF
[package]
name = "btree_port"
version = "0.0.1"
edition = "2021"
[lib]
path = "src/lib.rs"
EOF
cat > /tmp/btree_port/btree_crate/src/lib.rs <<EOF
#![allow(unused)]
pub mod btree;
EOF

# 4. Transpile.
target/debug/rusty-cpp-transpiler --crate /tmp/btree_port/btree_crate/Cargo.toml \
   --output-dir /tmp/btree_port/cpp_out

# 5. Build.
cd /tmp/btree_port/cpp_out
cmake -B build -S . -G Ninja \
  -DCMAKE_CXX_FLAGS="-I<path-to-rusty-lib>/include -std=c++23" \
  -DCMAKE_CXX_STANDARD=23
cmake --build build
```

## Architectural limit — resolved (step 8)

Resolved via option 1 from the original analysis: the cyclic
btree submodules (node, search, navigate, merge_iter, fix,
remove, split, append, plus the supporting leaf files mem, borrow,
set_val, dedup_sorted_iter) are now concatenated into a single
`btree_internal.rs` by prep.sh before transpilation. The merged
file produces `btree_port.btree.btree_internal.cppm` — one TU
containing all interdependent types and impls, no cycle in the
module import graph.

The merged file's name is `btree_internal`, not `core`, because
`core` collides with Rust's stdlib `core::*` crate path and the
transpiler's path-mapping table misnormalizes it to `std::*`.

Before this step the build failed at the dyndep stage with
"CMake Error: Circular dependency detected in the C++ module
import graph"; after it the build proceeds to actual compilation
with ~55 ordinary errors (template instantiations, missing
declarations) which are individual blockers to chip away at
rather than an architectural wall.

## Architectural limit — earlier analysis (kept for reference)

This is the dominant remaining blocker and it isn't a transpiler bug —
it's a structural mismatch between Rust's module system and C++20
modules.

**The problem.** Rust's `library/alloc/src/collections/btree/` has
cyclic dependencies between sibling files:

- `node.rs` defines `NodeRef<…>`, `Handle<…>`, `LeafNode<…>`,
  `InternalNode<…>`.
- `navigate.rs` defines `LeafRange<…>`, `LazyLeafRange<…>`,
  `Position<…>` AND adds `impl<…> NodeRef<…> { … }` orphan-impls
  that return `LeafRange` values (so node.cppm absorbs those
  methods and now references `LeafRange`).
- `search.rs`, `merge_iter.rs`, `fix.rs`, `remove.rs`, `split.rs`,
  `append.rs` all add similar orphan-impls — each ends up
  referencing types declared in its own file (or in another
  sibling) from inside methods that the injector absorbs into
  node.cppm.

In Rust this resolves cleanly because module references are name-
lookup, not compilation units. In C++20 modules each `.cppm` is a
TU, and the import graph must be a DAG. Trying to add the obvious
`import navigate;` to node.cppm produces:

```
CMake Error: Circular dependency detected in the C++ module import
graph. See modules named: "btree_port", "btree_port.btree",
"btree_port.btree.append", "btree_port.btree.fix", … (12 modules)
```

**Resolution paths.** Three options, none cheap:

1. **Merge cyclic siblings into one `.cppm`.** Concatenate
   node.rs/navigate.rs/search.rs/merge_iter.rs/fix.rs/remove.rs/
   split.rs/append.rs into a single `btree_port.btree.core.cppm`.
   The transpiler would need a "merge group" config or a coarser
   one-module-per-cycle-component mode. Loses the Rust-side
   modularity in the C++ output. Most pragmatic.

2. **Drop C++20 modules; use header-only emission.** Rewrite the
   crate-mode driver to emit `.hpp`/`.cpp` pairs with forward
   declarations and traditional include guards. Largest change
   but matches how `include/rusty/*.hpp` already works.

3. **Restructure the source to break cycles.** Move all
   `impl` blocks into the file that declares their host type, so
   `node.rs` owns every `impl NodeRef<…>` regardless of which
   sibling adds methods. Requires editing the vendored stdlib
   heavily; defeats the "transpile what's there" approach.

The hand-patch we briefly tried in iter 7 (`use super::navigate::*;`
etc. added to node.rs via prep.sh) is what surfaced the cycle —
it's been removed pending an architectural decision.

## Resolved blockers

Listed by the commit that landed the fix.

| # | Commit | Symptom | Fix |
|---|---|---|---|
| 1 | `c0c4909` | `assert!(cond, msg)` and `assert!(match …)` mis-expand in C `assert()` | Parse condition as `syn::Expr` + throw-form for message tail |
| 2 | `e44dac6` | TODO/skipped markers invisible until grep | Post-hoc scanner + `rusty_hand_slots.md` manifest |
| 3 | `ccac4dd` | `using ::node::Root;` (broken cross-module) + `pub(super)` items not exported | Sibling-module import + treat all `pub*` as `export` in module mode |
| 4 | `99fc733` | `derive(Eq, PartialEq)` emits templated defaulted operator==; `super::super` imports unresolved; `std::alloc::{Allocator,Global,AllocError}` unmapped | Concrete operator== + dedup; ancestor walk; new alloc mappings |

## Open blockers

These are the next things to fix, in order of how the compile fails.
Each is annotated with whether it looks like a transpiler bug (generic
fix) or a hand-patch (specific to the BTreeMap shape).

### Tier 1 — clear transpiler bugs

- **`auto a_next; auto b_next;` — `auto` without initializer.**
  `merge_iter.cppm:3699–3700`. Comes from Rust `let mut a_next; let
  mut b_next;` (uninitialized declarations). C++ `auto` requires an
  initializer. Fix: track uninitialized let-bindings and either emit
  `decltype(…) a_next;` (with inferred type) or initialize from the
  first assignment.

- **`std::num::NonZero` not declared.** `node.cppm:3911`. Source has
  `use core::num::NonZero;`. Mapping needs `rusty::num::NonZero<T>`
  added. Like the `Layout`/`Allocator` table, this is a single entry
  in `rewrite_std_alloc_import` (or a parallel `rewrite_std_num_import`).

- **`mem::take_mut` not a member of `mem`.** `node.cppm:4124`. The
  port has its own `mem.rs` defining `take_mut` (we saw it transpiled
  successfully in step 1). The call site references `mem::take_mut`
  but the `mem` it imports is `rusty::mem`. Should be the local
  `btree::mem::take_mut`. Likely the cross-module fix needs a
  variant for **method-style calls** (vs imports).

- **`std::println` not declared.** `node.cppm:4202`. Should map to
  `std::print`/`std::println` from `<print>` (C++23) or
  `printf`/`fmt`. Search for the call site — it's probably a
  `dbg!()` or `println!()` macro.

- **`MIN_LEN` not declared.** `node.cppm:4265, 4326`. Refers to a
  `pub(super) const MIN_LEN: usize = …` in `btree.rs` (parent
  module). Cross-module const reference — analogous to
  cross-module type references but for `const`s.

### Tier 2 — likely hand-patches

- **`MergeIter`/`MergeIterInner` referenced before declared in
  `node.cppm`.** `node.cppm:4207`. `node.rs` uses
  `super::merge_iter::MergeIter<…>`, which my cross-module fix
  should turn into an `import btree_port.btree.merge_iter`. Need
  to verify the fix is firing; if it is, the symbol may not be
  reachable because `merge_iter.cppm` itself has compile errors
  (the `auto a_next` blocker above) and so doesn't export.

- **`node` used as namespace** (`node::*` calls in node.cppm:4217).
  This is a "use the submodule as a namespace" pattern that the
  cross-module fix doesn't handle. Would need either a
  `namespace node = btree_port::btree::node;` alias after the
  import, or a separate use-emission rewrite that preserves the
  namespace-prefix shape.

### Tier 3 — fundamental gaps (likely won't bridge)

- **`NodeRef<BorrowType, K, V, Type>` type-state machinery.** The
  `Immut<'a>` / `Mut<'a>` / `ValMut<'a>` markers collapse to the
  same C++ type because lifetimes don't exist at the type level.
  Documented in the original analysis as blocker #8 of the
  empirical report. Workaround: accept the loss of borrow-state
  distinction in the C++ result. Already incurred.

## Test-count waterline

Transpiler unit tests pass at every step:

| Step | Test count | Notes |
|---|---|---|
| Pre-port | 1521/1521 | Steady state at start of port |
| Step 1 (assert!) | 1524/1524 | +3 assert!-form tests |
| Step 2 (slots) | 1531/1531 | +7 slots-module tests |
| Step 3 (cross-module) | 1534/1534 | +3 sibling-module tests |
| Step 4 (op==/walk/alloc) | 1537/1537 | +3 derive-eq dedup + ancestor-sibling tests |
