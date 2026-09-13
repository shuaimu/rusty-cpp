use super::*;

impl CodeGen {
    /// Rust's owning Wake receiver is an Arc parameter, not a C++ `this`.
    /// Normalize before metadata collection so argument inference agrees with
    /// the emitted static function and self-clones preserve Arc ownership.
    pub(super) fn normalize_standard_task_receivers(&mut self, file: &syn::File) -> syn::File {
        struct SelfPaths;
        impl syn::visit_mut::VisitMut for SelfPaths {
            fn visit_expr_mut(&mut self, expr: &mut syn::Expr) {
                if let syn::Expr::Path(path) = expr
                    && path.qself.is_none()
                    && path.path.is_ident("self")
                {
                    *expr = syn::parse_quote!(_wake_self);
                    return;
                }
                syn::visit_mut::visit_expr_mut(self, expr);
            }
        }
        struct PinnedSelf;
        impl syn::visit_mut::VisitMut for PinnedSelf {
            fn visit_expr_mut(&mut self, expr: &mut syn::Expr) {
                syn::visit_mut::visit_expr_mut(self, expr);
                if let syn::Expr::MethodCall(method) = expr
                    && method.args.is_empty()
                    && matches!(method.method.to_string().as_str(), "get_mut" | "as_mut")
                    && matches!(method.receiver.as_ref(), syn::Expr::Path(path) if path.path.is_ident("self"))
                {
                    *expr = syn::parse_quote!(self);
                }
            }
        }
        fn walk(cg: &mut CodeGen, items: &mut [syn::Item], scope: &mut Vec<String>) {
            for item in items {
                if let syn::Item::Mod(module) = item {
                    if let Some((_, items)) = &mut module.content {
                        scope.push(module.ident.to_string());
                        walk(cg, items, scope);
                        scope.pop();
                    }
                } else if let syn::Item::Impl(implementation) = item {
                    let Some((_, path, _)) = &implementation.trait_ else {
                        continue;
                    };
                    if cg.standard_future_path_is_in_scope(path, "std::task::Wake", scope) {
                        if let syn::Type::Path(target) = implementation.self_ty.as_ref() {
                            if let Some(tail) = target.path.segments.last() {
                                let name = scope
                                    .iter()
                                    .map(String::as_str)
                                    .chain(std::iter::once(tail.ident.to_string().as_str()))
                                    .collect::<Vec<_>>()
                                    .join("::");
                                let methods = implementation
                                    .items
                                    .iter()
                                    .filter_map(|item| match item {
                                        syn::ImplItem::Fn(method) => {
                                            Some(method.sig.ident.to_string())
                                        }
                                        _ => None,
                                    })
                                    .collect();
                                cg.standard_wake_methods.insert(name, methods);
                            }
                        }
                        for item in &mut implementation.items {
                            let syn::ImplItem::Fn(method) = item else {
                                continue;
                            };
                            if !matches!(
                                method.sig.ident.to_string().as_str(),
                                "wake" | "wake_by_ref"
                            ) {
                                continue;
                            }
                            let Some(syn::FnArg::Receiver(receiver)) = method.sig.inputs.first()
                            else {
                                continue;
                            };
                            assert!(
                                receiver.colon_token.is_some(),
                                "standard Wake methods require an explicit Arc<Self> receiver"
                            );
                            let ty = receiver.ty.clone();
                            *method.sig.inputs.first_mut().unwrap() =
                                syn::parse_quote!(_wake_self: #ty);
                            syn::visit_mut::VisitMut::visit_block_mut(
                                &mut SelfPaths,
                                &mut method.block,
                            );
                        }
                    } else if cg.standard_future_path_is_in_scope(
                        path,
                        "std::future::Future",
                        scope,
                    ) {
                        for item in &mut implementation.items {
                            let syn::ImplItem::Fn(method) = item else {
                                continue;
                            };
                            if method.sig.ident != "poll" {
                                continue;
                            }
                            let Some(syn::FnArg::Receiver(receiver)) = method.sig.inputs.first()
                            else {
                                continue;
                            };
                            if receiver.colon_token.is_none() {
                                continue;
                            }
                            let syn::Type::Path(pin) = receiver.ty.as_ref() else {
                                continue;
                            };
                            if !cg.standard_future_path_is_in_scope(
                                &pin.path,
                                "std::pin::Pin",
                                scope,
                            ) {
                                continue;
                            }
                            let syn::PathArguments::AngleBracketed(args) =
                                &pin.path.segments.last().unwrap().arguments
                            else {
                                continue;
                            };
                            assert!(
                                matches!(args.args.first(), Some(syn::GenericArgument::Type(syn::Type::Reference(reference))) if reference.mutability.is_some()),
                                "standard Future::poll requires Pin<&mut Self>"
                            );
                            *method.sig.inputs.first_mut().unwrap() = syn::parse_quote!(&mut self);
                            syn::visit_mut::VisitMut::visit_block_mut(
                                &mut PinnedSelf,
                                &mut method.block,
                            );
                        }
                    }
                }
            }
        }
        let mut file = file.clone();
        self.standard_wake_methods.clear();
        walk(self, &mut file.items, &mut Vec::new());
        file
    }

    /// A Ready binding moves its payload out of an owned Poll. Keep the local
    /// mutable in C++ so unwrap cannot select the const, borrowing overload.
    pub(super) fn collect_standard_poll_moved_scrutinees(
        &self,
        stmts: &[syn::Stmt],
    ) -> std::collections::HashSet<String> {
        struct Scan<'a> {
            cg: &'a CodeGen,
            found: std::collections::HashSet<String>,
        }
        impl<'ast> syn::visit::Visit<'ast> for Scan<'_> {
            fn visit_expr_let(&mut self, node: &'ast syn::ExprLet) {
                if let syn::Pat::TupleStruct(tuple) = node.pat.as_ref()
                    && self.cg.standard_poll_variant(&tuple.path).as_deref() == Some("Ready")
                    && tuple.elems.iter().any(|pat| {
                        matches!(pat, syn::Pat::Ident(binding) if binding.by_ref.is_none())
                    })
                    && let Some(name) = by_value_match_scrutinee_local(
                        self.cg.peel_paren_group_expr(&node.expr),
                    )
                {
                    self.found.insert(name);
                }
                syn::visit::visit_expr_let(self, node);
            }
        }
        let mut scan = Scan {
            cg: self,
            found: std::collections::HashSet::new(),
        };
        for stmt in stmts {
            syn::visit::Visit::visit_stmt(&mut scan, stmt);
        }
        scan.found
    }

    pub(super) fn try_emit_standard_poll_if_let(
        &mut self,
        let_expr: &syn::ExprLet,
        then_branch: &syn::Block,
        else_branch: &Option<(syn::token::Else, Box<syn::Expr>)>,
        first: bool,
    ) -> bool {
        let (path, binding) = match let_expr.pat.as_ref() {
            syn::Pat::TupleStruct(tuple) if tuple.elems.len() == 1 => {
                (&tuple.path, tuple.elems.first())
            }
            syn::Pat::Path(path) => (&path.path, None),
            _ => return false,
        };
        let Some(variant) = self.standard_poll_variant(path) else {
            return false;
        };
        let (expr, mut borrow) = match self.peel_paren_group_expr(&let_expr.expr) {
            syn::Expr::Reference(reference) => (
                reference.expr.as_ref(),
                Some(reference.mutability.is_some()),
            ),
            expr => (expr, None),
        };
        if borrow.is_none() {
            if let Some(syn::Type::Reference(reference)) = self.infer_simple_expr_type(expr) {
                borrow = Some(reference.mutability.is_some());
            }
        }
        let emitted = self.emit_expr_to_string(expr);
        let condition = if variant == "Ready" {
            "_poll_iflet.is_ready()"
        } else {
            "_poll_iflet.is_pending()"
        };
        self.emit_if_let_body(
            condition,
            binding,
            "_poll_iflet",
            expr,
            "unwrap",
            then_branch,
            else_branch,
            first,
            Some(&emitted),
            false,
            borrow,
        );
        true
    }

    /// Resolve source imports before recognizing standard task types. A local
    /// module/type/trait with the same spelling never acquires runtime semantics.
    pub(super) fn standard_future_path_is(&self, path: &syn::Path, target: &str) -> bool {
        self.standard_future_path_is_in_scope(path, target, &self.module_stack)
    }

    fn standard_future_path_is_in_scope(
        &self,
        path: &syn::Path,
        target: &str,
        scope: &[String],
    ) -> bool {
        let Some(mut spelling) = self.auto_trait_external_identity(path, scope) else {
            return false;
        };
        let original_root = path
            .segments
            .first()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default();
        // These are Rust prelude types/traits. Require no local declaration
        // when the import resolver leaves their bare source spelling intact.
        for (bare, qualified) in [
            ("Box", "std::boxed::Box"),
            ("Future", "std::future::Future"),
            ("Send", "std::marker::Send"),
            ("Sync", "std::marker::Sync"),
        ] {
            if path.leading_colon.is_none()
                && original_root == bare
                && (spelling == bare || spelling.starts_with(&format!("{bare}::")))
            {
                if self.bare_std_named_type_suppression_applies(bare) {
                    return false;
                }
                spelling = format!("{qualified}{}", &spelling[bare.len()..]);
                break;
            }
        }
        let suffix = target.strip_prefix("std::").unwrap_or(target);
        let wake_trait = suffix == "task::Wake" || suffix.starts_with("task::Wake::");
        let core_allowed =
            !suffix.starts_with("boxed::") && !suffix.starts_with("sync::Arc") && !wake_trait;
        let alloc_allowed =
            suffix.starts_with("boxed::") || suffix.starts_with("sync::Arc") || wake_trait;
        spelling == target
            || (core_allowed && spelling == format!("core::{suffix}"))
            || (alloc_allowed && spelling == format!("alloc::{suffix}"))
    }

    fn standard_future_resolve_alias(&self, ty: &syn::Type) -> syn::Type {
        let mut resolved = self.peel_paren_group_type(ty).clone();
        for _ in 0..8 {
            let Some(next) = self.resolve_type_alias_once(&resolved) else {
                break;
            };
            if next == resolved {
                break;
            }
            resolved = self.peel_paren_group_type(&next).clone();
        }
        resolved
    }

    fn standard_future_type_arg(&self, ty: &syn::Type, owner: &str) -> Option<syn::Type> {
        let ty = self.standard_future_resolve_alias(ty);
        let syn::Type::Path(tp) = &ty else {
            return None;
        };
        if tp.qself.is_some() || !self.standard_future_path_is(&tp.path, owner) {
            return None;
        }
        let syn::PathArguments::AngleBracketed(args) = &tp.path.segments.last()?.arguments else {
            return None;
        };
        let mut types = args.args.iter().filter_map(|arg| match arg {
            syn::GenericArgument::Type(ty) => Some(ty.clone()),
            _ => None,
        });
        let result = types.next()?;
        if types.next().is_some() {
            return None;
        }
        Some(result)
    }

    pub(super) fn standard_poll_output(&self, ty: &syn::Type) -> Option<syn::Type> {
        self.standard_future_type_arg(self.peel_reference_paren_group_type(ty), "std::task::Poll")
    }

    pub(super) fn standard_pinned_future_output(&self, ty: &syn::Type) -> Option<syn::Type> {
        let pin = self
            .standard_future_type_arg(self.peel_reference_paren_group_type(ty), "std::pin::Pin")?;
        let boxed = self.standard_future_type_arg(&pin, "std::boxed::Box")?;
        let boxed = self.standard_future_resolve_alias(&boxed);
        let syn::Type::TraitObject(object) = &boxed else {
            return None;
        };
        let mut output = None;
        let mut markers = HashSet::new();
        for bound in &object.bounds {
            match bound {
                syn::TypeParamBound::Lifetime(_) => {}
                syn::TypeParamBound::Trait(bound)
                    if bound.lifetimes.is_none()
                        && matches!(bound.modifier, syn::TraitBoundModifier::None) =>
                {
                    if self.standard_future_path_is(&bound.path, "std::future::Future") {
                        let syn::PathArguments::AngleBracketed(args) =
                            &bound.path.segments.last()?.arguments
                        else {
                            return None;
                        };
                        if args.args.len() != 1 {
                            return None;
                        }
                        let syn::GenericArgument::AssocType(assoc) = args.args.first()? else {
                            return None;
                        };
                        if assoc.ident != "Output"
                            || assoc.generics.is_some()
                            || output.replace(assoc.ty.clone()).is_some()
                        {
                            return None;
                        }
                    } else if let Some(marker) = ["Send", "Sync"].into_iter().find(|marker| {
                        self.standard_future_path_is(&bound.path, &format!("std::marker::{marker}"))
                    }) {
                        if !markers.insert(marker) {
                            return None;
                        }
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        output
    }

    pub(super) fn standard_future_output_cpp(&self, ty: &syn::Type) -> String {
        if matches!(self.peel_paren_group_type(ty), syn::Type::Tuple(tuple) if tuple.elems.is_empty())
        {
            "void".to_string()
        } else {
            self.map_type(ty)
        }
    }

    pub(super) fn try_map_standard_future_type(&self, ty: &syn::Type) -> Option<String> {
        // References retain their source constness through ordinary map_type.
        if !matches!(self.peel_paren_group_type(ty), syn::Type::Path(_)) {
            return None;
        }
        if let Some(output) = self.standard_pinned_future_output(ty) {
            return Some(format!(
                "rusty::Task<{}>",
                self.standard_future_output_cpp(&output)
            ));
        }
        if let Some(pin) = self.standard_future_type_arg(ty, "std::pin::Pin")
            && let Some(boxed) = self.standard_future_type_arg(&pin, "std::boxed::Box")
            && let syn::Type::TraitObject(object) = self.standard_future_resolve_alias(&boxed)
            && object.bounds.iter().any(|bound| {
                matches!(bound, syn::TypeParamBound::Trait(bound)
                if self.standard_future_path_is(&bound.path, "std::future::Future"))
            })
        {
            panic!(
                "unsupported standard pinned Future: expected Pin<Box<dyn Future<Output = T> + optional Send/Sync/lifetime bounds>> without higher-ranked or additional trait bounds"
            );
        }
        self.standard_poll_output(ty)
            .map(|output| format!("rusty::Poll<{}>", self.standard_future_output_cpp(&output)))
    }

    pub(super) fn standard_poll_variant(&self, path: &syn::Path) -> Option<String> {
        ["Ready", "Pending"]
            .into_iter()
            .find(|variant| {
                self.standard_future_path_is(path, &format!("std::task::Poll::{variant}"))
            })
            .map(str::to_string)
    }

    fn standard_call_owner(&self, path: &syn::Path, owner: &str, method: &str) -> bool {
        path.segments
            .last()
            .is_some_and(|segment| segment.ident == method)
            && self.standard_future_path_is(path, &format!("{owner}::{method}"))
    }

    pub(super) fn try_emit_standard_future_pending(
        &self,
        expr: &syn::Expr,
        expected: Option<&syn::Type>,
    ) -> Option<String> {
        let syn::Expr::Path(path) = self.peel_paren_group_expr(expr) else {
            return None;
        };
        if path.qself.is_some()
            || self.standard_poll_variant(&path.path).as_deref() != Some("Pending")
        {
            return None;
        }
        let output = expected
            .and_then(|ty| self.standard_poll_output(ty))
            .or_else(|| {
                path.path.segments.iter().find_map(|segment| {
                    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
                        return None;
                    };
                    args.args.iter().find_map(|arg| match arg {
                        syn::GenericArgument::Type(ty) => Some(ty.clone()),
                        _ => None,
                    })
                })
            });
        Some(match output {
            Some(output) => format!(
                "rusty::Poll<{}>::pending()",
                self.standard_future_output_cpp(&output)
            ),
            None => "rusty::future::Pending{}".to_string(),
        })
    }

    pub(super) fn try_emit_standard_future_call(
        &self,
        call: &syn::ExprCall,
        expected: Option<&syn::Type>,
    ) -> Option<String> {
        let syn::Expr::Path(path) = self.peel_paren_group_expr(&call.func) else {
            return None;
        };
        if path.qself.is_some() {
            return None;
        }
        for method in ["wake", "wake_by_ref"] {
            if self.standard_call_owner(&path.path, "std::task::Wake", method)
                && call.args.len() == 1
            {
                let ty = self.infer_simple_expr_type(&call.args[0])?;
                let target = self.standard_wake_target(&ty)?;
                let arg = match self.peel_paren_group_expr(&call.args[0]) {
                    syn::Expr::Reference(reference) => reference.expr.as_ref(),
                    arg => arg,
                };
                let value = if method == "wake" {
                    self.emit_expr_maybe_move(arg)
                } else {
                    self.emit_expr_to_string(arg)
                };
                return Some(self.standard_wake_dispatch(&target, method, &value));
            }
        }
        if self.standard_poll_variant(&path.path).as_deref() == Some("Ready")
            && call.args.len() == 1
        {
            let output = expected
                .and_then(|ty| self.standard_poll_output(ty))
                .or_else(|| self.infer_simple_expr_type(&call.args[0]));
            let arg = self.emit_expr_maybe_move(&call.args[0]);
            let cpp = output
                .as_ref()
                .map(|ty| self.standard_future_output_cpp(ty))
                .unwrap_or_else(|| format!("std::remove_cvref_t<decltype({arg})>"));
            return Some(if cpp == "void" {
                format!("([&]() {{ (void)({arg}); return rusty::Poll<void>::ready_with(); }}())")
            } else {
                format!("rusty::Poll<{cpp}>::ready_with({arg})")
            });
        }
        if self.standard_call_owner(&path.path, "std::boxed::Box", "pin") && call.args.len() == 1 {
            if expected.is_some_and(|ty| self.standard_pinned_future_output(ty).is_none()) {
                return None;
            }
            let arg = self.emit_expr_maybe_move(&call.args[0]);
            return Some(format!("rusty::future::pin({arg})"));
        }
        if self.standard_call_owner(&path.path, "std::task::Context", "from_waker")
            && call.args.len() == 1
        {
            let arg = match self.peel_paren_group_expr(&call.args[0]) {
                syn::Expr::Reference(reference) => reference.expr.as_ref(),
                arg => arg,
            };
            return Some(format!(
                "rusty::Context{{std::addressof({})}}",
                self.emit_expr_to_string(arg)
            ));
        }
        if self.standard_call_owner(&path.path, "std::task::Waker", "from") && call.args.len() == 1
        {
            return Some(format!(
                "rusty::Waker::from_arc({})",
                self.emit_expr_maybe_move(&call.args[0])
            ));
        }
        None
    }

    pub(super) fn try_emit_standard_future_method(
        &self,
        mc: &syn::ExprMethodCall,
    ) -> Option<String> {
        if !matches!(
            mc.method.to_string().as_str(),
            "as_mut" | "poll" | "waker" | "wake" | "wake_by_ref" | "clone"
        ) {
            return None;
        }
        let receiver_ty = self
            .infer_simple_expr_type(&mc.receiver)
            .or_else(|| self.infer_local_binding_type_from_initializer(&mc.receiver))?;
        let receiver_ty =
            self.standard_future_resolve_alias(self.peel_reference_paren_group_type(&receiver_ty));
        let method = mc.method.to_string();
        if matches!(method.as_str(), "wake" | "wake_by_ref")
            && mc.args.is_empty()
            && let Some(target) = self.standard_wake_target(&receiver_ty)
        {
            let value = if method == "wake" {
                self.emit_expr_maybe_move(&mc.receiver)
            } else {
                self.emit_expr_to_string(&mc.receiver)
            };
            return Some(self.standard_wake_dispatch(&target, &method, &value));
        }
        if self.standard_pinned_future_output(&receiver_ty).is_some() {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            if method == "as_mut" && mc.args.is_empty() {
                return Some(format!("({receiver})"));
            }
            if method == "poll" && mc.args.len() == 1 {
                let cx = match self.peel_paren_group_expr(&mc.args[0]) {
                    syn::Expr::Reference(reference) => reference.expr.as_ref(),
                    arg => arg,
                };
                return Some(format!(
                    "({receiver}).poll({})",
                    self.emit_expr_to_string(cx)
                ));
            }
        }
        let syn::Type::Path(tp) = receiver_ty else {
            return None;
        };
        if self.standard_future_path_is(&tp.path, "std::task::Context")
            && method == "waker"
            && mc.args.is_empty()
        {
            return Some(format!(
                "(*({}).waker)",
                self.emit_expr_to_string(&mc.receiver)
            ));
        }
        if self.standard_future_path_is(&tp.path, "std::task::Waker") && mc.args.is_empty() {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return match method.as_str() {
                "clone" => Some(format!("rusty::Waker({receiver})")),
                "wake_by_ref" => Some(format!("({receiver}).wake_by_ref()")),
                "wake" => Some(format!("std::move({receiver}).wake()")),
                _ => None,
            };
        }
        None
    }

    fn standard_wake_target(&self, ty: &syn::Type) -> Option<syn::Type> {
        let target = self
            .standard_future_type_arg(self.peel_reference_paren_group_type(ty), "std::sync::Arc")?;
        if self.standard_wake_has_method(&target, "wake") {
            Some(target)
        } else {
            None
        }
    }

    fn standard_wake_has_method(&self, target: &syn::Type, method: &str) -> bool {
        let syn::Type::Path(path) = target else {
            return false;
        };
        let Some(tail) = path.path.segments.last() else {
            return false;
        };
        let name = if tail.ident == "Self" {
            self.current_struct.clone().unwrap_or_default()
        } else {
            tail.ident.to_string()
        };
        let scoped = self.scoped_type_key(&name);
        self.standard_wake_methods
            .get(&scoped)
            .or_else(|| self.standard_wake_methods.get(&name))
            .is_some_and(|methods| methods.contains(method))
    }

    fn standard_wake_dispatch(&self, target: &syn::Type, method: &str, value: &str) -> String {
        let cpp = self.map_type(target);
        if method == "wake_by_ref" && !self.standard_wake_has_method(target, "wake_by_ref") {
            format!("{cpp}::wake(rusty::clone({value}))")
        } else {
            format!("{cpp}::{method}({value})")
        }
    }

    pub(super) fn infer_standard_future_method(
        &self,
        mc: &syn::ExprMethodCall,
    ) -> Option<syn::Type> {
        if !matches!(
            mc.method.to_string().as_str(),
            "as_mut" | "poll" | "waker" | "clone"
        ) {
            return None;
        }
        let ty = self
            .infer_simple_expr_type(&mc.receiver)
            .or_else(|| self.infer_local_binding_type_from_initializer(&mc.receiver))?;
        if let Some(output) = self.standard_pinned_future_output(&ty) {
            if mc.method == "as_mut" {
                return Some(ty);
            }
            if mc.method == "poll" {
                return Some(syn::parse_quote!(::std::task::Poll<#output>));
            }
        }
        let resolved =
            self.standard_future_resolve_alias(self.peel_reference_paren_group_type(&ty));
        if let syn::Type::Path(tp) = &resolved {
            if mc.method == "waker" && self.standard_future_path_is(&tp.path, "std::task::Context")
            {
                return Some(syn::parse_quote!(&::std::task::Waker));
            }
            if mc.method == "clone" && self.standard_future_path_is(&tp.path, "std::task::Waker") {
                return Some(syn::parse_quote!(::std::task::Waker));
            }
        }
        None
    }
}
