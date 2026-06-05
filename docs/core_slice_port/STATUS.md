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
| 5. Compile | 🟡 Phase A2 — first patcher pass landed, ~17 distinct error categories remain |

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

## Phase A2 status (first compile attempt)

CMake target `core_slice_port` is wired up under
`RUSTY_CPP_BUILD_CORE_SLICE_PORT=ON` opt-in. First patcher pass
(`docs/core_slice_port/post_transpile_patch.py`) handles 9 rewrite
categories:

- `std::ops::Bound` → `rusty::Bound`
- `std::ops::ControlFlow` → `rusty::ops::ControlFlow` (+ header stub)
- `std::convert::` → `rusty::convert::`
- `std::ops::` → `rusty::ops::` (catch-all)
- `std::range::` → `rusty::ops::`
- `std::ptr::` → `rusty::ptr::`
- `std::ascii::` → `rusty::ascii::`
- `size_of<T>()` → `sizeof(T)`
- Strip orphan `import core_slice_port.<submodule>;` lines
- Strip `using std::simd;` / `using std::ub_checks;`

Plus added `ControlFlow` and `IndexRange` stubs to `include/rusty/ops.hpp`.

## Phase A2 iter 2 — additional patches

Iteration 2 of the patcher added:

- Strip orphan `using std::ascii;` / `using std::range;` / `using rusty::ascii::EscapeDefault;`
- `make_slice()` → `as_slice()` (emit-name divergence)
- `::from_raw_parts_mut<` → `from_raw_parts_mut<` (drop leading `::`)
- `(?<![:\w])::rusty_ext::` → `rusty_ext::` (anchored to avoid `::de::rusty_ext::` corruption)
- `usize::repeat_u8(N)` → inline byte-broadcast constant
- visit_byte_buf stub (sibling-port pattern from borrow_port P1)
- `if (rusty::intrinsics::unreachable() && ...)` → `if (false) { #if 0 ... #endif }` (brace-counted wrap so type-check is skipped)
- `/* len!(self) */` → `0`

Plus infra additions:

- `include/rusty/intrinsics.hpp` (assume/likely; unreachable comes from cppm)
- `rusty::ptr::without_provenance{,_mut}` helpers

## Remaining error categories after iter 2

| Count | Category |
|---:|---|
| 6 | `rusty_ext` undeclared (concept/template body uses it before declaration) |
| 3 | `GetDisjointMutError` bare ref (needs namespace qualification) |
| 3 | `rusty::ops::OneSidedRangeBound` missing (header stub needed) |
| 1+ | `iter` / `ascii` undeclared (mid-method refs from collapsed bodies) |
| 1 | `None_t` → tuple<Direction, size_t> conversion |
| Various | Lambda capture, expected '>' — context-dependent |

## Next: Phase A2 iter 3

Add `OneSidedRangeBound` stub to rusty/ops.hpp. Qualify `GetDisjointMutError`
references via namespace prefix patch. Investigate `rusty_ext` ordering
(declaration vs use). Then re-attempt and iterate.

## Files

- `Cargo.toml.template` — minimal lib crate manifest
- `prep.sh` — extends core_str_port prep with: drop `pub mod sort;`,
  inline `iter/macros.rs` into iter.rs head, `impl const` → `impl`,
  `default impl` → `impl`, strip rustc_* attrs, path-prefixed macro
  strip, `pub const [unsafe] trait` strip
- `collapse.py` — reuses core_str_port collapse, adds inner-doc-comment
  (`//!`) mid-file stripping and `self`-in-brace dedup
