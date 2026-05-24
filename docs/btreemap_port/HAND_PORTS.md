# BTreeMap port — hand-ported function bodies

This doc tracks the **function bodies** in the transpiled BTreeMap output
that are *not* produced by the transpiler itself. They live in
`docs/btreemap_port/post_transpile_patch.py`, which runs after the
transpiler emits the `.cppm` modules and rewrites specific function
bodies via text-level patches.

Separate from this file:

- `STATUS.md` — overall port progress, phase-by-phase.
- `GENERIC_FIXES_PLAN.md` — patcher *text-fix* rules (regex/string
  rewrites) that should be lifted into the transpiler. Items 1–8.

This file covers a different class of patch: **entire function bodies
replaced with manually-written C++**, plus methods left as **stubs that
throw `runtime_error`** because nobody has implemented them yet.

The goal of this doc is to (a) catalogue what's hand-written so the
"is this transpiled?" question has a single source of truth, and (b)
analyse which hand-ports could be retired by a generic transpiler fix
vs. which need real human porting effort.

---

## Hand-ports (full function bodies)

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

## Stubs that throw `runtime_error` (NOT implemented)

If these are called at runtime, they throw `rusty-cpp-transpiler: …`.

| Method | File | Patcher fn |
|--------|------|------------|
| `OccupiedEntry::key` | map.cppm, map.entry.cppm | `stub_broken_map_methods`, `stub_broken_entry_method` |
| `OccupiedEntry::get_mut` | map.entry.cppm | `stub_broken_map_entry_methods` |
| `OccupiedEntry::into_mut` | map.entry.cppm | `stub_broken_map_entry_methods` |
| `OccupiedEntry::into_key` | map.cppm, map.entry.cppm | `stub_broken_map_methods`, `stub_broken_entry_method` |

The benchmark exercises only `insert` + `get`; these paths don't hit
the OccupiedEntry stubs. Anything that walks an Entry would.

---

## Root-cause categories

Group the hand-ports by *why* the transpiler couldn't handle them.
This is the lens for deciding "could a generic transpiler fix
retire this hand-port?"

### Category A — `Box::into_non_null_with_allocator` destructure

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

### Category B — `MaybeUninit` slot writes through generic-parameterized arrays

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
Item 2 in `GENERIC_FIXES_PLAN.md` (`slice.get_unchecked[_mut](i) →
slice[i]`). A proper fix is to thread element types through generic
array accesses so the `MaybeUninit<K>` slot type can be deduced at the
call site. Doable but not trivial — touches the type-inference paths
that already failed once on this code (see Iter 72–79 in the task
history).

### Category C — `loop { match self.force() { Leaf(x) => …, Internal(y) => … } }`

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

### Category D — Method-template params that fail deduction

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
GENERIC_FIXES_PLAN item 7 ("Wrong template-arg recovery").

### Category E — Composite: `BTreeMap::entry`

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

### Category F — Genuine unimplemented surface (stubs)

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

---

## Summary: which hand-ports could be retired by transpiler fixes

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

The current ~1700–10000× perf gap vs. `std::map` is largely *not*
caused by hand-port shortfalls — the hot path (`insert` → `entry` →
`search_tree` → `force()` → `push_with_handle`) is mostly hand-ported
and should be roughly hand-quality C++. The gap is from:

- SFINAE dispatcher lambdas around every method call (`[&](auto&&
  __recv) -> decltype(auto) { if constexpr (requires {…}) … }
  (receiver)`) defeating the inliner.
- `std::variant<...>` + `.index()` dispatch instead of direct branches.
- Generic `rusty::alloc::Global` allocator wrapper overhead.

Those are emit-shape issues, separate from the hand-port catalog
above. They'd need their own plan to address.
