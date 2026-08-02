# hashbrown_port memory-safety status

**RETIRED FOR CONSUMERS (2026-08-02, #177/#178/#185).** `rusty::HashMap` /
`rusty::HashSet` no longer alias this port: since the std-port retarget they
alias `::std_port::collections::hash::{map,set}` (include/rusty/rusty.cppm),
and the transpiler routes `rusty::HashMap`/`HashSet` at `std_port` too
(RUSTY_MODULE_TRIGGERS). Nothing new should import `hashbrown_port.*` — apart
from the bugs below, its entry API does not even compile
(`VacantEntry` has no `into_mut`; `OccupiedEntry::insert` is not
const-correct — verified empirically during #185).

**THE CMAKE TARGET IS DELETED (2026-08-02).** The last consumer —
`transpiled/vec_tests_port`'s `ZstTracker` member and `use std::collections::
HashMap` translation — turned out to need only type-completeness (the HashMap
using-decl had zero call sites and ZstTracker is never instantiated; its test
is SKIP'd), so it was migrated to std_port by three one-line substitutions and
its 151 tests pass unchanged. With that, the target, its two regression tests
(tests/hashbrown_port_{map,set}_test.cpp), and the `-lhashbrown_port` link
line in transpiler/src/main.rs were deleted together, per the checklist.

The vendored sources below remain in transpiled/hashbrown_port/ as the
artifact this document describes — nothing builds them.

--- Original status (pre-retirement context) follows. ---

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
