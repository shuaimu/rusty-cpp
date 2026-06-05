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

## Remaining error categories after first patcher pass

| Count | Category |
|---:|---|
| 4 | `from_raw_parts_mut` global-ns refs (probably need `rusty::slice::`) |
| 2 | `usize` undeclared (needs `using usize = size_t;` or rewrite) |
| 2 | `make_slice` not on `Iter<T>` / `IterMut<T>` (emit→`as_slice`) |
| 2 | `::rusty_ext::` leading `::` (sibling-port pattern) |
| 2 | `expected expression` |
| 1 | `rusty::Vec` not in namespace (needs vec_port import) |
| 1 | `rusty::ptr::without_provenance_mut` missing |
| 1 | `rusty::ptr::without_provenance` missing |
| 1 | `rusty::ascii` namespace missing (header needs ascii ns) |
| 1 | Residual `std::range` |
| 1 | Residual `std::ascii` |

## Next: Phase A2 continued

Iterate over these categories. Many are simple regex rewrites; some
will need rusty/ header stubs (ascii namespace, ptr helpers). The
40+ iterator!()/forward_iterator!() macro stubs from the slot
manifest are likely the bigger arc once these surface-level
rewrites clear.

## Files

- `Cargo.toml.template` — minimal lib crate manifest
- `prep.sh` — extends core_str_port prep with: drop `pub mod sort;`,
  inline `iter/macros.rs` into iter.rs head, `impl const` → `impl`,
  `default impl` → `impl`, strip rustc_* attrs, path-prefixed macro
  strip, `pub const [unsafe] trait` strip
- `collapse.py` — reuses core_str_port collapse, adds inner-doc-comment
  (`//!`) mid-file stripping and `self`-in-brace dedup
