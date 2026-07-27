//! Guard flow: the one place that decides whether an expression may yield a
//! GUARD OBJECT that Rust would see through implicitly.
//!
//! # The problem this module owns
//!
//! Rust's `RefCell::borrow_mut()` returns `RefMut<T>`, `Mutex::lock()` returns
//! (a `Result` of) `MutexGuard<T>` — and then *autoderef* makes the guard
//! invisible: `g.push(v)`, `*g = v`, `g.len()` all deref implicitly, so the
//! source usually contains no `*` at all. The C++ runtime mirrors the guards
//! as objects with `operator*`/`operator->`, but C++ has no autoderef: every
//! deref the Rust compiler inserted silently has to be *invented* by the
//! emitter at the consumption site.
//!
//! When the receiver's type is KNOWN (a visible `RefCell` field), the typed
//! lowerings elsewhere emit a direct `->` and none of this is needed. The gap
//! — and the source of issues #32, #34, #35 and their siblings — is the
//! UNTYPED tier: a receiver the inference cannot see through (generic, or
//! concrete but out of view). There the emitter must classify by the VALUE'S
//! FLOW, not by one syntactic shape, and adapt through the tolerant runtime
//! (`rusty::deref_call`, `rusty::detail::deref_if_pointer_like`), which
//! unwraps a guard and passes a plain reference through unchanged — correct
//! whichever the value turns out to be.
//!
//! # Doctrine
//!
//! 1. **Classify flow, not shape.** A guard reaches its consumer directly
//!    (`x.borrow_mut().m()`), through `unwrap()`/`expect()` (`lock()`),
//!    through a local binding (`let g = ...`), or through the tail of a
//!    block/`if`/`match` used as an initializer. Every flow path must give
//!    the same answer; a per-site syntactic predicate is how the class kept
//!    reopening one issue at a time.
//! 2. **Fire only on untypable receivers.** If `infer_simple_expr_type` can
//!    see the receiver, the typed tier already handles it — and emits better
//!    code. (An unconditional version regressed three golden tests.)
//! 3. **Adapt at the consumption site, once.** Method receivers go through
//!    the dispatch ladder; place/value uses through
//!    `deref_if_pointer_like`; free-helper arguments through
//!    [`CodeGen::maybe_guard_adapt`]. New consumption sites call these
//!    instead of re-deriving the decision.
//! 4. **The runtime stays tolerant as defense in depth.** Shape-laddered
//!    helpers (`rusty::iter`, `rusty::len`, …) end with a deref-peel arm, so
//!    hand-written C++ and any emission path this module has not reached yet
//!    still work.
//!
//! The regression surface for all of this is
//! `transpiler/tests/guard_context_matrix.rs`, which transpiles, compiles and
//! RUNS every known consumption context. A new guard bug is a new row there.

use super::*;

/// The guard-producing methods of the runtime's cell/lock types
/// (`RefCell`, `Mutex`, `RwLock`). `lock`/`read`/`write` and the `try_`
/// variants yield `Result<Guard, _>`; the guard itself only emerges behind
/// `unwrap()`/`expect()`.
const DIRECT_GUARD_PRODUCERS: &[&str] = &["borrow", "borrow_mut", "lock", "read", "write"];

/// The guard types themselves, by (Rust and runtime) name — used to classify
/// a user function whose declared return type re-surfaces one.
const GUARD_TYPE_TAILS: &[&str] = &[
    "Ref",
    "RefMut",
    "MutexGuard",
    "RwLockReadGuard",
    "RwLockWriteGuard",
    "ReadGuard",
    "WriteGuard",
];


const RESULT_GUARD_PRODUCERS: &[&str] = &[
    "lock",
    "read",
    "write",
    "try_borrow",
    "try_borrow_mut",
    "try_lock",
    "try_read",
    "try_write",
];

impl CodeGen {
    /// Does this declared type name a deref wrapper? Structural where it can
    /// be: any crate type with an `impl Deref`/`DerefMut` qualifies via the
    /// collected `user_deref_targets` — no name list involved, exactly as in
    /// Rust, where the trait impl IS the signal. The small name seed covers
    /// only the hand-written runtime guards (`RefCell`, `Mutex`, …), whose
    /// Rust-side `impl Deref` the transpiler never sees — the same role
    /// rustc's std metadata plays.
    /// Split by KNOWLEDGE SOURCE, because the two halves scope differently:
    ///
    /// * a SEED guard (`RefMut`, `MutexGuard`, …) classifies unconditionally —
    ///   these are hand-written runtime types the typed tier has no dispatch
    ///   for, so the tolerant path is the only correct one;
    /// * a CRATE type with `impl Deref` classifies only when the expression
    ///   is UNTYPABLE (doctrine rule 3). When inference can see the type, the
    ///   existing typed auto-deref engine already resolves Target methods as
    ///   `(*g).method()` and inherent methods directly — and in Rust every
    ///   smart pointer and collection implements Deref (`Vec`, `String`,
    ///   `SmallVec`…), so classifying them here re-routed perfectly good
    ///   typed emission through the tolerant machinery (caught by a golden
    ///   test and two matrix crates).
    fn declared_type_tail(ty: &syn::Type) -> Option<String> {
        let mut ty = ty;
        loop {
            match ty {
                syn::Type::Reference(r) => ty = &r.elem,
                syn::Type::Paren(p) => ty = &p.elem,
                syn::Type::Group(g) => ty = &g.elem,
                _ => break,
            }
        }
        let syn::Type::Path(tp) = ty else {
            return None;
        };
        tp.path.segments.last().map(|seg| seg.ident.to_string())
    }

    fn declared_type_is_seed_guard(ty: &syn::Type) -> bool {
        Self::declared_type_tail(ty).is_some_and(|t| GUARD_TYPE_TAILS.contains(&t.as_str()))
    }

    fn declared_type_is_crate_deref_wrapper(&self, ty: &syn::Type) -> bool {
        Self::declared_type_tail(ty).is_some_and(|t| {
            self.user_deref_targets.contains_key(&t)
                || self.user_deref_targets.contains_key(&self.scoped_type_key(&t))
        })
    }

    /// The two-tier answer for a call expression whose declared return type
    /// is known: see the scoping note above.
    fn signature_classifies_as_guard(&self, expr: &syn::Expr, ty: &syn::Type) -> bool {
        if Self::declared_type_is_seed_guard(ty) {
            return true;
        }
        self.declared_type_is_crate_deref_wrapper(ty)
            && self.infer_simple_expr_type(expr).is_none()
    }

    /// May `expr` evaluate to a guard object the source never spells a deref
    /// for? True only in the UNCERTAIN case: the receiver the guard came from
    /// is one inference cannot type (doctrine rule 2) — a typable receiver is
    /// the typed tier's business.
    pub(super) fn receiver_is_uncertain_guard_call(&self, expr: &syn::Expr) -> bool {
        let expr = self.peel_paren_group_expr(expr);
        // A call to a USER function whose DECLARED return type names a guard:
        // `fn locked(&self) -> MutexGuard<'_, T>` re-surfaces the guard under
        // a name the producer list cannot know. The signature is the flow
        // edge here (doctrine rule 1). A false positive (a user type that
        // merely shares a guard's name) still behaves: the tolerant paths
        // try the direct member first and only deref when that is ill-formed.
        if let syn::Expr::Call(call) = expr {
            return self
                .lookup_function_return_type(&call.func)
                .cloned()
                .is_some_and(|ty| self.signature_classifies_as_guard(expr, &ty));
        }
        let syn::Expr::MethodCall(mc) = expr else {
            return false;
        };
        let method = mc.method.to_string();
        // `unwrap()` / `expect(..)` PASS GUARD-NESS THROUGH: `lock().unwrap()`
        // peels the `Result` and hands back the `MutexGuard` itself, so the
        // unwrapped value needs exactly the same treatment as the guard call
        // it wraps (flow, not shape — doctrine rule 1).
        let peels_result = match method.as_str() {
            "unwrap" => mc.args.is_empty(),
            "expect" => mc.args.len() == 1,
            _ => false,
        };
        if peels_result {
            let inner = self.peel_paren_group_expr(&mc.receiver);
            let syn::Expr::MethodCall(pmc) = inner else {
                return false;
            };
            return pmc.args.is_empty()
                && RESULT_GUARD_PRODUCERS.contains(&pmc.method.to_string().as_str())
                && self.infer_simple_expr_type(&pmc.receiver).is_none();
        }
        if mc.args.is_empty() && DIRECT_GUARD_PRODUCERS.contains(&method.as_str()) {
            return self.infer_simple_expr_type(&mc.receiver).is_none();
        }
        // User METHOD with a guard-naming declared return type — the method
        // sibling of the `Call` arm above.
        self.lookup_known_method_return_type_by_name(&method)
            .is_some_and(|ty| self.signature_classifies_as_guard(expr, &ty))
    }

    /// Flow-following variant for a `let` initializer: the guard call may sit
    /// behind the TAIL of a block, both arms of an `if`, or the arms of a
    /// `match` — `let g = if cond { a.borrow_mut() } else { b.borrow_mut() };`
    /// binds a guard exactly as the direct form does.
    ///
    /// `if`/`match` require EVERY value arm to be a maybe-guard: in Rust the
    /// arms share one type, so mixed arms mean the shared type is something
    /// inference should usually see (and mis-adapting a non-guard arm is the
    /// kind of silent change rule 2 exists to prevent). All-arms is the
    /// conservative reading of an uncertain situation.
    pub(super) fn init_expr_yields_maybe_guard(&self, expr: &syn::Expr) -> bool {
        let expr = self.peel_paren_group_expr(expr);
        if self.receiver_is_uncertain_guard_call(expr) {
            return true;
        }
        match expr {
            syn::Expr::Block(block) => block
                .block
                .stmts
                .last()
                .is_some_and(|stmt| match stmt {
                    syn::Stmt::Expr(tail, None) => self.init_expr_yields_maybe_guard(tail),
                    _ => false,
                }),
            syn::Expr::If(if_expr) => {
                let then_tail = if_expr.then_branch.stmts.last().is_some_and(|stmt| {
                    matches!(stmt, syn::Stmt::Expr(tail, None)
                        if self.init_expr_yields_maybe_guard(tail))
                });
                let else_tail = if_expr.else_branch.as_ref().is_some_and(|(_, else_expr)| {
                    self.init_expr_yields_maybe_guard(else_expr)
                });
                then_tail && else_tail
            }
            syn::Expr::Match(match_expr) => {
                !match_expr.arms.is_empty()
                    && match_expr
                        .arms
                        .iter()
                        .all(|arm| self.init_expr_yields_maybe_guard(&arm.body))
            }
            _ => false,
        }
    }

    /// Record a local bound from a maybe-guard initializer (declared `auto&&`
    /// by the let lowering). Consumptions of the bare name then classify as
    /// maybe-guard via [`Self::expr_is_uncertain_guard_local`].
    pub(super) fn record_local_uncertain_guard_binding(&mut self, name: &str) {
        // The surrounding scope stack is only pushed on some emission paths
        // (inline-rust never pushes one), so make sure there is somewhere to
        // record this. `emit_file` clears the stack per file.
        if self.local_uncertain_guard_bindings.is_empty() {
            self.local_uncertain_guard_bindings.push(HashSet::new());
        }
        if let Some(scope) = self.local_uncertain_guard_bindings.last_mut() {
            scope.insert(name.to_string());
        }
    }

    /// Is this expression a plain path naming a local bound from an
    /// untypable guard initializer? See `local_uncertain_guard_bindings`.
    pub(super) fn expr_is_uncertain_guard_local(&self, expr: &syn::Expr) -> bool {
        let syn::Expr::Path(path) = self.peel_paren_group_expr(expr) else {
            return false;
        };
        let Some(ident) = path.path.get_ident() else {
            return false;
        };
        let name = ident.to_string();
        self.local_uncertain_guard_bindings
            .iter()
            .any(|scope| scope.contains(&name))
    }

    /// TYPED tier: the receiver's type is VISIBLE and it is a cell/lock
    /// container, so this call yields a guard BY VALUE — definitely, not
    /// maybe. The typed lowerings consult this one predicate instead of each
    /// re-deriving "borrow on a RefCell / lock on a Mutex" locally, so both
    /// tiers share a single source of truth about what produces a guard.
    ///
    /// (`try_lock` is included: its `Result<Guard, _>` is equally not a
    /// reference, which is what the callers are deciding.)
    pub(super) fn known_guard_producing_call(&self, expr: &syn::Expr) -> bool {
        let expr = self.peel_paren_group_expr(expr);
        let syn::Expr::MethodCall(mc) = expr else {
            return false;
        };
        self.known_guard_producing_method_call(mc)
    }

    /// Method-call form of [`Self::known_guard_producing_call`], for callers
    /// that already hold the `ExprMethodCall`.
    pub(super) fn known_guard_producing_method_call(&self, mc: &syn::ExprMethodCall) -> bool {
        let method = mc.method.to_string();
        (matches!(method.as_str(), "borrow" | "borrow_mut")
            && self.receiver_is_refcell_container_type(&mc.receiver))
            || (matches!(method.as_str(), "lock" | "try_lock" | "read" | "write")
                && self.receiver_is_mutex_container_type(&mc.receiver))
    }

    /// Does `body` consume any of `names` — a method call on the bare name
    /// whose every known definition takes `self` by value? Used by the match
    /// lowering: an arm payload that gets consumed cannot be carved out of a
    /// `std::as_const(..)` materialization.
    pub(super) fn expr_consumes_any_named_binding(
        &self,
        body: &syn::Expr,
        names: &std::collections::HashSet<String>,
    ) -> bool {
        struct Finder<'a> {
            cg: &'a CodeGen,
            names: &'a std::collections::HashSet<String>,
            hit: bool,
        }
        impl<'a, 'ast> syn::visit::Visit<'ast> for Finder<'a> {
            fn visit_expr_method_call(&mut self, mc: &'ast syn::ExprMethodCall) {
                if !self.hit
                    && let syn::Expr::Path(p) = self.cg.peel_paren_group_expr(&mc.receiver)
                    && let Some(ident) = p.path.get_ident()
                    && self.names.contains(&ident.to_string())
                    && self
                        .cg
                        .known_method_consumes_self_by_value(&mc.method.to_string())
                {
                    self.hit = true;
                }
                syn::visit::visit_expr_method_call(self, mc);
            }
        }
        let mut finder = Finder {
            cg: self,
            names,
            hit: false,
        };
        syn::visit::Visit::visit_expr(&mut finder, body);
        finder.hit
    }

    /// The single value/argument-position adapter (doctrine rule 3): when
    /// `expr` may be a guard, wrap its emitted form in the tolerant deref so
    /// the VALUE — not the guard — reaches the consumer. `emitted` is
    /// returned untouched for everything else, which keeps typed emission
    /// byte-identical.
    ///
    /// Use for consumption sites that need the pointee (free-helper
    /// arguments, value reads). Do NOT use where the guard itself is the
    /// operand — binding it, returning it, dropping it.
    pub(super) fn maybe_guard_adapt(&self, expr: &syn::Expr, emitted: String) -> String {
        if self.receiver_is_uncertain_guard_call(expr) || self.expr_is_uncertain_guard_local(expr)
        {
            format!("rusty::detail::deref_if_pointer_like({})", emitted)
        } else {
            emitted
        }
    }
}
