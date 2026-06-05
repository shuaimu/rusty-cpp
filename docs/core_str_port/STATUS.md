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

## Current state

After the multi-line use-dedup land + extended prep stripping:
- ✅ All parse-blocking syntax stripped (`#[cold]` on expr, multi-line
  `assert_unsafe_precondition!`, `const_eval_select!`, `impl_fn_for_zst!`,
  `derive_const`, `#![unstable(...)]` multi-line attrs, rustc-internal
  use-paths to `slice::memchr`/`ub_checks`/`intrinsics`).
- ✅ Use-statement dedup parses single + multi-leaf braced uses and
  rewrites multi-leaf imports to drop already-bound names.
- ⚠️ ~661 rustc resolution errors visible (was 1 parse error). Categories:
  - `cannot find X in slice/intrinsics/ub_checks/lang_items` — many rustc
    internal items the rusty surface doesn't replicate
  - `is not yet stable as a const fn` — many const fns reference
    unstable APIs we'd need to either add or de-const
  - Type collisions where collapsed bodies define types that the
    `mod.rs` body already references (e.g. `from_utf8` conflicts)

## Next steps (all multi-day arcs)

**A. Continue patching** — peel rustc errors layer by layer in the
playbook style. Estimate: 1–2 weeks for full compile clean. Most
errors fall into 10–15 categorical patches; iterating until the count
trends to zero.

**B. Hand-port the str surface** (sibling alternative discussed in
docs/string_port/STATUS.md): port just the ~5 fns String actually uses
(`find`, `starts_with`, `contains`, `split`, `replace`) without
the full Pattern trait machinery. ~½ day. Less complete but unblocks
string_port faster.

**C. Slice-port first** — many errors point at `std::slice::index::*`
helpers; if those land via a `core_slice_port` (similar collapse) the
core_str errors drop substantially.

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
