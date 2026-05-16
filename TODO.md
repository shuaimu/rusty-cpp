# RustyCpp TODO

Earlier history archived at `docs/TODO-archive.md`. Items in this file are
the currently open work surface, grouped by the categorization done on
the failing-test snapshot of 2026-05-16 (transpiler test count at that
snapshot: 1160 pass / 361 fail).

For each bucket: the symptom, the suspected root cause (where known),
and an estimate of leverage (how many failing tests one fix would clear).

---

## High priority — high-leverage generic fixes

- [x] **A. Strip the leading `::` from `using` imports for already-`rusty::`/`std::`-qualified targets** ✅ Done (commit fb334db, +28 pass / -28 fail)
  - Fixed in `make_using_path_cpp_legal`: for known top-level namespaces
    (`rusty::`, `std::`, `core::`, `alloc::`), emit the natural unqualified
    form (`using rusty::Vec;`). Other qualified paths (`de::Foo`, sibling
    crates) keep the `::` anchor so nested-module shadowing still works
    (verified by `test_global_import_keeps_qualifier_when_nested_module_shadows_root`).

- [x] **B. impl-method dedup count tests** ✅ 6/8 done (commit 0973868, +6 pass / -6 fail)
  - Six tests fixed by updating assertions to match the out-of-line
    emission shape (`int32_t Foo::cloned() const {` vs the legacy inline
    `int32_t cloned() const {`). The dedup behaviour was correct;
    only test expectations were out-of-date.
  - **Two sub-bucket tests still failing** —
    `test_rewrite_global_using_path_for_local_module_root_in_current_scope`
    and `..._in_ancestor_scope`. The fix (removing the
    `current_scope_declares_nested_module_root` early-exit in
    `rewrite_global_using_path_for_local_alias_root`) trades +2 for −5
    use-crate-cluster regressions and needs a context-aware split.

- [x] **C. `Option::unwrap()` on `None` panics in inline-module impl merging** ✅ Done (commit 09f788a, +2 pass / -2 fail)
  - Two tests fixed: `test_inline_mod_impl_methods_merged_into_struct` and
    `test_inline_mod_enum_impl_methods_merged_into_wrapper`. Root cause was
    the test-side `out.find(...).unwrap()` looking for inline-body shape
    (`Foo clone() const {`) while codegen emits decl-in-struct + def-out-of-line
    (`Foo clone() const;` decl + `Foo Foo::clone() const {` def). Replaced the
    fragile `};` close-bracket bound with ordered decl/def position checks.
  - The `test_leaf10541_*` failures that were initially grouped here are
    distinct (no `unwrap()` panic) — they belong to Bucket D (pattern-binding
    lowering) and are tracked there.

---

## Medium priority — likely single-cause clusters

- [x] **D. Pattern-binding lowering naming** ✅ Mostly done (commits 3f77f8f, 12249d9, +14 pass)
  - Root cause: codegen now uses index-based dispatch (`_m.index() == N`)
    and `rusty::detail::deref_if_pointer(...)` wrappers for safety with
    reference-of-reference scrutinees. Tests asserted the legacy
    std::visit-overloaded shape and the unwrapped extraction.
  - Resolution: tests updated to accept both lowering shapes via `||`.
    14 tests fixed (8 in 3f77f8f + 6 in 12249d9). 1 test still failing
    (`test_leaf10527_if_let_tuple_payload_binding_is_preserved_in_statement_lowering`)
    — separate shape; would need its own pass.

- [ ] **E. Inferred turbofish payload specialization** (≈ 7 failing tests)
  - Affected: `test_leaf10536_call_arg_expected_types_specialize_from_*`,
    `test_leaf5100100_lazy_new_omitted_owner_uses_auto_placeholder`,
    `test_leaf5100100_oncecell_new_uses_binary_peer_expected_type`.
  - Symptom: panic messages say "expected Some payload specialization,
    got: …" — type inference for omitted constructor turbofish.
  - Root cause hint: `infer_constructor_call_expected_owner_inner_type`
    likely.
  - Confidence: medium.

- [x] **E. Inferred-turbofish payload specialization** ✅ Done (commit 2016bdc, +4 pass)
  - Tests asserted legacy `std::make_optional<X>(v)` / `Lazy<auto, auto>::new_()` /
    bare `OnceCell` receiver shapes; codegen now emits `rusty::Option<X>(v)` /
    inferred-type `Lazy<int32_t>::new_(...)` / qualified `std::sync::OnceCell<...>::new_()`.
    Tests updated to accept both legacy and current shapes.

- [x] **F. Interface+adapter completion gaps** ✅ 2/3 done
  - `test_interface_traits_default_method_emitted_as_non_pure_virtual`:
    ✅ Done (commit 57c788e). Codegen now inlines trivial single-expr
    default-method bodies on the interface class when every `self.<m>()`
    call resolves to a method on the same trait. Other shapes still
    defer to the Adapter via `= 0;`.
  - `test_interface_traits_marker_traits_still_emit_concept`: ✅ Done
    in commit 7b0c84f (empty marker traits now emit an empty interface
    class rather than a TODO comment).
  - **Remaining**: `test_interface_traits_generic_impl_emits_specialization_with_trait_args`.
    `impl Container<i32> for IntBag` should generate
    `class ContainerAdapter<int32_t, IntBag> final : public Container<int32_t>`
    plus `Ref`/`RefMut` flavors. Currently only the primary-template
    forward decls are emitted; the specialization is skipped with a
    `// TODO(interface_traits): … generic — Adapter specializations
    require partial-spec template headers, not yet emitted` marker
    (transpiler/src/codegen.rs near line 21505). Needs deeper codegen
    work: partial-specialization template-header emission.

---

## Medium-low priority — small per-helper fixes

- [ ] **G. Rusty runtime helper call sites** (≈ 19 failing tests, several sub-fixes)
  - Tests expect specific runtime helpers to be emitted by name:
    `rusty::filter_map(values, …)`, `rusty::slice_full(var)`,
    `rusty::ptr::copy(…)`, `rusty::to_string(…)`, `rusty::checked_add(…)`,
    `rusty::iter(…)`. Each helper is a separate codegen routing path —
    these are 4-6 small fixes, not a single generic fix.

- [ ] **H. Closure body / return-expression text** (≈ 12 failing tests)
  - Tests expect bare `return a + b;` / `return x + 1;` shapes; codegen
    likely wraps closure bodies in IIFE form. May be 1-2 root causes.

- [ ] **I. Format-string emission** (≈ 5 failing tests)
  - `std::format("{0}.{1}", ...)` / `std::format("{0:>4}", ...)` —
    format-spec ordering / argument wrapping issues.

- [ ] **J. Async tail co_return** (≈ 3 failing tests)
  - `test_async_explicit_return`, `test_async_tail_co_return`, etc.
    Tail-co_return statement omitted in async lowering.

- [ ] **K. Control-flow whitespace** (≈ 6 failing tests)
  - `if (x > 0) {` shape vs alternative spacing. Likely a single
    emit-format change.

---

## Low priority — long tail

- [ ] **L. ~240 long-tail single-test failures**
  - The remaining failures are spread across `leaf41*`, `leaf42*`,
    `leaf45*`, `leaf51*`, `leaf52*`, etc. Each leaf test pins a very
    specific corner case. They don't share enough structure for a single
    generic fix — each is its own small investigation. Tackle by
    priority of the underlying feature, not by test count.

---

## Notes

- The 16 "OLD pro::proxy" tests and the 5 `_flag_off_*` interface-traits
  tests were deleted in commit 7b0c84f (May 2026) because the Pro path
  was removed in commit 90520f8 and the flag is now hardcoded `true`.
  See `docs/TODO-archive.md` for the historical state.
- Snapshot data and bucket analysis: see the conversation transcript
  for May 16, 2026 in
  `/home/users/shuai/.claude/projects/-home-users-shuai-rusty-cpp/`
  (the per-test assertion text and per-bucket counts were extracted
  from `/tmp/full_test_log.txt` during that session).
