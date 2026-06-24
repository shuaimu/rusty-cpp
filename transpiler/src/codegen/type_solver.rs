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

/// Callback supplying the element (item) type of a receiver iterator
/// expression — backed by `CodeGen::infer_iter_item_type_from_expr`.
/// Used by the block walk to pin closure item-parameters and `extend`
/// element types that the pure-structural pass can't see. `None` when
/// the item type can't be determined.
pub(crate) type ItemResolver<'a> = dyn Fn(&syn::Expr) -> Option<Type> + 'a;

/// Callback supplying the sole field type of a locally-declared
/// single-field "newtype" struct, keyed by the struct's (tail) name —
/// backed by `CodeGen::single_field_type_of_struct` reading
/// `struct_field_types`. Lets the collector turn a consumer
/// `Wrapper::from(local)` / `Wrapper::new(local)` into the constraint
/// `typeof(local) = <Wrapper's field type>` — the §13.14 C1 rule that
/// recovers an opaque wrapper's element (serde_bytes `ByteBuf`/`ByteArray`).
/// `None` when the name isn't a known single-field struct.
pub(crate) type FieldResolver<'a> = dyn Fn(&str) -> Option<Type> + 'a;

/// A callee's declared signature, as seen at a call site, for the §13.14 C2
/// call-signature rule. `type_params` are the callee's own generic parameter
/// names (e.g. `["T"]` for `fn f<T>(…)`); the collector allocates a fresh type
/// variable per call so distinct call sites monomorphize independently.
/// `params[i]` is the declared type of positional argument `i` (`None` when
/// unknown), and `ret` is the declared return type — both written in terms of
/// the callee's `type_params`, which `tyterm_from_syn` lowers to the
/// per-call fresh variables.
pub(crate) struct FnSig {
    pub(crate) type_params: Vec<String>,
    pub(crate) params: Vec<Option<Type>>,
    pub(crate) ret: Type,
}

/// Callback resolving a *call expression* to its callee's [`FnSig`] — backed by
/// `CodeGen`'s `lookup_function_type_param_names` / `lookup_function_arg_expected_type`
/// / `lookup_function_return_type`. Lets the collector pin a call's argument
/// types to the callee's parameters and the call's result to the (instantiated)
/// return type — Rust's cross-call unification that C++ per-site CTAD can't do.
/// `None` when the callee's signature isn't known.
pub(crate) type SignatureResolver<'a> = dyn Fn(&syn::ExprCall) -> Option<FnSig> + 'a;

/// Callback resolving a *method call* `recv.m(args)` to its result type —
/// backed by `CodeGen::infer_method_call_result_type_for_local` (+ user-method
/// return lookups). The common element source `v.push(x.method())` needs the
/// method's return type; method generics/arg-unification are rarer and skipped
/// for now (result-type only). `None` when the method's return isn't known.
pub(crate) type MethodResolver<'a> = dyn Fn(&syn::ExprMethodCall) -> Option<Type> + 'a;

/// The optional CodeGen-backed resolvers an owner-element/local inference query
/// may consult. Bundled so adding rules (§13.14 C-series) doesn't grow the
/// query signatures. `Default` is all-`None` (pure-structural inference).
#[derive(Default, Clone, Copy)]
pub(crate) struct OwnerElementResolvers<'a> {
    pub(crate) field: Option<&'a FieldResolver<'a>>,
    pub(crate) sig: Option<&'a SignatureResolver<'a>>,
    pub(crate) method: Option<&'a MethodResolver<'a>>,
}

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
///
/// `Clone` is required by the enclosing `CodeGen` struct's
/// snapshot-and-restore pattern; nothing inside the engine itself
/// relies on cloning being cheap. Phase 4 will revisit if the
/// snapshot path becomes hot.
#[derive(Debug, Clone)]
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
        // Pointer / reference constructors decompose so a type parameter buried
        // inside them can still flow — e.g. a return-only `*mut T` unifies its
        // element against the expected pointer type's element to solve `T`
        // (`invalid_mut<T>(addr) -> *mut T`). Heads carry the qualifier so
        // `*mut`/`*const`/`&`/`&mut` don't accidentally unify with each other.
        Type::Ptr(p) => {
            let head = if p.mutability.is_some() {
                "*mut"
            } else {
                "*const"
            };
            TyTerm::App {
                head: head.to_string(),
                args: vec![tyterm_from_syn(&p.elem, binders)],
            }
        }
        Type::Reference(r) => {
            let head = if r.mutability.is_some() { "&mut" } else { "&" };
            TyTerm::App {
                head: head.to_string(),
                args: vec![tyterm_from_syn(&r.elem, binders)],
            }
        }
        _ => TyTerm::Concrete(ty.clone()),
    }
}

/// Solve the generic type arguments of an owner type `Owner<…>` for an
/// associated call `Owner::ctor(args)` — Rust's cross-call/return unification
/// that C++ per-site CTAD on a class-template static member can't do.
///
/// This is the engine half of the §13.14 turbofish rule: CodeGen supplies the
/// *facts* (the owner's type-parameter names, the ctor's declared parameter and
/// return types written in those names, the call's concrete argument types, and
/// the expected result type), and the solver *unifies* them:
///   - **forward**: each argument's concrete type pins the matching parameter
///     slot (`UnsafeCell::new(value: T) → T`),
///   - **backward**: the expected result type pins return-determined params
///     (`x: Box<i32> = id_box(…) → T = i32`).
///
/// Returns one resolved `syn::Type` per `type_params` slot, in order, or `None`
/// if any slot remains a free variable — so the caller falls back to its
/// existing heuristic and the rule can only *add* coverage, never change an
/// answer (freeze-grow-reconcile).
///
/// Note: `tyterm_from_syn` treats `*mut T` / `&T` return shapes as opaque, so a
/// return-ONLY param hidden inside a pointer/reference isn't solved here yet —
/// that needs the structural pointer rule (a later slice). Path-shaped returns
/// (`Owner<T>`) and all forward (argument-determined) params work today.
/// Collapse pointer/reference qualifiers to a single neutral head
/// (`*mut`/`*const` → `*`, `&mut`/`&` → `&ref`) so unification ignores mutness
/// when solving the *element* type parameter — a `*mut T` return unifies against
/// a `*const T` (or vice versa) expected type to give `T`, since constness never
/// changes which type fills the slot. Only used inside `solve_owner_type_args`,
/// which returns the resolved type PARAMS (the elements), so the dropped
/// qualifier never reaches the emitted output.
fn neutralize_ptr_ref_heads(t: &TyTerm) -> TyTerm {
    match t {
        TyTerm::App { head, args } => {
            let neutral = match head.as_str() {
                "*mut" | "*const" => "*",
                "&mut" | "&" => "&ref",
                other => other,
            };
            TyTerm::App {
                head: neutral.to_string(),
                args: args.iter().map(neutralize_ptr_ref_heads).collect(),
            }
        }
        TyTerm::Concrete(c) => TyTerm::Concrete(c.clone()),
        TyTerm::Var(v) => TyTerm::Var(*v),
    }
}

pub(crate) fn solve_owner_type_args(
    type_params: &[String],
    params: &[Option<Type>],
    ret: Option<&Type>,
    arg_types: &[Option<Type>],
    expected: Option<&Type>,
) -> Option<Vec<Type>> {
    if type_params.is_empty() {
        return None;
    }
    let mut binders: HashMap<String, TyVarId> = HashMap::new();
    let mut vars: Vec<TyVarId> = Vec::with_capacity(type_params.len());
    for (i, tp) in type_params.iter().enumerate() {
        let v = TyVarId(i);
        binders.insert(tp.clone(), v);
        vars.push(v);
    }
    let no_binders: HashMap<String, TyVarId> = HashMap::new();
    let mut subst = Substitution::new();
    // Forward: argument type pins the corresponding parameter slot.
    for (i, param) in params.iter().enumerate() {
        if let (Some(param_ty), Some(Some(arg_ty))) = (param.as_ref(), arg_types.get(i)) {
            let p = neutralize_ptr_ref_heads(&tyterm_from_syn(param_ty, &binders));
            let a = neutralize_ptr_ref_heads(&tyterm_from_syn(arg_ty, &no_binders));
            let _ = unify(&mut subst, &p, &a);
        }
    }
    // Backward: the expected result type pins return-determined params.
    if let (Some(ret_ty), Some(exp_ty)) = (ret, expected) {
        let r = neutralize_ptr_ref_heads(&tyterm_from_syn(ret_ty, &binders));
        let e = neutralize_ptr_ref_heads(&tyterm_from_syn(exp_ty, &no_binders));
        let _ = unify(&mut subst, &r, &e);
    }
    let mut out = Vec::with_capacity(vars.len());
    for v in &vars {
        let resolved = subst.apply(&TyTerm::Var(*v));
        out.push(tyterm_to_syn_type(&resolved)?);
    }
    Some(out)
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
pub(crate) struct ConstraintCollector<'ctx, 'r> {
    ctx: &'ctx mut InferenceContext,
    /// Optional callback resolving a single-field newtype struct's field
    /// type by name (§13.14 C1). When set, `Wrapper::from/new(local)`
    /// consumer calls constrain `local`'s type to that field type. `None`
    /// for callers (if/else merge, fold reducer) that don't need it.
    field_resolver: Option<&'r FieldResolver<'r>>,
    /// Optional callback resolving a call expression to its callee's signature
    /// (§13.14 C2). When set, `summarize_expr` pins each call argument to the
    /// callee's parameter type and returns the (per-call-instantiated) return
    /// type instead of a fresh variable. `None` for callers that don't need it.
    sig_resolver: Option<&'r SignatureResolver<'r>>,
    /// Optional callback resolving a method call to its result type (§13.14 C2,
    /// method form). When set, `summarize_expr` returns it for an unrecognized
    /// `recv.m(args)` instead of a fresh variable. `None` for callers that
    /// don't need it.
    method_resolver: Option<&'r MethodResolver<'r>>,
    /// Type parameter names in scope for the function being
    /// walked (e.g. `T` from `fn foo<T>()`). Used by
    /// `tyterm_from_syn` to lower references to those names into
    /// the corresponding type variables instead of concrete
    /// types.
    binders: HashMap<String, TyVarId>,
    /// Value bindings in scope (let locals, closure params) mapped to
    /// the type variable representing each binding's type. Lets
    /// `summarize_expr` lower a bare identifier reference to the same
    /// variable other constraints pin, so e.g. `acc.push(v)` ties the
    /// accumulator's element to `v`'s type. Empty for the original
    /// if/else-merge callers; populated by the owner-usage queries.
    env: HashMap<String, TyVarId>,
    /// Fresh variable used as a synthetic "anything" placeholder
    /// for expressions whose type we don't reconstruct yet. Reset
    /// per expression by `fresh_expr_var`.
    expr_var_buffer: Vec<TyVarId>,
}

impl<'ctx, 'r> ConstraintCollector<'ctx, 'r> {
    pub(crate) fn new(ctx: &'ctx mut InferenceContext) -> Self {
        Self {
            ctx,
            field_resolver: None,
            sig_resolver: None,
            method_resolver: None,
            binders: HashMap::new(),
            env: HashMap::new(),
            expr_var_buffer: Vec::new(),
        }
    }

    pub(crate) fn with_binders(
        ctx: &'ctx mut InferenceContext,
        binders: HashMap<String, TyVarId>,
    ) -> Self {
        Self {
            ctx,
            field_resolver: None,
            sig_resolver: None,
            method_resolver: None,
            binders,
            env: HashMap::new(),
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
    /// if/else, match, struct-init, recognized variant
    /// constructor calls — we walk into them and emit proper
    /// structural constraints. For anything else we return a
    /// fresh variable, which the solver will leave unbound (and
    /// Phase 4 emit will fall back to local CTAD on).
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
            syn::Expr::Path(p)
                if p.qself.is_none()
                    && p.path.segments.len() == 1
                    && matches!(p.path.segments[0].arguments, syn::PathArguments::None) =>
            {
                // A bare identifier reference: lower to the variable
                // representing that binding's type when it is in scope
                // (a let local or closure param). This is what lets
                // `acc.push(v)` tie the accumulator's element to `v`'s
                // type. Unknown identifiers stay a fresh variable.
                let name = p.path.segments[0].ident.to_string();
                match self.env.get(&name) {
                    Some(v) => TyTerm::Var(*v),
                    None => TyTerm::Var(self.fresh_expr_var()),
                }
            }
            syn::Expr::Call(call) => {
                // Recognize `Enum::Variant(arg)` shapes. For enums
                // whose variants each fix a distinct type
                // parameter (Either being the canonical case),
                // each constructor produces an `App(Enum, ...)`
                // term where the argument types are pinned in the
                // matching positions and the others are fresh
                // variables.
                //
                // This is intentionally narrow today — `recognize_
                // variant_constructor_call` ships with hard-coded
                // knowledge of `Either::Left/Right`. The right
                // architectural fix is to surface the
                // enum-variant-to-parameter map from `CodeGen`'s
                // `data_enum_variant_indices_by_enum`, but the
                // collector is module-local and can't reach it
                // yet. Hard-coding the canonical case unblocks the
                // §13.3 fix while keeping the seam visible — Phase
                // 4c-ii / the eventual generalization replace
                // this with a passed-in oracle.
                if let Some(term) = self.recognize_variant_constructor_call(call) {
                    return term;
                }
                // A `Vec::new()` / `Vec::with_capacity(n)` /
                // `Vec::default()` constructor with no element argument:
                // an owner with a fresh, as-yet-unknown element.
                if let Some(head) = owner_constructor_head(call) {
                    return TyTerm::App {
                        head,
                        args: vec![TyTerm::Var(self.fresh_expr_var())],
                    };
                }
                // §13.14 C2: a call to a known-signature callee — pin args to
                // params and yield the instantiated return type.
                if let Some(term) = self.summarize_call_via_signature(call) {
                    return term;
                }
                TyTerm::Var(self.fresh_expr_var())
            }
            syn::Expr::Reference(r) => {
                // `&e` / `&mut e` — model as `App("&", [typeof(e)])`
                // (lifetimes/mutability erased per §13.4) so an element
                // pushed by reference resolves to a reference type
                // rather than silently dropping the `&`.
                let inner = self.summarize_expr(&r.expr);
                TyTerm::App {
                    head: "&".to_string(),
                    args: vec![inner],
                }
            }
            syn::Expr::Tuple(t) => {
                // `(a, b, …)` — a tuple term so a pushed tuple resolves
                // its element to `(typeof(a), typeof(b), …)`.
                let args = t.elems.iter().map(|e| self.summarize_expr(e)).collect();
                TyTerm::App {
                    head: "tuple".to_string(),
                    args,
                }
            }
            syn::Expr::Paren(p) => self.summarize_expr(&p.expr),
            syn::Expr::Group(g) => self.summarize_expr(&g.expr),
            syn::Expr::Lit(lit) => {
                lit_tyterm(&lit.lit).unwrap_or_else(|| TyTerm::Var(self.fresh_expr_var()))
            }
            syn::Expr::MethodCall(mc)
                if mc.args.is_empty()
                    && matches!(mc.method.to_string().as_str(), "clone" | "to_owned") =>
            {
                // `x.clone()` / `x.to_owned()` preserve the receiver's
                // type for element-inference purposes (the engine erases
                // the owned/borrowed distinction per §13.4).
                self.summarize_expr(&mc.receiver)
            }
            syn::Expr::MethodCall(mc) => {
                // §13.14 C2 (method form): an unrecognized `recv.m(args)` —
                // ask the method resolver for its result type so e.g.
                // `v.push(x.parse_thing())` resolves the Vec element. Falls
                // back to a fresh variable when no resolver / unknown method.
                if let Some(resolver) = self.method_resolver
                    && let Some(ty) = resolver(mc)
                {
                    return TyTerm::Concrete(ty);
                }
                TyTerm::Var(self.fresh_expr_var())
            }
            _ => TyTerm::Var(self.fresh_expr_var()),
        }
    }

    /// Hard-coded recognition of two-segment `Enum::Variant(arg)`
    /// calls where the variant pins a specific position of the
    /// enum's type parameter list. Returns `Some(App(...))` for
    /// recognized cases, `None` otherwise.
    ///
    /// Current coverage:
    /// - `Either::Left(arg)`  → `App("Either", [typeof(arg), ?R])`
    /// - `Either::Right(arg)` → `App("Either", [?L, typeof(arg)])`
    ///
    /// Adding a new enum is a one-line table entry until we
    /// promote this to a `CodeGen`-driven oracle.
    fn recognize_variant_constructor_call(
        &mut self,
        call: &syn::ExprCall,
    ) -> Option<TyTerm> {
        let path = match &*call.func {
            syn::Expr::Path(p) if p.qself.is_none() => &p.path,
            _ => return None,
        };
        if path.segments.len() != 2 {
            return None;
        }
        let enum_name = path.segments[0].ident.to_string();
        let variant_name = path.segments[1].ident.to_string();
        if call.args.len() != 1 {
            return None;
        }
        let arg_term = self.summarize_expr(&call.args[0]);
        match (enum_name.as_str(), variant_name.as_str()) {
            ("Either", "Left") => Some(TyTerm::App {
                head: "Either".to_string(),
                args: vec![arg_term, TyTerm::Var(self.fresh_expr_var())],
            }),
            ("Either", "Right") => Some(TyTerm::App {
                head: "Either".to_string(),
                args: vec![TyTerm::Var(self.fresh_expr_var()), arg_term],
            }),
            _ => None,
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

    /// Walk `expr` collecting element constraints from mutating method
    /// calls on owner bindings in `env` — `recv.push(arg)` /
    /// `recv.push_back(arg)` / `recv.push_front(arg)` pin the
    /// receiver's element to `arg`'s type. Recurses through the
    /// expression/statement shapes a reducer or block body uses.
    /// Anything unrecognized is skipped (incomplete, never incorrect).
    fn collect_owner_method_usage(&mut self, expr: &syn::Expr) {
        match expr {
            syn::Expr::Block(b) => self.collect_owner_method_usage_block(&b.block),
            syn::Expr::MethodCall(mc) => {
                self.collect_owner_method_usage(&mc.receiver);
                for a in &mc.args {
                    self.collect_owner_method_usage(a);
                }
                self.record_owner_push_constraint(mc);
            }
            syn::Expr::If(e) => {
                self.collect_owner_method_usage(&e.cond);
                self.collect_owner_method_usage_block(&e.then_branch);
                if let Some((_, els)) = &e.else_branch {
                    self.collect_owner_method_usage(els);
                }
            }
            syn::Expr::Match(e) => {
                self.collect_owner_method_usage(&e.expr);
                for arm in &e.arms {
                    self.collect_owner_method_usage(&arm.body);
                }
            }
            syn::Expr::ForLoop(e) => {
                self.collect_owner_method_usage(&e.expr);
                self.collect_owner_method_usage_block(&e.body);
            }
            syn::Expr::While(e) => {
                self.collect_owner_method_usage(&e.cond);
                self.collect_owner_method_usage_block(&e.body);
            }
            syn::Expr::Loop(e) => self.collect_owner_method_usage_block(&e.body),
            syn::Expr::Paren(e) => self.collect_owner_method_usage(&e.expr),
            syn::Expr::Group(e) => self.collect_owner_method_usage(&e.expr),
            syn::Expr::Reference(e) => self.collect_owner_method_usage(&e.expr),
            syn::Expr::Call(c) => {
                for a in &c.args {
                    self.collect_owner_method_usage(a);
                }
            }
            _ => {}
        }
    }

    fn collect_owner_method_usage_block(&mut self, block: &syn::Block) {
        for stmt in &block.stmts {
            match stmt {
                syn::Stmt::Expr(e, _) => self.collect_owner_method_usage(e),
                syn::Stmt::Local(l) => {
                    if let Some(init) = &l.init {
                        self.collect_owner_method_usage(&init.expr);
                    }
                }
                _ => {}
            }
        }
    }

    /// If `mc` is `recv.push(arg)` (or push_back/push_front) where
    /// `recv` is a bare identifier bound in `env`, constrain the
    /// receiver to `App("Vec", [typeof(arg)])` so its element type
    /// unifies with the pushed value's type.
    fn record_owner_push_constraint(&mut self, mc: &syn::ExprMethodCall) {
        if mc.args.len() != 1 {
            return;
        }
        let method = mc.method.to_string();
        if !matches!(method.as_str(), "push" | "push_back" | "push_front") {
            return;
        }
        let syn::Expr::Path(p) = &*mc.receiver else {
            return;
        };
        if p.qself.is_some()
            || p.path.segments.len() != 1
            || !matches!(p.path.segments[0].arguments, syn::PathArguments::None)
        {
            return;
        }
        let recv_name = p.path.segments[0].ident.to_string();
        let Some(&rv) = self.env.get(&recv_name) else {
            return;
        };
        let arg_term = self.summarize_expr(&mc.args[0]);
        self.ctx.push_constraint(Constraint {
            lhs: TyTerm::Var(rv),
            rhs: TyTerm::App {
                head: "Vec".to_string(),
                args: vec![arg_term],
            },
            origin: ConstraintOrigin::Synthetic("owner-push"),
        });
    }

    /// §13.14 C1: when `call` is a single-field-newtype consumer
    /// `Wrapper::from/new/new_/try_from(local)` and `local` is a bare
    /// identifier bound in `env`, constrain `local`'s type to `Wrapper`'s
    /// sole field type (supplied by `field_resolver`). This is how an
    /// opaque wrapper's element flows backward to the accumulator local —
    /// `Ok(ByteBuf::from(bytes))` pins `bytes : Vec<u8>` because `ByteBuf`
    /// has a single `Vec<u8>` field — without a bespoke heuristic pass.
    fn record_newtype_field_constraint(&mut self, call: &syn::ExprCall) {
        let Some(resolver) = self.field_resolver else {
            return;
        };
        let syn::Expr::Path(fp) = call.func.as_ref() else {
            return;
        };
        if fp.qself.is_some() || fp.path.segments.len() < 2 {
            return;
        }
        let last = fp.path.segments.last().unwrap().ident.to_string();
        if !matches!(last.as_str(), "from" | "new" | "new_" | "try_from") {
            return;
        }
        let owner = fp
            .path
            .segments
            .iter()
            .nth_back(1)
            .unwrap()
            .ident
            .to_string();
        let Some(field_ty) = resolver(&owner) else {
            return;
        };
        let field_term = tyterm_from_syn(&field_ty, &self.binders);
        for arg in &call.args {
            let syn::Expr::Path(ap) = arg else {
                continue;
            };
            if ap.qself.is_some()
                || ap.path.segments.len() != 1
                || !matches!(ap.path.segments[0].arguments, syn::PathArguments::None)
            {
                continue;
            }
            let name = ap.path.segments[0].ident.to_string();
            if let Some(&v) = self.env.get(&name) {
                self.ctx.push_constraint(Constraint {
                    lhs: TyTerm::Var(v),
                    rhs: field_term.clone(),
                    origin: ConstraintOrigin::Synthetic("newtype-field"),
                });
            }
        }
    }

    /// §13.14 C2: summarize a call `f(a0, a1, …)` via its callee's signature.
    /// Allocates a fresh type variable per callee generic (so each call site
    /// monomorphizes independently), constrains each argument's type to the
    /// matching parameter type, and returns the instantiated return type. This
    /// is the cross-call unification Rust does and C++ per-site deduction can't:
    /// `let v = Vec::new(); v.push(make());` resolves `v`'s element to `make`'s
    /// return type, and `f::<T>(x)`-style generic returns pin `T` from the
    /// argument. Returns `None` (→ fresh var, today's behavior) when no
    /// signature resolver is set or the callee is unknown.
    fn summarize_call_via_signature(&mut self, call: &syn::ExprCall) -> Option<TyTerm> {
        let resolver = self.sig_resolver?;
        let sig = resolver(call)?;
        let mut binders = HashMap::new();
        for tp in &sig.type_params {
            let v = self.ctx.fresh_var();
            binders.insert(tp.clone(), v);
        }
        for (i, arg) in call.args.iter().enumerate() {
            let Some(Some(param_ty)) = sig.params.get(i) else {
                continue;
            };
            let param_term = tyterm_from_syn(param_ty, &binders);
            let arg_term = self.summarize_expr(arg);
            self.ctx.push_constraint(Constraint {
                lhs: arg_term,
                rhs: param_term,
                origin: ConstraintOrigin::Synthetic("call-arg-param"),
            });
        }
        Some(tyterm_from_syn(&sig.ret, &binders))
    }

    /// Bind a closure's parameters into `env`, pinning any annotated
    /// parameter to its declared type. Used by the block walk so that
    /// references to the parameters inside the body resolve to the same
    /// variables (e.g. the fold reducer's `acc`/`v`).
    fn bind_closure_params(&mut self, cl: &syn::ExprClosure) {
        for input in &cl.inputs {
            let (name, ann) = closure_param_ident_and_type(input);
            let Some(name) = name else { continue };
            let v = self.ctx.fresh_var();
            self.env.insert(name, v);
            if let Some(ann) = ann {
                let term = tyterm_from_syn(ann, &self.binders);
                self.ctx.push_constraint(Constraint {
                    lhs: TyTerm::Var(v),
                    rhs: term,
                    origin: ConstraintOrigin::Synthetic("closure-param-ann"),
                });
            }
        }
    }

    /// Walk a block's statements registering `let`-binding variables and
    /// collecting element/owner constraints from initializers and bodies.
    /// This is the block-scoped driver behind
    /// `infer_local_owner_element_from_block`: it must see every `let`
    /// (to put owner locals in `env`), every closure (to bind params and
    /// recurse), and every `recv.push(arg)` (to pin elements) — including
    /// pushes nested inside fold/all reducer closures. `resolver` supplies
    /// the receiver-iterator item type for closure item-params and
    /// `extend` arguments (CodeGen's `infer_iter_item_type_from_expr`),
    /// or `None` when unavailable.
    fn collect_block_constraints(&mut self, stmts: &[syn::Stmt], resolver: &ItemResolver<'_>) {
        for stmt in stmts {
            match stmt {
                syn::Stmt::Local(local) => self.collect_local_binding(local, resolver),
                syn::Stmt::Expr(e, _) => self.collect_expr_constraints(e, resolver),
                _ => {}
            }
        }
    }

    fn collect_local_binding(&mut self, local: &syn::Local, resolver: &ItemResolver<'_>) {
        let (name, ann) = closure_param_ident_and_type(&local.pat);
        if let Some(name) = name {
            let v = self.ctx.fresh_var();
            self.env.insert(name, v);
            if let Some(ann) = ann {
                let term = tyterm_from_syn(ann, &self.binders);
                self.ctx.push_constraint(Constraint {
                    lhs: TyTerm::Var(v),
                    rhs: term,
                    origin: ConstraintOrigin::LetBinding,
                });
            }
            if let Some(init) = &local.init {
                let init_term = self.summarize_expr(&init.expr);
                self.ctx.push_constraint(Constraint {
                    lhs: TyTerm::Var(v),
                    rhs: init_term,
                    origin: ConstraintOrigin::LetBinding,
                });
                self.collect_expr_constraints(&init.expr, resolver);
            }
        } else if let Some(init) = &local.init {
            self.collect_expr_constraints(&init.expr, resolver);
        }
    }

    /// Pin a closure parameter that is the receiver iterator's element
    /// (`.all(|x| …)`, `.fold(init, |acc, x| …)`) to that item type, when
    /// the parameter carries no annotation and `resolver` can supply it.
    fn pin_iter_item_param(
        &mut self,
        receiver: &syn::Expr,
        closure: &syn::Expr,
        param_idx: usize,
        resolver: &ItemResolver<'_>,
    ) {
        let syn::Expr::Closure(cl) = peel_expr(closure) else {
            return;
        };
        let Some(p) = cl.inputs.get(param_idx) else {
            return;
        };
        let (Some(pname), ann) = closure_param_ident_and_type(p) else {
            return;
        };
        if ann.is_some() {
            return; // annotation already pins it
        }
        let Some(&pv) = self.env.get(&pname) else {
            return;
        };
        if let Some(item) = resolver(receiver) {
            self.ctx.push_constraint(Constraint {
                lhs: TyTerm::Var(pv),
                rhs: TyTerm::Concrete(item),
                origin: ConstraintOrigin::Synthetic("iter-item-param"),
            });
        }
    }

    /// Recurse through an expression collecting constraints: bind closure
    /// params, tie fold/rfold reducer accumulators to their init, pin
    /// iterator item-params and `extend` element types from `resolver`,
    /// and record `recv.push(arg)` element constraints. Conservative —
    /// only the shapes that contribute to owner-element inference walked.
    fn collect_expr_constraints(&mut self, expr: &syn::Expr, resolver: &ItemResolver<'_>) {
        match expr {
            syn::Expr::MethodCall(mc) => {
                let method = mc.method.to_string();
                self.collect_expr_constraints(&mc.receiver, resolver);
                for a in &mc.args {
                    self.collect_expr_constraints(a, resolver);
                }
                self.record_owner_push_constraint(mc);
                // `recv.extend(it)` — the receiver owner's element type is
                // the item type of the extended iterator.
                if method == "extend"
                    && mc.args.len() == 1
                    && let Some(rv) = self.receiver_env_var(&mc.receiver)
                    && let Some(item) = resolver(&mc.args[0])
                {
                    self.ctx.push_constraint(Constraint {
                        lhs: TyTerm::Var(rv),
                        rhs: TyTerm::App {
                            head: "Vec".to_string(),
                            args: vec![TyTerm::Concrete(item)],
                        },
                        origin: ConstraintOrigin::Synthetic("owner-extend"),
                    });
                }
                // Iterator-consuming closures: pin the element parameter to
                // the receiver's item type. `fold`/`rfold` take `|acc, x|`
                // (item is param 1); `all`/`any`/`for_each` take `|x|`.
                match method.as_str() {
                    "fold" | "rfold" if mc.args.len() == 2 => {
                        self.pin_iter_item_param(&mc.receiver, &mc.args[1], 1, resolver);
                    }
                    "all" | "any" | "for_each" if mc.args.len() == 1 => {
                        self.pin_iter_item_param(&mc.receiver, &mc.args[0], 0, resolver);
                    }
                    _ => {}
                }
                // fold/rfold: the reducer's first parameter (accumulator)
                // shares the init's type. The closure's params were bound
                // when its arg was recursed above, so tie it now.
                if matches!(method.as_str(), "fold" | "rfold")
                    && mc.args.len() == 2
                    && let syn::Expr::Closure(cl) = peel_expr(&mc.args[1])
                    && let Some(p0) = cl.inputs.first()
                    && let (Some(p0name), _) = closure_param_ident_and_type(p0)
                    && let Some(&v0) = self.env.get(&p0name)
                {
                    let init_term = self.summarize_expr(&mc.args[0]);
                    self.ctx.push_constraint(Constraint {
                        lhs: TyTerm::Var(v0),
                        rhs: init_term,
                        origin: ConstraintOrigin::Synthetic("fold-acc-init"),
                    });
                }
            }
            syn::Expr::Closure(cl) => {
                self.bind_closure_params(cl);
                self.collect_expr_constraints(&cl.body, resolver);
            }
            syn::Expr::Block(b) => self.collect_block_constraints(&b.block.stmts, resolver),
            syn::Expr::If(e) => {
                self.collect_expr_constraints(&e.cond, resolver);
                self.collect_block_constraints(&e.then_branch.stmts, resolver);
                if let Some((_, els)) = &e.else_branch {
                    self.collect_expr_constraints(els, resolver);
                }
            }
            syn::Expr::Match(e) => {
                self.collect_expr_constraints(&e.expr, resolver);
                for arm in &e.arms {
                    self.collect_expr_constraints(&arm.body, resolver);
                }
            }
            syn::Expr::ForLoop(e) => {
                self.collect_expr_constraints(&e.expr, resolver);
                self.collect_block_constraints(&e.body.stmts, resolver);
            }
            syn::Expr::While(e) => {
                self.collect_expr_constraints(&e.cond, resolver);
                self.collect_block_constraints(&e.body.stmts, resolver);
            }
            syn::Expr::Loop(e) => self.collect_block_constraints(&e.body.stmts, resolver),
            syn::Expr::Call(c) => {
                self.record_newtype_field_constraint(c);
                for a in &c.args {
                    self.collect_expr_constraints(a, resolver);
                }
            }
            syn::Expr::Paren(e) => self.collect_expr_constraints(&e.expr, resolver),
            syn::Expr::Group(e) => self.collect_expr_constraints(&e.expr, resolver),
            syn::Expr::Reference(e) => self.collect_expr_constraints(&e.expr, resolver),
            syn::Expr::Tuple(t) => {
                for e in &t.elems {
                    self.collect_expr_constraints(e, resolver);
                }
            }
            _ => {}
        }
    }

    /// The `env` variable for a bare-identifier receiver, if bound.
    fn receiver_env_var(&self, receiver: &syn::Expr) -> Option<TyVarId> {
        let syn::Expr::Path(p) = receiver else {
            return None;
        };
        if p.qself.is_some()
            || p.path.segments.len() != 1
            || !matches!(p.path.segments[0].arguments, syn::PathArguments::None)
        {
            return None;
        }
        self.env.get(&p.path.segments[0].ident.to_string()).copied()
    }
}

fn block_tail_expr(block: &syn::Block) -> Option<syn::Expr> {
    let last = block.stmts.iter().last()?;
    if let syn::Stmt::Expr(e, None) = last {
        return Some(e.clone());
    }
    None
}

/// Recognize an owner constructor call with no element argument —
/// `Vec::new()`, `Vec::with_capacity(n)`, `Vec::default()`, and their
/// std/alloc-qualified spellings — returning the owner's canonical
/// head (`"Vec"`). Only growable owners whose element type must be
/// recovered from later usage are recognized; everything else is
/// `None`. The element-bearing constructors (`vec![x]`, `Vec::from`)
/// don't need recovery and are intentionally excluded.
pub(crate) fn owner_constructor_head(call: &syn::ExprCall) -> Option<String> {
    let syn::Expr::Path(p) = &*call.func else {
        return None;
    };
    if p.qself.is_some() {
        return None;
    }
    let segs: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
    if segs.len() < 2 {
        return None;
    }
    let ctor = segs.last().map(String::as_str)?;
    if !matches!(ctor, "new" | "new_" | "with_capacity" | "default") {
        return None;
    }
    match segs[segs.len() - 2].as_str() {
        "Vec" => Some("Vec".to_string()),
        _ => None,
    }
}

/// Best-effort concrete type of a literal: suffixed integer/float
/// literals (`1u8`, `2.0f64`) carry their type; `bool`/`char`/string
/// literals are known. Unsuffixed numeric literals are ambiguous (Rust
/// defaults them by context) so they return `None` — the solver leaves
/// the variable free rather than guessing `i32`.
fn lit_tyterm(lit: &syn::Lit) -> Option<TyTerm> {
    let parsed: Type = match lit {
        syn::Lit::Bool(_) => syn::parse_str("bool").ok()?,
        syn::Lit::Char(_) => syn::parse_str("char").ok()?,
        syn::Lit::Str(_) => syn::parse_str("& str").ok()?,
        syn::Lit::Int(i) if !i.suffix().is_empty() => syn::parse_str(i.suffix()).ok()?,
        syn::Lit::Float(f) if !f.suffix().is_empty() => syn::parse_str(f.suffix()).ok()?,
        _ => return None,
    };
    Some(TyTerm::Concrete(parsed))
}

/// Strip `(…)` / `{ … }`-group wrappers from an expression so the
/// inner closure / call is reachable.
fn peel_expr(expr: &syn::Expr) -> &syn::Expr {
    let mut e = expr;
    loop {
        match e {
            syn::Expr::Paren(p) => e = &p.expr,
            syn::Expr::Group(g) => e = &g.expr,
            _ => return e,
        }
    }
}

/// Extract a binding pattern's name and optional type annotation. Used
/// for both closure parameters and `let` patterns. `|mut acc, v: I::Item|`
/// yields `("acc", None)` and `("v", Some(I::Item))`; `let x: T` yields
/// `("x", Some(T))`.
fn closure_param_ident_and_type(pat: &syn::Pat) -> (Option<String>, Option<&Type>) {
    match pat {
        syn::Pat::Ident(pi) => (Some(pi.ident.to_string()), None),
        syn::Pat::Type(pt) => {
            let name = match pt.pat.as_ref() {
                syn::Pat::Ident(pi) => Some(pi.ident.to_string()),
                _ => None,
            };
            (name, Some(pt.ty.as_ref()))
        }
        _ => (None, None),
    }
}

/// Render a fully-resolved `TyTerm` back to a `syn::Type`. `Concrete`
/// passes through; `App("&", [t])` becomes `&t`; other `App`s rebuild
/// `head<args…>`. Returns `None` if any sub-term is still a free
/// variable (underdetermined) or can't be re-parsed — the caller then
/// falls back to today's heuristic emit.
pub(crate) fn tyterm_to_syn_type(term: &TyTerm) -> Option<Type> {
    match term {
        TyTerm::Var(_) => None,
        TyTerm::Concrete(t) => Some(t.clone()),
        TyTerm::App { head, args } => {
            if head == "&" && args.len() == 1 {
                let inner = tyterm_to_syn_type(&args[0])?;
                return syn::parse2(quote::quote!(&#inner)).ok();
            }
            if head == "&mut" && args.len() == 1 {
                let inner = tyterm_to_syn_type(&args[0])?;
                return syn::parse2(quote::quote!(&mut #inner)).ok();
            }
            if (head == "*mut" || head == "*const") && args.len() == 1 {
                let inner = tyterm_to_syn_type(&args[0])?;
                let toks = if head == "*mut" {
                    quote::quote!(*mut #inner)
                } else {
                    quote::quote!(*const #inner)
                };
                return syn::parse2(toks).ok();
            }
            if head == "tuple" {
                let elems: Vec<Type> = args
                    .iter()
                    .map(tyterm_to_syn_type)
                    .collect::<Option<Vec<_>>>()?;
                return syn::parse2(quote::quote!( ( #(#elems),* ) )).ok();
            }
            let arg_types: Vec<Type> = args
                .iter()
                .map(tyterm_to_syn_type)
                .collect::<Option<Vec<_>>>()?;
            // Reject heads that aren't a plain type-path identifier
            // (e.g. structural heads like "tuple"/"&[]") — they don't
            // re-render as `head<...>`.
            if head.is_empty()
                || !head
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
            {
                return None;
            }
            let head_path: syn::Path = syn::parse_str(head).ok()?;
            if arg_types.is_empty() {
                return syn::parse2(quote::quote!(#head_path)).ok();
            }
            syn::parse2(quote::quote!(#head_path<#(#arg_types),*>)).ok()
        }
    }
}

/// Infer the element type of an owner accumulator threaded through a
/// `fold`/`rfold` reducer closure `|acc, item| { … ; acc }`.
///
/// Models the accumulator (the reducer's first parameter) as
/// `App(owner_head, [?elem])`, binds every parameter to a variable,
/// pins typed parameters to their annotation, seeds the element
/// parameter from `item_type_hint` (the receiver iterator's item type)
/// when it carries no annotation, then collects `acc.push(x)`-style
/// constraints from the body. After solving, returns the resolved
/// element `syn::Type`, or `None` when the engine can't pin it.
///
/// This is the Vec-accumulator instance of the §13 inference plan:
/// Rust resolves `fold(Vec::new(), |acc, v| { acc.push(v); acc })`
/// by unifying the empty vec's element with the pushed value; we do
/// the same so emit can produce `rusty::Vec<Elem>` with nothing left
/// for the C++ compiler to deduce.
pub(crate) fn infer_owner_accumulator_element_from_reducer(
    reducer: &syn::ExprClosure,
    owner_head: &str,
    item_type_hint: Option<&Type>,
) -> Option<Type> {
    if reducer.inputs.is_empty() {
        return None;
    }
    let mut ctx = InferenceContext::new();
    let elem = ctx.fresh_var();
    let mut have_acc = false;
    {
        let mut c = ConstraintCollector::new(&mut ctx);
        for (idx, input) in reducer.inputs.iter().enumerate() {
            let (name, ann) = closure_param_ident_and_type(input);
            let Some(name) = name else {
                continue;
            };
            let v = c.ctx.fresh_var();
            c.env.insert(name, v);
            if idx == 0 {
                have_acc = true;
                c.ctx.push_constraint(Constraint {
                    lhs: TyTerm::Var(v),
                    rhs: TyTerm::App {
                        head: owner_head.to_string(),
                        args: vec![TyTerm::Var(elem)],
                    },
                    origin: ConstraintOrigin::Synthetic("fold-acc"),
                });
            }
            if let Some(ann) = ann {
                let term = tyterm_from_syn(ann, &c.binders);
                c.ctx.push_constraint(Constraint {
                    lhs: TyTerm::Var(v),
                    rhs: term,
                    origin: ConstraintOrigin::Synthetic("param-ann"),
                });
            } else if idx == 1 {
                if let Some(hint) = item_type_hint {
                    c.ctx.push_constraint(Constraint {
                        lhs: TyTerm::Var(v),
                        rhs: TyTerm::Concrete(hint.clone()),
                        origin: ConstraintOrigin::Synthetic("fold-item-hint"),
                    });
                }
            }
        }
        if !have_acc {
            return None;
        }
        c.collect_owner_method_usage(&reducer.body);
    }
    if !ctx.solve().is_empty() {
        return None;
    }
    let term = ctx.resolve(elem)?;
    tyterm_to_syn_type(&term)
}

/// Infer the element type of a `Vec`-owner local `target` declared in
/// `stmts` (e.g. `let mut acc = Vec::new();`) from how it is used later
/// in the same block — `acc.push(x)`, including pushes nested inside a
/// fold/all reducer closure where the pushed value mentions other
/// inferred bindings (`acc.push((other_acc.clone(), v.clone()))`).
///
/// Models the whole block as one constraint set so interdependent
/// inferences resolve together (the sibling fold accumulator's element
/// is pinned by its own `push`, then feeds this local's tuple element).
/// Returns the resolved *element* `syn::Type`, or `None` when the engine
/// can't pin it — the caller then leaves today's behavior in place.
pub(crate) fn infer_local_owner_element_from_block(
    stmts: &[syn::Stmt],
    target: &str,
    resolver: &ItemResolver<'_>,
    extra: OwnerElementResolvers<'_>,
) -> Option<Type> {
    let mut ctx = InferenceContext::new();
    let target_var = {
        let mut c = ConstraintCollector::new(&mut ctx);
        c.field_resolver = extra.field;
        c.sig_resolver = extra.sig;
        c.method_resolver = extra.method;
        c.collect_block_constraints(stmts, resolver);
        c.env.get(target).copied()
    }?;
    if !ctx.solve().is_empty() {
        return None;
    }
    match ctx.resolve(target_var)? {
        TyTerm::App { head, args } if head == "Vec" && args.len() == 1 => {
            tyterm_to_syn_type(&args[0])
        }
        _ => None,
    }
}

impl<'ast> Visit<'ast> for ConstraintCollector<'_, '_> {
    fn visit_local(&mut self, local: &'ast syn::Local) {
        self.visit_local_for_constraints(local);
        syn::visit::visit_local(self, local);
    }
}

// ============================================================
// Phase 4b: localized query interface for emit sites.
//
// Per the rationale in §13.7, emit sites call into the engine to
// ask focused questions like "what's the unified type of these
// two ternary arms?" rather than walking a function-wide map.
// This keeps the engine cheap to consult from anywhere in emit
// and avoids the AST-identity bookkeeping that a function-wide
// constraint store would need (proc_macro2 spans from
// `syn::parse_str` are call-site placeholders, so they can't
// uniquely key arbitrary `Expr` nodes).
//
// The query helpers spin up a transient `InferenceContext`,
// feed it the constraints they need, solve, and return. Failure
// (no solution / underdetermined) is reported as `None` so the
// caller can fall back to today's heuristic emit.
// ============================================================

/// Given two expressions that must share a common type (the arms
/// of an `if`/`else` or `?:`), return the unified term if the
/// engine can compute one, or `None` to signal "fall back to
/// local CTAD".
///
/// Today this is the seed implementation for the Either case in
/// §13.3. It models each arm as a fresh variable, lets
/// `summarize_expr` decompose any nested structure it understands
/// (recursively if/else, match, blocks), and runs the solver.
/// When the arms are simple identifier references (the common
/// case after the `Either{Left{...}}` shape gets desugared), the
/// engine has no concrete type to anchor on and returns `None`;
/// Phase 4c will plug in the variant-constructor-recognition
/// logic that turns `Either_Left{e}` into `App("Either",
/// [typeof(e), ?R])`.
pub(crate) fn infer_branch_merge(
    arm_a: &syn::Expr,
    arm_b: &syn::Expr,
) -> Option<TyTerm> {
    let mut ctx = InferenceContext::new();
    let term_a;
    let term_b;
    {
        let mut c = ConstraintCollector::new(&mut ctx);
        term_a = c.summarize_expr(arm_a);
        term_b = c.summarize_expr(arm_b);
    }
    let merge = ctx.fresh_var();
    ctx.push_constraint(Constraint {
        lhs: TyTerm::Var(merge),
        rhs: term_a,
        origin: ConstraintOrigin::BranchMerge,
    });
    ctx.push_constraint(Constraint {
        lhs: TyTerm::Var(merge),
        rhs: term_b,
        origin: ConstraintOrigin::BranchMerge,
    });
    let errors = ctx.solve();
    if !errors.is_empty() {
        return None;
    }
    ctx.resolve(merge)
}

/// Render a resolved `TyTerm` back to a C++ type string. Returns
/// `None` if any sub-term is still a free variable — callers
/// treat that as "underdetermined, fall back to local CTAD".
///
/// Concrete `syn::Type` terms aren't yet mapped through the
/// codegen's full type-mapping pipeline (that lives on `CodeGen`,
/// outside this module); this helper exists so module tests can
/// assert on the engine's output and so Phase 4c can render the
/// `App` skeleton while substituting concrete types via the
/// caller's mapper.
pub(crate) fn render_tyterm_for_cpp(term: &TyTerm) -> Option<String> {
    match term {
        TyTerm::Var(_) => None,
        TyTerm::Concrete(t) => Some(render_concrete(t)),
        TyTerm::App { head, args } => {
            let rendered: Vec<String> = args
                .iter()
                .map(render_tyterm_for_cpp)
                .collect::<Option<Vec<_>>>()?;
            Some(format!("{}<{}>", head, rendered.join(", ")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn ty(s: &str) -> Type {
        syn::parse_str(s).unwrap()
    }

    fn render_ty(t: &Type) -> String {
        quote::quote!(#t).to_string().replace(' ', "")
    }

    #[test]
    fn solve_owner_type_args_forward_single_param() {
        // `UnsafeCell::new(value)` where `new<T>(value: T) -> UnsafeCell<T>`,
        // called with a `u8` argument → owner arg `T = u8`.
        let out = solve_owner_type_args(
            &["T".to_string()],
            &[Some(ty("T"))],
            Some(&ty("UnsafeCell<T>")),
            &[Some(ty("u8"))],
            None,
        )
        .expect("should solve T from the argument");
        assert_eq!(out.len(), 1);
        assert_eq!(render_ty(&out[0]), "u8");
    }

    #[test]
    fn solve_owner_type_args_forward_passthrough_enclosing_type_param() {
        // The argument's type is itself the enclosing function's type param `T`
        // (e.g. `UnsafeCell::new(value)` inside `impl<T> … { fn f(&self, value: T) }`).
        // The owner arg should come back as the literal `T` (carried as Concrete).
        let out = solve_owner_type_args(
            &["E".to_string()],
            &[Some(ty("E"))],
            Some(&ty("UnsafeCell<E>")),
            &[Some(ty("T"))],
            None,
        )
        .expect("should solve owner param from the enclosing type param");
        assert_eq!(render_ty(&out[0]), "T");
    }

    #[test]
    fn solve_owner_type_args_backward_from_expected_path_type() {
        // A zero-arg factory whose `T` is determined by the expected result:
        // `x: Box<i32> = make_box()` where `make_box<T>() -> Box<T>` → `T = i32`.
        let out = solve_owner_type_args(
            &["T".to_string()],
            &[],
            Some(&ty("Box<T>")),
            &[],
            Some(&ty("Box<i32>")),
        )
        .expect("should solve T from the expected return type");
        assert_eq!(render_ty(&out[0]), "i32");
    }

    #[test]
    fn solve_owner_type_args_unsolvable_returns_none() {
        // Nothing pins `T` (no arg type, no expected) → None, so the caller
        // falls back to its heuristic rather than emitting a bad turbofish.
        assert!(
            solve_owner_type_args(&["T".to_string()], &[None], None, &[None], None).is_none()
        );
    }

    #[test]
    fn solve_owner_type_args_backward_through_pointer_return() {
        // `invalid_mut<T>(addr: usize) -> *mut T`: T appears ONLY in the return
        // (the `usize` arg doesn't mention it), so it must be solved from the
        // expected pointer type's element. Requires the `*mut` structural rule.
        let out = solve_owner_type_args(
            &["T".to_string()],
            &[Some(ty("usize"))],
            Some(&ty("*mut T")),
            &[Some(ty("usize"))],
            Some(&ty("*mut i32")),
        )
        .expect("should solve T from the expected pointer element");
        assert_eq!(render_ty(&out[0]), "i32");
    }

    #[test]
    fn solve_owner_type_args_backward_ignores_pointer_constness() {
        // `invalid_mut<T>(usize) -> *mut T` solved against a `*const i32` expected
        // (the sibling branch resolved to a const pointer) still yields T = i32 —
        // constness never changes which type fills the element slot.
        let out = solve_owner_type_args(
            &["T".to_string()],
            &[Some(ty("usize"))],
            Some(&ty("*mut T")),
            &[Some(ty("usize"))],
            Some(&ty("*const i32")),
        )
        .expect("pointer constness mismatch should not block the element solve");
        assert_eq!(render_ty(&out[0]), "i32");
    }

    #[test]
    fn solve_owner_type_args_backward_through_mut_reference() {
        // Same for `&mut T` returns — the qualifier must match (a `&mut` return
        // does not unify with a `*mut` expected, etc.).
        let out = solve_owner_type_args(
            &["T".to_string()],
            &[],
            Some(&ty("&mut T")),
            &[],
            Some(&ty("&mut u64")),
        )
        .expect("should solve T from the expected &mut element");
        assert_eq!(render_ty(&out[0]), "u64");
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

    fn norm(t: &Type) -> String {
        use quote::ToTokens;
        t.to_token_stream().to_string().replace(' ', "")
    }

    #[test]
    fn fold_reducer_pins_vec_element_from_annotated_param() {
        // `|mut acc, v: i32| { acc.push(v); acc }` — element is i32.
        let reducer: syn::ExprClosure =
            parse_quote!(|mut acc, v: i32| { acc.push(v); acc });
        let elem = infer_owner_accumulator_element_from_reducer(&reducer, "Vec", None)
            .expect("element should resolve from annotated param");
        assert_eq!(norm(&elem), "i32");
    }

    #[test]
    fn fold_reducer_pins_vec_element_from_item_hint_when_unannotated() {
        // `|mut acc, n| { acc.push(n); acc }` — element comes from the
        // supplied iterator item-type hint (the receiver's item type).
        let reducer: syn::ExprClosure = parse_quote!(|mut acc, n| { acc.push(n); acc });
        let hint: Type = parse_quote!(u64);
        let elem = infer_owner_accumulator_element_from_reducer(&reducer, "Vec", Some(&hint))
            .expect("element should resolve from item hint");
        assert_eq!(norm(&elem), "u64");
    }

    #[test]
    fn block_local_element_from_fold_push_tuple_of_clones() {
        // The `parameters_from_fold` shape: a sibling local pushed inside
        // the fold reducer with a tuple of clones of interdependent
        // bindings resolves to `(Vec<i32>, i32)`.
        let block: syn::Block = parse_quote!({
            let mut params = Vec::new();
            let _r = it.fold(Vec::new(), |mut acc, v: i32| {
                params.push((acc.clone(), v.clone()));
                acc.push(v);
                acc
            });
        });
        let resolver: &ItemResolver = &|_e: &syn::Expr| None;
        let elem = infer_local_owner_element_from_block(&block.stmts, "params", resolver, OwnerElementResolvers::default())
            .expect("params element should resolve");
        assert_eq!(norm(&elem), "(Vec<i32>,i32)");
    }

    #[test]
    fn block_local_element_from_all_closure_uses_item_resolver() {
        // `.all(|x| { params.push(x.clone()); … })` — the closure param is
        // unannotated, so the element comes from the item resolver.
        let block: syn::Block = parse_quote!({
            let mut params = Vec::new();
            let _r = it.all(|x| {
                params.push(x.clone());
                true
            });
        });
        let resolver: &ItemResolver = &|_e: &syn::Expr| Some(parse_quote!(u8));
        let elem = infer_local_owner_element_from_block(&block.stmts, "params", resolver, OwnerElementResolvers::default())
            .expect("params element should resolve from item resolver");
        assert_eq!(norm(&elem), "u8");
    }

    #[test]
    fn block_local_element_from_newtype_field_consumer() {
        // §13.14 C1: a Vec accumulated then handed to a single-field newtype
        // wrapper (`Ok(ByteBuf::from(bytes))`) gets its element from the
        // wrapper's sole field type, supplied by the FieldResolver — even when
        // the push value (`b`, the while-let payload) is circular/unbound.
        let block: syn::Block = parse_quote!({
            let mut bytes = Vec::with_capacity(0);
            while let Some(b) = visitor.next_element()? {
                bytes.push(b);
            }
            Ok(ByteBuf::from(bytes))
        });
        let resolver: &ItemResolver = &|_e: &syn::Expr| None;
        let field_resolver: &FieldResolver = &|name: &str| {
            if name == "ByteBuf" {
                Some(parse_quote!(Vec<u8>))
            } else {
                None
            }
        };
        let elem = infer_local_owner_element_from_block(
            &block.stmts,
            "bytes",
            resolver,
            OwnerElementResolvers {
                field: Some(field_resolver),
                ..Default::default()
            },
        )
        .expect("bytes element should resolve from ByteBuf's Vec<u8> field");
        assert_eq!(norm(&elem), "u8");
    }

    #[test]
    fn block_local_element_newtype_consumer_inert_without_field_resolver() {
        // The same block, but with no FieldResolver, must NOT resolve — the
        // push value is circular and there is no other element witness, so the
        // engine returns None and the caller keeps today's behavior. Guards
        // that the C1 rule fires only via the resolver, never by accident.
        let block: syn::Block = parse_quote!({
            let mut bytes = Vec::with_capacity(0);
            while let Some(b) = visitor.next_element()? {
                bytes.push(b);
            }
            Ok(ByteBuf::from(bytes))
        });
        let resolver: &ItemResolver = &|_e: &syn::Expr| None;
        assert!(
            infer_local_owner_element_from_block(&block.stmts, "bytes", resolver, OwnerElementResolvers::default()).is_none()
        );
    }

    #[test]
    fn block_local_element_from_call_return_via_signature() {
        // §13.14 C2: a Vec element pushed as a function-call result resolves to
        // the callee's (non-generic) return type via the SignatureResolver.
        let block: syn::Block = parse_quote!({
            let mut v = Vec::new();
            v.push(make_widget());
        });
        let resolver: &ItemResolver = &|_e: &syn::Expr| None;
        let sig_resolver: &SignatureResolver = &|call: &syn::ExprCall| {
            let syn::Expr::Path(p) = call.func.as_ref() else {
                return None;
            };
            match p.path.segments.last()?.ident.to_string().as_str() {
                "make_widget" => Some(FnSig {
                    type_params: vec![],
                    params: vec![],
                    ret: parse_quote!(Widget),
                }),
                _ => None,
            }
        };
        let elem =
            infer_local_owner_element_from_block(&block.stmts, "v", resolver, OwnerElementResolvers { sig: Some(sig_resolver), ..Default::default() })
                .expect("v element should resolve from make_widget()'s return type");
        assert_eq!(norm(&elem), "Widget");
    }

    #[test]
    fn block_local_element_from_generic_call_instantiates_from_arg() {
        // §13.14 C2: a generic callee `fn identity<T>(x: T) -> T` monomorphizes
        // per call — the argument's type (`u8`) pins `T`, so the pushed result
        // is `u8` and the Vec element resolves to `u8`.
        let block: syn::Block = parse_quote!({
            let mut v = Vec::new();
            v.push(identity(7u8));
        });
        let resolver: &ItemResolver = &|_e: &syn::Expr| None;
        let sig_resolver: &SignatureResolver = &|call: &syn::ExprCall| {
            let syn::Expr::Path(p) = call.func.as_ref() else {
                return None;
            };
            match p.path.segments.last()?.ident.to_string().as_str() {
                "identity" => Some(FnSig {
                    type_params: vec!["T".to_string()],
                    params: vec![Some(parse_quote!(T))],
                    ret: parse_quote!(T),
                }),
                _ => None,
            }
        };
        let elem =
            infer_local_owner_element_from_block(&block.stmts, "v", resolver, OwnerElementResolvers { sig: Some(sig_resolver), ..Default::default() })
                .expect("v element should resolve via generic identity instantiation");
        assert_eq!(norm(&elem), "u8");
    }

    #[test]
    fn block_local_element_call_return_inert_without_sig_resolver() {
        // The same call-return push must stay unresolved with no
        // SignatureResolver — the C2 rule fires only via the resolver.
        let block: syn::Block = parse_quote!({
            let mut v = Vec::new();
            v.push(make_widget());
        });
        let resolver: &ItemResolver = &|_e: &syn::Expr| None;
        assert!(
            infer_local_owner_element_from_block(&block.stmts, "v", resolver, OwnerElementResolvers::default()).is_none()
        );
    }

    #[test]
    fn block_local_element_from_method_call_via_resolver() {
        // §13.14 C2 (method form): a Vec element pushed as a method-call result
        // resolves to the method's return type via the MethodResolver — the
        // common `v.push(x.make_item())` element source.
        let block: syn::Block = parse_quote!({
            let mut v = Vec::new();
            v.push(x.make_item());
        });
        let resolver: &ItemResolver = &|_e: &syn::Expr| None;
        let method_resolver: &MethodResolver = &|mc: &syn::ExprMethodCall| {
            if mc.method.to_string() == "make_item" {
                Some(parse_quote!(Widget))
            } else {
                None
            }
        };
        let elem = infer_local_owner_element_from_block(
            &block.stmts,
            "v",
            resolver,
            OwnerElementResolvers {
                method: Some(method_resolver),
                ..Default::default()
            },
        )
        .expect("v element should resolve from x.make_item()'s return type");
        assert_eq!(norm(&elem), "Widget");
    }

    #[test]
    fn block_local_element_unresolved_without_usage_is_none() {
        // A bare `Vec::new()` with no element-revealing usage stays None
        // so the caller keeps today's (now hard-failing) behavior.
        let block: syn::Block = parse_quote!({
            let mut params = Vec::new();
            let _ = params;
        });
        let resolver: &ItemResolver = &|_e: &syn::Expr| None;
        assert!(infer_local_owner_element_from_block(&block.stmts, "params", resolver, OwnerElementResolvers::default()).is_none());
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

    // ============================================================
    // Phase 4b tests — localized query interface used by emit
    // sites in Phase 4c.
    // ============================================================

    #[test]
    fn infer_branch_merge_returns_none_for_underspecified_arms() {
        // Bare identifier arms give the engine nothing to anchor
        // on; it should signal `None` so emit falls back rather
        // than emitting an invented type.
        let arm_a: syn::Expr = syn::parse_str("foo").unwrap();
        let arm_b: syn::Expr = syn::parse_str("bar").unwrap();
        assert!(infer_branch_merge(&arm_a, &arm_b).is_none());
    }

    #[test]
    fn infer_branch_merge_unifies_nested_if_else_into_one_var() {
        // `if … { a } else { b }` as one of the arms — the inner
        // if/else builds its own merge variable, and the outer
        // query unifies the two arms through the existing
        // structure. The result is still a Var (no concrete type
        // contributed), so render fails; the test asserts that
        // the engine *attempted* the unification cleanly (no
        // solver errors).
        let arm_a: syn::Expr = syn::parse_str("if c { x } else { y }").unwrap();
        let arm_b: syn::Expr = syn::parse_str("z").unwrap();
        let merge = infer_branch_merge(&arm_a, &arm_b);
        // No concrete contribution → Var → None from resolve.
        assert!(merge.is_none());
    }

    #[test]
    fn render_tyterm_concrete_returns_token_string() {
        let ty: syn::Type = syn::parse_str("i32").unwrap();
        assert_eq!(
            render_tyterm_for_cpp(&TyTerm::Concrete(ty)).as_deref(),
            Some("i32")
        );
    }

    #[test]
    fn render_tyterm_app_nests_arguments() {
        let ty: syn::Type = syn::parse_str("u8").unwrap();
        let inner = TyTerm::App {
            head: "Vec".to_string(),
            args: vec![TyTerm::Concrete(ty)],
        };
        let outer = TyTerm::App {
            head: "Either".to_string(),
            args: vec![inner.clone(), inner.clone()],
        };
        assert_eq!(
            render_tyterm_for_cpp(&outer).as_deref(),
            Some("Either<Vec<u8>, Vec<u8>>")
        );
    }

    #[test]
    fn render_tyterm_with_free_var_returns_none() {
        let term = TyTerm::App {
            head: "Either".to_string(),
            args: vec![TyTerm::Var(TyVarId(0)), TyTerm::Concrete(syn::parse_str("u64").unwrap())],
        };
        assert!(render_tyterm_for_cpp(&term).is_none());
    }

    // ============================================================
    // Phase 4c-i — variant-constructor recognition.
    // ============================================================

    #[test]
    fn variant_constructor_either_left_pins_first_param() {
        let mut ctx = InferenceContext::new();
        let mut c = ConstraintCollector::new(&mut ctx);
        let call: syn::ExprCall = syn::parse_str("Either::Left(x)").unwrap();
        let term = c
            .recognize_variant_constructor_call(&call)
            .expect("Either::Left should be recognized");
        match term {
            TyTerm::App { head, args } => {
                assert_eq!(head, "Either");
                assert_eq!(args.len(), 2);
                // arg[0] is the L position — should be the arg's
                // fresh var (not a concrete type yet, since `x` is
                // a bare ident).
                assert!(matches!(args[0], TyTerm::Var(_)));
                assert!(matches!(args[1], TyTerm::Var(_)));
            }
            other => panic!("expected Either App; got {:?}", other),
        }
    }

    #[test]
    fn variant_constructor_either_right_pins_second_param() {
        let mut ctx = InferenceContext::new();
        let mut c = ConstraintCollector::new(&mut ctx);
        let call: syn::ExprCall = syn::parse_str("Either::Right(y)").unwrap();
        let term = c
            .recognize_variant_constructor_call(&call)
            .expect("Either::Right should be recognized");
        match term {
            TyTerm::App { head, args } => {
                assert_eq!(head, "Either");
                assert_eq!(args.len(), 2);
                assert!(matches!(args[0], TyTerm::Var(_)));
                assert!(matches!(args[1], TyTerm::Var(_)));
            }
            other => panic!("expected Either App; got {:?}", other),
        }
    }

    #[test]
    fn variant_constructor_unrecognized_returns_none() {
        let mut ctx = InferenceContext::new();
        let mut c = ConstraintCollector::new(&mut ctx);
        let call: syn::ExprCall = syn::parse_str("Foo::Bar(x)").unwrap();
        assert!(c.recognize_variant_constructor_call(&call).is_none());
    }

    #[test]
    fn infer_branch_merge_either_ternary_resolves_through_constructors() {
        // The HEADLINE test: this is the §13.3 case end-to-end
        // through the public query API. Two ternary arms each
        // wrap an opaque value in a different Either constructor.
        // Today the arm values themselves remain free variables
        // (we don't reach back into the source to type `x` and
        // `y`), but the Either *constructor* tags resolve the
        // outer `App("Either", [...])` shape. The merge variable
        // unifies the two App terms — the L slot gets pinned by
        // arm A's contribution, and the R slot by arm B's. With
        // fully concrete arg types this would produce a complete
        // `Either<TypeOfX, TypeOfY>`; with bare-ident args we
        // produce `Either<?vx, ?vy>` where each slot is bound to
        // exactly ONE variable across both arms (proves the
        // unification worked).
        let arm_a: syn::Expr = syn::parse_str("Either::Left(x)").unwrap();
        let arm_b: syn::Expr = syn::parse_str("Either::Right(y)").unwrap();
        let merge = infer_branch_merge(&arm_a, &arm_b)
            .expect("Either ternary should produce a unified App term");
        match merge {
            TyTerm::App { head, args } => {
                assert_eq!(head, "Either");
                assert_eq!(args.len(), 2);
                // Both slots are still variables (since we don't
                // type `x`/`y`), but they should be DIFFERENT
                // variables — one corresponds to arm A's L
                // contribution, the other to arm B's R.
                let l = match args[0] {
                    TyTerm::Var(v) => v,
                    ref other => panic!("L slot should be Var; got {:?}", other),
                };
                let r = match args[1] {
                    TyTerm::Var(v) => v,
                    ref other => panic!("R slot should be Var; got {:?}", other),
                };
                assert_ne!(
                    l, r,
                    "L and R should be distinct variables — Either<L, R>"
                );
            }
            other => panic!("expected Either App; got {:?}", other),
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
