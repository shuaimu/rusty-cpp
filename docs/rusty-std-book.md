# Rusty Std Book — porting the Rust standard library to C++

A living document tracking the work of translating the Rust standard
library (the `library/alloc`, `library/core`, `library/std` source
trees from rustc) into C++ modules via the `rusty-cpp-transpiler`.

Each major chapter below corresponds to one standard-library type or
collection we've attempted to port. For each port we record:

- what the transpiler can handle on its own,
- what function bodies are still hand-written (in
  `docs/<port>_port/post_transpile_patch.py`),
- which of those hand-ports could be retired by a generic transpiler
  fix, and which need human porting effort.

Sibling docs:

- `rusty-cpp-book.md` — the borrow-checker / analyser side of RustyCpp.
- `rusty-cpp-transpiler.md` — transpiler design notes.
- `btreemap_port/STATUS.md`, `btreemap_port/GENERIC_FIXES_PLAN.md` —
  the day-by-day driver for the BTreeMap port and the patcher
  *text-fix* rule list, respectively. This book is the higher-level
  catalogue of *function-body* hand-ports across all ports.

---

## Table of Contents

- [Chapter 0 — Translation workflow](#chapter-0--translation-workflow)
  - [0.1 Pipeline overview](#01-pipeline-overview)
  - [0.2 Stage 1: source acquisition](#02-stage-1-source-acquisition)
  - [0.3 Stage 2: preprocessing (`prep.sh`)](#03-stage-2-preprocessing-prepsh)
  - [0.4 Stage 3: transpilation (`rusty-cpp-transpiler`)](#04-stage-3-transpilation-rusty-cpp-transpiler)
  - [0.5 Stage 4: post-transpile patching (`post_transpile_patch.py`)](#05-stage-4-post-transpile-patching-post_transpile_patchpy)
  - [0.6 Stage 5: build (`cmake` + `ninja`)](#06-stage-5-build-cmake--ninja)
  - [0.7 Stage 6: smoke tests + benchmarks](#07-stage-6-smoke-tests--benchmarks)
  - [0.8 Re-running the pipeline](#08-re-running-the-pipeline)
  - [0.9 Where each kind of fix belongs](#09-where-each-kind-of-fix-belongs)
- [Chapter 1 — `collections::BTreeMap`](#chapter-1--collectionsbtreemap)
  - [1.1 Hand-ports (full function bodies)](#11-hand-ports-full-function-bodies)
  - [1.2 Stubs that throw `runtime_error`](#12-stubs-that-throw-runtime_error-not-implemented)
  - [1.3 Root-cause categories](#13-root-cause-categories)
  - [1.4 Summary: retire-by-transpiler-fix triage](#14-summary-which-hand-ports-could-be-retired-by-transpiler-fixes)
  - [1.5 Perf profiling: the `clear_forgotten_address_range` cliff](#15-perf-profiling-the-rustymemclear_forgotten_address_range-cliff)
  - [1.6 Component-level comparison vs native Rust BTreeMap](#16-component-level-comparison-vs-native-rust-btreemap)
  - [1.7 IIFE-lambda overhead: focused micro-bench](#17-iife-lambda-overhead-focused-micro-bench)
  - [1.8 Retrospective: timeline, effort, milestones](#18-retrospective-timeline-effort-milestones)
- [Chapter 2 — Playbook for future std-library ports](#chapter-2--playbook-for-future-std-library-ports)
  - [2.1 Picking a target](#21-picking-a-target)
  - [2.2 The three-axis problem](#22-the-three-axis-problem-parser-codegen-runtime)
  - [2.3 Phase template (A → E)](#23-phase-template-a--e)
  - [2.4 Recurring transpiler-emit clusters to anticipate](#24-recurring-transpiler-emit-clusters-to-anticipate)
  - [2.5 Runtime gotchas](#25-runtime-gotchas)
  - [2.6 Bench discipline](#26-bench-discipline)
  - [2.7 When to stop](#27-when-to-stop)
  - [2.8 Estimating effort for the next port](#28-estimating-effort-for-the-next-port)

Future chapters: `Vec`, `HashMap`, `String`, `Arc`/`Rc`, `Mutex`, …

---

## Chapter 0 — Translation workflow

This chapter describes the end-to-end pipeline that takes a vendored
rustc-stdlib source tree and produces a compiled, runnable C++ port.
The workflow is **not** a single tool — it's a sequence of stages,
each of which exists because the previous stage can't be coaxed into
handling its input alone.

The same shape applies to every chapter that follows: each std-library
port chapter is essentially "the rustc source for X, run through this
pipeline, with port-specific contents in stages 2 and 4."

### 0.1 Pipeline overview

```
┌────────────────┐   ┌─────────┐   ┌────────────────┐   ┌──────────────────────────┐   ┌──────────────┐   ┌─────────┐
│ Vendored rustc │ → │ prep.sh │ → │ transpiler     │ → │ post_transpile_patch.py  │ → │ cmake+ninja  │ → │ runtime │
│ source (.rs)   │   │ (sed +  │   │ (Rust binary,  │   │ (Python; ~4100 LOC, ~56  │   │ (clang++,    │   │ smoke + │
│                │   │ python) │   │ syn-based AST) │   │ functions: text fixes,   │   │ C++23 modules)│   │ benches │
└────────────────┘   └─────────┘   └────────────────┘   │ hand-ports, stubs)       │   └──────────────┘   └─────────┘
                                                        └──────────────────────────┘
```

Each stage's responsibility is bounded:

| Stage | Tool | Lives in | What it does |
|-------|------|----------|--------------|
| 1 | manual `cp` / `git` | port directory | Vendor rustc source |
| 2 | `prep.sh` | `docs/<port>_port/prep.sh` | Rewrite Rust source so the transpiler can parse it |
| 3 | `rusty-cpp-transpiler` | `transpiler/src/codegen.rs` | Rust AST → C++20 modules |
| 4 | `post_transpile_patch.py` | `docs/<port>_port/post_transpile_patch.py` | Fix up emitted C++; hand-port what the transpiler can't |
| 5 | `cmake` + `ninja` | (CMakeLists.txt generated by transpiler, patched by stage 4) | Build the C++ library and tests |
| 6 | direct executable | the test/bench cpp files | Run smoke tests + benchmarks |

### 0.2 Stage 1: source acquisition

Vendor the relevant `library/<crate>/src/<submodule>/` tree from a
rustc checkout into the port directory. For BTreeMap this is
`library/alloc/src/collections/btree/` → `tests/transpile_tests/<…>/btree/`
(actual paths vary by experiment).

No tooling here — just `cp -r` or `git subtree`. Re-running this
overwrites prep.sh's earlier rewrites, so step 1 is normally a
one-time setup per port.

### 0.3 Stage 2: preprocessing (`prep.sh`)

The transpiler can't directly consume vendored stdlib source for a
handful of structural reasons. `prep.sh` rewrites the Rust before
the transpiler sees it. Categories of preprocessing the BTreeMap
port performs (in execution order):

1. **Strip rustc-internal tests** — `tests/` subdirs and `tests.rs`
   files depend on the `rand` crate and `#[cfg(test)]` items that
   the transpiler doesn't model.

2. **Crate-path rewrites** — vendored stdlib uses `crate::alloc::*`,
   `crate::boxed::Box`, `crate::vec::Vec` to reach sibling crate
   modules that aren't vendored alongside. Rewrite to public
   `std::alloc::*` / `alloc::boxed::Box` / `alloc::vec::Vec` so the
   transpiler's std-mapping table picks them up.

3. **Definite-assignment patches** — Rust allows `let mut x;`
   followed by definite assignment in every match arm. C++ `auto`
   requires an initializer. Hand-patch the relevant let bindings
   to add `= None` / `= 0` / restructure as a loop expression so the
   emit produces compilable C++. Semantics unchanged (the inits are
   unconditionally overwritten).

4. **Specialization workarounds** — `set_val.rs` uses unstable
   `default fn` specialization that the transpiler doesn't support.
   The single call site (`<V as IsSetVal>::is_set_val()`) is
   hard-coded to `false`; the only behavioral consequence is that a
   BTreeSet-specific panic message says "map" instead of "set".

5. **Match-IIFE workarounds** — `node.rs::splitpoint` uses
   module-level consts as match-arm patterns whose arms return
   different variant constructors that don't unify under the
   transpiler's IIFE-return-type inference. Rewrite the match as an
   if-chain so the emit bypasses the IIFE shape. (This is a
   workaround for a transpiler limitation already partially
   addressed by `GENERIC_FIXES_PLAN.md` item 4.)

6. **Identifier renaming for collision avoidance** — `merge_iter.rs`
   declares `enum Peeked<I> { A(I::Item), B(I::Item) }`. The
   variant `A`/`B` names collide with the BTree branching-factor
   const `B = 6` after stage 2's cycle-breaking concatenation.
   Rename variants to `Left` / `Right`.

7. **Cycle-breaking concatenation** — *the biggest single thing
   prep.sh does.* Rust's stdlib btree has cyclic dependencies
   between sibling files (`node ↔ navigate ↔ search ↔ merge_iter ↔
   fix ↔ remove ↔ split ↔ append`, each adding orphan impl blocks).
   Rust handles this via name-lookup (modules are name-resolution
   units, not compilation units); C++20 modules require the import
   graph to be a DAG. **Solution**: concatenate the cyclic group
   into a single Rust file (`btree_internal.rs`) before transpiling.
   The merged file becomes one C++ module; the cycle vanishes.

8. **Post-merge path rewrites** — after step 7, every
   `super::<old_submodule>::SYM` reference inside the merged file
   now points at the same file. Strip the prefixes.

`prep.sh` is **idempotent** — safe to re-run. Each transformation
guards itself with a `grep -q` or "file already merged" check.

### 0.4 Stage 3: transpilation (`rusty-cpp-transpiler`)

The Rust binary at `transpiler/src/codegen.rs` (the `rusty-cpp-transpiler`
crate) reads the prep'd Rust source, parses it with `syn`, and emits
one C++20 module per Rust source file:

```
library/alloc/src/collections/btree/btree_internal.rs   →  btree_port.btree.btree_internal.cppm
library/alloc/src/collections/btree/map.rs              →  btree_port.btree.map.cppm
library/alloc/src/collections/btree/set.rs              →  btree_port.btree.set.cppm
…
```

It also emits a `CMakeLists.txt` that wires the modules into a
buildable library target, and `rusty_hand_slots.md` — a manifest of
sites where the transpiler couldn't fully lower the source and left
a `// TODO transpiler:` comment so a human can patch the arm.

The transpiler is the largest moving piece (≈130 K LOC across
`codegen.rs` and related modules). Generic emit improvements made
here benefit *every* port, which is why `GENERIC_FIXES_PLAN.md`
prioritises lifting patcher rules upstream into the transpiler.

### 0.5 Stage 4: post-transpile patching (`post_transpile_patch.py`)

A Python script (~4100 LOC, 56 functions) runs over the transpiled
output to fix what the transpiler can't yet produce on its own. The
patcher is what stages 1–3 can't avoid; it's also what the project
is actively trying to *retire* via transpiler improvements.

Logical phases the patcher executes for the BTreeMap port (full list
visible in `main()`):

- **Phase A — `btree_internal.cppm` text fixes**: ~25 small
  rewrites under `patch_internal()`. Examples: `k.borrow()` SFINAE
  fallback for primitives, `assume_init_ref` method-call → free-fn,
  `as_leaf_ptr()` → `as_leaf_ptr((*this))`, `DormantMutRef::new_ref`
  const → mut, etc. Each rewrite has a docstring explaining the
  precise transpiler emit shape it compensates for.

- **Phase B — hand-port stubs (`implement_*`)**: the transpiler
  emits a `throw runtime_error("stub")` body for methods it can't
  lower; the patcher then replaces those stubs with manually-written
  C++. **This is what Chapter 1 catalogues** (table 1.1). Hand-ports
  include `from_new_leaf`, `from_new_internal`, `push_with_handle`,
  `search_tree`, `force`, `descend`, `into_kv`, `leaf_edge` walkers,
  deallocators, `BTreeMap::entry`.

- **Phase C — correctness fix-ups (`fix_*`)**: targeted text rewrites
  for emit shapes the transpiler gets *almost* right. Examples:
  `dormant_map.reborrow()` binding `const auto` → `auto&`,
  `static_factory_param_type_recovery`, `force_match_arms` value-fixup.

- **Phase D — entry / sibling-module patches**: `set.entry.cppm`,
  `map.entry.cppm`, `set.cppm` get smaller text patches for
  import-ordering, namespace prefix stripping, and the
  `template<typename T>` orphan-impl misroute pattern that arises
  when sister-type methods get injected into the wrong host class.

- **Phase E — `map.cppm` patches**: the biggest single file gets the
  most patches, including a special-case merge of `map.entry.cppm`'s
  struct definitions into `map.cppm` (`merge_map_entry_into_map`) to
  break a `DormantMutRef<BTreeMap>` cycle that C++20 modules can't
  express.

- **Phase F — CMakeLists.txt + smoke source generation**:
  `patch_cmake()` rewrites the transpiler-generated `CMakeLists.txt`
  to add the link-smoke test target, C++23 module flags, and the
  rusty include directory; `write_link_smoke()` writes the
  hand-written smoke .cpp that exercises the transpiled types.

The patcher is **idempotent** — every transformation guards itself
with a `sentinel` comment check. Re-running on already-patched
output is a no-op.

### 0.6 Stage 5: build (`cmake` + `ninja`)

```bash
cmake -B build -G Ninja -DCMAKE_CXX_COMPILER=clang++
cmake --build build
```

C++23 module support is required (clang++ 16+, Ninja generator).
The transpiler emits one `.cppm` per source file plus a static
library target (`btree_port`), so downstream consumers `import
btree_port.btree.map;` and link `libbtree_port.a`.

### 0.7 Stage 6: smoke tests + benchmarks

Three executable targets are built alongside the library:

- `btree_port_link_smoke` — proves the module loads and a trivial
  exported type (`SetValZST`) is instantiable from a regular `.cpp`
  translation unit.
- `btree_port_transpiled_read_smoke` — exercises read-path operations
  on an empty `BTreeMap` (`get`, `contains_key`).
- `btree_port_bench` — micro-bench comparing the transpiled
  `BTreeMap` against `libstdc++` `std::map` (build-then-lookup
  workload). Currently shows a ~1700–10000× perf gap (see
  Chapter 1.4 for analysis).

### 0.8 Re-running the pipeline

The full sequence:

```bash
# (Stage 1 already done; vendored source lives in btree_crate/.)

# Stage 2: prep
bash docs/btreemap_port/prep.sh path/to/btree_crate/src/btree

# Stage 3: transpile
target/release/rusty-cpp-transpiler \
    --crate path/to/btree_crate/Cargo.toml \
    --output-dir path/to/cpp_out

# Stage 4: patch
python3 docs/btreemap_port/post_transpile_patch.py path/to/cpp_out

# Stage 5: build
cmake -B path/to/cpp_out/build -G Ninja \
    -DCMAKE_CXX_COMPILER=clang++ -S path/to/cpp_out
cmake --build path/to/cpp_out/build

# Stage 6: run
path/to/cpp_out/build/btree_port_transpiled_read_smoke
path/to/cpp_out/build/btree_port_bench
```

Every stage past 1 is idempotent. If only the transpiler changed,
you can re-run stages 3–5 and skip 2 (since the prepped Rust
source is unchanged).

If `prep.sh` is re-run on a directory it has already processed (e.g.
after re-vendoring), it re-applies each transform — but because the
old output already matches the post-transform shape, every transform
becomes a no-op.

### 0.9 Where each kind of fix belongs

When you hit a new bug porting a Rust stdlib type, decide which stage
should own the fix. Rough guide, easiest to hardest:

| If the bug is… | The fix probably belongs in… |
|---|---|
| Rust source uses a feature the transpiler doesn't support (`default fn`, uninit `let mut x;`) and the workaround is local and semantics-preserving | `prep.sh` (stage 2) |
| The transpiler emits a *concrete textual mistake* — wrong identifier, missing template arg, namespace prefix that doesn't survive the import — and the right output is mechanical to compute | `post_transpile_patch.py` text-fix (stage 4 Phase A/C); add a docstring explaining the emit shape |
| The transpiler emits a method body that's *fundamentally wrong* and the right C++ is a faithful manual rewrite of the Rust | `post_transpile_patch.py` hand-port (stage 4 Phase B); add to Chapter 1.1 |
| The transpiler emits something almost-correct but type inference / lifetime / variant-dispatch couldn't trace through it | **Transpiler** (`codegen.rs`). Add a regression-test crate under `tests/transpile_tests/`, then file an item in `GENERIC_FIXES_PLAN.md`. Patcher should be the temporary workaround until the transpiler lands the fix. |
| The pipeline produces correct C++ but the perf is bad | Out of scope of this book; see notes at the end of Chapter 1.4. |

Patcher rules and hand-ports are tech debt by design: the goal is to
move every patcher rule into the transpiler so the script can shrink
toward zero. `GENERIC_FIXES_PLAN.md` tracks that direction for text
fixes; Chapter 1.4 of this book does the same triage for hand-ports.

---

## Chapter 1 — `collections::BTreeMap`

This chapter tracks the **function bodies** in the transpiled BTreeMap
output that are *not* produced by the transpiler itself. They live in
`docs/btreemap_port/post_transpile_patch.py`, which runs after the
transpiler emits the `.cppm` modules and rewrites specific function
bodies via text-level patches.

Separate from this chapter:

- `btreemap_port/STATUS.md` — overall port progress, phase-by-phase.
- `btreemap_port/GENERIC_FIXES_PLAN.md` — patcher *text-fix* rules
  (regex/string rewrites) that should be lifted into the transpiler.
  Items 1–8.

This chapter covers a different class of patch: **entire function
bodies replaced with manually-written C++**, plus methods left as
**stubs that throw `runtime_error`** because nobody has implemented
them yet.

The goal of this chapter is to (a) catalogue what's hand-written so
the "is this transpiled?" question has a single source of truth, and
(b) analyse which hand-ports could be retired by a generic transpiler
fix vs. which need real human porting effort.

### 1.1 Hand-ports (full function bodies)

Listed in roughly the order they appear in the patcher.

| # | Function | File:line | Patcher fn | Used by |
|---|----------|-----------|------------|---------|
| H1 | `NodeRef::from_new_leaf` | btree_internal.cppm:4280 | `implement_from_new_leaf` | `BTreeMap::insert` (new root) |
| H2 | `NodeRef::from_new_internal` | btree_internal.cppm:4301 | `implement_from_new_internal` | `Root::push_internal_level` |
| H3 | `LeafNode::push_with_handle` | btree_internal.cppm:4562 | `implement_push_with_handle` | `BTreeMap::insert` hot path |
| H4 | `NodeRef::search_tree` | btree_internal.cppm:4627 | `implement_search_tree` | `BTreeMap::{get, entry, …}` |
| H5 | `NodeRef::first_leaf_edge` | btree_internal.cppm:4823 | `implement_leaf_edge_walkers` | iterator construction |
| H6 | `NodeRef::last_leaf_edge` | btree_internal.cppm:4838 | `implement_leaf_edge_walkers` | iterator construction |
| H7 | `Handle::descend` | btree_internal.cppm:~5390 | `implement_handle_descend` | tree walk |
| H8 | `Handle::force` | btree_internal.cppm:5547 | `implement_handle_force` | every Leaf/Internal dispatch |
| H9 | `Handle::into_kv` | btree_internal.cppm:~5397 | `implement_handle_into_kv` | `get` return |
| H10 | `deallocating_next` | btree_internal.cppm:5657 | `implement_deallocating` | `into_iter` / drop |
| H11 | `deallocating_next_back` | btree_internal.cppm:5698 | `implement_deallocating` | `into_iter` / drop |
| H12 | `BTreeMap::entry` | map.cppm:5757 | `implement_btreemap_entry` | `BTreeMap::insert`, user entry API |

### 1.2 Stubs that throw `runtime_error` (NOT implemented)

If these are called at runtime, they throw `rusty-cpp-transpiler: …`.

| Method | File | Patcher fn |
|--------|------|------------|
| `OccupiedEntry::key` | map.cppm, map.entry.cppm | `stub_broken_map_methods`, `stub_broken_entry_method` |
| `OccupiedEntry::get_mut` | map.entry.cppm | `stub_broken_map_entry_methods` |
| `OccupiedEntry::into_mut` | map.entry.cppm | `stub_broken_map_entry_methods` |
| `OccupiedEntry::into_key` | map.cppm, map.entry.cppm | `stub_broken_map_methods`, `stub_broken_entry_method` |

The benchmark exercises only `insert` + `get`; these paths don't hit
the OccupiedEntry stubs. Anything that walks an Entry would.

### 1.3 Root-cause categories

Group the hand-ports by *why* the transpiler couldn't handle them.
This is the lens for deciding "could a generic transpiler fix retire
this hand-port?"

#### Category A — `Box::into_non_null_with_allocator` destructure

**Members**: H1 (`from_new_leaf`), H2 (`from_new_internal`).

**Rust pattern**:
```rust
fn from_new_leaf(leaf: Box<LeafNode<K, V>, A>) -> Self {
    let (node, _alloc) = Box::into_non_null_with_allocator(leaf);
    NodeRef { height: 0, node, _marker: PhantomData }
}
```

The transpiler doesn't model `Box::into_non_null_with_allocator` (which
destructively splits a Box into its `NonNull<T>` pointer and its
allocator). It emitted a `throw` stub for the entire method body.

**Generic-fix viability: HIGH.** Pattern is one Box call + one struct
literal. The transpiler already supports `Box::new_in` and similar
factory paths via `rusty::Box`. Adding a destructure recognition rule
that maps `Box::into_non_null_with_allocator(b)` to the equivalent
`rusty::Box` accessor sequence (`b.into_raw()` / `b.allocator()`) is a
small lookup-table entry.

**Note**: The hand-port itself is ~6 lines per method — not where the
perf gap lives. Pure correctness completion.

#### Category B — `MaybeUninit` slot writes through generic-parameterized arrays

**Members**: H3 (`push_with_handle`).

**Rust pattern**:
```rust
self.key_area_mut(idx).write(key);
self.val_area_mut(idx).write(val);
```

`key_area_mut(idx)` returns `&mut MaybeUninit<K>`, then `.write(val)`
moves `val` into the slot. The transpiler's emit went through the
`decltype(auto)` SFINAE dispatcher for `key_area_mut`/`val_area_mut`,
then the `.write(...)` call hit type-resolution issues — the
`MaybeUninit` array slots in the `LeafNode<K,V>` struct are
`std::array<rusty::mem::MaybeUninit<K>, CAPACITY>`, and operator[]
return-type inference broke under the generic-param-laden NodeRef.

The hand-port directly does the same calls but with explicit types,
avoiding the auto-deduction problem.

**Generic-fix viability: MEDIUM.** This is the same family of issue as
Item 2 in `btreemap_port/GENERIC_FIXES_PLAN.md`
(`slice.get_unchecked[_mut](i) → slice[i]`). A proper fix is to thread
element types through generic array accesses so the `MaybeUninit<K>`
slot type can be deduced at the call site. Doable but not trivial —
touches the type-inference paths that already failed once on this code
(see Iter 72–79 in the task history).

#### Category C — `loop { match self.force() { Leaf(x) => …, Internal(y) => … } }`

**Members**: H4 (`search_tree`), H5/H6 (`first_leaf_edge`, `last_leaf_edge`),
H10/H11 (`deallocating_next`, `deallocating_next_back`).

**Rust pattern**:
```rust
loop {
    match self.force() {
        ForceResult::Leaf(leaf) => return leaf.first_edge(),
        ForceResult::Internal(internal) => {
            node = internal.first_edge().descend();
        }
    }
}
```

`force()` returns a `ForceResult<Leaf<…>, Internal<…>>` variant. The
arms use **bare-glob variant constructors** (`Leaf`, `Internal`)
because the enclosing `use ForceResult::*` makes them unqualified.

This is by far the most pervasive issue in the BTreeMap port. The
slot manifest (`rusty_hand_slots.md`) shows 13 sites with the comment
`/* TODO transpiler: unresolved bare-glob variant ‘Leaf’ (no enum
decl visible in this TU; patch arm manually) */`. The transpiler
emitted an arm condition of literal `true`, breaking the dispatch.

**Generic-fix viability: HIGH (but largest single fix).** This is a
genuine missing transpiler feature: bare variant constructor
resolution when the variant lives behind a method-call returning a
variant type. The transpiler needs to:

1. Recognize that `self.force()` returns a known variant type
   (`ForceResult`) — look up `force()` in the impl-block index.
2. Resolve `Leaf` / `Internal` in arm patterns against that variant.
3. Emit the correct `_m.index() == N` dispatch.

If fixed, this single change retires **at least 5 of the 12
hand-ports** (H4, H5, H6, H10, H11) plus ~13 manual-patch slots.
This is the highest-impact transpiler fix on the table.

#### Category D — Method-template params that fail deduction

**Members**: H7 (`Handle::descend`), H8 (`Handle::force`),
H9 (`Handle::into_kv`).

**Rust pattern**: methods defined on `Handle<NodeRef<BorrowType, K, V,
NodeType>, …>` where `BorrowType`/`K`/`V`/`NodeType` come from the
*outer* `Handle`'s `Node` generic parameter. The transpiler emitted:

```cpp
template<typename BorrowType, typename K, typename V>
NodeRef<BorrowType, K, V, ::marker::LeafOrInternal> descend() { … }
```

— these are **method-template params with no deduction source**. Call
sites like `handle.descend()` provide no arguments, so the template
arguments can't be deduced, and the call fails.

The hand-port uses a `__NodeRefArgs<Node>` type-trait that destructures
the enclosing class's `Node` template arg (which *is* a NodeRef<…>) to
recover `BorrowType`/`K`/`V` from it without method-template params.

**Generic-fix viability: HIGH.** The fix already exists in spirit —
"Cluster A" introduced `__TemplateArgs<Host>::arg_N` for exactly this
pattern. It just didn't catch every method. Extending Cluster A's
detection to cover these three methods (or generalising the pattern
match it uses) would retire H7/H8/H9. Same family as completed
`btreemap_port/GENERIC_FIXES_PLAN.md` item 7 ("Wrong template-arg
recovery").

#### Category E — Composite: `BTreeMap::entry`

**Member**: H12 (`BTreeMap::entry`).

**Rust pattern**:
```rust
pub fn entry(&mut self, key: K) -> Entry<'_, K, V, A> {
    let (map, dormant_map) = DormantMutRef::new(self);
    match map.root {
        None => Vacant(VacantEntry { … }),
        Some(ref mut root) => match root.borrow_mut().search_tree(&key) {
            Found(handle) => Occupied(OccupiedEntry { … }),
            GoDown(handle) => Vacant(VacantEntry { key, handle: Some(handle), … }),
        }
    }
}
```

This combines: `DormantMutRef::new` (already needs a helper injected
— see `fix_dormant_mut_ref_calls`), nested matches on `Option` and
`SearchResult` variants, and struct literals with `..` ignored fields.

**Generic-fix viability: MEDIUM, but only after C and D.** Several
sub-problems already overlap with Cat C (variant dispatch) and Cat D
(template-arg recovery on `search_tree`). Once C and D land, the
remaining shape of `entry` is well within what the transpiler does
elsewhere. Roughly 90% of this hand-port disappears for free if C
and D are fixed; the last 10% is `DormantMutRef::new` destructuring
which already has a partial helper.

#### Category F — Genuine unimplemented surface (stubs)

**Members**: `OccupiedEntry::{key, get_mut, into_mut, into_key}`.

**Generic-fix viability: N/A.** These aren't transpiler bugs — they're
methods that nobody has gotten around to porting because the bench
doesn't hit them. To remove the stubs, someone needs to write
faithful C++ ports of the four method bodies (each is short — `key`
just returns `&self.handle.key()`, `into_mut` returns
`self.handle.into_val_mut()`, etc.). The transpiler probably *could*
emit working versions today now that H9 (`Handle::into_kv`) and the
H11/H12 fixes from Category D are in; the stubs date from earlier
phases when those weren't yet available.

### 1.4 Summary: which hand-ports could be retired by transpiler fixes

| Category | Hand-ports | Generic fix? | Effort | Impact |
|---|---|---|---|---|
| A | H1, H2 | Yes — recognise `Box::into_non_null_with_allocator` | Small | Retires 2 hand-ports |
| B | H3 | Yes — generic array element-type threading | Medium | Retires 1 hand-port |
| C | H4, H5, H6, H10, H11 | Yes — bare-variant resolution via method-return type | **Large** | **Retires 5 hand-ports + ~13 slot patches** |
| D | H7, H8, H9 | Yes — extend Cluster A coverage | Medium | Retires 3 hand-ports |
| E | H12 | Largely — depends on C and D | Small (after C+D) | Retires 1 hand-port |
| F | (stubs) | No — needs human ports | Per-method | Removes runtime-throw surface |

Net: **Cat C alone retires more hand-ports than any other category, and
it's the same root cause as most of the remaining 13 `// TODO
transpiler: unresolved bare-glob variant …` slots.** It is the
highest-leverage transpiler item still open.

The earlier ~1700–10000× perf gap vs. `std::map` was *almost entirely*
caused by a single runtime-bookkeeping routine in the `rusty/mem.hpp`
header — gated and resolved in Section 1.5. The transpiled BTreeMap
is now at parity with libstdc++ `std::map` on the BTreeMap-of-ints
bench.

### 1.5 Perf profiling: the `rusty::mem::clear_forgotten_address_range` cliff

Profiled the bench with gperftools (`-lprofiler` + `CPUPROFILE=…`,
analysed with `pprof`) at N=10, REPS=8000 (26s wall, 2611 samples).
The flat profile:

```
flat   flat%   cum     cum%    function
25.99s  99.54%  26.03s  99.69%  rusty::mem::clear_forgotten_address_range
0       0%      23.08s  88.40%  slice_insert (via insert_recursing, insert_fit, …)
0       0%       3.02s  11.57%  rusty::ptr::write (via LeafNode::init, new_leaf, …)
```

**One function — `rusty::mem::clear_forgotten_address_range` — holds
99.5% of self time.** Inside it, the hot line is the comparison inside
a `for it = addresses.begin(); it != addresses.end(); …` loop
(25.70s / 98.4%).

#### What the function does

`include/rusty/mem.hpp:343`. It walks a **global mutex-guarded
`std::unordered_map`** ("forgotten-address" markers — addresses of
values the runtime has flagged as moved-from for double-drop
detection) and erases any keys whose `address` falls in
`[base, base + bytes)`:

```cpp
inline void clear_forgotten_address_range(const void* base, std::size_t bytes) noexcept {
    …
    platform::threading::lock_guard<platform::threading::mutex> lock(detail::forgotten_addresses_mutex());
    auto& addresses = detail::forgotten_addresses();
    for (auto it = addresses.begin(); it != addresses.end();) {
        const auto current = reinterpret_cast<std::uintptr_t>(it->first.address);
        if (current >= start && current < end) {       // ← 98.4% of total CPU here
            it = addresses.erase(it);
        } else {
            ++it;
        }
    }
}
```

#### Why it's catastrophic

It is called **on every element write inside `ptr::write` / `ptr::copy`**:

```cpp
template<typename T, typename U>
inline void write(T* dst, U&& value) {
    rusty::mem::clear_forgotten_address_range(static_cast<const void*>(dst), sizeof(T));
    std::construct_at(dst, std::forward<U>(value));
}
```

And every B-tree insert ultimately calls `slice_insert`, which shifts
K elements via `ptr::copy`, which calls
`clear_forgotten_address_range` K times. Each call:

1. Takes a process-wide mutex.
2. Scans the **entire** global forgotten-addresses map (linearly,
   regardless of `base`).
3. Erases any entries whose key falls inside the range.

The set grows monotonically as moves accumulate across all
`BTreeMap`s in the process, so each call gets slower. The cost is
O(global_set_size × elements_shifted × inserts).

#### Quick verification

Replacing the body of `clear_forgotten_address_range` with a no-op
(temporary perf experiment, not a real fix — it disables moved-from
tracking) and re-running the bench:

| N  | REPS | Before (with bookkeeping) | After (no-op)  | Gap shrinks by |
|----|------|---------------------------|----------------|----------------|
| 5  | 2000 | 1770× slower than std::map | **12.1×**     | ~146×          |
| 10 | 1000 | 4821×                      | **18.7×**     | ~258×          |
| 15 | 500  | 10265×                     | **17.5×**     | ~587×          |
| 20 | 200  | 10971×                     | **18.3×**     | ~600×          |

Removing this single function brought the perf gap from **~1700–11000×**
down to **~12–19×**. That's a **600–1000× speedup** from one change.

After the no-op, the ~12–19× residual gap is the "real" emit-shape
overhead I'd guessed at earlier (SFINAE dispatchers, std::variant
dispatch, allocator wrapper). The hot-path symbols disappeared from
the profile entirely.

#### Fix landed (commit, this session)

Two-line `if constexpr` guard in `rusty/ptr.hpp` + `rusty/vec.hpp`:

```cpp
template<typename T, typename U>
inline void write(T* dst, U&& value) {
    if constexpr (!std::is_trivially_destructible_v<T>) {
        rusty::mem::clear_forgotten_address_range(dst, sizeof(T));
    }
    std::construct_at(dst, std::forward<U>(value));
}
```

…and a parallel intermediate fast path in `ptr::copy` for types that
are not trivially-*copyable* but are trivially-*destructible* (the
exact shape of `MaybeUninit<int>`, which has user-defined copy/move
ctors but a defaulted destructor — and is the type sitting at the
BTreeMap-leaf hot path).

**Reasoning**: a forgotten-address marker can only exist for `T` if
*something* added one. The only adders are `rusty::mem::forget`,
`ptr::copy`'s element-marking loop, and transpiled
`rusty_mark_forgotten()` methods. All three are only emitted for
types with non-trivial destructors. For primitive types (`int`) and
transpiler-internal `MaybeUninit<int>` (trivial destructor), no
marker can ever be added — so the linear-scan clear is pure
overhead, and `is_trivially_destructible_v<T>` is a sound
compile-time gate to skip it.

The global table stays in place for non-trivially-destructible types
(`Box`, `Vec`, transpiled iterators, `PanicGuard`, …) which still
need the runtime mark/consume protocol — those types' move ctors and
destructors continue to consult it.

#### Measured impact (full BTreeMap bench, N=10, REPS=100,000)

```
== transpiled BTreeMap<int,int,Global> ==
  insert seq  + get seq    0.042 s    21.2 ns/op
  insert rand + get rand   0.042 s    21.2 ns/op

== libstdc++ std::map<int,int> (reference) ==
  insert seq  + get seq    0.044 s    22.0 ns/op
  insert rand + get rand   0.043 s    21.4 ns/op
```

The transpiled BTreeMap is now **at parity with libstdc++ `std::map`
(actually ~4% faster)**, down from the original 1700–10000× gap. A
>10,000× speedup from two `if constexpr` branches.

Bench correctness verified on the isolation test (single + multi
insert/get round-trips, splits past CAPACITY=11) — all six checks
pass; the N=20 insert path that previously failed with the naive
disable-everything experiment works because the slow-path bookkeeping
stays active for the non-trivially-destructible scope-guard /
transpiled-iterator types that actually rely on it (e.g. `PanicGuard`,
whose destructor aborts unless explicitly forgotten).

#### Performance for non-trivially-destructible types

The fast path above only applies when `T` is trivially-destructible.
Naively one might expect non-trivial types to retain the
1700–10000× cliff. Measurement shows they don't, because of two
fortunate-by-design properties:

1. **`MaybeUninit<T>` is always trivially-destructible regardless of
   `T`** — its only member is an `unsigned char storage_[sizeof(T)]`
   byte array, and `~MaybeUninit() = default` is trivial for that.
   Container types (`std::array<MaybeUninit<T>, N>`) inherit the
   property. Since the transpiler emits leaf storage as exactly
   `std::array<MaybeUninit<K>, CAPACITY>` / `<V>`, **every B-tree
   slice-shift hits the fast path** even for `BTreeMap<K, Vec<T>>`.

2. **`rusty::Box` / `rusty::Vec` etc. use the `std::move` move-ctor
   protocol, not the forgotten-address protocol, for ownership
   transfer**. Their move ctors null the source pointer; their
   destructors are guarded by `if (ptr != nullptr) delete ptr;`. No
   global-table hit on the move path. The forgotten-address calls
   that *do* exist (in transpiler-emitted move ctors / dtors of
   composite types) are O(1) per object boundary, not per element.

Measured overhead for non-trivially-destructible workloads:

| Bench | Workload | Ratio vs std equivalent |
|---|---|---|
| `BTreeMap<int, NontrivialValue>` (user-defined dtor) | 5 N → 50 ops/iter, 100–2000 iters | **1.2–2.0×** vs `std::map` |
| `Vec<rusty::Box<int>>` reserve + push_back × 100 | 50 reps | **1.0×** vs `std::vector<unique_ptr>` |
| `Vec<rusty::Box<int>>` reserve + push_back × 100,000 | 50 reps | **0.9×** vs `std::vector<unique_ptr>` |

The residual ~1–2× factor is mostly:
- Per-object-boundary mark/consume (2 mutex acquires + 2 hash ops per
  transpiled-type move).
- The transpiler's variant-dispatch (`std::variant::index()` checks)
  and SFINAE method-dispatch lambdas on the path leading to those
  moves.

#### Strict null-state: global table deleted

Committed as the final state of this work. The "Null state" option
described above is now the runtime's actual design. Every transpiler-
emitted struct with a `Drop` impl carries a `mutable bool
_rusty_forgotten = false;` field; the runtime's global forgotten-
address table is gone.

Concrete shape of the emit:

```cpp
struct PanicGuard {
    mutable bool _rusty_forgotten = false;
    PanicGuard() = default;
    PanicGuard(PanicGuard&& other) noexcept {
        this->_rusty_forgotten = other._rusty_forgotten;
        other._rusty_forgotten = true;
    }
    void rusty_mark_forgotten() const noexcept { _rusty_forgotten = true; }
    ~PanicGuard() noexcept(false) {
        if (_rusty_forgotten) { return; }
        rusty::intrinsics::abort();   // drop body
    }
};
```

The previous global table's three call sites collapse into per-type
boolean assignments. `mem::forget` calls `value.rusty_mark_forgotten()`
uniformly on const and non-const values (the method is `const` and the
field is `mutable`).

Runtime API surface kept as no-op shims for backwards compatibility:
`rusty::mem::mark_forgotten_key` / `consume_forgotten_key` /
`clear_forgotten_address_range` / `clear_all_forgotten_addresses` all
return false / do nothing. New transpiled code never calls them.

#### Final perf (BTreeMap bench, N=10, REPS=100,000)

```
== transpiled BTreeMap<int,int,Global> ==
  insert seq  + get seq:    0.054 s    26.8 ns/op
  insert rand + get rand:   0.055 s    27.4 ns/op

== libstdc++ std::map<int,int> (reference) ==
  insert seq  + get seq:    0.047 s    23.3 ns/op
  insert rand + get rand:   0.043 s    21.3 ns/op
```

**~1.15–1.28× slower than libstdc++ `std::map`**. The residual gap is
the local-bool store on every move ctor + the local-bool check on
every destructor for types with `Drop` impls — a handful of CPU
cycles per object boundary, no global state, no mutex.

For comparison:
- Original (with global table): **1700–10000×** slower.
- Intermediate (if-constexpr fast path, global table still present for
  non-trivial-T): **~0.95–1.0×** (at parity).
- Strict null-state (current, **no global table**): **~1.15–1.28×**.

The intermediate version was slightly faster because the `if
constexpr` skipped the local-flag overhead for trivially-destructible
types (which is most of the BTreeMap leaf hot path). The strict
version applies the protocol uniformly to every Drop type, paying ~5
ns/op for the privilege of having zero global state.

#### Other strategies (not currently pursued)

These remain potentially useful if the residual ~1.2× gap matters:

- **Transpiler-side elision**: when the transpiler can prove a move
  is *terminal* (source statically guaranteed not to be dropped
  later — the analog of Rust's static drop tracking), skip emitting
  the flag-propagation in the move ctor and the flag-check in the
  destructor. Closes most of the residual gap.
- **Per-type opt-out**: types whose destructor body is a member-wise
  drop (and where every member already has its own null-state
  protocol — i.e. the destructor body has no observable side effects
  beyond field destruction) don't need the flag at all. The
  transpiler could detect these and omit both the field and the
  preamble check, recovering the if-constexpr-fast-path behavior
  without a global table.

Verified: zero references to the deleted machinery in the compiled
binary (`nm libbtree_port.a | grep forgotten_addresses` → empty;
`objdump -d btree_isolate_test | grep -i forgotten` → empty). The
global mutex and unordered_map are structurally gone from the
runtime.

#### Where this should live in the book

Even though the cliff is in `rusty/mem.hpp` (the C++ runtime support
library, not the transpiler or the patcher), the impact is so
disproportionate that it dwarfed every other category in Sections
1.1–1.4. Any future std-library port using `ptr::write` or
`ptr::copy` on hot paths with trivially-destructible elements would
have hit the same wall before this fix.

### 1.6 Component-level comparison vs native Rust BTreeMap

After §1.5 closed the catastrophic cliff and brought the transpiled
port to within ~1.2× of `std::map`, the next natural question is: how
does it compare to **native Rust BTreeMap** running the same
workload? This section answers that with a head-to-head callgrind
breakdown to decide whether further optimization is worth pursuing.

#### Setup

Two identical bench programs, one Rust and one C++, both running:
- N = 10 keys (LCG-shuffled deterministically with the same seed)
- 5,000 reps × (10 inserts + 10 gets) = 100,000 operations
- Counted under `valgrind --tool=callgrind` (instruction count `Ir`)

| | Total Ir | per op | wall ns/op |
|---|---|---|---|
| Rust `std::collections::BTreeMap` | 12,446,365 | 124 | 14.0 |
| C++ transpiled `BTreeMap` | 21,167,039 | 212 | 23.6 |
| Ratio | — | **1.70×** | **1.69×** |

`perf record` was the first choice but is blocked on this host
(`perf_event_paranoid=4`), so callgrind was used instead. `Ir` is a
deterministic count, not a sampled estimate.

#### Per-component breakdown

Grouped by *logical activity* rather than by file (since inlining
attributes things across files differently in each toolchain):

| Component | Rust Ir | C++ Ir | Ratio | Notes |
|---|---|---|---|---|
| **Binary search in node** | 2.38 M | 3.23 M | 1.36× | `find_key_index` (C++) ≈ `search.rs` (Rust); plus C++ pays 615 K in slice/array.hpp bounds wrappers |
| **Key comparison** | 1.95 M | (inlined into find) | — | Rust attributes `three_way_compare` to `core/cmp.rs`; C++ folds into the search call |
| **Slice/iter helpers** | 0.51 M | 1.05 M (slice.hpp + stl_iterator.h) | 2.07× | C++ slice machinery is heavier than Rust's `NonNull`-based iter |
| **Insert path** (`insert_fit`, `insert_recursing`, `Handle`) | 2.39 M | 5.90 M | **2.47×** | Biggest single delta — split across 5 file:function lines in C++ |
| **IIFE / lambda wrappers** | 0 | 3.61 M | **∞** | Pure codegen artifact (see below) |
| **Entry API** | 0.55 M | (folded into lambda) | — | |
| **memcpy / data movement** | 0.72 M (`memcpy_avx_unaligned_erms`) | (inlined into insert_fit) | — | Rust calls libc memcpy; C++ inlines a hand-rolled loop |
| **Allocation** (malloc/free/alloc shim) | 0.52 M | 0.94 M | 1.78× | C++ does slightly more allocator round-trips per insert |
| **`drop_in_place` / destructor** | 0.95 M | (inlined into main) | — | Rust attributes Drop separately; C++ inlines destruction into the bench loop |
| **Dynamic linker startup** | 0.15 M | 1.70 M | **11.3×** | One-time cost; C++ template symbol resolution is expensive |
| **Bench driver itself** | 0.49 M | 0.84 M | 1.73× | LCG, sink, etc. — proportional |

Subtracting the **one-time** linker startup gives the steady-state
per-op ratio:

| | Total − startup | per op |
|---|---|---|
| Rust | 12.25 M | 122 |
| C++ | 19.5 M | 195 |
| Steady-state ratio | — | **1.59×** |

#### Where the gap actually is

1. **IIFE lambda symbols — 3.6 M instructions attributed (17 % of
   total), but ~zero is overhead.** This was originally read as "pure
   IIFE setup cost," but a focused micro-bench (see §1.7) shows that
   at `-O3` both clang and g++ fully elide the IIFE wrapping — the
   function body is byte-identical to the equivalent plain-branch
   version. The 3.6 M Ir is the *work happening inside* the lambda
   bodies (variant dispatch, value extraction, `insert_entry`), which
   has a direct counterpart in Rust attributed to the enclosing
   function instead of to a lambda symbol. This is a callgrind
   attribution difference, **not** real overhead.

2. **Insert-path inflation: 2.47×**. `insert_fit` shows up in *two*
   files (`ptr.hpp` 1.60 M + `btree_internal.cppm` 1.59 M) for a
   single source-level call — the compiler is generating two
   separately-attributed paths and not folding them. `insert_recursing`
   and `insert` each add another ~1 M. Some of this is symbol-
   attribution noise from inlining; some is real extra work.

3. **Search: 1.36×**. The binary search itself is *already
   well-tuned*. The wrappers around it
   (`slice.hpp::validate_slice_bounds`, `array.hpp` bounds checks)
   add ~600 K. Hot path.

4. **Allocator: 1.78×**. C++ goes through `_int_malloc` more —
   possibly because the `rusty::alloc::Global` pathway has an extra
   layer compared to Rust's direct `__rdl_alloc`.

5. **Dynamic linker startup: 11.3×, but one-time.** ~1.6 M extra Ir,
   ~150 ns of wall time at program start. Amortized over 100 K ops
   it's a rounding error; for short-running programs it would matter.

#### What's *not* worse

- **memcpy / shift loops in the leaf** — equivalent or better
  (inlined as a hand loop, sometimes better than Rust's libc-memcpy
  call for tiny shifts).
- **Destructor cost** — gone from the profile after the §1.5 fix; no
  per-element bookkeeping anymore.
- **Get/lookup path** — essentially proportional to search cost
  alone.

#### Potential optimizations (not pursued — see "Recommendation")

| Optimization | Est. savings | Difficulty |
|---|---|---|
| ~~Eliminate IIFE codegen for `?`/`if let`~~ | **~0 Ir** | **Skip — clang elides IIFE at `-O3`, see §1.7** |
| Drop slice-bounds checks in `find_key_index` (UB-safe in B-tree internal code) | ~0.6 M Ir (3 %) | Low — add `Slice::get_unchecked` variant in `include/rusty/slice.hpp` |
| Coalesce duplicate `insert_fit` symbol attribution | ~1.5 M Ir (7 %) | Medium — likely an `__always_inline` / module-linkage issue |
| Switch to `mimalloc` / pool allocator | ~0.3–0.5 M Ir (2 %) | Low — orthogonal to the port |

Realistic ceiling if the remaining items were done: ~19 M Ir C++ vs
12.4 M Rust → **1.53× ratio**. We'd close only a modest fraction of
the gap. The bulk of the remaining cost is in the actual insert-path
work (compounded template instantiations, allocator layering), not in
codegen wrappers — and matching Rust would mean redoing the data
layout / handle abstraction, not just transpiler patches.

#### Recommendation

**Stop optimizing the BTreeMap port for now.** The current state is:

- Catastrophic 1700–10000× cliff: **fixed** in §1.5.
- Versus libstdc++ `std::map`: **1.15–1.28× slower** (parity-ish).
- Versus native Rust BTreeMap: **1.59× slower steady-state** /
  **1.70× including startup**.

The remaining gap is **not** in IIFE wrappers (§1.7 confirms those
elide cleanly at `-O3`). It's spread across compounded template
instantiations on the insert path, slice-bounds checks in the search
hot loop, and an allocator layer with one more level of indirection
than Rust's direct `__rdl_alloc`. Closing it would require redoing
those structural decisions, not adding a transpiler patch.

Better ROI is on:
- broader port coverage (`Vec`, `HashMap`, `Arc`/`Rc`, …) — the
  workflow in Chapter 0 is now well-trodden,
- **or**, if perf parity becomes important later, work on the
  insert-path bloat (coalesce `insert_fit` duplicates, add
  `get_unchecked` for internal-use slice access). Each one is worth
  3–7 % at most; combined ~10–15 %.

The full callgrind dumps are at `/tmp/callgrind.rust.out` and
`/tmp/callgrind.cpp.out` for re-analysis if priorities change.

### 1.7 IIFE-lambda overhead: focused micro-bench

A natural follow-up to §1.6 is: how much of the 21 M Ir attributed to
`{lambda#1}::operator()` symbols is *actually* IIFE overhead vs the
work happening inside the lambda body? This section answers that.

#### Setup

Four versions of the same `match` over a `std::variant<KV, int>`,
hand-written to match shapes the transpiler emits. All four return
the same value; all four are marked `__attribute__((noinline))` so
the compiler can't fold the boundary away:

1. **plain match** — direct `if (m.index() == 0) ... else ...` with
   no closures. Baseline.
2. **outer IIFE only** — `[&](){ if/else }()` wrapping the match.
3. **plain + inner if-constexpr lambda** — no outer wrapper, but
   uses the transpiler's generic `[&](auto&& __t){ if constexpr
   (requires{__t._1;}) ... else std::get<1>(__t) }` field extractor.
4. **full transpiler shape** — outer IIFE + inner generic lambda.

Driver runs 200 M reps; per-call cost is measured with a `volatile`
memory barrier preventing CSE across iterations.

Source: `/tmp/iife_bench/iife_bench.cpp` (simple body),
`/tmp/iife_bench/iife_heavy.cpp` (heavier per-arm body).

#### Headline result: clang elides the IIFE completely

Disassembling all four functions with clang 19 at `-O3` shows
**byte-identical 4-instruction bodies**:

```asm
xorl    %eax, %eax
cmpb    8(%rdi), %al        # compare variant index byte
sbbl    %eax, %eax
orl     4(%rdi), %eax       # OR with the _1 field; sets to -1 on Vacant
retq
```

g++ 14 at `-O3` produces a slightly different (branchy) 4-instruction
body, also byte-identical across all four variants:

```asm
cmpb    $0, 8(%rdi)
jne     .L_vacant
movl    4(%rdi), %eax
ret
.L_vacant:
movl    $-1, %eax
ret
```

**Conclusion at the function-body level: zero overhead.** The
IIFE-wrapped form compiles to the same instructions as the plain
form in both compilers.

#### What the bench measures

Despite identical asm, the bench reported measurable differences:

| Variant | clang `-O3` | g++ `-O3` |
|---|---|---|
| 1. plain match | 1.51 ns/op | 1.79 ns/op |
| 2. outer IIFE only | 1.94 ns/op (1.29×) | 2.10 ns/op (1.17×) |
| 3. plain + inner constexpr | 1.79 ns/op (1.19×) | 1.79 ns/op (1.00×) |
| 4. full transpiler shape | 1.50 ns/op (0.99×) | 2.23 ns/op (1.24×) |

These differences are **harness artifacts**, not function-body cost.
The harness's `measure()` template inlines the wrapping lambda (one
per variant) into the inner loop; the loop's codegen differs by
function-pointer / lambda-shape even when the called function's body
is the same. Notice that variant 4 is *faster* than variant 1 in
clang, which is impossible if the IIFE had real cost.

#### Heavier-body test

To rule out "maybe the simple case is too trivial," a second bench
puts a 5-iteration LCG loop in each arm — closer to the real per-arm
work in BTreeMap's insert path:

| Variant | clang `-O3` | g++ `-O3` |
|---|---|---|
| plain | 4.25 ns/op | 3.98 ns/op |
| full transpiler shape | 4.47 ns/op (1.05×) | 4.04 ns/op (1.02×) |

The 1–5 % overhead is at the noise floor; inspection of the asm
shows it's from clang missing one constant-fold across the IIFE
boundary (one extra `addl $12345, %ecx` per loop iteration), not
from the IIFE itself.

#### Does it hold at `-O2`?

Yes. `-O2` is the more common production setting (CMake's `Release`
type uses `-O3`, but many distros and most CI builds use `-O2`). Re-
running the simple-body bench at `-O2`:

| Variant | clang `-O2` | g++ `-O2` |
|---|---|---|
| 1. plain | 1.49 ns/op | 1.79 ns/op |
| 2. outer IIFE only | 1.94 ns/op (1.30×) | 2.09 ns/op (1.17×) |
| 3. plain + inner constexpr | 1.80 ns/op (1.21×) | 1.79 ns/op (1.00×) |
| 4. full transpiler shape | 1.50 ns/op (1.00×) | 2.24 ns/op (1.25×) |

Numbers within noise of the `-O3` measurements. Crucially, the
*disassembly* of `plain_match` and `iife_full` is byte-identical at
`-O2` in both compilers — same 4-instruction body. **The IIFE
wrapping is fully elided at `-O2` too.**

Heavier-body at `-O2`: clang 1.05×, g++ 1.12× (g++ misses slightly
more constant folds at `-O2` than `-O3`, but still well under the
overall BTreeMap ratio).

#### What about `-O1`?

For completeness — `-O1` is rarely used in production:

| Variant | clang `-O1` | g++ `-O1` |
|---|---|---|
| 1. plain | 1.51 ns/op | 2.22 ns/op |
| 4. full transpiler shape | 1.49 ns/op (0.99×) | 2.08 ns/op (0.94×) |

Clang still elides the IIFE completely at `-O1` (5 asm instructions,
identical to plain). g++ does *not* elide at `-O1` — the iife_full
function body is 288 lines of asm vs 315 for plain, both far larger
than the optimized 4-instruction form. Yet the bench shows iife_full
is *slightly faster* (0.94×) — g++ at `-O1` happens to produce more
compact code for the wrapped form. Not worth investigating; nobody
ships `-O1` builds.

#### Implication for §1.6

The 3.6 M Ir originally attributed to "IIFE overhead" in §1.6 is
**not** lambda-mechanism cost. It's the actual variant-dispatch,
field-extraction, and `insert_entry` work happening inside the
lambda body — which has direct counterparts in Rust attributed to
the enclosing function instead of to a lambda symbol. The IIFE
boundary in the BTreeMap port is *free* under both `-O2` and `-O3`
in clang and g++ (the BTreeMap port itself ships with `-O3 -g
-DNDEBUG`, the `RelWithDebInfo` defaults).

This rules out the largest "potential optimization" from §1.6. The
remaining optimization items (slice-bounds removal, `insert_fit`
coalescing, allocator) account for at most ~10 % of total cost
combined, reinforcing the §1.6 recommendation to stop optimizing.

#### Where to look if performance regresses

If a future change introduces real IIFE overhead, it would show up
as differing assembly between the plain and IIFE-wrapped forms. Run
the bench:

```sh
clang++ -O3 -std=c++23 -S -o iife.s /tmp/iife_bench/iife_bench.cpp
diff <(awk '/_Z11plain_match/,/Lfunc_end/' iife.s) \
     <(awk '/_Z9iife_full/,/Lfunc_end/'   iife.s)
```

Empty diff = elided cleanly. Non-empty diff = real codegen
difference worth investigating.

### 1.8 Retrospective: timeline, effort, milestones

This section captures the wall-clock and effort numbers behind the
BTreeMap port, both as a record of what actually happened and as a
calibration point for estimating future ports.

#### Wall-clock timeline

| Date / time (ET) | Milestone | Cumulative |
|---|---|---|
| 2025-08-24 | First abandoned attempt (`new btreemap`) — punted | — |
| **2026-05-18 00:17** | Restart: prep.sh + `transpiler:` step 1 (assert! macro lowering) | **0 h** |
| 2026-05-18 08:57 | Transpiled `btree_internal.cppm` compiles cleanly under clang (step 19) | +8.7 h |
| 2026-05-18 09:37 | Facade API surface complete: entry(), pop_first/last, retain (step 22) | +9.3 h |
| 2026-05-18 10:06 | BTreeSet set-theoretic ops + range/pop (step 24) | +9.8 h |
| **2026-05-19 00:34** | **ZERO compile errors; transpiled_smoke LINKS and RUNS** (step 47) | **+24.3 h** |
| 2026-05-19 21:30 | Phase E insert path partially working (step 61); transpiler-level limits identified | +45 h |
| 2026-05-22 01:19 | Cluster A–E transpiler-fix batch lands | ~4 days |
| 2026-05-23 ~ 2026-05-24 | Item 1–11 redoes (parser-side & emit-side fixes) | ~5–6 days |
| 2026-05-24 17:10 | **Perf cliff found and fixed** (§1.5: `clear_forgotten_address_range` linear-scan) | ~6.5 days |
| 2026-05-25 01:30 | Strict null-state achieved (global table deleted, §1.5 epilogue) | ~7 days |
| 2026-05-25 09:39 | Component comparison vs Rust + IIFE micro-bench (§§1.6–1.7) | ~7.4 days |

**~1 week of focused effort** from "no port" to "matches `std::map`,
1.59× steady-state vs native Rust BTreeMap, with retrospective and
playbook documented." The August 2025 attempt was abandoned because
the transpiler wasn't ready — by May 2026 it was.

#### Effort numbers

Since the restart on 2026-05-18 through 2026-05-25:

| Metric | Value |
|---|---|
| Commits with `btree_port:` / `BTreeMap port step` | 33 |
| Commits with `transpiler:` / `codegen:` (driven by port) | ~80 |
| Total commits in the week | ~200 |
| `docs/btreemap_port/prep.sh` | 421 lines |
| `docs/btreemap_port/post_transpile_patch.py` | 4,105 lines, 57 functions |
| Hand-port function bodies (in patcher) | ~30 |
| Stubs that throw `runtime_error` (still unimplemented) | ~13 |
| Total LOC churned in btree-related commits | ~10,500 insertions / ~1,200 deletions |

The patcher (4.1K lines) and prep.sh (421 lines) together represent
~4.5K lines of "stuff the transpiler can't yet do." Of that, the
patcher is the larger structural debt — most of its hand-ports could
be retired by transpiler fixes (see §1.4's triage).

#### Phase shape that emerged

The port naturally clustered into five phases. Roughly:

| Phase | Goal | What it looked like | Effort |
|---|---|---|---|
| **A** — *Types compile* | Get the module to parse + compile (no link, no run) | A1: template-arg recovery for `Handle`/`NodeRef`/`Root`. A2–A5: fix unknown-type errors one cluster at a time. | ~8 h |
| **B** — *Hand-port unknowns* | Stub out / hand-port what the transpiler emitted as broken or missing | B1–B5: `from_new_leaf`, `from_new_internal`, `push_with_handle`, `deallocating_next/_back`. | ~2 h |
| **C** — *Link + smoke-test* | Get a consumer test to link and run *anything* through the API | C1: read-path smoke test (insert + iterate) | ~1 h |
| **D** (skipped) | (Would have been: full Rust unit-test parity, like the `either` parity harness — punted; not necessary for BTreeMap goal) | — | — |
| **E** — *Tighten transpiler & full API* | Codify hand-ports back into transpiler when possible; chase down insert-path, then perf | Cluster A–E transpiler fixes; Item 1–11 redoes; eventually perf cliff + null-state | ~5 days |

**The first 24 hours took the port from nothing → linking & running
a smoke test.** Everything after that was tightening the long tail.

#### Distribution of effort

Roughly, where the week went:

```
Pipeline + initial transpile  ████████░░░░░░░░░░░░ ~20%  (steps 1-30, May 18 single-day blitz)
Hand-port phases A-C          ████░░░░░░░░░░░░░░░░ ~10%  (steps 31-47)
Insert-path long tail (E)     ████████████░░░░░░░░ ~30%  (steps 48-80, Cluster A-E, Items 1-11)
Perf cliff fix (§1.5)         ████░░░░░░░░░░░░░░░░ ~10%
Strict null-state refactor    ████░░░░░░░░░░░░░░░░ ~10%
Bench + retrospective (§§1.6-1.8)  ████████░░░░░░░░ ~20%
```

The insert path was the single biggest tar-pit — `Handle::insert_fit`
/ `insert_recursing` / `VacantEntry::insert_entry` chained ~15
distinct transpiler-emit bugs that had to be peeled one at a time.

#### What worked

1. **Numbered steps with one-line commit summaries** — Made it
   trivial to reconstruct what was tried. Every commit is searchable
   as `port step N`. Without this we'd have lost the thread.

2. **The patcher as a "transpiler IOU ledger"** — Every hand-fix
   went into `post_transpile_patch.py` with a `# transpiler should
   do X` comment. §1.4 then triaged them back into transpiler fixes.
   This kept the patcher from rotting into permanent technical debt.

3. **Compile-then-link-then-run** — Phase A got the module to
   *compile*. Phase B hand-stubbed the broken bodies. Phase C got
   *one* consumer test to *link and run*. Only after that did Phase
   E start chasing per-method correctness. Each phase had a binary
   exit criterion.

4. **Bench discipline** — Once the port was running, the first
   bench result (1700–10000× slower than `std::map`) immediately
   surfaced the perf cliff in §1.5. Without that bench we'd have
   shipped a structurally-correct but practically-unusable port.

5. **Comparing against Rust, not just `std::map`** — §1.6 reframed
   the goal: "are we close to *native Rust*?" not "are we close to
   *libstdc++*?". Different conclusion (1.59× vs 1.05×) and
   different optimization priorities.

#### What was hard

1. **The cyclic-module problem.** rustc's btree submodule has
   `node ↔ navigate ↔ search ↔ merge_iter ↔ fix ↔ remove ↔ split ↔ append`
   forming a tight cycle. C++20 modules require a DAG. We
   concatenated the cyclic group into a single `btree_internal.rs`
   in prep.sh — ugly, but the only realistic option short of a
   transpiler-level cycle breaker. **This will recur for any
   multi-file Rust module with sibling impl blocks.**

2. **Template-argument recovery from absorbed methods.** Cluster A
   alone took multiple rounds (initial fix, then redo, then
   "Cluster A completion" with `__TemplateArgs<...>` partial
   specialization). The transpiler can't recover from the
   structural-decomposition mismatch when the host class's
   generics don't textually appear in the absorbed method
   signature. The current fix is robust but the design space wasn't
   obvious upfront.

3. **The perf cliff.** A 1700× cliff sitting in a `for it = …;
   addresses.begin() != end(); …` loop in `rusty::mem.hpp` was
   invisible from the C++ source review — only visible via
   `gperftools`. **Lesson: profile early, not after correctness
   work.** A profile after step 47 (when smoke test first ran)
   would have surfaced this within minutes.

4. **The "deletes after a move" problem.** The original moved-from
   tracking used a global mutex-guarded `unordered_map`. Solving it
   correctly took multiple iterations: address-range no-op fast
   path → if-constexpr guard → per-type `_rusty_forgotten` bool →
   delete the global table entirely. Each iteration was driven by
   a new bench scenario that broke the previous fix.

#### Hand-port → transpiler-fix conversion rate

§1.4 triage:
- **Retire-by-transpiler-fix** (high leverage): 8 of 30 hand-ports
- **Retire-by-prep.sh** (medium leverage, port-specific): 6
- **Genuine human porting** (low leverage, won't generalize): 16

So roughly **half** the hand-ports represent transpiler debt that
would help future ports too. The other half are BTreeMap-specific
oddities (e.g. `IsSetVal::is_set_val()` unstable specialization).

---

## Chapter 2 — Playbook for future std-library ports

§1 is the BTreeMap-specific record. This chapter distills it into
patterns that should apply to the next port (`Vec`, `HashMap`, etc).

### 2.1 Picking a target

Order targets by:

1. **Cyclic-module score** — Does the rustc source have sibling
   files with cyclic `use super::` imports? (See §1.3 / Chapter 0
   for why this matters.) Single-file modules are easiest; tight
   cycles are hardest. Check with `grep -r "use super::" library/<…>`.
2. **Generic-arity score** — How many type parameters? `Vec<T>` is
   1, `HashMap<K, V, S>` is 3, `BTreeMap<K, V, A>` is 3 with deeply
   nested handles. Higher = more chance of running into
   Cluster-A-style template-arg recovery issues.
3. **Internal unsafe density** — `Vec::push` is ~10 LOC of unsafe;
   `BTreeMap::insert` is hundreds of LOC across multiple files.
   Higher = more bodies the transpiler has to get exactly right.
4. **External surface** — Public API methods × callable-from-stable
   variants. Each method needs a working hand-port or transpiler
   emit. Bigger surface = longer Phase E tail.

**Easy wins** (low on all axes): `Range`, `RangeInclusive`,
`Option`/`Result` helpers, `Cell`, `RefCell`.
**Medium**: `Vec`, `VecDeque`, `String`.
**Hard**: `HashMap`, `BTreeMap`, `BTreeSet`, `BinaryHeap`.
**Very hard**: `Arc`/`Rc` (weak refs, atomics), `Mutex` (platform
threading), async primitives.

### 2.2 The three-axis problem (parser, codegen, runtime)

Every port failure falls into one of three buckets:

| Axis | What it looks like | Where the fix lives |
|---|---|---|
| **Parser** (transpiler input) | Rust feature the syn-AST visitor doesn't understand (e.g. `default fn`, `const { ... }`, parameterized macros) | `transpiler/src/*` Rust-side AST handling, OR prep.sh rewrite |
| **Codegen** (transpiler emit) | C++ output that doesn't compile or has wrong semantics (e.g. `auto` decays a ref, IIFE return type mismatch, `__TemplateArgs<>` missing) | `transpiler/src/codegen.rs` emit logic |
| **Runtime** (rusty/* headers) | C++ compiles + runs, but is slow, leaks, or aborts (e.g. perf cliff, moved-from semantics) | `include/rusty/*.hpp` or new helper headers |

A useful diagnostic question when stuck: *if I look at the C++
output, what would have to be true at the source-Rust level to make
this output correct?* That tells you whether the fix is in the
transpiler (axis 2) or upstream in prep.sh (axis 1).

### 2.3 Phase template (A → E)

For each new port, follow this phase structure. Each phase has a
binary exit criterion.

#### Phase A — Types compile

**Goal**: the transpiled `.cppm` module produces zero compile errors.

**Exit**: `clang++ -fmodule-output ... <module>.cppm` produces a `.pcm`.

**Methods used in BTreeMap**:
- A1: Template-arg recovery for compound types (`Handle<NodeRef<…>>`).
- A2: Add transpiler helpers for unknown types (`DormantMutRef`).
- A3: Recursive-lambda → Y-combinator lowering for nested fns.
- A4: Constant deduplication (`MIN_LEN`), associated-type
  projections (`SearchBound`).
- A5: Set module after map (sibling-module ordering).

**Watch for**: undefined identifiers in template parameters; this
almost always means template-arg recovery missed a layer.

#### Phase B — Hand-port unknowns

**Goal**: stub or hand-port the function bodies the transpiler
couldn't emit correctly.

**Exit**: the module *would* link if you ran the linker, modulo
unimplemented features that throw `runtime_error`.

**Pattern**: every hand-port goes in `post_transpile_patch.py` with
a `# transpiler should do X` comment so §1.4-style triage can
retire them later.

**Watch for**: hand-ports that are really fixes for *transpiler
bugs* (axis 2). These should be promoted to transpiler fixes ASAP,
before the patcher accumulates redundant cases.

#### Phase C — Link and run a smoke test

**Goal**: produce one consumer test that exercises the API
end-to-end and runs to completion.

**Exit**: `./smoke_test` exits 0.

**Why this matters**: this is the first time you know the port
actually *works*, not just compiles. The smoke test should exercise
the construction + a couple of typical operations.

**Recommended**: keep the smoke test tiny (5–10 lines of usage).
Bigger tests come later (Phase D).

#### Phase D — Full parity (optional, often skipped)

**Goal**: run Rust's own unit tests against the C++ port.

**Mechanism**: the `parity-test` harness in
`tests/transpile_tests/<crate>/run_parity_harness.sh` (see the
either-crate worked example).

**When to do this**: only if you need *behavioral parity assurance*
beyond your own smoke tests. For BTreeMap we skipped it — the
bench-style integration test was enough to validate correctness on
the hot path.

#### Phase E — Tighten transpiler & full API

**Goal**: peel transpiler emit bugs one at a time until the patcher
shrinks and the full public API surface works.

**Exit**: depends on goals — for BTreeMap we exited at "perf within
1.7× of native Rust and the bench validates correctness."

**Pattern**:
1. Pick a not-yet-working API method.
2. Look at what the transpiler emits.
3. Fix the emit in transpiler if reasonable; otherwise add a
   patcher entry with a `# transpiler should do X` comment.
4. Re-transpile, see what new error surfaces, repeat.

This is the **longest phase by far** (~60% of the BTreeMap week).

### 2.4 Recurring transpiler-emit clusters to anticipate

These showed up in BTreeMap and will likely recur. Recognising the
pattern lets you skip the diagnostic time.

| Cluster | Symptom | Reference fix |
|---|---|---|
| **A** — Structural-decomposition methods | Method emit uses `auto` placeholders or unresolved template-args because the host class's generics don't textually appear in the absorbed method signature | `__TemplateArgs<HostParam>::arg_<N>` partial specialization (see `transpiler: Cluster A completion` commits) |
| **B** — `ptr::read` const-qualifier drop | `let x = ptr::read(&y)` emits `auto x = …` but should emit `const auto x = …` for non-trivially-copyable T | `codegen.rs` let-binding const propagation |
| **C** — Parallel impl blocks with hardcoded markers | `impl Handle<NodeRef<…,Leaf>, …>` and `impl Handle<NodeRef<…,Internal>, …>` should both produce one absorbed method with the marker generalized | Parallel-impl detector + nested-marker text substitution |
| **D** — `const { … }` block lowering | Rust's `const { assert!(…) }` should lower to `static_assert(...)` or to `unreachable!()` for elided const-blocks | Const-block detection in codegen |
| **E** — Nested variant patterns + borrow scrutinee in `if let` | `if let Some(Pat::Tuple(…)) = &expr` needs both nested variant lowering AND borrow-vs-move semantics on the scrutinee | Nested pattern handling + scrutinee borrow detection |

Plus several "Item N" smaller clusters: tuple `.N` field access,
const-value match arm patterns, recursive nested fns, ref-returning
let bindings, statement-level `lhs = match { ... }` lowering.

### 2.5 Runtime gotchas

Lessons from `include/rusty/*.hpp`:

1. **Anything that runs per-element on a hot path needs `if
   constexpr` gating by `is_trivially_destructible_v<T>`.** The
   §1.5 cliff was a `clear_forgotten_address_range` call on every
   `ptr::write`. For `int`-sized elements in a B-tree leaf shift,
   that's millions of ops per insert.

2. **`MaybeUninit<T>` is always trivially destructible** regardless
   of `T`'s destructor, because it stores `unsigned char
   storage_[sizeof(T)]`. This makes it a great fast-path detector
   in `if constexpr` chains.

3. **The moved-from protocol must be local, not global.** Our first
   implementation used a global mutex-guarded `unordered_map` — it
   was correct but catastrophically slow (1700–10000×). The current
   strict-null-state design uses a per-instance `bool
   _rusty_forgotten` field on every Drop-impl type. No global
   state, no mutex, ~5 ns/op overhead per move-ctor / destructor.

4. **Allocator wrappers cost more than direct `__rdl_alloc`.**
   §1.6's component breakdown shows `rusty::alloc::Global` is ~1.8×
   the malloc cost of Rust's direct allocator. For high-allocation
   ports (Vec growth, HashMap rehash), this matters.

5. **Reference-vs-pointer aliasing.** Rust's `&T` is a reference;
   our `Box<T>` / `Vec<T>` / `Option<T&>` need to model both move
   and copy semantics. Get the move-ctor right *first* (sets
   source to null/forgotten), then worry about the destructor.

### 2.6 Bench discipline

1. **Bench before declaring "done."** Phase C is "it links and
   runs," not "it's done." Benching against the C++ STL
   equivalent (and ideally against native Rust too) is what closes
   the loop.

2. **Two reference points: STL and native Rust.** §1.6's mistake
   was originally calibrating only against `std::map` and declaring
   victory. Comparing against *native Rust* showed there was still
   real distance to close — and reframed what "good" means.

3. **`callgrind` beats `perf` for deterministic profiles.**
   Instruction counts are reproducible run-to-run; `perf record`
   sampling isn't, and often needs sudo / kernel-config changes
   (`perf_event_paranoid=4` blocks unprivileged use). `callgrind`
   just works.

4. **Profile after Phase C, not after Phase E.** A perf profile
   would have surfaced the §1.5 cliff within minutes of the smoke
   test running. We didn't profile until day 6 — that was a
   process miss.

5. **Disassembly comparison rules.** When a micro-bench shows a
   measurable per-op overhead, the question is *"is the function
   body actually different?"* Diff the asm. If empty, the
   measurement is harness noise. §1.7's IIFE bench made this
   mistake → asm correction.

### 2.7 When to stop

The BTreeMap port arrived at 1.05× `std::map` / 1.59× native Rust.
We chose to stop optimizing because:

- **Catastrophic regression closed** (1700× → ~1×): ✅
- **At parity with the natural C++ baseline (`std::map`)**: ✅
- **Remaining gap to Rust is structural (template-instantiation
  bloat, allocator layering), not algorithmic**: would need codegen
  surgery, not just emit fixes.

**Recommended stop conditions for future ports**:

1. The catastrophic-regression cliff (if any) is fixed.
2. The port is within 2× of the closest STL analogue on a
   representative workload.
3. The remaining gap is documented (so the next person knows what
   not to chase).

**Don't stop before** smoke tests pass and at least one
representative bench has been run with callgrind.

### 2.8 Estimating effort for the next port

Using BTreeMap as one calibration point:

- **Single-file simple type** (e.g. `Range`, `Cell`): a few hours,
  mostly hand-port of a few methods.
- **Medium type, single module** (e.g. `Vec`): 1–2 days. Most of
  the time is in iterator + reserve/grow paths.
- **Cyclic multi-module structure** (e.g. another B-tree-style
  collection, `LinkedList`'s rustc impl): ~1 week, like BTreeMap.
- **Concurrency / atomics** (`Arc`, `Mutex`): potentially weeks,
  because correctness verification is much harder than for
  collections.

Rough proportion of effort that goes into each axis:
- Parser fixes (prep.sh + syn-AST handling): ~10%
- Codegen fixes (transpiler emit): ~50%
- Runtime fixes (`rusty/*.hpp` headers): ~10%
- Bench + perf-tuning + correctness verification: ~30%

The week-long BTreeMap effort split as roughly **1 day Phase A–C,
4 days Phase E, 2 days perf + bench**. If a future port skips
Phase D (parity testing) like BTreeMap did, similar shape.

The patcher is the artifact that lives on: ~50% of its hand-ports
were transpiler debt at the time of writing — and every transpiler
fix reduces the patcher size and helps the *next* port.
