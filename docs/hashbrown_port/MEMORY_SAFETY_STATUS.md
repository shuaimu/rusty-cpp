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

## RESOLVED BY DIRECTIVE — this port is being retired, not repaired

**2026-07-27, Shuai:** *"I think directly translating hashbrown and using it for
rusty hash is for some historical reasons. We should from now on translate from
std, and indirectly from hashbrown."*

So the unfixed use-after-free above is not getting a fix here.
`rusty::HashMap`/`HashSet` move onto the **std** port (`docs/rusty/`, which
transpiles `std/src/collections/hash/{map,set}.rs` over a recursively
transpiled hashbrown dep). Tracking: task #178, and the directive section at
the top of `docs/port_regen/STATUS.md` for the remaining blockers.

Until that swap lands, the aliases still point here, so the bug above is still
shipped — treat `HashMap::clone()` as unsafe in the meantime.

## Consequence for test porting

`std/src/collections/hash/{map,set}/tests.rs` is 138 upstream tests (94 map +
44 set) and none are ported. They belong to the std port, not this one — they
exercise std's API surface, and porting them here would produce a suite that is
red for implementation reasons rather than translation reasons. Sequence:
finish the swap → then port the tests against the std port. Its per-member API
coverage is measured by `docs/rusty/api_coverage.sh` (46/54 at 2026-07-30).

The two existing ctest targets (`hashbrown_port_map_test`, 6 asserts;
`hashbrown_port_set_test`, 7) are smoke tests only — neither clones, and the map
one looks up through the raw table rather than `get()`, which is why none of the
above was caught.
