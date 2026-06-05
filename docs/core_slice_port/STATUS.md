# core_slice_port — Phase A1 closed (transpile clean)

`library/core/src/slice/` (9 top-level files + 2 subdirs, 15421 LOC
source) → `core_slice_port.cppm` (8773 LOC C++20).

## Pipeline

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ rustup core/src/slice/ + nested iter/macros.rs |
| 2. Prep | ✅ Drops `sort/`, inlines `iter/macros.rs` into iter.rs head, strips rustc internal attrs + path-prefixed macros |
| 3. Collapse | ✅ Reuses core_str_port collapse with inner-doc-comment stripping (`//!` mid-file) |
| 4. Transpile | ✅ Zero errors. 60 slot manifest entries (43 todo macros, 16 rust-only assoc-type aliases, 1 todo_tagged trait method) |
| 5. Compile | 🔴 Phase A2 — not yet attempted |

## What unblocked transpile

Two more prep.sh patches landed in the last iteration:

1. **Path-prefixed macro strip** — the previous strip only matched
   bare `assert_unsafe_precondition!(...)`. The slice code calls
   `ub_checks::assert_unsafe_precondition!(...)` which left
   `ub_checks::();` behind. Now the regex eats any `path::path::` prefix.

2. **`pub const unsafe trait` strip** — existing strips covered
   `const trait`, `const unsafe trait`, and `const pub trait` but not
   `pub const trait` / `pub const unsafe trait` (rustc syntax order
   for `SliceIndex`). Added both.

## Slot manifest summary

60 hand-attention markers:
- 43 `todo` — macro invocations the transpiler emits as TODO
  (`spec_fill_int!`, `iterator!`, `forward_iterator!`,
  `impl_slice_contains!`, `always_applicable_ord!`)
- 16 `skipped_rust_only` — unbound-generic assoc-type aliases (mostly
  `Item` in `IntoIterator`/`Iterator` impls)
- 1 `todo_tagged` — `finish` method with unresolved Self/auto in
  signature (interface_traits backlog)

## Next: Phase A2

Try to compile `libcore_slice_port.a` and start peeling errors. Likely
top categories given the slot manifest:

- Iterator-macro stubs that need hand-port (slice::Iter / IterMut /
  Chunks / Windows / Split families)
- Unbound `Item` aliases blocking iterator trait conformance
- Forward declarations that reference symbols from collapsed bodies

## Files

- `Cargo.toml.template` — minimal lib crate manifest
- `prep.sh` — extends core_str_port prep with: drop `pub mod sort;`,
  inline `iter/macros.rs` into iter.rs head, `impl const` → `impl`,
  `default impl` → `impl`, strip rustc_* attrs, path-prefixed macro
  strip, `pub const [unsafe] trait` strip
- `collapse.py` — reuses core_str_port collapse, adds inner-doc-comment
  (`//!`) mid-file stripping and `self`-in-brace dedup
