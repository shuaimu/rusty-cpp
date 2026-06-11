# TODO: Plan for Remaining Parity-Matrix Failures

Status (as of 2026-06-11): **14 / 15 PASS** — arrayvec, bitflags, cfg-if, **either** (flipped, commit bb255fa), once_cell, pollster, semver, **serde** (flipped, commit 1ff08d7), **serde_bytes** (flipped, commit 064a6cc), serde_core, serde_repr, smallvec, take_mut, tap.

Remaining failure: **`itertools`** only.

This document captures the per-crate failure analysis, fix proposals, and recommended ordering. Each item is sized in rough-day units; treat as a working plan rather than a contract.

## Session log (newest first)

- **2026-06-11**: Matrix advanced 13/15 → 14/15. serde_bytes flipped via crate-namespacing wrap (Phase 1, narrow — commit 064a6cc). itertools advanced 5 unique errors → 1 via four converging fixes: by-value `auto` for non-reference params, supertrait inheritance skip when parent has assoc types, `<Trait>Traits` helper qualification with home namespace, `ArrayRepeatResult` template `operator=`.
- **2026-06-10**: Matrix advanced 11/15 → 13/15. either + serde flipped via inference engine end-to-end (Phases 1-5) + emit-site wiring + `value.into()` slice routing + `Vec::from(span)`.

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
- ~~Virtual `override` on non-virtual base methods~~ ✅ done (commit ffc3eea added `trait_class_skipped_method_keys` registry, populated when the trait class emits a TODO-skipped by-value `self` method; Adapter emitters consult it and skip the same methods).
- ~~`if (_m == Unit)` for data-enum unit variant arms~~ ✅ done (commit 2cc52b6 — use `variant_ctx.enum_name` to construct the variant tag and emit `rusty::detail::variant_holds<Foo_Unit>(_m)`; tag is in scope inside impl blocks so no qualification needed).
- ~~`no member named 'from_vec'` on OsString~~ ✅ done (commit f0cc6a4 — added `rusty::ffi::os_string_from_vec` template helper + mapping in types.rs).
- ~~Adapter storage uses `Formatter& value_`~~ ✅ done (commit 856c190 — strip trailing `&` from self_cpp before applying kind-specific qualifier).
- ~~`rusty::Weak` no longer exported~~ ✅ done (commit 6c27379 — route RcWeak alias / `use Weak` import / arity table all through `rusty::rc::Weak`).
- **NEW BLOCKER (post-fix-chain):** `std::span<const unsigned char>` / `std::vector<unsigned char>` → `rusty::Vec<uint8_t>` conversion fails. Surfaces in serde / serde_bytes at `Content_Bytes{rusty::to_owned(value)}` and similar.
  - **Partial fix landed (commit de9d340):** `try_emit_to_owned_method_call` and the `to_vec` arm now detect `&[T]` / `[T]` receivers and emit `rusty::Vec<T>::from_iter(value)` instead of the runtime helpers. Line 7879 (the `Content_Bytes{rusty::to_owned(value)}` site) is now `rusty::Vec<uint8_t>::from_iter(value)` and compiles.
  - **Still failing at line 8607:** `Content::ByteBuf(rusty::to_vec(value))` emits the function-form `rusty::to_vec(value)` even after the fix because `infer_simple_expr_type(&mc.receiver)` returns None at that emission site — the inference visitor doesn't reach the `value` parameter through whatever context wraps the call (likely a generic adapter method that the transpiler can't see typed). A stronger type-flow pass would catch it, or a target-driven heuristic (`if expected type is rusty::Vec<T>, emit Vec<T>::from_iter`) would too. Estimated ~half day; queued.
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
| 1 | ~~Perf track for serde_core~~ | ✅ done (matrix `--release`) | — | — |
| 2a | ~~itertools `tee` ↔ POSIX `tee()` collision~~ | ✅ done (mod rename) | — | — |
| 2b-i | ~~itertools `PoolIndexTraits` qualification~~ | ✅ done (commit 2f9c570 — qualified emit at `mod.rs:32228` via `trait_declared_path_by_short_name`) | — | 6 "no template named" errors cleared |
| 2b-ii | itertools requires-clause cross-pass alignment | ⏳ blocked — needs trait-uniqueness-by-use-context logic | ~2 days | last itertools error type clears |
| 2c | ~~itertools `make_entry_probe`~~ | ✅ no longer surfacing | — | — |
| 2d | ~~itertools const-binding mutability~~ | ✅ done (commit bb3887d — by-value Rust params lower as `auto`) | — | itertools `sub_scalar` compiles |
| 2e | ~~itertools supertrait assoc-types inheritance~~ | ✅ done (commit 49ccd98 — supertrait skipped when parent has assoc types) | — | `HomogeneousTuple: TupleCollect` compiles |
| 2f | ~~itertools `ArrayRepeatResult = Vec` assignment~~ | ✅ done (commit eb606e8 — templated `operator=` on any iterable) | — | itertools `test_checked_binomial` compiles |
| 3 | ~~either approach A — emit-side wiring~~ | ✅ done (commit bb255fa — explicit template args on variant ctors via inference engine output) | — | either flips |
| 4 | ~~serde-family correctness~~ | ✅ done (commit 1ff08d7 — slice routing + `Vec::from(span)` + `into_boxed_slice`) | — | serde flips |
| 5 | ~~serde_bytes namespacing~~ | ✅ done (commit 064a6cc — Phase 1 narrow wrap) | — | serde_bytes flips |
| 6 | Universal crate namespacing (Phase 2) | ⏳ optional — Phase 1 sufficient for current matrix | ~1.5 days | future cross-crate trait collisions |
| 7 | ~~pollster~~ | ✅ passes (was never failing — earlier session) | — | — |

**Achieved outcome:** 5 → 14 / 15 across two sessions. Only itertools' requires-clause-differs cross-pass alignment remains as a known blocker.

**Validation gate at each step:** run the 14 currently-passing crates to confirm no regression before moving to the next item.

## §2b-ii: requires-clause cross-pass alignment (remaining work)

**Symptom**

itertools' `CombinationsWithReplacement<I>` has two forward declarations and one struct body:

```cpp
// pre-pass forward decl (cppm:4231):
export template<typename I>
    requires (std::copyable<typename I::Item>)
struct CombinationsWithReplacement;

// final-pass forward decl + struct body (cppm:9633, 9657):
export template<typename I>
    requires (std::copyable<typename ::combinations::PoolIndexTraits<I>::Item>)
struct CombinationsWithReplacement;
```

C++ rejects: `requires clause differs in template redeclaration`.

**Root cause**

`lookup_unique_trait_for_assoc_name("Item")` returns:
- During pre-pass forward decl: `None` (the trait `PoolIndex` isn't yet registered in `trait_associated_type_names`).
- During final pass: `Some("PoolIndex")` (the trait's body emit registered it before the requires clause is mapped).

The pre-pass therefore falls through to bare `typename I::Item`, while the final pass uses the qualified `typename ::combinations::PoolIndexTraits<I>::Item` form.

**Why a simple pre-collect doesn't work**

Tried (this session, reverted): a recursive pre-collect that walks `Items` and populates `trait_associated_type_names` for every trait before the forward-decl pre-pass runs. This made the lookup succeed during pre-pass too — both passes produced the qualified form.

**But** it regressed a different itertools site (`adaptors::coalesce::CountItemTraits<C>::CItem`). The pre-collect over-registered traits whose qualified path passes through a segment (`coalesce`) that is *also* a function template name at the use site. Ambient lookup at the use site finds the function first and shadows the namespace, producing `'coalesce' is not a class, namespace, or enumeration`.

The final-pass machinery doesn't hit this bug because it selectively registers traits in a way that avoids these conflicts — exact selection rules not yet traced.

**Path forward (estimated 2 days)**

Option A: Per-use-site qualification check. At each `typename FooTraits<X>::Y` emit, verify that the qualified path resolves uniquely in the use site's lexical scope. Fall back to `typename X::Y` (unqualified) when the qualified form would be shadowed.

Option B: Pre-collect with conflict detection. The pre-pass registers a trait's assoc types only if all segments of its qualified path are exclusively namespaces in the use site's reachable scope (i.e., no function-template or type-alias collisions).

Both require building a per-scope symbol table — fundamentally a name-resolution pass that the transpiler currently doesn't have at this level of fidelity.

**Workaround until then**

Itertools stays as the one matrix failure. The remaining error is genuine cross-pass divergence — not a regression and not a correctness issue with any of the 14 passing crates.

### §2b-ii update (2026-06-11, late evening session)

Implemented the Ch. 14 Phase A + B design (`SymbolCategoryTable` + per-use-site `path_resolves_unambiguously` check) in commits `c10ab0e` and `b3309f4` (rename-aware path qualification). These ARE the correct infrastructure for Option A above; they just don't move itertools' specific failure because the actual flattening + uniqueness behavior of the existing emit pipeline doesn't intersect with where the trait helper paths actually resolve.

Also tried a **template_prefix_lines post-process** that rewrites `typename ::scope::FooTraits<X>::Y` → `typename X::Y` in requires clauses (since both forms are semantically equivalent in a requires clause and the pre-pass produces the unqualified form). The rewrite was syntactically correct — all 6 itertools requires clauses came out as `typename I::Item` — but **clang segfaulted in ASTReader during either.pcm precompile**, well before reaching itertools' own code. The crash reproduces consistently even after a fresh modules-cache wipe; standalone `either` precompile succeeds; only the itertools build context triggers it. This is a clang module-deserialization bug, not a transpiler issue. Reverted the post-process.

**Status going forward:** the requires-clause-differs error remains the only blocker for itertools. Three options to investigate when this becomes priority again:

1. **Clang version pin.** The segfault was on clang-19. Test on clang-20+ or other module-supporting toolchains.
2. **Skip C++20 modules for itertools.** The matrix script's Stage D uses `-x c++-module --precompile`; a fallback to header-include compilation might bypass the ASTReader path but requires significant matrix-runner changes.
3. **Bisect the post-process output.** Identify which specific change in the rewritten cppm triggers the clang crash. Could be a clang-specific quirk in how it deserializes certain template requires-clause shapes.

---

## Notes & risks

- **Either fix (A) touches every data-enum emission.** Highest regression risk in this plan. Have a tight verification loop on bitflags / once_cell / take_mut after each emission-path change.
- **serde perf work may not be the right framing.** If the transpiler is genuinely O(n²) on something, the right fix is structural, not micro-optimization. **Profile first**, don't guess.
- **The `make_entry_probe` gap** in itertools suggests there may be other port-runtime gaps lurking. Budget some slack in item 5.
- **Each commit should keep the currently-green 8 crates green.** The matrix is the regression test.
