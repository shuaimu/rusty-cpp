use super::*;

impl CodeGen {
    pub(super) fn emit_function_forward_decl(&mut self, f: &syn::ItemFn, allow_non_unit: bool) -> bool {
        // Skip Rust libtest scaffolding and #[test] entrypoints in forward declarations.
        // Those are emitted via dedicated test-wrapper paths, not as top-level functions.
        if self.is_rust_libtest_main(f) || Self::has_test_attr(&f.attrs) {
            return false;
        }
        let can_forward_declare = match &f.sig.output {
            syn::ReturnType::Default => true,
            syn::ReturnType::Type(_, ty) => {
                matches!(ty.as_ref(), syn::Type::Tuple(tuple) if tuple.elems.is_empty())
                    || (allow_non_unit && {
                        self.push_type_param_scope(&f.sig.generics);
                        let mapped = self.map_type(ty);
                        self.pop_type_param_scope();
                        !type_string_has_auto_placeholder(&mapped)
                    })
            }
        };
        if !can_forward_declare {
            return false;
        }

        let rust_name = f.sig.ident.to_string();
        let name = self
            .module_qualified_functions
            .get(&rust_name)
            .filter(|mapped| !mapped.is_empty() && !mapped.contains("::"))
            .cloned()
            .unwrap_or_else(|| escape_cpp_keyword(&rust_name));
        let is_async = f.sig.asyncness.is_some();
        let undeduced_return_type_param = self.undeduced_return_type_param_for_function(
            &f.sig.generics,
            &f.sig.inputs,
            &f.sig.output,
        );
        let mut emitted_generics = f.sig.generics.clone();
        if let Some(param_name) = undeduced_return_type_param.as_deref() {
            emitted_generics.params = emitted_generics
                .params
                .into_iter()
                .filter(|param| match param {
                    syn::GenericParam::Type(tp) => tp.ident != param_name,
                    _ => true,
                })
                .collect();
        }

        self.push_type_param_scope(&f.sig.generics);
        let prev_forward_decl_signature = self.in_forward_decl_signature;
        self.in_forward_decl_signature = true;
        let mut return_type = if undeduced_return_type_param.is_some() {
            "auto".to_string()
        } else {
            self.map_return_type(&f.sig.output)
        };
        let mut params = self.map_fn_params(&f.sig.inputs);
        let mut param_types = self.map_fn_param_types(&f.sig.inputs);
        let mut signature_has_unresolved_scoped_paths = self
            .forward_decl_type_spelling_has_unresolved_scoped_path(&return_type)
            || self.forward_decl_type_spelling_has_unresolved_scoped_path(&param_types);
        if signature_has_unresolved_scoped_paths {
            // Forward-decl mapping can over-qualify colliding tails in some alias-heavy
            // scopes (for example serde's `Content` reexports). Retry once with normal
            // signature mapping and keep it if it resolves cleanly.
            self.in_forward_decl_signature = false;
            let fallback_return_type = if undeduced_return_type_param.is_some() {
                "auto".to_string()
            } else {
                self.map_return_type(&f.sig.output)
            };
            let fallback_params = self.map_fn_params(&f.sig.inputs);
            let fallback_param_types = self.map_fn_param_types(&f.sig.inputs);
            let fallback_has_unresolved = self
                .forward_decl_type_spelling_has_unresolved_scoped_path(&fallback_return_type)
                || self
                    .forward_decl_type_spelling_has_unresolved_scoped_path(&fallback_param_types);
            let mut used_unqualified_fallback = false;
            if !fallback_has_unresolved {
                return_type = fallback_return_type;
                params = fallback_params;
                param_types = fallback_param_types;
                signature_has_unresolved_scoped_paths = false;
                used_unqualified_fallback = true;
            }
            self.in_forward_decl_signature = true;
            if used_unqualified_fallback {
                // The fallback re-mapped with `in_forward_decl_signature = false`,
                // which drops cross-namespace qualification (e.g. a `yaml::`-module
                // struct param of a free function emitted under `namespace api`,
                // triggered by a function-pointer-typedef param). Re-qualify BARE
                // local types only — the absolutizing variant would wrongly force
                // serde-style nested private aliases to the global root.
                return_type = self.qualify_bare_local_types_in_type_string(&return_type);
                params = self.qualify_bare_local_types_in_type_string(&params);
                param_types = self.qualify_bare_local_types_in_type_string(&param_types);
            }
        }
        self.in_forward_decl_signature = prev_forward_decl_signature;
        let signature_has_unqualified_unknown_type = self
            .forward_decl_signature_has_unqualified_unknown_type_name(&return_type, &param_types);
        self.pop_type_param_scope();
        if (signature_has_unresolved_scoped_paths || signature_has_unqualified_unknown_type)
            && !self.module_body_forward_decl_pass
        {
            return false;
        }

        if is_async {
            return_type = format!("rusty::Task<{}>", return_type);
        }
        let abi_prefix = if let Some(abi) = &f.sig.abi {
            if let Some(name) = &abi.name {
                if name.value() == "C" {
                    "extern \"C\" "
                } else {
                    ""
                }
            } else {
                "extern \"C\" "
            }
        } else {
            ""
        };
        let export_prefix = if self.should_export_item(&f.vis, &f.sig.ident.to_string())
            || self.should_force_export_private_root_module_function(f)
        {
            "export "
        } else {
            ""
        };
        let constexpr_prefix = if f.sig.constness.is_some() {
            "constexpr "
        } else {
            ""
        };
        let static_prefix = format!(
            "{}{}",
            if self.should_emit_internal_linkage_function(f) {
                "static "
            } else {
                ""
            },
            constexpr_prefix
        );
        self.emit_template_declaration_with_type_defaults(
            &emitted_generics,
            export_prefix,
            &format!(
                "{}{}{} {}({});",
                static_prefix, abi_prefix, return_type, name, params
            ),
        );
        let rust_name = f.sig.ident.to_string();
        let rust_path = if self.module_stack.is_empty() {
            rust_name
        } else {
            format!("{}::{}", self.module_stack.join("::"), rust_name)
        };
        self.forward_declared_function_paths.insert(rust_path);
        true
    }

    pub(super) fn emit_function(&mut self, f: &syn::ItemFn) {
        // Skip #[cfg(test)] functions in non-test output
        // (they'll be emitted separately as test cases)

        if self.is_rust_libtest_main(f) {
            self.writeln("// Rust-only libtest main omitted");
            return;
        }

        // Phase 4a (inference engine plumbing — see
        // docs/rusty-cpp-transpiler.md §13). Build a per-function
        // inference context, populate it by walking the body, and
        // solve. The resolved substitution is held on `self` for
        // emit consumers (added in Phase 4b/4c). No emit site
        // queries it yet — this commit just proves the construction
        // is correct and side-effect-free on the existing emit path.
        //
        // We build the engine even for monomorphic functions to keep
        // the contract uniform: emit always finds either Some(ctx)
        // with a fixpoint-solved substitution, or None for callers
        // (forward-decls, libtest main) that skipped this path.
        {
            use crate::codegen::type_solver::{
                ConstraintCollector, InferenceContext,
            };
            use syn::visit::Visit;
            let mut ctx = InferenceContext::new();
            let mut collector = ConstraintCollector::new(&mut ctx);
            collector.visit_block(&f.block);
            let _errors = ctx.solve();
            // Errors are collected for future telemetry (§13.10's
            // "--print-inference" dump) but never abort emit.
            self.inference = Some(ctx);
        }

        let fn_name = f.sig.ident.to_string();
        let scoped_name = if self.module_stack.is_empty() {
            fn_name.clone()
        } else {
            format!("{}::{}", self.module_stack.join("::"), fn_name)
        };
        self.emitted_top_level_functions.insert(scoped_name.clone());

        let is_test = Self::has_test_attr(&f.attrs);

        // Emit doc comments
        self.emit_doc_comments(&f.attrs);

        // Emit @unsafe annotation for unsafe functions
        if f.sig.unsafety.is_some() {
            self.writeln("// @unsafe");
        }

        let name = self
            .module_qualified_functions
            .get(&fn_name)
            .filter(|mapped| !mapped.is_empty() && !mapped.contains("::"))
            .cloned()
            .unwrap_or_else(|| escape_cpp_keyword(&f.sig.ident.to_string()));
        let is_async = f.sig.asyncness.is_some();
        let undeduced_return_type_param = self.undeduced_return_type_param_for_function(
            &f.sig.generics,
            &f.sig.inputs,
            &f.sig.output,
        );
        let has_explicit_return_hint = undeduced_return_type_param.is_none();
        let mut emitted_generics = f.sig.generics.clone();
        if let Some(param_name) = undeduced_return_type_param.as_deref() {
            emitted_generics.params = emitted_generics
                .params
                .into_iter()
                .filter(|param| match param {
                    syn::GenericParam::Type(tp) => tp.ident != param_name,
                    _ => true,
                })
                .collect();
        }
        self.push_type_param_scope(&f.sig.generics);
        let return_type = if undeduced_return_type_param.is_some() {
            "auto".to_string()
        } else {
            self.map_return_type(&f.sig.output)
        };
        let params = self.map_fn_params(&f.sig.inputs);

        // Wrap return type in Task<> for async functions
        let return_type = if is_async {
            format!("rusty::Task<{}>", return_type)
        } else {
            return_type
        };
        let is_into_future_block_on_helper = !is_async
            && name == "block_on"
            && f.sig.inputs.len() == 1
            && return_type.contains("::Output")
            && f.sig.generics.params.iter().any(|param| match param {
                syn::GenericParam::Type(tp) => tp.bounds.iter().any(|bound| match bound {
                    syn::TypeParamBound::Trait(trait_bound) => trait_bound
                        .path
                        .segments
                        .last()
                        .is_some_and(|seg| seg.ident == "IntoFuture"),
                    _ => false,
                }),
                _ => false,
            });
        let stub_expanded_async_test_body = self
            .is_expanded_test_marker_function(&f.sig.ident.to_string())
            && self.block_contains_async_expr(&f.block);
        if stub_expanded_async_test_body {
            // Expanded marker wrappers resolve against emitted top-level function set.
            // Removing this entry causes wrapper generation to skip the unsupported
            // async test body while still preserving marker metadata for diagnostics.
            self.emitted_top_level_functions.remove(&scoped_name);
        }

        // Emit test case wrapper if #[test]
        if is_test {
            self.writeln(&format!("TEST_CASE(\"{}\") {{", name));
            self.indent += 1;
            self.push_param_bindings(&f.sig.inputs);
            self.emit_block(&f.block);
            self.pop_param_bindings();
            self.indent -= 1;
            self.writeln("}");
            self.pop_type_param_scope();
            return;
        }

        let hoisted_local_enums = self.collect_hoistable_local_enums_in_block(&f.block);
        let outer_function_params = Self::generic_type_or_const_param_names(&f.sig.generics);
        let mut hoisted_local_generic_structs =
            self.collect_hoistable_local_generic_structs_in_block(&f.block);
        hoisted_local_generic_structs.retain(|s| {
            !Self::local_type_items_reference_names(
                &f.block,
                &s.ident.to_string(),
                &outer_function_params,
            )
        });
        let mut hoisted_local_type_names: HashSet<String> = hoisted_local_generic_structs
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        hoisted_local_type_names.extend(hoisted_local_enums.iter().map(|e| e.ident.to_string()));
        if !hoisted_local_type_names.is_empty() {
            self.push_type_param_scope(&f.sig.generics);
            self.hoisted_local_type_name_scopes
                .push(hoisted_local_type_names.clone());
            self.emit_hoisted_local_enums_for_block(&f.block, &hoisted_local_enums);
            self.emit_hoisted_local_generic_structs_for_block(
                &f.block,
                &hoisted_local_generic_structs,
            );
            self.hoisted_local_type_name_scopes.pop();
            self.pop_type_param_scope();
        }
        let filtered_function_block = if hoisted_local_type_names.is_empty() {
            None
        } else {
            Some(self.strip_hoisted_local_generic_struct_items_from_block(
                &f.block,
                &hoisted_local_type_names,
            ))
        };
        let block_for_emission = filtered_function_block.as_ref().unwrap_or(&f.block);

        // Check for extern "C" ABI
        let abi_prefix = if let Some(abi) = &f.sig.abi {
            if let Some(name) = &abi.name {
                if name.value() == "C" {
                    "extern \"C\" "
                } else {
                    ""
                }
            } else {
                "extern \"C\" "
            }
        } else {
            ""
        };

        let export_prefix = if self.should_export_item(&f.vis, &f.sig.ident.to_string())
            || self.should_force_export_private_root_module_function(f)
        {
            "export "
        } else {
            ""
        };
        let constexpr_prefix = if f.sig.constness.is_some() {
            "constexpr "
        } else {
            ""
        };
        let static_prefix = format!(
            "{}{}",
            if self.should_emit_internal_linkage_function(f) {
                "static "
            } else {
                ""
            },
            constexpr_prefix
        );
        // Template declarations must be emitted outside the function's type-param scope.
        // Otherwise in-scope params are treated as already declared and the `template<...>`
        // prefix is dropped.
        self.pop_type_param_scope();
        self.emit_template_declaration_with_type_defaults(
            &emitted_generics,
            export_prefix,
            &format!(
                "{}{}{} {}({}) {{",
                static_prefix, abi_prefix, return_type, name, params
            ),
        );
        self.push_type_param_scope(&f.sig.generics);
        self.indent += 1;

        if stub_expanded_async_test_body {
            self.writeln(
                "throw std::runtime_error(\"unsupported async test body in expanded parity mode\");",
            );
            self.indent -= 1;
            self.writeln("}");
            self.pop_type_param_scope();
            return;
        }

        if is_into_future_block_on_helper {
            let fut_name = f
                .sig
                .inputs
                .first()
                .and_then(|arg| match arg {
                    syn::FnArg::Typed(pat_ty) => match pat_ty.pat.as_ref() {
                        syn::Pat::Ident(id) => Some(escape_cpp_keyword(&id.ident.to_string())),
                        _ => None,
                    },
                    _ => None,
                })
                .unwrap_or_else(|| "fut".to_string());
            self.writeln(&format!("return rusty::block_on(std::move({}));", fut_name));
            self.indent -= 1;
            self.writeln("}");
            self.pop_type_param_scope();
            return;
        }

        // Async functions use co_return instead of return
        let prev_async = self.in_async;
        self.in_async = is_async;
        self.push_return_value_scope(&return_type);
        if has_explicit_return_hint {
            self.push_return_type_hint(&f.sig.output);
        }
        self.push_param_bindings(&f.sig.inputs);
        let mut hoisted_impl_override_state = if hoisted_local_type_names.is_empty() {
            None
        } else {
            Some({
                let (
                    local_impl_overrides,
                    local_drop_overrides,
                    local_operator_overrides,
                    local_inherent_method_overrides,
                ) = self.collect_local_impl_overrides(&f.block.stmts, &hoisted_local_type_names);

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
                let mut prev_operator_overrides: Vec<((String, String), Option<String>)> =
                    Vec::new();
                for (op_key, op_value) in local_operator_overrides {
                    let prev = self.operator_renames.insert(op_key.clone(), op_value);
                    prev_operator_overrides.push((op_key, prev));
                }
                let mut prev_inherent_overrides: Vec<(String, Option<HashSet<String>>)> =
                    Vec::new();
                for (type_name, method_names) in local_inherent_method_overrides {
                    let prev = self
                        .inherent_impl_method_names
                        .insert(type_name.clone(), method_names);
                    prev_inherent_overrides.push((type_name, prev));
                }
                (
                    prev_impl_overrides,
                    inserted_drop_overrides,
                    prev_operator_overrides,
                    prev_inherent_overrides,
                )
            })
        };
        let mut hoisted_local_generic_param_metadata = Some(
            self.push_hoisted_local_generic_type_param_metadata(&hoisted_local_generic_structs),
        );
        let mut hoisted_local_type_scope_pushed = false;
        if !hoisted_local_type_names.is_empty() {
            self.hoisted_local_type_name_scopes
                .push(hoisted_local_type_names.clone());
            hoisted_local_type_scope_pushed = true;
        }
        self.emit_block(block_for_emission);
        if let Some((
            prev_impl_overrides,
            inserted_drop_overrides,
            prev_operator_overrides,
            prev_inherent_overrides,
        )) = hoisted_impl_override_state.take()
        {
            for (type_name, prev) in prev_inherent_overrides {
                if let Some(prev_names) = prev {
                    self.inherent_impl_method_names
                        .insert(type_name, prev_names);
                } else {
                    self.inherent_impl_method_names.remove(&type_name);
                }
            }
            for (op_key, prev) in prev_operator_overrides {
                if let Some(prev_value) = prev {
                    self.operator_renames.insert(op_key, prev_value);
                } else {
                    self.operator_renames.remove(&op_key);
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
        }
        if let Some(metadata) = hoisted_local_generic_param_metadata.take() {
            self.restore_hoisted_local_generic_type_param_metadata(metadata);
        }
        if hoisted_local_type_scope_pushed {
            self.hoisted_local_type_name_scopes.pop();
        }
        self.pop_param_bindings();
        if has_explicit_return_hint {
            self.pop_return_type_hint();
        }
        self.pop_return_value_scope();
        self.in_async = prev_async;

        self.indent -= 1;
        self.writeln("}");
        self.pop_type_param_scope();
    }

    pub(super) fn emit_foreign_mod(&mut self, fm: &syn::ItemForeignMod) {
        // extern "C" { fn foo(...); } → extern "C" { declarations }
        let abi = if let Some(abi_name) = &fm.abi.name {
            format!("\"{}\"", abi_name.value())
        } else {
            "\"C\"".to_string()
        };
        self.writeln(&format!("extern {} {{", abi));
        self.indent += 1;
        for item in &fm.items {
            if let syn::ForeignItem::Fn(f) = item {
                let name = &f.sig.ident;
                let return_type = self.map_return_type(&f.sig.output);
                let params = self.map_fn_params(&f.sig.inputs);
                self.writeln(&format!("{} {}({});", return_type, name, params));
            }
        }
        self.indent -= 1;
        self.writeln("}");
    }

    /// Emit a Rust `union` (pervasive in c2rust C ports, e.g.
    /// `unnamed_yaml_token_t_data`) as a C++ `union`. These are plain data
    /// unions with no impls/methods, so — unlike structs — we deliberately do
    /// NOT synthesize clone()/constructors (which would be ill-formed for a
    /// union with several members). Field types resolve via `map_type`; field
    /// metadata for inference is recorded separately in `collect_struct_metadata`.
    pub(super) fn emit_union(&mut self, u: &syn::ItemUnion) {
        let name_str = u.ident.to_string();
        let name = self.named_module_root_type_decl_cpp_name(&name_str);
        self.emit_doc_comments(&u.attrs);
        let export_prefix = if self.should_export_item(&u.vis, &name_str) {
            "export "
        } else {
            ""
        };
        self.emit_template_declaration_with_type_defaults(
            &u.generics,
            export_prefix,
            &format!("union {} {{", name),
        );
        self.push_type_param_scope(&u.generics);
        let prev_struct = self.current_struct.clone();
        self.current_struct = Some(name_str.clone());
        self.indent += 1;
        for field in &u.fields.named {
            if let Some(ident) = &field.ident {
                let cpp_ty = self.map_type(&field.ty);
                let cpp_name = escape_cpp_keyword(&ident.to_string());
                self.writeln(&format!("{} {};", cpp_ty, cpp_name));
            }
        }
        self.indent -= 1;
        self.current_struct = prev_struct;
        self.pop_type_param_scope();
        self.writeln("};");
    }

    pub(super) fn emit_struct(&mut self, s: &syn::ItemStruct) {
        let name_str = s.ident.to_string();
        let name = self.named_module_root_type_decl_cpp_name(&name_str);
        let has_drop_impl = self.type_has_drop_impl(&name_str);
        let merged_impl_items = self.take_impls_for_type(&name_str);

        // `#[cpp_inherit] impl Trait for ThisType` (recorded in
        // cpp_inherit_trait during collection) → emit `struct ThisType :
        // public Trait { ... }` with direct C++ inheritance instead of the
        // default TraitAdapter wrapper. The base spelling is reused for the
        // base clause and the synthesized/cpp_ctor base-init.
        let cpp_inherit_base: Option<String> = self.cpp_inherit_base_name(&name_str);
        // When a `#[cpp_inherit]` type ALSO has a `#[cpp_ctor]` factory, the
        // custom ctor takes over construction: suppress the synthesized
        // fieldwise + move ctor (a single fieldwise ctor can't supply the
        // default/parametrized ctors that `make_shared<X>(args)` call sites
        // need, and would collide with the custom one). The implicit move
        // ctor then handles moves — and correctly moves a stateful base,
        // unlike the synthesized reconstruct-the-base move ctor.
        let has_cpp_ctor_method = merged_impl_items.as_ref().is_some_and(|items| {
            items.iter().any(|it| {
                matches!(it, syn::ImplItem::Fn(m) if Self::has_cpp_ctor_attr(&m.attrs))
            })
        });
        let reserved_member_names: HashSet<String> = merged_impl_items
            .as_ref()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| self.merged_impl_member_name(item))
                    .collect()
            })
            .unwrap_or_default();
        let has_bitflags_flags_const = merged_impl_items.as_ref().is_some_and(|items| {
            items
                .iter()
                .any(|item| matches!(item, syn::ImplItem::Const(c) if c.ident == "FLAGS"))
        });
        let has_bitflags_api_signal = merged_impl_items.as_ref().is_some_and(|items| {
            items.iter().any(|item| match item {
                syn::ImplItem::Const(c) => c.ident == "FLAGS",
                syn::ImplItem::Fn(method) => {
                    let ident = method.sig.ident.to_string();
                    matches!(
                        ident.as_str(),
                        "bits"
                            | "from_bits_retain"
                            | "from_bits_truncate"
                            | "contains"
                            | "intersects"
                            | "iter"
                            | "iter_names"
                    )
                }
                _ => false,
            })
        });

        // Emit doc comments
        self.emit_doc_comments(&s.attrs);

        let export_prefix = if self.should_export_item(&s.vis, &name_str) {
            "export "
        } else {
            ""
        };
        let base_clause = match &cpp_inherit_base {
            Some(base) => format!(" : public {}", base),
            None => String::new(),
        };
        self.emit_template_declaration_with_type_defaults(
            &s.generics,
            export_prefix,
            &format!("struct {}{} {{", name, base_clause),
        );
        self.push_type_param_scope(&s.generics);
        self.indent += 1;

        // Emit associated type aliases BEFORE fields, so field types like
        // `<Self as Trait>::Assoc` can resolve through the alias.
        // Emit scalar associated constants before fields as well; Rust allows
        // `[T; Self::N]` before the impl block appears, while C++ needs the
        // static member declared before it can be used in a field type.
        // Track emitted names to prevent duplicate emission later.
        let mut early_emitted_type_aliases: HashSet<String> = HashSet::new();
        let mut early_emitted_const_names: HashSet<String> = HashSet::new();
        let mut early_assoc_type_cpp_types: HashMap<String, String> = HashMap::new();
        if let Some(ref methods) = merged_impl_items {
            let prev_struct = self.current_struct.clone();
            self.current_struct = Some(name_str.clone());
            self.emitted_non_method_member_names.push(HashSet::new());
            let reordered = Self::reorder_const_members_by_dependency(methods);
            for impl_item in reordered {
                if let syn::ImplItem::Type(t) = impl_item {
                    let alias_rust_name = t.ident.to_string();
                    let alias_name = escape_cpp_keyword(&t.ident.to_string());
                    let alias_cpp_type = Self::rewrite_private_keyword_namespace_in_type_path(
                        &self.normalize_assoc_alias_target_type(
                            &alias_rust_name,
                            self.map_type(&t.ty),
                        ),
                    );
                    self.emit_impl_item(impl_item);
                    // Even when constrained mode skips emitting the alias as a C++ member,
                    // keep its mapped type available for subsequent method return/projection
                    // resolution in this struct emission pass.
                    early_assoc_type_cpp_types
                        .entry(alias_rust_name.clone())
                        .or_insert(alias_cpp_type.clone());
                    early_assoc_type_cpp_types
                        .entry(alias_name.clone())
                        .or_insert(alias_cpp_type.clone());
                    let alias_was_emitted = self
                        .emitted_non_method_member_names
                        .last()
                        .is_some_and(|scope| scope.contains(&alias_name));
                    if alias_was_emitted {
                        early_emitted_type_aliases.insert(alias_name);
                    }
                } else if let syn::ImplItem::Const(c) = impl_item
                    && self.impl_const_can_emit_before_fields(c)
                {
                    let const_name = escape_cpp_keyword(&c.ident.to_string());
                    self.emit_impl_item(impl_item);
                    let const_was_emitted = self
                        .emitted_non_method_member_names
                        .last()
                        .is_some_and(|scope| scope.contains(&const_name));
                    if const_was_emitted {
                        early_emitted_const_names.insert(const_name);
                    }
                }
            }
            self.emitted_non_method_member_names.pop();
            self.current_struct = prev_struct;
        }

        // Emit fields
        let mut named_field_types: HashMap<String, syn::Type> = HashMap::new();
        let mut named_field_order: Vec<String> = Vec::new();
        let mut named_field_cpp_names: HashMap<String, String> = HashMap::new();
        let mut named_reference_fields: HashSet<String> = HashSet::new();
        let mut unnamed_field_types: HashMap<String, syn::Type> = HashMap::new();
        let mut unnamed_field_order: Vec<String> = Vec::new();
        let mut unnamed_field_cpp_names: HashMap<String, String> = HashMap::new();
        let mut unnamed_reference_fields: HashSet<String> = HashSet::new();
        let prev_struct_for_fields = self.current_struct.clone();
        self.current_struct = Some(name_str.clone());
        match &s.fields {
            syn::Fields::Named(fields) => {
                let mut used_member_names: HashSet<String> = HashSet::new();
                for field in &fields.named {
                    // Emit field doc comments
                    self.emit_doc_comments(&field.attrs);
                    let field_name = field.ident.as_ref().unwrap().to_string();
                    let field_key = Self::format_by_value_field_name(None, &field_name);
                    let field_type = self.map_field_type_with_by_value_cycle_breaking_rewrite(
                        &name_str, &field_key, &field.ty,
                    );
                    let mut emitted_field_name = escape_cpp_keyword(&field_name);
                    if reserved_member_names.contains(&emitted_field_name) {
                        emitted_field_name = format!("{}_field", emitted_field_name);
                    }
                    if used_member_names.contains(&emitted_field_name) {
                        let mut idx = 2usize;
                        loop {
                            let candidate = format!("{}_{}", emitted_field_name, idx);
                            if !used_member_names.contains(&candidate)
                                && !reserved_member_names.contains(&candidate)
                            {
                                emitted_field_name = candidate;
                                break;
                            }
                            idx += 1;
                        }
                    }
                    self.writeln(&format!("{} {};", field_type, emitted_field_name));
                    used_member_names.insert(emitted_field_name.clone());
                    named_field_types.insert(field_name.clone(), field.ty.clone());
                    if matches!(field.ty, syn::Type::Reference(_)) {
                        named_reference_fields.insert(field_name.clone());
                    }
                    named_field_order.push(field_name.clone());
                    named_field_cpp_names.insert(field_name, emitted_field_name);
                }
            }
            syn::Fields::Unnamed(fields) => {
                for (i, field) in fields.unnamed.iter().enumerate() {
                    let field_name = format!("_{}", i);
                    let field_key = Self::format_by_value_field_name(None, &format!("#{}", i));
                    let mut field_type = self.map_field_type_with_by_value_cycle_breaking_rewrite(
                        &name_str, &field_key, &field.ty,
                    );
                    if self.block_depth > 0
                        && name_str == "AsBytes"
                        && fields.unnamed.len() == 1
                        && !field_type.contains('&')
                    {
                        field_type = format!("const {}&", field_type);
                    }
                    let emitted_field_name = escape_cpp_keyword(&field_name);
                    self.writeln(&format!("{} {};", field_type, emitted_field_name));
                    unnamed_field_types.insert(field_name.clone(), field.ty.clone());
                    unnamed_field_order.push(field_name.clone());
                    unnamed_field_cpp_names.insert(field_name.clone(), emitted_field_name);
                    if matches!(field.ty, syn::Type::Reference(_)) {
                        unnamed_reference_fields.insert(field_name);
                    }
                }
            }
            syn::Fields::Unit => {}
        }
        self.current_struct = prev_struct_for_fields;
        if matches!(&s.fields, syn::Fields::Unit) {
            self.unit_struct_types.insert(name_str.clone());
            let scoped_name = self.scoped_type_key(&name_str);
            self.unit_struct_types.insert(scoped_name);
        }
        if let syn::Fields::Unnamed(fields) = &s.fields {
            let arity = fields.unnamed.len();
            self.tuple_struct_arities.insert(name_str.clone(), arity);
            let scoped_name = self.scoped_type_key(&name_str);
            self.tuple_struct_arities.insert(scoped_name, arity);
        }
        if !named_field_types.is_empty() {
            std::rc::Rc::make_mut(&mut self.struct_field_types)
                .insert(name_str.clone(), named_field_types.clone());
            let scoped_name = self.scoped_type_key(&name_str);
            std::rc::Rc::make_mut(&mut self.struct_field_types)
                .insert(scoped_name, named_field_types);
            if !named_reference_fields.is_empty() {
                self.struct_reference_fields
                    .insert(name_str.clone(), named_reference_fields.clone());
                let scoped_name = self.scoped_type_key(&name_str);
                self.struct_reference_fields
                    .insert(scoped_name, named_reference_fields.clone());
            }
            std::rc::Rc::make_mut(&mut self.struct_field_order)
                .insert(name_str.clone(), named_field_order.clone());
            let scoped_name = self.scoped_type_key(&name_str);
            std::rc::Rc::make_mut(&mut self.struct_field_order)
                .insert(scoped_name, named_field_order);
            std::rc::Rc::make_mut(&mut self.struct_field_cpp_names)
                .insert(name_str.clone(), named_field_cpp_names.clone());
            let scoped_name = self.scoped_type_key(&name_str);
            std::rc::Rc::make_mut(&mut self.struct_field_cpp_names)
                .insert(scoped_name, named_field_cpp_names.clone());
        }
        if !unnamed_field_types.is_empty() {
            std::rc::Rc::make_mut(&mut self.struct_field_types)
                .insert(name_str.clone(), unnamed_field_types.clone());
            let scoped_name = self.scoped_type_key(&name_str);
            std::rc::Rc::make_mut(&mut self.struct_field_types)
                .insert(scoped_name, unnamed_field_types);
            if !unnamed_reference_fields.is_empty() {
                self.struct_reference_fields
                    .insert(name_str.clone(), unnamed_reference_fields.clone());
                let scoped_name = self.scoped_type_key(&name_str);
                self.struct_reference_fields
                    .insert(scoped_name, unnamed_reference_fields.clone());
            }
            std::rc::Rc::make_mut(&mut self.struct_field_order)
                .insert(name_str.clone(), unnamed_field_order.clone());
            let scoped_name = self.scoped_type_key(&name_str);
            std::rc::Rc::make_mut(&mut self.struct_field_order)
                .insert(scoped_name, unnamed_field_order);
            std::rc::Rc::make_mut(&mut self.struct_field_cpp_names)
                .insert(name_str.clone(), unnamed_field_cpp_names.clone());
            let scoped_name = self.scoped_type_key(&name_str);
            std::rc::Rc::make_mut(&mut self.struct_field_cpp_names)
                .insert(scoped_name, unnamed_field_cpp_names.clone());
        }

        if has_drop_impl {
            // Strict null-state convention (replaces the global
            // forgotten_addresses table from rusty/mem.hpp): every struct
            // with a Drop impl carries a local `_rusty_forgotten` flag
            // that move ctors / `mem::forget` set, and the destructor
            // short-circuits on. `mutable` so `mem::forget` can set it
            // on const-T locals (the previous code path used
            // mark_forgotten_address(&value) for this).
            self.writeln("mutable bool _rusty_forgotten = false;");
            // If any field is a known move-only wrapper (e.g. `SpinMutex`,
            // `Mutex`, `Box`), the implicit copy ctor / copy assign would be
            // deleted by C++ anyway. Emit them as `= delete;` explicitly so
            // diagnostics point at the right declaration instead of a
            // distant aggregate ctor.
            let copy_disposition = if self.struct_fields_have_known_non_copyable(&s.fields) {
                "delete"
            } else {
                "default"
            };
            match &s.fields {
                syn::Fields::Named(fields) => {
                    let first_field_has_drop_impl = fields
                        .named
                        .iter()
                        .next()
                        .map(|field| self.field_type_has_drop_impl(&field.ty))
                        .unwrap_or(false);
                    let ctor_params: Vec<String> = fields
                        .named
                        .iter()
                        .filter_map(|field| {
                            let field_name = field.ident.as_ref()?.to_string();
                            let param_name = format!("{}_init", field_name);
                            Some(format!("{} {}", self.map_type(&field.ty), param_name))
                        })
                        .collect();
                    let ctor_inits: Vec<String> = fields
                        .named
                        .iter()
                        .filter_map(|field| {
                            let rust_field_name = field.ident.as_ref()?.to_string();
                            let member_name = named_field_cpp_names
                                .get(&rust_field_name)
                                .cloned()
                                .unwrap_or_else(|| escape_cpp_keyword(&rust_field_name));
                            let param_name = format!("{}_init", rust_field_name);
                            let rewrite_field_key =
                                Self::format_by_value_field_name(None, &rust_field_name);
                            let init = if matches!(&field.ty, syn::Type::Reference(_)) {
                                format!("{}({})", member_name, param_name)
                            } else {
                                let moved = format!("std::move({})", param_name);
                                let wrapped = self.wrap_by_value_cycle_rewrite_field_initializer(
                                    &name_str,
                                    &rewrite_field_key,
                                    moved,
                                );
                                format!("{}({})", member_name, wrapped)
                            };
                            Some(init)
                        })
                        .collect();
                    self.writeln(&format!(
                        "{}({}) : {} {{}}",
                        name,
                        ctor_params.join(", "),
                        ctor_inits.join(", ")
                    ));
                    self.writeln(&format!(
                        "{}(const {}&) = {};",
                        name, name, copy_disposition
                    ));

                    let move_inits: Vec<String> = fields
                        .named
                        .iter()
                        .filter_map(|field| {
                            let rust_field_name = field.ident.as_ref()?.to_string();
                            let member_name = named_field_cpp_names
                                .get(&rust_field_name)
                                .cloned()
                                .unwrap_or_else(|| escape_cpp_keyword(&rust_field_name));
                            let init = if matches!(&field.ty, syn::Type::Reference(_)) {
                                format!("{}(other.{})", member_name, member_name)
                            } else {
                                format!("{}(std::move(other.{}))", member_name, member_name)
                            };
                            Some(init)
                        })
                        .collect();
                    if move_inits.is_empty() {
                        self.writeln(&format!("{}({}&& other) noexcept {{", name, name));
                    } else {
                        self.writeln(&format!(
                            "{}({}&& other) noexcept : {} {{",
                            name,
                            name,
                            move_inits.join(", ")
                        ));
                    }
                    self.indent += 1;
                    // Strict null-state: propagate other's forgotten flag
                    // (so we inherit "moved-from" status if other was) and
                    // unconditionally mark other as forgotten. Equivalent
                    // to the old consume/mark dance against the global
                    // forgotten_addresses table, but purely local.
                    self.writeln("this->_rusty_forgotten = other._rusty_forgotten;");
                    self.writeln("other._rusty_forgotten = true;");
                    let _ = first_field_has_drop_impl;
                    self.indent -= 1;
                    self.writeln("}");
                    // Rule of Five: when copy ctor and move ctor are declared,
                    // also declare assignment operators to prevent them from
                    // being implicitly deleted.
                    self.writeln(&format!(
                        "{}& operator=(const {}&) = {};",
                        name, name, copy_disposition
                    ));
                    self.writeln(&format!(
                        "{}& operator=({}&& other) noexcept {{",
                        name, name
                    ));
                    self.indent += 1;
                    self.writeln("if (this == &other) {");
                    self.indent += 1;
                    self.writeln("return *this;");
                    self.indent -= 1;
                    self.writeln("}");
                    self.writeln(&format!("this->~{}();", name));
                    self.writeln(&format!("new (this) {}(std::move(other));", name));
                    self.writeln("return *this;");
                    self.indent -= 1;
                    self.writeln("}");
                }
                syn::Fields::Unnamed(fields) => {
                    let first_field_has_drop_impl = fields
                        .unnamed
                        .iter()
                        .next()
                        .map(|field| self.field_type_has_drop_impl(&field.ty))
                        .unwrap_or(false);
                    let ctor_params: Vec<String> = fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(i, field)| format!("{} _{}_init", self.map_type(&field.ty), i))
                        .collect();
                    let ctor_inits: Vec<String> = fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(i, field)| {
                            let param_name = format!("_{}_init", i);
                            let rewrite_field_key =
                                Self::format_by_value_field_name(None, &format!("#{}", i));
                            if matches!(&field.ty, syn::Type::Reference(_)) {
                                format!("_{}({})", i, param_name)
                            } else {
                                let moved = format!("std::move({})", param_name);
                                let wrapped = self.wrap_by_value_cycle_rewrite_field_initializer(
                                    &name_str,
                                    &rewrite_field_key,
                                    moved,
                                );
                                format!("_{}({})", i, wrapped)
                            }
                        })
                        .collect();
                    self.writeln(&format!(
                        "{}({}) : {} {{}}",
                        name,
                        ctor_params.join(", "),
                        ctor_inits.join(", ")
                    ));
                    self.writeln(&format!(
                        "{}(const {}&) = {};",
                        name, name, copy_disposition
                    ));

                    let move_inits: Vec<String> = fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(i, field)| {
                            if matches!(&field.ty, syn::Type::Reference(_)) {
                                format!("_{}(other._{})", i, i)
                            } else {
                                format!("_{}(std::move(other._{}))", i, i)
                            }
                        })
                        .collect();
                    self.writeln(&format!(
                        "{}({}&& other) noexcept : {} {{",
                        name,
                        name,
                        move_inits.join(", ")
                    ));
                    self.indent += 1;
                    // Strict null-state: propagate other's forgotten flag
                    // (so we inherit "moved-from" status if other was) and
                    // unconditionally mark other as forgotten. Equivalent
                    // to the old consume/mark dance against the global
                    // forgotten_addresses table, but purely local.
                    self.writeln("this->_rusty_forgotten = other._rusty_forgotten;");
                    self.writeln("other._rusty_forgotten = true;");
                    let _ = first_field_has_drop_impl;
                    self.indent -= 1;
                    self.writeln("}");
                    self.writeln(&format!(
                        "{}& operator=(const {}&) = {};",
                        name, name, copy_disposition
                    ));
                    self.writeln(&format!(
                        "{}& operator=({}&& other) noexcept {{",
                        name, name
                    ));
                    self.indent += 1;
                    self.writeln("if (this == &other) {");
                    self.indent += 1;
                    self.writeln("return *this;");
                    self.indent -= 1;
                    self.writeln("}");
                    self.writeln(&format!("this->~{}();", name));
                    self.writeln(&format!("new (this) {}(std::move(other));", name));
                    self.writeln("return *this;");
                    self.indent -= 1;
                    self.writeln("}");
                }
                syn::Fields::Unit => {
                    self.writeln(&format!("{}() = default;", name));
                    self.writeln(&format!(
                        "{}(const {}&) = {};",
                        name, name, copy_disposition
                    ));
                    self.writeln(&format!("{}({}&& other) noexcept {{", name, name));
                    self.indent += 1;
                    // Strict null-state: propagate other's forgotten flag.
                    self.writeln("this->_rusty_forgotten = other._rusty_forgotten;");
                    self.writeln("other._rusty_forgotten = true;");
                    self.indent -= 1;
                    self.writeln("}");
                    self.writeln(&format!(
                        "{}& operator=(const {}&) = {};",
                        name, name, copy_disposition
                    ));
                    self.writeln(&format!(
                        "{}& operator=({}&& other) noexcept {{",
                        name, name
                    ));
                    self.indent += 1;
                    self.writeln("if (this == &other) {");
                    self.indent += 1;
                    self.writeln("return *this;");
                    self.indent -= 1;
                    self.writeln("}");
                    self.writeln(&format!("this->~{}();", name));
                    self.writeln(&format!("new (this) {}(std::move(other));", name));
                    self.writeln("return *this;");
                    self.indent -= 1;
                    self.writeln("}");
                }
            }
            self.writeln(
                "void rusty_mark_forgotten() const noexcept { _rusty_forgotten = true; }",
            );
            self.newline();
        }

        // `#[cpp_inherit]` structs are polymorphic (virtual base + overrides),
        // so they are NOT C++ aggregates — aggregate/designated init is
        // illegal. Synthesize an explicit fieldwise ctor (the positional
        // struct-literal lowering targets it) plus a move ctor that
        // reconstructs a fresh base subobject. The interface base deletes its
        // move ctor, which would implicitly delete the subclass move and break
        // `Arc<Self>::new_(Self::new_(...))`; reconstructing the (stateless)
        // base sidesteps that without touching the shared base class. Only the
        // named-fields case is handled (the inheritance migrations are all
        // record-shaped); unit/tuple cpp_inherit types fall through unchanged.
        if let Some(base) = &cpp_inherit_base {
            if !has_drop_impl && !has_cpp_ctor_method {
                if let syn::Fields::Named(fields) = &s.fields {
                    let member_of = |rust_name: &str| -> String {
                        named_field_cpp_names
                            .get(rust_name)
                            .cloned()
                            .unwrap_or_else(|| escape_cpp_keyword(rust_name))
                    };
                    // Fieldwise ctor: `Self(F0 f0_init, ...) : Base(), f0(...) {}`
                    let ctor_params: Vec<String> = fields
                        .named
                        .iter()
                        .filter_map(|field| {
                            let fname = field.ident.as_ref()?.to_string();
                            Some(format!("{} {}_init", self.map_type(&field.ty), fname))
                        })
                        .collect();
                    let mut ctor_inits: Vec<String> = vec![format!("{}()", base)];
                    for field in &fields.named {
                        let Some(fname) = field.ident.as_ref().map(|i| i.to_string()) else {
                            continue;
                        };
                        let member = member_of(&fname);
                        let param = format!("{}_init", fname);
                        if matches!(&field.ty, syn::Type::Reference(_)) {
                            ctor_inits.push(format!("{}({})", member, param));
                        } else {
                            ctor_inits.push(format!("{}(std::move({}))", member, param));
                        }
                    }
                    self.writeln(&format!(
                        "{}({}) : {} {{}}",
                        name,
                        ctor_params.join(", "),
                        ctor_inits.join(", ")
                    ));
                    // Move ctor: `Self(Self&& other) noexcept : Base(), f0(...) {}`
                    let mut move_inits: Vec<String> = vec![format!("{}()", base)];
                    for field in &fields.named {
                        let Some(fname) = field.ident.as_ref().map(|i| i.to_string()) else {
                            continue;
                        };
                        let member = member_of(&fname);
                        if matches!(&field.ty, syn::Type::Reference(_)) {
                            move_inits.push(format!("{}(other.{})", member, member));
                        } else {
                            move_inits.push(format!("{}(std::move(other.{}))", member, member));
                        }
                    }
                    self.writeln(&format!(
                        "{}({}&& other) noexcept : {} {{}}",
                        name,
                        name,
                        move_inits.join(", ")
                    ));
                    self.newline();
                }
            }
        }

        // Emit methods from impl blocks (merged)
        let mut emitted_methods_in_struct = std::collections::HashSet::<String>::new();
        if let Some(methods) = merged_impl_items {
            if !matches!(&s.fields, syn::Fields::Unit if methods.is_empty()) {
                self.newline();
            }
            let mut method_output_types = HashMap::new();
            for impl_item in &methods {
                let syn::ImplItem::Fn(method) = impl_item else {
                    continue;
                };
                let syn::ReturnType::Type(_, ret_ty) = &method.sig.output else {
                    continue;
                };
                method_output_types.insert(method.sig.ident.to_string(), (**ret_ty).clone());
            }
            let prev_struct = self.current_struct.clone();
            self.current_struct = Some(name_str.clone());
            self.emitted_method_conflict_keys.push(HashSet::new());
            let mut non_method_member_names: HashSet<String> =
                named_field_cpp_names.values().cloned().collect();
            // Include type aliases already emitted before fields
            non_method_member_names.extend(early_emitted_type_aliases.iter().cloned());
            non_method_member_names.extend(early_emitted_const_names.iter().cloned());
            self.emitted_non_method_member_names
                .push(non_method_member_names);
            self.current_struct_assoc_cpp_types
                .push(early_assoc_type_cpp_types.clone());
            self.current_struct_method_output_types
                .push(method_output_types);

            // Collect source modules for using-namespace inside method bodies
            let current_module = self.module_stack.join("::");
            let scoped = self.scoped_type_key(&name_str);
            let mut source_modules_for_methods: Vec<String> = self
                .impl_source_modules
                .get(&name_str)
                .or_else(|| self.impl_source_modules.get(&scoped))
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|m| m != &current_module && !m.is_empty())
                .map(|m| self.escape_and_rename_qualified_name(&m))
                .collect();
            source_modules_for_methods.sort();
            source_modules_for_methods.dedup();
            self.merged_method_using_namespaces = source_modules_for_methods;

            // Reorder static const members so dependencies come before dependents.
            // E.g., `const ABC = A.bits() | B.bits()` must come after `const A`, `const B`.
            let reordered = Self::reorder_const_members_by_dependency(&methods);
            let mut instance_method_collision_keys: HashSet<String> = HashSet::new();
            for impl_item in &reordered {
                let syn::ImplItem::Fn(method) = impl_item else {
                    continue;
                };
                if method_has_receiver(method) {
                    instance_method_collision_keys
                        .insert(impl_method_static_instance_collision_key(method));
                }
            }
            let struct_has_emitted_template_params = s.generics.params.iter().any(|param| {
                matches!(
                    param,
                    syn::GenericParam::Type(_) | syn::GenericParam::Const(_)
                )
            });
            let is_hoisted_local_type = self
                .hoisted_local_type_name_scopes
                .iter()
                .rev()
                .any(|scope| scope.contains(&name_str));
            let can_defer_struct_method_definitions = self.block_depth == 0
                && !is_hoisted_local_type
                && !struct_has_emitted_template_params
                && !self.deferred_method_definitions_stack.is_empty();
            let prev_local_out_of_line_owner = self.method_emission_out_of_line_owner.clone();
            let prev_local_skip_conflict = self.method_emission_skip_conflict_registration;
            if is_hoisted_local_type {
                self.method_emission_out_of_line_owner = None;
                self.method_emission_skip_conflict_registration = false;
            }
            for impl_item in &reordered {
                if let syn::ImplItem::Type(t) = impl_item {
                    let alias_rust_name = t.ident.to_string();
                    let alias_name = escape_cpp_keyword(&alias_rust_name);
                    let alias_cpp_type = Self::rewrite_private_keyword_namespace_in_type_path(
                        &self.normalize_assoc_alias_target_type(
                            &alias_rust_name,
                            self.map_type(&t.ty),
                        ),
                    );
                    if let Some(scope) = self.current_struct_assoc_cpp_types.last_mut() {
                        scope.insert(alias_rust_name, alias_cpp_type.clone());
                        scope.insert(alias_name, alias_cpp_type);
                    }
                }
                if let syn::ImplItem::Fn(method) = impl_item
                    && !method_has_receiver(method)
                {
                    let collision_key = impl_method_static_instance_collision_key(method);
                    if instance_method_collision_keys.contains(&collision_key) {
                        // C++ cannot overload static and non-static methods with
                        // the same signature shape; prefer the receiver-bearing
                        // method for trait-driven call sites.
                        continue;
                    }
                }
                if can_defer_struct_method_definitions && let syn::ImplItem::Fn(method) = impl_item
                {
                    let prev_decl_only = self.method_emission_declaration_only;
                    let prev_out_of_line_owner = self.method_emission_out_of_line_owner.clone();
                    let prev_skip_conflict = self.method_emission_skip_conflict_registration;

                    self.method_emission_declaration_only = true;
                    self.method_emission_out_of_line_owner = None;
                    self.method_emission_skip_conflict_registration = false;
                    let declaration_output_len = self.output.len();
                    self.emit_method(method);
                    let declaration_emitted = self.output.len() != declaration_output_len;
                    if !declaration_emitted {
                        self.method_emission_declaration_only = prev_decl_only;
                        self.method_emission_out_of_line_owner = prev_out_of_line_owner;
                        self.method_emission_skip_conflict_registration = prev_skip_conflict;
                        continue;
                    }

                    let owner_cpp_name = self.named_module_root_type_decl_cpp_name(&name_str);
                    self.method_emission_declaration_only = false;
                    self.method_emission_out_of_line_owner = Some(owner_cpp_name);
                    self.method_emission_skip_conflict_registration = true;
                    let saved_output = std::mem::take(&mut self.output);
                    let saved_indent = self.indent;
                    self.output = String::new();
                    self.indent = saved_indent.saturating_sub(1);
                    self.push_deferred_method_definition_scope();
                    self.emit_method(method);
                    let mut deferred_definition = std::mem::take(&mut self.output);
                    let nested_deferred_defs = self
                        .deferred_method_definitions_stack
                        .pop()
                        .unwrap_or_default();
                    for nested in nested_deferred_defs {
                        if !deferred_definition.is_empty() && !deferred_definition.ends_with('\n') {
                            deferred_definition.push('\n');
                        }
                        deferred_definition.push_str(&nested);
                    }
                    self.output = saved_output;
                    self.indent = saved_indent;

                    self.method_emission_declaration_only = prev_decl_only;
                    self.method_emission_out_of_line_owner = prev_out_of_line_owner;
                    self.method_emission_skip_conflict_registration = prev_skip_conflict;

                    self.queue_deferred_method_definition(deferred_definition);
                    continue;
                }
                // Cluster A completion: when emitting an absorbed method
                // whose impl-block generics structurally decomposed into
                // a host class param (e.g. `B/K/V/T` from `impl<B,K,V,T>
                // Handle<NodeRef<B,K,V,T>, Type>`), bracket the emit and
                // textually substitute the dropped-generic refs to
                // `typename __TemplateArgs<HostParam>::arg_<N>`. This
                // makes those refs dependent on the host class param so
                // they're checked at instantiation rather than at class-
                // template parse time (when the dropped names aren't in
                // scope).
                let cluster_a_decomp = match impl_item {
                    syn::ImplItem::Fn(method) => {
                        let key = (name_str.clone(), method.sig.ident.to_string());
                        self.method_structural_decompositions.get(&key).cloned()
                    }
                    _ => None,
                };
                if let Some(decomp) = cluster_a_decomp {
                    let start = self.output.len();
                    let prev_emit_decomp = self.current_emit_structural_decomp.take();
                    self.current_emit_structural_decomp = Some(decomp.clone());
                    self.emit_impl_item(impl_item);
                    self.current_emit_structural_decomp = prev_emit_decomp;
                    let method_text = self.output[start..].to_string();
                    let mut substituted =
                        apply_structural_decomp_text_substitution(&method_text, &decomp);
                    // Cluster C nested-marker subs: when parallel impls
                    // hardcode different markers at a nested arg
                    // position (e.g. NodeRef's 4th arg = Leaf vs
                    // Internal), substitute the concrete marker name
                    // with the host's dependent-path expression.
                    if let syn::ImplItem::Fn(method) = impl_item {
                        let method_key =
                            (name_str.clone(), method.sig.ident.to_string());
                        if let Some(nested_subs) = self
                            .parallel_impl_nested_marker_text_subs
                            .get(&method_key)
                        {
                            for (concrete, dep_path) in nested_subs {
                                substituted = replace_whole_word(
                                    &substituted,
                                    concrete,
                                    dep_path,
                                );
                            }
                        }
                    }
                    self.output.replace_range(start.., &substituted);
                } else {
                    self.emit_impl_item(impl_item);
                }
            }
            if is_hoisted_local_type {
                self.method_emission_out_of_line_owner = prev_local_out_of_line_owner;
                self.method_emission_skip_conflict_registration = prev_local_skip_conflict;
            }
            self.merged_method_using_namespaces.clear();
            self.current_struct_method_output_types.pop();
            self.current_struct_assoc_cpp_types.pop();
            self.emitted_non_method_member_names.pop();
            // Save emitted method names before popping for synthetic check
            emitted_methods_in_struct = self
                .emitted_method_conflict_keys
                .last()
                .map(|keys| {
                    keys.iter()
                        .filter_map(|k| k.split('|').next().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            self.emitted_method_conflict_keys.pop();
            self.current_struct = prev_struct;
        }

        // Emit derive-generated code
        let derives = self.extract_derives(&s.attrs);
        if !derives.is_empty() {
            self.newline();
        }
        // PartialEq/Eq both lower to the same `operator==`, and
        // PartialOrd/Ord both lower to the same `operator<=>`. If a
        // struct derives more than one of each family, we'd emit the
        // operator twice — C++ then sees redundant overloads ("cannot
        // be overloaded with itself"). Track which operator families
        // we've already emitted so the second derive in the family is
        // a no-op.
        let mut emitted_eq_operator = false;
        let mut emitted_ord_operator = false;
        for derive in &derives {
            match derive.as_str() {
                "Clone" => {
                    let clone_body = match &s.fields {
                        syn::Fields::Named(fields) => {
                            let field_inits: Vec<String> = fields
                                .named
                                .iter()
                                .filter_map(|field| field.ident.as_ref())
                                .map(|ident| {
                                    let rust_name = ident.to_string();
                                    let cpp_name = named_field_cpp_names
                                        .get(&rust_name)
                                        .cloned()
                                        .unwrap_or_else(|| escape_cpp_keyword(&rust_name));
                                    if self.struct_field_is_reference(&name_str, &rust_name) {
                                        format!(".{} = this->{}", cpp_name, cpp_name)
                                    } else {
                                        format!(".{} = rusty::clone(this->{})", cpp_name, cpp_name)
                                    }
                                })
                                .collect();
                            format!("return {}{{{}}};", name, field_inits.join(", "))
                        }
                        syn::Fields::Unnamed(fields) => {
                            let elems: Vec<String> = (0..fields.unnamed.len())
                                .map(|idx| {
                                    let rust_field = format!("_{}", idx);
                                    if self.struct_field_is_reference(&name_str, &rust_field) {
                                        format!("this->_{}", idx)
                                    } else {
                                        format!("rusty::clone(this->_{})", idx)
                                    }
                                })
                                .collect();
                            format!("return {}{{{}}};", name, elems.join(", "))
                        }
                        syn::Fields::Unit => format!("return {}{{}};", name),
                    };
                    self.writeln(&format!("{} clone() const {{ {} }}", name, clone_body));
                }
                "PartialEq" | "Eq" => {
                    // C++20 doesn't allow defaulting a *templated*
                    // operator (`auto operator==(const auto&) const = default;`
                    // — the `auto` parameter makes it a template).
                    // Use the concrete struct type instead so the
                    // emission matches the canonical `operator==`
                    // shape that the compiler will happily default.
                    // Dedup against PartialEq + Eq both deriving the
                    // same operator.
                    if !emitted_eq_operator {
                        self.writeln(&format!(
                            "bool operator==(const {}&) const = default;",
                            name
                        ));
                        emitted_eq_operator = true;
                    }
                }
                "PartialOrd" | "Ord" => {
                    // Same templated-default issue as above; same
                    // dedup against PartialOrd + Ord both deriving
                    // the same operator.
                    if !emitted_ord_operator {
                        self.writeln(&format!(
                            "auto operator<=>(const {}&) const = default;",
                            name
                        ));
                        emitted_ord_operator = true;
                    }
                }
                "Default" => {
                    // Default constructor
                    self.writeln(&format!("static {} default_() {{ return {{}}; }}", name));
                }
                "Debug" => {
                    // Simple stream operator stub
                    self.writeln(&format!(
                        "friend std::ostream& operator<<(std::ostream& os, const {}& v) {{",
                        name
                    ));
                    self.indent += 1;
                    self.writeln(&format!("return os << \"{} {{ ... }}\";", name));
                    self.indent -= 1;
                    self.writeln("}");
                }
                "Hash" => {
                    // Hash is emitted after the struct as a specialization
                    // (handled below)
                }
                _ => {
                    self.writeln(&format!("// TODO: derive({})", derive));
                }
            }
        }

        // For single-field newtype wrappers with operator traits (bitflags pattern),
        // emit synthetic inline methods for common Flags trait methods that
        // are defined inside const _ blocks and can't be directly extracted.
        if let syn::Fields::Unnamed(fields) = &s.fields {
            if fields.unnamed.len() == 1 {
                let name_str = name.to_string();
                let scoped_key = self.scoped_type_key(&name_str);
                let bits_cpp_type = self.map_type(&fields.unnamed[0].ty);
                let has_operators = self
                    .operator_renames
                    .keys()
                    .any(|(type_key, _)| *type_key == name_str || *type_key == scoped_key);
                let has_bitand_operator =
                    self.operator_renames.iter().any(|((type_key, _), op)| {
                        (*type_key == name_str || *type_key == scoped_key) && op == "operator&"
                    });
                let should_emit_bitflags_synthetic = has_operators
                    && (has_bitflags_flags_const || has_bitflags_api_signal || has_bitand_operator);
                if should_emit_bitflags_synthetic {
                    let n = name.to_string();
                    // Use the saved emitted method names from before the pop.
                    let merged_methods = &emitted_methods_in_struct;
                    self.newline();
                    self.writeln("// Synthetic bitwise trait methods (from const _ block impls)");
                    if !merged_methods.contains("bits") {
                        self.writeln(&format!(
                            "{} bits() const {{ return this->_0; }}",
                            bits_cpp_type
                        ));
                    }
                    // from_bits_retain is typically already provided by
                    // trait static default method injection (Leaf 27).
                    // Skip synthetic emission to avoid overload conflicts.
                    if !merged_methods.contains("empty") {
                        self.writeln(&format!("static {} empty() {{ return {}{{0}}; }}", n, n));
                    }
                    if !merged_methods.contains("all") {
                        self.writeln(&format!(
                            "static {} all() {{ return {}{{static_cast<{}>(~0)}}; }}",
                            n, n, bits_cpp_type
                        ));
                    }
                    if !merged_methods.contains("from_bits_retain") {
                        self.writeln(&format!(
                            "static {} from_bits_retain({} bits) {{ return {}{{bits}}; }}",
                            n, bits_cpp_type, n
                        ));
                    }
                    if !merged_methods.contains("is_empty") {
                        self.writeln("bool is_empty() const { return this->_0 == 0; }");
                    }
                    if !merged_methods.contains("contains") {
                        self.writeln(&format!(
                            "bool contains(const {}& other) const {{ return (this->_0 & other._0) == other._0; }}",
                            n
                        ));
                    }
                    if !merged_methods.contains("intersects") {
                        self.writeln(&format!(
                            "bool intersects(const {}& other) const {{ return (this->_0 & other._0) != 0; }}",
                            n
                        ));
                    }
                    if !merged_methods.contains("complement") {
                        if merged_methods.contains("from_bits_truncate") {
                            self.writeln(&format!(
                                "{} complement() const {{ return {}::from_bits_truncate(static_cast<decltype(this->_0)>(~this->_0)); }}",
                                n, n
                            ));
                        } else {
                            self.writeln(&format!(
                                "{} complement() const {{ return {}{{static_cast<decltype(this->_0)>(~this->_0)}}; }}",
                                n, n
                            ));
                        }
                    }
                    if !merged_methods.contains("is_all") {
                        self.writeln(&format!(
                            "bool is_all() const {{ return (this->_0 & {}::all()._0) == {}::all()._0; }}",
                            n, n
                        ));
                    }
                    if !merged_methods.contains("insert") {
                        self.writeln(&format!(
                            "void insert({} other) {{ this->_0 |= other._0; }}",
                            n
                        ));
                    }
                    if !merged_methods.contains("remove") {
                        self.writeln(&format!(
                            "void remove({} other) {{ this->_0 &= ~other._0; }}",
                            n
                        ));
                    }
                    if !merged_methods.contains("toggle") {
                        self.writeln(&format!(
                            "void toggle({} other) {{ this->_0 ^= other._0; }}",
                            n
                        ));
                    }
                    if !merged_methods.contains("set") {
                        self.writeln(&format!(
                            "void set({} other, bool value) {{ if (value) {{ insert(std::move(other)); }} else {{ remove(std::move(other)); }} }}",
                            n
                        ));
                    }
                    if !merged_methods.contains("intersection") {
                        self.writeln(&format!(
                            "{} intersection({} other) const {{ return {}{{static_cast<decltype(this->_0)>(this->_0 & other._0)}}; }}",
                            n, n, n
                        ));
                    }
                    if !merged_methods.contains("union_") {
                        self.writeln(&format!(
                            "{} union_({} other) const {{ return {}{{static_cast<decltype(this->_0)>(this->_0 | other._0)}}; }}",
                            n, n, n
                        ));
                    }
                    if !merged_methods.contains("difference") {
                        self.writeln(&format!(
                            "{} difference({} other) const {{ return {}{{static_cast<decltype(this->_0)>(this->_0 & ~other._0)}}; }}",
                            n, n, n
                        ));
                    }
                    if !merged_methods.contains("symmetric_difference") {
                        self.writeln(&format!(
                            "{} symmetric_difference({} other) const {{ return {}{{static_cast<decltype(this->_0)>(this->_0 ^ other._0)}}; }}",
                            n, n, n
                        ));
                    }
                    // iter: iterate over individual set flags using FLAGS constant
                    if !merged_methods.contains("iter") {
                        self.writeln(&format!(
                            "rusty::Vec<{n}> iter() const {{ rusty::Vec<{n}> result; {n} rem = *this; for (size_t i = 0; i < FLAGS.size(); i++) {{ if (FLAGS[i].name().empty()) {{ continue; }} const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) {{ result.push(flag); rem.remove(flag); }} }} if (!rem.is_empty()) {{ result.push(rem); }} return result; }}",
                            n = n
                        ));
                    }
                    // iter_names: iterate over (name, flag) pairs for set flags
                    // Returns a wrapper struct supporting both range-for and remaining().
                    if !merged_methods.contains("iter_names") {
                        self.writeln(&format!(
                            "auto iter_names() const {{ struct IterNames {{ rusty::Vec<std::tuple<std::string_view, {n}>> items; {n} remaining_; auto begin() const {{ return items.begin(); }} auto end() const {{ return items.end(); }} {n} remaining() const {{ return remaining_; }} }}; {n} rem = *this; rusty::Vec<std::tuple<std::string_view, {n}>> v; for (size_t i = 0; i < FLAGS.size(); i++) {{ if (FLAGS[i].name().empty()) {{ continue; }} const auto flag = FLAGS[i].value(); if (this->contains(flag) && rem.intersects(flag)) {{ v.push(std::make_tuple(FLAGS[i].name(), flag)); rem.remove(flag); }} }} return IterNames{{std::move(v), rem}}; }}",
                            n = n
                        ));
                    }
                    if !merged_methods.contains("to_string") {
                        self.writeln(&format!(
                            "std::string to_string() const {{ rusty::fmt::Formatter f; f.write_str(\"{n}(\"); bool first = true; const auto iter = this->iter_names(); for (auto&& [name, _] : rusty::for_in(rusty::iter(iter))) {{ if (!first) {{ f.write_str(\" | \"); }} first = false; f.write_str(name); }} const auto remaining = iter.remaining(); if (!remaining.is_empty()) {{ if (!first) {{ f.write_str(\" | \"); }} f.write_str(\"0x\"); f.write_str(std::format(\"{{0:x}}\", rusty::format_numeric_arg(remaining))); }} else if (first) {{ f.write_str(\"0x0\"); }} f.write_str(\")\"); return f.str(); }}",
                            n = n
                        ));
                    }
                    // extend: OR in flags from an iterator
                    if !merged_methods.contains("extend") {
                        self.writeln(&format!(
                            "template<typename Iter> void extend(Iter&& iter) {{ for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) {{ if constexpr (requires {{ item._0; }}) {{ this->_0 |= static_cast<decltype(this->_0)>(item._0); }} else if constexpr (requires {{ item.bits(); }}) {{ this->_0 |= static_cast<decltype(this->_0)>(item.bits()); }} else {{ this->_0 |= static_cast<decltype(this->_0)>(item); }} }} }}",
                        ));
                    }
                    // from_iter: collect iterator elements via bitwise OR
                    if !merged_methods.contains("from_iter") {
                        self.writeln(&format!(
                            "template<typename Iter> static {} from_iter(Iter&& iter) {{ {} result{{}}; for (auto&& item : rusty::for_in(std::forward<Iter>(iter))) {{ if constexpr (requires {{ item._0; }}) {{ result._0 |= static_cast<decltype(result._0)>(item._0); }} else if constexpr (requires {{ item.bits(); }}) {{ result._0 |= static_cast<decltype(result._0)>(item.bits()); }} else {{ result._0 |= static_cast<decltype(result._0)>(item); }} }} return result; }}",
                            n, n
                        ));
                    }
                }
            }
        }

        self.indent -= 1;
        self.writeln("};");

        // Emit deferred self-referential const definitions after struct body.
        // E.g., `inline const Prerelease Prerelease::EMPTY = Prerelease(...);`
        if !self.deferred_self_const_defs.is_empty() {
            let deferred = std::mem::take(&mut self.deferred_self_const_defs);
            let (non_flags, flags): (Vec<String>, Vec<String>) = deferred
                .into_iter()
                .partition(|def| !def.contains("::FLAGS ="));
            for def in non_flags.iter().chain(flags.iter()) {
                self.writeln(def);
            }
        }

        // Post-struct derives (Hash specialization)
        if derives.contains(&"Hash".to_string()) {
            self.newline();
            self.writeln("template<>");
            self.writeln(&format!("struct std::hash<{}> {{", name));
            self.indent += 1;
            self.writeln(&format!(
                "size_t operator()(const {}& v) const {{ return 0; /* TODO: hash fields */ }}",
                name
            ));
            self.indent -= 1;
            self.writeln("};");
        }
        self.pop_type_param_scope();
    }

    pub(super) fn emit_enum(&mut self, e: &syn::ItemEnum) {
        let name = &e.ident;
        let has_data = e.variants.iter().any(|v| !v.fields.is_empty());
        let is_local_scope = self.block_depth > 0;
        let include_unscoped = self.module_stack.is_empty() || is_local_scope;
        let enum_name = name.to_string();
        let enum_export_prefix = if self.should_export_item(&e.vis, &enum_name) {
            "export "
        } else {
            ""
        };
        let scoped_enum_name = self.scoped_type_key(&enum_name);
        if has_data && self.enum_uses_struct_wrapper(e) {
            if include_unscoped {
                self.data_enum_wrapper_types.insert(enum_name.clone());
            }
            self.data_enum_wrapper_types
                .insert(scoped_enum_name.clone());
        }
        if has_data {
            if include_unscoped {
                self.data_enum_types.insert(enum_name.clone());
            }
            self.data_enum_types.insert(scoped_enum_name.clone());
            let variants: HashSet<String> = e
                .variants
                .iter()
                .map(|variant| variant.ident.to_string())
                .collect();
            if include_unscoped {
                self.data_enum_variants_by_enum
                    .entry(enum_name.clone())
                    .or_default()
                    .extend(variants.iter().cloned());
            }
            self.data_enum_variants_by_enum
                .entry(scoped_enum_name.clone())
                .or_default()
                .extend(variants.iter().cloned());
            let mut variant_indices: HashMap<String, usize> = HashMap::new();
            for (idx, variant) in e.variants.iter().enumerate() {
                let raw = variant.ident.to_string();
                let canonical = self.canonical_variant_name(&raw).to_string();
                variant_indices.insert(raw.clone(), idx);
                variant_indices.insert(canonical.clone(), idx);
                let field_types: Vec<syn::Type> = match &variant.fields {
                    syn::Fields::Named(fields) => {
                        fields.named.iter().map(|field| field.ty.clone()).collect()
                    }
                    syn::Fields::Unnamed(fields) => fields
                        .unnamed
                        .iter()
                        .map(|field| field.ty.clone())
                        .collect(),
                    syn::Fields::Unit => Vec::new(),
                };
                let mut variant_type_keys = vec![
                    format!("{}::{}", enum_name, raw.clone()),
                    format!("{}::{}", scoped_enum_name, variant.ident),
                ];
                if canonical != raw {
                    variant_type_keys.push(format!("{}::{}", enum_name, canonical.clone()));
                    variant_type_keys.push(format!("{}::{}", scoped_enum_name, canonical.clone()));
                }
                for key in variant_type_keys {
                    std::rc::Rc::make_mut(&mut self.data_enum_variant_field_types)
                        .insert(key, field_types.clone());
                }
            }
            if include_unscoped {
                self.data_enum_variant_indices_by_enum
                    .entry(enum_name.clone())
                    .or_default()
                    .extend(
                        variant_indices
                            .iter()
                            .map(|(name, idx)| (name.clone(), *idx)),
                    );
            }
            self.data_enum_variant_indices_by_enum
                .entry(scoped_enum_name.clone())
                .or_default()
                .extend(
                    variant_indices
                        .iter()
                        .map(|(name, idx)| (name.clone(), *idx)),
                );
            self.data_enum_variant_names
                .extend(variants.iter().cloned());
            // Track which variants are unit variants (no fields)
            for variant in &e.variants {
                if variant.fields.is_empty() {
                    self.data_enum_unit_variants
                        .insert(format!("{}_{}", enum_name, variant.ident));
                    self.data_enum_unit_variants
                        .insert(format!("{}_{}", scoped_enum_name, variant.ident));
                }
            }
        } else {
            self.data_enum_types.remove(&scoped_enum_name);
            self.data_enum_variants_by_enum.remove(&scoped_enum_name);
            self.data_enum_variant_indices_by_enum
                .remove(&scoped_enum_name);
        }
        // Compute the where-bound `requires (...)` constraints BEFORE pushing the
        // type-param scope. collect_emitted_template_parts filters out params that
        // are "already visible for emission" (it returns no constraints once the
        // param list is empty), so after push_type_param_scope this would yield
        // nothing. The wrapper-struct DEFINITION must repeat the same requires
        // clause its forward declaration emits (via emit_template_declaration_
        // without_type_defaults → the same helper) or C++ rejects the redeclaration.
        let wrapper_template_constraints: Vec<String> = {
            let prev = self.in_constraint_emit.get();
            self.in_constraint_emit.set(true);
            let (_, constraints) = self.collect_emitted_template_parts(&e.generics, false);
            self.in_constraint_emit.set(prev);
            constraints
        };
        self.push_type_param_scope(&e.generics);

        // Collect type parameters (skip lifetimes)
        let type_params: Vec<String> = e
            .generics
            .params
            .iter()
            .filter_map(|p| {
                if let syn::GenericParam::Type(tp) = p {
                    Some(tp.ident.to_string())
                } else {
                    None
                }
            })
            .collect();
        self.enum_type_params
            .insert(name.to_string(), type_params.clone());
        let has_generics = !type_params.is_empty();
        let template_prefix = if has_generics {
            format!(
                "template<{}>",
                type_params
                    .iter()
                    .map(|p| format!("typename {}", p))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            String::new()
        };
        let template_args = if has_generics {
            format!("<{}>", type_params.join(", "))
        } else {
            String::new()
        };
        let enum_owner_name = name.to_string();

        if has_data {
            // Check if any variant references the enum type itself (recursive)
            let is_recursive = e
                .variants
                .iter()
                .any(|v| self.variant_references_type(v, &name.to_string()));

            // Emit forward declarations for recursive enums
            if is_recursive {
                if has_generics {
                    self.writeln(&format!("{}", template_prefix));
                }
                self.writeln(&format!(
                    "struct {};  // forward declaration for recursion",
                    name
                ));
            }

            // Enum with data → per-variant structs + std::variant
            self.writeln("// Algebraic data type");
            for variant in &e.variants {
                let vname = &variant.ident;
                let variant_struct_name = format!("{}_{}", name, vname);
                self.local_declared_types
                    .insert(variant_struct_name.clone());
                self.declared_type_params
                    .insert(variant_struct_name.clone(), type_params.clone());
                self.declared_type_param_kinds.insert(
                    variant_struct_name.clone(),
                    vec![GenericParamKind::Type; type_params.len()],
                );
                self.declared_type_param_defaults
                    .insert(variant_struct_name.clone(), vec![None; type_params.len()]);
                if !self.module_stack.is_empty() {
                    let scoped_variant_struct_name =
                        format!("{}::{}", self.module_stack.join("::"), variant_struct_name);
                    self.local_declared_types
                        .insert(scoped_variant_struct_name.clone());
                    self.declared_type_params
                        .insert(scoped_variant_struct_name.clone(), type_params.clone());
                    self.declared_type_param_kinds.insert(
                        scoped_variant_struct_name.clone(),
                        vec![GenericParamKind::Type; type_params.len()],
                    );
                    self.declared_type_param_defaults
                        .insert(scoped_variant_struct_name, vec![None; type_params.len()]);
                }
                match &variant.fields {
                    syn::Fields::Named(fields) => {
                        if has_generics {
                            if enum_export_prefix.is_empty() {
                                self.writeln(&template_prefix);
                            } else {
                                self.writeln(&format!("{}{}", enum_export_prefix, template_prefix));
                            }
                            self.writeln(&format!("struct {}_{} {{", name, vname));
                        } else {
                            self.writeln(&format!(
                                "{}struct {}_{} {{",
                                enum_export_prefix, name, vname
                            ));
                        }
                        self.indent += 1;
                        let variant_name = vname.to_string();
                        let mut named_field_cpp_names: HashMap<String, String> = HashMap::new();
                        for field in &fields.named {
                            let fname = field.ident.as_ref().unwrap();
                            let rust_field_name = fname.to_string();
                            let cpp_field_name = escape_cpp_keyword(&rust_field_name);
                            named_field_cpp_names
                                .insert(rust_field_name.clone(), cpp_field_name.clone());
                            let field_key = Self::format_by_value_field_name(
                                Some(variant_name.as_str()),
                                &rust_field_name,
                            );
                            let ftype = self.map_field_type_with_by_value_cycle_breaking_rewrite(
                                &enum_owner_name,
                                &field_key,
                                &field.ty,
                            );
                            self.writeln(&format!("{} {};", ftype, cpp_field_name));
                        }
                        let variant_struct_name = format!("{}_{}", name, vname);
                        std::rc::Rc::make_mut(&mut self.struct_field_cpp_names)
                            .insert(variant_struct_name.clone(), named_field_cpp_names.clone());
                        if !self.module_stack.is_empty() {
                            std::rc::Rc::make_mut(&mut self.struct_field_cpp_names).insert(
                                format!(
                                    "{}::{}",
                                    self.module_stack.join("::"),
                                    variant_struct_name
                                ),
                                named_field_cpp_names,
                            );
                        }
                        self.indent -= 1;
                        self.writeln("};");
                    }
                    syn::Fields::Unnamed(fields) => {
                        if has_generics {
                            if enum_export_prefix.is_empty() {
                                self.writeln(&template_prefix);
                            } else {
                                self.writeln(&format!("{}{}", enum_export_prefix, template_prefix));
                            }
                            self.writeln(&format!("struct {}_{} {{", name, vname));
                        } else {
                            self.writeln(&format!(
                                "{}struct {}_{} {{",
                                enum_export_prefix, name, vname
                            ));
                        }
                        self.indent += 1;
                        let variant_name = vname.to_string();
                        for (i, field) in fields.unnamed.iter().enumerate() {
                            let field_key = Self::format_by_value_field_name(
                                Some(variant_name.as_str()),
                                &format!("#{}", i),
                            );
                            let ftype = self.map_field_type_with_by_value_cycle_breaking_rewrite(
                                &enum_owner_name,
                                &field_key,
                                &field.ty,
                            );
                            self.writeln(&format!("{} _{};", ftype, i));
                        }
                        self.indent -= 1;
                        self.writeln("};");
                    }
                    syn::Fields::Unit => {
                        if has_generics {
                            if enum_export_prefix.is_empty() {
                                self.writeln(&template_prefix);
                            } else {
                                self.writeln(&format!("{}{}", enum_export_prefix, template_prefix));
                            }
                            self.writeln(&format!("struct {}_{} {{}};", name, vname));
                        } else {
                            self.writeln(&format!(
                                "{}struct {}_{} {{}};",
                                enum_export_prefix, name, vname
                            ));
                        }
                    }
                }
            }
            // Emit the variant type — with template args if generic
            let variant_list: Vec<String> = e
                .variants
                .iter()
                .map(|v| {
                    if has_generics {
                        format!("{}_{}{}", name, v.ident, template_args)
                    } else {
                        format!("{}_{}", name, v.ident)
                    }
                })
                .collect();

            // Predeclare variant constructor helpers so template methods emitted
            // inside the wrapper can reference `Left<...>/Right<...>` before the
            // helper definitions appear later in the file.
            if !is_local_scope {
                for variant in &e.variants {
                    let vname = &variant.ident;
                    if !self.should_emit_data_enum_variant_ctor_helper(&vname.to_string()) {
                        continue;
                    }
                    let variant_struct = if has_generics {
                        format!("{}_{}{}", name, vname, template_args)
                    } else {
                        format!("{}_{}", name, vname)
                    };
                    match &variant.fields {
                        syn::Fields::Unnamed(fields) => {
                            let variant_name = vname.to_string();
                            let ctor_name = variant_name.clone();
                            let params: Vec<String> = fields
                                .unnamed
                                .iter()
                                .enumerate()
                                .map(|(i, f)| {
                                    let ty = self.map_variant_ctor_param_type_for_field(
                                        &enum_owner_name,
                                        &variant_name,
                                        &format!("#{}", i),
                                        &f.ty,
                                        &ctor_name,
                                    );
                                    format!("{} _{}", ty, i)
                                })
                                .collect();
                            if has_generics {
                                self.writeln(&template_prefix);
                            }
                            self.writeln(&format!(
                                "{} {}({});",
                                variant_struct,
                                vname,
                                params.join(", ")
                            ));
                        }
                        syn::Fields::Named(fields) => {
                            let variant_name = vname.to_string();
                            let ctor_name = variant_name.clone();
                            let params: Vec<String> = fields
                                .named
                                .iter()
                                .map(|f| {
                                    let rust_field_name = f.ident.as_ref().unwrap().to_string();
                                    let cpp_field_name = escape_cpp_keyword(&rust_field_name);
                                    let ftype = self.map_variant_ctor_param_type_for_field(
                                        &enum_owner_name,
                                        &variant_name,
                                        &rust_field_name,
                                        &f.ty,
                                        &ctor_name,
                                    );
                                    format!("{} {}", ftype, cpp_field_name)
                                })
                                .collect();
                            if has_generics {
                                self.writeln(&template_prefix);
                            }
                            self.writeln(&format!(
                                "{} {}({});",
                                variant_struct,
                                vname,
                                params.join(", ")
                            ));
                        }
                        syn::Fields::Unit => {
                            if has_generics {
                                self.writeln(&template_prefix);
                            }
                            self.writeln(&format!("{} {}();", variant_struct, vname));
                        }
                    }
                }
            }

            // Use struct wrapper if recursive OR has impl blocks (so methods can be added)
            let has_impls = self.has_impls_for_type(&name.to_string());
            if is_recursive || has_impls {
                let variant_type = format!("std::variant<{}>", variant_list.join(", "));
                if has_generics {
                    self.writeln(&template_prefix);
                    // Repeat the where-bound `requires (...)` clause on the
                    // wrapper struct DEFINITION so it matches the forward
                    // declaration emitted by the pre-pass. C++ rejects a
                    // definition whose requires-clause differs from a prior
                    // declaration; the data-enum prefix is built from
                    // type_params only and omits the constraint, so emit it here.
                    if !wrapper_template_constraints.is_empty() {
                        self.writeln(&format!(
                            "    requires ({})",
                            wrapper_template_constraints.join(" && ")
                        ));
                    }
                }
                self.writeln(&format!("struct {} : {} {{", name, variant_type));
                self.indent += 1;
                self.writeln(&format!("using variant = {};", variant_type));
                self.writeln("using variant::variant;");
                let enum_wrapper_type = if has_generics {
                    format!("{}{}", name, template_args)
                } else {
                    name.to_string()
                };
                for variant in &e.variants {
                    let vname = &variant.ident;
                    let variant_struct = if has_generics {
                        format!("{}_{}{}", name, vname, template_args)
                    } else {
                        format!("{}_{}", name, vname)
                    };
                    match &variant.fields {
                        syn::Fields::Unnamed(fields) => {
                            let variant_name = vname.to_string();
                            let params: Vec<String> = fields
                                .unnamed
                                .iter()
                                .enumerate()
                                .map(|(i, f)| {
                                    let field_name = format!("#{}", i);
                                    let ty = self.map_variant_ctor_param_type_for_field(
                                        &enum_owner_name,
                                        &variant_name,
                                        &field_name,
                                        &f.ty,
                                        &variant_name,
                                    );
                                    format!("{} _{}", ty, i)
                                })
                                .collect();
                            let args: Vec<String> = fields
                                .unnamed
                                .iter()
                                .enumerate()
                                .map(|(i, f)| {
                                    let field_name = format!("#{}", i);
                                    let forwarded = format!("std::forward<decltype(_{i})>(_{i})");
                                    let param_type = self.map_variant_ctor_param_type_for_field(
                                        &enum_owner_name,
                                        &variant_name,
                                        &field_name,
                                        &f.ty,
                                        &variant_name,
                                    );
                                    self.wrap_variant_ctor_field_initializer(
                                        &enum_owner_name,
                                        &variant_name,
                                        &field_name,
                                        &param_type,
                                        forwarded,
                                    )
                                })
                                .collect();
                            self.writeln(&format!(
                                "static {} {}({}) {{ return {}{{{}{{{}}}}}; }}",
                                enum_wrapper_type,
                                vname,
                                params.join(", "),
                                enum_wrapper_type,
                                variant_struct,
                                args.join(", ")
                            ));
                        }
                        syn::Fields::Named(fields) => {
                            let variant_name = vname.to_string();
                            let params: Vec<String> = fields
                                .named
                                .iter()
                                .map(|f| {
                                    let rust_field_name = f.ident.as_ref().unwrap().to_string();
                                    let cpp_field_name = escape_cpp_keyword(&rust_field_name);
                                    let ftype = self.map_variant_ctor_param_type_for_field(
                                        &enum_owner_name,
                                        &variant_name,
                                        &rust_field_name,
                                        &f.ty,
                                        &variant_name,
                                    );
                                    format!("{} {}", ftype, cpp_field_name)
                                })
                                .collect();
                            let args: Vec<String> = fields
                                .named
                                .iter()
                                .map(|f| {
                                    let rust_field_name = f.ident.as_ref().unwrap().to_string();
                                    let cpp_field_name = escape_cpp_keyword(&rust_field_name);
                                    let forwarded = format!(
                                        "std::forward<decltype({})>({})",
                                        cpp_field_name, cpp_field_name
                                    );
                                    let param_type = self.map_variant_ctor_param_type_for_field(
                                        &enum_owner_name,
                                        &variant_name,
                                        &rust_field_name,
                                        &f.ty,
                                        &variant_name,
                                    );
                                    let wrapped = self.wrap_variant_ctor_field_initializer(
                                        &enum_owner_name,
                                        &variant_name,
                                        &rust_field_name,
                                        &param_type,
                                        forwarded,
                                    );
                                    format!(".{} = {}", cpp_field_name, wrapped)
                                })
                                .collect();
                            self.writeln(&format!(
                                "static {} {}({}) {{ return {}{{{}{{{}}}}}; }}",
                                enum_wrapper_type,
                                vname,
                                params.join(", "),
                                enum_wrapper_type,
                                variant_struct,
                                args.join(", ")
                            ));
                        }
                        syn::Fields::Unit => {
                            self.writeln(&format!(
                                "static {} {}() {{ return {}{{{}{{}}}}; }}",
                                enum_wrapper_type, vname, enum_wrapper_type, variant_struct
                            ));
                        }
                    }
                }
                self.newline();

                // Merge impl block methods into the enum struct
                if let Some(methods) = self.take_impls_for_type(&name.to_string()) {
                    self.newline();
                    let prev_struct = self.current_struct.clone();
                    self.current_struct = Some(name.to_string());
                    self.emitted_method_conflict_keys.push(HashSet::new());
                    self.emitted_non_method_member_names.push(HashSet::new());
                    let enum_has_emitted_template_params = e.generics.params.iter().any(|param| {
                        matches!(
                            param,
                            syn::GenericParam::Type(_) | syn::GenericParam::Const(_)
                        )
                    });
                    let can_defer_enum_method_definitions = self.block_depth == 0
                        && !enum_has_emitted_template_params
                        && !self.deferred_method_definitions_stack.is_empty();
                    for impl_item in &methods {
                        if can_defer_enum_method_definitions
                            && let syn::ImplItem::Fn(method) = impl_item
                        {
                            let prev_decl_only = self.method_emission_declaration_only;
                            let prev_out_of_line_owner =
                                self.method_emission_out_of_line_owner.clone();
                            let prev_skip_conflict =
                                self.method_emission_skip_conflict_registration;

                            self.method_emission_declaration_only = true;
                            self.method_emission_out_of_line_owner = None;
                            self.method_emission_skip_conflict_registration = false;
                            let declaration_output_len = self.output.len();
                            self.emit_method(method);
                            let declaration_emitted = self.output.len() != declaration_output_len;
                            if !declaration_emitted {
                                self.method_emission_declaration_only = prev_decl_only;
                                self.method_emission_out_of_line_owner = prev_out_of_line_owner;
                                self.method_emission_skip_conflict_registration =
                                    prev_skip_conflict;
                                continue;
                            }

                            let owner_cpp_name = escape_cpp_keyword(&name.to_string());
                            self.method_emission_declaration_only = false;
                            self.method_emission_out_of_line_owner = Some(owner_cpp_name);
                            self.method_emission_skip_conflict_registration = true;
                            let saved_output = std::mem::take(&mut self.output);
                            let saved_indent = self.indent;
                            self.output = String::new();
                            self.indent = saved_indent.saturating_sub(1);
                            self.push_deferred_method_definition_scope();
                            self.emit_method(method);
                            let mut deferred_definition = std::mem::take(&mut self.output);
                            let nested_deferred_defs = self
                                .deferred_method_definitions_stack
                                .pop()
                                .unwrap_or_default();
                            for nested in nested_deferred_defs {
                                if !deferred_definition.is_empty()
                                    && !deferred_definition.ends_with('\n')
                                {
                                    deferred_definition.push('\n');
                                }
                                deferred_definition.push_str(&nested);
                            }
                            self.output = saved_output;
                            self.indent = saved_indent;

                            self.method_emission_declaration_only = prev_decl_only;
                            self.method_emission_out_of_line_owner = prev_out_of_line_owner;
                            self.method_emission_skip_conflict_registration = prev_skip_conflict;

                            self.queue_deferred_method_definition(deferred_definition);
                            continue;
                        }
                        self.emit_impl_item(impl_item);
                    }
                    self.emitted_non_method_member_names.pop();
                    self.emitted_method_conflict_keys.pop();
                    self.current_struct = prev_struct;
                }

                self.indent -= 1;
                self.writeln("};");
            } else {
                if has_generics {
                    self.writeln(&template_prefix);
                }
                self.writeln(&format!(
                    "using {} = std::variant<{}>;",
                    name,
                    variant_list.join(", ")
                ));
            }

            // Emit constructor helper functions for each variant.
            // Return the variant struct directly — std::variant implicitly converts.
            // This avoids template arg deduction issues (e.g., Left(2) can't deduce R).
            for variant in &e.variants {
                let vname = &variant.ident;
                if !self.should_emit_data_enum_variant_ctor_helper(&vname.to_string()) {
                    continue;
                }
                let variant_struct = if has_generics {
                    format!("{}_{}{}", name, vname, template_args)
                } else {
                    format!("{}_{}", name, vname)
                };

                match &variant.fields {
                    syn::Fields::Unnamed(fields) => {
                        let variant_name = vname.to_string();
                        let ctor_name = vname.to_string();
                        let params: Vec<String> = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(i, f)| {
                                let ty = self.map_variant_ctor_param_type_for_field(
                                    &enum_owner_name,
                                    &variant_name,
                                    &format!("#{}", i),
                                    &f.ty,
                                    &ctor_name,
                                );
                                format!("{} _{}", ty, i)
                            })
                            .collect();
                        let args: Vec<String> = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(i, f)| {
                                let field_name = format!("#{}", i);
                                let ty = self.map_variant_ctor_param_type_for_field(
                                    &enum_owner_name,
                                    &variant_name,
                                    &field_name,
                                    &f.ty,
                                    &ctor_name,
                                );
                                let param = format!("_{}", i);
                                let forwarded = format!("std::forward<{}>({})", ty, param);
                                self.wrap_variant_ctor_field_initializer(
                                    &enum_owner_name,
                                    &variant_name,
                                    &field_name,
                                    &ty,
                                    forwarded,
                                )
                            })
                            .collect();
                        if is_local_scope {
                            self.writeln(&format!(
                                "const auto {} = [&]( {}) {{ return {}{{{}}};  }};",
                                vname,
                                params.join(", "),
                                variant_struct,
                                args.join(", ")
                            ));
                        } else {
                            if has_generics {
                                self.writeln(&template_prefix);
                            }
                            self.writeln(&format!(
                                "{} {}({}) {{ return {}{{{}}};  }}",
                                variant_struct,
                                vname,
                                params.join(", "),
                                variant_struct,
                                args.join(", ")
                            ));
                        }
                    }
                    syn::Fields::Named(fields) => {
                        let variant_name = vname.to_string();
                        let ctor_name = vname.to_string();
                        let params: Vec<String> = fields
                            .named
                            .iter()
                            .map(|f| {
                                let rust_field_name = f.ident.as_ref().unwrap().to_string();
                                let cpp_field_name = escape_cpp_keyword(&rust_field_name);
                                let ftype = self.map_variant_ctor_param_type_for_field(
                                    &enum_owner_name,
                                    &variant_name,
                                    &rust_field_name,
                                    &f.ty,
                                    &ctor_name,
                                );
                                format!("{} {}", ftype, cpp_field_name)
                            })
                            .collect();
                        let args: Vec<String> = fields
                            .named
                            .iter()
                            .map(|f| {
                                let rust_field_name = f.ident.as_ref().unwrap().to_string();
                                let cpp_field_name = escape_cpp_keyword(&rust_field_name);
                                let ftype = self.map_variant_ctor_param_type_for_field(
                                    &enum_owner_name,
                                    &variant_name,
                                    &rust_field_name,
                                    &f.ty,
                                    &ctor_name,
                                );
                                let forwarded =
                                    format!("std::forward<{}>({})", ftype, cpp_field_name);
                                let wrapped = self.wrap_variant_ctor_field_initializer(
                                    &enum_owner_name,
                                    &variant_name,
                                    &rust_field_name,
                                    &ftype,
                                    forwarded,
                                );
                                format!(".{} = {}", cpp_field_name, wrapped)
                            })
                            .collect();
                        if is_local_scope {
                            self.writeln(&format!(
                                "const auto {} = [&]( {}) {{ return {}{{{}}};  }};",
                                vname,
                                params.join(", "),
                                variant_struct,
                                args.join(", ")
                            ));
                        } else {
                            if has_generics {
                                self.writeln(&template_prefix);
                            }
                            self.writeln(&format!(
                                "{} {}({}) {{ return {}{{{}}};  }}",
                                variant_struct,
                                vname,
                                params.join(", "),
                                variant_struct,
                                args.join(", ")
                            ));
                        }
                    }
                    syn::Fields::Unit => {
                        if is_local_scope {
                            self.writeln(&format!(
                                "const auto {} = [&]() {{ return {}{{}};  }};",
                                vname, variant_struct
                            ));
                        } else {
                            if has_generics {
                                self.writeln(&template_prefix);
                            }
                            self.writeln(&format!(
                                "{} {}() {{ return {}{{}};  }}",
                                variant_struct, vname, variant_struct
                            ));
                        }
                    }
                }
            }
        } else {
            // C-like enum → enum class
            let enum_name = name.to_string();
            let scoped_enum_name = self.scoped_type_key(&enum_name);
            let export_prefix = if self.should_export_item(&e.vis, &enum_name) {
                "export "
            } else {
                ""
            };
            // Local Rust enums can reuse identifiers across function scopes.
            // Global duplicate-suppression is only valid for non-local emission.
            let already_defined = !is_local_scope
                && self
                    .forward_emitted_c_like_enums
                    .contains(&scoped_enum_name);
            if !already_defined {
                self.writeln(&format!("{}enum class {} {{", export_prefix, name));
                self.indent += 1;
                let variants = self.render_c_like_enum_variants(e);
                self.writeln(&variants.join(",\n    "));
                self.indent -= 1;
                self.writeln("};");
                if !is_local_scope {
                    let enum_cpp_name = escape_cpp_keyword(&name.to_string());
                    let type_params: Vec<String> = e
                        .generics
                        .params
                        .iter()
                        .filter_map(|p| match p {
                            syn::GenericParam::Type(tp) => Some(tp.ident.to_string()),
                            syn::GenericParam::Const(cp) => Some(cp.ident.to_string()),
                            _ => None,
                        })
                        .collect();
                    let enum_ty = if type_params.is_empty() {
                        enum_cpp_name.clone()
                    } else {
                        format!("{}<{}>", enum_cpp_name, type_params.join(", "))
                    };
                    for variant in &e.variants {
                        self.emit_template_prefix_without_type_defaults(&e.generics);
                        let helper_name = format!(
                            "{}_{}",
                            enum_cpp_name,
                            escape_cpp_keyword(&variant.ident.to_string())
                        );
                        let variant_cpp = escape_cpp_keyword(&variant.ident.to_string());
                        self.writeln(&format!(
                            "inline constexpr {} {}() {{ return {}::{}; }}",
                            enum_ty, helper_name, enum_ty, variant_cpp
                        ));
                    }
                }
                if !is_local_scope {
                    self.forward_emitted_c_like_enums
                        .insert(scoped_enum_name.clone());
                }
            }

            // Emit associated constants and display helpers from impl blocks for C-like enums.
            // C++ enum class can't have static members, so emit as standalone
            // inline constexpr with a namespaced name.
            let name_str = name.to_string();
            let scoped_name = self.scoped_type_key(&name_str);
            self.c_like_enum_types.insert(name_str.clone());
            self.c_like_enum_types.insert(scoped_name.clone());
            for variant in &e.variants {
                self.c_like_enum_variants
                    .insert(format!("{}_{}", name, variant.ident));
                self.c_like_enum_variants
                    .insert(format!("{}_{}", scoped_enum_name, variant.ident));
            }
            let enum_impl_items = self
                .impl_blocks
                .get(&scoped_name)
                .or_else(|| self.impl_blocks.get(&name_str))
                .cloned();
            if let Some(items) = enum_impl_items {
                for item in items.clone() {
                    if let syn::ImplItem::Const(c) = &item {
                        let const_name = escape_cpp_keyword(&c.ident.to_string());
                        let ty = self.map_type(&c.ty);
                        let expr = self.emit_expr_to_string_with_expected(&c.expr, Some(&c.ty));
                        // Use a namespace wrapper so `Op::DEFAULT` syntax works:
                        // namespace Op_ { inline constexpr Op DEFAULT = Op::Caret; }
                        // using Op_::DEFAULT; // — no, this doesn't help.
                        // Instead, emit as free-standing constant and rewrite
                        // path references from `Op::DEFAULT` to just `Op_DEFAULT`.
                        let key = format!("{}_{}", name, const_name);
                        self.c_like_enum_consts.insert(key);
                        self.writeln(&format!(
                            "inline constexpr {} {}_{} = {};",
                            ty, name, const_name, expr
                        ));
                    }
                    if let syn::ImplItem::Fn(method) = &item {
                        self.c_like_enum_inherent_method_names
                            .insert(method.sig.ident.to_string());
                        self.emit_c_like_enum_inherent_method_free_function(&name_str, method);
                        self.emit_c_like_enum_fmt_helper(&name_str, method);
                    }
                }
            }
        }
        self.pop_type_param_scope();
    }

    pub(super) fn emit_type_alias_once(&mut self, t: &syn::ItemType) -> bool {
        let alias_rust_name = t.ident.to_string();
        let name = escape_cpp_keyword(&alias_rust_name);
        let should_export_alias =
            self.block_depth == 0 && self.should_export_item(&t.vis, &alias_rust_name);
        let scoped_key = self.scoped_alias_key(&name);
        // Block-local `type` aliases are scoped per block in Rust and may reuse
        // the same alias name across different functions/blocks.
        // Keep global de-dup only for top-level/module aliases.
        if self.block_depth == 0 && !self.emitted_scoped_type_aliases.insert(scoped_key) {
            return false;
        }

        self.push_type_param_scope(&t.generics);
        let mut target = self.map_type(&t.ty);
        self.pop_type_param_scope();
        target = Self::rewrite_private_keyword_namespace_in_type_path(&target);
        if self.block_depth > 0 {
            self.emit_template_prefix(&t.generics);
            self.writeln(&format!("using {} [[maybe_unused]] = {};", name, target));
        } else {
            let export_prefix = if should_export_alias { "export " } else { "" };
            let declaration = format!("using {} = {};", name, target);
            self.emit_template_declaration_with_type_defaults(
                &t.generics,
                export_prefix,
                &declaration,
            );
            let owner_key = if self.module_stack.is_empty() {
                alias_rust_name.clone()
            } else {
                format!("{}::{}", self.module_stack.join("::"), alias_rust_name)
            };
            let _ = self.emit_type_alias_impl_free_function_decls_for_owner(&owner_key);
        }

        // Alias tracking only applies to concrete (non-generic) aliases.
        if t.generics.params.is_empty() {
            let alias = name.to_string();
            if is_numeric_cpp_scalar_type(&target) {
                self.numeric_type_aliases
                    .insert(alias.clone(), target.clone());
                self.numeric_type_aliases
                    .insert(self.scoped_type_key(&alias), target.clone());
            }
            if let syn::Type::Tuple(tuple_ty) = self.peel_paren_group_type(&t.ty) {
                let arity = tuple_ty.elems.len();
                self.tuple_type_aliases.insert(alias.clone(), arity);
                self.tuple_type_aliases
                    .insert(self.scoped_type_key(&alias), arity);
                let elem_tys: Vec<syn::Type> = tuple_ty.elems.iter().cloned().collect();
                self.tuple_type_alias_elem_types
                    .insert(alias.clone(), elem_tys.clone());
                self.tuple_type_alias_elem_types
                    .insert(self.scoped_type_key(&alias), elem_tys);
            }
        }
        true
    }

    pub(super) fn emit_type_alias(&mut self, t: &syn::ItemType) {
        let _ = self.emit_type_alias_once(t);
    }

    pub(super) fn emit_const(&mut self, c: &syn::ItemConst) {
        // Skip wildcard const bindings (`const _: () = ...;`) which are
        // Rust compile-time assertions or macro-internal scope blocks.
        if c.ident == "_" {
            return;
        }
        if self.is_thread_local_key_type(&c.ty) {
            self.writeln(&format!(
                "// Rust-only thread-local const skipped (no direct C++ LocalKey lowering): {}",
                c.ident
            ));
            return;
        }
        if self.block_depth == 0 {
            let rust_name = c.ident.to_string();
            let scoped_name = self.scoped_const_key(&rust_name);
            if self.forward_emitted_consts.contains(&scoped_name) {
                return;
            }
        }
        if self.is_rust_libtest_metadata_type(&c.ty) {
            let marker =
                Self::rustc_test_marker_name(&c.attrs).unwrap_or_else(|| c.ident.to_string());
            let should_panic = self.extract_libtest_should_panic_value(&c.expr);
            self.expanded_test_markers.push(marker.clone());
            if let Some(value) = should_panic {
                self.expanded_test_marker_should_panic
                    .insert(marker.clone(), value);
            }
            self.writeln(&format!(
                "// Rust-only libtest metadata const skipped: {} (marker: {}, should_panic: {})",
                c.ident,
                marker,
                match should_panic {
                    Some(true) => "yes",
                    Some(false) => "no",
                    None => "unknown",
                }
            ));
            return;
        }
        if self.is_local_new_const_constructor_call(&c.expr) {
            let rust_name = c.ident.to_string();
            let cpp_name = self.allocate_local_cpp_name(&rust_name);
            self.register_local_binding(rust_name.clone(), Some((*c.ty).clone()));
            self.record_local_const_binding(&rust_name, false);

            let ty = self.map_type(&c.ty);
            let expr = self.emit_expr_to_string_with_expected(&c.expr, Some(&c.ty));
            self.writeln(&format!("const auto {} = []() -> {} {{", cpp_name, ty));
            self.indent += 1;
            self.writeln(&format!("return {};", expr));
            self.indent -= 1;
            self.writeln("};");

            if let Some(scope) = self.local_cpp_bindings.last_mut() {
                scope.insert(rust_name, format!("{}()", cpp_name));
            }
            return;
        }
        let rust_name = c.ident.to_string();
        let cpp_name = escape_cpp_keyword(&rust_name);
        if self.block_depth > 0 {
            self.register_local_binding(rust_name.clone(), Some((*c.ty).clone()));
        }
        let ty = self.map_type(&c.ty);
        let expr = self.emit_expr_to_string_with_expected(&c.expr, Some(&c.ty));
        // `std::span` constants built from array literals need stable backing storage.
        // Emitting `constexpr std::span = std::array{...}` binds span to a temporary.
        if ty.starts_with("std::span<") && expr.trim_start().starts_with("std::array<") {
            if self.block_depth > 0 {
                self.record_local_const_binding(&rust_name, true);
                self.record_local_item_const_name(&rust_name);
            }
            let storage_name = format!("{}_storage", cpp_name);
            let storage_qualifier = if self.block_depth == 0 {
                "static const auto"
            } else {
                "const auto"
            };
            self.writeln(&format!(
                "{} {} = {};",
                storage_qualifier, storage_name, expr
            ));
            let export_prefix =
                if self.block_depth == 0 && self.should_export_item(&c.vis, &rust_name) {
                    "export "
                } else {
                    ""
                };
            self.writeln(&format!(
                "{}const {} {} = {};",
                export_prefix, ty, cpp_name, storage_name
            ));
            return;
        }
        let expr_requires_runtime_storage = expr.contains("thread_local ")
            || expr.contains(" static ")
            || expr.starts_with("[]()")
            || expr.contains("_slice_ref_tmp");
        let type_requires_runtime_const_storage =
            ty.contains("rusty::range_inclusive<") || ty.contains("rusty::range<");
        let storage = if self.block_depth > 0 {
            if (is_numeric_cpp_scalar_type(&ty)
                || matches!(ty.as_str(), "bool" | "float" | "double" | "long double"))
                && !expr_requires_runtime_storage
                && !type_requires_runtime_const_storage
            {
                "constexpr"
            } else {
                "const"
            }
        } else {
            if ty.contains('*')
                || ty.contains("std::span<")
                || expr_requires_runtime_storage
                || type_requires_runtime_const_storage
            {
                // Pointer constants often lower through reinterpret casts that are not
                // constexpr-friendly on all targets; prefer `const` storage.
                "const"
            } else {
                "constexpr"
            }
        };
        if self.block_depth > 0 {
            self.record_local_const_binding(&rust_name, true);
            self.record_local_item_const_name(&rust_name);
        }
        let export_prefix = if self.block_depth == 0 && self.should_export_item(&c.vis, &rust_name)
        {
            "export "
        } else {
            ""
        };
        if storage == "const" && ty.contains('*') {
            self.writeln(&format!(
                "{}{} const {} = {};",
                export_prefix, ty, cpp_name, expr
            ));
        } else {
            self.writeln(&format!(
                "{}{} {} {} = {};",
                export_prefix, storage, ty, cpp_name, expr
            ));
        }
    }

    pub(super) fn emit_static(&mut self, s: &syn::ItemStatic) {
        if self.is_rust_libtest_metadata_type(&s.ty) {
            self.writeln(&format!(
                "// Rust-only libtest metadata static skipped: {}",
                s.ident
            ));
            return;
        }
        let name = escape_cpp_keyword(&s.ident.to_string());
        if self.block_depth > 0 {
            self.register_local_binding(s.ident.to_string(), Some((*s.ty).clone()));
        }
        let ty = self.map_type(&s.ty);
        let expr = self.emit_expr_to_string_with_expected(&s.expr, Some(&s.ty));
        let storage = if self.block_depth > 0 {
            "static "
        } else {
            "inline "
        };
        self.writeln(&format!("{}{} {} = {};", storage, ty, name, expr));
    }

    pub(super) fn emit_trait(&mut self, t: &syn::ItemTrait) {
        // Interface + Adapter design (replaces Pro facade). See § 3.2.9 of
        // docs/rusty-cpp-transpiler.md.
        self.emit_trait_interface_pattern(t);
    }

    /// Emit a Rust trait as a plain C++ abstract base class plus undefined
    /// primary templates for the owning and reference adapters.
    ///
    /// See § 3.2.9 of `docs/rusty-cpp-transpiler.md` for the design.
    /// Phase 1 scope: ordinary traits with `&self` / `&mut self` methods only.
    /// Generic traits, associated types, default methods, by-value `self`,
    /// and static (no-receiver) methods are deferred to later phases and
    /// surface as `// TODO(interface_traits): ...` comments.
    pub(super) fn emit_trait_interface_pattern(&mut self, t: &syn::ItemTrait) {
        let trait_name = &t.ident;
        let trait_name_str = trait_name.to_string();

        // Visibility-aware anon-namespace wrapping.
        //
        // The original anon-namespace wrapper was added to handle the
        // case where two unrelated Rust crates each declare a trait
        // with the same name (e.g. `de::SeqAccess` in `serde_core` and
        // `serde_json`) and the transpiler produces two C++ classes
        // that the linker would otherwise see as conflicting. Wrapping
        // in `namespace {}` gives each trait class TU-local linkage.
        //
        // But that wrapper is wrong for `pub trait T` — a trait the
        // author intends to be visible outside the defining module.
        // Forcing TU-local linkage on a `pub` item silently breaks any
        // cross-module use. Rule:
        //   pub trait T (any pub form)   → emit at namespace scope
        //   trait T   (Visibility::Inherited) → wrap in `namespace {}`
        //
        // This mirrors what the transpiler effectively does for
        // structs (always namespace scope) but driven by the actual
        // `vis` field, which is the Rust idiom.
        let wrap_in_anon_ns = !Self::visibility_is_any_pub(&t.vis);

        // Marker traits: emit as a no-op concept, same as the Pro path.
        if matches!(
            trait_name_str.as_str(),
            "Send" | "Sync" | "Copy" | "Clone" | "Sized" | "Unpin"
        ) {
            self.writeln(&format!("// Marker trait: {}", trait_name));
            self.writeln("template<typename T>");
            self.writeln(&format!(
                "concept {} = true;  // marker trait — no runtime check",
                trait_name
            ));
            return;
        }

        // Collect the trait's own generic params (e.g., `T` in
        // `trait Container<T>`). They become template parameters on
        // both the Interface class and the three Adapter primary
        // templates. Const generics and lifetimes are skipped: the
        // analyzer-facing C++ interface is generic over type params only.
        let mut trait_generic_idents: Vec<String> = t
            .generics
            .params
            .iter()
            .filter_map(|p| match p {
                syn::GenericParam::Type(tp) => Some(tp.ident.to_string()),
                _ => None,
            })
            .collect();

        // Associated constants (`const FOO: T;`) are not yet supported.
        // They'd need to lower as virtual getter methods or as
        // template-parameter values, neither of which is implemented.
        // Skip the trait entirely with a TODO marker. Methods on the
        // implementing structs are still available as inherent methods
        // through the existing pipeline; only the dyn dispatch path is
        // unavailable.
        let has_assoc_const = t
            .items
            .iter()
            .any(|i| matches!(i, syn::TraitItem::Const(_)));
        if has_assoc_const {
            self.writeln(&format!(
                "// TODO(interface_traits): trait `{}` has associated constants, not yet supported",
                trait_name
            ));
            self.skipped_interface_traits
                .insert(trait_name.to_string());
            // Fall back to the Pro-path module-mode helper so callers like
            // `Trait::method(self, ...)` still resolve to the trait's
            // default-body static. Without this, the UFCS-trait-call
            // rewriter sees no receiver and turns the call into a
            // self-recursive `(*self).method(...)` (e.g., arrayvec's
            // `ArrayVecImpl::push` → infinite recursion).
            if self.emit_module_mode_trait_runtime_helper(t) {
                let scoped = self.scoped_type_key(&trait_name.to_string());
                self.module_runtime_helper_traits
                    .insert(trait_name.to_string());
                self.module_runtime_helper_traits.insert(scoped);
                // The helper no longer emits a `using {trait_name} =
                // {trait_name}RuntimeHelper;` alias unconditionally (it
                // collides with the trait class in the common case).
                // Emit it here, where the trait class was suppressed and
                // the alias is the only thing letting `Trait::method(...)`
                // call sites resolve to the helper.
                let trait_name_str = trait_name.to_string();
                let trait_cpp_name = escape_cpp_keyword(&trait_name_str);
                let helper_name = format!("{}RuntimeHelper", trait_cpp_name);
                if trait_cpp_name != helper_name && trait_cpp_name != "Serializer" {
                    self.writeln(&format!(
                        "using {} = {};",
                        trait_cpp_name, helper_name
                    ));
                }
            }
            return;
        }

        // If every method is going to be skipped (method-level generic,
        // by-value `self`, no receiver) AND the trait has no supertraits
        // worth inheriting, the resulting class would be a useless empty
        // shell that we can't dispatch through anyway. Skip emission and
        // mark as `skipped_interface_traits` so Adapter emission for impls
        // of this trait also skips. This avoids:
        //   - Duplicate `class T` declarations in dependent crates
        //     (cargo expand inlines re-exports of macro-defined traits).
        //   - Empty-class redefinition risk across modules.
        //   - Wasted transpile work for itertools-style trait-heavy crates.
        let any_emittable_method = t.items.iter().any(|item| {
            let syn::TraitItem::Fn(method) = item else {
                return false;
            };
            // Reject methods we'd skip downstream. Mirrors the per-method
            // skip checks in the emission loop below — keep these in sync.
            if !method.sig.generics.params.is_empty() {
                return false;
            }
            let receiver = method.sig.inputs.first();
            let Some(syn::FnArg::Receiver(r)) = receiver else {
                return false;
            };
            if r.reference.is_none() {
                return false;
            }
            true
        });
        // Count supertraits we'd actually inherit from. Marker traits,
        // operator traits, skipped traits, and foreign traits all get
        // filtered out later (see the supertrait collection block);
        // mirror that here so `: Sized` (Sized is filtered) doesn't
        // count as "has a supertrait".
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
            // Locally-declared trait? If so, it counts.
            self.trait_declared_path_by_short_name
                .contains_key(&name)
        });
        if !any_emittable_method && !has_emittable_supertrait {
            // Marker trait shape: no methods, no usable supertraits (e.g.
            // `trait Marker {}` or `trait Marker : Send {}`). Emit an empty
            // interface class so the type can still be named at use sites
            // (`&dyn Marker`, `Box<dyn Marker>`); no vtable contents.
            if wrap_in_anon_ns {
                self.writeln("namespace {");
            }
            self.writeln(&format!("class {} {{", trait_name));
            self.writeln("public:");
            self.indent += 1;
            self.writeln(&format!("virtual ~{}() noexcept(false) {{}}", trait_name));
            self.writeln(&format!("{}(const {}&) = delete;", trait_name, trait_name));
            self.writeln(&format!(
                "{}& operator=(const {}&) = delete;",
                trait_name, trait_name
            ));
            self.writeln(&format!("{}({}&&) = delete;", trait_name, trait_name));
            self.writeln(&format!(
                "{}& operator=({}&&) = delete;",
                trait_name, trait_name
            ));
            self.indent -= 1;
            self.writeln("protected:");
            self.indent += 1;
            self.writeln(&format!("{}() = default;", trait_name));
            self.indent -= 1;
            self.writeln("};");
            if wrap_in_anon_ns {
                self.writeln("}");
            }
            self.writeln("");
            // Module-mode static helper for qualified UFCS calls — see the
            // same call below for the substantive comment.
            if self.emit_module_mode_trait_runtime_helper(t) {
                let scoped = self.scoped_type_key(&trait_name.to_string());
                self.module_runtime_helper_traits
                    .insert(trait_name.to_string());
                self.module_runtime_helper_traits.insert(scoped);
            }
            return;
        }

        // Collect associated type declarations (`type Item;`) and add
        // them as additional template parameters. `Self::Item` then
        // becomes a class template parameter that the impl binds
        // concretely via the Adapter specialization.
        //
        // Distinguish two cases for adapter-emit gating:
        //   - REAL generics (`trait Foo<T>`): adapter partial specs need
        //     full template headers we don't yet emit → flag as
        //     `interface_traits_with_generics` (causes adapter bail).
        //   - Associated-types-only (`trait Bar { type Owned; … }`):
        //     adapter primary is `template<typename T> struct BarAdapter;`
        //     and each impl's full specialisation supplies `using Owned = …;`
        //     alongside method overrides. Phase 3a (this commit) lets
        //     these proceed through adapter emission.
        let trait_has_real_generics = !trait_generic_idents.is_empty();
        let trait_assoc_type_names: Vec<String> = t
            .items
            .iter()
            .filter_map(|i| match i {
                syn::TraitItem::Type(t) => Some(t.ident.to_string()),
                _ => None,
            })
            .collect();
        trait_generic_idents.extend(trait_assoc_type_names.iter().cloned());
        // Only mark as "with generics" if the trait has REAL type params
        // (Phase 3a: assoc-types-only traits proceed through adapter emit).
        if trait_has_real_generics {
            self.interface_traits_with_generics
                .insert(trait_name_str.clone());
        }
        // Remember the assoc-type names for adapter emission. Adapter
        // emit will fetch this when processing each impl block and
        // emit `using AssocName = ResolvedType;` typedefs.
        if !trait_assoc_type_names.is_empty() {
            self.trait_associated_type_names
                .insert(trait_name_str.clone(), trait_assoc_type_names.clone());
        }
        let trait_template_prefix = if trait_generic_idents.is_empty() {
            String::new()
        } else {
            format!(
                "template <{}>\n",
                trait_generic_idents
                    .iter()
                    .map(|g| format!("class {}", g))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let trait_generic_arglist = if trait_generic_idents.is_empty() {
            String::new()
        } else {
            format!("<{}>", trait_generic_idents.join(", "))
        };

        // Push the trait's generic params into scope so map_type emits
        // them as bare identifiers (not "auto" or template-arg-recovery).
        self.push_type_param_scope(&t.generics);

        // Push an associated-type scope so `Self::Item` resolves to the
        // bare template parameter name. The resolver
        // (resolve_current_struct_assoc_cpp_type) consults
        // current_struct_assoc_cpp_types to translate `Self::Item`.
        let mut assoc_scope: HashMap<String, String> = HashMap::new();
        for name in &trait_assoc_type_names {
            assoc_scope.insert(name.clone(), name.clone());
        }
        self.current_struct_assoc_cpp_types.push(assoc_scope);
        // Also set current_struct so the resolver finds the scope above
        // (it walks up from the current struct context).
        let prev_struct = self.current_struct.clone();
        self.current_struct = Some(trait_name_str.clone());

        // Collect supertraits — they become public bases of the interface.
        // Filter out:
        //   - Marker traits (Send/Sync/Sized/Copy/Clone/Unpin): lower to
        //     concepts, not classes.
        //   - Operator traits (PartialEq, BitAnd, etc.): no class form.
        //   - Skipped traits (had assoc consts, etc.): no class to inherit.
        //   - Generic traits we flagged: needs trait template arguments
        //     we don't yet emit on the inheritance line.
        //   - Foreign traits (not in trait_declared_paths): we don't have
        //     a C++ class spelling for them. Inheriting from a missing
        //     class is a hard compile error; dropping the constraint
        //     loses some semantic info but lets the rest compile.
        //
        // For locally-declared traits, qualify with the namespace recorded
        // in trait_declared_paths so `class X : public Outer::Sub` works
        // even when X is in a different namespace than Sub.
        let supertraits: Vec<String> = t
            .supertraits
            .iter()
            .filter_map(|b| {
                if let syn::TypeParamBound::Trait(tb) = b {
                    let name = tb.path.segments.last()?.ident.to_string();
                    if matches!(
                        name.as_str(),
                        "Send" | "Sync" | "Copy" | "Clone" | "Sized" | "Unpin"
                    ) {
                        return None;
                    }
                    if map_operator_trait(&name).is_some() {
                        return None;
                    }
                    if self.skipped_interface_traits.contains(&name) {
                        return None;
                    }
                    if self.interface_traits_with_generics.contains(&name) {
                        return None;
                    }
                    // Also skip when the supertrait has associated types
                    // (which become C++ template params via the assoc-type
                    // machinery). Without propagating matching params from
                    // the child trait through to the parent, the emitted
                    // inheritance `class Child : public Parent` rejects
                    // with "expected class name" — Parent is a template
                    // and can't appear without args. The Phase 3a comment
                    // upstream says assoc-types-only traits "proceed
                    // through adapter emit"; that's the right path for
                    // the impl side, but the inheritance side has no
                    // analogous machinery yet. Skip so the child trait
                    // emits at least as a standalone class. Surfaced by
                    // itertools' `trait HomogeneousTuple: TupleCollect`
                    // (itertools.cppm:10969).
                    if self
                        .trait_associated_type_names
                        .get(&name)
                        .is_some_and(|v| !v.is_empty())
                    {
                        return None;
                    }
                    // Find the qualified path for this trait among
                    // declared paths via the short-name index (built when
                    // each trait was declared).
                    let Some(qualified) = self
                        .trait_declared_path_by_short_name
                        .get(&name)
                        .cloned()
                    else {
                        // Foreign supertrait — skip.
                        return None;
                    };
                    // Escape each segment so C++ keywords used as Rust
                    // module names (e.g. `private::Sealed`) become
                    // `private_::Sealed` to match the namespace declaration.
                    let escaped = self.escape_and_rename_qualified_name(&qualified);
                    // Use leading `::` to make the path absolute,
                    // independent of current emission scope.
                    if escaped.contains("::") {
                        Some(format!("::{}", escaped))
                    } else {
                        Some(escaped)
                    }
                } else {
                    None
                }
            })
            .collect();

        let bases = if supertraits.is_empty() {
            String::new()
        } else {
            let supers = supertraits
                .iter()
                .map(|s| format!("public {}", s))
                .collect::<Vec<_>>()
                .join(", ");
            format!(" : {}", supers)
        };

        // Wrap the trait class in an anonymous namespace ONLY when the
        // trait is non-`pub`. The anon-ns gives module-internal linkage,
        // which is what Rust's default (`Visibility::Inherited`) means.
        // For `pub trait T`, the author has explicitly opted into
        // cross-module visibility — emit at namespace scope so other
        // TUs that import this module can name `T`.
        //
        // The original anon-ns wrap was added to handle a name-collision
        // edge case (two unrelated crates each declaring `SeqAccess`).
        // That edge case still occurs only for non-`pub` traits, since
        // a `pub` collision is the author's responsibility — the
        // transpiler can't paper over it with internal linkage anyway.
        // Adapter classes are emitted outside the anonymous namespace
        // either way, and find the trait class via name lookup within
        // the same TU when it's anon-wrapped.
        if wrap_in_anon_ns {
            self.writeln("namespace {");
        }
        // Open the abstract base class.
        if !trait_template_prefix.is_empty() {
            // Strip the trailing newline since writeln adds its own.
            self.writeln(&trait_template_prefix.trim_end());
        }
        self.writeln(&format!("class {}{} {{", trait_name, bases));
        self.writeln("public:");
        self.indent += 1;
        // Use noexcept(false) so derived Adapter classes whose value_
        // member's destructor is potentially-throwing (e.g.,
        // `rusty::Vec<T>::~Vec() noexcept(false)`) don't trigger
        // "exception specification of overriding function is more lax
        // than base version" when their implicit destructor inherits
        // the noexcept(false) status from a member. Use an explicit body
        // (`{}`) instead of `= default` because some clang versions
        // refuse `noexcept(false) = default` on a defaulted virtual dtor
        // ("exception specification is not available until end of class
        // definition") when the class inherits from another local trait.
        self.writeln(&format!("virtual ~{}() noexcept(false) {{}}", trait_name));

        // Emit one pure-virtual per trait method.
        for item in &t.items {
            let syn::TraitItem::Fn(method) = item else {
                continue;
            };

            let receiver = match method.sig.inputs.first() {
                Some(syn::FnArg::Receiver(r)) => Some(r),
                _ => None,
            };

            // No-receiver (static) trait methods are not lowered as virtuals.
            // Defer to a later phase.
            let Some(receiver) = receiver else {
                let m_name = escape_cpp_keyword(&method.sig.ident.to_string());
                self.writeln(&format!(
                    "// TODO(interface_traits): static trait method `{}` not yet supported",
                    m_name
                ));
                continue;
            };

            // By-value `self` (consuming receivers) need a different lowering;
            // skip in Phase 1.
            if receiver.reference.is_none() {
                let m_name = escape_cpp_keyword(&method.sig.ident.to_string());
                self.writeln(&format!(
                    "// TODO(interface_traits): by-value `self` method `{}` not yet supported",
                    m_name
                ));
                // Track so Adapter emit later also skips this method.
                // Without tracking, an impl on `&T` / `&mut T` Self
                // would rewrite `self` → `&self`/`&mut self` via
                // `normalize_impl_method_receiver_for_reference_self`,
                // and the Adapter spec would emit `override` for a base
                // method the trait class never declared. Surfaced by
                // serde_core's `impl Serializer for &mut fmt::Formatter`.
                let raw_method_name = method.sig.ident.to_string();
                self.trait_class_skipped_method_keys
                    .insert((trait_name_str.clone(), raw_method_name));
                continue;
            }

            // Method-level generics (e.g., `fn write_hex<W: Write>`) are
            // not yet supported. Skip the method declaration entirely
            // rather than emit `virtual R m(W w) const = 0;` with `W`
            // unbound in the class scope.
            if !method.sig.generics.params.is_empty() {
                let m_name = escape_cpp_keyword(&method.sig.ident.to_string());
                self.writeln(&format!(
                    "// TODO(interface_traits): generic method `{}` not yet supported",
                    m_name
                ));
                continue;
            }

            let is_const = receiver.mutability.is_none();

            let method_name = escape_cpp_keyword(&method.sig.ident.to_string());
            let return_type = self.map_return_type(&method.sig.output);

            let params: Vec<String> = method
                .sig
                .inputs
                .iter()
                .filter_map(|arg| match arg {
                    syn::FnArg::Receiver(_) => None,
                    syn::FnArg::Typed(pt) => {
                        let ty = self.map_type(&pt.ty);
                        let name = match pt.pat.as_ref() {
                            syn::Pat::Ident(pi) => escape_cpp_keyword(&pi.ident.to_string()),
                            _ => "_".to_string(),
                        };
                        Some(format!("{} {}", ty, name))
                    }
                })
                .collect();

            // C++ virtual methods can't be templates. Skip a method when
            // its signature can't be expressed as a non-template virtual:
            //   - `auto` placeholder (abbreviated template syntax —
            //     illegal as virtual).
            //   - `Self::` qualifier left in the type (Self is the
            //     implementing type, unknown at trait class scope; only
            //     resolved if the trait's assoc-type scope provided a
            //     concrete name).
            //   - bare `Self` reference (same problem, by-value Self).
            let signature_strings: Vec<&str> = std::iter::once(return_type.as_str())
                .chain(params.iter().map(|p| p.as_str()))
                .collect();
            // Also catch `TraitName::AssocType` references in the
            // signature: when the resolver couldn't fully resolve
            // `Self::Item` for a trait that lacks `type Item;`
            // declaration in its own scope (typically inherited from a
            // foreign supertrait like `Iterator`), it leaves
            // `Itertools::Item` unresolved against the trait class.
            let trait_name_qualifier = format!("{}::", trait_name);
            let unresolved = signature_strings.iter().any(|s| {
                s.split_whitespace().any(|w| w == "auto" || w == "Self")
                    || s.contains("<auto>")
                    || s.contains("<auto,")
                    || s.contains(" auto>")
                    || s.contains("Self::")
                    || s.contains(&trait_name_qualifier)
            });
            if unresolved {
                self.writeln(&format!(
                    "// TODO(interface_traits): method `{}` uses unresolved Self/auto in signature, not yet supported",
                    method_name
                ));
                continue;
            }

            let const_suffix = if is_const { " const" } else { "" };
            // Default method bodies frequently call other trait methods on
            // `self` (`this->next()`, `this->all(...)`, etc.). When those
            // methods belong to a foreign or skipped supertrait (e.g.,
            // Iterator's `next` / `all` referenced from itertools'
            // `Itertools::all_equal`), those member calls don't resolve
            // on the trait class itself and the build fails ("no member
            // named 'next' in 'Itertools'"). The default-emit-with-body
            // path below is guarded so we only inline trivial bodies whose
            // `self.<m>()` calls reference methods declared on THIS trait;
            // every other shape stays pure virtual and defers to the
            // Adapter (which sees the inherent impl-side method).
            let inline_body = method
                .default
                .as_ref()
                .and_then(|body| {
                    self.maybe_inline_trait_default_method_body(body, t, &method.sig.ident)
                });
            if let Some(body_expr) = inline_body {
                self.writeln(&format!(
                    "virtual {} {}({}){} {{ return {}; }}",
                    return_type,
                    method_name,
                    params.join(", "),
                    const_suffix,
                    body_expr
                ));
            } else {
                self.writeln(&format!(
                    "virtual {} {}({}){} = 0;",
                    return_type,
                    method_name,
                    params.join(", "),
                    const_suffix
                ));
            }
        }

        // dyn objects are unsized in Rust and must not be stored by value in
        // C++ either. Force all access through references / smart pointers.
        // Inside a class template the bare name `T` refers to the current
        // instantiation, so no generic-arg suffix is needed on the
        // copy/move special-member declarations.
        self.writeln(&format!("{}(const {}&) = delete;", trait_name, trait_name));
        self.writeln(&format!(
            "{}& operator=(const {}&) = delete;",
            trait_name, trait_name
        ));
        self.writeln(&format!("{}({}&&) = delete;", trait_name, trait_name));
        self.writeln(&format!(
            "{}& operator=({}&&) = delete;",
            trait_name, trait_name
        ));

        self.indent -= 1;
        self.writeln("protected:");
        self.indent += 1;
        self.writeln(&format!("{}() = default;", trait_name));
        self.indent -= 1;
        self.writeln("};");
        // Close the anonymous namespace if we opened one above.
        if wrap_in_anon_ns {
            self.writeln("}");
        }

        // Adapter primary templates — left undefined; specializations are
        // emitted per `impl T for U` in a later phase.
        // Three flavors:
        //   TraitAdapter<U>       — owns U by value; used by Box<dyn T> / Rc<dyn T> / Arc<dyn T>
        //   TraitAdapterRef<U>    — borrows const U&; used to materialize &dyn T from a concrete U
        //   TraitAdapterRefMut<U> — borrows U&; used to materialize &mut dyn T
        //
        // For a generic trait `trait Foo<T>`, the Adapter primary templates
        // need BOTH the trait's generic params AND the impl-type param U:
        //   template <class T, class U> class FooAdapter;
        // The Adapter specialization for `impl Foo<i32> for IntBag` then
        // becomes `template<> class FooAdapter<int32_t, IntBag> ...`.
        // Pick a non-conflicting placeholder name for the implementing
        // type. Default is `U`, but if the trait already declares a
        // generic named `U` we suffix-bump it to avoid duplicate
        // template parameter names (a hard compile error).
        let impl_param_name = {
            let mut candidate = "U".to_string();
            while trait_generic_idents.iter().any(|g| g == &candidate) {
                candidate.push('_');
            }
            candidate
        };
        let adapter_template_args = if trait_generic_idents.is_empty() {
            format!("class {}", impl_param_name)
        } else {
            format!(
                "{}, class {}",
                trait_generic_idents
                    .iter()
                    .map(|g| format!("class {}", g))
                    .collect::<Vec<_>>()
                    .join(", "),
                impl_param_name
            )
        };
        self.newline();
        self.writeln(&format!(
            "template <{}> class {}Adapter;",
            adapter_template_args, trait_name
        ));
        self.writeln(&format!(
            "template <{}> class {}AdapterRef;",
            adapter_template_args, trait_name
        ));
        self.writeln(&format!(
            "template <{}> class {}AdapterRefMut;",
            adapter_template_args, trait_name
        ));

        // Phase 3b.1: helper traits class forward decl. For each trait
        // with associated types, emit `template <class B> struct
        // <Trait>Traits;` — a single-arg helper indexed by the impl
        // type. Each `impl Trait for U { type AssocName = X; ... }`
        // emits a full specialization that binds the assoc names to
        // their concrete C++ types. Code that references
        // `typename B::AssocName` where B is a type-param bound by
        // Trait can then route through `typename <Trait>Traits<B>::AssocName`,
        // which works even when B is foreign (no nested typedef).
        // Phase 3b.2 wires the rewrite at type-emit sites.
        if !trait_assoc_type_names.is_empty() {
            // Primary template: default each associated type to the nested
            // typedef on the impl type B (`typename B::Assoc`). A concrete impl
            // that materializes its assoc types as nested members — e.g.
            // serde_test's `struct Serializer { using Ok = ...; using Error = ...;
            // ... }` — then resolves `<Trait>Traits<B>` through this primary
            // WITHOUT needing a full specialization. That matters cross-crate:
            // the impl can live in a downstream crate (serde_test) that doesn't
            // know this (foreign) trait's assoc-name list, so it can't emit the
            // full spec, yet a generic `S: Trait` lowered in THIS crate still
            // needs `<Trait>Traits<that impl type>::Ok` to resolve. The `S*`/`S&`
            // partial specs below and any explicit full spec remain more
            // specialized and still override this primary. (Previously this was
            // an undefined forward-decl, so any unspecced `<Trait>Traits<B>` was
            // a hard error — currently-passing code therefore never reached it,
            // making this strictly additive.)
            let mut primary = format!("template <class B> struct {}Traits {{ ", trait_name);
            for name in &trait_assoc_type_names {
                primary.push_str(&format!("using {0} = typename B::{0}; ", name));
            }
            primary.push_str("};");
            self.writeln(&primary);
            // STEP B (task #39): reference/pointer forwarding partial specs —
            // the type-level analog of serde's blanket `impl Tr for &mut S`
            // (associated types forward to the pointee). When a generic
            // `S: Tr` is instantiated with a `&mut`-derived C++ pointer `U*`
            // (or reference `U&`), `<Tr>Traits<S>` must still resolve, so
            // forward each assoc type to `<Tr>Traits<U>`. Inert unless something
            // instantiates `<Tr>Traits<T*>`/`<T&>` (only the STEP-A routing of a
            // pointer/reference-bound param does).
            for ptr_or_ref in ["S*", "S&"] {
                let mut spec = format!(
                    "template <class S> struct {}Traits<{}> {{ ",
                    trait_name, ptr_or_ref
                );
                for name in &trait_assoc_type_names {
                    spec.push_str(&format!(
                        "using {0} = typename {1}Traits<S>::{0}; ",
                        name, trait_name
                    ));
                }
                spec.push_str("};");
                self.writeln(&spec);
            }
        }

        self.pop_type_param_scope();
        self.current_struct_assoc_cpp_types.pop();
        self.current_struct = prev_struct;

        // Register the trait class so generic-arg recovery (e.g., the
        // Box<dyn Trait> coercion in emit_call_func_with_owner_template_recovery)
        // recognizes `TraitName` as a real C++ type rather than treating
        // it as a value-identifier placeholder. The bare name (without
        // generic-arg suffix) is what downstream lookups consult.
        self.local_declared_types.insert(trait_name_str);
        // Remember the trait's generic arity so dyn type mapping and
        // adapter-spec emission can recover the right template form.
        // (For now this is implicit via t.generics; we don't store it
        // in a separate map yet — see follow-up phases.)
        let _ = trait_generic_arglist;
    }

    pub(super) fn emit_module_mode_trait_runtime_helper(&mut self, t: &syn::ItemTrait) -> bool {
        if !(self.module_name.is_some() || self.expanded_libtest_mode) {
            return false;
        }
        if !t.generics.params.is_empty() {
            return false;
        }

        let default_methods: Vec<&syn::TraitItemFn> = t
            .items
            .iter()
            .filter_map(|item| {
                if let syn::TraitItem::Fn(method) = item {
                    method.default.as_ref().map(|_| method)
                } else {
                    None
                }
            })
            .collect();
        if default_methods.is_empty() {
            return false;
        }
        if !default_methods
            .iter()
            .any(|method| matches!(method.sig.inputs.first(), Some(syn::FnArg::Receiver(_))))
        {
            return false;
        }
        let self_assoc_type_placeholders: Vec<String> = t
            .items
            .iter()
            .filter_map(|item| {
                if let syn::TraitItem::Type(assoc) = item {
                    Some(format!("typename Self_::{}", assoc.ident))
                } else {
                    None
                }
            })
            .collect();
        let trait_name = t.ident.to_string();
        let helper_struct_name = format!("{}RuntimeHelper", escape_cpp_keyword(&trait_name));
        let mut helper_method_names = HashSet::new();
        for method in &default_methods {
            let raw_name = method.sig.ident.to_string();
            helper_method_names.insert(raw_name.clone());
            helper_method_names.insert(escape_cpp_keyword(&raw_name));
            helper_method_names.insert(Self::escape_cpp_method_name(&raw_name));
        }
        self.module_runtime_helper_trait_type_names
            .insert(trait_name.clone(), helper_struct_name.clone());
        self.module_runtime_helper_trait_type_names.insert(
            self.scoped_type_key(&trait_name),
            helper_struct_name.clone(),
        );
        for key in [
            trait_name.clone(),
            self.scoped_type_key(&trait_name),
            helper_struct_name.clone(),
            self.scoped_type_key(&helper_struct_name),
        ] {
            self.module_runtime_helper_trait_methods
                .insert(key, helper_method_names.clone());
        }
        self.module_runtime_helper_traits
            .insert(helper_struct_name.clone());
        self.module_runtime_helper_traits
            .insert(self.scoped_type_key(&helper_struct_name));

        self.writeln(&format!(
            "// Module-mode trait fallback for default methods on {}",
            t.ident
        ));
        self.writeln(&format!("struct {} {{", helper_struct_name));
        self.indent += 1;

        let mut emitted_any = false;
        for method in default_methods {
            let Some(default_body) = &method.default else {
                continue;
            };
            let Some(syn::FnArg::Receiver(receiver)) = method.sig.inputs.first() else {
                self.writeln(&format!(
                    "// Rust-only trait default method skipped (no receiver): {}",
                    method.sig.ident
                ));
                continue;
            };

            let receiver_param = if receiver.reference.is_some() {
                if receiver.mutability.is_some() {
                    "auto& self_".to_string()
                } else {
                    "const auto& self_".to_string()
                }
            } else {
                "auto self_".to_string()
            };

            let mut params = vec![receiver_param];
            for (idx, arg) in method.sig.inputs.iter().enumerate().skip(1) {
                let syn::FnArg::Typed(pat_type) = arg else {
                    continue;
                };
                let name = match pat_type.pat.as_ref() {
                    syn::Pat::Ident(pi) => escape_cpp_keyword(&pi.ident.to_string()),
                    _ => format!("_arg{}", idx),
                };
                params.push(format!("auto {}", name));
            }

            // Map return type with a temporary `Self_` context so associated
            // projections (e.g. `Self::Item`) lower to `typename Self_::Item`.
            let mapped_return_type = {
                let prev_struct = self.current_struct.clone();
                self.current_struct = Some("Self_".to_string());
                let mut self_scope = HashSet::new();
                self_scope.insert("Self_".to_string());
                self.type_param_scopes.push(self_scope);
                self.callable_type_param_return_scopes.push(HashMap::new());
                let prev_self_declared_params = if self_assoc_type_placeholders.is_empty() {
                    None
                } else {
                    Some(
                        self.declared_type_params
                            .insert("Self_".to_string(), self_assoc_type_placeholders.clone()),
                    )
                };
                let mapped = self.map_return_type(&method.sig.output);
                if let Some(prev) = prev_self_declared_params {
                    if let Some(prev_params) = prev {
                        self.declared_type_params
                            .insert("Self_".to_string(), prev_params);
                    } else {
                        self.declared_type_params.remove("Self_");
                    }
                }
                self.type_param_scopes.pop();
                self.callable_type_param_return_scopes.pop();
                self.current_struct = prev_struct;
                mapped
            };
            // `Self_` is local to the function body, so use a trailing return
            // type that references the receiver expression directly.
            let signature_return_type =
                mapped_return_type.replace("Self_", "std::remove_cvref_t<decltype(self_)>");
            self.writeln(&format!(
                "static auto {}({}) -> {} {{",
                escape_cpp_keyword(&method.sig.ident.to_string()),
                params.join(", "),
                signature_return_type
            ));
            self.indent += 1;
            self.writeln("using Self_ = std::remove_cvref_t<decltype(self_)>;");

            let prev_struct = self.current_struct.clone();
            self.current_struct = Some("Self_".to_string());
            let mut self_scope = HashSet::new();
            self_scope.insert("Self_".to_string());
            self.type_param_scopes.push(self_scope);
            self.callable_type_param_return_scopes.push(HashMap::new());
            let prev_self_declared_params = if self_assoc_type_placeholders.is_empty() {
                None
            } else {
                Some(
                    self.declared_type_params
                        .insert("Self_".to_string(), self_assoc_type_placeholders.clone()),
                )
            };
            let return_type = mapped_return_type;
            self.push_return_value_scope(&return_type);
            self.push_return_type_hint(&method.sig.output);
            self.push_param_bindings(&method.sig.inputs);
            self.push_self_receiver_ref_scope(&method.sig.inputs);
            self.push_self_path_override(Some("self_".to_string()));
            self.emit_block(default_body);
            self.pop_self_path_override();
            self.pop_self_receiver_ref_scope();
            self.pop_param_bindings();
            self.pop_return_type_hint();
            self.pop_return_value_scope();
            if let Some(prev) = prev_self_declared_params {
                if let Some(prev_params) = prev {
                    self.declared_type_params
                        .insert("Self_".to_string(), prev_params);
                } else {
                    self.declared_type_params.remove("Self_");
                }
            }
            self.type_param_scopes.pop();
            self.callable_type_param_return_scopes.pop();
            self.current_struct = prev_struct;

            self.indent -= 1;
            self.writeln("}");
            emitted_any = true;
        }

        self.indent -= 1;
        self.writeln("};");
        let trait_cpp_name = escape_cpp_keyword(&trait_name);
        // The `using {trait_name} = {helper_struct_name};` alias is here so
        // call sites like `Trait::method(self, ...)` can resolve to the
        // helper's static dispatch when the trait itself wasn't emitted as
        // a class (e.g. traits with associated constants are skipped via the
        // `skipped_interface_traits` path — arrayvec's `ArrayVecImpl` is the
        // canonical case). When the trait class IS emitted by
        // `emit_trait_interface_pattern`, the alias collides with it
        // (typedef-redefinition error — surfaced by either's `IntoEither`
        // trait). Only emit the alias when the class wasn't already emitted.
        // Skip the `using {trait_name} = {helper_struct_name};` alias
        // unconditionally now. The alias was added so call sites like
        // `Trait::method(self, ...)` resolve to the helper's static
        // dispatch when the trait itself wasn't emitted as a class. But
        // when the trait IS emitted as a class (the common case — both
        // the marker-trait fallback at line ~2835 AND the regular
        // interface emission below all produce a class with the trait's
        // name), the alias collides ("typedef redefinition with
        // different types"). The collision was surfaced by either's
        // `IntoEither` trait and would block any trait with default
        // methods whose class survives emission.
        //
        // The original callers that *needed* the alias (e.g. arrayvec's
        // `ArrayVecImpl` — has associated constants, so its class is
        // suppressed and only the helper is emitted) now do their own
        // alias emission at the call site (see the assoc-const branch
        // at line ~2748, which now writes the alias directly).
        let _ = (trait_cpp_name, helper_struct_name);
        emitted_any
    }

    /// Emit `TraitAdapter<U>` / `TraitAdapterRef<U>` / `TraitAdapterRefMut<U>`
    /// specializations for each impl block (one trio per implementing
    /// type `U`). Each override delegates to the corresponding
    /// `rusty_ext::method_name(value_, args...)` free function emitted
    /// earlier in this scope.
    pub(super) fn emit_trait_adapter_specializations(
        &mut self,
        trait_name: &str,
        methods: &[ExtensionImplMethod],
    ) {
        // Generic traits need partial specializations whose template
        // headers and base-class arglists we don't yet emit. Skip with a
        // TODO marker rather than emit broken specs that fail to compile.
        if self.interface_traits_with_generics.contains(trait_name) {
            self.writeln(&format!(
                "// TODO(interface_traits): {} is generic — Adapter specializations require partial-spec template headers, not yet emitted",
                trait_name
            ));
            return;
        }
        // Group methods by implementing self type.
        let mut by_self: Vec<(String, Vec<&ExtensionImplMethod>)> = Vec::new();
        for m in methods {
            let self_cpp = self.map_type(&m.self_ty);
            if let Some(group) = by_self.iter_mut().find(|(k, _)| k == &self_cpp) {
                group.1.push(m);
            } else {
                by_self.push((self_cpp, vec![m]));
            }
        }

        for (self_cpp, group) in &by_self {
            // Skip when self type maps to a placeholder we don't recognize.
            if self_cpp.contains("/* TODO") || type_string_has_auto_placeholder(self_cpp) {
                self.writeln(&format!(
                    "// TODO(interface_traits): skipped {}Adapter<{}> — unresolved self type",
                    trait_name, self_cpp
                ));
                continue;
            }
            // Skip generic impls (self type contains free type parameters
            // like `Option<T>`, `RangeInclusive<Idx>`). Full + partial
            // specializations of the Adapter primary template require
            // `template <T>` headers and inheritance/storage forms not
            // yet implemented. Detection: the impl had any type params
            // OR the self type's textual rendering still references one
            // of those param idents.
            if let Some(first_method) = group.first() {
                let has_impl_generics = !first_method.impl_generic_names.is_empty();
                let mentions_impl_generic = first_method
                    .impl_generic_names
                    .iter()
                    .any(|name| self_cpp.contains(name));
                if has_impl_generics && mentions_impl_generic {
                    self.writeln(&format!(
                        "// TODO(interface_traits): skipped generic impl `{}Adapter<{}>`",
                        trait_name, self_cpp
                    ));
                    continue;
                }
                if self.type_contains_unbound_single_letter_generic(&first_method.self_ty) {
                    continue;
                }
            }
            // Dedup: skip if we've already emitted Adapter trio for this
            // (trait, self) pair. Foreign-impl pipelines may iterate the
            // same impl twice when the trait has methods collected under
            // multiple module scopes; without this dedup the C++ build
            // sees a redefinition error.
            let dedup_key = (trait_name.to_string(), self_cpp.clone());
            if !self.emitted_foreign_adapter_specs.insert(dedup_key) {
                continue;
            }
            let methods_only: Vec<&syn::ImplItemFn> =
                group.iter().map(|m| &m.method).collect();
            // Phase 3a step 3: derive `trait_args` from the impl block's
            // `type Owned = X;` bindings. The trait's assoc-type names
            // were captured at trait-emit time and surface here via
            // `trait_associated_type_names`; we resolve each one against
            // this impl's bindings to get the concrete C++ types. The
            // local-impl pipeline does the analogous work in
            // collect_local_trait_impls_for_adapter (lines 23425-23462).
            //
            // All methods in a group come from the same impl block
            // (Rust forbids two `impl Trait for U` blocks), so reading
            // the first method's bindings is safe.
            let assoc_bindings: HashMap<String, syn::Type> = group
                .first()
                .map(|m| m.associated_type_bindings.clone())
                .unwrap_or_default();
            let bindings_cpp = self.extension_assoc_cpp_bindings(&assoc_bindings);
            let trait_args: Vec<String> = self
                .trait_associated_type_names
                .get(trait_name)
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(|assoc_name| {
                    bindings_cpp
                        .get(assoc_name)
                        .or_else(|| bindings_cpp.get(&escape_cpp_keyword(assoc_name)))
                        .cloned()
                })
                .collect();
            self.emit_one_foreign_adapter(
                trait_name,
                &trait_args,
                "Adapter",
                self_cpp,
                AdapterStorageKind::Owning,
                &methods_only,
            );
            self.emit_one_foreign_adapter(
                trait_name,
                &trait_args,
                "AdapterRef",
                self_cpp,
                AdapterStorageKind::ConstRef,
                &methods_only,
            );
            self.emit_one_foreign_adapter(
                trait_name,
                &trait_args,
                "AdapterRefMut",
                self_cpp,
                AdapterStorageKind::MutRef,
                &methods_only,
            );
            // Phase 3b.1: helper traits class spec for this impl. Lets
            // downstream code resolve `<T as Trait>::AssocName` via
            // `typename <Trait>Traits<T>::AssocName` (Phase 3b.2 wires
            // that rewrite). Spec emits only if the trait has assoc
            // types AND this impl supplies bindings.
            let assoc_pairs: Vec<(String, String)> = self
                .trait_associated_type_names
                .get(trait_name)
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(|assoc_name| {
                    bindings_cpp
                        .get(assoc_name)
                        .or_else(|| bindings_cpp.get(&escape_cpp_keyword(assoc_name)))
                        .cloned()
                        .map(|ty| (assoc_name.clone(), ty))
                })
                .collect();
            self.emit_assoc_type_helper_spec(trait_name, self_cpp, &assoc_pairs);
        }
    }

    pub(super) fn emit_mod(&mut self, m: &syn::ItemMod) {
        // Skip #[cfg(test)] modules — test code is not transpiled into production output
        if Self::has_cfg_test(&m.attrs) {
            self.writeln("// #[cfg(test)] module omitted");
            return;
        }

        let mod_name = &m.ident;
        let mod_name_str = mod_name.to_string();
        // Check if this module was renamed due to function name collision
        let scope_prefix = self.module_stack.join("::");
        let qualified_mod = if scope_prefix.is_empty() {
            mod_name_str.clone()
        } else {
            format!("{}::{}", scope_prefix, mod_name_str)
        };
        let mod_cpp_name = if let Some(renamed) = self.module_namespace_renames.get(&qualified_mod)
        {
            renamed.clone()
        } else {
            escape_cpp_keyword(&mod_name_str)
        };
        let is_pub = matches!(m.vis, syn::Visibility::Public(_));
        let export_mod_namespace_prefix =
            if self.module_name.is_some() && is_pub && mod_name_str.starts_with("__private") {
                "export "
            } else {
                ""
            };
        let has_inline_content = m.content.is_some();

        if let Some(ref parent_module) = self.module_name.clone() {
            // In module mode:
            // - `mod foo;`   → import parent.foo;
            // - `mod foo {}` → inline namespace emission only (no import line)
            if !has_inline_content {
                let full_name = format!("{}.{}", parent_module, mod_name);
                if is_pub {
                    self.writeln(&format!("export import {};", full_name));
                } else {
                    self.writeln(&format!("import {};", full_name));
                }
            }
        } else {
            // Without module mode, just emit a comment
            self.writeln(&format!("// mod {}", mod_name));
        }

        // If the mod has inline content (mod foo { ... }), emit it
        if let Some((_, items)) = &m.content {
            if self.deferred_module_function_pass {
                // Deferred pass: only emit functions and nested modules
                self.writeln(&format!(
                    "{}namespace {} {{",
                    export_mod_namespace_prefix, mod_cpp_name
                ));
                self.indent += 1;
                self.module_stack.push(mod_name.to_string());
                let mut nested_mod_names: Vec<String> = items
                    .iter()
                    .filter_map(|item| match item {
                        syn::Item::Mod(nested) => Some(nested.ident.to_string()),
                        _ => None,
                    })
                    .collect();
                nested_mod_names.sort();
                nested_mod_names.dedup();
                for nested_name in nested_mod_names {
                    let qualified_nested = if self.module_stack.is_empty() {
                        nested_name.clone()
                    } else {
                        format!("{}::{}", self.module_stack.join("::"), nested_name)
                    };
                    let emitted_nested = self
                        .module_namespace_renames
                        .get(&qualified_nested)
                        .cloned()
                        .unwrap_or_else(|| escape_cpp_keyword(&nested_name));
                    self.writeln(&format!("namespace {} {{}}", emitted_nested));
                }
                if !items.is_empty() {
                    self.newline();
                }
                let scope_path = self.module_stack.clone();
                if self.emit_extension_trait_forward_decls_for_scope(&scope_path) {
                    self.newline();
                }
                let ordered_items = Self::prioritize_use_items_for_emission(
                    self.order_items_for_emission(items, true, false),
                );
                for item in ordered_items {
                    match item {
                        syn::Item::Fn(_) | syn::Item::Mod(_) => {
                            self.emit_item(item);
                            self.newline();
                        }
                        _ => {}
                    }
                }
                if self.emit_extension_trait_free_functions_for_scope(&scope_path) {
                    self.newline();
                }
                self.module_stack.pop();
                self.indent -= 1;
                self.writeln("}");
            } else {
                // Check if this module has both structs and functions
                let has_structs = items.iter().any(|i| matches!(i, syn::Item::Struct(_)));
                let has_fns = items.iter().any(|i| matches!(i, syn::Item::Fn(_)));
                let has_inline_mods = items
                    .iter()
                    .any(|i| matches!(i, syn::Item::Mod(m) if m.content.is_some()));
                let should_split =
                    has_fns && (has_structs || has_inline_mods) && self.module_stack.is_empty();
                let inherited_recursive_deferral = self.defer_module_functions_recursively;
                if should_split {
                    self.defer_module_functions_recursively = true;
                }
                let should_defer_functions_now =
                    inherited_recursive_deferral || self.defer_module_functions_recursively;

                self.writeln(&format!(
                    "{}namespace {} {{",
                    export_mod_namespace_prefix, mod_cpp_name
                ));
                self.indent += 1;
                self.module_stack.push(mod_name.to_string());
                let mut nested_mod_names: Vec<String> = items
                    .iter()
                    .filter_map(|item| match item {
                        syn::Item::Mod(nested) => Some(nested.ident.to_string()),
                        _ => None,
                    })
                    .collect();
                nested_mod_names.sort();
                nested_mod_names.dedup();
                for nested_name in nested_mod_names {
                    let qualified_nested = if self.module_stack.is_empty() {
                        nested_name.clone()
                    } else {
                        format!("{}::{}", self.module_stack.join("::"), nested_name)
                    };
                    let emitted_nested = self
                        .module_namespace_renames
                        .get(&qualified_nested)
                        .cloned()
                        .unwrap_or_else(|| escape_cpp_keyword(&nested_name));
                    self.writeln(&format!("namespace {} {{}}", emitted_nested));
                }
                if !items.is_empty() {
                    self.newline();
                }
                if self.emit_forward_decl_namespace_alias_imports(items) {
                    self.newline();
                }
                let scope_path = self.module_stack.clone();
                let prev_module_body_forward_decl_pass = self.module_body_forward_decl_pass;
                self.module_body_forward_decl_pass = true;
                let emitted_forward_decls =
                    self.emit_item_forward_decls(items, self.module_stack.len());
                self.module_body_forward_decl_pass = prev_module_body_forward_decl_pass;
                if emitted_forward_decls {
                    self.newline();
                }
                if self.emit_extension_trait_forward_decls_for_scope(&scope_path) {
                    self.newline();
                }
                let ordered_items = Self::prioritize_use_items_for_emission(
                    self.order_items_for_emission(items, true, false),
                );
                let mut pending_alias_impl_owner_defs: Vec<String> = Vec::new();
                for item in ordered_items {
                    if let syn::Item::Impl(i) = item {
                        if let Some(owner_key) = self.alias_impl_owner_key_from_impl(i)
                            && !pending_alias_impl_owner_defs
                                .iter()
                                .any(|k| k == &owner_key)
                        {
                            pending_alias_impl_owner_defs.push(owner_key);
                        }
                        continue;
                    }
                    if should_defer_functions_now && matches!(item, syn::Item::Fn(_)) {
                        // Skip functions — they'll be emitted in deferred pass
                        continue;
                    }
                    self.emit_item(item);
                    self.newline();
                }
                for owner_key in pending_alias_impl_owner_defs {
                    if self.emit_type_alias_impl_defs_for_owner(&owner_key) {
                        self.newline();
                    }
                }
                if !should_defer_functions_now
                    && self.emit_extension_trait_free_functions_for_scope(&scope_path)
                {
                    self.newline();
                }
                self.module_stack.pop();
                self.indent -= 1;
                self.writeln("}");

                if should_split {
                    self.deferred_module_items.push(m.clone());
                }
                self.defer_module_functions_recursively = inherited_recursive_deferral;
            }
        }
    }

    pub(super) fn emit_use(&mut self, u: &syn::ItemUse) {
        let is_pub = matches!(u.vis, syn::Visibility::Public(_));

        // Detect external crate imports
        let root_ident = self.get_use_root(&u.tree);
        let mapped_external_root = self.name_resolver.external_crate_target(&root_ident);
        let root_is_scope_import = self
            .resolve_scope_import_binding_path(&root_ident)
            .or_else(|| self.resolve_scope_import_binding_path_for_scope("", &root_ident))
            .or_else(|| self.resolve_unique_scope_import_binding_path_any_scope(&root_ident))
            .is_some();
        let is_external = !matches!(
            root_ident.as_str(),
            "crate" | "self" | "super" | "std" | "core" | "alloc" | "cpp"
        ) && root_ident.chars().next().is_some_and(|c| c.is_lowercase())
            && self.crate_name.as_deref() != Some(root_ident.as_str())
            && !self.declared_item_names.contains(&root_ident)
            && !self.declared_module_names.contains(&root_ident)
            && !self.import_alias_names.contains(&root_ident)
            && !root_is_scope_import
            && mapped_external_root.is_none();

        if is_external {
            self.writeln(&format!(
                "// TODO: external crate '{}' — provide type mapping or transpile dependency",
                root_ident
            ));
        }

        // Flatten group imports into separate using declarations
        let paths = self.flatten_use_tree(&u.tree, "");

        let export_prefix = if self.is_exported(&u.vis) {
            "export "
        } else {
            ""
        };
        for raw_path in &paths {
            let path = self.rewrite_external_crate_import_path(raw_path);
            if let Some(cpp_import) = classify_cpp_module_use_import(&path) {
                self.record_cpp_module_use_import(&cpp_import);
                if cpp_import.explicit_alias {
                    self.writeln(&format!(
                        "// C++ module import (reserved cpp::): {} as {}",
                        cpp_import.module_path, cpp_import.binding_name
                    ));
                } else {
                    self.writeln(&format!(
                        "// C++ module import (reserved cpp::): {}",
                        cpp_import.module_path
                    ));
                }
                continue;
            }
            let resolved_path = self.resolve_unqualified_local_import_path(&path);
            let resolved_path = self.strip_current_crate_prefix_from_import_path(&resolved_path);
            let resolved_path = self.resolve_nested_local_reexport_path(&resolved_path);
            // Own-crate imports are redundant in flat libtest targets (the
            // crate's items are visible via `import <crate>;`) and ill-formed
            // for macro-only names; skip them rather than emit a `using`
            // referencing a nonexistent `<crate>::` namespace.
            if self.import_path_root_is_current_crate(&resolved_path) {
                self.writeln(&format!("// Rust-only: using {};", resolved_path));
                continue;
            }
            let normalized_import = normalize_use_import_path(&resolved_path);
            if normalized_import.starts_with("std::os::")
                || normalized_import.starts_with("core::os::")
            {
                self.writeln(&format!("// Rust-only: using {};", resolved_path));
                continue;
            }
            if matches!(
                normalized_import.trim_start_matches("::"),
                "fmt::Write"
                    | "std::fmt::Write"
                    | "core::fmt::Write"
                    | "self::fmt::Write"
                    | "super::fmt::Write"
                    | "crate::fmt::Write"
                    | "rusty::fmt::Write"
                    | "de::DeserializeOwned"
                    | "serde::de::DeserializeOwned"
                    | "serde_core::de::DeserializeOwned"
            ) {
                self.writeln(&format!("// Rust-only: using {};", resolved_path));
                continue;
            }
            if let Some(mapped_crate_single) =
                self.resolve_crate_single_segment_type_import(&resolved_path)
            {
                self.writeln(&format!(
                    "{}using ::{};",
                    export_prefix, mapped_crate_single
                ));
                continue;
            }
            self.record_option_alias_import(&resolved_path);
            self.record_variant_constructor_alias_import(&resolved_path);
            if self.is_variant_constructor_alias_import(&resolved_path) {
                self.writeln(&format!(
                    "// Rust-only constructor alias import: using {};",
                    resolved_path
                ));
                continue;
            }
            if is_pub
                && self.module_name.is_some()
                && is_module_linkage_sensitive_reexport(normalize_use_import_path(&resolved_path))
            {
                self.writeln(&format!("// Rust-only: using {};", resolved_path));
                continue;
            }
            if self.is_skipped_module_trait_import(&resolved_path) {
                self.writeln(&format!("// Rust-only: using {};", resolved_path));
                continue;
            }
            if self.is_macro_rules_import(&resolved_path) {
                self.writeln(&format!(
                    "// Rust-only macro import: using {};",
                    resolved_path
                ));
                continue;
            }
            if self.should_skip_unresolved_bare_import(&resolved_path) {
                self.writeln(&format!(
                    "// Rust-only unresolved import: using {};",
                    resolved_path
                ));
                continue;
            }
            let use_action = classify_use_import(&resolved_path);
            if is_external {
                let allow_external_mapping = is_supported_external_import_mapping(&resolved_path);
                if !allow_external_mapping || matches!(use_action, UseImportAction::RustOnly) {
                    self.writeln(&format!(
                        "// Rust-only unresolved import: using {};",
                        resolved_path
                    ));
                    continue;
                }
            }
            if let Some(namespace_target) =
                self.resolve_bare_module_namespace_import(&resolved_path)
            {
                self.emit_namespace_alias_import(&namespace_target);
                continue;
            }
            match use_action {
                UseImportAction::RustOnly => {
                    // Check if this is an enum variant import that we can emit
                    // as constexpr auto. Two well-known `Ordering` namespaces
                    // in std have stable, finite variant sets that the rusty
                    // support library mirrors verbatim:
                    //
                    //   std::sync::atomic::Ordering  → rusty::sync::atomic::Ordering
                    //   std::cmp::Ordering            → rusty::cmp::Ordering
                    //
                    // Recognize `use Ordering::{Variant}` against both and
                    // emit a `constexpr auto Variant = rusty::...::Ordering::Variant;`
                    // alias so call sites that say `Equal` (etc.) resolve.
                    let parts: Vec<&str> = resolved_path.split("::").collect();
                    let emitted_as_enum_const = if parts.len() >= 2 {
                        let variant = *parts.last().unwrap_or(&"");
                        let parent = parts[parts.len() - 2];
                        let owner_root = parts.first().copied().unwrap_or("");
                        let is_atomic_ordering = parent == "Ordering"
                            && (owner_root == "atomic" || owner_root == "sync" || owner_root == "core" || owner_root == "std")
                            && matches!(
                                variant,
                                "SeqCst" | "Acquire" | "Release" | "AcqRel" | "Relaxed"
                            );
                        let is_cmp_ordering = parent == "Ordering"
                            && matches!(variant, "Less" | "Equal" | "Greater");
                        if is_atomic_ordering {
                            self.writeln(&format!(
                                "constexpr auto {} = rusty::sync::atomic::Ordering::{};",
                                variant, variant
                            ));
                            true
                        } else if is_cmp_ordering {
                            self.writeln(&format!(
                                "constexpr auto {} = rusty::cmp::Ordering::{};",
                                variant, variant
                            ));
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    if !emitted_as_enum_const {
                        self.writeln(&format!("// Rust-only: using {};", resolved_path));
                    }
                }
                UseImportAction::Using(mapped_path) => {
                    if let Some((alias, _)) =
                        split_use_import_alias(normalize_use_import_path(&mapped_path))
                        && alias.trim() == "_"
                    {
                        // Rust trait-import marker (`use Trait as _;`) is used for
                        // method resolution only and should not emit a C++ alias.
                        self.writeln(&format!(
                            "// Rust-only trait import marker: using {};",
                            resolved_path
                        ));
                        continue;
                    }
                    if let Some(template_alias_stmt) =
                        self.template_alias_import_statement(&mapped_path)
                    {
                        self.writeln(&format!("{}{}", export_prefix, template_alias_stmt));
                        continue;
                    }
                    let using_path = make_using_path_cpp_legal(&mapped_path);
                    if let Some(namespace_target) = using_path.strip_prefix("namespace ") {
                        let ns = namespace_target.trim();
                        if !ns.is_empty() {
                            // Apply module renames to namespace target
                            let renamed_ns = self.apply_module_renames_to_path(ns);
                            self.emit_namespace_using_import(&renamed_ns);
                        }
                        continue;
                    }
                    // Apply module namespace renames (e.g., sync → sync_mod)
                    // to using declaration paths.
                    let using_path = self.apply_module_renames_to_path(&using_path);
                    let using_path =
                        self.rewrite_global_using_path_for_local_alias_root(&using_path);
                    let using_path = self.rewrite_using_path_with_scope_import_root(&using_path);
                    let using_path = self
                        .try_resolve_nested_local_type_path(&using_path)
                        .unwrap_or(using_path);
                    let using_path = self.rewrite_using_content_reexport_path(&using_path);
                    let using_path = self.rewrite_using_path_with_scope_import_root(&using_path);
                    let using_path = Self::strip_crate_root_cpp_path(&using_path);
                    let using_path =
                        self.rewrite_global_using_path_for_private_alias_root(&using_path);
                    let using_path =
                        self.rewrite_using_path_for_module_runtime_helper_trait(&using_path);
                    // Collapse a renamed re-export of a Rust primitive
                    // (`pub use core::primitive::u8 as yaml_char_t;`) to the C++
                    // primitive (`using yaml_char_t = uint8_t;`). Otherwise the
                    // alias target stays the undefined `std::primitive::u8` — a
                    // typedef pattern c2rust-ported crates (unsafe-libyaml) use
                    // heavily for `yaml_char_t` / `size_t` / `ptrdiff_t`.
                    let using_path = if let Some((alias, target)) =
                        split_use_import_alias(&using_path)
                        && let Some(prim) = Self::map_qualified_primitive_alias_path(target)
                    {
                        format!("{} = {}", alias, prim)
                    } else {
                        using_path
                    };
                    if let Some((alias, _)) = split_use_import_alias(&using_path)
                        && alias.trim() == "_"
                    {
                        // Rust trait-import marker (`use Trait as _;`) is used for
                        // method resolution only and should not emit a C++ alias.
                        self.writeln(&format!(
                            "// Rust-only trait import marker: using {};",
                            resolved_path
                        ));
                        continue;
                    }
                    self.emit_private_alias_forward_decl_for_using_path(&using_path);
                    if self.should_skip_unresolved_single_segment_type_import(&using_path) {
                        self.writeln(&format!(
                            "// Rust-only unresolved import: using {};",
                            resolved_path
                        ));
                        continue;
                    }
                    if self.should_skip_unresolved_function_using_import(&using_path) {
                        self.writeln(&format!(
                            "// Rust-only unresolved function import (forward decl unavailable): using {};",
                            resolved_path
                        ));
                        continue;
                    }
                    if let Some(local_name) = Self::using_import_local_name(&using_path)
                        && self.should_force_qualified_import_binding_name(&local_name)
                    {
                        self.writeln(&format!(
                            "// Rust-only conflicting re-export skipped (trait/type name collision): using {};",
                            resolved_path
                        ));
                        continue;
                    }
                    if let Some(ns_alias_stmt) =
                        self.namespace_alias_statement_for_module_import(&using_path)
                    {
                        self.emit_namespace_alias_statement_once(&ns_alias_stmt, export_prefix);
                        continue;
                    }
                    // Own-crate items are already visible via `import <crate>;`
                    // in flat libtest targets. A private `using <crate>::item;`
                    // then references a nonexistent `<crate>` namespace — and is
                    // ill-formed for macro-only names (`iproduct!`, `izip!`).
                    // The scope-import rewrites above resolve crate aliases
                    // (`use itertools as it; use crate::it::chain;`) to this
                    // `<crate>::…` form, so test it here on the final path. Pub
                    // re-exports are intentionally left intact.
                    if !is_pub && self.import_path_root_is_current_crate(&using_path) {
                        self.writeln(&format!("// Rust-only: using {};", resolved_path));
                        continue;
                    }
                    let using_tail = using_path.trim_start_matches("::");
                    if !using_tail.is_empty()
                        && !using_tail.contains("::")
                        && !using_tail.contains(" = ")
                        && using_tail
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_lowercase())
                    {
                        self.writeln(&format!(
                            "// Rust-only namespace re-export: using {};",
                            resolved_path
                        ));
                        continue;
                    }
                    // Check if this is an enum variant import (e.g., Ordering::SeqCst).
                    // C++ enum class values can't be imported with `using`.
                    // Emit `constexpr auto SeqCst = ...::Ordering::SeqCst;` instead.
                    let using_segments: Vec<&str> = using_path.split("::").collect();
                    // The owning enum of an imported c-like variant. The parent
                    // segment is usually the enum (`Ordering::SeqCst`), but a
                    // c2rust-style re-export imports the variant via its MODULE
                    // (`use crate::yaml::YAML_ALIAS_EVENT`) — there the parent is
                    // `yaml`, so resolve the unique owning enum by variant name.
                    let enum_variant_owner: Option<String> = if using_segments.len() >= 2 {
                        let variant = *using_segments.last().unwrap_or(&"");
                        let parent = using_segments[using_segments.len() - 2];
                        let key = format!("{}_{}", parent, variant);
                        if self.c_like_enum_consts.contains(&key)
                            || (parent == "Ordering"
                                && matches!(
                                    variant,
                                    "SeqCst" | "Acquire" | "Release" | "AcqRel" | "Relaxed"
                                ))
                        {
                            Some(parent.to_string())
                        } else if using_segments
                            .first()
                            .is_some_and(|root| matches!(*root, "rusty" | "std" | "core" | "alloc"))
                        {
                            // A runtime/external TYPE import already mapped to a
                            // `rusty::`/std path (`use std::string::String` ->
                            // `rusty::String`) must NOT be hijacked as a local
                            // c-like-enum variant import just because the leaf
                            // (`String`) happens to name a variant of some local
                            // enum (e.g. serde's `private_::ser::Unsupported::String`).
                            // Genuine c2rust variant re-exports resolve to a LOCAL
                            // sibling-module path, never a runtime/std root.
                            None
                        } else {
                            self.unique_c_like_enum_owner_for_variant_name(variant)
                        }
                    } else {
                        None
                    };
                    if let Some(owner) = enum_variant_owner {
                        let variant_name = *using_segments.last().unwrap();
                        // C++20 `enum class` variants can't be `using`-imported;
                        // bind a constant to the scoped `Owner::VARIANT`. When the
                        // parent segment is the module (not the enum), inject the
                        // owning enum between the module path and the variant.
                        let variant_ref =
                            if using_segments[using_segments.len() - 2] == owner.as_str() {
                                using_path.clone()
                            } else {
                                let parent_path =
                                    using_segments[..using_segments.len() - 1].join("::");
                                format!("{}::{}::{}", parent_path, owner, variant_name)
                            };
                        self.writeln(&format!(
                            "{}constexpr auto {} = {};",
                            export_prefix, variant_name, variant_ref
                        ));
                    } else {
                        // Cross-module sibling detection.
                        //
                        // When the using path looks like `::seg::Item;`
                        // and `seg` matches a sibling C++ module in
                        // this crate (i.e. `<parent>.<seg>` is in
                        // `crate_module_names`), we have to emit a
                        // C++20 `import …;` for the sibling module
                        // instead of (or in addition to) the using —
                        // otherwise name lookup fails because `::seg`
                        // doesn't exist as a global namespace in
                        // module mode. After import, the symbol is
                        // accessible at the same name it was declared
                        // under in the sibling module, so the using
                        // statement itself is redundant and would
                        // refer to a non-existent global path; we
                        // drop it.
                        let mut emitted_as_module_import = false;
                        if let Some(first_segment) = using_segments
                            .iter()
                            .find(|seg| !seg.is_empty())
                            .copied()
                            && let Some(sibling_module) =
                                self.resolve_sibling_module_path(first_segment)
                            && self
                                .sibling_modules_imported
                                .insert(sibling_module.clone())
                        {
                            self.writeln(&format!("import {};", sibling_module));
                            emitted_as_module_import = true;
                        } else if let Some(first_segment) = using_segments
                            .iter()
                            .find(|seg| !seg.is_empty())
                            .copied()
                            && self.resolve_sibling_module_path(first_segment).is_some()
                        {
                            // Module was already imported earlier in
                            // this file by a previous use-statement;
                            // skip both the redundant import and the
                            // broken using path.
                            emitted_as_module_import = true;
                        }
                        if !emitted_as_module_import {
                            self.writeln(&format!("{}using {};", export_prefix, using_path));
                        }
                    }
                    let cow_target =
                        split_use_import_alias(normalize_use_import_path(&mapped_path))
                            .map(|(_, target)| target.trim())
                            .unwrap_or_else(|| normalize_use_import_path(&mapped_path).trim());
                    if cow_target == "rusty::Cow" {
                        self.writeln(&format!("{}using rusty::Cow_Borrowed;", export_prefix));
                        self.writeln(&format!("{}using rusty::Cow_Owned;", export_prefix));
                    }
                    if cow_target == "rusty::Either" {
                        self.writeln(&format!("{}using rusty::Either_Left;", export_prefix));
                        self.writeln(&format!("{}using rusty::Either_Right;", export_prefix));
                    }
                    // When importing a data enum type, also import its namespace
                    // so variant structs (EnumName_VariantName) are accessible.
                    // Derive from the final emitted `using_path` so local scope
                    // rewrites (e.g. `scalar::Type` inside `namespace decoder`)
                    // are preserved instead of forcing global `::scalar`.
                    let imported_name = mapped_path.rsplit("::").next().unwrap_or(&mapped_path);
                    if self.data_enum_types.contains(imported_name)
                        && let Some((ns, _)) = using_path.rsplit_once("::")
                    {
                        self.emit_namespace_using_import(ns);
                    }
                }
                UseImportAction::Raw(statement) => {
                    if let Some(alias_name) = parse_namespace_alias_name(&statement) {
                        if let Some(target) = parse_namespace_alias_target(&statement)
                            && self.should_skip_namespace_alias_statement(alias_name, target)
                        {
                            continue;
                        }
                        if self.current_scope_declares_nested_module_root(alias_name) {
                            self.writeln(&format!(
                                "// Rust-only namespace alias shadowed by nested module: {}",
                                statement
                            ));
                            continue;
                        }
                        if !self.record_namespace_alias_import_name(alias_name) {
                            continue;
                        }
                    }
                    self.writeln(&format!("{}{}", export_prefix, statement));
                }
            }
        }
    }

    pub(super) fn emit_type_alias_impl_free_function_decls_for_owner(&mut self, owner_key: &str) -> bool {
        if !self.type_key_is_declared_alias(owner_key) {
            return false;
        }
        if !self
            .emitted_alias_impl_owner_forward_decls
            .insert(owner_key.to_string())
        {
            return false;
        }
        self.emit_type_alias_impl_free_functions_for_owner(owner_key, true)
    }

    pub(super) fn emit_type_alias_impl_defs_for_owner(&mut self, owner_key: &str) -> bool {
        if !self
            .emitted_alias_impl_owner_defs
            .insert(owner_key.to_string())
        {
            return false;
        }
        self.emit_type_alias_impl_free_functions_for_owner(owner_key, false)
    }

    pub(super) fn emit_type_alias_impl_block(&mut self, i: &syn::ItemImpl) -> bool {
        let Some(owner_key) = self.alias_impl_owner_key_from_impl(i) else {
            return false;
        };
        self.emit_type_alias_impl_defs_for_owner(&owner_key)
    }

    pub(super) fn emit_type_alias_impl_free_functions_for_owner(
        &mut self,
        owner_key: &str,
        declaration_only: bool,
    ) -> bool {
        let owner_tail = owner_key.rsplit("::").next().unwrap_or(owner_key);
        let items = if declaration_only {
            self.impl_blocks
                .get(owner_key)
                .or_else(|| self.impl_blocks.get(owner_tail))
                .cloned()
        } else {
            self.take_impls_for_owner_key(owner_key).or_else(|| {
                if owner_tail != owner_key {
                    self.take_impls_for_type(owner_tail)
                } else {
                    None
                }
            })
        };
        let Some(items) = items else {
            return false;
        };

        let mut emitted_any = false;
        for item in items {
            let syn::ImplItem::Fn(method) = item else {
                continue;
            };
            let method_name = method.sig.ident.to_string();
            let has_alias_receiver_shape = self
                .lookup_alias_inherent_owner_method_has_receiver_for_owner_key(
                    owner_key,
                    &method_name,
                )
                .or_else(|| {
                    self.lookup_alias_inherent_owner_method_has_receiver_for_owner_key(
                        owner_tail,
                        &method_name,
                    )
                })
                .is_some();
            if !has_alias_receiver_shape {
                continue;
            }
            if self.emit_type_alias_impl_free_function(owner_key, &method, declaration_only) {
                emitted_any = true;
                if !declaration_only {
                    self.newline();
                }
            }
        }
        emitted_any
    }

    pub(super) fn emit_type_alias_impl_free_function(
        &mut self,
        owner_key: &str,
        method: &syn::ImplItemFn,
        declaration_only: bool,
    ) -> bool {
        let emission_key = Self::alias_impl_method_emission_key(owner_key, method);
        let inserted = if declaration_only {
            self.emitted_alias_impl_decl_keys.insert(emission_key)
        } else {
            self.emitted_alias_impl_def_keys.insert(emission_key)
        };
        if !inserted {
            return false;
        }

        let owner_tail = owner_key
            .rsplit("::")
            .next()
            .unwrap_or(owner_key)
            .to_string();
        let method_name = method.sig.ident.to_string();
        let helper_name = self.alias_impl_helper_function_name(owner_key, &method_name);

        self.emit_template_prefix(&method.sig.generics);
        self.push_type_param_scope(&method.sig.generics);

        let mut params = Vec::new();
        let has_receiver = matches!(method.sig.inputs.first(), Some(syn::FnArg::Receiver(_)));
        if let Some(syn::FnArg::Receiver(recv)) = method.sig.inputs.first() {
            let self_param = if recv.reference.is_some() {
                if recv.mutability.is_some() {
                    "auto& self_".to_string()
                } else {
                    "const auto& self_".to_string()
                }
            } else {
                "auto self_".to_string()
            };
            params.push(self_param);
        }
        for (idx, arg) in method.sig.inputs.iter().enumerate() {
            let syn::FnArg::Typed(pat_type) = arg else {
                continue;
            };
            let ty = self.resolve_param_cpp_type(&pat_type.ty);
            let param_name = match pat_type.pat.as_ref() {
                syn::Pat::Ident(pi) => escape_cpp_keyword(&pi.ident.to_string()),
                _ => format!("_arg{}", idx),
            };
            params.push(format!("{} {}", ty, param_name));
        }

        let mapped_return = self.map_impl_method_return_type(method);
        let return_type = if mapped_return.contains("/* TODO")
            || type_string_has_auto_placeholder(&mapped_return)
        {
            "auto".to_string()
        } else {
            mapped_return
        };

        if declaration_only {
            self.writeln(&format!(
                "{} {}({});",
                return_type,
                helper_name,
                params.join(", ")
            ));
            self.pop_type_param_scope();
            return true;
        }

        self.writeln(&format!(
            "inline {} {}({}) {{",
            return_type,
            helper_name,
            params.join(", ")
        ));
        self.indent += 1;

        let prev_struct = self.current_struct.clone();
        self.current_struct = Some(owner_tail);
        self.push_return_value_scope(&return_type);
        self.push_return_type_hint(&method.sig.output);
        self.push_param_bindings(&method.sig.inputs);
        self.push_self_receiver_ref_scope(&method.sig.inputs);
        if has_receiver {
            self.push_self_path_override(Some("self_".to_string()));
        }
        let scoped_owner = self.scoped_type_key(self.current_struct.as_deref().unwrap_or(""));
        if (self
            .current_struct
            .as_ref()
            .is_some_and(|owner| self.c_like_enum_types.contains(owner))
            || self.c_like_enum_types.contains(&scoped_owner))
            && self.method_is_trivial_deref_self_clone(method)
        {
            self.writeln("return self_;");
        } else {
            self.emit_block(&method.block);
        }
        if has_receiver {
            self.pop_self_path_override();
        }
        self.pop_self_receiver_ref_scope();
        self.pop_param_bindings();
        self.pop_return_type_hint();
        self.pop_return_value_scope();
        self.current_struct = prev_struct;

        self.indent -= 1;
        self.writeln("}");
        self.pop_type_param_scope();
        true
    }

    pub(super) fn emit_impl_block(&mut self, i: &syn::ItemImpl) {
        if self.emit_type_alias_impl_block(i) {
            return;
        }
        // This is called for impl blocks whose struct wasn't found in the same file.
        // Emit methods as free-standing functions (fallback).
        let type_name = if let Some(tp) = Self::impl_self_type_path(i.self_ty.as_ref()) {
            tp.path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::")
        } else {
            "UnknownType".to_string()
        };

        // Determine whether the impl is cross-module (the host type isn't
        // declared in this TU). When cross-module, the methods will be
        // emitted as free-standing template functions that reference
        // `this` / `(*this)` — they will not compile as written. Surface a
        // grep-able marker block so users can find and patch the sites.
        // (When the host type IS local, the methods get drained via
        // `take_impls_for_type` in `emit_struct` and this fallback never
        // runs — see the deferred-items loop in `finalize_module`.)
        let host_tail = match i.self_ty.as_ref() {
            syn::Type::Path(tp) => tp.path.segments.last().map(|seg| seg.ident.to_string()),
            _ => None,
        };
        let host_is_local = host_tail
            .as_ref()
            .map(|name| self.declared_item_names.contains(name))
            .unwrap_or(false);
        // Cross-file orphan impl whose host type IS declared somewhere in
        // the crate (directly as a struct, or via a type-alias chain that
        // resolves to a struct): suppress the free-fn emission here —
        // the host's file will absorb the methods via the
        // `seed_cross_file_impl_blocks` injection in `emit_file` (slice
        // Y2/Z). Without this suppression the methods would be emitted
        // twice: once as in-struct members in the host's file, and once
        // as broken free-fns here.
        if !host_is_local
            && let Some(tail) = host_tail.as_ref()
        {
            let resolved = self.resolve_type_alias_tail(tail);
            if self.cross_file_struct_tails.contains(resolved) {
                self.writeln(&format!(
                    "// orphan impl: methods for `{}` absorbed into the host struct in another file",
                    type_name
                ));
                return;
            }
        }
        if !host_is_local {
            self.writeln(&format!(
                "// TODO orphan impl: methods for `{}` were declared in this file but the",
                type_name
            ));
            self.writeln(
                "// host type lives in another module / TU. These methods are emitted as",
            );
            self.writeln(
                "// free-standing template functions that reference `this`/`(*this)`,",
            );
            self.writeln(
                "// which is not valid C++ outside a member function. Move them into the",
            );
            self.writeln(
                "// host type's struct body, or rewrite `this`/`(*this)` to an explicit",
            );
            self.writeln("// `self_` parameter and qualify all call sites accordingly.");
            // Wrap orphan-impl body in `#if 0` so the broken free-standing
            // methods are visible for inspection but excluded from the build.
            // Matches the manual `#if 0 ... #endif patcher` blocks the
            // post-transpile patcher adds to maintained ports (rc_port,
            // arc_port, …) when an impl block names a foreign host type.
            self.writeln("#if 0  // patcher: orphan-impl block stubbed");
        }
        self.writeln(&format!("// Methods for {}", type_name));
        for item in &i.items {
            self.emit_impl_item(item);
        }
        if !host_is_local {
            self.writeln("#endif  // patcher: end orphan-impl stub");
        }
    }

    pub(super) fn emit_impl_item(&mut self, item: &syn::ImplItem) {
        match item {
            syn::ImplItem::Fn(method) => self.emit_method(method),
            syn::ImplItem::Const(c) => {
                let name = escape_cpp_keyword(&c.ident.to_string());
                if !self.mark_emitted_non_method_member_name(&name) {
                    return;
                }
                let ty = self.map_type(&c.ty);
                let expr = self.emit_impl_const_expr(c);
                // A const whose initializer needs the enclosing type COMPLETE —
                // e.g. `const WIDTH: usize = size_of::<Self>()` — can't be an
                // in-class `static constexpr` (the class is incomplete within its
                // own body). Emit it as a `static constexpr` member FUNCTION: a
                // function body is a complete-class context, so `sizeof(Self)` is
                // valid there. `Owner::NAME` value uses are rewritten to
                // `Owner::NAME()` in a finalize post-pass.
                if let Some(owner_leaf) = self.const_init_requires_complete_self(&expr) {
                    self.writeln(&format!(
                        "static constexpr {} {}() {{ return {}; }}",
                        ty, name, expr
                    ));
                    self.self_sizeof_const_fns.insert((owner_leaf, name));
                }
                // Self-referential const (type is the enclosing struct):
                // split into declaration inside struct + definition after.
                else if self.is_self_referential_const_type(&ty) {
                    self.writeln(&format!("static const {} {};", ty, name));
                    if let Some(ref struct_name) = self.current_struct.clone() {
                        self.deferred_self_const_defs.push(format!(
                            "inline const {} {}::{} = {};",
                            ty, struct_name, name, expr
                        ));
                    }
                } else {
                    let storage_spec = if self.impl_const_type_requires_inline_const(&ty) {
                        "static inline const"
                    } else {
                        "static constexpr"
                    };
                    self.writeln(&format!("{} {} {} = {};", storage_spec, ty, name, expr));
                }
            }
            syn::ImplItem::Type(t) => {
                if self.should_soften_dependent_assoc_mode()
                    && self.type_contains_dependent_assoc(&t.ty)
                {
                    self.writeln(&format!(
                        "// Rust-only dependent associated type alias skipped in constrained mode: {}",
                        t.ident
                    ));
                    return;
                }
                if self.should_soften_dependent_assoc_mode()
                    && self.type_contains_unbound_single_letter_generic(&t.ty)
                {
                    self.writeln(&format!(
                        "// Rust-only associated type alias with unbound generic skipped in constrained mode: {}",
                        t.ident
                    ));
                    return;
                }
                let alias_rust_name = t.ident.to_string();
                let name = escape_cpp_keyword(&alias_rust_name);
                let alias_is_type_param = self.is_type_param_in_scope(&alias_rust_name);
                let mut ty =
                    self.normalize_assoc_alias_target_type(&alias_rust_name, self.map_type(&t.ty));
                ty = Self::rewrite_private_keyword_namespace_in_type_path(&ty);
                if self.current_struct_is_generic()
                    && let Some(dependent_ty) =
                        self.maybe_make_owner_cpp_type_dependent_in_template_scope(&ty)
                {
                    ty = dependent_ty;
                }
                if (ty == name || ty.starts_with(&format!("{}<", name)))
                    && !(alias_is_type_param && ty == name)
                {
                    let ns_prefix = if self.module_stack.is_empty() {
                        String::new()
                    } else {
                        let escaped_module = self
                            .module_stack
                            .iter()
                            .map(|seg| escape_cpp_keyword(seg))
                            .collect::<Vec<_>>()
                            .join("::");
                        format!("{}::", escaped_module)
                    };
                    ty = format!("::{}{}", ns_prefix, ty);
                }
                // When inside a namespace that shadows a sibling module (e.g.,
                // tests::iter shadows ::iter), qualify with :: to avoid collision
                if !self.module_stack.is_empty() && !ty.starts_with("::") {
                    let first_segment = ty.split("::").next().unwrap_or("");
                    if !first_segment.is_empty()
                        && self.declared_item_names.contains(first_segment)
                        && first_segment
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_lowercase())
                    {
                        ty = format!("::{}", ty);
                    }
                }
                if ty.starts_with("::") {
                    let scope_key = self.module_stack.join("::");
                    ty = self.rewrite_forced_global_private_alias_root_for_scope(&ty, &scope_key);
                }
                if alias_is_type_param && ty == name {
                    // C++ forbids redeclaring a template parameter name as a nested alias.
                    // The template parameter itself remains the associated type binding.
                    return;
                }
                let alias_conflicts_with_current_struct =
                    self.current_struct.as_ref().is_some_and(|owner| {
                        let owner_tail = owner.rsplit("::").next().unwrap_or(owner);
                        owner == &name
                            || owner == &alias_rust_name
                            || owner_tail == name
                            || owner_tail == alias_rust_name
                    });
                let alias_target_is_current_struct =
                    self.current_struct.as_ref().is_some_and(|owner| {
                        let owner_tail = owner.rsplit("::").next().unwrap_or(owner);
                        let ty_trimmed = ty.trim_start_matches("::");
                        let ty_tail = ty_trimmed.rsplit("::").next().unwrap_or(ty_trimmed);
                        ty_trimmed == owner
                            || ty_trimmed == owner_tail
                            || ty_tail == owner
                            || ty_tail == owner_tail
                            || ty_trimmed.starts_with(&format!("{}<", owner))
                            || ty_trimmed.starts_with(&format!("{}<", owner_tail))
                            || ty_tail.starts_with(&format!("{}<", owner))
                            || ty_tail.starts_with(&format!("{}<", owner_tail))
                    });
                if alias_conflicts_with_current_struct && alias_target_is_current_struct {
                    // Rust patterns like `type Deserializer = Self;` on `struct Deserializer`
                    // are identity aliases that collide with the enclosing C++ type name.
                    // Keep the owner type as the canonical spelling and skip this alias.
                    return;
                }
                if !self.mark_emitted_non_method_member_name(&name) {
                    return;
                }
                self.writeln(&format!("using {} = {};", name, ty));
            }
            _ => {
                self.writeln("// TODO: unhandled impl item");
            }
        }
    }

    pub(super) fn emit_impl_const_expr(&self, c: &syn::ImplItemConst) -> String {
        if let Some(resolved) = self.resolve_shadowed_impl_const_expr(c) {
            return resolved;
        }
        self.emit_expr_to_string_with_expected(&c.expr, Some(&c.ty))
    }

    pub(super) fn emit_method(&mut self, method: &syn::ImplItemFn) {
        let profile_method = std::env::var_os("RUSTY_CPP_PROFILE_METHODS").is_some();
        let method_profile_start = if profile_method {
            let owner = self.current_struct.as_deref().unwrap_or("<free-impl>");
            eprintln!(
                "[rusty-cpp][emit-method] start {}::{}",
                owner, method.sig.ident
            );
            Some(std::time::Instant::now())
        } else {
            None
        };
        let rewritten_local_template_method =
            self.try_rewrite_local_class_member_template_method(method);
        let method = rewritten_local_template_method.as_ref().unwrap_or(method);
        let rewritten_shadowing_method = self.try_rename_shadowing_method_type_params(method);
        let method = rewritten_shadowing_method.as_ref().unwrap_or(method);
        let declaration_only = self.method_emission_declaration_only;
        let hoisted_local_enums = if declaration_only {
            Vec::new()
        } else {
            self.collect_hoistable_local_enums_in_block(&method.block)
        };
        let hoisted_local_generic_structs = if declaration_only {
            Vec::new()
        } else {
            self.collect_hoistable_local_generic_structs_in_block(&method.block)
        };
        let mut hoisted_local_type_names: HashSet<String> = hoisted_local_generic_structs
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        hoisted_local_type_names.extend(hoisted_local_enums.iter().map(|e| e.ident.to_string()));
        if !declaration_only
            && (!hoisted_local_enums.is_empty() || !hoisted_local_generic_structs.is_empty())
        {
            self.push_type_param_scope(&method.sig.generics);
            if !hoisted_local_type_names.is_empty() {
                self.hoisted_local_type_name_scopes
                    .push(hoisted_local_type_names.clone());
            }
            self.emit_hoisted_local_enums_for_block(&method.block, &hoisted_local_enums);
            self.emit_hoisted_local_generic_structs_for_block(
                &method.block,
                &hoisted_local_generic_structs,
            );
            if !hoisted_local_type_names.is_empty() {
                self.hoisted_local_type_name_scopes.pop();
            }
            self.pop_type_param_scope();
        }
        let filtered_method_block = if declaration_only || hoisted_local_type_names.is_empty() {
            None
        } else {
            Some(self.strip_hoisted_local_generic_struct_items_from_block(
                &method.block,
                &hoisted_local_type_names,
            ))
        };
        let block_for_emission = filtered_method_block.as_ref().unwrap_or(&method.block);

        let method_ident = method.sig.ident.to_string();
        let deduced_return_aliases = self
            .deduced_callable_return_type_aliases_for_function(
                &method.sig.generics,
                &method.sig.inputs,
                &method.sig.output,
            )
            .into_iter()
            .filter(|(alias_name, alias_expr)| {
                // Defensive guard against self-referential aliases in specialized impls.
                // Example bad emission:
                // `using T = std::remove_cvref_t<std::invoke_result_t<F&, T, T>>;`
                !Self::cpp_type_expr_mentions_identifier(alias_expr, alias_name)
            })
            .collect::<Vec<_>>();
        let deduced_return_alias_names: HashSet<String> = deduced_return_aliases
            .iter()
            .map(|(name, _)| name.clone())
            .collect();
        let mut emitted_generics = method.sig.generics.clone();
        if !deduced_return_alias_names.is_empty() {
            emitted_generics.params = emitted_generics
                .params
                .into_iter()
                .filter(|param| match param {
                    syn::GenericParam::Type(tp) => {
                        !deduced_return_alias_names.contains(&tp.ident.to_string())
                    }
                    _ => true,
                })
                .collect();
        }
        let mut forced_placeholder_params: std::collections::BTreeSet<String> =
            std::collections::BTreeSet::new();
        for arg in &method.sig.inputs {
            if let syn::FnArg::Typed(pat_type) = arg {
                self.collect_unscoped_placeholder_type_idents_in_type(
                    &pat_type.ty,
                    &mut forced_placeholder_params,
                );
            }
        }
        if let syn::ReturnType::Type(_, ret_ty) = &method.sig.output {
            self.collect_unscoped_placeholder_type_idents_in_type(
                ret_ty,
                &mut forced_placeholder_params,
            );
        }
        let mut emitted_generic_names: HashSet<String> = emitted_generics
            .params
            .iter()
            .filter_map(|param| match param {
                syn::GenericParam::Type(tp) => Some(tp.ident.to_string()),
                syn::GenericParam::Const(cp) => Some(cp.ident.to_string()),
                syn::GenericParam::Lifetime(lp) => Some(lp.lifetime.ident.to_string()),
            })
            .collect();
        for name in forced_placeholder_params {
            if emitted_generic_names.insert(name.clone()) {
                emitted_generics
                    .params
                    .push(syn::GenericParam::Type(syn::TypeParam {
                        attrs: Vec::new(),
                        ident: syn::Ident::new(&name, proc_macro2::Span::call_site()),
                        colon_token: None,
                        bounds: syn::punctuated::Punctuated::new(),
                        eq_token: None,
                        default: None,
                    }));
            }
        }
        let emitted_template_key = self.emitted_template_signature_key(&emitted_generics);
        let mut method_template_prefix_lines = self.template_prefix_lines(&emitted_generics, true);
        if method_template_prefix_lines.is_empty() {
            let forced_shadow_params: Vec<String> = emitted_generics
                .params
                .iter()
                .filter_map(|param| match param {
                    syn::GenericParam::Type(tp) if tp.ident.to_string().starts_with("__") => {
                        Some(format!("typename {}", tp.ident))
                    }
                    syn::GenericParam::Const(cp) if cp.ident.to_string().starts_with("__") => {
                        Some(format!("{} {}", self.map_type(&cp.ty), cp.ident))
                    }
                    _ => None,
                })
                .collect();
            if !forced_shadow_params.is_empty() {
                method_template_prefix_lines =
                    vec![format!("template<{}>", forced_shadow_params.join(", "))];
            }
        }

        self.push_type_param_scope(&method.sig.generics);

        let mut is_drop_destructor = false;
        // Check if this method is an operator trait impl (renamed)
        let name = if let Some(ref struct_name) = self.current_struct {
            let scoped_name = self.scoped_type_key(struct_name);
            is_drop_destructor = self
                .drop_trait_methods
                .contains(&(struct_name.clone(), method_ident.clone()))
                || self
                    .drop_trait_methods
                    .contains(&(scoped_name.clone(), method_ident.clone()));
            if is_drop_destructor {
                format!("~{}", struct_name)
            } else if let Some(op) = self
                .operator_renames
                .get(&(struct_name.clone(), method_ident.clone()))
                .or_else(|| {
                    self.operator_renames
                        .get(&(scoped_name, method_ident.clone()))
                })
            {
                op.clone()
            } else {
                escape_cpp_keyword(&method_ident)
            }
        } else {
            escape_cpp_keyword(&method_ident)
        };
        let is_deref_method = method_ident == "deref" && name == "operator*";
        let is_deref_mut_method = method_ident == "deref_mut";

        let mapped_return_type = self.map_impl_method_return_type(method);
        let mut return_type = mapped_return_type.clone();
        let mut can_use_mapped_auto_trailing_return = false;
        let mut softened_dependent_assoc_return = false;
        if !deduced_return_aliases.is_empty() {
            return_type = "auto".to_string();
        }
        let method_has_type_template_params = method
            .sig
            .generics
            .params
            .iter()
            .any(|param| matches!(param, syn::GenericParam::Type(_)));
        if method_has_type_template_params
            && return_type.starts_with("rusty::Result<")
            && !self.cpp_type_mentions_in_scope_type_param(&return_type)
        {
            return_type = "auto".to_string();
            can_use_mapped_auto_trailing_return = true;
        }
        let method_returns_reference = self.return_type_is_reference(&method.sig.output);
        // operator<=> must return std::partial_ordering for C++ comparison synthesis.
        // When body returns Option<Ordering>, we wrap it with to_partial_ordering.
        let wrap_body_with_partial_ordering = name == "operator<=>"
            && (return_type.contains("Option") || return_type.contains("cmp::Ordering"));
        if wrap_body_with_partial_ordering {
            return_type = "std::partial_ordering".to_string();
        }
        // In constrained emission modes (named module output and expanded tests),
        // merged trait impls can surface unconstrained associated-type returns
        // (e.g. `L::IntoIter`, `Self::Output`, `Either::Item`) that hard-fail when
        // instantiating concrete `Either<int,int>`.
        // Falling back to `auto` keeps these methods lazily checked at use sites.
        if self.should_soften_dependent_assoc_mode()
            && (self.return_type_contains_dependent_assoc(&method.sig.output)
                || self.return_type_references_current_struct_assoc(&method.sig.output))
        {
            let can_keep_explicit_current_struct_assoc_return =
                self.return_type_current_struct_assoc_aliases_emitted(&method.sig.output);
            if !can_keep_explicit_current_struct_assoc_return {
                return_type = "auto".to_string();
                softened_dependent_assoc_return = true;
            }
        }
        // When softening dependent associated-type signatures, preserve reference
        // category for methods that are semantically `-> &T` in Rust (notably
        // `Index::index`) to avoid silently decaying lvalue references to values.
        if return_type == "auto" && method_returns_reference {
            return_type = "decltype(auto)".to_string();
        }
        // For Deref/DerefMut implementations where Target resolves to a view-like
        // runtime type (`std::span`/`std::string_view`), returning references creates
        // dangling references when body helpers synthesize temporaries. Return
        // view targets by value instead.
        if (is_deref_method || is_deref_mut_method)
            && method_returns_reference
            && let Some(target_ty) = self.resolve_current_struct_assoc_cpp_type("Target")
            && Self::is_view_like_cpp_type(&target_ty)
        {
            return_type = if is_deref_mut_method {
                Self::to_mutable_view_cpp_type(&target_ty)
            } else {
                target_ty
            };
        }
        let should_force_typed_option_ctor = return_type == "auto"
            && self.should_soften_dependent_assoc_mode()
            && self.return_type_is_option_like(&method.sig.output)
            && !((self.return_type_contains_dependent_assoc(&method.sig.output)
                || self.return_type_references_current_struct_assoc(&method.sig.output))
                && !self.return_type_current_struct_assoc_aliases_emitted(&method.sig.output));
        let is_data_enum_owner = self.current_struct.as_ref().is_some_and(|struct_name| {
            let scoped = self.scoped_type_key(struct_name);
            self.data_enum_variants_by_enum.contains_key(struct_name)
                || self.data_enum_variants_by_enum.contains_key(&scoped)
        });
        let use_deref_mut_ref_fallback = is_deref_mut_method
            && softened_dependent_assoc_return
            && is_data_enum_owner
            && self
                .resolve_current_struct_assoc_cpp_type_if_concrete("Target")
                .is_none();
        let mapped_return_looks_concrete = !mapped_return_type.contains("/* TODO:")
            && !type_string_has_auto_placeholder(&mapped_return_type)
            && !self.mapped_assoc_type_contains_unbound_placeholder(&mapped_return_type);
        let mapped_return_mentions_template_params =
            self.cpp_type_mentions_in_scope_type_param(&mapped_return_type);
        let auto_trailing_return = if return_type == "auto"
            && deduced_return_aliases.is_empty()
            && mapped_return_looks_concrete
            && !softened_dependent_assoc_return
            && (can_use_mapped_auto_trailing_return || mapped_return_mentions_template_params)
        {
            Some(mapped_return_type.clone())
        } else {
            None
        };

        // Analyze receiver to determine method kind
        let (mut qualifier, mut is_static) = match method.sig.inputs.first() {
            Some(syn::FnArg::Receiver(recv)) => {
                if recv.reference.is_some() {
                    if recv.mutability.is_some() {
                        // &mut self → non-const method
                        ("".to_string(), false)
                    } else {
                        // &self → const method
                        (" const".to_string(), false)
                    }
                } else if recv.mutability.is_some() {
                    // `fn foo(mut self)` — the `mut` keyword is a
                    // binding modifier that lets the body mutate
                    // through `self` (call `&mut self` methods, do
                    // `self.field = ...`, etc.). Emit as non-const so
                    // the body's mutable accesses (e.g. btree_port's
                    // `fn split(mut self, alloc)` calling its own
                    // non-const `split_leaf_data`) typecheck.
                    ("".to_string(), false)
                } else if Self::body_moves_out_self_field(&method.block)
                    && !self.current_struct_is_bitflags_like()
                    && !self.current_struct_is_copy()
                {
                    // `fn foo(self)` whose body moves a non-Copy field
                    // out of `self` (e.g. `Foo { kv: self.kv, ... }`
                    // where `kv: (K, V)` is potentially non-Copy).
                    // The transpiler lowers `self.kv` to
                    // `std::move(this->kv)`. On a const method
                    // `std::move(this->X)` produces `const T&&`, which
                    // a copy ctor consumes (and copies). For move-only
                    // T (e.g. `std::pair<long, rusty::Function<…>>`)
                    // there IS no copy ctor → compile error.
                    //
                    // Emit as non-const so the move-out is legitimate.
                    // This is the right semantic anyway — the method
                    // consumes self in Rust, which more closely
                    // matches non-const C++ method behavior on `*this`.
                    //
                    // Bitflags-like types are exempt: their single
                    // primitive field (`_0: uN`) is trivially Copy, so
                    // `self.0` reads inside operator impls (`fn not(self)`
                    // → `Self::from_bits_truncate(!self.0)`) are not
                    // moves. Emitting these as const lets call sites
                    // like `!TestFlags::A` (where A is `static const`)
                    // type-check.
                    ("".to_string(), false)
                } else {
                    // `fn foo(self)` (no `mut`, no field moves) →
                    // emit as const. The body only reads through
                    // self in ways the compiler can lift to copies.
                    //
                    // Modeling as a C++ const method lets call sites
                    // where the receiver is bound through a const
                    // borrow (e.g., from `std::as_const(_m).unwrap()`
                    // in a match arm — see btree_port B3 in
                    // tests/btree_port_iter_remove_movonly_test.cpp)
                    // dispatch successfully.
                    //
                    // Heuristic limitation: `body_moves_out_self_field`
                    // looks for `self.<ident>` and counts ALL field
                    // reads as potential moves (since the field's
                    // Copy-ness depends on T which we don't know at
                    // emit time). For Copy-only-field bodies (like
                    // `fn forget_type(self) -> NodeRef { NodeRef {
                    // height: self.height, node: self.node, _marker:
                    // PhantomData } }` where `height: u8` and `node:
                    // NonNull` are both Copy), we over-classify as
                    // non-const, but that's the SAFE direction.
                    (" const".to_string(), false)
                }
            }
            _ => {
                // No self → static method
                ("".to_string(), true)
            }
        };
        if is_drop_destructor {
            // C++ destructors cannot be static/const-qualified and have no return type.
            qualifier.clear();
            is_static = false;
        }

        // Build params list (skip self receiver)
        let mut params: Vec<String> = Vec::new();
        let mut param_pattern_bindings: Vec<(syn::Pat, String)> = Vec::new();
        for (idx, arg) in method.sig.inputs.iter().enumerate() {
            let syn::FnArg::Typed(pat_type) = arg else {
                continue;
            };
            let ty = self.resolve_param_cpp_type(&pat_type.ty);
            let param_name = match pat_type.pat.as_ref() {
                syn::Pat::Ident(pi) => escape_cpp_keyword(&pi.ident.to_string()),
                syn::Pat::Wild(_) => format!("_arg{}", idx),
                _ => {
                    let generated = format!("_arg{}", idx);
                    param_pattern_bindings.push(((*pat_type.pat).clone(), generated.clone()));
                    generated
                }
            };
            params.push(format!("{} {}", ty, param_name));
        }

        if self.should_skip_recursive_bitflags_forwarder(&name, method, is_static) {
            self.pop_type_param_scope();
            return;
        }

        // `#[cpp_ctor]` lowering — see `emit_cpp_ctor`. Emits a real
        // C++ constructor instead of the default `static Owner Owner::new_(args)`
        // factory; only triggered when the function body is a single
        // `Self { ... }` (or `Owner { ... }`) struct literal that the
        // ctor init list can express directly. Anything richer falls
        // through to the regular factory emission.
        if Self::has_cpp_ctor_attr(&method.attrs) {
            let ctor_owner = self
                .method_emission_out_of_line_owner
                .clone()
                .or_else(|| self.current_struct.clone());
            if let Some(owner) = ctor_owner {
                if let Some(struct_lit) =
                    Self::extract_cpp_ctor_struct_literal(block_for_emission, &owner)
                {
                    let out_of_line = self.method_emission_out_of_line_owner.is_some();
                    let declaration_only = self.method_emission_declaration_only;
                    self.emit_cpp_ctor(
                        &owner,
                        out_of_line,
                        &params,
                        struct_lit,
                        declaration_only,
                    );
                    self.pop_type_param_scope();
                    return;
                }
            }
        }

        let out_of_line_owner = self.method_emission_out_of_line_owner.clone();
        let mut emitted_return_type = return_type.clone();
        let mut emitted_auto_trailing_return = auto_trailing_return.clone();
        if let Some(owner) = out_of_line_owner.as_ref() {
            emitted_return_type = self
                .qualify_out_of_line_owner_assoc_aliases_in_cpp_type(&emitted_return_type, owner);
            emitted_auto_trailing_return = emitted_auto_trailing_return
                .map(|ty| self.qualify_out_of_line_owner_assoc_aliases_in_cpp_type(&ty, owner));
        }
        // §B signature template-defer: a member whose SIGNATURE references the
        // enclosing type's self-sizeof const (`Group::WIDTH()`) can't be a plain
        // member — a member's signature is NOT a complete-class context, so
        // `sizeof(Self)` there is ill-formed while the class is still being
        // defined (the in-class declaration). Make it a member TEMPLATE with a
        // defaulted `Self_`: rewriting `Owner::CONST()` → `Self_::CONST()` makes
        // the size a DEPENDENT expression, evaluated lazily at the call site
        // where the type is complete. The default `Self_ = Owner` lets existing
        // `Owner::method()` calls resolve unchanged (function-template default
        // args), so no call-site rewrite is needed. Guarded to methods with no
        // other template params (so we never emit two `template<...>` prefixes).
        if method_template_prefix_lines.is_empty()
            && let Some(owner_leaf) = self
                .current_struct
                .as_deref()
                .map(|s| s.rsplit("::").next().unwrap_or(s).to_string())
        {
            // The bare const reference `Owner::CONST` is what appears in the
            // signature here; the `CONST → CONST()` call form is added later by
            // the finalize post-pass, which keys on `Owner::CONST` and so won't
            // touch the `Self_::CONST()` we produce.
            let self_const = self
                .self_sizeof_const_fns
                .iter()
                .find(|(o, c)| {
                    *o == owner_leaf && {
                        let reference = format!("{}::{}", owner_leaf, c);
                        emitted_return_type.contains(&reference)
                            || params.iter().any(|p| p.contains(&reference))
                    }
                })
                .map(|(_, c)| c.clone());
            if let Some(const_name) = self_const {
                let from = format!("{}::{}", owner_leaf, const_name);
                let to = format!("Self_::{}()", const_name);
                emitted_return_type = emitted_return_type.replace(&from, &to);
                for p in params.iter_mut() {
                    *p = p.replace(&from, &to);
                }
                method_template_prefix_lines.push(if out_of_line_owner.is_some() {
                    // Out-of-line definition: default template args live on the
                    // in-class declaration only, not the definition.
                    "template<class Self_>".to_string()
                } else {
                    format!("template<class Self_ = {}>", owner_leaf)
                });
            }
        }
        // Rust `const fn` → C++ `constexpr`. Fold the qualifier into the prefix
        // so it applies to every signature form below (decl + def, in-line +
        // out-of-line). Without this, a `const`-initialized associated item whose
        // initializer calls a `const fn` becomes a runtime (dynamic) initializer
        // — a static-initialization-order hazard. (A `const fn` whose transpiled
        // body is not constexpr-eligible, e.g. uses reinterpret_cast, is handled
        // by stripping the qualifier in post_transpile_patch.py.)
        let constexpr_prefix = if method.sig.constness.is_some() {
            "constexpr "
        } else {
            ""
        };
        let static_prefix = format!(
            "{}{}",
            if is_static && out_of_line_owner.is_none() {
                "static "
            } else {
                ""
            },
            constexpr_prefix
        );
        let emitted_callable_name = if let Some(ref owner) = out_of_line_owner {
            format!("{}::{}", owner, name)
        } else {
            name.clone()
        };
        if !self.method_emission_skip_conflict_registration {
            let conflict_key = self.emitted_method_conflict_key(
                &name,
                &emitted_template_key,
                &qualifier,
                is_static,
                &params,
            );
            if !self.mark_emitted_method_conflict_key(conflict_key) {
                self.pop_type_param_scope();
                return;
            }
        }
        for line in &method_template_prefix_lines {
            self.writeln(line);
        }
        if self.method_emission_declaration_only {
            if is_drop_destructor {
                self.writeln(&format!(
                    "{}({}) noexcept(false);",
                    emitted_callable_name,
                    params.join(", ")
                ));
            } else if let Some(trailing_return) = emitted_auto_trailing_return.as_ref() {
                self.writeln(&format!(
                    "{}{} {}({}){} -> {};",
                    static_prefix,
                    emitted_return_type,
                    emitted_callable_name,
                    params.join(", "),
                    qualifier,
                    trailing_return
                ));
            } else {
                self.writeln(&format!(
                    "{}{} {}({}){};",
                    static_prefix,
                    emitted_return_type,
                    emitted_callable_name,
                    params.join(", "),
                    qualifier
                ));
            }
            self.pop_type_param_scope();
            return;
        }
        let mut hoisted_impl_override_state = Some({
            let (
                local_impl_overrides,
                local_drop_overrides,
                local_operator_overrides,
                local_inherent_method_overrides,
            ) = self.collect_local_impl_overrides(&method.block.stmts, &hoisted_local_type_names);

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
            (
                prev_impl_overrides,
                inserted_drop_overrides,
                prev_operator_overrides,
                prev_inherent_overrides,
            )
        });
        let mut hoisted_local_generic_param_metadata = Some(
            self.push_hoisted_local_generic_type_param_metadata(&hoisted_local_generic_structs),
        );
        let mut hoisted_local_type_scope_pushed = false;
        if !hoisted_local_type_names.is_empty() {
            self.hoisted_local_type_name_scopes
                .push(hoisted_local_type_names.clone());
            hoisted_local_type_scope_pushed = true;
        }
        if is_drop_destructor {
            self.writeln(&format!(
                "{}({}) noexcept(false) {{",
                emitted_callable_name,
                params.join(", ")
            ));
        } else if let Some(trailing_return) = emitted_auto_trailing_return.as_ref() {
            self.writeln(&format!(
                "{}{} {}({}){} -> {} {{",
                static_prefix,
                emitted_return_type,
                emitted_callable_name,
                params.join(", "),
                qualifier,
                trailing_return
            ));
        } else {
            self.writeln(&format!(
                "{}{} {}({}){} {{",
                static_prefix,
                emitted_return_type,
                emitted_callable_name,
                params.join(", "),
                qualifier
            ));
        }
        self.indent += 1;
        if let Some(owner) = out_of_line_owner.as_ref() {
            self.emit_out_of_line_owner_assoc_aliases(owner);
        }
        for (alias_name, alias_ty) in &deduced_return_aliases {
            let alias_ty = Self::rewrite_private_keyword_namespace_in_type_path(alias_ty);
            self.writeln(&format!("using {} = {};", alias_name, alias_ty));
        }
        if is_drop_destructor {
            self.writeln("if (_rusty_forgotten) { return; }");
        }
        if wrap_body_with_partial_ordering {
            // Wrap operator<=> body: return to_partial_ordering([&]() -> Option<Ordering> { <body> }())
            self.writeln(&format!(
                "return rusty::to_partial_ordering([&]() -> {} {{",
                self.map_return_type(&method.sig.output)
            ));
            self.indent += 1;
        }
        self.push_return_value_scope(&emitted_return_type);
        self.push_return_type_hint(&method.sig.output);
        self.push_force_typed_option_ctor_scope(should_force_typed_option_ctor);
        self.push_param_bindings(&method.sig.inputs);
        self.push_self_receiver_ref_scope(&method.sig.inputs);
        self.push_deref_method_scope(is_deref_method);
        self.push_deref_mut_method_scope(is_deref_mut_method);
        self.push_deref_mut_ref_fallback_scope(use_deref_mut_ref_fallback);
        self.push_transient_statement_scope();
        // Emit using-namespace for methods merged from sibling modules
        for ns in &self.merged_method_using_namespaces.clone() {
            self.writeln(&format!("using namespace {};", ns));
        }
        for (pat, param_name) in &param_pattern_bindings {
            let mut binding_stmts = Vec::new();
            let mut binding_map = HashMap::new();
            if self.collect_pattern_binding_stmts_with_cpp_name_map(
                pat,
                param_name,
                &mut binding_stmts,
                &mut binding_map,
            ) {
                for stmt in binding_stmts {
                    self.writeln(&stmt);
                }
                self.register_statement_scope_binding_map(&binding_map);
            } else {
                self.writeln("// TODO: complex method parameter pattern binding");
            }
        }
        // For operator methods from const _ blocks whose body calls a
        // known bitwise helper (union, intersection, etc.), emit the
        // body as a direct bitwise operation on `_0` field instead of
        // calling the unavailable method.
        if name.starts_with("operator") && !is_drop_destructor {
            if let Some(inline_body) = self.try_inline_operator_body(&name, method) {
                self.writeln(&inline_body);
                self.pop_transient_statement_scope();
                self.pop_deref_mut_ref_fallback_scope();
                self.pop_deref_mut_method_scope();
                self.pop_deref_method_scope();
                self.pop_self_receiver_ref_scope();
                self.pop_param_bindings();
                self.pop_force_typed_option_ctor_scope();
                self.pop_return_type_hint();
                self.pop_return_value_scope();
                self.indent -= 1;
                self.writeln("}");
                if let Some((
                    prev_impl_overrides,
                    inserted_drop_overrides,
                    prev_operator_overrides,
                    prev_inherent_overrides,
                )) = hoisted_impl_override_state.take()
                {
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
                }
                if let Some(metadata) = hoisted_local_generic_param_metadata.take() {
                    self.restore_hoisted_local_generic_type_param_metadata(metadata);
                }
                if hoisted_local_type_scope_pushed {
                    self.hoisted_local_type_name_scopes.pop();
                }
                self.pop_type_param_scope();
                return;
            }
        }
        let emit_serializer_end_fast_path = method_ident == "end"
            && method.sig.inputs.len() == 1
            && emitted_return_type.starts_with("rusty::Result<std::tuple<>,")
            && self.current_struct.as_ref().is_some_and(|struct_name| {
                struct_name == "Serializer"
                    || struct_name
                        .rsplit("::")
                        .next()
                        .is_some_and(|tail| tail == "Serializer")
            });
        if emit_serializer_end_fast_path {
            self.writeln("if constexpr (requires { this->tokens; rusty::first(this->tokens); }) {");
            self.indent += 1;
            self.writeln("auto __peek = rusty::first(this->tokens);");
            self.writeln("if (__peek.is_some()) {");
            self.indent += 1;
            self.writeln(
                "auto&& __tok = rusty::detail::deref_if_pointer(std::as_const(__peek).unwrap());",
            );
            self.writeln("bool __matched_end = false;");
            self.writeln(
                "if constexpr (requires { rusty::detail::variant_holds<token::Token_SeqEnd>(__tok); }) {",
            );
            self.indent += 1;
            self.writeln("__matched_end = __matched_end || rusty::detail::variant_holds<token::Token_SeqEnd>(__tok);");
            self.indent -= 1;
            self.writeln("}");
            self.writeln("if constexpr (requires { rusty::detail::variant_holds<token::Token_TupleEnd>(__tok); }) {");
            self.indent += 1;
            self.writeln("__matched_end = __matched_end || rusty::detail::variant_holds<token::Token_TupleEnd>(__tok);");
            self.indent -= 1;
            self.writeln("}");
            self.writeln("if constexpr (requires { rusty::detail::variant_holds<token::Token_TupleStructEnd>(__tok); }) {");
            self.indent += 1;
            self.writeln("__matched_end = __matched_end || rusty::detail::variant_holds<token::Token_TupleStructEnd>(__tok);");
            self.indent -= 1;
            self.writeln("}");
            self.writeln("if constexpr (requires { rusty::detail::variant_holds<token::Token_TupleVariantEnd>(__tok); }) {");
            self.indent += 1;
            self.writeln("__matched_end = __matched_end || rusty::detail::variant_holds<token::Token_TupleVariantEnd>(__tok);");
            self.indent -= 1;
            self.writeln("}");
            self.writeln(
                "if constexpr (requires { rusty::detail::variant_holds<token::Token_MapEnd>(__tok); }) {",
            );
            self.indent += 1;
            self.writeln("__matched_end = __matched_end || rusty::detail::variant_holds<token::Token_MapEnd>(__tok);");
            self.indent -= 1;
            self.writeln("}");
            self.writeln("if constexpr (requires { rusty::detail::variant_holds<token::Token_StructEnd>(__tok); }) {");
            self.indent += 1;
            self.writeln("__matched_end = __matched_end || rusty::detail::variant_holds<token::Token_StructEnd>(__tok);");
            self.indent -= 1;
            self.writeln("}");
            self.writeln("if constexpr (requires { rusty::detail::variant_holds<token::Token_StructVariantEnd>(__tok); }) {");
            self.indent += 1;
            self.writeln("__matched_end = __matched_end || rusty::detail::variant_holds<token::Token_StructVariantEnd>(__tok);");
            self.indent -= 1;
            self.writeln("}");
            self.writeln("if (__matched_end) {");
            self.indent += 1;
            self.writeln("static_cast<void>(rusty::next_token((*this)));");
            self.writeln(&format!(
                "return {}::Ok(std::make_tuple());",
                emitted_return_type
            ));
            self.indent -= 1;
            self.writeln("}");
            self.indent -= 1;
            self.writeln("}");
            self.indent -= 1;
            self.writeln("}");
        }
        if let Some(clone_return_stmt) = self.try_emit_fieldwise_clone_return_stmt(method) {
            self.writeln(&clone_return_stmt);
        } else {
            self.emit_block(block_for_emission);
        }
        self.pop_transient_statement_scope();
        self.pop_deref_mut_ref_fallback_scope();
        self.pop_deref_mut_method_scope();
        self.pop_deref_method_scope();
        self.pop_self_receiver_ref_scope();
        self.pop_param_bindings();
        self.pop_force_typed_option_ctor_scope();
        self.pop_return_type_hint();
        self.pop_return_value_scope();
        if wrap_body_with_partial_ordering {
            self.indent -= 1;
            self.writeln("}());");
        }
        self.indent -= 1;
        self.writeln("}");
        if let Some((
            prev_impl_overrides,
            inserted_drop_overrides,
            prev_operator_overrides,
            prev_inherent_overrides,
        )) = hoisted_impl_override_state.take()
        {
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
        }
        if let Some(metadata) = hoisted_local_generic_param_metadata.take() {
            self.restore_hoisted_local_generic_type_param_metadata(metadata);
        }
        if hoisted_local_type_scope_pushed {
            self.hoisted_local_type_name_scopes.pop();
        }
        self.pop_type_param_scope();
        if let Some(start) = method_profile_start {
            let owner = self.current_struct.as_deref().unwrap_or("<free-impl>");
            eprintln!(
                "[rusty-cpp][emit-method] done {}::{}: {:.3}s",
                owner,
                method.sig.ident,
                start.elapsed().as_secs_f64()
            );
        }
    }

    pub(super) fn emit_constructor_hint_arg_cpp(
        &self,
        arg: &syn::Expr,
        if_let_unwrap_method: Option<&'static str>,
    ) -> String {
        let arg = self.extract_value_expr(arg).unwrap_or(arg);
        if let Some(unwrap_method) = if_let_unwrap_method {
            if let syn::Expr::Path(path) = arg {
                if path.path.segments.len() == 1 {
                    let ident = path.path.segments[0].ident.to_string();
                    if ident != "self" && self.lookup_local_binding_type(&ident).is_none() {
                        if unwrap_method == IF_LET_OPTION_TAKE_VALUE_HELPER_MARKER {
                            return "rusty::detail::option_take_value(_iflet)".to_string();
                        }
                        return format!("_iflet.{}()", unwrap_method);
                    }
                }
            }
        }
        self.emit_expr_maybe_move(arg)
    }

    pub(super) fn emit_struct_expr_to_string_with_expected(
        &self,
        struct_expr: &syn::ExprStruct,
        expected_ty: Option<&syn::Type>,
    ) -> String {
        let expected_target_name = self.expected_struct_literal_cpp_type(struct_expr, expected_ty);
        let allow_omitted_generic_recovery = expected_target_name.as_ref().is_none_or(|name| {
            !name.contains('<')
                || name.contains("<auto")
                || name.contains("<typename")
                || name.contains("<decltype")
        });
        let variant_struct_target = self
            .try_emit_data_enum_variant_struct_literal_target(&struct_expr.path)
            .or_else(|| {
                self.fallback_data_enum_variant_struct_literal_target_from_expected(
                    &struct_expr.path,
                    expected_ty,
                )
            });
        let mut is_variant_struct_literal_target = variant_struct_target.is_some();
        let mut target_name = variant_struct_target
            .clone()
            .or(expected_target_name)
            .unwrap_or_else(|| self.emit_path_to_string(&struct_expr.path));
        if !target_name.contains("::")
            && struct_expr.path.segments.len() >= 2
            && let Some(expected_ty) = expected_ty
            && self.expected_data_enum_name(expected_ty).is_some()
            && let Some(variant_seg) = struct_expr.path.segments.last()
        {
            let expected_cpp = self.map_type(expected_ty);
            let expected_cpp_base = expected_cpp
                .split('<')
                .next()
                .unwrap_or(expected_cpp.as_str())
                .trim();
            if expected_cpp_base.contains("::") {
                target_name = format!(
                    "{}_{}",
                    expected_cpp_base,
                    escape_cpp_keyword(&variant_seg.ident.to_string())
                );
                is_variant_struct_literal_target = true;
            }
        }
        if struct_expr
            .path
            .segments
            .last()
            .is_some_and(|seg| matches!(seg.arguments, syn::PathArguments::None))
            && allow_omitted_generic_recovery
            && !is_variant_struct_literal_target
        {
            let is_local_type_path = if struct_expr.path.segments.len() == 1 {
                let local_name = struct_expr.path.segments[0].ident.to_string();
                let is_current_struct_local = self
                    .declared_type_key_for_path(&struct_expr.path)
                    .is_some_and(|key| {
                        self.current_struct
                            .as_ref()
                            .is_some_and(|owner| key.starts_with(&format!("{}::", owner)))
                    });
                self.is_local_type_name_in_scope(&local_name)
                    || self.local_declared_types.contains(&local_name)
                    || is_current_struct_local
            } else {
                false
            };
            if let Some(recovered) = self
                .recover_omitted_struct_literal_generic_type_args_from_fields(
                    struct_expr,
                    &target_name,
                )
            {
                target_name = recovered;
            } else if !target_name.contains('<')
                && !is_local_type_path
                && let Some(recovered) =
                    self.recover_omitted_local_generic_type_args(&struct_expr.path, &target_name)
            {
                target_name = recovered;
            }
        }

        let target_base_name = target_name
            .split('<')
            .next()
            .unwrap_or(target_name.as_str())
            .trim()
            .trim_start_matches("::")
            .to_string();
        let resolved_struct_name_from_path = struct_expr.path.segments.last().map(|seg| {
            let raw = seg.ident.to_string();
            if raw == "Self" {
                let current = self
                    .current_struct
                    .clone()
                    .unwrap_or_else(|| "Self".to_string());
                if current == "Self_" && !target_base_name.is_empty() {
                    target_base_name.clone()
                } else {
                    current
                }
            } else {
                raw
            }
        });
        let resolved_struct_name = if target_base_name.is_empty() {
            resolved_struct_name_from_path
        } else {
            let scoped_target_name = self.scoped_type_key(&target_base_name);
            let target_known = self.struct_field_types.contains_key(&target_base_name)
                || self.struct_field_types.contains_key(&scoped_target_name)
                || self.struct_field_cpp_names.contains_key(&target_base_name)
                || self
                    .struct_field_cpp_names
                    .contains_key(&scoped_target_name)
                || self.struct_field_order.contains_key(&target_base_name)
                || self.struct_field_order.contains_key(&scoped_target_name);
            if target_known {
                Some(target_base_name.clone())
            } else {
                resolved_struct_name_from_path
            }
        };
        // Use positional constructor syntax (not designated initializers) when the
        // struct has impl blocks that make it non-aggregate in C++.
        let has_impl_methods = resolved_struct_name.as_ref().is_some_and(|name| {
            // cpp_inherit structs are non-aggregate; designated init is illegal,
            // so always route them through the positional-ctor form (which
            // targets the synthesized fieldwise ctor).
            self.is_cpp_inherit_type(name)
                || self.type_has_drop_impl(name)
                || self
                    .impl_blocks
                    .get(name)
                    .or_else(|| self.impl_blocks.get(&self.scoped_type_key(name)))
                    .is_some_and(|items| {
                        items
                            .iter()
                            .any(|item| matches!(item, syn::ImplItem::Fn(_)))
                    })
                || self
                    .operator_renames
                    .keys()
                    .any(|(t, _)| t == name || t == &self.scoped_type_key(name))
        });
        if struct_expr.rest.is_none() && has_impl_methods {
            if let Some(struct_name) = resolved_struct_name.as_ref() {
                if let Some(field_order) = self.lookup_struct_field_order(struct_name) {
                    let field_exprs: HashMap<String, &syn::Expr> = struct_expr
                        .fields
                        .iter()
                        .filter_map(|field| match &field.member {
                            syn::Member::Named(ident) => Some((ident.to_string(), &field.expr)),
                            _ => None,
                        })
                        .collect();
                    if field_order
                        .iter()
                        .all(|name| field_exprs.contains_key(name))
                    {
                        let args: Vec<String> = field_order
                            .iter()
                            .map(|field_name| {
                                let expr = field_exprs
                                    .get(field_name)
                                    .expect("field presence checked above");
                                let field_ty = self
                                    .lookup_struct_literal_field_type(
                                        struct_expr,
                                        field_name,
                                        expected_ty,
                                    )
                                    .or_else(|| {
                                        self.lookup_struct_field_type(struct_name, field_name)
                                    });
                                let field_ty = field_ty.filter(|ty| {
                                    !self.type_contains_infer(ty)
                                        && !self.type_contains_unresolved_placeholder_like(ty)
                                        && !self.type_contains_unbound_single_letter_generic(ty)
                                });
                                let value = self
                                    .emit_expr_to_string_with_expected_and_move_if_needed(
                                        expr,
                                        field_ty.as_ref(),
                                    );
                                let value = if !value.trim_start().starts_with("std::move(")
                                    && self.should_move_local_binding_for_owned_expected_value(
                                        expr,
                                        field_ty.as_ref(),
                                    ) {
                                    format!("std::move({})", value)
                                } else {
                                    value
                                };
                                let rewrite_field_key =
                                    Self::format_by_value_field_name(None, field_name);
                                self.wrap_by_value_cycle_rewrite_field_initializer(
                                    struct_name,
                                    &rewrite_field_key,
                                    value,
                                )
                            })
                            .collect();
                        let emitted = format!("{}({})", target_name, args.join(", "));
                        if let Some(expected_ty) = expected_ty
                            && is_variant_struct_literal_target
                            && let Some(wrapped) = self
                                .wrap_data_enum_variant_payload_with_expected(expected_ty, &emitted)
                        {
                            return wrapped;
                        }
                        return emitted;
                    }
                }
            }
        }

        let mut ordered_fields: Vec<&syn::FieldValue> = struct_expr.fields.iter().collect();
        if let Some(struct_name) = resolved_struct_name.as_ref() {
            if let Some(field_order) = self.lookup_struct_field_order(struct_name) {
                let field_rank: HashMap<String, usize> = field_order
                    .iter()
                    .enumerate()
                    .map(|(idx, name)| (name.clone(), idx))
                    .collect();
                if ordered_fields
                    .iter()
                    .all(|f| matches!(f.member, syn::Member::Named(_)))
                {
                    ordered_fields.sort_by_key(|f| match &f.member {
                        syn::Member::Named(ident) => field_rank
                            .get(&ident.to_string())
                            .copied()
                            .unwrap_or(usize::MAX),
                        syn::Member::Unnamed(_) => usize::MAX,
                    });
                }
            }
        }

        // Fallback for external enum-style struct literals where local data-enum
        // metadata is unavailable (e.g. `Token::Struct { name: ..., len: ... }`).
        // Prefer associated constructor syntax (`Owner::Variant(...)`) when the
        // owner type exposes a matching inherent method.
        if struct_expr.rest.is_none()
            && struct_expr.path.segments.len() >= 2
            && !is_variant_struct_literal_target
            && ordered_fields
                .iter()
                .all(|field| matches!(field.member, syn::Member::Named(_)))
        {
            let variant_name = struct_expr
                .path
                .segments
                .last()
                .map(|seg| seg.ident.to_string())
                .unwrap_or_default();
            let variant_looks_like_enum_variant = variant_name
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase());

            let owner_segments: Vec<String> = struct_expr
                .path
                .segments
                .iter()
                .take(struct_expr.path.segments.len() - 1)
                .map(|seg| seg.ident.to_string())
                .collect();
            let owner_path = owner_segments.join("::");
            let owner_tail = owner_segments.last().cloned().unwrap_or_else(String::new);

            let owner_looks_like_type = owner_tail
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase());
            let expected_owner_matches = expected_ty
                .and_then(|ty| self.expected_type_path(ty))
                .and_then(|path| path.segments.last())
                .map(|seg| seg.ident.to_string())
                .is_some_and(|expected_owner| expected_owner == owner_tail);
            let target_is_known_struct = resolved_struct_name
                .as_ref()
                .is_some_and(|name| self.lookup_struct_field_order(name).is_some());

            if variant_looks_like_enum_variant
                && (expected_owner_matches || owner_looks_like_type)
                && !target_is_known_struct
            {
                let args: Vec<String> = ordered_fields
                    .iter()
                    .enumerate()
                    .map(|(idx, field)| {
                        let field_expected = if owner_path.is_empty() {
                            None
                        } else {
                            self.lookup_owner_method_arg_expected_type(
                                &owner_path,
                                &variant_name,
                                idx,
                                Some(&field.expr),
                            )
                            .or_else(|| {
                                self.lookup_owner_method_arg_expected_type(
                                    &owner_tail,
                                    &variant_name,
                                    idx,
                                    Some(&field.expr),
                                )
                            })
                        };
                        self.emit_expr_to_string_with_expected_and_move_if_needed(
                            &field.expr,
                            field_expected.as_ref(),
                        )
                    })
                    .collect();
                let emitted = format!("{}({})", target_name, args.join(", "));
                if let Some(expected_ty) = expected_ty
                    && let Some(wrapped) =
                        self.wrap_data_enum_variant_payload_with_expected(expected_ty, &emitted)
                {
                    return wrapped;
                }
                return emitted;
            }
        }

        let fields: Vec<String> = ordered_fields
            .iter()
            .map(|f| {
                let rust_member_name = match &f.member {
                    syn::Member::Named(ident) => ident.to_string(),
                    syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                };
                let mapped_member_name = resolved_struct_name
                    .as_ref()
                    .and_then(|name| self.lookup_struct_field_cpp_name(name, &rust_member_name))
                    .or_else(|| {
                        self.lookup_struct_literal_field_cpp_name(struct_expr, &rust_member_name)
                    });
                let mut emitted_member_name = mapped_member_name
                    .clone()
                    .unwrap_or_else(|| escape_cpp_keyword(&rust_member_name));
                if mapped_member_name.is_none()
                    && let Some(struct_name) = resolved_struct_name.as_ref()
                    && self
                        .struct_member_name_conflicts_with_method(struct_name, &emitted_member_name)
                {
                    emitted_member_name = format!("{}_field", emitted_member_name);
                }
                let field_ty = self
                    .lookup_struct_literal_field_type(struct_expr, &rust_member_name, expected_ty)
                    .or_else(|| {
                        resolved_struct_name
                            .as_ref()
                            .and_then(|name| self.lookup_struct_field_type(name, &rust_member_name))
                    });
                let field_ty = field_ty.filter(|ty| {
                    !self.type_contains_infer(ty)
                        && !self.type_contains_unresolved_placeholder_like(ty)
                        && !self.type_contains_unbound_single_letter_generic(ty)
                });
                let val = self.emit_expr_to_string_with_expected_and_move_if_needed(
                    &f.expr,
                    field_ty.as_ref(),
                );
                let val = self.rewrite_iterator_wrapper_field_initializer(
                    &f.expr,
                    field_ty.as_ref(),
                    val,
                );
                let val = self.wrap_box_field_initializer(field_ty.as_ref(), val);
                let val = if !val.trim_start().starts_with("std::move(")
                    && self.should_move_local_binding_for_owned_expected_value(
                        &f.expr,
                        field_ty.as_ref(),
                    ) {
                    format!("std::move({})", val)
                } else {
                    val
                };
                let rewrite_field_member = match &f.member {
                    syn::Member::Named(ident) => ident.to_string(),
                    syn::Member::Unnamed(idx) => format!("#{}", idx.index),
                };
                let rewrite_field_key =
                    Self::format_by_value_field_name(None, &rewrite_field_member);
                let val = if is_variant_struct_literal_target {
                    let owner_name = struct_expr
                        .path
                        .segments
                        .iter()
                        .nth_back(1)
                        .map(|seg| seg.ident.to_string())
                        .or_else(|| {
                            expected_ty
                                .and_then(|ty| self.expected_type_path(ty))
                                .and_then(|path| path.segments.last())
                                .map(|seg| seg.ident.to_string())
                        });
                    let variant_name = struct_expr
                        .path
                        .segments
                        .last()
                        .map(|seg| seg.ident.to_string());
                    if let (Some(owner_name), Some(variant_name)) = (owner_name, variant_name) {
                        let rewrite_variant_field_key = Self::format_by_value_field_name(
                            Some(&variant_name),
                            &rewrite_field_member,
                        );
                        self.wrap_by_value_cycle_rewrite_field_initializer(
                            &owner_name,
                            &rewrite_variant_field_key,
                            val,
                        )
                    } else {
                        val
                    }
                } else if let Some(struct_name) = resolved_struct_name.as_ref() {
                    self.wrap_by_value_cycle_rewrite_field_initializer(
                        struct_name,
                        &rewrite_field_key,
                        val,
                    )
                } else {
                    val
                };
                format!(".{} = {}", emitted_member_name, val)
            })
            .collect();
        // Functional-update syntax `Struct { explicit_field: …, ..base }`:
        // expand the rest's fields explicitly so designated-initializer
        // emit covers every field. Otherwise the missing fields are
        // value-initialized, which fails to compile for fields whose
        // type has no default constructor (e.g. NonNull<T>, NodeRef<…>).
        let mut all_fields = fields;
        if let Some(rest_expr) = struct_expr.rest.as_deref() {
            let explicit_members: std::collections::HashSet<String> = struct_expr
                .fields
                .iter()
                .filter_map(|f| match &f.member {
                    syn::Member::Named(ident) => Some(ident.to_string()),
                    syn::Member::Unnamed(_) => None,
                })
                .collect();
            // Find the target struct's full field list. Try the resolved
            // name first, then the bare path tail (e.g. `SplitResult`
            // before generics).
            let field_order = resolved_struct_name
                .as_ref()
                .and_then(|name| self.lookup_struct_field_order(name))
                .or_else(|| {
                    let bare = target_base_name.as_str();
                    self.lookup_struct_field_order(bare)
                });
            if let Some(field_order) = field_order {
                let rest_str = self.emit_expr_to_string(rest_expr);
                for missing in field_order
                    .iter()
                    .filter(|name| !explicit_members.contains(*name))
                {
                    let cpp_name = resolved_struct_name
                        .as_ref()
                        .and_then(|sn| self.lookup_struct_field_cpp_name(sn, missing))
                        .unwrap_or_else(|| escape_cpp_keyword(missing));
                    // Rust's `..base` moves out of base. Emit `std::move(base.field)`.
                    all_fields.push(format!(
                        ".{} = std::move({}.{})",
                        cpp_name, rest_str, cpp_name
                    ));
                }
            }
        }
        let emitted = format!("{}{{{}}}", target_name, all_fields.join(", "));
        if let Some(expected_ty) = expected_ty
            && is_variant_struct_literal_target
            && let Some(wrapped) =
                self.wrap_data_enum_variant_payload_with_expected(expected_ty, &emitted)
        {
            return wrapped;
        }
        emitted
    }

    pub(super) fn emit_method_call_template_args(
        &self,
        mc: &syn::ExprMethodCall,
        emitted_args: &[String],
    ) -> Option<String> {
        let turbofish = mc.turbofish.as_ref()?;
        let mut mapped_args: Vec<String> = Vec::new();
        let mut type_param_idx = 0usize;
        for arg in turbofish.args.iter() {
            match arg {
                syn::GenericArgument::Type(t) => {
                    if matches!(t, syn::Type::Infer(_)) {
                        mapped_args.push(self.infer_method_turbofish_type_arg_from_call_arg(
                            mc,
                            emitted_args,
                            type_param_idx,
                        )?);
                    } else {
                        mapped_args.push(self.map_type(t));
                    }
                    type_param_idx += 1;
                }
                syn::GenericArgument::Const(c) => {
                    if matches!(c, syn::Expr::Infer(_)) {
                        return None;
                    }
                    mapped_args.push(self.emit_expr_to_string(c));
                }
                _ => {}
            }
        }
        if mapped_args.is_empty() {
            return None;
        }
        let joined = mapped_args.join(", ");
        // A turbofish that maps to an invalid `<auto>` template argument cannot
        // be emitted as an explicit C++ template-argument list. This happens for
        // Rust placeholder turbofishes like `collect_tuple::<(_, _)>()` /
        // `tuple_windows::<(_, _, _, _)>()`, where the `(_, _)` tuple is a
        // `syn::Type::Tuple` of `Infer` elements that map to `std::tuple<auto,
        // auto>`. Drop the turbofish entirely so the call deduces its type from
        // context, exactly like the equivalent no-turbofish call (which the
        // emitter already handles correctly). This is a faithful, valid
        // translation — not an `auto` leak — so it always degrades here; any
        // `<auto>` that still reaches the final output is caught unconditionally
        // by the `into_output` strict-auto backstop.
        if type_string_contains_auto_template_arg(&joined) {
            return None;
        }
        Some(format!("<{}>", joined))
    }
}
