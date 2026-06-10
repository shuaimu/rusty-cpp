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

### A. Per-variant trimmed template params (recommended — partial)

Emit `Either_Left<L>` / `Either_Right<R>`, and `using Either<L, R> = std::variant<Either_Left<L>, Either_Right<R>>`. CTAD works for the variant tag — but **not for the type alias `Either<L, R>`** when only one variant is supplied in a brace-init.

The failing case is the ternary:
```cpp
auto reader = (use_empty
    ? Either{Either_Left{cursor1}}      // deduces L only
    : Either{Either_Right{cursor2}});   // deduces R only
```

Even with trimmed variant params, `Either{Either_Left<L>{val}}` can deduce L but not R. C++20's alias-CTAD doesn't fill in the missing parameter — both branches need the same `Either<L, R>` type, and neither branch alone determines both.

Approach A solves the *struct CTAD* problem (no more "no viable constructor or deduction guide of `Either_Left`") but is **not sufficient by itself** for the ternary case. Either also need one of:

- A1. Transpiler emits `either::Left<L, R>(value)` / `either::Right<L, R>(value)` free-function calls (where L and R are inferred from both branches at the transpiler level, not C++'s CTAD).
- A2. Transpiler wraps each ternary arm in a lambda with explicit return type (`[&] -> Either<L, R> { return ...; }()`) when it detects a unifying Either type from sibling branches.

Both require the transpiler to (a) detect ternary/if-else with Either-typed branches and (b) infer the unified `<L, R>` from the union of arm types. Substantial design work.

Touches the data-enum emission path; must also update:
- All internal accesses (`std::get<0>(_m)._0`, etc.) to match the new variant types.
- `Either_Left<L, R>{value}` call sites the transpiler emits elsewhere → `Either_Left<L>{value}`.

Affects every data enum, not just Either — but most enums (Option, Result, ...) already have their CTAD shapes covered by the `rusty::` runtime types, so approach A's blast radius is mostly limited to Either and any user-defined enum that exhibits the ternary-unify pattern.

**Status:** deferred. The win is real but the design needs to handle both per-variant trimming AND the ternary unification step. Item 4 (serde correctness) is now lower-risk and higher-value; recommend tackling 4 first.

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

**Status:** partially scoped, not committed. The naïve fix (qualify with the trait's declared namespace via `trait_declared_path_by_short_name`) makes `PoolIndexTraits` resolve, but exposes a deeper issue:

The **forward decl** of `combinations_with_replacement<I>(I)` and its body declaration emit DIFFERENT requires clauses:
- Forward decl: `requires (std::copyable<typename I::Item>)` (simple `I::Item` form, emitted before any trait has been processed; `trait_associated_type_names` is empty → fallback to the simple form)
- Body: `requires (std::copyable<typename combinations::PoolIndexTraits<I>::Item>)` (helper form, emitted after `PoolIndex` trait has registered its assoc-types)

Clang then errors with "requires clause differs in template redeclaration" — both declarations are valid in isolation but their constraints don't match token-for-token.

**Tried** a pre-pass that populates `trait_associated_type_names` before forward-decl emission. That made both sides use the helper form for *every* trait with assoc types — but only some traits (e.g. `PoolIndex`) actually get a `<Trait>Traits<U>` helper specialization emitted. Others like `FuncLR` and `IteratorIndex` produced "no type named 'FuncLRTraits' in namespace 'merge_join'" because the helper never exists for them.

**Real fix:** make the dependent-type substitution at `mod.rs:32024` conditional on whether the trait will actually receive a `<Trait>Traits<U>` specialization. That requires either:
- A pre-pass that walks impl blocks and decides which traits get helpers.
- Suppressing the helper substitution specifically inside requires-clause emission (use the simple `typename I::Item` form for constraints, and the helper form only at use sites where the type is actually needed).

The second approach is cleaner but requires plumbing an "in-requires-clause" flag through `map_type`. Deferred until either the matrix or another consumer makes the work clearly worth the design churn.

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

- ~~`std::collections` not mapped to `rusty::collections`~~ ✅ done (commit f5f4361 added BinaryHeap/LinkedList rows mirroring BTreeMap/BTreeSet/HashMap/HashSet/VecDeque).
- Virtual `override` on non-virtual base methods — **deep issue**, deferred.
  Concretely: `class Serializer { virtual bool is_human_readable() const; }` is the base, then the Adapter spec inherits as `class SerializerAdapter<…> final : public Serializer<…> { rusty::fmt::Result serialize_u8(uint8_t v) override { … } … };`.
  - Base class is emitted as `class Serializer` (non-templated, no `serialize_*` methods — all are commented `// TODO(interface_traits): by-value 'self' method '...' not yet supported`).
  - Adapter spec inherits from `Serializer<…>` (templated form — clang seems to accept this despite no template declaration; possibly substituted away through an import?) and provides `override` for the by-value `self` methods.
  - The local-impl (`emit_one_local_adapter_method`, mod.rs:11219) and foreign-impl (`emit_one_foreign_adapter_method`, mod.rs:11067) paths both SKIP by-value self methods. So a THIRD emit path is producing the Adapter spec's overrides. Need to find and align it with the base's TODO-skip behavior.
  - Estimated 0.5–1 day to track down the third emit path and skip by-value-self methods consistently. Item 4 is partially done; the remaining `override` fix is queued.
- `SerializeMap` typedef redefinition (same shape as the `IntoEither` fix just landed — likely the helper-alias path needs the same gate widened, or it's a separate trait).
- `if (_m == Unit)` for **data-enum** unit variant arms — `Unit` is emitted as a static factory `static Unexpected Unit() { … }`, so `_m == Unit` becomes `Unexpected == Unexpected ()` and clang errors with "invalid operands to binary expression". Attempted fix: emit `rusty::detail::variant_holds<Foo_Unit>(_m)` instead, looking up the scoped key from `data_enum_unit_variants`. That works for the type name but the registry stores the *Rust* scope path (e.g. `__private::content::Content_Unit`) where the emitted C++ namespace is `__private228::content::*` and `private` is a C++ keyword. Needs a proper mapping from registry path to emitted scope. Deferred.
- `no member named 'from_vec' in 'std::basic_string<char>'` — string literal/vec construction mismatch in serde_core; separate codegen issue.

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
| 2a | ~~itertools `tee` ↔ POSIX `tee()` collision~~ | ✅ done (mod rename) | ~5 min | itertools' first build blocker |
| 2b | itertools `PoolIndexTraits` qualification | ⏳ partial — needs design (see §2b) | ~1 day | second build blocker on itertools |
| 3 | either approach A | ⏳ deferred — see §A note (insufficient by itself) | ~1.5 day | either + future data-enum cases |
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
