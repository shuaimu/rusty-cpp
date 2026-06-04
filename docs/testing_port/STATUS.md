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

### C++ compile (next-tier blockers)

The two originally-reported transpiler emit bugs have been fixed
upstream in commit `e298bf8 transpiler: generic .eq()/.ne() dispatch
+ qualify glob-imported enum variants`:

- ✅ **Bug 1**: `.eq()` on primitive receiver now lowers via the SFINAE
  helper `rusty::cmp::eq(lhs, rhs)`, which dispatches to `lhs.eq(b)`
  when available, else `lhs == b`.
- ✅ **Bug 2**: Bare enum-variant idents (`A`, `Equal`) brought in via
  `use EnumName::*;` are now qualified to `Cyclic3::A` /
  `Ordering::Equal` at the pattern and path-expr emit sites.

Re-transpiling against current HEAD now leaves the crash_test and
ord_chaos files past the originally-blocking errors. Compile then
surfaces two NEW downstream blockers that the original bugs masked:

1. **`testing_port.ord_chaos.cppm:3729`** — `ptr::eq` on references:
   ```
   error: no matching function for call to 'eq'
   assert((ptr::eq(this->_1, other._1)));
   ```
   Rust `ptr::eq(self.1, other.1)` auto-coerces `&T` to `*const T`;
   `rusty::ptr::eq` accepts only raw pointers. Either accept references
   in the helper or emit `&this->_1` / `std::addressof(this->_1)`.

2. **`testing_port.crash_test.cppm:354`** — `rusty::Vec` lookup:
   ```
   error: no template named 'Vec' in namespace 'rusty'
   ```
   Pre-existing namespace issue with `rusty::Vec` visibility from
   inside the transpiled module — same shape as the deep-ns migration
   work in other ports. Likely needs an `import vec_port.vec;` or the
   `--cxx-namespace` flag treatment.

3. **`testing_port.cppm:33`** — depends on (1) and (2) since the
   umbrella `import testing_port.crash_test;` fails when crash_test
   won't compile. Will resolve once crash_test does.

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

The originally-blocking transpiler bugs (Option A from the prior plan)
are now fixed (commit `e298bf8`). Next-tier blockers above (ptr::eq
on references, rusty::Vec module visibility) are the remaining work
to fully unblock crash_test + ord_chaos compilation.

**Recommended next step:** Fix `rusty::ptr::eq` to accept references
(small helper change in `include/rusty/ptr.hpp`), then chase the Vec
visibility issue. Once both pass, the testing_port helpers should
build clean and unblock the binary_heap test ports.
