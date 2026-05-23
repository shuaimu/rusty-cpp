# Generic Transpiler Fixes: Lifting Patcher Rules Upstream

The BTreeMap port's `prep.sh` + `post_transpile_patch.py` carry roughly 4,700
lines of source rewrites that compensate for transpiler emit gaps. Most of
those are BTreeMap-shaped hand-ports, but ~10 rules encode generic emit
bugs whose fix would help any non-trivial Rust → C++ port.

This file is a checklist for lifting those rules into the transpiler. Each
item is sized to be one focused commit. The per-item workflow is:

1. Reproduce the bug with a minimal `/tmp/<feature>_test` crate (so the
   fix has a regression test we can keep).
2. Add the transpiler fix (and a `cargo test` entry exercising the minimal
   crate's emit).
3. Update `docs/rusty-cpp-transpiler.md` with a short subsection describing
   what now lowers cleanly.
4. Delete the corresponding rule from `prep.sh` / `post_transpile_patch.py`.
5. Re-run the full BTreeMap pipeline (transpile → patch → cmake → ninja →
   smoke tests) and confirm read+write smokes still pass.
6. `cargo test` — must stay at 1543+.
7. Commit.

The items are ordered by expected ROI (cheap mechanical fixes first, then
ones that need real emit-level reasoning).

---

## 1. Tuple `.N` field access → `std::get<N>`

**Patcher rule**: `fix_tuple_dot_underscore_access` (~50 LOC).

**Bug**: Rust `tuple.0` / `tuple.1` accesses on `std::tuple` values are
emitted as `tuple._0` / `tuple._1`, which only works when the value is a
tuple-struct with explicit `_0`/`_1` fields. For `std::tuple`-typed
expressions (e.g. an `into_kv()` return), we need `std::get<N>(tuple)`.

**Fix**: In the emit path for `syn::Member::Unnamed(idx)`, look up the
receiver's known type. If it's `std::tuple<…>` (or anything mapped to
it), emit `std::get<N>(receiver)`; otherwise keep the `_N` field-access
form for tuple-structs.

**Cleanup**: Remove `fix_tuple_dot_underscore_access` and its call site
in `main()`.

**Deferred**: Minimal repros of `h.into_kv().1` (with known receiver
type) emit `std::get<1>` correctly. The actual btree case fails
because the receiver is `auto&&`-bound (via `auto&& handle =
rusty::detail::deref_if_pointer(std::get<0>(...).....0)`) so the
transpiler can't look up the return-type of `into_kv` from the
receiver's impl. Fix would either (a) trace through auto-bindings to
recover the method receiver's struct, or (b) wrap every `expr.N`
access in a `requires {std::get<N>(...)}` SFINAE that picks
`std::get<N>` vs `._N` at C++ compile time. Both have non-trivial
emit-shape impact. Defer until we have a clean repro that doesn't
require BTreeMap context.

---

## 2. `slice.get_unchecked[_mut](i)` → `slice[i]`

**Patcher rule**: `fix_std_array_get_unchecked` (~45 LOC). Sites: 12 in
`btree_internal.cppm`.

**Bug**: Rust's `[T]::get_unchecked(i) -> &T` / `get_unchecked_mut(i)` are
unsafe slice indexers. C++ `std::array<T, N>` has no `get_unchecked` — the
correct lowering is `arr[i]` (no bounds check, matching the unsafe
contract).

**Fix**: In `emit_method_call`, when the method name is `get_unchecked` /
`get_unchecked_mut` and the receiver type is `std::array` / `std::span` /
`std::vector`, emit `recv[idx]` instead of `recv.get_unchecked(idx)`.

**Cleanup**: Remove `fix_std_array_get_unchecked` and its call site.

---

## 3. By-value `self` method → emit C++ `const`

**Patcher rule**: `fix_const_correctness` (~60 LOC). Marks
`into_leaf`, `first_leaf_edge`, `last_leaf_edge`, `left_kv`, `right_kv`
const after the fact.

**Bug**: Rust `fn foo(self) -> …` consumes the receiver — definitionally
does not mutate the existing receiver. In C++, we emit it as a non-`const`
member, which then can't be called from `const` callers.

**Fix**: In `emit_impl_item_fn`, when `receiver` is `syn::Receiver` with
no `&` (by-value), emit `const` qualifier on the C++ method. The body
still moves `*this` if needed via the existing receiver-binding logic.

**Caveat**: The body may call other non-const methods on `*this`. Those
would need to be const too — but Rust's `self`-consuming pattern only
chains other `self`-consuming or `&self` methods, both of which should
end up const. Verify with the BTreeMap smoke after applying.

**Cleanup**: Remove `fix_const_correctness` and its call site.

---

## 4. Const-value match arm patterns

**Prep rule**: The `splitpoint` rewrite from Rust `match` to if-chain
(~30 LOC of the prep.sh). Affects `node.rs::splitpoint`.

**Bug**: A Rust match arm `CONST_NAME => …` (where `CONST_NAME` is a
module-level `const`) compares the scrutinee to the const's value. The
transpiler emits this as a fresh variable binding (shadowing the const)
plus `return …`, which breaks IIFE return-type unification.

**Fix**: In match-arm lowering, when an arm pattern is a single ident
that resolves to a `const` item in scope, lower to
`if (scrutinee == CONST_NAME) { return arm_body; }` instead of a
binding-with-rename.

**Cleanup**: Remove the `splitpoint` if-chain rewrite from `prep.sh`
and confirm the original `match` form survives a re-transpile.

---

## 5. Uninitialized `let` bindings with definite-assignment

**Prep rule**: Three sites in `prep.sh`:
- `let mut a_next;` / `let mut b_next;` → `= None;` in `merge_iter.rs`
- `let (length_a, length_b);` → `let (mut length_a, mut length_b) = (0, 0);` in `split.rs`
- `let mut open_node;` + later `loop { …; open_node = …; break; … }` → `let mut open_node = loop { …; break …; … };` in `append.rs`

**Bug**: Rust allows `let mut x;` (uninitialized) when the compiler can
prove definite assignment via match arms or unconditional loop breaks.
C++ `auto` requires an initializer.

**Fix options** (pick one):
- (a) Look ahead in the block for the first unconditional assignment to
  `x`. If found, hoist it into the `let` initializer (or emit `auto x = …;`
  at that point and delete the no-init line). Requires modest data-flow.
- (b) Emit `std::optional<T> x;` followed by `x = …;` rewritten to
  `x.emplace(…);`, with later reads going through `*x`. Simpler but
  invasive at use sites.
- (c) Look at the binding type from later usage (the most reliable
  signal) and emit `T x{};` (default-construct) — works when `T` is
  default-constructible.

Recommend (a) for the common shapes; fall back to (c) when the binding
type is default-constructible; punt to a comment-and-throw stub for
truly opaque cases.

**Cleanup**: Remove all three prep.sh patches and confirm the original
Rust source survives a re-transpile.

**Deferred status**: The transpiler already handles the simplest case
(`let mut x; if cond { x = 1; } else { x = 2; } x` → `std::optional<int>`
with `.emplace`/`.value()`). But the BTreeMap merge_iter case:

    let mut a_next;
    let mut b_next;
    match self.peeked.take() {
        Some(Peeked::Left(next)) => { a_next = Some(next); … }
        …
    }

emits `auto a_next; auto b_next;` (no type), because the
`inferred_binding_ty` for the let is None (no initializer) and the
existing `augment_uninitialized_local_type_hints_from_usage`
pre-pass doesn't pick up assignments-from-method-call-return as
type hints. Fix would extend the pre-pass to:
- Look at the RHS expression type of later assignments.
- Apply the inferred type to the let binding.

The shape of the work is in `collect_local_generic_placeholder_hints`
and friends — a separate-enough investigation to defer.

---

## 6. Reference-returning `let` bindings: `const auto x = ref_call()` → `auto& x = …`

**Patcher rule**: `fix_dormant_map_reborrow_binding` (~30 LOC).

**Bug**: When a Rust `let x = self.field.method();` calls a method whose
signature returns `&T` / `&mut T`, the C++ emit picks `const auto x = …`,
which decays the reference to a value copy. For non-copyable types
(BTreeMap is the prime example) this fails compile; for any non-trivial
type it silently copies.

The minimal repro emits correctly (`Container& x = …`); the actual
btree case hits a different code path. The precondition for the
`const auto` mode was not isolated in the prior session — it appears
to involve **cross-module** ref-return resolution inside a deeply
nested lambda IIFE.

**Fix**: In `emit_local`, infer the return type of the RHS expression.
If it's `Type::Reference(_)`, emit `auto&` (or `const auto&` per
mutability) instead of `auto` / `const auto`. The infrastructure
exists (`return_type_is_reference` at codegen.rs:29073) — needs to be
threaded into the let-binding emit decision.

**Caveat**: Has to look up the receiver's struct/impl-block to find
the method's return type. Cross-module lookup needs to use the same
machinery as the existing per-call recovery. May surface as the
"why doesn't the minimal repro hit this mode?" question — answer that
first.

**Cleanup**: Remove `fix_dormant_map_reborrow_binding` and its call
site. Re-verify Issues A surface no errors.

---

## 7. Wrong template-arg recovery: `LeafNode<A, Node>` / `NodeRef<Mut, …, Internal>::from_new_leaf`

**Patcher rules**:
- `fix_leafnode_new_template_args` (~20 LOC)
- `fix_from_new_leaf_markers` (~20 LOC)

**Bug**: For a Rust `LeafNode::new(alloc)` call (no explicit template
args), the transpiler tries to recover the args. Inside an absorbed
Cluster A method, K/V are no longer in scope as type params, so the
recovery falls back to picking other in-scope idents (`A` from the
method's template, `Node` from the host class). For `NodeRef::from_new_leaf(…)`,
it picks the impl block's NodeRef markers (`Mut`, `Internal`) when the
construction site needs `Owned`, `Leaf`.

**Fix**: In the `auto_args` fallback at codegen.rs:69763:
- When inside an absorbed-method context (Cluster A is active), use
  `typename __TemplateArgs<Node>::arg_<N>` for any inner positions that
  match the host's structural decomposition, instead of grabbing
  whatever happens to be in scope.
- For `NodeRef::from_new_leaf`, the return type is `NodeRef<Owned, K, V, Leaf>`
  by construction (not the receiver's NodeRef instantiation). Look at the
  method's RETURN type, not the path's owner shape.

Both are recovery-site improvements: the existing code does string-level
guessing where AST-level reasoning is available.

**Status (partial)**: The LeafNode case is done — `recover_omitted_owner_generic_args_from_scope` now consults the current emitting method's structural decomposition (`current_emit_structural_decomp`) and returns `typename __TemplateArgs<HostParam>::arg_<pos>` for dropped owner-declared params before falling back to ordered-scope. `LeafNode<A, Node>::new_(alloc)` now emits directly as `LeafNode<arg_1, arg_2>::new_(alloc)`. `fix_leafnode_new_template_args` is removed from the patcher.

The `NodeRef::from_new_leaf` case is **not** generically fixed yet — it needs lookup of the *called method's* defining impl block (which has the concrete `Owned`/`Leaf` markers), separate from the *current method's* impl block. Cluster A only tracks the latter. The patcher's `fix_from_new_leaf_markers` is kept, now matching both pre- and post-Item-7 emit shapes.

**Cleanup**: `fix_leafnode_new_template_args` removed. `fix_from_new_leaf_markers` kept.

---

## 8. Recursive lambda → Y-combinator

**Patcher rule**: `fix_recursive_lambda_clone_subtree` (~85 LOC). Site:
`BTreeMap::clone()::clone_subtree`.

**Bug**: Rust lets a `let f = |args| { …; f(args) };` lambda reference
its own name in the body — because `f`'s name is in scope of the
initializer (for closures). C++ `auto`-deduced lambdas can't do this:
the lambda's type isn't known when the initializer runs, so the name
`f` in the body has no type.

**Fix**: Detect recursive self-reference in lambda init bodies.
Rewrite to Y-combinator form:
```cpp
auto f = [&](auto&& self, args) -> ReturnType {
    ...
    return self(self, args);
};
// callers: f(f, args) or wrap:
auto f_wrapped = [&](args) { return f(f, args); };
```

Trickier emit-wise but Y-combinator is the well-known C++ workaround.

**Status (done)**: `emit_nested_function` now detects self-recursion by token-stream-scanning the body for the fn's own name, and lowers the lambda with `auto&& __self` prepended. `try_emit_y_combinator_call` (hook at the top of `emit_call_expr_to_string`) rewrites internal `NAME(args)` calls to `__self(__self, args)` and external callers (later statements in the same scope) to `NAME(NAME, args)`. Scope tracking is block-scoped via `recursive_nested_fns_in_scope`. `fix_recursive_lambda_clone_subtree` is removed from the patcher.

**Cleanup**: `fix_recursive_lambda_clone_subtree` removed.

---

## Borderline items (lower priority, leaves the patcher carrying these)

### 9. C++20 module-namespace prefix stripping

**Patcher rules**: `patch_entry_imports` + `strip_module_namespace_prefixes` (~120 LOC).

**Bug**: When a C++20 module imports symbols from another module, those
symbols are at file scope — there's no `othermod::` namespace alias.
The transpiler emits `othermod::Symbol` references because Rust's
`use other_mod::Symbol` syntactically looks namespace-qualified. Each
emitted reference needs `othermod::` stripped.

**Fix**: At emit time, when we resolve a path through an `import`-rooted
module, strip the leading module-name segments. The transpiler already
knows which module each symbol came from (it tracks them in
`import_paths` / `declared_module_names`).

Lower priority because the patcher rules are already mechanical text
rewrites that work fine; the generic version saves ~120 LOC but
doesn't unblock anything new.

### 10. Cluster A direct positional matches (DONE in this session)

Already lifted as commit `cae6682` ("transpiler: Cluster A direct
positional matches"). Listed here for completeness.

### 11. Complex tuple-pattern match → real match-lowering

**Patcher rule**: `stub_insert_recursing` (~60 LOC) — workaround, not a
real fix.

**Bug**: A Rust match over a tuple of variants (e.g.
`match (Option<X>, Y) { (None, y) => …, (Some(_), _) => unreachable!() }`)
lowers to a `std::visit(overloaded { [&](auto&&) { unreachable(); },
[&](auto&&) { unreachable(); } }, …)` because the transpiler couldn't
decompose the tuple-and-variant nested pattern. Both arms become
unreachable, the destructuring of the visit result fails.

**Fix**: Proper nested-pattern lowering for `(Variant, …)` shape:
- `(None, handle)` → check `std::get<0>(t).is_none()`; bind `handle = std::get<1>(t)`
- `(Some(x), y)` → check `std::get<0>(t).is_some()`; bind via unwrap

Real match-lowering work, not a one-line emit redirect. Lower priority
because the empty-map insert path doesn't trigger this at runtime —
only larger inserts would.

**Status (partial)**: Two of the three pieces have landed in the transpiler:

1. *Tuple-arity from arm patterns*: `match_expr_to_string` now also derives the scrutinee's tuple arity from the arms when `infer_simple_expr_type` returns nothing for the scrutinee. If every arm is a `Pat::Tuple` of the same arity (or `_` wildcard fallback), it routes to the value-conditions emit instead of the broken `std::visit(overloaded { … })` shape. Method-call scrutinees like `self.foo()` returning a tuple no longer degrade to two-unreachable lambdas.
2. *Movable payload bindings*: `emit_match_expr_tuple_value_conditions_for_scrutinee_expr` now applies `allow_runtime_match_binding_payload_moves` for unguarded arms, so `Some(x)` pattern extraction strips the `std::as_const(_).unwrap()` wrap when the same source isn't reused — keeping the payload movable for the arm body (necessary when the bound value calls non-const member functions like `SplitResult::forget_node_type`).
3. *NOT done — early-return-arm in let-binding*: when one match arm has a non-local `return X` and the whole match is bound to a `let`, the C++ lowering still wraps the arms in a lambda whose `return` becomes lambda-local rather than function-local — and the arm types diverge so the lambda's `auto` return deduction fails. Lifting the diverging arm OUT of the IIFE requires statement-level match lowering (emit if-return before the let-binding, keep only the success arm value as the let-binding's RHS). That's a larger refactor that touches `Stmt::Local` emission; left as follow-on work. For now the patcher's `stub_insert_recursing` remains in place to keep the BTreeMap pipeline green.

The two parts that did land unblock all other tuple-pattern matches in this shape that *don't* use `return` in an arm.

**Cleanup**: `stub_insert_recursing` kept (still needed for the let+match+return case).

---

## Status tracking (update as we go)

| #  | Item                                         | Status      | Commit |
|----|----------------------------------------------|-------------|--------|
| 1  | Tuple `.N` → `std::get<N>`                   | deferred*   | —      |
| 2  | `slice.get_unchecked` → `slice[i]`           | done        | 0dc9512|
| 3  | By-value self → C++ `const`                  | deferred*   | —      |
| 4  | Const-value match patterns                   | partial*    | 0fc730f |
| 5  | Uninitialized `let` bindings                 | deferred*   | —      |
| 6  | Ref-returning `let` bindings                 | partial*    | aedd910 |
| 7  | Wrong template-arg recovery                  | partial*    | 1e6b8c6 |
| 8  | Recursive lambda → Y-combinator              | done        | 9073eaa |
| 9  | Module-namespace prefix stripping (borderline) | pending   | —      |
| 10 | Cluster A direct positional matches          | done        | cae6682|
| 11 | Tuple-pattern match lowering (borderline)    | partial*    | 54cbe73 |
