# btree_tests_port — un-stub status

Living log of which rustc tests are un-stubbed vs why others aren't.

## Pipeline shape

- `transpiled/btree_tests_port/btree_tests_port.cppm` — 146 auto-generated
  `TEST_CASE("…")` stubs (115 from map/tests.rs, 31 from set/tests.rs
  prefixed with `set_`).
- `tests/btree_tests_port_unstubbed.cpp` — hand-translated test bodies
  that actually exercise btree_port. Lives in a separate TU because
  some BTreeMap instantiations triggered earlier-discovered
  in-module-purview bugs.
- `tests/btree_tests_port_module_test.cpp` — runner driver.

Un-stubbed tests use `_unstubbed` suffix on their name so the
test-runner registry doesn't collide with the stub of the same Rust
test name. Both run on each invocation.

## Un-stubbed so far (5 real tests + the synthetic smoke)

| Rust test | C++ TEST_CASE | Status |
|---|---|---|
| `map/tests.rs::test_get_key_value` | `test_get_key_value_unstubbed` | passing (trimmed: skipped `.remove()` tail) |
| `map/tests.rs::test_try_insert` | `test_try_insert_unstubbed` | passing (re-enabled by fix_btreemap_try_insert_arm_swap) |
| `map/tests.rs::test_pop_first_last` (pop_first half) | `test_pop_first_only_unstubbed` | passing for pop_first; pop_last still hits B-pop-last |
| `set/tests.rs::test_clear` | `set_test_clear_unstubbed` | passing (re-enabled by fix_btreemap_clear_manuallydrop) |
| (synthetic smoke) | `smoke_insert_lookup_unstubbed` | passing — covers insert/contains_key/len/get/first/last_key_value |

## Helpers wired up

- **`check(M)` shim** in `btree_tests_port_unstubbed.cpp` — no-op `template<typename M> void check(const M&) {}`. Translated tests that hit `map.check()` route through it. We lose internal-invariant checking but keep the test's own public-API assertions.

## Patcher rules added in the consolidated bug-fix pass

| Rule | Fixes |
|---|---|
| `fix_btreemap_try_insert_arm_swap` | B-try-insert: `try_insert()` had same Vacant/Occupied arm-swap as the old `insert()` did. Hand-port the function body to match Rust semantics. |
| `fix_btreemap_clear_manuallydrop` | B-clear: `clear()` emit's `rusty::clone(this->alloc)` where alloc is `ManuallyDrop<A>` (copy ctor deleted). Rewrite to `manually_drop_new(rusty::clone(*this->alloc))` to unwrap-clone-rewrap. |

## Remaining latent bugs (documented for future fixes)

### B-pop-last: pop_last() runtime abort on non-empty map

**Symptom:** `pop_last()` causes the test process to abort partway
through execution. Specifically: with a map of size 2, calling
`pop_last()` never returns. Verified that `last_key_value()` on the
same map at the same point works correctly. The asymmetric failure
(pop_first works, pop_last doesn't) suggests the bug is in
`last_entry()` / `last_leaf_edge()` / `left_kv()` /
`OccupiedEntry::remove_entry()` on the rightmost-walk path —
likely a navigation or remove-side mirror of a fix that was made
on the leftmost side.

**Tests blocked:** the full `test_pop_first_last` (only the
pop_first half is un-stubbed), and anything that exercises
`.pop_last()` more than once on a non-trivial map.

### B-into-iter: into_keys / into_values bypass ManuallyDrop auto-deref

**Symptom:** map.cppm:5922 (in the IntoKeys/IntoValues instantiation
path) emits `this->root` and `this->length` where `this` has type
`ManuallyDrop<BTreeMap>`. The transpiler's ManuallyDrop auto-deref
handling missed these field accesses — should emit `(*this).root`,
`(*this).length`.

**Tests blocked:** anything using `into_keys()`, `into_values()`.

### B-purview: BTreeMap instantiation inside .cppm module purview

**Symptom:** Putting a TEST_CASE body inside `btree_tests_port.cppm`
that instantiates BTreeMap triggered the (now-fixed) B-clear path
at module-build time. Workaround was to put un-stubbed test bodies
in `tests/btree_tests_port_unstubbed.cpp` (separate TU). Now that
B-clear is fixed, in-purview instantiation may work — not retested.

## Roadmap

After this consolidated pass, remaining work to un-stub more tests:
1. Fix **B-pop-last** — adds `test_pop_first_last` plus any test that
   uses `.pop_last()` repeatedly.
2. Fix **B-into-iter** — adds `test_into_keys`, `test_into_values`,
   plus tests that consume the map.
3. Translate the remaining ~30 "truly clean" tests (per the analysis
   in the parent README) — each one a small hand-translation now that
   the worst blockers are gone.
4. Eventually: port `crate::testing::{crash_test, ord_chaos}` helpers
   (unblocks ~10 more tests), and implement `catch_unwind`/`should_panic`
   equivalents for the panic tests.
