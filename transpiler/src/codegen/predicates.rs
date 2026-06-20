use super::*;

impl CodeGen {
    pub(super) fn should_rewrite_by_value_cycle_field_declaration(
        &self,
        owner_type: &str,
        field_name: &str,
    ) -> bool {
        let key = ByValueCycleRewriteFieldKey {
            owner_type: owner_type.to_string(),
            field_name: field_name.to_string(),
        };
        self.auto_cross_module_by_value_rewrite_fields
            .contains(&key)
            || (self.enable_by_value_cycle_breaking_prototype
                && self.by_value_cycle_breaking_rewrite_fields.contains(&key))
    }

    /// Helper: check if node `start` can reach any node in the cycle set via outgoing edges.
    pub(super) fn can_reach_cycle(
        start: usize,
        outgoing: &[HashSet<usize>],
        _sorted_set: &std::collections::HashSet<usize>,
        in_cycle: &[bool],
    ) -> bool {
        let mut visited = vec![false; outgoing.len()];
        let mut stack = vec![start];
        while let Some(pos) = stack.pop() {
            if pos == start {
                continue; // Skip self
            }
            if in_cycle[pos] {
                return true;
            }
            if visited[pos] {
                continue;
            }
            visited[pos] = true;
            if let Some(nexts) = outgoing.get(pos) {
                for &next in nexts {
                    if !visited[next] {
                        stack.push(next);
                    }
                }
            }
        }
        false
    }

    pub(super) fn is_indirection_container_name(ident: &syn::Ident) -> bool {
        matches!(
            ident.to_string().as_str(),
            "Box" | "Rc" | "Arc" | "Weak" | "NonNull" | "Unique" | "Pin" | "AtomicPtr"
        )
    }

    pub(super) fn is_forward_constexpr_literal_expr(expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Lit(_) => true,
            syn::Expr::Tuple(t) => t.elems.iter().all(Self::is_forward_constexpr_literal_expr),
            syn::Expr::Array(a) => a.elems.iter().all(Self::is_forward_constexpr_literal_expr),
            syn::Expr::Paren(p) => Self::is_forward_constexpr_literal_expr(&p.expr),
            syn::Expr::Group(g) => Self::is_forward_constexpr_literal_expr(&g.expr),
            syn::Expr::Unary(u) => Self::is_forward_constexpr_literal_expr(&u.expr),
            syn::Expr::Cast(c) => Self::is_forward_constexpr_literal_expr(&c.expr),
            // Primitive associated constants (`i32::MAX`, `f32::MIN`, `u8::MIN`,
            // etc.) lower to `std::numeric_limits<T>::{max,min,lowest,…}()`,
            // all of which are `constexpr` in the C++14+ standard library.
            // Treat them as forward-emittable so we don't fall back to a
            // redundant `extern const T NAME;` declaration ahead of the
            // `constexpr T NAME = …;` definition.
            syn::Expr::Path(p) => Self::is_primitive_assoc_constexpr_path(&p.path),
            _ => false,
        }
    }

    pub(super) fn is_primitive_assoc_constexpr_path(path: &syn::Path) -> bool {
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        let (primitive, member) = match segments.as_slice() {
            [primitive, member] => (primitive.as_str(), member.as_str()),
            [root, primitive, member] if matches!(root.as_str(), "std" | "core") => {
                (primitive.as_str(), member.as_str())
            }
            [root, primitive_kw, primitive, member]
                if matches!(root.as_str(), "std" | "core") && primitive_kw == "primitive" =>
            {
                (primitive.as_str(), member.as_str())
            }
            _ => return false,
        };
        if types::map_primitive_type(primitive).is_none() {
            return false;
        }
        matches!(
            member,
            "MAX"
                | "MIN"
                | "MIN_POSITIVE"
                | "EPSILON"
                | "NAN"
                | "INFINITY"
                | "NEG_INFINITY"
                | "BITS"
        )
    }

    pub(super) fn should_emit_internal_linkage_function(&self, f: &syn::ItemFn) -> bool {
        self.is_non_root_expanded_test_module() && !matches!(f.vis, syn::Visibility::Public(_))
    }

    pub(super) fn is_non_root_expanded_test_module(&self) -> bool {
        if !self.expanded_libtest_mode {
            return false;
        }
        let (Some(module_name), Some(crate_name)) =
            (self.module_name.as_deref(), self.crate_name.as_deref())
        else {
            return false;
        };
        module_name != crate_name
    }

    pub(super) fn should_force_export_private_root_module_function(&self, f: &syn::ItemFn) -> bool {
        let (Some(module_name), Some(crate_name)) =
            (self.module_name.as_deref(), self.crate_name.as_deref())
        else {
            return false;
        };
        module_name == crate_name
            && self.module_stack.is_empty()
            && !self.inside_hidden_private_module_scope()
            && !matches!(f.vis, syn::Visibility::Public(_))
    }

    pub(super) fn has_forward_decl_items(items: &[syn::Item]) -> bool {
        items.iter().any(|item| match item {
            syn::Item::Struct(_) => true,
            syn::Item::Enum(_) => true,
            syn::Item::Type(_) => true,
            syn::Item::Fn(_) => true,
            syn::Item::Mod(m) => m
                .content
                .as_ref()
                .is_some_and(|(_, nested)| Self::has_forward_decl_items(nested)),
            _ => false,
        })
    }

    pub(super) fn should_strip_forced_global_for_private_alias_root(
        &self,
        scope_key: &str,
        root: &str,
    ) -> bool {
        if !root.ends_with("_private") {
            return false;
        }
        let Some(raw_bound_target) =
            self.resolve_scope_import_binding_target_in_scope_chain(scope_key, root)
        else {
            return false;
        };
        let normalized_root = root.trim_start_matches("::");
        let normalized_target = raw_bound_target.trim().trim_start_matches("::");
        !normalized_target.is_empty() && normalized_target != normalized_root
    }

    pub(super) fn is_known_free_function_path(&self, key: &str) -> bool {
        self.function_arg_pass_styles.contains_key(key)
            || self.function_arg_expected_types.contains_key(key)
            || self.function_type_param_names.contains_key(key)
            || self.function_return_types.contains_key(key)
    }

    pub(super) fn is_pointer_like_autoderef_owner_name(name: &str) -> bool {
        matches!(
            name,
            "Box"
                | "Rc"
                | "Arc"
                | "Ref"
                | "RefMut"
                | "MutexGuard"
                | "RwLockReadGuard"
                | "RwLockWriteGuard"
        )
    }

    pub(super) fn should_insert_move_for_deref_expected_value(
        &self,
        expr: &syn::Expr,
        expected_ty: Option<&syn::Type>,
    ) -> bool {
        let Some(expected_ty) = expected_ty else {
            return false;
        };
        if matches!(
            self.peel_reference_paren_group_type(expected_ty),
            syn::Type::Reference(_) | syn::Type::Ptr(_)
        ) {
            return false;
        }
        let syn::Expr::Unary(unary) = self.peel_paren_group_expr(expr) else {
            return false;
        };
        if !matches!(unary.op, syn::UnOp::Deref(_)) {
            return false;
        }
        let base = self.peel_paren_group_expr(&unary.expr);
        if !self.should_insert_move(base) {
            return false;
        }
        if let Some(base_ty) = self.infer_simple_expr_type(base) {
            match self.peel_paren_group_type(&base_ty) {
                syn::Type::Reference(_) | syn::Type::Ptr(_) => return false,
                _ => {}
            }
        }
        true
    }

    pub(super) fn should_move_reference_binding_for_expected_value(
        &self,
        expr: &syn::Expr,
        expected_ty: Option<&syn::Type>,
    ) -> bool {
        let Some(expected_ty) = expected_ty else {
            return false;
        };
        if matches!(
            self.peel_reference_paren_group_type(expected_ty),
            syn::Type::Reference(_) | syn::Type::Ptr(_)
        ) || self.is_known_scalar_like_type(expected_ty)
        {
            return false;
        }
        let syn::Expr::Path(path) = self.peel_paren_group_expr(expr) else {
            return false;
        };
        if path.path.segments.len() != 1 {
            return false;
        }
        let name = path.path.segments[0].ident.to_string();
        if self.is_pattern_ref_binding_in_scope(&name)
            && !self.is_local_reference_binding_in_scope(&name)
        {
            return true;
        }
        self.lookup_local_binding_type(&name).is_some_and(|ty| {
            matches!(
                self.peel_reference_paren_group_type(&ty),
                syn::Type::Reference(_)
            )
        })
    }

    pub(super) fn should_move_local_binding_for_owned_expected_value(
        &self,
        expr: &syn::Expr,
        expected_ty: Option<&syn::Type>,
    ) -> bool {
        let Some(expected_ty) = expected_ty else {
            return false;
        };
        if matches!(
            self.peel_reference_paren_group_type(expected_ty),
            syn::Type::Reference(_) | syn::Type::Ptr(_)
        ) || self.is_known_scalar_like_type(expected_ty)
        {
            return false;
        }
        let syn::Expr::Path(path) = self.peel_paren_group_expr(expr) else {
            return false;
        };
        if path.path.segments.len() != 1 {
            return false;
        }
        let name = path.path.segments[0].ident.to_string();
        if matches!(
            name.as_str(),
            "Self" | "self" | "true" | "false" | "None" | "Some" | "Ok" | "Err"
        ) || name.starts_with(|c: char| c.is_uppercase())
            || (name.len() > 1 && name.chars().all(|c| c.is_uppercase() || c == '_'))
            || self.is_const_local_binding_in_scope(&name)
            || matches!(name.as_str(), "formatter" | "f")
        {
            return false;
        }
        if let Some(local_ty) = self.lookup_local_binding_type(&name) {
            if matches!(
                self.peel_paren_group_type(&local_ty),
                syn::Type::Reference(_) | syn::Type::Ptr(_)
            ) || self.reference_type_lowers_to_value_cpp(&local_ty)
            {
                return false;
            }
            let mapped_local_ty = self.map_type(&local_ty);
            if mapped_local_ty.contains('&')
                || mapped_local_ty.starts_with("std::span<")
                || mapped_local_ty.starts_with("std::array<")
            {
                return false;
            }
        }
        self.is_local_binding_in_scope(&name) || self.lookup_local_binding_cpp_name(&name).is_some()
    }

    pub(super) fn has_impls_for_type(&self, type_name: &str) -> bool {
        let scoped = self.scoped_type_key(type_name);
        self.impl_blocks.contains_key(&scoped)
            || (scoped != type_name && self.impl_blocks.contains_key(type_name))
    }

    /// Returns true if the item should get a C++ `export` keyword in
    /// module mode.
    ///
    /// C++ modules have a binary export/private split with no analogue
    /// to Rust's intra-crate visibility tiers (`pub(crate)`,
    /// `pub(super)`, `pub(in path)`). Items declared with any of those
    /// restricted-public forms are designed to be visible across
    /// sibling/parent files inside the same Rust crate — which in our
    /// crate-mode lowering means crossing C++ module boundaries.
    /// Without exporting them they're invisible to importers and
    /// referenced types fail name lookup (see the BTreeMap port:
    /// `pub(super) struct NodeRef<...>` in `node.rs` is imported by
    /// `map.rs`, `search.rs`, etc.).
    ///
    /// We therefore treat every restricted-public visibility as
    /// exported in module mode. Truly private items (no visibility
    /// specifier, the implicit private form) remain non-exported.
    pub(super) fn is_exported(&self, vis: &syn::Visibility) -> bool {
        self.module_name.is_some()
            && Self::visibility_is_any_pub(vis)
            && !self.inside_hidden_private_module_scope()
    }

    /// Returns true if visibility is pub and we're in module mode.
    /// `module_depth` is kept for call-site compatibility.
    pub(super) fn is_exported_at_module_depth(&self, vis: &syn::Visibility, module_depth: usize) -> bool {
        let _ = module_depth;
        self.module_name.is_some()
            && Self::visibility_is_any_pub(vis)
            && !self.inside_hidden_private_module_scope()
    }

    pub(super) fn is_force_exported_reexport_target(&self, item_name: &str) -> bool {
        if self.module_name.is_none() {
            return false;
        }
        if self.inside_hidden_private_module_scope() {
            return false;
        }
        let scoped_name = self.scoped_type_key(item_name);
        self.crate_pub_reexport_targets.contains(&scoped_name)
    }

    pub(super) fn should_export_item(&self, vis: &syn::Visibility, item_name: &str) -> bool {
        self.is_exported(vis) || self.is_force_exported_reexport_target(item_name)
    }

    pub(super) fn should_export_item_at_module_depth(
        &self,
        vis: &syn::Visibility,
        module_depth: usize,
        item_name: &str,
    ) -> bool {
        self.is_exported_at_module_depth(vis, module_depth)
            || self.is_force_exported_reexport_target(item_name)
    }

    pub(super) fn should_prefix_named_module_root_type(type_name: &str) -> bool {
        matches!(type_name, "Buffer")
    }

    /// Check if attributes contain `#[test]`.
    pub(super) fn has_test_attr(attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|a| a.path().is_ident("test"))
    }

    /// Check if attributes contain `#[cpp_ctor]`. When set on an
    /// associated function inside an `impl Owner { ... }` block whose
    /// body is a single `Self { field: expr, ... }` (or
    /// `Owner { field: expr, ... }`) literal, the function is emitted as
    /// a real C++ constructor:
    ///
    ///   * declaration: `Owner(args);` (no `static`, no return type)
    ///   * definition:  `Owner::Owner(args) : field1(expr1), ... {}`
    ///
    /// Without the attribute, factory-style `fn new(...) -> Self`
    /// continues to lower to `static Owner Owner::new_(args)`.
    pub(super) fn has_cpp_ctor_attr(attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|a| a.path().is_ident("cpp_ctor"))
    }

    /// `#[cpp_inherit]` on an `impl Trait for Type` opts that impl into
    /// direct C++ inheritance: the concrete `Type` is emitted as
    /// `struct Type : public Trait { ... override ... }` (with a synthesized
    /// fieldwise + move ctor) instead of the default `TraitAdapter<Type>`
    /// wrapper, so existing call sites that upcast `Arc<Type>` /
    /// `shared_ptr<Type>` to the trait base keep compiling. Opt-in only.
    pub(super) fn has_cpp_inherit_attr(attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|a| a.path().is_ident("cpp_inherit"))
    }

    /// True when `type_name`'s simple name was recorded (via a
    /// `#[cpp_inherit]` impl) as using direct-inheritance emission.
    /// Checks both the bare name and the module-scoped key.
    pub(super) fn is_cpp_inherit_type(&self, type_name: &str) -> bool {
        self.cpp_inherit_trait.contains_key(type_name)
            || self
                .cpp_inherit_trait
                .contains_key(&self.scoped_type_key(type_name))
    }

    /// For a `#[cpp_inherit]` type, resolve the absolute C++ base-class
    /// spelling to inherit from (`::ns::Trait`), falling back to the bare
    /// trait name for cross-block inline-rust traits (declared in a sibling
    /// block, so absent from `trait_declared_path_by_short_name`). Returns
    /// `None` for non-cpp_inherit types. Shared by `emit_struct` (the base
    /// clause) and `emit_cpp_ctor` (the base-subobject init prefix).
    pub(super) fn cpp_inherit_base_name(&self, type_name: &str) -> Option<String> {
        let scoped_key = self.scoped_type_key(type_name);
        let trait_short = self
            .cpp_inherit_trait
            .get(type_name)
            .or_else(|| self.cpp_inherit_trait.get(&scoped_key))
            .cloned()?;
        Some(
            match self
                .trait_declared_path_by_short_name
                .get(&trait_short)
                .cloned()
            {
                Some(qualified) => {
                    let escaped = self.escape_and_rename_qualified_name(&qualified);
                    if escaped.contains("::") {
                        format!("::{}", escaped)
                    } else {
                        escaped
                    }
                }
                None => trait_short,
            },
        )
    }

    /// Skip items behind `#[cfg(...)]` when the predicate is known-false in
    /// transpiler mode. Unknown predicates are kept conservatively.
    pub(super) fn should_skip_cfg_attrs(attrs: &[syn::Attribute]) -> bool {
        attrs
            .iter()
            .filter(|a| a.path().is_ident("cfg"))
            .any(|a| matches!(Self::eval_cfg_meta(&a.meta), CfgEval::False))
    }

    pub(super) fn has_cfg_test(attrs: &[syn::Attribute]) -> bool {
        Self::should_skip_cfg_attrs(attrs)
    }

    pub(super) fn is_rust_libtest_metadata_type(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(tp) => {
                let parts: Vec<String> = tp
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                parts.len() >= 2
                    && parts[0] == "test"
                    && matches!(parts.last().map(|s| s.as_str()), Some("TestDescAndFn"))
            }
            syn::Type::Reference(r) => self.is_rust_libtest_metadata_type(&r.elem),
            syn::Type::Ptr(p) => self.is_rust_libtest_metadata_type(&p.elem),
            syn::Type::Paren(p) => self.is_rust_libtest_metadata_type(&p.elem),
            syn::Type::Group(g) => self.is_rust_libtest_metadata_type(&g.elem),
            syn::Type::Array(a) => self.is_rust_libtest_metadata_type(&a.elem),
            syn::Type::Slice(s) => self.is_rust_libtest_metadata_type(&s.elem),
            syn::Type::Tuple(t) => t
                .elems
                .iter()
                .any(|elem| self.is_rust_libtest_metadata_type(elem)),
            _ => false,
        }
    }

    pub(super) fn is_rust_libtest_main(&self, f: &syn::ItemFn) -> bool {
        if f.sig.ident != "main" {
            return false;
        }
        let body = normalize_token_text(f.block.to_token_stream().to_string());
        body.contains("test :: test_main_static") || body.contains("test::test_main_static")
    }

    pub(super) fn is_expanded_test_marker_function(&self, fn_name: &str) -> bool {
        if !self.expanded_libtest_mode {
            return false;
        }
        let scoped = if self.module_stack.is_empty() {
            fn_name.to_string()
        } else {
            format!("{}::{}", self.module_stack.join("::"), fn_name)
        };
        self.expanded_test_markers.iter().any(|marker| {
            marker == fn_name
                || marker == &scoped
                || marker
                    .rsplit("::")
                    .next()
                    .is_some_and(|tail| tail == fn_name)
        })
    }

    pub(super) fn has_rustc_test_marker_attr(attrs: &[syn::Attribute]) -> bool {
        attrs
            .iter()
            .any(|attr| attr.path().is_ident("rustc_test_marker"))
    }

    pub(super) fn is_local_function_name_in_scope(&self, name: &str) -> bool {
        self.local_function_bindings
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
    }

    pub(super) fn is_local_type_name_in_scope(&self, name: &str) -> bool {
        self.local_type_bindings
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
    }

    pub(super) fn should_emit_data_enum_variant_ctor_helper(&self, ctor_name: &str) -> bool {
        if ctor_name.is_empty() {
            return false;
        }
        if matches!(ctor_name, "Left" | "Right") {
            // `Either::Left/Right` helpers are widely referenced as bare
            // constructors in expanded code paths; keep emitting them.
            return true;
        }
        if self.current_scope_declares_type_name(ctor_name)
            || self.is_type_param_in_scope(ctor_name)
        {
            return false;
        }
        let scope_key = self.module_stack.join("::");
        let type_like_import = self
            .resolve_scope_import_binding_path_for_scope(&scope_key, ctor_name)
            .or_else(|| self.resolve_scope_import_binding_path_for_scope("", ctor_name))
            .is_some_and(|target| {
                let normalized = target.trim().trim_start_matches("::").to_string();
                if normalized.is_empty() {
                    return false;
                }
                let escaped = Self::escape_qualified_path_preserve_global(&normalized);
                if self.is_known_free_function_path(&normalized)
                    || self.is_known_free_function_path(&escaped)
                {
                    return false;
                }
                if self.local_declared_types.contains(&normalized)
                    || self.local_declared_types.contains(&escaped)
                {
                    return true;
                }
                let tail = normalized.rsplit("::").next().unwrap_or("");
                tail.chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_uppercase())
            });
        !type_like_import
    }

    pub(super) fn is_local_new_const_constructor_call(&self, expr: &syn::Expr) -> bool {
        if self.block_depth == 0 {
            return false;
        }
        let syn::Expr::Call(call) = self.peel_paren_group_expr(expr) else {
            return false;
        };
        if !call.args.is_empty() {
            return false;
        }
        let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(call.func.as_ref()) else {
            return false;
        };
        path_expr
            .path
            .segments
            .last()
            .is_some_and(|seg| seg.ident == "new_const")
    }

    pub(super) fn is_thread_local_key_type(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_paren_group_type(ty);
        let syn::Type::Path(type_path) = ty else {
            return false;
        };
        type_path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "LocalKey")
    }

    pub(super) fn should_skip_generic_serialize_extension_overload(
        &self,
        method_name: &str,
        self_cpp_ty: &str,
        free_generics: &syn::Generics,
        params: &[String],
    ) -> bool {
        if method_name != "serialize" {
            return false;
        }

        let self_ident = self_cpp_ty.trim();
        if self_ident.is_empty()
            || !self_ident
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return false;
        }

        let generic_names: HashSet<String> = free_generics
            .params
            .iter()
            .filter_map(|param| match param {
                syn::GenericParam::Type(tp) => Some(tp.ident.to_string()),
                _ => None,
            })
            .collect();
        if !generic_names.contains(self_ident) {
            return false;
        }

        let receiver_prefix = format!("const {}& ", self_ident);
        let receiver_is_generic_const_ref = params
            .first()
            .is_some_and(|param| param.starts_with(&receiver_prefix));
        let serializer_is_generic_s = params.get(1).is_some_and(|param| param.starts_with("S "));

        receiver_is_generic_const_ref && serializer_is_generic_s
    }

    pub(super) fn is_plain_self_type(ty: &syn::Type) -> bool {
        matches!(ty, syn::Type::Path(tp) if tp.qself.is_none() && tp.path.segments.len() == 1 && tp.path.segments[0].ident == "Self")
    }

    pub(super) fn is_local_import_alias_name(&self, name: &str) -> bool {
        if self.import_alias_names.contains(name) {
            return true;
        }
        self.import_alias_names
            .iter()
            .any(|alias| escape_cpp_keyword(alias) == name)
    }

    pub(super) fn is_top_level_module_namespace_alias_name(&self, name: &str) -> bool {
        self.module_namespace_renames.iter().any(|(raw, renamed)| {
            !raw.contains("::")
                && raw != renamed
                && (raw == name || escape_cpp_keyword(raw) == name)
        })
    }

    pub(super) fn should_skip_namespace_alias_statement(&self, alias: &str, target: &str) -> bool {
        let alias = alias.trim().trim_start_matches("::");
        let target = target.trim().trim_start_matches("::");
        if alias.is_empty() || target.is_empty() {
            return false;
        }
        if self.module_name.is_some() && alias == "fmt" && target == "rusty::fmt" {
            return true;
        }
        target == alias
    }

    pub(super) fn is_variant_constructor_alias_import(&self, path: &str) -> bool {
        let normalized = normalize_use_import_path(path);
        let Some((_, target)) = split_use_import_alias(normalized) else {
            return false;
        };
        Self::canonical_constructor_name_for_import_target(target).is_some()
    }

    pub(super) fn should_rebind_owner_to_descendant(&self, owner_module: &str, owner_type: &str) -> bool {
        if owner_module.is_empty() || owner_type.is_empty() {
            return false;
        }
        let escaped_owner_module = owner_module
            .split("::")
            .filter(|seg| !seg.is_empty())
            .map(escape_cpp_keyword)
            .collect::<Vec<String>>()
            .join("::");
        let escaped_owner_type = escape_cpp_keyword(owner_type);
        let direct_candidates = [
            format!("{}::{}", owner_module, owner_type),
            format!("{}::{}", owner_module, escaped_owner_type),
            format!("{}::{}", escaped_owner_module, owner_type),
            format!("{}::{}", escaped_owner_module, escaped_owner_type),
        ];
        if direct_candidates
            .iter()
            .any(|candidate| self.local_declared_types.contains(candidate))
        {
            return false;
        }
        if let Some(bound_owner_type) =
            self.resolve_scope_import_binding_path_for_scope(owner_module, owner_type)
        {
            let normalized = bound_owner_type.trim_start_matches("::");
            if !normalized.is_empty()
                && !matches!(classify_use_import(normalized), UseImportAction::RustOnly)
            {
                return false;
            }
        }
        true
    }

    pub(super) fn is_skipped_module_trait_import(&self, path: &str) -> bool {
        let normalized = normalize_use_import_path(path).trim();
        if normalized.is_empty() || normalized.starts_with("namespace ") {
            return false;
        }
        let stripped = normalized.trim_start_matches("::");
        let stripped_tail = stripped.rsplit("::").next().unwrap_or(stripped);
        if self.skipped_module_traits.contains(stripped) {
            return true;
        }
        if self.skipped_module_traits.contains(stripped_tail) {
            return true;
        }
        if let Some((_, target)) = split_use_import_alias(stripped) {
            let target = target.trim_start_matches("::");
            let target_tail = target.rsplit("::").next().unwrap_or(target);
            if self.skipped_module_traits.contains(target) {
                return true;
            }
            if self.skipped_module_traits.contains(target_tail) {
                return true;
            }
        }
        if stripped.starts_with("de::")
            && matches!(
                stripped_tail,
                "Deserialize"
                    | "DeserializeSeed"
                    | "Deserializer"
                    | "EnumAccess"
                    | "Error"
                    | "Expected"
                    | "IntoDeserializer"
                    | "MapAccess"
                    | "SeqAccess"
                    | "VariantAccess"
                    | "Visitor"
            )
        {
            return true;
        }
        if stripped.starts_with("ser::")
            && matches!(
                stripped_tail,
                "Serialize"
                    | "Serializer"
                    | "SerializeSeq"
                    | "SerializeTuple"
                    | "SerializeTupleStruct"
                    | "SerializeTupleVariant"
                    | "SerializeMap"
                    | "SerializeStruct"
                    | "SerializeStructVariant"
            )
        {
            return true;
        }
        self.skipped_module_traits.iter().any(|scoped| {
            let tail = scoped.rsplit("::").next().unwrap_or(scoped);
            tail == stripped || tail == stripped_tail || scoped == stripped
        })
    }

    pub(super) fn is_macro_rules_import(&self, path: &str) -> bool {
        let normalized = normalize_use_import_path(path);
        let last = normalized.split("::").last().unwrap_or(normalized);
        self.macro_rules_names.contains(last)
    }

    pub(super) fn should_skip_unresolved_bare_import(&self, path: &str) -> bool {
        if self.module_stack.is_empty() {
            return false;
        }
        if path.trim_start().starts_with("namespace ") {
            return false;
        }
        let normalized = normalize_use_import_path(path);
        if normalized.is_empty()
            || normalized.contains("::")
            || normalized.contains(" = ")
            || normalized.starts_with("namespace ")
        {
            return false;
        }
        if self.declared_item_names.contains(normalized) {
            return false;
        }
        normalized
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase())
    }

    pub(super) fn should_skip_unresolved_single_segment_type_import(&self, using_path: &str) -> bool {
        if !(self.module_name.is_some() || self.expanded_libtest_mode) {
            return false;
        }
        let normalized = using_path
            .trim()
            .trim_start_matches("::")
            .trim_start_matches("typename ")
            .trim();
        if normalized.is_empty()
            || normalized.contains("::")
            || normalized.contains(" = ")
            || normalized.starts_with("namespace ")
        {
            return false;
        }
        if !normalized
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            return false;
        }
        if self.declared_item_names.contains(normalized)
            || self.local_declared_types.contains(normalized)
            || self.import_alias_names.contains(normalized)
            || self.is_local_type_name_in_scope(normalized)
        {
            return false;
        }
        true
    }

    pub(super) fn should_skip_unresolved_function_using_import(&self, using_path: &str) -> bool {
        let normalized = using_path
            .trim()
            .trim_start_matches("::")
            .trim_start_matches("typename ")
            .trim();
        if normalized.is_empty()
            || !normalized.contains("::")
            || normalized.contains(" = ")
            || normalized.starts_with("namespace ")
        {
            return false;
        }
        let tail = normalized.rsplit("::").next().unwrap_or(normalized);
        if !tail
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_lowercase() || ch == '_')
        {
            return false;
        }
        let normalized_variants = self.owner_key_spelling_variants(normalized);
        let is_known_function_path = normalized_variants
            .iter()
            .any(|candidate| self.is_known_free_function_path(candidate));
        if !is_known_function_path {
            return false;
        }
        !normalized_variants
            .iter()
            .any(|candidate| self.forward_declared_function_paths.contains(candidate))
    }

    pub(super) fn should_force_qualified_import_binding_name(&self, local_name: &str) -> bool {
        if local_name.is_empty() || !(self.module_name.is_some() || self.expanded_libtest_mode) {
            return false;
        }
        let escaped = escape_cpp_keyword(local_name);
        let scope = self.module_stack.join("::");
        let scoped_raw = if scope.is_empty() {
            local_name.to_string()
        } else {
            format!("{}::{}", scope, local_name)
        };
        let scoped_escaped = if scope.is_empty() {
            escaped.clone()
        } else {
            format!("{}::{}", scope, escaped)
        };
        self.module_runtime_helper_trait_type_names
            .contains_key(local_name)
            || self
                .module_runtime_helper_trait_type_names
                .contains_key(&escaped)
            || self
                .module_runtime_helper_trait_type_names
                .contains_key(&scoped_raw)
            || self
                .module_runtime_helper_trait_type_names
                .contains_key(&scoped_escaped)
    }

    /// Check if a const member's type is the enclosing struct (self-referential).
    /// These need split declaration (inside struct) + definition (after struct).
    pub(super) fn is_self_referential_const_type(&self, ty_cpp: &str) -> bool {
        if let Some(ref struct_name) = self.current_struct {
            // Direct self-reference: const type IS the struct
            if ty_cpp == *struct_name || ty_cpp.starts_with(&format!("{}<", struct_name)) {
                return true;
            }
            // Indirect self-reference: const type contains the struct name
            // in a template argument (e.g., `std::span<const Flag<TestFlags>>`)
            // which requires the struct to be complete at instantiation.
            if ty_cpp.contains(&format!("<{}>", struct_name))
                || ty_cpp.contains(&format!("<{},", struct_name))
                || ty_cpp.contains(&format!(", {}>", struct_name))
            {
                return true;
            }
            false
        } else {
            false
        }
    }

    pub(super) fn is_view_like_cpp_type(ty: &str) -> bool {
        let mut normalized = ty.trim();
        if let Some(stripped) = normalized.strip_prefix("const ") {
            normalized = stripped.trim();
        }
        if let Some(stripped) = normalized.strip_suffix('&') {
            normalized = stripped.trim();
        }
        normalized.starts_with("std::span<") || normalized == "std::string_view"
    }

    pub(super) fn should_skip_recursive_bitflags_forwarder(
        &self,
        emitted_name: &str,
        method: &syn::ImplItemFn,
        is_static: bool,
    ) -> bool {
        if !self.current_struct_is_bitflags_like() {
            return false;
        }
        match emitted_name {
            "bits" if !is_static => self.method_is_direct_recursive_bits_forwarder(method),
            "from_bits_retain" if is_static => {
                self.method_is_direct_recursive_from_bits_retain_forwarder(method)
            }
            _ => false,
        }
    }

    pub(super) fn is_binding_only_tuple_arm_pattern(&self, pat: &syn::Pat, arity: usize) -> bool {
        match pat {
            syn::Pat::Tuple(tuple_pat) => {
                tuple_pat.elems.len() == arity
                    && tuple_pat
                        .elems
                        .iter()
                        .all(|elem| self.is_binding_only_pattern(elem))
            }
            syn::Pat::Wild(_) => true,
            syn::Pat::Ident(pi) => {
                pi.ident == "_" || !self.pattern_ident_is_const_value(&pi.ident.to_string())
            }
            _ => false,
        }
    }

    pub(super) fn is_binding_only_pattern(&self, pat: &syn::Pat) -> bool {
        match pat {
            syn::Pat::Ident(pi) => {
                pi.ident == "_" || !self.pattern_ident_is_const_value(&pi.ident.to_string())
            }
            syn::Pat::Wild(_) => true,
            syn::Pat::Tuple(tuple_pat) => tuple_pat
                .elems
                .iter()
                .all(|elem| self.is_binding_only_pattern(elem)),
            syn::Pat::Type(pt) => self.is_binding_only_pattern(&pt.pat),
            syn::Pat::Reference(r) => self.is_binding_only_pattern(&r.pat),
            syn::Pat::Paren(p) => self.is_binding_only_pattern(&p.pat),
            _ => false,
        }
    }

    pub(super) fn is_stable_reference_lvalue_expr(&self, expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                let name = path.path.segments[0].ident.to_string();
                name == "self"
                    || self.lookup_local_binding_type(&name).is_some()
                    || self.is_local_binding_in_scope(&name)
            }
            syn::Expr::Field(field) => self.is_stable_reference_lvalue_expr(&field.base),
            syn::Expr::Index(index) => self.is_stable_reference_lvalue_expr(&index.expr),
            syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
                // `&*x` is stable for raw pointers/references. For value-like
                // deref receivers (for example `SmallVec`/`Box` operator*), use
                // temporary materialization at call sites instead of taking
                // address directly from `*x`.
                if self.is_expr_raw_pointer_like(&unary.expr) {
                    return true;
                }
                if !self.is_stable_reference_lvalue_expr(&unary.expr) {
                    return false;
                }
                self.infer_simple_expr_type(&unary.expr)
                    .as_ref()
                    .is_some_and(|ty| {
                        matches!(
                            self.peel_reference_paren_group_type(ty),
                            syn::Type::Reference(_) | syn::Type::Ptr(_)
                        )
                    })
            }
            syn::Expr::Reference(r) => self.is_stable_reference_lvalue_expr(&r.expr),
            syn::Expr::Paren(p) => self.is_stable_reference_lvalue_expr(&p.expr),
            syn::Expr::Group(g) => self.is_stable_reference_lvalue_expr(&g.expr),
            _ => false,
        }
    }

    pub(super) fn is_reference_to_slice_range_index_expr(&self, expr: &syn::Expr) -> bool {
        let reference_target = match self.peel_paren_group_expr(expr) {
            syn::Expr::Reference(r) => self.peel_reference_target_expr(&r.expr),
            _ => return false,
        };
        self.is_slice_range_index_target_expr(reference_target)
    }

    pub(super) fn is_slice_range_index_target_expr(&self, expr: &syn::Expr) -> bool {
        match self.peel_paren_group_expr(expr) {
            syn::Expr::Index(idx) => self.is_slice_range_index_expr(&idx.index),
            _ => false,
        }
    }

    pub(super) fn should_normalize_tuple_reference_target_to_slice_full(&self, expr: &syn::Expr) -> bool {
        match self.peel_paren_group_expr(expr) {
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                self.is_stable_reference_lvalue_expr(expr)
            }
            syn::Expr::Field(field) => {
                self.should_normalize_tuple_reference_target_to_slice_full(&field.base)
            }
            _ => false,
        }
    }

    pub(super) fn is_simple_ident(name: &str) -> bool {
        let mut chars = name.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        if !(first.is_ascii_alphabetic() || first == '_') {
            return false;
        }
        chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    pub(super) fn is_unsuffixed_int_literal_expr(expr: &syn::Expr) -> bool {
        matches!(
            expr,
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(lit),
                ..
            }) if lit.suffix().is_empty()
        )
    }

    pub(super) fn is_plain_ident_path_expr(expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Paren(paren) => Self::is_plain_ident_path_expr(&paren.expr),
            syn::Expr::Group(group) => Self::is_plain_ident_path_expr(&group.expr),
            syn::Expr::Path(path_expr) => {
                path_expr.qself.is_none()
                    && path_expr.path.leading_colon.is_none()
                    && path_expr.path.segments.len() == 1
            }
            _ => false,
        }
    }

    pub(super) fn is_mut_raw_pointer_type(ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Ptr(ptr) => ptr.mutability.is_some(),
            syn::Type::Paren(p) => Self::is_mut_raw_pointer_type(&p.elem),
            syn::Type::Group(g) => Self::is_mut_raw_pointer_type(&g.elem),
            _ => false,
        }
    }

    pub(super) fn is_mut_reference_type(ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Reference(reference) => reference.mutability.is_some(),
            syn::Type::Paren(p) => Self::is_mut_reference_type(&p.elem),
            syn::Type::Group(g) => Self::is_mut_reference_type(&g.elem),
            _ => false,
        }
    }

    pub(super) fn is_reference_binding_lowered_to_pointer_storage(&self, name: &str) -> bool {
        // Immutable shadow bindings must not inherit pointer-lowered behavior
        // from an earlier mutable binding with the same Rust name.
        if self.is_const_local_binding_in_scope(name) {
            return false;
        }
        if !self.is_rebind_reference_pointer_binding_in_scope(name) {
            return false;
        }
        true
    }

    pub(super) fn is_rebind_reference_binding(&self, name: &str) -> bool {
        self.is_reference_binding_lowered_to_pointer_storage(name)
    }

    pub(super) fn is_raw_pointer_type(ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Ptr(_) => true,
            syn::Type::Paren(p) => Self::is_raw_pointer_type(&p.elem),
            syn::Type::Group(g) => Self::is_raw_pointer_type(&g.elem),
            _ => false,
        }
    }

    pub(super) fn should_materialize_slice_range_pointer_storage(
        &self,
        resolved_ty: &syn::Type,
        init_expr: &syn::Expr,
    ) -> bool {
        if !Self::is_raw_pointer_type(resolved_ty) {
            return false;
        }
        let syn::Expr::Reference(r) = self.peel_paren_group_expr(init_expr) else {
            return false;
        };
        self.is_slice_range_index_target_expr(self.peel_reference_target_expr(&r.expr))
    }

    pub(super) fn should_emit_repeat_seed_cast(elem_cpp: &str) -> bool {
        let normalized = elem_cpp.trim();
        if normalized.is_empty()
            || normalized.contains("/* TODO")
            || type_string_has_auto_placeholder(normalized)
        {
            return false;
        }
        is_numeric_cpp_scalar_type(normalized)
            || matches!(
                normalized,
                "bool" | "char" | "char32_t" | "float" | "double"
            )
    }

    pub(super) fn should_use_optional_delayed_init_storage(&self, ty: &syn::Type) -> bool {
        !matches!(
            ty,
            syn::Type::Reference(_) | syn::Type::ImplTrait(_) | syn::Type::Infer(_)
        )
    }

    pub(super) fn is_delayed_init_local(&self, name: &str) -> bool {
        self.delayed_init_locals
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
    }

    pub(super) fn is_local_item_const_name_in_scope(&self, name: &str) -> bool {
        self.local_item_const_names
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
    }

    pub(super) fn is_rebind_reference_pointer_binding_in_scope(&self, name: &str) -> bool {
        self.rebind_reference_pointer_bindings
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
    }

    pub(super) fn is_local_reference_binding_in_scope(&self, name: &str) -> bool {
        self.local_reference_bindings
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
    }

    pub(super) fn is_local_manually_drop_binding_in_scope(&self, name: &str) -> bool {
        self.local_manually_drop_bindings
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
    }

    pub(super) fn is_const_local_binding_in_scope(&self, name: &str) -> bool {
        self.local_const_bindings
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
            .unwrap_or(false)
    }

    /// Returns true if `name` is a type parameter of the current struct
    /// (e.g., `A` in `impl<A: Array> SmallVec<A>` when emitting SmallVec methods).
    pub(super) fn is_struct_type_param(&self, name: &str) -> bool {
        if let Some(struct_name) = &self.current_struct {
            let key = self.scoped_type_key(struct_name);
            if let Some(params) = self
                .declared_type_params
                .get(struct_name)
                .or_else(|| self.declared_type_params.get(&key))
            {
                return params.iter().any(|p| p == name);
            }
        }
        false
    }

    pub(super) fn is_u8_raw_pointer_type(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Ptr(ptr) = ty else {
            return false;
        };
        let elem = self.peel_reference_paren_group_type(&ptr.elem);
        matches!(
            elem,
            syn::Type::Path(tp)
                if tp.qself.is_none()
                    && tp.path.segments.len() == 1
                    && tp.path.segments[0].ident == "u8"
        )
    }

    pub(super) fn should_skip_expected_cast_for_inferred_as_ptr_u8_fallback(
        &self,
        expr: &syn::Expr,
        inferred_ty: &syn::Type,
    ) -> bool {
        if !self.is_u8_raw_pointer_type(inferred_ty) {
            return false;
        }
        let expr = self.peel_paren_group_expr(expr);
        let syn::Expr::MethodCall(mc) = expr else {
            return false;
        };
        if !mc.args.is_empty() {
            return false;
        }
        let method = mc.method.to_string();
        if method != "as_mut_ptr" {
            return false;
        }
        // If pointee inference fails, the local binder currently falls back to `*mut u8`
        // solely to keep pointer-flow analyses active. In that case, don't force a
        // `u8*` cast in emitted initializer expression.
        self.infer_array_element_type_from_expr(&mc.receiver)
            .is_none()
    }

    pub(super) fn is_manually_drop_type(&self, ty: &syn::Type) -> bool {
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
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        matches!(
            joined.as_str(),
            "ManuallyDrop"
                | "mem::ManuallyDrop"
                | "std::mem::ManuallyDrop"
                | "core::mem::ManuallyDrop"
                | "rusty::mem::ManuallyDrop"
        )
    }

    pub(super) fn should_emit_inferred_sum_type_for_local(
        &self,
        local: &syn::Local,
        binding_name: &str,
        inferred_binding_ty: Option<&syn::Type>,
    ) -> bool {
        if get_local_type(local).is_some() || inferred_binding_ty.is_none() {
            return false;
        }
        if !self.reassigned_vars.contains(binding_name) {
            return false;
        }

        let Some(init) = &local.init else {
            return false;
        };
        let Some(expr) = self.extract_value_expr(&init.expr) else {
            return false;
        };

        if expr_is_option_none_constructor(expr)
            && inferred_binding_ty.is_some_and(|ty| self.is_option_like_syn_type(ty))
        {
            return true;
        }

        if self.extract_constructor_call_expr(expr).is_some() {
            return true;
        }

        match expr {
            syn::Expr::If(if_expr) => self.extract_constructor_pair_from_if(if_expr).is_some(),
            syn::Expr::Match(match_expr) => self
                .extract_constructor_pair_from_match(match_expr)
                .is_some(),
            _ => false,
        }
    }

    pub(super) fn should_emit_inferred_numeric_seed_type_for_local(
        &self,
        local: &syn::Local,
        binding_name: &str,
        inferred_binding_ty: Option<&syn::Type>,
    ) -> bool {
        if get_local_type(local).is_some() {
            return false;
        }
        if !self.reassigned_vars.contains(binding_name) {
            return false;
        }
        if !self.local_binding_is_mutable(local) {
            return false;
        }
        let Some(init) = &local.init else {
            return false;
        };
        if !Self::is_unsuffixed_int_literal_expr(self.peel_paren_group_expr(&init.expr)) {
            return false;
        }
        inferred_binding_ty.is_some_and(|ty| self.type_is_concrete_hint_candidate(ty))
    }

    pub(super) fn is_option_like_syn_type(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return false;
        };
        let Some(last) = tp.path.segments.last() else {
            return false;
        };
        let last_name = last.ident.to_string();
        last_name == "Option"
            || last_name == "optional"
            || self.option_type_aliases.contains(&last_name)
    }

    pub(super) fn is_local_binding_in_scope(&self, name: &str) -> bool {
        self.local_bindings
            .iter()
            .rev()
            .any(|scope| scope.contains_key(name))
    }

    pub(super) fn should_fallback_to_deref_ref_in_deref_mut_scope(&self) -> bool {
        self.deref_mut_ref_fallback_scopes
            .last()
            .copied()
            .unwrap_or(false)
    }

    pub(super) fn should_collapse_untyped_iterator_map_param_deref(&self, name: &str) -> bool {
        if !self
            .iterator_map_closure_param_scopes
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
        {
            return false;
        }
        self.lookup_local_binding_type(name).is_none()
    }

    pub(super) fn should_lower_untyped_closure_param_deref(&self, name: &str) -> bool {
        if !self
            .untyped_closure_param_scopes
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
        {
            return false;
        }
        self.lookup_local_binding_type(name).is_none()
    }

    pub(super) fn should_lower_char_predicate_on_untyped_closure_param(&self, name: &str) -> bool {
        if !self
            .char_predicate_closure_param_scopes
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
        {
            return false;
        }
        self.lookup_local_binding_type(name).is_none()
    }

    pub(super) fn should_force_typed_option_ctor_in_current_scope(&self) -> bool {
        self.force_typed_option_ctor_scopes
            .last()
            .copied()
            .unwrap_or(false)
    }

    pub(super) fn is_pattern_ref_binding_in_scope(&self, name: &str) -> bool {
        self.pattern_ref_bindings
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
    }

    pub(super) fn is_expr_reference_like(&self, expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                let name = path.path.segments[0].ident.to_string();
                if name == "self" {
                    return self.current_self_receiver_is_reference();
                }
                if self.is_pattern_ref_binding_in_scope(&name) {
                    return true;
                }
                if self.is_local_reference_binding_in_scope(&name) {
                    return true;
                }
                self.lookup_local_binding_type(&name)
                    .is_some_and(|ty| self.type_is_reference_like(&ty))
            }
            syn::Expr::Paren(p) => self.is_expr_reference_like(&p.expr),
            syn::Expr::Group(g) => self.is_expr_reference_like(&g.expr),
            syn::Expr::Reference(_) => true,
            syn::Expr::Field(field_expr) => {
                if self
                    .infer_simple_expr_type(expr)
                    .as_ref()
                    .is_some_and(|ty| self.type_is_reference_like(ty))
                {
                    return true;
                }
                let member_name = match &field_expr.member {
                    syn::Member::Named(ident) => ident.to_string(),
                    syn::Member::Unnamed(index) => format!("_{}", index.index),
                };
                self.lookup_field_type_for_expr_base(&field_expr.base, &member_name)
                    .is_some_and(|ty| {
                        matches!(
                            self.peel_reference_paren_group_type(&ty),
                            syn::Type::Reference(_)
                        )
                    })
            }
            syn::Expr::MethodCall(mc) => {
                self.infer_method_call_result_type_for_local(mc)
                    .as_ref()
                    .is_some_and(|ty| self.type_is_reference_like(ty))
                    || self.method_call_is_reference_like_by_shape(mc)
            }
            syn::Expr::Call(call) => {
                self.lookup_associated_call_return_type(call)
                    .as_ref()
                    .is_some_and(|ty| self.type_is_reference_like(ty))
                    || self
                        .lookup_function_return_type(call.func.as_ref())
                        .is_some_and(|ty| self.type_is_reference_like(ty))
                    || self.associated_call_is_reference_like_by_shape(call)
            }
            _ => false,
        }
    }

    pub(super) fn is_self_reference_field_access(&self, expr: &syn::Expr) -> bool {
        let expr = self.peel_paren_group_expr(expr);
        let syn::Expr::Field(field_expr) = expr else {
            return false;
        };
        let syn::Expr::Path(base_path) = self.peel_paren_group_expr(&field_expr.base) else {
            return false;
        };
        if base_path.path.segments.len() != 1 || base_path.path.segments[0].ident != "self" {
            return false;
        }
        let field_name = match &field_expr.member {
            syn::Member::Named(ident) => ident.to_string(),
            syn::Member::Unnamed(idx) => format!("_{}", idx.index),
        };
        if let Some(struct_name) = self.current_struct.as_ref()
            && (self
                .struct_reference_fields
                .get(struct_name)
                .is_some_and(|fields| fields.contains(&field_name))
                || self
                    .struct_reference_fields
                    .get(&self.scoped_type_key(struct_name))
                    .is_some_and(|fields| fields.contains(&field_name)))
        {
            return true;
        }
        self.lookup_local_binding_type("self")
            .or_else(|| self.lookup_local_binding_type("self_"))
            .and_then(|self_ty| self.lookup_field_type_from_type(&self_ty, &field_name))
            .is_some_and(|ty| {
                matches!(
                    self.peel_reference_paren_group_type(&ty),
                    syn::Type::Reference(_)
                )
            })
    }

    /// Detect whether a Rust expression is diverging (never returns), e.g. calls
    /// to `panic!`, `unreachable!`, `unreachable_display`, `abort`, etc.
    /// Used to avoid emitting `return <void-expr>` in match arm bodies.
    pub(super) fn is_expr_diverging(&self, expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Return(_) | syn::Expr::Break(_) | syn::Expr::Continue(_) => true,
            syn::Expr::Call(call) => {
                let path_str = self.expr_path_string(&call.func);
                Self::is_diverging_function_path(&path_str)
            }
            syn::Expr::Block(eb) => {
                // Block is diverging if its last statement/expression is diverging
                if let Some(syn::Stmt::Expr(e, _)) = eb.block.stmts.last() {
                    self.is_expr_diverging(e)
                } else {
                    false
                }
            }
            syn::Expr::Unsafe(unsafe_expr) => unsafe_expr.block.stmts.last().is_some_and(
                |stmt| matches!(stmt, syn::Stmt::Expr(e, _) if self.is_expr_diverging(e)),
            ),
            syn::Expr::If(if_expr) => {
                let then_diverges = self.is_expr_diverging(&syn::Expr::Block(syn::ExprBlock {
                    attrs: Vec::new(),
                    label: None,
                    block: if_expr.then_branch.clone(),
                }));
                let else_diverges = if let Some((_, else_expr)) = &if_expr.else_branch {
                    self.is_expr_diverging(else_expr)
                } else {
                    false
                };
                then_diverges && else_diverges
            }
            syn::Expr::Match(match_expr) => {
                !match_expr.arms.is_empty()
                    && match_expr
                        .arms
                        .iter()
                        .all(|arm| self.is_expr_diverging(&arm.body))
            }
            syn::Expr::Macro(m) => {
                let macro_name = m
                    .mac
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();
                matches!(
                    macro_name.as_str(),
                    "panic" | "unreachable" | "unimplemented" | "todo" | "abort"
                )
            }
            _ => false,
        }
    }

    pub(super) fn is_diverging_function_path(path: &str) -> bool {
        matches!(
            path,
            "panic"
                | "panic_fmt"
                | "unreachable"
                | "unreachable_display"
                | "abort"
                | "std::process::abort"
                | "std::abort"
                | "core::panicking::panic"
                | "core::panicking::panic_fmt"
                | "core::panicking::unreachable_display"
                | "core::intrinsics::unreachable"
                | "core::hint::unreachable_unchecked"
                | "std::hint::unreachable_unchecked"
                | "alloc::alloc::handle_alloc_error"
                | "alloc::handle_alloc_error"
                | "core::alloc::handle_alloc_error"
                | "std::alloc::handle_alloc_error"
                | "panicking::panic"
                | "panicking::panic_fmt"
                | "panicking::unreachable_display"
                | "intrinsics::unreachable"
                | "hint::unreachable_unchecked"
                | "unreachable_unchecked"
        )
    }

    pub(super) fn is_expr_raw_pointer_like(&self, expr: &syn::Expr) -> bool {
        let peeled = self.peel_paren_group_expr(expr);
        if self
            .infer_simple_expr_type(peeled)
            .is_some_and(|ty| self.is_type_raw_pointer_like(&ty))
        {
            return true;
        }

        match peeled {
            syn::Expr::RawAddr(_) => true,
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                let name = path.path.segments[0].ident.to_string();
                self.lookup_local_binding_type(&name)
                    .is_some_and(|ty| self.is_type_raw_pointer_like(&ty))
            }
            syn::Expr::Cast(cast) => matches!(cast.ty.as_ref(), syn::Type::Ptr(_)),
            syn::Expr::MethodCall(mc) => {
                let method = mc.method.to_string();
                if matches!(method.as_str(), "as_ptr" | "as_mut_ptr") {
                    return true;
                }
                if method == "get" && mc.args.is_empty() {
                    if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver) {
                        let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
                        if let syn::Type::Path(tp) = receiver_ty
                            && let Some(last) = tp.path.segments.last()
                            && last.ident == "UnsafeCell"
                        {
                            return true;
                        }
                    }
                }
                if method == "load" && !mc.args.is_empty() {
                    if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver) {
                        let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
                        if let syn::Type::Path(tp) = receiver_ty
                            && let Some(last) = tp.path.segments.last()
                            && last.ident == "AtomicPtr"
                        {
                            return true;
                        }
                    }
                }
                if matches!(
                    method.as_str(),
                    "add" | "offset" | "sub" | "wrapping_add" | "wrapping_sub" | "wrapping_offset"
                ) {
                    return self.is_expr_raw_pointer_like(&mc.receiver);
                }
                // General: a user method whose DECLARED return type is a raw
                // pointer (e.g. `RawTableInner::ctrl(&self, i) -> *mut u8`).
                // Resolve the receiver's owner type, look the method's return
                // type up in its impl block, and check for a pointer. This lets
                // the pointer-intrinsic lowerings (copy_to_nonoverlapping,
                // write_bytes, ptr::add) fire on `recv.ctrl(i).copy_to(...)`
                // chains, which `infer_simple_expr_type` can't type (it has no
                // MethodCall arm).
                if let Some(owner) = self.infer_method_call_receiver_owner_name(&mc.receiver) {
                    let scoped = self.scoped_type_key(&owner);
                    if let Some(ret_ty) = self
                        .lookup_method_return_type_for_owner_key(&scoped, &method)
                        .or_else(|| self.lookup_method_return_type_for_owner_key(&owner, &method))
                        && self.is_type_raw_pointer_like(&ret_ty)
                    {
                        return true;
                    }
                }
                false
            }
            syn::Expr::Call(call) => {
                let syn::Expr::Path(path) = call.func.as_ref() else {
                    return false;
                };
                Self::is_ptr_add_or_offset_call_path(&path.path)
            }
            _ => false,
        }
    }

    pub(super) fn is_type_raw_pointer_like(&self, ty: &syn::Type) -> bool {
        if matches!(ty, syn::Type::Ptr(_)) {
            return true;
        }
        let mapped = self.map_type(ty);
        let canonical = self.canonical_into_target_cpp_type(&mapped);
        let canonical = canonical.trim();
        canonical.starts_with("std::add_pointer_t<") || canonical.ends_with('*')
    }

    pub(super) fn is_ptr_add_or_offset_call_path(path: &syn::Path) -> bool {
        let joined = path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        matches!(
            joined.as_str(),
            "rusty::ptr::add"
                | "rusty::ptr::offset"
                | "rusty::ptr::sub"
                | "ptr::add"
                | "ptr::offset"
                | "ptr::sub"
                | "core::ptr::add"
                | "std::ptr::add"
                | "core::ptr::offset"
                | "std::ptr::offset"
                | "core::ptr::sub"
                | "std::ptr::sub"
                | "core::ptr::mut_ptr::add"
                | "std::ptr::mut_ptr::add"
                | "ptr::mut_ptr::add"
                | "core::ptr::mut_ptr::offset"
                | "std::ptr::mut_ptr::offset"
                | "ptr::mut_ptr::offset"
                | "core::ptr::mut_ptr::wrapping_offset"
                | "std::ptr::mut_ptr::wrapping_offset"
                | "ptr::mut_ptr::wrapping_offset"
                | "core::ptr::mut_ptr::sub"
                | "std::ptr::mut_ptr::sub"
                | "ptr::mut_ptr::sub"
                | "core::ptr::const_ptr::add"
                | "std::ptr::const_ptr::add"
                | "ptr::const_ptr::add"
                | "core::ptr::const_ptr::offset"
                | "std::ptr::const_ptr::offset"
                | "ptr::const_ptr::offset"
                | "core::ptr::const_ptr::wrapping_offset"
                | "std::ptr::const_ptr::wrapping_offset"
                | "ptr::const_ptr::wrapping_offset"
                | "core::ptr::const_ptr::sub"
                | "std::ptr::const_ptr::sub"
                | "ptr::const_ptr::sub"
        )
    }

    pub(super) fn is_known_integer_like_type(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(tp) if tp.qself.is_none() && tp.path.segments.len() == 1 => {
                let name = tp.path.segments[0].ident.to_string();
                matches!(
                    name.as_str(),
                    "i8" | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "isize"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "usize"
                ) || self.numeric_type_aliases.keys().any(|candidate| {
                    candidate == &name || candidate.ends_with(&format!("::{}", name))
                })
            }
            _ => {
                let mapped = self.map_type(ty);
                let normalized = mapped
                    .trim_start_matches("const ")
                    .trim_end_matches('&')
                    .trim_end_matches('*')
                    .trim();
                is_numeric_cpp_scalar_type(normalized)
            }
        }
    }

    pub(super) fn should_lower_saturating_method_call(&self, receiver: &syn::Expr) -> bool {
        let peeled = self.peel_paren_group_expr(receiver);
        match self.infer_simple_expr_type(peeled) {
            Some(ty) => self.is_known_integer_like_type(&ty),
            // Pattern-bound temporaries in match lowering often have no explicit
            // local binding type. Rust saturating arithmetic methods are numeric
            // intrinsics, so unknown receiver type defaults to helper lowering.
            None => true,
        }
    }

    pub(super) fn should_lower_integer_rotate_method_call(&self, receiver: &syn::Expr) -> bool {
        let peeled = self.peel_paren_group_expr(receiver);
        if let Some(ty) = self.infer_simple_expr_type(peeled) {
            return self.is_known_integer_like_type(&ty);
        }
        matches!(
            peeled,
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(_),
                ..
            })
        )
    }

    pub(super) fn should_lower_integer_intrinsic_method_call(&self, receiver: &syn::Expr) -> bool {
        let peeled = self.peel_paren_group_expr(receiver);
        if let Some(ty) = self.infer_simple_expr_type(peeled) {
            return self.is_known_integer_like_type(&ty);
        }
        if let syn::Expr::Path(path) = peeled
            && path.path.segments.len() == 1
        {
            let name = path.path.segments[0].ident.to_string();
            if self.lookup_local_binding_cpp_name(&name).is_some()
                || self.lookup_local_binding_type(&name).is_some()
                || self.lookup_local_placeholder_type_hint(&name).is_some()
            {
                return true;
            }
        }
        matches!(
            peeled,
            syn::Expr::Binary(_)
                | syn::Expr::Cast(_)
                | syn::Expr::Paren(_)
                | syn::Expr::Group(_)
                | syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(_),
                    ..
                })
        )
    }

    pub(super) fn is_known_scalar_like_type(&self, ty: &syn::Type) -> bool {
        if self.is_known_integer_like_type(ty) {
            return true;
        }
        match ty {
            syn::Type::Path(tp) if tp.qself.is_none() && tp.path.segments.len() == 1 => {
                let name = tp.path.segments[0].ident.to_string();
                matches!(name.as_str(), "f32" | "f64" | "bool" | "char")
            }
            _ => {
                let canonical = self.canonical_into_target_cpp_type(&self.map_type(ty));
                Self::is_scalar_into_target_cpp_type(&canonical)
            }
        }
    }

    pub(super) fn is_known_float_like_type(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(tp) if tp.qself.is_none() && tp.path.segments.len() == 1 => {
                let name = tp.path.segments[0].ident.to_string();
                if matches!(name.as_str(), "f32" | "f64" | "float" | "double") {
                    return true;
                }
            }
            _ => {}
        }
        let canonical = self.canonical_into_target_cpp_type(&self.map_type(ty));
        matches!(canonical.as_str(), "float" | "double" | "long double")
    }

    pub(super) fn is_known_string_like_type(&self, ty: &syn::Type) -> bool {
        let canonical = self.canonical_into_target_cpp_type(&self.map_type(ty));
        matches!(
            canonical.as_str(),
            "rusty::String" | "std::string" | "std::string_view" | "char*"
        )
    }

    pub(super) fn is_known_cow_like_type(&self, ty: &syn::Type) -> bool {
        let mut current = self.peel_reference_paren_group_type(ty).clone();
        for _ in 0..4 {
            let canonical = self.canonical_into_target_cpp_type(&self.map_type(&current));
            let compact = canonical.replace(' ', "");
            if compact == "rusty::Cow"
                || compact.contains("std::variant<rusty::Cow_Borrowed,rusty::Cow_Owned>")
                || compact.contains("std::variant<Cow_Borrowed,Cow_Owned>")
            {
                return true;
            }
            let Some(next) = self.resolve_type_alias_once(&current) else {
                break;
            };
            if next == current {
                break;
            }
            current = next;
        }
        false
    }

    pub(super) fn is_known_alloc_layout_type(&self, ty: &syn::Type) -> bool {
        let canonical = self.canonical_into_target_cpp_type(&self.map_type(ty));
        if canonical == "rusty::alloc::Layout" {
            return true;
        }
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return false;
        };
        let segs: Vec<String> = tp
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        match segs.as_slice() {
            [single] => single == "Layout",
            [root, module, leaf, ..] => {
                matches!(root.as_str(), "std" | "core") && module == "alloc" && leaf == "Layout"
            }
            [root, leaf] => root == "alloc" && leaf == "Layout",
            _ => false,
        }
    }

    pub(super) fn is_slice_view_constructor_path(path: &syn::Path) -> bool {
        let joined = path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        matches!(
            joined.as_str(),
            "slice::from_raw_parts"
                | "core::slice::from_raw_parts"
                | "std::slice::from_raw_parts"
                | "slice::from_raw_parts_mut"
                | "core::slice::from_raw_parts_mut"
                | "std::slice::from_raw_parts_mut"
        )
    }

    pub(super) fn should_lower_slice_deref_method_call(&self, receiver: &syn::Expr) -> bool {
        if self.expr_lowers_to_slice_or_span_view(receiver) {
            return true;
        }
        if self.receiver_has_slice_like_view_method(receiver) {
            return true;
        }

        if matches!(self.peel_paren_group_expr(receiver), syn::Expr::Path(path)
            if path.path.segments.len() == 1 && path.path.segments[0].ident == "self")
        {
            if let Some(current_struct) = self.current_struct.as_ref() {
                return matches!(current_struct.as_str(), "Vec" | "ArrayVec" | "SmallVec");
            }
        }

        let Some(receiver_ty) = self
            .infer_simple_expr_type(receiver)
            .or_else(|| self.infer_local_binding_type_from_initializer(receiver))
        else {
            return false;
        };
        let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
        if self.type_is_slice_or_span_like(receiver_ty) {
            return true;
        }

        let syn::Type::Path(tp) = receiver_ty else {
            return false;
        };
        let receiver_cpp = self.canonical_into_target_cpp_type(&self.map_type(receiver_ty));
        if receiver_cpp.starts_with("rusty::Vec<")
            || receiver_cpp.starts_with("rusty::VecDeque<")
            || receiver_cpp.starts_with("rusty::slice::")
            || receiver_cpp.starts_with("std::span<")
        {
            return true;
        }
        let Some(last) = tp.path.segments.last() else {
            return false;
        };
        let receiver_name = if last.ident == "Self" {
            self.current_struct
                .as_ref()
                .cloned()
                .unwrap_or_else(|| "Self".to_string())
        } else {
            last.ident.to_string()
        };
        matches!(receiver_name.as_str(), "Vec" | "ArrayVec" | "SmallVec")
    }

    pub(super) fn should_lower_swap_method_call_to_index_swap(&self, receiver: &syn::Expr) -> bool {
        if matches!(self.peel_paren_group_expr(receiver), syn::Expr::Path(path)
            if path.path.segments.len() == 1 && path.path.segments[0].ident == "self")
        {
            if let Some(current_struct) = self.current_struct.as_ref() {
                return matches!(
                    current_struct.as_str(),
                    "Vec" | "VecDeque" | "ArrayVec" | "SmallVec" | "DeArray"
                );
            }
        }

        if self.expr_lowers_to_slice_or_span_view(receiver) {
            return true;
        }

        let Some(receiver_ty) = self
            .infer_simple_expr_type(receiver)
            .or_else(|| self.infer_local_binding_type_from_initializer(receiver))
        else {
            return false;
        };
        let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
        if self.type_is_slice_or_span_like(receiver_ty) {
            return true;
        }

        let syn::Type::Path(tp) = receiver_ty else {
            return false;
        };
        let Some(last) = tp.path.segments.last() else {
            return false;
        };
        let receiver_name = if last.ident == "Self" {
            self.current_struct
                .as_ref()
                .cloned()
                .unwrap_or_else(|| "Self".to_string())
        } else {
            last.ident.to_string()
        };
        matches!(
            receiver_name.as_str(),
            "Vec" | "VecDeque" | "ArrayVec" | "SmallVec" | "DeArray"
        )
    }

    pub(super) fn should_lower_index_method_call_to_index_op(&self, receiver: &syn::Expr) -> bool {
        if matches!(self.peel_paren_group_expr(receiver), syn::Expr::Path(path)
            if path.path.segments.len() == 1 && path.path.segments[0].ident == "self")
        {
            if let Some(current_struct) = self.current_struct.as_ref() {
                return matches!(
                    current_struct.as_str(),
                    "Vec" | "VecDeque" | "ArrayVec" | "SmallVec" | "DeArray"
                );
            }
        }

        if self.receiver_is_fixed_array_like_expr(receiver) {
            return true;
        }

        if self.expr_lowers_to_slice_or_span_view(receiver) {
            return true;
        }

        let Some(receiver_ty) = self
            .infer_simple_expr_type(receiver)
            .or_else(|| self.infer_local_binding_type_from_initializer(receiver))
        else {
            return false;
        };
        let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
        if self.type_is_slice_or_span_like(receiver_ty) {
            return true;
        }

        let syn::Type::Path(tp) = receiver_ty else {
            return false;
        };
        let Some(last) = tp.path.segments.last() else {
            return false;
        };
        let receiver_name = if last.ident == "Self" {
            self.current_struct
                .as_ref()
                .cloned()
                .unwrap_or_else(|| "Self".to_string())
        } else {
            last.ident.to_string()
        };
        let mut receiver_name_candidates = vec![receiver_name.clone()];
        if matches!(
            receiver_name.as_str(),
            "Box" | "Rc" | "Arc" | "Pin" | "Cow" | "Cell" | "RefCell" | "ManuallyDrop"
        ) && let syn::PathArguments::AngleBracketed(args) = &last.arguments
            && let Some(inner_ty) = args.args.iter().find_map(|arg| match arg {
                syn::GenericArgument::Type(ty) => Some(ty),
                _ => None,
            })
            && let syn::Type::Path(inner_tp) = self.peel_reference_paren_group_type(inner_ty)
            && let Some(inner_last) = inner_tp.path.segments.last()
        {
            receiver_name_candidates.push(inner_last.ident.to_string());
        }
        receiver_name_candidates.iter().any(|name| {
            matches!(
                name.as_str(),
                "Vec" | "VecDeque" | "ArrayVec" | "SmallVec" | "DeArray"
            )
        })
    }

    pub(super) fn should_lower_unknown_local_index_method_call(
        &self,
        receiver: &syn::Expr,
        arg: &syn::Expr,
    ) -> bool {
        let receiver = self.peel_paren_group_expr(receiver);
        let syn::Expr::Path(path) = receiver else {
            return false;
        };
        if path.path.segments.len() != 1 {
            return false;
        }
        let local_name = path.path.segments[0].ident.to_string();
        if !self.is_local_binding_in_scope(&local_name)
            && !local_name
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_lowercase() || ch == '_')
        {
            return false;
        }

        let arg = self.peel_paren_group_expr(arg);
        if self.is_slice_range_index_expr(arg) {
            return true;
        }
        if let syn::Expr::Unary(unary) = arg
            && matches!(unary.op, syn::UnOp::Deref(_))
        {
            return true;
        }
        if matches!(
            arg,
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(_),
                ..
            })
        ) {
            return true;
        }
        if matches!(
            arg,
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(_),
                ..
            })
        ) {
            return false;
        }
        let Some(arg_ty) = self
            .infer_simple_expr_type(arg)
            .or_else(|| self.infer_local_binding_type_from_initializer(arg))
            .or_else(|| {
                let syn::Expr::Path(path_expr) = arg else {
                    return None;
                };
                if path_expr.path.segments.len() != 1 {
                    return None;
                }
                let raw = path_expr.path.segments[0].ident.to_string();
                let candidate = raw.strip_suffix('_')?;
                self.lookup_local_binding_type(candidate)
            })
        else {
            if let syn::Expr::Path(path_expr) = arg
                && path_expr.path.segments.len() == 1
            {
                let name = path_expr.path.segments[0].ident.to_string();
                if name == "idx" || name == "index" || name.contains("index") {
                    return true;
                }
            }
            return false;
        };
        let arg_ty = self.peel_reference_paren_group_type(&arg_ty);
        if self.is_known_string_like_type(arg_ty) {
            return false;
        }
        if self.is_known_scalar_like_type(arg_ty) {
            return true;
        }
        let mapped = self.map_type(arg_ty);
        let canonical = mapped
            .chars()
            .filter(|c| !c.is_ascii_whitespace())
            .collect::<String>();
        canonical.starts_with("rusty::range<")
            || canonical.starts_with("rusty::range_from<")
            || canonical.starts_with("rusty::range_inclusive<")
            || canonical.starts_with("rusty::range_to<")
    }

    pub(super) fn should_lower_swap_method_call_via_deref_mut_view(&self, receiver: &syn::Expr) -> bool {
        if matches!(self.peel_paren_group_expr(receiver), syn::Expr::Path(path)
            if path.path.segments.len() == 1 && path.path.segments[0].ident == "self")
        {
            return self.current_struct.as_ref().is_some_and(|current_struct| {
                current_struct
                    .rsplit("::")
                    .next()
                    .is_some_and(|tail| tail == "SmallVec")
            });
        }

        let Some(receiver_ty) = self.infer_simple_expr_type(receiver) else {
            return false;
        };
        let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
        let syn::Type::Path(tp) = receiver_ty else {
            return false;
        };
        let Some(last) = tp.path.segments.last() else {
            return false;
        };
        let receiver_name = if last.ident == "Self" {
            self.current_struct
                .as_ref()
                .cloned()
                .unwrap_or_else(|| "Self".to_string())
        } else {
            last.ident.to_string()
        };
        receiver_name == "SmallVec"
    }

    pub(super) fn is_scalar_into_target_cpp_type(canonical_cpp_ty: &str) -> bool {
        matches!(
            canonical_cpp_ty,
            "int8_t"
                | "int16_t"
                | "int32_t"
                | "int64_t"
                | "__int128"
                | "uint8_t"
                | "uint16_t"
                | "uint32_t"
                | "uint64_t"
                | "unsigned__int128"
                | "size_t"
                | "ptrdiff_t"
                | "float"
                | "double"
                | "longdouble"
                | "bool"
                | "char"
                | "char8_t"
                | "char16_t"
                | "char32_t"
                | "wchar_t"
        )
    }

    pub(super) fn should_collapse_reborrow_of_deref_operand(&self, operand: &syn::Expr) -> bool {
        if self.is_expr_reference_like(operand) {
            return true;
        }
        if self.is_expr_raw_pointer_like(operand) {
            return true;
        }
        match operand {
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                let name = path.path.segments[0].ident.to_string();
                if let Some(ty) = self.lookup_local_binding_type(&name) {
                    !matches!(ty, syn::Type::Ptr(_))
                } else {
                    false
                }
            }
            syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
                self.should_collapse_reborrow_of_deref_operand(&unary.expr)
            }
            syn::Expr::Paren(p) => self.should_collapse_reborrow_of_deref_operand(&p.expr),
            syn::Expr::Group(g) => self.should_collapse_reborrow_of_deref_operand(&g.expr),
            _ => false,
        }
    }

    pub(super) fn is_serde_json_formatter_default_method(method_name: &str) -> bool {
        matches!(
            method_name,
            "write_null"
                | "write_bool"
                | "write_i8"
                | "write_i16"
                | "write_i32"
                | "write_i64"
                | "write_i128"
                | "write_u8"
                | "write_u16"
                | "write_u32"
                | "write_u64"
                | "write_u128"
                | "write_f32"
                | "write_f64"
                | "write_number_str"
                | "begin_string"
                | "end_string"
                | "write_string_fragment"
                | "write_char_escape"
                | "write_byte_array"
                | "begin_array"
                | "end_array"
                | "begin_array_value"
                | "end_array_value"
                | "begin_object"
                | "end_object"
                | "begin_object_key"
                | "end_object_key"
                | "begin_object_value"
                | "end_object_value"
                | "write_raw_fragment"
        )
    }

    pub(super) fn should_lower_char_is_whitespace_method_call(&self, receiver: &syn::Expr) -> bool {
        if self.expr_is_char_like(receiver) {
            return true;
        }
        let Some(name) = extract_simple_local_ident(receiver) else {
            return false;
        };
        self.should_lower_char_predicate_on_untyped_closure_param(&name)
    }

    pub(super) fn should_coerce_self_path_to_deref_mut(
        &self,
        path: &syn::Path,
        expected_ty: Option<&syn::Type>,
    ) -> bool {
        if !Self::path_is_simple_self(path) {
            return false;
        }
        let expected_ty = match expected_ty {
            Some(ty) => self.peel_paren_group_type(ty),
            None => return false,
        };
        let syn::Type::Reference(expected_ref) = expected_ty else {
            return false;
        };
        if expected_ref.mutability.is_none() {
            return false;
        }
        if self.type_is_current_struct_self_type(&expected_ref.elem) {
            return false;
        }
        self.lookup_current_struct_method_return_type("deref_mut")
            .is_some()
    }

    pub(super) fn should_coerce_self_path_to_deref(
        &self,
        path: &syn::Path,
        expected_ty: Option<&syn::Type>,
    ) -> bool {
        if !Self::path_is_simple_self(path) {
            return false;
        }
        let expected_ty = match expected_ty {
            Some(ty) => self.peel_paren_group_type(ty),
            None => return false,
        };
        let syn::Type::Reference(expected_ref) = expected_ty else {
            return false;
        };
        if expected_ref.mutability.is_some() {
            return false;
        }
        if self.type_is_current_struct_self_type(&expected_ref.elem) {
            return false;
        }
        self.lookup_current_struct_method_return_type("deref")
            .is_some()
    }

    pub(super) fn should_suppress_inferred_expected_for_struct_literal(
        &self,
        expr: &syn::Expr,
        inferred_ty: &syn::Type,
    ) -> bool {
        let syn::Expr::Struct(struct_expr) = self.peel_paren_group_expr(expr) else {
            return false;
        };
        if !self.expected_type_matches_struct_literal_path(inferred_ty, &struct_expr.path) {
            return false;
        }
        let inferred_ty = self.peel_reference_paren_group_type(inferred_ty);
        let syn::Type::Path(tp) = inferred_ty else {
            return false;
        };
        if tp.qself.is_some() {
            return false;
        }
        let inferred_omits_all_args = tp
            .path
            .segments
            .iter()
            .all(|seg| matches!(seg.arguments, syn::PathArguments::None));
        if !inferred_omits_all_args {
            return false;
        }
        struct_expr
            .path
            .segments
            .iter()
            .all(|seg| matches!(seg.arguments, syn::PathArguments::None))
    }

    pub(super) fn is_u8_syn_type(ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(tp) => {
                if tp
                    .path
                    .segments
                    .last()
                    .is_some_and(|seg| matches!(seg.ident.to_string().as_str(), "u8" | "uint8_t"))
                {
                    return true;
                }
                if tp
                    .path
                    .segments
                    .last()
                    .is_some_and(|seg| seg.ident == "Item")
                    && let Some(qself) = &tp.qself
                {
                    return Self::is_u8_assoc_item_owner(qself.ty.as_ref());
                }
                false
            }
            syn::Type::Paren(p) => Self::is_u8_syn_type(&p.elem),
            syn::Type::Group(g) => Self::is_u8_syn_type(&g.elem),
            _ => false,
        }
    }

    pub(super) fn is_u8_assoc_item_owner(ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Array(arr) => Self::is_u8_syn_type(&arr.elem),
            syn::Type::Reference(r) => Self::is_u8_assoc_item_owner(&r.elem),
            syn::Type::Paren(p) => Self::is_u8_assoc_item_owner(&p.elem),
            syn::Type::Group(g) => Self::is_u8_assoc_item_owner(&g.elem),
            _ => false,
        }
    }

    pub(super) fn is_compound_assign_binop(op: &syn::BinOp) -> bool {
        matches!(
            op,
            syn::BinOp::AddAssign(_)
                | syn::BinOp::SubAssign(_)
                | syn::BinOp::MulAssign(_)
                | syn::BinOp::DivAssign(_)
                | syn::BinOp::RemAssign(_)
                | syn::BinOp::BitXorAssign(_)
                | syn::BinOp::BitAndAssign(_)
                | syn::BinOp::BitOrAssign(_)
                | syn::BinOp::ShlAssign(_)
                | syn::BinOp::ShrAssign(_)
        )
    }

    pub(super) fn is_option_none_path(&self, path: &syn::Path) -> bool {
        if Self::path_is_option_none(path) {
            return true;
        }
        if path.segments.len() >= 2
            && let Some(owner) = path.segments.iter().nth_back(1)
            && let Some(last) = path.segments.last()
            && self.canonical_variant_name(&last.ident.to_string()) == "None"
            && owner.ident.to_string().starts_with("__private")
        {
            return true;
        }
        if path.segments.len() != 1 {
            return false;
        }
        let Some(last) = path.segments.last() else {
            return false;
        };
        let raw = last.ident.to_string();
        let canonical = self.canonical_variant_name(&raw).to_string();
        if canonical != "None" {
            return false;
        }
        let known_non_option_owner = self
            .unique_data_enum_name_for_variant_name(&raw)
            .or_else(|| self.unique_data_enum_name_for_variant_name(&canonical))
            .is_some_and(|owner| owner != "Option");
        !known_non_option_owner
    }

    pub(super) fn is_option_some_path(&self, path: &syn::Path) -> bool {
        if Self::path_is_option_some(path) {
            return true;
        }
        if path.segments.len() >= 2
            && let Some(owner) = path.segments.iter().nth_back(1)
            && let Some(last) = path.segments.last()
            && self.canonical_variant_name(&last.ident.to_string()) == "Some"
            && owner.ident.to_string().starts_with("__private")
        {
            return true;
        }
        if path.segments.len() != 1 {
            return false;
        }
        let Some(last) = path.segments.last() else {
            return false;
        };
        let raw = last.ident.to_string();
        let canonical = self.canonical_variant_name(&raw).to_string();
        if canonical != "Some" {
            return false;
        }
        let known_non_option_owner = self
            .unique_data_enum_name_for_variant_name(&raw)
            .or_else(|| self.unique_data_enum_name_for_variant_name(&canonical))
            .is_some_and(|owner| owner != "Option");
        !known_non_option_owner
    }

    /// Returns true if `ty` looks like a type parameter (e.g., `T`, `F`, `E`).
    /// These are single-segment paths with an uppercase-first-identifier — Rust's
    /// convention for type parameter names. Type parameters are not valid C++
    /// template arguments, so we skip them and fall through to decltype inference.
    pub(super) fn is_type_parameter(ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(tp) => {
                tp.qself.is_none()
                    && tp.path.segments.len() == 1
                    && tp.path.segments[0]
                        .ident
                        .to_string()
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_uppercase())
            }
            syn::Type::Reference(r) => Self::is_type_parameter(&r.elem),
            syn::Type::Paren(p) => Self::is_type_parameter(&p.elem),
            syn::Type::Group(g) => Self::is_type_parameter(&g.elem),
            _ => false,
        }
    }

    pub(super) fn is_iterator_like_receiver_expr(&self, expr: &syn::Expr) -> bool {
        self.infer_iter_item_type_from_expr(expr).is_some()
    }

    pub(super) fn is_std_optional_like_receiver_expr(&self, expr: &syn::Expr) -> bool {
        if self
            .infer_simple_expr_type(expr)
            .is_some_and(|ty| self.is_std_optional_syn_type(&ty))
        {
            return true;
        }
        let expr = self.peel_paren_group_expr(expr);
        let syn::Expr::MethodCall(mc) = expr else {
            return false;
        };
        if !matches!(mc.method.to_string().as_str(), "next" | "next_back") || !mc.args.is_empty() {
            return false;
        }
        if self.is_iterator_like_receiver_expr(&mc.receiver) {
            return true;
        }
        let receiver = self.peel_paren_group_expr(&mc.receiver);
        if let syn::Expr::MethodCall(inner) = receiver {
            return inner.method == "into_iter" && inner.args.is_empty();
        }
        if let syn::Expr::Path(path) = receiver {
            if path.path.segments.len() == 1 {
                let name = path.path.segments[0].ident.to_string();
                if self.lookup_local_binding_cpp_name(&name).is_some()
                    && self.lookup_local_binding_type(&name).is_none()
                {
                    return true;
                }
            }
        }
        false
    }

    pub(super) fn is_to_vec_runtime_receiver_expr(&self, expr: &syn::Expr) -> bool {
        let expr = self.peel_paren_group_expr(expr);
        if matches!(
            expr,
            syn::Expr::Array(_)
                | syn::Expr::Repeat(_)
                | syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::ByteStr(_),
                    ..
                })
        ) {
            return true;
        }
        if let syn::Expr::Reference(reference) = expr {
            return self.is_to_vec_runtime_receiver_expr(&reference.expr);
        }
        if let syn::Expr::Cast(cast) = expr {
            return self.is_to_vec_runtime_receiver_expr(&cast.expr);
        }
        self.infer_simple_expr_type(expr)
            .is_some_and(|ty| self.is_to_vec_runtime_receiver_type(&ty))
    }

    pub(super) fn is_to_vec_runtime_receiver_type(&self, ty: &syn::Type) -> bool {
        let ty = self.peel_reference_paren_group_type(ty);
        match ty {
            syn::Type::Array(_) | syn::Type::Slice(_) => true,
            syn::Type::Path(tp) => tp
                .path
                .segments
                .last()
                .is_some_and(|seg| seg.ident == "ArrayVec"),
            _ => false,
        }
    }

    pub(super) fn is_string_from_call_expr(&self, expr: &syn::Expr) -> bool {
        let syn::Expr::Call(call) = expr else {
            return false;
        };
        let syn::Expr::Path(path) = call.func.as_ref() else {
            return false;
        };
        let joined = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        matches!(
            joined.as_str(),
            "String::from" | "std::string::String::from" | "alloc::string::String::from"
        )
    }

    pub(super) fn is_core_from_path_expr(&self, expr: &syn::Expr) -> bool {
        let syn::Expr::Path(path) = expr else {
            return false;
        };
        let joined = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        matches!(
            joined.as_str(),
            "From::from" | "core::convert::From::from" | "std::convert::From::from"
        )
    }

    pub(super) fn is_default_trait_default_path_expr(&self, expr: &syn::Expr) -> bool {
        let syn::Expr::Path(path) = expr else {
            return false;
        };
        let joined = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        matches!(
            joined.as_str(),
            "Default::default"
                | "core::default::Default::default"
                | "std::default::Default::default"
        )
    }

    pub(super) fn is_default_value_shorthand_path_expr(&self, expr: &syn::Expr) -> bool {
        let syn::Expr::Path(path) = expr else {
            return false;
        };
        if path.path.segments.len() != 1 {
            return false;
        }
        let name = path.path.segments[0].ident.to_string();
        if !matches!(name.as_str(), "default" | "default_") {
            return false;
        }
        // Respect in-scope local bindings/functions when they exist.
        if self.lookup_local_binding_cpp_name(&name).is_some() {
            return false;
        }
        if self.is_local_function_name_in_scope(&name) {
            return false;
        }
        true
    }

    pub(super) fn is_noreturn_panic_like_call_path(&self, path: &str) -> bool {
        matches!(
            path,
            "rusty::panicking::panic"
                | "rusty::panicking::panic_fmt"
                | "rusty::panicking::assert_failed"
                | "rusty::intrinsics::unreachable"
        )
    }

    pub(super) fn is_ufcs_io_write_fmt_call_path(&self, function_path: &str) -> bool {
        if !function_path.ends_with("::Write::write_fmt") {
            return false;
        }
        function_path.starts_with("io::") || function_path.contains("::io::")
    }

    pub(super) fn is_concrete_cpp_type_for_iflet_init(mapped: &str) -> bool {
        mapped != "auto" && !type_string_has_auto_placeholder(mapped) && !mapped.contains("/* TODO")
    }

    pub(super) fn is_probably_iterator_receiver_expr(&self, expr: &syn::Expr) -> bool {
        let expr = self.peel_paren_group_expr(expr);
        match expr {
            syn::Expr::MethodCall(mc) => {
                let method = mc.method.to_string();
                matches!(
                    method.as_str(),
                    "iter"
                        | "iter_mut"
                        | "into_iter"
                        | "drain"
                        | "iter_names"
                        | "bytes"
                        | "as_bytes"
                        | "chars"
                        | "map"
                        | "filter"
                        | "filter_map"
                        | "enumerate"
                        | "rev"
                        | "take"
                        | "skip"
                        | "scan"
                        | "split"
                ) || self.is_probably_iterator_receiver_expr(&mc.receiver)
            }
            syn::Expr::Call(call) => {
                let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(call.func.as_ref())
                else {
                    return false;
                };
                let last = path_expr
                    .path
                    .segments
                    .last()
                    .map(|seg| seg.ident.to_string());
                if matches!(
                    last.as_deref(),
                    Some(
                        "iter"
                            | "iter_mut"
                            | "into_iter"
                            | "drain"
                            | "iter_names"
                            | "bytes"
                            | "as_bytes"
                            | "chars"
                            | "map"
                            | "filter"
                            | "filter_map"
                            | "enumerate"
                            | "rev"
                            | "take"
                            | "skip"
                            | "scan"
                            | "split"
                    )
                ) {
                    return true;
                }
                if path_expr.path.segments.len() == 1 {
                    let binding_name = path_expr.path.segments[0].ident.to_string();
                    if let Some(callee_ty) = self.lookup_local_binding_type(&binding_name) {
                        if let Some(return_ty) =
                            self.extract_callable_return_type_from_type(&callee_ty)
                        {
                            if self.extract_iter_item_type_from_type(&return_ty).is_some()
                                || self.type_has_iterator_surface(&return_ty)
                            {
                                return true;
                            }
                        }
                    }
                }
                let joined = path_expr
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                matches!(
                    joined.as_str(),
                    "iter"
                        | "rusty::iter"
                        | "iter_mut"
                        | "rusty::iter_mut"
                        | "map"
                        | "rusty::map"
                        | "filter"
                        | "rusty::filter"
                        | "filter_map"
                        | "rusty::filter_map"
                        | "enumerate"
                        | "rusty::enumerate"
                        | "rev"
                        | "rusty::rev"
                        | "take"
                        | "rusty::take"
                        | "skip"
                        | "rusty::skip"
                        | "scan"
                        | "rusty::scan"
                        | "split"
                        | "str_runtime::split"
                        | "rusty::str_runtime::split"
                )
            }
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                let name = path.path.segments[0].ident.to_string();
                self.lookup_local_binding_type(&name)
                    .as_ref()
                    .is_some_and(|ty| {
                        self.extract_iter_item_type_from_type(ty).is_some()
                            || self.type_has_iterator_surface(ty)
                    })
            }
            _ => false,
        }
    }

    pub(super) fn should_bridge_into_iter_receiver_to_iter(&self, receiver: &syn::Expr) -> bool {
        if self.receiver_is_fixed_array_like_expr(receiver) {
            return true;
        }
        if let Some(receiver_ty) = self.infer_simple_expr_type(receiver) {
            let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
            let receiver_cpp = self.map_type(receiver_ty);
            if receiver_cpp.starts_with("rusty::Vec<") || receiver_cpp.starts_with("std::span<") {
                return true;
            }
        }
        if self.is_iterator_like_receiver_expr(receiver)
            || self.is_probably_iterator_receiver_expr(receiver)
        {
            return false;
        }
        let Some(receiver_ty) = self.infer_simple_expr_type(receiver) else {
            // Preserve historical fallback behavior for unresolved receiver shapes.
            return true;
        };
        let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
        let syn::Type::Path(tp) = receiver_ty else {
            return false;
        };
        if tp.qself.is_some() || tp.path.segments.len() != 1 {
            return false;
        }
        self.is_type_param_in_scope(&tp.path.segments[0].ident.to_string())
    }

    pub(super) fn should_bridge_direct_into_iter_receiver_to_iter(&self, receiver: &syn::Expr) -> bool {
        if self.receiver_is_fixed_array_like_expr(receiver) {
            return true;
        }
        if let Some(receiver_ty) = self.infer_simple_expr_type(receiver) {
            let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
            let receiver_cpp = self.map_type(receiver_ty);
            // `Vec::new()` whose element type is not resolvable context-free
            // infers to a bare `rusty::Vec` (no args); its `.into_iter()` still
            // bridges through `rusty::iter(...)` like any other Vec receiver.
            if receiver_cpp.starts_with("rusty::Vec<")
                || receiver_cpp == "rusty::Vec"
                || receiver_cpp.starts_with("std::span<")
            {
                return true;
            }
        }
        if self.is_iterator_like_receiver_expr(receiver)
            || self.is_probably_iterator_receiver_expr(receiver)
        {
            return false;
        }
        let Some(receiver_ty) = self.infer_simple_expr_type(receiver) else {
            // Unknown direct `.into_iter()` receiver types are usually generic or
            // pattern-introduced values without a concrete member surface.
            // Bridge through `rusty::iter(...)` to keep call sites compilable.
            return true;
        };
        let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
        let syn::Type::Path(tp) = receiver_ty else {
            return false;
        };
        if tp.qself.is_some() || tp.path.segments.len() != 1 {
            return false;
        }
        self.is_type_param_in_scope(&tp.path.segments[0].ident.to_string())
    }

    pub(super) fn is_ordering_like_type(&self, ty: &syn::Type) -> bool {
        let mapped = self.map_type(ty);
        mapped == "Ordering" || mapped == "rusty::cmp::Ordering" || mapped.ends_with("::Ordering")
    }

    pub(super) fn is_ordering_then_with_receiver_shape(&self, expr: &syn::Expr) -> bool {
        match self.peel_paren_group_expr(expr) {
            syn::Expr::MethodCall(inner) => {
                (inner.method == "cmp" && inner.args.len() == 1)
                    || (inner.method == "then_with" && inner.args.len() == 1)
            }
            syn::Expr::Call(call) => {
                let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(&call.func) else {
                    return false;
                };
                path_expr
                    .path
                    .segments
                    .last()
                    .is_some_and(|seg| seg.ident == "cmp" || seg.ident == "then_with")
            }
            _ => false,
        }
    }

    pub(super) fn is_slice_range_index_expr(&self, index: &syn::Expr) -> bool {
        matches!(self.peel_paren_group_expr(index), syn::Expr::Range(_))
    }

    pub(super) fn is_range_expression(expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Range(_) => true,
            syn::Expr::Paren(p) => Self::is_range_expression(&p.expr),
            _ => false,
        }
    }

    pub(super) fn should_force_size_t_visit_return_for_bound_match(
        &self,
        match_expr: &syn::ExprMatch,
        expected_ty: Option<&syn::Type>,
    ) -> bool {
        if expected_ty.is_some() {
            return false;
        }
        let syn::Expr::MethodCall(mc) = self.peel_paren_group_expr(&match_expr.expr) else {
            return false;
        };
        matches!(mc.method.to_string().as_str(), "start_bound" | "end_bound")
    }

    pub(super) fn should_elide_in_scope_local_alias_type_args(
        &self,
        path: &syn::Path,
        args: &syn::AngleBracketedGenericArguments,
    ) -> bool {
        if path.leading_colon.is_some() || path.segments.len() != 1 || args.args.is_empty() {
            return false;
        }
        if self.block_depth == 0 {
            return false;
        }
        let Some(seg) = path.segments.first() else {
            return false;
        };
        let local_name = seg.ident.to_string();
        if !self.is_local_type_name_in_scope(&local_name) {
            return false;
        }
        // Keep real struct/enum template instantiations intact.
        if self.struct_field_order.contains_key(&local_name)
            || self.tuple_struct_arities.contains_key(&local_name)
            || self.data_enum_types.contains(&local_name)
        {
            return false;
        }
        if let Some(type_key) = self.declared_type_key_for_path(path)
            && let Some(params) = self.declared_type_params.get(&type_key)
        {
            if params.is_empty() || params.len() != args.args.len() {
                return false;
            }
            let param_kinds = self.declared_type_param_kinds.get(&type_key);
            return args.args.iter().enumerate().all(|(idx, arg)| {
                let param = &params[idx];
                if !self.is_type_param_in_scope(param) {
                    return false;
                }
                let expected_kind = param_kinds.and_then(|kinds| kinds.get(idx));
                match (expected_kind, arg) {
                    (Some(GenericParamKind::Type), syn::GenericArgument::Type(ty))
                    | (None, syn::GenericArgument::Type(ty)) => self
                        .extract_simple_type_param_name(ty)
                        .is_some_and(|name| name == *param),
                    (Some(GenericParamKind::Const), syn::GenericArgument::Const(expr))
                    | (None, syn::GenericArgument::Const(expr)) => self
                        .extract_simple_const_param_name(expr)
                        .is_some_and(|name| name == *param),
                    _ => false,
                }
            });
        }
        args.args.iter().all(|arg| match arg {
            syn::GenericArgument::Type(ty) => self
                .extract_simple_type_param_name(ty)
                .is_some_and(|name| self.is_type_param_in_scope(&name)),
            syn::GenericArgument::Const(expr) => self
                .extract_simple_const_param_name(expr)
                .is_some_and(|name| self.is_type_param_in_scope(&name)),
            _ => false,
        })
    }

    pub(super) fn should_elide_shadowed_current_struct_local_type_args(
        &self,
        path: &syn::Path,
        args: &syn::AngleBracketedGenericArguments,
    ) -> bool {
        // Inside a generic impl body, associated aliases emitted into the current
        // struct (`using IntoIter = ...;`) can shadow single-segment generic spellings
        // from Rust (`IntoIter<T, CAP>`). In C++, the alias itself is not a template,
        // so keep only the alias name in this specific shape.
        if path.leading_colon.is_some() || path.segments.len() != 1 {
            return false;
        }
        if args.args.is_empty() {
            return false;
        }
        let Some(seg) = path.segments.first() else {
            return false;
        };
        self.current_struct_assoc_alias_exists(&seg.ident.to_string())
    }

    pub(super) fn is_unit_struct_path(&self, path: &syn::Path) -> bool {
        let joined = path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if self.unit_struct_types.contains(&joined) {
            return true;
        }
        if let Some(last) = path.segments.last() {
            if last.ident == "Self"
                && let Some(current) = &self.current_struct
            {
                let scoped_current = self.scoped_type_key(current);
                if self.unit_struct_types.contains(current)
                    || self.unit_struct_types.contains(&scoped_current)
                {
                    return true;
                }
            }
            if self.unit_struct_types.contains(&last.ident.to_string()) {
                return true;
            }
        }
        false
    }

    pub(super) fn should_elide_shadowed_current_struct_local_recovered_args(
        &self,
        type_key: &str,
        params: &[String],
        recovered_args: &[String],
    ) -> bool {
        if params.is_empty() || params.len() != recovered_args.len() {
            return false;
        }
        let Some(current_struct) = self.current_struct.as_ref() else {
            return false;
        };
        let base = type_key.rsplit("::").next().unwrap_or(type_key);
        let scoped_key = format!("{}::{}", current_struct, base);
        let Some(scoped_params) = self.declared_type_params.get(&scoped_key) else {
            return false;
        };
        if scoped_params != params {
            return false;
        }
        params
            .iter()
            .zip(recovered_args.iter())
            .all(|(param, recovered)| self.is_type_param_in_scope(param) && param == recovered)
    }

    pub(super) fn should_sanitize_array_capacity_expr(&self, len_expr: &syn::Expr, len_cpp: &str) -> bool {
        if self.should_sanitize_array_capacity_cpp_len(len_cpp) {
            return true;
        }
        matches!(self.peel_paren_group_expr(len_expr), syn::Expr::Path(_))
    }

    pub(super) fn should_sanitize_array_capacity_cpp_len(&self, len_cpp: &str) -> bool {
        let trimmed = len_cpp.trim();
        if trimmed.contains("rusty::sanitize_array_capacity<") {
            return false;
        }
        if trimmed.contains("std::numeric_limits<size_t>::max()") {
            return true;
        }
        if trimmed.is_empty() || trimmed.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | ':'))
    }

    pub(super) fn should_force_move_consumed_local_initializer_expr(&self, expr: &syn::Expr) -> bool {
        let Some(local_name) = extract_simple_local_ident(expr) else {
            return false;
        };
        self.lookup_local_binding_type(&local_name)
            .is_some_and(|ty| matches!(ty, syn::Type::Reference(_)))
    }

    pub(super) fn is_associated_const_value_path(&self, path: &syn::Path) -> bool {
        if path.segments.len() < 2 {
            return false;
        }
        let Some(owner_seg) = path.segments.iter().nth_back(1) else {
            return false;
        };
        let Some(last_seg) = path.segments.last() else {
            return false;
        };
        let owner = owner_seg.ident.to_string();
        let member = last_seg.ident.to_string();
        owner.chars().next().is_some_and(|c| c.is_uppercase())
            && !member.is_empty()
            && member
                .chars()
                .all(|c| c.is_uppercase() || c.is_ascii_digit() || c == '_')
    }

    /// Determine whether an expression represents a local variable that should
    /// be wrapped in std::move() when used by value.
    pub(super) fn should_insert_move(&self, expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Path(path) => {
                // Multi-segment associated const values are handled separately via
                // `rusty::clone(...)` in `emit_expr_maybe_move(...)`.
                if path.path.segments.len() > 1 {
                    return false;
                }

                let name = path.path.segments[0].ident.to_string();

                // By-value receiver methods treat `self` as a consumable value path.
                // Keep reference receivers unchanged to avoid moving borrowed `self`.
                if name == "self" {
                    return !self.current_self_receiver_is_reference();
                }

                // Skip keywords and special names
                if matches!(
                    name.as_str(),
                    "Self" | "true" | "false" | "None" | "Some" | "Ok" | "Err"
                ) {
                    return false;
                }
                if matches!(name.as_str(), "formatter") {
                    return false;
                }

                // Skip ALL_CAPS names (likely constants)
                if name.chars().all(|c| c.is_uppercase() || c == '_') && name.len() > 1 {
                    return false;
                }

                // Skip names that start with uppercase (likely type names or enum variants)
                if name.starts_with(|c: char| c.is_uppercase()) {
                    return false;
                }

                // Borrowed bindings (`&T` / `&mut T`) should not be moved.
                if let Some(local_ty) = self.lookup_local_binding_type(&name) {
                    if matches!(
                        self.peel_paren_group_type(&local_ty),
                        syn::Type::Reference(_) | syn::Type::Ptr(_)
                    ) || self.reference_type_lowers_to_value_cpp(&local_ty)
                    {
                        return false;
                    }
                    let mapped_local_ty = self.map_type(&local_ty);
                    // The `'&' anywhere` heuristic catches type aliases that
                    // lower to a reference, but it also falsely matches the
                    // `&` inside a function-signature template arg, e.g.
                    // `rusty::Function<void(BinaryWriteArchive&)>` — that's an
                    // owned, move-only value type, not a reference, and the
                    // caller should still be moved into a `Some(...)` /
                    // constructor call. Restrict the check to a top-level
                    // trailing `&` (`T&` / `T const&`), which is what a real
                    // C++ reference type looks like.
                    if mapped_local_ty.trim_end().ends_with('&')
                        || mapped_local_ty.starts_with("std::span<")
                        || mapped_local_ty.starts_with("std::array<")
                        || (name == "f" && mapped_local_ty.contains("Formatter"))
                    {
                        return false;
                    }
                } else if name == "f" {
                    return false;
                }

                // For non-Copy types (structs containing Vec, String, etc.), always insert
                // std::move. In Rust, `let x = y` where y: T (Clone but not Copy) calls Clone,
                // not a hypothetical copy operation. In C++, the copy constructor may be deleted
                // or invalid (e.g., for structs containing Vec). Using std::move() calls the
                // move constructor which is valid for all types and in C++ effectively calls
                // the same clone/move semantics as Rust.

                true
            }
            syn::Expr::Field(field) => {
                if self
                    .infer_simple_expr_type(&field.base)
                    .is_some_and(|ty| matches!(ty, syn::Type::Reference(_)))
                {
                    return false;
                }
                self.should_insert_move(&field.base)
            }
            _ => false,
        }
    }

    /// Check if an initializer expression is a reference (`&expr` or `&mut expr`).
    pub(super) fn is_ref_init(&self, expr: &syn::Expr) -> bool {
        let expr = self.peel_paren_group_expr(expr);
        let syn::Expr::Reference(reference_expr) = expr else {
            return false;
        };
        if self
            .infer_local_binding_type_from_initializer(expr)
            .as_ref()
            .is_some_and(|ty| self.reference_type_lowers_to_value_cpp(ty))
        {
            return false;
        }
        if let syn::Expr::Index(index_expr) = self.peel_paren_group_expr(&reference_expr.expr)
            && matches!(
                self.peel_paren_group_expr(&index_expr.index),
                syn::Expr::Range(_)
            )
        {
            return false;
        }
        // Rust slice borrows (`&arr[..n]`) lower to span-by-value in C++.
        // Keep these as value locals, not C++ reference bindings to temporaries.
        !self.expr_lowers_to_slice_or_span_view(&reference_expr.expr)
    }

    /// Check if an expression is an rvalue (temporary/function-call result)
    /// rather than an lvalue (variable/field access).
    pub(super) fn is_rvalue_expr(&self, expr: &syn::Expr) -> bool {
        matches!(
            expr,
            syn::Expr::Call(_)
                | syn::Expr::MethodCall(_)
                | syn::Expr::Lit(_)
                | syn::Expr::Struct(_)
                | syn::Expr::Tuple(_)
                | syn::Expr::Array(_)
                | syn::Expr::Binary(_)
                | syn::Expr::Unary(_)
                | syn::Expr::If(_)
                | syn::Expr::Match(_)
                | syn::Expr::Block(_)
                | syn::Expr::Closure(_)
        )
    }

    // In module mode, trait-object error surfaces like `Box<dyn Error>` are
    // currently erased to `void*`. For closure lambdas with `auto` return type,
    // forcing that erased Result hint breaks `?` lowering (`RUSTY_TRY_INTO` can
    // no longer convert concrete errors into `void*`). In that case, keep the
    // legacy inferred Result constructor path by skipping the explicit hint.
    pub(super) fn should_push_return_type_hint_for_closure(&self, output: &syn::ReturnType) -> bool {
        let syn::ReturnType::Type(_, ty) = output else {
            return true;
        };
        let Some(err_ty) = self.expected_result_type_arg(Some(ty), 1) else {
            return true;
        };
        let mapped_err = self.map_type(err_ty);
        if !matches!(mapped_err.as_str(), "void*" | "const void*") {
            return true;
        }
        !type_contains_trait_object(err_ty)
    }

    pub(super) fn is_serde_error_trait_static_call_path(path: &syn::Path, method_name: &str) -> bool {
        if path
            .segments
            .last()
            .map_or(true, |seg| seg.ident.to_string() != method_name)
        {
            return false;
        }
        let owner = path
            .segments
            .iter()
            .take(path.segments.len().saturating_sub(1))
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        matches!(
            owner.as_str(),
            "de::Error" | "serde::de::Error" | "serde_core::de::Error" | "serde_json::de::Error"
        )
    }

    pub(super) fn has_concrete_error_module_type(&self) -> bool {
        self.local_declared_types.contains("error::Error")
            || self.local_declared_types.contains("::error::Error")
            || self.declared_module_names.contains("error")
            || self.declared_module_paths.contains("error")
    }

    pub(super) fn is_type_param_in_scope(&self, name: &str) -> bool {
        self.type_param_scopes
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
    }

    pub(super) fn is_std_optional_syn_type(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(tp) if tp.qself.is_none() => {
                let parts: Vec<String> = tp
                    .path
                    .segments
                    .iter()
                    .map(|seg| seg.ident.to_string())
                    .collect();
                match parts.as_slice() {
                    [single] => single == "optional",
                    [std, opt] => std == "std" && opt == "optional",
                    _ => false,
                }
            }
            syn::Type::Paren(p) => self.is_std_optional_syn_type(&p.elem),
            syn::Type::Group(g) => self.is_std_optional_syn_type(&g.elem),
            _ => false,
        }
    }

    pub(super) fn is_rust_option_syn_type(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Path(tp) if tp.qself.is_none() => {
                let parts: Vec<String> = tp
                    .path
                    .segments
                    .iter()
                    .map(|seg| seg.ident.to_string())
                    .collect();
                match parts.as_slice() {
                    [single] => single == "Option",
                    [ns, opt] => (ns == "rusty" || ns == "core" || ns == "std") && opt == "Option",
                    [prefix, option, opt] => {
                        (prefix == "core" || prefix == "std")
                            && option == "option"
                            && opt == "Option"
                    }
                    _ => false,
                }
            }
            syn::Type::Paren(p) => self.is_rust_option_syn_type(&p.elem),
            syn::Type::Group(g) => self.is_rust_option_syn_type(&g.elem),
            _ => false,
        }
    }

    pub(super) fn should_soften_dependent_assoc_mode(&self) -> bool {
        self.module_name.is_some() || self.expanded_libtest_mode
    }

    pub(super) fn is_explicit_unit_type(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Tuple(t) => t.elems.is_empty(),
            syn::Type::Paren(p) => self.is_explicit_unit_type(&p.elem),
            syn::Type::Group(g) => self.is_explicit_unit_type(&g.elem),
            _ => false,
        }
    }
}
