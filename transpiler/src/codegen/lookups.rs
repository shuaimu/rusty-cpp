use super::*;

impl CodeGen {
    pub(super) fn find_deterministic_cycle_path_for_component(
        component: &[usize],
        edges: &[Vec<usize>],
        pos_to_name: &[String],
    ) -> Vec<String> {
        if component.is_empty() {
            return Vec::new();
        }
        if component.len() == 1 {
            let node = component[0];
            if edges.get(node).is_some_and(|nexts| nexts.contains(&node)) {
                let name = pos_to_name
                    .get(node)
                    .cloned()
                    .unwrap_or_else(|| format!("_node_{}", node));
                return vec![name.clone(), name];
            }
            return vec![
                pos_to_name
                    .get(node)
                    .cloned()
                    .unwrap_or_else(|| format!("_node_{}", node)),
            ];
        }

        let component_set: HashSet<usize> = component.iter().copied().collect();
        let mut starts = component.to_vec();
        starts.sort_by(|a, b| {
            pos_to_name
                .get(*a)
                .map(|s| s.as_str())
                .unwrap_or("")
                .cmp(pos_to_name.get(*b).map(|s| s.as_str()).unwrap_or(""))
                .then_with(|| a.cmp(b))
        });

        for start in starts {
            let mut path = vec![start];
            let mut in_path = HashSet::from([start]);
            if let Some(found) = Self::dfs_cycle_path_within_component(
                start,
                start,
                edges,
                &component_set,
                pos_to_name,
                &mut path,
                &mut in_path,
            ) {
                return found
                    .into_iter()
                    .map(|pos| {
                        pos_to_name
                            .get(pos)
                            .cloned()
                            .unwrap_or_else(|| format!("_node_{}", pos))
                    })
                    .collect();
            }
        }

        // Fallback should be unreachable for true SCC cycles; keep deterministic anyway.
        let mut names: Vec<String> = component
            .iter()
            .map(|&pos| {
                pos_to_name
                    .get(pos)
                    .cloned()
                    .unwrap_or_else(|| format!("_node_{}", pos))
            })
            .collect();
        names.sort();
        names.dedup();
        if let Some(first) = names.first().cloned() {
            names.push(first);
        }
        names
    }

    pub(super) fn extract_cpp_scoped_path_tokens(spelling: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut flush = |buf: &mut String, out: &mut Vec<String>| {
            if buf.contains("::") {
                let trimmed = buf.trim_matches(':');
                if !trimmed.is_empty() {
                    out.push(buf.clone());
                }
            }
            buf.clear();
        };
        for ch in spelling.chars() {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == ':' {
                cur.push(ch);
            } else {
                flush(&mut cur, &mut out);
            }
        }
        flush(&mut cur, &mut out);
        out.sort();
        out.dedup();
        out
    }

    /// Cluster C helper: extract the impl-block's host-type template args
    /// as a vector of normalized strings (e.g. `["::marker::Immut", "K", "V"]`).
    /// Returns None if not a `Path<...>` self-type.
    pub(super) fn extract_class_template_args(ty: &syn::Type) -> Option<Vec<String>> {
        let syn::Type::Path(p) = ty else {
            return None;
        };
        let last = p.path.segments.last()?;
        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
            return Some(Vec::new());
        };
        let mut out = Vec::with_capacity(args.args.len());
        for arg in &args.args {
            match arg {
                syn::GenericArgument::Type(t) => {
                    out.push(Self::type_to_normalized_string(t));
                }
                syn::GenericArgument::Lifetime(_) => continue,
                _ => out.push(arg.to_token_stream().to_string()),
            }
        }
        Some(out)
    }

    pub(super) fn lookup_owner_method_has_receiver(&self, owner: &str, method_name: &str) -> Option<bool> {
        let mut keys = Vec::new();
        keys.push(Self::owner_method_key(owner, method_name));
        if let Some(last) = owner.rsplit("::").next() {
            keys.push(Self::owner_method_key(last, method_name));
        }
        let mut dedup = HashSet::new();
        keys.retain(|key| dedup.insert(key.clone()));

        let mut merged: Option<bool> = None;
        for key in keys {
            let Some(entry) = self.owner_method_has_receiver.get(&key) else {
                continue;
            };
            let Some(value) = *entry else {
                return None;
            };
            match merged {
                Some(existing) if existing != value => return None,
                Some(_) => {}
                None => merged = Some(value),
            }
        }
        merged
    }

    pub(super) fn lookup_owner_method_has_receiver_for_owner_key(
        &self,
        owner_key: &str,
        method_name: &str,
    ) -> Option<Option<bool>> {
        let key = Self::owner_method_key(owner_key, method_name);
        self.owner_method_has_receiver.get(&key).copied()
    }

    pub(super) fn lookup_alias_inherent_owner_method_has_receiver_for_owner_key(
        &self,
        owner_key: &str,
        method_name: &str,
    ) -> Option<Option<bool>> {
        let key = Self::owner_method_key(owner_key, method_name);
        self.alias_inherent_owner_method_has_receiver
            .get(&key)
            .copied()
    }

    pub(super) fn lookup_owner_method_has_receiver_from_owner_path(
        &self,
        owner_path: Option<&syn::Path>,
        owner_name: &str,
        method_name: &str,
    ) -> Option<bool> {
        let owner_path = owner_path?;
        let owner_keys = self.owner_path_to_candidate_owner_keys(owner_path, owner_name);
        if owner_keys.is_empty() {
            return None;
        }

        let mut ordered_keys: Vec<String> = Vec::new();
        if owner_path.segments.len() == 1 && !self.module_stack.is_empty() {
            let current_prefix = format!("{}::", self.module_stack.join("::"));
            for key in owner_keys
                .iter()
                .filter(|key| key.starts_with(&current_prefix))
            {
                ordered_keys.push(key.clone());
            }
        }
        ordered_keys.extend(owner_keys);
        let mut dedup = HashSet::new();
        ordered_keys.retain(|key| dedup.insert(key.clone()));

        let mut merged: Option<bool> = None;
        let mut saw_any = false;
        for owner_key in ordered_keys {
            let Some(entry) =
                self.lookup_owner_method_has_receiver_for_owner_key(&owner_key, method_name)
            else {
                continue;
            };
            saw_any = true;
            let Some(value) = entry else {
                return None;
            };
            match merged {
                Some(existing) if existing != value => return None,
                Some(_) => {}
                None => merged = Some(value),
            }
        }
        if saw_any { merged } else { None }
    }

    pub(super) fn lookup_owner_method_type_param_names<'a>(
        &'a self,
        owner: &str,
        method_name: &str,
    ) -> Option<&'a Vec<String>> {
        for key in self.owner_method_lookup_keys(owner, method_name) {
            if let Some(params) = self.function_type_param_names.get(&key)
                && !params.is_empty()
            {
                return Some(params);
            }
        }
        None
    }

    pub(super) fn lookup_owner_method_arg_expected_types_for_template_inference<'a>(
        &'a self,
        owner: &str,
        method_name: &str,
    ) -> Option<&'a Vec<Option<syn::Type>>> {
        for key in self.owner_method_lookup_keys(owner, method_name) {
            if let Some(expected_types) = self.function_arg_expected_types.get(&key) {
                return Some(expected_types);
            }
        }
        None
    }

    pub(super) fn lookup_owner_method_return_type_for_template_inference<'a>(
        &'a self,
        owner: &str,
        method_name: &str,
    ) -> Option<&'a syn::Type> {
        for key in self.owner_method_lookup_keys(owner, method_name) {
            if let Some(Some(ret_ty)) = self.function_return_types.get(&key) {
                return Some(ret_ty);
            }
        }
        None
    }

    pub(super) fn lookup_function_arg_pass_style(
        &self,
        func: &syn::Expr,
        arg_idx: usize,
    ) -> Option<ArgPassStyle> {
        let syn::Expr::Path(path_expr) = func else {
            return None;
        };
        for key in self.call_path_candidates(&path_expr.path) {
            if let Some(styles) = self.function_arg_pass_styles.get(&key) {
                if let Some(style) = styles.get(arg_idx).copied() {
                    return Some(style);
                }
            }
        }
        let func_name = path_expr
            .path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default();
        if func_name.is_empty() {
            return None;
        }
        let suffix = format!("::{}", func_name);
        let mut fallback_keys: Vec<&String> = self
            .function_arg_pass_styles
            .keys()
            .filter(|key| key.as_str() == func_name || key.ends_with(&suffix))
            .collect();
        fallback_keys.sort();
        fallback_keys.dedup();
        if fallback_keys.len() == 1
            && let Some(styles) = self.function_arg_pass_styles.get(fallback_keys[0])
            && let Some(style) = styles.get(arg_idx).copied()
        {
            return Some(style);
        }
        None
    }

    pub(super) fn lookup_function_arg_expected_type<'a>(
        &'a self,
        func: &syn::Expr,
        arg_idx: usize,
    ) -> Option<&'a syn::Type> {
        let syn::Expr::Path(path_expr) = func else {
            return None;
        };
        for key in self.call_path_candidates(&path_expr.path) {
            if let Some(expected) = self.function_arg_expected_types.get(&key) {
                if let Some(Some(ty)) = expected.get(arg_idx) {
                    return Some(ty);
                }
            }
        }
        let func_name = path_expr
            .path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default();
        if func_name.is_empty() {
            return None;
        }
        let suffix = format!("::{}", func_name);
        let mut fallback_keys: Vec<&String> = self
            .function_arg_expected_types
            .keys()
            .filter(|key| key.as_str() == func_name || key.ends_with(&suffix))
            .collect();
        fallback_keys.sort();
        fallback_keys.dedup();
        if fallback_keys.len() == 1
            && let Some(expected) = self.function_arg_expected_types.get(fallback_keys[0])
            && let Some(Some(ty)) = expected.get(arg_idx)
        {
            return Some(ty);
        }
        None
    }

    pub(super) fn lookup_function_arg_expected_type_for_call(
        &self,
        call: &syn::ExprCall,
        arg_idx: usize,
        substitutions: Option<&HashMap<String, syn::Type>>,
    ) -> Option<syn::Type> {
        let expected = self.lookup_function_arg_expected_type(call.func.as_ref(), arg_idx)?;
        match substitutions {
            Some(substitutions) if !substitutions.is_empty() => {
                Some(self.substitute_type_params_in_type(expected, substitutions))
            }
            _ => Some(expected.clone()),
        }
    }

    pub(super) fn lookup_function_type_param_names<'a>(&'a self, func: &syn::Expr) -> Option<&'a Vec<String>> {
        let syn::Expr::Path(path_expr) = func else {
            return None;
        };
        for key in self.call_path_candidates(&path_expr.path) {
            if let Some(params) = self.function_type_param_names.get(&key) {
                if !params.is_empty() {
                    return Some(params);
                }
            }
        }
        None
    }

    /// Like `lookup_function_type_param_names`, but falls back to a unique
    /// leaf-name match when the current-module candidates miss — so a bare
    /// `guard(...)` call resolves to a cross-module `scopeguard::guard` (function
    /// names are globally unique in c2rust-style ports). The uniqueness guard
    /// avoids guessing across same-named functions.
    pub(super) fn lookup_function_type_param_names_with_import_fallback<'a>(
        &'a self,
        func: &syn::Expr,
    ) -> Option<&'a Vec<String>> {
        if let Some(params) = self.lookup_function_type_param_names(func) {
            return Some(params);
        }
        let syn::Expr::Path(path_expr) = func else {
            return None;
        };
        let leaf = path_expr.path.segments.last()?.ident.to_string();
        let suffix = format!("::{}", leaf);
        let mut found: Option<&Vec<String>> = None;
        let mut count = 0usize;
        for (key, params) in &self.function_type_param_names {
            if (key == &leaf || key.ends_with(&suffix)) && !params.is_empty() {
                count += 1;
                if count > 1 {
                    return None;
                }
                found = Some(params);
            }
        }
        found
    }

    /// Like `lookup_function_arg_expected_type`, but falls back to a unique
    /// leaf-name match (same rationale as the type-param-names fallback).
    pub(super) fn lookup_function_arg_expected_type_with_import_fallback<'a>(
        &'a self,
        func: &syn::Expr,
        arg_idx: usize,
    ) -> Option<&'a syn::Type> {
        if let Some(ty) = self.lookup_function_arg_expected_type(func, arg_idx) {
            return Some(ty);
        }
        let syn::Expr::Path(path_expr) = func else {
            return None;
        };
        let leaf = path_expr.path.segments.last()?.ident.to_string();
        let suffix = format!("::{}", leaf);
        let mut found: Option<&syn::Type> = None;
        let mut count = 0usize;
        for (key, expected) in &self.function_arg_expected_types {
            if key == &leaf || key.ends_with(&suffix) {
                count += 1;
                if count > 1 {
                    return None;
                }
                if let Some(Some(ty)) = expected.get(arg_idx) {
                    found = Some(ty);
                }
            }
        }
        found
    }

    pub(super) fn lookup_function_return_type<'a>(&'a self, func: &syn::Expr) -> Option<&'a syn::Type> {
        let syn::Expr::Path(path_expr) = func else {
            return None;
        };
        for key in self.call_path_candidates(&path_expr.path) {
            if let Some(Some(ty)) = self.function_return_types.get(&key) {
                return Some(ty);
            }
        }
        None
    }

    /// Like `lookup_function_return_type`, but adds a fallback for a bare call to
    /// a function imported via `use` from another module (so the current-module
    /// candidates miss): if exactly ONE recorded crate function has this leaf
    /// name, use its return type. Function names are globally unique in c2rust
    /// ports, so this resolves `yaml_malloc()` called from `dumper`/`parser`/…
    /// back to the `api::yaml_malloc` record. The uniqueness guard avoids
    /// guessing when two modules declare same-named functions.
    pub(super) fn lookup_fn_return_type_with_import_fallback(
        &self,
        func: &syn::Expr,
    ) -> Option<syn::Type> {
        if let Some(ty) = self.lookup_function_return_type(func) {
            return Some(ty.clone());
        }
        let syn::Expr::Path(path_expr) = func else {
            return None;
        };
        let leaf = path_expr.path.segments.last()?.ident.to_string();
        let suffix = format!("::{}", leaf);
        let mut found: Option<&syn::Type> = None;
        let mut count = 0usize;
        for (key, ret) in &self.function_return_types {
            if (key == &leaf || key.ends_with(&suffix))
                && let Some(ty) = ret
            {
                count += 1;
                if count > 1 {
                    return None;
                }
                found = Some(ty);
            }
        }
        found.cloned()
    }

    pub(super) fn lookup_method_arg_pass_style(
        &self,
        method_name: &str,
        arg_idx: usize,
    ) -> Option<ArgPassStyle> {
        self.method_arg_pass_styles
            .get(method_name)
            .and_then(|styles| styles.get(arg_idx).copied())
    }

    pub(super) fn lookup_method_arg_expected_type<'a>(
        &'a self,
        method_name: &str,
        arg_idx: usize,
    ) -> Option<&'a syn::Type> {
        self.method_arg_expected_types
            .get(method_name)
            .and_then(|expected| expected.get(arg_idx))
            .and_then(|ty| ty.as_ref())
    }

    pub(super) fn lookup_owner_method_arg_expected_type(
        &self,
        owner: &str,
        method_name: &str,
        arg_idx: usize,
        arg_expr: Option<&syn::Expr>,
    ) -> Option<syn::Type> {
        let mut keys = Vec::new();
        keys.push(Self::owner_method_key(owner, method_name));
        if let Some(last) = owner.rsplit("::").next() {
            keys.push(Self::owner_method_key(last, method_name));
        }
        let mut dedup = HashSet::new();
        keys.retain(|key| dedup.insert(key.clone()));
        for key in keys {
            if let Some(variants) = self.owner_method_arg_expected_type_variants.get(&key)
                && let Some(ty) =
                    self.select_owner_method_arg_expected_type_variant(variants, arg_idx, arg_expr)
            {
                return Some(ty);
            }
            if let Some(expected) = self.owner_method_arg_expected_types.get(&key)
                && let Some(Some(ty)) = expected.get(arg_idx)
            {
                return Some(ty.clone());
            }
        }
        None
    }

    pub(super) fn lookup_owner_method_arg_expected_type_for_owner_key(
        &self,
        owner_key: &str,
        method_name: &str,
        arg_idx: usize,
        arg_expr: Option<&syn::Expr>,
    ) -> Option<syn::Type> {
        let key = Self::owner_method_key(owner_key, method_name);
        if let Some(variants) = self.owner_method_arg_expected_type_variants.get(&key)
            && let Some(ty) =
                self.select_owner_method_arg_expected_type_variant(variants, arg_idx, arg_expr)
        {
            return Some(ty);
        }
        if let Some(expected) = self.owner_method_arg_expected_types.get(&key)
            && let Some(Some(ty)) = expected.get(arg_idx)
        {
            return Some(ty.clone());
        }
        None
    }

    pub(super) fn lookup_owner_method_arg_expected_type_from_owner_path(
        &self,
        owner_path: Option<&syn::Path>,
        owner_name: &str,
        method_name: &str,
        arg_idx: usize,
        arg_expr: Option<&syn::Expr>,
    ) -> Option<syn::Type> {
        let owner_path = owner_path?;
        let owner_keys = self.owner_path_to_candidate_owner_keys(owner_path, owner_name);
        if owner_path.segments.len() == 1 && !self.module_stack.is_empty() {
            let current_prefix = format!("{}::", self.module_stack.join("::"));
            for owner_key in owner_keys
                .iter()
                .filter(|key| key.starts_with(&current_prefix))
            {
                if let Some(expected) = self.lookup_owner_method_arg_expected_type_for_owner_key(
                    owner_key,
                    method_name,
                    arg_idx,
                    arg_expr,
                ) {
                    return Some(expected);
                }
            }
        }
        for owner_key in owner_keys {
            if let Some(expected) = self.lookup_owner_method_arg_expected_type_for_owner_key(
                &owner_key,
                method_name,
                arg_idx,
                arg_expr,
            ) {
                return Some(expected);
            }
        }
        None
    }

    pub(super) fn lookup_associated_call_return_type(&self, call: &syn::ExprCall) -> Option<syn::Type> {
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        if path_expr.path.segments.len() < 2 {
            return None;
        }
        let owner_seg_idx = path_expr.path.segments.len().saturating_sub(2);
        let owner_tail = path_expr
            .path
            .segments
            .iter()
            .nth(owner_seg_idx)
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default();
        let owner_full = path_expr
            .path
            .segments
            .iter()
            .take(owner_seg_idx + 1)
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        let method = path_expr
            .path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default();
        if owner_tail.is_empty() || method.is_empty() {
            return None;
        }
        let mut owner_candidates = Vec::new();
        if !owner_full.is_empty() {
            owner_candidates.push(owner_full.clone());
            owner_candidates.push(self.scoped_type_key(&owner_full));
        }
        owner_candidates.push(owner_tail.clone());
        owner_candidates.push(self.scoped_type_key(&owner_tail));
        let mut dedup = HashSet::new();
        owner_candidates.retain(|candidate| dedup.insert(candidate.clone()));

        let mut ret_ty: Option<syn::Type> = None;
        for owner in owner_candidates {
            if let Some(found) = self.lookup_method_return_type_for_owner_key(&owner, &method) {
                ret_ty = Some(found);
                break;
            }
        }
        let receiver_info = call.args.first().and_then(|receiver_expr| {
            self.receiver_owner_name_and_type_substitutions(receiver_expr)
        });
        if ret_ty.is_none()
            && let Some((receiver_owner, _)) = receiver_info.as_ref()
        {
            let mut receiver_candidates = Vec::new();
            receiver_candidates.push(receiver_owner.clone());
            receiver_candidates.push(self.scoped_type_key(receiver_owner));
            if let Some(receiver_tail) = receiver_owner.rsplit("::").next() {
                receiver_candidates.push(receiver_tail.to_string());
                receiver_candidates.push(self.scoped_type_key(receiver_tail));
            }
            let mut receiver_dedup = HashSet::new();
            receiver_candidates.retain(|candidate| receiver_dedup.insert(candidate.clone()));
            for owner in receiver_candidates {
                if let Some(found) = self.lookup_method_return_type_for_owner_key(&owner, &method) {
                    ret_ty = Some(found);
                    break;
                }
            }
        }
        let mut ret_ty = ret_ty?;

        // Resolve `Self` in the return type to the *defining* owner's impl Self
        // type when the call crosses impl boundaries. Without this, `Self`
        // propagates out of this lookup and gets resolved against the *calling*
        // context's `current_struct` by downstream `map_type`. Concretely, for
        // `Node::new(elt) -> Self` called from inside `impl LinkedList<T, A>`,
        // we want `Self` → `Node<T>`, not `LinkedList<T, A>` (the
        // linked_list_port Box::new_in regression). Limited to cross-impl
        // calls (owner_tail != current_struct) so in-impl `Self::method(...)`
        // shapes that downstream code expects to see as `Self` are unaffected.
        // Runs before owner-segment substitutions so explicit generic args at
        // the call site (e.g. `Node::<i32>::new(...)`) still propagate through.
        let cross_impl_call = self
            .current_struct
            .as_deref()
            .is_some_and(|cs| cs.rsplit("::").next() != Some(owner_tail.as_str()));
        if cross_impl_call
            && let Some(owner_self_ty) = self.compose_owner_self_type_for_lookup(&owner_tail)
        {
            let mut self_subs: HashMap<String, syn::Type> = HashMap::new();
            self_subs.insert("Self".to_string(), owner_self_ty);
            ret_ty = self.substitute_type_params_in_type(&ret_ty, &self_subs);
        }

        if let Some(substitutions) =
            self.owner_segment_type_arg_substitutions(&path_expr.path, owner_seg_idx)
        {
            ret_ty = self.substitute_type_params_in_type(&ret_ty, &substitutions);
        }

        // Associated functions that model receiver methods (`Type::method(self, ...)`)
        // should specialize owner type params from the first argument's concrete type.
        // This avoids incorrect fallback substitutions such as `T -> Lazy<T>` for
        // calls like `Lazy::force_mut(&mut lazy)`.
        let receiver_matches_owner = receiver_info
            .as_ref()
            .and_then(|(receiver_owner, _)| receiver_owner.rsplit("::").next())
            .is_some_and(|tail| tail == owner_tail);
        if receiver_matches_owner
            && let Some((_, receiver_substitutions)) = receiver_info.as_ref()
            && !receiver_substitutions.is_empty()
        {
            return Some(self.substitute_type_params_in_type(&ret_ty, receiver_substitutions));
        }

        if receiver_matches_owner {
            // Keep receiver-style associated return shape when owner substitutions
            // are unresolved. This preserves reference-ness (`T&`) and avoids
            // incorrect fallback substitutions like `T -> Lazy<T>`.
            return Some(ret_ty);
        }

        Some(self.substitute_single_unbound_return_type_param_from_call_args(ret_ty, &call.args))
    }

    pub(super) fn lookup_callable_param_bound_arg_intent(
        &self,
        func: &syn::Expr,
        arg_idx: usize,
    ) -> Option<CallableArgPassIntent> {
        let syn::Expr::Path(path_expr) = func else {
            return None;
        };
        if path_expr.path.segments.len() != 1 {
            return None;
        }
        let name = path_expr.path.segments[0].ident.to_string();
        for scope in self.callable_param_bound_scopes.iter().rev() {
            if let Some(meta) = scope.get(&name) {
                return meta.arg_pass_intents.get(arg_idx).copied();
            }
        }
        None
    }

    /// If `block` consists of a single `Self { ... }` (or
    /// `<owner> { ... }`) struct literal — either as a bare tail
    /// expression or via an explicit `return ...;` — return the field
    /// initializers in source order. Otherwise return None.
    ///
    /// Used by the `#[cpp_ctor]` lowering path to translate a Rust
    /// "factory body" into a C++ ctor initializer list.
    pub(super) fn extract_cpp_ctor_struct_literal<'a>(
        block: &'a syn::Block,
        owner_name: &str,
    ) -> Option<&'a syn::ExprStruct> {
        if block.stmts.len() != 1 {
            return None;
        }
        let lit = match &block.stmts[0] {
            syn::Stmt::Expr(syn::Expr::Struct(s), _) => s,
            syn::Stmt::Expr(syn::Expr::Return(r), _) => {
                let Some(returned) = r.expr.as_deref() else {
                    return None;
                };
                match returned {
                    syn::Expr::Struct(s) => s,
                    _ => return None,
                }
            }
            _ => return None,
        };
        let last_seg = lit.path.segments.last()?;
        let lit_owner = last_seg.ident.to_string();
        if lit_owner != "Self" && lit_owner != owner_name {
            return None;
        }
        if lit.rest.is_some() {
            // `Self { f: v, ..base }` — base-update syntax isn't
            // expressible as a ctor init list.
            return None;
        }
        if lit.qself.is_some() {
            return None;
        }
        Some(lit)
    }

    pub(super) fn extract_libtest_should_panic_value(&self, expr: &syn::Expr) -> Option<bool> {
        let expr = self.peel_paren_group_expr(expr);
        match expr {
            syn::Expr::Struct(struct_expr) => {
                for field in &struct_expr.fields {
                    if let syn::Member::Named(name) = &field.member {
                        if name == "should_panic" {
                            return self.extract_should_panic_flag_from_expr(&field.expr);
                        }
                    }
                    if let Some(value) = self.extract_libtest_should_panic_value(&field.expr) {
                        return Some(value);
                    }
                }
                if let Some(rest) = &struct_expr.rest {
                    return self.extract_libtest_should_panic_value(rest);
                }
                None
            }
            syn::Expr::Reference(r) => self.extract_libtest_should_panic_value(&r.expr),
            syn::Expr::Array(arr) => arr
                .elems
                .iter()
                .find_map(|elem| self.extract_libtest_should_panic_value(elem)),
            syn::Expr::Tuple(tuple) => tuple
                .elems
                .iter()
                .find_map(|elem| self.extract_libtest_should_panic_value(elem)),
            _ => self.extract_should_panic_flag_from_expr(expr),
        }
    }

    pub(super) fn extract_should_panic_flag_from_expr(&self, expr: &syn::Expr) -> Option<bool> {
        let expr = self.peel_paren_group_expr(expr);
        match expr {
            syn::Expr::Path(path_expr) => {
                let last = path_expr.path.segments.last()?.ident.to_string();
                match last.as_str() {
                    "No" => Some(false),
                    "Yes" => Some(true),
                    _ => None,
                }
            }
            syn::Expr::Call(call_expr) => {
                let func_expr = self.peel_paren_group_expr(call_expr.func.as_ref());
                let syn::Expr::Path(path_expr) = func_expr else {
                    return None;
                };
                let last = path_expr.path.segments.last()?.ident.to_string();
                match last.as_str() {
                    "No" => Some(false),
                    "Yes" | "YesWithMessage" => Some(true),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub(super) fn extract_pointer_like_expected_type_param_name(&self, ty: &syn::Type) -> Option<String> {
        let ty = self.peel_reference_paren_group_type(ty);
        match ty {
            syn::Type::Ptr(ptr_ty) => self.extract_simple_type_param_name(&ptr_ty.elem),
            syn::Type::Path(tp) => {
                let outer_seg = tp.path.segments.last()?;
                if outer_seg.ident != "add_pointer_t" {
                    return None;
                }
                let syn::PathArguments::AngleBracketed(outer_args) = &outer_seg.arguments else {
                    return None;
                };
                let outer_inner = outer_args.args.iter().find_map(|arg| match arg {
                    syn::GenericArgument::Type(t) => Some(t),
                    _ => None,
                })?;
                if let Some(name) = self.extract_simple_type_param_name(outer_inner) {
                    return Some(name);
                }
                let outer_inner = self.peel_reference_paren_group_type(outer_inner);
                let syn::Type::Path(inner_path) = outer_inner else {
                    return None;
                };
                let inner_seg = inner_path.path.segments.last()?;
                if inner_seg.ident != "add_const_t" {
                    return None;
                }
                let syn::PathArguments::AngleBracketed(inner_args) = &inner_seg.arguments else {
                    return None;
                };
                let inner_inner = inner_args.args.iter().find_map(|arg| match arg {
                    syn::GenericArgument::Type(t) => Some(t),
                    _ => None,
                })?;
                self.extract_simple_type_param_name(inner_inner)
            }
            _ => None,
        }
    }

    /// Extract derive trait names from attributes.
    pub(super) fn extract_derives(&self, attrs: &[syn::Attribute]) -> Vec<String> {
        let mut derives = Vec::new();
        for attr in attrs {
            if attr.path().is_ident("derive") {
                if let syn::Meta::List(list) = &attr.meta {
                    // Parse the token stream for ident names
                    let tokens = list.tokens.to_string();
                    for part in tokens.split(',') {
                        let trimmed = part.trim();
                        if !trimmed.is_empty() {
                            derives.push(trimmed.to_string());
                        }
                    }
                }
            }
        }
        derives
    }

    pub(super) fn extract_single_stmt_expr<'a>(&self, block: &'a syn::Block) -> Option<&'a syn::Expr> {
        if block.stmts.len() != 1 {
            return None;
        }
        match &block.stmts[0] {
            syn::Stmt::Expr(expr, _) => Some(expr),
            _ => None,
        }
    }

    pub(super) fn extract_candidate_local_name_for_hint_expr(&self, expr: &syn::Expr) -> Option<String> {
        let expr = self.peel_paren_group_expr(expr);
        if let syn::Expr::Reference(reference) = expr {
            return self.extract_candidate_local_name_for_hint_expr(&reference.expr);
        }
        if let syn::Expr::Cast(cast_expr) = expr {
            return self.extract_candidate_local_name_for_hint_expr(&cast_expr.expr);
        }
        if let Some(name) = extract_simple_local_ident(expr) {
            return Some(name);
        }
        if let Some(name) = extract_index_base_ident(expr) {
            return Some(name);
        }
        if let syn::Expr::Unary(unary) = expr {
            return self.extract_candidate_local_name_for_hint_expr(&unary.expr);
        }
        None
    }

    pub(super) fn lookup_associated_call_arg_expected_type_fallback(
        &self,
        call: &syn::ExprCall,
        arg_idx: usize,
        arg_expr: Option<&syn::Expr>,
    ) -> Option<syn::Type> {
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        if path_expr.path.segments.len() < 2 {
            return None;
        }
        let owner_seg_idx = path_expr.path.segments.len().saturating_sub(2);
        let owner_seg = path_expr.path.segments.iter().nth(owner_seg_idx)?;
        let owner = path_expr
            .path
            .segments
            .iter()
            .nth(owner_seg_idx)
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default();
        let method = path_expr
            .path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default();
        let owner_looks_like_type = owner == "Self"
            || owner
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase());
        if !owner_looks_like_type {
            return None;
        }
        let mut owner_path = syn::Path {
            leading_colon: path_expr.path.leading_colon,
            segments: syn::punctuated::Punctuated::new(),
        };
        for seg in path_expr.path.segments.iter().take(owner_seg_idx + 1) {
            owner_path.segments.push(seg.clone());
        }
        // For explicit associated calls (`Type::method(...)`), avoid falling back
        // to method-name-only signatures from unrelated owners. That fallback can
        // cross-wire hints for common names like `new_unchecked` and trigger
        // invalid coercions at call sites.
        let mut expected = self.lookup_owner_method_arg_expected_type_from_owner_path(
            Some(&owner_path),
            &owner,
            &method,
            arg_idx,
            arg_expr,
        );
        if expected.is_none() && owner_path.segments.len() > 1 {
            expected =
                self.lookup_owner_method_arg_expected_type(&owner, &method, arg_idx, arg_expr);
        }
        if expected.is_none() {
            return self.infer_associated_call_arg_expected_type_from_owner(
                owner_seg, &owner, &method, arg_idx, arg_expr,
            );
        }
        let mut expected = expected?;
        if let Some(substitutions) =
            self.owner_segment_type_arg_substitutions(&path_expr.path, owner_seg_idx)
        {
            expected = self.substitute_type_params_in_type(&expected, &substitutions);
        }
        let expected_needs_owner_recovery = self.type_contains_infer(&expected)
            || self.type_contains_in_scope_type_param(&expected)
            || self.type_contains_unbound_single_letter_generic(&expected)
            || matches!(
                self.peel_reference_paren_group_type(&expected),
                syn::Type::Path(tp)
                    if tp.qself.is_none()
                        && tp.path.segments.len() == 1
                        && tp.path.segments[0]
                            .ident
                            .to_string()
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_uppercase())
            );
        if expected_needs_owner_recovery
            && let Some(owner_expected) = self.infer_associated_call_arg_expected_type_from_owner(
                owner_seg, &owner, &method, arg_idx, arg_expr,
            )
        {
            return Some(owner_expected);
        }
        Some(expected)
    }

    pub(super) fn lookup_unique_method_return_type_by_name(&self, method_name: &str) -> Option<syn::Type> {
        let mut unique: Option<syn::Type> = None;
        for (key, ret_ty) in &self.function_return_types {
            let Some(ret_ty) = ret_ty.as_ref() else {
                continue;
            };
            let Some((_, tail)) = key.rsplit_once("::") else {
                continue;
            };
            if tail != method_name {
                continue;
            }
            if self.type_contains_infer(ret_ty)
                || self.type_contains_in_scope_type_param(ret_ty)
                || self.type_contains_unbound_single_letter_generic(ret_ty)
                || self.type_contains_unresolved_placeholder_like(ret_ty)
            {
                continue;
            }
            if let Some(existing) = unique.as_ref() {
                if existing != ret_ty {
                    return None;
                }
            } else {
                unique = Some(ret_ty.clone());
            }
        }
        unique
    }

    pub(super) fn lookup_call_arg_pass_style_for_consumption(
        &self,
        call: &syn::ExprCall,
        arg_idx: usize,
        arg_expr: &syn::Expr,
    ) -> Option<ArgPassStyle> {
        let mut style = self.lookup_function_arg_pass_style(call.func.as_ref(), arg_idx);
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return style;
        };
        let method_name = path_expr
            .path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default();
        if method_name.is_empty() {
            return style;
        }

        if style.is_none() || matches!(style, Some(ArgPassStyle::Mixed)) {
            if let Some(method_style) = self.lookup_method_arg_pass_style(&method_name, arg_idx) {
                if style.is_none() || !matches!(method_style, ArgPassStyle::Mixed) {
                    style = Some(method_style);
                }
            }
        }

        if (style.is_none() || matches!(style, Some(ArgPassStyle::Mixed)))
            && path_expr.path.segments.len() >= 2
        {
            let owner = path_expr
                .path
                .segments
                .iter()
                .nth_back(1)
                .map(|seg| seg.ident.to_string())
                .unwrap_or_default();
            if !owner.is_empty()
                && let Some(expected_ty) = self.lookup_owner_method_arg_expected_type(
                    &owner,
                    &method_name,
                    arg_idx,
                    Some(arg_expr),
                )
            {
                style = Some(self.arg_pass_style_for_type(&expected_ty));
            }
        }

        if (style.is_none() || matches!(style, Some(ArgPassStyle::Mixed)))
            && let Some(expected_ty) = self.lookup_method_arg_expected_type(&method_name, arg_idx)
        {
            style = Some(self.arg_pass_style_for_type(expected_ty));
        }

        if style.is_none() || matches!(style, Some(ArgPassStyle::Mixed)) {
            if let Some(mapped_style) =
                self.lookup_mapped_runtime_call_arg_pass_style_for_consumption(call, arg_idx)
            {
                style = Some(mapped_style);
            }
        }
        if (style.is_none() || matches!(style, Some(ArgPassStyle::Mixed)))
            && arg_idx == 0
            && (method_name.starts_with("into_")
                || matches!(method_name.as_str(), "into_value" | "into_inner" | "take"))
        {
            style = Some(ArgPassStyle::Value);
        }

        style
    }

    pub(super) fn lookup_mapped_runtime_call_arg_pass_style_for_consumption(
        &self,
        call: &syn::ExprCall,
        arg_idx: usize,
    ) -> Option<ArgPassStyle> {
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return None;
        };
        for candidate in self.call_path_candidates(&path_expr.path) {
            let mapped = types::map_function_path(&candidate).unwrap_or(candidate.as_str());
            let style = match mapped {
                "rusty::ptr::read" => (arg_idx == 0).then_some(ArgPassStyle::Pointer),
                "rusty::ptr::write" => match arg_idx {
                    0 => Some(ArgPassStyle::Pointer),
                    1 => Some(ArgPassStyle::Value),
                    _ => None,
                },
                "rusty::ptr::copy" | "rusty::ptr::copy_nonoverlapping" => {
                    if arg_idx <= 1 {
                        Some(ArgPassStyle::Pointer)
                    } else if arg_idx == 2 {
                        Some(ArgPassStyle::Value)
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if style.is_some() {
                return style;
            }
        }
        None
    }

    pub(super) fn extract_tuple_struct_bindings(
        &self,
        elems: &syn::punctuated::Punctuated<syn::Pat, syn::token::Comma>,
    ) -> Vec<String> {
        elems
            .iter()
            .map(|p| match p {
                syn::Pat::Ident(pi) => escape_cpp_keyword(&pi.ident.to_string()),
                syn::Pat::Wild(_) => "_".to_string(),
                _ => "_".to_string(),
            })
            .collect()
    }

    pub(super) fn extract_variant_pattern_enum_name(
        &self,
        path: &syn::Path,
        resolved_cpp_type: &str,
    ) -> Option<String> {
        if path.segments.len() >= 2 {
            let penultimate = path.segments.iter().nth_back(1)?.ident.to_string();
            if penultimate == "Self" {
                return self.current_struct.clone();
            }
            if self.data_enum_name_matches(&penultimate) {
                return Some(penultimate);
            }
            // Keep explicit type-like owners (e.g. `Either::Left`) as the
            // variant enum context even when the owner is imported from another
            // crate and not pre-registered in local enum metadata.
            if penultimate
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
            {
                return Some(penultimate);
            }
        }
        if let Some(last_ident) = path.segments.last().map(|seg| seg.ident.to_string())
            && let Some((enum_name, _)) = self.flattened_data_enum_variant_parts(&last_ident)
        {
            return Some(enum_name);
        }
        if let Some(struct_name) = &self.current_struct {
            if resolved_cpp_type.starts_with(&format!("{}_", struct_name)) {
                return Some(struct_name.clone());
            }
        }
        None
    }

    pub(super) fn extract_for_loop_iterable_root_name(&self, iterable_expr: &syn::Expr) -> Option<String> {
        let mut expr = self.peel_paren_group_expr(iterable_expr);
        if let syn::Expr::Reference(r) = expr {
            if self.is_expr_raw_pointer_like(&r.expr) {
                return None;
            }
            expr = self.peel_paren_group_expr(&r.expr);
        }

        if let syn::Expr::Path(path_expr) = expr {
            if path_expr.path.segments.len() == 1 {
                return Some(path_expr.path.segments[0].ident.to_string());
            }
        }
        None
    }

    pub(super) fn lookup_local_binding_cpp_name(&self, rust_name: &str) -> Option<String> {
        self.local_cpp_bindings
            .iter()
            .rev()
            .find_map(|scope| scope.get(rust_name).cloned())
            .or_else(|| {
                self.param_bindings
                    .iter()
                    .rev()
                    .find_map(|scope| scope.get(rust_name).map(|_| escape_cpp_keyword(rust_name)))
            })
    }

    pub(super) fn lookup_rust_binding_name_for_cpp_name(&self, cpp_name: &str) -> Option<String> {
        self.local_cpp_bindings.iter().rev().find_map(|scope| {
            scope.iter().find_map(|(rust_name, mapped_cpp)| {
                (mapped_cpp == cpp_name).then_some(rust_name.clone())
            })
        })
    }

    pub(super) fn lookup_local_placeholder_type_hint(&self, name: &str) -> Option<&syn::Type> {
        self.local_placeholder_type_hints
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
    }

    pub(super) fn lookup_owner_method_return_type_from_receiver_type(
        &self,
        receiver_ty: &syn::Type,
        method_name: &str,
    ) -> Option<syn::Type> {
        let receiver_ty = self.peel_reference_paren_group_type(receiver_ty);
        let syn::Type::Path(tp) = receiver_ty else {
            return None;
        };

        let mut owner_candidates = Vec::new();
        let full_owner = tp
            .path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if !full_owner.is_empty() {
            owner_candidates.push(full_owner.clone());
        }
        if let Some(last) = tp.path.segments.last() {
            owner_candidates.push(last.ident.to_string());
        }
        if !full_owner.is_empty() {
            owner_candidates.push(self.scoped_type_key(&full_owner));
        }
        if let Some(last) = tp.path.segments.last() {
            owner_candidates.push(self.scoped_type_key(&last.ident.to_string()));
        }

        let mut dedup = HashSet::new();
        owner_candidates.retain(|candidate| dedup.insert(candidate.clone()));
        for owner in owner_candidates {
            if let Some(ret_ty) = self.lookup_method_return_type_for_owner_key(&owner, method_name)
            {
                return Some(ret_ty);
            }
        }

        None
    }

    pub(super) fn lookup_method_return_type_for_owner_key(
        &self,
        owner: &str,
        method_name: &str,
    ) -> Option<syn::Type> {
        if let Some(items) = self.impl_blocks.get(owner)
            && let Some(ret_ty) = Self::lookup_method_return_type_in_items(items, method_name)
        {
            return Some(ret_ty);
        }
        if let Some(items) = self.consumed_impl_blocks.get(owner)
            && let Some(ret_ty) = Self::lookup_method_return_type_in_items(items, method_name)
        {
            return Some(ret_ty);
        }
        None
    }

    pub(super) fn lookup_method_return_type_in_items(
        items: &[syn::ImplItem],
        method_name: &str,
    ) -> Option<syn::Type> {
        for item in items {
            let syn::ImplItem::Fn(method) = item else {
                continue;
            };
            if method.sig.ident != method_name {
                continue;
            }
            let syn::ReturnType::Type(_, ret_ty) = &method.sig.output else {
                return None;
            };
            return Some((**ret_ty).clone());
        }
        None
    }

    pub(super) fn lookup_method_arg_type_for_owner_key(
        &self,
        owner: &str,
        method_name: &str,
        arg_idx: usize,
    ) -> Option<syn::Type> {
        if let Some(items) = self.impl_blocks.get(owner)
            && let Some(arg_ty) = Self::lookup_method_arg_type_in_items(items, method_name, arg_idx)
        {
            return Some(arg_ty);
        }
        if let Some(items) = self.consumed_impl_blocks.get(owner)
            && let Some(arg_ty) = Self::lookup_method_arg_type_in_items(items, method_name, arg_idx)
        {
            return Some(arg_ty);
        }
        None
    }

    pub(super) fn lookup_method_arg_type_in_items(
        items: &[syn::ImplItem],
        method_name: &str,
        arg_idx: usize,
    ) -> Option<syn::Type> {
        for item in items {
            let syn::ImplItem::Fn(method) = item else {
                continue;
            };
            if method.sig.ident != method_name {
                continue;
            }
            let typed_inputs: Vec<&syn::PatType> = method
                .sig
                .inputs
                .iter()
                .filter_map(|input| match input {
                    syn::FnArg::Typed(pat_ty) => Some(pat_ty),
                    _ => None,
                })
                .collect();
            let arg_ty = typed_inputs.get(arg_idx)?;
            return Some((*(arg_ty.ty)).clone());
        }
        None
    }

    pub(super) fn lookup_method_arg_type_from_receiver_type(
        &self,
        receiver: &syn::Expr,
        method_name: &str,
        arg_idx: usize,
    ) -> Option<syn::Type> {
        let receiver_ty = self.infer_simple_expr_type(receiver)?;
        let receiver_ty = self.peel_reference_paren_group_type(&receiver_ty);
        let syn::Type::Path(tp) = receiver_ty else {
            return None;
        };

        let mut owner_candidates = Vec::new();
        let full_owner = tp
            .path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if !full_owner.is_empty() {
            owner_candidates.push(full_owner.clone());
        }
        if let Some(last) = tp.path.segments.last() {
            owner_candidates.push(last.ident.to_string());
        }
        if !full_owner.is_empty() {
            owner_candidates.push(self.scoped_type_key(&full_owner));
        }
        if let Some(last) = tp.path.segments.last() {
            owner_candidates.push(self.scoped_type_key(&last.ident.to_string()));
        }

        let mut dedup = HashSet::new();
        owner_candidates.retain(|candidate| dedup.insert(candidate.clone()));

        let mut arg_ty = owner_candidates.into_iter().find_map(|owner| {
            self.lookup_method_arg_type_for_owner_key(&owner, method_name, arg_idx)
        })?;
        if let Some((_, substitutions)) = self.receiver_owner_name_and_type_substitutions(receiver)
            && !substitutions.is_empty()
        {
            arg_ty = self.substitute_type_params_in_type(&arg_ty, &substitutions);
        }
        Some(arg_ty)
    }

    pub(super) fn extract_pointer_pointee_info_from_type(&self, ty: &syn::Type) -> Option<(syn::Type, bool)> {
        let ty = self.peel_reference_paren_group_type(ty);
        match ty {
            syn::Type::Ptr(ptr) => Some(((*ptr.elem).clone(), ptr.mutability.is_some())),
            syn::Type::Path(tp) => {
                let last = tp.path.segments.last()?;
                let owner = last.ident.to_string();
                let is_mut_ptr = match owner.as_str() {
                    "NonNull" | "Unique" | "MutPtr" => true,
                    "ConstNonNull" | "Ptr" => false,
                    // `std::add_pointer_t<T>` lowers Rust raw-pointer spellings in
                    // some local contexts. Treat it as pointer-like for inference.
                    "add_pointer_t" => true,
                    _ => return None,
                };
                let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
                    return None;
                };
                let pointee = args.args.iter().find_map(|arg| match arg {
                    syn::GenericArgument::Type(t) => Some(t.clone()),
                    _ => None,
                })?;
                Some((pointee, is_mut_ptr))
            }
            _ => None,
        }
    }

    pub(super) fn lookup_current_struct_method_return_type(&self, method_name: &str) -> Option<syn::Type> {
        if let Some(scope) = self.current_struct_method_output_types.last() {
            if let Some(ty) = scope.get(method_name) {
                return Some(ty.clone());
            }
        }
        let struct_name = self.current_struct.as_ref()?;
        let candidates = [struct_name.clone(), self.scoped_type_key(struct_name)];
        for key in candidates {
            let Some(items) = self.impl_blocks.get(&key) else {
                continue;
            };
            for item in items {
                let syn::ImplItem::Fn(method) = item else {
                    continue;
                };
                if method.sig.ident != method_name {
                    continue;
                }
                let syn::ReturnType::Type(_, ret_ty) = &method.sig.output else {
                    return None;
                };
                return Some((**ret_ty).clone());
            }
        }
        None
    }

    pub(super) fn extract_callable_return_type_from_type(&self, ty: &syn::Type) -> Option<syn::Type> {
        let ty = self.peel_reference_paren_group_type(ty);
        match ty {
            syn::Type::Path(tp) => {
                let seg = tp.path.segments.last()?;
                // Function-like path surfaces can appear either as:
                // - SafeFn<Ret(Args...)>
                // - UnsafeFn<Ret(Args...)>
                // - Function<Ret(Args...)>
                // - Fn/FnMut/FnOnce(Args...) -> Ret
                let seg_name = seg.ident.to_string();
                if let syn::PathArguments::Parenthesized(args) = &seg.arguments {
                    return match &args.output {
                        syn::ReturnType::Type(_, ret_ty) => Some((**ret_ty).clone()),
                        syn::ReturnType::Default => None,
                    };
                }
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if matches!(
                        seg_name.as_str(),
                        "SafeFn" | "UnsafeFn" | "Function" | "function"
                    ) {
                        let sig_ty = args.args.iter().find_map(|arg| match arg {
                            syn::GenericArgument::Type(t) => Some(t),
                            _ => None,
                        })?;
                        return self.extract_callable_return_type_from_type(sig_ty);
                    }
                }
                if tp.qself.is_none()
                    && tp.path.segments.len() == 1
                    && matches!(seg.arguments, syn::PathArguments::None)
                {
                    let type_param = seg.ident.to_string();
                    if self.is_type_param_in_scope(&type_param) {
                        return self.lookup_callable_return_type_for_type_param(&type_param);
                    }
                }
                None
            }
            syn::Type::TraitObject(trait_obj) => trait_obj.bounds.iter().find_map(|bound| {
                let syn::TypeParamBound::Trait(trait_bound) = bound else {
                    return None;
                };
                let seg = trait_bound.path.segments.last()?;
                if !matches!(seg.ident.to_string().as_str(), "Fn" | "FnMut" | "FnOnce") {
                    return None;
                }
                let syn::PathArguments::Parenthesized(args) = &seg.arguments else {
                    return None;
                };
                match &args.output {
                    syn::ReturnType::Type(_, ret_ty) => Some((**ret_ty).clone()),
                    syn::ReturnType::Default => None,
                }
            }),
            syn::Type::ImplTrait(impl_trait) => impl_trait.bounds.iter().find_map(|bound| {
                let syn::TypeParamBound::Trait(trait_bound) = bound else {
                    return None;
                };
                let seg = trait_bound.path.segments.last()?;
                if !matches!(seg.ident.to_string().as_str(), "Fn" | "FnMut" | "FnOnce") {
                    return None;
                }
                let syn::PathArguments::Parenthesized(args) = &seg.arguments else {
                    return None;
                };
                match &args.output {
                    syn::ReturnType::Type(_, ret_ty) => Some((**ret_ty).clone()),
                    syn::ReturnType::Default => None,
                }
            }),
            syn::Type::BareFn(bare_fn) => match &bare_fn.output {
                syn::ReturnType::Type(_, ret_ty) => Some((**ret_ty).clone()),
                syn::ReturnType::Default => None,
            },
            syn::Type::Paren(paren) => self.extract_callable_return_type_from_type(&paren.elem),
            syn::Type::Group(group) => self.extract_callable_return_type_from_type(&group.elem),
            _ => None,
        }
    }

    pub(super) fn extract_callable_param_count_from_type(&self, ty: &syn::Type) -> Option<usize> {
        let ty = self.peel_reference_paren_group_type(ty);
        match ty {
            syn::Type::Path(tp) => {
                let seg = tp.path.segments.last()?;
                let seg_name = seg.ident.to_string();
                if let syn::PathArguments::Parenthesized(args) = &seg.arguments {
                    return Some(args.inputs.len());
                }
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments
                    && matches!(
                        seg_name.as_str(),
                        "SafeFn" | "UnsafeFn" | "Function" | "function"
                    )
                {
                    let sig_ty = args.args.iter().find_map(|arg| match arg {
                        syn::GenericArgument::Type(t) => Some(t),
                        _ => None,
                    })?;
                    return self.extract_callable_param_count_from_type(sig_ty);
                }
                None
            }
            syn::Type::TraitObject(trait_obj) => trait_obj.bounds.iter().find_map(|bound| {
                let syn::TypeParamBound::Trait(trait_bound) = bound else {
                    return None;
                };
                let seg = trait_bound.path.segments.last()?;
                if !matches!(seg.ident.to_string().as_str(), "Fn" | "FnMut" | "FnOnce") {
                    return None;
                }
                let syn::PathArguments::Parenthesized(args) = &seg.arguments else {
                    return None;
                };
                Some(args.inputs.len())
            }),
            syn::Type::ImplTrait(impl_trait) => impl_trait.bounds.iter().find_map(|bound| {
                let syn::TypeParamBound::Trait(trait_bound) = bound else {
                    return None;
                };
                let seg = trait_bound.path.segments.last()?;
                if !matches!(seg.ident.to_string().as_str(), "Fn" | "FnMut" | "FnOnce") {
                    return None;
                }
                let syn::PathArguments::Parenthesized(args) = &seg.arguments else {
                    return None;
                };
                Some(args.inputs.len())
            }),
            syn::Type::BareFn(bare_fn) => Some(bare_fn.inputs.len()),
            syn::Type::Paren(paren) => self.extract_callable_param_count_from_type(&paren.elem),
            syn::Type::Group(group) => self.extract_callable_param_count_from_type(&group.elem),
            _ => None,
        }
    }

    pub(super) fn extract_constructor_pair_from_if<'a>(
        &self,
        if_expr: &'a syn::ExprIf,
    ) -> Option<(String, &'a syn::Expr, String, &'a syn::Expr)> {
        let then_expr = self.extract_single_expr_from_block(&if_expr.then_branch)?;
        let (_, else_expr) = if_expr.else_branch.as_ref()?;
        let else_expr = self.extract_value_expr(else_expr)?;
        let (lhs_name, lhs_arg) = self.extract_constructor_call_expr(then_expr)?;
        let (rhs_name, rhs_arg) = self.extract_constructor_call_expr(else_expr)?;
        Some((lhs_name, lhs_arg, rhs_name, rhs_arg))
    }

    pub(super) fn extract_constructor_pair_from_match<'a>(
        &self,
        match_expr: &'a syn::ExprMatch,
    ) -> Option<(String, &'a syn::Expr, String, &'a syn::Expr)> {
        let mut left: Option<(String, &'a syn::Expr)> = None;
        let mut right: Option<(String, &'a syn::Expr)> = None;

        for arm in &match_expr.arms {
            let body_expr = self.extract_value_expr(&arm.body)?;
            if let Some((ctor_name, ctor_arg)) = self.extract_constructor_call_expr(body_expr) {
                match ctor_name.as_str() {
                    "Left" | "Ok" => {
                        if left.is_none() {
                            left = Some((ctor_name, ctor_arg));
                        }
                    }
                    "Right" | "Err" => {
                        if right.is_none() {
                            right = Some((ctor_name, ctor_arg));
                        }
                    }
                    _ => {}
                }
            }
        }

        let (left_name, left_arg) = left?;
        let (right_name, right_arg) = right?;
        Some((left_name, left_arg, right_name, right_arg))
    }

    pub(super) fn extract_value_expr<'a>(&self, expr: &'a syn::Expr) -> Option<&'a syn::Expr> {
        match expr {
            syn::Expr::Group(g) => self.extract_value_expr(&g.expr),
            syn::Expr::Paren(p) => self.extract_value_expr(&p.expr),
            syn::Expr::Block(block_expr) => self.extract_single_expr_from_block(&block_expr.block),
            _ => Some(expr),
        }
    }

    pub(super) fn extract_match_arm_value_expr<'a>(&self, expr: &'a syn::Expr) -> Option<&'a syn::Expr> {
        match expr {
            syn::Expr::Group(g) => self.extract_match_arm_value_expr(&g.expr),
            syn::Expr::Paren(p) => self.extract_match_arm_value_expr(&p.expr),
            syn::Expr::Block(block_expr) => self
                .extract_single_expr_from_block(&block_expr.block)
                .or_else(|| self.extract_tail_expr_from_block(&block_expr.block)),
            _ => Some(expr),
        }
    }

    pub(super) fn extract_single_expr_from_block<'a>(&self, block: &'a syn::Block) -> Option<&'a syn::Expr> {
        if block.stmts.len() != 1 {
            return None;
        }
        match &block.stmts[0] {
            syn::Stmt::Expr(expr, None) => Some(expr),
            _ => None,
        }
    }

    pub(super) fn extract_tail_expr_from_block<'a>(&self, block: &'a syn::Block) -> Option<&'a syn::Expr> {
        match block.stmts.last()? {
            syn::Stmt::Expr(expr, None) => Some(expr),
            _ => None,
        }
    }

    pub(super) fn extract_single_value_expr<'a>(&self, expr: &'a syn::Expr) -> Option<&'a syn::Expr> {
        match expr {
            syn::Expr::Block(block) => self.extract_single_expr_from_block(&block.block),
            _ => self.extract_value_expr(expr),
        }
    }

    pub(super) fn extract_single_value_expr_deep<'a>(&self, expr: &'a syn::Expr) -> Option<&'a syn::Expr> {
        match expr {
            syn::Expr::Group(group) => self.extract_single_value_expr_deep(&group.expr),
            syn::Expr::Paren(paren) => self.extract_single_value_expr_deep(&paren.expr),
            syn::Expr::Block(block) => self
                .extract_single_expr_from_block(&block.block)
                .and_then(|inner| self.extract_single_value_expr_deep(inner)),
            _ => Some(expr),
        }
    }

    pub(super) fn extract_option_some_call_arg<'a>(&self, expr: &'a syn::Expr) -> Option<&'a syn::Expr> {
        let expr = self.peel_paren_group_expr(expr);
        let syn::Expr::Call(call) = expr else {
            return None;
        };
        if call.args.len() != 1 {
            return None;
        }
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
            "Some" | "Option::Some" | "core::option::Option::Some"
        ) {
            return call.args.first();
        }
        None
    }

    pub(super) fn extract_constructor_call_expr<'a>(
        &self,
        expr: &'a syn::Expr,
    ) -> Option<(String, &'a syn::Expr)> {
        let expr = self.extract_value_expr(expr)?;
        let call = match expr {
            syn::Expr::Call(call) => call,
            _ => return None,
        };
        if call.args.len() != 1 {
            return None;
        }
        let func_path = match call.func.as_ref() {
            syn::Expr::Path(path) => &path.path,
            _ => return None,
        };
        let ctor_name = if func_path.segments.len() == 1 {
            func_path.segments[0].ident.to_string()
        } else if let Some(ctor) = self.variant_ctor_name_from_path(func_path) {
            ctor
        } else {
            return None;
        };
        if !matches!(ctor_name.as_str(), "Left" | "Right" | "Ok" | "Err") {
            return None;
        }
        Some((ctor_name, &call.args[0]))
    }

    /// Look up the nearest in-scope local binding type for a variable name.
    pub(super) fn lookup_local_binding_type(&self, name: &str) -> Option<syn::Type> {
        let skip_current_scope_binding = self
            .in_progress_local_initializers
            .iter()
            .rev()
            .any(|current| current == name);
        for (scope_idx, scope) in self.local_bindings.iter().rev().enumerate() {
            if let Some(maybe_ty) = scope.get(name) {
                if skip_current_scope_binding && scope_idx == 0 {
                    if let Some(previous_ty) = self
                        .local_shadowed_binding_types
                        .last()
                        .and_then(|shadow_scope| shadow_scope.get(name))
                        .and_then(|stack| stack.last())
                        .cloned()
                    {
                        return previous_ty;
                    }
                    continue;
                }
                return maybe_ty.clone();
            }
        }
        for scope in self.param_bindings.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    pub(super) fn lookup_item_const_type(&self, name: &str) -> Option<syn::Type> {
        for depth in (1..=self.module_stack.len()).rev() {
            let scoped = format!("{}::{}", self.module_stack[..depth].join("::"), name);
            if let Some(ty) = self.item_const_types.get(&scoped) {
                return Some(ty.clone());
            }
        }
        self.item_const_types.get(name).cloned()
    }

    pub(super) fn lookup_field_type_for_expr_base(
        &self,
        base: &syn::Expr,
        field_name: &str,
    ) -> Option<syn::Type> {
        match base {
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                let base_name = path.path.segments[0].ident.to_string();
                if matches!(base_name.as_str(), "self" | "self_") {
                    if let Some(struct_name) = &self.current_struct
                        && let Some(field_ty) =
                            self.lookup_struct_field_type(struct_name, field_name)
                    {
                        return Some(field_ty);
                    }
                    if let Some(self_ty) = self
                        .lookup_local_binding_type(&base_name)
                        .or_else(|| self.lookup_local_binding_type("self"))
                        .or_else(|| self.lookup_local_binding_type("self_"))
                        && let Some(field_ty) =
                            self.lookup_field_type_from_type(&self_ty, field_name)
                    {
                        return Some(field_ty);
                    }
                    return None;
                }
                let base_ty = self.lookup_local_binding_type(&base_name)?;
                self.lookup_field_type_from_type(&base_ty, field_name)
            }
            syn::Expr::Paren(p) => self.lookup_field_type_for_expr_base(&p.expr, field_name),
            syn::Expr::Group(g) => self.lookup_field_type_for_expr_base(&g.expr, field_name),
            syn::Expr::Reference(r) => self.lookup_field_type_for_expr_base(&r.expr, field_name),
            syn::Expr::Field(field_expr) => {
                // For chained field access like `self.comparators.iter()` where the base
                // is itself a field (`self.comparators`), first resolve the base field,
                // then look up the target field within that type.
                let base_field_name = match &field_expr.member {
                    syn::Member::Named(ident) => ident.to_string(),
                    syn::Member::Unnamed(_) => return None,
                };
                if let Some(base_field_ty) =
                    self.lookup_field_type_for_expr_base(&field_expr.base, &base_field_name)
                {
                    return self.lookup_field_type_from_type(&base_field_ty, field_name);
                }
                None
            }
            _ => None,
        }
    }

    pub(super) fn lookup_field_type_from_type(&self, ty: &syn::Type, field_name: &str) -> Option<syn::Type> {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return None;
        };
        let struct_name = tp.path.segments.last()?.ident.to_string();
        let struct_name = if struct_name == "Self" {
            self.current_struct.clone()?
        } else {
            struct_name
        };
        self.lookup_struct_field_type(&struct_name, field_name)
    }

    pub(super) fn lookup_struct_field_type(&self, struct_name: &str, field_name: &str) -> Option<syn::Type> {
        let struct_name_tail = struct_name.rsplit("::").next().unwrap_or(struct_name);
        let scoped = self.scoped_type_key(struct_name);
        self.struct_field_types
            .get(&scoped)
            .and_then(|fields| fields.get(field_name).cloned())
            .or_else(|| {
                if struct_name.contains("::") {
                    return None;
                }
                let mut qualified_tail_matches = self
                    .struct_field_types
                    .iter()
                    .filter_map(|(key, fields)| {
                        (key.contains("::")
                            && key
                                .rsplit("::")
                                .next()
                                .is_some_and(|tail| tail == struct_name_tail))
                        .then_some(fields)
                    })
                    .collect::<Vec<_>>();
                qualified_tail_matches.dedup_by(|a, b| std::ptr::eq(*a, *b));
                if qualified_tail_matches.len() == 1 {
                    qualified_tail_matches[0].get(field_name).cloned()
                } else {
                    None
                }
            })
            .or_else(|| {
                self.struct_field_types
                    .get(struct_name)
                    .and_then(|fields| fields.get(field_name).cloned())
            })
            .or_else(|| {
                let mut tail_matches = self
                    .struct_field_types
                    .iter()
                    .filter_map(|(key, fields)| {
                        key.rsplit("::")
                            .next()
                            .is_some_and(|tail| tail == struct_name_tail)
                            .then_some(fields)
                    })
                    .collect::<Vec<_>>();
                tail_matches.dedup_by(|a, b| std::ptr::eq(*a, *b));
                if tail_matches.len() == 1 {
                    tail_matches[0].get(field_name).cloned()
                } else {
                    None
                }
            })
    }

    pub(super) fn lookup_struct_field_cpp_name(&self, struct_name: &str, field_name: &str) -> Option<String> {
        let struct_name_tail = struct_name.rsplit("::").next().unwrap_or(struct_name);
        let scoped = self.scoped_type_key(struct_name);
        self.struct_field_cpp_names
            .get(&scoped)
            .and_then(|fields| fields.get(field_name).cloned())
            .or_else(|| {
                if struct_name.contains("::") {
                    return None;
                }
                let mut qualified_tail_matches = self
                    .struct_field_cpp_names
                    .iter()
                    .filter_map(|(key, fields)| {
                        (key.contains("::")
                            && key
                                .rsplit("::")
                                .next()
                                .is_some_and(|tail| tail == struct_name_tail))
                        .then_some(fields)
                    })
                    .collect::<Vec<_>>();
                qualified_tail_matches.dedup_by(|a, b| std::ptr::eq(*a, *b));
                if qualified_tail_matches.len() == 1 {
                    qualified_tail_matches[0].get(field_name).cloned()
                } else {
                    None
                }
            })
            .or_else(|| {
                self.struct_field_cpp_names
                    .get(struct_name)
                    .and_then(|fields| fields.get(field_name).cloned())
            })
            .or_else(|| {
                let mut tail_matches = self
                    .struct_field_cpp_names
                    .iter()
                    .filter_map(|(key, fields)| {
                        key.rsplit("::")
                            .next()
                            .is_some_and(|tail| tail == struct_name_tail)
                            .then_some(fields)
                    })
                    .collect::<Vec<_>>();
                tail_matches.dedup_by(|a, b| std::ptr::eq(*a, *b));
                if tail_matches.len() == 1 {
                    tail_matches[0].get(field_name).cloned()
                } else {
                    None
                }
            })
    }

    pub(super) fn lookup_field_cpp_name_from_type(&self, ty: &syn::Type, field_name: &str) -> Option<String> {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return None;
        };
        let struct_name = tp.path.segments.last()?.ident.to_string();
        let struct_name = if struct_name == "Self" {
            self.current_struct.clone()?
        } else {
            struct_name
        };
        self.lookup_struct_field_cpp_name(&struct_name, field_name)
    }

    pub(super) fn lookup_field_cpp_name_for_expr_base(
        &self,
        base: &syn::Expr,
        field_name: &str,
    ) -> Option<String> {
        match base {
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                let base_name = path.path.segments[0].ident.to_string();
                if base_name == "self" {
                    if let Some(struct_name) = &self.current_struct {
                        return self.lookup_struct_field_cpp_name(struct_name, field_name);
                    }
                    return None;
                }
                let base_ty = self.lookup_local_binding_type(&base_name)?;
                self.lookup_field_cpp_name_from_type(&base_ty, field_name)
            }
            syn::Expr::Paren(p) => self.lookup_field_cpp_name_for_expr_base(&p.expr, field_name),
            syn::Expr::Group(g) => self.lookup_field_cpp_name_for_expr_base(&g.expr, field_name),
            syn::Expr::Reference(r) => {
                self.lookup_field_cpp_name_for_expr_base(&r.expr, field_name)
            }
            syn::Expr::Field(field_expr) => {
                let base_field_name = match &field_expr.member {
                    syn::Member::Named(ident) => ident.to_string(),
                    syn::Member::Unnamed(_) => return None,
                };
                if let Some(base_field_ty) =
                    self.lookup_field_type_for_expr_base(&field_expr.base, &base_field_name)
                {
                    return self.lookup_field_cpp_name_from_type(&base_field_ty, field_name);
                }
                None
            }
            _ => self
                .infer_simple_expr_type(base)
                .and_then(|ty| self.lookup_field_cpp_name_from_type(&ty, field_name)),
        }
    }

    pub(super) fn lookup_struct_field_order(&self, struct_name: &str) -> Option<&Vec<String>> {
        self.struct_field_order.get(struct_name).or_else(|| {
            let scoped = self.scoped_type_key(struct_name);
            self.struct_field_order.get(&scoped)
        })
    }

    pub(super) fn extract_add_pointer_inner_cpp_type(ty: &str) -> Option<String> {
        let trimmed = ty.trim();
        let prefix = "std::add_pointer_t<";
        if !trimmed.starts_with(prefix) || !trimmed.ends_with('>') {
            return None;
        }
        let inner = trimmed
            .strip_prefix(prefix)?
            .strip_suffix('>')?
            .trim()
            .to_string();
        if inner.is_empty() {
            return None;
        }
        Some(inner)
    }

    pub(super) fn lookup_method_arg_expected_type_from_receiver_owner(
        &self,
        receiver: &syn::Expr,
        method_name: &str,
        arg_idx: usize,
        arg_expr: Option<&syn::Expr>,
    ) -> Option<syn::Type> {
        let (owner, substitutions) = self.receiver_owner_name_and_type_substitutions(receiver)?;
        let expected =
            self.lookup_owner_method_arg_expected_type(&owner, method_name, arg_idx, arg_expr)?;
        if substitutions.is_empty() {
            Some(expected)
        } else {
            Some(self.substitute_type_params_in_type(&expected, &substitutions))
        }
    }

    pub(super) fn lookup_unique_c_like_owner_with_method(&self, method_name: &str) -> Option<String> {
        let mut owners: Vec<String> = self
            .inherent_impl_method_names
            .iter()
            .filter_map(|(owner, methods)| {
                if !methods.contains(method_name) {
                    return None;
                }
                let owner_tail = owner.rsplit("::").next().unwrap_or(owner.as_str());
                let is_c_like_owner = self.c_like_enum_types.contains(owner)
                    || self.c_like_enum_types.contains(owner_tail)
                    || self
                        .c_like_enum_types
                        .iter()
                        .any(|name| name.ends_with(&format!("::{}", owner_tail)));
                is_c_like_owner.then_some(owner.clone())
            })
            .collect();
        owners.sort();
        owners.dedup();
        if owners.len() == 1 {
            owners.into_iter().next()
        } else {
            None
        }
    }

    pub(super) fn extract_smallvec_owner_array_cpp_from_func(func: &str) -> Option<String> {
        let marker = "SmallVec<";
        let start = func.find(marker)? + marker.len();
        let bytes = func.as_bytes();
        let mut depth: i32 = 1;
        let mut idx = start;
        while idx < bytes.len() {
            match bytes[idx] as char {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        let inner = func[start..idx].trim();
                        if inner.is_empty() {
                            return None;
                        }
                        return Some(inner.to_string());
                    }
                }
                _ => {}
            }
            idx += 1;
        }
        None
    }

    pub(super) fn lookup_struct_literal_field_type(
        &self,
        struct_expr: &syn::ExprStruct,
        field_name: &str,
        expected_ty: Option<&syn::Type>,
    ) -> Option<syn::Type> {
        let struct_name = struct_expr.path.segments.last()?.ident.to_string();
        let struct_name = if struct_name == "Self" {
            self.current_struct.clone()?
        } else {
            struct_name
        };
        let base_field_ty = self.lookup_struct_field_type(&struct_name, field_name)?;

        let mut substitutions = HashMap::new();
        if let Some(last_idx) = struct_expr.path.segments.len().checked_sub(1) {
            if let Some(owner_subs) =
                self.owner_segment_type_arg_substitutions(&struct_expr.path, last_idx)
            {
                substitutions.extend(owner_subs);
            }
        }

        let resolved_expected_ty =
            self.resolve_expected_type_for_struct_literal(expected_ty, &struct_expr.path);
        if substitutions.is_empty() {
            if let Some(syn::Type::Path(tp)) = resolved_expected_ty.as_ref() {
                if let Some(last_seg) = tp.path.segments.last() {
                    if let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments {
                        let provided_type_args: Vec<syn::Type> = args
                            .args
                            .iter()
                            .filter_map(|arg| match arg {
                                syn::GenericArgument::Type(ty)
                                    if !matches!(ty, syn::Type::Infer(_)) =>
                                {
                                    Some(ty.clone())
                                }
                                _ => None,
                            })
                            .collect();
                        if !provided_type_args.is_empty() {
                            let scoped_key = self.scoped_type_key(&struct_name);
                            let params = self
                                .declared_type_params
                                .get(&struct_name)
                                .or_else(|| self.declared_type_params.get(&scoped_key));
                            let param_kinds = self
                                .declared_type_param_kinds
                                .get(&struct_name)
                                .or_else(|| self.declared_type_param_kinds.get(&scoped_key));
                            if let Some(params) = params {
                                let mut provided_iter = provided_type_args.into_iter();
                                for (idx, param) in params.iter().enumerate() {
                                    let is_type_param = param_kinds
                                        .and_then(|kinds| kinds.get(idx))
                                        .is_none_or(|kind| matches!(kind, GenericParamKind::Type));
                                    if !is_type_param {
                                        continue;
                                    }
                                    let Some(concrete_ty) = provided_iter.next() else {
                                        break;
                                    };
                                    substitutions.insert(param.clone(), concrete_ty);
                                }
                            }
                        }
                    }
                }
            }
        }

        if substitutions.is_empty()
            && struct_expr
                .path
                .segments
                .last()
                .is_some_and(|seg| matches!(seg.arguments, syn::PathArguments::None))
        {
            let mapped_path = self.emit_path_to_string(&struct_expr.path);
            if let Some(recovered) = self
                .recover_omitted_struct_literal_generic_type_args_from_fields(
                    struct_expr,
                    &mapped_path,
                )
                && let Ok(syn::Type::Path(tp)) = syn::parse_str::<syn::Type>(&recovered)
                && let Some(last_seg) = tp.path.segments.last()
                && let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments
            {
                let provided_type_args: Vec<syn::Type> = args
                    .args
                    .iter()
                    .filter_map(|arg| match arg {
                        syn::GenericArgument::Type(ty) if !matches!(ty, syn::Type::Infer(_)) => {
                            Some(ty.clone())
                        }
                        _ => None,
                    })
                    .collect();
                if !provided_type_args.is_empty() {
                    if provided_type_args
                        .iter()
                        .any(|ty| self.type_arg_is_value_identifier_type(ty))
                    {
                        // Ignore leaked value-identifier recoveries (e.g. `<ptr>`) for
                        // field substitution; leave unresolved params to normal scope flow.
                        // This keeps `Self`-typed struct literals from back-propagating local
                        // variable names into owner template arguments.
                    } else {
                        let scoped_key = self.scoped_type_key(&struct_name);
                        let params = self
                            .declared_type_params
                            .get(&struct_name)
                            .or_else(|| self.declared_type_params.get(&scoped_key));
                        let param_kinds = self
                            .declared_type_param_kinds
                            .get(&struct_name)
                            .or_else(|| self.declared_type_param_kinds.get(&scoped_key));
                        if let Some(params) = params {
                            let mut provided_iter = provided_type_args.into_iter();
                            for (idx, param) in params.iter().enumerate() {
                                let is_type_param = param_kinds
                                    .and_then(|kinds| kinds.get(idx))
                                    .is_none_or(|kind| matches!(kind, GenericParamKind::Type));
                                if !is_type_param {
                                    continue;
                                }
                                let Some(concrete_ty) = provided_iter.next() else {
                                    break;
                                };
                                substitutions.insert(param.clone(), concrete_ty);
                            }
                        }
                    }
                }
            }
        }

        if substitutions.is_empty() {
            Some(base_field_ty)
        } else {
            Some(self.substitute_type_params_in_type(&base_field_ty, &substitutions))
        }
    }

    pub(super) fn lookup_struct_literal_field_cpp_name(
        &self,
        struct_expr: &syn::ExprStruct,
        field_name: &str,
    ) -> Option<String> {
        let struct_name = if let Some(seg) = struct_expr.path.segments.last() {
            let ident = seg.ident.to_string();
            if ident == "Self" {
                self.current_struct.clone()?
            } else {
                ident
            }
        } else {
            self.current_struct.clone()?
        };
        self.lookup_struct_field_cpp_name(&struct_name, field_name)
    }

    pub(super) fn extract_iter_item_type_from_type(&self, ty: &syn::Type) -> Option<syn::Type> {
        match ty {
            syn::Type::Reference(r) => self.extract_iter_item_type_from_type(&r.elem),
            syn::Type::Slice(s) => Some((*s.elem).clone()),
            syn::Type::Array(a) => Some((*a.elem).clone()),
            syn::Type::ImplTrait(it) => self.extract_iter_item_type_from_trait_bounds(&it.bounds),
            syn::Type::TraitObject(obj) => {
                self.extract_iter_item_type_from_trait_bounds(&obj.bounds)
            }
            syn::Type::Path(tp) => {
                let last = tp.path.segments.last()?;
                match last.ident.to_string().as_str() {
                    "IntoIter" | "Iter"
                        if matches!(last.arguments, syn::PathArguments::None)
                            && tp.path.segments.len() >= 2 =>
                    {
                        let mut owner_path = syn::Path {
                            leading_colon: tp.path.leading_colon,
                            segments: syn::punctuated::Punctuated::new(),
                        };
                        for seg in tp.path.segments.iter().take(tp.path.segments.len() - 1) {
                            owner_path.segments.push(seg.clone());
                        }
                        let owner_ty = syn::Type::Path(syn::TypePath {
                            qself: None,
                            path: owner_path,
                        });
                        Some(parse_quote!(rusty::detail::associated_item_t<#owner_ty>))
                    }
                    "SmallVec" => {
                        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
                            return None;
                        };
                        let owner_ty = args.args.iter().find_map(|arg| match arg {
                            syn::GenericArgument::Type(t) => Some(t.clone()),
                            _ => None,
                        })?;
                        Some(parse_quote!(rusty::detail::associated_item_t<#owner_ty>))
                    }
                    "ArrayVec" | "IntoIter" | "Iter" | "Vec" | "array" | "span" | "range"
                    | "range_inclusive" | "range_from" | "range_to" | "range_to_inclusive"
                    | "Range" | "RangeInclusive" | "RangeFrom" | "RangeTo" | "RangeToInclusive" => {
                        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
                            return None;
                        };
                        args.args.iter().find_map(|arg| match arg {
                            syn::GenericArgument::Type(t) => Some(t.clone()),
                            _ => None,
                        })
                    }
                    "SplitIter" => Some(parse_quote!(&str)),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub(super) fn extract_iter_item_type_from_trait_bounds(
        &self,
        bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::token::Plus>,
    ) -> Option<syn::Type> {
        bounds.iter().find_map(|bound| {
            let syn::TypeParamBound::Trait(trait_bound) = bound else {
                return None;
            };
            let seg = trait_bound.path.segments.last()?;
            if !matches!(seg.ident.to_string().as_str(), "Iterator" | "IntoIterator") {
                return None;
            }
            let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
                return None;
            };
            args.args.iter().find_map(|arg| match arg {
                syn::GenericArgument::AssocType(assoc) if assoc.ident == "Item" => {
                    Some(assoc.ty.clone())
                }
                _ => None,
            })
        })
    }

    pub(super) fn extract_deref_target_type_from_inner(&self, ty: &syn::Type) -> Option<syn::Type> {
        let ty = self.peel_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return None;
        };
        let last = tp.path.segments.last()?;
        let owner = last.ident.to_string();
        if !matches!(
            owner.as_str(),
            "Box"
                | "Rc"
                | "Arc"
                | "Ref"
                | "RefMut"
                | "MutexGuard"
                | "SpinMutexGuard"                | "RwLockReadGuard"
                | "RwLockWriteGuard"
        ) {
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

    pub(super) fn lookup_data_enum_variant_arg_expected_type(
        &self,
        enum_path: &syn::Path,
        enum_name: &str,
        variant_name: &str,
        arg_idx: usize,
    ) -> Option<syn::Type> {
        let mut owner_candidates = self.owner_path_to_candidate_owner_keys(enum_path, enum_name);
        owner_candidates.push(enum_name.to_string());
        owner_candidates.push(self.scoped_type_key(enum_name));
        if let Some(owner_tail) = enum_name.rsplit("::").next() {
            owner_candidates.push(owner_tail.to_string());
            owner_candidates.push(self.scoped_type_key(owner_tail));
        }
        let mut dedup_owners = HashSet::new();
        owner_candidates.retain(|owner| dedup_owners.insert(owner.clone()));

        let mut variant_candidates = vec![variant_name.to_string()];
        let canonical_variant = self.canonical_variant_name(variant_name).to_string();
        if canonical_variant != variant_name {
            variant_candidates.push(canonical_variant);
        }
        let mut dedup_variants = HashSet::new();
        variant_candidates.retain(|variant| dedup_variants.insert(variant.clone()));

        for owner in owner_candidates {
            for variant in &variant_candidates {
                let key = format!("{}::{}", owner, variant);
                if let Some(field_types) = self.data_enum_variant_field_types.get(&key)
                    && let Some(field_ty) = field_types.get(arg_idx)
                {
                    return Some(field_ty.clone());
                }
            }
        }
        None
    }

    pub(super) fn lookup_constructor_template_args(&self, ctor_name: &str) -> Option<Vec<String>> {
        for scope in self.constructor_template_hints.iter().rev() {
            if let Some(args) = scope.get(ctor_name) {
                if args.len() == 2 {
                    return Some(args.clone());
                }
            }
        }
        None
    }

    pub(super) fn find_local_binding_in_block_by_name<'a>(
        &self,
        block: &'a syn::Block,
        name: &str,
    ) -> Option<&'a syn::Local> {
        block.stmts.iter().rev().find_map(|stmt| {
            let syn::Stmt::Local(local) = stmt else {
                return None;
            };
            let local_name = local_binding_name(local)?;
            if local_name == name {
                Some(local)
            } else {
                None
            }
        })
    }

    /// Extract the binding name from an if-let pattern like `Some(x)` → "x"
    pub(super) fn extract_if_let_binding_name(&self, pat: &syn::Pat) -> Option<String> {
        match pat {
            syn::Pat::TupleStruct(ts) => {
                if let Some(inner) = ts.elems.first() {
                    if let syn::Pat::Ident(pi) = inner {
                        return Some(escape_cpp_keyword(&pi.ident.to_string()));
                    }
                }
                None
            }
            syn::Pat::Ident(pi) => Some(escape_cpp_keyword(&pi.ident.to_string())),
            _ => None,
        }
    }

    pub(super) fn extract_simple_type_param_name(&self, ty: &syn::Type) -> Option<String> {
        let ty = self.peel_reference_paren_group_type(ty);
        let syn::Type::Path(tp) = ty else {
            return None;
        };
        if tp.qself.is_some() || tp.path.segments.len() != 1 {
            return None;
        }
        let seg = tp.path.segments.first()?;
        if !matches!(seg.arguments, syn::PathArguments::None) {
            return None;
        }
        Some(seg.ident.to_string())
    }

    pub(super) fn extract_simple_const_param_name(&self, expr: &syn::Expr) -> Option<String> {
        let expr = self.peel_paren_group_expr(expr);
        let syn::Expr::Path(path_expr) = expr else {
            return None;
        };
        if path_expr.path.segments.len() != 1 {
            return None;
        }
        let seg = path_expr.path.segments.first()?;
        if !matches!(seg.arguments, syn::PathArguments::None) {
            return None;
        }
        Some(seg.ident.to_string())
    }

    pub(super) fn lookup_declared_type_key_for_base(&self, scoped_base: &str, base: &str) -> Option<String> {
        if self.declared_type_params.contains_key(scoped_base) {
            return Some(scoped_base.to_string());
        }
        // For qualified paths (for example `format::Buf`), only accept an exact
        // suffix-qualified match. Tail-only fallback can incorrectly bind to an
        // unrelated same-tail generic type (`some::Buf<T>`), which then injects
        // bogus template arguments into non-generic owners.
        if scoped_base.contains("::") {
            let qualified_suffix = format!("::{}", scoped_base);
            let mut suffix_matches: Vec<&String> = self
                .declared_type_params
                .keys()
                .filter(|key| *key == scoped_base || key.ends_with(&qualified_suffix))
                .collect();
            suffix_matches.sort();
            suffix_matches.dedup();
            if suffix_matches.len() == 1 {
                return Some((*suffix_matches[0]).clone());
            }
            return None;
        }
        if self.declared_type_params.contains_key(base) {
            return Some(base.to_string());
        }

        let mut tail_matches: Vec<&String> = self
            .declared_type_params
            .keys()
            .filter(|key| key.rsplit("::").next().is_some_and(|tail| tail == base))
            .collect();
        tail_matches.sort();
        tail_matches.dedup();
        if tail_matches.len() == 1 {
            return Some((*tail_matches[0]).clone());
        }

        let mut scoped_tail_matches: Vec<&String> = tail_matches
            .into_iter()
            .filter(|key| key.contains("::"))
            .collect();
        scoped_tail_matches.sort();
        scoped_tail_matches.dedup();
        if scoped_tail_matches.len() == 1 {
            return Some((*scoped_tail_matches[0]).clone());
        }

        None
    }

    pub(super) fn lookup_associated_const_type(&self, path: &syn::Path) -> Option<syn::Type> {
        if path.segments.len() < 2 {
            return None;
        }
        let member = path.segments.last()?.ident.to_string();
        let mut owner_segments: Vec<String> = path
            .segments
            .iter()
            .take(path.segments.len() - 1)
            .map(|seg| seg.ident.to_string())
            .collect();
        if owner_segments.first().is_some_and(|seg| seg == "Self") {
            let current = self.current_struct.as_ref()?;
            owner_segments[0] = current.clone();
        }
        while owner_segments
            .first()
            .is_some_and(|seg| matches!(seg.as_str(), "crate" | "self" | "super"))
        {
            owner_segments.remove(0);
        }
        if owner_segments.is_empty() {
            return None;
        }

        let owner = owner_segments.join("::");
        let owner_tail = owner_segments.last().cloned().unwrap_or_default();
        let mut candidates = vec![
            owner.clone(),
            owner_tail.clone(),
            self.scoped_type_key(&owner),
        ];
        if owner_tail != owner {
            candidates.push(self.scoped_type_key(&owner_tail));
        }
        candidates.sort();
        candidates.dedup();

        let lookup = |items: &[syn::ImplItem]| -> Option<syn::Type> {
            items.iter().find_map(|item| {
                if let syn::ImplItem::Const(c) = item
                    && c.ident == member
                {
                    return Some(c.ty.clone());
                }
                None
            })
        };

        for key in &candidates {
            if let Some(items) = self.impl_blocks.get(key)
                && let Some(ty) = lookup(items)
            {
                return Some(ty);
            }
            if let Some(items) = self.consumed_impl_blocks.get(key)
                && let Some(ty) = lookup(items)
            {
                return Some(ty);
            }
        }

        let suffix = format!("::{}", owner_tail);
        for (key, items) in &self.impl_blocks {
            if (key == &owner || key.ends_with(&suffix))
                && let Some(ty) = lookup(items)
            {
                return Some(ty);
            }
        }
        for (key, items) in &self.consumed_impl_blocks {
            if (key == &owner || key.ends_with(&suffix))
                && let Some(ty) = lookup(items)
            {
                return Some(ty);
            }
        }
        None
    }

    /// Extract the inner expression from a reference expression, as a string.
    pub(super) fn extract_ref_inner(&self, expr: &syn::Expr) -> String {
        if let syn::Expr::Reference(r) = expr {
            self.emit_expr_to_string(&r.expr)
        } else {
            self.emit_expr_to_string(expr)
        }
    }

    /// Search the `cross_file_impl_blocks` index for a method with this name
    /// and return its return type if any matching impl-block has one. When
    /// multiple impl-blocks define the same name, pick the first one with a
    /// non-default return type. The lookup is by method name only (not by
    /// fully-qualified path), so it's intentionally permissive — we only
    /// trust references it returns, never values.
    /// Walks every impl-block we know about (cross-file index plus the
    /// current crate's `impl_blocks` / `consumed_impl_blocks` maps) and
    /// returns a representative return type for `method` *only* when every
    /// definition that uses this name returns a reference. When even one
    /// impl returns a value, the name is ambiguous (e.g. `NodeRef::reborrow`
    /// returns a value-typed wrapper, while `DormantMutRef::reborrow`
    /// returns `&mut T`) and we can't safely treat it as ref-returning from
    /// name alone.
    pub(super) fn lookup_known_method_return_type_by_name(&self, method: &str) -> Option<syn::Type> {
        let mut representative: Option<syn::Type> = None;
        let mut seen = false;
        let mut visit_item = |item: &syn::ImplItem| -> Option<()> {
            if let syn::ImplItem::Fn(f) = item {
                if f.sig.ident == method {
                    seen = true;
                    let ty = match &f.sig.output {
                        syn::ReturnType::Default => return None,
                        syn::ReturnType::Type(_, ty) => (**ty).clone(),
                    };
                    if !self.type_is_reference_like(&ty) {
                        return None;
                    }
                    if representative.is_none() {
                        representative = Some(ty);
                    }
                }
            }
            Some(())
        };
        for impl_block in &self.cross_file_impl_blocks {
            for item in &impl_block.items {
                if visit_item(item).is_none() {
                    return None;
                }
            }
        }
        for items in self.impl_blocks.values() {
            for item in items {
                if visit_item(item).is_none() {
                    return None;
                }
            }
        }
        for items in self.consumed_impl_blocks.values() {
            for item in items {
                if visit_item(item).is_none() {
                    return None;
                }
            }
        }
        if seen { representative } else { None }
    }

    pub(super) fn lookup_callable_return_type_for_type_param(&self, type_param: &str) -> Option<syn::Type> {
        self.callable_type_param_return_scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(type_param).cloned())
    }

    /// Phase 3b.2: if `assoc_name` is uniquely declared as an
    /// associated type by exactly one trait in this translation unit,
    /// return that trait's name. Returns `None` if no trait declares
    /// it, or if multiple traits do (ambiguous — we'd need trait-bound
    /// context to disambiguate, which we don't track yet).
    pub(super) fn lookup_unique_trait_for_assoc_name(&self, assoc_name: &str) -> Option<String> {
        let mut matches = self
            .trait_associated_type_names
            .iter()
            .filter(|(_, names)| names.iter().any(|n| n == assoc_name));
        let first = matches.next()?;
        if matches.next().is_some() {
            return None; // ambiguous
        }
        Some(first.0.clone())
    }

    /// Disambiguate a multiply-declared associated name via the OWNER type
    /// param's trait bound. `lookup_unique_trait_for_assoc_name` returns `None`
    /// when an assoc name (e.g. serde's `Ok`/`Error`) is declared by several
    /// traits. But if the owner is a concrete type param `S` bound by exactly
    /// one trait that declares the name (`fn serialize<S: Serializer>` →
    /// `Serializer` is the only bound declaring `Ok`/`Error`), the projection
    /// `S::Ok` is unambiguous and can route through `<Trait>Traits<S>`. Returns
    /// the `trait_associated_type_names` key (short trait name) iff exactly one
    /// of `type_param`'s bound traits declares `assoc_name`.
    pub(super) fn lookup_trait_for_assoc_via_param_bound(
        &self,
        type_param: &str,
        assoc_name: &str,
    ) -> Option<String> {
        let mut matched_keys: Vec<String> = Vec::new();
        for scope in self.trait_bound_type_param_scopes.iter().rev() {
            for (bound_trait, bound_param) in scope {
                if bound_param != type_param {
                    continue;
                }
                // `bound_trait` may be qualified (`a::b::Serializer`); the
                // `trait_associated_type_names` keys are short names. Match on
                // the last segment (mirrors `type_param_has_trait_bound`).
                let bound_short = bound_trait.rsplit("::").next().unwrap_or(bound_trait);
                for (trait_key, names) in &self.trait_associated_type_names {
                    let key_short = trait_key.rsplit("::").next().unwrap_or(trait_key);
                    if (key_short == bound_short || trait_key == bound_trait)
                        && names.iter().any(|n| n == assoc_name)
                        && !matched_keys.contains(trait_key)
                    {
                        matched_keys.push(trait_key.clone());
                    }
                }
            }
        }
        if matched_keys.len() == 1 {
            matched_keys.into_iter().next()
        } else {
            None
        }
    }
}
