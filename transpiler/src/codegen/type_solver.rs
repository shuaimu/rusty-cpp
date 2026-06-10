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
