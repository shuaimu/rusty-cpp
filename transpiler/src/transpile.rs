use crate::codegen::CodeGen;
use crate::types::UserTypeMap;
use quote::ToTokens;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use syn::visit::{self, Visit};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CppModuleSymbolIndex {
    pub modules: BTreeMap<String, CppModuleIndexModule>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CppModuleIndexModule {
    pub namespace: Option<String>,
    pub symbols: BTreeMap<String, CppModuleIndexSymbol>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CppModuleIndexSymbol {
    pub kind: Option<String>,
    pub callable_signatures: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct CppModuleSymbolIndexFile {
    #[serde(default = "default_cpp_module_symbol_index_version")]
    version: u32,
    #[serde(default)]
    modules: BTreeMap<String, CppModuleIndexModuleFile>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct CppModuleIndexModuleFile {
    #[serde(default)]
    namespace: Option<String>,
    #[serde(default)]
    symbols: BTreeMap<String, CppModuleIndexSymbolFile>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct CppModuleIndexSymbolFile {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    callable_signatures: Vec<String>,
}

fn default_cpp_module_symbol_index_version() -> u32 {
    1
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TranspileOptions {
    /// Opt-in diagnostic-only prototype for by-value SCC cycle-breaking planning.
    /// Default is `false`.
    pub by_value_cycle_breaking_prototype: bool,
    /// Optional C++ module symbol index for `use cpp::...` interop resolution.
    pub cpp_module_symbol_index: Option<CppModuleSymbolIndex>,
    /// Source paths used to load the configured C++ module symbol index.
    /// Used in diagnostics so unresolved-symbol errors point to the configured index input.
    pub cpp_module_symbol_index_sources: Vec<PathBuf>,
    /// Maps Rust external crate roots to transpiled C++ module namespaces available
    /// in the current compilation unit (for example `serde_core` -> `serde_core`).
    pub external_crate_module_aliases: HashMap<String, String>,
}

pub fn load_cpp_module_symbol_index_files(
    index_paths: &[PathBuf],
) -> Result<CppModuleSymbolIndex, String> {
    let mut merged = CppModuleSymbolIndex::default();
    for path in index_paths {
        let content = fs::read_to_string(path).map_err(|e| {
            format!(
                "Failed to read C++ module symbol index {}: {}",
                path.display(),
                e
            )
        })?;
        let file = parse_cpp_module_symbol_index_file(path, &content)?;
        merge_cpp_module_symbol_index_file(&mut merged, path, file)?;
    }
    Ok(merged)
}

fn parse_cpp_module_symbol_index_file(
    path: &Path,
    content: &str,
) -> Result<CppModuleSymbolIndexFile, String> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase());
    let parsed: CppModuleSymbolIndexFile = match ext.as_deref() {
        Some("json") => serde_json::from_str(content).map_err(|e| {
            format!(
                "Invalid JSON C++ module symbol index {}: {}",
                path.display(),
                e
            )
        })?,
        Some("toml") => toml::from_str(content).map_err(|e| {
            format!(
                "Invalid TOML C++ module symbol index {}: {}",
                path.display(),
                e
            )
        })?,
        _ => match serde_json::from_str(content) {
            Ok(v) => v,
            Err(json_err) => toml::from_str(content).map_err(|toml_err| {
                format!(
                    "Failed to parse C++ module symbol index {} as JSON ({}) or TOML ({})",
                    path.display(),
                    json_err,
                    toml_err
                )
            })?,
        },
    };

    if parsed.version != 1 {
        return Err(format!(
            "Unsupported C++ module symbol index version {} in {} (expected version 1)",
            parsed.version,
            path.display()
        ));
    }
    Ok(parsed)
}

fn merge_cpp_module_symbol_index_file(
    merged: &mut CppModuleSymbolIndex,
    source_path: &Path,
    file: CppModuleSymbolIndexFile,
) -> Result<(), String> {
    for (raw_module_path, module) in file.modules {
        let module_path = canonical_cpp_module_path(&raw_module_path);
        if module_path.is_empty() {
            return Err(format!(
                "C++ module symbol index {} contains an empty module path key",
                source_path.display()
            ));
        }

        let incoming = CppModuleIndexModule {
            namespace: module.namespace,
            symbols: module
                .symbols
                .into_iter()
                .map(|(name, symbol)| {
                    (
                        name,
                        CppModuleIndexSymbol {
                            kind: symbol.kind,
                            callable_signatures: symbol.callable_signatures,
                        },
                    )
                })
                .collect(),
        };

        if let Some(existing) = merged.modules.get_mut(&module_path) {
            merge_cpp_module_entry(existing, &incoming, source_path, &module_path)?;
        } else {
            merged.modules.insert(module_path, incoming);
        }
    }
    Ok(())
}

fn merge_cpp_module_entry(
    existing: &mut CppModuleIndexModule,
    incoming: &CppModuleIndexModule,
    source_path: &Path,
    module_path: &str,
) -> Result<(), String> {
    match (&existing.namespace, &incoming.namespace) {
        (Some(a), Some(b)) if a != b => {
            return Err(format!(
                "C++ module symbol index {} has conflicting namespace for module '{}': '{}' vs '{}'",
                source_path.display(),
                module_path,
                a,
                b
            ));
        }
        (None, Some(ns)) => {
            existing.namespace = Some(ns.clone());
        }
        _ => {}
    }

    for (symbol_name, symbol) in &incoming.symbols {
        if symbol_name.trim().is_empty() {
            return Err(format!(
                "C++ module symbol index {} has empty symbol name in module '{}'",
                source_path.display(),
                module_path
            ));
        }
        if let Some(existing_symbol) = existing.symbols.get(symbol_name) {
            if existing_symbol != symbol {
                return Err(format!(
                    "C++ module symbol index {} has conflicting definition for '{}::{}'",
                    source_path.display(),
                    module_path,
                    symbol_name
                ));
            }
        } else {
            existing.symbols.insert(symbol_name.clone(), symbol.clone());
        }
    }
    Ok(())
}

fn canonical_cpp_module_path(path: &str) -> String {
    path.trim().replace('.', "::")
}

fn cpp_symbol_kind_contains(symbol: &CppModuleIndexSymbol, needle: &str) -> bool {
    symbol
        .kind
        .as_deref()
        .is_some_and(|kind| kind.to_ascii_lowercase().contains(needle))
}

fn cpp_symbol_is_member_method(symbol: &CppModuleIndexSymbol) -> bool {
    cpp_symbol_kind_contains(symbol, "method")
}

fn collect_cpp_module_member_symbol_map(
    index: &CppModuleSymbolIndex,
) -> HashMap<String, HashSet<String>> {
    let mut by_module: HashMap<String, HashSet<String>> = HashMap::new();
    for (module_path, module_entry) in &index.modules {
        let mut member_symbols = HashSet::new();
        for (symbol_name, symbol) in &module_entry.symbols {
            if cpp_symbol_is_member_method(symbol) {
                member_symbols.insert(symbol_name.clone());
            }
        }
        if !member_symbols.is_empty() {
            by_module.insert(module_path.clone(), member_symbols);
        }
    }
    by_module
}

/// Transpile Rust source code to C++ code.
/// If `module_name` is provided, emit C++20 module declarations.
pub fn transpile(rust_source: &str, module_name: Option<&str>) -> Result<String, String> {
    transpile_with_type_map(rust_source, module_name, &UserTypeMap::default())
}

/// Transpile with user-provided type mappings for external crate types.
pub fn transpile_with_type_map(
    rust_source: &str,
    module_name: Option<&str>,
    type_map: &UserTypeMap,
) -> Result<String, String> {
    transpile_with_type_map_and_extension_hints_and_options(
        rust_source,
        module_name,
        type_map,
        &HashSet::new(),
        &TranspileOptions::default(),
    )
}

/// Transpile with user-provided type mappings plus cross-source extension-method hints.
pub fn transpile_with_type_map_and_extension_hints(
    rust_source: &str,
    module_name: Option<&str>,
    type_map: &UserTypeMap,
    extension_method_hints: &HashSet<String>,
) -> Result<String, String> {
    transpile_with_type_map_and_extension_hints_and_options(
        rust_source,
        module_name,
        type_map,
        extension_method_hints,
        &TranspileOptions::default(),
    )
}

/// Transpile with user-provided type mappings plus cross-source extension-method
/// hints and explicit transpilation options.
pub fn transpile_with_type_map_and_extension_hints_and_options(
    rust_source: &str,
    module_name: Option<&str>,
    type_map: &UserTypeMap,
    extension_method_hints: &HashSet<String>,
    options: &TranspileOptions,
) -> Result<String, String> {
    transpile_full_with_options(
        rust_source,
        module_name,
        type_map,
        extension_method_hints,
        None,
        options,
    )
}

/// Transpile with all options including crate name for path stripping.
pub fn transpile_full(
    rust_source: &str,
    module_name: Option<&str>,
    type_map: &UserTypeMap,
    extension_method_hints: &HashSet<String>,
    crate_name: Option<&str>,
) -> Result<String, String> {
    transpile_full_with_options(
        rust_source,
        module_name,
        type_map,
        extension_method_hints,
        crate_name,
        &TranspileOptions::default(),
    )
}

/// Transpile with all options including crate name for path stripping and
/// explicit transpilation options.
pub fn transpile_full_with_options(
    rust_source: &str,
    module_name: Option<&str>,
    type_map: &UserTypeMap,
    extension_method_hints: &HashSet<String>,
    crate_name: Option<&str>,
    options: &TranspileOptions,
) -> Result<String, String> {
    let profile_transpile = std::env::var_os("RUSTY_CPP_PROFILE_TRANSPILE").is_some();
    let profile_this_call = profile_transpile && rust_source.lines().take(2001).count() >= 2000;
    let profile_start = std::time::Instant::now();
    let module_label = module_name.unwrap_or("<none>");
    let crate_label = crate_name.unwrap_or("<none>");
    let log_profile = |label: &str| {
        if profile_this_call {
            eprintln!(
                "[rusty-cpp][transpile-full] module={} crate={} {}: {:.3}s",
                module_label,
                crate_label,
                label,
                profile_start.elapsed().as_secs_f64()
            );
        }
    };
    log_profile("start");
    let file: syn::File = parse_with_expand_hygiene_fallback(rust_source)
        .map_err(|e| format!("Parse error: {}", e))?;
    log_profile("parse_with_expand_hygiene_fallback");
    let has_cpp_module_imports = file_contains_cpp_module_imports(&file);
    log_profile("file_contains_cpp_module_imports");
    if has_cpp_module_imports {
        match options.cpp_module_symbol_index.as_ref() {
            Some(index) if !index.modules.is_empty() => {}
            Some(_) => {
                return Err(
                    "Found `use cpp::...` import, but configured C++ module symbol index is empty"
                        .to_string(),
                )
            }
            None => {
                return Err(
                    "Found `use cpp::...` import, but no C++ module symbol index is configured. Pass --cpp-module-index <path>"
                        .to_string(),
                )
            }
        }
    }
    log_profile("cpp_module_index_validation");
    if has_cpp_module_imports {
        if let Some(index) = options.cpp_module_symbol_index.as_ref() {
            let resolution_diagnostics = collect_cpp_foreign_call_resolution_diagnostics(
                &file,
                index,
                &options.cpp_module_symbol_index_sources,
            );
            if !resolution_diagnostics.is_empty() {
                return Err(format!(
                    "Unresolved or invalid `cpp::` symbol usage detected:\n- {}",
                    resolution_diagnostics.join("\n- ")
                ));
            }
        }
    }
    log_profile("cpp_foreign_call_resolution_diagnostics");
    let cpp_call_unsafe_violations = collect_cpp_foreign_call_unsafe_violations(&file);
    log_profile("collect_cpp_foreign_call_unsafe_violations");
    if !cpp_call_unsafe_violations.is_empty() {
        return Err(format!(
            "Foreign C++ calls imported through `cpp::` require `unsafe` context:\n- {}",
            cpp_call_unsafe_violations.join("\n- ")
        ));
    }

    let mut codegen = if extension_method_hints.is_empty() {
        CodeGen::with_type_map(type_map.clone())
    } else {
        CodeGen::with_type_map_and_extension_hints(type_map.clone(), extension_method_hints.clone())
    };
    if let Some(name) = crate_name {
        codegen.set_crate_name(name);
    }
    codegen.set_by_value_cycle_breaking_prototype(options.by_value_cycle_breaking_prototype);
    codegen.set_external_crate_module_aliases(options.external_crate_module_aliases.clone());
    if let Some(index) = options.cpp_module_symbol_index.as_ref() {
        let member_symbols = collect_cpp_module_member_symbol_map(index);
        codegen.set_cpp_module_member_symbols(member_symbols);
    }
    log_profile("codegen_setup");
    codegen.emit_file(&file, module_name);
    log_profile("codegen_emit_file");
    Ok(codegen.into_output())
}

fn parse_with_expand_hygiene_fallback(rust_source: &str) -> Result<syn::File, syn::Error> {
    match syn::parse_str::<syn::File>(rust_source) {
        Ok(file) => Ok(file),
        Err(primary_err) => {
            // rustc/cargo-expand output can contain hygiene-prefixed statement
            // forms such as `super let ...` that are not valid source syntax.
            // Normalize that artifact and retry parsing once.
            let normalized = rust_source.replace("super let ", "let ");
            if normalized == rust_source {
                return Err(primary_err);
            }
            syn::parse_str::<syn::File>(&normalized).map_err(|_| primary_err)
        }
    }
}

fn file_contains_cpp_module_imports(file: &syn::File) -> bool {
    file.items.iter().any(item_contains_cpp_module_import)
}

fn item_contains_cpp_module_import(item: &syn::Item) -> bool {
    match item {
        syn::Item::Use(use_item) => use_tree_contains_cpp_module_root(&use_item.tree, true),
        syn::Item::Mod(module) => module
            .content
            .as_ref()
            .is_some_and(|(_, items)| items.iter().any(item_contains_cpp_module_import)),
        _ => false,
    }
}

fn use_tree_contains_cpp_module_root(tree: &syn::UseTree, at_root: bool) -> bool {
    match tree {
        syn::UseTree::Path(path) => {
            if at_root && path.ident == "cpp" {
                return true;
            }
            use_tree_contains_cpp_module_root(&path.tree, false)
        }
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .any(|item| use_tree_contains_cpp_module_root(item, at_root)),
        syn::UseTree::Name(_) | syn::UseTree::Rename(_) | syn::UseTree::Glob(_) => false,
    }
}

fn collect_cpp_foreign_call_unsafe_violations(file: &syn::File) -> Vec<String> {
    let mut visitor = CppForeignCallSafetyVisitor::default();
    visitor.visit_file(file);
    visitor.into_diagnostics()
}

#[derive(Default)]
struct CppForeignCallSafetyVisitor {
    cpp_binding_scopes: Vec<HashMap<String, String>>,
    unsafe_context_depth: usize,
    diagnostics: Vec<String>,
    diagnostic_keys: HashSet<String>,
    context_stack: Vec<String>,
}

impl CppForeignCallSafetyVisitor {
    fn push_cpp_binding_scope(&mut self, bindings: HashMap<String, String>) {
        self.cpp_binding_scopes.push(bindings);
    }

    fn pop_cpp_binding_scope(&mut self) {
        self.cpp_binding_scopes.pop();
    }

    fn lookup_cpp_binding(&self, binding: &str) -> Option<&str> {
        for scope in self.cpp_binding_scopes.iter().rev() {
            if let Some(module_path) = scope.get(binding) {
                return Some(module_path);
            }
        }
        None
    }

    fn current_context_label(&self) -> String {
        if self.context_stack.is_empty() {
            "<module>".to_string()
        } else {
            self.context_stack.join("::")
        }
    }

    fn record_safe_context_cpp_call_violation(
        &mut self,
        call: &syn::ExprCall,
        binding_name: &str,
        module_path: &str,
    ) {
        let call_site = call.to_token_stream().to_string();
        let context = self.current_context_label();
        let key = format!("{}|{}", context, call_site);
        if self.diagnostic_keys.insert(key) {
            self.diagnostics.push(format!(
                "safe-context foreign C++ call requires `unsafe`: `{}` (binding `{}` -> `{}`) in `{}`",
                call_site, binding_name, module_path, context
            ));
        }
    }

    fn check_cpp_call_requires_unsafe(&mut self, call: &syn::ExprCall) {
        if self.unsafe_context_depth > 0 {
            return;
        }
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return;
        };
        if path_expr.path.segments.len() < 2 {
            return;
        }
        let Some(first_segment) = path_expr.path.segments.first() else {
            return;
        };
        let binding_name = first_segment.ident.to_string();
        let Some(module_path) = self
            .lookup_cpp_binding(&binding_name)
            .map(ToOwned::to_owned)
        else {
            return;
        };
        self.record_safe_context_cpp_call_violation(call, &binding_name, &module_path);
    }

    fn into_diagnostics(mut self) -> Vec<String> {
        self.diagnostics.sort();
        self.diagnostics.dedup();
        self.diagnostics
    }
}

impl<'ast> Visit<'ast> for CppForeignCallSafetyVisitor {
    fn visit_file(&mut self, file: &'ast syn::File) {
        self.push_cpp_binding_scope(collect_cpp_bindings_from_items(&file.items));
        for item in &file.items {
            self.visit_item(item);
        }
        self.pop_cpp_binding_scope();
    }

    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        let Some((_, items)) = &module.content else {
            return;
        };
        self.context_stack.push(module.ident.to_string());
        self.push_cpp_binding_scope(collect_cpp_bindings_from_items(items));
        for item in items {
            self.visit_item(item);
        }
        self.pop_cpp_binding_scope();
        self.context_stack.pop();
    }

    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        self.context_stack.push(function.sig.ident.to_string());
        let was_unsafe = function.sig.unsafety.is_some();
        if was_unsafe {
            self.unsafe_context_depth += 1;
        }
        visit::visit_block(self, &function.block);
        if was_unsafe {
            self.unsafe_context_depth -= 1;
        }
        self.context_stack.pop();
    }

    fn visit_impl_item_fn(&mut self, method: &'ast syn::ImplItemFn) {
        self.context_stack.push(method.sig.ident.to_string());
        let was_unsafe = method.sig.unsafety.is_some();
        if was_unsafe {
            self.unsafe_context_depth += 1;
        }
        visit::visit_block(self, &method.block);
        if was_unsafe {
            self.unsafe_context_depth -= 1;
        }
        self.context_stack.pop();
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.push_cpp_binding_scope(collect_cpp_bindings_from_stmts(&block.stmts));
        for stmt in &block.stmts {
            self.visit_stmt(stmt);
        }
        self.pop_cpp_binding_scope();
    }

    fn visit_expr_unsafe(&mut self, unsafe_expr: &'ast syn::ExprUnsafe) {
        self.unsafe_context_depth += 1;
        visit::visit_expr_unsafe(self, unsafe_expr);
        self.unsafe_context_depth -= 1;
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        self.check_cpp_call_requires_unsafe(call);
        visit::visit_expr_call(self, call);
    }
}

fn collect_cpp_foreign_call_resolution_diagnostics(
    file: &syn::File,
    index: &CppModuleSymbolIndex,
    index_sources: &[PathBuf],
) -> Vec<String> {
    let mut visitor = CppForeignCallResolutionVisitor::new(index, index_sources);
    visitor.visit_file(file);
    visitor.into_diagnostics()
}

struct CppForeignCallResolutionVisitor<'a> {
    cpp_binding_scopes: Vec<HashMap<String, String>>,
    diagnostics: Vec<String>,
    diagnostic_keys: HashSet<String>,
    context_stack: Vec<String>,
    index: &'a CppModuleSymbolIndex,
    index_source_label: String,
}

impl<'a> CppForeignCallResolutionVisitor<'a> {
    fn new(index: &'a CppModuleSymbolIndex, index_sources: &[PathBuf]) -> Self {
        Self {
            cpp_binding_scopes: Vec::new(),
            diagnostics: Vec::new(),
            diagnostic_keys: HashSet::new(),
            context_stack: Vec::new(),
            index,
            index_source_label: format_cpp_module_index_sources(index_sources),
        }
    }

    fn push_cpp_binding_scope(&mut self, bindings: HashMap<String, String>) {
        self.cpp_binding_scopes.push(bindings);
    }

    fn pop_cpp_binding_scope(&mut self) {
        self.cpp_binding_scopes.pop();
    }

    fn lookup_cpp_binding(&self, binding: &str) -> Option<&str> {
        for scope in self.cpp_binding_scopes.iter().rev() {
            if let Some(module_path) = scope.get(binding) {
                return Some(module_path);
            }
        }
        None
    }

    fn current_context_label(&self) -> String {
        if self.context_stack.is_empty() {
            "<module>".to_string()
        } else {
            self.context_stack.join("::")
        }
    }

    fn record_diagnostic(
        &mut self,
        site: &str,
        module_path: &str,
        symbol_name: &str,
        detail: &str,
    ) {
        let context = self.current_context_label();
        let key = format!("{}|{}|{}|{}", context, module_path, symbol_name, detail);
        if self.diagnostic_keys.insert(key) {
            self.diagnostics.push(format!(
                "{} (module `{}`, symbol `{}`, index source `{}`, call `{}`, context `{}`)",
                detail, module_path, symbol_name, self.index_source_label, site, context
            ));
        }
    }

    fn resolve_cpp_symbol_for_path(&self, path: &syn::Path) -> Option<(String, String)> {
        if path.segments.len() < 2 {
            return None;
        }
        let first_segment = path.segments.first()?;
        let binding_name = first_segment.ident.to_string();
        let module_path = self.lookup_cpp_binding(&binding_name)?.to_string();
        let symbol_name = path
            .segments
            .iter()
            .skip(1)
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<String>>()
            .join("::");
        if symbol_name.is_empty() {
            return None;
        }
        Some((module_path, symbol_name))
    }

    fn lookup_index_symbol<'b>(
        &self,
        module: &'b CppModuleIndexModule,
        symbol_name: &str,
    ) -> Option<&'b CppModuleIndexSymbol> {
        module.symbols.get(symbol_name).or_else(|| {
            symbol_name
                .rsplit("::")
                .next()
                .and_then(|tail| module.symbols.get(tail))
        })
    }

    fn symbol_kind_contains(symbol: &CppModuleIndexSymbol, needle: &str) -> bool {
        symbol
            .kind
            .as_deref()
            .is_some_and(|kind| kind.to_ascii_lowercase().contains(needle))
    }

    fn symbol_is_macro(symbol: &CppModuleIndexSymbol) -> bool {
        Self::symbol_kind_contains(symbol, "macro")
    }

    fn symbol_is_template(symbol: &CppModuleIndexSymbol) -> bool {
        Self::symbol_kind_contains(symbol, "template")
    }

    fn symbol_is_member_method(symbol: &CppModuleIndexSymbol) -> bool {
        Self::symbol_kind_contains(symbol, "method")
    }

    fn symbol_is_callable_kind(symbol: &CppModuleIndexSymbol) -> bool {
        Self::symbol_kind_contains(symbol, "function")
            || Self::symbol_kind_contains(symbol, "method")
            || Self::symbol_kind_contains(symbol, "callable")
            || Self::symbol_kind_contains(symbol, "ctor")
            || Self::symbol_kind_contains(symbol, "constructor")
    }

    fn validate_cpp_module_symbol_access(
        &mut self,
        site: &str,
        module_path: &str,
        symbol_name: &str,
    ) -> Option<CppModuleIndexSymbol> {
        let Some(module) = self.index.modules.get(module_path) else {
            self.record_diagnostic(
                site,
                module_path,
                symbol_name,
                "module path is not present in configured C++ module symbol index",
            );
            return None;
        };
        let Some(symbol) = self.lookup_index_symbol(module, symbol_name) else {
            self.record_diagnostic(
                site,
                module_path,
                symbol_name,
                "symbol is not present in configured C++ module symbol index module entry",
            );
            return None;
        };
        Some(symbol.clone())
    }

    fn validate_cpp_call_symbol(&mut self, call: &syn::ExprCall) {
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return;
        };
        let Some((module_path, symbol_name)) = self.resolve_cpp_symbol_for_path(&path_expr.path)
        else {
            return;
        };
        let call_site = call.to_token_stream().to_string();

        let Some(symbol) =
            self.validate_cpp_module_symbol_access(&call_site, &module_path, &symbol_name)
        else {
            return;
        };
        if Self::symbol_is_macro(&symbol) {
            self.record_diagnostic(
                &call_site,
                &module_path,
                &symbol_name,
                "TODO(leaf22.7): `cpp::` macro exports are unsupported in MVP",
            );
            return;
        }

        let call_arity = call.args.len();
        let member_style_arity = (path_expr.path.segments.len() > 2
            && call_arity > 0
            && Self::symbol_is_member_method(&symbol))
        .then_some(call_arity - 1);
        if Self::symbol_is_template(&symbol) && symbol.callable_signatures.is_empty() {
            self.record_diagnostic(
                &call_site,
                &module_path,
                &symbol_name,
                "TODO(leaf22.7): template-only export without indexed callable signatures is unsupported in MVP",
            );
            return;
        }
        if symbol.callable_signatures.is_empty() {
            self.record_diagnostic(
                &call_site,
                &module_path,
                &symbol_name,
                "call cannot be matched to indexed callable family (no callable signatures indexed)",
            );
            return;
        }

        let mut has_arity_match = false;
        for signature in &symbol.callable_signatures {
            if parse_callable_signature_arity(signature).is_some_and(|arity| {
                arity == call_arity || member_style_arity.is_some_and(|adjusted| arity == adjusted)
            }) {
                has_arity_match = true;
                break;
            }
        }
        if !has_arity_match {
            let arity_label = if let Some(adjusted) = member_style_arity {
                format!("{} (receiver-adjusted: {})", call_arity, adjusted)
            } else {
                call_arity.to_string()
            };
            self.record_diagnostic(
                &call_site,
                &module_path,
                &symbol_name,
                &format!(
                    "call cannot be matched to indexed callable family (arity {} does not match signatures [{}])",
                    arity_label,
                    symbol.callable_signatures.join(", ")
                ),
            );
        }
    }

    fn validate_cpp_value_symbol(&mut self, path_expr: &syn::ExprPath) {
        let Some((module_path, symbol_name)) = self.resolve_cpp_symbol_for_path(&path_expr.path)
        else {
            return;
        };
        let path_site = path_expr.to_token_stream().to_string();
        if path_expr.path.segments.len() > 2 {
            self.record_diagnostic(
                &path_site,
                &module_path,
                &symbol_name,
                "TODO(leaf22.7): member-function import syntax is unsupported for `cpp::` MVP (only module constants are supported in non-call positions)",
            );
            return;
        }
        let Some(symbol) =
            self.validate_cpp_module_symbol_access(&path_site, &module_path, &symbol_name)
        else {
            return;
        };

        if Self::symbol_is_macro(&symbol) {
            self.record_diagnostic(
                &path_site,
                &module_path,
                &symbol_name,
                "TODO(leaf22.7): `cpp::` macro exports are unsupported in MVP",
            );
            return;
        }

        if Self::symbol_is_template(&symbol) && symbol.callable_signatures.is_empty() {
            self.record_diagnostic(
                &path_site,
                &module_path,
                &symbol_name,
                "TODO(leaf22.7): template-only export without indexed callable signatures is unsupported in MVP",
            );
            return;
        }

        if Self::symbol_is_callable_kind(&symbol) || !symbol.callable_signatures.is_empty() {
            self.record_diagnostic(
                &path_site,
                &module_path,
                &symbol_name,
                "TODO(leaf22.7): non-call function symbol usage is unsupported for `cpp::` MVP (only module constants are supported in value position)",
            );
        }
    }

    fn validate_cpp_macro_symbol_with_site(&mut self, path: &syn::Path, site: &str) {
        let Some((module_path, symbol_name)) = self.resolve_cpp_symbol_for_path(path) else {
            return;
        };
        self.record_diagnostic(
            site,
            &module_path,
            &symbol_name,
            "TODO(leaf22.7): `cpp::` macro imports are unsupported in MVP",
        );
    }

    fn into_diagnostics(mut self) -> Vec<String> {
        self.diagnostics.sort();
        self.diagnostics.dedup();
        self.diagnostics
    }
}

impl<'ast> Visit<'ast> for CppForeignCallResolutionVisitor<'_> {
    fn visit_file(&mut self, file: &'ast syn::File) {
        self.push_cpp_binding_scope(collect_cpp_bindings_from_items(&file.items));
        for item in &file.items {
            self.visit_item(item);
        }
        self.pop_cpp_binding_scope();
    }

    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        let Some((_, items)) = &module.content else {
            return;
        };
        self.context_stack.push(module.ident.to_string());
        self.push_cpp_binding_scope(collect_cpp_bindings_from_items(items));
        for item in items {
            self.visit_item(item);
        }
        self.pop_cpp_binding_scope();
        self.context_stack.pop();
    }

    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        self.context_stack.push(function.sig.ident.to_string());
        visit::visit_block(self, &function.block);
        self.context_stack.pop();
    }

    fn visit_impl_item_fn(&mut self, method: &'ast syn::ImplItemFn) {
        self.context_stack.push(method.sig.ident.to_string());
        visit::visit_block(self, &method.block);
        self.context_stack.pop();
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.push_cpp_binding_scope(collect_cpp_bindings_from_stmts(&block.stmts));
        for stmt in &block.stmts {
            self.visit_stmt(stmt);
        }
        self.pop_cpp_binding_scope();
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        self.validate_cpp_call_symbol(call);
        let cpp_bound_call_path = match call.func.as_ref() {
            syn::Expr::Path(path_expr) => {
                self.resolve_cpp_symbol_for_path(&path_expr.path).is_some()
            }
            _ => false,
        };
        if !cpp_bound_call_path {
            self.visit_expr(&call.func);
        }
        for arg in &call.args {
            self.visit_expr(arg);
        }
    }

    fn visit_expr_path(&mut self, path_expr: &'ast syn::ExprPath) {
        self.validate_cpp_value_symbol(path_expr);
        visit::visit_expr_path(self, path_expr);
    }

    fn visit_expr_macro(&mut self, expr_macro: &'ast syn::ExprMacro) {
        let site = expr_macro.to_token_stream().to_string();
        self.validate_cpp_macro_symbol_with_site(&expr_macro.mac.path, &site);
        visit::visit_expr_macro(self, expr_macro);
    }

    fn visit_stmt_macro(&mut self, stmt_macro: &'ast syn::StmtMacro) {
        let site = stmt_macro.mac.to_token_stream().to_string();
        self.validate_cpp_macro_symbol_with_site(&stmt_macro.mac.path, &site);
        visit::visit_stmt_macro(self, stmt_macro);
    }
}

fn format_cpp_module_index_sources(index_sources: &[PathBuf]) -> String {
    if index_sources.is_empty() {
        "<unknown>".to_string()
    } else {
        index_sources
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<String>>()
            .join(", ")
    }
}

fn parse_callable_signature_arity(signature: &str) -> Option<usize> {
    let start = signature.find('(')?;
    let end = signature.rfind(')')?;
    if end < start {
        return None;
    }
    let args = signature[start + 1..end].trim();
    if args.is_empty() {
        return Some(0);
    }

    let mut arity = 1usize;
    let mut paren_depth = 0usize;
    let mut angle_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    for ch in args.chars() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '<' => angle_depth += 1,
            '>' => angle_depth = angle_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            ',' if paren_depth == 0
                && angle_depth == 0
                && bracket_depth == 0
                && brace_depth == 0 =>
            {
                arity += 1;
            }
            _ => {}
        }
    }
    Some(arity)
}

fn collect_cpp_bindings_from_items(items: &[syn::Item]) -> HashMap<String, String> {
    let mut bindings = HashMap::new();
    for item in items {
        if let syn::Item::Use(use_item) = item {
            collect_cpp_bindings_from_use_tree(&use_item.tree, true, false, "", &mut bindings);
        }
    }
    bindings
}

fn collect_cpp_bindings_from_stmts(stmts: &[syn::Stmt]) -> HashMap<String, String> {
    let mut bindings = HashMap::new();
    for stmt in stmts {
        if let syn::Stmt::Item(syn::Item::Use(use_item)) = stmt {
            collect_cpp_bindings_from_use_tree(&use_item.tree, true, false, "", &mut bindings);
        }
    }
    bindings
}

fn collect_cpp_bindings_from_use_tree(
    tree: &syn::UseTree,
    at_root: bool,
    in_cpp_root: bool,
    prefix: &str,
    out: &mut HashMap<String, String>,
) {
    match tree {
        syn::UseTree::Path(path) => {
            if in_cpp_root {
                let new_prefix = join_cpp_module_prefix(prefix, &path.ident.to_string());
                collect_cpp_bindings_from_use_tree(&path.tree, false, true, &new_prefix, out);
            } else if at_root && path.ident == "cpp" {
                collect_cpp_bindings_from_use_tree(&path.tree, false, true, "", out);
            } else {
                collect_cpp_bindings_from_use_tree(&path.tree, false, false, prefix, out);
            }
        }
        syn::UseTree::Name(name) => {
            if !in_cpp_root {
                return;
            }
            if name.ident == "self" {
                if let Some(binding) = cpp_module_tail_segment(prefix) {
                    record_cpp_binding(out, binding.to_string(), prefix.to_string());
                }
                return;
            }
            let ident = name.ident.to_string();
            let module_path = join_cpp_module_prefix(prefix, &ident);
            record_cpp_binding(out, ident, module_path);
        }
        syn::UseTree::Rename(rename) => {
            if !in_cpp_root {
                return;
            }
            let target = if rename.ident == "self" {
                prefix.to_string()
            } else {
                join_cpp_module_prefix(prefix, &rename.ident.to_string())
            };
            if target.is_empty() {
                return;
            }
            record_cpp_binding(out, rename.rename.to_string(), target);
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_cpp_bindings_from_use_tree(item, at_root, in_cpp_root, prefix, out);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

fn join_cpp_module_prefix(prefix: &str, segment: &str) -> String {
    if prefix.is_empty() {
        segment.to_string()
    } else {
        format!("{}::{}", prefix, segment)
    }
}

fn cpp_module_tail_segment(path: &str) -> Option<&str> {
    path.rsplit("::").find(|segment| !segment.is_empty())
}

fn record_cpp_binding(out: &mut HashMap<String, String>, binding: String, module_path: String) {
    if binding.is_empty() || module_path.is_empty() {
        return;
    }
    let canonical = canonical_cpp_module_path(&module_path);
    out.entry(binding).or_insert(canonical);
}

/// Collect extension-method names from a Rust source unit.
/// A method is treated as extension-shaped when it appears in a trait impl
/// targeting a non-local type in that same source unit.
pub fn collect_extension_method_hints(rust_source: &str) -> HashSet<String> {
    let Ok(file) = syn::parse_str::<syn::File>(rust_source) else {
        return HashSet::new();
    };

    let mut local_types = HashSet::new();
    collect_local_declared_types(&file.items, &[], &mut local_types);

    let mut methods = HashSet::new();
    collect_extension_method_names(&file.items, &[], &local_types, &mut methods);
    methods
}

fn collect_local_declared_types(
    items: &[syn::Item],
    module_path: &[String],
    out: &mut HashSet<String>,
) {
    for item in items {
        match item {
            syn::Item::Struct(s) => record_local_type(module_path, &s.ident.to_string(), out),
            syn::Item::Enum(e) => record_local_type(module_path, &e.ident.to_string(), out),
            syn::Item::Type(t) => record_local_type(module_path, &t.ident.to_string(), out),
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    let mut nested_path = module_path.to_vec();
                    nested_path.push(m.ident.to_string());
                    collect_local_declared_types(nested, &nested_path, out);
                }
            }
            _ => {}
        }
    }
}

fn record_local_type(module_path: &[String], type_name: &str, out: &mut HashSet<String>) {
    out.insert(type_name.to_string());
    if !module_path.is_empty() {
        out.insert(format!("{}::{}", module_path.join("::"), type_name));
    }
}

fn collect_extension_method_names(
    items: &[syn::Item],
    module_path: &[String],
    local_types: &HashSet<String>,
    out: &mut HashSet<String>,
) {
    for item in items {
        match item {
            syn::Item::Impl(impl_block) => {
                if impl_block.trait_.is_none() {
                    continue;
                }
                let Some(tp) = (match impl_block.self_ty.as_ref() {
                    syn::Type::Path(tp) => Some(tp),
                    _ => None,
                }) else {
                    continue;
                };

                let raw_self_name = tp
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                let scoped_self_name = qualify_relative_path(&raw_self_name, module_path);
                if local_types.contains(&raw_self_name) || local_types.contains(&scoped_self_name) {
                    continue;
                }

                for impl_item in &impl_block.items {
                    if let syn::ImplItem::Fn(method) = impl_item {
                        out.insert(method.sig.ident.to_string());
                    }
                }
            }
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    let mut nested_path = module_path.to_vec();
                    nested_path.push(m.ident.to_string());
                    collect_extension_method_names(nested, &nested_path, local_types, out);
                }
            }
            _ => {}
        }
    }
}

fn qualify_relative_path(raw: &str, module_path: &[String]) -> String {
    let parts: Vec<&str> = raw.split("::").collect();
    if parts.is_empty() {
        return raw.to_string();
    }
    if parts.len() == 1 {
        if module_path.is_empty() {
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

    let mut out_parts = resolved_prefix;
    out_parts.extend(parts[idx..].iter().map(|s| s.to_string()));
    if out_parts.is_empty() {
        raw.to_string()
    } else {
        out_parts.join("::")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_transpile_basic() {
        let result = transpile("fn main() { let x = 42; }", None);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("void main()"));
        assert!(output.contains("const auto x = 42;"));
    }

    #[test]
    fn test_transpile_error() {
        let result = transpile("fn {{{ invalid", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_transpile_parses_cargo_expand_super_let_hygiene_artifact() {
        let result = transpile(
            r#"
            fn f(v: i32) -> i32 {
                let out = {
                    super let mut inner = v;
                    inner += 1;
                    inner
                };
                out
            }
            "#,
            None,
        );
        assert!(result.is_ok(), "{result:?}");
        let output = result.unwrap();
        assert!(output.contains("int32_t f"));
    }

    #[test]
    fn test_transpile_multiple_items() {
        let result = transpile(
            r#"
            struct Point { x: f64, y: f64 }
            const PI: f64 = 3.14159;
            fn distance(a: &Point, b: &Point) -> f64 {
                0.0
            }
        "#,
            None,
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("struct Point"));
        assert!(output.contains("constexpr double PI"));
        assert!(output.contains("double distance"));
    }

    #[test]
    fn test_transpile_complete_program() {
        let result = transpile(
            r#"
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }

            fn main() {
                let result = add(1, 2);
            }
        "#,
            None,
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("int32_t add(int32_t a, int32_t b)"));
        assert!(output.contains("return a + b;"));
        assert!(output.contains("void main()"));
        assert!(output.contains("add(1, 2)"));
    }

    #[test]
    fn test_transpile_with_module() {
        let result = transpile("pub fn hello() {}", Some("my_crate"));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("export module my_crate;"));
        assert!(output.contains("export void hello()"));
    }

    #[test]
    fn test_transpile_without_module() {
        let result = transpile("pub fn hello() {}", None);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.contains("export module"));
        // Without module mode, pub is ignored
    }

    #[test]
    fn test_collect_extension_method_hints_detects_non_local_impl_methods() {
        let src = r#"
            struct Local;
            trait TapOps { fn tap(self) -> Self; }
            impl TapOps for Local { fn tap(self) -> Self { self } }
            trait TapOptionOps<T> { fn tap_none<F>(self, f: F) -> Self; }
            impl<T> TapOptionOps<T> for Option<T> { fn tap_none<F>(self, f: F) -> Self { self } }
        "#;
        let hints = collect_extension_method_hints(src);
        assert!(hints.contains("tap_none"));
        assert!(!hints.contains("tap"));
    }

    #[test]
    fn test_transpile_with_extension_hints_rewrites_method_calls() {
        let mut hints = HashSet::new();
        hints.insert("tap".to_string());
        let result = transpile_with_type_map_and_extension_hints(
            "fn f() { let _ = 10.tap(); }",
            None,
            &UserTypeMap::default(),
            &hints,
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("static_cast<void>(rusty_ext::tap(10));"));
    }

    #[test]
    fn test_transpile_with_runtime_extension_hints_keeps_rusty_namespace() {
        let mut hints = HashSet::new();
        hints.insert("size_hint".to_string());
        let result = transpile_with_type_map_and_extension_hints(
            "fn f(iter: std::ops::Range<i32>) { let _ = iter.size_hint(); }",
            None,
            &UserTypeMap::default(),
            &hints,
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("rusty::size_hint(iter)"));
    }

    #[test]
    fn test_transpile_with_external_tap_err_hint_routes_to_rusty_ext() {
        let mut hints = HashSet::new();
        hints.insert("tap_err".to_string());
        let result = transpile_with_type_map_and_extension_hints(
            r#"
            fn f(result: Result<i32, i32>) {
                let _ = result.tap_err(|e| {
                    let _ = *e;
                });
            }
            "#,
            None,
            &UserTypeMap::default(),
            &hints,
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("rusty_ext::tap_err(result,"));
        assert!(!output.contains("rusty::tap_err("));
    }

    #[test]
    fn test_transpile_options_toggle_by_value_cycle_breaking_prototype_diagnostics() {
        let src = r#"
            struct A {
                b: B,
            }

            struct B {
                a: A,
            }
        "#;
        let default_out = transpile(src, None).expect("default transpile should succeed");
        assert!(
            !default_out.contains("// PROTOTYPE: by-value cycle-breaking flag enabled"),
            "default mode should not emit prototype cycle-breaking diagnostics\nGot: {default_out}"
        );

        let options = TranspileOptions {
            by_value_cycle_breaking_prototype: true,
            ..TranspileOptions::default()
        };
        let opt_in_out = transpile_full_with_options(
            src,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("opt-in transpile should succeed");
        assert!(
            opt_in_out.contains("// PROTOTYPE: by-value cycle-breaking flag enabled"),
            "opt-in mode should emit prototype cycle-breaking diagnostics\nGot: {opt_in_out}"
        );
    }

    #[test]
    fn test_load_cpp_module_symbol_index_json() {
        let dir = tempdir().expect("tempdir");
        let index_path = dir.path().join("cpp_index.json");
        std::fs::write(
            &index_path,
            r#"
{
  "version": 1,
  "modules": {
    "std": {
      "namespace": "std",
      "symbols": {
        "max": {
          "kind": "function",
          "callable_signatures": ["int(int,int)"]
        }
      }
    }
  }
}
"#,
        )
        .expect("write json index");

        let index = load_cpp_module_symbol_index_files(&[index_path]).expect("load json index");
        let std_module = index.modules.get("std").expect("std module");
        assert_eq!(std_module.namespace.as_deref(), Some("std"));
        let max = std_module.symbols.get("max").expect("max symbol");
        assert_eq!(max.kind.as_deref(), Some("function"));
        assert_eq!(max.callable_signatures, vec!["int(int,int)".to_string()]);
    }

    #[test]
    fn test_load_cpp_module_symbol_index_toml() {
        let dir = tempdir().expect("tempdir");
        let index_path = dir.path().join("cpp_index.toml");
        std::fs::write(
            &index_path,
            r#"
version = 1

[modules.std]
namespace = "std"

[modules.std.symbols.max]
kind = "function"
callable_signatures = ["int(int,int)"]
"#,
        )
        .expect("write toml index");

        let index = load_cpp_module_symbol_index_files(&[index_path]).expect("load toml index");
        let std_module = index.modules.get("std").expect("std module");
        assert_eq!(std_module.namespace.as_deref(), Some("std"));
        let max = std_module.symbols.get("max").expect("max symbol");
        assert_eq!(max.kind.as_deref(), Some("function"));
        assert_eq!(max.callable_signatures, vec!["int(int,int)".to_string()]);
    }

    #[test]
    fn test_cpp_module_import_requires_symbol_index() {
        let err = transpile("use cpp::std as cpp_std;\nfn f() {}", None)
            .expect_err("cpp import without index should fail");
        assert!(err.contains("no C++ module symbol index is configured"));
        assert!(err.contains("--cpp-module-index"));
    }

    #[test]
    fn test_cpp_module_import_with_symbol_index_is_allowed() {
        let mut modules = BTreeMap::new();
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols: BTreeMap::new(),
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            ..TranspileOptions::default()
        };

        let output = transpile_full_with_options(
            "use cpp::std as cpp_std;\nfn f() {}",
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("cpp import with index should transpile");
        assert!(output.contains("// C++ module import (reserved cpp::): std as cpp_std"));
    }

    #[test]
    fn test_cpp_module_foreign_call_requires_unsafe_context() {
        let mut modules = BTreeMap::new();
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "max".to_string(),
            CppModuleIndexSymbol {
                kind: Some("function".to_string()),
                callable_signatures: vec!["int(int,int)".to_string()],
            },
        );
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols,
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            ..TranspileOptions::default()
        };

        let err = transpile_full_with_options(
            r#"
use cpp::std as cpp_std;
fn max2(lo: i32, hi: i32) -> i32 {
    cpp_std::max(lo, hi)
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("safe-context foreign C++ call should fail");

        assert!(err.contains("require `unsafe` context"));
        assert!(err.contains("cpp_std"));
        assert!(err.contains("max2"));
    }

    #[test]
    fn test_cpp_module_foreign_call_in_unsafe_context_is_allowed() {
        let mut modules = BTreeMap::new();
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "max".to_string(),
            CppModuleIndexSymbol {
                kind: Some("function".to_string()),
                callable_signatures: vec!["int(int,int)".to_string()],
            },
        );
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols,
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            ..TranspileOptions::default()
        };

        let output = transpile_full_with_options(
            r#"
use cpp::std as cpp_std;
fn max2(lo: i32, hi: i32) -> i32 {
    unsafe { cpp_std::max(lo, hi) }
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("unsafe-context foreign C++ call should transpile");

        assert!(output.contains("// @unsafe"));
        assert!(output.contains("std::max("));
    }

    #[test]
    fn test_cpp_module_call_errors_when_module_path_missing_from_index() {
        let mut modules = BTreeMap::new();
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols: BTreeMap::new(),
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            cpp_module_symbol_index_sources: vec![PathBuf::from("/tmp/cpp-index.toml")],
            ..TranspileOptions::default()
        };

        let err = transpile_full_with_options(
            r#"
use cpp::alpha::beta;
fn f(v: i32) -> i32 {
    unsafe { beta::transform(v) }
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("missing cpp module path should fail");

        assert!(err.contains("module path is not present"));
        assert!(err.contains("module `alpha::beta`"));
        assert!(err.contains("symbol `transform`"));
        assert!(err.contains("/tmp/cpp-index.toml"));
    }

    #[test]
    fn test_cpp_module_call_errors_when_symbol_missing_from_index_module() {
        let mut modules = BTreeMap::new();
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "max".to_string(),
            CppModuleIndexSymbol {
                kind: Some("function".to_string()),
                callable_signatures: vec!["int(int,int)".to_string()],
            },
        );
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols,
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            cpp_module_symbol_index_sources: vec![PathBuf::from("/tmp/cpp-index.toml")],
            ..TranspileOptions::default()
        };

        let err = transpile_full_with_options(
            r#"
use cpp::std as cpp_std;
fn f() -> i32 {
    unsafe { cpp_std::min(1, 2) }
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("missing indexed symbol should fail");

        assert!(err.contains("symbol is not present"));
        assert!(err.contains("module `std`"));
        assert!(err.contains("symbol `min`"));
        assert!(err.contains("/tmp/cpp-index.toml"));
    }

    #[test]
    fn test_cpp_module_call_errors_when_signature_family_does_not_match_call_shape() {
        let mut modules = BTreeMap::new();
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "max".to_string(),
            CppModuleIndexSymbol {
                kind: Some("function".to_string()),
                callable_signatures: vec!["int(int,int)".to_string()],
            },
        );
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols,
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            cpp_module_symbol_index_sources: vec![PathBuf::from("/tmp/cpp-index.toml")],
            ..TranspileOptions::default()
        };

        let err = transpile_full_with_options(
            r#"
use cpp::std as cpp_std;
fn f() -> i32 {
    unsafe { cpp_std::max(1) }
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("call arity mismatch should fail");

        assert!(err.contains("call cannot be matched to indexed callable family"));
        assert!(err.contains("arity 1"));
        assert!(err.contains("int(int,int)"));
        assert!(err.contains("/tmp/cpp-index.toml"));
    }

    #[test]
    fn test_cpp_module_constant_value_access_is_allowed() {
        let mut modules = BTreeMap::new();
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "ANSWER".to_string(),
            CppModuleIndexSymbol {
                kind: Some("constant".to_string()),
                callable_signatures: Vec::new(),
            },
        );
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols,
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            cpp_module_symbol_index_sources: vec![PathBuf::from("/tmp/cpp-index.toml")],
            ..TranspileOptions::default()
        };

        let output = transpile_full_with_options(
            r#"
use cpp::std as cpp_std;
fn f() -> i32 {
    cpp_std::ANSWER
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("module-constant access should transpile");

        assert!(output.contains("std::ANSWER"));
    }

    #[test]
    fn test_cpp_module_constant_access_errors_when_symbol_missing_from_index_module() {
        let mut modules = BTreeMap::new();
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "max".to_string(),
            CppModuleIndexSymbol {
                kind: Some("function".to_string()),
                callable_signatures: vec!["int(int,int)".to_string()],
            },
        );
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols,
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            cpp_module_symbol_index_sources: vec![PathBuf::from("/tmp/cpp-index.toml")],
            ..TranspileOptions::default()
        };

        let err = transpile_full_with_options(
            r#"
use cpp::std as cpp_std;
fn f() -> i32 {
    cpp_std::ANSWER
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("missing module constant should fail");

        assert!(err.contains("symbol is not present"));
        assert!(err.contains("symbol `ANSWER`"));
        assert!(err.contains("/tmp/cpp-index.toml"));
    }

    #[test]
    fn test_cpp_module_call_member_function_import_syntax_is_allowed() {
        let mut modules = BTreeMap::new();
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "vector::push_back".to_string(),
            CppModuleIndexSymbol {
                kind: Some("method".to_string()),
                callable_signatures: vec!["void(int)".to_string()],
            },
        );
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols,
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            cpp_module_symbol_index_sources: vec![PathBuf::from("/tmp/cpp-index.toml")],
            ..TranspileOptions::default()
        };

        let out = transpile_full_with_options(
            r#"
use cpp::std as cpp_std;
fn f(v: i32) -> i32 {
    let mut vec: *mut i32 = core::ptr::null_mut();
    unsafe { cpp_std::vector::push_back(vec, v) }
    0
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("member-function import syntax should transpile");

        assert!(out.contains("vec->push_back("));
    }

    #[test]
    fn test_cpp_module_call_errors_for_template_only_export_without_call_shape() {
        let mut modules = BTreeMap::new();
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "sort".to_string(),
            CppModuleIndexSymbol {
                kind: Some("function_template".to_string()),
                callable_signatures: Vec::new(),
            },
        );
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols,
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            cpp_module_symbol_index_sources: vec![PathBuf::from("/tmp/cpp-index.toml")],
            ..TranspileOptions::default()
        };

        let err = transpile_full_with_options(
            r#"
use cpp::std as cpp_std;
fn f(v: i32) -> i32 {
    unsafe { cpp_std::sort(v) }
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("template-only symbol without callable shape should fail");

        assert!(err.contains("TODO(leaf22.7)"));
        assert!(
            err.contains("template-only export without indexed callable signatures is unsupported")
        );
        assert!(err.contains("symbol `sort`"));
        assert!(err.contains("/tmp/cpp-index.toml"));
    }

    #[test]
    fn test_cpp_module_macro_usage_errors_as_unsupported_surface() {
        let mut modules = BTreeMap::new();
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols: BTreeMap::new(),
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            cpp_module_symbol_index_sources: vec![PathBuf::from("/tmp/cpp-index.toml")],
            ..TranspileOptions::default()
        };

        let err = transpile_full_with_options(
            r#"
use cpp::std as cpp_std;
fn f() -> i32 {
    unsafe {
        let _ = cpp_std::max!(1, 2);
    }
    0
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("cpp macro usage should fail under MVP limits");

        assert!(err.contains("TODO(leaf22.7)"));
        assert!(err.contains("`cpp::` macro imports are unsupported in MVP"));
        assert!(err.contains("symbol `max`"));
        assert!(err.contains("/tmp/cpp-index.toml"));
    }
}
