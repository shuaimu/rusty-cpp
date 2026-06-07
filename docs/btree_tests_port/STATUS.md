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

## Un-stubbed so far (5 real tests + the synthetic smoke)

| Rust test | C++ TEST_CASE | Status |
|---|---|---|
| `map/tests.rs::test_get_key_value` | `test_get_key_value_unstubbed` | passing (trimmed: skipped `.remove()` tail) |
| `map/tests.rs::test_try_insert` | `test_try_insert_unstubbed` | passing (re-enabled by fix_btreemap_try_insert_arm_swap) |
| `map/tests.rs::test_pop_first_last` (pop_first half) | `test_pop_first_only_unstubbed` | passing — drains a 4-element map by pop_first to empty |
| `set/tests.rs::test_clear` | `set_test_clear_unstubbed` | passing (re-enabled by fix_btreemap_clear_manuallydrop) |
| (synthetic smoke) | `smoke_insert_lookup_unstubbed` | passing — covers insert/contains_key/len/get/first/last_key_value |

## Helpers wired up

- **`check(M)` shim** in `btree_tests_port_unstubbed.cpp` — no-op `template<typename M> void check(const M&) {}`. Translated tests that hit `map.check()` route through it. We lose internal-invariant checking but keep the test's own public-API assertions.

## Patcher rules added

| Rule | Fixes |
|---|---|
| `fix_btreemap_try_insert_arm_swap` | B-try-insert: `try_insert()` had same Vacant/Occupied arm-swap as the old `insert()` did. Hand-port the function body to match Rust semantics. |
| `fix_btreemap_clear_manuallydrop` | B-clear: `clear()` emit's `rusty::clone(this->alloc)` where alloc is `ManuallyDrop<A>` (copy ctor deleted). Rewrite to `manually_drop_new(rusty::clone(*this->alloc))` to unwrap-clone-rewrap. |

## Remaining latent bugs

### B-pop-last: pop_last on the **3rd** consecutive pop crashes

**Reproduce:**
```cpp
auto m = make_map<int, int>();
m.insert(1, 10); m.insert(2, 20); m.insert(3, 30); m.insert(4, 40);
m.pop_first();  // ok
m.pop_first();  // ok
m.pop_last();   // ← aborts mid-execution
```

**What I know:**
- `pop_first` works in isolation, even drained to empty.
- `pop_last` works in isolation for 1 or 2 calls.
- `pop_last` aborts on the 3rd consecutive call, regardless of what
  came before (pop_first or pop_last).
- `last_entry()` itself returns OK. The crash is inside
  `OccupiedEntry::remove_entry()` → `Handle::remove_kv_tracking()` →
  somewhere in the rebalance/merge path.
- `NodeRef::remove()` emit does `slice_remove(..., std::move(this->idx_field))`
  twice for key and value columns. Harmless for size_t (std::move on
  a primitive is a no-op for the source) but worth noting.
- The 3rd pop fires when the leaf is at len=2 (well under MIN_LEN=5),
  so the rebalance branch in `remove_leaf_kv` is triggered every time.
- `choose_parent_kv()` on a single-leaf root returns Err — the
  Err-arm constructs a new edge handle but doesn't actually merge or
  steal. Yet the crash happens here.

**Hypothesis (unconfirmed):** the leaf storage layout after pop_first
ends up with valid data at slots [0, len) and uninitialized memory in
later slots. NodeRef::remove() at the rightmost valid index then
either reads uninit garbage or trips an invariant inside
`assume_init_read` / `slice_remove` / the post-remove rebalance.

**Next investigation step:** gdb / runtime tracing through
`remove_kv_tracking` → `remove_leaf_kv` → `choose_parent_kv` →
`new_edge(pos, idx)`. Specifically check the values of `pos.node.len`,
`pos.idx`, and the actual KV slot contents before and after each step.

**Tests blocked:** the full `test_pop_first_last` (only pop_first half
runs), and anything that exercises a drain mixing pop_first/pop_last
or repeats either past 2 calls.

### B-into-iter: into_keys / into_values miss ManuallyDrop auto-deref

map.cppm:5922 emits `this->root` / `this->length` on a
ManuallyDrop<BTreeMap> without dereferencing — should be `(*this).root`
etc. Separate from B-clear despite both involving ManuallyDrop.
Needs a transpiler-side fix to the ManuallyDrop emit, not a
patcher rule.

**Tests blocked:** any test using `.into_keys()`, `.into_values()`.

## Roadmap

After the consolidated bug-fix pass, remaining work:
1. **B-pop-last** — needs runtime debugging (gdb). When fixed, the full
   `test_pop_first_last` and many other entry-removal tests un-stub.
2. **B-into-iter** — transpiler-side fix to ManuallyDrop auto-deref;
   un-stubs `into_keys`/`into_values` tests.
3. Translate the remaining ~30 "truly clean" tests once 1+2 are done
   — each one a small hand-translation against btree_port's public API.
4. Eventually: port `crate::testing::{crash_test, ord_chaos}` helpers
   (unblocks ~10 more tests), and implement `catch_unwind`/`should_panic`
   equivalents for the panic tests.
