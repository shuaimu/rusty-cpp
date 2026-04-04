use proc_macro2::TokenTree;
use quote::ToTokens;
use std::collections::{HashMap, HashSet};
use syn;
use syn::parse::Parser;
use syn::parse_quote;

use crate::types;

#[derive(Debug, Clone, PartialEq, Eq)]
struct UfcsTraitCallInfo {
    function_path: String,
    method_name: String,
    receiver_is_mut: bool,
    non_receiver_arg_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VariantTypeContext {
    enum_name: String,
    template_args: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeMatchEnumKind {
    Option,
    Result,
}

/// Code generation context tracking indentation and output buffer.
pub struct CodeGen {
    output: String,
    indent: usize,
    /// Collected impl blocks indexed by type name.
    /// Populated during the first pass of emit_file.
    impl_blocks: HashMap<String, Vec<syn::ImplItem>>,
    /// Dedup keys for merged impl methods by type.
    /// Prevents duplicate C++ method emissions when expanded Rust yields
    /// overlapping inherent impl methods with the same callable signature.
    impl_method_conflict_keys: HashMap<String, HashSet<String>>,
    /// Maps (type_name, method_name) → C++ operator name for operator trait impls.
    operator_renames: HashMap<(String, String), String>,
    /// Current struct name when emitting methods inside a struct.
    /// Used to resolve `Self` type references.
    current_struct: Option<String>,
    /// Stack of in-scope generic type parameter names.
    /// Used for dependent-type emission (`typename T::Assoc`).
    type_param_scopes: Vec<HashSet<String>>,
    /// Generic type parameters declared by enum name.
    /// Used to recover variant template arguments in pattern lowering.
    enum_type_params: HashMap<String, Vec<String>>,
    /// Enum names that lower to `std::variant` wrappers (enums with data).
    /// Used to decide when match-path patterns require `std::visit` rather than `switch`.
    data_enum_types: HashSet<String>,
    /// Named struct field types for local type-context recovery in match lowering.
    struct_field_types: HashMap<String, HashMap<String, syn::Type>>,
    /// Set of variable names that are reassigned in the current block.
    /// Used for reference rebinding detection: if a `let mut r: &T` variable
    /// is reassigned, emit it as a pointer instead of a reference.
    reassigned_vars: std::collections::HashSet<String>,
    /// Immutable local names in the current block that are consumed through
    /// by-value method calls (for example `res.unwrap_err()`).
    /// Such bindings must not be emitted as `const auto`.
    consuming_method_receiver_vars: std::collections::HashSet<String>,
    /// Per-block inferred element type hints for untyped `[val; N]` locals.
    /// Derived from indexed assignments in the same block (for example
    /// `mockdata[i] = i as u8;`) to avoid invalid C++ auto-deduction drift.
    repeat_elem_type_hints: HashMap<String, syn::Type>,
    /// Scoped local bindings for expected-type propagation in expression emission.
    /// `None` means the binding exists but has no explicit type annotation.
    local_bindings: Vec<HashMap<String, Option<syn::Type>>>,
    /// Scoped Rust-local to emitted-C++ local name mappings.
    /// Used to preserve Rust shadowing semantics where repeated `let` names
    /// in the same block must be renamed for valid C++.
    local_cpp_bindings: Vec<HashMap<String, String>>,
    /// Function/method parameter bindings visible to expression/type inference.
    param_bindings: Vec<HashMap<String, syn::Type>>,
    /// Tracks whether the current method receiver is a reference (`&self` / `&mut self`).
    /// Used to lower deref of `self` correctly (`*self` should not become `*(*this)` recursion).
    self_receiver_ref_scopes: Vec<bool>,
    /// Pattern bindings introduced as `ref`/`ref mut` in the current emission scope.
    /// Used for reference-aware unary deref lowering in match/visit arms.
    pattern_ref_bindings: Vec<HashSet<String>>,
    /// Scope stack marking emission inside a mapped Deref trait method (`operator*`).
    /// Used for scoped deref fallback lowering in expanded outputs.
    deref_method_scopes: Vec<bool>,
    /// Scope stack marking emission inside a DerefMut trait method (`deref_mut`).
    deref_mut_method_scopes: Vec<bool>,
    /// When set, emit C++20 module declarations and `export` for pub items.
    module_name: Option<String>,
    /// Current inline module nesting path while emitting items.
    module_stack: Vec<String>,
    /// Traits intentionally skipped in module mode (Proxy facade unavailable),
    /// tracked by scoped path so `pub use` can be lowered as Rust-only imports.
    skipped_module_traits: HashSet<String>,
    /// Expanded Rust libtest marker names discovered from skipped
    /// `test::TestDescAndFn` metadata consts.
    /// Used to emit runnable wrappers for transpiled test-body functions.
    expanded_test_markers: Vec<String>,
    /// True when input appears to be `cargo expand --tests` output containing
    /// Rust libtest harness metadata/scaffolding.
    /// In this mode, trait facade/proxy emission is skipped to avoid unresolved
    /// Proxy runtime symbols in generated compile probes.
    expanded_libtest_mode: bool,
    /// Top-level function names emitted in this file.
    /// Used to validate marker→function wrapper emission.
    emitted_top_level_functions: HashSet<String>,
    /// `macro_rules!` names discovered in the current file (including inline modules).
    /// `use` imports that target these names are macro-only and should not emit C++ `using`.
    macro_rules_names: HashSet<String>,
    /// Top-level declared item names available as global `::Name` lookup targets.
    /// Used to avoid emitting invalid bare `using ::name;` imports in inline modules.
    declared_item_names: HashSet<String>,
    /// Per-type method signature keys already emitted in the current type body.
    /// This catches collisions that only appear after Rust-path → C++ mapping.
    emitted_method_conflict_keys: Vec<HashSet<String>>,
    /// Stack tracking whether current function/method context returns a value.
    /// Used for tail expression lowering decisions (e.g., tail `match`).
    return_value_scopes: Vec<bool>,
    /// Optional concrete return type in current function/method context.
    /// Used to propagate expected type into tail expression lowering.
    return_type_hints: Vec<Option<syn::Type>>,
    /// Expression-local constructor template hints recovered from nearby context.
    /// Used when no explicit/typed expected Rust type exists.
    constructor_template_hints: Vec<HashMap<String, Vec<String>>>,
    /// True when emitting inside an async function body.
    /// Controls whether `return` emits `co_return` instead.
    in_async: bool,
    /// User-provided type mappings for external crate types.
    user_type_map: types::UserTypeMap,
}

impl CodeGen {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
            impl_blocks: HashMap::new(),
            impl_method_conflict_keys: HashMap::new(),
            operator_renames: HashMap::new(),
            current_struct: None,
            type_param_scopes: Vec::new(),
            enum_type_params: HashMap::new(),
            data_enum_types: HashSet::new(),
            struct_field_types: HashMap::new(),
            reassigned_vars: std::collections::HashSet::new(),
            consuming_method_receiver_vars: std::collections::HashSet::new(),
            repeat_elem_type_hints: HashMap::new(),
            local_bindings: Vec::new(),
            local_cpp_bindings: Vec::new(),
            param_bindings: Vec::new(),
            self_receiver_ref_scopes: Vec::new(),
            pattern_ref_bindings: Vec::new(),
            deref_method_scopes: Vec::new(),
            deref_mut_method_scopes: Vec::new(),
            module_name: None,
            module_stack: Vec::new(),
            skipped_module_traits: HashSet::new(),
            expanded_test_markers: Vec::new(),
            expanded_libtest_mode: false,
            emitted_top_level_functions: HashSet::new(),
            macro_rules_names: HashSet::new(),
            declared_item_names: HashSet::new(),
            emitted_method_conflict_keys: Vec::new(),
            return_value_scopes: Vec::new(),
            return_type_hints: Vec::new(),
            constructor_template_hints: Vec::new(),
            in_async: false,
            user_type_map: types::UserTypeMap::default(),
        }
    }

    /// Create a CodeGen with user-provided type mappings.
    pub fn with_type_map(type_map: types::UserTypeMap) -> Self {
        Self {
            user_type_map: type_map,
            ..Self::new()
        }
    }

    pub fn into_output(self) -> String {
        self.output
    }

    fn writeln(&mut self, s: &str) {
        self.write_indent();
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
    }

    fn newline(&mut self) {
        self.output.push('\n');
    }

    /// Emit a complete Rust file as C++ code.
    /// Uses a two-pass approach: first collects impl blocks, then emits items
    /// with methods merged into their struct definitions.
    /// If `module_name` is set, emits C++20 module declarations.
    pub fn emit_file(&mut self, file: &syn::File, module_name: Option<&str>) {
        self.module_name = module_name.map(|s| s.to_string());
        self.module_stack.clear();
        self.impl_blocks.clear();
        self.impl_method_conflict_keys.clear();
        self.operator_renames.clear();
        self.skipped_module_traits.clear();
        self.expanded_test_markers.clear();
        self.expanded_libtest_mode = false;
        self.emitted_top_level_functions.clear();
        self.macro_rules_names.clear();
        self.declared_item_names.clear();
        self.emitted_method_conflict_keys.clear();
        self.return_value_scopes.clear();
        self.return_type_hints.clear();
        self.constructor_template_hints.clear();
        self.enum_type_params.clear();
        self.data_enum_types.clear();
        self.struct_field_types.clear();
        self.reassigned_vars.clear();
        self.consuming_method_receiver_vars.clear();
        self.repeat_elem_type_hints.clear();
        self.param_bindings.clear();
        self.local_bindings.clear();
        self.local_cpp_bindings.clear();
        self.self_receiver_ref_scopes.clear();
        self.pattern_ref_bindings.clear();
        self.deref_method_scopes.clear();
        self.deref_mut_method_scopes.clear();

        // Detect expanded libtest output up front so trait emission strategy can be
        // selected consistently regardless of item ordering.
        self.expanded_libtest_mode = self.detect_expanded_libtest_mode(&file.items);

        // Pass 1a: collect globally declared item names for unresolved bare-import
        // filtering and impl target disambiguation.
        self.collect_top_level_item_names(&file.items);
        // Pass 1b: collect all impl blocks (including inline-module nested ones)
        // by scoped type name.
        self.collect_impl_blocks(&file.items, &[]);
        // Pass 1c: collect macro_rules! names so macro-only imports can be skipped.
        self.collect_macro_rules_names(&file.items);

        // Pass 2: emit all items
        self.writeln("// Auto-generated by rusty-cpp-transpiler");
        self.writeln("// Do not edit manually.");
        self.newline();

        if self.module_name.is_some() {
            // Use a global module fragment so standard/rusty headers stay in the global module.
            // This avoids named-module ODR conflicts with libstdc++ declarations.
            self.writeln("module;");
            self.newline();
        }

        self.writeln("#include <cstdint>");
        self.writeln("#include <cstddef>");
        self.writeln("#include <variant>");
        self.writeln("#include <tuple>");
        self.writeln("#include <utility>");
        self.writeln("#include <type_traits>");
        self.writeln("#include <string>");
        self.writeln("#include <string_view>");
        self.writeln("#include <charconv>");
        self.writeln("#include <cstdlib>");
        self.writeln("#include <stdexcept>");
        self.writeln("#include <rusty/rusty.hpp>");
        self.writeln("#include <rusty/try.hpp>");
        self.newline();

        if let Some(ref mod_name) = self.module_name {
            self.writeln(&format!("export module {};", mod_name));
            self.newline();
        }

        // Reserve an insertion point immediately after module declaration (or includes in non-module mode).
        // We only insert the helper if code generation actually emits `std::visit(overloaded { ... })`.
        let helper_insert_pos = self.output.len();

        for item in &file.items {
            // Skip impl blocks — they've been merged into structs
            if matches!(item, syn::Item::Impl(_)) {
                continue;
            }
            self.emit_item(item);
            self.newline();
        }

        self.emit_expanded_test_wrappers();

        let mut helper_text = String::new();
        if self.output.contains("std::visit(overloaded {") {
            helper_text.push_str(visit_overloaded_helper_text());
        }
        if self.output.contains("iterator::IterEither<") {
            helper_text.push_str(iterator_iter_either_forward_decl_text());
        }
        if needs_runtime_path_fallback_helpers(&self.output) {
            helper_text.push_str(runtime_path_fallback_helpers_text());
        }
        if !helper_text.is_empty() {
            self.output.insert_str(helper_insert_pos, &helper_text);
        }
    }

    fn emit_item(&mut self, item: &syn::Item) {
        match item {
            syn::Item::Fn(f) => self.emit_function(f),
            syn::Item::Struct(s) => self.emit_struct(s),
            syn::Item::Enum(e) => self.emit_enum(e),
            syn::Item::Type(t) => self.emit_type_alias(t),
            syn::Item::Const(c) => self.emit_const(c),
            syn::Item::Static(s) => self.emit_static(s),
            syn::Item::Impl(i) => self.emit_impl_block(i),
            syn::Item::ForeignMod(fm) => self.emit_foreign_mod(fm),
            syn::Item::Trait(t) => self.emit_trait(t),
            syn::Item::Mod(m) => self.emit_mod(m),
            syn::Item::Use(u) => self.emit_use(u),
            syn::Item::ExternCrate(ec) => {
                // extern crate foo; → no-op in C++20 modules (deps handled via import)
                self.writeln(&format!("// extern crate {}", ec.ident));
            }
            syn::Item::Macro(m) => {
                // Top-level macro invocations (macro_rules! definitions, etc.)
                if let Some(ref ident) = m.ident {
                    // macro_rules! name { ... } → compile-time only, skip
                    self.writeln(&format!("// macro_rules! {} {{ ... }}", ident));
                } else {
                    // Unnamed macro invocation at top level
                    self.emit_macro_stmt(&m.mac);
                }
            }
            _ => {
                self.writeln("// TODO: unhandled item kind");
            }
        }
    }

    fn collect_impl_blocks(&mut self, items: &[syn::Item], module_path: &[String]) {
        for item in items {
            match item {
                syn::Item::Impl(impl_block) => {
                    let Some(tp) = (match impl_block.self_ty.as_ref() {
                        syn::Type::Path(tp) => Some(tp),
                        _ => None,
                    }) else {
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
                    );

                    let op_name = impl_block.trait_.as_ref().and_then(|(_, path, _)| {
                        let trait_name = path.segments.last()?.ident.to_string();
                        map_operator_trait(&trait_name).map(|s| s.to_string())
                    });

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
                            merge_impl_type_generics_into_method(&mut merged, &impl_block.generics);
                            let key = impl_method_conflict_key(&merged);
                            if seen_method_keys.contains(&key) {
                                continue;
                            }
                            seen_method_keys.insert(key);
                            if let Some(op) = &op_name {
                                let method_name = merged.sig.ident.to_string();
                                self.operator_renames
                                    .insert((type_name.clone(), method_name), op.clone());
                            }
                            collected_item = syn::ImplItem::Fn(merged);
                        }
                        entry.push(collected_item);
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested_items)) = &m.content {
                        let mut nested_path = module_path.to_vec();
                        nested_path.push(m.ident.to_string());
                        self.collect_impl_blocks(nested_items, &nested_path);
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_macro_rules_names(&mut self, items: &[syn::Item]) {
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

    fn collect_top_level_item_names(&mut self, items: &[syn::Item]) {
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
                }
                _ => {}
            }
        }
    }

    fn scoped_type_key(&self, type_name: &str) -> String {
        if self.module_stack.is_empty() {
            type_name.to_string()
        } else {
            format!("{}::{}", self.module_stack.join("::"), type_name)
        }
    }

    fn has_impls_for_type(&self, type_name: &str) -> bool {
        let scoped = self.scoped_type_key(type_name);
        self.impl_blocks.contains_key(&scoped)
            || (scoped != type_name && self.impl_blocks.contains_key(type_name))
    }

    fn take_impls_for_type(&mut self, type_name: &str) -> Option<Vec<syn::ImplItem>> {
        let scoped = self.scoped_type_key(type_name);
        if let Some(methods) = self.impl_blocks.remove(&scoped) {
            return Some(methods);
        }
        if scoped != type_name {
            return self.impl_blocks.remove(type_name);
        }
        None
    }

    /// Returns true if visibility is pub, we're in module mode, and the item is at
    /// top-level module scope (not nested inside an inline namespace block).
    fn is_exported(&self, vis: &syn::Visibility) -> bool {
        self.module_name.is_some()
            && self.module_stack.is_empty()
            && matches!(vis, syn::Visibility::Public(_))
    }

    /// Emit doc comments from attributes as Doxygen-style `///` comments.
    fn emit_doc_comments(&mut self, attrs: &[syn::Attribute]) {
        for attr in attrs {
            if attr.path().is_ident("doc") {
                if let syn::Meta::NameValue(nv) = &attr.meta {
                    if let syn::Expr::Lit(lit) = &nv.value {
                        if let syn::Lit::Str(s) = &lit.lit {
                            let text = s.value();
                            if text.is_empty() {
                                self.writeln("///");
                            } else {
                                self.writeln(&format!("///{}", text));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Check if attributes contain `#[test]`.
    fn has_test_attr(attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|a| a.path().is_ident("test"))
    }

    /// Check if attributes contain `#[cfg(test)]`.
    fn has_cfg_test(attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|a| {
            if a.path().is_ident("cfg") {
                // Check if it's cfg(test)
                let tokens = a.meta.to_token_stream().to_string();
                tokens.contains("test")
            } else {
                false
            }
        })
    }

    fn is_rust_libtest_metadata_type(&self, ty: &syn::Type) -> bool {
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

    fn is_rust_libtest_main(&self, f: &syn::ItemFn) -> bool {
        if f.sig.ident != "main" {
            return false;
        }
        let body = normalize_token_text(f.block.to_token_stream().to_string());
        body.contains("test :: test_main_static") || body.contains("test::test_main_static")
    }

    fn rustc_test_marker_name(attrs: &[syn::Attribute]) -> Option<String> {
        for attr in attrs {
            if !attr.path().is_ident("rustc_test_marker") {
                continue;
            }
            if let syn::Meta::NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let syn::Lit::Str(s) = &expr_lit.lit {
                        return Some(s.value());
                    }
                }
            }
        }
        None
    }

    fn emit_expanded_test_wrappers(&mut self) {
        if self.expanded_test_markers.is_empty() {
            return;
        }

        self.newline();
        self.writeln("// Runnable wrappers for expanded Rust test bodies");
        let mut seen = HashSet::new();
        let markers = self.expanded_test_markers.clone();

        for marker in markers {
            if !seen.insert(marker.clone()) {
                continue;
            }
            let Some(call_target) = self.resolve_expanded_test_marker_target(&marker) else {
                self.writeln(&format!(
                    "// Rust-only libtest marker without emitted function: {}",
                    marker
                ));
                continue;
            };

            let call_name = Self::escape_cpp_qualified_name(&call_target);
            let wrapper_name = format!("rusty_test_{}", Self::marker_wrapper_suffix(&marker));
            let export_prefix = if self.module_name.is_some() {
                "export "
            } else {
                ""
            };
            self.writeln(&format!("{}void {}() {{", export_prefix, wrapper_name));
            self.indent += 1;
            self.writeln(&format!("{}();", call_name));
            self.indent -= 1;
            self.writeln("}");
        }
    }

    fn resolve_expanded_test_marker_target(&self, marker: &str) -> Option<String> {
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

    fn marker_wrapper_suffix(marker: &str) -> String {
        marker
            .replace("::", "_")
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    fn escape_cpp_qualified_name(path: &str) -> String {
        path.split("::")
            .map(escape_cpp_keyword)
            .collect::<Vec<_>>()
            .join("::")
    }

    fn has_rustc_test_marker_attr(attrs: &[syn::Attribute]) -> bool {
        attrs
            .iter()
            .any(|attr| attr.path().is_ident("rustc_test_marker"))
    }

    fn detect_expanded_libtest_mode(&self, items: &[syn::Item]) -> bool {
        for item in items {
            match item {
                syn::Item::Const(c) => {
                    if self.is_rust_libtest_metadata_type(&c.ty)
                        || Self::has_rustc_test_marker_attr(&c.attrs)
                    {
                        return true;
                    }
                }
                syn::Item::Static(s) => {
                    if self.is_rust_libtest_metadata_type(&s.ty) {
                        return true;
                    }
                }
                syn::Item::Fn(f) => {
                    if self.is_rust_libtest_main(f) {
                        return true;
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested_items)) = &m.content {
                        if self.detect_expanded_libtest_mode(nested_items) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Emit a nested function definition as a C++ lambda.
    /// `fn foo(x: i32) -> i32 { x + 1 }` → `const auto foo = [&](int32_t x) -> int32_t { return x + 1; };`
    fn emit_nested_function(&mut self, f: &syn::ItemFn) {
        let name = escape_cpp_keyword(&f.sig.ident.to_string());
        let return_type = self.map_return_type(&f.sig.output);
        let params = self.map_fn_params(&f.sig.inputs);

        let ret_annotation = if return_type == "void" {
            String::new()
        } else {
            format!(" -> {}", return_type)
        };

        self.writeln(&format!(
            "const auto {} = [&]({}){} {{",
            name, params, ret_annotation
        ));
        self.indent += 1;
        self.push_return_value_scope(&return_type);
        self.push_return_type_hint(&f.sig.output);
        self.emit_block(&f.block);
        self.pop_return_type_hint();
        self.pop_return_value_scope();
        self.indent -= 1;
        self.writeln("};");
    }

    fn emit_function(&mut self, f: &syn::ItemFn) {
        // Skip #[cfg(test)] functions in non-test output
        // (they'll be emitted separately as test cases)

        if self.is_rust_libtest_main(f) {
            self.writeln("// Rust-only libtest main omitted");
            return;
        }

        let fn_name = f.sig.ident.to_string();
        let scoped_name = if self.module_stack.is_empty() {
            fn_name
        } else {
            format!("{}::{}", self.module_stack.join("::"), fn_name)
        };
        self.emitted_top_level_functions.insert(scoped_name);

        let is_test = Self::has_test_attr(&f.attrs);

        // Emit doc comments
        self.emit_doc_comments(&f.attrs);

        // Emit @unsafe annotation for unsafe functions
        if f.sig.unsafety.is_some() {
            self.writeln("// @unsafe");
        }

        let name = escape_cpp_keyword(&f.sig.ident.to_string());
        let is_async = f.sig.asyncness.is_some();
        // Emit template prefix if generic
        self.emit_template_prefix(&f.sig.generics);
        self.push_type_param_scope(&f.sig.generics);
        let return_type = self.map_return_type(&f.sig.output);
        let params = self.map_fn_params(&f.sig.inputs);

        // Wrap return type in Task<> for async functions
        let return_type = if is_async {
            format!("rusty::Task<{}>", return_type)
        } else {
            return_type
        };

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

        let export_prefix = if self.is_exported(&f.vis) {
            "export "
        } else {
            ""
        };
        self.writeln(&format!(
            "{}{}{} {}({}) {{",
            export_prefix, abi_prefix, return_type, name, params
        ));
        self.indent += 1;

        // Async functions use co_return instead of return
        let prev_async = self.in_async;
        self.in_async = is_async;
        self.push_return_value_scope(&return_type);
        self.push_return_type_hint(&f.sig.output);
        self.push_param_bindings(&f.sig.inputs);
        self.emit_block(&f.block);
        self.pop_param_bindings();
        self.pop_return_type_hint();
        self.pop_return_value_scope();
        self.in_async = prev_async;

        self.indent -= 1;
        self.writeln("}");
        self.pop_type_param_scope();
    }

    fn emit_foreign_mod(&mut self, fm: &syn::ItemForeignMod) {
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

    fn emit_struct(&mut self, s: &syn::ItemStruct) {
        let name = &s.ident;
        let name_str = name.to_string();

        // Emit doc comments
        self.emit_doc_comments(&s.attrs);

        self.emit_template_prefix(&s.generics);
        self.push_type_param_scope(&s.generics);
        let export_prefix = if self.is_exported(&s.vis) {
            "export "
        } else {
            ""
        };
        self.writeln(&format!("{}struct {} {{", export_prefix, name));
        self.indent += 1;

        // Emit fields
        let mut named_field_types: HashMap<String, syn::Type> = HashMap::new();
        match &s.fields {
            syn::Fields::Named(fields) => {
                for field in &fields.named {
                    // Emit field doc comments
                    self.emit_doc_comments(&field.attrs);
                    let field_name = field.ident.as_ref().unwrap();
                    let field_type = self.map_type(&field.ty);
                    self.writeln(&format!("{} {};", field_type, field_name));
                    named_field_types.insert(field_name.to_string(), field.ty.clone());
                }
            }
            syn::Fields::Unnamed(fields) => {
                for (i, field) in fields.unnamed.iter().enumerate() {
                    let field_type = self.map_type(&field.ty);
                    self.writeln(&format!("{} _{};", field_type, i));
                }
            }
            syn::Fields::Unit => {}
        }
        if !named_field_types.is_empty() {
            self.struct_field_types
                .insert(name_str.clone(), named_field_types.clone());
            let scoped_name = self.scoped_type_key(&name_str);
            self.struct_field_types
                .insert(scoped_name, named_field_types);
        }

        // Emit methods from impl blocks (merged)
        if let Some(methods) = self.take_impls_for_type(&name_str) {
            if !matches!(&s.fields, syn::Fields::Unit if methods.is_empty()) {
                self.newline();
            }
            self.current_struct = Some(name_str.clone());
            self.emitted_method_conflict_keys.push(HashSet::new());
            for impl_item in &methods {
                self.emit_impl_item(impl_item);
            }
            self.emitted_method_conflict_keys.pop();
            self.current_struct = None;
        }

        // Emit derive-generated code
        let derives = self.extract_derives(&s.attrs);
        if !derives.is_empty() {
            self.newline();
        }
        for derive in &derives {
            match derive.as_str() {
                "Clone" => {
                    // Clone → copy constructor (C++ default) + explicit clone() method
                    self.writeln(&format!("{} clone() const {{ return *this; }}", name));
                }
                "PartialEq" | "Eq" => {
                    self.writeln("auto operator==(const auto&) const = default;");
                }
                "PartialOrd" | "Ord" => {
                    self.writeln("auto operator<=>(const auto&) const = default;");
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

        self.indent -= 1;
        self.writeln("};");

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

    /// Extract derive trait names from attributes.
    fn extract_derives(&self, attrs: &[syn::Attribute]) -> Vec<String> {
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

    fn emit_enum(&mut self, e: &syn::ItemEnum) {
        let name = &e.ident;
        let has_data = e.variants.iter().any(|v| !v.fields.is_empty());
        if has_data {
            self.data_enum_types.insert(name.to_string());
        } else {
            self.data_enum_types.remove(&name.to_string());
        }
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
                match &variant.fields {
                    syn::Fields::Named(fields) => {
                        if has_generics {
                            self.writeln(&template_prefix);
                        }
                        self.writeln(&format!("struct {}_{} {{", name, vname));
                        self.indent += 1;
                        for field in &fields.named {
                            let fname = field.ident.as_ref().unwrap();
                            let ftype = self.map_type(&field.ty);
                            self.writeln(&format!("{} {};", ftype, fname));
                        }
                        self.indent -= 1;
                        self.writeln("};");
                    }
                    syn::Fields::Unnamed(fields) => {
                        if has_generics {
                            self.writeln(&template_prefix);
                        }
                        self.writeln(&format!("struct {}_{} {{", name, vname));
                        self.indent += 1;
                        for (i, field) in fields.unnamed.iter().enumerate() {
                            let ftype = self.map_type(&field.ty);
                            self.writeln(&format!("{} _{};", ftype, i));
                        }
                        self.indent -= 1;
                        self.writeln("};");
                    }
                    syn::Fields::Unit => {
                        if has_generics {
                            self.writeln(&template_prefix);
                        }
                        self.writeln(&format!("struct {}_{} {{}};", name, vname));
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
            for variant in &e.variants {
                let vname = &variant.ident;
                let variant_struct = if has_generics {
                    format!("{}_{}{}", name, vname, template_args)
                } else {
                    format!("{}_{}", name, vname)
                };
                match &variant.fields {
                    syn::Fields::Unnamed(fields) => {
                        let params: Vec<String> = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(i, f)| {
                                let ty = self.map_type(&f.ty);
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
                        let params: Vec<String> = fields
                            .named
                            .iter()
                            .map(|f| {
                                let fname = f.ident.as_ref().unwrap();
                                let ftype = self.map_type(&f.ty);
                                format!("{} {}", ftype, fname)
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

            // Use struct wrapper if recursive OR has impl blocks (so methods can be added)
            let has_impls = self.has_impls_for_type(&name.to_string());
            if is_recursive || has_impls {
                let variant_type = format!("std::variant<{}>", variant_list.join(", "));
                if has_generics {
                    self.writeln(&template_prefix);
                }
                self.writeln(&format!("struct {} : {} {{", name, variant_type));
                self.indent += 1;
                self.writeln(&format!("using variant = {};", variant_type));
                self.writeln("using variant::variant;");

                // Merge impl block methods into the enum struct
                if let Some(methods) = self.take_impls_for_type(&name.to_string()) {
                    self.newline();
                    self.current_struct = Some(name.to_string());
                    self.emitted_method_conflict_keys.push(HashSet::new());
                    for impl_item in &methods {
                        self.emit_impl_item(impl_item);
                    }
                    self.emitted_method_conflict_keys.pop();
                    self.current_struct = None;
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
                let variant_struct = if has_generics {
                    format!("{}_{}{}", name, vname, template_args)
                } else {
                    format!("{}_{}", name, vname)
                };

                match &variant.fields {
                    syn::Fields::Unnamed(fields) => {
                        let params: Vec<String> = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(i, f)| {
                                let ty = self.map_type(&f.ty);
                                format!("{} _{}", ty, i)
                            })
                            .collect();
                        let args: Vec<String> = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(i, f)| {
                                let ty = self.map_type(&f.ty);
                                let param = format!("_{}", i);
                                format!("std::forward<{}>({})", ty, param)
                            })
                            .collect();
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
                    syn::Fields::Named(fields) => {
                        let params: Vec<String> = fields
                            .named
                            .iter()
                            .map(|f| {
                                let fname = f.ident.as_ref().unwrap();
                                let ftype = self.map_type(&f.ty);
                                format!("{} {}", ftype, fname)
                            })
                            .collect();
                        let args: Vec<String> = fields
                            .named
                            .iter()
                            .map(|f| {
                                let fname = f.ident.as_ref().unwrap();
                                let ftype = self.map_type(&f.ty);
                                format!(".{} = std::forward<{}>({})", fname, ftype, fname)
                            })
                            .collect();
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
                    syn::Fields::Unit => {
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
        } else {
            // C-like enum → enum class
            self.writeln(&format!("enum class {} {{", name));
            self.indent += 1;
            let variants: Vec<String> = e.variants.iter().map(|v| v.ident.to_string()).collect();
            self.writeln(&variants.join(",\n    "));
            self.indent -= 1;
            self.writeln("};");
        }
        self.pop_type_param_scope();
    }

    /// Check if a variant's fields reference a given type name (for recursion detection).
    fn variant_references_type(&self, variant: &syn::Variant, type_name: &str) -> bool {
        match &variant.fields {
            syn::Fields::Named(fields) => fields
                .named
                .iter()
                .any(|f| self.type_references_name(&f.ty, type_name)),
            syn::Fields::Unnamed(fields) => fields
                .unnamed
                .iter()
                .any(|f| self.type_references_name(&f.ty, type_name)),
            syn::Fields::Unit => false,
        }
    }

    /// Check if a type references a given name (recursively through generics).
    fn type_references_name(&self, ty: &syn::Type, name: &str) -> bool {
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
            _ => false,
        }
    }

    fn emit_type_alias(&mut self, t: &syn::ItemType) {
        let name = &t.ident;
        let target = self.map_type(&t.ty);
        self.writeln(&format!("using {} = {};", name, target));
    }

    fn emit_const(&mut self, c: &syn::ItemConst) {
        if self.is_rust_libtest_metadata_type(&c.ty) {
            let marker =
                Self::rustc_test_marker_name(&c.attrs).unwrap_or_else(|| c.ident.to_string());
            self.expanded_test_markers.push(marker);
            self.writeln(&format!(
                "// Rust-only libtest metadata const skipped: {}",
                c.ident
            ));
            return;
        }
        let name = &c.ident;
        let ty = self.map_type(&c.ty);
        let expr = self.emit_expr_to_string(&c.expr);
        self.writeln(&format!("constexpr {} {} = {};", ty, name, expr));
    }

    fn emit_static(&mut self, s: &syn::ItemStatic) {
        if self.is_rust_libtest_metadata_type(&s.ty) {
            self.writeln(&format!(
                "// Rust-only libtest metadata static skipped: {}",
                s.ident
            ));
            return;
        }
        let name = &s.ident;
        let ty = self.map_type(&s.ty);
        let expr = self.emit_expr_to_string(&s.expr);
        self.writeln(&format!("static {} {} = {};", ty, name, expr));
    }

    fn emit_trait(&mut self, t: &syn::ItemTrait) {
        let trait_name = &t.ident;
        let scoped = self.scoped_type_key(&trait_name.to_string());

        // Expanded crate output in module mode commonly lacks Proxy runtime wiring.
        // Guard by skipping trait-facade emission there to avoid unresolved `pro::*` symbols.
        if self.module_name.is_some() {
            self.skipped_module_traits.insert(scoped);
            self.writeln(&format!(
                "// Rust-only trait {} (Proxy facade emission skipped in module mode)",
                trait_name
            ));
            return;
        }
        if self.expanded_libtest_mode {
            self.skipped_module_traits.insert(scoped);
            self.writeln(&format!(
                "// Rust-only trait {} (Proxy facade emission skipped in expanded-test mode)",
                trait_name
            ));
            return;
        }

        // Check for known marker traits — emit as concepts
        let trait_name_str = trait_name.to_string();
        if matches!(
            trait_name_str.as_str(),
            "Send" | "Sync" | "Copy" | "Clone" | "Sized" | "Unpin"
        ) {
            self.writeln(&format!("// Marker trait: {}", trait_name));
            self.writeln(&format!("template<typename T>"));
            self.writeln(&format!(
                "concept {} = true;  // marker trait — no runtime check",
                trait_name
            ));
            return;
        }

        // Collect supertrait names
        let supertraits: Vec<String> = t
            .supertraits
            .iter()
            .filter_map(|b| {
                if let syn::TypeParamBound::Trait(tb) = b {
                    Some(tb.path.segments.last()?.ident.to_string())
                } else {
                    None
                }
            })
            .collect();

        if !supertraits.is_empty() {
            let supers = supertraits
                .iter()
                .map(|s| format!("{}Facade", s))
                .collect::<Vec<_>>()
                .join(", ");
            self.writeln(&format!("// Requires: {}", supers));
        }

        // Step 1: Emit PRO_DEF_MEM_DISPATCH for each method
        let mut methods: Vec<(String, String, String, bool)> = Vec::new(); // (dispatch_name, method_name, signature, is_const)

        for item in &t.items {
            if let syn::TraitItem::Fn(method) = item {
                let method_name = method.sig.ident.to_string();
                let escaped_name = escape_cpp_keyword(&method_name);
                let dispatch_name = format!("Mem{}_{}", trait_name, escaped_name);

                // Determine if const (from &self)
                let is_const = matches!(
                    method.sig.inputs.first(),
                    Some(syn::FnArg::Receiver(r)) if r.reference.is_some() && r.mutability.is_none()
                );

                // Build the signature: return_type(param_types...) [const]
                let return_type = self.map_return_type(&method.sig.output);
                let param_types: Vec<String> = method
                    .sig
                    .inputs
                    .iter()
                    .filter_map(|arg| match arg {
                        syn::FnArg::Receiver(_) => None,
                        syn::FnArg::Typed(pt) => Some(self.map_type(&pt.ty)),
                    })
                    .collect();

                let sig = if param_types.is_empty() {
                    format!("{}()", return_type)
                } else {
                    format!("{}({})", return_type, param_types.join(", "))
                };

                self.writeln(&format!(
                    "PRO_DEF_MEM_DISPATCH(Mem{}_{}, {});",
                    trait_name, escaped_name, escaped_name
                ));

                methods.push((dispatch_name, escaped_name, sig, is_const));
            }
        }

        self.newline();

        // Step 2: Emit facade_builder struct
        self.writeln(&format!(
            "struct {}Facade : pro::facade_builder",
            trait_name
        ));
        self.indent += 1;
        for (i, (dispatch_name, _, sig, is_const)) in methods.iter().enumerate() {
            let const_suffix = if *is_const { " const" } else { "" };
            if i == methods.len() - 1 {
                self.writeln(&format!(
                    "::add_convention<{}, {}{}>",
                    dispatch_name, sig, const_suffix
                ));
                self.writeln("::build {};");
            } else {
                self.writeln(&format!(
                    "::add_convention<{}, {}{}>",
                    dispatch_name, sig, const_suffix
                ));
            }
        }
        if methods.is_empty() {
            self.writeln("::build {};");
        }
        self.indent -= 1;

        // Step 3: Emit default method implementations as free functions
        for item in &t.items {
            if let syn::TraitItem::Fn(method) = item {
                if let Some(default_body) = &method.default {
                    let method_name = escape_cpp_keyword(&method.sig.ident.to_string());
                    let return_type = self.map_return_type(&method.sig.output);

                    // Build params (replacing self with proxy_view)
                    let mut params = vec![format!("pro::proxy_view<{}Facade> _self", trait_name)];
                    for arg in &method.sig.inputs {
                        if let syn::FnArg::Typed(pt) = arg {
                            let ty = self.map_type(&pt.ty);
                            let name = match pt.pat.as_ref() {
                                syn::Pat::Ident(pi) => pi.ident.to_string(),
                                _ => "_".to_string(),
                            };
                            params.push(format!("{} {}", ty, name));
                        }
                    }

                    self.newline();
                    self.writeln(&format!(
                        "{} {}({}) {{",
                        return_type,
                        method_name,
                        params.join(", ")
                    ));
                    self.indent += 1;
                    self.push_return_value_scope(&return_type);
                    self.push_return_type_hint(&method.sig.output);
                    self.emit_block(default_body);
                    self.pop_return_type_hint();
                    self.pop_return_value_scope();
                    self.indent -= 1;
                    self.writeln("}");
                }
            }
        }
    }

    fn emit_mod(&mut self, m: &syn::ItemMod) {
        // Skip #[cfg(test)] modules — test code is not transpiled into production output
        if Self::has_cfg_test(&m.attrs) {
            self.writeln("// #[cfg(test)] module omitted");
            return;
        }

        let mod_name = &m.ident;
        let is_pub = matches!(m.vis, syn::Visibility::Public(_));
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
            self.writeln(&format!("namespace {} {{", mod_name));
            self.indent += 1;
            self.module_stack.push(mod_name.to_string());
            for item in items {
                if matches!(item, syn::Item::Impl(_)) {
                    continue;
                }
                self.emit_item(item);
                self.newline();
            }
            self.module_stack.pop();
            self.indent -= 1;
            self.writeln("}");
        }
    }

    fn emit_use(&mut self, u: &syn::ItemUse) {
        let is_pub = matches!(u.vis, syn::Visibility::Public(_));

        // Detect external crate imports
        let root_ident = self.get_use_root(&u.tree);
        let is_external = !matches!(
            root_ident.as_str(),
            "crate" | "self" | "super" | "std" | "core" | "alloc"
        ) && root_ident.chars().next().is_some_and(|c| c.is_lowercase());

        if is_external {
            self.writeln(&format!(
                "// TODO: external crate '{}' — provide type mapping or transpile dependency",
                root_ident
            ));
        }

        // Flatten group imports into separate using declarations
        let paths = self.flatten_use_tree(&u.tree, "");

        let export_prefix = if is_pub && self.module_name.is_some() {
            "export "
        } else {
            ""
        };
        for path in &paths {
            if is_pub
                && self.module_name.is_some()
                && is_module_linkage_sensitive_reexport(normalize_use_import_path(path))
            {
                self.writeln(&format!("// Rust-only: using {};", path));
                continue;
            }
            if self.is_skipped_module_trait_import(path) {
                self.writeln(&format!("// Rust-only: using {};", path));
                continue;
            }
            if self.is_macro_rules_import(path) {
                self.writeln(&format!("// Rust-only macro import: using {};", path));
                continue;
            }
            if self.should_skip_unresolved_bare_import(path) {
                self.writeln(&format!("// Rust-only unresolved import: using {};", path));
                continue;
            }
            match classify_use_import(path) {
                UseImportAction::RustOnly => {
                    self.writeln(&format!("// Rust-only: using {};", path));
                }
                UseImportAction::Using(mapped_path) => {
                    let using_path = make_using_path_cpp_legal(&mapped_path);
                    self.writeln(&format!("{}using {};", export_prefix, using_path));
                }
                UseImportAction::Raw(statement) => {
                    self.writeln(&format!("{}{}", export_prefix, statement));
                }
            }
        }
    }

    fn is_skipped_module_trait_import(&self, path: &str) -> bool {
        let normalized = normalize_use_import_path(path);
        self.skipped_module_traits.contains(normalized)
    }

    fn is_macro_rules_import(&self, path: &str) -> bool {
        let normalized = normalize_use_import_path(path);
        let last = normalized.split("::").last().unwrap_or(normalized);
        self.macro_rules_names.contains(last)
    }

    fn should_skip_unresolved_bare_import(&self, path: &str) -> bool {
        if self.module_stack.is_empty() {
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

    fn emitted_method_conflict_key(
        &self,
        method_name: &str,
        emitted_template_key: &str,
        qualifier: &str,
        is_static: bool,
        params: &[String],
    ) -> String {
        let static_key = if is_static { "static" } else { "instance" };
        let qualifier_key = qualifier.trim();
        format!(
            "{}|{}|{}|{}|{}",
            method_name,
            static_key,
            qualifier_key,
            emitted_template_key,
            params.join(",")
        )
    }

    fn collect_emitted_template_parts(
        &self,
        generics: &syn::Generics,
    ) -> (Vec<String>, Vec<String>) {
        let type_params: Vec<&syn::TypeParam> = generics
            .params
            .iter()
            .filter_map(|p| {
                if let syn::GenericParam::Type(tp) = p {
                    Some(tp)
                } else {
                    None
                }
            })
            .filter(|tp| !self.is_type_param_in_scope(&tp.ident.to_string()))
            .collect();

        if type_params.is_empty() {
            return (Vec::new(), Vec::new());
        }

        let params_str: Vec<String> = type_params.iter().map(|tp| tp.ident.to_string()).collect();

        let mut constraints: Vec<String> = Vec::new();
        let skip_facade_constraints = self.module_name.is_some();

        for tp in &type_params {
            for bound in &tp.bounds {
                if let syn::TypeParamBound::Trait(tb) = bound {
                    if skip_facade_constraints {
                        continue;
                    }
                    if let Some(facade_name) = facade_name_for_trait_path(&tb.path) {
                        constraints
                            .push(format!("{}::is_satisfied_by<{}>()", facade_name, tp.ident));
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
                            if skip_facade_constraints {
                                continue;
                            }
                            if let Some(facade_name) = facade_name_for_trait_path(&tb.path) {
                                constraints.push(format!(
                                    "{}::is_satisfied_by<{}>()",
                                    facade_name, ty_name
                                ));
                            }
                        }
                    }
                }
            }
        }

        (params_str, constraints)
    }

    fn emitted_template_signature_key(&self, generics: &syn::Generics) -> String {
        let (params, constraints) = self.collect_emitted_template_parts(generics);
        if params.is_empty() {
            return String::new();
        }
        if constraints.is_empty() {
            return params.join(",");
        }
        format!("{}|{}", params.join(","), constraints.join(" && "))
    }

    /// Returns true if key is first-seen in current type scope, false if duplicate.
    fn mark_emitted_method_conflict_key(&mut self, key: String) -> bool {
        if let Some(scope) = self.emitted_method_conflict_keys.last_mut() {
            scope.insert(key)
        } else {
            true
        }
    }

    /// Flatten a use tree into a list of fully-qualified C++ paths.
    /// Handles groups by expanding each item with the parent prefix.
    fn flatten_use_tree(&self, tree: &syn::UseTree, prefix: &str) -> Vec<String> {
        match tree {
            syn::UseTree::Path(p) => {
                let ident = p.ident.to_string();

                // Map path prefixes (crate::, self::, core::, etc.)
                let mapped = match ident.as_str() {
                    "crate" => {
                        // `crate::` paths refer to the current Rust module tree.
                        // In C++ module output, module names are not C++ namespaces,
                        // so keep these as local namespace paths (same as `self::`).
                        return self.flatten_use_tree(&p.tree, prefix);
                    }
                    "self" => {
                        if self.module_stack.is_empty() {
                            return self.flatten_use_tree(&p.tree, prefix);
                        }
                        let module_prefix = self.module_stack.join("::");
                        let new_prefix = if prefix.is_empty() {
                            module_prefix
                        } else {
                            format!("{}::{}", prefix, module_prefix)
                        };
                        return self.flatten_use_tree(&p.tree, &new_prefix);
                    }
                    "super" => {
                        if self.module_stack.len() > 1 {
                            self.module_stack[..self.module_stack.len() - 1].join("::")
                        } else {
                            return self.flatten_use_tree(&p.tree, prefix);
                        }
                    }
                    "std" | "core" | "alloc" => "std".to_string(),
                    _ => ident,
                };

                let new_prefix = if prefix.is_empty() {
                    mapped
                } else {
                    format!("{}::{}", prefix, mapped)
                };

                self.flatten_use_tree(&p.tree, &new_prefix)
            }
            syn::UseTree::Name(n) => {
                let full = if prefix.is_empty() {
                    n.ident.to_string()
                } else {
                    format!("{}::{}", prefix, n.ident)
                };
                vec![full]
            }
            syn::UseTree::Rename(r) => {
                let full = if prefix.is_empty() {
                    format!("{} = {}", r.rename, r.ident)
                } else {
                    format!("{}::{} = {}::{}", prefix, r.rename, prefix, r.ident)
                };
                vec![full]
            }
            syn::UseTree::Glob(_) => {
                // `use foo::*` → just use the namespace
                vec![format!("namespace {}", prefix)]
            }
            syn::UseTree::Group(g) => {
                // Expand each item in the group with the current prefix
                let mut result = Vec::new();
                for item in &g.items {
                    match item {
                        syn::UseTree::Name(n) if n.ident == "self" => {
                            // `use foo::{self}` → `using foo`
                            if !prefix.is_empty() {
                                result.push(prefix.to_string());
                            }
                        }
                        _ => {
                            result.extend(self.flatten_use_tree(item, prefix));
                        }
                    }
                }
                result
            }
        }
    }

    /// Get the root identifier of a use tree (first path segment).
    fn get_use_root(&self, tree: &syn::UseTree) -> String {
        match tree {
            syn::UseTree::Path(p) => p.ident.to_string(),
            syn::UseTree::Name(n) => n.ident.to_string(),
            syn::UseTree::Rename(r) => r.ident.to_string(),
            syn::UseTree::Group(g) => g
                .items
                .first()
                .map_or(String::new(), |t| self.get_use_root(t)),
            syn::UseTree::Glob(_) => "*".to_string(),
        }
    }

    fn emit_impl_block(&mut self, i: &syn::ItemImpl) {
        // This is called for impl blocks whose struct wasn't found in the same file.
        // Emit methods as free-standing functions (fallback).
        let type_name = if let syn::Type::Path(tp) = i.self_ty.as_ref() {
            tp.path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::")
        } else {
            "UnknownType".to_string()
        };

        self.writeln(&format!("// Methods for {}", type_name));
        for item in &i.items {
            self.emit_impl_item(item);
        }
    }

    fn emit_impl_item(&mut self, item: &syn::ImplItem) {
        match item {
            syn::ImplItem::Fn(method) => self.emit_method(method),
            syn::ImplItem::Const(c) => {
                let name = &c.ident;
                let ty = self.map_type(&c.ty);
                let expr = self.emit_expr_to_string(&c.expr);
                self.writeln(&format!("static constexpr {} {} = {};", ty, name, expr));
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
                let name = &t.ident;
                let ty = self.map_type(&t.ty);
                self.writeln(&format!("using {} = {};", name, ty));
            }
            _ => {
                self.writeln("// TODO: unhandled impl item");
            }
        }
    }

    fn emit_method(&mut self, method: &syn::ImplItemFn) {
        let method_ident = method.sig.ident.to_string();
        let emitted_template_key = self.emitted_template_signature_key(&method.sig.generics);

        // Emit template prefix for generic methods (excluding names already
        // bound by outer type/impl scopes to avoid shadowing).
        self.emit_template_prefix(&method.sig.generics);
        self.push_type_param_scope(&method.sig.generics);

        // Check if this method is an operator trait impl (renamed)
        let name = if let Some(ref struct_name) = self.current_struct {
            if let Some(op) = self
                .operator_renames
                .get(&(struct_name.clone(), method_ident.clone()))
                .or_else(|| {
                    let scoped_name = self.scoped_type_key(struct_name);
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

        let mut return_type = self.map_return_type(&method.sig.output);
        // In constrained emission modes (named module output and expanded tests),
        // merged trait impls can surface unconstrained associated-type returns
        // (e.g. `L::IntoIter`, `Self::Output`, `Either::Item`) that hard-fail when
        // instantiating concrete `Either<int,int>`.
        // Falling back to `auto` keeps these methods lazily checked at use sites.
        if self.should_soften_dependent_assoc_mode()
            && (self.return_type_contains_dependent_assoc(&method.sig.output)
                || self.return_type_references_current_struct_assoc(&method.sig.output))
        {
            return_type = "auto".to_string();
        }

        // Analyze receiver to determine method kind
        let (qualifier, is_static) = match method.sig.inputs.first() {
            Some(syn::FnArg::Receiver(recv)) => {
                if recv.reference.is_some() {
                    if recv.mutability.is_some() {
                        // &mut self → non-const method
                        ("", false)
                    } else {
                        // &self → const method
                        (" const", false)
                    }
                } else {
                    // self (by value) → non-const method (consumes)
                    ("", false)
                }
            }
            _ => {
                // No self → static method
                ("", true)
            }
        };

        // Build params list (skip self receiver)
        let params: Vec<String> = method
            .sig
            .inputs
            .iter()
            .filter_map(|arg| match arg {
                syn::FnArg::Receiver(_) => None,
                syn::FnArg::Typed(pat_type) => {
                    let ty = self.map_type(&pat_type.ty);
                    let param_name = match pat_type.pat.as_ref() {
                        syn::Pat::Ident(pi) => pi.ident.to_string(),
                        _ => "_".to_string(),
                    };
                    Some(format!("{} {}", ty, param_name))
                }
            })
            .collect();

        let static_prefix = if is_static { "static " } else { "" };
        let conflict_key = self.emitted_method_conflict_key(
            &name,
            &emitted_template_key,
            qualifier,
            is_static,
            &params,
        );
        if !self.mark_emitted_method_conflict_key(conflict_key) {
            self.pop_type_param_scope();
            return;
        }
        self.writeln(&format!(
            "{}{} {}({}){} {{",
            static_prefix,
            return_type,
            name,
            params.join(", "),
            qualifier
        ));
        self.indent += 1;
        self.push_return_value_scope(&return_type);
        self.push_return_type_hint(&method.sig.output);
        self.push_param_bindings(&method.sig.inputs);
        self.push_self_receiver_ref_scope(&method.sig.inputs);
        self.push_deref_method_scope(is_deref_method);
        self.push_deref_mut_method_scope(is_deref_mut_method);
        self.emit_block(&method.block);
        self.pop_deref_mut_method_scope();
        self.pop_deref_method_scope();
        self.pop_self_receiver_ref_scope();
        self.pop_param_bindings();
        self.pop_return_type_hint();
        self.pop_return_value_scope();
        self.indent -= 1;
        self.writeln("}");
        self.pop_type_param_scope();
    }

    fn emit_block(&mut self, block: &syn::Block) {
        // Pre-scan: find variables that are reassigned (for reference rebinding detection)
        let reassigned = collect_reassigned_vars(&block.stmts);
        let consuming = collect_consuming_method_receiver_vars(&block.stmts);
        let repeat_hints = collect_repeat_element_type_hints(&block.stmts);
        let prev = std::mem::replace(&mut self.reassigned_vars, reassigned);
        let prev_consuming = std::mem::replace(&mut self.consuming_method_receiver_vars, consuming);
        let prev_repeat_hints = std::mem::replace(&mut self.repeat_elem_type_hints, repeat_hints);
        self.local_bindings.push(HashMap::new());
        self.local_cpp_bindings.push(HashMap::new());

        let stmts = &block.stmts;
        let len = stmts.len();

        for (i, stmt) in stmts.iter().enumerate() {
            let is_last = i == len - 1;
            self.emit_stmt(stmt, is_last);
        }

        self.local_bindings.pop();
        self.local_cpp_bindings.pop();
        self.reassigned_vars = prev;
        self.consuming_method_receiver_vars = prev_consuming;
        self.repeat_elem_type_hints = prev_repeat_hints;
    }

    fn emit_stmt(&mut self, stmt: &syn::Stmt, is_tail: bool) {
        match stmt {
            syn::Stmt::Local(local) => self.emit_local(local),
            syn::Stmt::Expr(expr, semi) => {
                // Tail `match` expressions in non-void functions must stay in
                // expression-lowering path so we emit `return <iife>;` instead
                // of a statement-level `std::visit(...)` with fallthrough.
                let force_expr_path = self.in_value_return_scope()
                    && is_tail
                    && semi.is_none()
                    && matches!(expr, syn::Expr::Match(_));
                // Control flow expressions are emitted as statements directly
                if !force_expr_path && self.try_emit_control_flow(expr) {
                    return;
                }
                let expr_str = if is_tail && semi.is_none() {
                    self.emit_expr_to_string_with_expected(expr, self.current_return_type_hint())
                } else {
                    self.emit_expr_to_string(expr)
                };
                if is_tail && semi.is_none() {
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
                    let return_ty = match &f.sig.output {
                        syn::ReturnType::Type(_, ty) => Some((**ty).clone()),
                        syn::ReturnType::Default => None,
                    };
                    self.register_local_binding(f.sig.ident.to_string(), return_ty);
                    self.emit_nested_function(f);
                } else {
                    self.emit_item(item);
                }
            }
            syn::Stmt::Macro(stmt_macro) => {
                self.emit_macro_stmt(&stmt_macro.mac);
            }
        }
    }

    /// Try to emit an expression as a control flow statement.
    /// Returns true if it was handled, false if it should go through emit_expr_to_string.
    fn try_emit_control_flow(&mut self, expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::If(if_expr) => {
                self.emit_if(if_expr);
                true
            }
            syn::Expr::While(while_expr) => {
                self.emit_while(while_expr);
                true
            }
            syn::Expr::Loop(loop_expr) => {
                self.emit_loop(loop_expr);
                true
            }
            syn::Expr::ForLoop(for_expr) => {
                self.emit_for_loop(for_expr);
                true
            }
            syn::Expr::Block(block_expr) => {
                self.writeln("{");
                self.indent += 1;
                self.emit_block(&block_expr.block);
                self.indent -= 1;
                self.writeln("}");
                true
            }
            syn::Expr::Unsafe(unsafe_expr) => {
                self.writeln("// @unsafe");
                self.writeln("{");
                self.indent += 1;
                self.emit_block(&unsafe_expr.block);
                self.indent -= 1;
                self.writeln("}");
                true
            }
            syn::Expr::Match(match_expr) => {
                self.emit_match(match_expr);
                true
            }
            _ => false,
        }
    }

    fn emit_match(&mut self, match_expr: &syn::ExprMatch) {
        // Expanded test assertions commonly lower to tuple-binding statement matches:
        // `match (&a, &b) { (left_val, right_val) => ... }`.
        // These are pure bindings and do not need variant visitation.
        if self.try_emit_binding_tuple_match(match_expr) {
            return;
        }

        let scrutinee = self.emit_expr_to_string(&match_expr.expr);
        let variant_ctx = self.infer_variant_type_context_from_expr(&match_expr.expr);

        // Decide strategy: switch-compatible value patterns vs std::visit variant dispatch.
        if self.all_arms_are_switch_compatible(&match_expr.arms, variant_ctx.as_ref()) {
            self.emit_match_as_switch(&scrutinee, &match_expr.arms);
        } else {
            self.emit_match_as_visit(&scrutinee, &match_expr.arms, variant_ctx.as_ref());
        }
    }

    /// Check if all match arms can be lowered to value-based switch/if matching.
    fn all_arms_are_switch_compatible(
        &self,
        arms: &[syn::Arm],
        variant_ctx: Option<&VariantTypeContext>,
    ) -> bool {
        arms.iter().all(|arm| match &arm.pat {
            syn::Pat::Lit(_) | syn::Pat::Wild(_) | syn::Pat::Ident(_) | syn::Pat::Range(_) => true,
            syn::Pat::Path(pp) => !self.path_pattern_requires_visit(&pp.path, variant_ctx),
            syn::Pat::Or(or_pat) => or_pat.cases.iter().all(|case| match case {
                syn::Pat::Lit(_) | syn::Pat::Wild(_) | syn::Pat::Range(_) => true,
                syn::Pat::Path(pp) => !self.path_pattern_requires_visit(&pp.path, variant_ctx),
                _ => false,
            }),
            _ => false,
        })
    }

    fn path_pattern_requires_visit(
        &self,
        path: &syn::Path,
        variant_ctx: Option<&VariantTypeContext>,
    ) -> bool {
        let enum_name = if path.segments.len() >= 2 {
            path.segments
                .iter()
                .nth_back(1)
                .map(|s| s.ident.to_string())
        } else if let Some(ctx) = variant_ctx {
            Some(ctx.enum_name.clone())
        } else {
            self.current_struct.clone()
        };

        enum_name
            .as_ref()
            .is_some_and(|name| self.data_enum_types.contains(name))
    }

    fn try_emit_binding_tuple_match(&mut self, match_expr: &syn::ExprMatch) -> bool {
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

        self.writeln("{");
        self.indent += 1;

        let mut tuple_elem_names = Vec::new();
        for (idx, elem) in tuple_scrutinee.elems.iter().enumerate() {
            let elem_name = format!("_m{}", idx);
            match elem {
                syn::Expr::Reference(r) => {
                    let reference_target = self.peel_reference_target_expr(&r.expr);
                    let mut inner_raw = self.emit_expr_to_string_with_expected(
                        reference_target,
                        tuple_expected_ty.as_ref(),
                    );
                    let inner = self.maybe_wrap_variant_constructor_with_expected_enum(
                        reference_target,
                        inner_raw,
                        tuple_expected_ty.as_ref(),
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

                    if self.is_stable_reference_lvalue_expr(reference_target)
                        && !is_slice_range_target
                        && !should_normalize_to_slice_full
                    {
                        self.writeln(&format!("auto {} = &{};", elem_name, inner_raw));
                    } else {
                        let tmp_name = format!("_m{}_tmp", idx);
                        self.writeln(&format!("auto {} = {};", tmp_name, inner_raw));
                        self.writeln(&format!("auto {} = &{};", elem_name, tmp_name));
                    }
                }
                _ => {
                    let elem_expr_raw =
                        self.emit_expr_to_string_with_expected(elem, tuple_expected_ty.as_ref());
                    let elem_expr = self.maybe_wrap_variant_constructor_with_expected_enum(
                        elem,
                        elem_expr_raw,
                        tuple_expected_ty.as_ref(),
                    );
                    self.writeln(&format!("auto {} = {};", elem_name, elem_expr));
                }
            }
            tuple_elem_names.push(elem_name);
        }

        self.writeln(&format!(
            "auto _m_tuple = std::make_tuple({});",
            tuple_elem_names.join(", ")
        ));
        self.writeln("do {");
        self.indent += 1;

        for arm in &match_expr.arms {
            self.writeln("{");
            self.indent += 1;

            let mut binding_stmts = Vec::new();
            match &arm.pat {
                syn::Pat::Tuple(_) => {
                    let _ = self.collect_pattern_binding_stmts(
                        &arm.pat,
                        "_m_tuple",
                        &mut binding_stmts,
                    );
                }
                syn::Pat::Ident(pi) => {
                    if pi.ident != "_" {
                        binding_stmts.push(format!("const auto& {} = _m_tuple;", pi.ident));
                    }
                }
                syn::Pat::Wild(_) => {}
                _ => {}
            }

            for stmt in binding_stmts {
                self.writeln(&stmt);
            }

            if let Some((_, guard)) = &arm.guard {
                let guard_str = self.emit_expr_to_string(guard);
                self.writeln(&format!("if ({}) {{", guard_str));
                self.indent += 1;
                self.emit_arm_body(&arm.body);
                self.writeln("break;");
                self.indent -= 1;
                self.writeln("}");
            } else {
                self.emit_arm_body(&arm.body);
                self.writeln("break;");
            }

            self.indent -= 1;
            self.writeln("}");
        }

        self.indent -= 1;
        self.writeln("} while (false);");
        self.indent -= 1;
        self.writeln("}");
        true
    }

    fn is_binding_only_tuple_arm_pattern(&self, pat: &syn::Pat, arity: usize) -> bool {
        match pat {
            syn::Pat::Tuple(tuple_pat) => {
                tuple_pat.elems.len() == arity
                    && tuple_pat
                        .elems
                        .iter()
                        .all(|elem| self.is_binding_only_pattern(elem))
            }
            syn::Pat::Wild(_) | syn::Pat::Ident(_) => true,
            _ => false,
        }
    }

    fn is_binding_only_pattern(&self, pat: &syn::Pat) -> bool {
        match pat {
            syn::Pat::Ident(_) | syn::Pat::Wild(_) => true,
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

    fn is_stable_reference_lvalue_expr(&self, expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                let name = path.path.segments[0].ident.to_string();
                name == "self"
                    || self.lookup_local_binding_type(&name).is_some()
                    || self.is_local_binding_in_scope(&name)
            }
            syn::Expr::Field(field) => self.is_stable_reference_lvalue_expr(&field.base),
            syn::Expr::Index(index) => self.is_stable_reference_lvalue_expr(&index.expr),
            syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => true,
            syn::Expr::Reference(r) => self.is_stable_reference_lvalue_expr(&r.expr),
            syn::Expr::Paren(p) => self.is_stable_reference_lvalue_expr(&p.expr),
            syn::Expr::Group(g) => self.is_stable_reference_lvalue_expr(&g.expr),
            _ => false,
        }
    }

    fn peel_reference_target_expr<'a>(&self, expr: &'a syn::Expr) -> &'a syn::Expr {
        let mut current = self.peel_paren_group_expr(expr);
        while let syn::Expr::Reference(r) = current {
            current = self.peel_paren_group_expr(&r.expr);
        }
        current
    }

    fn is_reference_to_slice_range_index_expr(&self, expr: &syn::Expr) -> bool {
        let reference_target = match self.peel_paren_group_expr(expr) {
            syn::Expr::Reference(r) => self.peel_reference_target_expr(&r.expr),
            _ => return false,
        };
        self.is_slice_range_index_target_expr(reference_target)
    }

    fn is_slice_range_index_target_expr(&self, expr: &syn::Expr) -> bool {
        match self.peel_paren_group_expr(expr) {
            syn::Expr::Index(idx) => self.is_slice_range_index_expr(&idx.index),
            _ => false,
        }
    }

    fn should_normalize_tuple_reference_target_to_slice_full(&self, expr: &syn::Expr) -> bool {
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

    fn emit_match_as_switch(&mut self, scrutinee: &str, arms: &[syn::Arm]) {
        self.writeln(&format!("switch ({}) {{", scrutinee));
        for arm in arms {
            match &arm.pat {
                syn::Pat::Wild(_) => {
                    self.writeln("default: {");
                }
                syn::Pat::Lit(lit) => {
                    let val = self.emit_lit(&lit.lit);
                    self.writeln(&format!("case {}: {{", val));
                }
                syn::Pat::Path(pp) => {
                    let val = self.emit_path_to_string(&pp.path);
                    self.writeln(&format!("case {}: {{", val));
                }
                syn::Pat::Or(or_pat) => {
                    // Multiple patterns: `1 | 2 | 3 =>`
                    for (i, case) in or_pat.cases.iter().enumerate() {
                        if let syn::Pat::Lit(lit) = case {
                            let val = self.emit_lit(&lit.lit);
                            if i < or_pat.cases.len() - 1 {
                                self.writeln(&format!("case {}:", val));
                            } else {
                                self.writeln(&format!("case {}: {{", val));
                            }
                        } else if let syn::Pat::Wild(_) = case {
                            self.writeln("default: {");
                        } else if let syn::Pat::Path(pp) = case {
                            let val = self.emit_path_to_string(&pp.path);
                            if i < or_pat.cases.len() - 1 {
                                self.writeln(&format!("case {}:", val));
                            } else {
                                self.writeln(&format!("case {}: {{", val));
                            }
                        }
                    }
                }
                syn::Pat::Ident(pi) => {
                    // Catch-all binding: `x =>`  acts like default
                    let name = &pi.ident;
                    self.writeln("default: {");
                    self.indent += 1;
                    self.writeln(&format!("auto {} = {};", name, scrutinee));
                    self.indent -= 1;
                }
                syn::Pat::Range(_) => {
                    self.writeln("// TODO: range pattern in switch");
                    self.writeln("default: {");
                }
                _ => {
                    self.writeln("// TODO: unhandled switch pattern");
                    self.writeln("default: {");
                }
            }

            self.indent += 1;

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

            self.writeln("break;");
            self.indent -= 1;
            self.writeln("}");
        }
        self.writeln("}");
    }

    fn emit_match_as_visit(
        &mut self,
        scrutinee: &str,
        arms: &[syn::Arm],
        variant_ctx: Option<&VariantTypeContext>,
    ) {
        // Emit: std::visit(overloaded { [](Type& v) { ... }, ... }, scrutinee);
        self.writeln(&format!("std::visit(overloaded {{"));
        self.indent += 1;

        for arm in arms {
            self.emit_visit_arm(arm, variant_ctx);
        }

        self.indent -= 1;
        self.writeln(&format!("}}, {});", scrutinee));
    }

    fn emit_visit_arm(&mut self, arm: &syn::Arm, variant_ctx: Option<&VariantTypeContext>) {
        match &arm.pat {
            syn::Pat::TupleStruct(ts) => {
                // Pattern like `Shape::Circle(r)` → [](const Shape_Circle& _v) { auto r = _v._0; ... }
                let cpp_type = self.variant_pattern_cpp_type(&ts.path, variant_ctx);
                let Some(binding_stmts) = self.tuple_struct_binding_stmts(&ts.elems, "_v") else {
                    self.writeln("// TODO: complex tuple-struct pattern binding");
                    self.writeln("[&](const auto&) {},");
                    return;
                };

                self.write_indent();
                let needs_mut_param = binding_stmts.iter().any(|s| s.starts_with("auto& "));
                let visit_param =
                    if needs_mut_param || self.pattern_requires_mut_ref_binding(&arm.pat) {
                        format!("{}& _v", cpp_type)
                    } else {
                        format!("const {}& _v", cpp_type)
                    };
                self.output.push_str(&format!("[&]({}) {{", visit_param));
                self.push_pattern_ref_binding_scope(&arm.pat);

                if !binding_stmts.is_empty() || arm.guard.is_some() {
                    self.output.push('\n');
                    self.indent += 1;
                    // Emit bindings
                    for stmt in &binding_stmts {
                        self.writeln(stmt);
                    }
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
                    self.indent -= 1;
                    self.writeln("},");
                    self.pop_pattern_ref_binding_scope();
                } else {
                    let body_str = self.emit_expr_to_string(&arm.body);
                    self.output.push_str(&format!(" {}; }},\n", body_str));
                    self.pop_pattern_ref_binding_scope();
                }
            }
            syn::Pat::Struct(ps) => {
                // Pattern like `Shape::Rect { w, h }` → [](const Shape_Rect& _v) { ... }
                let cpp_type = self.variant_pattern_cpp_type(&ps.path, variant_ctx);

                self.write_indent();
                let visit_param = if self.pattern_requires_mut_ref_binding(&arm.pat) {
                    format!("{}& _v", cpp_type)
                } else {
                    format!("const {}& _v", cpp_type)
                };
                self.output.push_str(&format!("[&]({}) {{\n", visit_param));
                self.push_pattern_ref_binding_scope(&arm.pat);
                self.indent += 1;

                // Emit field bindings
                for field_pat in &ps.fields {
                    let field_name = field_pat.member.clone();
                    let field_name_str = match &field_name {
                        syn::Member::Named(ident) => ident.to_string(),
                        syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                    };
                    let binding_name = match &*field_pat.pat {
                        syn::Pat::Ident(pi) => pi.ident.to_string(),
                        _ => field_name_str.clone(),
                    };
                    self.writeln(&format!(
                        "const auto& {} = _v.{};",
                        binding_name, field_name_str
                    ));
                }

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

                self.indent -= 1;
                self.writeln("},");
                self.pop_pattern_ref_binding_scope();
            }
            syn::Pat::Path(pp) => {
                // Unit variant: `Shape::None` → [](const Shape_None&) { ... }
                let cpp_type = self.variant_pattern_cpp_type(&pp.path, variant_ctx);

                self.write_indent();
                self.output
                    .push_str(&format!("[&](const {}&) {{\n", cpp_type));
                self.push_pattern_ref_binding_scope(&arm.pat);
                self.indent += 1;
                self.emit_arm_body(&arm.body);
                self.indent -= 1;
                self.writeln("},");
                self.pop_pattern_ref_binding_scope();
            }
            syn::Pat::Wild(_) => {
                // Wildcard: `_ =>` → [](const auto&) { ... }
                self.write_indent();
                self.output.push_str("[&](const auto&) {\n");
                self.push_pattern_ref_binding_scope(&arm.pat);
                self.indent += 1;
                self.emit_arm_body(&arm.body);
                self.indent -= 1;
                self.writeln("},");
                self.pop_pattern_ref_binding_scope();
            }
            syn::Pat::Ident(pi) => {
                // Catch-all binding: `x =>` → [](const auto& x) { ... }
                let name = &pi.ident;
                self.write_indent();
                self.output
                    .push_str(&format!("[&](const auto& {}) {{\n", name));
                self.push_pattern_ref_binding_scope(&arm.pat);
                self.indent += 1;
                self.emit_arm_body(&arm.body);
                self.indent -= 1;
                self.writeln("},");
                self.pop_pattern_ref_binding_scope();
            }
            _ => {
                self.writeln("// TODO: unhandled match pattern");
                self.writeln("[&](const auto&) {},");
            }
        }
    }

    fn extract_tuple_struct_bindings(
        &self,
        elems: &syn::punctuated::Punctuated<syn::Pat, syn::token::Comma>,
    ) -> Vec<String> {
        elems
            .iter()
            .map(|p| match p {
                syn::Pat::Ident(pi) => pi.ident.to_string(),
                syn::Pat::Wild(_) => "_".to_string(),
                _ => "_".to_string(),
            })
            .collect()
    }

    fn tuple_struct_binding_stmts(
        &self,
        elems: &syn::punctuated::Punctuated<syn::Pat, syn::token::Comma>,
        base_expr: &str,
    ) -> Option<Vec<String>> {
        let mut stmts = Vec::new();
        for (i, elem_pat) in elems.iter().enumerate() {
            let field_expr = format!("{}._{}", base_expr, i);
            if !self.collect_pattern_binding_stmts(elem_pat, &field_expr, &mut stmts) {
                return None;
            }
        }
        Some(stmts)
    }

    fn collect_pattern_binding_stmts(
        &self,
        pat: &syn::Pat,
        source_expr: &str,
        out: &mut Vec<String>,
    ) -> bool {
        match pat {
            syn::Pat::Ident(pi) => {
                if pi.ident != "_" {
                    let binding_prefix = if pi.by_ref.is_some() && pi.mutability.is_some() {
                        "auto&"
                    } else if pi.by_ref.is_some() {
                        "const auto&"
                    } else {
                        // By-value pattern bindings should preserve reference payloads
                        // (e.g., `R = T&`) without forcing `const`.
                        "auto&&"
                    };
                    out.push(format!(
                        "{} {} = {};",
                        binding_prefix, pi.ident, source_expr
                    ));
                }
                true
            }
            syn::Pat::Wild(_) => true,
            syn::Pat::Tuple(tuple_pat) => {
                for (i, elem) in tuple_pat.elems.iter().enumerate() {
                    let elem_expr = format!("std::get<{}>({})", i, source_expr);
                    if !self.collect_pattern_binding_stmts(elem, &elem_expr, out) {
                        return false;
                    }
                }
                true
            }
            syn::Pat::Type(pt) => self.collect_pattern_binding_stmts(&pt.pat, source_expr, out),
            syn::Pat::Reference(r) => self.collect_pattern_binding_stmts(&r.pat, source_expr, out),
            syn::Pat::Paren(p) => self.collect_pattern_binding_stmts(&p.pat, source_expr, out),
            _ => false,
        }
    }

    fn pattern_requires_mut_ref_binding(&self, pat: &syn::Pat) -> bool {
        match pat {
            syn::Pat::Ident(pi) => pi.by_ref.is_some() && pi.mutability.is_some(),
            syn::Pat::TupleStruct(ts) => ts
                .elems
                .iter()
                .any(|p| self.pattern_requires_mut_ref_binding(p)),
            syn::Pat::Struct(ps) => ps
                .fields
                .iter()
                .any(|f| self.pattern_requires_mut_ref_binding(&f.pat)),
            syn::Pat::Tuple(tuple) => tuple
                .elems
                .iter()
                .any(|p| self.pattern_requires_mut_ref_binding(p)),
            syn::Pat::Reference(r) => {
                r.mutability.is_some() || self.pattern_requires_mut_ref_binding(&r.pat)
            }
            syn::Pat::Type(pt) => self.pattern_requires_mut_ref_binding(&pt.pat),
            syn::Pat::Paren(p) => self.pattern_requires_mut_ref_binding(&p.pat),
            syn::Pat::Or(or_pat) => or_pat
                .cases
                .iter()
                .any(|p| self.pattern_requires_mut_ref_binding(p)),
            _ => false,
        }
    }

    /// Emit a macro invocation as a statement.
    fn emit_macro_stmt(&mut self, mac: &syn::Macro) {
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
            "assert" => {
                self.writeln(&format!("assert({});", self.convert_macro_tokens(&tokens)));
            }
            "assert_eq" => {
                let parts = self.split_macro_args(&tokens);
                if parts.len() >= 2 {
                    let left = self.convert_macro_tokens(parts[0].trim());
                    let right = self.convert_macro_tokens(parts[1].trim());
                    self.writeln(&format!("assert(({} == {}));", left, right));
                }
            }
            "assert_ne" => {
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

    fn split_for_both_macro_parts(
        &self,
        tokens: &proc_macro2::TokenStream,
    ) -> Option<(
        proc_macro2::TokenStream,
        proc_macro2::TokenStream,
        proc_macro2::TokenStream,
    )> {
        let token_vec: Vec<TokenTree> = tokens.clone().into_iter().collect();
        let comma_idx = token_vec
            .iter()
            .position(|tt| matches!(tt, TokenTree::Punct(p) if p.as_char() == ','))?;
        let receiver_tokens: proc_macro2::TokenStream =
            token_vec[..comma_idx].iter().cloned().collect();

        let rest: Vec<TokenTree> = token_vec[comma_idx + 1..].to_vec();
        let arrow_idx = (0..rest.len().saturating_sub(1)).find(|&i| {
            if let (TokenTree::Punct(eq), TokenTree::Punct(gt)) = (&rest[i], &rest[i + 1]) {
                eq.as_char() == '=' && gt.as_char() == '>'
            } else {
                false
            }
        })?;

        let pattern_tokens: proc_macro2::TokenStream = rest[..arrow_idx].iter().cloned().collect();
        let body_tokens: proc_macro2::TokenStream = rest[arrow_idx + 2..].iter().cloned().collect();
        if body_tokens.is_empty() {
            return None;
        }
        Some((receiver_tokens, pattern_tokens, body_tokens))
    }

    fn for_both_lambda_components(&self, pat: &syn::Pat) -> Option<(String, String, String)> {
        match pat {
            syn::Pat::Ident(pi) => {
                let name = pi.ident.to_string();
                if pi.by_ref.is_some() && pi.mutability.is_some() {
                    Some((
                        "auto& _v".to_string(),
                        format!("auto& {} = _v._0; ", name),
                        "_m".to_string(),
                    ))
                } else if pi.by_ref.is_some() {
                    Some((
                        "const auto& _v".to_string(),
                        format!("const auto& {} = _v._0; ", name),
                        "_m".to_string(),
                    ))
                } else {
                    Some((
                        "auto&& _v".to_string(),
                        format!("auto&& {} = _v._0; ", name),
                        "std::move(_m)".to_string(),
                    ))
                }
            }
            syn::Pat::Wild(_) => Some(("auto&& _v".to_string(), String::new(), "_m".to_string())),
            _ => None,
        }
    }

    fn try_lower_for_both_macro_expr(&self, mac: &syn::Macro) -> Option<String> {
        let (receiver_tokens, pattern_tokens, body_tokens) =
            self.split_for_both_macro_parts(&mac.tokens)?;
        let receiver_expr = syn::parse2::<syn::Expr>(receiver_tokens).ok()?;
        let binding_pat = syn::Pat::parse_single.parse2(pattern_tokens).ok()?;
        let body_expr = syn::parse2::<syn::Expr>(body_tokens).ok()?;
        let receiver_cpp = self.emit_expr_to_string(&receiver_expr);
        let body_cpp = self
            .try_emit_for_both_io_method_shape_dispatch(&binding_pat, &body_expr)
            .unwrap_or_else(|| self.emit_expr_to_string(&body_expr));
        let (lambda_param, binding_stmt, visit_arg) =
            self.for_both_lambda_components(&binding_pat)?;
        Some(format!(
            "[&]() {{ auto&& _m = {}; return std::visit(overloaded {{ [&]({}) -> decltype(auto) {{ {}return {}; }} }}, {}); }}()",
            receiver_cpp, lambda_param, binding_stmt, body_cpp, visit_arg
        ))
    }

    fn try_emit_for_both_io_method_shape_dispatch(
        &self,
        binding_pat: &syn::Pat,
        body_expr: &syn::Expr,
    ) -> Option<String> {
        let binding_name = match binding_pat {
            syn::Pat::Ident(pi) => pi.ident.to_string(),
            _ => return None,
        };

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
        if receiver_name != binding_name {
            return None;
        }

        let arg = self.emit_expr_maybe_move(mc.args.first()?);
        match mc.method.to_string().as_str() {
            "read" => Some(format!("rusty::io::read({}, {})", binding_name, arg)),
            "write" => Some(format!("rusty::io::write({}, {})", binding_name, arg)),
            _ => None,
        }
    }

    /// Emit a macro invocation as an expression (returns a string).
    fn emit_macro_expr(&self, mac: &syn::Macro) -> String {
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

    /// Convert Rust format string args to C++ std::format args.
    /// `"hello {}", x` → `"hello {}", x`
    /// The format syntax is mostly compatible between Rust and C++23.
    fn convert_format_args(&self, tokens: &str) -> String {
        // Rust's format!("...", args) is very close to C++23's std::format("...", args)
        // The {} placeholders are identical. Named args differ but basic usage is the same.
        self.convert_macro_tokens(tokens)
    }

    /// Convert raw macro tokens to a C++ expression string.
    /// Applies basic Rust→C++ token transformations.
    fn convert_macro_tokens(&self, tokens: &str) -> String {
        let mut result = tokens.to_string();
        // Replace Rust Option/Result constructors in token context
        result = result.replace("None", "std::nullopt");
        // Replace Some(x) or Some (x) → std::make_optional(x)
        // Normalize "Some (" to "Some(" first
        result = result.replace("Some (", "Some(");
        while let Some(pos) = result.find("Some(") {
            if let Some(end) = find_matching_paren(&result, pos + 4) {
                let inner = &result[pos + 5..end].to_string();
                result = format!(
                    "{}std::make_optional({}){}",
                    &result[..pos],
                    inner,
                    &result[end + 1..]
                );
            } else {
                break;
            }
        }
        // Replace &mut with & (C++ doesn't have &mut)
        result = result.replace("& mut ", "&");
        result
    }

    /// Split macro arguments by top-level commas (respecting nesting).
    fn split_macro_args(&self, tokens: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut depth = 0i32;

        for ch in tokens.chars() {
            match ch {
                '(' | '[' | '{' => {
                    depth += 1;
                    current.push(ch);
                }
                ')' | ']' | '}' => {
                    depth -= 1;
                    current.push(ch);
                }
                ',' if depth == 0 => {
                    result.push(current.clone());
                    current.clear();
                }
                _ => current.push(ch),
            }
        }
        if !current.is_empty() {
            result.push(current);
        }
        result
    }

    /// Emit a switch-style match as an expression (returns string for IIFE body).
    fn emit_match_expr_switch(&self, arms: &[syn::Arm], expected_ty: Option<&syn::Type>) -> String {
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
                syn::Pat::Wild(_) => parts.push(format!("return {};", body)),
                syn::Pat::Lit(lit) => {
                    let val = self.emit_lit(&lit.lit);
                    parts.push(format!("if (_m == {}) return {};", val, body));
                }
                syn::Pat::Path(pp) => {
                    let val = self.emit_path_to_string(&pp.path);
                    parts.push(format!("if (_m == {}) return {};", val, body));
                }
                syn::Pat::Or(or_pat) => {
                    let mut conds = Vec::new();
                    for case in &or_pat.cases {
                        match case {
                            syn::Pat::Lit(lit) => {
                                conds.push(format!("_m == {}", self.emit_lit(&lit.lit)))
                            }
                            syn::Pat::Path(pp) => {
                                conds.push(format!("_m == {}", self.emit_path_to_string(&pp.path)))
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
                        parts.push(format!("return {};", body));
                    } else {
                        parts.push(format!("if ({}) return {};", conds.join(" || "), body));
                    }
                }
                _ => parts.push(format!("return {};", body)),
            }
        }
        parts.join(" ")
    }

    /// Emit a variant match as expression body (returns string for visit lambdas).
    fn emit_match_expr_visit(
        &self,
        arms: &[syn::Arm],
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
                syn::Pat::TupleStruct(ts) => {
                    let cpp_type = self.variant_pattern_cpp_type(&ts.path, variant_ctx);
                    let Some(binding_stmts) = self.tuple_struct_binding_stmts(&ts.elems, "_v")
                    else {
                        parts.push(format!(
                            "[&](const auto&) {{ return {}; }}",
                            self.match_expr_unreachable_fallback_with_expected(expected_ty)
                        ));
                        continue;
                    };
                    if let Some((_, guard)) = &arm.guard {
                        let guard_str = self.emit_expr_to_string(guard);
                        let needs_mut_param = binding_stmts.iter().any(|s| s.starts_with("auto& "));
                        let visit_param =
                            if needs_mut_param || self.pattern_requires_mut_ref_binding(&arm.pat) {
                                format!("{}& _v", cpp_type)
                            } else {
                                format!("const {}& _v", cpp_type)
                            };
                        parts.push(format!(
                            "[&]({}) {{ {} if ({}) return {}; return {}; }}",
                            visit_param,
                            binding_stmts.join(" "),
                            guard_str,
                            body,
                            self.match_expr_unreachable_fallback_with_expected(expected_ty),
                        ));
                    } else {
                        let needs_mut_param = binding_stmts.iter().any(|s| s.starts_with("auto& "));
                        let visit_param =
                            if needs_mut_param || self.pattern_requires_mut_ref_binding(&arm.pat) {
                                format!("{}& _v", cpp_type)
                            } else {
                                format!("const {}& _v", cpp_type)
                            };
                        parts.push(format!(
                            "[&]({}) {{ {} return {}; }}",
                            visit_param,
                            binding_stmts.join(" "),
                            body
                        ));
                    }
                }
                syn::Pat::Path(pp) => {
                    let cpp_type = self.variant_pattern_cpp_type(&pp.path, variant_ctx);
                    if let Some((_, guard)) = &arm.guard {
                        let guard_str = self.emit_expr_to_string(guard);
                        parts.push(format!(
                            "[&](const {}& _v) {{ if ({}) return {}; return {}; }}",
                            cpp_type,
                            guard_str,
                            body,
                            self.match_expr_unreachable_fallback_with_expected(expected_ty),
                        ));
                    } else {
                        parts.push(format!("[&](const {}&) {{ return {}; }}", cpp_type, body));
                    }
                }
                syn::Pat::Wild(_) => {
                    parts.push(format!("[&](const auto&) {{ return {}; }}", body));
                }
                syn::Pat::Ident(pi) => {
                    parts.push(format!(
                        "[&](const auto& {}) {{ return {}; }}",
                        pi.ident, body
                    ));
                }
                _ => {
                    parts.push(format!(
                        "[&](const auto&) {{ return {}; }}",
                        self.match_expr_unreachable_fallback_with_expected(expected_ty)
                    ));
                }
            }
        }
        parts.join(", ")
    }

    fn runtime_match_enum_kind_by_name(&self, enum_name: &str) -> Option<RuntimeMatchEnumKind> {
        if self.data_enum_types.contains(enum_name) {
            return None;
        }
        match enum_name {
            "Option" => Some(RuntimeMatchEnumKind::Option),
            "Result" => Some(RuntimeMatchEnumKind::Result),
            _ => None,
        }
    }

    fn runtime_match_enum_kind_for_path(
        &self,
        path: &syn::Path,
        variant_ctx: Option<&VariantTypeContext>,
    ) -> Option<RuntimeMatchEnumKind> {
        if path.segments.len() >= 2 {
            let enum_name = path.segments.iter().nth_back(1)?.ident.to_string();
            if let Some(kind) = self.runtime_match_enum_kind_by_name(&enum_name) {
                return Some(kind);
            }
        }
        variant_ctx.and_then(|ctx| self.runtime_match_enum_kind_by_name(&ctx.enum_name))
    }

    fn runtime_tuple_struct_match_methods(
        &self,
        path: &syn::Path,
        variant_ctx: Option<&VariantTypeContext>,
    ) -> Option<(&'static str, &'static str)> {
        let kind = self.runtime_match_enum_kind_for_path(path, variant_ctx)?;
        let variant_name = path.segments.last()?.ident.to_string();
        match (kind, variant_name.as_str()) {
            (RuntimeMatchEnumKind::Option, "Some") => Some(("is_some", "unwrap")),
            (RuntimeMatchEnumKind::Result, "Ok") => Some(("is_ok", "unwrap")),
            (RuntimeMatchEnumKind::Result, "Err") => Some(("is_err", "unwrap_err")),
            _ => None,
        }
    }

    fn runtime_path_match_condition_method(
        &self,
        path: &syn::Path,
        variant_ctx: Option<&VariantTypeContext>,
    ) -> Option<&'static str> {
        let kind = self.runtime_match_enum_kind_for_path(path, variant_ctx)?;
        let variant_name = path.segments.last()?.ident.to_string();
        match (kind, variant_name.as_str()) {
            (RuntimeMatchEnumKind::Option, "None") => Some("is_none"),
            _ => None,
        }
    }

    fn emit_runtime_match_expr(
        &self,
        match_expr: &syn::ExprMatch,
        variant_ctx: Option<&VariantTypeContext>,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        let scrutinee = self.emit_expr_to_string(&match_expr.expr);
        let mut out = format!("[&]() {{ auto&& _m = {}; ", scrutinee);

        let mut saw_runtime_pattern = false;
        for (idx, arm) in match_expr.arms.iter().enumerate() {
            let body = {
                let emitted = self.emit_expr_to_string_with_expected(&arm.body, expected_ty);
                self.maybe_wrap_variant_constructor_with_expected_enum(
                    &arm.body,
                    emitted,
                    expected_ty,
                )
            };
            match &arm.pat {
                syn::Pat::TupleStruct(ts) => {
                    let Some((cond_method, unwrap_method)) =
                        self.runtime_tuple_struct_match_methods(&ts.path, variant_ctx)
                    else {
                        return None;
                    };
                    if ts.elems.len() != 1 {
                        return None;
                    }
                    saw_runtime_pattern = true;
                    out.push_str(&format!("if (_m.{}()) {{ ", cond_method));
                    let matched_value = format!("_mv{}", idx);
                    out.push_str(&format!(
                        "auto {} = _m.{}(); ",
                        matched_value, unwrap_method
                    ));

                    let mut binding_stmts = Vec::new();
                    if !self.collect_pattern_binding_stmts(
                        &ts.elems[0],
                        &matched_value,
                        &mut binding_stmts,
                    ) {
                        return None;
                    }
                    for stmt in binding_stmts {
                        out.push_str(&stmt);
                        out.push(' ');
                    }

                    if let Some((_, guard)) = &arm.guard {
                        let guard_str = self.emit_expr_to_string(guard);
                        out.push_str(&format!("if ({}) {{ return {}; }} ", guard_str, body));
                    } else {
                        out.push_str(&format!("return {}; ", body));
                    }
                    out.push_str("} ");
                }
                syn::Pat::Path(pp) => {
                    let Some(cond_method) =
                        self.runtime_path_match_condition_method(&pp.path, variant_ctx)
                    else {
                        return None;
                    };
                    saw_runtime_pattern = true;
                    out.push_str(&format!("if (_m.{}()) {{ ", cond_method));
                    if let Some((_, guard)) = &arm.guard {
                        let guard_str = self.emit_expr_to_string(guard);
                        out.push_str(&format!("if ({}) {{ return {}; }} ", guard_str, body));
                    } else {
                        out.push_str(&format!("return {}; ", body));
                    }
                    out.push_str("} ");
                }
                syn::Pat::Wild(_) => {
                    out.push_str("if (true) { ");
                    if let Some((_, guard)) = &arm.guard {
                        let guard_str = self.emit_expr_to_string(guard);
                        out.push_str(&format!("if ({}) {{ return {}; }} ", guard_str, body));
                    } else {
                        out.push_str(&format!("return {}; ", body));
                    }
                    out.push_str("} ");
                }
                syn::Pat::Ident(pi) => {
                    out.push_str("if (true) { ");
                    out.push_str(&format!("const auto& {} = _m; ", pi.ident));
                    if let Some((_, guard)) = &arm.guard {
                        let guard_str = self.emit_expr_to_string(guard);
                        out.push_str(&format!("if ({}) {{ return {}; }} ", guard_str, body));
                    } else {
                        out.push_str(&format!("return {}; ", body));
                    }
                    out.push_str("} ");
                }
                _ => return None,
            }
        }

        if !saw_runtime_pattern {
            return None;
        }

        out.push_str(&format!(
            "return {}; }}()",
            self.match_expr_unreachable_fallback_with_expected(expected_ty)
        ));
        Some(out)
    }

    /// Emit a tuple-scrutinee match expression as overloaded std::visit lambdas.
    /// This handles patterns like:
    /// `(E::A(x), E::A(y)) => x == y`
    fn emit_match_expr_visit_tuple(
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
                            binding_stmts.join(" "),
                            guard_str,
                            body,
                            self.match_expr_unreachable_fallback_with_expected(expected_ty)
                        ));
                    } else {
                        parts.push(format!(
                            "[&]({}) {{ {} return {}; }}",
                            params.join(", "),
                            binding_stmts.join(" "),
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

    fn emit_tuple_visit_subpattern(
        &self,
        pat: &syn::Pat,
        index: usize,
        params: &mut Vec<String>,
        binding_stmts: &mut Vec<String>,
        variant_ctx: Option<&VariantTypeContext>,
    ) -> bool {
        match pat {
            syn::Pat::TupleStruct(ts) => {
                let cpp_type = self.variant_pattern_cpp_type(&ts.path, variant_ctx);
                let param_name = format!("_v{}", index);
                params.push(format!("const {}& {}", cpp_type, param_name));
                let Some(stmts) = self.tuple_struct_binding_stmts(&ts.elems, &param_name) else {
                    return false;
                };
                binding_stmts.extend(stmts);
                true
            }
            syn::Pat::Path(pp) => {
                let cpp_type = self.variant_pattern_cpp_type(&pp.path, variant_ctx);
                params.push(format!("const {}& _v{}", cpp_type, index));
                true
            }
            syn::Pat::Ident(pi) => {
                params.push(format!("const auto& {}", pi.ident));
                true
            }
            syn::Pat::Wild(_) => {
                params.push(format!("const auto& _v{}", index));
                true
            }
            _ => false,
        }
    }

    fn match_expr_unreachable_fallback(&self) -> &'static str {
        "rusty::intrinsics::unreachable()"
    }

    fn match_expr_unreachable_fallback_with_expected(
        &self,
        expected_ty: Option<&syn::Type>,
    ) -> String {
        if let Some(expected) = expected_ty {
            return format!(
                "[&]() -> {} {{ rusty::intrinsics::unreachable(); }}()",
                self.map_type(expected)
            );
        }
        self.match_expr_unreachable_fallback().to_string()
    }

    /// Map Rust enum-variant pattern paths to generated C++ variant struct names.
    /// Examples:
    /// - `Either::Left` -> `Either_Left`
    /// - `crate::Either::Left` -> `Either_Left`
    /// - `Left` inside `impl Either` -> `Either_Left`
    /// When generic context is known, append template arguments:
    /// - `Either::Left` in `Either<L, R>` context -> `Either_Left<L, R>`
    fn variant_pattern_cpp_type(
        &self,
        path: &syn::Path,
        variant_ctx: Option<&VariantTypeContext>,
    ) -> String {
        let raw = self.emit_path_to_string(path).replace("::", "_");
        let stripped = raw
            .strip_prefix("crate_")
            .or_else(|| raw.strip_prefix("self_"))
            .or_else(|| raw.strip_prefix("super_"))
            .unwrap_or(&raw)
            .to_string();
        let looks_like_variant_name = stripped
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase());
        let base = if stripped.contains('_') {
            stripped
        } else if looks_like_variant_name {
            if let Some(ctx) = variant_ctx {
                format!("{}_{}", ctx.enum_name, stripped)
            } else if let Some(struct_name) = &self.current_struct {
                if self.data_enum_types.contains(struct_name) {
                    format!("{}_{}", struct_name, stripped)
                } else {
                    stripped
                }
            } else {
                stripped
            }
        } else {
            stripped
        };

        let enum_name = self
            .extract_variant_pattern_enum_name(path, &base)
            .or_else(|| variant_ctx.map(|ctx| ctx.enum_name.clone()));
        let template_args =
            self.variant_pattern_template_args(path, enum_name.as_deref(), variant_ctx);
        if template_args.is_empty() {
            base
        } else {
            format!("{}<{}>", base, template_args.join(", "))
        }
    }

    fn extract_variant_pattern_enum_name(
        &self,
        path: &syn::Path,
        resolved_cpp_type: &str,
    ) -> Option<String> {
        if path.segments.len() >= 2 {
            let penultimate = path.segments.iter().nth_back(1)?.ident.to_string();
            if penultimate == "Self" {
                return self.current_struct.clone();
            }
            return Some(penultimate);
        }
        if let Some(struct_name) = &self.current_struct {
            if resolved_cpp_type.starts_with(&format!("{}_", struct_name)) {
                return Some(struct_name.clone());
            }
        }
        None
    }

    fn variant_pattern_template_args(
        &self,
        path: &syn::Path,
        enum_name: Option<&str>,
        variant_ctx: Option<&VariantTypeContext>,
    ) -> Vec<String> {
        if path.segments.len() >= 2 {
            if let Some(enum_segment) = path.segments.iter().nth_back(1) {
                if let syn::PathArguments::AngleBracketed(args) = &enum_segment.arguments {
                    let explicit_args: Vec<String> = args
                        .args
                        .iter()
                        .filter_map(|arg| match arg {
                            syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                            _ => None,
                        })
                        .collect();
                    if !explicit_args.is_empty() {
                        return explicit_args;
                    }
                }
            }
        }

        if let (Some(name), Some(ctx)) = (enum_name, variant_ctx) {
            if (ctx.enum_name == name || matches!(name, "crate" | "self" | "super"))
                && !ctx.template_args.is_empty()
            {
                return ctx.template_args.clone();
            }
        }

        if let Some(name) = enum_name {
            if let Some(params) = self.enum_type_params.get(name) {
                if !params.is_empty() && params.iter().all(|p| self.is_type_param_in_scope(p)) {
                    return params.clone();
                }
            }
        }
        Vec::new()
    }

    fn infer_variant_type_context_from_expr(&self, expr: &syn::Expr) -> Option<VariantTypeContext> {
        match expr {
            syn::Expr::Path(path) => {
                if path.path.segments.len() == 1 {
                    let name = path.path.segments[0].ident.to_string();
                    if name == "self" {
                        if let Some(enum_name) = &self.current_struct {
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
            syn::Expr::Paren(p) => self.infer_variant_type_context_from_expr(&p.expr),
            syn::Expr::Group(g) => self.infer_variant_type_context_from_expr(&g.expr),
            syn::Expr::Reference(r) => self.infer_variant_type_context_from_expr(&r.expr),
            _ => None,
        }
    }

    fn infer_variant_type_context_from_type(&self, ty: &syn::Type) -> Option<VariantTypeContext> {
        let syn::Type::Path(tp) = ty else {
            return None;
        };
        let last = tp.path.segments.last()?;
        let enum_name = last.ident.to_string();
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

    fn emit_arm_body(&mut self, body: &syn::Expr) {
        // If the body is a block, emit its statements
        if let syn::Expr::Block(block) = body {
            self.emit_block(&block.block);
        } else if self.try_emit_control_flow(body) {
            // Control flow handled
        } else {
            let body_str = self.emit_expr_to_string(body);
            self.writeln(&format!("{};", body_str));
        }
    }

    fn emit_if(&mut self, if_expr: &syn::ExprIf) {
        self.emit_if_inner(if_expr, true);
    }

    /// Emit `if let Pattern = expr { ... } else { ... }` as C++ code.
    fn emit_if_let(
        &mut self,
        let_expr: &syn::ExprLet,
        then_branch: &syn::Block,
        else_branch: &Option<(syn::token::Else, Box<syn::Expr>)>,
        first: bool,
    ) {
        let scrutinee = self.emit_expr_to_string(&let_expr.expr);

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
                        // if let Some(v) = opt → if (opt.is_some()) { auto v = opt.unwrap(); ... }
                        let binding = self.extract_tuple_struct_bindings(&ts.elems);
                        let cond = format!("{}.is_some()", scrutinee);
                        self.emit_if_let_body(
                            &cond,
                            &binding,
                            &scrutinee,
                            "unwrap",
                            then_branch,
                            else_branch,
                            first,
                        );
                    }
                    "Ok" | "Result::Ok" => {
                        // if let Ok(v) = result → if (result.is_ok()) { auto v = result.unwrap(); ... }
                        let binding = self.extract_tuple_struct_bindings(&ts.elems);
                        let cond = format!("{}.is_ok()", scrutinee);
                        self.emit_if_let_body(
                            &cond,
                            &binding,
                            &scrutinee,
                            "unwrap",
                            then_branch,
                            else_branch,
                            first,
                        );
                    }
                    "Err" | "Result::Err" => {
                        // if let Err(e) = result → if (result.is_err()) { auto e = result.unwrap_err(); ... }
                        let binding = self.extract_tuple_struct_bindings(&ts.elems);
                        let cond = format!("{}.is_err()", scrutinee);
                        self.emit_if_let_body(
                            &cond,
                            &binding,
                            &scrutinee,
                            "unwrap_err",
                            then_branch,
                            else_branch,
                            first,
                        );
                    }
                    _ => {
                        // Generic enum variant: if let Variant(x) = val
                        // → if (std::holds_alternative<Variant>(val)) { ... }
                        let cpp_type = path_str.replace("::", "_");
                        let binding = self.extract_tuple_struct_bindings(&ts.elems);
                        let cond = format!("std::holds_alternative<{}>({})", cpp_type, scrutinee);

                        if first {
                            self.writeln(&format!("if ({}) {{", cond));
                        } else {
                            self.output.push_str(&format!("if ({}) {{\n", cond));
                        }
                        self.indent += 1;
                        // Emit bindings
                        for (i, name) in binding.iter().enumerate() {
                            if name != "_" {
                                self.writeln(&format!(
                                    "const auto& {} = std::get<{}>({})._{};",
                                    name, cpp_type, scrutinee, i
                                ));
                            }
                        }
                        self.emit_block(then_branch);
                        self.indent -= 1;
                        self.emit_if_let_else(else_branch);
                    }
                }
            }
            syn::Pat::Ident(pi) => {
                let name = pi.ident.to_string();
                // Check for known enum variants that parse as idents
                let cond = match name.as_str() {
                    "None" => Some(format!("{}.is_none()", scrutinee)),
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
                    if first {
                        self.writeln("if (true) {");
                    } else {
                        self.output.push_str("if (true) {\n");
                    }
                    self.indent += 1;
                    self.writeln(&format!("auto {} = {};", name, scrutinee));
                    self.emit_block(then_branch);
                    self.indent -= 1;
                    self.emit_if_let_else(else_branch);
                }
            }
            syn::Pat::Path(pp) => {
                // if let None = opt → if (opt.is_none())
                let path_str = pp
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                let cond = match path_str.as_str() {
                    "None" | "Option::None" => format!("{}.is_none()", scrutinee),
                    _ => {
                        let cpp_type = path_str.replace("::", "_");
                        format!("std::holds_alternative<{}>({})", cpp_type, scrutinee)
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
                // Fallback
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

    /// Helper for common if-let patterns (Some/Ok/Err).
    fn emit_if_let_body(
        &mut self,
        cond: &str,
        bindings: &[String],
        scrutinee: &str,
        unwrap_method: &str,
        then_branch: &syn::Block,
        else_branch: &Option<(syn::token::Else, Box<syn::Expr>)>,
        first: bool,
    ) {
        if first {
            self.writeln(&format!("if ({}) {{", cond));
        } else {
            self.output.push_str(&format!("if ({}) {{\n", cond));
        }
        self.indent += 1;

        // Emit bindings
        if bindings.len() == 1 && bindings[0] != "_" {
            self.writeln(&format!(
                "auto {} = {}.{}();",
                bindings[0], scrutinee, unwrap_method
            ));
        }

        self.emit_block(then_branch);
        self.indent -= 1;
        self.emit_if_let_else(else_branch);
    }

    /// Emit the else branch of an if-let, if present.
    fn emit_if_let_else(&mut self, else_branch: &Option<(syn::token::Else, Box<syn::Expr>)>) {
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

    fn emit_if_inner(&mut self, if_expr: &syn::ExprIf, first: bool) {
        // Check for `if let` pattern
        if let syn::Expr::Let(let_expr) = &*if_expr.cond {
            self.emit_if_let(let_expr, &if_expr.then_branch, &if_expr.else_branch, first);
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

    fn emit_while(&mut self, while_expr: &syn::ExprWhile) {
        let cond = self.emit_expr_to_string(&while_expr.cond);
        self.writeln(&format!("while ({}) {{", cond));
        self.indent += 1;
        self.emit_block(&while_expr.body);
        self.indent -= 1;
        self.writeln("}");
    }

    fn emit_loop(&mut self, loop_expr: &syn::ExprLoop) {
        // Check if this loop uses break-with-value.
        // If a loop has `break <expr>`, it's used as a value-producing loop.
        // We emit it as-is, and when used as `let x = loop { ... break val; }`,
        // the break-with-value is handled by emit_expr_to_string emitting `return val`.
        // The enclosing `let x = loop { ... }` should wrap in a lambda —
        // but that's handled at the let-binding level.
        self.writeln("while (true) {");
        self.indent += 1;
        self.emit_block(&loop_expr.body);
        self.indent -= 1;
        self.writeln("}");
    }

    fn emit_for_loop(&mut self, for_expr: &syn::ExprForLoop) {
        let pat = self.emit_pat_to_string(&for_expr.pat);
        let iter = self.emit_expr_to_string(&for_expr.expr);

        // Rust `for x in expr` → C++ `for (auto& x : expr)` or `for (auto x : expr)`
        // Use auto&& to handle both references and values correctly
        self.writeln(&format!("for (auto&& {} : {}) {{", pat, iter));
        self.indent += 1;
        self.emit_block(&for_expr.body);
        self.indent -= 1;
        self.writeln("}");
    }

    fn emit_pat_to_string(&self, pat: &syn::Pat) -> String {
        match pat {
            syn::Pat::Ident(pi) => pi.ident.to_string(),
            syn::Pat::Wild(_) => "_".to_string(),
            syn::Pat::Tuple(pt) => {
                let elems: Vec<String> = pt
                    .elems
                    .iter()
                    .map(|p| self.emit_pat_to_string(p))
                    .collect();
                format!("[{}]", elems.join(", "))
            }
            syn::Pat::Reference(pr) => self.emit_pat_to_string(&pr.pat),
            _ => "/* TODO: pattern */".to_string(),
        }
    }

    fn emit_local(&mut self, local: &syn::Local) {
        let pat = &local.pat;
        self.register_local_binding_pattern(pat);

        match pat {
            syn::Pat::Ident(pat_ident) => {
                let name = &pat_ident.ident;
                let name_str = name.to_string();
                let cpp_name = self.allocate_local_cpp_name(&name_str);
                let is_mut = pat_ident.mutability.is_some();
                let inferred_binding_ty = local
                    .init
                    .as_ref()
                    .and_then(|init| self.infer_local_binding_type_from_initializer(&init.expr));
                if let Some(ty) = inferred_binding_ty.clone() {
                    self.update_local_binding_type(name_str.clone(), ty);
                }

                let type_str = if let Some(ty) = get_local_type(local) {
                    self.map_type(ty)
                } else if self.should_emit_inferred_sum_type_for_local(
                    local,
                    &name_str,
                    inferred_binding_ty.as_ref(),
                ) {
                    self.map_type(
                        inferred_binding_ty
                            .as_ref()
                            .expect("inferred sum type should exist when selected"),
                    )
                } else {
                    "auto".to_string()
                };

                let is_consumed = self.consuming_method_receiver_vars.contains(&name_str);
                let qualifier = if is_mut || is_consumed { "" } else { "const " };

                if let Some(init) = &local.init {
                    // Special case: `let x = loop { ... break val; }` → lambda wrapper
                    if let syn::Expr::Loop(loop_expr) = init.expr.as_ref() {
                        self.writeln(&format!(
                            "{}{} {} = [&]() {{",
                            qualifier, type_str, cpp_name
                        ));
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
                        if is_mut && self.reassigned_vars.contains(&name_str) {
                            // Reference rebinding detected: `let mut r = &x; ... r = &y;`
                            // Emit as pointer instead of reference
                            let ptr_type = self.map_ref_as_pointer_type(local, &init.expr);
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
                        let recovered_hints =
                            self.recover_constructor_template_hints_from_expr(&init.expr);
                        let pushed_hints = !recovered_hints.is_empty();
                        if pushed_hints {
                            self.constructor_template_hints.push(recovered_hints);
                        }
                        let expr_str = if get_local_type(local).is_none() {
                            if let syn::Expr::Repeat(repeat) = init.expr.as_ref() {
                                if let Some(elem_hint) = self.repeat_elem_type_hints.get(&name_str)
                                {
                                    self.emit_repeat_expr_with_element_hint(repeat, elem_hint)
                                } else if let Some(ty) = inferred_binding_ty.as_ref() {
                                    self.emit_expr_to_string_with_expected(&init.expr, Some(ty))
                                } else {
                                    self.emit_expr_maybe_move(&init.expr)
                                }
                            } else if let Some(ty) = inferred_binding_ty.as_ref() {
                                self.emit_expr_to_string_with_expected(&init.expr, Some(ty))
                            } else {
                                self.emit_expr_maybe_move(&init.expr)
                            }
                        } else if let Some(ty) = inferred_binding_ty.as_ref() {
                            self.emit_expr_to_string_with_expected(&init.expr, Some(ty))
                        } else {
                            self.emit_expr_maybe_move(&init.expr)
                        };
                        if pushed_hints {
                            self.constructor_template_hints.pop();
                        }
                        self.writeln(&format!(
                            "{}{} {} = {};",
                            qualifier, type_str, cpp_name, expr_str
                        ));
                    }
                } else {
                    self.writeln(&format!("{}{} {};", qualifier, type_str, cpp_name));
                }
            }
            syn::Pat::Tuple(tuple) => {
                // let (a, b) = expr; → auto [a, b] = expr;
                let names: Vec<String> = tuple
                    .elems
                    .iter()
                    .map(|p| self.emit_pat_to_string(p))
                    .collect();
                if let Some(init) = &local.init {
                    let expr_str = self.emit_expr_to_string(&init.expr);
                    self.writeln(&format!("auto [{}] = {};", names.join(", "), expr_str));
                }
            }
            syn::Pat::Type(pat_type) => {
                // let x: Type = expr;
                // The type annotation is on the pattern, inner pat has the name
                if let syn::Pat::Ident(pi) = pat_type.pat.as_ref() {
                    let name = &pi.ident;
                    let cpp_name = self.allocate_local_cpp_name(&name.to_string());
                    let is_mut = pi.mutability.is_some();
                    let ty = self.map_type(&pat_type.ty);
                    let is_consumed = self
                        .consuming_method_receiver_vars
                        .contains(&name.to_string());
                    let qualifier = if is_mut || is_consumed { "" } else { "const " };
                    if let Some(init) = &local.init {
                        let expr_str =
                            self.emit_expr_to_string_with_expected(&init.expr, Some(&pat_type.ty));
                        self.writeln(&format!("{}{} {} = {};", qualifier, ty, cpp_name, expr_str));
                    } else {
                        self.writeln(&format!("{}{} {};", qualifier, ty, cpp_name));
                    }
                } else {
                    self.writeln("// TODO: complex typed pattern binding");
                }
            }
            _ => {
                self.writeln("// TODO: complex pattern binding");
            }
        }
    }

    fn emit_repeat_expr_with_element_hint(
        &self,
        repeat: &syn::ExprRepeat,
        elem_hint: &syn::Type,
    ) -> String {
        let val = self.emit_expr_to_string(&repeat.expr);
        let len = self.emit_expr_to_string(&repeat.len);
        let elem_ty = self.map_type(elem_hint);
        format!(
            "rusty::array_repeat(static_cast<{}>({}), {})",
            elem_ty, val, len
        )
    }

    /// Register local bindings in the current scope for expected-type lookup.
    fn register_local_binding_pattern(&mut self, pat: &syn::Pat) {
        match pat {
            syn::Pat::Ident(pi) => {
                self.register_local_binding(pi.ident.to_string(), None);
            }
            syn::Pat::Type(pt) => {
                if let syn::Pat::Ident(pi) = pt.pat.as_ref() {
                    self.register_local_binding(pi.ident.to_string(), Some((*pt.ty).clone()));
                } else {
                    self.register_local_binding_pattern(&pt.pat);
                }
            }
            syn::Pat::Tuple(tuple) => {
                for elem in &tuple.elems {
                    self.register_local_binding_pattern(elem);
                }
            }
            syn::Pat::Reference(r) => {
                self.register_local_binding_pattern(&r.pat);
            }
            _ => {}
        }
    }

    fn register_local_binding(&mut self, name: String, ty: Option<syn::Type>) {
        if let Some(scope) = self.local_bindings.last_mut() {
            scope.insert(name, ty);
        }
    }

    fn allocate_local_cpp_name(&mut self, rust_name: &str) -> String {
        let Some(scope) = self.local_cpp_bindings.last_mut() else {
            return rust_name.to_string();
        };

        if !scope.contains_key(rust_name) && !scope.values().any(|v| v == rust_name) {
            let cpp_name = rust_name.to_string();
            scope.insert(rust_name.to_string(), cpp_name.clone());
            return cpp_name;
        }

        let mut idx = 1usize;
        loop {
            let candidate = format!("{}_shadow{}", rust_name, idx);
            if !scope.values().any(|v| v == &candidate) {
                scope.insert(rust_name.to_string(), candidate.clone());
                return candidate;
            }
            idx += 1;
        }
    }

    fn lookup_local_binding_cpp_name(&self, rust_name: &str) -> Option<String> {
        self.local_cpp_bindings
            .iter()
            .rev()
            .find_map(|scope| scope.get(rust_name).cloned())
    }

    fn update_local_binding_type(&mut self, name: String, ty: syn::Type) {
        for scope in self.local_bindings.iter_mut().rev() {
            if let Some(slot) = scope.get_mut(&name) {
                *slot = Some(ty);
                return;
            }
        }
    }

    fn infer_local_binding_type_from_initializer(
        &self,
        init_expr: &syn::Expr,
    ) -> Option<syn::Type> {
        let expr = self.extract_value_expr(init_expr)?;
        match expr {
            syn::Expr::Closure(closure) => match &closure.output {
                syn::ReturnType::Type(_, ty) => Some((**ty).clone()),
                syn::ReturnType::Default => None,
            },
            syn::Expr::Call(call) => {
                if let Some((ctor_name, ctor_arg)) = self.extract_constructor_call_expr(expr) {
                    let arg_ty = self.infer_simple_expr_type(ctor_arg)?;
                    match ctor_name.as_str() {
                        "Left" | "Right" => {
                            let left = arg_ty.clone();
                            let right = arg_ty;
                            Some(parse_quote!(Either<#left, #right>))
                        }
                        "Ok" | "Err" => {
                            let ok_ty = arg_ty.clone();
                            let err_ty = arg_ty;
                            Some(parse_quote!(Result<#ok_ty, #err_ty>))
                        }
                        _ => None,
                    }
                } else if call.args.is_empty() {
                    if let syn::Expr::Path(path) = call.func.as_ref() {
                        if path.path.segments.len() == 1 {
                            let name = path.path.segments[0].ident.to_string();
                            return self.lookup_local_binding_type(&name);
                        }
                    }
                    None
                } else {
                    None
                }
            }
            syn::Expr::If(if_expr) => self.infer_constructor_expected_type_from_if(if_expr),
            syn::Expr::Match(match_expr) => {
                self.infer_constructor_expected_type_from_match(match_expr)
            }
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                let name = path.path.segments[0].ident.to_string();
                self.lookup_local_binding_type(&name)
            }
            _ => None,
        }
    }

    fn should_emit_inferred_sum_type_for_local(
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

    fn infer_constructor_expected_type_from_if(&self, if_expr: &syn::ExprIf) -> Option<syn::Type> {
        let then_expr = self.extract_single_expr_from_block(&if_expr.then_branch)?;
        let (_, else_expr) = if_expr.else_branch.as_ref()?;
        let else_expr = self.extract_value_expr(else_expr)?;
        self.infer_constructor_expected_type_from_pair(then_expr, else_expr)
    }

    fn infer_constructor_expected_type_from_match(
        &self,
        match_expr: &syn::ExprMatch,
    ) -> Option<syn::Type> {
        let mut left_arg_ty: Option<syn::Type> = None;
        let mut right_arg_ty: Option<syn::Type> = None;
        let mut ok_arg_ty: Option<syn::Type> = None;
        let mut err_arg_ty: Option<syn::Type> = None;

        for arm in &match_expr.arms {
            let body_expr = self.extract_value_expr(&arm.body)?;
            if let Some((ctor_name, ctor_arg)) = self.extract_constructor_call_expr(body_expr) {
                let arg_ty = self.infer_simple_expr_type(ctor_arg)?;
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
            return Some(parse_quote!(Either<#left, #right>));
        }
        if let (Some(ok_ty), Some(err_ty)) = (ok_arg_ty, err_arg_ty) {
            return Some(parse_quote!(Result<#ok_ty, #err_ty>));
        }
        None
    }

    fn infer_constructor_expected_type_from_pair(
        &self,
        lhs: &syn::Expr,
        rhs: &syn::Expr,
    ) -> Option<syn::Type> {
        let (lhs_ctor, lhs_arg) = self.extract_constructor_call_expr(lhs)?;
        let (rhs_ctor, rhs_arg) = self.extract_constructor_call_expr(rhs)?;
        let lhs_ty = self.infer_simple_expr_type(lhs_arg)?;
        let rhs_ty = self.infer_simple_expr_type(rhs_arg)?;

        match (lhs_ctor.as_str(), rhs_ctor.as_str()) {
            ("Left", "Right") => Some(parse_quote!(Either<#lhs_ty, #rhs_ty>)),
            ("Right", "Left") => Some(parse_quote!(Either<#rhs_ty, #lhs_ty>)),
            ("Ok", "Err") => Some(parse_quote!(Result<#lhs_ty, #rhs_ty>)),
            ("Err", "Ok") => Some(parse_quote!(Result<#rhs_ty, #lhs_ty>)),
            _ => None,
        }
    }

    fn infer_simple_expr_type(&self, expr: &syn::Expr) -> Option<syn::Type> {
        let expr = self.extract_value_expr(expr)?;
        match expr {
            syn::Expr::Lit(lit) => self.infer_literal_type(&lit.lit),
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                let name = path.path.segments[0].ident.to_string();
                self.lookup_local_binding_type(&name)
            }
            syn::Expr::Range(range) => self.infer_range_expr_type(range),
            syn::Expr::Tuple(tuple) => {
                let mut elems = Vec::new();
                for elem in &tuple.elems {
                    elems.push(self.infer_simple_expr_type(elem)?);
                }
                Some(parse_quote!((#(#elems),*)))
            }
            syn::Expr::Call(call) => {
                if call.args.is_empty() {
                    if let syn::Expr::Path(path) = call.func.as_ref() {
                        if path.path.segments.len() == 1 {
                            let name = path.path.segments[0].ident.to_string();
                            if let Some(ty) = self.lookup_local_binding_type(&name) {
                                return Some(ty);
                            }
                        }
                    }
                }
                self.infer_local_binding_type_from_initializer(expr)
            }
            _ => None,
        }
    }

    fn infer_range_expr_type(&self, range: &syn::ExprRange) -> Option<syn::Type> {
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

    fn infer_literal_type(&self, lit: &syn::Lit) -> Option<syn::Type> {
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

    fn recover_constructor_template_hints_from_expr(
        &self,
        expr: &syn::Expr,
    ) -> HashMap<String, Vec<String>> {
        let mut hints = HashMap::new();
        let Some(expr) = self.extract_value_expr(expr) else {
            return hints;
        };
        let if_let_unwrap_method = self.expr_if_let_unwrap_method(expr);

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
        }

        hints
    }

    fn collect_constructor_arg_cpp_strings(
        &self,
        expr: &syn::Expr,
        out: &mut HashMap<String, String>,
        if_let_unwrap_method: Option<&'static str>,
    ) {
        let Some(expr) = self.extract_value_expr(expr) else {
            return;
        };

        if let Some((ctor_name, ctor_arg)) = self.extract_constructor_call_expr(expr) {
            out.entry(ctor_name).or_insert_with(|| {
                self.emit_constructor_hint_arg_cpp(ctor_arg, if_let_unwrap_method)
            });
        }

        match expr {
            syn::Expr::If(if_expr) => {
                if let Some(then_expr) = self.extract_single_expr_from_block(&if_expr.then_branch) {
                    self.collect_constructor_arg_cpp_strings(then_expr, out, if_let_unwrap_method);
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
                for arg in &call.args {
                    self.collect_constructor_arg_cpp_strings(arg, out, if_let_unwrap_method);
                }
            }
            _ => {}
        }
    }

    fn expr_if_let_unwrap_method(&self, expr: &syn::Expr) -> Option<&'static str> {
        match expr {
            syn::Expr::If(if_expr) => {
                if matches!(if_expr.cond.as_ref(), syn::Expr::Let(_)) {
                    if let syn::Expr::Let(let_expr) = if_expr.cond.as_ref() {
                        if let Some((_, _, unwrap_method)) =
                            self.if_let_expr_condition_parts(let_expr)
                        {
                            if unwrap_method.is_some() {
                                return unwrap_method;
                            }
                        }
                    }
                }
                for stmt in &if_expr.then_branch.stmts {
                    match stmt {
                        syn::Stmt::Expr(e, _) => {
                            if let Some(method) = self.expr_if_let_unwrap_method(e) {
                                return Some(method);
                            }
                        }
                        syn::Stmt::Local(local) => {
                            if let Some(init) = &local.init {
                                if let Some(method) = self.expr_if_let_unwrap_method(&init.expr) {
                                    return Some(method);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    return self.expr_if_let_unwrap_method(else_expr);
                }
                None
            }
            syn::Expr::Block(block_expr) => {
                for stmt in &block_expr.block.stmts {
                    match stmt {
                        syn::Stmt::Expr(e, _) => {
                            if let Some(method) = self.expr_if_let_unwrap_method(e) {
                                return Some(method);
                            }
                        }
                        syn::Stmt::Local(local) => {
                            if let Some(init) = &local.init {
                                if let Some(method) = self.expr_if_let_unwrap_method(&init.expr) {
                                    return Some(method);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                None
            }
            syn::Expr::Match(match_expr) => match_expr
                .arms
                .iter()
                .find_map(|arm| self.expr_if_let_unwrap_method(&arm.body)),
            syn::Expr::Paren(p) => self.expr_if_let_unwrap_method(&p.expr),
            syn::Expr::Group(g) => self.expr_if_let_unwrap_method(&g.expr),
            syn::Expr::Call(call) => self.expr_if_let_unwrap_method(&call.func).or_else(|| {
                call.args
                    .iter()
                    .find_map(|arg| self.expr_if_let_unwrap_method(arg))
            }),
            syn::Expr::Reference(r) => self.expr_if_let_unwrap_method(&r.expr),
            _ => None,
        }
    }

    fn emit_constructor_hint_arg_cpp(
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
                        return format!("_iflet.{}()", unwrap_method);
                    }
                }
            }
        }
        self.emit_expr_maybe_move(arg)
    }

    fn extract_constructor_pair_from_if<'a>(
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

    fn extract_constructor_pair_from_match<'a>(
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

    fn insert_constructor_pair_hints(
        &self,
        hints: &mut HashMap<String, Vec<String>>,
        lhs_name: &str,
        lhs_arg: &syn::Expr,
        rhs_name: &str,
        rhs_arg: &syn::Expr,
        if_let_unwrap_method: Option<&'static str>,
    ) {
        let lhs_cpp = self.emit_constructor_hint_arg_cpp(lhs_arg, if_let_unwrap_method);
        let rhs_cpp = self.emit_constructor_hint_arg_cpp(rhs_arg, if_let_unwrap_method);
        let lhs_ty = format!("decltype(({}))", lhs_cpp);
        let rhs_ty = format!("decltype(({}))", rhs_cpp);

        match (lhs_name, rhs_name) {
            ("Left", "Right") => {
                let args = vec![lhs_ty.clone(), rhs_ty.clone()];
                hints.insert("Left".to_string(), args.clone());
                hints.insert("Right".to_string(), args);
            }
            ("Right", "Left") => {
                let args = vec![rhs_ty.clone(), lhs_ty.clone()];
                hints.insert("Left".to_string(), args.clone());
                hints.insert("Right".to_string(), args);
            }
            ("Ok", "Err") => {
                let args = vec![lhs_ty.clone(), rhs_ty.clone()];
                hints.insert("Ok".to_string(), args.clone());
                hints.insert("Err".to_string(), args);
            }
            ("Err", "Ok") => {
                let args = vec![rhs_ty.clone(), lhs_ty.clone()];
                hints.insert("Ok".to_string(), args.clone());
                hints.insert("Err".to_string(), args);
            }
            _ => {}
        }
    }

    fn extract_value_expr<'a>(&self, expr: &'a syn::Expr) -> Option<&'a syn::Expr> {
        match expr {
            syn::Expr::Group(g) => self.extract_value_expr(&g.expr),
            syn::Expr::Paren(p) => self.extract_value_expr(&p.expr),
            syn::Expr::Block(block_expr) => self.extract_single_expr_from_block(&block_expr.block),
            _ => Some(expr),
        }
    }

    fn extract_single_expr_from_block<'a>(&self, block: &'a syn::Block) -> Option<&'a syn::Expr> {
        if block.stmts.len() != 1 {
            return None;
        }
        match &block.stmts[0] {
            syn::Stmt::Expr(expr, None) => Some(expr),
            _ => None,
        }
    }

    fn extract_single_value_expr<'a>(&self, expr: &'a syn::Expr) -> Option<&'a syn::Expr> {
        match expr {
            syn::Expr::Block(block) => self.extract_single_expr_from_block(&block.block),
            _ => self.extract_value_expr(expr),
        }
    }

    fn extract_constructor_call_expr<'a>(
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
        if func_path.segments.len() != 1 {
            return None;
        }
        let ctor_name = func_path.segments[0].ident.to_string();
        if !matches!(ctor_name.as_str(), "Left" | "Right" | "Ok" | "Err") {
            return None;
        }
        Some((ctor_name, &call.args[0]))
    }

    fn infer_expected_type_from_tuple_elements(
        &self,
        elems: &syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>,
    ) -> Option<syn::Type> {
        elems
            .iter()
            .find_map(|elem| self.infer_expected_type_from_tuple_element(elem))
    }

    fn infer_expected_type_from_tuple_element(&self, elem: &syn::Expr) -> Option<syn::Type> {
        match elem {
            syn::Expr::Reference(r) => self.infer_expected_type_from_tuple_element(&r.expr),
            syn::Expr::Paren(p) => self.infer_expected_type_from_tuple_element(&p.expr),
            syn::Expr::Group(g) => self.infer_expected_type_from_tuple_element(&g.expr),
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

    fn expected_data_enum_name(&self, ty: &syn::Type) -> Option<String> {
        match ty {
            syn::Type::Path(tp) => {
                let last = tp.path.segments.last()?;
                let name = if last.ident == "Self" {
                    self.current_struct.clone()?
                } else {
                    last.ident.to_string()
                };
                if self.data_enum_types.contains(&name) {
                    Some(name)
                } else {
                    None
                }
            }
            syn::Type::Reference(r) => self.expected_data_enum_name(&r.elem),
            syn::Type::Paren(p) => self.expected_data_enum_name(&p.elem),
            syn::Type::Group(g) => self.expected_data_enum_name(&g.elem),
            _ => None,
        }
    }

    fn maybe_wrap_variant_constructor_with_expected_enum(
        &self,
        expr: &syn::Expr,
        emitted: String,
        expected_ty: Option<&syn::Type>,
    ) -> String {
        let Some(ty) = expected_ty else {
            return emitted;
        };
        if self.expected_data_enum_name(ty).is_none() {
            return emitted;
        }
        let Some(value_expr) = self.extract_value_expr(expr) else {
            return emitted;
        };
        let syn::Expr::Call(call) = value_expr else {
            return emitted;
        };
        let syn::Expr::Path(path) = call.func.as_ref() else {
            return emitted;
        };
        if self.variant_ctor_name_from_path(&path.path).is_none() {
            return emitted;
        }
        let expected_cpp_ty = self.map_type(ty);
        format!("{}({})", expected_cpp_ty, emitted)
    }

    /// Look up the nearest in-scope local binding type for a variable name.
    fn lookup_local_binding_type(&self, name: &str) -> Option<syn::Type> {
        for scope in self.local_bindings.iter().rev() {
            if let Some(maybe_ty) = scope.get(name) {
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

    fn is_local_binding_in_scope(&self, name: &str) -> bool {
        self.local_bindings
            .iter()
            .rev()
            .any(|scope| scope.contains_key(name))
    }

    fn lookup_field_type_for_expr_base(
        &self,
        base: &syn::Expr,
        field_name: &str,
    ) -> Option<syn::Type> {
        match base {
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                let base_name = path.path.segments[0].ident.to_string();
                if base_name == "self" {
                    if let Some(struct_name) = &self.current_struct {
                        return self.lookup_struct_field_type(struct_name, field_name);
                    }
                    return None;
                }
                let base_ty = self.lookup_local_binding_type(&base_name)?;
                self.lookup_field_type_from_type(&base_ty, field_name)
            }
            syn::Expr::Paren(p) => self.lookup_field_type_for_expr_base(&p.expr, field_name),
            syn::Expr::Group(g) => self.lookup_field_type_for_expr_base(&g.expr, field_name),
            syn::Expr::Reference(r) => self.lookup_field_type_for_expr_base(&r.expr, field_name),
            _ => None,
        }
    }

    fn lookup_field_type_from_type(&self, ty: &syn::Type, field_name: &str) -> Option<syn::Type> {
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

    fn lookup_struct_field_type(&self, struct_name: &str, field_name: &str) -> Option<syn::Type> {
        self.struct_field_types
            .get(struct_name)
            .and_then(|fields| fields.get(field_name).cloned())
            .or_else(|| {
                let scoped = self.scoped_type_key(struct_name);
                self.struct_field_types
                    .get(&scoped)
                    .and_then(|fields| fields.get(field_name).cloned())
            })
    }

    fn push_param_bindings(
        &mut self,
        inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
    ) {
        let mut scope = HashMap::new();
        for input in inputs {
            if let syn::FnArg::Typed(pat_type) = input {
                if let syn::Pat::Ident(pi) = pat_type.pat.as_ref() {
                    scope.insert(pi.ident.to_string(), (*pat_type.ty).clone());
                }
            }
        }
        self.param_bindings.push(scope);
    }

    fn pop_param_bindings(&mut self) {
        self.param_bindings.pop();
    }

    fn push_self_receiver_ref_scope(
        &mut self,
        inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
    ) {
        let is_ref = matches!(
            inputs.first(),
            Some(syn::FnArg::Receiver(recv)) if recv.reference.is_some()
        );
        self.self_receiver_ref_scopes.push(is_ref);
    }

    fn pop_self_receiver_ref_scope(&mut self) {
        self.self_receiver_ref_scopes.pop();
    }

    fn current_self_receiver_is_reference(&self) -> bool {
        self.self_receiver_ref_scopes
            .last()
            .copied()
            .unwrap_or(false)
    }

    fn push_deref_method_scope(&mut self, enabled: bool) {
        self.deref_method_scopes.push(enabled);
    }

    fn pop_deref_method_scope(&mut self) {
        self.deref_method_scopes.pop();
    }

    fn in_deref_method_scope(&self) -> bool {
        self.deref_method_scopes.last().copied().unwrap_or(false)
    }

    fn push_deref_mut_method_scope(&mut self, enabled: bool) {
        self.deref_mut_method_scopes.push(enabled);
    }

    fn pop_deref_mut_method_scope(&mut self) {
        self.deref_mut_method_scopes.pop();
    }

    fn in_deref_mut_method_scope(&self) -> bool {
        self.deref_mut_method_scopes
            .last()
            .copied()
            .unwrap_or(false)
    }

    fn collect_pattern_ref_binding_names(&self, pat: &syn::Pat, out: &mut HashSet<String>) {
        match pat {
            syn::Pat::Ident(pi) => {
                if pi.by_ref.is_some() {
                    out.insert(pi.ident.to_string());
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

    fn push_pattern_ref_binding_scope(&mut self, pat: &syn::Pat) {
        let mut refs = HashSet::new();
        self.collect_pattern_ref_binding_names(pat, &mut refs);
        self.pattern_ref_bindings.push(refs);
    }

    fn pop_pattern_ref_binding_scope(&mut self) {
        self.pattern_ref_bindings.pop();
    }

    fn is_pattern_ref_binding_in_scope(&self, name: &str) -> bool {
        self.pattern_ref_bindings
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
    }

    fn is_expr_reference_like(&self, expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                let name = path.path.segments[0].ident.to_string();
                if name == "self" {
                    return self.current_self_receiver_is_reference();
                }
                if self.is_pattern_ref_binding_in_scope(&name) {
                    return true;
                }
                self.lookup_local_binding_type(&name)
                    .is_some_and(|ty| matches!(ty, syn::Type::Reference(_)))
            }
            syn::Expr::Paren(p) => self.is_expr_reference_like(&p.expr),
            syn::Expr::Group(g) => self.is_expr_reference_like(&g.expr),
            syn::Expr::Reference(_) => true,
            _ => false,
        }
    }

    fn should_collapse_reborrow_of_deref_operand(&self, operand: &syn::Expr) -> bool {
        if self.is_expr_reference_like(operand) {
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
            syn::Expr::Paren(p) => self.should_collapse_reborrow_of_deref_operand(&p.expr),
            syn::Expr::Group(g) => self.should_collapse_reborrow_of_deref_operand(&g.expr),
            _ => false,
        }
    }

    fn peel_paren_group_expr<'a>(&self, mut expr: &'a syn::Expr) -> &'a syn::Expr {
        loop {
            match expr {
                syn::Expr::Paren(p) => expr = &p.expr,
                syn::Expr::Group(g) => expr = &g.expr,
                _ => return expr,
            }
        }
    }

    /// Emit an expression with optional expected type context from its parent.
    /// Currently used for typed `let` initializers to guide enum variant constructor calls.
    fn emit_expr_to_string_with_expected(
        &self,
        expr: &syn::Expr,
        expected_ty: Option<&syn::Type>,
    ) -> String {
        match expr {
            syn::Expr::Call(call) => self.emit_call_expr_to_string(call, expected_ty),
            syn::Expr::Match(match_expr) => self.emit_match_expr_to_string(match_expr, expected_ty),
            syn::Expr::Binary(bin) => {
                self.emit_binary_expr_to_string_with_expected(bin, expected_ty)
            }
            syn::Expr::Reference(r) => {
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
                        && self.should_collapse_reborrow_of_deref_operand(&un.expr)
                    {
                        return self.emit_expr_to_string_with_expected(ref_inner, expected_ty);
                    }
                }
                let inner = self.emit_expr_to_string_with_expected(&r.expr, expected_ty);
                format!("&{}", inner)
            }
            syn::Expr::Paren(p) => {
                let inner = self.emit_expr_to_string_with_expected(&p.expr, expected_ty);
                format!("({})", inner)
            }
            syn::Expr::Group(g) => self.emit_expr_to_string_with_expected(&g.expr, expected_ty),
            syn::Expr::Tuple(tup) => {
                let elems: Vec<String> = tup
                    .elems
                    .iter()
                    .map(|e| self.emit_expr_to_string_with_expected(e, expected_ty))
                    .collect();
                format!("std::make_tuple({})", elems.join(", "))
            }
            syn::Expr::Path(path)
                if path.path.segments.len() == 1 && path.path.segments[0].ident == "None" =>
            {
                if let Some(inner_cpp) = self.option_ctor_inner_cpp_type(expected_ty) {
                    return format!("rusty::Option<{}>(rusty::None)", inner_cpp);
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
            _ => self.emit_expr_to_string(expr),
        }
    }

    fn emit_binary_expr_to_string_with_expected(
        &self,
        bin: &syn::ExprBinary,
        _expected_ty: Option<&syn::Type>,
    ) -> String {
        match &bin.op {
            // Logical operators always require boolean operands.
            // Thread `bool` into both sides so nested match fallbacks emit typed
            // unreachable lambdas compatible with std::visit return unification.
            syn::BinOp::And(_) | syn::BinOp::Or(_) => {
                let bool_ty: syn::Type = parse_quote!(bool);
                let left = self.emit_expr_to_string_with_expected(&bin.left, Some(&bool_ty));
                let op = self.emit_binop(&bin.op);
                let right = self.emit_expr_to_string_with_expected(&bin.right, Some(&bool_ty));
                format!("{} {} {}", left, op, right)
            }
            _ => {
                let left = self.emit_expr_to_string(&bin.left);
                let op = self.emit_binop(&bin.op);
                let right = self.emit_expr_to_string(&bin.right);
                format!("{} {} {}", left, op, right)
            }
        }
    }

    fn option_ctor_inner_cpp_type(&self, expected_ty: Option<&syn::Type>) -> Option<String> {
        let ty = expected_ty?;
        let syn::Type::Path(tp) = ty else {
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
            syn::GenericArgument::Type(t) => Some(t),
            _ => None,
        })?;

        let is_dependent_assoc_inner = self.type_contains_dependent_assoc(inner_ty)
            || self.type_references_current_struct_assoc(inner_ty);
        // In constrained modes (module/expanded libtest), associated projections
        // like `Self::Item` are intentionally softened/skipped at declaration time.
        // Avoid reintroducing them through explicit Option ctor typing in value position.
        if self.should_soften_dependent_assoc_mode() && is_dependent_assoc_inner {
            return None;
        }

        let needs_explicit_ctor = matches!(inner_ty, syn::Type::Reference(_))
            || self.type_mentions_in_scope_type_param(inner_ty)
            || is_dependent_assoc_inner;
        if !needs_explicit_ctor {
            return None;
        }
        Some(self.map_type(inner_ty))
    }

    /// Emit a call expression, optionally using expected type context from parent.
    fn emit_call_expr_to_string(
        &self,
        call: &syn::ExprCall,
        expected_ty: Option<&syn::Type>,
    ) -> String {
        if let Some(ty) = expected_ty {
            if let Some(emitted) = self.try_emit_iter_either_new_call_with_expected(call, ty) {
                return emitted;
            }
            if let Some(emitted) = self.try_emit_variant_constructor_call_with_expected(call, ty) {
                return emitted;
            }
        }
        if expected_ty.is_none() {
            if let Some(emitted) = self.try_emit_variant_constructor_call_with_recovered_hints(call)
            {
                return emitted;
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

        let func = self.emit_expr_to_string(&call.func);
        if let Some(expected) = expected_ty {
            if self.is_noreturn_panic_like_call_path(&func) {
                let args: Vec<String> = call
                    .args
                    .iter()
                    .map(|a| self.emit_expr_maybe_move(a))
                    .collect();
                let expected_cpp = self.map_type(expected);
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

        // Map Rust Option::Some(x) → std::optional{x}
        if func == "Some" && call.args.len() == 1 {
            if let Some(inner_cpp) = self.option_ctor_inner_cpp_type(expected_ty) {
                let arg = self.emit_expr_to_string(&call.args[0]);
                return format!("rusty::Option<{}>({})", inner_cpp, arg);
            }
            if let Some(ref_arg) = self.emit_some_ref_constructor_arg(&call.args[0]) {
                return format!("rusty::SomeRef({})", ref_arg);
            }
            let arg = self.emit_expr_to_string(&call.args[0]);
            return format!("std::make_optional({})", arg);
        }
        // Map Rust conversion shim in expanded output.
        if self.is_core_from_path_expr(call.func.as_ref()) && call.args.len() == 1 {
            if let Some(expected) = expected_ty {
                let expected_cpp = self.map_type(expected);
                return self.emit_from_conversion_to_target(&call.args[0], &expected_cpp);
            }
            return self.emit_expr_maybe_move(&call.args[0]);
        }
        // Map Ok(x) and Err(x) for Result
        if func == "Ok" && call.args.len() == 1 {
            let arg = self.emit_expr_to_string(&call.args[0]);
            if let Some(expected) = expected_ty {
                let expected_cpp = self.map_type(expected);
                if expected_cpp.starts_with("rusty::Result<") {
                    return format!("{}::Ok({})", expected_cpp, arg);
                }
            }
            if let Some(args) = self.lookup_constructor_template_args("Ok") {
                return format!("rusty::Result<{}, {}>::Ok({})", args[0], args[1], arg);
            }
            return format!("Ok({})", arg);
        }
        if func == "Err" && call.args.len() == 1 {
            let arg = self.emit_expr_to_string(&call.args[0]);
            if let Some(expected) = expected_ty {
                let expected_cpp = self.map_type(expected);
                if expected_cpp.starts_with("rusty::Result<") {
                    return format!("{}::Err({})", expected_cpp, arg);
                }
            }
            if let Some(args) = self.lookup_constructor_template_args("Err") {
                return format!("rusty::Result<{}, {}>::Err({})", args[0], args[1], arg);
            }
            return format!("Err({})", arg);
        }

        let args: Vec<String> = call
            .args
            .iter()
            .map(|a| self.emit_expr_maybe_move(a))
            .collect();
        format!("{}({})", func, args.join(", "))
    }

    fn emit_cursor_new_arg_expr(&self, arg: &syn::Expr) -> String {
        let peeled = self.peel_paren_group_expr(arg);
        if let syn::Expr::Array(arr) = peeled {
            // Expanded io tests use `Cursor::new([])`. Keep this path concrete
            // instead of falling back to `unreachable()`.
            if arr.elems.is_empty() {
                return "rusty::array_repeat(static_cast<uint8_t>(0), 0)".to_string();
            }
        }
        self.emit_expr_maybe_move(arg)
    }

    fn emit_some_ref_constructor_arg(&self, arg: &syn::Expr) -> Option<String> {
        let syn::Expr::Reference(r) = arg else {
            return None;
        };
        let inner = self.emit_expr_to_string(&r.expr);
        if self.is_stable_reference_lvalue_expr(&r.expr) {
            return Some(inner);
        }
        if r.mutability.is_none() {
            // `Some(&<rvalue>)` appears in expanded assertions (`&2`, etc.).
            // Materialize a static object and return a stable const reference.
            return Some(format!(
                "[&]() -> const auto& {{ static const auto _some_ref_tmp = {}; return _some_ref_tmp; }}()",
                inner
            ));
        }
        // Mutable references to rvalues need a stable storage target as well.
        Some(format!(
            "[&]() -> auto& {{ static auto _some_mut_ref_tmp = {}; return _some_mut_ref_tmp; }}()",
            inner
        ))
    }

    fn variant_ctor_name_from_path(&self, path: &syn::Path) -> Option<String> {
        if path.segments.is_empty() {
            return None;
        }
        let last = path.segments.last()?.ident.to_string();
        if !matches!(last.as_str(), "Left" | "Right") {
            return None;
        }
        if path.segments.len() == 1 {
            return Some(last);
        }
        let first = path.segments.first()?.ident.to_string();
        if matches!(first.as_str(), "crate" | "self" | "super") {
            return Some(last);
        }
        None
    }

    fn is_string_from_call_expr(&self, expr: &syn::Expr) -> bool {
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

    fn is_core_from_path_expr(&self, expr: &syn::Expr) -> bool {
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
        joined == "core::convert::From::from" || joined == "std::convert::From::from"
    }

    fn is_noreturn_panic_like_call_path(&self, path: &str) -> bool {
        matches!(
            path,
            "rusty::panicking::panic"
                | "rusty::panicking::panic_fmt"
                | "rusty::panicking::assert_failed"
                | "rusty::intrinsics::unreachable"
        )
    }

    fn expr_is_noreturn_panic_like(&self, expr: &syn::Expr) -> bool {
        match self.peel_paren_group_expr(expr) {
            syn::Expr::Call(call) => {
                let func = self.emit_expr_to_string(&call.func);
                self.is_noreturn_panic_like_call_path(&func)
            }
            _ => false,
        }
    }

    fn method_call_single_turbofish_type<'a>(
        &self,
        mc: &'a syn::ExprMethodCall,
    ) -> Option<&'a syn::Type> {
        let turbofish = mc.turbofish.as_ref()?;
        if turbofish.args.len() != 1 {
            return None;
        }
        match turbofish.args.first()? {
            syn::GenericArgument::Type(ty) => Some(ty),
            _ => None,
        }
    }

    fn emit_from_conversion_to_target(&self, arg: &syn::Expr, target_cpp_ty: &str) -> String {
        let target_is_ref = target_cpp_ty.contains('&');
        let inner = match arg {
            syn::Expr::Call(call)
                if self.is_core_from_path_expr(call.func.as_ref()) && call.args.len() == 1 =>
            {
                if target_is_ref {
                    self.emit_expr_to_string(&call.args[0])
                } else {
                    self.emit_expr_maybe_move(&call.args[0])
                }
            }
            _ => {
                if target_is_ref {
                    self.emit_expr_to_string(arg)
                } else {
                    self.emit_expr_maybe_move(arg)
                }
            }
        };
        if target_cpp_ty == "rusty::String" {
            if self.is_string_from_call_expr(arg) || inner.starts_with("rusty::String::from(") {
                return inner;
            }
            return format!("rusty::String::from({})", inner);
        }
        inner
    }

    fn lookup_constructor_template_args(&self, ctor_name: &str) -> Option<Vec<String>> {
        for scope in self.constructor_template_hints.iter().rev() {
            if let Some(args) = scope.get(ctor_name) {
                if args.len() == 2 {
                    return Some(args.clone());
                }
            }
        }
        None
    }

    fn try_emit_variant_constructor_call_with_recovered_hints(
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

        let args = self.lookup_constructor_template_args(&ctor_name)?;
        let target_cpp_ty = if ctor_name == "Left" {
            args[0].as_str()
        } else {
            args[1].as_str()
        };
        let arg = self.emit_from_conversion_to_target(&call.args[0], target_cpp_ty);
        Some(format!("{}<{}, {}>({})", ctor_name, args[0], args[1], arg))
    }

    fn try_emit_variant_constructor_call_with_template_args(
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
        Some(format!(
            "{}<{}, {}>({})",
            ctor_name, template_args[0], template_args[1], arg
        ))
    }

    fn emit_expr_to_string_with_variant_ctx(
        &self,
        expr: &syn::Expr,
        variant_ctx: Option<&VariantTypeContext>,
    ) -> String {
        match expr {
            syn::Expr::Reference(r) => {
                let inner = self.emit_expr_to_string_with_variant_ctx(&r.expr, variant_ctx);
                format!("&{}", inner)
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

    /// Detect UFCS trait method calls in the form `Trait::method(&self, args...)`
    /// or `module::Trait::method(&mut self, args...)`.
    fn detect_ufcs_trait_method_call(&self, call: &syn::ExprCall) -> Option<UfcsTraitCallInfo> {
        let func_path = match call.func.as_ref() {
            syn::Expr::Path(path) => &path.path,
            _ => return None,
        };

        if func_path.segments.len() < 2 || call.args.is_empty() {
            return None;
        }

        // Heuristic guard: trait segment is typically UpperCamelCase.
        // This avoids rewriting regular namespaced free functions.
        let trait_segment = func_path.segments.iter().nth_back(1)?.ident.to_string();
        if !trait_segment.starts_with(|c: char| c.is_uppercase()) {
            return None;
        }

        // First argument must be an explicit reference: `&receiver` or `&mut receiver`.
        let receiver_ref = match call.args.first() {
            Some(syn::Expr::Reference(r)) => r,
            _ => return None,
        };

        let method_name = func_path.segments.last()?.ident.to_string();
        // Constructor-like calls (notably `Type::new(&...)`) are ordinary static
        // functions in Rust path space, not UFCS trait-method dispatch.
        if matches!(method_name.as_str(), "new" | "new_") {
            return None;
        }
        let function_path = func_path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        // Guard known mapped constructor paths (for example `io::Cursor::new`)
        // so they keep normal function-path lowering.
        if let Some(mapped) = types::map_function_path(&function_path) {
            if mapped.ends_with("::new_") {
                return None;
            }
        }

        Some(UfcsTraitCallInfo {
            function_path,
            method_name,
            receiver_is_mut: receiver_ref.mutability.is_some(),
            non_receiver_arg_count: call.args.len().saturating_sub(1),
        })
    }

    /// If this is a variant-constructor call like `Left(2)` and the expected type
    /// is known (e.g., `Either<i32, i32>`), emit explicit template args:
    /// `Left<int32_t, int32_t>(2)`.
    fn try_emit_variant_constructor_call_with_expected(
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

        let expected_args = self.expected_type_template_args(expected_ty)?;
        if expected_args.is_empty() {
            return None;
        }
        if !matches!(
            func_path.segments.last().map(|s| &s.arguments),
            Some(syn::PathArguments::None)
        ) {
            return None;
        }
        let target_cpp_ty = if ctor_name == "Left" {
            expected_args[0].as_str()
        } else {
            expected_args[1].as_str()
        };

        let args: Vec<String> = call
            .args
            .iter()
            .map(|a| self.emit_from_conversion_to_target(a, target_cpp_ty))
            .collect();

        Some(format!(
            "{}<{}>({})",
            ctor_name,
            expected_args.join(", "),
            args.join(", ")
        ))
    }

    /// If this is `IterEither::new_(...)` in expression position with a known
    /// expected return type, emit a fully-specialized static call:
    /// `iterator::IterEither<A, B>::new_(...)`.
    fn try_emit_iter_either_new_call_with_expected(
        &self,
        call: &syn::ExprCall,
        expected_ty: &syn::Type,
    ) -> Option<String> {
        let func = self.emit_expr_to_string(&call.func);
        if !func.ends_with("IterEither::new_") {
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

    /// Extract mapped template arguments from an expected type path.
    /// `Either<i32, i32>` -> `["int32_t", "int32_t"]`
    /// `Self` inside `impl<L, R> Either<L, R>` -> `["L", "R"]`
    fn expected_type_template_args(&self, expected_ty: &syn::Type) -> Option<Vec<String>> {
        match expected_ty {
            syn::Type::Path(tp) => self.expected_type_template_args_from_path(tp),
            syn::Type::Reference(r) => self.expected_type_template_args(&r.elem),
            syn::Type::Paren(p) => self.expected_type_template_args(&p.elem),
            syn::Type::Group(g) => self.expected_type_template_args(&g.elem),
            _ => None,
        }
    }

    fn expected_type_template_args_from_path(&self, tp: &syn::TypePath) -> Option<Vec<String>> {
        let last = tp.path.segments.last()?;
        if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
            let type_args: Vec<String> = args
                .args
                .iter()
                .filter_map(|arg| match arg {
                    syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                    _ => None,
                })
                .collect();

            if !type_args.is_empty() {
                return Some(type_args);
            }
        }

        let enum_name = if last.ident == "Self" {
            self.current_struct.clone()
        } else {
            Some(last.ident.to_string())
        };

        if let Some(enum_name) = enum_name.as_deref() {
            if let Some(params) = self.enum_type_params.get(enum_name) {
                if !params.is_empty() && params.iter().all(|p| self.is_type_param_in_scope(p)) {
                    return Some(params.clone());
                }
            }
        }

        None
    }

    fn emit_expr_to_string(&self, expr: &syn::Expr) -> String {
        match expr {
            syn::Expr::Lit(lit) => self.emit_lit(&lit.lit),
            syn::Expr::Path(path) => self.emit_expr_path_to_string(&path.path),
            syn::Expr::Group(group) => self.emit_expr_to_string(&group.expr),
            syn::Expr::Binary(bin) => {
                let left = self.emit_expr_to_string(&bin.left);
                let op = self.emit_binop(&bin.op);
                let right = self.emit_expr_to_string(&bin.right);
                format!("{} {} {}", left, op, right)
            }
            syn::Expr::Unary(un) => match un.op {
                syn::UnOp::Neg(_) => {
                    let operand = self.emit_expr_to_string(&un.expr);
                    format!("-{}", operand)
                }
                syn::UnOp::Not(_) => {
                    let operand = self.emit_expr_to_string(&un.expr);
                    format!("!{}", operand)
                }
                syn::UnOp::Deref(_) => {
                    if self.is_expr_reference_like(&un.expr) {
                        self.emit_expr_to_string(&un.expr)
                    } else {
                        let operand = self.emit_expr_to_string(&un.expr);
                        if self.in_deref_method_scope() {
                            format!("rusty::deref_ref({})", operand)
                        } else if self.in_deref_mut_method_scope() {
                            format!("rusty::deref_mut({})", operand)
                        } else {
                            format!("*{}", operand)
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
                    if matches!(un.op, syn::UnOp::Deref(_))
                        && self.should_collapse_reborrow_of_deref_operand(&un.expr)
                    {
                        return self.emit_expr_to_string(ref_inner);
                    }
                }
                let inner = self.emit_expr_to_string(&r.expr);
                format!("&{}", inner)
            }
            syn::Expr::Call(call) => self.emit_call_expr_to_string(call, None),
            syn::Expr::MethodCall(mc) => {
                if mc.method == "len" && mc.args.is_empty() {
                    let receiver = self.emit_expr_to_string(&mc.receiver);
                    return format!("rusty::len({})", receiver);
                }
                if let Some(description_call) = self.try_emit_error_description_dispatch_call(mc) {
                    return description_call;
                }
                if mc.method == "parse" && mc.args.is_empty() {
                    if let Some(parsed_ty) = self.method_call_single_turbofish_type(mc) {
                        let receiver = self.emit_expr_to_string(&mc.receiver);
                        return format!(
                            "rusty::str_runtime::parse<{}>({})",
                            self.map_type(parsed_ty),
                            receiver
                        );
                    }
                }
                if mc.method == "collect"
                    && mc.args.is_empty()
                    && Self::is_range_expression(&mc.receiver)
                {
                    let receiver = self.emit_expr_to_string(&mc.receiver);
                    return format!("rusty::collect_range({})", receiver);
                }
                if let Some(io_call) = self.try_emit_io_read_write_buffer_call(mc) {
                    return io_call;
                }
                let method = &mc.method;
                let args: Vec<String> = mc
                    .args
                    .iter()
                    .map(|a| self.emit_expr_maybe_move(a))
                    .collect();
                // Check if receiver is `self` — in C++ methods, call directly
                let is_self = matches!(mc.receiver.as_ref(), syn::Expr::Path(p)
                    if p.path.segments.len() == 1 && p.path.segments[0].ident == "self");
                if is_self {
                    format!("{}({})", method, args.join(", "))
                } else {
                    let receiver = self.emit_expr_to_string(&mc.receiver);
                    format!("{}.{}({})", receiver, method, args.join(", "))
                }
            }
            syn::Expr::Field(f) => {
                // Check if base is `self` — in C++ methods, fields are directly accessible
                let is_self = matches!(f.base.as_ref(), syn::Expr::Path(p)
                    if p.path.segments.len() == 1 && p.path.segments[0].ident == "self");
                if is_self {
                    match &f.member {
                        syn::Member::Named(ident) => ident.to_string(),
                        syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                    }
                } else {
                    let base = self.emit_expr_to_string(&f.base);
                    match &f.member {
                        syn::Member::Named(ident) => format!("{}.{}", base, ident),
                        syn::Member::Unnamed(idx) => format!("{}._{}", base, idx.index),
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
                    None => "break".to_string(),
                }
            }
            syn::Expr::Continue(_) => "continue".to_string(),
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
                        let val = self
                            .emit_expr_to_string_with_expected(e, self.current_return_type_hint());
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
                let left = self.emit_expr_to_string(&a.left);
                let expected_ty = if let syn::Expr::Path(path) = a.left.as_ref() {
                    if path.path.segments.len() == 1 {
                        let name = path.path.segments[0].ident.to_string();
                        self.lookup_local_binding_type(&name)
                    } else {
                        None
                    }
                } else {
                    None
                };
                let right = self.emit_expr_to_string_with_expected(&a.right, expected_ty.as_ref());
                format!("{} = {}", left, right)
            }
            syn::Expr::Struct(s) => {
                let name = self.emit_path_to_string(&s.path);
                let fields: Vec<String> = s
                    .fields
                    .iter()
                    .map(|f| {
                        let val = self.emit_expr_to_string(&f.expr);
                        let member_name = match &f.member {
                            syn::Member::Named(ident) => ident.to_string(),
                            syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                        };
                        format!(".{} = {}", member_name, val)
                    })
                    .collect();
                format!("{}{{{}}}", name, fields.join(", "))
            }
            syn::Expr::Paren(p) => {
                let inner = self.emit_expr_to_string(&p.expr);
                format!("({})", inner)
            }
            syn::Expr::Cast(c) => {
                let expr = self.emit_expr_to_string(&c.expr);
                let ty = self.map_type(&c.ty);
                format!("static_cast<{}>({})", ty, expr)
            }
            syn::Expr::Index(idx) => {
                if let Some(slice_expr) = self.try_emit_slice_index_expr_to_string(idx) {
                    return slice_expr;
                }
                let base = self.emit_expr_to_string(&idx.expr);
                let index = self.emit_expr_to_string(&idx.index);
                format!("{}[{}]", base, index)
            }
            syn::Expr::Tuple(tup) => {
                let tuple_expected_ty = self.infer_expected_type_from_tuple_elements(&tup.elems);
                let elems: Vec<String> = tup
                    .elems
                    .iter()
                    .map(|e| self.emit_expr_to_string_with_expected(e, tuple_expected_ty.as_ref()))
                    .collect();
                format!("std::make_tuple({})", elems.join(", "))
            }
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
                let try_macro = self.current_try_macro();
                format!("{}({})", try_macro, inner)
            }
            syn::Expr::Repeat(rep) => {
                // [val; N] → std::array filled with val
                let val = self.emit_expr_to_string(&rep.expr);
                let len = self.emit_expr_to_string(&rep.len);
                format!("rusty::array_repeat({}, {})", val, len)
            }
            _ => self.match_expr_unreachable_fallback().to_string(),
        }
    }

    fn emit_if_expr_to_string(
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
            return "/* TODO: if-expression */".to_string();
        };
        let cond = self.emit_expr_to_string(&if_expr.cond);
        let Some(then_expr) = self.extract_single_expr_from_block(&if_expr.then_branch) else {
            return "/* TODO: if-expression */".to_string();
        };
        let Some(else_expr) = self.extract_single_value_expr(else_branch) else {
            return "/* TODO: if-expression */".to_string();
        };

        let inferred_expected_ty = if expected_ty.is_none() {
            self.infer_constructor_expected_type_from_pair(then_expr, else_expr)
        } else {
            None
        };
        let expected_ty_for_branches = expected_ty.or(inferred_expected_ty.as_ref());
        let inferred_ctor_args = self.infer_variant_ctor_template_args_from_if(if_expr);
        let inferred_expected_cpp_ty = inferred_ctor_args
            .as_ref()
            .map(|args| format!("Either<{}, {}>", args[0], args[1]));
        let expected_cpp_ty = expected_ty_for_branches
            .and_then(|ty| {
                if self.expected_data_enum_name(ty).is_some() {
                    Some(self.map_type(ty))
                } else {
                    None
                }
            })
            .or(inferred_expected_cpp_ty);

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

    fn emit_if_let_expr_to_string(
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
        let (cond_expr, binding_name, unwrap_method) =
            self.if_let_expr_condition_parts(let_expr)?;

        let inferred_expected_ty = if expected_ty.is_none() {
            self.infer_constructor_expected_type_from_pair(then_expr, else_expr)
        } else {
            None
        };
        let branch_expected_ty = expected_ty.or(inferred_expected_ty.as_ref());
        let scrutinee = self.emit_expr_to_string(&let_expr.expr);

        let then_value = {
            let emitted = self.emit_expr_to_string_with_expected(then_expr, branch_expected_ty);
            if let Some(binding) = binding_name {
                if let Some(unwrap) = unwrap_method {
                    format!(
                        "([&]() {{ auto {} = _iflet.{}(); return {}; }}())",
                        binding, unwrap, emitted
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

    fn if_let_expr_condition_parts(
        &self,
        let_expr: &syn::ExprLet,
    ) -> Option<(String, Option<String>, Option<&'static str>)> {
        match &*let_expr.pat {
            syn::Pat::TupleStruct(ts) => {
                let path_str = ts
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                let (cond, unwrap) = match path_str.as_str() {
                    "Some" | "Option::Some" => ("_iflet.is_some()", Some("unwrap")),
                    "Ok" | "Result::Ok" => ("_iflet.is_ok()", Some("unwrap")),
                    "Err" | "Result::Err" => ("_iflet.is_err()", Some("unwrap_err")),
                    _ => return None,
                };
                let binding = if ts.elems.len() == 1 {
                    match ts.elems.first()? {
                        syn::Pat::Ident(pi) if pi.ident != "_" => Some(pi.ident.to_string()),
                        syn::Pat::Wild(_) => None,
                        _ => return None,
                    }
                } else if ts.elems.is_empty() {
                    None
                } else {
                    return None;
                };
                Some((cond.to_string(), binding, unwrap))
            }
            syn::Pat::Path(pp) => {
                let path_str = pp
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                let cond = match path_str.as_str() {
                    "None" | "Option::None" => "_iflet.is_none()",
                    _ => return None,
                };
                Some((cond.to_string(), None, None))
            }
            syn::Pat::Ident(pi) => {
                let binding = if pi.ident == "_" {
                    None
                } else {
                    Some(pi.ident.to_string())
                };
                Some(("true".to_string(), binding, None))
            }
            _ => None,
        }
    }

    fn emit_if_ternary_branch_expr(
        &self,
        branch_expr: &syn::Expr,
        expected_ty: Option<&syn::Type>,
        ctor_template_args: Option<&[String]>,
    ) -> String {
        let value_expr = self.extract_value_expr(branch_expr).unwrap_or(branch_expr);
        if expected_ty.is_none() {
            if let (Some(args), syn::Expr::Call(call)) = (ctor_template_args, value_expr) {
                if let Some(emitted) =
                    self.try_emit_variant_constructor_call_with_template_args(call, args)
                {
                    return emitted;
                }
            }
        }
        self.emit_expr_to_string_with_expected(value_expr, expected_ty)
    }

    fn maybe_wrap_variant_constructor_with_expected_cpp_type(
        &self,
        expr: &syn::Expr,
        emitted: String,
        expected_cpp_ty: Option<&str>,
    ) -> String {
        let Some(expected_cpp_ty) = expected_cpp_ty else {
            return emitted;
        };
        let Some(value_expr) = self.extract_value_expr(expr) else {
            return emitted;
        };
        let syn::Expr::Call(call) = value_expr else {
            return emitted;
        };
        let syn::Expr::Path(path) = call.func.as_ref() else {
            return emitted;
        };
        if self.variant_ctor_name_from_path(&path.path).is_none() {
            return emitted;
        }
        format!("{}({})", expected_cpp_ty, emitted)
    }

    fn infer_variant_ctor_template_args_from_if(
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

    fn try_emit_io_read_write_buffer_call(&self, mc: &syn::ExprMethodCall) -> Option<String> {
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
        let receiver_name = match self.peel_paren_group_expr(&mc.receiver) {
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                Some(path.path.segments[0].ident.to_string())
            }
            _ => None,
        };
        let receiver = self.emit_expr_to_string(&mc.receiver);
        let is_self = receiver_name.as_deref() == Some("self");
        let arg_expr = match mc.args.first()? {
            syn::Expr::Reference(arg_ref) => {
                self.emit_io_read_write_buffer_view_expr(&arg_ref.expr)
            }
            arg => self.emit_expr_maybe_move(arg),
        };

        // Leaf 4.39: expanded `for_both`/match-lowered io methods bind payload as `inner`
        // and can instantiate non-io branches. Route read/write through helper dispatch so
        // non-member payload branches (e.g. spans) compile and fall back deterministically.
        if matches!(method.as_str(), "read" | "write") && receiver_name.as_deref() == Some("inner")
        {
            return Some(format!("rusty::io::{}({}, {})", method, receiver, arg_expr));
        }

        // Existing normalization for by-reference buffer calls: `read(&buf)`/`write(&buf)` ->
        // `read(rusty::slice_full(buf))`/`write(rusty::slice_full(buf))`.
        if !matches!(mc.args.first()?, syn::Expr::Reference(_)) {
            return None;
        }
        if is_self {
            return Some(format!("{}({})", method, arg_expr));
        }
        Some(format!("{}.{}({})", receiver, method, arg_expr))
    }

    fn try_emit_error_description_dispatch_call(&self, mc: &syn::ExprMethodCall) -> Option<String> {
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

    fn emit_io_read_write_buffer_view_expr(&self, expr: &syn::Expr) -> String {
        let target = self.peel_reference_target_expr(expr);
        if self.is_slice_range_index_target_expr(target) {
            return self.emit_expr_to_string(target);
        }
        let target_expr = self.emit_expr_to_string(target);
        format!("rusty::slice_full({})", target_expr)
    }

    fn is_slice_range_index_expr(&self, index: &syn::Expr) -> bool {
        matches!(self.peel_paren_group_expr(index), syn::Expr::Range(_))
    }

    fn try_emit_slice_index_expr_to_string(&self, idx: &syn::ExprIndex) -> Option<String> {
        let range = match self.peel_paren_group_expr(&idx.index) {
            syn::Expr::Range(r) => r,
            _ => return None,
        };
        let base = self.emit_expr_to_string(&idx.expr);
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
            (None, None, _) => format!("rusty::slice_full({})", base),
        };
        Some(emitted)
    }

    fn is_range_expression(expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Range(_) => true,
            syn::Expr::Paren(p) => Self::is_range_expression(&p.expr),
            _ => false,
        }
    }

    fn emit_match_expr_to_string(
        &self,
        match_expr: &syn::ExprMatch,
        expected_ty: Option<&syn::Type>,
    ) -> String {
        if self.match_expr_has_explicit_return_arm(match_expr) {
            if let Some(lowered) = self.emit_try_style_either_match_expr(match_expr, expected_ty) {
                return lowered;
            }
        }
        let variant_ctx = self.infer_variant_type_context_from_expr(&match_expr.expr);
        if let Some(runtime_expr) =
            self.emit_runtime_match_expr(match_expr, variant_ctx.as_ref(), expected_ty)
        {
            return runtime_expr;
        }
        // Match as expression → immediately-invoked lambda
        if self.all_arms_are_switch_compatible(&match_expr.arms, variant_ctx.as_ref()) {
            let scrutinee =
                self.emit_expr_to_string_with_variant_ctx(&match_expr.expr, variant_ctx.as_ref());
            // Simple switch-like match → ternary chain or IIFE with switch
            format!(
                "[&]() {{ auto&& _m = {}; {} }}()",
                scrutinee,
                self.emit_match_expr_switch(&match_expr.arms, expected_ty)
            )
        } else if let syn::Expr::Tuple(tuple_scrutinee) = match_expr.expr.as_ref() {
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
            // Variant match → IIFE with std::visit
            format!(
                "[&]() {{ auto&& _m = {}; return std::visit(overloaded {{ {} }}, _m); }}()",
                scrutinee,
                self.emit_match_expr_visit(&match_expr.arms, variant_ctx.as_ref(), expected_ty)
            )
        }
    }

    fn match_expr_has_explicit_return_arm(&self, match_expr: &syn::ExprMatch) -> bool {
        match_expr.arms.iter().any(|arm| {
            self.extract_value_expr(&arm.body)
                .is_some_and(|expr| matches!(expr, syn::Expr::Return(_)))
        })
    }

    fn emit_try_style_either_match_expr(
        &self,
        match_expr: &syn::ExprMatch,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        if match_expr.arms.len() != 2 {
            return None;
        }

        let mut left_arm: Option<&syn::Arm> = None;
        let mut right_arm: Option<&syn::Arm> = None;
        for arm in &match_expr.arms {
            if arm.guard.is_some() {
                return None;
            }
            let syn::Pat::TupleStruct(ts) = &arm.pat else {
                return None;
            };
            if ts.elems.len() != 1 {
                return None;
            }
            let variant = ts.path.segments.last()?.ident.to_string();
            match variant.as_str() {
                "Left" => left_arm = Some(arm),
                "Right" => right_arm = Some(arm),
                _ => return None,
            }
        }

        let left_arm = left_arm?;
        let right_arm = right_arm?;
        let left_is_return = self
            .extract_value_expr(&left_arm.body)
            .is_some_and(|expr| matches!(expr, syn::Expr::Return(_)));
        let right_is_return = self
            .extract_value_expr(&right_arm.body)
            .is_some_and(|expr| matches!(expr, syn::Expr::Return(_)));
        if left_is_return == right_is_return {
            return None;
        }

        let variant_ctx = self.infer_variant_type_context_from_expr(&match_expr.expr)?;
        if variant_ctx.template_args.len() < 2 {
            return None;
        }

        let mut scrutinee =
            self.emit_expr_to_string_with_variant_ctx(&match_expr.expr, Some(&variant_ctx));
        if let syn::Expr::Call(call) = match_expr.expr.as_ref() {
            if let syn::Expr::Path(path) = call.func.as_ref() {
                if self.variant_ctor_name_from_path(&path.path).is_some() {
                    let enum_ty = format!(
                        "{}<{}>",
                        variant_ctx.enum_name,
                        variant_ctx.template_args.join(", ")
                    );
                    scrutinee = format!("{}({})", enum_ty, scrutinee);
                }
            }
        }

        let (success_arm, success_variant, return_arm, return_variant) = if left_is_return {
            (right_arm, "Right", left_arm, "Left")
        } else {
            (left_arm, "Left", right_arm, "Right")
        };
        let value_ty = expected_ty.map(|t| self.map_type(t)).unwrap_or_else(|| {
            if success_variant == "Left" {
                variant_ctx.template_args[0].clone()
            } else {
                variant_ctx.template_args[1].clone()
            }
        });
        let success_body = self.emit_expr_to_string_with_expected(&success_arm.body, expected_ty);
        let return_body = match self.extract_value_expr(&return_arm.body) {
            Some(syn::Expr::Return(ret)) => {
                self.emit_return_expr_with_variant_ctx(ret, &variant_ctx)
            }
            _ => return None,
        };

        let success_ts = match &success_arm.pat {
            syn::Pat::TupleStruct(ts) => ts,
            _ => return None,
        };
        let return_ts = match &return_arm.pat {
            syn::Pat::TupleStruct(ts) => ts,
            _ => return None,
        };
        let mut success_bindings = Vec::new();
        if !self.collect_pattern_binding_stmts(&success_ts.elems[0], "_mv", &mut success_bindings) {
            return None;
        }
        let mut return_bindings = Vec::new();
        if !self.collect_pattern_binding_stmts(&return_ts.elems[0], "_mv", &mut return_bindings) {
            return None;
        }
        let success_bindings_str = if success_bindings.is_empty() {
            String::new()
        } else {
            success_bindings.push(String::new());
            success_bindings.join(" ")
        };
        let return_bindings_str = if return_bindings.is_empty() {
            String::new()
        } else {
            return_bindings.push(String::new());
            return_bindings.join(" ")
        };

        let success_check = if success_variant == "Left" {
            "is_left"
        } else {
            "is_right"
        };
        let success_unwrap = if success_variant == "Left" {
            "unwrap_left"
        } else {
            "unwrap_right"
        };
        let return_unwrap = if return_variant == "Left" {
            "unwrap_left"
        } else {
            "unwrap_right"
        };

        Some(format!(
            "({{ auto&& _m = {}; {} _match_value; if (_m.{}()) {{ auto _mv = _m.{}(); {}_match_value = {}; }} else {{ auto _mv = _m.{}(); {}{}; }} _match_value; }})",
            scrutinee,
            value_ty,
            success_check,
            success_unwrap,
            success_bindings_str,
            success_body,
            return_unwrap,
            return_bindings_str,
            return_body
        ))
    }

    fn emit_return_expr_with_variant_ctx(
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
                        return format!(
                            "return {}<{}, {}>({})",
                            ctor_name, return_ctor_args[0], return_ctor_args[1], arg
                        );
                    }
                }
            }
        }
        format!("return {}", self.emit_expr_to_string(expr))
    }

    fn emit_lit(&self, lit: &syn::Lit) -> String {
        match lit {
            syn::Lit::Int(i) => {
                if i.suffix() == "u8" {
                    return format!("static_cast<uint8_t>({})", i.base10_digits());
                }
                i.base10_digits().to_string()
            }
            syn::Lit::Float(f) => f.base10_digits().to_string(),
            syn::Lit::Bool(b) => if b.value { "true" } else { "false" }.to_string(),
            syn::Lit::Str(s) => format!("\"{}\"", s.value()),
            syn::Lit::Char(c) => format!("U'{}'", c.value()),
            syn::Lit::Byte(b) => format!("static_cast<uint8_t>({})", b.value()),
            syn::Lit::ByteStr(bs) => {
                let bytes: Vec<String> =
                    bs.value().iter().map(|b| format!("0x{:02x}", b)).collect();
                format!(
                    "std::array<uint8_t, {}>{{{{ {} }}}}",
                    bytes.len(),
                    bytes.join(", ")
                )
            }
            _ => "/* TODO: literal */".to_string(),
        }
    }

    fn emit_binop(&self, op: &syn::BinOp) -> &'static str {
        match op {
            syn::BinOp::Add(_) => "+",
            syn::BinOp::Sub(_) => "-",
            syn::BinOp::Mul(_) => "*",
            syn::BinOp::Div(_) => "/",
            syn::BinOp::Rem(_) => "%",
            syn::BinOp::And(_) => "&&",
            syn::BinOp::Or(_) => "||",
            syn::BinOp::BitXor(_) => "^",
            syn::BinOp::BitAnd(_) => "&",
            syn::BinOp::BitOr(_) => "|",
            syn::BinOp::Shl(_) => "<<",
            syn::BinOp::Shr(_) => ">>",
            syn::BinOp::Eq(_) => "==",
            syn::BinOp::Lt(_) => "<",
            syn::BinOp::Le(_) => "<=",
            syn::BinOp::Ne(_) => "!=",
            syn::BinOp::Ge(_) => ">=",
            syn::BinOp::Gt(_) => ">",
            syn::BinOp::AddAssign(_) => "+=",
            syn::BinOp::SubAssign(_) => "-=",
            syn::BinOp::MulAssign(_) => "*=",
            syn::BinOp::DivAssign(_) => "/=",
            syn::BinOp::RemAssign(_) => "%=",
            syn::BinOp::BitXorAssign(_) => "^=",
            syn::BinOp::BitAndAssign(_) => "&=",
            syn::BinOp::BitOrAssign(_) => "|=",
            syn::BinOp::ShlAssign(_) => "<<=",
            syn::BinOp::ShrAssign(_) => ">>=",
            _ => "/* unknown op */",
        }
    }

    fn emit_path_to_string(&self, path: &syn::Path) -> String {
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        let joined = segments.join("::");

        // Resolve `Self::...` paths to the current struct name in impl scope.
        if segments.first().is_some_and(|s| s == "Self") && segments.len() > 1 {
            if let Some(struct_name) = &self.current_struct {
                let mut resolved = segments.clone();
                resolved[0] = struct_name.clone();
                if let Some(last) = resolved.last_mut() {
                    *last = escape_cpp_keyword(last);
                }
                return resolved.join("::");
            }
        }

        // Resolve `Self` to current struct name, or `auto` in trait context
        if segments.len() == 1 && segments[0] == "Self" {
            if let Some(ref struct_name) = self.current_struct {
                return struct_name.clone();
            } else {
                // In trait context, Self = the implementing type → use auto
                return "auto".to_string();
            }
        }

        // Resolve `self` to `(*this)` — for field access, `self.x` becomes `this->x`
        if segments.len() == 1 && segments[0] == "self" {
            return "(*this)".to_string();
        }

        // Resolve module-relative Rust path prefixes in expression/type paths.
        if let Some(first) = segments.first() {
            match first.as_str() {
                "crate" if segments.len() > 1 => {
                    let mut resolved = segments[1..].to_vec();
                    if let Some(last) = resolved.last_mut() {
                        *last = escape_cpp_keyword(last);
                    }
                    return resolved.join("::");
                }
                "self" if segments.len() > 1 => {
                    let mut resolved = if self.module_stack.is_empty() {
                        Vec::new()
                    } else {
                        self.module_stack.clone()
                    };
                    resolved.extend(segments[1..].iter().cloned());
                    if let Some(last) = resolved.last_mut() {
                        *last = escape_cpp_keyword(last);
                    }
                    return resolved.join("::");
                }
                "super" if segments.len() > 1 => {
                    let mut resolved = if self.module_stack.len() > 1 {
                        self.module_stack[..self.module_stack.len() - 1].to_vec()
                    } else {
                        Vec::new()
                    };
                    resolved.extend(segments[1..].iter().cloned());
                    if let Some(last) = resolved.last_mut() {
                        *last = escape_cpp_keyword(last);
                    }
                    return resolved.join("::");
                }
                _ => {}
            }
        }

        // Map Rust Option constructors
        if segments.len() == 1 && segments[0] == "None" {
            return "std::nullopt".to_string();
        }
        if joined == "core::option::Option::None" || joined == "std::option::Option::None" {
            return "std::nullopt".to_string();
        }
        if joined == "core::option::Option::Some" || joined == "std::option::Option::Some" {
            return "Some".to_string();
        }

        // Expanded either crate commonly references `IterEither` through imports.
        // Use a stable fully-qualified path so type/call sites resolve before re-exports.
        if !segments.is_empty() && segments[0] == "IterEither" {
            if segments.len() == 1 {
                return "iterator::IterEither".to_string();
            }
            let mut escaped = segments.clone();
            if let Some(last) = escaped.last_mut() {
                *last = escape_cpp_keyword(last);
            }
            return format!("iterator::{}", escaped.join("::"));
        }

        if let Some(kind) = joined.strip_prefix("core::panicking::AssertKind::") {
            return format!("rusty::panicking::AssertKind::{}", kind);
        }
        if joined == "core::panicking::panic" {
            return "rusty::panicking::panic".to_string();
        }

        // Map Rust Ordering enum variants to fallback ordering enum variants.
        match joined.as_str() {
            "core::cmp::Ordering::Less" => return "rusty::cmp::Ordering::Less".to_string(),
            "core::cmp::Ordering::Equal" => return "rusty::cmp::Ordering::Equal".to_string(),
            "core::cmp::Ordering::Greater" => return "rusty::cmp::Ordering::Greater".to_string(),
            _ => {}
        }

        // Try user-provided type mappings first (highest priority)
        if let Some(cpp_type) = self.user_type_map.lookup(&joined) {
            return cpp_type.to_string();
        }

        // Try mapping as a function/method path (e.g., Box::new → rusty::Box::new_)
        if let Some(cpp_fn) = types::map_function_path(&joined) {
            return cpp_fn.to_string();
        }

        // Try mapping as a standard type
        if let Some((cpp_type, _)) = types::map_std_type(&joined) {
            return cpp_type.to_string();
        }

        // Try as primitive
        if segments.len() == 1 {
            if let Some(cpp_prim) = types::map_primitive_type(&segments[0]) {
                return cpp_prim.to_string();
            }
        }

        // Escape C++ keywords in the last segment (e.g., Point::new → Point::new_)
        if segments.len() > 1 {
            let mut escaped = segments.clone();
            if let Some(last) = escaped.last_mut() {
                *last = escape_cpp_keyword(last);
            }
            return escaped.join("::");
        }

        // Single segment — escape if keyword
        escape_cpp_keyword(&joined)
    }

    fn emit_expr_path_to_string(&self, path: &syn::Path) -> String {
        if path.segments.len() == 1 {
            let name = path.segments[0].ident.to_string();
            if let Some(mapped) = self.lookup_local_binding_cpp_name(&name) {
                return mapped;
            }
        }
        self.emit_path_to_string(path)
    }

    fn map_type(&self, ty: &syn::Type) -> String {
        match ty {
            syn::Type::Path(tp) => {
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
                        return self.maybe_prefix_typename_for_dependent_path(format!(
                            "{}::{}",
                            self_type,
                            assoc_segments.join("::")
                        ));
                    }
                    return self_type;
                }

                let mut path_str = self.emit_path_to_string(&tp.path);
                if self.current_struct.is_some() && path_str.starts_with("Self::") {
                    path_str = path_str.trim_start_matches("Self::").to_string();
                }

                // Special case: Box<dyn Trait> → pro::proxy<TraitFacade> or std::move_only_function for Fn traits
                if let Some(last_seg) = tp.path.segments.last() {
                    let seg_name = last_seg.ident.to_string();
                    if seg_name == "Box" {
                        if let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments {
                            if let Some(syn::GenericArgument::Type(syn::Type::TraitObject(to))) =
                                args.args.first()
                            {
                                // Check for Fn → move_only_function
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
                                    let facade = if trait_names.len() == 1 {
                                        format!("{}Facade", trait_names[0])
                                    } else {
                                        format!("{}Facade", trait_names.join("And"))
                                    };
                                    return format!("pro::proxy<{}>", facade);
                                }
                            }
                        }
                    }
                }

                // Check if the last segment has generic arguments
                if let Some(last_seg) = tp.path.segments.last() {
                    if let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments {
                        let type_args: Vec<String> = args
                            .args
                            .iter()
                            .filter_map(|arg| {
                                if let syn::GenericArgument::Type(t) = arg {
                                    Some(self.map_type(t))
                                } else {
                                    None
                                }
                            })
                            .collect();

                        if !type_args.is_empty() {
                            // Reuse path_str so single-segment remaps (e.g. IterEither →
                            // iterator::IterEither) are preserved for generic type paths.
                            let mut base = path_str.clone();
                            if self.current_struct.is_some() && base.starts_with("Self::") {
                                base = base.trim_start_matches("Self::").to_string();
                            }
                            return self.maybe_prefix_typename_for_dependent_path(format!(
                                "{}<{}>",
                                base,
                                type_args.join(", ")
                            ));
                        }
                    }
                }

                if path_str.contains("::") {
                    return self.maybe_prefix_typename_for_dependent_path(path_str);
                }
                path_str
            }
            syn::Type::Reference(r) => {
                // Special case: &str → std::string_view (not const std::string_view&)
                if let syn::Type::Path(tp) = r.elem.as_ref() {
                    if tp.path.segments.len() == 1 && tp.path.segments[0].ident == "str" {
                        return "std::string_view".to_string();
                    }
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
                        let facade_name = if trait_names.len() == 1 {
                            format!("{}Facade", trait_names[0])
                        } else {
                            format!("{}Facade", trait_names.join("And"))
                        };
                        return format!("pro::proxy_view<{}>", facade_name);
                    }
                }
                let inner = self.map_type(&r.elem);
                if r.mutability.is_some() {
                    format!("{}&", inner)
                } else {
                    format!("const {}&", inner)
                }
            }
            syn::Type::Ptr(p) => {
                let inner = self.map_type(&p.elem);
                if p.mutability.is_some() {
                    format!("{}*", inner)
                } else {
                    format!("const {}*", inner)
                }
            }
            syn::Type::Tuple(t) => {
                if t.elems.is_empty() {
                    "void".to_string()
                } else {
                    let elems: Vec<String> = t.elems.iter().map(|e| self.map_type(e)).collect();
                    format!("std::tuple<{}>", elems.join(", "))
                }
            }
            syn::Type::Array(a) => {
                let elem = self.map_type(&a.elem);
                let len = self.emit_expr_to_string(&a.len);
                format!("std::array<{}, {}>", elem, len)
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
                    format!("pro::proxy_view<{}Facade>", trait_names[0])
                } else if trait_names.len() > 1 {
                    if trait_paths
                        .iter()
                        .any(|p| facade_name_for_trait_path(p).is_none())
                    {
                        return "void*".to_string();
                    }
                    // Multiple bounds: combine facade names
                    let combined = trait_names.join("And");
                    format!("pro::proxy_view<{}Facade>", combined)
                } else {
                    "/* TODO: complex trait object */".to_string()
                }
            }
            syn::Type::ImplTrait(it) => {
                // Check for Fn traits first
                if let Some(first) = it.bounds.first() {
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
                    format!("pro::proxy<{}Facade>", trait_names[0])
                } else if trait_names.len() > 1 {
                    if trait_paths
                        .iter()
                        .any(|p| facade_name_for_trait_path(p).is_none())
                    {
                        return "void*".to_string();
                    }
                    let combined = trait_names.join("And");
                    format!("pro::proxy<{}Facade>", combined)
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
    fn emit_closure_to_string(&self, closure: &syn::ExprClosure) -> String {
        // Determine capture mode
        let capture = if closure.capture.is_some() {
            // `move` closure → capture by move
            // We'd need to know which variables are captured to emit
            // [var1 = std::move(var1), var2 = std::move(var2)]
            // but without full analysis, we use a simpler approach:
            // emit [=] (copy capture) as a reasonable default for move closures
            // since the Rust compiler already verified the moves
            "="
        } else {
            // Default: borrow environment → capture by reference
            "&"
        };

        // Build parameter list
        let params: Vec<String> = closure
            .inputs
            .iter()
            .map(|p| self.emit_closure_param(p))
            .collect();

        let params_str = params.join(", ");

        // Determine if the body is a block or a single expression
        match closure.body.as_ref() {
            syn::Expr::Block(block) => {
                // Multi-statement body
                let mut inner = CodeGen::new_();
                inner.current_struct = self.current_struct.clone();
                inner.type_param_scopes = self.type_param_scopes.clone();
                inner.module_stack = self.module_stack.clone();
                inner.indent = 0;
                inner.emit_block(&block.block);
                let body_str = inner.into_output();
                format!("[{}]({}) {{\n{}}}", capture, params_str, body_str)
            }
            _ => {
                // Single expression body → return it
                let body = self.emit_expr_to_string(&closure.body);
                format!("[{}]({}) {{ return {}; }}", capture, params_str, body)
            }
        }
    }

    /// Emit a single closure parameter.
    fn emit_closure_param(&self, pat: &syn::Pat) -> String {
        match pat {
            syn::Pat::Ident(pi) => {
                // Untyped param: |x| → auto x
                format!("auto {}", pi.ident)
            }
            syn::Pat::Type(pt) => {
                // Typed param: |x: i32| → int32_t x
                let ty = self.map_type(&pt.ty);
                let name = match pt.pat.as_ref() {
                    syn::Pat::Ident(pi) => pi.ident.to_string(),
                    _ => "_".to_string(),
                };
                format!("{} {}", ty, name)
            }
            syn::Pat::Wild(_) => "auto _".to_string(),
            syn::Pat::Reference(pr) => {
                // |&x| → auto& x  or  |&mut x| → auto& x
                let inner = self.emit_closure_param(&pr.pat);
                format!("auto& {}", inner.trim_start_matches("auto "))
            }
            _ => format!("auto {}", self.emit_pat_to_string(pat)),
        }
    }

    /// Try to map a Fn/FnMut/FnOnce trait bound to a C++ function type.
    /// Returns Some(cpp_type) if it's a Fn trait, None otherwise.
    fn try_map_fn_trait(&self, tb: &syn::TraitBound) -> Option<String> {
        let last_seg = tb.path.segments.last()?;
        let trait_name = last_seg.ident.to_string();

        let wrapper = match trait_name.as_str() {
            "Fn" => "std::function",
            "FnMut" => "std::function",
            "FnOnce" => "std::move_only_function",
            _ => return None,
        };

        // Extract the parenthesized args: Fn(i32, i32) -> i32
        if let syn::PathArguments::Parenthesized(args) = &last_seg.arguments {
            let param_types: Vec<String> = args.inputs.iter().map(|t| self.map_type(t)).collect();

            let return_type = match &args.output {
                syn::ReturnType::Default => "void".to_string(),
                syn::ReturnType::Type(_, ty) => self.map_type(ty),
            };

            Some(format!(
                "{}<{}({})>",
                wrapper,
                return_type,
                param_types.join(", ")
            ))
        } else {
            // Fn without parens — treat as regular trait
            None
        }
    }

    /// Map Box<dyn Fn/FnMut/FnOnce> to the appropriate C++ function type.
    /// All Box<dyn Fn*> → std::move_only_function since Box implies ownership.
    fn try_map_fn_trait_boxed(&self, tb: &syn::TraitBound) -> Option<String> {
        let last_seg = tb.path.segments.last()?;
        let trait_name = last_seg.ident.to_string();

        if !matches!(trait_name.as_str(), "Fn" | "FnMut" | "FnOnce") {
            return None;
        }

        if let syn::PathArguments::Parenthesized(args) = &last_seg.arguments {
            let param_types: Vec<String> = args.inputs.iter().map(|t| self.map_type(t)).collect();
            let return_type = match &args.output {
                syn::ReturnType::Default => "void".to_string(),
                syn::ReturnType::Type(_, ty) => self.map_type(ty),
            };
            Some(format!(
                "std::move_only_function<{}({})>",
                return_type,
                param_types.join(", ")
            ))
        } else {
            None
        }
    }

    /// Create a new CodeGen for inner use (e.g., closure bodies).
    fn new_() -> Self {
        Self {
            output: String::new(),
            indent: 0,
            impl_blocks: HashMap::new(),
            impl_method_conflict_keys: HashMap::new(),
            operator_renames: HashMap::new(),
            current_struct: None,
            type_param_scopes: Vec::new(),
            enum_type_params: HashMap::new(),
            data_enum_types: HashSet::new(),
            struct_field_types: HashMap::new(),
            reassigned_vars: std::collections::HashSet::new(),
            consuming_method_receiver_vars: std::collections::HashSet::new(),
            repeat_elem_type_hints: HashMap::new(),
            local_bindings: Vec::new(),
            local_cpp_bindings: Vec::new(),
            param_bindings: Vec::new(),
            self_receiver_ref_scopes: Vec::new(),
            pattern_ref_bindings: Vec::new(),
            deref_method_scopes: Vec::new(),
            deref_mut_method_scopes: Vec::new(),
            module_name: None,
            module_stack: Vec::new(),
            skipped_module_traits: HashSet::new(),
            expanded_test_markers: Vec::new(),
            expanded_libtest_mode: false,
            emitted_top_level_functions: HashSet::new(),
            macro_rules_names: HashSet::new(),
            declared_item_names: HashSet::new(),
            emitted_method_conflict_keys: Vec::new(),
            return_value_scopes: Vec::new(),
            return_type_hints: Vec::new(),
            constructor_template_hints: Vec::new(),
            in_async: false,
            user_type_map: types::UserTypeMap::default(),
        }
    }

    fn emit_expr_maybe_move(&self, expr: &syn::Expr) -> String {
        if self.should_insert_move(expr) {
            let inner = self.emit_expr_to_string(expr);
            format!("std::move({})", inner)
        } else {
            self.emit_expr_to_string(expr)
        }
    }

    /// Determine whether an expression represents a local variable that should
    /// be wrapped in std::move() when used by value.
    fn should_insert_move(&self, expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Path(path) => {
                // Only single-segment paths (local variables)
                if path.path.segments.len() != 1 {
                    return false;
                }
                let name = path.path.segments[0].ident.to_string();

                // Skip keywords and special names
                if matches!(
                    name.as_str(),
                    "self" | "Self" | "true" | "false" | "None" | "Some" | "Ok" | "Err"
                ) {
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

                true
            }
            _ => false,
        }
    }

    /// Check if an initializer expression is a reference (`&expr` or `&mut expr`).
    fn is_ref_init(&self, expr: &syn::Expr) -> bool {
        matches!(expr, syn::Expr::Reference(_))
    }

    /// Extract the inner expression from a reference expression, as a string.
    fn extract_ref_inner(&self, expr: &syn::Expr) -> String {
        if let syn::Expr::Reference(r) = expr {
            self.emit_expr_to_string(&r.expr)
        } else {
            self.emit_expr_to_string(expr)
        }
    }

    /// For a reference binding that will become a pointer, determine the pointer type.
    /// `let mut r = &x` where x: T → `const T*`
    /// `let mut r = &mut x` where x: T → `T*`
    /// `let mut r: &T = &x` → `const T*`
    /// `let mut r: &mut T = &mut x` → `T*`
    fn map_ref_as_pointer_type(&self, local: &syn::Local, init_expr: &syn::Expr) -> String {
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
    fn map_ref_as_ref_type(&self, local: &syn::Local, init_expr: &syn::Expr) -> String {
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

    /// If a block contains exactly one expression (tail expr), return its string form.
    fn block_single_expr<'a>(&self, block: &'a syn::Block) -> Option<&'a syn::Expr> {
        if block.stmts.len() == 1 {
            if let syn::Stmt::Expr(expr, None) = &block.stmts[0] {
                return Some(expr);
            }
        }
        None
    }

    fn push_return_value_scope(&mut self, return_type: &str) {
        self.return_value_scopes.push(return_type != "void");
    }

    fn pop_return_value_scope(&mut self) {
        self.return_value_scopes.pop();
    }

    fn in_value_return_scope(&self) -> bool {
        self.return_value_scopes.last().copied().unwrap_or(false)
    }

    fn push_return_type_hint(&mut self, output: &syn::ReturnType) {
        match output {
            syn::ReturnType::Default => self.return_type_hints.push(None),
            syn::ReturnType::Type(_, ty) => self.return_type_hints.push(Some((**ty).clone())),
        }
    }

    fn pop_return_type_hint(&mut self) {
        self.return_type_hints.pop();
    }

    fn current_return_type_hint(&self) -> Option<&syn::Type> {
        self.return_type_hints.last().and_then(|hint| hint.as_ref())
    }

    fn current_try_macro(&self) -> &'static str {
        let returns_option = self
            .current_return_type_hint()
            .map(is_option_type_hint)
            .unwrap_or(false);
        match (self.in_async, returns_option) {
            (true, true) => "RUSTY_CO_TRY_OPT",
            (true, false) => "RUSTY_CO_TRY",
            (false, true) => "RUSTY_TRY_OPT",
            (false, false) => "RUSTY_TRY",
        }
    }

    /// Best-effort lowering of a Rust block used in expression position.
    /// Emits an IIFE that preserves simple local bindings and tail-expression return.
    fn block_expr_to_iife_string(
        &self,
        block: &syn::Block,
        expected_ty: Option<&syn::Type>,
    ) -> Option<String> {
        if let Some(single) = self.block_single_expr(block) {
            return Some(self.emit_expr_to_string_with_expected(single, expected_ty));
        }

        if block.stmts.is_empty() {
            return Some(self.match_expr_unreachable_fallback().to_string());
        }

        let mut stmts = Vec::new();
        let last_idx = block.stmts.len() - 1;

        for (idx, stmt) in block.stmts.iter().enumerate() {
            let is_last = idx == last_idx;
            match stmt {
                syn::Stmt::Local(local) => match &local.pat {
                    syn::Pat::Ident(pi) => {
                        let qualifier = if pi.mutability.is_some() {
                            ""
                        } else {
                            "const "
                        };
                        let ty = if let Some(ty) = get_local_type(local) {
                            self.map_type(ty)
                        } else {
                            "auto".to_string()
                        };
                        if let Some(init) = &local.init {
                            let init_str = self.emit_expr_to_string(&init.expr);
                            stmts.push(format!("{}{} {} = {};", qualifier, ty, pi.ident, init_str));
                        } else {
                            stmts.push(format!("{}{} {};", qualifier, ty, pi.ident));
                        }
                    }
                    syn::Pat::Type(pt) => {
                        if let syn::Pat::Ident(pi) = pt.pat.as_ref() {
                            let qualifier = if pi.mutability.is_some() {
                                ""
                            } else {
                                "const "
                            };
                            let ty = self.map_type(&pt.ty);
                            if let Some(init) = &local.init {
                                let init_str = self.emit_expr_to_string(&init.expr);
                                stmts.push(format!(
                                    "{}{} {} = {};",
                                    qualifier, ty, pi.ident, init_str
                                ));
                            } else {
                                stmts.push(format!("{}{} {};", qualifier, ty, pi.ident));
                            }
                        } else {
                            return None;
                        }
                    }
                    _ => return None,
                },
                syn::Stmt::Expr(expr, semi) => {
                    let force_diverging_tail_return = is_last
                        && semi.is_some()
                        && expected_ty.is_some()
                        && self.expr_is_noreturn_panic_like(expr);
                    if (is_last && semi.is_none()) || force_diverging_tail_return {
                        stmts.push(format!(
                            "return {};",
                            self.emit_expr_to_string_with_expected(expr, expected_ty)
                        ));
                    } else {
                        stmts.push(format!("{};", self.emit_expr_to_string(expr)));
                    }
                }
                syn::Stmt::Macro(stmt_macro) => {
                    if is_last {
                        return None;
                    }
                    stmts.push(format!("{};", self.emit_macro_expr(&stmt_macro.mac)));
                }
                syn::Stmt::Item(_) => return None,
            }
        }

        Some(format!("[&]() {{ {} }}()", stmts.join(" ")))
    }

    fn push_type_param_scope(&mut self, generics: &syn::Generics) {
        let mut scope = HashSet::new();
        for param in &generics.params {
            if let syn::GenericParam::Type(tp) = param {
                scope.insert(tp.ident.to_string());
            }
        }
        self.type_param_scopes.push(scope);
    }

    fn pop_type_param_scope(&mut self) {
        self.type_param_scopes.pop();
    }

    fn is_type_param_in_scope(&self, name: &str) -> bool {
        self.type_param_scopes
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
    }

    fn normalize_qself_base_for_assoc(&self, self_type: &str) -> String {
        let mut base = self_type.trim().to_string();
        while let Some(stripped) = base.strip_prefix("const ") {
            base = stripped.trim().to_string();
        }
        while base.ends_with('&') || base.ends_with('*') {
            base.pop();
            base = base.trim_end().to_string();
        }
        base
    }

    fn maybe_prefix_typename_for_dependent_path(&self, path: String) -> String {
        if path.starts_with("typename ") {
            return path;
        }
        let first = path.split("::").next().unwrap_or_default().trim();
        if first == "Self" || self.is_type_param_in_scope(first) {
            return format!("typename {}", path);
        }
        path
    }

    /// Emit a `template<typename T, typename U, ...>` prefix if the generics
    /// have type parameters. Lifetime parameters are erased (skipped).
    /// Trait bounds are emitted as `requires` clauses.
    fn emit_template_prefix(&mut self, generics: &syn::Generics) {
        let (params, constraints) = self.collect_emitted_template_parts(generics);
        if params.is_empty() {
            return;
        }

        let params_str: Vec<String> = params.iter().map(|p| format!("typename {}", p)).collect();

        self.writeln(&format!("template<{}>", params_str.join(", ")));

        if !constraints.is_empty() {
            self.writeln(&format!("    requires ({})", constraints.join(" && ")));
        }
    }

    fn should_soften_dependent_assoc_mode(&self) -> bool {
        self.module_name.is_some() || self.expanded_libtest_mode
    }

    fn return_type_contains_dependent_assoc(&self, output: &syn::ReturnType) -> bool {
        let syn::ReturnType::Type(_, ty) = output else {
            return false;
        };
        self.type_contains_dependent_assoc(ty)
    }

    fn return_type_references_current_struct_assoc(&self, output: &syn::ReturnType) -> bool {
        let syn::ReturnType::Type(_, ty) = output else {
            return false;
        };
        self.type_references_current_struct_assoc(ty)
    }

    fn type_contains_dependent_assoc(&self, ty: &syn::Type) -> bool {
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

    fn type_references_current_struct_assoc(&self, ty: &syn::Type) -> bool {
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

    fn type_mentions_in_scope_type_param(&self, ty: &syn::Type) -> bool {
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

    fn map_return_type(&self, output: &syn::ReturnType) -> String {
        match output {
            syn::ReturnType::Default => "void".to_string(),
            syn::ReturnType::Type(_, ty) => self.map_type(ty),
        }
    }

    fn map_fn_params(
        &self,
        inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
    ) -> String {
        let params: Vec<String> = inputs
            .iter()
            .map(|arg| match arg {
                syn::FnArg::Typed(pat_type) => {
                    let ty = self.map_type(&pat_type.ty);
                    let name = match pat_type.pat.as_ref() {
                        syn::Pat::Ident(pi) => pi.ident.to_string(),
                        _ => "_".to_string(),
                    };
                    format!("{} {}", ty, name)
                }
                syn::FnArg::Receiver(_) => "/* self */".to_string(),
            })
            .collect();
        params.join(", ")
    }
}

/// Escape C++ reserved keywords by appending an underscore.
/// Map Rust operator trait names to C++ operator function names.
fn map_operator_trait(trait_name: &str) -> Option<&'static str> {
    match trait_name {
        "Add" => Some("operator+"),
        "Sub" => Some("operator-"),
        "Mul" => Some("operator*"),
        "Div" => Some("operator/"),
        "Rem" => Some("operator%"),
        "Neg" => Some("operator-"),
        "Not" => Some("operator!"),
        "BitAnd" => Some("operator&"),
        "BitOr" => Some("operator|"),
        "BitXor" => Some("operator^"),
        "Shl" => Some("operator<<"),
        "Shr" => Some("operator>>"),
        "AddAssign" => Some("operator+="),
        "SubAssign" => Some("operator-="),
        "MulAssign" => Some("operator*="),
        "DivAssign" => Some("operator/="),
        "RemAssign" => Some("operator%="),
        "Index" => Some("operator[]"),
        "Deref" => Some("operator*"),
        "PartialEq" => Some("operator=="),
        "PartialOrd" => Some("operator<=>"),
        _ => None,
    }
}

/// Build a deterministic conflict key for merged impl methods.
/// Return type is intentionally excluded because C++ cannot overload by return type.
fn impl_method_conflict_key(method: &syn::ImplItemFn) -> String {
    let receiver_key = match method.sig.inputs.first() {
        Some(syn::FnArg::Receiver(recv)) => {
            if recv.reference.is_some() {
                if recv.mutability.is_some() {
                    "recv:&mut"
                } else {
                    "recv:&"
                }
            } else {
                "recv:self"
            }
        }
        _ => "recv:static",
    };

    let mut params = Vec::new();
    for arg in &method.sig.inputs {
        if let syn::FnArg::Typed(pt) = arg {
            params.push(normalize_token_text(pt.ty.to_token_stream().to_string()));
        }
    }
    let params_key = params.join(",");
    let generics_key = normalize_token_text(method.sig.generics.to_token_stream().to_string());
    format!(
        "{}|{}|{}|{}",
        method.sig.ident, receiver_key, generics_key, params_key
    )
}

/// Merge impl-level type params into a method's generic parameter list.
/// This preserves placeholders introduced by specialized impl blocks
/// (e.g., impl<L, R, E> Either<Result<L,E>, Result<R,E>>).
fn merge_impl_type_generics_into_method(
    method: &mut syn::ImplItemFn,
    impl_generics: &syn::Generics,
) {
    let mut existing: HashSet<String> = method
        .sig
        .generics
        .params
        .iter()
        .filter_map(|p| match p {
            syn::GenericParam::Type(tp) => Some(tp.ident.to_string()),
            _ => None,
        })
        .collect();

    for param in &impl_generics.params {
        if let syn::GenericParam::Type(tp) = param {
            if existing.insert(tp.ident.to_string()) {
                method
                    .sig
                    .generics
                    .params
                    .push(syn::GenericParam::Type(tp.clone()));
            }
        }
    }

    if let Some(impl_where) = &impl_generics.where_clause {
        let where_clause = method.sig.generics.make_where_clause();
        let mut seen_preds: HashSet<String> = where_clause
            .predicates
            .iter()
            .map(|p| normalize_token_text(p.to_token_stream().to_string()))
            .collect();

        for pred in &impl_where.predicates {
            let key = normalize_token_text(pred.to_token_stream().to_string());
            if seen_preds.insert(key) {
                where_clause.predicates.push(pred.clone());
            }
        }
    }
}

fn normalize_token_text(tokens: String) -> String {
    tokens.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn facade_name_for_trait_path(path: &syn::Path) -> Option<String> {
    let first = path.segments.first()?.ident.to_string();
    let last = path.segments.last()?.ident.to_string();

    // Expanded crates frequently reference external traits that do not have generated
    // Proxy facade types in the transpiled module. Skip those facade references.
    let skip = matches!(first.as_str(), "std" | "core" | "alloc")
        || matches!(
            last.as_str(),
            "Fn" | "FnMut"
                | "FnOnce"
                | "Into"
                | "Error"
                | "Hasher"
                | "IntoIterator"
                | "Iterator"
                | "DoubleEndedIterator"
                | "Extend"
                | "FromIterator"
                | "Default"
                | "AsRef"
                | "AsMut"
        );
    if skip {
        return None;
    }

    Some(format!("{}Facade", last))
}

fn visit_overloaded_helper_text() -> &'static str {
    "template<class... Ts>\n\
struct overloaded : Ts... { using Ts::operator()...; };\n\
template<class... Ts>\n\
overloaded(Ts...) -> overloaded<Ts...>;\n\
\n"
}

fn iterator_iter_either_forward_decl_text() -> &'static str {
    "namespace iterator {\n\
template<typename L, typename R>\n\
struct IterEither;\n\
}\n\
\n"
}

fn needs_runtime_path_fallback_helpers(output: &str) -> bool {
    let markers = [
        "rusty::intrinsics::",
        "rusty::panicking::",
        "rusty::hash::hash",
        "rusty::fmt::",
        "rusty::pin::",
        "rusty::path::Path",
        "rusty::ffi::",
        "rusty::cmp::Ordering",
        "rusty::str_runtime::from_utf8",
        "rusty::str_runtime::parse<",
        "rusty::deref_ref(",
        "rusty::deref_mut(",
        "core::cmp::",
    ];
    markers.iter().any(|m| output.contains(m))
}

fn runtime_path_fallback_helpers_text() -> &'static str {
    "namespace rusty {\n\
namespace cmp {\n\
enum class Ordering { Less, Equal, Greater };\n\
}\n\
namespace fmt {\n\
using Result = bool;\n\
struct Arguments {};\n\
struct Formatter {\n\
    template<typename... Args>\n\
    static Result debug_tuple_field1_finish(Args&&...) { return true; }\n\
    template<typename... Args>\n\
    static Result debug_struct_field1_finish(Args&&...) { return true; }\n\
};\n\
}\n\
namespace path {\n\
using Path = std::string;\n\
}\n\
namespace ffi {\n\
using OsStr = std::string;\n\
using CStr = std::string;\n\
}\n\
namespace pin {\n\
template<typename T>\n\
using Pin = T;\n\
template<typename T>\n\
constexpr decltype(auto) new_unchecked(T&& value) {\n\
    return std::forward<T>(value);\n\
}\n\
template<typename T>\n\
constexpr const T* get_ref(const T& value) {\n\
    return &value;\n\
}\n\
template<typename T>\n\
constexpr T* get_unchecked_mut(T& value) {\n\
    return &value;\n\
}\n\
}\n\
namespace hash {\n\
template<typename T, typename State>\n\
void hash(const T&, State&) {}\n\
}\n\
namespace str_runtime {\n\
inline bool is_valid_utf8(const unsigned char* data, std::size_t len) {\n\
    std::size_t i = 0;\n\
    while (i < len) {\n\
        const auto byte = data[i];\n\
        if (byte <= 0x7F) {\n\
            ++i;\n\
            continue;\n\
        }\n\
        if ((byte >> 5) == 0x6) {\n\
            if (i + 1 >= len) return false;\n\
            const auto b1 = data[i + 1];\n\
            if ((b1 & 0xC0) != 0x80 || byte < 0xC2) return false;\n\
            i += 2;\n\
            continue;\n\
        }\n\
        if ((byte >> 4) == 0xE) {\n\
            if (i + 2 >= len) return false;\n\
            const auto b1 = data[i + 1];\n\
            const auto b2 = data[i + 2];\n\
            if ((b1 & 0xC0) != 0x80 || (b2 & 0xC0) != 0x80) return false;\n\
            if (byte == 0xE0 && b1 < 0xA0) return false;\n\
            if (byte == 0xED && b1 >= 0xA0) return false;\n\
            i += 3;\n\
            continue;\n\
        }\n\
        if ((byte >> 3) == 0x1E) {\n\
            if (i + 3 >= len) return false;\n\
            const auto b1 = data[i + 1];\n\
            const auto b2 = data[i + 2];\n\
            const auto b3 = data[i + 3];\n\
            if ((b1 & 0xC0) != 0x80 || (b2 & 0xC0) != 0x80 || (b3 & 0xC0) != 0x80) return false;\n\
            if (byte == 0xF0 && b1 < 0x90) return false;\n\
            if (byte == 0xF4 && b1 >= 0x90) return false;\n\
            if (byte > 0xF4) return false;\n\
            i += 4;\n\
            continue;\n\
        }\n\
        return false;\n\
    }\n\
    return true;\n\
}\n\
template<typename Bytes>\n\
rusty::Result<std::string_view, rusty::String> from_utf8(const Bytes& bytes) {\n\
    if constexpr (requires { bytes.data(); bytes.size(); }) {\n\
        const auto* raw = bytes.data();\n\
        const std::size_t len = static_cast<std::size_t>(bytes.size());\n\
        const auto* data = reinterpret_cast<const unsigned char*>(raw);\n\
        if (!is_valid_utf8(data, len)) {\n\
            return rusty::Result<std::string_view, rusty::String>::Err(rusty::String::from(\"invalid utf-8\"));\n\
        }\n\
        return rusty::Result<std::string_view, rusty::String>::Ok(\n\
            std::string_view(reinterpret_cast<const char*>(raw), len)\n\
        );\n\
    }\n\
    return rusty::Result<std::string_view, rusty::String>::Err(rusty::String::from(\"unsupported from_utf8 input\"));\n\
}\n\
template<typename T, typename Input>\n\
rusty::Result<T, rusty::String> parse(const Input& input) {\n\
    std::string_view text;\n\
    if constexpr (std::is_convertible_v<Input, std::string_view>) {\n\
        text = std::string_view(input);\n\
    } else if constexpr (requires { input.as_str(); }) {\n\
        text = std::string_view(input.as_str());\n\
    } else {\n\
        return rusty::Result<T, rusty::String>::Err(rusty::String::from(\"unsupported parse input\"));\n\
    }\n\
    if constexpr (std::is_integral_v<T> && !std::is_same_v<T, bool>) {\n\
        T value{};\n\
        const auto* begin = text.data();\n\
        const auto* end = begin + text.size();\n\
        const auto [ptr, ec] = std::from_chars(begin, end, value);\n\
        if (ec == std::errc() && ptr == end) {\n\
            return rusty::Result<T, rusty::String>::Ok(value);\n\
        }\n\
        return rusty::Result<T, rusty::String>::Err(rusty::String::from(\"invalid digit found in string\"));\n\
    }\n\
    return rusty::Result<T, rusty::String>::Err(rusty::String::from(\"unsupported parse target\"));\n\
}\n\
}\n\
template<typename T>\n\
auto deref_ref(const T& value) {\n\
    if constexpr (requires { value.as_str(); }) {\n\
        return value.as_str();\n\
    } else if constexpr (requires { *value; }) {\n\
        return *value;\n\
    } else {\n\
        return value;\n\
    }\n\
}\n\
template<typename T>\n\
decltype(auto) deref_mut(T& value) {\n\
    if constexpr (requires { *value; }) {\n\
        return *value;\n\
    } else {\n\
        return (value);\n\
    }\n\
}\n\
namespace panicking {\n\
enum class AssertKind { Eq, Ne };\n\
template<typename... Args>\n\
[[noreturn]] inline void assert_failed(Args&&...) { std::abort(); }\n\
template<typename... Args>\n\
[[noreturn]] inline void panic(Args&&...) { std::abort(); }\n\
template<typename... Args>\n\
[[noreturn]] inline void panic_fmt(Args&&...) { std::abort(); }\n\
}\n\
namespace intrinsics {\n\
struct Discriminant {\n\
    std::size_t value;\n\
    bool operator==(const Discriminant&) const = default;\n\
    rusty::cmp::Ordering cmp(const Discriminant& other) const {\n\
        if (value < other.value) return rusty::cmp::Ordering::Less;\n\
        if (value > other.value) return rusty::cmp::Ordering::Greater;\n\
        return rusty::cmp::Ordering::Equal;\n\
    }\n\
    Option<rusty::cmp::Ordering> partial_cmp(const Discriminant& other) const {\n\
        return Option<rusty::cmp::Ordering>(cmp(other));\n\
    }\n\
    template<typename State>\n\
    void hash(State& state) const {\n\
        rusty::hash::hash(value, state);\n\
    }\n\
};\n\
template<typename V>\n\
Discriminant discriminant_value(const V& value) {\n\
    return Discriminant{static_cast<std::size_t>(value.index())};\n\
}\n\
[[noreturn]] inline void unreachable() { std::abort(); }\n\
}\n\
}\n\
\n\
namespace core {\n\
namespace cmp {\n\
using Ordering = ::rusty::cmp::Ordering;\n\
struct PartialOrd {\n\
    template<typename A, typename B>\n\
    static auto partial_cmp(A&& a, B&& b) {\n\
        return std::forward<A>(a).partial_cmp(std::forward<B>(b));\n\
    }\n\
};\n\
struct Ord {\n\
    template<typename A, typename B>\n\
    static auto cmp(A&& a, B&& b) {\n\
        return std::forward<A>(a).cmp(std::forward<B>(b));\n\
    }\n\
};\n\
}\n\
}\n\
\n"
}

/// Build a scoped impl key from a self-type path and current inline-module path.
/// For local impls like `impl Foo` inside `mod a`, this returns `a::Foo`.
/// For explicit paths (`foo::Bar`), keep the path as-is.
/// For `self::` / `super::` / `crate::`, resolve to the current module stack.
fn qualify_impl_type_name(
    raw: &str,
    module_path: &[String],
    top_level_declared_item_names: &HashSet<String>,
) -> String {
    let parts: Vec<&str> = raw.split("::").collect();
    if parts.is_empty() {
        return raw.to_string();
    }

    if parts.len() == 1 {
        if module_path.is_empty() {
            return raw.to_string();
        }
        // If a nested module `impl` targets a top-level type imported from
        // `super`/`crate` (for example `use super::Either; impl Iterator for Either`),
        // keep the top-level name so methods merge into the real type definition.
        if top_level_declared_item_names.contains(raw) {
            return raw.to_string();
        }
        return format!("{}::{}", module_path.join("::"), raw);
    }

    let mut resolved_prefix = module_path.to_vec();
    let mut idx = 0usize;
    let mut had_relative_prefix = false;
    while idx < parts.len() {
        match parts[idx] {
            "self" => {
                had_relative_prefix = true;
                idx += 1;
            }
            "super" => {
                had_relative_prefix = true;
                if !resolved_prefix.is_empty() {
                    resolved_prefix.pop();
                }
                idx += 1;
            }
            "crate" => {
                had_relative_prefix = true;
                resolved_prefix.clear();
                idx += 1;
            }
            _ => break,
        }
    }

    if !had_relative_prefix {
        return raw.to_string();
    }

    let mut out = resolved_prefix;
    out.extend(parts[idx..].iter().map(|s| s.to_string()));
    if out.is_empty() {
        raw.to_string()
    } else {
        out.join("::")
    }
}

enum UseImportAction {
    RustOnly,
    Using(String),
    Raw(String),
}

fn classify_use_import(path: &str) -> UseImportAction {
    let normalized = normalize_use_import_path(path);

    if is_either_variant_reexport(normalized) {
        return UseImportAction::RustOnly;
    }
    if let Some(action) = rewrite_std_io_import(normalized) {
        return action;
    }
    if let Some(action) = rewrite_std_string_import(normalized) {
        return action;
    }
    if is_rust_only_import(normalized) {
        return UseImportAction::RustOnly;
    }
    UseImportAction::Using(path.to_string())
}

fn normalize_use_import_path(path: &str) -> &str {
    path.strip_prefix("namespace ").unwrap_or(path)
}

/// C++ `using` declarations require either a nested-name-specifier (`a::b`) or alias
/// form (`X = Y`). Rust `use super::{Foo}` can flatten to bare names, so normalize
/// those to global-qualified form.
fn make_using_path_cpp_legal(path: &str) -> String {
    if path.is_empty()
        || path.starts_with("namespace ")
        || path.starts_with("::")
        || path.contains("::")
        || path.contains(" = ")
    {
        path.to_string()
    } else {
        format!("::{}", path)
    }
}

/// `pub use ...::Either::{Left, Right};` often appears before the enum declaration in the
/// source order. Emitting a C++ `using` for these names at that point is invalid because
/// `Either` is not yet declared, so treat these as Rust-only imports for now.
fn is_either_variant_reexport(path: &str) -> bool {
    let parts: Vec<&str> = path.split("::").collect();
    if parts.len() < 2 {
        return false;
    }

    let enum_name = parts[parts.len() - 2];
    let variant_name = parts[parts.len() - 1];
    enum_name == "Either" && matches!(variant_name, "Left" | "Right")
}

/// Some pub re-exports in module mode refer to declarations with module linkage
/// (for example `pub use iterator::IterEither;` in expanded either output).
/// Emitting `export using` for these names is rejected by compilers, so keep
/// them as Rust-only until the originating declaration is explicitly exported.
fn is_module_linkage_sensitive_reexport(path: &str) -> bool {
    let parts: Vec<&str> = path.split("::").collect();
    if parts.len() < 2 {
        return false;
    }
    parts[parts.len() - 2] == "iterator" && parts[parts.len() - 1] == "IterEither"
}

fn rewrite_std_io_import(path: &str) -> Option<UseImportAction> {
    // `use std::io;` imports the module name itself; alias it to rusty::io.
    if path == "std::io" {
        return Some(UseImportAction::Raw(
            "namespace io = rusty::io;".to_string(),
        ));
    }

    let io_item = path.strip_prefix("std::io::")?;
    let action = match io_item {
        // Runtime I/O types have direct rusty::io equivalents.
        "Cursor" | "Error" | "Result" | "SeekFrom" | "Stdin" | "Stdout" | "Stderr" => {
            UseImportAction::Using(format!("rusty::io::{}", io_item))
        }
        // Function renames match existing path mapping (`stdin` → `stdin_`, etc.).
        "stdin" => UseImportAction::Using("rusty::io::stdin_".to_string()),
        "stdout" => UseImportAction::Using("rusty::io::stdout_".to_string()),
        "stderr" => UseImportAction::Using("rusty::io::stderr_".to_string()),
        // Read/Write/Seek/BufRead and related trait imports are Rust-only.
        _ => UseImportAction::RustOnly,
    };
    Some(action)
}

fn rewrite_std_string_import(path: &str) -> Option<UseImportAction> {
    if path == "std::string::String" {
        return Some(UseImportAction::Using("rusty::String".to_string()));
    }
    None
}

/// Check if a using path refers to a Rust-only trait/module with no C++ equivalent.
/// These are silently skipped (commented out) since they have no meaning in C++.
fn is_rust_only_import(path: &str) -> bool {
    // Rust std trait modules that don't exist in C++
    let rust_only_prefixes = [
        "std::convert::", // AsRef, AsMut, From, Into
        "std::ops::",     // Deref, DerefMut, Add, Sub, Index, etc.
        "std::fmt",       // Display, Debug, Formatter, etc.
        "std::iter",      // Iterator, IntoIterator, etc.
        "std::error",     // Error trait
        "std::future",    // Future trait
        "std::pin",       // Pin type (no C++ equivalent)
        "std::marker",    // Send, Sync, Copy, Sized, etc.
        "std::clone",     // Clone trait
        "std::cmp",       // PartialEq, Ord, etc.
        "std::hash",      // Hash trait
        "std::default",   // Default trait
        "std::borrow",    // Borrow, BorrowMut
        "std::prelude::", // Rust prelude modules (e.g., rust_2018)
    ];

    for prefix in &rust_only_prefixes {
        if path.starts_with(prefix) || path == *prefix {
            return true;
        }
    }
    false
}

/// Find the matching closing paren for an opening paren at `open_pos`.
fn find_matching_paren(s: &str, open_pos: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.get(open_pos) != Some(&b'(') {
        return std::option::Option::None;
    }
    let mut depth = 1i32;
    for i in (open_pos + 1)..bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return std::option::Option::Some(i);
                }
            }
            _ => {}
        }
    }
    std::option::Option::None
}

fn escape_cpp_keyword(name: &str) -> String {
    match name {
        "new" | "delete" | "class" | "template" | "virtual" | "override" | "final" | "operator"
        | "friend" | "namespace" | "using" | "throw" | "catch" | "try" | "public" | "private"
        | "protected" | "mutable" | "volatile" | "register" | "inline" | "explicit" | "export"
        | "typedef" | "typename" | "decltype" | "constexpr" | "nullptr" | "alignof" | "alignas"
        | "thread_local" | "static_assert" | "noexcept" | "consteval" | "constinit"
        | "co_await" | "co_return" | "co_yield" | "requires" | "concept" | "import" | "module"
        | "default" => {
            format!("{}_", name)
        }
        _ => name.to_string(),
    }
}

/// Scan a list of statements and collect the names of variables that are reassigned.
/// This is used for reference rebinding detection: `let mut r = &x; r = &y;`
/// means `r` should be emitted as a pointer, not a reference.
fn collect_reassigned_vars(stmts: &[syn::Stmt]) -> std::collections::HashSet<String> {
    let mut result = std::collections::HashSet::new();
    for stmt in stmts {
        collect_assignments_in_stmt(stmt, &mut result);
    }
    result
}

/// Scan statements and collect immutable locals that are consumed through
/// by-value method calls (for example `res.unwrap_err()`).
fn collect_consuming_method_receiver_vars(
    stmts: &[syn::Stmt],
) -> std::collections::HashSet<String> {
    let mut result = std::collections::HashSet::new();
    for stmt in stmts {
        collect_consuming_method_receivers_in_stmt(stmt, &mut result);
    }
    result
}

/// Infer element types for untyped repeat-array locals (`let x = [0; N];`) by
/// scanning indexed assignments in the same block (`x[i] = ... as u8`).
fn collect_repeat_element_type_hints(stmts: &[syn::Stmt]) -> HashMap<String, syn::Type> {
    let mut candidates = std::collections::HashSet::new();
    for stmt in stmts {
        let syn::Stmt::Local(local) = stmt else {
            continue;
        };
        if get_local_type(local).is_some() {
            continue;
        }
        let syn::Pat::Ident(pat_ident) = &local.pat else {
            continue;
        };
        let Some(init) = &local.init else {
            continue;
        };
        if !is_untyped_repeat_with_integer_seed(&init.expr) {
            continue;
        }
        candidates.insert(pat_ident.ident.to_string());
    }

    if candidates.is_empty() {
        return HashMap::new();
    }

    let mut hints = HashMap::new();
    for stmt in stmts {
        collect_repeat_assignment_hints_in_stmt(stmt, &candidates, &mut hints);
    }
    hints
}

fn is_untyped_repeat_with_integer_seed(expr: &syn::Expr) -> bool {
    let syn::Expr::Repeat(repeat) = peel_paren_group_expr(expr) else {
        return false;
    };
    matches!(
        peel_paren_group_expr(&repeat.expr),
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(_),
            ..
        })
    )
}

fn collect_repeat_assignment_hints_in_stmt(
    stmt: &syn::Stmt,
    candidates: &std::collections::HashSet<String>,
    hints: &mut HashMap<String, syn::Type>,
) {
    match stmt {
        syn::Stmt::Local(local) => {
            if let Some(init) = &local.init {
                collect_repeat_assignment_hints_in_expr(&init.expr, candidates, hints);
            }
        }
        syn::Stmt::Expr(expr, _) => {
            collect_repeat_assignment_hints_in_expr(expr, candidates, hints)
        }
        syn::Stmt::Item(_) | syn::Stmt::Macro(_) => {}
    }
}

fn collect_repeat_assignment_hints_in_expr(
    expr: &syn::Expr,
    candidates: &std::collections::HashSet<String>,
    hints: &mut HashMap<String, syn::Type>,
) {
    match expr {
        syn::Expr::Assign(assign) => {
            if let Some(base_name) = extract_index_base_ident(&assign.left) {
                if candidates.contains(&base_name) && !hints.contains_key(&base_name) {
                    if let Some(rhs_ty) = infer_repeat_assignment_rhs_type(&assign.right) {
                        hints.insert(base_name, rhs_ty);
                    }
                }
            }
            collect_repeat_assignment_hints_in_expr(&assign.left, candidates, hints);
            collect_repeat_assignment_hints_in_expr(&assign.right, candidates, hints);
        }
        syn::Expr::Block(block) => {
            for stmt in &block.block.stmts {
                collect_repeat_assignment_hints_in_stmt(stmt, candidates, hints);
            }
        }
        syn::Expr::If(if_expr) => {
            collect_repeat_assignment_hints_in_expr(&if_expr.cond, candidates, hints);
            for stmt in &if_expr.then_branch.stmts {
                collect_repeat_assignment_hints_in_stmt(stmt, candidates, hints);
            }
            if let Some((_, else_expr)) = &if_expr.else_branch {
                collect_repeat_assignment_hints_in_expr(else_expr, candidates, hints);
            }
        }
        syn::Expr::While(while_expr) => {
            collect_repeat_assignment_hints_in_expr(&while_expr.cond, candidates, hints);
            for stmt in &while_expr.body.stmts {
                collect_repeat_assignment_hints_in_stmt(stmt, candidates, hints);
            }
        }
        syn::Expr::Loop(loop_expr) => {
            for stmt in &loop_expr.body.stmts {
                collect_repeat_assignment_hints_in_stmt(stmt, candidates, hints);
            }
        }
        syn::Expr::ForLoop(for_expr) => {
            collect_repeat_assignment_hints_in_expr(&for_expr.expr, candidates, hints);
            for stmt in &for_expr.body.stmts {
                collect_repeat_assignment_hints_in_stmt(stmt, candidates, hints);
            }
        }
        syn::Expr::Unsafe(unsafe_expr) => {
            for stmt in &unsafe_expr.block.stmts {
                collect_repeat_assignment_hints_in_stmt(stmt, candidates, hints);
            }
        }
        syn::Expr::Call(call) => {
            collect_repeat_assignment_hints_in_expr(&call.func, candidates, hints);
            for arg in &call.args {
                collect_repeat_assignment_hints_in_expr(arg, candidates, hints);
            }
        }
        syn::Expr::MethodCall(method_call) => {
            collect_repeat_assignment_hints_in_expr(&method_call.receiver, candidates, hints);
            for arg in &method_call.args {
                collect_repeat_assignment_hints_in_expr(arg, candidates, hints);
            }
        }
        syn::Expr::Binary(binary) => {
            collect_repeat_assignment_hints_in_expr(&binary.left, candidates, hints);
            collect_repeat_assignment_hints_in_expr(&binary.right, candidates, hints);
        }
        syn::Expr::Unary(unary) => {
            collect_repeat_assignment_hints_in_expr(&unary.expr, candidates, hints);
        }
        syn::Expr::Index(index) => {
            collect_repeat_assignment_hints_in_expr(&index.expr, candidates, hints);
            collect_repeat_assignment_hints_in_expr(&index.index, candidates, hints);
        }
        syn::Expr::Reference(reference) => {
            collect_repeat_assignment_hints_in_expr(&reference.expr, candidates, hints);
        }
        syn::Expr::Paren(paren) => {
            collect_repeat_assignment_hints_in_expr(&paren.expr, candidates, hints);
        }
        syn::Expr::Group(group) => {
            collect_repeat_assignment_hints_in_expr(&group.expr, candidates, hints);
        }
        syn::Expr::Match(match_expr) => {
            collect_repeat_assignment_hints_in_expr(&match_expr.expr, candidates, hints);
            for arm in &match_expr.arms {
                if let Some((_, guard)) = &arm.guard {
                    collect_repeat_assignment_hints_in_expr(guard, candidates, hints);
                }
                collect_repeat_assignment_hints_in_expr(&arm.body, candidates, hints);
            }
        }
        syn::Expr::Array(array) => {
            for elem in &array.elems {
                collect_repeat_assignment_hints_in_expr(elem, candidates, hints);
            }
        }
        syn::Expr::Tuple(tuple) => {
            for elem in &tuple.elems {
                collect_repeat_assignment_hints_in_expr(elem, candidates, hints);
            }
        }
        syn::Expr::Struct(struct_expr) => {
            for field in &struct_expr.fields {
                collect_repeat_assignment_hints_in_expr(&field.expr, candidates, hints);
            }
            if let Some(rest) = &struct_expr.rest {
                collect_repeat_assignment_hints_in_expr(rest, candidates, hints);
            }
        }
        syn::Expr::Await(await_expr) => {
            collect_repeat_assignment_hints_in_expr(&await_expr.base, candidates, hints);
        }
        syn::Expr::Try(try_expr) => {
            collect_repeat_assignment_hints_in_expr(&try_expr.expr, candidates, hints);
        }
        syn::Expr::Break(brk) => {
            if let Some(value) = &brk.expr {
                collect_repeat_assignment_hints_in_expr(value, candidates, hints);
            }
        }
        syn::Expr::Return(ret) => {
            if let Some(value) = &ret.expr {
                collect_repeat_assignment_hints_in_expr(value, candidates, hints);
            }
        }
        syn::Expr::Closure(closure) => {
            collect_repeat_assignment_hints_in_expr(&closure.body, candidates, hints);
        }
        syn::Expr::Let(let_expr) => {
            collect_repeat_assignment_hints_in_expr(&let_expr.expr, candidates, hints);
        }
        _ => {}
    }
}

fn extract_index_base_ident(expr: &syn::Expr) -> Option<String> {
    let mut current = peel_paren_group_expr(expr);
    while let syn::Expr::Index(index) = current {
        current = peel_paren_group_expr(&index.expr);
    }
    if let syn::Expr::Path(path) = current {
        if path.path.segments.len() == 1 {
            return Some(path.path.segments[0].ident.to_string());
        }
    }
    None
}

fn infer_repeat_assignment_rhs_type(expr: &syn::Expr) -> Option<syn::Type> {
    let current = peel_paren_group_expr(expr);
    if let syn::Expr::Cast(cast_expr) = current {
        return Some((*cast_expr.ty).clone());
    }
    None
}

fn peel_paren_group_expr<'a>(expr: &'a syn::Expr) -> &'a syn::Expr {
    let mut current = expr;
    loop {
        match current {
            syn::Expr::Paren(paren) => current = &paren.expr,
            syn::Expr::Group(group) => current = &group.expr,
            _ => return current,
        }
    }
}

fn collect_consuming_method_receivers_in_stmt(
    stmt: &syn::Stmt,
    result: &mut std::collections::HashSet<String>,
) {
    match stmt {
        syn::Stmt::Local(local) => {
            if let Some(init) = &local.init {
                collect_consuming_method_receivers_in_expr(&init.expr, result);
            }
        }
        syn::Stmt::Expr(expr, _) => collect_consuming_method_receivers_in_expr(expr, result),
        _ => {}
    }
}

fn collect_consuming_method_receivers_in_expr(
    expr: &syn::Expr,
    result: &mut std::collections::HashSet<String>,
) {
    match expr {
        syn::Expr::MethodCall(mc) => {
            if is_consuming_method_name(&mc.method.to_string()) {
                if let Some(name) = extract_simple_local_ident(&mc.receiver) {
                    result.insert(name);
                }
            }
            collect_consuming_method_receivers_in_expr(&mc.receiver, result);
            for arg in &mc.args {
                collect_consuming_method_receivers_in_expr(arg, result);
            }
        }
        syn::Expr::Call(call) => {
            collect_consuming_method_receivers_in_expr(&call.func, result);
            for arg in &call.args {
                collect_consuming_method_receivers_in_expr(arg, result);
            }
        }
        syn::Expr::Binary(bin) => {
            collect_consuming_method_receivers_in_expr(&bin.left, result);
            collect_consuming_method_receivers_in_expr(&bin.right, result);
        }
        syn::Expr::Unary(un) => collect_consuming_method_receivers_in_expr(&un.expr, result),
        syn::Expr::Reference(r) => collect_consuming_method_receivers_in_expr(&r.expr, result),
        syn::Expr::Assign(assign) => {
            collect_consuming_method_receivers_in_expr(&assign.left, result);
            collect_consuming_method_receivers_in_expr(&assign.right, result);
        }
        syn::Expr::Block(block) => {
            for stmt in &block.block.stmts {
                collect_consuming_method_receivers_in_stmt(stmt, result);
            }
        }
        syn::Expr::If(if_expr) => {
            collect_consuming_method_receivers_in_expr(&if_expr.cond, result);
            for stmt in &if_expr.then_branch.stmts {
                collect_consuming_method_receivers_in_stmt(stmt, result);
            }
            if let Some((_, else_expr)) = &if_expr.else_branch {
                collect_consuming_method_receivers_in_expr(else_expr, result);
            }
        }
        syn::Expr::Match(match_expr) => {
            collect_consuming_method_receivers_in_expr(&match_expr.expr, result);
            for arm in &match_expr.arms {
                if let Some((_, guard)) = &arm.guard {
                    collect_consuming_method_receivers_in_expr(guard, result);
                }
                collect_consuming_method_receivers_in_expr(&arm.body, result);
            }
        }
        syn::Expr::While(w) => {
            collect_consuming_method_receivers_in_expr(&w.cond, result);
            for stmt in &w.body.stmts {
                collect_consuming_method_receivers_in_stmt(stmt, result);
            }
        }
        syn::Expr::Loop(l) => {
            for stmt in &l.body.stmts {
                collect_consuming_method_receivers_in_stmt(stmt, result);
            }
        }
        syn::Expr::ForLoop(f) => {
            collect_consuming_method_receivers_in_expr(&f.expr, result);
            for stmt in &f.body.stmts {
                collect_consuming_method_receivers_in_stmt(stmt, result);
            }
        }
        syn::Expr::Paren(p) => collect_consuming_method_receivers_in_expr(&p.expr, result),
        syn::Expr::Group(g) => collect_consuming_method_receivers_in_expr(&g.expr, result),
        syn::Expr::Array(arr) => {
            for elem in &arr.elems {
                collect_consuming_method_receivers_in_expr(elem, result);
            }
        }
        syn::Expr::Tuple(tuple) => {
            for elem in &tuple.elems {
                collect_consuming_method_receivers_in_expr(elem, result);
            }
        }
        syn::Expr::Struct(struct_expr) => {
            for field in &struct_expr.fields {
                collect_consuming_method_receivers_in_expr(&field.expr, result);
            }
            if let Some(rest) = &struct_expr.rest {
                collect_consuming_method_receivers_in_expr(rest, result);
            }
        }
        syn::Expr::Field(field) => collect_consuming_method_receivers_in_expr(&field.base, result),
        syn::Expr::Index(index) => {
            collect_consuming_method_receivers_in_expr(&index.expr, result);
            collect_consuming_method_receivers_in_expr(&index.index, result);
        }
        syn::Expr::Cast(cast_expr) => {
            collect_consuming_method_receivers_in_expr(&cast_expr.expr, result)
        }
        syn::Expr::Await(await_expr) => {
            collect_consuming_method_receivers_in_expr(&await_expr.base, result)
        }
        syn::Expr::Try(try_expr) => {
            collect_consuming_method_receivers_in_expr(&try_expr.expr, result)
        }
        syn::Expr::Break(brk) => {
            if let Some(value) = &brk.expr {
                collect_consuming_method_receivers_in_expr(value, result);
            }
        }
        syn::Expr::Return(ret) => {
            if let Some(value) = &ret.expr {
                collect_consuming_method_receivers_in_expr(value, result);
            }
        }
        syn::Expr::Closure(closure) => {
            collect_consuming_method_receivers_in_expr(&closure.body, result)
        }
        syn::Expr::Let(let_expr) => {
            collect_consuming_method_receivers_in_expr(&let_expr.expr, result)
        }
        syn::Expr::Unsafe(unsafe_expr) => {
            for stmt in &unsafe_expr.block.stmts {
                collect_consuming_method_receivers_in_stmt(stmt, result);
            }
        }
        _ => {}
    }
}

fn extract_simple_local_ident(expr: &syn::Expr) -> Option<String> {
    let mut current = expr;
    loop {
        match current {
            syn::Expr::Paren(p) => current = &p.expr,
            syn::Expr::Group(g) => current = &g.expr,
            syn::Expr::Path(path) if path.path.segments.len() == 1 => {
                return Some(path.path.segments[0].ident.to_string());
            }
            _ => return None,
        }
    }
}

fn is_consuming_method_name(method: &str) -> bool {
    matches!(
        method,
        "unwrap"
            | "unwrap_err"
            | "expect"
            | "expect_err"
            | "unwrap_left"
            | "unwrap_right"
            | "expect_left"
            | "expect_right"
    ) || method.starts_with("into_")
}

/// Recursively collect assignment targets from a statement.
fn collect_assignments_in_stmt(stmt: &syn::Stmt, result: &mut std::collections::HashSet<String>) {
    match stmt {
        syn::Stmt::Expr(expr, _) => collect_assignments_in_expr(expr, result),
        _ => {}
    }
}

/// Recursively collect assignment targets from an expression.
fn collect_assignments_in_expr(expr: &syn::Expr, result: &mut std::collections::HashSet<String>) {
    match expr {
        syn::Expr::Assign(assign) => {
            // `r = ...` → r is reassigned
            if let syn::Expr::Path(path) = assign.left.as_ref() {
                if path.path.segments.len() == 1 {
                    result.insert(path.path.segments[0].ident.to_string());
                }
            }
        }
        syn::Expr::Block(block) => {
            for s in &block.block.stmts {
                collect_assignments_in_stmt(s, result);
            }
        }
        syn::Expr::If(if_expr) => {
            for s in &if_expr.then_branch.stmts {
                collect_assignments_in_stmt(s, result);
            }
            if let Some((_, else_branch)) = &if_expr.else_branch {
                collect_assignments_in_expr(else_branch, result);
            }
        }
        syn::Expr::While(w) => {
            for s in &w.body.stmts {
                collect_assignments_in_stmt(s, result);
            }
        }
        syn::Expr::Loop(l) => {
            for s in &l.body.stmts {
                collect_assignments_in_stmt(s, result);
            }
        }
        syn::Expr::ForLoop(f) => {
            for s in &f.body.stmts {
                collect_assignments_in_stmt(s, result);
            }
        }
        syn::Expr::Unsafe(u) => {
            for s in &u.block.stmts {
                collect_assignments_in_stmt(s, result);
            }
        }
        _ => {}
    }
}

fn is_option_type_hint(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(tp) => tp
            .path
            .segments
            .last()
            .map(|seg| seg.ident == "Option")
            .unwrap_or(false),
        syn::Type::Group(group) => is_option_type_hint(&group.elem),
        syn::Type::Paren(paren) => is_option_type_hint(&paren.elem),
        syn::Type::Reference(reference) => is_option_type_hint(&reference.elem),
        _ => false,
    }
}

/// Extract the type annotation from a local variable binding, if present.
fn get_local_type(local: &syn::Local) -> Option<&syn::Type> {
    match &local.pat {
        syn::Pat::Type(pt) => Some(&pt.ty),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transpile_str(rust_code: &str) -> String {
        let file: syn::File = syn::parse_str(rust_code).unwrap();
        let mut cg = CodeGen::new();
        cg.emit_file(&file, None);
        cg.into_output()
    }

    fn transpile_str_module(rust_code: &str, module_name: &str) -> String {
        let file: syn::File = syn::parse_str(rust_code).unwrap();
        let mut cg = CodeGen::new();
        cg.emit_file(&file, Some(module_name));
        cg.into_output()
    }

    #[test]
    fn test_simple_function() {
        let out = transpile_str("fn add(a: i32, b: i32) -> i32 { a + b }");
        assert!(out.contains("int32_t add(int32_t a, int32_t b)"));
        assert!(out.contains("return a + b;"));
    }

    #[test]
    fn test_void_function() {
        let out = transpile_str("fn noop() {}");
        assert!(out.contains("void noop()"));
    }

    #[test]
    fn test_struct() {
        let out = transpile_str("struct Point { x: f64, y: f64 }");
        assert!(out.contains("struct Point {"));
        assert!(out.contains("double x;"));
        assert!(out.contains("double y;"));
    }

    #[test]
    fn test_const() {
        let out = transpile_str("const MAX: i32 = 100;");
        assert!(out.contains("constexpr int32_t MAX = 100;"));
    }

    #[test]
    fn test_c_like_enum() {
        let out = transpile_str("enum Color { Red, Green, Blue }");
        assert!(out.contains("enum class Color {"));
        assert!(out.contains("Red"));
        assert!(out.contains("Green"));
        assert!(out.contains("Blue"));
    }

    #[test]
    fn test_enum_with_data() {
        let out = transpile_str("enum Shape { Circle(f64), Rect { w: f64, h: f64 }, None }");
        assert!(out.contains("struct Shape_Circle {"));
        assert!(out.contains("double _0;"));
        assert!(out.contains("struct Shape_Rect {"));
        assert!(out.contains("double w;"));
        assert!(out.contains("struct Shape_None {}"));
        assert!(out.contains("using Shape = std::variant<"));
    }

    #[test]
    fn test_type_alias() {
        let out = transpile_str("type Num = i32;");
        assert!(out.contains("using Num = int32_t;"));
    }

    #[test]
    fn test_let_binding_immutable() {
        let out = transpile_str("fn f() { let x = 5; }");
        assert!(out.contains("const auto x = 5;"));
    }

    #[test]
    fn test_let_binding_mutable() {
        let out = transpile_str("fn f() { let mut x = 5; }");
        assert!(out.contains("auto x = 5;"));
    }

    #[test]
    fn test_std_type_mapping() {
        let out = transpile_str("fn f(v: Vec<i32>) {}");
        assert!(out.contains("rusty::Vec<int32_t>"));
    }

    #[test]
    fn test_reference_types() {
        let out = transpile_str("fn f(a: &i32, b: &mut i32) {}");
        assert!(out.contains("const int32_t&"));
        assert!(out.contains("int32_t&"));
    }

    #[test]
    fn test_option_type() {
        let out = transpile_str("fn f() -> Option<i32> { 42 }");
        assert!(out.contains("rusty::Option<int32_t>"));
    }

    #[test]
    fn test_tuple_return() {
        let out = transpile_str("fn f() -> (i32, f64) { (1, 2.0) }");
        assert!(out.contains("std::tuple<int32_t, double>"));
    }

    #[test]
    fn test_array_type() {
        let out = transpile_str("fn f(a: [i32; 5]) {}");
        assert!(out.contains("std::array<int32_t, 5>"));
    }

    #[test]
    fn test_unit_return_is_void() {
        let out = transpile_str("fn f() -> () {}");
        assert!(out.contains("void f()"));
    }

    #[test]
    fn test_binary_operators() {
        let out = transpile_str("fn f() -> i32 { 1 + 2 * 3 }");
        assert!(out.contains("return 1 + 2 * 3;"));
    }

    #[test]
    fn test_static_var() {
        let out = transpile_str("static COUNT: i32 = 0;");
        assert!(out.contains("static int32_t COUNT = 0;"));
    }

    #[test]
    fn test_cast() {
        let out = transpile_str("fn f() { let x = 5 as f64; }");
        assert!(out.contains("static_cast<double>(5)"));
    }

    #[test]
    fn test_string_literal() {
        let out = transpile_str(r#"fn f() { let s = "hello"; }"#);
        assert!(out.contains("\"hello\""));
    }

    #[test]
    fn test_bool_literal() {
        let out = transpile_str("fn f() -> bool { true }");
        assert!(out.contains("return true;"));
    }

    #[test]
    fn test_method_call() {
        let out = transpile_str("fn f() { v.push(1); }");
        assert!(out.contains("v.push(1);"));
    }

    #[test]
    fn test_field_access() {
        let out = transpile_str("fn f() -> f64 { p.x }");
        assert!(out.contains("return p.x;"));
    }

    #[test]
    fn test_index_expr() {
        let out = transpile_str("fn f() { let x = arr[0]; }");
        assert!(out.contains("arr[0]"));
    }

    #[test]
    fn test_struct_literal() {
        let out = transpile_str("fn f() -> Point { Point { x: 1.0, y: 2.0 } }");
        assert!(out.contains("Point{"));
        assert!(out.contains(".x = 1.0"));
        assert!(out.contains(".y = 2.0"));
    }

    #[test]
    fn test_result_type() {
        let out = transpile_str("fn f() -> Result<i32, String> { Ok(1) }");
        assert!(out.contains("rusty::Result<int32_t, rusty::String>"));
    }

    #[test]
    fn test_box_type() {
        let out = transpile_str("fn f(b: Box<i32>) {}");
        assert!(out.contains("rusty::Box<int32_t>"));
    }

    #[test]
    fn test_hashmap_type() {
        let out = transpile_str("fn f(m: HashMap<String, i32>) {}");
        assert!(out.contains("rusty::HashMap<rusty::String, int32_t>"));
    }

    // ── Control flow tests ──────────────────────────────────────

    #[test]
    fn test_if_simple() {
        let out = transpile_str("fn f(x: i32) { if x > 0 { x; } }");
        assert!(out.contains("if (x > 0) {"));
        assert!(out.contains("x;"));
        assert!(out.contains("}"));
    }

    #[test]
    fn test_if_else() {
        let out = transpile_str("fn f(x: i32) { if x > 0 { x; } else { 0; } }");
        assert!(out.contains("if (x > 0) {"));
        assert!(out.contains("} else {"));
    }

    #[test]
    fn test_if_else_if() {
        let out =
            transpile_str("fn f(x: i32) { if x > 0 { 1; } else if x < 0 { 2; } else { 0; } }");
        assert!(out.contains("if (x > 0) {"));
        assert!(out.contains("} else if (x < 0) {"));
        assert!(out.contains("} else {"));
    }

    #[test]
    fn test_if_tail_expr_returns() {
        // When if/else is the tail expression of a function, each branch gets `return`
        let out = transpile_str("fn f(c: bool) -> i32 { if c { 1 } else { 0 } }");
        assert!(out.contains("if (c) {"));
        assert!(out.contains("return 1;"));
        assert!(out.contains("return 0;"));
    }

    #[test]
    fn test_if_expr_as_ternary() {
        // When if/else is used in a let binding with simple branches → ternary
        let out = transpile_str("fn f(c: bool) { let x = if c { 1 } else { 0 }; }");
        assert!(out.contains("(c ? 1 : 0)"));
    }

    #[test]
    fn test_leaf4296_if_expr_constructor_pair_wraps_arms_to_common_either_type() {
        let out = transpile_str(
            r#"
            fn f(use_empty: bool, mockdata: [u8; 8]) {
                let reader = if use_empty {
                    Left(io::Cursor::new([]))
                } else {
                    Right(io::Cursor::new(&mockdata[..]))
                };
            }
        "#,
        );
        assert!(out.contains("auto reader = (use_empty ? Either<"));
        assert!(out.contains(">(Left<"));
        assert!(out.contains(") : Either<"));
        assert!(!out.contains("auto reader = (use_empty ? Left<"));
    }

    #[test]
    fn test_leaf4296_typed_if_expr_constructor_pair_uses_expected_either_wrapper() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn f(c: bool) {
                let e: Either<i32, i32> = if c { Left(1) } else { Right(2) };
            }
        "#,
        );
        assert!(out.contains(
            "const Either<int32_t, int32_t> e = (c ? Either<int32_t, int32_t>(Left<int32_t, int32_t>(1)) : Either<int32_t, int32_t>(Right<int32_t, int32_t>(2)));"
        ));
    }

    #[test]
    fn test_leaf4297_read_write_ref_buffer_args_lower_to_slice_view() {
        let out = transpile_str(
            r#"
            struct IO;
            impl IO {
                fn read(&mut self, _buf: &mut [u8]) -> usize { 0 }
                fn write(&mut self, _buf: &[u8]) -> usize { 0 }
            }
            fn f(io: &mut IO) {
                let mut read_buf = [0u8; 16];
                let write_buf = [1u8; 16];
                io.read(&mut read_buf);
                io.write(&write_buf);
            }
        "#,
        );
        assert!(out.contains("io.read(rusty::slice_full(read_buf));"));
        assert!(out.contains("io.write(rusty::slice_full(write_buf));"));
        assert!(!out.contains("io.read(&read_buf);"));
        assert!(!out.contains("io.write(&write_buf);"));
    }

    #[test]
    fn test_leaf4297_u8_repeat_preserves_byte_literal_type() {
        let out = transpile_str(
            r#"
            fn f() {
                let a = [0u8; 4];
                let b = [1u8; 8];
            }
        "#,
        );
        assert!(out.contains("const auto a = rusty::array_repeat(static_cast<uint8_t>(0), 4);"));
        assert!(out.contains("const auto b = rusty::array_repeat(static_cast<uint8_t>(1), 8);"));
    }

    #[test]
    fn test_while_loop() {
        let out = transpile_str("fn f() { let mut x = 10; while x > 0 { x = x - 1; } }");
        assert!(out.contains("while (x > 0) {"));
    }

    #[test]
    fn test_infinite_loop() {
        let out = transpile_str("fn f() { loop { break; } }");
        assert!(out.contains("while (true) {"));
        assert!(out.contains("break;"));
    }

    #[test]
    fn test_for_in_range() {
        let out = transpile_str("fn f() { for i in 0..10 { i; } }");
        assert!(out.contains("for (auto&& i : rusty::range(0, 10)) {"));
    }

    #[test]
    fn test_for_in_variable() {
        let out = transpile_str("fn f() { for x in items { x; } }");
        assert!(out.contains("for (auto&& x : items) {"));
    }

    #[test]
    fn test_break_continue() {
        let out = transpile_str("fn f() { loop { if true { break; } continue; } }");
        assert!(out.contains("break;"));
        assert!(out.contains("continue;"));
    }

    #[test]
    fn test_loop_break_with_value() {
        let out = transpile_str("fn f() { let result = loop { if true { break 42; } }; }");
        assert!(out.contains("[&]()"));
        assert!(out.contains("while (true)"));
        assert!(out.contains("return 42;"));
        assert!(out.contains("}();"));
    }

    #[test]
    fn test_nested_if_in_loop() {
        let out = transpile_str("fn f() { while true { if true { break; } else { continue; } } }");
        assert!(out.contains("while (true) {"));
        assert!(out.contains("if (true) {"));
        assert!(out.contains("break;"));
        assert!(out.contains("} else {"));
        assert!(out.contains("continue;"));
    }

    #[test]
    fn test_nested_loops() {
        let out = transpile_str("fn f() { for i in 0..5 { for j in 0..5 { i; } } }");
        assert!(out.contains("for (auto&& i : rusty::range(0, 5)) {"));
        assert!(out.contains("for (auto&& j : rusty::range(0, 5)) {"));
    }

    #[test]
    fn test_if_with_let_binding() {
        let out = transpile_str("fn f() { if true { let x = 1; x; } }");
        assert!(out.contains("if (true) {"));
        assert!(out.contains("const auto x = 1;"));
    }

    #[test]
    fn test_closure_basic() {
        let out = transpile_str("fn f() { let add = |a, b| a + b; }");
        assert!(out.contains("[&]"));
        assert!(out.contains("return a + b;"));
    }

    // ── Impl block and method tests ─────────────────────────────

    #[test]
    fn test_impl_block_merged_into_struct() {
        let out = transpile_str(
            r#"
            struct Foo { x: i32 }
            impl Foo {
                fn get(&self) -> i32 { self.x }
            }
        "#,
        );
        assert!(out.contains("struct Foo {"));
        assert!(out.contains("int32_t x;"));
        assert!(out.contains("int32_t get() const {"));
        assert!(out.contains("return x;"));
        // Impl block should NOT appear separately
        assert!(!out.contains("// TODO: impl block"));
    }

    #[test]
    fn test_method_receivers() {
        let out = transpile_str(
            r#"
            struct S { v: i32 }
            impl S {
                fn by_ref(&self) {}
                fn by_mut_ref(&mut self) {}
                fn by_value(self) {}
                fn associated() {}
            }
        "#,
        );
        assert!(out.contains("void by_ref() const {"));
        assert!(out.contains("void by_mut_ref() {"));
        assert!(out.contains("void by_value() {"));
        assert!(out.contains("static void associated() {"));
    }

    #[test]
    fn test_multiple_impl_blocks_merged() {
        let out = transpile_str(
            r#"
            struct P { x: f64 }
            impl P { fn a(&self) -> f64 { self.x } }
            impl P { fn b(&self) -> f64 { self.x } }
        "#,
        );
        assert!(out.contains("double a() const {"));
        assert!(out.contains("double b() const {"));
        // Both should be inside the struct
        let struct_pos = out.find("struct P {").unwrap();
        let close_pos = out[struct_pos..].find("};").unwrap() + struct_pos;
        let a_pos = out.find("double a()").unwrap();
        let b_pos = out.find("double b()").unwrap();
        assert!(a_pos > struct_pos && a_pos < close_pos);
        assert!(b_pos > struct_pos && b_pos < close_pos);
    }

    #[test]
    fn test_leaf45_duplicate_method_signature_keeps_first() {
        let out = transpile_str(
            r#"
            struct Foo {}
            impl Foo { fn cloned(&self) -> i32 { 1 } }
            impl Foo { fn cloned(&self) -> i32 { 2 } }
        "#,
        );

        assert_eq!(out.matches("int32_t cloned() const {").count(), 1);
        assert!(out.contains("return 1;"));
        assert!(!out.contains("return 2;"));
    }

    #[test]
    fn test_leaf45_methods_with_different_params_not_deduped() {
        let out = transpile_str(
            r#"
            struct Foo {}
            impl Foo { fn as_ref(&self) -> i32 { 1 } }
            impl Foo { fn as_ref(&self, x: i32) -> i32 { x } }
        "#,
        );

        assert_eq!(out.matches("as_ref(").count(), 2);
        assert!(out.contains("int32_t as_ref() const {"));
        assert!(out.contains("int32_t as_ref(int32_t x) const {"));
    }

    #[test]
    fn test_leaf45_same_name_different_return_type_is_deduped() {
        let out = transpile_str(
            r#"
            struct Foo {}
            impl Foo { fn as_mut(&self) -> i32 { 1 } }
            impl Foo { fn as_mut(&self) -> bool { true } }
        "#,
        );

        assert_eq!(out.matches(" as_mut() const {").count(), 1);
        assert!(out.contains("int32_t as_mut() const {"));
        assert!(!out.contains("bool as_mut() const {"));
    }

    #[test]
    fn test_leaf45_mapped_param_type_collision_is_deduped() {
        let out = transpile_str(
            r#"
            struct Foo {}
            impl Foo { fn fmt(&self, f: core::fmt::Formatter) -> core::fmt::Result { true } }
            impl Foo { fn fmt(&self, f: fmt::Formatter) -> fmt::Result { true } }
        "#,
        );

        assert_eq!(
            out.matches("rusty::fmt::Result fmt(rusty::fmt::Formatter f) const {")
                .count(),
            1
        );
    }

    #[test]
    fn test_leaf414_impl_bounds_not_emitted_do_not_bypass_dedup() {
        let out = transpile_str_module(
            r#"
            struct Foo<T> { inner: T }
            impl<T: core::fmt::Debug> Foo<T> {
                fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result { true }
            }
            impl<T: core::fmt::Display> Foo<T> {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { true }
            }
        "#,
            "either",
        );

        assert_eq!(
            out.matches("rusty::fmt::Result fmt(rusty::fmt::Formatter& f) const {")
                .count(),
            1
        );
    }

    #[test]
    fn test_self_field_access() {
        let out = transpile_str(
            r#"
            struct S { val: i32 }
            impl S {
                fn get(&self) -> i32 { self.val }
                fn set(&mut self, v: i32) { self.val = v; }
            }
        "#,
        );
        // self.val → val (direct field access in C++ methods)
        assert!(out.contains("return val;"));
        assert!(out.contains("val = v;"));
    }

    #[test]
    fn test_self_type_resolved() {
        let out = transpile_str(
            r#"
            struct Foo { x: i32 }
            impl Foo {
                fn make() -> Self { Foo { x: 0 } }
            }
        "#,
        );
        // Self → Foo
        assert!(out.contains("static Foo make()"));
    }

    #[test]
    fn test_cpp_keyword_escaping() {
        let out = transpile_str(
            r#"
            struct Widget {}
            impl Widget {
                fn new() -> Self { Widget {} }
                fn delete(&mut self) {}
            }
        "#,
        );
        // new → new_, delete → delete_
        assert!(out.contains("static Widget new_()"));
        assert!(out.contains("void delete_()"));
    }

    #[test]
    fn test_keyword_in_call() {
        let out = transpile_str("fn f() { let w = Widget::new(); }");
        assert!(out.contains("Widget::new_()"));
    }

    #[test]
    fn test_default_keyword_escaped_in_impl_and_call() {
        let out = transpile_str(
            r#"
            struct D {}
            impl D {
                fn default() -> Self { D {} }
            }
            fn f() { let _d = D::default(); }
        "#,
        );
        assert!(out.contains("static D default_()"));
        assert!(out.contains("D::default_()"));
        assert!(!out.contains("D::default()"));
    }

    #[test]
    fn test_default_keyword_escaped_in_generic_path_call() {
        let out = transpile_str("fn f<T>() -> T { T::default() }");
        assert!(out.contains("T::default_()"));
    }

    #[test]
    fn test_impl_const() {
        let out = transpile_str(
            r#"
            struct C {}
            impl C {
                const MAX: i32 = 100;
            }
        "#,
        );
        assert!(out.contains("static constexpr int32_t MAX = 100;"));
    }

    // ── extern "C" and unsafe tests ─────────────────────────────

    #[test]
    fn test_extern_c_function() {
        let out = transpile_str(r#"extern "C" fn add(a: i32, b: i32) -> i32 { a + b }"#);
        assert!(out.contains("extern \"C\" int32_t add("));
    }

    #[test]
    fn test_extern_c_block() {
        let out = transpile_str(
            r#"
            extern "C" {
                fn c_malloc(size: usize) -> *mut u8;
                fn c_free(ptr: *mut u8);
            }
        "#,
        );
        assert!(out.contains("extern \"C\" {"));
        assert!(out.contains("uint8_t* c_malloc(size_t size);"));
        assert!(out.contains("void c_free(uint8_t* ptr);"));
    }

    #[test]
    fn test_unsafe_block() {
        let out = transpile_str("fn f() { unsafe { let x = 1; } }");
        assert!(out.contains("// @unsafe"));
        assert!(out.contains("const auto x = 1;"));
    }

    // ── Tuple and destructuring tests ───────────────────────────

    #[test]
    fn test_tuple_destructuring() {
        let out = transpile_str("fn f() { let (a, b) = pair; }");
        assert!(out.contains("auto [a, b] = pair;"));
    }

    #[test]
    fn test_tuple_expression() {
        let out = transpile_str("fn f() { let t = (1, 2, 3); }");
        assert!(out.contains("std::make_tuple(1, 2, 3)"));
    }

    #[test]
    fn test_typed_let_binding() {
        let out = transpile_str("fn f() { let x: i32 = 5; }");
        assert!(out.contains("const int32_t x = 5;"));
    }

    #[test]
    fn test_typed_let_mut_binding() {
        let out = transpile_str("fn f() { let mut x: i32 = 5; }");
        assert!(out.contains("int32_t x = 5;"));
        assert!(!out.contains("const"));
    }

    // ── Reference rebinding tests ───────────────────────────────

    #[test]
    fn test_ref_no_rebind_emits_reference() {
        let out = transpile_str("fn f() { let x = 5; let r = &x; }");
        assert!(out.contains("const auto& r = x;"));
    }

    #[test]
    fn test_ref_rebind_emits_pointer() {
        let out = transpile_str("fn f() { let x = 5; let y = 10; let mut r = &x; r = &y; }");
        assert!(out.contains("const auto* r = &x;"));
    }

    #[test]
    fn test_mut_ref_no_rebind_emits_reference() {
        let out = transpile_str("fn f() { let mut x = 5; let mr = &mut x; }");
        assert!(out.contains("auto& mr = x;"));
    }

    #[test]
    fn test_mut_ref_rebind_emits_pointer() {
        let out = transpile_str(
            "fn f() { let mut x = 5; let mut y = 10; let mut mr = &mut x; mr = &mut y; }",
        );
        assert!(out.contains("auto* mr = &x;"));
    }

    #[test]
    fn test_ref_rebind_in_if_branch() {
        // Rebinding inside if branch should still be detected
        let out =
            transpile_str("fn f() { let x = 5; let y = 10; let mut r = &x; if true { r = &y; } }");
        assert!(out.contains("const auto* r = &x;"));
    }

    #[test]
    fn test_ref_rebind_in_loop() {
        let out = transpile_str(
            "fn f() { let x = 5; let y = 10; let mut r = &x; loop { r = &y; break; } }",
        );
        assert!(out.contains("const auto* r = &x;"));
    }

    #[test]
    fn test_no_rebind_mut_binding_stays_reference() {
        // let mut r = &x with no reassignment → reference, not pointer
        let out = transpile_str("fn f() { let x = 5; let mut r = &x; }");
        assert!(out.contains("auto& r = x;") || out.contains("const auto& r = x;"));
        assert!(!out.contains("auto* r"));
    }

    // ── Implicit move insertion tests ───────────────────────────

    #[test]
    fn test_move_on_variable_binding() {
        // let b = a → auto b = std::move(a)
        let out = transpile_str("fn f() { let a = 1; let b = a; }");
        assert!(out.contains("std::move(a)"));
    }

    #[test]
    fn test_move_on_function_call_arg() {
        // consume(x) → consume(std::move(x))
        let out = transpile_str("fn f() { let x = 1; consume(x); }");
        assert!(out.contains("consume(std::move(x))"));
    }

    #[test]
    fn test_move_on_method_call_arg() {
        // v.push(item) → v.push(std::move(item))
        let out = transpile_str("fn f() { let item = 1; v.push(item); }");
        assert!(out.contains("v.push(std::move(item))"));
    }

    #[test]
    fn test_no_move_on_literal() {
        let out = transpile_str("fn f() { let x = 42; consume(42); }");
        assert!(!out.contains("std::move(42)"));
        assert!(out.contains("consume(42)"));
    }

    #[test]
    fn test_no_move_on_reference() {
        // &x should NOT be wrapped in move
        let out = transpile_str("fn f() { let x = 1; let r = &x; }");
        assert!(out.contains("const auto& r = x;"));
        assert!(!out.contains("std::move"));
    }

    #[test]
    fn test_no_move_on_ref_arg() {
        // borrow(&x) should not move — the & creates a reference
        let out = transpile_str("fn f() { let x = 1; borrow(&x); }");
        assert!(!out.contains("std::move(&x)"));
    }

    #[test]
    fn test_no_move_on_constant() {
        // ALL_CAPS names are constants, not moved
        let out = transpile_str("fn f() { let x = MAX_SIZE; }");
        assert!(!out.contains("std::move(MAX_SIZE)"));
    }

    #[test]
    fn test_no_move_on_type_name() {
        // Uppercase names are types/constructors, not moved
        let out = transpile_str("fn f() { let x = Default; }");
        assert!(!out.contains("std::move(Default)"));
    }

    #[test]
    fn test_no_move_on_bool() {
        let out = transpile_str("fn f() { let x = true; }");
        assert!(!out.contains("std::move"));
    }

    #[test]
    fn test_no_move_on_none() {
        let out = transpile_str("fn f() { let x = None; }");
        assert!(!out.contains("std::move"));
    }

    #[test]
    fn test_move_multiple_args() {
        let out = transpile_str("fn f() { let a = 1; let b = 2; process(a, b); }");
        assert!(out.contains("process(std::move(a), std::move(b))"));
    }

    #[test]
    fn test_no_move_on_expression() {
        // Complex expressions produce temporaries — no need for std::move
        let out = transpile_str("fn f() { let x = a + b; }");
        assert!(!out.contains("std::move(a + b)"));
        // But the individual variables a and b don't get moved in a binary expr
        // because binary ops borrow their operands
    }

    // ── Phase 2: Standard library type mapping tests ────────────

    #[test]
    fn test_str_param() {
        let out = transpile_str("fn f(s: &str) {}");
        assert!(out.contains("std::string_view s"));
    }

    #[test]
    fn test_box_new_mapping() {
        let out = transpile_str("fn f() { let b = Box::new(42); }");
        assert!(out.contains("rusty::Box::new_(42)"));
    }

    #[test]
    fn test_string_from_mapping() {
        let out = transpile_str(r#"fn f() { let s = String::from("hello"); }"#);
        assert!(out.contains("rusty::String::from("));
    }

    #[test]
    fn test_vec_new_mapping() {
        let out = transpile_str("fn f() { let v = Vec::new(); }");
        assert!(out.contains("rusty::Vec::new_()"));
    }

    #[test]
    fn test_condvar_type() {
        let out = transpile_str("fn f(cv: Condvar) {}");
        assert!(out.contains("rusty::Condvar cv"));
    }

    #[test]
    fn test_barrier_type() {
        let out = transpile_str("fn f(b: Barrier) {}");
        assert!(out.contains("rusty::Barrier b"));
    }

    #[test]
    fn test_once_type() {
        let out = transpile_str("fn f(o: Once) {}");
        assert!(out.contains("rusty::Once o"));
    }

    #[test]
    fn test_nested_generic_types() {
        let out = transpile_str("fn f(v: Vec<Option<String>>) {}");
        assert!(out.contains("rusty::Vec<rusty::Option<rusty::String>>"));
    }

    #[test]
    fn test_arc_mutex_nested() {
        let out = transpile_str("fn f(m: Arc<Mutex<i32>>) {}");
        assert!(out.contains("rusty::Arc<rusty::Mutex<int32_t>>"));
    }

    #[test]
    fn test_rc_type() {
        let out = transpile_str("fn f(r: Rc<i32>) {}");
        assert!(out.contains("rusty::Rc<int32_t>"));
    }

    #[test]
    fn test_weak_type() {
        let out = transpile_str("fn f(w: Weak<i32>) {}");
        assert!(out.contains("rusty::Weak<int32_t>"));
    }

    #[test]
    fn test_cell_type() {
        let out = transpile_str("fn f(c: Cell<i32>) {}");
        assert!(out.contains("rusty::Cell<int32_t>"));
    }

    #[test]
    fn test_refcell_type() {
        let out = transpile_str("fn f(r: RefCell<String>) {}");
        assert!(out.contains("rusty::RefCell<rusty::String>"));
    }

    #[test]
    fn test_vecdeque_type() {
        let out = transpile_str("fn f(d: VecDeque<i32>) {}");
        assert!(out.contains("rusty::VecDeque<int32_t>"));
    }

    #[test]
    fn test_btreemap_type() {
        let out = transpile_str("fn f(m: BTreeMap<i32, String>) {}");
        assert!(out.contains("rusty::BTreeMap<int32_t, rusty::String>"));
    }

    #[test]
    fn test_maybe_uninit_type() {
        let out = transpile_str("fn f(m: MaybeUninit<i32>) {}");
        assert!(out.contains("rusty::MaybeUninit<int32_t>"));
    }

    // ── Phase 3: Match expression tests ─────────────────────────

    #[test]
    fn test_match_int_switch() {
        let out =
            transpile_str("fn f(x: i32) { match x { 1 => { a(); } 2 => { b(); } _ => { c(); } } }");
        assert!(out.contains("switch (x) {"));
        assert!(out.contains("case 1: {"));
        assert!(out.contains("case 2: {"));
        assert!(out.contains("default: {"));
        assert!(out.contains("break;"));
    }

    #[test]
    fn test_match_multi_pattern() {
        let out = transpile_str("fn f(x: i32) { match x { 1 | 2 | 3 => { ok(); } _ => {} } }");
        assert!(out.contains("case 1:"));
        assert!(out.contains("case 2:"));
        assert!(out.contains("case 3: {"));
    }

    #[test]
    fn test_match_enum_visit() {
        let out = transpile_str(
            r#"
            enum E { A(i32), B }
            fn f(e: E) { match e { E::A(x) => { use_x(x); } E::B => { b(); } } }
        "#,
        );
        assert!(out.contains("std::visit(overloaded {"));
        assert!(out.contains("[&](const E_A& _v)"));
        assert!(out.contains("auto&& x = _v._0;"));
        assert!(out.contains("[&](const E_B&)"));
    }

    #[test]
    fn test_match_struct_variant() {
        let out = transpile_str(
            r#"
            enum M { Point { x: f64, y: f64 } }
            fn f(m: M) { match m { M::Point { x, y } => { use_point(x, y); } } }
        "#,
        );
        assert!(out.contains("[&](const M_Point& _v)"));
        assert!(out.contains("const auto& x = _v.x;"));
        assert!(out.contains("const auto& y = _v.y;"));
    }

    #[test]
    fn test_match_wildcard_arm() {
        let out = transpile_str(
            r#"
            enum E { A(i32), B }
            fn f(e: E) { match e { E::A(x) => { a(); } _ => { other(); } } }
        "#,
        );
        assert!(out.contains("[&](const auto&) {"));
        assert!(out.contains("other();"));
    }

    #[test]
    fn test_match_with_guard() {
        let out =
            transpile_str("fn f(x: i32) { match x { n if n > 0 => { pos(); } _ => { neg(); } } }");
        // Guard should use if-else inside the case
        assert!(out.contains("if ("));
    }

    #[test]
    fn test_match_unit_variant() {
        let out = transpile_str(
            r#"
            enum E { A, B, C }
            fn f(e: E) { match e { E::A => { a(); } E::B => { b(); } E::C => { c(); } } }
        "#,
        );
        assert!(out.contains("switch (e) {"));
        assert!(out.contains("case E::A: {"));
        assert!(out.contains("case E::B: {"));
        assert!(out.contains("case E::C: {"));
    }

    #[test]
    fn test_match_catch_all_binding() {
        let out = transpile_str(
            r#"
            enum E { A(i32), B }
            fn f(e: E) { match e { E::A(x) => { a(); } other => { b(); } } }
        "#,
        );
        assert!(out.contains("[&](const auto& other)"));
    }

    // ── Phase 4: Trait / Proxy facade tests ─────────────────────

    #[test]
    fn test_trait_dispatch_macros() {
        let out = transpile_str("trait Drawable { fn draw(&self); fn area(&self) -> f64; }");
        assert!(out.contains("PRO_DEF_MEM_DISPATCH(MemDrawable_draw, draw);"));
        assert!(out.contains("PRO_DEF_MEM_DISPATCH(MemDrawable_area, area);"));
    }

    #[test]
    fn test_trait_facade_builder() {
        let out = transpile_str("trait Drawable { fn draw(&self); fn area(&self) -> f64; }");
        assert!(out.contains("struct DrawableFacade : pro::facade_builder"));
        assert!(out.contains("::add_convention<MemDrawable_draw, void() const>"));
        assert!(out.contains("::add_convention<MemDrawable_area, double() const>"));
        assert!(out.contains("::build {};"));
    }

    #[test]
    fn test_trait_mut_method() {
        let out = transpile_str("trait Mutator { fn mutate(&mut self); }");
        // &mut self → no const suffix
        assert!(out.contains("::add_convention<MemMutator_mutate, void()>"));
        assert!(!out.contains("void() const"));
    }

    #[test]
    fn test_trait_method_with_params() {
        let out = transpile_str("trait Adder { fn add(&self, x: i32, y: i32) -> i32; }");
        assert!(out.contains("::add_convention<MemAdder_add, int32_t(int32_t, int32_t) const>"));
    }

    #[test]
    fn test_dyn_trait_param() {
        let out = transpile_str("trait Foo { fn bar(&self); } fn f(x: &dyn Foo) {}");
        assert!(out.contains("pro::proxy_view<FooFacade> x"));
    }

    #[test]
    fn test_box_dyn_trait() {
        let out = transpile_str("trait Foo { fn bar(&self); } fn f(x: Box<dyn Foo>) {}");
        assert!(out.contains("pro::proxy<FooFacade> x"));
    }

    #[test]
    fn test_unresolved_dyn_trait_param_falls_back_to_void_ptr() {
        let out = transpile_str("fn f(x: &dyn std::error::Error) {}");
        assert!(out.contains("void f(const void* x)"));
        assert!(!out.contains("pro::proxy_view<ErrorFacade>"));
    }

    #[test]
    fn test_unresolved_box_dyn_trait_param_falls_back_to_void_ptr() {
        let out = transpile_str("fn f(x: Box<dyn std::error::Error>) {}");
        assert!(out.contains("void f(void* x)"));
        assert!(!out.contains("pro::proxy<ErrorFacade>"));
    }

    #[test]
    fn test_impl_trait_return() {
        let out = transpile_str("trait Foo { fn bar(&self); } fn f() -> impl Foo { todo!() }");
        assert!(out.contains("pro::proxy<FooFacade> f()"));
    }

    #[test]
    fn test_trait_with_keyword_method() {
        let out = transpile_str("trait Builder { fn new(&self) -> Self; }");
        assert!(out.contains("PRO_DEF_MEM_DISPATCH(MemBuilder_new_, new_);"));
    }

    #[test]
    fn test_trait_impl_just_methods() {
        // impl Trait for Type → just emit methods on the struct (Proxy auto-resolves)
        let out = transpile_str(
            r#"
            struct Dog { name: String }
            trait Animal { fn speak(&self) -> String; }
            impl Animal for Dog {
                fn speak(&self) -> String { self.name }
            }
        "#,
        );
        // Methods should be in the struct
        assert!(out.contains("struct Dog {"));
        assert!(out.contains("rusty::String speak() const"));
    }

    #[test]
    fn test_empty_trait() {
        let out = transpile_str("trait Marker {}");
        assert!(out.contains("struct MarkerFacade : pro::facade_builder"));
        assert!(out.contains("::build {};"));
    }

    #[test]
    fn test_trait_default_method() {
        let out = transpile_str(
            r#"trait Greet {
                fn name(&self) -> String;
                fn greet(&self) -> String { self.name() }
            }"#,
        );
        // Default method should be emitted as a free function
        assert!(out.contains("rusty::String greet(pro::proxy_view<GreetFacade> _self)"));
    }

    #[test]
    fn test_trait_facade_emission_skipped_in_module_mode() {
        let out = transpile_str_module("trait Foo { fn bar(&self); }", "my_crate");
        assert!(
            out.contains("// Rust-only trait Foo (Proxy facade emission skipped in module mode)")
        );
        assert!(!out.contains("struct FooFacade : pro::facade_builder"));
    }

    #[test]
    fn test_trait_reexport_skipped_when_trait_emission_is_skipped_in_module_mode() {
        let out = transpile_str_module(
            r#"
            mod into_either {
                pub trait IntoEither {
                    fn into_either(self) -> i32;
                }
            }
            pub use into_either::IntoEither;
        "#,
            "my_crate",
        );
        assert!(out.contains(
            "// Rust-only trait IntoEither (Proxy facade emission skipped in module mode)"
        ));
        assert!(out.contains("// Rust-only: using into_either::IntoEither;"));
        assert!(!out.contains("export using into_either::IntoEither;"));
    }

    #[test]
    fn test_leaf431_trait_facade_emission_skipped_in_expanded_libtest_mode() {
        let out = transpile_str(
            r#"
            #[rustc_test_marker = "basic"]
            const BASIC: test::TestDescAndFn = unsafe { std::mem::zeroed() };
            trait IntoEither { fn into_either(self) -> i32; }
        "#,
        );
        assert!(out.contains(
            "// Rust-only trait IntoEither (Proxy facade emission skipped in expanded-test mode)"
        ));
        assert!(!out.contains("PRO_DEF_MEM_DISPATCH("));
        assert!(!out.contains("struct IntoEitherFacade : pro::facade_builder"));
    }

    // ── Phase 5: Generics / templates tests ─────────────────────

    #[test]
    fn test_generic_function() {
        let out = transpile_str("fn identity<T>(x: T) -> T { x }");
        assert!(out.contains("template<typename T>"));
        assert!(out.contains("T identity(T x)"));
    }

    #[test]
    fn test_generic_two_params() {
        let out = transpile_str("fn pair<T, U>(a: T, b: U) {}");
        assert!(out.contains("template<typename T, typename U>"));
    }

    #[test]
    fn test_generic_struct() {
        let out = transpile_str("struct Wrapper<T> { value: T }");
        assert!(out.contains("template<typename T>"));
        assert!(out.contains("struct Wrapper {"));
        assert!(out.contains("T value;"));
    }

    #[test]
    fn test_generic_method() {
        let out = transpile_str(
            r#"
            struct S<T> { v: T }
            impl<T> S<T> {
                fn map<U>(&self, x: U) -> U { x }
            }
        "#,
        );
        // Struct should have template prefix
        assert!(out.contains("template<typename T>"));
        // Method should also have its own template
        assert!(out.contains("template<typename U>"));
    }

    #[test]
    fn test_trait_bound() {
        let out = transpile_str("fn f<T: Clone>(x: T) {}");
        assert!(out.contains("template<typename T>"));
        assert!(out.contains("requires"));
        assert!(out.contains("Clone"));
    }

    #[test]
    fn test_multiple_bounds() {
        let out = transpile_str("fn f<T: Clone + Send>(x: T) {}");
        assert!(out.contains("requires"));
        assert!(out.contains("&&"));
    }

    #[test]
    fn test_where_clause() {
        let out = transpile_str("fn f<T>(x: T) where T: Clone {}");
        assert!(out.contains("template<typename T>"));
        assert!(out.contains("requires"));
        assert!(out.contains("Clone"));
    }

    #[test]
    fn test_trait_bound_constraints_skipped_in_module_mode() {
        let out = transpile_str_module("fn f<F: FnOnce(i32) -> i32>(x: F) {}", "my_crate");
        assert!(!out.contains("FnOnceFacade::is_satisfied_by"));
        assert!(!out.contains("requires"));
    }

    #[test]
    fn test_external_trait_bound_requires_skipped() {
        let out = transpile_str("fn f<T: core::hash::Hasher>(x: T) {}");
        assert!(!out.contains("HasherFacade::is_satisfied_by"));
    }

    #[test]
    fn test_fnonce_trait_bound_requires_skipped() {
        let out = transpile_str("fn f<F: FnOnce(i32) -> i32>(x: F) {}");
        assert!(!out.contains("FnOnceFacade::is_satisfied_by"));
    }

    #[test]
    fn test_leaf430_std_iterator_family_trait_bound_requires_skipped() {
        let out = transpile_str(
            r#"
            fn f<T, I, E, C, D>(t: T, i: I, e: E, c: C, d: D)
            where
                T: IntoIterator,
                I: Iterator,
                E: Extend<u8>,
                C: FromIterator<u8>,
                D: Default,
            {}
        "#,
        );
        assert!(!out.contains("IntoIteratorFacade::is_satisfied_by"));
        assert!(!out.contains("IteratorFacade::is_satisfied_by"));
        assert!(!out.contains("ExtendFacade::is_satisfied_by"));
        assert!(!out.contains("FromIteratorFacade::is_satisfied_by"));
        assert!(!out.contains("DefaultFacade::is_satisfied_by"));
    }

    #[test]
    fn test_leaf431_additional_std_traits_requires_skipped() {
        let out = transpile_str(
            r#"
            fn f<I, A, M>(iter: I, a: A, m: M)
            where
                I: DoubleEndedIterator,
                A: AsRef<str>,
                M: AsMut<str>,
            {}
        "#,
        );
        assert!(!out.contains("DoubleEndedIteratorFacade::is_satisfied_by"));
        assert!(!out.contains("AsRefFacade::is_satisfied_by"));
        assert!(!out.contains("AsMutFacade::is_satisfied_by"));
    }

    #[test]
    fn test_lifetime_erased() {
        // Lifetime params should not appear in template
        let out = transpile_str("fn f<'a>(x: &'a i32) -> &'a i32 { x }");
        assert!(!out.contains("template"));
        assert!(out.contains("const int32_t& f(const int32_t& x)"));
    }

    #[test]
    fn test_generic_with_lifetime_mixed() {
        // Only type params should appear, lifetimes erased
        let out = transpile_str("fn f<'a, T>(x: &'a T) -> T { todo!() }");
        assert!(out.contains("template<typename T>"));
        assert!(!out.contains("'a"));
    }

    #[test]
    fn test_no_template_without_generics() {
        // Non-generic function should NOT have template prefix
        let out = transpile_str("fn add(a: i32, b: i32) -> i32 { a + b }");
        assert!(!out.contains("template"));
    }

    // ── Phase 6: Closure / lambda tests ─────────────────────────

    #[test]
    fn test_closure_untyped_params() {
        let out = transpile_str("fn f() { let add = |a, b| a + b; }");
        assert!(out.contains("[&](auto a, auto b)"));
        assert!(out.contains("return a + b;"));
    }

    #[test]
    fn test_closure_typed_params() {
        let out = transpile_str("fn f() { let inc = |x: i32| x + 1; }");
        assert!(out.contains("[&](int32_t x)"));
    }

    #[test]
    fn test_closure_move_capture() {
        let out = transpile_str("fn f() { let c = move || 42; }");
        assert!(out.contains("[=]()"));
    }

    #[test]
    fn test_closure_no_params() {
        let out = transpile_str("fn f() { let c = || 42; }");
        assert!(out.contains("[&]()"));
        assert!(out.contains("return 42;"));
    }

    #[test]
    fn test_impl_fn_param() {
        let out = transpile_str("fn apply(f: impl Fn(i32) -> i32, x: i32) -> i32 { f(x) }");
        assert!(out.contains("std::function<int32_t(int32_t)> f"));
    }

    #[test]
    fn test_impl_fn_mut_param() {
        let out = transpile_str("fn apply(f: impl FnMut(i32)) {}");
        assert!(out.contains("std::function<void(int32_t)> f"));
    }

    #[test]
    fn test_impl_fn_once_param() {
        let out = transpile_str("fn apply(f: impl FnOnce() -> String) {}");
        assert!(out.contains("std::move_only_function<rusty::String()> f"));
    }

    #[test]
    fn test_dyn_fn_ref() {
        let out = transpile_str("fn apply(f: &dyn Fn(i32) -> i32) {}");
        assert!(out.contains("const std::function<int32_t(int32_t)>& f"));
    }

    #[test]
    fn test_box_dyn_fn_once() {
        let out = transpile_str("fn apply(f: Box<dyn FnOnce() -> i32>) {}");
        assert!(out.contains("std::move_only_function<int32_t()> f"));
    }

    #[test]
    fn test_fn_multi_params() {
        let out = transpile_str("fn apply(f: impl Fn(i32, f64, bool) -> String) {}");
        assert!(out.contains("std::function<rusty::String(int32_t, double, bool)>"));
    }

    // ── Phase 7: C++20 module tests ─────────────────────────────

    #[test]
    fn test_module_declaration() {
        let out = transpile_str_module("fn f() {}", "my_crate");
        assert!(out.contains("export module my_crate;"));
    }

    #[test]
    fn test_module_mode_emits_global_fragment_before_includes() {
        let out = transpile_str_module("fn f() {}", "my_crate");
        let module_idx = out.find("\nmodule;\n").unwrap();
        let include_idx = out.find("#include <cstdint>").unwrap();
        let export_idx = out.find("export module my_crate;").unwrap();
        assert!(module_idx < include_idx);
        assert!(include_idx < export_idx);
    }

    #[test]
    fn test_emits_required_cpp_and_rusty_includes() {
        let out = transpile_str("fn f() {}");
        assert!(out.contains("#include <variant>"));
        assert!(out.contains("#include <utility>"));
        assert!(out.contains("#include <rusty/rusty.hpp>"));
        assert!(out.contains("#include <rusty/try.hpp>"));
    }

    #[test]
    fn test_visit_overloaded_helper_emitted_once() {
        let out = transpile_str(
            r#"
            enum E { A(i32), B(i32) }
            fn f(e: E) { match e { E::A(x) => { use_x(x); }, E::B(y) => { use_y(y); } } }
        "#,
        );
        assert!(out.contains("struct overloaded : Ts... { using Ts::operator()...; };"));
        assert!(out.contains("overloaded(Ts...) -> overloaded<Ts...>;"));
        assert_eq!(
            out.matches("struct overloaded : Ts... { using Ts::operator()...; };")
                .count(),
            1
        );
    }

    #[test]
    fn test_visit_overloaded_helper_precedes_visit_use_in_module_mode() {
        let out = transpile_str_module(
            r#"
            pub enum E { A(i32), B(i32) }
            pub fn f(e: E) { match e { E::A(x) => { use_x(x); }, E::B(y) => { use_y(y); } } }
        "#,
            "my_crate",
        );
        let export_idx = out.find("export module my_crate;").unwrap();
        let helper_idx = out
            .find("struct overloaded : Ts... { using Ts::operator()...; };")
            .unwrap();
        let visit_idx = out.find("std::visit(overloaded {").unwrap();
        assert!(export_idx < helper_idx);
        assert!(helper_idx < visit_idx);
    }

    #[test]
    fn test_leaf42_runtime_type_paths_lowered() {
        let out = transpile_str(
            r#"
            fn f(
                x: core::option::Option<core::cmp::Ordering>,
                p: Pin<i32>,
                poll: core::task::Poll<i32>,
                cx: core::task::Context,
                args: fmt::Arguments,
                path: std::path::Path,
                os: std::ffi::OsStr,
                c: std::ffi::CStr
            ) {}
        "#,
        );
        assert!(out.contains("rusty::Option<rusty::cmp::Ordering> x"));
        assert!(out.contains("rusty::pin::Pin<int32_t> p"));
        assert!(out.contains("rusty::Poll<int32_t> poll"));
        assert!(out.contains("rusty::Context cx"));
        assert!(out.contains("rusty::fmt::Arguments args"));
        assert!(out.contains("rusty::path::Path path"));
        assert!(out.contains("rusty::ffi::OsStr os"));
        assert!(out.contains("rusty::ffi::CStr c"));
        assert!(!out.contains("core::option::Option"));
        assert!(!out.contains("core::task::Poll"));
        assert!(!out.contains("std::path::Path"));
        assert!(!out.contains("std::ffi::CStr"));
    }

    #[test]
    fn test_leaf42_runtime_function_paths_lowered() {
        let out = transpile_str(
            r#"
            fn f(x: i32, y: i32) {
                core::intrinsics::discriminant_value(x);
                core::intrinsics::unreachable();
                core::panicking::panic_fmt();
                let kind = core::panicking::AssertKind::Eq;
                core::panicking::assert_failed(kind, &x, &y, core::option::Option::None);
                core::hash::Hash::hash(x, y);
                core::fmt::Formatter::debug_tuple_field1_finish();
                Pin::new_unchecked(x);
                Pin::get_ref(x);
                Pin::get_unchecked_mut(y);
                let _ = core::cmp::Ordering::Equal;
            }
        "#,
        );
        assert!(out.contains("rusty::intrinsics::discriminant_value"));
        assert!(out.contains("rusty::intrinsics::unreachable"));
        assert!(out.contains("rusty::panicking::panic_fmt"));
        assert!(out.contains("rusty::panicking::AssertKind::Eq"));
        assert!(out.contains("rusty::panicking::assert_failed"));
        assert!(out.contains("rusty::hash::hash"));
        assert!(out.contains("rusty::fmt::Formatter::debug_tuple_field1_finish"));
        assert!(out.contains("rusty::pin::new_unchecked"));
        assert!(out.contains("rusty::pin::get_ref"));
        assert!(out.contains("rusty::pin::get_unchecked_mut"));
        assert!(out.contains("rusty::cmp::Ordering::Equal"));
        assert!(out.contains("std::nullopt"));
        assert!(!out.contains("core::intrinsics::"));
        assert!(!out.contains("core::panicking::"));
        assert!(!out.contains("core::option::Option::None"));
        assert!(!out.contains("core::hash::Hash::hash"));
    }

    #[test]
    fn test_runtime_fallback_helpers_emitted_when_needed() {
        let out = transpile_str(
            r#"
            fn f() {
                core::intrinsics::unreachable();
                core::panicking::panic_fmt();
                let kind = core::panicking::AssertKind::Eq;
                core::panicking::assert_failed(kind, &1, &2, core::option::Option::None);
                Pin::new_unchecked(1);
            }
        "#,
        );
        assert!(out.contains("namespace intrinsics {"));
        assert!(out.contains("namespace panicking {"));
        assert!(out.contains("namespace pin {"));
        assert!(out.contains("enum class Ordering { Less, Equal, Greater };"));
        assert!(out.contains("enum class AssertKind { Eq, Ne };"));
        assert!(out.contains("assert_failed(Args&&...)"));
    }

    #[test]
    fn test_runtime_fallback_helpers_not_emitted_when_unused() {
        let out = transpile_str("fn f() { let x = 1; }");
        assert!(!out.contains("namespace intrinsics {"));
        assert!(!out.contains("namespace panicking {"));
        assert!(!out.contains("namespace pin {"));
    }

    #[test]
    fn test_leaf422_core_option_none_path_lowered() {
        let out = transpile_str("fn f() { let x = core::option::Option::None; }");
        assert!(out.contains("std::nullopt"));
        assert!(!out.contains("core::option::Option::None"));
    }

    #[test]
    fn test_leaf423_some_ref_rvalue_no_address_of_rvalue() {
        let out = transpile_str("fn f() { let x = Some(&2); }");
        assert!(!out.contains("std::make_optional(&2)"));
        assert!(out.contains("static const auto _some_ref_tmp = 2"));
    }

    #[test]
    fn test_leaf424_some_ref_uses_rusty_someref_shape() {
        let out = transpile_str("fn f() { let x = Some(&2); let y = Some(&mut 2); }");
        assert!(out.contains("rusty::SomeRef("));
        assert!(!out.contains("std::make_optional([&]() { static const auto _some_ref_tmp"));
        assert!(out.contains("static auto _some_mut_ref_tmp = 2"));
    }

    #[test]
    fn test_leaf424_some_ref_lvalue_does_not_take_address() {
        let out = transpile_str("fn f() { let x = 2; let y = Some(&x); }");
        assert!(out.contains("rusty::SomeRef(x)"));
        assert!(!out.contains("rusty::SomeRef(&x)"));
    }

    #[test]
    fn test_leaf423_std_string_import_rewritten() {
        let out = transpile_str("use std::string::String;");
        assert!(out.contains("using rusty::String;"));
        assert!(!out.contains("using std::string::String;"));
    }

    #[test]
    fn test_leaf423_core_from_ctor_uses_target_conversion() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn f() -> Either<String, &'static str> {
                Left(core::convert::From::from("foo"))
            }
        "#,
        );
        assert!(out.contains(
            "return Left<rusty::String, std::string_view>(rusty::String::from(\"foo\"));"
        ));
        assert!(!out.contains("core::convert::From::from"));
        assert!(!out.contains("rusty::String::from(rusty::String::from("));
    }

    #[test]
    fn test_leaf423_match_return_arm_lowers_without_return_return() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn f() -> Either<String, &'static str> {
                Right(match Left("foo bar") {
                    Left(err) => return Left(err),
                    Right(val) => val,
                })
            }
        "#,
        );
        assert!(!out.contains("return return"));
        assert!(!out.contains("std::visit(overloaded {"));
        assert!(out.contains("_m.is_right()"));
        assert!(out.contains("return Left<rusty::String, std::string_view>("));
    }

    #[test]
    fn test_leaf423_crate_prefixed_variant_paths_use_template_args() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn make_left() -> Either<i32, i32> {
                crate::Left(1)
            }
            fn sum(x: Either<i32, i32>) -> i32 {
                match x {
                    crate::Left(v) => v,
                    crate::Right(v) => v,
                }
            }
        "#,
        );
        assert!(out.contains("return Left<int32_t, int32_t>(1);"));
        assert!(out.contains("const Either_Left<int32_t, int32_t>& _v"));
        assert!(out.contains("const Either_Right<int32_t, int32_t>& _v"));
    }

    #[test]
    fn test_leaf43_dependent_assoc_type_prefixed_with_typename() {
        let out = transpile_str("fn f<L, R>(x: Either<L::IntoIter, R::IntoIter>) {}");
        assert!(out.contains("Either<typename L::IntoIter, typename R::IntoIter> x"));
    }

    #[test]
    fn test_leaf43_qself_ref_assoc_type_normalized() {
        let out = transpile_str(
            "fn f<L>(x: <&L as IntoIterator>::IntoIter, y: <&mut L as IntoIterator>::IntoIter) {}",
        );
        assert!(out.contains("typename L::IntoIter x"));
        assert!(out.contains("typename L::IntoIter y"));
        assert!(!out.contains("const L&::IntoIter"));
        assert!(!out.contains("L&::IntoIter"));
    }

    #[test]
    fn test_leaf43_self_assoc_type_stripped_in_struct_scope() {
        let out = transpile_str(
            r#"
            struct Foo {}
            impl Foo {
                type Output = i32;
                fn poll(&self) -> Self::Output { 0 }
            }
        "#,
        );
        assert!(out.contains("using Output = int32_t;"));
        assert!(out.contains("Output poll() const"));
        assert!(!out.contains("Self::Output"));
    }

    #[test]
    fn test_leaf43_assoc_alias_uses_typename() {
        let out = transpile_str(
            r#"
            struct Foo<L> { inner: L }
            impl<L> Foo<L> {
                type Iter = L::IntoIter;
            }
        "#,
        );
        assert!(out.contains("using Iter = typename L::IntoIter;"));
    }

    #[test]
    fn test_pub_fn_exported() {
        let out = transpile_str_module("pub fn hello() {}", "my_crate");
        assert!(out.contains("export void hello()"));
    }

    #[test]
    fn test_private_fn_not_exported() {
        let out = transpile_str_module("fn helper() {}", "my_crate");
        assert!(!out.contains("export void helper"));
        assert!(out.contains("void helper()"));
    }

    #[test]
    fn test_pub_struct_exported() {
        let out = transpile_str_module("pub struct Foo { pub x: i32 }", "my_crate");
        assert!(out.contains("export struct Foo"));
    }

    #[test]
    fn test_mod_import() {
        let out = transpile_str_module("mod utils;", "my_crate");
        assert!(out.contains("import my_crate.utils;"));
    }

    #[test]
    fn test_pub_mod_export_import() {
        let out = transpile_str_module("pub mod api;", "my_crate");
        assert!(out.contains("export import my_crate.api;"));
    }

    #[test]
    fn test_use_statement() {
        let out = transpile_str_module("use std::collections::HashMap;", "my_crate");
        assert!(out.contains("using std::collections::HashMap;"));
    }

    #[test]
    fn test_pub_use_export() {
        let out = transpile_str_module("pub use crate::types::MyType;", "my_crate");
        // crate:: resolves within the same module tree (module name is not a namespace)
        assert!(out.contains("export using types::MyType;"));
    }

    #[test]
    fn test_pub_use_either_variants_rewritten_to_ctor_functions() {
        let out = transpile_str_module("pub use crate::Either::{Left, Right};", "either");
        assert!(out.contains("// Rust-only: using Either::Left;"));
        assert!(out.contains("// Rust-only: using Either::Right;"));
        assert!(!out.contains("export using Either::Left;"));
        assert!(!out.contains("export using Either::Right;"));
    }

    #[test]
    fn test_pub_use_iter_either_reexport_skipped_in_module_mode() {
        let out = transpile_str_module(
            r#"
            mod iterator {
                pub struct IterEither<L, R> { pub _0: L, pub _1: R }
            }
            pub use crate::iterator::IterEither;
        "#,
            "either",
        );
        assert!(out.contains("// Rust-only: using iterator::IterEither;"));
        assert!(!out.contains("export using iterator::IterEither;"));
    }

    #[test]
    fn test_pub_use_iter_either_reexport_kept_without_module_mode() {
        let out = transpile_str(
            r#"
            mod iterator {
                pub struct IterEither<L, R> { pub _0: L, pub _1: R }
            }
            pub use crate::iterator::IterEither;
        "#,
        );
        assert!(out.contains("using iterator::IterEither;"));
    }

    #[test]
    fn test_no_module_no_export() {
        // Without --module-name, pub doesn't emit export
        let out = transpile_str("pub fn hello() {}");
        assert!(!out.contains("export"));
        assert!(out.contains("void hello()"));
    }

    #[test]
    fn test_inline_mod() {
        let out = transpile_str("mod inner { pub fn foo() {} }");
        assert!(out.contains("namespace inner {"));
        assert!(out.contains("void foo()"));
    }

    #[test]
    fn test_inline_mod_in_module_mode_does_not_emit_import() {
        let out = transpile_str_module("mod inner { pub fn foo() {} }", "my_crate");
        assert!(!out.contains("import my_crate.inner;"));
        assert!(out.contains("namespace inner {"));
        assert!(out.contains("void foo()"));
    }

    #[test]
    fn test_leaf44_no_nested_export_prefix_for_inline_pub_items() {
        let out = transpile_str_module(
            r#"
            mod inner {
                pub struct Foo { pub x: i32 }
                pub fn f() {}
            }
        "#,
            "my_crate",
        );

        assert!(out.contains("namespace inner {"));
        assert!(out.contains("struct Foo {"));
        assert!(out.contains("void f()"));
        assert!(!out.contains("export struct Foo"));
        assert!(!out.contains("export void f()"));
    }

    #[test]
    fn test_leaf44_nested_super_group_use_emits_qualified_using_names() {
        let out = transpile_str(
            r#"
            struct Foo {}
            fn bar() {}
            mod inner {
                use super::{Foo, bar};
            }
        "#,
        );

        assert!(out.contains("namespace inner {"));
        assert!(out.contains("using ::Foo;"));
        assert!(out.contains("using ::bar;"));
        assert!(!out.contains("using Foo;"));
        assert!(!out.contains("using bar;"));
    }

    #[test]
    fn test_leaf416_unresolved_bare_super_import_is_skipped() {
        let out = transpile_str(
            r#"
            mod inner {
                use super::for_both;
            }
        "#,
        );
        assert!(out.contains("// Rust-only unresolved import: using for_both;"));
        assert!(!out.contains("using ::for_both;"));
    }

    #[test]
    fn test_leaf417_skips_libtest_metadata_const_and_static() {
        let out = transpile_str(
            r#"
            const BASIC: test::TestDescAndFn = unsafe { std::mem::zeroed() };
            static HARNESS: test::TestDescAndFn = unsafe { std::mem::zeroed() };
            const KEEP: i32 = 7;
        "#,
        );

        assert!(out.contains("// Rust-only libtest metadata const skipped: BASIC"));
        assert!(out.contains("// Rust-only libtest metadata static skipped: HARNESS"));
        assert!(out.contains("constexpr int32_t KEEP = 7;"));
        assert!(!out.contains("BASIC ="));
        assert!(!out.contains("HARNESS ="));
    }

    #[test]
    fn test_leaf417_skips_generated_libtest_main_function() {
        let out = transpile_str(
            r#"
            fn main() { test::test_main_static(&[]); }
            fn helper() {}
        "#,
        );

        assert!(out.contains("// Rust-only libtest main omitted"));
        assert!(out.contains("void helper() {"));
        assert!(!out.contains("void main("));
    }

    #[test]
    fn test_leaf417_regular_main_without_libtest_call_is_not_skipped() {
        let out = transpile_str("fn main() {}");
        assert!(!out.contains("// Rust-only libtest main omitted"));
        assert!(out.contains("void main() {"));
    }

    #[test]
    fn test_leaf419_emits_runnable_wrappers_for_libtest_markers_in_module_mode() {
        let out = transpile_str_module(
            r#"
            #[rustc_test_marker = "basic"]
            pub const basic: test::TestDescAndFn = unsafe { std::mem::zeroed() };
            fn basic() {}
        "#,
            "either",
        );

        assert!(out.contains("// Rust-only libtest metadata const skipped: basic"));
        assert!(out.contains("void basic() {"));
        assert!(out.contains("export void rusty_test_basic() {"));
        assert!(out.contains("basic();"));
    }

    #[test]
    fn test_leaf419_reports_marker_without_emitted_function() {
        let out = transpile_str_module(
            r#"
            #[rustc_test_marker = "missing_test_fn"]
            pub const marker_only: test::TestDescAndFn = unsafe { std::mem::zeroed() };
        "#,
            "either",
        );

        assert!(out.contains("// Rust-only libtest metadata const skipped: marker_only"));
        assert!(
            out.contains("// Rust-only libtest marker without emitted function: missing_test_fn")
        );
        assert!(!out.contains("rusty_test_missing_test_fn"));
    }

    #[test]
    fn test_leaf452_scoped_libtest_marker_emits_wrapper_for_nested_test_fn() {
        let out = transpile_str_module(
            r#"
            mod tests {
                fn unit_add() {}
            }
            #[rustc_test_marker = "tests::unit_add"]
            pub const unit_add: test::TestDescAndFn = unsafe { std::mem::zeroed() };
        "#,
            "fixture",
        );

        assert!(out.contains("namespace tests {"));
        assert!(out.contains("void unit_add() {"));
        assert!(out.contains("export void rusty_test_tests_unit_add() {"));
        assert!(out.contains("tests::unit_add();"));
        assert!(!out.contains("marker without emitted function: tests::unit_add"));
    }

    #[test]
    fn test_leaf452_deep_scoped_libtest_marker_emits_wrapper() {
        let out = transpile_str_module(
            r#"
            mod tests {
                mod nested {
                    fn deep_case() {}
                }
            }
            #[rustc_test_marker = "tests::nested::deep_case"]
            pub const deep_case: test::TestDescAndFn = unsafe { std::mem::zeroed() };
        "#,
            "fixture",
        );

        assert!(out.contains("export void rusty_test_tests_nested_deep_case() {"));
        assert!(out.contains("tests::nested::deep_case();"));
        assert!(!out.contains("marker without emitted function: tests::nested::deep_case"));
    }

    #[test]
    fn test_inline_mod_impl_methods_merged_into_struct() {
        let out = transpile_str(
            r#"
            mod inner {
                struct Foo { x: i32 }
                impl Foo {
                    fn clone(&self) -> Foo { Foo { x: self.x } }
                }
            }
        "#,
        );

        assert!(out.contains("namespace inner {"));
        assert!(out.contains("struct Foo {"));
        assert!(!out.contains("// Methods for Foo"));

        let ns_pos = out.find("namespace inner {").unwrap();
        let struct_pos = out[ns_pos..].find("struct Foo {").unwrap() + ns_pos;
        let close_pos = out[struct_pos..].find("};").unwrap() + struct_pos;
        let clone_pos = out.find("Foo clone() const {").unwrap();
        assert!(clone_pos > struct_pos && clone_pos < close_pos);
    }

    #[test]
    fn test_inline_mod_enum_impl_methods_merged_into_wrapper() {
        let out = transpile_str(
            r#"
            mod inner {
                enum E { A(i32), B(i32) }
                impl E {
                    fn is_a(&self) -> bool { true }
                }
            }
        "#,
        );

        assert!(out.contains("namespace inner {"));
        assert!(out.contains("struct E : std::variant<"));
        assert!(!out.contains("// Methods for E"));

        let ns_pos = out.find("namespace inner {").unwrap();
        let struct_pos = out[ns_pos..].find("struct E : std::variant<").unwrap() + ns_pos;
        let close_pos = out[struct_pos..].find("};").unwrap() + struct_pos;
        let method_pos = out.find("bool is_a() const {").unwrap();
        assert!(method_pos > struct_pos && method_pos < close_pos);
    }

    #[test]
    fn test_leaf428_inline_module_impl_for_imported_top_level_type_merges_into_top_level_type() {
        let out = transpile_str_module(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            mod iterator {
                use super::Either;
                trait IterLike { fn next(&mut self) -> Option<i32>; }
                impl<L, R> IterLike for Either<L, R> {
                    fn next(&mut self) -> Option<i32> { None }
                }
            }
        "#,
            "either",
        );

        assert!(out.contains("struct Either : std::variant<"));
        assert!(out.contains("rusty::Option<int32_t> next() {"));
        assert!(!out.contains("// Methods for iterator::Either"));
    }

    #[test]
    fn test_leaf428_iterator_trait_impl_on_imported_top_level_either_emits_next_and_count() {
        let out = transpile_str_module(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            mod iterator {
                use super::Either;
                use std::iter::Iterator;
                impl<L, R> Iterator for Either<L, R> {
                    type Item = i32;
                    fn next(&mut self) -> Option<Self::Item> { todo!() }
                    fn count(self) -> usize { todo!() }
                }
            }
        "#,
            "either",
        );

        assert!(out.contains("auto next() {"));
        assert!(out.contains("size_t count() {"));
        assert!(!out.contains("// Methods for iterator::Either"));
    }

    #[test]
    fn test_submodule_name() {
        let out = transpile_str_module("mod sub;", "my_crate.parent");
        assert!(out.contains("import my_crate.parent.sub;"));
    }

    // ── Phase 8: Async/await tests ──────────────────────────────

    #[test]
    fn test_async_fn_returns_task() {
        let out = transpile_str("async fn fetch() -> String { todo!() }");
        assert!(out.contains("rusty::Task<rusty::String> fetch()"));
    }

    #[test]
    fn test_async_fn_void_returns_task_void() {
        let out = transpile_str("async fn work() {}");
        assert!(out.contains("rusty::Task<void> work()"));
    }

    #[test]
    fn test_await_to_co_await() {
        let out = transpile_str("async fn f() -> i32 { let x = foo().await; x }");
        assert!(out.contains("co_await foo()"));
    }

    #[test]
    fn test_async_tail_co_return() {
        let out = transpile_str("async fn f() -> i32 { 42 }");
        assert!(out.contains("co_return 42;"));
    }

    #[test]
    fn test_sync_fn_uses_return() {
        let out = transpile_str("fn f() -> i32 { 42 }");
        assert!(out.contains("return 42;"));
        assert!(!out.contains("co_return"));
    }

    #[test]
    fn test_async_explicit_return() {
        let out = transpile_str("async fn f() -> i32 { return 42; }");
        assert!(out.contains("co_return 42;"));
    }

    #[test]
    fn test_async_chained_await() {
        let out = transpile_str("async fn f() { client.get(url).send().await; }");
        assert!(out.contains("co_await client.get(std::move(url)).send()"));
    }

    #[test]
    fn test_async_with_params() {
        let out = transpile_str("async fn process(data: Vec<i32>) -> bool { true }");
        assert!(out.contains("rusty::Task<bool> process(rusty::Vec<int32_t> data)"));
        assert!(out.contains("co_return true;"));
    }

    // ── Phase 9: Macro and derive tests ─────────────────────────

    #[test]
    fn test_println_macro() {
        let out = transpile_str(r#"fn f() { println!("hello"); }"#);
        assert!(out.contains("std::println("));
    }

    #[test]
    fn test_format_macro() {
        let out = transpile_str(r#"fn f() { let s = format!("val: {}", 42); }"#);
        assert!(out.contains("std::format("));
    }

    #[test]
    fn test_vec_macro() {
        let out = transpile_str("fn f() { let v = vec![1, 2, 3]; }");
        assert!(out.contains("rusty::Vec{"));
    }

    #[test]
    fn test_todo_macro() {
        let out = transpile_str("fn f() { todo!(); }");
        assert!(out.contains("throw std::logic_error"));
    }

    #[test]
    fn test_assert_macro() {
        let out = transpile_str("fn f() { assert!(x > 0); }");
        assert!(out.contains("assert("));
    }

    #[test]
    fn test_assert_eq_macro() {
        let out = transpile_str("fn f() { assert_eq!(a, b); }");
        assert!(out.contains("assert((a == b))"));
    }

    #[test]
    fn test_panic_macro() {
        let out = transpile_str(r#"fn f() { panic!("error"); }"#);
        assert!(out.contains("std::abort()"));
    }

    #[test]
    fn test_derive_clone() {
        let out = transpile_str("#[derive(Clone)] struct S { x: i32 }");
        assert!(out.contains("S clone() const { return *this; }"));
    }

    #[test]
    fn test_derive_partial_eq() {
        let out = transpile_str("#[derive(PartialEq)] struct S { x: i32 }");
        assert!(out.contains("operator==(const auto&) const = default"));
    }

    #[test]
    fn test_derive_partial_ord() {
        let out = transpile_str("#[derive(PartialOrd)] struct S { x: i32 }");
        assert!(out.contains("operator<=>(const auto&) const = default"));
    }

    #[test]
    fn test_derive_default() {
        let out = transpile_str("#[derive(Default)] struct S { x: i32 }");
        assert!(out.contains("static S default_()"));
    }

    #[test]
    fn test_derive_debug() {
        let out = transpile_str("#[derive(Debug)] struct S { x: i32 }");
        assert!(out.contains("operator<<(std::ostream&"));
    }

    #[test]
    fn test_derive_hash() {
        let out = transpile_str("#[derive(Hash)] struct S { x: i32 }");
        assert!(out.contains("struct std::hash<S>"));
    }

    #[test]
    fn test_multiple_derives() {
        let out = transpile_str("#[derive(Clone, PartialEq, Debug)] struct P { x: f64 }");
        assert!(out.contains("clone() const"));
        assert!(out.contains("operator=="));
        assert!(out.contains("operator<<"));
    }

    #[test]
    fn test_todo_expr() {
        let out = transpile_str("fn f() -> i32 { todo!() }");
        assert!(out.contains("throw std::logic_error"));
    }

    #[test]
    fn test_leaf4173_for_both_lowers_read_write_seek_deref_fmt_paths() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            struct IO;
            impl IO {
                fn read(&mut self, _buf: &mut [u8]) -> usize { 0 }
                fn write(&mut self, _buf: &[u8]) -> usize { 0 }
                fn seek(&mut self, _pos: i32) -> usize { 0 }
                fn fmt(&self, _f: i32) -> usize { 0 }
            }
            fn read_like(e: &mut Either<IO, IO>, buf: &mut [u8]) -> usize {
                for_both!(*e, ref mut inner => inner.read(buf))
            }
            fn write_like(e: &mut Either<IO, IO>, buf: &[u8]) -> usize {
                for_both!(*e, ref mut inner => inner.write(buf))
            }
            fn seek_like(e: &mut Either<IO, IO>, pos: i32) -> usize {
                for_both!(*e, ref mut inner => inner.seek(pos))
            }
            fn deref_like(e: &Either<IO, IO>) -> &IO {
                for_both!(*e, ref inner => inner)
            }
            fn fmt_like(e: &Either<IO, IO>, f: i32) -> usize {
                for_both!(*e, ref inner => inner.fmt(f))
            }
        "#,
        );
        assert!(!out.contains("/* for_both!("));
        assert!(out.contains("return rusty::io::read(inner, std::move(buf));"));
        assert!(out.contains("return rusty::io::write(inner, std::move(buf));"));
        assert!(out.contains("return inner.seek(std::move(pos));"));
        assert!(out.contains("const auto& inner = _v._0; return inner;"));
        assert!(out.contains("return inner.fmt(std::move(f));"));
    }

    #[test]
    fn test_leaf439_for_both_read_uses_io_dispatch_helper() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            struct IO;
            impl IO {
                fn read(&mut self, _buf: &mut [u8]) -> usize { 0 }
            }
            fn read_like(e: &mut Either<IO, IO>, buf: &mut [u8]) -> usize {
                for_both!(*e, ref mut inner => inner.read(buf))
            }
        "#,
        );
        assert!(out.contains("return rusty::io::read(inner, std::move(buf));"));
        assert!(!out.contains("return inner.read(std::move(buf));"));
    }

    #[test]
    fn test_leaf439_for_both_write_uses_io_dispatch_helper() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            struct IO;
            impl IO {
                fn write(&mut self, _buf: &[u8]) -> usize { 0 }
            }
            fn write_like(e: &mut Either<IO, IO>, buf: &[u8]) -> usize {
                for_both!(*e, ref mut inner => inner.write(buf))
            }
        "#,
        );
        assert!(out.contains("return rusty::io::write(inner, std::move(buf));"));
        assert!(!out.contains("return inner.write(std::move(buf));"));
    }

    #[test]
    fn test_leaf439_match_bound_inner_read_write_use_io_dispatch_helper() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            struct IO;
            impl IO {
                fn read(&mut self, _buf: &mut [u8]) -> usize { 0 }
                fn write(&mut self, _buf: &[u8]) -> usize { 0 }
            }
            fn read_like(e: &mut Either<IO, IO>, buf: &mut [u8]) -> usize {
                match e {
                    Either::Left(ref mut inner) => inner.read(buf),
                    Either::Right(ref mut inner) => inner.read(buf),
                }
            }
            fn write_like(e: &mut Either<IO, IO>, buf: &[u8]) -> usize {
                match e {
                    Either::Left(ref mut inner) => inner.write(buf),
                    Either::Right(ref mut inner) => inner.write(buf),
                }
            }
        "#,
        );
        assert!(out.contains("return rusty::io::read(inner, std::move(buf));"));
        assert!(out.contains("return rusty::io::write(inner, std::move(buf));"));
        assert!(!out.contains("return inner.read(std::move(buf));"));
        assert!(!out.contains("return inner.write(std::move(buf));"));
    }

    #[test]
    fn test_leaf440_match_bound_inner_description_uses_error_dispatch_helper() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            struct E;
            impl E {
                fn description(&self) -> &str { "e" }
            }
            fn describe(e: &Either<E, E>) -> &str {
                match *e {
                    Either::Left(ref inner) => inner.description(),
                    Either::Right(ref inner) => inner.description(),
                }
            }
        "#,
        );
        assert!(out.contains("return rusty::error::description(inner);"));
        assert!(!out.contains("return inner.description();"));
    }

    #[test]
    fn test_leaf440_non_inner_description_call_is_not_rewritten() {
        let out = transpile_str(
            r#"
            struct E;
            impl E {
                fn description(&self) -> &str { "e" }
            }
            fn describe(err: &E) -> &str {
                err.description()
            }
        "#,
        );
        assert!(out.contains("return err.description();"));
        assert!(!out.contains("rusty::error::description(err)"));
    }

    #[test]
    fn test_leaf441_tuple_visit_unreachable_fallback_is_typed_for_bool() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn eq_like(a: &Either<i32, i32>, b: &Either<i32, i32>) -> bool {
                match (a, b) {
                    (Either::Left(x), Either::Left(y)) => x == y,
                    (Either::Right(x), Either::Right(y)) => x == y,
                    _ => core::intrinsics::unreachable(),
                }
            }
        "#,
        );
        assert!(out.contains(
            "[&](const auto&...) { return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }"
        ));
        assert!(!out.contains("[&](const auto&...) { return rusty::intrinsics::unreachable(); }"));
    }

    #[test]
    fn test_leaf441_variant_guard_unreachable_fallback_is_typed_for_bool() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn guard_like(e: &Either<i32, i32>) -> bool {
                match *e {
                    Either::Left(x) if x > 0 => true,
                    _ => false,
                }
            }
        "#,
        );
        assert!(out.contains("return [&]() -> bool { rusty::intrinsics::unreachable(); }();"));
        assert!(!out.contains("return rusty::intrinsics::unreachable();"));
    }

    #[test]
    fn test_leaf441_logical_binary_propagates_bool_expected_type_to_match_rhs() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn logical_match_rhs(a: &Either<i32, i32>, b: &Either<i32, i32>) -> bool {
                true && match (a, b) {
                    (Either::Left(x), Either::Left(y)) => x == y,
                    (Either::Right(x), Either::Right(y)) => x == y,
                    _ => core::intrinsics::unreachable(),
                }
            }
        "#,
        );
        assert!(out.contains("return true &&"));
        assert!(out.contains(
            "[&](const auto&...) { return [&]() -> bool { rusty::intrinsics::unreachable(); }(); }"
        ));
        assert!(!out.contains("[&](const auto&...) { return rusty::intrinsics::unreachable(); }"));
    }

    #[test]
    fn test_leaf441_unsafe_unreachable_arm_is_typed_in_logical_match_rhs() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn logical_match_rhs_unsafe(a: &Either<i32, i32>, b: &Either<i32, i32>) -> bool {
                true && match (a, b) {
                    (Either::Left(x), Either::Left(y)) => x == y,
                    (Either::Right(x), Either::Right(y)) => x == y,
                    _ => unsafe { core::intrinsics::unreachable() },
                }
            }
        "#,
        );
        assert!(out.contains("return true &&"));
        assert!(out.contains("[&]() -> bool { rusty::intrinsics::unreachable(); }()"));
        assert!(!out.contains("[&](const auto&...) { return rusty::intrinsics::unreachable(); }"));
    }

    #[test]
    fn test_leaf4173_for_both_unsupported_pattern_uses_comment_fallback() {
        let out = transpile_str(
            r#"
            fn f(e: i32) -> i32 {
                for_both!(e, (a, b) => a)
            }
        "#,
        );
        assert!(out.contains("/* for_both!(e , (a , b) => a) */"));
    }

    // ── Phase 10: ? operator tests ──────────────────────────────

    #[test]
    fn test_try_on_result() {
        let out = transpile_str("fn f() -> Result<i32, String> { let x = parse()?; Ok(x) }");
        assert!(out.contains("RUSTY_TRY(parse())"));
    }

    #[test]
    fn test_try_on_option_uses_try_opt() {
        let out = transpile_str("fn f(opt: Option<i32>) -> Option<i32> { let x = opt?; Some(x) }");
        assert!(out.contains("RUSTY_TRY_OPT(opt)"));
        assert!(!out.contains("RUSTY_TRY(opt)"));
    }

    #[test]
    fn test_try_on_generic_option_uses_try_opt() {
        let out = transpile_str("fn f<T>(opt: Option<T>) -> Option<T> { Some(opt?) }");
        assert!(out.contains("RUSTY_TRY_OPT(opt)"));
    }

    #[test]
    fn test_try_on_method_call() {
        let out = transpile_str("fn f() -> Result<i32, String> { let x = foo.bar()?; Ok(x) }");
        assert!(out.contains("RUSTY_TRY(foo.bar())"));
    }

    #[test]
    fn test_try_in_async() {
        let out = transpile_str("async fn f() -> Result<i32, String> { let x = parse()?; Ok(x) }");
        assert!(out.contains("RUSTY_CO_TRY(parse())"));
    }

    #[test]
    fn test_try_on_option_in_async_uses_co_try_opt() {
        let out =
            transpile_str("async fn f(opt: Option<i32>) -> Option<i32> { let x = opt?; Some(x) }");
        assert!(out.contains("RUSTY_CO_TRY_OPT(opt)"));
        assert!(!out.contains("RUSTY_CO_TRY(opt)"));
    }

    #[test]
    fn test_try_with_await() {
        let out = transpile_str(
            "async fn f() -> Result<String, String> { let x = fetch().await?; Ok(x) }",
        );
        assert!(out.contains("RUSTY_CO_TRY(co_await fetch())"));
    }

    #[test]
    fn test_try_chained() {
        let out = transpile_str("fn f() -> Result<i32, String> { let x = a()?.b()?; Ok(x) }");
        // Inner ? → RUSTY_TRY, outer ? wraps the method call on the result
        assert!(out.contains("RUSTY_TRY("));
    }

    #[test]
    fn test_try_not_in_sync() {
        // In sync context, should use RUSTY_TRY not RUSTY_CO_TRY
        let out = transpile_str("fn f() -> Result<i32, String> { let x = g()?; Ok(x) }");
        assert!(out.contains("RUSTY_TRY("));
        assert!(!out.contains("RUSTY_CO_TRY"));
    }

    // ── Phase 11: Doc comments, tests, build integration ────────

    #[test]
    fn test_doc_comment_on_function() {
        let out = transpile_str("/// Does something.\nfn f() {}");
        assert!(out.contains("/// Does something."));
    }

    #[test]
    fn test_doc_comment_on_struct() {
        let out = transpile_str("/// A point.\nstruct Point { x: f64 }");
        assert!(out.contains("/// A point."));
        assert!(out.contains("struct Point"));
    }

    #[test]
    fn test_doc_comment_on_field() {
        let out = transpile_str("struct S {\n/// The value.\nx: i32\n}");
        assert!(out.contains("/// The value."));
        assert!(out.contains("int32_t x;"));
    }

    #[test]
    fn test_doc_comment_multiline() {
        let out = transpile_str("/// Line 1.\n/// Line 2.\nfn f() {}");
        assert!(out.contains("/// Line 1."));
        assert!(out.contains("/// Line 2."));
    }

    #[test]
    fn test_test_attr_to_test_case() {
        let out = transpile_str("#[test]\nfn test_add() { assert!(true); }");
        assert!(out.contains("TEST_CASE(\"test_add\")"));
    }

    #[test]
    fn test_test_function_body() {
        let out = transpile_str("#[test]\nfn test_math() { let x = 1 + 2; }");
        assert!(out.contains("TEST_CASE(\"test_math\") {"));
        assert!(out.contains("const auto x = 1 + 2;"));
    }

    #[test]
    fn test_cfg_test_module_omitted() {
        let out = transpile_str("#[cfg(test)]\nmod tests { fn t() {} }");
        assert!(out.contains("// #[cfg(test)] module omitted"));
        assert!(!out.contains("namespace tests"));
    }

    #[test]
    fn test_normal_function_no_test_case() {
        let out = transpile_str("fn regular() {}");
        assert!(!out.contains("TEST_CASE"));
        assert!(out.contains("void regular()"));
    }

    // ── if let pattern tests ────────────────────────────────────

    #[test]
    fn test_if_let_some() {
        let out = transpile_str("fn f(opt: Option<i32>) { if let Some(v) = opt { use_v(v); } }");
        assert!(out.contains("if (opt.is_some()) {"));
        assert!(out.contains("auto v = opt.unwrap();"));
    }

    #[test]
    fn test_if_let_some_with_else() {
        let out = transpile_str(
            "fn f(opt: Option<i32>) { if let Some(v) = opt { ok(v); } else { fail(); } }",
        );
        assert!(out.contains("if (opt.is_some()) {"));
        assert!(out.contains("} else {"));
        assert!(out.contains("fail();"));
    }

    #[test]
    fn test_if_let_ok() {
        let out = transpile_str("fn f(r: Result<i32, String>) { if let Ok(v) = r { use_v(v); } }");
        assert!(out.contains("if (r.is_ok()) {"));
        assert!(out.contains("auto v = r.unwrap();"));
    }

    #[test]
    fn test_if_let_err() {
        let out =
            transpile_str("fn f(r: Result<i32, String>) { if let Err(e) = r { report(e); } }");
        assert!(out.contains("if (r.is_err()) {"));
        assert!(out.contains("auto e = r.unwrap_err();"));
    }

    #[test]
    fn test_if_let_none() {
        let out = transpile_str("fn f(opt: Option<i32>) { if let None = opt { nothing(); } }");
        assert!(out.contains("if (opt.is_none()) {"));
    }

    // ── Operator trait tests ────────────────────────────────────

    #[test]
    fn test_impl_add_operator() {
        let out = transpile_str(
            r#"
            struct V { x: f64 }
            impl std::ops::Add for V {
                type Output = V;
                fn add(self, other: V) -> V { V { x: self.x + other.x } }
            }
        "#,
        );
        assert!(out.contains("operator+("));
        assert!(!out.contains("using Output"));
    }

    #[test]
    fn test_impl_sub_operator() {
        let out = transpile_str(
            r#"
            struct V { x: f64 }
            impl std::ops::Sub for V {
                type Output = V;
                fn sub(self, other: V) -> V { V { x: self.x - other.x } }
            }
        "#,
        );
        assert!(out.contains("operator-("));
    }

    #[test]
    fn test_impl_neg_operator() {
        let out = transpile_str(
            r#"
            struct V { x: f64 }
            impl std::ops::Neg for V {
                type Output = V;
                fn neg(self) -> V { V { x: -self.x } }
            }
        "#,
        );
        assert!(out.contains("operator-()"));
    }

    #[test]
    fn test_impl_partial_eq_operator() {
        let out = transpile_str(
            r#"
            struct V { x: f64 }
            impl PartialEq for V {
                fn eq(&self, other: &V) -> bool { self.x == other.x }
            }
        "#,
        );
        assert!(out.contains("operator==("));
    }

    #[test]
    fn test_impl_mul_operator() {
        let out = transpile_str(
            r#"
            struct V { x: f64 }
            impl std::ops::Mul for V {
                type Output = V;
                fn mul(self, other: V) -> V { V { x: self.x * other.x } }
            }
        "#,
        );
        assert!(out.contains("operator*("));
    }

    #[test]
    fn test_impl_index_operator() {
        let out = transpile_str(
            r#"
            struct V { data: Vec<i32> }
            impl std::ops::Index<usize> for V {
                type Output = i32;
                fn index(&self, i: usize) -> &i32 { todo!() }
            }
        "#,
        );
        assert!(out.contains("operator[]("));
    }

    #[test]
    fn test_output_type_suppressed() {
        // `type Output = T` from operator traits should not appear in output
        let out = transpile_str(
            r#"
            struct N { v: i32 }
            impl std::ops::Add for N {
                type Output = N;
                fn add(self, other: N) -> N { N { v: self.v + other.v } }
            }
        "#,
        );
        assert!(!out.contains("using Output"));
    }

    #[test]
    fn test_non_operator_method_unchanged() {
        // Regular impl methods should not be affected by operator renaming
        let out = transpile_str(
            r#"
            struct S { v: i32 }
            impl S { fn add(&self, x: i32) -> i32 { self.v + x } }
        "#,
        );
        assert!(out.contains("int32_t add("));
        assert!(!out.contains("operator+"));
    }

    // ── Function pointer type tests ─────────────────────────────

    #[test]
    fn test_fn_pointer_safe() {
        let out = transpile_str("fn f(callback: fn(i32) -> i32) {}");
        assert!(out.contains("rusty::SafeFn<int32_t(int32_t)>"));
    }

    #[test]
    fn test_fn_pointer_unsafe() {
        let out = transpile_str("fn f(callback: unsafe fn(i32) -> i32) {}");
        assert!(out.contains("rusty::UnsafeFn<int32_t(int32_t)>"));
    }

    #[test]
    fn test_fn_pointer_void() {
        let out = transpile_str("fn f(callback: fn()) {}");
        assert!(out.contains("rusty::SafeFn<void()>"));
    }

    #[test]
    fn test_fn_pointer_multi_param() {
        let out = transpile_str("fn f(callback: fn(i32, f64) -> bool) {}");
        assert!(out.contains("rusty::SafeFn<bool(int32_t, double)>"));
    }

    #[test]
    fn test_pub_crate_not_exported() {
        // pub(crate) should not generate export in module mode
        let out = transpile_str_module("pub(crate) fn internal() {}", "my_crate");
        assert!(!out.contains("export void internal"));
        assert!(out.contains("void internal()"));
    }

    // ── Recursive enum tests ────────────────────────────────────

    #[test]
    fn test_recursive_enum_forward_decl() {
        let out = transpile_str("enum List { Cons(i32, Box<List>), Nil }");
        assert!(out.contains("struct List;  // forward declaration"));
    }

    #[test]
    fn test_recursive_enum_struct_wrapper() {
        let out = transpile_str("enum List { Cons(i32, Box<List>), Nil }");
        assert!(out.contains("struct List : std::variant<"));
        assert!(out.contains("using variant = std::variant<"));
        assert!(out.contains("using variant::variant;"));
    }

    #[test]
    fn test_non_recursive_enum_uses_using() {
        let out = transpile_str("enum Simple { A(i32), B(f64) }");
        assert!(out.contains("using Simple = std::variant<"));
        assert!(!out.contains("struct Simple;  // forward"));
    }

    #[test]
    fn test_recursive_enum_box_field() {
        let out = transpile_str("enum Expr { Num(f64), Add(Box<Expr>, Box<Expr>) }");
        assert!(out.contains("rusty::Box<Expr> _0;"));
        assert!(out.contains("struct Expr;  // forward"));
    }

    // ── Marker trait tests ──────────────────────────────────────

    #[test]
    fn test_marker_trait_send() {
        let out = transpile_str("trait Send {}");
        assert!(out.contains("concept Send = true;"));
    }

    #[test]
    fn test_marker_trait_copy() {
        let out = transpile_str("trait Copy {}");
        assert!(out.contains("concept Copy = true;"));
    }

    #[test]
    fn test_regular_trait_not_concept() {
        let out = transpile_str("trait Foo { fn bar(&self); }");
        assert!(!out.contains("concept"));
        assert!(out.contains("FooFacade"));
    }

    // ── Multiple trait bounds and supertrait tests ───────────────

    #[test]
    fn test_dyn_multi_trait() {
        let out = transpile_str(
            "trait A { fn a(&self); } trait B { fn b(&self); } fn f(x: &(dyn A + B)) {}",
        );
        assert!(out.contains("pro::proxy_view<AAndBFacade>"));
    }

    #[test]
    fn test_impl_multi_trait() {
        let out = transpile_str(
            "trait A { fn a(&self); } trait B { fn b(&self); } fn f(x: impl A + B) {}",
        );
        assert!(out.contains("pro::proxy<AAndBFacade>"));
    }

    #[test]
    fn test_box_dyn_multi_trait() {
        let out = transpile_str(
            "trait A { fn a(&self); } trait B { fn b(&self); } fn f(x: Box<dyn A + B>) {}",
        );
        assert!(out.contains("pro::proxy<AAndBFacade>"));
    }

    #[test]
    fn test_single_trait_no_and() {
        let out = transpile_str("trait A { fn a(&self); } fn f(x: &dyn A) {}");
        assert!(out.contains("pro::proxy_view<AFacade>"));
        assert!(!out.contains("And"));
    }

    #[test]
    fn test_supertrait_comment() {
        let out = transpile_str(
            "trait Base { fn base(&self); } trait Derived: Base { fn derived(&self); }",
        );
        assert!(out.contains("// Requires: BaseFacade"));
        assert!(out.contains("DerivedFacade"));
    }

    // ── use crate:: path rewriting tests ────────────────────────

    #[test]
    fn test_use_crate_rewritten() {
        let out = transpile_str_module("use crate::utils::helper;", "my_crate");
        assert!(out.contains("using utils::helper;"));
        // Should not contain bare "crate::"
        assert!(!out.contains("using crate::"));
    }

    #[test]
    fn test_use_self_rewritten() {
        let out = transpile_str_module("use self::internal::Secret;", "my_crate");
        assert!(out.contains("using internal::Secret;"));
        assert!(!out.contains("self::"));
    }

    #[test]
    fn test_use_std_preserved() {
        let out = transpile_str("use std::collections::HashMap;");
        assert!(out.contains("using std::collections::HashMap;"));
    }

    #[test]
    fn test_use_crate_without_module_name() {
        // Without --module-name, crate:: is stripped (just use the path)
        let out = transpile_str("use crate::foo::bar;");
        assert!(out.contains("using foo::bar;"));
        assert!(!out.contains("using crate::"));
    }

    #[test]
    fn test_use_external_crate_comment() {
        let out = transpile_str("use serde::Serialize;");
        assert!(out.contains("// TODO: external crate 'serde'"));
        assert!(out.contains("using serde::Serialize;"));
    }

    #[test]
    fn test_use_std_no_external_comment() {
        let out = transpile_str("use std::io::Read;");
        assert!(!out.contains("// TODO: external crate"));
    }

    #[test]
    fn test_use_std_prelude_glob_skipped_as_rust_only() {
        let out = transpile_str("use std::prelude::rust_2018::*;");
        assert!(out.contains("// Rust-only: using namespace std::prelude::rust_2018;"));
        assert!(!out.contains("\nusing namespace std::prelude::rust_2018;"));
    }

    #[test]
    fn test_use_core_prelude_glob_skipped_as_rust_only() {
        let out = transpile_str("use core::prelude::rust_2018::*;");
        assert!(out.contains("// Rust-only: using namespace std::prelude::rust_2018;"));
        assert!(!out.contains("\nusing namespace std::prelude::rust_2018;"));
    }

    #[test]
    fn test_use_crate_no_external_comment() {
        let out = transpile_str_module("use crate::types::Foo;", "my_app");
        assert!(!out.contains("// TODO: external crate"));
    }

    // ── Phase 15 Gap 1: Generic enum tests ──────────────────────

    #[test]
    fn test_generic_enum_two_params() {
        let out = transpile_str("enum Either<L, R> { Left(L), Right(R) }");
        assert!(out.contains("template<typename L, typename R>"));
        assert!(out.contains("struct Either_Left {"));
        assert!(out.contains("L _0;"));
        assert!(out.contains("struct Either_Right {"));
        assert!(out.contains("R _0;"));
        assert!(out.contains("using Either = std::variant<Either_Left<L, R>, Either_Right<L, R>>"));
    }

    #[test]
    fn test_generic_enum_one_param() {
        let out = transpile_str("enum Maybe<T> { Just(T), Nothing }");
        assert!(out.contains("template<typename T>"));
        assert!(out.contains("struct Maybe_Just {"));
        assert!(out.contains("T _0;"));
        assert!(out.contains("Maybe_Just<T>"));
        assert!(out.contains("Maybe_Nothing<T>"));
    }

    #[test]
    fn test_generic_enum_named_fields() {
        let out = transpile_str("enum Result<T, E> { Ok { value: T }, Err { error: E } }");
        assert!(out.contains("template<typename T, typename E>"));
        assert!(out.contains("T value;"));
        assert!(out.contains("E error;"));
    }

    #[test]
    fn test_generic_recursive_enum() {
        let out = transpile_str("enum List<T> { Cons(T, Box<List<T>>), Nil }");
        assert!(out.contains("template<typename T>"));
        assert!(out.contains("struct List;  // forward declaration"));
        assert!(out.contains("rusty::Box<List<T>>"));
        assert!(out.contains("struct List : std::variant<List_Cons<T>, List_Nil<T>>"));
    }

    #[test]
    fn test_non_generic_enum_unchanged() {
        let out = transpile_str("enum Color { Red(u8), Green(u8), Blue(u8) }");
        assert!(!out.contains("template"));
        assert!(out.contains("struct Color_Red {"));
        assert!(out.contains("using Color = std::variant<Color_Red, Color_Green, Color_Blue>"));
    }

    // ── Phase 15 Gap 2: core::/alloc:: path mapping ─────────────

    #[test]
    fn test_use_core_maps_to_std() {
        let out = transpile_str("use core::convert::AsRef;");
        assert!(out.contains("using std::convert::AsRef;"));
        assert!(!out.contains("core::"));
    }

    #[test]
    fn test_use_alloc_maps_to_std() {
        let out = transpile_str("use alloc::vec::Vec;");
        assert!(out.contains("using std::vec::Vec;"));
        assert!(!out.contains("alloc::"));
    }

    #[test]
    fn test_use_core_no_external_crate_comment() {
        let out = transpile_str("use core::fmt::Display;");
        assert!(!out.contains("// TODO: external crate"));
    }

    // ── Phase 15 Gap 3: Group use import expansion ──────────────

    #[test]
    fn test_use_group_expanded() {
        let out = transpile_str("use std::io::{Read, SeekFrom};");
        assert!(out.contains("// Rust-only: using std::io::Read;"));
        assert!(out.contains("using rusty::io::SeekFrom;"));
        assert!(!out.contains("{"));
    }

    #[test]
    fn test_use_group_with_self() {
        let out = transpile_str("use std::io::{self, BufRead};");
        assert!(out.contains("namespace io = rusty::io;"));
        assert!(out.contains("// Rust-only: using std::io::BufRead;"));
        assert!(!out.contains("using std::io;"));
    }

    #[test]
    fn test_use_group_core_mapped() {
        let out = transpile_str("use core::convert::{AsRef, AsMut};");
        assert!(out.contains("using std::convert::AsRef;"));
        assert!(out.contains("using std::convert::AsMut;"));
    }

    #[test]
    fn test_use_group_crate_rewritten() {
        let out = transpile_str_module("use crate::types::{Foo, Bar};", "my_app");
        assert!(out.contains("using types::Foo;"));
        assert!(out.contains("using types::Bar;"));
    }

    #[test]
    fn test_use_group_three_items() {
        let out = transpile_str("use std::collections::{HashMap, HashSet, BTreeMap};");
        assert!(out.contains("using std::collections::HashMap;"));
        assert!(out.contains("using std::collections::HashSet;"));
        assert!(out.contains("using std::collections::BTreeMap;"));
    }

    // ── Phase 15 Gap 4: Unhandled item kinds ────────────────────

    #[test]
    fn test_extern_crate() {
        let out = transpile_str("extern crate serde;");
        assert!(out.contains("// extern crate serde"));
        assert!(!out.contains("// TODO: unhandled"));
    }

    #[test]
    fn test_macro_rules_skipped() {
        let out = transpile_str(r#"macro_rules! my_macro { ($x:expr) => { $x + 1 }; }"#);
        assert!(out.contains("// macro_rules! my_macro"));
        assert!(!out.contains("// TODO: unhandled"));
    }

    #[test]
    fn test_leaf416_macro_rules_import_is_skipped_as_rust_only() {
        let out = transpile_str(
            r#"
            macro_rules! for_both { ($x:expr) => { $x }; }
            mod inner {
                use super::for_both;
            }
        "#,
        );
        assert!(out.contains("// Rust-only macro import: using for_both;"));
        assert!(!out.contains("using ::for_both;"));
    }

    // ── Phase 15 Gap 5: Nested functions → lambdas ──────────────

    #[test]
    fn test_nested_fn_to_lambda() {
        let out = transpile_str("fn outer() { fn inner(x: i32) -> i32 { x + 1 } inner(1); }");
        assert!(out.contains("const auto inner = [&](int32_t x) -> int32_t {"));
        assert!(out.contains("return x + 1;"));
        assert!(out.contains("inner(1);"));
    }

    #[test]
    fn test_nested_fn_void() {
        let out = transpile_str("fn outer() { fn helper() { do_it(); } helper(); }");
        assert!(out.contains("const auto helper = [&]() {"));
        assert!(!out.contains("-> void"));
    }

    #[test]
    fn test_nested_fn_multiple() {
        let out = transpile_str("fn outer() { fn a() {} fn b() {} a(); b(); }");
        assert!(out.contains("const auto a = [&]()"));
        assert!(out.contains("const auto b = [&]()"));
    }

    #[test]
    fn test_nested_fn_not_at_toplevel() {
        // Top-level functions should NOT be lambdas
        let out = transpile_str("fn top_level() {}");
        assert!(out.contains("void top_level()"));
        assert!(!out.contains("const auto top_level"));
    }

    // ── Phase 15 Gap 6: Range syntax variants ───────────────────

    #[test]
    fn test_range_closed() {
        let out = transpile_str("fn f() { for i in 0..10 { i; } }");
        assert!(out.contains("rusty::range(0, 10)"));
    }

    #[test]
    fn test_range_inclusive() {
        let out = transpile_str("fn f() { for i in 0..=10 { i; } }");
        assert!(out.contains("rusty::range_inclusive(0, 10)"));
    }

    #[test]
    fn test_range_from() {
        let out = transpile_str("fn f() { let r = 5..; }");
        assert!(out.contains("rusty::range_from(5)"));
    }

    #[test]
    fn test_range_to() {
        let out = transpile_str("fn f() { let r = ..10; }");
        assert!(out.contains("rusty::range_to(10)"));
    }

    #[test]
    fn test_range_full() {
        let out = transpile_str("fn f() { let r = ..; }");
        assert!(out.contains("rusty::range_full()"));
    }

    #[test]
    fn test_range_to_inclusive() {
        let out = transpile_str("fn f() { let r = ..=10; }");
        assert!(out.contains("rusty::range_to_inclusive(10)"));
    }

    #[test]
    fn test_leaf4292_full_slice_index_lowers_to_slice_helper() {
        let out = transpile_str("fn f() { let v = [0u8; 8]; let s = &v[..]; }");
        assert!(out.contains("const auto& s = rusty::slice_full(v);"));
        assert!(!out.contains("v[rusty::range_full()]"));
        assert!(!out.contains("= &rusty::slice_full(v)"));
    }

    #[test]
    fn test_leaf4292_range_to_slice_index_lowers_to_slice_helper() {
        let out = transpile_str("fn f(n: usize) { let v = [0u8; 8]; let s = &v[..n]; }");
        assert!(out.contains("const auto& s = rusty::slice_to(v, n);"));
        assert!(!out.contains("v[rusty::range_to(n)]"));
        assert!(!out.contains("= &rusty::slice_to(v, n)"));
    }

    #[test]
    fn test_leaf4293_len_method_call_lowers_to_rusty_len_helper() {
        let out = transpile_str("fn f() { let buf = [0u8; 16]; let n = buf.len(); }");
        assert!(out.contains("const auto n = rusty::len(buf);"));
        assert!(!out.contains("buf.len()"));
    }

    #[test]
    fn test_collect_on_range_expression() {
        let out = transpile_str("fn f() { let v = (0..10).collect(); }");
        assert!(out.contains("rusty::collect_range("));
        assert!(out.contains("rusty::range(0, 10)"));
    }

    #[test]
    fn test_collect_on_inclusive_range_expression() {
        let out = transpile_str("fn f() { let v = (1..=3).collect(); }");
        assert!(out.contains("rusty::collect_range("));
        assert!(out.contains("rusty::range_inclusive(1, 3)"));
    }

    #[test]
    fn test_collect_on_non_range_expression_unchanged() {
        let out = transpile_str("fn f() { it.collect(); }");
        assert!(out.contains("it.collect()"));
        assert!(!out.contains("rusty::collect_range(it)"));
    }

    // ── Phase 15 Gap 7: Array repeat + byte string ──────────────

    #[test]
    fn test_array_repeat() {
        let out = transpile_str("fn f() { let a = [0u8; 256]; }");
        assert!(out.contains("rusty::array_repeat("));
        assert!(out.contains("256"));
    }

    #[test]
    fn test_leaf446_repeat_array_infers_u8_from_index_cast_assignment() {
        let out = transpile_str(
            r#"
            fn f() {
                let mut mockdata = [0; 16];
                for i in 0..16 {
                    mockdata[i] = i as u8;
                }
            }
            "#,
        );
        assert!(out.contains("auto mockdata = rusty::array_repeat(static_cast<uint8_t>(0), 16);"));
    }

    #[test]
    fn test_leaf446_repeat_array_without_index_cast_keeps_default_literal_seed() {
        let out = transpile_str(
            r#"
            fn f() {
                let a = [0; 4];
                let _x = a[0];
            }
            "#,
        );
        assert!(out.contains("const auto a = rusty::array_repeat(0, 4);"));
    }

    #[test]
    fn test_byte_string_literal() {
        let out = transpile_str(r#"fn f() { let b = b"hello"; }"#);
        assert!(out.contains("std::array<uint8_t,"));
        assert!(out.contains("0x68")); // 'h'
    }

    // ── Phase 15 Gap 8: Self in trait signatures ────────────────

    #[test]
    fn test_self_in_trait_return_auto() {
        let out = transpile_str("trait Builder { fn build(&self) -> Self; }");
        // Self in trait context → auto
        assert!(out.contains("auto() const"));
    }

    #[test]
    fn test_self_in_struct_resolved() {
        let out = transpile_str(
            r#"
            struct Foo {}
            impl Foo { fn new() -> Self { Foo {} } }
        "#,
        );
        // Self in struct context → struct name
        assert!(out.contains("static Foo new_()"));
    }

    // ── Phase 16: Compilable test fixes ─────────────────────────

    #[test]
    fn test_rust_only_imports_skipped() {
        let out =
            transpile_str("use std::convert::AsRef;\nuse std::ops::Deref;\nuse std::fmt::Display;");
        assert!(out.contains("// Rust-only: using std::convert::AsRef;"));
        assert!(out.contains("// Rust-only: using std::ops::Deref;"));
        assert!(out.contains("// Rust-only: using std::fmt::Display;"));
    }

    #[test]
    fn test_non_rust_only_imports_kept() {
        let out = transpile_str("use std::collections::HashMap;");
        assert!(out.contains("using std::collections::HashMap;"));
        assert!(!out.contains("Rust-only"));
    }

    #[test]
    fn test_std_io_module_import_emits_namespace_alias() {
        let out = transpile_str("use std::io;");
        assert!(out.contains("namespace io = rusty::io;"));
        assert!(!out.contains("using std::io;"));
    }

    #[test]
    fn test_std_io_type_import_remapped_to_rusty_io() {
        let out = transpile_str("use std::io::SeekFrom;");
        assert!(out.contains("using rusty::io::SeekFrom;"));
        assert!(!out.contains("using std::io::SeekFrom;"));
    }

    #[test]
    fn test_std_io_function_import_remapped_to_underscore_variant() {
        let out = transpile_str("use std::io::stdin;");
        assert!(out.contains("using rusty::io::stdin_;"));
        assert!(!out.contains("using std::io::stdin;"));
    }

    #[test]
    fn test_std_io_trait_imports_skipped() {
        let out = transpile_str("use std::io::Read;");
        assert!(out.contains("// Rust-only: using std::io::Read;"));
        assert!(!out.contains("\nusing std::io::Read;"));
    }

    #[test]
    fn test_variant_constructor_generated() {
        let out = transpile_str("enum Maybe<T> { Just(T), Nothing }");
        assert!(out.contains("Maybe_Just<T> Just(T _0)"));
        assert!(out.contains("Maybe_Nothing<T> Nothing()"));
    }

    #[test]
    fn test_variant_constructor_two_params() {
        let out = transpile_str("enum Either<L, R> { Left(L), Right(R) }");
        assert!(out.contains("Either_Left<L, R> Left(L _0)"));
        assert!(out.contains("Either_Right<L, R> Right(R _0)"));
    }

    #[test]
    fn test_variant_constructor_non_generic() {
        let out = transpile_str("enum Token { Num(i32), Str(String) }");
        assert!(out.contains("Token_Num Num(int32_t _0)"));
        assert!(out.contains("Token_Str Str(rusty::String _0)"));
    }

    #[test]
    fn test_enum_with_impl_gets_struct_wrapper() {
        let out = transpile_str(
            r#"
            enum Opt<T> { Some(T), None }
            impl<T> Opt<T> { fn is_some(&self) -> bool { true } }
        "#,
        );
        // Should use struct wrapper (not using alias) since it has impl
        assert!(out.contains("struct Opt : std::variant<"));
        assert!(out.contains("using variant = std::variant<"));
        assert!(out.contains("using variant::variant;"));
        assert!(out.contains("bool is_some() const"));
    }

    #[test]
    fn test_match_as_expression() {
        let out = transpile_str("fn f(x: i32) { let val = match x { 1 => 10, _ => 0 }; }");
        assert!(out.contains("[&]()"));
        assert!(out.contains("if (_m == 1) return 10;"));
        assert!(out.contains("return 0;"));
    }

    #[test]
    fn test_leaf46_tail_match_expr_returns_from_function() {
        let out = transpile_str("fn f(x: i32) -> i32 { match x { 1 => 10, _ => 0 } }");
        assert!(out.contains("return [&]() { auto&& _m = x;"));
        assert!(out.contains("if (_m == 1) return 10;"));
        assert!(out.contains("return 0;"));
        assert!(!out.contains("switch (x) {"));
    }

    #[test]
    fn test_leaf46_tuple_match_expr_lowers_to_multi_visit_args() {
        let out = transpile_str(
            r#"
            enum E { A(i32), B(i32) }
            fn eq_like(a: E, b: E) -> bool {
                match (a, b) {
                    (E::A(x), E::A(y)) => x == y,
                    _ => false,
                }
            }
        "#,
        );
        assert!(out.contains("return std::visit(overloaded {"));
        assert!(out.contains("}, a, b);"));
        assert!(out.contains("auto&& x = _v0._0;"));
        assert!(out.contains("auto&& y = _v1._0;"));
        assert!(!out.contains("/* TODO: expr */"));
    }

    #[test]
    fn test_leaf46_visit_lambdas_capture_outer_locals() {
        let out = transpile_str(
            r#"
            enum E { A(i32), B(i32) }
            fn f(e: E, n: i32) -> i32 {
                match e {
                    E::A(x) => x + n,
                    E::B(y) => y + n,
                }
            }
        "#,
        );
        assert!(out.contains("[&](const E_A& _v)"));
        assert!(out.contains("[&](const E_B& _v)"));
        assert!(!out.contains("[](const E_A& _v)"));
    }

    #[test]
    fn test_leaf46_block_expr_arm_no_todo_placeholder() {
        let out = transpile_str(
            r#"
            enum E { A(i32), B(i32) }
            fn f(e: E) -> i32 {
                match e {
                    E::A(x) => { let y = x + 1; y },
                    E::B(y) => y,
                }
            }
        "#,
        );
        assert!(!out.contains("/* TODO: expr */"));
        assert!(out.contains("const auto y = x + 1;"));
    }

    #[test]
    fn test_leaf48_generic_enum_match_on_self_uses_variant_template_args() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            impl<L, R> Either<L, R> {
                fn flip(self) -> Either<R, L> {
                    match self {
                        Either::Left(l) => Right(l),
                        Either::Right(r) => Left(r),
                    }
                }
            }
        "#,
        );
        assert!(out.contains("[&](const Either_Left<L, R>& _v)"));
        assert!(out.contains("[&](const Either_Right<L, R>& _v)"));
        assert!(!out.contains("[&](const Either_Left& _v)"));
        assert!(!out.contains("[&](const Either_Right& _v)"));
    }

    #[test]
    fn test_leaf48_typed_param_match_uses_concrete_variant_template_args() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn sum(e: Either<i32, i32>) -> i32 {
                match e {
                    Either::Left(x) => x,
                    Either::Right(y) => y,
                }
            }
        "#,
        );
        assert!(out.contains("[&](const Either_Left<int32_t, int32_t>& _v)"));
        assert!(out.contains("[&](const Either_Right<int32_t, int32_t>& _v)"));
    }

    #[test]
    fn test_leaf49_generic_match_arm_constructor_calls_use_return_expected_type() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn flip<L, R>(e: Either<L, R>) -> Either<R, L> {
                match e {
                    Either::Left(l) => Right(l),
                    Either::Right(r) => Left(r),
                }
            }
        "#,
        );
        assert!(out.contains("return Either<R, L>(Right<R, L>(std::move(l)));"));
        assert!(out.contains("return Either<R, L>(Left<R, L>(std::move(r)));"));
    }

    #[test]
    fn test_leaf49_self_return_match_constructor_calls_use_in_scope_type_params() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            impl<L, R> Either<L, R> {
                fn from_result(r: Result<R, L>) -> Self {
                    match r {
                        Err(e) => Left(e),
                        Ok(o) => Right(o),
                    }
                }
            }
        "#,
        );
        assert!(out.contains("return Either(Left<L, R>(std::move(e)));"));
        assert!(out.contains("return Either(Right<L, R>(std::move(o)));"));
    }

    #[test]
    fn test_leaf416_self_variant_patterns_emit_resolved_enum_variant_types() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            impl<L: Clone, R: Clone> Either<L, R> {
                fn cloned(self) -> Either<L, R> {
                    match self {
                        Self::Left(l) => Either::Left(l.clone()),
                        Self::Right(r) => Either::Right(r.clone()),
                    }
                }
            }
        "#,
        );
        assert!(out.contains("[&](const Either_Left<L, R>& _v)"));
        assert!(out.contains("[&](const Either_Right<L, R>& _v)"));
        assert!(!out.contains("Self_Left"));
        assert!(!out.contains("Self_Right"));
    }

    #[test]
    fn test_leaf416_result_match_expression_uses_runtime_conditionals() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            impl<L, R> Either<L, R> {
                fn from_result(r: Result<R, L>) -> Self {
                    match r {
                        Err(e) => Left(e),
                        Ok(o) => Right(o),
                    }
                }
            }
        "#,
        );
        assert!(out.contains("if (_m.is_err()) {"));
        assert!(out.contains("if (_m.is_ok()) {"));
        assert!(!out.contains("Result_Err"));
        assert!(!out.contains("Result_Ok"));
        assert!(!out.contains("std::visit(overloaded {"));
    }

    #[test]
    fn test_leaf416_result_constructors_use_expected_result_specialization() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            impl<L, R> Either<L, R> {
                fn into_result(self) -> Result<R, L> {
                    match self {
                        Either::Left(l) => Err(l),
                        Either::Right(r) => Ok(r),
                    }
                }
            }
        "#,
        );
        assert!(out.contains("rusty::Result<R, L>::Err(l)"));
        assert!(out.contains("rusty::Result<R, L>::Ok(r)"));
        assert!(!out.contains("rusty::Result::err"));
        assert!(!out.contains("rusty::Result::ok"));
    }

    #[test]
    fn test_leaf415_nested_tuple_variant_pattern_emits_bindings() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            impl<T, L, R> Either<(T, L), (T, R)> {
                fn factor_first(self) -> (T, Either<L, R>) {
                    match self {
                        Left((t, l)) => (t, Left(l)),
                        Right((t, r)) => (t, Right(r)),
                    }
                }
            }
            impl<T, L, R> Either<(L, T), (R, T)> {
                fn factor_second(self) -> (Either<L, R>, T) {
                    match self {
                        Left((l, t)) => (Left(l), t),
                        Right((r, t)) => (Right(r), t),
                    }
                }
            }
        "#,
        );

        assert!(out.contains("auto&& t = std::get<0>(_v._0);"));
        assert!(out.contains("auto&& l = std::get<1>(_v._0);"));
        assert!(out.contains("auto&& r = std::get<1>(_v._0);"));
        assert!(out.contains("auto&& l = std::get<0>(_v._0);"));
        assert!(out.contains("auto&& t = std::get<1>(_v._0);"));
        assert!(out.contains("auto&& r = std::get<0>(_v._0);"));
    }

    #[test]
    fn test_leaf410_iter_either_new_call_uses_expected_return_specialization() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            mod iterator {
                struct IterEither<L, R> { inner: Either<L, R> }
                impl<L, R> IterEither<L, R> {
                    fn new_(inner: Either<L, R>) -> Self { IterEither { inner } }
                }
            }
            impl<L, R> Either<L, R> {
                fn factor(self) -> iterator::IterEither<L, R> {
                    iterator::IterEither::new_(self)
                }
            }
        "#,
        );
        assert!(out.contains("iterator::IterEither<L, R>::new_("));
        assert!(!out.contains("iterator::IterEither::new_("));
    }

    #[test]
    fn test_leaf410_core_cmp_fallback_helpers_emitted_for_core_cmp_calls() {
        let out = transpile_str(
            r#"
            fn f<T>(a: T, b: T) {
                core::cmp::Ord::cmp(a, b);
                core::cmp::PartialOrd::partial_cmp(a, b);
            }
        "#,
        );
        assert!(out.contains("core::cmp::Ord::cmp("));
        assert!(out.contains("core::cmp::PartialOrd::partial_cmp("));
        assert!(out.contains("namespace core {"));
        assert!(out.contains("struct PartialOrd {"));
        assert!(out.contains("struct Ord {"));
    }

    #[test]
    fn test_leaf410_ordering_match_does_not_emit_flattened_placeholder_name() {
        let out = transpile_str(
            r#"
            fn is_eq(o: rusty::cmp::Ordering) -> bool {
                match o {
                    rusty::cmp::Ordering::Equal => true,
                    _ => false,
                }
            }
        "#,
        );
        assert!(out.contains("rusty::cmp::Ordering::Equal"));
        assert!(!out.contains("rusty_cmp_Ordering_Equal"));
    }

    #[test]
    fn test_leaf410_specialized_impl_method_keeps_impl_generic_placeholders() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            impl<L, R, E> Either<Result<L, E>, Result<R, E>> {
                fn factor_err(self) -> Result<Either<L, R>, E> {
                    match self {
                        Left(l) => l.map(Left),
                        Right(r) => r.map(Right),
                    }
                }
            }
        "#,
        );
        assert!(out.contains("template<typename E>"));
        assert!(out.contains("rusty::Result<Either<L, R>, E> factor_err()"));
    }

    // ── Phase 17 Fix 3: UFCS and expanded macro patterns ────────

    #[test]
    fn test_ufcs_associated_type() {
        // <T as Iterator>::Item → T::Item
        let out = transpile_str("fn f<T>(x: <T as Iterator>::Item) {}");
        assert!(out.contains("T::Item"));
        assert!(!out.contains("<T as"));
    }

    // ── Phase 18 Blocker 1: typed-let context propagation ──────

    #[test]
    fn test_typed_let_variant_constructor_left_gets_template_args() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn f() { let e: Either<i32, i32> = Left(2); }
        "#,
        );
        assert!(out.contains("const Either<int32_t, int32_t> e = Left<int32_t, int32_t>(2);"));
    }

    #[test]
    fn test_typed_let_variant_constructor_right_gets_template_args() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn f() { let e: Either<i32, i32> = Right(3); }
        "#,
        );
        assert!(out.contains("const Either<int32_t, int32_t> e = Right<int32_t, int32_t>(3);"));
    }

    #[test]
    fn test_leaf4172_untyped_local_variant_constructor_recovers_expected_type() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn f() {
                let e = Left(2);
                let r = Right(2);
            }
        "#,
        );
        assert!(out.contains("const auto e = Left<int32_t, int32_t>(2);"));
        assert!(out.contains("const auto r = Right<int32_t, int32_t>(2);"));
    }

    #[test]
    fn test_typed_let_option_some_still_uses_make_optional() {
        let out = transpile_str("fn f() { let o: Option<i32> = Some(2); }");
        assert!(out.contains("const rusty::Option<int32_t> o = std::make_optional(2);"));
        assert!(!out.contains("Some<int32_t>"));
    }

    #[test]
    fn test_typed_assignment_variant_constructor_gets_template_args() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn f() {
                let mut e: Either<i32, i32> = Left(1);
                e = Right(2);
            }
        "#,
        );
        assert!(out.contains("e = Right<int32_t, int32_t>(2);"));
    }

    #[test]
    fn test_leaf420_mut_reassigned_untyped_variant_local_uses_sum_type() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn f() {
                let mut e = Left(1);
                let r = Right(2);
                e = r;
            }
        "#,
        );
        assert!(out.contains("Either<int32_t, int32_t> e = Left<int32_t, int32_t>(1);"));
        assert!(out.contains("e = r;"));
    }

    #[test]
    fn test_untyped_shadow_blocks_outer_assignment_type_context() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn f() {
                let mut e: Either<i32, i32> = Left(1);
                {
                    let mut e = Left(3);
                    e = Right(4);
                }
                e = Right(2);
            }
        "#,
        );
        assert!(out.contains("e = Right<int32_t, int32_t>(4);"));
        assert!(out.contains("e = Right<int32_t, int32_t>(2);"));
    }

    #[test]
    fn test_leaf4172_tuple_assertion_context_uses_local_binding_type() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn f() {
                let e = Left(2);
                let t = (&e, &Right(2), &Left(3));
            }
        "#,
        );
        assert!(out.contains("&Right<int32_t, int32_t>(2)"));
        assert!(out.contains("&Left<int32_t, int32_t>(3)"));
    }

    #[test]
    fn test_leaf4172_tuple_context_from_local_callable_return_type() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn f() {
                let a = || -> Either<i32, i32> { Right(1) };
                let t = (&a(), &Right(2));
            }
        "#,
        );
        assert!(out.contains("const auto t = std::make_tuple(&a(), &Right<int32_t, int32_t>(2));"));
    }

    #[test]
    fn test_leaf4172_tuple_context_from_nested_fn_return_type() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn f() {
                fn b() -> Either<String, &'static str> { Right("foo") }
                let t = (&b(), &Left(String::from("foo")));
            }
        "#,
        );
        assert!(out.contains(
            "const auto t = std::make_tuple(&b(), &Left<rusty::String, std::string_view>(rusty::String::from(\"foo\")));"
        ));
    }

    #[test]
    fn test_leaf420_binding_tuple_match_statement_avoids_visit_and_rvalue_address_of() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn f() {
                let mut e = Left(2);
                let r = Right(2);
                match (&e, &Left(2)) {
                    (left_val, right_val) => {
                        let _ = *left_val == *right_val;
                    }
                };
                e = r;
                match (&e.left(), &None) {
                    (left_val, right_val) => {
                        let _ = *left_val == *right_val;
                    }
                };
            }
        "#,
        );
        assert!(!out.contains("std::visit(overloaded {"));
        assert!(out.contains("auto _m1_tmp = Either<int32_t, int32_t>(Left<int32_t, int32_t>(2));"));
        assert!(out.contains("auto _m1_tmp = std::nullopt;"));
        assert!(out.contains("auto&& left_val = std::get<0>(_m_tuple);"));
        assert!(out.contains("auto&& right_val = std::get<1>(_m_tuple);"));
    }

    #[test]
    fn test_leaf425_binding_tuple_match_wraps_variant_constructor_to_expected_enum() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn f() {
                let b = || -> Either<String, &'static str> { Right("foo") };
                match (&b(), &Left(String::from("foo"))) {
                    (left_val, right_val) => {
                        let _ = *left_val == *right_val;
                    }
                };
            }
        "#,
        );
        assert!(out.contains(
            "auto _m1_tmp = Either<rusty::String, std::string_view>(Left<rusty::String, std::string_view>(rusty::String::from(\"foo\")));"
        ));
        assert!(!out.contains(
            "auto _m1_tmp = Left<rusty::String, std::string_view>(rusty::String::from(\"foo\"));"
        ));
    }

    #[test]
    fn test_leaf426_deref_trait_match_uses_reference_aware_deref_lowering() {
        let out = transpile_str_module(
            r#"
            use std::ops::Deref;
            enum Either<L, R> { Left(L), Right(R) }
            impl<L: Deref, R: Deref<Target = L::Target>> Deref for Either<L, R> {
                type Target = L::Target;
                fn deref(&self) -> &Self::Target {
                    match *self {
                        Either::Left(ref inner) => &**inner,
                        Either::Right(ref inner) => &**inner,
                    }
                }
            }
        "#,
            "either",
        );
        assert!(out.contains("auto operator*() const {"));
        assert!(out.contains("auto&& _m = (*this);"));
        assert!(!out.contains("auto _m = *(*this);"));
        assert!(out.contains("return rusty::deref_ref("));
        assert!(!out.contains("&**inner"));
    }

    #[test]
    fn test_leaf4294_tuple_match_slice_assertion_materializes_slice_temps() {
        let out = transpile_str(
            r#"
            fn f() {
                let mut mock = [0u8; 32];
                let buf = [0u8; 8];
                match (&buf, &mock[..buf.len()]) {
                    (left_val, right_val) => {
                        let _ = *left_val == *right_val;
                    }
                };
            }
        "#,
        );
        assert!(out.contains("auto _m0_tmp = rusty::slice_full(buf);"));
        assert!(out.contains("auto _m1_tmp = rusty::slice_to(mock, rusty::len(buf));"));
        assert!(!out.contains("auto _m1 = &rusty::slice_to(mock, rusty::len(buf));"));
    }

    #[test]
    fn test_leaf4294_tuple_match_nested_reference_avoids_double_address_artifact() {
        let out = transpile_str(
            r#"
            fn f() {
                let mut mock = [0u8; 32];
                let buf = [0u8; 8];
                match (&&buf, &&mock[..buf.len()]) {
                    (left_val, right_val) => {
                        let _ = *left_val == *right_val;
                    }
                };
            }
        "#,
        );
        assert!(!out.contains("auto _m0 = &&buf;"));
        assert!(out.contains("auto _m0_tmp = rusty::slice_full(buf);"));
        assert!(out.contains("auto _m1_tmp = rusty::slice_to(mock, rusty::len(buf));"));
        assert!(!out.contains("auto _m1 = &rusty::slice_to(mock, rusty::len(buf));"));
    }

    #[test]
    fn test_leaf426_reborrow_of_deref_typed_non_pointer_drops_address_of() {
        let out = transpile_str(
            r#"
            use std::ops::Deref;
            enum Either<L, R> { Left(L), Right(R) }
            fn f(value: Either<String, &'static str>) {
                let is_str = |_: &str| {};
                is_str(&*value);
            }
        "#,
        );
        assert!(out.contains("is_str(*value);"));
        assert!(!out.contains("is_str(&*value);"));
    }

    #[test]
    fn test_leaf421_module_mode_dependent_assoc_signatures_are_softened() {
        let out = transpile_str_module(
            r#"
            trait FutureLike { type Output; fn poll(self) -> Self::Output; }
            trait DerefLike { type Target; fn deref(self) -> Self::Target; }
            enum Either<L, R> { Left(L), Right(R) }
            impl<L, R> Either<L, R> {
                fn iter(self) -> Either<L::IntoIter, R::IntoIter> { todo!() }
            }
            impl<L: FutureLike, R: FutureLike<Output = L::Output>> FutureLike for Either<L, R> {
                type Output = L::Output;
                fn poll(self) -> Self::Output { todo!() }
            }
            impl<L: DerefLike, R: DerefLike<Target = L::Target>> DerefLike for Either<L, R> {
                type Target = L::Target;
                fn deref(self) -> Self::Target { todo!() }
            }
        "#,
            "either",
        );
        assert!(out.contains("auto iter("));
        assert!(!out.contains("Either<typename L::IntoIter, typename R::IntoIter> iter("));
        assert!(out.contains("auto poll("));
        assert!(out.contains("auto deref("));
        assert!(!out.contains("using Output = typename L::Output;"));
        assert!(!out.contains("using Target = typename L::Target;"));
    }

    #[test]
    fn test_leaf432_expanded_mode_dependent_assoc_signatures_are_softened() {
        let out = transpile_str(
            r#"
            #[rustc_test_marker = "basic"]
            const BASIC: test::TestDescAndFn = unsafe { std::mem::zeroed() };
            trait IntoIterLike { type IntoIter; fn into_iter(self) -> Self::IntoIter; }
            enum Either<L, R> { Left(L), Right(R) }
            impl<L, R> Either<L, R> {
                fn iter(self) -> Either<L::IntoIter, R::IntoIter> { todo!() }
            }
            impl<L: IntoIterLike, R: IntoIterLike<IntoIter = L::IntoIter>> IntoIterLike for Either<L, R> {
                type IntoIter = L::IntoIter;
                fn into_iter(self) -> Either::IntoIter { todo!() }
            }
        "#,
        );
        assert!(out.contains("auto iter("));
        assert!(!out.contains("Either<typename L::IntoIter, typename R::IntoIter> iter("));
        assert!(out.contains("auto into_iter("));
        assert!(!out.contains("Either::IntoIter into_iter("));
        assert!(!out.contains("using IntoIter = typename L::IntoIter;"));
    }

    #[test]
    fn test_leaf432_expanded_mode_softens_current_struct_assoc_projection_returns() {
        let out = transpile_str(
            r#"
            #[rustc_test_marker = "basic"]
            const BASIC: test::TestDescAndFn = unsafe { std::mem::zeroed() };
            trait IteratorLike { type Item; fn next(self) -> Self::Item; }
            enum Either<L, R> { Left(L), Right(R) }
            impl<L: IteratorLike, R: IteratorLike<Item = L::Item>> IteratorLike for Either<L, R> {
                type Item = L::Item;
                fn next(self) -> rusty::Option<Either::Item> { todo!() }
            }
        "#,
        );
        assert!(out.contains("auto next("));
        assert!(!out.contains("rusty::Option<Either::Item> next("));
        assert!(!out.contains("using Item = typename L::Item;"));
    }

    #[test]
    fn test_leaf434_expanded_mode_option_none_avoids_assoc_ctor_type_in_value_position() {
        let out = transpile_str(
            r#"
            #[rustc_test_marker = "basic"]
            const BASIC: test::TestDescAndFn = unsafe { std::mem::zeroed() };
            trait IteratorLike { type Item; fn next(self) -> Option<Self::Item>; }
            enum Either<L, R> { Left(L), Right(R) }
            impl<L: IteratorLike, R: IteratorLike<Item = L::Item>> IteratorLike for Either<L, R> {
                type Item = L::Item;
                fn next(self) -> Option<Self::Item> { None }
            }
        "#,
        );
        assert!(out.contains("return std::nullopt;"));
        assert!(!out.contains("rusty::Option<Either::Item>(rusty::None)"));
    }

    #[test]
    fn test_leaf434_expanded_mode_option_some_avoids_assoc_ctor_type_in_value_position() {
        let out = transpile_str(
            r#"
            #[rustc_test_marker = "basic"]
            const BASIC: test::TestDescAndFn = unsafe { std::mem::zeroed() };
            trait IteratorLike { type Item; fn next(self) -> Option<Self::Item>; }
            enum Either<L, R> { Left(L), Right(R) }
            impl<L: IteratorLike, R: IteratorLike<Item = L::Item>> IteratorLike for Either<L, R> {
                type Item = L::Item;
                fn next(self) -> Option<Self::Item> { Some(todo!()) }
            }
        "#,
        );
        assert!(out.contains("return std::make_optional("));
        assert!(!out.contains("rusty::Option<Either::Item>("));
    }

    #[test]
    fn test_leaf433_same_scope_shadowing_local_is_renamed() {
        let out = transpile_str(
            r#"
            fn f() {
                let mut buf = [0u8; 4];
                let _x = buf.len();
                let buf = [1u8; 4];
                let _y = buf.len();
            }
        "#,
        );
        assert!(out.contains("auto buf = rusty::array_repeat(static_cast<uint8_t>(0), 4);"));
        assert!(out
            .contains("const auto buf_shadow1 = rusty::array_repeat(static_cast<uint8_t>(1), 4);"));
        assert!(out.contains("const auto _y = rusty::len(buf_shadow1);"));
    }

    #[test]
    fn test_leaf433_if_let_expr_lowers_without_unreachable_condition() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn f() {
                let invalid_utf8 = b"\xff";
                let _res = if let Err(error) = std::str::from_utf8(invalid_utf8) {
                    Err(Left(error))
                } else if let Err(error) = "x".parse::<i32>() {
                    Err(Right(error))
                } else {
                    Ok(())
                };
            }
        "#,
        );
        assert!(!out.contains("rusty::intrinsics::unreachable() ?"));
        assert!(out.contains("auto&& _iflet = rusty::str_runtime::from_utf8("));
        assert!(out.contains("rusty::str_runtime::parse<int32_t>(\"x\")"));
        assert!(!out.contains("std::str::from_utf8("));
        assert!(!out.contains("\"x\".parse()"));
        assert!(out.contains("_iflet.is_err() ?"));
        assert!(out.contains("auto error = _iflet.unwrap_err();"));
    }

    #[test]
    fn test_leaf435_constructor_hint_recovery_uses_iflet_unwrap_type_placeholder() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn f() {
                let invalid_utf8 = b"\xff";
                let _res = if let Err(error) = std::str::from_utf8(invalid_utf8) {
                    Err(Left(error))
                } else if let Err(error) = "x".parse::<i32>() {
                    Err(Right(error))
                } else {
                    Ok(())
                };
            }
        "#,
        );
        assert!(!out.contains("decltype((std::move(error)))"));
        assert!(out.contains("decltype((_iflet.unwrap_err()))"));
    }

    #[test]
    fn test_leaf436_consuming_method_receiver_binding_is_not_const() {
        let out = transpile_str(
            r#"
            fn f() {
                let res = Err::<(), i32>(1);
                res.unwrap_err().to_string();
            }
        "#,
        );
        assert!(out.contains("auto res = "));
        assert!(!out.contains("const auto res = "));
        assert!(out.contains("res.unwrap_err().to_string();"));
    }

    #[test]
    fn test_leaf436_non_consuming_receiver_binding_stays_const() {
        let out = transpile_str(
            r#"
            fn f() {
                let res = Err::<(), i32>(1);
                let _is_err = res.is_err();
            }
        "#,
        );
        assert!(out.contains("const auto res = "));
        assert!(out.contains("const auto _is_err = res.is_err();"));
    }

    #[test]
    fn test_leaf437_as_ref_as_mut_reference_constructor_args_are_not_moved() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            impl<L, R> Either<L, R> {
                fn as_ref(&self) -> Either<&L, &R> {
                    match *self {
                        Either::Left(ref inner) => Left(inner),
                        Either::Right(ref inner) => Right(inner),
                    }
                }
                fn as_mut(&mut self) -> Either<&mut L, &mut R> {
                    match *self {
                        Either::Left(ref mut inner) => Left(inner),
                        Either::Right(ref mut inner) => Right(inner),
                    }
                }
            }
        "#,
        );
        assert!(out.contains("return Either<const L&, const R&>(Left<const L&, const R&>(inner));"));
        assert!(
            out.contains("return Either<const L&, const R&>(Right<const L&, const R&>(inner));")
        );
        assert!(out.contains("return Either<L&, R&>(Left<L&, R&>(inner));"));
        assert!(out.contains("return Either<L&, R&>(Right<L&, R&>(inner));"));
        assert!(!out.contains("Left<const L&, const R&>(std::move(inner))"));
        assert!(!out.contains("Right<const L&, const R&>(std::move(inner))"));
        assert!(!out.contains("Left<L&, R&>(std::move(inner))"));
        assert!(!out.contains("Right<L&, R&>(std::move(inner))"));
    }

    #[test]
    fn test_leaf442_variant_constructor_helpers_use_forward_for_reference_instantiations() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
        "#,
        );
        assert!(out.contains("Either_Left<L, R> Left(L _0)"));
        assert!(out.contains("Either_Right<L, R> Right(R _0)"));
        assert!(out.contains("return Either_Left<L, R>{std::forward<L>(_0)};"));
        assert!(out.contains("return Either_Right<L, R>{std::forward<R>(_0)};"));
        assert!(!out.contains("return Either_Left<L, R>{std::move(_0)};"));
        assert!(!out.contains("return Either_Right<L, R>{std::move(_0)};"));
    }

    #[test]
    fn test_leaf437_option_match_binding_uses_forwarding_ref_for_by_value_pat() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            impl<L, R> Either<L, R> {
                fn right(self) -> Option<R> {
                    match self {
                        Either::Left(_) => None,
                        Either::Right(r) => Some(r),
                    }
                }
            }
        "#,
        );
        assert!(out.contains("auto&& r = _v._0;"));
        assert!(!out.contains("const auto& r = _v._0;"));
        assert!(out.contains("return rusty::Option<R>(r);"));
    }

    #[test]
    fn test_leaf438_match_panic_fmt_arm_in_nonvoid_context_is_typed() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            impl<L, R> Either<L, R> {
                fn unwrap_right(self) -> R {
                    match self {
                        Either::Right(r) => r,
                        Either::Left(l) => { core::panicking::panic_fmt(); },
                    }
                }
            }
        "#,
        );
        assert!(out.contains("return [&]() -> R { rusty::panicking::panic_fmt("));
        assert!(!out.contains("return [&]() { rusty::panicking::panic_fmt("));
    }

    #[test]
    fn test_leaf438_match_unreachable_arm_in_nonvoid_context_is_typed() {
        let out = transpile_str(
            r#"
            enum E { A(i32), B }
            fn f(e: E) -> i32 {
                match e {
                    E::A(x) => x,
                    E::B => { core::intrinsics::unreachable(); },
                }
            }
        "#,
        );
        assert!(out.contains("return [&]() -> int32_t { rusty::intrinsics::unreachable(); }();"));
    }

    #[test]
    fn test_leaf433_generic_option_some_none_use_option_ctor_shape() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            impl<L, R> Either<L, R> {
                fn right(self) -> Option<R> {
                    match self {
                        Either::Left(_) => None,
                        Either::Right(r) => Some(r),
                    }
                }
            }
        "#,
        );
        assert!(out.contains("return rusty::Option<R>(rusty::None);"));
        assert!(out.contains("return rusty::Option<R>(r);"));
        assert!(!out.contains("return std::nullopt;"));
    }

    #[test]
    fn test_leaf433_core_panicking_panic_path_is_mapped() {
        let out = transpile_str("fn f() { core::panicking::panic(\"boom\"); }");
        assert!(out.contains("rusty::panicking::panic(\"boom\")"));
    }

    #[test]
    fn test_leaf433_variant_constructor_helpers_are_predeclared_before_wrapper_methods() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            impl<L, R> Either<L, R> {
                fn as_ref(&self) -> Either<&L, &R> {
                    match *self {
                        Either::Left(ref inner) => Left(inner),
                        Either::Right(ref inner) => Right(inner),
                    }
                }
            }
        "#,
        );
        let decl_pos = out
            .find("Either_Left<L, R> Left(L _0);")
            .expect("expected Left predeclaration");
        let wrapper_pos = out
            .find("struct Either : std::variant<")
            .expect("expected enum wrapper");
        assert!(decl_pos < wrapper_pos);
        assert!(!out.contains("auto Left(L _0)"));
        assert!(!out.contains("auto Right(R _0)"));
    }

    #[test]
    fn test_leaf4172_untyped_match_local_recovers_constructor_pair() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn f(x: i32) {
                let iter = match x {
                    0 => Left(1),
                    _ => Right(2),
                };
            }
        "#,
        );
        assert!(out.contains("return Either<int32_t, int32_t>(Left<int32_t, int32_t>(1));"));
        assert!(out.contains("return Either<int32_t, int32_t>(Right<int32_t, int32_t>(2));"));
    }

    #[test]
    fn test_leaf427_untyped_match_local_with_mixed_constructor_payloads_wraps_expected_enum() {
        let out = transpile_str(
            r#"
            enum Either<L, R> { Left(L), Right(R) }
            fn f(x: i32) {
                let iter = match x {
                    3 => Left(0..10),
                    _ => Right(17..),
                };
            }
        "#,
        );
        assert!(out.contains(
            "if (_m == 3) return Either<rusty::range<int32_t>, rusty::range_from<int32_t>>(Left<rusty::range<int32_t>, rusty::range_from<int32_t>>(rusty::range(0, 10)));"
        ));
        assert!(out.contains(
            "return Either<rusty::range<int32_t>, rusty::range_from<int32_t>>(Right<rusty::range<int32_t>, rusty::range_from<int32_t>>(rusty::range_from(17)));"
        ));
    }

    #[test]
    fn test_leaf428_ref_mut_pattern_binding_emits_mutable_reference() {
        let out = transpile_str_module(
            r#"
            struct Iter;
            impl Iter { fn next(&mut self) -> i32 { 0 } }
            enum Either<L, R> { Left(L), Right(R) }
            impl Either<Iter, Iter> {
                fn f(&mut self) -> i32 {
                    match *self {
                        Either::Left(ref mut inner) => inner.next(),
                        Either::Right(ref mut inner) => inner.next(),
                    }
                }
            }
        "#,
            "either",
        );
        assert!(out.contains("auto& inner = _v._0;"));
        assert!(!out.contains("const auto& inner = _v._0; return inner.next();"));
    }

    // ── Phase 18 Blocker 2: UFCS trait method call detection ────

    #[test]
    fn test_detect_ufcs_trait_call_with_mut_receiver() {
        let expr: syn::Expr = syn::parse_str("io::Read::read(&mut cursor, &mut buf)").unwrap();
        let call = match expr {
            syn::Expr::Call(c) => c,
            _ => panic!("expected call expression"),
        };
        let cg = CodeGen::new();
        let info = cg
            .detect_ufcs_trait_method_call(&call)
            .expect("should detect UFCS call");

        assert_eq!(info.function_path, "io::Read::read");
        assert_eq!(info.method_name, "read");
        assert!(info.receiver_is_mut);
        assert_eq!(info.non_receiver_arg_count, 1);
    }

    #[test]
    fn test_detect_ufcs_trait_call_with_shared_receiver() {
        let expr: syn::Expr = syn::parse_str("Iterator::next(&it)").unwrap();
        let call = match expr {
            syn::Expr::Call(c) => c,
            _ => panic!("expected call expression"),
        };
        let cg = CodeGen::new();
        let info = cg
            .detect_ufcs_trait_method_call(&call)
            .expect("should detect UFCS call");

        assert_eq!(info.function_path, "Iterator::next");
        assert_eq!(info.method_name, "next");
        assert!(!info.receiver_is_mut);
        assert_eq!(info.non_receiver_arg_count, 0);
    }

    #[test]
    fn test_detect_ufcs_trait_call_rejects_non_reference_receiver() {
        let expr: syn::Expr = syn::parse_str("Trait::method(obj, arg)").unwrap();
        let call = match expr {
            syn::Expr::Call(c) => c,
            _ => panic!("expected call expression"),
        };
        let cg = CodeGen::new();
        assert!(cg.detect_ufcs_trait_method_call(&call).is_none());
    }

    #[test]
    fn test_detect_ufcs_trait_call_rejects_plain_function_call() {
        let expr: syn::Expr = syn::parse_str("helper(&x)").unwrap();
        let call = match expr {
            syn::Expr::Call(c) => c,
            _ => panic!("expected call expression"),
        };
        let cg = CodeGen::new();
        assert!(cg.detect_ufcs_trait_method_call(&call).is_none());
    }

    #[test]
    fn test_detect_ufcs_trait_call_rejects_namespaced_free_function() {
        let expr: syn::Expr = syn::parse_str("io::read(&x, y)").unwrap();
        let call = match expr {
            syn::Expr::Call(c) => c,
            _ => panic!("expected call expression"),
        };
        let cg = CodeGen::new();
        assert!(cg.detect_ufcs_trait_method_call(&call).is_none());
    }

    #[test]
    fn test_leaf429_detect_ufcs_trait_call_rejects_constructor_like_new_path() {
        let expr: syn::Expr = syn::parse_str("io::Cursor::new(&data)").unwrap();
        let call = match expr {
            syn::Expr::Call(c) => c,
            _ => panic!("expected call expression"),
        };
        let cg = CodeGen::new();
        assert!(cg.detect_ufcs_trait_method_call(&call).is_none());
    }

    #[test]
    fn test_emit_ufcs_read_call_common_pattern() {
        let expr: syn::Expr = syn::parse_str("io::Read::read(&mut cursor, &mut buf)").unwrap();
        let call = match expr {
            syn::Expr::Call(c) => c,
            _ => panic!("expected call expression"),
        };
        let cg = CodeGen::new();
        let out = cg.emit_call_expr_to_string(&call, None);
        assert_eq!(out, "cursor.read(buf)");
    }

    #[test]
    fn test_emit_ufcs_trait_call_with_self_receiver() {
        let expr: syn::Expr = syn::parse_str("Trait::tick(&self, 1)").unwrap();
        let call = match expr {
            syn::Expr::Call(c) => c,
            _ => panic!("expected call expression"),
        };
        let cg = CodeGen::new();
        let out = cg.emit_call_expr_to_string(&call, None);
        assert_eq!(out, "tick(1)");
    }

    #[test]
    fn test_emit_ufcs_write_call_common_pattern() {
        let expr: syn::Expr = syn::parse_str("io::Write::write(&mut writer, &buf)").unwrap();
        let call = match expr {
            syn::Expr::Call(c) => c,
            _ => panic!("expected call expression"),
        };
        let cg = CodeGen::new();
        let out = cg.emit_call_expr_to_string(&call, None);
        assert_eq!(out, "writer.write(buf)");
    }

    #[test]
    fn test_emit_ufcs_iterator_next_common_pattern() {
        let expr: syn::Expr = syn::parse_str("Iterator::next(&it)").unwrap();
        let call = match expr {
            syn::Expr::Call(c) => c,
            _ => panic!("expected call expression"),
        };
        let cg = CodeGen::new();
        let out = cg.emit_call_expr_to_string(&call, None);
        assert_eq!(out, "it.next()");
    }

    #[test]
    fn test_emit_ufcs_custom_trait_method_common_pattern() {
        let expr: syn::Expr = syn::parse_str("MyTrait::apply(&obj, &value)").unwrap();
        let call = match expr {
            syn::Expr::Call(c) => c,
            _ => panic!("expected call expression"),
        };
        let cg = CodeGen::new();
        let out = cg.emit_call_expr_to_string(&call, None);
        assert_eq!(out, "obj.apply(value)");
    }

    #[test]
    fn test_leaf429_emit_constructor_like_new_path_keeps_function_call_shape() {
        let expr: syn::Expr = syn::parse_str("io::Cursor::new(&data)").unwrap();
        let call = match expr {
            syn::Expr::Call(c) => c,
            _ => panic!("expected call expression"),
        };
        let cg = CodeGen::new();
        let out = cg.emit_call_expr_to_string(&call, None);
        assert!(out.starts_with("rusty::io::cursor_new("));
        assert!(!out.contains(".new("));
    }

    #[test]
    fn test_leaf4295_cursor_new_empty_array_lowers_to_concrete_empty_buffer() {
        let expr: syn::Expr = syn::parse_str("io::Cursor::new([])").unwrap();
        let call = match expr {
            syn::Expr::Call(c) => c,
            _ => panic!("expected call expression"),
        };
        let cg = CodeGen::new();
        let out = cg.emit_call_expr_to_string(&call, None);
        assert_eq!(
            out,
            "rusty::io::cursor_new(rusty::array_repeat(static_cast<uint8_t>(0), 0))"
        );
    }
}
