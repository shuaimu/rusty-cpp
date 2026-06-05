# core_slice_port — Phase A1 infrastructure landed

`library/core/src/slice/` (9 top-level files + 2 subdirs, 15421 LOC
total) — collapse + prep infrastructure landed; ~138 rustc resolution
errors remain.

## Pipeline

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ rustup core/src/slice/ + nested iter/macros.rs |
| 2. Prep | ✅ Drops `sort/`, inlines `iter/macros.rs` into iter.rs head, strips 6 rustc internal attrs, `impl const` → `impl`, `default impl` → `impl` |
| 3. Collapse | ✅ Reuses core_str_port collapse with inner-doc-comment stripping (`//!` mid-file) |
| 4. Transpile | 🔴 138 rustc resolution errors block syn parse |

## Remaining error categories (138 total)

| Count | Category |
|---:|---|
| 13 | `cannot find module/crate sort/rotate/simd/intrinsics` |
| 11 | `specialization is unstable` |
| 6 | `cannot find intrinsics in crate` |
| 4 | `slice_get_unchecked` not found |
| 2 | `iter_next_chunk` is private |
| 2 | `align_offset` is private |
| ~100 | misc resolution + const-eval restrictions |

## Strategy

The slice port is large but has cleaner content than core_str — fewer
rustc-internal macros, fewer trait-machinery hot spots. Phase A2
patcher iteration could likely converge in 3–7 days vs core_str's
estimated 1–2 weeks.

But the unblock value is asymmetric:
- core_str_port full body needs core_slice_port's `slice::index::*` — but
  only ~3 SliceIndex impls. The simpler ports of just those impls would
  unblock str in a fraction of the slice port effort.
- Better strategy: extract just the SliceIndex trait + Range/RangeFrom/etc.
  impls as a focused `core_slice_index_port` (~1075 LOC), skip the rest
  of slice. Much smaller arc to unblock str.

## Files

- `Cargo.toml.template` — minimal lib crate manifest
- `prep.sh` — extends core_str_port prep with: drop `pub mod sort;`,
  inline `iter/macros.rs` into iter.rs head, `impl const` →
  `impl`, `default impl` → `impl`, strip 6 more rustc_* attrs
- `collapse.py` — reuses core_str_port collapse, adds inner-doc-comment
  (`//!`) mid-file stripping
