use super::*;

impl CodeGen {
    pub(super) fn type_references_module_path(ty: &syn::Type, module_name: &str) -> bool {
        let mut collector = ModulePathReferenceCollector::new(module_name);
        collector.visit_type(ty);
        collector.found
    }

    pub(super) fn map_field_type_with_by_value_cycle_breaking_rewrite(
        &self,
        owner_type: &str,
        field_name: &str,
        ty: &syn::Type,
    ) -> String {
        // Struct/tuple field storage for `[T]` should be mutable-element span.
        // This preserves `AsMut<[T]>` / `DerefMut<Target=[T]>` semantics on
        // transparent DST wrappers while method constness still controls read-only views.
        let mapped = match self.peel_paren_group_type(ty) {
            syn::Type::Slice(slice) => format!("std::span<{}>", self.map_type(&slice.elem)),
            _ => self.map_type(ty),
        };
        if self.should_rewrite_by_value_cycle_field_declaration(owner_type, field_name) {
            format!("rusty::Box<{}>", mapped)
        } else {
            mapped
        }
    }

    pub(super) fn type_tokens_contain_import_alias(&self, ty: &syn::Type) -> bool {
        if self.import_alias_names.is_empty() {
            return false;
        }
        let tokens = ty.to_token_stream().to_string();
        tokens
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .any(|token| !token.is_empty() && self.import_alias_names.contains(token))
    }

    pub(super) fn map_scope_import_binding_target_path(&self, target_path: &str) -> String {
        let normalized = normalize_use_import_path(target_path);
        if let Some(mapped_crate_single) = self.resolve_crate_single_segment_type_import(normalized)
        {
            return mapped_crate_single;
        }
        match classify_use_import(normalized) {
            UseImportAction::Using(mapped) => mapped,
            UseImportAction::Raw(raw_stmt) => raw_stmt
                .strip_prefix("namespace ")
                .and_then(|stmt| stmt.split_once('='))
                .map(|(_, rhs)| rhs.trim().trim_end_matches(';').trim().to_string())
                .filter(|rhs| !rhs.is_empty())
                .unwrap_or_else(|| target_path.to_string()),
            _ => target_path.to_string(),
        }
    }

    /// Cluster C helper: get the host type's leaf name from
    /// `impl ... HostName<...> { }`'s self_ty. For `marker::Foo<X, Y>`
    /// returns "Foo" (the last path segment).
    pub(super) fn type_path_leaf_name(ty: &syn::Type) -> Option<String> {
        let syn::Type::Path(p) = ty else {
            return None;
        };
        Some(p.path.segments.last()?.ident.to_string())
    }

    /// Cluster C helper: stringify a Type via tokens, collapsing whitespace
    /// so `marker :: Immut` and `marker::Immut` compare equal.
    pub(super) fn type_to_normalized_string(ty: &syn::Type) -> String {
        let tokens = ty.to_token_stream().to_string();
        tokens.split_whitespace().collect::<Vec<_>>().join("")
    }

    pub(super) fn type_is_vec_like_expected_type(&self, ty: &syn::Type) -> bool {
        let mapped = self.map_type(self.peel_reference_paren_group_type(ty));
        mapped.starts_with("rusty::Vec<") || mapped.starts_with("std::vector<")
    }

    pub(super) fn type_key_is_declared_alias(&self, type_key: &str) -> bool {
        if self.type_alias_targets.contains_key(type_key) {
            return true;
        }
        let tail = type_key.rsplit("::").next().unwrap_or(type_key);
        let mut tail_matches: Vec<String> = self
            .type_alias_targets
            .keys()
            .filter(|key| key.rsplit("::").next().is_some_and(|k| k == tail))
            .cloned()
            .collect();
        tail_matches.sort();
        tail_matches.dedup();
        tail_matches.len() == 1
    }

    pub(super) fn type_is_reference_to_slice(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Reference(r) => match r.elem.as_ref() {
                syn::Type::Slice(_) => true,
                other => self.type_is_reference_to_slice(other),
            },
            syn::Type::Paren(p) => self.type_is_reference_to_slice(&p.elem),
            syn::Type::Group(g) => self.type_is_reference_to_slice(&g.elem),
            _ => false,
        }
    }

    pub(super) fn type_is_pointer_like_owner_type(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return false;
        };
        tp.path
            .segments
            .last()
            .is_some_and(|seg| Self::is_pointer_like_autoderef_owner_name(&seg.ident.to_string()))
    }

    pub(super) fn type_has_drop_impl(&self, type_name: &str) -> bool {
        let method_name = "drop".to_string();
        self.drop_trait_methods
            .contains(&(type_name.to_string(), method_name.clone()))
            || self
                .drop_trait_methods
                .contains(&(self.scoped_type_key(type_name), method_name))
    }

    /// Returns true if `name` is a well-known move-only wrapper type whose
    /// presence as a field disqualifies the enclosing struct from a
    /// `= default` copy ctor / copy-assign. Used to decide whether to emit
    /// `= delete` versus `= default` on Drop-trait structs. Matching is
    /// conservative — pre-existing aliases that wrap one of these (e.g.
    /// `type Guarded<T> = Mutex<T>`) won't be caught, but those would also
    /// fail to copy at the C++ level so the user would notice. Cheaper than
    /// `is_copy_constructible_v`-style probing at codegen time.
    pub(super) fn type_name_is_known_non_copyable(name: &str) -> bool {
        matches!(
            name,
            "SpinMutex"
                | "Mutex"
                | "RwLock"
                | "Box"
                | "UnsafeCell"
                | "RefCell"
                | "Cell"
                | "OnceCell"
                | "OnceLock"
                | "LazyLock"
                | "unique_ptr"
        )
    }

    /// Recursively checks whether `ty` contains (at any depth, including
    /// generic parameters) a type whose final path segment is in the
    /// known-non-copyable list. Walks tuples, references, and generic args
    /// of `Type::Path`.
    pub(super) fn type_contains_known_non_copyable(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(tp) => {
                let Some(last) = tp.path.segments.last() else {
                    return false;
                };
                if Self::type_name_is_known_non_copyable(&last.ident.to_string()) {
                    return true;
                }
                if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                    args.args.iter().any(|arg| match arg {
                        syn::GenericArgument::Type(inner) => {
                            self.type_contains_known_non_copyable(inner)
                        }
                        _ => false,
                    })
                } else {
                    false
                }
            }
            syn::Type::Reference(r) => self.type_contains_known_non_copyable(&r.elem),
            syn::Type::Tuple(t) => {
                t.elems.iter().any(|e| self.type_contains_known_non_copyable(e))
            }
            syn::Type::Paren(p) => self.type_contains_known_non_copyable(&p.elem),
            syn::Type::Group(g) => self.type_contains_known_non_copyable(&g.elem),
            _ => false,
        }
    }

    /// Emit a nested function definition as a local callable.
    /// Rust nested `fn` items cannot capture non-item locals, so emit
    /// captureless callables by default. If a nested function references a
    /// sibling nested function (lowered to another local callable), use a local
    /// capture to keep the emitted C++ valid.
    pub(super) fn type_contains_named_type_params(&self, ty: &syn::Type, names: &HashSet<String>) -> bool {
        let ty = self.peel_paren_group_type(ty);
        match ty {
            syn::Type::Path(tp) => {
                if tp.qself.is_none()
                    && tp.path.segments.len() == 1
                    && names.contains(&tp.path.segments[0].ident.to_string())
                {
                    return true;
                }
                tp.path.segments.iter().any(|seg| {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        args.args.iter().any(|arg| match arg {
                            syn::GenericArgument::Type(inner) => {
                                self.type_contains_named_type_params(inner, names)
                            }
                            _ => false,
                        })
                    } else {
                        false
                    }
                })
            }
            syn::Type::Reference(r) => self.type_contains_named_type_params(&r.elem, names),
            syn::Type::Ptr(p) => self.type_contains_named_type_params(&p.elem, names),
            syn::Type::Slice(s) => self.type_contains_named_type_params(&s.elem, names),
            syn::Type::Array(a) => self.type_contains_named_type_params(&a.elem, names),
            syn::Type::Tuple(tuple) => tuple
                .elems
                .iter()
                .any(|elem| self.type_contains_named_type_params(elem, names)),
            syn::Type::Paren(p) => self.type_contains_named_type_params(&p.elem, names),
            syn::Type::Group(g) => self.type_contains_named_type_params(&g.elem, names),
            _ => false,
        }
    }

    pub(super) fn map_variant_ctor_param_type(&self, ty: &syn::Type, ctor_name: &str) -> String {
        self.normalize_variant_ctor_param_type(ty, ctor_name, self.map_type(ty))
    }

    pub(super) fn map_variant_ctor_param_type_for_field(
        &self,
        owner_type: &str,
        variant_name: &str,
        field_name: &str,
        ty: &syn::Type,
        ctor_name: &str,
    ) -> String {
        let rewrite_field_key = Self::format_by_value_field_name(Some(variant_name), field_name);
        let mapped = self.map_field_type_with_by_value_cycle_breaking_rewrite(
            owner_type,
            &rewrite_field_key,
            ty,
        );
        self.normalize_variant_ctor_param_type(ty, ctor_name, mapped)
    }

    /// Check if a type references a given name (recursively through generics).
    pub(super) fn type_references_name(&self, ty: &syn::Type, name: &str) -> bool {
        match ty {
            syn::Type::Path(tp) => tp.path.segments.iter().any(|seg| {
                if seg.ident == name {
                    return true;
                }
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    args.args.iter().any(|arg| {
                        if let syn::GenericArgument::Type(t) = arg {
                            self.type_references_name(t, name)
                        } else {
                            false
                        }
                    })
                } else {
                    false
                }
            }),
            syn::Type::Reference(r) => self.type_references_name(&r.elem, name),
            syn::Type::Ptr(p) => self.type_references_name(&p.elem, name),
            syn::Type::Slice(s) => self.type_references_name(&s.elem, name),
            syn::Type::Array(a) => self.type_references_name(&a.elem, name),
            syn::Type::Tuple(t) => t
                .elems
                .iter()
                .any(|elem| self.type_references_name(elem, name)),
            syn::Type::Paren(p) => self.type_references_name(&p.elem, name),
            syn::Type::Group(g) => self.type_references_name(&g.elem, name),
            syn::Type::BareFn(bf) => {
                bf.inputs
                    .iter()
                    .any(|arg| self.type_references_name(&arg.ty, name))
                    || match &bf.output {
                        syn::ReturnType::Type(_, ret) => self.type_references_name(ret, name),
                        syn::ReturnType::Default => false,
                    }
            }
            _ => false,
        }
    }

    pub(super) fn type_contains_unbound_single_letter_generic(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(tp) => {
                let is_unbound_generic_like = |ident: &str| {
                    ident.len() == 1
                        && ident.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                        && !self.is_type_param_in_scope(ident)
                        && !self.is_local_type_name_in_scope(ident)
                        && !self.data_enum_name_matches(ident)
                        && !self.local_declared_types.contains(ident)
                        && !self
                            .local_declared_types
                            .iter()
                            .any(|decl| decl.rsplit("::").next().is_some_and(|tail| tail == ident))
                        && !self.declared_item_names.contains(ident)
                        && types::map_primitive_type(ident).is_none()
                };

                if tp.qself.is_none() && !tp.path.segments.is_empty() {
                    let first_ident = tp.path.segments[0].ident.to_string();
                    // Catch both bare `A` and dependent projections like `A::Item`.
                    if is_unbound_generic_like(&first_ident) {
                        return true;
                    }
                }
                tp.path.segments.iter().any(|seg| {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        args.args.iter().any(|arg| {
                            if let syn::GenericArgument::Type(inner_ty) = arg {
                                self.type_contains_unbound_single_letter_generic(inner_ty)
                            } else {
                                false
                            }
                        })
                    } else {
                        false
                    }
                })
            }
            syn::Type::Reference(r) => self.type_contains_unbound_single_letter_generic(&r.elem),
            syn::Type::Ptr(p) => self.type_contains_unbound_single_letter_generic(&p.elem),
            syn::Type::Slice(s) => self.type_contains_unbound_single_letter_generic(&s.elem),
            syn::Type::Array(a) => self.type_contains_unbound_single_letter_generic(&a.elem),
            syn::Type::Tuple(t) => t
                .elems
                .iter()
                .any(|elem| self.type_contains_unbound_single_letter_generic(elem)),
            syn::Type::BareFn(bf) => {
                bf.inputs
                    .iter()
                    .any(|arg| self.type_contains_unbound_single_letter_generic(&arg.ty))
                    || match &bf.output {
                        syn::ReturnType::Type(_, ret_ty) => {
                            self.type_contains_unbound_single_letter_generic(ret_ty)
                        }
                        syn::ReturnType::Default => false,
                    }
            }
            syn::Type::ImplTrait(it) => it.bounds.iter().any(|bound| match bound {
                syn::TypeParamBound::Trait(tb) => {
                    tb.path.segments.iter().any(|seg| match &seg.arguments {
                        syn::PathArguments::Parenthesized(args) => {
                            args.inputs.iter().any(|arg_ty| {
                                self.type_contains_unbound_single_letter_generic(arg_ty)
                            }) || match &args.output {
                                syn::ReturnType::Type(_, ret_ty) => {
                                    self.type_contains_unbound_single_letter_generic(ret_ty)
                                }
                                syn::ReturnType::Default => false,
                            }
                        }
                        syn::PathArguments::AngleBracketed(args) => args.args.iter().any(|arg| {
                            if let syn::GenericArgument::Type(inner_ty) = arg {
                                self.type_contains_unbound_single_letter_generic(inner_ty)
                            } else {
                                false
                            }
                        }),
                        _ => false,
                    })
                }
                _ => false,
            }),
            syn::Type::TraitObject(to) => to.bounds.iter().any(|bound| match bound {
                syn::TypeParamBound::Trait(tb) => {
                    tb.path.segments.iter().any(|seg| match &seg.arguments {
                        syn::PathArguments::Parenthesized(args) => {
                            args.inputs.iter().any(|arg_ty| {
                                self.type_contains_unbound_single_letter_generic(arg_ty)
                            }) || match &args.output {
                                syn::ReturnType::Type(_, ret_ty) => {
                                    self.type_contains_unbound_single_letter_generic(ret_ty)
                                }
                                syn::ReturnType::Default => false,
                            }
                        }
                        syn::PathArguments::AngleBracketed(args) => args.args.iter().any(|arg| {
                            if let syn::GenericArgument::Type(inner_ty) = arg {
                                self.type_contains_unbound_single_letter_generic(inner_ty)
                            } else {
                                false
                            }
                        }),
                        _ => false,
                    })
                }
                _ => false,
            }),
            syn::Type::Paren(p) => self.type_contains_unbound_single_letter_generic(&p.elem),
            syn::Type::Group(g) => self.type_contains_unbound_single_letter_generic(&g.elem),
            _ => false,
        }
    }

    pub(super) fn map_extension_impl_return_type(
        &self,
        output: &syn::ReturnType,
        _self_ty: &syn::Type,
        self_cpp_ty: &str,
        associated_type_cpp_bindings: &HashMap<String, String>,
    ) -> String {
        match output {
            syn::ReturnType::Default => "void".to_string(),
            syn::ReturnType::Type(_, ty) => {
                if Self::is_plain_self_type(ty) {
                    self_cpp_ty.to_string()
                } else {
                    self.map_extension_impl_type_with_self_assoc(
                        ty,
                        self_cpp_ty,
                        associated_type_cpp_bindings,
                    )
                }
            }
        }
    }

    pub(super) fn map_extension_impl_type_with_self_assoc(
        &self,
        ty: &syn::Type,
        self_cpp_ty: &str,
        associated_type_cpp_bindings: &HashMap<String, String>,
    ) -> String {
        let mapped = if Self::is_plain_self_type(ty) {
            self_cpp_ty.to_string()
        } else {
            self.map_type(ty)
        };
        let rewritten = self.rewrite_extension_self_assoc_cpp_type(
            mapped,
            self_cpp_ty,
            associated_type_cpp_bindings,
        );
        let rewritten = if rewritten.contains("::MAX_STR_LEN") || rewritten.contains("::Buffer") {
            self.rewrite_extension_integer_assoc_projection_fallbacks(&rewritten)
        } else {
            rewritten
        };
        collapse_redundant_typename_tokens(&rewritten)
    }

    pub(super) fn type_current_struct_assoc_aliases_emitted(&self, ty: &syn::Type) -> bool {
        let mut assoc_names = HashSet::new();
        self.collect_current_struct_assoc_projection_names(ty, &mut assoc_names);
        if assoc_names.is_empty() {
            return false;
        }
        assoc_names.into_iter().all(|name| {
            self.emitted_non_method_member_names
                .last()
                .is_some_and(|scope| scope.contains(&escape_cpp_keyword(&name)))
        })
    }

    pub(super) fn type_references_current_struct_assoc_projection(&self, ty: &syn::Type) -> bool {
        let mut assoc_names = HashSet::new();
        self.collect_current_struct_assoc_projection_names(ty, &mut assoc_names);
        !assoc_names.is_empty()
    }

    pub(super) fn map_type_with_explicit_owner_generic_recovery(&self, ty: &syn::Type) -> String {
        let mut mapped = self.map_type(ty);
        if !mapped.contains('<')
            && let Some(recovered) = self.recover_explicit_owner_type_from_type(ty, &mapped)
        {
            mapped = recovered;
        }
        mapped
    }

    pub(super) fn map_impl_method_return_type(&self, method: &syn::ImplItemFn) -> String {
        if self.impl_method_is_fmt_formatter_method(method) {
            return "rusty::fmt::Result".to_string();
        }
        self.map_return_type(&method.sig.output)
    }

    pub(super) fn type_is_reference_like(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Reference(_) => true,
            syn::Type::Paren(p) => self.type_is_reference_like(&p.elem),
            syn::Type::Group(g) => self.type_is_reference_like(&g.elem),
            _ => false,
        }
    }

    pub(super) fn type_hint_is_map_like(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return false;
        };
        tp.path.segments.last().is_some_and(|seg| {
            matches!(
                seg.ident.to_string().as_str(),
                "Map" | "HashMap" | "BTreeMap"
            )
        })
    }

    pub(super) fn type_supports_variant_context(&self, type_name: &str) -> bool {
        self.data_enum_name_matches(type_name)
            || self.runtime_match_enum_kind_by_name(type_name).is_some()
            || (type_name == "Either"
                && !self.is_local_type_name_in_scope("Either")
                && !self.local_declared_types.contains("Either"))
            || (type_name == "Bound" && !self.data_enum_name_matches("Bound"))
    }

    pub(super) fn type_is_non_raw_reference(&self, ty: &syn::Type) -> bool {
        fn peel_paren_group_type<'a>(ty: &'a syn::Type) -> &'a syn::Type {
            match ty {
                syn::Type::Paren(paren) => peel_paren_group_type(&paren.elem),
                syn::Type::Group(group) => peel_paren_group_type(&group.elem),
                _ => ty,
            }
        }
        let ty = peel_paren_group_type(ty);
        let syn::Type::Reference(reference) = ty else {
            return false;
        };
        !matches!(peel_paren_group_type(&reference.elem), syn::Type::Ptr(_))
    }

    pub(super) fn type_is_single_in_scope_type_param(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return false;
        };
        if tp.qself.is_some() || tp.path.segments.len() != 1 {
            return false;
        }
        let seg = tp.path.segments.last().expect("checked len == 1 above");
        matches!(seg.arguments, syn::PathArguments::None)
            && self.is_type_param_in_scope(&seg.ident.to_string())
    }

    pub(super) fn type_contains_infer(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Infer(_) => true,
            syn::Type::Path(tp) => tp.path.segments.iter().any(|seg| match &seg.arguments {
                syn::PathArguments::AngleBracketed(args) => args.args.iter().any(|arg| match arg {
                    syn::GenericArgument::Type(inner) => self.type_contains_infer(inner),
                    _ => false,
                }),
                _ => false,
            }),
            syn::Type::Reference(r) => self.type_contains_infer(&r.elem),
            syn::Type::Paren(p) => self.type_contains_infer(&p.elem),
            syn::Type::Group(g) => self.type_contains_infer(&g.elem),
            syn::Type::Tuple(t) => t.elems.iter().any(|elem| self.type_contains_infer(elem)),
            syn::Type::Array(a) => self.type_contains_infer(&a.elem),
            syn::Type::Slice(s) => self.type_contains_infer(&s.elem),
            _ => false,
        }
    }

    /// Returns true if the type contains any in-scope type parameters
    /// (e.g., `A`, `T`, `E` from the enclosing generic context).
    /// Such types cannot be used as concrete template arguments.
    /// Returns true if the type contains type parameters that are NOT in scope
    /// (i.e., leaked from an outer context). Types with in-scope params are fine.
    pub(super) fn type_contains_in_scope_type_param(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(tp) => {
                // Check each path segment's identifier
                for seg in &tp.path.segments {
                    let name = seg.ident.to_string();
                    // If it's a valid type param in scope, that's fine — don't flag it.
                    if self.is_type_param_in_scope(&name) || self.is_struct_type_param(&name) {
                        continue;
                    }
                    // Reject unresolved single-segment uppercase names that look like
                    // generic type params but aren't declared in the current scope.
                    // For example, `A` from an enclosing struct's generic context
                    // leaking into a free test function.
                    // Only flag single-letter names that are NOT known concrete types.
                    if name.len() == 1
                        && name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                        && !self.local_declared_types.contains(&name)
                        && !self.declared_item_names.contains(&name)
                    {
                        return true;
                    }
                }
                // Check generic arguments recursively
                tp.path.segments.iter().any(|seg| match &seg.arguments {
                    syn::PathArguments::AngleBracketed(args) => {
                        args.args.iter().any(|arg| match arg {
                            syn::GenericArgument::Type(inner) => {
                                self.type_contains_in_scope_type_param(inner)
                            }
                            _ => false,
                        })
                    }
                    _ => false,
                })
            }
            syn::Type::Reference(r) => self.type_contains_in_scope_type_param(&r.elem),
            syn::Type::Ptr(p) => self.type_contains_in_scope_type_param(&p.elem),
            syn::Type::Paren(p) => self.type_contains_in_scope_type_param(&p.elem),
            syn::Type::Group(g) => self.type_contains_in_scope_type_param(&g.elem),
            syn::Type::Tuple(t) => t
                .elems
                .iter()
                .any(|elem| self.type_contains_in_scope_type_param(elem)),
            syn::Type::Array(a) => self.type_contains_in_scope_type_param(&a.elem),
            syn::Type::Slice(s) => self.type_contains_in_scope_type_param(&s.elem),
            syn::Type::ImplTrait(it) => it.bounds.iter().any(|bound| match bound {
                syn::TypeParamBound::Trait(tb) => {
                    if tb.path.segments.iter().any(|seg| {
                        self.type_contains_in_scope_type_param(&syn::Type::Path(syn::TypePath {
                            qself: None,
                            path: syn::Path::from(seg.ident.clone()),
                        }))
                    }) {
                        return true;
                    }
                    tb.path.segments.iter().any(|seg| match &seg.arguments {
                        syn::PathArguments::AngleBracketed(args) => {
                            args.args.iter().any(|arg| match arg {
                                syn::GenericArgument::Type(inner) => {
                                    self.type_contains_in_scope_type_param(inner)
                                }
                                _ => false,
                            })
                        }
                        syn::PathArguments::Parenthesized(args) => {
                            args.inputs
                                .iter()
                                .any(|inner| self.type_contains_in_scope_type_param(inner))
                                || match &args.output {
                                    syn::ReturnType::Type(_, ty) => {
                                        self.type_contains_in_scope_type_param(ty)
                                    }
                                    syn::ReturnType::Default => false,
                                }
                        }
                        syn::PathArguments::None => false,
                    })
                }
                _ => false,
            }),
            syn::Type::TraitObject(obj) => obj.bounds.iter().any(|bound| match bound {
                syn::TypeParamBound::Trait(tb) => {
                    tb.path.segments.iter().any(|seg| match &seg.arguments {
                        syn::PathArguments::AngleBracketed(args) => {
                            args.args.iter().any(|arg| match arg {
                                syn::GenericArgument::Type(inner) => {
                                    self.type_contains_in_scope_type_param(inner)
                                }
                                _ => false,
                            })
                        }
                        syn::PathArguments::Parenthesized(args) => {
                            args.inputs
                                .iter()
                                .any(|inner| self.type_contains_in_scope_type_param(inner))
                                || match &args.output {
                                    syn::ReturnType::Type(_, ty) => {
                                        self.type_contains_in_scope_type_param(ty)
                                    }
                                    syn::ReturnType::Default => false,
                                }
                        }
                        syn::PathArguments::None => false,
                    })
                }
                _ => false,
            }),
            syn::Type::BareFn(bare_fn) => {
                bare_fn
                    .inputs
                    .iter()
                    .any(|arg| self.type_contains_in_scope_type_param(&arg.ty))
                    || match &bare_fn.output {
                        syn::ReturnType::Type(_, ty) => self.type_contains_in_scope_type_param(ty),
                        syn::ReturnType::Default => false,
                    }
            }
            syn::Type::Infer(_) => true,
            _ => false,
        }
    }

    pub(super) fn type_resolves_to_tuple_alias(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return false;
        };
        if tp.path.segments.is_empty() {
            return false;
        }
        let joined = tp
            .path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if self.tuple_type_aliases.contains_key(&joined) {
            return true;
        }
        tp.path
            .segments
            .last()
            .is_some_and(|seg| self.tuple_type_aliases.contains_key(&seg.ident.to_string()))
    }

    pub(super) fn type_is_range_with_private_end_field(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return false;
        };
        let Some(last) = tp.path.segments.last() else {
            return false;
        };
        let last_name = last.ident.to_string();
        if matches!(
            last_name.as_str(),
            "range" | "Range" | "range_inclusive" | "RangeInclusive"
        ) {
            return true;
        }
        if last_name == "Self"
            && let Some(current_struct) = self.current_struct.as_ref()
        {
            let tail = current_struct
                .rsplit("::")
                .next()
                .unwrap_or(current_struct.as_str());
            return matches!(
                tail,
                "range" | "Range" | "range_inclusive" | "RangeInclusive"
            );
        }
        false
    }

    pub(super) fn type_is_concrete_result_like_hint(&self, ty: &syn::Type) -> bool {
        self.result_type_args_owned(ty).is_some()
            && !self.type_maps_to_auto_placeholder_like(ty)
            && !self.type_maps_to_branch_local_decltype(ty)
    }

    pub(super) fn type_is_mut_rusty_string_reference(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Reference(reference) = ty else {
            return false;
        };
        if reference.mutability.is_none() {
            return false;
        }
        self.canonical_into_target_cpp_type(&self.map_type(&reference.elem)) == "rusty::String"
    }

    pub(super) fn type_is_string_view_like(&self, ty: &syn::Type) -> bool {
        self.canonical_into_target_cpp_type(
            &self.map_type(self.peel_reference_paren_group_type(ty)),
        ) == "std::string_view"
    }

    pub(super) fn type_is_slice_or_span_like(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        match ty {
            syn::Type::Slice(_) | syn::Type::Array(_) => true,
            syn::Type::Path(tp) => {
                if tp
                    .path
                    .segments
                    .last()
                    .is_some_and(|seg| seg.ident == "span")
                {
                    return true;
                }
                let canonical = self.canonical_into_target_cpp_type(&self.map_type(ty));
                canonical.starts_with("std::span<") || canonical.starts_with("span<")
            }
            _ => false,
        }
    }

    pub(super) fn type_uses_as_str_string_view_coercion(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return false;
        };
        let Some(last) = tp.path.segments.last() else {
            return false;
        };
        matches!(last.ident.to_string().as_str(), "ArrayString" | "String")
    }

    pub(super) fn type_is_char_like(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return false;
        };
        tp.path
            .segments
            .last()
            .is_some_and(|seg| matches!(seg.ident.to_string().as_str(), "char" | "char32_t"))
    }

    pub(super) fn type_looks_like_assoc_projection(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(tp) => {
                if tp.qself.is_some() {
                    return true;
                }
                if tp.path.segments.len() >= 2
                    && tp.path.segments.last().is_some_and(|seg| {
                        let first = tp
                            .path
                            .segments
                            .first()
                            .map(|s| s.ident.to_string())
                            .unwrap_or_default();
                        matches!(seg.arguments, syn::PathArguments::None)
                            && (first == "Self"
                                || first.chars().next().is_some_and(|c| c.is_ascii_uppercase()))
                    })
                {
                    return true;
                }
                tp.path.segments.iter().any(|seg| {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        args.args.iter().any(|arg| match arg {
                            syn::GenericArgument::Type(inner_ty) => {
                                self.type_looks_like_assoc_projection(inner_ty)
                            }
                            _ => false,
                        })
                    } else {
                        false
                    }
                })
            }
            syn::Type::Reference(r) => self.type_looks_like_assoc_projection(&r.elem),
            syn::Type::Ptr(p) => self.type_looks_like_assoc_projection(&p.elem),
            syn::Type::Slice(s) => self.type_looks_like_assoc_projection(&s.elem),
            syn::Type::Array(a) => self.type_looks_like_assoc_projection(&a.elem),
            syn::Type::Paren(p) => self.type_looks_like_assoc_projection(&p.elem),
            syn::Type::Group(g) => self.type_looks_like_assoc_projection(&g.elem),
            syn::Type::Tuple(tup) => tup
                .elems
                .iter()
                .any(|elem| self.type_looks_like_assoc_projection(elem)),
            _ => false,
        }
    }

    pub(super) fn type_is_current_struct_self_type(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        if let syn::Type::Path(tp) = ty
            && tp.qself.is_none()
            && tp.path.segments.len() == 1
        {
            let ident = tp.path.segments[0].ident.to_string();
            if ident == "Self" {
                return true;
            }
            if self.current_struct.as_ref().is_some_and(|current| {
                current == &ident || current.rsplit("::").next() == Some(ident.as_str())
            }) {
                return true;
            }
        }
        let Some(current) = self.current_struct.as_ref() else {
            return false;
        };
        let mapped = self.map_type(ty);
        if mapped.contains('&')
            || mapped == "auto"
            || mapped.contains("/* TODO")
            || type_string_has_auto_placeholder(&mapped)
        {
            return false;
        }
        let mapped = mapped.trim_start_matches("typename ").trim();
        mapped == current
            || mapped.rsplit("::").next() == Some(current.as_str())
            || current.rsplit("::").next() == mapped.rsplit("::").next()
    }

    pub(super) fn type_contains_mut_reference(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_paren_group_type(ty);
        match ty {
            syn::Type::Reference(r) => {
                r.mutability.is_some() || self.type_contains_mut_reference(&r.elem)
            }
            syn::Type::Tuple(t) => t
                .elems
                .iter()
                .any(|elem| self.type_contains_mut_reference(elem)),
            syn::Type::Path(tp) => tp.path.segments.iter().any(|seg| {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    args.args.iter().any(|arg| match arg {
                        syn::GenericArgument::Type(inner) => {
                            self.type_contains_mut_reference(inner)
                        }
                        _ => false,
                    })
                } else {
                    false
                }
            }),
            syn::Type::Array(a) => self.type_contains_mut_reference(&a.elem),
            syn::Type::Slice(s) => self.type_contains_mut_reference(&s.elem),
            syn::Type::Ptr(p) => self.type_contains_mut_reference(&p.elem),
            _ => false,
        }
    }

    pub(super) fn type_needs_typed_tuple_ctor(&self, ty: &syn::Type) -> bool {
        if self.should_soften_dependent_assoc_mode()
            && (self.type_contains_dependent_assoc(ty)
                || self.type_references_current_struct_assoc(ty))
        {
            return false;
        }
        match ty {
            syn::Type::Path(tp) => {
                if tp.qself.is_some() {
                    return true;
                }
                if tp.path.segments.len() >= 2
                    && tp.path.segments.first().is_some_and(|seg| {
                        seg.ident
                            .to_string()
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_uppercase())
                    })
                {
                    return true;
                }
                tp.path.segments.iter().any(|seg| {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        args.args.iter().any(|arg| match arg {
                            syn::GenericArgument::Type(inner) => {
                                self.type_needs_typed_tuple_ctor(inner)
                            }
                            _ => false,
                        })
                    } else {
                        false
                    }
                })
            }
            syn::Type::Reference(_) => true,
            syn::Type::Ptr(p) => self.type_needs_typed_tuple_ctor(&p.elem),
            syn::Type::Array(a) => self.type_needs_typed_tuple_ctor(&a.elem),
            syn::Type::Slice(s) => self.type_needs_typed_tuple_ctor(&s.elem),
            syn::Type::Paren(p) => self.type_needs_typed_tuple_ctor(&p.elem),
            syn::Type::Group(g) => self.type_needs_typed_tuple_ctor(&g.elem),
            syn::Type::Tuple(t) => t
                .elems
                .iter()
                .any(|elem| self.type_needs_typed_tuple_ctor(elem)),
            _ => false,
        }
    }

    pub(super) fn type_arg_is_value_identifier_type(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return false;
        };
        if tp.qself.is_some() || tp.path.segments.len() != 1 {
            return false;
        }
        let seg = &tp.path.segments[0];
        if !matches!(seg.arguments, syn::PathArguments::None) {
            return false;
        }
        self.owner_template_arg_is_value_identifier(&seg.ident.to_string())
    }

    pub(super) fn type_is_u8_slice_like(&self, ty: &syn::Type) -> bool {
        if self
            .expected_array_element_type(Some(ty))
            .is_some_and(Self::is_u8_syn_type)
        {
            return true;
        }
        // Some normalization paths carry mapped C++ span-like expected types.
        let compact = normalize_token_text(ty.to_token_stream().to_string())
            .chars()
            .filter(|c| !c.is_ascii_whitespace())
            .collect::<String>();
        matches!(
            compact.as_str(),
            "std::span<constuint8_t>"
                | "span<constuint8_t>"
                | "std::span<uint8_t>"
                | "span<uint8_t>"
                | "std::span<constunsignedchar>"
                | "span<constunsignedchar>"
                | "std::span<unsignedchar>"
                | "span<unsignedchar>"
        )
    }

    pub(super) fn type_prefers_direct_binary_ordering_surface(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return false;
        };
        let owner_path = tp
            .path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        let owner_tail = tp
            .path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default();
        ["cmp", "partial_cmp"].iter().any(|method| {
            self.lookup_owner_method_has_receiver(&owner_path, method) == Some(true)
                || self.lookup_owner_method_has_receiver(&owner_tail, method) == Some(true)
        })
    }

    pub(super) fn type_contains_tuple_placeholder_marker(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        match ty {
            syn::Type::Path(tp) => {
                let Some(last) = tp.path.segments.last() else {
                    return false;
                };
                if last.ident == "tuple" && matches!(last.arguments, syn::PathArguments::None) {
                    return true;
                }
                if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                    return args.args.iter().any(|arg| match arg {
                        syn::GenericArgument::Type(inner) => {
                            self.type_contains_tuple_placeholder_marker(inner)
                        }
                        _ => false,
                    });
                }
                false
            }
            syn::Type::Tuple(tuple) => tuple
                .elems
                .iter()
                .any(|elem| self.type_contains_tuple_placeholder_marker(elem)),
            syn::Type::Reference(reference) => {
                self.type_contains_tuple_placeholder_marker(&reference.elem)
            }
            syn::Type::Paren(paren) => self.type_contains_tuple_placeholder_marker(&paren.elem),
            syn::Type::Group(group) => self.type_contains_tuple_placeholder_marker(&group.elem),
            syn::Type::Slice(slice) => self.type_contains_tuple_placeholder_marker(&slice.elem),
            syn::Type::Array(array) => self.type_contains_tuple_placeholder_marker(&array.elem),
            _ => false,
        }
    }

    pub(super) fn type_contains_unresolved_self_type_path(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(tp) => {
                if tp.qself.is_none()
                    && tp.path.segments.len() == 1
                    && tp.path.segments[0].ident == "Self"
                    && self.current_struct.is_none()
                {
                    return true;
                }
                if let Some(qself) = tp.qself.as_ref()
                    && self.type_contains_unresolved_self_type_path(&qself.ty)
                {
                    return true;
                }
                tp.path.segments.iter().any(|seg| match &seg.arguments {
                    syn::PathArguments::AngleBracketed(args) => {
                        args.args.iter().any(|arg| match arg {
                            syn::GenericArgument::Type(inner) => {
                                self.type_contains_unresolved_self_type_path(inner)
                            }
                            _ => false,
                        })
                    }
                    syn::PathArguments::Parenthesized(args) => {
                        args.inputs
                            .iter()
                            .any(|inner| self.type_contains_unresolved_self_type_path(inner))
                            || match &args.output {
                                syn::ReturnType::Type(_, ty) => {
                                    self.type_contains_unresolved_self_type_path(ty)
                                }
                                syn::ReturnType::Default => false,
                            }
                    }
                    syn::PathArguments::None => false,
                })
            }
            syn::Type::Reference(r) => self.type_contains_unresolved_self_type_path(&r.elem),
            syn::Type::Ptr(p) => self.type_contains_unresolved_self_type_path(&p.elem),
            syn::Type::Paren(p) => self.type_contains_unresolved_self_type_path(&p.elem),
            syn::Type::Group(g) => self.type_contains_unresolved_self_type_path(&g.elem),
            syn::Type::Tuple(t) => t
                .elems
                .iter()
                .any(|elem| self.type_contains_unresolved_self_type_path(elem)),
            syn::Type::Array(a) => self.type_contains_unresolved_self_type_path(&a.elem),
            syn::Type::Slice(s) => self.type_contains_unresolved_self_type_path(&s.elem),
            syn::Type::ImplTrait(it) => it.bounds.iter().any(|bound| match bound {
                syn::TypeParamBound::Trait(tb) => {
                    if tb.path.segments.iter().any(|seg| {
                        self.type_contains_unresolved_self_type_path(&syn::Type::Path(
                            syn::TypePath {
                                qself: None,
                                path: syn::Path::from(seg.ident.clone()),
                            },
                        ))
                    }) {
                        return true;
                    }
                    tb.path.segments.iter().any(|seg| match &seg.arguments {
                        syn::PathArguments::AngleBracketed(args) => {
                            args.args.iter().any(|arg| match arg {
                                syn::GenericArgument::Type(inner) => {
                                    self.type_contains_unresolved_self_type_path(inner)
                                }
                                _ => false,
                            })
                        }
                        syn::PathArguments::Parenthesized(args) => {
                            args.inputs
                                .iter()
                                .any(|inner| self.type_contains_unresolved_self_type_path(inner))
                                || match &args.output {
                                    syn::ReturnType::Type(_, ty) => {
                                        self.type_contains_unresolved_self_type_path(ty)
                                    }
                                    syn::ReturnType::Default => false,
                                }
                        }
                        syn::PathArguments::None => false,
                    })
                }
                _ => false,
            }),
            syn::Type::TraitObject(obj) => obj.bounds.iter().any(|bound| match bound {
                syn::TypeParamBound::Trait(tb) => tb.path.segments.iter().any(|seg| {
                    if self.type_contains_unresolved_self_type_path(&syn::Type::Path(
                        syn::TypePath {
                            qself: None,
                            path: syn::Path::from(seg.ident.clone()),
                        },
                    )) {
                        return true;
                    }
                    match &seg.arguments {
                        syn::PathArguments::AngleBracketed(args) => {
                            args.args.iter().any(|arg| match arg {
                                syn::GenericArgument::Type(inner) => {
                                    self.type_contains_unresolved_self_type_path(inner)
                                }
                                _ => false,
                            })
                        }
                        syn::PathArguments::Parenthesized(args) => {
                            args.inputs
                                .iter()
                                .any(|inner| self.type_contains_unresolved_self_type_path(inner))
                                || match &args.output {
                                    syn::ReturnType::Type(_, ty) => {
                                        self.type_contains_unresolved_self_type_path(ty)
                                    }
                                    syn::ReturnType::Default => false,
                                }
                        }
                        syn::PathArguments::None => false,
                    }
                }),
                _ => false,
            }),
            _ => false,
        }
    }

    pub(super) fn type_contains_unresolved_placeholder_like(&self, ty: &syn::Type) -> bool {
        self.type_contains_infer(ty)
            || self.type_contains_in_scope_type_param(ty)
            || self.type_contains_unresolved_self_type_path(ty)
    }

    pub(super) fn type_is_concrete_hint_candidate(&self, ty: &syn::Type) -> bool {
        !self.type_contains_infer(ty)
            && !self.type_contains_in_scope_type_param(ty)
            && !self.type_contains_unbound_single_letter_generic(ty)
            && !self.type_contains_unresolved_placeholder_like(ty)
            && !self.type_contains_single_segment_local_binding_type_path(ty)
    }

    pub(super) fn type_is_placeholder_hint_candidate_allow_scoped_generics(&self, ty: &syn::Type) -> bool {
        !self.type_contains_infer(ty)
            && !self.type_contains_unbound_single_letter_generic(ty)
            && !self.type_contains_unresolved_self_type_path(ty)
            && !self.type_maps_to_auto_placeholder_like(ty)
            && !self.type_contains_single_segment_local_binding_type_path(ty)
    }

    pub(super) fn type_contains_single_segment_local_binding_type_path(&self, ty: &syn::Type) -> bool {
        match self.peel_reference_paren_group_type(ty) {
            syn::Type::Path(tp) => {
                if tp.qself.is_none()
                    && tp.path.segments.len() == 1
                    && let Some(seg) = tp.path.segments.first()
                    && matches!(seg.arguments, syn::PathArguments::None)
                {
                    let ident = seg.ident.to_string();
                    let lower_unresolved = ident
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_ascii_lowercase())
                        && types::map_primitive_type(&ident).is_none()
                        && types::map_std_type(&ident).is_none()
                        && !self.is_local_type_name_in_scope(&ident)
                        && !self.local_declared_types.contains(&ident);
                    if self.is_local_binding_in_scope(&ident) || lower_unresolved {
                        return true;
                    }
                }
                tp.path.segments.iter().any(|seg| match &seg.arguments {
                    syn::PathArguments::AngleBracketed(args) => {
                        args.args.iter().any(|arg| match arg {
                            syn::GenericArgument::Type(inner) => {
                                self.type_contains_single_segment_local_binding_type_path(inner)
                            }
                            _ => false,
                        })
                    }
                    syn::PathArguments::Parenthesized(args) => {
                        args.inputs.iter().any(|inner| {
                            self.type_contains_single_segment_local_binding_type_path(inner)
                        }) || match &args.output {
                            syn::ReturnType::Type(_, output) => {
                                self.type_contains_single_segment_local_binding_type_path(output)
                            }
                            syn::ReturnType::Default => false,
                        }
                    }
                    syn::PathArguments::None => false,
                })
            }
            syn::Type::Reference(r) => {
                self.type_contains_single_segment_local_binding_type_path(&r.elem)
            }
            syn::Type::Ptr(p) => self.type_contains_single_segment_local_binding_type_path(&p.elem),
            syn::Type::Paren(p) => {
                self.type_contains_single_segment_local_binding_type_path(&p.elem)
            }
            syn::Type::Group(g) => {
                self.type_contains_single_segment_local_binding_type_path(&g.elem)
            }
            syn::Type::Tuple(t) => t
                .elems
                .iter()
                .any(|elem| self.type_contains_single_segment_local_binding_type_path(elem)),
            syn::Type::Array(a) => {
                self.type_contains_single_segment_local_binding_type_path(&a.elem)
            }
            syn::Type::Slice(s) => {
                self.type_contains_single_segment_local_binding_type_path(&s.elem)
            }
            _ => false,
        }
    }

    pub(super) fn type_maps_to_auto_placeholder_like(&self, ty: &syn::Type) -> bool {
        let mapped = self.map_type(ty);
        mapped.contains("/* TODO") || type_string_has_auto_placeholder(&mapped)
    }

    pub(super) fn type_maps_to_branch_local_decltype(&self, ty: &syn::Type) -> bool {
        self.map_type(ty).contains("decltype((std::move(")
    }

    pub(super) fn type_is_bare_generic_param_like(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return false;
        };
        if tp.qself.is_some() || tp.path.segments.len() != 1 {
            return false;
        }
        let ident = tp.path.segments[0].ident.to_string();
        if self.is_type_param_in_scope(&ident) || self.is_struct_type_param(&ident) {
            return true;
        }
        if self.is_local_type_name_in_scope(&ident)
            || self.local_declared_types.contains(&ident)
            || self.declared_item_names.contains(&ident)
            || types::map_primitive_type(&ident).is_some()
        {
            return false;
        }
        let mut has_alpha = false;
        ident.chars().all(|c| {
            if c.is_ascii_alphabetic() {
                has_alpha = true;
                c.is_ascii_uppercase()
            } else {
                c.is_ascii_digit() || c == '_'
            }
        }) && has_alpha
    }

    pub(super) fn type_has_iterator_surface(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        match ty {
            syn::Type::ImplTrait(it) => it.bounds.iter().any(|bound| {
                let syn::TypeParamBound::Trait(trait_bound) = bound else {
                    return false;
                };
                trait_bound.path.segments.last().is_some_and(|seg| {
                    let name = seg.ident.to_string();
                    matches!(name.as_str(), "Iter" | "IntoIter" | "IterNames")
                        || name.ends_with("Iterator")
                        || name.ends_with("Iter")
                })
            }),
            syn::Type::TraitObject(obj) => obj.bounds.iter().any(|bound| {
                let syn::TypeParamBound::Trait(trait_bound) = bound else {
                    return false;
                };
                trait_bound.path.segments.last().is_some_and(|seg| {
                    let name = seg.ident.to_string();
                    matches!(name.as_str(), "Iter" | "IntoIter" | "IterNames")
                        || name.ends_with("Iterator")
                        || name.ends_with("Iter")
                })
            }),
            syn::Type::Path(tp) => {
                let Some(last) = tp.path.segments.last() else {
                    return false;
                };
                let name = last.ident.to_string();
                if matches!(name.as_str(), "Iter" | "IntoIter" | "IterNames")
                    || name.ends_with("Iterator")
                    || name.ends_with("Iter")
                {
                    return true;
                }
                let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
                    return false;
                };
                args.args.iter().any(|arg| {
                    matches!(arg, syn::GenericArgument::AssocType(assoc) if assoc.ident == "Item")
                })
            }
            _ => false,
        }
    }

    pub(super) fn type_is_fixed_array_like(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        if matches!(ty, syn::Type::Array(_)) {
            return true;
        }
        let mapped = self
            .map_type(ty)
            .chars()
            .filter(|ch| !ch.is_ascii_whitespace())
            .collect::<String>();
        mapped.starts_with("std::array<") || mapped.starts_with("::std::array<")
    }

    pub(super) fn map_angle_bracketed_type_args(
        &self,
        args: &syn::AngleBracketedGenericArguments,
    ) -> Vec<String> {
        self.type_arg_nesting.set(self.type_arg_nesting.get() + 1);
        let mapped: Vec<String> = args
            .args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                syn::GenericArgument::Const(c) => Some(self.emit_expr_to_string(c)),
                _ => None,
            })
            .collect();
        self.type_arg_nesting.set(self.type_arg_nesting.get() - 1);
        mapped
    }

    pub(super) fn map_assoc_into_iter_cpp_type(&self, owner_cpp: &str) -> Option<String> {
        let owner_trimmed = owner_cpp.trim();
        if owner_trimmed == "auto"
            || owner_trimmed.contains("/* TODO")
            || type_string_has_auto_placeholder(owner_trimmed)
        {
            return None;
        }
        let owner_norm = owner_trimmed
            .trim_start_matches("typename ")
            .trim()
            .trim_start_matches("::");
        if owner_norm == "auto"
            || owner_norm.contains("/* TODO")
            || type_string_has_auto_placeholder(owner_norm)
        {
            return None;
        }
        Some(format!(
            "decltype(rusty::iter(std::declval<{}>()))",
            owner_norm
        ))
    }

    pub(super) fn type_path_matches_slice_iter_family(path: &syn::Path, family: &str) -> bool {
        let mut path_idents: Vec<String> =
            path.segments.iter().map(|s| s.ident.to_string()).collect();
        while matches!(
            path_idents.first().map(|s| s.as_str()),
            Some("crate" | "self" | "super")
        ) {
            path_idents.remove(0);
        }
        let normalized: &[String] = if matches!(
            path_idents.first().map(|s| s.as_str()),
            Some("std" | "core" | "alloc")
        ) {
            &path_idents[1..]
        } else {
            &path_idents
        };
        matches!(normalized, [slice, iter] if slice == "slice" && iter == family)
    }

    pub(super) fn type_is_primitive_str_path(ty: &syn::Type) -> bool {
        let syn::Type::Path(tp) = ty else {
            return false;
        };
        if tp.qself.is_some() {
            return false;
        }
        let idents: Vec<String> = tp
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        if idents.is_empty() {
            return false;
        }
        if idents.len() == 1 {
            return idents[0] == "str";
        }
        if idents.last().is_some_and(|last| last == "str")
            && idents
                .iter()
                .nth_back(1)
                .is_some_and(|prev| prev == "primitive")
        {
            return true;
        }
        false
    }

    pub(super) fn map_qualified_primitive_alias_path(path: &str) -> Option<&'static str> {
        let segments: Vec<&str> = path
            .trim()
            .trim_start_matches("::")
            .split("::")
            .filter(|seg| !seg.is_empty())
            .collect();
        if segments.is_empty() {
            return None;
        }
        let primitive = types::map_primitive_type(segments.last().copied()?)?;
        if segments.len() == 1 {
            return Some(primitive);
        }
        let prev = segments[segments.len() - 2];
        if matches!(prev, "core" | "std") {
            return Some(primitive);
        }
        if prev == "primitive" && segments.len() >= 3 {
            let prev2 = segments[segments.len() - 3];
            if matches!(prev2, "core" | "std") {
                return Some(primitive);
            }
        }
        None
    }

    pub(super) fn map_type(&self, ty: &syn::Type) -> String {
        // `NonNull<[u8]>` is Rust's fat byte-pointer return from
        // `Allocator::allocate`. Our `rusty::alloc::Allocator` concept (and
        // `rusty::ptr::NonNull<T>`) is thin-pointer-only, so map the slice
        // argument away to keep the call-site types compatible with what the
        // concept's `allocate` actually returns. This is intentionally lossy
        // (the slice's length information is discarded), matching how most
        // call sites use `.cast::<T>()` on the result anyway.
        if let syn::Type::Path(tp) = ty
            && tp.qself.is_none()
            && let Some(last) = tp.path.segments.last()
            && last.ident == "NonNull"
            && let syn::PathArguments::AngleBracketed(args) = &last.arguments
            && args.args.len() == 1
            && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
            && let syn::Type::Slice(slice) = inner
            && let syn::Type::Path(elem_tp) = slice.elem.as_ref()
            && elem_tp.qself.is_none()
            && let Some(elem_last) = elem_tp.path.segments.last()
            && elem_last.ident == "u8"
        {
            let mut rewritten = tp.clone();
            let last_seg = rewritten.path.segments.last_mut().unwrap();
            let elem_ty = (*slice.elem).clone();
            last_seg.arguments =
                syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    colon2_token: None,
                    lt_token: Default::default(),
                    args: std::iter::once(syn::GenericArgument::Type(elem_ty)).collect(),
                    gt_token: Default::default(),
                });
            return self.map_type(&syn::Type::Path(rewritten));
        }
        match ty {
            syn::Type::Path(tp) => {
                if tp.qself.is_none()
                    && tp.path.segments.len() == 1
                    && tp.path.segments[0].ident == "Self"
                {
                    let mut mapped_self = self.emit_path_to_string(&tp.path);
                    if !mapped_self.contains('<')
                        && let Some(recovered) =
                            self.recover_omitted_local_generic_type_args(&tp.path, &mapped_self)
                    {
                        mapped_self = recovered;
                    }
                    return mapped_self;
                }
                if tp.qself.is_none() && Self::type_path_is_fmt_result(&tp.path) {
                    return "rusty::fmt::Result".to_string();
                }
                if tp.qself.is_none()
                    && tp.path.segments.len() == 1
                    && let Some(current_struct) = self.current_struct.as_ref()
                    && let Some(seg) = tp.path.segments.first()
                {
                    let local_name = seg.ident.to_string();
                    let current_tail = current_struct.rsplit("::").next().unwrap_or(current_struct);
                    if (local_name == current_tail
                        || escape_cpp_keyword(&local_name) == current_tail)
                        && let syn::PathArguments::AngleBracketed(args) = &seg.arguments
                    {
                        let generic_args: Vec<String> = args
                            .args
                            .iter()
                            .filter_map(|arg| match arg {
                                syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                                syn::GenericArgument::Const(c) => Some(self.emit_expr_to_string(c)),
                                _ => None,
                            })
                            .collect();
                        if !generic_args.is_empty() {
                            let base = escape_cpp_keyword(&local_name);
                            return self.maybe_prefix_typename_for_dependent_type_path(
                                tp,
                                format!("{}<{}>", base, generic_args.join(", ")),
                            );
                        }
                    }
                }
                if let Some(mapped_std_iter_item) = self.try_map_std_iter_item_projection_type(tp) {
                    return mapped_std_iter_item;
                }
                if let Some(mapped_assoc_item) =
                    self.try_map_dependent_assoc_item_projection_type(tp)
                {
                    return mapped_assoc_item;
                }
                if let Some(mapped_float_assoc) = self.try_map_float_trait_assoc_type_path(tp) {
                    return self
                        .maybe_prefix_typename_for_dependent_type_path(tp, mapped_float_assoc);
                }
                if let Some(mapped_owner_into_iter) = self.try_map_owner_assoc_into_iter_type(tp) {
                    return mapped_owner_into_iter;
                }
                let mut alias_resolved_path: Option<syn::TypePath> = None;
                let alias_shadowed_by_local_type = tp.path.segments.len() == 1
                    && tp.path.segments.last().is_some_and(|seg| {
                        let local_name = seg.ident.to_string();
                        self.is_local_type_name_in_scope(&local_name)
                            || self.current_scope_declares_type_name(&local_name)
                            || self.current_module_declares_type_name_exact(&local_name)
                            || self.current_owner_module_declares_type_name(&local_name)
                    });
                if tp.qself.is_none() && !alias_shadowed_by_local_type {
                    // Guard alias-chain mapping from unbounded self-expansion. Some crates define
                    // alias graphs that can repeatedly re-wrap the same path under suffix matching.
                    // Cap this local chain and continue mapping with the latest resolved path.
                    let mut alias_ty = syn::Type::Path(tp.clone());
                    let mut seen_alias_shapes = HashSet::new();
                    seen_alias_shapes.insert(alias_ty.to_token_stream().to_string());
                    let mut alias_steps = 0usize;
                    while alias_steps < 32 {
                        let Some(resolved_alias) = self.resolve_type_alias_once(&alias_ty) else {
                            break;
                        };
                        if resolved_alias == alias_ty {
                            break;
                        }
                        if Self::alias_expansion_looks_self_wrapping(&alias_ty, &resolved_alias) {
                            break;
                        }
                        let resolved_shape = resolved_alias.to_token_stream().to_string();
                        if !seen_alias_shapes.insert(resolved_shape) {
                            break;
                        }
                        alias_ty = resolved_alias;
                        alias_steps += 1;
                        let should_continue = matches!(alias_ty, syn::Type::Path(ref next_tp) if next_tp.qself.is_none());
                        if !should_continue {
                            break;
                        }
                    }
                    if alias_steps > 0 {
                        match alias_ty {
                            syn::Type::Path(resolved_path) => {
                                alias_resolved_path = Some(resolved_path);
                            }
                            other => return self.map_type(&other),
                        }
                    }
                }
                let tp = alias_resolved_path.as_ref().unwrap_or(tp);
                if Self::type_is_primitive_str_path(&syn::Type::Path(tp.clone())) {
                    return "std::string_view".to_string();
                }
                if !self.in_forward_decl_signature
                    && let Some(scope_bound_ty) = self.try_map_scope_bound_type_path(tp)
                {
                    let scope_bound_ty =
                        Self::rewrite_builtin_namespace_aliases_in_type(&scope_bound_ty);
                    let scope_bound_ty =
                        Self::rewrite_private_keyword_namespace_in_type_path(&scope_bound_ty);
                    let scope_bound_ty = self
                        .maybe_force_global_for_shadowed_module_root_in_type_path(&scope_bound_ty);
                    return self.maybe_prefix_typename_for_dependent_type_path(tp, scope_bound_ty);
                }

                // Handle qualified self types: <T as Trait>::Assoc → T::Assoc
                if let Some(qself) = &tp.qself {
                    let self_type = self.normalize_qself_base_for_assoc(&self.map_type(&qself.ty));
                    // Get the path segments after the `as Trait` part
                    let assoc_segments: Vec<String> = tp
                        .path
                        .segments
                        .iter()
                        .skip(qself.position)
                        .map(|s| s.ident.to_string())
                        .collect();
                    if !assoc_segments.is_empty() {
                        // Try to resolve the associated type through impl blocks.
                        // E.g., <TestFlags as PublicFlags>::Internal → InternalBitFlags
                        if assoc_segments.len() == 1 {
                            let assoc_name = &assoc_segments[0];
                            if let Some(resolved) =
                                self.resolve_assoc_type_from_impl_blocks(&self_type, assoc_name)
                            {
                                if !self.mapped_assoc_type_contains_unbound_placeholder(&resolved) {
                                    return self.maybe_prefix_typename_for_dependent_path(resolved);
                                }
                            }
                        }
                        if let Some(struct_name) = &self.current_struct {
                            if &self_type == struct_name {
                                return assoc_segments.join("::");
                            }
                        }
                        let assoc_path = format!("{}::{}", self_type, assoc_segments.join("::"));
                        if let Some(mapped_into_iter) =
                            self.rewrite_mapped_assoc_into_iter_cpp_type(&assoc_path)
                        {
                            return mapped_into_iter;
                        }
                        if assoc_segments.len() == 1 && assoc_segments[0] == "IntoIter" {
                            let sibling_assoc = match qself.ty.as_ref() {
                                syn::Type::Reference(reference)
                                    if reference.mutability.is_some() =>
                                {
                                    "IterMutImpl"
                                }
                                syn::Type::Reference(_) => "IterImpl",
                                _ => "IntoIterImpl",
                            };
                            if let Some(sibling_into_iter) =
                                self.resolve_owner_sibling_assoc_type(&self_type, sibling_assoc)
                            {
                                return sibling_into_iter;
                            }
                            if let Some(sibling_into_iter) =
                                self.resolve_owner_sibling_assoc_type(&self_type, "IntoIter")
                            {
                                return sibling_into_iter;
                            }
                        }
                        return self.maybe_prefix_typename_for_dependent_type_path(tp, assoc_path);
                    }
                    return self_type;
                }

                let mut path_str = self.emit_path_to_string(&tp.path);
                if tp.qself.is_none()
                    && tp.path.segments.len() == 1
                    && tp
                        .path
                        .segments
                        .first()
                        .is_some_and(|seg| matches!(seg.arguments, syn::PathArguments::None))
                    && let Some(current_struct) = self.current_struct.as_ref()
                {
                    let local_name = tp.path.segments[0].ident.to_string();
                    let current_tail = current_struct.rsplit("::").next().unwrap_or(current_struct);
                    if local_name == current_tail || escape_cpp_keyword(&local_name) == current_tail
                    {
                        let mapped_base = self
                            .current_named_module_root_type_cpp_name(&local_name)
                            .unwrap_or_else(|| escape_cpp_keyword(&local_name));
                        let has_defaulted_params = self
                            .declared_type_param_defaults_for_path(&tp.path)
                            .or_else(|| {
                                self.current_struct_declared_type_key_for_recovery(
                                    &tp.path,
                                    &mapped_base,
                                )
                                .and_then(|key| self.declared_type_param_defaults.get(&key))
                            })
                            .is_some_and(|defaults| defaults.iter().any(|entry| entry.is_some()));
                        if has_defaulted_params
                            && let Some(recovered) =
                                self.recover_omitted_local_generic_type_args(&tp.path, &mapped_base)
                        {
                            return self
                                .maybe_prefix_typename_for_dependent_type_path(tp, recovered);
                        }
                        return mapped_base;
                    }
                }
                path_str = Self::strip_crate_root_cpp_path(&path_str);
                if path_str == "private" {
                    path_str = "private_".to_string();
                } else if path_str == "::private" {
                    path_str = "::private_".to_string();
                } else {
                    if path_str.starts_with("private::") {
                        path_str = format!("private_::{}", &path_str["private::".len()..]);
                    } else if path_str.starts_with("::private::") {
                        path_str = format!("::private_::{}", &path_str["::private::".len()..]);
                    }
                    if path_str.contains("::private::") {
                        path_str = path_str.replace("::private::", "::private_::");
                    }
                }
                if path_str == "__private::Result" {
                    path_str = "rusty::Result".to_string();
                }
                if path_str == "fmt::Result" || path_str.ends_with("::fmt::Result") {
                    path_str = "rusty::fmt::Result".to_string();
                }
                if let Some(mapped_std_btree_map) =
                    self.try_map_std_collections_btree_map_type(tp, &path_str)
                {
                    return mapped_std_btree_map;
                }
                let path_is_current_struct_assoc_projection =
                    self.path_is_current_struct_assoc_projection(&tp.path);
                if path_is_current_struct_assoc_projection {
                    if self.current_struct_is_generic()
                        && let Some(assoc_seg) = tp.path.segments.iter().nth(1)
                    {
                        let assoc_name = assoc_seg.ident.to_string();
                        if self.current_struct_assoc_alias_exists(&assoc_name) {
                            let assoc_cpp_name = escape_cpp_keyword(&assoc_name);
                            let alias_emitted = self
                                .emitted_non_method_member_names
                                .last()
                                .is_some_and(|scope| scope.contains(&assoc_cpp_name));
                            if alias_emitted {
                                return assoc_cpp_name;
                            }
                        }
                    }
                    let assoc_ty = syn::Type::Path(tp.clone());
                    if let Some(resolved_assoc) =
                        self.resolve_current_struct_assoc_projection_cpp_type(&assoc_ty)
                    {
                        if !self.mapped_assoc_type_contains_unbound_placeholder(&resolved_assoc) {
                            return self.maybe_prefix_typename_for_dependent_path(resolved_assoc);
                        }
                    }
                }
                if self.current_struct.is_some() && path_str.starts_with("Self::") {
                    path_str = path_str.trim_start_matches("Self::").to_string();
                }
                if let Some(mapped_primitive) = Self::map_qualified_primitive_alias_path(&path_str)
                {
                    return mapped_primitive.to_string();
                }
                if let Some(struct_name) = &self.current_struct {
                    let struct_prefix = format!("{}::", struct_name);
                    if path_str.starts_with(&struct_prefix) {
                        let candidate = path_str.trim_start_matches(&struct_prefix).to_string();
                        if let Some(first_assoc) = candidate.split("::").next() {
                            if self.is_local_type_name_in_scope(first_assoc) {
                                path_str = candidate;
                            }
                        }
                    }
                }
                if tp.path.segments.len() == 1
                    && tp.path.segments[0].ident != "Self"
                    && !path_str.contains("::")
                {
                    let local_name = tp.path.segments[0].ident.to_string();
                    let local_type_declared_here = self
                        .current_scope_declares_type_name(&local_name)
                        || self.current_module_declares_type_name_exact(&local_name)
                        || self.current_owner_module_declares_type_name(&local_name);
                    let looks_like_type_name = local_name
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_ascii_uppercase() || ch == '_');
                    let has_scope_import_binding = self
                        .resolve_scope_import_binding_path_for_scope(
                            &self.module_stack.join("::"),
                            &local_name,
                        )
                        .or_else(|| {
                            self.resolve_scope_import_binding_path_for_scope("", &local_name)
                        })
                        .is_some();
                    let should_try_rebound = types::map_primitive_type(&local_name).is_none()
                        && !self.is_type_param_in_scope(&local_name)
                        && !local_type_declared_here
                        && (has_scope_import_binding
                            || (looks_like_type_name
                                && !self.current_scope_declares_type_name(&local_name))
                            || self.current_struct_has_data_enum_variant_named(&local_name));
                    if should_try_rebound
                        && let Some(rebound) =
                            self.resolve_single_segment_scope_import_bound_type(&local_name)
                        && !rebound.is_empty()
                        && (rebound != local_name || rebound.starts_with("::"))
                    {
                        path_str = rebound;
                    }
                }
                if tp.path.segments.len() == 1
                    && !path_str.contains("::")
                    && tp.path.segments[0].ident != "Self"
                {
                    let local_name = tp.path.segments[0].ident.to_string();
                    let local_type_declared_here = self
                        .current_scope_declares_type_name(&local_name)
                        || self.current_module_declares_type_name_exact(&local_name)
                        || self.current_owner_module_declares_type_name(&local_name);
                    // Preserve primitive aliases (`usize`, `isize`, etc.) on
                    // the primitive mapping path. Rebinding through scope
                    // imports like `use std::usize;` produces malformed C++
                    // spellings (`std::usize`).
                    if types::map_primitive_type(&local_name).is_none()
                        && !local_type_declared_here
                        && let Some(rebound) =
                            self.resolve_single_segment_scope_import_bound_type(&local_name)
                        && !rebound.is_empty()
                        && rebound != local_name
                    {
                        path_str = rebound;
                    }
                }
                if tp.path.segments.len() == 1
                    && !path_str.contains("::")
                    && tp.path.segments[0].ident != "Self"
                {
                    let local_name = tp.path.segments[0].ident.to_string();
                    let scoped_local_name = self.scoped_type_key(&local_name);
                    let has_variant_name_shadow = self
                        .current_struct_has_data_enum_variant_named(&local_name)
                        || self.any_data_enum_variant_named(&local_name);
                    if has_variant_name_shadow
                        && (self.current_module_declares_type_name_exact(&local_name)
                            || self.local_declared_types.contains(&local_name)
                            || self.local_declared_types.contains(&scoped_local_name)
                            || self.type_alias_targets.contains_key(&local_name)
                            || self.type_alias_targets.contains_key(&scoped_local_name))
                    {
                        let module_scope = self.module_stack.join("::");
                        if !module_scope.is_empty() {
                            let escaped_scope =
                                self.escape_and_rename_qualified_name(&module_scope);
                            path_str =
                                format!("::{}::{}", escaped_scope, escape_cpp_keyword(&local_name));
                        }
                    }
                }
                if let Some(mapped_primitive) = Self::map_qualified_primitive_alias_path(&path_str)
                {
                    return mapped_primitive.to_string();
                }
                if tp.path.segments.len() == 1
                    && tp.path.segments[0].ident == "Bound"
                    && !self.is_local_type_name_in_scope("Bound")
                    && !self.local_declared_types.contains("Bound")
                {
                    path_str = "rusty::Bound".to_string();
                }
                if tp.path.segments.len() == 1
                    && tp.path.segments[0].ident == "Range"
                    && !self.is_local_type_name_in_scope("Range")
                    && !self.local_declared_types.contains("Range")
                {
                    path_str = "rusty::range".to_string();
                }
                if tp.path.segments.len() == 1
                    && tp.path.segments[0].ident == "RangeInclusive"
                    && !self.is_local_type_name_in_scope("RangeInclusive")
                    && !self.local_declared_types.contains("RangeInclusive")
                {
                    path_str = "rusty::range_inclusive".to_string();
                }
                if tp.path.segments.len() == 1
                    && tp.path.segments[0].ident == "RangeFrom"
                    && !self.is_local_type_name_in_scope("RangeFrom")
                    && !self.local_declared_types.contains("RangeFrom")
                {
                    path_str = "rusty::range_from".to_string();
                }
                if tp.path.segments.len() == 1
                    && tp.path.segments[0].ident == "RangeTo"
                    && !self.is_local_type_name_in_scope("RangeTo")
                    && !self.local_declared_types.contains("RangeTo")
                {
                    path_str = "rusty::range_to".to_string();
                }
                if tp.path.segments.len() == 1
                    && tp.path.segments[0].ident == "RangeToInclusive"
                    && !self.is_local_type_name_in_scope("RangeToInclusive")
                    && !self.local_declared_types.contains("RangeToInclusive")
                {
                    path_str = "rusty::range_to_inclusive".to_string();
                }
                if tp.path.segments.len() == 1
                    && tp.path.segments[0].ident == "RangeFull"
                    && !self.is_local_type_name_in_scope("RangeFull")
                    && !self.local_declared_types.contains("RangeFull")
                {
                    path_str = "rusty::range_full".to_string();
                }
                if tp.path.segments.len() == 1
                    && tp.path.segments[0].ident == "Either"
                    && !self.is_local_type_name_in_scope("Either")
                    && !self.local_declared_types.contains("Either")
                {
                    path_str = "rusty::Either".to_string();
                }
                if path_str == "RcWeak" || path_str.ends_with("::RcWeak") {
                    // rusty.cppm previously exposed `rusty::Weak` as an
                    // ambiguous direct re-export of rc::Weak. After the
                    // ambiguity fix (commit dcbf08…), the rusty umbrella
                    // only exports `rusty::rc::Weak` and `rusty::sync::Weak`.
                    // `RcWeak` (from `use alloc::rc::Weak as RcWeak`) must
                    // map to the `rc` form. Mirrors `Rc` → `rusty::Rc`
                    // (also under `rc`).
                    path_str = "rusty::rc::Weak".to_string();
                }
                if path_str == "ArcWeak" || path_str.ends_with("::ArcWeak") {
                    path_str = "rusty::sync::Weak".to_string();
                }
                if let Some(mapped_into_iter) =
                    self.rewrite_mapped_assoc_into_iter_cpp_type(&path_str)
                {
                    return mapped_into_iter;
                }
                if let Some(mapped_iter_adapter) = self.try_map_iterator_adapter_type(tp) {
                    return mapped_iter_adapter;
                }

                if self.in_forward_decl_signature
                    && tp.path.segments.len() == 1
                    && !path_str.contains("::")
                {
                    let local_name = tp.path.segments[0].ident.to_string();
                    if !local_name
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_ascii_uppercase())
                    {
                        // Keep primitive aliases (`usize`, etc.) on the normal mapping path.
                        // Forward-decl import binding qualification only applies to type-like names.
                        // Skip to avoid malformed spellings like `std::usize`.
                    } else {
                        if !self.current_module_declares_type_name_exact(&local_name)
                            && let Some(bound_target) = self
                                .resolve_scope_import_binding_path(&local_name)
                                .or_else(|| {
                                    self.resolve_scope_import_binding_path_for_scope(
                                        "",
                                        &local_name,
                                    )
                                })
                        {
                            let mut rewritten =
                                self.rewrite_cpp_import_bound_type_spelling(&bound_target);
                            rewritten = self.resolve_nested_local_reexport_path(&rewritten);
                            if let Some(resolved_nested) =
                                self.try_resolve_nested_local_type_path(&rewritten)
                            {
                                rewritten = resolved_nested;
                            }
                            let rewritten_trimmed = rewritten.trim_start_matches("::");
                            let rewritten_parts: Vec<&str> = rewritten_trimmed
                                .split("::")
                                .filter(|seg| !seg.is_empty())
                                .collect();
                            if rewritten_parts.len() >= 2 {
                                let owner_module = rewritten_parts[0];
                                let owner_type = rewritten_parts[1];
                                if self.should_rebind_owner_to_descendant(owner_module, owner_type)
                                    && let Some(resolved_owner) = self
                                        .resolve_descendant_type_path_in_module(
                                            owner_module,
                                            owner_type,
                                        )
                                {
                                    let mut rebuilt: Vec<String> = resolved_owner
                                        .split("::")
                                        .filter(|seg| !seg.is_empty())
                                        .map(|seg| seg.to_string())
                                        .collect();
                                    rebuilt.extend(
                                        rewritten_parts
                                            .iter()
                                            .skip(2)
                                            .map(|seg| (*seg).to_string()),
                                    );
                                    let rebuilt_path = rebuilt.join("::");
                                    if !rebuilt_path.is_empty() {
                                        rewritten = rebuilt_path;
                                    }
                                }
                            }
                            if rewritten.contains("::")
                                && rewritten.trim_start_matches("::") != local_name
                            {
                                path_str = rewritten;
                            }
                        }
                        if !path_str.contains("::")
                            && let Some(scoped) = self
                                .resolve_unique_forward_decl_type_path(&local_name)
                                .or_else(|| self.resolve_unique_nonlocal_type_path(&local_name))
                        {
                            path_str = scoped;
                        }
                    }
                }

                if tp.path.segments.len() == 1
                    && tp
                        .path
                        .segments
                        .first()
                        .is_some_and(|seg| matches!(seg.arguments, syn::PathArguments::None))
                {
                    let local_name = tp.path.segments[0].ident.to_string();
                    if self.is_local_type_name_in_scope(&local_name)
                        || self.local_declared_types.contains(&local_name)
                    {
                        if let Some(remapped) =
                            self.remap_forward_decl_qualified_type_path(&path_str)
                        {
                            path_str = remapped;
                        }
                        if !path_str.contains('<')
                            && !path_is_current_struct_assoc_projection
                            && let Some(recovered) =
                                self.recover_omitted_local_generic_type_args(&tp.path, &path_str)
                        {
                            return self
                                .maybe_prefix_typename_for_dependent_type_path(tp, recovered);
                        }
                        return path_str;
                    }
                }

                // Special case: Box<dyn Trait> → pro::proxy<TraitFacade> or rusty::Function for Fn traits
                if let Some(last_seg) = tp.path.segments.last() {
                    let seg_name = last_seg.ident.to_string();
                    if seg_name == "Box" {
                        if let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments {
                            if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                if Self::type_is_primitive_str_path(inner_ty) {
                                    // `Box<str>` is owned string storage in Rust.
                                    // Keep ownership in C++ using `rusty::String`.
                                    return "rusty::Box<rusty::String>".to_string();
                                }
                            }
                            if let Some(syn::GenericArgument::Type(syn::Type::TraitObject(to))) =
                                args.args.first()
                            {
                                // Check for Fn → rusty::Function
                                if let Some(syn::TypeParamBound::Trait(tb)) = to.bounds.first() {
                                    if let Some(fn_type) = self.try_map_fn_trait_boxed(tb) {
                                        return fn_type;
                                    }
                                }
                                if self.module_name.is_some() {
                                    return "void*".to_string();
                                }
                                // Collect all trait names for multi-bound
                                let trait_paths: Vec<&syn::Path> = to
                                    .bounds
                                    .iter()
                                    .filter_map(|b| match b {
                                        syn::TypeParamBound::Trait(tb) => Some(&tb.path),
                                        _ => None,
                                    })
                                    .collect();
                                let trait_names: Vec<String> = trait_paths
                                    .iter()
                                    .filter_map(|p| p.segments.last().map(|s| s.ident.to_string()))
                                    .collect();
                                if !trait_names.is_empty() {
                                    if trait_paths
                                        .iter()
                                        .any(|p| facade_name_for_trait_path(p).is_none())
                                    {
                                        return "void*".to_string();
                                    }
                                    // Interface+adapter (§ 3.2.9): Box<dyn T> -> rusty::Box<T>
                                    // (T is the abstract interface class). For multi-bound
                                    // `Box<dyn A + B>`, synthesize a combined interface
                                    // `AAndB : public A, public B` (emitted at end-of-file)
                                    // and use it as the inner type.
                                    if trait_names.len() == 1 {
                                        // Include generic args, e.g.
                                        // Box<dyn Container<i32>> -> rusty::Box<Container<int32_t>>.
                                        let trait_cpp =
                                            self.interface_trait_cpp_name(trait_paths[0]);
                                        return format!("rusty::Box<{}>", trait_cpp);
                                    }
                                    let mut sorted = trait_names.clone();
                                    sorted.sort();
                                    let combined =
                                        self.register_and_synthesize_dyn_multi_name(sorted);
                                    return format!("rusty::Box<{}>", combined);
                                }
                            }
                        }
                    }
                }

                // Special case: Vec → rusty::Vec
                // This handles both Vec and Vec<T> by rewriting the base type.
                if let Some(last_seg) = tp.path.segments.last() {
                    let seg_name = last_seg.ident.to_string();
                    if seg_name == "Vec" {
                        // Rewrite Vec to rusty::Vec, preserving any generic arguments
                        if let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments {
                            let generic_args: Vec<String> = args
                                .args
                                .iter()
                                .filter_map(|arg| match arg {
                                    syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                                    syn::GenericArgument::Const(c) => {
                                        Some(self.emit_expr_to_string(c))
                                    }
                                    _ => None,
                                })
                                .collect();
                            return format!("rusty::Vec<{}>", generic_args.join(", "));
                        }
                        // Vec without generic args
                        return "rusty::Vec".to_string();
                    }
                }

                // Check if the last segment has generic arguments
                if let Some(last_seg) = tp.path.segments.last() {
                    if let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments {
                        if self.should_elide_in_scope_local_alias_type_args(&tp.path, args) {
                            return self.maybe_prefix_typename_for_dependent_type_path(
                                tp,
                                path_str.clone(),
                            );
                        }
                        let joined_no_args = tp
                            .path
                            .segments
                            .iter()
                            .map(|seg| seg.ident.to_string())
                            .collect::<Vec<_>>()
                            .join("::");
                        let std_generic_base = types::map_std_type(&joined_no_args).and_then(
                            |(cpp_type, needs_template_args)| {
                                needs_template_args.then_some(cpp_type.to_string())
                            },
                        );
                        if let Some((cpp_type, needs_template_args)) =
                            types::map_std_type(&joined_no_args)
                        {
                            if !needs_template_args {
                                return cpp_type.to_string();
                            }
                        }
                        self.type_arg_nesting.set(self.type_arg_nesting.get() + 1);
                        let mut generic_args: Vec<String> = args
                            .args
                            .iter()
                            .filter_map(|arg| match arg {
                                syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                                syn::GenericArgument::Const(c) => Some(self.emit_expr_to_string(c)),
                                _ => None,
                            })
                            .collect();
                        self.type_arg_nesting.set(self.type_arg_nesting.get() - 1);
                        self.trim_trailing_defaulted_infer_type_args(
                            &tp.path,
                            args,
                            &mut generic_args,
                        );
                        if tp
                            .path
                            .segments
                            .last()
                            .is_some_and(|seg| seg.ident == "Lazy")
                            && generic_args.len() > 1
                        {
                            while generic_args.len() > 1 {
                                let drop_tail = generic_args.last().is_some_and(|last| {
                                    last == "auto"
                                        || last.contains("/* TODO")
                                        || type_string_has_auto_placeholder(last)
                                });
                                if drop_tail {
                                    generic_args.pop();
                                } else {
                                    break;
                                }
                            }
                        }

                        // Rust `slice::Iter<'a, T>` yields immutable references (`&'a T`).
                        // Model that as `Iter<const T>` while keeping `IterMut<'a, T>` as `Iter<T>`.
                        if Self::type_path_matches_slice_iter_family(&tp.path, "Iter")
                            && let Some(elem_ty) = generic_args.first_mut()
                        {
                            let trimmed = elem_ty.trim_start();
                            if !trimmed.starts_with("const ") {
                                *elem_ty = format!("const {}", elem_ty);
                            }
                        }

                        // Box<[T]> maps through `Type::Slice` and would otherwise become
                        // `rusty::Box<std::span<const T>>`, which blocks mutable slice APIs.
                        // Owned boxed slices should carry mutable element spans.
                        if tp
                            .path
                            .segments
                            .last()
                            .is_some_and(|seg| seg.ident == "Box")
                            && let Some(inner) = generic_args.first_mut()
                        {
                            let trimmed = inner.trim();
                            if let Some(rest) = trimmed.strip_prefix("std::span<const ") {
                                *inner = format!("std::span<{}", rest);
                            }
                        }

                        if !generic_args.is_empty() {
                            if tp
                                .path
                                .segments
                                .last()
                                .is_some_and(|seg| seg.ident == "Zip")
                                && generic_args.len() == 2
                                && (!self.local_declared_types.contains("Zip")
                                    || !self.local_declared_type_has_matching_arity("Zip", 2))
                            {
                                return format!(
                                    "decltype(rusty::zip(std::declval<{}>(), std::declval<{}>()))",
                                    generic_args[0], generic_args[1]
                                );
                            }
                            // Reuse path_str so single-segment remaps (e.g. IterEither →
                            // iterator::IterEither) are preserved for generic type paths.
                            let mut base = path_str.clone();
                            if Self::type_path_matches_slice_iter_family(&tp.path, "Iter")
                                || Self::type_path_matches_slice_iter_family(&tp.path, "IterMut")
                            {
                                base = "rusty::slice_iter::Iter".to_string();
                            }
                            if let Some(std_base) = &std_generic_base
                                && self
                                    .declared_type_param_arity_for_owner_cpp_path(&base)
                                    .is_some_and(|arity| arity != generic_args.len())
                            {
                                base = std_base.clone();
                            }
                            if tp
                                .path
                                .segments
                                .last()
                                .is_some_and(|seg| seg.ident == "Result")
                                && generic_args.len() == 2
                                && self
                                    .declared_type_param_arity_for_owner_cpp_path(&base)
                                    .is_some_and(|arity| arity == 1)
                            {
                                base = "rusty::Result".to_string();
                            }
                            if tp.path.segments.len() == 1 {
                                let local_name = tp.path.segments[0].ident.to_string();
                                if self.is_local_type_name_in_scope(&local_name)
                                    || self.local_declared_types.contains(&local_name)
                                    || self.current_owner_module_declares_type_name(&local_name)
                                {
                                    if !self.current_struct_assoc_alias_exists(&local_name)
                                        && !path_str.contains("::")
                                    {
                                        base = self
                                            .current_named_module_root_type_cpp_name(&local_name)
                                            .unwrap_or_else(|| escape_cpp_keyword(&local_name));
                                    }
                                }
                            }
                            if self.current_struct.is_some() && base.starts_with("Self::") {
                                base = base.trim_start_matches("Self::").to_string();
                            }
                            if !base.contains("::")
                                && self.should_elide_shadowed_current_struct_local_type_args(
                                    &tp.path, args,
                                )
                            {
                                return self
                                    .maybe_prefix_typename_for_dependent_type_path(tp, base);
                            }
                            if tp.qself.is_none() && tp.path.segments.len() >= 2 {
                                let first = tp
                                    .path
                                    .segments
                                    .first()
                                    .map(|seg| seg.ident.to_string())
                                    .unwrap_or_default();
                                let current_struct_is_generic = self
                                    .current_struct
                                    .as_ref()
                                    .is_some_and(|name| name == &first)
                                    && self.declared_type_params.iter().any(|(key, params)| {
                                        !params.is_empty()
                                            && (key == &first
                                                || key.ends_with(&format!("::{}", first)))
                                    });
                                let dependent_owner = first == "Self"
                                    || self.is_type_param_in_scope(&first)
                                    || current_struct_is_generic
                                    || (first.len() == 1
                                        && first.chars().all(|ch| ch.is_ascii_uppercase()));
                                if dependent_owner
                                    && !base.contains("::template ")
                                    && let Some((owner, member)) = base.rsplit_once("::")
                                {
                                    base = format!("{}::template {}", owner, member);
                                }
                            }
                            // itertools' `enum EitherOrBoth<A, B = A>` carries a
                            // defaulted second type parameter, but the generated C++
                            // template declares both params with no default, so a
                            // single-arg `EitherOrBoth<T>` (the common `MergeJoinBy`
                            // shape where both sides share an item type) fails with
                            // "too few template arguments". Rust's default is `B = A`,
                            // so duplicating the sole argument reproduces it exactly.
                            if generic_args.len() == 1 && base.ends_with("EitherOrBoth") {
                                generic_args.push(generic_args[0].clone());
                            }
                            return self.maybe_prefix_typename_for_dependent_type_path(
                                tp,
                                format!("{}<{}>", base, generic_args.join(", ")),
                            );
                        }
                    }
                }

                if !path_str.contains('<') && !path_is_current_struct_assoc_projection {
                    if let Some(recovered) =
                        self.recover_omitted_local_generic_type_args(&tp.path, &path_str)
                    {
                        return self.maybe_prefix_typename_for_dependent_type_path(tp, recovered);
                    }
                }
                if let Some(remapped) = self.remap_forward_decl_qualified_type_path(&path_str) {
                    path_str = remapped;
                }
                if !path_str.contains("::")
                    && let Some(mapped_root_type) =
                        self.current_named_module_root_type_cpp_name(&path_str)
                {
                    path_str = mapped_root_type;
                }

                if path_str.contains("::") {
                    if let Some(reexport_target) =
                        self.resolve_type_reexport_path_via_scope_binding(&path_str)
                    {
                        path_str = reexport_target;
                    }
                    path_str =
                        self.maybe_force_global_for_shadowed_module_root_in_type_path(&path_str);
                    return self.maybe_prefix_typename_for_dependent_type_path(tp, path_str);
                }
                path_str
            }
            syn::Type::Reference(r) => {
                // Special case: &str → std::string_view (not const std::string_view&)
                if Self::type_is_primitive_str_path(r.elem.as_ref()) {
                    return "std::string_view".to_string();
                }
                // Special case: slice references map to span-by-value.
                // `&[T]` -> `std::span<const T>`, `&mut [T]` -> `std::span<T>`.
                if let syn::Type::Slice(s) = r.elem.as_ref() {
                    let elem = self.map_type(&s.elem);
                    if r.mutability.is_some() {
                        return format!("std::span<{}>", elem);
                    }
                    return format!("std::span<const {}>", elem);
                }
                // Special case: &dyn Trait → pro::proxy_view or std::function for Fn traits
                // Special case: &dyn Trait → pro::proxy_view or std::function for Fn traits
                if let syn::Type::TraitObject(to) = r.elem.as_ref() {
                    if r.mutability.is_none() && Self::trait_object_is_fmt_display_only(to) {
                        return "rusty::fmt::DisplayRef".to_string();
                    }
                    // Check for Fn first
                    if let Some(syn::TypeParamBound::Trait(tb)) = to.bounds.first() {
                        if let Some(fn_type) = self.try_map_fn_trait(tb) {
                            return format!("const {}&", fn_type);
                        }
                    }
                    if self.module_name.is_some() {
                        if r.mutability.is_some() {
                            return "void*".to_string();
                        }
                        return "const void*".to_string();
                    }
                    // Collect all trait names for multi-bound
                    let trait_paths: Vec<&syn::Path> = to
                        .bounds
                        .iter()
                        .filter_map(|b| match b {
                            syn::TypeParamBound::Trait(tb) => Some(&tb.path),
                            _ => None,
                        })
                        .collect();
                    let trait_names: Vec<String> = trait_paths
                        .iter()
                        .filter_map(|p| p.segments.last().map(|s| s.ident.to_string()))
                        .collect();
                    if !trait_names.is_empty() {
                        if trait_paths
                            .iter()
                            .any(|p| facade_name_for_trait_path(p).is_none())
                        {
                            if r.mutability.is_some() {
                                return "void*".to_string();
                            }
                            return "const void*".to_string();
                        }
                        // Interface+adapter (§ 3.2.9):
                        //   &dyn T          -> const T&
                        //   &mut dyn T      -> T&
                        //   &(dyn A + B)    -> const AAndB& (synthesized combined interface)
                        //   &mut (dyn A+B)  -> AAndB&
                        if trait_names.len() == 1 {
                            let trait_cpp = self.interface_trait_cpp_name(trait_paths[0]);
                            if r.mutability.is_some() {
                                return format!("{}&", trait_cpp);
                            }
                            return format!("const {}&", trait_cpp);
                        }
                        let mut sorted = trait_names.clone();
                        sorted.sort();
                        let combined = self.register_and_synthesize_dyn_multi_name(sorted);
                        if r.mutability.is_some() {
                            return format!("{}&", combined);
                        }
                        return format!("const {}&", combined);
                    }
                }
                let inner = self.map_type(&r.elem);
                // Nested Rust references (`&&T`, `& &mut T`) should not produce
                // C++ reference-of-reference spellings like `const const T&&`.
                if inner.trim_end().ends_with('&') {
                    if r.mutability.is_some() {
                        let base = inner.trim_end().trim_end_matches('&').trim_end();
                        return format!("{}&", base);
                    }
                    return inner;
                }
                if r.mutability.is_some() {
                    format!("{}&", inner)
                } else {
                    format!("const {}&", inner)
                }
            }
            syn::Type::Ptr(p) => {
                let inner = {
                    let elem_ty = self.peel_paren_group_type(&p.elem);
                    if let syn::Type::Slice(slice_ty) = elem_ty {
                        let elem = self.map_type(&slice_ty.elem);
                        if p.mutability.is_some() {
                            format!("std::span<{}>", elem)
                        } else {
                            format!("std::span<const {}>", elem)
                        }
                    } else {
                        self.map_type(&p.elem)
                    }
                };
                let needs_pointer_trait_hardening =
                    inner.ends_with('&') || self.type_references_in_scope_type_param(&p.elem);
                let needs_assoc_pointer_hardening = self.type_contains_dependent_assoc(&p.elem)
                    || self.type_references_current_struct_assoc(&p.elem);
                if needs_pointer_trait_hardening {
                    if p.mutability.is_some() {
                        return format!("std::add_pointer_t<{}>", inner);
                    }
                    return format!("std::add_pointer_t<std::add_const_t<{}>>", inner);
                }
                if needs_assoc_pointer_hardening {
                    if p.mutability.is_some() {
                        return format!("std::add_pointer_t<{}>", inner);
                    }
                    return format!("std::add_pointer_t<std::add_const_t<{}>>", inner);
                }
                if p.mutability.is_some() {
                    format!("{}*", inner)
                } else {
                    format!("const {}*", inner)
                }
            }
            syn::Type::Tuple(t) => {
                if t.elems.is_empty() {
                    "std::tuple<>".to_string()
                } else {
                    let elems: Vec<String> = t.elems.iter().map(|e| self.map_type(e)).collect();
                    format!("std::tuple<{}>", elems.join(", "))
                }
            }
            syn::Type::Array(a) => {
                let elem = self.map_array_element_type(&a.elem);
                let len = self.emit_expr_to_string(&a.len);
                if self.should_sanitize_array_capacity_expr(&a.len, &len) {
                    format!(
                        "std::array<{}, rusty::sanitize_array_capacity<{}>()>",
                        elem, len
                    )
                } else {
                    format!("std::array<{}, {}>", elem, len)
                }
            }
            syn::Type::Slice(s) => {
                let elem = self.map_type(&s.elem);
                format!("std::span<const {}>", elem)
            }
            syn::Type::Never(_) => "[[noreturn]] void".to_string(),
            syn::Type::Infer(_) => "auto".to_string(),
            syn::Type::TraitObject(to) => {
                // Check for Fn traits first (single bound)
                if let Some(first) = to.bounds.first() {
                    if let syn::TypeParamBound::Trait(tb) = first {
                        if let Some(fn_type) = self.try_map_fn_trait(tb) {
                            return fn_type;
                        }
                    }
                }
                if self.module_name.is_some() {
                    return "void*".to_string();
                }
                // Collect all trait names
                let trait_paths: Vec<&syn::Path> = to
                    .bounds
                    .iter()
                    .filter_map(|b| match b {
                        syn::TypeParamBound::Trait(tb) => Some(&tb.path),
                        _ => None,
                    })
                    .collect();
                let trait_names: Vec<String> = trait_paths
                    .iter()
                    .filter_map(|p| p.segments.last().map(|s| s.ident.to_string()))
                    .collect();
                if trait_names.len() == 1 {
                    if trait_paths
                        .iter()
                        .any(|p| facade_name_for_trait_path(p).is_none())
                    {
                        return "void*".to_string();
                    }
                    // Interface+adapter (§ 3.2.9): bare `dyn T` (no outer reference)
                    // is used inside generic args like Rc<dyn T> / Arc<dyn T>, where
                    // the inner spelling should be just the interface class name
                    // (with the trait's own generic args, if any).
                    self.interface_trait_cpp_name(trait_paths[0])
                } else if trait_names.len() > 1 {
                    if trait_paths
                        .iter()
                        .any(|p| facade_name_for_trait_path(p).is_none())
                    {
                        return "void*".to_string();
                    }
                    // Bare `dyn A + B` (no outer reference) used inside generic args
                    // like Rc<dyn A + B>: synthesize a combined Interface and use it.
                    let mut sorted = trait_names.clone();
                    sorted.sort();
                    self.register_and_synthesize_dyn_multi_name(sorted)
                } else {
                    "/* TODO: complex trait object */".to_string()
                }
            }
            syn::Type::ImplTrait(it) => {
                // In module/argument position, prefer `auto&&`/`const auto&` for
                // Fn trait bounds to allow generic lambdas with auto params.
                // In non-argument position (return types), use concrete
                // rusty::Function wrapper.
                let in_argument_position =
                    self.module_name.is_some() && self.type_arg_nesting.get() == 0;
                if !in_argument_position {
                    if let Some(first) = it.bounds.first() {
                        if let syn::TypeParamBound::Trait(tb) = first {
                            if let Some(fn_type) = self.try_map_fn_trait(tb) {
                                return fn_type;
                            }
                        }
                    }
                }
                // In argument position, `impl Trait` is equivalent to a template parameter.
                // Use C++20 abbreviated function template with `const auto&` to avoid
                // copying non-copyable types.  Rust's `impl Trait` params accept
                // both owned values and references; C++ `const auto&` handles both
                // via reference binding / lifetime extension.
                // However, `auto` cannot appear inside type arguments like
                // `SafeFn<uint64_t(auto)>`.  When inside a generic argument
                // context, fall through to the facade/concept path instead.
                if self.module_name.is_some() && self.type_arg_nesting.get() == 0 {
                    // Check if any bound is a mutable trait (e.g., fmt::Write,
                    // io::Write) — these need `auto&` not `const auto&`.
                    let has_mutable_trait = it.bounds.iter().any(|b| {
                        if let syn::TypeParamBound::Trait(tb) = b {
                            let last = tb.path.segments.last().map(|s| s.ident.to_string());
                            matches!(last.as_deref(), Some("Write"))
                        } else {
                            false
                        }
                    });
                    return if has_mutable_trait {
                        // Use forwarding reference to accept both lvalues and
                        // std::move'd rvalues (Rust passes impl Write by value,
                        // so the transpiler may emit std::move on last use).
                        "auto&&".to_string()
                    } else {
                        "const auto&".to_string()
                    };
                }
                // Collect all trait names
                let trait_paths: Vec<&syn::Path> = it
                    .bounds
                    .iter()
                    .filter_map(|b| match b {
                        syn::TypeParamBound::Trait(tb) => Some(&tb.path),
                        _ => None,
                    })
                    .collect();
                let trait_names: Vec<String> = trait_paths
                    .iter()
                    .filter_map(|p| p.segments.last().map(|s| s.ident.to_string()))
                    .collect();
                if trait_names.len() == 1 {
                    if trait_paths
                        .iter()
                        .any(|p| facade_name_for_trait_path(p).is_none())
                    {
                        return "void*".to_string();
                    }
                    // Interface+adapter (§ 3.2.9): `impl Trait` in non-arg
                    // position maps to the interface class spelling.
                    self.interface_trait_cpp_name(trait_paths[0])
                } else if trait_names.len() > 1 {
                    if trait_paths
                        .iter()
                        .any(|p| facade_name_for_trait_path(p).is_none())
                    {
                        return "void*".to_string();
                    }
                    let mut sorted = trait_names.clone();
                    sorted.sort();
                    self.register_and_synthesize_dyn_multi_name(sorted)
                } else {
                    "auto".to_string()
                }
            }
            syn::Type::BareFn(bf) => {
                // fn(A, B) -> C → rusty::SafeFn<C(A, B)>
                // unsafe fn(A, B) -> C → rusty::UnsafeFn<C(A, B)>
                let param_types: Vec<String> =
                    bf.inputs.iter().map(|arg| self.map_type(&arg.ty)).collect();
                let return_type = match &bf.output {
                    syn::ReturnType::Default => "void".to_string(),
                    syn::ReturnType::Type(_, ty) => self.map_type(ty),
                };
                let wrapper = if bf.unsafety.is_some() {
                    "rusty::UnsafeFn"
                } else {
                    "rusty::SafeFn"
                };
                format!("{}<{}({})>", wrapper, return_type, param_types.join(", "))
            }
            syn::Type::Paren(p) => self.map_type(&p.elem),
            _ => "/* TODO: type */".to_string(),
        }
    }

    pub(super) fn map_array_element_type(&self, elem_ty: &syn::Type) -> String {
        match elem_ty {
            syn::Type::Reference(r) => {
                let referent_cpp = self.map_type(&r.elem);
                let normalized_referent_cpp = if referent_cpp.ends_with('&') {
                    format!("std::remove_reference_t<{}>", referent_cpp)
                } else {
                    referent_cpp
                };
                if r.mutability.is_none()
                    && self.array_reference_element_decays_to_value(&normalized_referent_cpp)
                {
                    return normalized_referent_cpp;
                }
                let wrapper_target = if r.mutability.is_some() {
                    normalized_referent_cpp
                } else {
                    format!("std::add_const_t<{}>", normalized_referent_cpp)
                };
                format!("std::reference_wrapper<{}>", wrapper_target)
            }
            syn::Type::Paren(p) => self.map_array_element_type(&p.elem),
            syn::Type::Group(g) => self.map_array_element_type(&g.elem),
            _ => self.map_type(elem_ty),
        }
    }

    pub(super) fn type_head_is_explicit_crate_path(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(tp) => {
                tp.qself.is_none()
                    && tp.path.segments.len() > 1
                    && tp
                        .path
                        .segments
                        .first()
                        .is_some_and(|seg| seg.ident == "crate")
            }
            syn::Type::Reference(r) => self.type_head_is_explicit_crate_path(&r.elem),
            syn::Type::Ptr(p) => self.type_head_is_explicit_crate_path(&p.elem),
            syn::Type::Paren(p) => self.type_head_is_explicit_crate_path(&p.elem),
            syn::Type::Group(g) => self.type_head_is_explicit_crate_path(&g.elem),
            _ => false,
        }
    }

    pub(super) fn map_callable_surface_type(&self, ty: &syn::Type) -> String {
        let mapped = self.map_type(ty);
        let force_global = self.type_head_is_explicit_crate_path(ty);
        self.disambiguate_callable_surface_type_path(mapped, force_global)
    }

    pub(super) fn map_reference_type_to_pointer_cpp_type(&self, ty: &syn::Type) -> Option<String> {
        let ty = self.peel_paren_group_type(ty);
        let syn::Type::Reference(reference) = ty else {
            return None;
        };
        let inner_cpp = self.map_type(&reference.elem);
        if reference.mutability.is_some() {
            return Some(format!("{}*", inner_cpp));
        }
        let const_inner_cpp = if inner_cpp.trim_start().starts_with("const ") {
            inner_cpp
        } else {
            format!("const {}", inner_cpp)
        };
        Some(format!("{}*", const_inner_cpp))
    }

    /// For a reference binding that will become a pointer, determine the pointer type.
    /// `let mut r = &x` where x: T → `const T*`
    /// `let mut r = &mut x` where x: T → `T*`
    /// `let mut r: &T = &x` → `const T*`
    /// `let mut r: &mut T = &mut x` → `T*`
    pub(super) fn map_ref_as_pointer_type(&self, local: &syn::Local, init_expr: &syn::Expr) -> String {
        // If there's a type annotation, use it
        if let Some(ty) = get_local_type(local) {
            if let syn::Type::Reference(r) = ty {
                let inner = self.map_type(&r.elem);
                return if r.mutability.is_some() {
                    format!("{}*", inner)
                } else {
                    format!("const {}*", inner)
                };
            }
        }
        // Infer from the init expression
        if let syn::Expr::Reference(r) = init_expr {
            if r.mutability.is_some() {
                "auto*".to_string()
            } else {
                "const auto*".to_string()
            }
        } else {
            "auto*".to_string()
        }
    }

    /// For a non-rebound reference binding, determine the reference type.
    /// `let r = &x` → `auto&` (the const qualifier is added separately)
    /// `let r = &mut x` → `auto&` (mutable, no const)
    pub(super) fn map_ref_as_ref_type(&self, local: &syn::Local, init_expr: &syn::Expr) -> String {
        if let Some(ty) = get_local_type(local) {
            if let syn::Type::Reference(r) = ty {
                let inner = self.map_type(&r.elem);
                return if r.mutability.is_some() {
                    format!("{}&", inner)
                } else {
                    format!("const {}&", inner)
                };
            }
        }
        if let syn::Expr::Reference(r) = init_expr {
            if r.mutability.is_some() {
                "auto&".to_string()
            } else {
                "auto&".to_string()
            }
        } else {
            "auto&".to_string()
        }
    }

    pub(super) fn type_param_has_trait_bound(&self, type_param: &str, trait_name: &str) -> bool {
        let suffix = format!("::{}", trait_name);
        self.trait_bound_type_param_scopes
            .iter()
            .rev()
            .any(|scope| {
                scope.iter().any(|(bound_trait, bound_param)| {
                    bound_param == type_param
                        && (bound_trait == trait_name || bound_trait.ends_with(&suffix))
                })
            })
    }

    pub(super) fn type_param_has_float_trait_bound(&self, type_param: &str) -> bool {
        self.type_param_has_trait_bound(type_param, "FloatTraits")
    }

    pub(super) fn type_param_has_uint_trait_bound(&self, type_param: &str) -> bool {
        self.type_param_has_trait_bound(type_param, "UInt")
    }

    pub(super) fn type_param_static_conversion_owner_path(&self, path: &syn::Path) -> Option<syn::Path> {
        if path.segments.len() < 2 {
            return None;
        }
        let method = path.segments.last()?.ident.to_string();
        if !matches!(method.as_str(), "from" | "truncate" | "enlarge") {
            return None;
        }
        let owner_path = Self::path_without_last_segment(path)?;
        let owner_segments = Self::path_segments_as_strings(&owner_path);
        let owner_supported = match owner_segments.as_slice() {
            [owner] => {
                self.is_type_param_in_scope(owner) && self.type_param_has_uint_trait_bound(owner)
            }
            _ if method == "from" => self.path_segments_name_float_trait_sig_type(&owner_segments),
            _ => false,
        };
        owner_supported.then_some(owner_path)
    }

    pub(super) fn type_references_in_scope_type_param(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(tp) => {
                if tp
                    .path
                    .segments
                    .iter()
                    .any(|seg| self.is_type_param_in_scope(&seg.ident.to_string()))
                {
                    return true;
                }
                tp.path.segments.iter().any(|seg| {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        args.args.iter().any(|arg| match arg {
                            syn::GenericArgument::Type(inner) => {
                                self.type_references_in_scope_type_param(inner)
                            }
                            _ => false,
                        })
                    } else {
                        false
                    }
                })
            }
            syn::Type::Reference(r) => self.type_references_in_scope_type_param(&r.elem),
            syn::Type::Ptr(p) => self.type_references_in_scope_type_param(&p.elem),
            syn::Type::Array(a) => self.type_references_in_scope_type_param(&a.elem),
            syn::Type::Slice(s) => self.type_references_in_scope_type_param(&s.elem),
            syn::Type::Tuple(t) => t
                .elems
                .iter()
                .any(|elem| self.type_references_in_scope_type_param(elem)),
            syn::Type::Paren(p) => self.type_references_in_scope_type_param(&p.elem),
            syn::Type::Group(g) => self.type_references_in_scope_type_param(&g.elem),
            _ => false,
        }
    }

    pub(super) fn type_path_is_fmt_result(path: &syn::Path) -> bool {
        let segments: Vec<String> = path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect();
        segments.len() >= 2
            && segments.last().is_some_and(|seg| seg == "Result")
            && segments.iter().nth_back(1).is_some_and(|seg| seg == "fmt")
    }

    pub(super) fn map_fp_category_path(path: &str) -> Option<String> {
        let normalized = path.trim_start_matches("::");
        for prefix in ["std::num::", "core::num::", "num::"] {
            if let Some(tail) = normalized.strip_prefix(prefix) {
                if tail == "FpCategory" || tail.starts_with("FpCategory_") {
                    return Some(format!("rusty::num::{}", tail));
                }
                if let Some(variant) = tail.strip_prefix("FpCategory::") {
                    return Some(format!("rusty::num::FpCategory_{}", variant));
                }
            }
        }
        None
    }

    pub(super) fn type_path_requires_typename_prefix(&self, tp: &syn::TypePath) -> bool {
        if let Some(qself) = &tp.qself
            && (self.type_mentions_in_scope_type_param(&qself.ty)
                || self.type_contains_dependent_assoc(&qself.ty)
                || self.type_references_current_struct_assoc(&qself.ty))
        {
            return true;
        }

        let segment_count = tp.path.segments.len();
        if tp.qself.is_none() && segment_count >= 2 {
            if let Some(first) = tp.path.segments.first().map(|s| s.ident.to_string()) {
                let current_struct_is_generic = self
                    .current_struct
                    .as_ref()
                    .is_some_and(|name| name == &first)
                    && self.declared_type_params.iter().any(|(key, params)| {
                        !params.is_empty()
                            && (key == &first || key.ends_with(&format!("::{}", first)))
                    });
                if first == "Self"
                    || self.is_type_param_in_scope(&first)
                    || current_struct_is_generic
                {
                    return true;
                }
            }

            // Handles dependent qualified names whose first segment is a namespace
            // (e.g. `rusty::detail::associated_item_t<I>::IntoIter`).
            for seg in tp
                .path
                .segments
                .iter()
                .take(segment_count.saturating_sub(1))
            {
                if self.path_arguments_contain_dependent_type_param(&seg.arguments) {
                    return true;
                }
            }
        }

        false
    }

    pub(super) fn type_contains_dependent_assoc(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(tp) => {
                if tp.qself.is_none() && tp.path.segments.len() >= 2 {
                    let first = tp.path.segments.first().map(|s| s.ident.to_string());
                    if first
                        .as_ref()
                        .is_some_and(|name| name == "Self" || self.is_type_param_in_scope(name))
                    {
                        return true;
                    }
                }

                if let Some(qself) = &tp.qself {
                    if self.type_mentions_in_scope_type_param(&qself.ty) {
                        return true;
                    }
                    if self.type_contains_dependent_assoc(&qself.ty) {
                        return true;
                    }
                }

                tp.path.segments.iter().any(|seg| {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        args.args.iter().any(|arg| {
                            if let syn::GenericArgument::Type(inner_ty) = arg {
                                self.type_contains_dependent_assoc(inner_ty)
                            } else {
                                false
                            }
                        })
                    } else {
                        false
                    }
                })
            }
            syn::Type::Reference(r) => self.type_contains_dependent_assoc(&r.elem),
            syn::Type::Ptr(p) => self.type_contains_dependent_assoc(&p.elem),
            syn::Type::Slice(s) => self.type_contains_dependent_assoc(&s.elem),
            syn::Type::Array(a) => self.type_contains_dependent_assoc(&a.elem),
            syn::Type::Paren(p) => self.type_contains_dependent_assoc(&p.elem),
            syn::Type::Group(g) => self.type_contains_dependent_assoc(&g.elem),
            syn::Type::Tuple(tup) => tup
                .elems
                .iter()
                .any(|elem| self.type_contains_dependent_assoc(elem)),
            _ => false,
        }
    }

    pub(super) fn type_references_current_struct_assoc(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(tp) => {
                if tp.qself.is_none() && tp.path.segments.len() >= 2 {
                    let first = tp.path.segments.first().map(|s| s.ident.to_string());
                    if let (Some(struct_name), Some(first_seg)) =
                        (self.current_struct.as_ref(), first.as_ref())
                    {
                        if first_seg == struct_name {
                            return true;
                        }
                    }
                }

                if let Some(qself) = &tp.qself {
                    let qself_base = self.peel_reference_paren_group_type(&qself.ty);
                    if let syn::Type::Path(base_tp) = qself_base
                        && base_tp.qself.is_none()
                        && base_tp.path.segments.len() == 1
                        && let Some(struct_name) = self.current_struct.as_ref()
                    {
                        let base_name = base_tp.path.segments[0].ident.to_string();
                        if base_name == "Self" || base_name == *struct_name {
                            return true;
                        }
                    }
                    if self.type_references_current_struct_assoc(&qself.ty) {
                        return true;
                    }
                }

                tp.path.segments.iter().any(|seg| {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        args.args.iter().any(|arg| {
                            if let syn::GenericArgument::Type(inner_ty) = arg {
                                self.type_references_current_struct_assoc(inner_ty)
                            } else {
                                false
                            }
                        })
                    } else {
                        false
                    }
                })
            }
            syn::Type::Reference(r) => self.type_references_current_struct_assoc(&r.elem),
            syn::Type::Ptr(p) => self.type_references_current_struct_assoc(&p.elem),
            syn::Type::Slice(s) => self.type_references_current_struct_assoc(&s.elem),
            syn::Type::Array(a) => self.type_references_current_struct_assoc(&a.elem),
            syn::Type::Paren(p) => self.type_references_current_struct_assoc(&p.elem),
            syn::Type::Group(g) => self.type_references_current_struct_assoc(&g.elem),
            syn::Type::Tuple(tup) => tup
                .elems
                .iter()
                .any(|elem| self.type_references_current_struct_assoc(elem)),
            _ => false,
        }
    }

    pub(super) fn type_mentions_in_scope_type_param(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(tp) => {
                if tp.qself.is_none()
                    && tp.path.segments.len() == 1
                    && tp.path.segments.first().is_some_and(|seg| {
                        let name = seg.ident.to_string();
                        name == "Self" || self.is_type_param_in_scope(&name)
                    })
                {
                    return true;
                }

                if let Some(qself) = &tp.qself {
                    if self.type_mentions_in_scope_type_param(&qself.ty) {
                        return true;
                    }
                }

                tp.path.segments.iter().any(|seg| {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        args.args.iter().any(|arg| {
                            if let syn::GenericArgument::Type(inner_ty) = arg {
                                self.type_mentions_in_scope_type_param(inner_ty)
                            } else {
                                false
                            }
                        })
                    } else {
                        false
                    }
                })
            }
            syn::Type::Reference(r) => self.type_mentions_in_scope_type_param(&r.elem),
            syn::Type::Ptr(p) => self.type_mentions_in_scope_type_param(&p.elem),
            syn::Type::Slice(s) => self.type_mentions_in_scope_type_param(&s.elem),
            syn::Type::Array(a) => self.type_mentions_in_scope_type_param(&a.elem),
            syn::Type::Paren(p) => self.type_mentions_in_scope_type_param(&p.elem),
            syn::Type::Group(g) => self.type_mentions_in_scope_type_param(&g.elem),
            syn::Type::Tuple(tup) => tup
                .elems
                .iter()
                .any(|elem| self.type_mentions_in_scope_type_param(elem)),
            _ => false,
        }
    }

    pub(super) fn map_return_type(&self, output: &syn::ReturnType) -> String {
        match output {
            syn::ReturnType::Default => "void".to_string(),
            syn::ReturnType::Type(_, ty) => {
                if self.is_explicit_unit_type(ty) {
                    "void".to_string()
                } else {
                    self.map_type(ty)
                }
            }
        }
    }

    pub(super) fn type_mentions_named_type_param(&self, ty: &syn::Type, name: &str) -> bool {
        match ty {
            syn::Type::Path(tp) => {
                if tp.qself.is_none()
                    && tp.path.segments.len() == 1
                    && tp.path.segments[0].ident == name
                {
                    return true;
                }
                if let Some(qself) = &tp.qself {
                    if self.type_mentions_named_type_param(&qself.ty, name) {
                        return true;
                    }
                }
                tp.path.segments.iter().any(|seg| {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        args.args.iter().any(|arg| match arg {
                            syn::GenericArgument::Type(inner) => {
                                self.type_mentions_named_type_param(inner, name)
                            }
                            _ => false,
                        })
                    } else {
                        false
                    }
                })
            }
            syn::Type::Reference(r) => self.type_mentions_named_type_param(&r.elem, name),
            syn::Type::Ptr(p) => self.type_mentions_named_type_param(&p.elem, name),
            syn::Type::Slice(s) => self.type_mentions_named_type_param(&s.elem, name),
            syn::Type::Array(a) => self.type_mentions_named_type_param(&a.elem, name),
            syn::Type::Tuple(tup) => tup
                .elems
                .iter()
                .any(|elem| self.type_mentions_named_type_param(elem, name)),
            syn::Type::Paren(p) => self.type_mentions_named_type_param(&p.elem, name),
            syn::Type::Group(g) => self.type_mentions_named_type_param(&g.elem, name),
            _ => false,
        }
    }

    pub(super) fn map_fn_params(
        &self,
        inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
    ) -> String {
        let params: Vec<String> = inputs
            .iter()
            .map(|arg| match arg {
                syn::FnArg::Typed(pat_type) => {
                    let ty = self.resolve_param_cpp_type(&pat_type.ty);
                    let name = match pat_type.pat.as_ref() {
                        syn::Pat::Ident(pi) => escape_cpp_keyword(&pi.ident.to_string()),
                        _ => "_".to_string(),
                    };
                    format!("{} {}", ty, name)
                }
                syn::FnArg::Receiver(_) => "/* self */".to_string(),
            })
            .collect();
        params.join(", ")
    }

    pub(super) fn map_fn_param_types(
        &self,
        inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
    ) -> String {
        let params: Vec<String> = inputs
            .iter()
            .filter_map(|arg| match arg {
                syn::FnArg::Typed(pat_type) => Some(self.resolve_param_cpp_type(&pat_type.ty)),
                syn::FnArg::Receiver(_) => None,
            })
            .collect();
        params.join(", ")
    }
}
