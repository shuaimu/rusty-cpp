use super::*;

impl CodeGen {
    /// Walk all top-level items (and nested modules) and, for every
    /// `impl Trait for U` where both `Trait` and `U` are locally declared,
    /// emit `TraitAdapter<U>` / `TraitAdapterRef<U>` / `TraitAdapterRefMut<U>`
    /// specializations that delegate to `U`'s inherent methods (which were
    /// merged in by emit_struct).
    ///
    /// This complements `emit_trait_adapter_specializations`, which handles
    /// the foreign-type case (impls collected via the rusty_ext pipeline).
    pub(super) fn emit_local_trait_adapter_specializations(&mut self, items: &[syn::Item]) {
        // Tuple shape: (trait_name, trait_args, self_cpp, methods, module_path)
        // where trait_args is the C++ form of the impl's trait generic args
        // (e.g., "int32_t" for `impl Container<i32> for Foo`), empty when the
        // trait is non-generic.
        let mut grouped: Vec<(
            String,
            Vec<String>,
            String,
            Vec<syn::ImplItemFn>,
            Vec<String>,
        )> = Vec::new();
        self.collect_local_trait_impls_for_adapter(items, &[], &mut grouped);

        for (trait_name, trait_args, self_cpp, methods, module_path) in &grouped {
            // Skip Adapter emission when the trait was itself skipped.
            // Generic traits ARE emitted: their primary template is
            // declared `template <T..., U> class TraitAdapter;`, and
            // each `impl Trait<...> for U` produces a full specialization
            // `template<> class TraitAdapter<concrete_T..., U>`. The
            // emission code in `emit_one_local_adapter` already builds
            // `adapter_spec_args = "T_concrete..., U_concrete"`, so the
            // generic path works once we allow it.
            if self.skipped_interface_traits.contains(trait_name) {
                continue;
            }
            // Dedup against the foreign-impl pipeline's emitted set so
            // we never emit two `class TraitAdapter<U>` for the same
            // (trait, U). This shares one set with the foreign-impl
            // path because they emit into the same C++ namespace and
            // would collide on `template <> class TraitAdapter<U>`.
            let dedup_key = (trait_name.clone(), self_cpp.clone());
            if !self.emitted_foreign_adapter_specs.insert(dedup_key) {
                continue;
            }
            // Class template specializations must live in the trait's
            // declaring namespace. The impl block may be in a different
            // module than the trait declaration (`impl Flags for X` in
            // a sibling/child module). Recover the trait's declaring
            // namespace from trait_declared_paths; fall back to the
            // impl's module path if no scoped declaration is recorded
            // (matches the prior behavior for traits declared in the
            // same module as the impl).
            let mut trait_ns_path: Vec<String> = Vec::new();
            let found_in_paths = if let Some(full) = self
                .trait_declared_path_by_short_name
                .get(trait_name)
            {
                let segments: Vec<&str> = full.split("::").collect();
                if segments.len() > 1 {
                    trait_ns_path = segments[..segments.len() - 1]
                        .iter()
                        .map(|s| s.to_string())
                        .collect();
                }
                true
            } else {
                false
            };
            if !found_in_paths {
                trait_ns_path = module_path.clone();
            }
            // When the spec lives in a different namespace than the
            // impl type, qualify the impl type with a leading `::` and
            // its module path so the name resolves correctly inside
            // the trait's namespace (otherwise C++ would search the
            // trait's namespace first and miss it).
            let qualified_self_cpp: String =
                if !self_cpp.contains("::") && trait_ns_path != *module_path && !module_path.is_empty() {
                    let renamed = self.renamed_module_scope_segments(module_path);
                    format!("::{}::{}", renamed.join("::"), self_cpp)
                } else {
                    self_cpp.clone()
                };
            let needs_ns = !trait_ns_path.is_empty();
            if needs_ns {
                let renamed = self.renamed_module_scope_segments(&trait_ns_path);
                self.writeln(&format!("namespace {} {{", renamed.join("::")));
            }
            // Owning: holds U by value.
            self.emit_one_local_adapter(
                trait_name,
                trait_args,
                "Adapter",
                &qualified_self_cpp,
                AdapterStorageKind::Owning,
                methods,
            );
            // Borrowing const ref: holds const U&.
            self.emit_one_local_adapter(
                trait_name,
                trait_args,
                "AdapterRef",
                &qualified_self_cpp,
                AdapterStorageKind::ConstRef,
                methods,
            );
            // Borrowing mut ref: holds U&.
            self.emit_one_local_adapter(
                trait_name,
                trait_args,
                "AdapterRefMut",
                &qualified_self_cpp,
                AdapterStorageKind::MutRef,
                methods,
            );
            // Phase 3b.1: helper traits class spec for this impl. The
            // local-impl pipeline appends each assoc-type binding to
            // trait_args after the explicit generic args (see
            // collect_local_trait_impls_for_adapter lines 23447-23462),
            // so the assoc-binding tail equals the last
            // |trait_associated_type_names[trait_name]| entries.
            let assoc_names = self
                .trait_associated_type_names
                .get(trait_name)
                .cloned()
                .unwrap_or_default();
            if !assoc_names.is_empty() && trait_args.len() >= assoc_names.len() {
                let tail_start = trait_args.len() - assoc_names.len();
                let assoc_pairs: Vec<(String, String)> = assoc_names
                    .iter()
                    .zip(trait_args[tail_start..].iter())
                    .map(|(n, t)| (n.clone(), t.clone()))
                    .collect();
                self.emit_assoc_type_helper_spec(
                    trait_name,
                    &qualified_self_cpp,
                    &assoc_pairs,
                );
            }
            if needs_ns {
                self.writeln("}");
                self.newline();
            }
        }
    }

    pub(super) fn emit_stmt(&mut self, stmt: &syn::Stmt, is_tail: bool) {
        // Item 11 completion: statement-level lowering for
        //   `let pat = match tuple_scrutinee { (P0..) => return …, (Q0..) => (…) };`
        // The default lowering wraps both arms in an IIFE whose `return`
        // becomes lambda-local rather than function-local — and with
        // diverging arm types the lambda's `auto` deduction fails. Lift
        // the diverging arm to a top-level `if (cond) { …; return X; }`
        // and wrap the single non-diverging arm in its own IIFE whose
        // return type unambiguously deduces.
        if let syn::Stmt::Local(local) = stmt {
            if self.try_emit_let_match_return_statement_level(local) {
                return;
            }
            // `let x = match { ... fn-level-return arms ... }` whose value
            // type differs from the fn's return: no expression-position
            // lowering can host the returns. Declare the local, then run
            // the statement if-chain with `x = <arm value>` assignments.
            // Option/Result scrutinees stay on the established try-style
            // lowering (shadow renaming, emplace machinery); this path is
            // for custom data enums it declines (e.g. loader's Progress).
            if let syn::Pat::Ident(pat_ident) = &local.pat
                && pat_ident.subpat.is_none()
                && let Some(init) = &local.init
                && init.diverge.is_none()
                && let syn::Expr::Match(match_expr) = self.peel_paren_group_expr(&init.expr)
                && self.match_expr_has_explicit_return_arm(match_expr)
                && match_expr.arms.iter().all(|arm| arm.guard.is_none())
                && !self.match_arms_reference_option_result_variants(&match_expr.arms)
            {
                let arm_ty = self
                    .infer_match_arms_common_type(&match_expr.arms)
                    .or_else(|| self.infer_match_arms_common_type_with_scrutinee(match_expr));
                let fn_ret_cpp = self
                    .current_return_type_hint()
                    .map(|t| self.map_type(t))
                    .unwrap_or_default();
                if let Some(arm_ty) = arm_ty {
                    let arm_cpp = self.map_type(&arm_ty);
                    if !arm_cpp.is_empty()
                        && arm_cpp != "auto"
                        && !type_string_has_auto_placeholder(&arm_cpp)
                        && !fn_ret_cpp.is_empty()
                        && fn_ret_cpp != "auto"
                        && arm_cpp != fn_ret_cpp
                    {
                        let rust_name = pat_ident.ident.to_string();
                        // Emit the scrutinee BEFORE allocating the binding's
                        // C++ name: `let event = match event { … }` must read
                        // the OUTER `event`, and the new binding needs a
                        // shadow name (`event_shadow1`) — a bare re-decl of
                        // the same name is a C++ redefinition error.
                        let scrutinee = self.emit_expr_to_string(&match_expr.expr);
                        let cpp_name = self.allocate_local_cpp_name(&rust_name);
                        let variant_ctx =
                            self.infer_variant_type_context_from_expr(&match_expr.expr);
                        let decl = format!("{} {};", arm_cpp, cpp_name);
                        self.register_local_binding(rust_name.clone(), Some(arm_ty.clone()));
                        if self.variant_match_if_chain_impl(
                            &scrutinee,
                            match_expr,
                            variant_ctx.as_ref(),
                            Some(&rust_name),
                            Some(&decl),
                        ) {
                            return;
                        }
                        // Chain declined: fall through to the normal let
                        // emission (the bare decl line above is harmless —
                        // it will be shadowed).
                    }
                }
            }
        }
        // Deferred-init `let x;` whose declaration was suppressed by
        // `emit_local` (because no type could be inferred — `auto X;`
        // is invalid C++). Materialize the declaration at the first
        // assignment site as `auto cpp_name = rhs_text;`. Recurse into
        // `unsafe { x = expr; }` and similar trivial wrappers.
        //
        // Note: matches `Stmt::Expr(_, _)` regardless of trailing
        // semicolon. `unsafe { x = expr; }` at statement level parses
        // as `Stmt::Expr(Expr::Unsafe, None)` (braces close the stmt
        // without a semi); direct `x = expr;` parses as
        // `Stmt::Expr(Expr::Assign, Some(_))`.
        if let syn::Stmt::Expr(expr, _) = stmt
            && self.try_emit_pending_uninit_let_assign(expr)
        {
            return;
        }
        // Generalization: `lhs = match scrutinee { ... };` where the match has
        // explicit-return arms. Default lowering wraps the match in an IIFE
        // whose `return` becomes lambda-local rather than function-local.
        // Push the assignment down into each non-diverging arm tail (recursing
        // through blocks and nested matches), then emit the rewritten match
        // as a statement-level runtime match.
        if let syn::Stmt::Expr(syn::Expr::Assign(assign), Some(_)) = stmt {
            if self.try_emit_assign_match_return_statement_level(&assign.left, &assign.right) {
                return;
            }
        }
        match stmt {
            syn::Stmt::Local(local) => self.emit_local(local),
            syn::Stmt::Expr(expr, semi) => {
                // Cluster D: Rust 2024's `const { EXPR }` is a compile-time
                // fence — the inner expression is checked at constant-evaluation
                // time and never executed at runtime. When the inner EXPR
                // contains references the transpiler can't lower (e.g.
                // `assert!(size_of::<T>() == N)` against opaque template
                // params), the fallback used to emit
                // `rusty::intrinsics::unreachable()`, turning the surrounding
                // function into an unconditional panic at the first
                // instruction (BTreeMap's `ascend()` was hit by this).
                // Elide const-blocks at the statement level — leave a
                // comment so the position is visible in the emit.
                if matches!(expr, syn::Expr::Const(_)) {
                    self.writeln("// const-block elided (Rust 2024 compile-time fence)");
                    return;
                }
                // Tail `match` expressions in non-void functions must stay in
                // expression-lowering path so we emit `return <iife>;` instead
                // of a statement-level `std::visit(...)` with fallthrough.
                let force_expr_path = self.in_value_return_scope()
                    && is_tail
                    && semi.is_none()
                    && matches!(expr, syn::Expr::Match(match_expr) if self.match_expr_is_value_like(match_expr));
                let preserve_control_flow_tail_returns =
                    is_tail && semi.is_none() && self.in_value_return_scope();
                // Control flow expressions are emitted as statements directly
                if !force_expr_path
                    && self.try_emit_control_flow(expr, preserve_control_flow_tail_returns)
                {
                    return;
                }
                let should_emit_tail_return =
                    is_tail && semi.is_none() && self.in_value_return_scope();
                if !should_emit_tail_return
                    && self.try_emit_statement_compound_assign_without_unit_wrapper(expr)
                {
                    return;
                }
                let mut expr_str = if is_tail && semi.is_none() {
                    self.emit_expr_to_string_with_expected_and_move_if_needed(
                        expr,
                        self.current_return_type_hint(),
                    )
                } else {
                    self.emit_expr_to_string(expr)
                };
                if should_emit_tail_return
                    && matches!(
                        self.peel_paren_group_expr(expr),
                        syn::Expr::Path(path)
                            if path.path.segments.len() == 1 && path.path.segments[0].ident == "self"
                    )
                    && !self.current_self_receiver_is_reference()
                {
                    expr_str = if let Some(self_name) = self.current_self_path_override() {
                        format!("std::move({})", self_name)
                    } else {
                        "std::move((*this))".to_string()
                    };
                }
                if should_emit_tail_return {
                    // Tail expression without semicolon → return (or co_return in async)
                    let keyword = if self.in_async { "co_return" } else { "return" };
                    self.writeln(&format!("{} {};", keyword, expr_str));
                } else {
                    self.writeln(&format!("{};", expr_str));
                }
            }
            syn::Stmt::Item(item) => {
                // Nested function definitions → emit as lambda (C++ doesn't allow nested fns)
                if let syn::Item::Fn(f) = item {
                    let local_fn_name = f.sig.ident.to_string();
                    let local_styles =
                        self.collect_arg_pass_styles_from_inputs(&f.sig.inputs, false);
                    self.record_function_arg_pass_styles(&local_fn_name, local_styles);
                    let local_expected =
                        self.collect_arg_expected_types_from_inputs(&f.sig.inputs, false);
                    self.record_function_arg_expected_types(&local_fn_name, local_expected);
                    let local_type_params =
                        self.collect_type_param_names_from_generics(&f.sig.generics);
                    self.record_function_type_param_names(&local_fn_name, local_type_params);
                    let return_ty = self.collect_return_type_from_output(&f.sig.output);
                    self.record_function_return_type(&local_fn_name, return_ty.clone());
                    self.register_local_binding(local_fn_name, return_ty);
                    self.emit_nested_function(f);
                } else if self.block_depth > 0
                    && let syn::Item::Impl(impl_block) = item
                {
                    self.record_local_impl_method_metadata(impl_block);
                    self.writeln("// Rust-only nested impl block skipped in local scope");
                } else {
                    // Nested items emitted while generating deferred out-of-line
                    // method bodies must not inherit the outer owner qualifier.
                    let prev_decl_only = self.method_emission_declaration_only;
                    let prev_out_of_line_owner = self.method_emission_out_of_line_owner.clone();
                    let prev_skip_conflict = self.method_emission_skip_conflict_registration;
                    if self.block_depth > 0 {
                        self.method_emission_declaration_only = false;
                        self.method_emission_out_of_line_owner = None;
                        self.method_emission_skip_conflict_registration = false;
                    }
                    self.emit_item(item);
                    if self.block_depth > 0 {
                        self.method_emission_declaration_only = prev_decl_only;
                        self.method_emission_out_of_line_owner = prev_out_of_line_owner;
                        self.method_emission_skip_conflict_registration = prev_skip_conflict;
                    }
                }
            }
            syn::Stmt::Macro(stmt_macro) => {
                self.emit_macro_stmt(&stmt_macro.mac);
            }
        }
    }

    /// `{ return <ident>; }` or `{ <ident> }` (a single identity
    /// return/tail) → the ident.
    fn block_single_return_ident(block: &syn::Block) -> Option<String> {
        if block.stmts.len() != 1 {
            return None;
        }
        let ident_of = |expr: &syn::Expr| match expr {
            syn::Expr::Path(p) if p.path.segments.len() == 1 && p.qself.is_none() => {
                Some(p.path.segments[0].ident.to_string())
            }
            _ => None,
        };
        match &block.stmts[0] {
            syn::Stmt::Expr(syn::Expr::Return(ret), _) => ident_of(ret.expr.as_deref()?),
            syn::Stmt::Expr(expr, None) => ident_of(expr),
            _ => None,
        }
    }

    /// A ` -> std::tuple<...>` return annotation for an un-annotated
    /// tuple-valued runtime match, inferred per slot across arms: a slot
    /// types from any arm where it is `Some(<pattern binding>)` (→
    /// Option of the binding's payload type), a bare pattern binding, or
    /// an expression the simple inferencer understands. Bare `None`
    /// slots contribute nothing — they are exactly what the annotation
    /// exists to type.
    fn infer_runtime_match_tuple_annotation(
        &self,
        match_expr: &syn::ExprMatch,
    ) -> Option<String> {
        let scrutinee_ty = self
            .infer_simple_expr_type(&match_expr.expr)
            .or_else(|| self.infer_local_binding_type_from_initializer(&match_expr.expr))?;
        let mut arity: Option<usize> = None;
        let mut arm_data: Vec<(HashMap<String, syn::Type>, syn::ExprTuple)> = Vec::new();
        for arm in &match_expr.arms {
            if self.is_expr_diverging(&arm.body) {
                continue;
            }
            let value = self
                .extract_match_arm_value_expr(&arm.body)
                .unwrap_or(&arm.body);
            let syn::Expr::Tuple(tuple) = self.peel_paren_group_expr(value) else {
                return None;
            };
            match arity {
                Some(n) if n != tuple.elems.len() => return None,
                None => arity = Some(tuple.elems.len()),
                _ => {}
            }
            let mut env = HashMap::new();
            self.bind_pattern_types_into_env(&arm.pat, &scrutinee_ty, &mut env);
            arm_data.push((env, tuple.clone()));
        }
        let arity = arity?;
        if arity == 0 || arm_data.is_empty() {
            return None;
        }
        let clean_cpp = |ty: &syn::Type| -> Option<String> {
            let cpp = self.map_type(ty);
            (!cpp.is_empty() && cpp != "auto" && !type_string_has_auto_placeholder(&cpp))
                .then_some(cpp)
        };
        let mut slot_cpp: Vec<Option<String>> = vec![None; arity];
        for (env, tuple) in &arm_data {
            for (idx, slot) in slot_cpp.iter_mut().enumerate() {
                if slot.is_some() {
                    continue;
                }
                let elem = self.peel_paren_group_expr(&tuple.elems[idx]);
                if let syn::Expr::Call(call) = elem
                    && let syn::Expr::Path(fp) = call.func.as_ref()
                    && fp.path.segments.last().is_some_and(|seg| seg.ident == "Some")
                    && call.args.len() == 1
                    && let syn::Expr::Path(ap) = self.peel_paren_group_expr(&call.args[0])
                    && ap.path.segments.len() == 1
                    && let Some(inner_ty) =
                        env.get(&ap.path.segments[0].ident.to_string())
                {
                    if let Some(inner_cpp) = clean_cpp(inner_ty) {
                        *slot = Some(format!(
                            "rusty::Option<std::remove_cvref_t<{}>>",
                            inner_cpp
                        ));
                    }
                } else if let syn::Expr::Path(ap) = elem
                    && ap.path.segments.len() == 1
                    && let Some(ty) = env.get(&ap.path.segments[0].ident.to_string())
                {
                    if let Some(cpp) = clean_cpp(ty) {
                        *slot = Some(format!("std::remove_cvref_t<{}>", cpp));
                    }
                } else if !expr_is_option_none_constructor(elem)
                    && let Some(ty) = self.infer_simple_expr_type(&tuple.elems[idx])
                {
                    if let Some(cpp) = clean_cpp(&ty) {
                        *slot = Some(format!("std::remove_cvref_t<{}>", cpp));
                    }
                }
            }
        }
        let slots: Option<Vec<String>> = slot_cpp.into_iter().collect();
        Some(format!(" -> std::tuple<{}>", slots?.join(", ")))
    }

    /// Any arm pattern (including or-pattern cases) naming an
    /// Option/Result variant (`Some`/`None`/`Ok`/`Err`).
    fn match_arms_reference_option_result_variants(&self, arms: &[syn::Arm]) -> bool {
        fn pat_names_std_variant(pat: &syn::Pat) -> bool {
            let path = match pat {
                syn::Pat::TupleStruct(ts) => &ts.path,
                syn::Pat::Path(p) => &p.path,
                syn::Pat::Struct(ps) => &ps.path,
                syn::Pat::Or(or) => return or.cases.iter().any(pat_names_std_variant),
                syn::Pat::Reference(r) => return pat_names_std_variant(&r.pat),
                syn::Pat::Paren(p) => return pat_names_std_variant(&p.pat),
                _ => return false,
            };
            path.segments
                .last()
                .is_some_and(|seg| {
                    matches!(
                        seg.ident.to_string().as_str(),
                        "Some" | "None" | "Ok" | "Err"
                    )
                })
        }
        arms.iter().any(|arm| pat_names_std_variant(&arm.pat))
    }

    pub(super) fn emit_control_flow_with_return_scope<F>(&mut self, preserve_tail_returns: bool, emit: F)
    where
        F: FnOnce(&mut Self),
    {
        if preserve_tail_returns {
            emit(self);
            return;
        }
        self.return_value_scopes.push(false);
        emit(self);
        self.return_value_scopes.pop();
    }

    pub(super) fn emit_runtime_match_expr(
        &self,
        match_expr: &syn::ExprMatch,
        variant_ctx: Option<&VariantTypeContext>,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let expected_ty_has_runtime_annotation = expected_ty.is_some_and(|ty| {
            !self
                .expected_lambda_return_annotation(Some(ty), false)
                .is_empty()
        });
        let inferred_runtime_match_ty = if !expected_ty_has_runtime_annotation {
            self.infer_match_arms_common_type_with_scrutinee(match_expr)
                .or_else(|| self.infer_match_arms_common_type(&match_expr.arms))
                .or_else(|| {
                    self.infer_match_arms_common_variant_constructor_owner(&match_expr.arms)
                })
        } else {
            None
        };
        let runtime_match_expected = if expected_ty_has_runtime_annotation {
            expected_ty
        } else {
            inferred_runtime_match_ty.as_ref().or(expected_ty)
        };
        let force_size_t_return = runtime_match_expected.is_none()
            && self.should_force_size_t_visit_return_for_bound_match(match_expr, expected_ty);
        // An arm that RETURNS FROM THE ENCLOSING FUNCTION (`_ => return None`)
        // cannot live inside this IIFE-lambda lowering — its `return` would
        // bind to the lambda ("no viable conversion from Option<Range<usize>>
        // to size_t", indexmap's try_simplify_range). When guards force the
        // fallthrough-arm shape (which the try-style lowerings can't take
        // either) AND the function's return type provably differs from the
        // match's value type (the shape where the lambda-bound `return` is a
        // hard type error — same-typed early returns keep the historical
        // lambda lowering), delegate to the statement-expression lowering,
        // where `return` stays a real function return. Thread this lowering's
        // own resolved value type — including the forced-size_t Bound
        // heuristic — as the expected type.
        if self.match_expr_has_explicit_return_arm(match_expr) {
            let usize_ty: syn::Type = parse_quote!(usize);
            let stmt_expr_expected = if force_size_t_return {
                Some(&usize_ty)
            } else {
                runtime_match_expected
            };
            let fn_return_differs_from_match_value = match (
                self.current_return_type_hint(),
                stmt_expr_expected,
            ) {
                (Some(fn_ret), Some(match_val)) => {
                    let fn_ret_cpp = self.map_type(fn_ret);
                    let match_val_cpp = self.map_type(match_val);
                    !fn_ret_cpp.is_empty()
                        && !match_val_cpp.is_empty()
                        && fn_ret_cpp != "auto"
                        && match_val_cpp != "auto"
                        && fn_ret_cpp != match_val_cpp
                }
                _ => false,
            };
            if fn_return_differs_from_match_value {
                if let Some(lowered) = self
                    .emit_match_expr_switch_statement_expr_with_arm_mode(
                        match_expr,
                        stmt_expr_expected,
                        variant_ctx,
                        true,
                    )
                {
                    return Some(lowered);
                }
            }
        }
        let callable_passthrough_arm = self.match_expr_has_callable_passthrough_arm(match_expr);
        let runtime_match_return_annotation = if force_size_t_return {
            " -> size_t".to_string()
        } else {
            let typed = self.expected_lambda_return_annotation(runtime_match_expected, false);
            if typed.is_empty() && runtime_match_expected.is_none() && callable_passthrough_arm {
                " -> decltype(auto)".to_string()
            } else {
                typed
            }
        };
        // Un-annotated tuple-valued match: infer per-slot types across arms
        // (a `None` slot types from a sibling's `Some(<pattern binding>)`,
        // the binding from the arm pattern's payload) so the lambda's
        // return deduction can't lock onto None_t.
        let runtime_match_return_annotation = if runtime_match_return_annotation.is_empty() {
            self.infer_runtime_match_tuple_annotation(match_expr)
                .unwrap_or(runtime_match_return_annotation)
        } else {
            runtime_match_return_annotation
        };
        let scrutinee_borrows_payload =
            self.runtime_match_scrutinee_borrows_payload(&match_expr.expr);
        let payload_source = if scrutinee_borrows_payload {
            "std::as_const(_m)"
        } else {
            "_m"
        };
        // Owned scrutinee: Rust MOVES by-value pattern payloads into their
        // bindings, so arm-body uses may std::move them. Only a borrowing
        // scrutinee keeps the bindings references.
        let match_bindings_are_refs = scrutinee_borrows_payload;
        // A tuple arm carrying a bare `None` slot needs the lambda RETURN
        // ANNOTATED to type that slot (`(s, None)` alongside
        // `(variant, Some(value))` — un-annotated, the deduction locks
        // onto None_t from whichever arm lowers first). Fall back to the
        // expression lowerings that carry expected types.
        if runtime_match_return_annotation.is_empty()
            && match_expr.arms.iter().any(|arm| {
                let value = self
                    .extract_match_arm_value_expr(&arm.body)
                    .unwrap_or(&arm.body);
                if let syn::Expr::Tuple(tuple) = self.peel_paren_group_expr(value) {
                    tuple
                        .elems
                        .iter()
                        .any(|elem| expr_is_option_none_constructor(elem))
                } else {
                    false
                }
            })
        {
            return None;
        }
        let arm_expected_ty = runtime_match_expected;
        let scrutinee = self.emit_expr_to_string(&match_expr.expr);
        let mut out = format!(
            "[&](){} {{ auto&& _m = {}; ",
            runtime_match_return_annotation, scrutinee
        );

        let mut saw_runtime_pattern = false;
        for (idx, arm) in match_expr.arms.iter().enumerate() {
            let arm_bindings_are_refs = match_bindings_are_refs
                || !self.runtime_match_enum_is_type_param_free(&arm.pat, variant_ctx);
            match &arm.pat {
                syn::Pat::TupleStruct(ts) => {
                    if let Some((cond_method, unwrap_method)) =
                        self.runtime_tuple_struct_match_methods(&ts.path, variant_ctx)
                    {
                        if ts.elems.len() != 1 {
                            return None;
                        }
                        let mut binding_map = HashMap::new();
                        let direct_binding_passthrough = arm.guard.is_none()
                            && matches!(
                                (&ts.elems[0], self.extract_value_expr(&arm.body)),
                                (syn::Pat::Ident(pi), Some(syn::Expr::Path(body_path)))
                                    if body_path.path.segments.len() == 1
                                        && body_path.path.segments[0].ident == pi.ident
                            );
                        saw_runtime_pattern = true;
                        if direct_binding_passthrough {
                            out.push_str(&format!(
                                "if (_m.{}()) {{ return {}.{}(); }} ",
                                cond_method, payload_source, unwrap_method
                            ));
                            continue;
                        }
                        out.push_str(&format!("if (_m.{}()) {{ ", cond_method));
                        let matched_value = format!("_mv{}", idx);
                        let mut binding_stmts = Vec::new();
                        let payload_variant_ctx =
                            self.infer_variant_type_context_from_pattern(&ts.elems[0], variant_ctx);
                        let payload_match_condition = self
                            .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                                &ts.elems[0],
                                &matched_value,
                                &mut binding_stmts,
                                &mut binding_map,
                                payload_variant_ctx.as_ref(),
                            )?;
                        let needs_payload_materialization = !binding_stmts.is_empty()
                            || payload_match_condition.is_some()
                            || arm.guard.is_some();
                        let mut payload_bindings_are_refs = true;
                        if needs_payload_materialization {
                            let payload_value_source =
                                if payload_match_condition.is_some() || arm.guard.is_some() {
                                    "std::as_const(_m)"
                                } else {
                                    payload_source
                                };
                            payload_bindings_are_refs = payload_value_source != "_m";
                            out.push_str(&format!(
                                "auto&& {} = {}.{}(); ",
                                matched_value, payload_value_source, unwrap_method
                            ));
                        }
                        if let Some(cond) = &payload_match_condition {
                            out.push_str(&format!("if ({}) {{ ", cond));
                        }
                        for stmt in binding_stmts {
                            out.push_str(&stmt);
                            out.push(' ');
                        }
                        let body = {
                            let emitted = self
                                .emit_expr_with_try_style_binding_scope_with_ref_mode(
                                    &arm.body,
                                    arm_expected_ty,
                                    &binding_map,
                                    payload_bindings_are_refs,
                                );
                            self.maybe_wrap_variant_constructor_with_expected_enum(
                                &arm.body,
                                emitted,
                                arm_expected_ty,
                            )
                        };
                        // Keep runtime-expression match return typing coherent.
                        // Diverging arms should still return when the emitted body is already
                        // typed (e.g. `[&]() -> T { panic(...); }()`), otherwise C++
                        // may deduce inconsistent lambda return types (`T` vs `void`).
                        let diverging = self.is_expr_diverging(&arm.body);
                        let body_trimmed = body.trim_start();
                        let body_is_return_expr = body_trimmed.starts_with("return ");
                        let body_is_typed_iife = body_trimmed.starts_with("[&]() -> ");
                        let ret_prefix =
                            if body_is_return_expr || (diverging && !body_is_typed_iife) {
                                ""
                            } else {
                                "return "
                            };

                        if let Some((_, guard)) = &arm.guard {
                            let guard_str = self
                                .emit_expr_with_try_style_binding_scope_with_ref_mode(
                                    guard,
                                    None,
                                    &binding_map,
                                    payload_bindings_are_refs,
                                );
                            out.push_str(&format!(
                                "if ({}) {{ {}{}; }} ",
                                guard_str, ret_prefix, body
                            ));
                        } else {
                            out.push_str(&format!("{}{}; ", ret_prefix, body));
                        }
                        if payload_match_condition.is_some() {
                            out.push_str("} ");
                        }
                        out.push_str("} ");
                    } else {
                        let mut binding_stmts = Vec::new();
                        let mut binding_map = HashMap::new();
                        let wrapper_value_bindings =
                            self.arm_pointer_wrapper_value_bindings(&arm.pat, &arm.body);
                        if !wrapper_value_bindings.is_empty() {
                            self.pointer_unwrap_suppressed_bindings
                                .borrow_mut()
                                .extend(wrapper_value_bindings.iter().cloned());
                        }
                        let collected = self
                            .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                                &arm.pat,
                                "_m",
                                &mut binding_stmts,
                                &mut binding_map,
                                variant_ctx,
                            );
                        if !wrapper_value_bindings.is_empty() {
                            self.pointer_unwrap_suppressed_bindings.borrow_mut().clear();
                        }
                        let cond = collected?;
                        saw_runtime_pattern = true;
                        let cond_expr = cond.unwrap_or_else(|| "true".to_string());
                        let body = {
                            let emitted = self.emit_expr_with_try_style_binding_scope_with_ref_mode(
                                &arm.body,
                                arm_expected_ty,
                                &binding_map,
                                arm_bindings_are_refs,
                            );
                            self.maybe_wrap_variant_constructor_with_expected_enum(
                                &arm.body,
                                emitted,
                                arm_expected_ty,
                            )
                        };
                        let diverging = self.is_expr_diverging(&arm.body);
                        let body_trimmed = body.trim_start();
                        let body_is_return_expr = body_trimmed.starts_with("return ");
                        let body_is_typed_iife = body_trimmed.starts_with("[&]() -> ");
                        let ret_prefix =
                            if body_is_return_expr || (diverging && !body_is_typed_iife) {
                                ""
                            } else {
                                "return "
                            };
                        out.push_str(&format!("if ({}) {{ ", cond_expr));
                        for stmt in binding_stmts {
                            out.push_str(&stmt);
                            out.push(' ');
                        }
                        if let Some((_, guard)) = &arm.guard {
                            let guard_str = self.emit_expr_with_try_style_binding_scope(
                                guard,
                                None,
                                &binding_map,
                            );
                            out.push_str(&format!(
                                "if ({}) {{ {}{}; }} ",
                                guard_str, ret_prefix, body
                            ));
                        } else {
                            out.push_str(&format!("{}{}; ", ret_prefix, body));
                        }
                        out.push_str("} ");
                    }
                }
                syn::Pat::Path(_) => {
                    let mut binding_stmts = Vec::new();
                    let mut binding_map = HashMap::new();
                    let wrapper_value_bindings =
                        self.arm_pointer_wrapper_value_bindings(&arm.pat, &arm.body);
                    if !wrapper_value_bindings.is_empty() {
                        self.pointer_unwrap_suppressed_bindings
                            .borrow_mut()
                            .extend(wrapper_value_bindings.iter().cloned());
                    }
                    let collected = self
                        .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                            &arm.pat,
                            "_m",
                            &mut binding_stmts,
                            &mut binding_map,
                            variant_ctx,
                        );
                    if !wrapper_value_bindings.is_empty() {
                        self.pointer_unwrap_suppressed_bindings.borrow_mut().clear();
                    }
                    let cond = collected?;
                    saw_runtime_pattern = true;
                    let cond_expr = cond.unwrap_or_else(|| "true".to_string());
                    let body = {
                        let emitted = self.emit_expr_with_try_style_binding_scope_with_ref_mode(
                            &arm.body,
                            arm_expected_ty,
                            &binding_map,
                            arm_bindings_are_refs,
                        );
                        self.maybe_wrap_variant_constructor_with_expected_enum(
                            &arm.body,
                            emitted,
                            arm_expected_ty,
                        )
                    };
                    let diverging = self.is_expr_diverging(&arm.body);
                    let body_trimmed = body.trim_start();
                    let body_is_return_expr = body_trimmed.starts_with("return ");
                    let body_is_typed_iife = body_trimmed.starts_with("[&]() -> ");
                    let ret_prefix = if body_is_return_expr || (diverging && !body_is_typed_iife) {
                        ""
                    } else {
                        "return "
                    };
                    out.push_str(&format!("if ({}) {{ ", cond_expr));
                    for stmt in binding_stmts {
                        out.push_str(&stmt);
                        out.push(' ');
                    }
                    if let Some((_, guard)) = &arm.guard {
                        let guard_str =
                            self.emit_expr_with_try_style_binding_scope(guard, None, &binding_map);
                        out.push_str(&format!(
                            "if ({}) {{ {}{}; }} ",
                            guard_str, ret_prefix, body
                        ));
                    } else {
                        out.push_str(&format!("{}{}; ", ret_prefix, body));
                    }
                    out.push_str("} ");
                }
                syn::Pat::Wild(_) => {
                    let binding_map = HashMap::new();
                    let body = {
                        let emitted = self.emit_expr_with_try_style_binding_scope_with_ref_mode(
                            &arm.body,
                            arm_expected_ty,
                            &binding_map,
                            arm_bindings_are_refs,
                        );
                        self.maybe_wrap_variant_constructor_with_expected_enum(
                            &arm.body,
                            emitted,
                            arm_expected_ty,
                        )
                    };
                    let diverging = self.is_expr_diverging(&arm.body);
                    let body_trimmed = body.trim_start();
                    let body_is_return_expr = body_trimmed.starts_with("return ");
                    let body_is_typed_iife = body_trimmed.starts_with("[&]() -> ");
                    let ret_prefix = if body_is_return_expr || (diverging && !body_is_typed_iife) {
                        ""
                    } else {
                        "return "
                    };
                    out.push_str("if (true) { ");
                    if let Some((_, guard)) = &arm.guard {
                        let guard_str =
                            self.emit_expr_with_try_style_binding_scope(guard, None, &binding_map);
                        out.push_str(&format!(
                            "if ({}) {{ {}{}; }} ",
                            guard_str, ret_prefix, body
                        ));
                    } else {
                        out.push_str(&format!("{}{}; ", ret_prefix, body));
                    }
                    out.push_str("} ");
                }
                syn::Pat::Ident(pi) => {
                    let mut binding_stmts = Vec::new();
                    let mut binding_map = HashMap::new();
                    if pi.subpat.is_some() {
                        let cond = self
                            .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                                &arm.pat,
                                "_m",
                                &mut binding_stmts,
                                &mut binding_map,
                                variant_ctx,
                            )?;
                        saw_runtime_pattern = true;
                        let cond_expr = cond.unwrap_or_else(|| "true".to_string());
                        out.push_str(&format!("if ({}) {{ ", cond_expr));
                        for stmt in &binding_stmts {
                            out.push_str(stmt);
                            out.push(' ');
                        }
                    } else if let Some(cond_method) =
                        self.runtime_ident_match_condition_method(pi, variant_ctx)
                    {
                        saw_runtime_pattern = true;
                        out.push_str(&format!("if (_m.{}()) {{ ", cond_method));
                    } else if pi.by_ref.is_none()
                        && pi.mutability.is_none()
                        && self.pattern_ident_is_const_value(&pi.ident.to_string())
                    {
                        // A bare ident that resolves to a known external std
                        // unit variant (e.g. Less/Equal/Greater from cmp::Ordering
                        // brought in by slice R, or AllIncluded/AllExcluded of
                        // a same-file data-enum). Without this check the arm
                        // would emit `if (true) { const auto& Less = _m; ... }`
                        // — silently treating the unit variant as a fresh
                        // binding. With it the arm emits the equality check
                        // against the constant.
                        saw_runtime_pattern = true;
                        let raw_ident = pi.ident.to_string();
                        let ident = escape_cpp_keyword(&raw_ident);
                        // For **data-enum** unit variants (`enum E { Unit }`
                        // emitted as `struct E_Unit; using E = variant<…,
                        // E_Unit, …>;` + factory `static E Unit() { … }`),
                        // the bare `Unit` is a pointer-to-function, NOT a
                        // value. Comparing `_m == Unit` resolves to
                        // `E == E ()` and clang errors with "invalid
                        // operands to binary expression". The match arm
                        // typically sits inside an `impl ::fmt::Display for
                        // E`, so the variant tag `E_Unit` is in scope at
                        // the emission site — emit `variant_holds<E_Unit>`
                        // using the bare tag from `variant_ctx.enum_name`.
                        //
                        // For C-like enum constants (`Less`, `Acquire`,
                        // …) the bare name truly is a value, so keep the
                        // equality form.
                        let data_unit_emit = variant_ctx.and_then(|ctx| {
                            let key = format!("{}_{}", ctx.enum_name, raw_ident);
                            if self.data_enum_unit_variants.contains(&key) {
                                Some(format!("{}_{}", ctx.enum_name, raw_ident))
                            } else {
                                None
                            }
                        });
                        // A bare C-like enum variant (`use Enum::*`) compares
                        // against the scoped `Enum::VARIANT` — C++20 `enum class`
                        // does not flatten variants into the surrounding scope, so
                        // the bare name is undeclared. Qualify with the unique
                        // owning enum (same resolution the expression path uses).
                        let c_like_qualified = self
                            .unique_c_like_enum_owner_for_variant_name(&raw_ident)
                            .map(|owner| format!("{}::{}", owner, ident));
                        if let Some(tag) = data_unit_emit {
                            out.push_str(&format!(
                                "if (rusty::detail::variant_holds<{}>(_m)) {{ ",
                                tag
                            ));
                        } else if let Some(qualified) = c_like_qualified {
                            out.push_str(&format!("if (_m == {}) {{ ", qualified));
                        } else {
                            out.push_str(&format!("if (_m == {}) {{ ", ident));
                        }
                    } else {
                        let cpp_name = escape_cpp_keyword(&pi.ident.to_string());
                        binding_map.insert(pi.ident.to_string(), cpp_name.clone());
                        out.push_str("if (true) { ");
                        out.push_str(&format!("const auto& {} = _m; ", cpp_name));
                    }
                    let body = {
                        let emitted = self.emit_expr_with_try_style_binding_scope_with_ref_mode(
                            &arm.body,
                            arm_expected_ty,
                            &binding_map,
                            arm_bindings_are_refs,
                        );
                        self.maybe_wrap_variant_constructor_with_expected_enum(
                            &arm.body,
                            emitted,
                            arm_expected_ty,
                        )
                    };
                    let diverging = self.is_expr_diverging(&arm.body);
                    let body_trimmed = body.trim_start();
                    let body_is_return_expr = body_trimmed.starts_with("return ");
                    let body_is_typed_iife = body_trimmed.starts_with("[&]() -> ");
                    let ret_prefix = if body_is_return_expr || (diverging && !body_is_typed_iife) {
                        ""
                    } else {
                        "return "
                    };
                    if let Some((_, guard)) = &arm.guard {
                        let guard_str =
                            self.emit_expr_with_try_style_binding_scope(guard, None, &binding_map);
                        out.push_str(&format!(
                            "if ({}) {{ {}{}; }} ",
                            guard_str, ret_prefix, body
                        ));
                    } else {
                        out.push_str(&format!("{}{}; ", ret_prefix, body));
                    }
                    out.push_str("} ");
                }
                syn::Pat::Struct(_)
                | syn::Pat::Slice(_)
                | syn::Pat::Reference(_)
                | syn::Pat::Type(_)
                | syn::Pat::Paren(_) => {
                    let mut binding_stmts = Vec::new();
                    let mut binding_map = HashMap::new();
                    let wrapper_value_bindings =
                        self.arm_pointer_wrapper_value_bindings(&arm.pat, &arm.body);
                    if !wrapper_value_bindings.is_empty() {
                        self.pointer_unwrap_suppressed_bindings
                            .borrow_mut()
                            .extend(wrapper_value_bindings.iter().cloned());
                    }
                    let collected = self
                        .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                            &arm.pat,
                            "_m",
                            &mut binding_stmts,
                            &mut binding_map,
                            variant_ctx,
                        );
                    if !wrapper_value_bindings.is_empty() {
                        self.pointer_unwrap_suppressed_bindings.borrow_mut().clear();
                    }
                    let cond = collected?;
                    saw_runtime_pattern = true;
                    let cond_expr = cond.unwrap_or_else(|| "true".to_string());
                    let body = {
                        let emitted = self.emit_expr_with_try_style_binding_scope_with_ref_mode(
                            &arm.body,
                            arm_expected_ty,
                            &binding_map,
                            arm_bindings_are_refs,
                        );
                        self.maybe_wrap_variant_constructor_with_expected_enum(
                            &arm.body,
                            emitted,
                            arm_expected_ty,
                        )
                    };
                    let diverging = self.is_expr_diverging(&arm.body);
                    let body_trimmed = body.trim_start();
                    let body_is_return_expr = body_trimmed.starts_with("return ");
                    let body_is_typed_iife = body_trimmed.starts_with("[&]() -> ");
                    let ret_prefix = if body_is_return_expr || (diverging && !body_is_typed_iife) {
                        ""
                    } else {
                        "return "
                    };
                    out.push_str(&format!("if ({}) {{ ", cond_expr));
                    for stmt in binding_stmts {
                        out.push_str(&stmt);
                        out.push(' ');
                    }
                    if let Some((_, guard)) = &arm.guard {
                        let guard_str =
                            self.emit_expr_with_try_style_binding_scope(guard, None, &binding_map);
                        out.push_str(&format!(
                            "if ({}) {{ {}{}; }} ",
                            guard_str, ret_prefix, body
                        ));
                    } else {
                        out.push_str(&format!("{}{}; ", ret_prefix, body));
                    }
                    out.push_str("} ");
                }
                syn::Pat::Or(_) => {
                    // The collector handles or-cases WITH bindings too
                    // (identical binding sets select their payload through a
                    // condition-guarded ternary chain — error.rs mark()).
                    let mut binding_stmts = Vec::new();
                    let mut binding_map = HashMap::new();
                    let wrapper_value_bindings =
                        self.arm_pointer_wrapper_value_bindings(&arm.pat, &arm.body);
                    if !wrapper_value_bindings.is_empty() {
                        self.pointer_unwrap_suppressed_bindings
                            .borrow_mut()
                            .extend(wrapper_value_bindings.iter().cloned());
                    }
                    let collected = self
                        .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                            &arm.pat,
                            "_m",
                            &mut binding_stmts,
                            &mut binding_map,
                            variant_ctx,
                        );
                    if !wrapper_value_bindings.is_empty() {
                        self.pointer_unwrap_suppressed_bindings.borrow_mut().clear();
                    }
                    let cond = collected?;
                    saw_runtime_pattern = true;
                    let cond_expr = cond.unwrap_or_else(|| "true".to_string());
                    let body = {
                        let emitted = self.emit_expr_with_try_style_binding_scope_with_ref_mode(
                            &arm.body,
                            arm_expected_ty,
                            &binding_map,
                            arm_bindings_are_refs,
                        );
                        self.maybe_wrap_variant_constructor_with_expected_enum(
                            &arm.body,
                            emitted,
                            arm_expected_ty,
                        )
                    };
                    let diverging = self.is_expr_diverging(&arm.body);
                    let body_trimmed = body.trim_start();
                    let body_is_return_expr = body_trimmed.starts_with("return ");
                    let body_is_typed_iife = body_trimmed.starts_with("[&]() -> ");
                    let ret_prefix = if body_is_return_expr || (diverging && !body_is_typed_iife) {
                        ""
                    } else {
                        "return "
                    };
                    out.push_str(&format!("if ({}) {{ ", cond_expr));
                    for stmt in binding_stmts {
                        out.push_str(&stmt);
                        out.push(' ');
                    }
                    if let Some((_, guard)) = &arm.guard {
                        let guard_str =
                            self.emit_expr_with_try_style_binding_scope(guard, None, &binding_map);
                        out.push_str(&format!(
                            "if ({}) {{ {}{}; }} ",
                            guard_str, ret_prefix, body
                        ));
                    } else {
                        out.push_str(&format!("{}{}; ", ret_prefix, body));
                    }
                    out.push_str("} ");
                }
                _ => return None,
            }
        }

        if !saw_runtime_pattern {
            return None;
        }

        if let Some(expected) = runtime_match_expected {
            if runtime_match_return_annotation.is_empty() {
                out.push_str("rusty::intrinsics::unreachable(); }()");
            } else {
                out.push_str(&format!(
                    "return {}; }}()",
                    self.match_expr_unreachable_fallback_with_expected(Some(expected))
                ));
            }
        } else {
            out.push_str("rusty::intrinsics::unreachable(); }()");
        }
        Some(out)
    }

    pub(super) fn emit_arm_body(&mut self, body: &syn::Expr) {
        // If the body is a block, emit its statements.
        // Statement-level arm bodies must not tail-return: a block whose
        // tail is a value-producing expression (e.g. a nested match) is
        // executed for side effects here, not returned as the enclosing
        // function's value. Suppress value-return scope while emitting.
        if let syn::Expr::Block(block) = body {
            self.emit_control_flow_with_return_scope(false, |this| {
                this.emit_block(&block.block);
            });
        } else if self.try_emit_control_flow(body, false) {
            // Control flow handled
        } else {
            let body_str = self.emit_expr_to_string(body);
            self.writeln(&format!("{};", body_str));
        }
    }

    pub(super) fn emit_if(&mut self, if_expr: &syn::ExprIf) {
        self.emit_if_inner(if_expr, true);
    }

    pub(super) fn emit_if_let_unwrap_expr(&self, scrutinee_cpp: &str, unwrap_method: &str) -> String {
        if unwrap_method == IF_LET_OPTION_TAKE_VALUE_HELPER_MARKER {
            format!("rusty::detail::option_take_value({})", scrutinee_cpp)
        } else {
            format!("{}.{}()", scrutinee_cpp, unwrap_method)
        }
    }

    pub(super) fn emit_if_let_has_value_expr(&self, scrutinee_cpp: &str) -> String {
        format!("rusty::detail::option_has_value({})", scrutinee_cpp)
    }

    pub(super) fn emit_if_let_runtime_pattern(
        &mut self,
        pat: &syn::Pat,
        scrutinee_expr: &syn::Expr,
        scrutinee_cpp: &str,
        then_branch: &syn::Block,
        else_branch: &Option<(syn::token::Else, Box<syn::Expr>)>,
        first: bool,
    ) -> bool {
        let scrutinee_var = "_iflet_scrutinee";
        let runtime_entry_probe_scrutinee =
            self.try_emit_runtime_entry_probe_for_pattern(pat, scrutinee_expr);
        let effective_scrutinee_cpp = runtime_entry_probe_scrutinee
            .as_deref()
            .unwrap_or(scrutinee_cpp);
        let use_scrutinee_storage = self.if_let_requires_single_eval_scrutinee(scrutinee_expr)
            || runtime_entry_probe_scrutinee.is_some();
        let source_expr = if use_scrutinee_storage {
            scrutinee_var
        } else {
            effective_scrutinee_cpp
        };
        let variant_ctx = self.infer_variant_type_context_from_expr(scrutinee_expr);
        let mut binding_stmts = Vec::new();
        let mut binding_map = HashMap::new();
        // Identity-return then-branch (`{ return <binding>; }`): the bound
        // payload is returned AS its wrapper type — suppress the pointer
        // unwrap while collecting this pattern's bindings.
        let identity_return = Self::block_single_return_ident(then_branch);
        if let Some(name) = &identity_return {
            self.pointer_unwrap_suppressed_bindings
                .borrow_mut()
                .insert(name.clone());
        }
        let collected = self.collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
            pat,
            source_expr,
            &mut binding_stmts,
            &mut binding_map,
            variant_ctx.as_ref(),
        );
        if identity_return.is_some() {
            self.pointer_unwrap_suppressed_bindings.borrow_mut().clear();
        }
        let Some(cond_opt) = collected else {
            return false;
        };
        let cond = cond_opt.unwrap_or_else(|| "true".to_string());
        let if_header = if use_scrutinee_storage {
            format!(
                "if (auto&& {} = {}; {}) {{",
                scrutinee_var, effective_scrutinee_cpp, cond
            )
        } else {
            format!("if ({}) {{", cond)
        };
        if first {
            self.writeln(&if_header);
        } else {
            self.output.push_str(&format!("{}\n", if_header));
        }
        self.indent += 1;
        for stmt in binding_stmts {
            self.writeln(&stmt);
        }
        if binding_map.is_empty() {
            self.emit_block(then_branch);
        } else {
            self.local_cpp_bindings.push(binding_map.clone());
            let mut local_types = HashMap::new();
            let mut local_consts = HashMap::new();
            for rust_name in binding_map.keys() {
                local_types.insert(rust_name.clone(), None);
                local_consts.insert(rust_name.clone(), false);
            }
            self.local_bindings.push(local_types);
            self.local_shadowed_binding_types.push(HashMap::new());
            self.local_const_bindings.push(local_consts);
            self.local_reference_bindings.push(HashSet::new());
            self.rebind_reference_pointer_bindings.push(HashSet::new());
            self.emit_block(then_branch);
            self.local_reference_bindings.pop();
            self.rebind_reference_pointer_bindings.pop();
            self.local_const_bindings.pop();
            self.local_shadowed_binding_types.pop();
            self.local_bindings.pop();
            self.local_cpp_bindings.pop();
        }
        self.indent -= 1;
        self.emit_if_let_else(else_branch);
        true
    }

    /// Emit `if let Pattern = expr { ... } else { ... }` as C++ code.
    pub(super) fn emit_if_let(
        &mut self,
        let_expr: &syn::ExprLet,
        then_branch: &syn::Block,
        else_branch: &Option<(syn::token::Else, Box<syn::Expr>)>,
        first: bool,
    ) {
        // Strip reference from scrutinee: `if let Some(x) = &self.field` → use `self.field`
        // The `&` is a Rust borrow for pattern matching that has no C++ equivalent.
        // Cluster E: remember whether the original scrutinee was a `&` /
        // `&mut` borrow so we can use `std::as_const(...).unwrap()` /
        // `.as_mut().unwrap()` to keep the payload as a reference into the
        // original storage rather than consuming it.
        let (scrutinee_expr, scrutinee_borrow_mode) = match let_expr.expr.as_ref() {
            syn::Expr::Reference(r) => (r.expr.as_ref(), Some(r.mutability.is_some())),
            other => (other, None),
        };
        let scrutinee = self.emit_expr_to_string(scrutinee_expr);
        let (_, _, option_unwrap_method, _) =
            self.option_like_pattern_surface_for_expr(scrutinee_expr);

        // Binding-less data-enum variant tests (incl. or-patterns:
        // `if let State::CheckForTag | State::CheckForDuplicateTag = self.state`)
        // must test the variant — the Option surface (`.is_some()`) does not
        // exist on the std::variant scrutinee. Single-eval storage keeps
        // or-pattern conditions from re-evaluating an effectful scrutinee.
        if !matches!(&*let_expr.pat, syn::Pat::TupleStruct(ts) if ts.path.segments.last().is_some_and(|s| matches!(s.ident.to_string().as_str(), "Some" | "Ok" | "Err")))
            && let Some(variant_cond) = self
                .if_let_binding_less_variant_condition(&let_expr.pat, "_iflet_scrutinee")
        {
            if first {
                self.writeln("{");
            } else {
                self.output.push_str("{\n");
            }
            self.indent += 1;
            self.writeln(&format!("auto&& _iflet_scrutinee = {};", scrutinee));
            self.writeln(&format!("if ({}) {{", variant_cond));
            self.indent += 1;
            self.emit_block(then_branch);
            self.indent -= 1;
            self.emit_if_let_else(else_branch);
            self.indent -= 1;
            self.writeln("}");
            return;
        }

        match &*let_expr.pat {
            syn::Pat::TupleStruct(ts) => {
                let path_str = ts
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");

                match path_str.as_str() {
                    "Some" | "Option::Some" => {
                        // if let Some(v) = opt → if (opt.<is_some/has_value>()) { auto v = opt.<unwrap/value>(); ... }
                        let scrutinee_var = "_iflet_scrutinee";
                        let use_scrutinee_storage =
                            self.if_let_requires_single_eval_scrutinee(scrutinee_expr);
                        let cond_scrutinee = if use_scrutinee_storage {
                            scrutinee_var
                        } else {
                            &scrutinee
                        };
                        let body_scrutinee = if use_scrutinee_storage {
                            scrutinee_var
                        } else {
                            &scrutinee
                        };
                        let if_init_expr = if use_scrutinee_storage {
                            Some(scrutinee.as_str())
                        } else {
                            None
                        };
                        let cond = self.format_option_like_pattern_condition(
                            scrutinee_expr,
                            cond_scrutinee,
                            true,
                        );
                        self.emit_if_let_body(
                            &cond,
                            ts.elems.first(),
                            body_scrutinee,
                            scrutinee_expr,
                            option_unwrap_method,
                            then_branch,
                            else_branch,
                            first,
                            if_init_expr,
                            scrutinee.ends_with(".as_mut()"),
                            scrutinee_borrow_mode,
                        );
                    }
                    "Ok" | "Result::Ok" => {
                        // if let Ok(v) = result → if (result.is_ok()) { auto v = result.unwrap(); ... }
                        let scrutinee_var = "_iflet_scrutinee";
                        let use_scrutinee_storage =
                            self.if_let_requires_single_eval_scrutinee(scrutinee_expr);
                        let cond_scrutinee = if use_scrutinee_storage {
                            scrutinee_var
                        } else {
                            &scrutinee
                        };
                        let body_scrutinee = if use_scrutinee_storage {
                            scrutinee_var
                        } else {
                            &scrutinee
                        };
                        let if_init_expr = if use_scrutinee_storage {
                            Some(scrutinee.as_str())
                        } else {
                            None
                        };
                        let cond = format!("{}.is_ok()", cond_scrutinee);
                        self.emit_if_let_body(
                            &cond,
                            ts.elems.first(),
                            body_scrutinee,
                            scrutinee_expr,
                            "unwrap",
                            then_branch,
                            else_branch,
                            first,
                            if_init_expr,
                            scrutinee.ends_with(".as_mut()"),
                            scrutinee_borrow_mode,
                        );
                    }
                    "Err" | "Result::Err" => {
                        // if let Err(e) = result → if (result.is_err()) { auto e = result.unwrap_err(); ... }
                        let scrutinee_var = "_iflet_scrutinee";
                        let use_scrutinee_storage =
                            self.if_let_requires_single_eval_scrutinee(scrutinee_expr);
                        let cond_scrutinee = if use_scrutinee_storage {
                            scrutinee_var
                        } else {
                            &scrutinee
                        };
                        let body_scrutinee = if use_scrutinee_storage {
                            scrutinee_var
                        } else {
                            &scrutinee
                        };
                        let if_init_expr = if use_scrutinee_storage {
                            Some(scrutinee.as_str())
                        } else {
                            None
                        };
                        let cond = format!("{}.is_err()", cond_scrutinee);
                        self.emit_if_let_body(
                            &cond,
                            ts.elems.first(),
                            body_scrutinee,
                            scrutinee_expr,
                            "unwrap_err",
                            then_branch,
                            else_branch,
                            first,
                            if_init_expr,
                            scrutinee.ends_with(".as_mut()"),
                            scrutinee_borrow_mode,
                        );
                    }
                    _ => {
                        if !self.emit_if_let_runtime_pattern(
                            &let_expr.pat,
                            scrutinee_expr,
                            &scrutinee,
                            then_branch,
                            else_branch,
                            first,
                        ) {
                            if first {
                                self.writeln("if (/* TODO: if let tuple struct */) {");
                            } else {
                                self.output
                                    .push_str("if (/* TODO: if let tuple struct */) {\n");
                            }
                            self.indent += 1;
                            self.emit_block(then_branch);
                            self.indent -= 1;
                            self.emit_if_let_else(else_branch);
                        }
                    }
                }
            }
            syn::Pat::Ident(pi) => {
                let name = pi.ident.to_string();
                // Check for known enum variants that parse as idents
                let cond = match name.as_str() {
                    "None" => Some(self.format_option_like_pattern_condition(
                        scrutinee_expr,
                        &scrutinee,
                        false,
                    )),
                    _ => None,
                };

                if let Some(cond) = cond {
                    if first {
                        self.writeln(&format!("if ({}) {{", cond));
                    } else {
                        self.output.push_str(&format!("if ({}) {{\n", cond));
                    }
                    self.indent += 1;
                    self.emit_block(then_branch);
                    self.indent -= 1;
                    self.emit_if_let_else(else_branch);
                } else {
                    // if let x = expr → if (true) { auto x = expr; ... } (always matches)
                    let cpp_name = escape_cpp_keyword(&name);
                    if first {
                        self.writeln("if (true) {");
                    } else {
                        self.output.push_str("if (true) {\n");
                    }
                    self.indent += 1;
                    self.writeln(&format!("auto {} = {};", cpp_name, scrutinee));
                    self.emit_block(then_branch);
                    self.indent -= 1;
                    self.emit_if_let_else(else_branch);
                }
            }
            syn::Pat::Path(pp) => {
                // if let None = opt → if (opt.<is_none/!has_value>())
                let path_str = pp
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                let cond = match path_str.as_str() {
                    "None" | "Option::None" => {
                        self.format_option_like_pattern_condition(scrutinee_expr, &scrutinee, false)
                    }
                    _ => {
                        if self.emit_if_let_runtime_pattern(
                            &let_expr.pat,
                            scrutinee_expr,
                            &scrutinee,
                            then_branch,
                            else_branch,
                            first,
                        ) {
                            return;
                        }
                        let cpp_type = path_str.replace("::", "_");
                        format!("rusty::detail::variant_holds<{}>({})", cpp_type, scrutinee)
                    }
                };
                if first {
                    self.writeln(&format!("if ({}) {{", cond));
                } else {
                    self.output.push_str(&format!("if ({}) {{\n", cond));
                }
                self.indent += 1;
                self.emit_block(then_branch);
                self.indent -= 1;
                self.emit_if_let_else(else_branch);
            }
            _ => {
                if !self.emit_if_let_runtime_pattern(
                    &let_expr.pat,
                    scrutinee_expr,
                    &scrutinee,
                    then_branch,
                    else_branch,
                    first,
                ) {
                    if first {
                        self.writeln("if (/* TODO: if let pattern */) {");
                    } else {
                        self.output.push_str("if (/* TODO: if let pattern */) {\n");
                    }
                    self.indent += 1;
                    self.emit_block(then_branch);
                    self.indent -= 1;
                    self.emit_if_let_else(else_branch);
                }
            }
        }
    }

    /// Helper for common if-let patterns (Some/Ok/Err).
    /// Cluster E: `scrutinee_borrow_mode` is `None` for by-value scrutinees
    /// (consume via `unwrap()`), `Some(false)` for `&` (use
    /// `std::as_const(...).unwrap()` to keep a const-ref payload), and
    /// `Some(true)` for `&mut` (use `.as_mut().unwrap()` to keep a mut-ref
    /// payload).
    /// The payload type of an `Option<T>` / `Result<T, _>` (the first type arg).
    pub(super) fn option_or_result_ok_payload_type(&self, ty: &syn::Type) -> Option<syn::Type> {
        let peeled = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = peeled else {
            return None;
        };
        let seg = tp.path.segments.last()?;
        if !matches!(seg.ident.to_string().as_str(), "Option" | "Result") {
            return None;
        }
        let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
            return None;
        };
        args.args.iter().find_map(|a| match a {
            syn::GenericArgument::Type(t) => Some(t.clone()),
            _ => None,
        })
    }

    /// For `if let Some(name) = SCRUTINEE` where SCRUTINEE is `Option<fn ptr>` /
    /// `Result<fn ptr, _>`, the fn-pointer payload type — so the bound `name` is
    /// typed and a later `name(args)` call can detect an `unsafe fn` and lower it
    /// to `.call_unsafe(args)`. Returns None for non-fn-pointer payloads (which
    /// stay untyped, preserving prior behavior).
    fn if_let_some_binding_fn_pointer_type(
        &self,
        binding_pat: Option<&syn::Pat>,
        scrutinee_expr: &syn::Expr,
        name: &str,
    ) -> Option<syn::Type> {
        // `binding_pat` is the PAYLOAD pattern (the inner `x` of `Some(x)`/`Ok(x)`),
        // not the `Some(..)` wrapper — it must be a simple ident binding `name`.
        let syn::Pat::Ident(pi) = binding_pat? else {
            return None;
        };
        if pi.subpat.is_some() || pi.ident != name {
            return None;
        }
        let scrutinee_ty = self.infer_simple_expr_type(scrutinee_expr)?;
        let payload = self.option_or_result_ok_payload_type(&scrutinee_ty)?;
        matches!(
            self.peel_reference_paren_group_type(&payload),
            syn::Type::BareFn(_)
        )
        .then_some(payload)
    }

    pub(super) fn emit_if_let_body(
        &mut self,
        cond: &str,
        binding_pat: Option<&syn::Pat>,
        scrutinee: &str,
        scrutinee_expr: &syn::Expr,
        unwrap_method: &str,
        then_branch: &syn::Block,
        else_branch: &Option<(syn::token::Else, Box<syn::Expr>)>,
        first: bool,
        if_init_expr: Option<&str>,
        scrutinee_is_as_mut: bool,
        scrutinee_borrow_mode: Option<bool>,
    ) {
        let if_header = if let Some(init_expr) = if_init_expr {
            format!("if (auto&& {} = {}; {}) {{", scrutinee, init_expr, cond)
        } else {
            format!("if ({}) {{", cond)
        };
        if first {
            self.writeln(&if_header);
        } else {
            self.output.push_str(&format!("{}\n", if_header));
        }
        self.indent += 1;

        // Cluster E helper: construct the `<scrutinee>.unwrap()` or
        // borrow-preserving equivalent. For borrowed scrutinees (`&expr`,
        // `&mut expr`) route through `std::as_const(...).unwrap()` or
        // `.as_mut().unwrap()` so the payload stays a reference into the
        // original storage instead of consuming it.
        let borrow_aware_unwrap_expr = |scrutinee_cpp: &str, unwrap_m: &str| -> String {
            if unwrap_m == IF_LET_OPTION_TAKE_VALUE_HELPER_MARKER {
                return format!("rusty::detail::option_take_value({})", scrutinee_cpp);
            }
            if scrutinee_is_as_mut {
                return format!("{}.{}()", scrutinee_cpp, unwrap_m);
            }
            match scrutinee_borrow_mode {
                None => format!("{}.{}()", scrutinee_cpp, unwrap_m),
                Some(false) => format!("std::as_const({}).{}()", scrutinee_cpp, unwrap_m),
                Some(true) => format!("{}.as_mut().{}()", scrutinee_cpp, unwrap_m),
            }
        };

        // Emit bindings with local Rust-name → C++-name scope mapping.
        let mut binding_map = HashMap::new();
        // Cluster E: when the inner pattern requires a `variant_holds` check
        // (e.g. `if let Some(Inner::Root(root)) = ...`), open a second `if`
        // *inside* the outer `if`. Track whether we did so we can close it
        // after the body.
        let mut emitted_inner_variant_if = false;
        if let Some(pat) = binding_pat {
            let simple_ident = match pat {
                syn::Pat::Ident(pi) if pi.ident != "_" && pi.subpat.is_none() => {
                    Some(pi.ident.to_string())
                }
                _ => None,
            };

            if let Some(rust_name) = simple_ident {
                let cpp_name = self
                    .lookup_local_binding_cpp_name(&rust_name)
                    .unwrap_or_else(|| escape_cpp_keyword(&rust_name));
                binding_map.insert(rust_name, cpp_name.clone());
                if unwrap_method == IF_LET_OPTION_TAKE_VALUE_HELPER_MARKER {
                    self.writeln(&format!("auto&& _iflet_take = {};", scrutinee));
                    self.writeln(&format!(
                        "auto {} = rusty::detail::option_take_value(_iflet_take);",
                        cpp_name
                    ));
                } else {
                    let mut unwrap_scrutinee = scrutinee.to_string();
                    if scrutinee == cpp_name {
                        let local_scrutinee = "_iflet_bound_scrutinee";
                        self.writeln(&format!("auto&& {} = {};", local_scrutinee, scrutinee));
                        unwrap_scrutinee = local_scrutinee.to_string();
                    }
                    let unwrap_expr =
                        borrow_aware_unwrap_expr(unwrap_scrutinee.as_str(), unwrap_method);
                    // `if let ... = <option_or_result>.as_mut()` binds `&mut T` / `&mut E`.
                    // Those are represented as pointer-like unwrap values in C++ runtime types;
                    // bind through `auto&` to preserve one-layer borrow shape in downstream `&mut`
                    // call arguments (avoid producing pointer-to-pointer by accident).
                    if scrutinee_is_as_mut {
                        self.writeln(&format!(
                            "auto& {} = rusty::detail::deref_if_pointer_like({});",
                            cpp_name, unwrap_expr
                        ));
                    } else {
                        // Preserve both value and reference payload categories exactly.
                        self.writeln(&format!("decltype(auto) {} = {};", cpp_name, unwrap_expr));
                    }
                }
            } else {
                let payload_var = "_iflet_payload";
                if unwrap_method == IF_LET_OPTION_TAKE_VALUE_HELPER_MARKER {
                    self.writeln(&format!("auto&& _iflet_take = {};", scrutinee));
                    self.writeln(&format!(
                        "auto&& {} = rusty::detail::option_take_value(_iflet_take);",
                        payload_var
                    ));
                } else {
                    let unwrap_expr = borrow_aware_unwrap_expr(scrutinee, unwrap_method);
                    if scrutinee_is_as_mut {
                        self.writeln(&format!(
                            "auto&& {} = rusty::detail::deref_if_pointer_like({});",
                            payload_var, unwrap_expr
                        ));
                    } else {
                        self.writeln(&format!("auto&& {} = {};", payload_var, unwrap_expr));
                    }
                }

                // Cluster E: try the runtime variant lowering path first. If
                // the inner pattern is a known data-enum variant (e.g.
                // `Inner::Root(root)` when `Inner` is an enum visible in the
                // TU), the runtime path produces:
                //   - an extra `variant_holds<Inner_Root<...>>(_iflet_payload)`
                //     check (the inner condition), and
                //   - bindings shaped as `std::get<N>(_iflet_payload)._0`
                //     rather than the (incorrect) `_iflet_payload._0` that
                //     the non-runtime path emits for variants.
                let inner_variant_ctx =
                    self.infer_variant_type_context_from_pattern(pat, None);
                let mut runtime_stmts = Vec::new();
                let mut runtime_map = HashMap::new();
                let runtime_result = self
                    .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                        pat,
                        payload_var,
                        &mut runtime_stmts,
                        &mut runtime_map,
                        inner_variant_ctx.as_ref(),
                    );

                let use_runtime_variant_path = matches!(runtime_result, Some(Some(_)));
                if use_runtime_variant_path {
                    let inner_cond = match runtime_result {
                        Some(Some(c)) => c,
                        _ => unreachable!(),
                    };
                    self.writeln(&format!("if ({}) {{", inner_cond));
                    self.indent += 1;
                    emitted_inner_variant_if = true;
                    for stmt in runtime_stmts {
                        self.writeln(&stmt);
                    }
                    binding_map.extend(runtime_map);
                } else {
                    let mut binding_stmts = Vec::new();
                    if self.collect_pattern_binding_stmts_with_cpp_name_map(
                        pat,
                        payload_var,
                        &mut binding_stmts,
                        &mut binding_map,
                    ) {
                        for stmt in binding_stmts {
                            self.writeln(&stmt);
                        }
                    } else {
                        binding_map.clear();
                    }
                }
            }
        }

        if binding_map.is_empty() {
            self.emit_block(then_branch);
        } else {
            self.local_cpp_bindings.push(binding_map.clone());
            let mut local_types = HashMap::new();
            let mut local_consts = HashMap::new();
            for rust_name in binding_map.keys() {
                // Register the Some/Ok payload type for fn-pointer if-let bindings
                // (rare) so a later `binding(args)` call can detect an `unsafe fn`
                // (→ rusty::UnsafeFn) and lower it to `.call_unsafe(args)`. Other
                // bindings stay untyped, exactly as before.
                let ty =
                    self.if_let_some_binding_fn_pointer_type(binding_pat, scrutinee_expr, rust_name);
                local_types.insert(rust_name.clone(), ty);
                local_consts.insert(rust_name.clone(), false);
            }
            self.local_bindings.push(local_types);
            self.local_shadowed_binding_types.push(HashMap::new());
            self.local_const_bindings.push(local_consts);
            self.local_reference_bindings.push(HashSet::new());
            self.rebind_reference_pointer_bindings.push(HashSet::new());
            self.emit_block(then_branch);
            self.local_const_bindings.pop();
            self.local_reference_bindings.pop();
            self.rebind_reference_pointer_bindings.pop();
            self.local_bindings.pop();
            self.local_shadowed_binding_types.pop();
            self.local_cpp_bindings.pop();
        }
        // Cluster E: close the inner `if (variant_holds<...>(payload))`
        // block opened above for nested variant patterns.
        if emitted_inner_variant_if {
            self.indent -= 1;
            self.writeln("}");
        }
        self.indent -= 1;
        self.emit_if_let_else(else_branch);
    }

    /// Emit the else branch of an if-let, if present.
    pub(super) fn emit_if_let_else(&mut self, else_branch: &Option<(syn::token::Else, Box<syn::Expr>)>) {
        if let Some((_, else_expr)) = else_branch {
            match else_expr.as_ref() {
                syn::Expr::If(else_if) => {
                    self.write_indent();
                    self.output.push_str("} else ");
                    self.emit_if_inner(else_if, false);
                    return;
                }
                syn::Expr::Block(block) => {
                    self.writeln("} else {");
                    self.indent += 1;
                    self.emit_block(&block.block);
                    self.indent -= 1;
                }
                _ => {
                    let else_str = self.emit_expr_to_string(else_expr);
                    self.writeln("} else {");
                    self.indent += 1;
                    self.writeln(&format!("{};", else_str));
                    self.indent -= 1;
                }
            }
        }
        self.writeln("}");
    }

    pub(super) fn emit_if_inner(&mut self, if_expr: &syn::ExprIf, first: bool) {
        // Check for `if let` pattern
        if let syn::Expr::Let(let_expr) = &*if_expr.cond {
            self.emit_if_let(let_expr, &if_expr.then_branch, &if_expr.else_branch, first);
            return;
        }
        if self.try_emit_if_option_some_none_auto_return(if_expr, first) {
            return;
        }

        let cond = self.emit_expr_to_string(&if_expr.cond);
        if first {
            self.writeln(&format!("if ({}) {{", cond));
        } else {
            // Continuation of else-if chain — "} else if (...) {" already on current line
            self.output.push_str(&format!("if ({}) {{\n", cond));
        }
        self.indent += 1;
        self.emit_block(&if_expr.then_branch);
        self.indent -= 1;

        if let Some((_, else_branch)) = &if_expr.else_branch {
            match else_branch.as_ref() {
                syn::Expr::If(else_if) => {
                    self.write_indent();
                    self.output.push_str("} else ");
                    self.emit_if_inner(else_if, false);
                    return;
                }
                syn::Expr::Block(block) => {
                    self.writeln("} else {");
                    self.indent += 1;
                    self.emit_block(&block.block);
                    self.indent -= 1;
                }
                _ => {
                    let else_str = self.emit_expr_to_string(else_branch);
                    self.writeln("} else {");
                    self.indent += 1;
                    self.writeln(&format!("{};", else_str));
                    self.indent -= 1;
                }
            }
        }
        self.writeln("}");
    }

    /// For `let a = if let Some(v) = X { v } else { diverge }`, the type to give
    /// the (pre-block) result variable `a`: the scrutinee's Some-payload type
    /// `std::remove_cvref_t<decltype((X).unwrap())>`, which is in scope — unlike
    /// `decltype(v)`, where `v` is bound only inside the if-let block. Only fires
    /// when the then-tail is exactly the `Some(v)` payload binding.
    pub(super) fn if_let_some_payload_result_decl_type(
        &self,
        if_expr: &syn::ExprIf,
        then_tail: &syn::Expr,
    ) -> Option<String> {
        let syn::Expr::Let(let_expr) = if_expr.cond.as_ref() else {
            return None;
        };
        let binding = self.simple_some_payload_binding_name(&let_expr.pat)?;
        let syn::Expr::Path(p) = self.peel_paren_group_expr(then_tail) else {
            return None;
        };
        if p.qself.is_some()
            || p.path.segments.len() != 1
            || p.path.segments[0].ident != binding
        {
            return None;
        }
        let scrutinee_cpp = self.emit_expr_to_string(&let_expr.expr);
        Some(format!(
            "std::remove_cvref_t<decltype(({}).unwrap())>",
            scrutinee_cpp
        ))
    }

    pub(super) fn emit_local(&mut self, local: &syn::Local) {
        let pat = &local.pat;
        self.register_local_binding_pattern(pat);

        match pat {
            syn::Pat::Ident(pat_ident) => {
                let name = &pat_ident.ident;
                let name_str = name.to_string();
                let previous_cpp_name_in_current_scope = self
                    .local_cpp_bindings
                    .last()
                    .and_then(|scope| scope.get(&name_str).cloned());
                let cpp_name = self.allocate_local_cpp_name(&name_str);
                // If the new name shadows the Rust name, temporarily hide the
                // new mapping while emitting the init expression so that
                // `let rhs = rhs.next()` references the OUTER `rhs`, not the
                // newly allocated `rhs_shadow1`.
                let has_outer_same_rust_binding = self
                    .local_cpp_bindings
                    .iter()
                    .rev()
                    .skip(1)
                    .any(|scope| scope.contains_key(&name_str));
                let shadows_outer =
                    cpp_name != escape_cpp_keyword(&name_str) || has_outer_same_rust_binding;
                if shadows_outer {
                    // Temporarily remove the new mapping from the current scope.
                    // If there was a previous same-scope binding for this Rust name,
                    // restore it for initializer emission so `let x = x.next()` in
                    // same-scope shadow chains still resolves to the previous local.
                    if let Some(scope) = self.local_cpp_bindings.last_mut() {
                        scope.remove(&name_str);
                        if let Some(previous_cpp_name) = previous_cpp_name_in_current_scope.clone()
                        {
                            scope.insert(name_str.clone(), previous_cpp_name);
                        }
                    }
                }
                let shadows_param = self
                    .param_bindings
                    .last()
                    .is_some_and(|params| params.contains_key(&name_str));
                let track_in_progress_initializer = local.init.is_some();
                if track_in_progress_initializer {
                    self.push_in_progress_local_initializer(&name_str);
                }
                let is_mut = pat_ident.mutability.is_some();
                if local
                    .init
                    .as_ref()
                    .is_some_and(|init| self.expr_is_manually_drop_new_call(&init.expr))
                {
                    self.mark_local_manually_drop_binding(&name_str);
                }
                let mut inferred_binding_ty = local
                    .init
                    .as_ref()
                    .and_then(|init| self.infer_local_binding_type_from_initializer(&init.expr));
                if inferred_binding_ty.is_none()
                    && let Some(init) = local.init.as_ref()
                    && matches!(
                        self.peel_paren_group_expr(&init.expr),
                        syn::Expr::Reference(_)
                            | syn::Expr::Path(_)
                            | syn::Expr::Field(_)
                            | syn::Expr::RawAddr(_)
                            | syn::Expr::Unary(syn::ExprUnary {
                                op: syn::UnOp::Deref(_),
                                ..
                            })
                    )
                {
                    // Record the concrete type of an un-annotated `let x = <expr>`
                    // (const/static path like `NULL_STRING`, a field access, or a
                    // deref) so later `x.field` / `x.method()` / `x as *T` inference
                    // resolves instead of treating `x` as opaque `auto`. c2rust
                    // output is full of `let s = NULL_STRING; s.pointer.wrapping_offset(..)`.
                    inferred_binding_ty = self.infer_simple_expr_type(&init.expr);
                }
                if let Some(placeholder_ty) =
                    self.infer_local_type_from_placeholder_hint(local, &name_str)
                {
                    let should_override_inferred = inferred_binding_ty.as_ref().is_none_or(|ty| {
                        self.type_contains_infer(ty)
                            || self.type_contains_unresolved_placeholder_like(ty)
                            || self.type_contains_unbound_single_letter_generic(ty)
                            // Two-parameter generic owner shape: when
                            // the initializer-inferred type is a bare
                            // owner (`HashMap`) but the hint has full
                            // template args (`HashMap<K, V>`), the
                            // hint is the more-specific answer from
                            // the forward-scan pass and should win.
                            // Without this, the bare-owner inference
                            // satisfies `is_some()` and blocks the
                            // override, leaving the emit as
                            // `HashMap<auto, auto>::new_()`. See Ch.
                            // 13 of `docs/rusty-cpp-transpiler.md`.
                            || self.bare_owner_should_yield_to_specialized_hint(ty, &placeholder_ty)
                    });
                    if should_override_inferred {
                        inferred_binding_ty = Some(placeholder_ty);
                    }
                }
                // Fall back to a same-named field in the enclosing impl's
                // struct for constructor inits whose element type is otherwise
                // unresolved — `let comparators = Vec::new_();` inside an impl
                // whose struct has `comparators: Vec<i32>`. Trigger not only
                // when inference produced nothing, but also when it produced a
                // bare/placeholder owner (`Vec` / `Vec<auto>`) that the field
                // type can specialize.
                if let Some(field_ty) =
                    self.infer_local_binding_type_from_current_struct_field(local, &name_str)
                {
                    let should_use = inferred_binding_ty.as_ref().is_none_or(|ty| {
                        self.type_contains_infer(ty)
                            || self.type_contains_unresolved_placeholder_like(ty)
                            || self.bare_owner_should_yield_to_specialized_hint(ty, &field_ty)
                            || self.bare_owner_specialized_by_field_hint(ty, &field_ty)
                    });
                    if should_use {
                        inferred_binding_ty = Some(field_ty);
                    }
                }
                let has_generic_ctor_init = local
                    .init
                    .as_ref()
                    .is_some_and(|init| call_owner_placeholder_target(&init.expr).is_some());
                let generic_ctor_owner_target = if has_generic_ctor_init {
                    local
                        .init
                        .as_ref()
                        .and_then(|init| call_owner_placeholder_target(&init.expr))
                } else {
                    None
                };
                if inferred_binding_ty.is_none() {
                    // Check placeholder hints from forward-looking analysis.
                    // For uninitialized locals AND for `Type::new_()` calls
                    // where T was inferred from later usage (e.g., get_or_init).
                    if local.init.is_none()
                        || has_generic_ctor_init
                        || inferred_binding_ty.is_none()
                    {
                        if inferred_binding_ty.is_none() {
                            inferred_binding_ty =
                                self.lookup_local_placeholder_type_hint(&name_str).cloned();
                        }
                        if inferred_binding_ty.is_none()
                            && has_generic_ctor_init
                            && let Some(owner_target) = generic_ctor_owner_target.as_ref()
                            && let Some(return_hint) = self.current_return_type_hint().cloned()
                            && self
                                .expected_type_generic_args_for_owner(&return_hint, owner_target)
                                .is_some()
                            && !self.type_contains_infer(&return_hint)
                            && !self.type_contains_unresolved_placeholder_like(&return_hint)
                            && !self.type_contains_unbound_single_letter_generic(&return_hint)
                        {
                            inferred_binding_ty = Some(return_hint);
                        }
                        if inferred_binding_ty.is_none()
                            && has_generic_ctor_init
                            && let Some(owner_target) = generic_ctor_owner_target.as_ref()
                            && matches!(owner_target.as_str(), "OnceCell" | "OnceBox" | "Lazy")
                            && let Some(fallback_inner) = self
                                .lookup_local_placeholder_type_hint(
                                    ONCECELL_FALLBACK_INNER_HINT_KEY,
                                )
                                .cloned()
                        {
                            let normalized_inner = normalize_placeholder_hint_for_owner(
                                Some(owner_target.as_str()),
                                fallback_inner,
                            );
                            if !self.type_contains_infer(&normalized_inner)
                                && !self.type_contains_in_scope_type_param(&normalized_inner)
                                && !self
                                    .type_contains_unbound_single_letter_generic(&normalized_inner)
                                && !self
                                    .type_contains_unresolved_placeholder_like(&normalized_inner)
                            {
                                let owner_hint_str = format!(
                                    "{}<{}>",
                                    owner_target,
                                    quote::quote!(#normalized_inner)
                                );
                                inferred_binding_ty =
                                    syn::parse_str::<syn::Type>(&owner_hint_str).ok();
                            }
                        }
                    }
                }
                if has_generic_ctor_init
                    && let Some(owner_target) = generic_ctor_owner_target.as_ref()
                    && let Some(placeholder_ty) =
                        self.lookup_local_placeholder_type_hint(&name_str).cloned()
                {
                    let normalized_placeholder = normalize_placeholder_hint_for_owner(
                        Some(owner_target.as_str()),
                        placeholder_ty,
                    );
                    let placeholder_matches_owner = self
                        .expected_type_generic_args_for_owner(&normalized_placeholder, owner_target)
                        .is_some();
                    let inferred_is_weak = inferred_binding_ty.as_ref().is_none_or(|ty| {
                        self.type_contains_infer(ty)
                            || self.type_contains_unresolved_placeholder_like(ty)
                            || self.type_contains_unbound_single_letter_generic(ty)
                            || self
                                .expected_type_generic_args_for_owner(ty, owner_target)
                                .is_some_and(|owner_args| {
                                    owner_args.iter().any(|arg| {
                                        arg == "auto"
                                            || type_string_has_auto_placeholder(arg)
                                            || (Self::is_simple_ident(arg)
                                                && self.is_type_param_in_scope(arg))
                                    })
                                })
                    });
                    if placeholder_matches_owner
                        && inferred_is_weak
                        && !self.type_contains_infer(&normalized_placeholder)
                        && !self.type_contains_unresolved_placeholder_like(&normalized_placeholder)
                        && !self
                            .type_contains_unbound_single_letter_generic(&normalized_placeholder)
                    {
                        inferred_binding_ty = Some(normalized_placeholder);
                    }
                }
                let inferred_is_weak_generic_ctor_hint = has_generic_ctor_init
                    && inferred_binding_ty.as_ref().is_some_and(|ty| {
                        self.type_contains_infer(ty)
                            || self.type_contains_unresolved_placeholder_like(ty)
                            || self.type_contains_unbound_single_letter_generic(ty)
                            || generic_ctor_owner_target
                                .as_ref()
                                .and_then(|owner| {
                                    self.expected_type_generic_args_for_owner(ty, owner)
                                })
                                .is_some_and(|owner_args| {
                                    !owner_args.is_empty()
                                        && owner_args.iter().all(|arg| {
                                            Self::is_simple_ident(arg)
                                                && self.is_type_param_in_scope(arg)
                                        })
                                })
                    });
                if inferred_is_weak_generic_ctor_hint
                    && let Some(owner_target) = generic_ctor_owner_target.as_ref()
                    && let Some(return_hint) = self.current_return_type_hint().cloned()
                    && self
                        .expected_type_generic_args_for_owner(&return_hint, owner_target)
                        .is_some()
                    && !self.type_contains_infer(&return_hint)
                    && !self.type_contains_unresolved_placeholder_like(&return_hint)
                    && !self.type_contains_unbound_single_letter_generic(&return_hint)
                {
                    inferred_binding_ty = Some(return_hint);
                }
                if let Some(ty) = inferred_binding_ty.clone() {
                    self.update_local_binding_type(name_str.clone(), ty);
                }
                let uninitialized_inferred_ty = if local.init.is_none() {
                    inferred_binding_ty
                        .as_ref()
                        .filter(|ty| !self.type_contains_infer(ty))
                        .cloned()
                } else {
                    None
                };
                let inferred_mut_reference_binding = inferred_binding_ty
                    .as_ref()
                    .is_some_and(|ty| Self::is_mut_reference_type(ty));

                let type_str = if let Some(ty) = get_local_type(local) {
                    self.map_type(ty)
                } else if local.init.is_some() && inferred_mut_reference_binding {
                    self.map_type(
                        inferred_binding_ty
                            .as_ref()
                            .expect("mutable reference inference should exist when selected"),
                    )
                } else if local.init.is_none()
                    && uninitialized_inferred_ty
                        .as_ref()
                        .is_some_and(|ty| self.should_use_optional_delayed_init_storage(ty))
                {
                    self.map_type(
                        uninitialized_inferred_ty
                            .as_ref()
                            .expect("checked Some in condition"),
                    )
                } else if local
                    .init
                    .as_ref()
                    .is_some_and(|init| expr_is_option_none_constructor(&init.expr))
                    && inferred_binding_ty
                        .as_ref()
                        .is_some_and(|ty| self.is_option_like_syn_type(ty))
                {
                    self.map_type(
                        inferred_binding_ty
                            .as_ref()
                            .expect("inferred Option type should exist when selected"),
                    )
                } else if self.should_emit_inferred_sum_type_for_local(
                    local,
                    &name_str,
                    inferred_binding_ty.as_ref(),
                ) || self.should_emit_inferred_numeric_seed_type_for_local(
                    local,
                    &name_str,
                    inferred_binding_ty.as_ref(),
                ) {
                    self.map_type(
                        inferred_binding_ty
                            .as_ref()
                            .expect("inferred sum type should exist when selected"),
                    )
                } else if inferred_binding_ty
                    .as_ref()
                    .is_some_and(|ty| self.type_is_single_in_scope_type_param(ty))
                {
                    self.map_type(
                        inferred_binding_ty
                            .as_ref()
                            .expect("inferred type parameter should exist when selected"),
                    )
                } else {
                    "auto".to_string()
                };

                let is_consumed = self.consuming_method_receiver_vars.contains(&name_str);
                // `let ref ... = expr` should stay a reference binding shape even if
                // conservative move heuristics classify the local as "consumed".
                let emits_ref_binding = pat_ident.by_ref.is_some()
                    && !(is_mut
                        && local
                            .init
                            .as_ref()
                            .is_some_and(|init| self.is_rvalue_expr(&init.expr)));
                let init_returns_mut_reference = local.init.as_ref().is_some_and(|init| {
                    if self
                        .infer_local_binding_type_from_initializer(&init.expr)
                        .as_ref()
                        .is_some_and(|ty| Self::is_mut_reference_type(ty))
                    {
                        return true;
                    }
                    // Peel through Expr::Unsafe / Expr::Block / Expr::Paren /
                    // Expr::Group to find a method call at the tail position.
                    // `let x = unsafe { recv.method() };` is a common shape
                    // for &mut T-returning unsafe methods (e.g. reborrow).
                    let peeled = peel_to_tail_expr(&init.expr);
                    if let Some(syn::Expr::MethodCall(mc)) = peeled {
                        let method = mc.method.to_string();
                        // `Arc<T>::get_mut()` returns `Option<&mut T>` BY VALUE,
                        // not `&mut T`. Skip the generic `get_mut → &mut T`
                        // shortcut when the receiver is actually an `Arc` —
                        // otherwise we'd emit `auto& opt = …` which can't
                        // bind to the by-value `Option` temporary.
                        let receiver_is_arc_get_mut = matches!(method.as_str(), "get_mut")
                            && self.receiver_is_arc_wrapper_type(&mc.receiver);
                        if receiver_is_arc_get_mut {
                            return false;
                        }
                        return matches!(
                            method.as_str(),
                            "get_mut"
                                | "force_mut"
                                | "as_mut"
                                | "deref_mut"
                                | "into_mut"
                                | "reborrow"
                        ) || method.ends_with("_mut");
                    }
                    if let Some(syn::Expr::Call(call)) = peeled
                        && let syn::Expr::Path(path_expr) = call.func.as_ref()
                    {
                        let method = path_expr
                            .path
                            .segments
                            .last()
                            .map(|seg| seg.ident.to_string())
                            .unwrap_or_default();
                        return matches!(
                            method.as_str(),
                            "get_mut"
                                | "force_mut"
                                | "as_mut"
                                | "deref_mut"
                                | "into_mut"
                                | "reborrow"
                        ) || method.ends_with("_mut");
                    }
                    if let syn::Expr::If(if_expr) = self.peel_paren_group_expr(&init.expr) {
                        let branch_mut_like = |branch: &syn::Expr| {
                            self.infer_local_binding_type_from_initializer(branch)
                                .or_else(|| self.infer_simple_expr_type(branch))
                                .as_ref()
                                .is_some_and(Self::is_mut_reference_type)
                                || self.expr_method_chain_contains_reference_method(branch)
                        };
                        let then_expr = self
                            .extract_tail_expr_from_block(&if_expr.then_branch)
                            .or_else(|| self.extract_single_expr_from_block(&if_expr.then_branch));
                        let else_expr = if_expr
                            .else_branch
                            .as_ref()
                            .and_then(|(_, expr)| self.extract_value_expr(expr));
                        if let (Some(then_expr), Some(else_expr)) = (then_expr, else_expr)
                            && branch_mut_like(then_expr)
                            && branch_mut_like(else_expr)
                        {
                            return true;
                        }
                        return self
                            .infer_common_value_type_from_if(if_expr)
                            .as_ref()
                            .is_some_and(Self::is_mut_reference_type);
                    }
                    false
                });
                // Detect method/function/branch expressions that propagate references.
                // This keeps local binding shape as reference (`auto&`) instead of value copy.
                let init_returns_reference = local.init.as_ref().is_some_and(|init| {
                    // Guard: methods on the primitive-optimized once_cell
                    // variants (OnceNonZeroUsize, OnceBool) return their
                    // payload BY VALUE (`NonZeroUsize`, `bool`), unlike
                    // `OnceCell<T>::get_unchecked() -> &T`. Without this
                    // explicit shortcut, `lookup_known_method_return_type_by_name`
                    // below sees one of the `OnceCell::get_unchecked() -> &T`
                    // signatures, returns reference-like, and we emit
                    // `auto& value = cell.get_unchecked()` which binds a
                    // non-const lvalue ref to the by-value temporary.
                    let peeled_for_guard = peel_to_tail_expr(&init.expr);
                    if let Some(syn::Expr::MethodCall(mc)) = peeled_for_guard {
                        let method = mc.method.to_string();
                        if matches!(
                            method.as_str(),
                            "get_unchecked" | "get" | "get_or_init" | "get_or_try_init"
                        ) && self
                            .method_call_receiver_owner_tail(&mc.receiver)
                            .is_some_and(|owner| {
                                matches!(owner.as_str(), "OnceNonZeroUsize" | "OnceBool")
                            })
                        {
                            return false;
                        }
                    }
                    if self
                        .infer_local_binding_type_from_initializer(&init.expr)
                        .as_ref()
                        .is_some_and(|ty| {
                            self.type_is_reference_like(ty)
                                && !self.reference_type_lowers_to_value_cpp(ty)
                        })
                    {
                        return true;
                    }
                    // Peel through Expr::Unsafe / Expr::Block / Expr::Paren /
                    // Expr::Group to find a method call at the tail position.
                    let peeled_ref = peel_to_tail_expr(&init.expr);
                    if let Some(syn::Expr::MethodCall(mc)) = peeled_ref {
                        let method = mc.method.to_string();
                        if matches!(method.as_str(), "unwrap" | "unwrap_unchecked" | "expect")
                            && self.expr_method_chain_contains_reference_method(&mc.receiver)
                        {
                            return true;
                        }
                        // `RefCell::borrow_mut()` returns `RefMut<T>` BY VALUE,
                        // not `&mut T`. Skip the `borrow_mut` shortcut when the
                        // receiver is actually a `RefCell` — otherwise we'd
                        // emit `auto& guard = …` which can't bind to the
                        // by-value temporary.
                        let receiver_is_refcell_borrow = matches!(method.as_str(), "borrow_mut")
                            && self.receiver_is_refcell_container_type(&mc.receiver);
                        // Same shape: `Arc<T>::get_mut()` returns `Option<&mut T>`
                        // by value, not `&mut T`. Suppress the generic
                        // `get_mut → &mut T` heuristic when the receiver is an `Arc`.
                        let receiver_is_arc_get_mut = matches!(method.as_str(), "get_mut")
                            && self.receiver_is_arc_wrapper_type(&mc.receiver);
                        // `OnceNonZeroUsize::get_unchecked()` / `OnceBool::get_unchecked()`
                        // — the primitive-optimized once_cell variants — return their
                        // inner value BY VALUE (`NonZeroUsize` / `bool`), not by ref
                        // like `OnceCell<T>::get_unchecked() -> &T`. Without this
                        // filter once_cell's tests emit `auto& value =
                        // OnceNonZeroUsize::get_unchecked()` which binds a non-const
                        // lvalue ref to a temporary and fails to compile.
                        let receiver_is_value_returning_once = matches!(
                            method.as_str(),
                            "get_unchecked" | "get" | "get_or_init" | "get_or_try_init"
                        ) && self
                            .method_call_receiver_owner_tail(&mc.receiver)
                            .is_some_and(|owner| matches!(
                                owner.as_str(),
                                "OnceNonZeroUsize" | "OnceBool"
                            ));
                        // `slice.get_unchecked(a..b)` / `get_unchecked(a..)` — the
                        // RANGE-argument form — returns a SUB-SLICE, which lowers to
                        // a `std::span<…>` BY VALUE (rusty::slice_from), not an
                        // element reference. Binding as `auto&` would bind a
                        // non-const lvalue ref to that span temporary. Only the
                        // SCALAR-index form `get_unchecked(i)` yields a true element
                        // reference, so restrict the suppression to the range form.
                        let receiver_is_range_get_unchecked = matches!(
                            method.as_str(),
                            "get_unchecked" | "get_unchecked_mut"
                        ) && mc.args.len() == 1
                            && matches!(
                                self.peel_paren_group_expr(&mc.args[0]),
                                syn::Expr::Range(_)
                            );
                        if !receiver_is_refcell_borrow
                            && !receiver_is_arc_get_mut
                            && !receiver_is_value_returning_once
                            && !receiver_is_range_get_unchecked
                            && (matches!(
                                method.as_str(),
                                "get_or_init"
                                    | "get_or_try_init"
                                    | "get_mut"
                                    | "get_unchecked"
                                    | "wait"
                                    | "force"
                                    | "force_mut"
                                    | "as_mut"
                                    | "deref_mut"
                                    | "borrow_mut"
                                    | "into_mut"
                                    | "reborrow"
                            ) || method.ends_with("_mut")
                                || method.ends_with("_ref"))
                        {
                            return true;
                        }
                        // Final fallback before per-receiver inference: search every
                        // impl-block we know about for a method with this name. If every
                        // matching definition returns a reference, the name itself is a
                        // reliable signal (e.g. `into_leaf` returning `&'a LeafNode`).
                        // The lookup returns `None` for ambiguous names (reborrow has
                        // both `&mut T`-returning and value-returning impls), so this is
                        // safe to consult without a receiver-type filter.
                        if let Some(ty) = self.lookup_known_method_return_type_by_name(&method) {
                            if self.type_is_reference_like(&ty) {
                                return true;
                            }
                        }
                        if self
                            .infer_method_call_result_type_for_local(mc)
                            .as_ref()
                            .is_some_and(|ty| {
                                self.type_is_reference_like(ty)
                                    && !self.reference_type_lowers_to_value_cpp(ty)
                            })
                        {
                            return true;
                        }
                    } else if let syn::Expr::Call(call) = init.expr.as_ref()
                        && (self
                            .lookup_associated_call_return_type(call)
                            .as_ref()
                            .is_some_and(|ty| {
                                self.type_is_reference_like(ty)
                                    && !self.reference_type_lowers_to_value_cpp(ty)
                            })
                            || self.associated_call_is_reference_like_by_shape(call))
                    {
                        return true;
                    }
                    if let syn::Expr::If(if_expr) = self.peel_paren_group_expr(&init.expr) {
                        let branch_ref_like = |branch: &syn::Expr| {
                            self.infer_local_binding_type_from_initializer(branch)
                                .or_else(|| self.infer_simple_expr_type(branch))
                                .as_ref()
                                .is_some_and(|ty| {
                                    self.type_is_reference_like(ty)
                                        && !self.reference_type_lowers_to_value_cpp(ty)
                                })
                                || self.expr_method_chain_contains_reference_method(branch)
                                || (self.expr_is_reference_yielding(branch)
                                    && !self.expr_reference_type_lowers_to_value_cpp(branch))
                        };
                        let then_expr = self
                            .extract_tail_expr_from_block(&if_expr.then_branch)
                            .or_else(|| self.extract_single_expr_from_block(&if_expr.then_branch));
                        let else_expr = if_expr
                            .else_branch
                            .as_ref()
                            .and_then(|(_, expr)| self.extract_value_expr(expr));
                        if let (Some(then_expr), Some(else_expr)) = (then_expr, else_expr)
                            && branch_ref_like(then_expr)
                            && branch_ref_like(else_expr)
                        {
                            return true;
                        }
                        if self
                            .infer_common_value_type_from_if(if_expr)
                            .as_ref()
                            .is_some_and(|ty| {
                                self.type_is_reference_like(ty)
                                    && !self.reference_type_lowers_to_value_cpp(ty)
                            })
                        {
                            return true;
                        }
                    }
                    if let syn::Expr::If(_) = self.peel_paren_group_expr(&init.expr)
                        && self.expr_method_chain_contains_reference_method(&init.expr)
                    {
                        return true;
                    }
                    matches!(
                        self.peel_paren_group_expr(&init.expr),
                        syn::Expr::If(_) | syn::Expr::Match(_)
                    ) && self.expr_is_reference_yielding(&init.expr)
                        && !self.expr_reference_type_lowers_to_value_cpp(&init.expr)
                });
                let init_returns_reference_by_shape = local.init.as_ref().is_some_and(|init| {
                    match self.peel_paren_group_expr(&init.expr) {
                        syn::Expr::MethodCall(mc) => {
                            self.method_call_is_reference_like_by_shape(mc)
                        }
                        syn::Expr::Call(call) => {
                            self.associated_call_is_reference_like_by_shape(call)
                        }
                        _ => false,
                    }
                });
                let init_returns_reference_binding = (init_returns_reference
                    || init_returns_reference_by_shape)
                    && !local.init.as_ref().is_some_and(|init| {
                        !self.is_ref_init(&init.expr)
                            && self.is_rvalue_expr(&init.expr)
                            && !self.expr_is_reference_yielding(&init.expr)
                    });
                // Cluster B: `let val = unsafe { ptr::read(v) };` is a
                // bit-extraction that almost always gets handed straight to a
                // move-consuming destination (e.g. `f(val)` where f takes T
                // by value). Marking `val` as `const auto` makes the implicit
                // `std::move(val)` collapse to `const T&&` — which can't
                // bind to T-by-value sinks for move-only T, and silently
                // copies for copyable T (defeating the whole point of the
                // bit-extraction). Detect ptr::read inits and force non-const.
                let init_is_ptr_read =
                    local.init.as_ref().is_some_and(|init| {
                        Self::expr_is_ptr_read_init(self.peel_paren_group_expr(&init.expr))
                    });
                // `let guard = mutex.lock().unwrap();` (and `try_lock`,
                // RwLock `write()`) yields a `MutexGuard<T>` / `SpinMutexGuard<T>` /
                // `RwLockWriteGuard<T>` BY VALUE. The guard provides write
                // access to the protected value via `operator*` / `operator->`,
                // so the local must be NON-const — otherwise `*guard = expr` or
                // `guard->mut_method()` won't compile (the `const T&` /
                // `const T*` overloads of operator*/-> would be selected).
                // Same shape applies to `RefCell::borrow_mut()` returning
                // `RefMut<T>` by value, but that path is already covered by
                // the `_mut` suffix in `init_returns_mut_reference`.
                let init_returns_writable_guard_by_value =
                    local.init.as_ref().is_some_and(|init| {
                        let ty = self
                            .infer_local_binding_type_from_initializer(&init.expr)
                            .or_else(|| self.infer_simple_expr_type(&init.expr));
                        ty.as_ref().is_some_and(|ty| {
                            let peeled = self.peel_reference_paren_group_type(ty);
                            matches!(peeled, syn::Type::Path(tp)
                                if tp.path.segments.last().is_some_and(|seg| matches!(
                                    seg.ident.to_string().as_str(),
                                    "MutexGuard" | "SpinMutexGuard" | "RwLockWriteGuard" | "RefMut"
                                )))
                        })
                    });
                let qualifier = if emits_ref_binding {
                    if is_mut { "" } else { "const " }
                } else if is_mut
                    || is_consumed
                    || init_is_ptr_read
                    || self.mutable_pointer_aliased_vars.contains(&name_str)
                    || self.deref_assigned_vars.contains(&name_str)
                    || inferred_mut_reference_binding
                    || init_returns_mut_reference
                    || init_returns_reference_binding
                    || init_returns_writable_guard_by_value
                    || type_str.trim_start().starts_with("const ")
                {
                    ""
                } else {
                    "const "
                };
                // For immutable `let p: *mut T = ...`, preserve Rust binding immutability
                // without changing mutable pointee semantics:
                // emit `T* const p`, not `const T* p`.
                let mut_raw_ptr_binding = get_local_type(local)
                    .is_some_and(|ty| Self::is_mut_raw_pointer_type(ty))
                    || inferred_binding_ty
                        .as_ref()
                        .is_some_and(|ty| Self::is_mut_raw_pointer_type(ty));
                // `let ref r = expr` → `const auto& r = expr;` (reference binding).
                // Exception: `let ref mut r = rvalue_expr;` — when the init is
                // an rvalue (e.g., function call), binding a mutable reference
                // to it is invalid in C++. Emit as owned value instead.
                if init_returns_reference_binding {
                    let inferred_ref_from_init =
                        local
                            .init
                            .as_ref()
                            .and_then(|init| match init.expr.as_ref() {
                                syn::Expr::MethodCall(mc) => self
                                    .infer_method_call_result_type_for_local(mc)
                                    .filter(|ty| self.type_is_reference_like(ty)),
                                syn::Expr::Call(call) => self
                                    .lookup_associated_call_return_type(call)
                                    .or_else(|| {
                                        self.lookup_function_return_type(call.func.as_ref())
                                            .cloned()
                                    })
                                    .filter(|ty| self.type_is_reference_like(ty)),
                                _ => None,
                            });
                    let promoted_ref_ty = inferred_ref_from_init
                        .or_else(|| {
                            self.lookup_local_binding_type(&name_str).and_then(|ty| {
                                if self.type_is_reference_like(&ty) {
                                    Some(ty)
                                } else {
                                    let promoted: syn::Type = if is_mut {
                                        parse_quote!(&mut #ty)
                                    } else {
                                        parse_quote!(&#ty)
                                    };
                                    Some(promoted)
                                }
                            })
                        })
                        .or_else(|| {
                            let placeholder_ref: syn::Type = if is_mut {
                                parse_quote!(&mut _)
                            } else {
                                parse_quote!(&_)
                            };
                            Some(placeholder_ref)
                        });
                    if let Some(promoted_ref_ty) = promoted_ref_ty {
                        self.update_local_binding_type(name_str.clone(), promoted_ref_ty);
                    }
                }
                let ref_suffix = if emits_ref_binding || init_returns_reference_binding {
                    "&"
                } else {
                    ""
                };
                let init_is_ref_binding = local
                    .init
                    .as_ref()
                    .is_some_and(|init| self.is_ref_init(&init.expr));
                let local_decl_is_reference = ref_suffix == "&"
                    || init_is_ref_binding
                    || inferred_binding_ty.as_ref().is_some_and(|ty| {
                        self.type_is_reference_like(ty)
                            && !self.reference_type_lowers_to_value_cpp(ty)
                    });
                let needs_rebind_pointer_local =
                    is_mut && self.reassigned_vars.contains(&name_str) && local_decl_is_reference;
                let rebind_pointer_decl_type = if needs_rebind_pointer_local {
                    get_local_type(local)
                        .and_then(|ty| self.map_reference_type_to_pointer_cpp_type(ty))
                        .or_else(|| {
                            inferred_binding_ty
                                .as_ref()
                                .and_then(|ty| self.map_reference_type_to_pointer_cpp_type(ty))
                        })
                        .or_else(|| {
                            local
                                .init
                                .as_ref()
                                .map(|init| self.map_ref_as_pointer_type(local, &init.expr))
                        })
                        .or(Some("auto*".to_string()))
                } else {
                    None
                };
                let effective_ref_suffix = if ref_suffix.is_empty() || type_str.contains('&') {
                    ""
                } else {
                    ref_suffix
                };
                let decl_type = if let Some(ptr_decl) = rebind_pointer_decl_type.as_ref() {
                    ptr_decl.clone()
                } else if qualifier == "const " && mut_raw_ptr_binding && type_str.contains('*') {
                    format!("{} const", type_str)
                } else {
                    format!("{}{}{}", qualifier, type_str, effective_ref_suffix)
                };
                let decl_type = if decl_type == "auto"
                    && local.init.as_ref().is_some_and(|init| {
                        matches!(
                            self.peel_paren_group_expr(&init.expr),
                            syn::Expr::Match(match_expr)
                                if self.match_expr_has_callable_passthrough_arm(match_expr)
                        )
                    }) {
                    "decltype(auto)".to_string()
                } else {
                    decl_type
                };
                self.record_local_const_binding(
                    &name_str,
                    qualifier == "const " && local.init.is_some(),
                );
                self.record_local_reference_binding(&name_str, local_decl_is_reference);
                if needs_rebind_pointer_local {
                    self.record_rebind_reference_pointer_binding(&name_str);
                }

                if shadows_param && previous_cpp_name_in_current_scope.is_none() {
                    self.local_cpp_bindings
                        .last_mut()
                        .and_then(|scope| scope.remove(&name_str));
                }

                if let Some(init) = &local.init {
                    let force_move_from_consumed_ref_local = is_consumed
                        && self.should_force_move_consumed_local_initializer_expr(&init.expr);
                    // Special case: `let x = if let Some(y) = ... { ...?... } else { val };`
                    // Emit as statement block to keep ? in outer function scope.
                    if let Some(syn::Expr::If(if_expr)) =
                        self.extract_single_value_expr_deep(&init.expr)
                    {
                        if self.block_contains_early_return_or_try(&if_expr.then_branch)
                            || if_expr.else_branch.as_ref().is_some_and(|(_, else_expr)| {
                                self.expr_contains_early_return_or_try(else_expr)
                            })
                        {
                            if self
                                .emit_single_if_let_as_statement_block(
                                    &cpp_name, &decl_type, if_expr,
                                )
                                .is_some()
                            {
                                if let Some(scope) = self.local_cpp_bindings.last_mut() {
                                    scope.insert(name_str.clone(), cpp_name.clone());
                                }
                                if let Some(used) = self.local_cpp_names_used.last_mut() {
                                    used.insert(cpp_name.clone());
                                }
                                if track_in_progress_initializer {
                                    self.pop_in_progress_local_initializer();
                                }
                                return;
                            }
                        }
                    }
                    if self.try_emit_local_match_break_initializer(
                        local,
                        &cpp_name,
                        &decl_type,
                        inferred_binding_ty.as_ref(),
                    ) {
                        if let Some(scope) = self.local_cpp_bindings.last_mut() {
                            scope.insert(name_str.clone(), cpp_name.clone());
                        }
                        if let Some(used) = self.local_cpp_names_used.last_mut() {
                            used.insert(cpp_name.clone());
                        }
                        if track_in_progress_initializer {
                            self.pop_in_progress_local_initializer();
                        }
                        return;
                    }
                    // Special case: `let x = loop { ... break val; }` → lambda wrapper
                    if let syn::Expr::Loop(loop_expr) = init.expr.as_ref() {
                        self.writeln(&format!("{} {} = [&]() {{", decl_type, cpp_name));
                        self.indent += 1;
                        self.writeln("while (true) {");
                        self.indent += 1;
                        self.emit_block(&loop_expr.body);
                        self.indent -= 1;
                        self.writeln("}");
                        self.indent -= 1;
                        self.writeln("}();");
                    } else if self.is_ref_init(&init.expr) {
                        let inner = self.extract_ref_inner(&init.expr);
                        if needs_rebind_pointer_local {
                            // Reference rebinding detected: `let mut r = &x; ... r = &y;`
                            // Emit as pointer instead of reference
                            let ptr_type = rebind_pointer_decl_type
                                .clone()
                                .unwrap_or_else(|| self.map_ref_as_pointer_type(local, &init.expr));
                            self.writeln(&format!("{} {} = &{};", ptr_type, cpp_name, inner));
                        } else {
                            // No rebinding: `let r = &x` → `const auto& r = x`
                            //               `let r = &mut x` → `auto& r = x`
                            let is_mut_ref = matches!(&*init.expr, syn::Expr::Reference(r) if r.mutability.is_some());
                            let ref_qualifier = if is_mut_ref { "" } else { "const " };
                            let ref_type = self.map_ref_as_ref_type(local, &init.expr);
                            self.writeln(&format!(
                                "{}{} {} = {};",
                                ref_qualifier, ref_type, cpp_name, inner
                            ));
                        }
                    } else {
                        // For `let y = x` where x is a local variable → insert std::move
                        //
                        // Constructor-hint recovery walks nested expressions before we emit the
                        // initializer itself. Temporarily hide the current binding so closures
                        // like `let t = || Ok(t.do_thing()?)` don't resolve `t` to the outer
                        // `t_shadowN` local and create self-referential `decltype` hints.
                        let hidden_current_binding = self
                            .local_cpp_bindings
                            .last_mut()
                            .and_then(|scope| scope.remove(&name_str));
                        let recovered_hints =
                            self.recover_constructor_template_hints_from_expr(&init.expr);
                        if let Some(current_cpp_name) = hidden_current_binding {
                            if let Some(scope) = self.local_cpp_bindings.last_mut() {
                                scope.insert(name_str.clone(), current_cpp_name);
                            }
                        }
                        let pushed_hints = !recovered_hints.is_empty();
                        if pushed_hints {
                            self.constructor_template_hints.push(recovered_hints);
                        }
                        let expr_str = if let Some(callable_item_expr) =
                            self.emit_callable_path_item_expr(&init.expr)
                        {
                            callable_item_expr
                        } else if get_local_type(local).is_none() {
                            if let syn::Expr::Repeat(repeat) = init.expr.as_ref() {
                                if let Some(elem_hint) = self.repeat_elem_type_hints.get(&name_str)
                                {
                                    self.emit_repeat_expr_with_element_hint(repeat, elem_hint)
                                } else if let Some(ty) = inferred_binding_ty.as_ref() {
                                    {
                                        self.emit_expr_to_string_with_expected(&init.expr, Some(ty))
                                    }
                                } else {
                                    self.emit_expr_maybe_move(&init.expr)
                                }
                            } else if let Some(ty) = inferred_binding_ty.as_ref() {
                                if self.should_skip_expected_cast_for_inferred_as_ptr_u8_fallback(
                                    &init.expr, ty,
                                ) {
                                    self.emit_expr_maybe_move(&init.expr)
                                } else {
                                    // For zero-arg generic constructor calls (OnceCell::new_(), Box::new()),
                                    // wrap the inferred inner type T into Owner<T> so the call
                                    // handler can recover template args.
                                    let is_zero_arg_ctor = local.init.as_ref()
                                        .is_some_and(|init| matches!(init.expr.as_ref(), syn::Expr::Call(c) if c.args.is_empty()));
                                    let effective_ty = if is_zero_arg_ctor {
                                        local.init.as_ref()
                                            .and_then(|init| call_owner_placeholder_target(&init.expr))
                                            .and_then(|owner| {
                                                if matches!(owner.as_str(), "OnceCell" | "OnceBox" | "Lazy") {
                                                    let owner_matches_hint = matches!(
                                                        self.peel_reference_paren_group_type(ty),
                                                        syn::Type::Path(tp)
                                                            if tp.path.segments.last().is_some_and(|seg| seg.ident.to_string() == owner)
                                                    );
                                                    if owner_matches_hint {
                                                        Some(ty.clone())
                                                    } else {
                                                        let owner_inner_ty = if owner == "OnceBox" {
                                                            let peeled = self.peel_reference_paren_group_type(ty);
                                                            if let syn::Type::Path(tp) = peeled {
                                                                if let Some(last) = tp.path.segments.last() {
                                                                    if last.ident == "Box" {
                                                                        if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                                                                            args.args.iter().find_map(|arg| match arg {
                                                                                syn::GenericArgument::Type(inner) => Some(inner.clone()),
                                                                                _ => None,
                                                                            })
                                                                        } else {
                                                                            None
                                                                        }
                                                                    } else {
                                                                        None
                                                                    }
                                                                } else {
                                                                    None
                                                                }
                                                            } else {
                                                                None
                                                            }
                                                        } else {
                                                            None
                                                        };
                                                        let wrapped_inner = owner_inner_ty.unwrap_or_else(|| ty.clone());
                                                        syn::parse_str::<syn::Type>(
                                                            &format!(
                                                                "{}<{}>",
                                                                owner,
                                                                quote::quote!(#wrapped_inner)
                                                            )
                                                        ).ok()
                                                    }
                                                } else {
                                                    None
                                                }
                                            })
                                    } else {
                                        None
                                    };
                                    let expected = effective_ty.as_ref().unwrap_or(ty);
                                    let expected = if effective_ty.is_none()
                                        && self
                                            .should_suppress_inferred_expected_for_struct_literal(
                                                &init.expr, expected,
                                            ) {
                                        None
                                    } else {
                                        Some(expected)
                                    };
                                    self.emit_expr_to_string_with_expected_and_move_if_needed(
                                        &init.expr, expected,
                                    )
                                }
                            } else if let syn::Expr::Match(match_expr) =
                                self.peel_paren_group_expr(&init.expr)
                                && match_expr.arms.len() <= 4
                                && self.expr_contains_early_return_or_try(&init.expr)
                                && let Some(match_ty) = self
                                    .infer_match_arms_common_type(&match_expr.arms)
                                    .or_else(|| {
                                        self.infer_match_arms_common_type_with_scrutinee(match_expr)
                                    })
                            {
                                self.emit_expr_to_string_with_expected_and_move_if_needed(
                                    &init.expr,
                                    Some(&match_ty),
                                )
                            } else {
                                self.emit_expr_maybe_move(&init.expr)
                            }
                        } else if let Some(ty) = inferred_binding_ty
                            .as_ref()
                            .or_else(|| get_local_type(local))
                        {
                            self.emit_expr_to_string_with_expected(&init.expr, Some(ty))
                        } else {
                            self.emit_expr_maybe_move(&init.expr)
                        };
                        let expr_str = if force_move_from_consumed_ref_local {
                            format!("std::move({})", expr_str)
                        } else {
                            expr_str
                        };
                        let expr_str = if self.expr_is_reference_yielding(&init.expr)
                            && expr_str.starts_with("std::move(")
                        {
                            self.emit_expr_to_string(&init.expr)
                        } else {
                            expr_str
                        };
                        let expr_str = if needs_rebind_pointer_local {
                            let init_ref_like_by_binding = inferred_binding_ty
                                .as_ref()
                                .is_some_and(|ty| self.type_is_reference_like(ty))
                                || get_local_type(local)
                                    .is_some_and(|ty| self.type_is_reference_like(ty));
                            if expr_str.starts_with('&') {
                                expr_str
                            } else if self.expr_is_reference_yielding(&init.expr)
                                || init_ref_like_by_binding
                            {
                                self.emit_try_expr_reference_pointer(&init.expr)
                                    .unwrap_or_else(|| format!("&({})", expr_str))
                            } else {
                                expr_str
                            }
                        } else {
                            expr_str
                        };
                        if pushed_hints {
                            self.constructor_template_hints.pop();
                        }
                        if local_decl_is_reference
                            && !needs_rebind_pointer_local
                            && self.expr_tree_has_try(&init.expr)
                        {
                            if let Some(ptr_expr) = self.emit_try_expr_reference_pointer(&init.expr)
                            {
                                let ptr_name = self.reserve_synthetic_cpp_name(&format!(
                                    "{}_try_ref_ptr",
                                    cpp_name
                                ));
                                let ptr_decl = if decl_type.trim_start().starts_with("const ") {
                                    "const auto*"
                                } else {
                                    "auto*"
                                };
                                self.writeln(&format!("{} {} = {};", ptr_decl, ptr_name, ptr_expr));
                                self.writeln(&format!(
                                    "{} {} = *{};",
                                    decl_type, cpp_name, ptr_name
                                ));
                            } else {
                                self.writeln(&format!(
                                    "{} {} = {};",
                                    decl_type, cpp_name, expr_str
                                ));
                            }
                        } else {
                            // Detect self-reference: if the initializer expression
                            // references the same name being declared (e.g.,
                            // `auto right = ptr::read(&right)`), use a temporary
                            // to avoid C++ UB from reading an uninitialized variable.
                            let escaped_cpp = escape_cpp_keyword(&name_str);
                            let self_ref = cpp_name == escaped_cpp
                                && (expr_str.contains(&format!("&{}", escaped_cpp))
                                    || expr_str.contains(&format!("({})", escaped_cpp))
                                    || expr_str.contains(&format!("{},", escaped_cpp))
                                    || expr_str.contains(&format!(", {}", escaped_cpp)));
                            if self_ref {
                                // Emit: auto _tmp = init; auto name = _tmp;
                                let tmp_name = self.reserve_synthetic_cpp_name(&format!(
                                    "{}_self_ref_tmp",
                                    cpp_name
                                ));
                                self.writeln(&format!(
                                    "{} {} = {};",
                                    decl_type, tmp_name, expr_str
                                ));
                                // For reference bindings (`auto&`,
                                // `const auto&`, `T&`), binding the
                                // alias via `std::move(tmp)` produces a
                                // non-const-ref-to-xvalue which C++
                                // rejects. Re-bind directly.
                                let trimmed = decl_type.trim();
                                let is_ref_binding =
                                    trimmed.ends_with('&') || trimmed.ends_with("&&");
                                if is_ref_binding {
                                    self.writeln(&format!(
                                        "{} {} = {};",
                                        decl_type, cpp_name, tmp_name
                                    ));
                                } else {
                                    self.writeln(&format!(
                                        "{} {} = std::move({});",
                                        decl_type, cpp_name, tmp_name
                                    ));
                                }
                            } else {
                                self.writeln(&format!(
                                    "{} {} = {};",
                                    decl_type, cpp_name, expr_str
                                ));
                            }
                        }
                    }
                } else {
                    // `let x: T;` can be initialized later; emit mutable storage.
                    if let Some(inferred_ty) = uninitialized_inferred_ty.as_ref() {
                        if self.should_use_optional_delayed_init_storage(inferred_ty) {
                            self.mark_delayed_init_local(&name_str);
                            self.writeln(&format!("std::optional<{}> {};", type_str, cpp_name));
                        } else {
                            self.writeln(&format!("{} {};", type_str, cpp_name));
                        }
                    } else if type_str == "auto" {
                        // No type inferred → cannot emit `auto X;` (C++ rejects
                        // it: auto needs an initializer). Defer the declaration
                        // and let the first assignment site materialize as
                        // `auto cpp_name = expr;`. See
                        // `try_emit_pending_uninit_let_assign` for the merge.
                        if let Some(scope) = self.pending_uninit_let_locals.last_mut() {
                            scope.insert(name_str.clone(), cpp_name.clone());
                        }
                    } else {
                        self.writeln(&format!("{} {};", type_str, cpp_name));
                    }
                }
                if shadows_param {
                    if let Some(scope) = self.local_cpp_bindings.last_mut() {
                        scope.insert(name_str.clone(), cpp_name.clone());
                    }
                }
                // Restore the shadow name mapping after init expression emission.
                // This was temporarily removed to prevent self-referential init
                // (e.g., `let rhs = rhs.next()` → `rhs_shadow2 = rhs_shadow1.next()`).
                if shadows_outer {
                    if let Some(scope) = self.local_cpp_bindings.last_mut() {
                        scope.insert(name_str, cpp_name);
                    }
                }
                if track_in_progress_initializer {
                    self.pop_in_progress_local_initializer();
                }
            }
            syn::Pat::Tuple(tuple) => {
                // let (a, b) = expr; → auto [a, b] = expr;
                // Special case: if the init is an if-let expression with ?/return,
                // emit as a statement block instead of an expression (since the
                // IIFE approach can't propagate ? to the outer function).
                let tuple_initializer_names = if local.init.is_some() {
                    let mut names = HashSet::new();
                    for elem in &tuple.elems {
                        self.collect_closure_param_names_from_pat(elem, &mut names);
                    }
                    let names: Vec<String> = names.into_iter().collect();
                    for name in &names {
                        self.push_in_progress_local_initializer(name);
                    }
                    names
                } else {
                    Vec::new()
                };
                if let Some(init) = &local.init {
                    if let syn::Expr::If(if_expr) = &*init.expr {
                        if self.block_contains_early_return_or_try(&if_expr.then_branch)
                            || if_expr.else_branch.as_ref().is_some_and(|(_, else_expr)| {
                                self.expr_contains_early_return_or_try(else_expr)
                            })
                        {
                            if self
                                .emit_if_let_as_statement_block(tuple, if_expr)
                                .is_some()
                            {
                                for _ in &tuple_initializer_names {
                                    self.pop_in_progress_local_initializer();
                                }
                                return;
                            }
                        }
                    }
                    if let syn::Expr::Match(match_expr) = self.peel_paren_group_expr(&init.expr) {
                        if self.try_emit_tuple_local_match_initializer(tuple, match_expr) {
                            for _ in &tuple_initializer_names {
                                self.pop_in_progress_local_initializer();
                            }
                            return;
                        }
                    }
                }

                // Emit the init expression FIRST (using current scope names),
                // THEN allocate shadow names for the pattern elements.
                // This prevents self-referential initialization like
                // `auto [major, text_shadow1] = f(text_shadow1)`.
                let inferred_tuple_type = local
                    .init
                    .as_ref()
                    .and_then(|init| self.infer_local_binding_type_from_initializer(&init.expr))
                    .and_then(|ty| self.resolve_tuple_type_from_type(&ty));
                let expr_str = if let Some(init) = &local.init {
                    Some(self.emit_expr_to_string(&init.expr))
                } else {
                    None
                };
                for _ in &tuple_initializer_names {
                    self.pop_in_progress_local_initializer();
                }
                let rust_binding_names: Vec<Option<String>> = tuple
                    .elems
                    .iter()
                    .map(|p| match p {
                        syn::Pat::Ident(pi) if pi.ident != "_" => Some(pi.ident.to_string()),
                        _ => None,
                    })
                    .collect();
                let mut tuple_binding_names: Vec<String> = Vec::with_capacity(tuple.elems.len());
                let mut tuple_rebind_pointer_bindings: Vec<(usize, String, String, String)> =
                    Vec::new();
                for (idx, p) in tuple.elems.iter().enumerate() {
                    let raw = self.emit_pat_to_string(p);
                    if raw == "_" {
                        tuple_binding_names.push(
                            self.reserve_synthetic_cpp_name(&format!("_tuple_ignore{}", idx)),
                        );
                        continue;
                    }
                    let cpp_name = self.allocate_local_cpp_name(&raw);
                    let needs_rebind_pointer = matches!(p, syn::Pat::Ident(pi) if pi.mutability.is_some())
                        && self.reassigned_vars.contains(&raw)
                        && inferred_tuple_type
                            .as_ref()
                            .and_then(|tuple_ty| tuple_ty.elems.iter().nth(idx))
                            .is_some_and(|elem_ty| {
                                matches!(
                                    self.peel_paren_group_type(elem_ty),
                                    syn::Type::Reference(_)
                                )
                            });
                    if needs_rebind_pointer {
                        let tuple_slot_name =
                            self.reserve_synthetic_cpp_name(&format!("{}_ref", cpp_name));
                        tuple_binding_names.push(tuple_slot_name.clone());
                        tuple_rebind_pointer_bindings.push((
                            idx,
                            raw.clone(),
                            cpp_name,
                            tuple_slot_name,
                        ));
                    } else {
                        tuple_binding_names.push(cpp_name);
                    }
                }
                if let Some(expr_str) = expr_str {
                    if tuple_binding_names.is_empty() {
                        self.writeln(&format!("static_cast<void>({});", expr_str));
                        return;
                    }
                    // Check if this is a constructor call without template args that would
                    // cause CTAD failure. These have incomplete types and can't be used in
                    // structured bindings. Fall back to static_cast<void> to preserve side
                    // effects while avoiding invalid C++ code.
                    let is_incomplete_constructor = local.init.as_ref().is_some_and(|init| {
                        self.init_expr_is_incomplete_constructor_call(&init.expr)
                    });
                    if is_incomplete_constructor {
                        // Constructor call without template args → incomplete type.
                        // Can't use in structured binding, fall back to void cast.
                        self.writeln(&format!("static_cast<void>({});", expr_str));
                    } else {
                        let tuple_source_expr =
                            format!("rusty::detail::deref_if_pointer_like({})", expr_str);
                        let tuple_has_reference_elems =
                            inferred_tuple_type.as_ref().is_some_and(|tuple_ty| {
                                tuple_ty.elems.iter().any(|elem_ty| {
                                    matches!(
                                        self.peel_paren_group_type(elem_ty),
                                        syn::Type::Reference(_)
                                    )
                                })
                            });
                        if tuple_has_reference_elems {
                            // `auto [..]` drops tuple element references in C++, which breaks
                            // Rust tuple-destructuring semantics when an element is `&T`/`&mut T`.
                            // Materialize once, then bind each element preserving only the
                            // reference-typed slots.
                            let tuple_tmp = self.reserve_synthetic_cpp_name("_tuple_destructure");
                            self.writeln(&format!("auto {} = {};", tuple_tmp, tuple_source_expr));
                            for (idx, binding_name) in tuple_binding_names.iter().enumerate() {
                                let is_ref_elem = inferred_tuple_type
                                    .as_ref()
                                    .and_then(|tuple_ty| tuple_ty.elems.iter().nth(idx))
                                    .is_some_and(|elem_ty| {
                                        matches!(
                                            self.peel_paren_group_type(elem_ty),
                                            syn::Type::Reference(_)
                                        )
                                    });
                                let binding_auto = if is_ref_elem { "auto&&" } else { "auto" };
                                self.writeln(&format!(
                                    "{} {} = std::get<{}>(rusty::detail::deref_if_pointer({}));",
                                    binding_auto, binding_name, idx, tuple_tmp
                                ));
                            }
                        } else {
                            // `auto&&` so move-only tuple elements don't
                            // require a copy ctor (btree_port B4). See
                            // notes at the sibling structured-binding
                            // emit site lower in this file.
                            //
                            // Exception: when the init is a Rust-rvalue
                            // expression (function call, struct/tuple/array
                            // literal, etc.), the source pair temp is
                            // routed through `deref_if_pointer_like` which
                            // collapses to `std::forward<T>(value)` and
                            // returns an *xvalue*. `auto&&` binding to an
                            // xvalue does NOT lifetime-extend the original
                            // prvalue temp — it just records a reference.
                            // The temp dies at the end of this full
                            // expression, leaving `[tx, rx]` dangling.
                            // (See e.g. once_cell::stampede_once where
                            // `let (tx, rx) = channel()` produced a
                            // dangling sender pair, and take_mut's
                            // scope_based_take where the Hole inside a
                            // returned tuple died before `.fill()` ran.)
                            //
                            // Use `auto` for rvalue inits: the invisible
                            // binding variable becomes its own moved-out
                            // copy of the pair, owning it for the
                            // enclosing scope. Move-only element types
                            // still flow through because the pair/tuple
                            // is move-constructed (not copy-constructed)
                            // from the rvalue.
                            let init_is_rvalue = local
                                .init
                                .as_ref()
                                .is_some_and(|init| self.is_rvalue_expr(&init.expr));
                            let binding_kw = if init_is_rvalue { "auto" } else { "auto&&" };
                            self.writeln(&format!(
                                "{} [{}] = {};",
                                binding_kw,
                                tuple_binding_names.join(", "),
                                tuple_source_expr
                            ));
                        }
                        for (idx, rust_name, cpp_name, tuple_slot_name) in
                            &tuple_rebind_pointer_bindings
                        {
                            let ptr_ty = inferred_tuple_type
                                .as_ref()
                                .and_then(|tuple_ty| tuple_ty.elems.iter().nth(*idx))
                                .and_then(|elem_ty| {
                                    let elem_ty = self.peel_paren_group_type(elem_ty);
                                    if let syn::Type::Reference(reference) = elem_ty {
                                        let inner = self.map_type(&reference.elem);
                                        if reference.mutability.is_some() {
                                            Some(format!("{}*", inner))
                                        } else {
                                            Some(format!("const {}*", inner))
                                        }
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or_else(|| "auto*".to_string());
                            self.writeln(&format!(
                                "{} {} = &{};",
                                ptr_ty, cpp_name, tuple_slot_name
                            ));
                            self.record_local_reference_binding(rust_name, true);
                            self.record_rebind_reference_pointer_binding(rust_name);
                        }
                        if let Some(tuple_ty) = inferred_tuple_type.as_ref() {
                            for (raw_name, elem_ty) in
                                rust_binding_names.iter().zip(tuple_ty.elems.iter())
                            {
                                if let Some(name) = raw_name {
                                    self.update_local_binding_type(name.clone(), elem_ty.clone());
                                }
                            }
                        }
                    }
                }
            }
            syn::Pat::Slice(slice) => {
                let has_rest = slice
                    .elems
                    .iter()
                    .any(|elem| self.pat_is_slice_rest_like(elem));
                if has_rest {
                    if let Some(init) = &local.init {
                        if !self.emit_complex_local_pattern_binding_from_init(
                            &local.pat, &init.expr, None,
                        ) {
                            self.writeln("// TODO: complex slice pattern binding");
                        }
                    } else {
                        self.writeln("// TODO: complex slice pattern binding");
                    }
                    return;
                }

                let expr_str = if let Some(init) = &local.init {
                    Some(self.emit_expr_to_string(&init.expr))
                } else {
                    None
                };
                let names: Vec<String> = slice
                    .elems
                    .iter()
                    .enumerate()
                    .map(|(idx, p)| {
                        let raw = self.emit_pat_to_string(p);
                        if raw == "_" {
                            self.reserve_synthetic_cpp_name(&format!("_slice_ignore{}", idx))
                        } else {
                            self.allocate_local_cpp_name(&raw)
                        }
                    })
                    .collect();
                if names.iter().any(|name| name.contains("/* TODO:")) {
                    self.writeln("// TODO: complex slice pattern binding");
                    return;
                }
                if let Some(expr_str) = expr_str {
                    if names.is_empty() {
                        self.writeln(&format!("static_cast<void>({});", expr_str));
                        return;
                    }
                    let is_incomplete_constructor = local.init.as_ref().is_some_and(|init| {
                        self.init_expr_is_incomplete_constructor_call(&init.expr)
                    });
                    if is_incomplete_constructor {
                        self.writeln(&format!("static_cast<void>({});", expr_str));
                    } else {
                        // Use `auto&&` for structured bindings so move-only
                        // payloads bind without requiring a copy. `let (a, b)
                        // = expr;` in Rust moves the parts of the tuple; the
                        // C++ equivalent `auto [a, b] = expr;` would COPY
                        // each element, which fails to compile when an
                        // element is move-only (e.g.,
                        // `std::pair<long, rusty::Function<void()>>`).
                        // Forwarding-ref binding keeps the lifetime extended
                        // (rvalue → lvalue-ref-to-temp via reference
                        // collapse) while exposing the elements as bindable
                        // names. Matches btree_port B4 — see
                        // tests/btree_port_iter_remove_movonly_test.cpp.
                        //
                        // Exception (see sibling tuple-destructure site
                        // above): for Rust-rvalue inits routed through
                        // `deref_if_pointer_like`, `auto&&` does NOT
                        // lifetime-extend the underlying prvalue temp —
                        // the helper returns an xvalue and the temp dies
                        // at end of the full expression, leaving the
                        // bindings dangling. Emit `auto` for rvalues so
                        // the invisible binding variable owns its own
                        // moved-out value.
                        let init_is_rvalue = local
                            .init
                            .as_ref()
                            .is_some_and(|init| self.is_rvalue_expr(&init.expr));
                        let binding_kw = if init_is_rvalue { "auto" } else { "auto&&" };
                        self.writeln(&format!(
                            "{} [{}] = {};",
                            binding_kw,
                            names.join(", "),
                            expr_str
                        ));
                    }
                }
            }
            syn::Pat::Wild(_) => {
                // `let _ = expr;` keeps side effects while discarding the value.
                if let Some(init) = &local.init {
                    let expr_str = self.emit_expr_maybe_move(&init.expr);
                    self.writeln(&format!("static_cast<void>({});", expr_str));
                }
            }
            syn::Pat::Type(pat_type) => {
                // let x: Type = expr;
                // The type annotation is on the pattern, inner pat has the name
                if let syn::Pat::Ident(pi) = pat_type.pat.as_ref() {
                    let name = &pi.ident;
                    let name_str = name.to_string();
                    let previous_cpp_name_in_current_scope = self
                        .local_cpp_bindings
                        .last()
                        .and_then(|scope| scope.get(&name_str).cloned());
                    let cpp_name = self.allocate_local_cpp_name(&name_str);
                    // Match Pat::Ident shadow handling so typed shadow initializers
                    // resolve RHS names to the previous binding (`let x: T = x;`).
                    let has_outer_same_rust_binding = self
                        .local_cpp_bindings
                        .iter()
                        .rev()
                        .skip(1)
                        .any(|scope| scope.contains_key(&name_str));
                    let shadows_outer =
                        cpp_name != escape_cpp_keyword(&name_str) || has_outer_same_rust_binding;
                    if shadows_outer {
                        if let Some(scope) = self.local_cpp_bindings.last_mut() {
                            scope.remove(&name_str);
                            if let Some(previous_cpp_name) =
                                previous_cpp_name_in_current_scope.clone()
                            {
                                scope.insert(name_str.clone(), previous_cpp_name);
                            }
                        }
                    }
                    let shadows_param = self
                        .param_bindings
                        .last()
                        .is_some_and(|params| params.contains_key(&name_str));
                    let track_in_progress_initializer = local.init.is_some();
                    if track_in_progress_initializer {
                        self.push_in_progress_local_initializer(&name_str);
                    }
                    let is_mut = pi.mutability.is_some();
                    if local
                        .init
                        .as_ref()
                        .is_some_and(|init| self.expr_is_manually_drop_new_call(&init.expr))
                    {
                        self.mark_local_manually_drop_binding(&name_str);
                    }
                    let resolved_ty =
                        if let Some(hint) = self.lookup_local_placeholder_type_hint(&name_str) {
                            self.substitute_owner_infer_with_hint(
                                &pat_type.ty,
                                &["ArrayVec", "Cell", "Vec", "HashMap", "SmallVec"],
                                hint,
                            )
                        } else {
                            (*pat_type.ty).clone()
                        };
                    let has_unresolved_infer = self.type_contains_infer(&resolved_ty);
                    if !has_unresolved_infer {
                        self.update_local_binding_type(name_str.clone(), resolved_ty.clone());
                    }
                    let mapped_ty = self.map_type(&resolved_ty);
                    // A mapped type containing `<auto>` as a template argument
                    // (e.g. `std::span<auto>*` from a raw-pointer-to-slice
                    // annotation like `*const [_]`) is not valid C++. Fall
                    // back to plain `auto` so the initializer drives type
                    // deduction. Bare `auto` / `const auto*` outside angle
                    // brackets is fine and stays as-is.
                    let mapped_has_invalid_auto_targ = type_string_contains_auto_template_arg(&mapped_ty);
                    let ty = if (has_unresolved_infer || mapped_has_invalid_auto_targ)
                        && local.init.is_some()
                    {
                        "auto".to_string()
                    } else {
                        mapped_ty
                    };
                    let resolved_mut_reference_binding = Self::is_mut_reference_type(&resolved_ty);
                    let is_consumed = self
                        .consuming_method_receiver_vars
                        .contains(&name.to_string());
                    let qualifier = if is_mut
                        || is_consumed
                        || self.mutable_pointer_aliased_vars.contains(&name_str)
                        || resolved_mut_reference_binding
                        || ty.trim_start().starts_with("const ")
                    {
                        ""
                    } else {
                        "const "
                    };
                    let decl_type = if qualifier == "const "
                        && Self::is_mut_raw_pointer_type(&resolved_ty)
                        && ty.contains('*')
                    {
                        format!("{} const", ty)
                    } else {
                        format!("{}{}", qualifier, ty)
                    };
                    self.record_local_const_binding(
                        &name_str,
                        qualifier == "const " && local.init.is_some(),
                    );
                    let local_decl_is_reference = Self::is_mut_reference_type(&resolved_ty)
                        || local
                            .init
                            .as_ref()
                            .is_some_and(|init| self.is_ref_init(&init.expr))
                        || matches!(
                            self.peel_paren_group_type(&resolved_ty),
                            syn::Type::Reference(_)
                        );
                    let needs_rebind_pointer_local = is_mut
                        && self.reassigned_vars.contains(&name_str)
                        && local_decl_is_reference;
                    let rebind_pointer_decl_type = if needs_rebind_pointer_local {
                        self.map_reference_type_to_pointer_cpp_type(&resolved_ty)
                            .or(Some("auto*".to_string()))
                    } else {
                        None
                    };
                    let decl_type = if let Some(ptr_decl) = rebind_pointer_decl_type.as_ref() {
                        ptr_decl.clone()
                    } else {
                        decl_type
                    };
                    self.record_local_reference_binding(&name_str, local_decl_is_reference);
                    if needs_rebind_pointer_local {
                        self.record_rebind_reference_pointer_binding(&name_str);
                    }
                    if shadows_param {
                        self.local_cpp_bindings
                            .last_mut()
                            .and_then(|scope| scope.remove(&name_str));
                    }
                    if let Some(init) = &local.init {
                        if self.should_materialize_slice_range_pointer_storage(
                            &resolved_ty,
                            &init.expr,
                        ) {
                            let syn::Expr::Reference(ref_expr) =
                                self.peel_paren_group_expr(&init.expr)
                            else {
                                unreachable!(
                                    "slice range pointer materialization requires reference init"
                                );
                            };
                            let backing_name = format!("{}_backing", cpp_name);
                            let backing_decl = if Self::is_mut_raw_pointer_type(&resolved_ty) {
                                "auto"
                            } else {
                                "const auto"
                            };
                            let slice_expr = self.emit_expr_to_string_with_expected(
                                &ref_expr.expr,
                                Some(&resolved_ty),
                            );
                            self.writeln(&format!(
                                "{} {} = {};",
                                backing_decl, backing_name, slice_expr
                            ));
                            self.writeln(&format!(
                                "{} {} = &{};",
                                decl_type, cpp_name, backing_name
                            ));
                        } else {
                            let expr_str = if let Some(callable_item_expr) =
                                self.emit_callable_path_item_expr(&init.expr)
                            {
                                callable_item_expr
                            } else {
                                self.emit_expr_to_string_with_expected(
                                    &init.expr,
                                    Some(&resolved_ty),
                                )
                            };
                            let expr_str = if needs_rebind_pointer_local {
                                let init_ref_like_by_binding =
                                    self.type_is_reference_like(&resolved_ty);
                                if expr_str.starts_with('&') {
                                    expr_str
                                } else if self.expr_is_reference_yielding(&init.expr)
                                    || init_ref_like_by_binding
                                {
                                    self.emit_try_expr_reference_pointer(&init.expr)
                                        .unwrap_or_else(|| format!("&({})", expr_str))
                                } else {
                                    expr_str
                                }
                            } else {
                                expr_str
                            };
                            self.writeln(&format!("{} {} = {};", decl_type, cpp_name, expr_str));
                        }
                    } else {
                        // `let x: T;` can be initialized later; emit mutable storage.
                        if self.should_use_optional_delayed_init_storage(&pat_type.ty) {
                            self.mark_delayed_init_local(&name.to_string());
                            self.writeln(&format!("std::optional<{}> {};", ty, cpp_name));
                        } else {
                            self.writeln(&format!("{} {};", ty, cpp_name));
                        }
                    }
                    if shadows_param {
                        if let Some(scope) = self.local_cpp_bindings.last_mut() {
                            scope.insert(name_str, cpp_name.clone());
                        }
                    }
                    if shadows_outer {
                        if let Some(scope) = self.local_cpp_bindings.last_mut() {
                            scope.insert(name.to_string(), cpp_name);
                        }
                    }
                    if track_in_progress_initializer {
                        self.pop_in_progress_local_initializer();
                    }
                } else if matches!(pat_type.pat.as_ref(), syn::Pat::Wild(_)) {
                    if let Some(init) = &local.init {
                        let expr_str =
                            self.emit_expr_to_string_with_expected(&init.expr, Some(&pat_type.ty));
                        self.writeln(&format!("static_cast<void>({});", expr_str));
                    }
                } else {
                    if let Some(init) = &local.init {
                        if !self.emit_complex_local_pattern_binding_from_init(
                            &pat_type.pat,
                            &init.expr,
                            Some(&pat_type.ty),
                        ) {
                            self.writeln("// TODO: complex typed pattern binding");
                        }
                    } else {
                        self.writeln("// TODO: complex typed pattern binding");
                    }
                }
            }
            _ => {
                if let Some(init) = &local.init {
                    if !self
                        .emit_complex_local_pattern_binding_from_init(&local.pat, &init.expr, None)
                    {
                        self.writeln("// TODO: complex pattern binding");
                    }
                } else {
                    self.writeln("// TODO: complex pattern binding");
                }
            }
        }
    }

    pub(super) fn emit_if_expr_to_string(
        &self,
        if_expr: &syn::ExprIf,
        expected_ty: Option<&syn::Type>,
    ) -> String {
        if let syn::Expr::Let(let_expr) = &*if_expr.cond {
            if let Some(lowered) = self.emit_if_let_expr_to_string(
                let_expr,
                &if_expr.then_branch,
                &if_expr.else_branch,
                expected_ty,
            ) {
                return lowered;
            }
        }

        // If used as an expression (e.g., `let x = if c { 1 } else { 2 };`)
        // -> C++ ternary when branches are simple single-expression values.
        let Some((_, else_branch)) = &if_expr.else_branch else {
            if !self.block_contains_early_return_or_try(&if_expr.then_branch) {
                let cond = self.emit_expr_to_string(&if_expr.cond);
                let then_body = if_expr
                    .then_branch
                    .stmts
                    .iter()
                    .map(|stmt| match stmt {
                        syn::Stmt::Expr(expr, None) => {
                            let expr_str = self.emit_expr_to_string(expr);
                            format!("{};", expr_str)
                        }
                        _ => self.emit_stmt_to_string(stmt),
                    })
                    .filter(|stmt| !stmt.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
                // Rust permits `if cond { ... }` expression form only for unit.
                // Lower to an IIFE so expression-position unit initializers compile.
                return format!(
                    "[&]() {{ if ({}) {{ {} }} return std::tuple<>(); }}()",
                    cond, then_body
                );
            }
            return "/* TODO: if-expression */".to_string();
        };
        let cond = self.emit_expr_to_string(&if_expr.cond);
        let then_single = self.extract_single_expr_from_block(&if_expr.then_branch);
        let else_single = self.extract_single_value_expr(else_branch);
        // When either branch is a multi-statement block, try IIFE only if
        // the branches don't contain early returns or `?` operators (which
        // would escape to the lambda instead of the enclosing function).
        if then_single.is_none() || else_single.is_none() {
            if !self.block_contains_early_return_or_try(&if_expr.then_branch)
                && !self.expr_contains_early_return_or_try(else_branch)
            {
                return self.emit_if_expr_as_iife(if_expr, expected_ty);
            }
            return "/* TODO: if-expression */".to_string();
        }
        let then_expr = then_single.unwrap();
        let else_expr = else_single.unwrap();

        let inferred_expected_ty = if expected_ty.is_none() {
            self.infer_constructor_expected_type_from_pair(then_expr, else_expr)
                .or_else(|| {
                    // §13.14 branch-merge: if one branch is a return-only-generic
                    // call (needs an expected type to be solved), derive it from
                    // the resolvable sibling branch (both share one Rust type).
                    self.infer_if_branch_expected_from_return_only_sibling(then_expr, else_expr)
                })
        } else {
            None
        };
        let expected_ty_for_branches = expected_ty.or(inferred_expected_ty.as_ref());
        // ctor_template_args feeds the inner-arm emit. When an
        // annotation is present (`let e: Either<i32, i32> = …`) the
        // expected_ty already carries concrete template args; prefer
        // those for precision. Otherwise fall back to the decltype-
        // based form from `infer_variant_ctor_template_args_from_if`.
        let inferred_ctor_args = expected_ty_for_branches
            .and_then(|ty| self.expected_either_concrete_template_args(ty))
            .or_else(|| self.infer_variant_ctor_template_args_from_if(if_expr));
        let inferred_expected_cpp_ty = inferred_ctor_args
            .as_ref()
            .map(|args| format!("Either<{}, {}>", args[0], args[1]));
        // Outer-wrap candidates, in priority order:
        //   1. `map_type(expected_ty)` IF it produces an args-bearing
        //      form (`Either<int32_t, int32_t>`). This is the
        //      annotated case — `let e: Either<i32, i32> = …` — and
        //      gives the most precise C++ type.
        //   2. `inferred_expected_cpp_ty` from
        //      `infer_variant_ctor_template_args_from_if` — the
        //      decltype-based form (`Either<decltype((a)),
        //      decltype((b))>`). Used when no annotation is present,
        //      the engine inferred arms have a common enum, and we
        //      need any non-bare wrap.
        //   3. `map_type(expected_ty)` for the bare-name case
        //      (`Either`) — last resort; the outer wrap will still
        //      fail CTAD on this but at least we tried both inputs.
        //
        // The §13.3 reason `inferred_expected_cpp_ty` was promoted
        // over `map_type` earlier was that `map_type` for an
        // unannotated user-defined data enum returns the bare name.
        // With the annotation, `map_type` IS args-bearing, so we
        // prefer it for the precision win.
        let mapped_expected_cpp_ty = expected_ty_for_branches.and_then(|ty| {
            if self.expected_data_enum_name(ty).is_some() {
                Some(self.map_type(ty))
            } else {
                None
            }
        });
        let mapped_has_template_args = mapped_expected_cpp_ty
            .as_deref()
            .is_some_and(|s| s.contains('<'));
        let expected_cpp_ty = if mapped_has_template_args {
            mapped_expected_cpp_ty
        } else {
            inferred_expected_cpp_ty.clone().or(mapped_expected_cpp_ty)
        };

        let then_emitted = self.emit_if_ternary_branch_expr(
            then_expr,
            expected_ty_for_branches,
            inferred_ctor_args.as_deref(),
        );
        let else_emitted = self.emit_if_ternary_branch_expr(
            else_expr,
            expected_ty_for_branches,
            inferred_ctor_args.as_deref(),
        );
        let then_val = self.maybe_wrap_variant_constructor_with_expected_cpp_type(
            then_expr,
            then_emitted,
            expected_cpp_ty.as_deref(),
        );
        let else_val = self.maybe_wrap_variant_constructor_with_expected_cpp_type(
            else_expr,
            else_emitted,
            expected_cpp_ty.as_deref(),
        );
        format!("({} ? {} : {})", cond, then_val, else_val)
    }

    /// Lower `let x = if let Some(y) = expr { ...?...; val } else { default };`
    /// as a statement block with pre-declared result variable.
    pub(super) fn emit_single_if_let_as_statement_block(
        &mut self,
        cpp_name: &str,
        decl_type: &str,
        if_expr: &syn::ExprIf,
    ) -> Option<()> {
        let Some((_, else_branch)) = &if_expr.else_branch else {
            return None;
        };

        // Try to extract else-branch value for initialization.
        // If else-branch is multi-statement with early return (no value),
        // use default initialization and emit else as statements.
        let else_result = match else_branch.as_ref() {
            syn::Expr::Block(b) => self
                .extract_single_expr_from_block(&b.block)
                .map(|e| self.emit_expr_to_string(e)),
            other => Some(self.emit_expr_to_string(other)),
        };
        let else_is_option_none_value = match else_branch.as_ref() {
            syn::Expr::Block(b) => self
                .extract_single_expr_from_block(&b.block)
                .is_some_and(|expr| self.expr_is_option_none_path(expr)),
            other => self.expr_is_option_none_path(other),
        };
        let else_has_early_return = match else_branch.as_ref() {
            syn::Expr::Block(b) => self.block_contains_early_return_or_try(&b.block),
            other => self.expr_contains_early_return_or_try(other),
        };
        let else_requires_lazy_eval = self.expr_contains_early_return_or_try(else_branch);

        let init_decl_type = decl_type
            .strip_prefix("const ")
            .unwrap_or(decl_type)
            .to_string();
        let resolved_init_decl_type = if (init_decl_type == "auto"
            || type_string_has_auto_placeholder(&init_decl_type))
            && else_is_option_none_value
        {
            let inferred_option_ty = self
                .extract_tail_expr_from_block(&if_expr.then_branch)
                .and_then(|tail| self.extract_option_some_call_arg(tail))
                .and_then(|payload_expr| {
                    self.infer_local_binding_type_from_initializer(payload_expr)
                        .or_else(|| self.infer_simple_expr_type(payload_expr))
                        .or_else(|| self.infer_try_payload_type_from_expr(payload_expr))
                        .or_else(|| {
                            let payload_expr = self.peel_paren_group_expr(payload_expr);
                            let syn::Expr::Path(path_expr) = payload_expr else {
                                return None;
                            };
                            if path_expr.path.segments.len() != 1 {
                                return None;
                            }
                            let payload_name = path_expr.path.segments[0].ident.to_string();
                            let payload_local = self.find_local_binding_in_block_by_name(
                                &if_expr.then_branch,
                                &payload_name,
                            )?;
                            if let Some(explicit_ty) = get_local_type(payload_local)
                                && !type_has_generic_placeholder(explicit_ty)
                            {
                                return Some(explicit_ty.clone());
                            }
                            let inferred_from_init = payload_local.init.as_ref().and_then(|init| {
                                let init_expr = init.expr.as_ref().clone();
                                let mut inferred: Option<syn::Type> = None;
                                self.with_pre_scan_known_local_scope(
                                    &if_expr.then_branch.stmts,
                                    |this| {
                                        inferred = this
                                            .infer_local_binding_type_from_initializer(&init_expr)
                                            .or_else(|| this.infer_simple_expr_type(&init_expr))
                                            .or_else(|| {
                                                this.infer_try_payload_type_from_expr(&init_expr)
                                            });
                                    },
                                );
                                inferred
                            });
                            inferred_from_init
                        })
                })
                .map(|inner_ty| {
                    let option_ty: syn::Type = parse_quote!(Option<#inner_ty>);
                    self.map_type(&option_ty)
                })
                .or_else(|| self.infer_option_cpp_type_from_if_some_payload_decltype(if_expr));
            inferred_option_ty
                .filter(|mapped| Self::is_concrete_cpp_type_for_iflet_init(mapped))
                .unwrap_or_else(|| init_decl_type.clone())
        } else {
            init_decl_type.clone()
        };
        let then_tail_needs_recursive_if_assign = if_expr
            .then_branch
            .stmts
            .last()
            .and_then(|stmt| {
                if let syn::Stmt::Expr(syn::Expr::If(inner_if), None) = stmt {
                    Some(
                        self.block_contains_early_return_or_try(&inner_if.then_branch)
                            || inner_if
                                .else_branch
                                .as_ref()
                                .is_some_and(|(_, e)| self.expr_contains_early_return_or_try(e)),
                    )
                } else {
                    None
                }
            })
            .unwrap_or(false);
        let use_lazy_else_optional = else_result.is_some()
            && else_requires_lazy_eval
            && !init_decl_type.contains('&')
            && !then_tail_needs_recursive_if_assign
            && !matches!(else_branch.as_ref(), syn::Expr::If(_));

        let mut lazy_storage_var: Option<String> = None;
        if use_lazy_else_optional {
            let storage_var = format!("_iflet_value{}", self.iflet_result_counter);
            self.iflet_result_counter += 1;
            let storage_ty = if init_decl_type == "auto"
                || type_string_has_auto_placeholder(&init_decl_type)
            {
                let inferred_lazy_storage_ty = self
                    .extract_tail_expr_from_block(&if_expr.then_branch)
                    .and_then(|tail| {
                        self.infer_local_binding_type_from_initializer(tail)
                            .or_else(|| self.infer_simple_expr_type(tail))
                    })
                    .or_else(|| {
                        self.infer_local_binding_type_from_initializer(else_branch.as_ref())
                            .or_else(|| self.infer_simple_expr_type(else_branch.as_ref()))
                    })
                    .map(|ty| self.map_type_with_explicit_owner_generic_recovery(&ty))
                    .filter(|mapped| {
                        mapped != "auto"
                            && !mapped.contains("/* TODO")
                            && !type_string_has_auto_placeholder(mapped)
                    });
                if let Some(mapped) = inferred_lazy_storage_ty {
                    mapped
                } else {
                    let else_value = else_result.as_ref()?;
                    if self.expr_text_contains_try_macro_invocation(else_value) {
                        if let Some(try_operand) = self.root_try_operand_expr(else_branch.as_ref())
                        {
                            let try_operand_value = self.emit_expr_to_string(try_operand);
                            format!(
                                "std::remove_cvref_t<decltype((({}).unwrap()))>",
                                try_operand_value
                            )
                        } else {
                            format!("std::remove_cvref_t<decltype(({}))>", else_value)
                        }
                    } else {
                        format!("std::remove_cvref_t<decltype(({}))>", else_value)
                    }
                }
            } else {
                init_decl_type.clone()
            };
            self.writeln(&format!("std::optional<{}> {};", storage_ty, storage_var));
            lazy_storage_var = Some(storage_var);
        }

        if use_lazy_else_optional {
            // Keep else-branch evaluation inside the generated `else` block so
            // `if let` statement lowering preserves branch evaluation order.
        } else if let Some(else_value) = &else_result
            && !else_has_early_return
        {
            self.writeln(&format!(
                "{} {} = {};",
                resolved_init_decl_type, cpp_name, else_value
            ));
        } else if else_has_early_return {
            // Else always returns early — use default-init (never reached via else path)
            if resolved_init_decl_type == "auto"
                || type_string_has_auto_placeholder(&resolved_init_decl_type)
            {
                let then_tail = self.extract_tail_expr_from_block(&if_expr.then_branch)?;
                if let Some(variant_ctx) = self.infer_variant_type_context_from_expr(then_tail) {
                    let inferred_ty = if variant_ctx.template_args.is_empty() {
                        variant_ctx.enum_name
                    } else {
                        format!(
                            "{}<{}>",
                            variant_ctx.enum_name,
                            variant_ctx.template_args.join(", ")
                        )
                    };
                    self.writeln(&format!("{} {} {{}};", inferred_ty, cpp_name));
                } else if let Some(enum_owner_ty) =
                    self.infer_data_enum_owner_type_from_variant_ctor_expr(then_tail)
                {
                    self.writeln(&format!("{} {} {{}};", enum_owner_ty, cpp_name));
                } else if let Some(payload_decl) =
                    self.if_let_some_payload_result_decl_type(if_expr, then_tail)
                {
                    // `let a = if let Some(v) = X { v } else { diverge }`: the result
                    // var is declared OUTSIDE the if-let block, so `decltype(v)`
                    // (v is block-local) is undefined. Type it from the scrutinee's
                    // payload instead — `decltype(X.unwrap())` — which is in scope.
                    self.writeln(&format!("{} {} {{}};", payload_decl, cpp_name));
                } else {
                    let then_tail_str = self.emit_expr_to_string(then_tail);
                    self.writeln(&format!("decltype({}) {} {{}};", then_tail_str, cpp_name));
                }
            } else {
                self.writeln(&format!("{} {} {{}};", resolved_init_decl_type, cpp_name));
            }
        } else {
            return None;
        }

        self.writeln("{");
        self.indent += 1;

        if let syn::Expr::Let(let_expr) = &*if_expr.cond {
            let scrutinee = self.emit_expr_to_string(&let_expr.expr);
            let (cond_expr, unwrap) =
                self.if_let_statement_condition_parts(let_expr, "_iflet_scrutinee");
            self.writeln(&format!("auto&& _iflet_scrutinee = {};", scrutinee));
            self.writeln(&format!("if ({}) {{", cond_expr));
            self.indent += 1;
            self.push_transient_statement_scope();
            if let Some(used) = self.local_cpp_names_used.last_mut() {
                used.insert(cpp_name.to_string());
            }
            self.emit_if_let_statement_scope_bindings(
                &let_expr.pat,
                "_iflet_scrutinee",
                unwrap,
                scrutinee.ends_with(".as_mut()"),
            );
        } else {
            let cond = self.emit_expr_to_string(&if_expr.cond);
            self.writeln(&format!("if ({}) {{", cond));
            self.indent += 1;
            self.push_transient_statement_scope();
            if let Some(used) = self.local_cpp_names_used.last_mut() {
                used.insert(cpp_name.to_string());
            }
        }

        let stmts = &if_expr.then_branch.stmts;
        for (i, stmt) in stmts.iter().enumerate() {
            let is_last = i == stmts.len() - 1;
            if is_last {
                if let syn::Stmt::Expr(expr, None) = stmt {
                    // Recursive: if tail is another if-expression with ?, lower it
                    if let syn::Expr::If(inner_if) = expr {
                        if self.block_contains_early_return_or_try(&inner_if.then_branch)
                            || inner_if
                                .else_branch
                                .as_ref()
                                .is_some_and(|(_, e)| self.expr_contains_early_return_or_try(e))
                        {
                            self.emit_if_assign_as_statement_block(cpp_name, inner_if, None);
                            continue;
                        }
                    }
                    let val = self.emit_expr_to_string(expr);
                    if let Some(storage_var) = lazy_storage_var.as_ref() {
                        self.writeln(&format!("{}.emplace({});", storage_var, val));
                    } else {
                        self.writeln(&format!("{} = {};", cpp_name, val));
                    }
                    continue;
                }
            }
            self.emit_stmt(stmt, false);
        }

        self.pop_transient_statement_scope();
        self.indent -= 1;
        if let Some(storage_var) = lazy_storage_var.as_ref() {
            let else_value = else_result.as_ref()?;
            self.writeln(&format!(
                "}} else {{ {}.emplace({}); }}",
                storage_var, else_value
            ));
        } else if else_has_early_return {
            // Emit else-branch statements if it has early returns
            self.writeln("} else {");
            self.indent += 1;
            self.push_transient_statement_scope();
            if let Some((_, else_expr)) = &if_expr.else_branch {
                if let syn::Expr::Block(b) = else_expr.as_ref() {
                    for (idx, stmt) in b.block.stmts.iter().enumerate() {
                        let is_last = idx + 1 == b.block.stmts.len();
                        if is_last && let syn::Stmt::Expr(expr, None) = stmt {
                            if let syn::Expr::If(inner_if) = expr {
                                if self.block_contains_early_return_or_try(&inner_if.then_branch)
                                    || inner_if.else_branch.as_ref().is_some_and(|(_, e)| {
                                        self.expr_contains_early_return_or_try(e)
                                    })
                                {
                                    self.emit_if_assign_as_statement_block(
                                        cpp_name, inner_if, None,
                                    );
                                    continue;
                                }
                            }
                            let else_val = self.emit_expr_to_string(expr);
                            self.writeln(&format!("{} = {};", cpp_name, else_val));
                            continue;
                        }
                        self.emit_stmt(stmt, false);
                    }
                } else if let syn::Expr::If(inner_if) = else_expr.as_ref() {
                    self.emit_if_assign_as_statement_block(cpp_name, inner_if, None);
                } else {
                    let else_val = self.emit_expr_to_string(else_expr);
                    self.writeln(&format!("{} = {};", cpp_name, else_val));
                }
            }
            self.pop_transient_statement_scope();
            self.indent -= 1;
            self.writeln("}");
        } else {
            self.writeln("}");
        }
        self.indent -= 1;
        self.writeln("}");
        if let Some(storage_var) = lazy_storage_var {
            self.writeln(&format!(
                "{} {} = std::move({}).value();",
                decl_type, cpp_name, storage_var
            ));
        }
        Some(())
    }

    /// Lower `result_var = if cond { ...?... } else { val };` as a statement block
    /// that assigns to `result_var` in each branch. Handles nested if-expressions
    /// with `?` recursively.
    pub(super) fn emit_if_assign_as_statement_block(
        &mut self,
        result_var: &str,
        if_expr: &syn::ExprIf,
        expected_ty: Option<&syn::Type>,
    ) {
        if let syn::Expr::Let(let_expr) = &*if_expr.cond {
            let scrutinee = self.emit_expr_to_string(&let_expr.expr);
            let (cond_expr, unwrap) = self.if_let_statement_condition_parts(let_expr, "_iflet_s");
            self.writeln(&format!("{{ auto&& _iflet_s = {};", scrutinee));
            self.writeln(&format!("if ({}) {{", cond_expr));
            self.indent += 1;
            self.push_transient_statement_scope();
            if let Some(used) = self.local_cpp_names_used.last_mut() {
                used.insert(result_var.to_string());
            }
            self.emit_if_let_statement_scope_bindings(
                &let_expr.pat,
                "_iflet_s",
                unwrap,
                scrutinee.ends_with(".as_mut()"),
            );
        } else {
            let cond = self.emit_expr_to_string(&if_expr.cond);
            self.writeln(&format!("{{ if ({}) {{", cond));
            self.indent += 1;
            self.push_transient_statement_scope();
        }

        // Emit then-branch; tail assigns to result_var
        let stmts = &if_expr.then_branch.stmts;
        for (i, stmt) in stmts.iter().enumerate() {
            let is_last = i == stmts.len() - 1;
            if is_last {
                if let syn::Stmt::Expr(expr, None) = stmt {
                    if let syn::Expr::If(inner_if) = expr {
                        if self.block_contains_early_return_or_try(&inner_if.then_branch)
                            || inner_if
                                .else_branch
                                .as_ref()
                                .is_some_and(|(_, e)| self.expr_contains_early_return_or_try(e))
                        {
                            self.emit_if_assign_as_statement_block(
                                result_var,
                                inner_if,
                                expected_ty,
                            );
                            self.pop_transient_statement_scope();
                            self.indent -= 1;
                            self.writeln("}}");
                            return;
                        }
                    }
                    let val = self.emit_expr_to_string_with_expected(expr, expected_ty);
                    self.writeln(&format!("{} = {};", result_var, val));
                    continue;
                }
            }
            self.emit_stmt(stmt, false);
        }

        self.pop_transient_statement_scope();
        self.indent -= 1;
        if let Some((_, else_expr)) = &if_expr.else_branch {
            match else_expr.as_ref() {
                syn::Expr::Block(b) => {
                    if let Some(single) = self.extract_single_expr_from_block(&b.block) {
                        let val = self.emit_expr_to_string_with_expected(single, expected_ty);
                        self.writeln(&format!("}} else {{ {} = {}; }}}}", result_var, val));
                    } else {
                        self.writeln("} else {");
                        self.indent += 1;
                        self.push_transient_statement_scope();
                        let stmts = &b.block.stmts;
                        for (i, stmt) in stmts.iter().enumerate() {
                            let is_last = i == stmts.len() - 1;
                            if is_last {
                                if let syn::Stmt::Expr(expr, None) = stmt {
                                    if let syn::Expr::If(inner_if) = expr {
                                        if self.block_contains_early_return_or_try(
                                            &inner_if.then_branch,
                                        ) || inner_if.else_branch.as_ref().is_some_and(
                                            |(_, e)| self.expr_contains_early_return_or_try(e),
                                        ) {
                                            self.emit_if_assign_as_statement_block(
                                                result_var,
                                                inner_if,
                                                expected_ty,
                                            );
                                            continue;
                                        }
                                    }
                                    let val =
                                        self.emit_expr_to_string_with_expected(expr, expected_ty);
                                    self.writeln(&format!("{} = {};", result_var, val));
                                    continue;
                                }
                            }
                            self.emit_stmt(stmt, false);
                        }
                        self.pop_transient_statement_scope();
                        self.indent -= 1;
                        self.writeln("}}");
                    }
                }
                syn::Expr::If(else_if) => {
                    if self.block_contains_early_return_or_try(&else_if.then_branch)
                        || else_if
                            .else_branch
                            .as_ref()
                            .is_some_and(|(_, e)| self.expr_contains_early_return_or_try(e))
                    {
                        self.writeln("} else {");
                        self.indent += 1;
                        self.emit_if_assign_as_statement_block(result_var, else_if, expected_ty);
                        self.indent -= 1;
                        self.writeln("}}");
                    } else {
                        let val = self.emit_expr_to_string_with_expected(else_expr, expected_ty);
                        self.writeln(&format!("}} else {{ {} = {}; }}}}", result_var, val));
                    }
                }
                other => {
                    let val = self.emit_expr_to_string_with_expected(other, expected_ty);
                    self.writeln(&format!("}} else {{ {} = {}; }}}}", result_var, val));
                }
            }
        } else {
            self.writeln("}}");
        }
    }

    pub(super) fn emit_if_let_statement_scope_bindings(
        &mut self,
        let_pat: &syn::Pat,
        scrutinee_var: &str,
        unwrap_method: &str,
        scrutinee_is_as_mut: bool,
    ) {
        let binding_pat = match let_pat {
            syn::Pat::TupleStruct(ts) if ts.elems.len() == 1 => ts.elems.first(),
            syn::Pat::Ident(_) => Some(let_pat),
            _ => None,
        };

        let Some(binding_pat) = binding_pat else {
            return;
        };

        let simple_ident = match binding_pat {
            syn::Pat::Ident(pi) if pi.ident != "_" && pi.subpat.is_none() => {
                Some(pi.ident.to_string())
            }
            _ => None,
        };

        if let Some(rust_name) = simple_ident {
            let cpp_name = self.allocate_local_cpp_name(&rust_name);
            if unwrap_method == IF_LET_OPTION_TAKE_VALUE_HELPER_MARKER {
                self.writeln(&format!("auto&& _iflet_take = {};", scrutinee_var));
                self.writeln(&format!(
                    "auto {} = rusty::detail::option_take_value(_iflet_take);",
                    cpp_name
                ));
            } else {
                let unwrap_expr = self.emit_if_let_unwrap_expr(scrutinee_var, unwrap_method);
                if scrutinee_is_as_mut {
                    self.writeln(&format!("auto& {} = *{};", cpp_name, unwrap_expr));
                } else {
                    self.writeln(&format!("auto {} = {};", cpp_name, unwrap_expr));
                }
            }
            let mut binding_map = HashMap::new();
            binding_map.insert(rust_name, cpp_name);
            self.register_statement_scope_binding_map(&binding_map);
            return;
        }

        let payload_var = "_iflet_payload";
        if unwrap_method == IF_LET_OPTION_TAKE_VALUE_HELPER_MARKER {
            self.writeln(&format!("auto&& _iflet_take = {};", scrutinee_var));
            self.writeln(&format!(
                "auto&& {} = rusty::detail::option_take_value(_iflet_take);",
                payload_var
            ));
        } else {
            let unwrap_expr = self.emit_if_let_unwrap_expr(scrutinee_var, unwrap_method);
            if scrutinee_is_as_mut {
                self.writeln(&format!("auto&& {} = *{};", payload_var, unwrap_expr));
            } else {
                self.writeln(&format!("auto&& {} = {};", payload_var, unwrap_expr));
            }
        }

        let mut binding_stmts = Vec::new();
        let mut binding_map = HashMap::new();
        if self.collect_pattern_binding_stmts_with_cpp_name_map(
            binding_pat,
            payload_var,
            &mut binding_stmts,
            &mut binding_map,
        ) {
            for stmt in binding_stmts {
                self.writeln(&stmt);
            }
            self.register_statement_scope_binding_map(&binding_map);
        }
    }

    pub(super) fn emit_if_let_as_statement_block(
        &mut self,
        tuple: &syn::PatTuple,
        if_expr: &syn::ExprIf,
    ) -> Option<()> {
        let Some((_, else_branch)) = &if_expr.else_branch else {
            return None;
        };

        let inferred_result_ty = self.infer_tuple_result_type_for_if_expr(if_expr);
        let inferred_result_elem_expected =
            self.infer_tuple_result_elem_expected_types_for_if_expr(if_expr);

        // Emit else-branch value as default for the result variable.
        let else_result = match else_branch.as_ref() {
            syn::Expr::Block(b) => self.extract_single_expr_from_block(&b.block).map(|e| {
                self.emit_expr_with_tuple_elem_expected_types(
                    e,
                    inferred_result_elem_expected.as_deref(),
                    inferred_result_ty.as_ref(),
                )
            }),
            other => Some(self.emit_expr_with_tuple_elem_expected_types(
                other,
                inferred_result_elem_expected.as_deref(),
                inferred_result_ty.as_ref(),
            )),
        };
        let else_has_early_return = match else_branch.as_ref() {
            syn::Expr::Block(b) => self.block_contains_early_return_or_try(&b.block),
            _ => false,
        };

        let result_var = format!("_iflet_result{}", self.iflet_result_counter);
        self.iflet_result_counter += 1;
        if let Some(else_value) = &else_result {
            if let Some(result_ty) = inferred_result_ty.as_ref() {
                let result_cpp_ty = self.map_type(result_ty);
                self.writeln(&format!(
                    "{} {} = {};",
                    result_cpp_ty, result_var, else_value
                ));
            } else {
                self.writeln(&format!("auto {} = {};", result_var, else_value));
            }
        } else if else_has_early_return {
            // Else always returns early — we need a dummy init.
            // Extract then-branch tail expr to get the type via decltype.
            let then_tail = if_expr
                .then_branch
                .stmts
                .last()
                .and_then(|s| {
                    if let syn::Stmt::Expr(e, None) = s {
                        Some(e)
                    } else {
                        None
                    }
                })
                .map(|e| self.emit_expr_to_string(e));
            if let Some(tail_str) = then_tail {
                self.writeln(&format!("decltype({}) {} {{}};", tail_str, result_var));
            } else {
                return None;
            }
        } else {
            return None;
        }

        self.writeln("{");
        self.indent += 1;

        if let syn::Expr::Let(let_expr) = &*if_expr.cond {
            let scrutinee = self.emit_expr_to_string(&let_expr.expr);
            self.writeln(&format!("auto&& _iflet_scrutinee = {};", scrutinee));
            if let Some(variant_cond) =
                self.if_let_binding_less_variant_condition(&let_expr.pat, "_iflet_scrutinee")
            {
                // Data-enum variant test with no bindings: the Option surface
                // (`.is_some()`) does not exist on the variant scrutinee.
                self.writeln(&format!("if ({}) {{", variant_cond));
                self.indent += 1;
                self.push_transient_statement_scope();
            } else {
                let (some_check, _none, unwrap, _neg) =
                    self.option_like_pattern_surface_for_expr(&let_expr.expr);
                self.writeln(&format!("if (_iflet_scrutinee.{}()) {{", some_check));
                self.indent += 1;
                self.push_transient_statement_scope();
                self.emit_if_let_statement_scope_bindings(
                    &let_expr.pat,
                    "_iflet_scrutinee",
                    unwrap,
                    scrutinee.ends_with(".as_mut()"),
                );
            }
        } else {
            let cond = self.emit_expr_to_string(&if_expr.cond);
            self.writeln(&format!("if ({}) {{", cond));
            self.indent += 1;
            self.push_transient_statement_scope();
        }

        // Emit then-branch statements; assign tail expression to _iflet_result
        let stmts = &if_expr.then_branch.stmts;
        for (i, stmt) in stmts.iter().enumerate() {
            let is_last = i == stmts.len() - 1;
            if is_last {
                if let syn::Stmt::Expr(expr, None) = stmt {
                    // If the tail expression is itself an if-expression with
                    // ?/return, recursively lower it as a statement block
                    // that assigns to result_var.
                    if let syn::Expr::If(inner_if) = expr {
                        if self.block_contains_early_return_or_try(&inner_if.then_branch)
                            || inner_if
                                .else_branch
                                .as_ref()
                                .is_some_and(|(_, e)| self.expr_contains_early_return_or_try(e))
                        {
                            self.emit_if_assign_as_statement_block(
                                &result_var,
                                inner_if,
                                inferred_result_ty.as_ref(),
                            );
                            continue;
                        }
                    }
                    let val =
                        self.emit_expr_to_string_with_expected(expr, inferred_result_ty.as_ref());
                    self.writeln(&format!("{} = {};", result_var, val));
                    continue;
                }
            }
            self.emit_stmt(stmt, false);
        }

        self.pop_transient_statement_scope();
        self.indent -= 1;
        self.writeln("}");
        self.indent -= 1;
        self.writeln("}");

        // Destructure the result into the pattern bindings
        let names: Vec<String> = tuple
            .elems
            .iter()
            .map(|p| {
                let raw = self.emit_pat_to_string(p);
                if raw == "_" {
                    raw
                } else {
                    self.allocate_local_cpp_name(&raw)
                }
            })
            .collect();
        self.writeln(&format!(
            "auto [{}] = std::move({});",
            names.join(", "),
            result_var
        ));
        if let Some(syn::Type::Tuple(tuple_ty)) = inferred_result_ty.as_ref() {
            for (pat, elem_ty) in tuple.elems.iter().zip(tuple_ty.elems.iter()) {
                if let syn::Pat::Ident(pi) = pat {
                    if pi.ident != "_" {
                        self.update_local_binding_type(pi.ident.to_string(), elem_ty.clone());
                    }
                }
            }
        }
        Some(())
    }

    /// Lower an if-expression with multi-statement branches into a C++ IIFE
    /// (immediately-invoked function expression): `[&]() { if (...) { ... } else { ... } }()`.
    pub(super) fn emit_if_expr_as_iife(
        &self,
        if_expr: &syn::ExprIf,
        expected_ty: Option<&syn::Type>,
    ) -> String {
        let mut parts = Vec::new();
        parts.push("[&]() {".to_string());

        // Emit the if condition
        if let syn::Expr::Let(let_expr) = &*if_expr.cond {
            // if let Some(x) = expr { ... } else { ... }
            let scrutinee = self.emit_expr_to_string(&let_expr.expr);
            parts.push(format!("auto&& _iflet_scrutinee = {};", scrutinee));
            if let Some(variant_cond) =
                self.if_let_binding_less_variant_condition(&let_expr.pat, "_iflet_scrutinee")
            {
                // Data-enum variant test with no bindings: the Option surface
                // (`.is_some()`) does not exist on the variant scrutinee.
                parts.push(format!("if ({}) {{", variant_cond));
            } else {
                let (some_check, _none_check, unwrap_method, _negated) =
                    self.option_like_pattern_surface_for_expr(&let_expr.expr);
                // Extract the binding name from the pattern
                let binding = self.extract_if_let_binding_name(&let_expr.pat);
                parts.push(format!("if (_iflet_scrutinee.{}()) {{", some_check));
                if let Some(name) = &binding {
                    parts.push(format!(
                        "auto {} = _iflet_scrutinee.{}();",
                        name, unwrap_method
                    ));
                }
            }
        } else {
            let cond = self.emit_expr_to_string(&if_expr.cond);
            parts.push(format!("if ({}) {{", cond));
        }

        // Emit then-branch body
        if !if_expr.then_branch.stmts.is_empty() {
            let mut then_inner = self.new_inner_for_block();
            let then_len = if_expr.then_branch.stmts.len();
            for (idx, stmt) in if_expr.then_branch.stmts.iter().enumerate() {
                let stmt_expected = if idx + 1 == then_len {
                    expected_ty
                } else {
                    None
                };
                match stmt {
                    // Only the TAIL expression yields the branch value. A NON-final
                    // `Stmt::Expr(_, None)` is a brace-terminated statement (`loop{}`,
                    // `if{}`, `match{}`, …) in statement position — `return`ing it
                    // would give the IIFE conflicting deduced return types
                    // (e.g. void/`loop{}` vs `rusty::Unit`/`if{}` vs the real tail).
                    syn::Stmt::Expr(expr, None) if idx + 1 == then_len => {
                        let expr_str =
                            then_inner.emit_expr_to_string_with_expected(expr, stmt_expected);
                        then_inner.writeln(&format!("return {};", expr_str));
                    }
                    _ => then_inner.emit_stmt(stmt, false),
                }
            }
            let then_body = then_inner.output.trim().to_string();
            if !then_body.is_empty() {
                parts.push(then_body);
            }
        }
        parts.push("} else {".to_string());

        // Emit else-branch body
        if let Some((_, else_expr)) = &if_expr.else_branch {
            match else_expr.as_ref() {
                syn::Expr::Block(block) => {
                    if !block.block.stmts.is_empty() {
                        let mut else_inner = self.new_inner_for_block();
                        let else_len = block.block.stmts.len();
                        for (idx, stmt) in block.block.stmts.iter().enumerate() {
                            let stmt_expected = if idx + 1 == else_len {
                                expected_ty
                            } else {
                                None
                            };
                            match stmt {
                                // Only the TAIL expression yields the branch value;
                                // a non-final `Stmt::Expr(_, None)` is a
                                // brace-terminated statement, not a return value.
                                syn::Stmt::Expr(expr, None) if idx + 1 == else_len => {
                                    let expr_str = else_inner
                                        .emit_expr_to_string_with_expected(expr, stmt_expected);
                                    else_inner.writeln(&format!("return {};", expr_str));
                                }
                                _ => else_inner.emit_stmt(stmt, false),
                            }
                        }
                        let else_body = else_inner.output.trim().to_string();
                        if !else_body.is_empty() {
                            parts.push(else_body);
                        }
                    }
                }
                syn::Expr::If(nested_if) => {
                    // else if ... — emit as nested if
                    let nested = self.emit_if_expr_as_iife(nested_if, expected_ty);
                    parts.push(format!("return {};", nested));
                }
                other => {
                    let else_str = self.emit_expr_to_string_with_expected(other, expected_ty);
                    parts.push(format!("return {};", else_str));
                }
            }
        }
        parts.push("}".to_string());
        parts.push("}()".to_string());

        parts.join("\n")
    }

    pub(super) fn emit_stmt_to_string_with_expected(
        &self,
        stmt: &syn::Stmt,
        expected_ty: Option<&syn::Type>,
    ) -> String {
        match stmt {
            syn::Stmt::Expr(expr, None) => {
                let expr_str = self.emit_expr_to_string_with_expected(expr, expected_ty);
                format!("return {};", expr_str)
            }
            _ => self.emit_stmt_to_string(stmt),
        }
    }

    /// Emit a single statement as a string (for use inside IIFE blocks).
    pub(super) fn emit_stmt_to_string(&self, stmt: &syn::Stmt) -> String {
        let mut inner = self.new_inner_for_block();
        inner.emit_stmt(stmt, false);
        inner.output.trim().to_string()
    }

    pub(super) fn emit_if_let_expr_to_string(
        &self,
        let_expr: &syn::ExprLet,
        then_branch: &syn::Block,
        else_branch: &Option<(syn::token::Else, Box<syn::Expr>)>,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let Some((_, else_expr_box)) = else_branch else {
            return None;
        };
        let then_expr = self.extract_single_expr_from_block(then_branch)?;
        let else_expr = self.extract_single_value_expr(else_expr_box.as_ref())?;
        let Some((cond_expr, binding_name, unwrap_method)) =
            self.if_let_expr_condition_parts(let_expr)
        else {
            return self.emit_if_let_expr_runtime_pattern_to_string(
                let_expr,
                then_expr,
                else_expr,
                expected_ty,
            );
        };

        let inferred_expected_ty = if expected_ty.is_none() && binding_name.is_none() {
            self.infer_constructor_expected_type_from_pair(then_expr, else_expr)
        } else {
            None
        };
        let branch_expected_ty = expected_ty.or(inferred_expected_ty.as_ref());
        let scrutinee = self.emit_expr_to_string(&let_expr.expr);

        let then_value = {
            let mut emitted = self.emit_expr_to_string_with_expected(then_expr, branch_expected_ty);
            if let Some(binding) = binding_name {
                // `if let Err(error) = ... { ... error ... }` can shadow same-named
                // item paths. The branch emitter may conservatively qualify unresolved
                // paths as `::error`; remap those back to the local binding for this
                // expression-scoped if-let lowering.
                let shadowed_global = format!("::{}", binding);
                if emitted.contains(&shadowed_global) {
                    emitted = emitted.replace(&shadowed_global, &binding);
                }
                if let Some(unwrap) = unwrap_method {
                    let unwrap_expr = self.emit_if_let_unwrap_expr("_iflet", unwrap);
                    format!(
                        "([&]() {{ auto {} = {}; return {}; }}())",
                        binding, unwrap_expr, emitted
                    )
                } else {
                    format!(
                        "([&]() {{ auto {} = _iflet; return {}; }}())",
                        binding, emitted
                    )
                }
            } else {
                emitted
            }
        };

        let else_value = match else_expr {
            syn::Expr::If(else_if) => self.emit_if_expr_to_string(else_if, branch_expected_ty),
            _ => self.emit_expr_to_string_with_expected(else_expr, branch_expected_ty),
        };

        Some(format!(
            "[&]() {{ auto&& _iflet = {}; return ({} ? {} : {}); }}()",
            scrutinee, cond_expr, then_value, else_value
        ))
    }

    pub(super) fn emit_if_let_expr_runtime_pattern_to_string(
        &self,
        let_expr: &syn::ExprLet,
        then_expr: &syn::Expr,
        else_expr: &syn::Expr,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let scrutinee = self.emit_expr_to_string(&let_expr.expr);
        let variant_ctx = self.infer_variant_type_context_from_expr(&let_expr.expr);
        let mut binding_stmts = Vec::new();
        let mut binding_map = HashMap::new();
        let cond_expr = self
            .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                &let_expr.pat,
                "_iflet",
                &mut binding_stmts,
                &mut binding_map,
                variant_ctx.as_ref(),
            )?
            .unwrap_or_else(|| "true".to_string());

        let inferred_expected_ty = if expected_ty.is_none() {
            self.infer_constructor_expected_type_from_pair(then_expr, else_expr)
        } else {
            None
        };
        let branch_expected_ty = expected_ty.or(inferred_expected_ty.as_ref());

        let then_value = if binding_map.is_empty() {
            self.emit_expr_to_string_with_expected(then_expr, branch_expected_ty)
        } else {
            let mut inner = self.new_inner_for_block();
            inner.local_cpp_bindings.push(binding_map.clone());
            let mut local_types = HashMap::new();
            let mut local_consts = HashMap::new();
            for rust_name in binding_map.keys() {
                local_types.insert(rust_name.clone(), None);
                local_consts.insert(rust_name.clone(), false);
            }
            inner.local_bindings.push(local_types);
            inner.local_shadowed_binding_types.push(HashMap::new());
            inner.local_const_bindings.push(local_consts);
            inner.local_reference_bindings.push(HashSet::new());
            inner.rebind_reference_pointer_bindings.push(HashSet::new());
            let emitted = inner.emit_expr_to_string_with_expected(then_expr, branch_expected_ty);
            inner.local_reference_bindings.pop();
            inner.rebind_reference_pointer_bindings.pop();
            inner.local_const_bindings.pop();
            inner.local_shadowed_binding_types.pop();
            inner.local_bindings.pop();
            inner.local_cpp_bindings.pop();
            emitted
        };

        let else_value = match else_expr {
            syn::Expr::If(else_if) => self.emit_if_expr_to_string(else_if, branch_expected_ty),
            _ => self.emit_expr_to_string_with_expected(else_expr, branch_expected_ty),
        };

        let lambda_return_annotation =
            self.expected_lambda_return_annotation(branch_expected_ty, false);

        let bindings_prefix = if binding_stmts.is_empty() {
            String::new()
        } else {
            format!("{}\n", binding_stmts.join("\n"))
        };

        Some(format!(
            "[&](){} {{ auto&& _iflet = {}; if ({}) {{ {}return {}; }} return {}; }}()",
            lambda_return_annotation, scrutinee, cond_expr, bindings_prefix, then_value, else_value
        ))
    }

    pub(super) fn emit_if_ternary_branch_expr(
        &self,
        branch_expr: &syn::Expr,
        expected_ty: Option<&syn::Type>,
        ctor_template_args: Option<&[String]>,
    ) -> String {
        let value_expr = self.extract_value_expr(branch_expr).unwrap_or(branch_expr);
        // Prefer the explicit-template-args path whenever we have args
        // available, regardless of whether `expected_ty` is Some. The
        // expected-ty path emits `Either_Left{arg}` (bare brace-init),
        // which fails CTAD because `Either_Left<L, R>` can't deduce
        // `R` from a single brace-init element. The template-args path
        // emits `rusty::either::Left<L, R>(arg)` — explicit args on a
        // free-function factory — which has no deduction to do. The
        // surrounding `inferred_expected_cpp_ty` wrap (`Either<L,
        // R>(...)`) at the ternary-emit site handles the alias-CTAD
        // case once both arms produce a known type. See
        // docs/rusty-cpp-transpiler.md §13.3 for the architectural
        // background.
        if let (Some(args), syn::Expr::Call(call)) = (ctor_template_args, value_expr) {
            if let Some(emitted) =
                self.try_emit_variant_constructor_call_with_template_args(call, args)
            {
                return emitted;
            }
        }
        self.emit_expr_to_string_with_expected(value_expr, expected_ty)
    }
}
