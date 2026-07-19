use super::*;

/// Replaces every bare `Type::Path` whose single-segment identifier equals
/// `from` with the type `to` (used to substitute an assoc-bound impl generic
/// `A` with `<SelfParam as Iterator>::Item`).
pub(super) struct TypeIdentReplacer {
    pub(super) from: String,
    pub(super) to: syn::Type,
}

impl syn::visit_mut::VisitMut for TypeIdentReplacer {
    fn visit_type_mut(&mut self, ty: &mut syn::Type) {
        if let syn::Type::Path(tp) = ty
            && tp.qself.is_none()
            && tp
                .path
                .get_ident()
                .map(|i| i.to_string())
                .as_deref()
                == Some(self.from.as_str())
        {
            *ty = self.to.clone();
            return;
        }
        syn::visit_mut::visit_type_mut(self, ty);
    }
}

impl CodeGen {
    pub(super) fn collect_struct_enum_items_recursive<'a>(
        items: &'a [syn::Item],
        out: &mut Vec<&'a syn::Item>,
    ) {
        for item in items {
            match item {
                syn::Item::Struct(_) => out.push(item),
                syn::Item::Enum(e) if e.variants.iter().any(|v| !v.fields.is_empty()) => {
                    out.push(item);
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested)) = &m.content {
                        Self::collect_struct_enum_items_recursive(nested, out);
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_top_level_type_names(items: &[syn::Item]) -> HashSet<String> {
        items
            .iter()
            .filter_map(Self::item_top_level_type_name)
            .collect()
    }

    pub(super) fn collect_known_type_references_in_module_items(
        items: &[syn::Item],
        known_type_names: &HashSet<String>,
        trait_default_type_deps: &HashMap<String, HashSet<String>>,
        ignore_fn_items: bool,
    ) -> HashSet<String> {
        let mut collector = KnownTypeReferenceCollector::new(known_type_names);
        for item in items {
            if ignore_fn_items
                && matches!(
                    item,
                    syn::Item::Fn(_) | syn::Item::Use(_) | syn::Item::Impl(_)
                )
            {
                continue;
            }
            collector.visit_item(item);
        }
        let mut refs = collector.into_referenced_type_names();
        if ignore_fn_items {
            return refs;
        }
        for item in items {
            let syn::Item::Use(u) = item else {
                continue;
            };
            let mut prefix: Vec<String> = Vec::new();
            Self::collect_use_tree_known_type_references(
                &u.tree,
                &mut prefix,
                known_type_names,
                &mut refs,
            );
        }
        Self::collect_trait_impl_default_method_dependency_hints(
            items,
            trait_default_type_deps,
            &mut refs,
        );
        refs
    }

    pub(super) fn collect_use_tree_known_type_references(
        tree: &syn::UseTree,
        prefix: &mut Vec<String>,
        known_type_names: &HashSet<String>,
        out: &mut HashSet<String>,
    ) {
        match tree {
            syn::UseTree::Path(p) => {
                prefix.push(p.ident.to_string());
                Self::collect_use_tree_known_type_references(
                    &p.tree,
                    prefix,
                    known_type_names,
                    out,
                );
                let _ = prefix.pop();
            }
            syn::UseTree::Name(n) => {
                if known_type_names.contains(n.ident.to_string().as_str()) {
                    out.insert(n.ident.to_string());
                }
            }
            syn::UseTree::Rename(r) => {
                if known_type_names.contains(r.ident.to_string().as_str()) {
                    out.insert(r.ident.to_string());
                }
            }
            syn::UseTree::Glob(_) => {
                if let Some(last) = prefix.last()
                    && known_type_names.contains(last.as_str())
                {
                    out.insert(last.clone());
                }
            }
            syn::UseTree::Group(g) => {
                for nested in &g.items {
                    Self::collect_use_tree_known_type_references(
                        nested,
                        prefix,
                        known_type_names,
                        out,
                    );
                }
            }
        }
    }

    pub(super) fn collect_trait_default_method_type_dependencies(
        items: &[syn::Item],
        known_type_names: &HashSet<String>,
    ) -> HashMap<String, HashSet<String>> {
        let mut deps_by_trait: HashMap<String, HashSet<String>> = HashMap::new();
        for item in items {
            let syn::Item::Trait(trait_item) = item else {
                continue;
            };
            let trait_name = trait_item.ident.to_string();
            let mut trait_deps: HashSet<String> = HashSet::new();
            for trait_member in &trait_item.items {
                let syn::TraitItem::Fn(method) = trait_member else {
                    continue;
                };
                if method.default.is_none() {
                    continue;
                }
                let mut collector = KnownTypeReferenceCollector::new(known_type_names);
                collector.visit_trait_item_fn(method);
                trait_deps.extend(collector.into_referenced_type_names());
            }
            if !trait_deps.is_empty() {
                deps_by_trait.insert(trait_name, trait_deps);
            }
        }
        deps_by_trait
    }

    pub(super) fn collect_trait_impl_default_method_dependency_hints(
        items: &[syn::Item],
        trait_default_type_deps: &HashMap<String, HashSet<String>>,
        out: &mut HashSet<String>,
    ) {
        for item in items {
            match item {
                syn::Item::Impl(item_impl) => {
                    let Some((_, trait_path, _)) = &item_impl.trait_ else {
                        continue;
                    };
                    let Some(trait_name) =
                        trait_path.segments.last().map(|seg| seg.ident.to_string())
                    else {
                        continue;
                    };
                    if let Some(deps) = trait_default_type_deps.get(trait_name.as_str()) {
                        out.extend(deps.iter().cloned());
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested)) = &m.content {
                        Self::collect_trait_impl_default_method_dependency_hints(
                            nested,
                            trait_default_type_deps,
                            out,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_by_value_cycle_breaking_rewrite_plan(
        &mut self,
        diagnostic: &ByValueCycleDiagnostic,
    ) {
        for edge in &diagnostic.feedback_edges {
            if !matches!(
                edge.rewrite_eligibility,
                ByValueCycleEdgeRewriteEligibility::DirectFieldType
            ) {
                continue;
            }
            self.by_value_cycle_breaking_rewrite_fields
                .insert(ByValueCycleRewriteFieldKey {
                    owner_type: edge.owner_type.clone(),
                    field_name: edge.field_name.clone(),
                });
        }
    }

    pub(super) fn collect_impl_body_assoc_call_type_dependencies(
        items: &[&syn::Item],
        type_names: &HashSet<String>,
    ) -> HashMap<String, HashSet<String>> {
        let mut dependencies_by_owner: HashMap<String, HashSet<String>> = HashMap::new();
        for item in items {
            let syn::Item::Impl(item_impl) = item else {
                continue;
            };
            let Some(owner_name) = Self::impl_self_type_name(item_impl) else {
                continue;
            };
            if !type_names.contains(owner_name.as_str()) {
                continue;
            }

            let mut owner_deps: HashSet<String> = HashSet::new();
            for impl_item in &item_impl.items {
                let syn::ImplItem::Fn(method) = impl_item else {
                    continue;
                };
                for input in &method.sig.inputs {
                    let syn::FnArg::Typed(pat_ty) = input else {
                        continue;
                    };
                    Self::collect_type_name_segments(&pat_ty.ty, type_names, &mut owner_deps);
                }
                if let syn::ReturnType::Type(_, ret_ty) = &method.sig.output {
                    Self::collect_type_name_segments(ret_ty, type_names, &mut owner_deps);
                }
                let mut collector = AssocCallOwnerTypeCollector::new(type_names);
                collector.visit_block(&method.block);
                owner_deps.extend(
                    collector
                        .into_owner_type_names()
                        .into_iter()
                        .filter(|name| name != &owner_name),
                );
            }

            if !owner_deps.is_empty() {
                dependencies_by_owner
                    .entry(owner_name)
                    .or_default()
                    .extend(owner_deps);
            }
        }
        dependencies_by_owner
    }

    pub(super) fn collect_auto_cross_module_by_value_rewrite_plan(&mut self, items: &[syn::Item]) {
        self.collect_auto_cross_module_by_value_rewrite_plan_in_scope(items, &[]);
    }

    pub(super) fn collect_auto_cross_module_by_value_rewrite_plan_in_scope(
        &mut self,
        items: &[syn::Item],
        scope_path: &[String],
    ) {
        let inline_modules: Vec<&syn::ItemMod> = items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Mod(m) if m.content.is_some() => Some(m),
                _ => None,
            })
            .collect();
        let recurse_nested = |this: &mut Self| {
            for item in items {
                let syn::Item::Mod(module_item) = item else {
                    continue;
                };
                let Some((_, nested_items)) = &module_item.content else {
                    continue;
                };
                let mut nested_scope_path = scope_path.to_vec();
                nested_scope_path.push(module_item.ident.to_string());
                this.collect_auto_cross_module_by_value_rewrite_plan_in_scope(
                    nested_items,
                    &nested_scope_path,
                );
            }
        };
        if inline_modules.len() < 2 {
            recurse_nested(self);
            return;
        }
        let module_names: Vec<String> = inline_modules
            .iter()
            .map(|module_item| module_item.ident.to_string())
            .collect();
        let known_modules: HashSet<String> = module_names.iter().cloned().collect();
        let forward_declable_types_by_module: HashMap<String, HashSet<String>> = inline_modules
            .iter()
            .filter_map(|module_item| {
                let module_name = module_item.ident.to_string();
                let Some((_, module_items)) = &module_item.content else {
                    return None;
                };
                let mut names: HashSet<String> = HashSet::new();
                for nested_item in module_items {
                    match nested_item {
                        syn::Item::Struct(s) => {
                            names.insert(s.ident.to_string());
                        }
                        syn::Item::Enum(e) => {
                            names.insert(e.ident.to_string());
                        }
                        syn::Item::Union(u) => {
                            names.insert(u.ident.to_string());
                        }
                        _ => {}
                    }
                }
                if names.is_empty() {
                    None
                } else {
                    Some((module_name, names))
                }
            })
            .collect();
        let module_index: HashMap<String, usize> = module_names
            .iter()
            .enumerate()
            .map(|(idx, name)| (name.clone(), idx))
            .collect();
        let mut module_by_value_field_deps: HashMap<String, HashSet<String>> = HashMap::new();
        for module_item in &inline_modules {
            let owner_module = module_item.ident.to_string();
            let Some((_, module_items)) = &module_item.content else {
                continue;
            };
            let mut deps: HashSet<String> = HashSet::new();
            self.collect_auto_cross_module_by_value_field_dependencies_for_module_items(
                module_items,
                &known_modules,
                &forward_declable_types_by_module,
                &HashMap::new(),
                &HashMap::new(),
                &mut deps,
            );
            deps.remove(&owner_module);
            module_by_value_field_deps.insert(owner_module, deps);
        }
        let mut outgoing: Vec<HashSet<usize>> = vec![HashSet::new(); inline_modules.len()];
        for (owner_module, deps) in &module_by_value_field_deps {
            let Some(&module_pos) = module_index.get(owner_module) else {
                continue;
            };
            for dep in deps {
                let Some(&dep_pos) = module_index.get(dep) else {
                    continue;
                };
                if dep_pos != module_pos {
                    outgoing[module_pos].insert(dep_pos);
                }
            }
        }
        let mut reversed: Vec<Vec<usize>> = vec![Vec::new(); inline_modules.len()];
        for (from, nexts) in outgoing.iter().enumerate() {
            for &to in nexts {
                reversed[to].push(from);
            }
        }
        for nexts in &mut reversed {
            nexts.sort_unstable();
            nexts.dedup();
        }
        fn dfs_order(
            node: usize,
            graph: &[HashSet<usize>],
            seen: &mut [bool],
            order: &mut Vec<usize>,
        ) {
            if seen[node] {
                return;
            }
            seen[node] = true;
            let mut nexts: Vec<usize> = graph[node].iter().copied().collect();
            nexts.sort_unstable();
            for next in nexts {
                if !seen[next] {
                    dfs_order(next, graph, seen, order);
                }
            }
            order.push(node);
        }
        fn dfs_collect(
            node: usize,
            graph: &[Vec<usize>],
            seen: &mut [bool],
            component: &mut Vec<usize>,
        ) {
            if seen[node] {
                return;
            }
            seen[node] = true;
            component.push(node);
            let mut nexts = graph[node].clone();
            nexts.sort_unstable();
            for next in nexts {
                if !seen[next] {
                    dfs_collect(next, graph, seen, component);
                }
            }
        }
        let mut order: Vec<usize> = Vec::with_capacity(inline_modules.len());
        let mut seen = vec![false; inline_modules.len()];
        for node in 0..inline_modules.len() {
            if !seen[node] {
                dfs_order(node, &outgoing, &mut seen, &mut order);
            }
        }
        let mut seen_rev = vec![false; inline_modules.len()];
        let mut module_scc: HashMap<String, usize> = HashMap::new();
        let mut scc_id = 0usize;
        while let Some(node) = order.pop() {
            if seen_rev[node] {
                continue;
            }
            let mut component = Vec::new();
            dfs_collect(node, &reversed, &mut seen_rev, &mut component);
            if component.len() > 1 {
                for idx in component {
                    if let Some(name) = module_names.get(idx) {
                        module_scc.insert(name.clone(), scc_id);
                    }
                }
                scc_id += 1;
            }
        }
        if module_scc.is_empty() {
            recurse_nested(self);
            return;
        }
        for module_item in inline_modules {
            let owner_module = module_item.ident.to_string();
            let Some(owner_scc) = module_scc.get(&owner_module).copied() else {
                continue;
            };
            let Some((_, module_items)) = &module_item.content else {
                continue;
            };
            self.mark_auto_cross_module_rewrite_fields_for_module_items(
                module_items,
                &owner_module,
                scope_path,
                owner_scc,
                &module_scc,
                &known_modules,
                &forward_declable_types_by_module,
                &HashMap::new(),
            );
        }

        // Apply the same SCC-based rewrite planning recursively inside nested
        // inline modules so lower scopes (e.g. `de::parser`) also benefit from
        // automatic cross-module by-value cycle breaking.
        recurse_nested(self);
    }

    pub(super) fn collect_auto_cross_module_by_value_field_dependencies_for_module_items(
        &self,
        items: &[syn::Item],
        known_modules: &HashSet<String>,
        forward_declable_types_by_module: &HashMap<String, HashSet<String>>,
        inherited_imports: &HashMap<String, String>,
        inherited_type_aliases: &HashMap<String, syn::Type>,
        out: &mut HashSet<String>,
    ) {
        let mut imported_name_to_module: HashMap<String, String> = inherited_imports.clone();
        let mut type_aliases: HashMap<String, syn::Type> = inherited_type_aliases.clone();
        for item in items {
            match item {
                syn::Item::Use(u) => {
                    let mut prefix: Vec<String> = Vec::new();
                    self.collect_use_tree_module_aliases_for_hard_dependencies(
                        &u.tree,
                        &mut prefix,
                        known_modules,
                        forward_declable_types_by_module,
                        &mut imported_name_to_module,
                    );
                }
                syn::Item::Type(t) => {
                    type_aliases.insert(t.ident.to_string(), (*t.ty).clone());
                }
                _ => {}
            }
        }

        for item in items {
            match item {
                syn::Item::Struct(s) => {
                    for field in &s.fields {
                        self.collect_type_module_dependencies(
                            &field.ty,
                            known_modules,
                            forward_declable_types_by_module,
                            &imported_name_to_module,
                            true,
                            out,
                        );
                        self.collect_field_alias_module_dependencies_for_auto_cycle_rewrite(
                            &field.ty,
                            &type_aliases,
                            known_modules,
                            forward_declable_types_by_module,
                            &imported_name_to_module,
                            out,
                        );
                    }
                }
                syn::Item::Enum(e) if e.variants.iter().any(|v| !v.fields.is_empty()) => {
                    for variant in &e.variants {
                        for field in &variant.fields {
                            self.collect_type_module_dependencies_strict(
                                &field.ty,
                                known_modules,
                                forward_declable_types_by_module,
                                &imported_name_to_module,
                                true,
                                out,
                            );
                            self.collect_field_alias_module_dependencies_for_auto_cycle_rewrite(
                                &field.ty,
                                &type_aliases,
                                known_modules,
                                forward_declable_types_by_module,
                                &imported_name_to_module,
                                out,
                            );
                        }
                    }
                }
                syn::Item::Mod(module_item) => {
                    let Some((_, nested_items)) = &module_item.content else {
                        continue;
                    };
                    self.collect_auto_cross_module_by_value_field_dependencies_for_module_items(
                        nested_items,
                        known_modules,
                        forward_declable_types_by_module,
                        &imported_name_to_module,
                        &type_aliases,
                        out,
                    );
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_field_alias_module_dependencies_for_auto_cycle_rewrite(
        &self,
        ty: &syn::Type,
        type_aliases: &HashMap<String, syn::Type>,
        known_modules: &HashSet<String>,
        forward_declable_types_by_module: &HashMap<String, HashSet<String>>,
        imported_name_to_module: &HashMap<String, String>,
        out: &mut HashSet<String>,
    ) {
        let syn::Type::Path(type_path) = ty else {
            return;
        };
        if type_path.qself.is_some() {
            return;
        }
        if type_path.path.segments.len() != 1 {
            return;
        }
        let seg = match type_path.path.segments.first() {
            Some(seg) => seg,
            None => return,
        };
        if !matches!(seg.arguments, syn::PathArguments::None) {
            return;
        }
        let alias_name = seg.ident.to_string();
        let Some(alias_ty) = type_aliases.get(&alias_name) else {
            return;
        };
        self.collect_type_module_dependencies(
            alias_ty,
            known_modules,
            forward_declable_types_by_module,
            imported_name_to_module,
            true,
            out,
        );
    }

    pub(super) fn collect_by_value_field_edges(
        fields: &syn::Fields,
        variant_name: Option<&str>,
        owner_type: &str,
        known: &HashSet<String>,
    ) -> Vec<ByValueCycleFeedbackEdge> {
        let mut result = Vec::new();
        for (field_idx, field) in fields.iter().enumerate() {
            let mut targets = HashSet::new();
            Self::collect_by_value_type_name_segments(&field.ty, known, &mut targets, false);
            if targets.is_empty() {
                continue;
            }
            let base_name = field
                .ident
                .as_ref()
                .map(|ident| ident.to_string())
                .unwrap_or_else(|| format!("#{}", field_idx));
            let field_name = if let Some(variant) = variant_name {
                Self::format_by_value_field_name(Some(variant), base_name.as_str())
            } else {
                Self::format_by_value_field_name(None, base_name.as_str())
            };
            let mut sorted_targets: Vec<String> = targets.into_iter().collect();
            sorted_targets.sort();
            sorted_targets.dedup();
            for target_type in sorted_targets {
                let rewrite_eligibility =
                    if Self::field_type_is_direct_by_value_target(&field.ty, &target_type) {
                        ByValueCycleEdgeRewriteEligibility::DirectFieldType
                    } else {
                        ByValueCycleEdgeRewriteEligibility::NonDirectFieldType
                    };
                result.push(ByValueCycleFeedbackEdge {
                    owner_type: owner_type.to_string(),
                    field_name: field_name.clone(),
                    target_type,
                    rewrite_eligibility,
                });
            }
        }
        result.sort();
        result.dedup();
        result
    }

    /// Extract type-name segments from struct/enum fields that match any name
    /// in `known`.  Only extracts the final segment of paths (e.g. `Prerelease`
    /// from `Option<Prerelease>`).
    pub(super) fn collect_field_type_names(fields: &syn::Fields, known: &HashSet<String>) -> HashSet<String> {
        let mut result = HashSet::new();
        for field in fields {
            Self::collect_type_name_segments(&field.ty, known, &mut result);
        }
        result
    }

    pub(super) fn collect_type_name_segments(
        ty: &syn::Type,
        known: &HashSet<String>,
        out: &mut HashSet<String>,
    ) {
        match ty {
            syn::Type::Path(tp) => {
                // Check the final segment name.
                if let Some(last) = tp.path.segments.last() {
                    let name = last.ident.to_string();
                    if Self::path_tail_may_name_local_type(&tp.path) && known.contains(&name) {
                        out.insert(name);
                    }
                    // Recurse into generic arguments.
                    if let syn::PathArguments::AngleBracketed(ab) = &last.arguments {
                        for arg in &ab.args {
                            if let syn::GenericArgument::Type(inner) = arg {
                                Self::collect_type_name_segments(inner, known, out);
                            }
                        }
                    }
                }
            }
            syn::Type::Reference(r) => {
                Self::collect_type_name_segments(&r.elem, known, out);
            }
            syn::Type::Slice(s) => {
                Self::collect_type_name_segments(&s.elem, known, out);
            }
            syn::Type::Array(a) => {
                Self::collect_type_name_segments(&a.elem, known, out);
            }
            syn::Type::Tuple(t) => {
                for elem in &t.elems {
                    Self::collect_type_name_segments(elem, known, out);
                }
            }
            syn::Type::Paren(p) => {
                Self::collect_type_name_segments(&p.elem, known, out);
            }
            _ => {}
        }
    }

    pub(super) fn collect_by_value_type_name_segments(
        ty: &syn::Type,
        known: &HashSet<String>,
        out: &mut HashSet<String>,
        under_indirection: bool,
    ) {
        match ty {
            syn::Type::Path(tp) => {
                if let Some(last) = tp.path.segments.last() {
                    let seg_name = last.ident.to_string();
                    if !under_indirection
                        && Self::path_tail_may_name_local_type(&tp.path)
                        && known.contains(&seg_name)
                    {
                        out.insert(seg_name);
                    }
                    let nested_indirection =
                        under_indirection || Self::is_indirection_container_name(&last.ident);
                    if let syn::PathArguments::AngleBracketed(ab) = &last.arguments {
                        for arg in &ab.args {
                            if let syn::GenericArgument::Type(inner) = arg {
                                Self::collect_by_value_type_name_segments(
                                    inner,
                                    known,
                                    out,
                                    nested_indirection,
                                );
                            }
                        }
                    }
                }
            }
            syn::Type::Reference(r) => {
                Self::collect_by_value_type_name_segments(&r.elem, known, out, true);
            }
            syn::Type::Ptr(p) => {
                Self::collect_by_value_type_name_segments(&p.elem, known, out, true);
            }
            syn::Type::Slice(s) => {
                Self::collect_by_value_type_name_segments(&s.elem, known, out, under_indirection);
            }
            syn::Type::Array(a) => {
                Self::collect_by_value_type_name_segments(&a.elem, known, out, under_indirection);
            }
            syn::Type::Tuple(t) => {
                for elem in &t.elems {
                    Self::collect_by_value_type_name_segments(elem, known, out, under_indirection);
                }
            }
            syn::Type::Paren(p) => {
                Self::collect_by_value_type_name_segments(&p.elem, known, out, under_indirection);
            }
            _ => {}
        }
    }

    /// Collect function names at a specific scope level (non-recursive).
    pub(super) fn collect_scope_function_names(items: &[syn::Item]) -> HashSet<String> {
        items
            .iter()
            .filter_map(|item| {
                if let syn::Item::Fn(f) = item {
                    if !Self::has_cfg_test(&f.attrs) {
                        return Some(f.sig.ident.to_string());
                    }
                }
                None
            })
            .collect()
    }

    pub(super) fn collect_use_imported_names_for_namespace_collision_detection(
        &self,
        tree: &syn::UseTree,
        prefix: &[String],
        module_path: &[String],
        module_fn_map: &HashMap<String, HashSet<String>>,
        out: &mut HashSet<String>,
    ) {
        match tree {
            syn::UseTree::Path(path) => {
                let segment = path.ident.to_string();
                let mut next_prefix = if prefix.is_empty() {
                    module_path.to_vec()
                } else {
                    prefix.to_vec()
                };
                match segment.as_str() {
                    "crate" => {
                        next_prefix.clear();
                    }
                    "self" => {}
                    "super" => {
                        next_prefix.pop();
                    }
                    _ => {
                        if prefix.is_empty() {
                            next_prefix.clear();
                        }
                        next_prefix.push(segment);
                    }
                }
                self.collect_use_imported_names_for_namespace_collision_detection(
                    &path.tree,
                    &next_prefix,
                    module_path,
                    module_fn_map,
                    out,
                );
            }
            syn::UseTree::Name(name) => {
                out.insert(name.ident.to_string());
            }
            syn::UseTree::Rename(rename) => {
                out.insert(rename.rename.to_string());
            }
            syn::UseTree::Glob(_) => {
                let key = prefix.join("::");
                if let Some(functions) = module_fn_map.get(&key) {
                    out.extend(functions.iter().cloned());
                }
            }
            syn::UseTree::Group(group) => {
                for nested in &group.items {
                    self.collect_use_imported_names_for_namespace_collision_detection(
                        nested,
                        prefix,
                        module_path,
                        module_fn_map,
                        out,
                    );
                }
            }
        }
    }

    pub(super) fn collect_scope_module_dependencies(
        &self,
        items: &[syn::Item],
        known_modules: &HashSet<String>,
        out: &mut HashSet<String>,
    ) {
        for item in items {
            match item {
                syn::Item::Use(u) => {
                    let mut prefix: Vec<String> = Vec::new();
                    self.collect_use_tree_module_dependencies(
                        &u.tree,
                        &mut prefix,
                        known_modules,
                        out,
                    );
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested)) = &m.content {
                        self.collect_scope_module_dependencies(nested, known_modules, out);
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_scope_module_hard_dependencies(
        &self,
        items: &[syn::Item],
        known_modules: &HashSet<String>,
        forward_declable_types_by_module: &HashMap<String, HashSet<String>>,
        out: &mut HashSet<String>,
    ) {
        let imported_name_to_module: HashMap<String, String> = HashMap::new();
        self.collect_scope_module_hard_dependencies_with_imports(
            items,
            known_modules,
            forward_declable_types_by_module,
            &imported_name_to_module,
            out,
        );
    }

    pub(super) fn collect_scope_module_strict_type_dependencies(
        &self,
        items: &[syn::Item],
        known_modules: &HashSet<String>,
        forward_declable_types_by_module: &HashMap<String, HashSet<String>>,
        out: &mut HashSet<String>,
    ) {
        let imported_name_to_module: HashMap<String, String> = HashMap::new();
        self.collect_scope_module_strict_type_dependencies_with_imports(
            items,
            known_modules,
            forward_declable_types_by_module,
            &imported_name_to_module,
            out,
        );
    }

    pub(super) fn collect_scope_module_strict_type_dependencies_with_imports(
        &self,
        items: &[syn::Item],
        known_modules: &HashSet<String>,
        forward_declable_types_by_module: &HashMap<String, HashSet<String>>,
        inherited_imports: &HashMap<String, String>,
        out: &mut HashSet<String>,
    ) {
        let mut imported_name_to_module: HashMap<String, String> = inherited_imports.clone();
        for item in items {
            let syn::Item::Use(u) = item else {
                continue;
            };
            let mut prefix: Vec<String> = Vec::new();
            self.collect_use_tree_module_aliases_for_hard_dependencies(
                &u.tree,
                &mut prefix,
                known_modules,
                forward_declable_types_by_module,
                &mut imported_name_to_module,
            );
        }

        for item in items {
            match item {
                syn::Item::Struct(s) => match &s.fields {
                    syn::Fields::Named(fields) => {
                        for field in &fields.named {
                            self.collect_type_module_dependencies_strict(
                                &field.ty,
                                known_modules,
                                forward_declable_types_by_module,
                                &imported_name_to_module,
                                true,
                                out,
                            );
                        }
                    }
                    syn::Fields::Unnamed(fields) => {
                        for field in &fields.unnamed {
                            self.collect_type_module_dependencies_strict(
                                &field.ty,
                                known_modules,
                                forward_declable_types_by_module,
                                &imported_name_to_module,
                                true,
                                out,
                            );
                        }
                    }
                    syn::Fields::Unit => {}
                },
                syn::Item::Enum(e) => {
                    for variant in &e.variants {
                        match &variant.fields {
                            syn::Fields::Named(fields) => {
                                for field in &fields.named {
                                    self.collect_type_module_dependencies_strict(
                                        &field.ty,
                                        known_modules,
                                        forward_declable_types_by_module,
                                        &imported_name_to_module,
                                        true,
                                        out,
                                    );
                                }
                            }
                            syn::Fields::Unnamed(fields) => {
                                for field in &fields.unnamed {
                                    self.collect_type_module_dependencies_strict(
                                        &field.ty,
                                        known_modules,
                                        forward_declable_types_by_module,
                                        &imported_name_to_module,
                                        true,
                                        out,
                                    );
                                }
                            }
                            syn::Fields::Unit => {}
                        }
                    }
                }
                syn::Item::Union(u) => {
                    for field in &u.fields.named {
                        self.collect_type_module_dependencies_strict(
                            &field.ty,
                            known_modules,
                            forward_declable_types_by_module,
                            &imported_name_to_module,
                            true,
                            out,
                        );
                    }
                }
                syn::Item::Const(c) => {
                    self.collect_type_module_dependencies_strict(
                        &c.ty,
                        known_modules,
                        forward_declable_types_by_module,
                        &imported_name_to_module,
                        true,
                        out,
                    );
                    self.collect_expr_module_strict_dependencies_for_member_access(
                        &c.expr,
                        known_modules,
                        forward_declable_types_by_module,
                        &imported_name_to_module,
                        out,
                    );
                }
                syn::Item::Fn(f) => {
                    // Signature positions only need the type NAMEABLE: the
                    // per-module pre-pass forward-declares structs and emits
                    // data-enum variant aliases up front, so a fwd-declarable
                    // param/return type must not force a completeness edge
                    // (that fabricated the de<->loader style module cycles).
                    for input in &f.sig.inputs {
                        let syn::FnArg::Typed(pat_ty) = input else {
                            continue;
                        };
                        self.collect_type_module_dependencies_strict(
                            &pat_ty.ty,
                            known_modules,
                            forward_declable_types_by_module,
                            &imported_name_to_module,
                            false,
                            out,
                        );
                    }
                    if let syn::ReturnType::Type(_, ret_ty) = &f.sig.output {
                        self.collect_type_module_dependencies_strict(
                            ret_ty,
                            known_modules,
                            forward_declable_types_by_module,
                            &imported_name_to_module,
                            false,
                            out,
                        );
                    }
                    self.collect_block_module_strict_dependencies_for_member_access(
                        &f.block,
                        known_modules,
                        forward_declable_types_by_module,
                        &imported_name_to_module,
                        out,
                    );
                }
                syn::Item::Impl(imp) => {
                    for impl_item in &imp.items {
                        match impl_item {
                            syn::ImplItem::Const(c) => {
                                self.collect_type_module_dependencies_strict(
                                    &c.ty,
                                    known_modules,
                                    forward_declable_types_by_module,
                                    &imported_name_to_module,
                                    true,
                                    out,
                                );
                                self.collect_expr_module_strict_dependencies_for_member_access(
                                    &c.expr,
                                    known_modules,
                                    forward_declable_types_by_module,
                                    &imported_name_to_module,
                                    out,
                                );
                            }
                            syn::ImplItem::Type(t) => {
                                // An associated type becomes a member alias
                                // (`using Output = value::Value;`) — its RHS
                                // only needs the name declared, so a
                                // fwd-declarable target must not force a
                                // completeness edge (mapping→value cycle).
                                self.collect_type_module_dependencies_strict(
                                    &t.ty,
                                    known_modules,
                                    forward_declable_types_by_module,
                                    &imported_name_to_module,
                                    false,
                                    out,
                                );
                            }
                            syn::ImplItem::Fn(method) => {
                                // See Item::Fn above: signatures need names,
                                // not complete types, under the pre-pass.
                                for input in &method.sig.inputs {
                                    let syn::FnArg::Typed(pat_ty) = input else {
                                        continue;
                                    };
                                    self.collect_type_module_dependencies_strict(
                                        &pat_ty.ty,
                                        known_modules,
                                        forward_declable_types_by_module,
                                        &imported_name_to_module,
                                        false,
                                        out,
                                    );
                                }
                                if let syn::ReturnType::Type(_, ret_ty) = &method.sig.output {
                                    self.collect_type_module_dependencies_strict(
                                        ret_ty,
                                        known_modules,
                                        forward_declable_types_by_module,
                                        &imported_name_to_module,
                                        false,
                                        out,
                                    );
                                }
                                self.collect_block_module_strict_dependencies_for_member_access(
                                    &method.block,
                                    known_modules,
                                    forward_declable_types_by_module,
                                    &imported_name_to_module,
                                    out,
                                );
                            }
                            _ => {}
                        }
                    }
                }
                syn::Item::Static(s) => {
                    self.collect_type_module_dependencies_strict(
                        &s.ty,
                        known_modules,
                        forward_declable_types_by_module,
                        &imported_name_to_module,
                        true,
                        out,
                    );
                    self.collect_expr_module_strict_dependencies_for_member_access(
                        &s.expr,
                        known_modules,
                        forward_declable_types_by_module,
                        &imported_name_to_module,
                        out,
                    );
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested)) = &m.content {
                        self.collect_scope_module_strict_type_dependencies_with_imports(
                            nested,
                            known_modules,
                            forward_declable_types_by_module,
                            &imported_name_to_module,
                            out,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_expr_module_strict_dependencies_for_member_access(
        &self,
        expr: &syn::Expr,
        known_modules: &HashSet<String>,
        forward_declable_types_by_module: &HashMap<String, HashSet<String>>,
        imported_name_to_module: &HashMap<String, String>,
        out: &mut HashSet<String>,
    ) {
        struct StrictMemberDependencyExprVisitor<'a> {
            codegen: &'a CodeGen,
            known_modules: &'a HashSet<String>,
            forward_declable_types_by_module: &'a HashMap<String, HashSet<String>>,
            imported_name_to_module: &'a HashMap<String, String>,
            out: &'a mut HashSet<String>,
        }

        impl<'ast> Visit<'ast> for StrictMemberDependencyExprVisitor<'_> {
            fn visit_path(&mut self, path: &'ast syn::Path) {
                if self.codegen.path_requires_complete_type_member_ordering(
                    path,
                    self.known_modules,
                    self.imported_name_to_module,
                ) {
                    // allow_skip=true: `Type::member` in a (phase-2) body is
                    // satisfied once the pre-pass has declared the type; only
                    // names with no early declaration force ordering.
                    self.codegen
                        .record_path_module_dependency_for_hard_dependencies(
                            path,
                            self.known_modules,
                            self.forward_declable_types_by_module,
                            self.imported_name_to_module,
                            true,
                            true,
                            self.out,
                        );
                }
                visit::visit_path(self, path);
            }
        }

        let mut visitor = StrictMemberDependencyExprVisitor {
            codegen: self,
            known_modules,
            forward_declable_types_by_module,
            imported_name_to_module,
            out,
        };
        visitor.visit_expr(expr);
    }

    pub(super) fn collect_block_module_strict_dependencies_for_member_access(
        &self,
        block: &syn::Block,
        known_modules: &HashSet<String>,
        forward_declable_types_by_module: &HashMap<String, HashSet<String>>,
        imported_name_to_module: &HashMap<String, String>,
        out: &mut HashSet<String>,
    ) {
        struct StrictMemberDependencyBlockVisitor<'a> {
            codegen: &'a CodeGen,
            known_modules: &'a HashSet<String>,
            forward_declable_types_by_module: &'a HashMap<String, HashSet<String>>,
            imported_name_to_module: &'a HashMap<String, String>,
            out: &'a mut HashSet<String>,
        }

        impl<'ast> Visit<'ast> for StrictMemberDependencyBlockVisitor<'_> {
            fn visit_path(&mut self, path: &'ast syn::Path) {
                if self.codegen.path_requires_complete_type_member_ordering(
                    path,
                    self.known_modules,
                    self.imported_name_to_module,
                ) {
                    // allow_skip=true: `Type::member` in a (phase-2) body is
                    // satisfied once the pre-pass has declared the type; only
                    // names with no early declaration force ordering.
                    self.codegen
                        .record_path_module_dependency_for_hard_dependencies(
                            path,
                            self.known_modules,
                            self.forward_declable_types_by_module,
                            self.imported_name_to_module,
                            true,
                            true,
                            self.out,
                        );
                }
                visit::visit_path(self, path);
            }
        }

        let mut visitor = StrictMemberDependencyBlockVisitor {
            codegen: self,
            known_modules,
            forward_declable_types_by_module,
            imported_name_to_module,
            out,
        };
        visitor.visit_block(block);
    }

    pub(super) fn collect_scope_module_hard_dependencies_with_imports(
        &self,
        items: &[syn::Item],
        known_modules: &HashSet<String>,
        forward_declable_types_by_module: &HashMap<String, HashSet<String>>,
        inherited_imports: &HashMap<String, String>,
        out: &mut HashSet<String>,
    ) {
        let mut imported_name_to_module: HashMap<String, String> = inherited_imports.clone();
        for item in items {
            let syn::Item::Use(u) = item else {
                continue;
            };
            let mut prefix: Vec<String> = Vec::new();
            self.collect_use_tree_module_aliases_for_hard_dependencies(
                &u.tree,
                &mut prefix,
                known_modules,
                forward_declable_types_by_module,
                &mut imported_name_to_module,
            );
            let mut glob_prefix: Vec<String> = Vec::new();
            self.collect_use_tree_glob_module_dependencies_for_hard_dependencies(
                &u.tree,
                &mut glob_prefix,
                known_modules,
                out,
            );
        }

        for item in items {
            match item {
                syn::Item::Struct(s) => match &s.fields {
                    syn::Fields::Named(fields) => {
                        for field in &fields.named {
                            self.collect_type_module_dependencies(
                                &field.ty,
                                known_modules,
                                forward_declable_types_by_module,
                                &imported_name_to_module,
                                true,
                                out,
                            );
                        }
                    }
                    syn::Fields::Unnamed(fields) => {
                        for field in &fields.unnamed {
                            self.collect_type_module_dependencies(
                                &field.ty,
                                known_modules,
                                forward_declable_types_by_module,
                                &imported_name_to_module,
                                true,
                                out,
                            );
                        }
                    }
                    syn::Fields::Unit => {}
                },
                syn::Item::Enum(e) => {
                    for variant in &e.variants {
                        match &variant.fields {
                            syn::Fields::Named(fields) => {
                                for field in &fields.named {
                                    self.collect_type_module_dependencies(
                                        &field.ty,
                                        known_modules,
                                        forward_declable_types_by_module,
                                        &imported_name_to_module,
                                        true,
                                        out,
                                    );
                                }
                            }
                            syn::Fields::Unnamed(fields) => {
                                for field in &fields.unnamed {
                                    self.collect_type_module_dependencies(
                                        &field.ty,
                                        known_modules,
                                        forward_declable_types_by_module,
                                        &imported_name_to_module,
                                        true,
                                        out,
                                    );
                                }
                            }
                            syn::Fields::Unit => {}
                        }
                    }
                }
                syn::Item::Union(u) => {
                    for field in &u.fields.named {
                        self.collect_type_module_dependencies(
                            &field.ty,
                            known_modules,
                            forward_declable_types_by_module,
                            &imported_name_to_module,
                            true,
                            out,
                        );
                    }
                }
                syn::Item::Fn(f) => {
                    for input in &f.sig.inputs {
                        let syn::FnArg::Typed(pat_ty) = input else {
                            continue;
                        };
                        self.collect_type_module_dependencies(
                            &pat_ty.ty,
                            known_modules,
                            forward_declable_types_by_module,
                            &imported_name_to_module,
                            false,
                            out,
                        );
                    }
                    if let syn::ReturnType::Type(_, ret_ty) = &f.sig.output {
                        self.collect_type_module_dependencies(
                            ret_ty,
                            known_modules,
                            forward_declable_types_by_module,
                            &imported_name_to_module,
                            false,
                            out,
                        );
                    }
                    self.collect_block_module_dependencies_for_hard_dependencies(
                        &f.block,
                        known_modules,
                        forward_declable_types_by_module,
                        &imported_name_to_module,
                        out,
                    );
                }
                syn::Item::Impl(imp) => {
                    for impl_item in &imp.items {
                        match impl_item {
                            syn::ImplItem::Const(c) => {
                                self.collect_type_module_dependencies(
                                    &c.ty,
                                    known_modules,
                                    forward_declable_types_by_module,
                                    &imported_name_to_module,
                                    true,
                                    out,
                                );
                                self.collect_expr_module_dependencies_for_hard_dependencies(
                                    &c.expr,
                                    known_modules,
                                    forward_declable_types_by_module,
                                    &imported_name_to_module,
                                    out,
                                );
                            }
                            syn::ImplItem::Type(t) => {
                                self.collect_type_module_dependencies(
                                    &t.ty,
                                    known_modules,
                                    forward_declable_types_by_module,
                                    &imported_name_to_module,
                                    false,
                                    out,
                                );
                            }
                            syn::ImplItem::Fn(method) => {
                                for input in &method.sig.inputs {
                                    match input {
                                        syn::FnArg::Receiver(_) => {}
                                        syn::FnArg::Typed(pat_ty) => {
                                            self.collect_type_module_dependencies(
                                                &pat_ty.ty,
                                                known_modules,
                                                forward_declable_types_by_module,
                                                &imported_name_to_module,
                                                false,
                                                out,
                                            );
                                        }
                                    }
                                }
                                if let syn::ReturnType::Type(_, ret_ty) = &method.sig.output {
                                    self.collect_type_module_dependencies(
                                        ret_ty,
                                        known_modules,
                                        forward_declable_types_by_module,
                                        &imported_name_to_module,
                                        false,
                                        out,
                                    );
                                }
                                self.collect_block_module_dependencies_for_hard_dependencies(
                                    &method.block,
                                    known_modules,
                                    forward_declable_types_by_module,
                                    &imported_name_to_module,
                                    out,
                                );
                            }
                            _ => {}
                        }
                    }
                }
                syn::Item::Type(t) => {
                    self.collect_type_module_dependencies(
                        &t.ty,
                        known_modules,
                        forward_declable_types_by_module,
                        &imported_name_to_module,
                        false,
                        out,
                    );
                }
                syn::Item::Const(c) => {
                    self.collect_type_module_dependencies(
                        &c.ty,
                        known_modules,
                        forward_declable_types_by_module,
                        &imported_name_to_module,
                        true,
                        out,
                    );
                    self.collect_expr_module_dependencies_for_hard_dependencies(
                        &c.expr,
                        known_modules,
                        forward_declable_types_by_module,
                        &imported_name_to_module,
                        out,
                    );
                }
                syn::Item::Static(s) => {
                    self.collect_type_module_dependencies(
                        &s.ty,
                        known_modules,
                        forward_declable_types_by_module,
                        &imported_name_to_module,
                        true,
                        out,
                    );
                    self.collect_expr_module_dependencies_for_hard_dependencies(
                        &s.expr,
                        known_modules,
                        forward_declable_types_by_module,
                        &imported_name_to_module,
                        out,
                    );
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested)) = &m.content {
                        self.collect_scope_module_hard_dependencies_with_imports(
                            nested,
                            known_modules,
                            forward_declable_types_by_module,
                            &imported_name_to_module,
                            out,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_use_tree_module_aliases_for_hard_dependencies(
        &self,
        tree: &syn::UseTree,
        prefix: &mut Vec<String>,
        known_modules: &HashSet<String>,
        forward_declable_types_by_module: &HashMap<String, HashSet<String>>,
        imported_name_to_module: &mut HashMap<String, String>,
    ) {
        match tree {
            syn::UseTree::Path(p) => {
                prefix.push(p.ident.to_string());
                self.collect_use_tree_module_aliases_for_hard_dependencies(
                    &p.tree,
                    prefix,
                    known_modules,
                    forward_declable_types_by_module,
                    imported_name_to_module,
                );
                let _ = prefix.pop();
            }
            syn::UseTree::Name(n) => {
                let mut full = prefix.clone();
                full.push(n.ident.to_string());
                self.record_use_alias_module_for_hard_dependencies(
                    &full,
                    &n.ident.to_string(),
                    known_modules,
                    forward_declable_types_by_module,
                    imported_name_to_module,
                );
            }
            syn::UseTree::Rename(r) => {
                let mut full = prefix.clone();
                full.push(r.ident.to_string());
                self.record_use_alias_module_for_hard_dependencies(
                    &full,
                    &r.rename.to_string(),
                    known_modules,
                    forward_declable_types_by_module,
                    imported_name_to_module,
                );
            }
            syn::UseTree::Glob(_) => {}
            syn::UseTree::Group(g) => {
                for item in &g.items {
                    self.collect_use_tree_module_aliases_for_hard_dependencies(
                        item,
                        prefix,
                        known_modules,
                        forward_declable_types_by_module,
                        imported_name_to_module,
                    );
                }
            }
        }
    }

    pub(super) fn collect_use_tree_glob_module_dependencies_for_hard_dependencies(
        &self,
        tree: &syn::UseTree,
        prefix: &mut Vec<String>,
        known_modules: &HashSet<String>,
        out: &mut HashSet<String>,
    ) {
        match tree {
            syn::UseTree::Path(p) => {
                prefix.push(p.ident.to_string());
                self.collect_use_tree_glob_module_dependencies_for_hard_dependencies(
                    &p.tree,
                    prefix,
                    known_modules,
                    out,
                );
                let _ = prefix.pop();
            }
            syn::UseTree::Name(_) | syn::UseTree::Rename(_) => {}
            syn::UseTree::Glob(_) => {
                self.record_module_dependency_from_segments(prefix, known_modules, out);
            }
            syn::UseTree::Group(g) => {
                for item in &g.items {
                    self.collect_use_tree_glob_module_dependencies_for_hard_dependencies(
                        item,
                        prefix,
                        known_modules,
                        out,
                    );
                }
            }
        }
    }

    pub(super) fn collect_type_module_dependencies(
        &self,
        ty: &syn::Type,
        known_modules: &HashSet<String>,
        forward_declable_types_by_module: &HashMap<String, HashSet<String>>,
        imported_name_to_module: &HashMap<String, String>,
        require_complete_types: bool,
        out: &mut HashSet<String>,
    ) {
        self.collect_type_module_dependencies_with_rehardening(
            ty,
            known_modules,
            forward_declable_types_by_module,
            imported_name_to_module,
            require_complete_types,
            true,
            false,
            out,
        );
    }

    pub(super) fn collect_type_module_dependencies_strict(
        &self,
        ty: &syn::Type,
        known_modules: &HashSet<String>,
        forward_declable_types_by_module: &HashMap<String, HashSet<String>>,
        imported_name_to_module: &HashMap<String, String>,
        require_complete_types: bool,
        out: &mut HashSet<String>,
    ) {
        self.collect_type_module_dependencies_with_rehardening(
            ty,
            known_modules,
            forward_declable_types_by_module,
            imported_name_to_module,
            require_complete_types,
            true,
            true,
            out,
        );
    }

    pub(super) fn collect_type_module_dependencies_with_rehardening(
        &self,
        ty: &syn::Type,
        known_modules: &HashSet<String>,
        forward_declable_types_by_module: &HashMap<String, HashSet<String>>,
        imported_name_to_module: &HashMap<String, String>,
        require_complete_types: bool,
        allow_rehardening_when_soft: bool,
        strict_member_ordering: bool,
        out: &mut HashSet<String>,
    ) {
        match ty {
            syn::Type::Path(tp) => {
                // `<Mapping as IntoIterator>::IntoIter` — the projection's
                // C++ lowering names a type owned by the qself's module, so
                // the qself type carries the dependency (the trait path does
                // not resolve locally).
                if let Some(qself) = &tp.qself {
                    self.collect_type_module_dependencies_with_rehardening(
                        &qself.ty,
                        known_modules,
                        forward_declable_types_by_module,
                        imported_name_to_module,
                        require_complete_types,
                        allow_rehardening_when_soft,
                        strict_member_ordering,
                        out,
                    );
                }
                self.record_path_module_dependency_for_hard_dependencies(
                    &tp.path,
                    known_modules,
                    forward_declable_types_by_module,
                    imported_name_to_module,
                    !require_complete_types,
                    false,
                    out,
                );
                for segment in &tp.path.segments {
                    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
                        continue;
                    };
                    let (nested_require_complete, nested_allow_rehardening_when_soft) =
                        if strict_member_ordering {
                            // A by-value field only needs its payload complete when
                            // the wrapper stores it inline. Pointer-backed containers
                            // (Box/Vec/maps/...) compile against a forward declaration
                            // — treating their payloads as strict fabricates module
                            // cycles (loader{Vec<(de::Event,_)>} ↔ de{Iterable(Loader)})
                            // whose fallback order then breaks the GENUINE inline edge.
                            // Cross-crate wrappers answer via their manifest's
                            // args_inline (transpiled iterators store args inline
                            // through decltype members; IndexMap does not).
                            if Self::segment_payload_is_pointer_indirect(&segment.ident)
                                || self
                                    .dep_manifest_wrapper_payload_is_pointer_indirect(
                                        &tp.path, segment,
                                    )
                                    .unwrap_or(false)
                            {
                                (false, false)
                            } else {
                                (
                                    require_complete_types || allow_rehardening_when_soft,
                                    allow_rehardening_when_soft,
                                )
                            }
                        } else {
                            match Self::segment_generic_payload_dependency_mode(&segment.ident) {
                                GenericPayloadDependencyMode::NonHardTransitive => (false, false),
                                GenericPayloadDependencyMode::NonHardShallow => {
                                    (false, allow_rehardening_when_soft)
                                }
                                GenericPayloadDependencyMode::Hard => (
                                    require_complete_types || allow_rehardening_when_soft,
                                    allow_rehardening_when_soft,
                                ),
                            }
                        };
                    for arg in &args.args {
                        if let syn::GenericArgument::Type(arg_ty) = arg {
                            self.collect_type_module_dependencies_with_rehardening(
                                arg_ty,
                                known_modules,
                                forward_declable_types_by_module,
                                imported_name_to_module,
                                nested_require_complete,
                                nested_allow_rehardening_when_soft,
                                strict_member_ordering,
                                out,
                            );
                        }
                    }
                }
            }
            syn::Type::Array(a) => {
                self.collect_type_module_dependencies_with_rehardening(
                    &a.elem,
                    known_modules,
                    forward_declable_types_by_module,
                    imported_name_to_module,
                    require_complete_types,
                    allow_rehardening_when_soft,
                    strict_member_ordering,
                    out,
                );
            }
            syn::Type::Group(g) => {
                self.collect_type_module_dependencies_with_rehardening(
                    &g.elem,
                    known_modules,
                    forward_declable_types_by_module,
                    imported_name_to_module,
                    require_complete_types,
                    allow_rehardening_when_soft,
                    strict_member_ordering,
                    out,
                );
            }
            syn::Type::Paren(p) => {
                self.collect_type_module_dependencies_with_rehardening(
                    &p.elem,
                    known_modules,
                    forward_declable_types_by_module,
                    imported_name_to_module,
                    require_complete_types,
                    allow_rehardening_when_soft,
                    strict_member_ordering,
                    out,
                );
            }
            syn::Type::Slice(s) => {
                self.collect_type_module_dependencies_with_rehardening(
                    &s.elem,
                    known_modules,
                    forward_declable_types_by_module,
                    imported_name_to_module,
                    require_complete_types,
                    allow_rehardening_when_soft,
                    strict_member_ordering,
                    out,
                );
            }
            syn::Type::Reference(r) => {
                let (nested_require_complete, nested_allow_rehardening_when_soft) =
                    if strict_member_ordering {
                        (require_complete_types, allow_rehardening_when_soft)
                    } else {
                        (false, false)
                    };
                self.collect_type_module_dependencies_with_rehardening(
                    &r.elem,
                    known_modules,
                    forward_declable_types_by_module,
                    imported_name_to_module,
                    nested_require_complete,
                    nested_allow_rehardening_when_soft,
                    strict_member_ordering,
                    out,
                );
            }
            syn::Type::Ptr(p) => {
                let (nested_require_complete, nested_allow_rehardening_when_soft) =
                    if strict_member_ordering {
                        (require_complete_types, allow_rehardening_when_soft)
                    } else {
                        (false, false)
                    };
                self.collect_type_module_dependencies_with_rehardening(
                    &p.elem,
                    known_modules,
                    forward_declable_types_by_module,
                    imported_name_to_module,
                    nested_require_complete,
                    nested_allow_rehardening_when_soft,
                    strict_member_ordering,
                    out,
                );
            }
            syn::Type::Tuple(t) => {
                for elem in &t.elems {
                    self.collect_type_module_dependencies_with_rehardening(
                        elem,
                        known_modules,
                        forward_declable_types_by_module,
                        imported_name_to_module,
                        require_complete_types,
                        allow_rehardening_when_soft,
                        strict_member_ordering,
                        out,
                    );
                }
            }
            syn::Type::BareFn(f) => {
                let (nested_require_complete, nested_allow_rehardening_when_soft) =
                    if strict_member_ordering {
                        (require_complete_types, allow_rehardening_when_soft)
                    } else {
                        (false, false)
                    };
                for input in &f.inputs {
                    self.collect_type_module_dependencies_with_rehardening(
                        &input.ty,
                        known_modules,
                        forward_declable_types_by_module,
                        imported_name_to_module,
                        nested_require_complete,
                        nested_allow_rehardening_when_soft,
                        strict_member_ordering,
                        out,
                    );
                }
                if let syn::ReturnType::Type(_, ret_ty) = &f.output {
                    self.collect_type_module_dependencies_with_rehardening(
                        ret_ty,
                        known_modules,
                        forward_declable_types_by_module,
                        imported_name_to_module,
                        nested_require_complete,
                        nested_allow_rehardening_when_soft,
                        strict_member_ordering,
                        out,
                    );
                }
            }
            _ => {}
        }
    }

    pub(super) fn collect_expr_module_dependencies_for_hard_dependencies(
        &self,
        expr: &syn::Expr,
        known_modules: &HashSet<String>,
        forward_declable_types_by_module: &HashMap<String, HashSet<String>>,
        imported_name_to_module: &HashMap<String, String>,
        out: &mut HashSet<String>,
    ) {
        struct HardDependencyExprVisitor<'a> {
            codegen: &'a CodeGen,
            known_modules: &'a HashSet<String>,
            forward_declable_types_by_module: &'a HashMap<String, HashSet<String>>,
            imported_name_to_module: &'a HashMap<String, String>,
            out: &'a mut HashSet<String>,
        }

        impl<'ast> Visit<'ast> for HardDependencyExprVisitor<'_> {
            fn visit_path(&mut self, path: &'ast syn::Path) {
                self.codegen
                    .record_path_module_dependency_for_hard_dependencies(
                        path,
                        self.known_modules,
                        self.forward_declable_types_by_module,
                        self.imported_name_to_module,
                        true,
                        false,
                        self.out,
                    );
                visit::visit_path(self, path);
            }

            fn visit_type(&mut self, ty: &'ast syn::Type) {
                self.codegen.collect_type_module_dependencies(
                    ty,
                    self.known_modules,
                    self.forward_declable_types_by_module,
                    self.imported_name_to_module,
                    false,
                    self.out,
                );
                visit::visit_type(self, ty);
            }
        }

        let mut visitor = HardDependencyExprVisitor {
            codegen: self,
            known_modules,
            forward_declable_types_by_module,
            imported_name_to_module,
            out,
        };
        visitor.visit_expr(expr);
    }

    pub(super) fn collect_block_module_dependencies_for_hard_dependencies(
        &self,
        block: &syn::Block,
        known_modules: &HashSet<String>,
        forward_declable_types_by_module: &HashMap<String, HashSet<String>>,
        imported_name_to_module: &HashMap<String, String>,
        out: &mut HashSet<String>,
    ) {
        struct HardDependencyBlockVisitor<'a> {
            codegen: &'a CodeGen,
            known_modules: &'a HashSet<String>,
            forward_declable_types_by_module: &'a HashMap<String, HashSet<String>>,
            imported_name_to_module: &'a HashMap<String, String>,
            out: &'a mut HashSet<String>,
        }

        impl<'ast> Visit<'ast> for HardDependencyBlockVisitor<'_> {
            fn visit_path(&mut self, path: &'ast syn::Path) {
                self.codegen
                    .record_path_module_dependency_for_hard_dependencies(
                        path,
                        self.known_modules,
                        self.forward_declable_types_by_module,
                        self.imported_name_to_module,
                        true,
                        false,
                        self.out,
                    );
                visit::visit_path(self, path);
            }

            fn visit_type(&mut self, ty: &'ast syn::Type) {
                self.codegen.collect_type_module_dependencies(
                    ty,
                    self.known_modules,
                    self.forward_declable_types_by_module,
                    self.imported_name_to_module,
                    false,
                    self.out,
                );
                visit::visit_type(self, ty);
            }
        }

        let mut visitor = HardDependencyBlockVisitor {
            codegen: self,
            known_modules,
            forward_declable_types_by_module,
            imported_name_to_module,
            out,
        };
        visitor.visit_block(block);
    }

    pub(super) fn collect_use_tree_module_dependencies(
        &self,
        tree: &syn::UseTree,
        prefix: &mut Vec<String>,
        known_modules: &HashSet<String>,
        out: &mut HashSet<String>,
    ) {
        match tree {
            syn::UseTree::Path(p) => {
                prefix.push(p.ident.to_string());
                self.collect_use_tree_module_dependencies(&p.tree, prefix, known_modules, out);
                let _ = prefix.pop();
            }
            syn::UseTree::Name(n) => {
                prefix.push(n.ident.to_string());
                self.record_module_dependency_from_segments(prefix, known_modules, out);
                let _ = prefix.pop();
            }
            syn::UseTree::Rename(r) => {
                prefix.push(r.ident.to_string());
                self.record_module_dependency_from_segments(prefix, known_modules, out);
                let _ = prefix.pop();
            }
            syn::UseTree::Glob(_) => {
                self.record_module_dependency_from_segments(prefix, known_modules, out);
            }
            syn::UseTree::Group(g) => {
                for item in &g.items {
                    self.collect_use_tree_module_dependencies(item, prefix, known_modules, out);
                }
            }
        }
    }

    pub(super) fn collect_forward_decl_sibling_dependencies_from_signature(
        sig: &syn::Signature,
        sibling_names: &HashSet<String>,
        reexported_type_to_sibling_module: Option<&HashMap<String, String>>,
        out: &mut HashSet<String>,
    ) {
        let mut collector = SiblingModulePathDependencyCollector {
            sibling_names,
            reexported_type_to_sibling_module,
            out,
        };
        collector.visit_signature(sig);
    }

    pub(super) fn collect_forward_decl_sibling_dependencies_from_type(
        ty: &syn::Type,
        sibling_names: &HashSet<String>,
        reexported_type_to_sibling_module: Option<&HashMap<String, String>>,
        out: &mut HashSet<String>,
    ) {
        let mut collector = SiblingModulePathDependencyCollector {
            sibling_names,
            reexported_type_to_sibling_module,
            out,
        };
        collector.visit_type(ty);
    }

    pub(super) fn collect_forward_decl_sibling_dependencies_from_path(
        path: &syn::Path,
        sibling_names: &HashSet<String>,
        reexported_type_to_sibling_module: Option<&HashMap<String, String>>,
        out: &mut HashSet<String>,
    ) {
        let mut collector = SiblingModulePathDependencyCollector {
            sibling_names,
            reexported_type_to_sibling_module,
            out,
        };
        collector.visit_path(path);
    }

    pub(super) fn collect_forward_decl_sibling_dependencies_from_generics(
        generics: &syn::Generics,
        sibling_names: &HashSet<String>,
        reexported_type_to_sibling_module: Option<&HashMap<String, String>>,
        out: &mut HashSet<String>,
    ) {
        let mut collector = SiblingModulePathDependencyCollector {
            sibling_names,
            reexported_type_to_sibling_module,
            out,
        };
        collector.visit_generics(generics);
    }

    pub(super) fn collect_forward_decl_sibling_dependencies_from_type_param_bound(
        bound: &syn::TypeParamBound,
        sibling_names: &HashSet<String>,
        reexported_type_to_sibling_module: Option<&HashMap<String, String>>,
        out: &mut HashSet<String>,
    ) {
        let mut collector = SiblingModulePathDependencyCollector {
            sibling_names,
            reexported_type_to_sibling_module,
            out,
        };
        collector.visit_type_param_bound(bound);
    }

    pub(super) fn collect_forward_decl_sibling_dependencies_from_use_tree(
        tree: &syn::UseTree,
        prefix: &mut Vec<String>,
        sibling_names: &HashSet<String>,
        out: &mut HashSet<String>,
    ) {
        match tree {
            syn::UseTree::Path(p) => {
                prefix.push(p.ident.to_string());
                Self::collect_forward_decl_sibling_dependencies_from_use_tree(
                    &p.tree,
                    prefix,
                    sibling_names,
                    out,
                );
                let _ = prefix.pop();
            }
            syn::UseTree::Name(n) => {
                prefix.push(n.ident.to_string());
                Self::record_forward_decl_sibling_dependency_from_segments(
                    prefix,
                    sibling_names,
                    out,
                );
                let _ = prefix.pop();
            }
            syn::UseTree::Rename(r) => {
                prefix.push(r.ident.to_string());
                Self::record_forward_decl_sibling_dependency_from_segments(
                    prefix,
                    sibling_names,
                    out,
                );
                let _ = prefix.pop();
            }
            syn::UseTree::Glob(_) => {
                Self::record_forward_decl_sibling_dependency_from_segments(
                    prefix,
                    sibling_names,
                    out,
                );
            }
            syn::UseTree::Group(g) => {
                for item in &g.items {
                    Self::collect_forward_decl_sibling_dependencies_from_use_tree(
                        item,
                        prefix,
                        sibling_names,
                        out,
                    );
                }
            }
        }
    }

    pub(super) fn collect_forward_decl_sibling_reexports_from_use_tree(
        tree: &syn::UseTree,
        prefix: &mut Vec<String>,
        sibling_names: &HashSet<String>,
        out: &mut HashMap<String, String>,
    ) {
        match tree {
            syn::UseTree::Path(p) => {
                prefix.push(p.ident.to_string());
                Self::collect_forward_decl_sibling_reexports_from_use_tree(
                    &p.tree,
                    prefix,
                    sibling_names,
                    out,
                );
                let _ = prefix.pop();
            }
            syn::UseTree::Name(n) => {
                prefix.push(n.ident.to_string());
                Self::record_forward_decl_sibling_reexport_from_segments(
                    prefix,
                    &n.ident.to_string(),
                    sibling_names,
                    out,
                );
                let _ = prefix.pop();
            }
            syn::UseTree::Rename(r) => {
                prefix.push(r.ident.to_string());
                Self::record_forward_decl_sibling_reexport_from_segments(
                    prefix,
                    &r.rename.to_string(),
                    sibling_names,
                    out,
                );
                let _ = prefix.pop();
            }
            syn::UseTree::Glob(_) => {}
            syn::UseTree::Group(g) => {
                for item in &g.items {
                    Self::collect_forward_decl_sibling_reexports_from_use_tree(
                        item,
                        prefix,
                        sibling_names,
                        out,
                    );
                }
            }
        }
    }

    pub(super) fn collect_forward_decl_sibling_type_reexports(
        items: &[&syn::Item],
        sibling_names: &HashSet<String>,
    ) -> HashMap<String, String> {
        let mut out = HashMap::new();
        for item in items {
            let syn::Item::Use(u) = item else {
                continue;
            };
            let mut prefix = Vec::new();
            Self::collect_forward_decl_sibling_reexports_from_use_tree(
                &u.tree,
                &mut prefix,
                sibling_names,
                &mut out,
            );
        }
        out
    }

    pub(super) fn collect_forward_decl_sibling_dependencies_from_items(
        items: &[syn::Item],
        sibling_names: &HashSet<String>,
        reexported_type_to_sibling_module: Option<&HashMap<String, String>>,
        out: &mut HashSet<String>,
    ) {
        for item in items {
            match item {
                syn::Item::Fn(f) => {
                    Self::collect_forward_decl_sibling_dependencies_from_signature(
                        &f.sig,
                        sibling_names,
                        reexported_type_to_sibling_module,
                        out,
                    );
                }
                syn::Item::Type(t) => {
                    Self::collect_forward_decl_sibling_dependencies_from_generics(
                        &t.generics,
                        sibling_names,
                        reexported_type_to_sibling_module,
                        out,
                    );
                    Self::collect_forward_decl_sibling_dependencies_from_type(
                        &t.ty,
                        sibling_names,
                        reexported_type_to_sibling_module,
                        out,
                    );
                }
                syn::Item::Struct(s) => {
                    Self::collect_forward_decl_sibling_dependencies_from_generics(
                        &s.generics,
                        sibling_names,
                        reexported_type_to_sibling_module,
                        out,
                    );
                    for field in &s.fields {
                        Self::collect_forward_decl_sibling_dependencies_from_type(
                            &field.ty,
                            sibling_names,
                            reexported_type_to_sibling_module,
                            out,
                        );
                    }
                }
                syn::Item::Enum(e) => {
                    Self::collect_forward_decl_sibling_dependencies_from_generics(
                        &e.generics,
                        sibling_names,
                        reexported_type_to_sibling_module,
                        out,
                    );
                    for variant in &e.variants {
                        for field in &variant.fields {
                            Self::collect_forward_decl_sibling_dependencies_from_type(
                                &field.ty,
                                sibling_names,
                                reexported_type_to_sibling_module,
                                out,
                            );
                        }
                    }
                }
                syn::Item::Union(u) => {
                    Self::collect_forward_decl_sibling_dependencies_from_generics(
                        &u.generics,
                        sibling_names,
                        reexported_type_to_sibling_module,
                        out,
                    );
                    for field in &u.fields.named {
                        Self::collect_forward_decl_sibling_dependencies_from_type(
                            &field.ty,
                            sibling_names,
                            reexported_type_to_sibling_module,
                            out,
                        );
                    }
                }
                syn::Item::Const(c) => {
                    Self::collect_forward_decl_sibling_dependencies_from_type(
                        &c.ty,
                        sibling_names,
                        reexported_type_to_sibling_module,
                        out,
                    );
                }
                syn::Item::Static(s) => {
                    Self::collect_forward_decl_sibling_dependencies_from_type(
                        &s.ty,
                        sibling_names,
                        reexported_type_to_sibling_module,
                        out,
                    );
                }
                syn::Item::Impl(i) => {
                    Self::collect_forward_decl_sibling_dependencies_from_generics(
                        &i.generics,
                        sibling_names,
                        reexported_type_to_sibling_module,
                        out,
                    );
                    if let Some((_, trait_path, _)) = &i.trait_ {
                        Self::collect_forward_decl_sibling_dependencies_from_path(
                            trait_path,
                            sibling_names,
                            reexported_type_to_sibling_module,
                            out,
                        );
                    }
                    Self::collect_forward_decl_sibling_dependencies_from_type(
                        &i.self_ty,
                        sibling_names,
                        reexported_type_to_sibling_module,
                        out,
                    );
                    for impl_item in &i.items {
                        match impl_item {
                            syn::ImplItem::Fn(f) => {
                                Self::collect_forward_decl_sibling_dependencies_from_signature(
                                    &f.sig,
                                    sibling_names,
                                    reexported_type_to_sibling_module,
                                    out,
                                );
                            }
                            syn::ImplItem::Const(c) => {
                                Self::collect_forward_decl_sibling_dependencies_from_type(
                                    &c.ty,
                                    sibling_names,
                                    reexported_type_to_sibling_module,
                                    out,
                                );
                            }
                            syn::ImplItem::Type(t) => {
                                Self::collect_forward_decl_sibling_dependencies_from_type(
                                    &t.ty,
                                    sibling_names,
                                    reexported_type_to_sibling_module,
                                    out,
                                );
                            }
                            _ => {}
                        }
                    }
                }
                syn::Item::Trait(t) => {
                    Self::collect_forward_decl_sibling_dependencies_from_generics(
                        &t.generics,
                        sibling_names,
                        reexported_type_to_sibling_module,
                        out,
                    );
                    for bound in &t.supertraits {
                        Self::collect_forward_decl_sibling_dependencies_from_type_param_bound(
                            bound,
                            sibling_names,
                            reexported_type_to_sibling_module,
                            out,
                        );
                    }
                    for trait_item in &t.items {
                        match trait_item {
                            syn::TraitItem::Fn(f) => {
                                Self::collect_forward_decl_sibling_dependencies_from_signature(
                                    &f.sig,
                                    sibling_names,
                                    reexported_type_to_sibling_module,
                                    out,
                                );
                            }
                            syn::TraitItem::Const(c) => {
                                Self::collect_forward_decl_sibling_dependencies_from_type(
                                    &c.ty,
                                    sibling_names,
                                    reexported_type_to_sibling_module,
                                    out,
                                );
                            }
                            syn::TraitItem::Type(t) => {
                                for bound in &t.bounds {
                                    Self::collect_forward_decl_sibling_dependencies_from_type_param_bound(
                                        bound,
                                        sibling_names,
                                        reexported_type_to_sibling_module,
                                        out,
                                    );
                                }
                                if let Some((_, default_ty)) = &t.default {
                                    Self::collect_forward_decl_sibling_dependencies_from_type(
                                        default_ty,
                                        sibling_names,
                                        reexported_type_to_sibling_module,
                                        out,
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                }
                syn::Item::Use(u) => {
                    let mut prefix = Vec::new();
                    Self::collect_forward_decl_sibling_dependencies_from_use_tree(
                        &u.tree,
                        &mut prefix,
                        sibling_names,
                        out,
                    );
                }
                _ => {}
            }
            if let syn::Item::Mod(nested) = item
                && let Some((_, nested_items)) = &nested.content
            {
                Self::collect_forward_decl_sibling_dependencies_from_items(
                    nested_items,
                    sibling_names,
                    reexported_type_to_sibling_module,
                    out,
                );
            }
        }
    }

    pub(super) fn collect_expanded_libtest_internal_function_renames(&mut self, items: &[syn::Item]) {
        if !self.is_non_root_expanded_test_module() {
            return;
        }
        for item in items {
            let syn::Item::Fn(f) = item else {
                continue;
            };
            if !self.should_emit_internal_linkage_function(f) {
                continue;
            }
            let rust_name = f.sig.ident.to_string();
            let Some(mapped) = self.libtest_internal_function_renamed_cpp_name(&rust_name) else {
                continue;
            };
            self.module_qualified_functions
                .entry(rust_name)
                .or_insert(mapped);
        }
    }

    /// An impl generic that is NOT a parameter of the self type but is fixed by
    /// a `where SelfParam: Iterator<Item = A>` bound is `SelfParam::Item`, not a
    /// free parameter. Left alone, the transpiler promotes `A` as a spurious
    /// `template<typename A>` onto every method (breaking trait-override shapes)
    /// and leaves `using Item = (A, A)` referencing an undeclared `A`. Rewrite
    /// the impl so `A` is replaced by `<SelfParam as Iterator>::Item` (which maps
    /// to `associated_item_t<SelfParam>`) in the assoc types, method signatures,
    /// and where-clause, and drop `A` from the impl generics. Method BODIES are
    /// deliberately left untouched so a body-local `type X<A> = ..` that reuses
    /// the name `A` keeps its own (shadowing) generic parameter.
    pub(super) fn normalize_impl_assoc_bound_generics(
        &self,
        imp: &syn::ItemImpl,
    ) -> Option<syn::ItemImpl> {
        let impl_type_params: std::collections::HashSet<String> = imp
            .generics
            .params
            .iter()
            .filter_map(|p| match p {
                syn::GenericParam::Type(tp) => Some(tp.ident.to_string()),
                _ => None,
            })
            .collect();
        if impl_type_params.is_empty() {
            return None;
        }
        let mut self_idents = std::collections::HashSet::new();
        Self::collect_type_referenced_idents(&imp.self_ty, &mut self_idents);
        let wc = imp.generics.where_clause.as_ref()?;
        let mut subs: Vec<(String, syn::Type)> = Vec::new();
        for pred in &wc.predicates {
            let syn::WherePredicate::Type(pt) = pred else {
                continue;
            };
            let syn::Type::Path(bounded) = &pt.bounded_ty else {
                continue;
            };
            let Some(x_name) = bounded.path.get_ident().map(|i| i.to_string()) else {
                continue;
            };
            if !self_idents.contains(&x_name) {
                continue;
            }
            for bound in &pt.bounds {
                let syn::TypeParamBound::Trait(tb) = bound else {
                    continue;
                };
                let Some(last) = tb.path.segments.last() else {
                    continue;
                };
                if last.ident != "Iterator" {
                    continue;
                }
                let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
                    continue;
                };
                for a in &args.args {
                    let syn::GenericArgument::AssocType(at) = a else {
                        continue;
                    };
                    if at.ident != "Item" {
                        continue;
                    }
                    let syn::Type::Path(item_tp) = &at.ty else {
                        continue;
                    };
                    let Some(p_name) = item_tp.path.get_ident().map(|i| i.to_string()) else {
                        continue;
                    };
                    if impl_type_params.contains(&p_name)
                        && !self_idents.contains(&p_name)
                        && !subs.iter().any(|(n, _)| n == &p_name)
                        && let Ok(proj) = syn::parse_str::<syn::Type>(&format!(
                            "<{} as Iterator>::Item",
                            x_name
                        ))
                    {
                        subs.push((p_name, proj));
                    }
                }
            }
        }
        if subs.is_empty() {
            return None;
        }
        let sub_names: std::collections::HashSet<String> =
            subs.iter().map(|(n, _)| n.clone()).collect();
        let mut rewritten = imp.clone();
        for (from, to) in &subs {
            let mut rep = TypeIdentReplacer {
                from: from.clone(),
                to: to.clone(),
            };
            for it in &mut rewritten.items {
                match it {
                    syn::ImplItem::Type(t) => rep.visit_type_mut(&mut t.ty),
                    syn::ImplItem::Fn(f) => {
                        for input in &mut f.sig.inputs {
                            if let syn::FnArg::Typed(pt) = input {
                                rep.visit_type_mut(&mut pt.ty);
                            }
                        }
                        if let syn::ReturnType::Type(_, ty) = &mut f.sig.output {
                            rep.visit_type_mut(ty);
                        }
                    }
                    _ => {}
                }
            }
            if let Some(wc) = &mut rewritten.generics.where_clause {
                for pred in &mut wc.predicates {
                    rep.visit_where_predicate_mut(pred);
                }
            }
        }
        rewritten.generics.params = rewritten
            .generics
            .params
            .into_iter()
            .filter(|p| match p {
                syn::GenericParam::Type(tp) => !sub_names.contains(&tp.ident.to_string()),
                _ => true,
            })
            .collect();
        Some(rewritten)
    }

    pub(super) fn collect_impl_blocks(&mut self, items: &[syn::Item], module_path: &[String]) {
        for item in items {
            match item {
                syn::Item::Impl(impl_block) => {
                    let normalized_impl = self.normalize_impl_assoc_bound_generics(impl_block);
                    let impl_block: &syn::ItemImpl =
                        normalized_impl.as_ref().unwrap_or(impl_block);
                    if Self::impl_self_type_path(impl_block.self_ty.as_ref()).is_none() {
                        // Self type isn't a path (e.g. `impl Trait for [T; N]`).
                        // We can't key these in `impl_blocks` (which is keyed by
                        // module-path-qualified names) but they may carry
                        // associated-type declarations that callers will want
                        // to resolve at call sites. Capture them now.
                        let self_cpp = self.map_type(impl_block.self_ty.as_ref());
                        if !self_cpp.is_empty()
                            && !self_cpp.contains("auto")
                            && !self_cpp.contains("/* TODO")
                        {
                            for impl_item in &impl_block.items {
                                if let syn::ImplItem::Type(assoc_type) = impl_item {
                                    let assoc_cpp = self.map_type(&assoc_type.ty);
                                    if !assoc_cpp.contains("auto")
                                        && !assoc_cpp.contains("/* TODO")
                                    {
                                        self.non_path_impl_assoc_types
                                            .entry(self_cpp.clone())
                                            .or_default()
                                            .insert(assoc_type.ident.to_string(), assoc_cpp);
                                    }
                                }
                            }
                        }
                        continue;
                    }
                    let tp = Self::impl_self_type_path(impl_block.self_ty.as_ref()).unwrap();

                    let raw_type_name = tp
                        .path
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    let type_name = qualify_impl_type_name(
                        &raw_type_name,
                        module_path,
                        &self.declared_item_names,
                        &self.local_declared_types,
                    );

                    let trait_path = impl_block.trait_.as_ref().map(|(_, path, _)| path);
                    let trait_name = trait_path
                        .and_then(|path| path.segments.last())
                        .map(|seg| seg.ident.to_string());
                    let is_inherent_impl = trait_name.is_none();
                    // Record `impl Iterator for Tail<Args..>` Item facts so
                    // item inference resolves map-shaped iterators
                    // (IntoIter<K, V> yields (K, V), not K).
                    if trait_name.as_deref() == Some("Iterator")
                        && let Some(item_ty) = impl_block.items.iter().find_map(|it| match it {
                            syn::ImplItem::Type(t) if t.ident == "Item" => {
                                Some(t.ty.clone())
                            }
                            _ => None,
                        })
                        && let Some(self_seg) = tp.path.segments.last()
                    {
                        let impl_params: Vec<String> = impl_block
                            .generics
                            .params
                            .iter()
                            .filter_map(|p| match p {
                                syn::GenericParam::Type(tp) => Some(tp.ident.to_string()),
                                _ => None,
                            })
                            .collect();
                        let self_args: Vec<syn::Type> = match &self_seg.arguments {
                            syn::PathArguments::AngleBracketed(a) => a
                                .args
                                .iter()
                                .filter_map(|arg| match arg {
                                    syn::GenericArgument::Type(t) => Some(t.clone()),
                                    _ => None,
                                })
                                .collect(),
                            _ => Vec::new(),
                        };
                        self.iterator_impl_items
                            .entry(self_seg.ident.to_string())
                            .or_default()
                            .push((impl_params, self_args, item_ty));
                    }

                    // Record user `Clone` impls (incl. expanded derives). Used
                    // by Drop-struct emission: a Drop type's only Rust
                    // duplication path is `Clone::clone`, so its C++ copy ctor
                    // must delegate to the emitted `clone()` member instead of
                    // `= default` (a shallow default copy of an owning field —
                    // semver Identifier's NonNull — double-frees at scope
                    // exit).
                    if matches!(trait_name.as_deref(), Some("Clone")) {
                        let simple = tp
                            .path
                            .segments
                            .last()
                            .map(|s| s.ident.to_string())
                            .unwrap_or_else(|| raw_type_name.clone());
                        let scoped = self.scoped_type_key(&simple);
                        self.types_with_user_clone.insert(simple);
                        self.types_with_user_clone.insert(type_name.clone());
                        self.types_with_user_clone.insert(scoped);
                    }

                    // Record `impl Deref for T { type Target = U }` so field /
                    // method access through Deref coercion can emit an explicit
                    // `(*x)` when the member lives on `U`, not `T` (C++ has no
                    // auto-deref). c2rust ports use this (unsafe-libyaml's
                    // `Success: Deref<Target = Failure>` for `.fail`/`.ok`).
                    if matches!(trait_name.as_deref(), Some("Deref") | Some("DerefMut")) {
                        for impl_item in &impl_block.items {
                            if let syn::ImplItem::Type(assoc) = impl_item
                                && assoc.ident == "Target"
                            {
                                let simple = tp
                                    .path
                                    .segments
                                    .last()
                                    .map(|s| s.ident.to_string())
                                    .unwrap_or_else(|| raw_type_name.clone());
                                let scoped = self.scoped_type_key(&simple);
                                self.user_deref_targets
                                    .insert(simple, assoc.ty.clone());
                                self.user_deref_targets
                                    .insert(type_name.clone(), assoc.ty.clone());
                                self.user_deref_targets.insert(scoped, assoc.ty.clone());
                            }
                        }
                    }

                    // `#[cpp_inherit] impl Trait for Type` → record that Type
                    // uses direct-inheritance emission (see cpp_inherit_trait
                    // doc in mod.rs). Keyed by the simple type name (matches
                    // `emit_struct`'s `s.ident`) and the module-scoped key.
                    if Self::has_cpp_inherit_attr(&impl_block.attrs) {
                        if let Some(trait_short) = &trait_name {
                            let simple_type_name = tp
                                .path
                                .segments
                                .last()
                                .map(|seg| seg.ident.to_string())
                                .unwrap_or_else(|| raw_type_name.clone());
                            self.cpp_inherit_trait
                                .insert(simple_type_name.clone(), trait_short.clone());
                            self.cpp_inherit_trait
                                .insert(type_name.clone(), trait_short.clone());
                            let scoped = self.scoped_type_key(&simple_type_name);
                            self.cpp_inherit_trait.insert(scoped, trait_short.clone());
                        }
                    }
                    // Borrowed `IntoIterator` impls (`impl IntoIterator for &T` / `&mut T`)
                    // collide with owned `into_iter(self)` when merged into one C++ struct:
                    // C++ cannot distinguish both non-const receiver shapes.
                    // Keep the owned impl and rely on `rusty::iter(...)` lowering for borrowed
                    // iterator entry points.
                    let skip_borrowed_into_iterator_impl = trait_name.as_deref()
                        == Some("IntoIterator")
                        && Self::impl_self_reference_mutability(impl_block.self_ty.as_ref())
                            .is_some();
                    if skip_borrowed_into_iterator_impl {
                        continue;
                    }
                    let is_drop_trait = trait_name.as_deref() == Some("Drop");
                    if trait_name.as_deref() == Some("Copy") {
                        self.copy_derived_types.insert(type_name.clone());
                        let scoped = self.scoped_type_key(&type_name);
                        self.copy_derived_types.insert(scoped);
                    }
                    if trait_name.as_deref() == Some("Display") {
                        self.display_impl_types.insert(type_name.clone());
                        let scoped = self.scoped_type_key(&type_name);
                        self.display_impl_types.insert(scoped);
                    }
                    // Types with a user `impl Iterator` are iterator
                    // receivers: adapter/terminal calls on them must route to
                    // the rusty:: free-function family (the struct has no
                    // `map`/`filter`/`count` members — and state fields like
                    // `count` would even SHADOW the trait method).
                    if trait_name.as_deref() == Some("Iterator") {
                        self.crate_iterator_impl_types.insert(type_name.clone());
                        let scoped = self.scoped_type_key(&type_name);
                        self.crate_iterator_impl_types.insert(scoped);
                    }
                    if trait_name.as_deref() == Some("IntoIterator") {
                        self.crate_intoiter_impl_types.insert(type_name.clone());
                        let scoped = self.scoped_type_key(&type_name);
                        self.crate_intoiter_impl_types.insert(scoped);
                    }
                    let op_name = trait_name
                        .as_ref()
                        .and_then(|name| map_operator_trait(name).map(|s| s.to_string()));
                    let impl_is_automatically_derived =
                        impl_block_is_automatically_derived(impl_block);
                    let type_is_declared_alias =
                        is_inherent_impl && self.type_key_is_declared_alias(&type_name);
                    let mut inherent_method_names_for_type: Vec<String> = Vec::new();
                    let mut alias_method_receiver_shapes: Vec<(String, bool)> = Vec::new();

                    // Record module path for methods merged from sibling modules
                    if !module_path.is_empty() {
                        self.impl_source_modules
                            .entry(type_name.clone())
                            .or_default()
                            .insert(module_path.join("::"));
                    }

                    let entry = self.impl_blocks.entry(type_name.clone()).or_default();
                    let seen_method_keys = self
                        .impl_method_conflict_keys
                        .entry(type_name.clone())
                        .or_default();
                    for impl_item in &impl_block.items {
                        if op_name.is_some() {
                            if let syn::ImplItem::Type(assoc) = impl_item {
                                // Operator traits use `type Output = ...`, but merged C++ methods
                                // don't require this alias and duplicate aliases cause conflicts.
                                if assoc.ident == "Output" {
                                    continue;
                                }
                            }
                        }
                        let mut collected_item = impl_item.clone();
                        if let syn::ImplItem::Fn(method) = impl_item {
                            let mut merged = method.clone();
                            // Cluster C: if this absorbed method is part of a
                            // parallel-impl group (sibling impl blocks of
                            // the same host with differing concrete markers),
                            // generalize the markers in the signature back
                            // to the host's class param name. This must
                            // happen BEFORE Cluster A's merge so the
                            // structural decomposition sees a signature
                            // with the host-param shape it expects.
                            let method_ident_str = merged.sig.ident.to_string();
                            let parallel_subs = self
                                .parallel_impl_substitutions
                                .get(&(raw_type_name.clone(), method_ident_str.clone()))
                                .or_else(|| {
                                    self.parallel_impl_substitutions
                                        .get(&(type_name.clone(), method_ident_str.clone()))
                                })
                                .cloned();
                            if let Some(subs) = parallel_subs {
                                Self::apply_parallel_impl_substitutions_to_fn(
                                    &mut merged,
                                    &subs,
                                );
                            }
                            // Cluster A: detect if this impl block's generics
                            // decompose structurally into host class params.
                            let host_class_params = self
                                .declared_type_params
                                .get(&type_name)
                                .cloned()
                                .or_else(|| {
                                    self.declared_type_params.get(&raw_type_name).cloned()
                                })
                                .unwrap_or_default();
                            let mut structural_decomp = detect_impl_structural_decomposition(
                                impl_block,
                                &host_class_params,
                            );
                            // Cluster A completion: fill in the per-position
                            // concrete C++ types from the impl block's inner
                            // args. Impl-generic positions stay None.
                            if let Some(decomp) = structural_decomp.as_mut() {
                                fill_decomp_inner_full_args(impl_block, decomp);
                            }
                            // Cluster A completion: record per-method
                            // decomposition so the method emitter can perform
                            // textual substitution of dropped-generic refs
                            // to `typename __TemplateArgs<HostParam>::arg_<N>`,
                            // and mark the inner struct as needing its
                            // `__TemplateArgs<...>` partial specialization.
                            if let Some(decomp) = &structural_decomp {
                                let method_ident_str = merged.sig.ident.to_string();
                                self.method_structural_decompositions.insert(
                                    (type_name.clone(), method_ident_str.clone()),
                                    decomp.clone(),
                                );
                                self.pending_template_args_specializations
                                    .insert(decomp.inner_struct.clone());
                            }
                            merge_impl_type_generics_into_method_with_decomp(
                                &mut merged,
                                &impl_block.generics,
                                structural_decomp.as_ref(),
                            );
                            Self::normalize_impl_method_receiver_for_reference_self(
                                &mut merged,
                                impl_block.self_ty.as_ref(),
                            );
                            if impl_is_automatically_derived {
                                mark_method_automatically_derived(&mut merged);
                            }
                            let key = impl_method_conflict_key(&merged);
                            if seen_method_keys.contains(&key) {
                                if let Some(existing_index) =
                                    find_impl_method_conflict_index(entry, &key)
                                {
                                    let should_replace = match &entry[existing_index] {
                                        syn::ImplItem::Fn(existing_method) => {
                                            should_replace_conflicting_impl_method(
                                                existing_method,
                                                &merged,
                                            )
                                        }
                                        _ => false,
                                    };
                                    if should_replace {
                                        entry[existing_index] = syn::ImplItem::Fn(merged);
                                    }
                                }
                                continue;
                            }
                            seen_method_keys.insert(key);
                            if let Some(op) = &op_name {
                                let method_name = merged.sig.ident.to_string();
                                self.operator_renames
                                    .insert((type_name.clone(), method_name), op.clone());
                            }
                            if is_drop_trait {
                                let method_name = merged.sig.ident.to_string();
                                self.drop_trait_methods
                                    .insert((type_name.clone(), method_name));
                            }
                            if is_inherent_impl {
                                let method_name = merged.sig.ident.to_string();
                                inherent_method_names_for_type.push(method_name.clone());
                                if type_is_declared_alias {
                                    alias_method_receiver_shapes.push((
                                        method_name,
                                        matches!(
                                            merged.sig.inputs.first(),
                                            Some(syn::FnArg::Receiver(_))
                                        ),
                                    ));
                                }
                            }
                            collected_item = syn::ImplItem::Fn(merged);
                        }
                        entry.push(collected_item);
                    }
                    if is_inherent_impl && !inherent_method_names_for_type.is_empty() {
                        let inherent_names = self
                            .inherent_impl_method_names
                            .entry(type_name.clone())
                            .or_default();
                        inherent_names.extend(inherent_method_names_for_type);
                    }
                    if type_is_declared_alias {
                        for (method_name, has_receiver) in alias_method_receiver_shapes {
                            self.record_alias_inherent_owner_method_has_receiver(
                                &type_name,
                                &method_name,
                                has_receiver,
                            );
                            if let Some(type_tail) = type_name.rsplit("::").next() {
                                self.record_alias_inherent_owner_method_has_receiver(
                                    type_tail,
                                    &method_name,
                                    has_receiver,
                                );
                            }
                        }
                    }

                    // Inject trait static default methods (no receiver) into the type.
                    // These are methods like `Flags::empty()`, `Flags::all()` that have
                    // default implementations in the trait but aren't in the explicit impl block.
                    if let Some(trait_path) = trait_path
                        && let Some(default_key) =
                            self.resolve_trait_static_default_key_for_impl(trait_path, module_path)
                    {
                        if let Some(static_defaults) =
                            self.trait_static_default_methods.get(&default_key).cloned()
                        {
                            let entry = self.impl_blocks.entry(type_name.clone()).or_default();
                            let seen_keys = self
                                .impl_method_conflict_keys
                                .entry(type_name.clone())
                                .or_default();
                            for default_fn in &static_defaults {
                                let key = impl_method_conflict_key(default_fn);
                                if !seen_keys.contains(&key) {
                                    seen_keys.insert(key);
                                    entry.push(syn::ImplItem::Fn(default_fn.clone()));
                                }
                            }
                        }
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested_items)) = &m.content {
                        let mut nested_path = module_path.to_vec();
                        nested_path.push(m.ident.to_string());
                        self.collect_impl_blocks(nested_items, &nested_path);
                    }
                }
                // Collect type aliases from impl blocks inside `const _: () = { ... }`.
                // For aliases pointing to const-block-local types (like InternalBitFlags),
                // resolve through to the underlying primitive type instead.
                syn::Item::Const(c) if c.ident == "_" => {
                    if let syn::Expr::Block(block) = c.expr.as_ref() {
                        // First, collect local struct definitions to detect newtype wrappers
                        let local_newtypes: HashMap<String, syn::Type> = block
                            .block
                            .stmts
                            .iter()
                            .filter_map(|stmt| {
                                if let syn::Stmt::Item(syn::Item::Struct(s)) = stmt {
                                    if let syn::Fields::Unnamed(fields) = &s.fields {
                                        if fields.unnamed.len() == 1 {
                                            return Some((
                                                s.ident.to_string(),
                                                fields.unnamed[0].ty.clone(),
                                            ));
                                        }
                                    }
                                }
                                None
                            })
                            .collect();

                        for stmt in &block.block.stmts {
                            if let syn::Stmt::Item(syn::Item::Impl(impl_block)) = stmt {
                                let Some(tp) =
                                    Self::impl_self_type_path(impl_block.self_ty.as_ref())
                                else {
                                    continue;
                                };
                                let raw_type_name = tp
                                    .path
                                    .segments
                                    .iter()
                                    .map(|s| s.ident.to_string())
                                    .collect::<Vec<_>>()
                                    .join("::");
                                let type_name = qualify_impl_type_name(
                                    &raw_type_name,
                                    module_path,
                                    &self.declared_item_names,
                                    &self.local_declared_types,
                                );
                                let type_tail = tp
                                    .path
                                    .segments
                                    .last()
                                    .map(|seg| seg.ident.to_string())
                                    .unwrap_or_default();
                                let trait_path =
                                    impl_block.trait_.as_ref().map(|(_, path, _)| path);
                                let trait_name = trait_path
                                    .as_ref()
                                    .and_then(|path| path.segments.last())
                                    .map(|seg| seg.ident.to_string());
                                let is_inherent_impl = trait_name.is_none();
                                let skip_borrowed_into_iterator_impl = trait_name.as_deref()
                                    == Some("IntoIterator")
                                    && Self::impl_self_reference_mutability(
                                        impl_block.self_ty.as_ref(),
                                    )
                                    .is_some();
                                if skip_borrowed_into_iterator_impl {
                                    continue;
                                }
                                let is_drop_trait = trait_name.as_deref() == Some("Drop");
                                // Check if this is an operator trait impl (BitOr, BitAnd, etc.)
                                let op_name = trait_name.as_ref().and_then(|name| {
                                    map_operator_trait(name).map(|s| s.to_string())
                                });
                                let impl_is_automatically_derived =
                                    impl_block_is_automatically_derived(impl_block);
                                let owner_is_locally_declared =
                                    self.local_declared_types.contains(&type_tail)
                                        || self.local_declared_types.contains(&type_name)
                                        || self.local_declared_types.contains(&raw_type_name)
                                        || self.declared_item_names.contains(&type_tail)
                                        || self.type_key_is_declared_alias(&type_name)
                                        || self.type_key_is_declared_alias(&raw_type_name);
                                let owner_is_const_block_derive_helper =
                                    type_tail.starts_with("__");
                                let allow_non_operator_const_impl_methods =
                                    owner_is_locally_declared
                                        && !owner_is_const_block_derive_helper
                                        && impl_is_automatically_derived;
                                let allow_local_serde_trait_impl_methods = owner_is_locally_declared
                                    && matches!(
                                        trait_name.as_deref(),
                                        Some(
                                            "Visitor"
                                                | "DeserializeSeed"
                                                | "SeqAccess"
                                                | "MapAccess"
                                                | "EnumAccess"
                                                | "VariantAccess"
                                        )
                                    );
                                // Keep historical operator trait extraction for const-block internals.
                                // For non-operator impls, we may still need associated type aliases
                                // (`type Internal = ...`) even when method extraction is disabled.
                                // Method-level filtering remains below.
                                let type_is_declared_alias =
                                    is_inherent_impl && self.type_key_is_declared_alias(&type_name);
                                let mut inherent_method_names_for_type: Vec<String> = Vec::new();
                                let mut alias_method_receiver_shapes: Vec<(String, bool)> =
                                    Vec::new();
                                let entry = self.impl_blocks.entry(type_name.clone()).or_default();

                                let seen_method_keys = self
                                    .impl_method_conflict_keys
                                    .entry(type_name.clone())
                                    .or_default();

                                for impl_item in &impl_block.items {
                                    if op_name.is_some()
                                        && let syn::ImplItem::Type(assoc) = impl_item
                                        && assoc.ident == "Output"
                                    {
                                        continue;
                                    }
                                    if let syn::ImplItem::Type(assoc) = impl_item {
                                        // Check if the alias target is a const-block-local type
                                        let target_name = match &assoc.ty {
                                            syn::Type::Path(tp) if tp.qself.is_none() => {
                                                tp.path.segments.last().map(|s| s.ident.to_string())
                                            }
                                            _ => None,
                                        };
                                        if let Some(ref target) = target_name {
                                            if let Some(inner_ty) = local_newtypes.get(target) {
                                                let mut resolved_assoc = assoc.clone();
                                                resolved_assoc.ty = inner_ty.clone();
                                                entry.push(syn::ImplItem::Type(resolved_assoc));
                                                continue;
                                            }
                                        }
                                        entry.push(impl_item.clone());
                                        continue;
                                    }
                                    if let syn::ImplItem::Fn(method) = impl_item {
                                        if op_name.is_none()
                                            && !allow_non_operator_const_impl_methods
                                            && !allow_local_serde_trait_impl_methods
                                        {
                                            continue;
                                        }
                                        let mut merged = method.clone();
                                        merge_impl_type_generics_into_method(
                                            &mut merged,
                                            &impl_block.generics,
                                        );
                                        Self::normalize_impl_method_receiver_for_reference_self(
                                            &mut merged,
                                            impl_block.self_ty.as_ref(),
                                        );
                                        if impl_is_automatically_derived {
                                            mark_method_automatically_derived(&mut merged);
                                        }
                                        let key = impl_method_conflict_key(&merged);
                                        if seen_method_keys.contains(&key) {
                                            if let Some(existing_index) =
                                                find_impl_method_conflict_index(entry, &key)
                                            {
                                                let should_replace = match &entry[existing_index] {
                                                    syn::ImplItem::Fn(existing_method) => {
                                                        should_replace_conflicting_impl_method(
                                                            existing_method,
                                                            &merged,
                                                        )
                                                    }
                                                    _ => false,
                                                };
                                                if should_replace {
                                                    entry[existing_index] =
                                                        syn::ImplItem::Fn(merged);
                                                }
                                            }
                                            continue;
                                        }
                                        seen_method_keys.insert(key);
                                        let method_name = merged.sig.ident.to_string();
                                        if let Some(op) = &op_name {
                                            self.operator_renames.insert(
                                                (type_name.clone(), method_name.clone()),
                                                op.clone(),
                                            );
                                        }
                                        if is_drop_trait {
                                            self.drop_trait_methods
                                                .insert((type_name.clone(), method_name.clone()));
                                        }
                                        if is_inherent_impl {
                                            inherent_method_names_for_type
                                                .push(method_name.clone());
                                            if type_is_declared_alias {
                                                alias_method_receiver_shapes.push((
                                                    method_name,
                                                    matches!(
                                                        merged.sig.inputs.first(),
                                                        Some(syn::FnArg::Receiver(_))
                                                    ),
                                                ));
                                            }
                                        }
                                        entry.push(syn::ImplItem::Fn(merged));
                                    }
                                }
                                if is_inherent_impl && !inherent_method_names_for_type.is_empty() {
                                    let inherent_names = self
                                        .inherent_impl_method_names
                                        .entry(type_name.clone())
                                        .or_default();
                                    inherent_names.extend(inherent_method_names_for_type);
                                }
                                if type_is_declared_alias {
                                    for (method_name, has_receiver) in alias_method_receiver_shapes
                                    {
                                        self.record_alias_inherent_owner_method_has_receiver(
                                            &type_name,
                                            &method_name,
                                            has_receiver,
                                        );
                                        if let Some(type_tail) = type_name.rsplit("::").next() {
                                            self.record_alias_inherent_owner_method_has_receiver(
                                                type_tail,
                                                &method_name,
                                                has_receiver,
                                            );
                                        }
                                    }
                                }
                                if let Some(trait_path) = trait_path
                                    && let Some(default_key) = self
                                        .resolve_trait_static_default_key_for_impl(
                                            trait_path,
                                            module_path,
                                        )
                                    && let Some(static_defaults) =
                                        self.trait_static_default_methods.get(&default_key).cloned()
                                {
                                    let entry =
                                        self.impl_blocks.entry(type_name.clone()).or_default();
                                    let seen_keys = self
                                        .impl_method_conflict_keys
                                        .entry(type_name.clone())
                                        .or_default();
                                    for default_fn in &static_defaults {
                                        let key = impl_method_conflict_key(default_fn);
                                        if !seen_keys.contains(&key) {
                                            seen_keys.insert(key);
                                            entry.push(syn::ImplItem::Fn(default_fn.clone()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Collect trait default static methods (no `self` receiver) so they can be
    /// injected into implementing types' struct definitions.
    /// Instance methods (with receivers) are NOT injected because they have
    /// complex dependencies on `Self::Bits`, `Self::FLAGS`, etc. that don't
    /// resolve correctly when transplanted to a different type context.
    /// Collect `pub use module::{Name1, Name2}` re-exports (crate-root and nested).
    /// These map simple names to their source module, so `use crate::Flags`
    /// can resolve to `traits::Flags` when `pub use traits::Flags;` exists.
    /// Also records fully-qualified target paths to force-export re-exported
    /// declarations that would otherwise have module linkage.
    pub(super) fn collect_crate_reexports(&mut self, items: &[syn::Item]) {
        self.collect_crate_reexports_recursive(items, &[]);
    }

    pub(super) fn collect_crate_reexports_recursive(&mut self, items: &[syn::Item], module_path: &[String]) {
        let saved_stack = self.module_stack.clone();
        self.module_stack = module_path.to_vec();
        for item in items {
            match item {
                syn::Item::Use(u) => {
                    if !matches!(u.vis, syn::Visibility::Public(_)) {
                        continue;
                    }
                    for raw in self.flatten_use_tree(&u.tree, "") {
                        let normalized = normalize_use_import_path(&raw);
                        if normalized.starts_with("namespace ") {
                            continue;
                        }
                        if let Some((alias, target)) = split_use_import_alias(&normalized) {
                            let target = target.trim().trim_start_matches("::");
                            if target.is_empty() {
                                continue;
                            }
                            if let Some((prefix, name)) = target.rsplit_once("::") {
                                self.crate_reexports
                                    .insert(alias.trim().to_string(), prefix.to_string());
                                if name != "self" {
                                    self.crate_pub_reexport_targets
                                        .insert(format!("{}::{}", prefix, name));
                                }
                            }
                            continue;
                        }
                        let path = normalized.trim().trim_start_matches("::");
                        if let Some((prefix, name)) = path.rsplit_once("::") {
                            self.crate_reexports
                                .insert(name.to_string(), prefix.to_string());
                            self.crate_pub_reexport_targets
                                .insert(format!("{}::{}", prefix, name));
                        }
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested_items)) = &m.content {
                        let mut nested_path = module_path.to_vec();
                        nested_path.push(m.ident.to_string());
                        self.collect_crate_reexports_recursive(nested_items, &nested_path);
                    }
                }
                _ => {}
            }
        }
        self.module_stack = saved_stack;
    }

    pub(super) fn collect_trait_method_receiver_shapes(
        &mut self,
        items: &[syn::Item],
        module_path: &[String],
    ) {
        for item in items {
            match item {
                syn::Item::Trait(t) => {
                    let trait_name = t.ident.to_string();
                    let scoped_trait_name = if module_path.is_empty() {
                        trait_name.clone()
                    } else {
                        format!("{}::{}", module_path.join("::"), trait_name)
                    };
                    for trait_item in &t.items {
                        let syn::TraitItem::Fn(method) = trait_item else {
                            continue;
                        };
                        let method_name = method.sig.ident.to_string();
                        let has_receiver =
                            matches!(method.sig.inputs.first(), Some(syn::FnArg::Receiver(_)));
                        std::rc::Rc::make_mut(&mut self.trait_method_has_receiver)
                            .insert(format!("{}::{}", trait_name, method_name), has_receiver);
                        std::rc::Rc::make_mut(&mut self.trait_method_has_receiver).insert(
                            format!("{}::{}", scoped_trait_name, method_name),
                            has_receiver,
                        );
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested_items)) = &m.content {
                        let mut nested_path = module_path.to_vec();
                        nested_path.push(m.ident.to_string());
                        self.collect_trait_method_receiver_shapes(nested_items, &nested_path);
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_trait_static_default_methods(
        &mut self,
        items: &[syn::Item],
        module_path: &[String],
    ) {
        for item in items {
            match item {
                syn::Item::Trait(t) => {
                    let trait_name = t.ident.to_string();
                    let scoped_trait_name = qualify_impl_type_name(
                        &trait_name,
                        module_path,
                        &self.declared_item_names,
                        &self.local_declared_types,
                    );
                    self.trait_declared_paths.insert(scoped_trait_name.clone());
                    // Maintain short-name → first-scoped-path index used by
                    // the supertrait qualification lookup. Only insert if
                    // not present so the first declaration wins (matches
                    // the prior `find` behavior over the HashSet).
                    let short = scoped_trait_name
                        .rsplit("::")
                        .next()
                        .unwrap_or(&scoped_trait_name)
                        .to_string();
                    self.trait_declared_path_by_short_name
                        .entry(short)
                        .or_insert_with(|| scoped_trait_name.clone());
                    let mut static_defaults = Vec::new();
                    for trait_item in &t.items {
                        // Associated CONSTS with a default body: record `NAME →
                        // (body, trait)` so a type-param access `T::NAME` lowers to
                        // the default body (the trait itself is skipped — see the
                        // interface_traits TODO). First declaration wins.
                        if let syn::TraitItem::Const(c) = trait_item
                            && let Some((_, default_expr)) = &c.default
                        {
                            self.trait_default_const_exprs
                                .entry(c.ident.to_string())
                                .or_insert_with(|| (default_expr.clone(), trait_name.clone()));
                        }
                        if let syn::TraitItem::Fn(method) = trait_item {
                            let has_default = method.default.is_some();
                            let has_receiver =
                                matches!(method.sig.inputs.first(), Some(syn::FnArg::Receiver(_)));
                            // Only collect static methods (no receiver) with default bodies.
                            // Instance methods have complex dependencies (Self::Bits,
                            // iter::Iter<Self>) that cause namespace collisions and type
                            // resolution issues when injected into implementing types.
                            if has_default && !has_receiver {
                                let impl_fn = syn::ImplItemFn {
                                    attrs: method.attrs.clone(),
                                    vis: syn::Visibility::Public(syn::token::Pub::default()),
                                    defaultness: None,
                                    sig: method.sig.clone(),
                                    block: method.default.clone().unwrap(),
                                };
                                static_defaults.push(impl_fn);
                            }
                        }
                    }
                    if !static_defaults.is_empty() {
                        self.trait_static_default_methods
                            .insert(scoped_trait_name, static_defaults);
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested_items)) = &m.content {
                        let mut nested_path = module_path.to_vec();
                        nested_path.push(m.ident.to_string());
                        self.collect_trait_static_default_methods(nested_items, &nested_path);
                    }
                }
                _ => {}
            }
        }
    }

    /// Pre-pass: identify traits that `emit_trait_interface_pattern`
    /// will skip (no `&self` methods, no usable supertraits). Populating
    /// `skipped_interface_traits` upfront lets *other* traits that name a
    /// skipped trait as their own supertrait correctly drop the supertrait
    /// from their inheritance list during emission. Without this, a derived
    /// trait emitted before its skipped supertrait would emit
    /// `class Derived : public ::ns::Skipped {}` and fail at compile time.
    /// Iterates to a fixpoint so chains of empty traits all get marked.
    pub(super) fn collect_skipped_interface_traits(&mut self, items: &[syn::Item]) {
        let mut all_traits: Vec<&syn::ItemTrait> = Vec::new();
        Self::collect_all_traits(items, &mut all_traits);
        loop {
            let mut changed = false;
            for t in &all_traits {
                let trait_name = t.ident.to_string();
                if self.skipped_interface_traits.contains(&trait_name) {
                    continue;
                }
                let any_emittable_method = t.items.iter().any(|item| {
                    let syn::TraitItem::Fn(method) = item else {
                        return false;
                    };
                    if !method.sig.generics.params.is_empty() {
                        return false;
                    }
                    let receiver = method.sig.inputs.first();
                    let Some(syn::FnArg::Receiver(r)) = receiver else {
                        return false;
                    };
                    r.reference.is_some()
                });
                let has_emittable_supertrait = t.supertraits.iter().any(|b| {
                    let syn::TypeParamBound::Trait(tb) = b else {
                        return false;
                    };
                    let Some(seg) = tb.path.segments.last() else {
                        return false;
                    };
                    let name = seg.ident.to_string();
                    if matches!(
                        name.as_str(),
                        "Send" | "Sync" | "Copy" | "Clone" | "Sized" | "Unpin"
                    ) {
                        return false;
                    }
                    if map_operator_trait(&name).is_some() {
                        return false;
                    }
                    if self.skipped_interface_traits.contains(&name) {
                        return false;
                    }
                    self.trait_declared_path_by_short_name
                        .contains_key(&name)
                });
                if !any_emittable_method && !has_emittable_supertrait {
                    self.skipped_interface_traits.insert(trait_name);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }

    pub(super) fn collect_all_traits<'a>(items: &'a [syn::Item], out: &mut Vec<&'a syn::ItemTrait>) {
        for item in items {
            match item {
                syn::Item::Trait(t) => out.push(t),
                syn::Item::Mod(m) => {
                    if let Some((_, nested)) = &m.content {
                        Self::collect_all_traits(nested, out);
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_macro_rules_names(&mut self, items: &[syn::Item]) {
        for item in items {
            match item {
                syn::Item::Macro(m) => {
                    if m.mac.path.is_ident("macro_rules") {
                        if let Some(ident) = &m.ident {
                            self.macro_rules_names.insert(ident.to_string());
                        }
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested_items)) = &m.content {
                        self.collect_macro_rules_names(nested_items);
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_import_alias_names(&mut self, items: &[syn::Item]) {
        for item in items {
            match item {
                syn::Item::Use(u) => self.collect_import_alias_names_in_tree(&u.tree),
                syn::Item::Mod(m) => {
                    if let Some((_, nested_items)) = &m.content {
                        self.collect_import_alias_names(nested_items);
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_import_alias_names_in_tree(&mut self, tree: &syn::UseTree) {
        match tree {
            syn::UseTree::Rename(rename) => {
                self.import_alias_names.insert(rename.rename.to_string());
            }
            syn::UseTree::Path(path) => self.collect_import_alias_names_in_tree(&path.tree),
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.collect_import_alias_names_in_tree(item);
                }
            }
            syn::UseTree::Name(_) | syn::UseTree::Glob(_) => {}
        }
    }

    /// Collect crate-wide `extern crate <crate> as <alias>;` renames as alias
    /// edges in the name resolver (`alias -> crate`). `extern crate` aliases are
    /// crate-global in Rust, so this recurses the whole item tree (including
    /// `const _: () = { … }` expansion wrappers) regardless of module scope.
    pub(super) fn collect_extern_crate_aliases(&mut self, items: &[syn::Item]) {
        for item in items {
            match item {
                syn::Item::ExternCrate(ec) => {
                    if let Some((_, rename)) = &ec.rename {
                        let alias = rename.to_string();
                        let target = ec.ident.to_string();
                        if alias != "_" {
                            self.name_resolver.add_alias(alias, target);
                        }
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested)) = &m.content {
                        self.collect_extern_crate_aliases(nested);
                    }
                }
                syn::Item::Const(c) if c.ident == "_" => {
                    if let syn::Expr::Block(block) = c.expr.as_ref() {
                        for stmt in &block.block.stmts {
                            if let syn::Stmt::Item(nested) = stmt {
                                self.collect_extern_crate_aliases(std::slice::from_ref(nested));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Collect intra-crate `use <module> as <alias>;` renames as alias edges in
    /// the name resolver (`<scope>::<alias>` -> `<scope>::<target-module>`).
    /// Two walks: first gather every declared module's full path, then record a
    /// rename only when its target resolves to one of those modules (so a
    /// type/value `use Foo as Bar` is never mistaken for a module alias).
    pub(super) fn collect_module_path_aliases(&mut self, items: &[syn::Item]) {
        let mut modules: HashSet<String> = HashSet::new();
        Self::collect_module_full_paths(items, &mut Vec::new(), &mut modules);
        self.collect_module_path_aliases_inner(items, &[], &modules);
    }

    fn collect_module_full_paths(
        items: &[syn::Item],
        prefix: &mut Vec<String>,
        out: &mut HashSet<String>,
    ) {
        for item in items {
            if let syn::Item::Mod(m) = item {
                prefix.push(m.ident.to_string());
                out.insert(prefix.join("::"));
                if let Some((_, nested)) = &m.content {
                    Self::collect_module_full_paths(nested, prefix, out);
                }
                prefix.pop();
            }
        }
    }

    fn collect_module_path_aliases_inner(
        &mut self,
        items: &[syn::Item],
        module_path: &[String],
        modules: &HashSet<String>,
    ) {
        for item in items {
            match item {
                syn::Item::Use(u) => {
                    self.record_module_path_alias_from_use_tree(&u.tree, module_path, modules);
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested)) = &m.content {
                        let mut nested_path = module_path.to_vec();
                        nested_path.push(m.ident.to_string());
                        self.collect_module_path_aliases_inner(nested, &nested_path, modules);
                    }
                }
                _ => {}
            }
        }
    }

    fn record_module_path_alias_from_use_tree(
        &mut self,
        tree: &syn::UseTree,
        module_path: &[String],
        modules: &HashSet<String>,
    ) {
        self.record_module_path_alias_inner(tree, module_path, modules, &[]);
    }

    fn record_module_path_alias_inner(
        &mut self,
        tree: &syn::UseTree,
        module_path: &[String],
        modules: &HashSet<String>,
        path_prefix: &[String],
    ) {
        match tree {
            syn::UseTree::Rename(r) => {
                let target_seg = r.ident.to_string();
                let alias = r.rename.to_string();
                if matches!(target_seg.as_str(), "self" | "super" | "crate") || alias == "_" {
                    return;
                }
                let alias_full = if module_path.is_empty() {
                    alias.clone()
                } else {
                    format!("{}::{}", module_path.join("::"), alias)
                };
                // CROSS-CRATE (refactor step M2): `use serde_core::__private as serde_core_private`
                // is rooted at a namespace-WRAPPED dependency, so the rename targets the DEP's
                // module. Record the FULL crate-qualified target so resolving `serde_core_private::X`
                // lands in the dependency — never a same-named LOCAL module (the cross-crate
                // stripping bug). Resolve a hygiene shell (`__private228`) to its canonical at
                // recording time so the edge points directly at the real module
                // (`serde_core::private_`). M1's complete module recognition keeps the resulting
                // crate-qualified module references emitting as namespace aliases (no cascade).
                if let Some(root) = path_prefix.first() {
                    if crate::transpile::crate_is_namespace_wrapped(root) {
                        let full_target = match self
                            .dependency_ufcs_trait_manifests
                            .iter()
                            .find(|m| &m.module == root)
                            .and_then(|dep| dep.hygiene_aliases.get(&target_seg))
                        {
                            Some(canonical) => format!("{}::{}", root, canonical),
                            None => path_prefix
                                .iter()
                                .cloned()
                                .chain(std::iter::once(target_seg))
                                .collect::<Vec<_>>()
                                .join("::"),
                        };
                        self.name_resolver.add_alias(alias_full, full_target);
                        return;
                    }
                }
                // Resolve the single-segment rename target relative to the current module (child)
                // or the crate root (sibling/top-level), against the LOCAL module set.
                let mut child = module_path.to_vec();
                child.push(target_seg.clone());
                for cand in [child.join("::"), target_seg.clone()] {
                    if modules.contains(&cand) {
                        self.name_resolver.add_alias(alias_full, cand);
                        return;
                    }
                }
            }
            syn::UseTree::Path(p) => {
                let mut next = path_prefix.to_vec();
                next.push(p.ident.to_string());
                self.record_module_path_alias_inner(&p.tree, module_path, modules, &next);
            }
            syn::UseTree::Group(g) => {
                for t in &g.items {
                    self.record_module_path_alias_inner(t, module_path, modules, path_prefix);
                }
            }
            _ => {}
        }
    }

    pub(super) fn collect_scope_import_bindings(&mut self, items: &[syn::Item], module_path: &[String]) {
        let prev_stack = self.module_stack.clone();
        self.module_stack = module_path.to_vec();

        for item in items {
            match item {
                syn::Item::ExternCrate(ec) => {
                    let local_name = ec
                        .rename
                        .as_ref()
                        .map(|(_, rename)| rename.to_string())
                        .unwrap_or_else(|| ec.ident.to_string());
                    let target = ec.ident.to_string();
                    self.record_scope_import_binding(
                        module_path,
                        &format!("{} = {}", local_name, target),
                    );
                }
                syn::Item::Use(u) => {
                    for raw_path in self.flatten_use_tree_preserve_crate(&u.tree, "") {
                        self.record_scope_import_binding(module_path, &raw_path);
                    }
                }
                syn::Item::Const(c) if c.ident == "_" => {
                    if let syn::Expr::Block(block) = c.expr.as_ref() {
                        for stmt in &block.block.stmts {
                            let syn::Stmt::Item(nested_item) = stmt else {
                                continue;
                            };
                            match nested_item {
                                syn::Item::Use(u) => {
                                    for raw_path in
                                        self.flatten_use_tree_preserve_crate(&u.tree, "")
                                    {
                                        self.record_scope_import_binding(module_path, &raw_path);
                                    }
                                }
                                syn::Item::ExternCrate(ec) => {
                                    let local_name = ec
                                        .rename
                                        .as_ref()
                                        .map(|(_, rename)| rename.to_string())
                                        .unwrap_or_else(|| ec.ident.to_string());
                                    let target = ec.ident.to_string();
                                    self.record_scope_import_binding(
                                        module_path,
                                        &format!("{} = {}", local_name, target),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested_items)) = &m.content {
                        let mut nested_path = module_path.to_vec();
                        nested_path.push(m.ident.to_string());
                        self.collect_scope_import_bindings(nested_items, &nested_path);
                    }
                }
                _ => {}
            }
        }

        self.module_stack = prev_stack;
    }

    pub(super) fn collect_local_impl_overrides(
        &self,
        stmts: &[syn::Stmt],
        local_types: &HashSet<String>,
    ) -> (
        HashMap<String, Vec<syn::ImplItem>>,
        HashSet<(String, String)>,
        HashMap<(String, String), String>,
        HashMap<String, HashSet<String>>,
    ) {
        let mut local_impl_blocks: HashMap<String, Vec<syn::ImplItem>> = HashMap::new();
        let mut local_drop_trait_methods: HashSet<(String, String)> = HashSet::new();
        let mut local_operator_renames: HashMap<(String, String), String> = HashMap::new();
        let mut local_inherent_method_names: HashMap<String, HashSet<String>> = HashMap::new();
        let mut local_impl_method_conflict_keys: HashMap<String, HashSet<String>> = HashMap::new();

        for stmt in stmts {
            let syn::Stmt::Item(syn::Item::Impl(impl_block)) = stmt else {
                continue;
            };
            let Some(tp) = Self::impl_self_type_path(impl_block.self_ty.as_ref()) else {
                continue;
            };

            let Some(type_seg) = tp.path.segments.last() else {
                continue;
            };
            let type_name = type_seg.ident.to_string();
            if !local_types.contains(&type_name) {
                continue;
            }

            let trait_name = impl_block
                .trait_
                .as_ref()
                .and_then(|(_, path, _)| path.segments.last())
                .map(|seg| seg.ident.to_string());
            let is_inherent_impl = trait_name.is_none();
            let skip_borrowed_into_iterator_impl = trait_name.as_deref() == Some("IntoIterator")
                && Self::impl_self_reference_mutability(impl_block.self_ty.as_ref()).is_some();
            if skip_borrowed_into_iterator_impl {
                continue;
            }
            let is_drop_trait = trait_name.as_deref() == Some("Drop");
            let impl_is_automatically_derived = impl_block_is_automatically_derived(impl_block);
            let is_default_trait = trait_name.as_deref() == Some("Default");
            let is_local_serde_trait_impl = matches!(
                trait_name.as_deref(),
                Some(
                    "Visitor"
                        | "DeserializeSeed"
                        | "SeqAccess"
                        | "MapAccess"
                        | "EnumAccess"
                        | "VariantAccess"
                )
            );
            // fmt-family impls absorb as members too: the runtime's
            // write_fmt/format dispatch duck-probes `writer.write_str(…)` /
            // `v.fmt(…)`, so a member on the (wrapper-enum or struct) host
            // is exactly what makes a fn-local `impl fmt::Write for T`
            // callable after T hoists to namespace scope.
            let is_local_fmt_trait_impl =
                matches!(trait_name.as_deref(), Some("Write" | "Display" | "Debug"));
            let allow_non_inherent_trait_impl = is_drop_trait
                || is_default_trait
                || impl_is_automatically_derived
                || is_local_serde_trait_impl
                || is_local_fmt_trait_impl;
            if !is_inherent_impl && !allow_non_inherent_trait_impl {
                continue;
            }
            let op_name = trait_name
                .as_ref()
                .and_then(|name| map_operator_trait(name).map(|s| s.to_string()));

            let entry = local_impl_blocks.entry(type_name.clone()).or_default();
            let seen_method_keys = local_impl_method_conflict_keys
                .entry(type_name.clone())
                .or_default();
            for impl_item in &impl_block.items {
                if op_name.is_some() {
                    if let syn::ImplItem::Type(assoc) = impl_item {
                        if assoc.ident == "Output" {
                            continue;
                        }
                    }
                }
                let mut collected_item = impl_item.clone();
                if let syn::ImplItem::Fn(method) = impl_item {
                    let mut merged = method.clone();
                    merge_impl_type_generics_into_method(&mut merged, &impl_block.generics);
                    Self::normalize_impl_method_receiver_for_reference_self(
                        &mut merged,
                        impl_block.self_ty.as_ref(),
                    );
                    if impl_is_automatically_derived {
                        mark_method_automatically_derived(&mut merged);
                    }
                    let key = impl_method_conflict_key(&merged);
                    if seen_method_keys.contains(&key) {
                        if let Some(existing_index) = find_impl_method_conflict_index(entry, &key) {
                            let should_replace = match &entry[existing_index] {
                                syn::ImplItem::Fn(existing_method) => {
                                    should_replace_conflicting_impl_method(existing_method, &merged)
                                }
                                _ => false,
                            };
                            if should_replace {
                                entry[existing_index] = syn::ImplItem::Fn(merged);
                            }
                        }
                        continue;
                    }
                    seen_method_keys.insert(key);
                    if let Some(op) = &op_name {
                        let method_name = merged.sig.ident.to_string();
                        local_operator_renames.insert((type_name.clone(), method_name), op.clone());
                    }
                    if is_drop_trait {
                        let method_name = merged.sig.ident.to_string();
                        local_drop_trait_methods.insert((type_name.clone(), method_name));
                    }
                    if is_inherent_impl {
                        local_inherent_method_names
                            .entry(type_name.clone())
                            .or_default()
                            .insert(merged.sig.ident.to_string());
                    }
                    collected_item = syn::ImplItem::Fn(merged);
                }
                entry.push(collected_item);
            }
        }

        (
            local_impl_blocks,
            local_drop_trait_methods,
            local_operator_renames,
            local_inherent_method_names,
        )
    }

    pub(super) fn collect_top_level_item_names(&mut self, items: &[syn::Item]) {
        for item in items {
            match item {
                syn::Item::Fn(f) => {
                    self.declared_item_names.insert(f.sig.ident.to_string());
                }
                syn::Item::Struct(s) => {
                    self.declared_item_names.insert(s.ident.to_string());
                }
                syn::Item::Enum(e) => {
                    self.declared_item_names.insert(e.ident.to_string());
                }
                syn::Item::Type(t) => {
                    self.declared_item_names.insert(t.ident.to_string());
                }
                syn::Item::Const(c) => {
                    self.declared_item_names.insert(c.ident.to_string());
                }
                syn::Item::Static(s) => {
                    self.declared_item_names.insert(s.ident.to_string());
                }
                syn::Item::Trait(t) => {
                    self.declared_item_names.insert(t.ident.to_string());
                }
                syn::Item::Mod(m) => {
                    self.declared_item_names.insert(m.ident.to_string());
                    self.declared_module_names.insert(m.ident.to_string());
                    self.declared_module_paths.insert(m.ident.to_string());
                    if let Some((_, nested_items)) = &m.content {
                        self.collect_nested_module_names(nested_items, &[m.ident.to_string()]);
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_nested_module_names(&mut self, items: &[syn::Item], parent_path: &[String]) {
        for item in items {
            if let syn::Item::Mod(m) = item {
                let mod_name = m.ident.to_string();
                self.declared_module_names.insert(mod_name.clone());
                let mut scoped = parent_path.to_vec();
                scoped.push(mod_name.clone());
                self.declared_module_paths.insert(scoped.join("::"));
                if let Some((_, nested_items)) = &m.content {
                    self.collect_nested_module_names(nested_items, &scoped);
                }
            }
        }
    }

    pub(super) fn collect_item_const_types(&mut self, items: &[syn::Item], module_path: &[String]) {
        for item in items {
            match item {
                syn::Item::Const(c) => {
                    // A `#[rustc_test_marker]` const is test-harness machinery
                    // named after its test FN (`#[test] fn entry()` expands to
                    // `pub const entry: test::TestDescAndFn`). Recording it
                    // would make same-named PATTERN BINDINGS elsewhere in the
                    // crate lower as const-value equality tests (indexmap's
                    // `Entry::Occupied(entry)` arms vs its `entry` test).
                    if Self::has_rustc_test_marker_attr(&c.attrs) {
                        continue;
                    }
                    let name = c.ident.to_string();
                    let ty = (*c.ty).clone();
                    self.item_const_types
                        .entry(name.clone())
                        .or_insert_with(|| ty.clone());
                    if !module_path.is_empty() {
                        let scoped = format!("{}::{}", module_path.join("::"), name);
                        self.item_const_types.insert(scoped, ty);
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested_items)) = &m.content {
                        let mut nested_path = module_path.to_vec();
                        nested_path.push(m.ident.to_string());
                        self.collect_item_const_types(nested_items, &nested_path);
                    }
                }
                syn::Item::Static(s) => {
                    let name = s.ident.to_string();
                    let ty = (*s.ty).clone();
                    self.item_const_types
                        .entry(name.clone())
                        .or_insert_with(|| ty.clone());
                    if !module_path.is_empty() {
                        let scoped = format!("{}::{}", module_path.join("::"), name);
                        self.item_const_types.insert(scoped, ty);
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_required_top_level_module_aliases(
        &mut self,
        items: &[syn::Item],
        module_path: &[String],
    ) {
        let prev_stack = self.module_stack.clone();
        self.module_stack = module_path.to_vec();

        for item in items {
            match item {
                syn::Item::Use(u) => {
                    let paths = self.flatten_use_tree(&u.tree, "");
                    for path in paths {
                        let resolved = self.resolve_unqualified_local_import_path(&path);
                        if let Some(namespace_target) =
                            self.resolve_bare_module_namespace_import(&resolved)
                        {
                            self.required_top_level_module_aliases
                                .insert(namespace_target);
                        }
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested_items)) = &m.content {
                        let mut nested_path = module_path.to_vec();
                        nested_path.push(m.ident.to_string());
                        self.collect_required_top_level_module_aliases(nested_items, &nested_path);
                    }
                }
                _ => {}
            }
        }

        self.module_stack = prev_stack;
    }

    pub(super) fn collect_skipped_module_trait_names(&mut self, items: &[syn::Item], module_path: &[String]) {
        for item in items {
            match item {
                syn::Item::Trait(t) => {
                    let scoped = if module_path.is_empty() {
                        t.ident.to_string()
                    } else {
                        format!("{}::{}", module_path.join("::"), t.ident)
                    };
                    self.skipped_module_traits.insert(scoped);
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested_items)) = &m.content {
                        let mut nested_path = module_path.to_vec();
                        nested_path.push(m.ident.to_string());
                        self.collect_skipped_module_trait_names(nested_items, &nested_path);
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_local_declared_types(&mut self, items: &[syn::Item], module_path: &[String]) {
        for item in items {
            match item {
                syn::Item::Struct(s) => {
                    self.record_local_declared_type(module_path, &s.ident.to_string());
                    self.record_local_declared_type_params(
                        module_path,
                        &s.ident.to_string(),
                        &s.generics,
                        true,
                    );
                    // A slice-tail wrapper (`struct Slice<K, V> { entries:
                    // [Bucket<K, V>] }`, repr(transparent) DSTs): its C++
                    // form is a single-span VIEW VALUE — `&Slice`/`&mut
                    // Slice` lower to values and the from-slice pun
                    // constructs instead of reinterpreting.
                    let mut fields = s.fields.iter();
                    if let (Some(only), None) = (fields.next(), fields.next())
                        && matches!(&only.ty, syn::Type::Slice(_))
                    {
                        let field_name = only
                            .ident
                            .as_ref()
                            .map(|i| i.to_string())
                            .unwrap_or_else(|| "_0".to_string());
                        self.slice_tail_view_types
                            .insert(s.ident.to_string(), field_name);
                    }
                }
                syn::Item::Union(u) => {
                    self.record_local_declared_type(module_path, &u.ident.to_string());
                    self.record_local_declared_type_params(
                        module_path,
                        &u.ident.to_string(),
                        &u.generics,
                        true,
                    );
                }
                syn::Item::Enum(e) => {
                    self.record_local_declared_type(module_path, &e.ident.to_string());
                    self.record_local_declared_type_params(
                        module_path,
                        &e.ident.to_string(),
                        &e.generics,
                        true,
                    );
                    self.record_data_enum_variant_metadata(module_path, e);
                    self.record_c_like_enum_variant_consts(module_path, e);
                }
                syn::Item::Type(t) => {
                    self.record_local_declared_type(module_path, &t.ident.to_string());
                    self.record_local_declared_type_params(
                        module_path,
                        &t.ident.to_string(),
                        &t.generics,
                        false,
                    );
                    self.record_type_alias_target(module_path, t);
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested_items)) = &m.content {
                        // Record a NESTED module (leaf → escaped parent path) in the manifest
                        // surface so a consumer requalifies a crate-stripped submodule path it
                        // references (`private_::string`, `de::rusty_ext`) to
                        // `serde_core::private_::string`. Type-less modules (`string`,
                        // `size_hint`) have no declared_types to carry them otherwise.
                        if !module_path.is_empty() {
                            self.record_manifest_module(module_path, &m.ident.to_string());
                        }
                        // HYGIENE-ALIAS (book § 32): a glob-only re-export shell
                        // (`pub mod __private228 { pub use crate::private::* }`) is a
                        // macro-hygiene artifact whose body just re-globs the canonical module.
                        // Record shell→canonical so a consumer resolves `<crate>::__private228`
                        // through the linkage (the .rmeta analog) rather than by matching the
                        // crate-local hygiene number.
                        if nested_items.len() == 1
                            && let syn::Item::Use(u) = &nested_items[0]
                            && matches!(u.vis, syn::Visibility::Public(_))
                            && let Some(target) = self.glob_use_target_namespace(&u.tree)
                        {
                            let mut full: Vec<String> = module_path
                                .iter()
                                .map(|s| escape_cpp_keyword(s))
                                .collect();
                            full.push(escape_cpp_keyword(&m.ident.to_string()));
                            let shell = full.join("::");
                            if shell != target {
                                self.hygiene_module_aliases.insert(shell, target);
                            }
                        }
                        let mut nested_path = module_path.to_vec();
                        nested_path.push(m.ident.to_string());
                        self.collect_local_declared_types(nested_items, &nested_path);
                    }
                }
                syn::Item::Use(u) if matches!(u.vis, syn::Visibility::Public(_)) => {
                    // A `pub use ignored_any::IgnoredAny;` makes a type accessible at THIS
                    // (public, shorter) module. Record it in the cross-crate manifest surface
                    // so a consumer requalifies the re-export path it actually references
                    // (`de::IgnoredAny`) rather than the deep declaration path
                    // (`de::ignored_any::IgnoredAny`). Manifest-only — own qualification untouched.
                    self.record_reexported_manifest_types(module_path, &u.tree);
                }
                _ => {}
            }
        }
    }

    fn record_reexported_manifest_types(&mut self, module_path: &[String], tree: &syn::UseTree) {
        // via_local_path=false: a bare top-level `pub use Ok;` re-exports a glob-imported
        // EXTERNAL name (rusty::Ok), which must NOT be mapped into this crate's surface.
        // Only a name reached THROUGH a local module path (`pub use ignored_any::IgnoredAny`)
        // names a type this crate actually owns at this module.
        self.record_reexported_inner(module_path, tree, false);
    }

    fn record_reexported_inner(
        &mut self,
        module_path: &[String],
        tree: &syn::UseTree,
        via_local_path: bool,
    ) {
        match tree {
            syn::UseTree::Path(p) => {
                // A re-export rooted at a runtime/external crate (`pub use rusty::Ok`) or a
                // namespace-wrapped dependency does NOT make a LOCAL type accessible here.
                let root = p.ident.to_string();
                if matches!(
                    root.as_str(),
                    "rusty" | "std" | "core" | "alloc" | "proc_macro"
                ) || crate::transpile::crate_is_namespace_wrapped(&root)
                {
                    return;
                }
                self.record_reexported_inner(module_path, &p.tree, true)
            }
            syn::UseTree::Group(g) => {
                for item in &g.items {
                    self.record_reexported_inner(module_path, item, via_local_path);
                }
            }
            syn::UseTree::Name(n) if via_local_path => {
                self.record_manifest_reexport(module_path, &n.ident.to_string())
            }
            syn::UseTree::Rename(r) if via_local_path => {
                self.record_manifest_reexport(module_path, &r.rename.to_string())
            }
            _ => {}
        }
    }

    fn record_manifest_reexport(&mut self, module_path: &[String], name: &str) {
        // Only TYPE-like names (Capitalized); module/function re-exports aren't requalifiable
        // types. Keep the SHORTEST (most-public) path so a re-export wins over a deep decl.
        if module_path.is_empty()
            || !name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        {
            return;
        }
        // A crate often re-exports a RUNTIME/std type through a local-looking path
        // (`pub use crate::lib::result::Result;`). These map to `rusty::…`, are NOT the
        // crate's own types, and must never enter its manifest surface — recording them
        // mis-qualifies them to the re-export module (e.g. `private_::Ok`) on the
        // self-manifest back-edge.
        if matches!(
            name,
            "Result" | "Option" | "Ok" | "Err" | "Some" | "None" | "Box" | "Vec"
                | "String" | "Cow" | "Rc" | "Arc" | "Cell" | "RefCell"
        ) {
            return;
        }
        let escaped_path = module_path
            .iter()
            .map(|segment| escape_cpp_keyword(segment))
            .collect::<Vec<_>>()
            .join("::");
        let shorter = match self.manifest_type_module_path.get(name) {
            Some(existing) => escaped_path.split("::").count() < existing.split("::").count(),
            None => true,
        };
        if shorter {
            self.manifest_type_module_path
                .insert(name.to_string(), escaped_path);
        }
    }

    /// Record a NESTED module (escaped leaf → escaped parent path) in the manifest surface,
    /// so a consumer requalifies a crate-stripped submodule path (`private_::string`,
    /// `de::rusty_ext`) to `<crate>::private_::string`. Unlike record_manifest_reexport this
    /// allows lowercase (module) names. Keep the SHORTEST (most-public) parent path.
    fn record_manifest_module(&mut self, module_path: &[String], mod_name: &str) {
        let escaped_path = module_path
            .iter()
            .map(|segment| escape_cpp_keyword(segment))
            .collect::<Vec<_>>()
            .join("::");
        let name = escape_cpp_keyword(mod_name);
        let shorter = match self.manifest_type_module_path.get(&name) {
            Some(existing) => escaped_path.split("::").count() < existing.split("::").count(),
            None => true,
        };
        if shorter {
            self.manifest_type_module_path.insert(name, escaped_path);
        }
    }

    /// Cluster C: walk the file's items (recursing into nested modules) and
    /// detect groups of parallel inherent impl blocks of the same host type
    /// whose self-type args differ at positions consumed by the host's
    /// class-level generic params. For each such position, record a
    /// substitution `(concrete_marker_name → host_class_param_name)` keyed
    /// by `(host_type_name, method_ident)`.
    ///
    /// Example:
    /// ```ignore
    /// impl<K, V> LazyRange<marker::Immut, K, V> {
    ///     pub fn full_range(self) -> LazyRange<marker::Immut, K, V> { ... }
    /// }
    /// impl<K, V> LazyRange<marker::ValMut, K, V> {
    ///     pub fn full_range(self) -> LazyRange<marker::ValMut, K, V> { ... }
    /// }
    /// ```
    /// Host: `LazyRange<BorrowType, K, V>`. Position 0 varies (Immut vs
    /// ValMut). Host param at position 0 is `BorrowType`. Substitution:
    /// `(Immut → BorrowType)` and `(ValMut → BorrowType)`, applied to
    /// whichever impl's method got absorbed into the merged C++ struct.
    pub(super) fn collect_parallel_impl_groups(&mut self, items: &[syn::Item]) {
        // (host_type_name, method_ident) → Vec<class-template-args-of-this-impl>
        // host_type_name is MODULE-QUALIFIED (see walk_items_for_parallel_impls)
        // so sibling families across modules never share a group.
        let mut groups: HashMap<(String, String), Vec<Vec<String>>> = HashMap::new();
        self.walk_items_for_parallel_impls(items, &[], &mut groups);

        let mut nested_marker_subs: HashMap<(String, String), Vec<(String, String)>> =
            HashMap::new();
        for ((host_type_name, method_ident), arg_vec_list) in groups {
            if arg_vec_list.len() < 2 {
                continue;
            }
            // Arity guard: a parallel-impl group must have a uniform
            // class-template-arg width — impls of genuinely the same host type
            // share an arity. Differing widths mean distinct types were
            // conflated (which qualified keying should already prevent); skip
            // rather than emit cross-arity substitutions.
            let group_width = arg_vec_list[0].len();
            if arg_vec_list.iter().any(|a| a.len() != group_width) {
                continue;
            }
            // Look up host class params; need positions to know what each
            // position should generalize to.
            let host_params = match self.declared_type_params.get(&host_type_name) {
                Some(p) => p.clone(),
                None => continue,
            };
            // Find positions where the arg differs across impls.
            let first = &arg_vec_list[0];
            let width = first.len();
            let mut subs: Vec<(String, String)> = Vec::new();
            for pos in 0..width {
                let mut all_args: Vec<&String> = Vec::with_capacity(arg_vec_list.len());
                let mut ok = true;
                for args in &arg_vec_list {
                    if let Some(a) = args.get(pos) {
                        all_args.push(a);
                    } else {
                        ok = false;
                        break;
                    }
                }
                if !ok {
                    continue;
                }
                // Are all args at this position the same? If yes, no
                // substitution needed.
                let same = all_args.iter().all(|a| *a == all_args[0]);
                if same {
                    continue;
                }
                // Differing position. The host param at this position is
                // what every concrete marker should generalize to.
                let Some(host_param) = host_params.get(pos) else {
                    continue;
                };
                for concrete in &all_args {
                    if *concrete == host_param {
                        // Already generic in this impl; skip.
                        continue;
                    }
                    subs.push(((*concrete).clone(), host_param.clone()));
                }
                // Nested-marker detection: when the differing args at
                // this position have a common outer wrapper (e.g. both
                // are `NodeRef<…, Leaf>` vs `NodeRef<…, Internal>`),
                // walk into the wrapper and record substitutions for
                // each inner position that varies. The RHS uses
                // `typename __TemplateArgs<host_param>::arg_<inner_pos>`
                // — a dependent path the absorbed method body can
                // resolve once instantiated.
                let parsed_args: Vec<Option<syn::Type>> = all_args
                    .iter()
                    .map(|s| syn::parse_str::<syn::Type>(s).ok())
                    .collect();
                if parsed_args.iter().all(Option::is_some) {
                    let parsed: Vec<&syn::Type> =
                        parsed_args.iter().map(|t| t.as_ref().unwrap()).collect();
                    let mut nested_subs: Vec<(String, String)> = Vec::new();
                    Self::collect_nested_marker_subs(
                        &parsed,
                        host_param,
                        &mut nested_subs,
                    );
                    if !nested_subs.is_empty() {
                        nested_marker_subs
                            .entry((host_type_name.clone(), method_ident.clone()))
                            .or_insert_with(Vec::new)
                            .extend(nested_subs);
                    }
                }
            }
            if subs.is_empty() {
                continue;
            }
            self.parallel_impl_substitutions
                .insert((host_type_name, method_ident), subs);
        }
        // Move the locally-collected nested-marker subs into the field.
        for (key, val) in nested_marker_subs {
            self.parallel_impl_nested_marker_text_subs.insert(key, val);
        }
    }

    /// Cluster C nested-marker helper: given N parsed types (one per
    /// parallel impl) for the SAME host-template-arg position, walk
    /// them simultaneously and record `(concrete_marker, dependent_path)`
    /// substitutions for each leaf identifier that varies across the
    /// inputs.
    ///
    /// Caller: `host_param` is the host's class-template parameter name
    /// at this position (e.g. `Node` when the differing args are
    /// `Handle`'s position-0 wrapper). The dependent path emitted is
    /// `typename __TemplateArgs<host_param>::arg_<inner_pos>` where
    /// `inner_pos` is the position inside the WRAPPER where the leaf
    /// ident varies.
    pub(super) fn collect_nested_marker_subs(
        types: &[&syn::Type],
        host_param: &str,
        out: &mut Vec<(String, String)>,
    ) {
        if types.len() < 2 {
            return;
        }
        let first = types[0];
        // All types must share the same outer Path leaf ident, otherwise
        // they're structurally different and we can't decompose.
        let syn::Type::Path(first_tp) = first else {
            return;
        };
        let Some(first_last) = first_tp.path.segments.last() else {
            return;
        };
        let first_leaf = first_last.ident.to_string();
        for t in types.iter().skip(1) {
            let syn::Type::Path(tp) = t else { return };
            let Some(last) = tp.path.segments.last() else {
                return;
            };
            if last.ident != first_leaf {
                return;
            }
        }
        // Same wrapper. Compare each inner angle-bracket arg position.
        let syn::PathArguments::AngleBracketed(first_args) = &first_last.arguments else {
            return;
        };
        let inner_width = first_args.args.len();
        for inner_pos in 0..inner_width {
            let inner_args: Vec<Option<&syn::GenericArgument>> = types
                .iter()
                .map(|t| {
                    let syn::Type::Path(tp) = t else { return None };
                    let last = tp.path.segments.last()?;
                    let syn::PathArguments::AngleBracketed(ab) = &last.arguments else {
                        return None;
                    };
                    ab.args.get(inner_pos)
                })
                .collect();
            if inner_args.iter().any(Option::is_none) {
                continue;
            }
            let inner_types: Vec<Option<&syn::Type>> = inner_args
                .iter()
                .map(|a| match a.unwrap() {
                    syn::GenericArgument::Type(t) => Some(t),
                    _ => None,
                })
                .collect();
            if inner_types.iter().any(Option::is_none) {
                continue;
            }
            let inner_strs: Vec<String> = inner_types
                .iter()
                .map(|t| Self::type_to_normalized_string(t.unwrap()))
                .collect();
            let same = inner_strs.iter().all(|s| s == &inner_strs[0]);
            if same {
                continue;
            }
            // Each `inner_strs[i]` is the concrete value at this nested
            // position for impl i. Record a substitution from each
            // distinct concrete value to the dependent path.
            let dep_path = format!(
                "typename __TemplateArgs<{}>::arg_{}",
                host_param, inner_pos
            );
            let mut seen: HashSet<String> = HashSet::new();
            for s in &inner_strs {
                if !seen.insert(s.clone()) {
                    continue;
                }
                // Push three substitution shapes: the normalized full
                // path (`::marker::Leaf`), the same without leading
                // `::` (`marker::Leaf`), and the bare leaf ident
                // (`Leaf`). The post-emit text substitution tries
                // each in order via `replace_whole_word`. Order
                // matters: the longer paths must be tried first so
                // they consume the prefix; the bare leaf is a
                // last-resort match for sites that emit without the
                // namespace qualifier.
                let leaf = s.rsplit("::").next().unwrap_or(s.as_str());
                if leaf.is_empty() {
                    continue;
                }
                let trimmed = s.trim_start_matches("::");
                if !trimmed.is_empty() && trimmed.contains("::") {
                    out.push((format!("::{}", trimmed), dep_path.clone()));
                    out.push((trimmed.to_string(), dep_path.clone()));
                }
                out.push((leaf.to_string(), dep_path.clone()));
            }
        }
    }

    pub(super) fn collect_data_enum_wrapper_types(&mut self, items: &[syn::Item], module_path: &[String]) {
        for item in items {
            match item {
                syn::Item::Enum(e) => {
                    if !e.variants.iter().any(|variant| !variant.fields.is_empty()) {
                        continue;
                    }
                    let enum_name = e.ident.to_string();
                    let scoped_enum_name = if module_path.is_empty() {
                        enum_name.clone()
                    } else {
                        format!("{}::{}", module_path.join("::"), enum_name)
                    };
                    let has_impls = self.impl_blocks.contains_key(&scoped_enum_name)
                        || self.impl_blocks.contains_key(&enum_name);
                    let is_recursive = e
                        .variants
                        .iter()
                        .any(|variant| self.variant_references_type(variant, &enum_name));
                    if has_impls || is_recursive {
                        self.data_enum_wrapper_types.insert(enum_name);
                        self.data_enum_wrapper_types.insert(scoped_enum_name);
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested_items)) = &m.content {
                        let mut nested_path = module_path.to_vec();
                        nested_path.push(m.ident.to_string());
                        self.collect_data_enum_wrapper_types(nested_items, &nested_path);
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_struct_metadata(&mut self, items: &[syn::Item], module_path: &[String]) {
        for item in items {
            match item {
                syn::Item::Struct(s) => {
                    let struct_name = s.ident.to_string();
                    if matches!(s.fields, syn::Fields::Unit) {
                        self.unit_struct_types.insert(struct_name.clone());
                        if !module_path.is_empty() {
                            self.unit_struct_types.insert(format!(
                                "{}::{}",
                                module_path.join("::"),
                                struct_name
                            ));
                        }
                    }
                    if let syn::Fields::Unnamed(fields) = &s.fields {
                        let arity = fields.unnamed.len();
                        self.tuple_struct_arities.insert(struct_name.clone(), arity);
                        if !module_path.is_empty() {
                            self.tuple_struct_arities.insert(
                                format!("{}::{}", module_path.join("::"), struct_name),
                                arity,
                            );
                        }

                        let unnamed_field_types: HashMap<String, syn::Type> = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(idx, field)| (format!("_{}", idx), field.ty.clone()))
                            .collect();
                        let unnamed_field_order: Vec<String> = (0..fields.unnamed.len())
                            .map(|idx| format!("_{}", idx))
                            .collect();
                        let unnamed_field_cpp_names: HashMap<String, String> =
                            (0..fields.unnamed.len())
                                .map(|idx| {
                                    let rust_name = format!("_{}", idx);
                                    (rust_name.clone(), escape_cpp_keyword(&rust_name))
                                })
                                .collect();
                        let reference_fields: HashSet<String> = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .filter_map(|(idx, field)| {
                                matches!(field.ty, syn::Type::Reference(_))
                                    .then_some(format!("_{}", idx))
                            })
                            .collect();
                        if !unnamed_field_types.is_empty() {
                            std::rc::Rc::make_mut(&mut self.struct_field_types)
                                .insert(struct_name.clone(), unnamed_field_types.clone());
                            std::rc::Rc::make_mut(&mut self.struct_field_order)
                                .insert(struct_name.clone(), unnamed_field_order.clone());
                            std::rc::Rc::make_mut(&mut self.struct_field_cpp_names)
                                .insert(struct_name.clone(), unnamed_field_cpp_names.clone());
                            if !reference_fields.is_empty() {
                                self.struct_reference_fields
                                    .insert(struct_name.clone(), reference_fields.clone());
                            }
                            if !module_path.is_empty() {
                                std::rc::Rc::make_mut(&mut self.struct_field_types).insert(
                                    format!("{}::{}", module_path.join("::"), struct_name),
                                    unnamed_field_types,
                                );
                                std::rc::Rc::make_mut(&mut self.struct_field_order).insert(
                                    format!("{}::{}", module_path.join("::"), struct_name),
                                    unnamed_field_order,
                                );
                                std::rc::Rc::make_mut(&mut self.struct_field_cpp_names).insert(
                                    format!("{}::{}", module_path.join("::"), struct_name),
                                    unnamed_field_cpp_names,
                                );
                                if !reference_fields.is_empty() {
                                    self.struct_reference_fields.insert(
                                        format!("{}::{}", module_path.join("::"), struct_name),
                                        reference_fields,
                                    );
                                }
                            }
                        }
                    }

                    if let syn::Fields::Named(fields) = &s.fields {
                        let named_field_types: HashMap<String, syn::Type> = fields
                            .named
                            .iter()
                            .filter_map(|field| {
                                field
                                    .ident
                                    .as_ref()
                                    .map(|ident| (ident.to_string(), field.ty.clone()))
                            })
                            .collect();
                        let named_field_order: Vec<String> = fields
                            .named
                            .iter()
                            .filter_map(|field| field.ident.as_ref().map(|ident| ident.to_string()))
                            .collect();
                        let named_field_cpp_names: HashMap<String, String> = fields
                            .named
                            .iter()
                            .filter_map(|field| {
                                field.ident.as_ref().map(|ident| {
                                    let rust_name = ident.to_string();
                                    (rust_name.clone(), escape_cpp_keyword(&rust_name))
                                })
                            })
                            .collect();
                        let reference_fields: HashSet<String> = fields
                            .named
                            .iter()
                            .filter_map(|field| {
                                let ident = field.ident.as_ref()?;
                                if matches!(field.ty, syn::Type::Reference(_)) {
                                    Some(ident.to_string())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        let nonpub_fields: HashSet<String> = fields
                            .named
                            .iter()
                            .filter_map(|field| {
                                let ident = field.ident.as_ref()?;
                                matches!(field.vis, syn::Visibility::Inherited)
                                    .then(|| ident.to_string())
                            })
                            .collect();
                        if !nonpub_fields.is_empty() {
                            self.struct_nonpub_fields
                                .insert(struct_name.clone(), nonpub_fields.clone());
                            if !module_path.is_empty() {
                                self.struct_nonpub_fields.insert(
                                    format!("{}::{}", module_path.join("::"), struct_name),
                                    nonpub_fields,
                                );
                            }
                        }
                        if !named_field_types.is_empty() {
                            std::rc::Rc::make_mut(&mut self.struct_field_types)
                                .insert(struct_name.clone(), named_field_types.clone());
                            std::rc::Rc::make_mut(&mut self.struct_field_order)
                                .insert(struct_name.clone(), named_field_order.clone());
                            std::rc::Rc::make_mut(&mut self.struct_field_cpp_names)
                                .insert(struct_name.clone(), named_field_cpp_names.clone());
                            if !reference_fields.is_empty() {
                                self.struct_reference_fields
                                    .insert(struct_name.clone(), reference_fields.clone());
                            }
                            if !module_path.is_empty() {
                                std::rc::Rc::make_mut(&mut self.struct_field_types).insert(
                                    format!("{}::{}", module_path.join("::"), struct_name),
                                    named_field_types,
                                );
                                std::rc::Rc::make_mut(&mut self.struct_field_order).insert(
                                    format!("{}::{}", module_path.join("::"), struct_name),
                                    named_field_order,
                                );
                                std::rc::Rc::make_mut(&mut self.struct_field_cpp_names).insert(
                                    format!("{}::{}", module_path.join("::"), struct_name),
                                    named_field_cpp_names,
                                );
                                if !reference_fields.is_empty() {
                                    self.struct_reference_fields.insert(
                                        format!("{}::{}", module_path.join("::"), struct_name),
                                        reference_fields,
                                    );
                                }
                            }
                        }
                    }
                }
                syn::Item::Union(u) => {
                    // c2rust unions: record field types so `x.field` (a union
                    // member access) infers, exactly like a struct's named fields.
                    let union_name = u.ident.to_string();
                    let named_field_types: HashMap<String, syn::Type> = u
                        .fields
                        .named
                        .iter()
                        .filter_map(|f| f.ident.as_ref().map(|id| (id.to_string(), f.ty.clone())))
                        .collect();
                    let named_field_order: Vec<String> = u
                        .fields
                        .named
                        .iter()
                        .filter_map(|f| f.ident.as_ref().map(|id| id.to_string()))
                        .collect();
                    let named_field_cpp_names: HashMap<String, String> = u
                        .fields
                        .named
                        .iter()
                        .filter_map(|f| {
                            f.ident
                                .as_ref()
                                .map(|id| (id.to_string(), escape_cpp_keyword(&id.to_string())))
                        })
                        .collect();
                    if !named_field_types.is_empty() {
                        std::rc::Rc::make_mut(&mut self.struct_field_types)
                            .insert(union_name.clone(), named_field_types.clone());
                        std::rc::Rc::make_mut(&mut self.struct_field_order)
                            .insert(union_name.clone(), named_field_order.clone());
                        std::rc::Rc::make_mut(&mut self.struct_field_cpp_names)
                            .insert(union_name.clone(), named_field_cpp_names.clone());
                        if !module_path.is_empty() {
                            let scoped = format!("{}::{}", module_path.join("::"), union_name);
                            std::rc::Rc::make_mut(&mut self.struct_field_types).insert(scoped.clone(), named_field_types);
                            std::rc::Rc::make_mut(&mut self.struct_field_order).insert(scoped.clone(), named_field_order);
                            std::rc::Rc::make_mut(&mut self.struct_field_cpp_names)
                                .insert(scoped, named_field_cpp_names);
                        }
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested_items)) = &m.content {
                        let mut nested_path = module_path.to_vec();
                        nested_path.push(m.ident.to_string());
                        self.collect_struct_metadata(nested_items, &nested_path);
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_call_arg_pass_styles(&mut self, items: &[syn::Item], module_path: &[String]) {
        for item in items {
            match item {
                syn::Item::Fn(f) => {
                    let fn_name = f.sig.ident.to_string();
                    let scoped_name = if module_path.is_empty() {
                        fn_name.clone()
                    } else {
                        format!("{}::{}", module_path.join("::"), fn_name)
                    };
                    let styles = self.collect_arg_pass_styles_from_inputs(&f.sig.inputs, false);
                    self.record_function_arg_pass_styles(&scoped_name, styles);
                    let expected_types =
                        self.collect_arg_expected_types_from_inputs(&f.sig.inputs, false);
                    let expected_types =
                        self.resolve_fn_bound_arg_expected_types(expected_types, &f.sig.generics);
                    self.record_function_arg_expected_types(&scoped_name, expected_types);
                    let type_params = self.collect_type_param_names_from_generics(&f.sig.generics);
                    self.record_function_type_param_names(&scoped_name, type_params);
                    let return_ty = self.collect_return_type_from_output(&f.sig.output);
                    self.record_function_return_type(&scoped_name, return_ty);
                }
                syn::Item::Impl(impl_block) => {
                    // #88: record each method's declaring-impl Self type
                    // (keyed by the type's TAIL name, matching current_struct
                    // at emission). Poison on a conflicting Self for the same
                    // method name — no single answer exists then.
                    if let Some(tp) = Self::impl_self_type_path(impl_block.self_ty.as_ref())
                        && let Some(tail_seg) = tp.path.segments.last()
                    {
                        use quote::ToTokens;
                        let tail = tail_seg.ident.to_string();
                        for item in &impl_block.items {
                            let syn::ImplItem::Fn(m) = item else { continue };
                            if let Some(syn::FnArg::Receiver(recv)) = m.sig.inputs.first() {
                                let kind: u8 = match (recv.reference.is_some(), recv.mutability.is_some()) {
                                    (true, false) => 0,
                                    (true, true) => 1,
                                    (false, false) => 2,
                                    (false, true) => 3,
                                };
                                self.impl_method_receiver_kinds
                                    .entry(tail.clone())
                                    .or_default()
                                    .insert(m.sig.ident.to_string(), kind);
                            }
                            let per_type =
                                self.impl_method_self_tys.entry(tail.clone()).or_default();
                            match per_type.entry(m.sig.ident.to_string()) {
                                std::collections::hash_map::Entry::Occupied(mut o) => {
                                    let same = o.get().as_ref().is_some_and(|prev| {
                                        prev.to_token_stream().to_string()
                                            == impl_block.self_ty.to_token_stream().to_string()
                                    });
                                    if !same {
                                        o.insert(None);
                                    }
                                }
                                std::collections::hash_map::Entry::Vacant(v) => {
                                    v.insert(Some((*impl_block.self_ty).clone()));
                                }
                            }
                        }
                    }
                    let mut owner_keys: Vec<String> = Vec::new();
                    if let Some(tp) = Self::impl_self_type_path(impl_block.self_ty.as_ref())
                        && !tp.path.segments.is_empty()
                    {
                        let joined = tp
                            .path
                            .segments
                            .iter()
                            .map(|seg| seg.ident.to_string())
                            .collect::<Vec<_>>()
                            .join("::");
                        if !joined.is_empty() {
                            owner_keys.push(joined.clone());
                        }
                        if let Some(last) = tp.path.segments.last() {
                            owner_keys.push(last.ident.to_string());
                        }
                        if tp.path.segments.len() == 1 && !module_path.is_empty() {
                            owner_keys.push(format!(
                                "{}::{}",
                                module_path.join("::"),
                                tp.path.segments[0].ident
                            ));
                        }
                        let mut dedup = HashSet::new();
                        owner_keys.retain(|key| dedup.insert(key.clone()));
                    }
                    let mut trait_keys: Vec<String> = Vec::new();
                    if let Some((_, trait_path, _)) = &impl_block.trait_
                        && !trait_path.segments.is_empty()
                    {
                        let joined = trait_path
                            .segments
                            .iter()
                            .map(|seg| seg.ident.to_string())
                            .collect::<Vec<_>>()
                            .join("::");
                        if !joined.is_empty() {
                            trait_keys.push(joined.clone());
                        }
                        if let Some(last) = trait_path.segments.last() {
                            trait_keys.push(last.ident.to_string());
                        }
                        if trait_path.segments.len() == 1 && !module_path.is_empty() {
                            trait_keys.push(format!(
                                "{}::{}",
                                module_path.join("::"),
                                trait_path.segments[0].ident
                            ));
                        }
                        let mut dedup = HashSet::new();
                        trait_keys.retain(|key| dedup.insert(key.clone()));
                    }
                    for impl_item in &impl_block.items {
                        let syn::ImplItem::Fn(method) = impl_item else {
                            continue;
                        };
                        let method_name = method.sig.ident.to_string();
                        let has_receiver =
                            matches!(method.sig.inputs.first(), Some(syn::FnArg::Receiver(_)));
                        let styles =
                            self.collect_arg_pass_styles_from_inputs(&method.sig.inputs, true);
                        self.record_method_arg_pass_styles(&method_name, styles);
                        let expected_types = self.resolve_fn_bound_arg_expected_types(
                            self.collect_arg_expected_types_from_inputs(&method.sig.inputs, true),
                            &method.sig.generics,
                        );
                        self.record_method_arg_expected_types(&method_name, expected_types.clone());
                        let method_type_params =
                            self.collect_type_param_names_from_generics(&method.sig.generics);
                        let method_return_ty =
                            self.collect_return_type_from_output(&method.sig.output);
                        for trait_name in &trait_keys {
                            std::rc::Rc::make_mut(&mut self.trait_method_has_receiver)
                                .insert(format!("{}::{}", trait_name, method_name), has_receiver);
                        }
                        for owner in &owner_keys {
                            let owner_method_key = format!("{}::{}", owner, method_name);
                            self.record_function_arg_pass_styles(
                                &owner_method_key,
                                self.collect_arg_pass_styles_from_inputs(&method.sig.inputs, true),
                            );
                            self.record_function_arg_expected_types(
                                &owner_method_key,
                                expected_types.clone(),
                            );
                            self.record_function_type_param_names(
                                &owner_method_key,
                                method_type_params.clone(),
                            );
                            self.record_function_return_type(
                                &owner_method_key,
                                method_return_ty.clone(),
                            );
                            self.record_owner_method_arg_expected_types(
                                owner,
                                &method_name,
                                expected_types.clone(),
                            );
                            self.record_owner_method_has_receiver(
                                owner,
                                &method_name,
                                has_receiver,
                            );
                        }
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested_items)) = &m.content {
                        let mut nested_path = module_path.to_vec();
                        nested_path.push(m.ident.to_string());
                        self.collect_call_arg_pass_styles(nested_items, &nested_path);
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_arg_pass_styles_from_inputs(
        &self,
        inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
        skip_receiver: bool,
    ) -> Vec<ArgPassStyle> {
        let mut styles = Vec::new();
        for input in inputs {
            match input {
                syn::FnArg::Receiver(_) => {
                    if !skip_receiver {
                        styles.push(ArgPassStyle::Value);
                    }
                }
                syn::FnArg::Typed(pt) => {
                    styles.push(self.arg_pass_style_for_type(&pt.ty));
                }
            }
        }
        styles
    }

    pub(super) fn collect_arg_expected_types_from_inputs(
        &self,
        inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
        skip_receiver: bool,
    ) -> Vec<Option<syn::Type>> {
        let mut expected = Vec::new();
        for input in inputs {
            match input {
                syn::FnArg::Receiver(_) => {
                    if !skip_receiver {
                        expected.push(None);
                    }
                }
                syn::FnArg::Typed(pt) => expected.push(Some((*pt.ty).clone())),
            }
        }
        expected
    }

    /// A bare generic-param type (`f: F`) records as its Fn-trait bound
    /// spelled impl-trait (`impl FnOnce(&mut [Bucket<K, V>])`) so call-site
    /// machinery can type CLOSURE arguments and their params — indexmap's
    /// `with_entries<F: FnOnce(&mut [Bucket<K,V>])>` closures otherwise
    /// carry no param types and their bodies emit member calls on unknowns.
    /// Only PARENTHESIZED Fn/FnMut/FnOnce bounds substitute; ordinary trait
    /// bounds leave the param untouched.
    pub(super) fn resolve_fn_bound_arg_expected_types(
        &self,
        expected: Vec<Option<syn::Type>>,
        generics: &syn::Generics,
    ) -> Vec<Option<syn::Type>> {
        fn fn_trait_bound(bound: &syn::TypeParamBound) -> Option<&syn::TypeParamBound> {
            let syn::TypeParamBound::Trait(trait_bound) = bound else {
                return None;
            };
            let seg = trait_bound.path.segments.last()?;
            if !matches!(seg.ident.to_string().as_str(), "Fn" | "FnMut" | "FnOnce") {
                return None;
            }
            matches!(seg.arguments, syn::PathArguments::Parenthesized(_)).then_some(bound)
        }
        // An Iterator-family bound with a CONCRETE `Item = X` binding
        // (`I2: Iterator<Item = i32>`) also substitutes: the arg expected
        // becomes `impl Iterator<Item = i32>` so return-only generic args
        // (`check(.., empty())`) can take their type from the bound.
        let fn_type_param_names: HashSet<String> = generics
            .params
            .iter()
            .filter_map(|param| match param {
                syn::GenericParam::Type(tp) => Some(tp.ident.to_string()),
                _ => None,
            })
            .collect();
        let iterator_bound = |bound: &syn::TypeParamBound| -> Option<syn::TypeParamBound> {
            let syn::TypeParamBound::Trait(trait_bound) = bound else {
                return None;
            };
            let seg = trait_bound.path.segments.last()?;
            if !matches!(
                seg.ident.to_string().as_str(),
                "Iterator" | "IntoIterator" | "DoubleEndedIterator" | "ExactSizeIterator"
            ) {
                return None;
            }
            let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
                return None;
            };
            let item = args.args.iter().find_map(|arg| match arg {
                syn::GenericArgument::AssocType(assoc) if assoc.ident == "Item" => {
                    Some(&assoc.ty)
                }
                _ => None,
            })?;
            // Concrete plain-path items only (peel references): a generic
            // Item (T) or a composite one has no context-free spelling.
            let mut peeled = item;
            while let syn::Type::Reference(r) = peeled {
                peeled = &r.elem;
            }
            let syn::Type::Path(item_tp) = peeled else {
                return None;
            };
            let last = item_tp.path.segments.last()?;
            if !matches!(last.arguments, syn::PathArguments::None) {
                return None;
            }
            if item_tp.path.segments.len() == 1
                && fn_type_param_names.contains(&last.ident.to_string())
            {
                return None;
            }
            Some(bound.clone())
        };
        // A `Q: Equivalent<K>` / `Comparable<K>` lookup-key bound: the arg
        // slot rewrites to K so call-site keys spell as the map's key type
        // (string literals on IndexMap<&str, _>::get need string_view; the
        // receiver-substitution layer concretizes an impl-param K).
        let equivalent_bound = |bound: &syn::TypeParamBound| -> Option<syn::Type> {
            let syn::TypeParamBound::Trait(trait_bound) = bound else {
                return None;
            };
            let seg = trait_bound.path.segments.last()?;
            if !matches!(seg.ident.to_string().as_str(), "Equivalent" | "Comparable") {
                return None;
            }
            let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
                return None;
            };
            args.args.iter().find_map(|arg| match arg {
                syn::GenericArgument::Type(t) => Some(t.clone()),
                _ => None,
            })
        };
        let mut fn_bounds: HashMap<String, syn::TypeParamBound> = HashMap::new();
        let mut iter_bounds: HashMap<String, syn::TypeParamBound> = HashMap::new();
        let mut equiv_bounds: HashMap<String, syn::Type> = HashMap::new();
        for param in &generics.params {
            if let syn::GenericParam::Type(tp) = param {
                for bound in &tp.bounds {
                    if let Some(bound) = fn_trait_bound(bound) {
                        fn_bounds.insert(tp.ident.to_string(), bound.clone());
                    } else if let Some(bound) = iterator_bound(bound) {
                        iter_bounds.insert(tp.ident.to_string(), bound);
                    } else if let Some(k) = equivalent_bound(bound) {
                        equiv_bounds.insert(tp.ident.to_string(), k);
                    }
                }
            }
        }
        if let Some(where_clause) = &generics.where_clause {
            for predicate in &where_clause.predicates {
                if let syn::WherePredicate::Type(pt) = predicate
                    && let syn::Type::Path(tp) = &pt.bounded_ty
                    && tp.qself.is_none()
                    && tp.path.segments.len() == 1
                {
                    let name = tp.path.segments[0].ident.to_string();
                    for bound in &pt.bounds {
                        if let Some(bound) = fn_trait_bound(bound) {
                            fn_bounds.insert(name.clone(), bound.clone());
                        } else if let Some(bound) = iterator_bound(bound) {
                            iter_bounds.insert(name.clone(), bound);
                        } else if let Some(k) = equivalent_bound(bound) {
                            equiv_bounds.insert(name.clone(), k);
                        }
                    }
                }
            }
        }
        if fn_bounds.is_empty() && iter_bounds.is_empty() && equiv_bounds.is_empty() {
            return expected;
        }
        expected
            .into_iter()
            .map(|slot| {
                let Some(ty) = slot else { return None };
                // Lookup-key params appear as `&Q` — peel one reference and
                // re-wrap the substituted K.
                let (peeled, ref_shell) = match &ty {
                    syn::Type::Reference(r) => (r.elem.as_ref(), Some(r.clone())),
                    other => (other, None),
                };
                if let syn::Type::Path(tp) = peeled
                    && tp.qself.is_none()
                    && tp.path.segments.len() == 1
                    && matches!(tp.path.segments[0].arguments, syn::PathArguments::None)
                    && let Some(k) = equiv_bounds.get(&tp.path.segments[0].ident.to_string())
                {
                    return Some(match ref_shell {
                        Some(mut shell) => {
                            shell.elem = Box::new(k.clone());
                            syn::Type::Reference(shell)
                        }
                        None => k.clone(),
                    });
                }
                if let syn::Type::Path(tp) = &ty
                    && tp.qself.is_none()
                    && tp.path.segments.len() == 1
                    && matches!(tp.path.segments[0].arguments, syn::PathArguments::None)
                {
                    let name = tp.path.segments[0].ident.to_string();
                    if let Some(bound) = fn_bounds.get(&name)
                        && Self::fn_bound_inputs_mention_slice(bound)
                    {
                        return Some(syn::Type::ImplTrait(syn::TypeImplTrait {
                            impl_token: Default::default(),
                            bounds: std::iter::once(bound.clone()).collect(),
                        }));
                    }
                    if let Some(bound) = iter_bounds.get(&name) {
                        return Some(syn::Type::ImplTrait(syn::TypeImplTrait {
                            impl_token: Default::default(),
                            bounds: std::iter::once(bound.clone()).collect(),
                        }));
                    }
                }
                Some(ty)
            })
            .collect()
    }

    pub(super) fn collect_type_param_names_from_generics(&self, generics: &syn::Generics) -> Vec<String> {
        generics
            .params
            .iter()
            .filter_map(|param| match param {
                syn::GenericParam::Type(tp) => Some(tp.ident.to_string()),
                _ => None,
            })
            .collect()
    }

    pub(super) fn collect_return_type_from_output(&self, output: &syn::ReturnType) -> Option<syn::Type> {
        match output {
            syn::ReturnType::Type(_, ty) => Some((**ty).clone()),
            syn::ReturnType::Default => None,
        }
    }

    pub(super) fn collect_known_free_function_paths(&self) -> HashSet<String> {
        let mut out = HashSet::new();
        for key in self.function_arg_pass_styles.keys() {
            out.insert(key.clone());
        }
        for key in self.function_arg_expected_types.keys() {
            out.insert(key.clone());
        }
        for key in self.function_type_param_names.keys() {
            out.insert(key.clone());
        }
        for key in self.function_return_types.keys() {
            out.insert(key.clone());
        }
        for (trait_key, methods) in &self.extension_trait_impl_methods {
            let mut parts: Vec<&str> = trait_key.split("::").collect();
            if parts.pop().is_none() {
                continue;
            }
            let module_scope = parts.join("::");
            for method in methods {
                let Some(syn::FnArg::Receiver(_)) = method.method.sig.inputs.first() else {
                    // No receiver means no emitted free-function entry point.
                    continue;
                };
                let method_name = method.method.sig.ident.to_string();
                let full_path = if module_scope.is_empty() {
                    format!("rusty_ext::{}", method_name)
                } else {
                    format!("{}::rusty_ext::{}", module_scope, method_name)
                };
                out.insert(full_path);
            }
        }
        out
    }

    pub(super) fn collect_known_rusty_ext_free_function_paths(&self) -> HashSet<String> {
        let mut out = HashSet::new();
        for (trait_key, methods) in &self.extension_trait_impl_methods {
            let mut parts: Vec<&str> = trait_key.split("::").collect();
            if parts.pop().is_none() {
                continue;
            }
            let module_scope = parts.join("::");
            for method in methods {
                let Some(syn::FnArg::Receiver(_)) = method.method.sig.inputs.first() else {
                    continue;
                };
                let method_name = method.method.sig.ident.to_string();
                let full_path = if module_scope.is_empty() {
                    format!("rusty_ext::{}", method_name)
                } else {
                    format!("{}::rusty_ext::{}", module_scope, method_name)
                };
                out.insert(full_path);
            }
        }
        // Also include already-known free function metadata. Some extension calls
        // come from cross-source hints where impl method bodies are unavailable,
        // but the emitted free-function symbol is still known by path.
        for key in self.collect_known_free_function_paths() {
            if key == "rusty_ext" || key.starts_with("rusty_ext::") || key.contains("::rusty_ext::")
            {
                out.insert(key);
            }
        }
        // Cross-crate dependency `rusty_ext` free functions. A blanket/orphan trait
        // impl in a dependency (e.g. `equivalent`'s `Equivalent::equivalent`, the
        // blanket `impl<Q> Equivalent<K> for Q`) has no concrete host type in the
        // dep, so it is emitted as a crate-wrapped free function
        // `::<dep>::rusty_ext::<method>`, not a member. A consumer that CALLS it
        // must qualify to that absolute path — an unqualified `rusty_ext::<m>` is
        // shadowed by any active `using namespace <mod>;` that pulls in a nested
        // `<mod>::rusty_ext` (indexmap's `get_index_of` under `using namespace
        // map::slice;`). Register the dep path (mirroring the bridge at
        // `emit_cross_crate_rusty_ext_bridge`) so the resolver qualifies it.
        //
        // Coherence guard: never register a method name this crate already knows a
        // `rusty_ext` path for (local impl or free-fn metadata). That keeps a
        // locally-defined method resolving to its own path and only adds paths for
        // methods that are exclusively cross-crate (like `equivalent`). Two deps
        // exposing the same name stay ambiguous → resolver returns None → unchanged.
        const CROSS_CRATE_PRELUDE: [&str; 6] = [
            "deserialize",
            "deserialize_any",
            "deserialize_in_place",
            "serialize",
            "serialize_value",
            "forward_serializer",
        ];
        // The std iterator-entry surface + name-only-hint collisions from
        // `skip_unqualified_cross_source_fallback` (emit_expr.rs): these are
        // "never user extension shims" — registering a dep's same-named
        // rusty_ext method makes the RESOLVER qualify them (serde_core built
        // in the serde_yaml context resolved `into_iter` to
        // `::indexmap::map::slice::rusty_ext::into_iter` — 12 errors), which
        // bypasses the dedicated iterator lowering entirely.
        const CROSS_CRATE_NAME_ONLY_COLLISIONS: [&str; 8] = [
            "get",
            "newline",
            "whitespace",
            "write",
            "write_",
            "into_iter",
            "iter",
            "iter_mut",
        ];
        let locally_known_method_names: HashSet<String> = out
            .iter()
            .filter_map(|path| path.rsplit("::").next().map(str::to_string))
            .collect();
        for dep in &self.dependency_ufcs_trait_manifests {
            if !crate::transpile::crate_is_namespace_wrapped(&dep.module) {
                continue;
            }
            for (module, methods) in &dep.rusty_ext_methods_by_module {
                for meth in methods {
                    if CROSS_CRATE_PRELUDE.contains(&meth.as_str())
                        || CROSS_CRATE_NAME_ONLY_COLLISIONS.contains(&meth.as_str())
                        || locally_known_method_names.contains(meth)
                    {
                        continue;
                    }
                    let dep_path = if module.is_empty() {
                        format!("{}::rusty_ext::{}", dep.module, meth)
                    } else {
                        format!("{}::{}::rusty_ext::{}", dep.module, module, meth)
                    };
                    out.insert(dep_path);
                }
            }
        }
        out
    }

    pub(super) fn collect_extension_trait_impl_methods(
        &mut self,
        items: &[syn::Item],
        module_path: &[String],
    ) {
        for item in items {
            match item {
                syn::Item::Impl(impl_block) => {
                    let Some((_, trait_path, _)) = &impl_block.trait_ else {
                        continue;
                    };

                    let Some(tp) = Self::impl_self_type_path(impl_block.self_ty.as_ref()) else {
                        continue;
                    };

                    let raw_self_name = tp
                        .path
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    let resolved_raw_self_name =
                        self.resolve_nested_local_reexport_path(&raw_self_name);
                    let scoped_self_name = qualify_impl_type_name(
                        &raw_self_name,
                        module_path,
                        &self.declared_item_names,
                        &self.local_declared_types,
                    );
                    let resolved_scoped_self_name =
                        self.resolve_nested_local_reexport_path(&scoped_self_name);
                    if self.local_declared_types.contains(&raw_self_name)
                        || self.local_declared_types.contains(&scoped_self_name)
                        || self.local_declared_types.contains(&resolved_raw_self_name)
                        || self
                            .local_declared_types
                            .contains(&resolved_scoped_self_name)
                    {
                        continue;
                    }

                    let trait_scoped_key =
                        self.resolve_trait_scoped_key_for_impl(trait_path, module_path);
                    let entry = self
                        .extension_trait_impl_methods
                        .entry(trait_scoped_key)
                        .or_default();
                    let mut associated_type_bindings: HashMap<String, syn::Type> = HashMap::new();
                    for impl_item in &impl_block.items {
                        if let syn::ImplItem::Type(assoc) = impl_item {
                            associated_type_bindings
                                .insert(assoc.ident.to_string(), assoc.ty.clone());
                        }
                    }

                    for impl_item in &impl_block.items {
                        let syn::ImplItem::Fn(method) = impl_item else {
                            continue;
                        };
                        let mut merged = method.clone();
                        merge_impl_type_generics_into_method(&mut merged, &impl_block.generics);
                        Self::normalize_impl_method_receiver_for_reference_self(
                            &mut merged,
                            impl_block.self_ty.as_ref(),
                        );
                        let callable_param_metadata =
                            Self::collect_callable_param_bound_metadata_from_generics(
                                &merged.sig.generics,
                                &merged.sig.inputs,
                            );
                        let method_name = merged.sig.ident.to_string();
                        self.extension_method_names.insert(method_name.clone());
                        self.local_extension_method_names.insert(method_name);
                        let impl_generic_names: Vec<String> = impl_block
                            .generics
                            .params
                            .iter()
                            .filter_map(|p| match p {
                                syn::GenericParam::Type(tp) => Some(tp.ident.to_string()),
                                _ => None,
                            })
                            .collect();
                        entry.push(ExtensionImplMethod {
                            self_ty: (*impl_block.self_ty).clone(),
                            method: merged,
                            callable_param_metadata,
                            associated_type_bindings: associated_type_bindings.clone(),
                            impl_generic_names,
                            self_is_template_param: false,
                            extra_template_requires: None,
                        });
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested_items)) = &m.content {
                        let mut nested_path = module_path.to_vec();
                        nested_path.push(m.ident.to_string());
                        self.collect_extension_trait_impl_methods(nested_items, &nested_path);
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_callable_param_bound_metadata_from_generics(
        generics: &syn::Generics,
        inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
    ) -> HashMap<String, CallableParamBoundMetadata> {
        let mut callable_param_type_names: HashMap<String, String> = HashMap::new();
        for arg in inputs.iter().skip(1) {
            let syn::FnArg::Typed(pt) = arg else {
                continue;
            };
            let syn::Pat::Ident(pi) = pt.pat.as_ref() else {
                continue;
            };
            let syn::Type::Path(tp) = pt.ty.as_ref() else {
                continue;
            };
            if tp.qself.is_some() || tp.path.segments.len() != 1 {
                continue;
            }
            let type_name = tp.path.segments[0].ident.to_string();
            if type_name.is_empty() {
                continue;
            }
            callable_param_type_names.insert(pi.ident.to_string(), type_name);
        }
        if callable_param_type_names.is_empty() {
            return HashMap::new();
        }

        let mut type_bound_meta: HashMap<String, CallableParamBoundMetadata> = HashMap::new();
        let mut conflicted_type_params: HashSet<String> = HashSet::new();

        for param in &generics.params {
            let syn::GenericParam::Type(tp) = param else {
                continue;
            };
            let type_name = tp.ident.to_string();
            for bound in &tp.bounds {
                if let Some(meta) = Self::callable_bound_metadata_from_type_param_bound(bound) {
                    Self::record_callable_type_param_metadata(
                        &mut type_bound_meta,
                        &mut conflicted_type_params,
                        type_name.clone(),
                        meta,
                    );
                }
            }
        }

        if let Some(where_clause) = &generics.where_clause {
            for predicate in &where_clause.predicates {
                let syn::WherePredicate::Type(type_pred) = predicate else {
                    continue;
                };
                let syn::Type::Path(tp) = &type_pred.bounded_ty else {
                    continue;
                };
                if tp.qself.is_some() || tp.path.segments.len() != 1 {
                    continue;
                }
                let type_name = tp.path.segments[0].ident.to_string();
                for bound in &type_pred.bounds {
                    if let Some(meta) = Self::callable_bound_metadata_from_type_param_bound(bound) {
                        Self::record_callable_type_param_metadata(
                            &mut type_bound_meta,
                            &mut conflicted_type_params,
                            type_name.clone(),
                            meta,
                        );
                    }
                }
            }
        }

        let mut out: HashMap<String, CallableParamBoundMetadata> = HashMap::new();
        for (param_name, type_name) in callable_param_type_names {
            if conflicted_type_params.contains(&type_name) {
                continue;
            }
            if let Some(meta) = type_bound_meta.get(&type_name) {
                out.insert(param_name, meta.clone());
            }
        }
        out
    }

    pub(super) fn collect_hoistable_local_generic_structs_in_block(
        &self,
        block: &syn::Block,
    ) -> Vec<syn::ItemStruct> {
        let mut local_impl_template_targets: HashSet<String> = block
            .stmts
            .iter()
            .filter_map(|stmt| match stmt {
                syn::Stmt::Item(syn::Item::Impl(impl_block))
                    if Self::impl_block_emits_member_templates(impl_block) =>
                {
                    Self::local_impl_target_type_name(impl_block)
                }
                _ => None,
            })
            .collect();
        local_impl_template_targets
            .extend(Self::local_trait_impl_hoist_relevant_type_names(block));
        block
            .stmts
            .iter()
            .filter_map(|stmt| match stmt {
                syn::Stmt::Item(syn::Item::Struct(s)) => {
                    let has_type_or_const_generics = s.generics.params.iter().any(|param| {
                        matches!(
                            param,
                            syn::GenericParam::Type(_) | syn::GenericParam::Const(_)
                        )
                    });
                    let has_template_impl =
                        local_impl_template_targets.contains(&s.ident.to_string());
                    (has_type_or_const_generics || has_template_impl).then_some(s.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// Type names a fn-local TRAIT impl forces to namespace scope: the impl's
    /// self type plus every locally-relevant type its items name (associated
    /// `type Value = Void;` aliases, method signatures). The trait-impl
    /// machinery (UFCS free fns, traits-map adapters) only exists at
    /// namespace scope, and it spells these types module-qualified — a
    /// type left fn-local would be unnameable there.
    pub(super) fn local_trait_impl_hoist_relevant_type_names(
        block: &syn::Block,
    ) -> HashSet<String> {
        use syn::visit::Visit;
        struct TypeIdentCollector<'a> {
            out: &'a mut HashSet<String>,
        }
        impl<'ast> Visit<'ast> for TypeIdentCollector<'_> {
            fn visit_type_path(&mut self, tp: &'ast syn::TypePath) {
                if tp.qself.is_none()
                    && let Some(seg) = tp.path.segments.last()
                {
                    self.out.insert(seg.ident.to_string());
                }
                syn::visit::visit_type_path(self, tp);
            }
        }
        let mut names = HashSet::new();
        for stmt in &block.stmts {
            let syn::Stmt::Item(syn::Item::Impl(impl_block)) = stmt else {
                continue;
            };
            // Only traits whose machinery LIVES at namespace scope force a
            // hoist: serde adapters (`using Value = …` in the traits map)
            // and fmt dispatch (out-of-line members on the wrapper enum).
            // Drop/Default/Clone/… impls are handled fine at block scope —
            // and hoisting their host away from fn-local statics they
            // reference would break those references.
            let needs_namespace_scope = impl_block
                .trait_
                .as_ref()
                .and_then(|(_, path, _)| path.segments.last())
                .is_some_and(|seg| {
                    matches!(
                        seg.ident.to_string().as_str(),
                        "Visitor"
                            | "DeserializeSeed"
                            | "SeqAccess"
                            | "MapAccess"
                            | "EnumAccess"
                            | "VariantAccess"
                            | "Write"
                            | "Display"
                            | "Debug"
                    )
                });
            if !needs_namespace_scope {
                continue;
            }
            if let Some(target) = Self::local_impl_target_type_name(impl_block) {
                names.insert(target);
            }
            let mut collector = TypeIdentCollector { out: &mut names };
            for item in &impl_block.items {
                match item {
                    syn::ImplItem::Type(assoc) => collector.visit_type(&assoc.ty),
                    syn::ImplItem::Fn(f) => collector.visit_signature(&f.sig),
                    _ => {}
                }
            }
        }
        names
    }

    /// Inline fn-local `use` aliases into the block's ITEM stmts as full Rust
    /// paths. A hoisted local item (struct/impl/static) is emitted at
    /// namespace scope, where the fn-local `using` lines don't reach — a
    /// hoisted Drop impl's `Ordering::Relaxed` (via `use std::sync::atomic::
    /// Ordering;`) would be undeclared there. Rewriting to the full Rust path
    /// (`std::sync::atomic::Ordering::Relaxed`) lets the normal path mapping
    /// produce the qualified C++ spelling. Only multi-segment paths are
    /// rewritten (single idents resolve through the type mapper already), and
    /// only inside item stmts — plain statements keep using the local
    /// `using`s. Returns None when the block has no local use-aliases.
    pub(super) fn inline_local_use_aliases_into_block_items(
        &self,
        block: &syn::Block,
    ) -> Option<syn::Block> {
        fn collect_use_aliases(
            tree: &syn::UseTree,
            prefix: &mut Vec<syn::Ident>,
            out: &mut HashMap<String, Vec<syn::Ident>>,
        ) {
            match tree {
                syn::UseTree::Path(p) => {
                    prefix.push(p.ident.clone());
                    collect_use_aliases(&p.tree, prefix, out);
                    prefix.pop();
                }
                syn::UseTree::Name(n) => {
                    let mut full = prefix.clone();
                    full.push(n.ident.clone());
                    out.insert(n.ident.to_string(), full);
                }
                syn::UseTree::Rename(r) => {
                    let mut full = prefix.clone();
                    full.push(r.ident.clone());
                    out.insert(r.rename.to_string(), full);
                }
                syn::UseTree::Group(g) => {
                    for item in &g.items {
                        collect_use_aliases(item, prefix, out);
                    }
                }
                syn::UseTree::Glob(_) => {}
            }
        }
        let mut aliases: HashMap<String, Vec<syn::Ident>> = HashMap::new();
        for stmt in &block.stmts {
            if let syn::Stmt::Item(syn::Item::Use(use_item)) = stmt {
                let mut prefix = Vec::new();
                collect_use_aliases(&use_item.tree, &mut prefix, &mut aliases);
            }
        }
        // Self-referential aliases (`use foo::Bar;` giving Bar -> foo::Bar with
        // a 1-segment target) are fine; drop entries whose target IS the bare
        // alias (`use Bar;` — nothing to qualify).
        aliases.retain(|alias, full| full.len() > 1 || full[0] != *alias);
        if aliases.is_empty() {
            return None;
        }
        struct AliasInliner<'a> {
            aliases: &'a HashMap<String, Vec<syn::Ident>>,
        }
        impl syn::visit_mut::VisitMut for AliasInliner<'_> {
            fn visit_path_mut(&mut self, path: &mut syn::Path) {
                if path.leading_colon.is_none() && path.segments.len() >= 2 {
                    let first = &path.segments[0];
                    if first.arguments.is_none()
                        && let Some(full) = self.aliases.get(&first.ident.to_string())
                    {
                        let tail: Vec<syn::PathSegment> =
                            path.segments.iter().skip(1).cloned().collect();
                        let mut segments = syn::punctuated::Punctuated::new();
                        for ident in full {
                            segments.push(syn::PathSegment::from(ident.clone()));
                        }
                        for seg in tail {
                            segments.push(seg);
                        }
                        path.segments = segments;
                    }
                }
                syn::visit_mut::visit_path_mut(self, path);
            }
        }
        let mut rewritten = block.clone();
        let mut inliner = AliasInliner { aliases: &aliases };
        for stmt in &mut rewritten.stmts {
            if let syn::Stmt::Item(item) = stmt
                && !matches!(item, syn::Item::Use(_))
            {
                syn::visit_mut::VisitMut::visit_item_mut(&mut inliner, item);
            }
        }
        Some(rewritten)
    }

    /// Fn-local `static`s referenced by the hoisted local types' definitions
    /// or impls (`TrackedDrop`'s Drop counting into `static DROPPED`). A C++
    /// fn-local static isn't visible at namespace scope, so the hoisted item
    /// would reference an undeclared name — such statics must hoist too.
    pub(super) fn collect_hoistable_local_statics_for_hoisted_types(
        block: &syn::Block,
        hoisted_type_names: &HashSet<String>,
    ) -> Vec<syn::ItemStatic> {
        use syn::visit::Visit;
        struct PathIdentCollector<'a> {
            out: &'a mut HashSet<String>,
        }
        impl<'ast> Visit<'ast> for PathIdentCollector<'_> {
            fn visit_path(&mut self, path: &'ast syn::Path) {
                for seg in &path.segments {
                    self.out.insert(seg.ident.to_string());
                }
                syn::visit::visit_path(self, path);
            }
        }
        let mut referenced: HashSet<String> = HashSet::new();
        for stmt in &block.stmts {
            let syn::Stmt::Item(item) = stmt else {
                continue;
            };
            let is_hoisted_item = match item {
                syn::Item::Struct(s) => hoisted_type_names.contains(&s.ident.to_string()),
                syn::Item::Enum(e) => hoisted_type_names.contains(&e.ident.to_string()),
                syn::Item::Type(t) => hoisted_type_names.contains(&t.ident.to_string()),
                syn::Item::Impl(impl_block) => Self::local_impl_target_type_name(impl_block)
                    .is_some_and(|name| hoisted_type_names.contains(&name)),
                _ => false,
            };
            if is_hoisted_item {
                let mut collector = PathIdentCollector {
                    out: &mut referenced,
                };
                collector.visit_item(item);
            }
        }
        block
            .stmts
            .iter()
            .filter_map(|stmt| match stmt {
                syn::Stmt::Item(syn::Item::Static(s))
                    if referenced.contains(&s.ident.to_string()) =>
                {
                    Some(s.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// Local GENERIC `type X<T> = …` aliases in a function body. C++ forbids
    /// in-function templates, so these must be hoisted to namespace scope (where
    /// `template<typename T> using X = …` is legal). Non-generic local aliases are
    /// fine in-function and are NOT hoisted.
    pub(super) fn collect_hoistable_local_generic_type_aliases_in_block(
        &self,
        block: &syn::Block,
    ) -> Vec<syn::ItemType> {
        block
            .stmts
            .iter()
            .filter_map(|stmt| match stmt {
                syn::Stmt::Item(syn::Item::Type(t)) => {
                    let has_generics = t.generics.params.iter().any(|param| {
                        matches!(
                            param,
                            syn::GenericParam::Type(_) | syn::GenericParam::Const(_)
                        )
                    });
                    has_generics.then_some(t.clone())
                }
                _ => None,
            })
            .collect()
    }

    pub(super) fn collect_hoistable_local_enums_in_block(&self, block: &syn::Block) -> Vec<syn::ItemEnum> {
        let mut local_impl_template_targets: HashSet<String> = block
            .stmts
            .iter()
            .filter_map(|stmt| match stmt {
                syn::Stmt::Item(syn::Item::Impl(impl_block))
                    if Self::impl_block_emits_member_templates(impl_block) =>
                {
                    Self::local_impl_target_type_name(impl_block)
                }
                _ => None,
            })
            .collect();
        local_impl_template_targets
            .extend(Self::local_trait_impl_hoist_relevant_type_names(block));
        block
            .stmts
            .iter()
            .filter_map(|stmt| match stmt {
                syn::Stmt::Item(syn::Item::Enum(e))
                    if local_impl_template_targets.contains(&e.ident.to_string()) =>
                {
                    Some(e.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// Recursive helper: walk `items` (including nested mod contents) and
    /// collect (trait_name, self_cpp_type, methods) tuples for every
    /// `impl Trait for U` block where both `Trait` and `U` are locally
    /// declared. Foreign-type impls (handled via rusty_ext) are skipped.
    pub(super) fn collect_local_trait_impls_for_adapter(
        &self,
        items: &[syn::Item],
        module_path: &[String],
        out: &mut Vec<(
            String,
            Vec<String>,
            String,
            Vec<syn::ImplItemFn>,
            Vec<String>,
        )>,
    ) {
        for item in items {
            match item {
                syn::Item::Impl(impl_block) => {
                    let Some((_, trait_path, _)) = &impl_block.trait_ else {
                        continue;
                    };
                    let Some(trait_seg) = trait_path.segments.last() else {
                        continue;
                    };
                    let trait_name = trait_seg.ident.to_string();
                    // Extract the impl's trait generic args (e.g.,
                    // `["int32_t"]` for `impl Container<i32> for Foo`).
                    let mut trait_args: Vec<String> = match &trait_seg.arguments {
                        syn::PathArguments::AngleBracketed(args) => args
                            .args
                            .iter()
                            .filter_map(|a| match a {
                                syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                                syn::GenericArgument::Const(c) => {
                                    Some(self.emit_expr_to_string(c))
                                }
                                _ => None,
                            })
                            .collect(),
                        _ => Vec::new(),
                    };
                    // Extract associated-type bindings from the impl
                    // block (`type Item = i32;`). The trait declaration
                    // expanded each `type Item;` into an additional
                    // template parameter, so the Adapter spec must
                    // bind them concretely here in declaration order.
                    // We walk the items list looking for the matching
                    // trait declaration to recover that order.
                    let trait_assoc_names_in_order: Vec<String> = items
                        .iter()
                        .find_map(|i| {
                            if let syn::Item::Trait(t) = i {
                                if t.ident == trait_name.as_str() {
                                    return Some(
                                        t.items
                                            .iter()
                                            .filter_map(|ti| {
                                                if let syn::TraitItem::Type(t) = ti {
                                                    Some(t.ident.to_string())
                                                } else {
                                                    None
                                                }
                                            })
                                            .collect(),
                                    );
                                }
                            }
                            None
                        })
                        .unwrap_or_default();
                    for assoc_name in &trait_assoc_names_in_order {
                        let impl_binding = impl_block.items.iter().find_map(|i| {
                            if let syn::ImplItem::Type(t) = i {
                                if t.ident == assoc_name.as_str() {
                                    return Some(self.map_type(&t.ty));
                                }
                            }
                            None
                        });
                        // Skip impls that don't bind every associated
                        // type — the spec would be malformed.
                        let Some(binding) = impl_binding else {
                            continue;
                        };
                        trait_args.push(binding);
                    }
                    if matches!(
                        trait_name.as_str(),
                        "Send"
                            | "Sync"
                            | "Copy"
                            | "Clone"
                            | "Sized"
                            | "Unpin"
                            | "Drop"
                    ) {
                        continue;
                    }
                    // Skip operator traits (Add/Sub/Mul/...,
                    // PartialEq/PartialOrd, Index/Deref, ...). These
                    // lower to C++ operator overloads on the
                    // implementing struct directly — they have no
                    // class form to inherit from and no Adapter shape.
                    if map_operator_trait(&trait_name).is_some() {
                        continue;
                    }
                    // Skip if the trait declaration was skipped (e.g.,
                    // had associated constants). The Adapter would
                    // inherit from a class that doesn't exist.
                    if self.skipped_interface_traits.contains(&trait_name) {
                        continue;
                    }

                    let Some(tp) = Self::impl_self_type_path(impl_block.self_ty.as_ref())
                    else {
                        continue;
                    };
                    let raw_self_name = tp
                        .path
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    // Only handle local self types — foreign ones go via
                    // emit_trait_adapter_specializations (rusty_ext path).
                    if !self.local_declared_types.contains(&raw_self_name) {
                        continue;
                    }
                    // `#[cpp_inherit]` impls emit the concrete self type as a
                    // direct `struct Self : public Trait` subclass (handled in
                    // emit_struct), so suppress the TraitAdapter<Self> spec that
                    // would otherwise wrap it.
                    if Self::has_cpp_inherit_attr(&impl_block.attrs) {
                        continue;
                    }
                    // We need the trait's Interface class name to inherit
                    // from. For a locally-declared trait we have it. Under
                    // inline-rust, each `#if RUSTYCPP_RUST` block is transpiled
                    // independently, so a trait declared in a *different* block
                    // won't be in `trait_declared_paths` even though its C++
                    // class is emitted earlier in the same file. Accept such
                    // cross-block traits — they are referenced by a bare
                    // single-segment name and the Adapter inherits
                    // `class <trait_name>` (emitted by the trait's block).
                    //
                    // This cross-block relaxation applies ONLY in inline-rust
                    // mode. In NORMAL transpilation every local trait is already
                    // in `trait_declared_paths`, so a non-declared unqualified
                    // trait is a FOREIGN/prelude trait (e.g. `impl Iterator for
                    // BadIter`) with no local Interface class — emitting an
                    // Adapter for it produces a broken `IteratorAdapter<…>`
                    // (undeclared primary template + unresolved `Self::Item`).
                    // Skip those (restores pre-1a4f2a8 behavior for real crates).
                    let trait_locally_declared = self
                        .trait_declared_paths
                        .iter()
                        .any(|p| p.ends_with(&trait_name));
                    let accept_cross_block =
                        self.inline_rust_block && trait_path.segments.len() == 1;
                    if !trait_locally_declared && !accept_cross_block {
                        continue;
                    }

                    // Skip generic impls (`impl<T> Foo for Bar<T>`).
                    // These require partial template specialization, not
                    // a full specialization, and the emit path here only
                    // produces `template <> class FooAdapter<...>`. A
                    // generic-impl form would need an outer `template <T>`.
                    // Deferred follow-up.
                    if !impl_block.generics.params.is_empty() {
                        continue;
                    }

                    let self_cpp = self.map_type(impl_block.self_ty.as_ref());
                    let methods: Vec<syn::ImplItemFn> = impl_block
                        .items
                        .iter()
                        .filter_map(|i| {
                            if let syn::ImplItem::Fn(m) = i {
                                Some(m.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    if methods.is_empty() {
                        continue;
                    }
                    out.push((
                        trait_name,
                        trait_args,
                        self_cpp,
                        methods,
                        module_path.to_vec(),
                    ));
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested)) = &m.content {
                        let mut nested_path = module_path.to_vec();
                        nested_path.push(m.ident.to_string());
                        self.collect_local_trait_impls_for_adapter(nested, &nested_path, out);
                    }
                }
                _ => {}
            }
        }
    }

    /// A trait bound resolves to a CRATE-LOCAL trait (not the support-header
    /// concept of the same name). hashbrown defines its own `Allocator` trait
    /// (`allocate -> Result<_, ()>`), so a `where A: Allocator` bound refers to
    /// THAT trait — emitting the support-header `rusty::alloc::Allocator<A>`
    /// concept (which requires `Result<_, AllocError>`) wrongly fails the
    /// constraint. Same keystone rule as `map_std_type` / `crate_declares_std_
    /// named_type`: when the crate owns the name, keep it local. Precise — only
    /// crates that declare their own trait of that name are affected.
    fn trait_path_is_crate_declared_local(&self, path: &syn::Path) -> bool {
        path.segments.last().is_some_and(|seg| {
            self.ufcs_declared_trait_names
                .contains(&seg.ident.to_string())
        })
    }

    pub(super) fn collect_emitted_template_parts(
        &self,
        generics: &syn::Generics,
        include_type_defaults: bool,
    ) -> (Vec<String>, Vec<String>) {
        let mut params: Vec<String> = Vec::new();
        let mut emitted_type_params: Vec<&syn::TypeParam> = Vec::new();
        for param in &generics.params {
            match param {
                syn::GenericParam::Type(tp)
                    if !self
                        .template_param_is_already_visible_for_emission(&tp.ident.to_string()) =>
                {
                    if include_type_defaults && let Some(default) = &tp.default {
                        let default_ty = self.map_type(default);
                        params.push(format!("typename {} = {}", tp.ident, default_ty));
                    } else {
                        params.push(format!("typename {}", tp.ident));
                    }
                    emitted_type_params.push(tp);
                }
                syn::GenericParam::Const(cp)
                    if !self
                        .template_param_is_already_visible_for_emission(&cp.ident.to_string()) =>
                {
                    let const_ty = self.map_type(&cp.ty);
                    if let Some(default) = &cp.default {
                        let default_expr = self.emit_expr_to_string(default);
                        params.push(format!("{} {} = {}", const_ty, cp.ident, default_expr));
                    } else {
                        params.push(format!("{} {}", const_ty, cp.ident));
                    }
                }
                _ => {}
            }
        }

        if params.is_empty() {
            return (Vec::new(), Vec::new());
        }

        let constraints =
            self.collect_template_constraints_for_params(generics, &emitted_type_params);

        (params, constraints)
    }

    /// The constraint half of `collect_emitted_template_parts`, independent of
    /// the param-visibility filter (#89: the out-of-line class-template method
    /// prefix must repeat the class's `requires` clause, computed while the
    /// class params are already in scope).
    pub(super) fn collect_template_constraints_for_params(
        &self,
        generics: &syn::Generics,
        emitted_type_params: &[&syn::TypeParam],
    ) -> Vec<String> {
        let mut constraints: Vec<String> = Vec::new();
        // User-trait bounds intentionally emit NO constraint: nothing ever
        // generates a `<Trait>Facade` type, so the old
        // `<Trait>Facade::is_satisfied_by<T>()` clause was an unbound
        // identifier in every non-module output (modules already skipped it,
        // and impl-level bounds are dropped the same way). Only the
        // well-known std/runtime concepts survive as real constraints.

        for tp in emitted_type_params {
            for bound in &tp.bounds {
                if let syn::TypeParamBound::Trait(tb) = bound {
                    if let Some(concept) = well_known_concept_for_trait_path(&tb.path) {
                        if !self.trait_path_is_crate_declared_local(&tb.path) {
                            constraints.push(format!("{}<{}>", concept, tp.ident));
                        }
                    }
                }
            }
        }

        if let Some(where_clause) = &generics.where_clause {
            for pred in &where_clause.predicates {
                if let syn::WherePredicate::Type(pt) = pred {
                    let ty_name = self.map_type(&pt.bounded_ty);
                    for bound in &pt.bounds {
                        if let syn::TypeParamBound::Trait(tb) = bound {
                            if let Some(concept) = well_known_concept_for_trait_path(&tb.path) {
                                if !self.trait_path_is_crate_declared_local(&tb.path) {
                                    constraints.push(format!("{}<{}>", concept, ty_name));
                                }
                            }
                        }
                    }
                }
            }
        }

        constraints
    }

    pub(super) fn collect_current_struct_assoc_projection_names(
        &self,
        ty: &syn::Type,
        names: &mut HashSet<String>,
    ) {
        match ty {
            syn::Type::Path(tp) => {
                if let Some(current) = self.current_struct.as_deref() {
                    if let Some(qself) = &tp.qself {
                        if let syn::Type::Path(base) = qself.ty.as_ref() {
                            if base.path.segments.len() == 1 {
                                let base_name = base.path.segments[0].ident.to_string();
                                if base_name == "Self" || base_name == current {
                                    if let Some(last) = tp.path.segments.last() {
                                        names.insert(last.ident.to_string());
                                    }
                                }
                            }
                        }
                        self.collect_current_struct_assoc_projection_names(&qself.ty, names);
                    }
                    if tp.path.segments.len() >= 2 {
                        if let Some(first) = tp.path.segments.first() {
                            let first_name = first.ident.to_string();
                            if first_name == "Self" || first_name == current {
                                if let Some(last) = tp.path.segments.last() {
                                    names.insert(last.ident.to_string());
                                }
                            }
                        }
                    }
                }
                for segment in &tp.path.segments {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        for arg in &args.args {
                            if let syn::GenericArgument::Type(inner) = arg {
                                self.collect_current_struct_assoc_projection_names(inner, names);
                            }
                        }
                    }
                }
            }
            syn::Type::Reference(reference) => {
                self.collect_current_struct_assoc_projection_names(&reference.elem, names);
            }
            syn::Type::Paren(paren) => {
                self.collect_current_struct_assoc_projection_names(&paren.elem, names);
            }
            syn::Type::Group(group) => {
                self.collect_current_struct_assoc_projection_names(&group.elem, names);
            }
            syn::Type::Tuple(tuple) => {
                for elem in &tuple.elems {
                    self.collect_current_struct_assoc_projection_names(elem, names);
                }
            }
            syn::Type::Array(array) => {
                self.collect_current_struct_assoc_projection_names(&array.elem, names);
            }
            syn::Type::Slice(slice) => {
                self.collect_current_struct_assoc_projection_names(&slice.elem, names);
            }
            syn::Type::Ptr(ptr) => {
                self.collect_current_struct_assoc_projection_names(&ptr.elem, names);
            }
            _ => {}
        }
    }

    pub(super) fn collect_outer_in_scope_type_params(&self) -> Vec<String> {
        let mut names = Vec::new();
        for scope in &self.type_param_scope_order {
            names.extend(scope.iter().cloned());
        }
        names
    }

    pub(super) fn collect_local_generic_placeholder_candidate_owner_targets_in_stmts(
        &self,
        stmts: &[syn::Stmt],
        hints: &HashMap<String, syn::Type>,
        candidate_owner_targets: &mut HashMap<String, String>,
    ) {
        for stmt in stmts {
            self.collect_local_generic_placeholder_candidate_owner_targets_in_stmt(
                stmt,
                hints,
                candidate_owner_targets,
            );
        }
    }

    pub(super) fn collect_local_generic_placeholder_candidate_owner_targets_in_stmt(
        &self,
        stmt: &syn::Stmt,
        hints: &HashMap<String, syn::Type>,
        candidate_owner_targets: &mut HashMap<String, String>,
    ) {
        match stmt {
            syn::Stmt::Local(local) => {
                if let Some(name) = local_binding_name(local)
                    && !hints.contains_key(&name)
                    && !candidate_owner_targets.contains_key(&name)
                {
                    let has_placeholder_type =
                        get_local_type(local).is_some_and(type_has_generic_placeholder);
                    let owner_target = local
                        .init
                        .as_ref()
                        .and_then(|init| call_owner_placeholder_target(&init.expr));
                    let option_none_initializer = local
                        .init
                        .as_ref()
                        .is_some_and(|init| expr_is_option_none_constructor(&init.expr));
                    let has_placeholder_owner_call = owner_target.is_some();
                    if has_placeholder_type || has_placeholder_owner_call || option_none_initializer
                    {
                        if let Some(owner_target) = owner_target {
                            candidate_owner_targets.insert(name, owner_target);
                        } else if option_none_initializer {
                            candidate_owner_targets.insert(name, "Option".to_string());
                        }
                    }
                }
                if let Some(init) = &local.init {
                    self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                        &init.expr,
                        hints,
                        candidate_owner_targets,
                    );
                }
            }
            syn::Stmt::Expr(expr, _) => {
                self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                    expr,
                    hints,
                    candidate_owner_targets,
                );
            }
            syn::Stmt::Item(_) | syn::Stmt::Macro(_) => {}
        }
    }

    pub(super) fn collect_local_generic_placeholder_candidate_owner_targets_in_expr(
        &self,
        expr: &syn::Expr,
        hints: &HashMap<String, syn::Type>,
        candidate_owner_targets: &mut HashMap<String, String>,
    ) {
        match expr {
            syn::Expr::Block(block) => {
                self.collect_local_generic_placeholder_candidate_owner_targets_in_stmts(
                    &block.block.stmts,
                    hints,
                    candidate_owner_targets,
                );
            }
            syn::Expr::If(if_expr) => {
                self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                    &if_expr.cond,
                    hints,
                    candidate_owner_targets,
                );
                self.collect_local_generic_placeholder_candidate_owner_targets_in_stmts(
                    &if_expr.then_branch.stmts,
                    hints,
                    candidate_owner_targets,
                );
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                        else_expr,
                        hints,
                        candidate_owner_targets,
                    );
                }
            }
            syn::Expr::While(while_expr) => {
                self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                    &while_expr.cond,
                    hints,
                    candidate_owner_targets,
                );
                self.collect_local_generic_placeholder_candidate_owner_targets_in_stmts(
                    &while_expr.body.stmts,
                    hints,
                    candidate_owner_targets,
                );
            }
            syn::Expr::Loop(loop_expr) => {
                self.collect_local_generic_placeholder_candidate_owner_targets_in_stmts(
                    &loop_expr.body.stmts,
                    hints,
                    candidate_owner_targets,
                );
            }
            syn::Expr::ForLoop(for_expr) => {
                self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                    &for_expr.expr,
                    hints,
                    candidate_owner_targets,
                );
                self.collect_local_generic_placeholder_candidate_owner_targets_in_stmts(
                    &for_expr.body.stmts,
                    hints,
                    candidate_owner_targets,
                );
            }
            syn::Expr::Match(match_expr) => {
                self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                    &match_expr.expr,
                    hints,
                    candidate_owner_targets,
                );
                for arm in &match_expr.arms {
                    if let Some((_, guard)) = &arm.guard {
                        self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                            guard,
                            hints,
                            candidate_owner_targets,
                        );
                    }
                    self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                        &arm.body,
                        hints,
                        candidate_owner_targets,
                    );
                }
            }
            syn::Expr::Call(call) => {
                self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                    &call.func,
                    hints,
                    candidate_owner_targets,
                );
                for arg in &call.args {
                    self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                        arg,
                        hints,
                        candidate_owner_targets,
                    );
                }
            }
            syn::Expr::MethodCall(method_call) => {
                self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                    &method_call.receiver,
                    hints,
                    candidate_owner_targets,
                );
                for arg in &method_call.args {
                    self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                        arg,
                        hints,
                        candidate_owner_targets,
                    );
                }
            }
            syn::Expr::Struct(struct_expr) => {
                for field in &struct_expr.fields {
                    self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                        &field.expr,
                        hints,
                        candidate_owner_targets,
                    );
                }
                if let Some(rest) = &struct_expr.rest {
                    self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                        rest,
                        hints,
                        candidate_owner_targets,
                    );
                }
            }
            syn::Expr::Array(array_expr) => {
                for elem in &array_expr.elems {
                    self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                        elem,
                        hints,
                        candidate_owner_targets,
                    );
                }
            }
            syn::Expr::Tuple(tuple_expr) => {
                for elem in &tuple_expr.elems {
                    self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                        elem,
                        hints,
                        candidate_owner_targets,
                    );
                }
            }
            syn::Expr::Assign(assign) => {
                self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                    &assign.left,
                    hints,
                    candidate_owner_targets,
                );
                self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                    &assign.right,
                    hints,
                    candidate_owner_targets,
                );
            }
            syn::Expr::Unsafe(unsafe_expr) => {
                self.collect_local_generic_placeholder_candidate_owner_targets_in_stmts(
                    &unsafe_expr.block.stmts,
                    hints,
                    candidate_owner_targets,
                );
            }
            syn::Expr::Closure(closure_expr) => {
                self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                    &closure_expr.body,
                    hints,
                    candidate_owner_targets,
                );
            }
            syn::Expr::Await(await_expr) => {
                self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                    &await_expr.base,
                    hints,
                    candidate_owner_targets,
                );
            }
            syn::Expr::Try(try_expr) => {
                self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                    &try_expr.expr,
                    hints,
                    candidate_owner_targets,
                );
            }
            syn::Expr::Return(return_expr) => {
                if let Some(value) = &return_expr.expr {
                    self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                        value,
                        hints,
                        candidate_owner_targets,
                    );
                }
            }
            syn::Expr::Break(break_expr) => {
                if let Some(value) = &break_expr.expr {
                    self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                        value,
                        hints,
                        candidate_owner_targets,
                    );
                }
            }
            syn::Expr::Reference(reference_expr) => {
                self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                    &reference_expr.expr,
                    hints,
                    candidate_owner_targets,
                );
            }
            syn::Expr::Unary(unary_expr) => {
                self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                    &unary_expr.expr,
                    hints,
                    candidate_owner_targets,
                );
            }
            syn::Expr::Paren(paren_expr) => {
                self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                    &paren_expr.expr,
                    hints,
                    candidate_owner_targets,
                );
            }
            syn::Expr::Group(group_expr) => {
                self.collect_local_generic_placeholder_candidate_owner_targets_in_expr(
                    &group_expr.expr,
                    hints,
                    candidate_owner_targets,
                );
            }
            _ => {}
        }
    }

    pub(super) fn collect_local_generic_placeholder_function_call_hints_in_stmt(
        &self,
        stmt: &syn::Stmt,
        candidate_owner_targets: &HashMap<String, String>,
        hints: &mut HashMap<String, syn::Type>,
    ) {
        match stmt {
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    let context_expected_ty = get_local_type(local);
                    self.collect_local_generic_placeholder_function_call_hints_in_expr(
                        &init.expr,
                        candidate_owner_targets,
                        context_expected_ty,
                        hints,
                    );
                }
            }
            syn::Stmt::Expr(expr, semi) => {
                let context_expected_ty = if semi.is_none() {
                    self.current_return_type_hint()
                } else {
                    None
                };
                self.collect_local_generic_placeholder_function_call_hints_in_expr(
                    expr,
                    candidate_owner_targets,
                    context_expected_ty,
                    hints,
                );
            }
            syn::Stmt::Item(_) | syn::Stmt::Macro(_) => {}
        }
    }

    pub(super) fn collect_local_generic_placeholder_function_call_hints_in_expr(
        &self,
        expr: &syn::Expr,
        candidate_owner_targets: &HashMap<String, String>,
        context_expected_ty: Option<&syn::Type>,
        hints: &mut HashMap<String, syn::Type>,
    ) {
        match expr {
            syn::Expr::Call(call) => {
                self.collect_local_generic_placeholder_function_call_hints_from_call(
                    call,
                    candidate_owner_targets,
                    context_expected_ty,
                    hints,
                );
                self.collect_local_generic_placeholder_function_call_hints_in_expr(
                    &call.func,
                    candidate_owner_targets,
                    context_expected_ty,
                    hints,
                );
                let mut merged_substitutions = self
                    .function_call_type_arg_substitutions(call)
                    .unwrap_or_default();
                if let Some(expected_substitutions) = self
                    .call_owner_type_arg_substitutions_from_expected_type(call, context_expected_ty)
                {
                    merged_substitutions.extend(expected_substitutions);
                }
                let substitutions =
                    (!merged_substitutions.is_empty()).then_some(merged_substitutions);
                for (idx, arg) in call.args.iter().enumerate() {
                    let mut arg_expected = self.lookup_function_arg_expected_type_for_call(
                        call,
                        idx,
                        substitutions.as_ref(),
                    );
                    let expected_needs_owner_recovery =
                        arg_expected.as_ref().is_some_and(|expected| {
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
                    if (arg_expected.is_none() || expected_needs_owner_recovery)
                        && let Some(fallback) = self
                            .lookup_associated_call_arg_expected_type_fallback(call, idx, Some(arg))
                    {
                        let fallback = if let Some(substitutions) = substitutions.as_ref() {
                            self.substitute_type_params_in_type(&fallback, substitutions)
                        } else {
                            fallback
                        };
                        arg_expected = Some(fallback);
                    }
                    if arg_expected.is_none()
                        && let Some(fallback) = self
                            .infer_associated_call_arg_expected_type_from_call_expected_owner(
                                call,
                                context_expected_ty,
                                idx,
                            )
                    {
                        let fallback = if let Some(substitutions) = substitutions.as_ref() {
                            self.substitute_type_params_in_type(&fallback, substitutions)
                        } else {
                            fallback
                        };
                        arg_expected = Some(fallback);
                    }
                    if arg_expected.is_none() {
                        arg_expected = self.infer_tuple_struct_constructor_call_arg_expected_type(
                            call,
                            context_expected_ty,
                            idx,
                        );
                    }
                    if arg_expected.is_none() {
                        arg_expected = self.result_ctor_expected_arg_type_for_call(
                            call,
                            context_expected_ty,
                            idx,
                        );
                    }
                    self.collect_local_generic_placeholder_function_call_hints_in_expr(
                        arg,
                        candidate_owner_targets,
                        arg_expected.as_ref().or(context_expected_ty),
                        hints,
                    );
                }
            }
            syn::Expr::MethodCall(method_call) => {
                // Infer generic type from method calls on candidate variables.
                // e.g., `c.get_or_init(|| 92)` on `let c = OnceCell::new_()`
                // → infer T=i32 from the closure return type.
                if let Some(receiver_name) = extract_simple_local_ident(&method_call.receiver) {
                    if candidate_owner_targets.contains_key(&receiver_name)
                        && !hints.contains_key(&receiver_name)
                    {
                        let method = method_call.method.to_string();
                        let owner_target = candidate_owner_targets
                            .get(&receiver_name)
                            .map(String::as_str);
                        let infer_from_arg = |arg: &syn::Expr| {
                            self.infer_local_binding_type_from_initializer(arg)
                                .or_else(|| self.infer_simple_expr_type(arg))
                                .or_else(|| self.infer_hint_type_from_expr(arg))
                        };
                        if matches!(owner_target, Some("HashMap" | "Map"))
                            && method == "insert"
                            && method_call.args.len() >= 2
                        {
                            let key_ty = method_call.args.first().and_then(infer_from_arg);
                            let inferred_value_ty =
                                method_call.args.iter().nth(1).and_then(infer_from_arg);
                            let next_value_ty = if owner_target == Some("Map") {
                                method_call.args.iter().nth(1).and_then(|arg| {
                                    self.infer_map_value_hint_from_next_value_arg(arg)
                                })
                            } else {
                                None
                            };
                            let value_ty = if inferred_value_ty.as_ref().is_some_and(|ty| {
                                self.type_is_bare_generic_param_like(ty)
                                    || self.type_contains_in_scope_type_param(ty)
                                    || self.type_contains_unresolved_placeholder_like(ty)
                                    || self.type_contains_unbound_single_letter_generic(ty)
                            }) {
                                next_value_ty.or(inferred_value_ty)
                            } else {
                                inferred_value_ty.or(next_value_ty)
                            };
                            match (key_ty, value_ty) {
                                (Some(key_ty), Some(value_ty)) => {
                                    let normalized = if owner_target == Some("Map") {
                                        parse_quote!(Map<#key_ty, #value_ty>)
                                    } else {
                                        parse_quote!(HashMap<#key_ty, #value_ty>)
                                    };
                                    if self
                                        .type_is_placeholder_hint_candidate_allow_scoped_generics(
                                            &normalized,
                                        )
                                    {
                                        if std::env::var_os("RUSTY_CPP_DEBUG_HINT_INSERTS")
                                            .is_some()
                                        {
                                            eprintln!(
                                                "[rusty-cpp][hint-insert] phase=augment_local_generic_placeholder_hints_from_function_calls expr=MethodCall local={} source=map_insert owner_target={:?} hint={}",
                                                receiver_name,
                                                owner_target,
                                                quote::quote!(#normalized)
                                            );
                                        }
                                        hints.insert(receiver_name.clone(), normalized);
                                    }
                                }
                                (None, Some(value_ty)) if owner_target == Some("Map") => {
                                    let key_ty: syn::Type = parse_quote!(String);
                                    let normalized: syn::Type =
                                        parse_quote!(Map<#key_ty, #value_ty>);
                                    if self
                                        .type_is_placeholder_hint_candidate_allow_scoped_generics(
                                            &normalized,
                                        )
                                    {
                                        if std::env::var_os("RUSTY_CPP_DEBUG_HINT_INSERTS")
                                            .is_some()
                                        {
                                            eprintln!(
                                                "[rusty-cpp][hint-insert] phase=augment_local_generic_placeholder_hints_from_function_calls expr=MethodCall local={} source=map_insert owner_target={:?} hint={}",
                                                receiver_name,
                                                owner_target,
                                                quote::quote!(#normalized)
                                            );
                                        }
                                        hints.insert(receiver_name.clone(), normalized);
                                    }
                                }
                                _ => {}
                            }
                        }
                        if hints.contains_key(&receiver_name) {
                            self.collect_local_generic_placeholder_function_call_hints_in_expr(
                                &method_call.receiver,
                                candidate_owner_targets,
                                context_expected_ty,
                                hints,
                            );
                            for arg in &method_call.args {
                                self.collect_local_generic_placeholder_function_call_hints_in_expr(
                                    arg,
                                    candidate_owner_targets,
                                    context_expected_ty,
                                    hints,
                                );
                            }
                            return;
                        }
                        let inferred = match method.as_str() {
                            "get_or_init" | "get_or_try_init" => {
                                method_call.args.first().and_then(|arg| {
                                    if let syn::Expr::Closure(c) = peel_paren_group_expr(arg) {
                                        self.infer_type_from_closure_body(c)
                                    } else {
                                        None
                                    }
                                })
                            }
                            "set" | "push" => method_call.args.first().and_then(infer_from_arg),
                            "extend" => method_call
                                .args
                                .first()
                                .and_then(|arg| self.infer_iter_item_type_from_expr(arg)),
                            "get_or_insert" | "insert" | "replace"
                                if owner_target == Some("Option") =>
                            {
                                method_call.args.first().and_then(infer_from_arg)
                            }
                            "get_or_insert_with" | "unwrap_or_else"
                                if owner_target == Some("Option") =>
                            {
                                method_call.args.first().and_then(|arg| {
                                    if let syn::Expr::Closure(c) = peel_paren_group_expr(arg) {
                                        self.infer_type_from_closure_body(c)
                                            .or_else(|| self.infer_closure_return_type(arg))
                                    } else {
                                        None
                                    }
                                })
                            }
                            "unwrap_or" if owner_target == Some("Option") => {
                                method_call.args.first().and_then(infer_from_arg)
                            }
                            "unwrap_or_default" if owner_target == Some("Option") => {
                                context_expected_ty.cloned()
                            }
                            _ => None,
                        };
                        if let Some(ty) = inferred {
                            // Placeholder hints store the missing generic argument(s),
                            // not the fully wrapped owner type. The owner wrapper is
                            // reconstructed later from the initializer path.
                            let normalized = if owner_target == Some("Option") {
                                self.expected_option_type_arg(Some(&ty))
                                    .cloned()
                                    .unwrap_or(ty)
                            } else {
                                normalize_placeholder_hint_for_owner(owner_target, ty)
                            };
                            if self.type_is_placeholder_hint_candidate_allow_scoped_generics(
                                &normalized,
                            ) {
                                if std::env::var_os("RUSTY_CPP_DEBUG_HINT_INSERTS").is_some() {
                                    eprintln!(
                                        "[rusty-cpp][hint-insert] phase=augment_local_generic_placeholder_hints_from_function_calls expr=MethodCall local={} source=method_inferred owner_target={:?} hint={}",
                                        receiver_name,
                                        owner_target,
                                        quote::quote!(#normalized)
                                    );
                                }
                                hints.insert(receiver_name, normalized);
                            }
                        }
                    }
                }
                self.collect_local_generic_placeholder_function_call_hints_in_expr(
                    &method_call.receiver,
                    candidate_owner_targets,
                    context_expected_ty,
                    hints,
                );
                for arg in &method_call.args {
                    self.collect_local_generic_placeholder_function_call_hints_in_expr(
                        arg,
                        candidate_owner_targets,
                        context_expected_ty,
                        hints,
                    );
                }
            }
            syn::Expr::Path(path_expr) => {
                if path_expr.path.segments.len() == 1
                    && let Some(expected_ty) = context_expected_ty
                {
                    let name = path_expr.path.segments[0].ident.to_string();
                    if let Some(owner_target) = candidate_owner_targets.get(&name)
                        && self.context_expected_type_is_safe_placeholder_hint(
                            owner_target,
                            expected_ty,
                        )
                        && let Some(hint_ty) = self
                            .placeholder_hint_from_expected_argument_type(owner_target, expected_ty)
                        && self.type_is_placeholder_hint_candidate_allow_scoped_generics(&hint_ty)
                    {
                        let should_insert = hints.get(&name).is_none_or(|existing| {
                            !self.type_is_concrete_hint_candidate(existing)
                                && self.type_is_concrete_hint_candidate(&hint_ty)
                        });
                        if !should_insert {
                            return;
                        }
                        if std::env::var_os("RUSTY_CPP_DEBUG_HINT_INSERTS").is_some() {
                            eprintln!(
                                "[rusty-cpp][hint-insert] phase=augment_local_generic_placeholder_hints_from_function_calls expr=Path local={} source=path_expected owner_target={} hint={}",
                                name,
                                owner_target,
                                quote::quote!(#hint_ty)
                            );
                        }
                        hints.insert(name, hint_ty);
                    }
                }
            }
            syn::Expr::Assign(assign) => {
                if let Some(name) = extract_simple_local_ident(&assign.left)
                    && !hints.contains_key(&name)
                    && let Some(owner_target) = candidate_owner_targets.get(&name)
                {
                    let owner_target = owner_target.as_str();
                    let option_ctor_marker_ty = |ty: &syn::Type| {
                        let ty = self.peel_reference_paren_group_type(ty);
                        matches!(
                            ty,
                            syn::Type::Path(tp)
                                if tp.path.segments.last().is_some_and(|seg| {
                                    matches!(seg.ident.to_string().as_str(), "Some" | "None")
                                })
                        )
                    };
                    let infer_from_expr = |expr: &syn::Expr| {
                        self.infer_local_binding_type_from_initializer(expr)
                            .or_else(|| self.infer_simple_expr_type(expr))
                            .or_else(|| self.infer_hint_type_from_expr(expr))
                    };
                    let mut inferred = None;
                    if owner_target == "Option" {
                        let rhs = peel_paren_group_expr(&assign.right);
                        if let syn::Expr::Call(call) = rhs
                            && call.args.len() == 1
                            && let syn::Expr::Path(path_expr) = call.func.as_ref()
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
                                "Some"
                                    | "Option::Some"
                                    | "core::option::Option::Some"
                                    | "std::option::Option::Some"
                            ) {
                                inferred = call.args.first().and_then(infer_from_expr);
                                if inferred.is_none()
                                    && let Some(rhs_ty) = infer_from_expr(&assign.right)
                                {
                                    let rhs_inner = self
                                        .expected_option_type_arg(Some(&rhs_ty))
                                        .cloned()
                                        .unwrap_or(rhs_ty);
                                    if !option_ctor_marker_ty(&rhs_inner) {
                                        inferred = Some(rhs_inner);
                                    }
                                }
                            }
                        }
                    }
                    if inferred.is_none() {
                        inferred = infer_from_expr(&assign.right);
                    }
                    if let Some(ty) = inferred {
                        let normalized = if owner_target == "Option" {
                            self.expected_option_type_arg(Some(&ty))
                                .cloned()
                                .unwrap_or(ty)
                        } else {
                            normalize_placeholder_hint_for_owner(Some(owner_target), ty)
                        };
                        if self
                            .type_is_placeholder_hint_candidate_allow_scoped_generics(&normalized)
                            && !(owner_target == "Option" && option_ctor_marker_ty(&normalized))
                        {
                            if std::env::var_os("RUSTY_CPP_DEBUG_HINT_INSERTS").is_some() {
                                eprintln!(
                                    "[rusty-cpp][hint-insert] phase=augment_local_generic_placeholder_hints_from_function_calls expr=Assign local={} source=assign_rhs owner_target={} hint={}",
                                    name,
                                    owner_target,
                                    quote::quote!(#normalized)
                                );
                            }
                            hints.insert(name, normalized);
                        }
                    }
                }
                self.collect_local_generic_placeholder_function_call_hints_in_expr(
                    &assign.left,
                    candidate_owner_targets,
                    context_expected_ty,
                    hints,
                );
                self.collect_local_generic_placeholder_function_call_hints_in_expr(
                    &assign.right,
                    candidate_owner_targets,
                    context_expected_ty,
                    hints,
                );
            }
            syn::Expr::Block(block) => {
                for stmt in &block.block.stmts {
                    self.collect_local_generic_placeholder_function_call_hints_in_stmt(
                        stmt,
                        candidate_owner_targets,
                        hints,
                    );
                }
            }
            syn::Expr::If(if_expr) => {
                self.collect_local_generic_placeholder_function_call_hints_in_expr(
                    &if_expr.cond,
                    candidate_owner_targets,
                    context_expected_ty,
                    hints,
                );
                for stmt in &if_expr.then_branch.stmts {
                    self.collect_local_generic_placeholder_function_call_hints_in_stmt(
                        stmt,
                        candidate_owner_targets,
                        hints,
                    );
                }
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    self.collect_local_generic_placeholder_function_call_hints_in_expr(
                        else_expr,
                        candidate_owner_targets,
                        context_expected_ty,
                        hints,
                    );
                }
            }
            syn::Expr::While(while_expr) => {
                self.collect_local_generic_placeholder_function_call_hints_in_expr(
                    &while_expr.cond,
                    candidate_owner_targets,
                    context_expected_ty,
                    hints,
                );
                for stmt in &while_expr.body.stmts {
                    self.collect_local_generic_placeholder_function_call_hints_in_stmt(
                        stmt,
                        candidate_owner_targets,
                        hints,
                    );
                }
            }
            syn::Expr::Loop(loop_expr) => {
                for stmt in &loop_expr.body.stmts {
                    self.collect_local_generic_placeholder_function_call_hints_in_stmt(
                        stmt,
                        candidate_owner_targets,
                        hints,
                    );
                }
            }
            syn::Expr::ForLoop(for_expr) => {
                self.collect_local_generic_placeholder_function_call_hints_in_expr(
                    &for_expr.expr,
                    candidate_owner_targets,
                    context_expected_ty,
                    hints,
                );
                for stmt in &for_expr.body.stmts {
                    self.collect_local_generic_placeholder_function_call_hints_in_stmt(
                        stmt,
                        candidate_owner_targets,
                        hints,
                    );
                }
            }
            syn::Expr::Match(match_expr) => {
                self.collect_local_generic_placeholder_function_call_hints_in_expr(
                    &match_expr.expr,
                    candidate_owner_targets,
                    context_expected_ty,
                    hints,
                );
                for arm in &match_expr.arms {
                    if let Some((_, guard)) = &arm.guard {
                        self.collect_local_generic_placeholder_function_call_hints_in_expr(
                            guard,
                            candidate_owner_targets,
                            context_expected_ty,
                            hints,
                        );
                    }
                    self.collect_local_generic_placeholder_function_call_hints_in_expr(
                        &arm.body,
                        candidate_owner_targets,
                        context_expected_ty,
                        hints,
                    );
                }
            }
            syn::Expr::Struct(struct_expr) => {
                for field in &struct_expr.fields {
                    let field_expected = match &field.member {
                        syn::Member::Named(ident) => self.lookup_struct_literal_field_type(
                            struct_expr,
                            &ident.to_string(),
                            context_expected_ty,
                        ),
                        syn::Member::Unnamed(_) => None,
                    };
                    self.collect_local_generic_placeholder_function_call_hints_in_expr(
                        &field.expr,
                        candidate_owner_targets,
                        field_expected.as_ref().or(context_expected_ty),
                        hints,
                    );
                }
                if let Some(rest) = &struct_expr.rest {
                    self.collect_local_generic_placeholder_function_call_hints_in_expr(
                        rest,
                        candidate_owner_targets,
                        context_expected_ty,
                        hints,
                    );
                }
            }
            syn::Expr::Array(array_expr) => {
                for elem in &array_expr.elems {
                    self.collect_local_generic_placeholder_function_call_hints_in_expr(
                        elem,
                        candidate_owner_targets,
                        context_expected_ty,
                        hints,
                    );
                }
            }
            syn::Expr::Tuple(tuple_expr) => {
                for elem in &tuple_expr.elems {
                    self.collect_local_generic_placeholder_function_call_hints_in_expr(
                        elem,
                        candidate_owner_targets,
                        context_expected_ty,
                        hints,
                    );
                }
            }
            syn::Expr::Unsafe(unsafe_expr) => {
                for stmt in &unsafe_expr.block.stmts {
                    self.collect_local_generic_placeholder_function_call_hints_in_stmt(
                        stmt,
                        candidate_owner_targets,
                        hints,
                    );
                }
            }
            syn::Expr::Closure(closure_expr) => {
                self.collect_local_generic_placeholder_function_call_hints_in_expr(
                    &closure_expr.body,
                    candidate_owner_targets,
                    context_expected_ty,
                    hints,
                );
            }
            syn::Expr::Await(await_expr) => {
                self.collect_local_generic_placeholder_function_call_hints_in_expr(
                    &await_expr.base,
                    candidate_owner_targets,
                    context_expected_ty,
                    hints,
                );
            }
            syn::Expr::Try(try_expr) => {
                self.collect_local_generic_placeholder_function_call_hints_in_expr(
                    &try_expr.expr,
                    candidate_owner_targets,
                    context_expected_ty,
                    hints,
                );
            }
            syn::Expr::Return(return_expr) => {
                if let Some(value) = &return_expr.expr {
                    let return_expected = self.current_return_type_hint().or(context_expected_ty);
                    self.collect_local_generic_placeholder_function_call_hints_in_expr(
                        value,
                        candidate_owner_targets,
                        return_expected,
                        hints,
                    );
                }
            }
            syn::Expr::Break(break_expr) => {
                if let Some(value) = &break_expr.expr {
                    self.collect_local_generic_placeholder_function_call_hints_in_expr(
                        value,
                        candidate_owner_targets,
                        context_expected_ty,
                        hints,
                    );
                }
            }
            syn::Expr::Reference(reference_expr) => {
                self.collect_local_generic_placeholder_function_call_hints_in_expr(
                    &reference_expr.expr,
                    candidate_owner_targets,
                    context_expected_ty,
                    hints,
                );
            }
            syn::Expr::Unary(unary_expr) => {
                self.collect_local_generic_placeholder_function_call_hints_in_expr(
                    &unary_expr.expr,
                    candidate_owner_targets,
                    context_expected_ty,
                    hints,
                );
            }
            syn::Expr::Paren(paren_expr) => {
                self.collect_local_generic_placeholder_function_call_hints_in_expr(
                    &paren_expr.expr,
                    candidate_owner_targets,
                    context_expected_ty,
                    hints,
                );
            }
            syn::Expr::Group(group_expr) => {
                self.collect_local_generic_placeholder_function_call_hints_in_expr(
                    &group_expr.expr,
                    candidate_owner_targets,
                    context_expected_ty,
                    hints,
                );
            }
            syn::Expr::Let(let_expr) => {
                self.collect_local_generic_placeholder_function_call_hints_in_expr(
                    &let_expr.expr,
                    candidate_owner_targets,
                    context_expected_ty,
                    hints,
                );
            }
            _ => {}
        }
    }

    pub(super) fn collect_local_generic_placeholder_function_call_hints_from_call(
        &self,
        call: &syn::ExprCall,
        candidate_owner_targets: &HashMap<String, String>,
        context_expected_ty: Option<&syn::Type>,
        hints: &mut HashMap<String, syn::Type>,
    ) {
        let syn::Expr::Path(_) = call.func.as_ref() else {
            return;
        };
        let mut merged_substitutions = self
            .function_call_type_arg_substitutions(call)
            .unwrap_or_default();
        if let Some(expected_substitutions) =
            self.call_owner_type_arg_substitutions_from_expected_type(call, context_expected_ty)
        {
            merged_substitutions.extend(expected_substitutions);
        }
        let substitutions = (!merged_substitutions.is_empty()).then_some(merged_substitutions);
        for (idx, arg) in call.args.iter().enumerate() {
            let Some(name) = extract_value_consumed_local_ident(arg) else {
                continue;
            };
            if hints.contains_key(&name) {
                continue;
            }
            let Some(owner_target) = candidate_owner_targets.get(&name) else {
                continue;
            };
            let mut expected_ty =
                self.lookup_function_arg_expected_type_for_call(call, idx, substitutions.as_ref());
            let expected_needs_owner_recovery = expected_ty.as_ref().is_some_and(|expected| {
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
            if (expected_ty.is_none() || expected_needs_owner_recovery)
                && let Some(fallback) =
                    self.lookup_associated_call_arg_expected_type_fallback(call, idx, Some(arg))
            {
                let fallback = if let Some(substitutions) = substitutions.as_ref() {
                    self.substitute_type_params_in_type(&fallback, substitutions)
                } else {
                    fallback
                };
                expected_ty = Some(fallback);
            }
            if expected_ty.is_none()
                && let Some(fallback) = self
                    .infer_associated_call_arg_expected_type_from_call_expected_owner(
                        call,
                        context_expected_ty,
                        idx,
                    )
            {
                let fallback = if let Some(substitutions) = substitutions.as_ref() {
                    self.substitute_type_params_in_type(&fallback, substitutions)
                } else {
                    fallback
                };
                expected_ty = Some(fallback);
            }
            if expected_ty.is_none() {
                expected_ty = self.infer_tuple_struct_constructor_call_arg_expected_type(
                    call,
                    context_expected_ty,
                    idx,
                );
            }
            if expected_ty.is_none() {
                expected_ty = self.infer_tuple_constructor_arg_expected_type_from_context(
                    call,
                    context_expected_ty,
                    idx,
                );
            }
            if expected_ty.is_none() {
                expected_ty =
                    self.result_ctor_expected_arg_type_for_call(call, context_expected_ty, idx);
            }
            if expected_ty.is_none()
                && context_expected_ty.is_some_and(|ty| {
                    self.context_expected_type_is_safe_placeholder_hint(owner_target, ty)
                })
            {
                expected_ty = context_expected_ty.cloned();
            }
            let Some(expected_ty) = expected_ty else {
                continue;
            };
            let Some(hint_ty) =
                self.placeholder_hint_from_expected_argument_type(owner_target, &expected_ty)
            else {
                continue;
            };
            if self.type_is_placeholder_hint_candidate_allow_scoped_generics(&hint_ty) {
                if std::env::var_os("RUSTY_CPP_DEBUG_HINT_INSERTS").is_some() {
                    eprintln!(
                        "[rusty-cpp][hint-insert] phase=collect_local_generic_placeholder_function_call_hints_from_call expr=Call local={} source=call_arg_expected owner_target={} hint={}",
                        name,
                        owner_target,
                        quote::quote!(#hint_ty)
                    );
                }
                hints.insert(name, hint_ty);
            }
        }
    }

    pub(super) fn collect_mut_unsuffixed_seed_option_return_hints_in_stmt(
        &self,
        stmt: &syn::Stmt,
        candidates: &HashSet<String>,
        option_inner_ty: &syn::Type,
        hints: &mut HashMap<String, syn::Type>,
    ) {
        match stmt {
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                        &init.expr,
                        candidates,
                        option_inner_ty,
                        hints,
                    );
                }
            }
            syn::Stmt::Expr(expr, _) => {
                self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                    expr,
                    candidates,
                    option_inner_ty,
                    hints,
                );
            }
            syn::Stmt::Item(_) | syn::Stmt::Macro(_) => {}
        }
    }

    pub(super) fn collect_mut_unsuffixed_seed_option_return_hints_in_expr(
        &self,
        expr: &syn::Expr,
        candidates: &HashSet<String>,
        option_inner_ty: &syn::Type,
        hints: &mut HashMap<String, syn::Type>,
    ) {
        if let Some(name) = self.option_some_payload_local_name(expr)
            && candidates.contains(&name)
        {
            hints.insert(name, option_inner_ty.clone());
        }

        match expr {
            syn::Expr::Return(ret) => {
                if let Some(value) = &ret.expr {
                    self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                        value,
                        candidates,
                        option_inner_ty,
                        hints,
                    );
                }
            }
            syn::Expr::Call(call) => {
                self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                    &call.func,
                    candidates,
                    option_inner_ty,
                    hints,
                );
                for arg in &call.args {
                    self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                        arg,
                        candidates,
                        option_inner_ty,
                        hints,
                    );
                }
            }
            syn::Expr::MethodCall(mc) => {
                self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                    &mc.receiver,
                    candidates,
                    option_inner_ty,
                    hints,
                );
                for arg in &mc.args {
                    self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                        arg,
                        candidates,
                        option_inner_ty,
                        hints,
                    );
                }
            }
            syn::Expr::Assign(assign) => {
                self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                    &assign.left,
                    candidates,
                    option_inner_ty,
                    hints,
                );
                self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                    &assign.right,
                    candidates,
                    option_inner_ty,
                    hints,
                );
            }
            syn::Expr::Block(block) => {
                for stmt in &block.block.stmts {
                    self.collect_mut_unsuffixed_seed_option_return_hints_in_stmt(
                        stmt,
                        candidates,
                        option_inner_ty,
                        hints,
                    );
                }
            }
            syn::Expr::If(if_expr) => {
                self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                    &if_expr.cond,
                    candidates,
                    option_inner_ty,
                    hints,
                );
                for stmt in &if_expr.then_branch.stmts {
                    self.collect_mut_unsuffixed_seed_option_return_hints_in_stmt(
                        stmt,
                        candidates,
                        option_inner_ty,
                        hints,
                    );
                }
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                        else_expr,
                        candidates,
                        option_inner_ty,
                        hints,
                    );
                }
            }
            syn::Expr::While(while_expr) => {
                self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                    &while_expr.cond,
                    candidates,
                    option_inner_ty,
                    hints,
                );
                for stmt in &while_expr.body.stmts {
                    self.collect_mut_unsuffixed_seed_option_return_hints_in_stmt(
                        stmt,
                        candidates,
                        option_inner_ty,
                        hints,
                    );
                }
            }
            syn::Expr::Loop(loop_expr) => {
                for stmt in &loop_expr.body.stmts {
                    self.collect_mut_unsuffixed_seed_option_return_hints_in_stmt(
                        stmt,
                        candidates,
                        option_inner_ty,
                        hints,
                    );
                }
            }
            syn::Expr::ForLoop(for_expr) => {
                self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                    &for_expr.expr,
                    candidates,
                    option_inner_ty,
                    hints,
                );
                for stmt in &for_expr.body.stmts {
                    self.collect_mut_unsuffixed_seed_option_return_hints_in_stmt(
                        stmt,
                        candidates,
                        option_inner_ty,
                        hints,
                    );
                }
            }
            syn::Expr::Match(match_expr) => {
                self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                    &match_expr.expr,
                    candidates,
                    option_inner_ty,
                    hints,
                );
                for arm in &match_expr.arms {
                    if let Some((_, guard)) = &arm.guard {
                        self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                            guard,
                            candidates,
                            option_inner_ty,
                            hints,
                        );
                    }
                    self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                        &arm.body,
                        candidates,
                        option_inner_ty,
                        hints,
                    );
                }
            }
            syn::Expr::Struct(struct_expr) => {
                for field in &struct_expr.fields {
                    self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                        &field.expr,
                        candidates,
                        option_inner_ty,
                        hints,
                    );
                }
                if let Some(rest) = &struct_expr.rest {
                    self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                        rest,
                        candidates,
                        option_inner_ty,
                        hints,
                    );
                }
            }
            syn::Expr::Array(array_expr) => {
                for elem in &array_expr.elems {
                    self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                        elem,
                        candidates,
                        option_inner_ty,
                        hints,
                    );
                }
            }
            syn::Expr::Tuple(tuple_expr) => {
                for elem in &tuple_expr.elems {
                    self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                        elem,
                        candidates,
                        option_inner_ty,
                        hints,
                    );
                }
            }
            syn::Expr::Unsafe(unsafe_expr) => {
                for stmt in &unsafe_expr.block.stmts {
                    self.collect_mut_unsuffixed_seed_option_return_hints_in_stmt(
                        stmt,
                        candidates,
                        option_inner_ty,
                        hints,
                    );
                }
            }
            syn::Expr::Closure(closure_expr) => {
                self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                    &closure_expr.body,
                    candidates,
                    option_inner_ty,
                    hints,
                );
            }
            syn::Expr::Await(await_expr) => {
                self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                    &await_expr.base,
                    candidates,
                    option_inner_ty,
                    hints,
                );
            }
            syn::Expr::Try(try_expr) => {
                self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                    &try_expr.expr,
                    candidates,
                    option_inner_ty,
                    hints,
                );
            }
            syn::Expr::Break(break_expr) => {
                if let Some(value) = &break_expr.expr {
                    self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                        value,
                        candidates,
                        option_inner_ty,
                        hints,
                    );
                }
            }
            syn::Expr::Reference(reference_expr) => {
                self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                    &reference_expr.expr,
                    candidates,
                    option_inner_ty,
                    hints,
                );
            }
            syn::Expr::Unary(unary_expr) => {
                self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                    &unary_expr.expr,
                    candidates,
                    option_inner_ty,
                    hints,
                );
            }
            syn::Expr::Paren(paren_expr) => {
                self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                    &paren_expr.expr,
                    candidates,
                    option_inner_ty,
                    hints,
                );
            }
            syn::Expr::Group(group_expr) => {
                self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                    &group_expr.expr,
                    candidates,
                    option_inner_ty,
                    hints,
                );
            }
            syn::Expr::Let(let_expr) => {
                self.collect_mut_unsuffixed_seed_option_return_hints_in_expr(
                    &let_expr.expr,
                    candidates,
                    option_inner_ty,
                    hints,
                );
            }
            _ => {}
        }
    }

    pub(super) fn collect_oncecell_fallback_inner_type_hint(&self, stmts: &[syn::Stmt]) -> Option<syn::Type> {
        let mut enumerate_index_names: HashSet<String> = HashSet::new();
        for stmt in stmts {
            self.collect_enumerate_index_binding_names_in_stmt(stmt, &mut enumerate_index_names);
        }

        let mut inferred_inner: Option<syn::Type> = None;
        let mut conflict = false;
        for stmt in stmts {
            self.collect_oncecell_inner_type_candidates_in_stmt(
                stmt,
                &enumerate_index_names,
                &mut inferred_inner,
                &mut conflict,
            );
            if conflict {
                return None;
            }
        }
        inferred_inner
    }

    pub(super) fn collect_enumerate_index_binding_names_in_stmt(
        &self,
        stmt: &syn::Stmt,
        names: &mut HashSet<String>,
    ) {
        match stmt {
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.collect_enumerate_index_binding_names_in_expr(&init.expr, names);
                }
            }
            syn::Stmt::Expr(expr, _) => {
                self.collect_enumerate_index_binding_names_in_expr(expr, names);
            }
            syn::Stmt::Item(_) | syn::Stmt::Macro(_) => {}
        }
    }

    pub(super) fn collect_enumerate_index_binding_names_in_expr(
        &self,
        expr: &syn::Expr,
        names: &mut HashSet<String>,
    ) {
        match expr {
            syn::Expr::ForLoop(for_expr) => {
                if let Some(index_name) = self.for_loop_enumerate_index_binding_name(for_expr) {
                    names.insert(index_name);
                }
                self.collect_enumerate_index_binding_names_in_expr(&for_expr.expr, names);
                for stmt in &for_expr.body.stmts {
                    self.collect_enumerate_index_binding_names_in_stmt(stmt, names);
                }
            }
            syn::Expr::Call(call) => {
                self.collect_enumerate_index_binding_names_in_expr(&call.func, names);
                for arg in &call.args {
                    self.collect_enumerate_index_binding_names_in_expr(arg, names);
                }
            }
            syn::Expr::MethodCall(method_call) => {
                self.collect_enumerate_index_binding_names_in_expr(&method_call.receiver, names);
                for arg in &method_call.args {
                    self.collect_enumerate_index_binding_names_in_expr(arg, names);
                }
            }
            syn::Expr::Assign(assign) => {
                self.collect_enumerate_index_binding_names_in_expr(&assign.left, names);
                self.collect_enumerate_index_binding_names_in_expr(&assign.right, names);
            }
            syn::Expr::Block(block) => {
                for stmt in &block.block.stmts {
                    self.collect_enumerate_index_binding_names_in_stmt(stmt, names);
                }
            }
            syn::Expr::If(if_expr) => {
                self.collect_enumerate_index_binding_names_in_expr(&if_expr.cond, names);
                for stmt in &if_expr.then_branch.stmts {
                    self.collect_enumerate_index_binding_names_in_stmt(stmt, names);
                }
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    self.collect_enumerate_index_binding_names_in_expr(else_expr, names);
                }
            }
            syn::Expr::While(while_expr) => {
                self.collect_enumerate_index_binding_names_in_expr(&while_expr.cond, names);
                for stmt in &while_expr.body.stmts {
                    self.collect_enumerate_index_binding_names_in_stmt(stmt, names);
                }
            }
            syn::Expr::Loop(loop_expr) => {
                for stmt in &loop_expr.body.stmts {
                    self.collect_enumerate_index_binding_names_in_stmt(stmt, names);
                }
            }
            syn::Expr::Match(match_expr) => {
                self.collect_enumerate_index_binding_names_in_expr(&match_expr.expr, names);
                for arm in &match_expr.arms {
                    if let Some((_, guard)) = &arm.guard {
                        self.collect_enumerate_index_binding_names_in_expr(guard, names);
                    }
                    self.collect_enumerate_index_binding_names_in_expr(&arm.body, names);
                }
            }
            syn::Expr::Struct(struct_expr) => {
                for field in &struct_expr.fields {
                    self.collect_enumerate_index_binding_names_in_expr(&field.expr, names);
                }
                if let Some(rest) = &struct_expr.rest {
                    self.collect_enumerate_index_binding_names_in_expr(rest, names);
                }
            }
            syn::Expr::Array(array_expr) => {
                for elem in &array_expr.elems {
                    self.collect_enumerate_index_binding_names_in_expr(elem, names);
                }
            }
            syn::Expr::Tuple(tuple_expr) => {
                for elem in &tuple_expr.elems {
                    self.collect_enumerate_index_binding_names_in_expr(elem, names);
                }
            }
            syn::Expr::Unsafe(unsafe_expr) => {
                for stmt in &unsafe_expr.block.stmts {
                    self.collect_enumerate_index_binding_names_in_stmt(stmt, names);
                }
            }
            syn::Expr::Closure(closure_expr) => {
                self.collect_enumerate_index_binding_names_in_expr(&closure_expr.body, names);
            }
            syn::Expr::Await(await_expr) => {
                self.collect_enumerate_index_binding_names_in_expr(&await_expr.base, names);
            }
            syn::Expr::Try(try_expr) => {
                self.collect_enumerate_index_binding_names_in_expr(&try_expr.expr, names);
            }
            syn::Expr::Return(return_expr) => {
                if let Some(value) = &return_expr.expr {
                    self.collect_enumerate_index_binding_names_in_expr(value, names);
                }
            }
            syn::Expr::Break(break_expr) => {
                if let Some(value) = &break_expr.expr {
                    self.collect_enumerate_index_binding_names_in_expr(value, names);
                }
            }
            syn::Expr::Reference(reference_expr) => {
                self.collect_enumerate_index_binding_names_in_expr(&reference_expr.expr, names);
            }
            syn::Expr::Unary(unary_expr) => {
                self.collect_enumerate_index_binding_names_in_expr(&unary_expr.expr, names);
            }
            syn::Expr::Paren(paren_expr) => {
                self.collect_enumerate_index_binding_names_in_expr(&paren_expr.expr, names);
            }
            syn::Expr::Group(group_expr) => {
                self.collect_enumerate_index_binding_names_in_expr(&group_expr.expr, names);
            }
            syn::Expr::Let(let_expr) => {
                self.collect_enumerate_index_binding_names_in_expr(&let_expr.expr, names);
            }
            _ => {}
        }
    }

    pub(super) fn collect_oncecell_inner_type_candidates_in_stmt(
        &self,
        stmt: &syn::Stmt,
        enumerate_index_names: &HashSet<String>,
        inferred_inner: &mut Option<syn::Type>,
        conflict: &mut bool,
    ) {
        match stmt {
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.collect_oncecell_inner_type_candidates_in_expr(
                        &init.expr,
                        enumerate_index_names,
                        inferred_inner,
                        conflict,
                    );
                }
            }
            syn::Stmt::Expr(expr, _) => {
                self.collect_oncecell_inner_type_candidates_in_expr(
                    expr,
                    enumerate_index_names,
                    inferred_inner,
                    conflict,
                );
            }
            syn::Stmt::Item(_) | syn::Stmt::Macro(_) => {}
        }
    }

    pub(super) fn collect_oncecell_inner_type_candidates_in_expr(
        &self,
        expr: &syn::Expr,
        enumerate_index_names: &HashSet<String>,
        inferred_inner: &mut Option<syn::Type>,
        conflict: &mut bool,
    ) {
        if *conflict {
            return;
        }
        match expr {
            syn::Expr::MethodCall(method_call) => {
                let method = method_call.method.to_string();
                let candidate = match method.as_str() {
                    "set" => method_call.args.first().and_then(|arg| {
                        self.infer_simple_expr_type(arg)
                            .or_else(|| self.infer_hint_type_from_expr(arg))
                    }),
                    "get_or_init" => method_call.args.first().and_then(|arg| {
                        self.infer_oncecell_inner_type_from_get_or_init_arg(
                            arg,
                            enumerate_index_names,
                        )
                    }),
                    "get_or_try_init" => method_call.args.first().and_then(|arg| {
                        self.infer_oncecell_inner_type_from_get_or_try_init_arg(
                            arg,
                            enumerate_index_names,
                        )
                    }),
                    _ => None,
                };
                self.record_oncecell_inner_type_candidate(candidate, inferred_inner, conflict);

                self.collect_oncecell_inner_type_candidates_in_expr(
                    &method_call.receiver,
                    enumerate_index_names,
                    inferred_inner,
                    conflict,
                );
                for arg in &method_call.args {
                    self.collect_oncecell_inner_type_candidates_in_expr(
                        arg,
                        enumerate_index_names,
                        inferred_inner,
                        conflict,
                    );
                }
            }
            syn::Expr::Call(call) => {
                self.collect_oncecell_inner_type_candidates_in_expr(
                    &call.func,
                    enumerate_index_names,
                    inferred_inner,
                    conflict,
                );
                for arg in &call.args {
                    self.collect_oncecell_inner_type_candidates_in_expr(
                        arg,
                        enumerate_index_names,
                        inferred_inner,
                        conflict,
                    );
                }
            }
            syn::Expr::Assign(assign) => {
                self.collect_oncecell_inner_type_candidates_in_expr(
                    &assign.left,
                    enumerate_index_names,
                    inferred_inner,
                    conflict,
                );
                self.collect_oncecell_inner_type_candidates_in_expr(
                    &assign.right,
                    enumerate_index_names,
                    inferred_inner,
                    conflict,
                );
            }
            syn::Expr::Block(block) => {
                for stmt in &block.block.stmts {
                    self.collect_oncecell_inner_type_candidates_in_stmt(
                        stmt,
                        enumerate_index_names,
                        inferred_inner,
                        conflict,
                    );
                }
            }
            syn::Expr::If(if_expr) => {
                self.collect_oncecell_inner_type_candidates_in_expr(
                    &if_expr.cond,
                    enumerate_index_names,
                    inferred_inner,
                    conflict,
                );
                for stmt in &if_expr.then_branch.stmts {
                    self.collect_oncecell_inner_type_candidates_in_stmt(
                        stmt,
                        enumerate_index_names,
                        inferred_inner,
                        conflict,
                    );
                }
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    self.collect_oncecell_inner_type_candidates_in_expr(
                        else_expr,
                        enumerate_index_names,
                        inferred_inner,
                        conflict,
                    );
                }
            }
            syn::Expr::While(while_expr) => {
                self.collect_oncecell_inner_type_candidates_in_expr(
                    &while_expr.cond,
                    enumerate_index_names,
                    inferred_inner,
                    conflict,
                );
                for stmt in &while_expr.body.stmts {
                    self.collect_oncecell_inner_type_candidates_in_stmt(
                        stmt,
                        enumerate_index_names,
                        inferred_inner,
                        conflict,
                    );
                }
            }
            syn::Expr::Loop(loop_expr) => {
                for stmt in &loop_expr.body.stmts {
                    self.collect_oncecell_inner_type_candidates_in_stmt(
                        stmt,
                        enumerate_index_names,
                        inferred_inner,
                        conflict,
                    );
                }
            }
            syn::Expr::ForLoop(for_expr) => {
                self.collect_oncecell_inner_type_candidates_in_expr(
                    &for_expr.expr,
                    enumerate_index_names,
                    inferred_inner,
                    conflict,
                );
                for stmt in &for_expr.body.stmts {
                    self.collect_oncecell_inner_type_candidates_in_stmt(
                        stmt,
                        enumerate_index_names,
                        inferred_inner,
                        conflict,
                    );
                }
            }
            syn::Expr::Match(match_expr) => {
                self.collect_oncecell_inner_type_candidates_in_expr(
                    &match_expr.expr,
                    enumerate_index_names,
                    inferred_inner,
                    conflict,
                );
                for arm in &match_expr.arms {
                    if let Some((_, guard)) = &arm.guard {
                        self.collect_oncecell_inner_type_candidates_in_expr(
                            guard,
                            enumerate_index_names,
                            inferred_inner,
                            conflict,
                        );
                    }
                    self.collect_oncecell_inner_type_candidates_in_expr(
                        &arm.body,
                        enumerate_index_names,
                        inferred_inner,
                        conflict,
                    );
                }
            }
            syn::Expr::Struct(struct_expr) => {
                for field in &struct_expr.fields {
                    self.collect_oncecell_inner_type_candidates_in_expr(
                        &field.expr,
                        enumerate_index_names,
                        inferred_inner,
                        conflict,
                    );
                }
                if let Some(rest) = &struct_expr.rest {
                    self.collect_oncecell_inner_type_candidates_in_expr(
                        rest,
                        enumerate_index_names,
                        inferred_inner,
                        conflict,
                    );
                }
            }
            syn::Expr::Array(array_expr) => {
                for elem in &array_expr.elems {
                    self.collect_oncecell_inner_type_candidates_in_expr(
                        elem,
                        enumerate_index_names,
                        inferred_inner,
                        conflict,
                    );
                }
            }
            syn::Expr::Tuple(tuple_expr) => {
                for elem in &tuple_expr.elems {
                    self.collect_oncecell_inner_type_candidates_in_expr(
                        elem,
                        enumerate_index_names,
                        inferred_inner,
                        conflict,
                    );
                }
            }
            syn::Expr::Unsafe(unsafe_expr) => {
                for stmt in &unsafe_expr.block.stmts {
                    self.collect_oncecell_inner_type_candidates_in_stmt(
                        stmt,
                        enumerate_index_names,
                        inferred_inner,
                        conflict,
                    );
                }
            }
            syn::Expr::Closure(closure_expr) => {
                self.collect_oncecell_inner_type_candidates_in_expr(
                    &closure_expr.body,
                    enumerate_index_names,
                    inferred_inner,
                    conflict,
                );
            }
            syn::Expr::Await(await_expr) => {
                self.collect_oncecell_inner_type_candidates_in_expr(
                    &await_expr.base,
                    enumerate_index_names,
                    inferred_inner,
                    conflict,
                );
            }
            syn::Expr::Try(try_expr) => {
                self.collect_oncecell_inner_type_candidates_in_expr(
                    &try_expr.expr,
                    enumerate_index_names,
                    inferred_inner,
                    conflict,
                );
            }
            syn::Expr::Return(return_expr) => {
                if let Some(value) = &return_expr.expr {
                    self.collect_oncecell_inner_type_candidates_in_expr(
                        value,
                        enumerate_index_names,
                        inferred_inner,
                        conflict,
                    );
                }
            }
            syn::Expr::Break(break_expr) => {
                if let Some(value) = &break_expr.expr {
                    self.collect_oncecell_inner_type_candidates_in_expr(
                        value,
                        enumerate_index_names,
                        inferred_inner,
                        conflict,
                    );
                }
            }
            syn::Expr::Reference(reference_expr) => {
                self.collect_oncecell_inner_type_candidates_in_expr(
                    &reference_expr.expr,
                    enumerate_index_names,
                    inferred_inner,
                    conflict,
                );
            }
            syn::Expr::Unary(unary_expr) => {
                self.collect_oncecell_inner_type_candidates_in_expr(
                    &unary_expr.expr,
                    enumerate_index_names,
                    inferred_inner,
                    conflict,
                );
            }
            syn::Expr::Paren(paren_expr) => {
                self.collect_oncecell_inner_type_candidates_in_expr(
                    &paren_expr.expr,
                    enumerate_index_names,
                    inferred_inner,
                    conflict,
                );
            }
            syn::Expr::Group(group_expr) => {
                self.collect_oncecell_inner_type_candidates_in_expr(
                    &group_expr.expr,
                    enumerate_index_names,
                    inferred_inner,
                    conflict,
                );
            }
            syn::Expr::Let(let_expr) => {
                self.collect_oncecell_inner_type_candidates_in_expr(
                    &let_expr.expr,
                    enumerate_index_names,
                    inferred_inner,
                    conflict,
                );
            }
            _ => {}
        }
    }

    pub(super) fn collect_uninitialized_local_type_hints_in_stmt(
        &mut self,
        stmt: &syn::Stmt,
        candidates: &HashSet<String>,
        hints: &mut HashMap<String, syn::Type>,
    ) {
        match stmt {
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.collect_uninitialized_local_type_hints_in_expr(
                        &init.expr, candidates, hints,
                    );
                }
            }
            syn::Stmt::Expr(expr, _) => {
                self.collect_uninitialized_local_type_hints_in_expr(expr, candidates, hints);
            }
            syn::Stmt::Item(_) | syn::Stmt::Macro(_) => {}
        }
    }

    pub(super) fn collect_uninitialized_local_type_hints_in_expr(
        &mut self,
        expr: &syn::Expr,
        candidates: &HashSet<String>,
        hints: &mut HashMap<String, syn::Type>,
    ) {
        match expr {
            syn::Expr::Call(call) => {
                self.collect_uninitialized_local_type_hints_from_call(call, candidates, hints);
                self.collect_uninitialized_local_type_hints_in_expr(&call.func, candidates, hints);
                for arg in &call.args {
                    self.collect_uninitialized_local_type_hints_in_expr(arg, candidates, hints);
                }
            }
            syn::Expr::MethodCall(method_call) => {
                self.collect_uninitialized_local_type_hints_from_method_call(
                    method_call,
                    candidates,
                    hints,
                );
                self.collect_uninitialized_local_type_hints_in_expr(
                    &method_call.receiver,
                    candidates,
                    hints,
                );
                for arg in &method_call.args {
                    self.collect_uninitialized_local_type_hints_in_expr(arg, candidates, hints);
                }
            }
            syn::Expr::Assign(assign) => {
                if let Some(name) = extract_simple_local_ident(&assign.left) {
                    if candidates.contains(&name) && !hints.contains_key(&name) {
                        let debug_hint_sources =
                            std::env::var_os("RUSTY_CPP_DEBUG_OPTION_HINT_SOURCES").is_some();
                        let rhs_candidates: [(&str, Option<syn::Type>); 6] = [
                            (
                                "infer_local_binding_type_from_initializer",
                                self.infer_local_binding_type_from_initializer(&assign.right),
                            ),
                            (
                                "infer_simple_expr_type",
                                self.infer_simple_expr_type(&assign.right),
                            ),
                            (
                                "infer_option_hint_type_from_some_expr",
                                self.infer_option_hint_type_from_some_expr(assign.right.as_ref()),
                            ),
                            (
                                "infer_option_hint_type_from_return_context_for_unresolved_some",
                                self.extract_option_some_call_arg(assign.right.as_ref())
                                    .and_then(|_| {
                                        self.infer_option_hint_type_from_return_context_for_unresolved_some()
                                    }),
                            ),
                            (
                                "lookup_local_binding_type(rhs_ident)",
                                extract_simple_local_ident(&assign.right)
                                    .and_then(|rhs_name| self.lookup_local_binding_type(&rhs_name)),
                            ),
                            (
                                "infer_type_param_static_conversion_call_return_type",
                                self.infer_type_param_static_conversion_call_return_type(&assign.right),
                            ),
                        ];
                        let mut chosen: Option<(String, syn::Type)> = None;
                        for (source, candidate) in rhs_candidates {
                            if let Some(rhs_ty) = candidate {
                                let accepted = self
                                    .type_is_placeholder_hint_candidate_allow_scoped_generics(
                                        &rhs_ty,
                                    );
                                if debug_hint_sources {
                                    eprintln!(
                                        "[rusty-cpp][option-hint-source] local={} source={} accepted={} ty={}",
                                        name,
                                        source,
                                        accepted,
                                        quote::quote!(#rhs_ty)
                                    );
                                }
                                if accepted {
                                    chosen = Some((source.to_string(), rhs_ty));
                                    break;
                                }
                            } else if debug_hint_sources {
                                eprintln!(
                                    "[rusty-cpp][option-hint-source] local={} source={} accepted=false ty=<none>",
                                    name, source
                                );
                            }
                        }
                        if let Some((source, rhs_ty)) = chosen {
                            if debug_hint_sources {
                                eprintln!(
                                    "[rusty-cpp][option-hint-source] local={} chosen={} ty={}",
                                    name,
                                    source,
                                    quote::quote!(#rhs_ty)
                                );
                            }
                            hints.insert(name, rhs_ty);
                        }
                    }
                }
                self.collect_uninitialized_local_type_hints_in_expr(
                    &assign.left,
                    candidates,
                    hints,
                );
                self.collect_uninitialized_local_type_hints_in_expr(
                    &assign.right,
                    candidates,
                    hints,
                );
            }
            syn::Expr::Block(block) => {
                self.with_pre_scan_known_local_scope(&block.block.stmts, |this| {
                    for stmt in &block.block.stmts {
                        this.collect_uninitialized_local_type_hints_in_stmt(
                            stmt, candidates, hints,
                        );
                    }
                });
            }
            syn::Expr::If(if_expr) => {
                self.collect_uninitialized_local_type_hints_in_expr(
                    &if_expr.cond,
                    candidates,
                    hints,
                );
                self.with_pre_scan_known_local_scope(&if_expr.then_branch.stmts, |this| {
                    for stmt in &if_expr.then_branch.stmts {
                        this.collect_uninitialized_local_type_hints_in_stmt(
                            stmt, candidates, hints,
                        );
                    }
                });
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    self.collect_uninitialized_local_type_hints_in_expr(
                        else_expr, candidates, hints,
                    );
                }
            }
            syn::Expr::While(while_expr) => {
                self.collect_uninitialized_local_type_hints_in_expr(
                    &while_expr.cond,
                    candidates,
                    hints,
                );
                self.with_pre_scan_known_local_scope(&while_expr.body.stmts, |this| {
                    for stmt in &while_expr.body.stmts {
                        this.collect_uninitialized_local_type_hints_in_stmt(
                            stmt, candidates, hints,
                        );
                    }
                });
            }
            syn::Expr::Loop(loop_expr) => {
                self.with_pre_scan_known_local_scope(&loop_expr.body.stmts, |this| {
                    for stmt in &loop_expr.body.stmts {
                        this.collect_uninitialized_local_type_hints_in_stmt(
                            stmt, candidates, hints,
                        );
                    }
                });
            }
            syn::Expr::ForLoop(for_expr) => {
                self.collect_uninitialized_local_type_hints_in_expr(
                    &for_expr.expr,
                    candidates,
                    hints,
                );
                self.with_pre_scan_known_local_scope(&for_expr.body.stmts, |this| {
                    for stmt in &for_expr.body.stmts {
                        this.collect_uninitialized_local_type_hints_in_stmt(
                            stmt, candidates, hints,
                        );
                    }
                });
            }
            syn::Expr::Match(match_expr) => {
                self.collect_uninitialized_local_type_hints_in_expr(
                    &match_expr.expr,
                    candidates,
                    hints,
                );
                for arm in &match_expr.arms {
                    if let Some((_, guard)) = &arm.guard {
                        self.collect_uninitialized_local_type_hints_in_expr(
                            guard, candidates, hints,
                        );
                    }
                    self.collect_uninitialized_local_type_hints_in_expr(
                        &arm.body, candidates, hints,
                    );
                }
            }
            syn::Expr::Struct(struct_expr) => {
                for field in &struct_expr.fields {
                    self.collect_uninitialized_local_type_hints_in_expr(
                        &field.expr,
                        candidates,
                        hints,
                    );
                }
                if let Some(rest) = &struct_expr.rest {
                    self.collect_uninitialized_local_type_hints_in_expr(rest, candidates, hints);
                }
            }
            syn::Expr::Array(array_expr) => {
                for elem in &array_expr.elems {
                    self.collect_uninitialized_local_type_hints_in_expr(elem, candidates, hints);
                }
            }
            syn::Expr::Tuple(tuple_expr) => {
                self.collect_uninitialized_local_type_hints_from_tuple(
                    tuple_expr, candidates, hints,
                );
                for elem in &tuple_expr.elems {
                    self.collect_uninitialized_local_type_hints_in_expr(elem, candidates, hints);
                }
            }
            syn::Expr::Index(index_expr) => {
                self.collect_uninitialized_local_type_hints_in_expr(
                    &index_expr.expr,
                    candidates,
                    hints,
                );
                self.collect_uninitialized_local_type_hints_in_expr(
                    &index_expr.index,
                    candidates,
                    hints,
                );
            }
            syn::Expr::Unsafe(unsafe_expr) => {
                self.with_pre_scan_known_local_scope(&unsafe_expr.block.stmts, |this| {
                    for stmt in &unsafe_expr.block.stmts {
                        this.collect_uninitialized_local_type_hints_in_stmt(
                            stmt, candidates, hints,
                        );
                    }
                });
            }
            syn::Expr::Closure(closure_expr) => {
                self.collect_uninitialized_local_type_hints_in_expr(
                    &closure_expr.body,
                    candidates,
                    hints,
                );
            }
            syn::Expr::Await(await_expr) => {
                self.collect_uninitialized_local_type_hints_in_expr(
                    &await_expr.base,
                    candidates,
                    hints,
                );
            }
            syn::Expr::Try(try_expr) => {
                self.collect_uninitialized_local_type_hints_in_expr(
                    &try_expr.expr,
                    candidates,
                    hints,
                );
            }
            syn::Expr::Return(return_expr) => {
                if let Some(value) = &return_expr.expr {
                    self.collect_uninitialized_local_type_hints_in_expr(value, candidates, hints);
                }
            }
            syn::Expr::Break(break_expr) => {
                if let Some(value) = &break_expr.expr {
                    self.collect_uninitialized_local_type_hints_in_expr(value, candidates, hints);
                }
            }
            syn::Expr::Reference(reference_expr) => {
                self.collect_uninitialized_local_type_hints_in_expr(
                    &reference_expr.expr,
                    candidates,
                    hints,
                );
            }
            syn::Expr::Unary(unary_expr) => {
                self.collect_uninitialized_local_type_hints_in_expr(
                    &unary_expr.expr,
                    candidates,
                    hints,
                );
            }
            syn::Expr::Paren(paren_expr) => {
                self.collect_uninitialized_local_type_hints_in_expr(
                    &paren_expr.expr,
                    candidates,
                    hints,
                );
            }
            syn::Expr::Group(group_expr) => {
                self.collect_uninitialized_local_type_hints_in_expr(
                    &group_expr.expr,
                    candidates,
                    hints,
                );
            }
            syn::Expr::Let(let_expr) => {
                self.collect_uninitialized_local_type_hints_in_expr(
                    &let_expr.expr,
                    candidates,
                    hints,
                );
            }
            _ => {}
        }
    }

    pub(super) fn collect_pre_scan_known_local_type_hints(
        &self,
        stmts: &[syn::Stmt],
    ) -> HashMap<String, syn::Type> {
        let mut known = collect_known_local_type_hints(stmts);
        let mut rolling = self.new_inner_for_block();
        rolling.local_bindings.push(
            known
                .iter()
                .map(|(name, ty)| (name.clone(), Some(ty.clone())))
                .collect(),
        );
        for stmt in stmts {
            let syn::Stmt::Local(local) = stmt else {
                continue;
            };
            let explicit_inferred = if let Some(explicit_ty) = get_local_type(local)
                && !type_has_generic_placeholder(explicit_ty)
                && self.type_is_placeholder_hint_candidate_allow_scoped_generics(explicit_ty)
            {
                Some(explicit_ty.clone())
            } else {
                None
            };
            let inferred = explicit_inferred.or_else(|| {
                let init = local.init.as_ref()?;
                if expr_is_option_none_constructor(&init.expr) {
                    return None;
                }
                rolling
                    .infer_local_binding_type_from_initializer(&init.expr)
                    .or_else(|| rolling.infer_simple_expr_type(&init.expr))
                    .or_else(|| rolling.infer_hint_type_from_expr(&init.expr))
            });
            if let Some(inferred) = inferred
                && self.type_is_placeholder_hint_candidate_allow_scoped_generics(&inferred)
            {
                if std::env::var_os("RUSTY_CPP_DEBUG_PRE_SCAN_KNOWN").is_some() {
                    let pat = &local.pat;
                    eprintln!(
                        "[rusty-cpp][pre-scan-known] pat={} inferred={}",
                        quote::quote!(#pat),
                        quote::quote!(#inferred)
                    );
                }
                let mut bindings = HashMap::new();
                self.bind_pattern_types_into_env(&local.pat, &inferred, &mut bindings);
                if bindings.is_empty()
                    && let Some(name) = local_binding_name(local)
                {
                    bindings.insert(name, inferred);
                }
                if let Some(scope) = rolling.local_bindings.last_mut() {
                    for (name, ty) in bindings {
                        if self.type_is_placeholder_hint_candidate_allow_scoped_generics(&ty) {
                            if std::env::var_os("RUSTY_CPP_DEBUG_PRE_SCAN_KNOWN").is_some() {
                                eprintln!(
                                    "[rusty-cpp][pre-scan-known] bind {}={}",
                                    name,
                                    quote::quote!(#ty)
                                );
                            }
                            known.insert(name.clone(), ty.clone());
                            scope.insert(name, Some(ty));
                        }
                    }
                }
            }
        }
        known
    }

    pub(super) fn collect_type_param_names_in_type(&self, ty: &syn::Type, out: &mut HashSet<String>) {
        let ty = self.peel_reference_paren_group_type(ty);
        match ty {
            syn::Type::Path(tp) => {
                if tp.qself.is_none() {
                    for seg in &tp.path.segments {
                        let name = seg.ident.to_string();
                        if self.is_type_param_in_scope(&name) || self.is_struct_type_param(&name) {
                            out.insert(name);
                        }
                    }
                }
                for seg in &tp.path.segments {
                    match &seg.arguments {
                        syn::PathArguments::AngleBracketed(args) => {
                            for arg in &args.args {
                                if let syn::GenericArgument::Type(inner) = arg {
                                    self.collect_type_param_names_in_type(inner, out);
                                }
                            }
                        }
                        syn::PathArguments::Parenthesized(args) => {
                            for input in &args.inputs {
                                self.collect_type_param_names_in_type(input, out);
                            }
                            if let syn::ReturnType::Type(_, output) = &args.output {
                                self.collect_type_param_names_in_type(output, out);
                            }
                        }
                        syn::PathArguments::None => {}
                    }
                }
            }
            syn::Type::Reference(r) => self.collect_type_param_names_in_type(&r.elem, out),
            syn::Type::Ptr(p) => self.collect_type_param_names_in_type(&p.elem, out),
            syn::Type::Paren(p) => self.collect_type_param_names_in_type(&p.elem, out),
            syn::Type::Group(g) => self.collect_type_param_names_in_type(&g.elem, out),
            syn::Type::Tuple(t) => {
                for elem in &t.elems {
                    self.collect_type_param_names_in_type(elem, out);
                }
            }
            syn::Type::Array(a) => self.collect_type_param_names_in_type(&a.elem, out),
            syn::Type::Slice(s) => self.collect_type_param_names_in_type(&s.elem, out),
            _ => {}
        }
    }

    pub(super) fn collect_unscoped_placeholder_type_idents_in_type(
        &self,
        ty: &syn::Type,
        out: &mut std::collections::BTreeSet<String>,
    ) {
        let ty = self.peel_reference_paren_group_type(ty);
        match ty {
            syn::Type::Path(tp) => {
                if tp.qself.is_none()
                    && tp.path.segments.len() == 1
                    && let Some(seg) = tp.path.segments.first()
                    && matches!(seg.arguments, syn::PathArguments::None)
                {
                    let name = seg.ident.to_string();
                    if name.starts_with("__")
                        && !self.is_type_param_in_scope(&name)
                        && !self.is_struct_type_param(&name)
                    {
                        out.insert(name);
                    }
                }
                for seg in &tp.path.segments {
                    match &seg.arguments {
                        syn::PathArguments::AngleBracketed(args) => {
                            for arg in &args.args {
                                if let syn::GenericArgument::Type(inner) = arg {
                                    self.collect_unscoped_placeholder_type_idents_in_type(
                                        inner, out,
                                    );
                                }
                            }
                        }
                        syn::PathArguments::Parenthesized(args) => {
                            for input in &args.inputs {
                                self.collect_unscoped_placeholder_type_idents_in_type(input, out);
                            }
                            if let syn::ReturnType::Type(_, output) = &args.output {
                                self.collect_unscoped_placeholder_type_idents_in_type(output, out);
                            }
                        }
                        syn::PathArguments::None => {}
                    }
                }
            }
            syn::Type::Reference(r) => {
                self.collect_unscoped_placeholder_type_idents_in_type(&r.elem, out)
            }
            syn::Type::Ptr(p) => {
                self.collect_unscoped_placeholder_type_idents_in_type(&p.elem, out)
            }
            syn::Type::Paren(p) => {
                self.collect_unscoped_placeholder_type_idents_in_type(&p.elem, out)
            }
            syn::Type::Group(g) => {
                self.collect_unscoped_placeholder_type_idents_in_type(&g.elem, out)
            }
            syn::Type::Tuple(t) => {
                for elem in &t.elems {
                    self.collect_unscoped_placeholder_type_idents_in_type(elem, out);
                }
            }
            syn::Type::Array(a) => {
                self.collect_unscoped_placeholder_type_idents_in_type(&a.elem, out)
            }
            syn::Type::Slice(s) => {
                self.collect_unscoped_placeholder_type_idents_in_type(&s.elem, out)
            }
            _ => {}
        }
    }

    pub(super) fn collect_uninitialized_local_type_hints_from_tuple(
        &self,
        tuple_expr: &syn::ExprTuple,
        candidates: &HashSet<String>,
        hints: &mut HashMap<String, syn::Type>,
    ) {
        if tuple_expr.elems.len() < 2 {
            return;
        }

        for (idx, elem) in tuple_expr.elems.iter().enumerate() {
            let Some(name) = self.extract_candidate_local_name_for_hint_expr(elem) else {
                continue;
            };
            if !candidates.contains(&name) || hints.contains_key(&name) {
                continue;
            }
            let mut peer_hint = None;
            for (peer_idx, peer_expr) in tuple_expr.elems.iter().enumerate() {
                if idx == peer_idx {
                    continue;
                }
                if let Some(inferred) = self.infer_concrete_hint_type_from_peer_expr(peer_expr) {
                    peer_hint = Some(inferred);
                    break;
                }
            }
            if let Some(inferred) = peer_hint {
                hints.insert(name, inferred);
            }
        }
    }

    /// A hint harvested from a `&v` / `&mut v` argument carries the
    /// parameter's FULL type including the reference — but the local `v`
    /// itself has the pointee type (`bump(&mut v)` on `let mut v = 1;` was
    /// declaring `int32_t& v = <prvalue>`, ill-formed). Peel one
    /// `Type::Reference` layer per `Expr::Reference` layer on the arg.
    fn peel_hint_reference_layers_for_arg(arg: &syn::Expr, mut ty: syn::Type) -> syn::Type {
        let mut cur = arg;
        loop {
            cur = match cur {
                syn::Expr::Paren(p) => &p.expr,
                syn::Expr::Group(g) => &g.expr,
                syn::Expr::Reference(r) => {
                    if let syn::Type::Reference(tr) = ty {
                        ty = (*tr.elem).clone();
                    }
                    &r.expr
                }
                _ => break,
            };
        }
        ty
    }

    pub(super) fn collect_uninitialized_local_type_hints_from_call(
        &self,
        call: &syn::ExprCall,
        candidates: &HashSet<String>,
        hints: &mut HashMap<String, syn::Type>,
    ) {
        let substitutions = self.function_call_type_arg_substitutions(call);
        for (idx, arg) in call.args.iter().enumerate() {
            let Some(name) = extract_value_consumed_local_ident(arg) else {
                continue;
            };
            if !candidates.contains(&name) {
                continue;
            }
            let expected_ty = self
                .lookup_function_arg_expected_type_for_call(call, idx, substitutions.as_ref())
                .or_else(|| {
                    self.lookup_associated_call_arg_expected_type_fallback(call, idx, Some(arg))
                });
            let Some(expected_ty) = expected_ty else {
                continue;
            };
            let expected_ty = Self::peel_hint_reference_layers_for_arg(arg, expected_ty);
            if !self.type_is_concrete_hint_candidate(&expected_ty) {
                continue;
            }
            hints.insert(name, expected_ty);
        }
    }

    pub(super) fn collect_uninitialized_local_type_hints_from_method_call(
        &self,
        method_call: &syn::ExprMethodCall,
        candidates: &HashSet<String>,
        hints: &mut HashMap<String, syn::Type>,
    ) {
        let method_name = method_call.method.to_string();

        // Phase 1: check if any argument is a candidate (existing logic)
        for (idx, arg) in method_call.args.iter().enumerate() {
            let Some(name) = extract_value_consumed_local_ident(arg) else {
                continue;
            };
            if !candidates.contains(&name) {
                continue;
            }
            let declared_expected = self.lookup_method_arg_expected_type(&method_name, idx);
            let expected_ty = self
                .infer_method_arg_expected_type_from_receiver(
                    &method_call.receiver,
                    &method_name,
                    idx,
                    declared_expected,
                    Some(arg),
                )
                .or_else(|| declared_expected.cloned());
            let Some(expected_ty) = expected_ty else {
                continue;
            };
            let expected_ty = Self::peel_hint_reference_layers_for_arg(arg, expected_ty);
            if !self.type_is_concrete_hint_candidate(&expected_ty) {
                continue;
            }
            hints.insert(name, expected_ty);
        }

        // Phase 1b: `&mut candidate` against a known std trait signature.
        // io::Read's read_to_end/read_to_string are declared by no crate, so
        // the declared tables can't type `rdr.read_to_end(&mut buffer)`'s
        // buffer — without this, loader-style `let mut buffer = Vec::new()`
        // leaks `rusty::Vec<auto>`.
        for (idx, arg) in method_call.args.iter().enumerate() {
            let syn::Expr::Reference(r) = arg else {
                continue;
            };
            let Some(name) = extract_simple_local_ident(&r.expr) else {
                continue;
            };
            if !candidates.contains(&name) || hints.contains_key(&name) {
                continue;
            }
            let Some(expected) = builtin_std_method_arg_expected_type(&method_name, idx)
            else {
                continue;
            };
            let peeled = match &expected {
                syn::Type::Reference(tr) => (*tr.elem).clone(),
                other => other.clone(),
            };
            if self.type_is_concrete_hint_candidate(&peeled) {
                hints.insert(name, peeled);
            }
        }

        // Phase 2: check if the receiver is a candidate and infer owner type
        // from the method call.  For example, `cell.set(42)` where `cell` is
        // a candidate initialized with `OnceCell::new()` → infer `OnceCell<i32>`.
        if let Some(receiver_name) = extract_simple_local_ident(&method_call.receiver) {
            if candidates.contains(&receiver_name) && !hints.contains_key(&receiver_name) {
                if let Some(owner_ty) = self.infer_owner_type_from_method_usage(
                    &receiver_name,
                    &method_name,
                    &method_call.args,
                ) {
                    if self.type_is_concrete_hint_candidate(&owner_ty) {
                        hints.insert(receiver_name, owner_ty);
                    }
                }
            }
        }
    }

    pub(super) fn collect_consuming_method_receiver_vars_with_signature_hints(
        &self,
        stmts: &[syn::Stmt],
    ) -> HashSet<String> {
        let mut result = collect_consuming_method_receiver_vars(stmts);
        for stmt in stmts {
            self.collect_value_call_argument_locals_in_stmt(stmt, &mut result);
        }
        // `place = local;` consumes the local (Rust moves assignment RHS
        // places). A `const auto` binding would turn that emitted
        // `std::move(local)` into a copy — deleted for variants holding
        // move-only alternatives (`this->state = std::move(state)`).
        struct AssignRhsLocals<'a> {
            result: &'a mut HashSet<String>,
        }
        impl<'ast> Visit<'ast> for AssignRhsLocals<'_> {
            fn visit_expr_assign(&mut self, assign: &'ast syn::ExprAssign) {
                if let syn::Expr::Path(path) = assign.right.as_ref()
                    && path.qself.is_none()
                    && path.path.segments.len() == 1
                {
                    self.result
                        .insert(path.path.segments[0].ident.to_string());
                }
                visit::visit_expr_assign(self, assign);
            }
        }
        let mut visitor = AssignRhsLocals {
            result: &mut result,
        };
        for stmt in stmts {
            visitor.visit_stmt(stmt);
        }
        // `S { f: …, ..base }` moves the remaining fields out of `base` — a
        // `const auto` binding would turn the emitted `std::move(base.field)`
        // pulls into deleted copies for move-only field types.
        struct StructUpdateRestLocals<'a> {
            result: &'a mut HashSet<String>,
        }
        impl<'ast> Visit<'ast> for StructUpdateRestLocals<'_> {
            fn visit_expr_struct(&mut self, st: &'ast syn::ExprStruct) {
                if let Some(rest) = st.rest.as_deref() {
                    let mut root = rest;
                    loop {
                        match root {
                            syn::Expr::Field(f) => root = &f.base,
                            syn::Expr::Paren(p) => root = &p.expr,
                            _ => break,
                        }
                    }
                    if let syn::Expr::Path(path) = root
                        && path.qself.is_none()
                        && path.path.segments.len() == 1
                    {
                        self.result
                            .insert(path.path.segments[0].ident.to_string());
                    }
                }
                visit::visit_expr_struct(self, st);
            }
        }
        let mut rest_visitor = StructUpdateRestLocals {
            result: &mut result,
        };
        for stmt in stmts {
            rest_visitor.visit_stmt(stmt);
        }
        // A `move` closure init-captures its free variables by value via
        // `x = std::move(x)` — locals it references must not bind const, or
        // the move decays to a (possibly deleted) copy.
        struct MoveClosureCaptures<'a> {
            result: &'a mut HashSet<String>,
        }
        impl<'ast> Visit<'ast> for MoveClosureCaptures<'_> {
            fn visit_expr_closure(&mut self, c: &'ast syn::ExprClosure) {
                if c.capture.is_some() {
                    struct Idents<'a> {
                        result: &'a mut HashSet<String>,
                    }
                    impl<'ast> Visit<'ast> for Idents<'_> {
                        fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
                            if p.qself.is_none() && p.path.segments.len() == 1 {
                                self.result
                                    .insert(p.path.segments[0].ident.to_string());
                            }
                            visit::visit_expr_path(self, p);
                        }
                    }
                    let mut idents = Idents {
                        result: self.result,
                    };
                    idents.visit_expr(&c.body);
                }
                visit::visit_expr_closure(self, c);
            }
        }
        let mut capture_visitor = MoveClosureCaptures {
            result: &mut result,
        };
        for stmt in stmts {
            capture_visitor.visit_stmt(stmt);
        }
        // `if let Some(v) = local` (and Ok/Err) MOVES `local` in Rust: the
        // payload is consumed into `v` and dropped at the body's end. The
        // emitted lowering uses the destructive lvalue `unwrap()` — which
        // needs a NON-const local; a `const auto local` binding silently
        // degrades to the borrowing const overload, keeping the payload
        // alive past the if-let (observable drop-timing divergence:
        // Rc::strong_count read 2 where Rust reads 1). Precise AST shape
        // only — a `&`-borrowed scrutinee or a `ref` binding is skipped.
        struct ConsumingIfLetScrutinees<'a> {
            result: &'a mut HashSet<String>,
        }
        impl<'ast> Visit<'ast> for ConsumingIfLetScrutinees<'_> {
            fn visit_expr_if(&mut self, if_expr: &'ast syn::ExprIf) {
                if let syn::Expr::Let(let_expr) = if_expr.cond.as_ref() {
                    let by_value_binding = match let_expr.pat.as_ref() {
                        syn::Pat::TupleStruct(ts) => {
                            ts.path.segments.last().is_some_and(|s| {
                                matches!(
                                    s.ident.to_string().as_str(),
                                    "Some" | "Ok" | "Err"
                                )
                            }) && ts.elems.iter().all(|elem| {
                                !matches!(elem, syn::Pat::Ident(pi) if pi.by_ref.is_some())
                            })
                        }
                        _ => false,
                    };
                    if by_value_binding {
                        if let syn::Expr::Path(p) = let_expr.expr.as_ref() {
                            if p.qself.is_none()
                                && p.path.segments.len() == 1
                                && p.path.segments[0].ident != "self"
                            {
                                self.result
                                    .insert(p.path.segments[0].ident.to_string());
                            }
                        }
                    }
                }
                visit::visit_expr_if(self, if_expr);
            }
        }
        let mut iflet_visitor = ConsumingIfLetScrutinees {
            result: &mut result,
        };
        for stmt in stmts {
            iflet_visitor.visit_stmt(stmt);
        }
        result
    }

    /// Bare single-segment locals iterated by a `for` loop (`for x in v`).
    /// Rust moves the iterable; the emit-side qualifier decision combines
    /// this NAME set with a crate-Iterator/IntoIterator TYPE gate so plain
    /// slice/Vec loops keep their existing const emission.
    pub(super) fn collect_for_loop_iterated_bare_locals(stmts: &[syn::Stmt]) -> HashSet<String> {
        struct ForIterables {
            result: HashSet<String>,
        }
        impl<'ast> Visit<'ast> for ForIterables {
            fn visit_expr_for_loop(&mut self, fl: &'ast syn::ExprForLoop) {
                if let syn::Expr::Path(p) = fl.expr.as_ref() {
                    if p.qself.is_none()
                        && p.path.segments.len() == 1
                        && p.path.segments[0].ident != "self"
                    {
                        self.result.insert(p.path.segments[0].ident.to_string());
                    }
                }
                visit::visit_expr_for_loop(self, fl);
            }
        }
        let mut visitor = ForIterables {
            result: HashSet::new(),
        };
        for stmt in stmts {
            visitor.visit_stmt(stmt);
        }
        visitor.result
    }

    pub(super) fn collect_value_call_argument_locals_in_stmt(
        &self,
        stmt: &syn::Stmt,
        result: &mut HashSet<String>,
    ) {
        match stmt {
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.collect_value_call_argument_locals_in_expr(&init.expr, result);
                }
            }
            syn::Stmt::Expr(expr, _) => {
                self.collect_value_call_argument_locals_in_expr(expr, result);
            }
            syn::Stmt::Macro(sm) => {
                self.collect_value_call_argument_locals_in_macro(&sm.mac, result);
            }
            syn::Stmt::Item(_) => {}
        }
    }

    /// Call arguments inside format-like macro token streams (`println!("{}",
    /// consume(x))`) are invisible to the AST walkers above — parse each
    /// top-level macro arg back to an expression so by-value consumption of
    /// locals is still detected.
    pub(super) fn collect_value_call_argument_locals_in_macro(
        &self,
        mac: &syn::Macro,
        result: &mut HashSet<String>,
    ) {
        let name = mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        if !matches!(
            name.as_str(),
            "println"
                | "print"
                | "eprintln"
                | "eprint"
                | "format"
                | "format_args"
                | "write"
                | "writeln"
                | "panic"
                | "assert"
                | "debug_assert"
                | "assert_eq"
                | "assert_ne"
                | "debug_assert_eq"
                | "debug_assert_ne"
                | "vec"
                | "matches"
                | "dbg"
                | "unreachable"
                | "todo"
        ) {
            return;
        }
        for part in self.split_macro_args(&mac.tokens.to_string()) {
            if let Ok(expr) = syn::parse_str::<syn::Expr>(part.trim()) {
                self.collect_value_call_argument_locals_in_expr(&expr, result);
            }
        }
    }

    pub(super) fn collect_value_call_argument_locals_in_expr(
        &self,
        expr: &syn::Expr,
        result: &mut HashSet<String>,
    ) {
        let expr = self.peel_paren_group_expr(expr);
        match expr {
            syn::Expr::Call(call) => {
                self.mark_value_call_argument_local_bindings(call, result);
                self.collect_value_call_argument_locals_in_expr(call.func.as_ref(), result);
                for arg in &call.args {
                    self.collect_value_call_argument_locals_in_expr(arg, result);
                }
            }
            syn::Expr::MethodCall(method_call) => {
                let method_name = method_call.method.to_string();
                for (idx, arg) in method_call.args.iter().enumerate() {
                    let style = self
                        .lookup_method_arg_pass_style(&method_name, idx)
                        .or_else(|| {
                            self.lookup_method_arg_expected_type_from_receiver_owner(
                                &method_call.receiver,
                                &method_name,
                                idx,
                                Some(arg),
                            )
                            .map(|ty| self.arg_pass_style_for_type(&ty))
                        })
                        .or_else(|| {
                            self.lookup_method_arg_expected_type(&method_name, idx)
                                .map(|ty| self.arg_pass_style_for_type(ty))
                        });
                    let heuristic_value = matches!(
                        (method_name.as_str(), idx),
                        ("set", 0)
                            | ("push", 0)
                            | ("try_push", 0)
                            | ("insert", 1)
                            | ("try_insert", 1)
                            | ("init", 0)
                            | ("get_or_init", 0)
                            | ("get_or_try_init", 0)
                            | ("new", 0)
                            | ("new_", 0)
                            // Iterator adapters consume their iterator arg
                            // by value (`diff1.chain(diff2)` — a const diff2
                            // can't move and its non-const next() is
                            // unreachable through the const carrier).
                            | ("chain", 0)
                            | ("zip", 0)
                    );
                    if (matches!(style, Some(ArgPassStyle::Value))
                        || (matches!(style, None | Some(ArgPassStyle::Mixed)) && heuristic_value))
                        && let Some(name) = extract_value_consumed_local_ident(arg)
                    {
                        result.insert(name);
                    }
                }
                self.collect_value_call_argument_locals_in_expr(&method_call.receiver, result);
                for arg in &method_call.args {
                    self.collect_value_call_argument_locals_in_expr(arg, result);
                }
            }
            syn::Expr::Binary(bin) => {
                self.collect_value_call_argument_locals_in_expr(&bin.left, result);
                self.collect_value_call_argument_locals_in_expr(&bin.right, result);
            }
            syn::Expr::Unary(unary) => {
                self.collect_value_call_argument_locals_in_expr(&unary.expr, result)
            }
            syn::Expr::Reference(reference) => {
                self.collect_value_call_argument_locals_in_expr(&reference.expr, result)
            }
            syn::Expr::Assign(assign) => {
                self.collect_value_call_argument_locals_in_expr(&assign.left, result);
                self.collect_value_call_argument_locals_in_expr(&assign.right, result);
            }
            syn::Expr::Block(block) => {
                for stmt in &block.block.stmts {
                    self.collect_value_call_argument_locals_in_stmt(stmt, result);
                }
            }
            syn::Expr::If(if_expr) => {
                self.collect_value_call_argument_locals_in_expr(&if_expr.cond, result);
                for stmt in &if_expr.then_branch.stmts {
                    self.collect_value_call_argument_locals_in_stmt(stmt, result);
                }
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    self.collect_value_call_argument_locals_in_expr(else_expr, result);
                }
            }
            syn::Expr::Match(match_expr) => {
                self.collect_value_call_argument_locals_in_expr(&match_expr.expr, result);
                for arm in &match_expr.arms {
                    if let Some((_, guard)) = &arm.guard {
                        self.collect_value_call_argument_locals_in_expr(guard, result);
                    }
                    self.collect_value_call_argument_locals_in_expr(&arm.body, result);
                }
            }
            syn::Expr::While(while_expr) => {
                self.collect_value_call_argument_locals_in_expr(&while_expr.cond, result);
                for stmt in &while_expr.body.stmts {
                    self.collect_value_call_argument_locals_in_stmt(stmt, result);
                }
            }
            syn::Expr::Loop(loop_expr) => {
                for stmt in &loop_expr.body.stmts {
                    self.collect_value_call_argument_locals_in_stmt(stmt, result);
                }
            }
            syn::Expr::ForLoop(for_loop) => {
                self.collect_value_call_argument_locals_in_expr(&for_loop.expr, result);
                for stmt in &for_loop.body.stmts {
                    self.collect_value_call_argument_locals_in_stmt(stmt, result);
                }
            }
            syn::Expr::Array(array) => {
                for elem in &array.elems {
                    self.collect_value_call_argument_locals_in_expr(elem, result);
                }
            }
            syn::Expr::Tuple(tuple) => {
                for elem in &tuple.elems {
                    self.collect_value_call_argument_locals_in_expr(elem, result);
                }
            }
            syn::Expr::Struct(struct_expr) => {
                for field in &struct_expr.fields {
                    self.collect_value_call_argument_locals_in_expr(&field.expr, result);
                }
                if let Some(rest) = &struct_expr.rest {
                    self.collect_value_call_argument_locals_in_expr(rest, result);
                }
            }
            syn::Expr::Field(field) => {
                self.collect_value_call_argument_locals_in_expr(&field.base, result)
            }
            syn::Expr::Index(index) => {
                self.collect_value_call_argument_locals_in_expr(&index.expr, result);
                self.collect_value_call_argument_locals_in_expr(&index.index, result);
            }
            syn::Expr::Cast(cast_expr) => {
                self.collect_value_call_argument_locals_in_expr(&cast_expr.expr, result)
            }
            syn::Expr::Await(await_expr) => {
                self.collect_value_call_argument_locals_in_expr(&await_expr.base, result)
            }
            syn::Expr::Try(try_expr) => {
                self.collect_value_call_argument_locals_in_expr(&try_expr.expr, result)
            }
            syn::Expr::Break(brk) => {
                if let Some(value) = &brk.expr {
                    self.collect_value_call_argument_locals_in_expr(value, result);
                }
            }
            syn::Expr::Return(ret) => {
                if let Some(value) = &ret.expr {
                    self.collect_value_call_argument_locals_in_expr(value, result);
                }
            }
            syn::Expr::Closure(closure) => {
                self.collect_value_call_argument_locals_in_expr(&closure.body, result)
            }
            syn::Expr::Macro(macro_expr) => {
                self.collect_value_call_argument_locals_in_macro(&macro_expr.mac, result)
            }
            syn::Expr::Let(let_expr) => {
                self.collect_value_call_argument_locals_in_expr(&let_expr.expr, result)
            }
            syn::Expr::Unsafe(unsafe_expr) => {
                for stmt in &unsafe_expr.block.stmts {
                    self.collect_value_call_argument_locals_in_stmt(stmt, result);
                }
            }
            _ => {}
        }
    }

    pub(super) fn collect_pattern_binding_stmts(
        &self,
        pat: &syn::Pat,
        source_expr: &str,
        out: &mut Vec<String>,
    ) -> bool {
        match pat {
            syn::Pat::Ident(pi) => {
                if pi.ident != "_" {
                    let rust_name = pi.ident.to_string();
                    if pi.by_ref.is_none()
                        && pi.mutability.is_none()
                        && pi.subpat.is_none()
                        && self.pattern_ident_is_const_value(&rust_name)
                    {
                        return true;
                    }
                    let cpp_name = escape_cpp_keyword(&rust_name);
                    let binding_prefix = if pi.by_ref.is_some() && pi.mutability.is_some() {
                        "auto&"
                    } else if pi.by_ref.is_some() {
                        "const auto&"
                    } else {
                        // By-value pattern bindings (both `x` and `mut x`):
                        // use `auto&&` to preserve reference payloads (e.g.,
                        // `R = T&`) without forcing `const`, AND to avoid the
                        // hidden copy when the value is move-only.
                        //
                        // `let mut x = expr` in Rust binds `x` mutably to the
                        // value. In C++ `auto x = expr` would COPY (or move
                        // for movable types). For move-only T where the only
                        // available ctor is the move ctor, an expression that
                        // produces an lvalue can't be auto-bound by-value
                        // without a copy ctor — fails to compile. `auto&&`
                        // collapses to `T&` for lvalues and `T&&` for rvalues,
                        // mutation still works in both cases.
                        //
                        // btree_port B4 surfacing: match arm `Occupied(mut
                        // entry) => Some(entry.insert(value))` emitted as
                        // `auto entry_shadow1 = ...` (copy required) on a
                        // `OccupiedEntry<K, std::pair<K, MoveOnlyV>>` instan-
                        // tiation. See
                        // tests/btree_port_iter_remove_movonly_test.cpp.
                        "auto&&"
                    };
                    // Subslice rest-bindings (`[head, rest @ ..]`) produce a
                    // rusty::slice(...) PRVALUE (a std::span view). Routing it
                    // through deref_if_pointer returns a reference INTO the
                    // full-expression temporary — `auto&&` does not lifetime-
                    // extend through the call, so the binding dangled (ASan
                    // stack-use-after-scope). Spans are trivially copyable
                    // views: bind by value.
                    if pi.by_ref.is_none() && source_expr.starts_with("rusty::slice(") {
                        out.push(format!("auto {} = {};", cpp_name, source_expr));
                    } else {
                        let binding_source = if pi.by_ref.is_none() {
                            format!("rusty::detail::deref_if_pointer({})", source_expr)
                        } else {
                            source_expr.to_string()
                        };
                        out.push(format!(
                            "{} {} = {};",
                            binding_prefix, cpp_name, binding_source
                        ));
                    }
                }
                if let Some((_, subpat)) = &pi.subpat {
                    let mut sub_bindings = Vec::new();
                    if self.collect_pattern_binding_stmts(subpat, source_expr, &mut sub_bindings) {
                        out.extend(sub_bindings);
                    }
                }
                true
            }
            syn::Pat::Wild(_) => true,
            syn::Pat::Tuple(tuple_pat) => {
                for (i, elem) in tuple_pat.elems.iter().enumerate() {
                    let elem_expr = format!(
                        "std::get<{}>(rusty::detail::deref_if_pointer({}))",
                        i, source_expr
                    );
                    if !self.collect_pattern_binding_stmts(elem, &elem_expr, out) {
                        return false;
                    }
                }
                true
            }
            syn::Pat::TupleStruct(tuple_struct_pat) => {
                let variant_name = tuple_struct_pat
                    .path
                    .segments
                    .last()
                    .map(|seg| seg.ident.to_string())
                    .unwrap_or_default();
                let owner_name = tuple_struct_pat
                    .path
                    .segments
                    .iter()
                    .nth_back(1)
                    .map(|seg| seg.ident.to_string())
                    .unwrap_or_default();
                if tuple_struct_pat.elems.len() == 1
                    && matches!(variant_name.as_str(), "Borrowed" | "Owned")
                    && self.owner_name_is_known_cow_like(&owner_name)
                {
                    let base_expr = format!("rusty::detail::deref_if_pointer({})", source_expr);
                    let inner_expr = if variant_name == "Borrowed" {
                        format!("rusty::to_string_view({})", base_expr)
                    } else {
                        format!("rusty::into_owned({})", base_expr)
                    };
                    if let Some(elem) = tuple_struct_pat.elems.first() {
                        return self.collect_pattern_binding_stmts(elem, &inner_expr, out);
                    }
                }
                if tuple_struct_pat.elems.len() == 1
                    && matches!(variant_name.as_str(), "Some" | "Ok" | "Err")
                {
                    let base_expr = format!("rusty::detail::deref_if_pointer({})", source_expr);
                    let inner_expr = if variant_name == "Err" {
                        format!("({}).unwrap_err()", base_expr)
                    } else {
                        format!("({}).unwrap()", base_expr)
                    };
                    if let Some(elem) = tuple_struct_pat.elems.first() {
                        return self.collect_pattern_binding_stmts(elem, &inner_expr, out);
                    }
                }
                let base_expr = format!("rusty::detail::deref_if_pointer({})", source_expr);
                for (i, elem) in tuple_struct_pat.elems.iter().enumerate() {
                    let field_expr = format!("{}._{}", base_expr, i);
                    if !self.collect_pattern_binding_stmts(elem, &field_expr, out) {
                        return false;
                    }
                }
                true
            }
            syn::Pat::Slice(slice_pat) => {
                let base_expr = format!("rusty::detail::deref_if_pointer({})", source_expr);
                let len_expr = format!("rusty::len({})", base_expr);
                let rest_pos = slice_pat
                    .elems
                    .iter()
                    .position(|elem| self.pat_is_slice_rest_like(elem));
                if let Some(rest_pos) = rest_pos {
                    let suffix_count = slice_pat.elems.len().saturating_sub(rest_pos + 1);
                    for (i, elem) in slice_pat.elems.iter().take(rest_pos).enumerate() {
                        let elem_expr = format!("{}[{}]", base_expr, i);
                        if !self.collect_pattern_binding_stmts(elem, &elem_expr, out) {
                            return false;
                        }
                    }
                    let rest_pat = match slice_pat.elems.iter().nth(rest_pos) {
                        Some(pat) => pat,
                        None => return false,
                    };
                    if !matches!(self.peel_pat_type_ref_paren(rest_pat), syn::Pat::Rest(_)) {
                        let rest_end = if suffix_count == 0 {
                            len_expr.clone()
                        } else {
                            format!("({} - {})", len_expr, suffix_count)
                        };
                        let rest_expr =
                            format!("rusty::slice({}, {}, {})", base_expr, rest_pos, rest_end);
                        if !self.collect_pattern_binding_stmts(rest_pat, &rest_expr, out) {
                            return false;
                        }
                    }
                    for (offset, elem) in slice_pat.elems.iter().skip(rest_pos + 1).enumerate() {
                        let idx_expr = format!("({} - {})", len_expr, suffix_count - offset);
                        let elem_expr = format!("{}[{}]", base_expr, idx_expr);
                        if !self.collect_pattern_binding_stmts(elem, &elem_expr, out) {
                            return false;
                        }
                    }
                } else {
                    for (i, elem) in slice_pat.elems.iter().enumerate() {
                        let elem_expr = format!("{}[{}]", base_expr, i);
                        if !self.collect_pattern_binding_stmts(elem, &elem_expr, out) {
                            return false;
                        }
                    }
                }
                true
            }
            syn::Pat::Struct(struct_pat) => {
                for field_pat in &struct_pat.fields {
                    let field_name = match &field_pat.member {
                        syn::Member::Named(ident) => ident.to_string(),
                        syn::Member::Unnamed(index) => format!("_{}", index.index),
                    };
                    let emitted_field_name = match &field_pat.member {
                        syn::Member::Named(_) => self.resolve_struct_pattern_field_cpp_name(
                            &struct_pat.path,
                            &field_name,
                            None,
                        ),
                        syn::Member::Unnamed(_) => field_name.clone(),
                    };
                    let field_expr = format!("{}.{}", source_expr, emitted_field_name);
                    if !self.collect_pattern_binding_stmts(&field_pat.pat, &field_expr, out) {
                        return false;
                    }
                }
                true
            }
            syn::Pat::Type(pt) => self.collect_pattern_binding_stmts(&pt.pat, source_expr, out),
            syn::Pat::Reference(r) => {
                let deref_expr = format!("rusty::detail::deref_if_pointer({})", source_expr);
                self.collect_pattern_binding_stmts(&r.pat, &deref_expr, out)
            }
            syn::Pat::Paren(p) => self.collect_pattern_binding_stmts(&p.pat, source_expr, out),
            _ => false,
        }
    }

    pub(super) fn collect_runtime_match_binding_stmts_and_condition(
        &self,
        pat: &syn::Pat,
        source_expr: &str,
        out: &mut Vec<String>,
        variant_ctx: Option<&VariantTypeContext>,
    ) -> Option<Option<String>> {
        match pat {
            syn::Pat::Wild(_) => {
                if self.collect_pattern_binding_stmts(pat, source_expr, out) {
                    Some(None)
                } else {
                    None
                }
            }
            syn::Pat::Ident(pi) => {
                if let Some((_, subpat)) = &pi.subpat {
                    let mut ident_only = pi.clone();
                    ident_only.subpat = None;
                    let ident_pat = syn::Pat::Ident(ident_only);
                    if !self.collect_pattern_binding_stmts(&ident_pat, source_expr, out) {
                        return None;
                    }
                    if matches!(self.peel_pat_type_ref_paren(subpat), syn::Pat::Rest(_)) {
                        return Some(None);
                    }
                    self.collect_runtime_match_binding_stmts_and_condition(
                        subpat,
                        source_expr,
                        out,
                        variant_ctx,
                    )
                } else if let Some(cond_method) =
                    self.runtime_ident_match_condition_method(pi, variant_ctx)
                {
                    Some(Some(format!(
                        "rusty::detail::deref_if_pointer({}).{}()",
                        source_expr, cond_method
                    )))
                } else if pi.by_ref.is_none() && pi.mutability.is_none() && pi.subpat.is_none() {
                    let ident_name = pi.ident.to_string();
                    if self.pattern_ident_is_const_value(&ident_name) {
                        Some(Some(self.const_value_ident_pattern_condition(
                            &pi.ident,
                            source_expr,
                            variant_ctx,
                        )))
                    } else if self.collect_pattern_binding_stmts(pat, source_expr, out) {
                        Some(None)
                    } else {
                        None
                    }
                } else if self.collect_pattern_binding_stmts(pat, source_expr, out) {
                    Some(None)
                } else {
                    None
                }
            }
            syn::Pat::Tuple(tuple_pat) => {
                let mut conditions = Vec::new();
                for (i, elem_pat) in tuple_pat.elems.iter().enumerate() {
                    let elem_expr = format!(
                        "std::get<{}>(rusty::detail::deref_if_pointer({}))",
                        i, source_expr
                    );
                    let elem_condition = self.collect_runtime_match_binding_stmts_and_condition(
                        elem_pat,
                        &elem_expr,
                        out,
                        variant_ctx,
                    )?;
                    if let Some(cond) = elem_condition {
                        conditions.push(cond);
                    }
                }
                Some(Self::combine_runtime_match_conditions(conditions))
            }
            syn::Pat::Slice(slice_pat) => {
                let mut conditions = Vec::new();
                let base_expr = format!("rusty::detail::deref_if_pointer({})", source_expr);
                let len_expr = format!("rusty::len({})", base_expr);
                let rest_pos = slice_pat
                    .elems
                    .iter()
                    .position(|elem| self.pat_is_slice_rest_like(elem));
                if let Some(rest_pos) = rest_pos {
                    let suffix_count = slice_pat.elems.len().saturating_sub(rest_pos + 1);
                    conditions.push(format!("{} >= {}", len_expr, rest_pos + suffix_count));
                    for (i, elem_pat) in slice_pat.elems.iter().take(rest_pos).enumerate() {
                        let elem_expr = format!("{}[{}]", base_expr, i);
                        let elem_condition = self
                            .collect_runtime_match_binding_stmts_and_condition(
                                elem_pat,
                                &elem_expr,
                                out,
                                variant_ctx,
                            )?;
                        if let Some(cond) = elem_condition {
                            conditions.push(cond);
                        }
                    }
                    let rest_pat = slice_pat.elems.iter().nth(rest_pos)?;
                    if !matches!(self.peel_pat_type_ref_paren(rest_pat), syn::Pat::Rest(_)) {
                        let rest_end = if suffix_count == 0 {
                            len_expr.clone()
                        } else {
                            format!("({} - {})", len_expr, suffix_count)
                        };
                        let rest_expr =
                            format!("rusty::slice({}, {}, {})", base_expr, rest_pos, rest_end);
                        let rest_condition = self
                            .collect_runtime_match_binding_stmts_and_condition(
                                rest_pat,
                                &rest_expr,
                                out,
                                variant_ctx,
                            )?;
                        if let Some(cond) = rest_condition {
                            conditions.push(cond);
                        }
                    }
                    for (offset, elem_pat) in slice_pat.elems.iter().skip(rest_pos + 1).enumerate()
                    {
                        let idx_expr = format!("({} - {})", len_expr, suffix_count - offset);
                        let elem_expr = format!("{}[{}]", base_expr, idx_expr);
                        let elem_condition = self
                            .collect_runtime_match_binding_stmts_and_condition(
                                elem_pat,
                                &elem_expr,
                                out,
                                variant_ctx,
                            )?;
                        if let Some(cond) = elem_condition {
                            conditions.push(cond);
                        }
                    }
                } else {
                    conditions.push(format!("{} == {}", len_expr, slice_pat.elems.len()));
                    for (i, elem_pat) in slice_pat.elems.iter().enumerate() {
                        let elem_expr = format!("{}[{}]", base_expr, i);
                        let elem_condition = self
                            .collect_runtime_match_binding_stmts_and_condition(
                                elem_pat,
                                &elem_expr,
                                out,
                                variant_ctx,
                            )?;
                        if let Some(cond) = elem_condition {
                            conditions.push(cond);
                        }
                    }
                }
                Some(Self::combine_runtime_match_conditions(conditions))
            }
            syn::Pat::TupleStruct(tuple_struct_pat) => {
                if let Some((cond_method, unwrap_method)) =
                    self.runtime_tuple_struct_match_methods(&tuple_struct_pat.path, variant_ctx)
                {
                    if tuple_struct_pat.elems.len() != 1 {
                        return None;
                    }
                    let matched_base = format!("rusty::detail::deref_if_pointer({})", source_expr);
                    let matched_value =
                        format!("std::as_const({}).{}()", matched_base, unwrap_method);
                    let mut conditions = vec![format!("{}.{}()", matched_base, cond_method)];
                    let payload_variant_ctx = self.infer_variant_type_context_from_pattern(
                        tuple_struct_pat.elems.first()?,
                        variant_ctx,
                    );
                    let payload_condition = self
                        .collect_runtime_match_binding_stmts_and_condition(
                            tuple_struct_pat.elems.first()?,
                            &matched_value,
                            out,
                            payload_variant_ctx.as_ref(),
                        )?;
                    if let Some(payload_condition) = payload_condition {
                        conditions.push(payload_condition);
                    }
                    return Some(Self::combine_runtime_match_conditions(conditions));
                }
                // Fallback for ambiguous single-segment runtime tuple variants.
                // `Some(x)` / `Ok(x)` / `Err(x)` payload bindings are lowered via
                // `.unwrap*()` in collect_pattern_binding_stmts_*; ensure we also
                // synthesize an explicit runtime condition so these arms do not
                // degrade to `if (true)` and unconditional payload extraction.
                if tuple_struct_pat.elems.len() == 1
                    && !self.path_is_known_data_enum_variant_with_ctx(
                        &tuple_struct_pat.path,
                        variant_ctx,
                    )
                    && let Some(last) = tuple_struct_pat.path.segments.last()
                {
                    let canonical_variant = self
                        .canonical_variant_name(&last.ident.to_string())
                        .to_string();
                    let runtime_tuple_methods = match canonical_variant.as_str() {
                        "Some" => Some(("is_some", "unwrap")),
                        "Ok" => Some(("is_ok", "unwrap")),
                        "Err" => Some(("is_err", "unwrap_err")),
                        _ => None,
                    };
                    if let Some((cond_method, unwrap_method)) = runtime_tuple_methods {
                        let matched_base =
                            format!("rusty::detail::deref_if_pointer({})", source_expr);
                        let matched_value =
                            format!("std::as_const({}).{}()", matched_base, unwrap_method);
                        let mut conditions = vec![format!("{}.{}()", matched_base, cond_method)];
                        let payload_variant_ctx = self.infer_variant_type_context_from_pattern(
                            tuple_struct_pat.elems.first()?,
                            variant_ctx,
                        );
                        let payload_condition = self
                            .collect_runtime_match_binding_stmts_and_condition(
                                tuple_struct_pat.elems.first()?,
                                &matched_value,
                                out,
                                payload_variant_ctx.as_ref(),
                            )?;
                        if let Some(payload_condition) = payload_condition {
                            conditions.push(payload_condition);
                        }
                        return Some(Self::combine_runtime_match_conditions(conditions));
                    }
                }

                let is_data_enum_variant = self
                    .path_is_known_data_enum_variant_with_ctx(&tuple_struct_pat.path, variant_ctx);
                let scrutinee_base = format!("rusty::detail::deref_if_pointer({})", source_expr);
                let mut conditions = Vec::new();
                let payload_base_expr = if is_data_enum_variant {
                    conditions.push(self.runtime_variant_match_condition_for_path(
                        &tuple_struct_pat.path,
                        variant_ctx,
                        &scrutinee_base,
                    ));
                    self.runtime_variant_payload_expr_for_path(
                        &tuple_struct_pat.path,
                        variant_ctx,
                        &scrutinee_base,
                    )
                } else {
                    // Bare-glob unresolved variant (common pattern when a Rust
                    // enum is glob-imported across files via `use Foo::*;` and
                    // the transpiler can't see the enum's definition in this
                    // translation unit). The codegen has no way to synthesize
                    // the right `_v.index() == N` check or `std::get<N>(_v)._0`
                    // binding, so without intervention the arm silently
                    // degrades to `if (true)` plus a `._0` access on a
                    // `std::variant`. Drop a grep-able TODO marker into the
                    // condition so the broken sites are easy to find and
                    // visible in any compiler error that mentions the line.
                    if let Some(variant_name) = tuple_struct_pat
                        .path
                        .segments
                        .last()
                        .map(|seg| seg.ident.to_string())
                    {
                        conditions.push(format!(
                            "/* TODO transpiler: unresolved bare-glob variant `{}` (no enum decl visible in this TU; patch arm manually) */ true",
                            variant_name
                        ));
                    }
                    scrutinee_base
                };
                for (idx, elem_pat) in tuple_struct_pat.elems.iter().enumerate() {
                    let field_expr = self.data_enum_variant_tuple_field_binding_expr(
                        &tuple_struct_pat.path,
                        variant_ctx,
                        &payload_base_expr,
                        idx,
                    );
                    let field_condition = self.collect_runtime_match_binding_stmts_and_condition(
                        elem_pat,
                        &field_expr,
                        out,
                        variant_ctx,
                    )?;
                    if let Some(cond) = field_condition {
                        conditions.push(cond);
                    }
                }
                Some(Self::combine_runtime_match_conditions(conditions))
            }
            syn::Pat::Path(path_pat) => {
                if let Some(cond_method) =
                    self.runtime_path_match_condition_method(&path_pat.path, variant_ctx)
                {
                    let source_expr = format!("rusty::detail::deref_if_pointer({})", source_expr);
                    Some(Some(format!(
                        "rusty::detail::deref_if_pointer({}).{}()",
                        source_expr, cond_method
                    )))
                } else if self.path_is_known_data_enum_variant_with_ctx(&path_pat.path, variant_ctx)
                {
                    let scrutinee_base =
                        format!("rusty::detail::deref_if_pointer({})", source_expr);
                    Some(Some(self.runtime_variant_match_condition_for_path(
                        &path_pat.path,
                        variant_ctx,
                        &scrutinee_base,
                    )))
                } else {
                    self.tuple_pattern_elem_value_condition(pat, source_expr)
                }
            }
            syn::Pat::Struct(struct_pat) => {
                let is_data_enum_variant =
                    self.path_is_known_data_enum_variant_with_ctx(&struct_pat.path, variant_ctx);
                let scrutinee_base = format!("rusty::detail::deref_if_pointer({})", source_expr);
                let field_base_expr = if is_data_enum_variant {
                    self.runtime_variant_payload_expr_for_path(
                        &struct_pat.path,
                        variant_ctx,
                        &scrutinee_base,
                    )
                } else {
                    source_expr.to_string()
                };
                let mut conditions = Vec::new();
                if is_data_enum_variant {
                    conditions.push(self.runtime_variant_match_condition_for_path(
                        &struct_pat.path,
                        variant_ctx,
                        &scrutinee_base,
                    ));
                }
                for field_pat in &struct_pat.fields {
                    let field_name = match &field_pat.member {
                        syn::Member::Named(ident) => ident.to_string(),
                        syn::Member::Unnamed(index) => format!("_{}", index.index),
                    };
                    let emitted_field_name = match &field_pat.member {
                        syn::Member::Named(_) => self.resolve_struct_pattern_field_cpp_name(
                            &struct_pat.path,
                            &field_name,
                            variant_ctx,
                        ),
                        syn::Member::Unnamed(_) => field_name.clone(),
                    };
                    let field_expr = format!("{}.{}", field_base_expr, emitted_field_name);
                    let field_condition = self.collect_runtime_match_binding_stmts_and_condition(
                        &field_pat.pat,
                        &field_expr,
                        out,
                        variant_ctx,
                    )?;
                    if let Some(cond) = field_condition {
                        conditions.push(cond);
                    }
                }
                Some(Self::combine_runtime_match_conditions(conditions))
            }
            syn::Pat::Or(or_pat) => {
                let mut conditions = Vec::new();
                for case in &or_pat.cases {
                    let mut case_bindings = Vec::new();
                    let case_condition = self.collect_runtime_match_binding_stmts_and_condition(
                        case,
                        source_expr,
                        &mut case_bindings,
                        variant_ctx,
                    )?;
                    if !case_bindings.is_empty() {
                        return None;
                    }
                    if case_condition.is_none() {
                        return Some(None);
                    }
                    conditions.push(case_condition.unwrap_or_default());
                }
                if conditions.is_empty() {
                    Some(None)
                } else if conditions.len() == 1 {
                    Some(Some(conditions.remove(0)))
                } else {
                    Some(Some(format!("({})", conditions.join(" || "))))
                }
            }
            syn::Pat::Type(pt) => self.collect_runtime_match_binding_stmts_and_condition(
                &pt.pat,
                source_expr,
                out,
                variant_ctx,
            ),
            syn::Pat::Reference(r) => {
                let deref_expr = format!("rusty::detail::deref_if_pointer({})", source_expr);
                self.collect_runtime_match_binding_stmts_and_condition(
                    &r.pat,
                    &deref_expr,
                    out,
                    variant_ctx,
                )
            }
            syn::Pat::Paren(p) => self.collect_runtime_match_binding_stmts_and_condition(
                &p.pat,
                source_expr,
                out,
                variant_ctx,
            ),
            _ => self.tuple_pattern_elem_value_condition(pat, source_expr),
        }
    }

    pub(super) fn collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
        &self,
        pat: &syn::Pat,
        source_expr: &str,
        out: &mut Vec<String>,
        rust_to_cpp: &mut HashMap<String, String>,
        variant_ctx: Option<&VariantTypeContext>,
    ) -> Option<Option<String>> {
        match pat {
            syn::Pat::Wild(_) => {
                if self.collect_pattern_binding_stmts_with_cpp_name_map(
                    pat,
                    source_expr,
                    out,
                    rust_to_cpp,
                ) {
                    Some(None)
                } else {
                    None
                }
            }
            syn::Pat::Ident(pi) => {
                if let Some((_, subpat)) = &pi.subpat {
                    let mut ident_only = pi.clone();
                    ident_only.subpat = None;
                    let ident_pat = syn::Pat::Ident(ident_only);
                    if !self.collect_pattern_binding_stmts_with_cpp_name_map(
                        &ident_pat,
                        source_expr,
                        out,
                        rust_to_cpp,
                    ) {
                        return None;
                    }
                    if matches!(self.peel_pat_type_ref_paren(subpat), syn::Pat::Rest(_)) {
                        return Some(None);
                    }
                    self.collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                        subpat,
                        source_expr,
                        out,
                        rust_to_cpp,
                        variant_ctx,
                    )
                } else if let Some(cond_method) =
                    self.runtime_ident_match_condition_method(pi, variant_ctx)
                {
                    Some(Some(format!(
                        "rusty::detail::deref_if_pointer({}).{}()",
                        source_expr, cond_method
                    )))
                } else if pi.by_ref.is_none() && pi.mutability.is_none() && pi.subpat.is_none() {
                    let ident_name = pi.ident.to_string();
                    if self.pattern_ident_is_const_value(&ident_name) {
                        Some(Some(self.const_value_ident_pattern_condition(
                            &pi.ident,
                            source_expr,
                            variant_ctx,
                        )))
                    } else if self.collect_pattern_binding_stmts_with_cpp_name_map(
                        pat,
                        source_expr,
                        out,
                        rust_to_cpp,
                    ) {
                        Some(None)
                    } else {
                        None
                    }
                } else if self.collect_pattern_binding_stmts_with_cpp_name_map(
                    pat,
                    source_expr,
                    out,
                    rust_to_cpp,
                ) {
                    Some(None)
                } else {
                    None
                }
            }
            syn::Pat::Tuple(tuple_pat) => {
                let mut conditions = Vec::new();
                for (i, elem_pat) in tuple_pat.elems.iter().enumerate() {
                    let elem_expr = format!(
                        "std::get<{}>(rusty::detail::deref_if_pointer({}))",
                        i, source_expr
                    );
                    let elem_condition = self
                        .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                            elem_pat,
                            &elem_expr,
                            out,
                            rust_to_cpp,
                            variant_ctx,
                        )?;
                    if let Some(cond) = elem_condition {
                        conditions.push(cond);
                    }
                }
                Some(Self::combine_runtime_match_conditions(conditions))
            }
            syn::Pat::Slice(slice_pat) => {
                let mut conditions = Vec::new();
                let base_expr = format!("rusty::detail::deref_if_pointer({})", source_expr);
                let len_expr = format!("rusty::len({})", base_expr);
                let rest_pos = slice_pat
                    .elems
                    .iter()
                    .position(|elem| self.pat_is_slice_rest_like(elem));
                if let Some(rest_pos) = rest_pos {
                    let suffix_count = slice_pat.elems.len().saturating_sub(rest_pos + 1);
                    conditions.push(format!("{} >= {}", len_expr, rest_pos + suffix_count));
                    for (i, elem_pat) in slice_pat.elems.iter().take(rest_pos).enumerate() {
                        let elem_expr = format!("{}[{}]", base_expr, i);
                        let elem_condition = self
                            .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                                elem_pat,
                                &elem_expr,
                                out,
                                rust_to_cpp,
                                variant_ctx,
                            )?;
                        if let Some(cond) = elem_condition {
                            conditions.push(cond);
                        }
                    }
                    let rest_pat = slice_pat.elems.iter().nth(rest_pos)?;
                    if !matches!(self.peel_pat_type_ref_paren(rest_pat), syn::Pat::Rest(_)) {
                        let rest_end = if suffix_count == 0 {
                            len_expr.clone()
                        } else {
                            format!("({} - {})", len_expr, suffix_count)
                        };
                        let rest_expr =
                            format!("rusty::slice({}, {}, {})", base_expr, rest_pos, rest_end);
                        let rest_condition = self
                            .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                                rest_pat,
                                &rest_expr,
                                out,
                                rust_to_cpp,
                                variant_ctx,
                            )?;
                        if let Some(cond) = rest_condition {
                            conditions.push(cond);
                        }
                    }
                    for (offset, elem_pat) in slice_pat.elems.iter().skip(rest_pos + 1).enumerate()
                    {
                        let idx_expr = format!("({} - {})", len_expr, suffix_count - offset);
                        let elem_expr = format!("{}[{}]", base_expr, idx_expr);
                        let elem_condition = self
                            .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                                elem_pat,
                                &elem_expr,
                                out,
                                rust_to_cpp,
                                variant_ctx,
                            )?;
                        if let Some(cond) = elem_condition {
                            conditions.push(cond);
                        }
                    }
                } else {
                    conditions.push(format!("{} == {}", len_expr, slice_pat.elems.len()));
                    for (i, elem_pat) in slice_pat.elems.iter().enumerate() {
                        let elem_expr = format!("{}[{}]", base_expr, i);
                        let elem_condition = self
                            .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                                elem_pat,
                                &elem_expr,
                                out,
                                rust_to_cpp,
                                variant_ctx,
                            )?;
                        if let Some(cond) = elem_condition {
                            conditions.push(cond);
                        }
                    }
                }
                Some(Self::combine_runtime_match_conditions(conditions))
            }
            syn::Pat::TupleStruct(tuple_struct_pat) => {
                if let Some((cond_method, unwrap_method)) =
                    self.runtime_tuple_struct_match_methods(&tuple_struct_pat.path, variant_ctx)
                {
                    if tuple_struct_pat.elems.len() != 1 {
                        return None;
                    }
                    let matched_base = format!("rusty::detail::deref_if_pointer({})", source_expr);
                    let matched_value =
                        format!("std::as_const({}).{}()", matched_base, unwrap_method);
                    let mut conditions = vec![format!("{}.{}()", matched_base, cond_method)];
                    let payload_variant_ctx = self.infer_variant_type_context_from_pattern(
                        tuple_struct_pat.elems.first()?,
                        variant_ctx,
                    );
                    let payload_condition = self
                        .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                            tuple_struct_pat.elems.first()?,
                            &matched_value,
                            out,
                            rust_to_cpp,
                            payload_variant_ctx.as_ref(),
                        )?;
                    if let Some(payload_condition) = payload_condition {
                        conditions.push(payload_condition);
                    }
                    return Some(Self::combine_runtime_match_conditions(conditions));
                }
                // Fallback for ambiguous single-segment runtime tuple variants.
                // Keep `Some(x)` / `Ok(x)` / `Err(x)` conditions aligned with
                // payload binding lowering that unwraps these runtime wrappers.
                if tuple_struct_pat.elems.len() == 1
                    && !self.path_is_known_data_enum_variant_with_ctx(
                        &tuple_struct_pat.path,
                        variant_ctx,
                    )
                    && let Some(last) = tuple_struct_pat.path.segments.last()
                {
                    let canonical_variant = self
                        .canonical_variant_name(&last.ident.to_string())
                        .to_string();
                    let runtime_tuple_methods = match canonical_variant.as_str() {
                        "Some" => Some(("is_some", "unwrap")),
                        "Ok" => Some(("is_ok", "unwrap")),
                        "Err" => Some(("is_err", "unwrap_err")),
                        _ => None,
                    };
                    if let Some((cond_method, unwrap_method)) = runtime_tuple_methods {
                        let matched_base =
                            format!("rusty::detail::deref_if_pointer({})", source_expr);
                        let matched_value =
                            format!("std::as_const({}).{}()", matched_base, unwrap_method);
                        let mut conditions = vec![format!("{}.{}()", matched_base, cond_method)];
                        let payload_variant_ctx = self.infer_variant_type_context_from_pattern(
                            tuple_struct_pat.elems.first()?,
                            variant_ctx,
                        );
                        let payload_condition = self
                            .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                                tuple_struct_pat.elems.first()?,
                                &matched_value,
                                out,
                                rust_to_cpp,
                                payload_variant_ctx.as_ref(),
                            )?;
                        if let Some(payload_condition) = payload_condition {
                            conditions.push(payload_condition);
                        }
                        return Some(Self::combine_runtime_match_conditions(conditions));
                    }
                }

                let is_data_enum_variant = self
                    .path_is_known_data_enum_variant_with_ctx(&tuple_struct_pat.path, variant_ctx);
                let scrutinee_base = format!("rusty::detail::deref_if_pointer({})", source_expr);
                let mut conditions = Vec::new();
                let payload_base_expr = if is_data_enum_variant {
                    conditions.push(self.runtime_variant_match_condition_for_path(
                        &tuple_struct_pat.path,
                        variant_ctx,
                        &scrutinee_base,
                    ));
                    self.runtime_variant_payload_expr_for_path(
                        &tuple_struct_pat.path,
                        variant_ctx,
                        &scrutinee_base,
                    )
                } else {
                    // Bare-glob unresolved variant (common pattern when a Rust
                    // enum is glob-imported across files via `use Foo::*;` and
                    // the transpiler can't see the enum's definition in this
                    // translation unit). The codegen has no way to synthesize
                    // the right `_v.index() == N` check or `std::get<N>(_v)._0`
                    // binding, so without intervention the arm silently
                    // degrades to `if (true)` plus a `._0` access on a
                    // `std::variant`. Drop a grep-able TODO marker into the
                    // condition so the broken sites are easy to find and
                    // visible in any compiler error that mentions the line.
                    if let Some(variant_name) = tuple_struct_pat
                        .path
                        .segments
                        .last()
                        .map(|seg| seg.ident.to_string())
                    {
                        conditions.push(format!(
                            "/* TODO transpiler: unresolved bare-glob variant `{}` (no enum decl visible in this TU; patch arm manually) */ true",
                            variant_name
                        ));
                    }
                    scrutinee_base
                };
                for (idx, elem_pat) in tuple_struct_pat.elems.iter().enumerate() {
                    let field_expr = self.data_enum_variant_tuple_field_binding_expr(
                        &tuple_struct_pat.path,
                        variant_ctx,
                        &payload_base_expr,
                        idx,
                    );
                    let field_condition = self
                        .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                            elem_pat,
                            &field_expr,
                            out,
                            rust_to_cpp,
                            variant_ctx,
                        )?;
                    if let Some(cond) = field_condition {
                        conditions.push(cond);
                    }
                }
                Some(Self::combine_runtime_match_conditions(conditions))
            }
            syn::Pat::Path(path_pat) => {
                if let Some(cond_method) =
                    self.runtime_path_match_condition_method(&path_pat.path, variant_ctx)
                {
                    let source_expr = format!("rusty::detail::deref_if_pointer({})", source_expr);
                    Some(Some(format!(
                        "rusty::detail::deref_if_pointer({}).{}()",
                        source_expr, cond_method
                    )))
                } else if self.path_is_known_data_enum_variant_with_ctx(&path_pat.path, variant_ctx)
                {
                    let scrutinee_base =
                        format!("rusty::detail::deref_if_pointer({})", source_expr);
                    Some(Some(self.runtime_variant_match_condition_for_path(
                        &path_pat.path,
                        variant_ctx,
                        &scrutinee_base,
                    )))
                } else {
                    self.tuple_pattern_elem_value_condition(pat, source_expr)
                }
            }
            syn::Pat::Struct(struct_pat) => {
                let is_data_enum_variant =
                    self.path_is_known_data_enum_variant_with_ctx(&struct_pat.path, variant_ctx);
                let scrutinee_base = format!("rusty::detail::deref_if_pointer({})", source_expr);
                let field_base_expr = if is_data_enum_variant {
                    self.runtime_variant_payload_expr_for_path(
                        &struct_pat.path,
                        variant_ctx,
                        &scrutinee_base,
                    )
                } else {
                    source_expr.to_string()
                };
                let mut conditions = Vec::new();
                if is_data_enum_variant {
                    conditions.push(self.runtime_variant_match_condition_for_path(
                        &struct_pat.path,
                        variant_ctx,
                        &scrutinee_base,
                    ));
                }
                for field_pat in &struct_pat.fields {
                    let field_name = match &field_pat.member {
                        syn::Member::Named(ident) => ident.to_string(),
                        syn::Member::Unnamed(index) => format!("_{}", index.index),
                    };
                    let emitted_field_name = match &field_pat.member {
                        syn::Member::Named(_) => self.resolve_struct_pattern_field_cpp_name(
                            &struct_pat.path,
                            &field_name,
                            variant_ctx,
                        ),
                        syn::Member::Unnamed(_) => field_name.clone(),
                    };
                    let field_expr = format!("{}.{}", field_base_expr, emitted_field_name);
                    let field_condition = self
                        .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                            &field_pat.pat,
                            &field_expr,
                            out,
                            rust_to_cpp,
                            variant_ctx,
                        )?;
                    if let Some(cond) = field_condition {
                        conditions.push(cond);
                    }
                }
                Some(Self::combine_runtime_match_conditions(conditions))
            }
            syn::Pat::Or(or_pat) => {
                let mut conditions = Vec::new();
                let mut case_data: Vec<(Vec<String>, HashMap<String, String>)> = Vec::new();
                for case in &or_pat.cases {
                    let mut case_bindings = Vec::new();
                    let mut case_binding_map = HashMap::new();
                    let case_condition = self
                        .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                            case,
                            source_expr,
                            &mut case_bindings,
                            &mut case_binding_map,
                            variant_ctx,
                        )?;
                    let Some(cond) = case_condition else {
                        if case_bindings.is_empty() && case_binding_map.is_empty() {
                            // Irrefutable case: the whole or-pattern matches.
                            return Some(None);
                        }
                        return None;
                    };
                    conditions.push(cond);
                    case_data.push((case_bindings, case_binding_map));
                }
                let all_binding_less = case_data
                    .iter()
                    .all(|(stmts, map)| stmts.is_empty() && map.is_empty());
                if all_binding_less {
                    return if conditions.is_empty() {
                        Some(None)
                    } else if conditions.len() == 1 {
                        Some(Some(conditions.remove(0)))
                    } else {
                        Some(Some(format!("({})", conditions.join(" || "))))
                    };
                }
                // Or-cases WITH bindings (error.rs mark():
                // `Message(_, Some(Pos { mark, .. })) | UnknownAnchor(mark)
                // => Some(*mark)`): every case must bind the SAME Rust
                // names. Bind each name once via a condition-selected
                // ternary chain — the per-case payload expressions are all
                // the same type (Rust requires or-case bindings to agree).
                let first_map = &case_data[0].1;
                let mut rust_names: Vec<String> = first_map.keys().cloned().collect();
                rust_names.sort();
                if case_data.iter().any(|(_, map)| {
                    map.len() != first_map.len()
                        || !rust_names.iter().all(|name| map.contains_key(name))
                }) {
                    return None;
                }
                fn binding_stmt_expr(stmts: &[String], cpp_name: &str) -> Option<String> {
                    for stmt in stmts {
                        if let Some(eq) = stmt.find(" = ")
                            && stmt.ends_with(';')
                        {
                            let lhs = stmt[..eq].trim_end();
                            if lhs.ends_with(cpp_name)
                                && lhs.len() > cpp_name.len()
                                && lhs[..lhs.len() - cpp_name.len()]
                                    .ends_with([' ', '&', '*'])
                            {
                                return Some(stmt[eq + 3..stmt.len() - 1].to_string());
                            }
                        }
                    }
                    None
                }
                for rust_name in &rust_names {
                    let mut case_exprs = Vec::new();
                    for (stmts, map) in &case_data {
                        let cpp = map.get(rust_name)?;
                        case_exprs.push(binding_stmt_expr(stmts, cpp)?);
                    }
                    // c0 ? e0 : (c1 ? e1 : e_last)
                    let mut selected = case_exprs.pop()?;
                    for (cond, expr) in conditions
                        .iter()
                        .zip(case_exprs.iter())
                        .rev()
                    {
                        selected = format!("({} ? ({}) : ({}))", cond, expr, selected);
                    }
                    let cpp_name = first_map.get(rust_name)?.clone();
                    out.push(format!("auto&& {} = {};", cpp_name, selected));
                    rust_to_cpp.insert(rust_name.clone(), cpp_name);
                }
                Some(Some(format!("({})", conditions.join(" || "))))
            }
            syn::Pat::Type(pt) => self
                .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                    &pt.pat,
                    source_expr,
                    out,
                    rust_to_cpp,
                    variant_ctx,
                ),
            syn::Pat::Reference(r) => {
                let deref_expr = format!("rusty::detail::deref_if_pointer({})", source_expr);
                self.collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                    &r.pat,
                    &deref_expr,
                    out,
                    rust_to_cpp,
                    variant_ctx,
                )
            }
            syn::Pat::Paren(p) => self
                .collect_runtime_match_binding_stmts_and_condition_with_cpp_name_map(
                    &p.pat,
                    source_expr,
                    out,
                    rust_to_cpp,
                    variant_ctx,
                ),
            _ => self.tuple_pattern_elem_value_condition(pat, source_expr),
        }
    }

    pub(super) fn collect_pattern_binding_stmts_with_cpp_name_map(
        &self,
        pat: &syn::Pat,
        source_expr: &str,
        out: &mut Vec<String>,
        rust_to_cpp: &mut HashMap<String, String>,
    ) -> bool {
        match pat {
            syn::Pat::Ident(pi) => {
                if pi.ident != "_" {
                    let rust_name = pi.ident.to_string();
                    if pi.by_ref.is_none()
                        && pi.mutability.is_none()
                        && pi.subpat.is_none()
                        && self.pattern_ident_is_const_value(&rust_name)
                    {
                        // Constant-like identifiers in patterns (e.g. None, Equal) are
                        // value matches, not new bindings.
                        return true;
                    }
                    let cpp_name = if let Some(existing) = rust_to_cpp.get(&rust_name) {
                        existing.clone()
                    } else {
                        let resolved = self
                            .lookup_local_binding_cpp_name(&rust_name)
                            .unwrap_or_else(|| {
                                self.fallback_pattern_binding_cpp_name(&rust_name, rust_to_cpp)
                            });
                        rust_to_cpp.insert(rust_name.clone(), resolved.clone());
                        resolved
                    };
                    let by_value_mut = pi.by_ref.is_none() && pi.mutability.is_some();
                    let binding_prefix = if pi.by_ref.is_some() && pi.mutability.is_some() {
                        "auto&"
                    } else if pi.by_ref.is_some() {
                        "const auto&"
                    } else if by_value_mut {
                        // A `mut` by-value binding is an INDEPENDENT mutable
                        // place — Rust moves (non-Copy) or copies (Copy) the
                        // payload into it. `auto&&` would inherit the payload's
                        // const category (a borrowed scrutinee yields `const
                        // T&`), so `&mut name` / a `T&`-parameter call fails
                        // ("drops const"). A bare `auto` with a moved source
                        // makes a fresh mutable object: move-ctor for owned
                        // move-only payloads (btree_port `Occupied(mut entry)`),
                        // copy-ctor for const Copy scalars (de.rs
                        // `Event::Alias(mut pos)` → `jump(&mut pos)`).
                        "auto"
                    } else {
                        // Plain by-value binding (`x`): `auto&&` preserves
                        // reference payloads (`R = T&`) without forcing const,
                        // and avoids a hidden copy of move-only values.
                        "auto&&"
                    };
                    // Subslice rest-bindings (`[head, rest @ ..]`) produce a
                    // rusty::slice(...) PRVALUE (a std::span view); see the
                    // twin comment in collect_pattern_binding_stmts — routing
                    // it through deref_if_pointer dangles. Bind by value.
                    if pi.by_ref.is_none() && source_expr.starts_with("rusty::slice(") {
                        out.push(format!("auto {} = {};", cpp_name, source_expr));
                    } else {
                        let binding_source = if pi.by_ref.is_none() {
                            let derefed =
                                format!("rusty::detail::deref_if_pointer({})", source_expr);
                            if by_value_mut {
                                format!("std::move({})", derefed)
                            } else {
                                derefed
                            }
                        } else {
                            source_expr.to_string()
                        };
                        out.push(format!(
                            "{} {} = {};",
                            binding_prefix, cpp_name, binding_source
                        ));
                    }
                }
                if let Some((_, subpat)) = &pi.subpat {
                    let mut sub_bindings = Vec::new();
                    if self.collect_pattern_binding_stmts_with_cpp_name_map(
                        subpat,
                        source_expr,
                        &mut sub_bindings,
                        rust_to_cpp,
                    ) {
                        out.extend(sub_bindings);
                    }
                }
                true
            }
            syn::Pat::Wild(_) => true,
            syn::Pat::Tuple(tuple_pat) => {
                for (i, elem) in tuple_pat.elems.iter().enumerate() {
                    let elem_expr = format!(
                        "std::get<{}>(rusty::detail::deref_if_pointer({}))",
                        i, source_expr
                    );
                    if !self.collect_pattern_binding_stmts_with_cpp_name_map(
                        elem,
                        &elem_expr,
                        out,
                        rust_to_cpp,
                    ) {
                        return false;
                    }
                }
                true
            }
            syn::Pat::TupleStruct(tuple_struct_pat) => {
                let variant_name = tuple_struct_pat
                    .path
                    .segments
                    .last()
                    .map(|seg| seg.ident.to_string())
                    .unwrap_or_default();
                if tuple_struct_pat.elems.len() == 1
                    && matches!(variant_name.as_str(), "Some" | "Ok" | "Err")
                {
                    let base_expr = format!("rusty::detail::deref_if_pointer({})", source_expr);
                    let inner_expr = if variant_name == "Err" {
                        format!("({}).unwrap_err()", base_expr)
                    } else {
                        format!("({}).unwrap()", base_expr)
                    };
                    if let Some(elem) = tuple_struct_pat.elems.first() {
                        // `.unwrap()` / `.unwrap_err()` CONSUMES the Option/Result.
                        // If the inner pattern binds more than one value (e.g.
                        // `Some((a, b))`), recursing with the unwrap *string* as
                        // the source would textually duplicate it into each leaf
                        // access (`get<0>((x).unwrap())`, `get<1>((x).unwrap())`),
                        // double-consuming a moved-from value → "unwrap on None"
                        // at runtime. Bind the unwrap to a temp ONCE first.
                        let mut inner_names = HashSet::new();
                        self.collect_pattern_binding_names(elem, &mut inner_names);
                        let binds_multiple =
                            inner_names.iter().filter(|n| n.as_str() != "_").count() > 1;
                        if binds_multiple {
                            let tmp = format!(
                                "_let_unwrapped_{}",
                                self.unwrap_tmp_counter.get()
                            );
                            self.unwrap_tmp_counter
                                .set(self.unwrap_tmp_counter.get() + 1);
                            out.push(format!("auto {} = {};", tmp, inner_expr));
                            return self.collect_pattern_binding_stmts_with_cpp_name_map(
                                elem,
                                &tmp,
                                out,
                                rust_to_cpp,
                            );
                        }
                        return self.collect_pattern_binding_stmts_with_cpp_name_map(
                            elem,
                            &inner_expr,
                            out,
                            rust_to_cpp,
                        );
                    }
                }
                let base_expr = format!("rusty::detail::deref_if_pointer({})", source_expr);
                for (i, elem) in tuple_struct_pat.elems.iter().enumerate() {
                    let field_expr = format!("{}._{}", base_expr, i);
                    if !self.collect_pattern_binding_stmts_with_cpp_name_map(
                        elem,
                        &field_expr,
                        out,
                        rust_to_cpp,
                    ) {
                        return false;
                    }
                }
                true
            }
            syn::Pat::Slice(slice_pat) => {
                let base_expr = format!("rusty::detail::deref_if_pointer({})", source_expr);
                let len_expr = format!("rusty::len({})", base_expr);
                let rest_pos = slice_pat
                    .elems
                    .iter()
                    .position(|elem| self.pat_is_slice_rest_like(elem));
                if let Some(rest_pos) = rest_pos {
                    let suffix_count = slice_pat.elems.len().saturating_sub(rest_pos + 1);
                    for (i, elem) in slice_pat.elems.iter().take(rest_pos).enumerate() {
                        let elem_expr = format!("{}[{}]", base_expr, i);
                        if !self.collect_pattern_binding_stmts_with_cpp_name_map(
                            elem,
                            &elem_expr,
                            out,
                            rust_to_cpp,
                        ) {
                            return false;
                        }
                    }
                    let rest_pat = match slice_pat.elems.iter().nth(rest_pos) {
                        Some(pat) => pat,
                        None => return false,
                    };
                    if !matches!(self.peel_pat_type_ref_paren(rest_pat), syn::Pat::Rest(_)) {
                        let rest_end = if suffix_count == 0 {
                            len_expr.clone()
                        } else {
                            format!("({} - {})", len_expr, suffix_count)
                        };
                        let rest_expr =
                            format!("rusty::slice({}, {}, {})", base_expr, rest_pos, rest_end);
                        if !self.collect_pattern_binding_stmts_with_cpp_name_map(
                            rest_pat,
                            &rest_expr,
                            out,
                            rust_to_cpp,
                        ) {
                            return false;
                        }
                    }
                    for (offset, elem) in slice_pat.elems.iter().skip(rest_pos + 1).enumerate() {
                        let idx_expr = format!("({} - {})", len_expr, suffix_count - offset);
                        let elem_expr = format!("{}[{}]", base_expr, idx_expr);
                        if !self.collect_pattern_binding_stmts_with_cpp_name_map(
                            elem,
                            &elem_expr,
                            out,
                            rust_to_cpp,
                        ) {
                            return false;
                        }
                    }
                } else {
                    for (i, elem) in slice_pat.elems.iter().enumerate() {
                        let elem_expr = format!("{}[{}]", base_expr, i);
                        if !self.collect_pattern_binding_stmts_with_cpp_name_map(
                            elem,
                            &elem_expr,
                            out,
                            rust_to_cpp,
                        ) {
                            return false;
                        }
                    }
                }
                true
            }
            syn::Pat::Struct(struct_pat) => {
                for field_pat in &struct_pat.fields {
                    let field_name = match &field_pat.member {
                        syn::Member::Named(ident) => ident.to_string(),
                        syn::Member::Unnamed(index) => format!("_{}", index.index),
                    };
                    let emitted_field_name = match &field_pat.member {
                        syn::Member::Named(_) => self.resolve_struct_pattern_field_cpp_name(
                            &struct_pat.path,
                            &field_name,
                            None,
                        ),
                        syn::Member::Unnamed(_) => field_name.clone(),
                    };
                    let field_expr = format!("{}.{}", source_expr, emitted_field_name);
                    if !self.collect_pattern_binding_stmts_with_cpp_name_map(
                        &field_pat.pat,
                        &field_expr,
                        out,
                        rust_to_cpp,
                    ) {
                        return false;
                    }
                }
                true
            }
            syn::Pat::Type(pt) => self.collect_pattern_binding_stmts_with_cpp_name_map(
                &pt.pat,
                source_expr,
                out,
                rust_to_cpp,
            ),
            syn::Pat::Reference(r) => {
                let deref_expr = format!("rusty::detail::deref_if_pointer({})", source_expr);
                self.collect_pattern_binding_stmts_with_cpp_name_map(
                    &r.pat,
                    &deref_expr,
                    out,
                    rust_to_cpp,
                )
            }
            syn::Pat::Paren(p) => self.collect_pattern_binding_stmts_with_cpp_name_map(
                &p.pat,
                source_expr,
                out,
                rust_to_cpp,
            ),
            syn::Pat::Or(or_pat) => {
                // An irrefutable (let) or-pattern binds the SAME names in every arm.
                // Handle the `Ok(i) | Err(i)` idiom (Result<T,T>, e.g. from
                // `binary_search`'s "index whether found or not"): the bound value is
                // the same regardless of which variant matched, so extract it with a
                // variant-neutral ternary. Every arm must be a single-field Ok/Err
                // tuple-struct binding one ident, all the same name.
                let arms: Vec<&syn::Pat> = or_pat.cases.iter().collect();
                let mut binding_ident: Option<&syn::Pat> = None;
                let mut binding_name: Option<String> = None;
                let mut has_ok = false;
                let mut has_err = false;
                let mut well_formed = arms.len() == 2;
                for arm in &arms {
                    let syn::Pat::TupleStruct(ts) = *arm else {
                        well_formed = false;
                        break;
                    };
                    let (Some(variant), true) = (
                        ts.path.segments.last().map(|s| s.ident.to_string()),
                        ts.elems.len() == 1,
                    ) else {
                        well_formed = false;
                        break;
                    };
                    let Some(inner @ syn::Pat::Ident(pi)) = ts.elems.first() else {
                        well_formed = false;
                        break;
                    };
                    let name = pi.ident.to_string();
                    match &binding_name {
                        Some(existing) if *existing != name => {
                            well_formed = false;
                            break;
                        }
                        None => {
                            binding_name = Some(name);
                            binding_ident = Some(inner);
                        }
                        _ => {}
                    }
                    match variant.as_str() {
                        "Ok" => has_ok = true,
                        "Err" => has_err = true,
                        _ => {
                            well_formed = false;
                            break;
                        }
                    }
                }
                if !well_formed || !has_ok || !has_err {
                    return false;
                }
                let base = format!("rusty::detail::deref_if_pointer({})", source_expr);
                let value_expr =
                    format!("({b}.is_ok() ? ({b}).unwrap() : ({b}).unwrap_err())", b = base);
                self.collect_pattern_binding_stmts_with_cpp_name_map(
                    binding_ident.expect("checked well_formed"),
                    &value_expr,
                    out,
                    rust_to_cpp,
                )
            }
            _ => false,
        }
    }

    pub(super) fn collect_switch_match_tuple_literal_hints(
        &self,
        arms: &[syn::Arm],
    ) -> Option<Vec<Option<String>>> {
        let mut tuple_exprs: Vec<&syn::ExprTuple> = Vec::new();
        let mut tuple_len: Option<usize> = None;
        for arm in arms {
            let arm_value_expr = self
                .extract_match_arm_value_expr(&arm.body)
                .unwrap_or(&arm.body);
            let syn::Expr::Tuple(tuple) = self.peel_paren_group_expr(arm_value_expr) else {
                return None;
            };
            if let Some(expected_len) = tuple_len {
                if tuple.elems.len() != expected_len {
                    return None;
                }
            } else {
                tuple_len = Some(tuple.elems.len());
            }
            tuple_exprs.push(tuple);
        }
        let tuple_len = tuple_len?;
        if tuple_len == 0 {
            return None;
        }

        let mut hints = vec![None; tuple_len];
        for idx in 0..tuple_len {
            for tuple in &tuple_exprs {
                let elem = self.peel_paren_group_expr(&tuple.elems[idx]);
                if Self::is_unsuffixed_int_literal_expr(elem) {
                    continue;
                }
                if Self::is_plain_ident_path_expr(elem) {
                    continue;
                }
                hints[idx] =
                    Some(self.emit_expr_to_string_with_expected_and_move_if_needed(
                        &tuple.elems[idx],
                        None,
                    ));
                break;
            }
        }

        if hints.iter().all(|hint| hint.is_none()) {
            None
        } else {
            Some(hints)
        }
    }

    pub(super) fn collect_single_char_type_param_names_in_type(
        &self,
        ty: &syn::Type,
        out: &mut HashSet<String>,
    ) {
        match ty {
            syn::Type::Path(tp) => {
                if tp.qself.is_none() && tp.path.segments.len() == 1 {
                    let ident = tp.path.segments[0].ident.to_string();
                    if ident.len() == 1
                        && ident.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                    {
                        out.insert(ident);
                    }
                }
                for seg in &tp.path.segments {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        for arg in &args.args {
                            if let syn::GenericArgument::Type(inner) = arg {
                                self.collect_single_char_type_param_names_in_type(inner, out);
                            }
                        }
                    }
                }
            }
            syn::Type::Reference(r) => {
                self.collect_single_char_type_param_names_in_type(&r.elem, out)
            }
            syn::Type::Ptr(p) => self.collect_single_char_type_param_names_in_type(&p.elem, out),
            syn::Type::Slice(s) => self.collect_single_char_type_param_names_in_type(&s.elem, out),
            syn::Type::Array(a) => self.collect_single_char_type_param_names_in_type(&a.elem, out),
            syn::Type::Tuple(t) => {
                for elem in &t.elems {
                    self.collect_single_char_type_param_names_in_type(elem, out);
                }
            }
            syn::Type::Paren(p) => self.collect_single_char_type_param_names_in_type(&p.elem, out),
            syn::Type::Group(g) => self.collect_single_char_type_param_names_in_type(&g.elem, out),
            _ => {}
        }
    }

    pub(super) fn collect_assoc_binding_type_param_candidates_in_type(
        &self,
        ty: &syn::Type,
        out: &mut HashSet<String>,
    ) {
        match ty {
            syn::Type::Path(tp) => {
                if tp.qself.is_none() && tp.path.segments.len() == 1 {
                    let seg = &tp.path.segments[0];
                    if matches!(seg.arguments, syn::PathArguments::None) {
                        let ident = seg.ident.to_string();
                        let looks_like_type_param =
                            ident.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                                && ident != "Self"
                                && !self.is_type_param_in_scope(&ident)
                                && !self.is_local_type_name_in_scope(&ident)
                                && !self.declared_item_names.contains(&ident)
                                // A concrete crate-declared type (e.g. `IgnoredAny`, used
                                // as a unit-struct assoc binding `type Value = IgnoredAny`)
                                // is NOT a type param — the scope-sensitive guards above can
                                // miss it; `local_declared_types` is the flat set of every
                                // declared type. Without this, a spurious `typename
                                // IgnoredAny` template param is added and a body local of the
                                // same name shadows it.
                                && !self.local_declared_types.contains(&ident)
                                && types::map_primitive_type(&ident).is_none();
                        if looks_like_type_param {
                            out.insert(ident);
                        }
                    }
                }
                for seg in &tp.path.segments {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        for arg in &args.args {
                            if let syn::GenericArgument::Type(inner) = arg {
                                self.collect_assoc_binding_type_param_candidates_in_type(
                                    inner, out,
                                );
                            }
                        }
                    }
                }
            }
            syn::Type::Reference(r) => {
                self.collect_assoc_binding_type_param_candidates_in_type(&r.elem, out)
            }
            syn::Type::Ptr(p) => {
                self.collect_assoc_binding_type_param_candidates_in_type(&p.elem, out)
            }
            syn::Type::Slice(s) => {
                self.collect_assoc_binding_type_param_candidates_in_type(&s.elem, out)
            }
            syn::Type::Array(a) => {
                self.collect_assoc_binding_type_param_candidates_in_type(&a.elem, out)
            }
            syn::Type::Tuple(t) => {
                for elem in &t.elems {
                    self.collect_assoc_binding_type_param_candidates_in_type(elem, out);
                }
            }
            syn::Type::Paren(p) => {
                self.collect_assoc_binding_type_param_candidates_in_type(&p.elem, out)
            }
            syn::Type::Group(g) => {
                self.collect_assoc_binding_type_param_candidates_in_type(&g.elem, out)
            }
            _ => {}
        }
    }

    /// Single-binding if-let pattern → (binding ident, Result/Option unwrap
    /// method) for scope-independent ctor-hint spelling.
    fn single_iflet_binding_and_unwrap_method(
        pat: &syn::Pat,
    ) -> Option<(String, &'static str)> {
        let syn::Pat::TupleStruct(ts) = pat else {
            return None;
        };
        let method = match ts.path.segments.last()?.ident.to_string().as_str() {
            "Err" => "unwrap_err",
            "Ok" | "Some" => "unwrap",
            _ => return None,
        };
        if ts.elems.len() != 1 {
            return None;
        }
        let syn::Pat::Ident(ident) = &ts.elems[0] else {
            return None;
        };
        Some((ident.ident.to_string(), method))
    }

    pub(super) fn collect_constructor_arg_cpp_strings(
        &self,
        expr: &syn::Expr,
        out: &mut HashMap<String, String>,
        if_let_unwrap_method: Option<&'static str>,
    ) {
        let expr = self.peel_paren_group_expr(expr);

        if let Some((ctor_name, ctor_arg)) = self.extract_constructor_call_expr(expr) {
            out.entry(ctor_name).or_insert_with(|| {
                self.emit_constructor_hint_arg_cpp(ctor_arg, if_let_unwrap_method)
            });
        }

        match expr {
            syn::Expr::If(if_expr) => {
                // For an if-LET arm, record binding → scrutinee so ctor-hint
                // args from the THEN branch are spelled scope-independently
                // (`(<scrutinee>).unwrap_err()`), not as the branch-local
                // `_iflet.unwrap_err()` — nested if-let chains re-bind
                // `_iflet` per branch, so a spelling hoisted into the shared
                // type hint would silently change meaning across branches
                // (see `iflet_hint_scrutinees`).
                let mut pushed = false;
                if let syn::Expr::Let(let_expr) = &*if_expr.cond
                    && let Some((binding, method)) =
                        Self::single_iflet_binding_and_unwrap_method(&let_expr.pat)
                {
                    let scrutinee_cpp = self.emit_expr_maybe_move(&let_expr.expr);
                    if !scrutinee_cpp.is_empty() {
                        self.iflet_hint_scrutinees.borrow_mut().push((
                            binding,
                            scrutinee_cpp,
                            method,
                        ));
                        pushed = true;
                    }
                }
                if let Some(then_expr) = self.extract_single_expr_from_block(&if_expr.then_branch) {
                    self.collect_constructor_arg_cpp_strings(then_expr, out, if_let_unwrap_method);
                }
                if pushed {
                    self.iflet_hint_scrutinees.borrow_mut().pop();
                }
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    self.collect_constructor_arg_cpp_strings(else_expr, out, if_let_unwrap_method);
                }
            }
            syn::Expr::Match(match_expr) => {
                for arm in &match_expr.arms {
                    self.collect_constructor_arg_cpp_strings(&arm.body, out, if_let_unwrap_method);
                }
            }
            syn::Expr::Reference(r) => {
                self.collect_constructor_arg_cpp_strings(&r.expr, out, if_let_unwrap_method)
            }
            syn::Expr::Paren(p) => {
                self.collect_constructor_arg_cpp_strings(&p.expr, out, if_let_unwrap_method)
            }
            syn::Expr::Group(g) => {
                self.collect_constructor_arg_cpp_strings(&g.expr, out, if_let_unwrap_method)
            }
            syn::Expr::Call(call) => {
                self.collect_constructor_arg_cpp_strings(&call.func, out, if_let_unwrap_method);
                for arg in &call.args {
                    self.collect_constructor_arg_cpp_strings(arg, out, if_let_unwrap_method);
                }
            }
            syn::Expr::Try(try_expr) => {
                self.collect_constructor_arg_cpp_strings(&try_expr.expr, out, if_let_unwrap_method)
            }
            syn::Expr::Block(block_expr) => {
                for stmt in &block_expr.block.stmts {
                    self.collect_constructor_arg_cpp_strings_in_stmt(
                        stmt,
                        out,
                        if_let_unwrap_method,
                    );
                }
            }
            syn::Expr::Closure(closure) => {
                self.collect_constructor_arg_cpp_strings(&closure.body, out, if_let_unwrap_method)
            }
            _ => {}
        }
    }

    pub(super) fn collect_constructor_arg_cpp_strings_in_stmt(
        &self,
        stmt: &syn::Stmt,
        out: &mut HashMap<String, String>,
        if_let_unwrap_method: Option<&'static str>,
    ) {
        match stmt {
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.collect_constructor_arg_cpp_strings(&init.expr, out, if_let_unwrap_method);
                }
            }
            syn::Stmt::Expr(expr, _) => {
                self.collect_constructor_arg_cpp_strings(expr, out, if_let_unwrap_method);
            }
            syn::Stmt::Item(_) | syn::Stmt::Macro(_) => {}
        }
    }

    /// Idents a pattern binds BY VALUE (no `ref`, not behind `&`) —
    /// move-on-use candidates when the match consumes its scrutinee.
    pub(super) fn collect_pattern_value_binding_names(
        &self,
        pat: &syn::Pat,
        out: &mut HashSet<String>,
    ) {
        match pat {
            syn::Pat::Ident(pi) => {
                if pi.by_ref.is_none() && pi.ident != "_" {
                    let name = pi.ident.to_string();
                    if !self.pattern_ident_is_const_value(&name) {
                        out.insert(name);
                    }
                }
                if let Some((_, subpat)) = &pi.subpat {
                    self.collect_pattern_value_binding_names(subpat, out);
                }
            }
            syn::Pat::Tuple(tuple_pat) => {
                for elem in &tuple_pat.elems {
                    self.collect_pattern_value_binding_names(elem, out);
                }
            }
            syn::Pat::TupleStruct(ts) => {
                for elem in &ts.elems {
                    self.collect_pattern_value_binding_names(elem, out);
                }
            }
            syn::Pat::Struct(ps) => {
                for field in &ps.fields {
                    self.collect_pattern_value_binding_names(&field.pat, out);
                }
            }
            syn::Pat::Or(or_pat) => {
                for case in &or_pat.cases {
                    self.collect_pattern_value_binding_names(case, out);
                }
            }
            syn::Pat::Type(pt) => self.collect_pattern_value_binding_names(&pt.pat, out),
            syn::Pat::Paren(p) => self.collect_pattern_value_binding_names(&p.pat, out),
            // Pat::Reference / Pat::Slice bindings bind borrowed — skip.
            _ => {}
        }
    }

    pub(super) fn collect_pattern_ref_binding_names(&self, pat: &syn::Pat, out: &mut HashSet<String>) {
        match pat {
            syn::Pat::Ident(pi) => {
                if pi.ident != "_" {
                    let name = pi.ident.to_string();
                    if !self.pattern_ident_is_const_value(&name) {
                        out.insert(name);
                    }
                }
                if let Some((_, subpat)) = &pi.subpat {
                    self.collect_pattern_ref_binding_names(subpat, out);
                }
            }
            syn::Pat::Tuple(tuple_pat) => {
                for elem in &tuple_pat.elems {
                    self.collect_pattern_ref_binding_names(elem, out);
                }
            }
            syn::Pat::TupleStruct(ts) => {
                for elem in &ts.elems {
                    self.collect_pattern_ref_binding_names(elem, out);
                }
            }
            syn::Pat::Struct(ps) => {
                for field in &ps.fields {
                    self.collect_pattern_ref_binding_names(&field.pat, out);
                }
            }
            syn::Pat::Reference(r) => self.collect_pattern_ref_binding_names(&r.pat, out),
            syn::Pat::Type(pt) => self.collect_pattern_ref_binding_names(&pt.pat, out),
            syn::Pat::Paren(p) => self.collect_pattern_ref_binding_names(&p.pat, out),
            syn::Pat::Slice(slice) => {
                for elem in &slice.elems {
                    self.collect_pattern_ref_binding_names(elem, out);
                }
            }
            syn::Pat::Or(or_pat) => {
                for case in &or_pat.cases {
                    self.collect_pattern_ref_binding_names(case, out);
                }
            }
            _ => {}
        }
    }

    /// Names bound with an explicit `ref` in a pattern (`Content::Seq(ref v)`)
    /// ONLY — unlike collect_pattern_ref_binding_names, which collects every
    /// binding for borrowed-iterable for-loops. A `ref` binding's C++ carrier
    /// is a reference into the scrutinee; consuming-arg emission must not
    /// `std::move` it (that moves the REFERENT where Rust copies the borrow —
    /// serde's `Content::Seq(ref v)` arm passed as_slice(std::move(v))).
    pub(super) fn collect_pattern_explicit_ref_binding_names(
        &self,
        pat: &syn::Pat,
        out: &mut HashSet<String>,
    ) {
        match pat {
            syn::Pat::Ident(pi) => {
                if pi.by_ref.is_some() {
                    out.insert(pi.ident.to_string());
                }
                if let Some((_, subpat)) = &pi.subpat {
                    self.collect_pattern_explicit_ref_binding_names(subpat, out);
                }
            }
            syn::Pat::Tuple(t) => {
                for elem in &t.elems {
                    self.collect_pattern_explicit_ref_binding_names(elem, out);
                }
            }
            syn::Pat::TupleStruct(ts) => {
                for elem in &ts.elems {
                    self.collect_pattern_explicit_ref_binding_names(elem, out);
                }
            }
            syn::Pat::Struct(ps) => {
                for field in &ps.fields {
                    self.collect_pattern_explicit_ref_binding_names(&field.pat, out);
                }
            }
            syn::Pat::Reference(r) => self.collect_pattern_explicit_ref_binding_names(&r.pat, out),
            syn::Pat::Type(pt) => self.collect_pattern_explicit_ref_binding_names(&pt.pat, out),
            syn::Pat::Paren(pp) => self.collect_pattern_explicit_ref_binding_names(&pp.pat, out),
            syn::Pat::Slice(sl) => {
                for elem in &sl.elems {
                    self.collect_pattern_explicit_ref_binding_names(elem, out);
                }
            }
            syn::Pat::Or(or_pat) => {
                for case in &or_pat.cases {
                    self.collect_pattern_explicit_ref_binding_names(case, out);
                }
            }
            _ => {}
        }
    }

    pub(super) fn collect_pattern_binding_names(&self, pat: &syn::Pat, out: &mut HashSet<String>) {
        match pat {
            syn::Pat::Ident(pi) => {
                let name = pi.ident.to_string();
                if !self.pattern_ident_is_const_value(&name) {
                    out.insert(name);
                }
                if let Some((_, subpat)) = &pi.subpat {
                    self.collect_pattern_binding_names(subpat, out);
                }
            }
            syn::Pat::Tuple(tuple_pat) => {
                for elem in &tuple_pat.elems {
                    self.collect_pattern_binding_names(elem, out);
                }
            }
            syn::Pat::TupleStruct(ts) => {
                for elem in &ts.elems {
                    self.collect_pattern_binding_names(elem, out);
                }
            }
            syn::Pat::Struct(ps) => {
                for field in &ps.fields {
                    self.collect_pattern_binding_names(&field.pat, out);
                }
            }
            syn::Pat::Reference(r) => self.collect_pattern_binding_names(&r.pat, out),
            syn::Pat::Type(pt) => self.collect_pattern_binding_names(&pt.pat, out),
            syn::Pat::Paren(p) => self.collect_pattern_binding_names(&p.pat, out),
            syn::Pat::Slice(slice) => {
                for elem in &slice.elems {
                    self.collect_pattern_binding_names(elem, out);
                }
            }
            syn::Pat::Or(or_pat) => {
                for case in &or_pat.cases {
                    self.collect_pattern_binding_names(case, out);
                }
            }
            _ => {}
        }
    }

    pub(super) fn collect_scan_char_item_param_scope(&self, closure: &syn::ExprClosure) -> HashSet<String> {
        let mut names = HashSet::new();
        if let Some(item_pat) = closure.inputs.iter().nth(1) {
            self.collect_closure_param_names_from_pat(item_pat, &mut names);
        }
        names.retain(|name| name != "_");
        names
    }

    pub(super) fn collect_type_param_substitutions_from_expected_match(
        &self,
        template_ty: &syn::Type,
        concrete_ty: &syn::Type,
        substitutions: &mut HashMap<String, syn::Type>,
    ) -> bool {
        let template_ty = self.peel_reference_paren_group_type(template_ty);
        let concrete_ty = self.peel_reference_paren_group_type(concrete_ty);

        if let syn::Type::Path(template_tp) = template_ty
            && template_tp.qself.is_none()
            && template_tp.path.segments.len() == 1
            && let Some(seg) = template_tp.path.segments.first()
            && matches!(seg.arguments, syn::PathArguments::None)
        {
            let param_name = seg.ident.to_string();
            if self.ident_looks_like_unresolved_type_param_name(&param_name) {
                if !self.type_is_concrete_hint_candidate(concrete_ty) {
                    return false;
                }
                if let Some(existing) = substitutions.get(&param_name) {
                    return Self::types_equivalent_by_tokens(existing, concrete_ty);
                }
                substitutions.insert(param_name, concrete_ty.clone());
                return true;
            }
        }

        match (template_ty, concrete_ty) {
            (syn::Type::Path(template_tp), syn::Type::Path(concrete_tp)) => {
                let Some(template_last) = template_tp.path.segments.last() else {
                    return false;
                };
                let Some(concrete_last) = concrete_tp.path.segments.last() else {
                    return false;
                };
                if template_last.ident != concrete_last.ident {
                    return false;
                }
                match (&template_last.arguments, &concrete_last.arguments) {
                    (
                        syn::PathArguments::AngleBracketed(template_args),
                        syn::PathArguments::AngleBracketed(concrete_args),
                    ) => {
                        let mut template_type_args = template_args.args.iter().filter_map(|arg| {
                            if let syn::GenericArgument::Type(ty) = arg {
                                Some(ty)
                            } else {
                                None
                            }
                        });
                        let mut concrete_type_args = concrete_args.args.iter().filter_map(|arg| {
                            if let syn::GenericArgument::Type(ty) = arg {
                                Some(ty)
                            } else {
                                None
                            }
                        });
                        loop {
                            match (template_type_args.next(), concrete_type_args.next()) {
                                (Some(template_inner), Some(concrete_inner)) => {
                                    if !self.collect_type_param_substitutions_from_expected_match(
                                        template_inner,
                                        concrete_inner,
                                        substitutions,
                                    ) {
                                        return false;
                                    }
                                }
                                (None, None) => break,
                                _ => return false,
                            }
                        }
                        true
                    }
                    (syn::PathArguments::None, syn::PathArguments::None) => true,
                    _ => false,
                }
            }
            (syn::Type::Reference(template_ref), syn::Type::Reference(concrete_ref)) => self
                .collect_type_param_substitutions_from_expected_match(
                    &template_ref.elem,
                    &concrete_ref.elem,
                    substitutions,
                ),
            (syn::Type::Ptr(template_ptr), syn::Type::Ptr(concrete_ptr)) => self
                .collect_type_param_substitutions_from_expected_match(
                    &template_ptr.elem,
                    &concrete_ptr.elem,
                    substitutions,
                ),
            (syn::Type::Tuple(template_tuple), syn::Type::Tuple(concrete_tuple)) => {
                if template_tuple.elems.len() != concrete_tuple.elems.len() {
                    return false;
                }
                for (template_elem, concrete_elem) in
                    template_tuple.elems.iter().zip(concrete_tuple.elems.iter())
                {
                    if !self.collect_type_param_substitutions_from_expected_match(
                        template_elem,
                        concrete_elem,
                        substitutions,
                    ) {
                        return false;
                    }
                }
                true
            }
            (syn::Type::Array(template_arr), syn::Type::Array(concrete_arr)) => self
                .collect_type_param_substitutions_from_expected_match(
                    &template_arr.elem,
                    &concrete_arr.elem,
                    substitutions,
                ),
            (syn::Type::Slice(template_slice), syn::Type::Slice(concrete_slice)) => self
                .collect_type_param_substitutions_from_expected_match(
                    &template_slice.elem,
                    &concrete_slice.elem,
                    substitutions,
                ),
            (syn::Type::Paren(template_paren), _) => self
                .collect_type_param_substitutions_from_expected_match(
                    &template_paren.elem,
                    concrete_ty,
                    substitutions,
                ),
            (syn::Type::Group(template_group), _) => self
                .collect_type_param_substitutions_from_expected_match(
                    &template_group.elem,
                    concrete_ty,
                    substitutions,
                ),
            (_, syn::Type::Paren(concrete_paren)) => self
                .collect_type_param_substitutions_from_expected_match(
                    template_ty,
                    &concrete_paren.elem,
                    substitutions,
                ),
            (_, syn::Type::Group(concrete_group)) => self
                .collect_type_param_substitutions_from_expected_match(
                    template_ty,
                    &concrete_group.elem,
                    substitutions,
                ),
            _ => false,
        }
    }

    pub(super) fn collect_target_supports_from_iter(&self, cpp_type: &str) -> bool {
        let canonical = self.canonical_into_target_cpp_type(cpp_type);
        if canonical.starts_with("std::span<")
            || canonical == "std::string_view"
            || canonical.starts_with("std::basic_string_view<")
        {
            return false;
        }
        true
    }

    pub(super) fn collect_c_like_owner_tail_candidates_from_type(
        &self,
        ty: &syn::Type,
        out: &mut Vec<String>,
    ) {
        let ty = self.peel_paren_group_type(ty);
        match ty {
            syn::Type::Reference(reference) => {
                self.collect_c_like_owner_tail_candidates_from_type(&reference.elem, out);
            }
            syn::Type::Tuple(tuple) => {
                for elem in &tuple.elems {
                    self.collect_c_like_owner_tail_candidates_from_type(elem, out);
                }
            }
            syn::Type::Path(type_path) => {
                if let Some(last) = type_path.path.segments.last() {
                    let owner_tail = escape_cpp_keyword(&last.ident.to_string());
                    if !owner_tail.is_empty() {
                        out.push(owner_tail);
                    }
                    if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                        for arg in &args.args {
                            if let syn::GenericArgument::Type(inner_ty) = arg {
                                self.collect_c_like_owner_tail_candidates_from_type(inner_ty, out);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn collect_omitted_generic_bindings_from_field_types(
        &self,
        declared_ty: &syn::Type,
        inferred_expr_ty: &syn::Type,
        params: &[String],
        param_kinds: Option<&Vec<GenericParamKind>>,
        inferred_args: &mut [Option<String>],
        conflict: &mut bool,
    ) {
        if *conflict {
            return;
        }
        let declared_ty = self.peel_reference_paren_group_type(declared_ty);
        let inferred_expr_ty = self.peel_reference_paren_group_type(inferred_expr_ty);

        if let Some((declared_pointee, _)) =
            self.extract_pointer_pointee_info_from_type(declared_ty)
            && let Some((inferred_pointee, _)) =
                self.extract_pointer_pointee_info_from_type(inferred_expr_ty)
        {
            self.collect_omitted_generic_bindings_from_field_types(
                &declared_pointee,
                &inferred_pointee,
                params,
                param_kinds,
                inferred_args,
                conflict,
            );
            return;
        }

        if let syn::Type::Path(tp) = declared_ty
            && tp.qself.is_none()
            && tp.path.segments.len() == 1
        {
            let seg = &tp.path.segments[0];
            if matches!(seg.arguments, syn::PathArguments::None) {
                let param_name = seg.ident.to_string();
                if let Some(idx) = params.iter().position(|p| p == &param_name) {
                    let is_type_param = param_kinds
                        .and_then(|kinds| kinds.get(idx))
                        .is_none_or(|kind| matches!(kind, GenericParamKind::Type));
                    if !is_type_param {
                        return;
                    }
                    // Guard against value-name leakage from local inference
                    // (e.g. recovering `ptr` as a template argument instead of
                    // the intended in-scope type parameter).
                    let inferred_expr_ty = self.peel_reference_paren_group_type(inferred_expr_ty);
                    if let syn::Type::Path(inferred_path) = inferred_expr_ty
                        && inferred_path.qself.is_none()
                        && inferred_path.path.segments.len() == 1
                        && matches!(
                            inferred_path.path.segments[0].arguments,
                            syn::PathArguments::None
                        )
                    {
                        let inferred_ident = inferred_path.path.segments[0].ident.to_string();
                        if self.lookup_local_binding_type(&inferred_ident).is_some()
                            || self
                                .lookup_local_placeholder_type_hint(&inferred_ident)
                                .is_some()
                        {
                            return;
                        }
                    }
                    let candidate = self.map_type(inferred_expr_ty);
                    match inferred_args.get_mut(idx) {
                        Some(slot @ None) => *slot = Some(candidate),
                        Some(Some(existing)) if existing == &candidate => {}
                        Some(Some(_)) => *conflict = true,
                        None => {}
                    }
                    return;
                }
            }
        }

        match (declared_ty, inferred_expr_ty) {
            (syn::Type::Path(lhs), syn::Type::Path(rhs)) => {
                let matched_shape = if lhs.qself.is_none() && rhs.qself.is_none() {
                    if lhs.path.segments.len() == rhs.path.segments.len()
                        && !lhs.path.segments.is_empty()
                    {
                        lhs.path
                            .segments
                            .iter()
                            .zip(rhs.path.segments.iter())
                            .all(|(l, r)| l.ident == r.ident)
                    } else {
                        lhs.path
                            .segments
                            .last()
                            .zip(rhs.path.segments.last())
                            .is_some_and(|(l, r)| l.ident == r.ident)
                    }
                } else {
                    false
                };
                if !matched_shape {
                    return;
                }

                if lhs.path.segments.len() == rhs.path.segments.len() {
                    for (lhs_seg, rhs_seg) in lhs.path.segments.iter().zip(rhs.path.segments.iter())
                    {
                        if let (
                            syn::PathArguments::AngleBracketed(lhs_args),
                            syn::PathArguments::AngleBracketed(rhs_args),
                        ) = (&lhs_seg.arguments, &rhs_seg.arguments)
                        {
                            self.collect_omitted_generic_bindings_from_angle_args(
                                lhs_args,
                                rhs_args,
                                params,
                                param_kinds,
                                inferred_args,
                                conflict,
                            );
                        }
                        if *conflict {
                            return;
                        }
                    }
                } else if let (Some(lhs_last), Some(rhs_last)) =
                    (lhs.path.segments.last(), rhs.path.segments.last())
                {
                    if let (
                        syn::PathArguments::AngleBracketed(lhs_args),
                        syn::PathArguments::AngleBracketed(rhs_args),
                    ) = (&lhs_last.arguments, &rhs_last.arguments)
                    {
                        self.collect_omitted_generic_bindings_from_angle_args(
                            lhs_args,
                            rhs_args,
                            params,
                            param_kinds,
                            inferred_args,
                            conflict,
                        );
                    }
                }
            }
            (syn::Type::Ptr(lhs), syn::Type::Ptr(rhs)) => {
                self.collect_omitted_generic_bindings_from_field_types(
                    &lhs.elem,
                    &rhs.elem,
                    params,
                    param_kinds,
                    inferred_args,
                    conflict,
                );
            }
            (syn::Type::Reference(lhs), syn::Type::Reference(rhs)) => {
                self.collect_omitted_generic_bindings_from_field_types(
                    &lhs.elem,
                    &rhs.elem,
                    params,
                    param_kinds,
                    inferred_args,
                    conflict,
                );
            }
            (syn::Type::Array(lhs), syn::Type::Array(rhs)) => {
                self.collect_omitted_generic_bindings_from_field_types(
                    &lhs.elem,
                    &rhs.elem,
                    params,
                    param_kinds,
                    inferred_args,
                    conflict,
                );
            }
            (syn::Type::Slice(lhs), syn::Type::Slice(rhs)) => {
                self.collect_omitted_generic_bindings_from_field_types(
                    &lhs.elem,
                    &rhs.elem,
                    params,
                    param_kinds,
                    inferred_args,
                    conflict,
                );
            }
            (syn::Type::Tuple(lhs), syn::Type::Tuple(rhs)) => {
                for (lhs_elem, rhs_elem) in lhs.elems.iter().zip(rhs.elems.iter()) {
                    self.collect_omitted_generic_bindings_from_field_types(
                        lhs_elem,
                        rhs_elem,
                        params,
                        param_kinds,
                        inferred_args,
                        conflict,
                    );
                    if *conflict {
                        return;
                    }
                }
            }
            (syn::Type::Paren(lhs), rhs) => {
                self.collect_omitted_generic_bindings_from_field_types(
                    &lhs.elem,
                    rhs,
                    params,
                    param_kinds,
                    inferred_args,
                    conflict,
                );
            }
            (syn::Type::Group(lhs), rhs) => {
                self.collect_omitted_generic_bindings_from_field_types(
                    &lhs.elem,
                    rhs,
                    params,
                    param_kinds,
                    inferred_args,
                    conflict,
                );
            }
            (lhs, syn::Type::Paren(rhs)) => {
                self.collect_omitted_generic_bindings_from_field_types(
                    lhs,
                    &rhs.elem,
                    params,
                    param_kinds,
                    inferred_args,
                    conflict,
                );
            }
            (lhs, syn::Type::Group(rhs)) => {
                self.collect_omitted_generic_bindings_from_field_types(
                    lhs,
                    &rhs.elem,
                    params,
                    param_kinds,
                    inferred_args,
                    conflict,
                );
            }
            _ => {}
        }
    }

    pub(super) fn collect_omitted_generic_bindings_from_angle_args(
        &self,
        lhs_args: &syn::AngleBracketedGenericArguments,
        rhs_args: &syn::AngleBracketedGenericArguments,
        params: &[String],
        param_kinds: Option<&Vec<GenericParamKind>>,
        inferred_args: &mut [Option<String>],
        conflict: &mut bool,
    ) {
        let lhs_relevant: Vec<&syn::GenericArgument> = lhs_args
            .args
            .iter()
            .filter(|arg| {
                matches!(
                    arg,
                    syn::GenericArgument::Type(_) | syn::GenericArgument::Const(_)
                )
            })
            .collect();
        let rhs_relevant: Vec<&syn::GenericArgument> = rhs_args
            .args
            .iter()
            .filter(|arg| {
                matches!(
                    arg,
                    syn::GenericArgument::Type(_) | syn::GenericArgument::Const(_)
                )
            })
            .collect();
        for (lhs_arg, rhs_arg) in lhs_relevant.into_iter().zip(rhs_relevant.into_iter()) {
            match (lhs_arg, rhs_arg) {
                (syn::GenericArgument::Type(lhs_inner), syn::GenericArgument::Type(rhs_inner)) => {
                    self.collect_omitted_generic_bindings_from_field_types(
                        lhs_inner,
                        rhs_inner,
                        params,
                        param_kinds,
                        inferred_args,
                        conflict,
                    );
                    if *conflict {
                        return;
                    }
                }
                (
                    syn::GenericArgument::Const(lhs_const),
                    syn::GenericArgument::Const(rhs_const),
                ) => {
                    let syn::Expr::Path(lhs_path) = lhs_const else {
                        continue;
                    };
                    if lhs_path.path.segments.len() != 1 {
                        continue;
                    }
                    let lhs_ident = lhs_path.path.segments[0].ident.to_string();
                    let Some(idx) = params.iter().position(|p| p == &lhs_ident) else {
                        continue;
                    };
                    let is_const_param = param_kinds
                        .and_then(|kinds| kinds.get(idx))
                        .is_some_and(|kind| matches!(kind, GenericParamKind::Const));
                    if !is_const_param {
                        continue;
                    }
                    let candidate = self.emit_expr_to_string(rhs_const);
                    match inferred_args.get_mut(idx) {
                        Some(slot @ None) => *slot = Some(candidate),
                        Some(Some(existing)) if existing == &candidate => {}
                        Some(Some(_)) => *conflict = true,
                        None => {}
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn collect_move_closure_capture_cpp_names(&self, closure: &syn::ExprClosure) -> Vec<String> {
        let mut referenced_names = HashSet::new();
        self.collect_path_local_names_for_move_capture(&closure.body, &mut referenced_names);

        let mut param_names = HashSet::new();
        for input in &closure.inputs {
            self.collect_closure_param_names_from_pat(input, &mut param_names);
        }

        let mut local_binding_names = HashSet::new();
        self.collect_local_binding_names_in_expr_for_move_capture(
            &closure.body,
            &mut local_binding_names,
        );

        let mut cpp_names = Vec::new();
        for rust_name in referenced_names {
            if param_names.contains(&rust_name) || local_binding_names.contains(&rust_name) {
                continue;
            }
            if rust_name == "self"
                && self.current_self_path_override().is_none()
                && !self.self_receiver_ref_scopes.is_empty()
            {
                continue;
            }
            // The method receiver is emitted as `self_`. A move-closure capturing
            // `self` must name it `self_` in the init-capture — otherwise
            // `lookup_local_binding_cpp_name` returns the verbatim Rust `self`,
            // emitting `self = std::move(self)` whose RHS is undeclared in the
            // lambda ("use of undeclared identifier 'self'"). The body already
            // refers to the receiver as `self_`.
            if rust_name == "self" {
                cpp_names.push("self_".to_string());
                continue;
            }
            let Some(cpp_name) = self.lookup_local_binding_cpp_name(&rust_name) else {
                continue;
            };
            if !is_simple_cpp_identifier(&cpp_name) {
                continue;
            }
            cpp_names.push(cpp_name.to_string());
        }
        cpp_names.sort();
        cpp_names.dedup();
        cpp_names
    }

    pub(super) fn collect_path_local_names_for_move_capture(
        &self,
        expr: &syn::Expr,
        out: &mut HashSet<String>,
    ) {
        let expr = self.peel_paren_group_expr(expr);
        match expr {
            syn::Expr::Path(path_expr) => {
                if path_expr.path.segments.len() == 1 {
                    out.insert(path_expr.path.segments[0].ident.to_string());
                }
            }
            syn::Expr::Call(call) => {
                self.collect_path_local_names_for_move_capture(&call.func, out);
                for arg in &call.args {
                    self.collect_path_local_names_for_move_capture(arg, out);
                }
            }
            syn::Expr::MethodCall(method_call) => {
                self.collect_path_local_names_for_move_capture(&method_call.receiver, out);
                for arg in &method_call.args {
                    self.collect_path_local_names_for_move_capture(arg, out);
                }
            }
            syn::Expr::Binary(bin) => {
                self.collect_path_local_names_for_move_capture(&bin.left, out);
                self.collect_path_local_names_for_move_capture(&bin.right, out);
            }
            syn::Expr::Unary(unary) => {
                self.collect_path_local_names_for_move_capture(&unary.expr, out);
            }
            syn::Expr::Reference(reference) => {
                self.collect_path_local_names_for_move_capture(&reference.expr, out);
            }
            syn::Expr::Assign(assign) => {
                self.collect_path_local_names_for_move_capture(&assign.left, out);
                self.collect_path_local_names_for_move_capture(&assign.right, out);
            }
            syn::Expr::Let(let_expr) => {
                self.collect_path_local_names_for_move_capture(&let_expr.expr, out);
            }
            syn::Expr::Field(field) => {
                self.collect_path_local_names_for_move_capture(&field.base, out);
            }
            syn::Expr::Index(index) => {
                self.collect_path_local_names_for_move_capture(&index.expr, out);
                self.collect_path_local_names_for_move_capture(&index.index, out);
            }
            syn::Expr::Cast(cast_expr) => {
                self.collect_path_local_names_for_move_capture(&cast_expr.expr, out);
            }
            syn::Expr::Try(try_expr) => {
                self.collect_path_local_names_for_move_capture(&try_expr.expr, out);
            }
            syn::Expr::Await(await_expr) => {
                self.collect_path_local_names_for_move_capture(&await_expr.base, out);
            }
            syn::Expr::Block(block) => {
                for stmt in &block.block.stmts {
                    self.collect_path_local_names_in_stmt_for_move_capture(stmt, out);
                }
            }
            syn::Expr::If(if_expr) => {
                self.collect_path_local_names_for_move_capture(&if_expr.cond, out);
                for stmt in &if_expr.then_branch.stmts {
                    self.collect_path_local_names_in_stmt_for_move_capture(stmt, out);
                }
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    self.collect_path_local_names_for_move_capture(else_expr, out);
                }
            }
            syn::Expr::Match(match_expr) => {
                self.collect_path_local_names_for_move_capture(&match_expr.expr, out);
                for arm in &match_expr.arms {
                    if let Some((_, guard)) = &arm.guard {
                        self.collect_path_local_names_for_move_capture(guard, out);
                    }
                    self.collect_path_local_names_for_move_capture(&arm.body, out);
                }
            }
            syn::Expr::While(while_expr) => {
                self.collect_path_local_names_for_move_capture(&while_expr.cond, out);
                for stmt in &while_expr.body.stmts {
                    self.collect_path_local_names_in_stmt_for_move_capture(stmt, out);
                }
            }
            syn::Expr::Loop(loop_expr) => {
                for stmt in &loop_expr.body.stmts {
                    self.collect_path_local_names_in_stmt_for_move_capture(stmt, out);
                }
            }
            syn::Expr::ForLoop(for_loop) => {
                self.collect_path_local_names_for_move_capture(&for_loop.expr, out);
                for stmt in &for_loop.body.stmts {
                    self.collect_path_local_names_in_stmt_for_move_capture(stmt, out);
                }
            }
            syn::Expr::Tuple(tuple) => {
                for elem in &tuple.elems {
                    self.collect_path_local_names_for_move_capture(elem, out);
                }
            }
            syn::Expr::Array(array) => {
                for elem in &array.elems {
                    self.collect_path_local_names_for_move_capture(elem, out);
                }
            }
            syn::Expr::Struct(struct_expr) => {
                for field in &struct_expr.fields {
                    self.collect_path_local_names_for_move_capture(&field.expr, out);
                }
                if let Some(rest) = &struct_expr.rest {
                    self.collect_path_local_names_for_move_capture(rest, out);
                }
            }
            syn::Expr::Break(brk) => {
                if let Some(value) = &brk.expr {
                    self.collect_path_local_names_for_move_capture(value, out);
                }
            }
            syn::Expr::Return(ret) => {
                if let Some(value) = &ret.expr {
                    self.collect_path_local_names_for_move_capture(value, out);
                }
            }
            syn::Expr::Closure(closure) => {
                self.collect_path_local_names_for_move_capture(&closure.body, out);
            }
            syn::Expr::Unsafe(unsafe_expr) => {
                for stmt in &unsafe_expr.block.stmts {
                    self.collect_path_local_names_in_stmt_for_move_capture(stmt, out);
                }
            }
            _ => {}
        }
    }

    pub(super) fn collect_path_local_names_in_stmt_for_move_capture(
        &self,
        stmt: &syn::Stmt,
        out: &mut HashSet<String>,
    ) {
        match stmt {
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.collect_path_local_names_for_move_capture(&init.expr, out);
                }
            }
            syn::Stmt::Expr(expr, _) => self.collect_path_local_names_for_move_capture(expr, out),
            syn::Stmt::Item(_) | syn::Stmt::Macro(_) => {}
        }
    }

    pub(super) fn collect_local_binding_names_in_expr_for_move_capture(
        &self,
        expr: &syn::Expr,
        out: &mut HashSet<String>,
    ) {
        let expr = self.peel_paren_group_expr(expr);
        match expr {
            syn::Expr::Let(let_expr) => {
                self.collect_closure_param_names_from_pat(&let_expr.pat, out);
                self.collect_local_binding_names_in_expr_for_move_capture(&let_expr.expr, out);
            }
            syn::Expr::Block(block) => {
                for stmt in &block.block.stmts {
                    self.collect_local_binding_names_in_stmt_for_move_capture(stmt, out);
                }
            }
            syn::Expr::If(if_expr) => {
                self.collect_local_binding_names_in_expr_for_move_capture(&if_expr.cond, out);
                for stmt in &if_expr.then_branch.stmts {
                    self.collect_local_binding_names_in_stmt_for_move_capture(stmt, out);
                }
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    self.collect_local_binding_names_in_expr_for_move_capture(else_expr, out);
                }
            }
            syn::Expr::Match(match_expr) => {
                self.collect_local_binding_names_in_expr_for_move_capture(&match_expr.expr, out);
                for arm in &match_expr.arms {
                    self.collect_closure_param_names_from_pat(&arm.pat, out);
                    if let Some((_, guard)) = &arm.guard {
                        self.collect_local_binding_names_in_expr_for_move_capture(guard, out);
                    }
                    self.collect_local_binding_names_in_expr_for_move_capture(&arm.body, out);
                }
            }
            syn::Expr::While(while_expr) => {
                self.collect_local_binding_names_in_expr_for_move_capture(&while_expr.cond, out);
                for stmt in &while_expr.body.stmts {
                    self.collect_local_binding_names_in_stmt_for_move_capture(stmt, out);
                }
            }
            syn::Expr::Loop(loop_expr) => {
                for stmt in &loop_expr.body.stmts {
                    self.collect_local_binding_names_in_stmt_for_move_capture(stmt, out);
                }
            }
            syn::Expr::ForLoop(for_loop) => {
                self.collect_closure_param_names_from_pat(&for_loop.pat, out);
                self.collect_local_binding_names_in_expr_for_move_capture(&for_loop.expr, out);
                for stmt in &for_loop.body.stmts {
                    self.collect_local_binding_names_in_stmt_for_move_capture(stmt, out);
                }
            }
            syn::Expr::Closure(closure) => {
                for input in &closure.inputs {
                    self.collect_closure_param_names_from_pat(input, out);
                }
                self.collect_local_binding_names_in_expr_for_move_capture(&closure.body, out);
            }
            syn::Expr::Call(call) => {
                self.collect_local_binding_names_in_expr_for_move_capture(&call.func, out);
                for arg in &call.args {
                    self.collect_local_binding_names_in_expr_for_move_capture(arg, out);
                }
            }
            syn::Expr::MethodCall(method_call) => {
                self.collect_local_binding_names_in_expr_for_move_capture(
                    &method_call.receiver,
                    out,
                );
                for arg in &method_call.args {
                    self.collect_local_binding_names_in_expr_for_move_capture(arg, out);
                }
            }
            syn::Expr::Binary(bin) => {
                self.collect_local_binding_names_in_expr_for_move_capture(&bin.left, out);
                self.collect_local_binding_names_in_expr_for_move_capture(&bin.right, out);
            }
            syn::Expr::Unary(unary) => {
                self.collect_local_binding_names_in_expr_for_move_capture(&unary.expr, out);
            }
            syn::Expr::Reference(reference) => {
                self.collect_local_binding_names_in_expr_for_move_capture(&reference.expr, out);
            }
            syn::Expr::Assign(assign) => {
                self.collect_local_binding_names_in_expr_for_move_capture(&assign.left, out);
                self.collect_local_binding_names_in_expr_for_move_capture(&assign.right, out);
            }
            syn::Expr::Field(field) => {
                self.collect_local_binding_names_in_expr_for_move_capture(&field.base, out);
            }
            syn::Expr::Index(index) => {
                self.collect_local_binding_names_in_expr_for_move_capture(&index.expr, out);
                self.collect_local_binding_names_in_expr_for_move_capture(&index.index, out);
            }
            syn::Expr::Cast(cast_expr) => {
                self.collect_local_binding_names_in_expr_for_move_capture(&cast_expr.expr, out);
            }
            syn::Expr::Try(try_expr) => {
                self.collect_local_binding_names_in_expr_for_move_capture(&try_expr.expr, out);
            }
            syn::Expr::Await(await_expr) => {
                self.collect_local_binding_names_in_expr_for_move_capture(&await_expr.base, out);
            }
            syn::Expr::Tuple(tuple) => {
                for elem in &tuple.elems {
                    self.collect_local_binding_names_in_expr_for_move_capture(elem, out);
                }
            }
            syn::Expr::Array(array) => {
                for elem in &array.elems {
                    self.collect_local_binding_names_in_expr_for_move_capture(elem, out);
                }
            }
            syn::Expr::Struct(struct_expr) => {
                for field in &struct_expr.fields {
                    self.collect_local_binding_names_in_expr_for_move_capture(&field.expr, out);
                }
                if let Some(rest) = &struct_expr.rest {
                    self.collect_local_binding_names_in_expr_for_move_capture(rest, out);
                }
            }
            syn::Expr::Break(brk) => {
                if let Some(value) = &brk.expr {
                    self.collect_local_binding_names_in_expr_for_move_capture(value, out);
                }
            }
            syn::Expr::Return(ret) => {
                if let Some(value) = &ret.expr {
                    self.collect_local_binding_names_in_expr_for_move_capture(value, out);
                }
            }
            syn::Expr::Unsafe(unsafe_expr) => {
                for stmt in &unsafe_expr.block.stmts {
                    self.collect_local_binding_names_in_stmt_for_move_capture(stmt, out);
                }
            }
            _ => {}
        }
    }

    pub(super) fn collect_local_binding_names_in_stmt_for_move_capture(
        &self,
        stmt: &syn::Stmt,
        out: &mut HashSet<String>,
    ) {
        match stmt {
            syn::Stmt::Local(local) => {
                self.collect_closure_param_names_from_pat(&local.pat, out);
                if let Some(init) = &local.init {
                    self.collect_local_binding_names_in_expr_for_move_capture(&init.expr, out);
                }
            }
            syn::Stmt::Expr(expr, _) => {
                self.collect_local_binding_names_in_expr_for_move_capture(expr, out);
            }
            syn::Stmt::Item(_) | syn::Stmt::Macro(_) => {}
        }
    }

    pub(super) fn collect_closure_param_names_for_scope(&self, closure: &syn::ExprClosure) -> HashSet<String> {
        let mut names = HashSet::new();
        for input in &closure.inputs {
            self.collect_closure_param_names_from_pat(input, &mut names);
        }
        names
    }

    pub(super) fn collect_untyped_closure_param_names_for_scope(
        &self,
        closure: &syn::ExprClosure,
    ) -> HashSet<String> {
        let mut names = HashSet::new();
        for input in &closure.inputs {
            self.collect_untyped_closure_param_names_from_pat(input, &mut names);
        }
        names
    }

    pub(super) fn collect_untyped_closure_param_names_from_pat(
        &self,
        pat: &syn::Pat,
        out: &mut HashSet<String>,
    ) {
        match pat {
            syn::Pat::Ident(pi) => {
                if pi.ident != "_" {
                    out.insert(pi.ident.to_string());
                }
            }
            syn::Pat::Paren(p) => self.collect_untyped_closure_param_names_from_pat(&p.pat, out),
            syn::Pat::Type(_) => {}
            syn::Pat::Reference(_) => {}
            syn::Pat::Tuple(_) | syn::Pat::TupleStruct(_) | syn::Pat::Struct(_) => {}
            _ => {}
        }
    }

    pub(super) fn collect_closure_param_names_from_pat(&self, pat: &syn::Pat, out: &mut HashSet<String>) {
        match pat {
            syn::Pat::Ident(pi) => {
                if pi.ident != "_" {
                    out.insert(pi.ident.to_string());
                }
            }
            syn::Pat::Type(pt) => self.collect_closure_param_names_from_pat(&pt.pat, out),
            syn::Pat::Reference(pr) => self.collect_closure_param_names_from_pat(&pr.pat, out),
            syn::Pat::Paren(p) => self.collect_closure_param_names_from_pat(&p.pat, out),
            syn::Pat::Tuple(tuple_pat) => {
                for elem in &tuple_pat.elems {
                    self.collect_closure_param_names_from_pat(elem, out);
                }
            }
            _ => {}
        }
    }

    pub(super) fn collect_trait_bound_type_param_map(generics: &syn::Generics) -> HashMap<String, String> {
        let mut raw: HashMap<String, Option<String>> = HashMap::new();
        for param in &generics.params {
            let syn::GenericParam::Type(tp) = param else {
                continue;
            };
            let type_param_name = tp.ident.to_string();
            for bound in &tp.bounds {
                if let Some(trait_name) = Self::trait_name_from_bound(bound) {
                    Self::record_trait_bound_type_param_binding(
                        &mut raw,
                        trait_name,
                        &type_param_name,
                    );
                }
            }
        }

        if let Some(where_clause) = &generics.where_clause {
            for predicate in &where_clause.predicates {
                let syn::WherePredicate::Type(type_pred) = predicate else {
                    continue;
                };
                let syn::Type::Path(type_path) = &type_pred.bounded_ty else {
                    continue;
                };
                if type_path.qself.is_some() || type_path.path.segments.len() != 1 {
                    continue;
                }
                let type_param_name = type_path.path.segments[0].ident.to_string();
                for bound in &type_pred.bounds {
                    if let Some(trait_name) = Self::trait_name_from_bound(bound) {
                        Self::record_trait_bound_type_param_binding(
                            &mut raw,
                            trait_name,
                            &type_param_name,
                        );
                    }
                }
            }
        }

        raw.into_iter()
            .filter_map(|(trait_name, type_param)| type_param.map(|tp| (trait_name, tp)))
            .collect()
    }

    /// A Parenthesized Fn bound whose INPUTS mention a slice type. The
    /// Fn-bound threading (impl-Fn arg-expected substitution, callable-param
    /// invocation typing, closure-param typing) engages only for these —
    /// slice params are what the C++ side cannot recover (`entries.sort_by`
    /// on an unknown), while `&mut Self`-style bounds already emit correctly
    /// and their spellings must not churn.
    pub(super) fn fn_bound_inputs_mention_slice(bound: &syn::TypeParamBound) -> bool {
        fn ty_mentions_slice(ty: &syn::Type) -> bool {
            match ty {
                syn::Type::Slice(_) => true,
                syn::Type::Reference(r) => ty_mentions_slice(&r.elem),
                syn::Type::Paren(p) => ty_mentions_slice(&p.elem),
                syn::Type::Group(g) => ty_mentions_slice(&g.elem),
                _ => false,
            }
        }
        let syn::TypeParamBound::Trait(trait_bound) = bound else {
            return false;
        };
        let Some(seg) = trait_bound.path.segments.last() else {
            return false;
        };
        let syn::PathArguments::Parenthesized(args) = &seg.arguments else {
            return false;
        };
        args.inputs.iter().any(ty_mentions_slice)
    }

    /// Fn-bound ARG types per fn-generic (`F: FnOnce(&mut [Bucket<K, V>])`
    /// → F ↦ [&mut [Bucket<K, V>]]) — the invocation-side sibling of the
    /// return map below. Conflicting bounds drop the entry. SLICE-carrying
    /// signatures only (see fn_bound_inputs_mention_slice).
    pub(super) fn collect_callable_type_param_arg_map(
        generics: &syn::Generics,
    ) -> HashMap<String, Vec<syn::Type>> {
        let mut resolved: HashMap<String, Vec<syn::Type>> = HashMap::new();
        let mut conflicted: HashSet<String> = HashSet::new();
        let mut record = |type_param: String, arg_tys: Vec<syn::Type>| {
            if conflicted.contains(&type_param) {
                return;
            }
            if resolved.contains_key(&type_param) {
                resolved.remove(&type_param);
                conflicted.insert(type_param);
                return;
            }
            resolved.insert(type_param, arg_tys);
        };
        for param in &generics.params {
            let syn::GenericParam::Type(tp) = param else {
                continue;
            };
            for bound in &tp.bounds {
                if let Some((arg_tys, _)) =
                    Self::callable_bound_return_signature_from_type_param_bound(bound)
                    && Self::fn_bound_inputs_mention_slice(bound)
                {
                    record(tp.ident.to_string(), arg_tys);
                }
            }
        }
        if let Some(where_clause) = &generics.where_clause {
            for predicate in &where_clause.predicates {
                let syn::WherePredicate::Type(type_pred) = predicate else {
                    continue;
                };
                let syn::Type::Path(type_path) = &type_pred.bounded_ty else {
                    continue;
                };
                if type_path.qself.is_some() || type_path.path.segments.len() != 1 {
                    continue;
                }
                for bound in &type_pred.bounds {
                    if let Some((arg_tys, _)) =
                        Self::callable_bound_return_signature_from_type_param_bound(bound)
                        && Self::fn_bound_inputs_mention_slice(bound)
                    {
                        record(type_path.path.segments[0].ident.to_string(), arg_tys);
                    }
                }
            }
        }
        resolved
    }

    pub(super) fn collect_callable_type_param_return_map(
        generics: &syn::Generics,
    ) -> HashMap<String, syn::Type> {
        let mut resolved: HashMap<String, syn::Type> = HashMap::new();
        let mut conflicted: HashSet<String> = HashSet::new();
        let mut record = |type_param: String, ret_ty: syn::Type| {
            if conflicted.contains(&type_param) {
                return;
            }
            if let Some(existing) = resolved.get(&type_param) {
                if !Self::types_equivalent_by_tokens(existing, &ret_ty) {
                    resolved.remove(&type_param);
                    conflicted.insert(type_param);
                }
                return;
            }
            resolved.insert(type_param, ret_ty);
        };

        for param in &generics.params {
            let syn::GenericParam::Type(tp) = param else {
                continue;
            };
            let type_param = tp.ident.to_string();
            for bound in &tp.bounds {
                let Some((_, return_ty)) =
                    Self::callable_bound_return_signature_from_type_param_bound(bound)
                else {
                    continue;
                };
                record(type_param.clone(), return_ty);
            }
        }

        if let Some(where_clause) = &generics.where_clause {
            for predicate in &where_clause.predicates {
                let syn::WherePredicate::Type(type_pred) = predicate else {
                    continue;
                };
                let syn::Type::Path(type_path) = &type_pred.bounded_ty else {
                    continue;
                };
                if type_path.qself.is_some() || type_path.path.segments.len() != 1 {
                    continue;
                }
                let type_param = type_path.path.segments[0].ident.to_string();
                for bound in &type_pred.bounds {
                    let Some((_, return_ty)) =
                        Self::callable_bound_return_signature_from_type_param_bound(bound)
                    else {
                        continue;
                    };
                    record(type_param.clone(), return_ty);
                }
            }
        }

        resolved
    }

    pub(super) fn collect_callable_bound_return_signatures_for_function(
        &self,
        generics: &syn::Generics,
    ) -> Vec<(String, Vec<syn::Type>, syn::Type)> {
        let mut signatures: Vec<(String, Vec<syn::Type>, syn::Type)> = Vec::new();
        for param in &generics.params {
            let syn::GenericParam::Type(tp) = param else {
                continue;
            };
            let callable_name = tp.ident.to_string();
            self.collect_callable_bound_return_signatures_from_bounds(
                &callable_name,
                &tp.bounds,
                &mut signatures,
            );
        }
        if let Some(where_clause) = &generics.where_clause {
            for predicate in &where_clause.predicates {
                let syn::WherePredicate::Type(type_pred) = predicate else {
                    continue;
                };
                let syn::Type::Path(tp) = &type_pred.bounded_ty else {
                    continue;
                };
                if tp.qself.is_some() || tp.path.segments.len() != 1 {
                    continue;
                }
                let callable_name = tp.path.segments[0].ident.to_string();
                self.collect_callable_bound_return_signatures_from_bounds(
                    &callable_name,
                    &type_pred.bounds,
                    &mut signatures,
                );
            }
        }
        signatures
    }

    pub(super) fn collect_callable_bound_return_signatures_from_bounds(
        &self,
        callable_name: &str,
        bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::token::Plus>,
        out: &mut Vec<(String, Vec<syn::Type>, syn::Type)>,
    ) {
        for bound in bounds {
            let Some((arg_tys, return_ty)) =
                Self::callable_bound_return_signature_from_type_param_bound(bound)
            else {
                continue;
            };
            out.push((callable_name.to_string(), arg_tys, return_ty));
        }
    }
}

/// Known std trait method signatures whose argument types pin an
/// otherwise-uninferable local. These traits (io::Read) are declared by
/// no transpiled crate, so the declared-method tables never learn them.
fn builtin_std_method_arg_expected_type(method_name: &str, arg_idx: usize) -> Option<syn::Type> {
    match (method_name, arg_idx) {
        ("read_to_end", 0) => Some(syn::parse_quote!(&mut Vec<u8>)),
        ("read_to_string", 0) => Some(syn::parse_quote!(&mut String)),
        _ => None,
    }
}
