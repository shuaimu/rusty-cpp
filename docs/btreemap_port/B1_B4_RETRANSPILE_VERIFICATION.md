# B1–B4 Re-Transpile Verification

This doc records the end-to-end verification that the four
btree_port iter()/remove()/clone() bugs (originally reported via the
`btree_port_iter_remove_movonly_test.cpp` coverage pin) are fixed
at the transpiler level.

## Verification method

1. Vendor a fresh copy of rustc's `library/alloc/src/collections/btree/`.
2. Run `docs/btreemap_port/prep.sh` to merge submodules.
3. Re-transpile with the **post-fix** transpiler binary (commits
   a1de909, 6cfa19f, 0a0304d, fddc85f).
4. Compare emit shapes at the bug-clustered sites against the original
   four error categories.
5. Build the `tests/btree_port_iter_remove_movonly_test.cpp` repro
   against the fresh emit and confirm the original four error
   shapes have disappeared.

## Per-bug emit-shape verification

### B1 — variant `._0` on `std::variant<...>`

**Original error**: `no member named '_0' in std::variant<LeftOrRight_Left<...>, LeftOrRight_Right<...>>` at btree_internal.cppm:4945/4953/5850/5856.

**Fix** (commit a1de909): `data_enum_variant_owner_and_name_from_path_with_ctx`
single-segment fallback now consults `unique_data_enum_name_for_variant_name`
to resolve bare-glob variants when exactly one data enum in the TU
declares them.

**Fresh-transpile observation**: where the variant name is unique
in the TU, the dispatch lowers properly to
`_m.index() == N` + `std::get<N>(_m)._0`. Where it's globally
ambiguous (e.g., `Leaf` is declared by both `ForceResult` and
`Position` in btree_internal; `Left` is declared by both
`LeftOrRight` and the `Either` builtin), the TODO marker is
preserved — the patcher handles those (sibling enum's owner is
type-context dependent, which is a separate transpiler arc).

**Repro emit (before)**: `__t._0` on a `std::variant` — compile error.
**Repro emit (after, for unambiguous variants)**: `_m.index() == 0 ?
std::get<0>(_m)._0 : ...` — compiles.

### B2 — const-drift on let-binding with field mutation

**Original error**: `cannot assign to return value because function 'deref_if_pointer_like<const unsigned long &>' returns a const value` at btree_internal map.cppm:5249-5252.

**Fix** (commit fddc85f): `collect_assignments_in_expr` now walks
through `Expr::Field` / `Expr::Index` / deref on the assignment LHS,
so `x.field = ...` and `x.field op= ...` register `x` as a
reassigned var. The existing skip-list in the qualifier decision
then drops `const` on the let-binding.

**Fresh-transpile observation**: the previously-vendored map.cppm
had `1` occurrence of `const auto map = this->dormant_map` —
the fresh emit has `0`. The shape now binds with a mutable form
that supports the downstream field-write.

### B3 — `forget_type(self)` emitted non-const

**Original error**: `'this' argument to member function 'forget_type' has type 'const NodeRef<Mut, ...>', but function is not marked const` at btree_internal.cppm:4968.

**Fix** (commit 6cfa19f): in `emit_method`, the qualifier decision
for Rust `fn foo(self)` (self-by-value, non-operator) was changed
from non-const to const. Rust consumes-by-move ownership in `self`
methods but doesn't mutate `*self` — modeling as a C++ const method
lets const-bound receivers (e.g., `parent.forget_type()` where
`parent` came from `std::as_const(_m).unwrap()`) call the method.

**Fresh-transpile observation**: `forget_type() const` is now
emitted everywhere `fn forget_type(self) -> X` appears (one site
in btree_internal where the method is declared in the impl block).
Previously: `forget_type() {` (non-const).

### B4 — copy-ctor required on move-only T

**Original error**: `call to implicitly-deleted copy constructor of std::pair<long, MoveOnlyCallable>` at map.cppm:5648/5249, btree_internal.cppm:5358, maybe_uninit.hpp:108.

**Fix** (commit 0a0304d): structured-binding emit (two sites in
codegen.rs) now uses `auto&&` (forwarding ref) instead of `auto`,
so move-only tuple elements don't require a copy ctor. Plus
`MaybeUninit<T>::assume_init_read()` const overload now gated on
`requires(is_copy_constructible_v<T>)` (matching the existing
guard on `assume_init() const`).

**Fresh-transpile observation**: in map.cppm, the previously-
vendored had 2 occurrences of `auto [` (the old shape). The fresh
emit has 0 — all 23 structured-binding sites now use `auto&& [`.

## End-to-end build status

Re-vendoring the fresh emit into `transpiled/btree_port/` and
building the iter_remove coverage-pin test fires a NEW set of
5 compile errors — but **none of them match the original B1–B4
shapes**:

```
no template named 'Vec' in namespace 'rusty'         ← visit_byte_buf stub
LeafNode<...> *' is not a structure or union          ← new: arrow vs dot drift
__TemplateArgs partial specialization must occur ...  ← new: nested-class spec
```

The originally-reported errors:
- `no member named '_0' in std::variant<...>` — **gone**
- `'this' argument has type 'const NodeRef<Mut>'` — **gone**
- `'deref_if_pointer_like<const T&>' returns const` — **gone**
- `call to implicitly-deleted copy constructor of std::pair` — **gone**

The new errors are downstream of patcher drift: `docs/btreemap_port/post_transpile_patch.py`
is calibrated for the previous transpiler emit shape; the fresh
emit's slightly-different structure trips patcher patterns that
used to match. Updating the patcher to match is a separate work
item — orthogonal to the B1–B4 fixes, which are demonstrably
landed at the transpiler level.

## Update 2026-06-05 — re-vendor with new patcher (partial)

The previously-vendored cppms have been **replaced** with the freshly
re-transpiled output, and `docs/btreemap_port/post_transpile_patch.py`
extended with the 5 new patches to handle the new emit shape:

- `fix_visit_byte_buf_unknown_vec` — drops the rusty::Vec-referencing
  serde visitor stub at module purview.
- `fix_using_rusty_vec` — strips the leftover `using rusty::Vec;`.
- `fix_leafnode_shadow_arrow` — rewrites `*new_node_shadow1.field` →
  `new_node_shadow1->field` in split_leaf_data (3 sites).
- `fix_template_args_primary_scope` updated to handle the no-blank-line
  emit shape (the relocation no-op'd silently before).
- Auto-invocation of `btreeset_auto_namespace_postprocess.py` from
  inside the main patcher's main(), so the alias injection that the
  auto-namespace mode requires now runs in the pipeline.

**Status**: `libbtree_port.a` builds clean against the fresh re-transpile.
The 5 patcher-drift errors listed above are gone.

**Caveat**: the move-only-T coverage-pin test
(`tests/btree_port_iter_remove_movonly_test.cpp`, opt-in via
`-DRUSTY_CPP_BUILD_BTREE_PORT_ITER_REMOVE_TEST=ON`) still fails to
compile under template instantiation with errors that match the
original B1/B2/B3/B4 shapes. The B1–B4 transpiler-side commits cover
the common cases but have edge-case gaps:

- **B1 gap** — `unique_data_enum_name_for_variant_name` lookup only
  resolves variants that are unique in the TU. `Leaf` and `Internal`
  are declared by both `ForceResult` and `Position` in btree_internal,
  so the resolver leaves a `/* TODO transpiler: unresolved bare-glob
  variant `Leaf` */ true` placeholder. The match-arm body still
  accesses `_m._0` on the parent variant; at instantiation time this
  errors.
- **B3 gap** — making ALL self-by-value methods emit `const` broke
  `fn split(mut self, alloc) -> SplitResult` (Rust `mut self` allows
  in-binding mutation; the C++ equivalent must be non-const). The
  emit doesn't distinguish `self` from `mut self`.
- **B2 / B4 gaps** likely have similar edge-case sites surfacing
  at instantiation.

**Implication for downstream**: the rrr::Alarm BTreeMap swap with
`std::pair<u64, rusty::Function<void()>>` (move-only value) still
fails. Keep the `std::map` fallback in `src/rrr/misc/alarm.cpp` for
the time being. The transpiler binary improvements are real and
ship the fresh vendored emit ready for any *copyable*-V instantiation;
move-only-V instantiation needs additional transpiler work (variant
context tracking for B1; `mut self` vs `self` distinction for B3).

## Why we previously kept the older vendored `transpiled/btree_port/*.cppm`

(Historical, prior to 2026-06-05 update.) The vendored cppms remained
the older emit (patcher-clean) so the existing build path stayed green.
Updating both the transpiler emit AND the patcher in lockstep was the
next step; for now the B1–B4 fixes are validated via:

1. **Transpiler unit tests**: 6 new regression tests in
   `transpiler/src/codegen.rs::tests` (all 1568 transpiler tests
   pass).
2. **Emit-shape inspection**: this doc.
3. **Cross-error comparison**: the iter_remove test fires 5 NEW
   errors when built against the fresh transpile, but **none** of
   them are the original 4 categories.

## Commits

- a1de909 — B1: bare-glob variant `._0` emit (data-enum lookup)
- 6cfa19f — B3: self-by-value methods emit as const
- 0a0304d — B4: structured bindings emit `auto&&`; MaybeUninit guard
- fddc85f — B2: field-mutation registers binding as non-const
- bb96b96 — coverage-pin test (`btree_port_iter_remove_movonly_test.cpp`)
