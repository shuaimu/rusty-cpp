use super::*;


/// Emitted argument prefixes that already denote a span-producing rusty
/// helper — wrapping them again in as_slice would be redundant noise (and
/// breaks exact-emission guard tests).
const ARG_ALREADY_SPAN_PREFIXES: &[&str] = &[
    "rusty::as_slice(",
    "rusty::as_mut_slice(",
    "rusty::slice_full(",
    "rusty::slice(",
    "rusty::slice_from(",
    "rusty::slice_to(",
    "rusty::slice_to_inclusive(",
    "rusty::slice_inclusive(",
    "rusty::index_with_range(",
];

impl CodeGen {
    /// Rust `&[T]` params map to std::span — TEMPLATE DEDUCTION cannot
    /// convert a Vec argument (indexmap's `get_hash(&self.entries)`), so
    /// slice-expected args wrap in rusty::as_slice/as_mut_slice; idempotent
    /// when already a span. A Rust `&slice[range]` arg is the span VALUE
    /// itself — an address-of over a span-producing call
    /// (`&rusty::index_with_range(...)`) would pass span*, so the leading
    /// `&` strips instead.
    pub(super) fn coerce_slice_expected_arg_cpp(
        &self,
        arg_cpp: String,
        arg_expected_ty: Option<&syn::Type>,
    ) -> String {
        // Address-of over an RVALUE span producer is ill-formed C++ no matter
        // the callee — strip it even when the declared param is unknown
        // (map::Slice's two-param owner lookup can miss it while set::Slice
        // resolves).
        let trimmed = arg_cpp.trim_start();
        if let Some(rest) = trimmed.strip_prefix('&').map(str::trim_start)
            && ARG_ALREADY_SPAN_PREFIXES
                .iter()
                .any(|prefix| rest.starts_with(prefix))
        {
            return rest.to_string();
        }
        match arg_expected_ty.map(|t| self.peel_paren_group_type(t)) {
            Some(syn::Type::Reference(r)) if matches!(r.elem.as_ref(), syn::Type::Slice(_)) => {
                if ARG_ALREADY_SPAN_PREFIXES
                    .iter()
                    .any(|prefix| trimmed.starts_with(prefix))
                {
                    return arg_cpp;
                }
                if r.mutability.is_some() {
                    format!("rusty::as_mut_slice({})", arg_cpp)
                } else {
                    format!("rusty::as_slice({})", arg_cpp)
                }
            }
            _ => arg_cpp,
        }
    }

    /// --interface-traits (§ 3.2.9): if the expected parameter type is
    /// `&dyn Trait` / `&mut dyn Trait` (where Trait is a locally declared
    /// trait), and the argument is `&val` / `&mut val`, wrap the value
    /// with the matching `TraitAdapterRef<U>` / `TraitAdapterRefMut<U>`.
    /// `U` is left as `std::remove_cvref_t<decltype(val)>` so the C++
    /// compiler picks the right adapter specialization at instantiation
    /// time — codegen does not need to recover the concrete inner type
    /// from local-binding tables here.
    ///
    /// Returns `None` if the pattern doesn't match (no rewrite needed).
    pub(super) fn try_emit_interface_traits_dyn_ref_coercion(
        &self,
        arg: &syn::Expr,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let expected = expected_ty?;
        let syn::Type::Reference(ref_ty) = expected else {
            return None;
        };
        let is_mut = ref_ty.mutability.is_some();
        let syn::Type::TraitObject(to) = ref_ty.elem.as_ref() else {
            return None;
        };
        if to.bounds.len() != 1 {
            // Multi-bound dyn coercion is handled in Phase 6 (combined
            // interface synthesis). Skip here.
            return None;
        }
        let syn::TypeParamBound::Trait(tb) = to.bounds.first()? else {
            return None;
        };
        let trait_seg = tb.path.segments.last()?;
        let trait_name = trait_seg.ident.to_string();
        if !self
            .trait_declared_path_by_short_name
            .contains_key(&trait_name)
        {
            return None;
        }
        // Extract trait generic args (e.g., `["int32_t"]` for
        // `&dyn Container<i32>`). Empty for non-generic traits.
        let trait_args: Vec<String> = match &trait_seg.arguments {
            syn::PathArguments::AngleBracketed(args) => args
                .args
                .iter()
                .filter_map(|a| match a {
                    syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                    syn::GenericArgument::Const(c) => Some(self.emit_expr_to_string(c)),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };

        // Accept `&val` / `&mut val`, or a bare value if the existing
        // arg-shape lowering would have passed it by reference anyway.
        let inner_expr = match self.peel_paren_group_expr(arg) {
            syn::Expr::Reference(r) => self.peel_paren_group_expr(r.expr.as_ref()),
            other => other,
        };
        // Only wrap variables / field accesses / call results that name
        // a concrete value site. Skip literals and bare trait-object
        // sources to avoid double-wrapping.
        if !matches!(
            inner_expr,
            syn::Expr::Path(_) | syn::Expr::Field(_) | syn::Expr::Call(_) | syn::Expr::MethodCall(_)
        ) {
            return None;
        }

        let inner_cpp = self.emit_expr_to_string(inner_expr);
        let adapter_suffix = if is_mut { "AdapterRefMut" } else { "AdapterRef" };
        // The Adapter's template args are (trait_args..., decltype(value)).
        let adapter_args = if trait_args.is_empty() {
            format!("std::remove_cvref_t<decltype({})>", inner_cpp)
        } else {
            format!(
                "{}, std::remove_cvref_t<decltype({})>",
                trait_args.join(", "),
                inner_cpp
            )
        };
        Some(format!(
            "{}{}<{}>({})",
            trait_name, adapter_suffix, adapter_args, inner_cpp
        ))
    }

    pub(super) fn try_emit_auto_deref_arg_for_expected_reference(
        &self,
        arg: &syn::Expr,
        expected_ref_ty: &syn::Type,
    ) -> Option<String> {
        let expected_inner = self.expected_reference_inner_type(Some(expected_ref_ty))?;
        let expected_inner = self.peel_reference_paren_group_type(expected_inner);
        if matches!(
            expected_inner,
            syn::Type::Reference(_)
                | syn::Type::Ptr(_)
                | syn::Type::Slice(_)
                | syn::Type::Array(_)
                | syn::Type::Tuple(_)
        ) {
            return None;
        }
        let _expected_owner = match expected_inner {
            syn::Type::Path(tp) => {
                let seg = tp.path.segments.last()?;
                let owner = seg.ident.to_string();
                if Self::is_pointer_like_autoderef_owner_name(&owner) {
                    return None;
                }
                // Restrict unknown-type fallback to user-defined type-like
                // references (e.g. &Content). Primitive/string/slice borrows
                // should keep direct argument passing.
                if !owner
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_uppercase())
                {
                    return None;
                }
                owner
            }
            _ => return None,
        };
        if matches!(
            self.peel_paren_group_expr(arg),
            syn::Expr::Path(path)
                if path.path.segments.len() == 1 && path.path.segments[0].ident == "self"
        ) {
            return None;
        }
        let inferred_arg_ty = self.infer_simple_expr_type(arg);
        let arg_supports_autoderef = if let Some(arg_ty) = inferred_arg_ty.as_ref() {
            let arg_ty = self.peel_reference_paren_group_type(&arg_ty);
            match arg_ty {
                syn::Type::Path(arg_tp) => {
                    let arg_owner = arg_tp.path.segments.last()?.ident.to_string();
                    Self::is_pointer_like_autoderef_owner_name(&arg_owner)
                }
                syn::Type::Ptr(_) => true,
                _ => false,
            }
        } else {
            false
        };
        if !arg_supports_autoderef {
            // Fallback: some field/method expressions lose concrete wrapper types during
            // inference; preserve Rust auto-deref semantics conservatively here.
            if inferred_arg_ty.is_none()
                && matches!(
                    self.peel_paren_group_expr(arg),
                    syn::Expr::Field(_) | syn::Expr::MethodCall(_) | syn::Expr::Path(_)
                )
            {
                let arg_cpp = self.emit_expr_to_string(arg);
                return Some(format!("rusty::detail::deref_if_pointer_like({})", arg_cpp));
            }
            return None;
        }
        // Expected-reference argument sites must preserve lvalue identity.
        // Moving here turns `T&` calls into rvalues and breaks mutable-ref APIs.
        let arg_cpp = self.emit_expr_to_string(arg);
        Some(format!("rusty::detail::deref_if_pointer_like({})", arg_cpp))
    }

    pub(super) fn try_emit_expected_constructor_callable(
        &self,
        path: &syn::Path,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        if path.segments.len() < 2 {
            return None;
        }
        let method = path.segments.last()?.ident.to_string();
        if !matches!(method.as_str(), "new" | "new_" | "default" | "default_") {
            return None;
        }
        if !matches!(
            path.segments.last().map(|seg| &seg.arguments),
            Some(syn::PathArguments::None)
        ) {
            return None;
        }
        let expected_ty = expected_ty?;
        let param_count = self.extract_callable_param_count_from_type(expected_ty)?;
        if param_count != 0 {
            // Constructor path items (`Type::new` / `Type::default`) only
            // lower to zero-arg callables when the target callable expects 0 args.
            return None;
        }
        let ret_ty = self.extract_callable_return_type_from_type(expected_ty)?;
        if self.type_contains_unresolved_placeholder_like(&ret_ty)
            || self.type_contains_unbound_single_letter_generic(&ret_ty)
            || self.type_contains_in_scope_type_param(&ret_ty)
        {
            return None;
        }
        let ret_cpp = self.map_type(&ret_ty);
        if ret_cpp == "auto"
            || ret_cpp.contains("/* TODO")
            || type_string_has_auto_placeholder(&ret_cpp)
        {
            return None;
        }
        match method.as_str() {
            // C++ reserves `new`; Rust `new` methods are emitted as `new_`.
            "new" => Some(format!("[&]() {{ return {}::new_(); }}", ret_cpp)),
            "new_" => Some(format!("[&]() {{ return {}::new_(); }}", ret_cpp)),
            "default" | "default_" => Some(format!(
                "[&]() {{ return rusty::default_value<{}>(); }}",
                ret_cpp
            )),
            _ => None,
        }
    }

    pub(super) fn try_emit_tuple_struct_constructor_callable(&self, path: &syn::Path) -> Option<String> {
        if path.segments.len() != 1 {
            return None;
        }

        let raw_ident = path.segments.first()?.ident.to_string();
        let type_name = if raw_ident == "Self" {
            self.current_struct.clone()?
        } else {
            raw_ident.clone()
        };
        if raw_ident != "Self" && self.lookup_local_binding_type(&raw_ident).is_some() {
            return None;
        }

        let scoped_type_name = self.scoped_type_key(&type_name);
        let arity = self
            .tuple_struct_arities
            .get(&type_name)
            .copied()
            .or_else(|| self.tuple_struct_arities.get(&scoped_type_name).copied())?;
        if arity != 1 {
            return None;
        }

        let mut ctor_type = self.emit_path_to_string(path);
        if raw_ident == "Self" {
            if ctor_type == "Self" {
                ctor_type = type_name.clone();
            }
            if !ctor_type.contains('<') {
                if let Ok(owner_path) = syn::parse_str::<syn::Path>(&type_name) {
                    if let Some(recovered) =
                        self.recover_omitted_local_generic_type_args(&owner_path, &type_name)
                    {
                        ctor_type = recovered;
                    }
                }
            }
        }
        if let Some(recovered) = self.recover_omitted_local_generic_type_args(path, &ctor_type) {
            ctor_type = recovered;
        } else if let Some(last_seg) = path.segments.last()
            && let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments
        {
            let mapped_args: Vec<String> = args
                .args
                .iter()
                .filter_map(|arg| match arg {
                    syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                    syn::GenericArgument::Const(c) => Some(self.emit_expr_to_string(c)),
                    _ => None,
                })
                .collect();
            if !mapped_args.is_empty() {
                ctor_type = format!("{}<{}>", ctor_type, mapped_args.join(", "));
            }
        }

        Some(format!(
            "[](auto&& _v) {{ return {}(std::forward<decltype(_v)>(_v)); }}",
            ctor_type
        ))
    }

    pub(super) fn try_emit_data_enum_variant_constructor_callable_path(
        &self,
        path: &syn::Path,
    ) -> Option<String> {
        if path.segments.len() < 2 {
            return None;
        }
        if !matches!(
            path.segments.last().map(|seg| &seg.arguments),
            Some(syn::PathArguments::None)
        ) {
            return None;
        }
        let enum_name = path.segments.iter().nth_back(1)?.ident.to_string();
        let raw_variant_name = path.segments.last()?.ident.to_string();
        let variant_name = self.canonical_variant_name(&raw_variant_name).to_string();
        if !self.enum_has_variant_name(&enum_name, &raw_variant_name)
            && !self.enum_has_variant_name(&enum_name, &variant_name)
        {
            return None;
        }

        let unit_key_raw = format!("{}_{}", enum_name, raw_variant_name);
        let unit_key_canonical = format!("{}_{}", enum_name, variant_name);
        let is_unit_variant = self.data_enum_unit_variants.contains(&unit_key_raw)
            || self.data_enum_unit_variants.contains(&unit_key_canonical)
            || self.data_enum_unit_variants.iter().any(|known| {
                known
                    .rsplit("::")
                    .next()
                    .is_some_and(|tail| tail == unit_key_raw || tail == unit_key_canonical)
            });
        if is_unit_variant {
            return None;
        }

        let enum_path: syn::Path = {
            let segs: Vec<syn::PathSegment> = path
                .segments
                .iter()
                .take(path.segments.len() - 1)
                .cloned()
                .collect();
            let mut p = path.clone();
            p.segments = segs.into_iter().collect();
            p
        };
        let variant_ty = self.data_enum_variant_struct_type_name(&enum_path, &variant_name);
        Some(format!(
            "[](auto&& _v) {{ return {}{{std::forward<decltype(_v)>(_v)}}; }}",
            variant_ty
        ))
    }

    /// If `path` is `Type::method` where `Type` is a known local type and
    /// `method` is a lowercase instance method name, emit as forwarding lambda wrapper.
    pub(super) fn try_emit_method_reference_lambda(&self, path: &syn::Path) -> Option<String> {
        if path.segments.len() < 2 {
            return None;
        }
        let method_seg = path.segments.last()?;
        let type_seg = &path.segments[path.segments.len() - 2];
        let method_name = method_seg.ident.to_string();
        let type_name = type_seg.ident.to_string();
        let mut owner_path = path.clone();
        owner_path.segments = path
            .segments
            .iter()
            .take(path.segments.len() - 1)
            .cloned()
            .collect();
        let path_segments: Vec<String> = path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect();
        let trait_receiver_shape = self.trait_static_call_has_receiver_for_segments(&path_segments);
        if trait_receiver_shape == Some(false) {
            return None;
        }
        let receiver_shape = self
            .lookup_owner_method_has_receiver_from_owner_path(
                Some(&owner_path),
                &type_name,
                &method_name,
            )
            .or_else(|| self.lookup_owner_method_has_receiver(&type_name, &method_name))
            .or(trait_receiver_shape);
        // Heuristic: method names start with lowercase
        if !method_name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase())
        {
            return None;
        }
        // Check if Type is a known declared type
        let is_known_runtime_method_ref_type =
            matches!(type_name.as_str(), "String" | "Display" | "Debug");
        if !self.local_declared_types.contains(&type_name)
            && !self.is_local_type_name_in_scope(&type_name)
            && !is_known_runtime_method_ref_type
            && receiver_shape != Some(true)
        {
            return None;
        }
        // Known static methods should NOT be wrapped in an instance lambda —
        // they're function pointers, not member function references.
        // Return None to let them fall through to normal path emission
        // (e.g., `TestFlags::all` emits as `TestFlags::all`).
        let is_static = matches!(
            method_name.as_str(),
            "new"
                | "all"
                | "empty"
                | "from"
                | "from_bits"
                | "from_bits_retain"
                | "from_bits_truncate"
                | "from_name"
                | "default"
                | "default_"
                | "new_"
                | "new_const"
                | "from_utf8"
                | "from_utf8_lossy"
                | "from_utf8_unchecked"
                | "from_iter"
        );
        if is_static || receiver_shape == Some(false) {
            return None;
        }
        let escaped = escape_cpp_keyword(&method_name);
        Some(format!(
            "[](auto&& _f, auto&&... _args) -> decltype(auto) {{ return rusty::detail::deref_if_pointer_like(std::forward<decltype(_f)>(_f)).{}(std::forward<decltype(_args)>(_args)...); }}",
            escaped
        ))
    }

    pub(super) fn try_emit_fieldwise_clone_return_stmt(&self, method: &syn::ImplItemFn) -> Option<String> {
        let is_auto_derived_clone =
            method.sig.ident == "clone" && impl_method_is_automatically_derived(method);
        if !self.method_is_trivial_deref_self_clone(method) && !is_auto_derived_clone {
            return None;
        }

        let struct_name = self.current_struct.as_ref()?;
        let scoped_name = self.scoped_type_key(struct_name);
        if self.c_like_enum_types.contains(struct_name)
            || self.c_like_enum_types.contains(&scoped_name)
        {
            return Some("return self_;".to_string());
        }
        if is_auto_derived_clone
            && !self.struct_field_order.contains_key(struct_name)
            && !self.struct_field_order.contains_key(&scoped_name)
            && !self.tuple_struct_arities.contains_key(struct_name)
            && !self.tuple_struct_arities.contains_key(&scoped_name)
        {
            return None;
        }
        let field_order = self
            .struct_field_order
            .get(struct_name)
            .or_else(|| self.struct_field_order.get(&scoped_name))
            .cloned()
            .unwrap_or_default();
        if field_order.is_empty() {
            return Some("return {};".to_string());
        }
        let field_cpp_names = self
            .struct_field_cpp_names
            .get(struct_name)
            .or_else(|| self.struct_field_cpp_names.get(&scoped_name));
        let is_tuple_struct = self.tuple_struct_arities.contains_key(struct_name)
            || self.tuple_struct_arities.contains_key(&scoped_name);
        if is_tuple_struct {
            let elems: Vec<String> = field_order
                .iter()
                .map(|rust_name| {
                    let cpp_name = field_cpp_names
                        .and_then(|names| names.get(rust_name))
                        .cloned()
                        .unwrap_or_else(|| escape_cpp_keyword(rust_name));
                    if self.struct_field_is_reference(struct_name, rust_name) {
                        format!("this->{}", cpp_name)
                    } else {
                        format!("rusty::clone(this->{})", cpp_name)
                    }
                })
                .collect();
            return Some(format!("return {{{}}};", elems.join(", ")));
        }

        let field_inits: Vec<String> = field_order
            .iter()
            .map(|rust_name| {
                let cpp_name = field_cpp_names
                    .and_then(|names| names.get(rust_name))
                    .cloned()
                    .unwrap_or_else(|| escape_cpp_keyword(rust_name));
                if self.struct_field_is_reference(struct_name, rust_name) {
                    format!(".{} = this->{}", cpp_name, cpp_name)
                } else {
                    format!(".{} = rusty::clone(this->{})", cpp_name, cpp_name)
                }
            })
            .collect();
        Some(format!("return {{{}}};", field_inits.join(", ")))
    }

    pub(super) fn emit_block(&mut self, block: &syn::Block) {
        let profile_blocks = std::env::var_os("RUSTY_CPP_PROFILE_BLOCKS").is_some();
        let profile_this_block = profile_blocks && block.stmts.len() >= 8;
        let block_profile_start = std::time::Instant::now();
        let mut block_profile_mark = |label: &str| {
            if profile_this_block {
                eprintln!(
                    "[rusty-cpp][block-profile] stmts={} {}: {:.3}s",
                    block.stmts.len(),
                    label,
                    block_profile_start.elapsed().as_secs_f64()
                );
            }
        };
        block_profile_mark("start");
        // Pre-scan: find variables used multiple times (skip std::move for these)
        let multi_use = collect_multi_use_vars(&block.stmts);
        block_profile_mark("collect_multi_use_vars");
        let prev_multi_use = std::mem::replace(&mut self.multi_use_vars, multi_use);
        // Pre-scan: find variables that are reassigned (for reference rebinding detection)
        let reassigned = collect_reassigned_vars(&block.stmts);
        block_profile_mark("collect_reassigned_vars");
        let deref_assigned = collect_deref_assigned_vars(&block.stmts);
        block_profile_mark("collect_deref_assigned_vars");
        let consuming =
            self.collect_consuming_method_receiver_vars_with_signature_hints(&block.stmts);
        block_profile_mark("collect_consuming_method_receiver_vars_with_signature_hints");
        let mutable_pointer_aliased = collect_mutable_pointer_aliased_locals(&block.stmts);
        block_profile_mark("collect_mutable_pointer_aliased_locals");
        let repeat_hints = collect_repeat_element_type_hints(&block.stmts);
        block_profile_mark("collect_repeat_element_type_hints");
        let mut placeholder_hints = collect_local_generic_placeholder_hints(&block.stmts);
        block_profile_mark("collect_local_generic_placeholder_hints");
        // Large expanded test functions can contain massive generated blocks.
        // Keep default hint augmentation for normal blocks, but skip the
        // heaviest passes for very large blocks to avoid pathological slowdown.
        let enable_block_hint_augmentation = block.stmts.len() <= 128;
        if enable_block_hint_augmentation {
            // Seed pre-scan local type knowledge (from earlier locals in this block)
            // so placeholder augmentation can resolve method-return shapes like:
            // `let heap = Heap::default(); cell.set(Box::new(heap.new_pebble(...)))`.
            let pre_scan_known_local_types =
                self.collect_pre_scan_known_local_type_hints(&block.stmts);
            block_profile_mark("collect_pre_scan_known_local_type_hints");
            let pushed_pre_scan_known_local_scope = if pre_scan_known_local_types.is_empty() {
                false
            } else {
                let pre_scan_scope: HashMap<String, Option<syn::Type>> = pre_scan_known_local_types
                    .into_iter()
                    .map(|(name, ty)| (name, Some(ty)))
                    .collect();
                self.local_bindings.push(pre_scan_scope);
                true
            };
            self.augment_local_generic_placeholder_hints_from_function_calls(
                &block.stmts,
                &mut placeholder_hints,
            );
            block_profile_mark("augment_local_generic_placeholder_hints_from_function_calls");
            self.augment_uninitialized_local_type_hints_from_usage(
                &block.stmts,
                &mut placeholder_hints,
            );
            block_profile_mark("augment_uninitialized_local_type_hints_from_usage");
            // Solver-backed pass for owner locals whose element type the
            // heuristic scan above can't follow (pushes nested in fold/all
            // reducer closures, tuple-of-clone elements). Fills only
            // still-unresolved bindings.
            self.augment_owner_local_type_hints_from_solver(
                &block.stmts,
                &mut placeholder_hints,
            );
            block_profile_mark("augment_owner_local_type_hints_from_solver");
            // Backward pass: un-annotated `let x = ....collect()` whose target
            // type only the consuming struct field pins
            // (`Self { iter: x.into_iter() }`). Fill-only; see the pass doc.
            self.augment_collect_local_type_hints_from_struct_literal_consumption(
                &block.stmts,
                &mut placeholder_hints,
            );
            block_profile_mark(
                "augment_collect_local_type_hints_from_struct_literal_consumption",
            );
            // Two-parameter generic owners (HashMap, BTreeMap) need
            // both K and V from their `insert(K, V)` usage. The
            // single-inner pipeline above handles only Vec&lt;T&gt;-shaped
            // owners; this pass writes fully resolved owner types
            // (`HashMap&lt;K, V&gt;`) directly for the multi-parameter
            // shapes. See `augment_two_param_local_type_hints_from_
            // usage`.
            self.augment_two_param_local_type_hints_from_usage(
                &block.stmts,
                &mut placeholder_hints,
            );
            block_profile_mark("augment_two_param_local_type_hints_from_usage");
            self.augment_option_none_placeholder_hints_from_return_context(
                &block.stmts,
                &mut placeholder_hints,
            );
            block_profile_mark("augment_option_none_placeholder_hints_from_return_context");
            self.augment_mut_unsuffixed_int_seed_hints_from_option_return_context(
                &block.stmts,
                &mut placeholder_hints,
            );
            block_profile_mark("augment_mut_unsuffixed_int_seed_hints_from_option_return_context");
            if let Some(fallback_inner_ty) =
                self.collect_oncecell_fallback_inner_type_hint(&block.stmts)
            {
                placeholder_hints.insert(
                    ONCECELL_FALLBACK_INNER_HINT_KEY.to_string(),
                    fallback_inner_ty,
                );
            }
            block_profile_mark("collect_oncecell_fallback_inner_type_hint");
            if pushed_pre_scan_known_local_scope {
                self.local_bindings.pop();
            }
        }
        let prev = std::mem::replace(&mut self.reassigned_vars, reassigned);
        let prev_deref_assigned =
            std::mem::replace(&mut self.deref_assigned_vars, deref_assigned);
        let prev_consuming = std::mem::replace(&mut self.consuming_method_receiver_vars, consuming);
        let prev_mutable_pointer_aliased = std::mem::replace(
            &mut self.mutable_pointer_aliased_vars,
            mutable_pointer_aliased,
        );
        let prev_repeat_hints = std::mem::replace(&mut self.repeat_elem_type_hints, repeat_hints);
        if std::env::var_os("RUSTY_CPP_DEBUG_PLACEHOLDER_HINTS").is_some()
            && !placeholder_hints.is_empty()
        {
            let mut hint_names: Vec<String> = placeholder_hints.keys().cloned().collect();
            hint_names.sort();
            eprintln!(
                "[rusty-cpp][placeholder-hints] stmts={} count={}",
                block.stmts.len(),
                hint_names.len()
            );
            for name in hint_names {
                if let Some(ty) = placeholder_hints.get(&name) {
                    eprintln!("  {} => {}", name, quote::quote!(#ty));
                }
            }
        }
        let decltype_element_overrides =
            self.collect_collection_decltype_element_overrides(&block.stmts, &placeholder_hints);
        self.local_placeholder_type_hints.push(placeholder_hints);
        self.int_literal_usage_type_hints
            .push(self.collect_int_literal_usage_type_hints(&block.stmts));
        self.collection_ctor_usage_type_hints
            .push(self.collect_collection_ctor_usage_type_hints(&block.stmts));
        self.collection_decltype_element_overrides
            .push(decltype_element_overrides);
        self.local_bindings.push(HashMap::new());
        self.local_shadowed_binding_types.push(HashMap::new());
        self.local_cpp_bindings.push(HashMap::new());
        self.local_cpp_names_used.push(HashSet::new());
        self.local_const_bindings.push(HashMap::new());
        self.local_item_const_names.push(HashSet::new());
        self.local_reference_bindings.push(HashSet::new());
        self.rebind_reference_pointer_bindings.push(HashSet::new());
        self.delayed_init_locals.push(HashSet::new());
        self.pending_uninit_let_locals.push(HashMap::new());
        self.emit_rebind_pointer_aliases_for_reassigned_params();

        // Inject for-loop variable names into the body scope so that
        // shadowing inside the loop body generates _shadow1 names.
        if !self.pending_loop_var_bindings.is_empty() {
            let loop_vars = std::mem::take(&mut self.pending_loop_var_bindings);
            if let Some(scope) = self.local_cpp_bindings.last_mut() {
                for name in &loop_vars {
                    // Register the ESCAPED C++ name: the loop head / destructure
                    // prelude declare keyword-named bindings escaped (`new` →
                    // `new_`), so a verbatim registration desyncs every body
                    // use (`std::move(new)` — parse error).
                    scope.insert(name.clone(), escape_cpp_keyword(name));
                }
            }
            if let Some(used) = self.local_cpp_names_used.last_mut() {
                for name in &loop_vars {
                    used.insert(escape_cpp_keyword(name));
                }
            }
        }
        if !self.pending_loop_var_binding_types.is_empty() {
            let loop_var_types = std::mem::take(&mut self.pending_loop_var_binding_types);
            if let Some(scope) = self.local_bindings.last_mut() {
                for (name, ty) in loop_var_types {
                    scope.insert(name, Some(ty));
                }
            }
        }

        let local_functions: HashSet<String> = block
            .stmts
            .iter()
            .filter_map(|stmt| match stmt {
                syn::Stmt::Item(syn::Item::Fn(f)) => Some(f.sig.ident.to_string()),
                _ => None,
            })
            .collect();
        let mut local_types: HashSet<String> = block
            .stmts
            .iter()
            .filter_map(|stmt| match stmt {
                syn::Stmt::Item(syn::Item::Struct(s)) => Some(s.ident.to_string()),
                syn::Stmt::Item(syn::Item::Enum(e)) => Some(e.ident.to_string()),
                syn::Stmt::Item(syn::Item::Type(t)) => Some(t.ident.to_string()),
                _ => None,
            })
            .collect();
        for hoisted in &self.hoisted_local_type_name_scopes {
            local_types.extend(hoisted.iter().cloned());
        }
        let (
            local_impl_overrides,
            local_drop_overrides,
            local_operator_overrides,
            local_inherent_method_overrides,
        ) = self.collect_local_impl_overrides(&block.stmts, &local_types);
        block_profile_mark("collect_local_impl_overrides");
        self.local_function_bindings.push(local_functions);
        self.local_type_bindings.push(local_types);
        self.local_manually_drop_bindings.push(HashSet::new());
        self.recursive_nested_fns_in_scope.push(HashSet::new());
        self.block_depth += 1;

        // Block-local impls are merged into local type declarations in this scope.
        // Apply temporary overrides and restore them after the block is emitted.
        let mut prev_impl_overrides: Vec<(String, Option<Vec<syn::ImplItem>>)> = Vec::new();
        for (type_name, impl_items) in local_impl_overrides {
            let prev = self.impl_blocks.insert(type_name.clone(), impl_items);
            prev_impl_overrides.push((type_name, prev));
        }
        let mut inserted_drop_overrides: Vec<((String, String), bool)> = Vec::new();
        for drop_key in local_drop_overrides {
            let inserted = self.drop_trait_methods.insert(drop_key.clone());
            inserted_drop_overrides.push((drop_key, inserted));
        }
        let mut prev_operator_overrides: Vec<((String, String), Option<String>)> = Vec::new();
        for (op_key, op_value) in local_operator_overrides {
            let prev = self.operator_renames.insert(op_key.clone(), op_value);
            prev_operator_overrides.push((op_key, prev));
        }
        let mut prev_inherent_overrides: Vec<(String, Option<HashSet<String>>)> = Vec::new();
        for (type_name, method_names) in local_inherent_method_overrides {
            let prev = self
                .inherent_impl_method_names
                .insert(type_name.clone(), method_names);
            prev_inherent_overrides.push((type_name, prev));
        }

        let stmts = &block.stmts;
        for stmt in stmts {
            if let syn::Stmt::Item(syn::Item::Impl(impl_block)) = stmt {
                self.record_local_impl_method_metadata(impl_block);
            }
        }
        block_profile_mark("record_local_impl_method_metadata");
        // Rust block-scoped items are usable across the whole block regardless of
        // lexical order. Emit local type/const/static items first (in source order)
        // so nested/local functions and local type methods can reference them.
        for stmt in stmts {
            if matches!(
                stmt,
                syn::Stmt::Item(
                    syn::Item::Struct(_)
                        | syn::Item::Enum(_)
                        | syn::Item::Type(_)
                        | syn::Item::Use(_)
                        | syn::Item::Const(_)
                        | syn::Item::Static(_)
                )
            ) {
                self.emit_stmt(stmt, false);
            }
        }
        block_profile_mark("emit_hoisted_type_use_const_static_items");
        // Rust block-scoped `fn` items are also usable across the whole block.
        // Emit them before non-item statements so earlier call sites still resolve.
        for stmt in stmts {
            if matches!(stmt, syn::Stmt::Item(syn::Item::Fn(_))) {
                self.emit_stmt(stmt, false);
            }
        }
        block_profile_mark("emit_hoisted_fn_items");

        let non_hoisted_stmts: Vec<&syn::Stmt> = stmts
            .iter()
            .filter(|stmt| {
                !matches!(
                    stmt,
                    syn::Stmt::Item(
                        syn::Item::Fn(_)
                            | syn::Item::Struct(_)
                            | syn::Item::Enum(_)
                            | syn::Item::Type(_)
                            | syn::Item::Use(_)
                            | syn::Item::Const(_)
                            | syn::Item::Static(_)
                    )
                )
            })
            .collect();
        let len = non_hoisted_stmts.len();
        for (i, stmt) in non_hoisted_stmts.iter().enumerate() {
            let is_last = i == len - 1;
            let stmt_profile_start = if profile_this_block {
                Some(std::time::Instant::now())
            } else {
                None
            };
            self.emit_stmt(stmt, is_last);
            if let Some(start) = stmt_profile_start {
                let elapsed = start.elapsed().as_secs_f64();
                if elapsed >= 0.050 {
                    let stmt_kind = match stmt {
                        syn::Stmt::Local(_) => "local",
                        syn::Stmt::Expr(expr, semi) => {
                            if semi.is_some() {
                                "expr_semi"
                            } else {
                                match self.peel_paren_group_expr(expr) {
                                    syn::Expr::Call(_) => "expr_call",
                                    syn::Expr::MethodCall(_) => "expr_method_call",
                                    syn::Expr::Match(_) => "expr_match",
                                    syn::Expr::If(_) => "expr_if",
                                    syn::Expr::Block(_) => "expr_block",
                                    syn::Expr::Assign(_) => "expr_assign",
                                    syn::Expr::While(_) => "expr_while",
                                    syn::Expr::ForLoop(_) => "expr_for",
                                    syn::Expr::Loop(_) => "expr_loop",
                                    _ => "expr_other",
                                }
                            }
                        }
                        syn::Stmt::Item(item) => match item {
                            syn::Item::Fn(_) => "item_fn",
                            syn::Item::Struct(_) => "item_struct",
                            syn::Item::Enum(_) => "item_enum",
                            syn::Item::Type(_) => "item_type",
                            syn::Item::Use(_) => "item_use",
                            syn::Item::Const(_) => "item_const",
                            syn::Item::Static(_) => "item_static",
                            syn::Item::Impl(_) => "item_impl",
                            _ => "item_other",
                        },
                        syn::Stmt::Macro(_) => "macro",
                    };
                    eprintln!(
                        "[rusty-cpp][block-profile] stmts={} non_hoisted stmt_idx={} kind={} took {:.3}s snippet={}",
                        block.stmts.len(),
                        i,
                        stmt_kind,
                        elapsed,
                        {
                            let stmt_snippet = stmt.to_token_stream().to_string();
                            let mut truncated = stmt_snippet.chars().take(120).collect::<String>();
                            if stmt_snippet.chars().count() > 120 {
                                truncated.push_str("...");
                            }
                            truncated
                        }
                    );
                }
            }
        }
        block_profile_mark("emit_non_hoisted_stmts");

        for (op_key, prev) in prev_operator_overrides {
            if let Some(prev_value) = prev {
                self.operator_renames.insert(op_key, prev_value);
            } else {
                self.operator_renames.remove(&op_key);
            }
        }
        for (type_name, prev) in prev_inherent_overrides {
            if let Some(prev_names) = prev {
                self.inherent_impl_method_names
                    .insert(type_name, prev_names);
            } else {
                self.inherent_impl_method_names.remove(&type_name);
            }
        }
        for (drop_key, inserted) in inserted_drop_overrides {
            if inserted {
                self.drop_trait_methods.remove(&drop_key);
            }
        }
        for (type_name, prev) in prev_impl_overrides {
            if let Some(prev_items) = prev {
                self.impl_blocks.insert(type_name, prev_items);
            } else {
                self.impl_blocks.remove(&type_name);
            }
        }

        self.block_depth -= 1;
        self.local_bindings.pop();
        self.local_shadowed_binding_types.pop();
        self.local_cpp_bindings.pop();
        self.local_cpp_names_used.pop();
        self.local_const_bindings.pop();
        self.local_item_const_names.pop();
        self.local_reference_bindings.pop();
        self.rebind_reference_pointer_bindings.pop();
        self.delayed_init_locals.pop();
        self.pending_uninit_let_locals.pop();
        self.local_function_bindings.pop();
        self.local_type_bindings.pop();
        self.local_manually_drop_bindings.pop();
        self.recursive_nested_fns_in_scope.pop();
        self.local_placeholder_type_hints.pop();
        self.int_literal_usage_type_hints.pop();
        self.collection_ctor_usage_type_hints.pop();
        self.collection_decltype_element_overrides.pop();
        self.reassigned_vars = prev;
        self.deref_assigned_vars = prev_deref_assigned;
        self.consuming_method_receiver_vars = prev_consuming;
        self.mutable_pointer_aliased_vars = prev_mutable_pointer_aliased;
        self.repeat_elem_type_hints = prev_repeat_hints;
        self.multi_use_vars = prev_multi_use;
        block_profile_mark("done");
    }

    pub(super) fn try_emit_statement_compound_assign_without_unit_wrapper(
        &mut self,
        expr: &syn::Expr,
    ) -> bool {
        let syn::Expr::Binary(bin) = self.peel_paren_group_expr(expr) else {
            return false;
        };
        if !Self::is_compound_assign_binop(&bin.op) {
            return false;
        }
        let left = self.autoderef_compound_assign_lhs_if_needed(
            &bin.left,
            self.emit_expr_to_string(&bin.left),
        );
        let op = self.emit_binop(&bin.op);
        let right = self.emit_expr_to_string(&bin.right);
        self.writeln(&format!("{} {} {};", left, op, right));
        true
    }

    /// Try to emit an expression as a control flow statement.
    /// Returns true if it was handled, false if it should go through emit_expr_to_string.
    pub(super) fn try_emit_control_flow(&mut self, expr: &syn::Expr, preserve_tail_returns: bool) -> bool {
        match expr {
            syn::Expr::If(if_expr) => {
                self.emit_control_flow_with_return_scope(preserve_tail_returns, |this| {
                    this.emit_if(if_expr);
                });
                true
            }
            syn::Expr::While(while_expr) => {
                self.emit_control_flow_with_return_scope(preserve_tail_returns, |this| {
                    this.emit_while(while_expr);
                });
                true
            }
            syn::Expr::Loop(loop_expr) => {
                self.emit_control_flow_with_return_scope(preserve_tail_returns, |this| {
                    this.emit_loop(loop_expr);
                });
                true
            }
            syn::Expr::ForLoop(for_expr) => {
                self.emit_control_flow_with_return_scope(preserve_tail_returns, |this| {
                    this.emit_for_loop(for_expr);
                });
                true
            }
            syn::Expr::Block(block_expr) => {
                self.writeln("{");
                self.indent += 1;
                self.emit_control_flow_with_return_scope(preserve_tail_returns, |this| {
                    this.emit_block(&block_expr.block);
                });
                self.indent -= 1;
                self.writeln("}");
                true
            }
            syn::Expr::Unsafe(unsafe_expr) => {
                self.writeln("// @unsafe");
                self.writeln("{");
                self.indent += 1;
                self.emit_control_flow_with_return_scope(preserve_tail_returns, |this| {
                    this.emit_block(&unsafe_expr.block);
                });
                self.indent -= 1;
                self.writeln("}");
                true
            }
            syn::Expr::Match(match_expr) => {
                self.emit_control_flow_with_return_scope(preserve_tail_returns, |this| {
                    this.emit_match(match_expr);
                });
                true
            }
            _ => false,
        }
    }

    pub(super) fn emit_match_visit_scrutinee(
        &self,
        expr: &syn::Expr,
        variant_ctx: Option<&VariantTypeContext>,
    ) -> String {
        let expr = self.peel_paren_group_expr(expr);
        if let syn::Expr::Reference(reference) = expr {
            if !self.is_expr_raw_pointer_like(&reference.expr) {
                return self.emit_expr_to_string_with_variant_ctx(&reference.expr, variant_ctx);
            }
        }
        // Reference-Ok `?` scrutinee: the try macro's statement-expr value
        // decays the reference (deleted Event copy) — take the
        // pointer-valued expansion; deref_if_pointer absorbs it.
        if let syn::Expr::Try(try_expr) = expr
            && self.try_operand_ok_type_is_reference(&try_expr.expr)
            && let Some(ptr_form) = self.emit_try_expr_reference_pointer(expr)
        {
            return ptr_form;
        }
        self.emit_expr_to_string_with_variant_ctx(expr, variant_ctx)
    }

    pub(super) fn try_emit_runtime_tuple_match_stmt(&mut self, match_expr: &syn::ExprMatch) -> bool {
        let tuple_expr = self.peel_paren_group_expr(&match_expr.expr);
        let syn::Expr::Tuple(tuple_scrutinee) = tuple_expr else {
            return false;
        };
        if tuple_scrutinee.elems.is_empty() {
            return false;
        }

        let tuple_value_names: Vec<String> = (0..tuple_scrutinee.elems.len())
            .map(|idx| format!("_m{}", idx))
            .collect();
        let tuple_value_exprs: Vec<String> = tuple_scrutinee
            .elems
            .iter()
            .map(|elem| self.emit_match_visit_scrutinee(elem, None))
            .collect();

        let mut arm_plans = Vec::with_capacity(match_expr.arms.len());
        for arm in &match_expr.arms {
            let mut arm_bindings = Vec::new();
            let mut binding_map = HashMap::new();
            let Some(arm_condition) = self
                .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                    &arm.pat,
                    "_m_tuple",
                    &mut arm_bindings,
                    &mut binding_map,
                    None,
                )
            else {
                return false;
            };
            arm_plans.push((
                arm_condition.unwrap_or_else(|| "true".to_string()),
                arm_bindings,
                binding_map,
            ));
        }

        self.writeln("{");
        self.indent += 1;
        for (idx, elem_value) in tuple_value_exprs.iter().enumerate() {
            let elem_name = &tuple_value_names[idx];
            self.writeln(&format!("auto&& {} = {};", elem_name, elem_value));
        }
        self.writeln(&format!(
            "auto _m_tuple = std::forward_as_tuple({});",
            tuple_value_names.join(", ")
        ));
        self.writeln("bool _m_matched = false;");
        for (arm, (arm_condition, arm_bindings, binding_map)) in
            match_expr.arms.iter().zip(arm_plans.into_iter())
        {
            self.writeln(&format!("if (!_m_matched && ({})) {{", arm_condition));
            self.indent += 1;
            for binding in arm_bindings {
                self.writeln(&binding);
            }
            let pushed_binding_scope = self.push_local_cpp_binding_scope(&binding_map);
            if let Some((_, guard)) = &arm.guard {
                let guard_condition = self.emit_expr_to_string(guard);
                self.writeln(&format!("if ({}) {{", guard_condition));
                self.indent += 1;
                self.emit_arm_body(&arm.body);
                self.writeln("_m_matched = true;");
                self.indent -= 1;
                self.writeln("}");
            } else {
                self.emit_arm_body(&arm.body);
                self.writeln("_m_matched = true;");
            }
            self.pop_local_cpp_binding_scope(pushed_binding_scope);
            self.indent -= 1;
            self.writeln("}");
        }
        self.indent -= 1;
        self.writeln("}");
        true
    }

    pub(super) fn try_emit_runtime_match_stmt(
        &mut self,
        match_expr: &syn::ExprMatch,
        variant_ctx: Option<&VariantTypeContext>,
    ) -> bool {
        let match_scrutinee_ty = self
            .infer_simple_expr_type(&match_expr.expr)
            .or_else(|| self.infer_local_binding_type_from_initializer(&match_expr.expr));
        let payload_source = if self.runtime_match_scrutinee_borrows_payload(&match_expr.expr) {
            "std::as_const(_m)"
        } else {
            "_m"
        };
        let mut arm_plans: Vec<(
            String,
            Vec<String>,
            Option<String>,
            Vec<String>,
            Vec<String>,
            HashMap<String, String>,
            HashMap<String, syn::Type>,
        )> = Vec::with_capacity(match_expr.arms.len());

        for (idx, arm) in match_expr.arms.iter().enumerate() {
            let mut arm_pre_lines = Vec::new();
            let mut arm_payload_condition = None;
            let mut arm_payload_setup_lines = Vec::new();
            let mut arm_binding_lines = Vec::new();
            let mut arm_binding_map = HashMap::new();
            let arm_condition = match &arm.pat {
                syn::Pat::TupleStruct(ts) => {
                    let Some((cond_method, unwrap_method)) =
                        self.runtime_tuple_struct_match_methods(&ts.path, variant_ctx)
                    else {
                        return false;
                    };
                    if ts.elems.len() != 1 {
                        return false;
                    }
                    let matched_value = format!("_mv{}", idx);
                    let mut binding_stmts = Vec::new();
                    let Some(payload_condition) = self
                        .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                            &ts.elems[0],
                            &matched_value,
                            &mut binding_stmts,
                            &mut arm_binding_map,
                            variant_ctx,
                        )
                    else {
                        return false;
                    };
                    let needs_payload_materialization = !binding_stmts.is_empty()
                        || payload_condition.is_some()
                        || arm.guard.is_some();
                    if needs_payload_materialization {
                        let payload_value_source =
                            if payload_condition.is_some() || arm.guard.is_some() {
                                "std::as_const(_m)"
                            } else {
                                payload_source
                            };
                        arm_payload_setup_lines.push(format!(
                            "auto&& {} = {}.{}();",
                            matched_value, payload_value_source, unwrap_method
                        ));
                    }
                    arm_binding_lines.extend(binding_stmts);
                    arm_payload_condition = payload_condition;
                    format!("_m.{}()", cond_method)
                }
                syn::Pat::Path(pp) => {
                    let Some(cond_method) =
                        self.runtime_path_match_condition_method(&pp.path, variant_ctx)
                    else {
                        return false;
                    };
                    format!("_m.{}()", cond_method)
                }
                syn::Pat::Wild(_) => "true".to_string(),
                syn::Pat::Ident(pi) => {
                    if pi.subpat.is_some() {
                        let Some(cond) = self
                            .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                                &arm.pat,
                                "_m",
                                &mut arm_binding_lines,
                                &mut arm_binding_map,
                                variant_ctx,
                            )
                        else {
                            return false;
                        };
                        cond.unwrap_or_else(|| "true".to_string())
                    } else if let Some(cond_method) =
                        self.runtime_ident_match_condition_method(pi, variant_ctx)
                    {
                        format!("_m.{}()", cond_method)
                    } else {
                        if pi.ident != "_" {
                            let rust_name = pi.ident.to_string();
                            let cpp_name = self
                                .lookup_local_binding_cpp_name(&rust_name)
                                .unwrap_or_else(|| {
                                    self.fallback_pattern_binding_cpp_name(
                                        &rust_name,
                                        &arm_binding_map,
                                    )
                                });
                            arm_binding_map.insert(rust_name, cpp_name.clone());
                            arm_binding_lines.push(format!("const auto& {} = _m;", cpp_name));
                        }
                        "true".to_string()
                    }
                }
                syn::Pat::Or(or_pat) => {
                    let mut has_wild = false;
                    let mut tuple_methods: Option<(&'static str, &'static str)> = None;
                    let mut tuple_payload_conditions: Vec<Option<String>> = Vec::new();
                    let mut path_conditions = Vec::new();
                    let tuple_matched_value = format!("_m_orv{}", idx);
                    let arm_match_var = format!("_m_or_match{}", idx);

                    for case in &or_pat.cases {
                        match case {
                            syn::Pat::TupleStruct(ts_case) => {
                                let Some((cond_method, unwrap_method)) = self
                                    .runtime_tuple_struct_match_methods(&ts_case.path, variant_ctx)
                                else {
                                    return false;
                                };
                                if ts_case.elems.len() != 1 {
                                    return false;
                                }
                                if let Some((existing_cond, existing_unwrap)) = tuple_methods {
                                    if existing_cond != cond_method
                                        || existing_unwrap != unwrap_method
                                    {
                                        return false;
                                    }
                                } else {
                                    tuple_methods = Some((cond_method, unwrap_method));
                                }

                                let mut case_binding_stmts = Vec::new();
                                let mut case_binding_map = HashMap::new();
                                let Some(case_condition) = self
                                    .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                                        &ts_case.elems[0],
                                        &tuple_matched_value,
                                        &mut case_binding_stmts,
                                        &mut case_binding_map,
                                        variant_ctx,
                                    )
                                else {
                                    return false;
                                };
                                if case_binding_stmts.is_empty() {
                                    tuple_payload_conditions.push(case_condition);
                                } else {
                                    // OR arms with runtime payload bindings require branch-scoped
                                    // binding synthesis to keep names/guards coherent.
                                    return false;
                                }
                            }
                            syn::Pat::Path(pp_case) => {
                                let Some(cond_method) = self.runtime_path_match_condition_method(
                                    &pp_case.path,
                                    variant_ctx,
                                ) else {
                                    return false;
                                };
                                path_conditions.push(format!("_m.{}()", cond_method));
                            }
                            syn::Pat::Wild(_) => {
                                has_wild = true;
                            }
                            syn::Pat::Ident(pi) if pi.ident == "_" => {
                                has_wild = true;
                            }
                            syn::Pat::Ident(pi) => {
                                let Some(cond_method) =
                                    self.runtime_ident_match_condition_method(pi, variant_ctx)
                                else {
                                    return false;
                                };
                                path_conditions.push(format!("_m.{}()", cond_method));
                            }
                            _ => return false,
                        }
                    }

                    if has_wild {
                        "true".to_string()
                    } else {
                        arm_pre_lines.push(format!("bool {} = false;", arm_match_var));
                        if let Some((cond_method, unwrap_method)) = tuple_methods {
                            let tuple_case_condition = if tuple_payload_conditions.is_empty()
                                || tuple_payload_conditions.iter().any(|cond| cond.is_none())
                            {
                                "true".to_string()
                            } else {
                                tuple_payload_conditions
                                    .iter()
                                    .filter_map(|cond| cond.clone())
                                    .collect::<Vec<_>>()
                                    .join(" || ")
                            };
                            arm_pre_lines.push(format!(
                                "if (_m.{}()) {{ auto&& {} = std::as_const(_m).{}(); {} = ({}); }}",
                                cond_method,
                                tuple_matched_value,
                                unwrap_method,
                                arm_match_var,
                                tuple_case_condition
                            ));
                        }
                        if !path_conditions.is_empty() {
                            arm_pre_lines.push(format!(
                                "if (!{} && ({})) {{ {} = true; }}",
                                arm_match_var,
                                path_conditions.join(" || "),
                                arm_match_var
                            ));
                        }
                        if tuple_methods.is_none() && path_conditions.is_empty() {
                            return false;
                        }
                        arm_match_var
                    }
                }
                _ => return false,
            };
            let mut arm_binding_types = HashMap::new();
            if let Some(scrutinee_ty) = match_scrutinee_ty.as_ref() {
                self.bind_pattern_types_into_env(&arm.pat, scrutinee_ty, &mut arm_binding_types);
                arm_binding_types.retain(|name, _| arm_binding_map.contains_key(name));
            }
            arm_plans.push((
                arm_condition,
                arm_pre_lines,
                arm_payload_condition,
                arm_payload_setup_lines,
                arm_binding_lines,
                arm_binding_map,
                arm_binding_types,
            ));
        }

        // Reference-Ok `?` scrutinee: the try macro's statement-expr value
        // decays the reference (deleted Event copy) — take the
        // pointer-valued expansion; deref_if_pointer absorbs it.
        let scrutinee = if let syn::Expr::Try(try_expr) =
            self.peel_paren_group_expr(&match_expr.expr)
            && self.try_operand_ok_type_is_reference(&try_expr.expr)
            && let Some(ptr_form) = self.emit_try_expr_reference_pointer(&match_expr.expr)
        {
            ptr_form
        } else {
            self.emit_expr_to_string(&match_expr.expr)
        };
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!("auto&& _m = {};", scrutinee));
        self.writeln("bool _m_matched = false;");

        for (
            arm,
            (
                arm_condition,
                arm_pre_lines,
                arm_payload_condition,
                arm_payload_setup_lines,
                arm_binding_lines,
                arm_binding_map,
                arm_binding_types,
            ),
        ) in match_expr.arms.iter().zip(arm_plans.into_iter())
        {
            self.writeln("if (!_m_matched) {");
            self.indent += 1;
            for pre_line in arm_pre_lines {
                self.writeln(&pre_line);
            }
            self.writeln(&format!("if ({}) {{", arm_condition));
            self.indent += 1;
            for payload_setup_line in arm_payload_setup_lines {
                self.writeln(&payload_setup_line);
            }
            if let Some(payload_condition) = &arm_payload_condition {
                self.writeln(&format!("if ({}) {{", payload_condition));
                self.indent += 1;
            }
            for binding_line in arm_binding_lines {
                self.writeln(&binding_line);
            }
            let pushed_binding_scope = self.push_local_cpp_binding_scope_with_types(
                &arm_binding_map,
                Some(&arm_binding_types),
            );

            if let Some((_, guard)) = &arm.guard {
                let guard_condition = self.emit_expr_to_string(guard);
                self.writeln(&format!("if ({}) {{", guard_condition));
                self.indent += 1;
                self.emit_arm_body(&arm.body);
                self.writeln("_m_matched = true;");
                self.indent -= 1;
                self.writeln("}");
            } else {
                self.emit_arm_body(&arm.body);
                self.writeln("_m_matched = true;");
            }
            self.pop_local_cpp_binding_scope(pushed_binding_scope);

            if arm_payload_condition.is_some() {
                self.indent -= 1;
                self.writeln("}");
            }
            self.indent -= 1;
            self.writeln("}");
            self.indent -= 1;
            self.writeln("}");
        }

        self.indent -= 1;
        self.writeln("}");
        true
    }

    pub(super) fn try_emit_binding_tuple_match(&mut self, match_expr: &syn::ExprMatch) -> bool {
        let syn::Expr::Tuple(tuple_scrutinee) = match_expr.expr.as_ref() else {
            return false;
        };

        let arity = tuple_scrutinee.elems.len();
        if !match_expr
            .arms
            .iter()
            .all(|arm| self.is_binding_only_tuple_arm_pattern(&arm.pat, arity))
        {
            return false;
        }

        let tuple_expected_ty =
            self.infer_expected_type_from_tuple_elements(&tuple_scrutinee.elems);
        let tuple_has_slice_range_reference = tuple_scrutinee
            .elems
            .iter()
            .any(|elem| self.is_reference_to_slice_range_index_expr(elem));
        let match_scrutinee_ty = self
            .infer_simple_expr_type(&match_expr.expr)
            .or_else(|| self.infer_local_binding_type_from_initializer(&match_expr.expr));

        self.writeln("{");
        self.indent += 1;

        let mut tuple_elem_names = Vec::new();
        for (idx, elem) in tuple_scrutinee.elems.iter().enumerate() {
            let elem_name = format!("_m{}", idx);
            let peer_expr = self.binding_tuple_peer_expr_for_index(tuple_scrutinee, idx);
            let tuple_elem_expected_from_peer = if tuple_expected_ty.is_none() {
                self.infer_binding_tuple_element_expected_type_from_peer(elem, peer_expr)
            } else {
                None
            };
            let tuple_elem_expected_ty = tuple_expected_ty
                .as_ref()
                .or(tuple_elem_expected_from_peer.as_ref());
            match elem {
                syn::Expr::Reference(r) => {
                    let mut reference_target = self.peel_reference_target_expr(&r.expr);
                    // Collapse `&*variable` reborrow for non-pointer paths:
                    // `&*r` where `r` is a value/ref → just `r`
                    // Do NOT collapse for raw pointers (`*p` is a real dereference)
                    if let syn::Expr::Unary(un) = reference_target {
                        if matches!(un.op, syn::UnOp::Deref(_)) {
                            let operand = self.peel_paren_group_expr(&un.expr);
                            let is_simple_non_pointer_surface =
                                matches!(operand, syn::Expr::Path(_) | syn::Expr::Field(_))
                                    && !self.is_expr_raw_pointer_like(&un.expr);
                            let operand_ty = self.infer_simple_expr_type(&un.expr);
                            let operand_is_reference_like = operand_ty.as_ref().is_some_and(|ty| {
                                matches!(
                                    self.peel_reference_paren_group_type(ty),
                                    syn::Type::Reference(_) | syn::Type::Ptr(_)
                                )
                            });
                            let operand_type_unknown = operand_ty.is_none();
                            let operand_is_local_path = matches!(operand, syn::Expr::Path(path)
                            if path.path.segments.len() == 1
                                && {
                                    let name = path.path.segments[0].ident.to_string();
                                    self.is_local_binding_in_scope(&name)
                                        || self.lookup_local_binding_type(&name).is_some()
                                });
                            if is_simple_non_pointer_surface
                                && (operand_is_reference_like
                                    || (operand_type_unknown && operand_is_local_path))
                            {
                                reference_target = &un.expr;
                            }
                        }
                    }
                    let mut inner_raw = self
                        .emit_result_ctor_expr_with_peer_context(
                            reference_target,
                            tuple_elem_expected_ty,
                            peer_expr,
                        )
                        .unwrap_or_else(|| {
                            self.emit_expr_to_string_with_expected(
                                reference_target,
                                tuple_elem_expected_ty,
                            )
                        });
                    let inner = self.maybe_wrap_variant_constructor_with_expected_enum(
                        reference_target,
                        inner_raw,
                        tuple_elem_expected_ty,
                    );
                    let is_slice_range_target =
                        self.is_slice_range_index_target_expr(reference_target);
                    let should_normalize_to_slice_full = tuple_has_slice_range_reference
                        && self.should_normalize_tuple_reference_target_to_slice_full(
                            reference_target,
                        );
                    if should_normalize_to_slice_full {
                        inner_raw = format!("rusty::slice_full({})", inner);
                    } else {
                        inner_raw = inner;
                    }
                    if let Some(string_view_expr) = self
                        .try_emit_tuple_reference_string_literal_deref_as_string_view(
                            reference_target,
                        )
                    {
                        // Expanded assertion tuple matches may bind `&*"literal"` (Rust `&str`)
                        // on one side. Materialize the borrowed literal as string_view so
                        // downstream `*right_val` compares stay string-like instead of scalar.
                        inner_raw = string_view_expr;
                    }
                    inner_raw =
                        self.materialize_unit_rvalue_expr_if_needed(reference_target, inner_raw);

                    // An empty `Vec::new()` operand compared to a typed peer has
                    // no element signal of its own; recover it from the peer via
                    // C++ decltype so it doesn't leak `<auto>`. Guarded to the
                    // already-broken case so working emits are untouched.
                    if type_string_contains_auto_template_arg(&inner_raw)
                        && let Some(peer_ctor) = self
                            .emit_owner_ctor_expr_with_peer_context(reference_target, peer_expr)
                    {
                        inner_raw = peer_ctor;
                    }

                    let reference_target_raw = self.emit_expr_to_string(reference_target);
                    let is_index_reference_target =
                        matches!(reference_target, syn::Expr::Index(_)) && !is_slice_range_target;
                    let can_take_address_directly = self.is_stable_reference_lvalue_expr(reference_target)
                        && !is_slice_range_target
                        && !should_normalize_to_slice_full
                        // Coercions like `std::string_view(t)` produce temporaries; they must
                        // be materialized before taking an address in tuple assertion scaffolding.
                        && inner_raw == reference_target_raw;
                    if can_take_address_directly {
                        if is_index_reference_target {
                            self.writeln(&format!(
                                "auto {} = rusty::as_ref_ptr({});",
                                elem_name, inner_raw
                            ));
                        } else {
                            self.writeln(&format!("auto {} = &{};", elem_name, inner_raw));
                        }
                    } else {
                        let tmp_name = format!("_m{}_tmp", idx);
                        if self.reference_target_requires_owned_materialization_for_address(
                            reference_target,
                        ) {
                            // `&*<rvalue>` can borrow through a temporary owner (e.g.
                            // `*v.swap_remove(1)`). Binding `auto&&` would preserve that
                            // borrow and dangle once the owner is destroyed. Materialize
                            // the deref result by value before taking an address.
                            self.writeln(&format!("auto {} = {};", tmp_name, inner_raw));
                        } else {
                            self.writeln(&format!("auto&& {} = {};", tmp_name, inner_raw));
                        }
                        if is_index_reference_target {
                            self.writeln(&format!(
                                "auto {} = rusty::as_ref_ptr({});",
                                elem_name, tmp_name
                            ));
                        } else {
                            self.writeln(&format!("auto {} = &{};", elem_name, tmp_name));
                        }
                    }
                }
                _ => {
                    let elem_expr_raw = self
                        .emit_result_ctor_expr_with_peer_context(
                            elem,
                            tuple_elem_expected_ty,
                            peer_expr,
                        )
                        .unwrap_or_else(|| {
                            self.emit_expr_to_string_with_expected(elem, tuple_elem_expected_ty)
                        });
                    let elem_expr = self.maybe_wrap_variant_constructor_with_expected_enum(
                        elem,
                        elem_expr_raw,
                        tuple_elem_expected_ty,
                    );
                    let elem_expr = self.materialize_unit_rvalue_expr_if_needed(elem, elem_expr);
                    self.writeln(&format!("auto {} = {};", elem_name, elem_expr));
                }
            }
            tuple_elem_names.push(elem_name);
        }

        self.writeln(&format!(
            "auto _m_tuple = std::make_tuple({});",
            tuple_elem_names.join(", ")
        ));
        self.writeln("bool _m_matched = false;");

        for arm in &match_expr.arms {
            self.writeln("if (!_m_matched) {");
            self.indent += 1;

            let mut binding_stmts = Vec::new();
            let mut binding_map = HashMap::new();
            let _ = self.collect_pattern_binding_stmts_with_cpp_name_map(
                &arm.pat,
                "_m_tuple",
                &mut binding_stmts,
                &mut binding_map,
            );
            let mut arm_binding_types = HashMap::new();
            if let Some(scrutinee_ty) = match_scrutinee_ty.as_ref() {
                self.bind_pattern_types_into_env(&arm.pat, scrutinee_ty, &mut arm_binding_types);
                arm_binding_types.retain(|name, _| binding_map.contains_key(name));
            }

            for stmt in binding_stmts {
                self.writeln(&stmt);
            }

            let pushed_binding_scope = self
                .push_local_cpp_binding_scope_with_types(&binding_map, Some(&arm_binding_types));
            if let Some((_, guard)) = &arm.guard {
                let guard_str = self.emit_expr_to_string(guard);
                self.writeln(&format!("if ({}) {{", guard_str));
                self.indent += 1;
                self.emit_arm_body(&arm.body);
                self.writeln("_m_matched = true;");
                self.indent -= 1;
                self.writeln("}");
            } else {
                self.emit_arm_body(&arm.body);
                self.writeln("_m_matched = true;");
            }
            self.pop_local_cpp_binding_scope(pushed_binding_scope);

            self.indent -= 1;
            self.writeln("}");
        }

        self.indent -= 1;
        self.writeln("}");
        true
    }

    pub(super) fn try_emit_tuple_reference_string_literal_deref_as_string_view(
        &self,
        reference_target: &syn::Expr,
    ) -> Option<String> {
        let reference_target = self.peel_paren_group_expr(reference_target);
        if matches!(
            reference_target,
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(_),
                ..
            })
        ) {
            let literal = self.emit_expr_to_string(reference_target);
            return Some(format!("std::string_view({})", literal));
        }
        let syn::Expr::Unary(unary) = reference_target else {
            return None;
        };
        if !matches!(unary.op, syn::UnOp::Deref(_)) {
            return None;
        }
        let inner = self.peel_paren_group_expr(&unary.expr);
        let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(_),
            ..
        }) = inner
        else {
            return None;
        };
        let literal = self.emit_expr_to_string(inner);
        Some(format!("std::string_view({})", literal))
    }

    /// The switch case label for a bare ident that names a UNIQUE C-like-enum
    /// variant (e.g. glob-imported `YAML_UTF8_ENCODING` → `yaml_encoding_t::YAML_UTF8_ENCODING`).
    /// `None` for a real binding ident (which becomes the `default:` catch-all).
    fn bare_c_like_enum_const_case_label(&self, name: &str) -> Option<String> {
        let owner = self.unique_c_like_enum_owner_for_variant_name(name)?;
        Some(format!("{}::{}", owner, name))
    }

    pub(super) fn emit_match_as_switch(
        &mut self,
        scrutinee: &str,
        arms: &[syn::Arm],
        variant_ctx: Option<&VariantTypeContext>,
    ) {
        // A `switch` catches a plain C++ `break`, so a Rust `break` inside an arm
        // (which targets the enclosing loop) must be lowered to a `goto`. Track
        // the switch frame so break/continue lowering knows it sits in between.
        self.cf_stack.push(crate::codegen::CfFrame::Switch);
        self.writeln(&format!("switch ({}) {{", scrutinee));

        // Track emitted case values across ALL arms to detect duplicates.
        // When multiple arms have the same pattern (e.g., `false if guard =>` and `false =>`),
        // we emit the first as case label and subsequent arms as else-if chains.
        let mut emitted_cases: HashMap<String, (bool, &syn::Arm)> = HashMap::new();

        for arm in arms {
            // Determine the case label(s) for this arm's pattern
            let case_labels: Vec<String> = match &arm.pat {
                syn::Pat::Wild(_) => vec!["default".to_string()],
                syn::Pat::Lit(lit) => vec![self.emit_lit(&lit.lit)],
                syn::Pat::Path(pp) => {
                    vec![self.emit_switch_pattern_path_value(&pp.path, variant_ctx)]
                }
                syn::Pat::Or(or_pat) => {
                    // OR pattern: collect all case labels, deduplicating within the OR
                    let mut labels: Vec<String> = Vec::new();
                    let mut seen_in_or: HashSet<String> = HashSet::new();
                    for case in &or_pat.cases {
                        match case {
                            syn::Pat::Lit(lit) => {
                                let val = self.emit_lit(&lit.lit);
                                if !seen_in_or.contains(&val) {
                                    seen_in_or.insert(val.clone());
                                    labels.push(val);
                                }
                            }
                            syn::Pat::Wild(_) => {
                                if !seen_in_or.contains("default") {
                                    seen_in_or.insert("default".to_string());
                                    labels.push("default".to_string());
                                }
                            }
                            syn::Pat::Path(pp) => {
                                let val =
                                    self.emit_switch_pattern_path_value(&pp.path, variant_ctx);
                                if !seen_in_or.contains(&val) {
                                    seen_in_or.insert(val.clone());
                                    labels.push(val);
                                }
                            }
                            // Bare C-like-enum variant by name (glob-imported), e.g.
                            // `YAML_UTF16LE_ENCODING | YAML_UTF16BE_ENCODING`.
                            syn::Pat::Ident(pi) if pi.subpat.is_none() => {
                                if let Some(val) =
                                    self.bare_c_like_enum_const_case_label(&pi.ident.to_string())
                                    && !seen_in_or.contains(&val)
                                {
                                    seen_in_or.insert(val.clone());
                                    labels.push(val);
                                }
                            }
                            _ => {}
                        }
                    }
                    labels
                }
                // A bare ident that names a unique C-like-enum constant is a CASE,
                // not a binding catch-all (`default`).
                syn::Pat::Ident(pi)
                    if pi.subpat.is_none()
                        && self
                            .bare_c_like_enum_const_case_label(&pi.ident.to_string())
                            .is_some() =>
                {
                    vec![self
                        .bare_c_like_enum_const_case_label(&pi.ident.to_string())
                        .unwrap()]
                }
                _ => vec!["default".to_string()],
            };

            // Check if any case label was already emitted by a previous arm
            let first_label = case_labels.first().cloned().unwrap_or_default();
            let is_duplicate = emitted_cases.contains_key(&first_label);

            if is_duplicate {
                // This arm has a duplicate pattern with a previous arm.
                // For duplicate patterns, we need to emit an else clause.
                // However, if the previous arm had a guard and this arm's body is empty,
                // we can skip this arm because the fall-through is implicit.
                let prev_had_guard = emitted_cases
                    .get(&first_label)
                    .map(|(had_guard, _)| *had_guard)
                    .unwrap_or(false);

                // Check if this arm's body is empty (just "{}")
                let is_body_empty = match &*arm.body {
                    syn::Expr::Block(block) => block.block.stmts.is_empty(),
                    _ => false,
                };

                if prev_had_guard && is_body_empty {
                    // Previous arm had a guard and this arm has empty body.
                    // Skip this arm - fall-through behavior is correct.
                } else if prev_had_guard {
                    // Previous arm had a guard, emit as else clause
                    self.writeln("} else {");
                    self.indent += 1;
                    self.emit_arm_body(&arm.body);
                    self.indent -= 1;
                    self.writeln("}");
                } else {
                    // Previous arm had no guard - this is a problem
                    self.writeln("// TODO: duplicate pattern without guard ordering");
                }
            } else {
                // Emit case label(s) for this arm
                for label in &case_labels {
                    emitted_cases.insert(label.clone(), (arm.guard.is_some(), arm));
                    if label == "default" {
                        self.writeln("default:");
                    } else {
                        self.writeln(&format!("case {}:", label));
                    }
                }

                self.writeln("{");
                self.indent += 1;
                let mut pattern_binding_stmts = Vec::new();
                if self.collect_pattern_binding_stmts(
                    &arm.pat,
                    scrutinee,
                    &mut pattern_binding_stmts,
                ) {
                    for stmt in &pattern_binding_stmts {
                        self.writeln(stmt);
                    }
                }

                // Emit guard if present
                if let Some((_, guard)) = &arm.guard {
                    let guard_str = self.emit_expr_to_string(guard);
                    self.writeln(&format!("if ({}) {{", guard_str));
                    self.indent += 1;
                    self.emit_arm_body(&arm.body);
                    self.indent -= 1;
                    self.writeln("}");
                } else {
                    self.emit_arm_body(&arm.body);
                }

                // Emit the switch-case terminator `break;` UNLESS the (unguarded)
                // arm body already diverges — i.e. ends in a Rust break/continue/
                // return, which lowers to a C++ break/goto/return that makes the
                // terminator unreachable dead code. (A guarded arm can still fall
                // through when the guard is false, so it always needs the break.)
                if arm.guard.is_some() || !self.match_arm_body_diverges(&arm.body) {
                    self.writeln("break;");
                }
                self.indent -= 1;
                self.writeln("}");
            }
        }
        self.writeln("}");
        self.cf_stack.pop();
    }

    /// Whether a match-arm body unconditionally diverges: it is, or ends in, a
    /// Rust `break`/`continue`/`return`. Used to suppress the dead switch-case
    /// terminator `break;` after such an arm.
    fn match_arm_body_diverges(&self, body: &syn::Expr) -> bool {
        fn expr_diverges(e: &syn::Expr) -> bool {
            match e {
                syn::Expr::Break(_) | syn::Expr::Continue(_) | syn::Expr::Return(_) => true,
                syn::Expr::Block(b) => match b.block.stmts.last() {
                    Some(syn::Stmt::Expr(tail, _)) => expr_diverges(tail),
                    _ => false,
                },
                _ => false,
            }
        }
        expr_diverges(body)
    }

    pub(super) fn emit_match_as_visit(
        &mut self,
        scrutinee: &str,
        arms: &[syn::Arm],
        variant_ctx: Option<&VariantTypeContext>,
        visit_mutably: bool,
    ) {
        // Emit in a local scope so path-pattern visit parameter recovery can
        // refer to `_m` via `decltype(_m)` when template args are implicit.
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!("auto&& _m = {};", scrutinee));
        self.writeln("std::visit(overloaded {");
        self.indent += 1;

        for arm in arms {
            self.emit_visit_arm(arm, variant_ctx, Some("_m"), visit_mutably);
        }

        self.indent -= 1;
        self.writeln("}, rusty::detail::deref_if_pointer(_m));");
        self.indent -= 1;
        self.writeln("}");
    }

    /// Emit a macro invocation as a statement.
    pub(super) fn emit_macro_stmt(&mut self, mac: &syn::Macro) {
        let macro_name = mac
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        let tokens = mac.tokens.to_string();

        match macro_name.as_str() {
            "println" => {
                let args = self.convert_format_args(&tokens);
                self.writeln(&format!("std::println({});", args));
            }
            "eprintln" => {
                let args = self.convert_format_args(&tokens);
                self.writeln(&format!("std::println(stderr, {});", args));
            }
            "print" => {
                let args = self.convert_format_args(&tokens);
                self.writeln(&format!("std::print({});", args));
            }
            "panic" => {
                if tokens.is_empty() {
                    self.writeln("std::abort();");
                } else {
                    let args = self.convert_format_args(&tokens);
                    self.writeln(&format!("std::println(stderr, {});", args));
                    self.writeln("std::abort();");
                }
            }
            "todo" => {
                self.writeln("throw std::logic_error(\"not yet implemented\");");
            }
            "unimplemented" => {
                self.writeln("throw std::logic_error(\"not implemented\");");
            }
            "unreachable" => {
                // Mirror Rust's `unreachable!()` macro: a noreturn call.
                // Lower to `rusty::intrinsics::unreachable()` (defined in the
                // runtime helper boilerplate as `[[noreturn]] inline void
                // unreachable() { throw ... }`) so the program aborts
                // loudly at runtime if the arm is ever reached, instead of
                // silently no-op'ing as `/* unreachable!() */;`.
                if tokens.is_empty() {
                    self.writeln("rusty::intrinsics::unreachable();");
                } else {
                    let args = self.convert_format_args(&tokens);
                    self.writeln(&format!("std::println(stderr, {});", args));
                    self.writeln("rusty::intrinsics::unreachable();");
                }
            }
            "assert" | "debug_assert" => {
                // Rust's `assert!` accepts up to three shapes:
                //   assert!(cond)
                //   assert!(cond, "message")
                //   assert!(cond, "fmt {} {}", a, b)
                // C's `assert()` is a single-arg macro that splits on commas
                // before tokenizing, so passing `assert!(cond, msg)` through
                // verbatim breaks the preprocessor. Worse, the body of
                // `assert!` is an expression that may itself contain `match`
                // or other constructs that the previous text-only
                // `convert_macro_tokens` left as raw Rust syntax.
                //
                // Lower properly: split on top-level commas, parse the
                // condition as `syn::Expr` so nested constructs go through
                // the real expression emitter, and use a throw-based
                // fallback when a message tail is present so the message
                // survives.
                let parts = self.split_macro_args(&tokens);
                let cond_text = parts.first().map(|s| s.trim()).unwrap_or("");
                let cond_cpp = if let Ok(expr) = syn::parse_str::<syn::Expr>(cond_text) {
                    self.emit_expr_to_string(&expr)
                } else {
                    self.convert_macro_tokens(cond_text)
                };
                if parts.len() <= 1 {
                    self.writeln(&format!("assert(({}));", cond_cpp));
                } else {
                    let msg_parts: Vec<&str> =
                        parts.iter().skip(1).map(|s| s.trim()).collect();
                    let msg_cpp = if msg_parts.len() == 1 {
                        // Bare message: pass through (string literal, or
                        // an expression like a variable holding the message).
                        if let Ok(expr) = syn::parse_str::<syn::Expr>(msg_parts[0]) {
                            self.emit_expr_to_string(&expr)
                        } else {
                            self.convert_macro_tokens(msg_parts[0])
                        }
                    } else {
                        // Format-args message: defer to std::format.
                        let joined = msg_parts.join(", ");
                        format!("std::format({})", self.convert_format_args(&joined))
                    };
                    self.writeln(&format!(
                        "if (!({})) {{ throw std::logic_error({}); }}",
                        cond_cpp, msg_cpp
                    ));
                }
            }
            "assert_eq" | "debug_assert_eq" => {
                let parts = self.split_macro_args(&tokens);
                if parts.len() >= 2 {
                    let left = self.convert_macro_tokens(parts[0].trim());
                    let right = self.convert_macro_tokens(parts[1].trim());
                    self.writeln(&format!("assert(({} == {}));", left, right));
                }
            }
            "assert_ne" | "debug_assert_ne" => {
                let parts = self.split_macro_args(&tokens);
                if parts.len() >= 2 {
                    let left = self.convert_macro_tokens(parts[0].trim());
                    let right = self.convert_macro_tokens(parts[1].trim());
                    self.writeln(&format!("assert(({} != {}));", left, right));
                }
            }
            "dbg" => {
                self.writeln(&format!(
                    "std::println(stderr, \"{{}}\", {});",
                    self.convert_macro_tokens(&tokens)
                ));
            }
            "for_both" => {
                if let Some(lowered) = self.try_lower_for_both_macro_expr(mac) {
                    self.writeln(&format!("{};", lowered));
                } else {
                    self.writeln(&format!("// TODO: {}!(...)", macro_name));
                }
            }
            _ => {
                self.writeln(&format!("// TODO: {}!(...)", macro_name));
            }
        }
    }

    pub(super) fn try_emit_for_both_io_method_shape_dispatch(
        &self,
        binding_pat: &syn::Pat,
        body_expr: &syn::Expr,
    ) -> Option<String> {
        let rust_binding_name = match binding_pat {
            syn::Pat::Ident(pi) => pi.ident.to_string(),
            _ => return None,
        };
        let cpp_binding_name = escape_cpp_keyword(&rust_binding_name);

        let body_expr = self.peel_paren_group_expr(body_expr);
        let syn::Expr::MethodCall(mc) = body_expr else {
            return None;
        };
        if mc.args.len() != 1 {
            return None;
        }

        let receiver_expr = self.peel_paren_group_expr(&mc.receiver);
        let syn::Expr::Path(path) = receiver_expr else {
            return None;
        };
        if path.path.segments.len() != 1 {
            return None;
        }
        let receiver_name = path.path.segments[0].ident.to_string();
        if receiver_name != rust_binding_name {
            return None;
        }

        let arg = self.emit_expr_maybe_move(mc.args.first()?);
        match mc.method.to_string().as_str() {
            "read" => Some(format!("rusty::io::read({}, {})", cpp_binding_name, arg)),
            "write" => Some(format!("rusty::io::write({}, {})", cpp_binding_name, arg)),
            _ => None,
        }
    }

    /// Emit a macro invocation as an expression (returns a string).
    pub(super) fn emit_macro_expr(&self, mac: &syn::Macro) -> String {
        let macro_name = mac
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        let tokens = mac.tokens.to_string();

        match macro_name.as_str() {
            "format" => {
                format!("std::format({})", self.convert_format_args(&tokens))
            }
            "format_args" => {
                let parts = self.split_macro_args(&tokens);
                if parts.is_empty() {
                    "std::string{}".to_string()
                } else {
                    let fmt_expr = parts[0].trim();
                    let mut debug_arg_positions = HashSet::new();
                    let mut pretty_debug_arg_positions = HashSet::new();
                    let mut native_arg_conversions: HashMap<usize, char> = HashMap::new();
                    let mut native_passthrough_arg_positions: HashSet<usize> = HashSet::new();
                    let fmt_cpp = if let Ok(lit) = syn::parse_str::<syn::LitStr>(fmt_expr) {
                        let fmt_literal = lit.value();
                        debug_arg_positions = self.format_literal_debug_arg_positions(
                            &fmt_literal,
                            parts.len().saturating_sub(1),
                        );
                        pretty_debug_arg_positions = self
                            .format_literal_pretty_debug_arg_positions(
                                &fmt_literal,
                                parts.len().saturating_sub(1),
                            );
                        native_arg_conversions = self.format_literal_native_arg_conversions(
                            &fmt_literal,
                            parts.len().saturating_sub(1),
                        );
                        native_passthrough_arg_positions = self
                            .format_literal_native_passthrough_arg_positions(
                                &fmt_literal,
                                parts.len().saturating_sub(1),
                            );
                        let rewritten = self.rewrite_rust_format_literal_for_cpp(&fmt_literal);
                        let escaped = escape_cpp_string_literal_content(&rewritten);
                        format!("\"{}\"", escaped)
                    } else {
                        self.convert_macro_tokens(fmt_expr)
                    };
                    if parts.len() == 1 {
                        format!("std::string({})", fmt_cpp)
                    } else {
                        let wrapped_args: Vec<String> = parts
                            .iter()
                            .skip(1)
                            .enumerate()
                            .map(|(arg_idx, arg)| {
                                let lowered = self.convert_format_arg_expr(arg.trim());
                                if pretty_debug_arg_positions.contains(&arg_idx) {
                                    format!("rusty::to_debug_string_pretty({})", lowered)
                                } else if debug_arg_positions.contains(&arg_idx) {
                                    format!("rusty::to_debug_string({})", lowered)
                                } else if let Some(conversion) =
                                    native_arg_conversions.get(&arg_idx).copied()
                                {
                                    if self.format_conversion_requires_numeric_bridge(conversion)
                                        && !self.format_arg_is_known_integer_like(arg.trim())
                                    {
                                        format!("rusty::format_numeric_arg({})", lowered)
                                    } else {
                                        lowered
                                    }
                                } else if native_passthrough_arg_positions.contains(&arg_idx) {
                                    lowered
                                } else if self.format_arg_is_known_scalar_like(arg.trim()) {
                                    lowered
                                } else {
                                    format!("rusty::to_string({})", lowered)
                                }
                            })
                            .collect();
                        format!("std::format({}, {})", fmt_cpp, wrapped_args.join(", "))
                    }
                }
            }
            "vec" => {
                // vec![1, 2, 3] → rusty::Vec<T>{1, 2, 3}
                // We can't infer T at this level, so use initializer list
                let items = self.convert_macro_tokens(&tokens);
                format!("rusty::Vec{{{}}}", items)
            }
            "String::from" => {
                format!(
                    "rusty::String::from({})",
                    self.convert_macro_tokens(&tokens)
                )
            }
            "todo" => "throw std::logic_error(\"not yet implemented\")".to_string(),
            "unimplemented" => "throw std::logic_error(\"not implemented\")".to_string(),
            "unreachable" => {
                // Mirror Rust's `unreachable!()` macro in expression position.
                // Wrap in an IIFE so the noreturn helper can appear where any
                // value would have been; the type comes from context.
                if tokens.is_empty() {
                    "rusty::intrinsics::unreachable()".to_string()
                } else {
                    let args = self.convert_format_args(&tokens);
                    format!(
                        "([&]() {{ std::println(stderr, {}); rusty::intrinsics::unreachable(); }}())",
                        args
                    )
                }
            }
            "stringify" => {
                format!("\"{}\"", tokens.replace('"', "\\\""))
            }
            "concat" => {
                let parts = self.split_macro_args(&tokens);
                let joined: Vec<String> = parts
                    .iter()
                    .map(|p| format!("std::string({})", p.trim()))
                    .collect();
                if joined.len() == 1 {
                    joined[0].clone()
                } else {
                    joined.join(" + ")
                }
            }
            "for_both" => self
                .try_lower_for_both_macro_expr(mac)
                .unwrap_or_else(|| format!("/* {}!({}) */", macro_name, tokens)),
            _ => {
                format!("/* {}!({}) */", macro_name, tokens)
            }
        }
    }

    /// Emit a switch-style match as an expression (returns string for IIFE body).
    pub(super) fn emit_match_expr_switch(
        &self,
        arms: &[syn::Arm],
        expected_ty: Option<&syn::Type>,
        variant_ctx: Option<&VariantTypeContext>,
    ) -> String {
        self.emit_match_expr_switch_with_consumed_scrutinee(arms, expected_ty, variant_ctx, false)
    }

    pub(super) fn emit_match_expr_switch_with_consumed_scrutinee(
        &self,
        arms: &[syn::Arm],
        expected_ty: Option<&syn::Type>,
        variant_ctx: Option<&VariantTypeContext>,
        scrutinee_is_consumed_place: bool,
    ) -> String {
        let tuple_literal_hints = self.collect_switch_match_tuple_literal_hints(arms);
        let mut parts = Vec::new();
        for arm in arms {
            let arm_value_expr = self
                .extract_match_arm_value_expr(&arm.body)
                .unwrap_or(&arm.body);
            // A consumed scrutinee's by-value pattern bindings move at
            // their arm-body uses (`Tag(string) => Out::Some2(string)` —
            // Rust moves the payload out of the matched value).
            let pushed_movable_frame = if scrutinee_is_consumed_place {
                let mut names = HashSet::new();
                self.collect_pattern_value_binding_names(&arm.pat, &mut names);
                if names.is_empty() {
                    false
                } else {
                    self.movable_match_binding_scopes.borrow_mut().push(names);
                    true
                }
            } else {
                false
            };
            let body = {
                let emitted = if let (Some(hints), syn::Expr::Tuple(tuple)) = (
                    tuple_literal_hints.as_ref(),
                    self.peel_paren_group_expr(arm_value_expr),
                ) {
                    self.emit_switch_arm_tuple_expr_with_hints(tuple, expected_ty, hints)
                } else {
                    self.emit_expr_to_string_with_expected(&arm.body, expected_ty)
                };
                self.maybe_wrap_variant_constructor_with_expected_enum(
                    &arm.body,
                    emitted,
                    expected_ty,
                )
            };
            if pushed_movable_frame {
                self.movable_match_binding_scopes.borrow_mut().pop();
            }
            // Detect diverging (never-returning) arm bodies to avoid `return <void>;`
            let diverging = self.is_expr_diverging(&arm.body);
            let ret_prefix = if diverging { "" } else { "return " };
            match &arm.pat {
                syn::Pat::Wild(_) => parts.push(format!("{}{};", ret_prefix, body)),
                syn::Pat::Lit(lit) => {
                    let val = self.emit_lit(&lit.lit);
                    parts.push(format!("if (_m == {}) {}{};", val, ret_prefix, body));
                }
                syn::Pat::Path(pp) => {
                    let cond = self.path_pattern_value_condition(&pp.path, "_m", variant_ctx);
                    parts.push(format!("if ({}) {}{};", cond, ret_prefix, body));
                }
                syn::Pat::Or(or_pat) => {
                    let mut conds = Vec::new();
                    for case in &or_pat.cases {
                        match case {
                            syn::Pat::Lit(lit) => {
                                conds.push(format!("_m == {}", self.emit_lit(&lit.lit)))
                            }
                            syn::Pat::Path(pp) => conds.push(self.path_pattern_value_condition(
                                &pp.path,
                                "_m",
                                variant_ctx,
                            )),
                            syn::Pat::Range(_range_pat) => {
                                let Some(cond) =
                                    self.tuple_pattern_elem_value_condition(case, "_m")
                                else {
                                    continue;
                                };
                                if let Some(cond) = cond {
                                    conds.push(cond);
                                } else {
                                    conds.clear();
                                    conds.push("true".to_string());
                                    break;
                                }
                            }
                            syn::Pat::Wild(_) => {
                                conds.clear();
                                conds.push("true".to_string());
                                break;
                            }
                            _ => {}
                        }
                    }
                    if conds.is_empty() {
                        parts.push(format!("{}{};", ret_prefix, body));
                    } else {
                        parts.push(format!(
                            "if ({}) {}{};",
                            conds.join(" || "),
                            ret_prefix,
                            body
                        ));
                    }
                }
                syn::Pat::Ident(pi) if pi.ident != "_" => {
                    let mut binding_stmts = Vec::new();
                    if pi.subpat.is_some() {
                        if let Some(cond) = self.collect_runtime_match_binding_stmts_and_condition(
                            &arm.pat,
                            "_m",
                            &mut binding_stmts,
                            variant_ctx,
                        ) {
                            let bindings = if binding_stmts.is_empty() {
                                String::new()
                            } else {
                                format!("{} ", binding_stmts.join(" "))
                            };
                            if let Some(cond) = cond {
                                parts.push(format!(
                                    "if ({}) {{ {}{}{}; }}",
                                    cond, bindings, ret_prefix, body
                                ));
                            } else {
                                parts.push(format!("{{ {}{}{}; }}", bindings, ret_prefix, body));
                            }
                        } else {
                            parts.push(format!("{}{};", ret_prefix, body));
                        }
                    } else {
                        // A bare ident in a match arm with no subpattern is
                        // ambiguous in syn — could be either a fresh binding
                        // (`x => …`) or a const-value pattern
                        // (`CONST_NAME => …` where `CONST_NAME` is a const
                        // item in scope). We don't have a full name-resolution
                        // pass; use Rust's SCREAMING_SNAKE_CASE convention
                        // as the heuristic: if the ident is all-uppercase
                        // letters/digits/underscores AND contains at least
                        // one letter, treat it as a const-value pattern and
                        // emit an equality test. Otherwise treat as binding.
                        let ident_str = pi.ident.to_string();
                        let is_const_style = ident_str.chars().any(|c| c.is_ascii_alphabetic())
                            && ident_str.chars().all(|c| {
                                c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'
                            });
                        if is_const_style {
                            // A bare C-like enum variant (`use Enum::*`) must be
                            // compared against the scoped `Enum::VARIANT` — C++20
                            // `enum class` does not flatten variants into scope.
                            let cmp = self
                                .unique_c_like_enum_owner_for_variant_name(&ident_str)
                                .map(|owner| format!("{}::{}", owner, ident_str))
                                .unwrap_or_else(|| ident_str.clone());
                            parts.push(format!(
                                "if (_m == {}) {{ {}{};  }}",
                                cmp, ret_prefix, body
                            ));
                        } else {
                            // Catch-all with binding: `cmp => cmp` → declare alias to scrutinee
                            let cpp_name = escape_cpp_keyword(&ident_str);
                            parts.push(format!(
                                "{{ const auto& {} = _m; {}{};  }}",
                                cpp_name, ret_prefix, body
                            ));
                        }
                    }
                }
                syn::Pat::Range(_) => {
                    if let Some(cond) = self.tuple_pattern_elem_value_condition(&arm.pat, "_m") {
                        if let Some(cond) = cond {
                            parts.push(format!("if ({}) {}{};", cond, ret_prefix, body));
                        } else {
                            parts.push(format!("{}{};", ret_prefix, body));
                        }
                    } else {
                        parts.push(format!("{}{};", ret_prefix, body));
                    }
                }
                _ => parts.push(format!("{}{};", ret_prefix, body)),
            }
        }
        parts.join("\n")
    }

    pub(super) fn emit_match_expr_switch_statement_expr(
        &self,
        match_expr: &syn::ExprMatch,
        expected_ty: Option<&syn::Type>,
        variant_ctx: Option<&VariantTypeContext>,
    ) -> Option<String> {
        self.emit_match_expr_switch_statement_expr_with_arm_mode(
            match_expr,
            expected_ty,
            variant_ctx,
            false,
        )
    }

    /// `allow_guarded_variant_arms` is set ONLY by the runtime-match
    /// delegation (`emit_runtime_match_expr`): it enables TupleStruct-pattern
    /// arms and guard lowering for the fn-return-vs-match-value shape the
    /// IIFE lambda cannot host. The general entry keeps them off so
    /// Result/Option payload matches keep their historical routing to the
    /// runtime-match / unwrap-call lowerings.
    pub(super) fn emit_match_expr_switch_statement_expr_with_arm_mode(
        &self,
        match_expr: &syn::ExprMatch,
        expected_ty: Option<&syn::Type>,
        variant_ctx: Option<&VariantTypeContext>,
        allow_guarded_variant_arms: bool,
    ) -> Option<String> {
        if std::env::var_os("RUSTY_CPP_DISABLE_MATCH_SWITCH_STATEMENT_EXPR").is_some() {
            return None;
        }
        if expected_ty.is_none() {
            return None;
        }
        let match_contains_early_return_or_try =
            self.expr_contains_early_return_or_try(&syn::Expr::Match(match_expr.clone()));
        if self.should_soften_dependent_assoc_mode() && !match_contains_early_return_or_try {
            return None;
        }
        let expected_ty_for_arms = if self.should_soften_dependent_assoc_mode()
            && expected_ty.is_some_and(|ty| {
                self.type_references_current_struct_assoc_projection(ty)
                    && !self.type_current_struct_assoc_aliases_emitted(ty)
            }) {
            None
        } else {
            expected_ty
        };
        let tuple_literal_hints = self.collect_switch_match_tuple_literal_hints(&match_expr.arms);
        let mut value_cpp = expected_ty_for_arms.and_then(|expected| {
            let mapped = self.map_type(expected);
            if mapped == "auto"
                || mapped.contains("/* TODO")
                || type_string_has_auto_placeholder(&mapped)
            {
                None
            } else {
                Some(mapped)
            }
        });

        if value_cpp.is_none() {
            value_cpp = self
                .infer_match_arms_common_type(&match_expr.arms)
                .or_else(|| self.infer_match_arms_common_type_with_scrutinee(match_expr))
                .or_else(|| {
                    match_expr.arms.iter().find_map(|arm| {
                        if self.is_expr_diverging(&arm.body) {
                            return None;
                        }
                        self.infer_match_arm_type_with_scrutinee_bindings(&match_expr.expr, arm)
                            .or_else(|| self.infer_local_binding_type_from_initializer(&arm.body))
                            .or_else(|| self.infer_simple_expr_type(&arm.body))
                    })
                })
                .map(|ty| self.map_type(&ty))
                .filter(|mapped| {
                    mapped != "auto"
                        && !mapped.contains("/* TODO")
                        && !type_string_has_auto_placeholder(mapped)
                });
        }

        if value_cpp.is_none() {
            value_cpp = tuple_literal_hints.as_ref().and_then(|hints| {
                match_expr.arms.iter().find_map(|arm| {
                    if self.is_expr_diverging(&arm.body) {
                        return None;
                    }
                    let arm_value_expr = self.extract_match_arm_value_expr(&arm.body)?;
                    let syn::Expr::Tuple(tuple) = self.peel_paren_group_expr(arm_value_expr) else {
                        return None;
                    };
                    let tuple_expr = self.emit_switch_arm_tuple_expr_with_hints(
                        tuple,
                        expected_ty_for_arms,
                        hints,
                    );
                    if Self::cpp_expr_uses_statement_expr_syntax(&tuple_expr)
                        && !match_contains_early_return_or_try
                    {
                        return None;
                    }
                    Some(format!("std::remove_cvref_t<decltype(({}))>", tuple_expr))
                })
            });
        }
        if value_cpp.is_none() {
            value_cpp = match_expr.arms.iter().find_map(|arm| {
                if self.is_expr_diverging(&arm.body) {
                    return None;
                }
                let body_expr =
                    self.emit_expr_to_string_with_expected(&arm.body, expected_ty_for_arms);
                if Self::cpp_expr_uses_statement_expr_syntax(&body_expr)
                    && !match_contains_early_return_or_try
                {
                    return None;
                }
                Some(format!("std::remove_cvref_t<decltype(({}))>", body_expr))
            });
        }
        // Deduction from a single arm locks a bare-None tuple slot onto
        // None_t; the cross-arm tuple annotation types it properly.
        let value_cpp = value_cpp.or_else(|| {
            self.infer_runtime_match_tuple_annotation(match_expr)
                .map(|annotation| annotation.trim_start_matches(" -> ").to_string())
        });
        let value_cpp = value_cpp?;
        let storage_cpp = if value_cpp.contains('&') {
            format!("std::remove_cvref_t<{}>", value_cpp)
        } else {
            value_cpp.clone()
        };
        let scrutinee = self.emit_expr_to_string_with_variant_ctx(&match_expr.expr, variant_ctx);
        let mut out = format!(
            "({{ auto&& _m = {}; std::optional<{}> _match_value; bool _m_matched = false; ",
            scrutinee, storage_cpp
        );

        for arm in &match_expr.arms {
            let diverging = self.is_expr_diverging(&arm.body);
            let arm_body_stmt = if diverging {
                let Some(diverging_stmt) = self.emit_match_diverging_arm_stmt(&arm.body) else {
                    return None;
                };
                format!("{} _m_matched = true;", diverging_stmt)
            } else {
                let arm_value_expr = self
                    .extract_match_arm_value_expr(&arm.body)
                    .unwrap_or(&arm.body);
                let body = {
                    let emitted = if let (Some(hints), syn::Expr::Tuple(tuple)) = (
                        tuple_literal_hints.as_ref(),
                        self.peel_paren_group_expr(arm_value_expr),
                    ) {
                        self.emit_switch_arm_tuple_expr_with_hints(
                            tuple,
                            expected_ty_for_arms,
                            hints,
                        )
                    } else {
                        self.emit_expr_to_string_with_expected(&arm.body, expected_ty_for_arms)
                    };
                    self.maybe_wrap_variant_constructor_with_expected_enum(
                        &arm.body,
                        emitted,
                        expected_ty_for_arms,
                    )
                };
                if Self::cpp_expr_uses_statement_expr_syntax(&body)
                    && !match_contains_early_return_or_try
                {
                    return None;
                }
                let body_trimmed = body.trim_start();
                let body_expr = if body_trimmed.starts_with("return ") {
                    body_trimmed
                        .trim_start_matches("return ")
                        .trim_end_matches(';')
                        .trim()
                        .to_string()
                } else {
                    body.clone()
                };
                format!(
                    "_match_value.emplace(std::move({})); _m_matched = true;",
                    body_expr
                )
            };
            // Rust guard semantics: pattern bindings are in scope for the
            // guard, and a FAILED guard falls through without matching (the
            // next arm stays eligible). Wrap the arm body in the guard
            // condition — bindings are spliced before it by each pattern arm
            // below, and `_m_matched` is only set inside the guard.
            let arm_body_stmt = if let (true, Some((_, guard_expr))) =
                (allow_guarded_variant_arms, &arm.guard)
            {
                let guard_cpp = self.emit_expr_to_string(guard_expr);
                if guard_cpp.trim().is_empty() {
                    return None;
                }
                format!("if ({}) {{ {} }}", guard_cpp, arm_body_stmt)
            } else {
                arm_body_stmt
            };

            match &arm.pat {
                syn::Pat::Wild(_) => {
                    out.push_str(&format!("if (!_m_matched) {{ {} }} ", arm_body_stmt));
                }
                // Variant patterns with payload bindings (`Bound::Included(&i)`,
                // indexmap's try_simplify_range). Without this arm the whole
                // statement-expression lowering bailed and the match fell to the
                // IIFE-lambda path — which mis-lowers an early-return arm
                // (`_ => return None`) into a `return` FROM THE LAMBDA, yielding
                // "no viable conversion from Option<Range<usize>> to size_t".
                // The statement-expression keeps `return` a real function return.
                syn::Pat::TupleStruct(_) | syn::Pat::Struct(_)
                    if allow_guarded_variant_arms =>
                {
                    let mut binding_stmts = Vec::new();
                    let Some(cond) = self.collect_runtime_match_binding_stmts_and_condition(
                        &arm.pat,
                        "_m",
                        &mut binding_stmts,
                        variant_ctx,
                    ) else {
                        return None;
                    };
                    let bindings = if binding_stmts.is_empty() {
                        String::new()
                    } else {
                        format!("{} ", binding_stmts.join(" "))
                    };
                    if let Some(cond) = cond {
                        out.push_str(&format!(
                            "if (!_m_matched && ({})) {{ {}{} }} ",
                            cond, bindings, arm_body_stmt
                        ));
                    } else {
                        out.push_str(&format!(
                            "if (!_m_matched) {{ {}{} }} ",
                            bindings, arm_body_stmt
                        ));
                    }
                }
                syn::Pat::Lit(lit) => {
                    let val = self.emit_lit(&lit.lit);
                    out.push_str(&format!(
                        "if (!_m_matched && (_m == {})) {{ {} }} ",
                        val, arm_body_stmt
                    ));
                }
                syn::Pat::Path(pp) => {
                    let cond = self.path_pattern_value_condition(&pp.path, "_m", variant_ctx);
                    out.push_str(&format!(
                        "if (!_m_matched && ({})) {{ {} }} ",
                        cond, arm_body_stmt
                    ));
                }
                syn::Pat::Or(or_pat) => {
                    let mut conds = Vec::new();
                    for case in &or_pat.cases {
                        match case {
                            syn::Pat::Lit(lit) => {
                                conds.push(format!("_m == {}", self.emit_lit(&lit.lit)))
                            }
                            syn::Pat::Path(pp) => conds.push(self.path_pattern_value_condition(
                                &pp.path,
                                "_m",
                                variant_ctx,
                            )),
                            syn::Pat::Range(_) => {
                                let Some(cond) =
                                    self.tuple_pattern_elem_value_condition(case, "_m")
                                else {
                                    continue;
                                };
                                if let Some(cond) = cond {
                                    conds.push(cond);
                                } else {
                                    conds.clear();
                                    conds.push("true".to_string());
                                    break;
                                }
                            }
                            syn::Pat::Wild(_) => {
                                conds.clear();
                                conds.push("true".to_string());
                                break;
                            }
                            // `Progress::Iterable(_) | Progress::Document(_)`:
                            // an all-wildcard TupleStruct or-case is just the
                            // variant condition.
                            syn::Pat::TupleStruct(ts)
                                if ts.elems.iter().all(|elem| {
                                    matches!(
                                        self.peel_pat_type_ref_paren(elem),
                                        syn::Pat::Wild(_)
                                    )
                                }) =>
                            {
                                conds.push(self.runtime_variant_match_condition_for_path(
                                    &ts.path,
                                    variant_ctx,
                                    "_m",
                                ));
                            }
                            _ => return None,
                        }
                    }
                    if conds.is_empty() {
                        out.push_str(&format!("if (!_m_matched) {{ {} }} ", arm_body_stmt));
                    } else {
                        out.push_str(&format!(
                            "if (!_m_matched && ({})) {{ {} }} ",
                            conds.join(" || "),
                            arm_body_stmt
                        ));
                    }
                }
                syn::Pat::Ident(pi) if pi.ident != "_" => {
                    let mut binding_stmts = Vec::new();
                    if pi.subpat.is_some() {
                        let Some(cond) = self.collect_runtime_match_binding_stmts_and_condition(
                            &arm.pat,
                            "_m",
                            &mut binding_stmts,
                            variant_ctx,
                        ) else {
                            return None;
                        };
                        let bindings = if binding_stmts.is_empty() {
                            String::new()
                        } else {
                            format!("{} ", binding_stmts.join(" "))
                        };
                        if let Some(cond) = cond {
                            out.push_str(&format!(
                                "if (!_m_matched && ({})) {{ {}{} }} ",
                                cond, bindings, arm_body_stmt
                            ));
                        } else {
                            out.push_str(&format!(
                                "if (!_m_matched) {{ {}{} }} ",
                                bindings, arm_body_stmt
                            ));
                        }
                    } else {
                        let cpp_name = escape_cpp_keyword(&pi.ident.to_string());
                        out.push_str(&format!(
                            "if (!_m_matched) {{ const auto& {} = _m; {} }} ",
                            cpp_name, arm_body_stmt
                        ));
                    }
                }
                syn::Pat::Range(_) => {
                    if let Some(cond) = self.tuple_pattern_elem_value_condition(&arm.pat, "_m") {
                        if let Some(cond) = cond {
                            out.push_str(&format!(
                                "if (!_m_matched && ({})) {{ {} }} ",
                                cond, arm_body_stmt
                            ));
                        } else {
                            out.push_str(&format!("if (!_m_matched) {{ {} }} ", arm_body_stmt));
                        }
                    } else {
                        out.push_str(&format!("if (!_m_matched) {{ {} }} ", arm_body_stmt));
                    }
                }
                _ => return None,
            }
        }

        out.push_str(
            "if (!_m_matched) { rusty::intrinsics::unreachable(); } std::move(_match_value).value(); })",
        );
        Some(out)
    }

    pub(super) fn emit_match_diverging_arm_stmt(&self, expr: &syn::Expr) -> Option<String> {
        let expr = self.peel_paren_group_expr(expr);
        match expr {
            syn::Expr::Block(block_expr) => {
                let mut inner = self.new_inner_for_block();
                let mut stmts: Vec<String> = Vec::new();
                for stmt in &block_expr.block.stmts {
                    let emitted = inner.emit_stmt_to_string(stmt);
                    if !emitted.trim().is_empty() {
                        stmts.push(emitted);
                    }
                }
                if stmts.is_empty() {
                    None
                } else {
                    Some(stmts.join(" "))
                }
            }
            syn::Expr::Unsafe(unsafe_expr) => {
                let mut inner = self.new_inner_for_block();
                let mut stmts: Vec<String> = vec!["// @unsafe".to_string()];
                for stmt in &unsafe_expr.block.stmts {
                    let emitted = inner.emit_stmt_to_string(stmt);
                    if !emitted.trim().is_empty() {
                        stmts.push(emitted);
                    }
                }
                if stmts.len() == 1 {
                    None
                } else {
                    Some(stmts.join(" "))
                }
            }
            _ => {
                let emitted = self.emit_expr_to_string(expr);
                if emitted.trim().is_empty() {
                    None
                } else if emitted.trim_end().ends_with(';') {
                    Some(emitted)
                } else {
                    Some(format!("{};", emitted))
                }
            }
        }
    }

    /// Emit a variant match as expression body (returns string for visit lambdas).
    pub(super) fn emit_match_expr_visit(
        &self,
        scrutinee_expr: &syn::Expr,
        arms: &[syn::Arm],
        variant_ctx: Option<&VariantTypeContext>,
        expected_ty: Option<&syn::Type>,
        borrow_payload: bool,
    ) -> String {
        // Expression-form match arms may dereference payload bindings (e.g. `*ch`) that
        // are references due match ergonomics. Evaluate each arm body with a temporary
        // pattern-ref scope so unary deref lowering can collapse reference layers.
        let mut scoped = self.clone();
        let match_scrutinee_ty = scoped
            .infer_simple_expr_type(scrutinee_expr)
            .or_else(|| scoped.infer_local_binding_type_from_initializer(scrutinee_expr));
        let mut parts = Vec::new();
        let lambda_return_annotation = scoped.expected_lambda_return_annotation(expected_ty, false);
        let borrow_payload_mutably = borrow_payload
            && (scoped
                .infer_simple_expr_type(scrutinee_expr)
                .or_else(|| scoped.infer_local_binding_type_from_initializer(scrutinee_expr))
                .as_ref()
                .is_some_and(Self::is_mut_reference_type)
                || scoped.expr_method_chain_contains_reference_method(scrutinee_expr));
        for arm in arms {
            scoped.push_pattern_ref_binding_scope(&arm.pat);
            let mut arm_binding_type_hints = HashMap::new();
            if let Some(scrutinee_ty) = match_scrutinee_ty.as_ref() {
                scoped.bind_pattern_types_into_env(
                    &arm.pat,
                    scrutinee_ty,
                    &mut arm_binding_type_hints,
                );
            }
            let mut arm_binding_names = HashSet::new();
            scoped.collect_pattern_binding_names(&arm.pat, &mut arm_binding_names);
            arm_binding_type_hints.retain(|name, _| arm_binding_names.contains(name));
            let pushed_binding_scope = if arm_binding_names.is_empty() {
                false
            } else {
                let mut local_types = HashMap::new();
                let mut local_consts = HashMap::new();
                for name in &arm_binding_names {
                    local_types.insert(name.clone(), arm_binding_type_hints.get(name).cloned());
                    local_consts.insert(name.clone(), false);
                }
                scoped.local_bindings.push(local_types);
                scoped.local_shadowed_binding_types.push(HashMap::new());
                scoped.local_const_bindings.push(local_consts);
                scoped.local_reference_bindings.push(HashSet::new());
                scoped
                    .rebind_reference_pointer_bindings
                    .push(HashSet::new());
                scoped.local_cpp_bindings.push(HashMap::new());
                true
            };
            let body = {
                let emitted = scoped.emit_expr_to_string_with_expected(&arm.body, expected_ty);
                scoped.maybe_wrap_variant_constructor_with_expected_enum(
                    &arm.body,
                    emitted,
                    expected_ty,
                )
            };
            let diverging = scoped.is_expr_diverging(&arm.body);
            let arm_value = if diverging {
                format!(
                    "(static_cast<void>({}), {})",
                    body,
                    scoped.match_expr_unreachable_fallback_with_expected(expected_ty)
                )
            } else {
                body.clone()
            };
            match &arm.pat {
                syn::Pat::TupleStruct(ts) => {
                    let cpp_type = scoped.visit_pattern_cpp_type(&ts.path, variant_ctx, Some("_m"));
                    let cpp_type = scoped.visit_variant_deduced_type(cpp_type);
                    let Some(binding_stmts) =
                        scoped.tuple_struct_binding_stmts(&ts.path, &ts.elems, "_v", variant_ctx)
                    else {
                        parts.push(format!(
                            "[&](const auto&) {{ return {}; }}",
                            scoped.match_expr_unreachable_fallback_with_expected(expected_ty)
                        ));
                        if pushed_binding_scope {
                            scoped.local_cpp_bindings.pop();
                            scoped.local_reference_bindings.pop();
                            scoped.rebind_reference_pointer_bindings.pop();
                            scoped.local_const_bindings.pop();
                            scoped.local_shadowed_binding_types.pop();
                            scoped.local_bindings.pop();
                        }
                        scoped.pop_pattern_ref_binding_scope();
                        continue;
                    };
                    if let Some((_, guard)) = &arm.guard {
                        let guard_str = scoped.emit_expr_to_string(guard);
                        let needs_mut_param = binding_stmts.iter().any(|s| s.starts_with("auto& "))
                            || scoped.pattern_requires_mut_ref_binding(&arm.pat)
                            || scoped.expected_type_contains_mut_reference(expected_ty);
                        let visit_param = if needs_mut_param {
                            format!("{}& _v", cpp_type)
                        } else if borrow_payload {
                            if borrow_payload_mutably {
                                format!("{}& _v", cpp_type)
                            } else {
                                format!("const {}& _v", cpp_type)
                            }
                        } else {
                            format!("{}&& _v", cpp_type)
                        };
                        parts.push(format!(
                            "[&]({}){} {{ {} if ({}) return {}; return {}; }}",
                            visit_param,
                            lambda_return_annotation,
                            binding_stmts.join("\n"),
                            guard_str,
                            arm_value,
                            scoped.match_expr_unreachable_fallback_with_expected(expected_ty),
                        ));
                    } else {
                        let needs_mut_param = binding_stmts.iter().any(|s| s.starts_with("auto& "))
                            || scoped.pattern_requires_mut_ref_binding(&arm.pat)
                            || scoped.expected_type_contains_mut_reference(expected_ty);
                        let visit_param = if needs_mut_param {
                            format!("{}& _v", cpp_type)
                        } else if borrow_payload {
                            if borrow_payload_mutably {
                                format!("{}& _v", cpp_type)
                            } else {
                                format!("const {}& _v", cpp_type)
                            }
                        } else {
                            format!("{}&& _v", cpp_type)
                        };
                        parts.push(format!(
                            "[&]({}){} {{ {} return {}; }}",
                            visit_param,
                            lambda_return_annotation,
                            binding_stmts.join("\n"),
                            arm_value
                        ));
                    }
                }
                syn::Pat::Path(pp) => {
                    let cpp_type = scoped.visit_pattern_cpp_type(&pp.path, variant_ctx, Some("_m"));
                    let cpp_type = scoped.visit_variant_deduced_type(cpp_type);
                    if let Some((_, guard)) = &arm.guard {
                        let guard_str = scoped.emit_expr_to_string(guard);
                        let visit_param = if borrow_payload {
                            if borrow_payload_mutably {
                                format!("{}& _v", cpp_type)
                            } else {
                                format!("const {}& _v", cpp_type)
                            }
                        } else {
                            format!("{}&& _v", cpp_type)
                        };
                        parts.push(format!(
                            "[&]({}){} {{ if ({}) return {}; return {}; }}",
                            visit_param,
                            lambda_return_annotation,
                            guard_str,
                            arm_value,
                            scoped.match_expr_unreachable_fallback_with_expected(expected_ty),
                        ));
                    } else {
                        let visit_param = if borrow_payload {
                            if borrow_payload_mutably {
                                format!("{}&", cpp_type)
                            } else {
                                format!("const {}&", cpp_type)
                            }
                        } else {
                            format!("{}&&", cpp_type)
                        };
                        parts.push(format!(
                            "[&]({}){} {{ return {}; }}",
                            visit_param, lambda_return_annotation, arm_value
                        ));
                    }
                }
                syn::Pat::Struct(ps) => {
                    let cpp_type = scoped.visit_pattern_cpp_type(&ps.path, variant_ctx, Some("_m"));
                    let cpp_type = scoped.visit_variant_deduced_type(cpp_type);
                    let mut binding_stmts = Vec::new();
                    let mut supported = true;
                    for field_pat in &ps.fields {
                        let field_name = field_pat.member.clone();
                        let field_name_str = match &field_name {
                            syn::Member::Named(ident) => ident.to_string(),
                            syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                        };
                        let field_expr = format!("_v.{}", field_name_str);
                        if !scoped.collect_pattern_binding_stmts(
                            &field_pat.pat,
                            &field_expr,
                            &mut binding_stmts,
                        ) {
                            supported = false;
                            break;
                        }
                    }
                    if !supported {
                        parts.push(format!(
                            "[&](const auto&){} {{ return {}; }}",
                            lambda_return_annotation,
                            scoped.match_expr_unreachable_fallback_with_expected(expected_ty)
                        ));
                        if pushed_binding_scope {
                            scoped.local_cpp_bindings.pop();
                            scoped.local_reference_bindings.pop();
                            scoped.rebind_reference_pointer_bindings.pop();
                            scoped.local_const_bindings.pop();
                            scoped.local_shadowed_binding_types.pop();
                            scoped.local_bindings.pop();
                        }
                        scoped.pop_pattern_ref_binding_scope();
                        continue;
                    }
                    let visit_param = if scoped.pattern_requires_mut_ref_binding(&arm.pat)
                        || scoped.expected_type_contains_mut_reference(expected_ty)
                    {
                        format!("{}& _v", cpp_type)
                    } else if borrow_payload {
                        if borrow_payload_mutably {
                            format!("{}& _v", cpp_type)
                        } else {
                            format!("const {}& _v", cpp_type)
                        }
                    } else {
                        format!("{}&& _v", cpp_type)
                    };
                    if let Some((_, guard)) = &arm.guard {
                        let guard_str = scoped.emit_expr_to_string(guard);
                        parts.push(format!(
                            "[&]({}){} {{ {} if ({}) return {}; return {}; }}",
                            visit_param,
                            lambda_return_annotation,
                            binding_stmts.join("\n"),
                            guard_str,
                            arm_value,
                            scoped.match_expr_unreachable_fallback_with_expected(expected_ty),
                        ));
                    } else {
                        parts.push(format!(
                            "[&]({}){} {{ {} return {}; }}",
                            visit_param,
                            lambda_return_annotation,
                            binding_stmts.join("\n"),
                            arm_value
                        ));
                    }
                }
                syn::Pat::Wild(_) => {
                    parts.push(format!(
                        "[&]({}){} {{ return {}; }}",
                        if borrow_payload {
                            if borrow_payload_mutably {
                                "auto&"
                            } else {
                                "const auto&"
                            }
                        } else {
                            "auto&&"
                        },
                        lambda_return_annotation,
                        arm_value
                    ));
                }
                syn::Pat::Ident(pi) => {
                    if pi.by_ref.is_none() && pi.mutability.is_none() && pi.subpat.is_none() {
                        let ident_name = pi.ident.to_string();
                        let data_enum_variant_path = variant_ctx.and_then(|ctx| {
                            self.data_enum_variants_by_enum
                                .get(&ctx.enum_name)
                                .filter(|variants| variants.contains(&ident_name))
                                .and_then(|_| {
                                    syn::parse_str::<syn::Path>(&format!(
                                        "{}::{}",
                                        ctx.enum_name, ident_name
                                    ))
                                    .ok()
                                })
                        });
                        if let Some(variant_path) = data_enum_variant_path {
                            let cpp_type = scoped.visit_pattern_cpp_type(
                                &variant_path,
                                variant_ctx,
                                Some("_m"),
                            );
                            if let Some((_, guard)) = &arm.guard {
                                let guard_str = scoped.emit_expr_to_string(guard);
                                let visit_param = if borrow_payload {
                                    if borrow_payload_mutably {
                                        format!("{}& _v", cpp_type)
                                    } else {
                                        format!("const {}& _v", cpp_type)
                                    }
                                } else {
                                    format!("{}&& _v", cpp_type)
                                };
                                parts.push(format!(
                                    "[&]({}){} {{ if ({}) return {}; return {}; }}",
                                    visit_param,
                                    lambda_return_annotation,
                                    guard_str,
                                    arm_value,
                                    scoped
                                        .match_expr_unreachable_fallback_with_expected(expected_ty),
                                ));
                            } else {
                                let visit_param = if borrow_payload {
                                    if borrow_payload_mutably {
                                        format!("{}&", cpp_type)
                                    } else {
                                        format!("const {}&", cpp_type)
                                    }
                                } else {
                                    format!("{}&&", cpp_type)
                                };
                                parts.push(format!(
                                    "[&]({}){} {{ return {}; }}",
                                    visit_param, lambda_return_annotation, arm_value
                                ));
                            }
                            scoped.pop_pattern_ref_binding_scope();
                            if pushed_binding_scope {
                                scoped.local_cpp_bindings.pop();
                                scoped.local_reference_bindings.pop();
                                scoped.rebind_reference_pointer_bindings.pop();
                                scoped.local_const_bindings.pop();
                                scoped.local_shadowed_binding_types.pop();
                                scoped.local_bindings.pop();
                            }
                            continue;
                        }
                    }
                    let cpp_name = escape_cpp_keyword(&pi.ident.to_string());
                    let visit_param = if borrow_payload {
                        if borrow_payload_mutably {
                            format!("auto& {}", cpp_name)
                        } else {
                            format!("const auto& {}", cpp_name)
                        }
                    } else {
                        // Identifier patterns in Rust bind the whole scrutinee value.
                        // In `std::visit`, use the owning enum type instead of the
                        // payload-alternative type so member calls like `e.type_str()`
                        // resolve on the enum (`Value`, `DeValue`, ...).
                        format!("std::remove_cvref_t<decltype(_m)> {}", cpp_name)
                    };
                    parts.push(format!(
                        "[&]({}){} {{ return {}; }}",
                        visit_param, lambda_return_annotation, arm_value
                    ));
                }
                _ => {
                    parts.push(format!(
                        "[&]({}){} {{ return {}; }}",
                        if borrow_payload {
                            if borrow_payload_mutably {
                                "auto&"
                            } else {
                                "const auto&"
                            }
                        } else {
                            "auto&&"
                        },
                        lambda_return_annotation,
                        scoped.match_expr_unreachable_fallback_with_expected(expected_ty)
                    ));
                }
            }
            if pushed_binding_scope {
                scoped.local_cpp_bindings.pop();
                scoped.local_reference_bindings.pop();
                scoped.rebind_reference_pointer_bindings.pop();
                scoped.local_const_bindings.pop();
                scoped.local_shadowed_binding_types.pop();
                scoped.local_bindings.pop();
            }
            scoped.pop_pattern_ref_binding_scope();
        }
        parts
            .iter()
            .map(|p| {
                // A std::visit lambda whose parameter type was rewritten to the
                // arg-deducing form `Base<__Vs...>` needs a C++20 template-lambda
                // header so the pack is deduced from the actual alternative.
                if p.contains("__Vs") {
                    p.replacen("[&](", "[&]<typename... __Vs>(", 1)
                } else {
                    p.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Rewrite a variant struct type `Base<args...>` into `Base<__Vs...>` so a
    /// `std::visit` lambda DEDUCES the enum's type arguments instead of spelling
    /// them out — spelling them out leaks generic parameters that aren't in scope
    /// at the match site (e.g. a method-introduced `Q` from `map.entry_ref(&T)`).
    /// Pairs with the `__Vs` post-process above. Non-templated types are unchanged.
    pub(super) fn visit_variant_deduced_type(&self, cpp_type: String) -> String {
        // Only deduce args for a CONCRETE user variant-struct type spelled with
        // explicit args (`Enum_Variant<args...>`). Leave already-generic forms
        // alone: std type traits (`std::variant_alternative_t<0, decltype(_m)>`),
        // anything routed through `decltype(...)`, etc. — they don't leak params.
        if cpp_type.starts_with("std::")
            || cpp_type.contains("variant_alternative")
            || cpp_type.contains("decltype(")
        {
            return cpp_type;
        }
        match cpp_type.find('<') {
            Some(idx) if cpp_type.trim_end().ends_with('>') => {
                format!("{}<__Vs...>", &cpp_type[..idx])
            }
            _ => cpp_type,
        }
    }

    pub(super) fn try_emit_runtime_entry_probe_expr(&self, expr: &syn::Expr) -> Option<String> {
        let syn::Expr::MethodCall(mc) = self.peel_paren_group_expr(expr) else {
            return None;
        };
        if mc.method != "entry" || mc.args.len() != 1 {
            return None;
        }
        let receiver = self.emit_expr_to_string(&mc.receiver);
        let key = self.emit_expr_maybe_move(&mc.args[0]);
        Some(format!(
            "rusty::detail::make_entry_probe({}, {})",
            receiver, key
        ))
    }

    pub(super) fn try_emit_runtime_entry_probe_for_pattern(
        &self,
        pat: &syn::Pat,
        expr: &syn::Expr,
    ) -> Option<String> {
        if !self.pat_uses_runtime_entry_variant(pat) {
            return None;
        }
        self.try_emit_runtime_entry_probe_expr(expr)
    }

    pub(super) fn try_emit_runtime_entry_probe_for_match_arms(
        &self,
        arms: &[syn::Arm],
        expr: &syn::Expr,
    ) -> Option<String> {
        if !arms
            .iter()
            .any(|arm| self.pat_uses_runtime_entry_variant(&arm.pat))
        {
            return None;
        }
        self.try_emit_runtime_entry_probe_expr(expr)
    }

    /// Emit a tuple-scrutinee match expression as overloaded std::visit lambdas.
    /// This handles patterns like:
    /// `(E::A(x), E::A(y)) => x == y`
    pub(super) fn emit_match_expr_visit_tuple(
        &self,
        arms: &[syn::Arm],
        arity: usize,
        variant_ctx: Option<&VariantTypeContext>,
        expected_ty: Option<&syn::Type>,
    ) -> String {
        let mut parts = Vec::new();
        for arm in arms {
            let body = {
                let emitted = self.emit_expr_to_string_with_expected(&arm.body, expected_ty);
                self.maybe_wrap_variant_constructor_with_expected_enum(
                    &arm.body,
                    emitted,
                    expected_ty,
                )
            };
            match &arm.pat {
                syn::Pat::Tuple(tuple_pat) if tuple_pat.elems.len() == arity => {
                    let mut params = Vec::new();
                    let mut binding_stmts = Vec::new();
                    let mut supported = true;
                    for (idx, elem_pat) in tuple_pat.elems.iter().enumerate() {
                        if !self.emit_tuple_visit_subpattern(
                            elem_pat,
                            idx,
                            &mut params,
                            &mut binding_stmts,
                            variant_ctx,
                        ) {
                            supported = false;
                            break;
                        }
                    }

                    if !supported {
                        parts.push(format!(
                            "[&](const auto&...) {{ return {}; }}",
                            self.match_expr_unreachable_fallback_with_expected(expected_ty)
                        ));
                        continue;
                    }

                    if let Some((_, guard)) = &arm.guard {
                        let guard_str = self.emit_expr_to_string(guard);
                        parts.push(format!(
                            "[&]({}) {{ {} if ({}) return {}; return {}; }}",
                            params.join(", "),
                            binding_stmts.join("\n"),
                            guard_str,
                            body,
                            self.match_expr_unreachable_fallback_with_expected(expected_ty)
                        ));
                    } else {
                        parts.push(format!(
                            "[&]({}) {{ {} return {}; }}",
                            params.join(", "),
                            binding_stmts.join("\n"),
                            body
                        ));
                    }
                }
                syn::Pat::Wild(_) => {
                    parts.push(format!("[&](const auto&...) {{ return {}; }}", body));
                }
                _ => {
                    parts.push(format!(
                        "[&](const auto&...) {{ return {}; }}",
                        self.match_expr_unreachable_fallback_with_expected(expected_ty)
                    ));
                }
            }
        }
        parts.join(", ")
    }

    /// Emit a tuple-scrutinee match as value-condition if/else chain.
    /// This is used for non-variant tuple heads like `(bool, bool)` where
    /// `std::visit` is not valid.
    pub(super) fn emit_match_expr_tuple_value_conditions(
        &self,
        tuple_scrutinee: &syn::ExprTuple,
        arms: &[syn::Arm],
        expected_ty: Option<&syn::Type>,
    ) -> String {
        let concrete_expected_ty = expected_ty.filter(|ty| {
            !self
                .expected_lambda_return_annotation(Some(*ty), true)
                .is_empty()
        });
        let inferred_match_ty = if concrete_expected_ty.is_none() {
            self.infer_match_arms_common_type_with_tuple_scrutinee(tuple_scrutinee, arms)
                .or_else(|| self.infer_match_arms_common_type(arms))
        } else {
            None
        }
        .and_then(|ty| {
            if self.type_contains_tuple_placeholder_marker(&ty)
                || self.type_maps_to_branch_local_decltype(&ty)
            {
                None
            } else {
                Some(ty)
            }
        });
        let match_expected_ty = concrete_expected_ty.or(inferred_match_ty.as_ref());
        let lambda_return_annotation =
            self.expected_lambda_return_annotation(match_expected_ty, true);
        let mut out = format!("[&](){} {{ ", lambda_return_annotation);
        for (idx, expr) in tuple_scrutinee.elems.iter().enumerate() {
            if let syn::Expr::Reference(reference_expr) = self.peel_paren_group_expr(expr) {
                let reference_target = self.peel_paren_group_expr(&reference_expr.expr);
                if !self.is_stable_reference_lvalue_expr(reference_target) {
                    let tmp_name = format!("_m{}_tmp", idx);
                    let target_value = self.emit_expr_to_string(reference_target);
                    out.push_str(&format!("auto {} = {}; ", tmp_name, target_value));
                    out.push_str(&format!("auto&& _m{} = &{}; ", idx, tmp_name));
                    continue;
                }
            }
            let value = self.emit_expr_to_string(expr);
            out.push_str(&format!("auto&& _m{} = {}; ", idx, value));
        }

        for arm in arms {
            match &arm.pat {
                syn::Pat::Tuple(tuple_pat)
                    if tuple_pat.elems.len() == tuple_scrutinee.elems.len() =>
                {
                    let mut conditions = Vec::new();
                    let mut bindings = Vec::new();
                    let mut binding_map = HashMap::new();
                    let mut supported = true;
                    for (idx, elem_pat) in tuple_pat.elems.iter().enumerate() {
                        let value_name = format!("_m{}", idx);
                        let mut elem_bindings = Vec::new();
                        let Some(cond) = self
                            .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                                elem_pat,
                                &value_name,
                                &mut elem_bindings,
                                &mut binding_map,
                                None,
                            )
                        else {
                            supported = false;
                            break;
                        };
                        bindings.extend(elem_bindings);
                        if let Some(cond_expr) = cond {
                            conditions.push(cond_expr);
                        }
                    }
                    if !supported {
                        continue;
                    }
                    let cond_expr = if conditions.is_empty() {
                        "true".to_string()
                    } else {
                        conditions.join(" && ")
                    };
                    if arm.guard.is_none() {
                        Self::allow_runtime_match_binding_payload_moves(
                            &mut bindings,
                            &mut binding_map,
                        );
                    }
                    let body = {
                        let emitted = self.emit_expr_with_try_style_binding_scope(
                            &arm.body,
                            match_expected_ty,
                            &binding_map,
                        );
                        self.maybe_wrap_variant_constructor_with_expected_enum(
                            &arm.body,
                            emitted,
                            match_expected_ty,
                        )
                    };
                    let diverging = self.is_expr_diverging(&arm.body);
                    let body_is_return_expr = body.trim_start().starts_with("return ");
                    let ret_prefix = if diverging || body_is_return_expr {
                        ""
                    } else {
                        "return "
                    };
                    out.push_str(&format!("if ({}) {{ ", cond_expr));
                    for binding in bindings {
                        out.push_str(&binding);
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
                        let emitted = self.emit_expr_with_try_style_binding_scope(
                            &arm.body,
                            match_expected_ty,
                            &binding_map,
                        );
                        self.maybe_wrap_variant_constructor_with_expected_enum(
                            &arm.body,
                            emitted,
                            match_expected_ty,
                        )
                    };
                    let diverging = self.is_expr_diverging(&arm.body);
                    let body_is_return_expr = body.trim_start().starts_with("return ");
                    let ret_prefix = if diverging || body_is_return_expr {
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
                syn::Pat::Or(or_pat) => {
                    let mut emitted_any = false;
                    for case in &or_pat.cases {
                        match case {
                            syn::Pat::Tuple(tuple_pat)
                                if tuple_pat.elems.len() == tuple_scrutinee.elems.len() =>
                            {
                                let mut conditions = Vec::new();
                                let mut bindings = Vec::new();
                                let mut binding_map = HashMap::new();
                                let mut supported = true;
                                let mut tuple_case_conditions = Vec::new();
                                for (idx, elem_pat) in tuple_pat.elems.iter().enumerate() {
                                    let value_name = format!("_m{}", idx);
                                    let mut elem_bindings = Vec::new();
                                    let Some(cond) = self
                                        .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                                            elem_pat,
                                            &value_name,
                                            &mut elem_bindings,
                                            &mut binding_map,
                                            None,
                                        )
                                    else {
                                        supported = false;
                                        break;
                                    };
                                    bindings.extend(elem_bindings);
                                    if let Some(cond_expr) = cond {
                                        tuple_case_conditions.push(cond_expr);
                                    }
                                }
                                if !supported {
                                    continue;
                                }
                                conditions.push(if tuple_case_conditions.is_empty() {
                                    "true".to_string()
                                } else {
                                    tuple_case_conditions.join(" && ")
                                });
                                let cond_expr = conditions.join(" && ");
                                if arm.guard.is_none() {
                                    Self::allow_runtime_match_binding_payload_moves(
                                        &mut bindings,
                                        &mut binding_map,
                                    );
                                }
                                let body = {
                                    let emitted = self.emit_expr_with_try_style_binding_scope(
                                        &arm.body,
                                        match_expected_ty,
                                        &binding_map,
                                    );
                                    self.maybe_wrap_variant_constructor_with_expected_enum(
                                        &arm.body,
                                        emitted,
                                        match_expected_ty,
                                    )
                                };
                                let diverging = self.is_expr_diverging(&arm.body);
                                let body_is_return_expr = body.trim_start().starts_with("return ");
                                let ret_prefix = if diverging || body_is_return_expr {
                                    ""
                                } else {
                                    "return "
                                };
                                out.push_str(&format!("if ({}) {{ ", cond_expr));
                                for binding in bindings {
                                    out.push_str(&binding);
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
                                emitted_any = true;
                            }
                            syn::Pat::Wild(_) => {
                                let binding_map = HashMap::new();
                                let body = {
                                    let emitted = self.emit_expr_with_try_style_binding_scope(
                                        &arm.body,
                                        match_expected_ty,
                                        &binding_map,
                                    );
                                    self.maybe_wrap_variant_constructor_with_expected_enum(
                                        &arm.body,
                                        emitted,
                                        match_expected_ty,
                                    )
                                };
                                let diverging = self.is_expr_diverging(&arm.body);
                                let body_is_return_expr = body.trim_start().starts_with("return ");
                                let ret_prefix = if diverging || body_is_return_expr {
                                    ""
                                } else {
                                    "return "
                                };
                                out.push_str("if (true) { ");
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
                                emitted_any = true;
                                break;
                            }
                            syn::Pat::Ident(pi) if pi.ident == "_" => {
                                let binding_map = HashMap::new();
                                let body = {
                                    let emitted = self.emit_expr_with_try_style_binding_scope(
                                        &arm.body,
                                        match_expected_ty,
                                        &binding_map,
                                    );
                                    self.maybe_wrap_variant_constructor_with_expected_enum(
                                        &arm.body,
                                        emitted,
                                        match_expected_ty,
                                    )
                                };
                                let diverging = self.is_expr_diverging(&arm.body);
                                let body_is_return_expr = body.trim_start().starts_with("return ");
                                let ret_prefix = if diverging || body_is_return_expr {
                                    ""
                                } else {
                                    "return "
                                };
                                out.push_str("if (true) { ");
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
                                emitted_any = true;
                                break;
                            }
                            _ => {}
                        }
                    }
                    if !emitted_any {
                        continue;
                    }
                }
                _ => {}
            }
        }

        if let Some(match_expected_ty) = match_expected_ty {
            out.push_str(&format!(
                "return {}; }}()",
                self.match_expr_unreachable_fallback_with_expected(Some(match_expected_ty))
            ));
        } else {
            out.push_str("rusty::intrinsics::unreachable(); }()");
        }
        out
    }

    pub(super) fn emit_match_expr_tuple_value_conditions_for_scrutinee_expr(
        &self,
        tuple_scrutinee_expr: &syn::Expr,
        arity: usize,
        arms: &[syn::Arm],
        expected_ty: Option<&syn::Type>,
    ) -> String {
        let concrete_expected_ty = expected_ty.filter(|ty| {
            !self
                .expected_lambda_return_annotation(Some(*ty), true)
                .is_empty()
        });
        let inferred_match_ty = if concrete_expected_ty.is_none() {
            self.infer_match_arms_common_type(arms)
        } else {
            None
        }
        .and_then(|ty| {
            if self.type_contains_tuple_placeholder_marker(&ty)
                || self.type_maps_to_branch_local_decltype(&ty)
            {
                None
            } else {
                Some(ty)
            }
        });
        let match_expected_ty = concrete_expected_ty.or(inferred_match_ty.as_ref());
        let lambda_return_annotation =
            self.expected_lambda_return_annotation(match_expected_ty, true);
        let mut out = format!("[&](){} {{ ", lambda_return_annotation);
        let scrutinee = self.emit_expr_to_string(tuple_scrutinee_expr);
        out.push_str(&format!("auto&& _m_tuple = {}; ", scrutinee));
        for idx in 0..arity {
            out.push_str(&format!(
                "auto&& _m{} = std::get<{}>(rusty::detail::deref_if_pointer(_m_tuple)); ",
                idx, idx
            ));
        }

        for arm in arms {
            match &arm.pat {
                syn::Pat::Tuple(tuple_pat) if tuple_pat.elems.len() == arity => {
                    let mut conditions = Vec::new();
                    let mut bindings = Vec::new();
                    let mut binding_map = HashMap::new();
                    let mut supported = true;
                    for (idx, elem_pat) in tuple_pat.elems.iter().enumerate() {
                        let value_name = format!("_m{}", idx);
                        let mut elem_bindings = Vec::new();
                        let Some(cond) = self
                            .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                                elem_pat,
                                &value_name,
                                &mut elem_bindings,
                                &mut binding_map,
                                None,
                            )
                        else {
                            supported = false;
                            break;
                        };
                        bindings.extend(elem_bindings);
                        if let Some(cond_expr) = cond {
                            conditions.push(cond_expr);
                        }
                    }
                    if !supported {
                        continue;
                    }
                    let cond_expr = if conditions.is_empty() {
                        "true".to_string()
                    } else {
                        conditions.join(" && ")
                    };
                    if arm.guard.is_none() {
                        // Item 11: for guarded-free arms, allow payload
                        // extraction to MOVE out of the scrutinee instead
                        // of binding `std::as_const(...).unwrap()`. The
                        // const overload of Option::unwrap returns a
                        // const reference, which then can't satisfy
                        // non-const member calls in the arm body (e.g.
                        // `split.forget_node_type()` on a SplitResult).
                        // Per-arm conditions are mutually exclusive in
                        // value-pattern matches, so moving out is safe.
                        Self::allow_runtime_match_binding_payload_moves(
                            &mut bindings,
                            &mut binding_map,
                        );
                    }
                    let body = {
                        let emitted = self.emit_expr_with_try_style_binding_scope(
                            &arm.body,
                            match_expected_ty,
                            &binding_map,
                        );
                        self.maybe_wrap_variant_constructor_with_expected_enum(
                            &arm.body,
                            emitted,
                            match_expected_ty,
                        )
                    };
                    let diverging = self.is_expr_diverging(&arm.body);
                    let body_is_return_expr = body.trim_start().starts_with("return ");
                    let ret_prefix = if diverging || body_is_return_expr {
                        ""
                    } else {
                        "return "
                    };
                    out.push_str(&format!("if ({}) {{ ", cond_expr));
                    for binding in bindings {
                        out.push_str(&binding);
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
                        let emitted = self.emit_expr_with_try_style_binding_scope(
                            &arm.body,
                            match_expected_ty,
                            &binding_map,
                        );
                        self.maybe_wrap_variant_constructor_with_expected_enum(
                            &arm.body,
                            emitted,
                            match_expected_ty,
                        )
                    };
                    let diverging = self.is_expr_diverging(&arm.body);
                    let body_is_return_expr = body.trim_start().starts_with("return ");
                    let ret_prefix = if diverging || body_is_return_expr {
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
                syn::Pat::Or(or_pat) => {
                    let mut emitted_any = false;
                    for case in &or_pat.cases {
                        match case {
                            syn::Pat::Tuple(tuple_pat) if tuple_pat.elems.len() == arity => {
                                let mut conditions = Vec::new();
                                let mut bindings = Vec::new();
                                let mut binding_map = HashMap::new();
                                let mut supported = true;
                                for (idx, elem_pat) in tuple_pat.elems.iter().enumerate() {
                                    let value_name = format!("_m{}", idx);
                                    let mut elem_bindings = Vec::new();
                                    let Some(cond) = self
                                        .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                                            elem_pat,
                                            &value_name,
                                            &mut elem_bindings,
                                            &mut binding_map,
                                            None,
                                        )
                                    else {
                                        supported = false;
                                        break;
                                    };
                                    bindings.extend(elem_bindings);
                                    if let Some(cond_expr) = cond {
                                        conditions.push(cond_expr);
                                    }
                                }
                                if !supported {
                                    continue;
                                }
                                let cond_expr = if conditions.is_empty() {
                                    "true".to_string()
                                } else {
                                    conditions.join(" && ")
                                };
                                if arm.guard.is_none() {
                                    Self::allow_runtime_match_binding_payload_moves(
                                        &mut bindings,
                                        &mut binding_map,
                                    );
                                }
                                let body = {
                                    let emitted = self.emit_expr_with_try_style_binding_scope(
                                        &arm.body,
                                        match_expected_ty,
                                        &binding_map,
                                    );
                                    self.maybe_wrap_variant_constructor_with_expected_enum(
                                        &arm.body,
                                        emitted,
                                        match_expected_ty,
                                    )
                                };
                                let diverging = self.is_expr_diverging(&arm.body);
                                let body_is_return_expr = body.trim_start().starts_with("return ");
                                let ret_prefix = if diverging || body_is_return_expr {
                                    ""
                                } else {
                                    "return "
                                };
                                out.push_str(&format!("if ({}) {{ ", cond_expr));
                                for binding in bindings {
                                    out.push_str(&binding);
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
                                emitted_any = true;
                            }
                            syn::Pat::Wild(_) => {
                                let binding_map = HashMap::new();
                                let body = {
                                    let emitted = self.emit_expr_with_try_style_binding_scope(
                                        &arm.body,
                                        match_expected_ty,
                                        &binding_map,
                                    );
                                    self.maybe_wrap_variant_constructor_with_expected_enum(
                                        &arm.body,
                                        emitted,
                                        match_expected_ty,
                                    )
                                };
                                let diverging = self.is_expr_diverging(&arm.body);
                                let body_is_return_expr = body.trim_start().starts_with("return ");
                                let ret_prefix = if diverging || body_is_return_expr {
                                    ""
                                } else {
                                    "return "
                                };
                                out.push_str("if (true) { ");
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
                                emitted_any = true;
                                break;
                            }
                            syn::Pat::Ident(pi) if pi.ident == "_" => {
                                let binding_map = HashMap::new();
                                let body = {
                                    let emitted = self.emit_expr_with_try_style_binding_scope(
                                        &arm.body,
                                        match_expected_ty,
                                        &binding_map,
                                    );
                                    self.maybe_wrap_variant_constructor_with_expected_enum(
                                        &arm.body,
                                        emitted,
                                        match_expected_ty,
                                    )
                                };
                                let diverging = self.is_expr_diverging(&arm.body);
                                let body_is_return_expr = body.trim_start().starts_with("return ");
                                let ret_prefix = if diverging || body_is_return_expr {
                                    ""
                                } else {
                                    "return "
                                };
                                out.push_str("if (true) { ");
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
                                emitted_any = true;
                                break;
                            }
                            _ => {}
                        }
                    }
                    if !emitted_any {
                        continue;
                    }
                }
                _ => {}
            }
        }

        if let Some(match_expected_ty) = match_expected_ty {
            out.push_str(&format!(
                "return {}; }}()",
                self.match_expr_unreachable_fallback_with_expected(Some(match_expected_ty))
            ));
        } else {
            out.push_str("rusty::intrinsics::unreachable(); }()");
        }
        out
    }

    pub(super) fn try_emit_if_option_some_none_auto_return(
        &mut self,
        if_expr: &syn::ExprIf,
        first: bool,
    ) -> bool {
        if !self.in_value_return_scope() || self.current_return_type_hint().is_some() {
            return false;
        }
        let Some((_, else_branch)) = &if_expr.else_branch else {
            return false;
        };
        let Some(then_expr) = self.extract_single_expr_from_block(&if_expr.then_branch) else {
            return false;
        };
        let Some(else_expr) = self.extract_single_value_expr(else_branch) else {
            return false;
        };

        let then_is_none = self.expr_is_option_none_path(then_expr);
        let else_is_none = self.expr_is_option_none_path(else_expr);
        if !(then_is_none ^ else_is_none) {
            return false;
        }

        let then_some_arg = self.extract_option_some_call_arg(then_expr);
        let else_some_arg = self.extract_option_some_call_arg(else_expr);
        if then_some_arg.is_none() && else_some_arg.is_none() {
            return false;
        }

        let typed_none_from_peer =
            |none_expr: &syn::Expr, peer_expr: &syn::Expr| -> Option<String> {
                if !self.expr_is_option_none_path(none_expr) {
                    return None;
                }
                let peer_some_arg = self.extract_option_some_call_arg(peer_expr)?;
                let peer_some_cpp = self.emit_expr_to_string(peer_some_arg);
                Some(format!(
                    "decltype(rusty::Some({}))(rusty::None)",
                    peer_some_cpp
                ))
            };

        let then_value = typed_none_from_peer(then_expr, else_expr).unwrap_or_else(|| {
            self.emit_expr_to_string_with_expected(then_expr, self.current_return_type_hint())
        });
        let else_value = typed_none_from_peer(else_expr, then_expr).unwrap_or_else(|| {
            self.emit_expr_to_string_with_expected(else_expr, self.current_return_type_hint())
        });

        let cond = self.emit_expr_to_string(&if_expr.cond);
        if first {
            self.writeln(&format!("if ({}) {{", cond));
        } else {
            self.output.push_str(&format!("if ({}) {{\n", cond));
        }
        self.indent += 1;
        let keyword = if self.in_async { "co_return" } else { "return" };
        self.writeln(&format!("{} {};", keyword, then_value));
        self.indent -= 1;
        self.writeln("} else {");
        self.indent += 1;
        self.writeln(&format!("{} {};", keyword, else_value));
        self.indent -= 1;
        self.writeln("}");
        true
    }

    pub(super) fn try_emit_local_match_break_initializer(
        &mut self,
        local: &syn::Local,
        cpp_name: &str,
        decl_type: &str,
        inferred_binding_ty: Option<&syn::Type>,
    ) -> bool {
        let Some(init) = &local.init else {
            return false;
        };
        let syn::Expr::Match(match_expr) = self.peel_paren_group_expr(&init.expr) else {
            return false;
        };
        if match_expr.arms.len() != 2 || match_expr.arms.iter().any(|arm| arm.guard.is_some()) {
            return false;
        }

        let mut break_arm: Option<&syn::Arm> = None;
        let mut success_arm: Option<&syn::Arm> = None;
        for arm in &match_expr.arms {
            if self.expr_is_unlabeled_break_without_value(&arm.body) {
                if break_arm.is_some() {
                    return false;
                }
                break_arm = Some(arm);
            } else {
                if success_arm.is_some() {
                    return false;
                }
                success_arm = Some(arm);
            }
        }
        let Some(break_arm) = break_arm else {
            return false;
        };
        let Some(success_arm) = success_arm else {
            return false;
        };
        let break_arm_prefix_stmts: Vec<syn::Stmt> =
            match self.peel_paren_group_expr(&break_arm.body) {
                syn::Expr::Block(block_expr) => {
                    if block_expr.block.stmts.is_empty() {
                        return false;
                    }
                    let Some(syn::Stmt::Expr(tail_expr, _)) = block_expr.block.stmts.last() else {
                        return false;
                    };
                    if !self.expr_is_unlabeled_break_without_value(tail_expr) {
                        return false;
                    }
                    block_expr.block.stmts[..block_expr.block.stmts.len() - 1].to_vec()
                }
                _ => Vec::new(),
            };

        let variant_ctx = self.infer_variant_type_context_from_expr(&match_expr.expr);
        let scrutinee_var = self.allocate_local_cpp_name("_m");
        let success_matched_var = self.allocate_local_cpp_name("_mv");
        let break_matched_var = self.allocate_local_cpp_name("_mv_break");
        let Some((success_cond, success_bindings, success_binding_map, success_unwrap_method)) =
            self.runtime_try_pattern_details(
                &success_arm.pat,
                variant_ctx.as_ref(),
                &scrutinee_var,
                &success_matched_var,
            )
        else {
            return false;
        };
        let Some((break_cond, break_bindings, break_binding_map, _)) = self
            .runtime_try_pattern_details(
                &break_arm.pat,
                variant_ctx.as_ref(),
                &scrutinee_var,
                &break_matched_var,
            )
        else {
            return false;
        };
        if !break_bindings.is_empty() || !break_binding_map.is_empty() {
            return false;
        }
        if break_cond == "true" {
            return false;
        }

        let expected_ty = get_local_type(local).or(inferred_binding_ty);
        let local_is_consumed = matches!(
            &local.pat,
            syn::Pat::Ident(pat_ident)
                if self
                    .consuming_method_receiver_vars
                    .contains(&pat_ident.ident.to_string())
        );
        let mut success_expr = {
            let emitted = self.emit_expr_with_try_style_binding_scope(
                &success_arm.body,
                expected_ty,
                &success_binding_map,
            );
            self.maybe_wrap_variant_constructor_with_expected_enum(
                &success_arm.body,
                emitted,
                expected_ty,
            )
        };
        if local_is_consumed {
            success_expr = format!("std::move({})", success_expr);
        }

        let scrutinee =
            self.emit_expr_to_string_with_variant_ctx(&match_expr.expr, variant_ctx.as_ref());
        let match_value_cpp_type = if let Some(unwrap_method) = success_unwrap_method {
            Some(format!(
                "std::remove_cvref_t<decltype(({}.{}()))>",
                scrutinee_var, unwrap_method
            ))
        } else if let Some(expected_ty) = expected_ty {
            let mapped = self.map_type(expected_ty);
            if mapped.contains("/* TODO") || type_string_has_auto_placeholder(&mapped) {
                None
            } else {
                Some(format!("std::remove_cvref_t<{}>", mapped))
            }
        } else {
            None
        };
        let Some(match_value_cpp_type) = match_value_cpp_type else {
            return false;
        };
        let match_value_name = self.allocate_local_cpp_name("_match_value");
        self.writeln(&format!("auto&& {} = {};", scrutinee_var, scrutinee));
        self.writeln(&format!(
            "std::optional<{}> {};",
            match_value_cpp_type, match_value_name
        ));
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!("if ({}) {{", break_cond));
        self.indent += 1;
        for stmt in &break_arm_prefix_stmts {
            self.emit_stmt(stmt, false);
        }
        self.writeln("break;");
        self.indent -= 1;
        self.writeln("}");
        if success_cond != "true" {
            self.writeln(&format!(
                "if (!({})) {{ rusty::intrinsics::unreachable(); }}",
                success_cond
            ));
        }
        for binding in success_bindings {
            self.writeln(&binding);
        }
        self.writeln(&format!("{}.emplace({});", match_value_name, success_expr));
        self.indent -= 1;
        self.writeln("}");
        self.writeln(&format!(
            "{} {} = std::move({}).value();",
            decl_type, cpp_name, match_value_name
        ));
        true
    }

    pub(super) fn emit_match_assign_body_to_optional_result(
        &mut self,
        result_var: &str,
        body: &syn::Expr,
        expected_ty: Option<&syn::Type>,
    ) {
        match self.peel_paren_group_expr(body) {
            syn::Expr::Return(_) | syn::Expr::Break(_) | syn::Expr::Continue(_) => {
                self.emit_arm_body(body);
            }
            syn::Expr::Match(inner_match) if self.expr_contains_early_return_or_try(body) => {
                if !self.try_emit_match_assign_to_optional_result(
                    result_var,
                    inner_match,
                    expected_ty,
                ) {
                    let value = self.emit_expr_to_string_with_expected(body, expected_ty);
                    self.writeln(&format!("{}.emplace({});", result_var, value));
                }
            }
            syn::Expr::Block(block_expr) => {
                self.writeln("{");
                self.indent += 1;
                self.push_transient_statement_scope();
                let (tail, prefix) = match block_expr.block.stmts.split_last() {
                    Some((tail, prefix)) => (Some(tail), prefix),
                    None => (None, &[][..]),
                };
                for stmt in prefix {
                    self.emit_stmt(stmt, false);
                }
                match tail {
                    Some(syn::Stmt::Expr(expr, None)) => {
                        self.emit_match_assign_body_to_optional_result(
                            result_var,
                            expr,
                            expected_ty,
                        );
                    }
                    Some(stmt) => self.emit_stmt(stmt, false),
                    None => {}
                }
                self.pop_transient_statement_scope();
                self.indent -= 1;
                self.writeln("}");
            }
            _ => {
                let value = self.emit_expr_to_string_with_expected(body, expected_ty);
                let value = self.maybe_wrap_variant_constructor_with_expected_enum(
                    body,
                    value,
                    expected_ty,
                );
                self.writeln(&format!("{}.emplace({});", result_var, value));
            }
        }
    }

    pub(super) fn try_emit_match_assign_to_optional_result(
        &mut self,
        result_var: &str,
        match_expr: &syn::ExprMatch,
        expected_ty: Option<&syn::Type>,
    ) -> bool {
        if match_expr.arms.is_empty() {
            return false;
        }
        if !self.match_assign_to_optional_result_is_supported(match_expr) {
            return false;
        }

        let variant_ctx = self.infer_variant_type_context_from_expr(&match_expr.expr);
        let scrutinee_var = self.reserve_synthetic_cpp_name("_m");
        let match_scrutinee_ty = self
            .infer_simple_expr_type(&match_expr.expr)
            .or_else(|| self.infer_local_binding_type_from_initializer(&match_expr.expr));
        let mut arm_plans = Vec::with_capacity(match_expr.arms.len());

        for (idx, arm) in match_expr.arms.iter().enumerate() {
            let matched_var = self.reserve_synthetic_cpp_name(&format!("_mv{}", idx));
            let Some((condition, bindings, binding_map, _)) = self.runtime_try_pattern_details(
                &arm.pat,
                variant_ctx.as_ref(),
                &scrutinee_var,
                &matched_var,
            ) else {
                return false;
            };
            let mut binding_types = HashMap::new();
            if let Some(scrutinee_ty) = match_scrutinee_ty.as_ref() {
                self.bind_pattern_types_into_env(&arm.pat, scrutinee_ty, &mut binding_types);
                binding_types.retain(|name, _| binding_map.contains_key(name));
            }
            arm_plans.push((condition, bindings, binding_map, binding_types));
        }

        let scrutinee =
            self.emit_expr_to_string_with_variant_ctx(&match_expr.expr, variant_ctx.as_ref());
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!("auto&& {} = {};", scrutinee_var, scrutinee));
        self.writeln("bool _m_matched = false;");

        for (arm, (condition, bindings, binding_map, binding_types)) in
            match_expr.arms.iter().zip(arm_plans.into_iter())
        {
            self.writeln("if (!_m_matched) {");
            self.indent += 1;
            self.writeln(&format!("if ({}) {{", condition));
            self.indent += 1;
            for binding in bindings {
                self.writeln(&binding);
            }
            let pushed_binding_scope =
                self.push_local_cpp_binding_scope_with_types(&binding_map, Some(&binding_types));
            if let Some((_, guard)) = &arm.guard {
                let guard_condition = self.emit_expr_to_string(guard);
                self.writeln(&format!("if ({}) {{", guard_condition));
                self.indent += 1;
                self.emit_match_assign_body_to_optional_result(result_var, &arm.body, expected_ty);
                self.writeln("_m_matched = true;");
                self.indent -= 1;
                self.writeln("}");
            } else {
                self.emit_match_assign_body_to_optional_result(result_var, &arm.body, expected_ty);
                self.writeln("_m_matched = true;");
            }
            self.pop_local_cpp_binding_scope(pushed_binding_scope);
            self.indent -= 1;
            self.writeln("}");
            self.indent -= 1;
            self.writeln("}");
        }

        self.writeln("if (!_m_matched) { rusty::intrinsics::unreachable(); }");
        self.indent -= 1;
        self.writeln("}");
        true
    }

    /// Item 11 completion. Handles `let pat = match tuple_scrutinee { … }`
    /// where some arms have a non-local `return X` body and one arm is the
    /// success arm that yields a tuple matching `pat`. Required because the
    /// existing IIFE-based lowering can't propagate `return` out of the
    /// lambda — the lambda absorbs it as a lambda-local return, and
    /// diverging arm types break `auto` return deduction.
    ///
    /// Emits statement-level shape:
    ///   auto&& _let_match_tuple = scrutinee;
    ///   auto&& _let_match_m0 = std::get<0>(_let_match_tuple);
    ///   auto&& _let_match_mN = std::get<N>(_let_match_tuple);
    ///   if (cond_diverging) {
    ///       <bindings of diverging arm pattern>
    ///       <emit arm body — already a Return expr in Rust>;
    ///   }
    ///   auto _let_match_result = [&]() {
    ///       <bindings of success arm pattern>
    ///       return <emit success arm body>;
    ///   }();
    ///   auto [pat...] = _let_match_result;
    ///
    /// The IIFE around the success arm body avoids inner/outer name
    /// shadowing collisions (e.g. when an inner pattern binds `b` and the
    /// outer let-pat also binds `b`) and lets `auto` deduction work on a
    /// single arm's value.
    /// Materialize a deferred `let x;` declaration (whose `emit_local`
    /// suppressed the broken `auto x;` because no type was inferable)
    /// as `auto cpp_name = rhs;` at the first assignment site.
    ///
    /// Handles the direct `x = expr;` shape and the
    /// `unsafe { x = expr; }` / plain-block `{ x = expr; }` wrappers
    /// (Rust idiomatic deferred-init lives inside an unsafe block —
    /// see linked_list_port::split_off_before_node).
    ///
    /// Returns true if it took over emission for this statement; the
    /// caller should not fall through to the regular Expr path.
    pub(super) fn try_emit_pending_uninit_let_assign(&mut self, expr: &syn::Expr) -> bool {
        // Direct `x = rhs;`
        if let syn::Expr::Assign(assign) = expr {
            return self.try_emit_pending_uninit_let_assign_inner(
                &assign.left,
                &assign.right,
            );
        }
        // `unsafe { stmts }` / `{ stmts }` containing a single
        // `Stmt::Expr(Expr::Assign(...), Some(_))` body. Don't open
        // a scope around the assign — the auto declaration must live
        // in the *outer* scope so subsequent uses can see it.
        let block_stmts = match expr {
            syn::Expr::Unsafe(u) => Some(&u.block.stmts),
            syn::Expr::Block(b) => Some(&b.block.stmts),
            _ => None,
        };
        let Some(stmts) = block_stmts else {
            return false;
        };
        // Require exactly one statement so we don't strip wrapping
        // side-effects.
        if stmts.len() != 1 {
            return false;
        }
        let syn::Stmt::Expr(inner_expr, Some(_)) = &stmts[0] else {
            return false;
        };
        let syn::Expr::Assign(assign) = inner_expr else {
            return false;
        };
        // Probe: only consume if the LHS is a pending uninit local.
        // The `unsafe` annotation gets re-emitted as a comment so the
        // // @unsafe marker survives in the output.
        let is_pending = {
            let syn::Expr::Path(path_expr) = assign.left.as_ref() else {
                return false;
            };
            if path_expr.path.segments.len() != 1 {
                return false;
            }
            let name = path_expr.path.segments[0].ident.to_string();
            self.pending_uninit_let_locals
                .iter()
                .rev()
                .any(|scope| scope.contains_key(&name))
        };
        if !is_pending {
            return false;
        }
        if matches!(expr, syn::Expr::Unsafe(_)) {
            self.writeln("// @unsafe");
        }
        self.try_emit_pending_uninit_let_assign_inner(&assign.left, &assign.right)
    }

    pub(super) fn try_emit_pending_uninit_let_assign_inner(
        &mut self,
        left: &syn::Expr,
        right: &syn::Expr,
    ) -> bool {
        let syn::Expr::Path(path_expr) = left else {
            return false;
        };
        if path_expr.path.segments.len() != 1 {
            return false;
        }
        let name = path_expr.path.segments[0].ident.to_string();
        // Find and consume the pending entry in the innermost scope
        // that has it.
        let cpp_name = {
            let mut found_cpp_name: Option<String> = None;
            for scope in self.pending_uninit_let_locals.iter_mut().rev() {
                if let Some(cpp) = scope.remove(&name) {
                    found_cpp_name = Some(cpp);
                    break;
                }
            }
            match found_cpp_name {
                Some(cpp) => cpp,
                None => return false,
            }
        };
        let rhs_text = self.emit_expr_to_string(right);
        self.writeln(&format!("auto {} = {};", cpp_name, rhs_text));
        true
    }

    pub(super) fn try_emit_let_match_return_statement_level(
        &mut self,
        local: &syn::Local,
    ) -> bool {
        // Gate 0: must be `Pat::Tuple` on the let.
        let tuple_pat = match &local.pat {
            syn::Pat::Tuple(t) => t,
            _ => return false,
        };
        // Gate 1: must have an init that's a Match.
        let init = match &local.init {
            Some(i) => i,
            None => return false,
        };
        let match_expr = match self.peel_paren_group_expr(&init.expr) {
            syn::Expr::Match(m) => m,
            _ => return false,
        };
        // Gate 2: identify exactly-one success arm + at least one
        // diverging arm. Reject guard'd arms — we can't easily separate
        // the guard test from the arm condition in this shape.
        let mut diverging_arms: Vec<&syn::Arm> = Vec::new();
        let mut success_arm: Option<&syn::Arm> = None;
        for arm in &match_expr.arms {
            if arm.guard.is_some() {
                return false;
            }
            if self.expr_is_try_style_return_flow(&arm.body) {
                diverging_arms.push(arm);
            } else if success_arm.is_some() {
                return false;
            } else {
                success_arm = Some(arm);
            }
        }
        if diverging_arms.is_empty() {
            return false;
        }
        let success_arm = match success_arm {
            Some(a) => a,
            None => return false,
        };
        // Gate 3: arity & every arm pattern must be Pat::Tuple of the
        // same arity matching the outer let pattern.
        let arity = tuple_pat.elems.len();
        for arm in &match_expr.arms {
            match &arm.pat {
                syn::Pat::Tuple(tp) if tp.elems.len() == arity => {}
                _ => return false,
            }
        }
        if !self.tuple_match_can_lower_as_value_conditions(&match_expr.arms, arity) {
            return false;
        }
        // We've committed to this lowering. Allocate synthetics.
        let tuple_tmp = self.reserve_synthetic_cpp_name("_let_match_tuple");
        let mut slot_names: Vec<String> = Vec::with_capacity(arity);
        for idx in 0..arity {
            slot_names.push(self.reserve_synthetic_cpp_name(&format!("_let_match_m{}", idx)));
        }
        // Register let-pat bindings as local cpp names. Must be done
        // BEFORE emitting the IIFE so subsequent code in the enclosing
        // block can resolve them.
        self.register_local_binding_pattern(&local.pat);
        let mut outer_binding_names: Vec<String> = Vec::with_capacity(arity);
        let mut rust_binding_names: Vec<Option<String>> = Vec::with_capacity(arity);
        for (idx, p) in tuple_pat.elems.iter().enumerate() {
            match p {
                syn::Pat::Ident(pi) if pi.ident != "_" => {
                    let raw = pi.ident.to_string();
                    let cpp_name = self.allocate_local_cpp_name(&raw);
                    outer_binding_names.push(cpp_name);
                    rust_binding_names.push(Some(raw));
                }
                _ => {
                    outer_binding_names.push(
                        self.reserve_synthetic_cpp_name(&format!("_let_match_ignore{}", idx)),
                    );
                    rust_binding_names.push(None);
                }
            }
        }
        // Compute per-arm plans (condition + bindings).
        struct ArmPlan<'a> {
            arm: &'a syn::Arm,
            cond: String,
            bindings: Vec<String>,
            binding_map: std::collections::HashMap<String, String>,
        }
        let mut arm_plans: Vec<ArmPlan> = Vec::with_capacity(match_expr.arms.len());
        for arm in &match_expr.arms {
            let syn::Pat::Tuple(tp) = &arm.pat else {
                return false;
            };
            let mut conditions: Vec<String> = Vec::new();
            let mut bindings: Vec<String> = Vec::new();
            let mut binding_map: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            for (i, elem_pat) in tp.elems.iter().enumerate() {
                let value_name = slot_names[i].clone();
                let mut elem_bindings: Vec<String> = Vec::new();
                let Some(cond) = self
                    .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                        elem_pat,
                        &value_name,
                        &mut elem_bindings,
                        &mut binding_map,
                        None,
                    )
                else {
                    return false;
                };
                bindings.extend(elem_bindings);
                if let Some(c) = cond {
                    conditions.push(c);
                }
            }
            Self::allow_runtime_match_binding_payload_moves(&mut bindings, &mut binding_map);
            let cond = if conditions.is_empty() {
                "true".to_string()
            } else {
                conditions.join(" && ")
            };
            arm_plans.push(ArmPlan { arm, cond, bindings, binding_map });
        }
        // Emit:
        //   auto&& _let_match_tuple = scrutinee;
        //   auto&& _let_match_mN = std::get<N>(_let_match_tuple);
        let scrutinee_str = self.emit_expr_to_string(&match_expr.expr);
        self.writeln(&format!("auto&& {} = {};", tuple_tmp, scrutinee_str));
        for (idx, name) in slot_names.iter().enumerate() {
            self.writeln(&format!(
                "auto&& {} = std::get<{}>(rusty::detail::deref_if_pointer({}));",
                name, idx, tuple_tmp
            ));
        }
        // Diverging arms first.
        for plan in &arm_plans {
            if !std::ptr::eq(plan.arm, success_arm) {
                self.writeln(&format!("if ({}) {{", plan.cond));
                self.indent += 1;
                for binding in &plan.bindings {
                    self.writeln(binding);
                }
                let pushed = self.push_local_cpp_binding_scope_with_types(&plan.binding_map, None);
                let body_str = self.emit_expr_to_string(&plan.arm.body);
                // The arm body is a Rust `return X` expression, which
                // emits as `return X` (no trailing semicolon) at expr
                // level. Terminate as a statement here.
                self.writeln(&format!("{};", body_str));
                self.pop_local_cpp_binding_scope(pushed);
                self.indent -= 1;
                self.writeln("}");
            }
        }
        // Success arm — wrap in IIFE to avoid name-shadow collisions
        // between inner pattern bindings (e.g. `Some(s), b`) and outer
        // let-pattern bindings (e.g. outer `a, b`).
        let result_tmp = self.reserve_synthetic_cpp_name("_let_match_result");
        let success_plan = arm_plans
            .iter()
            .find(|p| std::ptr::eq(p.arm, success_arm))
            .expect("success plan present");
        self.writeln(&format!("auto {} = [&]() {{", result_tmp));
        self.indent += 1;
        for binding in &success_plan.bindings {
            self.writeln(binding);
        }
        let pushed = self.push_local_cpp_binding_scope_with_types(&success_plan.binding_map, None);
        let body_str = self.emit_expr_to_string(&success_arm.body);
        self.writeln(&format!("return {};", body_str));
        self.pop_local_cpp_binding_scope(pushed);
        self.indent -= 1;
        self.writeln(&format!("}}();"));
        // Outer destructuring. Use `auto&& [..]` so each element is
        // bound by forwarding reference. Plain `auto [..]` would
        // value-decay each element into a copy of the tuple member,
        // which breaks move-only Ts (B4 — same root cause as the other
        // structured-binding emit sites; the let-match-result tmp's
        // tuple contains the success arm's value chain and we must
        // forward through it). See
        // tests/btree_port_iter_remove_movonly_test.cpp:btree_internal
        // .cppm:5364 for the original surfacing.
        if outer_binding_names.is_empty() {
            self.writeln(&format!("static_cast<void>({});", result_tmp));
        } else {
            self.writeln(&format!(
                "auto&& [{}] = {};",
                outer_binding_names.join(", "),
                result_tmp
            ));
        }
        true
    }

    /// Statement-level lowering for `lhs = match scrutinee { ... };` where
    /// the match has at least one diverging (early-return) arm. The default
    /// expression-lowering wraps the match in an IIFE; any `return` inside
    /// the arm bodies then escapes only the lambda — not the enclosing
    /// function — and the IIFE's `auto` return type can't reconcile a
    /// diverging arm with a value arm.
    ///
    /// The fix is a pure AST transform: push the assignment down into the
    /// tail position of every non-diverging arm body (recursing through
    /// blocks and nested matches), then emit the rewritten match as a
    /// statement-level runtime match. Diverging arm bodies are left intact
    /// so `return X;` lives at function scope.
    ///
    /// An inner pattern binding can shadow the outer LHS path identifier
    /// (e.g. `outer_split = match { ... Some(split) => split.foo() }` would
    /// emit `split = split.foo()` after inner pattern binding `auto&& split`
    /// shadows the outer). To avoid that, alias the LHS through a synthetic
    /// reference declared *before* the match opens, and use the synthetic
    /// as the push-down target — its name is fresh by construction.
    pub(super) fn try_emit_assign_match_return_statement_level(
        &mut self,
        lhs_expr: &syn::Expr,
        rhs_expr: &syn::Expr,
    ) -> bool {
        let rhs_peeled = self.peel_paren_group_expr(rhs_expr);
        let syn::Expr::Match(match_expr) = rhs_peeled else {
            return false;
        };
        if !self.match_expr_has_explicit_return_arm(match_expr) {
            return false;
        }
        if match_expr.arms.iter().any(|a| a.guard.is_some()) {
            return false;
        }
        // Pre-emit alias `auto& <synth> = <lhs>;` so inner-pattern bindings
        // that happen to share a name with the LHS path don't shadow it.
        let lhs_cpp = self.emit_expr_to_string(lhs_expr);
        let synth_name = self.reserve_synthetic_cpp_name("_assign_match_lhs");
        self.writeln(&format!("auto& {} = {};", synth_name, lhs_cpp));
        let Ok(synth_expr) = syn::parse_str::<syn::Expr>(&synth_name) else {
            return false;
        };
        let new_match = self.push_assign_into_match(&synth_expr, match_expr);
        self.emit_match(&new_match);
        true
    }

    /// Statement-level lowering for `place = if cond { ...?...; val_a } else { val_b };`.
    ///
    /// When a branch of the RHS if-expression contains `?` or an early
    /// `return`, the default expression path can only wrap it in an IIFE — but
    /// that traps the control flow inside the lambda instead of escaping to the
    /// enclosing function, so the emitter degrades to a `/* TODO: if-expression
    /// */` placeholder. Instead, alias the LHS place and push the assignment
    /// into each branch tail (reusing `emit_if_assign_as_statement_block`), so
    /// the if-expression becomes an if-*statement* and the `?`/`return` lower to
    /// ordinary function-level control flow.
    pub(super) fn try_emit_assign_if_return_statement_level(
        &mut self,
        lhs_expr: &syn::Expr,
        rhs_expr: &syn::Expr,
    ) -> bool {
        let rhs_peeled = self.peel_paren_group_expr(rhs_expr);
        let syn::Expr::If(if_expr) = rhs_peeled else {
            return false;
        };
        // Assignment needs a value in every path, so an else branch is required.
        let Some((_, else_branch)) = &if_expr.else_branch else {
            return false;
        };
        // Only intercept when a branch has escaping control flow (`?`/`return`);
        // the plain ternary/IIFE path handles the value-only case correctly.
        let needs_stmt_lowering = self.block_contains_early_return_or_try(&if_expr.then_branch)
            || self.expr_contains_early_return_or_try(else_branch);
        if !needs_stmt_lowering {
            return false;
        }
        // Pre-emit alias `auto& <synth> = <lhs>;` so inner-pattern bindings that
        // happen to share a name with the LHS path don't shadow it (mirrors the
        // match-assign handler).
        let lhs_cpp = self.emit_expr_to_string(lhs_expr);
        let synth_name = self.reserve_synthetic_cpp_name("_assign_if_lhs");
        self.writeln(&format!("auto& {} = {};", synth_name, lhs_cpp));
        let expected_ty = self.infer_simple_expr_type(lhs_expr);
        self.emit_if_assign_as_statement_block(&synth_name, if_expr, expected_ty.as_ref());
        true
    }

    pub(super) fn try_emit_tuple_local_match_initializer(
        &mut self,
        tuple: &syn::PatTuple,
        match_expr: &syn::ExprMatch,
    ) -> bool {
        if !self.expr_contains_early_return_or_try(&syn::Expr::Match(match_expr.clone())) {
            return false;
        }
        let Some(tuple_ty) = self
            .infer_match_arms_common_type_with_scrutinee(match_expr)
            .or_else(|| self.infer_match_arms_common_type(&match_expr.arms))
        else {
            return false;
        };
        let Some(resolved_tuple_ty) = self.resolve_tuple_type_from_type(&tuple_ty) else {
            return false;
        };
        if resolved_tuple_ty.elems.len() != tuple.elems.len() {
            return false;
        }
        let tuple_cpp = self.map_type(&tuple_ty);
        if tuple_cpp == "auto"
            || tuple_cpp.contains("/* TODO")
            || type_string_has_auto_placeholder(&tuple_cpp)
        {
            return false;
        }
        if !self.match_assign_to_optional_result_is_supported(match_expr) {
            return false;
        }

        let result_var = self.reserve_synthetic_cpp_name("_tuple_match_value");
        self.writeln(&format!("std::optional<{}> {};", tuple_cpp, result_var));
        if !self.try_emit_match_assign_to_optional_result(&result_var, match_expr, Some(&tuple_ty))
        {
            return false;
        }

        let rust_binding_names: Vec<Option<String>> = tuple
            .elems
            .iter()
            .map(|p| match p {
                syn::Pat::Ident(pi) if pi.ident != "_" => Some(pi.ident.to_string()),
                _ => None,
            })
            .collect();
        let mut tuple_binding_names = Vec::with_capacity(tuple.elems.len());
        for (idx, p) in tuple.elems.iter().enumerate() {
            let raw = self.emit_pat_to_string(p);
            if raw == "_" {
                tuple_binding_names
                    .push(self.reserve_synthetic_cpp_name(&format!("_tuple_ignore{}", idx)));
            } else {
                tuple_binding_names.push(self.allocate_local_cpp_name(&raw));
            }
        }
        if tuple_binding_names.is_empty() {
            self.writeln(&format!(
                "static_cast<void>(std::move({}).value());",
                result_var
            ));
        } else if resolved_tuple_ty
            .elems
            .iter()
            .any(|elem_ty| matches!(self.peel_paren_group_type(elem_ty), syn::Type::Reference(_)))
        {
            let tuple_tmp = self.reserve_synthetic_cpp_name("_tuple_destructure");
            self.writeln(&format!(
                "auto {} = std::move({}).value();",
                tuple_tmp, result_var
            ));
            for (idx, binding_name) in tuple_binding_names.iter().enumerate() {
                let is_ref_elem = resolved_tuple_ty
                    .elems
                    .iter()
                    .nth(idx)
                    .is_some_and(|elem_ty| {
                        matches!(self.peel_paren_group_type(elem_ty), syn::Type::Reference(_))
                    });
                let binding_auto = if is_ref_elem { "auto&&" } else { "auto" };
                self.writeln(&format!(
                    "{} {} = std::get<{}>(rusty::detail::deref_if_pointer({}));",
                    binding_auto, binding_name, idx, tuple_tmp
                ));
            }
        } else {
            self.writeln(&format!(
                "auto [{}] = std::move({}).value();",
                tuple_binding_names.join(", "),
                result_var
            ));
        }
        for (raw_name, elem_ty) in rust_binding_names
            .iter()
            .zip(resolved_tuple_ty.elems.iter())
        {
            if let Some(name) = raw_name {
                self.update_local_binding_type(name.clone(), elem_ty.clone());
            }
        }
        true
    }

    pub(super) fn emit_expr_with_tuple_elem_expected_types(
        &self,
        expr: &syn::Expr,
        tuple_elem_expected: Option<&[Option<syn::Type>]>,
        fallback_expected: Option<&syn::Type>,
    ) -> String {
        let expr = self.peel_paren_group_expr(expr);
        if let Some(expected) = tuple_elem_expected {
            if let syn::Expr::Tuple(tuple_expr) = expr {
                if tuple_expr.elems.len() == expected.len() {
                    let elems: Vec<String> = tuple_expr
                        .elems
                        .iter()
                        .zip(expected.iter())
                        .map(|(elem, elem_ty)| {
                            self.emit_expr_to_string_with_expected_and_move_if_needed(
                                elem,
                                elem_ty.as_ref(),
                            )
                        })
                        .collect();
                    return format!("std::make_tuple({})", elems.join(", "));
                }
            }
        }
        self.emit_expr_to_string_with_expected(expr, fallback_expected)
    }

    pub(super) fn try_emit_static_typed_self_method_call(
        &self,
        receiver_expr: &syn::Expr,
        method_name: &str,
        method_template_args: Option<&str>,
        args: &[String],
        receiver_expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        if self.expr_is_deref_of_self_path(receiver_expr)
            && matches!(method_name, "force" | "force_mut")
        {
            let owner_cpp = self.receiver_owner_cpp_type_for_method_call(receiver_expr)?;
            let raw_receiver = if let Some(expected) = receiver_expected_ty {
                self.emit_expr_to_string_with_expected(receiver_expr, Some(expected))
            } else {
                self.emit_expr_to_string(receiver_expr)
            };
            let receiver = if self.method_receiver_needs_parentheses(receiver_expr) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let mut static_args = Vec::with_capacity(args.len() + 1);
            static_args.push(receiver);
            static_args.extend(args.iter().cloned());
            let escaped_method_name = escape_cpp_keyword(method_name);
            let method_call = if let Some(template_args) = method_template_args {
                format!("template {}{}", escaped_method_name, template_args)
            } else {
                escaped_method_name
            };
            return Some(format!(
                "{}::{}({})",
                owner_cpp,
                method_call,
                static_args.join(", ")
            ));
        }
        let receiver_is_simple_path = matches!(
            self.peel_paren_group_expr(receiver_expr),
            syn::Expr::Path(_)
        ) || self.expr_is_deref_of_self_path(receiver_expr);
        if !receiver_is_simple_path {
            return None;
        }
        let owner_name = if self.expr_is_deref_of_self_path(receiver_expr) {
            self.current_struct.clone()?
        } else if let Some((owner_name, _)) =
            self.receiver_owner_name_and_type_substitutions(receiver_expr)
        {
            owner_name
        } else {
            return None;
        };
        let has_receiver = self.lookup_owner_method_has_receiver(&owner_name, method_name);
        if matches!(has_receiver, Some(true)) {
            return None;
        }
        if has_receiver.is_none() && !self.expr_is_deref_of_self_path(receiver_expr) {
            return None;
        }
        let first_expected = self
            .lookup_method_arg_expected_type_from_receiver_owner(
                receiver_expr,
                method_name,
                0,
                None,
            )
            .or_else(|| {
                self.lookup_owner_method_arg_expected_type(&owner_name, method_name, 0, None)
            });
        if let Some(first_expected) = first_expected {
            if !self.typed_self_param_matches_receiver(&first_expected, receiver_expr) {
                return None;
            }
        } else if !self.expr_is_deref_of_self_path(receiver_expr) {
            return None;
        }
        let owner_cpp = self.receiver_owner_cpp_type_for_method_call(receiver_expr)?;
        let raw_receiver = if let Some(expected) = receiver_expected_ty {
            self.emit_expr_to_string_with_expected(receiver_expr, Some(expected))
        } else {
            self.emit_expr_to_string(receiver_expr)
        };
        let receiver = if self.method_receiver_needs_parentheses(receiver_expr) {
            format!("({})", raw_receiver)
        } else {
            raw_receiver
        };
        let mut static_args = Vec::with_capacity(args.len() + 1);
        static_args.push(receiver);
        static_args.extend(args.iter().cloned());
        let escaped_method_name = escape_cpp_keyword(method_name);
        let method_call = if let Some(template_args) = method_template_args {
            format!("template {}{}", escaped_method_name, template_args)
        } else {
            escaped_method_name
        };
        Some(format!(
            "{}::{}({})",
            owner_cpp,
            method_call,
            static_args.join(", ")
        ))
    }

    pub(super) fn try_emit_deserialize_map_seed_rewrite(
        &self,
        mc: &syn::ExprMethodCall,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        if mc.method != "map" || mc.args.len() != 1 {
            return None;
        }
        let syn::Expr::Call(receiver_call) = self.peel_paren_group_expr(&mc.receiver) else {
            return None;
        };
        if receiver_call.args.len() != 1 {
            return None;
        }
        let syn::Expr::Path(func_path_expr) =
            self.peel_paren_group_expr(receiver_call.func.as_ref())
        else {
            return None;
        };
        if func_path_expr.qself.is_some()
            || !Self::path_is_deserialize_trait_call(&func_path_expr.path)
        {
            return None;
        }
        let seed_ty = self.lookup_function_arg_expected_type(&mc.args[0], 0)?;
        let seed_cpp = self.map_type(seed_ty);
        if seed_cpp == "auto"
            || seed_cpp.contains("/* TODO")
            || type_string_has_auto_placeholder(&seed_cpp)
        {
            return None;
        }
        let deserializer = self.emit_deserializer_call_arg(&receiver_call.args[0]);
        let mapper = self
            .try_emit_map_callable_arg_with_expected(&mc.args[0], expected_ty)
            .unwrap_or_else(|| self.emit_expr_maybe_move(&mc.args[0]));
        Some(format!(
            "(::de::rusty_ext::deserialize(rusty::PhantomData<{}>{{}}, {})).map({})",
            seed_cpp, deserializer, mapper
        ))
    }

    /// `LocalError::invalid_value(unexp, exp)` where the local impl doesn't
    /// override the DEP trait's PROVIDED STATIC: dispatch to the member when
    /// it exists, else to the declaring crate's RuntimeHelper templated
    /// static (`::serde_core::de::ErrorRuntimeHelper::invalid_value<Owner>`).
    /// The if-constexpr keeps overridden statics (invalid_type) on the
    /// member without needing local-override bookkeeping.
    fn try_emit_dep_trait_static_default_call(&self, call: &syn::ExprCall) -> Option<String> {
        let syn::Expr::Path(func_path) = call.func.as_ref() else {
            return None;
        };
        if func_path.qself.is_some() || func_path.path.segments.len() < 2 {
            return None;
        }
        let method = func_path.path.segments.last()?.ident.to_string();
        let owner_seg = func_path.path.segments.iter().nth_back(1)?;
        if !matches!(owner_seg.arguments, syn::PathArguments::None) {
            return None;
        }
        let owner_name = owner_seg.ident.to_string();
        // Owner must be a LOCALLY-declared type (a dep's own types keep the
        // plain spelling — their members exist in the dep).
        if !self.local_declared_types.contains(&owner_name) {
            return None;
        }
        // Some dependency trait must declare the method AND carry its module
        // path (manifest declared_trait_modules).
        let mut helper: Option<(String, String, String)> = None;
        for m in &self.dependency_ufcs_trait_manifests {
            for (trait_name, methods) in &m.declared_trait_methods {
                if !methods.iter().any(|mm| mm == &method) {
                    continue;
                }
                let Some(module_path) = m.declared_trait_modules.get(trait_name) else {
                    continue;
                };
                if helper.is_some() {
                    return None; // ambiguous across deps/traits — stay safe
                }
                helper = Some((m.module.clone(), module_path.clone(), trait_name.clone()));
            }
        }
        helper.as_ref()?;
        let owner_cpp = {
            let owner_path: Vec<String> = func_path
                .path
                .segments
                .iter()
                .take(func_path.path.segments.len() - 1)
                .map(|seg| seg.ident.to_string())
                .collect();
            let joined = owner_path.join("::");
            let ty: syn::Type = syn::parse_str(&joined).ok()?;
            self.map_type(&ty)
        };
        let args: Vec<String> = call
            .args
            .iter()
            .map(|arg| self.emit_expr_maybe_move(arg))
            .collect();
        let args = args.join(", ");
        self.dep_trait_static_dispatch_call(&owner_cpp, &method, &args)
    }

    /// Wrap a static trait-method call on `owner_cpp` in the
    /// member-vs-RuntimeHelper SFINAE dispatch, when exactly one
    /// dependency trait manifest declares `method` (with a module path
    /// for its exported RuntimeHelper). The concrete owner may
    /// implement/override the method (the requires-branch picks the
    /// member) or rely on the trait's provided default (routed to the
    /// dep's helper). Returns None when no dep declares the method, on
    /// cross-dep ambiguity, or when the owner spelling still contains
    /// an unresolved `auto` placeholder.
    pub(super) fn dep_trait_static_dispatch_call(
        &self,
        owner_cpp: &str,
        method: &str,
        args: &str,
    ) -> Option<String> {
        if owner_cpp.contains("auto") || type_string_has_auto_placeholder(owner_cpp) {
            return None;
        }
        let mut helper: Option<(String, String, String)> = None;
        for m in &self.dependency_ufcs_trait_manifests {
            for (trait_name, methods) in &m.declared_trait_methods {
                if !methods.iter().any(|mm| mm == method) {
                    continue;
                }
                let Some(module_path) = m.declared_trait_modules.get(trait_name) else {
                    continue;
                };
                if helper.is_some() {
                    return None; // ambiguous across deps/traits — stay safe
                }
                helper = Some((m.module.clone(), module_path.clone(), trait_name.clone()));
            }
        }
        let (dep, module_path, trait_name) = helper?;
        let helper_path = if module_path.is_empty() {
            format!("::{}::{}RuntimeHelper", dep, escape_cpp_keyword(&trait_name))
        } else {
            format!(
                "::{}::{}::{}RuntimeHelper",
                dep,
                module_path,
                escape_cpp_keyword(&trait_name)
            )
        };
        let escaped_method = escape_cpp_keyword(method);
        Some(format!(
            "[&]<typename __Owner = {owner}>() -> decltype(auto) {{ if constexpr (requires {{ __Owner::{m}({args}); }}) {{ return __Owner::{m}({args}); }} else {{ return {helper}::template {m}<__Owner>({args}); }} }}()",
            owner = owner_cpp,
            m = escaped_method,
            args = args,
            helper = helper_path
        ))
    }

    pub(super) fn try_emit_visit_method_fallback_call(
        &self,
        mc: &syn::ExprMethodCall,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let method_name = mc.method.to_string();
        if !(self.module_name.is_some() || self.expanded_libtest_mode)
            || !method_name.starts_with("visit_")
        {
            return None;
        }
        // The serde-visitor fallback emits a `if constexpr (requires { call;
        // })` SFINAE dispatch that re-uses the full call expression — including
        // any argument bodies — inside the requires-clause. Serde's actual
        // `visit_*` methods take primitive values (`bool`, `i64`, `&str`, an
        // `A: SeqAccess`/`MapAccess`, …) and never take closures, so this is
        // fine for them.
        //
        // Other crates can have methods that *happen* to start with `visit_`
        // and take a closure (e.g. `btree_port::btree::btree_internal::
        // NodeRef::visit_nodes_in_order(|pos| { ... })`). Lowering those
        // through this fallback produces a requires-clause that contains the
        // closure body verbatim, and Clang then chokes on the closure's `[&]`
        // capture of locals from the enclosing function:
        //   "reference to local variable 'result' declared in enclosing function"
        //
        // Skip the fallback whenever any argument is a closure — those calls
        // are not serde visitor dispatches and should use the plain
        // method-emit path. Detecting on the syntactic form (`syn::Expr::
        // Closure`) keeps the test purely structural, no type info needed.
        if mc
            .args
            .iter()
            .any(|arg| matches!(arg, syn::Expr::Closure(_)))
        {
            return None;
        }
        let receiver = self.emit_expr_to_string_with_expected(&mc.receiver, expected_ty);
        let args: Vec<String> = mc
            .args
            .iter()
            .map(|arg| {
                let emitted = self.emit_expr_maybe_move(arg);
                self.rewrite_current_deserializer_access_ctor_arg(emitted)
            })
            .collect();
        let template_args = self
            .emit_method_call_template_args(mc, &args)
            .unwrap_or_default();
        let escaped_method_name = escape_cpp_keyword(&method_name);
        let method_call = if template_args.is_empty() {
            escaped_method_name.clone()
        } else {
            format!("template {}{}", escaped_method_name, template_args)
        };
        let target_expr =
            "rusty::detail::deref_if_pointer_like(std::forward<decltype(__visitor)>(__visitor))";
        let direct_call = format!("{}.{}({})", target_expr, method_call, args.join(", "));
        let inferred_err_cpp = self
            .expected_result_type_arg_owned(expected_ty, 1)
            .or_else(|| self.expected_result_type_arg_owned(self.current_return_type_hint(), 1))
            .map(|ty| self.map_type(&ty))
            .filter(|mapped| {
                mapped != "auto"
                    && !mapped.contains("/* TODO")
                    && !type_string_has_auto_placeholder(mapped)
                    // A bare short-uppercase ident is an UNBOUND generic param
                    // (`Result<T, E = Error>` recovery yielding the alias's own
                    // `E`) — emitting `.template visit_bool<E>(v)` leaves E
                    // undeclared at the call site. Unmapped RUST primitive
                    // spellings (i64/u64/…) are equally unusable in C++.
                    && !(mapped.len() <= 2
                        && mapped
                            .chars()
                            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()))
                    && !matches!(
                        mapped.as_str(),
                        "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32"
                            | "u64" | "u128" | "f32" | "f64" | "usize" | "isize"
                    )
            });
        let mut candidate_calls = vec![direct_call.clone()];
        if template_args.is_empty()
            && let Some(err_cpp) = inferred_err_cpp.as_ref()
        {
            candidate_calls.push(format!(
                "{}.template {}<{}>({})",
                target_expr,
                escaped_method_name,
                err_cpp,
                args.join(", ")
            ));
        }
        match method_name.as_str() {
            "visit_borrowed_str" => {
                let alias_call = format!("{}.visit_str({})", target_expr, args.join(", "));
                candidate_calls.push(alias_call);
                if template_args.is_empty()
                    && let Some(err_cpp) = inferred_err_cpp.as_ref()
                {
                    candidate_calls.push(format!(
                        "{}.template visit_str<{}>({})",
                        target_expr,
                        err_cpp,
                        args.join(", ")
                    ));
                }
            }
            "visit_borrowed_bytes" => {
                let alias_call = format!("{}.visit_bytes({})", target_expr, args.join(", "));
                candidate_calls.push(alias_call);
                if template_args.is_empty()
                    && let Some(err_cpp) = inferred_err_cpp.as_ref()
                {
                    candidate_calls.push(format!(
                        "{}.template visit_bytes<{}>({})",
                        target_expr,
                        err_cpp,
                        args.join(", ")
                    ));
                }
            }
            _ => {}
        }
        let missing_visit_return = inferred_err_cpp.as_ref().map(|err_cpp| {
            format!(
                "return [&]() -> rusty::Result<typename __TargetVisitorT::Value, {}> {{ rusty::intrinsics::unreachable(); }}();",
                err_cpp
            )
        });

        let mut value_dispatch = String::new();
        for (idx, call) in candidate_calls.iter().enumerate() {
            if idx == 0 {
                value_dispatch.push_str(&format!(
                    "if constexpr (requires {{ {}; }}) {{ return {}; }}",
                    call, call
                ));
            } else {
                value_dispatch.push_str(&format!(
                    " else if constexpr (requires {{ {}; }}) {{ return {}; }}",
                    call, call
                ));
            }
        }
        if let Some(missing_visit_return) = missing_visit_return {
            value_dispatch.push_str(&format!(" else {{ {} }}", missing_visit_return));
        } else {
            value_dispatch.push_str(" else { rusty::intrinsics::unreachable(); }");
        }
        Some(format!(
            "([&](auto&& __visitor) -> decltype(auto) {{ using __TargetVisitorT = std::remove_cv_t<std::remove_reference_t<decltype({})>>; if constexpr (requires {{ typename __TargetVisitorT::Value; }}) {{ {} }} else {{ return {}; }} }}({}))",
            target_expr, value_dispatch, direct_call, receiver
        ))
    }

    /// A raw-pointer intrinsic (`p.write(v)`, `p.read()`, …) on a raw-pointer
    /// receiver has a dedicated `rusty::ptr::*` lowering later in
    /// `emit_method_call_expr_to_string`. It must NOT be intercepted by the UFCS
    /// trait dispatch when a DEPENDENCY happens to declare a same-named trait
    /// method (e.g. itoa's `Sealed::write`), which classifies the name `TraitOnly`
    /// and would emit a `Sealed_::write_(p, v)` dispatch — a hard error on a
    /// pointer (no member `write_`; the `Sealed_` namespace isn't visible
    /// cross-module). Mirrors the receiver test used by those lowerings.
    fn method_call_is_raw_pointer_intrinsic(
        &self,
        mc: &syn::ExprMethodCall,
        method_name: &str,
    ) -> bool {
        if !matches!(
            method_name,
            "write"
                | "read"
                | "write_unaligned"
                | "read_unaligned"
                | "write_volatile"
                | "read_volatile"
                | "write_bytes"
        ) {
            return false;
        }
        if self.is_expr_raw_pointer_like(&mc.receiver) {
            return true;
        }
        let raw_receiver = self.emit_expr_to_string(&mc.receiver);
        Self::emitted_pointer_add_or_offset_call(&raw_receiver)
    }

    pub(super) fn emit_method_call_expr_to_string(
        &self,
        mc: &syn::ExprMethodCall,
        expected_ty: Option<&syn::Type>,
    ) -> String {
        if let Some(try_into_call) = self.try_emit_try_into_method_call(mc, expected_ty) {
            return try_into_call;
        }
        if let Some(into_call) = self.try_emit_into_method_call(mc, expected_ty) {
            return into_call;
        }
        if let Some(to_owned_call) = self.try_emit_to_owned_method_call(mc) {
            return to_owned_call;
        }
        // Rust auto-derefs Box/Rc/Arc for method calls: `self.0.location()`
        // on `Error(Box<ErrorImpl>)` calls ErrorImpl::location. When the
        // receiver is a SELF field whose declared type is a pointer wrapper
        // and the POINTEE declares the method, route through the deref
        // (member syntax on the wrapper only sees Box's own methods).
        if let syn::Expr::Field(field) = self.peel_paren_group_expr(&mc.receiver)
            && matches!(
                field.base.as_ref(),
                syn::Expr::Path(p) if p.path.is_ident("self")
            )
            && let Some(current) = self.current_struct.as_deref()
        {
            let field_name = match &field.member {
                syn::Member::Named(ident) => ident.to_string(),
                syn::Member::Unnamed(idx) => format!("_{}", idx.index),
            };
            let owner_tail = current.rsplit("::").next().unwrap_or(current);
            let pointee_declares_method = self
                .lookup_struct_field_type(owner_tail, &field_name)
                .map(|ty| self.peel_reference_paren_group_type(&ty).clone())
                .and_then(|ty| match ty {
                    syn::Type::Path(tp) => {
                        let seg = tp.path.segments.last()?;
                        if !matches!(seg.ident.to_string().as_str(), "Box" | "Rc" | "Arc") {
                            return None;
                        }
                        let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
                            return None;
                        };
                        let pointee = args.args.iter().find_map(|arg| match arg {
                            syn::GenericArgument::Type(t) => Some(t),
                            _ => None,
                        })?;
                        let pointee_tail = match self.peel_reference_paren_group_type(pointee) {
                            syn::Type::Path(ptp) => {
                                ptp.path.segments.last().map(|s| s.ident.to_string())
                            }
                            _ => None,
                        }?;
                        self.lookup_owner_method_has_receiver(
                            &pointee_tail,
                            &mc.method.to_string(),
                        )
                    }
                    _ => None,
                })
                .is_some();
            if pointee_declares_method {
                let receiver = self.emit_expr_to_string(&mc.receiver);
                let args: Vec<String> = mc
                    .args
                    .iter()
                    .map(|arg| self.emit_expr_maybe_move(arg))
                    .collect();
                return format!(
                    "rusty::detail::deref_if_pointer_like({}).{}({})",
                    receiver,
                    Self::escape_cpp_method_name(&mc.method.to_string()),
                    args.join(", ")
                );
            }
        }
        if let Some(into_owned_call) = self.try_emit_into_owned_method_call(mc) {
            return into_owned_call;
        }
        if let Some(visit_call) = self.try_emit_visit_method_fallback_call(mc, expected_ty) {
            return visit_call;
        }
        if let Some(seed_rewrite) = self.try_emit_deserialize_map_seed_rewrite(mc, expected_ty) {
            return seed_rewrite;
        }
        // c2rust's `ops::ForceAdd::force_add` / `ForceMul::force_mul` — checked
        // integer arithmetic helpers (`self OP rhs`). They are crate-declared
        // trait methods, so they MUST be intercepted before the UFCS trait
        // dispatch below (which otherwise routes them through a member-call
        // fallback `x.force_add(y)` on a scalar — not a struct). Lower to
        // parenthesized arithmetic (operands parenthesized to keep precedence).
        if matches!(mc.method.to_string().as_str(), "force_add" | "force_mul")
            && mc.args.len() == 1
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let rhs = self.emit_expr_to_string(&mc.args[0]);
            let op = if mc.method == "force_add" { "+" } else { "*" };
            return format!("(({}) {} ({}))", receiver, op, rhs);
        }
        // c2rust's `ops::ForceInto::force_into<U>(self) -> U` — a checked
        // `TryInto` conversion. Like force_add it must be intercepted before the
        // UFCS dispatch (the member fallback `x.force_into()` hard-errors on a
        // scalar — "member reference base type 'unsigned long' is not a structure").
        // Lower to the value, cast to the expected target type when known; C++
        // integer conversions otherwise apply implicitly at the use site.
        if mc.method == "force_into" && mc.args.is_empty() {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            if let Some(ty) = expected_ty {
                let cpp_ty = self.map_type(ty);
                if cpp_ty != "auto" && !type_string_has_auto_placeholder(&cpp_ty) {
                    return format!("static_cast<{}>({})", cpp_ty, receiver);
                }
            }
            return format!("({})", receiver);
        }
        // UFCS Phase 3 (book § 3.2.3): a method whose name is a *trait-only*
        // method of one of THIS crate's traits lowers to a free call
        // `m(recv, args)` (resolved via the `<Tr>_` namespace + `using`s in
        // phase 4). The classifier never sees std/inherent methods, so those
        // fall through to the existing member-call lowering unchanged. Guarded
        // by `ufcs_traits` (default off).
        {
            let method_name = mc.method.to_string();
            if std::env::var_os("RUSTY_CPP_DBG_UFCS_GATE").is_some()
                && matches!(method_name.as_str(), "exactly_one" | "sum1" | "next_array")
            {
                eprintln!(
                    "[ufcs-gate] {} class={:?} owners={:?}",
                    method_name,
                    self.ufcs_method_classes.get(&method_name),
                    self.ufcs_method_trait_owners.get(&method_name)
                );
            }
            if matches!(
                self.ufcs_method_classes.get(&method_name),
                Some(crate::transpile::MethodNameClass::TraitOnly)
            ) && !Self::method_prefers_runtime_helper_namespace(&method_name)
                && !self.method_call_is_raw_pointer_intrinsic(mc, &method_name)
            {
                // Only intercept when a CONCRETE impl actually emits a
                // `<Tr>_::m` free function (the owner map is built from
                // concrete impls only). A TraitOnly name with no concrete owner
                // is a default trait method or a generic/blanket-impl method —
                // there is no free function to call, and the method is
                // materialized as a struct member, so the unqualified shim would
                // be useless and, worse, can HARD-error if the bare name
                // collides with a namespace (e.g. `iter`: `requires { iter(x) }`
                // is a parse error, not SFINAE, when `iter` names a namespace).
                // For those, fall through to the normal member-call lowering.
                if let Some(traits) = self.ufcs_method_trait_owners.get(&method_name) {
                    let receiver = self.emit_expr_to_string(&mc.receiver);
                    let args: Vec<String> = mc
                        .args
                        .iter()
                        .map(|a| self.emit_expr_to_string(a))
                        .collect();
                    // Single owner → qualify `<Tr>_::m` (or `<module>::<Tr>_::m`
                    // for a dependency trait, § 3.2.7), so the unqualified
                    // `m(__self)` can't be shadowed by a local of the same name.
                    let escaped = escape_cpp_keyword(&method_name);
                    if traits.len() == 1 {
                        let mut callee = format!(
                            "{}::{}",
                            self.ufcs_trait_namespace(traits.iter().next().unwrap()),
                            escaped
                        );
                        // #36: serde's SeqAccess/MapAccess accessors
                        // (next_element/next_key/next_value) are generic over the
                        // element/key/value type, which appears in no argument, so
                        // C++ cannot deduce it. The member-call path infers it from
                        // the expected `Result<Option<T>, E>` type and emits a
                        // `<T>` turbofish (see the block near
                        // `infer_serde_access_method_template_type_from_expected`),
                        // but this UFCS TraitOnly path short-circuits before that.
                        // Mirror the inference here so the turbofish threads into
                        // the `<Tr>_::next_element<T>(...)` callee (the autoderef
                        // fallback emitter splits/reattaches the `<...>` suffix
                        // across its direct/deref/member branches). When inference
                        // yields nothing — the normal seq/map case where the
                        // expected type doesn't encode the element — the callee is
                        // left bare, unchanged.
                        if mc.args.is_empty()
                            && mc.turbofish.is_none()
                            && matches!(
                                method_name.as_str(),
                                "next_element" | "next_key" | "next_value"
                            )
                            && self.method_call_may_need_serde_access_template_arg(
                                &mc.receiver,
                                &method_name,
                            )
                            && let Some(access_ty) = self
                                .infer_serde_access_method_template_type_from_expected(
                                    &method_name,
                                    expected_ty,
                                )
                        {
                            let access_cpp = self.map_type(&access_ty);
                            if access_cpp != "auto"
                                && !access_cpp.contains("/* TODO")
                                && !type_string_has_auto_placeholder(&access_cpp)
                            {
                                callee = format!("{}<{}>", callee, access_cpp);
                            }
                        }
                        // A method TURBOFISH supplies generics C++ can't deduce
                        // (`.sum1::<i32>()` — S only in the return type). The
                        // free fn declares Self_ FIRST, and explicit template
                        // args fill left-to-right, so Self_ must be spelled
                        // too — from the receiver's own type. Dropping the
                        // turbofish left the requires-probes SFINAE-false and
                        // the dispatch fell through to the (hard-error) member
                        // branch.
                        if let Some(turbofish) = &mc.turbofish
                            && !callee.contains('<')
                        {
                            let mapped: Vec<String> = turbofish
                                .args
                                .iter()
                                .filter_map(|arg| match arg {
                                    syn::GenericArgument::Type(ty) => {
                                        Some(self.map_type(ty))
                                    }
                                    _ => None,
                                })
                                .collect();
                            let all_viable = !mapped.is_empty()
                                && mapped.iter().all(|t| {
                                    t != "auto"
                                        && !t.contains("/* TODO")
                                        && !type_string_has_auto_placeholder(t)
                                        // Crate-path-qualified args can be
                                        // mis-qualified at this position
                                        // (serde's `::<IgnoredAny>` mapped to
                                        // `serde::de::IgnoredAny`, unnameable
                                        // inside the crate) — thread only
                                        // primitives/std/rusty spellings and
                                        // leave the rest to the member path
                                        // as before.
                                        && t.split('<')
                                            .flat_map(|part| part.split(", "))
                                            .all(|part| {
                                                !part.contains("::")
                                                    || part.trim_start_matches("typename ")
                                                        .starts_with("std::")
                                                    || part.trim_start_matches("typename ")
                                                        .starts_with("rusty::")
                                            })
                                });
                            if all_viable {
                                callee = format!(
                                    "{}<std::remove_cvref_t<decltype({})>, {}>",
                                    callee,
                                    receiver,
                                    mapped.join(", ")
                                );
                            }
                        }
                        return self.emit_extension_call_with_receiver_autoderef_fallback(
                            &callee, &receiver, &args,
                        );
                    }
                    // Multi-owner (Fix A): try each owner's qualified `<Tr>_::m`
                    // rather than the unqualified `m`, which would clash with a
                    // same-named module/namespace (serde's `de::size_hint`).
                    let callees: Vec<String> = traits
                        .iter()
                        .map(|t| format!("{}::{}", self.ufcs_trait_namespace(t), escaped))
                        .collect();
                    return self.emit_multi_owner_ufcs_call(
                        &callees, &receiver, &args, &escaped,
                    );
                }
            }
        }
        if matches!(mc.method.to_string().as_str(), "compact" | "readable") && mc.args.is_empty() {
            // serde_test::Configure extension helpers are test-only adapters.
            // Keep parity execution moving by treating them as identity.
            return self.emit_expr_to_string_with_expected(&mc.receiver, expected_ty);
        }
        if mc.method == "next"
            && mc.args.is_empty()
            && self.expr_is_named_field(&mc.receiver, "iter")
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let member_op = if self.method_receiver_uses_pointer_member_access(&mc.receiver) {
                "->"
            } else {
                "."
            };
            return format!("{}{}next()", receiver, member_op);
        }
        if matches!(
            mc.method.to_string().as_str(),
            "is_nonfinite" | "format_nonfinite"
        ) && mc.args.is_empty()
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return self.emit_extension_call_with_receiver_autoderef_fallback(
                &format!("::private_::rusty_ext::{}", mc.method),
                &receiver,
                &[],
            );
        }
        if mc.method == "expect"
            && mc.args.len() == 1
            && let syn::Expr::MethodCall(inner_try_into) = self.peel_paren_group_expr(&mc.receiver)
            && inner_try_into.method == "try_into"
            && inner_try_into.args.is_empty()
        {
            if let Some(expected) = expected_ty
                && let Some(target_ty) =
                    self.resolve_try_into_target_type(expected, &inner_try_into.receiver)
            {
                let target_cpp = self.map_type(&target_ty);
                if target_cpp != "auto"
                    && !target_cpp.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&target_cpp)
                {
                    let receiver = self.emit_try_into_receiver_arg(&inner_try_into.receiver);
                    let expect_arg = self.emit_expr_to_string(&mc.args[0]);
                    let target_is_array_like = matches!(
                        self.peel_reference_paren_group_type(&target_ty),
                        syn::Type::Array(_)
                    );
                    let target_is_scalar_like =
                        target_cpp == "Self" || is_numeric_cpp_scalar_type(target_cpp.trim());
                    if target_is_array_like || target_is_scalar_like {
                        return format!(
                            "rusty::try_from<{}>({}).expect({})",
                            target_cpp, receiver, expect_arg
                        );
                    }
                    return format!(
                        "{}::try_from({}).expect({})",
                        target_cpp, receiver, expect_arg
                    );
                }
            }
            // Fallback for unconstrained generic branches where the expected
            // try-into target is unavailable at transpile time.
            return self.emit_try_into_receiver_arg(&inner_try_into.receiver);
        }
        if mc.method == "serialize_bytes" && mc.args.len() == 1 {
            let receiver = self.emit_expr_to_string_with_expected(&mc.receiver, expected_ty);
            let bytes = self.emit_expr_maybe_move(&mc.args[0]);
            return self.emit_extension_call_with_receiver_autoderef_fallback(
                "::ser::rusty_ext::serialize_bytes",
                &receiver,
                &[bytes],
            );
        }
        if mc.method == "serialize_entry" && mc.args.len() == 2 {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let key = self.emit_expr_maybe_move(&mc.args[0]);
            let value = self.emit_expr_maybe_move(&mc.args[1]);
            return format!(
                "([&]() {{ auto&& __serialize_entry_target = {}; auto __key_res = __serialize_entry_target.serialize_key({}); if (__key_res.is_err()) {{ return __key_res; }} return __serialize_entry_target.serialize_value({}); }}())",
                receiver, key, value
            );
        }
        if mc.method == "deserialize_any" && mc.args.len() == 1 {
            let receiver = self.emit_expr_to_string_with_expected(&mc.receiver, expected_ty);
            let visitor = self.emit_expr_maybe_move(&mc.args[0]);
            return self.emit_extension_call_with_receiver_autoderef_fallback(
                "::de::rusty_ext::deserialize_any",
                &receiver,
                &[visitor],
            );
        }
        if matches!(
            mc.method.to_string().as_str(),
            "deserialize_bytes" | "deserialize_byte_buf"
        ) && mc.args.len() == 1
        {
            let receiver = self.emit_expr_to_string_with_expected(&mc.receiver, expected_ty);
            let visitor = self.emit_expr_maybe_move(&mc.args[0]);
            return self.emit_extension_call_with_receiver_autoderef_fallback(
                "::de::rusty_ext::deserialize_any",
                &receiver,
                &[visitor],
            );
        }
        if mc.method == "deserialize_in_place" && mc.args.len() == 1 {
            let receiver = self.emit_expr_to_string_with_expected(&mc.receiver, expected_ty);
            let place = self.emit_expr_maybe_move(&mc.args[0]);
            return self.emit_extension_call_with_receiver_autoderef_fallback(
                "::de::rusty_ext::deserialize_in_place",
                &receiver,
                &[place],
            );
        }
        if mc.method == "invalid_type" && mc.args.len() == 1 {
            let receiver_tail = self.infer_simple_expr_type(&mc.receiver).and_then(|ty| {
                self.expected_type_path(self.peel_reference_paren_group_type(&ty))
                    .and_then(|path| path.segments.last())
                    .map(|seg| seg.ident.to_string())
            });
            let receiver_is_self_value = matches!(
                self.peel_paren_group_expr(&mc.receiver),
                syn::Expr::Path(path)
                    if path.path.segments.len() == 1
                        && path.path.segments[0].ident == "self"
                        && self.current_struct.as_deref().is_some_and(|name| name.ends_with("Value"))
            );
            if receiver_tail.as_deref() != Some("Value") && !receiver_is_self_value {
                let exp = self.emit_expr_maybe_move(&mc.args[0]);
                return self.emit_receiver_member_call(
                    &mc.receiver,
                    "invalid_type",
                    None,
                    &[exp],
                    None,
                );
            }
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let member_op = if self.method_receiver_uses_pointer_member_access(&mc.receiver) {
                "->"
            } else {
                "."
            };
            let err_cpp = expected_ty
                .map(|ty| self.map_type(ty))
                .filter(|mapped| {
                    mapped != "auto"
                        && !mapped.contains("/* TODO")
                        && !type_string_has_auto_placeholder(mapped)
                })
                .unwrap_or_else(|| "error::Error".to_string());
            let exp = self.emit_expr_maybe_move(&mc.args[0]);
            return format!(
                "{}{}template invalid_type<{}>({})",
                receiver, member_op, err_cpp, exp
            );
        }
        if mc.method == "deserialize" && mc.args.is_empty() {
            let receiver = self.emit_expr_to_string_with_expected(&mc.receiver, expected_ty);
            let seed_cpp = self
                .method_call_single_turbofish_type(mc)
                .map(|ty| self.map_type(ty))
                .or_else(|| {
                    self.expected_result_type_arg(expected_ty, 0)
                        .map(|ty| self.map_type(ty))
                })
                .or_else(|| {
                    self.ordered_type_params_in_scope()
                        .into_iter()
                        .next()
                        .map(|name| escape_cpp_keyword(&name))
                });
            if let Some(seed_cpp) = seed_cpp
                && seed_cpp != "auto"
                && !seed_cpp.contains("/* TODO")
                && !type_string_has_auto_placeholder(&seed_cpp)
            {
                return format!(
                    "::de::rusty_ext::deserialize(rusty::PhantomData<{}>{{}}, {})",
                    seed_cpp, receiver
                );
            }
        }
        if matches!(mc.method.to_string().as_str(), "Bytes" | "BorrowedBytes")
            && mc.args.is_empty()
            && let syn::Expr::Call(call) = self.peel_paren_group_expr(&mc.receiver)
            && let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(call.func.as_ref())
        {
            let joined = path_expr
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            if matches!(joined.as_str(), "rusty::array_repeat" | "array_repeat") {
                let receiver = self.emit_expr_to_string(&mc.receiver);
                if mc.method == "Bytes" {
                    return format!("Token::Bytes({})", receiver);
                }
                return format!("Token::BorrowedBytes({})", receiver);
            }
        }
        if mc.method == "ok" && mc.args.is_empty() {
            if let syn::Expr::MethodCall(parse_mc) = self.peel_paren_group_expr(&mc.receiver)
                && parse_mc.method == "parse"
                && parse_mc.args.is_empty()
            {
                let parsed_ty = self
                    .method_call_single_turbofish_type(parse_mc)
                    .map(|ty| self.peel_reference_paren_group_type(ty).clone())
                    .or_else(|| self.expected_option_type_arg(expected_ty).cloned())
                    .or_else(|| {
                        expected_ty.map(|ty| self.peel_reference_paren_group_type(ty).clone())
                    });
                if let Some(parsed_ty) = parsed_ty {
                    let parsed_cpp = self.map_type(&parsed_ty);
                    if parsed_cpp != "auto"
                        && !parsed_cpp.contains("/* TODO")
                        && !type_string_has_auto_placeholder(&parsed_cpp)
                    {
                        let receiver = self.emit_expr_to_string(&parse_mc.receiver);
                        return format!(
                            "rusty::str_runtime::parse<{}>({}).ok()",
                            parsed_cpp, receiver
                        );
                    }
                }
            }
        }
        if let Some(then_with_call) = self.try_emit_ordering_then_with_call(mc) {
            return then_with_call;
        }
        if mc.method == "or_insert_with" && mc.args.len() == 1 {
            if let syn::Expr::MethodCall(entry_mc) = self.peel_paren_group_expr(&mc.receiver) {
                if entry_mc.method == "entry"
                    && entry_mc.args.len() == 1
                    && self.expr_is_default_constructor_callable(&mc.args[0])
                {
                    // `map.entry(k).or_insert_with(Vec::new_)` is equivalent
                    // to `map.entry(k)` on rusty::HashMap, which inserts a
                    // default-constructed value for missing keys.
                    return self.emit_method_call_expr_to_string(entry_mc, None);
                }
            }
        }
        if mc.method == "assume_init" && mc.args.is_empty() {
            if let Some(expected) = expected_ty {
                let maybe_uninit_expected: syn::Type = parse_quote!(MaybeUninit<#expected>);
                let raw_receiver = self
                    .emit_expr_to_string_with_expected(&mc.receiver, Some(&maybe_uninit_expected));
                let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                    format!("({})", raw_receiver)
                } else {
                    raw_receiver
                };
                let member_op = if self.method_receiver_uses_pointer_member_access(&mc.receiver) {
                    "->"
                } else {
                    "."
                };
                return format!("{}{}assume_init()", receiver, member_op);
            }
        }
        if mc.method == "len"
            && mc.args.is_empty()
            && !self.receiver_declares_inherent_method(&mc.receiver, "len")
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.receiver_is_lazy_wrapper_type(&mc.receiver) {
                if self.method_receiver_needs_parentheses(&mc.receiver) {
                    format!("(*({}))", raw_receiver)
                } else {
                    format!("(*{})", raw_receiver)
                }
            } else {
                raw_receiver
            };
            return format!("rusty::len({})", receiver);
        }
        if mc.method == "contains"
            && mc.args.len() == 1
            && self.expr_lowers_to_slice_or_span_view(&mc.receiver)
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let needle = self.emit_expr_maybe_move(&mc.args[0]);
            return format!(
                "[&]() {{ auto&& _haystack = {}; auto&& _needle = {}; for (const auto& _item : _haystack) {{ if constexpr (requires {{ _item == _needle; }}) {{ if (_item == _needle) return true; }} else if constexpr (requires {{ _needle == _item; }}) {{ if (_needle == _item) return true; }} }} return false; }}()",
                receiver, needle
            );
        }
        if mc.method == "contains" && mc.args.len() == 1 {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let needle = self.emit_expr_maybe_move(&mc.args[0]);
            return format!("rusty::contains({}, {})", receiver, needle);
        }
        if mc.method == "starts_with" && mc.args.len() == 1 {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let prefix = self.emit_expr_maybe_move(&mc.args[0]);
            return format!("rusty::starts_with({}, {})", receiver, prefix);
        }
        if mc.method == "next_token" && mc.args.is_empty() {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return format!("rusty::next_token({})", receiver);
        }
        if mc.method == "peek_token" && mc.args.is_empty() {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return format!("rusty::peek_token({})", receiver);
        }
        if mc.method == "offset_from" && mc.args.len() == 1 {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let from = self.emit_expr_to_string(&mc.args[0]);
            return format!("rusty::offset_from({}, {})", receiver, from);
        }
        // c2rust's raw-pointer `PointerExt::c_offset_from(origin)` — the
        // element-count difference between two pointers. Emit the pointer
        // arithmetic helper directly (NOT the winnow string/span `offset_from`)
        // and keep the receiver as a pointer, so it is not auto-dereferenced
        // into a `(*ptr).c_offset_from(...)` member call on a non-struct.
        if mc.method == "c_offset_from" && mc.args.len() == 1 {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let origin = self.emit_expr_to_string(&mc.args[0]);
            return format!("rusty::ptr::offset_from({}, {})", receiver, origin);
        }
        if mc.method == "offset_for" && mc.args.len() == 1 {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let pred = self.emit_expr_to_string(&mc.args[0]);
            return format!("rusty::offset_for({}, {})", receiver, pred);
        }
        if mc.method == "contains_token" && mc.args.len() == 1 {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let token = self.emit_expr_to_string(&mc.args[0]);
            return format!("rusty::contains_token({}, {})", receiver, token);
        }
        if mc.method == "find_slice" && mc.args.len() == 1 {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let pattern = self.emit_expr_to_string(&mc.args[0]);
            return format!("rusty::find_slice({}, {})", receiver, pattern);
        }
        if mc.method == "next_slice" && mc.args.len() == 1 {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let offset = self.emit_expr_to_string(&mc.args[0]);
            return format!("rusty::next_slice({}, {})", receiver, offset);
        }
        if mc.method == "checkpoint" && mc.args.is_empty() {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return format!("rusty::checkpoint({})", receiver);
        }
        if mc.method == "reset" && mc.args.len() == 1 {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let checkpoint = self.emit_expr_to_string(&mc.args[0]);
            return format!("rusty::reset({}, {})", receiver, checkpoint);
        }
        if mc.method == "eof_offset" && mc.args.is_empty() {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return format!("rusty::eof_offset({})", receiver);
        }
        if matches!(mc.method.to_string().as_str(), "min" | "max") && mc.args.len() == 1 {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let helper = if mc.method == "min" {
                "rusty::min"
            } else {
                "rusty::max"
            };
            return format!(
                "{}({}, {})",
                helper,
                receiver,
                self.emit_expr_maybe_move(&mc.args[0])
            );
        }
        if matches!(mc.method.to_string().as_str(), "size" | "align")
            && mc.args.is_empty()
            && self
                .infer_simple_expr_type(&mc.receiver)
                .is_some_and(|ty| self.is_known_alloc_layout_type(&ty))
        {
            let field_name = if mc.method == "size" { "size" } else { "align" };
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let member_op = if self.method_receiver_uses_pointer_member_access(&mc.receiver) {
                "->"
            } else {
                "."
            };
            return format!("{}{}{}", receiver, member_op, field_name);
        }
        // Rust `str::as_bytes()` → `rusty::as_bytes()`.
        // In C++, std::string_view doesn't have as_bytes(), so we use a helper.
        // A LOCAL type declaring its own method keeps method-call syntax
        // (libyaml::cstr::CStr::to_bytes) — the str helper only owns the name
        // for string-like receivers.
        if mc.method == "as_bytes"
            && mc.args.is_empty()
            && !self.receiver_declares_inherent_method(&mc.receiver, "as_bytes")
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return format!("rusty::as_bytes({})", receiver);
        }
        if mc.method == "to_bytes"
            && mc.args.is_empty()
            && !self.receiver_declares_inherent_method(&mc.receiver, "to_bytes")
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return format!("rusty::as_bytes({})", receiver);
        }
        // Rust `str::bytes()` returns an iterator; keep byte-span fallback only
        // for non-string-like surfaces.
        if mc.method == "bytes" && mc.args.is_empty() {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let receiver_is_string_like = self.expr_is_string_view_like(&mc.receiver)
                || self
                    .infer_simple_expr_type(&mc.receiver)
                    .as_ref()
                    .is_some_and(|ty| self.type_is_string_view_like(ty))
                || self.expected_type_is_string_view(expected_ty)
                // `scalar[1..].bytes()` — an index/slice whose BASE is
                // string-like yields a string view.
                || matches!(
                    self.peel_paren_group_expr(&mc.receiver),
                    syn::Expr::Index(idx)
                        if self
                            .infer_simple_expr_type(&idx.expr)
                            .as_ref()
                            .is_some_and(|ty| self.type_is_string_view_like(ty))
                )
                // The emitted subscript-view lowering itself: a slice_from
                // result is a view, never an io::Read (Read::bytes is the
                // only non-string Rust .bytes()); shadowed rebind locals
                // defeat the type-based checks above.
                || receiver.trim_start().starts_with("rusty::slice_from(");
            if receiver_is_string_like {
                // Rust str::bytes() iterates u8 — a byte span over the view
                // is range-compatible with every iterator consumer.
                return format!(
                    "rusty::as_bytes(rusty::to_string_view({}))",
                    receiver
                );
            }
            return format!("rusty::io::bytes({})", receiver);
        }
        // Rust range `start/end` accessors can lower to runtime field helpers.
        // This avoids collisions with iterator `end()` members and private
        // storage layouts (`end_`) across range adapters.
        if matches!(mc.method.to_string().as_str(), "start" | "end")
            && mc.args.is_empty()
            && let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(&mc.receiver)
            && path_expr.path.segments.len() == 1
            && path_expr.path.segments[0].ident == "self"
            && let Some(self_name) = self.current_self_path_override()
        {
            return if mc.method == "start" {
                format!("rusty::field_start({})", self_name)
            } else {
                format!("rusty::field_end({})", self_name)
            };
        }
        if matches!(mc.method.to_string().as_str(), "start" | "end")
            && mc.args.is_empty()
            && self
                .infer_simple_expr_type(&mc.receiver)
                .as_ref()
                .is_some_and(|ty| self.type_is_range_with_private_end_field(ty))
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return if mc.method == "start" {
                format!("rusty::field_start({})", receiver)
            } else {
                format!("rusty::field_end({})", receiver)
            };
        }
        // Rust `value.to_string()` (Display trait) → `rusty::to_string(value)`.
        // C++ user-defined types don't have a `.to_string()` member; dispatch
        // through a SFINAE helper that tries `.to_string()`, then `std::to_string()`.
        if mc.method == "to_string" && mc.args.is_empty() {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return format!("rusty::to_string({})", receiver);
        }
        if mc.method == "join" && mc.args.len() == 1 {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let sep = self.emit_expr_maybe_move(&mc.args[0]);
            return format!("rusty::join({}, {})", receiver, sep);
        }
        // Rust float predicate methods are inherent scalar helpers.
        if mc.method == "is_finite" && mc.args.is_empty() {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return format!("rusty::is_finite({})", receiver);
        }
        if mc.method == "classify"
            && mc.args.is_empty()
            && self
                .infer_simple_expr_type(&mc.receiver)
                .as_ref()
                .is_some_and(|ty| self.is_known_float_like_type(ty))
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return format!("rusty::classify_float({})", receiver);
        }
        if mc.method == "unsigned_abs" && mc.args.is_empty() {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!(
                "([&]() {{ auto&& _v = {recv}; using _V = std::remove_cv_t<std::remove_reference_t<decltype(_v)>>; using _U = std::make_unsigned_t<_V>; if constexpr (std::is_signed_v<_V>) {{ auto _u = static_cast<_U>(_v); return (_v < 0) ? static_cast<_U>(0) - _u : _u; }} else {{ return static_cast<_U>(_v); }} }})()",
                recv = receiver
            );
        }
        if mc.method == "is_nan" && mc.args.is_empty() {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return format!("rusty::is_nan({})", receiver);
        }
        if mc.method == "is_infinite" && mc.args.is_empty() {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return format!("rusty::is_infinite({})", receiver);
        }
        if mc.method == "is_sign_negative" && mc.args.is_empty() {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return format!("rusty::is_sign_negative({})", receiver);
        }
        if mc.method == "is_sign_positive" && mc.args.is_empty() {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return format!("rusty::is_sign_positive({})", receiver);
        }
        if mc.method == "copysign" && mc.args.len() == 1 {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let rhs = self.emit_expr_maybe_move(&mc.args[0]);
            return format!("std::copysign({}, {})", receiver, rhs);
        }
        if mc.method == "deref" && mc.args.is_empty() {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return format!("rusty::deref_ref({})", receiver);
        }
        if mc.method == "deref_mut" && mc.args.is_empty() {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            if self.in_deref_mut_method_scope()
                && self.should_fallback_to_deref_ref_in_deref_mut_scope()
            {
                return format!("rusty::deref_ref({})", receiver);
            }
            return format!("rusty::deref_mut({})", receiver);
        }
        if mc.method == "swap"
            && mc.args.len() == 2
            && self.should_lower_swap_method_call_to_index_swap(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let lhs_idx = self.emit_expr_to_string(&mc.args[0]);
            let rhs_idx = self.emit_expr_to_string(&mc.args[1]);
            let swap_view_expr =
                if self.should_lower_swap_method_call_via_deref_mut_view(&mc.receiver) {
                    "rusty::deref_mut(_swap_recv)"
                } else {
                    "_swap_recv"
                };
            return format!(
                "[&]() {{ auto&& _swap_recv = {}; auto&& _swap_view = {}; const auto _swap_i = {}; const auto _swap_j = {}; const auto _swap_len = rusty::len(_swap_view); if (!(_swap_i < _swap_len && _swap_j < _swap_len)) {{ rusty::panicking::panic(\"index out of bounds\"); }} rusty::mem::swap(_swap_view[_swap_i], _swap_view[_swap_j]); }}()",
                receiver, swap_view_expr, lhs_idx, rhs_idx
            );
        }
        if mc.method == "to_vec"
            && mc.args.is_empty()
            && (self.is_to_vec_runtime_receiver_expr(&mc.receiver)
                || self.expr_lowers_to_slice_or_span_view(&mc.receiver)
                || (self.infer_simple_expr_type(&mc.receiver).is_none()
                    && !self.receiver_has_inherent_method_named(&mc.receiver, "to_vec")))
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            // Prefer `rusty::Vec<T>::from_iter(value)` when we can identify
            // the element type from a slice/array receiver — the generic
            // `rusty::to_vec` returns `std::vector<Elem>`, which doesn't
            // implicitly convert to `rusty::Vec<Elem>` (the transpiled Vec
            // doesn't expose a `std::vector` ctor). Brace-init sites like
            // `Content_ByteBuf{value.to_vec()}` in serde's content
            // serialization path need a `rusty::Vec<u8>` directly.
            if let Some(elem_cpp) = self
                .infer_simple_expr_type(&mc.receiver)
                .as_ref()
                .and_then(|ty| self.byte_slice_elem_cpp_type(ty))
            {
                return format!("rusty::Vec<{}>::from_iter({})", elem_cpp, receiver);
            }
            // Codegen-time receiver-type inference can't always reach a
            // parameter inside a generic visitor method (serde_core's
            // `ContentVisitor::visit_bytes<E>(self, value: &[u8])` is the
            // canonical case — `value` is a bare ident, infer returns
            // None, and `expr_lowers_to_slice_or_span_view` also falls
            // through). But we're already inside the to_vec block, which
            // means the receiver is iterable at C++ level. Recover the
            // element type via `decltype` so
            // `rusty::Vec<T>::from_iter` still pins T at C++ compile
            // time. The std::remove_cvref_t<decltype(*std::begin(value))>
            // shape works for std::span, std::array, std::vector, and
            // the transpiled slice views uniformly. Brace-init sites
            // like `Content_ByteBuf{value.to_vec()}` in serde's content
            // serialization path need a `rusty::Vec<u8>` directly —
            // the old `rusty::to_vec` returned `std::vector<u8>` which
            // doesn't convert.
            return format!(
                "rusty::Vec<std::remove_cvref_t<decltype(*std::begin({0}))>>::from_iter({0})",
                receiver
            );
        }
        if mc.method == "into_boxed_slice" && mc.args.is_empty() {
            let receiver = self.emit_expr_maybe_move(&mc.receiver);
            return format!("rusty::into_boxed_slice({})", receiver);
        }
        if mc.method == "into_boxed_str" && mc.args.is_empty() {
            let receiver = self.emit_expr_maybe_move(&mc.receiver);
            return format!("rusty::into_boxed_str({})", receiver);
        }
        if let Some(description_call) = self.try_emit_error_description_dispatch_call(mc) {
            return description_call;
        }
        if mc.method == "parse" && mc.args.is_empty() {
            let parsed_ty = self
                .method_call_single_turbofish_type(mc)
                .map(|ty| self.peel_reference_paren_group_type(ty))
                .or_else(|| {
                    self.expected_result_type_arg(expected_ty, 0)
                        .map(|ty| self.peel_reference_paren_group_type(ty))
                })
                .or_else(|| {
                    self.current_return_type_hint()
                        .and_then(|ret| self.expected_option_type_arg(Some(ret)))
                        .map(|ty| self.peel_reference_paren_group_type(ty))
                })
                .or_else(|| expected_ty.map(|ty| self.peel_reference_paren_group_type(ty)));
            if let Some(parsed_ty) = parsed_ty {
                let receiver = self.emit_expr_to_string(&mc.receiver);
                let parsed_cpp = self.map_type(parsed_ty);
                let parsed_cpp_has_explicit_generic_args = parsed_cpp.contains('<');
                let mut parsed_assoc_owner_cpp = if parsed_cpp_has_explicit_generic_args {
                    parsed_cpp.clone()
                } else {
                    self.expected_type_last_ident(parsed_ty)
                        .filter(|name| self.resolve_scope_import_binding_path(name).is_some())
                        .map(|name| escape_cpp_keyword(&name))
                        .unwrap_or_else(|| parsed_cpp.clone())
                };
                if !parsed_assoc_owner_cpp.contains("::") {
                    let mut owner_alias_matches: Vec<String> = self
                        .import_alias_names
                        .iter()
                        .filter_map(|alias| {
                            let target = self.resolve_scope_import_binding_path(alias)?;
                            if !target.contains("::") {
                                return None;
                            }
                            let target_tail = target
                                .trim_start_matches("::")
                                .rsplit("::")
                                .next()
                                .unwrap_or_default();
                            let escaped_tail = escape_cpp_keyword(target_tail);
                            if target_tail == parsed_assoc_owner_cpp
                                || escaped_tail == parsed_assoc_owner_cpp
                            {
                                Some(escape_cpp_keyword(alias))
                            } else {
                                None
                            }
                        })
                        .collect();
                    owner_alias_matches.sort();
                    owner_alias_matches.dedup();
                    if owner_alias_matches.len() == 1 {
                        parsed_assoc_owner_cpp = owner_alias_matches.pop().expect("len() checked");
                    }
                }
                let parsed_scalar = self.canonical_into_target_cpp_type(&parsed_cpp);
                let parse_via_runtime = is_numeric_cpp_scalar_type(parsed_scalar.as_str())
                    || matches!(parsed_scalar.as_str(), "bool" | "char32_t")
                    || parsed_scalar.contains("__int128")
                    || parsed_cpp.contains("__int128");
                if parse_via_runtime {
                    return format!("rusty::str_runtime::parse<{}>({})", parsed_cpp, receiver);
                }
                // Non-numeric parse targets generally use trait-like
                // `T::from_str(...)`. For local type aliases (for example
                // `crate::RustDocument` in toml compliance tests), lowering to
                // the free `from_str<T>(...)` helper preserves Rust FromStr
                // behavior even when the alias target itself has no static member.
                let parsed_alias_name = self.expected_type_last_ident(parsed_ty);
                let parsed_cpp_owner_key = parsed_cpp
                    .trim_start_matches("typename ")
                    .trim()
                    .trim_start_matches("::")
                    .split('<')
                    .next()
                    .unwrap_or_default()
                    .trim();
                let parsed_crate_alias_name = match parsed_ty {
                    syn::Type::Path(tp)
                        if tp.qself.is_none()
                            && tp.path.segments.len() == 2
                            && tp
                                .path
                                .segments
                                .first()
                                .is_some_and(|seg| seg.ident == "crate") =>
                    {
                        tp.path.segments.last().map(|seg| seg.ident.to_string())
                    }
                    _ => None,
                };
                let parsed_is_crate_import_alias =
                    parsed_crate_alias_name.as_deref().is_some_and(|alias| {
                        self.resolve_scope_import_binding_path(alias)
                            .or_else(|| self.resolve_scope_import_binding_path_for_scope("", alias))
                            .is_some_and(|target| {
                                let normalized = target.trim_start_matches("::");
                                !normalized.is_empty() && normalized != alias
                            })
                    });
                let parsed_owner_has_static_from_str =
                    parsed_alias_name.as_deref().is_some_and(|name| {
                        self.lookup_owner_method_has_receiver(name, "from_str")
                            .is_some_and(|has_receiver| !has_receiver)
                    }) || (!parsed_cpp_owner_key.is_empty()
                        && self
                            .lookup_owner_method_has_receiver(parsed_cpp_owner_key, "from_str")
                            .is_some_and(|has_receiver| !has_receiver))
                        || self
                            .lookup_owner_method_has_receiver(&parsed_assoc_owner_cpp, "from_str")
                            .is_some_and(|has_receiver| !has_receiver);
                let parsed_cpp_owner_is_unqualified = !parsed_cpp_owner_key.contains("::");
                let parsed_is_local_alias = parsed_is_crate_import_alias
                    || (parsed_cpp_owner_is_unqualified
                        && parsed_alias_name
                            .as_deref()
                            .is_some_and(|name| self.type_key_is_declared_alias(name)))
                    || (parsed_cpp_owner_is_unqualified
                        && !parsed_cpp_owner_key.is_empty()
                        && self.type_key_is_declared_alias(parsed_cpp_owner_key));
                if !parsed_owner_has_static_from_str && parsed_is_local_alias {
                    return format!(
                        "from_str<{}>(rusty::to_string_view({}))",
                        parsed_cpp, receiver
                    );
                }
                return format!(
                    "{}::from_str(rusty::to_string_view({}))",
                    parsed_assoc_owner_cpp, receiver
                );
            }
        }
        if mc.method == "repeat"
            && mc.args.len() == 1
            && matches!(
                self.peel_paren_group_expr(&mc.receiver),
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(_),
                    ..
                })
            )
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let count = self.emit_expr_to_string(&mc.args[0]);
            return format!("rusty::String::from({}).repeat({})", receiver, count);
        }
        if mc.method == "iter"
            && mc.args.is_empty()
            && !self.receiver_type_has_user_iter_method(&mc.receiver, "iter")
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return format!("rusty::iter({})", receiver);
        }
        if mc.method == "iter_mut"
            && mc.args.is_empty()
            && !self.receiver_type_has_user_iter_method(&mc.receiver, "iter_mut")
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return format!("rusty::iter_mut({})", receiver);
        }
        if mc.method == "into_iter" && mc.args.is_empty() {
            if self.should_bridge_direct_into_iter_receiver_to_iter(&mc.receiver) {
                let receiver_expected = self
                    .infer_into_iter_receiver_expected_type_from_call_expected(
                        &mc.receiver,
                        expected_ty,
                    );
                let receiver = if let Some(receiver_expected) = receiver_expected.as_ref() {
                    self.emit_expr_to_string_with_expected_and_move_if_needed(
                        &mc.receiver,
                        Some(receiver_expected),
                    )
                } else {
                    self.emit_expr_maybe_move(&mc.receiver)
                };
                return format!("rusty::iter({})", receiver);
            }
        }
        if mc.method == "find"
            && mc.args.len() == 1
            && (self.is_iterator_like_receiver_expr(&mc.receiver)
                || self.is_probably_iterator_receiver_expr(&mc.receiver))
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let pred = self.emit_expr_to_string(&mc.args[0]);
            return format!("rusty::find({}, {})", receiver, pred);
        }
        if mc.method == "collect" && mc.args.is_empty() {
            // For unresolved generic receivers, bridge `.into_iter()` through
            // `rusty::iter(...)` since concrete C++ method surfaces may not exist.
            let receiver = if let syn::Expr::MethodCall(inner_mc) = &*mc.receiver {
                if inner_mc.method == "into_iter" && inner_mc.args.is_empty() {
                    if self.should_bridge_into_iter_receiver_to_iter(&inner_mc.receiver) {
                        let inner = self.emit_expr_maybe_move(&inner_mc.receiver);
                        format!("rusty::iter({})", inner)
                    } else {
                        self.emit_expr_to_string(&mc.receiver)
                    }
                } else {
                    self.emit_expr_to_string(&mc.receiver)
                }
            } else {
                self.emit_expr_to_string(&mc.receiver)
            };
            let mut unresolved_vec_placeholder_collect = false;
            // Try turbofish type arg first: `collect::<T>()` → `T::from_iter(receiver)`
            if let Some(turbofish_ty) = self.method_call_single_turbofish_type(mc) {
                let mut collect_type = self.map_type(turbofish_ty);
                if let Some(explicit) =
                    self.recover_explicit_owner_type_from_type(turbofish_ty, &collect_type)
                {
                    collect_type = explicit;
                }
                if collect_type.starts_with("rusty::Vec<")
                    && type_string_has_auto_placeholder(&collect_type)
                {
                    unresolved_vec_placeholder_collect = true;
                }
                if collect_type != "auto"
                    && !collect_type.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&collect_type)
                    && self.collect_target_supports_from_iter(&collect_type)
                {
                    let receiver_for_collect = if collect_type.starts_with("rusty::Vec<") {
                        self.expected_vec_element_type(Some(turbofish_ty))
                            .cloned()
                            .map(|item_ty| {
                                let iter_expected: syn::Type =
                                    parse_quote!(impl Iterator<Item = #item_ty>);
                                self.emit_expr_to_string_with_expected(
                                    &mc.receiver,
                                    Some(&iter_expected),
                                )
                            })
                            .unwrap_or_else(|| receiver.clone())
                    } else {
                        receiver.clone()
                    };
                    return format!("{}::from_iter({})", collect_type, receiver_for_collect);
                }
            }
            if let Some(expected) = expected_ty {
                let resolved_expected =
                    self.resolve_expected_type_with_iter_hint(expected, &mc.receiver);
                let mut expected_cpp = self.map_type(&resolved_expected);
                if let Some(explicit) =
                    self.recover_explicit_owner_type_from_type(&resolved_expected, &expected_cpp)
                {
                    expected_cpp = explicit;
                }
                if expected_cpp.starts_with("rusty::Vec<")
                    && type_string_has_auto_placeholder(&expected_cpp)
                {
                    unresolved_vec_placeholder_collect = true;
                }
                if expected_cpp != "auto"
                    && !expected_cpp.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&expected_cpp)
                    && self.collect_target_supports_from_iter(&expected_cpp)
                {
                    let receiver_for_collect = if expected_cpp.starts_with("rusty::Vec<") {
                        self.expected_vec_element_type(Some(&resolved_expected))
                            .cloned()
                            .map(|item_ty| {
                                let iter_expected: syn::Type =
                                    parse_quote!(impl Iterator<Item = #item_ty>);
                                self.emit_expr_to_string_with_expected(
                                    &mc.receiver,
                                    Some(&iter_expected),
                                )
                            })
                            .unwrap_or_else(|| receiver.clone())
                    } else {
                        receiver.clone()
                    };
                    return format!("{}::from_iter({})", expected_cpp, receiver_for_collect);
                }
            }
            if unresolved_vec_placeholder_collect {
                return format!("rusty::collect_range({})", receiver);
            }
            if Self::is_range_expression(&mc.receiver) {
                return format!("rusty::collect_range({})", receiver);
            }
            if self.is_iterator_like_receiver_expr(&mc.receiver)
                || self.is_probably_iterator_receiver_expr(&mc.receiver)
            {
                return format!("rusty::collect_range({})", receiver);
            }
            // Unknown receiver shape — leave `.collect()` intact rather than
            // blindly rewriting to `rusty::collect_range`.
            return format!("{}.collect()", receiver);
        }
        if mc.method == "by_ref"
            && mc.args.is_empty()
            && self.is_iterator_like_receiver_expr(&mc.receiver)
        {
            return self.emit_expr_to_string(&mc.receiver);
        }
        if mc.method == "take"
            && mc.args.len() == 1
            && self.is_iterator_like_receiver_expr(&mc.receiver)
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let count = self.emit_expr_maybe_move(&mc.args[0]);
            return format!("rusty::take({}, {})", receiver, count);
        }
        if mc.method == "chain" && mc.args.len() == 1 {
            if let Some(item_ty) = expected_ty
                .and_then(|ty| self.extract_iter_item_type_from_type(ty))
                .filter(|ty| self.type_is_concrete_hint_candidate(ty))
            {
                let receiver = self.emit_chain_arg_with_item_hint(&mc.receiver, &item_ty);
                let rhs = self.emit_chain_arg_with_item_hint(&mc.args[0], &item_ty);
                return format!("rusty::chain({}, {})", receiver, rhs);
            }
            let receiver = self.emit_expr_maybe_move(&mc.receiver);
            let rhs = self.emit_expr_maybe_move(&mc.args[0]);
            return format!("rusty::chain({}, {})", receiver, rhs);
        }
        if mc.method == "skip"
            && mc.args.len() == 1
            && (self.is_iterator_like_receiver_expr(&mc.receiver)
                || self.is_probably_iterator_receiver_expr(&mc.receiver))
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let count = self.emit_expr_maybe_move(&mc.args[0]);
            return format!("rusty::skip({}, {})", receiver, count);
        }
        if mc.method == "scan"
            && mc.args.len() == 2
            && (self.is_iterator_like_receiver_expr(&mc.receiver)
                || self.is_probably_iterator_receiver_expr(&mc.receiver))
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let state = self.emit_expr_maybe_move(&mc.args[0]);
            let scanner = self.emit_scan_callable_arg(&mc.receiver, &mc.args[1]);
            return format!("rusty::scan({}, {}, {})", receiver, state, scanner);
        }
        if mc.method == "filter"
            && mc.args.len() == 1
            && !self.receiver_is_option_or_result_like_expr(&mc.receiver)
            && (self.is_iterator_like_receiver_expr(&mc.receiver)
                || self.is_probably_iterator_receiver_expr(&mc.receiver))
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let predicate = self.emit_expr_maybe_move(&mc.args[0]);
            return format!("rusty::filter({}, {})", receiver, predicate);
        }
        if mc.method == "filter_map"
            && mc.args.len() == 1
            && !self.receiver_is_option_or_result_like_expr(&mc.receiver)
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let mapper = self.emit_expr_maybe_move(&mc.args[0]);
            return format!("rusty::filter_map({}, {})", receiver, mapper);
        }
        if let Some(enumerate_call) = self.try_emit_iter_enumerate_call(mc) {
            return enumerate_call;
        }
        if let Some(rev_call) = self.try_emit_iter_rev_call(mc) {
            return rev_call;
        }
        if let Some(copied_call) = self.try_emit_iter_copied_cloned_call(mc) {
            return copied_call;
        }
        if let Some(bs_call) = self.try_emit_slice_binary_search_call(mc) {
            return bs_call;
        }
        // BuildHasher::hash_one routes through the prelude free fn
        // (member-preference; inert builders use rusty's deterministic
        // hashing — indexmap never asserts hash VALUES).
        if mc.method == "hash_one" && mc.args.len() == 1 {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let arg = self.emit_expr_maybe_move(&mc.args[0]);
            return format!("rusty::hash_one({}, {})", receiver, arg);
        }
        // Note: is_some()/is_none() are kept as-is for rusty::Option (which has
        // these methods). The has_value() rewrite was only needed for std::optional
        // but incorrectly matched rusty::Option from iterator .next() calls.
        if mc.method == "is_some"
            && mc.args.is_empty()
            && self
                .infer_simple_expr_type(&mc.receiver)
                .is_some_and(|ty| self.is_std_optional_syn_type(&ty))
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let member_op = if self.method_receiver_uses_pointer_member_access(&mc.receiver) {
                "->"
            } else {
                "."
            };
            return format!("{}{}has_value()", receiver, member_op);
        }
        if mc.method == "is_none"
            && mc.args.is_empty()
            && self
                .infer_simple_expr_type(&mc.receiver)
                .is_some_and(|ty| self.is_std_optional_syn_type(&ty))
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let member_op = if self.method_receiver_uses_pointer_member_access(&mc.receiver) {
                "->"
            } else {
                "."
            };
            return format!("!{}{}has_value()", receiver, member_op);
        }
        if mc.method == "unwrap"
            && mc.args.is_empty()
            && self
                .infer_simple_expr_type(&mc.receiver)
                .is_some_and(|ty| self.is_std_optional_syn_type(&ty))
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let member_op = if self.method_receiver_uses_pointer_member_access(&mc.receiver) {
                "->"
            } else {
                "."
            };
            return format!("{}{}value()", receiver, member_op);
        }
        if mc.method == "unwrap_unchecked" && mc.args.is_empty() {
            if let syn::Expr::MethodCall(inner_mc) = self.peel_paren_group_expr(&mc.receiver)
                && inner_mc.method == "take"
                && inner_mc.args.is_empty()
                && self
                    .infer_simple_expr_type(&inner_mc.receiver)
                    .is_some_and(|ty| self.is_std_optional_syn_type(&ty))
            {
                let receiver = self.emit_expr_to_string(&inner_mc.receiver);
                return format!("rusty::mem::take({}).value()", receiver);
            }
            if self
                .infer_simple_expr_type(&mc.receiver)
                .is_some_and(|ty| self.is_std_optional_syn_type(&ty))
            {
                let raw_receiver = self.emit_expr_to_string(&mc.receiver);
                let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                    format!("({})", raw_receiver)
                } else {
                    raw_receiver
                };
                let member_op = if self.method_receiver_uses_pointer_member_access(&mc.receiver) {
                    "->"
                } else {
                    "."
                };
                return format!("{}{}value()", receiver, member_op);
            }
        }
        if let Some(io_call) = self.try_emit_io_read_write_buffer_call(mc) {
            return io_call;
        }
        if let Some(filter_map_call) = self.try_emit_iter_filter_map_call(mc) {
            return filter_map_call;
        }
        if let Some(array_map_call) = self.try_emit_fixed_array_map_call(mc, expected_ty) {
            return array_map_call;
        }
        if let Some(map_call) = self.try_emit_iter_map_call(mc, expected_ty) {
            return map_call;
        }
        if mc.method == "map"
            && mc.args.len() == 1
            && !self.receiver_is_option_or_result_like_expr(&mc.receiver)
            && (self.receiver_is_fixed_array_like_expr(&mc.receiver)
                || self.expr_lowers_to_slice_or_span_view(&mc.receiver)
                || self.should_lower_swap_method_call_to_index_swap(&mc.receiver))
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            // Path-callable mappers (`Some`, `Option::unwrap`) need their
            // dedicated lowering — a raw path is not a valid C++ callable.
            let mapper = self
                .try_emit_assoc_method_path_as_forwarding_lambda(&mc.args[0])
                .or_else(|| {
                    self.infer_iter_item_type_with_generic_fallback(&mc.receiver)
                        .and_then(|item_ty| {
                            self.try_emit_path_callable_arg_to_target(&mc.args[0], &item_ty)
                        })
                })
                .unwrap_or_else(|| self.emit_expr_maybe_move(&mc.args[0]));
            return format!("rusty::map({}, {})", receiver, mapper);
        }
        if let Some(try_fold_call) = self.try_emit_iter_try_fold_call(mc, expected_ty) {
            return try_fold_call;
        }
        if let Some(fold_call) = self.try_emit_iter_fold_call(mc, expected_ty) {
            return fold_call;
        }
        if let Some(rfold_call) = self.try_emit_iter_rfold_call(mc) {
            return rfold_call;
        }
        if let Some(all_call) = self.try_emit_iter_all_call(mc) {
            return all_call;
        }
        if let Some(count_call) = self.try_emit_iter_count_call(mc) {
            return count_call;
        }
        if let Some(sum_call) = self.try_emit_iter_sum_call(mc) {
            return sum_call;
        }
        if let Some(step_by_call) = self.try_emit_iter_step_by_call(mc) {
            return step_by_call;
        }
        if let Some(flat_map_call) = self.try_emit_iter_flat_map_call(mc) {
            return flat_map_call;
        }
        if let Some(for_each_call) = self.try_emit_iter_for_each_call(mc) {
            return for_each_call;
        }
        if let Some(all_any_call) = self.try_emit_iter_all_any_call(mc) {
            return all_any_call;
        }
        if let Some(try_for_each_call) = self.try_emit_iter_try_for_each_call(mc) {
            return try_for_each_call;
        }
        if mc.method == "as_ref" && mc.args.is_empty() {
            if self.expr_lowers_to_slice_or_span_view(&mc.receiver)
                || self.is_to_vec_runtime_receiver_expr(&mc.receiver)
            {
                let receiver = self.emit_expr_to_string(&mc.receiver);
                return format!("rusty::as_slice({})", receiver);
            }
            let receiver_is_known_cow = self.expr_is_known_cow_like(&mc.receiver);
            // Fallback for unresolved closure-parameter chains like
            // `key.get_ref().as_ref()` where local type inference cannot recover
            // `Cow` but Rust surface expects a string-view borrow.
            let receiver_looks_like_get_ref_chain = matches!(self.peel_paren_group_expr(&mc.receiver), syn::Expr::MethodCall(inner_mc)
                    if inner_mc.method == "get_ref" && inner_mc.args.is_empty());
            if receiver_is_known_cow
                || receiver_looks_like_get_ref_chain
                || self.expr_is_self_inner_field(&mc.receiver)
            {
                let raw_receiver = self.emit_expr_to_string(&mc.receiver);
                let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                    format!("({})", raw_receiver)
                } else {
                    raw_receiver
                };
                return format!("rusty::to_string_view({})", receiver);
            }
        }
        if mc.method == "as_mut"
            && mc.args.is_empty()
            && (self.expr_lowers_to_slice_or_span_view(&mc.receiver)
                || self.is_to_vec_runtime_receiver_expr(&mc.receiver))
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return format!("rusty::as_mut_slice({})", receiver);
        }

        let method_name = mc.method.to_string();
        // Record a `.map(closure)` receiver's Ok/Some type so the closure's
        // destructured param types its bindings (see
        // pending_map_closure_input_type). Only for a closure arg — path
        // callables (Some/From) take a different route and would otherwise
        // leave a stale hint.
        if method_name == "map"
            && mc.args.len() == 1
            && matches!(self.peel_paren_group_expr(&mc.args[0]), syn::Expr::Closure(_))
        {
            let input_ty = self.infer_simple_expr_type(&mc.receiver).and_then(|recv_ty| {
                // The receiver is Result/Option, possibly via a crate alias
                // (`Result<T>` / `R<T>`); the closure input is the Ok/Some type
                // = the first generic arg. expected_result/option_type_arg miss
                // aliases, so fall back to the first type arg.
                self.expected_result_type_arg(Some(&recv_ty), 0)
                    .or_else(|| self.expected_option_type_arg(Some(&recv_ty)))
                    .cloned()
                    .or_else(|| Self::first_generic_type_arg(&recv_ty).cloned())
                    .filter(|ty| {
                        !self.type_contains_infer(ty)
                            && !self.type_contains_unresolved_placeholder_like(ty)
                    })
            });
            *self.pending_map_closure_input_type.borrow_mut() = input_ty;
            // RETURN-side: the map call's own expected Option/Result payload
            // is the closure's return. Threaded only for REFERENCE payloads
            // (`Option<&mut V>`) — an unannotated lambda's deduced return
            // decays V& to V, so the annotation must be forced; value
            // payloads keep the pre-existing deduction behavior. Tail-return
            // map calls carry no arg-expected; fall back to the ambient
            // return-type hint (the enclosing closure's annotation).
            let return_payload = expected_ty
                .or_else(|| self.current_return_type_hint())
                .and_then(|expected| {
                    self.expected_option_type_arg(Some(expected))
                        .or_else(|| self.expected_result_type_arg(Some(expected), 0))
                        .cloned()
                });
            *self.pending_map_closure_return_type.borrow_mut() = return_payload.filter(|ty| {
                matches!(self.peel_paren_group_type(ty), syn::Type::Reference(_))
                    && !self.type_contains_infer(ty)
                    && !self.type_contains_unresolved_placeholder_like(ty)
            });
        }
        if matches!(method_name.as_str(), "eq" | "ne")
            && mc.args.len() == 1
            && (self.expr_lowers_to_slice_or_span_view(&mc.receiver)
                || self.is_to_vec_runtime_receiver_expr(&mc.receiver))
        {
            let lhs = self.emit_expr_to_string(&mc.receiver);
            let rhs = self.emit_expr_maybe_move(&mc.args[0]);
            let op = if method_name == "eq" { "==" } else { "!=" };
            return format!("({}) {} ({})", lhs, op, rhs);
        }
        // Generic PartialEq dispatch: when `.eq()`/`.ne()` is called on a
        // receiver that does NOT have an inherent eq/ne method (typical case —
        // PartialEq is a trait, primitive receivers have no `.eq()` method),
        // route through rusty::cmp::eq/ne which SFINAE-dispatches to .eq() if
        // available, else uses operator==. This handles primitives (size_t.eq),
        // generic T parameters, and types whose PartialEq impl was lowered to a
        // free function rather than a member.
        if matches!(method_name.as_str(), "eq" | "ne")
            && mc.args.len() == 1
            && !self.receiver_has_inherent_method_named(&mc.receiver, &method_name)
        {
            let lhs = self.emit_expr_to_string(&mc.receiver);
            let rhs = self.emit_expr_maybe_move(&mc.args[0]);
            return format!("rusty::cmp::{}({}, {})", method_name, lhs, rhs);
        }
        if method_name == "fmt"
            && mc.args.len() == 1
            && (self.expr_lowers_to_slice_or_span_view(&mc.receiver)
                || self.is_to_vec_runtime_receiver_expr(&mc.receiver))
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let formatter = self.emit_expr_maybe_move(&mc.args[0]);
            return format!(
                "rusty::write_fmt({}, rusty::to_debug_string({}))",
                formatter, receiver
            );
        }
        if method_name == "fmt"
            && mc.args.len() == 1
            && !self.receiver_has_inherent_method_named(&mc.receiver, "fmt")
            && self
                .infer_simple_expr_type(&mc.receiver)
                .as_ref()
                .is_some_and(|ty| {
                    matches!(
                        self.map_type(self.peel_reference_paren_group_type(ty))
                            .as_str(),
                        "std::string_view" | "std::string" | "rusty::String"
                    )
                })
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let formatter = self.emit_expr_maybe_move(&mc.args[0]);
            return format!(
                "rusty::write_fmt({}, rusty::to_string({}))",
                formatter, receiver
            );
        }
        let mut args = Vec::with_capacity(mc.args.len());
        for (idx, arg) in mc.args.iter().enumerate() {
            let style = self
                .lookup_method_arg_pass_style(&method_name, idx)
                .or_else(|| {
                    // Rust `a.append(&mut b)`: collection append takes the
                    // other container by `&mut`; the C++ runtime/port methods
                    // take an lvalue reference (`Coll& other`), so an emitted
                    // address-of would pass an unbindable pointer.
                    if method_name == "append" && idx == 0 && mc.args.len() == 1 {
                        return Some(ArgPassStyle::Reference);
                    }
                    if method_name == "clone_from" && idx == 0 {
                        Some(ArgPassStyle::Reference)
                    } else {
                        None
                    }
                });
            let method_expected_ty = self.lookup_method_arg_expected_type(&method_name, idx);
            let owner_expected_ty = self.lookup_method_arg_expected_type_from_receiver_owner(
                &mc.receiver,
                &method_name,
                idx,
                Some(arg),
            );
            let declared_expected = owner_expected_ty.as_ref().or(method_expected_ty);
            let inferred_expected_from_receiver = self
                .infer_method_arg_expected_type_from_receiver(
                    &mc.receiver,
                    &method_name,
                    idx,
                    declared_expected,
                    Some(arg),
                );
            let inferred_expected = if declared_expected.is_none()
                && inferred_expected_from_receiver.is_none()
            {
                self.infer_slice_arg_expected_type_from_receiver(&mc.receiver, &method_name, idx)
            } else {
                None
            };
            let inferred_expected_from_call_expected_return = self
                .infer_method_arg_expected_type_from_call_expected_return(
                    &mc.receiver,
                    &method_name,
                    idx,
                    declared_expected,
                    expected_ty,
                );
            let inferred_map_callable_expected = if idx == 0 {
                self.infer_map_like_callable_expected_from_call_expected(
                    &mc.receiver,
                    &method_name,
                    expected_ty,
                )
            } else {
                None
            };
            if method_name == "map" && idx == 0 && std::env::var("RUSTY_DEBUG_MAP_EXPECTED").is_ok()
            {
                let call_expected_dbg = expected_ty
                    .map(|ty| quote::quote!(#ty).to_string())
                    .unwrap_or_else(|| "<none>".to_string());
                let inferred_dbg = inferred_map_callable_expected
                    .as_ref()
                    .map(|ty| quote::quote!(#ty).to_string())
                    .unwrap_or_else(|| "<none>".to_string());
                let receiver_ty_dbg = self
                    .infer_simple_expr_type(&mc.receiver)
                    .map(|ty| quote::quote!(#ty).to_string())
                    .unwrap_or_else(|| "<none>".to_string());
                eprintln!(
                    "DBG map expected methodcall={} call_expected={} inferred={} receiver_ty={}",
                    quote::quote!(#mc),
                    call_expected_dbg,
                    inferred_dbg,
                    receiver_ty_dbg
                );
            }
            let mut resolved_from_call_expected: Option<syn::Type> = None;
            if method_name == "get_or_try_init" && idx == 0 {
                let call_ok_ty = self.expected_result_type_arg(expected_ty, 0);
                let call_err_ty = self.expected_result_type_arg(expected_ty, 1);
                let receiver_owner_name =
                    self.infer_simple_expr_type(&mc.receiver).and_then(|ty| {
                        match self.peel_reference_paren_group_type(&ty) {
                            syn::Type::Path(tp) => {
                                tp.path.segments.last().map(|seg| seg.ident.to_string())
                            }
                            _ => None,
                        }
                    });
                let call_err_conflicts_with_ok =
                    call_ok_ty.zip(call_err_ty).is_some_and(|(ok_ty, err_ty)| {
                        Self::types_equivalent_by_tokens(
                            self.peel_reference_paren_group_type(ok_ty),
                            self.peel_reference_paren_group_type(err_ty),
                        )
                    });
                if let Some(inferred_ret) = inferred_expected_from_receiver.as_ref()
                    && let Some(call_err_ty) = call_err_ty
                    && !call_err_conflicts_with_ok
                    && self
                        .expected_result_type_arg(expected_ty, 0)
                        .zip(self.expected_result_type_arg(Some(inferred_ret), 0))
                        .is_some_and(|(call_ok_ty, inferred_ok_ty)| {
                            Self::types_equivalent_by_tokens(
                                self.peel_reference_paren_group_type(call_ok_ty),
                                self.peel_reference_paren_group_type(inferred_ok_ty),
                            )
                        })
                {
                    resolved_from_call_expected =
                        self.resolve_result_error_slot_from_expected(inferred_ret, call_err_ty);
                }
                if resolved_from_call_expected.is_none()
                    && let Some(call_err_ty) = call_err_ty
                    && !call_err_conflicts_with_ok
                {
                    let inferred_initializer_result = inferred_expected_from_receiver
                        .as_ref()
                        .cloned()
                        .or_else(|| {
                            owner_expected_ty.as_ref().and_then(|owner| {
                                self.extract_callable_return_type_from_type(owner)
                            })
                        });
                    if let Some(inferred_ret) = inferred_initializer_result {
                        resolved_from_call_expected = self
                            .resolve_result_error_slot_from_expected(&inferred_ret, call_err_ty);
                    }
                }
                if resolved_from_call_expected.is_none() && !call_err_conflicts_with_ok {
                    // Fallback to call-site expected Result<Ok, Err> only for owners
                    // whose initializer closure returns the same Ok payload shape as
                    // the method call's Result Ok payload (for example OnceBool).
                    // Skip containers like OnceCell/OnceBox/Lazy where call Ok is
                    // reference-like but initializer returns an owned value.
                    let skip_owner_call_fallback = matches!(
                        receiver_owner_name.as_deref(),
                        Some("OnceCell" | "OnceBox" | "Lazy")
                    );
                    if !skip_owner_call_fallback
                        && let (Some(call_ok_ty), Some(call_err_ty)) = (call_ok_ty, call_err_ty)
                    {
                        resolved_from_call_expected =
                            Some(parse_quote!(Result<#call_ok_ty, #call_err_ty>));
                    }
                }
            }
            let arg_expected = resolved_from_call_expected
                .as_ref()
                .or(inferred_expected_from_call_expected_return.as_ref())
                .or(inferred_map_callable_expected.as_ref())
                .or(inferred_expected_from_receiver
                    .as_ref()
                    .or(owner_expected_ty.as_ref())
                    .or(method_expected_ty)
                    .or(inferred_expected.as_ref()));
            if method_name == "get_or_insert_with"
                && idx == 0
                && self.expr_is_default_constructor_callable(arg)
            {
                let option_inner_ty = self
                    .infer_simple_expr_type(&mc.receiver)
                    .and_then(|receiver_ty| {
                        self.expected_option_type_arg(Some(&receiver_ty)).cloned()
                    })
                    .or_else(|| {
                        self.infer_local_binding_type_from_initializer(&mc.receiver)
                            .and_then(|receiver_ty| {
                                self.expected_option_type_arg(Some(&receiver_ty)).cloned()
                            })
                    })
                    .or_else(|| {
                        let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(&mc.receiver)
                        else {
                            return None;
                        };
                        if path_expr.path.segments.len() != 1 {
                            return None;
                        }
                        let local = path_expr.path.segments[0].ident.to_string();
                        let receiver_ty = self.lookup_local_binding_type(&local)?;
                        self.expected_option_type_arg(Some(&receiver_ty)).cloned()
                    })
                    .or_else(|| {
                        arg_expected.and_then(|expected| {
                            self.extract_callable_return_type_from_type(expected)
                        })
                    });
                if let Some(inner_ty) = option_inner_ty {
                    let inner_cpp = self.map_type(&inner_ty);
                    if inner_cpp != "auto"
                        && !inner_cpp.contains("/* TODO")
                        && !type_string_has_auto_placeholder(&inner_cpp)
                    {
                        args.push(format!(
                            "[&]() {{ return rusty::default_value<{}>(); }}",
                            inner_cpp
                        ));
                        continue;
                    }
                }
                let receiver_cpp = self.emit_expr_to_string(&mc.receiver);
                args.push(format!(
                    "[&]() {{ using _rusty_opt_t = std::remove_cvref_t<decltype({})>; using _rusty_inner_t = std::remove_cvref_t<typename _rusty_opt_t::value_type>; return rusty::default_value<_rusty_inner_t>(); }}",
                    receiver_cpp
                ));
                continue;
            }
            if method_name == "invalid_length"
                && idx == 1
                && let syn::Expr::Reference(reference) = self.peel_paren_group_expr(arg)
                && !self.is_stable_reference_lvalue_expr(&reference.expr)
            {
                let inner = self.emit_expr_to_string_with_expected(&reference.expr, None);
                args.push(format!("rusty::addr_of_temp({})", inner));
                continue;
            }
            if (method_name == "clone_from" || (method_name == "append" && mc.args.len() == 1))
                && idx == 0
            {
                let clone_arg = match self.peel_paren_group_expr(arg) {
                    syn::Expr::Reference(reference) => self.emit_expr_to_string(&reference.expr),
                    expr => self.emit_expr_to_string_with_expected(expr, arg_expected),
                };
                args.push(clone_arg);
                continue;
            }
            if method_name == "map_or" {
                let map_or_target_ty = expected_ty.or_else(|| self.current_return_type_hint());
                if idx == 0 {
                    if let Some(target_ty) = map_or_target_ty {
                        args.push(self.emit_expr_to_string_with_expected_and_move_if_needed(
                            arg,
                            Some(target_ty),
                        ));
                        continue;
                    }
                } else if idx == 1
                    && let Some(target_ty) = map_or_target_ty
                    && let Some(callable) =
                        self.try_emit_path_callable_arg_to_target(arg, target_ty)
                {
                    args.push(callable);
                    continue;
                }
            }
            let mut emitted_arg =
                self.emit_call_arg_with_pass_style(arg, style, arg_expected, false, None);
            let map_like_insert_key_arg = method_name == "insert"
                && idx == 0
                && (self
                    .infer_simple_expr_type(&mc.receiver)
                    .or_else(|| self.infer_local_binding_type_from_initializer(&mc.receiver))
                    .or_else(|| {
                        extract_simple_local_ident(&mc.receiver).and_then(|name| {
                            self.lookup_local_placeholder_type_hint(&name).cloned()
                        })
                    })
                    .as_ref()
                    .is_some_and(|ty| self.type_hint_is_map_like(ty))
                    || extract_simple_local_ident(&mc.receiver).is_some());
            if map_like_insert_key_arg
                && extract_simple_local_ident(arg).is_some()
                && self.is_expr_reference_like(arg)
                && !emitted_arg.starts_with("std::move(")
                && !emitted_arg.starts_with("rusty::clone(")
            {
                emitted_arg = format!("rusty::clone({})", emitted_arg);
            } else if (self.method_arg_prefers_value_move_heuristic(&method_name, idx)
                || map_like_insert_key_arg)
                && extract_simple_local_ident(arg).is_some()
                && !emitted_arg.starts_with("std::move(")
                && !emitted_arg.starts_with("rusty::clone(")
            {
                emitted_arg = format!("std::move({})", emitted_arg);
            }
            args.push(emitted_arg);
        }
        if let Some(toml_write_call) =
            self.try_emit_toml_write_method_call(mc, &method_name, &args, expected_ty)
        {
            return toml_write_call;
        }
        if method_name == "map"
            && args.len() == 1
            && let Some(callable) =
                self.try_emit_map_callable_arg_with_expected(&mc.args[0], expected_ty)
        {
            return self.emit_receiver_member_call(
                &mc.receiver,
                &method_name,
                None,
                &[callable],
                None,
            );
        }
        let mut method_template_args = self.emit_method_call_template_args(mc, &args);
        if method_template_args.is_none()
            && method_name == "init"
            && let Some(err_ty) = self.expected_result_type_arg(expected_ty, 1)
        {
            let err_cpp = self.map_type(err_ty);
            if err_cpp != "auto"
                && !err_cpp.contains("/* TODO")
                && !type_string_has_auto_placeholder(&err_cpp)
            {
                method_template_args = Some(format!("<{}>", err_cpp));
            }
        }
        if method_template_args.is_none()
            && mc.turbofish.is_none()
            && args.is_empty()
            && matches!(
                method_name.as_str(),
                "next_element" | "next_key" | "next_value"
            )
            && self.method_call_may_need_serde_access_template_arg(&mc.receiver, &method_name)
            && let Some(access_ty) = self
                .infer_serde_access_method_template_type_from_expected(&method_name, expected_ty)
        {
            let access_cpp = self.map_type(&access_ty);
            if access_cpp != "auto"
                && !access_cpp.contains("/* TODO")
                && !type_string_has_auto_placeholder(&access_cpp)
            {
                method_template_args = Some(format!("<{}>", access_cpp));
            }
        }
        if method_template_args.is_none()
            && mc.turbofish.is_none()
            && args.is_empty()
            && method_name == "next_entry"
            && let Some(access_args) =
                self.infer_serde_next_entry_template_args_from_expected(expected_ty)
        {
            method_template_args = Some(access_args);
        }
        if method_template_args.is_none()
            && mc.turbofish.is_none()
            && args.is_empty()
            && method_name == "next_entry"
            && let Some(access_args) =
                self.infer_serde_next_entry_template_args_from_in_scope_map_hint()
        {
            method_template_args = Some(access_args);
        }
        if method_template_args.is_none()
            && mc.turbofish.is_none()
            && let Some(owner_name) = self.infer_method_call_receiver_owner_name(&mc.receiver)
            && let Some(inferred_args) = self
                .infer_method_template_args_from_in_scope_undeduced_params(
                    &owner_name,
                    &method_name,
                )
        {
            method_template_args = Some(inferred_args);
        }
        if method_template_args.is_none()
            && mc.turbofish.is_none()
            && mc.args.is_empty()
            && let Some(owner_name) = self.infer_method_call_receiver_owner_name(&mc.receiver)
            && let Some(type_params) =
                self.lookup_owner_method_type_param_names(&owner_name, &method_name)
            && !type_params.is_empty()
            && type_params
                .iter()
                .all(|param| self.is_type_param_in_scope(param))
        {
            let mapped_params = type_params
                .iter()
                .map(|param| escape_cpp_keyword(param))
                .collect::<Vec<String>>()
                .join(", ");
            method_template_args = Some(format!("<{}>", mapped_params));
        }
        if matches!(method_name.as_str(), "as_ref" | "as_mut")
            && args.is_empty()
            && self.is_expr_raw_pointer_like(&mc.receiver)
        {
            let should_lower_ptr_helper = self
                .infer_simple_expr_type(&mc.receiver)
                .and_then(|receiver_ty| self.extract_pointer_pointee_info_from_type(&receiver_ty))
                .map(|(pointee_ty, is_mut_ptr)| {
                    if method_name == "as_mut" && !is_mut_ptr {
                        return false;
                    }
                    // Preserve pointee member-method dispatch (e.g. `Option<T>::as_ref`)
                    // for raw pointers to Option/Result payloads.
                    if self.option_or_result_type_args(&pointee_ty).is_some() {
                        return false;
                    }
                    if let Some(expected_inner) = self.expected_option_type_arg(expected_ty) {
                        let expected_inner =
                            self.peel_reference_paren_group_type(expected_inner).clone();
                        if !Self::types_equivalent_by_tokens(&expected_inner, &pointee_ty) {
                            return false;
                        }
                    }
                    true
                })
                .unwrap_or(false);
            if should_lower_ptr_helper {
                let raw_receiver = self.emit_expr_to_string(&mc.receiver);
                let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                    format!("({})", raw_receiver)
                } else {
                    raw_receiver
                };
                if method_name == "as_ref" {
                    return format!("rusty::ptr::as_ref({})", receiver);
                }
                return format!("rusty::ptr::as_mut({})", receiver);
            }
        }
        if method_name == "as_slice"
            && args.is_empty()
            && !self.method_receiver_uses_pointer_member_access(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::as_slice({})", receiver);
        }
        if method_name == "as_mut_slice"
            && args.is_empty()
            && !self.method_receiver_uses_pointer_member_access(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::as_mut_slice({})", receiver);
        }
        if matches!(method_name.as_str(), "clone_from_slice" | "copy_from_slice")
            && args.len() == 1
            && self.should_lower_slice_deref_method_call(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let dst = if self.expr_lowers_to_slice_or_span_view(&mc.receiver) {
                receiver
            } else {
                format!("rusty::as_mut_slice({})", receiver)
            };
            // Rust `copy_from_slice` and `clone_from_slice` share element-wise
            // semantics for our lowered span/slice surface.
            return format!("rusty::clone_from_slice({}, {})", dst, args[0]);
        }
        if method_name == "write_fmt" && args.len() == 1 {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let receiver_ty = self.infer_simple_expr_type(&mc.receiver);
            let has_receiver_write_fmt = self
                .lookup_method_arg_type_from_receiver_type(&mc.receiver, "write_fmt", 0)
                .is_some()
                || receiver_ty.as_ref().is_some_and(|ty| {
                    self.lookup_owner_method_return_type_from_receiver_type(ty, "write_fmt")
                        .is_some()
                });
            let has_receiver_write_str = self
                .lookup_method_arg_type_from_receiver_type(&mc.receiver, "write_str", 0)
                .is_some()
                || receiver_ty.as_ref().is_some_and(|ty| {
                    self.lookup_owner_method_return_type_from_receiver_type(ty, "write_str")
                        .is_some()
                });
            let has_receiver_write_char = self
                .lookup_method_arg_type_from_receiver_type(&mc.receiver, "write_char", 0)
                .is_some()
                || receiver_ty.as_ref().is_some_and(|ty| {
                    self.lookup_owner_method_return_type_from_receiver_type(ty, "write_char")
                        .is_some()
                });
            let has_receiver_write = self
                .lookup_method_arg_type_from_receiver_type(&mc.receiver, "write", 0)
                .is_some()
                || receiver_ty.as_ref().is_some_and(|ty| {
                    self.lookup_owner_method_return_type_from_receiver_type(ty, "write")
                        .is_some()
                });
            let has_receiver_write_all = self
                .lookup_method_arg_type_from_receiver_type(&mc.receiver, "write_all", 0)
                .is_some()
                || receiver_ty.as_ref().is_some_and(|ty| {
                    self.lookup_owner_method_return_type_from_receiver_type(ty, "write_all")
                        .is_some()
                });
            if has_receiver_write_fmt {
                return format!("{}.write_fmt({})", receiver, args[0]);
            }
            if has_receiver_write_str || has_receiver_write_char {
                return format!("{}.write_str({})", receiver, args[0]);
            }
            if has_receiver_write || has_receiver_write_all {
                return format!("rusty::io::write_fmt({}, {})", receiver, args[0]);
            }
            if matches!(
                self.peel_paren_group_expr(&mc.receiver),
                syn::Expr::Reference(_)
            ) {
                return format!("rusty::io::write_fmt({}, {})", receiver, args[0]);
            }
            return format!("rusty::write_fmt({}, {})", receiver, args[0]);
        }
        if method_name == "ok_or" && mc.args.len() == 1 {
            // `ok_or` maps `Option<T>` -> `Result<T, E>`. Its receiver is an
            // `Option<T>`, so thread `Option<Ok>` (from the expected `Result`)
            // down to the receiver — NOT the `Result` itself. This lets a
            // chained `seq.next_element()?.ok_or(())?` recover the
            // `next_element<T>` element type from the assignment/`?` target
            // (serde_bytes ByteArray): the receiver `next_element()?` then sees
            // expected `Option<T>`, its `?` operand `next_element()` sees
            // `Result<Option<T>, _>`, and the turbofish injection fires.
            let receiver_expected: Option<syn::Type> = self
                .expected_result_type_arg(expected_ty, 0)
                .filter(|ok_ty| {
                    !self.type_contains_infer(ok_ty)
                        && !self.type_contains_unresolved_placeholder_like(ok_ty)
                })
                .map(|ok_ty| parse_quote!(Option<#ok_ty>));
            let receiver_expected_arg = receiver_expected.as_ref().or(expected_ty);
            if let Some(expected_err_ty) = self.expected_result_type_arg(expected_ty, 1) {
                let expected_err_cpp = self.map_type(expected_err_ty);
                let expected_err_is_unit_tuple =
                    matches!(expected_err_cpp.trim(), "std::tuple" | "std::tuple<>");
                if !expected_err_is_unit_tuple
                    && expected_err_cpp != "auto"
                    && !expected_err_cpp.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&expected_err_cpp)
                {
                    let coerced_arg =
                        self.emit_from_conversion_to_target(&mc.args[0], &expected_err_cpp);
                    return self.emit_receiver_member_call(
                        &mc.receiver,
                        &method_name,
                        None,
                        &[coerced_arg],
                        receiver_expected_arg,
                    );
                }
            }
            return self.emit_receiver_member_call(
                &mc.receiver,
                &method_name,
                None,
                &args,
                receiver_expected_arg,
            );
        }
        if method_name == "map_err" && mc.args.len() == 1 {
            if let Some(expected_err_ty) = self.expected_result_type_arg(expected_ty, 1) {
                let callable_arg = self
                    .try_emit_map_err_callable_arg(&mc.args[0])
                    .or_else(|| {
                        self.try_emit_error_trait_callable_with_expected_owner(
                            &mc.args[0],
                            expected_err_ty,
                        )
                    })
                    .unwrap_or_else(|| self.emit_expr_to_string(&mc.args[0]));
                let expected_err_cpp = self.map_type(expected_err_ty);
                let expected_err_is_unit_tuple =
                    matches!(expected_err_cpp.trim(), "std::tuple" | "std::tuple<>");
                if !expected_err_is_unit_tuple
                    && expected_err_cpp != "auto"
                    && !expected_err_cpp.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&expected_err_cpp)
                {
                    let typed_callable_arg = format!(
                        "[&](auto&& _err) -> {} {{ return ({}) (std::forward<decltype(_err)>(_err)); }}",
                        expected_err_cpp, callable_arg
                    );
                    return self.emit_receiver_member_call(
                        &mc.receiver,
                        &method_name,
                        None,
                        &[typed_callable_arg],
                        expected_ty,
                    );
                }
            }
            let callable_arg = self
                .try_emit_map_err_callable_arg(&mc.args[0])
                .unwrap_or_else(|| self.emit_expr_to_string(&mc.args[0]));
            return self.emit_receiver_member_call(
                &mc.receiver,
                &method_name,
                None,
                &[callable_arg],
                expected_ty,
            );
        }
        if method_name == "then_some" && args.len() == 1 {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::then_some({}, {})", receiver, args[0]);
        }
        if method_name == "into_deserializer" && args.is_empty() {
            let receiver = self.emit_expr_maybe_move(&mc.receiver);
            let scoped_into_deserializer_fn = self
                .resolve_scoped_namespace_function_expr_path("rusty_ext", "into_deserializer")
                .or_else(|| {
                    self.resolve_unscoped_namespace_function_expr_path(
                        "rusty_ext",
                        "into_deserializer",
                    )
                });
            if let Some(err_cpp) = self.into_deserializer_error_cpp_type(expected_ty) {
                if let Some(ctor_expr) =
                    self.try_emit_builtin_into_deserializer_ctor(&mc.receiver, &receiver, &err_cpp)
                {
                    return ctor_expr;
                }
                // Keep builtin/value trait lowering resilient when scoped `rusty_ext`
                // exists in-module (for example serde_core). Prefer value helpers
                // when their `<E>` overload is valid for the receiver, then scoped
                // helper overloads, then receiver member fallback.
                let value_into_fn =
                    format!("::de::value::rusty_ext::into_deserializer<{}>", err_cpp);
                let forwarded_receiver =
                    "std::forward<decltype(_into_deser_recv)>(_into_deser_recv)";
                if let Some(scoped_fn) = scoped_into_deserializer_fn.as_ref() {
                    return format!(
                        "([&](auto&& _into_deser_recv) -> decltype(auto) {{ if constexpr (requires {{ {}({}); }}) {{ return {}({}); }} else if constexpr (requires {{ {}({}); }}) {{ return {}({}); }} else {{ return {}.into_deserializer(); }} }})({})",
                        value_into_fn,
                        forwarded_receiver,
                        value_into_fn,
                        forwarded_receiver,
                        scoped_fn,
                        forwarded_receiver,
                        scoped_fn,
                        forwarded_receiver,
                        forwarded_receiver,
                        receiver
                    );
                }
                return format!(
                    "([&](auto&& _into_deser_recv) -> decltype(auto) {{ if constexpr (requires {{ {}({}); }}) {{ return {}({}); }} else {{ return {}.into_deserializer(); }} }})({})",
                    value_into_fn,
                    forwarded_receiver,
                    value_into_fn,
                    forwarded_receiver,
                    forwarded_receiver,
                    receiver
                );
            }
            if let Some(qualified_fn) = scoped_into_deserializer_fn {
                return format!("{}({})", qualified_fn, receiver);
            }
            // Serde value helpers are emitted under `de::value::rusty_ext`.
            return format!("::de::value::rusty_ext::into_deserializer({})", receiver);
        }
        if method_name == "parse" && args.is_empty() {
            if std::env::var("RUSTY_DEBUG_PARSE_EXPECTED").is_ok() {
                let expected_dbg = expected_ty
                    .map(|ty| quote::quote!(#ty).to_string())
                    .unwrap_or_else(|| "<none>".to_string());
                let return_hint_dbg = self
                    .current_return_type_hint()
                    .map(|ty| quote::quote!(#ty).to_string())
                    .unwrap_or_else(|| "<none>".to_string());
                let receiver_ty_dbg = self
                    .infer_simple_expr_type(&mc.receiver)
                    .map(|ty| quote::quote!(#ty).to_string())
                    .unwrap_or_else(|| "<none>".to_string());
                eprintln!(
                    "DBG parse methodcall={} expected={} return_hint={} receiver_ty={}",
                    quote::quote!(#mc),
                    expected_dbg,
                    return_hint_dbg,
                    receiver_ty_dbg
                );
            }
            let receiver_has_known_parse_member = self
                .infer_simple_expr_type(&mc.receiver)
                .and_then(|ty| {
                    self.lookup_owner_method_return_type_from_receiver_type(&ty, "parse")
                })
                .is_some();
            if receiver_has_known_parse_member {
                // Preserve real receiver member parse methods when known.
                return self.emit_receiver_member_call(
                    &mc.receiver,
                    &method_name,
                    None,
                    &args,
                    expected_ty,
                );
            }
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let receiver_looks_like_as_ref = matches!(
                self.peel_paren_group_expr(&mc.receiver),
                syn::Expr::MethodCall(inner_mc)
                    if inner_mc.method == "as_ref" && inner_mc.args.is_empty()
            );
            let receiver_is_string_like = self
                .infer_simple_expr_type(&mc.receiver)
                .as_ref()
                .is_some_and(|ty| {
                    self.expected_type_is_string_view(Some(ty))
                        || self.map_type(self.peel_reference_paren_group_type(ty))
                            == "rusty::String"
                })
                || receiver_looks_like_as_ref;
            if let Some(ok_ty) = self.expected_result_type_arg(expected_ty, 0) {
                let ok_cpp = self.map_type(ok_ty);
                if ok_cpp != "auto"
                    && !ok_cpp.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&ok_cpp)
                {
                    return format!("rusty::str_runtime::parse<{}>({})", ok_cpp, receiver);
                }
            }
            if receiver_is_string_like && let Some(target_ty) = expected_ty {
                let target_cpp = self.map_type(target_ty);
                if target_cpp != "auto"
                    && !target_cpp.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&target_cpp)
                {
                    return format!("rusty::str_runtime::parse<{}>({})", target_cpp, receiver);
                }
            }
            return self.emit_receiver_member_call(
                &mc.receiver,
                &method_name,
                None,
                &args,
                expected_ty,
            );
        }
        if method_name == "as_ref"
            && args.is_empty()
            && !self.is_expr_raw_pointer_like(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let receiver_is_known_cow = self.expr_is_known_cow_like(&mc.receiver);
            let expected_is_string_view = self.expected_type_is_string_view(expected_ty);
            if receiver_is_known_cow || expected_is_string_view {
                return format!("rusty::to_string_view({})", receiver);
            }
            if !self
                .infer_simple_expr_type(&mc.receiver)
                .as_ref()
                .is_some_and(|ty| self.expected_type_is_string_view(Some(ty)))
            {
                return self.emit_receiver_member_call(
                    &mc.receiver,
                    &method_name,
                    None,
                    &args,
                    expected_ty,
                );
            }
            if let Some(expected_ty) = expected_ty {
                let expected_cpp = self.map_type(expected_ty);
                if expected_cpp != "auto"
                    && !expected_cpp.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&expected_cpp)
                {
                    return format!("rusty::as_ref_into<{}>({})", expected_cpp, receiver);
                }
            }
            return receiver;
        }
        if matches!(method_name.as_str(), "as_ptr" | "as_mut_ptr") && args.is_empty() {
            // Expose the expected pointer's element type to the receiver emission so
            // a return-only-`T` method-template receiver can recover its turbofish.
            let prev_as_ptr_elem = self.as_ptr_expected_element.borrow().clone();
            *self.as_ptr_expected_element.borrow_mut() =
                self.expected_pointer_element_type(expected_ty);
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            *self.as_ptr_expected_element.borrow_mut() = prev_as_ptr_elem;
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            // Rust method resolution can auto-deref `ManuallyDrop<T>` to call
            // `T::as_ptr/as_mut_ptr`; preserve that by dispatching through `*`.
            if self.method_receiver_is_manually_drop_expr(&mc.receiver) {
                return format!("(*{}).{}()", receiver, method_name);
            }
            let helper = if method_name == "as_mut_ptr" {
                "rusty::as_mut_ptr"
            } else {
                "rusty::as_ptr"
            };
            let helper_call = format!("{}({})", helper, receiver);
            if let Some(expected_ptr_cpp) = self.expected_raw_pointer_cpp_type(expected_ty) {
                // Raw-pointer surfaces can carry storage wrappers (`MaybeUninit<T>*`)
                // while call context expects payload pointers (`T*`).
                if method_name == "as_ptr"
                    && let Some(const_target) =
                        Self::pointer_const_cast_target_cpp_type(&expected_ptr_cpp)
                {
                    return format!(
                        "const_cast<{}>(reinterpret_cast<{}>({}))",
                        expected_ptr_cpp, const_target, helper_call
                    );
                }
                return format!("reinterpret_cast<{}>({})", expected_ptr_cpp, helper_call);
            }
            return helper_call;
        }
        if method_name == "chars" && args.is_empty() {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::str_runtime::chars({})", receiver);
        }
        if method_name == "char_indices" && args.is_empty() {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::str_runtime::char_indices({})", receiver);
        }
        if method_name == "len_utf8" && args.is_empty() {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::char_runtime::len_utf8({})", receiver);
        }
        if method_name == "encode_utf8" && args.len() == 1 && self.expr_is_char_like(&mc.receiver) {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!(
                "rusty::char_runtime::encode_utf8({}, {})",
                receiver, args[0]
            );
        }
        if method_name == "is_whitespace"
            && args.is_empty()
            && self.should_lower_char_is_whitespace_method_call(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::char_runtime::is_whitespace({})", receiver);
        }
        if method_name == "is_char_boundary" && args.len() == 1 {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!(
                "rusty::str_runtime::is_char_boundary({}, {})",
                receiver, args[0]
            );
        }
        // Rust integer pow method (`10_u32.pow(exp)`) should lower to the runtime
        // helper because primitive C++ integers do not have a `.pow()` member.
        if method_name == "pow" && args.len() == 1 {
            let receiver_is_numeric_scalar = self
                .infer_simple_expr_type(&mc.receiver)
                .as_ref()
                .is_some_and(|ty| {
                    is_numeric_cpp_scalar_type(
                        self.map_type(self.peel_reference_paren_group_type(ty))
                            .trim(),
                    )
                });
            if receiver_is_numeric_scalar {
                let raw_receiver = self.emit_expr_to_string(&mc.receiver);
                let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                    format!("({})", raw_receiver)
                } else {
                    raw_receiver
                };
                return format!("rusty::pow({}, {})", receiver, args[0]);
            }
        }
        if method_name == "is_ascii"
            && mc.args.is_empty()
            && self.should_lower_char_is_whitespace_method_call(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("(static_cast<uint32_t>({}) <= 0x7F)", receiver);
        }
        // Rust `u8::is_ascii_digit()` → `rusty::is_ascii_digit(b)`.
        // C++ integer scalars do not have `.is_ascii_digit()` members.
        if method_name == "is_ascii_digit" && mc.args.is_empty() {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::is_ascii_digit({})", receiver);
        }
        // Rust `char/u8::is_ascii_hexdigit()` → `rusty::is_ascii_hexdigit(x)`.
        // C++ scalars do not expose `.is_ascii_hexdigit()` members.
        if method_name == "is_ascii_hexdigit" && mc.args.is_empty() {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::is_ascii_hexdigit({})", receiver);
        }
        if method_name == "fill" && args.len() == 1 {
            let receiver_ty = self.infer_simple_expr_type(&mc.receiver);
            let receiver_is_span = receiver_ty
                .as_ref()
                .and_then(|ty| self.span_element_type(ty))
                .is_some();
            if receiver_is_span || self.expr_lowers_to_slice_or_span_view(&mc.receiver) {
                let raw_receiver = self.emit_expr_to_string(&mc.receiver);
                let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                    format!("({})", raw_receiver)
                } else {
                    raw_receiver
                };
                return format!("rusty::fill({}, {})", receiver, args[0]);
            }
        }
        if method_name == "get_mut"
            && args.len() == 1
            && self.should_lower_slice_deref_method_call(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::get_mut({}, {})", receiver, args[0]);
        }
        if method_name == "get"
            && args.len() == 1
            && self.should_lower_slice_deref_method_call(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::get({}, {})", receiver, args[0]);
        }
        if matches!(
            method_name.as_str(),
            "first" | "first_mut" | "last" | "last_mut"
        ) && args.is_empty()
            && self.should_lower_slice_deref_method_call(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::{}({})", method_name, receiver);
        }
        if method_name == "split_first"
            && args.is_empty()
            && self.should_lower_slice_deref_method_call(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::split_first({})", receiver);
        }
        if method_name == "split_first"
            && args.is_empty()
            && !self.receiver_has_inherent_method_named(&mc.receiver, "split_first")
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::split_first({})", receiver);
        }
        if method_name == "chunks_exact"
            && args.len() == 1
            && !self.receiver_has_inherent_method_named(&mc.receiver, "chunks_exact")
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::chunks_exact({}, {})", receiver, args[0]);
        }
        // Rust slice methods `.first()` and `.get(n)` on std::span.
        // In Rust, &[T]::first() -> Option<&T> and &[T]::get(n) -> Option<&T>.
        // In C++, std::span has no such methods, so we emit equivalent expressions.
        // First, check if receiver is a method call to as_bytes (emitted as rusty::as_bytes)
        if let syn::Expr::MethodCall(as_bytes_mc) = mc.receiver.as_ref() {
            if as_bytes_mc.method == "as_bytes" && as_bytes_mc.args.is_empty() {
                let raw_receiver = self.emit_expr_to_string(&mc.receiver);
                let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                    format!("({})", raw_receiver)
                } else {
                    raw_receiver
                };
                let opt_type = "rusty::Option<const uint8_t&>";
                if method_name == "first" && args.is_empty() {
                    return format!(
                        "(!{}.empty() ? {}({}[0]) : {}(rusty::None))",
                        receiver, opt_type, receiver, opt_type
                    );
                }
                if method_name == "get" && args.len() == 1 {
                    let idx = &args[0];
                    return format!(
                        "(({} < {}.size()) ? {}({}[{}]) : {}(rusty::None))",
                        idx, receiver, opt_type, receiver, idx, opt_type
                    );
                }
            }
        }
        // Infer receiver type to detect span types from local bindings.
        let receiver_ty = self.infer_simple_expr_type(&mc.receiver);
        if method_name == "unwrap_or_default" && args.is_empty() {
            let fallback_ty = receiver_ty
                .as_ref()
                .and_then(|ty| self.expected_option_type_arg(Some(ty)).cloned())
                .or_else(|| {
                    receiver_ty
                        .as_ref()
                        .and_then(|ty| self.expected_result_type_arg(Some(ty), 0).cloned())
                })
                .or_else(|| expected_ty.cloned())
                .filter(|ty| {
                    !self.type_contains_infer(ty)
                        && !self.type_contains_unresolved_placeholder_like(ty)
                        && !self.type_contains_in_scope_type_param(ty)
                        && !self.type_contains_unbound_single_letter_generic(ty)
                });
            if let Some(fallback_ty) = fallback_ty {
                let fallback_cpp = self.map_type(&fallback_ty);
                if fallback_cpp != "auto"
                    && !fallback_cpp.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&fallback_cpp)
                {
                    let fallback_arg = format!("rusty::default_value<{}>()", fallback_cpp);
                    return self.emit_receiver_member_call(
                        &mc.receiver,
                        "unwrap_or",
                        None,
                        &[fallback_arg],
                        None,
                    );
                }
            }
        }
        if let Some((is_const, elem_cpp)) = receiver_ty
            .as_ref()
            .and_then(|ty| self.span_element_type(ty))
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            // Construct the Option type: Option<T&> or Option<const T&>
            let opt_type = if is_const {
                format!("rusty::Option<const {}&>", elem_cpp)
            } else {
                format!("rusty::Option<{}&>", elem_cpp)
            };
            // .first() with no args: bounds check + element access
            if method_name == "first" && args.is_empty() {
                return format!(
                    "(!{}.empty() ? {}({}[0]) : {}(rusty::None))",
                    receiver, opt_type, receiver, opt_type
                );
            }
            // .get(n) with one arg: bounds check + element access
            if method_name == "get" && args.len() == 1 {
                let idx = &args[0];
                return format!(
                    "(({} < {}.size()) ? {}({}[{}]) : {}(rusty::None))",
                    idx, receiver, opt_type, receiver, idx, opt_type
                );
            }
            if method_name == "split_first" && args.is_empty() {
                return format!("rusty::split_first({})", receiver);
            }
        }
        // Rust Vec/ArrayVec/SmallVec-style `.get(index)` fallback surface.
        // Shared lowering path: preserve member calls on unrelated `get(...)` APIs.
        if method_name == "get"
            && args.len() == 1
            && !self.is_slice_range_index_expr(&mc.args[0])
            && (self.should_lower_index_method_call_to_index_op(&mc.receiver)
                || self.should_lower_unknown_local_index_method_call(&mc.receiver, &mc.args[0]))
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::get({}, {})", receiver, args[0]);
        }
        if method_name == "get_mut"
            && args.len() == 1
            && !self.is_slice_range_index_expr(&mc.args[0])
            && (self.should_lower_index_method_call_to_index_op(&mc.receiver)
                || self.should_lower_unknown_local_index_method_call(&mc.receiver, &mc.args[0]))
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::get_mut({}, {})", receiver, args[0]);
        }
        if method_name == "get"
            && args.len() == 1
            && !self.is_slice_range_index_expr(&mc.args[0])
            && self.expr_is_tuple_field_access(&mc.receiver)
            && self.index_trait_arg_supports_bracket_access(&mc.args[0])
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::get({}, {})", receiver, args[0]);
        }
        if matches!(method_name.as_str(), "get_unchecked" | "get_unchecked_mut")
            && args.len() == 1
            && let Some(range_expr) = self.try_emit_get_unchecked_range_method_call(mc, expected_ty)
        {
            return range_expr;
        }
        if matches!(method_name.as_str(), "get_unchecked" | "get_unchecked_mut")
            && args.len() == 1
            && !self.is_slice_range_index_expr(&mc.args[0])
            && (self.should_lower_index_method_call_to_index_op(&mc.receiver)
                || self.should_lower_unknown_local_index_method_call(&mc.receiver, &mc.args[0]))
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("{}[{}]", receiver, args[0]);
        }
        if matches!(method_name.as_str(), "get_unchecked" | "get_unchecked_mut")
            && args.len() == 1
            && !self.is_slice_range_index_expr(&mc.args[0])
            && self.expr_is_tuple_field_access(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("{}[{}]", receiver, args[0]);
        }
        // Last-resort fallback for `recv.get_unchecked[_mut](idx)` where the
        // receiver type couldn't be statically determined (auto&&-bound
        // chains like `(*ptr).edges` or `leaf.keys` where `ptr`/`leaf`
        // come from method calls whose return type we couldn't trace).
        // Emit a `requires { recv[idx] }` SFINAE wrapper so std::array
        // / std::vector / std::span receivers route to operator[] while
        // bona-fide get_unchecked-having types (Rusty slice helpers) still
        // call the named method.
        if matches!(method_name.as_str(), "get_unchecked" | "get_unchecked_mut")
            && args.len() == 1
            && !self.is_slice_range_index_expr(&mc.args[0])
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!(
                "([&](auto&& __recv, auto&& __idx) -> decltype(auto) {{ \
                if constexpr (requires {{ __recv[__idx]; }}) {{ \
                return __recv[__idx]; \
                }} else {{ \
                return __recv.{}(__idx); \
                }} }})({}, {})",
                method_name, receiver, args[0]
            );
        }
        // std::string_view and Rust string-like surfaces expose `.get(..)` in
        // Rust through index/range helpers, not as C++ member methods.
        if method_name == "get"
            && args.len() == 1
            && receiver_ty.as_ref().is_some_and(|ty| {
                self.is_known_string_like_type(ty) || self.map_type(ty) == "std::string_view"
            })
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::get({}, {})", receiver, args[0]);
        }
        // Note: `.as_bytes()` on slices is NOT rewritten generically —
        // it would require full Rust slice API on the returned span.
        // Rust `is_empty()` → dispatch to `.is_empty()` or `.empty()` depending on type
        if method_name == "is_empty" && args.is_empty() {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::is_empty({})", receiver);
        }
        if method_name == "escape_debug" && args.is_empty() {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            if self.expr_is_char_like(&mc.receiver) {
                return format!(
                    "rusty::detail::escape_debug_string(rusty::detail::utf8_from_char32(static_cast<char32_t>({})))",
                    receiver
                );
            }
            return format!(
                "rusty::detail::escape_debug_string(std::string({}))",
                receiver
            );
        }
        // Rust string methods that don't exist on std::string_view
        if method_name == "trim" && args.is_empty() {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::str_runtime::trim({})", receiver);
        }
        if matches!(
            method_name.as_str(),
            "trim_start_matches" | "trim_end_matches"
        ) && args.len() == 1
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!(
                "rusty::str_runtime::{}({}, {})",
                method_name, receiver, args[0]
            );
        }
        if method_name == "strip_prefix" && args.len() == 1 {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!(
                "rusty::str_runtime::strip_prefix({}, {})",
                receiver, args[0]
            );
        }
        if method_name == "replace" && args.len() == 2 {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!(
                "rusty::str_runtime::replace({}, {}, {})",
                receiver, args[0], args[1]
            );
        }
        if method_name == "find"
            && args.len() == 1
            && !matches!(
                self.peel_paren_group_expr(&mc.args[0]),
                syn::Expr::Closure(_)
            )
            && !self.is_iterator_like_receiver_expr(&mc.receiver)
            && !self.is_probably_iterator_receiver_expr(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::str_runtime::find({}, {})", receiver, args[0]);
        }
        if method_name == "split_at" && args.len() == 1 {
            if self
                .infer_simple_expr_type(&mc.receiver)
                .as_ref()
                .is_some_and(|ty| self.is_known_string_like_type(ty))
            {
                let raw_receiver = self.emit_expr_to_string(&mc.receiver);
                let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                    format!("({})", raw_receiver)
                } else {
                    raw_receiver
                };
                return format!("rusty::split_at({}, {})", receiver, args[0]);
            }
            if self.should_lower_slice_deref_method_call(&mc.receiver) {
                let raw_receiver = self.emit_expr_to_string(&mc.receiver);
                let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                    format!("({})", raw_receiver)
                } else {
                    raw_receiver
                };
                let split_receiver = if self.expr_lowers_to_slice_or_span_view(&mc.receiver) {
                    receiver
                } else {
                    format!("rusty::as_slice({})", receiver)
                };
                return format!("rusty::split_at({}, {})", split_receiver, args[0]);
            }
        }
        if method_name == "split" && args.len() == 1 {
            // `.split(arg)` is ambiguous in lowering — string-split
            // (`rusty::str_runtime::split`) vs. a user method (e.g.
            // BTreeMap's `Handle::split`, which takes an allocator).
            // Resolve by receiver type: only redirect to string-split
            // when the receiver is a known string-like type. Otherwise,
            // fall through to the default method-call lowering so the
            // user's method is invoked.
            let receiver_ty = self
                .infer_hint_type_from_expr(&mc.receiver)
                .or_else(|| self.infer_simple_expr_type(&mc.receiver));
            let mapped_receiver = receiver_ty.as_ref().map(|ty| self.map_type(ty));
            let receiver_is_stringish = mapped_receiver
                .as_ref()
                .map(|mapped| {
                    mapped.contains("string_view")
                        || mapped.contains("rusty::String")
                        || mapped.contains("StrView")
                        || *mapped == "std::string"
                })
                .unwrap_or(false);
            // When type inference can't resolve the receiver (`rhs.as_str()`
            // on a struct type whose `as_str` impl isn't in the visible
            // impl_blocks set), fall back to a syntactic check: a tail
            // `.as_str()` method call almost always yields `&str` in Rust
            // (string-port, semver Version, …). Without this, semver emits
            // `rhs.as_str().split(U'.')` as a method call and clang errors
            // "no member named 'split' in 'std::basic_string_view<char>'".
            let receiver_chain_ends_in_as_str = matches!(
                self.peel_paren_group_expr(&mc.receiver),
                syn::Expr::MethodCall(inner) if inner.method == "as_str" && inner.args.is_empty()
            );
            if receiver_is_stringish || receiver_chain_ends_in_as_str {
                let raw_receiver = self.emit_expr_to_string(&mc.receiver);
                let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                    format!("({})", raw_receiver)
                } else {
                    raw_receiver
                };
                return format!("rusty::str_runtime::split({}, {})", receiver, args[0]);
            }
        }
        if method_name == "hash"
            && args.len() == 1
            // A type with its OWN inherent `hash` method (indexmap's
            // `IndexMap::hash(&self, key) -> HashValue`) keeps the member
            // call — only Hash-TRAIT protocol calls route to the
            // void-returning rusty::hash::hash(value, state).
            && !self.receiver_has_inherent_method_named(&mc.receiver, "hash")
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            // Strip `&` from the state argument — Rust passes `&mut state`
            // but rusty::hash::hash takes `State&` by reference, not pointer.
            let state_arg = match mc.args.first() {
                Some(syn::Expr::Reference(r)) => self.emit_expr_to_string(&r.expr),
                _ => args[0].clone(),
            };
            return format!("rusty::hash::hash({}, {})", receiver, state_arg);
        }
        if method_name == "borrow"
            && args.is_empty()
            // Borrow-TRAIT protocol: the blanket `Borrow<T> for T` is an
            // identity borrow, which primitives can't spell as a member
            // (`key.borrow()` with K=int in equivalent's blanket impl). The
            // helper member-prefers, so String's Borrow<str> port keeps its
            // member dispatch.
            && !self.receiver_has_inherent_method_named(&mc.receiver, "borrow")
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::borrow({})", receiver);
        }
        if method_name == "to_bits" && args.is_empty() {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::float_to_bits({})", receiver);
        }
        // Rust `.clone()` → `rusty::clone(receiver)`.
        // Expanded `#[derive(Clone)]` calls `.clone()` on every field, but C++
        // primitives (`uint64_t`, `bool`, etc.) and enum classes don't have a
        // `.clone()` member.  `rusty::clone()` dispatches via SFINAE: calls
        // `.clone()` if available, falls back to copy construction otherwise.
        if method_name == "clone" && mc.args.is_empty() {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::clone({})", receiver);
        }
        if method_name == "to_mut"
            && args.is_empty()
            && self
                .infer_simple_expr_type(&mc.receiver)
                .as_ref()
                .is_some_and(|ty| {
                    self.canonical_into_target_cpp_type(&self.map_type(ty)) == "rusty::Cow"
                })
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::to_mut({})", receiver);
        }
        // Rust `.cmp(&other)` on primitives → `rusty::cmp::cmp(a, b)`.
        // Expanded `#[derive(Ord)]` calls `.cmp()` on every field.
        if method_name == "cmp" && mc.args.len() == 1 {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let rhs = match self.peel_paren_group_expr(&mc.args[0]) {
                syn::Expr::Reference(r) if !self.is_expr_raw_pointer_like(&r.expr) => {
                    self.emit_expr_to_string(&r.expr)
                }
                _ => self.emit_expr_to_string(&mc.args[0]),
            };
            return format!("rusty::cmp::cmp({}, {})", receiver, rhs);
        }
        // Rust `.partial_cmp(&other)` on primitives → `rusty::partial_cmp(a, b)`.
        // Expanded `#[derive(PartialOrd)]` calls `.partial_cmp()` on every field.
        if method_name == "partial_cmp" && mc.args.len() == 1 {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let rhs = match self.peel_paren_group_expr(&mc.args[0]) {
                syn::Expr::Reference(r) if !self.is_expr_raw_pointer_like(&r.expr) => {
                    self.emit_expr_to_string(&r.expr)
                }
                _ => self.emit_expr_to_string(&mc.args[0]),
            };
            return format!("rusty::partial_cmp({}, {})", receiver, rhs);
        }
        if method_name == "zip" && mc.args.len() == 1 {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let rhs = match self.peel_paren_group_expr(&mc.args[0]) {
                syn::Expr::Reference(r) if !self.is_expr_raw_pointer_like(&r.expr) => {
                    self.emit_expr_to_string(&r.expr)
                }
                _ => self.emit_expr_maybe_move(&mc.args[0]),
            };
            return format!("rusty::zip({}, {})", receiver, rhs);
        }
        if method_name == "write" && args.len() == 1 {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver_is_raw_pointer = self.is_expr_raw_pointer_like(&mc.receiver)
                || Self::emitted_pointer_add_or_offset_call(&raw_receiver);
            if receiver_is_raw_pointer {
                let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                    format!("({})", raw_receiver)
                } else {
                    raw_receiver
                };
                let mut value_arg = args[0].clone();
                if !value_arg.starts_with("std::move(") {
                    value_arg = format!("std::move({})", value_arg);
                }
                return format!("rusty::ptr::write({}, {})", receiver, value_arg);
            }
        }
        if method_name == "read" && args.is_empty() {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver_is_raw_pointer = self.is_expr_raw_pointer_like(&mc.receiver)
                || Self::emitted_pointer_add_or_offset_call(&raw_receiver);
            if receiver_is_raw_pointer {
                let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                    format!("({})", raw_receiver)
                } else {
                    raw_receiver
                };
                return format!("rusty::ptr::read({})", receiver);
            }
        }
        if method_name == "write_unaligned" && args.len() == 1 {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver_is_raw_pointer = self.is_expr_raw_pointer_like(&mc.receiver)
                || Self::emitted_pointer_add_or_offset_call(&raw_receiver);
            if receiver_is_raw_pointer {
                let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                    format!("({})", raw_receiver)
                } else {
                    raw_receiver
                };
                return format!("rusty::ptr::write_unaligned({}, {})", receiver, args[0]);
            }
        }
        if method_name == "read_unaligned" && args.is_empty() {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver_is_raw_pointer = self.is_expr_raw_pointer_like(&mc.receiver)
                || Self::emitted_pointer_add_or_offset_call(&raw_receiver);
            if receiver_is_raw_pointer {
                let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                    format!("({})", raw_receiver)
                } else {
                    raw_receiver
                };
                return format!("rusty::ptr::read_unaligned({})", receiver);
            }
        }
        // Rust `ptr.is_null()` → C++ `ptr == nullptr`
        if method_name == "is_null" && args.is_empty() {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("({} == nullptr)", receiver);
        }
        // `<*const T>::cast_mut()` / `<*mut T>::cast_const()` as method calls on a
        // raw pointer — C++ raw pointers carry no const in the value category here,
        // so route through the same `rusty::ptr::cast_{mut,const}` helpers the path
        // form uses. (Path/UFCS form is handled separately near line 13504.)
        if matches!(method_name.as_str(), "cast_mut" | "cast_const")
            && args.is_empty()
            && self.is_expr_raw_pointer_like(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::ptr::{}({})", method_name, receiver);
        }
        if method_name == "cast" && args.is_empty() {
            // Rust's zero-arg `.cast()` exists ONLY on raw pointers and
            // NonNull, and receiver inference can disagree with the actual
            // lowering (the Deref-routed `owned.ptr` infers Owned's private
            // NonNull field while the emission yields InitPtr's raw
            // pointer). Route ALL of them here: the reinterpret path when
            // the target is known and the receiver is positively raw, the
            // `rusty::ptr::cast` proxy otherwise (overloaded for raw
            // pointers AND NonNull; converts to U*, NonNull<U>, and chains
            // .as_non_null_ptr()).
            // Target type: explicit turbofish `ptr.cast::<U>()` first; otherwise
            // try the expected_ty (Rust `let x: *mut U = ptr.cast()` and similar
            // arg-position calls). When neither tells us U, the cast can't be
            // emitted as `reinterpret_cast` — fall through to the bare-method
            // emission, which will fail to compile but at least surface a
            // clear error from the C++ compiler.
            let target_ty: Option<syn::Type> = self
                .method_call_single_turbofish_type(mc)
                .map(|ty| ty.clone())
                .or_else(|| {
                    let ty = expected_ty?;
                    let peeled = self.peel_reference_paren_group_type(ty);
                    if let syn::Type::Ptr(ptr) = peeled {
                        Some((*ptr.elem).clone())
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    // c2rust Deref idiom: `&*addr_of!(*self).cast()` inside a
                    // `-> &Target` fn. The `&*` wrapper doesn't propagate a
                    // pointer expected type to the `.cast()`, so derive the cast
                    // target pointee from the enclosing fn's reference return
                    // type. Gated to an addr-of receiver so a bare `ptr.cast()`
                    // in a reference-returning fn isn't mis-targeted.
                    if !matches!(
                        self.peel_paren_group_expr(&mc.receiver),
                        syn::Expr::RawAddr(_)
                    ) {
                        return None;
                    }
                    let ret = self.current_return_type_hint()?;
                    if !matches!(ret, syn::Type::Reference(_)) {
                        return None;
                    }
                    Some(self.peel_reference_paren_group_type(ret).clone())
                });
            if let Some(target_ty) = target_ty {
                let is_mut = self
                    .infer_raw_pointer_mutability_for_expr(&mc.receiver)
                    .or_else(|| {
                        expected_ty.and_then(|ty| {
                            let peeled = self.peel_reference_paren_group_type(ty);
                            let syn::Type::Ptr(ptr) = peeled else {
                                return None;
                            };
                            Some(ptr.mutability.is_some())
                        })
                    })
                    .or_else(|| {
                        // `addr_of!(x)` → `*const`, `addr_of_mut!(x)` → `*mut`.
                        // Const-ness of the source pointer carries through `.cast()`
                        // so the deref-idiom cast doesn't strip qualifiers.
                        match self.peel_paren_group_expr(&mc.receiver) {
                            syn::Expr::RawAddr(raw) => {
                                Some(matches!(raw.mutability, syn::PointerMutability::Mut(_)))
                            }
                            _ => None,
                        }
                    })
                    .unwrap_or(true);
                let cast_ty: syn::Type = if is_mut {
                    parse_quote!(*mut #target_ty)
                } else {
                    parse_quote!(*const #target_ty)
                };
                let cast_cpp = self
                    .rewrite_extension_integer_assoc_projection_fallbacks(&self.map_type(&cast_ty));
                let raw_receiver = self.emit_expr_to_string(&mc.receiver);
                let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                    format!("({})", raw_receiver)
                } else {
                    raw_receiver
                };
                return format!("reinterpret_cast<{}>({})", cast_cpp, receiver);
            }
            // Target pointee undeterminable (no turbofish, and the result flows
            // into a callee whose signature we don't model — e.g. a C SIMD
            // intrinsic). Emit a cast proxy that adapts to whatever pointer type
            // the surrounding context requires, rather than the bare
            // `ptr->cast()` member call (raw pointers have no `cast` member).
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::ptr::cast({})", receiver);
        }
        // `<*const T>::align_offset(align)` — raw-pointer alignment helper.
        if method_name == "align_offset"
            && mc.args.len() == 1
            && self.is_expr_raw_pointer_like(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let align_arg = self.emit_expr_to_string(&mc.args[0]);
            return format!("rusty::ptr::align_offset({}, {})", receiver, align_arg);
        }
        if matches!(
            method_name.as_str(),
            "wrapping_add" | "wrapping_sub" | "wrapping_offset"
        ) && args.len() == 1
            && self.is_expr_raw_pointer_like(&mc.receiver)
        {
            let receiver_expected = expected_ty
                .map(|ty| self.peel_reference_paren_group_type(ty))
                .filter(|ty| matches!(ty, syn::Type::Ptr(_)));
            let raw_receiver = if let Some(receiver_expected) = receiver_expected {
                self.emit_expr_to_string_with_expected(&mc.receiver, Some(receiver_expected))
            } else {
                self.emit_expr_to_string(&mc.receiver)
            };
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let op = match method_name.as_str() {
                "wrapping_sub" => "sub",
                "wrapping_add" => "add",
                // Rust raw-pointer wrapping_offset uses signed offsets and wraps
                // address space. Route through ptr::offset helper for pointer surfaces.
                "wrapping_offset" => "offset",
                _ => unreachable!(),
            };
            let mut offset_arg = args[0].clone();
            let arg0_binding = mc.args.first().and_then(|arg0| {
                let arg0 = self.peel_paren_group_expr(arg0);
                if let syn::Expr::Path(path) = arg0
                    && path.path.segments.len() == 1
                {
                    return Some((path.path.segments[0].ident.to_string(), false));
                }
                if let syn::Expr::Unary(unary) = arg0
                    && matches!(unary.op, syn::UnOp::Deref(_))
                    && let syn::Expr::Path(path) = self.peel_paren_group_expr(&unary.expr)
                    && path.path.segments.len() == 1
                {
                    return Some((path.path.segments[0].ident.to_string(), true));
                }
                None
            });
            if let Some((name, explicit_deref_arg)) = arg0_binding {
                if std::env::var_os("RUSTY_CPP_DEBUG_REBIND_POINTER").is_some() {
                    let ty_dbg = self
                        .lookup_local_binding_type(&name)
                        .map(|ty| ty.to_token_stream().to_string())
                        .unwrap_or_else(|| "<none>".to_string());
                    eprintln!(
                        "[rebind-pointer] wrapping_ptr_add name={} lowered={} explicit_deref={} reassigned={} const_in_scope={} local_cpp={:?} ty={}",
                        name,
                        self.is_reference_binding_lowered_to_pointer_storage(&name),
                        explicit_deref_arg,
                        self.reassigned_vars.contains(&name),
                        self.is_const_local_binding_in_scope(&name),
                        self.lookup_local_binding_cpp_name(&name),
                        ty_dbg
                    );
                }
                let lowered = self.is_reference_binding_lowered_to_pointer_storage(&name)
                    || (explicit_deref_arg && self.reassigned_vars.contains(&name));
                if lowered && !offset_arg.trim_start().starts_with('*') {
                    offset_arg = format!("*({})", offset_arg);
                }
            }
            let emitted_call = format!("rusty::ptr::{}({}, {})", op, receiver, offset_arg);
            if std::env::var_os("RUSTY_CPP_DEBUG_REBIND_POINTER").is_some() {
                eprintln!(
                    "[rebind-pointer] emit_method_ptr_add op={} recv={} arg={}",
                    op, receiver, offset_arg
                );
            }
            return emitted_call;
        }
        if matches!(method_name.as_str(), "copy_to_nonoverlapping" | "copy_to")
            && args.len() == 2
            && self.is_expr_raw_pointer_like(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let helper = if method_name == "copy_to_nonoverlapping" {
                "rusty::ptr::copy_nonoverlapping"
            } else {
                "rusty::ptr::copy"
            };
            return format!("{}({}, {}, {})", helper, receiver, args[0], args[1]);
        }
        if matches!(
            method_name.as_str(),
            "copy_from_nonoverlapping" | "copy_from"
        ) && args.len() == 2
            && self.is_expr_raw_pointer_like(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let helper = if method_name == "copy_from_nonoverlapping" {
                "rusty::ptr::copy_nonoverlapping"
            } else {
                "rusty::ptr::copy"
            };
            return format!("{}({}, {}, {})", helper, args[0], receiver, args[1]);
        }
        // `<ptr>.write_bytes(val, count)` (a raw-pointer intrinsic, like
        // `copy_to_nonoverlapping` above) → free function
        // `rusty::ptr::write_bytes(ptr, val, count)`. The rusty header exposes
        // it only as a free function (include/rusty/ptr.hpp), so the method
        // form must be lowered. Guarded on a pointer-like receiver so we don't
        // intercept an unrelated user method named `write_bytes`.
        if method_name == "write_bytes"
            && args.len() == 2
            && self.is_expr_raw_pointer_like(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!(
                "rusty::ptr::write_bytes({}, {}, {})",
                receiver, args[0], args[1]
            );
        }
        // `<*mut T>::drop_in_place()` runs T's destructor through a raw pointer.
        // As a METHOD on a raw-pointer receiver it has no C++ member equivalent
        // (`ptr->drop_in_place()` looks for a member of the pointee) — lower it to
        // the free function `rusty::ptr::drop_in_place(ptr)`, matching the
        // path-call form `ptr::drop_in_place(ptr)`.
        if method_name == "drop_in_place"
            && args.is_empty()
            && self.is_expr_raw_pointer_like(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::ptr::drop_in_place({})", receiver);
        }
        if matches!(method_name.as_str(), "add" | "offset" | "sub")
            && args.len() == 1
            && self.is_expr_raw_pointer_like(&mc.receiver)
        {
            let receiver_expected = expected_ty
                .map(|ty| self.peel_reference_paren_group_type(ty))
                .filter(|ty| matches!(ty, syn::Type::Ptr(_)));
            let raw_receiver = if let Some(receiver_expected) = receiver_expected {
                self.emit_expr_to_string_with_expected(&mc.receiver, Some(receiver_expected))
            } else {
                self.emit_expr_to_string(&mc.receiver)
            };
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let mut offset_arg = args[0].clone();
            let arg0_binding = mc.args.first().and_then(|arg0| {
                let arg0 = self.peel_paren_group_expr(arg0);
                if let syn::Expr::Path(path) = arg0
                    && path.path.segments.len() == 1
                {
                    return Some((path.path.segments[0].ident.to_string(), false));
                }
                if let syn::Expr::Unary(unary) = arg0
                    && matches!(unary.op, syn::UnOp::Deref(_))
                    && let syn::Expr::Path(path) = self.peel_paren_group_expr(&unary.expr)
                    && path.path.segments.len() == 1
                {
                    return Some((path.path.segments[0].ident.to_string(), true));
                }
                None
            });
            if let Some((name, explicit_deref_arg)) = arg0_binding {
                if std::env::var_os("RUSTY_CPP_DEBUG_REBIND_POINTER").is_some() {
                    let ty_dbg = self
                        .lookup_local_binding_type(&name)
                        .map(|ty| ty.to_token_stream().to_string())
                        .unwrap_or_else(|| "<none>".to_string());
                    eprintln!(
                        "[rebind-pointer] ptr_add name={} lowered={} explicit_deref={} reassigned={} const_in_scope={} local_cpp={:?} ty={}",
                        name,
                        self.is_reference_binding_lowered_to_pointer_storage(&name),
                        explicit_deref_arg,
                        self.reassigned_vars.contains(&name),
                        self.is_const_local_binding_in_scope(&name),
                        self.lookup_local_binding_cpp_name(&name),
                        ty_dbg
                    );
                }
                let lowered = self.is_reference_binding_lowered_to_pointer_storage(&name)
                    || (explicit_deref_arg && self.reassigned_vars.contains(&name));
                if lowered && !offset_arg.trim_start().starts_with('*') {
                    offset_arg = format!("*({})", offset_arg);
                }
            }
            let emitted_call = format!("rusty::ptr::{}({}, {})", method_name, receiver, offset_arg);
            if std::env::var_os("RUSTY_CPP_DEBUG_REBIND_POINTER").is_some() {
                eprintln!(
                    "[rebind-pointer] emit_method_ptr_add op={} recv={} arg={}",
                    method_name, receiver, offset_arg
                );
            }
            return emitted_call;
        }
        if matches!(
            method_name.as_str(),
            "saturating_add" | "saturating_sub" | "saturating_mul"
        ) && args.len() == 1
            && self.should_lower_saturating_method_call(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let rhs = format!("rusty::detail::deref_if_pointer({})", args[0]);
            return format!("rusty::{}({}, {})", method_name, receiver, rhs);
        }
        if matches!(method_name.as_str(), "rotate_right" | "rotate_left")
            && args.len() == 1
            && self.should_lower_swap_method_call_to_index_swap(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let helper = if method_name == "rotate_right" {
                "rusty::rotate_right"
            } else {
                "rusty::rotate_left"
            };
            return format!("{}({}, {})", helper, receiver, args[0]);
        }
        // Rust integer intrinsic methods → C++ equivalents
        if matches!(method_name.as_str(), "rotate_right" | "rotate_left")
            && args.len() == 1
            && !self.is_expr_raw_pointer_like(&mc.receiver)
            && self.should_lower_integer_rotate_method_call(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let cpp_fn = if method_name == "rotate_right" {
                "std::rotr"
            } else {
                "std::rotl"
            };
            return format!("{}(static_cast<size_t>({}), {})", cpp_fn, receiver, args[0]);
        }
        if matches!(
            method_name.as_str(),
            "leading_zeros"
                | "trailing_zeros"
                | "count_ones"
                | "count_zeros"
                | "swap_bytes"
                | "is_power_of_two"
        ) && args.is_empty()
            && !self.is_expr_raw_pointer_like(&mc.receiver)
            && self.should_lower_integer_intrinsic_method_call(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::{}({})", method_name, receiver);
        }
        // `.abs()` — primitive in Rust std; the rusty::abs helper
        // member-prefers, so unknown-typed receivers (closure params) route
        // safely too.
        if method_name == "abs"
            && args.is_empty()
            && !self.is_expr_raw_pointer_like(&mc.receiver)
            && (self.should_lower_integer_intrinsic_method_call(&mc.receiver)
                || self.receiver_type_unresolved_for_iter_default_routing(&mc.receiver))
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::abs({})", receiver);
        }
        // One-arg integer intrinsics (`usize::div_ceil`): same routing shape.
        if method_name == "div_ceil"
            && args.len() == 1
            && !self.is_expr_raw_pointer_like(&mc.receiver)
            && self.should_lower_integer_intrinsic_method_call(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::div_ceil({}, {})", receiver, args[0]);
        }
        if matches!(
            method_name.as_str(),
            "to_le" | "to_be" | "from_le" | "from_be"
        ) && args.is_empty()
            && !self.is_expr_raw_pointer_like(&mc.receiver)
            && self.should_lower_integer_intrinsic_method_call(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::{}({})", method_name, receiver);
        }
        if matches!(method_name.as_str(), "wrapping_neg" | "wrapping_abs")
            && args.is_empty()
            && !self.is_expr_raw_pointer_like(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::{}({})", method_name, receiver);
        }
        if matches!(
            method_name.as_str(),
            "wrapping_add" | "wrapping_sub" | "wrapping_mul" | "wrapping_div" | "wrapping_rem"
        ) && args.len() == 1
            && !self.is_expr_raw_pointer_like(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let op = match method_name.as_str() {
                "wrapping_add" => "+",
                "wrapping_sub" => "-",
                "wrapping_mul" => "*",
                // Unsigned division/remainder never wrap (no overflow except /0,
                // which traps in Rust too), so the plain operators are exact.
                "wrapping_div" => "/",
                "wrapping_rem" => "%",
                _ => unreachable!(),
            };
            // C++ unsigned arithmetic wraps naturally; cast to size_t to ensure unsigned
            return format!(
                "(static_cast<size_t>({}) {} static_cast<size_t>({}))",
                receiver, op, args[0]
            );
        }
        if matches!(method_name.as_str(), "wrapping_shr" | "wrapping_shl")
            && args.len() == 1
            && !self.is_expr_raw_pointer_like(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            let op = if method_name == "wrapping_shr" {
                ">>"
            } else {
                "<<"
            };
            return format!(
                "([&]() {{ using _rusty_shift_lhs_t = std::remove_cvref_t<decltype(({}))>; constexpr uint32_t _rusty_shift_bits = static_cast<uint32_t>(sizeof(_rusty_shift_lhs_t) * 8); auto _rusty_shift_amount = static_cast<uint32_t>({}) % (_rusty_shift_bits == 0 ? 1 : _rusty_shift_bits); return static_cast<_rusty_shift_lhs_t>(static_cast<std::make_unsigned_t<_rusty_shift_lhs_t>>({}) {} _rusty_shift_amount); }}())",
                receiver, args[0], receiver, op
            );
        }
        // Rust checked arithmetic methods → rusty:: free-function helpers returning Option<T>
        if matches!(
            method_name.as_str(),
            "checked_add" | "checked_sub" | "checked_mul" | "checked_div"
        ) && args.len() == 1
            && !self.is_expr_raw_pointer_like(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            // Explicit return type `Option<T>` — without it, when the call
            // site wraps the checked-arithmetic in another `RUSTY_TRY_OPT(
            // [&]() { … }())` chain, the inner macro's early `return
            // ::rusty::None;` makes C++ deduce the lambda's return type
            // as `None_t`, then the actual `return rusty::checked_X(…)`
            // — which yields `Option<T>` — triggers
            // "return type 'Option<unsigned long>' must match previous
            //  return type 'None_t' when lambda expression has unspecified
            //  explicit return type". Surfaced by itertools' binomial /
            //  mixed-radix counters in `combinations` / `cartesian_product`.
            //
            // The lhs is passed as a forwarding PARAMETER rather than bound in
            // the body, so the explicit return type can spell the element type
            // as `decltype(_checked_lhs)` (a plain id-expression) instead of
            // `decltype((<receiver>))`. When `<receiver>` is itself a
            // statement-expression (e.g. a nested `RUSTY_TRY_OPT(...)`),
            // embedding it inside a `decltype` that is then serialized into a
            // C++23 module BMI crashes clang's lazy AST deserializer
            // (StmtProfiler walks the not-yet-deserialized embedded DeclStmt →
            // SIGSEGV). Referencing the parameter keeps the statement-
            // expression out of every `decltype`. See memory
            // `itertools-clang-crash-rootcause`.
            return format!(
                "[&](auto&& _checked_lhs) -> rusty::Option<std::remove_cvref_t<decltype(_checked_lhs)>> {{ return rusty::{1}(_checked_lhs, static_cast<std::remove_cvref_t<decltype((_checked_lhs))>>({2})); }}({0})",
                receiver, method_name, args[0]
            );
        }
        if method_name == "checked_next_power_of_two"
            && args.is_empty()
            && !self.is_expr_raw_pointer_like(&mc.receiver)
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("rusty::checked_next_power_of_two({})", receiver);
        }
        if method_name == "serialize" && args.len() == 1 {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            return self.emit_serialize_dispatch_call(&receiver, &args[0]);
        }
        if method_name == "parse_str_bytes" && args.len() == 3 {
            let result_ok_ty = expected_ty
                .and_then(|hint| self.expected_result_type_arg(Some(hint), 0))
                .or_else(|| {
                    self.current_return_type_hint()
                        .and_then(|hint| self.expected_result_type_arg(Some(hint), 0))
                });
            if let Some(inner_ty) = result_ok_ty
                .and_then(|ty| self.expected_path_type_arg_by_last_ident(ty, "Reference", 0))
            {
                let inner_cpp = self.map_type(inner_ty);
                if inner_cpp != "auto"
                    && !inner_cpp.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&inner_cpp)
                {
                    let template_args = format!("<{}>", inner_cpp);
                    return self.emit_receiver_member_call(
                        &mc.receiver,
                        &method_name,
                        Some(&template_args),
                        &args,
                        expected_ty,
                    );
                }
            }
        }
        if method_name == "truncate"
            && args.len() == 1
            && receiver_ty.as_ref().is_some_and(|ty| {
                self.type_is_mut_rusty_string_reference(ty)
                    || self.canonical_into_target_cpp_type(&self.map_type(ty)) == "rusty::String"
            })
        {
            let raw_receiver = self.emit_expr_to_string(&mc.receiver);
            let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
                format!("({})", raw_receiver)
            } else {
                raw_receiver
            };
            return format!("{}.truncate({})", receiver, args[0]);
        }
        if let Some(ext_call) = self.try_emit_extension_method_call(mc, &args, expected_ty) {
            return ext_call;
        }
        if let Some(c_like_callee) =
            self.resolve_c_like_enum_inherent_method_free_function(&mc.receiver, &method_name)
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let mut all_args = Vec::with_capacity(args.len() + 1);
            all_args.push(receiver);
            all_args.extend(args.iter().cloned());
            return format!("{}({})", c_like_callee, all_args.join(", "));
        }
        if let Some(owner_enum_cpp) =
            self.receiver_data_enum_owner_with_inherent_method(&mc.receiver, &method_name)
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let wrapped = format!("{}{{{}}}", owner_enum_cpp, receiver);
            let escaped_method = Self::escape_cpp_method_name(&method_name);
            return format!("({}).{}({})", wrapped, escaped_method, args.join(", "));
        }
        if args.is_empty()
            && self
                .c_like_enum_inherent_method_names
                .contains(&method_name)
            && self.receiver_is_c_like_enum_with_inherent_method(&mc.receiver, &method_name)
            && !self.receiver_has_inherent_method_named(&mc.receiver, &method_name)
            && self
                .receiver_data_enum_owner_with_inherent_method(&mc.receiver, &method_name)
                .is_none()
        {
            let receiver = self.emit_expr_to_string(&mc.receiver);
            if let Some(callee) =
                self.resolve_known_unqualified_free_function_expr_path(&method_name)
            {
                return format!("{}({})", callee, receiver);
            }
            if !self.module_stack.is_empty()
                && self.current_module_has_c_like_owner_with_method(&method_name)
            {
                let module_scope = self.module_stack.join("::");
                let escaped_scope = self.escape_and_rename_qualified_name(&module_scope);
                return format!(
                    "::{}::{}({})",
                    escaped_scope,
                    escape_cpp_keyword(&method_name),
                    receiver
                );
            }
            if let Some(owner_path) = self.lookup_unique_c_like_owner_with_method(&method_name)
                && let Some(callee) = self
                    .resolve_c_like_enum_inherent_method_free_function_for_owner_path(
                        &owner_path,
                        &method_name,
                    )
            {
                return format!("{}({})", callee, receiver);
            }
            if let Ok(path) = syn::parse_str::<syn::Path>(&method_name)
                && let Some(callee) = self.resolve_known_free_function_expr_path(&path)
            {
                return format!("{}({})", callee, receiver);
            }
            return format!("{}({})", escape_cpp_keyword(&method_name), receiver);
        }

        let receiver_is_self = matches!(mc.receiver.as_ref(), syn::Expr::Path(p)
            if p.path.segments.len() == 1 && p.path.segments[0].ident == "self");
        if !receiver_is_self
            && self
                .c_like_enum_inherent_method_names
                .contains(&method_name)
            && !matches!(
                method_name.as_str(),
                "eq" | "ne" | "cmp" | "partial_cmp" | "clone" | "hash" | "fmt"
            )
        {
            // Last-resort bridge for c-like enum inherent methods when receiver type
            // inference is incomplete. Prefer member syntax when available; otherwise
            // use unqualified free-function form so ADL resolves `fn method(self, ...)`.
            let receiver = self.emit_expr_to_string(&mc.receiver);
            let escaped_method = Self::escape_cpp_method_name(&method_name);
            let self_expr = "std::forward<decltype(__self)>(__self)";
            let member_call = if let Some(template_args) = method_template_args.as_deref() {
                if args.is_empty() {
                    format!(
                        "{}.template {}{}()",
                        self_expr, escaped_method, template_args
                    )
                } else {
                    format!(
                        "{}.template {}{}({})",
                        self_expr,
                        escaped_method,
                        template_args,
                        args.join(", ")
                    )
                }
            } else if args.is_empty() {
                format!("{}.{}()", self_expr, escaped_method)
            } else {
                format!("{}.{}({})", self_expr, escaped_method, args.join(", "))
            };
            let mut free_fn = self
                .resolve_scoped_namespace_function_expr_path("rusty_ext", &method_name)
                .or_else(|| {
                    self.resolve_unscoped_namespace_function_expr_path("rusty_ext", &method_name)
                })
                .or_else(|| {
                    if self.module_stack.is_empty()
                        || !self.current_module_has_c_like_owner_with_method(&method_name)
                    {
                        return None;
                    }
                    let module_scope = self.module_stack.join("::");
                    let escaped_scope = self.escape_and_rename_qualified_name(&module_scope);
                    Some(format!(
                        "::{}::{}",
                        escaped_scope,
                        escape_cpp_keyword(&method_name)
                    ))
                })
                .or_else(|| {
                    self.lookup_unique_c_like_owner_with_method(&method_name)
                        .and_then(|owner| {
                            self.resolve_c_like_enum_inherent_method_free_function_for_owner_path(
                                &owner,
                                &method_name,
                            )
                        })
                })
                .unwrap_or_else(|| escaped_method.clone());
            if let Some(template_args) = method_template_args.as_deref() {
                free_fn.push_str(template_args);
            }
            let mut free_args = Vec::with_capacity(args.len() + 1);
            free_args.push(self_expr.to_string());
            free_args.extend(args.iter().cloned());
            let free_call = format!("{}({})", free_fn, free_args.join(", "));
            return format!(
                "([&](auto&& __self) -> decltype(auto) {{ if constexpr (requires {{ {}; }}) {{ return {}; }} else {{ return {}; }} }})({})",
                member_call, member_call, free_call, receiver
            );
        }

        let is_self = matches!(mc.receiver.as_ref(), syn::Expr::Path(p)
            if p.path.segments.len() == 1 && p.path.segments[0].ident == "self");
        if is_self {
            if let Some(static_call) = self.try_emit_static_typed_self_method_call(
                &mc.receiver,
                &method_name,
                method_template_args.as_deref(),
                &args,
                None,
            ) {
                return static_call;
            }
            let escaped_method = Self::escape_cpp_method_name(&method_name);
            if let Some(self_name) = self.current_self_path_override() {
                if let Some(template_args) = method_template_args.as_deref() {
                    format!(
                        "{}.template {}{}({})",
                        self_name,
                        escaped_method,
                        template_args,
                        args.join(", ")
                    )
                } else {
                    format!("{}.{}({})", self_name, escaped_method, args.join(", "))
                }
            } else {
                if let Some(template_args) = method_template_args.as_deref() {
                    format!(
                        "this->template {}{}({})",
                        escaped_method,
                        template_args,
                        args.join(", ")
                    )
                } else {
                    format!("this->{}({})", escaped_method, args.join(", "))
                }
            }
        } else {
            let receiver_expected_owned = if method_name == "into_iter" && mc.args.is_empty() {
                self.infer_into_iter_receiver_expected_type_from_call_expected(
                    &mc.receiver,
                    expected_ty,
                )
            } else if method_name == "transpose" && mc.args.is_empty() {
                self.infer_transpose_receiver_expected_type_from_call_expected(expected_ty)
            } else {
                None
            };
            let receiver_expected = if method_name == "unwrap" && mc.args.is_empty() {
                expected_ty
            } else {
                receiver_expected_owned.as_ref()
            };
            self.emit_receiver_member_call(
                &mc.receiver,
                &method_name,
                method_template_args.as_deref(),
                &args,
                receiver_expected,
            )
        }
    }

    pub(super) fn try_emit_map_err_callable_arg(&self, arg: &syn::Expr) -> Option<String> {
        let path_expr = match self.peel_paren_group_expr(arg) {
            syn::Expr::Path(path) => path,
            _ => return None,
        };
        if path_expr.path.segments.len() != 2 {
            return None;
        }
        let owner_seg = path_expr.path.segments.first()?;
        let method_seg = path_expr.path.segments.last()?;
        if !matches!(owner_seg.arguments, syn::PathArguments::None)
            || !matches!(method_seg.arguments, syn::PathArguments::None)
        {
            return None;
        }
        if method_seg.ident != "simplify" {
            return None;
        }
        Some(format!(
            "[&](auto&& _err) {{ return std::forward<decltype(_err)>(_err).{}(); }}",
            escape_cpp_keyword(&method_seg.ident.to_string())
        ))
    }

    pub(super) fn try_emit_error_trait_callable_with_expected_owner(
        &self,
        arg: &syn::Expr,
        expected_owner_ty: &syn::Type,
    ) -> Option<String> {
        let path_expr = match self.peel_paren_group_expr(arg) {
            syn::Expr::Path(path) => path,
            _ => return None,
        };
        if path_expr.path.segments.len() < 2 {
            return None;
        }
        let owner_seg = path_expr.path.segments.iter().nth_back(1)?;
        let method_seg = path_expr.path.segments.last()?;
        if owner_seg.ident != "Error" || method_seg.ident != "custom" {
            return None;
        }
        if !matches!(owner_seg.arguments, syn::PathArguments::None)
            || !matches!(method_seg.arguments, syn::PathArguments::None)
        {
            return None;
        }
        let segments: Vec<String> = path_expr
            .path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect();
        if self
            .resolve_trait_static_call_type_param_for_segments(&segments)
            .is_some()
        {
            return None;
        }
        let mut owner_cpp = self.map_type(self.peel_reference_paren_group_type(expected_owner_ty));
        if owner_cpp == "auto"
            || owner_cpp.contains("/* TODO")
            || type_string_has_auto_placeholder(&owner_cpp)
        {
            return None;
        }
        owner_cpp = owner_cpp.trim_start_matches("typename ").to_string();
        if owner_cpp.is_empty() {
            return None;
        }
        Some(format!("{}::custom", owner_cpp))
    }

    pub(super) fn try_emit_error_trait_path_with_expected_owner(
        &self,
        path: &syn::Path,
        expected_owner_ty: &syn::Type,
    ) -> Option<String> {
        if path.segments.len() < 2 {
            return None;
        }
        let owner_seg = path.segments.iter().nth_back(1)?;
        let method_seg = path.segments.last()?;
        if owner_seg.ident != "Error" {
            return None;
        }
        if !matches!(owner_seg.arguments, syn::PathArguments::None)
            || !matches!(method_seg.arguments, syn::PathArguments::None)
        {
            return None;
        }
        let segments: Vec<String> = path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect();
        if self
            .resolve_trait_static_call_type_param_for_segments(&segments)
            .is_some()
        {
            return None;
        }
        let mut owner_cpp = self.map_type(self.peel_reference_paren_group_type(expected_owner_ty));
        if owner_cpp == "auto"
            || owner_cpp.contains("/* TODO")
            || type_string_has_auto_placeholder(&owner_cpp)
        {
            return None;
        }
        owner_cpp = owner_cpp.trim_start_matches("typename ").to_string();
        if owner_cpp.is_empty() {
            return None;
        }
        Some(format!(
            "{}::{}",
            owner_cpp,
            escape_cpp_keyword(&method_seg.ident.to_string())
        ))
    }

    /// The first `<T, ...>` type argument of a path type's tail segment
    /// (`Result<T, E>` / `Option<T>` / a `Result<T>`-style alias → `T`).
    fn first_generic_type_arg(ty: &syn::Type) -> Option<&syn::Type> {
        let syn::Type::Path(tp) = ty else {
            return None;
        };
        let last = tp.path.segments.last()?;
        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
            return None;
        };
        args.args.iter().find_map(|a| match a {
            syn::GenericArgument::Type(t) => Some(t),
            _ => None,
        })
    }

    pub(super) fn try_emit_map_callable_arg_with_expected(
        &self,
        arg: &syn::Expr,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let path_expr = match self.peel_paren_group_expr(arg) {
            syn::Expr::Path(path) => path,
            _ => return None,
        };
        let joined = path_expr
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if matches!(
            joined.as_str(),
            "Some" | "Option::Some" | "core::option::Option::Some" | "std::option::Option::Some"
        ) {
            return Some(
                "[](auto&& _v) { return rusty::Some(std::forward<decltype(_v)>(_v)); }".to_string(),
            );
        }
        let call_expected_ty = expected_ty.or(self.current_return_type_hint());
        let target_ty = self
            .expected_result_type_arg(call_expected_ty, 0)
            .or_else(|| self.expected_option_type_arg(call_expected_ty));
        if let Some(target_ty) = target_ty
            && let Some(callable) = self.try_emit_path_callable_arg_to_target(arg, target_ty)
        {
            return Some(callable);
        }
        if let Some(variant_ctor) =
            self.try_emit_data_enum_variant_constructor_callable_path(&path_expr.path)
        {
            return Some(variant_ctor);
        }
        let target_ty = target_ty?;
        let target_cpp = self.map_type(target_ty);
        if target_cpp == "auto"
            || target_cpp.contains("/* TODO")
            || type_string_has_auto_placeholder(&target_cpp)
        {
            return None;
        }
        if matches!(
            joined.as_str(),
            "From::from"
                | "core::convert::From::from"
                | "std::convert::From::from"
                | "Into::into"
                | "core::convert::Into::into"
                | "std::convert::Into::into"
        ) {
            return Some(format!(
                "[&](auto&& _v) -> {} {{ return rusty::from_into<{}>(std::forward<decltype(_v)>(_v)); }}",
                target_cpp, target_cpp
            ));
        }
        if matches!(
            joined.as_str(),
            "AsRef::as_ref" | "core::convert::AsRef::as_ref" | "std::convert::AsRef::as_ref"
        ) {
            return Some(format!(
                "[&](auto&& _v) -> {} {{ return rusty::as_ref_into<{}>(std::forward<decltype(_v)>(_v)); }}",
                target_cpp, target_cpp
            ));
        }
        None
    }

    pub(super) fn try_emit_path_callable_arg_to_target(
        &self,
        arg: &syn::Expr,
        target_ty: &syn::Type,
    ) -> Option<String> {
        let path_expr = match self.peel_paren_group_expr(arg) {
            syn::Expr::Path(path) => path,
            _ => return None,
        };
        let joined = path_expr
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if matches!(
            joined.as_str(),
            "Some" | "Option::Some" | "core::option::Option::Some" | "std::option::Option::Some"
        ) {
            return Some(
                "[](auto&& _v) { return rusty::Some(std::forward<decltype(_v)>(_v)); }".to_string(),
            );
        }
        if let Some(variant_ctor) =
            self.try_emit_data_enum_variant_map_callable_with_target(&path_expr.path, target_ty)
        {
            return Some(variant_ctor);
        }
        let target_cpp = self.map_type(target_ty);
        if target_cpp == "auto"
            || target_cpp.contains("/* TODO")
            || type_string_has_auto_placeholder(&target_cpp)
        {
            return None;
        }
        if matches!(
            joined.as_str(),
            "From::from"
                | "core::convert::From::from"
                | "std::convert::From::from"
                | "Into::into"
                | "core::convert::Into::into"
                | "std::convert::Into::into"
        ) {
            return Some(format!(
                "[&](auto&& _v) -> {} {{ return rusty::from_into<{}>(std::forward<decltype(_v)>(_v)); }}",
                target_cpp, target_cpp
            ));
        }
        if matches!(
            joined.as_str(),
            "AsRef::as_ref" | "core::convert::AsRef::as_ref" | "std::convert::AsRef::as_ref"
        ) {
            return Some(format!(
                "[&](auto&& _v) -> {} {{ return rusty::as_ref_into<{}>(std::forward<decltype(_v)>(_v)); }}",
                target_cpp, target_cpp
            ));
        }
        self.try_emit_assoc_method_path_as_forwarding_lambda(arg)
    }

    /// `Owner::method` passed as a callable value (`key_values.map(Option::unwrap)`,
    /// indexmap get_disjoint_mut) — a receiver-taking associated method lowered
    /// to a type-agnostic forwarding lambda: `.method()` on each element. A raw
    /// `Option::unwrap` in C++ is neither qualified nor a bound member
    /// ("use of undeclared identifier 'Option'"). Conversion traits whose
    /// lowering needs the TARGET type (From/Into/AsRef/TryFrom/TryInto) are
    /// excluded — the typed arms above own them.
    pub(super) fn try_emit_assoc_method_path_as_forwarding_lambda(
        &self,
        arg: &syn::Expr,
    ) -> Option<String> {
        let path_expr = match self.peel_paren_group_expr(arg) {
            syn::Expr::Path(path) => path,
            _ => return None,
        };
        if path_expr.qself.is_some() || path_expr.path.segments.len() != 2 {
            return None;
        }
        if !path_expr
            .path
            .segments
            .iter()
            .all(|seg| seg.arguments.is_empty())
        {
            return None;
        }
        let owner = path_expr.path.segments[0].ident.to_string();
        let method = path_expr.path.segments[1].ident.to_string();
        if matches!(
            owner.as_str(),
            "From" | "Into" | "AsRef" | "AsMut" | "TryFrom" | "TryInto"
        ) {
            return None;
        }
        let owner_is_type_like = owner.chars().next().is_some_and(|c| c.is_ascii_uppercase());
        let method_is_value_like = method.chars().next().is_some_and(|c| c.is_ascii_lowercase());
        if !owner_is_type_like || !method_is_value_like {
            return None;
        }
        // Known owner: the runtime core wrappers, or a locally-declared type.
        let owner_is_known = matches!(owner.as_str(), "Option" | "Result" | "Box")
            || self.simple_ident_is_known_type_name(&owner);
        if !owner_is_known {
            return None;
        }
        let escaped_method = escape_cpp_keyword(&method);
        // A receiver-less associated fn (`Location::from_mark(mark)`) is a
        // STATIC taking the mapped value as its argument — a member call on
        // the value would look the method up on the wrong type.
        if self
            .lookup_owner_method_has_receiver(&owner, &method)
            .is_some_and(|has_receiver| !has_receiver)
            // A GENERIC owner can't be spelled bare (`Slice::method` needs
            // template args) — only non-generic statics take this form.
            && self
                .declared_type_params
                .get(&owner)
                .is_none_or(|params| params.is_empty())
        {
            let owner_cpp = self.emit_path_to_string(&syn::parse_str::<syn::Path>(&owner).ok()?);
            return Some(format!(
                "[](auto&& _v) -> decltype(auto) {{ return {}::{}(std::forward<decltype(_v)>(_v)); }}",
                owner_cpp, escaped_method
            ));
        }
        Some(format!(
            "[](auto&& _v) -> decltype(auto) {{ return std::forward<decltype(_v)>(_v).{}(); }}",
            escaped_method
        ))
    }

    pub(super) fn try_emit_data_enum_variant_map_callable_with_target(
        &self,
        path: &syn::Path,
        target_ty: &syn::Type,
    ) -> Option<String> {
        if path.segments.len() < 2 {
            return None;
        }
        if !matches!(
            path.segments.last().map(|seg| &seg.arguments),
            Some(syn::PathArguments::None)
        ) {
            return None;
        }
        let raw_variant_name = path.segments.last()?.ident.to_string();
        let variant_name = self.canonical_variant_name(&raw_variant_name).to_string();
        if !Self::ident_looks_like_variant_ctor_name(&variant_name) {
            return None;
        }
        let path_enum_name = path.segments.iter().nth_back(1)?.ident.to_string();
        let target_enum_name = self
            .expected_data_enum_name(target_ty)
            .or_else(|| self.expected_type_last_ident(target_ty))?;
        let target_tail = target_enum_name
            .rsplit("::")
            .next()
            .unwrap_or(&target_enum_name);
        let owner_matches = path_enum_name == target_enum_name
            || path_enum_name == target_tail
            || (path_enum_name == "Self"
                && self.current_struct.as_ref().is_some_and(|current| {
                    current == &target_enum_name
                        || current
                            .rsplit("::")
                            .next()
                            .is_some_and(|tail| tail == target_enum_name || tail == target_tail)
                }));
        if !owner_matches {
            return None;
        }
        if self.expected_data_enum_name(target_ty).is_some()
            && !self.enum_has_variant_name(&target_enum_name, &variant_name)
        {
            return None;
        }
        let mut owner_cpp = self.map_type(self.peel_reference_paren_group_type(target_ty));
        while owner_cpp.ends_with('&') {
            owner_cpp.pop();
            owner_cpp = owner_cpp.trim_end().to_string();
        }
        if let Some(rest) = owner_cpp.strip_prefix("const ") {
            owner_cpp = rest.trim_start().to_string();
        }
        if owner_cpp.is_empty()
            || owner_cpp.contains('<')
            || owner_cpp == "auto"
            || owner_cpp.contains("/* TODO")
            || type_string_has_auto_placeholder(&owner_cpp)
        {
            return None;
        }
        let expected_path = self.expected_type_path(target_ty)?;
        let variant_ty = self.data_enum_variant_struct_type_name(expected_path, &variant_name);
        let forward_arg = "std::forward<decltype(_v)>(_v)".to_string();
        let mut wrapped_args = self.wrap_data_enum_variant_tuple_constructor_args(
            &target_enum_name,
            &variant_name,
            vec![forward_arg.clone()],
        );
        if wrapped_args == vec![forward_arg.clone()] {
            let scoped_target_enum = self.scoped_type_key(&target_enum_name);
            wrapped_args = self.wrap_data_enum_variant_tuple_constructor_args(
                &scoped_target_enum,
                &variant_name,
                vec![forward_arg],
            );
        }
        let arg = wrapped_args.into_iter().next()?;
        let variant_expr = format!("{}{{{}}}", variant_ty, arg);
        let payload_expr =
            self.wrap_data_enum_variant_payload_with_expected(target_ty, &variant_expr)?;
        Some(format!(
            "[&](auto&& _v) -> {} {{ return {}; }}",
            owner_cpp, payload_expr
        ))
    }

    pub(super) fn try_emit_toml_write_method_call(
        &self,
        mc: &syn::ExprMethodCall,
        method_name: &str,
        args: &[String],
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let has_toml_write_import = self.scope_import_bindings.values().any(|targets| {
            targets.iter().any(|target| {
                let normalized = normalize_use_import_path(target)
                    .trim_start_matches("::")
                    .to_string();
                normalized == "TomlWrite" || normalized.ends_with("::TomlWrite")
            })
        });
        let has_toml_write_trait = self
            .skipped_module_traits
            .iter()
            .any(|name| name == "TomlWrite" || name.ends_with("::TomlWrite"));
        let receiver_name_looks_like_toml_writer = match self.peel_paren_group_expr(&mc.receiver) {
            syn::Expr::Path(path_expr) => path_expr.path.segments.last().is_some_and(|seg| {
                matches!(seg.ident.to_string().as_str(), "dst" | "writer" | "f")
            }),
            syn::Expr::Field(field_expr) => match &field_expr.member {
                syn::Member::Named(member) => {
                    matches!(member.to_string().as_str(), "dst" | "writer")
                }
                syn::Member::Unnamed(_) => false,
            },
            _ => false,
        };
        let receiver_is_toml_write_like = self
            .infer_simple_expr_type(&mc.receiver)
            .as_ref()
            .is_some_and(|ty| {
                let receiver_cpp = self
                    .map_type(self.peel_reference_paren_group_type(ty))
                    .trim()
                    .to_string();
                receiver_cpp == "rusty::String"
                    || receiver_cpp == "Buffer"
                    || receiver_cpp.ends_with("::Buffer")
            });
        let known_free_functions = self.collect_known_free_function_paths();
        let has_toml_key_helper = known_free_functions.iter().any(|name| {
            name == "key::rusty_ext::write_toml_key"
                || name.ends_with("::key::rusty_ext::write_toml_key")
        });
        let has_toml_value_helper = known_free_functions.iter().any(|name| {
            name == "value::rusty_ext::write_toml_value"
                || name.ends_with("::value::rusty_ext::write_toml_value")
        });
        if !has_toml_write_import
            && !has_toml_write_trait
            && !receiver_name_looks_like_toml_writer
            && !receiver_is_toml_write_like
            && !has_toml_key_helper
            && !has_toml_value_helper
        {
            return None;
        }
        if self.receiver_has_inherent_method_named(&mc.receiver, method_name) {
            return None;
        }

        let raw_receiver = self.emit_expr_to_string_with_expected(&mc.receiver, expected_ty);
        let receiver = if self.method_receiver_needs_parentheses(&mc.receiver) {
            format!("({})", raw_receiver)
        } else {
            raw_receiver
        };

        if method_name == "key" && args.len() == 1 {
            let key_arg = format!("rusty::to_string_view({})", args[0]);
            return Some(format!(
                "::key::rusty_ext::write_toml_key({}, {})",
                key_arg, receiver
            ));
        }
        if method_name == "value" && args.len() == 1 {
            return Some(format!(
                "::value::rusty_ext::write_toml_value({}, {})",
                args[0], receiver
            ));
        }
        if !args.is_empty() {
            return None;
        }

        let fmt_literal = match method_name {
            "open_table_header" => "[",
            "close_table_header" => "]",
            "open_array_of_tables_header" => "[[",
            "close_array_of_tables_header" => "]]",
            "open_inline_table" => "{{",
            "close_inline_table" => "}}",
            "open_array" => "[",
            "close_array" => "]",
            "key_sep" => ".",
            "keyval_sep" => "=",
            "val_sep" => ",",
            "space" => " ",
            "open_comment" => "#",
            "newline" => "\\n",
            _ => return None,
        };
        Some(format!(
            "rusty::write_fmt({}, std::format(\"{}\"))",
            receiver, fmt_literal
        ))
    }

    /// Method names that always lower to a hand-written `rusty::<name>`
    /// runtime helper (forwarding-reference signatures that correctly handle
    /// move-only and primitive receivers). These must NOT be intercepted by
    /// UFCS trait-method lowering: a UFCS per-type trait free function takes
    /// its owned parameters *by value* (faithful to Rust `mut writer: W`), so
    /// passing a move-only lvalue argument (e.g. bitflags
    /// `remaining.write_hex(writer)` where `writer: rusty::String` is
    /// non-copyable) fails the dispatch `requires` and falls back to a member
    /// call on a primitive — a hard error. The runtime helper takes
    /// `Writer&& writer`, so the lvalue binds without a copy. Routing these
    /// names to the helper keeps flag-on output identical to flag-off.
    pub(super) fn method_prefers_runtime_helper_namespace(name: &str) -> bool {
        matches!(name, "size_hint" | "left" | "right" | "write_hex")
    }

    /// `T::trait_method(a0, rest...)` where `T` is an in-scope GENERIC PARAM
    /// and the method is a registered extension (UFCS trait) method. A C++
    /// type param can't host a static trait call — `Q::equivalent(key, ...)`
    /// with Q=int is ill-formed ("type 'int' cannot be used prior to '::'"),
    /// and buried inside a lambda body it surfaces as a bare substitution
    /// failure on the enclosing candidate (indexmap's `equivalent` helper,
    /// 18 errors). Rust's trait-static form makes the first argument the
    /// receiver, so lower it exactly like `a0.trait_method(rest...)` through
    /// the extension dispatcher.
    pub(super) fn try_emit_type_param_trait_static_call(
        &self,
        call: &syn::ExprCall,
    ) -> Option<String> {
        let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(call.func.as_ref()) else {
            return None;
        };
        if path_expr.qself.is_some() {
            return None;
        }
        let segs = &path_expr.path.segments;
        if segs.len() != 2 || call.args.is_empty() {
            return None;
        }
        let owner = segs[0].ident.to_string();
        if !matches!(segs[0].arguments, syn::PathArguments::None) {
            return None;
        }
        let method_seg = segs.last()?;
        let method_name = method_seg.ident.to_string();
        let owner_is_type_param = self.is_type_param_in_scope(&owner)
            || self
                .nested_fn_type_params_stack
                .iter()
                .any(|params| params.contains(&owner));
        if !owner_is_type_param {
            return None;
        }
        if owner == "Self" || !self.extension_method_names.contains(&method_name) {
            return None;
        }
        // Only when the method provably takes `self` is the first argument a
        // receiver. Associated fns without one (Deserialize::
        // deserialize_in_place, Error::invalid_value) must stay on the
        // trait-static routing paths.
        if !self.trait_method_name_always_has_receiver(&method_name) {
            return None;
        }
        let synthetic_mc = syn::ExprMethodCall {
            attrs: Vec::new(),
            receiver: Box::new(call.args[0].clone()),
            dot_token: Default::default(),
            method: method_seg.ident.clone(),
            turbofish: match &method_seg.arguments {
                syn::PathArguments::AngleBracketed(ab) => Some(ab.clone()),
                _ => None,
            },
            paren_token: Default::default(),
            args: call.args.iter().skip(1).cloned().collect(),
        };
        let extra_args: Vec<String> = call
            .args
            .iter()
            .skip(1)
            .map(|arg| self.emit_expr_maybe_move(arg))
            .collect();
        self.try_emit_extension_method_call(&synthetic_mc, &extra_args, None)
    }

    pub(super) fn try_emit_extension_method_call(
        &self,
        mc: &syn::ExprMethodCall,
        args: &[String],
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let method_name = mc.method.to_string();
        if !self.extension_method_names.contains(&method_name) {
            return None;
        }
        let receiver_is_numeric_primitive = self
            .infer_simple_expr_type(&mc.receiver)
            .as_ref()
            .is_some_and(|ty| {
                let receiver_cpp = self
                    .map_type(self.peel_reference_paren_group_type(ty))
                    .trim()
                    .to_string();
                is_numeric_cpp_scalar_type(receiver_cpp.as_str())
            });
        if self.receiver_has_inherent_method_named(&mc.receiver, &method_name)
            && !receiver_is_numeric_primitive
        {
            return None;
        }
        if matches!(method_name.as_str(), "write" | "write_")
            && matches!(
                self.peel_paren_group_expr(&mc.receiver),
                syn::Expr::Index(_)
            )
        {
            return None;
        }

        let is_self_receiver = matches!(mc.receiver.as_ref(), syn::Expr::Path(p)
            if p.path.segments.len() == 1 && p.path.segments[0].ident == "self");
        if is_self_receiver && self.current_self_path_override().is_none() {
            return None;
        }

        let mut receiver = if is_self_receiver {
            self.current_self_path_override()?.to_string()
        } else {
            self.emit_expr_to_string_with_expected(&mc.receiver, expected_ty)
        };
        if matches!(method_name.as_str(), "clear" | "push_str" | "push_char")
            && let Some(cow_self_cpp) = self.active_mut_cow_self_cpp_binding()
        {
            let receiver_is_non_self_local = matches!(
                self.peel_paren_group_expr(&mc.receiver),
                syn::Expr::Path(path_expr)
                    if path_expr.path.segments.len() == 1
                        && path_expr.path.segments[0].ident != "self"
                        && path_expr.path.segments[0].ident != "self_"
            );
            if receiver_is_non_self_local {
                receiver = format!("rusty::to_mut({})", cow_self_cpp);
            }
        }
        if method_name == "next_key" && args.is_empty() {
            if let Some(turbofish) = mc.turbofish.as_ref()
                && turbofish.args.len() == 1
                && let Some(syn::GenericArgument::Type(seed_ty)) = turbofish.args.first()
            {
                let seed_ty = self.peel_reference_paren_group_type(seed_ty);
                if let syn::Type::Path(tp) = seed_ty
                    && tp.qself.is_none()
                    && tp.path.segments.len() == 1
                    && tp.path.segments[0].arguments.is_empty()
                {
                    let seed_ident = tp.path.segments[0].ident.to_string();
                    if seed_ident.starts_with("__Field") {
                        let seed_cpp = self.map_type(seed_ty);
                        if seed_cpp != "auto"
                            && !seed_cpp.contains("/* TODO")
                            && !type_string_has_auto_placeholder(&seed_cpp)
                        {
                            let visitor_cpp = format!("{}Visitor", escape_cpp_keyword(&seed_ident));
                            let seed_adapter = format!(
                                "::de::detail::identifier_seed<{}, {}>{{}}",
                                seed_cpp, visitor_cpp
                            );
                            let seed_call = self
                                .emit_extension_call_with_receiver_autoderef_fallback(
                                    "::de::rusty_ext::next_key_seed",
                                    &receiver,
                                    &[seed_adapter],
                                );
                            let fallback_call = self
                                .emit_extension_call_with_receiver_autoderef_fallback(
                                    &format!("::de::rusty_ext::next_key<{}>", seed_cpp),
                                    &receiver,
                                    &[],
                                );
                            return Some(format!(
                                "([&]() -> decltype(auto) {{ if constexpr (requires {{ typename {}::Value; requires std::is_same_v<typename {}::Value, {}>; {}; }}) {{ return {}; }} else {{ return {}; }} }})()",
                                visitor_cpp,
                                visitor_cpp,
                                seed_cpp,
                                seed_call,
                                seed_call,
                                fallback_call
                            ));
                        }
                    }
                }
            }
        }

        let mut all_args = Vec::with_capacity(args.len() + 1);
        all_args.push(receiver);
        all_args.extend(args.iter().cloned());
        let default_de_template_args = if mc.turbofish.is_none() && args.is_empty() {
            if let Some(access_ty) = self
                .infer_serde_access_method_template_type_from_expected(&method_name, expected_ty)
            {
                let access_cpp = self.map_type(&access_ty);
                if access_cpp != "auto"
                    && !access_cpp.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&access_cpp)
                {
                    Some(format!("<{}>", access_cpp))
                } else {
                    None
                }
            } else {
                match method_name.as_str() {
                    "next_element" => Some("<::de::IgnoredAny>".to_string()),
                    "next_entry" => Some("<::de::IgnoredAny, ::de::IgnoredAny>".to_string()),
                    _ => None,
                }
            }
        } else {
            None
        };
        if matches!(method_name.as_str(), "write" | "write_")
            && let Some(callee) = {
                let private_scoped = self
                    .resolve_scoped_namespace_function_expr_path("private::rusty_ext", &method_name)
                    .or_else(|| {
                        self.resolve_unscoped_namespace_function_expr_path(
                            "private::rusty_ext",
                            &method_name,
                        )
                    })
                    .or_else(|| {
                        let private_path = format!("private::rusty_ext::{}", method_name);
                        syn::parse_str::<syn::Path>(&private_path)
                            .ok()
                            .and_then(|path| self.resolve_known_free_function_expr_path(&path))
                    });
                if private_scoped.is_some() {
                    private_scoped
                } else {
                    let mut direct =
                        self.resolve_known_unqualified_free_function_expr_path(&method_name);
                    if !direct
                        .as_deref()
                        .is_some_and(|path| path.contains("::rusty_ext::"))
                        || direct.as_deref() == Some("::rusty_ext::write_")
                    {
                        direct = None;
                    }
                    direct
                }
            }
        {
            return Some(self.emit_extension_call_with_receiver_autoderef_fallback(
                &callee,
                &all_args[0],
                &all_args[1..],
            ));
        }
        if matches!(method_name.as_str(), "index" | "index_mut")
            && args.len() == 1
            && self.index_trait_arg_supports_bracket_access(&mc.args[0])
        {
            // Operator traits are emitted as C++ operators on concrete types.
            // Calling them through `rusty_ext::index(...)` can fail when no local
            // extension shim exists (e.g. delegating to Vec's Index impl).
            return Some(format!("{}[{}]", all_args[0], args[0]));
        }
        let should_prefer_runtime_namespace =
            Self::method_prefers_runtime_helper_namespace(&method_name);
        let is_cross_source_extension_hint =
            self.external_extension_method_hints.contains(&method_name);
        let extension_ns = if should_prefer_runtime_namespace {
            "rusty"
        } else if self.local_extension_method_names.contains(&method_name)
            || is_cross_source_extension_hint
        {
            "rusty_ext"
        } else {
            "rusty"
        };
        if extension_ns == "rusty_ext" {
            if let Some(qualified_fn) = self
                .resolve_scoped_namespace_function_expr_path("rusty_ext", &method_name)
                .or_else(|| {
                    self.resolve_unscoped_namespace_function_expr_path("rusty_ext", &method_name)
                })
            {
                let mut callee = qualified_fn;
                if let Some(default_args) = default_de_template_args
                    && !callee.contains('<')
                {
                    callee.push_str(&default_args);
                }
                return Some(self.emit_extension_call_with_receiver_autoderef_fallback(
                    &callee,
                    &all_args[0],
                    &all_args[1..],
                ));
            }
            if method_name == "deserialize" {
                return Some(format!(
                    "::de::rusty_ext::deserialize({})",
                    all_args.join(", ")
                ));
            }
            if method_name == "deserialize_in_place" && all_args.len() == 3 {
                return Some(format!(
                    "::de::rusty_ext::deserialize_in_place({})",
                    all_args.join(", ")
                ));
            }
            if method_name == "fmt" && all_args.len() == 2 {
                return Some(format!(
                    "::de::rusty_ext::fmt({}, {})",
                    all_args[0], all_args[1]
                ));
            }
            if method_name == "serialize" && all_args.len() == 2 {
                return Some(self.emit_serialize_dispatch_call(&all_args[0], &all_args[1]));
            }
            if method_name == "clear" && all_args.len() == 1 {
                return Some(format!("{}.clear()", all_args[0]));
            }
            if method_name == "push_str" && all_args.len() == 2 {
                return Some(format!("{}.push_str({})", all_args[0], all_args[1]));
            }
            // Cross-source extension hints are name-only and can collide with
            // common inherent/runtime methods (`get`, `newline`). Keep these
            // on receiver method syntax when no concrete extension path exists.
            //
            // The std iterator-entry methods (`into_iter`/`iter`/`iter_mut`) are
            // IntoIterator/Iterator surface with dedicated lowering (rusty::iter,
            // the into_iter bridge at emit_expr.rs ~5915-5940), never user
            // extension shims. Routing them through the unqualified
            // `rusty_ext::<m>` fallback emits a call that, inside a nested
            // `*::rusty_ext` namespace, binds the enclosing namespace (no such
            // member) and hard-errors (serde_core's 40 `into_iter` errors). Keep
            // them on method syntax so the dedicated iterator handling applies.
            let skip_unqualified_cross_source_fallback = matches!(
                method_name.as_str(),
                "get" | "newline" | "whitespace" | "write" | "write_"
                    | "into_iter" | "iter" | "iter_mut"
            );
            if is_cross_source_extension_hint && !skip_unqualified_cross_source_fallback {
                // Cross-target parity transpilation can call extension shims from
                // imported modules where local symbol metadata is unavailable.
                // Use an unqualified `rusty_ext::...` call and preserve Rust
                // method-call autoderef on pointer-like iter items.
                let unresolved_cross_source_fn =
                    format!("rusty_ext::{}", escape_cpp_keyword(&method_name));
                let unresolved_cross_source_fn =
                    if let Some(default_args) = default_de_template_args {
                        format!("{}{}", unresolved_cross_source_fn, default_args)
                    } else {
                        unresolved_cross_source_fn
                    };
                return Some(self.emit_extension_call_with_receiver_autoderef_fallback(
                    &unresolved_cross_source_fn,
                    &all_args[0],
                    &all_args[1..],
                ));
            }
            // No visible extension free function in scope: keep method syntax so
            // common inherent methods do not get rewritten to non-existent
            // extension shims.
            return None;
        }
        Some(format!(
            "{}::{}({})",
            extension_ns,
            escape_cpp_keyword(&method_name),
            all_args.join(", ")
        ))
    }

    pub(super) fn try_emit_get_unchecked_range_method_call(
        &self,
        mc: &syn::ExprMethodCall,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        if mc.args.len() != 1 {
            return None;
        }
        let index_expr = &mc.args[0];

        if let syn::Expr::Range(range) = self.peel_paren_group_expr(index_expr) {
            let base = self.emit_expr_to_string_with_expected(&mc.receiver, expected_ty);
            let start = range.start.as_ref().map(|e| self.emit_expr_to_string(e));
            let end = range.end.as_ref().map(|e| self.emit_expr_to_string(e));
            let inclusive = matches!(range.limits, syn::RangeLimits::Closed(_));
            let emitted = match (start, end, inclusive) {
                (Some(s), Some(e), false) => format!("rusty::slice({}, {}, {})", base, s, e),
                (Some(s), Some(e), true) => {
                    format!("rusty::slice_inclusive({}, {}, {})", base, s, e)
                }
                (Some(s), None, _) => format!("rusty::slice_from({}, {})", base, s),
                (None, Some(e), false) => format!("rusty::slice_to({}, {})", base, e),
                (None, Some(e), true) => format!("rusty::slice_to_inclusive({}, {})", base, e),
                (None, None, _) => {
                    let base_is_string_like = self
                        .infer_simple_expr_type(&mc.receiver)
                        .as_ref()
                        .is_some_and(|ty| {
                            self.is_known_string_like_type(ty)
                                || self.map_type(ty) == "std::string_view"
                        });
                    if self.expected_type_is_string_view(expected_ty) || base_is_string_like {
                        self.emit_from_conversion_to_target(&mc.receiver, "std::string_view")
                    } else {
                        format!("rusty::slice_full({})", base)
                    }
                }
            };
            return Some(emitted);
        }

        let base_is_string_like = self
            .infer_simple_expr_type(&mc.receiver)
            .as_ref()
            .is_some_and(|ty| {
                self.is_known_string_like_type(ty) || self.map_type(ty) == "std::string_view"
            });
        let is_runtime_range_index = self
            .infer_simple_expr_type(index_expr)
            .as_ref()
            .is_some_and(|index_ty| {
                let index_ty = self.peel_reference_paren_group_type(index_ty);
                let mapped = self.map_type(index_ty);
                let canonical = mapped
                    .chars()
                    .filter(|c| !c.is_ascii_whitespace())
                    .collect::<String>();
                canonical.starts_with("rusty::range<")
                    || canonical.starts_with("rusty::range_from<")
                    || canonical.starts_with("rusty::range_inclusive<")
                    || canonical.starts_with("rusty::range_to<")
                    || canonical.starts_with("rusty::range_to_inclusive<")
                    || canonical == "rusty::range_full"
            });
        let is_span_unwrap_range_hint = matches!(
            self.peel_paren_group_expr(index_expr),
            syn::Expr::MethodCall(outer)
                if outer.method == "unwrap"
                    && outer.args.is_empty()
                    && matches!(
                        self.peel_paren_group_expr(&outer.receiver),
                        syn::Expr::MethodCall(inner)
                            if inner.method == "span" && inner.args.is_empty()
                    )
        );
        if !is_runtime_range_index && !is_span_unwrap_range_hint {
            return None;
        }
        let base = if self.expected_type_is_string_view(expected_ty) || base_is_string_like {
            self.emit_from_conversion_to_target(&mc.receiver, "std::string_view")
        } else {
            self.emit_expr_to_string_with_expected(&mc.receiver, expected_ty)
        };
        let index = self.emit_expr_to_string(index_expr);
        Some(format!("rusty::index_with_range({}, {})", base, index))
    }

    /// Emit an expression with optional expected type context from its parent.
    /// Currently used for typed `let` initializers to guide enum variant constructor calls.
    pub(super) fn emit_expr_to_string_with_expected(
        &self,
        expr: &syn::Expr,
        expected_ty: Option<&syn::Type>,
    ) -> String {
        if self.expected_type_is_string_view(expected_ty)
            && matches!(self.peel_paren_group_expr(expr), syn::Expr::Field(_))
        {
            return self.emit_from_conversion_to_target(expr, "std::string_view");
        }
        if self.expected_type_maps_to_rusty_string(expected_ty)
            && self.expr_is_rusty_to_string_call(expr)
        {
            let inner = self.emit_expr_to_string(expr);
            return format!("rusty::String::from({})", inner);
        }
        match expr {
            syn::Expr::Path(path_expr)
                if path_expr
                    .path
                    .segments
                    .last()
                    .is_some_and(|seg| seg.ident == "PhantomData")
                    && path_expr.path.segments.last().is_some_and(|seg| {
                        matches!(seg.arguments, syn::PathArguments::None)
                            || matches!(
                                seg.arguments,
                                syn::PathArguments::AngleBracketed(ref ab)
                                    if ab.args.is_empty()
                            )
                    }) =>
            {
                if let Some(expected) = expected_ty {
                    let expected_cpp = self.map_type(expected);
                    if expected_cpp.starts_with("rusty::PhantomData<")
                        && !expected_cpp.contains("/* TODO")
                        && !type_string_has_auto_placeholder(&expected_cpp)
                    {
                        return format!("{}{{}}", expected_cpp);
                    }
                }
                return "rusty::PhantomData<std::tuple<>>{}".to_string();
            }
            syn::Expr::Lit(lit) => self.emit_lit_with_expected(&lit.lit, expected_ty),
            syn::Expr::Call(call) => self.emit_call_expr_to_string(call, expected_ty),
            syn::Expr::MethodCall(mc) => self.emit_method_call_expr_to_string(mc, expected_ty),
            syn::Expr::Match(match_expr) => self.emit_match_expr_to_string(match_expr, expected_ty),
            syn::Expr::Try(try_expr) => {
                let mut try_operand_expected: Option<syn::Type> = None;
                if let Some(ok_expected_ty) = expected_ty {
                    let fallback_try_err_ty = self
                        .current_return_type_hint()
                        .and_then(|ret_ty| self.expected_result_type_arg(Some(ret_ty), 1))
                        .filter(|err_ty| {
                            !self.type_contains_infer(err_ty)
                                && !self.type_contains_in_scope_type_param(err_ty)
                                && !self.type_contains_unresolved_placeholder_like(err_ty)
                                && !self.type_contains_unbound_single_letter_generic(err_ty)
                        })
                        .cloned()
                        .unwrap_or_else(|| parse_quote!(()));
                    if let Some(operand_ty) = self.infer_simple_expr_type(&try_expr.expr)
                        && let Some((owner, type_args)) =
                            self.option_or_result_type_args(&operand_ty)
                    {
                        if owner == "Option" {
                            try_operand_expected = Some(parse_quote!(Option<#ok_expected_ty>));
                        } else if owner == "Result" {
                            let err_ty = type_args
                                .get(1)
                                .cloned()
                                .filter(|ty| {
                                    !self.type_contains_infer(ty)
                                        && !self.type_contains_in_scope_type_param(ty)
                                        && !self.type_contains_unresolved_placeholder_like(ty)
                                        && !self.type_contains_unbound_single_letter_generic(ty)
                                })
                                .unwrap_or_else(|| fallback_try_err_ty.clone());
                            try_operand_expected =
                                Some(parse_quote!(Result<#ok_expected_ty, #err_ty>));
                        }
                    } else if matches!(
                        self.peel_paren_group_expr(&try_expr.expr),
                        syn::Expr::MethodCall(syn::ExprMethodCall { method, .. })
                            if matches!(
                                method.to_string().as_str(),
                                "map_err" | "ok_or" | "ok_or_else" | "transpose"
                            )
                    ) {
                        // Keep parse/ok_or chains typed through `?` in expression position.
                        try_operand_expected =
                            Some(parse_quote!(Result<#ok_expected_ty, #fallback_try_err_ty>));
                    } else if matches!(
                        self.peel_paren_group_expr(&try_expr.expr),
                        syn::Expr::MethodCall(syn::ExprMethodCall { method, .. })
                            if matches!(
                                method.to_string().as_str(),
                                "next_element" | "next_key" | "next_value"
                            )
                    ) {
                        // Serde accessors infer their method template from the
                        // unwrapped `?` result, e.g. `while let Some(v) =
                        // seq.next_element()?` should instantiate
                        // `next_element<T>()` from the `Option<T>` context.
                        try_operand_expected =
                            Some(parse_quote!(Result<#ok_expected_ty, #fallback_try_err_ty>));
                    } else if matches!(
                        self.peel_paren_group_expr(&try_expr.expr),
                        syn::Expr::Call(call)
                            if call.args.len() == 1
                                && matches!(
                                    self.peel_paren_group_expr(call.func.as_ref()),
                                    syn::Expr::Path(path_expr)
                                        if path_expr
                                            .path
                                            .segments
                                            .last()
                                            .is_some_and(|seg| seg.ident == "deserialize")
                                )
                    ) {
                        // Generic free-function `deserialize(...)` calls rely on
                        // expected Result context to recover their template output type.
                        try_operand_expected =
                            Some(parse_quote!(Result<#ok_expected_ty, #fallback_try_err_ty>));
                    }
                }
                let inner = self.emit_expr_to_string_with_expected(
                    &try_expr.expr,
                    try_operand_expected.as_ref(),
                );
                self.emit_try_macro_invocation_with_mode(
                    &inner,
                    self.try_operand_prefers_plain_try_macro(&try_expr.expr),
                )
            }
            syn::Expr::Binary(bin) => {
                self.emit_binary_expr_to_string_with_expected(bin, expected_ty)
            }
            syn::Expr::Unary(un) => {
                if matches!(un.op, syn::UnOp::Deref(_)) && !self.is_expr_raw_pointer_like(&un.expr)
                {
                    let operand = self.peel_paren_group_expr(&un.expr);
                    if let syn::Expr::Path(path) = operand
                        && path.path.segments.len() == 1
                    {
                        let local_name = path.path.segments[0].ident.to_string();
                        if self.is_rebind_reference_binding(&local_name) {
                            let mapped = self
                                .lookup_local_binding_cpp_name(&local_name)
                                .unwrap_or_else(|| escape_cpp_keyword(&local_name));
                            return format!("*{}", mapped);
                        }
                    }
                    let collapse_from_ref_shape = self.is_expr_reference_like(operand)
                        || self
                            .infer_simple_expr_type(operand)
                            .as_ref()
                            .is_some_and(|ty| self.type_is_reference_like(ty));
                    let collapse_from_unresolved_local_ref = if let syn::Expr::Path(path) = operand
                    {
                        if path.path.segments.len() == 1 {
                            let local_name = path.path.segments[0].ident.to_string();
                            self.lookup_local_binding_type(&local_name)
                                .is_some_and(|local_ty| {
                                    let peeled_local =
                                        self.peel_reference_paren_group_type(&local_ty);
                                    let local_is_known_deref_owner = matches!(peeled_local, syn::Type::Path(tp)
                                        if tp.path.segments.last().is_some_and(|seg| {
                                            matches!(
                                                seg.ident.to_string().as_str(),
                                                "Box"
                                                    | "Rc"
                                                    | "Arc"
                                                    | "Lazy"
                                                    | "Ref"
                                                    | "RefMut"
                                                    | "MutexGuard"
                                                    | "SpinMutexGuard"                                                    | "RwLockReadGuard"
                                                    | "RwLockWriteGuard"
                                            )
                                        }));
                                    !local_is_known_deref_owner
                                        && (self.type_contains_in_scope_type_param(&local_ty)
                                            || self.type_contains_unbound_single_letter_generic(
                                                &local_ty,
                                            )
                                            || self
                                                .type_contains_unresolved_placeholder_like(
                                                    &local_ty,
                                                ))
                                })
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    let collapse_local_nonpointer_path = if let syn::Expr::Path(path) = operand {
                        if path.path.segments.len() == 1 {
                            let local_name = path.path.segments[0].ident.to_string();
                            if self.is_reference_binding_lowered_to_pointer_storage(&local_name) {
                                false
                            } else {
                                let local_ty = self.lookup_local_binding_type(&local_name);
                                let local_is_known_deref_owner =
                                    local_ty.as_ref().is_some_and(|local_ty| {
                                        let peeled_local =
                                            self.peel_reference_paren_group_type(local_ty);
                                        matches!(peeled_local, syn::Type::Path(tp)
                                        if tp.path.segments.last().is_some_and(|seg| {
                                            matches!(
                                                seg.ident.to_string().as_str(),
                                                "Box"
                                                    | "Rc"
                                                    | "Arc"
                                                    | "Lazy"
                                                    | "Ref"
                                                    | "RefMut"
                                                    | "MutexGuard"
                                                    | "SpinMutexGuard"
                                                    | "RwLockReadGuard"
                                                    | "RwLockWriteGuard"
                                            )
                                        }))
                                    });
                                let local_is_known_pointer =
                                    local_ty.as_ref().is_some_and(|local_ty| {
                                        matches!(
                                            self.peel_reference_paren_group_type(local_ty),
                                            syn::Type::Ptr(_)
                                        )
                                    });
                                let local_type_is_concrete = local_ty.as_ref().is_some_and(|ty| {
                                    !self.type_contains_infer(ty)
                                        && !self.type_contains_in_scope_type_param(ty)
                                        && !self.type_contains_unbound_single_letter_generic(ty)
                                        && !self.type_contains_unresolved_placeholder_like(ty)
                                });
                                let local_is_in_scope =
                                    self.lookup_local_binding_cpp_name(&local_name).is_some();
                                local_is_in_scope
                                    && local_type_is_concrete
                                    && !local_is_known_deref_owner
                                    && !local_is_known_pointer
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    let collapse_without_expected = if expected_ty.is_none() {
                        if let syn::Expr::Path(path) = operand {
                            if path.path.segments.len() == 1 {
                                let local_name = path.path.segments[0].ident.to_string();
                                if !(self.is_expr_reference_like(operand)
                                    || collapse_from_ref_shape)
                                {
                                    false
                                } else {
                                    let local_is_known_deref_owner = self
                                        .lookup_local_binding_type(&local_name)
                                        .is_some_and(|local_ty| {
                                            let peeled_local =
                                                self.peel_reference_paren_group_type(&local_ty);
                                            matches!(peeled_local, syn::Type::Path(tp)
                                            if tp.path.segments.last().is_some_and(|seg| {
                                                matches!(
                                                    seg.ident.to_string().as_str(),
                                                    "Box"
                                                        | "Rc"
                                                        | "Arc"
                                                        | "Lazy"
                                                        | "Ref"
                                                        | "RefMut"
                                                        | "MutexGuard"
                                                        | "SpinMutexGuard"                                                        | "RwLockReadGuard"
                                                        | "RwLockWriteGuard"
                                                )
                                            }))
                                        });
                                    !local_is_known_deref_owner
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    if (expected_ty.is_some_and(|ty| self.type_is_reference_like(ty))
                        && (collapse_from_ref_shape || collapse_from_unresolved_local_ref))
                        || collapse_without_expected
                        || collapse_local_nonpointer_path
                    {
                        return self.emit_expr_to_string_with_expected(&un.expr, expected_ty);
                    }
                }
                self.emit_expr_to_string(expr)
            }
            syn::Expr::RawAddr(raw) => self.emit_raw_addr_expr_to_string(raw),
            syn::Expr::Reference(r) => {
                if let Some(span_expr) =
                    self.try_emit_reference_array_literal_with_expected_span(r, expected_ty)
                {
                    return span_expr;
                }
                if let Some(span_expr) =
                    self.try_emit_reference_expr_with_expected_span_storage(r, expected_ty)
                {
                    return span_expr;
                }
                if self.expected_type_is_string_view(expected_ty) {
                    return self.emit_expr_to_string_with_expected(&r.expr, expected_ty);
                }
                if let Some(expected_inner) = self.expected_reference_inner_type(expected_ty)
                    && self.expected_type_is_string_view(Some(expected_inner))
                    && matches!(
                        self.peel_paren_group_expr(&r.expr),
                        syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(_),
                            ..
                        })
                    )
                    && let Some(expected_ty) = expected_ty
                {
                    let literal =
                        self.emit_expr_to_string_with_expected(&r.expr, Some(expected_inner));
                    let expected_cpp = self.map_type(expected_ty);
                    let storage_decl = if expected_cpp.trim_start().starts_with("const ") {
                        "static const std::string_view"
                    } else {
                        "static std::string_view"
                    };
                    return format!(
                        "[&]() -> {} {{ {} _rusty_str_ref_tmp = {}; return _rusty_str_ref_tmp; }}()",
                        expected_cpp, storage_decl, literal
                    );
                }
                if let Some(expected_inner) = self.expected_reference_inner_type(expected_ty) {
                    return self.emit_expr_to_string_with_expected(&r.expr, Some(expected_inner));
                }
                if expected_ty.is_some_and(|ty| {
                    self.is_type_raw_pointer_like(self.peel_reference_paren_group_type(ty))
                }) && !self.is_stable_reference_lvalue_expr(&r.expr)
                {
                    let expected_pointee_ty = expected_ty
                        .and_then(|ty| self.extract_pointer_pointee_info_from_type(ty))
                        .map(|(pointee, _)| pointee);
                    let inner = self
                        .emit_expr_to_string_with_expected(&r.expr, expected_pointee_ty.as_ref());
                    let expected_ptr_cpp = expected_ty
                        .map(|ty| self.map_type(ty))
                        .filter(|mapped| {
                            !mapped.contains("/* TODO") && !type_string_has_auto_placeholder(mapped)
                        })
                        .unwrap_or_else(|| "auto*".to_string());
                    return format!(
                        "[&]() -> {} {{ auto _rusty_ref_ptr_value = ({}); thread_local std::optional<std::remove_cvref_t<decltype(_rusty_ref_ptr_value)>> _rusty_ref_ptr_tmp; _rusty_ref_ptr_tmp.reset(); _rusty_ref_ptr_tmp.emplace(std::move(_rusty_ref_ptr_value)); return &*_rusty_ref_ptr_tmp; }}()",
                        expected_ptr_cpp, inner
                    );
                }
                let ref_inner = self.peel_paren_group_expr(&r.expr);
                if self.in_deref_method_scope() || self.in_deref_mut_method_scope() {
                    return self.emit_expr_to_string_with_expected(ref_inner, expected_ty);
                }
                if let syn::Expr::Index(idx) = ref_inner {
                    if self.is_slice_range_index_expr(&idx.index) {
                        return self.emit_expr_to_string_with_expected(ref_inner, expected_ty);
                    }
                }
                if let syn::Expr::Unary(un) = ref_inner {
                    if matches!(un.op, syn::UnOp::Deref(_))
                        && expected_ty.is_some_and(|ty| {
                            self.is_type_raw_pointer_like(self.peel_reference_paren_group_type(ty))
                        })
                    {
                        // Keep pointer shape for reborrows in pointer-typed contexts.
                        return self.emit_expr_to_string_with_expected(&un.expr, expected_ty);
                    }
                    if matches!(un.op, syn::UnOp::Deref(_))
                        && self.should_collapse_reborrow_of_deref_operand(&un.expr)
                    {
                        return self.emit_expr_to_string_with_expected(ref_inner, expected_ty);
                    }
                }
                if let syn::Expr::MethodCall(mc) = self.peel_paren_group_expr(&r.expr)
                    && mc.method == "into_bytes"
                {
                    // Common expanded serde pattern: `&err.into_bytes()` is a borrow
                    // for byte-slice contexts. In C++, taking `&` here creates a
                    // pointer-to-container temporary. Keep the value expression and
                    // let surrounding span coercions normalize it.
                    return self.emit_expr_to_string_with_expected(&r.expr, expected_ty);
                }
                // In Rust, `&expr` borrows the expression. In C++, `&expr` takes
                // the address, which doesn't work on rvalues. Strip `&` for
                // specific known rvalue patterns that produce temporaries and
                // should pass by value to const-ref parameters.
                let inner_is_known_rvalue_call = match self.peel_paren_group_expr(&r.expr) {
                    syn::Expr::Call(call) => {
                        // Strip & for known formatting/allocation function calls
                        let func_str = self.emit_expr_to_string(&call.func);
                        func_str.contains("fmt::format")
                            || func_str.contains("must_use")
                            || func_str.contains("to_string")
                    }
                    syn::Expr::Macro(m) => {
                        // Strip & for macro invocations like format!(), vec!()
                        let name = m
                            .mac
                            .path
                            .segments
                            .last()
                            .map(|s| s.ident.to_string())
                            .unwrap_or_default();
                        matches!(name.as_str(), "format" | "format_args" | "vec")
                    }
                    _ => false,
                };
                if inner_is_known_rvalue_call {
                    self.emit_expr_to_string_with_expected(&r.expr, expected_ty)
                } else if matches!(self.peel_paren_group_expr(&r.expr), syn::Expr::Reference(_)) {
                    // Double reference `&&expr` in Rust → just `&expr` in C++
                    self.emit_expr_to_string_with_expected(&r.expr, expected_ty)
                } else if let syn::Expr::Unary(un) = self.peel_paren_group_expr(&r.expr) {
                    if matches!(un.op, syn::UnOp::Deref(_)) {
                        let deref_operand = self.peel_paren_group_expr(&un.expr);
                        if let syn::Expr::MethodCall(mc) = deref_operand
                            && matches!(mc.method.to_string().as_str(), "as_ref" | "as_mut")
                        {
                            let collapse_from_expected =
                                self.expected_option_type_arg(expected_ty).is_some();
                            let collapse_from_inferred = self
                                .infer_simple_expr_type(deref_operand)
                                .is_some_and(|ty| {
                                    self.option_or_result_type_args(&ty).is_some()
                                        || self.type_is_reference_like(&ty)
                                });
                            if collapse_from_expected || collapse_from_inferred {
                                // `&*x.as_ref()` / `&*x.as_mut()` should collapse when the
                                // method already yields Option/reference-like output.
                                return self
                                    .emit_expr_to_string_with_expected(deref_operand, expected_ty);
                            }
                        }
                        // Rust `&*expr` is a reborrow. Preserve the deref shape so
                        // raw-pointer borrows lower as `*ptr` instead of `ptr`.
                        self.emit_expr_to_string_with_expected(ref_inner, expected_ty)
                    } else {
                        let inner = self.emit_expr_to_string_with_expected(&r.expr, expected_ty);
                        format!("&{}", inner)
                    }
                } else {
                    // For simple local variable paths, strip & when the expression
                    // is NOT a raw pointer type. In C++, passing a value to
                    // const T& doesn't need explicit &.
                    // Do NOT strip for Field access (&self.field) which may need
                    // the address-of for pointer/reference semantics.
                    let ref_inner_peeled = self.peel_paren_group_expr(&r.expr);
                    if self
                        .infer_simple_expr_type(&r.expr)
                        .as_ref()
                        .is_some_and(|ty| self.type_is_slice_or_span_like(ty))
                    {
                        let inner = self.emit_expr_to_string_with_expected(&r.expr, expected_ty);
                        if r.mutability.is_some() {
                            return format!("rusty::as_mut_slice({})", inner);
                        }
                        return format!("rusty::as_slice({})", inner);
                    }
                    if r.mutability.is_some()
                        && expected_ty.is_some_and(|expected| {
                            !self.type_is_reference_like(expected)
                                && self.infer_simple_expr_type(&r.expr).as_ref().is_some_and(
                                    |inner_ty| Self::types_equivalent_by_tokens(inner_ty, expected),
                                )
                        })
                        && self.is_stable_reference_lvalue_expr(&r.expr)
                    {
                        let inner = self.emit_expr_to_string_with_expected(&r.expr, None);
                        return format!("&{}", inner);
                    }
                    // In Rust, both `&x` and `&mut x` borrow the variable.
                    // In C++, passing to const T& or T& doesn't need `&` — just pass the name.
                    // For `&mut path`, we also strip because C++ references bind automatically.
                    if matches!(ref_inner_peeled, syn::Expr::Path(_))
                        && !self.is_expr_raw_pointer_like(&r.expr)
                    {
                        return self.emit_expr_to_string_with_expected(&r.expr, expected_ty);
                    }
                    let inner = self.emit_expr_to_string_with_expected(&r.expr, expected_ty);
                    if !self.is_stable_reference_lvalue_expr(&r.expr) {
                        return format!("rusty::addr_of_temp({})", inner);
                    }
                    if inner.starts_with('&') {
                        inner
                    } else {
                        format!("&{}", inner)
                    }
                }
            }
            syn::Expr::Paren(p) => {
                let inner = self.emit_expr_to_string_with_expected(&p.expr, expected_ty);
                format!("({})", inner)
            }
            syn::Expr::Group(g) => self.emit_expr_to_string_with_expected(&g.expr, expected_ty),
            syn::Expr::Cast(cast_expr) => {
                if matches!(cast_expr.ty.as_ref(), syn::Type::Infer(_))
                    && let Some(expected) = expected_ty
                {
                    let expected = self.peel_reference_paren_group_type(expected);
                    if !matches!(expected, syn::Type::Infer(_)) {
                        return self.emit_cast_expr_to_string_with_target_override(
                            cast_expr,
                            Some(expected),
                        );
                    }
                }
                self.emit_cast_expr_to_string_with_target_override(cast_expr, None)
            }
            syn::Expr::Tuple(tup) => {
                if let Some(expected_tuple_ty) = self.expected_tuple_type(expected_ty) {
                    let elems: Vec<String> = tup
                        .elems
                        .iter()
                        .enumerate()
                        .map(|(idx, e)| {
                            self.emit_tuple_element_with_expected_type(
                                e,
                                expected_tuple_ty.elems.iter().nth(idx),
                            )
                        })
                        .collect();
                    if self.tuple_expected_needs_typed_constructor(&expected_tuple_ty) {
                        let expected_tuple_cpp =
                            self.map_type(&syn::Type::Tuple(expected_tuple_ty.clone()));
                        return format!("{}{{{}}}", expected_tuple_cpp, elems.join(", "));
                    }
                    return format!("std::make_tuple({})", elems.join(", "));
                }
                // `std::make_tuple` DECAYS references to values, copying the
                // referent — fatal for a tuple element that is a reference to a
                // non-copyable type (`(event, mark)` where `event: &Event`, and
                // Event has a deleted copy ctor). When every element type is
                // inferable and at least one is such a reference, build an
                // explicit `std::tuple<T0, T1>{...}` that preserves the
                // reference element.
                let elem_types: Option<Vec<syn::Type>> = tup
                    .elems
                    .iter()
                    .map(|e| self.infer_simple_expr_type(e))
                    .collect();
                if let Some(elem_types) = elem_types.filter(|tys| {
                    tys.iter().any(|ty| self.type_is_non_copyable_referent(ty))
                }) {
                    let elem_cpps: Vec<String> =
                        elem_types.iter().map(|t| self.map_type(t)).collect();
                    if elem_cpps.iter().all(|c| {
                        !c.is_empty()
                            && c != "auto"
                            && !c.contains("/* TODO")
                            && !type_string_has_auto_placeholder(c)
                    }) {
                        let elems: Vec<String> = tup
                            .elems
                            .iter()
                            .zip(elem_types.iter())
                            .map(|(e, ty)| self.emit_tuple_element_with_expected_type(e, Some(ty)))
                            .collect();
                        return format!(
                            "std::tuple<{}>{{{}}}",
                            elem_cpps.join(", "),
                            elems.join(", ")
                        );
                    }
                }
                let elems: Vec<String> = tup
                    .elems
                    .iter()
                    .map(|e| self.emit_expr_to_string_with_expected_and_move_if_needed(e, None))
                    .collect();
                format!("std::make_tuple({})", elems.join(", "))
            }
            syn::Expr::Array(array_expr) => {
                self.emit_array_expr_to_string_with_expected(array_expr, expected_ty)
            }
            syn::Expr::Repeat(repeat_expr) => {
                if let Some((elem_ty, len_ty)) = self.expected_fixed_array_type(expected_ty) {
                    self.emit_repeat_expr_with_fixed_array_hint(repeat_expr, elem_ty, len_ty)
                } else if let Some(elem_ty) = self.expected_array_element_type(expected_ty) {
                    self.emit_repeat_expr_with_element_hint(repeat_expr, elem_ty)
                } else if let Some(fixed_array_expr) =
                    self.maybe_emit_repeat_expr_with_size_of_len_hint(repeat_expr)
                {
                    fixed_array_expr
                } else {
                    let val = self.emit_expr_to_string(&repeat_expr.expr);
                    let len = self.emit_expr_to_string(&repeat_expr.len);
                    format!("rusty::array_repeat({}, {})", val, len)
                }
            }
            syn::Expr::Struct(struct_expr) => {
                self.emit_struct_expr_to_string_with_expected(struct_expr, expected_ty)
            }
            syn::Expr::Index(idx) => {
                if let Some(slice_expr) = self.try_emit_slice_index_expr_to_string(idx, expected_ty)
                {
                    return slice_expr;
                }
                if let Some(unreachable_expr) =
                    self.try_emit_empty_array_index_expr_to_string(idx, expected_ty)
                {
                    return unreachable_expr;
                }
                self.emit_index_expr_to_string(idx, expected_ty)
            }
            syn::Expr::Path(path) => {
                if let Some(clone_expr) =
                    self.try_emit_self_path_clone_for_expected_value(&path.path, expected_ty)
                {
                    return clone_expr;
                }
                if self.should_coerce_self_path_to_deref(&path.path, expected_ty) {
                    if let Some(self_name) = self.current_self_path_override() {
                        return format!("{}.operator*()", self_name);
                    }
                    return "this->operator*()".to_string();
                }
                if self.should_coerce_self_path_to_deref_mut(&path.path, expected_ty) {
                    if let Some(self_name) = self.current_self_path_override() {
                        return format!("{}.operator*()", self_name);
                    }
                    return "this->operator*()".to_string();
                }
                if self.is_option_none_path(&path.path) {
                    if let Some(expected_cpp) = self.expected_option_cpp_type_for_none(expected_ty)
                    {
                        return format!("{}{{rusty::None}}", expected_cpp);
                    }
                    if let Some(inner_cpp) = self.option_ctor_inner_cpp_type(expected_ty) {
                        return format!("rusty::Option<{}>{{rusty::None}}", inner_cpp);
                    }
                }
                if let Some(expected_variant_ctor) =
                    self.try_emit_data_enum_unit_variant_path_with_expected(&path.path, expected_ty)
                {
                    return expected_variant_ctor;
                }
                if let Some(expected_ctor) =
                    self.try_emit_path_constructor_with_expected(path, expected_ty)
                {
                    return expected_ctor;
                }
                if let Some(expected_ty) = expected_ty {
                    if let Some(assoc_path) =
                        self.try_emit_assoc_path_with_expected(&path.path, expected_ty)
                    {
                        return assoc_path;
                    }
                }
                if self.expected_type_is_string_view(expected_ty) {
                    return self.emit_from_conversion_to_target(expr, "std::string_view");
                }
                if self.is_associated_const_value_path(&path.path) {
                    let inner = self.emit_expr_path_to_string(&path.path);
                    if self.associated_const_value_path_can_use_directly(&path.path, expected_ty) {
                        return inner;
                    }
                    return format!("rusty::clone({})", inner);
                }
                self.emit_expr_path_to_string(&path.path)
            }
            syn::Expr::If(if_expr) => self.emit_if_expr_to_string(if_expr, expected_ty),
            syn::Expr::Unsafe(unsafe_expr) => {
                if let Some(single_expr) = self.extract_single_expr_from_block(&unsafe_expr.block) {
                    self.emit_expr_to_string_with_expected(single_expr, expected_ty)
                } else if let Some(expr_str) =
                    self.block_expr_to_iife_string(&unsafe_expr.block, expected_ty)
                {
                    expr_str
                } else {
                    self.match_expr_unreachable_fallback_with_expected(expected_ty)
                }
            }
            syn::Expr::Block(block_expr) => {
                if let Some(expr_str) =
                    self.block_expr_to_iife_string(&block_expr.block, expected_ty)
                {
                    expr_str
                } else {
                    self.match_expr_unreachable_fallback().to_string()
                }
            }
            syn::Expr::Closure(closure) => {
                // Propagate expected return type to closure body emission.
                // The expected type (e.g. Result<T, E> for get_or_try_init) must be
                // pushed AFTER the closure's own output hint so it takes precedence
                // during Err/Ok qualification in the lambda body.
                // Create an inner context that inherits from self, then push the expected
                // return type hint. emit_closure_to_string_with_param_scopes internally
                // creates its own inner context which clears hints, so we push on the
                // Create an inner context (for mutability to push hints).
                let mut inner = self.new_inner_for_block();
                // A declared impl-Fn expected type carries the closure's PARAM
                // types (with_entries' FnOnce(&mut [Bucket<K,V>]) bound) —
                // hand them to the body emission so slice/sort routings gate
                // correctly on the params. The RETURN hint then comes from the
                // callable's OUTPUT (often none -> deduced), never from the
                // callable type itself.
                let mut expected_is_callable = false;
                let mut callable_output: Option<syn::Type> = None;
                if let Some(expected) = expected_ty
                    && let Some(param_types) = self.extract_callable_param_types_from_type(expected)
                    && param_types.len() == closure.inputs.len()
                    && !param_types.is_empty()
                    // SLICE-carrying signatures only — those are what the
                    // body can't recover; other shapes already emit
                    // correctly and their spellings must not churn.
                    && param_types.iter().any(|ty| {
                        matches!(
                            self.peel_reference_paren_group_type(ty),
                            syn::Type::Slice(_)
                        )
                    })
                {
                    expected_is_callable = true;
                    callable_output = self.extract_callable_return_type_from_type(expected);
                    *inner.pending_closure_param_types.borrow_mut() = Some(param_types);
                }
                // Push the expected return type as a syn::ReturnType for the closure body.
                let expected_body_ty = if expected_is_callable {
                    callable_output
                } else {
                    self.closure_expected_return_type_from_context(expected_ty)
                };
                let expected_rt = expected_body_ty
                    .map(|t| syn::ReturnType::Type(Default::default(), Box::new(t)));
                inner.emit_closure_to_string_with_param_scopes(
                    closure,
                    None,
                    None,
                    expected_rt.as_ref(),
                )
            }
            _ => self.emit_expr_to_string(expr),
        }
    }

    pub(super) fn try_emit_data_enum_variant_struct_literal_target(&self, path: &syn::Path) -> Option<String> {
        if path.segments.len() < 2 {
            return None;
        }
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        let mut start = 0;
        while start < segments.len()
            && matches!(segments[start].as_str(), "crate" | "self" | "super")
        {
            start += 1;
        }
        if start + 2 > segments.len() {
            return None;
        }
        let enum_idx = segments.len() - 2;
        let variant_idx = segments.len() - 1;
        if enum_idx < start {
            return None;
        }
        let enum_name = &segments[enum_idx];
        if !self.data_enum_types.contains(enum_name) {
            return None;
        }
        let variant_name = &segments[variant_idx];
        let enum_path = Self::path_without_last_segment(path)?;
        Some(self.data_enum_variant_struct_type_name(&enum_path, variant_name))
    }

    pub(super) fn try_emit_self_path_clone_for_expected_value(
        &self,
        path: &syn::Path,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        if !Self::path_is_simple_self(path) {
            return None;
        }
        let expected_ty = expected_ty?;
        if matches!(
            self.peel_paren_group_type(expected_ty),
            syn::Type::Reference(_)
        ) {
            return None;
        }
        if !self.type_is_current_struct_self_type(expected_ty) {
            return None;
        }
        let current_struct = self.current_struct.as_ref()?;
        let scoped_current = self.scoped_type_key(current_struct);
        let current_is_data_enum = self.data_enum_types.contains(current_struct)
            || self.data_enum_types.contains(&scoped_current);
        if !current_is_data_enum && !self.owner_impl_blocks_has_method_name(current_struct, "clone")
        {
            return None;
        }
        if let Some(self_name) = self.current_self_path_override() {
            return Some(format!("{}.clone()", self_name));
        }
        Some("this->clone()".to_string())
    }

    pub(super) fn try_emit_path_constructor_with_expected(
        &self,
        path_expr: &syn::ExprPath,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let expected_ty = expected_ty?;
        if path_expr.path.segments.len() == 1 {
            let local_name = path_expr.path.segments[0].ident.to_string();
            if self.lookup_local_binding_cpp_name(&local_name).is_some() {
                return None;
            }
        }
        if !self.expected_type_matches_struct_literal_path(expected_ty, &path_expr.path) {
            return None;
        }
        Some(format!("{}{{}}", self.map_type(expected_ty)))
    }

    pub(super) fn try_emit_data_enum_unit_variant_path_with_expected(
        &self,
        path: &syn::Path,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let expected_ty = expected_ty?;
        let variant_name = path.segments.last()?.ident.to_string();
        let enum_name = if path.segments.len() >= 2 {
            path.segments.iter().nth_back(1)?.ident.to_string()
        } else {
            // A BARE unit-variant reference (`return NoElements;` after its
            // `use MinMaxResult::{..}` import was dropped — ill-formed as a
            // C++ using-decl). Resolve the owner from the EXPECTED type: this
            // only fires when the expected enum actually declares a variant
            // with this name, so ordinary identifiers can't be hijacked.
            let expected_path = self.expected_type_path(expected_ty)?;
            let expected_last = expected_path.segments.last()?.ident.to_string();
            let owner_suffix = format!("::{}", expected_last);
            let expected_enum_declares_variant = self
                .data_enum_variants_by_enum
                .iter()
                .any(|(known_enum, variants)| {
                    (known_enum == &expected_last || known_enum.ends_with(&owner_suffix))
                        && variants.contains(&variant_name)
                });
            if !expected_enum_declares_variant {
                return None;
            }
            expected_last
        };
        let variant_key = format!("{}_{}", enum_name, variant_name);
        let is_known_data_enum_unit_variant = self.data_enum_unit_variants.contains(&variant_key);

        let expected_path = self.expected_type_path(expected_ty)?;
        let expected_last = expected_path.segments.last()?.ident.to_string();
        let expected_matches = if expected_last == enum_name {
            true
        } else if expected_last == "Self" {
            self.current_struct
                .as_ref()
                .and_then(|s| s.rsplit("::").next())
                .is_some_and(|tail| tail == enum_name)
        } else {
            false
        };
        if !expected_matches {
            return None;
        }

        if is_known_data_enum_unit_variant {
            let variant_ty = self.data_enum_variant_struct_type_name(expected_path, &variant_name);
            let variant_expr = format!("{}{{}}", variant_ty);
            return self
                .wrap_data_enum_variant_payload_with_expected(expected_ty, &variant_expr)
                .or(Some(variant_expr));
        }

        // Local C-like enums should remain plain enum-class paths (`E::Variant`)
        // and must not be rewritten to zero-arg constructor style.
        let is_local_c_like_enum = self.forward_emitted_c_like_enums.iter().any(|enum_key| {
            enum_key
                .rsplit("::")
                .next()
                .is_some_and(|tail| tail == enum_name)
        });
        if is_local_c_like_enum {
            return None;
        }

        // C-like enum variants (including built-ins like core::cmp::Ordering)
        // are value paths, not zero-arg constructor calls.
        if self.path_matches_c_like_enum_const(&enum_name, &variant_name) {
            return None;
        }
        // Fallback for externally-transpiled data enums that expose unit variants
        // as static zero-arg constructors (`Type::Variant()`), where local
        // data-enum metadata is unavailable in this compilation unit.
        let variant_is_camel_case = variant_name
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
            && !variant_name
                .chars()
                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_');
        if !variant_is_camel_case {
            return None;
        }

        let mut owner_cpp = self.map_type(expected_ty);
        while owner_cpp.ends_with('&') {
            owner_cpp.pop();
            owner_cpp = owner_cpp.trim_end().to_string();
        }
        if let Some(rest) = owner_cpp.strip_prefix("const ") {
            owner_cpp = rest.trim_start().to_string();
        }
        if owner_cpp.is_empty()
            || owner_cpp == "auto"
            || owner_cpp.contains("/* TODO")
            || type_string_has_auto_placeholder(&owner_cpp)
        {
            return None;
        }
        Some(format!(
            "{}::{}()",
            owner_cpp,
            escape_cpp_keyword(&variant_name)
        ))
    }

    pub(super) fn try_emit_data_enum_variant_call_with_expected(
        &self,
        call: &syn::ExprCall,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let expected_ty = expected_ty?;
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        let path = &path_expr.path;
        if self.is_option_some_path(path) || self.is_option_none_path(path) {
            return None;
        }
        let variant_name = path.segments.last()?.ident.to_string();

        // RUNTIME Either (a std::variant alias — no static Left/Right):
        // `Either::Left(x)` with a resolvable expected `Either<A, B>`
        // constructs through the runtime factory, both sides explicit
        // (`Either_Left<A, B>` carries both params, so a single payload
        // can't deduce the other side). at_most_one's
        // `ExactlyOneError::new_(Some(Either::Left([first, second])), ..)`
        // — the call-arg sibling of the match-level recovery in
        // emit_runtime_match_expr.
        if matches!(variant_name.as_str(), "Left" | "Right")
            && call.args.len() == 1
            && (path.segments.len() == 1
                || path
                    .segments
                    .iter()
                    .nth_back(1)
                    .is_some_and(|seg| seg.ident == "Either"))
            // Only for the RUNTIME Either (a dep/preamble alias). A crate that
            // DECLARES its own Either data enum (the either crate itself) must
            // keep the local data-enum construction path — the runtime factory
            // returns rusty::Either_Left, which doesn't convert to a local
            // struct-wrapper Either.
            && !self
                .local_declared_types
                .iter()
                .any(|decl| decl == "Either" || decl.ends_with("::Either"))
        {
            let expected_peeled = self.peel_reference_paren_group_type(expected_ty);
            if let syn::Type::Path(tp) = expected_peeled
                && tp.path.segments.last().is_some_and(|seg| seg.ident == "Either")
                && let Some(syn::PathArguments::AngleBracketed(ab)) =
                    tp.path.segments.last().map(|seg| &seg.arguments)
            {
                let arg_tys: Vec<&syn::Type> = ab
                    .args
                    .iter()
                    .filter_map(|arg| match arg {
                        syn::GenericArgument::Type(ty) => Some(ty),
                        _ => None,
                    })
                    .collect();
                if arg_tys.len() == 2 {
                    let a0 = self.map_type(arg_tys[0]);
                    let a1 = self.map_type(arg_tys[1]);
                    let viable = |t: &str| {
                        t != "auto"
                            && !t.contains("/* TODO")
                            && !type_string_has_auto_placeholder(t)
                    };
                    if viable(&a0) && viable(&a1) {
                        let payload_expected = if variant_name == "Left" {
                            arg_tys[0]
                        } else {
                            arg_tys[1]
                        };
                        let arg = self.emit_expr_to_string_with_expected_and_move_if_needed(
                            call.args.first()?,
                            Some(payload_expected),
                        );
                        return Some(format!(
                            "rusty::either::{}<{}, {}>({})",
                            variant_name, a0, a1, arg
                        ));
                    }
                }
            }
            // Expected doesn't resolve both Either sides (or isn't an Either
            // at all — a mis-threaded hint like the match scrutinee's
            // `Option<Self::Item>`): construct through the DEFERRED one-sided
            // factory. `rusty::either::Left(x)` carries the payload and
            // converts to Either<L, R2> at the point where C++ knows the
            // destination — always correct, unlike the member-style
            // `rusty::Either::Left(x)` fallthrough (no static Left on a
            // std::variant alias).
            let owner_is_either = path
                .segments
                .iter()
                .nth_back(1)
                .is_some_and(|seg| seg.ident == "Either");
            if owner_is_either {
                let arg = self.emit_expr_to_string_with_expected_and_move_if_needed(
                    call.args.first()?,
                    None,
                );
                return Some(format!("rusty::either::{}({})", variant_name, arg));
            }
        }

        if let Some(expected_enum) = self.expected_data_enum_name(expected_ty) {
            if !self.enum_has_variant_name(&expected_enum, &variant_name) {
                return None;
            }

            // If an explicit owner is present (`Enum::Variant(...)`), ensure it
            // matches the expected enum so we don't rewrite unrelated paths.
            if path.segments.len() >= 2 {
                let owner_name = path.segments.iter().nth_back(1)?.ident.to_string();
                let expected_tail = expected_enum.rsplit("::").next().unwrap_or(&expected_enum);
                let owner_matches = if owner_name == expected_enum {
                    true
                } else if owner_name == expected_tail {
                    true
                } else if owner_name == "Self" {
                    self.current_struct
                        .as_ref()
                        .and_then(|s| s.rsplit("::").next())
                        .is_some_and(|tail| tail == expected_enum)
                } else {
                    false
                };
                if !owner_matches {
                    return None;
                }
            }

            let expected_path = self.expected_type_path(expected_ty)?;
            let expected_substitutions = self.concrete_type_path_arg_substitutions(expected_path);
            let variant_ty = self.data_enum_variant_struct_type_name(expected_path, &variant_name);
            let expected_has_reference_payload = self
                .peel_reference_paren_group_type(expected_ty)
                .to_token_stream()
                .to_string()
                .contains('&')
                || self.expected_type_contains_mut_reference(Some(expected_ty));
            let args: Vec<String> = call
                .args
                .iter()
                .enumerate()
                .map(|(idx, arg)| {
                    let field_expected_ty = self
                        .lookup_data_enum_variant_arg_expected_type(
                            expected_path,
                            &expected_enum,
                            &variant_name,
                            idx,
                        )
                        .map(|ty| {
                            if let Some(substitutions) = expected_substitutions.as_ref() {
                                self.substitute_type_params_in_type(&ty, substitutions)
                            } else {
                                ty
                            }
                        });
                    if expected_has_reference_payload {
                        self.emit_expr_to_string_with_expected(arg, field_expected_ty.as_ref())
                    } else {
                        self.emit_expr_to_string_with_expected_and_move_if_needed(
                            arg,
                            field_expected_ty.as_ref(),
                        )
                    }
                })
                .collect();
            let args = self.wrap_data_enum_variant_tuple_constructor_args(
                &expected_enum,
                &variant_name,
                args,
            );
            let variant_expr = if args.is_empty() {
                format!("{}{{}}", variant_ty)
            } else {
                format!("{}{{{}}}", variant_ty, args.join(", "))
            };
            return self
                .wrap_data_enum_variant_payload_with_expected(expected_ty, &variant_expr)
                .or(Some(variant_expr));
        }

        if path.segments.len() >= 2 {
            let owner_name = path.segments.iter().nth_back(1)?.ident.to_string();
            let expected_path = self.expected_type_path(expected_ty)?;
            let expected_tail = expected_path.segments.last()?.ident.to_string();
            let variant_is_camel_case = variant_name
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
                && !variant_name
                    .chars()
                    .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_');
            if !variant_is_camel_case {
                return None;
            }
            let owner_matches = owner_name == expected_tail
                || (owner_name == "Self"
                    && self.current_struct.as_ref().is_some_and(|current| {
                        current.rsplit("::").next() == Some(expected_tail.as_str())
                    }));
            if owner_matches {
                let mut owner_cpp = self.map_type(expected_ty);
                while owner_cpp.ends_with('&') {
                    owner_cpp.pop();
                    owner_cpp = owner_cpp.trim_end().to_string();
                }
                if let Some(rest) = owner_cpp.strip_prefix("const ") {
                    owner_cpp = rest.trim_start().to_string();
                }
                if owner_cpp.is_empty()
                    || owner_cpp == "auto"
                    || owner_cpp.contains("/* TODO")
                    || type_string_has_auto_placeholder(&owner_cpp)
                    || owner_cpp == "rusty::Cow"
                    || owner_cpp.starts_with("rusty::Option<")
                    || owner_cpp.starts_with("rusty::Result<")
                {
                    return None;
                }
                let args: Vec<String> = call
                    .args
                    .iter()
                    .enumerate()
                    .map(|(idx, arg)| {
                        let arg_expected = self
                            .lookup_owner_method_arg_expected_type_from_owner_path(
                                Some(expected_path),
                                &expected_tail,
                                &variant_name,
                                idx,
                                Some(arg),
                            )
                            .or_else(|| {
                                if expected_tail == "Unexpected"
                                    && variant_name == "Str"
                                    && idx == 0
                                {
                                    Some(parse_quote!(std::string_view))
                                } else {
                                    None
                                }
                            });
                        self.emit_expr_to_string_with_expected_and_move_if_needed(
                            arg,
                            arg_expected.as_ref(),
                        )
                    })
                    .collect();
                return Some(format!(
                    "{}::{}({})",
                    owner_cpp,
                    escape_cpp_keyword(&variant_name),
                    args.join(", ")
                ));
            }
            // Other explicitly-owned variant calls can be handled by normal path
            // lowering and constructor rules. Avoid re-binding them to unrelated
            // expected types.
            return None;
        }
        // Fallback for externally-transpiled data enums where local enum metadata
        // is unavailable in this compilation unit. Prefer static variant
        // constructors on the expected owner type (`Type::Variant(...)`).
        let expected_path = self.expected_type_path(expected_ty)?;
        let mut owner_tail = expected_path.segments.last()?.ident.to_string();
        if owner_tail == "Self" {
            owner_tail = self
                .current_struct
                .as_ref()
                .and_then(|name| name.rsplit("::").next().map(ToString::to_string))
                .unwrap_or(owner_tail);
        }
        let mapped_owner = self.map_type(expected_ty);
        let owner_maps_to_bare_ident = mapped_owner == owner_tail
            && !mapped_owner.contains("::")
            && !mapped_owner.contains('<');
        if owner_maps_to_bare_ident
            && !self.current_scope_declares_type_name(&owner_tail)
            && !self.data_enum_name_matches(&owner_tail)
        {
            // Expected owner is an unresolved placeholder-like identifier
            // (for example a generic from another function signature). Avoid
            // synthesizing `Owner::Variant(...)` from such names.
            return None;
        }
        if owner_tail == variant_name {
            // Tuple-struct constructors and same-name local value constructors
            // should stay as direct constructor calls (`Type(...)`), not
            // re-bound to `Type::Type(...)`.
            return None;
        }
        if self.is_type_param_in_scope(&owner_tail) {
            // Generic owner placeholders (e.g., `T`) do not expose static
            // variant-constructor surfaces in C++.
            return None;
        }
        let owner_is_known_local_type = self.local_declared_types.contains(&owner_tail)
            || self.local_declared_types.iter().any(|decl| {
                decl.rsplit("::")
                    .next()
                    .is_some_and(|tail| tail == owner_tail)
            });
        if owner_is_known_local_type && !self.data_enum_types.contains(&owner_tail) {
            return None;
        }
        if matches!(owner_tail.as_str(), "Option" | "Result") {
            return None;
        }
        let variant_is_camel_case = variant_name
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
            && !variant_name
                .chars()
                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_');
        if !variant_is_camel_case {
            return None;
        }

        let mut owner_cpp = self.map_type(expected_ty);
        while owner_cpp.ends_with('&') {
            owner_cpp.pop();
            owner_cpp = owner_cpp.trim_end().to_string();
        }
        if let Some(rest) = owner_cpp.strip_prefix("const ") {
            owner_cpp = rest.trim_start().to_string();
        }
        if owner_cpp.is_empty()
            || owner_cpp == "auto"
            || owner_cpp.contains("/* TODO")
            || type_string_has_auto_placeholder(&owner_cpp)
            || owner_cpp.starts_with("rusty::Option<")
            || owner_cpp.starts_with("rusty::Result<")
        {
            return None;
        }

        let args: Vec<String> = call
            .args
            .iter()
            .map(|arg| self.emit_expr_maybe_move(arg))
            .collect();
        if args.is_empty() {
            Some(format!(
                "{}::{}()",
                owner_cpp,
                escape_cpp_keyword(&variant_name)
            ))
        } else {
            Some(format!(
                "{}::{}({})",
                owner_cpp,
                escape_cpp_keyword(&variant_name),
                args.join(", ")
            ))
        }
    }

    /// Forward element inference for a bare polymorphic-collection constructor.
    /// The consuming context type `forward_ty` (an explicit expected type, or a
    /// fn return-type hint) is a container/iterator `Other<E..>` that shares its
    /// element with the ctor's collection but has a DIFFERENT owner (so the
    /// ordinary expected-type path bailed) — e.g. `VecDeque::new()` in a fn that
    /// returns `VecDequeIntoIter<E>`. Extract `E..` and synthesize `Coll<E..>`,
    /// then reuse the proven expected-type emission so the ctor gets its args.
    pub(super) fn try_emit_collection_ctor_with_forward_element(
        &self,
        call: &syn::ExprCall,
        forward_ty: &syn::Type,
    ) -> Option<String> {
        let syn::Expr::Path(func_path_expr) = self.peel_paren_group_expr(call.func.as_ref()) else {
            return None;
        };
        let func_path = &func_path_expr.path;
        let segs = &func_path.segments;
        if segs.len() < 2 {
            return None;
        }
        let ctor = segs.last()?;
        if !matches!(
            ctor.ident.to_string().as_str(),
            "new" | "new_" | "new_in" | "with_capacity" | "with_capacity_in" | "from"
                | "from_iter"
        ) || !matches!(ctor.arguments, syn::PathArguments::None)
        {
            return None;
        }
        let coll_seg = &segs[segs.len() - 2];
        let coll_name = coll_seg.ident.to_string();
        if !matches!(coll_seg.arguments, syn::PathArguments::None)
            || !Self::is_polymorphic_collection_name(&coll_name)
        {
            return None;
        }
        // Peel generic-ALIAS layers off the forward type before comparing
        // owners: an alias's own args are its PARAMS, not the collection's
        // element (indexmap `Entries<K, V> = Vec<Bucket<K, V>>` — taking `K`
        // positionally would build `Vec<K>`). An alias of this SAME collection
        // then hits the owner-match bail below and stays with the ordinary
        // expected-type paths (which resolve the alias with proper arg
        // substitution); an alias of a DIFFERENT container carries its
        // target's REAL args forward. Cap the peel so a malformed alias cycle
        // can't loop.
        let mut fwd_ty = self.peel_reference_paren_group_type(forward_ty).clone();
        for _ in 0..4 {
            let Some(resolved) = self.resolve_type_alias_once(&fwd_ty) else {
                break;
            };
            fwd_ty = self.peel_reference_paren_group_type(&resolved).clone();
        }
        let syn::Type::Path(fwd_tp) = &fwd_ty else {
            return None;
        };
        let fwd_last = fwd_tp.path.segments.last()?;
        // Same owner → the ordinary expected-type path already handles it.
        if fwd_last.ident == coll_name {
            return None;
        }
        let syn::PathArguments::AngleBracketed(fwd_args) = &fwd_last.arguments else {
            return None;
        };
        let fwd_type_args: Vec<syn::Type> = fwd_args
            .args
            .iter()
            .filter_map(|a| match a {
                syn::GenericArgument::Type(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        let is_map = matches!(coll_name.as_str(), "HashMap" | "BTreeMap" | "IndexMap");
        let needed = if is_map { 2 } else { 1 };
        if fwd_type_args.len() < needed {
            return None;
        }
        let elems: Vec<syn::Type> = fwd_type_args.into_iter().take(needed).collect();
        // Synthesize `Coll<elems..>`, preserving the ctor's own owner path.
        let mut owner_path = func_path.clone();
        owner_path.segments = func_path
            .segments
            .iter()
            .take(segs.len() - 1)
            .cloned()
            .collect();
        if let Some(last) = owner_path.segments.last_mut() {
            let mut generic = syn::punctuated::Punctuated::new();
            for e in elems {
                generic.push(syn::GenericArgument::Type(e));
            }
            last.arguments =
                syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    colon2_token: None,
                    lt_token: Default::default(),
                    args: generic,
                    gt_token: Default::default(),
                });
        }
        let coll_expected = syn::Type::Path(syn::TypePath {
            qself: None,
            path: owner_path,
        });
        self.try_emit_associated_call_with_expected_type(call, &coll_expected)
    }

    pub(super) fn try_emit_associated_call_with_expected_type(
        &self,
        call: &syn::ExprCall,
        expected_ty: &syn::Type,
    ) -> Option<String> {
        let syn::Expr::Path(func_path_expr) = call.func.as_ref() else {
            return None;
        };
        let func_path = &func_path_expr.path;
        if func_path.segments.len() < 2 {
            return None;
        }
        let owner_seg = func_path.segments.iter().nth_back(1)?;
        let expected_last = self.expected_type_last_ident(expected_ty)?;
        if owner_seg.ident != expected_last.as_str() {
            return None;
        }
        let mut owner_cpp = self.map_type(expected_ty);
        if !owner_cpp.contains('<') {
            return None;
        }
        if owner_cpp.contains("<auto") {
            // #79: an expected `Self` that resolved through the impl's self
            // type carries the impl's UNBOUND params (`IndexedEntry<auto,
            // auto>` for indexmap's `Entry::Occupied(e) =>
            // IndexedEntry::from(e)`). The From-conversion's declared
            // parameter shares those params — unify it against the
            // argument's type to recover them; bail only if that fails.
            if !call.args.is_empty()
                && let Some(recovered) =
                    self.recover_auto_owner_from_arg_signature(call, func_path)
            {
                owner_cpp = recovered;
            } else {
                return None;
            }
        }
        let rust_method_name = func_path.segments.last()?.ident.to_string();
        // The expected type describes the call's RESULT; spelling it as the
        // OWNER is only sound when the method returns Self with the owner's
        // own params (new/default). A staging constructor whose declared
        // return re-parameterizes the owner (`impl<T> Owned<T> { fn
        // new_uninit() -> Owned<MaybeUninit<T>, T> }`) would get
        // T=MaybeUninit<PP> — keep the source turbofish (the emitted class
        // template's defaulted params complete it).
        if let Some(ret_ty) = self.lookup_owner_method_return_type_for_template_inference(
            &owner_seg.ident.to_string(),
            &rust_method_name,
        ) && !self.assoc_return_owner_args_are_plain_params(
            &owner_seg.ident.to_string(),
            ret_ty,
        ) {
            return None;
        }
        if self.data_enum_types.contains(&expected_last)
            && self
                .data_enum_variants_by_enum
                .get(&expected_last)
                .is_some_and(|variants| variants.contains(&rust_method_name))
        {
            return None;
        }
        // `Rc::clone(&one)` (and friends) must NOT lower as
        // `Rc<int32_t>::clone(one)` — Rc/Arc/Box/etc. expose `clone()` as a
        // *member* in their C++ ports, so static-style dispatch fails to
        // compile. Defer to the Clone-UFCS branch in
        // `try_emit_known_trait_ufcs_call` (called later in the call
        // pipeline) which routes to `rusty::clone(arg)`.
        if rust_method_name == "clone" {
            const KNOWN_CLONE_OWNERS: &[&str] = &[
                "Rc", "Arc", "Weak", "Box", "RefCell", "Cell", "Mutex", "RwLock",
            ];
            let owner_ident = owner_seg.ident.to_string();
            let owner_clone_takes_receiver = self
                .lookup_owner_method_has_receiver(&owner_ident, "clone")
                .unwrap_or_else(|| KNOWN_CLONE_OWNERS.contains(&owner_ident.as_str()));
            if owner_clone_takes_receiver {
                return None;
            }
        }
        let method = self.mapped_assoc_method_name_for_expected_owner(func_path, &owner_cpp)?;
        if self.owner_cpp_has_unusable_template_args(&owner_cpp)
            && matches!(owner_seg.arguments, syn::PathArguments::None)
        {
            let recovered_func =
                self.emit_call_func_with_owner_template_recovery(call, Some(expected_ty));
            if let Some((recovered_owner, recovered_method)) = recovered_func.rsplit_once("::") {
                let method_tail = method.split('<').next().unwrap_or(method.as_str());
                let recovered_method_tail = recovered_method
                    .split('<')
                    .next()
                    .unwrap_or(recovered_method);
                let normalize_owner_base = |owner: &str| {
                    owner
                        .trim_start_matches("typename ")
                        .trim()
                        .trim_start_matches("::")
                        .split('<')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string()
                };
                let owner_base = normalize_owner_base(&owner_cpp);
                let recovered_owner_base = normalize_owner_base(recovered_owner);
                if method_tail == recovered_method_tail
                    && owner_base == recovered_owner_base
                    && !self.owner_cpp_has_unusable_template_args(recovered_owner)
                {
                    owner_cpp = recovered_owner.to_string();
                }
            }
        }
        // Merge substitutions from both the call's explicit turbofish
        // (`SmallVec::<[u32;2]>::from(...)`) and the surrounding expected
        // type. The turbofish path was previously skipped here, leaving
        // `A::Item` unsubstituted in argument expected-type lookups and
        // causing slice/array element types to leak as `typename A::Item`.
        let mut merged_substitutions = self
            .function_call_type_arg_substitutions(call)
            .unwrap_or_default();
        if let Some(expected_substitutions) =
            self.call_owner_type_arg_substitutions_from_expected_type(call, Some(expected_ty))
        {
            for (k, v) in expected_substitutions {
                merged_substitutions.entry(k).or_insert(v);
            }
        }
        let expected_substitutions = (!merged_substitutions.is_empty()).then_some(merged_substitutions);
        let args: Vec<String> = call
            .args
            .iter()
            .enumerate()
            .map(|(idx, arg)| {
                let style = self
                    .lookup_function_arg_pass_style(call.func.as_ref(), idx)
                    .or_else(|| self.lookup_method_arg_pass_style(&rust_method_name, idx))
                    .or_else(|| self.associated_receiver_style_first_arg_pass_style(call, idx));
                let mut arg_expected_ty = self
                    .lookup_function_arg_expected_type_for_call(
                        call,
                        idx,
                        expected_substitutions.as_ref(),
                    )
                    .or_else(|| {
                        let fallback = self.lookup_associated_call_arg_expected_type_fallback(
                            call,
                            idx,
                            Some(arg),
                        )?;
                        if let Some(substitutions) = expected_substitutions.as_ref() {
                            return Some(
                                self.substitute_type_params_in_type(&fallback, substitutions),
                            );
                        }
                        Some(fallback)
                    });
                let expected_needs_owner_recovery =
                    arg_expected_ty.as_ref().is_some_and(|expected| {
                        self.type_contains_infer(expected)
                            || self.type_contains_in_scope_type_param(expected)
                            || self.type_contains_unresolved_placeholder_like(expected)
                            || self.type_contains_unbound_single_letter_generic(expected)
                            || matches!(
                                self.peel_reference_paren_group_type(expected),
                                syn::Type::Path(tp)
                                    if tp.qself.is_none()
                                        && tp.path.segments.len() == 1
                                        && tp.path.segments[0]
                                            .ident
                                            .to_string()
                                            .chars()
                                            .next()
                                            .is_some_and(|c| c.is_ascii_uppercase())
                            )
                    });
                let arg_is_closure =
                    matches!(self.peel_paren_group_expr(arg), syn::Expr::Closure(_));
                let expected_closure_return_unknown = arg_is_closure
                    && arg_expected_ty.as_ref().is_some_and(|expected| {
                        self.extract_callable_return_type_from_type(expected)
                            .is_none()
                    });
                if (arg_expected_ty.is_none()
                    || expected_needs_owner_recovery
                    || expected_closure_return_unknown)
                    && let Some(fallback) = self
                        .infer_associated_call_arg_expected_type_from_call_expected_owner(
                            call,
                            Some(expected_ty),
                            idx,
                        )
                {
                    let fallback = if let Some(substitutions) = expected_substitutions.as_ref() {
                        self.substitute_type_params_in_type(&fallback, substitutions)
                    } else {
                        fallback
                    };
                    arg_expected_ty = Some(fallback);
                }
                if arg_expected_ty.is_none()
                    && owner_cpp.starts_with("rusty::Vec<")
                    && rust_method_name == "from_iter"
                    && idx == 0
                    && let Some(item_ty) =
                        self.expected_vec_element_type(Some(expected_ty)).cloned()
                {
                    let iter_expected: syn::Type = parse_quote!(impl Iterator<Item = #item_ty>);
                    arg_expected_ty = Some(iter_expected);
                }
                let arg_cpp = self.emit_call_arg_with_pass_style(
                    arg,
                    style,
                    arg_expected_ty.as_ref(),
                    false,
                    None,
                );
                self.coerce_slice_expected_arg_cpp(arg_cpp, arg_expected_ty.as_ref())
            })
            .collect();
        if owner_cpp.starts_with("SmallVec<")
            && method == "from"
            && args.len() == 1
            && let Some(array_cpp) = Self::extract_smallvec_owner_array_cpp_from_func(&owner_cpp)
            && matches!(
                self.peel_paren_group_expr(&call.args[0]),
                syn::Expr::Path(path_expr) if path_expr.path.segments.len() == 1
            )
        {
            let arg = self.emit_expr_maybe_move(&call.args[0]);
            let coerced = format!(
                "rusty::Vec<rusty::detail::associated_item_t<{}>>::from_iter({})",
                array_cpp, arg
            );
            return Some(format!("{}::{}({})", owner_cpp, method, coerced));
        }
        if owner_cpp.starts_with("rusty::HashMap")
            && matches!(method.as_str(), "new" | "new_")
            && args.is_empty()
        {
            return Some(format!("{}()", owner_cpp));
        }
        if owner_cpp.starts_with("rusty::Vec<") && method == "from_iter" && args.len() == 1 {
            // `Vec::from_iter(X)` is a Vec in Rust, but `collect_range` yields a
            // bare std::vector (losing rusty::Vec methods like sort_by). Re-wrap
            // it into a rusty::Vec; CTAD deduces the element from the collected
            // std::vector<E>. `Vec::from_iter` can't take the std::vector
            // directly (SpecFromIter drives it through `.next()`).
            return Some(format!("rusty::Vec(rusty::collect_range({}))", args[0]));
        }
        // A static/associated call's owner must be the BARE class type. An unsized
        // owner (e.g. indexmap's `Slice<T>`, only ever held by reference) maps to
        // `const Slice<T>&`; `const Slice<T>&::from_slice(..)` is ill-formed. Strip
        // the leading `const` and trailing `&`/`*` for the `Owner::method` spelling.
        let bare_owner = owner_cpp
            .trim()
            .trim_start_matches("const ")
            .trim_end_matches(|c| matches!(c, '&' | '*' | ' '))
            .trim();
        // The owner derived from the expected type can be a BARE, ambiguous leaf:
        // indexmap has both `map::slice::Slice<K,V>` and `set::slice::Slice<T>`, so a
        // bare `Slice<K,V>` fails to resolve in a scope that isn't a descendant of the
        // declaring module ("no template named 'Slice'; did you mean 'set::Slice'?").
        // When the func path itself carries a qualified owner
        // (`crate::map::Slice::from_slice`), render that owner and splice its
        // qualification onto the expected-type's turbofish args, preserving the
        // disambiguation the Rust source had.
        let bare_owner_base = bare_owner.split('<').next().unwrap_or("").trim();
        if !bare_owner_base.trim_start_matches("::").contains("::")
            && func_path.segments.len() >= 3
        {
            let keep = func_path.segments.len() - 1;
            let mut owner_only = func_path.clone();
            owner_only.segments = func_path.segments.iter().take(keep).cloned().collect();
            let qualified = self.emit_path_to_string(&owner_only);
            let qualified_base = qualified.split('<').next().unwrap_or("").trim();
            if qualified_base.trim_start_matches("::").contains("::")
                && qualified_base.rsplit("::").next()
                    == Some(bare_owner_base.trim_start_matches("::"))
            {
                let args_suffix = &bare_owner[bare_owner_base.len()..];
                return Some(format!(
                    "{}{}::{}({})",
                    qualified_base,
                    args_suffix,
                    method,
                    args.join(", ")
                ));
            }
        }
        Some(format!("{}::{}({})", bare_owner, method, args.join(", ")))
    }

    pub(super) fn try_emit_omitted_assoc_static_call_with_arg_decltype(
        &self,
        call: &syn::ExprCall,
    ) -> Option<String> {
        let syn::Expr::Path(func_path_expr) = call.func.as_ref() else {
            return None;
        };
        let func_path = &func_path_expr.path;
        if func_path.segments.len() != 2 || call.args.len() != 1 {
            return None;
        }
        let owner_seg = func_path.segments.iter().nth_back(1)?;
        let method_seg = func_path.segments.last()?;
        if !matches!(owner_seg.arguments, syn::PathArguments::None)
            || !matches!(method_seg.arguments, syn::PathArguments::None)
        {
            return None;
        }
        if !matches!(method_seg.ident.to_string().as_str(), "new" | "new_") {
            return None;
        }

        let owner = owner_seg.ident.to_string();
        let method_cpp = escape_cpp_keyword(&method_seg.ident.to_string());
        if owner == "TokenSlice" {
            let arg_decl = self.emit_expr_to_string(&call.args[0]);
            let arg = self.emit_expr_maybe_move(&call.args[0]);
            return Some(format!(
                "{}<std::remove_cvref_t<decltype(*(({}).begin()))>>::{}({})",
                escape_cpp_keyword(&owner),
                arg_decl,
                method_cpp,
                arg
            ));
        }
        if owner == "LocatingSlice" {
            let arg_decl = self.emit_expr_to_string(&call.args[0]);
            let arg = self.emit_expr_maybe_move(&call.args[0]);
            return Some(format!(
                "{}<std::remove_cvref_t<decltype(({}))>>::{}({})",
                escape_cpp_keyword(&owner),
                arg_decl,
                method_cpp,
                arg
            ));
        }
        if matches!(
            owner.as_str(),
            "SeqAccessDeserializer" | "MapAccessDeserializer"
        ) {
            let arg = self.emit_expr_maybe_move(&call.args[0]);
            return Some(format!(
                "{}<std::remove_cvref_t<decltype(({}))>>::{}({})",
                escape_cpp_keyword(&owner),
                arg,
                method_cpp,
                arg
            ));
        }
        if owner.ends_with("Deserializer") {
            return None;
        }
        let scoped_owner = if self.module_stack.is_empty() {
            owner.clone()
        } else {
            format!("{}::{}", self.module_stack.join("::"), owner)
        };
        let owner_key = self.lookup_declared_type_key_for_base(&scoped_owner, &owner)?;
        let owner_kinds = self.declared_type_param_kinds.get(&owner_key)?;
        if owner_kinds.as_slice() != [GenericParamKind::Type] {
            return None;
        }

        let arg = self.emit_expr_maybe_move(&call.args[0]);
        Some(format!(
            "{}<std::remove_cvref_t<decltype({})>>::{}({})",
            escape_cpp_keyword(&owner),
            arg,
            method_cpp,
            arg
        ))
    }

    pub(super) fn try_emit_array_repeat_as_u8(&self, expr: &syn::Expr) -> Option<String> {
        let expr = self.peel_paren_group_expr(expr);
        match expr {
            syn::Expr::Reference(reference) => self.try_emit_array_repeat_as_u8(&reference.expr),
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::ByteStr(bs),
                ..
            }) => {
                let bytes = bs.value();
                let elems: Vec<String> = bytes
                    .iter()
                    .map(|b| format!("static_cast<uint8_t>({})", b))
                    .collect();
                Some(format!(
                    "rusty::as_u8_slice(rusty::addr_of_temp(std::array<uint8_t, {}>{{{{ {} }}}}))",
                    bytes.len(),
                    elems.join(", ")
                ))
            }
            syn::Expr::Array(array_expr) => {
                let elems: Vec<String> = array_expr
                    .elems
                    .iter()
                    .map(|elem| format!("static_cast<uint8_t>({})", self.emit_expr_to_string(elem)))
                    .collect();
                Some(format!(
                    "rusty::as_u8_slice(rusty::addr_of_temp(std::array<uint8_t, {}>{{{{ {} }}}}))",
                    array_expr.elems.len(),
                    elems.join(", ")
                ))
            }
            syn::Expr::Repeat(repeat) => {
                let value = self.emit_expr_to_string(&repeat.expr);
                let len = self.emit_expr_to_string(&repeat.len);
                Some(format!(
                    "rusty::array_repeat(static_cast<uint8_t>({}), {})",
                    value, len
                ))
            }
            syn::Expr::Call(call) => {
                let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(call.func.as_ref())
                else {
                    return None;
                };
                let joined = path_expr
                    .path
                    .segments
                    .iter()
                    .map(|seg| seg.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                if !matches!(joined.as_str(), "rusty::array_repeat" | "array_repeat")
                    || call.args.len() != 2
                {
                    return None;
                }
                let value = self.emit_expr_to_string(&call.args[0]);
                let len = self.emit_expr_to_string(&call.args[1]);
                Some(format!(
                    "rusty::array_repeat(static_cast<uint8_t>({}), {})",
                    value, len
                ))
            }
            _ => None,
        }
    }

    pub(super) fn try_emit_array_element_zero_arg_associated_ctor_path(
        &self,
        path: &syn::Path,
    ) -> Option<String> {
        if path.segments.len() < 2 {
            return None;
        }
        if self.is_associated_const_value_path(path) {
            return None;
        }
        let variant = path.segments.last()?.ident.to_string();
        let owner = path.segments.iter().nth_back(1)?.ident.to_string();
        let owner_looks_like_type = owner
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase() || ch == '_');
        let variant_looks_like_ctor = variant
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase() || ch == '_');
        if !owner_looks_like_type || !variant_looks_like_ctor {
            return None;
        }
        if matches!(owner.as_str(), "Option" | "Result")
            || self.path_matches_c_like_enum_const(&owner, &variant)
        {
            return None;
        }
        if matches!(
            self.lookup_owner_method_has_receiver(&owner, &variant),
            Some(true)
        ) {
            return None;
        }
        Some(format!("{}()", self.emit_path_to_string(path)))
    }

    pub(super) fn try_emit_associated_bytes_constructor_call(
        &self,
        call: &syn::ExprCall,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        if call.args.len() != 1 || path_expr.path.segments.len() < 2 {
            return None;
        }
        let method_name = path_expr.path.segments.last()?.ident.to_string();
        if !matches!(method_name.as_str(), "Bytes" | "BorrowedBytes" | "ByteBuf") {
            return None;
        }
        let owner_name = path_expr
            .path
            .segments
            .iter()
            .nth_back(1)?
            .ident
            .to_string();
        let owner_looks_like_type = owner_name
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase() || ch == '_');
        if !owner_looks_like_type {
            return None;
        }
        let arg = self.try_emit_array_repeat_as_u8(&call.args[0])?;
        let mut func = self.emit_call_func_with_owner_template_recovery(call, expected_ty);
        func = self.rewrite_seed_ctor_path_string(&func);
        func = self.maybe_defer_static_owner_lookup_for_path_call(call, func);
        Some(format!("{}({})", func, arg))
    }

    pub(super) fn try_emit_reference_array_literal_with_expected_span(
        &self,
        reference: &syn::ExprReference,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let expected_ty = expected_ty?;
        let syn::Type::Reference(expected_ref) = expected_ty else {
            return None;
        };
        let syn::Type::Slice(slice_ty) = expected_ref.elem.as_ref() else {
            return None;
        };
        let ref_inner = self.peel_paren_group_expr(&reference.expr);
        let syn::Expr::Array(array_expr) = ref_inner else {
            return None;
        };

        let elem_ty = slice_ty.elem.as_ref();
        let elem_cpp = self.map_type(elem_ty);
        let is_mut = reference.mutability.is_some() || expected_ref.mutability.is_some();

        // If the element type references out-of-scope type parameters (e.g., T from
        // a callee's template), fall back to auto-deduced storage to avoid unresolved types.
        if self.mapped_type_has_out_of_scope_type_params(&elem_cpp) {
            let storage_decl = if is_mut {
                "auto _slice_ref_tmp".to_string()
            } else {
                "const auto _slice_ref_tmp".to_string()
            };
            let elems: Vec<String> = array_expr
                .elems
                .iter()
                .map(|elem| self.emit_expr_to_string(elem))
                .collect();
            let span_decl = if is_mut { "auto" } else { "const auto" };
            let capture = if self.block_depth == 0 { "[]" } else { "[&]" };
            return Some(format!(
                "{}() {{ static {} = std::array{{{}}}; {} _span = std::span(_slice_ref_tmp); return _span; }}()",
                capture,
                storage_decl,
                elems.join(", "),
                span_decl,
            ));
        }

        let span_cpp = if is_mut {
            format!("std::span<{}>", elem_cpp)
        } else {
            format!("std::span<const {}>", elem_cpp)
        };
        let array_cpp = format!("std::array<{}, {}>", elem_cpp, array_expr.elems.len());
        let storage_decl = if is_mut {
            format!("{} _slice_ref_tmp", array_cpp)
        } else {
            format!("const {} _slice_ref_tmp", array_cpp)
        };
        let elems: Vec<String> = array_expr
            .elems
            .iter()
            .map(|elem| self.emit_expr_to_string_with_expected(elem, Some(elem_ty)))
            .collect();
        // Use [] (no capture) at struct/namespace scope to avoid
        // "non-local lambda cannot have capture-default" errors.
        let capture = if self.block_depth == 0 { "[]" } else { "[&]" };
        Some(format!(
            "{}() -> {} {{ static {} = {{{}}}; return {}(_slice_ref_tmp); }}()",
            capture,
            span_cpp,
            storage_decl,
            elems.join(", "),
            span_cpp
        ))
    }

    pub(super) fn try_emit_reference_expr_with_expected_span_storage(
        &self,
        reference: &syn::ExprReference,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let expected_ty = expected_ty?;
        let syn::Type::Reference(expected_ref) = expected_ty else {
            return None;
        };
        let syn::Type::Slice(slice_ty) = expected_ref.elem.as_ref() else {
            return None;
        };
        if self.is_stable_reference_lvalue_expr(&reference.expr) {
            return None;
        }

        let elem_ty = slice_ty.elem.as_ref();
        let elem_cpp = self.map_type(elem_ty);
        let is_mut = reference.mutability.is_some() || expected_ref.mutability.is_some();

        // If expected element type still leaks out-of-scope type params
        // (for example `A::Item` outside generic scope), avoid emitting an
        // explicit span return type that will not compile in concrete call sites.
        if self.mapped_type_has_out_of_scope_type_params(&elem_cpp) {
            let storage_decl = if is_mut {
                "auto _slice_ref_tmp".to_string()
            } else {
                "const auto _slice_ref_tmp".to_string()
            };
            let inner = self.emit_expr_to_string_with_expected(
                &reference.expr,
                Some(expected_ref.elem.as_ref()),
            );
            let span_decl = if is_mut { "auto" } else { "const auto" };
            let capture = if self.block_depth == 0 { "[]" } else { "[&]" };
            return Some(format!(
                "{}() {{ static {} = {}; {} _span = std::span(_slice_ref_tmp); return _span; }}()",
                capture, storage_decl, inner, span_decl
            ));
        }

        let span_cpp = if is_mut {
            format!("std::span<{}>", elem_cpp)
        } else {
            format!("std::span<const {}>", elem_cpp)
        };
        let storage_decl = if is_mut {
            "auto _slice_ref_tmp".to_string()
        } else {
            "const auto _slice_ref_tmp".to_string()
        };
        let inner = self
            .emit_expr_to_string_with_expected(&reference.expr, Some(expected_ref.elem.as_ref()));
        let capture = if self.block_depth == 0 { "[]" } else { "[&]" };
        Some(format!(
            "{}() -> {} {{ static {} = {}; return {}(_slice_ref_tmp); }}()",
            capture, span_cpp, storage_decl, inner, span_cpp
        ))
    }

    pub(super) fn emit_binary_expr_to_string_with_expected(
        &self,
        bin: &syn::ExprBinary,
        expected_ty: Option<&syn::Type>,
    ) -> String {
        if Self::is_compound_assign_binop(&bin.op) {
            let left = self.autoderef_compound_assign_lhs_if_needed(
                &bin.left,
                self.emit_expr_to_string(&bin.left),
            );
            let op = self.emit_binop(&bin.op);
            let right = self.emit_expr_to_string(&bin.right);
            let assign_expr = format!("{} {} {}", left, op, right);
            // Rust assignment expressions always evaluate to unit `()`.
            return format!(
                "[&]() {{ static_cast<void>({}); return std::make_tuple(); }}()",
                assign_expr
            );
        }
        match &bin.op {
            // Logical operators always require boolean operands.
            // Thread `bool` into both sides so nested match fallbacks emit typed
            // unreachable lambdas compatible with std::visit return unification.
            syn::BinOp::And(_) | syn::BinOp::Or(_) => {
                let bool_ty: syn::Type = parse_quote!(bool);
                let left = self.autoderef_binary_value_operand_if_needed(
                    &bin.left,
                    &bin.op,
                    self.emit_binary_operand_expr_with_expected(&bin.left, Some(&bool_ty)),
                );
                let op = self.emit_binop(&bin.op);
                let right = self.autoderef_binary_value_operand_if_needed(
                    &bin.right,
                    &bin.op,
                    self.emit_binary_operand_expr_with_expected(&bin.right, Some(&bool_ty)),
                );
                format!("{} {} {}", left, op, right)
            }
            syn::BinOp::Eq(_) | syn::BinOp::Ne(_) => {
                let left_ty = self
                    .infer_simple_expr_type(&bin.left)
                    .or_else(|| self.infer_owner_type_from_constructor_expr(&bin.left));
                let right_ty = self
                    .infer_simple_expr_type(&bin.right)
                    .or_else(|| self.infer_owner_type_from_constructor_expr(&bin.right));
                // Threading each side's type as the OTHER's expected type
                // string-coerces a LOCAL type compared against a str const
                // (`tag == Tag::BOOL` became to_string_view(tag) == ...),
                // bypassing the type's own PartialEq<str> operator. A local
                // non-string operand facing a string-like one keeps its own
                // spelling — the emitted operator==(string_view) handles it.
                let stringish = |ty: &Option<syn::Type>| -> bool {
                    ty.as_ref().is_some_and(|t| self.type_is_string_view_like(t))
                };
                let left_plain = stringish(&right_ty) && !stringish(&left_ty);
                let right_plain = stringish(&left_ty) && !stringish(&right_ty);
                let left_expected = if left_plain { None } else { right_ty.as_ref() };
                let right_expected = if right_plain { None } else { left_ty.as_ref() };
                // The plain side also skips the autoderef wrap: a Deref-to-
                // slice type (Tag → [u8]) would otherwise decay to a span and
                // lose its own operator==(string_view).
                let left_emitted =
                    self.emit_binary_operand_expr_with_expected(&bin.left, left_expected);
                let left = if left_plain {
                    left_emitted
                } else {
                    self.autoderef_binary_value_operand_if_needed(
                        &bin.left,
                        &bin.op,
                        left_emitted,
                    )
                };
                let op = self.emit_binop(&bin.op);
                let right_emitted =
                    self.emit_binary_operand_expr_with_expected(&bin.right, right_expected);
                let right = if right_plain {
                    right_emitted
                } else {
                    self.autoderef_binary_value_operand_if_needed(
                        &bin.right,
                        &bin.op,
                        right_emitted,
                    )
                };
                format!("{} {} {}", left, op, right)
            }
            _ => {
                let left_expected = if expected_ty.is_some()
                    && matches!(
                        bin.op,
                        syn::BinOp::Add(_)
                            | syn::BinOp::Sub(_)
                            | syn::BinOp::Mul(_)
                            | syn::BinOp::Div(_)
                            | syn::BinOp::Rem(_)
                            | syn::BinOp::BitXor(_)
                            | syn::BinOp::BitAnd(_)
                            | syn::BinOp::BitOr(_)
                            | syn::BinOp::Shl(_)
                            | syn::BinOp::Shr(_)
                    ) {
                    expected_ty
                } else {
                    None
                };
                let right_expected = if expected_ty.is_some()
                    && matches!(
                        bin.op,
                        syn::BinOp::Add(_)
                            | syn::BinOp::Sub(_)
                            | syn::BinOp::Mul(_)
                            | syn::BinOp::Div(_)
                            | syn::BinOp::Rem(_)
                            | syn::BinOp::BitXor(_)
                            | syn::BinOp::BitAnd(_)
                            | syn::BinOp::BitOr(_)
                    ) {
                    expected_ty
                } else {
                    None
                };
                let left = self.autoderef_binary_value_operand_if_needed(
                    &bin.left,
                    &bin.op,
                    self.emit_binary_operand_expr_with_expected(&bin.left, left_expected),
                );
                let op = self.emit_binop(&bin.op);
                let right = self.autoderef_binary_value_operand_if_needed(
                    &bin.right,
                    &bin.op,
                    self.emit_binary_operand_expr_with_expected(&bin.right, right_expected),
                );
                if matches!(bin.op, syn::BinOp::Rem(_)) {
                    let left_is_float = self
                        .infer_simple_expr_type(&bin.left)
                        .as_ref()
                        .is_some_and(|ty| self.is_known_float_like_type(ty));
                    let right_is_float = self
                        .infer_simple_expr_type(&bin.right)
                        .as_ref()
                        .is_some_and(|ty| self.is_known_float_like_type(ty));
                    if left_is_float || right_is_float {
                        return format!("std::fmod({}, {})", left, right);
                    }
                }
                format!("{} {} {}", left, op, right)
            }
        }
    }

    pub(super) fn emit_binary_operand_expr(&self, expr: &syn::Expr) -> String {
        self.emit_binary_operand_expr_with_expected(expr, None)
    }

    pub(super) fn emit_binary_operand_expr_with_expected(
        &self,
        expr: &syn::Expr,
        expected_ty: Option<&syn::Type>,
    ) -> String {
        let emitted = self.emit_expr_to_string_with_expected(expr, expected_ty);
        if self.binary_operand_needs_parentheses(expr) {
            format!("({})", emitted)
        } else {
            emitted
        }
    }

    pub(super) fn try_emit_builtin_into_deserializer_ctor(
        &self,
        receiver_expr: &syn::Expr,
        receiver_cpp: &str,
        err_cpp: &str,
    ) -> Option<String> {
        let infer_path_binding_type = |expr: &syn::Expr| -> Option<syn::Type> {
            let path = match self.peel_paren_group_expr(expr) {
                syn::Expr::Path(path) => path,
                _ => return None,
            };
            if path.path.segments.len() != 1 {
                return None;
            }
            let local_name = path.path.segments[0].ident.to_string();
            self.lookup_local_binding_type(&local_name).or_else(|| {
                let struct_name = self.current_struct.as_ref()?;
                let field_ty = self.lookup_struct_field_type(struct_name, &local_name)?;
                self.expected_option_type_arg(Some(&field_ty))
                    .cloned()
                    .or(Some(field_ty))
            })
        };

        let recv_ty = self
            .infer_simple_expr_type(receiver_expr)
            .or_else(|| infer_path_binding_type(receiver_expr))
            .or_else(|| {
                let call = match self.peel_paren_group_expr(receiver_expr) {
                    syn::Expr::Call(call) => call,
                    _ => return None,
                };
                let func_path = match self.peel_paren_group_expr(call.func.as_ref()) {
                    syn::Expr::Path(path_expr) => &path_expr.path,
                    _ => return None,
                };
                let is_std_move = func_path
                    .segments
                    .last()
                    .is_some_and(|seg| seg.ident == "move");
                if !is_std_move || call.args.len() != 1 {
                    return None;
                }
                let arg = call.args.first()?;
                self.infer_simple_expr_type(arg)
                    .or_else(|| infer_path_binding_type(arg))
            })?;
        let recv_ty = self.peel_reference_paren_group_type(&recv_ty);
        let recv_cpp = Self::strip_cpp_cvref_qualifiers(self.map_type(recv_ty));
        let owner = self.builtin_deserializer_owner_for_cpp_type(recv_cpp.as_str())?;
        Some(format!(
            "::de::value::{}<{}>::new_({})",
            owner, err_cpp, receiver_cpp
        ))
    }

    pub(super) fn try_emit_boxed_u8_array_expr(&self, expr: &syn::Expr) -> Option<String> {
        let peeled = self.peel_paren_group_expr(expr);
        match peeled {
            syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::ByteStr(bs),
                    ..
                }) = self.peel_paren_group_expr(&unary.expr)
                {
                    let bytes = bs.value();
                    if bytes.is_empty() {
                        return Some("std::array<uint8_t, 0>{}".to_string());
                    }
                    let elems: Vec<String> = bytes
                        .iter()
                        .map(|b| format!("static_cast<uint8_t>({})", b))
                        .collect();
                    return Some(format!("std::array{{{}}}", elems.join(", ")));
                }
                None
            }
            syn::Expr::Array(arr) => {
                if arr.elems.is_empty() {
                    return Some("std::array<uint8_t, 0>{}".to_string());
                }
                let elems: Vec<String> = arr
                    .elems
                    .iter()
                    .map(|elem| format!("static_cast<uint8_t>({})", self.emit_expr_to_string(elem)))
                    .collect();
                Some(format!("std::array{{{}}}", elems.join(", ")))
            }
            syn::Expr::Repeat(repeat) => {
                let value = self.emit_expr_to_string(&repeat.expr);
                let len = self.emit_expr_to_string(&repeat.len);
                Some(format!(
                    "rusty::array_repeat(static_cast<uint8_t>({}), {})",
                    value, len
                ))
            }
            _ => None,
        }
    }

    pub(super) fn try_emit_boxed_array_expr_with_expected_elem_type(
        &self,
        expr: &syn::Expr,
        expected_elem_ty: &syn::Type,
    ) -> Option<String> {
        let peeled = self.peel_paren_group_expr(expr);
        match peeled {
            syn::Expr::Array(arr) => {
                if arr.elems.is_empty() {
                    let elem_cpp = self.map_type(expected_elem_ty);
                    return Some(format!("std::array<{}, 0>{{}}", elem_cpp));
                }
                let elems: Vec<String> = arr
                    .elems
                    .iter()
                    .map(|elem| {
                        self.emit_expr_to_string_with_expected(elem, Some(expected_elem_ty))
                    })
                    .collect();
                Some(format!("std::array{{{}}}", elems.join(", ")))
            }
            syn::Expr::Repeat(repeat) => {
                let value =
                    self.emit_expr_to_string_with_expected(&repeat.expr, Some(expected_elem_ty));
                let len = self.emit_expr_to_string(&repeat.len);
                Some(format!("rusty::array_repeat({}, {})", value, len))
            }
            _ => None,
        }
    }

    pub(super) fn try_emit_boxed_tuple_array_expr_with_expected_tuple(
        &self,
        expr: &syn::Expr,
        expected_tuple_ty: &syn::TypeTuple,
    ) -> Option<String> {
        let peeled = self.peel_paren_group_expr(expr);
        let syn::Expr::Array(arr) = peeled else {
            return None;
        };
        if arr.elems.is_empty() {
            let expected_tuple_cpp = self.map_type(&syn::Type::Tuple(expected_tuple_ty.clone()));
            return Some(format!("std::array<{}, 0>{{}}", expected_tuple_cpp));
        }
        let elems: Vec<String> = arr
            .elems
            .iter()
            .map(|elem| {
                let tuple = self.peel_paren_group_expr(elem);
                let syn::Expr::Tuple(tuple_expr) = tuple else {
                    return None;
                };
                self.emit_tuple_expr_with_expected_shape(tuple_expr, expected_tuple_ty)
            })
            .collect::<Option<Vec<_>>>()?;
        Some(format!("std::array{{{}}}", elems.join(", ")))
    }

    pub(super) fn try_emit_boxed_tuple_array_expr_with_inferred_tuple_harmonization(
        &self,
        expr: &syn::Expr,
    ) -> Option<String> {
        let peeled = self.peel_paren_group_expr(expr);
        let syn::Expr::Array(arr) = peeled else {
            return None;
        };
        let first = arr.elems.first()?;
        let first = self.peel_paren_group_expr(first);
        let syn::Expr::Tuple(first_tuple) = first else {
            return None;
        };
        let expected_tuple_ty = self.infer_tuple_type_from_tuple_expr(first_tuple)?;
        let elems: Vec<String> = arr
            .elems
            .iter()
            .map(|elem| {
                let tuple = self.peel_paren_group_expr(elem);
                let syn::Expr::Tuple(tuple_expr) = tuple else {
                    return None;
                };
                self.emit_tuple_expr_with_expected_shape(tuple_expr, &expected_tuple_ty)
            })
            .collect::<Option<Vec<_>>>()?;
        Some(format!("std::array{{{}}}", elems.join(", ")))
    }

    pub(super) fn try_emit_into_vec_box_new_with_u8_payload(&self, arg: &syn::Expr) -> Option<String> {
        let syn::Expr::Call(box_call) = self.peel_paren_group_expr(arg) else {
            return None;
        };
        if box_call.args.len() != 1 {
            return None;
        }
        let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(box_call.func.as_ref()) else {
            return None;
        };
        let joined = path_expr
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        let use_boxed_helper = matches!(
            joined.as_str(),
            "rusty::boxed::box_new" | "box_new" | "alloc::boxed::box_new" | "std::boxed::box_new"
        );
        let use_box_ctor = matches!(
            joined.as_str(),
            "Box::new"
                | "Box::new_"
                | "rusty::Box::new"
                | "rusty::Box::new_"
                | "alloc::boxed::Box::new"
                | "std::boxed::Box::new"
        );
        if !use_boxed_helper && !use_box_ctor {
            return None;
        }
        let payload = self.try_emit_boxed_u8_array_expr(&box_call.args[0])?;
        let ctor = if use_box_ctor {
            "rusty::Box::new_"
        } else {
            "rusty::boxed::box_new"
        };
        Some(format!("{}({})", ctor, payload))
    }

    pub(super) fn try_emit_into_vec_box_new_with_expected_elem_payload(
        &self,
        arg: &syn::Expr,
        expected_elem_ty: &syn::Type,
    ) -> Option<String> {
        let syn::Expr::Call(box_call) = self.peel_paren_group_expr(arg) else {
            return None;
        };
        if box_call.args.len() != 1 {
            return None;
        }
        let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(box_call.func.as_ref()) else {
            return None;
        };
        let joined = path_expr
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        let use_boxed_helper = matches!(
            joined.as_str(),
            "rusty::boxed::box_new" | "box_new" | "alloc::boxed::box_new" | "std::boxed::box_new"
        );
        let use_box_ctor = matches!(
            joined.as_str(),
            "Box::new"
                | "Box::new_"
                | "rusty::Box::new"
                | "rusty::Box::new_"
                | "alloc::boxed::Box::new"
                | "std::boxed::Box::new"
        );
        if !use_boxed_helper && !use_box_ctor {
            return None;
        }
        let payload = self.try_emit_boxed_array_expr_with_expected_elem_type(
            &box_call.args[0],
            expected_elem_ty,
        )?;
        let ctor = if use_box_ctor {
            "rusty::Box::new_"
        } else {
            "rusty::boxed::box_new"
        };
        Some(format!("{}({})", ctor, payload))
    }

    pub(super) fn try_emit_into_vec_box_new_with_expected_tuple_payload(
        &self,
        arg: &syn::Expr,
        expected_tuple_ty: &syn::TypeTuple,
    ) -> Option<String> {
        let syn::Expr::Call(box_call) = self.peel_paren_group_expr(arg) else {
            return None;
        };
        if box_call.args.len() != 1 {
            return None;
        }
        let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(box_call.func.as_ref()) else {
            return None;
        };
        let joined = path_expr
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        let use_boxed_helper = matches!(
            joined.as_str(),
            "rusty::boxed::box_new" | "box_new" | "alloc::boxed::box_new" | "std::boxed::box_new"
        );
        let use_box_ctor = matches!(
            joined.as_str(),
            "Box::new"
                | "Box::new_"
                | "rusty::Box::new"
                | "rusty::Box::new_"
                | "alloc::boxed::Box::new"
                | "std::boxed::Box::new"
        );
        if !use_boxed_helper && !use_box_ctor {
            return None;
        }
        let payload = self.try_emit_boxed_tuple_array_expr_with_expected_tuple(
            &box_call.args[0],
            expected_tuple_ty,
        )?;
        let ctor = if use_box_ctor {
            "rusty::Box::new_"
        } else {
            "rusty::boxed::box_new"
        };
        Some(format!("{}({})", ctor, payload))
    }

    pub(super) fn try_emit_into_vec_box_new_with_inferred_tuple_payload(
        &self,
        arg: &syn::Expr,
    ) -> Option<String> {
        let syn::Expr::Call(box_call) = self.peel_paren_group_expr(arg) else {
            return None;
        };
        if box_call.args.len() != 1 {
            return None;
        }
        let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(box_call.func.as_ref()) else {
            return None;
        };
        let joined = path_expr
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        let use_boxed_helper = matches!(
            joined.as_str(),
            "rusty::boxed::box_new" | "box_new" | "alloc::boxed::box_new" | "std::boxed::box_new"
        );
        let use_box_ctor = matches!(
            joined.as_str(),
            "Box::new"
                | "Box::new_"
                | "rusty::Box::new"
                | "rusty::Box::new_"
                | "alloc::boxed::Box::new"
                | "std::boxed::Box::new"
        );
        if !use_boxed_helper && !use_box_ctor {
            return None;
        }
        let payload = self
            .try_emit_boxed_tuple_array_expr_with_inferred_tuple_harmonization(&box_call.args[0])?;
        let ctor = if use_box_ctor {
            "rusty::Box::new_"
        } else {
            "rusty::boxed::box_new"
        };
        Some(format!("{}({})", ctor, payload))
    }

    pub(super) fn try_emit_default_value_expr_for_type(&self, ty: &syn::Type) -> Option<String> {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return None;
        };
        let last = tp.path.segments.last()?;
        if !matches!(last.ident.to_string().as_str(), "Range" | "range") {
            return None;
        }
        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
            return None;
        };
        let inner_ty = args.args.iter().find_map(|arg| match arg {
            syn::GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })?;
        let inner_cpp = self.map_type(inner_ty);
        if inner_cpp == "auto"
            || inner_cpp.contains("/* TODO")
            || type_string_has_auto_placeholder(&inner_cpp)
        {
            return None;
        }
        Some(format!(
            "rusty::range<{}>(static_cast<{}>(0), static_cast<{}>(0))",
            inner_cpp, inner_cpp, inner_cpp
        ))
    }

    pub(super) fn try_emit_try_into_method_call(
        &self,
        mc: &syn::ExprMethodCall,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        if mc.method != "try_into" || !mc.args.is_empty() {
            return None;
        }
        let expected_ty = expected_ty?;
        let target_ty = self.resolve_try_into_target_type(expected_ty, &mc.receiver)?;
        let target_cpp = self.map_type(&target_ty);
        let receiver = self.emit_try_into_receiver_arg(&mc.receiver);
        let target_is_array_like = matches!(
            self.peel_reference_paren_group_type(&target_ty),
            syn::Type::Array(_)
        );
        let target_is_scalar_like =
            target_cpp == "Self" || is_numeric_cpp_scalar_type(target_cpp.trim());
        if target_is_array_like || target_is_scalar_like {
            return Some(format!("rusty::try_from<{}>({})", target_cpp, receiver));
        }
        Some(format!("{}::try_from({})", target_cpp, receiver))
    }

    pub(super) fn try_emit_into_method_call(
        &self,
        mc: &syn::ExprMethodCall,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        if mc.method != "into" || !mc.args.is_empty() {
            return None;
        }
        if expected_ty.is_none() && self.expr_lowers_to_slice_or_span_view(&mc.receiver) {
            let receiver = self.emit_expr_maybe_move(&mc.receiver);
            // `rusty::to_vec` returns `std::vector<Elem>`, which doesn't
            // convert to `rusty::Vec<Elem>`. Brace-init / factory sites
            // like `Content::ByteBuf(value.into())` in serde need a
            // `rusty::Vec<u8>`. Pin the element type via `decltype` so
            // `rusty::Vec<T>::from_iter` works at C++ compile time
            // without depending on receiver-type inference reaching the
            // value's declaration. See twin emit in
            // `emit_method_call_expr_to_string` for the `.to_vec()`
            // variant and `docs/rusty-cpp-transpiler.md` §13 for the
            // general inference-architecture rationale.
            return Some(format!(
                "rusty::Vec<std::remove_cvref_t<decltype(*std::begin({0}))>>::from_iter({0})",
                receiver
            ));
        }
        let receiver_kind = self.classify_into_receiver_expr(&mc.receiver);
        let expected_was_none = expected_ty.is_none();
        let target_ty = match expected_ty {
            Some(expected) => expected.clone(),
            // Keep no-context string-literal lowering compilable (`"x".into()`) while
            // avoiding blanket rewrites for non-string primitives without target type.
            None if receiver_kind == Some(IntoReceiverKind::StringLike) => {
                parse_quote!(rusty::String)
            }
            // No explicit target and scalar receiver (`x.into()`) can stay value-shaped:
            // C++ implicit conversions at the call site handle concrete destination types.
            None if receiver_kind == Some(IntoReceiverKind::ScalarLike) => {
                return Some(self.emit_expr_maybe_move(&mc.receiver));
            }
            // Without a concrete target type, Rust `.into()` has no standalone
            // conversion meaning; keep value-shape lowering unless an inherent
            // receiver `into` method is explicitly available.
            None if !self.receiver_has_inherent_method_named(&mc.receiver, "into") => {
                return Some(self.emit_expr_maybe_move(&mc.receiver));
            }
            None => return None,
        };
        let target_ty = if self.type_contains_infer(&target_ty) {
            // `let set2: IndexSet<_> = [1, 2, 3, 4].into();` — fill the
            // infer from the receiver's element/item type (set-shaped
            // targets take the item whole, map-shaped fill positionally).
            let filled = self.resolve_expected_type_with_iter_hint(&target_ty, &mc.receiver);
            if self.type_contains_infer(&filled) {
                return None;
            }
            filled
        } else {
            target_ty
        };
        let target_cpp = self.map_type(&target_ty);
        if target_cpp == "auto"
            || target_cpp.contains("/* TODO")
            || type_string_has_auto_placeholder(&target_cpp)
        {
            return None;
        }

        let stripped_target = self.strip_into_target_cpp_type(&target_cpp);
        let canonical_target = self.canonical_into_target_cpp_type(&target_cpp);
        let receiver = self.emit_expr_maybe_move(&mc.receiver);
        if expected_was_none
            && self.expr_lowers_to_slice_or_span_view(&mc.receiver)
            && canonical_target.starts_with("rusty::Vec<")
        {
            return Some(format!("rusty::to_vec({})", receiver));
        }

        match receiver_kind {
            Some(IntoReceiverKind::StringLike) => match canonical_target.as_str() {
                "rusty::String" => Some(format!("rusty::String::from({})", receiver)),
                "std::string" => Some(format!("std::string({})", receiver)),
                "std::string_view" => Some(format!("std::string_view({})", receiver)),
                // Rust `&str` can lower to C-string receiver surfaces.
                "char*" => Some(receiver),
                _ => None,
            },
            Some(IntoReceiverKind::ScalarLike) => {
                if Self::is_scalar_into_target_cpp_type(&canonical_target) {
                    Some(format!("static_cast<{}>({})", stripped_target, receiver))
                } else {
                    Some(format!("rusty::from_into<{}>({})", target_cpp, receiver))
                }
            }
            None => Some(format!("rusty::from_into<{}>({})", target_cpp, receiver)),
        }
    }

    pub(super) fn try_emit_arc_from_assoc_call(
        &self,
        call: &syn::ExprCall,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let syn::Expr::Path(func_path) = call.func.as_ref() else {
            return None;
        };
        if call.args.len() != 1 || func_path.path.segments.len() < 2 {
            return None;
        }
        let method = func_path.path.segments.last()?.ident.to_string();
        if method != "from" {
            return None;
        }
        let owner_seg = func_path.path.segments.iter().nth_back(1)?;
        if owner_seg.ident != "Arc" {
            return None;
        }

        let explicit_owner_inner = match &owner_seg.arguments {
            syn::PathArguments::AngleBracketed(args) => {
                args.args.iter().find_map(|arg| match arg {
                    syn::GenericArgument::Type(t) => {
                        let mapped = self.map_type(t);
                        (mapped != "auto"
                            && !mapped.contains("/* TODO")
                            && !type_string_has_auto_placeholder(&mapped))
                        .then_some(mapped)
                    }
                    _ => None,
                })
            }
            _ => None,
        };
        let expected_owner_inner = expected_ty
            .and_then(|ty| self.expected_type_generic_args_for_owner(ty, "Arc"))
            .and_then(|args| args.first().cloned())
            .filter(|mapped| {
                mapped != "auto"
                    && !mapped.contains("/* TODO")
                    && !type_string_has_auto_placeholder(mapped)
            });
        let return_owner_inner = self
            .current_return_type_hint()
            .and_then(|ty| self.expected_type_generic_args_for_owner(ty, "Arc"))
            .and_then(|args| args.first().cloned())
            .filter(|mapped| {
                mapped != "auto"
                    && !mapped.contains("/* TODO")
                    && !type_string_has_auto_placeholder(mapped)
            });
        let inferred_owner_inner = call
            .args
            .first()
            .and_then(|arg| {
                self.infer_hint_type_from_expr(arg)
                    .or_else(|| self.infer_simple_expr_type(arg))
            })
            .map(|ty| self.map_type(&ty))
            .filter(|mapped| {
                mapped != "auto"
                    && !mapped.contains("/* TODO")
                    && !type_string_has_auto_placeholder(mapped)
            })
            .or_else(|| {
                call.args
                    .first()
                    .and_then(|arg| self.infer_remove_cvref_decltype_from_expr(arg))
            });

        let owner_inner = explicit_owner_inner
            .or(expected_owner_inner)
            .or(return_owner_inner)
            .or(inferred_owner_inner)?;
        if self.owner_template_arg_is_value_identifier(&owner_inner) {
            return None;
        }
        let arg = self.emit_expr_maybe_move(call.args.first()?);
        Some(format!(
            "rusty::from_into<rusty::sync::Arc<{}>>({})",
            owner_inner, arg
        ))
    }

    pub(super) fn try_emit_runtime_cow_variant_ctor_call(&self, call: &syn::ExprCall) -> Option<String> {
        let syn::Expr::Path(func_path) = call.func.as_ref() else {
            return None;
        };
        if call.args.len() != 1 || func_path.path.segments.len() < 2 {
            return None;
        }
        let owner = func_path
            .path
            .segments
            .iter()
            .nth_back(1)?
            .ident
            .to_string();
        if !self.owner_name_is_known_cow_like(&owner) {
            return None;
        }
        let variant = func_path.path.segments.last()?.ident.to_string();
        let arg_expr = call.args.first()?;
        let wrapped = match variant.as_str() {
            "Borrowed" => {
                let arg = self.emit_expr_maybe_move(arg_expr);
                format!("rusty::Cow_Borrowed({})", arg)
            }
            "Owned" => {
                let arg = if let syn::Expr::Path(path) = self.peel_paren_group_expr(arg_expr)
                    && path.path.segments.len() == 1
                {
                    let name = path.path.segments[0].ident.to_string();
                    let emitted = self.emit_expr_to_string(arg_expr);
                    if self.is_pattern_ref_binding_in_scope(&name)
                        && !self.is_local_reference_binding_in_scope(&name)
                    {
                        format!("std::move({})", emitted)
                    } else {
                        self.emit_expr_maybe_move(arg_expr)
                    }
                } else {
                    self.emit_expr_maybe_move(arg_expr)
                };
                format!("rusty::Cow_Owned({})", arg)
            }
            _ => return None,
        };
        Some(format!("rusty::Cow({})", wrapped))
    }

    pub(super) fn try_emit_to_owned_method_call(&self, mc: &syn::ExprMethodCall) -> Option<String> {
        if mc.method != "to_owned" || !mc.args.is_empty() {
            return None;
        }
        let receiver = self.emit_expr_maybe_move(&mc.receiver);
        let receiver_is_string_like = self.classify_into_receiver_expr(&mc.receiver)
            == Some(IntoReceiverKind::StringLike)
            || self
                .infer_simple_expr_type(&mc.receiver)
                .as_ref()
                .is_some_and(|ty| {
                    self.is_known_string_like_type(ty) || self.map_type(ty) == "std::string_view"
                });
        if receiver_is_string_like {
            return Some(format!("rusty::String::from({})", receiver));
        }
        // Slice receiver (`&[T]` / `[T]`) — emit
        // `rusty::Vec<T>::from_iter(value)`. `rusty::to_owned(span)` was
        // removed from the runtime during Vec retirement (rusty/rusty.hpp:435
        // comment: "Callers should import rusty; and use rusty::Vec ctors
        // directly"), so the generic fallback below would return
        // `std::span<T>` and fail to convert to the `rusty::Vec<T>` field
        // type at brace-init sites like `Content_Bytes{rusty::to_owned(
        // value)}` in serde's content serialization path. `from_iter` is
        // the Vec API surface that accepts any iterable; the iterator-pair
        // constructor isn't exposed on the transpiled `Vec` (only
        // `Vec(std::initializer_list<T>)` and the typed `from_iter` static).
        if let Some(elem_cpp) = self
            .infer_simple_expr_type(&mc.receiver)
            .as_ref()
            .and_then(|ty| self.byte_slice_elem_cpp_type(ty))
        {
            return Some(format!(
                "rusty::Vec<{}>::from_iter({})",
                elem_cpp, receiver
            ));
        }
        // Generic ToOwned fallback: runtime helper dispatches to .clone() when available.
        Some(format!("rusty::to_owned({})", receiver))
    }

    /// If `ty` is a slice (`[T]`) or a reference to a slice (`&[T]`),
    /// return the C++-mapped element type. Returns None for non-slice
    /// types. Used by `try_emit_to_owned_method_call` to route byte-slice
    /// (and other slice) `to_owned` through a direct `rusty::Vec<T>(it,
    /// end)` construction instead of the generic runtime helper.
    fn byte_slice_elem_cpp_type(&self, ty: &syn::Type) -> Option<String> {
        let mut inner = ty;
        // Strip leading `&` / `&mut`.
        while let syn::Type::Reference(r) = inner {
            inner = &r.elem;
        }
        if let syn::Type::Slice(s) = inner {
            return Some(self.map_type(&s.elem));
        }
        None
    }

    pub(super) fn try_emit_into_owned_method_call(&self, mc: &syn::ExprMethodCall) -> Option<String> {
        if mc.method != "into_owned" || !mc.args.is_empty() {
            return None;
        }
        let receiver = self.emit_expr_maybe_move(&mc.receiver);
        Some(format!("rusty::into_owned({})", receiver))
    }

    pub(super) fn try_emit_arrayvec_from_repeat_with_fixed_array_arg(
        &self,
        call: &syn::ExprCall,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        if path_expr.path.segments.len() < 2 || call.args.len() != 1 {
            return None;
        }
        let owner_idx = path_expr.path.segments.len() - 2;
        let owner_seg = path_expr.path.segments.iter().nth_back(1)?;
        if owner_seg.ident != "ArrayVec" {
            return None;
        }
        let method_name = path_expr.path.segments.last()?.ident.to_string();
        if !matches!(method_name.as_str(), "from" | "try_from") {
            return None;
        }
        let repeat_expr = match self.peel_paren_group_expr(&call.args[0]) {
            syn::Expr::Repeat(repeat_expr) => repeat_expr,
            syn::Expr::Cast(cast_expr) => match self.peel_paren_group_expr(&cast_expr.expr) {
                syn::Expr::Repeat(repeat_expr) => repeat_expr,
                _ => return None,
            },
            _ => return None,
        };

        let mut elem_cpp: Option<String> = None;
        let mut cap_cpp: Option<String> = None;
        if let syn::PathArguments::AngleBracketed(owner_args) = &owner_seg.arguments {
            for arg in &owner_args.args {
                match arg {
                    syn::GenericArgument::Type(t) if !matches!(t, syn::Type::Infer(_)) => {
                        elem_cpp = Some(self.map_type(t));
                    }
                    syn::GenericArgument::Const(c) => {
                        cap_cpp = Some(self.emit_expr_to_string(c));
                    }
                    _ => {}
                }
            }
        }
        let expected_owner_args = expected_ty.and_then(|ty| self.expected_type_generic_args(ty));
        if elem_cpp.is_none() {
            elem_cpp = expected_owner_args
                .as_ref()
                .and_then(|args| args.first())
                .filter(|arg| *arg != "auto")
                .cloned();
        }
        if cap_cpp.is_none() {
            cap_cpp = expected_owner_args
                .as_ref()
                .and_then(|args| args.get(1))
                .filter(|arg| *arg != "auto")
                .cloned();
        }
        if elem_cpp.is_none() || cap_cpp.is_none() {
            if let Some(inferred_owner_args) =
                self.infer_owner_template_args_for_call(None, "ArrayVec", &method_name, call)
            {
                if elem_cpp.is_none() {
                    elem_cpp = inferred_owner_args
                        .first()
                        .and_then(|entry| entry.as_ref())
                        .cloned();
                }
                if cap_cpp.is_none() {
                    cap_cpp = inferred_owner_args
                        .get(1)
                        .and_then(|entry| entry.as_ref())
                        .cloned();
                }
            }
        }
        if elem_cpp.is_none() || cap_cpp.is_none() {
            let mut owner_path = syn::Path {
                leading_colon: path_expr.path.leading_colon,
                segments: syn::punctuated::Punctuated::new(),
            };
            for seg in path_expr.path.segments.iter().take(owner_idx + 1) {
                owner_path.segments.push(seg.clone());
            }
            if let Some(scoped_owner_args) =
                self.recover_omitted_owner_generic_args_from_scope(&owner_path)
            {
                if elem_cpp.is_none() {
                    elem_cpp = scoped_owner_args.first().cloned();
                }
                if cap_cpp.is_none() {
                    cap_cpp = scoped_owner_args.get(1).cloned();
                }
            }
        }

        let (elem_cpp, cap_cpp) = (elem_cpp?, cap_cpp?);
        let value = self.emit_expr_to_string(&repeat_expr.expr);
        let repeat_cap = self.maybe_sanitize_array_capacity_cpp_len(&cap_cpp);
        let repeat_seed = Self::emit_repeat_seed_with_cast("_seed", &elem_cpp);
        let fixed_array_arg = format!(
            "[](auto _seed) {{ std::array<{}, {}> _repeat{{}}; _repeat.fill({}); return _repeat; }}({})",
            elem_cpp, repeat_cap, repeat_seed, value
        );
        let func = self.emit_call_func_with_owner_template_recovery(call, expected_ty);
        Some(format!("{}({})", func, fixed_array_arg))
    }

    pub(super) fn try_emit_array_from_fn_call_with_expected(
        &self,
        call: &syn::ExprCall,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        let mut segments = path_expr.path.segments.iter().rev();
        let method = segments.next()?.ident.to_string();
        let owner = segments.next()?.ident.to_string();
        if method != "from_fn" || owner != "array" {
            return None;
        }
        if call.args.len() != 1 {
            return None;
        }
        let cap_expr = self
            .expected_fixed_array_type(expected_ty)
            .map(|(_, len)| self.emit_expr_to_string(len))
            .or_else(|| {
                ["N", "K", "CAP"]
                    .into_iter()
                    .find(|name| self.is_type_param_in_scope(name))
                    .map(|name| name.to_string())
            })?;
        let cap_expr = self.maybe_sanitize_array_capacity_cpp_len(&cap_expr);
        let mapper = self.emit_expr_maybe_move(&call.args[0]);
        Some(format!("rusty::array_from_fn<{}>({})", cap_expr, mapper))
    }

    pub(super) fn try_emit_cpp_import_bound_member_call(&self, call: &syn::ExprCall) -> Option<String> {
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        if path_expr.path.segments.len() < 3 || call.args.is_empty() {
            return None;
        }
        let (module_path, symbol_name) =
            self.resolve_cpp_import_bound_symbol_for_path(&path_expr.path)?;
        if !self.cpp_import_symbol_is_member_method(&module_path, &symbol_name) {
            return None;
        }
        let method_name = path_expr.path.segments.last()?.ident.to_string();
        let method_template_args = self.emit_expr_path_template_args(&path_expr.path);
        let receiver = call.args.first()?;
        let member_args: Vec<String> = call
            .args
            .iter()
            .skip(1)
            .map(|arg| self.emit_expr_maybe_move(arg))
            .collect();
        Some(self.emit_receiver_member_call(
            receiver,
            &method_name,
            method_template_args.as_deref(),
            &member_args,
            None,
        ))
    }

    pub(super) fn try_emit_type_alias_impl_associated_call(&self, call: &syn::ExprCall) -> Option<String> {
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        if path_expr.path.segments.len() < 2 {
            return None;
        }

        let mut owner_path = syn::Path {
            leading_colon: path_expr.path.leading_colon,
            segments: syn::punctuated::Punctuated::new(),
        };
        for seg in path_expr
            .path
            .segments
            .iter()
            .take(path_expr.path.segments.len().saturating_sub(1))
        {
            owner_path.segments.push(seg.clone());
        }
        let owner_name = owner_path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default();
        let method_name = path_expr
            .path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default();
        let (owner_key, receiver_shape) = self
            .resolve_alias_owner_key_with_method_from_owner_path(
                Some(&owner_path),
                &owner_name,
                &method_name,
            )?;
        let helper_path = self.alias_impl_helper_function_path(
            &owner_key,
            &method_name,
            self.emit_expr_path_template_args(&path_expr.path)
                .as_deref(),
        );

        if matches!(receiver_shape, Some(true)) {
            let receiver_expr = call.args.first()?;
            let receiver_arg = match self.peel_paren_group_expr(receiver_expr) {
                syn::Expr::Reference(reference) => self.emit_expr_to_string(&reference.expr),
                _ => self.emit_expr_maybe_move(receiver_expr),
            };
            let mut args = Vec::with_capacity(call.args.len());
            args.push(receiver_arg);
            args.extend(
                call.args
                    .iter()
                    .skip(1)
                    .map(|arg| self.emit_expr_maybe_move(arg)),
            );
            return Some(format!("{}({})", helper_path, args.join(", ")));
        }

        if matches!(receiver_shape, Some(false) | None) {
            let args: Vec<String> = call
                .args
                .iter()
                .map(|arg| self.emit_expr_maybe_move(arg))
                .collect();
            return Some(format!("{}({})", helper_path, args.join(", ")));
        }
        None
    }

    /// Emit a call expression, optionally using expected type context from parent.
    /// Item 8: emit `NAME(args)` as either `__self(__self, args)` (inside the
    /// body of NAME, where NAME's own type isn't deducible yet) or
    /// `NAME(NAME, args)` (outside the body, seeding the Y-combinator).
    /// Returns `Some(emit)` when the call resolves to a recursive nested
    /// fn currently tracked; the caller should use this result and skip
    /// the regular path-resolution flow.
    pub(super) fn try_emit_y_combinator_call(&self, call: &syn::ExprCall) -> Option<String> {
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        if path_expr.qself.is_some() {
            return None;
        }
        if path_expr.path.segments.len() != 1 {
            return None;
        }
        let seg = path_expr.path.segments.first()?;
        if !matches!(seg.arguments, syn::PathArguments::None) {
            return None;
        }
        let ident = seg.ident.to_string();
        let inside_own_body = self
            .recursive_nested_fn_self_emit_stack
            .last()
            .is_some_and(|top| top == &ident);
        let in_outer_scope = !inside_own_body
            && self
                .recursive_nested_fns_in_scope
                .iter()
                .rev()
                .any(|scope| scope.contains(&ident));
        if !inside_own_body && !in_outer_scope {
            return None;
        }
        let seed = if inside_own_body {
            "__self".to_string()
        } else {
            escape_cpp_keyword(&ident)
        };
        let mut args = vec![seed.clone()];
        for a in &call.args {
            args.push(self.emit_expr_maybe_move(a));
        }
        Some(format!("{}({})", seed, args.join(", ")))
    }

    /// A self-recursive nested fn's name used as a VALUE (not the head of a
    /// call) — `iter_cmp_by(a, b, total_cmp)`. The Y-combinator lambda takes
    /// `__self` as its first parameter, so the bare name can't be passed
    /// directly (and `const auto total_cmp = [...total_cmp...]` self-references
    /// its own initializer). Emit a self-bound wrapper lambda that forwards to
    /// `__self(__self, ...)` (inside the body) or `NAME(NAME, ...)` (outer
    /// scope), presenting the ordinary N-argument signature callers expect.
    pub(super) fn try_emit_recursive_nested_fn_value_reference(
        &self,
        path: &syn::Path,
    ) -> Option<String> {
        if path.leading_colon.is_some() || path.segments.len() != 1 {
            return None;
        }
        let seg = path.segments.first()?;
        if !matches!(seg.arguments, syn::PathArguments::None) {
            return None;
        }
        let ident = seg.ident.to_string();
        let inside_own_body = self
            .recursive_nested_fn_self_emit_stack
            .last()
            .is_some_and(|top| top == &ident);
        let in_outer_scope = !inside_own_body
            && self
                .recursive_nested_fns_in_scope
                .iter()
                .rev()
                .any(|scope| scope.contains(&ident));
        if !inside_own_body && !in_outer_scope {
            return None;
        }
        let seed = if inside_own_body {
            "__self".to_string()
        } else {
            escape_cpp_keyword(&ident)
        };
        Some(format!(
            "[&](auto&&... __rec_args) -> decltype(auto) {{ return {}({}, std::forward<decltype(__rec_args)>(__rec_args)...); }}",
            seed, seed
        ))
    }

    /// `slice::from_raw_parts[_mut](ptr, len)` → `rusty::from_raw_parts[_mut](ptr, len)`
    /// (a `std::span<T>`). The slice element `T` lives only in the RESULT type
    /// (`&[T]` / `&mut [T]`), so without it the `.cast()` on the pointer argument
    /// defaults to `void*` and `span<void>` fails to instantiate. Recover `T` from
    /// the call's expected type and feed `*const T` / `*mut T` as the pointer arg's
    /// expected type, so the cast lands on `T*` and the span element deduces.
    pub(super) fn try_emit_slice_from_raw_parts_call(
        &self,
        call: &syn::ExprCall,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let syn::Expr::Path(p) = call.func.as_ref() else {
            return None;
        };
        if !Self::is_slice_view_constructor_path(&p.path) || call.args.len() != 2 {
            return None;
        }
        let is_mut = p
            .path
            .segments
            .last()
            .is_some_and(|s| s.ident == "from_raw_parts_mut");
        let elem_ty = extract_slice_element_type_for_hint(expected_ty?)?;
        let ptr_expected: syn::Type = if is_mut {
            syn::parse_quote!(*mut #elem_ty)
        } else {
            syn::parse_quote!(*const #elem_ty)
        };
        let ptr_arg = self.emit_expr_to_string_with_expected(&call.args[0], Some(&ptr_expected));
        let len_arg = self.emit_expr_to_string(&call.args[1]);
        let func = if is_mut {
            "rusty::from_raw_parts_mut"
        } else {
            "rusty::from_raw_parts"
        };
        Some(format!("{}({}, {})", func, ptr_arg, len_arg))
    }

    /// `<ScalarType>::from(x)` (a widening/primitive conversion, e.g.
    /// `ptrdiff_t::from(i32::MAX)`) → `static_cast<ScalarType>(x)`. Rust's
    /// primitive `From` is just a numeric conversion, but the C++ scalar type is
    /// not a class, so `ScalarType::from(...)` is ill-formed.
    pub(super) fn try_emit_scalar_from_call(&self, call: &syn::ExprCall) -> Option<String> {
        let syn::Expr::Path(p) = call.func.as_ref() else {
            return None;
        };
        if p.qself.is_some() || call.args.len() != 1 || p.path.segments.len() < 2 {
            return None;
        }
        if p.path.segments.last()?.ident != "from" {
            return None;
        }
        let mut owner_path = p.path.clone();
        owner_path.segments = p
            .path
            .segments
            .iter()
            .take(p.path.segments.len() - 1)
            .cloned()
            .collect();
        // A scalar conversion owner is a plain type name with no generic args.
        if !matches!(
            owner_path.segments.last()?.arguments,
            syn::PathArguments::None
        ) {
            return None;
        }
        let owner_cpp = self.map_type(&syn::Type::Path(syn::TypePath {
            qself: None,
            path: owner_path,
        }));
        // A scalar type (`ptrdiff_t`, `int32_t`, …) is global; map_type may have
        // namespace-qualified it (`yaml::ptrdiff_t`). Cast to the bare scalar name.
        let owner_bare = owner_cpp.rsplit("::").next().unwrap_or(&owner_cpp).trim();
        if !Self::is_scalar_into_target_cpp_type(&owner_bare.replace(' ', "")) {
            return None;
        }
        Some(format!(
            "static_cast<{}>({})",
            owner_bare,
            self.emit_expr_to_string(&call.args[0])
        ))
    }

    pub(super) fn emit_call_expr_to_string(
        &self,
        call: &syn::ExprCall,
        expected_ty: Option<&syn::Type>,
    ) -> String {
        // `<_>::default()` — a qself-INFER associated call takes its owner
        // from the position's EXPECTED type (indexmap's
        // `Self::with_capacity_and_hasher(n, <_>::default())`: arg 2's
        // declared param is the hasher S). Without this, owner recovery
        // substitutes Self and builds a MAP where a HASHER is expected.
        if let syn::Expr::Path(fp) = call.func.as_ref()
            && let Some(q) = &fp.qself
            && matches!(q.ty.as_ref(), syn::Type::Infer(_))
            && fp.path.segments.len() == 1
        {
            if let Some(expected) = expected_ty
                && !matches!(expected, syn::Type::Infer(_))
            {
                let owner_cpp = self.map_type(self.peel_reference_paren_group_type(expected));
                if owner_cpp != "auto"
                    && !type_string_has_auto_placeholder(&owner_cpp)
                    && !owner_cpp.contains("/* TODO")
                {
                    let method = fp.path.segments[0].ident.to_string();
                    let method_cpp = if method == "default" {
                        "default_".to_string()
                    } else {
                        escape_cpp_keyword(&method)
                    };
                    let args: Vec<String> = call
                        .args
                        .iter()
                        .map(|a| self.emit_expr_maybe_move(a))
                        .collect();
                    return format!("{}::{}({})", owner_cpp, method_cpp, args.join(", "));
                }
            }
            // No expected type threaded (plain arg position): `<_>::default()`
            // lowers to `{}` — C++ copy-list-initialization default-constructs
            // whatever the PARAMETER type is, which is exactly Rust's
            // context-inferred `<_>::default()` (indexmap's
            // `Self::with_capacity_and_hasher(n, <_>::default())`).
            if fp.path.segments[0].ident == "default" && call.args.is_empty() {
                return "{}".to_string();
            }
        }
        // Empty `Vec::new()`/`with_capacity` whose element is the item type of an
        // `auto`-typed iterator chain (recovered from a later `.extend(...)`): emit
        // the element via `decltype` instead of leaking `Vec<auto>`. This is the
        // engine's authoritative answer for a type C++ can only name via decltype.
        if let Some(emitted) = self.try_emit_empty_collection_ctor_with_decltype_element(call) {
            return emitted;
        }
        // An empty `Vec::new()` inside an itertools `assert_equal(A, B)` second
        // arg: its type equals `A`'s item type (element-wise comparison).
        if let Some(emitted) = self.try_emit_assert_equal_sibling_empty_vec(call) {
            return emitted;
        }
        // Calling an `unsafe fn` value (→ rusty::UnsafeFn, which has no call
        // operator by design): lower `f(args)` to `f.call_unsafe(args)`.
        if let Some(emitted) = self.try_emit_unsafe_fn_call(call) {
            return emitted;
        }
        // `slice::from_raw_parts[_mut]` — feed the result's element type to the
        // pointer-arg cast so `span<T>` deduces instead of `span<void>`.
        if let Some(emitted) = self.try_emit_slice_from_raw_parts_call(call, expected_ty) {
            return emitted;
        }
        // `T::trait_method(a0, ...)` on a generic param T: lower through the
        // extension dispatcher (first arg is the Rust receiver) — a C++ type
        // param can't host a static trait call for primitive substitutions.
        if let Some(emitted) = self.try_emit_type_param_trait_static_call(call) {
            return emitted;
        }
        // `<ScalarType>::from(x)` (primitive conversion) → `static_cast<ScalarType>(x)`.
        if let Some(emitted) = self.try_emit_scalar_from_call(call) {
            return emitted;
        }
        // `assert_equal(A, B)` itself: emit A, then B with the sibling-item context
        // set so empty Vecs inside B adopt A's item type.
        if let Some(emitted) = self.try_emit_assert_equal_call(call) {
            return emitted;
        }
        // Item 8: recursive-nested-fn → Y-combinator call shape. When the
        // call's func is a bare ident matching either the body of the
        // currently-emitting recursive nested fn (use `__self(__self,…)`)
        // or a recursive nested fn declared earlier in the same scope
        // (use `NAME(NAME,…)`), rewrite the call form. This is the
        // emit-side companion to the Y-combinator-shaped signature
        // produced in `emit_nested_function`.
        if let Some(rewritten) = self.try_emit_y_combinator_call(call) {
            return rewritten;
        }
        if let Some(emitted) =
            self.try_emit_slice_full_call_with_expected_array_type(call, expected_ty)
        {
            return emitted;
        }
        if let Some(emitted) =
            self.try_emit_arrayvec_from_repeat_with_fixed_array_arg(call, expected_ty)
        {
            return emitted;
        }
        if let Some(emitted) = self.try_emit_array_from_fn_call_with_expected(call, expected_ty) {
            return emitted;
        }
        if let Some(emitted) = self.try_emit_associated_bytes_constructor_call(call, expected_ty) {
            return emitted;
        }
        // `Box::new(w)` where the EXPECTED type erases the Box entirely
        // (`Box<dyn io::Write + 'a>` → rusty::io::DynWrite): construct the
        // type-erased wrapper directly — a `Box<void*>` can neither own nor
        // dispatch the writer (serde_yaml Emitter::into_inner's sink).
        if let syn::Expr::Path(p) = call.func.as_ref()
            && p.path
                .segments
                .iter()
                .nth_back(1)
                .is_some_and(|s| s.ident == "Box")
            && p.path
                .segments
                .last()
                .is_some_and(|s| matches!(s.ident.to_string().as_str(), "new" | "new_"))
            && call.args.len() == 1
            && let Some(expected) = expected_ty
            && self.map_type(expected) == "rusty::io::DynWrite"
        {
            return format!(
                "rusty::io::DynWrite({})",
                self.emit_expr_maybe_move(&call.args[0])
            );
        }
        if let syn::Expr::Path(path_expr) = call.func.as_ref()
            && call.args.len() == 1
            && path_expr
                .path
                .segments
                .last()
                .is_some_and(|seg| seg.ident == "drop_in_place")
            && let syn::Expr::Reference(reference) = self.peel_paren_group_expr(&call.args[0])
            && self.expr_lowers_to_slice_or_span_view(&reference.expr)
        {
            let mut func = self.emit_call_func_with_owner_template_recovery(call, expected_ty);
            func = self.rewrite_seed_ctor_path_string(&func);
            func = self.maybe_defer_static_owner_lookup_for_path_call(call, func);
            let arg = self.emit_expr_to_string(&reference.expr);
            return format!("{}({})", func, arg);
        }
        if let syn::Expr::Path(path_expr) = call.func.as_ref()
            && call.args.len() == 2
        {
            let joined = path_expr
                .path
                .segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            if matches!(
                joined.as_str(),
                "mem::swap" | "core::mem::swap" | "std::mem::swap" | "rusty::mem::swap"
            ) {
                let mut func = self.emit_call_func_with_owner_template_recovery(call, expected_ty);
                func = self.rewrite_seed_ctor_path_string(&func);
                func = self.maybe_defer_static_owner_lookup_for_path_call(call, func);
                let args: Vec<String> = call
                    .args
                    .iter()
                    .map(|arg| match self.peel_paren_group_expr(arg) {
                        syn::Expr::Reference(reference) => {
                            self.emit_expr_to_string(&reference.expr)
                        }
                        _ => self.emit_expr_maybe_move(arg),
                    })
                    .collect();
                return format!("{}({})", func, args.join(", "));
            }
        }
        if let syn::Expr::Path(path_expr) = call.func.as_ref()
            && call.args.len() == 1
            && path_expr
                .path
                .segments
                .last()
                .is_some_and(|seg| seg.ident == "from_str")
        {
            let mut func = self.emit_call_func_with_owner_template_recovery(call, expected_ty);
            func = self.rewrite_seed_ctor_path_string(&func);
            func = self.maybe_defer_static_owner_lookup_for_path_call(call, func);
            let arg = self.emit_expr_maybe_move(&call.args[0]);
            if matches!(func.as_str(), "from_str" | "::from_str")
                && let Some(ok_ty) = self.expected_from_str_target_type(expected_ty)
            {
                let ok_cpp = self.map_type(&ok_ty);
                if ok_cpp != "auto"
                    && !ok_cpp.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&ok_cpp)
                {
                    return format!("{}<{}>(rusty::to_string_view({}))", func, ok_cpp, arg);
                }
            }
            return format!("{}(rusty::to_string_view({}))", func, arg);
        }
        if let syn::Expr::Path(path_expr) = call.func.as_ref()
            && path_expr.path.segments.last().is_some_and(|seg| {
                matches!(
                    seg.ident.to_string().as_str(),
                    "to_writer" | "to_writer_pretty"
                )
            })
            && call.args.len() >= 2
            && let syn::Expr::Reference(reference) = self.peel_paren_group_expr(&call.args[0])
            && reference.mutability.is_some()
            && self.is_stable_reference_lvalue_expr(&reference.expr)
        {
            let mut func = self.emit_call_func_with_owner_template_recovery(call, expected_ty);
            func = self.rewrite_seed_ctor_path_string(&func);
            func = self.maybe_defer_static_owner_lookup_for_path_call(call, func);
            let mut args = Vec::with_capacity(call.args.len());
            args.push(self.emit_explicit_reference_call_arg(reference, None));
            args.extend(
                call.args
                    .iter()
                    .skip(1)
                    .map(|arg| self.emit_expr_maybe_move(arg)),
            );
            return format!("{}({})", func, args.join(", "));
        }
        if let syn::Expr::Path(path_expr) = call.func.as_ref()
            && call.args.len() == 1
            && path_expr
                .path
                .segments
                .last()
                .is_some_and(|seg| seg.ident == "from_trait")
        {
            let mut func = self.emit_call_func_with_owner_template_recovery(call, expected_ty);
            func = self.rewrite_seed_ctor_path_string(&func);
            func = self.maybe_defer_static_owner_lookup_for_path_call(call, func);
            let arg = self.emit_expr_maybe_move(&call.args[0]);
            if let Some(ok_ty) = expected_ty
                .and_then(|hint| self.expected_result_type_arg(Some(hint), 0))
                .or_else(|| {
                    self.current_return_type_hint()
                        .and_then(|hint| self.expected_result_type_arg(Some(hint), 0))
                })
            {
                let ok_cpp = self.map_type(ok_ty);
                if ok_cpp != "auto"
                    && !ok_cpp.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&ok_cpp)
                {
                    return format!(
                        "{}<std::remove_cvref_t<decltype({})>, {}>({})",
                        func, arg, ok_cpp, arg
                    );
                }
            }
            return format!("{}({})", func, arg);
        }
        if let syn::Expr::Path(path_expr) = call.func.as_ref()
            && call.args.len() == 2
            && path_expr
                .path
                .segments
                .last()
                .is_some_and(|seg| seg.ident == "error")
            && let Some(ok_ty) = expected_ty
                .and_then(|hint| self.expected_result_type_arg(Some(hint), 0))
                .or_else(|| {
                    self.current_return_type_hint()
                        .and_then(|hint| self.expected_result_type_arg(Some(hint), 0))
                })
        {
            let ok_cpp = self.map_type(ok_ty);
            if ok_cpp != "auto"
                && !ok_cpp.contains("/* TODO")
                && !type_string_has_auto_placeholder(&ok_cpp)
            {
                let mut func = self.emit_path_to_string(&path_expr.path);
                func = self.rewrite_seed_ctor_path_string(&func);
                func = self.maybe_defer_static_owner_lookup_for_path_call(call, func);
                let read_arg = self.emit_expr_to_string(&call.args[0]);
                let reason_arg = self.emit_expr_maybe_move(&call.args[1]);
                return format!(
                    "{}<std::remove_cvref_t<decltype({})>, {}>({}, {})",
                    func, read_arg, ok_cpp, read_arg, reason_arg
                );
            }
        }
        if let syn::Expr::Path(path_expr) = call.func.as_ref() {
            let joined = path_expr
                .path
                .segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .collect::<Vec<String>>()
                .join("::");
            if call.args.len() == 2
                && Self::is_serde_error_trait_static_call_path(&path_expr.path, "invalid_type")
            {
                let owner = self.serde_error_trait_static_call_owner_cpp(expected_ty);
                let unexp = self.emit_serde_unexpected_static_call_arg(&call.args[0]);
                let exp = self.emit_expr_maybe_move(&call.args[1]);
                // The concrete owner only has an `invalid_type` member when
                // its impl overrides the trait's provided default; route
                // non-overriding owners to the dep's RuntimeHelper.
                if let Some(dispatch) = self.dep_trait_static_dispatch_call(
                    &owner,
                    "invalid_type",
                    &format!("{}, {}", unexp, exp),
                ) {
                    return dispatch;
                }
                return format!("{}::invalid_type({}, {})", owner, unexp, exp);
            }
            if call.args.len() == 2
                && path_expr
                    .path
                    .segments
                    .last()
                    .is_some_and(|seg| seg.ident == "error")
                && let Some(ok_ty) = expected_ty
                    .and_then(|hint| self.expected_result_type_arg(Some(hint), 0))
                    .or_else(|| {
                        self.current_return_type_hint()
                            .and_then(|hint| self.expected_result_type_arg(Some(hint), 0))
                    })
                    .or_else(|| {
                        self.return_type_hints.iter().rev().find_map(|hint| {
                            hint.as_ref()
                                .and_then(|ty| self.expected_result_type_arg(Some(ty), 0))
                        })
                    })
            {
                let ok_cpp = self.map_type(ok_ty);
                if ok_cpp != "auto"
                    && !ok_cpp.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&ok_cpp)
                {
                    let mut path_no_args = path_expr.path.clone();
                    if let Some(last) = path_no_args.segments.last_mut() {
                        last.arguments = syn::PathArguments::None;
                    }
                    let mut func = self.emit_path_to_string(&path_no_args);
                    func = self.rewrite_seed_ctor_path_string(&func);
                    func = self.maybe_defer_static_owner_lookup_for_path_call(call, func);
                    let read_arg = self.emit_expr_to_string(&call.args[0]);
                    let reason_arg = self.emit_expr_maybe_move(&call.args[1]);
                    return format!(
                        "{}<std::remove_cvref_t<decltype({})>, {}>({}, {})",
                        func, read_arg, ok_cpp, read_arg, reason_arg
                    );
                }
            }
            if call.args.len() == 2
                && path_expr
                    .path
                    .segments
                    .last()
                    .is_some_and(|seg| seg.ident == "error")
            {
                let reason_preview = self.emit_expr_to_string(&call.args[1]);
                if reason_preview.contains("InvalidUnicodeCodePoint") {
                    let mut path_no_args = path_expr.path.clone();
                    if let Some(last) = path_no_args.segments.last_mut() {
                        last.arguments = syn::PathArguments::None;
                    }
                    let mut func = self.emit_path_to_string(&path_no_args);
                    func = self.rewrite_seed_ctor_path_string(&func);
                    func = self.maybe_defer_static_owner_lookup_for_path_call(call, func);
                    let read_arg = self.emit_expr_to_string(&call.args[0]);
                    return format!(
                        "{}<std::remove_cvref_t<decltype({})>, std::string_view>({}, {})",
                        func, read_arg, read_arg, reason_preview
                    );
                }
            }
            if call.args.len() == 3
                && matches!(
                    joined.as_str(),
                    "SerializeMap::serialize_entry"
                        | "ser::SerializeMap::serialize_entry"
                        | "serde::ser::SerializeMap::serialize_entry"
                        | "serde_core::ser::SerializeMap::serialize_entry"
                )
            {
                let receiver = match self.peel_paren_group_expr(&call.args[0]) {
                    syn::Expr::Reference(reference) => self.emit_expr_to_string(&reference.expr),
                    _ => self.emit_expr_to_string(&call.args[0]),
                };
                let key = self.emit_expr_maybe_move(&call.args[1]);
                let value = self.emit_expr_maybe_move(&call.args[2]);
                return format!(
                    "([&]() {{ auto&& __serialize_entry_target = {}; auto __key_res = __serialize_entry_target.serialize_key({}); if (__key_res.is_err()) {{ return __key_res; }} return __serialize_entry_target.serialize_value({}); }}())",
                    receiver, key, value
                );
            }
            if call.args.len() == 1
                && matches!(
                    joined.as_str(),
                    "mem::discriminant" | "std::mem::discriminant" | "core::mem::discriminant"
                )
            {
                let arg = self.emit_expr_to_string(&call.args[0]);
                return format!("rusty::intrinsics::discriminant_value({})", arg);
            }
            if call.args.len() == 1
                && matches!(
                    joined.as_str(),
                    "From::from"
                        | "core::convert::From::from"
                        | "std::convert::From::from"
                        | "Into::into"
                        | "core::convert::Into::into"
                        | "std::convert::Into::into"
                )
                && let Some(target_ty) = expected_ty.or_else(|| self.current_return_type_hint())
            {
                let target_cpp = self.map_type(target_ty);
                if target_cpp != "auto"
                    && !target_cpp.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&target_cpp)
                {
                    let arg = self
                        .emit_expr_to_string_with_expected_and_move_if_needed(&call.args[0], None);
                    return format!("rusty::from_into<{}>({})", target_cpp, arg);
                }
            }
            if call.args.len() == 1
                && matches!(
                    joined.as_str(),
                    "mem::take" | "std::mem::take" | "core::mem::take" | "rusty::mem::take"
                )
            {
                let mut func = self.emit_call_func_with_owner_template_recovery(call, expected_ty);
                func = self.rewrite_seed_ctor_path_string(&func);
                func = self.maybe_defer_static_owner_lookup_for_path_call(call, func);
                let mut arg = match self.peel_paren_group_expr(&call.args[0]) {
                    syn::Expr::Reference(reference) => self.emit_expr_to_string(&reference.expr),
                    _ => self.emit_expr_to_string(&call.args[0]),
                };
                if let Some(stripped) = arg.strip_prefix('&') {
                    arg = stripped.trim_start().to_string();
                }
                return format!("{}({})", func, arg);
            }
            if call.args.len() == 2
                && matches!(
                    joined.as_str(),
                    "fmt::write" | "std::fmt::write" | "core::fmt::write"
                )
            {
                // Rust `fmt::write(&mut w, format_args!(…))` drives `w`'s
                // fmt::Write impl. The module-level `write_fmt` helper is
                // that dispatch (member write_str/write_fmt, formatter
                // fields, or a trait free fn) — route there instead of a
                // nonexistent `rusty::fmt::write_` runtime symbol.
                let mut writer = match self.peel_paren_group_expr(&call.args[0]) {
                    syn::Expr::Reference(reference) => self.emit_expr_to_string(&reference.expr),
                    _ => self.emit_expr_to_string(&call.args[0]),
                };
                if let Some(stripped) = writer.strip_prefix('&') {
                    writer = stripped.trim_start().to_string();
                }
                let fmt_arg = self.emit_expr_to_string(&call.args[1]);
                return format!("write_fmt({}, {})", writer, fmt_arg);
            }
            if call.args.len() == 3
                && matches!(
                    joined.as_str(),
                    "hint::select_unpredictable"
                        | "std::hint::select_unpredictable"
                        | "core::hint::select_unpredictable"
                )
            {
                let condition = self.emit_expr_to_string(&call.args[0]);
                let true_value = self.emit_expr_maybe_move(&call.args[1]);
                let false_value = self.emit_expr_maybe_move(&call.args[2]);
                return format!("({} ? {} : {})", condition, true_value, false_value);
            }
            if call.args.len() == 4
                && matches!(
                    joined.as_str(),
                    "_MM_SHUFFLE"
                        | "_MM_SHUFFLE_"
                        | "stdarch_x86::_MM_SHUFFLE"
                        | "stdarch_x86::_MM_SHUFFLE_"
                )
            {
                let z = self.emit_expr_to_string(&call.args[0]);
                let y = self.emit_expr_to_string(&call.args[1]);
                let x = self.emit_expr_to_string(&call.args[2]);
                let w = self.emit_expr_to_string(&call.args[3]);
                return format!(
                    "static_cast<int32_t>((({}) << 6) | (({}) << 4) | (({}) << 2) | ({}))",
                    z, y, x, w
                );
            }
        }
        if let Some(alias_call) = self.try_emit_type_alias_impl_associated_call(call) {
            return alias_call;
        }
        if let Some(ty) = expected_ty {
            if let Some(emitted) =
                self.try_emit_data_enum_variant_call_with_expected(call, Some(ty))
            {
                return emitted;
            }
            if let Some(emitted) = self.try_emit_iter_either_new_call_with_expected(call, ty) {
                return emitted;
            }
            if let Some(emitted) = self.try_emit_variant_constructor_call_with_expected(call, ty) {
                return emitted;
            }
            if let Some(emitted) = self.try_emit_associated_call_with_expected_type(call, ty) {
                return emitted;
            }
        }
        // Forward element inference for a bare collection constructor whose
        // element type wasn't pinned above: the consuming context (an explicit
        // expected type, or — for a `return`/tail expr — the fn's return type)
        // is a container/iterator `Other<E..>` that SHARES the element with the
        // ctor's collection (`VecDeque::new()` returning `VecDequeIntoIter<E>`).
        // Synthesize the matching `Coll<E..>` and reuse the proven expected-type
        // path so `VecDeque::new_()` emits `VecDeque<E>::new_()`.
        if let Some(forward_ty) = expected_ty.or_else(|| self.current_return_type_hint())
            && let Some(emitted) =
                self.try_emit_collection_ctor_with_forward_element(call, forward_ty)
        {
            return emitted;
        }
        // For Ok/Err calls inside closures, the closure's return type hint may be more
        // relevant than any outer expected type (closure return type overrides).
        let ok_err_hint = if expected_ty.is_none() {
            self.current_return_type_hint()
        } else {
            None
        };
        let raw_resolved_hint = expected_ty.or(ok_err_hint);
        let contextual_hint = self.current_return_type_hint();
        let resolved_hint_owned = raw_resolved_hint.map(|hint| {
            if self.type_contains_infer(hint) {
                if let Some(ctx) = contextual_hint {
                    self.resolve_type_infers_from_expected(hint, ctx)
                } else {
                    hint.clone()
                }
            } else {
                hint.clone()
            }
        });
        let resolved_hint = resolved_hint_owned.as_ref();
        if let Some(emitted) = self.try_emit_data_enum_variant_call_with_expected(
            call,
            expected_ty.or(self.current_return_type_hint()),
        ) {
            return emitted;
        }
        let result_ctor_with_infer_turbofish =
            if let syn::Expr::Path(path_expr) = call.func.as_ref() {
                path_expr.path.segments.last().and_then(|last| {
                    let ctor_name = last.ident.to_string();
                    if !matches!(ctor_name.as_str(), "Ok" | "Err") {
                        return None;
                    }
                    let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
                        return None;
                    };
                    let has_infer_placeholder = args.args.iter().any(|arg| match arg {
                        syn::GenericArgument::Type(t) => self.type_contains_infer(t),
                        syn::GenericArgument::Const(c) => matches!(c, syn::Expr::Infer(_)),
                        _ => false,
                    });
                    if has_infer_placeholder {
                        Some((ctor_name, args.clone()))
                    } else {
                        None
                    }
                })
            } else {
                None
            };
        let resolved_ctor_hint_owned =
            result_ctor_with_infer_turbofish
                .as_ref()
                .and_then(|(_, ctor_args)| {
                    let mut type_args: Vec<syn::Type> = ctor_args
                        .args
                        .iter()
                        .filter_map(|arg| match arg {
                            syn::GenericArgument::Type(t) => Some(t.clone()),
                            _ => None,
                        })
                        .collect();
                    if type_args.len() != 2 {
                        return None;
                    }
                    for (idx, ty) in type_args.iter_mut().enumerate() {
                        if matches!(ty, syn::Type::Infer(_)) {
                            let replacement = resolved_hint
                                .and_then(|hint| self.expected_result_type_arg(Some(hint), idx))
                                .cloned()?;
                            *ty = replacement;
                        }
                    }
                    let ok_ty = type_args.first()?.clone();
                    let err_ty = type_args.get(1)?.clone();
                    Some(parse_quote!(Result<#ok_ty, #err_ty>))
                });
        let effective_resolved_hint = resolved_ctor_hint_owned.as_ref().or(resolved_hint);
        let can_resolve_result_ctor_infer =
            result_ctor_with_infer_turbofish.as_ref().is_some_and(|_| {
                effective_resolved_hint
                    .is_some_and(|hint| !self.type_contains_unresolved_placeholder_like(hint))
            });
        if let Some((ctor_name, _)) = result_ctor_with_infer_turbofish.as_ref()
            && effective_resolved_hint
                .is_some_and(|hint| self.type_contains_unresolved_placeholder_like(hint))
        {
            panic!(
                "unresolved placeholder in {} turbofish without concrete expected Result context",
                ctor_name
            );
        }
        if let Some(emitted) = self.try_emit_variant_constructor_call_with_recovered_hints(call) {
            return emitted;
        }

        if let syn::Expr::Path(func_path) = call.func.as_ref() {
            if call.args.len() == 1 && self.path_is_from_iterator_from_iter(&func_path.path) {
                let arg = self.emit_expr_maybe_move(&call.args[0]);
                if let Some(target_ty) = expected_ty.or(self.current_return_type_hint()) {
                    let mut target_cpp =
                        self.map_type(self.peel_reference_paren_group_type(target_ty));
                    if let Some(explicit) =
                        self.recover_explicit_owner_type_from_type(target_ty, &target_cpp)
                    {
                        target_cpp = explicit;
                    }
                    if !target_cpp.contains("/* TODO")
                        && !type_string_has_auto_placeholder(&target_cpp)
                    {
                        return format!("{}::from_iter({})", target_cpp, arg);
                    }
                }
            }
        }

        if let Some(cow_ctor) = self.try_emit_runtime_cow_variant_ctor_call(call) {
            return cow_ctor;
        }

        // General data enum variant constructor: `EnumName::VariantName(args)`
        // → `EnumName_VariantName{args}` when EnumName is a known data enum.
        if let Some(variant_ctor) = self.try_emit_data_enum_variant_constructor(call) {
            return variant_ctor;
        }
        if let Some(c_like_variant) = self.try_emit_c_like_enum_variant_zero_arg_call(call) {
            return c_like_variant;
        }

        // Lower fully-qualified std iterator-trait UFCS calls produced by
        // itertools' macros (`$crate::__std_iter::Iterator::map(into_iter(x), f)`)
        // into receiver-method form. These pass the receiver by value, so the
        // `&receiver` UFCS handler below won't catch them.
        if let Some(iter_call) = self.try_emit_std_iter_trait_ufcs_call(call) {
            return iter_call;
        }

        // Intercept derived-trait UFCS calls with arbitrary arg shapes
        // (including `std::move(arg)` from expanded derive code) that the
        // main UFCS handler below won't catch (it requires `&receiver`).
        if let Some(trait_call) = self.try_emit_known_trait_ufcs_call(call) {
            return trait_call;
        }
        if let Some(trait_call) = self.try_emit_trait_ufcs_by_value_receiver_call(call) {
            return trait_call;
        }
        if let syn::Expr::Path(path_expr) = call.func.as_ref() {
            let segments: Vec<String> = path_expr
                .path
                .segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .collect();
            let joined = segments.join("::");
            if call.args.len() == 2
                && matches!(
                    joined.as_str(),
                    "Display::fmt"
                        | "fmt::Display::fmt"
                        | "core::fmt::Display::fmt"
                        | "std::fmt::Display::fmt"
                        | "rusty::fmt::Display::fmt"
                )
            {
                let value = match self.peel_paren_group_expr(&call.args[0]) {
                    syn::Expr::Reference(r) => self.emit_expr_to_string(&r.expr),
                    _ => self.emit_expr_maybe_move(&call.args[0]),
                };
                let formatter = match self.peel_paren_group_expr(&call.args[1]) {
                    syn::Expr::Reference(r) => self.emit_expr_to_string(&r.expr),
                    _ => self.emit_expr_maybe_move(&call.args[1]),
                };
                return format!(
                    "rusty::write_fmt({}, rusty::to_string({}))",
                    formatter, value
                );
            }
            if call.args.len() == 2
                && matches!(
                    joined.as_str(),
                    "Debug::fmt"
                        | "fmt::Debug::fmt"
                        | "core::fmt::Debug::fmt"
                        | "std::fmt::Debug::fmt"
                        | "rusty::fmt::Debug::fmt"
                )
            {
                let value = match self.peel_paren_group_expr(&call.args[0]) {
                    syn::Expr::Reference(r) => self.emit_expr_to_string(&r.expr),
                    _ => self.emit_expr_maybe_move(&call.args[0]),
                };
                let formatter = match self.peel_paren_group_expr(&call.args[1]) {
                    syn::Expr::Reference(r) => self.emit_expr_to_string(&r.expr),
                    _ => self.emit_expr_maybe_move(&call.args[1]),
                };
                return format!(
                    "rusty::write_fmt({}, rusty::to_debug_string({}))",
                    formatter, value
                );
            }
            if call.args.len() == 3
                && matches!(
                    joined.as_str(),
                    "SerializeMap::serialize_entry"
                        | "ser::SerializeMap::serialize_entry"
                        | "serde::ser::SerializeMap::serialize_entry"
                        | "serde_core::ser::SerializeMap::serialize_entry"
                )
            {
                let receiver = match self.peel_paren_group_expr(&call.args[0]) {
                    syn::Expr::Reference(reference) => self.emit_expr_to_string(&reference.expr),
                    _ => self.emit_expr_to_string(&call.args[0]),
                };
                let key = self.emit_expr_maybe_move(&call.args[1]);
                let value = self.emit_expr_maybe_move(&call.args[2]);
                return format!(
                    "([&]() {{ auto&& __serialize_entry_target = {}; auto __key_res = __serialize_entry_target.serialize_key({}); if (__key_res.is_err()) {{ return __key_res; }} return __serialize_entry_target.serialize_value({}); }}())",
                    receiver, key, value
                );
            }
            let mut owner_path = syn::Path {
                leading_colon: path_expr.path.leading_colon,
                segments: syn::punctuated::Punctuated::new(),
            };
            for seg in path_expr
                .path
                .segments
                .iter()
                .take(path_expr.path.segments.len().saturating_sub(1))
            {
                owner_path.segments.push(seg.clone());
            }
            let owner_name = owner_path
                .segments
                .last()
                .map(|seg| seg.ident.to_string())
                .unwrap_or_default();
            let method_name = segments.last().cloned().unwrap_or_default();
            let deserializer_trait_style_method = method_name.starts_with("deserialize");
            let owner_receiver_shape = self.lookup_owner_method_has_receiver_from_owner_path(
                Some(&owner_path),
                &owner_name,
                &method_name,
            );
            if segments.len() >= 2
                && segments
                    .get(segments.len().saturating_sub(2))
                    .is_some_and(|seg| seg == "Deserializer")
                && !deserializer_trait_style_method
                && owner_receiver_shape != Some(false)
                && owner_receiver_shape
                    .or_else(|| self.trait_static_call_has_receiver_for_segments(&segments))
                    .unwrap_or(true)
                && !call.args.is_empty()
            {
                let method = segments
                    .last()
                    .expect("segments.len() >= 2 implies non-empty")
                    .to_string();
                let args: Vec<String> = call
                    .args
                    .iter()
                    .skip(1)
                    .map(|arg| self.emit_expr_maybe_move(arg))
                    .collect();
                return self.emit_receiver_member_call(&call.args[0], &method, None, &args, None);
            }
        }

        // Phase 18 Blocker 2 (leaf 2): Rewrite UFCS trait-method calls from:
        // `Trait::method(&receiver, args...)` to `receiver.method(args...)`.
        if let Some(ufcs) = self.detect_ufcs_trait_method_call(call) {
            if let Some(syn::Expr::Reference(receiver_ref)) = call.args.first() {
                let receiver = self.emit_expr_to_string(&receiver_ref.expr);
                let args: Vec<String> = call
                    .args
                    .iter()
                    .skip(1)
                    .map(|a| match a {
                        // For UFCS-to-method rewrite, non-receiver reference args are
                        // method arguments, not C++ address-of operations.
                        syn::Expr::Reference(r) => self.emit_expr_to_string(&r.expr),
                        _ => self.emit_expr_maybe_move(a),
                    })
                    .collect();
                if ufcs.method_name == "serialize" && args.len() == 1 {
                    return self.emit_serialize_dispatch_call(&receiver, &args[0]);
                }
                if ufcs.method_name == "deserialize_any" && args.len() == 1 {
                    return format!(
                        "::de::rusty_ext::deserialize_any({}, {})",
                        receiver, args[0]
                    );
                }
                if ufcs.method_name == "deserialize_in_place" && args.len() == 1 {
                    return format!(
                        "::de::rusty_ext::deserialize_in_place({}, {})",
                        receiver, args[0]
                    );
                }
                if self.is_ufcs_io_write_fmt_call_path(&ufcs.function_path) && args.len() == 1 {
                    return format!("rusty::io::write_fmt({}, {})", receiver, args[0]);
                }
                if args.len() == 1
                    && matches!(
                        ufcs.function_path.as_str(),
                        "Display::fmt"
                            | "fmt::Display::fmt"
                            | "core::fmt::Display::fmt"
                            | "std::fmt::Display::fmt"
                            | "rusty::fmt::Display::fmt"
                    )
                {
                    return format!(
                        "rusty::write_fmt({}, rusty::to_string({}))",
                        args[0], receiver
                    );
                }
                if args.len() == 1
                    && matches!(
                        ufcs.function_path.as_str(),
                        "Debug::fmt"
                            | "fmt::Debug::fmt"
                            | "core::fmt::Debug::fmt"
                            | "std::fmt::Debug::fmt"
                            | "rusty::fmt::Debug::fmt"
                    )
                {
                    return format!(
                        "rusty::write_fmt({}, rusty::to_debug_string({}))",
                        args[0], receiver
                    );
                }
                // Intercept derived-trait UFCS calls that map to rusty:: free
                // functions.  Without this, `Clone::clone(&self.field)` would
                // emit `this->field.clone()` which fails on C++ primitives.
                match (ufcs.method_name.as_str(), args.len()) {
                    ("clone", 0) => {
                        // `Clone::clone(&self.field)` where `self.field` is itself a reference
                        // (Rust type `&T`) should clone the reference (`&T`), not the pointee
                        // (`T`). Emitting `rusty::clone(field_ref)` would call `T::clone()`
                        // and materialize a temporary, which can dangle for reference fields.
                        let preserve_reference_identity = self
                            .infer_simple_expr_type(&receiver_ref.expr)
                            .as_ref()
                            .is_some_and(|ty| {
                                let ty = self.peel_reference_paren_group_type(ty);
                                if let syn::Type::Reference(r) = ty {
                                    self.type_is_reference_like(&r.elem)
                                } else {
                                    false
                                }
                            });
                        if preserve_reference_identity {
                            return receiver;
                        }
                        return format!("rusty::clone({})", receiver);
                    }
                    ("cmp", 1) => return format!("rusty::cmp::cmp({}, {})", receiver, args[0]),
                    ("partial_cmp", 1) => {
                        return format!("rusty::partial_cmp({}, {})", receiver, args[0]);
                    }
                    ("hash", 1) => return format!("rusty::hash::hash({}, {})", receiver, args[0]),
                    _ => {}
                }
                // UFCS qualified disambiguation (book § 3.2.3): a disambiguated
                // trait call `Trait::method(&recv, …)` / `<T as Trait>::method(
                // &recv, …)` lowers to the qualified free function
                // `<Trait>_::method(recv, …)`. This is the ONLY correct
                // route when one type implements two traits sharing a method
                // name — the member call `recv.method()` collapses to whichever
                // impl won the struct's single member slot. Gate on the owner
                // map (a CONCRETE impl of THIS trait actually emits the free
                // function), not just "crate-declared": default trait methods
                // and runtime-helper (assoc-const) traits have no
                // `<Trait>_::m`, so qualifying them is a HARD error — fall
                // through to the member call instead.
                if let Some(trait_name) = ufcs.function_path.rsplit("::").nth(1)
                    && self
                        .ufcs_method_trait_owners
                        .get(&ufcs.method_name)
                        .is_some_and(|owners| owners.contains(trait_name))
                {
                    let mut all_args = Vec::with_capacity(args.len() + 1);
                    all_args.push(receiver.clone());
                    all_args.extend(args.iter().cloned());
                    return format!(
                        "{}::{}({})",
                        self.ufcs_trait_namespace(trait_name),
                        escape_cpp_keyword(&ufcs.method_name),
                        all_args.join(", ")
                    );
                }
                let is_self = matches!(
                    receiver_ref.expr.as_ref(),
                    syn::Expr::Path(p)
                        if p.path.segments.len() == 1 && p.path.segments[0].ident == "self"
                );
                if is_self {
                    return format!("{}({})", ufcs.method_name, args.join(", "));
                }
                return format!("{}.{}({})", receiver, ufcs.method_name, args.join(", "));
            }
        }

        // Handle `Type::method(value, ...)` static-style calls to C-like enum
        // inherent methods by routing them to the generated free-function form.
        if let syn::Expr::Path(func_path) = call.func.as_ref() {
            if func_path.path.segments.len() >= 2 && !call.args.is_empty() {
                let owner_segments: Vec<String> = func_path
                    .path
                    .segments
                    .iter()
                    .take(func_path.path.segments.len() - 1)
                    .map(|seg| seg.ident.to_string())
                    .collect();
                let owner_path = owner_segments.join("::");
                let owner_tail = owner_segments.last().cloned().unwrap_or_default();
                let method_name = func_path
                    .path
                    .segments
                    .last()
                    .expect("segments.len() >= 2")
                    .ident
                    .to_string();
                let owner_is_serialize_trait =
                    owner_tail == "Serialize" || owner_path.ends_with("::Serialize");
                if owner_is_serialize_trait && method_name == "serialize" && call.args.len() == 2 {
                    let value = self.emit_expr_to_string(&call.args[0]);
                    let serializer = self.emit_expr_maybe_move(&call.args[1]);
                    return self.emit_serialize_dispatch_call(&value, &serializer);
                }
                // UpperCamelCase call names are typically enum-variant or
                // constructor-like associated calls (`Type::Variant(...)`).
                // Never rewrite those to receiver-form (`value.Variant()`).
                let method_is_variant_like = method_name
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_uppercase());
                let method_template_args = self.emit_expr_path_template_args(&func_path.path);
                let method_has_receiver = self
                    .lookup_owner_method_has_receiver(&owner_path, &method_name)
                    .or_else(|| self.lookup_owner_method_has_receiver(&owner_tail, &method_name));
                let is_runtime_helper_method = (self.module_name.is_some()
                    || self.expanded_libtest_mode)
                    && (self.module_runtime_helper_traits.contains(&owner_tail)
                        || self.module_runtime_helper_traits.contains(&owner_path));
                if matches!(method_has_receiver, Some(true))
                    && !is_runtime_helper_method
                    && !method_is_variant_like
                {
                    if let Some(c_like_callee) = self
                        .resolve_c_like_enum_inherent_method_free_function_for_owner_path(
                            &owner_path,
                            &method_name,
                        )
                        .or_else(|| {
                            self.resolve_c_like_enum_inherent_method_free_function_for_owner_path(
                                &owner_tail,
                                &method_name,
                            )
                        })
                    {
                        let ufcs_args: Vec<String> = call
                            .args
                            .iter()
                            .map(|arg| self.emit_expr_maybe_move(arg))
                            .collect();
                        return format!("{}({})", c_like_callee, ufcs_args.join(", "));
                    }
                    let receiver = &call.args[0];
                    let member_args: Vec<String> = call
                        .args
                        .iter()
                        .skip(1)
                        .map(|arg| self.emit_expr_maybe_move(arg))
                        .collect();
                    return self.emit_receiver_member_call(
                        receiver,
                        &method_name,
                        method_template_args.as_deref(),
                        &member_args,
                        None,
                    );
                }
            }
        }

        // Handle `Type::method(self)` / `Type::method(value)` UFCS where the
        // first arg is NOT a reference — convert to `receiver.method()`.
        // Catches patterns like `TestFlags::bits(self)` inside impl methods.
        if let syn::Expr::Path(func_path) = call.func.as_ref() {
            if func_path.path.segments.len() >= 2 && call.args.len() == 1 {
                let owner_segments: Vec<String> = func_path
                    .path
                    .segments
                    .iter()
                    .take(func_path.path.segments.len() - 1)
                    .map(|seg| seg.ident.to_string())
                    .collect();
                let owner_path = owner_segments.join("::");
                let owner_tail = owner_segments.last().cloned().unwrap_or_default();
                let method_name = func_path
                    .path
                    .segments
                    .last()
                    .expect("segments.len() >= 2")
                    .ident
                    .to_string();
                let method_has_receiver = self
                    .lookup_owner_method_has_receiver(&owner_path, &method_name)
                    .or_else(|| self.lookup_owner_method_has_receiver(&owner_tail, &method_name));
                // Only convert if the type matches the current struct (self-call)
                // and the method is not a known constructor/static method.
                let is_current_struct = self.current_struct.as_deref() == Some(owner_tail.as_str());
                let is_constructor = matches!(
                    method_name.as_str(),
                    "new"
                        | "new_"
                        | "from"
                        | "try_from"
                        | "from_slice"
                        | "from_vec"
                        | "empty"
                        | "all"
                        | "from_bits"
                        | "from_bits_retain"
                        | "from_bits_truncate"
                        | "from_name"
                        | "from_str"
                        | "default_"
                        | "from_iter"
                );
                let arg = &call.args[0];
                if !is_constructor
                    && !matches!(method_has_receiver, Some(false))
                    && let Some(arg_ty) = self.infer_simple_expr_type(arg)
                {
                    let arg_owner = match self.peel_reference_paren_group_type(&arg_ty) {
                        syn::Type::Path(tp) => {
                            tp.path.segments.last().map(|seg| seg.ident.to_string())
                        }
                        _ => None,
                    };
                    let owner_mismatch = arg_owner
                        .as_deref()
                        .is_some_and(|owner| owner != owner_tail && owner != "Self");
                    if owner_mismatch
                        && let Some(c_like_callee) = self
                            .resolve_c_like_enum_inherent_method_free_function(arg, &method_name)
                    {
                        let mut ufcs_args = Vec::with_capacity(call.args.len());
                        ufcs_args.push(self.emit_expr_to_string(arg));
                        ufcs_args.extend(
                            call.args
                                .iter()
                                .skip(1)
                                .map(|extra| self.emit_expr_maybe_move(extra)),
                        );
                        return format!("{}({})", c_like_callee, ufcs_args.join(", "));
                    }
                }
                let arg_is_self_like = {
                    let peeled_arg = self.peel_paren_group_expr(arg);
                    matches!(peeled_arg, syn::Expr::Path(path)
                        if path.path.segments.len() == 1 && path.path.segments[0].ident == "self")
                        || self
                            .infer_simple_expr_type(peeled_arg)
                            .is_some_and(|ty| self.type_is_current_struct_self_type(&ty))
                };
                if is_current_struct
                    && !is_constructor
                    && !matches!(method_has_receiver, Some(false))
                    && arg_is_self_like
                {
                    let receiver = match arg {
                        syn::Expr::Reference(r) => self.emit_expr_to_string(&r.expr),
                        _ => self.emit_expr_to_string(arg),
                    };
                    return format!("{}.{}()", receiver, escape_cpp_keyword(&method_name));
                }
            }
        }

        // QSelf associated parse helper: `<T>::parse_hex(x)` should lower to
        // runtime helper form, because C++ primitive/storage types do not have
        // static member parse surfaces.
        if let syn::Expr::Path(func_path) = call.func.as_ref() {
            if let Some(qself) = &func_path.qself {
                if func_path.path.segments.len() == 1 && call.args.len() == 1 {
                    let helper = match func_path.path.segments[0].ident.to_string().as_str() {
                        "cast_mut" => Some("rusty::ptr::cast_mut"),
                        "cast_const" => Some("rusty::ptr::cast_const"),
                        _ => None,
                    };
                    if let Some(helper) = helper {
                        let arg = self.emit_expr_to_string(&call.args[0]);
                        return format!("{}({})", helper, arg);
                    }
                }
                if func_path.path.segments.len() == 1
                    && func_path.path.segments[0].ident == "parse_hex"
                    && call.args.len() == 1
                {
                    let target_cpp = self.map_type(self.peel_reference_paren_group_type(&qself.ty));
                    if target_cpp != "auto" && !target_cpp.contains("/* TODO") {
                        let input = self.emit_expr_maybe_move(&call.args[0]);
                        return format!("rusty::parse_hex<{}>({})", target_cpp, input);
                    }
                }
            }
        }
        // Lower static-style Box::from_raw(ptr) to receiver-form into_raw().
        // This avoids fragile owner-template recovery when the pointer argument
        // type is only available as a placeholder-like surface.
        if let syn::Expr::Path(func_path) = call.func.as_ref() {
            if call.args.len() == 1 && func_path.path.segments.len() >= 2 {
                let owner = func_path.path.segments.iter().nth_back(1);
                let method = func_path.path.segments.last();
                if matches!(
                    owner.map(|seg| seg.ident.to_string()).as_deref(),
                    Some("Box")
                ) && matches!(
                    method.map(|seg| seg.ident.to_string()).as_deref(),
                    Some("from_raw")
                ) {
                    // Infer template arg from the pointer argument's pointee type.
                    let inferred = call
                        .args
                        .first()
                        .and_then(|arg| self.infer_hint_type_from_expr(arg))
                        .and_then(|arg_ty| {
                            let arg_ty = self.peel_reference_paren_group_type(&arg_ty);
                            match arg_ty {
                                // T* → Box<T>
                                syn::Type::Ptr(ptr) => Some(self.map_type(&ptr.elem)),
                                _ => None,
                            }
                        })
                        .or_else(|| {
                            // Fallback: try decltype of the argument expression
                            call.args.first().map(|arg| {
                                let arg_cpp = self.emit_expr_to_string(arg);
                                format!("std::remove_pointer_t<std::remove_reference_t<decltype(({}))>>", arg_cpp)
                            })
                        });
                    if let Some(inner) = inferred {
                        let ptr_arg = self.emit_expr_maybe_move(&call.args[0]);
                        let boxed_cpp = format!("rusty::Box<{}>", inner);
                        return format!("{}::from_raw({})", boxed_cpp, ptr_arg);
                    }
                    // Fallback: emit with the pointer argument as-is
                    let ptr_arg = self.emit_expr_maybe_move(&call.args[0]);
                    return format!("rusty::Box::from_raw({})", ptr_arg);
                }
            }
        }
        // Lower static-style Box::into_raw(value) to receiver-form into_raw().
        // This avoids fragile owner-template recovery when the value expression
        // type is only available as a local placeholder-like surface.
        if let syn::Expr::Path(func_path) = call.func.as_ref() {
            if call.args.len() == 1 && func_path.path.segments.len() >= 2 {
                let owner = func_path.path.segments.iter().nth_back(1);
                let method = func_path.path.segments.last();
                if matches!(
                    owner.map(|seg| seg.ident.to_string()).as_deref(),
                    Some("Box")
                ) && matches!(
                    method.map(|seg| seg.ident.to_string()).as_deref(),
                    Some("into_raw")
                ) {
                    let receiver = self.emit_expr_maybe_move(&call.args[0]);
                    return format!("({}).into_raw()", receiver);
                }
            }
        }
        // Same shape for `Box::leak(value)` — mirror the static-to-receiver
        // transformation so we don't emit `rusty::Box<auto>::leak(...)` (which
        // is syntactically invalid in C++ because `auto` cannot appear as a
        // template argument). `rusty::Box::leak` is a member method on Box
        // that returns the raw pointer and nulls out the source.
        if let syn::Expr::Path(func_path) = call.func.as_ref() {
            if call.args.len() == 1 && func_path.path.segments.len() >= 2 {
                let owner = func_path.path.segments.iter().nth_back(1);
                let method = func_path.path.segments.last();
                if matches!(
                    owner.map(|seg| seg.ident.to_string()).as_deref(),
                    Some("Box")
                ) && matches!(
                    method.map(|seg| seg.ident.to_string()).as_deref(),
                    Some("leak")
                ) {
                    let receiver = self.emit_expr_maybe_move(&call.args[0]);
                    return format!("({}).leak()", receiver);
                }
            }
        }
        // Rust generic associated static-style size call shape (`A::size()`).
        // For type parameters bound to array-like traits this is not a valid C++
        // static member call on `std::array`, so lower to a shared type-level helper.
        if let syn::Expr::Path(func_path) = call.func.as_ref() {
            if func_path.qself.is_none()
                && call.args.is_empty()
                && func_path.path.segments.len() == 2
                && func_path.path.segments[1].ident == "size"
                && matches!(
                    func_path.path.segments[0].arguments,
                    syn::PathArguments::None
                )
            {
                let owner_name = func_path.path.segments[0].ident.to_string();
                if self.is_type_param_in_scope(&owner_name) {
                    return format!(
                        "rusty::detail::type_level_size<{}>()",
                        escape_cpp_keyword(&owner_name)
                    );
                }
            }
        }
        if let syn::Expr::Path(func_path) = call.func.as_ref()
            && call.args.len() == 1
            && let Some(owner_path) = self.type_param_static_conversion_owner_path(&func_path.path)
        {
            let owner_ty = syn::Type::Path(syn::TypePath {
                qself: None,
                path: owner_path,
            });
            let target_cpp = self.map_type(&owner_ty);
            let arg = self.emit_expr_maybe_move(&call.args[0]);
            return format!("static_cast<{}>({})", target_cpp, arg);
        }
        // Primitive scalar conversion constructors (e.g. `u32::from(x)`) lower to casts.
        if let syn::Expr::Path(func_path) = call.func.as_ref() {
            if func_path.qself.is_none()
                && call.args.len() <= 2
                && func_path.path.segments.len() >= 2
            {
                let method_name = func_path
                    .path
                    .segments
                    .last()
                    .map(|seg| seg.ident.to_string());
                let owner_name = func_path
                    .path
                    .segments
                    .iter()
                    .nth_back(1)
                    .map(|seg| seg.ident.to_string());
                let target_cpp = owner_name.as_deref().and_then(|owner_name| {
                    rust_primitive_cast_target_cpp_type(owner_name)
                        .map(str::to_string)
                        .or_else(|| self.numeric_type_aliases.get(owner_name).cloned())
                });
                if method_name.as_deref() == Some("from")
                    && let Some(target_cpp) = target_cpp.as_deref()
                {
                    let arg = self.emit_expr_maybe_move(&call.args[0]);
                    return format!("static_cast<{}>({})", target_cpp, arg);
                }
                if matches!(
                    method_name.as_deref(),
                    Some("from_le_bytes" | "from_be_bytes" | "from_ne_bytes")
                ) && call.args.len() == 1
                    && let Some(target_cpp) = target_cpp.as_deref()
                {
                    let arg_expr = &call.args[0];
                    let bytes_arg = match self.peel_paren_group_expr(arg_expr) {
                        syn::Expr::MethodCall(unwrap_mc)
                            if unwrap_mc.method == "unwrap" && unwrap_mc.args.is_empty() =>
                        {
                            if let syn::Expr::MethodCall(try_into_mc) =
                                self.peel_paren_group_expr(&unwrap_mc.receiver)
                                && try_into_mc.method == "try_into"
                                && try_into_mc.args.is_empty()
                            {
                                self.emit_expr_to_string(&try_into_mc.receiver)
                            } else {
                                self.emit_expr_maybe_move(arg_expr)
                            }
                        }
                        _ => self.emit_expr_maybe_move(arg_expr),
                    };
                    return format!(
                        "rusty::{}<{}>({})",
                        method_name.as_deref().unwrap(),
                        target_cpp,
                        bytes_arg
                    );
                }
                // Primitive `Ord::max`/`min` UFCS path-calls, e.g.
                // `usize::max(a, b)` -> `rusty::max(a, b)`. Two args
                // disambiguate the Ord method from the `MAX`/`MIN` associated
                // consts (which are paths, not calls).
                if matches!(method_name.as_deref(), Some("max" | "min"))
                    && call.args.len() == 2
                    && target_cpp.is_some()
                {
                    let helper = if method_name.as_deref() == Some("min") {
                        "rusty::min"
                    } else {
                        "rusty::max"
                    };
                    return format!(
                        "{}({}, {})",
                        helper,
                        self.emit_expr_maybe_move(&call.args[0]),
                        self.emit_expr_maybe_move(&call.args[1])
                    );
                }
                if method_name.as_deref() == Some("try_from")
                    && call.args.len() == 1
                    && let Some(target_cpp) = target_cpp.as_deref()
                {
                    let arg = self.emit_expr_maybe_move(&call.args[0]);
                    return format!("rusty::try_from<{}>({})", target_cpp, arg);
                }
                if method_name.as_deref() == Some("from_str_radix")
                    && call.args.len() == 2
                    && let Some(owner_name) = owner_name.as_deref()
                    && let Some(target_cpp) = rust_primitive_cast_target_cpp_type(owner_name)
                {
                    let input = self.emit_expr_maybe_move(&call.args[0]);
                    let radix = self.emit_expr_maybe_move(&call.args[1]);
                    return format!(
                        "rusty::from_str_radix<{}>({}, {})",
                        target_cpp, input, radix
                    );
                }
                // Deprecated `i64::max_value()` / `u64::min_value()` — the
                // zero-arg method forms of the `MAX`/`MIN` associated consts
                // (still used by serde_yaml's integer parsing). For integer
                // primitives, `numeric_limits::min()` equals Rust's `MIN`.
                if matches!(method_name.as_deref(), Some("max_value" | "min_value"))
                    && call.args.is_empty()
                    && let Some(owner_name) = owner_name.as_deref()
                    && let Some(target_cpp) = rust_primitive_cast_target_cpp_type(owner_name)
                {
                    let limit = if method_name.as_deref() == Some("min_value") {
                        "min"
                    } else {
                        "max"
                    };
                    return format!("std::numeric_limits<{}>::{}()", target_cpp, limit);
                }
            }
        }
        if let Some(emitted) = self.try_emit_arc_from_assoc_call(call, expected_ty) {
            return emitted;
        }
        if let Some(emitted) = self.try_emit_dep_trait_static_default_call(call) {
            return emitted;
        }

        let mut func = self.emit_call_func_with_owner_template_recovery(call, expected_ty);
        func = self.rewrite_seed_ctor_path_string(&func);
        func = self.maybe_defer_static_owner_lookup_for_path_call(call, func);
        // Boxing INTO `void*` can never own or dispatch the value — the
        // dyn-erased hint (`Box<dyn io::Write>` → element void*) leaked into
        // the owner recovery. Fall back to the argument's own type; the
        // resulting Box converts to the erased target (rusty::io::DynWrite's
        // universal ctor) at the use site.
        if func.contains("Box<void*>")
            && call.args.len() == 1
            && matches!(
                call.func.as_ref(),
                syn::Expr::Path(p) if p.path.segments.last()
                    .is_some_and(|s| matches!(s.ident.to_string().as_str(), "new" | "new_"))
            )
        {
            let arg_cpp = self.emit_expr_to_string(&call.args[0]);
            func = format!(
                "rusty::Box<std::remove_cvref_t<decltype(({}))>>::new_",
                arg_cpp
            );
        }
        if matches!(func.as_str(), "new" | "new_")
            && let syn::Expr::Path(path_expr) = call.func.as_ref()
            && path_expr.path.segments.len() >= 2
        {
            // Keep associated constructor owners when recovery heuristics collapse
            // to an unqualified `new_` inside template-heavy deserializer code.
            let fallback = path_expr
                .path
                .segments
                .iter()
                .map(|seg| escape_cpp_keyword(&seg.ident.to_string()))
                .collect::<Vec<_>>()
                .join("::");
            if !fallback.is_empty() {
                func = if path_expr.path.leading_colon.is_some() {
                    format!("::{}", fallback)
                } else {
                    fallback
                };
                func = self.rewrite_seed_ctor_path_string(&func);
                func = self.maybe_defer_static_owner_lookup_for_path_call(call, func);
            }
        }
        func = match func.as_str() {
            "std::string_view::from_utf8" | "::std::string_view::from_utf8" => {
                "rusty::str_runtime::from_utf8".to_string()
            }
            "String::from_utf8_unchecked"
            | "::String::from_utf8_unchecked"
            | "rusty::String::from_utf8_unchecked"
            | "::rusty::String::from_utf8_unchecked" => {
                "rusty::str_runtime::from_utf8_unchecked".to_string()
            }
            "std::string_view::from_utf8_unchecked" | "::std::string_view::from_utf8_unchecked" => {
                "rusty::str_runtime::from_utf8_unchecked".to_string()
            }
            "std::string_view::from_utf8_unchecked_mut"
            | "::std::string_view::from_utf8_unchecked_mut" => {
                "rusty::str_runtime::from_utf8_unchecked_mut".to_string()
            }
            _ => func,
        };
        if call.args.len() == 1 {
            let func_leaf = func.rsplit("::").next().unwrap_or_default();
            let unresolved_box_ctor = matches!(func_leaf, "new" | "new_" | "make")
                && (matches!(
                    func.as_str(),
                    "Box::new"
                        | "Box::new_"
                        | "Box::make"
                        | "rusty::Box::new"
                        | "rusty::Box::new_"
                        | "rusty::Box::make"
                ) || func.contains("Box<auto>::"));
            if unresolved_box_ctor {
                let box_inner_expected_ty = expected_ty.and_then(|ty| {
                    let ty = self.peel_reference_paren_group_type(ty);
                    let syn::Type::Path(tp) = ty else {
                        return None;
                    };
                    let last = tp.path.segments.last()?;
                    if last.ident != "Box" {
                        return None;
                    }
                    let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
                        return None;
                    };
                    args.args.iter().find_map(|arg| match arg {
                        syn::GenericArgument::Type(inner) => Some(inner.clone()),
                        _ => None,
                    })
                });
                let arg = self.emit_expr_to_string_with_expected_and_move_if_needed(
                    &call.args[0],
                    box_inner_expected_ty.as_ref(),
                );
                return format!("rusty::make_box({})", arg);
            }
            if matches!(func_leaf, "from" | "from_vec") {
                let smallvec_array_ty = expected_ty
                    .and_then(extract_smallvec_array_type_for_hint)
                    .or_else(|| {
                        infer_known_smallvec_type_from_call(call)
                            .and_then(|ty| extract_smallvec_array_type_for_hint(&ty))
                    })
                    .or_else(|| {
                        self.lookup_associated_call_return_type(call)
                            .and_then(|ty| extract_smallvec_array_type_for_hint(&ty))
                    })
                    .or_else(|| {
                        self.in_progress_local_initializers
                            .last()
                            .and_then(|name| self.lookup_local_binding_type(name))
                            .and_then(|ty| extract_smallvec_array_type_for_hint(&ty))
                    });
                let expected_elem_cpp = smallvec_array_ty
                    .map(|array_ty| {
                        // Prefer a direct `Item = T` resolution from an
                        // `impl Trait for [T; N]` block (recorded in
                        // `non_path_impl_assoc_types`) over the opaque
                        // `associated_item_t<...>` wrapper. The wrapper still
                        // works for downstream coercion but leaks unresolved
                        // template surfaces; the direct form lets array
                        // literals coerce their elements to the concrete
                        // type (e.g. `static_cast<uint8_t>(...)`).
                        let array_cpp = self.map_type(&array_ty);
                        if let Some(resolved) = self
                            .non_path_impl_assoc_types
                            .get(&array_cpp)
                            .and_then(|map| map.get("Item"))
                        {
                            return resolved.clone();
                        }
                        let expected_elem_ty: syn::Type =
                            parse_quote!(rusty::detail::associated_item_t<#array_ty>);
                        self.map_type(&expected_elem_ty)
                    })
                    .or_else(|| {
                        Self::extract_smallvec_owner_array_cpp_from_func(&func).map(|array_cpp| {
                            if let Some(resolved) = self
                                .non_path_impl_assoc_types
                                .get(&array_cpp)
                                .and_then(|map| map.get("Item"))
                            {
                                return resolved.clone();
                            }
                            format!("rusty::detail::associated_item_t<{}>", array_cpp)
                        })
                    });
                if let Some(expected_elem_cpp) = expected_elem_cpp {
                    if expected_elem_cpp != "auto"
                        && !type_string_has_auto_placeholder(&expected_elem_cpp)
                    {
                        let arg_expr = &call.args[0];
                        let arg_is_simple_local_path = matches!(
                            self.peel_paren_group_expr(arg_expr),
                            syn::Expr::Path(path_expr) if path_expr.path.segments.len() == 1
                        );
                        if func_leaf == "from" && arg_is_simple_local_path {
                            // Rust resolves `SmallVec::from(vec)` through trait impls even when
                            // C++ sees multiple static `from(...)` overloads. Force a typed
                            // Vec conversion so overload resolution picks the Vec-based impl.
                            let arg = self.emit_expr_maybe_move(arg_expr);
                            let coerced =
                                format!("rusty::Vec<{}>::from_iter({})", expected_elem_cpp, arg);
                            return format!("{}({})", func, coerced);
                        }
                        let arg_vec_elem_ty = self
                            .infer_hint_type_from_expr(arg_expr)
                            .or_else(|| self.infer_simple_expr_type(arg_expr))
                            .or_else(|| self.infer_local_binding_type_from_initializer(arg_expr))
                            .or_else(|| {
                                let syn::Expr::Path(path_expr) =
                                    self.peel_paren_group_expr(arg_expr)
                                else {
                                    return None;
                                };
                                if path_expr.path.segments.len() != 1 {
                                    return None;
                                }
                                let name = path_expr.path.segments[0].ident.to_string();
                                self.lookup_local_binding_type(&name)
                            })
                            .and_then(|ty| extract_vec_element_type_for_hint(&ty));
                        if let Some(arg_elem_ty) = arg_vec_elem_ty {
                            let arg_elem_cpp = self.map_type(&arg_elem_ty);
                            // When the arg is a chain of constructor calls
                            // wrapping an array literal (e.g.
                            // `into_vec(box_new([0,1,2]))`), we can thread
                            // the expected element type through the chain
                            // and let the inner array literal pick up the
                            // correct C++ element type instead of forcing
                            // an outer `from_iter` wrap. The wrap is only
                            // needed for stable lvalues that we cannot
                            // retype in place (e.g. `from_vec(local_vec)`).
                            let arg_can_be_retyped = matches!(
                                self.peel_paren_group_expr(arg_expr),
                                syn::Expr::Call(_) | syn::Expr::MethodCall(_)
                            );
                            if arg_elem_cpp != "auto"
                                && !type_string_has_auto_placeholder(&arg_elem_cpp)
                                && arg_elem_cpp != expected_elem_cpp
                                && !arg_can_be_retyped
                            {
                                let arg = self.emit_expr_maybe_move(arg_expr);
                                let coerced = format!(
                                    "rusty::Vec<{}>::from_iter({})",
                                    expected_elem_cpp, arg
                                );
                                return format!("{}({})", func, coerced);
                            }
                        } else if arg_is_simple_local_path {
                            // Local `let vec = ...; SmallVec::from(vec)` can still surface as
                            // `auto` in C++ when Rust-side expected type flows only through the
                            // downstream `SmallVec::from` call. Coerce eagerly to preserve
                            // associated-item element type.
                            let arg = self.emit_expr_maybe_move(arg_expr);
                            let coerced =
                                format!("rusty::Vec<{}>::from_iter({})", expected_elem_cpp, arg);
                            return format!("{}({})", func, coerced);
                        }
                    }
                }
            }
            if func_leaf == "from_str" {
                let arg = self.emit_expr_maybe_move(&call.args[0]);
                if matches!(func.as_str(), "from_str" | "::from_str")
                    && let Some(ok_ty) = self
                        .expected_from_str_target_type(effective_resolved_hint)
                        .or_else(|| self.expected_from_str_target_type(expected_ty))
                {
                    let ok_cpp = self.map_type(&ok_ty);
                    if ok_cpp != "auto"
                        && !ok_cpp.contains("/* TODO")
                        && !type_string_has_auto_placeholder(&ok_cpp)
                    {
                        return format!("{}<{}>(rusty::to_string_view({}))", func, ok_cpp, arg);
                    }
                }
                return format!("{}(rusty::to_string_view({}))", func, arg);
            }
        }
        if func == "rusty_ext::deserialize"
            || func == "::rusty_ext::deserialize"
            || func.ends_with("::rusty_ext::deserialize")
        {
            func = "::de::rusty_ext::deserialize".to_string();
        }
        if (func == "rusty_ext::serialize"
            || func == "::rusty_ext::serialize"
            || func.ends_with("::rusty_ext::serialize")
            || func == "Serialize::serialize"
            || func.ends_with("::Serialize::serialize"))
            && call.args.len() == 2
        {
            let value = self.emit_expr_maybe_move(&call.args[0]);
            let serializer = self.emit_expr_maybe_move(&call.args[1]);
            return self.emit_serialize_dispatch_call(&value, &serializer);
        }
        if func == "::de::rusty_ext::deserialize" && call.args.len() == 2 {
            let seed = self.emit_expr_maybe_move(&call.args[0]);
            let deserializer = self.emit_expr_maybe_move(&call.args[1]);
            return format!(
                "([&](auto&& __seed, auto&& __deserializer) -> decltype(auto) {{ if constexpr (requires {{ std::forward<decltype(__seed)>(__seed).deserialize(std::forward<decltype(__deserializer)>(__deserializer)); }}) {{ return std::forward<decltype(__seed)>(__seed).deserialize(std::forward<decltype(__deserializer)>(__deserializer)); }} else {{ return ::de::rusty_ext::deserialize(std::forward<decltype(__seed)>(__seed), std::forward<decltype(__deserializer)>(__deserializer)); }} }}({}, {}))",
                seed, deserializer
            );
        }
        let (func_owner_path, func_method_tail) = func
            .rsplit_once("::")
            .map_or((String::new(), func.as_str()), |(owner, method)| {
                (owner.to_string(), method)
            });
        let func_owner_tail = func_owner_path
            .rsplit("::")
            .next()
            .unwrap_or(func_owner_path.as_str())
            .to_string();
        let func_owner_is_type_param =
            !func_owner_tail.is_empty() && self.is_type_param_in_scope(&func_owner_tail);
        let func_is_deserialize_trait_call = func == "Deserialize::deserialize"
            || func.ends_with("::Deserialize::deserialize")
            || (func.ends_with("::deserialize")
                && (func.contains("::Deserialize,") || func.contains("::de::Deserialize,")))
            || (func_method_tail == "deserialize" && func_owner_is_type_param);
        if func_is_deserialize_trait_call && call.args.len() == 1 {
            let contextual_ok_ty = effective_resolved_hint
                .or(expected_ty)
                .or_else(|| {
                    if func_owner_is_type_param {
                        self.current_return_type_hint()
                    } else {
                        None
                    }
                })
                .and_then(|hint| self.expected_result_type_arg(Some(hint), 0));
            let expected_ok_ty = contextual_ok_ty;
            let ok_cpp = if let Some(ok_ty) = expected_ok_ty {
                self.map_type(ok_ty)
            } else if func_owner_is_type_param {
                escape_cpp_keyword(&func_owner_tail)
            } else {
                String::new()
            };
            if !ok_cpp.is_empty()
                && ok_cpp != "auto"
                && !ok_cpp.contains("/* TODO")
                && !type_string_has_auto_placeholder(&ok_cpp)
            {
                let deserializer = self.emit_deserializer_call_arg(&call.args[0]);
                return format!(
                    "::de::rusty_ext::deserialize(rusty::PhantomData<{}>{{}}, {})",
                    ok_cpp, deserializer
                );
            }
        }
        // `Sequence::deserialize(de)` where `Sequence` is a concrete local
        // type/alias (`Sequence = Vec<Value>`) is `<Sequence as
        // Deserialize>::deserialize` — route to the UFCS free-fn form. A member
        // call `Sequence::deserialize` fails: `Vec` (and other blanket-impl
        // types) has no deserialize member. The owner type IS the deserialize
        // target, so PhantomData<Sequence> drives dispatch.
        if func_method_tail == "deserialize"
            && !func_is_deserialize_trait_call
            && call.args.len() == 1
            && func_owner_tail
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
            && !func_owner_tail.is_empty()
            && (self.local_declared_types.contains(&func_owner_tail)
                || self.is_local_type_name_in_scope(&func_owner_tail)
                || self.type_alias_targets.contains_key(&func_owner_tail))
        {
            if let syn::Expr::Path(func_path) = call.func.as_ref()
                && let Some(owner_path) = Self::path_without_last_segment(&func_path.path)
            {
                let owner_ty = syn::Type::Path(syn::TypePath {
                    qself: None,
                    path: owner_path,
                });
                let owner_cpp = self.map_type(&owner_ty);
                if !owner_cpp.is_empty()
                    && owner_cpp != "auto"
                    && !owner_cpp.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&owner_cpp)
                {
                    let deserializer = self.emit_deserializer_call_arg(&call.args[0]);
                    return format!(
                        "::de::rusty_ext::deserialize(rusty::PhantomData<{}>{{}}, {})",
                        owner_cpp, deserializer
                    );
                }
            }
        }
        if func_method_tail == "deserialize"
            && !func_is_deserialize_trait_call
            && call.args.len() == 1
            && !func_method_tail.contains('<')
            && !func_owner_tail
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            let expected_output_ty = self
                .expected_result_type_arg(expected_ty, 0)
                .or(expected_ty)
                .cloned();
            if let Some(output_ty) = expected_output_ty {
                let output_cpp = self.map_type(&output_ty);
                if output_cpp != "auto"
                    && !output_cpp.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&output_cpp)
                {
                    let deserializer = self.emit_deserializer_call_arg(&call.args[0]);
                    return format!("{}<{}>({})", func, output_cpp, deserializer);
                }
            }
        }
        if func_method_tail == "duplicate_field"
            && call.args.len() == 1
            && !func_owner_path.is_empty()
        {
            let owner = func_owner_path.clone();
            let field_arg = self.emit_expr_maybe_move(&call.args[0]);
            return format!(
                "([&]() -> decltype(auto) {{ auto&& __field = {}; if constexpr (requires {{ {}::duplicate_field(__field); }}) {{ return {}::duplicate_field(__field); }} else {{ return {}::custom(std::format(\"duplicate field `{{0}}`\", rusty::to_string(__field))); }} }})()",
                field_arg, owner, owner, owner
            );
        }
        if func_method_tail == "missing_field"
            && call.args.len() == 1
            && !func_owner_path.is_empty()
        {
            let owner = func_owner_path.clone();
            let field_arg = self.emit_expr_maybe_move(&call.args[0]);
            return format!(
                "([&]() -> decltype(auto) {{ auto&& __field = {}; if constexpr (requires {{ {}::missing_field(__field); }}) {{ return {}::missing_field(__field); }} else {{ return {}::custom(std::format(\"missing field `{{0}}`\", rusty::to_string(__field))); }} }})()",
                field_arg, owner, owner, owner
            );
        }
        if func_method_tail == "invalid_length"
            && call.args.len() == 2
            && !func_owner_path.is_empty()
        {
            let owner = self.maybe_prefix_typename_for_dependent_path(func_owner_path.clone());
            let len_arg = self.emit_expr_maybe_move(&call.args[0]);
            let expected_arg = self.emit_expr_maybe_move(&call.args[1]);
            return format!(
                "rusty::error::invalid_length<{}>({}, {})",
                owner, len_arg, expected_arg
            );
        }
        if func_method_tail == "deserialize_in_place"
            && func_owner_is_type_param
            && call.args.len() == 2
        {
            let owner_cpp = escape_cpp_keyword(&func_owner_tail);
            let deserializer = self.emit_deserializer_call_arg(&call.args[0]);
            let place = self.emit_expr_maybe_move(&call.args[1]);
            return format!(
                "::de::rusty_ext::deserialize_in_place(rusty::PhantomData<{}>{{}}, {}, {})",
                owner_cpp, deserializer, place
            );
        }
        if let syn::Expr::Path(func_path) = call.func.as_ref()
            && call.args.len() == 1
            && func_path.path.segments.len() >= 2
        {
            let owner_name = func_path
                .path
                .segments
                .iter()
                .nth_back(1)
                .map(|seg| seg.ident.to_string())
                .unwrap_or_default();
            let method_name = func_path
                .path
                .segments
                .last()
                .map(|seg| seg.ident.to_string())
                .unwrap_or_default();
            if owner_name == "Bytes" && matches!(method_name.as_str(), "new" | "new_" | "from") {
                let arg = self.emit_expr_maybe_move(&call.args[0]);
                return format!("{}(rusty::as_u8_slice({}))", func, arg);
            }
            if owner_name == "ByteArray"
                && matches!(method_name.as_str(), "new" | "new_" | "from" | "try_from")
            {
                let arg_is_array_repeat_call = match self.peel_paren_group_expr(&call.args[0]) {
                    syn::Expr::Call(arg_call) => {
                        if let syn::Expr::Path(arg_path) =
                            self.peel_paren_group_expr(arg_call.func.as_ref())
                        {
                            let joined = arg_path
                                .path
                                .segments
                                .iter()
                                .map(|seg| seg.ident.to_string())
                                .collect::<Vec<_>>()
                                .join("::");
                            matches!(joined.as_str(), "rusty::array_repeat" | "array_repeat")
                        } else {
                            false
                        }
                    }
                    _ => false,
                };
                let arg_is_array_repeat_result = self
                    .infer_simple_expr_type(&call.args[0])
                    .is_some_and(|ty| {
                        let ty = self.peel_reference_paren_group_type(&ty);
                        matches!(ty, syn::Type::Path(tp)
                                if tp.path
                                    .segments
                                    .last()
                                    .is_some_and(|seg| seg.ident == "ArrayRepeatResult"))
                    });
                let arg = self.emit_expr_maybe_move(&call.args[0]);
                let arg = if arg_is_array_repeat_call || arg_is_array_repeat_result {
                    arg
                } else {
                    format!("rusty::as_u8_array({})", arg)
                };
                return format!("{}({})", func, arg);
            }
        }
        if can_resolve_result_ctor_infer
            && let Some((ctor_name, _)) = result_ctor_with_infer_turbofish.as_ref()
        {
            // Keep infer-turbofish Result ctors in the expected-type-aware Ok/Err lowering
            // path instead of emitting invalid template args like `Ok<auto, ...>(...)`.
            func = ctor_name.clone();
        }
        if matches!(
            func.as_str(),
            "String::from_utf8_lossy"
                | "rusty::String::from_utf8_lossy"
                | "std::string::String::from_utf8_lossy"
                | "alloc::string::String::from_utf8_lossy"
        ) && call.args.len() == 1
        {
            let arg = self.emit_expr_maybe_move(&call.args[0]);
            let expects_cow = expected_ty
                .map(|ty| self.map_type(ty))
                .is_some_and(|mapped| mapped == "rusty::Cow");
            if expects_cow {
                return format!("rusty::Cow_Owned(rusty::String::from_utf8_lossy({}))", arg);
            }
            return format!("rusty::String::from_utf8_lossy({})", arg);
        }
        if matches!(
            func.as_str(),
            "rusty::fmt::Formatter::write_str"
                | "rusty::fmt::Formatter::write_char"
                | "core::fmt::Formatter::write_str"
                | "core::fmt::Formatter::write_char"
                | "std::fmt::Formatter::write_str"
                | "std::fmt::Formatter::write_char"
                | "fmt::Formatter::write_str"
                | "fmt::Formatter::write_char"
        ) && call.args.len() >= 2
        {
            let receiver = match self.peel_paren_group_expr(&call.args[0]) {
                syn::Expr::Reference(r) => self.emit_expr_to_string(&r.expr),
                expr => self.emit_expr_to_string(expr),
            };
            let method = if func.ends_with("write_char") {
                "write_char"
            } else {
                "write_str"
            };
            let args: Vec<String> = call
                .args
                .iter()
                .skip(1)
                .map(|arg| self.emit_expr_maybe_move(arg))
                .collect();
            return format!("{}.{}({})", receiver, method, args.join(", "));
        }
        if matches!(
            func.as_str(),
            "rusty::ptr::read"
                | "rusty::ptr::write"
                | "rusty::ptr::copy"
                | "rusty::ptr::copy_nonoverlapping"
        ) && !call.args.is_empty()
        {
            let args: Vec<String> = call
                .args
                .iter()
                .enumerate()
                .map(|(idx, arg)| {
                    let is_ptr_arg = match func.as_str() {
                        "rusty::ptr::read" | "rusty::ptr::write" => idx == 0,
                        "rusty::ptr::copy" | "rusty::ptr::copy_nonoverlapping" => idx <= 1,
                        _ => false,
                    };
                    if is_ptr_arg {
                        if let Some(ptr_arg) = self.emit_raw_pointer_call_arg(arg) {
                            return ptr_arg;
                        }
                    }
                    if func == "rusty::ptr::write" && idx == 1 {
                        let value = self.emit_expr_to_string(arg);
                        if value.starts_with("std::move(") {
                            return value;
                        }
                        return format!("std::move({})", value);
                    }
                    self.emit_expr_maybe_move(arg)
                })
                .collect();
            return format!("{}({})", func, args.join(", "));
        }
        if matches!(
            func.as_str(),
            "rusty::ptr::add"
                | "rusty::ptr::offset"
                | "core::ptr::mut_ptr::add"
                | "std::ptr::mut_ptr::add"
                | "ptr::mut_ptr::add"
                | "core::ptr::mut_ptr::offset"
                | "std::ptr::mut_ptr::offset"
                | "ptr::mut_ptr::offset"
                | "core::ptr::const_ptr::add"
                | "std::ptr::const_ptr::add"
                | "ptr::const_ptr::add"
                | "core::ptr::const_ptr::offset"
                | "std::ptr::const_ptr::offset"
                | "ptr::const_ptr::offset"
        ) && call.args.len() == 2
        {
            let ptr_expected = expected_ty
                .map(|ty| self.peel_reference_paren_group_type(ty))
                .filter(|ty| matches!(ty, syn::Type::Ptr(_)));
            let ptr_arg = if let Some(ptr_expected) = ptr_expected {
                self.emit_expr_to_string_with_expected(&call.args[0], Some(ptr_expected))
            } else if let Some(raw) = self.emit_raw_pointer_call_arg(&call.args[0]) {
                raw
            } else {
                self.emit_expr_maybe_move(&call.args[0])
            };
            let mut offset_arg = self.emit_expr_maybe_move(&call.args[1]);
            if let syn::Expr::Path(path) = self.peel_paren_group_expr(&call.args[1])
                && path.path.segments.len() == 1
            {
                let name = path.path.segments[0].ident.to_string();
                if std::env::var_os("RUSTY_CPP_DEBUG_REBIND_POINTER").is_some() {
                    let ty_dbg = self
                        .lookup_local_binding_type(&name)
                        .map(|ty| ty.to_token_stream().to_string())
                        .unwrap_or_else(|| "<none>".to_string());
                    eprintln!(
                        "[rebind-pointer] call_ptr_add name={} lowered={} reassigned={} const_in_scope={} local_cpp={:?} ty={}",
                        name,
                        self.is_reference_binding_lowered_to_pointer_storage(&name),
                        self.reassigned_vars.contains(&name),
                        self.is_const_local_binding_in_scope(&name),
                        self.lookup_local_binding_cpp_name(&name),
                        ty_dbg
                    );
                }
                if self.is_reference_binding_lowered_to_pointer_storage(&name)
                    && !offset_arg.trim_start().starts_with('*')
                {
                    offset_arg = format!("*({})", offset_arg);
                }
            }
            let emitted_call = format!("{}({}, {})", func, ptr_arg, offset_arg);
            if std::env::var_os("RUSTY_CPP_DEBUG_REBIND_POINTER").is_some() {
                eprintln!(
                    "[rebind-pointer] emit_call_ptr_add fn={} arg0={} arg1={}",
                    func, ptr_arg, offset_arg
                );
            }
            return emitted_call;
        }
        if func == "rusty::mem::replace" && !call.args.is_empty() {
            let replace_value_expected_ty = call
                .args
                .first()
                .and_then(|arg0| self.infer_simple_expr_type(arg0))
                .and_then(|arg0_ty| self.expected_reference_inner_type(Some(&arg0_ty)).cloned());
            let args: Vec<String> = call
                .args
                .iter()
                .enumerate()
                .map(|(idx, arg)| {
                    if idx == 0 {
                        if let syn::Expr::Reference(r) = self.peel_paren_group_expr(arg) {
                            return self.emit_expr_to_string(&r.expr);
                        }
                    }
                    if idx == 1 {
                        return self.emit_expr_to_string_with_expected_and_move_if_needed(
                            arg,
                            replace_value_expected_ty.as_ref(),
                        );
                    }
                    self.emit_expr_maybe_move(arg)
                })
                .collect();
            return format!("{}({})", func, args.join(", "));
        }
        // Qualify unqualified Ok/Err with rusty:: prefix when they have
        // explicit template args (turbofish), e.g., `Ok::<_, ()>(value)` →
        // `rusty::Ok<auto, std::tuple<>>(value)`. Plain `Ok(value)` is
        // handled by expected-type-aware Result constructor lowering above.
        let func = if (func.starts_with("Ok<") || func.starts_with("Err<"))
            && !func.starts_with("rusty::")
        {
            format!("rusty::{}", func)
        } else {
            func
        };
        if matches!(
            func.as_str(),
            "rusty::mem::transmute"
                | "mem::transmute"
                | "transmute"
                | "core::mem::transmute"
                | "std::mem::transmute"
        ) && call.args.len() == 1
        {
            let arg = self.emit_expr_maybe_move(&call.args[0]);
            if let Some(expected) = effective_resolved_hint.or(expected_ty) {
                let expected_cpp = self.map_type(expected);
                if expected_cpp != "auto"
                    && !expected_cpp.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&expected_cpp)
                {
                    return format!(
                        "rusty::mem::transmute<std::remove_cvref_t<decltype(({}))>, {}>({})",
                        arg, expected_cpp, arg
                    );
                }
            }
            return format!("rusty::mem::transmute({})", arg);
        }
        if matches!(
            func.as_str(),
            "rusty::str_runtime::from_utf8"
                | "rusty::str_runtime::from_utf8_unchecked"
                | "rusty::str_runtime::from_utf8_unchecked_mut"
        ) && call.args.len() == 1
        {
            let arg = match self.peel_paren_group_expr(&call.args[0]) {
                syn::Expr::Reference(r) if !self.is_expr_raw_pointer_like(&r.expr) => {
                    self.emit_expr_to_string(&r.expr)
                }
                _ => self.emit_expr_maybe_move(&call.args[0]),
            };
            return format!("{}({})", func, arg);
        }
        if let Some(expected) = effective_resolved_hint {
            if self.is_noreturn_panic_like_call_path(&func) {
                let args: Vec<String> = call
                    .args
                    .iter()
                    .map(|a| self.emit_expr_maybe_move(a))
                    .collect();
                // `[[noreturn]]` is a function/decl attribute, not a type
                // attribute: a `!`-typed expected (`map_type` → `[[noreturn]]
                // void`) is illegal as a lambda trailing return type. Strip it —
                // the IIFE wraps a diverging call and yields nothing anyway.
                let expected_cpp = self.map_type(expected).replace("[[noreturn]] ", "");
                if expected_cpp == "auto"
                    || expected_cpp.contains("/* TODO")
                    || expected_cpp.contains("Self::")
                    || type_string_has_auto_placeholder(&expected_cpp)
                    || self.type_contains_unresolved_self_type_path(expected)
                {
                    return format!("{}({})", func, args.join(", "));
                }
                return format!(
                    "[&]() -> {} {{ {}({}); }}()",
                    expected_cpp,
                    func,
                    args.join(", ")
                );
            }
        }

        // Keep Cursor constructor lowering as a normal function call shape with
        // template argument deduction, so generated `decltype((...))` contexts
        // do not require an explicit `Cursor<T>::new_` specialization at call sites.
        if func == "rusty::io::Cursor::new_" && call.args.len() == 1 {
            let arg = self.emit_cursor_new_arg_expr(&call.args[0]);
            return format!("rusty::io::cursor_new({})", arg);
        }

        if matches!(func.as_str(), "Cell::new_" | "rusty::Cell::new_") && call.args.len() == 1 {
            let arg = self.emit_expr_maybe_move(&call.args[0]);
            return format!(
                "rusty::Cell<std::remove_cvref_t<decltype({})>>::new_({})",
                arg, arg
            );
        }
        if matches!(func.as_str(), "RefCell::new_" | "rusty::RefCell::new_") && call.args.len() == 1
        {
            let expected_inner_ty = expected_ty.and_then(|ty| {
                let ty = self.peel_reference_paren_group_type(ty);
                let syn::Type::Path(tp) = ty else {
                    return None;
                };
                let last = tp.path.segments.last()?;
                if last.ident != "RefCell" {
                    return None;
                }
                let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
                    return None;
                };
                args.args.iter().find_map(|arg| match arg {
                    syn::GenericArgument::Type(inner) => Some(inner.clone()),
                    _ => None,
                })
            });
            if let Some(expected) = expected_ty {
                let expected_cpp = self.map_type(expected);
                if expected_cpp.starts_with("rusty::RefCell<")
                    && !type_string_has_auto_placeholder(&expected_cpp)
                {
                    let arg = self.emit_expr_to_string_with_expected_and_move_if_needed(
                        &call.args[0],
                        expected_inner_ty.as_ref(),
                    );
                    return format!("{}::new_({})", expected_cpp, arg);
                }
            }
            let arg = self.emit_expr_maybe_move(&call.args[0]);
            return format!(
                "rusty::RefCell<std::remove_cvref_t<decltype(({}))>>::new_({})",
                arg, arg
            );
        }
        if matches!(
            func.as_str(),
            "ManuallyDrop::new_"
                | "rusty::mem::ManuallyDrop::new_"
                | "std::mem::ManuallyDrop::new_"
                | "core::mem::ManuallyDrop::new_"
        ) && call.args.len() == 1
        {
            let expected_inner_ty = expected_ty.and_then(|ty| {
                let ty = self.peel_reference_paren_group_type(ty);
                let syn::Type::Path(tp) = ty else {
                    return None;
                };
                let last = tp.path.segments.last()?;
                if last.ident != "ManuallyDrop" {
                    return None;
                }
                let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
                    return None;
                };
                args.args.iter().find_map(|arg| match arg {
                    syn::GenericArgument::Type(inner) => Some(inner.clone()),
                    _ => None,
                })
            });
            if let Some(expected) = expected_ty {
                let expected_cpp = self.map_type(expected);
                if expected_cpp.starts_with("rusty::mem::ManuallyDrop<")
                    && !type_string_has_auto_placeholder(&expected_cpp)
                {
                    let arg = self.emit_expr_to_string_with_expected_and_move_if_needed(
                        &call.args[0],
                        expected_inner_ty.as_ref(),
                    );
                    return format!("{}::new_({})", expected_cpp, arg);
                }
            }
            let arg = self.emit_expr_maybe_move(&call.args[0]);
            return format!(
                "rusty::mem::ManuallyDrop<std::remove_cvref_t<decltype(({}))>>::new_({})",
                arg, arg
            );
        }

        if call.args.is_empty() {
            let is_vec_new_call = matches!(func.as_str(), "Vec::new_" | "rusty::Vec::new_")
                || ((func.starts_with("rusty::Vec<") || func.starts_with("::rusty::Vec<"))
                    && func.ends_with(">::new_"));
            if is_vec_new_call {
                if let Some(expected) = expected_ty {
                    let expected_cpp = self.map_type(expected);
                    if expected_cpp.starts_with("rusty::Vec<")
                        && !type_string_has_auto_placeholder(&expected_cpp)
                    {
                        return format!("{}::new_()", expected_cpp);
                    }
                }
            }
            let is_btreemap_new_call = matches!(
                func.as_str(),
                "BTreeMap::new_" | "rusty::BTreeMap::new_" | "::rusty::BTreeMap::new_"
            ) || ((func.starts_with("rusty::BTreeMap<")
                || func.starts_with("::rusty::BTreeMap<"))
                && func.ends_with(">::new_"));
            if is_btreemap_new_call {
                if let Some(expected) = expected_ty {
                    let expected_cpp = self.map_type(expected);
                    let normalized_expected = expected_cpp.trim_start_matches("::");
                    if (normalized_expected.starts_with("rusty::BTreeMap<")
                        || normalized_expected.starts_with("BTreeMap<")
                        || normalized_expected.starts_with("MapImpl<")
                        || normalized_expected.contains("::MapImpl<"))
                        && !expected_cpp.contains("/* TODO")
                        && !type_string_has_auto_placeholder(&expected_cpp)
                    {
                        return format!("{}::new_()", expected_cpp);
                    }
                }
            }
        }
        if matches!(func.as_str(), "Vec::new_" | "rusty::Vec::new_") && call.args.is_empty() {
            if let Some(expected) = expected_ty {
                let expected_cpp = self.map_type(expected);
                if expected_cpp.starts_with("rusty::Vec<") {
                    return format!("{}::new_()", expected_cpp);
                }
            }
        }
        if call.args.len() == 1 {
            if matches!(
                func.as_str(),
                "rusty::boxed::box_assume_init_into_vec_unsafe"
                    | "box_assume_init_into_vec_unsafe"
                    | "alloc::boxed::box_assume_init_into_vec_unsafe"
                    | "std::boxed::box_assume_init_into_vec_unsafe"
            ) {
                if let syn::Expr::Call(inner_call) = self.peel_paren_group_expr(&call.args[0]) {
                    let mut inner_func =
                        self.emit_call_func_with_owner_template_recovery(inner_call, expected_ty);
                    inner_func = self.rewrite_seed_ctor_path_string(&inner_func);
                    if matches!(
                        inner_func.as_str(),
                        "rusty::intrinsics::write_box_via_move"
                            | "write_box_via_move"
                            | "alloc::intrinsics::write_box_via_move"
                            | "std::intrinsics::write_box_via_move"
                    ) && inner_call.args.len() == 2
                    {
                        let payload_expr = &inner_call.args[1];
                        if let Some(elem_ty) = self.expected_vec_element_type(expected_ty) {
                            if Self::is_u8_syn_type(elem_ty) {
                                if let Some(payload) =
                                    self.try_emit_boxed_u8_array_expr(payload_expr)
                                {
                                    return format!(
                                        "rusty::boxed::into_vec(rusty::boxed::box_new({}))",
                                        payload
                                    );
                                }
                            } else if let Some(expected_tuple_ty) =
                                self.expected_tuple_type(Some(elem_ty))
                            {
                                if let Some(payload) = self
                                    .try_emit_boxed_tuple_array_expr_with_expected_tuple(
                                        payload_expr,
                                        &expected_tuple_ty,
                                    )
                                {
                                    return format!(
                                        "rusty::boxed::into_vec(rusty::boxed::box_new({}))",
                                        payload
                                    );
                                }
                            }
                            if let Some(payload) = self
                                .try_emit_boxed_array_expr_with_expected_elem_type(
                                    payload_expr,
                                    elem_ty,
                                )
                            {
                                return format!(
                                    "rusty::boxed::into_vec(rusty::boxed::box_new({}))",
                                    payload
                                );
                            }
                        }
                        if let Some(payload) = self
                            .try_emit_boxed_tuple_array_expr_with_inferred_tuple_harmonization(
                                payload_expr,
                            )
                        {
                            return format!(
                                "rusty::boxed::into_vec(rusty::boxed::box_new({}))",
                                payload
                            );
                        }
                        let payload = self.emit_expr_maybe_move(payload_expr);
                        return format!(
                            "rusty::boxed::into_vec(rusty::boxed::box_new({}))",
                            payload
                        );
                    }
                }
                let arg = self.emit_expr_maybe_move(&call.args[0]);
                return format!("rusty::boxed::into_vec({})", arg);
            }

            let is_vec_from_iter_call =
                matches!(func.as_str(), "Vec::from_iter" | "rusty::Vec::from_iter")
                    || ((func.starts_with("rusty::Vec<") || func.starts_with("::rusty::Vec<"))
                        && func.ends_with(">::from_iter"));
            if is_vec_from_iter_call {
                let arg = self.emit_expr_maybe_move(&call.args[0]);
                // `Vec::from_iter(X)` is a Vec in Rust; `collect_range` yields a
                // bare std::vector (losing rusty::Vec methods like sort_by), so
                // re-wrap into a rusty::Vec (CTAD deduces the element from the
                // collected std::vector<E>).
                return format!("rusty::Vec(rusty::collect_range({}))", arg);
            }
        }
        if matches!(
            func.as_str(),
            "rusty::intrinsics::write_box_via_move"
                | "write_box_via_move"
                | "alloc::intrinsics::write_box_via_move"
                | "std::intrinsics::write_box_via_move"
        ) && call.args.len() == 2
        {
            let payload = self.emit_expr_maybe_move(&call.args[1]);
            return format!("rusty::boxed::box_new({})", payload);
        }
        if matches!(
            func.as_str(),
            "rusty::boxed::into_vec"
                | "into_vec"
                | "alloc::boxed::into_vec"
                | "std::boxed::into_vec"
        ) && call.args.len() == 1
        {
            if let Some(specialized_arg) =
                self.try_emit_into_vec_box_new_with_inferred_tuple_payload(&call.args[0])
            {
                return format!("{}({})", func, specialized_arg);
            }
            if let Some(elem_ty) = self.expected_vec_element_type(expected_ty) {
                if Self::is_u8_syn_type(elem_ty) {
                    if let Some(specialized_arg) =
                        self.try_emit_into_vec_box_new_with_u8_payload(&call.args[0])
                    {
                        return format!("{}({})", func, specialized_arg);
                    }
                } else if let Some(expected_tuple_ty) = self.expected_tuple_type(Some(elem_ty)) {
                    if let Some(specialized_arg) = self
                        .try_emit_into_vec_box_new_with_expected_tuple_payload(
                            &call.args[0],
                            &expected_tuple_ty,
                        )
                    {
                        return format!("{}({})", func, specialized_arg);
                    }
                }
                if let Some(specialized_arg) = self
                    .try_emit_into_vec_box_new_with_expected_elem_payload(&call.args[0], elem_ty)
                {
                    return format!("{}({})", func, specialized_arg);
                }
            }
        }

        // `rusty::HashMap` exposes constructor/make surfaces instead of `new_`.
        // Lower associated `HashMap::new()` calls to direct value construction
        // after owner-template recovery so omitted generics become concrete.
        if call.args.is_empty() {
            let is_hashmap_assoc_new = if let syn::Expr::Path(path_expr) = call.func.as_ref() {
                if path_expr.path.segments.len() >= 2 {
                    let owner = path_expr.path.segments.iter().nth_back(1);
                    let method = path_expr.path.segments.last();
                    matches!(
                        owner.map(|seg| seg.ident.to_string()).as_deref(),
                        Some("HashMap")
                    ) && matches!(
                        method.map(|seg| seg.ident.to_string()).as_deref(),
                        Some("new") | Some("new_")
                    )
                } else {
                    false
                }
            } else {
                false
            };
            if is_hashmap_assoc_new {
                if let Some((owner_cpp, _)) = func.rsplit_once("::") {
                    if owner_cpp.starts_with("rusty::HashMap") {
                        return format!("{}()", owner_cpp);
                    }
                }
            }
        }

        if func == "rusty::array_repeat" && call.args.len() == 2 {
            if let Some(expected_array_ty) = expected_ty.and_then(|ty| {
                let ty = self.peel_reference_paren_group_type(ty);
                let syn::Type::Array(array_ty) = ty else {
                    return None;
                };
                Some(array_ty)
            }) {
                let elem_ty = expected_array_ty.elem.as_ref();
                let elem_cpp = self.map_type(elem_ty);
                let value = self.emit_expr_to_string_with_expected(&call.args[0], Some(elem_ty));
                let len_expr = self.emit_expr_to_string(&expected_array_ty.len);
                let repeat_len = if self
                    .should_sanitize_array_capacity_expr(&expected_array_ty.len, &len_expr)
                {
                    format!("rusty::sanitize_array_capacity<{}>()", len_expr)
                } else {
                    len_expr
                };
                let repeat_seed = Self::emit_repeat_seed_with_cast("_seed", &elem_cpp);
                return format!(
                    "[](auto _seed) {{ std::array<{}, {}> _repeat{{}}; _repeat.fill({}); return _repeat; }}({})",
                    elem_cpp, repeat_len, repeat_seed, value
                );
            }
            // `alloc::vec::from_elem` is lowered to `rusty::array_repeat`.
            // Preserve expected `Vec<T>` element typing so unsuffixed literals
            // inside `Some(0)` become `Some(size_t)` when context requires it.
            let elem_ty = self
                .expected_array_element_type(expected_ty)
                .or_else(|| self.expected_vec_element_type(expected_ty));
            if let Some(elem_ty) = elem_ty {
                let value = self.emit_expr_to_string_with_expected(&call.args[0], Some(elem_ty));
                let count = self.emit_expr_to_string(&call.args[1]);
                return format!("rusty::array_repeat({}, {})", value, count);
            }
        }

        if (func == "chain" || func.ends_with("::chain") || func.starts_with("rusty::chain"))
            && call.args.len() == 2
        {
            let expected_item_ty = expected_ty
                .and_then(|ty| self.extract_iter_item_type_from_type(ty))
                .filter(|ty| self.type_is_concrete_hint_candidate(ty));
            let chain_item_ty = expected_item_ty.or_else(|| {
                let left_seed = self.expr_is_once_with_unsuffixed_numeric_seed(&call.args[0]);
                let right_seed = self.expr_is_once_with_unsuffixed_numeric_seed(&call.args[1]);
                let left_item = self.infer_iter_item_type_from_expr(&call.args[0])?;
                let right_item = self.infer_iter_item_type_from_expr(&call.args[1])?;
                if Self::types_equivalent_by_tokens(&left_item, &right_item) {
                    return None;
                }
                if left_seed && !right_seed && self.type_is_concrete_hint_candidate(&right_item) {
                    return Some(right_item);
                }
                if right_seed && !left_seed && self.type_is_concrete_hint_candidate(&left_item) {
                    return Some(left_item);
                }
                None
            });
            if let Some(item_ty) = chain_item_ty {
                let left = self.emit_chain_arg_with_item_hint(&call.args[0], &item_ty);
                let right = self.emit_chain_arg_with_item_hint(&call.args[1], &item_ty);
                return format!("{}({}, {})", func, left, right);
            }
        }

        // Map Rust Option::Some(x) to rusty::Option<T>(x) whenever we can recover T.
        if matches!(
            func.as_str(),
            "Some"
                | "rusty::Some"
                | "Option::Some"
                | "core::option::Option::Some"
                | "std::option::Option::Some"
        ) && call.args.len() == 1
        {
            let expected_inner_ty = self.expected_option_type_arg(expected_ty);
            let expected_inner_is_ref = expected_inner_ty.is_some_and(|ty| {
                matches!(self.peel_paren_group_type(ty), syn::Type::Reference(_))
            });
            if let Some(inner_cpp) = self.option_ctor_inner_cpp_type(expected_ty) {
                if expected_inner_is_ref {
                    let expected_inner_lowers_to_value = expected_inner_ty
                        .is_some_and(|ty| self.reference_type_lowers_to_value_cpp(ty));
                    let inner_cpp_is_pointer =
                        inner_cpp.trim_start().starts_with("std::add_pointer_t<")
                            || inner_cpp.trim_end().ends_with('*');
                    let inner_cpp_is_reference = inner_cpp.contains('&');
                    if expected_inner_lowers_to_value
                        || (!inner_cpp_is_pointer && !inner_cpp_is_reference)
                    {
                        let arg = self.emit_expr_to_string_with_expected_and_move_if_needed(
                            &call.args[0],
                            expected_inner_ty,
                        );
                        return format!("rusty::Option<{}>({})", inner_cpp, arg);
                    }
                    if inner_cpp_is_pointer {
                        let arg_peeled = self.peel_single_expr_through_unsafe(&call.args[0]);
                        if let syn::Expr::Unary(unary) = arg_peeled
                            && matches!(unary.op, syn::UnOp::Deref(_))
                            && self.is_expr_raw_pointer_like(&unary.expr)
                        {
                            let ptr_arg = self.emit_expr_to_string(&unary.expr);
                            return format!("rusty::Option<{}>({})", inner_cpp, ptr_arg);
                        }
                        if let syn::Expr::Reference(reference) = arg_peeled
                            && let syn::Expr::Unary(unary) =
                                self.peel_single_expr_through_unsafe(&reference.expr)
                            && matches!(unary.op, syn::UnOp::Deref(_))
                            && self.is_expr_raw_pointer_like(&unary.expr)
                        {
                            let ptr_arg = self.emit_expr_to_string(&unary.expr);
                            return format!("rusty::Option<{}>({})", inner_cpp, ptr_arg);
                        }
                    }
                    if let Some(ref_arg) = self.emit_some_ref_constructor_arg(
                        &call.args[0],
                        Some(&inner_cpp),
                        expected_inner_ty,
                    ) {
                        return format!("rusty::Option<{}>({})", inner_cpp, ref_arg);
                    }
                    let arg_peeled = self.peel_single_expr_through_unsafe(&call.args[0]);
                    let can_bind_ref_ctor = self.is_expr_reference_like(&call.args[0])
                        || self.is_expr_reference_like(arg_peeled)
                        || self.is_stable_reference_lvalue_expr(&call.args[0])
                        || self.is_stable_reference_lvalue_expr(arg_peeled)
                        || matches!(
                            arg_peeled,
                            syn::Expr::Call(inner_call)
                                if matches!(
                                    self.peel_paren_group_expr(inner_call.func.as_ref()),
                                    syn::Expr::Closure(_)
                                )
                        )
                        || matches!(
                            arg_peeled,
                            syn::Expr::Path(_)
                                | syn::Expr::Field(_)
                                | syn::Expr::Index(_)
                                | syn::Expr::MethodCall(_)
                        );
                    if !can_bind_ref_ctor {
                        // Avoid binding Option<T&>/Option<const T&> from non-reference
                        // rvalues (e.g. literals or temporary Strings).
                        let preserves_control_flow = matches!(
                            arg_peeled,
                            syn::Expr::Call(inner_call)
                                if matches!(
                                    self.peel_paren_group_expr(inner_call.func.as_ref()),
                                    syn::Expr::Closure(_)
                                )
                        ) || matches!(
                            arg_peeled,
                            syn::Expr::Block(_)
                                | syn::Expr::If(_)
                                | syn::Expr::Match(_)
                                | syn::Expr::Unsafe(_)
                        );
                        if preserves_control_flow {
                            // Preserve expected typing through control-flow args that
                            // carry non-local `return`/`?` from expanded derive code.
                            let arg = self.emit_expr_to_string_with_expected(
                                &call.args[0],
                                expected_inner_ty,
                            );
                            return format!("rusty::Option<{}>({})", inner_cpp, arg);
                        }
                    } else {
                        if let Some(expected_inner) = expected_inner_ty
                            && let Some(coerced_arg) = self
                                .try_emit_reference_coercion_for_expected_option_inner(
                                    &call.args[0],
                                    expected_inner,
                                )
                        {
                            return format!("rusty::Option<{}>({})", inner_cpp, coerced_arg);
                        }
                        // Use the `_and_move_if_needed` variant so an owned-typed
                        // lvalue path (e.g. a non-reference function parameter
                        // being moved into `Some(...)`) gets `std::move(...)`
                        // emitted. The plain `_with_expected` variant skipped
                        // the move, so e.g. `Some(archive_cb)` on a move-only
                        // `Function<Sig>` parameter emitted `Option<...>(archive_cb)`
                        // which fails the deleted copy ctor. The helper checks
                        // the expected inner type and only forces `std::move`
                        // when the field genuinely consumes the value (it leaves
                        // `Option<T&>` bindings alone via
                        // `should_move_reference_binding_for_expected_value`).
                        let arg = self.emit_expr_to_string_with_expected_and_move_if_needed(
                            &call.args[0],
                            expected_inner_ty,
                        );
                        return format!("rusty::Option<{}>({})", inner_cpp, arg);
                    }
                } else {
                    let arg = self.emit_expr_to_string_with_expected_and_move_if_needed(
                        &call.args[0],
                        expected_inner_ty,
                    );
                    return format!("rusty::Option<{}>({})", inner_cpp, arg);
                }
            }
            if let Some(ref_arg) = self.emit_some_ref_constructor_arg(&call.args[0], None, None) {
                return format!("rusty::SomeRef({})", ref_arg);
            }
            let some_expected_inner_ty = if expected_inner_is_ref {
                None
            } else {
                expected_inner_ty
            };
            let arg = self.emit_expr_to_string_with_expected_and_move_if_needed(
                &call.args[0],
                some_expected_inner_ty,
            );
            // Prefer contextual Option inner typing when available; otherwise
            // integer literals in `Some(0)` can lock constructors to `i32` and
            // break associated-type call sites (for example `Option<T::Bits>`).
            if expected_inner_ty.is_none() {
                if let Some(inferred_inner_ty) = self.infer_simple_expr_type(&call.args[0]) {
                    let inferred_unemitted_current_assoc = self
                        .should_soften_dependent_assoc_mode()
                        && self.type_references_current_struct_assoc_projection(&inferred_inner_ty)
                        && !self.type_current_struct_assoc_aliases_emitted(&inferred_inner_ty);
                    let inferred_inner_cpp = self.map_type(&inferred_inner_ty);
                    if !inferred_unemitted_current_assoc
                        && inferred_inner_cpp != "auto"
                        && !inferred_inner_cpp.contains("/* TODO")
                        && !type_string_has_auto_placeholder(&inferred_inner_cpp)
                    {
                        return format!("rusty::Option<{}>({})", inferred_inner_cpp, arg);
                    }
                }
            }
            if let Some(inner_ty) = expected_inner_ty
                && !expected_inner_is_ref
            {
                let expected_unemitted_current_assoc = self.should_soften_dependent_assoc_mode()
                    && self.type_references_current_struct_assoc_projection(inner_ty)
                    && !self.type_current_struct_assoc_aliases_emitted(inner_ty);
                let mapped_inner = self.map_type(inner_ty);
                if !expected_unemitted_current_assoc
                    && mapped_inner != "auto"
                    && !mapped_inner.contains("/* TODO")
                    && !type_string_has_auto_placeholder(&mapped_inner)
                {
                    return format!("rusty::Option<{}>({})", mapped_inner, arg);
                }
            }
            return format!("rusty::Some({})", arg);
        }
        // Map Rust conversion shim in expanded output.
        if self.is_core_from_path_expr(call.func.as_ref()) && call.args.len() == 1 {
            if let Some(expected) = expected_ty {
                let expected_cpp = self.map_type(expected);
                return self.emit_from_conversion_to_target(&call.args[0], &expected_cpp);
            }
            return self.emit_expr_maybe_move(&call.args[0]);
        }
        if (self.is_default_trait_default_path_expr(call.func.as_ref())
            || self.is_default_value_shorthand_path_expr(call.func.as_ref()))
            && call.args.is_empty()
        {
            if let Some(expected) = expected_ty
                .cloned()
                .or_else(|| self.infer_default_call_expected_type_from_in_progress_local_name())
            {
                if let Some(default_expr) = self.try_emit_default_value_expr_for_type(&expected) {
                    return default_expr;
                }
                let expected_cpp = self.map_type(&expected);
                return format!("rusty::default_value<{}>()", expected_cpp);
            }
        }
        // Map Ok(x) and Err(x) for Result.
        // `effective_resolved_hint` already merges parent expected type with closure return
        // type hint and resolves `_` placeholders using contextual hints when available.
        let func_leaf = func.rsplit("::").next().unwrap_or(func.as_str());
        let func_leaf_base = func_leaf.split('<').next().unwrap_or(func_leaf);
        let explicit_ctor_hint_owned =
            if matches!(func_leaf_base, "Ok" | "Err") && call.args.len() == 1 {
                if let syn::Expr::Path(path_expr) = call.func.as_ref() {
                    let from_segment_args = |seg: &syn::PathSegment| -> Option<syn::Type> {
                        let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
                            return None;
                        };
                        let type_args: Vec<syn::Type> = args
                            .args
                            .iter()
                            .filter_map(|arg| match arg {
                                syn::GenericArgument::Type(ty) => Some(ty.clone()),
                                _ => None,
                            })
                            .collect();
                        if type_args.len() != 2 {
                            return None;
                        }
                        let ok_ty = type_args.first()?.clone();
                        let err_ty = type_args.get(1)?.clone();
                        Some(parse_quote!(Result<#ok_ty, #err_ty>))
                    };
                    path_expr
                        .path
                        .segments
                        .last()
                        .and_then(from_segment_args)
                        .or_else(|| {
                            let owner_seg = path_expr.path.segments.iter().nth_back(1)?;
                            if owner_seg.ident != "Result" {
                                return None;
                            }
                            from_segment_args(owner_seg)
                        })
                } else {
                    None
                }
            } else {
                None
            };
        let mut ctor_expected_hint_owned = if matches!(func_leaf_base, "Ok" | "Err")
            && call.args.len() == 1
        {
            let ctor_idx = if func_leaf_base == "Ok" { 0 } else { 1 };
            effective_resolved_hint.and_then(|hint| {
                self.resolve_result_ctor_expected_type_from_ctor_arg(hint, ctor_idx, &call.args[0])
            })
        } else {
            None
        };
        if matches!(func_leaf_base, "Ok" | "Err")
            && call.args.len() == 1
            && ctor_expected_hint_owned
                .as_ref()
                .is_some_and(|hint| self.type_maps_to_branch_local_decltype(hint))
            && let Some(return_hint) = self.current_return_type_hint()
            && self.type_is_concrete_result_like_hint(return_hint)
        {
            ctor_expected_hint_owned = Some(return_hint.clone());
        }
        let explicit_ctor_hint_usable = explicit_ctor_hint_owned
            .as_ref()
            .is_some_and(|hint| !self.type_maps_to_auto_placeholder_like(hint));
        let ctor_expected_hint_owned_usable = ctor_expected_hint_owned
            .as_ref()
            .is_some_and(|hint| !self.type_maps_to_auto_placeholder_like(hint));
        let ctor_template_args_use_branch_locals = if matches!(func_leaf_base, "Ok" | "Err") {
            self.lookup_constructor_template_args(func_leaf_base)
                .is_some_and(|args| args.iter().any(|arg| arg.contains("decltype((std::move(")))
        } else {
            false
        };
        let effective_decltype_return_hint_owned = if matches!(func_leaf_base, "Ok" | "Err")
            && call.args.len() == 1
            && effective_resolved_hint
                .is_some_and(|hint| self.type_maps_to_branch_local_decltype(hint))
            && let Some(return_hint) = self.current_return_type_hint()
            && self.type_is_concrete_result_like_hint(return_hint)
        {
            Some(return_hint.clone())
        } else {
            None
        };
        let template_decltype_return_hint_owned = if matches!(func_leaf_base, "Ok" | "Err")
            && call.args.len() == 1
            && ctor_template_args_use_branch_locals
            && let Some(return_hint) = self.current_return_type_hint()
            && self.type_is_concrete_result_like_hint(return_hint)
        {
            Some(return_hint.clone())
        } else {
            None
        };
        if matches!(func_leaf_base, "Ok" | "Err")
            && call.args.len() == 1
            && !explicit_ctor_hint_usable
        {
            let fallback_err_ty: syn::Type = parse_quote!(());
            if let Some(resolved_hint) = ctor_expected_hint_owned
                .as_ref()
                .and_then(|hint| {
                    self.resolve_result_error_slot_from_expected(hint, &fallback_err_ty)
                })
                .or_else(|| {
                    explicit_ctor_hint_owned.as_ref().and_then(|hint| {
                        self.resolve_result_error_slot_from_expected(hint, &fallback_err_ty)
                    })
                })
                .or_else(|| {
                    effective_resolved_hint.and_then(|hint| {
                        self.resolve_result_error_slot_from_expected(hint, &fallback_err_ty)
                    })
                })
            {
                ctor_expected_hint_owned = Some(resolved_hint);
            }
        }
        let ctor_expected_hint = ctor_expected_hint_owned
            .as_ref()
            .filter(|_| ctor_expected_hint_owned_usable)
            .or_else(|| {
                if explicit_ctor_hint_usable {
                    explicit_ctor_hint_owned.as_ref()
                } else {
                    None
                }
            })
            .or(ctor_expected_hint_owned.as_ref())
            .or(effective_decltype_return_hint_owned.as_ref())
            .or(template_decltype_return_hint_owned.as_ref())
            .or(effective_resolved_hint);
        if func_leaf_base == "Ok" && call.args.len() == 1 {
            let ok_expected_ty = self.expected_result_type_arg(ctor_expected_hint, 0);
            let arg = if let Some(expected_ok) = ok_expected_ty {
                let expected_ok_ty = self.peel_paren_group_type(expected_ok);
                let expected_ok_maps_to_reference =
                    self.map_type(expected_ok).trim_end().ends_with('&');
                if matches!(expected_ok_ty, syn::Type::Reference(_)) {
                    self.emit_result_ref_constructor_arg(&call.args[0], expected_ok)
                } else if expected_ok_maps_to_reference {
                    // Associated type aliases (for example `Self::Ok`) often
                    // map to reference C++ types without syntactic `Type::Reference`.
                    self.emit_expr_to_string_with_expected(&call.args[0], ok_expected_ty)
                } else {
                    self.emit_expr_to_string_with_expected_and_move_if_needed(
                        &call.args[0],
                        ok_expected_ty,
                    )
                }
            } else {
                self.emit_expr_to_string_with_expected_and_move_if_needed(
                    &call.args[0],
                    ok_expected_ty,
                )
            };
            let arg = self.maybe_move_owned_pattern_binding_value(&call.args[0], arg);
            let arg = self.rewrite_current_assoc_error_paths(&arg);
            if let Some(expected) = ctor_expected_hint {
                let expected_cpp = self.rewrite_current_assoc_error_paths(&self.map_type(expected));
                if expected_cpp.starts_with("rusty::Result<")
                    && !type_string_has_auto_placeholder(&expected_cpp)
                {
                    return format!("{}::Ok({})", expected_cpp, arg);
                }
                // map_type may produce "Result<...>" (without rusty:: prefix) when Result
                // isn't a locally declared type. If the ok type T is concrete (not a type
                // parameter in scope), we can still construct rusty::Result<...>::Ok(...).
                if expected_cpp.starts_with("Result<")
                    && !type_string_has_auto_placeholder(&expected_cpp)
                {
                    if let Some(t_type) = self.expected_result_type_arg(ctor_expected_hint, 0) {
                        let t_mapped =
                            self.rewrite_current_assoc_error_paths(&self.map_type(t_type));
                        let t_has_infer = self.type_contains_infer(t_type);
                        let e_mapped = self
                            .expected_result_type_arg(ctor_expected_hint, 1)
                            .map(|e| self.rewrite_current_assoc_error_paths(&self.map_type(e)))
                            .unwrap_or_else(|| "auto".to_string());
                        let e_has_infer = self
                            .expected_result_type_arg(ctor_expected_hint, 1)
                            .is_some_and(|e| self.type_contains_infer(e));
                        if !t_has_infer
                            && !e_has_infer
                            && !type_string_has_auto_placeholder(&t_mapped)
                            && !type_string_has_auto_placeholder(&e_mapped)
                        {
                            return format!(
                                "rusty::Result<{}, {}>::Ok({})",
                                t_mapped, e_mapped, arg
                            );
                        }
                    }
                }
                if expected_cpp == "rusty::fmt::Result"
                    || expected_cpp == "fmt::Result"
                    || expected_cpp.ends_with("::fmt::Result")
                {
                    return format!("rusty::fmt::Result::Ok({})", arg);
                }
                if Self::cpp_result_alias_is_io_like(&expected_cpp) {
                    return format!("{}::ok({})", expected_cpp, arg);
                }
                if Self::cpp_result_alias_can_use_static_ctor(&expected_cpp) {
                    return format!("{}::Ok({})", expected_cpp, arg);
                }
            }
            if let Some(args) = self.lookup_constructor_template_args("Ok") {
                let ok_ty = self.rewrite_current_assoc_error_paths(&args[0]);
                let err_ty = self.rewrite_current_assoc_error_paths(&args[1]);
                return format!("rusty::Result<{}, {}>::Ok({})", ok_ty, err_ty, arg);
            }
            return format!("rusty::Ok({})", arg);
        }
        if func_leaf_base == "Err" && call.args.len() == 1 {
            let err_expected_ty = self.expected_result_type_arg(ctor_expected_hint, 1);
            let arg = if let Some(expected_err) = err_expected_ty {
                let expected_err_ty = self.peel_paren_group_type(expected_err);
                let expected_err_maps_to_reference =
                    self.map_type(expected_err).trim_end().ends_with('&');
                if matches!(expected_err_ty, syn::Type::Reference(_)) {
                    self.emit_result_ref_constructor_arg(&call.args[0], expected_err)
                } else if expected_err_maps_to_reference {
                    self.emit_expr_to_string_with_expected(&call.args[0], err_expected_ty)
                } else {
                    self.emit_expr_to_string_with_expected_and_move_if_needed(
                        &call.args[0],
                        err_expected_ty,
                    )
                }
            } else {
                self.emit_expr_to_string_with_expected_and_move_if_needed(
                    &call.args[0],
                    err_expected_ty,
                )
            };
            let arg = self.maybe_move_owned_pattern_binding_value(&call.args[0], arg);
            let arg = self.rewrite_current_assoc_error_paths(&arg);
            if let Some(expected) = ctor_expected_hint {
                let expected_cpp = self.rewrite_current_assoc_error_paths(&self.map_type(expected));
                if expected_cpp.starts_with("rusty::Result<")
                    && !type_string_has_auto_placeholder(&expected_cpp)
                {
                    return format!("{}::Err({})", expected_cpp, arg);
                }
                // map_type may produce "Result<...>" (without rusty:: prefix) when Result
                // isn't a locally declared type. If the error type E is concrete (not a type
                // parameter in scope), we can still construct rusty::Result<...>::Err(...).
                if expected_cpp.starts_with("Result<")
                    && !type_string_has_auto_placeholder(&expected_cpp)
                {
                    if let Some(e_type) = self.expected_result_type_arg(ctor_expected_hint, 1) {
                        let e_mapped =
                            self.rewrite_current_assoc_error_paths(&self.map_type(e_type));
                        let e_has_infer = self.type_contains_infer(e_type);
                        let t_mapped = self
                            .expected_result_type_arg(ctor_expected_hint, 0)
                            .map(|t| self.rewrite_current_assoc_error_paths(&self.map_type(t)))
                            .unwrap_or_else(|| "auto".to_string());
                        let t_has_infer = self
                            .expected_result_type_arg(ctor_expected_hint, 0)
                            .is_some_and(|t| self.type_contains_infer(t));
                        if !e_has_infer
                            && !t_has_infer
                            && !type_string_has_auto_placeholder(&t_mapped)
                            && !type_string_has_auto_placeholder(&e_mapped)
                        {
                            return format!(
                                "rusty::Result<{}, {}>::Err({})",
                                t_mapped, e_mapped, arg
                            );
                        }
                    }
                }
                if expected_cpp == "rusty::fmt::Result"
                    || expected_cpp == "fmt::Result"
                    || expected_cpp.ends_with("::fmt::Result")
                {
                    return format!("rusty::fmt::Result::Err({})", arg);
                }
                if Self::cpp_result_alias_is_io_like(&expected_cpp) {
                    return format!("{}::err({})", expected_cpp, arg);
                }
                if Self::cpp_result_alias_can_use_static_ctor(&expected_cpp) {
                    return format!("{}::Err({})", expected_cpp, arg);
                }
            }
            if let Some(args) = self.lookup_constructor_template_args("Err") {
                let ok_ty = self.rewrite_current_assoc_error_paths(&args[0]);
                let err_ty = self.rewrite_current_assoc_error_paths(&args[1]);
                return format!("rusty::Result<{}, {}>::Err({})", ok_ty, err_ty, arg);
            }
            // Fallback for unqualified Err(...) - prefix with rusty:: since we know
            // Result is always in rusty:: namespace and plain Err is not valid C++.
            return format!("rusty::Err({})", arg);
        }
        if let Some(emitted) = self.try_emit_omitted_assoc_static_call_with_arg_decltype(call) {
            return emitted;
        }
        if let Some(emitted) = self.try_emit_cpp_import_bound_member_call(call) {
            return emitted;
        }
        if let syn::Expr::Path(path_expr) = call.func.as_ref()
            && path_expr.path.segments.len() >= 2
            && !call.args.is_empty()
        {
            let owner_segments: Vec<String> = path_expr
                .path
                .segments
                .iter()
                .take(path_expr.path.segments.len() - 1)
                .map(|seg| seg.ident.to_string())
                .collect();
            let owner_path = owner_segments.join("::");
            let owner_tail = owner_segments.last().cloned().unwrap_or_default();
            let mut owner_path_syn = syn::Path {
                leading_colon: path_expr.path.leading_colon,
                segments: syn::punctuated::Punctuated::new(),
            };
            for seg in path_expr
                .path
                .segments
                .iter()
                .take(path_expr.path.segments.len().saturating_sub(1))
            {
                owner_path_syn.segments.push(seg.clone());
            }
            let method_name = path_expr
                .path
                .segments
                .last()
                .map(|seg| seg.ident.to_string())
                .unwrap_or_default();
            let owner_is_serialize_trait =
                owner_tail == "Serialize" || owner_path.ends_with("::Serialize");
            if owner_is_serialize_trait && method_name == "serialize" && call.args.len() == 2 {
                let value = self.emit_expr_to_string(&call.args[0]);
                let serializer = self.emit_expr_maybe_move(&call.args[1]);
                return self.emit_serialize_dispatch_call(&value, &serializer);
            }
            let owner_receiver_shape = self
                .lookup_owner_method_has_receiver_from_owner_path(
                    Some(&owner_path_syn),
                    &owner_tail,
                    &method_name,
                )
                .or_else(|| self.lookup_owner_method_has_receiver(&owner_path, &method_name));
            let receiver_expr = &call.args[0];
            let looks_like_ufcs_receiver_call = self
                .infer_simple_expr_type(receiver_expr)
                .as_ref()
                .is_some_and(|receiver_ty| {
                    let receiver_ty = self.peel_reference_paren_group_type(receiver_ty);
                    let syn::Type::Path(tp) = receiver_ty else {
                        return false;
                    };
                    let receiver_owner = tp
                        .path
                        .segments
                        .iter()
                        .map(|seg| seg.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    !receiver_owner.is_empty()
                        && receiver_owner != owner_path
                        && self.receiver_has_inherent_method_named(receiver_expr, &method_name)
                });
            let looks_like_constructor_static = matches!(
                method_name.as_str(),
                "new"
                    | "new_"
                    | "from"
                    | "try_from"
                    | "from_slice"
                    | "from_vec"
                    | "empty"
                    | "all"
                    | "from_bits"
                    | "from_bits_retain"
                    | "from_bits_truncate"
                    | "from_name"
                    | "from_str"
                    | "default_"
                    | "from_iter"
            );
            if !owner_path.is_empty()
                && !method_name.is_empty()
                && (owner_receiver_shape == Some(true)
                    || (owner_receiver_shape != Some(false)
                        && looks_like_ufcs_receiver_call
                        && !looks_like_constructor_static))
            {
                let is_runtime_helper_owner = (self.module_name.is_some()
                    || self.expanded_libtest_mode)
                    && (self.module_runtime_helper_traits.contains(&owner_tail)
                        || self.module_runtime_helper_traits.contains(&owner_path));
                if is_runtime_helper_owner {
                    // Keep helper UFCS calls in associated-call form to avoid
                    // rewriting `Trait::method(self)` into recursive member calls.
                } else {
                    let method_template_args = self.emit_expr_path_template_args(&path_expr.path);
                    let member_args: Vec<String> = call
                        .args
                        .iter()
                        .skip(1)
                        .map(|arg| self.emit_expr_maybe_move(arg))
                        .collect();
                    if let Some(c_like_callee) = self
                        .resolve_c_like_enum_inherent_method_free_function_for_owner_path(
                            &owner_path,
                            &method_name,
                        )
                    {
                        let mut all_args = Vec::with_capacity(member_args.len() + 1);
                        all_args.push(self.emit_expr_to_string(receiver_expr));
                        all_args.extend(member_args);
                        return format!("{}({})", c_like_callee, all_args.join(", "));
                    }
                    return self.emit_receiver_member_call(
                        receiver_expr,
                        &method_name,
                        method_template_args.as_deref(),
                        &member_args,
                        expected_ty,
                    );
                }
            }
        }

        let callable_type_param_fallback = self.call_targets_callable_type_param(&call.func);
        let mut call_type_substitutions = self
            .function_call_type_arg_substitutions(call)
            .unwrap_or_default();
        if let Some(expected_substitutions) =
            self.call_owner_type_arg_substitutions_from_expected_type(call, expected_ty)
        {
            call_type_substitutions.extend(expected_substitutions);
        }
        let call_type_substitutions = if call_type_substitutions.is_empty() {
            None
        } else {
            Some(call_type_substitutions)
        };
        let call_is_invalid_length = matches!(
            self.peel_paren_group_expr(call.func.as_ref()),
            syn::Expr::Path(path_expr)
                if path_expr
                    .path
                    .segments
                    .last()
                    .is_some_and(|seg| seg.ident == "invalid_length")
        );
        let call_targets_rusty_ext_mut_string = matches!(
            self.peel_paren_group_expr(call.func.as_ref()),
            syn::Expr::Path(path_expr)
                if path_expr
                    .path
                    .segments
                    .last()
                    .is_some_and(|seg| matches!(seg.ident.to_string().as_str(), "clear" | "push_str"))
        );
        let call_expected_ty = expected_ty;
        let call_args: Vec<String> = call
            .args
            .iter()
            .enumerate()
            .map(|(idx, arg)| {
                let declared_arg_expected_ty =
                    self.lookup_function_arg_expected_type(call.func.as_ref(), idx);
                if self
                    .lookup_function_type_param_names(call.func.as_ref())
                    .is_some_and(|params| !params.is_empty())
                    && declared_arg_expected_ty
                        .is_none_or(|expected| self.type_is_bare_generic_param_like(expected))
                    && self
                        .lookup_function_arg_pass_style(&call.func, idx)
                        .is_none()
                    && let syn::Expr::Reference(reference) = self.peel_paren_group_expr(arg)
                    && reference.mutability.is_some()
                    && self.is_stable_reference_lvalue_expr(&reference.expr)
                {
                    return self.emit_explicit_reference_call_arg(reference, None);
                }
                if call_is_invalid_length
                    && idx == 1
                    && let syn::Expr::Reference(reference) = self.peel_paren_group_expr(arg)
                    && !self.is_stable_reference_lvalue_expr(&reference.expr)
                {
                    let inner = self.emit_expr_to_string_with_expected(&reference.expr, None);
                    return format!("rusty::addr_of_temp({})", inner);
                }
                let style = self
                    .lookup_function_arg_pass_style(&call.func, idx)
                    .or_else(|| self.associated_receiver_style_first_arg_pass_style(call, idx));
                let mut arg_expected_ty = self.lookup_function_arg_expected_type_for_call(
                    call,
                    idx,
                    call_type_substitutions.as_ref(),
                );
                let expected_needs_owner_recovery =
                    arg_expected_ty.as_ref().is_some_and(|expected| {
                        self.type_contains_infer(expected)
                            || self.type_contains_in_scope_type_param(expected)
                            || self.type_contains_unresolved_placeholder_like(expected)
                            || self.type_contains_unbound_single_letter_generic(expected)
                            || matches!(
                                self.peel_reference_paren_group_type(expected),
                                syn::Type::Path(tp)
                                    if tp.qself.is_none()
                                        && tp.path.segments.len() == 1
                                        && tp.path.segments[0]
                                            .ident
                                            .to_string()
                                            .chars()
                                            .next()
                                            .is_some_and(|c| c.is_ascii_uppercase())
                            )
                    });
                let arg_is_closure =
                    matches!(self.peel_paren_group_expr(arg), syn::Expr::Closure(_));
                let expected_closure_return_unknown = arg_is_closure
                    && arg_expected_ty.as_ref().is_some_and(|expected| {
                        self.extract_callable_return_type_from_type(expected)
                            .is_none()
                    });
                if (arg_expected_ty.is_none()
                    || expected_needs_owner_recovery
                    || expected_closure_return_unknown)
                    && let Some(fallback) =
                        self.lookup_associated_call_arg_expected_type_fallback(call, idx, Some(arg))
                {
                    let fallback = if let Some(substitutions) = call_type_substitutions.as_ref() {
                        self.substitute_type_params_in_type(&fallback, substitutions)
                    } else {
                        fallback
                    };
                    arg_expected_ty = Some(fallback);
                }
                if (arg_expected_ty.is_none() || expected_closure_return_unknown)
                    && let Some(fallback) = self
                        .infer_associated_call_arg_expected_type_from_call_expected_owner(
                            call,
                            call_expected_ty,
                            idx,
                        )
                {
                    let fallback = if let Some(substitutions) = call_type_substitutions.as_ref() {
                        self.substitute_type_params_in_type(&fallback, substitutions)
                    } else {
                        fallback
                    };
                    arg_expected_ty = Some(fallback);
                }
                if arg_expected_ty.is_none() {
                    arg_expected_ty = self.infer_tuple_struct_constructor_call_arg_expected_type(
                        call,
                        call_expected_ty,
                        idx,
                    );
                }
                if arg_expected_ty.is_none()
                    && let Some(fold_hint) = self
                        .infer_fold_like_init_expected_type_from_call_context(
                            call,
                            idx,
                            call_expected_ty,
                        )
                {
                    arg_expected_ty = Some(fold_hint);
                }
                if (arg_expected_ty.is_none() || expected_needs_owner_recovery)
                    && idx == 0
                    && self.call_expr_is_iter_like(call)
                    && let Some(iter_expected) = self
                        .infer_into_iter_receiver_expected_type_from_call_expected(
                            arg,
                            call_expected_ty,
                        )
                {
                    arg_expected_ty = Some(iter_expected);
                }
                if std::env::var("RUSTY_DEBUG_CALL_EXPECTED").is_ok()
                    && matches!(call.func.as_ref(), syn::Expr::Path(path_expr)
                        if path_expr.path.segments.last().is_some_and(|seg| seg.ident == "content_clone"))
                {
                    let expected_dbg = arg_expected_ty
                        .as_ref()
                        .map(|ty| ty.to_token_stream().to_string())
                        .unwrap_or_else(|| "None".to_string());
                    eprintln!(
                        "[debug-call-expected] idx={} expected={} style={:?}",
                        idx, expected_dbg, style
                    );
                }
                if idx == 0
                    && call_targets_rusty_ext_mut_string
                    && let Some(cow_self_cpp) = self.active_mut_cow_self_cpp_binding()
                {
                    return format!("rusty::to_mut({})", cow_self_cpp);
                }
                if idx == 0
                    && arg_expected_ty
                        .as_ref()
                        .is_some_and(|ty| self.type_is_mut_rusty_string_reference(ty))
                    && self.expr_is_string_view_like(arg)
                    && let Some(cow_self_cpp) = self.active_mut_cow_self_cpp_binding()
                {
                    return format!("rusty::to_mut({})", cow_self_cpp);
                }
                let callable_bound_arg_intent =
                    self.lookup_callable_param_bound_arg_intent(&call.func, idx);
                let reference_arg = match self.peel_paren_group_expr(arg) {
                    syn::Expr::Reference(reference)
                        if self.is_stable_reference_lvalue_expr(&reference.expr) =>
                    {
                        Some(reference)
                    }
                    _ => None,
                };
                let call_has_generic_type_params = self
                    .lookup_function_type_param_names(call.func.as_ref())
                    .is_some_and(|params| !params.is_empty());
                let inferred_expected_matches_borrowed_value =
                    reference_arg.is_some_and(|reference| {
                        call_has_generic_type_params
                            && arg_expected_ty.as_ref().is_some_and(|expected| {
                                self.infer_simple_expr_type(&reference.expr)
                                    .as_ref()
                                    .is_some_and(|inner_ty| {
                                        Self::types_equivalent_by_tokens(inner_ty, expected)
                                    })
                            })
                    });
                let preserve_bare_generic_borrow = style.is_none()
                    && reference_arg.is_some()
                    && (declared_arg_expected_ty
                        .is_some_and(|expected| self.type_is_bare_generic_param_like(expected))
                        || inferred_expected_matches_borrowed_value);
                // `(recv.field)(&mut place)` — a CALL whose callee is a field
                // access (syntactically `Expr::Call` over `Expr::Field`; method
                // calls are `Expr::MethodCall`) — means the field holds a
                // closure / fn value. Rust closures receive `&mut T` / `&T`
                // params as C++ references (their bodies use dot-access on the
                // param, e.g. `self_.method()`), so bind the borrow as a
                // reference rather than taking its address — `&place` would
                // deduce the `auto&&` param as a pointer and break `.`-access.
                // Skip when the param is a raw pointer (intent Pointer).
                let callee_is_field_value_call =
                    matches!(self.peel_paren_group_expr(&call.func), syn::Expr::Field(_));
                let arg_cpp = if callee_is_field_value_call
                    && reference_arg.is_some()
                    && !matches!(callable_bound_arg_intent, Some(CallableArgPassIntent::Pointer))
                {
                    let reference =
                        reference_arg.expect("callee_is_field_value_call requires reference arg");
                    self.emit_expr_to_string_with_expected(&reference.expr, None)
                } else if preserve_bare_generic_borrow {
                    self.emit_explicit_reference_call_arg(
                        reference_arg.expect("preserve_bare_generic_borrow requires reference arg"),
                        None,
                    )
                } else {
                    self.emit_call_arg_with_pass_style(
                        arg,
                        style,
                        arg_expected_ty.as_ref(),
                        callable_type_param_fallback,
                        callable_bound_arg_intent,
                    )
                };
                let arg_cpp = if !matches!(style, Some(ArgPassStyle::Reference))
                    && !arg_cpp.trim_start().starts_with("std::move(")
                    && self.should_move_local_binding_for_owned_expected_value(
                        arg,
                        arg_expected_ty.as_ref(),
                    )
                {
                    format!("std::move({})", arg_cpp)
                } else {
                    arg_cpp
                };
                // Rust `&[T]` params map to std::span — see
                // coerce_slice_expected_arg_cpp.
                let arg_cpp =
                    self.coerce_slice_expected_arg_cpp(arg_cpp, arg_expected_ty.as_ref());
                self.wrap_tuple_struct_constructor_arg_for_by_value_cycle_rewrite(call, idx, arg_cpp)
            })
            .collect();
        let mut args = call_args.clone();
        let suppress_deserialize_template_inference = matches!(
            call.func.as_ref(),
            syn::Expr::Path(path_expr)
                if path_expr
                    .path
                    .segments
                    .last()
                    .is_some_and(|seg| {
                        let ident = seg.ident.to_string();
                        ident == "deserialize" || ident == "deserialize_in_place"
                    })
        );
        let recovered_function_template_args = if suppress_deserialize_template_inference {
            None
        } else {
            self.infer_function_type_template_args_from_pointer_call(call, &call_args)
        };
        let call_has_explicit_type_args = matches!(
            call.func.as_ref(),
            syn::Expr::Path(path_expr)
                if path_expr
                    .path
                    .segments
                    .last()
                    .is_some_and(|seg| matches!(seg.arguments, syn::PathArguments::AngleBracketed(_)))
        );
        // Also try to infer template args from a function-path last argument like `TypeName::all`
        // This handles cases where C++ template deduction cannot infer T from `typename T::Bits`
        let fn_path_template_args = if recovered_function_template_args.is_some()
            || call_has_explicit_type_args
            || suppress_deserialize_template_inference
        {
            None
        } else {
            self.infer_template_args_from_fn_path_return_type(call)
        };
        let const_generic_args = self.local_function_const_generic_call_args(call);
        if !const_generic_args.is_empty() {
            let mut merged = const_generic_args;
            merged.extend(args);
            args = merged;
        }
        if func
            .rsplit("::")
            .next()
            .is_some_and(|name| name == "initialize_or_wait")
            && !args.is_empty()
            && !args[0].contains("deref_if_pointer_like(")
        {
            let first = args[0].clone();
            args[0] = format!("rusty::detail::deref_if_pointer_like({})", first);
        }
        let func = if let Some(template_args) = recovered_function_template_args {
            format!("{}<{}>", func, template_args.join(", "))
        } else if let Some(template_args) = fn_path_template_args {
            format!("{}<{}>", func, template_args.join(", "))
        } else {
            func
        };
        let func = Self::collapse_constructor_like_call_path(&func);
        format!("{}({})", func, args.join(", "))
    }

    pub(super) fn try_emit_reference_coercion_for_expected_option_inner(
        &self,
        arg: &syn::Expr,
        expected_inner_ty: &syn::Type,
    ) -> Option<String> {
        let expected_inner_ty = self.peel_paren_group_type(expected_inner_ty);
        let syn::Type::Reference(expected_ref) = expected_inner_ty else {
            return None;
        };
        let expected_elem_ty = self.peel_reference_paren_group_type(&expected_ref.elem);

        let actual_ty = self.infer_simple_expr_type(arg)?;
        let actual_ty = self.peel_paren_group_type(&actual_ty);
        let syn::Type::Reference(actual_ref) = actual_ty else {
            return None;
        };
        let actual_elem_ty = self.peel_reference_paren_group_type(&actual_ref.elem);
        if Self::types_equivalent_by_tokens(actual_elem_ty, expected_elem_ty) {
            return Some(self.emit_expr_to_string_with_expected(arg, Some(expected_inner_ty)));
        }

        let deref_target = self.extract_deref_target_type_from_inner(actual_elem_ty)?;
        let deref_target = self.peel_reference_paren_group_type(&deref_target);
        if !Self::types_equivalent_by_tokens(deref_target, expected_elem_ty) {
            return None;
        }

        // For a `?`-operand, take the *pointer* form (`&(...unwrap())` inside the
        // statement expression) rather than the value form. A GCC statement
        // expression `({...; unwrap(); })` decays its trailing `const T&` result
        // to a prvalue `T`, which copies the (often move-only) referent before we
        // can deref it. The pointer form keeps a `const T*` result, no copy.
        let inner = self
            .emit_try_expr_reference_pointer(arg)
            .unwrap_or_else(|| self.emit_expr_to_string(arg));
        // Normalize through `deref_if_pointer_like` so the `*` reliably invokes
        // the referent's user `operator*` (Deref) rather than a raw pointer
        // dereference: the coerced `&T` may lower to either `const T*` (pointer)
        // or `const T&` (value/ref) depending on the surrounding expression.
        // Without this, `*ptr` yields `T` instead of `T::Target`, so `span v =
        // *ptr` tries to copy the (often move-only) `T` and fails.
        Some(format!(
            "*rusty::detail::deref_if_pointer_like({})",
            inner
        ))
    }

    pub(super) fn try_emit_variant_constructor_callable(&self, path: &syn::Path) -> Option<String> {
        let ctor_name = self.variant_ctor_name_from_path(path)?;
        if !matches!(
            path.segments.last().map(|seg| &seg.arguments),
            Some(syn::PathArguments::None)
        ) {
            return None;
        }

        let mut owner_name = path
            .segments
            .iter()
            .nth_back(1)
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default();
        let mut recovered_args = self.lookup_constructor_template_args(&ctor_name);
        if recovered_args.is_none() {
            recovered_args = self.recover_variant_constructor_owner_generic_args(path);
        }
        if recovered_args.is_none()
            && let Some(bound_path) =
                self.resolve_single_segment_variant_ctor_import_path(path, &ctor_name)
        {
            if owner_name.is_empty() {
                owner_name = bound_path
                    .segments
                    .iter()
                    .nth_back(1)
                    .map(|seg| seg.ident.to_string())
                    .unwrap_or_default();
            }
            recovered_args = self.recover_variant_constructor_owner_generic_args(&bound_path);
        }
        if recovered_args.is_none()
            && path.segments.len() == 1
            && matches!(ctor_name.as_str(), "Left" | "Right")
            && self.enum_has_variant_name("Either", &ctor_name)
        {
            let owner_path = syn::parse_str::<syn::Path>("Either").ok();
            recovered_args = owner_path
                .as_ref()
                .and_then(|owner| self.recover_omitted_owner_generic_args_from_scope(owner))
                .or_else(|| {
                    syn::parse_str::<syn::Path>("::Either")
                        .ok()
                        .and_then(|owner| {
                            self.recover_omitted_owner_generic_args_from_scope(&owner)
                        })
                });
            if recovered_args.is_some() && owner_name.is_empty() {
                owner_name = "Either".to_string();
            }
        }
        let recovered_args = recovered_args?;
        if recovered_args.len() < 2 {
            return None;
        }

        let ctor_cpp = if owner_name == "Either" {
            format!("rusty::either::{}", ctor_name)
        } else {
            ctor_name
        };
        Some(format!(
            "[](auto&& _v) {{ return {}<{}, {}>(std::forward<decltype(_v)>(_v)); }}",
            ctor_cpp, recovered_args[0], recovered_args[1]
        ))
    }

    /// Detect and emit a general data enum variant constructor call.
    /// E.g., `ErrorKind::LeadingZero(pos)` → `ErrorKind_LeadingZero{pos}`
    /// when `ErrorKind` is a known data enum type.
    pub(super) fn try_emit_data_enum_variant_constructor(&self, call: &syn::ExprCall) -> Option<String> {
        let syn::Expr::Path(ep) = call.func.as_ref() else {
            return None;
        };
        // Need at least 2 segments: EnumName::VariantName
        if ep.path.segments.len() < 2 {
            return None;
        }
        // Extract enum name and variant name from path
        let segments: Vec<String> = ep
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();

        // Skip crate/self/super prefixes
        let (enum_name, variant_name) = {
            let mut start = 0;
            while start < segments.len()
                && matches!(segments[start].as_str(), "crate" | "self" | "super")
            {
                start += 1;
            }
            if start + 2 > segments.len() {
                return None;
            }
            // The second-to-last is the enum name, the last is the variant name
            let enum_idx = segments.len() - 2;
            let variant_idx = segments.len() - 1;
            if enum_idx < start {
                return None;
            }
            (&segments[enum_idx], &segments[variant_idx])
        };

        // Check if enum_name is a known data enum (including scoped owners where
        // the path carries module qualification and only tail ident is present here).
        if !self.data_enum_name_matches(enum_name) {
            return None;
        }
        // Only rewrite real variants; associated methods like
        // `Enum::from_inline(...)` must remain method calls.
        let canonical_variant_name = self.canonical_variant_name(variant_name).to_string();
        if !self.enum_has_variant_name(enum_name, variant_name)
            && !self.enum_has_variant_name(enum_name, &canonical_variant_name)
        {
            return None;
        }

        // Build the C++ variant struct name: EnumName_VariantName
        // Use emit_path_to_string on the enum path (without the variant) to get
        // proper C++ namespace qualification, then append _VariantName.
        let enum_path: syn::Path = {
            let segs: Vec<syn::PathSegment> = ep
                .path
                .segments
                .iter()
                .take(ep.path.segments.len() - 1) // Remove last (variant name)
                .cloned()
                .collect();
            let mut p = ep.path.clone();
            p.segments = segs.into_iter().collect();
            p
        };
        let cpp_variant_struct = self.data_enum_variant_struct_type_name(&enum_path, variant_name);
        let owner_seg_idx = ep.path.segments.len().saturating_sub(2);
        let owner_substitutions =
            self.owner_segment_type_arg_substitutions(&ep.path, owner_seg_idx);

        // Emit args with expected-type/pass-style context so nested constructor
        // calls (for example `Default::default()`) can resolve correctly.
        let args: Vec<String> = call
            .args
            .iter()
            .enumerate()
            .map(|(idx, arg)| {
                let style = self
                    .lookup_function_arg_pass_style(call.func.as_ref(), idx)
                    .or_else(|| self.associated_receiver_style_first_arg_pass_style(call, idx));
                let arg_expected = self
                    .lookup_function_arg_expected_type_for_call(call, idx, None)
                    .or_else(|| {
                        self.lookup_associated_call_arg_expected_type_fallback(call, idx, Some(arg))
                    })
                    .or_else(|| {
                        self.lookup_owner_method_arg_expected_type(
                            enum_name,
                            variant_name,
                            idx,
                            Some(arg),
                        )
                    })
                    .or_else(|| {
                        self.lookup_data_enum_variant_arg_expected_type(
                            &enum_path,
                            enum_name,
                            variant_name,
                            idx,
                        )
                    });
                let arg_expected = arg_expected.map(|ty| {
                    if let Some(substitutions) = owner_substitutions.as_ref() {
                        self.substitute_type_params_in_type(&ty, substitutions)
                    } else {
                        ty
                    }
                });
                self.emit_call_arg_with_pass_style(arg, style, arg_expected.as_ref(), false, None)
            })
            .collect();
        let args =
            self.wrap_data_enum_variant_tuple_constructor_args(enum_name, variant_name, args);

        if args.is_empty() {
            Some(format!("{}{{}}", cpp_variant_struct))
        } else {
            Some(format!("{}{{{}}}", cpp_variant_struct, args.join(", ")))
        }
    }

    pub(super) fn try_emit_c_like_enum_variant_zero_arg_call(&self, call: &syn::ExprCall) -> Option<String> {
        if !call.args.is_empty() {
            return None;
        }
        let func_expr = self.peel_paren_group_expr(call.func.as_ref());
        let syn::Expr::Path(path_expr) = func_expr else {
            return None;
        };
        if path_expr.path.segments.len() < 2 {
            return None;
        }
        let enum_name = path_expr
            .path
            .segments
            .iter()
            .nth_back(1)?
            .ident
            .to_string();
        let variant_name = path_expr.path.segments.last()?.ident.to_string();
        if !self.path_matches_c_like_enum_const(&enum_name, &variant_name) {
            return None;
        }
        Some(self.emit_path_to_string(&path_expr.path))
    }

    pub(super) fn try_emit_variant_constructor_call_with_recovered_hints(
        &self,
        call: &syn::ExprCall,
    ) -> Option<String> {
        let func_path = match call.func.as_ref() {
            syn::Expr::Path(path) => &path.path,
            _ => return None,
        };
        if call.args.len() != 1 {
            return None;
        }

        let ctor_name = self.variant_ctor_name_from_path(func_path)?;
        if !matches!(
            func_path.segments.last().map(|s| &s.arguments),
            Some(syn::PathArguments::None)
        ) {
            return None;
        }

        let args = self
            .lookup_constructor_template_args(&ctor_name)
            .or_else(|| {
                if func_path.segments.len() < 2 {
                    return None;
                }
                let mut owner_path = syn::Path {
                    leading_colon: func_path.leading_colon,
                    segments: syn::punctuated::Punctuated::new(),
                };
                for seg in func_path
                    .segments
                    .iter()
                    .take(func_path.segments.len().saturating_sub(1))
                {
                    owner_path.segments.push(seg.clone());
                }
                self.recover_omitted_owner_generic_args_from_scope(&owner_path)
            })?;
        if args.len() < 2 {
            return None;
        }
        let target_cpp_ty = if ctor_name == "Left" {
            args[0].as_str()
        } else {
            args[1].as_str()
        };
        let arg = self.emit_from_conversion_to_target(&call.args[0], target_cpp_ty);
        let owner_name = func_path
            .segments
            .iter()
            .nth_back(1)
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default();
        let ctor_cpp = if owner_name == "Either" {
            format!("rusty::either::{}", ctor_name)
        } else {
            ctor_name
        };
        Some(format!("{}<{}, {}>({})", ctor_cpp, args[0], args[1], arg))
    }

    pub(super) fn try_emit_variant_constructor_call_with_template_args(
        &self,
        call: &syn::ExprCall,
        template_args: &[String],
    ) -> Option<String> {
        if template_args.len() < 2 {
            return None;
        }
        let func_path = match call.func.as_ref() {
            syn::Expr::Path(path) => &path.path,
            _ => return None,
        };
        if call.args.len() != 1 {
            return None;
        }
        if !matches!(
            func_path.segments.last().map(|s| &s.arguments),
            Some(syn::PathArguments::None)
        ) {
            return None;
        }
        let ctor_name = self.variant_ctor_name_from_path(func_path)?;
        let target_cpp_ty = if ctor_name == "Left" {
            template_args[0].as_str()
        } else {
            template_args[1].as_str()
        };
        let arg = self.emit_from_conversion_to_target(&call.args[0], target_cpp_ty);
        let owner_name = func_path
            .segments
            .iter()
            .nth_back(1)
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default();
        let ctor_cpp = if owner_name == "Either" {
            format!("rusty::either::{}", ctor_name)
        } else {
            ctor_name
        };
        Some(format!(
            "{}<{}, {}>({})",
            ctor_cpp, template_args[0], template_args[1], arg
        ))
    }

    pub(super) fn emit_expr_to_string_with_variant_ctx(
        &self,
        expr: &syn::Expr,
        variant_ctx: Option<&VariantTypeContext>,
    ) -> String {
        match expr {
            syn::Expr::Reference(r) => {
                let inner = self.emit_expr_to_string_with_variant_ctx(&r.expr, variant_ctx);
                // Variant match scrutinees should feed the variant value/reference directly
                // into `std::visit`; emitting `&inner` here turns the scrutinee into a
                // pointer and breaks overload resolution.
                if variant_ctx.is_some() && !self.is_expr_raw_pointer_like(&r.expr) {
                    inner
                } else if inner.starts_with('&') {
                    inner
                } else {
                    format!("&{}", inner)
                }
            }
            syn::Expr::Paren(p) => {
                let inner = self.emit_expr_to_string_with_variant_ctx(&p.expr, variant_ctx);
                format!("({})", inner)
            }
            syn::Expr::Group(g) => self.emit_expr_to_string_with_variant_ctx(&g.expr, variant_ctx),
            syn::Expr::Call(call) => {
                if let Some(ctx) = variant_ctx {
                    if let Some(emitted) = self
                        .try_emit_variant_constructor_call_with_template_args(
                            call,
                            &ctx.template_args,
                        )
                    {
                        return emitted;
                    }
                }
                self.emit_call_expr_to_string(call, None)
            }
            _ => self.emit_expr_to_string(expr),
        }
    }

    /// Intercept known derived-trait UFCS calls where args may not be
    /// references (e.g., `core::cmp::Ord::cmp(std::move(a), std::move(b))`).
    /// Returns `Some(emitted)` if the call matches a known trait method.
    /// Lower the fully-qualified std iterator-trait UFCS calls that
    /// itertools' macros emit (surfaced verbatim by `cargo expand`) into
    /// receiver-method form, reusing the existing `.map()` / `.zip()` /
    /// `.into_iter()` lowering:
    ///
    ///   `<..>::iter::Iterator::<method>(recv, args..)`  → `recv.<method>(args..)`
    ///   `<..>::iter::IntoIterator::into_iter(recv)`      → `recv.into_iter()`
    ///
    /// The owning module is the std iterator module under any of its
    /// spellings — `std::iter`, `core::iter`, or the `__std_iter` alias
    /// (`pub use std::iter as __std_iter`) re-exported by itertools. The
    /// trait segment must be immediately preceded by that module segment, so
    /// user types named `Iterator`/`IntoIterator` are not affected.
    pub(super) fn try_emit_std_iter_trait_ufcs_call(&self, call: &syn::ExprCall) -> Option<String> {
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        let segs: Vec<String> = path_expr
            .path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect();
        if segs.len() < 3 || call.args.is_empty() {
            return None;
        }
        let method = &segs[segs.len() - 1];
        let trait_seg = &segs[segs.len() - 2];
        let module_seg = &segs[segs.len() - 3];
        if module_seg != "iter" && module_seg != "__std_iter" {
            return None;
        }
        let is_iterator_method = trait_seg == "Iterator"
            && method
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_lowercase());
        let is_into_iter = trait_seg == "IntoIterator" && method == "into_iter";
        if !is_iterator_method && !is_into_iter {
            return None;
        }
        // First argument is the receiver; the rest become method arguments.
        let receiver = call.args.first()?.clone();
        let mut method_args = syn::punctuated::Punctuated::new();
        for arg in call.args.iter().skip(1) {
            method_args.push(arg.clone());
        }
        let method_call = syn::ExprMethodCall {
            attrs: Vec::new(),
            receiver: Box::new(receiver),
            dot_token: syn::token::Dot::default(),
            method: syn::Ident::new(method, proc_macro2::Span::call_site()),
            turbofish: None,
            paren_token: syn::token::Paren::default(),
            args: method_args,
        };
        Some(self.emit_expr_to_string(&syn::Expr::MethodCall(method_call)))
    }

    pub(super) fn try_emit_known_trait_ufcs_call(&self, call: &syn::ExprCall) -> Option<String> {
        let func_path = match call.func.as_ref() {
            syn::Expr::Path(p) => &p.path,
            _ => return None,
        };
        if func_path.segments.len() < 2 {
            return None;
        }
        let method = func_path.segments.last()?.ident.to_string();
        let trait_seg = func_path.segments.iter().nth_back(1)?.ident.to_string();

        // `Clone::clone(&x)` is the canonical UFCS form, but Rust also accepts
        // the impl-anchored form `Rc::clone(&x)` / `Arc::clone(&x)` /
        // `MyType::clone(&x)` (and the std reference docs actively recommend
        // it as a readability convention for refcounted handles). Both forms
        // resolve to the same `Clone::clone` trait method; we route both to
        // `rusty::clone(arg)`, which SFINAE-dispatches to the member
        // `.clone()` when one exists (the Rc/Arc port case) and falls back
        // to copy-construction otherwise.
        //
        // First, query the local method-has-receiver map. When the owner is
        // defined in another module (e.g. `Rc` from rc_port called from
        // smallvec), the map is empty for that owner and the lookup returns
        // None — fall back to a hardcoded allowlist of refcount/handle
        // owners whose ports are known to expose `.clone()` as a member.
        // This intentionally does NOT cover arbitrary user types; for those
        // we keep the existing static-call shape.
        const KNOWN_CLONE_OWNERS: &[&str] = &[
            "Rc", "Arc", "Weak", "Box", "RefCell", "Cell", "Mutex", "RwLock",
        ];
        let owner_clone_takes_receiver = self
            .lookup_owner_method_has_receiver(&trait_seg, "clone")
            .unwrap_or_else(|| KNOWN_CLONE_OWNERS.contains(&trait_seg.as_str()));
        let is_clone_ufcs = method == "clone"
            && call.args.len() == 1
            && (trait_seg == "Clone" || owner_clone_takes_receiver);
        if is_clone_ufcs {
            let receiver_expr = match call.args.first() {
                Some(syn::Expr::Reference(reference)) => reference.expr.as_ref(),
                Some(other) => other,
                None => return None,
            };
            let receiver = self.emit_expr_to_string(receiver_expr);
            let preserve_reference_identity = self
                .infer_simple_expr_type(receiver_expr)
                .as_ref()
                .is_some_and(|ty| {
                    let ty = self.peel_reference_paren_group_type(ty);
                    if let syn::Type::Reference(r) = ty {
                        self.type_is_reference_like(&r.elem)
                    } else {
                        false
                    }
                });
            if preserve_reference_identity {
                // `Clone::clone(&ref_like)` should preserve borrowed identity.
                return Some(receiver);
            }
            return Some(format!("rusty::clone({})", receiver));
        }

        // Match known trait methods by trait name + method name.
        let emit_fn = match (trait_seg.as_str(), method.as_str()) {
            ("Ord", "cmp") if call.args.len() == 2 => "rusty::cmp::cmp",
            ("PartialOrd", "partial_cmp") if call.args.len() == 2 => "rusty::partial_cmp",
            ("Hash", "hash") if call.args.len() == 2 => "rusty::hash::hash",
            // UFCS form `PartialEq::eq(a, b)` / `PartialEq::ne(a, b)` (common in
            // cargo-expand output and blanket impls, e.g. equivalent's
            // `Equivalent` impl). Symmetric with the `.eq()`/`.ne()` method-call
            // lowering (emit_expr.rs ~6315) — both target the same `rusty::cmp`
            // runtime helpers, which SFINAE-dispatch to `.eq()` or `==`.
            ("PartialEq", "eq") if call.args.len() == 2 => "rusty::cmp::eq",
            ("PartialEq", "ne") if call.args.len() == 2 => "rusty::cmp::ne",
            // UFCS `io::Write::write_all(writer, buf)` (serde_yaml's emitter
            // write handler calls through the trait path on the stored boxed
            // writer). The runtime free fn SFINAE-dispatches to a member
            // `.write_all` (DynWrite) or falls back to chunked `write`s.
            ("Write", "write_all") if call.args.len() == 2 => "rusty::io::write_all",
            _ => return None,
        };

        // Emit args, stripping outer & references (UFCS passes &receiver).
        let args: Vec<String> = call
            .args
            .iter()
            .map(|a| match a {
                syn::Expr::Reference(r) => self.emit_expr_to_string(&r.expr),
                _ => self.emit_expr_maybe_move(a),
            })
            .collect();
        Some(format!("{}({})", emit_fn, args.join(", ")))
    }

    pub(super) fn try_emit_trait_ufcs_by_value_receiver_call(&self, call: &syn::ExprCall) -> Option<String> {
        let func_path = match call.func.as_ref() {
            syn::Expr::Path(path) => &path.path,
            _ => return None,
        };
        if func_path.segments.len() < 2 || call.args.is_empty() {
            return None;
        }
        let emit_receiver_arg = |arg: &syn::Expr| match self.peel_paren_group_expr(arg) {
            syn::Expr::Reference(r) => self.emit_expr_to_string(&r.expr),
            _ => self.emit_expr_maybe_move(arg),
        };

        let method_name = func_path.segments.last()?.ident.to_string();
        // Trait method identifiers are Rust snake_case; UpperCamelCase names are
        // typically enum variants / associated constructors (`Type::Variant(...)`).
        // Do not reinterpret those as UFCS receiver calls.
        if method_name
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            return None;
        }
        let path_segments: Vec<String> = func_path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect();
        let owner_segments: Vec<String> = func_path
            .segments
            .iter()
            .take(func_path.segments.len().saturating_sub(1))
            .map(|seg| seg.ident.to_string())
            .collect();
        let owner_leaf = owner_segments.last()?.to_string();
        let owner_path = owner_segments.join("::");
        let mut owner_path_syn = syn::Path {
            leading_colon: func_path.leading_colon,
            segments: syn::punctuated::Punctuated::new(),
        };
        for seg in func_path
            .segments
            .iter()
            .take(func_path.segments.len().saturating_sub(1))
        {
            owner_path_syn.segments.push(seg.clone());
        }
        let owner_receiver_shape = self.lookup_owner_method_has_receiver_from_owner_path(
            Some(&owner_path_syn),
            &owner_leaf,
            &method_name,
        );
        let trait_receiver_shape = self.trait_static_call_has_receiver_for_segments(&path_segments);

        // In module/expanded modes, local trait runtime helpers are emitted as
        // concrete helper structs. Keep UFCS calls targeting those helpers in
        // associated-call form (`Trait::method(self, ...)`) to avoid rewriting
        // into self-recursive receiver calls (`self.method(...)`).
        let key_scoped = format!("{}::{}", owner_path, method_name);
        let key_unscoped = format!("{}::{}", owner_leaf, method_name);
        let is_runtime_helper_method = (self.module_name.is_some() || self.expanded_libtest_mode)
            && (self.module_runtime_helper_traits.contains(&owner_leaf)
                || self.module_runtime_helper_traits.contains(&owner_path))
            && (self.trait_method_has_receiver.contains_key(&key_scoped)
                || self.trait_method_has_receiver.contains_key(&key_unscoped)
                || trait_receiver_shape.is_some());
        let serde_trait_like_owner = matches!(
            owner_leaf.as_str(),
            "Serializer"
                | "SerializeStruct"
                | "SerializeTuple"
                | "SerializeTupleStruct"
                | "SerializeTupleVariant"
                | "SerializeMap"
                | "SerializeStructVariant"
                | "Deserializer"
                | "SeqAccess"
                | "MapAccess"
                | "EnumAccess"
                | "VariantAccess"
        ) || owner_leaf.ends_with("Deserializer");
        let runtime_by_value_receiver_owner =
            matches!(owner_leaf.as_str(), "Option" | "Result" | "Formatter");
        let private_runtime_owner_alias = runtime_by_value_receiver_owner
            && owner_segments
                .iter()
                .any(|seg| seg.starts_with("__private"));
        let receiver_is_self_path = call
            .args
            .first()
            .map(|arg| self.peel_paren_group_expr(arg))
            .is_some_and(|arg| {
                matches!(arg, syn::Expr::Path(p)
                    if p.path.segments.len() == 1 && p.path.segments[0].ident == "self")
            });
        if method_name == "deserialize"
            && call.args.len() == 1
            && self.is_type_param_in_scope(&owner_leaf)
        {
            let receiver = self.emit_deserializer_call_arg(call.args.first()?);
            let seed_cpp = escape_cpp_keyword(&owner_leaf);
            return Some(format!(
                "::de::rusty_ext::deserialize(rusty::PhantomData<{}>{{}}, {})",
                seed_cpp, receiver
            ));
        }
        if method_name == "deserialize"
            && call.args.len() == 1
            && owner_leaf == "Deserialize"
            && let Some(seed) = self.resolve_unique_trait_bound_type_param("Deserialize")
        {
            let receiver = self.emit_deserializer_call_arg(call.args.first()?);
            let seed_cpp = escape_cpp_keyword(&seed);
            return Some(format!(
                "::de::rusty_ext::deserialize(rusty::PhantomData<{}>{{}}, {})",
                seed_cpp, receiver
            ));
        }
        if is_runtime_helper_method && receiver_is_self_path {
            return None;
        }

        if (matches!(owner_receiver_shape, Some(false))
            || matches!(
                self.lookup_owner_method_has_receiver(&owner_leaf, &method_name),
                Some(false)
            ))
            && !private_runtime_owner_alias
        {
            return None;
        }
        let fallback_trait_receiver = (serde_trait_like_owner || runtime_by_value_receiver_owner)
            && !matches!(method_name.as_str(), "new" | "new_" | "from" | "try_from");
        let has_receiver = owner_receiver_shape
            .or_else(|| self.lookup_owner_method_has_receiver(&owner_leaf, &method_name))
            .or(trait_receiver_shape)
            .or_else(|| self.trait_method_has_receiver.get(&key_scoped).copied())
            .or_else(|| self.trait_method_has_receiver.get(&key_unscoped).copied())
            .unwrap_or(fallback_trait_receiver)
            || private_runtime_owner_alias;
        if !has_receiver {
            return None;
        }

        // Constructor-like/static methods should stay associated calls.
        if matches!(method_name.as_str(), "new" | "new_" | "from" | "try_from") {
            return None;
        }

        let joined = path_segments.join("::");
        if method_name == "fmt"
            && call.args.len() == 2
            && matches!(
                joined.as_str(),
                "Display::fmt"
                    | "fmt::Display::fmt"
                    | "core::fmt::Display::fmt"
                    | "std::fmt::Display::fmt"
                    | "rusty::fmt::Display::fmt"
            )
        {
            let value = emit_receiver_arg(call.args.first()?);
            let formatter = self.emit_expr_maybe_move(call.args.iter().nth(1)?);
            return Some(format!(
                "rusty::write_fmt({}, rusty::to_string({}))",
                formatter, value
            ));
        }
        if method_name == "fmt"
            && call.args.len() == 2
            && matches!(
                joined.as_str(),
                "Debug::fmt"
                    | "fmt::Debug::fmt"
                    | "core::fmt::Debug::fmt"
                    | "std::fmt::Debug::fmt"
                    | "rusty::fmt::Debug::fmt"
            )
        {
            let value = emit_receiver_arg(call.args.first()?);
            let formatter = self.emit_expr_maybe_move(call.args.iter().nth(1)?);
            return Some(format!(
                "rusty::write_fmt({}, rusty::to_debug_string({}))",
                formatter, value
            ));
        }

        let map_seq_access_ufcs = matches!(owner_leaf.as_str(), "MapAccess" | "SeqAccess")
            && matches!(
                method_name.as_str(),
                "next_key"
                    | "next_key_seed"
                    | "next_value"
                    | "next_value_seed"
                    | "next_entry"
                    | "next_entry_seed"
                    | "next_element"
                    | "next_element_seed"
            );
        if map_seq_access_ufcs {
            let receiver = emit_receiver_arg(call.args.first()?);
            let extra_args: Vec<String> = call
                .args
                .iter()
                .skip(1)
                .map(|arg| self.emit_expr_maybe_move(arg))
                .collect();
            let mut template_args = self
                .emit_expr_path_template_args(func_path)
                .unwrap_or_default();
            if method_name == "next_key" && extra_args.is_empty() {
                if let Some(last_seg) = func_path.segments.last()
                    && let syn::PathArguments::AngleBracketed(ab) = &last_seg.arguments
                    && ab.args.len() == 1
                    && let Some(syn::GenericArgument::Type(seed_ty)) = ab.args.first()
                {
                    let seed_ty = self.peel_reference_paren_group_type(seed_ty);
                    if let syn::Type::Path(tp) = seed_ty
                        && tp.qself.is_none()
                        && tp.path.segments.len() == 1
                        && tp.path.segments[0].arguments.is_empty()
                    {
                        let seed_ident = tp.path.segments[0].ident.to_string();
                        if seed_ident.starts_with("__Field") {
                            let seed_cpp = self.map_type(seed_ty);
                            if seed_cpp != "auto"
                                && !seed_cpp.contains("/* TODO")
                                && !type_string_has_auto_placeholder(&seed_cpp)
                            {
                                let visitor_cpp =
                                    format!("{}Visitor", escape_cpp_keyword(&seed_ident));
                                let seed_adapter = format!(
                                    "::de::detail::identifier_seed<{}, {}>{{}}",
                                    seed_cpp, visitor_cpp
                                );
                                let seed_call = self
                                    .emit_extension_call_with_receiver_autoderef_fallback(
                                        "::de::rusty_ext::next_key_seed",
                                        &receiver,
                                        &[seed_adapter],
                                    );
                                let fallback_call = self
                                    .emit_extension_call_with_receiver_autoderef_fallback(
                                        &format!("::de::rusty_ext::next_key<{}>", seed_cpp),
                                        &receiver,
                                        &[],
                                    );
                                return Some(format!(
                                    "([&]() -> decltype(auto) {{ if constexpr (requires {{ typename {}::Value; requires std::is_same_v<typename {}::Value, {}>; {}; }}) {{ return {}; }} else {{ return {}; }} }})()",
                                    visitor_cpp,
                                    visitor_cpp,
                                    seed_cpp,
                                    seed_call,
                                    seed_call,
                                    fallback_call
                                ));
                            }
                        }
                    }
                }
            }
            // Rust infers `MapAccess::next_value`'s generic from return position.
            // C++ cannot deduce function-template args from return type.
            if template_args.is_empty() && extra_args.is_empty() {
                match method_name.as_str() {
                    "next_value" => {
                        // Rust infers `MapAccess::next_value`'s generic from return position.
                        // C++ cannot deduce function-template args from return type.
                        template_args = "<std::tuple<>>".to_string();
                    }
                    "next_element" => {
                        // Rust default generic for `SeqAccess::next_element`.
                        template_args = "<::de::IgnoredAny>".to_string();
                    }
                    "next_entry" => {
                        // Rust default generics for `MapAccess::next_entry`.
                        template_args = "<::de::IgnoredAny, ::de::IgnoredAny>".to_string();
                    }
                    _ => {}
                }
            }
            let callee = format!("::de::rusty_ext::{}{}", method_name, template_args);
            return Some(self.emit_extension_call_with_receiver_autoderef_fallback(
                &callee,
                &receiver,
                &extra_args,
            ));
        }

        if method_name == "deserialize_any" && call.args.len() == 2 {
            let receiver = emit_receiver_arg(call.args.first()?);
            let visitor = self.emit_expr_maybe_move(call.args.iter().nth(1)?);
            return Some(format!(
                "::de::rusty_ext::deserialize_any({}, {})",
                receiver, visitor
            ));
        }
        if matches!(
            method_name.as_str(),
            "deserialize_bytes" | "deserialize_byte_buf"
        ) && call.args.len() == 2
        {
            let receiver = emit_receiver_arg(call.args.first()?);
            let visitor = self.emit_expr_maybe_move(call.args.iter().nth(1)?);
            return Some(format!(
                "::de::rusty_ext::deserialize_any({}, {})",
                receiver, visitor
            ));
        }
        if method_name == "deserialize_in_place" && call.args.len() == 2 {
            let receiver = emit_receiver_arg(call.args.first()?);
            let place = self.emit_expr_maybe_move(call.args.iter().nth(1)?);
            return Some(format!(
                "::de::rusty_ext::deserialize_in_place({}, {})",
                receiver, place
            ));
        }
        if method_name == "serialize" && call.args.len() == 2 {
            let value = emit_receiver_arg(call.args.first()?);
            let serializer = self.emit_expr_maybe_move(call.args.iter().nth(1)?);
            return Some(self.emit_serialize_dispatch_call(&value, &serializer));
        }

        // UFCS qualified disambiguation (see the detect_ufcs_trait_method_call
        // handler): `Trait::method(recv, …)` / `<T as Trait>::method(recv, …)`
        // lowers to the qualified free function `<Trait>_::method(recv, …)`.
        // Gate on the owner map (a CONCRETE impl of THIS trait emits the free
        // function), not just "crate-declared" — default trait methods and
        // runtime-helper (assoc-const) traits have no `<Trait>_::m`, so
        // qualifying them is a HARD error; fall through to the member call.
        // Placed AFTER the serde/Display/Debug special-cases above.
        if self
            .ufcs_method_trait_owners
            .get(&method_name)
            .is_some_and(|owners| owners.contains(&owner_leaf))
        {
            let recv = match self.peel_paren_group_expr(call.args.first()?) {
                syn::Expr::Reference(r) => self.emit_expr_to_string(&r.expr),
                other => self.emit_expr_to_string(other),
            };
            let extra_args: Vec<String> = call
                .args
                .iter()
                .skip(1)
                .map(|arg| match arg {
                    syn::Expr::Reference(r) => self.emit_expr_to_string(&r.expr),
                    _ => self.emit_expr_maybe_move(arg),
                })
                .collect();
            // Route through the member-fallback shim rather than a bare
            // `<Tr>_::m(recv, …)`: a FOREIGN trait (declared in another crate,
            // e.g. serde_core's `Serializer`) implemented for a type whose impl
            // is emitted only as a class MEMBER (serde_test's `ser::Serializer`)
            // has no `<Tr>_::m` free-function overload for that self type — so a
            // bare qualified call binds the WRONG overload (a sibling impl) and
            // hard-errors. The 3-branch shim's `else { deref(__self).m(args) }`
            // member branch resolves the member impl. (Unifies this by-value
            // trait-static path with the method-syntax path, which already uses
            // this shim.)
            let callee = format!(
                "{}::{}",
                self.ufcs_trait_namespace(&owner_leaf),
                escape_cpp_keyword(&method_name),
            );
            return Some(self.emit_extension_call_with_receiver_autoderef_fallback(
                &callee,
                &recv,
                &extra_args,
            ));
        }

        let receiver_expr = self.peel_paren_group_expr(call.args.first()?);
        let args: Vec<String> = call
            .args
            .iter()
            .skip(1)
            .map(|arg| self.emit_expr_maybe_move(arg))
            .collect();
        Some(self.emit_receiver_member_call(receiver_expr, &method_name, None, &args, None))
    }

    /// If this is a variant-constructor call like `Left(2)` and the expected type
    /// is known (e.g., `Either<i32, i32>`), emit explicit template args:
    /// `Left<int32_t, int32_t>(2)`.
    pub(super) fn try_emit_variant_constructor_call_with_expected(
        &self,
        call: &syn::ExprCall,
        expected_ty: &syn::Type,
    ) -> Option<String> {
        let func_path = match call.func.as_ref() {
            syn::Expr::Path(path) => &path.path,
            _ => return None,
        };
        let ctor_name = self.variant_ctor_name_from_path(func_path)?;

        // Keep existing special mappings intact.
        if matches!(ctor_name.as_str(), "Some" | "Ok" | "Err") {
            return None;
        }

        // Heuristic: enum variants are CamelCase and typically start uppercase.
        if !ctor_name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase())
        {
            return None;
        }

        let mut expected_args = self
            .expected_type_template_args(expected_ty)
            .unwrap_or_default();
        if expected_args.is_empty() {
            let mut owner_path = syn::Path {
                leading_colon: func_path.leading_colon,
                segments: syn::punctuated::Punctuated::new(),
            };
            for seg in func_path
                .segments
                .iter()
                .take(func_path.segments.len().saturating_sub(1))
            {
                owner_path.segments.push(seg.clone());
            }
            if let Some(recovered) = self.recover_omitted_owner_generic_args_from_scope(&owner_path)
                && !recovered.is_empty()
            {
                expected_args = recovered;
            }
        }
        if expected_args.is_empty() {
            return None;
        }
        if !matches!(
            func_path.segments.last().map(|s| &s.arguments),
            Some(syn::PathArguments::None)
        ) {
            return None;
        }
        if expected_args.len() < 2 {
            return None;
        }
        let target_cpp_ty = if ctor_name == "Left" {
            expected_args[0].as_str()
        } else {
            expected_args[1].as_str()
        };
        let is_left_or_right_ctor = matches!(ctor_name.as_str(), "Left" | "Right");

        let args: Vec<String> = call
            .args
            .iter()
            .map(|a| self.emit_from_conversion_to_target(a, target_cpp_ty))
            .collect();

        let ctor_cpp =
            if is_left_or_right_ctor && self.map_type(expected_ty).starts_with("rusty::Either<") {
                format!("rusty::either::{}", ctor_name)
            } else {
                ctor_name.clone()
            };

        Some(format!(
            "{}<{}>({})",
            ctor_cpp,
            expected_args.join(", "),
            args.join(", ")
        ))
    }

    /// If this is `IterEither::new_(...)` in expression position with a known
    /// expected return type, emit a fully-specialized static call:
    /// `iterator::IterEither<A, B>::new_(...)`.
    pub(super) fn try_emit_iter_either_new_call_with_expected(
        &self,
        call: &syn::ExprCall,
        expected_ty: &syn::Type,
    ) -> Option<String> {
        let is_iter_either_new = match call.func.as_ref() {
            syn::Expr::Path(path) => {
                let segs: Vec<&syn::PathSegment> = path.path.segments.iter().collect();
                if segs.len() < 2 {
                    false
                } else {
                    let method_seg = segs[segs.len() - 1];
                    let type_seg = segs[segs.len() - 2];
                    method_seg.ident == "new_" && type_seg.ident == "IterEither"
                }
            }
            _ => {
                let func = self.emit_expr_to_string(&call.func);
                func.ends_with("IterEither::new_")
            }
        };
        if !is_iter_either_new {
            return None;
        }

        let expected_cpp_ty = self.map_type(expected_ty);
        if !expected_cpp_ty.contains("IterEither<") {
            return None;
        }

        let args: Vec<String> = call
            .args
            .iter()
            .map(|a| self.emit_expr_maybe_move(a))
            .collect();
        Some(format!("{}::new_({})", expected_cpp_ty, args.join(", ")))
    }

    pub(super) fn emit_expr_to_string(&self, expr: &syn::Expr) -> String {
        match expr {
            syn::Expr::Lit(lit) => self.emit_lit(&lit.lit),
            syn::Expr::Path(path) => {
                if let Some(_qself) = &path.qself {
                    if path.path.segments.len() == 1 {
                        match path.path.segments[0].ident.to_string().as_str() {
                            "cast_mut" => return "rusty::ptr::cast_mut".to_string(),
                            "cast_const" => return "rusty::ptr::cast_const".to_string(),
                            _ => {}
                        }
                    }
                }
                if path.path.segments.len() == 2
                    && let Some(owner) = path.path.segments.first()
                    && let Some(last) = path.path.segments.last()
                    && self.canonical_variant_name(&last.ident.to_string()) == "None"
                    && owner.ident.to_string().starts_with("__private")
                {
                    return "rusty::None".to_string();
                }
                if let Some(lambda) = self.try_emit_recursive_nested_fn_value_reference(&path.path)
                {
                    return lambda;
                }
                if let Some(lambda) = self.try_emit_method_reference_lambda(&path.path) {
                    return lambda;
                }
                // `PhantomData` as a VALUE expression (no expected type to drive
                // the element — e.g. a UFCS member-fallback shim arg whose lambda
                // param is `auto&&`). It must be a CONSTRUCTED value, never the
                // bare class-template name `rusty::PhantomData` (a parse error in
                // value position: "expected '(' for function-style cast"). Mirror
                // the with-expected fallback in emit_expr_to_string_with_expected.
                if path.path.segments.last().is_some_and(|seg| {
                    seg.ident == "PhantomData"
                        && (matches!(seg.arguments, syn::PathArguments::None)
                            || matches!(
                                seg.arguments,
                                syn::PathArguments::AngleBracketed(ref ab) if ab.args.is_empty()
                            ))
                }) {
                    return "rusty::PhantomData<std::tuple<>>{}".to_string();
                }
                self.emit_expr_path_to_string(&path.path)
            }
            syn::Expr::Group(group) => self.emit_expr_to_string(&group.expr),
            syn::Expr::Binary(bin) => self.emit_binary_expr_to_string_with_expected(bin, None),
            syn::Expr::Unary(un) => match un.op {
                syn::UnOp::Neg(_) => {
                    let operand = self.emit_expr_to_string(&un.expr);
                    format!("-{}", operand)
                }
                syn::UnOp::Not(_) => {
                    let operand_ty = self.infer_simple_expr_type(&un.expr);
                    let use_bitwise_not = operand_ty
                        .as_ref()
                        .is_some_and(|ty| self.is_known_integer_like_type(ty))
                        || self.expr_is_probably_bitwise_not_operand(&un.expr);
                    if use_bitwise_not {
                        let operand = if let Some(operand_ty) = operand_ty.as_ref() {
                            self.emit_expr_to_string_with_expected(&un.expr, Some(operand_ty))
                        } else {
                            self.emit_expr_to_string(&un.expr)
                        };
                        format!("~{}", operand)
                    } else {
                        let bool_ty: syn::Type = parse_quote!(bool);
                        let operand =
                            self.emit_expr_to_string_with_expected(&un.expr, Some(&bool_ty));
                        format!("!{}", operand)
                    }
                }
                syn::UnOp::Deref(_) => {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::ByteStr(bs),
                        ..
                    }) = self.peel_paren_group_expr(&un.expr)
                    {
                        let bytes = bs.value();
                        if bytes.is_empty() {
                            return "std::array<uint8_t, 0>{}".to_string();
                        }
                        let elems: Vec<String> = bytes
                            .iter()
                            .map(|b| format!("static_cast<uint8_t>({})", b))
                            .collect();
                        return format!("std::array{{{}}}", elems.join(", "));
                    }
                    if let syn::Expr::Path(path) = self.peel_paren_group_expr(&un.expr) {
                        if path.path.segments.len() == 1 {
                            let name = path.path.segments[0].ident.to_string();
                            let local_binding_lowers_to_pointer =
                                self.is_reference_binding_lowered_to_pointer_storage(&name);
                            if self.should_collapse_untyped_iterator_map_param_deref(&name) {
                                return self.emit_expr_to_string(&un.expr);
                            }
                            if self.should_lower_untyped_closure_param_deref(&name) {
                                let operand = self.emit_expr_to_string(&un.expr);
                                return format!("rusty::deref_mut({})", operand);
                            }
                            if local_binding_lowers_to_pointer {
                                let operand = self
                                    .lookup_local_binding_cpp_name(&name)
                                    .unwrap_or_else(|| escape_cpp_keyword(&name));
                                return format!("*{}", operand);
                            }
                            if matches!(name.as_str(), "left_val" | "right_val")
                                && self.lookup_local_binding_type(&name).as_ref().is_some_and(
                                    |ty| {
                                        matches!(
                                            self.peel_reference_paren_group_type(ty),
                                            syn::Type::Ptr(_)
                                        )
                                    },
                                )
                            {
                                return self.emit_expr_to_string(&un.expr);
                            }
                            if name == "ptr" || name.ends_with("_ptr") {
                                let local_is_known_pointer = self
                                    .lookup_local_binding_type(&name)
                                    .as_ref()
                                    .is_some_and(|ty| {
                                        matches!(
                                            self.peel_reference_paren_group_type(ty),
                                            syn::Type::Ptr(_)
                                        )
                                    });
                                if local_is_known_pointer {
                                    let operand = self.emit_expr_to_string(&un.expr);
                                    return format!("*{}", operand);
                                }
                            }
                        }
                    }
                    let collapse_local_nonpointer_path = if let syn::Expr::Path(path) =
                        self.peel_paren_group_expr(&un.expr)
                    {
                        if path.path.segments.len() == 1 {
                            let local_name = path.path.segments[0].ident.to_string();
                            if self.is_reference_binding_lowered_to_pointer_storage(&local_name) {
                                false
                            } else {
                                let local_ty = self.lookup_local_binding_type(&local_name);
                                let local_is_known_deref_owner =
                                    local_ty.as_ref().is_some_and(|local_ty| {
                                        let peeled_local =
                                            self.peel_reference_paren_group_type(local_ty);
                                        matches!(peeled_local, syn::Type::Path(tp)
                                        if tp.path.segments.last().is_some_and(|seg| {
                                            matches!(
                                                seg.ident.to_string().as_str(),
                                                "Box"
                                                    | "Rc"
                                                    | "Arc"
                                                    | "Lazy"
                                                    | "Ref"
                                                    | "RefMut"
                                                    | "MutexGuard"
                                                    | "SpinMutexGuard"
                                                    | "RwLockReadGuard"
                                                    | "RwLockWriteGuard"
                                            )
                                        }))
                                    });
                                let local_is_known_pointer =
                                    local_ty.as_ref().is_some_and(|local_ty| {
                                        matches!(
                                            self.peel_reference_paren_group_type(local_ty),
                                            syn::Type::Ptr(_)
                                        )
                                    });
                                let local_type_is_concrete = local_ty.as_ref().is_some_and(|ty| {
                                    !self.type_contains_infer(ty)
                                        && !self.type_contains_in_scope_type_param(ty)
                                        && !self.type_contains_unbound_single_letter_generic(ty)
                                        && !self.type_contains_unresolved_placeholder_like(ty)
                                });
                                let local_is_in_scope =
                                    self.lookup_local_binding_cpp_name(&local_name).is_some();
                                local_is_in_scope
                                    && local_type_is_concrete
                                    && !local_is_known_deref_owner
                                    && !local_is_known_pointer
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    if collapse_local_nonpointer_path {
                        return self.emit_expr_to_string(&un.expr);
                    }
                    if matches!(self.peel_paren_group_expr(&un.expr), syn::Expr::Field(_))
                        && !self.is_expr_raw_pointer_like(&un.expr)
                        && !self.method_receiver_is_manually_drop_expr(&un.expr)
                        && self
                            .infer_simple_expr_type(&un.expr)
                            .as_ref()
                            .is_some_and(|ty| self.type_is_reference_like(ty))
                    {
                        return self.emit_expr_to_string(&un.expr);
                    }
                    if (self.is_expr_reference_like(&un.expr)
                        || self.is_self_reference_field_access(&un.expr))
                        && self.unary_deref_should_collapse_reference_like_operand(&un.expr)
                    {
                        self.emit_expr_to_string(&un.expr)
                    } else {
                        let operand = self.emit_expr_to_string(&un.expr);
                        if self.in_deref_method_scope() {
                            format!("rusty::deref_ref({})", operand)
                        } else if self.in_deref_mut_method_scope() {
                            if self.should_fallback_to_deref_ref_in_deref_mut_scope() {
                                format!("rusty::deref_ref({})", operand)
                            } else {
                                format!("rusty::deref_mut({})", operand)
                            }
                        } else if self.is_expr_raw_pointer_like(&un.expr) {
                            format!("*{}", operand)
                        } else if self
                            .infer_simple_expr_type(&un.expr)
                            .or_else(|| self.infer_local_binding_type_from_initializer(&un.expr))
                            .as_ref()
                            .is_some_and(|ty| {
                                let peeled = self.peel_reference_paren_group_type(ty);
                                matches!(peeled, syn::Type::Path(tp)
                                    if tp.path.segments.last().is_some_and(|seg| matches!(
                                        seg.ident.to_string().as_str(),
                                        "Ref" | "RefMut"
                                            | "MutexGuard"
                                            | "SpinMutexGuard"
                                            | "RwLockReadGuard"
                                            | "RwLockWriteGuard"
                                    )))
                            })
                        {
                            // For RAII guard wrappers (`Ref<T>`, `RefMut<T>`,
                            // `MutexGuard<T>`, ...) the operator* is the
                            // intended access path. Emit `*guard` directly so
                            // `*guard = expr` lowers as a real assignment
                            // through the guard rather than getting routed
                            // through the generic helper (which makes the
                            // intent fuzzy and can also break SFINAE on
                            // assignment).
                            format!("*{}", operand)
                        } else if self
                            .infer_simple_expr_type(&un.expr)
                            .or_else(|| self.infer_local_binding_type_from_initializer(&un.expr))
                            .as_ref()
                            .is_some_and(|ty| {
                                !matches!(
                                    self.peel_reference_paren_group_type(ty),
                                    syn::Type::Reference(_) | syn::Type::Ptr(_)
                                )
                            })
                        {
                            // Route value-surface unary deref through the generic helper:
                            // this preserves Deref-like operator* behavior while avoiding
                            // invalid direct `*value` on slice/span-backed models.
                            format!("rusty::detail::deref_if_pointer_like({})", operand)
                        } else {
                            format!("rusty::detail::deref_if_pointer_like({})", operand)
                        }
                    }
                }
                _ => {
                    let operand = self.emit_expr_to_string(&un.expr);
                    format!("/* unknown unary */ {}", operand)
                }
            },
            syn::Expr::Reference(r) => {
                let ref_inner = self.peel_paren_group_expr(&r.expr);
                if self.in_deref_method_scope() || self.in_deref_mut_method_scope() {
                    return self.emit_expr_to_string(ref_inner);
                }
                if let syn::Expr::Index(idx) = ref_inner {
                    if self.is_slice_range_index_expr(&idx.index) {
                        return self.emit_expr_to_string(ref_inner);
                    }
                }
                if let syn::Expr::Unary(un) = ref_inner {
                    if matches!(un.op, syn::UnOp::Deref(_)) {
                        let operand = self.peel_paren_group_expr(&un.expr);
                        // Collapse &*expr reborrow for simple single-deref cases:
                        // - `&*r` where r is a simple path variable → just `r`
                        // Do NOT collapse for:
                        // - Raw pointers (`*p` is a real dereference)
                        // - ManuallyDrop types (`*md` calls operator* for unwrapping)
                        // - Nested derefs like `&**self` (Deref trait)
                        if matches!(operand, syn::Expr::Path(_) | syn::Expr::Field(_))
                            && !self.is_expr_raw_pointer_like(&un.expr)
                            && !self.method_receiver_is_manually_drop_expr(&un.expr)
                        {
                            return self.emit_expr_to_string(&un.expr);
                        }
                        // Original collapse guard for reference-like operands
                        if self.should_collapse_reborrow_of_deref_operand(&un.expr)
                            && !self.method_receiver_is_manually_drop_expr(&un.expr)
                        {
                            return self.emit_expr_to_string(ref_inner);
                        }
                    }
                }
                if let syn::Expr::MethodCall(mc) = self.peel_paren_group_expr(&r.expr)
                    && mc.method == "into_bytes"
                {
                    // `&err.into_bytes()` in expanded serde-style code should lower as
                    // byte buffer value (later coerced to span), not address-of container.
                    return self.emit_expr_to_string(&r.expr);
                }
                let inner = self.emit_expr_to_string(&r.expr);
                if !self.is_stable_reference_lvalue_expr(&r.expr) {
                    return format!("rusty::addr_of_temp({})", inner);
                }
                if inner.starts_with('&') {
                    // Reuse an existing address-of expression for nested borrows.
                    // Emitting `&(&expr)` forms an rvalue pointer temporary and then
                    // takes its address, which is ill-formed in C++.
                    inner
                } else {
                    format!("&{}", inner)
                }
            }
            syn::Expr::RawAddr(raw) => self.emit_raw_addr_expr_to_string(raw),
            syn::Expr::Call(call) => self.emit_call_expr_to_string(call, None),
            syn::Expr::MethodCall(mc) => self.emit_method_call_expr_to_string(mc, None),
            syn::Expr::Field(f) => {
                // Check if base is `self`.
                let is_self = matches!(f.base.as_ref(), syn::Expr::Path(p)
                    if p.path.segments.len() == 1 && p.path.segments[0].ident == "self");
                if is_self {
                    let emitted_member = match &f.member {
                        syn::Member::Named(ident) => {
                            let rust_name = ident.to_string();
                            if self
                                .lookup_local_binding_type("self")
                                .is_some_and(|ty| self.type_is_range_with_private_end_field(&ty))
                            {
                                let self_expr =
                                    if let Some(self_name) = self.current_self_path_override() {
                                        self_name.to_string()
                                    } else {
                                        "(*this)".to_string()
                                    };
                                if rust_name == "end" {
                                    return format!("rusty::field_end({})", self_expr);
                                }
                                if rust_name == "start" {
                                    return format!("rusty::field_start({})", self_expr);
                                }
                            }
                            let mut emitted = self
                                .current_struct
                                .as_ref()
                                .and_then(|s| self.lookup_struct_field_cpp_name(s, &rust_name))
                                .unwrap_or_else(|| escape_cpp_keyword(&rust_name));
                            emitted
                        }
                        syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                    };
                    if let Some(self_name) = self.current_self_path_override() {
                        format!("{}.{}", self_name, emitted_member)
                    } else {
                        // Preserve receiver qualification so local bindings that match
                        // field names do not shadow `self` field reads/writes.
                        format!("this->{}", emitted_member)
                    }
                } else {
                    let base = self.emit_expr_to_string(&f.base);
                    let base_for_field =
                        if self.expr_base_needs_explicit_deref_for_field_access(&f.base) {
                            if self.method_receiver_needs_parentheses(&f.base) {
                                format!("(*({}))", base)
                            } else {
                                format!("(*{})", base)
                            }
                        } else {
                            base.clone()
                        };
                    match &f.member {
                        syn::Member::Named(ident) => {
                            let rust_name = ident.to_string();
                            // Deref coercion: when `field` is not a member of
                            // base's own struct but base's user `Deref::Target`
                            // has it, Rust auto-derefs; C++ does not, so access
                            // through `(*base)` (unsafe-libyaml `success.fail`).
                            if let Some(coerced) = self
                                .field_access_through_user_deref(&f.base, &base_for_field, &rust_name)
                            {
                                return coerced;
                            }
                            if self.expr_base_is_range_with_private_end_field(&f.base) {
                                if rust_name == "end" {
                                    return format!("rusty::field_end({})", base_for_field);
                                }
                                if rust_name == "start" {
                                    return format!("rusty::field_start({})", base_for_field);
                                }
                            }
                            let mut emitted = self
                                .lookup_field_cpp_name_for_expr_base(&f.base, &rust_name)
                                .unwrap_or_else(|| escape_cpp_keyword(&rust_name));
                            if rust_name == "end" && emitted == "end" {
                                return format!("rusty::field_end({})", base_for_field);
                            }
                            format!("{}.{}", base_for_field, emitted)
                        }
                        syn::Member::Unnamed(idx) => {
                            if self.expr_base_is_tuple_like_for_field_access(&f.base) {
                                format!("std::get<{}>({})", idx.index, base_for_field)
                            } else if self.infer_simple_expr_type(&f.base).is_some() {
                                // Base type is known and not tuple-like — must
                                // be a transpiler-synthesized tuple-struct
                                // whose members are named `_0`, `_1`, ….
                                format!("{}._{}", base_for_field, idx.index)
                            } else {
                                // Item 1 (GENERIC_FIXES_PLAN): receiver type
                                // is genuinely unknown (e.g. `auto&&`-bound
                                // through deref chains or returned from a
                                // method whose owner we can't trace). Pick
                                // the right field-access form at C++ compile
                                // time via a `requires` SFINAE dispatch:
                                //   - tuple-struct shape → `__t._N`
                                //   - std::tuple-like   → `std::get<N>(__t)`
                                // Evaluates the base exactly once.
                                // Lambda uses only its parameter `__t`; no
                                // capture is needed. Use `[]` (not `[&]`)
                                // so this expression remains well-formed
                                // when it appears nested inside another
                                // `requires { ... }` clause — Clang 21+
                                // rejects "non-local lambda expression
                                // cannot have a capture-default" when a
                                // lambda with a capture-default lives
                                // inside an unevaluated requires-operand.
                                format!(
                                    "([](auto&& __t) -> decltype(auto) {{ \
                                       if constexpr (requires {{ __t._{1}; }}) \
                                         return (std::forward<decltype(__t)>(__t)._{1}); \
                                       else \
                                         return std::get<{1}>(std::forward<decltype(__t)>(__t)); \
                                     }})({0})",
                                    base_for_field, idx.index
                                )
                            }
                        }
                    }
                }
            }
            syn::Expr::If(if_expr) => self.emit_if_expr_to_string(if_expr, None),
            syn::Expr::Break(brk) => {
                match &brk.expr {
                    Some(val) => {
                        let v = self.emit_expr_to_string(val);
                        format!("return {}", v) // break-with-value inside lambda wrapper
                    }
                    None => {
                        // A plain `break` / `break 'label`. If it can't reach its
                        // target loop with a C++ `break;` (an outer loop, or a
                        // `switch` from a lowered `match` sits in between), emit a
                        // `goto` to the loop's break label instead.
                        let label = brk.label.as_ref().map(|lt| lt.ident.to_string());
                        match self.cf_break_goto(label.as_deref()) {
                            Some(goto_label) => format!("goto {}", goto_label),
                            None => "break".to_string(),
                        }
                    }
                }
            }
            syn::Expr::Continue(cont) => {
                // A plain `continue` / `continue 'label`. A goto is needed only to
                // reach an outer loop (a `switch` does not catch `continue`).
                let label = cont.label.as_ref().map(|lt| lt.ident.to_string());
                match self.cf_continue_goto(label.as_deref()) {
                    Some(goto_label) => format!("goto {}", goto_label),
                    None => "continue".to_string(),
                }
            }
            syn::Expr::Range(range) => {
                let start = range.start.as_ref().map(|e| self.emit_expr_to_string(e));
                let end = range.end.as_ref().map(|e| self.emit_expr_to_string(e));
                let is_inclusive = matches!(range.limits, syn::RangeLimits::Closed(_));

                match (start, end, is_inclusive) {
                    (Some(s), Some(e), false) => format!("rusty::range({}, {})", s, e),
                    (Some(s), Some(e), true) => format!("rusty::range_inclusive({}, {})", s, e),
                    (Some(s), None, _) => format!("rusty::range_from({})", s),
                    (None, Some(e), false) => format!("rusty::range_to({})", e),
                    (None, Some(e), true) => format!("rusty::range_to_inclusive({})", e),
                    (None, None, _) => "rusty::range_full()".to_string(),
                }
            }
            syn::Expr::Closure(closure) => self.emit_closure_to_string(closure),
            syn::Expr::Return(ret) => {
                let keyword = if self.in_async { "co_return" } else { "return" };
                match &ret.expr {
                    Some(e) => {
                        let mut val = self
                            .emit_expr_to_string_with_expected(e, self.current_return_type_hint());
                        if matches!(
                            self.peel_paren_group_expr(e),
                            syn::Expr::Path(path)
                                if path.path.segments.len() == 1 && path.path.segments[0].ident == "self"
                        ) && !self.current_self_receiver_is_reference()
                        {
                            val = if let Some(self_name) = self.current_self_path_override() {
                                format!("std::move({})", self_name)
                            } else {
                                "std::move((*this))".to_string()
                            };
                        } else if self.return_expr_should_move_local(e)
                            && !val.starts_with("std::move(")
                        {
                            val = format!("std::move({})", val);
                        }
                        format!("{} {}", keyword, val)
                    }
                    None => keyword.to_string(),
                }
            }
            syn::Expr::Await(aw) => {
                let inner = self.emit_expr_to_string(&aw.base);
                format!("co_await {}", inner)
            }
            syn::Expr::Assign(a) => {
                let left_peeled = self.peel_paren_group_expr(&a.left);
                let is_discard_assign = matches!(left_peeled, syn::Expr::Infer(_))
                    || matches!(left_peeled, syn::Expr::Path(path)
                        if path.path.segments.len() == 1 && path.path.segments[0].ident == "_");
                if is_discard_assign {
                    let right = self.emit_expr_to_string(&a.right);
                    return format!(
                        "[&]() {{ static_cast<void>({}); return std::make_tuple(); }}()",
                        right
                    );
                }
                if let syn::Expr::Path(path) = self.peel_paren_group_expr(&a.left) {
                    if path.path.segments.len() == 1 {
                        let name = path.path.segments[0].ident.to_string();
                        if self.is_reference_binding_lowered_to_pointer_storage(&name) {
                            let left = self
                                .lookup_local_binding_cpp_name(&name)
                                .unwrap_or_else(|| escape_cpp_keyword(&name));
                            let right = self.emit_rebind_reference_assignment_rhs(&a.right);
                            return format!("{} = {}", left, right);
                        }
                    }
                }
                let mut delayed_init_local: Option<String> = None;
                let expected_ty = match self.peel_paren_group_expr(&a.left) {
                    syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                        let name = path.path.segments[0].ident.to_string();
                        if self.is_delayed_init_local(&name) {
                            delayed_init_local = Some(name.clone());
                        }
                        self.lookup_local_binding_type(&name).map(|local_ty| {
                            self.expected_reference_inner_type(Some(&local_ty))
                                .cloned()
                                .unwrap_or(local_ty)
                        })
                    }
                    syn::Expr::Field(field_expr) => match &field_expr.member {
                        syn::Member::Named(member) => self
                            .lookup_field_type_for_expr_base(&field_expr.base, &member.to_string()),
                        syn::Member::Unnamed(_) => None,
                    },
                    deref_left @ syn::Expr::Unary(u) if matches!(u.op, syn::UnOp::Deref(_)) => {
                        // `*x = ...`: the target is the deref'd value type. For a
                        // real reference / smart-pointer binding the deref yields
                        // it (`&mut T`/`Box<T>`/`*mut T` -> T). An iter_mut
                        // for-binding (`for byte in xs.iter_mut()`) is recorded as
                        // its ELEMENT value `T` (the C++ ref-elision model), so the
                        // deref yields nothing — fall back to the binding's own
                        // value type, since assigning through `*byte` still targets
                        // `T`. Seeds the `next_element()?.ok_or(())?` chain
                        // (serde_bytes ByteArray) with the element type. The
                        // fallback only fires when the deref produced nothing,
                        // which in well-formed code is exactly this mis-recorded
                        // iter-binding case.
                        let inner = self.peel_paren_group_expr(&u.expr);
                        self.infer_simple_expr_type(inner)
                            .and_then(|t| self.infer_deref_result_type_from_type(&t))
                            .or_else(|| self.infer_simple_expr_type(inner))
                            .or_else(|| self.infer_simple_expr_type(deref_left))
                    }
                    _ => None,
                }
                .or_else(|| self.infer_simple_expr_type(&a.left));
                let right = self.emit_expr_to_string_with_expected_and_move_if_needed(
                    &a.right,
                    expected_ty.as_ref(),
                );
                let right = self.maybe_move_owned_pattern_binding_value(&a.right, right);
                let right = self.maybe_move_local_binding_assignment_rhs(&a.right, right);
                if let Some(name) = delayed_init_local {
                    let left = self
                        .lookup_local_binding_cpp_name(&name)
                        .unwrap_or_else(|| escape_cpp_keyword(&name));
                    format!("{}.emplace({})", left, right)
                } else {
                    let left = self.emit_expr_to_string(&a.left);
                    format!("{} = {}", left, right)
                }
            }
            syn::Expr::Struct(s) => self.emit_struct_expr_to_string_with_expected(s, None),
            syn::Expr::Paren(p) => {
                let inner = self.emit_expr_to_string(&p.expr);
                format!("({})", inner)
            }
            syn::Expr::Cast(c) => self.emit_cast_expr_to_string_with_target_override(c, None),
            syn::Expr::Index(idx) => {
                if let Some(slice_expr) = self.try_emit_slice_index_expr_to_string(idx, None) {
                    return slice_expr;
                }
                if let Some(unreachable_expr) =
                    self.try_emit_empty_array_index_expr_to_string(idx, None)
                {
                    return unreachable_expr;
                }
                self.emit_index_expr_to_string(idx, None)
            }
            syn::Expr::Tuple(tup) => {
                let tuple_expected_ty = self.infer_expected_type_from_tuple_elements(&tup.elems);
                let elems: Vec<String> = tup
                    .elems
                    .iter()
                    .map(|e| {
                        self.emit_expr_to_string_with_expected_and_move_if_needed(
                            e,
                            tuple_expected_ty.as_ref(),
                        )
                    })
                    .collect();
                format!("std::make_tuple({})", elems.join(", "))
            }
            syn::Expr::Array(array) => self.emit_array_expr_to_string_with_expected(array, None),
            syn::Expr::Macro(m) => self.emit_macro_expr(&m.mac),
            syn::Expr::Block(block_expr) => {
                if let Some(expr_str) = self.block_expr_to_iife_string(&block_expr.block, None) {
                    expr_str
                } else {
                    self.match_expr_unreachable_fallback().to_string()
                }
            }
            syn::Expr::Unsafe(unsafe_expr) => {
                if let Some(single_expr) = self.extract_single_expr_from_block(&unsafe_expr.block) {
                    self.emit_expr_to_string(single_expr)
                } else if let Some(expr_str) =
                    self.block_expr_to_iife_string(&unsafe_expr.block, None)
                {
                    expr_str
                } else {
                    self.match_expr_unreachable_fallback().to_string()
                }
            }
            syn::Expr::Match(match_expr) => self.emit_match_expr_to_string(match_expr, None),
            syn::Expr::Try(try_expr) => {
                // Rust `expr?` → C++ try macro variant selected by return context.
                // Option-returning contexts use *_TRY_OPT and others use *_TRY.
                let inner = self.emit_expr_to_string(&try_expr.expr);
                self.emit_try_macro_invocation_with_mode(
                    &inner,
                    self.try_operand_prefers_plain_try_macro(&try_expr.expr),
                )
            }
            syn::Expr::Repeat(rep) => {
                // [val; N] → std::array filled with val
                if let Some(fixed_array_expr) =
                    self.maybe_emit_repeat_expr_with_size_of_len_hint(rep)
                {
                    fixed_array_expr
                } else {
                    let val = self.emit_expr_to_string(&rep.expr);
                    let len = self.emit_expr_to_string(&rep.len);
                    format!("rusty::array_repeat({}, {})", val, len)
                }
            }
            // Cluster D: Rust 2024 `const { EXPR }` is a compile-time fence
            // (typically used for `assert!` checks on associated constants
            // like `size_of::<T>() == N`). It must not execute at runtime.
            // Statement-level uses are caught earlier in `emit_stmt` and
            // elided entirely; in expression position (e.g. as a block
            // tail) lower to a comment + `(void)0` so the surrounding
            // expression context stays well-formed without injecting a
            // runtime `unreachable()` call.
            syn::Expr::Const(_) => {
                "/* const-block elided (Rust 2024 compile-time fence) */ (void)0".to_string()
            }
            _ => self.match_expr_unreachable_fallback().to_string(),
        }
    }

    pub(super) fn try_emit_io_read_write_buffer_call(&self, mc: &syn::ExprMethodCall) -> Option<String> {
        let method = mc.method.to_string();
        if !matches!(
            method.as_str(),
            "read" | "read_exact" | "write" | "write_all"
        ) {
            return None;
        }
        if mc.args.len() != 1 {
            return None;
        }
        // Guardrail: pointer-valued receiver writes (for example `ptr.add(i).write(v)`)
        // must be lowered via `rusty::ptr::*` helpers, not IO buffer member surfaces.
        let raw_receiver = self.emit_expr_to_string(&mc.receiver);
        if self.is_expr_raw_pointer_like(&mc.receiver)
            || Self::emitted_pointer_add_or_offset_call(&raw_receiver)
        {
            return None;
        }
        let declared_expected = self.lookup_method_arg_expected_type(&method, 0);
        let inferred_expected_from_receiver = self.infer_method_arg_expected_type_from_receiver(
            &mc.receiver,
            &method,
            0,
            declared_expected,
            mc.args.first(),
        );
        let inferred_expected =
            if declared_expected.is_none() && inferred_expected_from_receiver.is_none() {
                self.infer_slice_arg_expected_type_from_receiver(&mc.receiver, &method, 0)
                    .or_else(|| {
                        if matches!(method.as_str(), "write" | "write_all") {
                            self.infer_byte_write_expected_type_from_receiver_hint(&mc.receiver)
                        } else {
                            None
                        }
                    })
            } else {
                None
            };
        let arg_expected = inferred_expected_from_receiver
            .as_ref()
            .or(declared_expected)
            .or(inferred_expected.as_ref());
        let byte_write_expected = if matches!(method.as_str(), "write" | "write_all") {
            match arg_expected {
                Some(ty) if self.type_is_u8_slice_like(ty) => Some(ty),
                _ => None,
            }
        } else {
            None
        };
        let receiver_name = match self.peel_paren_group_expr(&mc.receiver) {
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                Some(path.path.segments[0].ident.to_string())
            }
            _ => None,
        };
        let receiver = self.emit_expr_to_string(&mc.receiver);
        let is_self = receiver_name.as_deref() == Some("self");
        let arg = mc.args.first()?;
        let write_arg_is_byte_buffer = matches!(method.as_str(), "write" | "write_all")
            && (byte_write_expected.is_some()
                || self
                    .infer_simple_expr_type(arg)
                    .as_ref()
                    .is_some_and(|ty| self.type_is_u8_slice_like(ty))
                || self.try_emit_array_repeat_as_u8(arg).is_some());
        let arg_is_reference = matches!(arg, syn::Expr::Reference(_));
        let arg_expr = match arg {
            syn::Expr::Reference(arg_ref) => {
                self.emit_io_read_write_buffer_view_expr(&arg_ref.expr, byte_write_expected)
            }
            _ => self
                .try_emit_slice_full_buffer_arg_expr(arg, byte_write_expected)
                .unwrap_or_else(|| {
                    self.emit_expr_to_string_with_expected_and_move_if_needed(
                        arg,
                        byte_write_expected,
                    )
                }),
        };

        // Leaf 4.39: expanded `for_both`/match-lowered io methods bind payload as `inner`
        // and can instantiate non-io branches. Route read/write through helper dispatch so
        // non-member payload branches (e.g. spans) compile and fall back deterministically.
        if matches!(method.as_str(), "read" | "write")
            && receiver_name.as_deref() == Some("inner")
            && (method != "write" || write_arg_is_byte_buffer)
        {
            return Some(format!("rusty::io::{}({}, {})", method, receiver, arg_expr));
        }

        let receiver_ty = self.infer_simple_expr_type(&mc.receiver);
        let receiver_is_generic_or_unknown = receiver_ty.as_ref().is_none_or(|ty| {
            self.type_is_bare_generic_param_like(ty)
                || self.type_contains_in_scope_type_param(ty)
                || self.type_contains_unresolved_placeholder_like(ty)
        });
        if matches!(method.as_str(), "write" | "write_all")
            && receiver_is_generic_or_unknown
            && write_arg_is_byte_buffer
        {
            if let Some((cond, then_arg, else_arg)) =
                Self::split_top_level_conditional_expr(&arg_expr)
                && Self::emitted_expr_is_std_array_temp(&then_arg)
                && Self::emitted_expr_is_std_array_temp(&else_arg)
            {
                return Some(format!(
                    "([&]() -> decltype(auto) {{ if ({}) {{ return rusty::io::{}({}, {}); }} else {{ return rusty::io::{}({}, {}); }} }}())",
                    cond, method, receiver, then_arg, method, receiver, else_arg
                ));
            }
            return Some(format!("rusty::io::{}({}, {})", method, receiver, arg_expr));
        }

        let escaped_method = escape_cpp_keyword(&method);

        // Existing normalization for by-reference buffer calls: `read(&buf)`/`write(&buf)` ->
        // `read(rusty::slice_full(buf))`/`write(rusty::slice_full(buf))`.
        if !arg_is_reference {
            if !matches!(method.as_str(), "write" | "write_all") || byte_write_expected.is_none() {
                return None;
            }
            if is_self {
                return Some(format!("{}({})", escaped_method, arg_expr));
            }
            return Some(format!("{}.{}({})", receiver, escaped_method, arg_expr));
        }
        if is_self {
            return Some(format!("{}({})", escaped_method, arg_expr));
        }
        Some(format!("{}.{}({})", receiver, escaped_method, arg_expr))
    }

    pub(super) fn try_emit_iter_filter_map_call(&self, mc: &syn::ExprMethodCall) -> Option<String> {
        if mc.method != "filter_map" || mc.args.len() != 1 {
            return None;
        }

        let iter_call = match self.peel_paren_group_expr(&mc.receiver) {
            syn::Expr::MethodCall(inner) => inner,
            _ => return None,
        };
        if iter_call.method != "iter" || !iter_call.args.is_empty() {
            return None;
        }

        let receiver = self.emit_expr_to_string(&iter_call.receiver);
        let mapper = self.emit_expr_maybe_move(mc.args.first()?);
        Some(format!("rusty::filter_map({}, {})", receiver, mapper))
    }

    pub(super) fn try_emit_fixed_array_map_call(
        &self,
        mc: &syn::ExprMethodCall,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        if mc.method != "map" || mc.args.len() != 1 {
            return None;
        }
        if self.receiver_is_option_or_result_like_expr(&mc.receiver) {
            return None;
        }
        let expected_array = self.expected_fixed_array_type(expected_ty);
        if !self.receiver_is_fixed_array_like_expr(&mc.receiver) && expected_array.is_none() {
            return None;
        }

        let receiver_expected_owned = self.infer_simple_expr_type(&mc.receiver);
        let receiver =
            self.emit_expr_to_string_with_expected(&mc.receiver, receiver_expected_owned.as_ref());
        let mapper_arg = self.peel_paren_group_expr(mc.args.first()?);
        let expected_return_ty = expected_array.map(|(elem_ty, _)| (*elem_ty).clone());
        let expected_return_rt =
            expected_return_ty.map(|ty| syn::ReturnType::Type(Default::default(), Box::new(ty)));
        let mapper = if let syn::Expr::Closure(closure) = mapper_arg {
            let map_param_scope = self.collect_closure_param_names_for_scope(closure);
            self.emit_closure_to_string_with_param_scopes(
                closure,
                Some(map_param_scope),
                None,
                expected_return_rt.as_ref(),
            )
        } else if let Some(callable) = self
            .infer_iter_item_type_with_generic_fallback(&mc.receiver)
            .and_then(|item_ty| {
                self.try_emit_path_callable_arg_to_target(mc.args.first()?, &item_ty)
            })
            .or_else(|| {
                // Path-callable mappers (`Some`, `Option::unwrap`) need their
                // dedicated lowering — a raw path is not a valid C++ callable.
                self.try_emit_assoc_method_path_as_forwarding_lambda(mc.args.first()?)
            })
        {
            callable
        } else {
            self.emit_call_arg_with_pass_style(mc.args.first()?, None, None, false, None)
        };
        Some(format!("rusty::map({}, {})", receiver, mapper))
    }

    pub(super) fn try_emit_iter_map_call(
        &self,
        mc: &syn::ExprMethodCall,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        if mc.method != "map" || mc.args.len() != 1 {
            return None;
        }
        if let syn::Expr::MethodCall(inner) = self.peel_paren_group_expr(&mc.receiver) {
            if inner.args.is_empty()
                && matches!(inner.method.to_string().as_str(), "next" | "next_back")
            {
                return None;
            }
        }
        if self.receiver_is_option_or_result_like_expr(&mc.receiver) {
            return None;
        }
        if !self.is_iterator_like_receiver_expr(&mc.receiver)
            && !self.is_probably_iterator_receiver_expr(&mc.receiver)
        {
            return None;
        }
        let receiver =
            if let syn::Expr::MethodCall(inner_mc) = self.peel_paren_group_expr(&mc.receiver) {
                if inner_mc.method == "into_iter" && inner_mc.args.is_empty() {
                    if !self.is_iterator_like_receiver_expr(&inner_mc.receiver)
                        && !self.is_probably_iterator_receiver_expr(&inner_mc.receiver)
                    {
                        let inner = self.emit_expr_maybe_move(&inner_mc.receiver);
                        format!("rusty::iter({})", inner)
                    } else {
                        self.emit_expr_to_string(&mc.receiver)
                    }
                } else {
                    self.emit_expr_to_string(&mc.receiver)
                }
            } else {
                self.emit_expr_to_string(&mc.receiver)
            };
        let mapper_arg = self.peel_paren_group_expr(mc.args.first()?);
        let expected_item_ty = expected_ty
            .and_then(|ty| self.extract_iter_item_type_from_type(ty))
            .or_else(|| self.expected_vec_element_type(expected_ty).cloned());
        let mapper = if let syn::Expr::Closure(closure) = mapper_arg {
            // Thread the expected item type into the mapper's return position
            // (mirrors `try_emit_fixed_array_map_call`): a struct-literal tail
            // whose generic associated-call fields are pinned only by the
            // collect target (`Bucket { .., value: MaybeUninit::uninit() }`
            // under `iter: vec::IntoIter<Bucket<K, MaybeUninit<V>>>`) needs
            // the element type as the closure's expected return.
            let expected_return_rt = expected_item_ty
                .as_ref()
                .map(|ty| syn::ReturnType::Type(Default::default(), Box::new(ty.clone())));
            let map_param_scope = self.collect_closure_param_names_for_scope(closure);
            self.emit_closure_to_string_with_param_scopes(
                closure,
                Some(map_param_scope),
                None,
                expected_return_rt.as_ref(),
            )
        } else if let Some(item_ty) = expected_item_ty.as_ref()
            && let Some(callable) =
                self.try_emit_path_callable_arg_to_target(mc.args.first()?, item_ty)
        {
            callable
        } else if let Some(forwarding) =
            self.try_emit_assoc_method_path_as_forwarding_lambda(mc.args.first()?)
        {
            forwarding
        } else {
            self.emit_call_arg_with_pass_style(mc.args.first()?, None, None, false, None)
        };
        Some(format!("rusty::map({}, {})", receiver, mapper))
    }

    pub(super) fn try_emit_iter_enumerate_call(&self, mc: &syn::ExprMethodCall) -> Option<String> {
        if mc.method != "enumerate" || !mc.args.is_empty() {
            return None;
        }
        if !self.is_iterator_like_receiver_expr(&mc.receiver)
            && !self.is_probably_iterator_receiver_expr(&mc.receiver)
        {
            return None;
        }
        let receiver = self.emit_expr_to_string(&mc.receiver);
        Some(format!("rusty::enumerate({})", receiver))
    }

    pub(super) fn try_emit_iter_rev_call(&self, mc: &syn::ExprMethodCall) -> Option<String> {
        if mc.method != "rev" || !mc.args.is_empty() {
            return None;
        }
        if !self.is_iterator_like_receiver_expr(&mc.receiver)
            && !self.is_probably_iterator_receiver_expr(&mc.receiver)
        {
            return None;
        }
        let receiver = self.emit_expr_to_string(&mc.receiver);
        Some(format!("rusty::rev({})", receiver))
    }

    /// `it.copied()` / `it.cloned()` route through the rusty free-function
    /// adapters. Runtime iterator types have member adapters (the free fn
    /// prefers the member spelling, so their exact adapter types are kept),
    /// but TRANSPILED iterator types — btree_port's set::Iter, map::Keys,
    /// set::Union/... — only expose option-like `next()`: their Rust
    /// `copied`/`cloned`/`cycle` are Iterator DEFAULT methods, not inherent
    /// ones, so a member call has nothing to resolve against cross-module.
    /// Unknown-typed receivers (closure params, generic-param locals) route
    /// too — the rusty:: free fns member-prefer, so a receiver that does own
    /// the member still dispatches to it.
    pub(super) fn try_emit_iter_copied_cloned_call(
        &self,
        mc: &syn::ExprMethodCall,
    ) -> Option<String> {
        if !matches!(mc.method.to_string().as_str(), "copied" | "cloned" | "cycle")
            || !mc.args.is_empty()
        {
            return None;
        }
        if self.receiver_is_option_or_result_like_expr(&mc.receiver) {
            return None;
        }
        if !self.is_iterator_like_receiver_expr(&mc.receiver)
            && !self.is_probably_iterator_receiver_expr(&mc.receiver)
            && !self.receiver_type_unresolved_for_iter_default_routing(&mc.receiver)
        {
            return None;
        }
        let receiver = self.emit_expr_to_string(&mc.receiver);
        Some(format!("rusty::{}({})", mc.method, receiver))
    }

    /// First `return rusty::Option<…>(` constructor TYPE text in an
    /// assembled match-arms block (balanced angle brackets), or None when
    /// absent or placeholder-tainted. Used to annotate deduced-return
    /// match lambdas whose other arms return the bare `rusty::None` tag.
    pub(super) fn extract_first_option_ctor_type(arms_text: &str) -> Option<String> {
        let start = arms_text.find("return rusty::Option<")? + "return ".len();
        let after_lt = start + "rusty::Option<".len();
        let bytes = arms_text.as_bytes();
        let mut depth = 1usize;
        let mut i = after_lt;
        while i < bytes.len() && depth > 0 {
            match bytes[i] {
                b'<' => depth += 1,
                b'>' => depth -= 1,
                _ => {}
            }
            i += 1;
        }
        if depth != 0 {
            return None;
        }
        let ty = &arms_text[start..i];
        if ty.contains("auto") || ty.contains("/* TODO") {
            return None;
        }
        Some(ty.to_string())
    }

    /// Rust slice binary-search family routes through rusty free functions
    /// with member-preference dispatch in the emitted prelude: std::span
    /// receivers (the `&[Bucket]` internals) have no such members, while
    /// transpiled Slice types keep their own delegating methods — the free
    /// fn picks whichever exists, so routing is unconditional.
    pub(super) fn try_emit_slice_binary_search_call(
        &self,
        mc: &syn::ExprMethodCall,
    ) -> Option<String> {
        let arity = match mc.method.to_string().as_str() {
            "binary_search" | "binary_search_by" | "partition_point" => 1,
            "binary_search_by_key" => 2,
            "is_sorted" => 0,
            "is_sorted_by" | "is_sorted_by_key" => 1,
            // sort* exist INHERENTLY on set/map types (IndexSet::sort_by
            // sorts values, not buckets) — only slice-shaped or
            // unknown-typed receivers route to the rusty:: slice helpers
            // (which member-prefer, so a mistyped receiver that owns the
            // member still dispatches to it).
            "sort_unstable" | "sort_unstable_by" | "sort_unstable_by_key" | "sort" | "sort_by"
            | "sort_by_key"
                if self.should_lower_slice_deref_method_call(&mc.receiver)
                    || self.receiver_type_unresolved_for_iter_default_routing(&mc.receiver) =>
            {
                if matches!(mc.method.to_string().as_str(), "sort_unstable" | "sort") {
                    0
                } else {
                    1
                }
            }
            _ => return None,
        };
        if mc.args.len() != arity {
            return None;
        }
        let receiver = self.emit_expr_to_string(&mc.receiver);
        let args: Vec<String> = mc
            .args
            .iter()
            .map(|a| self.emit_expr_maybe_move(a))
            .collect();
        if args.is_empty() {
            return Some(format!("rusty::{}({})", mc.method, receiver));
        }
        Some(format!(
            "rusty::{}({}, {})",
            mc.method,
            receiver,
            args.join(", ")
        ))
    }

    pub(super) fn try_emit_iter_fold_call(
        &self,
        mc: &syn::ExprMethodCall,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        if mc.method != "fold" || mc.args.len() != 2 {
            return None;
        }
        let receiver = self.emit_expr_to_string(&mc.receiver);
        // An empty `Vec::new()` accumulator that the reducer fills with the
        // receiver's items: name the element via `decltype` of the receiver
        // (its item type) instead of leaking `Vec<auto>`.
        let init = if self.fold_like_init_recoverable(mc) {
            self.build_fold_empty_vec_init_decltype(&receiver)
        } else {
            let init_expected =
                self.infer_fold_like_init_expected_type_from_method_call(mc, expected_ty, false);
            self.emit_expr_to_string_with_expected_and_move_if_needed(
                mc.args.first()?,
                init_expected.as_ref(),
            )
        };
        let reducer = self.emit_expr_maybe_move(mc.args.iter().nth(1)?);
        Some(format!("rusty::fold({}, {}, {})", receiver, init, reducer))
    }

    /// `rfold` with an empty `Vec::new()` accumulator the reducer fills with the
    /// receiver's items: emit `receiver.rfold(Vec<decltype-elem>::new_(), reducer)`
    /// so the accumulator element is named via `decltype` of the receiver rather
    /// than leaking `Vec<auto>`. Only intercepts that exact recoverable shape;
    /// otherwise returns None so the generic method emission handles `rfold`
    /// normally (and emits the receiver exactly once).
    pub(super) fn try_emit_iter_rfold_call(&self, mc: &syn::ExprMethodCall) -> Option<String> {
        if mc.method != "rfold" || mc.args.len() != 2 {
            return None;
        }
        if !self.fold_like_init_recoverable(mc) {
            return None;
        }
        let receiver = self.emit_expr_to_string(&mc.receiver);
        let init = self.build_fold_empty_vec_init_decltype(&receiver);
        let reducer = self.emit_expr_maybe_move(mc.args.iter().nth(1)?);
        Some(format!("{}.rfold({}, {})", receiver, init, reducer))
    }

    pub(super) fn try_emit_iter_try_fold_call(
        &self,
        mc: &syn::ExprMethodCall,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        if mc.method != "try_fold" || mc.args.len() != 2 {
            return None;
        }
        let receiver = self.emit_expr_to_string(&mc.receiver);
        let init_expected =
            self.infer_fold_like_init_expected_type_from_method_call(mc, expected_ty, true);
        let init = self.emit_expr_to_string_with_expected_and_move_if_needed(
            mc.args.first()?,
            init_expected.as_ref(),
        );
        let reducer_expr = mc.args.iter().nth(1)?;
        let reducer = if let syn::Expr::Closure(closure) = self.peel_paren_group_expr(reducer_expr)
        {
            let expected_rt = expected_ty
                .cloned()
                .map(|ty| syn::ReturnType::Type(Default::default(), Box::new(ty)));
            self.emit_closure_to_string_with_param_scopes(closure, None, None, expected_rt.as_ref())
        } else {
            self.emit_expr_maybe_move(reducer_expr)
        };
        Some(format!(
            "rusty::try_fold({}, {}, {})",
            receiver, init, reducer
        ))
    }

    pub(super) fn try_emit_iter_all_call(&self, mc: &syn::ExprMethodCall) -> Option<String> {
        if mc.method != "all" || mc.args.len() != 1 {
            return None;
        }
        if !self.is_iterator_like_receiver_expr(&mc.receiver) {
            return None;
        }
        let receiver = self.emit_expr_to_string(&mc.receiver);
        let predicate = self.emit_expr_maybe_move(mc.args.first()?);
        Some(format!("rusty::all({}, {})", receiver, predicate))
    }

    pub(super) fn try_emit_iter_count_call(&self, mc: &syn::ExprMethodCall) -> Option<String> {
        if mc.method != "count" || !mc.args.is_empty() {
            return None;
        }
        if self.receiver_is_option_or_result_like_expr(&mc.receiver) {
            return None;
        }
        if !self.is_iterator_like_receiver_expr(&mc.receiver)
            && !self.is_probably_iterator_receiver_expr(&mc.receiver)
        {
            return None;
        }
        let receiver = self.emit_expr_to_string(&mc.receiver);
        Some(format!("rusty::count({})", receiver))
    }

    pub(super) fn try_emit_iter_sum_call(&self, mc: &syn::ExprMethodCall) -> Option<String> {
        if mc.method != "sum" || !mc.args.is_empty() {
            return None;
        }
        if self.receiver_is_option_or_result_like_expr(&mc.receiver) {
            return None;
        }
        if !self.is_iterator_like_receiver_expr(&mc.receiver)
            && !self.is_probably_iterator_receiver_expr(&mc.receiver)
        {
            return None;
        }
        let receiver = self.emit_expr_to_string(&mc.receiver);
        Some(format!("rusty::sum({})", receiver))
    }

    pub(super) fn try_emit_iter_step_by_call(&self, mc: &syn::ExprMethodCall) -> Option<String> {
        if mc.method != "step_by" || mc.args.len() != 1 {
            return None;
        }
        if self.receiver_is_option_or_result_like_expr(&mc.receiver) {
            return None;
        }
        if !self.is_iterator_like_receiver_expr(&mc.receiver)
            && !self.is_probably_iterator_receiver_expr(&mc.receiver)
        {
            return None;
        }
        let receiver = self.emit_expr_to_string(&mc.receiver);
        let step = self.emit_expr_to_string(mc.args.first()?);
        Some(format!("rusty::step_by({}, {})", receiver, step))
    }

    pub(super) fn try_emit_iter_flat_map_call(&self, mc: &syn::ExprMethodCall) -> Option<String> {
        if mc.method != "flat_map" || mc.args.len() != 1 {
            return None;
        }
        if self.receiver_is_option_or_result_like_expr(&mc.receiver) {
            return None;
        }
        if !self.is_iterator_like_receiver_expr(&mc.receiver)
            && !self.is_probably_iterator_receiver_expr(&mc.receiver)
        {
            return None;
        }
        let receiver = self.emit_expr_to_string(&mc.receiver);
        let func = self.emit_expr_maybe_move(mc.args.first()?);
        Some(format!("rusty::flat_map({}, {})", receiver, func))
    }

    pub(super) fn try_emit_iter_for_each_call(&self, mc: &syn::ExprMethodCall) -> Option<String> {
        if mc.method != "for_each" || mc.args.len() != 1 {
            return None;
        }
        if self.receiver_has_inherent_method_named(&mc.receiver, "for_each") {
            return None;
        }
        if self.receiver_is_option_or_result_like_expr(&mc.receiver) {
            return None;
        }
        let receiver_is_self_path = matches!(
            self.peel_paren_group_expr(&mc.receiver),
            syn::Expr::Path(path)
                if path.path.segments.len() == 1 && path.path.segments[0].ident == "self"
        );
        let receiver_is_self_iterator = receiver_is_self_path
            && self
                .lookup_current_struct_method_return_type("next")
                .is_some();
        if !receiver_is_self_iterator
            && !self.is_iterator_like_receiver_expr(&mc.receiver)
            && !self.is_probably_iterator_receiver_expr(&mc.receiver)
            && !self.receiver_type_unresolved_for_iter_default_routing(&mc.receiver)
        {
            return None;
        }
        let receiver = self.emit_expr_to_string(&mc.receiver);
        let func = self.emit_call_arg_with_pass_style(mc.args.first()?, None, None, false, None);
        Some(format!("rusty::for_each({}, {})", receiver, func))
    }

    /// `.all(pred)` / `.any(pred)` — Iterator DEFAULT methods; same gating
    /// as for_each. The rusty:: free fns loop for_in over the receiver.
    pub(super) fn try_emit_iter_all_any_call(&self, mc: &syn::ExprMethodCall) -> Option<String> {
        let method = mc.method.to_string();
        if !matches!(method.as_str(), "all" | "any") || mc.args.len() != 1 {
            return None;
        }
        if self.receiver_has_inherent_method_named(&mc.receiver, &method) {
            return None;
        }
        if self.receiver_is_option_or_result_like_expr(&mc.receiver) {
            return None;
        }
        if !self.is_iterator_like_receiver_expr(&mc.receiver)
            && !self.is_probably_iterator_receiver_expr(&mc.receiver)
            && !self.receiver_type_unresolved_for_iter_default_routing(&mc.receiver)
        {
            return None;
        }
        let receiver = self.emit_expr_to_string(&mc.receiver);
        let pred = self.emit_call_arg_with_pass_style(mc.args.first()?, None, None, false, None);
        Some(format!("rusty::{}({}, {})", method, receiver, pred))
    }

    pub(super) fn try_emit_iter_try_for_each_call(
        &self,
        mc: &syn::ExprMethodCall,
    ) -> Option<String> {
        if mc.method != "try_for_each" || mc.args.len() != 1 {
            return None;
        }
        // `try_for_each` is an Iterator-trait method with no common non-iterator
        // collision, and `rusty::try_for_each` accepts any range (via `for_in`),
        // so lower broadly rather than gating on the receiver-iterator heuristic
        // (unlike `for_each`). That heuristic can't confirm a *generic* iterator
        // local — `let mut iter = x.into_iter(); iter.try_for_each(...)` in
        // serde/itertools, whose binding type is `<I as IntoIterator>::IntoIter`
        // — so a strict gate leaves the call as a member on a type with no such
        // member. The guards below still bail for a user type that owns
        // `try_for_each` and for Option/Result-like receivers.
        if self.receiver_has_inherent_method_named(&mc.receiver, "try_for_each") {
            return None;
        }
        if self.receiver_is_option_or_result_like_expr(&mc.receiver) {
            return None;
        }
        let receiver = self.emit_expr_to_string(&mc.receiver);
        let func = self.emit_call_arg_with_pass_style(mc.args.first()?, None, None, false, None);
        Some(format!("rusty::try_for_each({}, {})", receiver, func))
    }

    pub(super) fn try_emit_ordering_then_with_call(&self, mc: &syn::ExprMethodCall) -> Option<String> {
        if mc.method != "then_with" || mc.args.len() != 1 {
            return None;
        }
        let receiver_is_ordering = self
            .infer_simple_expr_type(&mc.receiver)
            .as_ref()
            .is_some_and(|ty| self.is_ordering_like_type(ty))
            || self.is_ordering_then_with_receiver_shape(&mc.receiver);
        if !receiver_is_ordering {
            return None;
        }
        let receiver = self.emit_expr_to_string(&mc.receiver);
        let callback = self.emit_expr_maybe_move(mc.args.first()?);
        Some(format!("rusty::cmp::then_with({}, {})", receiver, callback))
    }

    pub(super) fn try_emit_error_description_dispatch_call(&self, mc: &syn::ExprMethodCall) -> Option<String> {
        if mc.method != "description" || !mc.args.is_empty() {
            return None;
        }
        let receiver_name = match self.peel_paren_group_expr(&mc.receiver) {
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                Some(path.path.segments[0].ident.to_string())
            }
            _ => None,
        };
        // Leaf 4.40: keep rewrite narrow to expanded match-bound payload shape
        // (`ref inner => inner.description()`) to avoid broad method rewrites.
        if receiver_name.as_deref() != Some("inner") {
            return None;
        }
        let receiver = self.emit_expr_to_string(&mc.receiver);
        Some(format!("rusty::error::description({})", receiver))
    }

    pub(super) fn try_emit_slice_full_call_with_expected_array_type(
        &self,
        call: &syn::ExprCall,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        if call.args.len() != 1 {
            return None;
        }
        let expected_ty = expected_ty?;
        if self
            .expected_array_element_type(Some(expected_ty))
            .is_none()
        {
            return None;
        }
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        let path = path_expr
            .path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if !matches!(path.as_str(), "slice_full" | "rusty::slice_full") {
            return None;
        }
        let inner = self.emit_expr_to_string_with_expected(&call.args[0], Some(expected_ty));
        Some(format!("rusty::slice_full({})", inner))
    }

    pub(super) fn try_emit_slice_full_buffer_arg_expr(
        &self,
        expr: &syn::Expr,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let syn::Expr::Call(call) = self.peel_paren_group_expr(expr) else {
            return None;
        };
        if call.args.len() != 1 {
            return None;
        }
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        let path = path_expr
            .path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if !matches!(path.as_str(), "slice_full" | "rusty::slice_full") {
            return None;
        }
        let inner = self.emit_expr_to_string_with_expected(&call.args[0], expected_ty);
        Some(format!("rusty::slice_full({})", inner))
    }

    pub(super) fn try_emit_slice_index_expr_to_string(
        &self,
        idx: &syn::ExprIndex,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        if let syn::Expr::Range(range) = self.peel_paren_group_expr(&idx.index) {
            let base = self.emit_expr_to_string_with_expected(&idx.expr, expected_ty);
            let start = range.start.as_ref().map(|e| self.emit_expr_to_string(e));
            let end = range.end.as_ref().map(|e| self.emit_expr_to_string(e));
            let inclusive = matches!(range.limits, syn::RangeLimits::Closed(_));
            let emitted = match (start, end, inclusive) {
                (Some(s), Some(e), false) => format!("rusty::slice({}, {}, {})", base, s, e),
                (Some(s), Some(e), true) => {
                    format!("rusty::slice_inclusive({}, {}, {})", base, s, e)
                }
                (Some(s), None, _) => format!("rusty::slice_from({}, {})", base, s),
                (None, Some(e), false) => format!("rusty::slice_to({}, {})", base, e),
                (None, Some(e), true) => format!("rusty::slice_to_inclusive({}, {})", base, e),
                (None, None, _) => {
                    let base_is_string_like = self
                        .infer_simple_expr_type(&idx.expr)
                        .as_ref()
                        .is_some_and(|ty| {
                            self.is_known_string_like_type(ty)
                                || self.map_type(ty) == "std::string_view"
                        });
                    if self.expected_type_is_string_view(expected_ty) || base_is_string_like {
                        self.emit_from_conversion_to_target(&idx.expr, "std::string_view")
                    } else {
                        format!("rusty::slice_full({})", base)
                    }
                }
            };
            return Some(emitted);
        }

        // Dynamic range-shaped indices (for example `input[err.span().unwrap()]`)
        // cannot lower to raw `operator[]` on C-string bases. Route all known
        // runtime range types through a helper that handles both string and
        // slice-like containers.
        let base_is_string_like =
            self.infer_simple_expr_type(&idx.expr)
                .as_ref()
                .is_some_and(|ty| {
                    self.is_known_string_like_type(ty) || self.map_type(ty) == "std::string_view"
                });
        let is_runtime_range_index = self
            .infer_simple_expr_type(&idx.index)
            .as_ref()
            .is_some_and(|index_ty| {
                let index_ty = self.peel_reference_paren_group_type(index_ty);
                let mapped = self.map_type(index_ty);
                let canonical = mapped
                    .chars()
                    .filter(|c| !c.is_ascii_whitespace())
                    .collect::<String>();
                canonical.starts_with("rusty::range<")
                    || canonical.starts_with("rusty::range_from<")
                    || canonical.starts_with("rusty::range_inclusive<")
                    || canonical.starts_with("rusty::range_to<")
                    || canonical.starts_with("rusty::range_to_inclusive<")
                    || canonical == "rusty::range_full"
            });
        let is_span_unwrap_range_hint = matches!(
            self.peel_paren_group_expr(&idx.index),
            syn::Expr::MethodCall(outer)
                if outer.method == "unwrap"
                    && outer.args.is_empty()
                    && matches!(
                        self.peel_paren_group_expr(&outer.receiver),
                        syn::Expr::MethodCall(inner)
                            if inner.method == "span" && inner.args.is_empty()
                    )
        );
        if !is_runtime_range_index && !is_span_unwrap_range_hint {
            return None;
        }

        let base = if self.expected_type_is_string_view(expected_ty) || base_is_string_like {
            self.emit_from_conversion_to_target(&idx.expr, "std::string_view")
        } else {
            self.emit_expr_to_string_with_expected(&idx.expr, expected_ty)
        };
        let index = self.emit_expr_to_string(&idx.index);
        Some(format!("rusty::index_with_range({}, {})", base, index))
    }

    pub(super) fn try_emit_empty_array_index_expr_to_string(
        &self,
        idx: &syn::ExprIndex,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let base = self.peel_paren_group_expr(&idx.expr);
        let syn::Expr::Array(array) = base else {
            return None;
        };
        if !array.elems.is_empty() {
            return None;
        }
        Some(self.index_out_of_bounds_fallback_with_expected(expected_ty))
    }

    pub(super) fn emit_match_expr_to_string(
        &self,
        match_expr: &syn::ExprMatch,
        expected_ty: Option<&syn::Type>,
    ) -> String {
        if match_expr.arms.is_empty() {
            return self.match_expr_unreachable_fallback_with_expected(expected_ty);
        }
        let inferred_expected_ty = if expected_ty.is_none() {
            if let Some(return_hint) = self.current_return_type_hint()
                && self
                    .expected_result_type_arg(Some(return_hint), 0)
                    .is_some()
                && self.match_expr_likely_yields_current_result_type(match_expr, return_hint)
            {
                Some(return_hint.clone())
            } else if match_expr.arms.len() <= 4
                && self.expr_contains_early_return_or_try(&syn::Expr::Match(match_expr.clone()))
            {
                self.infer_match_arms_common_type(&match_expr.arms)
                    .or_else(|| self.infer_match_arms_common_type_with_scrutinee(match_expr))
            } else {
                None
            }
        } else {
            None
        };
        let expected_ty = expected_ty.or(inferred_expected_ty.as_ref());
        let try_style_match_expr_disabled =
            std::env::var_os("RUSTY_CPP_DISABLE_TRY_STYLE_MATCH_EXPR").is_some();
        if !try_style_match_expr_disabled && self.match_expr_has_explicit_return_arm(match_expr) {
            if let Some(lowered) = self.emit_try_style_runtime_match_expr(match_expr, expected_ty) {
                return lowered;
            }
            if let Some(lowered) = self.emit_try_style_either_match_expr(match_expr, expected_ty) {
                return lowered;
            }
            // A fn-level `return` inside a VALUE match whose type differs
            // from the enclosing fn's return cannot live in the IIFE lambda
            // (loader.rs: `return Err(io)` inside a Cow-valued Progress
            // match binds to the Cow lambda). Take the statement-expression
            // lowering with variant arms enabled — `return` stays a real
            // function return there.
            let delegation_expected: Option<syn::Type> = expected_ty.cloned().or_else(|| {
                // The 4-arm inference cap above doesn't apply here: any
                // common non-diverging arm type suffices to detect the
                // fn-return mismatch (loader's 6-arm Progress match).
                self.infer_match_arms_common_type(&match_expr.arms)
                    .or_else(|| self.infer_match_arms_common_type_with_scrutinee(match_expr))
            });
            if let (Some(fn_ret), Some(expected)) = (
                self.current_return_type_hint().cloned(),
                delegation_expected.as_ref(),
            ) {
                let fn_ret_cpp = self.map_type(&fn_ret);
                let expected_cpp = self.map_type(expected);
                if !fn_ret_cpp.is_empty()
                    && !expected_cpp.is_empty()
                    && fn_ret_cpp != "auto"
                    && expected_cpp != "auto"
                    && fn_ret_cpp != expected_cpp
                {
                    let variant_ctx =
                        self.infer_variant_type_context_from_expr(&match_expr.expr);
                    let lowered = self.emit_match_expr_switch_statement_expr_with_arm_mode(
                        match_expr,
                        Some(expected),
                        variant_ctx.as_ref(),
                        true,
                    );
                    if let Some(lowered) = lowered {
                        return lowered;
                    }
                }
            }
        }
        let variant_ctx = self.infer_variant_type_context_from_expr(&match_expr.expr);
        if let Some(lowered) = self.emit_match_expr_switch_statement_expr(
            match_expr,
            expected_ty,
            variant_ctx.as_ref(),
        ) {
            return lowered;
        }
        if self.switch_match_can_use_value_lowering(match_expr, variant_ctx.as_ref()) {
            let scrutinee =
                self.emit_expr_to_string_with_variant_ctx(&match_expr.expr, variant_ctx.as_ref());
            // Item 4 (GENERIC_FIXES_PLAN): if we know the expected
            // type (tail-position match's expected_ty was threaded in,
            // or one of the type-inference paths above filled it in),
            // pin the IIFE's return type to that expected type instead
            // of leaving `auto` to deduce from the first arm. This is
            // necessary when arms return different variant-constructor
            // types (e.g. `LeftOrRight_Left<size_t>` vs
            // `LeftOrRight_Right<size_t>`) that don't unify via `auto`
            // but DO convert into the common enum type
            // (`LeftOrRight<size_t>`) via the variant's converting
            // constructor.
            let return_clause = expected_ty
                .map(|ty| format!(" -> {}", self.map_type(ty)))
                .unwrap_or_default();
            let scrutinee_is_consumed_place = !self.expr_is_reference_yielding(&match_expr.expr)
                && matches!(
                    self.peel_paren_group_expr(&match_expr.expr),
                    syn::Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1
                );
            let arms_text = self.emit_match_expr_switch_with_consumed_scrutinee(
                &match_expr.arms,
                expected_ty,
                variant_ctx.as_ref(),
                scrutinee_is_consumed_place,
            );
            // A DEDUCED-return lambda whose arms mix `rusty::Option<T>(…)`
            // and the bare `rusty::None` tag is ill-formed (conflicting
            // deductions) — annotate from the sibling arm's Option ctor
            // (indexmap's get_disjoint_opt_mut inner mapper).
            let mut return_clause = return_clause;
            if return_clause.is_empty()
                && arms_text.contains("return rusty::None;")
                && let Some(opt_ty) = Self::extract_first_option_ctor_type(&arms_text)
            {
                return_clause = format!(" -> {}", opt_ty);
            }
            return format!(
                "[&](){} {{ auto&& _m = {}; {} }}()",
                return_clause, scrutinee, arms_text
            );
        }
        // Item 11: when type inference can't see the scrutinee's tuple shape
        // (often for method-call returns whose owner-impl is resolved
        // through deep generics), infer the arity from the arm patterns
        // themselves. If every arm is a `Pat::Tuple` of the same arity (or
        // a `_` wildcard fallback), the scrutinee must be a tuple of that
        // arity. This unblocks `match self.foo() { (None, h) => …,
        // (Some(x), h) => … }`-style matches that previously degraded to
        // `std::visit(overloaded { [&](auto&&) { unreachable(); }, … })`.
        let tuple_arity = self
            .match_expr_tuple_scrutinee_arity(&match_expr.expr)
            .or_else(|| Self::tuple_arity_from_arm_patterns(&match_expr.arms));
        if let Some(tuple_arity) = tuple_arity {
            if self.tuple_match_can_lower_as_value_conditions(&match_expr.arms, tuple_arity) {
                if let syn::Expr::Tuple(tuple_scrutinee) =
                    self.peel_paren_group_expr(&match_expr.expr)
                {
                    return self.emit_match_expr_tuple_value_conditions(
                        tuple_scrutinee,
                        &match_expr.arms,
                        expected_ty,
                    );
                }
                return self.emit_match_expr_tuple_value_conditions_for_scrutinee_expr(
                    &match_expr.expr,
                    tuple_arity,
                    &match_expr.arms,
                    expected_ty,
                );
            }
        }
        if let Some(runtime_expr) =
            self.emit_runtime_match_expr(match_expr, variant_ctx.as_ref(), expected_ty)
        {
            return runtime_expr;
        }
        // Match as expression → immediately-invoked lambda
        if let syn::Expr::Tuple(tuple_scrutinee) = self.peel_paren_group_expr(&match_expr.expr) {
            let tuple_variant_ctx = tuple_scrutinee
                .elems
                .iter()
                .find_map(|e| self.infer_variant_type_context_from_expr(e));
            // Tuple scrutinee variant patterns: std::visit over each tuple element.
            let visit_args: Vec<String> = tuple_scrutinee
                .elems
                .iter()
                .map(|e| self.emit_expr_to_string_with_variant_ctx(e, tuple_variant_ctx.as_ref()))
                .collect();
            format!(
                "[&]() {{ return std::visit(overloaded {{ {} }}, {}); }}()",
                self.emit_match_expr_visit_tuple(
                    &match_expr.arms,
                    tuple_scrutinee.elems.len(),
                    tuple_variant_ctx.as_ref(),
                    expected_ty,
                ),
                visit_args.join(", ")
            )
        } else {
            let scrutinee =
                self.emit_expr_to_string_with_variant_ctx(&match_expr.expr, variant_ctx.as_ref());
            let visit_borrows_payload =
                self.runtime_match_scrutinee_borrows_payload(&match_expr.expr);
            let visit_payload = if visit_borrows_payload {
                "rusty::detail::deref_if_pointer(_m)"
            } else {
                "std::move(rusty::detail::deref_if_pointer(_m))"
            };
            // Variant match → IIFE with std::visit
            if self.should_force_size_t_visit_return_for_bound_match(match_expr, expected_ty) {
                format!(
                    "[&]() {{ auto&& _m = {}; return std::visit<size_t>(overloaded {{ {} }}, {}); }}()",
                    scrutinee,
                    self.emit_match_expr_visit(
                        &match_expr.expr,
                        &match_expr.arms,
                        variant_ctx.as_ref(),
                        expected_ty,
                        visit_borrows_payload,
                    ),
                    visit_payload
                )
            } else {
                format!(
                    "[&]() {{ auto&& _m = {}; return std::visit(overloaded {{ {} }}, {}); }}()",
                    scrutinee,
                    self.emit_match_expr_visit(
                        &match_expr.expr,
                        &match_expr.arms,
                        variant_ctx.as_ref(),
                        expected_ty,
                        visit_borrows_payload,
                    ),
                    visit_payload
                )
            }
        }
    }

    pub(super) fn emit_return_value_with_try_style_binding_scope(
        &self,
        value_expr: &syn::Expr,
        binding_map: &HashMap<String, String>,
    ) -> String {
        let inner = self.new_inner_with_try_style_binding_scope(binding_map);
        let mut value =
            inner.emit_expr_to_string_with_expected(value_expr, inner.current_return_type_hint());
        if inner.return_expr_should_move_local(value_expr) && !value.starts_with("std::move(") {
            value = format!("std::move({})", value);
        }
        value
    }

    pub(super) fn emit_expr_with_try_style_binding_scope_with_ref_mode(
        &self,
        expr: &syn::Expr,
        expected_ty: Option<&syn::Type>,
        binding_map: &HashMap<String, String>,
        treat_bindings_as_refs: bool,
    ) -> String {
        let inner = self.new_inner_with_try_style_binding_scope_with_ref_mode(
            binding_map,
            treat_bindings_as_refs,
        );
        inner.emit_expr_to_string_with_expected(expr, expected_ty)
    }

    pub(super) fn emit_expr_with_try_style_binding_scope(
        &self,
        expr: &syn::Expr,
        expected_ty: Option<&syn::Type>,
        binding_map: &HashMap<String, String>,
    ) -> String {
        self.emit_expr_with_try_style_binding_scope_with_ref_mode(
            expr,
            expected_ty,
            binding_map,
            true,
        )
    }

    pub(super) fn emit_return_expr_with_variant_ctx_and_try_style_binding_scope(
        &self,
        ret: &syn::ExprReturn,
        variant_ctx: &VariantTypeContext,
        binding_map: &HashMap<String, String>,
    ) -> String {
        let inner = self.new_inner_with_try_style_binding_scope(binding_map);
        inner.emit_return_expr_with_variant_ctx(ret, variant_ctx)
    }

    pub(super) fn emit_return_expr_with_variant_ctx(
        &self,
        ret: &syn::ExprReturn,
        variant_ctx: &VariantTypeContext,
    ) -> String {
        let Some(expr) = &ret.expr else {
            return "return".to_string();
        };
        if let syn::Expr::Call(call) = expr.as_ref() {
            if let syn::Expr::Path(path_expr) = call.func.as_ref() {
                if let Some(ctor_name) = self.variant_ctor_name_from_path(&path_expr.path) {
                    if call.args.len() == 1 && variant_ctx.template_args.len() >= 2 {
                        let return_ctor_args = self
                            .current_return_type_hint()
                            .and_then(|ty| self.expected_type_template_args(ty))
                            .filter(|args| args.len() >= 2)
                            .unwrap_or_else(|| variant_ctx.template_args.clone());
                        let target_cpp_ty = if ctor_name == "Left" {
                            return_ctor_args[0].as_str()
                        } else {
                            return_ctor_args[1].as_str()
                        };
                        let arg = self.emit_from_conversion_to_target(&call.args[0], target_cpp_ty);
                        let mut ctor_cpp = ctor_name.clone();
                        if matches!(ctor_name.as_str(), "Left" | "Right") {
                            let return_is_runtime_either = self
                                .current_return_type_hint()
                                .is_some_and(|ty| self.map_type(ty).starts_with("rusty::Either<"));
                            if return_is_runtime_either {
                                ctor_cpp = format!("rusty::either::{}", ctor_name);
                            }
                        }
                        return format!(
                            "return {}<{}, {}>({})",
                            ctor_cpp, return_ctor_args[0], return_ctor_args[1], arg
                        );
                    }
                }
            }
        }
        format!(
            "return {}",
            self.emit_expr_to_string_with_expected(expr, self.current_return_type_hint())
        )
    }

    /// `u64::from_str_radix` in VALUE position (a fn passed as an argument:
    /// `parse_unsigned_int(v, u64::from_str_radix)`) — the call form lowers
    /// to `rusty::from_str_radix<T>(...)`, so the value form wraps the same
    /// helper in a lambda.
    pub(super) fn try_emit_primitive_assoc_fn_value(segments: &[String]) -> Option<String> {
        if segments.len() != 2 || segments[1] != "from_str_radix" {
            return None;
        }
        let target_cpp = rust_primitive_cast_target_cpp_type(&segments[0])?;
        Some(format!(
            "[](std::string_view __s, uint32_t __r) {{ return rusty::from_str_radix<{}>(__s, __r); }}",
            target_cpp
        ))
    }

    pub(super) fn try_emit_numeric_limits_path(
        &self,
        path: &syn::Path,
        segments: &[String],
    ) -> Option<String> {
        if segments.len() < 2 {
            return None;
        }
        // For integer primitives Rust's `MIN` matches C++'s `min()`
        // (signed `MIN` is the most negative value, which equals
        // `numeric_limits::lowest()` for integers). Floats are
        // deferred to `try_emit_primitive_float_assoc_const_segments`
        // because `f32::MIN` is the most negative value but
        // `numeric_limits<float>::min()` is the smallest positive
        // normal — the dedicated float handler emits `lowest()`.
        let method = match segments.last().map(String::as_str) {
            Some("MAX") => "max",
            Some("MIN") => "min",
            _ => return None,
        };
        // Primitive paths: `usize::MAX`, `std::u32::MIN`, `core::isize::MAX`, and
        // `std::primitive::i32::MAX` / `core::primitive::…` (c2rust ports import
        // primitives via `core::primitive`). The primitive is the segment right
        // before MAX/MIN; accept it under any of these known wrapper prefixes.
        let primitive_candidate = if segments.len() == 2 {
            Some(segments[0].as_str())
        } else {
            let prefix: Vec<&str> =
                segments[..segments.len() - 2].iter().map(String::as_str).collect();
            matches!(
                prefix.as_slice(),
                ["std"] | ["core"] | ["std", "primitive"] | ["core", "primitive"] | ["libc"]
            )
            .then(|| segments[segments.len() - 2].as_str())
        };
        // Normalize libc C type aliases to the matching Rust primitive
        // (`c_int::MAX`, common in c2rust ports, imported as a bare 2-segment
        // path) so `map_primitive_type` resolves them. `c_long`/`c_ulong` follow
        // the type mapping (`isize`->`ptrdiff_t`, `usize`->`size_t`).
        let primitive_candidate = primitive_candidate.map(|c| match c {
            "c_char" | "c_schar" => "i8",
            "c_uchar" => "u8",
            "c_short" => "i16",
            "c_ushort" => "u16",
            "c_int" => "i32",
            "c_uint" => "u32",
            "c_long" => "isize",
            "c_ulong" => "usize",
            "c_longlong" => "i64",
            "c_ulonglong" => "u64",
            other => other,
        });
        if let Some(candidate) = primitive_candidate {
            if let Some(cpp_prim) = types::map_primitive_type(candidate) {
                if candidate == "char" {
                    // Rust `char` is a Unicode scalar value (U+0000..=U+10FFFF),
                    // not the full storage range of `char32_t`.
                    return Some(
                        match method {
                            "max" => "static_cast<char32_t>(0x10FFFF)",
                            "min" => "static_cast<char32_t>(0)",
                            _ => unreachable!(),
                        }
                        .to_string(),
                    );
                }
                // Defer float `MIN` to the dedicated float handler so it
                // emits `lowest()`, not `min()`.
                if matches!(candidate, "f32" | "f64") && method == "min" {
                    return None;
                }
                return Some(format!("std::numeric_limits<{}>::{}()", cpp_prim, method));
            }
        }

        // Numeric alias paths like `LenUint::MAX`.
        let base_segments: Vec<String> = segments[..segments.len() - 1].to_vec();
        let mut candidates: Vec<String> = Vec::new();
        candidates.push(base_segments.join("::"));
        if let Some(last) = base_segments.last() {
            candidates.push(last.clone());
        }

        if base_segments.len() == 1 {
            candidates.push(self.scoped_type_key(&base_segments[0]));
        } else {
            match base_segments[0].as_str() {
                "crate" if base_segments.len() > 1 => {
                    candidates.push(base_segments[1..].join("::"));
                }
                "self" if base_segments.len() > 1 => {
                    let mut resolved = self.module_stack.clone();
                    resolved.extend(base_segments[1..].iter().cloned());
                    if !resolved.is_empty() {
                        candidates.push(resolved.join("::"));
                    }
                }
                "super" if base_segments.len() > 1 => {
                    let mut resolved = if self.module_stack.len() > 1 {
                        self.module_stack[..self.module_stack.len() - 1].to_vec()
                    } else {
                        Vec::new()
                    };
                    resolved.extend(base_segments[1..].iter().cloned());
                    if !resolved.is_empty() {
                        candidates.push(resolved.join("::"));
                    }
                }
                _ => {}
            }
        }

        let candidate_is_numeric_alias = candidates.iter().any(|candidate| {
            self.numeric_type_aliases.contains_key(candidate)
                || self
                    .type_alias_targets
                    .get(candidate)
                    .is_some_and(|alias_target| {
                        let mapped = self.map_type(alias_target);
                        is_numeric_cpp_scalar_type(&mapped)
                    })
        });
        if candidate_is_numeric_alias {
            let base_path = Self::path_without_last_segment(path)?;
            let base_cpp = self.emit_path_to_string(&base_path);
            return Some(format!("std::numeric_limits<{}>::{}()", base_cpp, method));
        }

        None
    }

    pub(super) fn try_emit_primitive_float_assoc_const_path(&self, path: &syn::Path) -> Option<String> {
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        Self::try_emit_primitive_float_assoc_const_segments(&segments)
    }

    pub(super) fn try_emit_primitive_float_assoc_const_segments(segments: &[String]) -> Option<String> {
        let (owner, member) = match segments {
            [owner, member] => (owner.as_str(), member.as_str()),
            [root, owner, member] if matches!(root.as_str(), "std" | "core") => {
                (owner.as_str(), member.as_str())
            }
            [root, primitive, owner, member]
                if matches!(root.as_str(), "std" | "core") && primitive == "primitive" =>
            {
                (owner.as_str(), member.as_str())
            }
            _ => return None,
        };
        // `char` assoc consts ride the same primitive-owner path shapes
        // (`char::REPLACEMENT_CHARACTER`, `core::char::MAX`, …); the keyword
        // escape alone would spell a nonexistent `char_::…`.
        if owner == "char" || owner == "char_" {
            return match member {
                "REPLACEMENT_CHARACTER" => {
                    Some("rusty::char_runtime::REPLACEMENT_CHARACTER".to_string())
                }
                "MAX" => Some("static_cast<char32_t>(0x10FFFF)".to_string()),
                _ => None,
            };
        }
        let (cpp_ty, num_bits, num_sig_bits, num_exp_bits, exp_mask, exp_bias, exp_offset) =
            match owner {
                "f32" => ("float", 32, 23, 8, 255, 127, 150),
                "f64" => ("double", 64, 52, 11, 2047, 1023, 1075),
                _ => return None,
            };
        let mapped = match member {
            "NUM_BITS" => num_bits.to_string(),
            "NUM_SIG_BITS" => num_sig_bits.to_string(),
            "NUM_EXP_BITS" => num_exp_bits.to_string(),
            "EXP_MASK" => exp_mask.to_string(),
            "EXP_BIAS" => exp_bias.to_string(),
            "EXP_OFFSET" => exp_offset.to_string(),
            "MANTISSA_DIGITS" => {
                if owner == "f32" {
                    "24".to_string()
                } else {
                    "53".to_string()
                }
            }
            "DIGITS" => {
                if owner == "f32" {
                    "6".to_string()
                } else {
                    "15".to_string()
                }
            }
            "MIN_10_EXP" => {
                if owner == "f32" {
                    "(-37)".to_string()
                } else {
                    "(-307)".to_string()
                }
            }
            "MAX_10_EXP" => {
                if owner == "f32" {
                    "38".to_string()
                } else {
                    "308".to_string()
                }
            }
            "MAX_DIGITS10" => {
                if owner == "f32" {
                    "9".to_string()
                } else {
                    "17".to_string()
                }
            }
            "RADIX" => "2".to_string(),
            "EPSILON" => format!("std::numeric_limits<{}>::epsilon()", cpp_ty),
            "MIN_POSITIVE" => format!("std::numeric_limits<{}>::min()", cpp_ty),
            "MIN" => format!("std::numeric_limits<{}>::lowest()", cpp_ty),
            "MAX" => format!("std::numeric_limits<{}>::max()", cpp_ty),
            "NAN" => format!("std::numeric_limits<{}>::quiet_NaN()", cpp_ty),
            "INFINITY" => format!("std::numeric_limits<{}>::infinity()", cpp_ty),
            "NEG_INFINITY" => format!("(-std::numeric_limits<{}>::infinity())", cpp_ty),
            "IMPLICIT_BIT" => {
                if owner == "f32" {
                    "static_cast<uint32_t>(8388608)".to_string()
                } else {
                    "static_cast<uint64_t>(4503599627370496)".to_string()
                }
            }
            _ => return None,
        };
        Some(mapped)
    }

    pub(super) fn try_emit_integer_max_str_len_path(
        &self,
        path: &syn::Path,
        segments: &[String],
    ) -> Option<String> {
        if segments.len() < 2 || segments.last().map_or(true, |seg| seg != "MAX_STR_LEN") {
            return None;
        }
        let owner = segments.iter().nth_back(1)?;
        let owner_is_type_like = owner
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase() || ch == '_')
            || owner == "Self"
            || self.is_type_param_in_scope(owner)
            || types::map_primitive_type(owner).is_some();
        if !owner_is_type_like {
            return None;
        }

        let owner_path = Self::path_without_last_segment(path)?;
        let mut owner_cpp = self.emit_path_to_string(&owner_path);
        if owner_cpp.starts_with("::") {
            owner_cpp = owner_cpp.trim_start_matches("::").to_string();
        }
        if owner_cpp.is_empty()
            || owner_cpp.contains("/* TODO")
            || type_string_has_auto_placeholder(&owner_cpp)
        {
            return None;
        }
        Some(format!("rusty::integer_max_str_len<{}>()", owner_cpp))
    }

    pub(super) fn try_emit_assoc_path_with_expected(
        &self,
        path: &syn::Path,
        expected_ty: &syn::Type,
    ) -> Option<String> {
        if path.segments.len() < 2 {
            return None;
        }
        let owner_idx = path.segments.len().saturating_sub(2);
        let owner_seg = path.segments.iter().nth(owner_idx)?;
        if !matches!(owner_seg.arguments, syn::PathArguments::None) {
            return None;
        }

        let owner = owner_seg.ident.to_string();
        let scoped_owner = if self.module_stack.is_empty() {
            owner.clone()
        } else {
            format!("{}::{}", self.module_stack.join("::"), owner)
        };
        let owner_key = self
            .lookup_declared_type_key_for_base(&scoped_owner, &owner)
            .or_else(|| self.lookup_declared_type_key_for_base(&owner, &owner))?;
        let owner_kinds = self.declared_type_param_kinds.get(&owner_key)?;
        if owner_kinds.is_empty() {
            return None;
        }
        let recovered_args =
            self.owner_template_args_from_expected_array_type(owner_kinds, expected_ty)?;
        if recovered_args.len() != owner_kinds.len() {
            return None;
        }

        let mut emitted_segs: Vec<String> = path
            .segments
            .iter()
            .map(|seg| escape_cpp_keyword(&seg.ident.to_string()))
            .collect();
        emitted_segs[owner_idx] = format!(
            "{}<{}>",
            escape_cpp_keyword(&owner),
            recovered_args.join(", ")
        );
        Some(emitted_segs.join("::"))
    }

    /// Emit an expression, wrapping local variable paths in std::move().
    /// In Rust, passing a non-Copy variable by value moves it. In C++, we need
    /// explicit std::move() to get move semantics. For Copy types, std::move()
    /// is a harmless no-op (the copy still happens).
    ///
    /// We wrap in std::move when:
    /// - The expression is a simple variable path (single ident)
    /// - It's not `self`, `true`, `false`, `Self`, or an ALL_CAPS constant
    /// - It's not a reference expression (&x)
    ///
    /// We do NOT wrap:
    /// - Literals (1, "hello", true)
    /// - Reference expressions (&x, &mut x) — these borrow, not move
    /// - Complex expressions (a + b, foo()) — these produce temporaries
    /// - Type paths / constants (ALL_CAPS names)
    /// Emit a closure expression as a C++ lambda.
    pub(super) fn emit_closure_to_string(&self, closure: &syn::ExprClosure) -> String {
        self.emit_closure_to_string_with_param_scopes(closure, None, None, None)
    }

    pub(super) fn emit_closure_to_string_with_iterator_map_context(
        &self,
        closure: &syn::ExprClosure,
    ) -> String {
        let param_scope = self.collect_closure_param_names_for_scope(closure);
        self.emit_closure_to_string_with_param_scopes(closure, Some(param_scope), None, None)
    }

    pub(super) fn emit_closure_to_string_with_char_predicate_context(
        &self,
        closure: &syn::ExprClosure,
        char_predicate_param_scope: HashSet<String>,
    ) -> String {
        self.emit_closure_to_string_with_param_scopes(
            closure,
            None,
            Some(char_predicate_param_scope),
            None,
        )
    }

    pub(super) fn emit_closure_to_string_with_param_scopes(
        &self,
        closure: &syn::ExprClosure,
        map_param_scope: Option<HashSet<String>>,
        char_predicate_param_scope: Option<HashSet<String>>,
        expected_return_type: Option<&syn::ReturnType>,
    ) -> String {
        let untyped_param_scope = self.collect_untyped_closure_param_names_for_scope(closure);
        let is_move_closure = closure.capture.is_some();
        let outer_captures = self.collect_move_closure_capture_cpp_names(closure);
        // Determine capture mode
        let capture = if is_move_closure {
            // `move` closure → value-capture by default plus explicit move-init
            // captures for referenced outer locals to preserve move ownership
            // semantics for non-copy values.
            if outer_captures.is_empty() {
                "=".to_string()
            } else {
                let mut capture_parts = vec!["=".to_string()];
                for cpp_name in &outer_captures {
                    let capture_ref = self
                        .lookup_rust_binding_name_for_cpp_name(cpp_name)
                        .and_then(|rust_name| self.lookup_local_binding_type(&rust_name))
                        .is_some_and(|ty| {
                            matches!(self.peel_paren_group_type(&ty), syn::Type::Reference(_))
                        });
                    if capture_ref {
                        // Capturing a reference-typed local by move can force a copy of
                        // the referent (`const T&&`), which fails for move-only payloads.
                        capture_parts.push(format!("&{}", cpp_name));
                    } else {
                        capture_parts.push(format!("{0} = std::move({0})", cpp_name));
                    }
                }
                capture_parts.join(", ")
            }
        } else {
            // Default: borrow environment. Keep non-move closures as `[&]`
            // to preserve Rust's implicit-by-reference capture semantics,
            // including shadowing patterns like `let x = &x` inside closures.
            "&".to_string()
        };
        let lambda_mutability = if is_move_closure { " mutable" } else { "" };

        // Build parameter list.
        let mut closure_param_prelude: Vec<String> = Vec::new();
        let params: Vec<String> = closure
            .inputs
            .iter()
            .enumerate()
            .map(|(idx, p)| {
                let (decl, prelude_stmt) = self.emit_closure_param_with_prelude(p, idx);
                if let Some(stmt) = prelude_stmt {
                    closure_param_prelude.push(stmt);
                }
                decl
            })
            .collect();

        let params_str = params.join(", ");

        let mut inner = self.new_inner_for_block();
        // The `.map(closure)` input type was recorded on `self`; hand it to the
        // sub-codegen so it types the closure's destructured params.
        *inner.pending_map_closure_input_type.borrow_mut() =
            self.pending_map_closure_input_type.borrow_mut().take();
        *inner.pending_closure_param_types.borrow_mut() =
            self.pending_closure_param_types.borrow_mut().take();
        // A REFERENCE map-payload return (Option<&mut V>) forces
        // `-> decltype(auto)` on the lambda — plain deduction decays the
        // reference, and the concrete payload type may not be spellable in
        // this scope (in-scope type params). Only fires when the closure has
        // no explicit Rust annotation.
        let force_decltype_auto_return = self
            .pending_map_closure_return_type
            .borrow_mut()
            .take()
            .is_some()
            && matches!(closure.output, syn::ReturnType::Default);
        inner.bind_closure_params_for_emission(closure);
        if !untyped_param_scope.is_empty() {
            inner.push_untyped_closure_param_scope(untyped_param_scope);
        }
        if let Some(names) = map_param_scope {
            inner.push_iterator_map_closure_param_scope(names);
        }
        if let Some(names) = char_predicate_param_scope {
            inner.push_char_predicate_closure_param_scope(names);
        }
        let resolved_closure_output =
            self.resolve_return_type_infers_from_expected(&closure.output, expected_return_type);
        let suppress_explicit_return_annotation =
            !self.should_push_return_type_hint_for_closure(&resolved_closure_output);
        let fallback_expected_return_ty = expected_return_type.and_then(|rt| match rt {
            syn::ReturnType::Type(_, ty) => {
                if self.type_is_bare_generic_param_like(ty) {
                    return None;
                }
                let expected_peeled = self.peel_reference_paren_group_type(ty);
                if let syn::Type::Path(tp) = expected_peeled
                    && tp.qself.is_none()
                    && tp.path.segments.len() == 1
                {
                    let ident = tp.path.segments[0].ident.to_string();
                    if self.is_type_param_in_scope(&ident) || self.is_struct_type_param(&ident) {
                        return None;
                    }
                }
                if self.type_contains_infer(ty)
                    || self.type_contains_in_scope_type_param(ty)
                    || self.type_contains_unresolved_placeholder_like(ty)
                {
                    None
                } else {
                    Some(ty.as_ref())
                }
            }
            syn::ReturnType::Default => None,
        });
        let lambda_return_annotation = if force_decltype_auto_return {
            " -> decltype(auto)".to_string()
        } else { match &resolved_closure_output {
            syn::ReturnType::Type(_, ty) => {
                if suppress_explicit_return_annotation {
                    String::new()
                } else if self.type_contains_in_scope_type_param(ty)
                    || self.type_is_bare_generic_param_like(ty)
                {
                    String::new()
                } else if let syn::Type::Path(tp) = self.peel_reference_paren_group_type(ty) {
                    if tp.qself.is_none() && tp.path.segments.len() == 1 {
                        let ident = tp.path.segments[0].ident.to_string();
                        if self.is_type_param_in_scope(&ident) || self.is_struct_type_param(&ident)
                        {
                            String::new()
                        } else {
                            let mapped = self.map_type(ty);
                            if mapped == "auto"
                                || mapped.contains("/* TODO")
                                || type_string_has_auto_placeholder(&mapped)
                            {
                                String::new()
                            } else {
                                format!(" -> {}", mapped)
                            }
                        }
                    } else {
                        let mapped = self.map_type(ty);
                        if mapped == "auto"
                            || mapped.contains("/* TODO")
                            || type_string_has_auto_placeholder(&mapped)
                        {
                            String::new()
                        } else {
                            format!(" -> {}", mapped)
                        }
                    }
                } else {
                    let mapped = self.map_type(ty);
                    if mapped == "auto"
                        || mapped.contains("/* TODO")
                        || type_string_has_auto_placeholder(&mapped)
                    {
                        String::new()
                    } else {
                        format!(" -> {}", mapped)
                    }
                }
            }
            syn::ReturnType::Default => fallback_expected_return_ty
                .map(|ty| self.map_type(ty))
                .filter(|mapped| {
                    mapped != "auto"
                        && !mapped.contains("/* TODO")
                        && !type_string_has_auto_placeholder(mapped)
                })
                .map(|mapped| format!(" -> {}", mapped))
                .unwrap_or_default(),
        } };
        // A `-> !` (never) closure maps to `-> [[noreturn]] void`, but the
        // attribute is ill-formed in a lambda trailing-return-type. Strip it —
        // the diverging body still deduces `void`. (It stays valid on real
        // function declarations, which go through a different emit path.)
        let lambda_return_annotation = lambda_return_annotation.replace("[[noreturn]] ", "");
        // `.map(|(event, _mark)| event)` in a fn returning Result<&Event>:
        // an un-annotated lambda's auto deduction DECAYS the reference slot
        // (deleted Event copy; the map deduces Result<Event> where
        // Result<const Event&> is required). When the body is a bare
        // destructured-binding path and the enclosing return hint is
        // Result<R, _>/Option<R> with a clean R, annotate ` -> R`.
        let lambda_return_annotation = if lambda_return_annotation.is_empty()
            && !closure_param_prelude.is_empty()
        {
            self.destructure_identity_closure_ok_annotation(closure)
                .unwrap_or(lambda_return_annotation)
        } else {
            lambda_return_annotation
        };
        let push_outer_expected_hint = matches!(&resolved_closure_output, syn::ReturnType::Default)
            || matches!(
                &resolved_closure_output,
                syn::ReturnType::Type(_, ty) if self.type_contains_infer(ty)
            );

        // Determine if the body is a block or a single expression
        match closure.body.as_ref() {
            syn::Expr::Block(block) => {
                // Multi-statement body
                // Closure blocks are expression bodies even when emitted inside
                // surrounding void-return contexts; keep tail-expression return
                // behavior local to the lambda body.
                inner.return_value_scopes.clear();
                inner.return_type_hints.clear();
                inner.push_return_value_scope("auto");
                if inner.should_push_return_type_hint_for_closure(&resolved_closure_output) {
                    inner.push_return_type_hint(&resolved_closure_output);
                }
                // If we have an additional expected return type from outer context
                // (e.g. Result<T, E> from get_or_try_init), push it on top only
                // when closure output is default or still contains unresolved `_`.
                if let Some(expected_rt) = expected_return_type
                    && push_outer_expected_hint
                {
                    if inner.should_push_return_type_hint_for_closure(expected_rt) {
                        inner.push_return_type_hint(expected_rt);
                    }
                }
                inner.emit_block(&block.block);
                let mut body_str = inner.into_output();
                let closure_expected_unit = expected_return_type
                    .and_then(|rt| match rt {
                        syn::ReturnType::Type(_, ty) => Some(ty.as_ref()),
                        syn::ReturnType::Default => None,
                    })
                    .is_some_and(|ty| self.is_explicit_unit_type(ty));
                if matches!(&resolved_closure_output, syn::ReturnType::Default)
                    && closure_expected_unit
                    && self.block_can_fallthrough_without_value(&block.block)
                {
                    body_str.push_str("return std::make_tuple();\n");
                }
                if !closure_param_prelude.is_empty() {
                    let mut prelude_str = String::new();
                    for stmt in &closure_param_prelude {
                        prelude_str.push_str(stmt);
                        prelude_str.push('\n');
                    }
                    body_str = format!("{}{}", prelude_str, body_str);
                }
                format!(
                    "[{}]({}){}{} {{\n{}}}",
                    capture, params_str, lambda_mutability, lambda_return_annotation, body_str
                )
            }
            _ => {
                // Single expression body → return it
                // Push the explicit return type hint so Err/Ok inside can use it for qualification
                inner.return_value_scopes.clear();
                inner.return_type_hints.clear();
                inner.push_return_value_scope("auto");
                if inner.should_push_return_type_hint_for_closure(&resolved_closure_output) {
                    inner.push_return_type_hint(&resolved_closure_output);
                }
                // If we have an additional expected return type from outer context
                // (e.g. Result<T, E> from get_or_try_init), push it on top only
                // when closure output is default or still contains unresolved `_`.
                if let Some(expected_rt) = expected_return_type
                    && push_outer_expected_hint
                {
                    if inner.should_push_return_type_hint_for_closure(expected_rt) {
                        inner.push_return_type_hint(expected_rt);
                    }
                }
                let body_expected_ty = match &resolved_closure_output {
                    syn::ReturnType::Type(_, ty) => Some(ty.as_ref()),
                    syn::ReturnType::Default => expected_return_type.and_then(|rt| match rt {
                        syn::ReturnType::Type(_, ty) => Some(ty.as_ref()),
                        syn::ReturnType::Default => None,
                    }),
                };
                let body = inner.emit_expr_to_string_with_expected_and_move_if_needed(
                    &closure.body,
                    body_expected_ty,
                );
                let body_diverging = inner.is_expr_diverging(&closure.body);
                if closure_param_prelude.is_empty() {
                    if body_diverging {
                        format!(
                            "[{}]({}){}{} {{ {}; }}",
                            capture, params_str, lambda_mutability, lambda_return_annotation, body
                        )
                    } else {
                        format!(
                            "[{}]({}){}{} {{ return {}; }}",
                            capture, params_str, lambda_mutability, lambda_return_annotation, body
                        )
                    }
                } else {
                    let mut prelude_str = String::new();
                    for stmt in &closure_param_prelude {
                        prelude_str.push_str(stmt);
                        prelude_str.push('\n');
                    }
                    if body_diverging {
                        format!(
                            "[{}]({}){}{} {{\n{}{};\n}}",
                            capture,
                            params_str,
                            lambda_mutability,
                            lambda_return_annotation,
                            prelude_str,
                            body
                        )
                    } else {
                        format!(
                            "[{}]({}){}{} {{\n{}return {};\n}}",
                            capture,
                            params_str,
                            lambda_mutability,
                            lambda_return_annotation,
                            prelude_str,
                            body
                        )
                    }
                }
            }
        }
    }

    /// Emit a single closure parameter.
    pub(super) fn emit_closure_param(&self, pat: &syn::Pat) -> String {
        match pat {
            syn::Pat::Ident(pi) => {
                // Untyped param: |x| → forwarding reference.
                // This preserves Rust call-site ownership/borrow behavior without
                // forcing eager copies for inferred reference parameters.
                format!(
                    "auto&& {}",
                    self.closure_param_cpp_name(&pi.ident.to_string())
                )
            }
            syn::Pat::Type(pt) => {
                // Typed param: |x: i32| → int32_t x
                let ty = self.map_type(&pt.ty);
                let name = match pt.pat.as_ref() {
                    syn::Pat::Ident(pi) => self.closure_param_cpp_name(&pi.ident.to_string()),
                    _ => "_".to_string(),
                };
                format!("{} {}", ty, name)
            }
            syn::Pat::Wild(_) => "auto _".to_string(),
            syn::Pat::Reference(pr) => {
                // |&x| → auto& x  or  |&mut x| → auto& x
                match pr.pat.as_ref() {
                    syn::Pat::Ident(pi) => {
                        format!(
                            "auto& {}",
                            self.closure_param_cpp_name(&pi.ident.to_string())
                        )
                    }
                    syn::Pat::Type(pt) => {
                        let name = match pt.pat.as_ref() {
                            syn::Pat::Ident(pi) => {
                                self.closure_param_cpp_name(&pi.ident.to_string())
                            }
                            _ => "_".to_string(),
                        };
                        format!("auto& {}", name)
                    }
                    _ => format!("auto& {}", self.emit_pat_to_string(&pr.pat)),
                }
            }
            _ => format!("auto {}", self.emit_pat_to_string(pat)),
        }
    }

    /// Emit one element of a tuple/slice structured binding. A nested unit `()`
    /// pattern renders (recursively) as an empty `[]`, which is NOT a valid
    /// structured-binding element — bind such an ignored element to a throwaway
    /// name instead. Example: a HashSet iterator item `(K, ())` destructured as
    /// `|(k, ())|` would otherwise emit `auto [k, []] = …` (a hard parse error).
    fn closure_destructure_binding_element(&self, elem: &syn::Pat) -> String {
        let emitted = self.emit_closure_destructure_pat_to_string(elem);
        if emitted == "[]" {
            "_".to_string()
        } else {
            emitted
        }
    }

    /// See the call site in the closure emitter: identity destructure
    /// closures returning a tuple-slot binding get the enclosing
    /// Result/Option Ok type as their return annotation, so a reference
    /// slot survives deduction.
    fn destructure_identity_closure_ok_annotation(
        &self,
        closure: &syn::ExprClosure,
    ) -> Option<String> {
        let body_val = match closure.body.as_ref() {
            syn::Expr::Block(block_expr) => {
                self.extract_tail_expr_from_block(&block_expr.block)?
            }
            other => other,
        };
        let syn::Expr::Path(path) = self.peel_paren_group_expr(body_val) else {
            return None;
        };
        if path.qself.is_some() || path.path.segments.len() != 1 {
            return None;
        }
        let name = path.path.segments[0].ident.to_string();
        let bound_by_destructure = closure.inputs.iter().any(|input| {
            if !Self::closure_param_needs_body_destructure(input) {
                return false;
            }
            let mut names = HashSet::new();
            self.collect_pattern_value_binding_names(input, &mut names);
            names.contains(&name)
        });
        if !bound_by_destructure {
            return None;
        }
        let ret = self.current_return_type_hint()?;
        let peeled = self.peel_reference_paren_group_type(ret);
        let ok_ty_owned;
        let ok_ty = if let Some((owner, args)) = self.option_or_result_type_args(peeled) {
            if !matches!(owner.as_str(), "Result" | "Option") {
                return None;
            }
            ok_ty_owned = args.first()?.clone();
            &ok_ty_owned
        } else {
            // `type Result<T> = result::Result<T, Error>;`-style aliases:
            // a single-arg generic whose tail resolves to Result/Option
            // carries the Ok type as its sole argument.
            let syn::Type::Path(tp) = peeled else {
                return None;
            };
            let seg = tp.path.segments.last()?;
            let seg_name = seg.ident.to_string();
            let target = self.type_alias_targets.get(&seg_name)?;
            let target_peeled = self.peel_reference_paren_group_type(target);
            let syn::Type::Path(target_path) = target_peeled else {
                return None;
            };
            let target_tail = target_path
                .path
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default();
            if !matches!(target_tail.as_str(), "Result" | "Option") {
                return None;
            }
            let syn::PathArguments::AngleBracketed(ab) = &seg.arguments else {
                return None;
            };
            let mut type_args = ab.args.iter().filter_map(|arg| match arg {
                syn::GenericArgument::Type(ty) => Some(ty),
                _ => None,
            });
            ok_ty_owned = type_args.next()?.clone();
            &ok_ty_owned
        };
        let cpp = self.map_type(ok_ty);
        (!cpp.is_empty() && cpp != "auto" && !type_string_has_auto_placeholder(&cpp))
            .then(|| format!(" -> {}", cpp))
    }

    pub(super) fn emit_closure_destructure_pat_to_string(&self, pat: &syn::Pat) -> String {
        match pat {
            syn::Pat::Ident(pi) => self.closure_param_cpp_name(&pi.ident.to_string()),
            syn::Pat::Wild(_) => "_".to_string(),
            syn::Pat::Tuple(tuple_pat) => {
                let elems: Vec<String> = tuple_pat
                    .elems
                    .iter()
                    .map(|elem| self.closure_destructure_binding_element(elem))
                    .collect();
                format!("[{}]", elems.join(", "))
            }
            syn::Pat::Slice(slice_pat) => {
                let elems: Vec<String> = slice_pat
                    .elems
                    .iter()
                    .map(|elem| self.closure_destructure_binding_element(elem))
                    .collect();
                format!("[{}]", elems.join(", "))
            }
            syn::Pat::Reference(ref_pat) => {
                self.emit_closure_destructure_pat_to_string(&ref_pat.pat)
            }
            syn::Pat::Type(type_pat) => self.emit_closure_destructure_pat_to_string(&type_pat.pat),
            syn::Pat::Paren(paren_pat) => {
                self.emit_closure_destructure_pat_to_string(&paren_pat.pat)
            }
            _ => self.emit_pat_to_string(pat),
        }
    }

    pub(super) fn emit_closure_param_with_prelude(
        &self,
        pat: &syn::Pat,
        index: usize,
    ) -> (String, Option<String>) {
        if matches!(pat, syn::Pat::Wild(_)) {
            // Rust allows repeated wildcard parameters (`|_, _|`), while C++
            // lambda parameters require distinct names.
            return (format!("auto _closure_wild{}", index), None);
        }
        // A TYPED reference-pattern param `&i: &usize` is `Pat::Type` wrapping a
        // `Pat::Reference`; unwrap the annotation so the reference-binding path below
        // (which binds `i` via `deref_if_pointer_like`) still fires. Without this the
        // binding is dropped: the param is emitted as an anonymous `_` while the body
        // still references `i` (indexmap's `move |&i: &usize| entries[i]…`).
        let pat = match pat {
            syn::Pat::Type(pt) if matches!(pt.pat.as_ref(), syn::Pat::Reference(_)) => {
                pt.pat.as_ref()
            }
            other => other,
        };
        // Tuple/struct destructuring can't be used in C++ lambda parameters.
        // Emit a temp parameter and destructure in the body.
        if Self::closure_param_needs_body_destructure(pat) {
            let temp_name = format!("_destruct_param{}", index);
            let mut binding_stmts = Vec::new();
            let mut binding_name_map = HashMap::new();
            let prelude = if self.collect_pattern_binding_stmts_with_cpp_name_map(
                pat,
                &temp_name,
                &mut binding_stmts,
                &mut binding_name_map,
            ) && !binding_stmts.is_empty()
            {
                binding_stmts.join("\n")
            } else {
                let binding = self.emit_closure_destructure_pat_to_string(pat);
                if binding.trim() == "[]" {
                    format!("static_cast<void>({});", temp_name)
                } else {
                    format!("auto&& {} = {};", binding, temp_name)
                }
            };
            return (format!("auto&& {}", temp_name), Some(prelude));
        }
        let syn::Pat::Reference(pr) = pat else {
            return (self.emit_closure_param(pat), None);
        };
        let raw_name = format!("_closure_ref_param{}", index);
        let binding_name = match pr.pat.as_ref() {
            syn::Pat::Ident(pi) => self.closure_param_cpp_name(&pi.ident.to_string()),
            syn::Pat::Type(pt) => match pt.pat.as_ref() {
                syn::Pat::Ident(pi) => self.closure_param_cpp_name(&pi.ident.to_string()),
                _ => self.emit_closure_destructure_pat_to_string(&pr.pat),
            },
            _ => self.emit_closure_destructure_pat_to_string(&pr.pat),
        };
        (
            format!("auto&& {}", raw_name),
            Some(format!(
                "auto {} = rusty::detail::deref_if_pointer_like({});",
                binding_name, raw_name
            )),
        )
    }

    pub(super) fn emit_expr_maybe_move(&self, expr: &syn::Expr) -> String {
        if let Some(deref_expr) = self.emit_lowered_reference_path_deref_if_needed(expr) {
            return deref_expr;
        }
        if self.expr_is_reference_yielding(expr) {
            return self.emit_expr_to_string(expr);
        }
        if let syn::Expr::Path(path) = expr {
            if path.path.segments.len() == 1 {
                let ident = path.path.segments[0].ident.to_string();
                let ident_is_all_caps = ident.len() > 1
                    && ident
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
                if ident_is_all_caps && self.is_const_local_binding_in_scope(&ident) {
                    // Rust const-item uses materialize a fresh value at each use-site.
                    // Cloning avoids deleted copy-ctor failures for move-only payloads.
                    return format!("rusty::clone({})", self.emit_expr_to_string(expr));
                }
            }
            if self.is_associated_const_value_path(&path.path) {
                // Rust associated const values are re-materialized at each use-site.
                // In C++, `Type::CONST` is an lvalue (often `const`), so `std::move`
                // does not avoid copy-ctor requirements. Clone preserves value semantics.
                let inner = self.emit_expr_to_string(expr);
                if self.associated_const_value_path_can_use_directly(&path.path, None) {
                    return inner;
                }
                return format!("rusty::clone({})", inner);
            }
        }
        if self.should_insert_move(expr) {
            let inner = self.emit_expr_to_string(expr);
            format!("std::move({})", inner)
        } else {
            self.emit_expr_to_string(expr)
        }
    }

    pub(super) fn emit_expr_to_string_with_expected_and_move_if_needed(
        &self,
        expr: &syn::Expr,
        expected_ty: Option<&syn::Type>,
    ) -> String {
        if let Some(deref_expr) = self.emit_lowered_reference_path_deref_if_needed(expr) {
            return deref_expr;
        }
        if self.expr_is_reference_yielding(expr) {
            let inner = self.emit_expr_to_string_with_expected(expr, expected_ty);
            if self.should_move_reference_binding_for_expected_value(expr, expected_ty)
                || (expected_ty.is_none() && self.expr_is_unborrowed_pattern_binding(expr))
            {
                return format!("std::move({})", inner);
            }
            return inner;
        }
        if let syn::Expr::Path(path) = expr {
            if path.path.segments.len() == 1 {
                let ident = path.path.segments[0].ident.to_string();
                let ident_is_all_caps = ident.len() > 1
                    && ident
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
                if ident_is_all_caps && self.is_const_local_binding_in_scope(&ident) {
                    let inner = self.emit_expr_to_string_with_expected(expr, expected_ty);
                    return format!("rusty::clone({})", inner);
                }
            }
            if self.is_associated_const_value_path(&path.path) {
                let inner = self.emit_expr_to_string_with_expected(expr, expected_ty);
                if self.associated_const_value_path_can_use_directly(&path.path, expected_ty) {
                    return inner;
                }
                return format!("rusty::clone({})", inner);
            }
        }
        let inner = self.emit_expr_to_string_with_expected(expr, expected_ty);
        let expected_unit = expected_ty
            .map(|ty| self.peel_reference_paren_group_type(ty))
            .is_some_and(|ty| self.is_explicit_unit_type(ty));
        let expr_is_closure = matches!(self.peel_paren_group_expr(expr), syn::Expr::Closure(_));
        if expected_unit && !expr_is_closure && !self.expr_is_known_unit_value_expr(expr) {
            return format!(
                "[&]() {{ static_cast<void>({}); return std::make_tuple(); }}()",
                inner
            );
        }
        if self.should_move_reference_binding_for_expected_value(expr, expected_ty) {
            return format!("std::move({})", inner);
        }
        // A REFERENCE-typed expected slot (the payload of `Option<&mut V>`,
        // a `&T` param) receives the place itself — moving it decays the
        // reference to a value (`kv.1` mapped into an `Option<V&>`
        // annotation produced Option<V>).
        let expected_is_reference = expected_ty
            .is_some_and(|ty| matches!(self.peel_paren_group_type(ty), syn::Type::Reference(_)));
        if expected_is_reference {
            return inner;
        }
        if self.should_insert_move(expr)
            || self.should_insert_move_for_deref_expected_value(expr, expected_ty)
        {
            format!("std::move({})", inner)
        } else {
            inner
        }
    }

    pub(super) fn try_emit_float_trait_assoc_path_segments(&self, segments: &[String]) -> Option<String> {
        let [owner, member] = segments else {
            return None;
        };
        if !self.is_type_param_in_scope(owner) || !self.type_param_has_float_trait_bound(owner) {
            return None;
        }
        match member.as_str() {
            "NUM_BITS" | "NUM_SIG_BITS" | "NUM_EXP_BITS" | "EXP_MASK" | "EXP_BIAS"
            | "EXP_OFFSET" | "MANTISSA_DIGITS" | "MIN_10_EXP" | "MAX_10_EXP" | "MAX_DIGITS10"
            | "IMPLICIT_BIT" | "to_bits" | "is_negative" | "get_sig" | "get_exp" => Some(format!(
                "rusty::float_traits<{}>::{}",
                escape_cpp_keyword(owner),
                escape_cpp_keyword(member)
            )),
            _ => None,
        }
    }

    pub(super) fn try_emit_float_trait_assoc_const_path(&self, path: &syn::Path) -> Option<String> {
        let segments = Self::path_segments_as_strings(path);
        self.try_emit_float_trait_assoc_path_segments(&segments)
            .filter(|_| {
                segments.last().is_some_and(|member| {
                    member
                        .chars()
                        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
                })
            })
    }
}
