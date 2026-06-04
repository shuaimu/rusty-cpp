# testing_port — pilot for transpiling rustc test helpers

Tests for rustc collection ports live in `library/alloctests/tests/`
and depend on a small `library/alloctests/testing/` helper crate
(NOT the external `rand` crate — they roll their own
`DeterministicRng` instead). This port vendors that helper crate so
the actual test files (e.g. `binary_heap.rs`, `vec.rs`, `btree/map/tests.rs`)
can compile against transpiled helpers.

## Source

`~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloctests/testing/`

| File           | Lines | Purpose                                      |
|----------------|-------|----------------------------------------------|
| `rng.rs`       |    28 | `DeterministicRng` — pure XorShift, no deps |
| `crash_test.rs`|   122 | `CrashTestDummy`, `Panic` — drop/clone counting + panic-on-event |
| `ord_chaos.rs` |   111 | `Cyclic3`, `Governor`, `IdBased` — Ord violators for stress testing ordered collections |
| `macros.rs`    |    37 | assertion helpers                            |
| `mod.rs`       |     4 | module declarations                          |

## Status

### Transpile clean ✓

After running `prep.sh`, all 5 files transpile with **0 parser errors**.

### C++ compile clean ✓ (2 of 5)

- `testing_port.rng.cppm` — `DeterministicRng` works as-is.
- `testing_port.macros.cppm` — assertion helpers work as-is.

### C++ compile blocked (3 of 5)

The remaining files hit known existing transpiler-emit bugs (not
specific to this port — they would surface on any port exercising
these patterns). Documented for future patcher / transpiler work:

1. **`testing_port.crash_test.cppm:3796`** —
   ```
   error: member reference base type 'size_t' is not a structure or union
   return this->id().eq(rusty::detail::deref_if_pointer_like(other.id()));
   ```
   `Instance::id()` returns `size_t` (primitive). Transpiler emits
   `.eq()` member call on a primitive — should lower to `==`.

2. **`testing_port.ord_chaos.cppm:3670`** —
   ```
   error: use of undeclared identifier 'A'
   if (_m0 == A && _m1 == A) { return Equal; }
   ```
   `A` is a variant of the local enum `Cyclic3`. The transpiler emits
   bare enum-variant identifiers in lifted match arms; they should be
   qualified (`Cyclic3::A`) or the file should `using` them in scope.

3. **`testing_port.cppm:33`** — depends on (1) since the umbrella
   `import testing_port.crash_test;` fails when crash_test won't
   compile. Will resolve once crash_test does.

Also of note (less blocking, same shape as hashbrown port):

- All 3 files reference `rusty::Vec<uint8_t>` in their serde-de
  `visit_byte_buf` prelude. Same fix as hashbrown patcher:
  stub `visit_byte_buf` body to return Err.

## What's usable today

`testing_port.rng.cppm` is the immediate win — `DeterministicRng` is
referenced by ~10+ rustc collection test files for reproducible random
inputs. Any test port can `import testing_port.rng;` and use
`DeterministicRng::new_()` + `.next()` directly.

## Path forward

Two options for unblocking the remaining 3 files:

**Option A — transpiler fixes.** The `.eq()`-on-primitive and bare-enum
patterns affect many ports; landing fixes upstream pays off
elsewhere too. ~2-3 days.

**Option B — manual stubs for crash_test + ord_chaos.** Hand-write the
~250 lines as inline `.hpp` files. Fast (a few hours), but the work
doesn't generalize.

For piloting collection tests, **Option B is the right first move** —
get `binary_heap.rs` tests transpiling against a stub, learn what
patches the test file itself needs, then circle back to fix the
helpers properly.
