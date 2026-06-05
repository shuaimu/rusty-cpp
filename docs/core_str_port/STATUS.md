# core_str_port — Phase A1 in progress (collapse infrastructure)

`library/core/src/str/*.rs` (9 files, 8838 LOC source) — collapse +
prep infrastructure landed; transpile blocked on use-statement dedup.

## Pipeline

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ 9 files from rustup core/src/str/ |
| 2. Prep | ✅ `prep.sh` — crate:: → std::, derive_const → derive, #[cold] on expr, multi-line assert_unsafe_precondition! |
| 3. Collapse | ⚠️ Body merge + crate-attr strip works; multi-line use-import dedup partial |
| 4. Transpile | 🔴 Blocked: duplicate symbol imports from braced uses |

## Current blocker

`cargo check` errors with duplicate imports:

```
error[E0252]: the name `FusedIterator` is defined multiple times
 --> src/lib.rs:15:46
13 | use std::iter::FusedIterator;
14 | use std::iter::{
15 |     Chain, Copied, Filter, FlatMap, Flatten, FusedIterator, ...};
```

The collapse dedupes single-line uses by leaf symbol but multi-line
braced uses contain BOTH already-seen and new symbols; current pass
keeps them whole.

## Next steps

**A. Smarter use-rewrite** (recommended): when a multi-leaf import
contains seen symbols, REWRITE it to drop the seen ones. Multi-line
braced-use parsing extends collapse.py cleanly. ~half-day.

**B. Hand-port the str surface** (sibling alternative discussed in
docs/string_port/STATUS.md): port just the ~5 fns String actually uses
(`find`, `starts_with`, `contains`, `split`, `replace`) without
the full Pattern trait machinery. Faster but less complete.

## Files

- `Cargo.toml.template` — minimal lib crate manifest
- `prep.sh` — crate:: → std::, attribute normalization, multi-line
  `assert_unsafe_precondition!` stripping, `#[cold]` on expr removal
- `collapse.py` — 8-submodule flatten + crate-attr stripping +
  single-line use dedup. Multi-line braced-use rewrite is the next add.

## Why "collapse + transpile single file"

vs. transpiling each file independently:
- Each Rust submodule references its siblings (`super::Chars`,
  `super::Pattern`) — multi-file emit would need 8 C++20 module
  partitions with cross-imports that form a cycle.
- vec_deque_port and other ports use the same collapse strategy
  successfully (see docs/vec_deque_port/collapse.py for the
  reference implementation).
