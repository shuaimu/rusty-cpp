//! Transpiler-side type inference engine.
//!
//! Background: the architectural rationale for this module is in
//! `docs/rusty-cpp-transpiler.md` §13. Briefly: Rust uses
//! constraint-solving inference (Hindley-Milner style), C++ uses
//! local per-call-site deduction. Several patterns the parity
//! matrix exercises — Either ternary, Vec::new() with later push,
//! `to_owned`-into-field, multi-arm closure returns — rely on
//! Rust's cross-site unification. Emitting their literal C++ shape
//! gives the compiler unsolvable deduction problems. The engine
//! does the unification at codegen time and lets emit produce
//! C++ with no template parameters left to deduce.
//!
//! Scope and non-goals (per §13.8):
//!  - this is NOT a re-implementation of the Rust borrow checker
//!    or trait resolution; the input has already type-checked
//!    upstream.
//!  - lifetimes are erased — `&'a T` and `&'b T` unify as `&T`.
//!  - no let-generalization; per-call-site monomorphization is
//!    handled by the surrounding emit pipeline.
//!
//! Implementation roadmap (§13.9):
//!  - Phase 1 (THIS FILE today): scaffolding — TyTerm /
//!    Constraint / Substitution / InferenceContext data model and
//!    Robinson-style unify with occurs-check, plus a few baseline
//!    tests. No wiring into emit yet.
//!  - Phase 2: walker that turns the AST into a constraint store.
//!  - Phase 3: drive the solver to fixpoint on collected
//!    constraints; expose resolved-type lookup.
//!  - Phase 4: type-directed emit — each emit site that today
//!    asks the C++ compiler to deduce a parameter consults the
//!    engine first and falls back to local CTAD only on engine
//!    failure.
//!  - Phase 5: parity-matrix validation; the engine is "real" by
//!    the acceptance criteria in §13.10.

use std::collections::HashMap;
use std::fmt;

use syn::visit::Visit;
use syn::Type;

/// Identifier for a free type variable allocated by the engine.
///
/// Variables are dense, monotonically-issued `usize` keys. A
/// `TypeVarSource` accompanies the id only for diagnostics — the
/// solver itself never inspects it.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct TyVarId(pub(crate) usize);

impl fmt::Display for TyVarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "?{}", self.0)
    }
}

/// The term language the solver operates over.
///
/// `Concrete` wraps a normalized `syn::Type` (lifetimes erased,
/// outer parens stripped — normalization lives in
/// `normalize_concrete_type` so collection and lookup agree on
/// shape). `Var` is a free type variable. `App` is an applied type
/// constructor — Rust path types, tuple types, slice/array
/// constructors, reference constructors — anything with positional
/// arguments. The `head` is the constructor's canonical name
/// (e.g. `"Either"`, `"Vec"`, `"tuple"`, `"&"`); see
/// `constructor_head` for the canonical-name policy.
///
/// `App("Either", [Concrete(i32), Var(?7)])` is what unification
/// produces when one ternary arm has pinned `L = i32` but `R`
/// remains a variable awaiting the sibling arm's contribution.
#[derive(Clone, Debug)]
pub(crate) enum TyTerm {
    Concrete(Type),
    Var(TyVarId),
    App {
        head: String,
        args: Vec<TyTerm>,
    },
}

/// A constraint the solver must satisfy.
///
/// Phase 1 only models structural equality; richer constraint
/// kinds (subtyping, `Sized`, etc.) are intentionally absent —
/// Rust has already discharged them upstream. Each constraint
/// carries an opaque `origin` hint solely for diagnostics; the
/// solver never branches on it.
#[derive(Clone, Debug)]
pub(crate) struct Constraint {
    pub(crate) lhs: TyTerm,
    pub(crate) rhs: TyTerm,
    pub(crate) origin: ConstraintOrigin,
}

/// What kind of AST node produced a given constraint. Recorded so
/// solver failures can name the syntactic site that contributed
/// the conflict. Add variants as constraint collection grows.
#[derive(Clone, Debug)]
pub(crate) enum ConstraintOrigin {
    /// Placeholder used during scaffolding / tests when the AST
    /// origin isn't relevant.
    Synthetic(&'static str),
    /// `let pat = expr;` — both sides must unify.
    LetBinding,
    /// `if … { a } else { b }` / `match { … }` — every arm and
    /// the merged scrutinee share one variable.
    BranchMerge,
    /// `return expr;` — constrain the expression to the function's
    /// (or closure's) return slot.
    Return,
    /// `Struct { field: expr }` — constrain the expression to the
    /// declared field type.
    StructFieldInit,
}

/// The substitution accumulated by the solver. Calling
/// `apply` walks the substitution transitively; calling
/// `bind` extends it with a `?v ↦ term` pair after the
/// occurs-check has run.
#[derive(Default, Debug, Clone)]
pub(crate) struct Substitution {
    map: HashMap<TyVarId, TyTerm>,
}

impl Substitution {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn bind(&mut self, v: TyVarId, t: TyTerm) {
        self.map.insert(v, t);
    }

    pub(crate) fn lookup(&self, v: TyVarId) -> Option<&TyTerm> {
        self.map.get(&v)
    }

    /// Walk substitutions transitively. `apply` is idempotent on
    /// a closed substitution (one whose ranges contain no bound
    /// variables): every variable is resolved to its ultimate
    /// term in a single pass.
    pub(crate) fn apply(&self, term: &TyTerm) -> TyTerm {
        match term {
            TyTerm::Var(v) => match self.map.get(v) {
                Some(t) => self.apply(t),
                None => TyTerm::Var(*v),
            },
            TyTerm::App { head, args } => TyTerm::App {
                head: head.clone(),
                args: args.iter().map(|a| self.apply(a)).collect(),
            },
            TyTerm::Concrete(t) => TyTerm::Concrete(t.clone()),
        }
    }
}

/// Result of a single `unify` call. `Failed` reasons exist so the
/// caller (the constraint loop) can decide between recording the
/// failure for later attribution and propagating it as a fatal
/// error. Phase 1 uses only `OccursCheck` and `Mismatch`.
#[derive(Clone, Debug)]
pub(crate) enum UnifyError {
    OccursCheck { var: TyVarId },
    Mismatch { lhs: String, rhs: String },
}

impl fmt::Display for UnifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnifyError::OccursCheck { var } => {
                write!(f, "occurs check: {} appears in its own binding", var)
            }
            UnifyError::Mismatch { lhs, rhs } => {
                write!(f, "cannot unify {} with {}", lhs, rhs)
            }
        }
    }
}

/// Textbook Robinson unification with occurs-check.
///
/// Mutates `subst` in place. On success returns `Ok(())` and the
/// substitution has been extended to make `lhs == rhs` after
/// application. On failure leaves the substitution in a usable
/// state — the caller can keep going on independent constraints.
pub(crate) fn unify(
    subst: &mut Substitution,
    lhs: &TyTerm,
    rhs: &TyTerm,
) -> Result<(), UnifyError> {
    let l = subst.apply(lhs);
    let r = subst.apply(rhs);
    match (l, r) {
        (TyTerm::Var(a), TyTerm::Var(b)) if a == b => Ok(()),
        (TyTerm::Var(v), other) | (other, TyTerm::Var(v)) => {
            if occurs(v, &other) {
                return Err(UnifyError::OccursCheck { var: v });
            }
            subst.bind(v, other);
            Ok(())
        }
        (TyTerm::App { head: h1, args: a1 }, TyTerm::App { head: h2, args: a2 }) => {
            if h1 != h2 || a1.len() != a2.len() {
                return Err(UnifyError::Mismatch {
                    lhs: format!("{}/{}", h1, a1.len()),
                    rhs: format!("{}/{}", h2, a2.len()),
                });
            }
            for (la, ra) in a1.iter().zip(a2.iter()) {
                unify(subst, la, ra)?;
            }
            Ok(())
        }
        (TyTerm::Concrete(a), TyTerm::Concrete(b)) => {
            if concrete_types_equal(&a, &b) {
                Ok(())
            } else {
                Err(UnifyError::Mismatch {
                    lhs: render_concrete(&a),
                    rhs: render_concrete(&b),
                })
            }
        }
        (a, b) => Err(UnifyError::Mismatch {
            lhs: render_term(&a),
            rhs: render_term(&b),
        }),
    }
}

/// Check whether `v` appears free in `term`. Required to keep
/// the substitution finite — without occurs-check we would
/// happily bind `?7 ↦ Vec<?7>`, then apply would diverge.
fn occurs(v: TyVarId, term: &TyTerm) -> bool {
    match term {
        TyTerm::Var(u) => *u == v,
        TyTerm::App { args, .. } => args.iter().any(|t| occurs(v, t)),
        TyTerm::Concrete(_) => false,
    }
}

/// Conservative structural equality on normalized `syn::Type`.
/// Today this is `tokens_to_string == tokens_to_string`; that's
/// sufficient because the upstream normalization (Phase 2) will
/// canonicalize both sides through the same path before they
/// reach unify. When richer equivalences are needed (e.g.
/// `i32` ≡ `core::primitive::i32`) we add them here, not in
/// the solver.
fn concrete_types_equal(a: &Type, b: &Type) -> bool {
    use quote::ToTokens;
    a.to_token_stream().to_string() == b.to_token_stream().to_string()
}

fn render_concrete(t: &Type) -> String {
    use quote::ToTokens;
    t.to_token_stream().to_string()
}

fn render_term(t: &TyTerm) -> String {
    match t {
        TyTerm::Var(v) => v.to_string(),
        TyTerm::Concrete(c) => render_concrete(c),
        TyTerm::App { head, args } => {
            let inner = args.iter().map(render_term).collect::<Vec<_>>().join(", ");
            format!("{}<{}>", head, inner)
        }
    }
}

/// Per-function inference state. Holds the variable counter, the
/// in-progress constraint store, and the working substitution.
/// Constructed at the entry of `emit_fn_body` (Phase 2) and
/// consulted from emit sites (Phase 4). Drops at the end of the
/// function — inference does not span function boundaries (§13.4).
#[derive(Debug)]
pub(crate) struct InferenceContext {
    next_var: usize,
    constraints: Vec<Constraint>,
    subst: Substitution,
}

impl InferenceContext {
    pub(crate) fn new() -> Self {
        Self {
            next_var: 0,
            constraints: Vec::new(),
            subst: Substitution::new(),
        }
    }

    /// Allocate a fresh type variable. Phase 2 calls this whenever
    /// it encounters an under-specified expression (`Vec::new()`
    /// before a `.push`, a let with no annotation, …).
    pub(crate) fn fresh_var(&mut self) -> TyVarId {
        let id = TyVarId(self.next_var);
        self.next_var += 1;
        id
    }

    /// Record a constraint without solving it yet. The solve pass
    /// (Phase 3) iterates the full constraint set to fixpoint.
    pub(crate) fn push_constraint(&mut self, c: Constraint) {
        self.constraints.push(c);
    }

    /// Borrow the collected constraints — primarily for tests and
    /// the `--print-inference` debug dump described in §13.10.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn constraints(&self) -> &[Constraint] {
        &self.constraints
    }

    /// Run unification over all collected constraints. Returns the
    /// list of failures (empty on success). The substitution is
    /// updated with every successful unify even when later
    /// constraints fail — this matches the "fall back cleanly per
    /// site" policy in §13.5.
    pub(crate) fn solve(&mut self) -> Vec<UnifyError> {
        let mut errors = Vec::new();
        // Iterate-to-fixpoint: a constraint solved later can pin a
        // variable that another constraint references, so we re-run
        // until no progress is made. In practice 2–3 passes is
        // typical because the engine's only constraint kind is
        // structural equality; we cap iterations to keep pathological
        // inputs (which the upstream type-checker would have rejected)
        // from looping.
        let mut iter_budget = self.constraints.len().saturating_mul(2) + 4;
        let mut last_subst_len = usize::MAX;
        while iter_budget > 0 && self.subst.map.len() != last_subst_len {
            last_subst_len = self.subst.map.len();
            errors.clear();
            for c in &self.constraints {
                if let Err(e) = unify(&mut self.subst, &c.lhs, &c.rhs) {
                    errors.push(e);
                }
            }
            iter_budget -= 1;
        }
        errors
    }

    /// Resolved type for a variable, or `None` if the solver
    /// couldn't pin it. Phase-4 emit sites call this to decide
    /// whether to emit explicit template args or fall back to
    /// today's heuristic emit.
    pub(crate) fn resolve(&self, v: TyVarId) -> Option<TyTerm> {
        let term = self.subst.apply(&TyTerm::Var(v));
        match term {
            TyTerm::Var(_) => None,
            other => Some(other),
        }
    }
}

impl Default for InferenceContext {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// Phase 2: AST → constraint collection.
//
// The collector walks a function body once. Each AST node that
// produces a typed expression contributes constraints to the
// `InferenceContext`. Today's coverage is intentionally narrow —
// only the shapes documented in §13.6 that have a concrete
// downstream emit consumer (the Either ternary in §13.3,
// brace-init field constraints from §13.6, let-bindings with
// annotations). Phase 4 (emit wiring) adds the rest as it
// encounters them.
// ============================================================

/// Lower a `syn::Type` into a `TyTerm`. Concrete types pass
/// through as `Concrete`; named generics that match `binders`
/// resolve to type variables. Today we recognize `Type::Path` and
/// fall back to `Concrete` for everything else. As collection
/// grows we extend the structural decomposition (tuples → App
/// with head `"tuple"`, slice → App with head `"&[]"`, etc.) —
/// per §13.5, the head string is the canonical constructor
/// identity the solver compares on.
pub(crate) fn tyterm_from_syn(ty: &Type, binders: &HashMap<String, TyVarId>) -> TyTerm {
    match ty {
        Type::Path(tp) if tp.qself.is_none() && tp.path.segments.len() == 1 => {
            let seg = &tp.path.segments[0];
            let name = seg.ident.to_string();
            // Bare ident that binds to a known type-parameter slot —
            // produce the var, not a concrete `T`.
            if let Some(v) = binders.get(&name) {
                if let syn::PathArguments::None = seg.arguments {
                    return TyTerm::Var(*v);
                }
            }
            // Otherwise decompose `Foo<a, b>` into `App { head:
            // "Foo", args: [tyterm(a), tyterm(b)] }`. Args with no
            // generics keep the concrete spelling.
            if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                let args = ab
                    .args
                    .iter()
                    .filter_map(|ga| match ga {
                        syn::GenericArgument::Type(t) => Some(tyterm_from_syn(t, binders)),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                return TyTerm::App { head: name, args };
            }
            TyTerm::Concrete(ty.clone())
        }
        _ => TyTerm::Concrete(ty.clone()),
    }
}

/// Visitor that walks a `syn::Block` and pushes constraints onto
/// the `InferenceContext`. Today it handles:
///
/// - `let x: T = expr;` — constrains the binding to the
///   expression's tentative type. The expression type itself is
///   represented as a fresh variable unless we already know it.
/// - `if … { a } else { b }` — both arms share a fresh merge
///   variable (this is the case that solves Either).
/// - `match scrut { … }` — every arm's body unifies with the
///   merge variable, same as if/else.
///
/// Coverage will grow as Phase 4 needs more sites. Anything we
/// don't recognize is silently skipped — the engine is allowed
/// to be incomplete, not allowed to be incorrect.
pub(crate) struct ConstraintCollector<'ctx> {
    ctx: &'ctx mut InferenceContext,
    /// Type parameter names in scope for the function being
    /// walked (e.g. `T` from `fn foo<T>()`). Used by
    /// `tyterm_from_syn` to lower references to those names into
    /// the corresponding type variables instead of concrete
    /// types.
    binders: HashMap<String, TyVarId>,
    /// Fresh variable used as a synthetic "anything" placeholder
    /// for expressions whose type we don't reconstruct yet. Reset
    /// per expression by `fresh_expr_var`.
    expr_var_buffer: Vec<TyVarId>,
}

impl<'ctx> ConstraintCollector<'ctx> {
    pub(crate) fn new(ctx: &'ctx mut InferenceContext) -> Self {
        Self {
            ctx,
            binders: HashMap::new(),
            expr_var_buffer: Vec::new(),
        }
    }

    pub(crate) fn with_binders(
        ctx: &'ctx mut InferenceContext,
        binders: HashMap<String, TyVarId>,
    ) -> Self {
        Self {
            ctx,
            binders,
            expr_var_buffer: Vec::new(),
        }
    }

    /// Allocate a placeholder for the type of an arbitrary
    /// expression. Phase 2 doesn't infer expression types in
    /// detail — it just creates the variable so other constraints
    /// can mention it.
    fn fresh_expr_var(&mut self) -> TyVarId {
        let v = self.ctx.fresh_var();
        self.expr_var_buffer.push(v);
        v
    }

    /// Process a `let pat = init;` binding. When `pat` is a typed
    /// `Pat::Type(pat: Pat = ty)` and the init expression's type
    /// can be summarized into a TyTerm, push a constraint that the
    /// init's term equals the annotation's term.
    fn visit_local_for_constraints(&mut self, local: &syn::Local) {
        // Extract the binding pattern's type annotation, if any.
        let annotated_ty = match &local.pat {
            syn::Pat::Type(pt) => Some(pt.ty.as_ref()),
            _ => None,
        };
        let Some(init) = &local.init else { return };
        let init_term = self.summarize_expr(&init.expr);
        if let Some(ann) = annotated_ty {
            let ann_term = tyterm_from_syn(ann, &self.binders);
            self.ctx.push_constraint(Constraint {
                lhs: init_term,
                rhs: ann_term,
                origin: ConstraintOrigin::LetBinding,
            });
        }
    }

    /// Best-effort summary of `expr`'s type as a TyTerm. For
    /// shapes whose contribution to inference we care about —
    /// if/else, match, struct-init — we walk into them and emit
    /// proper structural constraints. For anything else we
    /// return a fresh variable, which the solver will leave
    /// unbound (and Phase 4 emit will fall back to local CTAD on).
    fn summarize_expr(&mut self, expr: &syn::Expr) -> TyTerm {
        match expr {
            syn::Expr::If(if_expr) => self.collect_if_else(if_expr),
            syn::Expr::Match(match_expr) => self.collect_match(match_expr),
            syn::Expr::Block(b) => {
                if let Some(tail) = b.block.stmts.iter().last() {
                    if let syn::Stmt::Expr(e, None) = tail {
                        return self.summarize_expr(e);
                    }
                }
                TyTerm::Var(self.fresh_expr_var())
            }
            _ => TyTerm::Var(self.fresh_expr_var()),
        }
    }

    /// Build the constraint chain for `if cond { a } else { b }`.
    /// Both arms share a merge variable; the if/else's overall
    /// type is the merge variable.
    fn collect_if_else(&mut self, if_expr: &syn::ExprIf) -> TyTerm {
        let merge = self.ctx.fresh_var();
        // True branch's tail expression.
        if let Some(tail_expr) = block_tail_expr(&if_expr.then_branch) {
            let a = self.summarize_expr(&tail_expr);
            self.ctx.push_constraint(Constraint {
                lhs: TyTerm::Var(merge),
                rhs: a,
                origin: ConstraintOrigin::BranchMerge,
            });
        }
        if let Some((_, else_expr)) = &if_expr.else_branch {
            let b = self.summarize_expr(else_expr);
            self.ctx.push_constraint(Constraint {
                lhs: TyTerm::Var(merge),
                rhs: b,
                origin: ConstraintOrigin::BranchMerge,
            });
        }
        TyTerm::Var(merge)
    }

    /// Build the constraint chain for `match scrut { … }`. Every
    /// arm's body unifies with the merge variable, identical
    /// shape to if/else (per §13.6).
    fn collect_match(&mut self, match_expr: &syn::ExprMatch) -> TyTerm {
        let merge = self.ctx.fresh_var();
        for arm in &match_expr.arms {
            let body = self.summarize_expr(&arm.body);
            self.ctx.push_constraint(Constraint {
                lhs: TyTerm::Var(merge),
                rhs: body,
                origin: ConstraintOrigin::BranchMerge,
            });
        }
        TyTerm::Var(merge)
    }
}

fn block_tail_expr(block: &syn::Block) -> Option<syn::Expr> {
    let last = block.stmts.iter().last()?;
    if let syn::Stmt::Expr(e, None) = last {
        return Some(e.clone());
    }
    None
}

impl<'ast> Visit<'ast> for ConstraintCollector<'_> {
    fn visit_local(&mut self, local: &'ast syn::Local) {
        self.visit_local_for_constraints(local);
        syn::visit::visit_local(self, local);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn ty(s: &str) -> Type {
        syn::parse_str(s).unwrap()
    }

    fn either_app(l: TyTerm, r: TyTerm) -> TyTerm {
        TyTerm::App {
            head: "Either".to_string(),
            args: vec![l, r],
        }
    }

    #[test]
    fn fresh_vars_are_distinct() {
        let mut ctx = InferenceContext::new();
        let a = ctx.fresh_var();
        let b = ctx.fresh_var();
        assert_ne!(a, b);
    }

    #[test]
    fn unify_var_to_concrete_pins_it() {
        let mut ctx = InferenceContext::new();
        let v = ctx.fresh_var();
        ctx.push_constraint(Constraint {
            lhs: TyTerm::Var(v),
            rhs: TyTerm::Concrete(ty("i32")),
            origin: ConstraintOrigin::Synthetic("test"),
        });
        let errors = ctx.solve();
        assert!(errors.is_empty(), "expected clean solve, got {:?}", errors);
        let resolved = ctx.resolve(v).expect("variable should be resolved");
        match resolved {
            TyTerm::Concrete(t) => assert_eq!(render_concrete(&t), "i32"),
            other => panic!("expected concrete i32, got {:?}", other),
        }
    }

    #[test]
    fn unify_two_vars_chains_through() {
        let mut ctx = InferenceContext::new();
        let a = ctx.fresh_var();
        let b = ctx.fresh_var();
        ctx.push_constraint(Constraint {
            lhs: TyTerm::Var(a),
            rhs: TyTerm::Var(b),
            origin: ConstraintOrigin::Synthetic("chain"),
        });
        ctx.push_constraint(Constraint {
            lhs: TyTerm::Var(b),
            rhs: TyTerm::Concrete(ty("u8")),
            origin: ConstraintOrigin::Synthetic("pin"),
        });
        assert!(ctx.solve().is_empty());
        let resolved = ctx.resolve(a).expect("a should chain to u8");
        match resolved {
            TyTerm::Concrete(t) => assert_eq!(render_concrete(&t), "u8"),
            other => panic!("expected u8, got {:?}", other),
        }
    }

    #[test]
    fn either_ternary_solves_across_arms() {
        // Models the canonical case from §13.3:
        // arm A produces Either<Concrete(i32), ?R>
        // arm B produces Either<?L, Concrete(u64)>
        // both arms must equal a shared merge variable.
        let mut ctx = InferenceContext::new();
        let merge = ctx.fresh_var();
        let r = ctx.fresh_var();
        let l = ctx.fresh_var();
        let arm_a = either_app(TyTerm::Concrete(ty("i32")), TyTerm::Var(r));
        let arm_b = either_app(TyTerm::Var(l), TyTerm::Concrete(ty("u64")));
        ctx.push_constraint(Constraint {
            lhs: TyTerm::Var(merge),
            rhs: arm_a,
            origin: ConstraintOrigin::BranchMerge,
        });
        ctx.push_constraint(Constraint {
            lhs: TyTerm::Var(merge),
            rhs: arm_b,
            origin: ConstraintOrigin::BranchMerge,
        });
        assert!(ctx.solve().is_empty());
        let resolved = ctx.resolve(merge).expect("merge should solve");
        match resolved {
            TyTerm::App { head, args } => {
                assert_eq!(head, "Either");
                assert_eq!(args.len(), 2);
                let a0 = match &args[0] {
                    TyTerm::Concrete(t) => render_concrete(t),
                    other => panic!("arg[0] not concrete: {:?}", other),
                };
                let a1 = match &args[1] {
                    TyTerm::Concrete(t) => render_concrete(t),
                    other => panic!("arg[1] not concrete: {:?}", other),
                };
                assert_eq!(a0, "i32");
                assert_eq!(a1, "u64");
            }
            other => panic!("expected Either<i32, u64>, got {:?}", other),
        }
    }

    #[test]
    fn occurs_check_rejects_recursive_binding() {
        // Build ?v = Vec<?v> and confirm the solver flags it
        // instead of looping. Models the safety property in §13.5.
        let mut ctx = InferenceContext::new();
        let v = ctx.fresh_var();
        let recursive = TyTerm::App {
            head: "Vec".to_string(),
            args: vec![TyTerm::Var(v)],
        };
        ctx.push_constraint(Constraint {
            lhs: TyTerm::Var(v),
            rhs: recursive,
            origin: ConstraintOrigin::Synthetic("occurs"),
        });
        let errors = ctx.solve();
        assert_eq!(errors.len(), 1, "expected single occurs-check failure");
        match &errors[0] {
            UnifyError::OccursCheck { var } => assert_eq!(*var, v),
            other => panic!("expected occurs-check, got {:?}", other),
        }
    }

    #[test]
    fn mismatched_heads_fail_cleanly() {
        // Vec vs Option with different head names — solver records
        // a mismatch and the substitution remains usable for any
        // other (independent) constraints, per §13.5.
        let mut ctx = InferenceContext::new();
        ctx.push_constraint(Constraint {
            lhs: TyTerm::App {
                head: "Vec".to_string(),
                args: vec![TyTerm::Concrete(ty("i32"))],
            },
            rhs: TyTerm::App {
                head: "Option".to_string(),
                args: vec![TyTerm::Concrete(ty("i32"))],
            },
            origin: ConstraintOrigin::Synthetic("mismatch"),
        });
        let errors = ctx.solve();
        assert_eq!(errors.len(), 1);
        matches!(errors[0], UnifyError::Mismatch { .. });
    }

    // ============================================================
    // Phase 2 tests — the collector walks real syn AST and the
    // solver resolves the canonical cases end-to-end.
    // ============================================================

    fn parse_block(src: &str) -> syn::Block {
        syn::parse_str(src).expect("parse_block")
    }

    #[test]
    fn collector_records_let_annotation_constraint() {
        // `let x: Vec<i32> = expr;` should push a LetBinding
        // constraint pinning the init's fresh var to Vec<i32>.
        let block = parse_block("{ let x: Vec<i32> = make(); }");
        let mut ctx = InferenceContext::new();
        let mut c = ConstraintCollector::new(&mut ctx);
        c.visit_block(&block);
        assert!(
            !ctx.constraints().is_empty(),
            "expected at least one constraint from the annotated let"
        );
        // The annotation reaches the solver as `App {head: "Vec",
        // args: [Concrete(i32)]}` because we structurally decompose
        // path types with arguments.
        let found = ctx.constraints().iter().any(|c| {
            matches!(
                &c.rhs,
                TyTerm::App { head, args }
                    if head == "Vec" && args.len() == 1
                        && matches!(&args[0], TyTerm::Concrete(t)
                            if render_concrete(t) == "i32")
            )
        });
        assert!(
            found,
            "expected a constraint pinning the binding to Vec<i32>; got {:#?}",
            ctx.constraints()
        );
    }

    #[test]
    fn collector_if_else_merges_arms_into_one_variable() {
        // The walker must produce two BranchMerge constraints
        // (one per arm) sharing the merge variable, matching the
        // shape in §13.6.
        let block = parse_block("{ let _ = if cond { a } else { b }; }");
        let mut ctx = InferenceContext::new();
        let mut c = ConstraintCollector::new(&mut ctx);
        c.visit_block(&block);
        let branch_merge_count = ctx
            .constraints()
            .iter()
            .filter(|c| matches!(c.origin, ConstraintOrigin::BranchMerge))
            .count();
        assert!(
            branch_merge_count >= 2,
            "expected at least two BranchMerge constraints; got {:#?}",
            ctx.constraints()
        );
        // All merge constraints should share their LHS variable —
        // that's the whole point of the merge.
        let merge_vars: std::collections::HashSet<TyVarId> = ctx
            .constraints()
            .iter()
            .filter(|c| matches!(c.origin, ConstraintOrigin::BranchMerge))
            .filter_map(|c| match &c.lhs {
                TyTerm::Var(v) => Some(*v),
                _ => None,
            })
            .collect();
        assert_eq!(
            merge_vars.len(),
            1,
            "all BranchMerge constraints should reference the same merge var; saw {:?}",
            merge_vars
        );
    }

    #[test]
    fn collector_match_merges_all_arms() {
        // Same shape as if/else but with three arms; the merge
        // variable should appear in all of them.
        let block = parse_block(
            "{ let _ = match x { 0 => a, 1 => b, _ => c }; }",
        );
        let mut ctx = InferenceContext::new();
        let mut c = ConstraintCollector::new(&mut ctx);
        c.visit_block(&block);
        let branch_merge_count = ctx
            .constraints()
            .iter()
            .filter(|c| matches!(c.origin, ConstraintOrigin::BranchMerge))
            .count();
        assert!(
            branch_merge_count >= 3,
            "expected at least three BranchMerge constraints (one per arm); got {}",
            branch_merge_count
        );
    }

    #[test]
    fn tyterm_from_syn_decomposes_nested_generics() {
        // `Either<Vec<i32>, Option<u64>>` should produce
        // `App("Either", [App("Vec", [Concrete(i32)]),
        //                  App("Option", [Concrete(u64)])])`.
        let ty: syn::Type = syn::parse_str("Either<Vec<i32>, Option<u64>>").unwrap();
        let term = tyterm_from_syn(&ty, &HashMap::new());
        match term {
            TyTerm::App { head, args } => {
                assert_eq!(head, "Either");
                assert_eq!(args.len(), 2);
                match &args[0] {
                    TyTerm::App { head, args } => {
                        assert_eq!(head, "Vec");
                        assert_eq!(args.len(), 1);
                    }
                    other => panic!("arg[0] should be Vec App; got {:?}", other),
                }
                match &args[1] {
                    TyTerm::App { head, args } => {
                        assert_eq!(head, "Option");
                        assert_eq!(args.len(), 1);
                    }
                    other => panic!("arg[1] should be Option App; got {:?}", other),
                }
            }
            other => panic!("expected Either App; got {:?}", other),
        }
    }

    #[test]
    fn tyterm_from_syn_resolves_known_type_params_to_vars() {
        // Inside `fn f<T>() { let _: T = … }` the `T` in the
        // annotation should lower to the same TyVarId we
        // allocated for T, not to a Concrete spelling.
        let ty: syn::Type = syn::parse_str("T").unwrap();
        let mut binders = HashMap::new();
        let v = TyVarId(42);
        binders.insert("T".to_string(), v);
        let term = tyterm_from_syn(&ty, &binders);
        match term {
            TyTerm::Var(got) => assert_eq!(got, v),
            other => panic!("expected Var, got {:?}", other),
        }
    }

    #[test]
    fn end_to_end_either_ternary_through_collector_and_solver() {
        // The high-water acceptance test: take a Rust function whose
        // body matches the §13.3 shape, run collection + solve, and
        // confirm the merge variable resolves to the unified shape.
        // We feed the arms' concrete types directly (rather than
        // walking arbitrary expressions, which Phase 2 doesn't yet
        // do) by constructing an additional constraint per arm — the
        // shape of the constraint set is what matters for this
        // milestone, not perfect type extraction from every Expr
        // node.
        let block = parse_block("{ let _ = if cond { left } else { right }; }");
        let mut ctx = InferenceContext::new();
        let mut c = ConstraintCollector::new(&mut ctx);
        c.visit_block(&block);
        // Find the (single) merge variable used by both
        // BranchMerge constraints.
        let merge_var = ctx
            .constraints()
            .iter()
            .filter_map(|c| match &c.lhs {
                TyTerm::Var(v)
                    if matches!(c.origin, ConstraintOrigin::BranchMerge) =>
                {
                    Some(*v)
                }
                _ => None,
            })
            .next()
            .expect("at least one BranchMerge with a Var LHS");
        // Find the two RHS variables (one per arm) the merge points
        // at — these correspond to the if and else tail expressions'
        // placeholder types.
        let arm_vars: Vec<TyVarId> = ctx
            .constraints()
            .iter()
            .filter(|c| matches!(c.origin, ConstraintOrigin::BranchMerge))
            .filter_map(|c| match &c.rhs {
                TyTerm::Var(v) => Some(*v),
                _ => None,
            })
            .collect();
        assert_eq!(arm_vars.len(), 2);
        // Inject §13.3's contributions: arm A has type Either<i32, ?R>,
        // arm B has type Either<?L, u64>. The solver should pin merge
        // to Either<i32, u64>.
        let r = ctx.fresh_var();
        let l = ctx.fresh_var();
        ctx.push_constraint(Constraint {
            lhs: TyTerm::Var(arm_vars[0]),
            rhs: either_app(TyTerm::Concrete(ty("i32")), TyTerm::Var(r)),
            origin: ConstraintOrigin::Synthetic("arm A type"),
        });
        ctx.push_constraint(Constraint {
            lhs: TyTerm::Var(arm_vars[1]),
            rhs: either_app(TyTerm::Var(l), TyTerm::Concrete(ty("u64"))),
            origin: ConstraintOrigin::Synthetic("arm B type"),
        });
        let errors = ctx.solve();
        assert!(errors.is_empty(), "expected clean solve; got {:?}", errors);
        let resolved = ctx
            .resolve(merge_var)
            .expect("merge variable should resolve");
        match resolved {
            TyTerm::App { head, args } => {
                assert_eq!(head, "Either");
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[0], TyTerm::Concrete(t) if render_concrete(t) == "i32"));
                assert!(matches!(&args[1], TyTerm::Concrete(t) if render_concrete(t) == "u64"));
            }
            other => panic!("expected Either<i32, u64>; got {:?}", other),
        }
    }

    #[test]
    fn substitution_apply_walks_chains_to_fixpoint() {
        // ?0 ↦ ?1, ?1 ↦ ?2, ?2 ↦ i32 — applying to ?0 must yield i32.
        // Confirms `Substitution::apply` is closed under transitivity,
        // matching the iteration invariant in `solve`.
        let mut subst = Substitution::new();
        let v0 = TyVarId(0);
        let v1 = TyVarId(1);
        let v2 = TyVarId(2);
        subst.bind(v0, TyTerm::Var(v1));
        subst.bind(v1, TyTerm::Var(v2));
        subst.bind(v2, TyTerm::Concrete(parse_quote!(i32)));
        let resolved = subst.apply(&TyTerm::Var(v0));
        match resolved {
            TyTerm::Concrete(t) => assert_eq!(render_concrete(&t), "i32"),
            other => panic!("expected i32 via chain, got {:?}", other),
        }
    }
}
