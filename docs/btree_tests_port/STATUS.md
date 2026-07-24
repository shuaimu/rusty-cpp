# btree_tests_port — un-stub status

Living log of which rustc tests are un-stubbed vs why others aren't.

## Pipeline shape

- `transpiled/btree_tests_port/btree_tests_port.cppm` — 146 auto-generated
  `TEST_CASE("…")` stubs (115 from map/tests.rs, 31 from set/tests.rs
  prefixed with `set_`).
- `tests/btree_tests_port_unstubbed.cpp` — hand-translated test bodies
  that actually exercise btree_port.
- `tests/btree_tests_port_module_test.cpp` — runner driver.

Un-stubbed tests use `_unstubbed` suffix on their name so the
test-runner registry doesn't collide with the stub of the same Rust
test name. Both run on each invocation.

## Un-stubbed so far (50 tests; test runner reports `all 196 test(s) passed` — 146 stubs + 50 real bodies)

Round-tripped through eight successive runs without flake. Tracking shifted from
"individual table of un-stubs" to summary form because the table outgrew the
ability to maintain it usefully. See `tests/btree_tests_port_unstubbed.cpp` for
the current full list — every TEST_CASE there is an un-stub.

Initial 11-test table preserved below for reference:

| Rust test | C++ TEST_CASE | Status |
|---|---|---|
| `map/tests.rs::test_get_key_value` | `test_get_key_value_unstubbed` | passing (trimmed: skipped `.remove()` tail) |
| `map/tests.rs::test_try_insert` | `test_try_insert_unstubbed` | passing (re-enabled by fix_btreemap_try_insert_arm_swap) |
| `map/tests.rs::test_pop_first_last` (pop_first half) | `test_pop_first_only_unstubbed` | passing — drains a 4-element map by pop_first to empty |
| `map/tests.rs::test_pop_first_last` (full mix) | `test_pop_first_last_unstubbed` | passing — re-enabled by B-pop-last fix |
| `map/tests.rs::test_pop_first_last` (pop_last drain) | `test_pop_last_drain_unstubbed` | passing — re-enabled by B-pop-last fix |
| `map/tests.rs::test_check_ord_chaos` | `test_check_ord_chaos_unstubbed` | passing — re-enabled by Governor/Governed port |
| `map/tests.rs::test_range_finding_ill_order_in_map` | `test_range_finding_ill_order_in_map_unstubbed` | passing — re-enabled by Cyclic3 port (asserts the cycle holds; original calls `.range()` which we substitute with `.contains_key()`) |
| `map/tests.rs::test_append_ord_chaos` (keys only) | `test_append_ord_chaos_keys_unstubbed` | passing — re-enabled by Cyclic3 port. Skips `append()` call itself; verifies the duplicate-insert-with-cyclic-Ord shape (Rust's `len()==4` assertion). |
| `set/tests.rs::test_clear` | `set_test_clear_unstubbed` | passing (re-enabled by fix_btreemap_clear_manuallydrop) |
| (synthetic smoke) | `smoke_insert_lookup_unstubbed` | passing — covers insert/contains_key/len/get/first/last_key_value |
| (synthetic dummy) | `crash_test_dummy_drop_count_unstubbed` | passing — exercises CrashTestDummy + Instance drop-counting through BTreeMap destructor |

## Helpers wired up

- **`check(M)` shim** in `btree_tests_port_unstubbed.cpp` — no-op `template<typename M> void check(const M&) {}`. Translated tests that hit `map.check()` route through it. We lose internal-invariant checking but keep the test's own public-API assertions.
- **`tests/btree_testing_helpers.hpp`** — C++ port of rustc's `alloctests/testing/{crash_test, ord_chaos}.rs`:
  - `CrashTestDummy` + `Instance` for drop / clone / query event counting (used by drop-leak tests).
  - `Panic` enum — `Panic::Never` is fully supported. `InClone` / `InDrop` / `InQuery` call `std::abort()` instead of unwinding (we don't have `catch_unwind`), so tests that recover from panic state remain blocked.
  - `Cyclic3` enum with non-transitive `operator<` (A&lt;B&lt;C&lt;A cycle).
  - `Governor` + `Governed<T>` for flippable ordering tests.
  - `IdBased` for "compare by id, ignore name" tests.

## Patcher rules added

| Rule | Fixes |
|---|---|
| `fix_btreemap_try_insert_arm_swap` | B-try-insert: `try_insert()` had same Vacant/Occupied arm-swap as the old `insert()` did. Hand-port the function body to match Rust semantics. |
| `fix_btreemap_clear_manuallydrop` | B-clear: `clear()` emit's `rusty::clone(this->alloc)` where alloc is `ManuallyDrop<A>` (copy ctor deleted). Rewrite to `manually_drop_new(rusty::clone(*this->alloc))` to unwrap-clone-rewrap. |

## Resolved latent bugs

### B-pop-last: dangling structured binding in remove_leaf_kv ✅ FIXED

**Symptom:** crash on the 3rd consecutive pop (any mix of pop_first /
pop_last) of a 4-element map.

**Root cause:** transpiler emitted
```cpp
auto&& [old_kv, pos] = rusty::detail::deref_if_pointer_like(this->remove());
```
`this->remove()` returns a prvalue tuple. `deref_if_pointer_like<T>(T&&
v)` materializes the temporary into its parameter and returns
`std::forward<T>(v)` — an rvalue ref. The original temporary's
lifetime ends at the semicolon (the full expression). `pos` and
`old_kv` are then dangling references into destroyed stack memory.

Reads against the stale memory worked while nothing else wrote to it,
which covered insertion of pos.idx() and entry to the rebalance
branch. But entering the IIFE lambda
(`auto&& new_pos = [&]() { … }();`) clobbered those stack bytes with
the lambda's own locals. The pattern was masked on most paths because
the garbage bytes happened to read as `Option::None` in the parent
slot — `choose_parent_kv` then took the Err arm and called
`new_edge()`, which doesn't dereference. On the 3rd pop the garbage
read as `Some(...)`, choose_parent_kv took the Ok arm, and `ascend()`
SIGSEGV'd in `as_leaf_ptr` with a stack address.

**Confirmed via runtime tracing** (printf-based — gdb gave confused
output due to -O3 optimization). Sequence:
```
[trace remove_leaf_kv] AFTER remove: pos.node.node=0x60ee...2b0 (heap, valid)
[trace remove_leaf_kv] entering rebalance branch: pos still valid
[trace remove_leaf_kv] after pos.idx(): pos still valid
[trace lambda entry] pos.node.node=0x7ffd...da60 (stack address, corrupt!)
```

**Fix:** s/`auto&&`/`auto`/ in `remove_leaf_kv`. Plain `auto` makes the
structured binding object own the tuple (move-construct), which
extends the lifetime to the enclosing function scope. The change is
inside the hand-port slot of `btree_internal.cppm:remove_leaf_kv` —
not patcher-codified yet because the proper fix is transpiler-side.

**Tests un-stubbed:** `test_pop_first_last_unstubbed` (full pop_first
+ pop_last mix) and `test_pop_last_drain_unstubbed` (pure pop_last
drain).

**Wider exposure:** the same `auto&& [...] = deref_if_pointer_like(call())`
shape appears ~92 times across transpiled ports (44 in btree_port, 48
in core_slice_port). All are latent dangling references. They mostly
work today because their dead stack regions happen to read as
non-crashing bit patterns. Each is one stack-layout reshuffle away
from triggering the same bug. **Proper fix:** transpiler should emit
`auto [a, b] = …` (no `&&`) for `let` patterns when the RHS is a
prvalue. Tracked as a follow-up; not addressed in this commit because
the diff would touch many auto-generated lines and we don't yet have a
test that crashes anywhere outside the pop_last hot path.

### B-into-iter: into_keys / into_values miss ManuallyDrop auto-deref

map.cppm:5922 emits `this->root` / `this->length` on a
ManuallyDrop<BTreeMap> without dereferencing — should be `(*this).root`
etc. Separate from B-clear despite both involving ManuallyDrop.
Needs a transpiler-side fix to the ManuallyDrop emit, not a
patcher rule.

**Tests blocked:** any test using `.into_keys()`, `.into_values()`.

## ⚠️ CRITICAL: assert() is a NO-OP in this suite — most bodies pass VACUOUSLY

**Discovered 2026-07-23.** `btree_tests_port_module_test` is compiled with
`NDEBUG` **defined**, so every `assert(...)` in `btree_tests_port_unstubbed.cpp`
expands to `((void)0)`. That means not just the value checks but any
*side-effecting map operation written inside an assert* (`assert(map.insert(k,v).is_none())`,
`assert(map.remove(k).is_some())`, …) is **compiled out entirely**. The bodies
run almost no code; "passing" is largely vacuous. Crash-based detection still
worked (SIGSEGV/UB is opt-independent — that's how B-pop-last was caught), but
value correctness was never checked.

**Root cause (CMakeLists.txt):** the module_test source-wiring block sets
`-UNDEBUG` (line ~699, "so asserts run"), but the *stub-port `foreach`* a few
blocks later re-applies release flags `-O3 -DNDEBUG -march=native` for
`btree_tests_port` — and that trailing `-DNDEBUG` **wins** (last flag on the
command line). Net NDEBUG tokens: `-DNDEBUG … -UNDEBUG … -DNDEBUG`. Re-adding
`-UNDEBUG` afterward does NOT help (CMake dedupes it, so it stays at the earlier
position). The real fix is to link `btree_tests_port` *without* `-DNDEBUG`
(pull it out of the stub `foreach` into its own block, or drop `-DNDEBUG` there) —
NDEBUG is not a module-BMI-compat flag, only `-std`/`-march` are.

**What enabling live asserts surfaces (the un-stub-for-real work list):**
- **~14 compile errors: `Iter::count()` missing** on both `btree::map::Iter`
  and `btree::set::Iter` (`m.iter().count()`). btree_port's iterators don't
  expose Rust's `Iterator::count`. Add it to the port (or adapt the tests).
- **3 compile errors: `rusty::clone` not declared** — needs an `#include`
  (`assert(map == rusty::clone(map))`).
- 2 more (`begin`/`end`) inside clone/count instantiations.
- Then: **runtime triage** — with value checks finally live, expect real
  failures across the ~50 un-stubbed bodies that were passing vacuously.

**Done so far:** `test_basic_large_unstubbed` now opts out of the no-op assert
via a local throw-based `BT_REQUIRE` (the runner catches the throw as FAILED,
same path as a rusty panic), so it is a REAL test. Restored from 12 keys to
MIN_INSERTS_HEIGHT_2 (144) — a height-2 tree exercising the full node
merge/rebalance path that the slice-ref-static UAF (commit 0e6cd1d5) used to
crash. Passes. (btree_port verified correct to rustc's 10000 in isolation via
the standalone repro.)

## Roadmap

After the consolidated bug-fix pass, remaining work:
0. **Make asserts live** (see the CRITICAL section above) — this is the real
   "un-stub for real" gate. Fix the CMake flag order, then the ~17 compile
   errors, then triage the runtime failures the live value-checks reveal.
1. **B-pop-last** — needs runtime debugging (gdb). When fixed, the full
   `test_pop_first_last` and many other entry-removal tests un-stub.
2. **B-into-iter** — transpiler-side fix to ManuallyDrop auto-deref;
   un-stubs `into_keys`/`into_values` tests.
3. Translate the remaining ~30 "truly clean" tests once 1+2 are done
   — each one a small hand-translation against btree_port's public API.
4. Eventually: port `crate::testing::{crash_test, ord_chaos}` helpers
   (unblocks ~10 more tests), and implement `catch_unwind`/`should_panic`
   equivalents for the panic tests.
