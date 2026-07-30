# hashbrown_port memory-safety status

`rusty::HashMap` / `rusty::HashSet` (include/rusty/rusty.cppm:168,172) alias
`rusty::port::collections::hashbrown::HashMap` / `HashSet` from this port, so
everything here is what users get.

**The vendored .cppm files are a stale June emission.** The current transpiler
does not reproduce the bugs below — verified for the match-move case: a
minimal `let x = match f() { Ok(t) => t, ... }` now lowers to a non-const
`auto&& _m = f();` IIFE, which selects the moving `unwrap()`. These are
artifacts, not live transpiler bugs. **The real fix is regeneration, not
hand-patching**; the fixes below were applied by hand only because they are
memory-safety bugs in shipped code and regeneration is a separate project.

## Fixed (verified with ASan)

1. `RawTable::clone()` — `const auto result = new_uninitialized(...)` then
   `auto new_table = result.unwrap()`. `Result` has a const `unwrap()` that
   returns `const T&`, so a const scrutinee made `new_table` COPY-construct,
   giving two owners of one allocation. `HashMap::clone()` double-freed
   immediately. Fixed by dropping `const` (the non-const `unwrap()` moves).

2. `reserve_rehash_inner` and `fallible_with_capacity` — `auto&& x =
   deref_if_pointer(opt.unwrap())`. The non-const `Option::unwrap()` returns
   BY VALUE, so `deref_if_pointer` yields a reference into a temporary that
   dies at the end of the full-expression; `auto&&` bound the corpse. ASan:
   stack-use-after-scope on the first `insert` that rehashes. Fixed by binding
   by value.

## NOT fixed — HashMap::clone still has a heap-use-after-free

After both fixes, `insert` x200 + `clone()` succeeds, but reading from the
clone trips ASan:

```
heap-use-after-free ... in HashMap::get(...)   map.cppm:4603
freed by:  RawTable::~RawTable -> drop_inner_table -> free_buckets -> Global::deallocate
```

A temporary `RawTable` is being destroyed while another still points at the
same buckets. The enabler is visible at raw.cppm: `RawTable(const RawTable&) =
default` — a bitwise copy of a type that OWNS its allocation. Rust's `RawTable`
is not `Copy`; cloning goes through the explicit `clone()`.

Deleting the copy constructor is the right shape and would turn every
accidental copy into a compile error. It was tried and reverted: it compiles
the module fine (template bodies are only checked on instantiation) but breaks
at instantiation because `rusty::Result`'s storage currently requires
copy-constructibility. Making `Result` store non-copyable payloads correctly is
the prerequisite.

Reproducer: a map with enough inserts to rehash, then `clone()`, then `get()`
on the clone, built with `-fsanitize=address`.

## Consequence for test porting

`std/src/collections/hash/{map,set}/tests.rs` is ~75 upstream tests and none are
ported. Porting them against this implementation would produce a suite that is
red for implementation reasons rather than translation reasons, which is not
useful. Sequence the work as: regenerate the port (or retarget onto the `rusty`
std-port, which is already a green parity-matrix target with a working
HashMap/HashSet subset) → then port the tests.

The two existing ctest targets (`hashbrown_port_map_test`, 6 asserts;
`hashbrown_port_set_test`, 7) are smoke tests only — neither clones, and the map
one looks up through the raw table rather than `get()`, which is why none of the
above was caught.
