# hashbrown port — scope assessment

The HashMap port (docs/hashmap_port/) is blocked on the `hashbrown`
crate (which std::HashMap wraps). This directory will hold the
scaffolding when work begins.

## Scope (assessed, not vendored yet)

Source location: `~/.cargo/registry/src/index.crates.io-*/hashbrown-0.17.0/`

| File | Lines | Role |
|------|-------|------|
| `src/raw.rs` | 4630 | Core SwissTable raw table |
| `src/raw_entry.rs` | 1729 | Raw entry API |
| `src/map.rs` | (TBD) | HashMap wrapper |
| `src/set.rs` | (TBD) | HashSet wrapper |
| `src/control/group/{sse2,neon,lsx,generic}.rs` | ~600 | SIMD probe groups |
| `src/control/{tag.rs,bitmask.rs,mod.rs}` | ~200 | Control-byte machinery |
| `src/external_trait_impls/{serde,rayon}/*` | ~2100 | Skip for port |
| **Total (excl. external_trait_impls)** | **~20,000** | |

## Reasonable port budget

- **Faithful transpile**: 2 weeks. Same playbook as BTreeMap +
  Vec, but more files and SIMD-specific paths that need C++
  equivalents (the analyzer doesn't speak immintrin.h natively
  enough to handle hashbrown's `_mm_movemask_epi8` etc.).
- **Pragmatic skip**: use `rusty::hashmap` as it stands (hand-written,
  uses std::unordered_map internally per `include/rusty/hashmap.hpp`).
  Trades the SwissTable perf win for keeping the port out of scope.

## Why pause at scoping

A 2-week port doesn't fit a tail-end session. Worth doing only if:
1. A real SwissTable in C++ is the value prop (which it is — std::unordered_map is 2-5x slower on most workloads).
2. The transpiler is improved first to handle SIMD intrinsics +
   `core::arch::x86_64` paths cleanly (currently no support).

## What to do first when work resumes

1. Vendor `hashbrown-0.17.0/src/control/group/generic.rs` (the
   non-SIMD fallback, 152 lines). Transpile alone. See the
   minimum-viable cluster.
2. From there, decide: full vendor with SIMD vs generic-only.
3. Add `prep.sh` analogous to docs/vec_port/.

This file documents the entry point. The companion
`docs/hashmap_port/STATUS.md` cross-references this.
