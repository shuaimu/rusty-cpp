use super::*;

impl CodeGen {
    // ============================================================
    // Bridge between the type inference engine (`type_solver`) and
    // the emit pipeline. Phase 4c-ii — see §13 of
    // docs/rusty-cpp-transpiler.md.
    //
    // Emit sites that want to ask "given these two ternary arms,
    // what's the unified type?" call into this method rather than
    // reaching into `type_solver` directly. Centralizing the call
    // here lets us layer caching, telemetry, and the eventual
    // promotion of the hard-coded variant-constructor table
    // (`type_solver::recognize_variant_constructor_call`) to a
    // `CodeGen`-driven oracle without touching every consumer.
    //
    // Returns the rendered C++ string for the unified type, or
    // `None` if the engine can't fully pin every parameter. `None`
    // means "fall back to today's heuristic emit" — the caller
    // never has to interpret `None` as anything else.
    // ============================================================

    /// Query the inference engine for the unified type of two
    /// ternary (or match) arms. See `type_solver::infer_branch_merge`
    /// for the semantics. Today this is a thin wrapper; later
    /// commits will route through the per-function
    /// `self.inference` cache populated in `emit_function`.
    pub(crate) fn try_infer_ternary_arm_type(
        &self,
        arm_a: &syn::Expr,
        arm_b: &syn::Expr,
    ) -> Option<String> {
        let merged = super::type_solver::infer_branch_merge(arm_a, arm_b)?;
        super::type_solver::render_tyterm_for_cpp(&merged)
    }


    /// Given a single Rust use-path segment (e.g. `node`, taken from
    /// `using ::node::Root;`), return the fully-qualified C++ module
    /// name iff that segment corresponds to an ancestor-sibling module
    /// of the file currently being emitted.
    ///
    /// The current file's module name is in `self.module_name` (e.g.
    /// `btree_port.btree.map.entry`). The Rust `use` could have
    /// reached `segment` through any number of `super::`s, so the
    /// resolved C++ module could live as a sibling of any ancestor.
    /// We walk ancestors from closest to furthest and return the
    /// first match — closest takes precedence to mirror Rust's
    /// inside-out name resolution.
    ///
    /// For `module_name = "btree_port.btree.map.entry"`, the
    /// ancestor candidates for `segment = "borrow"` are:
    ///   1. `btree_port.btree.map.borrow`   (sibling of self)
    ///   2. `btree_port.btree.borrow`       (sibling of parent)
    ///   3. `btree_port.borrow`             (sibling of grandparent)
    ///
    /// Returns `None` when:
    ///   * we're not in module mode (`module_name` is `None`),
    ///   * the current module has no parent (top-level crate file),
    ///   * no ancestor produces a candidate present in
    ///     `crate_module_names`.
    pub(super) fn resolve_sibling_module_path(&self, segment: &str) -> Option<String> {
        let current = self.module_name.as_deref()?;
        let mut prefix = current;
        while let Some(dot) = prefix.rfind('.') {
            prefix = &prefix[..dot];
            let candidate = format!("{}.{}", prefix, segment);
            if self.crate_module_names.contains(&candidate) {
                return Some(candidate);
            }
        }
        None
    }

    /// Walk the type-alias chain from a given tail to its eventual
    /// underlying struct tail. Returns the input tail unchanged if it
    /// is not an alias, or the terminal struct tail. Bounded by an
    /// iteration cap to defend against accidental cycles in source.
    pub(super) fn resolve_type_alias_tail<'a>(&'a self, mut tail: &'a str) -> &'a str {
        for _ in 0..16 {
            match self.cross_file_type_alias_tails.get(tail) {
                Some(next) => tail = next.as_str(),
                None => return tail,
            }
        }
        tail
    }

    pub(super) fn resolve_direct_field_target_module_for_auto_cycle_rewrite(
        path: &syn::Path,
        known_modules: &HashSet<String>,
        imported_name_to_module: &HashMap<String, String>,
        scope_path: &[String],
    ) -> Option<String> {
        let segments: Vec<String> = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        if let Some(first) = segments.first()
            && let Some(imported_module) = imported_name_to_module.get(first)
            && known_modules.contains(imported_module)
        {
            return Some(imported_module.clone());
        }
        if segments.len() < 2 {
            return None;
        }
        for module_idx in 0..segments.len() {
            if module_idx + 1 >= segments.len() {
                continue;
            }
            let module_name = &segments[module_idx];
            if !known_modules.contains(module_name) {
                continue;
            }
            let prefix = &segments[..module_idx];
            if Self::auto_cycle_field_path_prefix_matches_scope(prefix, scope_path) {
                return Some(module_name.clone());
            }
        }
        None
    }

    pub(super) fn resolve_scope_import_binding_target_for_exact_scope(
        &self,
        scope_key: &str,
        local_name: &str,
    ) -> Option<String> {
        if local_name.is_empty() {
            return None;
        }
        let scope_variants = Self::scope_binding_key_variants(scope_key);
        let local_variants = Self::scope_binding_key_variants(local_name);
        let mut candidates: HashSet<String> = HashSet::new();
        for scope in &scope_variants {
            for local in &local_variants {
                if let Some(found) = self
                    .scope_import_bindings
                    .get(&(scope.clone(), local.clone()))
                {
                    candidates.extend(found.iter().cloned());
                }
            }
        }
        let target = self
            .pick_preferred_scope_binding_target(&candidates)
            .or_else(|| {
                if self.in_forward_decl_signature {
                    self.pick_preferred_forward_decl_scope_binding_target(&candidates, local_name)
                } else {
                    None
                }
            })?;
        if target.is_empty() {
            None
        } else {
            Some(target.to_string())
        }
    }

    pub(super) fn resolve_scope_import_binding_target_in_scope_chain(
        &self,
        scope_key: &str,
        local_name: &str,
    ) -> Option<String> {
        if local_name.is_empty() {
            return None;
        }
        let scope_segments: Vec<&str> = scope_key
            .split("::")
            .filter(|seg| !seg.is_empty())
            .collect();
        for depth in (0..=scope_segments.len()).rev() {
            let candidate_scope = if depth == 0 {
                String::new()
            } else {
                scope_segments[..depth].join("::")
            };
            if let Some(target) = self
                .resolve_scope_import_binding_target_for_exact_scope(&candidate_scope, local_name)
            {
                return Some(target);
            }
        }
        None
    }

    pub(super) fn resolve_scope_import_binding_path_for_scope(
        &self,
        scope_key: &str,
        local_name: &str,
    ) -> Option<String> {
        if local_name.is_empty() {
            return None;
        }
        let target =
            self.resolve_scope_import_binding_target_for_exact_scope(scope_key, local_name)?;
        if target.is_empty() {
            None
        } else {
            let mut resolved_target = target.to_string();
            resolved_target = self.resolve_unqualified_local_import_path(&resolved_target);
            resolved_target = self.strip_current_crate_prefix_from_import_path(&resolved_target);
            resolved_target = self.resolve_nested_local_reexport_path(&resolved_target);
            resolved_target = self.rewrite_external_crate_import_path(&resolved_target);
            let resolved_trimmed = resolved_target.trim_start_matches("::");
            if !scope_key.is_empty() && !resolved_trimmed.is_empty() {
                let scoped_candidate = format!("{}::{}", scope_key, resolved_trimmed);
                let escaped_scoped_candidate = scoped_candidate
                    .split("::")
                    .filter(|seg| !seg.is_empty())
                    .map(escape_cpp_keyword)
                    .collect::<Vec<String>>()
                    .join("::");
                let resolved_is_known = self.local_declared_types.contains(resolved_trimmed)
                    || self.local_declared_types.contains(
                        &resolved_trimmed
                            .split("::")
                            .filter(|seg| !seg.is_empty())
                            .map(escape_cpp_keyword)
                            .collect::<Vec<String>>()
                            .join("::"),
                    )
                    || self.declared_module_paths.contains(resolved_trimmed);
                let scoped_is_known = self.local_declared_types.contains(&scoped_candidate)
                    || self
                        .local_declared_types
                        .contains(&escaped_scoped_candidate)
                    || self.declared_module_paths.contains(&scoped_candidate)
                    || self
                        .declared_module_paths
                        .contains(&escaped_scoped_candidate);
                if !resolved_is_known && scoped_is_known {
                    resolved_target = scoped_candidate;
                }
            }
            let resolved_trimmed = resolved_target.trim_start_matches("::");
            if let Some((root, rest)) = resolved_trimmed.split_once("::")
                && let Some(private_target) = self.inferred_private_alias_target(root)
            {
                let private_target = private_target.trim_start_matches("::");
                if !private_target.is_empty() {
                    resolved_target = format!("{}::{}", private_target, rest);
                }
            }
            resolved_target = self.rewrite_seed_ctor_path_string(&resolved_target);
            let mut escaped_target = Self::escape_qualified_path_preserve_global(&resolved_target);
            escaped_target =
                self.rewrite_forced_global_private_alias_root_for_scope(&escaped_target, scope_key);
            if escaped_target.starts_with("::") {
                let root = escaped_target
                    .trim_start_matches("::")
                    .split("::")
                    .next()
                    .unwrap_or_default();
                if root.ends_with("_private") {
                    escaped_target = escaped_target.trim_start_matches("::").to_string();
                }
            }
            Some(escaped_target)
        }
    }

    pub(super) fn resolve_scope_import_binding_path(&self, local_name: &str) -> Option<String> {
        let scope_key = self.module_stack.join("::");
        let type_declared_in_scope = if self.in_forward_decl_signature {
            self.current_module_declares_type_name_exact(local_name)
        } else {
            self.current_scope_declares_type_name(local_name)
                || self.current_owner_module_declares_type_name(local_name)
        };
        if type_declared_in_scope || self.current_scope_declares_function_name(local_name) {
            return None;
        }
        self.resolve_scope_import_binding_path_for_scope(&scope_key, local_name)
    }

    pub(super) fn resolve_unique_scope_import_binding_path_any_scope(
        &self,
        local_name: &str,
    ) -> Option<String> {
        if local_name.is_empty() {
            return None;
        }
        let local_variants = Self::scope_binding_key_variants(local_name);
        let mut candidate_scopes: Vec<String> = self
            .scope_import_bindings
            .keys()
            .filter_map(|(scope, local)| {
                local_variants
                    .iter()
                    .any(|variant| variant == local)
                    .then_some(scope.clone())
            })
            .collect();
        candidate_scopes.sort();
        candidate_scopes.dedup();
        if candidate_scopes.is_empty() {
            return None;
        }

        let mut resolved_targets: Vec<String> = candidate_scopes
            .iter()
            .filter_map(|scope| self.resolve_scope_import_binding_path_for_scope(scope, local_name))
            .filter(|target| !target.is_empty())
            .collect();
        resolved_targets.sort();
        resolved_targets.dedup();
        if resolved_targets.len() == 1 {
            resolved_targets.first().cloned()
        } else {
            None
        }
    }

    pub(super) fn infer_method_template_args_from_in_scope_undeduced_params(
        &self,
        owner: &str,
        method_name: &str,
    ) -> Option<String> {
        let type_params = self.lookup_owner_method_type_param_names(owner, method_name)?;
        if type_params.is_empty() {
            return None;
        }
        let arg_expected_types =
            self.lookup_owner_method_arg_expected_types_for_template_inference(owner, method_name)?;
        let return_ty =
            self.lookup_owner_method_return_type_for_template_inference(owner, method_name)?;

        // Only inject explicit method template args when every method type parameter
        // is referenced in the return type and in none of the parameter types.
        // This matches Rust's expected-type inference cases that C++ cannot deduce.
        let all_undeduced_from_args = type_params.iter().all(|param| {
            self.type_mentions_named_type_param(return_ty, param)
                && !arg_expected_types
                    .iter()
                    .flatten()
                    .any(|arg_ty| self.type_mentions_named_type_param(arg_ty, param))
        });
        if !all_undeduced_from_args {
            return None;
        }
        if !type_params
            .iter()
            .all(|param| self.is_type_param_in_scope(param))
        {
            return None;
        }

        let mapped_params = type_params
            .iter()
            .map(|param| escape_cpp_keyword(param))
            .collect::<Vec<String>>()
            .join(", ");
        Some(format!("<{}>", mapped_params))
    }

    pub(super) fn resolve_known_free_function_expr_path(&self, path: &syn::Path) -> Option<String> {
        if !Self::looks_like_function_path(path) {
            return None;
        }
        let mut candidates: Vec<String> = self
            .call_path_candidates(path)
            .into_iter()
            .filter(|candidate| self.is_known_free_function_path(candidate))
            .collect();
        candidates.sort();
        candidates.dedup();
        if candidates.len() == 1 {
            let escaped = self.escape_and_rename_qualified_name(&candidates[0]);
            if path.segments.len() == 1 {
                let qualified_parent = escaped
                    .rsplit_once("::")
                    .map(|(parent, _)| parent)
                    .unwrap_or_default();
                let current_scope = self.module_stack.join("::");
                let escaped_current_scope = self
                    .module_stack
                    .iter()
                    .map(|seg| escape_cpp_keyword(seg))
                    .collect::<Vec<String>>()
                    .join("::");
                let directly_inside = (!current_scope.is_empty()
                    && (qualified_parent == current_scope
                        || current_scope.ends_with(&format!("::{}", qualified_parent))))
                    || (!escaped_current_scope.is_empty()
                        && (qualified_parent == escaped_current_scope
                            || escaped_current_scope
                                .ends_with(&format!("::{}", qualified_parent))));
                if directly_inside {
                    let local_name = escape_cpp_keyword(&path.segments[0].ident.to_string());
                    let collides_with_member = self
                        .emitted_non_method_member_names
                        .last()
                        .is_some_and(|members| members.contains(&local_name));
                    if collides_with_member {
                        // Inside methods, unqualified function calls can be shadowed by
                        // same-name fields (e.g. `validate_struct_keys` bool member).
                        // Keep the free-function call explicit in that case.
                        return Some(format!("::{}", escaped));
                    }
                    return Some(local_name);
                }
            }
            return Some(format!("::{}", escaped));
        }
        None
    }

    pub(super) fn resolve_scoped_namespace_function_expr_path(
        &self,
        namespace_name: &str,
        fn_name: &str,
    ) -> Option<String> {
        if namespace_name.is_empty() || fn_name.is_empty() {
            return None;
        }
        let namespace_is_rusty_ext =
            namespace_name == "rusty_ext" || namespace_name.ends_with("::rusty_ext");
        if namespace_name == "rusty_ext" {
            let has_private_prefix = self
                .module_stack
                .iter()
                .any(|seg| seg == "private" || seg == "private_" || seg.starts_with("__private"));
            if has_private_prefix {
                if self.module_stack.iter().any(|seg| seg == "de") {
                    if fn_name == "deserialize" {
                        return Some("::de::rusty_ext::deserialize".to_string());
                    }
                    if fn_name == "fmt" {
                        return Some("::de::rusty_ext::fmt".to_string());
                    }
                }
                if self.module_stack.iter().any(|seg| seg == "ser") {
                    if fn_name == "serialize" {
                        return Some("::ser::impls::rusty_ext::serialize".to_string());
                    }
                }
            }
            if fn_name == "deserialize" {
                return Some("::de::rusty_ext::deserialize".to_string());
            }
        }
        let mut candidates = Vec::new();
        // Prefer module-scoped extension shims over root-level `rusty_ext`.
        // Root candidates from skipped no-receiver trait methods can otherwise
        // shadow real scoped extension free functions and produce bad calls.
        if namespace_name != "rusty_ext" || self.module_stack.is_empty() {
            candidates.push(format!("{}::{}", namespace_name, fn_name));
        }
        for depth in (1..=self.module_stack.len()).rev() {
            let prefix = self.module_stack[..depth].join("::");
            candidates.push(format!("{}::{}::{}", prefix, namespace_name, fn_name));
        }
        if namespace_is_rusty_ext {
            let known = self.collect_known_rusty_ext_free_function_paths();
            candidates.retain(|candidate| known.contains(candidate));
        } else {
            candidates.retain(|candidate| self.is_known_free_function_path(candidate));
        }
        // `rusty_ext::foo` inside a Rust module is a relative path. Prefer an
        // existing known free-function path with best module-prefix match; only
        // fall back to lexical qualification when metadata is truly unavailable.
        if candidates.is_empty() && namespace_name == "rusty_ext" {
            return self.resolve_unscoped_namespace_function_expr_path(namespace_name, fn_name);
        }
        if namespace_name == "rusty_ext"
            && self.module_stack.is_empty()
            && candidates
                .iter()
                .any(|candidate| candidate == &format!("rusty_ext::{}", fn_name))
        {
            let exact = format!("rusty_ext::{}", fn_name);
            let suffix = format!("::{}", exact);
            let mut scoped_candidates: Vec<String> = self
                .collect_known_rusty_ext_free_function_paths()
                .into_iter()
                .filter(|key| key != &exact && key.ends_with(&suffix))
                .collect();
            scoped_candidates.sort();
            scoped_candidates.dedup();
            if scoped_candidates.len() == 1 {
                let escaped = self.escape_and_rename_qualified_name(&scoped_candidates[0]);
                return Some(format!("::{}", escaped));
            }
        }
        candidates.sort();
        candidates.dedup();
        if candidates.len() == 1 {
            let escaped = self.escape_and_rename_qualified_name(&candidates[0]);
            return Some(format!("::{}", escaped));
        }
        None
    }

    pub(super) fn resolve_unscoped_namespace_function_expr_path(
        &self,
        namespace_name: &str,
        fn_name: &str,
    ) -> Option<String> {
        if namespace_name.is_empty() || fn_name.is_empty() {
            return None;
        }
        let namespace_is_rusty_ext =
            namespace_name == "rusty_ext" || namespace_name.ends_with("::rusty_ext");
        if namespace_name == "rusty_ext" {
            let has_private_prefix = self
                .module_stack
                .iter()
                .any(|seg| seg == "private" || seg == "private_" || seg.starts_with("__private"));
            if has_private_prefix {
                if self.module_stack.iter().any(|seg| seg == "de") {
                    if fn_name == "deserialize" {
                        return Some("::de::rusty_ext::deserialize".to_string());
                    }
                    if fn_name == "fmt" {
                        return Some("::de::rusty_ext::fmt".to_string());
                    }
                }
                if self.module_stack.iter().any(|seg| seg == "ser") {
                    if fn_name == "serialize" {
                        return Some("::ser::impls::rusty_ext::serialize".to_string());
                    }
                }
            }
            if fn_name == "deserialize" {
                return Some("::de::rusty_ext::deserialize".to_string());
            }
        }
        let exact = format!("{}::{}", namespace_name, fn_name);
        let suffix = format!("::{}", exact);
        let known_paths = if namespace_is_rusty_ext {
            self.collect_known_rusty_ext_free_function_paths()
        } else {
            self.collect_known_free_function_paths()
        };
        let mut candidates: Vec<String> = known_paths
            .into_iter()
            .filter(|key| key == &exact || key.ends_with(&suffix))
            .collect();
        candidates.sort();
        candidates.dedup();
        if candidates.is_empty() {
            return None;
        }
        if namespace_name == "rusty_ext" {
            let exact = format!("rusty_ext::{}", fn_name);
            if candidates.iter().any(|candidate| candidate == &exact) {
                let mut scoped_candidates: Vec<String> = candidates
                    .iter()
                    .filter(|candidate| *candidate != &exact)
                    .cloned()
                    .collect();
                scoped_candidates.sort();
                scoped_candidates.dedup();
                if scoped_candidates.len() == 1 {
                    let escaped = self.escape_and_rename_qualified_name(&scoped_candidates[0]);
                    return Some(format!("::{}", escaped));
                }
            }
        }
        if candidates.len() == 1 {
            let escaped = self.escape_and_rename_qualified_name(&candidates[0]);
            return Some(format!("::{}", escaped));
        }
        let current_module = self.module_stack.join("::");
        let mut ranked: Vec<(usize, String)> = candidates
            .into_iter()
            .map(|candidate| {
                (
                    Self::common_module_prefix_depth(&candidate, &current_module),
                    candidate,
                )
            })
            .collect();
        ranked.sort_by(|(ld, lp), (rd, rp)| rd.cmp(ld).then_with(|| lp.cmp(rp)));
        let best = ranked.first()?.clone();
        let is_ambiguous = ranked
            .iter()
            .skip(1)
            .any(|(depth, _)| *depth == best.0 && best.0 > 0);
        if is_ambiguous {
            return None;
        }
        let escaped = self.escape_and_rename_qualified_name(&best.1);
        Some(format!("::{}", escaped))
    }

    pub(super) fn resolve_known_unqualified_free_function_expr_path(&self, fn_name: &str) -> Option<String> {
        if fn_name.is_empty() {
            return None;
        }
        let suffix = format!("::{}", fn_name);
        let mut candidates: Vec<String> = self
            .collect_known_free_function_paths()
            .into_iter()
            .filter(|key| key == fn_name || key.ends_with(&suffix))
            .collect();
        candidates.sort();
        candidates.dedup();
        if candidates.is_empty() {
            return None;
        }
        if candidates.len() == 1 {
            let escaped = self.escape_and_rename_qualified_name(&candidates[0]);
            return Some(format!("::{}", escaped));
        }
        let current_module = self.module_stack.join("::");
        let mut ranked: Vec<(usize, String)> = candidates
            .into_iter()
            .map(|candidate| {
                (
                    Self::common_module_prefix_depth(&candidate, &current_module),
                    candidate,
                )
            })
            .collect();
        ranked.sort_by(|(ld, lp), (rd, rp)| rd.cmp(ld).then_with(|| lp.cmp(rp)));
        let best = ranked.first()?.clone();
        let is_ambiguous = ranked
            .iter()
            .skip(1)
            .any(|(depth, _)| *depth == best.0 && best.0 > 0);
        if is_ambiguous {
            return None;
        }
        let escaped = self.escape_and_rename_qualified_name(&best.1);
        Some(format!("::{}", escaped))
    }

    pub(super) fn resolve_alias_owner_key_from_owner_path(
        &self,
        owner_path: Option<&syn::Path>,
        owner_name: &str,
    ) -> Option<String> {
        let candidates = self.alias_owner_key_candidates_from_owner_path(owner_path, owner_name);
        for candidate in candidates {
            if let Some(canonical) = self.canonical_alias_owner_key_for_candidate(&candidate) {
                return Some(canonical);
            }
        }
        None
    }

    pub(super) fn resolve_alias_owner_key_with_method_from_receiver_target_type(
        &self,
        receiver_ty: &syn::Type,
        method_name: &str,
    ) -> Option<(String, Option<bool>)> {
        let receiver_ty = self.peel_reference_paren_group_type(receiver_ty);
        let receiver_cpp = self.map_type(receiver_ty);
        if receiver_cpp.is_empty() {
            return None;
        }

        let mut matches: HashMap<String, Option<bool>> = HashMap::new();
        for (alias_key, alias_target) in &self.type_alias_targets {
            let alias_target = self.peel_reference_paren_group_type(alias_target);
            if self.map_type(alias_target) != receiver_cpp {
                continue;
            }

            let canonical = self
                .canonical_alias_owner_key_for_candidate(alias_key)
                .unwrap_or_else(|| alias_key.clone());
            let receiver_shape = self
                .lookup_alias_inherent_owner_method_has_receiver_for_owner_key(
                    &canonical,
                    method_name,
                )
                .or_else(|| {
                    canonical.rsplit("::").next().and_then(|tail| {
                        self.lookup_alias_inherent_owner_method_has_receiver_for_owner_key(
                            tail,
                            method_name,
                        )
                    })
                });
            let Some(receiver_shape) = receiver_shape else {
                continue;
            };

            match matches.entry(canonical) {
                std::collections::hash_map::Entry::Vacant(vac) => {
                    vac.insert(receiver_shape);
                }
                std::collections::hash_map::Entry::Occupied(mut occ) => {
                    if *occ.get() != receiver_shape {
                        occ.insert(None);
                    }
                }
            }
        }

        if matches.len() != 1 {
            return None;
        }
        matches.into_iter().next()
    }

    pub(super) fn resolve_alias_owner_key_with_receiver_method_name(
        &self,
        method_name: &str,
    ) -> Option<(String, Option<bool>)> {
        let mut matches: HashMap<String, Option<bool>> = HashMap::new();
        for (key, receiver_shape) in &self.alias_inherent_owner_method_has_receiver {
            let Some((owner, method)) = key.rsplit_once("::") else {
                continue;
            };
            if method != method_name {
                continue;
            }
            if !matches!(*receiver_shape, Some(true)) {
                continue;
            }
            let canonical = self
                .canonical_alias_owner_key_for_candidate(owner)
                .unwrap_or_else(|| owner.to_string());
            match matches.entry(canonical) {
                std::collections::hash_map::Entry::Vacant(vac) => {
                    vac.insert(Some(true));
                }
                std::collections::hash_map::Entry::Occupied(mut occ) => {
                    if !matches!(*occ.get(), Some(true)) {
                        occ.insert(None);
                    }
                }
            }
        }
        if matches.is_empty() {
            return None;
        }
        if matches.len() == 1 {
            return matches.into_iter().next();
        }

        let mut best_matches: Vec<(String, Option<bool>)> = Vec::new();
        let mut best_prefix = 0usize;
        for (owner_key, receiver_shape) in matches {
            let owner_segments: Vec<&str> = owner_key
                .split("::")
                .filter(|segment| !segment.is_empty())
                .collect();
            let owner_ns = if owner_segments.len() > 1 {
                &owner_segments[..owner_segments.len() - 1]
            } else {
                &[][..]
            };
            let prefix_len = owner_ns
                .iter()
                .zip(self.module_stack.iter())
                .take_while(|(owner_seg, scope_seg)| *owner_seg == scope_seg)
                .count();
            if prefix_len > best_prefix {
                best_prefix = prefix_len;
                best_matches.clear();
                best_matches.push((owner_key, receiver_shape));
            } else if prefix_len == best_prefix {
                best_matches.push((owner_key, receiver_shape));
            }
        }
        if best_prefix > 0 && best_matches.len() == 1 {
            return best_matches.into_iter().next();
        }
        None
    }

    pub(super) fn resolve_alias_owner_key_with_method_from_owner_path(
        &self,
        owner_path: Option<&syn::Path>,
        owner_name: &str,
        method_name: &str,
    ) -> Option<(String, Option<bool>)> {
        let candidates = self.alias_owner_key_candidates_from_owner_path(owner_path, owner_name);
        for candidate in candidates {
            let Some(canonical) = self.canonical_alias_owner_key_for_candidate(&candidate) else {
                continue;
            };
            let receiver_shape = self
                .lookup_alias_inherent_owner_method_has_receiver_for_owner_key(
                    &canonical,
                    method_name,
                )
                .or_else(|| {
                    canonical.rsplit("::").next().and_then(|tail| {
                        self.lookup_alias_inherent_owner_method_has_receiver_for_owner_key(
                            tail,
                            method_name,
                        )
                    })
                });
            if let Some(receiver_shape) = receiver_shape {
                return Some((canonical, receiver_shape));
            }
        }
        None
    }

    pub(super) fn resolve_trait_scoped_key_for_impl(
        &self,
        trait_path: &syn::Path,
        module_path: &[String],
    ) -> String {
        let raw = trait_path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        qualify_impl_type_name(
            &raw,
            module_path,
            &self.declared_item_names,
            &self.local_declared_types,
        )
    }

    pub(super) fn resolve_trait_static_default_key_for_impl(
        &self,
        trait_path: &syn::Path,
        module_path: &[String],
    ) -> Option<String> {
        let scoped_key = self.resolve_trait_scoped_key_for_impl(trait_path, module_path);
        if self.trait_static_default_methods.contains_key(&scoped_key) {
            return Some(scoped_key);
        }
        if self.trait_declared_paths.contains(&scoped_key) {
            // Trait is known locally but has no static default methods.
            return None;
        }
        let is_explicit_nonrelative = trait_path.segments.len() > 1
            && trait_path.segments.first().is_some_and(|seg| {
                !matches!(seg.ident.to_string().as_str(), "self" | "super" | "crate")
            });
        if is_explicit_nonrelative {
            return None;
        }
        let tail = trait_path.segments.last()?.ident.to_string();
        if self.trait_static_default_methods.contains_key(&tail) {
            return Some(tail);
        }
        let mut matches: Vec<String> = self
            .trait_static_default_methods
            .keys()
            .filter(|candidate| {
                candidate
                    .rsplit_once("::")
                    .is_some_and(|(_, suffix)| suffix == tail)
            })
            .cloned()
            .collect();
        matches.sort_unstable();
        matches.dedup();
        if matches.len() == 1 {
            return matches.into_iter().next();
        }
        None
    }

    pub(super) fn resolve_expanded_test_marker_target(&self, marker: &str) -> Option<String> {
        if self.emitted_top_level_functions.contains(marker) {
            return Some(marker.to_string());
        }

        let tail = marker.rsplit("::").next().unwrap_or(marker);
        let mut matches: Vec<String> = self
            .emitted_top_level_functions
            .iter()
            .filter(|name| name.as_str() == tail || name.ends_with(&format!("::{}", tail)))
            .cloned()
            .collect();
        matches.sort();
        matches.dedup();

        if matches.len() == 1 {
            matches.into_iter().next()
        } else {
            None
        }
    }

    pub(super) fn infer_function_type_template_args_from_pointer_call(
        &self,
        call: &syn::ExprCall,
        emitted_args: &[String],
    ) -> Option<Vec<String>> {
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        let last_seg = path_expr.path.segments.last()?;
        let fn_name = last_seg.ident.to_string();
        if self.is_local_function_name_in_scope(&fn_name) {
            return None;
        }
        if !matches!(last_seg.arguments, syn::PathArguments::None) {
            return None;
        }
        if call.args.is_empty() || emitted_args.is_empty() {
            return None;
        }
        let type_params = self.lookup_function_type_param_names(call.func.as_ref())?;
        if type_params.len() != 1 {
            return None;
        }
        let first_expected = self.lookup_function_arg_expected_type(call.func.as_ref(), 0)?;
        let pointee_ident = self.extract_pointer_like_expected_type_param_name(first_expected)?;
        if pointee_ident != type_params[0] {
            return None;
        }
        let first_arg_cpp = emitted_args.first()?;
        Some(vec![format!(
            "std::remove_pointer_t<std::remove_cvref_t<decltype(({}))>>",
            first_arg_cpp
        )])
    }

    /// Infer template arguments from a function-path argument's return type.
    ///
    /// Handles patterns like `case_(bits, TypeName::all)` where `TypeName::all` is a
    /// static method returning `TypeName`, but C++ template deduction cannot infer T
    /// from `typename T::Bits` alone.
    ///
    /// Returns Some(vec![type_name]) if the last argument is a path like `TypeName::method`
    /// and we can infer that T should be `TypeName`.
    pub(super) fn infer_template_args_from_fn_path_return_type(
        &self,
        call: &syn::ExprCall,
    ) -> Option<Vec<String>> {
        // Only handle free functions (not UFCS method calls)
        let syn::Expr::Path(_func_path) = call.func.as_ref() else {
            return None;
        };

        // Check if the called function has type parameters
        let type_params = self.lookup_function_type_param_names(call.func.as_ref())?;
        if type_params.is_empty() {
            return None;
        }

        // Get the last argument
        let last_arg = call.args.last()?;

        // Check if the last argument is a path expression (e.g., `TypeName::method`)
        let syn::Expr::Path(arg_path) = last_arg else {
            return None;
        };

        // Need at least 2 segments for `TypeName::method` pattern
        if arg_path.path.segments.len() < 2 {
            return None;
        }

        // The second-to-last segment is the type name (e.g., `TestFlags` in `TestFlags::all`)
        // For `tests::TestFlags::all`, segments are ["tests", "TestFlags", "all"]
        let type_seg = arg_path
            .path
            .segments
            .get(arg_path.path.segments.len() - 2)?;

        // Verify this looks like a type name (starts with uppercase)
        let type_name = &type_seg.ident.to_string();
        if !type_name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase())
        {
            return None;
        }

        // Check if this type name is actually a declared type (avoids false positives)
        if !self.local_declared_types.contains(type_name)
            && !self.is_local_type_name_in_scope(type_name)
        {
            return None;
        }

        // For now, return a single type argument (matches the first type parameter)
        // This handles the common case of `fn case<T: Flags>(expected: T::Bits, inherent: impl FnOnce() -> T)`
        Some(vec![type_name.clone()])
    }

    pub(super) fn resolve_single_segment_scope_import_bound_type(&self, local_name: &str) -> Option<String> {
        let scope_key = self.module_stack.join("::");
        let bound_target = self
            .resolve_scope_import_binding_path_for_scope(&scope_key, local_name)
            .or_else(|| self.resolve_scope_import_binding_path_for_scope("", local_name))
            .or_else(|| self.resolve_unique_scope_import_binding_path_any_scope(local_name))?;
        let mut rebound = self.rewrite_cpp_import_bound_type_spelling(&bound_target);
        rebound = self.resolve_nested_local_reexport_path(&rebound);
        if let Some(resolved_nested) = self.try_resolve_nested_local_type_path(&rebound) {
            rebound = resolved_nested;
        }
        let had_global_prefix = rebound.starts_with("::");
        let renamed = self.escape_and_rename_qualified_name(rebound.trim_start_matches("::"));
        rebound = if had_global_prefix && !renamed.starts_with("::") {
            format!("::{}", renamed)
        } else {
            renamed
        };
        Some(rebound)
    }

    pub(super) fn infer_private_alias_full_path_from_remainder(&self, remainder: &str) -> Option<String> {
        let remainder = remainder.trim();
        if remainder.is_empty() {
            return None;
        }
        let suffix = format!("::{}", remainder.trim_start_matches("::"));
        let mut candidates: Vec<String> = self
            .local_declared_types
            .iter()
            .filter(|candidate| {
                candidate
                    .split("::")
                    .next()
                    .is_some_and(|root| root.starts_with("__private"))
                    && candidate.ends_with(&suffix)
            })
            .cloned()
            .collect();
        if candidates.is_empty() {
            let mut module_candidates: Vec<String> = self
                .declared_module_paths
                .iter()
                .filter(|candidate| {
                    candidate
                        .split("::")
                        .next()
                        .is_some_and(|root| root.starts_with("__private"))
                })
                .map(|candidate| format!("{}::{}", candidate, remainder))
                .collect();
            candidates.append(&mut module_candidates);
        }
        candidates.sort();
        candidates.dedup();
        if candidates.is_empty() {
            return None;
        }
        if candidates.len() == 1 {
            return candidates.first().cloned();
        }
        let mut ranked: Vec<(usize, String)> = candidates
            .into_iter()
            .map(|candidate| {
                (
                    candidate.split("::").filter(|seg| !seg.is_empty()).count(),
                    candidate,
                )
            })
            .collect();
        ranked.sort_by(|(ld, lp), (rd, rp)| ld.cmp(rd).then_with(|| lp.cmp(rp)));
        let best_depth = ranked.first()?.0;
        let mut best: Vec<String> = ranked
            .into_iter()
            .filter_map(|(depth, candidate)| (depth == best_depth).then_some(candidate))
            .collect();
        best.sort();
        best.dedup();
        if best.len() == 1 {
            best.first().cloned()
        } else {
            None
        }
    }

    pub(super) fn resolve_bare_module_namespace_import(&self, path: &str) -> Option<String> {
        if path.trim_start().starts_with("namespace ") {
            return None;
        }
        let normalized = normalize_use_import_path(path);
        if normalized.is_empty()
            || normalized.contains("::")
            || normalized.contains(" = ")
            || normalized.starts_with("::")
            || normalized.starts_with("namespace ")
            || self.module_stack.is_empty()
        {
            return None;
        }
        if !self.declared_module_names.contains(normalized) {
            return None;
        }
        Some(normalized.to_string())
    }

    pub(super) fn resolve_unqualified_local_import_path(&self, path: &str) -> String {
        let normalized = normalize_use_import_path(path);
        if normalized.is_empty()
            || normalized.contains("::")
            || normalized.contains(" = ")
            || normalized.starts_with("::")
            || normalized.starts_with("namespace ")
        {
            return path.to_string();
        }
        if self.declared_item_names.contains(normalized) {
            return path.to_string();
        }

        let mut matches: Vec<String> = self
            .local_declared_types
            .iter()
            .filter(|scoped| {
                scoped
                    .rsplit("::")
                    .next()
                    .is_some_and(|tail| tail == normalized)
            })
            .cloned()
            .collect();
        matches.sort();
        matches.dedup();
        if matches.len() != 1 {
            let mut scoped_matches: Vec<String> = matches
                .iter()
                .filter(|name| name.contains("::"))
                .cloned()
                .collect();
            scoped_matches.sort();
            scoped_matches.dedup();
            if scoped_matches.len() == 1 {
                let resolved = &scoped_matches[0];
                if path.starts_with("namespace ") {
                    return format!("namespace {}", resolved);
                }
                return resolved.clone();
            }
            return path.to_string();
        }

        let resolved = &matches[0];
        if path.starts_with("namespace ") {
            format!("namespace {}", resolved)
        } else {
            resolved.clone()
        }
    }

    pub(super) fn resolve_crate_single_segment_type_import(&self, path: &str) -> Option<String> {
        let normalized = normalize_use_import_path(path)
            .trim()
            .trim_start_matches("::");
        let leaf = normalized.strip_prefix("crate::")?;
        if leaf.is_empty() || leaf.contains("::") || leaf.contains(" = ") {
            return None;
        }
        if !leaf
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            return None;
        }
        let escaped_leaf = escape_cpp_keyword(leaf);
        let mut candidates: Vec<String> = self
            .local_declared_types
            .iter()
            .filter(|candidate| {
                candidate.ends_with(&format!("::{}", leaf))
                    || candidate.ends_with(&format!("::{}", escaped_leaf))
            })
            .cloned()
            .collect();
        candidates.sort();
        candidates.dedup();
        if candidates.len() == 1 {
            return candidates.pop();
        }

        let module_guess = leaf.to_ascii_lowercase();
        let guessed_variants = [
            format!("{}::{}", module_guess, leaf),
            format!("{}::{}", module_guess, escaped_leaf),
        ];
        if let Some(found) = guessed_variants
            .iter()
            .find(|candidate| self.local_declared_types.contains(candidate.as_str()))
        {
            return Some((*found).clone());
        }

        let mut parent_matches: Vec<String> = candidates
            .iter()
            .filter(|candidate| {
                candidate
                    .rsplit("::")
                    .nth(1)
                    .is_some_and(|parent| parent.eq_ignore_ascii_case(&module_guess))
            })
            .cloned()
            .collect();
        parent_matches.sort();
        parent_matches.dedup();
        if parent_matches.len() == 1 {
            return parent_matches.pop();
        }
        let guessed = format!("{}::{}", module_guess, escaped_leaf);
        if self.local_declared_types.contains(&guessed)
            || self.declared_module_paths.contains(&module_guess)
        {
            return Some(guessed);
        }
        None
    }

    pub(super) fn resolve_nested_local_reexport_path(&self, path: &str) -> String {
        if path.starts_with("namespace ") {
            return path.to_string();
        }
        if let Some((alias, target)) = split_use_import_alias(path) {
            if let Some(resolved) = self.try_resolve_nested_local_type_path(target) {
                return format!("{} = {}", alias, resolved);
            }
            return path.to_string();
        }
        self.try_resolve_nested_local_type_path(path)
            .unwrap_or_else(|| path.to_string())
    }

    pub(super) fn resolve_type_reexport_path_via_scope_binding(&self, path: &str) -> Option<String> {
        let trimmed = path.trim();
        if trimmed.is_empty() || trimmed.starts_with("namespace ") || trimmed.contains(" = ") {
            return None;
        }
        let had_leading_colon = trimmed.starts_with("::");
        let normalized = trimmed.trim_start_matches("::");
        let (scope, leaf) = normalized.rsplit_once("::")?;
        if scope.is_empty()
            || leaf.is_empty()
            || !leaf
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            return None;
        }
        let escaped_scope = scope
            .split("::")
            .filter(|seg| !seg.is_empty())
            .map(escape_cpp_keyword)
            .collect::<Vec<String>>()
            .join("::");
        let escaped_leaf = escape_cpp_keyword(leaf);
        let direct_candidates = [
            format!("{}::{}", scope, leaf),
            format!("{}::{}", scope, escaped_leaf),
            format!("{}::{}", escaped_scope, leaf),
            format!("{}::{}", escaped_scope, escaped_leaf),
        ];
        if direct_candidates
            .iter()
            .any(|candidate| self.local_declared_types.contains(candidate))
        {
            return None;
        }
        let target = self.resolve_scope_import_binding_target_for_exact_scope(scope, leaf)?;
        let mut target = target.trim().trim_start_matches("::").to_string();
        if target.is_empty() || target == normalized || target.ends_with("::*") {
            return None;
        }
        target = self.rewrite_external_crate_import_path(&target);
        target = self.strip_current_crate_prefix_from_import_path(&target);
        target = Self::strip_crate_root_cpp_path(&target);
        target = self.rewrite_seed_ctor_path_string(&target);
        let escaped = Self::escape_qualified_path_preserve_global(target.trim());
        if escaped.trim_start_matches("::") == normalized {
            return None;
        }
        if had_leading_colon && !escaped.starts_with("::") {
            Some(format!("::{}", escaped))
        } else {
            Some(escaped)
        }
    }

    pub(super) fn resolve_descendant_type_path_in_module(
        &self,
        module_path: &str,
        type_name: &str,
    ) -> Option<String> {
        if module_path.is_empty() || type_name.is_empty() {
            return None;
        }
        let escaped_module = module_path
            .split("::")
            .filter(|seg| !seg.is_empty())
            .map(escape_cpp_keyword)
            .collect::<Vec<String>>()
            .join("::");
        let escaped_type = escape_cpp_keyword(type_name);
        let mut prefixes = vec![format!("{}::", module_path)];
        if !escaped_module.is_empty() && escaped_module != module_path {
            prefixes.push(format!("{}::", escaped_module));
        }
        prefixes.sort();
        prefixes.dedup();
        let mut suffixes = vec![format!("::{}", type_name)];
        if escaped_type != type_name {
            suffixes.push(format!("::{}", escaped_type));
        }
        suffixes.sort();
        suffixes.dedup();
        let mut candidates: Vec<String> = self
            .local_declared_types
            .iter()
            .filter(|candidate| {
                prefixes.iter().any(|prefix| candidate.starts_with(prefix))
                    && suffixes.iter().any(|suffix| candidate.ends_with(suffix))
            })
            .cloned()
            .collect();
        candidates.sort();
        candidates.dedup();
        if candidates.len() == 1 {
            candidates.into_iter().next()
        } else if self.in_forward_decl_signature {
            let module_depth = module_path
                .split("::")
                .filter(|seg| !seg.is_empty())
                .count();
            let mut nested_candidates: Vec<String> = candidates
                .into_iter()
                .filter(|candidate| {
                    candidate.split("::").filter(|seg| !seg.is_empty()).count() > module_depth + 1
                })
                .collect();
            nested_candidates.sort();
            nested_candidates.dedup();
            if nested_candidates.len() == 1 {
                nested_candidates.into_iter().next()
            } else {
                None
            }
        } else {
            None
        }
    }

    pub(super) fn resolve_shadowed_impl_const_expr(&self, c: &syn::ImplItemConst) -> Option<String> {
        let expr = self.peel_paren_group_expr(&c.expr);
        let syn::Expr::Path(path_expr) = expr else {
            return None;
        };
        if path_expr.path.leading_colon.is_some() || path_expr.path.segments.len() != 1 {
            return None;
        }
        let const_name = c.ident.to_string();
        let referenced = path_expr.path.segments.first()?.ident.to_string();
        if referenced != const_name {
            return None;
        }

        // Rust resolves bare names in associated const initializers against outer
        // module items before the const being declared. Qualify explicitly to avoid
        // C++ self-shadowing like `const START = START;`.
        let module_prefix = if self.module_stack.is_empty() {
            String::new()
        } else {
            format!("{}::", self.module_stack.join("::"))
        };
        let qualified_rust_path = format!("::{}{}", module_prefix, const_name);
        let Ok(qualified_expr) = syn::parse_str::<syn::Expr>(&qualified_rust_path) else {
            return None;
        };
        Some(self.emit_expr_to_string_with_expected(&qualified_expr, Some(&c.ty)))
    }

    pub(super) fn resolve_current_struct_assoc_cpp_type(&self, assoc_name: &str) -> Option<String> {
        let assoc_cpp_name = escape_cpp_keyword(assoc_name);
        let alias_emitted = self
            .emitted_non_method_member_names
            .last()
            .is_some_and(|scope| scope.contains(&assoc_cpp_name));
        if let Some(scope) = self.current_struct_assoc_cpp_types.last() {
            if let Some(ty) = scope
                .get(assoc_name)
                .or_else(|| scope.get(&escape_cpp_keyword(assoc_name)))
            {
                if !alias_emitted
                    && let Some(struct_name) = self.current_struct.as_ref()
                    && let Some(resolved) =
                        self.resolve_assoc_type_from_impl_blocks(struct_name, assoc_name)
                {
                    return Some(resolved);
                }
                return Some(ty.clone());
            }
        }
        let struct_name = self.current_struct.as_ref()?;
        self.resolve_assoc_type_from_impl_blocks(struct_name, assoc_name)
    }

    pub(super) fn resolve_current_struct_assoc_cpp_type_if_concrete(
        &self,
        assoc_name: &str,
    ) -> Option<String> {
        let resolved = self.resolve_current_struct_assoc_cpp_type(assoc_name)?;
        if self.mapped_assoc_type_contains_unbound_placeholder(&resolved) {
            None
        } else {
            Some(resolved)
        }
    }

    pub(super) fn infer_tuple_constructor_arg_expected_type_from_context(
        &self,
        call: &syn::ExprCall,
        context_expected_ty: Option<&syn::Type>,
        arg_idx: usize,
    ) -> Option<syn::Type> {
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        let joined = path_expr
            .path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if !matches!(
            joined.as_str(),
            "make_tuple" | "std::make_tuple" | "core::tuple::make_tuple"
        ) {
            return None;
        }
        let expected_ty = context_expected_ty?;
        let tuple_candidate = self
            .expected_wrapper_inner_type_candidates(expected_ty)
            .into_iter()
            .find(|inner| {
                matches!(
                    self.peel_reference_paren_group_type(inner),
                    syn::Type::Tuple(_)
                )
            })
            .or_else(|| Some(expected_ty.clone()))?;
        let tuple_candidate = self.peel_reference_paren_group_type(&tuple_candidate);
        let syn::Type::Tuple(tuple_ty) = tuple_candidate else {
            return None;
        };
        tuple_ty.elems.iter().nth(arg_idx).cloned()
    }

    pub(super) fn infer_map_value_hint_from_next_value_arg(&self, arg: &syn::Expr) -> Option<syn::Type> {
        let arg = self.peel_paren_group_expr(arg);
        match arg {
            syn::Expr::Try(try_expr) => {
                self.infer_map_value_hint_from_next_value_arg(&try_expr.expr)
            }
            syn::Expr::Block(block_expr) => self
                .extract_tail_expr_from_block(&block_expr.block)
                .and_then(|tail| self.infer_map_value_hint_from_next_value_arg(tail)),
            syn::Expr::Match(match_expr) => {
                self.infer_map_value_hint_from_next_value_arg(&match_expr.expr)
            }
            syn::Expr::MethodCall(mc) if mc.args.is_empty() && mc.method == "next_value" => {
                let value_ty = self.current_struct_assoc_type_hint("Value").or_else(|| {
                    self.expected_result_type_arg_owned(self.current_return_type_hint(), 0)
                })?;
                if self.type_hint_is_map_like(&value_ty) {
                    return None;
                }
                Some(value_ty)
            }
            _ => None,
        }
    }

    pub(super) fn resolve_current_struct_assoc_projection_syn_hint(
        &self,
        ty: &syn::Type,
    ) -> Option<syn::Type> {
        let mapped = self.resolve_current_struct_assoc_projection_cpp_type(ty)?;
        Self::parse_mapped_cpp_type_as_syn_hint(&mapped)
    }

    pub(super) fn infer_oncecell_inner_type_from_get_or_init_arg(
        &self,
        arg: &syn::Expr,
        enumerate_index_names: &HashSet<String>,
    ) -> Option<syn::Type> {
        if let syn::Expr::Closure(closure) = self.peel_paren_group_expr(arg) {
            if let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(&closure.body) {
                if path_expr.path.segments.len() == 1 {
                    let name = path_expr.path.segments[0].ident.to_string();
                    if enumerate_index_names.contains(&name) {
                        return Some(parse_quote!(usize));
                    }
                }
            }
        }
        self.infer_closure_return_type(arg)
    }

    pub(super) fn infer_oncecell_inner_type_from_get_or_try_init_arg(
        &self,
        arg: &syn::Expr,
        enumerate_index_names: &HashSet<String>,
    ) -> Option<syn::Type> {
        let ret_ty =
            self.infer_oncecell_inner_type_from_get_or_init_arg(arg, enumerate_index_names)?;
        let ret_ty = self.peel_reference_paren_group_type(&ret_ty);
        let syn::Type::Path(tp) = ret_ty else {
            return None;
        };
        let last = tp.path.segments.last()?;
        if last.ident != "Result" {
            return None;
        }
        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
            return None;
        };
        args.args.iter().find_map(|arg| match arg {
            syn::GenericArgument::Type(inner) => Some(inner.clone()),
            _ => None,
        })
    }

    pub(super) fn infer_option_hint_type_from_some_expr(&self, expr: &syn::Expr) -> Option<syn::Type> {
        let expr = self.peel_paren_group_expr(expr);
        let syn::Expr::Call(call) = expr else {
            return None;
        };
        let call_func = self.peel_paren_group_expr(call.func.as_ref());
        let syn::Expr::Path(path_expr) = call_func else {
            return None;
        };
        if !self.is_option_some_path(&path_expr.path) {
            return None;
        }
        let inner_expr = call.args.first()?;
        let inner_ty = self
            .infer_local_binding_type_from_initializer(inner_expr)
            .or_else(|| self.infer_simple_expr_type(inner_expr))
            .or_else(|| self.infer_hint_type_from_expr(inner_expr))?;
        Some(parse_quote!(Option<#inner_ty>))
    }

    pub(super) fn infer_option_hint_type_from_return_context_for_unresolved_some(&self) -> Option<syn::Type> {
        let return_ty = self.current_return_type_hint()?;
        let return_ty = self.peel_reference_paren_group_type(return_ty);
        let syn::Type::Path(tp) = return_ty else {
            return None;
        };
        let last = tp.path.segments.last()?;
        if last.ident != "Result" {
            return None;
        }
        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
            return None;
        };
        let type_args: Vec<syn::Type> = args
            .args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        if type_args.len() < 2 {
            return None;
        }

        let ok_ty = &type_args[0];
        let err_ty = &type_args[1];
        let mut ok_params = HashSet::new();
        let mut err_params = HashSet::new();
        self.collect_type_param_names_in_type(ok_ty, &mut ok_params);
        self.collect_type_param_names_in_type(err_ty, &mut err_params);

        let mut value_only_params: Vec<String> =
            ok_params.difference(&err_params).cloned().collect();
        value_only_params.sort();
        value_only_params.dedup();

        if value_only_params.len() != 1 {
            // Expanded trait impl methods often return `Result<Self::Value, E>`
            // where `Self::Value` hides the concrete generic (for example `T`).
            // Recover that by selecting the unique in-scope type parameter not used
            // by the error side when the Ok side is `Self::Value`.
            let ok_is_self_value_assoc = matches!(
                self.peel_reference_paren_group_type(ok_ty),
                syn::Type::Path(ok_path)
                    if ok_path.qself.is_none()
                        && ok_path.path.segments.len() == 2
                        && ok_path.path.segments[0].ident == "Self"
                        && ok_path.path.segments[1].ident == "Value"
            );
            if value_only_params.is_empty() && ok_is_self_value_assoc {
                let mut fallback_params: Vec<String> = self
                    .ordered_type_params_in_scope()
                    .into_iter()
                    .filter(|name| !err_params.contains(name))
                    .collect();
                fallback_params.sort();
                fallback_params.dedup();
                if fallback_params.len() == 1 {
                    value_only_params = fallback_params;
                }
            }
        }
        if value_only_params.len() != 1 {
            return None;
        }
        let inner_ty = syn::parse_str::<syn::Type>(&value_only_params[0]).ok()?;
        Some(parse_quote!(Option<#inner_ty>))
    }

    pub(super) fn infer_concrete_hint_type_from_peer_expr(&self, expr: &syn::Expr) -> Option<syn::Type> {
        let inferred = self
            .infer_local_binding_type_from_initializer(expr)
            .or_else(|| self.infer_simple_expr_type(expr))?;
        let inferred = self.peel_reference_paren_group_type(&inferred).clone();
        if self.type_is_concrete_hint_candidate(&inferred) {
            Some(inferred)
        } else {
            None
        }
    }

    pub(super) fn infer_associated_call_arg_expected_type_from_owner(
        &self,
        owner_seg: &syn::PathSegment,
        owner_name: &str,
        method_name: &str,
        arg_idx: usize,
        arg_expr: Option<&syn::Expr>,
    ) -> Option<syn::Type> {
        let owner_inner_ty = match &owner_seg.arguments {
            syn::PathArguments::AngleBracketed(owner_args) => owner_args
                .args
                .iter()
                .find_map(|arg| match arg {
                    syn::GenericArgument::Type(t) if !self.type_contains_infer(t) => {
                        Some(t.clone())
                    }
                    _ => None,
                })
                .or_else(|| {
                    self.infer_owner_inner_type_from_in_progress_placeholder_hint(owner_name)
                }),
            syn::PathArguments::None => self
                .infer_owner_inner_type_from_alias(owner_name)
                .or_else(|| {
                    self.infer_owner_inner_type_from_in_progress_placeholder_hint(owner_name)
                })
                .or_else(|| {
                    self.lookup_local_placeholder_type_hint(ONCECELL_FALLBACK_INNER_HINT_KEY)
                        .cloned()
                        .map(|ty| normalize_placeholder_hint_for_owner(Some(owner_name), ty))
                        .filter(|ty| {
                            !self.type_contains_infer(ty)
                                && !self.type_contains_in_scope_type_param(ty)
                                && !self.type_contains_unbound_single_letter_generic(ty)
                                && !self.type_contains_unresolved_placeholder_like(ty)
                        })
                })
                .or_else(|| {
                    let owner_key = if owner_name == "Self" {
                        self.current_struct
                            .as_ref()
                            .map(|name| self.scoped_type_key(name))
                    } else {
                        Some(self.scoped_type_key(owner_name))
                    };
                    owner_key
                        .as_ref()
                        .and_then(|key| self.declared_type_params.get(key))
                        .or_else(|| {
                            if owner_name == "Self" {
                                self.current_struct
                                    .as_ref()
                                    .and_then(|name| self.declared_type_params.get(name))
                            } else {
                                self.declared_type_params.get(owner_name)
                            }
                        })
                        .zip(
                            owner_key
                                .as_ref()
                                .and_then(|key| self.declared_type_param_kinds.get(key))
                                .or_else(|| {
                                    if owner_name == "Self" {
                                        self.current_struct.as_ref().and_then(|name| {
                                            self.declared_type_param_kinds.get(name)
                                        })
                                    } else {
                                        self.declared_type_param_kinds.get(owner_name)
                                    }
                                }),
                        )
                        .and_then(|(params, kinds)| {
                            params.iter().zip(kinds.iter()).find_map(|(param, kind)| {
                                matches!(kind, GenericParamKind::Type).then_some(param.clone())
                            })
                        })
                        .and_then(|param| syn::parse_str::<syn::Type>(&param).ok())
                }),
            _ => None,
        }?;
        match (owner_name, method_name, arg_idx) {
            ("Spanned", "new" | "new_", 0) => {
                syn::parse_str::<syn::Type>("core::ops::Range<usize>")
                    .ok()
                    .or_else(|| syn::parse_str::<syn::Type>("std::ops::Range<usize>").ok())
            }
            ("Spanned", "new" | "new_", 1) => Some(owner_inner_ty),
            ("Lazy", "new" | "new_", 0) => {
                let callable_ty: syn::Type = parse_quote!(impl FnOnce() -> #owner_inner_ty);
                Some(callable_ty)
            }
            ("OnceCell", "init" | "get_or_init", 0) => {
                let callable_ty: syn::Type = parse_quote!(impl FnOnce() -> #owner_inner_ty);
                Some(callable_ty)
            }
            ("OnceCell", "get_or_try_init", 0) => {
                let err_ty = self.inferred_try_init_error_type_or_unit(arg_expr);
                let callable_ty: syn::Type =
                    parse_quote!(impl FnOnce() -> Result<#owner_inner_ty, #err_ty>);
                Some(callable_ty)
            }
            ("OnceBox", "with_value" | "from", 0) => {
                let boxed_ty: syn::Type = parse_quote!(Box<#owner_inner_ty>);
                Some(boxed_ty)
            }
            ("Box", "new" | "new_" | "make", 0) => Some(owner_inner_ty),
            (name, "new" | "new_", 0) if name.ends_with("Deserializer") => None,
            (_, "new" | "new_", 0) => Some(owner_inner_ty),
            ("OnceBox", "get_or_init", 0) => {
                let callable_ty: syn::Type = parse_quote!(impl FnOnce() -> Box<#owner_inner_ty>);
                Some(callable_ty)
            }
            ("OnceBox", "get_or_try_init", 0) => {
                let err_ty = self.inferred_try_init_error_type_or_unit(arg_expr);
                let callable_ty: syn::Type =
                    parse_quote!(impl FnOnce() -> Result<Box<#owner_inner_ty>, #err_ty>);
                Some(callable_ty)
            }
            _ => None,
        }
    }

    pub(super) fn infer_associated_call_arg_expected_type_from_call_expected_owner(
        &self,
        call: &syn::ExprCall,
        call_expected_ty: Option<&syn::Type>,
        arg_idx: usize,
    ) -> Option<syn::Type> {
        let call_expected_ty = call_expected_ty?;
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        if path_expr.path.segments.len() < 2 {
            return None;
        }
        let owner_name = path_expr
            .path
            .segments
            .iter()
            .nth_back(1)
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default();
        let method_name = path_expr
            .path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default();
        if arg_idx != 0 {
            return None;
        }

        let expected_owner_ty = self.peel_reference_paren_group_type(call_expected_ty);
        let syn::Type::Path(expected_tp) = expected_owner_ty else {
            return None;
        };
        let expected_last = expected_tp.path.segments.last()?;
        if expected_last.ident != owner_name {
            return None;
        }
        let syn::PathArguments::AngleBracketed(expected_args) = &expected_last.arguments else {
            return None;
        };
        let owner_inner_ty = expected_args.args.iter().find_map(|arg| match arg {
            syn::GenericArgument::Type(t) => Some(t.clone()),
            _ => None,
        })?;
        let arg_expr = call.args.get(arg_idx);

        match (owner_name.as_str(), method_name.as_str(), arg_idx) {
            ("Spanned", "new" | "new_", 0) => {
                syn::parse_str::<syn::Type>("core::ops::Range<usize>")
                    .ok()
                    .or_else(|| syn::parse_str::<syn::Type>("std::ops::Range<usize>").ok())
            }
            ("Spanned", "new" | "new_", 1) => Some(owner_inner_ty),
            ("Lazy", "new" | "new_", 0) => {
                let callable_ty: syn::Type = parse_quote!(impl FnOnce() -> #owner_inner_ty);
                Some(callable_ty)
            }
            ("OnceCell", "init" | "get_or_init", 0) => {
                let callable_ty: syn::Type = parse_quote!(impl FnOnce() -> #owner_inner_ty);
                Some(callable_ty)
            }
            ("OnceCell", "get_or_try_init", 0) => {
                let err_ty = self.inferred_try_init_error_type_or_unit(arg_expr);
                let callable_ty: syn::Type =
                    parse_quote!(impl FnOnce() -> Result<#owner_inner_ty, #err_ty>);
                Some(callable_ty)
            }
            ("OnceBox", "with_value" | "from", 0) => {
                let boxed_ty: syn::Type = parse_quote!(Box<#owner_inner_ty>);
                Some(boxed_ty)
            }
            ("Box", "new" | "new_" | "make", 0) => Some(owner_inner_ty),
            (name, "new" | "new_", 0) if name.ends_with("Deserializer") => None,
            (_, "new" | "new_", 0) => Some(owner_inner_ty),
            ("OnceBox", "get_or_init", 0) => {
                let callable_ty: syn::Type = parse_quote!(impl FnOnce() -> Box<#owner_inner_ty>);
                Some(callable_ty)
            }
            ("OnceBox", "get_or_try_init", 0) => {
                let err_ty = self.inferred_try_init_error_type_or_unit(arg_expr);
                let callable_ty: syn::Type =
                    parse_quote!(impl FnOnce() -> Result<Box<#owner_inner_ty>, #err_ty>);
                Some(callable_ty)
            }
            _ => None,
        }
    }

    pub(super) fn infer_tuple_struct_constructor_call_arg_expected_type(
        &self,
        call: &syn::ExprCall,
        call_expected_ty: Option<&syn::Type>,
        arg_idx: usize,
    ) -> Option<syn::Type> {
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        let ctor_seg_idx = path_expr.path.segments.len().checked_sub(1)?;
        let ctor_seg = path_expr.path.segments.iter().nth(ctor_seg_idx)?;
        let ctor_name = ctor_seg.ident.to_string();
        let ctor_struct_name = if ctor_name == "Self" {
            self.current_struct.clone()?
        } else {
            ctor_name
        };

        let tuple_field_name = format!("_{}", arg_idx);
        let field_name = if self
            .lookup_struct_field_type(&ctor_struct_name, &tuple_field_name)
            .is_some()
        {
            tuple_field_name
        } else {
            self.lookup_struct_field_order(&ctor_struct_name)?
                .get(arg_idx)?
                .clone()
        };
        let mut field_ty = self.lookup_struct_field_type(&ctor_struct_name, &field_name)?;
        let mut substitutions = self
            .owner_segment_type_arg_substitutions(&path_expr.path, ctor_seg_idx)
            .unwrap_or_default();
        if substitutions.is_empty()
            && let Some(expected_substitutions) = self
                .tuple_struct_ctor_type_arg_substitutions_from_expected_type(
                    &ctor_struct_name,
                    call_expected_ty,
                )
        {
            substitutions = expected_substitutions;
        }
        if !substitutions.is_empty() {
            field_ty = self.substitute_type_params_in_type(&field_ty, &substitutions);
        }
        Some(field_ty)
    }

    pub(super) fn infer_owner_inner_type_from_in_progress_placeholder_hint(
        &self,
        owner_name: &str,
    ) -> Option<syn::Type> {
        let local_name = self.in_progress_local_initializers.last()?;
        let hint_ty = self.lookup_local_placeholder_type_hint(local_name)?;
        let normalized_hint =
            normalize_placeholder_hint_for_owner(Some(owner_name), hint_ty.clone());
        if self.type_contains_infer(&normalized_hint)
            || self.type_contains_in_scope_type_param(&normalized_hint)
        {
            return None;
        }

        let peeled = self.peel_reference_paren_group_type(&normalized_hint);
        if let syn::Type::Path(tp) = peeled
            && let Some(last) = tp.path.segments.last()
            && last.ident == owner_name
            && let syn::PathArguments::AngleBracketed(args) = &last.arguments
            && let Some(inner_ty) = args.args.iter().find_map(|arg| match arg {
                syn::GenericArgument::Type(t) => Some(t.clone()),
                _ => None,
            })
        {
            return Some(inner_ty);
        }

        Some(peeled.clone())
    }

    pub(super) fn infer_owner_inner_type_from_alias(&self, owner_name: &str) -> Option<syn::Type> {
        let owner_ty: syn::Type = syn::parse_str(owner_name).ok()?;
        let resolved = self.resolve_type_alias_once(&owner_ty)?;
        let resolved = self.peel_reference_paren_group_type(&resolved);
        let syn::Type::Path(tp) = resolved else {
            return None;
        };
        let last = tp.path.segments.last()?;
        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
            return None;
        };
        args.args.iter().find_map(|arg| match arg {
            syn::GenericArgument::Type(inner) => Some(inner.clone()),
            _ => None,
        })
    }

    /// Given a method call on a candidate local (e.g., `cell.set(42)` where
    /// `cell` was initialized with `OnceCell::new()`), try to infer the full
    /// owner type (e.g., `OnceCell<i32>`) from the method name and arguments.
    pub(super) fn infer_owner_type_from_method_usage(
        &self,
        receiver_name: &str,
        method_name: &str,
        args: &syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>,
    ) -> Option<syn::Type> {
        // For methods that accept the element type as their first argument,
        // we can infer T from the argument's type.
        // OnceCell/OnceBox/Lazy: set(T), get_or_init(|| T), get_or_try_init(|| Result<T, E>)
        // Vec-like APIs: push(T), insert(_, T)
        //
        // Return the *inner* type argument only. The caller re-wraps this with
        // the concrete owner detected from the initializer call target.
        let inferred_inner = match method_name {
            "set" | "push" | "try_push" => {
                let arg = args.first()?;
                let arg_ty = self.infer_simple_expr_type(arg)?;
                Some(arg_ty)
            }
            "get_or_insert" => {
                // Option::get_or_insert(value) -> infer inner T from value
                let arg = args.first()?;
                let arg_ty = self
                    .infer_simple_expr_type(arg)
                    .or_else(|| self.infer_local_binding_type_from_initializer(arg))
                    .or_else(|| {
                        let syn::Expr::MethodCall(mc) = self.peel_paren_group_expr(arg) else {
                            return None;
                        };
                        self.infer_method_call_result_type_for_local(mc)
                            .or_else(|| {
                                self.lookup_unique_method_return_type_by_name(
                                    &mc.method.to_string(),
                                )
                            })
                    })?;
                Some(arg_ty)
            }
            "get_or_insert_with" => {
                // Option::get_or_insert_with(|| T) -> infer inner T from closure return
                let arg = args.first()?;
                let ret_ty = self.infer_closure_return_type(arg)?;
                Some(ret_ty)
            }
            "get_or_init" => {
                // The closure argument returns T
                let arg = args.first()?;
                let ret_ty = self.infer_closure_return_type(arg)?;
                Some(ret_ty)
            }
            "get_or_try_init" => {
                // The closure argument returns Result<T, E>
                let arg = args.first()?;
                let ret_ty = self.infer_closure_return_type(arg)?;
                // Extract T from Result<T, E> if possible
                if let syn::Type::Path(tp) = &ret_ty {
                    if let Some(last) = tp.path.segments.last() {
                        if last.ident == "Result" {
                            if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                                    return self
                                        .resolve_receiver_owner_type_from_inner_hint(
                                            receiver_name,
                                            inner,
                                        )
                                        .or_else(|| Some(inner.clone()));
                                }
                            }
                        }
                    }
                }
                Some(ret_ty)
            }
            "insert" | "try_insert" => {
                // insert(index, T) — T is the second argument
                let arg = args.get(1)?;
                let arg_ty = self.infer_simple_expr_type(arg)?;
                Some(arg_ty)
            }
            _ => None,
        }?;

        if let Some(resolved_owner) =
            self.resolve_receiver_owner_type_from_inner_hint(receiver_name, &inferred_inner)
        {
            return Some(resolved_owner);
        }
        if matches!(method_name, "get_or_insert" | "get_or_insert_with") {
            return Some(parse_quote!(Option<#inferred_inner>));
        }
        Some(inferred_inner)
    }

    pub(super) fn resolve_receiver_owner_type_from_inner_hint(
        &self,
        receiver_name: &str,
        inner_hint: &syn::Type,
    ) -> Option<syn::Type> {
        let receiver_ty = self.lookup_local_binding_type(receiver_name)?;
        let peeled_receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
        let syn::Type::Path(receiver_tp) = peeled_receiver_ty else {
            return None;
        };
        let owner_name = receiver_tp.path.segments.last()?.ident.to_string();
        let normalized_hint =
            normalize_placeholder_hint_for_owner(Some(owner_name.as_str()), inner_hint.clone());

        let mut resolved_owner_ty = receiver_ty.clone();
        let syn::Type::Path(resolved_tp) = &mut resolved_owner_ty else {
            return None;
        };
        let last_seg = resolved_tp.path.segments.last_mut()?;
        if let syn::PathArguments::AngleBracketed(args) = &mut last_seg.arguments {
            for arg in args.args.iter_mut() {
                let syn::GenericArgument::Type(inner_ty) = arg else {
                    continue;
                };
                let inner_needs_resolution = self.type_contains_infer(inner_ty)
                    || self.type_contains_in_scope_type_param(inner_ty)
                    || self.type_contains_unbound_single_letter_generic(inner_ty)
                    || self.type_contains_unresolved_placeholder_like(inner_ty);
                if inner_needs_resolution {
                    *inner_ty = normalized_hint.clone();
                    return Some(resolved_owner_ty);
                }
            }
            return None;
        }

        if matches!(
            owner_name.as_str(),
            "Cell" | "Vec" | "Option" | "OnceCell" | "OnceBox" | "Lazy"
        ) {
            let mut owner_args = syn::punctuated::Punctuated::new();
            owner_args.push(syn::GenericArgument::Type(normalized_hint));
            last_seg.arguments =
                syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    colon2_token: None,
                    lt_token: syn::token::Lt::default(),
                    args: owner_args,
                    gt_token: syn::token::Gt::default(),
                });
            return Some(resolved_owner_ty);
        }

        None
    }

    /// Try to infer the return type of a closure expression.
    pub(super) fn infer_closure_return_type(&self, expr: &syn::Expr) -> Option<syn::Type> {
        match expr {
            syn::Expr::Closure(closure) => {
                // Check explicit return type annotation
                if let syn::ReturnType::Type(_, ty) = &closure.output {
                    return Some((**ty).clone());
                }
                if let syn::Expr::Block(block_expr) = self.peel_paren_group_expr(&closure.body) {
                    if let Some(tail_expr) = self.extract_tail_expr_from_block(&block_expr.block) {
                        if let Some(tail_ty) = self
                            .infer_simple_expr_type(tail_expr)
                            .or_else(|| self.infer_hint_type_from_expr(tail_expr))
                        {
                            return Some(tail_ty);
                        }
                    }
                    // Closures frequently use explicit `return ...;` inside blocks.
                    // Recover that return payload type when there is no tail expr.
                    for stmt in block_expr.block.stmts.iter().rev() {
                        let syn::Stmt::Expr(expr, _) = stmt else {
                            continue;
                        };
                        let syn::Expr::Return(ret_expr) = self.peel_paren_group_expr(expr) else {
                            continue;
                        };
                        let Some(ret_value) = &ret_expr.expr else {
                            continue;
                        };
                        if let Some(ret_ty) = self
                            .infer_simple_expr_type(ret_value)
                            .or_else(|| self.infer_hint_type_from_expr(ret_value))
                        {
                            return Some(ret_ty);
                        }
                    }
                }
                if let syn::Expr::Struct(struct_expr) = self.peel_paren_group_expr(&closure.body) {
                    return Some(syn::Type::Path(syn::TypePath {
                        qself: None,
                        path: struct_expr.path.clone(),
                    }));
                }
                if let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(&closure.body) {
                    if path_expr.path.segments.len() == 1
                        && path_expr.path.segments[0]
                            .ident
                            .to_string()
                            .chars()
                            .next()
                            .is_some_and(|ch| ch.is_ascii_uppercase())
                    {
                        return Some(syn::Type::Path(syn::TypePath {
                            qself: None,
                            path: path_expr.path.clone(),
                        }));
                    }
                }
                // Infer from the body expression
                self.infer_simple_expr_type(&closure.body)
            }
            syn::Expr::Call(call) => {
                // If it's a call expression wrapping a closure, recurse
                if let Some(arg) = call.args.first() {
                    let inner = self.infer_closure_return_type(arg);
                    if inner.is_some() {
                        return inner;
                    }
                }
                self.infer_simple_expr_type(expr)
            }
            _ => self.infer_simple_expr_type(expr),
        }
    }

    pub(super) fn infer_owner_type_from_constructor_expr(&self, expr: &syn::Expr) -> Option<syn::Type> {
        let syn::Expr::Call(call) = self.peel_paren_group_expr(expr) else {
            return None;
        };
        let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(call.func.as_ref()) else {
            return None;
        };
        if path_expr.path.segments.len() < 2 {
            return None;
        }
        let owner_len = path_expr.path.segments.len().saturating_sub(1);
        let owner_seg = path_expr.path.segments.iter().nth_back(1)?;
        let method_name = path_expr.path.segments.last()?.ident.to_string();
        if !matches!(
            method_name.as_str(),
            "new" | "new_" | "from" | "with_value" | "default" | "default_"
        ) {
            return None;
        }
        if !matches!(owner_seg.arguments, syn::PathArguments::AngleBracketed(_)) {
            return None;
        }
        let mut owner_path = syn::Path {
            leading_colon: path_expr.path.leading_colon,
            segments: syn::punctuated::Punctuated::new(),
        };
        for seg in path_expr.path.segments.iter().take(owner_len) {
            owner_path.segments.push(seg.clone());
        }
        if owner_path.segments.is_empty() {
            return None;
        }
        let owner_ty = syn::Type::Path(syn::TypePath {
            qself: None,
            path: owner_path,
        });
        if self.type_contains_infer(&owner_ty) {
            return None;
        }
        Some(owner_ty)
    }

    pub(super) fn infer_binding_tuple_element_expected_type_from_peer(
        &self,
        elem: &syn::Expr,
        peer_expr: Option<&syn::Expr>,
    ) -> Option<syn::Type> {
        let elem = self.peel_paren_group_expr(elem);
        match elem {
            syn::Expr::Reference(r) => {
                self.infer_binding_tuple_element_expected_type_from_peer(&r.expr, peer_expr)
            }
            syn::Expr::Array(_) | syn::Expr::Repeat(_) => {
                let peer_expr = peer_expr?;
                let peer_item_ty = self.infer_iter_item_type_from_expr(peer_expr)?;
                Some(parse_quote!([#peer_item_ty]))
            }
            _ => None,
        }
    }

    pub(super) fn resolve_struct_pattern_field_cpp_name(
        &self,
        path: &syn::Path,
        field_name: &str,
        variant_ctx: Option<&VariantTypeContext>,
    ) -> String {
        let mut candidates: Vec<String> = Vec::new();
        let mut push_candidate = |candidate: String| {
            let trimmed = candidate.trim().trim_start_matches("::").to_string();
            if !trimmed.is_empty() && !candidates.iter().any(|existing| existing == &trimmed) {
                candidates.push(trimmed);
            }
        };

        let raw_segments: Vec<String> = path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect();
        if !raw_segments.is_empty() {
            push_candidate(raw_segments.join("::"));
            if let Some(last) = raw_segments.last() {
                push_candidate(last.clone());
            }
        }

        let emitted = self.emit_path_to_string(path);
        if !emitted.is_empty() {
            let emitted_base = emitted
                .split('<')
                .next()
                .unwrap_or(emitted.as_str())
                .trim_start_matches("::")
                .to_string();
            push_candidate(emitted_base.clone());
            if let Some(tail) = emitted_base.rsplit("::").next() {
                push_candidate(tail.to_string());
            }
        }

        if self.path_is_known_data_enum_variant_with_ctx(path, variant_ctx) {
            let variant_cpp = self.variant_pattern_cpp_type(path, variant_ctx);
            let variant_base = variant_cpp
                .split('<')
                .next()
                .unwrap_or(variant_cpp.as_str())
                .trim_start_matches("::")
                .to_string();
            push_candidate(variant_base.clone());
            if let Some(tail) = variant_base.rsplit("::").next() {
                push_candidate(tail.to_string());
            }
        }

        for candidate in candidates {
            if let Some(mapped) = self.lookup_struct_field_cpp_name(&candidate, field_name) {
                return mapped;
            }
        }
        escape_cpp_keyword(field_name)
    }

    pub(super) fn infer_match_expr_common_arm_type(&self, match_expr: &syn::ExprMatch) -> Option<syn::Type> {
        self.infer_match_arms_common_type_with_scrutinee(match_expr)
            .or_else(|| self.infer_match_arms_common_type(&match_expr.arms))
            .or_else(|| self.infer_match_arms_common_variant_constructor_owner(&match_expr.arms))
    }

    pub(super) fn infer_match_arms_common_variant_constructor_owner(
        &self,
        arms: &[syn::Arm],
    ) -> Option<syn::Type> {
        let mut common: Option<syn::Type> = None;
        let mut saw_value_arm = false;
        for arm in arms {
            if self.is_expr_diverging(&arm.body) {
                continue;
            }
            let owner_ty = self.variant_constructor_owner_type_from_arm_body(&arm.body)?;
            saw_value_arm = true;
            if let Some(existing) = common.as_ref() {
                if !Self::types_equivalent_by_tokens(existing, &owner_ty) {
                    return None;
                }
            } else {
                common = Some(owner_ty);
            }
        }
        if saw_value_arm { common } else { None }
    }

    pub(super) fn infer_match_arm_type_with_tuple_scrutinee_bindings(
        &self,
        arm: &syn::Arm,
        scrutinee_elem_tys: &[Option<syn::Type>],
    ) -> Option<syn::Type> {
        let mut inner = self.new_inner_for_block();
        if inner.local_bindings.is_empty() {
            inner.local_bindings.push(HashMap::new());
            inner.local_shadowed_binding_types.push(HashMap::new());
            inner.local_const_bindings.push(HashMap::new());
            inner.local_reference_bindings.push(HashSet::new());
            inner.rebind_reference_pointer_bindings.push(HashSet::new());
            inner.local_cpp_bindings.push(HashMap::new());
        }

        let mut bind_tuple_pat = |tuple_pat: &syn::PatTuple| {
            for (idx, pat_elem) in tuple_pat.elems.iter().enumerate() {
                let Some(Some(elem_ty)) = scrutinee_elem_tys.get(idx) else {
                    continue;
                };
                let mut env = HashMap::new();
                inner.bind_pattern_types_into_env(pat_elem, elem_ty, &mut env);
                for (name, ty) in env {
                    inner.register_local_binding(name, Some(ty));
                }
            }
        };

        match &arm.pat {
            syn::Pat::Tuple(tuple_pat) => bind_tuple_pat(tuple_pat),
            syn::Pat::Or(or_pat) => {
                for case in &or_pat.cases {
                    if let syn::Pat::Tuple(tuple_pat) = case {
                        bind_tuple_pat(tuple_pat);
                        break;
                    }
                }
            }
            _ => {}
        }

        inner
            .infer_local_binding_type_from_initializer(&arm.body)
            .or_else(|| inner.infer_simple_expr_type(&arm.body))
    }

    pub(super) fn infer_match_arms_common_type_with_tuple_scrutinee(
        &self,
        tuple_scrutinee: &syn::ExprTuple,
        arms: &[syn::Arm],
    ) -> Option<syn::Type> {
        let scrutinee_elem_tys: Vec<Option<syn::Type>> = tuple_scrutinee
            .elems
            .iter()
            .map(|expr| self.infer_simple_expr_type(expr))
            .collect();
        if scrutinee_elem_tys.iter().all(Option::is_none) {
            return None;
        }

        let mut common: Option<syn::Type> = None;
        let mut common_from_option_some_int_lit = false;
        let mut saw_option_none_arm = false;

        for arm in arms {
            if self.is_expr_diverging(&arm.body) {
                continue;
            }
            if self.expr_is_option_none_value(&arm.body) {
                saw_option_none_arm = true;
                continue;
            }

            let arm_value_expr = self
                .extract_match_arm_value_expr(&arm.body)
                .unwrap_or(&arm.body);
            let arm_ty = self
                .data_enum_owner_syn_type_from_variant_ctor_expr(arm_value_expr)
                .or_else(|| {
                    self.infer_match_arm_type_with_tuple_scrutinee_bindings(
                        arm,
                        &scrutinee_elem_tys,
                    )
                })
                .or_else(|| self.infer_local_binding_type_from_initializer(arm_value_expr))
                .or_else(|| self.infer_simple_expr_type(arm_value_expr))?;
            let arm_from_option_some_int_lit =
                self.expr_is_option_some_integer_literal(arm_value_expr);

            if common.is_some() {
                if self.merge_match_common_arm_type(&mut common, arm_ty.clone()) {
                    continue;
                }
                let both_option_like = common
                    .as_ref()
                    .is_some_and(|existing| self.is_option_like_syn_type(existing))
                    && self.is_option_like_syn_type(&arm_ty);
                if both_option_like {
                    if common_from_option_some_int_lit && !arm_from_option_some_int_lit {
                        common = Some(arm_ty);
                        common_from_option_some_int_lit = false;
                        continue;
                    }
                    if arm_from_option_some_int_lit && !common_from_option_some_int_lit {
                        continue;
                    }
                }
                return None;
            } else {
                self.merge_match_common_arm_type(&mut common, arm_ty);
                common_from_option_some_int_lit = arm_from_option_some_int_lit;
            }
        }

        if saw_option_none_arm {
            let common_ty = common?;
            if self.expected_option_type_arg(Some(&common_ty)).is_some()
                || self.map_type(&common_ty).starts_with("rusty::Option<")
            {
                return Some(common_ty);
            }
            return None;
        }
        common
    }

    pub(super) fn infer_match_arms_common_type(&self, arms: &[syn::Arm]) -> Option<syn::Type> {
        let mut common: Option<syn::Type> = None;
        let mut saw_option_none_arm = false;
        for arm in arms {
            if self.is_expr_diverging(&arm.body) {
                continue;
            }
            if self.expr_is_option_none_value(&arm.body) {
                saw_option_none_arm = true;
                continue;
            }
            let arm_value_expr = self
                .extract_match_arm_value_expr(&arm.body)
                .unwrap_or(&arm.body);
            let Some(arm_ty) = self
                .data_enum_owner_syn_type_from_variant_ctor_expr(arm_value_expr)
                .or_else(|| self.infer_local_binding_type_from_initializer(arm_value_expr))
                .or_else(|| self.infer_simple_expr_type(arm_value_expr))
            else {
                if self.match_arm_unknown_type_can_defer(arm) {
                    continue;
                }
                return None;
            };
            if !self.merge_match_common_arm_type(&mut common, arm_ty) {
                return None;
            }
        }
        if saw_option_none_arm {
            let common_ty = common?;
            if self.expected_option_type_arg(Some(&common_ty)).is_some()
                || self.map_type(&common_ty).starts_with("rusty::Option<")
            {
                return Some(common_ty);
            }
            return None;
        }
        common
    }

    pub(super) fn infer_match_arm_type_with_scrutinee_bindings(
        &self,
        scrutinee_expr: &syn::Expr,
        arm: &syn::Arm,
    ) -> Option<syn::Type> {
        let mut inner = self.new_inner_for_block();
        if inner.local_bindings.is_empty() {
            inner.local_bindings.push(HashMap::new());
            inner.local_shadowed_binding_types.push(HashMap::new());
            inner.local_const_bindings.push(HashMap::new());
            inner.local_reference_bindings.push(HashSet::new());
            inner.rebind_reference_pointer_bindings.push(HashSet::new());
            inner.local_cpp_bindings.push(HashMap::new());
        }

        let mut env = HashMap::new();
        if let Some(scrutinee_ty) = self
            .infer_simple_expr_type(scrutinee_expr)
            .or_else(|| self.infer_local_binding_type_from_initializer(scrutinee_expr))
        {
            inner.bind_pattern_types_into_env(&arm.pat, &scrutinee_ty, &mut env);
        }
        if env.is_empty() {
            inner.bind_pattern_literal_types_into_env(&arm.pat, &mut env);
        }

        inner
            .infer_expr_type_with_env(&arm.body, &env)
            .or_else(|| inner.infer_local_binding_type_from_initializer(&arm.body))
            .or_else(|| inner.infer_simple_expr_type(&arm.body))
    }

    pub(super) fn infer_match_arms_common_type_with_scrutinee(
        &self,
        match_expr: &syn::ExprMatch,
    ) -> Option<syn::Type> {
        let mut common: Option<syn::Type> = None;
        let mut saw_option_none_arm = false;

        for arm in &match_expr.arms {
            if self.is_expr_diverging(&arm.body) {
                continue;
            }
            if self.expr_is_option_none_value(&arm.body) {
                saw_option_none_arm = true;
                continue;
            }
            let arm_value_expr = self
                .extract_match_arm_value_expr(&arm.body)
                .unwrap_or(&arm.body);
            let Some(arm_ty) = self
                .data_enum_owner_syn_type_from_variant_ctor_expr(arm_value_expr)
                .or_else(|| {
                    self.infer_match_arm_type_with_scrutinee_bindings(&match_expr.expr, arm)
                })
                .or_else(|| self.infer_local_binding_type_from_initializer(arm_value_expr))
                .or_else(|| self.infer_simple_expr_type(arm_value_expr))
            else {
                if self.match_arm_unknown_type_can_defer(arm) {
                    continue;
                }
                return None;
            };
            if !self.merge_match_common_arm_type(&mut common, arm_ty) {
                return None;
            }
        }

        if saw_option_none_arm {
            let common_ty = common?;
            if self.expected_option_type_arg(Some(&common_ty)).is_some()
                || self.map_type(&common_ty).starts_with("rusty::Option<")
            {
                return Some(common_ty);
            }
            return None;
        }
        common
    }

    pub(super) fn infer_variant_type_context_from_expr(&self, expr: &syn::Expr) -> Option<VariantTypeContext> {
        match expr {
            syn::Expr::Path(path) => {
                if path.path.segments.len() == 1 {
                    let name = path.path.segments[0].ident.to_string();
                    if name == "self" {
                        if let Some(enum_name) = &self.current_struct {
                            if !self.type_supports_variant_context(enum_name) {
                                return None;
                            }
                            let template_args = self
                                .enum_type_params
                                .get(enum_name)
                                .map(|p| {
                                    p.iter()
                                        .filter(|name| self.is_type_param_in_scope(name))
                                        .cloned()
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default();
                            return Some(VariantTypeContext {
                                enum_name: enum_name.clone(),
                                template_args,
                            });
                        }
                        return None;
                    }
                    let ty = self.lookup_local_binding_type(&name)?;
                    return self.infer_variant_type_context_from_type(&ty);
                }
                None
            }
            syn::Expr::Call(call) => {
                if let syn::Expr::Path(path) = call.func.as_ref() {
                    if path.path.segments.len() == 1 {
                        let func_name = path.path.segments[0].ident.to_string();
                        if call.args.is_empty() {
                            let ty = self.lookup_local_binding_type(&func_name)?;
                            return self.infer_variant_type_context_from_type(&ty);
                        }
                        if call.args.len() == 1 && matches!(func_name.as_str(), "Left" | "Right") {
                            let arg_ty = self.infer_simple_expr_type(&call.args[0])?;
                            let mapped = self.map_type(&arg_ty);
                            return Some(VariantTypeContext {
                                enum_name: "Either".to_string(),
                                template_args: vec![mapped.clone(), mapped],
                            });
                        }
                    }
                }
                None
            }
            syn::Expr::Field(field) => {
                let member = match &field.member {
                    syn::Member::Named(ident) => ident.to_string(),
                    syn::Member::Unnamed(_) => return None,
                };
                let ty = self.lookup_field_type_for_expr_base(&field.base, &member)?;
                self.infer_variant_type_context_from_type(&ty)
            }
            syn::Expr::MethodCall(mc) => {
                if matches!(mc.method.to_string().as_str(), "start_bound" | "end_bound")
                    && !self.data_enum_types.contains("Bound")
                {
                    if let Some(ty) = self.infer_simple_expr_type(expr) {
                        if let Some(ctx) = self.infer_variant_type_context_from_type(&ty) {
                            return Some(ctx);
                        }
                    }
                    return Some(VariantTypeContext {
                        enum_name: "Bound".to_string(),
                        template_args: Vec::new(),
                    });
                }
                self.infer_simple_expr_type(expr)
                    .and_then(|ty| self.infer_variant_type_context_from_type(&ty))
            }
            syn::Expr::Paren(p) => self.infer_variant_type_context_from_expr(&p.expr),
            syn::Expr::Group(g) => self.infer_variant_type_context_from_expr(&g.expr),
            syn::Expr::Reference(r) => self.infer_variant_type_context_from_expr(&r.expr),
            syn::Expr::Unary(unary) => self
                .infer_simple_expr_type(expr)
                .and_then(|ty| self.infer_variant_type_context_from_type(&ty))
                .or_else(|| self.infer_variant_type_context_from_expr(&unary.expr)),
            syn::Expr::Cast(cast) => self
                .infer_simple_expr_type(expr)
                .and_then(|ty| self.infer_variant_type_context_from_type(&ty))
                .or_else(|| self.infer_variant_type_context_from_expr(&cast.expr)),
            _ => None,
        }
    }

    pub(super) fn infer_variant_type_context_from_pattern(
        &self,
        pat: &syn::Pat,
        fallback: Option<&VariantTypeContext>,
    ) -> Option<VariantTypeContext> {
        let pat = self.peel_pat_type_ref_paren(pat);
        match pat {
            syn::Pat::TupleStruct(tuple_struct_pat) => {
                if tuple_struct_pat.path.segments.len() >= 2 {
                    let enum_name = tuple_struct_pat
                        .path
                        .segments
                        .iter()
                        .nth_back(1)
                        .map(|seg| seg.ident.to_string())?;
                    if self.type_supports_variant_context(&enum_name)
                        || self.path_is_plausible_data_enum_variant(&tuple_struct_pat.path)
                    {
                        let template_args = fallback
                            .filter(|ctx| ctx.enum_name == enum_name)
                            .map(|ctx| ctx.template_args.clone())
                            .unwrap_or_default();
                        return Some(VariantTypeContext {
                            enum_name,
                            template_args,
                        });
                    }
                }
                if tuple_struct_pat.path.segments.len() == 1 {
                    let variant_name = tuple_struct_pat
                        .path
                        .segments
                        .last()
                        .map(|seg| seg.ident.to_string())
                        .unwrap_or_default();
                    let canonical_variant_name =
                        self.canonical_variant_name(&variant_name).to_string();
                    if let Some(runtime_kind) =
                        self.runtime_match_enum_kind_by_variant_name(&variant_name)
                    {
                        let fallback_claims_variant = fallback.is_some_and(|ctx| {
                            self.enum_has_variant_name(&ctx.enum_name, &variant_name)
                                || self
                                    .enum_has_variant_name(&ctx.enum_name, &canonical_variant_name)
                        });
                        if !fallback_claims_variant {
                            let enum_name = match runtime_kind {
                                RuntimeMatchEnumKind::Option => "Option",
                                RuntimeMatchEnumKind::Result => "Result",
                                RuntimeMatchEnumKind::Entry => "Entry",
                            };
                            return Some(VariantTypeContext {
                                enum_name: enum_name.to_string(),
                                template_args: Vec::new(),
                            });
                        }
                    }
                    if let Some(ctx) = fallback
                        && Self::ident_looks_like_variant_ctor_name(&variant_name)
                    {
                        let fallback_claims_variant = self
                            .enum_has_variant_name(&ctx.enum_name, &variant_name)
                            || self.enum_has_variant_name(&ctx.enum_name, &canonical_variant_name);
                        let fallback_unknown_enum = !self.data_enum_types.contains(&ctx.enum_name)
                            && !self.data_enum_variants_by_enum.contains_key(&ctx.enum_name)
                            && !self.data_enum_variants_by_enum.keys().any(|known_enum| {
                                known_enum
                                    .rsplit("::")
                                    .next()
                                    .is_some_and(|tail| tail == ctx.enum_name)
                            });
                        if fallback_claims_variant || fallback_unknown_enum {
                            return Some(ctx.clone());
                        }
                    }
                }
                fallback.cloned()
            }
            syn::Pat::Struct(struct_pat) => {
                if struct_pat.path.segments.len() >= 2 {
                    let enum_name = struct_pat
                        .path
                        .segments
                        .iter()
                        .nth_back(1)
                        .map(|seg| seg.ident.to_string())?;
                    if self.type_supports_variant_context(&enum_name)
                        || self.path_is_plausible_data_enum_variant(&struct_pat.path)
                    {
                        let template_args = fallback
                            .filter(|ctx| ctx.enum_name == enum_name)
                            .map(|ctx| ctx.template_args.clone())
                            .unwrap_or_default();
                        return Some(VariantTypeContext {
                            enum_name,
                            template_args,
                        });
                    }
                }
                fallback.cloned()
            }
            syn::Pat::Path(path_pat) => {
                if path_pat.path.segments.len() >= 2 {
                    let enum_name = path_pat
                        .path
                        .segments
                        .iter()
                        .nth_back(1)
                        .map(|seg| seg.ident.to_string())?;
                    if self.type_supports_variant_context(&enum_name)
                        || self.path_is_plausible_data_enum_variant(&path_pat.path)
                    {
                        let template_args = fallback
                            .filter(|ctx| ctx.enum_name == enum_name)
                            .map(|ctx| ctx.template_args.clone())
                            .unwrap_or_default();
                        return Some(VariantTypeContext {
                            enum_name,
                            template_args,
                        });
                    }
                }
                if path_pat.path.segments.len() == 1 {
                    let variant_name = path_pat
                        .path
                        .segments
                        .last()
                        .map(|seg| seg.ident.to_string())
                        .unwrap_or_default();
                    let canonical_variant_name =
                        self.canonical_variant_name(&variant_name).to_string();
                    if let Some(runtime_kind) =
                        self.runtime_match_enum_kind_by_variant_name(&variant_name)
                    {
                        let fallback_claims_variant = fallback.is_some_and(|ctx| {
                            self.enum_has_variant_name(&ctx.enum_name, &variant_name)
                                || self
                                    .enum_has_variant_name(&ctx.enum_name, &canonical_variant_name)
                        });
                        if !fallback_claims_variant {
                            let enum_name = match runtime_kind {
                                RuntimeMatchEnumKind::Option => "Option",
                                RuntimeMatchEnumKind::Result => "Result",
                                RuntimeMatchEnumKind::Entry => "Entry",
                            };
                            return Some(VariantTypeContext {
                                enum_name: enum_name.to_string(),
                                template_args: Vec::new(),
                            });
                        }
                    }
                    if let Some(ctx) = fallback
                        && Self::ident_looks_like_variant_ctor_name(&variant_name)
                    {
                        let fallback_claims_variant = self
                            .enum_has_variant_name(&ctx.enum_name, &variant_name)
                            || self.enum_has_variant_name(&ctx.enum_name, &canonical_variant_name);
                        let fallback_unknown_enum = !self.data_enum_types.contains(&ctx.enum_name)
                            && !self.data_enum_variants_by_enum.contains_key(&ctx.enum_name)
                            && !self.data_enum_variants_by_enum.keys().any(|known_enum| {
                                known_enum
                                    .rsplit("::")
                                    .next()
                                    .is_some_and(|tail| tail == ctx.enum_name)
                            });
                        if fallback_claims_variant || fallback_unknown_enum {
                            return Some(ctx.clone());
                        }
                    }
                }
                fallback.cloned()
            }
            syn::Pat::Ident(ident_pat)
                if ident_pat.by_ref.is_none()
                    && ident_pat.mutability.is_none()
                    && ident_pat.subpat.is_none() =>
            {
                let variant_name = ident_pat.ident.to_string();
                let canonical_variant_name = self.canonical_variant_name(&variant_name).to_string();
                if let Some(runtime_kind) =
                    self.runtime_match_enum_kind_by_variant_name(&variant_name)
                {
                    let fallback_claims_variant = fallback.is_some_and(|ctx| {
                        self.enum_has_variant_name(&ctx.enum_name, &variant_name)
                            || self.enum_has_variant_name(&ctx.enum_name, &canonical_variant_name)
                    });
                    if !fallback_claims_variant {
                        let enum_name = match runtime_kind {
                            RuntimeMatchEnumKind::Option => "Option",
                            RuntimeMatchEnumKind::Result => "Result",
                            RuntimeMatchEnumKind::Entry => "Entry",
                        };
                        return Some(VariantTypeContext {
                            enum_name: enum_name.to_string(),
                            template_args: Vec::new(),
                        });
                    }
                }
                fallback.cloned()
            }
            syn::Pat::Reference(reference_pat) => {
                self.infer_variant_type_context_from_pattern(&reference_pat.pat, fallback)
            }
            syn::Pat::Type(type_pat) => {
                self.infer_variant_type_context_from_pattern(&type_pat.pat, fallback)
            }
            syn::Pat::Paren(paren_pat) => {
                self.infer_variant_type_context_from_pattern(&paren_pat.pat, fallback)
            }
            _ => fallback.cloned(),
        }
    }

    pub(super) fn infer_variant_type_context_from_type(&self, ty: &syn::Type) -> Option<VariantTypeContext> {
        let syn::Type::Path(tp) = ty else {
            return None;
        };
        let last = tp.path.segments.last()?;
        let enum_name = last.ident.to_string();
        if !self.type_supports_variant_context(&enum_name) {
            return None;
        }
        let mut template_args = if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
            args.args
                .iter()
                .filter_map(|arg| match arg {
                    syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        if template_args.is_empty() {
            if let Some(params) = self.enum_type_params.get(&enum_name) {
                if params.iter().all(|p| self.is_type_param_in_scope(p)) {
                    template_args = params.clone();
                }
            }
        }

        Some(VariantTypeContext {
            enum_name,
            template_args,
        })
    }

    pub(super) fn infer_while_let_some_payload_type_from_body(
        &self,
        pat: &syn::Pat,
        body: &syn::Block,
    ) -> Option<syn::Type> {
        if let Some(binding_name) = self.simple_some_payload_binding_name(pat) {
            return self.infer_local_binding_type_from_body_usage(&binding_name, &body.stmts);
        }

        let syn::Pat::TupleStruct(ts) = pat else {
            return None;
        };
        if ts.elems.len() != 1 || !self.is_option_some_path(&ts.path) {
            return None;
        }
        let syn::Pat::Tuple(tuple_pat) = ts.elems.first()? else {
            return None;
        };
        let mut elem_tys = Vec::with_capacity(tuple_pat.elems.len());
        for elem_pat in &tuple_pat.elems {
            let syn::Pat::Ident(pi) = elem_pat else {
                return None;
            };
            if pi.ident == "_" || pi.subpat.is_some() {
                return None;
            }
            elem_tys.push(
                self.infer_local_binding_type_from_body_usage(&pi.ident.to_string(), &body.stmts)?,
            );
        }
        syn::parse2::<syn::Type>(quote!((#(#elem_tys),*))).ok()
    }

    pub(super) fn infer_local_binding_type_from_body_usage(
        &self,
        binding_name: &str,
        stmts: &[syn::Stmt],
    ) -> Option<syn::Type> {
        for stmt in stmts {
            if let Some(ty) = self.infer_local_binding_type_from_stmt_usage(binding_name, stmt) {
                return Some(ty);
            }
        }
        None
    }

    pub(super) fn infer_local_binding_type_from_stmt_usage(
        &self,
        binding_name: &str,
        stmt: &syn::Stmt,
    ) -> Option<syn::Type> {
        match stmt {
            syn::Stmt::Local(local) => local.init.as_ref().and_then(|init| {
                self.infer_local_binding_type_from_expr_usage(binding_name, &init.expr)
            }),
            syn::Stmt::Expr(expr, _) => {
                self.infer_local_binding_type_from_expr_usage(binding_name, expr)
            }
            syn::Stmt::Item(_) | syn::Stmt::Macro(_) => None,
        }
    }

    pub(super) fn infer_local_binding_type_from_expr_usage(
        &self,
        binding_name: &str,
        expr: &syn::Expr,
    ) -> Option<syn::Type> {
        match expr {
            syn::Expr::MethodCall(mc) => {
                let method = mc.method.to_string();
                for (idx, arg) in mc.args.iter().enumerate() {
                    if extract_simple_local_ident(arg).as_deref() != Some(binding_name) {
                        continue;
                    }
                    let receiver_ty = self
                        .infer_simple_expr_type(&mc.receiver)
                        .or_else(|| self.infer_local_binding_type_from_initializer(&mc.receiver))
                        .or_else(|| {
                            extract_simple_local_ident(&mc.receiver).and_then(|name| {
                                self.lookup_local_placeholder_type_hint(&name).cloned()
                            })
                        });
                    if let Some(receiver_ty) = receiver_ty {
                        match method.as_str() {
                            "push" | "try_push" | "set" if idx == 0 => {
                                if let Some(elem_ty) = extract_sequence_element_type_for_hint(
                                    &receiver_ty,
                                )
                                .or_else(|| self.extract_iter_item_type_from_type(&receiver_ty))
                                .or_else(|| {
                                    // A `Vec::new()` receiver infers to a bare
                                    // `Vec` (no element) context-free; collection
                                    // locals instead carry their element type
                                    // directly as a placeholder hint (`values` →
                                    // `Value`). Recover the pushed element type
                                    // from that hint — extracting if the hint is
                                    // itself a full container, else using it as
                                    // the element type.
                                    extract_simple_local_ident(&mc.receiver).and_then(|name| {
                                        self.lookup_local_placeholder_type_hint(&name).map(|hint| {
                                            extract_sequence_element_type_for_hint(hint)
                                                .or_else(|| {
                                                    self.extract_iter_item_type_from_type(hint)
                                                })
                                                .unwrap_or_else(|| hint.clone())
                                        })
                                    })
                                })
                                {
                                    return Some(elem_ty);
                                }
                            }
                            "insert" | "try_insert" => {
                                if let Some((key_ty, value_ty)) =
                                    extract_hashmap_key_value_types_for_hint(&receiver_ty).or_else(
                                        || extract_map_key_value_types_for_hint(&receiver_ty),
                                    )
                                {
                                    return Some(if idx == 0 { key_ty } else { value_ty });
                                }
                            }
                            _ => {}
                        }
                    }
                }
                self.infer_local_binding_type_from_expr_usage(binding_name, &mc.receiver)
                    .or_else(|| {
                        mc.args.iter().find_map(|arg| {
                            self.infer_local_binding_type_from_expr_usage(binding_name, arg)
                        })
                    })
            }
            syn::Expr::Call(call) => {
                for (idx, arg) in call.args.iter().enumerate() {
                    if extract_simple_local_ident(arg).as_deref() == Some(binding_name)
                        && let Some(expected_ty) =
                            self.lookup_function_arg_expected_type_for_call(call, idx, None)
                    {
                        return Some(expected_ty);
                    }
                }
                self.infer_local_binding_type_from_expr_usage(binding_name, &call.func)
                    .or_else(|| {
                        call.args.iter().find_map(|arg| {
                            self.infer_local_binding_type_from_expr_usage(binding_name, arg)
                        })
                    })
            }
            syn::Expr::Block(block) => {
                self.infer_local_binding_type_from_body_usage(binding_name, &block.block.stmts)
            }
            syn::Expr::If(if_expr) => self
                .infer_local_binding_type_from_expr_usage(binding_name, &if_expr.cond)
                .or_else(|| {
                    self.infer_local_binding_type_from_body_usage(
                        binding_name,
                        &if_expr.then_branch.stmts,
                    )
                })
                .or_else(|| {
                    if_expr.else_branch.as_ref().and_then(|(_, else_expr)| {
                        self.infer_local_binding_type_from_expr_usage(binding_name, else_expr)
                    })
                }),
            syn::Expr::While(while_expr) => self
                .infer_local_binding_type_from_expr_usage(binding_name, &while_expr.cond)
                .or_else(|| {
                    self.infer_local_binding_type_from_body_usage(
                        binding_name,
                        &while_expr.body.stmts,
                    )
                }),
            syn::Expr::Loop(loop_expr) => {
                self.infer_local_binding_type_from_body_usage(binding_name, &loop_expr.body.stmts)
            }
            syn::Expr::ForLoop(for_expr) => self
                .infer_local_binding_type_from_expr_usage(binding_name, &for_expr.expr)
                .or_else(|| {
                    self.infer_local_binding_type_from_body_usage(
                        binding_name,
                        &for_expr.body.stmts,
                    )
                }),
            syn::Expr::Match(match_expr) => self
                .infer_local_binding_type_from_expr_usage(binding_name, &match_expr.expr)
                .or_else(|| {
                    match_expr.arms.iter().find_map(|arm| {
                        arm.guard
                            .as_ref()
                            .and_then(|(_, guard)| {
                                self.infer_local_binding_type_from_expr_usage(binding_name, guard)
                            })
                            .or_else(|| {
                                self.infer_local_binding_type_from_expr_usage(
                                    binding_name,
                                    &arm.body,
                                )
                            })
                    })
                }),
            syn::Expr::Unary(unary) => {
                self.infer_local_binding_type_from_expr_usage(binding_name, &unary.expr)
            }
            syn::Expr::Reference(reference) => {
                self.infer_local_binding_type_from_expr_usage(binding_name, &reference.expr)
            }
            syn::Expr::Paren(paren) => {
                self.infer_local_binding_type_from_expr_usage(binding_name, &paren.expr)
            }
            syn::Expr::Group(group) => {
                self.infer_local_binding_type_from_expr_usage(binding_name, &group.expr)
            }
            syn::Expr::Array(array) => array
                .elems
                .iter()
                .find_map(|elem| self.infer_local_binding_type_from_expr_usage(binding_name, elem)),
            syn::Expr::Tuple(tuple) => tuple
                .elems
                .iter()
                .find_map(|elem| self.infer_local_binding_type_from_expr_usage(binding_name, elem)),
            syn::Expr::Struct(struct_expr) => struct_expr
                .fields
                .iter()
                .find_map(|field| {
                    self.infer_local_binding_type_from_expr_usage(binding_name, &field.expr)
                })
                .or_else(|| {
                    struct_expr.rest.as_ref().and_then(|rest| {
                        self.infer_local_binding_type_from_expr_usage(binding_name, rest)
                    })
                }),
            syn::Expr::Unsafe(unsafe_expr) => self
                .infer_local_binding_type_from_body_usage(binding_name, &unsafe_expr.block.stmts),
            syn::Expr::Closure(closure) => {
                self.infer_local_binding_type_from_expr_usage(binding_name, &closure.body)
            }
            syn::Expr::Await(await_expr) => {
                self.infer_local_binding_type_from_expr_usage(binding_name, &await_expr.base)
            }
            syn::Expr::Try(try_expr) => {
                self.infer_local_binding_type_from_expr_usage(binding_name, &try_expr.expr)
            }
            syn::Expr::Break(brk) => brk.expr.as_ref().and_then(|value| {
                self.infer_local_binding_type_from_expr_usage(binding_name, value)
            }),
            syn::Expr::Return(ret) => ret.expr.as_ref().and_then(|value| {
                self.infer_local_binding_type_from_expr_usage(binding_name, value)
            }),
            syn::Expr::Let(let_expr) => {
                self.infer_local_binding_type_from_expr_usage(binding_name, &let_expr.expr)
            }
            _ => None,
        }
    }

    pub(super) fn resolve_type_infers_from_expected(&self, ty: &syn::Type, expected: &syn::Type) -> syn::Type {
        fn recurse(out: &mut syn::Type, expected: &syn::Type) {
            if matches!(out, syn::Type::Infer(_)) {
                *out = expected.clone();
                return;
            }
            match out {
                syn::Type::Path(out_tp) => {
                    let syn::Type::Path(expected_tp) = expected else {
                        return;
                    };
                    let Some(out_last) = out_tp.path.segments.last_mut() else {
                        return;
                    };
                    let Some(expected_last) = expected_tp.path.segments.last() else {
                        return;
                    };
                    if out_last.ident != expected_last.ident {
                        return;
                    }
                    let (
                        syn::PathArguments::AngleBracketed(out_args),
                        syn::PathArguments::AngleBracketed(expected_args),
                    ) = (&mut out_last.arguments, &expected_last.arguments)
                    else {
                        return;
                    };
                    for (out_arg, expected_arg) in
                        out_args.args.iter_mut().zip(expected_args.args.iter())
                    {
                        match (out_arg, expected_arg) {
                            (
                                syn::GenericArgument::Type(out_ty),
                                syn::GenericArgument::Type(expected_ty),
                            ) => recurse(out_ty, expected_ty),
                            (
                                syn::GenericArgument::Const(out_const),
                                syn::GenericArgument::Const(expected_const),
                            ) if matches!(out_const, syn::Expr::Infer(_)) => {
                                *out_const = expected_const.clone();
                            }
                            _ => {}
                        }
                    }
                }
                syn::Type::Reference(out_ref) => {
                    let syn::Type::Reference(expected_ref) = expected else {
                        return;
                    };
                    recurse(&mut out_ref.elem, &expected_ref.elem);
                }
                syn::Type::Ptr(out_ptr) => {
                    let syn::Type::Ptr(expected_ptr) = expected else {
                        return;
                    };
                    recurse(&mut out_ptr.elem, &expected_ptr.elem);
                }
                syn::Type::Paren(out_paren) => {
                    let syn::Type::Paren(expected_paren) = expected else {
                        return;
                    };
                    recurse(&mut out_paren.elem, &expected_paren.elem);
                }
                syn::Type::Group(out_group) => {
                    let syn::Type::Group(expected_group) = expected else {
                        return;
                    };
                    recurse(&mut out_group.elem, &expected_group.elem);
                }
                syn::Type::Tuple(out_tuple) => {
                    let syn::Type::Tuple(expected_tuple) = expected else {
                        return;
                    };
                    for (out_elem, expected_elem) in
                        out_tuple.elems.iter_mut().zip(expected_tuple.elems.iter())
                    {
                        recurse(out_elem, expected_elem);
                    }
                }
                syn::Type::Array(out_array) => {
                    let syn::Type::Array(expected_array) = expected else {
                        return;
                    };
                    recurse(&mut out_array.elem, &expected_array.elem);
                }
                syn::Type::Slice(out_slice) => {
                    let syn::Type::Slice(expected_slice) = expected else {
                        return;
                    };
                    recurse(&mut out_slice.elem, &expected_slice.elem);
                }
                _ => {}
            }
        }

        let mut out = ty.clone();
        recurse(&mut out, expected);
        out
    }

    pub(super) fn resolve_return_type_infers_from_expected(
        &self,
        output: &syn::ReturnType,
        expected_output: Option<&syn::ReturnType>,
    ) -> syn::ReturnType {
        let syn::ReturnType::Type(_, out_ty) = output else {
            return output.clone();
        };
        let Some(expected_output) = expected_output else {
            return output.clone();
        };
        let syn::ReturnType::Type(_, expected_ty) = expected_output else {
            return output.clone();
        };
        if self.type_is_bare_generic_param_like(expected_ty) {
            // Bare placeholder-like callable/type parameters (e.g. FI/FO/T)
            // are not concrete closure return types.
            return output.clone();
        }
        let expected_peeled = self.peel_reference_paren_group_type(expected_ty);
        if let syn::Type::Path(tp) = expected_peeled
            && tp.qself.is_none()
            && tp.path.segments.len() == 1
        {
            let ident = tp.path.segments[0].ident.to_string();
            if self.is_type_param_in_scope(&ident) || self.is_struct_type_param(&ident) {
                // Do not resolve closure `-> _` to a bare callable/type parameter
                // such as `FI`/`FO`; that parameter denotes the callable type, not
                // the closure return type.
                return output.clone();
            }
        }
        if self.type_contains_infer(expected_ty)
            || self.type_contains_in_scope_type_param(expected_ty)
            || self.type_contains_unresolved_placeholder_like(expected_ty)
        {
            return output.clone();
        }
        syn::ReturnType::Type(
            Default::default(),
            Box::new(self.resolve_type_infers_from_expected(out_ty, expected_ty)),
        )
    }

    pub(super) fn infer_local_type_from_placeholder_hint(
        &self,
        local: &syn::Local,
        binding_name: &str,
    ) -> Option<syn::Type> {
        let hint = self.lookup_local_placeholder_type_hint(binding_name)?;
        let init = local.init.as_ref()?;

        let init_expr = self.peel_paren_group_expr(&init.expr);
        if Self::is_unsuffixed_int_literal_expr(init_expr) {
            return Some(hint.clone());
        }
        // Two-parameter generic owner shape: when the hint is
        // already a `HashMap<K, V>` (or `BTreeMap<K, V>`) and the
        // init is the matching `Owner::new()` call, return the hint
        // as-is. The two-param scan
        // (`augment_two_param_local_type_hints_from_usage`) writes
        // fully resolved owner types into the hint map rather than
        // bare inner types, so the single-inner wrapping logic
        // below doesn't apply. See Ch. 13 of
        // `docs/rusty-cpp-transpiler.md` for the design.
        if let syn::Type::Path(hint_tp) = hint
            && let Some(hint_last) = hint_tp.path.segments.last()
            && matches!(hint_last.ident.to_string().as_str(), "HashMap" | "BTreeMap")
            && matches!(
                hint_last.arguments,
                syn::PathArguments::AngleBracketed(_)
            )
            && let syn::Expr::Call(call) = init_expr
            && let syn::Expr::Path(init_path) = call.func.as_ref()
            && let Some(init_owner_seg) = init_path
                .path
                .segments
                .iter()
                .nth_back(1)
            && init_owner_seg.ident == hint_last.ident
        {
            return Some(hint.clone());
        }
        if let Some(cap_expr) = self.infer_array_capacity_arg_for_expr(init_expr) {
            let elem_ty = extract_array_element_type_for_hint(hint)
                .or_else(|| extract_sequence_element_type_for_hint(hint))
                .unwrap_or_else(|| hint.clone());
            if let Ok(cap_expr) = syn::parse_str::<syn::Expr>(&cap_expr) {
                if let Ok(inferred) = syn::parse2::<syn::Type>(quote!([#elem_ty; #cap_expr])) {
                    return Some(inferred);
                }
            }
        }

        if expr_is_option_none_constructor(init_expr) {
            let normalized_hint =
                normalize_placeholder_hint_for_owner(Some("Option"), hint.clone());
            let option_inner_hint =
                extract_option_inner_type_for_hint(&normalized_hint).unwrap_or(normalized_hint);
            if let Ok(inferred) = syn::parse2::<syn::Type>(quote!(Option<#option_inner_hint>)) {
                return Some(inferred);
            }
            return None;
        }

        let call = match init_expr {
            syn::Expr::Call(call) => call,
            _ => return None,
        };
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        let joined = path_expr
            .path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if matches!(
            joined.as_str(),
            "rusty::boxed::into_vec"
                | "into_vec"
                | "alloc::boxed::into_vec"
                | "std::boxed::into_vec"
        ) {
            let elem_ty = extract_vec_element_type_for_hint(hint)
                .or_else(|| extract_sequence_element_type_for_hint(hint))
                .unwrap_or_else(|| hint.clone());
            if let Ok(inferred) = syn::parse2::<syn::Type>(quote!(Vec<#elem_ty>)) {
                return Some(inferred);
            }
            return None;
        }
        if matches!(
            joined.as_str(),
            "alloc::vec::from_elem"
                | "std::vec::from_elem"
                | "core::vec::from_elem"
                | "vec::from_elem"
        ) {
            let elem_ty = extract_vec_element_type_for_hint(hint)
                .or_else(|| extract_sequence_element_type_for_hint(hint))
                .unwrap_or_else(|| hint.clone());
            if let Ok(inferred) = syn::parse2::<syn::Type>(quote!(Vec<#elem_ty>)) {
                return Some(inferred);
            }
            return None;
        }
        if path_expr.path.segments.len() < 2 {
            return None;
        }
        let owner_len = path_expr.path.segments.len() - 1;
        let mut owner_path = syn::Path {
            leading_colon: path_expr.path.leading_colon,
            segments: syn::punctuated::Punctuated::new(),
        };
        for seg in path_expr.path.segments.iter().take(owner_len) {
            owner_path.segments.push(seg.clone());
        }
        if owner_path.segments.is_empty() {
            return None;
        }
        let owner_seg = owner_path.segments.last_mut()?;
        let owner_name = owner_seg.ident.to_string();
        let normalized_hint =
            normalize_placeholder_hint_for_owner(Some(owner_name.as_str()), hint.clone());
        let mut replaced = false;
        if let syn::PathArguments::AngleBracketed(args) = &mut owner_seg.arguments {
            for arg in args.args.iter_mut() {
                if let syn::GenericArgument::Type(inner) = arg {
                    if matches!(inner, syn::Type::Infer(_)) {
                        *inner = normalized_hint.clone();
                        replaced = true;
                    } else if replace_type_infer_placeholders(inner, &normalized_hint) {
                        replaced = true;
                    }
                }
            }
        } else {
            if owner_name == "Default" {
                return Some(normalized_hint);
            }
            if matches!(
                owner_name.as_str(),
                "Cell" | "Vec" | "OnceCell" | "OnceBox" | "Lazy"
            ) {
                let hint = if owner_name == "Vec" {
                    extract_vec_element_type_for_hint(&normalized_hint)
                        .or_else(|| extract_sequence_element_type_for_hint(&normalized_hint))
                        .unwrap_or(normalized_hint.clone())
                } else {
                    normalized_hint.clone()
                };
                let mut args = syn::punctuated::Punctuated::new();
                args.push(syn::GenericArgument::Type(hint));
                owner_seg.arguments =
                    syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                        colon2_token: None,
                        lt_token: syn::token::Lt::default(),
                        args,
                        gt_token: syn::token::Gt::default(),
                    });
                replaced = true;
            } else if owner_name == "ArrayBuilder" {
                let elem_ty = extract_array_element_type_for_hint(&normalized_hint)
                    .or_else(|| extract_sequence_element_type_for_hint(&normalized_hint))
                    .unwrap_or(normalized_hint.clone());
                let cap_expr = self
                    .current_return_type_hint()
                    .and_then(|ret_ty| {
                        self.expected_option_type_arg(Some(ret_ty))
                            .and_then(|inner_ty| {
                                let inner_ty = self.peel_reference_paren_group_type(inner_ty);
                                if let syn::Type::Array(array) = inner_ty {
                                    Some(array.len.clone())
                                } else {
                                    None
                                }
                            })
                    })
                    .or_else(|| self.is_type_param_in_scope("N").then(|| parse_quote!(N)));
                if let Some(cap_expr) = cap_expr {
                    let mut args = syn::punctuated::Punctuated::new();
                    args.push(syn::GenericArgument::Type(elem_ty));
                    args.push(syn::GenericArgument::Const(cap_expr));
                    owner_seg.arguments =
                        syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                            colon2_token: None,
                            lt_token: syn::token::Lt::default(),
                            args,
                            gt_token: syn::token::Gt::default(),
                        });
                    replaced = true;
                }
            } else if owner_name == "ArrayVec" {
                let method_name = path_expr
                    .path
                    .segments
                    .last()
                    .map(|seg| seg.ident.to_string())
                    .unwrap_or_default();
                let cap_expr = match method_name.as_str() {
                    "from" | "try_from" => call
                        .args
                        .first()
                        .and_then(|arg| self.infer_array_capacity_arg_for_expr(arg))
                        .and_then(|cap| syn::parse_str::<syn::Expr>(&cap).ok()),
                    _ => None,
                };
                if let Some(cap_expr) = cap_expr {
                    let mut args = syn::punctuated::Punctuated::new();
                    args.push(syn::GenericArgument::Type(hint.clone()));
                    args.push(syn::GenericArgument::Const(cap_expr));
                    owner_seg.arguments =
                        syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                            colon2_token: None,
                            lt_token: syn::token::Lt::default(),
                            args,
                            gt_token: syn::token::Gt::default(),
                        });
                    replaced = true;
                }
            } else if owner_name == "ArrayString" {
                if let Some(cap_expr) = extract_arraystring_capacity_expr_for_hint(hint) {
                    let mut args = syn::punctuated::Punctuated::new();
                    args.push(syn::GenericArgument::Const(cap_expr));
                    owner_seg.arguments =
                        syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                            colon2_token: None,
                            lt_token: syn::token::Lt::default(),
                            args,
                            gt_token: syn::token::Gt::default(),
                        });
                    replaced = true;
                }
            } else if owner_name == "HashMap" {
                if let Some((key_ty, value_ty)) = extract_hashmap_key_value_types_for_hint(hint) {
                    let mut args = syn::punctuated::Punctuated::new();
                    args.push(syn::GenericArgument::Type(key_ty));
                    args.push(syn::GenericArgument::Type(value_ty));
                    owner_seg.arguments =
                        syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                            colon2_token: None,
                            lt_token: syn::token::Lt::default(),
                            args,
                            gt_token: syn::token::Gt::default(),
                        });
                    replaced = true;
                }
            } else if owner_name == "SmallVec" {
                if let Some(array_ty) = extract_smallvec_array_type_for_hint(hint) {
                    let mut args = syn::punctuated::Punctuated::new();
                    args.push(syn::GenericArgument::Type(array_ty));
                    owner_seg.arguments =
                        syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                            colon2_token: None,
                            lt_token: syn::token::Lt::default(),
                            args,
                            gt_token: syn::token::Gt::default(),
                        });
                    replaced = true;
                }
            }
        }
        if !replaced {
            return None;
        }
        Some(syn::Type::Path(syn::TypePath {
            qself: None,
            path: owner_path,
        }))
    }

    pub(super) fn infer_local_binding_type_from_current_struct_field(
        &self,
        local: &syn::Local,
        binding_name: &str,
    ) -> Option<syn::Type> {
        let current_struct = self.current_struct.as_ref()?;
        let field_ty = self.lookup_struct_field_type(current_struct, binding_name)?;
        let init = local.init.as_ref()?;
        if let syn::Expr::Match(match_expr) = self.peel_paren_group_expr(&init.expr) {
            if self
                .infer_match_expr_common_arm_type(match_expr)
                .as_ref()
                .is_some_and(|inferred| Self::types_equivalent_by_tokens(inferred, &field_ty))
                || self.match_expr_constructs_expected_owner(match_expr, &field_ty)
            {
                return Some(field_ty);
            }
        }
        let syn::Expr::Call(call) = self.peel_paren_group_expr(&init.expr) else {
            return None;
        };
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        if path_expr.path.segments.len() < 2 {
            return None;
        }
        let owner_seg = path_expr.path.segments.iter().nth_back(1)?;
        let owner_name = owner_seg.ident.to_string();
        match owner_name.as_str() {
            "Vec" if extract_vec_element_type_for_hint(&field_ty).is_some() => Some(field_ty),
            "ArrayVec" if extract_arrayvec_element_type_for_hint(&field_ty).is_some() => {
                Some(field_ty)
            }
            "ArrayString" if extract_arraystring_capacity_expr_for_hint(&field_ty).is_some() => {
                Some(field_ty)
            }
            "HashMap" if extract_hashmap_key_value_types_for_hint(&field_ty).is_some() => {
                Some(field_ty)
            }
            "SmallVec" if extract_smallvec_array_type_for_hint(&field_ty).is_some() => {
                Some(field_ty)
            }
            "Cell" => {
                let peeled = peel_reference_paren_group_type_hint(&field_ty);
                let syn::Type::Path(tp) = peeled else {
                    return None;
                };
                let last = tp.path.segments.last()?;
                (last.ident == "Cell").then_some(field_ty)
            }
            _ => None,
        }
    }

    /// Infer the return type of a closure from its body expression.
    /// Used for generic type inference: `|| 92` → i32, `|| "hello".to_string()` → String.
    pub(super) fn infer_type_from_closure_body(&self, closure: &syn::ExprClosure) -> Option<syn::Type> {
        let closure_expr = syn::Expr::Closure(closure.clone());
        if let Some(inferred) = self.infer_closure_return_type(&closure_expr) {
            return Some(inferred);
        }
        if let syn::ReturnType::Type(_, ty) = &closure.output {
            return Some((**ty).clone());
        }
        match closure.body.as_ref() {
            syn::Expr::Lit(lit) => match &lit.lit {
                syn::Lit::Int(_) => Some(syn::parse_quote!(i32)),
                syn::Lit::Float(_) => Some(syn::parse_quote!(f64)),
                syn::Lit::Str(_) => Some(syn::parse_quote!(String)),
                syn::Lit::Bool(_) => Some(syn::parse_quote!(bool)),
                _ => None,
            },
            syn::Expr::Call(_call) => self.infer_local_binding_type_from_initializer(&closure.body),
            syn::Expr::MethodCall(mc) => {
                let method = mc.method.to_string();
                if method == "to_string" && mc.args.is_empty() {
                    Some(syn::parse_quote!(String))
                } else if method == "to_owned" && mc.args.is_empty() {
                    // `<Self as ToOwned>::Owned` from the receiver (str -> String,
                    // [T] -> Vec<T>, Clone blanket -> Self), not a blanket String.
                    self.infer_to_owned_result_type(&mc.receiver)
                } else {
                    None
                }
            }
            syn::Expr::Block(block) => block.block.stmts.last().and_then(|stmt| match stmt {
                syn::Stmt::Expr(expr, _) => self.infer_type_from_closure_body(&syn::ExprClosure {
                    attrs: vec![],
                    lifetimes: None,
                    constness: None,
                    movability: None,
                    asyncness: None,
                    capture: None,
                    or1_token: Default::default(),
                    inputs: Default::default(),
                    or2_token: Default::default(),
                    output: syn::ReturnType::Default,
                    body: Box::new(expr.clone()),
                }),
                _ => None,
            }),
            _ => None,
        }
    }

    /// True when `inferred` is a bare TWO-PARAMETER owner path
    /// (`HashMap`, `BTreeMap`) with no template args AND `hint` is
    /// the same owner with full template args (`HashMap<K, V>`).
    /// Used by `emit_local` to decide whether the placeholder-hint
    /// pipeline's fully-resolved type should override the initial
    /// bare-owner inference from the initializer call.
    ///
    /// Restricted to HashMap/BTreeMap to avoid clashing with the
    /// existing single-param owner machinery. For single-param
    /// owners (`Cell<T>`, `Vec<T>`, `OnceCell<T>`, …), the
    /// `recover_omitted_local_generic_type_args` mechanism at emit
    /// time already infers `T` from `Cell::new(literal)` etc. via
    /// the literal arg's type; overriding inferred_binding_ty here
    /// with the placeholder hint can preempt that and emit a
    /// doubly-wrapped type when another usage like
    /// `cell.set(other_cell)` causes the hint to become
    /// `Cell<Cell<i32>>` (the existing pipeline's wrap of the
    /// inferred inner). Surfaced by smallvec's `let cell =
    /// Cell::new(0); v.push(DropCounter(&cell));` shape where the
    /// pre-this-fix flow correctly inferred `Cell<int32_t>` from
    /// the literal arg.
    pub(super) fn bare_owner_should_yield_to_specialized_hint(
        &self,
        inferred: &syn::Type,
        hint: &syn::Type,
    ) -> bool {
        let syn::Type::Path(inferred_tp) = self.peel_reference_paren_group_type(inferred) else {
            return false;
        };
        let syn::Type::Path(hint_tp) = self.peel_reference_paren_group_type(hint) else {
            return false;
        };
        let Some(inferred_last) = inferred_tp.path.segments.last() else {
            return false;
        };
        let Some(hint_last) = hint_tp.path.segments.last() else {
            return false;
        };
        if inferred_last.ident != hint_last.ident {
            return false;
        }
        // Restrict to TWO-PARAMETER owners only — see the doc
        // comment above for why single-param Cell-style owners
        // must be left to the existing machinery.
        let ident_str = inferred_last.ident.to_string();
        if !matches!(ident_str.as_str(), "HashMap" | "BTreeMap") {
            return false;
        }
        if !matches!(inferred_last.arguments, syn::PathArguments::None) {
            return false;
        }
        matches!(
            hint_last.arguments,
            syn::PathArguments::AngleBracketed(_)
        )
    }

    /// Like `bare_owner_should_yield_to_specialized_hint`, but for any owner
    /// — including single-parameter `Vec`: true when `inferred` is a bare
    /// owner (no template args) and `hint` is the same owner specialized with
    /// args (`Vec` vs `Vec<i32>`). Used only by the impl-field-name fallback,
    /// where `hint` is the authoritative declared field type.
    pub(super) fn bare_owner_specialized_by_field_hint(
        &self,
        inferred: &syn::Type,
        hint: &syn::Type,
    ) -> bool {
        let syn::Type::Path(inferred_tp) = self.peel_reference_paren_group_type(inferred) else {
            return false;
        };
        let syn::Type::Path(hint_tp) = self.peel_reference_paren_group_type(hint) else {
            return false;
        };
        let (Some(inferred_last), Some(hint_last)) =
            (inferred_tp.path.segments.last(), hint_tp.path.segments.last())
        else {
            return false;
        };
        inferred_last.ident == hint_last.ident
            && matches!(inferred_last.arguments, syn::PathArguments::None)
            && matches!(hint_last.arguments, syn::PathArguments::AngleBracketed(_))
    }

    pub(super) fn infer_local_binding_type_from_initializer(
        &self,
        init_expr: &syn::Expr,
    ) -> Option<syn::Type> {
        let expr = self.extract_value_expr(init_expr)?;
        match expr {
            syn::Expr::Closure(closure) => match &closure.output {
                syn::ReturnType::Type(_, ty) => Some((**ty).clone()),
                syn::ReturnType::Default => None,
            },
            // Preserve string literal local typing (`let s = "..."`) so downstream
            // expected-type emission maps Rust `&str` locals to `std::string_view`
            // instead of raw C string pointers.
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(_),
                ..
            }) => self.infer_simple_expr_type(expr),
            // `&*raw_ptr` is a reference reborrow in Rust. Keep reference shape in
            // local inference so downstream method/expected-type inference sees
            // the pointee type (not the raw pointer wrapper type).
            syn::Expr::Reference(reference_expr) => {
                if let syn::Expr::Unary(unary_expr) =
                    self.peel_paren_group_expr(&reference_expr.expr)
                    && matches!(unary_expr.op, syn::UnOp::Deref(_))
                    && let Some(ptr_ty) = self.infer_simple_expr_type(&unary_expr.expr)
                    && self.is_type_raw_pointer_like(&ptr_ty)
                {
                    if let Some((pointee_ty, is_mut_ptr)) =
                        self.extract_pointer_pointee_info_from_type(&ptr_ty)
                    {
                        if reference_expr.mutability.is_some() && is_mut_ptr {
                            return Some(parse_quote!(&mut #pointee_ty));
                        }
                        return Some(parse_quote!(&#pointee_ty));
                    }
                    return Some(ptr_ty);
                }
                if let syn::Expr::Index(index_expr) =
                    self.peel_paren_group_expr(&reference_expr.expr)
                    && matches!(
                        self.peel_paren_group_expr(&index_expr.index),
                        syn::Expr::Range(_)
                    )
                    && let Some(base_ty) = self.infer_simple_expr_type(&index_expr.expr)
                {
                    let base_ty = self.peel_reference_paren_group_type(&base_ty).clone();
                    if let Some(elem_ty) = extract_sequence_element_type_for_hint(&base_ty)
                        .or_else(|| self.extract_iter_item_type_from_type(&base_ty))
                    {
                        let quoted = if reference_expr.mutability.is_some() {
                            quote!(&mut [#elem_ty])
                        } else {
                            quote!(&[#elem_ty])
                        };
                        if let Ok(inferred) = syn::parse2::<syn::Type>(quoted) {
                            return Some(inferred);
                        }
                    }
                }
                if let syn::Expr::Array(array_expr) =
                    self.peel_paren_group_expr(&reference_expr.expr)
                    && let Some(first) = array_expr.elems.first()
                    && let Some(elem_ty) = self
                        .infer_simple_expr_type(first)
                        .or_else(|| self.infer_local_binding_type_from_initializer(first))
                {
                    let quoted = if reference_expr.mutability.is_some() {
                        quote!(&mut [#elem_ty])
                    } else {
                        quote!(&[#elem_ty])
                    };
                    if let Ok(inferred) = syn::parse2::<syn::Type>(quoted) {
                        return Some(inferred);
                    }
                }
                None
            }
            syn::Expr::Struct(struct_expr) => {
                if let Some(last) = struct_expr.path.segments.last() {
                    let last_name = last.ident.to_string();
                    if let Some((enum_name, _)) = self.flattened_data_enum_variant_parts(&last_name)
                    {
                        let mut owner_path = struct_expr.path.clone();
                        if let Some(last_seg) = owner_path.segments.last_mut() {
                            last_seg.ident =
                                syn::Ident::new(&enum_name, proc_macro2::Span::call_site());
                            last_seg.arguments = syn::PathArguments::None;
                        }
                        return Some(syn::Type::Path(syn::TypePath {
                            qself: None,
                            path: owner_path,
                        }));
                    }
                }
                Some(syn::Type::Path(syn::TypePath {
                    qself: None,
                    path: struct_expr.path.clone(),
                }))
            }
            syn::Expr::Call(call) => {
                if let Some(ptr_ty) = self.infer_pointer_type_from_call_expr(call) {
                    return Some(ptr_ty);
                }
                if let syn::Expr::Path(path_expr) = call.func.as_ref() {
                    let joined = path_expr
                        .path
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    if matches!(
                        joined.as_str(),
                        "de::Error::invalid_type"
                            | "serde::de::Error::invalid_type"
                            | "serde_core::de::Error::invalid_type"
                            | "serde_json::de::Error::invalid_type"
                    ) {
                        return self.infer_serde_error_trait_static_call_return_type(None);
                    }
                    if matches!(
                        joined.as_str(),
                        "manually_drop_new"
                            | "rusty::mem::manually_drop_new"
                            | "ManuallyDrop::new"
                            | "mem::ManuallyDrop::new"
                            | "std::mem::ManuallyDrop::new"
                            | "core::mem::ManuallyDrop::new"
                    ) && call.args.len() == 1
                    {
                        if let Some(inner_ty) = self.infer_simple_expr_type(&call.args[0]) {
                            return Some(parse_quote!(rusty::mem::ManuallyDrop<#inner_ty>));
                        }
                    }
                    // rusty::as_bytes returns std::span<const uint8_t> for string_view input
                    if matches!(joined.as_str(), "rusty::as_bytes" | "as_bytes")
                        && call.args.len() == 1
                    {
                        // Return &[u8] so map_type converts it to std::span<const uint8_t>
                        return syn::parse_str::<syn::Type>("&[u8]").ok();
                    }
                    if matches!(
                        joined.as_str(),
                        "cmp::max"
                            | "cmp::min"
                            | "core::cmp::max"
                            | "core::cmp::min"
                            | "std::cmp::max"
                            | "std::cmp::min"
                    ) && !call.args.is_empty()
                    {
                        if let Some(arg_ty) = self.infer_simple_expr_type(&call.args[0]) {
                            return Some(arg_ty);
                        }
                    }
                    if matches!(
                        joined.as_str(),
                        "replace"
                            | "mem::replace"
                            | "rusty::mem::replace"
                            | "std::mem::replace"
                            | "core::mem::replace"
                    ) && !call.args.is_empty()
                    {
                        if let Some(arg_ty) = self.infer_simple_expr_type(&call.args[0]) {
                            let arg_ty = self.peel_reference_paren_group_type(&arg_ty);
                            if let syn::Type::Reference(reference_ty) = arg_ty {
                                return Some((*reference_ty.elem).clone());
                            }
                            return Some(arg_ty.clone());
                        }
                    }
                    if matches!(
                        joined.as_str(),
                        "Some"
                            | "Option::Some"
                            | "core::option::Option::Some"
                            | "std::option::Option::Some"
                    ) && call.args.len() == 1
                    {
                        if let Some(arg_ty) =
                            self.infer_simple_expr_type(&call.args[0]).or_else(|| {
                                self.infer_local_binding_type_from_initializer(&call.args[0])
                            })
                        {
                            if let Ok(option_ty) = syn::parse2::<syn::Type>(quote!(Option<#arg_ty>))
                            {
                                return Some(option_ty);
                            }
                        }
                        return None;
                    }
                    if let Some(ret_ty) =
                        self.infer_type_param_static_conversion_call_return_type(expr)
                    {
                        return Some(ret_ty);
                    }
                    if let Some(ret_ty) = self.lookup_associated_call_return_type(call) {
                        return Some(ret_ty);
                    }
                    if let Some(ret_ty) = self.lookup_function_return_type(call.func.as_ref()) {
                        return Some(ret_ty.clone());
                    }
                }
                if let Some((ctor_name, ctor_arg)) = self.extract_constructor_call_expr(expr) {
                    let arg_ty = self.infer_simple_expr_type(ctor_arg)?;
                    match ctor_name.as_str() {
                        "Left" | "Right" => {
                            let left = arg_ty.clone();
                            let right = arg_ty;
                            Some(self.make_either_type(left, right))
                        }
                        "Ok" | "Err" => {
                            let ok_ty = arg_ty.clone();
                            let err_ty = arg_ty;
                            Some(self.make_result_type(ok_ty, err_ty))
                        }
                        _ => None,
                    }
                } else {
                    if let syn::Expr::Path(path_expr) = call.func.as_ref() {
                        if path_expr.path.segments.len() == 1 && call.args.is_empty() {
                            let name = path_expr.path.segments[0].ident.to_string();
                            if let Some(local_ty) = self.lookup_local_binding_type(&name) {
                                if let Some(callable_ret) =
                                    self.extract_callable_return_type_from_type(&local_ty)
                                {
                                    return Some(callable_ret);
                                }
                                return Some(local_ty);
                            }
                            return None;
                        }
                        if path_expr.path.segments.len() >= 2 {
                            let owner_seg = path_expr.path.segments.iter().nth_back(1)?;
                            let method_seg = path_expr.path.segments.last()?;
                            let owner_name = owner_seg.ident.to_string();
                            let method_name = method_seg.ident.to_string();
                            if matches!(method_name.as_str(), "default" | "default_") {
                                let owner_len = path_expr.path.segments.len().saturating_sub(1);
                                let mut owner_path = syn::Path {
                                    leading_colon: path_expr.path.leading_colon,
                                    segments: syn::punctuated::Punctuated::new(),
                                };
                                for seg in path_expr.path.segments.iter().take(owner_len) {
                                    owner_path.segments.push(seg.clone());
                                }
                                if owner_path.segments.len() == 1
                                    && owner_path
                                        .segments
                                        .first()
                                        .is_some_and(|seg| seg.ident == "Default")
                                {
                                    return None;
                                }
                                if !owner_path.segments.is_empty() {
                                    let owner_ty = syn::Type::Path(syn::TypePath {
                                        qself: None,
                                        path: owner_path,
                                    });
                                    if !self.type_contains_infer(&owner_ty)
                                        && !self.type_contains_in_scope_type_param(&owner_ty)
                                        && !self
                                            .type_contains_unbound_single_letter_generic(&owner_ty)
                                    {
                                        return Some(owner_ty);
                                    }
                                }
                            }
                            // `T::new()` / `T::new_()` typically returns `Self = T`.
                            // Accept both the angle-bracketed form (`Vec::<int>::new()`)
                            // and the bare form (`OnceNonZeroUsize::new()`) so the
                            // resulting local binding type can be looked up downstream
                            // (e.g. for `method_call_receiver_owner_tail`). When the
                            // owner is generic but no args are given, the recovered
                            // owner type lacks the generics; that's OK because the
                            // owner-tail-name extraction only needs the identifier.
                            if matches!(method_name.as_str(), "new" | "new_")
                                && matches!(
                                    owner_seg.arguments,
                                    syn::PathArguments::AngleBracketed(_)
                                        | syn::PathArguments::None
                                )
                            {
                                let owner_len = path_expr.path.segments.len().saturating_sub(1);
                                let mut owner_path = syn::Path {
                                    leading_colon: path_expr.path.leading_colon,
                                    segments: syn::punctuated::Punctuated::new(),
                                };
                                for seg in path_expr.path.segments.iter().take(owner_len) {
                                    owner_path.segments.push(seg.clone());
                                }
                                if !owner_path.segments.is_empty() {
                                    let owner_ty = syn::Type::Path(syn::TypePath {
                                        qself: None,
                                        path: owner_path,
                                    });
                                    if !self.type_contains_infer(&owner_ty) {
                                        return Some(owner_ty);
                                    }
                                }
                            }
                            if owner_name == "OnceCell"
                                && matches!(
                                    method_name.as_str(),
                                    "new" | "new_" | "with_value" | "from"
                                )
                            {
                                if let syn::PathArguments::AngleBracketed(owner_args) =
                                    &owner_seg.arguments
                                {
                                    let inner_ty =
                                        owner_args.args.iter().find_map(|arg| match arg {
                                            syn::GenericArgument::Type(t)
                                                if !self.type_contains_infer(t) =>
                                            {
                                                Some(t.clone())
                                            }
                                            _ => None,
                                        });
                                    if let Some(inner_ty) = inner_ty {
                                        return Some(parse_quote!(OnceCell<#inner_ty>));
                                    }
                                }
                                if matches!(method_name.as_str(), "with_value" | "from") {
                                    if let Some(arg) = call.args.first() {
                                        if let Some(inner_ty) = self
                                            .infer_hint_type_from_expr(arg)
                                            .or_else(|| self.infer_simple_expr_type(arg))
                                        {
                                            return Some(parse_quote!(OnceCell<#inner_ty>));
                                        }
                                    }
                                }
                            }
                            if owner_name == "OnceBox"
                                && matches!(
                                    method_name.as_str(),
                                    "new" | "new_" | "from" | "with_value"
                                )
                            {
                                if let syn::PathArguments::AngleBracketed(owner_args) =
                                    &owner_seg.arguments
                                {
                                    let inner_ty =
                                        owner_args.args.iter().find_map(|arg| match arg {
                                            syn::GenericArgument::Type(t)
                                                if !self.type_contains_infer(t) =>
                                            {
                                                Some(t.clone())
                                            }
                                            _ => None,
                                        });
                                    if let Some(inner_ty) = inner_ty {
                                        if let Ok(inferred) =
                                            syn::parse2::<syn::Type>(quote!(OnceBox<#inner_ty>))
                                        {
                                            return Some(inferred);
                                        }
                                    }
                                }
                                if matches!(method_name.as_str(), "from" | "with_value")
                                    && let Some(arg) = call.args.first()
                                    && let Some(arg_ty) = self
                                        .infer_hint_type_from_expr(arg)
                                        .or_else(|| self.infer_simple_expr_type(arg))
                                {
                                    let boxed_inner =
                                        self.peel_reference_paren_group_type(&arg_ty).clone();
                                    if let syn::Type::Path(tp) = &boxed_inner
                                        && let Some(last) = tp.path.segments.last()
                                        && last.ident == "Box"
                                        && let syn::PathArguments::AngleBracketed(args) =
                                            &last.arguments
                                        && let Some(syn::GenericArgument::Type(inner)) = args
                                            .args
                                            .iter()
                                            .find(|a| matches!(a, syn::GenericArgument::Type(_)))
                                    {
                                        if let Ok(inferred) =
                                            syn::parse2::<syn::Type>(quote!(OnceBox<#inner>))
                                        {
                                            return Some(inferred);
                                        }
                                    }
                                    if let Ok(inferred) =
                                        syn::parse2::<syn::Type>(quote!(OnceBox<#arg_ty>))
                                    {
                                        return Some(inferred);
                                    }
                                }
                            }
                            if owner_name == "Box"
                                && matches!(method_name.as_str(), "new" | "new_" | "make")
                            {
                                if let syn::PathArguments::AngleBracketed(owner_args) =
                                    &owner_seg.arguments
                                {
                                    let explicit_inner =
                                        owner_args.args.iter().find_map(|arg| match arg {
                                            syn::GenericArgument::Type(t)
                                                if !self.type_contains_infer(t) =>
                                            {
                                                Some(t.clone())
                                            }
                                            _ => None,
                                        });
                                    if let Some(inner_ty) = explicit_inner {
                                        if let Ok(inferred) =
                                            syn::parse2::<syn::Type>(quote!(Box<#inner_ty>))
                                        {
                                            return Some(inferred);
                                        }
                                    }
                                }
                                if let Some(arg) = call.args.first()
                                    && let Some(inner_ty) = self
                                        .infer_hint_type_from_expr(arg)
                                        .or_else(|| self.infer_simple_expr_type(arg))
                                {
                                    if let Ok(inferred) =
                                        syn::parse2::<syn::Type>(quote!(Box<#inner_ty>))
                                    {
                                        return Some(inferred);
                                    }
                                }
                            }
                            if owner_name == "Lazy"
                                && matches!(method_name.as_str(), "new" | "new_" | "into_value")
                            {
                                if let syn::PathArguments::AngleBracketed(owner_args) =
                                    &owner_seg.arguments
                                {
                                    let inner_ty =
                                        owner_args.args.iter().find_map(|arg| match arg {
                                            syn::GenericArgument::Type(t)
                                                if !self.type_contains_infer(t) =>
                                            {
                                                Some(t.clone())
                                            }
                                            _ => None,
                                        });
                                    if let Some(inner_ty) = inner_ty {
                                        if let Ok(inferred) =
                                            syn::parse2::<syn::Type>(quote!(Lazy<#inner_ty>))
                                        {
                                            return Some(inferred);
                                        }
                                    }
                                }
                                if matches!(method_name.as_str(), "new" | "new_")
                                    && let Some(arg) = call.args.first()
                                    && let Some(inner_ty) = self
                                        .infer_closure_return_type(arg)
                                        .or_else(|| self.infer_hint_type_from_expr(arg))
                                        .or_else(|| self.infer_simple_expr_type(arg))
                                {
                                    if let Ok(inferred) =
                                        syn::parse2::<syn::Type>(quote!(Lazy<#inner_ty>))
                                    {
                                        return Some(inferred);
                                    }
                                }
                                if method_name == "into_value"
                                    && let Some(arg) = call.args.first()
                                    && let Some(arg_ty) = self
                                        .infer_hint_type_from_expr(arg)
                                        .or_else(|| self.infer_simple_expr_type(arg))
                                {
                                    let arg_ty = self.peel_reference_paren_group_type(&arg_ty);
                                    if let syn::Type::Path(tp) = arg_ty
                                        && let Some(last) = tp.path.segments.last()
                                        && last.ident == "Lazy"
                                        && let syn::PathArguments::AngleBracketed(args) =
                                            &last.arguments
                                        && let Some(syn::GenericArgument::Type(inner_ty)) = args
                                            .args
                                            .iter()
                                            .find(|a| matches!(a, syn::GenericArgument::Type(_)))
                                    {
                                        return Some(parse_quote!(Lazy<#inner_ty>));
                                    }
                                }
                            }
                            if owner_name == "ArrayVec" {
                                let mut elem_ty: Option<syn::Type> = None;
                                let mut cap_expr: Option<syn::Expr> = None;
                                if let syn::PathArguments::AngleBracketed(owner_args) =
                                    &owner_seg.arguments
                                {
                                    for arg in &owner_args.args {
                                        match arg {
                                            syn::GenericArgument::Type(t)
                                                if !matches!(t, syn::Type::Infer(_)) =>
                                            {
                                                elem_ty = Some(t.clone());
                                            }
                                            syn::GenericArgument::Const(c) => {
                                                cap_expr = Some(c.clone());
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                match method_name.as_str() {
                                    "from" | "try_from" => {
                                        if let Some(arg) = call.args.first() {
                                            if elem_ty.is_none() {
                                                elem_ty =
                                                    self.infer_array_element_type_from_expr(arg);
                                            }
                                            if cap_expr.is_none() {
                                                if let Some(arg_ty) =
                                                    self.infer_simple_expr_type(arg)
                                                {
                                                    if let syn::Type::Array(arr) = arg_ty {
                                                        cap_expr = Some(arr.len.clone());
                                                    }
                                                }
                                                if cap_expr.is_none() {
                                                    if let syn::Expr::Repeat(repeat) =
                                                        self.peel_paren_group_expr(arg)
                                                    {
                                                        cap_expr = Some((*repeat.len).clone());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    "from_iter" => {
                                        if let Some(arg) = call.args.first() {
                                            if elem_ty.is_none() {
                                                elem_ty = self.infer_iter_item_type_from_expr(arg);
                                            }
                                        }
                                    }
                                    "new" | "new_" => {}
                                    _ => {}
                                }
                                if let (Some(elem_ty), Some(cap_expr)) = (elem_ty, cap_expr) {
                                    if let Ok(inferred) = syn::parse2::<syn::Type>(
                                        quote!(ArrayVec<#elem_ty, #cap_expr>),
                                    ) {
                                        return Some(inferred);
                                    }
                                }
                            }
                            if owner_name == "ArrayBuilder"
                                && matches!(method_name.as_str(), "new" | "new_")
                            {
                                let mut elem_ty: Option<syn::Type> = None;
                                let mut cap_expr: Option<syn::Expr> = None;
                                if let syn::PathArguments::AngleBracketed(owner_args) =
                                    &owner_seg.arguments
                                {
                                    for arg in &owner_args.args {
                                        match arg {
                                            syn::GenericArgument::Type(t)
                                                if !matches!(t, syn::Type::Infer(_)) =>
                                            {
                                                elem_ty = Some(t.clone());
                                            }
                                            syn::GenericArgument::Const(c)
                                                if !matches!(c, syn::Expr::Infer(_)) =>
                                            {
                                                cap_expr = Some(c.clone());
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                if (elem_ty.is_none() || cap_expr.is_none())
                                    && let Some(ret_ty) = self.current_return_type_hint()
                                    && let Some(inner_ty) =
                                        self.expected_option_type_arg(Some(ret_ty))
                                {
                                    let inner_ty = self.peel_reference_paren_group_type(inner_ty);
                                    if let syn::Type::Array(arr) = inner_ty {
                                        if elem_ty.is_none() {
                                            elem_ty = Some((*arr.elem).clone());
                                        }
                                        if cap_expr.is_none() {
                                            cap_expr = Some(arr.len.clone());
                                        }
                                    }
                                }
                                if cap_expr.is_none() {
                                    let fallback_const = self
                                        .ordered_type_params_in_scope()
                                        .into_iter()
                                        .find(|name| {
                                            self.declared_type_param_kinds.values().any(|kinds| {
                                                kinds.iter().enumerate().any(|(idx, kind)| {
                                                    matches!(kind, GenericParamKind::Const)
                                                        && self.declared_type_params.values().any(
                                                            |params| {
                                                                params
                                                                    .get(idx)
                                                                    .is_some_and(|p| p == name)
                                                            },
                                                        )
                                                })
                                            })
                                        })
                                        .or_else(|| {
                                            self.is_type_param_in_scope("N")
                                                .then_some("N".to_string())
                                        })
                                        .or_else(|| {
                                            self.is_type_param_in_scope("K")
                                                .then_some("K".to_string())
                                        })
                                        .or_else(|| {
                                            self.is_type_param_in_scope("CAP")
                                                .then_some("CAP".to_string())
                                        });
                                    if let Some(param) = fallback_const
                                        && let Ok(parsed) = syn::parse_str::<syn::Expr>(&param)
                                    {
                                        cap_expr = Some(parsed);
                                    }
                                }
                                if let (Some(elem_ty), Some(cap_expr)) = (elem_ty, cap_expr)
                                    && let Ok(inferred) = syn::parse2::<syn::Type>(
                                        quote!(ArrayBuilder<#elem_ty, #cap_expr>),
                                    )
                                {
                                    return Some(inferred);
                                }
                            }
                            if owner_name == "ArrayString"
                                && matches!(
                                    method_name.as_str(),
                                    "new" | "new_" | "from" | "try_from" | "from_byte_string"
                                )
                            {
                                if let syn::PathArguments::AngleBracketed(owner_args) =
                                    &owner_seg.arguments
                                {
                                    let cap_expr =
                                        owner_args.args.iter().find_map(|arg| match arg {
                                            syn::GenericArgument::Const(c)
                                                if !matches!(c, syn::Expr::Infer(_)) =>
                                            {
                                                Some(c.clone())
                                            }
                                            _ => None,
                                        });
                                    if let Some(cap_expr) = cap_expr {
                                        let inferred: syn::Type =
                                            parse_quote!(ArrayString<#cap_expr>);
                                        return Some(inferred);
                                    }
                                }
                            }
                            if owner_name == "Cell"
                                && matches!(method_name.as_str(), "new" | "new_")
                            {
                                if let Some(arg) = call.args.first() {
                                    if let Some(elem_ty) = self.infer_hint_type_from_expr(arg) {
                                        if let Ok(inferred) =
                                            syn::parse2::<syn::Type>(quote!(Cell<#elem_ty>))
                                        {
                                            return Some(inferred);
                                        }
                                    }
                                }
                            }
                            if owner_name == "HashMap"
                                && matches!(method_name.as_str(), "new" | "new_")
                            {
                                if let syn::PathArguments::AngleBracketed(owner_args) =
                                    &owner_seg.arguments
                                {
                                    let mut type_args =
                                        owner_args.args.iter().filter_map(|arg| match arg {
                                            syn::GenericArgument::Type(t)
                                                if !matches!(t, syn::Type::Infer(_)) =>
                                            {
                                                Some(t.clone())
                                            }
                                            _ => None,
                                        });
                                    if let (Some(key_ty), Some(value_ty)) =
                                        (type_args.next(), type_args.next())
                                    {
                                        let inferred: syn::Type =
                                            parse_quote!(HashMap<#key_ty, #value_ty>);
                                        return Some(inferred);
                                    }
                                }
                            }
                            if owner_name == "SmallVec"
                                && matches!(
                                    method_name.as_str(),
                                    "new" | "new_" | "from" | "try_from" | "from_vec" | "from_iter"
                                )
                            {
                                if let syn::PathArguments::AngleBracketed(owner_args) =
                                    &owner_seg.arguments
                                {
                                    let array_ty =
                                        owner_args.args.iter().find_map(|arg| match arg {
                                            syn::GenericArgument::Type(t)
                                                if !self.type_contains_infer(t) =>
                                            {
                                                Some(t.clone())
                                            }
                                            _ => None,
                                        });
                                    if let Some(array_ty) = array_ty {
                                        let inferred: syn::Type = parse_quote!(SmallVec<#array_ty>);
                                        return Some(inferred);
                                    }
                                }
                            }
                        }
                    }
                    if call.args.is_empty() {
                        if let syn::Expr::Path(path) = call.func.as_ref() {
                            if path.path.segments.len() == 1 {
                                let name = path.path.segments[0].ident.to_string();
                                if let Some(local_ty) = self.lookup_local_binding_type(&name) {
                                    if let Some(callable_ret) =
                                        self.extract_callable_return_type_from_type(&local_ty)
                                    {
                                        return Some(callable_ret);
                                    }
                                    return Some(local_ty);
                                }
                            }
                        }
                    }
                    None
                }
            }
            syn::Expr::Try(try_expr) => self
                .infer_local_binding_type_from_initializer(&try_expr.expr)
                .or_else(|| self.infer_simple_expr_type(&try_expr.expr))
                .and_then(|ty| self.try_unwrap_try_operand_type(&ty)),
            syn::Expr::If(if_expr) => self
                .infer_constructor_expected_type_from_if(if_expr)
                .or_else(|| self.infer_common_value_type_from_if(if_expr)),
            syn::Expr::Match(match_expr) => self
                .infer_constructor_expected_type_from_match(match_expr)
                .or_else(|| self.infer_match_expr_common_arm_type(match_expr)),
            syn::Expr::Path(path) => {
                if path.path.segments.is_empty() {
                    return None;
                }
                if path.path.segments.len() >= 2 {
                    let owner_segments: Vec<String> = path
                        .path
                        .segments
                        .iter()
                        .take(path.path.segments.len().saturating_sub(1))
                        .map(|seg| seg.ident.to_string())
                        .collect();
                    let owner_path = owner_segments.join("::");
                    let owner_tail = owner_segments.last().cloned().unwrap_or_default();
                    let variant = path
                        .path
                        .segments
                        .last()
                        .map(|seg| seg.ident.to_string())
                        .unwrap_or_default();
                    let canonical_variant = self.canonical_variant_name(&variant).to_string();
                    let is_variant_owner = self
                        .path_matches_c_like_enum_const(&owner_path, &variant)
                        || self.path_matches_c_like_enum_const(&owner_tail, &variant)
                        || self.enum_has_variant_name(&owner_path, &canonical_variant)
                        || self.enum_has_variant_name(&owner_tail, &canonical_variant);
                    if is_variant_owner {
                        let owner_only = Self::path_without_last_segment(&path.path)?;
                        if !owner_only.segments.is_empty() {
                            return Some(syn::Type::Path(syn::TypePath {
                                qself: None,
                                path: owner_only,
                            }));
                        }
                    }
                }
                if path.path.segments.len() == 1 {
                    let name = path.path.segments[0].ident.to_string();
                    if let Some((enum_name, _)) = self.flattened_data_enum_variant_parts(&name)
                        && let Ok(enum_ty) = syn::parse_str::<syn::Type>(&enum_name)
                    {
                        return Some(enum_ty);
                    }
                    // First check declared bindings, then fall back to placeholder hints
                    // from pre-scan (handles forward references like `let x = y; let y = 5;`).
                    return self
                        .lookup_local_binding_type(&name)
                        .or_else(|| self.lookup_local_placeholder_type_hint(&name).cloned());
                }
                None
            }
            syn::Expr::Unsafe(unsafe_expr) => self
                .extract_single_expr_from_block(&unsafe_expr.block)
                .and_then(|inner| {
                    self.infer_local_binding_type_from_initializer(inner)
                        .or_else(|| self.infer_simple_expr_type(inner))
                }),
            syn::Expr::MethodCall(mc) => self.infer_method_call_result_type_for_local(mc),
            syn::Expr::Range(range) => self.infer_range_expr_type(range),
            _ => None,
        }
    }

    /// Result type of `receiver.to_owned()` — `<Self as ToOwned>::Owned`, where
    /// `Self` is the receiver type with AT MOST ONE reference peeled (the autoref
    /// `to_owned` resolves through). Returns `None` when the receiver type can't
    /// be determined, so callers defer to downstream deduction rather than
    /// guessing `String` (a wrong concrete type poisons later C++ deduction).
    pub(super) fn infer_to_owned_result_type(&self, receiver: &syn::Expr) -> Option<syn::Type> {
        let recv_ty = self
            .infer_simple_expr_type(receiver)
            .or_else(|| self.infer_local_binding_type_from_initializer(receiver))?;
        // ToOwned resolves `Self` by peeling at most one autoref: `&U`/`&mut U`
        // -> `U`; a value receiver (String, i32, Box<str>, ...) peels zero.
        let self_ty: syn::Type = match self.peel_paren_group_type(&recv_ty) {
            syn::Type::Reference(r) => self.peel_paren_group_type(&r.elem).clone(),
            other => other.clone(),
        };
        Some(self.to_owned_owned_type_from_self(&self_ty))
    }

    /// `<Self as ToOwned>::Owned` for the (already de-autoref'd) borrowed type
    /// `self_ty`. Bespoke std impls: `str -> String`, `[E] -> Vec<E>`,
    /// `Path -> PathBuf`. Everything else takes the blanket
    /// `impl<T: Clone> ToOwned` so `Owned = Self` (`i32 -> i32`, `MyStruct ->
    /// MyStruct`, generic `I -> I`, `&str -> &str` for the `&&str` case,
    /// `Box<str>`/`Cow<str>` -> themselves). The `str` match is exact — never a
    /// substring/"contains str" test — so str-family smart pointers are not
    /// mis-mapped to `String`.
    pub(super) fn to_owned_owned_type_from_self(&self, self_ty: &syn::Type) -> syn::Type {
        match self_ty {
            syn::Type::Slice(slice) => {
                let elem = &slice.elem;
                parse_quote!(Vec<#elem>)
            }
            syn::Type::Path(tp)
                if tp.qself.is_none()
                    && tp.path.segments.len() == 1
                    && matches!(tp.path.segments[0].arguments, syn::PathArguments::None) =>
            {
                match tp.path.segments[0].ident.to_string().as_str() {
                    "str" => parse_quote!(String),
                    "Path" => parse_quote!(std::path::PathBuf),
                    _ => self_ty.clone(),
                }
            }
            _ => self_ty.clone(),
        }
    }

    pub(super) fn infer_method_call_result_type_for_local(
        &self,
        mc: &syn::ExprMethodCall,
    ) -> Option<syn::Type> {
        let method = mc.method.to_string();
        if let Some(receiver_ty) = self
            .infer_simple_expr_type(&mc.receiver)
            .or_else(|| self.infer_local_binding_type_from_initializer(&mc.receiver))
            && let Some(ret_ty) =
                self.infer_unwrap_like_method_return_type_from_receiver_type(&receiver_ty, &method)
        {
            return Some(ret_ty);
        }

        // `RefCell::borrow()` / `borrow_mut()` return `Ref<T>` / `RefMut<T>`
        // BY VALUE. Track the wrapper type on the local so downstream
        // method calls on the guard route through wrapper-deref handling
        // (`guard->method()` instead of `guard.method()`).
        if matches!(method.as_str(), "borrow" | "borrow_mut") && mc.args.is_empty() {
            if let Some(receiver_ty) = self
                .infer_simple_expr_type(&mc.receiver)
                .or_else(|| self.infer_local_binding_type_from_initializer(&mc.receiver))
            {
                let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
                if let syn::Type::Path(tp) = receiver_ty
                    && let Some(last) = tp.path.segments.last()
                    && last.ident == "RefCell"
                    && let syn::PathArguments::AngleBracketed(args) = &last.arguments
                    && let Some(inner_ty) = args.args.iter().find_map(|arg| match arg {
                        syn::GenericArgument::Type(t) => Some(t.clone()),
                        _ => None,
                    })
                {
                    return Some(if method == "borrow_mut" {
                        parse_quote!(RefMut<#inner_ty>)
                    } else {
                        parse_quote!(Ref<#inner_ty>)
                    });
                }
            }
        }

        // `Mutex::lock()` / `SpinMutex::lock()` return `LockResult<T>` which
        // is `Result<MutexGuard<T>, _>` / `Result<SpinMutexGuard<T>, _>`.
        // `try_lock()` returns `Option<MutexGuard<T>>` / `Option<SpinMutexGuard<T>>`.
        if matches!(method.as_str(), "lock" | "try_lock") && mc.args.is_empty() {
            if let Some(receiver_ty) = self
                .infer_simple_expr_type(&mc.receiver)
                .or_else(|| self.infer_local_binding_type_from_initializer(&mc.receiver))
            {
                let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
                if let syn::Type::Path(tp) = receiver_ty
                    && let Some(last) = tp.path.segments.last()
                    && let syn::PathArguments::AngleBracketed(args) = &last.arguments
                    && let Some(inner_ty) = args.args.iter().find_map(|arg| match arg {
                        syn::GenericArgument::Type(t) => Some(t.clone()),
                        _ => None,
                    })
                {
                    let guard_name = match last.ident.to_string().as_str() {
                        "SpinMutex" => Some("SpinMutexGuard"),
                        "Mutex" => Some("MutexGuard"),
                        _ => None,
                    };
                    if let Some(guard_name) = guard_name {
                        let guard_ident = syn::Ident::new(guard_name, proc_macro2::Span::call_site());
                        let guard_ty: syn::Type = parse_quote!(#guard_ident<#inner_ty>);
                        return Some(if method == "try_lock" {
                            parse_quote!(Option<#guard_ty>)
                        } else {
                            // LockResult<T> = Result<MutexGuard<T>, PoisonError<T>>
                            parse_quote!(Result<#guard_ty, ()>)
                        });
                    }
                }
            }
        }

        if method == "get_mut" && mc.args.is_empty() {
            if let Some(receiver_ty) = self
                .infer_simple_expr_type(&mc.receiver)
                .or_else(|| self.infer_local_binding_type_from_initializer(&mc.receiver))
            {
                let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
                if let syn::Type::Path(tp) = receiver_ty
                    && let Some(last) = tp.path.segments.last()
                    && last.ident == "Cell"
                    && let syn::PathArguments::AngleBracketed(args) = &last.arguments
                    && let Some(inner_ty) = args.args.iter().find_map(|arg| match arg {
                        syn::GenericArgument::Type(t) => Some(t.clone()),
                        _ => None,
                    })
                {
                    return Some(parse_quote!(&mut #inner_ty));
                }
            }
        }

        if method == "take" && mc.args.is_empty() {
            if let Some(receiver_ty) = self
                .infer_simple_expr_type(&mc.receiver)
                .or_else(|| self.infer_local_binding_type_from_initializer(&mc.receiver))
                && let Some((owner, type_args)) = self.option_or_result_type_args(&receiver_ty)
                && owner == "Option"
                && let Some(inner_ty) = type_args.first().cloned()
            {
                return Some(parse_quote!(Option<#inner_ty>));
            }
        }

        if method == "pop" && mc.args.is_empty() {
            if let Some(receiver_ty) = self
                .infer_simple_expr_type(&mc.receiver)
                .or_else(|| self.infer_local_binding_type_from_initializer(&mc.receiver))
            {
                let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty).clone();
                if let Some(elem_ty) = extract_sequence_element_type_for_hint(&receiver_ty)
                    .or_else(|| self.extract_iter_item_type_from_type(&receiver_ty))
                {
                    return Some(parse_quote!(Option<#elem_ty>));
                }
            }
        }

        if matches!(method.as_str(), "ok_or" | "ok_or_else") && mc.args.len() == 1 {
            let debug_option_if = std::env::var("RUSTY_DEBUG_OPTION_IF").is_ok();
            if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver)
                && let Some((owner, type_args)) = self.option_or_result_type_args(&receiver_ty)
                && owner == "Option"
                && let Some(ok_ty) = type_args.first().cloned()
            {
                if debug_option_if {
                    eprintln!(
                        "DBG ok_or receiver_ty={} ok_ty={}",
                        quote::quote!(#receiver_ty),
                        quote::quote!(#ok_ty)
                    );
                }
                let err_ty = if method == "ok_or_else" {
                    self.infer_closure_return_type(&mc.args[0]).or_else(|| {
                        if let syn::Expr::Closure(closure) = self.peel_paren_group_expr(&mc.args[0])
                        {
                            self.infer_type_from_closure_body(closure)
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
                .or_else(|| self.infer_local_binding_type_from_initializer(&mc.args[0]))
                .or_else(|| self.infer_simple_expr_type(&mc.args[0]))
                .or_else(|| self.infer_hint_type_from_expr(&mc.args[0]))
                .unwrap_or_else(|| parse_quote!(()));
                if debug_option_if {
                    eprintln!("DBG ok_or err_ty={}", quote::quote!(#err_ty));
                }
                return Some(self.make_result_type(ok_ty, err_ty));
            } else if debug_option_if {
                let recv = self
                    .infer_simple_expr_type(&mc.receiver)
                    .map(|t| quote::quote!(#t).to_string())
                    .unwrap_or_else(|| "<none>".to_string());
                eprintln!("DBG ok_or unresolved receiver_ty={}", recv);
            }
        }

        if matches!(method.as_str(), "as_ptr" | "as_mut_ptr") && mc.args.is_empty() {
            let mut as_ptr_returns_mut = method == "as_mut_ptr";
            let pointee_ty = self
                .infer_array_element_type_from_expr(&mc.receiver)
                .or_else(|| {
                    self.infer_simple_expr_type(&mc.receiver).and_then(|ty| {
                        if let Some((pointee, is_mut_ptr)) =
                            self.extract_pointer_pointee_info_from_type(&ty)
                        {
                            if method == "as_ptr" {
                                as_ptr_returns_mut = is_mut_ptr;
                            }
                            Some(pointee)
                        } else {
                            self.extract_iter_item_type_from_type(&ty)
                        }
                    })
                })
                // When pointee type cannot be recovered from receiver context,
                // use the CURRENT STRUCT's first TYPE parameter if available.
                // Skip const generic params (like CAP) — they're not types.
                .or_else(|| {
                    if let Some(struct_name) = &self.current_struct {
                        let params = self.declared_type_params.get(struct_name)?;
                        let kinds = self.declared_type_param_kinds.get(struct_name);
                        for (idx, param) in params.iter().enumerate() {
                            let is_type = kinds
                                .and_then(|k| k.get(idx))
                                .is_some_and(|k| matches!(k, GenericParamKind::Type));
                            if is_type {
                                let param_ty = syn::parse_str::<syn::Type>(param).ok()?;
                                if struct_name == "SmallVec" {
                                    let item_ty: syn::Type =
                                        parse_quote!(rusty::detail::associated_item_t<#param_ty>);
                                    return Some(item_ty);
                                }
                                return Some(param_ty);
                            }
                        }
                        None
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| parse_quote!(u8));
            if as_ptr_returns_mut {
                return Some(parse_quote!(*mut #pointee_ty));
            }
            return Some(parse_quote!(*const #pointee_ty));
        }

        if method == "get" && mc.args.is_empty() {
            if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver) {
                let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
                if let syn::Type::Path(tp) = receiver_ty
                    && let Some(last) = tp.path.segments.last()
                    && last.ident == "UnsafeCell"
                    && let syn::PathArguments::AngleBracketed(args) = &last.arguments
                {
                    let inner_ty = args.args.iter().find_map(|arg| match arg {
                        syn::GenericArgument::Type(t) => Some(t.clone()),
                        _ => None,
                    })?;
                    return Some(parse_quote!(*mut #inner_ty));
                }
            }
        }

        if method == "load" && !mc.args.is_empty() {
            if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver) {
                let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
                if let syn::Type::Path(tp) = receiver_ty
                    && let Some(last) = tp.path.segments.last()
                    && last.ident == "AtomicPtr"
                    && let syn::PathArguments::AngleBracketed(args) = &last.arguments
                {
                    let inner_ty = args.args.iter().find_map(|arg| match arg {
                        syn::GenericArgument::Type(t) => Some(t.clone()),
                        _ => None,
                    })?;
                    return Some(parse_quote!(*mut #inner_ty));
                }
            }
        }

        if method == "as_ref" && mc.args.is_empty() {
            if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver)
                && let Some((pointee_ty, _)) =
                    self.extract_pointer_pointee_info_from_type(&receiver_ty)
            {
                return Some(parse_quote!(Option<&#pointee_ty>));
            }
            // Option::as_ref / Result::as_ref - including when wrapped in a
            // guard (e.g. `let guard = self.cell.borrow();` followed by
            // `guard.as_ref()` where guard has type `Ref<Option<T>>`).
            if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver)
                && let Some((owner, args)) = self.option_or_result_type_args(&receiver_ty)
            {
                match owner.as_str() {
                    "Option" => {
                        let inner = args.first().cloned()?;
                        return Some(parse_quote!(Option<&#inner>));
                    }
                    "Result" => {
                        let ok_ty = args.first().cloned()?;
                        let err_ty = args.get(1).cloned()?;
                        return Some(parse_quote!(Result<&#ok_ty, &#err_ty>));
                    }
                    _ => {}
                }
            }
        }

        if method == "as_mut" && mc.args.is_empty() {
            if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver) {
                if let Some((pointee_ty, is_mut_ptr)) =
                    self.extract_pointer_pointee_info_from_type(&receiver_ty)
                {
                    if is_mut_ptr {
                        return Some(parse_quote!(&mut #pointee_ty));
                    }
                }
            }
            // Option::as_mut / Result::as_mut - including through guard
            // wrappers like `RefMut<Option<T>>` / `MutexGuard<Option<T>>`.
            if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver)
                && let Some((owner, args)) = self.option_or_result_type_args(&receiver_ty)
            {
                match owner.as_str() {
                    "Option" => {
                        let inner = args.first().cloned()?;
                        return Some(parse_quote!(Option<&mut #inner>));
                    }
                    "Result" => {
                        let ok_ty = args.first().cloned()?;
                        let err_ty = args.get(1).cloned()?;
                        return Some(parse_quote!(Result<&mut #ok_ty, &mut #err_ty>));
                    }
                    _ => {}
                }
            }
        }

        // str::as_bytes() on &str / &str arguments returns std::span<const uint8_t>
        if method == "as_bytes" && mc.args.is_empty() {
            // Return &[u8] so map_type converts it to std::span<const uint8_t>
            return syn::parse_str::<syn::Type>("&[u8]").ok();
        }
        if method == "split" && mc.args.len() == 1 {
            if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver)
                && !self.is_known_string_like_type(&receiver_ty)
            {
                return None;
            }
            return syn::parse_str::<syn::Type>("rusty::str_runtime::SplitIter").ok();
        }
        if method == "hash" && mc.args.len() == 1 {
            // `Hash::hash` / `.hash(...)` always returns `()`.
            return Some(parse_quote!(()));
        }
        // str::bytes() exposes an iterator over u8 values.
        if method == "bytes" && mc.args.is_empty() {
            if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver) {
                if !self.is_known_string_like_type(&receiver_ty) {
                    return None;
                }
            }
            return syn::parse_str::<syn::Type>("&[u8]").ok();
        }
        // `x.clone()` preserves the receiver's type. Resolve it ONLY when the
        // receiver is (a reference to) a bare in-scope generic type parameter —
        // e.g. `let i = src.clone()` with `src: I` (`I: Iterator`). This is the
        // clone-chain that otherwise leaves the iterator's item type unresolved
        // (`.all`/`.fold`/`.extend` closure params leak `<auto>`); recording
        // `i: I` lets `infer_iter_item_type_with_generic_fallback` recover
        // `I::Item`. Restricting to type-parameter receivers keeps concrete-typed
        // clones (Vec/RefCell/Mutex/OnceCell/...) at today's behavior so the
        // wrapper/guard-detection paths that key off receiver types are
        // unaffected.
        if method == "clone" && mc.args.is_empty() {
            if let Some(ty) = self
                .infer_simple_expr_type(&mc.receiver)
                .or_else(|| self.infer_local_binding_type_from_initializer(&mc.receiver))
            {
                let peeled = self.peel_reference_paren_group_type(&ty);
                if let syn::Type::Path(tp) = peeled
                    && tp.qself.is_none()
                    && tp.path.segments.len() == 1
                    && matches!(tp.path.segments[0].arguments, syn::PathArguments::None)
                    && self.is_type_param_in_scope(&tp.path.segments[0].ident.to_string())
                {
                    return Some(peeled.clone());
                }
            }
        }
        if method == "to_string" && mc.args.is_empty() {
            // ToString::to_string is `-> String` for every impl (specialization
            // only changes strategy, never the return type).
            return Some(parse_quote!(rusty::String));
        }
        if method == "to_owned" && mc.args.is_empty() {
            // `<Self as ToOwned>::Owned` from the receiver type. Fall through
            // when unresolvable so the generic impl-block lookup below can still
            // find a user `ToOwned` impl.
            if let Some(owned) = self.infer_to_owned_result_type(&mc.receiver) {
                return Some(owned);
            }
        }
        if method == "split_at" && mc.args.len() == 1 {
            if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver) {
                if !self.is_known_string_like_type(&receiver_ty) {
                    return None;
                }
            } else {
                return None;
            }
            // Rust `str::split_at` returns a pair of `&str`.
            return syn::parse_str::<syn::Type>("(&str, &str)").ok();
        }

        if method == "all"
            && mc.args.len() == 1
            && self.is_iterator_like_receiver_expr(&mc.receiver)
        {
            return Some(parse_quote!(bool));
        }

        if matches!(
            method.as_str(),
            "checked_add"
                | "checked_sub"
                | "checked_mul"
                | "checked_div"
                | "checked_rem"
                | "checked_shl"
                | "checked_shr"
                | "checked_pow"
        ) && mc.args.len() == 1
        {
            if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver) {
                let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty).clone();
                return Some(parse_quote!(Option<#receiver_ty>));
            }
        }

        if matches!(
            method.as_str(),
            "saturating_add" | "saturating_sub" | "saturating_mul" | "saturating_pow"
        ) && mc.args.len() == 1
        {
            if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver) {
                let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty).clone();
                return Some(receiver_ty);
            }
        }

        if matches!(method.as_str(), "add" | "offset" | "sub") && mc.args.len() == 1 {
            if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver) {
                let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty).clone();
                if self.is_type_raw_pointer_like(&receiver_ty) {
                    return Some(receiver_ty);
                }
            }
        }

        if method == "cast" && mc.args.is_empty() {
            if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver) {
                let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
                if let syn::Type::Ptr(receiver_ptr) = receiver_ty {
                    if let Some(target_ty) = self.method_call_single_turbofish_type(mc) {
                        if receiver_ptr.mutability.is_some() {
                            return Some(parse_quote!(*mut #target_ty));
                        }
                        return Some(parse_quote!(*const #target_ty));
                    }
                }
            }
        }

        if matches!(
            method.as_str(),
            "wrapping_add" | "wrapping_sub" | "wrapping_offset"
        ) && mc.args.len() == 1
        {
            if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver) {
                let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty).clone();
                if self.is_type_raw_pointer_like(&receiver_ty) {
                    return Some(receiver_ty);
                }
            }
        }

        if matches!(method.as_str(), "next" | "next_back") && mc.args.is_empty() {
            if self.is_iterator_like_receiver_expr(&mc.receiver) {
                let item_ty = self.infer_iter_item_type_from_expr(&mc.receiver)?;
                return Some(parse_quote!(Option<#item_ty>));
            }
            let receiver = self.peel_paren_group_expr(&mc.receiver);
            if let syn::Expr::Path(path) = receiver {
                if path.path.segments.len() == 1 {
                    let name = path.path.segments[0].ident.to_string();
                    if let Some(local_ty) = self.lookup_local_binding_type(&name) {
                        let local_ty = self.peel_reference_paren_group_type(&local_ty);
                        if let syn::Type::Path(tp) = local_ty {
                            if tp.path.segments.len() == 1
                                && self
                                    .is_type_param_in_scope(&tp.path.segments[0].ident.to_string())
                            {
                                return None;
                            }
                        }
                    }
                }
            }
        }

        // iter()/iter_mut()/into_iter() on Field receiver - lookup field type directly
        if matches!(method.as_str(), "iter" | "iter_mut" | "into_iter") && mc.args.is_empty() {
            if let syn::Expr::Field(field_expr) = mc.receiver.as_ref() {
                let member_str = match &field_expr.member {
                    syn::Member::Named(ident) => ident.to_string(),
                    syn::Member::Unnamed(_) => return None,
                };
                if let Some(field_ty) =
                    self.lookup_field_type_for_expr_base(&field_expr.base, &member_str)
                {
                    if let Some(item_ty) = self.extract_iter_item_type_from_type(&field_ty) {
                        return Some(item_ty);
                    }
                }
            }
        }

        // Keep tuple/assoc return-shape inference available for self-method calls
        // so downstream local binding type tracking can collapse `*ref_like` derefs.
        let receiver_expr = self.peel_paren_group_expr(&mc.receiver);
        let receiver_is_self = matches!(
            receiver_expr,
            syn::Expr::Path(path)
                if path.path.segments.len() == 1 && path.path.segments[0].ident == "self"
        );
        if receiver_is_self {
            if let Some(ret_ty) = self.lookup_current_struct_method_return_type(&method) {
                return Some(ret_ty);
            }
        }

        if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver)
            && let Some(ret_ty) =
                self.lookup_owner_method_return_type_from_receiver_type(&receiver_ty, &method)
        {
            let ret_ty = self.substitute_self_type_with_receiver_type(ret_ty, &receiver_ty);
            return Some(
                self.substitute_single_unbound_return_type_param_from_call_args(ret_ty, &mc.args),
            );
        }

        None
    }

    pub(super) fn infer_unwrap_like_method_return_type_from_receiver_type(
        &self,
        receiver_ty: &syn::Type,
        method: &str,
    ) -> Option<syn::Type> {
        let (owner, args) = self.option_or_result_type_args(receiver_ty)?;
        match owner.as_str() {
            "Option" if matches!(method, "unwrap" | "unwrap_unchecked" | "expect") => {
                args.first().cloned()
            }
            "Result" if matches!(method, "unwrap" | "expect") => args.first().cloned(),
            "Result" if matches!(method, "unwrap_err" | "expect_err") => args.get(1).cloned(),
            _ => None,
        }
    }

    pub(super) fn infer_raw_pointer_mutability_for_expr(&self, expr: &syn::Expr) -> Option<bool> {
        let receiver_ty = self.infer_simple_expr_type(expr)?;
        let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
        let syn::Type::Ptr(ptr) = receiver_ty else {
            return None;
        };
        Some(ptr.mutability.is_some())
    }

    pub(super) fn infer_pointer_type_from_call_expr(&self, call: &syn::ExprCall) -> Option<syn::Type> {
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        let joined = path_expr
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");

        // std/core allocator entrypoints return raw byte pointers.
        if (matches!(
            joined.as_str(),
            "alloc"
                | "rusty::alloc::alloc"
                | "std::alloc::alloc"
                | "core::alloc::alloc"
                | "alloc::alloc"
                | "alloc_zeroed"
                | "rusty::alloc::alloc_zeroed"
                | "std::alloc::alloc_zeroed"
                | "core::alloc::alloc_zeroed"
                | "alloc::alloc_zeroed"
        ) && call.args.len() == 1)
            || (matches!(
                joined.as_str(),
                "realloc"
                    | "rusty::alloc::realloc"
                    | "std::alloc::realloc"
                    | "core::alloc::realloc"
                    | "alloc::realloc"
            ) && call.args.len() == 3)
        {
            return Some(parse_quote!(*mut u8));
        }

        if matches!(
            joined.as_str(),
            "as_mut_ptr" | "rusty::as_mut_ptr" | "as_ptr" | "rusty::as_ptr"
        ) && call.args.len() == 1
        {
            let mut as_ptr_returns_mut = joined.ends_with("as_mut_ptr");
            let pointee_ty = self
                .infer_array_element_type_from_expr(&call.args[0])
                .or_else(|| {
                    self.infer_simple_expr_type(&call.args[0]).and_then(|ty| {
                        if let Some((pointee, is_mut_ptr)) =
                            self.extract_pointer_pointee_info_from_type(&ty)
                        {
                            if joined.ends_with("as_ptr") {
                                as_ptr_returns_mut = is_mut_ptr;
                            }
                            Some(pointee)
                        } else {
                            self.extract_iter_item_type_from_type(&ty)
                        }
                    })
                })
                .unwrap_or_else(|| parse_quote!(u8));
            if as_ptr_returns_mut {
                return Some(parse_quote!(*mut #pointee_ty));
            }
            return Some(parse_quote!(*const #pointee_ty));
        }

        if Self::is_ptr_add_or_offset_call_path(&path_expr.path) && !call.args.is_empty() {
            if let Some(receiver_ty) = self.infer_simple_expr_type(&call.args[0]) {
                let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty).clone();
                if matches!(receiver_ty, syn::Type::Ptr(_)) {
                    return Some(receiver_ty);
                }
            }
        }

        if let Some(return_ty) = self.lookup_function_return_type(call.func.as_ref()) {
            let return_ty = self.peel_reference_paren_group_type(return_ty).clone();
            if matches!(return_ty, syn::Type::Ptr(_)) {
                return Some(return_ty);
            }
        }

        None
    }

    pub(super) fn infer_constructor_expected_type_from_if(&self, if_expr: &syn::ExprIf) -> Option<syn::Type> {
        let then_expr = self.extract_single_expr_from_block(&if_expr.then_branch)?;
        let (_, else_expr) = if_expr.else_branch.as_ref()?;
        let else_expr = self.extract_value_expr(else_expr)?;
        self.infer_constructor_expected_type_from_pair(then_expr, else_expr)
    }

    pub(super) fn infer_common_value_type_from_if(&self, if_expr: &syn::ExprIf) -> Option<syn::Type> {
        let then_expr = self
            .extract_tail_expr_from_block(&if_expr.then_branch)
            .or_else(|| self.extract_single_expr_from_block(&if_expr.then_branch))?;
        let (_, else_expr_raw) = if_expr.else_branch.as_ref()?;
        let else_expr = self.extract_value_expr(else_expr_raw)?;

        let mut then_ty = self
            .infer_local_binding_type_from_initializer(then_expr)
            .or_else(|| self.infer_simple_expr_type(then_expr));
        let mut else_ty = self
            .infer_local_binding_type_from_initializer(else_expr)
            .or_else(|| self.infer_simple_expr_type(else_expr));

        if then_ty.is_none() {
            let then_env = self.collect_pre_scan_known_local_type_hints(&if_expr.then_branch.stmts);
            then_ty = if let Some(some_arg) = self.extract_option_some_call_arg(then_expr) {
                self.infer_expr_type_with_env(some_arg, &then_env)
                    .map(|arg_ty| parse_quote!(Option<#arg_ty>))
            } else {
                self.infer_expr_type_with_env(then_expr, &then_env)
            };
        }
        if else_ty.is_none() {
            let else_env = match else_expr_raw.as_ref() {
                syn::Expr::Block(block) => {
                    self.collect_pre_scan_known_local_type_hints(&block.block.stmts)
                }
                _ => HashMap::new(),
            };
            else_ty = if let Some(some_arg) = self.extract_option_some_call_arg(else_expr) {
                self.infer_expr_type_with_env(some_arg, &else_env)
                    .map(|arg_ty| parse_quote!(Option<#arg_ty>))
            } else {
                self.infer_expr_type_with_env(else_expr, &else_env)
            };
        }

        let then_block_expr = syn::Expr::Block(syn::ExprBlock {
            attrs: Vec::new(),
            label: None,
            block: if_expr.then_branch.clone(),
        });
        let then_diverges = self.is_expr_diverging(&then_block_expr);
        let else_diverges = self.is_expr_diverging(else_expr);

        match (then_ty, else_ty) {
            (Some(lhs), Some(rhs)) => {
                if Self::types_equivalent_by_tokens(&lhs, &rhs) {
                    Some(lhs)
                } else {
                    None
                }
            }
            (Some(lhs), None)
                if expr_is_option_none_constructor(else_expr)
                    && self.is_option_like_syn_type(&lhs) =>
            {
                Some(lhs)
            }
            (None, Some(rhs))
                if expr_is_option_none_constructor(then_expr)
                    && self.is_option_like_syn_type(&rhs) =>
            {
                Some(rhs)
            }
            (Some(lhs), None) if else_diverges => Some(lhs),
            (None, Some(rhs)) if then_diverges => Some(rhs),
            _ => None,
        }
    }

    pub(super) fn infer_constructor_expected_type_from_match(
        &self,
        match_expr: &syn::ExprMatch,
    ) -> Option<syn::Type> {
        let mut left_arg_ty: Option<syn::Type> = None;
        let mut right_arg_ty: Option<syn::Type> = None;
        let mut ok_arg_ty: Option<syn::Type> = None;
        let mut err_arg_ty: Option<syn::Type> = None;
        let mut some_arg_ty: Option<syn::Type> = None;
        let mut saw_none = false;

        for arm in &match_expr.arms {
            if self.is_expr_diverging(&arm.body) {
                continue;
            }
            let body_expr = self.extract_match_arm_value_expr(&arm.body)?;
            let mut arm_env = HashMap::new();
            if let Some(scrutinee_ty) = self
                .infer_simple_expr_type(&match_expr.expr)
                .or_else(|| self.infer_local_binding_type_from_initializer(&match_expr.expr))
            {
                self.bind_pattern_types_into_env(&arm.pat, &scrutinee_ty, &mut arm_env);
            }
            if arm_env.is_empty() {
                self.bind_pattern_literal_types_into_env(&arm.pat, &mut arm_env);
            }
            if expr_is_option_none_constructor(body_expr)
                || self.expr_is_option_none_path(body_expr)
            {
                saw_none = true;
                continue;
            }
            if let Some(some_arg) = self.extract_option_some_call_arg(body_expr) {
                if let Some(arg_ty) = self
                    .infer_expr_type_with_env(some_arg, &arm_env)
                    .or_else(|| self.infer_local_binding_type_from_initializer(some_arg))
                    .or_else(|| self.infer_simple_expr_type(some_arg))
                    .or_else(|| self.infer_hint_type_from_expr(some_arg))
                {
                    some_arg_ty.get_or_insert(arg_ty);
                }
                continue;
            }
            if let Some((ctor_name, ctor_arg)) = self.extract_constructor_call_expr(body_expr) {
                let arg_ty = self
                    .infer_expr_type_with_env(ctor_arg, &arm_env)
                    .or_else(|| self.infer_local_binding_type_from_initializer(ctor_arg))
                    .or_else(|| self.infer_simple_expr_type(ctor_arg))
                    .or_else(|| self.infer_hint_type_from_expr(ctor_arg))?;
                match ctor_name.as_str() {
                    "Left" => {
                        left_arg_ty.get_or_insert(arg_ty);
                    }
                    "Right" => {
                        right_arg_ty.get_or_insert(arg_ty);
                    }
                    "Ok" => {
                        ok_arg_ty.get_or_insert(arg_ty);
                    }
                    "Err" => {
                        err_arg_ty.get_or_insert(arg_ty);
                    }
                    _ => {}
                }
            }
        }

        if let (Some(left), Some(right)) = (left_arg_ty, right_arg_ty) {
            return Some(self.make_either_type(left, right));
        }
        if let (Some(ok_ty), Some(err_ty)) = (ok_arg_ty, err_arg_ty) {
            return Some(self.make_result_type(ok_ty, err_ty));
        }
        if saw_none && let Some(some_ty) = some_arg_ty {
            return Some(parse_quote!(Option<#some_ty>));
        }
        None
    }

    pub(super) fn infer_constructor_expected_type_from_pair(
        &self,
        lhs: &syn::Expr,
        rhs: &syn::Expr,
    ) -> Option<syn::Type> {
        if let (Some((lhs_owner, _)), Some((rhs_owner, _))) = (
            self.data_enum_variant_owner_from_call_expr(lhs),
            self.data_enum_variant_owner_from_call_expr(rhs),
        ) && lhs_owner == rhs_owner
        {
            return syn::parse_str::<syn::Type>(&lhs_owner).ok();
        }

        let (lhs_ctor, lhs_arg) = self.extract_constructor_call_expr(lhs)?;
        let (rhs_ctor, rhs_arg) = self.extract_constructor_call_expr(rhs)?;
        let lhs_ty = self.infer_simple_expr_type(lhs_arg)?;
        let rhs_ty = self.infer_simple_expr_type(rhs_arg)?;

        match (lhs_ctor.as_str(), rhs_ctor.as_str()) {
            ("Left", "Right") => Some(self.make_either_type(lhs_ty, rhs_ty)),
            ("Right", "Left") => Some(self.make_either_type(rhs_ty, lhs_ty)),
            ("Ok", "Err") => Some(self.make_result_type(lhs_ty, rhs_ty)),
            ("Err", "Ok") => Some(self.make_result_type(rhs_ty, lhs_ty)),
            _ => None,
        }
    }

    pub(super) fn infer_simple_expr_type(&self, expr: &syn::Expr) -> Option<syn::Type> {
        let expr = self.extract_value_expr(expr)?;
        let expr_key = expr as *const syn::Expr as usize;
        let _inference_guard =
            ExprTypeInferenceGuard::enter(&self.expr_type_inference_in_progress, expr_key)?;
        match expr {
            syn::Expr::Lit(lit) => self.infer_literal_type(&lit.lit),
            syn::Expr::Path(path) => {
                if path.path.segments.is_empty() {
                    return None;
                }
                if path.path.segments.len() >= 2 {
                    let owner_segments: Vec<String> = path
                        .path
                        .segments
                        .iter()
                        .take(path.path.segments.len().saturating_sub(1))
                        .map(|seg| seg.ident.to_string())
                        .collect();
                    let owner_path = owner_segments.join("::");
                    let owner_tail = owner_segments.last().cloned().unwrap_or_default();
                    let variant = path
                        .path
                        .segments
                        .last()
                        .map(|seg| seg.ident.to_string())
                        .unwrap_or_default();
                    let canonical_variant = self.canonical_variant_name(&variant).to_string();
                    let is_variant_owner = self
                        .path_matches_c_like_enum_const(&owner_path, &variant)
                        || self.path_matches_c_like_enum_const(&owner_tail, &variant)
                        || self.enum_has_variant_name(&owner_path, &canonical_variant)
                        || self.enum_has_variant_name(&owner_tail, &canonical_variant);
                    if is_variant_owner {
                        let owner_only = Self::path_without_last_segment(&path.path)?;
                        if !owner_only.segments.is_empty() {
                            return Some(syn::Type::Path(syn::TypePath {
                                qself: None,
                                path: owner_only,
                            }));
                        }
                    }
                }
                if path.path.segments.len() != 1 {
                    return None;
                }
                let name = path.path.segments[0].ident.to_string();
                if name == "self" {
                    if let Some(struct_name) = &self.current_struct {
                        let scoped = self.scoped_type_key(struct_name);
                        let generic_params = self
                            .declared_type_params
                            .get(struct_name)
                            .or_else(|| self.declared_type_params.get(&scoped))
                            .cloned()
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|p| self.is_type_param_in_scope(p))
                            .collect::<Vec<_>>();
                        if generic_params.is_empty() {
                            if let Ok(ty) = syn::parse_str::<syn::Type>(struct_name) {
                                return Some(ty);
                            }
                        } else {
                            let ty_text = format!("{}<{}>", struct_name, generic_params.join(", "));
                            if let Ok(ty) = syn::parse_str::<syn::Type>(&ty_text) {
                                return Some(ty);
                            }
                        }
                    }
                }
                if let Some((enum_name, _)) = self.flattened_data_enum_variant_parts(&name)
                    && let Ok(enum_ty) = syn::parse_str::<syn::Type>(&enum_name)
                {
                    return Some(enum_ty);
                }
                self.lookup_local_binding_type(&name)
                    .or_else(|| self.lookup_local_placeholder_type_hint(&name).cloned())
                    .or_else(|| self.lookup_item_const_type(&name))
                    .or_else(|| {
                        let looks_like_type = name
                            .chars()
                            .next()
                            .is_some_and(|ch| ch.is_ascii_uppercase());
                        if looks_like_type
                            && (self.local_declared_types.contains(&name)
                                || self.declared_item_names.contains(&name))
                        {
                            Some(syn::Type::Path(syn::TypePath {
                                qself: None,
                                path: path.path.clone(),
                            }))
                        } else {
                            None
                        }
                    })
            }
            syn::Expr::Binary(binary) => match binary.op {
                syn::BinOp::Eq(_)
                | syn::BinOp::Ne(_)
                | syn::BinOp::Lt(_)
                | syn::BinOp::Le(_)
                | syn::BinOp::Gt(_)
                | syn::BinOp::Ge(_)
                | syn::BinOp::And(_)
                | syn::BinOp::Or(_) => syn::parse_str::<syn::Type>("bool").ok(),
                _ => {
                    let lhs = self.infer_simple_expr_type(&binary.left);
                    let rhs = self.infer_simple_expr_type(&binary.right);
                    match (lhs, rhs) {
                        (Some(lhs_ty), Some(rhs_ty)) => {
                            let lhs_peel = self.peel_reference_paren_group_type(&lhs_ty).clone();
                            let rhs_peel = self.peel_reference_paren_group_type(&rhs_ty).clone();
                            if Self::types_equivalent_by_tokens(&lhs_peel, &rhs_peel) {
                                Some(lhs_peel)
                            } else {
                                Some(lhs_peel)
                            }
                        }
                        (Some(lhs_ty), None) => {
                            Some(self.peel_reference_paren_group_type(&lhs_ty).clone())
                        }
                        (None, Some(rhs_ty)) => {
                            Some(self.peel_reference_paren_group_type(&rhs_ty).clone())
                        }
                        (None, None) => None,
                    }
                }
            },
            syn::Expr::Reference(reference) => {
                let inner_ty = self.infer_simple_expr_type(&reference.expr).or_else(|| {
                    let deref_inner = self.peel_paren_group_expr(&reference.expr);
                    if let syn::Expr::Unary(unary) = deref_inner
                        && matches!(unary.op, syn::UnOp::Deref(_))
                        && let Some(ptr_ty) = self.infer_simple_expr_type(&unary.expr)
                        && let Some((pointee_ty, _)) =
                            self.extract_pointer_pointee_info_from_type(&ptr_ty)
                    {
                        return Some(pointee_ty);
                    }
                    None
                })?;
                let quoted = if reference.mutability.is_some() {
                    quote!(&mut #inner_ty)
                } else {
                    quote!(&#inner_ty)
                };
                syn::parse2::<syn::Type>(quoted).ok()
            }
            syn::Expr::RawAddr(raw_addr) => {
                let inner_ty = self.infer_simple_expr_type(&raw_addr.expr).or_else(|| {
                    let deref_inner = self.peel_paren_group_expr(&raw_addr.expr);
                    if let syn::Expr::Unary(unary) = deref_inner
                        && matches!(unary.op, syn::UnOp::Deref(_))
                        && let Some(ptr_ty) = self.infer_simple_expr_type(&unary.expr)
                        && let Some((pointee_ty, _)) =
                            self.extract_pointer_pointee_info_from_type(&ptr_ty)
                    {
                        return Some(pointee_ty);
                    }
                    None
                })?;
                let quoted = match raw_addr.mutability {
                    syn::PointerMutability::Mut(_) => quote!(*mut #inner_ty),
                    syn::PointerMutability::Const(_) => quote!(*const #inner_ty),
                };
                syn::parse2::<syn::Type>(quoted).ok()
            }
            syn::Expr::Unary(unary) => match unary.op {
                syn::UnOp::Neg(_) | syn::UnOp::Not(_) => self.infer_simple_expr_type(&unary.expr),
                syn::UnOp::Deref(_) => {
                    // `*{ stmt; …; TAIL }` — c2rust's `POP!` expands to
                    // `*{ stack.top = stack.top.offset(-1); stack.top }`. The
                    // multi-statement block otherwise short-circuits to None
                    // (extract_value_expr only peels single-expr blocks), so
                    // infer the block's tail expression directly here.
                    let operand = self.peel_paren_group_expr(&unary.expr);
                    let base_ty = if let syn::Expr::Block(block_expr) = operand
                        && block_expr.block.stmts.len() > 1
                        && let Some(tail) = self.extract_tail_expr_from_block(&block_expr.block)
                    {
                        self.infer_simple_expr_type(tail)?
                    } else {
                        self.infer_simple_expr_type(&unary.expr)
                            .or_else(|| {
                                self.infer_local_binding_type_from_initializer(&unary.expr)
                            })?
                    };
                    self.infer_deref_result_type_from_type(&base_ty)
                }
                _ => None,
            },
            // Field access (`(*parser).buffer.pointer`, `parser.buffer`): resolve
            // the base to its owning struct — auto-dereferencing through a raw
            // pointer / reference, as Rust does — then look up the named field's
            // declared type. Recurses for chains. This is what lets a field of
            // raw-pointer type be recognized as such (`is_expr_raw_pointer_like`),
            // so the `rusty::ptr::*` lowering of `.offset()/.wrapping_offset()/…`
            // fires on struct-field receivers — pervasive in c2rust-ported crates
            // (unsafe-libyaml: `(*parser).buffer.pointer.wrapping_offset(n)`).
            syn::Expr::Field(field) => {
                let field_name = match &field.member {
                    syn::Member::Named(ident) => ident.to_string(),
                    syn::Member::Unnamed(index) => index.index.to_string(),
                };
                let base_ty = self.infer_simple_expr_type(&field.base)?;
                let struct_ty = self
                    .extract_pointer_pointee_info_from_type(&base_ty)
                    .map(|(pointee, _)| pointee)
                    .unwrap_or_else(|| self.peel_reference_paren_group_type(&base_ty).clone());
                // Tuple field `t.0`/`t.1`: return the i-th element type. (This
                // arm must preserve the prior tuple-field inference that
                // downstream match-arm / pattern-binding typing relies on —
                // returning None here regressed `match (a.1, b.1) { … }`.)
                if let syn::Type::Tuple(tuple) = &struct_ty
                    && let syn::Member::Unnamed(index) = &field.member
                {
                    return tuple.elems.iter().nth(index.index as usize).cloned();
                }
                let syn::Type::Path(tp) = &struct_ty else {
                    return None;
                };
                let struct_name = tp.path.segments.last()?.ident.to_string();
                // A `Self`-typed base (e.g. a `source: &Self` param in a trait
                // impl) resolves through the impl's concrete self type so its
                // fields can be looked up — `lookup_struct_field_type("Self", …)`
                // would otherwise miss.
                let struct_name = if struct_name == "Self" {
                    self.current_struct.clone().unwrap_or(struct_name)
                } else {
                    struct_name
                };
                let field_ty = self.lookup_struct_field_type(&struct_name, &field_name)?;
                // Substitute the owner's generic args into the field type, so a
                // field declared `*mut T` on `yaml_stack_t<yaml_tag_directive_t>`
                // resolves to `*mut yaml_tag_directive_t` rather than the unbound
                // `*mut T` (pervasive in c2rust's generic stack/queue structs).
                let last_idx = tp.path.segments.len() - 1;
                if let Some(subs) = self.owner_segment_type_arg_substitutions(&tp.path, last_idx)
                    && !subs.is_empty()
                {
                    return Some(self.substitute_type_params_in_type(&field_ty, &subs));
                }
                Some(field_ty)
            }
            syn::Expr::Try(try_expr) => {
                let inner_ty = self
                    .infer_simple_expr_type(&try_expr.expr)
                    .or_else(|| self.infer_local_binding_type_from_initializer(&try_expr.expr))?;
                let inner_ty = self.peel_reference_paren_group_type(&inner_ty);
                let syn::Type::Path(tp) = inner_ty else {
                    return None;
                };
                let last = tp.path.segments.last()?;
                if last.ident != "Result" && last.ident != "Option" {
                    return None;
                }
                let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
                    return None;
                };
                args.args.iter().find_map(|arg| match arg {
                    syn::GenericArgument::Type(ty) => Some(ty.clone()),
                    _ => None,
                })
            }
            syn::Expr::Range(range) => self.infer_range_expr_type(range),
            syn::Expr::Tuple(tuple) => {
                let mut elems = Vec::new();
                for elem in &tuple.elems {
                    elems.push(self.infer_simple_expr_type(elem)?);
                }
                syn::parse2::<syn::Type>(quote!((#(#elems),*))).ok()
            }
            syn::Expr::Array(array_expr) => {
                let first = array_expr.elems.first()?;
                let elem_ty = self.infer_simple_expr_type(first)?;
                let len = syn::LitInt::new(
                    &array_expr.elems.len().to_string(),
                    proc_macro2::Span::call_site(),
                );
                syn::parse2::<syn::Type>(quote!([#elem_ty; #len])).ok()
            }
            syn::Expr::Repeat(repeat_expr) => {
                let elem_ty = self.infer_simple_expr_type(&repeat_expr.expr)?;
                let len_expr = (*repeat_expr.len).clone();
                syn::parse2::<syn::Type>(quote!([#elem_ty; #len_expr])).ok()
            }
            syn::Expr::Struct(struct_expr) => {
                if let Some(last) = struct_expr.path.segments.last() {
                    let last_name = last.ident.to_string();
                    if let Some((enum_name, _)) = self.flattened_data_enum_variant_parts(&last_name)
                    {
                        let mut owner_path = struct_expr.path.clone();
                        if let Some(last_seg) = owner_path.segments.last_mut() {
                            last_seg.ident =
                                syn::Ident::new(&enum_name, proc_macro2::Span::call_site());
                            last_seg.arguments = syn::PathArguments::None;
                        }
                        return Some(syn::Type::Path(syn::TypePath {
                            qself: None,
                            path: owner_path,
                        }));
                    }
                }
                Some(syn::Type::Path(syn::TypePath {
                    qself: None,
                    path: struct_expr.path.clone(),
                }))
            }
            syn::Expr::Call(call) => {
                if let syn::Expr::Path(path) = call.func.as_ref() {
                    let joined = path
                        .path
                        .segments
                        .iter()
                        .map(|seg| seg.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    if matches!(
                        joined.as_str(),
                        "rusty::detail::deref_if_pointer_like" | "rusty::detail::deref_if_pointer"
                    ) && call.args.len() == 1
                    {
                        if let Some(arg_ty) =
                            self.infer_simple_expr_type(&call.args[0]).or_else(|| {
                                self.infer_local_binding_type_from_initializer(&call.args[0])
                            })
                        {
                            let arg_ty = self.peel_reference_paren_group_type(&arg_ty);
                            if let Some((pointee_ty, _)) =
                                self.extract_pointer_pointee_info_from_type(arg_ty)
                            {
                                return Some(pointee_ty);
                            }
                            if let Some(deref_ty) = self.infer_deref_result_type_from_type(arg_ty) {
                                return Some(deref_ty);
                            }
                            return Some(arg_ty.clone());
                        }
                    }
                    if path.path.segments.len() >= 2 {
                        let owner_segments: Vec<String> = path
                            .path
                            .segments
                            .iter()
                            .take(path.path.segments.len().saturating_sub(1))
                            .map(|seg| seg.ident.to_string())
                            .collect();
                        let owner_path = owner_segments.join("::");
                        let owner_tail = owner_segments.last().cloned().unwrap_or_default();
                        let variant = path
                            .path
                            .segments
                            .last()
                            .map(|seg| seg.ident.to_string())
                            .unwrap_or_default();
                        let canonical_variant = self.canonical_variant_name(&variant).to_string();
                        let is_variant_owner = self
                            .path_matches_c_like_enum_const(&owner_path, &variant)
                            || self.path_matches_c_like_enum_const(&owner_tail, &variant)
                            || self.enum_has_variant_name(&owner_path, &canonical_variant)
                            || self.enum_has_variant_name(&owner_tail, &canonical_variant);
                        if is_variant_owner {
                            let owner_only = Self::path_without_last_segment(&path.path)?;
                            if !owner_only.segments.is_empty() {
                                return Some(syn::Type::Path(syn::TypePath {
                                    qself: None,
                                    path: owner_only,
                                }));
                            }
                        }
                    }
                    if path.path.segments.len() == 1 {
                        let name = path.path.segments[0].ident.to_string();
                        if let Some(callee_ty) = self.lookup_local_binding_type(&name) {
                            if call.args.is_empty() {
                                return Some(callee_ty);
                            }
                            if let Some(return_ty) =
                                self.extract_callable_return_type_from_type(&callee_ty)
                            {
                                return Some(return_ty);
                            }
                        }
                    }
                }
                // Last resort: a free-function call returning a RAW POINTER
                // resolves to that pointer type (e.g. `yaml_malloc(...)` ->
                // `*mut c_void`), so a `ptr as usize` cast / raw-pointer method on
                // the result is typed. Restricted to raw-pointer returns to stay
                // additive without perturbing owner-type inference (e.g. Box::new
                // element inference keys off `None` here). Uses the import-aware
                // fallback so cross-module `use`-imported calls also resolve.
                if let Some(ret_ty) =
                    self.lookup_fn_return_type_with_import_fallback(call.func.as_ref())
                    && self.is_type_raw_pointer_like(self.peel_reference_paren_group_type(&ret_ty))
                {
                    return Some(ret_ty);
                }
                self.infer_local_binding_type_from_initializer(expr)
            }
            syn::Expr::Unsafe(unsafe_expr) => self
                .extract_single_expr_from_block(&unsafe_expr.block)
                .and_then(|inner| {
                    self.infer_local_binding_type_from_initializer(inner)
                        .or_else(|| self.infer_simple_expr_type(inner))
                }),
            syn::Expr::Block(block_expr) => self
                .extract_single_expr_from_block(&block_expr.block)
                .and_then(|inner| {
                    self.infer_local_binding_type_from_initializer(inner)
                        .or_else(|| self.infer_simple_expr_type(inner))
                }),
            syn::Expr::Match(match_expr) => self.infer_match_expr_common_arm_type(match_expr),
            syn::Expr::MethodCall(mc) => self.infer_method_call_result_type_for_local(mc),
            syn::Expr::Field(field) => match &field.member {
                syn::Member::Named(ident) => {
                    self.lookup_field_type_for_expr_base(&field.base, &ident.to_string())
                }
                syn::Member::Unnamed(index) => {
                    let base_ty = self.infer_simple_expr_type(&field.base)?;
                    self.resolve_tuple_field_type_from_type(&base_ty, index.index as usize)
                }
            },
            syn::Expr::Index(index_expr) => {
                let base_ty = self.infer_simple_expr_type(&index_expr.expr)?;
                let base_ty = self.peel_reference_paren_group_type(&base_ty);
                match base_ty {
                    syn::Type::Array(array_ty) => Some((*array_ty.elem).clone()),
                    syn::Type::Slice(slice_ty) => Some((*slice_ty.elem).clone()),
                    syn::Type::Path(tp) => {
                        let last = tp.path.segments.last()?;
                        let owner = last.ident.to_string();
                        if !matches!(owner.as_str(), "Vec" | "VecDeque" | "ArrayVec" | "SmallVec") {
                            return None;
                        }
                        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
                            return None;
                        };
                        args.args.iter().find_map(|arg| match arg {
                            syn::GenericArgument::Type(ty) => Some(ty.clone()),
                            _ => None,
                        })
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub(super) fn resolve_tuple_field_type_from_type(
        &self,
        ty: &syn::Type,
        index: usize,
    ) -> Option<syn::Type> {
        self.resolve_tuple_type_from_type(ty)
            .and_then(|tuple_ty| tuple_ty.elems.iter().nth(index).cloned())
    }

    pub(super) fn resolve_tuple_type_from_type(&self, ty: &syn::Type) -> Option<syn::TypeTuple> {
        let ty = self.peel_reference_paren_group_type(ty);
        match ty {
            syn::Type::Tuple(tuple_ty) => Some(tuple_ty.clone()),
            syn::Type::Path(tp) => {
                let joined = tp
                    .path
                    .segments
                    .iter()
                    .map(|seg| seg.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                let elems = if let Some(elems) = self.tuple_type_alias_elem_types.get(&joined) {
                    Some(elems.clone())
                } else {
                    let last = tp.path.segments.last()?.ident.to_string();
                    self.tuple_type_alias_elem_types.get(&last).cloned()
                }?;
                let mut punctuated = syn::punctuated::Punctuated::new();
                for elem in elems {
                    punctuated.push(elem);
                }
                Some(syn::TypeTuple {
                    paren_token: syn::token::Paren::default(),
                    elems: punctuated,
                })
            }
            _ => None,
        }
    }

    pub(super) fn infer_range_expr_type(&self, range: &syn::ExprRange) -> Option<syn::Type> {
        let elem_ty = range
            .start
            .as_deref()
            .and_then(|e| self.infer_simple_expr_type(e))
            .or_else(|| {
                range
                    .end
                    .as_deref()
                    .and_then(|e| self.infer_simple_expr_type(e))
            });

        match (range.start.as_ref(), range.end.as_ref(), &range.limits) {
            (Some(_), Some(_), syn::RangeLimits::HalfOpen(_)) => {
                let t = elem_ty?;
                Some(parse_quote!(rusty::range<#t>))
            }
            (Some(_), Some(_), syn::RangeLimits::Closed(_)) => {
                let t = elem_ty?;
                Some(parse_quote!(rusty::range_inclusive<#t>))
            }
            (Some(_), None, _) => {
                let t = elem_ty?;
                Some(parse_quote!(rusty::range_from<#t>))
            }
            (None, Some(_), syn::RangeLimits::HalfOpen(_)) => {
                let t = elem_ty?;
                Some(parse_quote!(rusty::range_to<#t>))
            }
            (None, Some(_), syn::RangeLimits::Closed(_)) => {
                let t = elem_ty?;
                Some(parse_quote!(rusty::range_to_inclusive<#t>))
            }
            (None, None, _) => Some(parse_quote!(rusty::range_full)),
        }
    }

    pub(super) fn infer_literal_type(&self, lit: &syn::Lit) -> Option<syn::Type> {
        match lit {
            syn::Lit::Int(int_lit) => {
                let ty_name = if int_lit.suffix().is_empty() {
                    "i32"
                } else {
                    int_lit.suffix()
                };
                syn::parse_str::<syn::Type>(ty_name).ok()
            }
            syn::Lit::Float(float_lit) => {
                let ty_name = if float_lit.suffix().is_empty() {
                    "f64"
                } else {
                    float_lit.suffix()
                };
                syn::parse_str::<syn::Type>(ty_name).ok()
            }
            syn::Lit::Bool(_) => syn::parse_str::<syn::Type>("bool").ok(),
            syn::Lit::Char(_) => syn::parse_str::<syn::Type>("char").ok(),
            syn::Lit::Str(_) => syn::parse_str::<syn::Type>("&str").ok(),
            syn::Lit::Byte(_) => syn::parse_str::<syn::Type>("u8").ok(),
            _ => None,
        }
    }

    pub(super) fn recover_constructor_template_hints_from_expr(
        &self,
        expr: &syn::Expr,
    ) -> HashMap<String, Vec<String>> {
        let mut hints = HashMap::new();
        let Some(expr) = self.extract_value_expr(expr) else {
            return hints;
        };
        let if_let_unwrap_method = self.expr_if_let_unwrap_method(expr);
        let try_result_cpp_ty = self.infer_try_result_cpp_type_from_expr(expr);

        if let Some((ctor_name, ctor_arg)) = self.extract_constructor_call_expr(expr) {
            let arg_cpp = self.emit_constructor_hint_arg_cpp(ctor_arg, if_let_unwrap_method);
            let ty_cpp = format!("decltype(({}))", arg_cpp);
            match ctor_name.as_str() {
                "Left" | "Right" => {
                    let args = vec![ty_cpp.clone(), ty_cpp];
                    hints.insert("Left".to_string(), args.clone());
                    hints.insert("Right".to_string(), args);
                }
                "Ok" | "Err" => {
                    let args = vec![ty_cpp.clone(), ty_cpp];
                    hints.insert("Ok".to_string(), args.clone());
                    hints.insert("Err".to_string(), args);
                }
                _ => {}
            }
            return hints;
        }

        if let syn::Expr::If(if_expr) = expr {
            if let Some((left_name, left_arg, right_name, right_arg)) =
                self.extract_constructor_pair_from_if(if_expr)
            {
                self.insert_constructor_pair_hints(
                    &mut hints,
                    &left_name,
                    left_arg,
                    &right_name,
                    right_arg,
                    if_let_unwrap_method,
                );
            }
        }

        if hints.is_empty() {
            if let syn::Expr::Match(match_expr) = expr {
                if let Some((left_name, left_arg, right_name, right_arg)) =
                    self.extract_constructor_pair_from_match(match_expr)
                {
                    self.insert_constructor_pair_hints(
                        &mut hints,
                        &left_name,
                        left_arg,
                        &right_name,
                        right_arg,
                        if_let_unwrap_method,
                    );
                }
            }
        }

        if hints.is_empty() {
            let mut ctor_args: HashMap<String, String> = HashMap::new();
            self.collect_constructor_arg_cpp_strings(expr, &mut ctor_args, if_let_unwrap_method);

            let mut inferred_either_ty: Option<String> = None;
            if let (Some(left_cpp), Some(right_cpp)) =
                (ctor_args.get("Left"), ctor_args.get("Right"))
            {
                let args = vec![
                    format!("decltype(({}))", left_cpp),
                    format!("decltype(({}))", right_cpp),
                ];
                hints.insert("Left".to_string(), args.clone());
                hints.insert("Right".to_string(), args);
                inferred_either_ty = Some(format!(
                    "Either<decltype(({})), decltype(({}))>",
                    left_cpp, right_cpp
                ));
            }

            if let (Some(ok_cpp), Some(err_cpp)) = (ctor_args.get("Ok"), ctor_args.get("Err")) {
                let err_ty =
                    inferred_either_ty.unwrap_or_else(|| format!("decltype(({}))", err_cpp));
                let args = vec![format!("decltype(({}))", ok_cpp), err_ty];
                hints.insert("Ok".to_string(), args.clone());
                hints.insert("Err".to_string(), args);
            }

            if let Some(result_cpp_ty) = try_result_cpp_ty.as_ref() {
                if !hints.contains_key("Ok") {
                    if let Some(ok_cpp) = ctor_args.get("Ok") {
                        let ok_ty = format!("decltype(({}))", ok_cpp);
                        let err_ty = format!(
                            "std::remove_cvref_t<decltype(std::declval<{}>().unwrap_err())>",
                            result_cpp_ty
                        );
                        let args = vec![ok_ty, err_ty];
                        hints.insert("Ok".to_string(), args.clone());
                        hints.insert("Err".to_string(), args);
                    }
                }
                if !hints.contains_key("Err") {
                    if let Some(err_cpp) = ctor_args.get("Err") {
                        let ok_ty = format!(
                            "std::remove_cvref_t<decltype(std::declval<{}>().unwrap())>",
                            result_cpp_ty
                        );
                        let err_ty = format!("decltype(({}))", err_cpp);
                        let args = vec![ok_ty, err_ty];
                        hints.insert("Ok".to_string(), args.clone());
                        hints.insert("Err".to_string(), args);
                    }
                }
            }
        }

        hints
    }

    pub(super) fn infer_try_result_cpp_type_from_stmt(&self, stmt: &syn::Stmt) -> Option<String> {
        match stmt {
            syn::Stmt::Local(local) => {
                let init = local.init.as_ref()?;
                self.infer_try_result_cpp_type_from_expr(&init.expr)
            }
            syn::Stmt::Expr(expr, _) => self.infer_try_result_cpp_type_from_expr(expr),
            syn::Stmt::Item(_) | syn::Stmt::Macro(_) => None,
        }
    }

    pub(super) fn infer_try_result_cpp_type_from_expr(&self, expr: &syn::Expr) -> Option<String> {
        let expr = self.peel_paren_group_expr(expr);
        match expr {
            syn::Expr::Try(try_expr) => {
                let inner = self.emit_expr_to_string(&try_expr.expr);
                Some(format!("std::remove_cvref_t<decltype(({}))>", inner))
            }
            syn::Expr::Block(block_expr) => block_expr
                .block
                .stmts
                .iter()
                .find_map(|stmt| self.infer_try_result_cpp_type_from_stmt(stmt)),
            syn::Expr::If(if_expr) => self
                .infer_try_result_cpp_type_from_expr(&if_expr.cond)
                .or_else(|| {
                    if_expr
                        .then_branch
                        .stmts
                        .iter()
                        .find_map(|stmt| self.infer_try_result_cpp_type_from_stmt(stmt))
                })
                .or_else(|| {
                    if_expr
                        .else_branch
                        .as_ref()
                        .and_then(|(_, expr)| self.infer_try_result_cpp_type_from_expr(expr))
                }),
            syn::Expr::Match(match_expr) => self
                .infer_try_result_cpp_type_from_expr(&match_expr.expr)
                .or_else(|| {
                    match_expr
                        .arms
                        .iter()
                        .find_map(|arm| self.infer_try_result_cpp_type_from_expr(&arm.body))
                }),
            syn::Expr::Call(call) => self
                .infer_try_result_cpp_type_from_expr(&call.func)
                .or_else(|| {
                    call.args
                        .iter()
                        .find_map(|arg| self.infer_try_result_cpp_type_from_expr(arg))
                }),
            syn::Expr::MethodCall(mc) => self
                .infer_try_result_cpp_type_from_expr(&mc.receiver)
                .or_else(|| {
                    mc.args
                        .iter()
                        .find_map(|arg| self.infer_try_result_cpp_type_from_expr(arg))
                }),
            syn::Expr::Closure(closure) => self.infer_try_result_cpp_type_from_expr(&closure.body),
            syn::Expr::Reference(r) => self.infer_try_result_cpp_type_from_expr(&r.expr),
            syn::Expr::Array(arr) => arr
                .elems
                .iter()
                .find_map(|elem| self.infer_try_result_cpp_type_from_expr(elem)),
            syn::Expr::Tuple(tuple) => tuple
                .elems
                .iter()
                .find_map(|elem| self.infer_try_result_cpp_type_from_expr(elem)),
            syn::Expr::Struct(struct_expr) => struct_expr
                .fields
                .iter()
                .find_map(|field| self.infer_try_result_cpp_type_from_expr(&field.expr))
                .or_else(|| {
                    struct_expr
                        .rest
                        .as_ref()
                        .and_then(|rest| self.infer_try_result_cpp_type_from_expr(rest))
                }),
            syn::Expr::Assign(assign) => self
                .infer_try_result_cpp_type_from_expr(&assign.left)
                .or_else(|| self.infer_try_result_cpp_type_from_expr(&assign.right)),
            syn::Expr::Return(ret) => ret
                .expr
                .as_ref()
                .and_then(|expr| self.infer_try_result_cpp_type_from_expr(expr)),
            syn::Expr::Paren(p) => self.infer_try_result_cpp_type_from_expr(&p.expr),
            syn::Expr::Group(g) => self.infer_try_result_cpp_type_from_expr(&g.expr),
            _ => None,
        }
    }

    pub(super) fn infer_tuple_result_type_for_if_expr(&self, if_expr: &syn::ExprIf) -> Option<syn::Type> {
        let env = HashMap::new();
        let elem_types = self.infer_tuple_result_elem_types_for_if_expr(if_expr, &env)?;
        let mut concrete = Vec::with_capacity(elem_types.len());
        for elem in elem_types {
            match elem {
                IfTupleElemType::Concrete(ty) => concrete.push(ty),
                IfTupleElemType::NonePath | IfTupleElemType::Unknown => return None,
            }
        }
        Some(parse_quote!((#(#concrete),*)))
    }

    pub(super) fn infer_tuple_result_elem_expected_types_for_if_expr(
        &self,
        if_expr: &syn::ExprIf,
    ) -> Option<Vec<Option<syn::Type>>> {
        let env = HashMap::new();
        let elem_types = self.infer_tuple_result_elem_types_for_if_expr(if_expr, &env)?;
        Some(
            elem_types
                .into_iter()
                .map(|elem| match elem {
                    IfTupleElemType::Concrete(ty) => Some(ty),
                    IfTupleElemType::NonePath | IfTupleElemType::Unknown => None,
                })
                .collect(),
        )
    }

    pub(super) fn infer_tuple_result_elem_types_for_if_expr(
        &self,
        if_expr: &syn::ExprIf,
        env: &HashMap<String, syn::Type>,
    ) -> Option<Vec<IfTupleElemType>> {
        let then_tail = self.extract_tail_expr_from_block(&if_expr.then_branch);
        let then_diverges = match then_tail {
            Some(tail) => self.expr_definitely_diverges_for_if_tuple_inference(tail),
            None => self.block_definitely_diverges_for_if_tuple_inference(&if_expr.then_branch),
        };
        let mut then_env = env.clone();
        if let syn::Expr::Let(let_expr) = if_expr.cond.as_ref() {
            self.populate_inferred_env_from_if_let_condition(let_expr, &mut then_env);
        }
        let then_types = then_tail
            .and_then(|tail| self.infer_tuple_result_elem_types_for_expr_with_env(tail, &then_env));
        let (_, else_expr) = if_expr.else_branch.as_ref()?;
        let else_types = self.infer_tuple_result_elem_types_for_expr_with_env(else_expr, env);
        let else_diverges = self.expr_definitely_diverges_for_if_tuple_inference(else_expr);
        match (then_types, else_types) {
            (Some(then_types), Some(else_types)) => {
                self.merge_if_tuple_elem_types(&then_types, &else_types)
            }
            (Some(then_types), None) if else_diverges => Some(then_types),
            (None, Some(else_types)) if then_diverges => Some(else_types),
            _ => None,
        }
    }

    pub(super) fn infer_tuple_result_elem_types_for_expr_with_env(
        &self,
        expr: &syn::Expr,
        env: &HashMap<String, syn::Type>,
    ) -> Option<Vec<IfTupleElemType>> {
        let expr = self.peel_paren_group_expr(expr);
        match expr {
            syn::Expr::Block(block_expr) => {
                self.infer_tuple_result_elem_types_from_block_with_env(&block_expr.block, env)
            }
            syn::Expr::If(if_expr) => self.infer_tuple_result_elem_types_for_if_expr(if_expr, env),
            syn::Expr::Tuple(tuple) => tuple
                .elems
                .iter()
                .map(|elem| self.infer_tuple_elem_type_with_env(elem, env))
                .collect(),
            _ => None,
        }
    }

    pub(super) fn infer_tuple_result_elem_types_from_block_with_env(
        &self,
        block: &syn::Block,
        env: &HashMap<String, syn::Type>,
    ) -> Option<Vec<IfTupleElemType>> {
        let mut local_env = env.clone();
        let tail_expr = self.extract_tail_expr_from_block(block)?;
        let stmt_count = block.stmts.len().saturating_sub(1);
        for stmt in block.stmts.iter().take(stmt_count) {
            self.update_inferred_local_env_from_stmt(stmt, &mut local_env);
        }
        self.infer_tuple_result_elem_types_for_expr_with_env(tail_expr, &local_env)
    }

    pub(super) fn infer_try_payload_type_from_expr(&self, expr: &syn::Expr) -> Option<syn::Type> {
        let expr = self.peel_paren_group_expr(expr);
        let syn::Expr::Try(try_expr) = expr else {
            return None;
        };
        let inner = self.peel_paren_group_expr(&try_expr.expr);
        match inner {
            syn::Expr::Call(call) => {
                let ret_ty = self.lookup_function_return_type(call.func.as_ref())?;
                self.try_unwrap_try_operand_type(ret_ty)
            }
            syn::Expr::MethodCall(method_call) => {
                let ret_ty = self.infer_method_call_result_type_for_local(method_call)?;
                self.try_unwrap_try_operand_type(&ret_ty)
            }
            _ => None,
        }
    }

    pub(super) fn infer_pattern_literal_type(&self, pat: &syn::Pat) -> Option<syn::Type> {
        match pat {
            syn::Pat::Lit(lit) => self.infer_literal_type(&lit.lit),
            syn::Pat::Range(range) => range
                .start
                .as_deref()
                .and_then(|start| self.infer_simple_expr_type(start))
                .or_else(|| {
                    range
                        .end
                        .as_deref()
                        .and_then(|end| self.infer_simple_expr_type(end))
                }),
            syn::Pat::Or(or_pat) => {
                let mut common: Option<syn::Type> = None;
                for case in &or_pat.cases {
                    let case_ty = self.infer_pattern_literal_type(case)?;
                    if let Some(existing) = &common {
                        if !Self::types_equivalent_by_tokens(existing, &case_ty) {
                            return None;
                        }
                    } else {
                        common = Some(case_ty);
                    }
                }
                common
            }
            syn::Pat::Ident(pi) => pi
                .subpat
                .as_ref()
                .and_then(|(_, subpat)| self.infer_pattern_literal_type(subpat)),
            syn::Pat::Paren(paren) => self.infer_pattern_literal_type(&paren.pat),
            syn::Pat::Reference(reference) => self.infer_pattern_literal_type(&reference.pat),
            _ => None,
        }
    }

    pub(super) fn infer_tuple_elem_type_with_env(
        &self,
        expr: &syn::Expr,
        env: &HashMap<String, syn::Type>,
    ) -> Option<IfTupleElemType> {
        if self.expr_is_option_none_path(expr) {
            return Some(IfTupleElemType::NonePath);
        }
        if let Some(some_arg) = self.extract_option_some_call_arg(expr) {
            if let Some(arg_ty) = self.infer_expr_type_with_env(some_arg, env) {
                return Some(IfTupleElemType::Concrete(parse_quote!(Option<#arg_ty>)));
            }
            return Some(IfTupleElemType::Unknown);
        }
        Some(
            self.infer_expr_type_with_env(expr, env)
                .map(IfTupleElemType::Concrete)
                .unwrap_or(IfTupleElemType::Unknown),
        )
    }

    pub(super) fn infer_expr_type_with_env(
        &self,
        expr: &syn::Expr,
        env: &HashMap<String, syn::Type>,
    ) -> Option<syn::Type> {
        let expr = self.extract_value_expr(expr)?;
        match expr {
            syn::Expr::Path(path) => {
                if path.path.segments.len() == 1 {
                    let name = path.path.segments[0].ident.to_string();
                    if let Some(ty) = env.get(&name) {
                        return Some(ty.clone());
                    }
                }
            }
            syn::Expr::Tuple(tuple) => {
                let mut elem_types = Vec::with_capacity(tuple.elems.len());
                for elem in &tuple.elems {
                    let elem_ty = self
                        .infer_expr_type_with_env(elem, env)
                        .or_else(|| self.infer_local_binding_type_from_initializer(elem))
                        .or_else(|| self.infer_simple_expr_type(elem))?;
                    elem_types.push(elem_ty);
                }
                return Some(parse_quote!((#(#elem_types),*)));
            }
            syn::Expr::Match(_) => {
                if !env.is_empty() {
                    let mut inner = self.new_inner_for_block();
                    inner.local_bindings.push(
                        env.iter()
                            .map(|(name, ty)| (name.clone(), Some(ty.clone())))
                            .collect(),
                    );
                    if let Some(inferred) = inner
                        .infer_local_binding_type_from_initializer(expr)
                        .or_else(|| inner.infer_simple_expr_type(expr))
                    {
                        return Some(inferred);
                    }
                }
            }
            _ => {}
        }
        self.infer_simple_expr_type(expr)
    }

    pub(super) fn infer_expected_type_from_tuple_elements(
        &self,
        elems: &syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>,
    ) -> Option<syn::Type> {
        let mut common: Option<syn::Type> = None;
        for elem in elems {
            let Some(elem_ty) = self.infer_expected_type_from_tuple_element(elem) else {
                continue;
            };
            if let Some(existing) = &common {
                if !Self::types_equivalent_by_tokens(existing, &elem_ty) {
                    return None;
                }
            } else {
                common = Some(elem_ty);
            }
        }
        common
    }

    pub(super) fn infer_expected_type_from_tuple_element(&self, elem: &syn::Expr) -> Option<syn::Type> {
        match elem {
            syn::Expr::Reference(r) => self.infer_expected_type_from_tuple_element(&r.expr),
            syn::Expr::Paren(p) => self.infer_expected_type_from_tuple_element(&p.expr),
            syn::Expr::Group(g) => self.infer_expected_type_from_tuple_element(&g.expr),
            syn::Expr::Lit(_) => self.infer_simple_expr_type(elem),
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                let name = path.path.segments[0].ident.to_string();
                self.lookup_local_binding_type(&name)
            }
            syn::Expr::Call(call) => {
                if call.args.is_empty() {
                    if let syn::Expr::Path(path) = call.func.as_ref() {
                        if path.path.segments.len() == 1 {
                            let name = path.path.segments[0].ident.to_string();
                            return self.lookup_local_binding_type(&name);
                        }
                    }
                }
                self.infer_local_binding_type_from_initializer(elem)
            }
            _ => None,
        }
    }

    pub(super) fn resolve_alias_impl_free_function_for_receiver_expr(
        &self,
        receiver_expr: &syn::Expr,
        method_name: &str,
        method_template_args: Option<&str>,
    ) -> Option<String> {
        let owner_match = self
            .infer_simple_expr_type(receiver_expr)
            .and_then(|receiver_ty| {
                let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
                if let syn::Type::Path(tp) = receiver_ty {
                    let owner_name = tp
                        .path
                        .segments
                        .last()
                        .map(|seg| seg.ident.to_string())
                        .unwrap_or_default();
                    self.resolve_alias_owner_key_with_method_from_owner_path(
                        Some(&tp.path),
                        &owner_name,
                        method_name,
                    )
                    .or_else(|| {
                        self.resolve_alias_owner_key_with_method_from_receiver_target_type(
                            receiver_ty,
                            method_name,
                        )
                    })
                } else {
                    self.resolve_alias_owner_key_with_method_from_receiver_target_type(
                        receiver_ty,
                        method_name,
                    )
                }
            })
            .or_else(|| self.resolve_alias_owner_key_with_receiver_method_name(method_name));
        let (owner_key, receiver_shape) = owner_match?;
        if matches!(receiver_shape, Some(false)) {
            return None;
        }
        Some(self.alias_impl_helper_function_path(&owner_key, method_name, method_template_args))
    }

    pub(super) fn infer_slice_arg_expected_type_from_receiver(
        &self,
        receiver: &syn::Expr,
        method_name: &str,
        arg_idx: usize,
    ) -> Option<syn::Type> {
        let uses_receiver_elem_slice = matches!(
            method_name,
            "try_extend_from_slice" | "extend_from_slice" | "copy_from_slice" | "clone_from_slice"
        );
        let is_write_method = matches!(method_name, "write" | "write_all");
        if !uses_receiver_elem_slice && !is_write_method {
            return None;
        }
        if uses_receiver_elem_slice {
            return self.infer_receiver_item_slice_expected_type(receiver, method_name, arg_idx);
        }
        if arg_idx != 0 {
            return None;
        }
        let receiver_ty = self.infer_simple_expr_type(receiver)?;
        let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
        let syn::Type::Path(tp) = receiver_ty else {
            return None;
        };
        let last = tp.path.segments.last()?;
        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
            return None;
        };
        let elem_ty = args.args.iter().find_map(|arg| match arg {
            syn::GenericArgument::Type(t) => Some(t.clone()),
            _ => None,
        })?;
        // Guardrail: only infer element-type write slices from receiver generics for
        // byte-write contexts to avoid blanket rewrites across unrelated `write` APIs.
        if is_write_method && !Self::is_u8_syn_type(&elem_ty) {
            return None;
        }
        Some(parse_quote!(&[#elem_ty]))
    }

    pub(super) fn infer_receiver_item_slice_expected_type(
        &self,
        receiver: &syn::Expr,
        method_name: &str,
        arg_idx: usize,
    ) -> Option<syn::Type> {
        let owner_arg_idx = match method_name {
            "insert_from_slice" if arg_idx == 1 => 0usize,
            "try_extend_from_slice"
            | "extend_from_slice"
            | "copy_from_slice"
            | "clone_from_slice"
                if arg_idx == 0 =>
            {
                0usize
            }
            _ => return None,
        };
        let receiver_ty = self.infer_simple_expr_type(receiver)?;
        let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
        let syn::Type::Path(tp) = receiver_ty else {
            return None;
        };
        let last = tp.path.segments.last()?;
        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
            return None;
        };
        let owner_ty = args
            .args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(t) => Some(t.clone()),
                _ => None,
            })
            .nth(owner_arg_idx)?;
        let item_ty = self
            .extract_iter_item_type_from_type(&owner_ty)
            .unwrap_or(owner_ty);
        Some(parse_quote!(&[#item_ty]))
    }

    pub(super) fn infer_method_call_receiver_owner_name(&self, receiver: &syn::Expr) -> Option<String> {
        if let Some((owner, _)) = self.receiver_owner_name_and_type_substitutions(receiver)
            && owner != "Self"
            && !owner.is_empty()
        {
            return Some(owner);
        }

        let receiver = self.peel_paren_group_expr(receiver);
        let syn::Expr::Call(call) = receiver else {
            return None;
        };
        if !call.args.is_empty() {
            return None;
        }
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        if path_expr.path.segments.len() < 2 {
            return None;
        }
        let method = path_expr.path.segments.last()?.ident.to_string();
        if !matches!(method.as_str(), "new" | "new_" | "default" | "default_") {
            return None;
        }
        let owner = path_expr
            .path
            .segments
            .iter()
            .take(path_expr.path.segments.len().saturating_sub(1))
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<String>>()
            .join("::");
        if owner.is_empty() { None } else { Some(owner) }
    }

    pub(super) fn infer_method_arg_expected_type_from_receiver(
        &self,
        receiver: &syn::Expr,
        method_name: &str,
        arg_idx: usize,
        declared_expected: Option<&syn::Type>,
        arg_expr: Option<&syn::Expr>,
    ) -> Option<syn::Type> {
        let force_receiver_closure_infer =
            arg_idx == 0 && matches!(method_name, "init" | "get_or_init" | "get_or_try_init");
        let declared_expected_assoc_like = declared_expected.is_some_and(|declared| {
            self.type_contains_dependent_assoc(declared)
                || self.type_references_current_struct_assoc(declared)
                || self.type_looks_like_assoc_projection(declared)
        });
        let allow_infer = match declared_expected {
            None => true,
            Some(declared_expected) => {
                matches!(
                    declared_expected,
                    syn::Type::Path(tp)
                        if tp.path.segments.len() == 1
                            && (self.is_type_param_in_scope(&tp.path.segments[0].ident.to_string())
                                || tp.path.segments[0]
                                    .ident
                                    .to_string()
                                    .chars()
                                    .next()
                                    .is_some_and(|c| c.is_ascii_uppercase()))
                ) || declared_expected_assoc_like
            }
        };
        if !allow_infer && !force_receiver_closure_infer {
            return None;
        }

        if let Some(slice_ty) =
            self.infer_receiver_item_slice_expected_type(receiver, method_name, arg_idx)
        {
            return Some(slice_ty);
        }

        // Option closure factories (`get_or_insert_with` / `unwrap_or_else`)
        // expect a zero-arg callable returning the option payload type.
        if arg_idx == 0 && matches!(method_name, "get_or_insert_with" | "unwrap_or_else") {
            let receiver_ty = self.infer_simple_expr_type(receiver)?;
            let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
            let syn::Type::Path(tp) = receiver_ty else {
                return None;
            };
            let last = tp.path.segments.last()?;
            if last.ident != "Option" {
                return None;
            }
            let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
                return None;
            };
            let inner_ty = args.args.iter().find_map(|arg| match arg {
                syn::GenericArgument::Type(t) => Some(t.clone()),
                _ => None,
            })?;
            let callable_ty: syn::Type = parse_quote!(impl FnOnce() -> #inner_ty);
            return Some(callable_ty);
        }

        // Infer closure return type for get_or_try_init/get_or_init/init from receiver.
        // e.g., `cell.get_or_try_init(|| Err(()))` on `cell: OnceCell<String>`
        // → infer closure return type = Result<String, _> so Err() can resolve
        // its error slot from local constructor context.
        let is_get_or_init = method_name == "get_or_init";
        let is_get_or_try_init = method_name == "get_or_try_init";
        let is_init = method_name == "init";
        if (is_get_or_init || is_get_or_try_init || is_init) && arg_idx == 0 {
            let receiver_ty = self.infer_simple_expr_type(receiver)?;
            let receiver_ty_peel = self.peel_reference_paren_group_type(&receiver_ty);

            // Prefer owner-shape inference for containers where closure output differs
            // from method return payload (e.g. OnceBox::get_or_try_init).
            if let syn::Type::Path(tp) = receiver_ty_peel
                && let Some(last) = tp.path.segments.last()
            {
                let receiver_owner_name = last.ident.to_string();
                if matches!(
                    receiver_owner_name.as_str(),
                    "OnceCell" | "OnceBox" | "Lazy" | "NonNull" | "OnceRef"
                ) && let syn::PathArguments::AngleBracketed(args) = &last.arguments
                    && let Some(inner_ty) = args.args.iter().find_map(|arg| match arg {
                        syn::GenericArgument::Type(t) => Some(t.clone()),
                        _ => None,
                    })
                {
                    // For OnceBox<T>, initializer closures return Box<T>.
                    // For OnceRef<T>, initializer closures return &T.
                    // For OnceCell/Lazy/NonNull, closures return T.
                    let init_value_ty: syn::Type = if receiver_owner_name == "OnceBox" {
                        parse_quote!(Box<#inner_ty>)
                    } else if receiver_owner_name == "OnceRef" {
                        parse_quote!(&#inner_ty)
                    } else {
                        inner_ty
                    };
                    if is_get_or_try_init {
                        let error_ty = self.inferred_try_init_error_type_or_unit(arg_expr);
                        let result_ty: syn::Type = parse_quote!(Result<#init_value_ty, #error_ty>);
                        return Some(result_ty);
                    }
                    return Some(init_value_ty);
                }
            }

            // Generic fallback: derive initializer payload from the receiver method
            // return shape. This covers non-generic owners like OnceNonZeroUsize.
            let ret_method = if is_init { "get_or_init" } else { method_name };
            let mut ret_ty =
                self.lookup_owner_method_return_type_from_receiver_type(&receiver_ty, ret_method)?;
            if let Some((_, substitutions)) =
                self.receiver_owner_name_and_type_substitutions(receiver)
                && !substitutions.is_empty()
            {
                ret_ty = self.substitute_type_params_in_type(&ret_ty, &substitutions);
            }
            if is_get_or_try_init {
                let init_value_ty = self.expected_result_type_arg(Some(&ret_ty), 0)?.clone();
                let error_ty = self.inferred_try_init_error_type_or_unit(arg_expr);
                let result_ty: syn::Type = parse_quote!(Result<#init_value_ty, #error_ty>);
                return Some(result_ty);
            }
            return Some(ret_ty);
        }

        let elem_arg_idx = match method_name {
            "push" | "try_push" if arg_idx == 0 => 0usize,
            "insert" | "try_insert" if arg_idx == 1 => 0usize,
            "set" if arg_idx == 0 => 0usize,
            _ => return None,
        };

        let receiver_ty = self.infer_simple_expr_type(receiver)?;
        let syn::Type::Path(tp) = receiver_ty else {
            return None;
        };
        let last = tp.path.segments.last()?;
        let receiver_owner_name = last.ident.to_string();
        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
            return None;
        };
        let inferred = args
            .args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(t) => Some(t.clone()),
                _ => None,
            })
            .nth(elem_arg_idx)?;

        if method_name == "set" && receiver_owner_name == "OnceBox" {
            let boxed_ty: syn::Type = parse_quote!(Box<#inferred>);
            return Some(boxed_ty);
        }
        if method_name == "set" && receiver_owner_name == "OnceRef" {
            let ref_ty: syn::Type = parse_quote!(&#inferred);
            return Some(ref_ty);
        }

        // For dependent associated-item method signatures (for example
        // `fn push(&mut self, value: A::Item)`), prefer the receiver's concrete
        // item type instead of the whole owner argument (`A` or `[T; N]`).
        if declared_expected_assoc_like
            && let Some(item_ty) = self.extract_iter_item_type_from_type(&inferred)
        {
            return Some(item_ty);
        }

        // SmallVec stores an array-like owner parameter (`[T; N]` / `A: Array`)
        // while element-taking methods expect the item type (`T` / `A::Item`).
        // When method-signature expected type metadata is ambiguous for shared
        // method names (for example `push`), recover item type from receiver owner.
        if matches!(receiver_owner_name.as_str(), "SmallVec")
            && matches!(method_name, "push" | "try_push" | "insert" | "try_insert")
            && let Some(item_ty) = self.extract_iter_item_type_from_type(&inferred)
        {
            return Some(item_ty);
        }

        Some(inferred)
    }

    pub(super) fn infer_try_init_error_type_from_arg_expr(
        &self,
        arg_expr: Option<&syn::Expr>,
    ) -> Option<syn::Type> {
        let expr = self.peel_paren_group_expr(arg_expr?);
        let infer_from_ty = |ty: &syn::Type| -> Option<syn::Type> {
            let err_ty = self.expected_result_type_arg(Some(ty), 1)?;
            if self.type_is_concrete_hint_candidate(err_ty) {
                Some(err_ty.clone())
            } else {
                None
            }
        };

        match expr {
            syn::Expr::Closure(closure) => {
                if let syn::ReturnType::Type(_, ty) = &closure.output
                    && let Some(from_output) = infer_from_ty(ty)
                {
                    return Some(from_output);
                }
                self.infer_try_init_error_type_from_expr(&closure.body)
            }
            _ => self.infer_try_init_error_type_from_expr(expr),
        }
    }

    pub(super) fn infer_try_init_error_type_from_block(&self, block: &syn::Block) -> Option<syn::Type> {
        for stmt in block.stmts.iter().rev() {
            match stmt {
                syn::Stmt::Expr(expr, _) => {
                    if let Some(err_ty) = self.infer_try_init_error_type_from_expr(expr) {
                        return Some(err_ty);
                    }
                }
                syn::Stmt::Local(local) => {
                    if let Some(init) = &local.init
                        && let Some(err_ty) =
                            self.infer_try_init_error_type_from_expr(init.expr.as_ref())
                    {
                        return Some(err_ty);
                    }
                }
                syn::Stmt::Item(_) | syn::Stmt::Macro(_) => {}
            }
        }
        None
    }

    pub(super) fn infer_try_init_error_type_from_expr(&self, expr: &syn::Expr) -> Option<syn::Type> {
        let expr = self.peel_paren_group_expr(expr);
        match expr {
            syn::Expr::Call(call) => {
                let syn::Expr::Path(path_expr) = call.func.as_ref() else {
                    return None;
                };
                let ctor = path_expr.path.segments.last()?.ident.to_string();
                if ctor != "Err" || call.args.len() != 1 {
                    return None;
                }
                let inferred = self
                    .infer_simple_expr_type(&call.args[0])
                    .or_else(|| self.infer_hint_type_from_expr(&call.args[0]))?;
                if self.type_is_concrete_hint_candidate(&inferred) {
                    Some(inferred)
                } else {
                    None
                }
            }
            syn::Expr::Block(block_expr) => {
                self.infer_try_init_error_type_from_block(&block_expr.block)
            }
            syn::Expr::If(if_expr) => {
                let then_err = self.infer_try_init_error_type_from_block(&if_expr.then_branch);
                let else_err = if_expr
                    .else_branch
                    .as_ref()
                    .and_then(|(_, else_expr)| self.infer_try_init_error_type_from_expr(else_expr));
                match (then_err, else_err) {
                    (Some(left), Some(right)) => {
                        if Self::types_equivalent_by_tokens(&left, &right) {
                            Some(left)
                        } else {
                            Some(left)
                        }
                    }
                    (Some(left), None) => Some(left),
                    (None, Some(right)) => Some(right),
                    (None, None) => None,
                }
            }
            syn::Expr::Match(match_expr) => {
                let mut inferred: Option<syn::Type> = None;
                for arm in &match_expr.arms {
                    let Some(arm_err) = self.infer_try_init_error_type_from_expr(&arm.body) else {
                        continue;
                    };
                    if let Some(prev) = inferred.as_ref()
                        && !Self::types_equivalent_by_tokens(prev, &arm_err)
                    {
                        return inferred;
                    }
                    inferred = Some(arm_err);
                }
                inferred
            }
            syn::Expr::Return(ret) => ret
                .expr
                .as_ref()
                .and_then(|inner| self.infer_try_init_error_type_from_expr(inner)),
            _ => None,
        }
    }

    pub(super) fn infer_map_like_callable_expected_from_call_expected(
        &self,
        receiver: &syn::Expr,
        method_name: &str,
        call_expected_ty: Option<&syn::Type>,
    ) -> Option<syn::Type> {
        if !matches!(method_name, "map" | "map_err") {
            return None;
        }
        let call_expected_ty = self.peel_reference_paren_group_type(call_expected_ty?);
        let receiver_ty = self.infer_simple_expr_type(receiver);
        let receiver_owner_and_args = receiver_ty.as_ref().and_then(|ty| {
            self.option_or_result_type_args(self.peel_reference_paren_group_type(ty))
        });
        let (call_owner, call_args) = self.option_or_result_type_args(call_expected_ty)?;
        match method_name {
            "map" => {
                if let Some((recv_owner, recv_args)) = receiver_owner_and_args.as_ref()
                    && *recv_owner == "Option"
                    && call_owner == "Option"
                {
                    let in_ty = recv_args.first()?.clone();
                    let out_ty = call_args.first()?.clone();
                    return Some(parse_quote!(impl FnOnce(#in_ty) -> #out_ty));
                }
                if let Some((recv_owner, recv_args)) = receiver_owner_and_args.as_ref()
                    && *recv_owner == "Result"
                    && call_owner == "Result"
                {
                    let recv_ok = recv_args.first()?.clone();
                    let recv_err = recv_args.get(1)?;
                    let call_ok = call_args.first()?.clone();
                    let call_err = call_args.get(1)?;
                    if Self::types_equivalent_by_tokens(
                        self.peel_reference_paren_group_type(recv_err),
                        self.peel_reference_paren_group_type(call_err),
                    ) {
                        return Some(parse_quote!(impl FnOnce(#recv_ok) -> #call_ok));
                    }
                }
                if call_owner == "Option" {
                    let out_ty = call_args.first()?.clone();
                    return Some(parse_quote!(impl FnOnce(()) -> #out_ty));
                }
                if call_owner == "Result" {
                    let out_ty = call_args.first()?.clone();
                    return Some(parse_quote!(impl FnOnce(()) -> #out_ty));
                }
            }
            "map_err" => {
                if let Some((recv_owner, recv_args)) = receiver_owner_and_args.as_ref()
                    && *recv_owner == "Result"
                    && call_owner == "Result"
                {
                    let recv_ok = recv_args.first()?;
                    let recv_err = recv_args.get(1)?.clone();
                    let call_ok = call_args.first()?;
                    let call_err = call_args.get(1)?.clone();
                    if Self::types_equivalent_by_tokens(
                        self.peel_reference_paren_group_type(recv_ok),
                        self.peel_reference_paren_group_type(call_ok),
                    ) {
                        return Some(parse_quote!(impl FnOnce(#recv_err) -> #call_err));
                    }
                }
                if call_owner == "Result" {
                    let call_err = call_args.get(1)?.clone();
                    return Some(parse_quote!(impl FnOnce(()) -> #call_err));
                }
            }
            _ => {}
        }
        None
    }

    pub(super) fn infer_transpose_receiver_expected_type_from_call_expected(
        &self,
        call_expected_ty: Option<&syn::Type>,
    ) -> Option<syn::Type> {
        let call_expected_ty = self.peel_reference_paren_group_type(call_expected_ty?);
        let (owner, type_args) = self.option_or_result_type_args(call_expected_ty)?;
        if owner == "Result" {
            let ok_ty = type_args.first()?;
            let err_ty = type_args
                .get(1)
                .cloned()
                .unwrap_or_else(|| parse_quote!(()));
            let (ok_owner, ok_args) = self.option_or_result_type_args(ok_ty)?;
            if ok_owner == "Option" {
                let inner = ok_args.first()?.clone();
                return Some(parse_quote!(Option<Result<#inner, #err_ty>>));
            }
            return None;
        }
        if owner == "Option" {
            let inner_ty = type_args.first()?;
            let (inner_owner, inner_args) = self.option_or_result_type_args(inner_ty)?;
            if inner_owner == "Result" {
                let ok_ty = inner_args.first()?.clone();
                let err_ty = inner_args
                    .get(1)
                    .cloned()
                    .unwrap_or_else(|| parse_quote!(()));
                return Some(parse_quote!(Result<Option<#ok_ty>, #err_ty>));
            }
        }
        None
    }

    pub(super) fn infer_method_arg_expected_type_from_call_expected_return(
        &self,
        receiver: &syn::Expr,
        method_name: &str,
        arg_idx: usize,
        arg_declared_expected_ty: Option<&syn::Type>,
        call_expected_ty: Option<&syn::Type>,
    ) -> Option<syn::Type> {
        let declared_expected_ty = arg_declared_expected_ty.cloned().or_else(|| {
            self.lookup_method_arg_type_from_receiver_type(receiver, method_name, arg_idx)
        })?;
        let declared_expected_ty = &declared_expected_ty;
        let call_expected_ty = call_expected_ty?;
        let should_resolve_from_call_expected = !self
            .type_is_concrete_hint_candidate(declared_expected_ty)
            || self.type_references_current_struct_assoc(declared_expected_ty)
            || self.type_looks_like_assoc_projection(declared_expected_ty);
        if !should_resolve_from_call_expected {
            return None;
        }

        let receiver_ty = match self.infer_simple_expr_type(receiver) {
            Some(ty) => ty,
            None => return None,
        };
        let mut method_return_ty = match self
            .lookup_owner_method_return_type_from_receiver_type(&receiver_ty, method_name)
        {
            Some(ty) => ty,
            None => return None,
        };
        if let Some((_, receiver_substitutions)) =
            self.receiver_owner_name_and_type_substitutions(receiver)
            && !receiver_substitutions.is_empty()
        {
            method_return_ty =
                self.substitute_type_params_in_type(&method_return_ty, &receiver_substitutions);
        }

        let mut substitutions = HashMap::new();
        if !self.collect_type_param_substitutions_from_expected_match(
            &method_return_ty,
            call_expected_ty,
            &mut substitutions,
        ) {
            return None;
        }
        if substitutions.is_empty() {
            return None;
        }
        let resolved = self.substitute_type_params_in_type(declared_expected_ty, &substitutions);
        self.type_is_concrete_hint_candidate(&resolved)
            .then_some(resolved)
    }

    pub(super) fn resolve_c_like_enum_inherent_method_free_function_for_owner_path(
        &self,
        owner_path: &str,
        method_name: &str,
    ) -> Option<String> {
        let owner_leaf = owner_path
            .rsplit("::")
            .next()
            .unwrap_or(owner_path)
            .to_string();
        let mut owner_candidates: Vec<String> = self
            .inherent_impl_method_names
            .iter()
            .filter_map(|(impl_ty, methods)| {
                if !methods.contains(method_name) {
                    return None;
                }
                let impl_tail = impl_ty.rsplit("::").next().unwrap_or(impl_ty.as_str());
                let type_matches = impl_ty == owner_path
                    || impl_ty == &owner_leaf
                    || impl_ty.ends_with(&format!("::{}", owner_leaf))
                    || owner_path.ends_with(&format!("::{}", impl_tail));
                if !type_matches {
                    return None;
                }
                let is_c_like_owner = self.c_like_enum_types.contains(impl_ty)
                    || self.c_like_enum_types.contains(impl_tail)
                    || self
                        .c_like_enum_types
                        .iter()
                        .any(|name| name.ends_with(&format!("::{}", impl_tail)));
                is_c_like_owner.then_some(impl_ty.clone())
            })
            .collect();
        owner_candidates.sort();
        owner_candidates.dedup();
        if owner_candidates.is_empty() {
            return None;
        }
        let resolved_owner = if owner_candidates
            .iter()
            .any(|candidate| candidate == owner_path)
        {
            owner_path.to_string()
        } else if owner_candidates.len() == 1 {
            owner_candidates[0].clone()
        } else {
            owner_candidates
                .into_iter()
                .max_by_key(|candidate| Self::common_module_prefix_depth(candidate, owner_path))
                .unwrap_or_else(|| owner_path.to_string())
        };
        let method_cpp = escape_cpp_keyword(method_name);
        let owner_segments: Vec<&str> = resolved_owner.split("::").collect();
        if owner_segments.len() <= 1 {
            return Some(method_cpp);
        }
        let owner_ns_raw = owner_segments[..owner_segments.len() - 1].join("::");
        let owner_ns_raw = Self::strip_crate_root_cpp_path(&owner_ns_raw);
        let owner_ns = self.escape_and_rename_qualified_name(&owner_ns_raw);
        Some(format!("::{}::{}", owner_ns, method_cpp))
    }

    pub(super) fn resolve_c_like_enum_inherent_method_free_function(
        &self,
        receiver: &syn::Expr,
        method_name: &str,
    ) -> Option<String> {
        let owner_path = if let Some(receiver_ty) = self.infer_simple_expr_type(receiver) {
            let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
            if let syn::Type::Path(tp) = receiver_ty {
                let segments: Vec<String> = tp
                    .path
                    .segments
                    .iter()
                    .map(|seg| seg.ident.to_string())
                    .collect();
                if segments.is_empty() {
                    None
                } else {
                    Some(segments.join("::"))
                }
            } else {
                None
            }
        } else {
            None
        }
        .or_else(|| {
            // Fallback for enum-variant paths like `Enum::Variant.method()`
            // when local inference does not recover the owner enum type.
            let receiver = self.peel_paren_group_expr(receiver);
            let syn::Expr::Path(path_expr) = receiver else {
                return None;
            };
            if path_expr.path.segments.len() < 2 {
                return None;
            }
            let segments: Vec<String> = path_expr
                .path
                .segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .collect();
            let mut owner_segments = segments.clone();
            let variant_name = owner_segments.pop().unwrap_or_default();
            if owner_segments.is_empty() {
                return None;
            }
            let owner_path = owner_segments.join("::");
            let owner_leaf = owner_segments.last().cloned().unwrap_or_default();
            let looks_like_variant = self
                .path_matches_c_like_enum_const(&owner_path, &variant_name)
                || self.path_matches_c_like_enum_const(&owner_leaf, &variant_name);
            if looks_like_variant {
                Some(owner_path)
            } else {
                None
            }
        })?;
        self.resolve_c_like_enum_inherent_method_free_function_for_owner_path(
            &owner_path,
            method_name,
        )
    }

    pub(super) fn resolve_expected_type_for_struct_literal(
        &self,
        expected_ty: Option<&syn::Type>,
        struct_path: &syn::Path,
    ) -> Option<syn::Type> {
        let expected_ty = self.peel_reference_paren_group_type(expected_ty?);
        if self.expected_type_matches_struct_literal_path(expected_ty, struct_path) {
            return Some(expected_ty.clone());
        }
        let mut current = expected_ty.clone();
        for _ in 0..4 {
            let Some(next) = self.resolve_type_alias_once(&current) else {
                break;
            };
            if self.expected_type_matches_struct_literal_path(&next, struct_path) {
                return Some(next);
            }
            if next == current {
                break;
            }
            current = next;
        }
        None
    }

    pub(super) fn resolve_type_alias_once(&self, ty: &syn::Type) -> Option<syn::Type> {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return None;
        };
        if tp.qself.is_some() {
            return None;
        }
        let last = tp.path.segments.last()?;
        let alias_name = last.ident.to_string();
        let full_alias_name = tp
            .path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        let path_is_qualified = tp.path.segments.len() > 1;

        let mut direct_candidates = Vec::new();
        if !path_is_qualified {
            direct_candidates.push(self.scoped_type_key(&alias_name));
        }
        direct_candidates.push(full_alias_name.clone());
        let alias_key = if let Some(direct) = direct_candidates
            .into_iter()
            .find(|key| self.type_alias_targets.contains_key(key))
        {
            direct
        } else {
            let mut suffix_matches: Vec<String> = self
                .type_alias_targets
                .keys()
                .filter(|key| {
                    key.rsplit("::")
                        .next()
                        .is_some_and(|tail| tail == alias_name)
                        && (key.as_str() == full_alias_name
                            || key.ends_with(&format!("::{}", full_alias_name))
                            || (!path_is_qualified
                                && full_alias_name.ends_with(&format!("::{}", key))))
                })
                .cloned()
                .collect();
            suffix_matches.sort();
            suffix_matches.dedup();
            if suffix_matches.len() == 1 {
                suffix_matches.pop().expect("len() == 1")
            } else {
                return None;
            }
        };
        let mut resolved = self.type_alias_targets.get(&alias_key)?.clone();

        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
            return Some(resolved);
        };
        let params = self
            .declared_type_params
            .get(&alias_key)
            .or_else(|| self.declared_type_params.get(&alias_name))?;
        let param_kinds = self
            .declared_type_param_kinds
            .get(&alias_key)
            .or_else(|| self.declared_type_param_kinds.get(&alias_name));
        let provided_type_args: Vec<syn::Type> = args
            .args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(ty) => Some(ty.clone()),
                _ => None,
            })
            .collect();
        if provided_type_args.is_empty() {
            return Some(resolved);
        }
        let mut substitutions = HashMap::new();
        let mut provided_iter = provided_type_args.into_iter();
        for (idx, param) in params.iter().enumerate() {
            let is_type_param = param_kinds
                .and_then(|kinds| kinds.get(idx))
                .is_none_or(|kind| matches!(kind, GenericParamKind::Type));
            if !is_type_param {
                continue;
            }
            let Some(provided_ty) = provided_iter.next() else {
                break;
            };
            substitutions.insert(param.clone(), provided_ty);
        }
        if !substitutions.is_empty() {
            resolved = self.substitute_type_params_in_type(&resolved, &substitutions);
        }
        Some(resolved)
    }

    pub(super) fn resolve_option_inner_cpp_type_for_forced_softened_scope(
        &self,
        inner_ty: &syn::Type,
    ) -> Option<String> {
        let mut inner_cpp = self.map_type(inner_ty);
        let references_unemitted_current_assoc = self
            .type_references_current_struct_assoc_projection(inner_ty)
            && !self.type_current_struct_assoc_aliases_emitted(inner_ty);
        if !references_unemitted_current_assoc
            && let Some(resolved_inner_cpp) =
                self.resolve_current_struct_assoc_projection_cpp_type(inner_ty)
        {
            inner_cpp = resolved_inner_cpp;
        }
        if !self.option_softened_dependent_inner_is_safe(&inner_cpp) {
            return None;
        }
        Some(inner_cpp)
    }

    pub(super) fn resolve_current_struct_assoc_projection_cpp_type(&self, ty: &syn::Type) -> Option<String> {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return None;
        };
        if tp.qself.is_some() || !self.path_is_current_struct_assoc_projection(&tp.path) {
            return None;
        }
        let assoc_seg = tp.path.segments.iter().nth(1)?;
        let mut resolved_assoc_cpp =
            self.resolve_current_struct_assoc_cpp_type(&assoc_seg.ident.to_string())?;
        if tp.path.segments.len() > 2 {
            let tail = tp
                .path
                .segments
                .iter()
                .skip(2)
                .map(|seg| escape_cpp_keyword(&seg.ident.to_string()))
                .collect::<Vec<_>>()
                .join("::");
            if !tail.is_empty() {
                resolved_assoc_cpp = format!("{}::{}", resolved_assoc_cpp, tail);
            }
        }
        Some(resolved_assoc_cpp)
    }

    pub(super) fn resolve_result_ctor_expected_type_from_ctor_arg(
        &self,
        expected_ty: &syn::Type,
        ctor_idx: usize,
        ctor_arg: &syn::Expr,
    ) -> Option<syn::Type> {
        let mut resolved = self.peel_reference_paren_group_type(expected_ty).clone();
        let syn::Type::Path(tp) = &mut resolved else {
            return None;
        };
        let last = tp.path.segments.last_mut()?;
        if last.ident != "Result" {
            return None;
        }
        let syn::PathArguments::AngleBracketed(args) = &mut last.arguments else {
            return None;
        };
        let mut type_arg_positions = Vec::new();
        for (idx, arg) in args.args.iter().enumerate() {
            if matches!(arg, syn::GenericArgument::Type(_)) {
                type_arg_positions.push(idx);
            }
        }
        if type_arg_positions.len() != 2 {
            return None;
        }
        let target_pos = *type_arg_positions.get(ctor_idx)?;
        let syn::GenericArgument::Type(target_ty) = args.args.get_mut(target_pos)? else {
            return None;
        };
        if !self.type_contains_unresolved_placeholder_like(target_ty)
            && !self.type_maps_to_auto_placeholder_like(target_ty)
        {
            return Some(resolved);
        }
        let inferred_ty = self
            .infer_simple_expr_type(ctor_arg)
            .or_else(|| self.infer_hint_type_from_expr(ctor_arg))?;
        if self.type_contains_unresolved_placeholder_like(&inferred_ty) {
            return None;
        }
        // Don't substitute the signature's T with the ctor arg's inferred
        // type when that inferred type is just `Self`. The signature's T
        // position is already the Ok-arm specific type (e.g.
        // `Handle<NodeRef<…>, marker::KV>`); `Self` is the impl's host
        // (e.g. `Handle<NodeRef<…>, marker::Edge>`), which is what the E
        // position already holds. Substituting T with Self collapses
        // both arms to the same type and emits a broken
        // `Result<Self, Self>::Ok(...)` qualifier.
        //
        // Trigger: the Ok-arm ctor expression returns `Self` from a
        // parallel impl block whose type-params decompose structurally
        // into the host struct's single `Node` parameter (Cluster A).
        // The decomposed params (e.g. `K, V`) are no longer in
        // `type_param_scopes`, so the placeholder-check above falsely
        // flags the signature's T as unresolved and routes through this
        // inference fallback. Bailing here preserves the signature.
        if let syn::Type::Path(tp) = &inferred_ty
            && tp.qself.is_none()
            && tp.path.segments.len() == 1
            && tp.path.segments[0].ident == "Self"
        {
            return None;
        }
        *target_ty = inferred_ty;
        Some(resolved)
    }

    pub(super) fn resolve_result_error_slot_from_expected(
        &self,
        result_ty: &syn::Type,
        expected_err_ty: &syn::Type,
    ) -> Option<syn::Type> {
        if self.type_contains_unresolved_placeholder_like(expected_err_ty) {
            return None;
        }

        let mut resolved = self.peel_reference_paren_group_type(result_ty).clone();
        let syn::Type::Path(tp) = &mut resolved else {
            return None;
        };
        let last = tp.path.segments.last_mut()?;
        if last.ident != "Result" {
            return None;
        }
        let syn::PathArguments::AngleBracketed(args) = &mut last.arguments else {
            return None;
        };

        let type_arg_positions: Vec<usize> = args
            .args
            .iter()
            .enumerate()
            .filter_map(|(idx, arg)| match arg {
                syn::GenericArgument::Type(_) => Some(idx),
                _ => None,
            })
            .collect();
        if type_arg_positions.len() != 2 {
            return None;
        }

        let err_pos = type_arg_positions[1];
        let syn::GenericArgument::Type(err_ty) = args.args.get_mut(err_pos)? else {
            return None;
        };
        if self.type_contains_unresolved_placeholder_like(err_ty)
            || self.type_maps_to_auto_placeholder_like(err_ty)
        {
            *err_ty = expected_err_ty.clone();
        }
        Some(resolved)
    }

    pub(super) fn infer_hint_type_from_expr(&self, expr: &syn::Expr) -> Option<syn::Type> {
        let expr = self.peel_paren_group_expr(expr);
        match expr {
            syn::Expr::Lit(lit) => self.infer_literal_type(&lit.lit),
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                let name = path.path.segments[0].ident.to_string();
                // Try local binding type, but skip it if it's a type parameter.
                // Type parameters (e.g., T, F, E) are not valid C++ template args —
                // falling through to decltype-based inference is better.
                let local_binding_ty = self
                    .lookup_local_binding_type(&name)
                    .filter(|ty| !Self::is_type_parameter(ty));
                local_binding_ty
                    .or_else(|| self.lookup_local_placeholder_type_hint(&name).cloned())
                    .or_else(|| {
                        let looks_like_type = name
                            .chars()
                            .next()
                            .is_some_and(|ch| ch.is_ascii_uppercase());
                        if looks_like_type
                            || self.local_declared_types.contains(&name)
                            || self.declared_item_names.contains(&name)
                        {
                            Some(syn::Type::Path(syn::TypePath {
                                qself: None,
                                path: path.path.clone(),
                            }))
                        } else {
                            None
                        }
                    })
            }
            syn::Expr::Reference(r) => self.infer_hint_type_from_expr(&r.expr),
            syn::Expr::Cast(cast) => {
                if !matches!(cast.ty.as_ref(), syn::Type::Infer(_)) {
                    Some((*cast.ty).clone())
                } else {
                    self.infer_hint_type_from_expr(&cast.expr)
                }
            }
            syn::Expr::Call(call) => {
                if let syn::Expr::Path(path_expr) = call.func.as_ref() {
                    let joined = path_expr
                        .path
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    if matches!(
                        joined.as_str(),
                        "Some"
                            | "Option::Some"
                            | "core::option::Option::Some"
                            | "std::option::Option::Some"
                    ) && call.args.len() == 1
                    {
                        return call
                            .args
                            .first()
                            .and_then(|arg| self.infer_hint_type_from_expr(arg));
                    }
                    if matches!(
                        joined.as_str(),
                        "Ok" | "Result::Ok"
                            | "core::result::Result::Ok"
                            | "std::result::Result::Ok"
                            | "Err"
                            | "Result::Err"
                            | "core::result::Result::Err"
                            | "std::result::Result::Err"
                    ) && call.args.len() == 1
                    {
                        return call
                            .args
                            .first()
                            .and_then(|arg| self.infer_hint_type_from_expr(arg));
                    }
                    if matches!(
                        joined.as_str(),
                        "rusty::boxed::into_vec"
                            | "into_vec"
                            | "alloc::boxed::into_vec"
                            | "std::boxed::into_vec"
                    ) && call.args.len() == 1
                    {
                        if let Some(inner) = self.infer_boxed_array_element_type(&call.args[0]) {
                            let out: syn::Type = parse_quote!(rusty::Vec<#inner>);
                            return Some(out);
                        }
                    }
                    if matches!(
                        joined.as_str(),
                        "rusty::boxed::box_assume_init_into_vec_unsafe"
                            | "box_assume_init_into_vec_unsafe"
                            | "alloc::boxed::box_assume_init_into_vec_unsafe"
                            | "std::boxed::box_assume_init_into_vec_unsafe"
                    ) && call.args.len() == 1
                    {
                        if let Some(inner) = self.infer_boxed_array_element_type(&call.args[0]) {
                            let out: syn::Type = parse_quote!(rusty::Vec<#inner>);
                            return Some(out);
                        }
                    }
                    if matches!(
                        joined.as_str(),
                        "std::move" | "core::mem::move" | "rusty::move" | "move"
                    ) && call.args.len() == 1
                    {
                        return self.infer_hint_type_from_expr(&call.args[0]);
                    }
                    if path_expr.path.segments.len() == 1
                        && path_expr.path.segments[0]
                            .ident
                            .to_string()
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_uppercase())
                    {
                        return Some(syn::Type::Path(syn::TypePath {
                            qself: None,
                            path: path_expr.path.clone(),
                        }));
                    }
                }
                self.infer_simple_expr_type(expr)
            }
            syn::Expr::MethodCall(mc) => {
                if mc.method == "into" && mc.args.is_empty() {
                    if matches!(
                        self.peel_paren_group_expr(&mc.receiver),
                        syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(_),
                            ..
                        })
                    ) {
                        let out: syn::Type = parse_quote!(rusty::String);
                        return Some(out);
                    }
                }
                if mc.args.is_empty()
                    && matches!(mc.method.to_string().as_str(), "span" | "before" | "after")
                    && let Ok(span_ty) = syn::parse_str::<syn::Type>("Span")
                {
                    return Some(span_ty);
                }
                self.infer_simple_expr_type(expr)
            }
            _ => self.infer_simple_expr_type(expr),
        }
    }

    pub(super) fn infer_boxed_array_element_type(&self, expr: &syn::Expr) -> Option<syn::Type> {
        let expr = self.peel_paren_group_expr(expr);
        if let syn::Expr::Call(call) = expr {
            if let syn::Expr::Path(path_expr) = call.func.as_ref() {
                let joined = path_expr
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                if matches!(
                    joined.as_str(),
                    "rusty::boxed::box_new"
                        | "box_new"
                        | "alloc::boxed::box_new"
                        | "std::boxed::box_new"
                ) && call.args.len() == 1
                {
                    return self.infer_array_element_type_from_expr(&call.args[0]);
                }
                if matches!(
                    joined.as_str(),
                    "rusty::intrinsics::write_box_via_move"
                        | "write_box_via_move"
                        | "alloc::intrinsics::write_box_via_move"
                        | "std::intrinsics::write_box_via_move"
                ) && call.args.len() == 2
                {
                    return self.infer_array_element_type_from_expr(&call.args[1]);
                }
                if matches!(
                    joined.as_str(),
                    "rusty::boxed::box_assume_init_into_vec_unsafe"
                        | "box_assume_init_into_vec_unsafe"
                        | "alloc::boxed::box_assume_init_into_vec_unsafe"
                        | "std::boxed::box_assume_init_into_vec_unsafe"
                ) && call.args.len() == 1
                {
                    return self.infer_boxed_array_element_type(&call.args[0]);
                }
            }
        }
        self.infer_array_element_type_from_expr(expr)
    }

    pub(super) fn infer_array_element_type_from_expr(&self, expr: &syn::Expr) -> Option<syn::Type> {
        let expr = self.peel_paren_group_expr(expr);
        match expr {
            syn::Expr::Array(arr) => arr
                .elems
                .first()
                .and_then(|elem| self.infer_hint_type_from_expr(elem)),
            syn::Expr::Repeat(repeat) => self.infer_hint_type_from_expr(&repeat.expr),
            syn::Expr::Reference(r) => self.infer_array_element_type_from_expr(&r.expr),
            syn::Expr::Cast(c) => {
                if let Some(from_cast) = self.extract_iter_item_type_from_type(&c.ty) {
                    if !matches!(from_cast, syn::Type::Infer(_)) {
                        return Some(from_cast);
                    }
                }
                self.infer_array_element_type_from_expr(&c.expr)
            }
            _ => self
                .infer_simple_expr_type(expr)
                .and_then(|ty| self.extract_iter_item_type_from_type(&ty)),
        }
    }

    pub(super) fn infer_tuple_type_from_tuple_expr(
        &self,
        tuple_expr: &syn::ExprTuple,
    ) -> Option<syn::TypeTuple> {
        let mut elems: syn::punctuated::Punctuated<syn::Type, syn::token::Comma> =
            syn::punctuated::Punctuated::new();
        for elem in tuple_expr.elems.iter() {
            elems.push(self.infer_hint_type_from_expr(elem)?);
        }
        Some(syn::TypeTuple {
            paren_token: syn::token::Paren::default(),
            elems,
        })
    }

    pub(super) fn infer_iter_item_type_from_expr(&self, expr: &syn::Expr) -> Option<syn::Type> {
        let expr = self.peel_paren_group_expr(expr);
        match expr {
            syn::Expr::MethodCall(mc) => {
                let method = mc.method.to_string();
                if method == "map" && mc.args.len() == 1 {
                    let source_item_ty = self.infer_iter_item_type_from_expr(&mc.receiver);
                    if let syn::Expr::Closure(closure) = self.peel_paren_group_expr(&mc.args[0]) {
                        if let Some(ret) = self.infer_hint_type_from_expr(&closure.body) {
                            return Some(ret);
                        }
                        if let Some(source_item_ty) = source_item_ty.as_ref()
                            && let Some(ret) = self
                                .infer_map_closure_item_type_from_source(closure, source_item_ty)
                        {
                            return Some(ret);
                        }
                    }
                    return source_item_ty;
                }
                if method == "filter_map" && mc.args.len() == 1 {
                    if let syn::Expr::Closure(closure) = self.peel_paren_group_expr(&mc.args[0]) {
                        if let Some(ret_ty) =
                            self.infer_hint_type_from_expr(&closure.body).or_else(|| {
                                self.infer_local_binding_type_from_initializer(&closure.body)
                            })
                        {
                            if let Some(option_inner) = self.expected_option_type_arg(Some(&ret_ty))
                            {
                                return Some(option_inner.clone());
                            }
                            return Some(ret_ty);
                        }
                    }
                    return self.infer_iter_item_type_from_expr(&mc.receiver);
                }
                if method == "drain" {
                    if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver) {
                        if let Some(item_ty) = self.extract_iter_item_type_from_type(&receiver_ty) {
                            return Some(item_ty);
                        }
                    }
                }
                if method == "iter" || method == "iter_mut" || method == "into_iter" {
                    if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver) {
                        if let Some(item_ty) = self.extract_iter_item_type_from_type(&receiver_ty) {
                            return Some(item_ty);
                        }
                    }
                }
                if method == "split" && mc.args.len() == 1 {
                    return Some(parse_quote!(&str));
                }
                if (method == "bytes" || method == "as_bytes") && mc.args.is_empty() {
                    if let Some(receiver_ty) = self.infer_simple_expr_type(&mc.receiver) {
                        if !self.is_known_string_like_type(&receiver_ty) {
                            return self.infer_iter_item_type_from_expr(&mc.receiver);
                        }
                    }
                    return Some(parse_quote!(u8));
                }
                self.infer_iter_item_type_from_expr(&mc.receiver)
            }
            syn::Expr::Call(call) => {
                if let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(call.func.as_ref()) {
                    let joined = path_expr
                        .path
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    if call.args.len() == 1
                        && matches!(
                            joined.as_str(),
                            "slice_full"
                                | "rusty::slice_full"
                                | "iter"
                                | "rusty::iter"
                                | "iter_mut"
                                | "rusty::iter_mut"
                                | "rev"
                                | "rusty::rev"
                                | "enumerate"
                                | "rusty::enumerate"
                        )
                    {
                        if let Some(source_ty) = self.infer_simple_expr_type(&call.args[0]) {
                            if let Some(item_ty) = self.extract_iter_item_type_from_type(&source_ty)
                            {
                                return Some(item_ty);
                            }
                        }
                        return self.infer_iter_item_type_from_expr(&call.args[0]);
                    }
                    if call.args.len() == 1
                        && matches!(
                            joined.as_str(),
                            "repeat"
                                | "iter::repeat"
                                | "core::iter::repeat"
                                | "std::iter::repeat"
                                | "rusty::repeat"
                        )
                    {
                        if let Some(item_ty) = self.infer_simple_expr_type(&call.args[0]) {
                            return Some(item_ty);
                        }
                        if let Some(item_ty) = self.infer_hint_type_from_expr(&call.args[0]) {
                            return Some(item_ty);
                        }
                    }
                    if joined == "map" || joined == "rusty::map" {
                        let source_item_ty = call
                            .args
                            .first()
                            .and_then(|source| self.infer_iter_item_type_from_expr(source));
                        if call.args.len() >= 2 {
                            if let syn::Expr::Closure(closure) =
                                self.peel_paren_group_expr(&call.args[1])
                            {
                                if let Some(ret) = self.infer_hint_type_from_expr(&closure.body) {
                                    return Some(ret);
                                }
                                if let Some(source_item_ty) = source_item_ty.as_ref()
                                    && let Some(ret) = self.infer_map_closure_item_type_from_source(
                                        closure,
                                        source_item_ty,
                                    )
                                {
                                    return Some(ret);
                                }
                            }
                        }
                        return source_item_ty;
                    }
                    if (joined == "take" || joined == "rusty::take") && !call.args.is_empty() {
                        return self.infer_iter_item_type_from_expr(&call.args[0]);
                    }
                    if (joined == "skip" || joined == "rusty::skip") && !call.args.is_empty() {
                        return self.infer_iter_item_type_from_expr(&call.args[0]);
                    }
                    if (joined == "scan" || joined == "rusty::scan") && !call.args.is_empty() {
                        return self.infer_iter_item_type_from_expr(&call.args[0]);
                    }
                    if (joined == "filter" || joined == "rusty::filter") && !call.args.is_empty() {
                        return self.infer_iter_item_type_from_expr(&call.args[0]);
                    }
                }
                self.infer_simple_expr_type(expr)
                    .and_then(|ty| self.extract_iter_item_type_from_type(&ty))
            }
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                let name = path.path.segments[0].ident.to_string();
                self.lookup_local_binding_type(&name)
                    .and_then(|ty| self.extract_iter_item_type_from_type(&ty))
            }
            syn::Expr::Index(idx) => self
                .infer_simple_expr_type(&idx.expr)
                .and_then(|ty| self.extract_iter_item_type_from_type(&ty)),
            syn::Expr::Field(field) => {
                // For field expressions like `this->comparators`, first try to get the
                // field's type directly and extract the iterator item type from it.
                if let Some(field_ty) = self.infer_simple_expr_type(expr) {
                    if let Some(item_ty) = self.extract_iter_item_type_from_type(&field_ty) {
                        return Some(item_ty);
                    }
                }
                // Fallback: try to get the iterator item type from the base expression
                self.infer_iter_item_type_from_expr(&field.base)
            }
            _ => self
                .infer_simple_expr_type(expr)
                .and_then(|ty| self.extract_iter_item_type_from_type(&ty)),
        }
    }

    pub(super) fn infer_map_closure_item_type_from_source(
        &self,
        closure: &syn::ExprClosure,
        source_item_ty: &syn::Type,
    ) -> Option<syn::Type> {
        if closure.inputs.len() != 1 {
            return None;
        }
        let syn::Pat::Ident(param_ident) = closure.inputs.first()? else {
            return None;
        };
        let param_name = param_ident.ident.to_string();
        let deref_depth = self.expr_deref_chain_depth_for_ident(&closure.body, &param_name)?;
        // Keep inference aligned with map closure emission:
        // for untyped iterator-map params, collapse one deref layer.
        let effective_deref_depth = deref_depth.saturating_sub(1);
        let mut inferred = source_item_ty.clone();
        for _ in 0..effective_deref_depth {
            inferred = self.infer_deref_result_type_from_type(&inferred)?;
        }
        Some(inferred)
    }

    pub(super) fn infer_deref_result_type_from_type(&self, ty: &syn::Type) -> Option<syn::Type> {
        let ty = self.peel_reference_paren_group_type(ty);
        match ty {
            syn::Type::Reference(reference) => Some((*reference.elem).clone()),
            syn::Type::Ptr(pointer) => Some((*pointer.elem).clone()),
            syn::Type::Path(tp) => {
                let last = tp.path.segments.last()?;
                let owner = last.ident.to_string();
                let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
                    return None;
                };
                let first_type_arg = || {
                    args.args.iter().find_map(|arg| match arg {
                        syn::GenericArgument::Type(t) => Some(t.clone()),
                        _ => None,
                    })
                };
                match owner.as_str() {
                    "associated_item_t" => {
                        let owner_ty = first_type_arg()?;
                        let item_ty = self.extract_iter_item_type_from_type(&owner_ty)?;
                        self.infer_deref_result_type_from_type(&item_ty)
                    }
                    "Box" | "NonNull" | "ConstNonNull" | "Ptr" | "MutPtr" | "Unique"
                    | "reference_wrapper" => first_type_arg(),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub(super) fn infer_array_capacity_arg_for_expr(&self, expr: &syn::Expr) -> Option<String> {
        let expr = self.peel_paren_group_expr(expr);
        match expr {
            syn::Expr::Array(arr) => Some(arr.elems.len().to_string()),
            syn::Expr::Repeat(repeat) => Some(self.emit_expr_to_string(&repeat.len)),
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::ByteStr(bs),
                ..
            }) => Some(bs.value().len().to_string()),
            _ => {
                if let syn::Expr::Path(path) = expr
                    && path.path.segments.len() == 1
                {
                    let name = path.path.segments[0].ident.to_string();
                    if let Some(local_ty) = self.lookup_local_binding_type(&name)
                        && let syn::Type::Array(arr) =
                            self.peel_reference_paren_group_type(&local_ty)
                    {
                        return Some(self.emit_expr_to_string(&arr.len));
                    }
                    if let Some(hint_ty) = self.lookup_local_placeholder_type_hint(&name)
                        && let syn::Type::Array(arr) = self.peel_reference_paren_group_type(hint_ty)
                    {
                        return Some(self.emit_expr_to_string(&arr.len));
                    }
                }
                if let Some(arg_ty) = self.infer_simple_expr_type(expr) {
                    if let syn::Type::Array(arr) = arg_ty {
                        return Some(self.emit_expr_to_string(&arr.len));
                    }
                }
                // Fallback for local/forwarded array-like values whose concrete type
                // is only visible in C++ surface form (`auto buf = std::array{...};`).
                // This keeps const-generic owners like ByteArray<N> concrete.
                let tuple_size_base = match expr {
                    syn::Expr::Path(_) => Some(expr),
                    syn::Expr::Reference(reference) => Some(reference.expr.as_ref()),
                    syn::Expr::Call(call) => {
                        if call.args.len() == 1
                            && let syn::Expr::Path(path_expr) =
                                self.peel_paren_group_expr(call.func.as_ref())
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
                                "std::move" | "core::mem::move" | "rusty::move" | "move"
                            ) {
                                call.args.first()
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                if let Some(base_expr) = tuple_size_base {
                    let base_cpp = self.emit_expr_to_string(base_expr);
                    if !base_cpp.contains('{') && !base_cpp.contains(';') {
                        return Some(format!(
                            "std::tuple_size_v<std::remove_reference_t<decltype(({}))>>",
                            base_cpp
                        ));
                    }
                }
                None
            }
        }
    }

    pub(super) fn infer_remove_cvref_decltype_from_expr(&self, expr: &syn::Expr) -> Option<String> {
        // Peel `&` / `&mut` from the source expr before taking decltype: in
        // C++ `&x` evaluates to a *pointer* (e.g. `Rc<int>*`), so
        // `std::remove_cvref_t<decltype(&x)>` recovers `Rc<int>*` — wrong
        // for callers that want the pointee type (e.g. as a template arg
        // for `Rc<T>::strong_count(&x)`). Rust `&x` is just a reference,
        // and the call sites here use the recovered type as the template
        // parameter of a wrapper owner, where the pointee is what's
        // needed. `std::remove_cvref_t` would still leave a stray
        // pointer; strip the reference syntactically first.
        let peeled = match expr {
            syn::Expr::Reference(reference) => reference.expr.as_ref(),
            _ => expr,
        };
        let expr_cpp = self.emit_expr_to_string(peeled);
        // GNU statement-expression lowering produces `({ ...; ...; expr; })`
        // which is not a valid operand for `decltype((...))`. Reject those
        // specifically. Brace-initializer expressions like `rusty::Vec{1, 2}`
        // are fine — `decltype(T{args})` is well-formed C++.
        let trimmed = expr_cpp.trim_start();
        if trimmed.starts_with("({") || expr_cpp.contains(';') {
            return None;
        }
        Some(format!("std::remove_cvref_t<decltype(({}))>", expr_cpp))
    }

    /// Look up the called method in the defining impl block(s) for `owner_name`,
    /// and extract concrete owner-template-arg values from the impl block's
    /// `Self<…>` arguments. For generic positions whose impl-block name is
    /// in scope on the call site, use that name; for positions whose
    /// impl-block name isn't in scope but the call has a corresponding
    /// argument, fall back to `std::remove_cvref_t<decltype(arg))>`; otherwise
    /// leave the position as `None`.
    ///
    /// Used as a last-resort fallback in `emit_call_func_with_owner_template_recovery`
    /// before emitting bare `auto` placeholders. For `Handle::new_edge(node, idx)`
    /// where `impl<…> Handle<NodeRef<…>, marker::Edge>` defines `new_edge`,
    /// position 0 deduces via `decltype(node)` and position 1 takes the
    /// concrete `marker::Edge` from the impl signature.
    pub(super) fn infer_owner_template_args_from_defining_impl_block(
        &self,
        owner_name: &str,
        method_name: &str,
        call: &syn::ExprCall,
    ) -> Option<Vec<Option<String>>> {
        // Walk every recorded ItemImpl and find one whose self_ty is
        // `Owner<…>` (matching `owner_name`) and whose body contains a
        // method named `method_name`. Use the FIRST match — for our
        // BTreeMap case `new_edge`/`new_kv` each have exactly one
        // defining impl block.
        let mut chosen: Option<&syn::ItemImpl> = None;
        for item_impl in self.cross_file_impl_blocks.iter() {
            // Self type must be a `Owner<…>` path.
            let syn::Type::Path(tp) = item_impl.self_ty.as_ref() else {
                continue;
            };
            let Some(last) = tp.path.segments.last() else {
                continue;
            };
            if last.ident != owner_name {
                continue;
            }
            // Body must define a method with the requested name.
            let defines_method = item_impl.items.iter().any(|item| match item {
                syn::ImplItem::Fn(f) => f.sig.ident == method_name,
                _ => false,
            });
            if !defines_method {
                continue;
            }
            chosen = Some(item_impl);
            break;
        }
        let item_impl = chosen?;
        // Extract `Self<…>`'s angle-bracket args from the impl's self_ty.
        let syn::Type::Path(tp) = item_impl.self_ty.as_ref() else {
            return None;
        };
        let last = tp.path.segments.last()?;
        let syn::PathArguments::AngleBracketed(impl_args) = &last.arguments else {
            return None;
        };
        // Impl-block declared generics: their names appear in
        // `item_impl.generics`. We'll deduce them from call args.
        let impl_generic_names: std::collections::HashSet<String> = item_impl
            .generics
            .params
            .iter()
            .filter_map(|p| match p {
                syn::GenericParam::Type(tp) => Some(tp.ident.to_string()),
                _ => None,
            })
            .collect();
        // Look up the method's signature within the chosen impl block so
        // we can map an impl arg containing impl-only generics to the
        // call argument that supplies a value of that type.
        let method_sig = item_impl.items.iter().find_map(|item| match item {
            syn::ImplItem::Fn(f) if f.sig.ident == method_name => Some(&f.sig),
            _ => None,
        });
        // True when `ty` recursively mentions any name in `names`.
        fn ty_mentions_any(ty: &syn::Type, names: &std::collections::HashSet<String>) -> bool {
            match ty {
                syn::Type::Path(tp) => {
                    for seg in &tp.path.segments {
                        if names.contains(&seg.ident.to_string()) {
                            return true;
                        }
                        if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                            if args.args.iter().any(|arg| match arg {
                                syn::GenericArgument::Type(inner) => ty_mentions_any(inner, names),
                                _ => false,
                            }) {
                                return true;
                            }
                        }
                    }
                    false
                }
                syn::Type::Reference(r) => ty_mentions_any(&r.elem, names),
                syn::Type::Ptr(p) => ty_mentions_any(&p.elem, names),
                syn::Type::Paren(p) => ty_mentions_any(&p.elem, names),
                syn::Type::Group(g) => ty_mentions_any(&g.elem, names),
                syn::Type::Tuple(t) => t.elems.iter().any(|e| ty_mentions_any(e, names)),
                syn::Type::Array(a) => ty_mentions_any(&a.elem, names),
                syn::Type::Slice(s) => ty_mentions_any(&s.elem, names),
                _ => false,
            }
        }
        // Find the first non-receiver method parameter whose type
        // matches `target` (after stripping references), and return
        // its position relative to the call arg list (i.e. skipping
        // `self`).
        let find_call_arg_idx_for_type = |target: &syn::Type| -> Option<usize> {
            let sig = method_sig?;
            let target_str = quote::quote!(#target).to_string();
            let mut call_idx = 0usize;
            for input in sig.inputs.iter() {
                let syn::FnArg::Typed(pat_ty) = input else {
                    // Receiver (`self`, `&self`, `&mut self`) — does not
                    // appear in `call.args` for `Owner::method(...)`.
                    continue;
                };
                // Strip a single outer reference for matching since the
                // call arg may be a (re)borrow expression and decltype
                // already produces a value type after remove_cvref_t.
                let param_ty: &syn::Type = match pat_ty.ty.as_ref() {
                    syn::Type::Reference(r) => &r.elem,
                    other => other,
                };
                if quote::quote!(#param_ty).to_string() == target_str {
                    return Some(call_idx);
                }
                call_idx += 1;
            }
            None
        };
        let mut out: Vec<Option<String>> = Vec::new();
        for arg in impl_args.args.iter() {
            let syn::GenericArgument::Type(arg_ty) = arg else {
                out.push(None);
                continue;
            };
            let mentions_impl_generic = ty_mentions_any(arg_ty, &impl_generic_names);
            if !mentions_impl_generic {
                // Fully concrete (e.g. `marker::Edge`) — emit literally.
                let mapped = self.map_type(arg_ty);
                if mapped == "auto"
                    || mapped.contains("/* TODO")
                    || type_string_has_auto_placeholder(&mapped)
                {
                    out.push(None);
                } else {
                    out.push(Some(mapped));
                }
                continue;
            }
            // Position mentions impl-only generics. Locate the method
            // parameter whose declared type matches this impl arg
            // exactly, then use `decltype(call.args[that_idx])`.
            if let Some(call_arg_idx) = find_call_arg_idx_for_type(arg_ty) {
                if let Some(call_arg) = call.args.get(call_arg_idx) {
                    if let Some(decltype_str) =
                        self.infer_remove_cvref_decltype_from_expr(call_arg)
                    {
                        out.push(Some(decltype_str));
                        continue;
                    }
                }
            }
            out.push(None);
        }
        if out.iter().all(|entry| entry.is_none()) {
            return None;
        }
        Some(out)
    }

    pub(super) fn infer_owner_first_type_arg_from_expr(
        &self,
        owner_name: &str,
        expr: &syn::Expr,
    ) -> Option<String> {
        let arg_ty = self
            .infer_hint_type_from_expr(expr)
            .or_else(|| self.infer_simple_expr_type(expr))?;
        let arg_ty = self.peel_reference_paren_group_type(&arg_ty);
        let syn::Type::Path(tp) = arg_ty else {
            return None;
        };
        let last = tp.path.segments.last()?;
        if last.ident != owner_name {
            return None;
        }
        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
            return None;
        };
        args.args.iter().find_map(|arg| match arg {
            syn::GenericArgument::Type(t) => Some(self.map_type(t)),
            _ => None,
        })
    }

    pub(super) fn infer_default_call_expected_type_from_in_progress_local_name(&self) -> Option<syn::Type> {
        let local_name = self.in_progress_local_initializers.last()?;
        if local_name == "span" || local_name.ends_with("_span") {
            for candidate in [
                "core::ops::Range<usize>",
                "std::ops::Range<usize>",
                "Range<usize>",
            ] {
                let Ok(candidate_ty) = syn::parse_str::<syn::Type>(candidate) else {
                    continue;
                };
                if self.type_contains_infer(&candidate_ty)
                    || self.type_contains_in_scope_type_param(&candidate_ty)
                    || self.type_contains_unresolved_placeholder_like(&candidate_ty)
                    || self.type_contains_unbound_single_letter_generic(&candidate_ty)
                {
                    continue;
                }
                return Some(candidate_ty);
            }
        }
        let mut candidates: Vec<String> = Vec::new();
        if let Some(pascal) = Self::snake_ident_to_pascal_case(local_name) {
            candidates.push(pascal);
        }
        candidates.sort();
        candidates.dedup();
        for candidate in candidates {
            if !self.simple_ident_is_known_type_name(&candidate) {
                continue;
            }
            if self.is_type_param_in_scope(&candidate) {
                continue;
            }
            let Ok(candidate_ty) = syn::parse_str::<syn::Type>(&candidate) else {
                continue;
            };
            if self.type_contains_infer(&candidate_ty)
                || self.type_contains_in_scope_type_param(&candidate_ty)
                || self.type_contains_unresolved_placeholder_like(&candidate_ty)
                || self.type_contains_unbound_single_letter_generic(&candidate_ty)
            {
                continue;
            }
            return Some(candidate_ty);
        }
        None
    }

    pub(super) fn infer_owner_template_args_from_declared_method_signature(
        &self,
        owner_path: Option<&syn::Path>,
        owner_name: &str,
        method_name: &str,
        call: &syn::ExprCall,
    ) -> Option<Vec<Option<String>>> {
        let type_params = if let Some(owner_path) = owner_path {
            // For qualified/associated calls, prefer owner-path keyed metadata and
            // avoid tail-only fallback that can cross-wire same-tail types.
            self.declared_owner_type_param_names_for_owner_path(owner_path, owner_name)?
        } else {
            self.declared_owner_type_param_names_by_tail(owner_name)?
        };
        if type_params.is_empty() {
            return None;
        }
        let mut inferred: Vec<Option<String>> = vec![None; type_params.len()];
        for (arg_idx, arg_expr) in call.args.iter().enumerate() {
            let Some(expected_arg_ty) = self.lookup_owner_method_arg_expected_type_from_owner_path(
                owner_path,
                owner_name,
                method_name,
                arg_idx,
                Some(arg_expr),
            ) else {
                continue;
            };
            let arg_cpp_ty = self
                .infer_hint_type_from_expr(arg_expr)
                .or_else(|| self.infer_simple_expr_type(arg_expr))
                .map(|ty| self.map_type(&ty))
                .filter(|mapped| {
                    mapped != "auto"
                        && !mapped.contains("/* TODO")
                        && !type_string_has_auto_placeholder(mapped)
                })
                .or_else(|| self.infer_remove_cvref_decltype_from_expr(arg_expr));
            let Some(arg_cpp_ty) = arg_cpp_ty else {
                continue;
            };
            if self.owner_template_arg_is_value_identifier(&arg_cpp_ty) {
                // Guard against leaking value identifiers (e.g. `ptr`) into owner
                // template arguments when omitted-owner recovery infers from call args.
                continue;
            }
            for (param_idx, param_name) in type_params.iter().enumerate() {
                if inferred[param_idx].is_none()
                    && self.expected_type_is_direct_type_param(&expected_arg_ty, param_name)
                {
                    inferred[param_idx] = Some(arg_cpp_ty.clone());
                }
            }
        }
        if inferred.iter().any(|entry| entry.is_some()) {
            Some(inferred)
        } else {
            None
        }
    }

    pub(super) fn infer_owner_template_args_for_call(
        &self,
        owner_path: Option<&syn::Path>,
        owner_name: &str,
        method_name: &str,
        call: &syn::ExprCall,
    ) -> Option<Vec<Option<String>>> {
        if owner_name.ends_with("Deserializer") && matches!(method_name, "new" | "new_") {
            let owner_path_is_unqualified = owner_path.is_some_and(|path| path.segments.len() == 1);
            let allow_unqualified_deserializer_tail_generic_recovery = matches!(
                owner_name,
                "BorrowedStrDeserializer"
                    | "BorrowedBytesDeserializer"
                    | "DatetimeDeserializer"
                    | "SpannedDeserializer"
            );
            let allow_tail_deserializer_recovery = !owner_path_is_unqualified
                || !owner_name.ends_with("Deserializer")
                || allow_unqualified_deserializer_tail_generic_recovery;
            let declared_owner_type_params = if let Some(path) = owner_path {
                let by_path = self.declared_owner_type_param_names_for_owner_path(path, owner_name);
                if by_path.is_some() {
                    by_path
                } else if allow_tail_deserializer_recovery {
                    self.declared_owner_type_param_names_by_tail(owner_name)
                } else {
                    None
                }
            } else {
                self.declared_owner_type_param_names_by_tail(owner_name)
            };
            let declared_owner_arity_by_tail = if allow_tail_deserializer_recovery {
                self.declared_type_param_arity_for_owner_tail(owner_name)
            } else {
                None
            };
            let explicit_owner_arg_arity = owner_path.and_then(|path| {
                path.segments.last().and_then(|seg| {
                    let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
                        return None;
                    };
                    Some(
                        args.args
                            .iter()
                            .filter(|arg| {
                                matches!(
                                    arg,
                                    syn::GenericArgument::Type(_) | syn::GenericArgument::Const(_)
                                )
                            })
                            .count(),
                    )
                })
            });

            let inferred_payload = call.args.first().and_then(|arg| {
                self.infer_hint_type_from_expr(arg)
                    .or_else(|| self.infer_simple_expr_type(arg))
                    .map(|ty| self.map_type(&ty))
                    .filter(|mapped| {
                        mapped != "auto"
                            && !mapped.contains("/* TODO")
                            && !type_string_has_auto_placeholder(mapped)
                    })
                    .or_else(|| self.infer_remove_cvref_decltype_from_expr(arg))
            });
            let inferred_error = self
                .current_return_type_hint()
                .and_then(|return_ty| self.expected_result_type_arg(Some(return_ty), 1))
                .map(|err_ty| {
                    self.map_type(err_ty)
                        .trim_start_matches("typename ")
                        .to_string()
                })
                .filter(|mapped| {
                    mapped != "auto"
                        && !mapped.contains("/* TODO")
                        && !type_string_has_auto_placeholder(mapped)
                })
                .or_else(|| self.is_type_param_in_scope("E").then_some("E".to_string()));
            let default_error_arg = inferred_error
                .clone()
                .or_else(|| self.is_type_param_in_scope("E").then_some("E".to_string()))
                .unwrap_or_else(|| "E".to_string());

            // Expanded serde frequently emits `Type::<_>::new_(...)` for these
            // constructors. In those single-slot forms, the owner slot is the
            // error type parameter, not payload.
            if explicit_owner_arg_arity == Some(1) && !owner_name.contains("Access") {
                return Some(vec![Some(default_error_arg.clone())]);
            }
            if owner_path_is_unqualified
                && self.is_type_param_in_scope("E")
                && matches!(
                    owner_name,
                    "SeqDeserializer"
                        | "MapDeserializer"
                        | "SeqRefDeserializer"
                        | "MapRefDeserializer"
                        | "ContentDeserializer"
                        | "ContentRefDeserializer"
                        | "EnumDeserializer"
                        | "VariantDeserializer"
                )
                && declared_owner_type_params
                    .as_ref()
                    .is_some_and(|params| params.len() == 1)
            {
                let err = inferred_error.clone().unwrap_or_else(|| "E".to_string());
                return Some(vec![Some(err)]);
            }

            if declared_owner_type_params.is_none()
                && let Some(owner_arity) = declared_owner_arity_by_tail
                && owner_arity > 0
                && !owner_name.contains("Access")
            {
                if owner_arity == 1 {
                    return Some(vec![Some(default_error_arg.clone())]);
                }
                let mut inferred = vec![None; owner_arity];
                if let Some(payload) = inferred_payload.clone() {
                    inferred[0] = Some(payload);
                }
                inferred[owner_arity - 1] = Some(default_error_arg.clone());
                if inferred.iter().any(|arg| arg.is_some()) {
                    return Some(inferred);
                }
            }

            // Access deserializer helpers are payload-bound (`A`) and should
            // recover their owner argument from the constructor payload.
            if owner_name.contains("Access") {
                if let Some(inferred_payload) = inferred_payload {
                    return Some(vec![Some(inferred_payload)]);
                }
                return None;
            }

            let owner_type_params = declared_owner_type_params.or_else(|| {
                if matches!(
                    owner_name,
                    "SeqDeserializer"
                        | "MapDeserializer"
                        | "SeqRefDeserializer"
                        | "MapRefDeserializer"
                ) {
                    if owner_path_is_unqualified
                        && self.is_type_param_in_scope("E")
                        && !self.is_type_param_in_scope("I")
                    {
                        Some(vec!["E".to_string()])
                    } else {
                        Some(vec!["I".to_string(), "E".to_string()])
                    }
                } else if matches!(
                    owner_name,
                    "ContentDeserializer"
                        | "ContentRefDeserializer"
                        | "EnumDeserializer"
                        | "VariantDeserializer"
                ) {
                    Some(vec!["E".to_string()])
                } else {
                    None
                }
            });
            if let Some(owner_type_params) = owner_type_params {
                let mut inferred = vec![None; owner_type_params.len()];

                if let Some(from_signature) = self
                    .infer_owner_template_args_from_declared_method_signature(
                        owner_path,
                        owner_name,
                        method_name,
                        call,
                    )
                {
                    for (idx, inferred_arg) in from_signature.into_iter().enumerate() {
                        if idx < inferred.len() && inferred[idx].is_none() {
                            inferred[idx] = inferred_arg;
                        }
                    }
                }

                let default_error_arg = Some(default_error_arg.clone());

                for (idx, param) in owner_type_params.iter().enumerate() {
                    if param == "E" && inferred[idx].is_none() {
                        inferred[idx] = default_error_arg.clone().or_else(|| {
                            self.is_type_param_in_scope(param).then_some(param.clone())
                        });
                    }
                }

                if matches!(
                    owner_name,
                    "SeqDeserializer"
                        | "MapDeserializer"
                        | "SeqRefDeserializer"
                        | "MapRefDeserializer"
                ) && owner_type_params.len() > 1
                    && !inferred.is_empty()
                    && inferred[0].is_none()
                    && owner_type_params.first().is_some_and(|param| param != "E")
                {
                    inferred[0] = inferred_payload.clone();
                }

                // Content/enum deserializer constructors in serde commonly expose
                // a single owner parameter that binds the error type. If signature
                // inference could not recover a concrete owner arg, prefer the
                // in-scope error slot instead of constructor payload type.
                if owner_type_params.len() == 1
                    && !owner_name.contains("Access")
                    && inferred.first().is_some_and(|arg| arg.is_none())
                {
                    inferred[0] = default_error_arg;
                }

                if inferred.iter().any(|arg| arg.is_some()) {
                    return Some(inferred);
                }
            }
            return None;
        }

        match owner_name {
            "ArrayVec" => {
                let mut inferred = vec![None, None];
                match method_name {
                    "from" | "try_from" => {
                        if let Some(arg) = call.args.first() {
                            inferred[0] = self
                                .infer_array_element_type_from_expr(arg)
                                .map(|ty| self.map_type(&ty));
                            inferred[1] = self.infer_array_capacity_arg_for_expr(arg);
                        }
                    }
                    "from_iter" => {
                        if let Some(arg) = call.args.first() {
                            inferred[0] = self
                                .infer_iter_item_type_from_expr(arg)
                                .map(|ty| self.map_type(&ty));
                        }
                    }
                    _ => {}
                }
                if inferred.iter().any(|arg| arg.is_some()) {
                    Some(inferred)
                } else {
                    None
                }
            }
            "ArrayBuilder" => {
                if matches!(method_name, "new" | "new_") && call.args.is_empty() {
                    let mut elem: Option<String> = None;
                    if let Some(local_name) = self.in_progress_local_initializers.last()
                        && let Some(hint_ty) = self.lookup_local_placeholder_type_hint(local_name)
                    {
                        let normalized = normalize_placeholder_hint_for_owner(
                            Some("ArrayBuilder"),
                            hint_ty.clone(),
                        );
                        let elem_hint = extract_array_element_type_for_hint(&normalized)
                            .or_else(|| extract_sequence_element_type_for_hint(&normalized))
                            .unwrap_or(normalized);
                        let mapped = self.map_type(&elem_hint);
                        if mapped != "auto"
                            && !mapped.contains("/* TODO")
                            && !type_string_has_auto_placeholder(&mapped)
                        {
                            elem = Some(mapped);
                        }
                    }
                    let mut cap: Option<String> = None;
                    if let Some(ret_ty) = self.current_return_type_hint()
                        && let Some(inner_ty) = self.expected_option_type_arg(Some(ret_ty))
                    {
                        let inner_ty = self.peel_reference_paren_group_type(inner_ty);
                        if let syn::Type::Array(array_ty) = inner_ty {
                            if elem.is_none() {
                                let mapped = self.map_type(array_ty.elem.as_ref());
                                if mapped != "auto"
                                    && !mapped.contains("/* TODO")
                                    && !type_string_has_auto_placeholder(&mapped)
                                {
                                    elem = Some(mapped);
                                }
                            }
                            cap = Some(self.emit_expr_to_string(&array_ty.len));
                        }
                    }
                    if cap.is_none() {
                        cap = ["N", "K", "CAP"]
                            .into_iter()
                            .find(|name| self.is_type_param_in_scope(name))
                            .map(|name| name.to_string());
                    }
                    if elem.is_some() || cap.is_some() {
                        return Some(vec![elem, cap]);
                    }
                }
                None
            }
            "Cell" => {
                if matches!(method_name, "new" | "new_") {
                    let inferred = call
                        .args
                        .first()
                        .and_then(|arg| self.infer_hint_type_from_expr(arg))
                        .map(|ty| self.map_type(&ty));
                    if inferred.is_some() {
                        Some(vec![inferred])
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "ArrayString" => {
                let mut inferred = vec![None];
                if matches!(method_name, "from" | "try_from" | "from_byte_string") {
                    if let Some(arg) = call.args.first() {
                        inferred[0] = self.infer_array_capacity_arg_for_expr(arg);
                    }
                }
                if inferred.iter().any(|arg| arg.is_some()) {
                    Some(inferred)
                } else {
                    None
                }
            }
            "NonNull" => {
                if matches!(method_name, "new" | "new_" | "new_unchecked" | "from") {
                    let inferred_from_type = call
                        .args
                        .first()
                        .and_then(|arg| self.infer_hint_type_from_expr(arg))
                        .and_then(
                            |arg_ty| match self.peel_reference_paren_group_type(&arg_ty) {
                                syn::Type::Ptr(ptr) => Some(self.map_type(&ptr.elem)),
                                _ => None,
                            },
                        );
                    let inferred_from_decltype = call.args.first().map(|arg| {
                        let arg_cpp = self.emit_expr_to_string(arg);
                        format!(
                            "std::remove_pointer_t<std::remove_reference_t<decltype(({}))>>",
                            arg_cpp
                        )
                    });
                    let inferred = match (inferred_from_type, inferred_from_decltype) {
                        (Some(from_type), Some(from_decltype))
                            if from_type == "auto"
                                || from_type.contains("/* TODO")
                                || self.owner_template_arg_is_value_identifier(&from_type)
                                || (from_type
                                    .chars()
                                    .next()
                                    .is_some_and(|c| c.is_ascii_uppercase())
                                    && from_type
                                        .chars()
                                        .all(|c| c.is_ascii_alphanumeric() || c == '_')) =>
                        {
                            Some(from_decltype)
                        }
                        (Some(from_type), _) => Some(from_type),
                        (None, from_decltype) => from_decltype,
                    };
                    if let Some(inferred) = inferred {
                        Some(vec![Some(inferred)])
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "Box" => {
                if method_name == "from_raw" {
                    let inferred_from_type = call
                        .args
                        .first()
                        .and_then(|arg| self.infer_hint_type_from_expr(arg))
                        .and_then(
                            |arg_ty| match self.peel_reference_paren_group_type(&arg_ty) {
                                syn::Type::Ptr(ptr) => Some(self.map_type(&ptr.elem)),
                                _ => None,
                            },
                        );
                    let inferred_from_decltype = call.args.first().map(|arg| {
                        let arg_cpp = self.emit_expr_to_string(arg);
                        format!(
                            "std::remove_pointer_t<std::remove_reference_t<decltype(({}))>>",
                            arg_cpp
                        )
                    });
                    let inferred = match (inferred_from_type, inferred_from_decltype) {
                        (Some(from_type), Some(from_decltype))
                            if from_type == "auto"
                                || from_type.contains("/* TODO")
                                || self.owner_template_arg_is_value_identifier(&from_type)
                                || (from_type
                                    .chars()
                                    .next()
                                    .is_some_and(|c| c.is_ascii_uppercase())
                                    && from_type
                                        .chars()
                                        .all(|c| c.is_ascii_alphanumeric() || c == '_')) =>
                        {
                            Some(from_decltype)
                        }
                        (Some(from_type), _) => Some(from_type),
                        (None, from_decltype) => from_decltype,
                    };
                    inferred.map(|inner| vec![Some(inner)])
                } else if matches!(method_name, "into_raw" | "leak") {
                    // `Box::leak(b)` and `Box::into_raw(b)` both consume a Box
                    // and yield the raw pointer. Infer T from the argument's
                    // declared `Box<T, ...>` type so we emit
                    // `rusty::Box<T>::leak(std::move(b))` rather than the
                    // syntactically invalid `rusty::Box<auto>::leak(...)`.
                    let inferred = call
                        .args
                        .first()
                        .and_then(|arg| self.infer_hint_type_from_expr(arg))
                        .and_then(|arg_ty| {
                            let arg_ty = self.peel_reference_paren_group_type(&arg_ty);
                            let syn::Type::Path(tp) = arg_ty else {
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
                                syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                                _ => None,
                            })
                        })
                        .or_else(|| {
                            call.args
                                .first()
                                .and_then(|arg| self.infer_hint_type_from_expr(arg))
                                .map(|arg_ty| {
                                    let arg_ty = self.peel_reference_paren_group_type(&arg_ty);
                                    self.map_type(arg_ty)
                                })
                                .filter(|mapped| mapped != "auto" && !mapped.contains("/* TODO"))
                        });
                    inferred.map(|inner| vec![Some(inner)])
                } else if matches!(
                    method_name,
                    "new"
                        | "new_"
                        | "make"
                        | "try_new"
                        | "try_new_in"
                        | "new_in"
                        | "try_new_uninit_in"
                        | "try_new_zeroed_in"
                        | "new_uninit_in"
                        | "new_zeroed_in"
                ) {
                    // Infer template arg from the argument expression (e.g.
                    // `Box::new_(92)` → T=i32, `Box::try_new(ArcInner<T>{...})`
                    // → T=ArcInner<T>, `Box::new_in(val, alloc)` → T from
                    // `val`'s type). All of these factories take the value-to-
                    // wrap as their first argument; the Rust call site relies
                    // on type inference to fill in `Box<T>`, but C++ rejects
                    // `Box<auto>` in template-argument position, so we must
                    // recover a concrete type here.
                    let inferred = call
                        .args
                        .first()
                        .and_then(|arg| {
                            self.infer_hint_type_from_expr(arg)
                                .or_else(|| self.infer_local_binding_type_from_initializer(arg))
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
                    if let Some(inferred) = inferred {
                        Some(vec![Some(inferred)])
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "Rc" | "Arc" | "Mutex" | "RwLock" => {
                if let Some(inferred_owner) = call
                    .args
                    .first()
                    .and_then(|arg| self.infer_owner_first_type_arg_from_expr(owner_name, arg))
                    && matches!(
                        method_name,
                        "clone"
                            | "ptr_eq"
                            | "strong_count"
                            | "weak_count"
                            | "get_mut"
                            | "make_mut"
                            | "try_unwrap"
                            | "unwrap_or_clone"
                            | "lock"
                            | "read"
                            | "write"
                    )
                {
                    return Some(vec![Some(inferred_owner)]);
                }
                if matches!(method_name, "new" | "new_" | "make") {
                    let inferred = call
                        .args
                        .first()
                        .and_then(|arg| {
                            self.infer_hint_type_from_expr(arg)
                                .or_else(|| self.infer_local_binding_type_from_initializer(arg))
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
                    if let Some(inferred) = inferred {
                        return Some(vec![Some(inferred)]);
                    }
                }
                None
            }
            "MaybeUninit" => {
                if matches!(method_name, "new" | "new_") {
                    let inferred = call
                        .args
                        .first()
                        .and_then(|arg| {
                            self.infer_hint_type_from_expr(arg)
                                .or_else(|| self.infer_simple_expr_type(arg))
                        })
                        .map(|ty| self.map_type(&ty))
                        .or_else(|| {
                            call.args
                                .first()
                                .and_then(|arg| self.infer_remove_cvref_decltype_from_expr(arg))
                        });
                    inferred.map(|inner| vec![Some(inner)])
                } else if matches!(
                    method_name,
                    "slice_assume_init_ref" | "slice_assume_init_mut"
                ) {
                    // `MaybeUninit::slice_assume_init_ref(&[MaybeUninit<T>]) -> &[T]`
                    // (and the `_mut` analogue). Otherwise the call would emit
                    // `rusty::MaybeUninit<auto>::slice_assume_init_ref(...)` —
                    // invalid because `auto` cannot appear as a template
                    // argument.
                    //
                    // First try arg-type inference: peel `&[MaybeUninit<T>]`
                    // from the argument's syn-type to recover T directly.
                    let from_args = call
                        .args
                        .first()
                        .and_then(|arg| self.infer_hint_type_from_expr(arg))
                        .and_then(|arg_ty| {
                            let inner = self.peel_reference_paren_group_type(&arg_ty);
                            let elem = match inner {
                                syn::Type::Slice(s) => &*s.elem,
                                syn::Type::Array(a) => &*a.elem,
                                other => other,
                            };
                            let syn::Type::Path(tp) = elem else {
                                return None;
                            };
                            let last = tp.path.segments.last()?;
                            if last.ident != "MaybeUninit" {
                                return None;
                            }
                            let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
                                return None;
                            };
                            args.args.iter().find_map(|arg| match arg {
                                syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                                _ => None,
                            })
                        });
                    if from_args.is_some() {
                        return from_args.map(|inner| vec![Some(inner)]);
                    }

                    // Bidirectional fallback: when the argument is a call
                    // expression (e.g. `slice_to(arr, n)`), syn-level
                    // type inference can't recover the slice's element
                    // type. Invert the method's known return-shape
                    // (`&[T]` or `&mut [T]`) against the enclosing
                    // function's return type to deduce T.
                    if let Some(ret_hint) = self.current_return_type_hint() {
                        let inner = self.peel_reference_paren_group_type(ret_hint);
                        let elem = match inner {
                            syn::Type::Slice(s) => &*s.elem,
                            syn::Type::Array(a) => &*a.elem,
                            other => other,
                        };
                        let mapped = self.map_type(elem);
                        if mapped != "auto"
                            && !mapped.contains("/* TODO")
                            && !type_string_has_auto_placeholder(&mapped)
                            && !type_string_contains_auto_template_arg(&mapped)
                        {
                            return Some(vec![Some(mapped)]);
                        }
                    }
                    None
                } else {
                    None
                }
            }
            "OnceCell" => {
                if let Some(inferred_owner) = call
                    .args
                    .first()
                    .and_then(|arg| self.infer_owner_first_type_arg_from_expr("OnceCell", arg))
                    && matches!(
                        method_name,
                        "get"
                            | "get_mut"
                            | "set"
                            | "get_or_init"
                            | "get_or_try_init"
                            | "take"
                            | "into_inner"
                            | "wait"
                            | "try_insert"
                            | "from_mut"
                            | "from_mut_ptr"
                    )
                {
                    return Some(vec![Some(inferred_owner)]);
                }
                if matches!(method_name, "from" | "with_value") {
                    let inferred = call
                        .args
                        .first()
                        .and_then(|arg| {
                            self.infer_hint_type_from_expr(arg)
                                .or_else(|| self.infer_simple_expr_type(arg))
                        })
                        .map(|ty| self.map_type(&ty))
                        .or_else(|| {
                            call.args
                                .first()
                                .and_then(|arg| self.infer_remove_cvref_decltype_from_expr(arg))
                        });
                    if let Some(inferred) = inferred
                        && inferred != "auto"
                        && !inferred.contains("/* TODO")
                        && !type_string_has_auto_placeholder(&inferred)
                    {
                        Some(vec![Some(inferred)])
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "OnceBox" => {
                if matches!(method_name, "new" | "new_") && call.args.is_empty() {
                    if let Some(local_name) = self.in_progress_local_initializers.last()
                        && let Some(hint_ty) = self.lookup_local_placeholder_type_hint(local_name)
                    {
                        let hint_ty = self.peel_reference_paren_group_type(hint_ty);
                        let inferred_from_hint = if let syn::Type::Path(tp) = hint_ty {
                            if let Some(last) = tp.path.segments.last() {
                                if (last.ident == "Box" || last.ident == "OnceBox")
                                    && let syn::PathArguments::AngleBracketed(args) =
                                        &last.arguments
                                    && let Some(syn::GenericArgument::Type(inner_ty)) = args
                                        .args
                                        .iter()
                                        .find(|arg| matches!(arg, syn::GenericArgument::Type(_)))
                                {
                                    Some(self.map_type(inner_ty))
                                } else {
                                    Some(self.map_type(hint_ty))
                                }
                            } else {
                                None
                            }
                        } else {
                            Some(self.map_type(hint_ty))
                        };
                        if let Some(inferred) = inferred_from_hint
                            && inferred != "auto"
                            && !inferred.contains("/* TODO")
                            && !type_string_has_auto_placeholder(&inferred)
                        {
                            return Some(vec![Some(inferred)]);
                        }
                    }
                    if self.current_struct.as_deref() == Some("OnceBox") {
                        let scoped_inner =
                            self.declared_type_params
                                .iter()
                                .find_map(|(type_key, params)| {
                                    if type_key
                                        .rsplit("::")
                                        .next()
                                        .is_some_and(|tail| tail == "OnceBox")
                                        && !params.is_empty()
                                    {
                                        Some(params[0].clone())
                                    } else {
                                        None
                                    }
                                });
                        if let Some(inner) = scoped_inner {
                            return Some(vec![Some(inner)]);
                        }
                    }
                }
                if let Some(inferred_owner) = call
                    .args
                    .first()
                    .and_then(|arg| self.infer_owner_first_type_arg_from_expr("OnceBox", arg))
                    && matches!(
                        method_name,
                        "get" | "get_mut" | "set" | "get_or_init" | "get_or_try_init" | "take"
                    )
                {
                    return Some(vec![Some(inferred_owner)]);
                }
                if matches!(method_name, "from" | "with_value") {
                    let inferred = call
                        .args
                        .first()
                        .and_then(|arg| {
                            self.infer_hint_type_from_expr(arg)
                                .or_else(|| self.infer_simple_expr_type(arg))
                                .and_then(|arg_ty| {
                                    let arg_ty = self.peel_reference_paren_group_type(&arg_ty);
                                    let syn::Type::Path(tp) = arg_ty else {
                                        return None;
                                    };
                                    let last = tp.path.segments.last()?;
                                    if last.ident != "Box" {
                                        return None;
                                    }
                                    let syn::PathArguments::AngleBracketed(args) = &last.arguments
                                    else {
                                        return None;
                                    };
                                    args.args.iter().find_map(|arg| match arg {
                                        syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                                        _ => None,
                                    })
                                })
                        })
                        .or_else(|| {
                            if self.current_struct.as_deref() != Some("OnceBox") {
                                return None;
                            }
                            self.declared_type_params
                                .iter()
                                .find_map(|(type_key, params)| {
                                    if type_key
                                        .rsplit("::")
                                        .next()
                                        .is_some_and(|tail| tail == "OnceBox")
                                        && !params.is_empty()
                                    {
                                        Some(params[0].clone())
                                    } else {
                                        None
                                    }
                                })
                        })
                        .or_else(|| {
                            call.args
                                .first()
                                .and_then(|arg| {
                                    self.infer_hint_type_from_expr(arg)
                                        .or_else(|| self.infer_simple_expr_type(arg))
                                })
                                .map(|ty| self.map_type(&ty))
                        })
                        .or_else(|| {
                            call.args
                                .first()
                                .and_then(|arg| self.infer_remove_cvref_decltype_from_expr(arg))
                        });
                    if let Some(inferred) = inferred {
                        Some(vec![Some(inferred)])
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "Lazy" => {
                if let Some(inferred_owner) = call
                    .args
                    .first()
                    .and_then(|arg| self.infer_owner_first_type_arg_from_expr("Lazy", arg))
                    && matches!(
                        method_name,
                        "get" | "get_mut" | "force" | "force_mut" | "into_value"
                    )
                {
                    return Some(vec![Some(inferred_owner)]);
                }
                if matches!(method_name, "new" | "new_") {
                    let inferred = call
                        .args
                        .first()
                        .and_then(|arg| {
                            self.infer_closure_return_type(arg)
                                .or_else(|| self.infer_hint_type_from_expr(arg))
                        })
                        .map(|ty| self.map_type(&ty))
                        .or_else(|| {
                            call.args.first().map(|arg| {
                                let arg_cpp = self.emit_expr_to_string(arg);
                                format!(
                                    "std::invoke_result_t<std::remove_cvref_t<decltype(({}))>&>",
                                    arg_cpp
                                )
                            })
                        });
                    if let Some(inner) = inferred {
                        // Keep only value type `T`; `F` defaults to `rusty::SafeFn<T()>`.
                        Some(vec![Some(inner)])
                    } else {
                        None
                    }
                } else if method_name == "into_value" {
                    let arg_ty = call.args.first().and_then(|arg| {
                        self.infer_hint_type_from_expr(arg)
                            .or_else(|| self.infer_simple_expr_type(arg))
                    })?;
                    let arg_ty = self.peel_reference_paren_group_type(&arg_ty);
                    let syn::Type::Path(tp) = arg_ty else {
                        return None;
                    };
                    let last = tp.path.segments.last()?;
                    if last.ident != "Lazy" {
                        return None;
                    }
                    let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
                        return None;
                    };
                    let type_args: Vec<String> = args
                        .args
                        .iter()
                        .filter_map(|arg| match arg {
                            syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                            _ => None,
                        })
                        .collect();
                    if type_args.is_empty() {
                        None
                    } else {
                        Some(type_args.into_iter().map(Some).collect())
                    }
                } else {
                    None
                }
            }
            "Spanned" => {
                if matches!(method_name, "new" | "new_") {
                    let inferred = call
                        .args
                        .get(1)
                        .and_then(|arg| self.infer_remove_cvref_decltype_from_expr(arg))
                        .or_else(|| {
                            call.args.get(1).and_then(|arg| {
                                self.infer_hint_type_from_expr(arg)
                                    .or_else(|| self.infer_simple_expr_type(arg))
                                    .map(|ty| self.map_type(&ty))
                                    .filter(|mapped| {
                                        mapped != "auto"
                                            && !mapped.contains("/* TODO")
                                            && !type_string_has_auto_placeholder(mapped)
                                    })
                            })
                        })
                        .or_else(|| {
                            call.args
                                .get(1)
                                .and_then(|arg| self.infer_remove_cvref_decltype_from_expr(arg))
                        });
                    inferred.map(|inner| vec![Some(inner)])
                } else {
                    None
                }
            }
            "Vec" => {
                if matches!(method_name, "new" | "new_" | "with_capacity") && call.args.len() <= 1 {
                    if let Some(local_name) = self.in_progress_local_initializers.last() {
                        if let Some(hint_ty) = self.lookup_local_placeholder_type_hint(local_name) {
                            let normalized =
                                normalize_placeholder_hint_for_owner(Some("Vec"), hint_ty.clone());
                            let elem_hint = extract_vec_element_type_for_hint(&normalized)
                                .or_else(|| extract_sequence_element_type_for_hint(&normalized))
                                .unwrap_or(normalized);
                            let mapped = self.map_type(&elem_hint);
                            if mapped != "auto"
                                && !mapped.contains("/* TODO")
                                && !type_string_has_auto_placeholder(&mapped)
                            {
                                return Some(vec![Some(mapped)]);
                            }
                        }
                        // Common serde visitors allocate byte buffers through
                        // `Vec::with_capacity` and only reveal the element type
                        // through later `next_element`/`push` flow. Keep omitted
                        // owner recovery concrete for these byte-oriented locals.
                        let local_lower = local_name.to_ascii_lowercase();
                        if local_lower.contains("byte") {
                            return Some(vec![Some("uint8_t".to_string())]);
                        }
                    }
                    if method_name == "with_capacity"
                        && let Some(return_hint) = self.current_return_type_hint()
                    {
                        let vec_hint = self
                            .expected_result_type_arg(Some(return_hint), 0)
                            .unwrap_or(return_hint);
                        if let Some(elem_hint) = extract_vec_element_type_for_hint(vec_hint) {
                            let mapped = self.map_type(&elem_hint);
                            if mapped != "auto"
                                && !mapped.contains("/* TODO")
                                && !type_string_has_auto_placeholder(&mapped)
                            {
                                return Some(vec![Some(mapped)]);
                            }
                        }
                    }
                }
                if matches!(method_name, "from_raw_parts" | "from_raw_parts_in") {
                    let inferred_from_type = call
                        .args
                        .first()
                        .and_then(|arg| self.infer_hint_type_from_expr(arg))
                        .and_then(
                            |arg_ty| match self.peel_reference_paren_group_type(&arg_ty) {
                                syn::Type::Ptr(ptr) => Some(self.map_type(&ptr.elem)),
                                _ => None,
                            },
                        );
                    let inferred_from_decltype = call.args.first().map(|arg| {
                        let arg_cpp = self.emit_expr_to_string(arg);
                        format!(
                            "std::remove_pointer_t<std::remove_reference_t<decltype(({}))>>",
                            arg_cpp
                        )
                    });
                    let inferred = match (inferred_from_type, inferred_from_decltype) {
                        (Some(from_type), Some(from_decltype))
                            if from_type == "auto"
                                || from_type.contains("/* TODO")
                                || self.owner_template_arg_is_value_identifier(&from_type)
                                || (from_type
                                    .chars()
                                    .next()
                                    .is_some_and(|c| c.is_ascii_uppercase())
                                    && from_type
                                        .chars()
                                        .all(|c| c.is_ascii_alphanumeric() || c == '_')) =>
                        {
                            Some(from_decltype)
                        }
                        (Some(from_type), _) => Some(from_type),
                        (None, from_decltype) => from_decltype,
                    };
                    if let Some(inferred) = inferred {
                        Some(vec![Some(inferred)])
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "Map" => {
                if matches!(method_name, "new" | "new_" | "with_capacity") && call.args.len() <= 1 {
                    if let Some(local_name) = self.in_progress_local_initializers.last()
                        && let Some(hint_ty) = self.lookup_local_placeholder_type_hint(local_name)
                    {
                        if let Some((key_ty, value_ty)) =
                            extract_map_key_value_types_for_hint(hint_ty)
                        {
                            let key = self.map_type(&key_ty);
                            let value = self.map_type(&value_ty);
                            if key != "auto"
                                && value != "auto"
                                && !key.contains("/* TODO")
                                && !value.contains("/* TODO")
                                && !type_string_has_auto_placeholder(&key)
                                && !type_string_has_auto_placeholder(&value)
                            {
                                return Some(vec![Some(key), Some(value)]);
                            }
                        }
                    }
                    return Some(vec![
                        Some("rusty::String".to_string()),
                        Some("value::Value".to_string()),
                    ]);
                }
                None
            }
            "ByteArray" => {
                if matches!(method_name, "new" | "new_" | "from" | "try_from")
                    && let Some(arg) = call.args.first()
                    && let Some(cap) = self.infer_array_capacity_arg_for_expr(arg)
                {
                    return Some(vec![Some(cap)]);
                }
                None
            }
            _ => self.infer_owner_template_args_from_declared_method_signature(
                owner_path,
                owner_name,
                method_name,
                call,
            ),
        }
    }

    pub(super) fn recover_single_segment_constructor_type_args_from_call(
        &self,
        path: &syn::Path,
        call: &syn::ExprCall,
    ) -> Option<String> {
        if path.segments.len() != 1 {
            return None;
        }
        let seg = path.segments.first()?;
        if !matches!(seg.arguments, syn::PathArguments::None) {
            return None;
        }

        let type_name = seg.ident.to_string();
        if self.is_local_function_name_in_scope(&type_name)
            || self.module_qualified_functions.contains_key(&type_name)
        {
            return None;
        }

        let type_key = self
            .declared_type_key_for_path(path)
            .or_else(|| self.lookup_declared_type_key_for_base(&type_name, &type_name))?;
        let params = self.declared_type_params.get(&type_key)?;
        if params.len() != 1 {
            return None;
        }

        let arg0 = call.args.first()?;
        let arg0_cpp = self.emit_expr_to_string(arg0);
        let arg0_is_raw_pointer = self.is_expr_raw_pointer_like(arg0);
        if !arg0_is_raw_pointer {
            return None;
        }
        let deduced_arg_cpp = format!(
            "std::remove_pointer_t<std::remove_cvref_t<decltype(({}))>>",
            arg0_cpp
        );
        Some(format!(
            "{}<{}>",
            escape_cpp_keyword(&type_name),
            deduced_arg_cpp
        ))
    }

    pub(super) fn resolve_try_into_target_type(
        &self,
        expected_ty: &syn::Type,
        receiver: &syn::Expr,
    ) -> Option<syn::Type> {
        let mut target = self
            .expected_result_type_arg(Some(expected_ty), 0)
            .cloned()
            .unwrap_or_else(|| expected_ty.clone());
        if self.type_contains_infer(&target) {
            if let Some(elem_ty) = self.infer_array_element_type_from_expr(receiver) {
                target = self.substitute_owner_infer_with_hint(&target, &["ArrayVec"], &elem_ty);
            }
        }
        if self.type_contains_infer(&target) {
            return None;
        }
        while let syn::Type::Reference(reference) = self.peel_paren_group_type(&target) {
            target = (*reference.elem).clone();
        }
        Some(target)
    }

    pub(super) fn resolve_expected_type_with_iter_hint(
        &self,
        expected_ty: &syn::Type,
        iter_expr: &syn::Expr,
    ) -> syn::Type {
        if !self.type_contains_infer(expected_ty) {
            return expected_ty.clone();
        }
        if let Some(item_ty) = self.infer_iter_item_type_from_expr(iter_expr) {
            return self.substitute_owner_infer_with_hint(
                expected_ty,
                &["ArrayVec", "Vec"],
                &item_ty,
            );
        }
        expected_ty.clone()
    }

    pub(super) fn infer_into_iter_receiver_expected_type_from_call_expected(
        &self,
        receiver: &syn::Expr,
        call_expected_ty: Option<&syn::Type>,
    ) -> Option<syn::Type> {
        let call_expected_ty = call_expected_ty?;
        let item_ty = self.extract_iter_item_type_from_type(call_expected_ty)?;
        let receiver = self.peel_paren_group_expr(receiver);
        let syn::Expr::Call(call) = receiver else {
            return None;
        };
        let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(call.func.as_ref()) else {
            return None;
        };
        // The `vec![..]` macro lowers (via `cargo expand`) to a slice/boxed
        // constructor — `<[_]>::into_vec(..)` or
        // `::alloc::boxed::box_assume_init_into_vec_unsafe(..)` — that yields a
        // `Vec<T>`. Typing its `.into_iter()` receiver as `Vec<item>` lets the
        // element type flow into the constructed elements; otherwise the
        // receiver falls back to the function's `IntoIter<T>` return type and
        // `vec![Ok(..), Err(..)]` elements wrongly qualify as
        // `IntoIter<Result<..>>::Ok(..)`.
        if path_expr
            .path
            .segments
            .last()
            .is_some_and(|seg| seg.ident.to_string().contains("into_vec"))
        {
            return Some(parse_quote!(Vec<#item_ty>));
        }
        if path_expr.path.segments.len() < 2 {
            return None;
        }
        let owner = path_expr
            .path
            .segments
            .iter()
            .nth_back(1)?
            .ident
            .to_string();
        match owner.as_str() {
            "Vec" => Some(parse_quote!(Vec<#item_ty>)),
            "VecDeque" => Some(parse_quote!(VecDeque<#item_ty>)),
            _ => None,
        }
    }

    pub(super) fn infer_fold_like_init_expected_type_from_call_context(
        &self,
        call: &syn::ExprCall,
        arg_idx: usize,
        call_expected_ty: Option<&syn::Type>,
    ) -> Option<syn::Type> {
        if arg_idx != 1 || call.args.len() < 2 {
            return None;
        }
        let init_expr = self.peel_paren_group_expr(&call.args[1]);
        if !Self::is_unsuffixed_int_literal_expr(init_expr) {
            return None;
        }
        let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(call.func.as_ref()) else {
            return None;
        };
        let mut is_fold_like = false;
        let mut is_try_fold_like = false;
        for candidate in self.call_path_candidates(&path_expr.path) {
            let mapped = types::map_function_path(&candidate).unwrap_or(candidate.as_str());
            if matches!(mapped, "fold" | "rusty::fold") {
                is_fold_like = true;
            }
            if matches!(mapped, "try_fold" | "rusty::try_fold") {
                is_fold_like = true;
                is_try_fold_like = true;
            }
        }
        if !is_fold_like {
            return None;
        }
        let contextual_expected_ty = call_expected_ty.or(self.current_return_type_hint());

        let from_call_expected = if is_try_fold_like {
            contextual_expected_ty
                .and_then(|ty| self.expected_option_type_arg(Some(ty)).cloned())
                .or_else(|| {
                    contextual_expected_ty
                        .and_then(|ty| self.expected_result_type_arg(Some(ty), 0).cloned())
                })
        } else {
            contextual_expected_ty.cloned()
        };
        if let Some(from_call_expected) = from_call_expected
            && self.type_is_concrete_hint_candidate(&from_call_expected)
        {
            return Some(from_call_expected);
        }

        if call.args.len() >= 3
            && let Some(from_reducer_acc) = self.lookup_function_arg_expected_type(&call.args[2], 0)
            && self.type_is_concrete_hint_candidate(from_reducer_acc)
        {
            return Some(from_reducer_acc.clone());
        }

        if let Some(iter_item_ty) = self.infer_iter_item_type_from_expr(&call.args[0])
            && self.type_is_concrete_hint_candidate(&iter_item_ty)
            && is_numeric_cpp_scalar_type(self.map_type(&iter_item_ty).trim())
        {
            return Some(iter_item_ty);
        }
        if let Some(counting_hint) =
            self.infer_fold_like_counting_init_hint_from_iter_expr(&call.args[0])
        {
            return Some(counting_hint);
        }
        None
    }

    pub(super) fn infer_fold_like_counting_init_hint_from_iter_expr(
        &self,
        iter_expr: &syn::Expr,
    ) -> Option<syn::Type> {
        let iter_expr = self.peel_paren_group_expr(iter_expr);
        match iter_expr {
            syn::Expr::MethodCall(mc) if mc.method == "enumerate" => Some(parse_quote!(size_t)),
            syn::Expr::Call(call) => {
                let syn::Expr::Path(path_expr) = self.peel_paren_group_expr(call.func.as_ref())
                else {
                    return None;
                };
                let is_counting_iter =
                    self.call_path_candidates(&path_expr.path)
                        .iter()
                        .any(|candidate| {
                            let mapped = types::map_function_path(candidate).unwrap_or(candidate);
                            matches!(
                                mapped,
                                "enumerate"
                                    | "rusty::enumerate"
                                    | "range"
                                    | "rusty::range"
                                    | "range_inclusive"
                                    | "rusty::range_inclusive"
                            )
                        });
                is_counting_iter.then_some(parse_quote!(size_t))
            }
            _ => None,
        }
    }

    pub(super) fn resolve_cpp_import_bound_symbol_for_path(
        &self,
        path: &syn::Path,
    ) -> Option<(String, String)> {
        if path.segments.len() < 2 {
            return None;
        }
        let binding = path.segments.first()?.ident.to_string();
        let module_path = self.cpp_module_import_bindings.get(&binding)?.clone();
        let symbol_name = path
            .segments
            .iter()
            .skip(1)
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if symbol_name.is_empty() {
            return None;
        }
        Some((module_path, symbol_name))
    }

    pub(super) fn infer_data_enum_owner_type_from_variant_ctor_expr(
        &self,
        expr: &syn::Expr,
    ) -> Option<String> {
        let expr = self.peel_paren_group_expr(expr);
        let syn::Expr::Call(call) = expr else {
            return None;
        };
        let func = self.peel_paren_group_expr(call.func.as_ref());
        let syn::Expr::Path(path_expr) = func else {
            return None;
        };
        let path = &path_expr.path;
        if !self.path_is_known_data_enum_variant(path) {
            return None;
        }

        if path.segments.len() >= 2 {
            let owner_path = Self::path_without_last_segment(path)?;
            let owner_cpp = self.emit_path_to_string(&owner_path);
            if !owner_cpp.is_empty() {
                return Some(owner_cpp);
            }
        }

        let variant_name = path.segments.last()?.ident.to_string();
        let (enum_name, _) = self.flattened_data_enum_variant_parts(&variant_name)?;
        Some(escape_cpp_keyword(&enum_name))
    }

    pub(super) fn recover_variant_constructor_owner_generic_args(
        &self,
        path: &syn::Path,
    ) -> Option<Vec<String>> {
        if path.segments.len() < 2 {
            return None;
        }
        if let Some(owner_seg) = path.segments.iter().nth_back(1)
            && let syn::PathArguments::AngleBracketed(owner_args) = &owner_seg.arguments
        {
            let mapped_owner_args: Vec<String> = owner_args
                .args
                .iter()
                .filter_map(|arg| match arg {
                    syn::GenericArgument::Type(ty) => Some(self.map_type(ty)),
                    syn::GenericArgument::Const(expr) => Some(self.emit_expr_to_string(expr)),
                    _ => None,
                })
                .collect();
            if !mapped_owner_args.is_empty() {
                return Some(mapped_owner_args);
            }
        }

        let mut owner_path = syn::Path {
            leading_colon: path.leading_colon,
            segments: syn::punctuated::Punctuated::new(),
        };
        for seg in path
            .segments
            .iter()
            .take(path.segments.len().saturating_sub(1))
        {
            owner_path.segments.push(seg.clone());
        }
        let recovered_from_scope = self
            .recover_omitted_owner_generic_args_from_scope(&owner_path)
            .or_else(|| {
                if owner_path.leading_colon.is_some() {
                    let mut unrooted = owner_path.clone();
                    unrooted.leading_colon = None;
                    self.recover_omitted_owner_generic_args_from_scope(&unrooted)
                } else {
                    None
                }
            });
        if recovered_from_scope.is_some() {
            return recovered_from_scope;
        }

        // Match-arm/lambda lowering can temporarily drop owner generic params
        // (for example `Either<L, R>` inside `impl IterEither<L, R>` methods).
        // If the owner params are declared on the current enclosing struct, use
        // those names directly.
        let owner_declared_params = self.declared_type_params_for_path(&owner_path)?;
        if owner_declared_params.is_empty() {
            return None;
        }
        let current_struct = self.current_struct.as_ref()?;
        let current_scoped = self.scoped_type_key(current_struct);
        let current_params = self
            .declared_type_params
            .get(current_struct)
            .or_else(|| self.declared_type_params.get(&current_scoped))?;
        if owner_declared_params
            .iter()
            .all(|param| current_params.contains(param))
        {
            return Some(owner_declared_params.to_vec());
        }
        None
    }

    pub(super) fn resolve_single_segment_variant_ctor_import_path(
        &self,
        path: &syn::Path,
        ctor_name: &str,
    ) -> Option<syn::Path> {
        if path.segments.len() != 1 {
            return None;
        }
        if !matches!(
            path.segments.last().map(|seg| &seg.arguments),
            Some(syn::PathArguments::None)
        ) {
            return None;
        }
        let local_name = path.segments.first()?.ident.to_string();
        let bound_target = self
            .resolve_scope_import_binding_path(&local_name)
            .or_else(|| self.resolve_scope_import_binding_path_for_scope("", &local_name))?;
        let bound_path = syn::parse_str::<syn::Path>(&bound_target).ok()?;
        if bound_path.segments.len() < 2 {
            return None;
        }
        let bound_ctor = self.variant_ctor_name_from_path(&bound_path)?;
        if bound_ctor != ctor_name {
            return None;
        }
        Some(bound_path)
    }

    pub(super) fn infer_method_turbofish_type_arg_from_call_arg(
        &self,
        mc: &syn::ExprMethodCall,
        emitted_args: &[String],
        type_param_idx: usize,
    ) -> Option<String> {
        let call_arg_cpp = emitted_args.get(type_param_idx).cloned().or_else(|| {
            mc.args
                .iter()
                .nth(type_param_idx)
                .map(|arg| self.emit_expr_maybe_move(arg))
        })?;
        Some(format!("std::remove_cvref_t<decltype(({}))>", call_arg_cpp))
    }

    pub(super) fn infer_serde_access_method_template_type_from_expected(
        &self,
        method_name: &str,
        expected_ty: Option<&syn::Type>,
    ) -> Option<syn::Type> {
        let expected_ty = expected_ty?;
        let ok_ty = self
            .expected_result_type_arg(Some(expected_ty), 0)
            .unwrap_or(expected_ty);
        let inferred = match method_name {
            "next_element" | "next_key" => extract_option_inner_type_for_hint(ok_ty),
            "next_value" => Some(ok_ty.clone()),
            _ => None,
        }?;
        if self.type_contains_infer(&inferred)
            || self.type_contains_unbound_single_letter_generic(&inferred)
            || self.type_contains_unresolved_placeholder_like(&inferred)
            || self.type_maps_to_auto_placeholder_like(&inferred)
        {
            return None;
        }
        Some(inferred)
    }

    pub(super) fn infer_serde_next_entry_template_args_from_expected(
        &self,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let expected_ty = expected_ty?;
        let ok_ty = self
            .expected_result_type_arg(Some(expected_ty), 0)
            .unwrap_or(expected_ty);
        let option_inner = extract_option_inner_type_for_hint(ok_ty)?;
        let tuple_ty = self.expected_tuple_type(Some(&option_inner))?;
        if tuple_ty.elems.len() != 2 {
            return None;
        }
        let key_cpp = self.map_type(tuple_ty.elems.first()?);
        let value_cpp = self.map_type(tuple_ty.elems.iter().nth(1)?);
        if [key_cpp.as_str(), value_cpp.as_str()].iter().any(|arg| {
            *arg == "auto" || arg.contains("/* TODO") || type_string_has_auto_placeholder(arg)
        }) {
            return None;
        }
        Some(format!("<{}, {}>", key_cpp, value_cpp))
    }

    pub(super) fn infer_serde_next_entry_template_args_from_in_scope_map_hint(&self) -> Option<String> {
        let mut candidates: Vec<(syn::Type, syn::Type)> = Vec::new();
        for scope in self.local_placeholder_type_hints.iter().rev() {
            for ty in scope.values() {
                if let Some((key_ty, value_ty)) = extract_map_key_value_types_for_hint(ty)
                    .or_else(|| extract_hashmap_key_value_types_for_hint(ty))
                {
                    candidates.push((key_ty, value_ty));
                }
            }
            if !candidates.is_empty() {
                break;
            }
        }
        candidates.dedup_by(|a, b| {
            Self::types_equivalent_by_tokens(&a.0, &b.0)
                && Self::types_equivalent_by_tokens(&a.1, &b.1)
        });
        if candidates.len() != 1 {
            return None;
        }
        let (key_ty, value_ty) = candidates.pop()?;
        let key_cpp = self.map_type(&key_ty);
        let value_cpp = self.map_type(&value_ty);
        if [key_cpp.as_str(), value_cpp.as_str()].iter().any(|arg| {
            *arg == "auto" || arg.contains("/* TODO") || type_string_has_auto_placeholder(arg)
        }) {
            return None;
        }
        Some(format!("<{}, {}>", key_cpp, value_cpp))
    }

    pub(super) fn infer_option_cpp_type_from_if_some_payload_decltype(
        &mut self,
        if_expr: &syn::ExprIf,
    ) -> Option<String> {
        let then_tail = self.extract_tail_expr_from_block(&if_expr.then_branch)?;
        let payload_expr = self.extract_option_some_call_arg(then_tail)?;
        let payload_expr = self.peel_paren_group_expr(payload_expr);
        let syn::Expr::Path(path_expr) = payload_expr else {
            return None;
        };
        if path_expr.path.segments.len() != 1 {
            return None;
        }
        let payload_name = path_expr.path.segments[0].ident.to_string();
        let payload_local =
            self.find_local_binding_in_block_by_name(&if_expr.then_branch, &payload_name)?;

        if let Some(explicit_ty) = get_local_type(payload_local)
            && !type_has_generic_placeholder(explicit_ty)
        {
            let option_ty: syn::Type = parse_quote!(Option<#explicit_ty>);
            let mapped = self.map_type(&option_ty);
            if Self::is_concrete_cpp_type_for_iflet_init(&mapped) {
                return Some(mapped);
            }
        }

        let init_expr = payload_local.init.as_ref().map(|init| init.expr.as_ref())?;
        let init_expr_owned = init_expr.clone();
        let mut inferred_from_init: Option<syn::Type> = None;
        self.with_pre_scan_known_local_scope(&if_expr.then_branch.stmts, |this| {
            inferred_from_init = this
                .infer_local_binding_type_from_initializer(&init_expr_owned)
                .or_else(|| this.infer_simple_expr_type(&init_expr_owned))
                .or_else(|| this.infer_try_payload_type_from_expr(&init_expr_owned));
        });
        if let Some(inner_ty) = inferred_from_init {
            let option_ty: syn::Type = parse_quote!(Option<#inner_ty>);
            let mapped = self.map_type(&option_ty);
            if Self::is_concrete_cpp_type_for_iflet_init(&mapped) {
                return Some(mapped);
            }
        }

        let init_expr = self.peel_paren_group_expr(init_expr);
        if let syn::Expr::Try(try_expr) = init_expr {
            // Keep this unevaluated: recover payload type from `Result<_, _>`/`Option<_>`
            // shape of the try-operand when Rust-side inference is unavailable.
            let try_operand_cpp = self.emit_expr_to_string(&try_expr.expr);
            let result_cpp = format!("std::remove_cvref_t<decltype(({}))>", try_operand_cpp);
            let inner_cpp = format!(
                "std::remove_cvref_t<decltype((std::declval<{}>().unwrap()))>",
                result_cpp
            );
            return Some(format!("rusty::Option<{}>", inner_cpp));
        }

        let init_cpp = self.emit_expr_to_string(init_expr);
        Some(format!(
            "rusty::Option<std::remove_cvref_t<decltype(({}))>>",
            init_cpp
        ))
    }

    /// Extract concrete C++ template args from an annotated
    /// `Either<L, R>` type. Used by the if/else ternary emit to
    /// prefer precise types (`Left<int32_t, int32_t>(1)`) over the
    /// decltype-based fallback (`Left<decltype((1)), decltype((2))>
    /// (decltype((1))(1))`) — the decltype form is correct but
    /// noisy and was the only available signal for unannotated
    /// ternaries.
    pub(super) fn expected_either_concrete_template_args(
        &self,
        ty: &syn::Type,
    ) -> Option<Vec<String>> {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return None;
        };
        if tp.qself.is_some() {
            return None;
        }
        let seg = tp.path.segments.last()?;
        if seg.ident != "Either" {
            return None;
        }
        let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
            return None;
        };
        let mut out = Vec::with_capacity(2);
        for arg in &args.args {
            let syn::GenericArgument::Type(arg_ty) = arg else {
                return None;
            };
            out.push(self.map_type(arg_ty));
            if out.len() == 2 {
                break;
            }
        }
        if out.len() == 2 {
            Some(out)
        } else {
            None
        }
    }

    pub(super) fn infer_variant_ctor_template_args_from_if(
        &self,
        if_expr: &syn::ExprIf,
    ) -> Option<Vec<String>> {
        let (lhs_name, lhs_arg, rhs_name, rhs_arg) =
            self.extract_constructor_pair_from_if(if_expr)?;
        let lhs_cpp = self.emit_expr_maybe_move(lhs_arg);
        let rhs_cpp = self.emit_expr_maybe_move(rhs_arg);
        let lhs_ty = format!("decltype(({}))", lhs_cpp);
        let rhs_ty = format!("decltype(({}))", rhs_cpp);

        match (lhs_name.as_str(), rhs_name.as_str()) {
            ("Left", "Right") => Some(vec![lhs_ty, rhs_ty]),
            ("Right", "Left") => Some(vec![rhs_ty, lhs_ty]),
            _ => None,
        }
    }

    pub(super) fn infer_fold_like_init_expected_type_from_method_call(
        &self,
        mc: &syn::ExprMethodCall,
        call_expected_ty: Option<&syn::Type>,
        is_try_fold: bool,
    ) -> Option<syn::Type> {
        if mc.args.len() != 2 {
            return None;
        }
        if let Some(from_reducer_acc) = self.lookup_function_arg_expected_type(&mc.args[1], 0)
            && self.type_is_concrete_hint_candidate(from_reducer_acc)
        {
            return Some(from_reducer_acc.clone());
        }
        // When the accumulator is an element-less `Vec::new()` and the
        // reducer pushes a typed value (`|mut acc, v: I::Item| { acc.push(v); acc }`),
        // the accumulator param itself carries no annotation, so the
        // check above misses it. Resolve the element type via the
        // constraint solver from the reducer's body/params and return
        // the recovered `Vec<Elem>` so the init emits a concrete owner
        // instead of leaking `rusty::Vec<auto>`.
        if let Some(vec_acc_ty) = self.infer_fold_vec_accumulator_expected_type(mc) {
            return Some(vec_acc_ty);
        }
        let contextual_expected_ty = call_expected_ty.or(self.current_return_type_hint());
        let from_call_expected = if is_try_fold {
            contextual_expected_ty
                .and_then(|ty| self.expected_option_type_arg(Some(ty)).cloned())
                .or_else(|| {
                    contextual_expected_ty
                        .and_then(|ty| self.expected_result_type_arg(Some(ty), 0).cloned())
                })
        } else {
            contextual_expected_ty.cloned()
        };
        if let Some(from_call_expected) = from_call_expected
            && self.type_is_concrete_hint_candidate(&from_call_expected)
        {
            return Some(from_call_expected);
        }
        let init_expr = self.peel_paren_group_expr(&mc.args[0]);
        if !Self::is_unsuffixed_int_literal_expr(init_expr) {
            return None;
        }
        if let Some(iter_item_ty) = self.infer_iter_item_type_from_expr(&mc.receiver)
            && self.type_is_concrete_hint_candidate(&iter_item_ty)
            && is_numeric_cpp_scalar_type(self.map_type(&iter_item_ty).trim())
        {
            return Some(iter_item_ty);
        }
        if let Some(counting_hint) =
            self.infer_fold_like_counting_init_hint_from_iter_expr(&mc.receiver)
        {
            return Some(counting_hint);
        }
        None
    }

    /// Resolve the expected type of a `fold`/`rfold` accumulator that
    /// is an element-less `Vec::new()` whose element type is revealed
    /// only by the reducer (a typed param and/or `acc.push(x)` in the
    /// body). Drives the constraint solver
    /// (`type_solver::infer_owner_accumulator_element_from_reducer`)
    /// and returns `Vec<Elem>` when the engine pins the element, else
    /// `None`. The resulting type may legitimately reference an
    /// in-scope type parameter (e.g. `Vec<I::Item>` →
    /// `rusty::Vec<rusty::detail::associated_item_t<I>>`), so it is
    /// validated with the scoped-generics-permitting candidate check
    /// rather than the concrete-only one.
    pub(super) fn infer_fold_vec_accumulator_expected_type(
        &self,
        mc: &syn::ExprMethodCall,
    ) -> Option<syn::Type> {
        if mc.args.len() != 2 {
            return None;
        }
        let init = self.peel_paren_group_expr(&mc.args[0]);
        let syn::Expr::Call(init_call) = init else {
            return None;
        };
        let owner_head = super::type_solver::owner_constructor_head(init_call)?;
        let reducer = self.peel_paren_group_expr(&mc.args[1]);
        let syn::Expr::Closure(reducer) = reducer else {
            return None;
        };
        let item_hint = self
            .infer_iter_item_type_with_generic_fallback(&mc.receiver)
            .filter(|t| self.type_is_placeholder_hint_candidate_allow_scoped_generics(t));
        let elem = super::type_solver::infer_owner_accumulator_element_from_reducer(
            reducer,
            &owner_head,
            item_hint.as_ref(),
        )?;
        if !self.type_is_placeholder_hint_candidate_allow_scoped_generics(&elem) {
            return None;
        }
        Some(parse_quote!(Vec<#elem>))
    }

    /// Iterator item type of `expr`, falling back to `P::Item` when the
    /// receiver is a bare local/param whose type is an in-scope generic
    /// type parameter `P` (e.g. `fn f<I: Iterator>(i: I)` → `I::Item`,
    /// which maps to `rusty::detail::associated_item_t<I>`).
    /// `infer_iter_item_type_from_expr` handles concrete iterator types;
    /// this adds the generic-iterator-parameter case that the structural
    /// `extract_iter_item_type_from_type` can't see.
    pub(super) fn infer_iter_item_type_with_generic_fallback(
        &self,
        expr: &syn::Expr,
    ) -> Option<syn::Type> {
        if let Some(item) = self.infer_iter_item_type_from_expr(expr) {
            return Some(item);
        }
        let peeled = self.peel_paren_group_expr(expr);
        let syn::Expr::Path(p) = peeled else {
            return None;
        };
        if p.qself.is_some() || p.path.segments.len() != 1 {
            return None;
        }
        let recv_ty = self.lookup_local_binding_type(&p.path.segments[0].ident.to_string())?;
        let syn::Type::Path(tp) = &recv_ty else {
            return None;
        };
        if tp.qself.is_some() || tp.path.segments.len() != 1 {
            return None;
        }
        let param_name = tp.path.segments[0].ident.to_string();
        if !self.is_type_param_in_scope(&param_name) {
            return None;
        }
        Some(parse_quote!(#recv_ty::Item))
    }

    pub(super) fn infer_byte_write_expected_type_from_receiver_hint(
        &self,
        receiver: &syn::Expr,
    ) -> Option<syn::Type> {
        let name = extract_simple_local_ident(receiver)?;
        let hint = self.lookup_local_placeholder_type_hint(&name)?;
        if !Self::is_u8_syn_type(hint) {
            return None;
        }
        let elem_ty = hint.clone();
        Some(parse_quote!(&[#elem_ty]))
    }

    pub(super) fn resolve_unqualified_runtime_helper_trait_type_name(
        &self,
        local_name: &str,
    ) -> Option<String> {
        if local_name.is_empty() || self.module_runtime_helper_trait_type_names.is_empty() {
            return None;
        }
        let scope_key = self.module_stack.join("::");
        let scoped_name = if scope_key.is_empty() {
            local_name.to_string()
        } else {
            format!("{}::{}", scope_key, local_name)
        };
        if let Some(mapped) = self.module_runtime_helper_trait_type_name_for_key(&scoped_name) {
            return Some(mapped);
        }

        let has_scope_binding = self.scope_has_import_binding_candidates(&scope_key, local_name);
        let has_root_binding = self.scope_has_import_binding_candidates("", local_name);
        if has_scope_binding || has_root_binding {
            let bound_target = self
                .resolve_scope_import_binding_path(local_name)
                .or_else(|| self.resolve_scope_import_binding_path_for_scope("", local_name));
            let Some(bound_target) = bound_target else {
                return None;
            };
            return self.module_runtime_helper_trait_type_name_for_key(
                bound_target.trim_start_matches("::"),
            );
        }

        if self.current_scope_declares_type_name(local_name)
            || self.current_scope_declares_function_name(local_name)
        {
            return None;
        }
        self.module_runtime_helper_trait_type_name_for_key(local_name)
    }

    pub(super) fn resolve_unique_forward_decl_type_path(&self, name: &str) -> Option<String> {
        if !self.in_forward_decl_signature
            || name.is_empty()
            || name == "Self"
            || self.is_type_param_in_scope(name)
        {
            return None;
        }

        let mut scoped_matches: Vec<&str> = self
            .local_declared_types
            .iter()
            .filter_map(|decl| {
                let (_, tail) = decl.rsplit_once("::")?;
                if tail == name {
                    Some(decl.as_str())
                } else {
                    None
                }
            })
            .collect();

        if scoped_matches.is_empty() {
            return None;
        }

        // Prefer local namespace declarations where available; unqualified names are
        // already valid there and should remain stable.
        let current_scoped = if self.module_stack.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", self.module_stack.join("::"), name)
        };
        if scoped_matches
            .iter()
            .any(|candidate| *candidate == current_scoped)
        {
            return None;
        }

        scoped_matches.sort_unstable();
        scoped_matches.dedup();
        if scoped_matches.len() != 1 {
            return None;
        }

        let escaped = self.escape_and_rename_qualified_name(scoped_matches[0]);
        Some(format!("::{}", escaped))
    }

    pub(super) fn resolve_unique_nonlocal_type_path(&self, name: &str) -> Option<String> {
        if name.is_empty()
            || name == "Self"
            || self.is_type_param_in_scope(name)
            || !name
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            return None;
        }
        let _resolution_guard =
            NonlocalTypeResolutionGuard::enter(&self.nonlocal_type_resolution_in_progress, name)?;
        let scope_key = self.module_stack.join("::");
        if let Some(bound_target) =
            self.resolve_scope_import_binding_path_for_scope(&scope_key, name)
        {
            // Prefer explicit in-scope `use` bindings over nonlocal fallback
            // qualification. If a local declaration with the same tail name
            // exists, qualify through the bound import target to avoid
            // resolving to an unrelated local type.
            let local_tail_conflict = self
                .local_declared_types
                .iter()
                .any(|decl| decl.rsplit_once("::").is_some_and(|(_, tail)| tail == name));
            if local_tail_conflict {
                let is_known_declared_type_path = |candidate: &str| {
                    let trimmed = candidate.trim_start_matches("::");
                    if trimmed.is_empty() {
                        return false;
                    }
                    let escaped = Self::escape_qualified_path_preserve_global(trimmed);
                    (self.local_declared_types.contains(trimmed)
                        || self.local_declared_types.contains(&escaped))
                        && self.qualified_path_root_is_emittable(trimmed)
                };
                let trimmed = bound_target.trim();
                if !trimmed.is_empty() {
                    let had_global_prefix = trimmed.starts_with("::");
                    let normalized_bound =
                        Self::strip_crate_root_cpp_path(trimmed.trim_start_matches("::"));
                    if !normalized_bound.is_empty()
                        && normalized_bound != name
                        && normalized_bound.contains("::")
                        && !matches!(
                            classify_use_import(normalized_bound.as_str()),
                            UseImportAction::RustOnly
                        )
                    {
                        let bound_root = normalized_bound.split("::").next().unwrap_or_default();
                        let avoid_forced_global_for_private_alias =
                            bound_root.ends_with("_private");
                        let mut escaped =
                            Self::escape_qualified_path_preserve_global(normalized_bound.as_str());
                        if avoid_forced_global_for_private_alias {
                            escaped = escaped.trim_start_matches("::").to_string();
                        } else if had_global_prefix && !escaped.starts_with("::") {
                            escaped = format!("::{}", escaped);
                        }
                        return Some(self.rewrite_forced_global_private_alias_root_for_scope(
                            &escaped, &scope_key,
                        ));
                    }
                    if let Ok(bound_path) = syn::parse_str::<syn::Path>(trimmed) {
                        let mut emitted = self.emit_path_to_string(&bound_path);
                        if !emitted.is_empty() && emitted != name {
                            if !emitted.starts_with("::") {
                                emitted = format!("::{}", emitted);
                            }
                            if is_known_declared_type_path(&emitted) {
                                return Some(
                                    self.rewrite_forced_global_private_alias_root_for_scope(
                                        &emitted, &scope_key,
                                    ),
                                );
                            }
                        }
                    }
                    let mut escaped = Self::escape_qualified_path_preserve_global(trimmed);
                    if !escaped.is_empty() && escaped != name {
                        if !escaped.starts_with("::") {
                            escaped = format!("::{}", escaped);
                        }
                        if is_known_declared_type_path(&escaped) {
                            return Some(self.rewrite_forced_global_private_alias_root_for_scope(
                                &escaped, &scope_key,
                            ));
                        }
                    }
                }
            } else {
                return None;
            }
        }

        let mut scoped_matches: Vec<&str> = self
            .local_declared_types
            .iter()
            .filter_map(|decl| {
                let (_, tail) = decl.rsplit_once("::")?;
                if tail == name && self.qualified_path_root_is_emittable(decl) {
                    Some(decl.as_str())
                } else {
                    None
                }
            })
            .collect();
        if scoped_matches.is_empty() {
            return None;
        }

        let current_scoped = if self.module_stack.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", self.module_stack.join("::"), name)
        };
        if scoped_matches
            .iter()
            .any(|candidate| *candidate == current_scoped)
        {
            return None;
        }

        scoped_matches.sort_unstable();
        scoped_matches.dedup();
        if scoped_matches.len() != 1 {
            return None;
        }

        let escaped = self.escape_and_rename_qualified_name(scoped_matches[0]);
        let qualified = format!("::{}", escaped);
        Some(self.rewrite_forced_global_private_alias_root_for_scope(&qualified, &scope_key))
    }

    pub(super) fn resolve_nonlocal_type_path_with_namespace_hint(
        &self,
        name: &str,
        namespace_hint: &str,
    ) -> Option<String> {
        if name.is_empty() || namespace_hint.is_empty() {
            return None;
        }
        let scope_key = self.module_stack.join("::");
        let mut scoped_matches: Vec<&str> = self
            .local_declared_types
            .iter()
            .filter_map(|decl| {
                let (_, tail) = decl.rsplit_once("::")?;
                if tail == name && self.qualified_path_root_is_emittable(decl) {
                    Some(decl.as_str())
                } else {
                    None
                }
            })
            .collect();
        if scoped_matches.is_empty() {
            return None;
        }
        let current_scoped = if self.module_stack.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", self.module_stack.join("::"), name)
        };
        scoped_matches.retain(|candidate| *candidate != current_scoped);
        if scoped_matches.is_empty() {
            return None;
        }
        scoped_matches.sort_unstable();
        scoped_matches.dedup();
        if scoped_matches.len() == 1 {
            let escaped = self.escape_and_rename_qualified_name(scoped_matches[0]);
            let qualified = format!("::{}", escaped);
            return Some(
                self.rewrite_forced_global_private_alias_root_for_scope(&qualified, &scope_key),
            );
        }

        let hint = namespace_hint.trim_start_matches("::");
        let rank = |candidate: &str| -> usize {
            let parent_tail = candidate
                .rsplit_once("::")
                .map(|(parent, _)| parent.rsplit("::").next().unwrap_or(parent))
                .unwrap_or_default();
            if parent_tail == hint {
                return 3;
            }
            if parent_tail.starts_with(hint) || hint.starts_with(parent_tail) {
                return 2;
            }
            if parent_tail.contains(hint) || hint.contains(parent_tail) {
                return 1;
            }
            0
        };
        let mut ranked: Vec<(usize, &str)> = scoped_matches
            .iter()
            .map(|candidate| (rank(candidate), *candidate))
            .filter(|(score, _)| *score > 0)
            .collect();
        if ranked.is_empty() {
            return None;
        }
        ranked.sort_by(|(ls, lp), (rs, rp)| rs.cmp(ls).then_with(|| lp.cmp(rp)));
        let best = ranked.first().copied()?;
        if ranked.iter().skip(1).any(|(score, _)| *score == best.0) {
            return None;
        }
        let escaped = self.escape_and_rename_qualified_name(best.1);
        let qualified = format!("::{}", escaped);
        Some(self.rewrite_forced_global_private_alias_root_for_scope(&qualified, &scope_key))
    }

    pub(super) fn resolve_c_like_enum_owner_for_variant_from_return_hint(
        &self,
        variant_name: &str,
    ) -> Option<String> {
        if variant_name.is_empty() {
            return None;
        }
        let mut owner_candidates: Vec<String> = Vec::new();
        if let Some(return_hint) = self.current_return_type_hint() {
            self.collect_c_like_owner_tail_candidates_from_type(return_hint, &mut owner_candidates);
            if let Some(ok_ty) = self.expected_result_type_arg(Some(return_hint), 0) {
                self.collect_c_like_owner_tail_candidates_from_type(ok_ty, &mut owner_candidates);
            }
        }
        owner_candidates.sort();
        owner_candidates.dedup();
        for owner_tail in owner_candidates {
            let key = format!("{}_{}", owner_tail, variant_name);
            if self.c_like_enum_consts.contains(&key)
                || self.c_like_enum_variants.contains(&key)
                || self.path_matches_c_like_enum_const(&owner_tail, variant_name)
            {
                return Some(owner_tail);
            }
        }
        None
    }

    pub(super) fn recover_omitted_local_generic_type_args(
        &self,
        path: &syn::Path,
        mapped_path: &str,
    ) -> Option<String> {
        // Avoid duplicating template argument lists when the mapped spelling is
        // already specialized (e.g. `ConstNonNull<T>` should not become
        // `ConstNonNull<T><T>`).
        if mapped_path.contains('<') {
            return Some(mapped_path.to_string());
        }
        let last_seg = path.segments.last()?;
        if !matches!(last_seg.arguments, syn::PathArguments::None) {
            return None;
        }
        if self.current_struct.is_some() && path.segments.len() == 1 {
            let local_name = last_seg.ident.to_string();
            if self.is_local_type_name_in_scope(&local_name) {
                let has_declared_generics = self
                    .declared_type_key_for_path(path)
                    .or_else(|| self.lookup_declared_type_key_for_base(&local_name, &local_name))
                    .and_then(|key| self.declared_type_params.get(&key))
                    .is_some_and(|params| !params.is_empty());
                if !has_declared_generics {
                    return Some(mapped_path.to_string());
                }
            }
        }
        let type_key = self
            .current_struct_declared_type_key_for_recovery(path, mapped_path)
            .or_else(|| self.declared_type_key_for_path(path))
            .or_else(|| {
                if path.segments.len() != 1 {
                    return None;
                }
                let base = path.segments.first()?.ident.to_string();
                let mut scoped_candidates: Vec<String> = Vec::new();
                if let Some(current_struct) = &self.current_struct {
                    scoped_candidates.push(format!("{}::{}", current_struct, base));
                    if !self.module_stack.is_empty() {
                        scoped_candidates.push(format!(
                            "{}::{}::{}",
                            self.module_stack.join("::"),
                            current_struct,
                            base
                        ));
                    }
                }
                if !self.module_stack.is_empty() {
                    scoped_candidates.push(format!("{}::{}", self.module_stack.join("::"), base));
                }
                scoped_candidates.push(base.clone());
                for scoped in scoped_candidates {
                    if let Some(key) = self.lookup_declared_type_key_for_base(&scoped, &base) {
                        return Some(key);
                    }
                }
                None
            })
            .or_else(|| {
                let mapped_base = mapped_path
                    .trim_start_matches("::")
                    .split('<')
                    .next()
                    .unwrap_or(mapped_path)
                    .trim();
                if mapped_base.is_empty() {
                    return None;
                }
                let base = mapped_base.rsplit("::").next().unwrap_or(mapped_base);
                self.lookup_declared_type_key_for_base(mapped_base, base)
            })?;
        let params = self.declared_type_params.get(&type_key)?;
        let defaults = self.declared_type_param_defaults_for_path(path);
        if params.is_empty() {
            return None;
        }
        let declared_kinds = self.declared_type_param_kinds.get(&type_key);
        let fallback_args_from_current_struct =
            self.fallback_owner_generic_args_for_local_type(&type_key, params, declared_kinds);
        let current_struct_scope_params: HashSet<String> = fallback_args_from_current_struct
            .as_ref()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();

        let mut recovered_args: Vec<String> = Vec::new();
        let mut resolved_type_defaults: HashMap<String, syn::Type> = HashMap::new();
        for (idx, param) in params.iter().enumerate() {
            if let Some(default) = defaults
                .and_then(|all| all.get(idx))
                .and_then(|entry| entry.as_ref())
            {
                let mapped_default = match default {
                    GenericParamDefault::Type(t) => {
                        let substituted = if resolved_type_defaults.is_empty() {
                            t.clone()
                        } else {
                            self.substitute_type_params_in_type(t, &resolved_type_defaults)
                        };
                        let mapped = self.map_type(&substituted);
                        resolved_type_defaults.insert(param.clone(), substituted);
                        mapped
                    }
                    GenericParamDefault::Const(c) => self.emit_expr_to_string(c),
                };
                recovered_args.push(mapped_default);
                continue;
            }
            if let Some(fallback) = fallback_args_from_current_struct.as_ref()
                && let Some(arg) = fallback.get(idx)
            {
                recovered_args.push(arg.clone());
                if let Ok(ty) = syn::parse_str::<syn::Type>(arg) {
                    resolved_type_defaults.insert(param.clone(), ty);
                }
                continue;
            }
            if current_struct_scope_params.contains(param) && self.is_type_param_in_scope(param) {
                recovered_args.push(param.clone());
                if let Ok(ty) = syn::parse_str::<syn::Type>(param) {
                    resolved_type_defaults.insert(param.clone(), ty);
                }
                continue;
            }
            return None;
        }

        if recovered_args.len() != params.len() {
            return None;
        }
        if recovered_args.is_empty() {
            return None;
        }
        if self.should_elide_shadowed_current_struct_local_recovered_args(
            &type_key,
            params,
            &recovered_args,
        ) {
            return Some(mapped_path.to_string());
        }
        Some(format!("{}<{}>", mapped_path, recovered_args.join(", ")))
    }

    pub(super) fn recover_explicit_owner_type_from_type(
        &self,
        ty: &syn::Type,
        mapped_owner: &str,
    ) -> Option<String> {
        if mapped_owner.contains('<') {
            return Some(mapped_owner.to_string());
        }
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return None;
        };
        let last = tp.path.segments.last()?;
        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
            return None;
        };
        let mapped_args: Vec<String> = args
            .args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                syn::GenericArgument::Const(c) => Some(self.emit_expr_to_string(c)),
                _ => None,
            })
            .collect();
        if mapped_args.is_empty() {
            return None;
        }
        let mut owner_path = tp.path.clone();
        if let Some(owner_last) = owner_path.segments.last_mut() {
            owner_last.arguments = syn::PathArguments::None;
        }
        let owner_cpp = self.emit_path_to_string(&owner_path);
        if owner_cpp.is_empty() {
            return None;
        }
        if owner_cpp.contains('<') {
            return Some(owner_cpp);
        }
        Some(format!("{}<{}>", owner_cpp, mapped_args.join(", ")))
    }

    pub(super) fn recover_omitted_struct_literal_generic_type_args_from_fields(
        &self,
        struct_expr: &syn::ExprStruct,
        mapped_path: &str,
    ) -> Option<String> {
        if std::env::var_os("RUSTY_CPP_DISABLE_OMITTED_STRUCT_FIELD_RECOVERY").is_some() {
            return None;
        }
        let last_seg = struct_expr.path.segments.last()?;
        if !matches!(last_seg.arguments, syn::PathArguments::None) {
            return None;
        }
        let mapped_path_base = {
            let trimmed = mapped_path.trim();
            let had_global = trimmed.starts_with("::");
            let body = trimmed.trim_start_matches("::");
            let base = body.split('<').next().unwrap_or(body).trim();
            if had_global {
                format!("::{}", base)
            } else {
                base.to_string()
            }
        };
        let type_key = self
            .declared_type_key_for_path(&struct_expr.path)
            .or_else(|| {
                if struct_expr.path.segments.len() != 1 {
                    return None;
                }
                let base = struct_expr.path.segments.first()?.ident.to_string();
                let mut scoped_candidates: Vec<String> = Vec::new();
                if let Some(current_struct) = &self.current_struct {
                    scoped_candidates.push(format!("{}::{}", current_struct, base));
                    if !self.module_stack.is_empty() {
                        scoped_candidates.push(format!(
                            "{}::{}::{}",
                            self.module_stack.join("::"),
                            current_struct,
                            base
                        ));
                    }
                }
                if !self.module_stack.is_empty() {
                    scoped_candidates.push(format!("{}::{}", self.module_stack.join("::"), base));
                }
                scoped_candidates.push(base.clone());
                for scoped in scoped_candidates {
                    if let Some(key) = self.lookup_declared_type_key_for_base(&scoped, &base) {
                        return Some(key);
                    }
                }
                None
            })?;
        let params = self.declared_type_params.get(&type_key)?;
        if params.is_empty() {
            return None;
        }
        let is_current_struct_local_type = self
            .current_struct
            .as_ref()
            .is_some_and(|owner| type_key.starts_with(&format!("{}::", owner)));

        let resolved_struct_name = {
            let raw = last_seg.ident.to_string();
            if raw == "Self" {
                self.current_struct.clone().unwrap_or(raw)
            } else {
                raw
            }
        };
        let param_kinds = self.declared_type_param_kinds.get(&type_key);
        let local_name = if struct_expr.path.segments.len() == 1 {
            Some(last_seg.ident.to_string())
        } else {
            None
        };
        let is_local_type_context = local_name.as_ref().is_some_and(|name| {
            self.is_local_type_name_in_scope(name) || self.local_declared_types.contains(name)
        }) || is_current_struct_local_type;
        let mut inferred_args: Vec<Option<String>> = vec![None; params.len()];
        let mut conflict = false;
        for field in &struct_expr.fields {
            let syn::Member::Named(ident) = &field.member else {
                continue;
            };
            let field_name = ident.to_string();
            let field_ty = self
                .struct_field_types
                .get(&type_key)
                .and_then(|fields| fields.get(&field_name).cloned())
                .or_else(|| self.lookup_struct_field_type(&resolved_struct_name, &field_name));
            let Some(field_ty) = field_ty else {
                continue;
            };
            let Some(expr_ty) = self.infer_simple_expr_type(&field.expr) else {
                continue;
            };
            self.collect_omitted_generic_bindings_from_field_types(
                &field_ty,
                &expr_ty,
                params,
                param_kinds,
                &mut inferred_args,
                &mut conflict,
            );
            if conflict {
                return None;
            }
        }

        let defaults = self.declared_type_param_defaults.get(&type_key);
        let fallback_args_from_current_struct =
            self.fallback_owner_generic_args_for_local_type(&type_key, params, param_kinds);
        let current_struct_scope_params: HashSet<String> = fallback_args_from_current_struct
            .as_ref()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();

        let mut recovered_args: Vec<Option<String>> = vec![None; params.len()];
        for (idx, param) in params.iter().enumerate() {
            if let Some(inferred) = inferred_args.get(idx).and_then(|arg| arg.clone()) {
                // Field-type inference should win when a struct literal explicitly
                // constrains a generic slot (e.g. `CapacityError { value: () }` in
                // `impl<T> CapacityError<T>` should become `CapacityError<()>`).
                recovered_args[idx] = Some(inferred);
                continue;
            }
            let is_type_param_kind = param_kinds
                .and_then(|kinds| kinds.get(idx))
                .is_none_or(|kind| matches!(kind, GenericParamKind::Type));
            if is_type_param_kind && self.is_type_param_in_scope(param) {
                recovered_args[idx] = Some(param.clone());
                continue;
            }
            let param_in_scope =
                current_struct_scope_params.contains(param) && self.is_type_param_in_scope(param);
            if let Some(default) = defaults
                .and_then(|all| all.get(idx))
                .and_then(|entry| entry.as_ref())
            {
                let mapped_default = match default {
                    GenericParamDefault::Type(t) => self.map_type(t),
                    GenericParamDefault::Const(c) => self.emit_expr_to_string(c),
                };
                recovered_args[idx] = Some(mapped_default);
                continue;
            }
            if !is_local_type_context
                && let Some(fallback) = fallback_args_from_current_struct.as_ref()
                && let Some(arg) = fallback.get(idx)
            {
                recovered_args[idx] = Some(arg.clone());
                continue;
            }
            if param_in_scope {
                recovered_args[idx] = Some(param.clone());
            }
        }

        self.fill_unresolved_struct_literal_type_args_from_iterator_owner(
            param_kinds,
            &mut recovered_args,
        );
        let recovered_args: Vec<String> = recovered_args.into_iter().collect::<Option<Vec<_>>>()?;
        if recovered_args.is_empty() {
            return None;
        }
        if recovered_args.len() != params.len() {
            return None;
        }
        if self.should_elide_shadowed_current_struct_local_recovered_args(
            &type_key,
            params,
            &recovered_args,
        ) {
            return Some(mapped_path_base.clone());
        }
        if struct_expr.path.segments.len() == 1 {
            let local_name = local_name.unwrap_or_else(|| last_seg.ident.to_string());
            if self.current_struct_assoc_alias_exists(&local_name) {
                return Some(escape_cpp_keyword(&local_name));
            }
        }
        Some(format!(
            "{}<{}>",
            mapped_path_base,
            recovered_args.join(", ")
        ))
    }

    pub(super) fn recover_omitted_owner_generic_args_from_scope(
        &self,
        owner_path: &syn::Path,
    ) -> Option<Vec<String>> {
        let ordered_scope_params = self.ordered_type_params_in_scope();
        let declared_params = self.declared_type_params_for_path(owner_path);
        if declared_params.is_none() {
            let owner_name = owner_path
                .segments
                .last()
                .map(|seg| seg.ident.to_string())
                .unwrap_or_default();
            let known_arity = match owner_name.as_str() {
                "Either" | "EitherOrBoth" => Some(2usize),
                _ => None,
            }?;
            if ordered_scope_params.len() < known_arity {
                return None;
            }
            return Some(ordered_scope_params.into_iter().take(known_arity).collect());
        }

        let params = declared_params?;
        if params.is_empty() {
            return None;
        }
        let defaults = self.declared_type_param_defaults_for_path(owner_path);
        let can_use_ordered_scope_fallback = ordered_scope_params.len() >= params.len();
        let mut ordered_scope_idx = 0usize;
        let mut resolved_type_defaults: HashMap<String, syn::Type> = HashMap::new();

        let mut recovered_args: Vec<String> = Vec::new();
        for (idx, param) in params.iter().enumerate() {
            if let Some(default) = defaults
                .and_then(|all| all.get(idx))
                .and_then(|entry| entry.as_ref())
            {
                let mapped_default = match default {
                    GenericParamDefault::Type(t) => {
                        let substituted = if resolved_type_defaults.is_empty() {
                            t.clone()
                        } else {
                            self.substitute_type_params_in_type(t, &resolved_type_defaults)
                        };
                        let mapped = self.map_type(&substituted);
                        resolved_type_defaults.insert(param.clone(), substituted);
                        mapped
                    }
                    GenericParamDefault::Const(c) => self.emit_expr_to_string(c),
                };
                recovered_args.push(mapped_default);
                continue;
            }
            if self.is_type_param_in_scope(param) {
                recovered_args.push(param.clone());
                if let Ok(ty) = syn::parse_str::<syn::Type>(param) {
                    resolved_type_defaults.insert(param.clone(), ty);
                }
                continue;
            }
            // Cluster A: in absorbed-method context, the declared param
            // names (e.g. K, V) of the owner type are dropped from
            // scope. Look them up in the current method's structural
            // decomposition — if the same name appears as an impl-block
            // generic at a tracked inner-struct position, recover via
            // `typename __TemplateArgs<host_param>::arg_<pos>`. This
            // prevents the loose ordered-scope fallback from picking
            // unrelated in-scope idents like `A` (method-local
            // allocator) or `Node` (host class param).
            if let Some(dep_path) = self.cluster_a_dependent_path_for_dropped_generic(param) {
                recovered_args.push(dep_path);
                continue;
            }
            if can_use_ordered_scope_fallback
                && let Some(scope_param) = ordered_scope_params.get(ordered_scope_idx)
            {
                recovered_args.push(scope_param.clone());
                ordered_scope_idx += 1;
                if let Ok(ty) = syn::parse_str::<syn::Type>(scope_param) {
                    resolved_type_defaults.insert(param.clone(), ty);
                }
                continue;
            }
            return None;
        }
        if recovered_args.len() != params.len() {
            return None;
        }
        Some(recovered_args)
    }

    pub(super) fn resolve_owner_sibling_assoc_type(
        &self,
        owner_cpp: &str,
        assoc_name: &str,
    ) -> Option<String> {
        let owner = owner_cpp
            .trim()
            .trim_start_matches("typename ")
            .trim()
            .trim_start_matches("::");
        if owner.is_empty() || owner == "auto" || owner.contains("/* TODO") {
            return None;
        }
        let owner_base = owner.split('<').next().unwrap_or(owner).trim();
        let Some((namespace, _owner_tail)) = owner_base.rsplit_once("::") else {
            return None;
        };
        if namespace.is_empty() {
            return None;
        }
        let candidate = format!("{}::{}", namespace, assoc_name);
        let escaped_candidate = candidate
            .split("::")
            .filter(|seg| !seg.is_empty())
            .map(escape_cpp_keyword)
            .collect::<Vec<String>>()
            .join("::");
        if self.local_declared_types.contains(&candidate)
            || self.local_declared_types.contains(&escaped_candidate)
            || self.type_key_is_declared_alias(&candidate)
            || self.type_key_is_declared_alias(&escaped_candidate)
        {
            if owner_cpp.trim_start().starts_with("::") {
                Some(format!("::{}", escaped_candidate))
            } else {
                Some(escaped_candidate)
            }
        } else {
            None
        }
    }

    pub(super) fn resolve_trait_static_call_type_param_for_segments(
        &self,
        segments: &[String],
    ) -> Option<String> {
        if segments.len() < 2 {
            return None;
        }
        if self
            .trait_static_call_has_receiver_for_segments(segments)
            .is_some_and(|has_receiver| has_receiver)
        {
            return None;
        }
        let trait_idx = segments.len() - 2;
        let trait_name = &segments[trait_idx];
        self.resolve_unique_trait_bound_type_param(trait_name)
    }

    pub(super) fn resolve_trait_static_call_owner_in_current_context(
        &self,
        segments: &[String],
    ) -> Option<String> {
        if segments.len() < 2 {
            return None;
        }
        let trait_idx = segments.len() - 2;
        let trait_name = &segments[trait_idx];
        let scoped_trait = segments[..=trait_idx].join("::");
        let trait_path_is_known = self
            .resolve_unique_trait_bound_type_param(trait_name)
            .is_some()
            || self.trait_method_has_receiver.contains_key(trait_name)
            || self.trait_method_has_receiver.contains_key(&scoped_trait)
            || self
                .trait_method_has_receiver
                .keys()
                .any(|key| key.starts_with(&format!("{}::", trait_name)));
        if !trait_path_is_known {
            return None;
        }
        if self
            .trait_static_call_has_receiver_for_segments(segments)
            .is_some_and(|has_receiver| has_receiver)
        {
            return None;
        }
        let method_name = segments.last()?;
        if matches!(method_name.as_str(), "deserialize" | "serialize") {
            // Rewriting trait static calls for serialize/deserialize to the current
            // impl owner frequently creates self-recursive calls in adapter impls.
            // Keep these on trait-dispatch lowering paths instead.
            return None;
        }
        let current_struct = self.current_struct.as_ref()?;
        let has_current_owner_method = self
            .lookup_owner_method_has_receiver(current_struct, method_name)
            .is_some_and(|has_receiver| !has_receiver);
        if has_current_owner_method {
            Some(current_struct.clone())
        } else {
            None
        }
    }

    pub(super) fn resolve_unique_trait_bound_type_param(&self, trait_name: &str) -> Option<String> {
        self.trait_bound_type_param_scopes
            .iter()
            .rev()
            .find_map(|scope| {
                if let Some(found) = scope.get(trait_name) {
                    return Some(found.clone());
                }
                let suffix = format!("::{}", trait_name);
                let mut suffix_matches: Vec<String> = scope
                    .iter()
                    .filter_map(|(bound_trait, bound_param)| {
                        (bound_trait.ends_with(&suffix)).then_some(bound_param.clone())
                    })
                    .collect();
                suffix_matches.sort();
                suffix_matches.dedup();
                if suffix_matches.len() == 1 {
                    Some(suffix_matches[0].clone())
                } else {
                    None
                }
            })
    }

    pub(super) fn infer_type_param_static_conversion_call_return_type(
        &self,
        expr: &syn::Expr,
    ) -> Option<syn::Type> {
        let syn::Expr::Call(call) = self.peel_paren_group_expr(expr) else {
            return None;
        };
        if call.args.len() != 1 {
            return None;
        }
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        let owner_path = self.type_param_static_conversion_owner_path(&path_expr.path)?;
        Some(syn::Type::Path(syn::TypePath {
            qself: None,
            path: owner_path,
        }))
    }

    pub(super) fn infer_serde_error_trait_static_call_return_type(
        &self,
        expected_ty: Option<&syn::Type>,
    ) -> Option<syn::Type> {
        if let Some(err_ty) = expected_ty.and_then(|ty| self.expected_result_type_arg(Some(ty), 1))
        {
            return Some(err_ty.clone());
        }
        if let Some(return_hint) = self.current_return_type_hint()
            && let Some(err_ty) = self.expected_result_type_arg(Some(return_hint), 1)
        {
            return Some(err_ty.clone());
        }
        if self.has_concrete_error_module_type() {
            return syn::parse_str::<syn::Type>("error::Error").ok();
        }
        if self.is_type_param_in_scope("E") {
            return syn::parse_str::<syn::Type>("E").ok();
        }
        if self.is_type_param_in_scope("Error") {
            return syn::parse_str::<syn::Type>("Error").ok();
        }
        syn::parse_str::<syn::Type>("error::Error").ok()
    }

    pub(super) fn resolve_trait_static_call_owner_from_return_hint(&self, trait_name: &str) -> Option<String> {
        let return_hint = self.current_return_type_hint()?;
        if let Some(owner) = self.mapped_trait_static_owner_from_type_hint(return_hint, trait_name)
        {
            return Some(owner);
        }
        if let Some(err_ty) = self.expected_result_type_arg(Some(return_hint), 1)
            && let Some(owner) = self.mapped_trait_static_owner_from_type_hint(err_ty, trait_name)
        {
            return Some(owner);
        }
        None
    }

    pub(super) fn recover_deserializer_error_template_arg_cpp(&self) -> Option<String> {
        let is_valid = |mapped: &str| {
            mapped != "auto"
                && !mapped.contains("/* TODO")
                && !type_string_has_auto_placeholder(mapped)
        };

        if let Some(return_hint) = self.current_return_type_hint() {
            if let Some(err_ty) = self.expected_result_type_arg(Some(return_hint), 1) {
                let mapped_err = self
                    .map_type(err_ty)
                    .trim_start_matches("typename ")
                    .to_string();
                if is_valid(&mapped_err) {
                    return Some(mapped_err);
                }
            }
            if let Some(mapped_err) =
                self.mapped_trait_static_owner_from_type_hint(return_hint, "Error")
                && is_valid(&mapped_err)
            {
                return Some(mapped_err);
            }
        }

        if self.is_type_param_in_scope("E") {
            return Some("E".to_string());
        }
        if self.is_type_param_in_scope("Error") {
            return Some("Error".to_string());
        }

        if let Some(current_struct) = self.current_struct.as_ref() {
            let scoped_current = self.scoped_type_key(current_struct);
            for key in [current_struct.as_str(), scoped_current.as_str()] {
                if let Some(params) = self.declared_type_params.get(key) {
                    if params.iter().any(|p| p == "E") {
                        return Some("E".to_string());
                    }
                    if params.iter().any(|p| p == "Error") {
                        return Some("Error".to_string());
                    }
                }
            }
            if current_struct
                .rsplit("::")
                .next()
                .is_some_and(|tail| tail.ends_with("Deserializer"))
            {
                return Some("Error".to_string());
            }
        }

        None
    }

    /// Try to resolve an associated type like `<TestFlags as Trait>::Internal`
    /// by looking up `type Internal = ...` in the impl blocks collected for the type.
    pub(super) fn resolve_assoc_type_from_impl_blocks(
        &self,
        type_name: &str,
        assoc_name: &str,
    ) -> Option<String> {
        let mut candidates = vec![type_name.to_string(), self.scoped_type_key(type_name)];
        if let Some(tail) = type_name.rsplit("::").next()
            && tail != type_name
        {
            candidates.push(tail.to_string());
            candidates.push(self.scoped_type_key(tail));
        }
        candidates.sort();
        candidates.dedup();

        let lookup_in_items = |items: &[syn::ImplItem]| -> Option<String> {
            for item in items {
                if let syn::ImplItem::Type(assoc_type) = item
                    && assoc_type.ident == assoc_name
                {
                    return Some(self.map_type(&assoc_type.ty));
                }
            }
            None
        };

        for key in &candidates {
            if let Some(items) = self.impl_blocks.get(key)
                && let Some(mapped) = lookup_in_items(items)
            {
                return Some(mapped);
            }
            if let Some(items) = self.consumed_impl_blocks.get(key)
                && let Some(mapped) = lookup_in_items(items)
            {
                return Some(mapped);
            }
        }

        // Also try all keys that end with ::TypeName (or ::TailName) for
        // impls recorded under fully-qualified owners.
        let mut suffixes = vec![format!("::{}", type_name)];
        if let Some(tail) = type_name.rsplit("::").next()
            && tail != type_name
        {
            suffixes.push(format!("::{}", tail));
        }
        for suffix in suffixes {
            for (key, items) in &self.impl_blocks {
                if (key.ends_with(&suffix) || key == type_name)
                    && let Some(mapped) = lookup_in_items(items)
                {
                    return Some(mapped);
                }
            }
            for (key, items) in &self.consumed_impl_blocks {
                if (key.ends_with(&suffix) || key == type_name)
                    && let Some(mapped) = lookup_in_items(items)
                {
                    return Some(mapped);
                }
            }
        }

        None
    }

    pub(super) fn resolve_param_cpp_type(&self, ty: &syn::Type) -> String {
        let mapped = self.map_type(ty);
        if let Some(softened) = self.soften_dyn_trait_object_param_type(ty, &mapped) {
            return softened;
        }
        self.soften_incomplete_nominal_param_type(ty, &mapped)
            .unwrap_or(mapped)
    }
}
