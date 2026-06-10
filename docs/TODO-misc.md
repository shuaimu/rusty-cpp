# TODO: Plan for Remaining Parity-Matrix Failures

Status at time of writing: **8 / 15 PASS** — arrayvec, bitflags, cfg-if, once_cell, semver, smallvec, take_mut, tap.

Remaining failures: `either`, `itertools`, `serde_bytes`, `serde_repr`, `serde_core`, `serde`, `pollster`.

This document captures the per-crate failure analysis, fix proposals, and recommended ordering. Each item is sized in rough-day units; treat as a working plan rather than a contract.

---

## 1. `either` — Either_Left / Either_Right CTAD failure

**Symptom**

`Either{Either_Left{value}}` fails to compile — `Either_Left<L, R>` has two template params, but CTAD with one argument can't deduce `R`. Surfaced in the `seek` test's ternary that returns either branch:

```cpp
auto reader = (use_empty
    ? Either{Either_Left{cursor_empty}}
    : Either{Either_Right{cursor_slice}});
// error: no viable constructor or deduction guide for deduction of
// template arguments of 'Either_Left' / 'Either_Right'
```

**Root cause**

Data-enum variant tags are emitted as `template<typename L, typename R> struct Either_Left { L _0; }` — every variant carries *all* enum type params for symmetry with `using Either<L, R> = std::variant<Either_Left<L, R>, Either_Right<L, R>>`. The "R" position on `Either_Left` is unused (the variant only holds an L), but it's still part of the type and clang needs it to instantiate.

**Three possible fixes** (ordered by cleanness):

### A. Per-variant trimmed template params (recommended)

Emit `Either_Left<L>` / `Either_Right<R>`, and `using Either<L, R> = std::variant<Either_Left<L>, Either_Right<R>>`. CTAD works directly.

Touches the data-enum emission path in the transpiler; must also update:
- All internal accesses (`std::get<0>(_m)._0`, etc.) to match the new variant types.
- `Either_Left<L, R>{value}` call sites the transpiler emits elsewhere → `Either_Left<L>{value}`.

Affects every data enum, not just Either. Need to verify on the bitflags / arrayvec / once_cell crates that don't use trimmed args today.

### B. Conversion-proxy wrapper

Add a single-template intermediate `Either_LeftCtor<L>` that implicitly converts to `Either_Left<L, R>` for any R. Either_Left's existing 2-param shape stays. Emit `Either_LeftCtor{value}` instead of `Either_Left{value}` at variant-constructor call sites.

Easier backward compat but adds boilerplate and a transpiler emission branch.

### C. Explicit type emission (fragile)

Detect the ternary-into-Either pattern, unify L from one branch and R from the other, emit `Either_Left<L, R>{value}`. Doesn't help non-ternary patterns where the type is fully implicit.

**Recommendation:** **A.** Mirrors the Rust convention (each variant only carries its own type), removes the CTAD trap for all future data enums. Budget ~half a day to get the emission right and verify the rest of the matrix stays green.

---

## 2. `itertools` — three blockers

After the `IntoEither` typedef-redefinition fix unblocked the import chain, three real issues surface in `itertools.cppm`:

### 2a. `tee` redefinition as different kind of symbol (~line 10691)

The emitted file declares `namespace tee {}` **four times** (lines 3703, 4126, 10691, 15206). The latter two re-declare contents — there's likely a `tee` function and a `tee` namespace conflicting. Cargo-expand inlining of macros may double-emit the body.

Likely the same shape that motivated the namespace-emission dedup work in earlier commits; need to extend the dedup pass to namespace bodies with substantive content (not just empty forward decls).

### 2b. `PoolIndexTraits` not in scope (~lines 9631, 10383, 10787, 10790)

```cpp
requires (std::copyable<typename PoolIndexTraits<I>::Item>)
// error: no template named 'PoolIndexTraits'; did you mean 'combinations::PoolIndexTraits'?
```

The transpiler dropped the namespace qualifier when emitting the `requires` clause type path. Small fix in the requires-clause type-path emission — qualification likely isn't running for types inside trait bounds.

### 2c. `make_entry_probe` missing from `rusty::detail` (~lines 11072, 11093)

Itertools' `unique_impl` reaches for a helper that doesn't exist in `slice.hpp`. Either:
- Implement the helper (mirror of Rust's `Iterator::collect` into a probe-style entry map).
- Stub it for the specific usage.

Shape needs reading the Rust source to know what behavior to mimic.

**Recommendation:** 2a + 2b are tractable transpiler fixes; 2c is port-runtime work. Budget ~1 day total.

---

## 3. `serde_bytes` / `serde_repr` / `serde_core` / `serde` — Stage C transpile blowup

**Symptom**

`serde_core`'s expanded source is 38K Rust lines and produces ~14K cppm lines.

### Track P (perf) — ✅ RESOLVED via matrix-script `--release`

Root cause was **not** a transpiler-internal quadratic — it was the matrix script invoking `cargo run -p` (default *debug* profile). Measured times for `serde_repr --stop-after transpile`:

- Debug:   14m28s
- Release:  5m24s  (~2.7× faster)

The 1800s matrix timeout couldn't fit debug-mode transpile + Stage D builds, so the script killed serde-family crates before they reached compilation. Earlier `[dedup-trace]` and per-walk memoization suspicions were red herrings — debug-mode is just uniformly slow on the multi-megabyte cppm output paths.

**Fix:** change `tests/transpile_tests/run_parity_matrix.sh` to invoke `cargo run --release -p rusty-cpp-transpiler`. `serde_bytes` now reaches Stage D in 3m22s (used to time out at 30min).

Verified no regression on cfg-if / once_cell / take_mut. See commit *(this commit)*.

### Track C (correctness) — known build-time errors (now reachable)

From the post-Track-P serde_bytes build.log:

- `std::collections` not mapped to `rusty::collections` (a few entries to add in the std-mapping table). Lines: 4339, 4348, 12001, 12019 of `serde_core.cppm`.
- Virtual `override` on non-virtual base methods (trait-class emission emits `virtual` only on some methods, but inheriting traits emit `override` on all). Lines: 12847, 12886, 12889 of `serde_core.cppm`.
- `SerializeMap` typedef redefinition (same shape as the `IntoEither` fix just landed — likely the helper-alias path needs the same gate widened, or it's a separate trait).

**Recommendation:** Now that Track P unblocks iteration, Track C is the active workstream. The `std::collections` mapping is the smallest fix and likely cascades into others; do it first.

---

## 4. `pollster` — no `std::task` analog

**Symptom**

Pollster is a minimal executor for `Future`. Requires `std::task::Waker`, `std::task::Context`, `std::task::Poll`. Rust's async model is library-level, not compiler-mandated, but the trait surface still needs C++ representation.

**Path forward**

Mock minimal `std::task` types in `include/rusty/task.hpp`:

- `Waker` = type-erased wrapper around a `void(*)(void*)` + `void* data` (matches RawWaker's vtable).
- `Context<'_>` = wrapper around `Waker&`.
- `Poll<T>` = `std::variant<Pending, Ready<T>>`.

Map `std::task::*` paths to `rusty::task::*` in the std-mapping table.

This wouldn't run a real executor (we'd still need `Future::poll` machinery), but it might let pollster compile. The actual `block_on` is straightforward (loop + park).

**Recommendation:** lowest priority. Budget ~1 day if attempted; consider deferring as out-of-scope for the matrix milestone.

---

## Recommended ordering

| # | Item | Status | Cost | Unlocks |
|---|---|---|---|---|
| 1 | ~~Perf track for serde_core~~ | ✅ done (matrix `--release`) | ~0.5 day | 4 crates from "can't test" → "can test" |
| 2 | itertools 2a + 2b | ⏳ next | ~0.5 day | itertools build progress |
| 3 | either approach A | ⏳ | ~0.5 day | either + future data-enum cases |
| 4 | serde-family correctness | ⏳ (now reachable) | ~1 day | serde_bytes likely passes; serde_repr/serde_core/serde may pass or surface new issues |
| 5 | itertools 2c | ⏳ | variable | itertools build complete |
| 6 | pollster | ⏳ optional | ~1 day | pollster build (no runtime guarantee) |

**Expected outcome if 1–4 land:** ~11–12 / 15. either + itertools at least build; serde_bytes likely passes; the rest of serde-family is uncertain.

**Validation gate at each step:** run the 8 currently-passing crates to confirm no regression before moving to the next item.

---

## Notes & risks

- **Either fix (A) touches every data-enum emission.** Highest regression risk in this plan. Have a tight verification loop on bitflags / once_cell / take_mut after each emission-path change.
- **serde perf work may not be the right framing.** If the transpiler is genuinely O(n²) on something, the right fix is structural, not micro-optimization. **Profile first**, don't guess.
- **The `make_entry_probe` gap** in itertools suggests there may be other port-runtime gaps lurking. Budget some slack in item 5.
- **Each commit should keep the currently-green 8 crates green.** The matrix is the regression test.
