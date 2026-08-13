//! Parser, validated IR, and semantic lowering for the deliberately narrow
//! source-owned C++ ABI adapters. Callers get either a complete, closed
//! contract and emission plan or an error, never partially accepted C++
//! syntax.
//!
//! Macro expansion can assemble an attribute from disconnected token
//! fragments, so exact marker provenance is undecidable before expansion.
//! Consequently `cpp_abi` and `cpp_abi_alias` are reserved identifiers
//! anywhere in opaque macro definitions or invocations.  This intentional
//! fail-closed language restriction does not match string literals or longer
//! identifiers.

use quote::ToTokens;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use syn::ext::IdentExt;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::visit::Visit;
use syn::visit_mut::VisitMut;
use syn::{Attribute, Expr, FnArg, GenericArgument, Item, Meta, PathArguments, Token, Type};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ModulePath(pub(crate) Vec<String>);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum CallableKey {
    Free {
        module: ModulePath,
        name: String,
    },
    InherentStatic {
        module: ModulePath,
        owner: String,
        name: String,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum ParamAdapter {
    StdStringBytes,
    ConstRef { alias: String, element: Type },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReturnAdapter {
    StdStringBytes,
}

#[derive(Clone, Debug)]
pub(crate) struct CallableContract {
    pub(crate) key: CallableKey,
    pub(crate) params: BTreeMap<String, ParamAdapter>,
    pub(crate) returns: Option<ReturnAdapter>,
}

#[derive(Clone, Debug)]
pub(crate) struct VectorAliasContract {
    pub(crate) module: ModulePath,
    pub(crate) name: String,
    pub(crate) element: Type,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CppAbiContracts {
    pub(crate) aliases: BTreeMap<(ModulePath, String), VectorAliasContract>,
    pub(crate) callables: BTreeMap<CallableKey, CallableContract>,
    flat_imports: BTreeMap<FlatImportKey, FlatImportContract>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct FlatImportKey {
    module: ModulePath,
    rust_child: String,
    leaves: Vec<String>,
}

#[derive(Clone, Debug)]
struct FlatImportContract {
    key: FlatImportKey,
    cpp_namespace: String,
}

/// Out-of-band instructions consumed by code generation after the semantic
/// Rust AST has been lowered.  Keeping the ABI facade out of the Rust type
/// system is deliberate: `Vec<u8>` remains `Vec<u8>` for rustc and for every
/// ordinary rusty-cpp expression, while only the explicitly marked public
/// declaration is rendered with its legacy STL spelling.
#[derive(Clone, Debug, Default)]
pub(crate) struct CppAbiEmissionPlan {
    pub(crate) aliases: BTreeMap<(ModulePath, String), VectorAliasContract>,
    pub(crate) facades: BTreeMap<CallableKey, CallableFacade>,
    pub(crate) semantic_helpers: BTreeMap<(ModulePath, String), CallableKey>,
    flat_imports: BTreeMap<FlatImportKey, FlatImportContract>,
    emit_string_support: bool,
    emit_vector_support: bool,
    inline_identity: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct CallableFacade {
    pub(crate) contract: CallableContract,
    pub(crate) helper_name: String,
}

impl CppAbiEmissionPlan {
    pub(crate) fn is_empty(&self) -> bool {
        self.facades.is_empty() && self.flat_imports.is_empty()
    }

    pub(crate) fn has_flat_imports(&self) -> bool {
        !self.flat_imports.is_empty()
    }

    pub(crate) fn facade(&self, key: &CallableKey) -> Option<&CallableFacade> {
        self.facades.get(key)
    }

    pub(crate) fn free_facade(&self, module: &[String], name: &str) -> Option<&CallableFacade> {
        self.facade(&CallableKey::Free {
            module: ModulePath(module.iter().map(|part| canonical_name(part)).collect()),
            name: canonical_name(name),
        })
    }

    pub(crate) fn method_facade(
        &self,
        module: &[String],
        owner: &str,
        name: &str,
    ) -> Option<&CallableFacade> {
        self.facade(&CallableKey::InherentStatic {
            module: ModulePath(module.iter().map(|part| canonical_name(part)).collect()),
            owner: canonical_name(owner.rsplit("::").next().unwrap_or(owner)),
            name: canonical_name(name),
        })
    }

    pub(crate) fn alias(&self, module: &[String], name: &str) -> Option<&VectorAliasContract> {
        self.aliases.get(&(
            ModulePath(module.iter().map(|part| canonical_name(part)).collect()),
            canonical_name(name),
        ))
    }

    pub(crate) fn is_semantic_helper(&self, module: &[String], name: &str) -> bool {
        self.semantic_helpers.contains_key(&(
            ModulePath(module.iter().map(|part| canonical_name(part)).collect()),
            canonical_name(name),
        ))
    }

    pub(crate) fn needs_string_adapter(&self) -> bool {
        self.emit_string_support
    }

    pub(crate) fn needs_vector_adapter(&self) -> bool {
        self.emit_vector_support
    }

    pub(crate) fn detail_namespace(&self) -> String {
        self.inline_identity
            .as_ref()
            .map(|identity| format!("rusty_cpp_abi_detail_{identity}"))
            .unwrap_or_else(|| "rusty_cpp_abi_detail".to_string())
    }

    pub(crate) fn flat_import_for_use(
        &self,
        module: &[String],
        item: &syn::ItemUse,
    ) -> Option<(&str, &str, &[String])> {
        let parsed = parse_flat_import_use(item, &ModulePath(
            module.iter().map(|part| canonical_name(part)).collect(),
        ))
        .ok()
        .flatten()?;
        let contract = self.flat_imports.get(&parsed.key)?;
        (contract.cpp_namespace == parsed.cpp_namespace).then_some((
            contract.cpp_namespace.as_str(),
            contract.key.rust_child.as_str(),
            contract.key.leaves.as_slice(),
        ))
    }

    pub(crate) fn validate_flat_import_namespace(
        &self,
        expected: Option<&str>,
        context: &str,
    ) -> Result<(), String> {
        validate_flat_import_namespaces(&self.flat_imports, expected, context)
    }

    fn derive_support_requirements(&mut self) {
        self.emit_string_support = self.facades.values().any(|facade| {
            facade.contract.returns.is_some()
                || facade
                    .contract
                    .params
                    .values()
                    .any(|adapter| matches!(adapter, ParamAdapter::StdStringBytes))
        });
        self.emit_vector_support = !self.aliases.is_empty();
    }
}

fn validate_flat_import_namespaces(
    imports: &BTreeMap<FlatImportKey, FlatImportContract>,
    expected: Option<&str>,
    context: &str,
) -> Result<(), String> {
    if imports.is_empty() {
        return Ok(());
    }
    let expected = expected.ok_or_else(|| {
        format!("cpp_import_namespace requires an active C++ namespace in {context}")
    })?;
    let expected = canonical_cpp_namespace(expected).ok_or_else(|| {
        format!(
            "active C++ namespace `{expected}` in {context} is not a canonical namespace path"
        )
    })?;
    for contract in imports.values() {
        if contract.cpp_namespace != expected {
            return Err(format!(
                "cpp_import_namespace `{}` does not match active C++ namespace `{expected}` in {context}",
                contract.cpp_namespace
            ));
        }
    }
    Ok(())
}

fn canonical_name(name: &str) -> String {
    name.strip_prefix("r#").unwrap_or(name).to_string()
}

#[derive(Clone, Debug)]
enum ParsedMarker {
    Callable(ParsedCallable),
    VectorAlias,
}

#[derive(Clone, Debug, Default)]
struct ParsedCallable {
    params: BTreeMap<String, ParsedParamAdapter>,
    returns: Option<ReturnAdapter>,
}

#[derive(Clone, Debug)]
enum ParsedParamAdapter {
    StdStringBytes,
    ConstRef { alias: String },
}

fn canonical_cpp_namespace(source: &str) -> Option<String> {
    let path = syn::parse_str::<syn::Path>(source).ok()?;
    canonical_cpp_namespace_path(&path)
}

fn canonical_cpp_namespace_path(path: &syn::Path) -> Option<String> {
    if path.leading_colon.is_some() || path.segments.is_empty() {
        return None;
    }
    let mut parts = Vec::with_capacity(path.segments.len());
    for segment in &path.segments {
        if !matches!(segment.arguments, PathArguments::None) {
            return None;
        }
        parts.push(exact_cpp_identifier(&segment.ident)?);
    }
    Some(parts.join("::"))
}

fn exact_cpp_identifier(ident: &proc_macro2::Ident) -> Option<String> {
    let spelling = ident.to_string();
    if spelling.starts_with("r#") || crate::codegen::escape_cpp_keyword(&spelling) != spelling {
        return None;
    }
    Some(spelling)
}

fn parse_flat_import_marker_attr(attr: &Attribute) -> Result<Option<String>, String> {
    let mentions_marker = attribute_mentions_flat_import_marker(attr);
    if !mentions_marker {
        return Ok(None);
    }
    if path_mentions_flat_import_marker(attr.path()) {
        return Err(
            "cpp_import_namespace must use the inert form #[cfg_attr(any(), cpp_import_namespace(...))]"
                .to_string(),
        );
    }
    if !is_simple_path(attr.path(), "cfg_attr") {
        return Err(
            "cpp_import_namespace marker must be the sole payload of #[cfg_attr(any(), ...)]"
                .to_string(),
        );
    }
    let Meta::List(cfg_attr) = &attr.meta else {
        return Err("cpp_import_namespace requires list-form cfg_attr".to_string());
    };
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let metas = parser
        .parse2(cfg_attr.tokens.clone())
        .map_err(|error| format!("malformed cpp_import_namespace cfg_attr: {error}"))?;
    if metas.len() != 2
        || meta_mentions_flat_import_marker(&metas[0])
        || !meta_mentions_flat_import_marker(&metas[1])
    {
        return Err(
            "cpp_import_namespace marker must be the sole payload of #[cfg_attr(any(), ...)]"
                .to_string(),
        );
    }
    let Meta::List(predicate) = &metas[0] else {
        return Err("cpp_import_namespace cfg_attr predicate must be exactly any()".to_string());
    };
    if !is_simple_path(&predicate.path, "any") || !predicate.tokens.is_empty() {
        return Err("cpp_import_namespace cfg_attr predicate must be exactly any()".to_string());
    }
    let Meta::List(marker) = &metas[1] else {
        return Err("cpp_import_namespace requires one parenthesized namespace path".to_string());
    };
    if !is_simple_path(&marker.path, "cpp_import_namespace") {
        return Err(
            "cpp_import_namespace marker path must be the exact unqualified identifier `cpp_import_namespace`"
                .to_string(),
        );
    }
    let namespace = syn::parse2::<syn::Path>(marker.tokens.clone())
        .ok()
        .and_then(|path| canonical_cpp_namespace_path(&path))
        .ok_or_else(|| {
            "cpp_import_namespace requires exactly one canonical namespace path".to_string()
        })?;
    Ok(Some(namespace))
}

fn parse_flat_import_use(
    item: &syn::ItemUse,
    module: &ModulePath,
) -> Result<Option<FlatImportContract>, String> {
    let mut namespace = None;
    for attr in &item.attrs {
        if let Some(parsed) = parse_flat_import_marker_attr(attr)? {
            if namespace.replace(parsed).is_some() {
                return Err("duplicate cpp_import_namespace marker attributes".to_string());
            }
        }
    }
    let Some(cpp_namespace) = namespace else {
        return Ok(None);
    };
    if item.attrs.len() != 1 {
        return Err(
            "cpp_import_namespace use items support only the namespace marker attribute"
                .to_string(),
        );
    }
    if !matches!(item.vis, syn::Visibility::Inherited) {
        return Err("cpp_import_namespace requires a private use item".to_string());
    }
    if item.leading_colon.is_some() {
        return Err(
            "cpp_import_namespace requires the exact path crate::<child>::<Name leaves>"
                .to_string(),
        );
    }
    let syn::UseTree::Path(crate_root) = &item.tree else {
        return Err(
            "cpp_import_namespace requires the exact path crate::<child>::<Name leaves>"
                .to_string(),
        );
    };
    if ident_key(&crate_root.ident) != "crate" {
        return Err(
            "cpp_import_namespace requires the exact path crate::<child>::<Name leaves>"
                .to_string(),
        );
    }
    let syn::UseTree::Path(child) = crate_root.tree.as_ref() else {
        return Err(
            "cpp_import_namespace requires exactly one crate child module"
                .to_string(),
        );
    };
    let rust_child = exact_cpp_identifier(&child.ident).ok_or_else(|| {
        "cpp_import_namespace crate child must already be an exact C++ identifier".to_string()
    })?;
    if matches!(rust_child.as_str(), "crate" | "self" | "super") {
        return Err("cpp_import_namespace requires a named crate child module".to_string());
    }
    let mut leaves = Vec::new();
    match child.tree.as_ref() {
        syn::UseTree::Name(name) if ident_key(&name.ident) != "self" => {
            leaves.push(exact_cpp_identifier(&name.ident).ok_or_else(|| {
                "cpp_import_namespace leaves must already be exact C++ identifiers".to_string()
            })?);
        }
        syn::UseTree::Group(group) if !group.items.is_empty() => {
            for leaf in &group.items {
                let syn::UseTree::Name(name) = leaf else {
                    return Err(
                        "cpp_import_namespace accepts only simple Name leaves (no glob, rename, self, or nested path)"
                            .to_string(),
                    );
                };
                let name = exact_cpp_identifier(&name.ident).ok_or_else(|| {
                    "cpp_import_namespace leaves must already be exact C++ identifiers"
                        .to_string()
                })?;
                if name == "self" {
                    return Err(
                        "cpp_import_namespace accepts only simple Name leaves (no glob, rename, self, or nested path)"
                            .to_string(),
                    );
                }
                leaves.push(name);
            }
        }
        _ => {
            return Err(
                "cpp_import_namespace accepts only simple Name leaves (no glob, rename, self, or nested path)"
                    .to_string(),
            );
        }
    }
    let mut unique = BTreeSet::new();
    if leaves.iter().any(|leaf| !unique.insert(leaf.clone())) {
        return Err("cpp_import_namespace rejects duplicate imported leaves".to_string());
    }
    let key = FlatImportKey {
        module: module.clone(),
        rust_child,
        leaves,
    };
    Ok(Some(FlatImportContract {
        key,
        cpp_namespace,
    }))
}

/// Collect and validate all ABI contracts in a Rust file.
///
/// The only recognized spellings are inert stable-Rust attributes of the form
/// `#[cfg_attr(any(), cpp_abi(...))]` and
/// `#[cfg_attr(any(), cpp_abi_alias(std_vector))]`.
/// Exact marker identifiers are reserved in all opaque macro tokens because a
/// macro can construct either attribute from otherwise disconnected tokens.
pub(crate) fn collect(file: &syn::File) -> Result<CppAbiContracts, String> {
    reject_marker_attrs(&file.attrs, "crate-level inner attribute")?;
    let mut contracts = CppAbiContracts::default();
    collect_module(&file.items, &ModulePath(Vec::new()), &mut contracts)?;
    Ok(contracts)
}

fn collect_module(
    items: &[Item],
    module: &ModulePath,
    contracts: &mut CppAbiContracts,
) -> Result<(), String> {
    let mut ordinary_alias_names = BTreeSet::new();

    // Aliases are collected first so callables may refer to a later alias.
    for item in items {
        if let Item::Type(alias) = item {
            reject_flat_import_marker_attrs(&alias.attrs, "type alias")?;
            reject_markers_in_type_alias_descendants(alias)?;
            ordinary_alias_names.insert(ident_key(&alias.ident));
            if let Some(marker) = parse_single_marker(&alias.attrs)? {
                let ParsedMarker::VectorAlias = marker else {
                    return Err(format!(
                        "cpp_abi is only valid on a callable; `{}` is a type alias",
                        alias.ident
                    ));
                };
                validate_marker_companion_attrs(&alias.attrs)?;
                if !is_public(&alias.vis) || !alias.generics.params.is_empty() {
                    return Err(format!(
                        "cpp_abi_alias `{}` must be a non-generic pub type alias",
                        alias.ident
                    ));
                }
                let element = exact_vec_element(&alias.ty).ok_or_else(|| {
                    format!(
                        "cpp_abi_alias `{}` must have the exact form `pub type {} = Vec<T>`",
                        alias.ident, alias.ident
                    )
                })?;
                let alias_name = ident_key(&alias.ident);
                let key = (module.clone(), alias_name.clone());
                if contracts.aliases.contains_key(&key) {
                    return Err(format!("duplicate cpp_abi_alias `{}`", alias.ident));
                }
                contracts.aliases.insert(
                    key,
                    VectorAliasContract {
                        module: module.clone(),
                        name: alias_name,
                        element: element.clone(),
                    },
                );
            }
        }
    }

    let mut used_aliases = BTreeSet::new();
    for item in items {
        match item {
            Item::Fn(function) => {
                reject_flat_import_marker_attrs(&function.attrs, "free function")?;
                reject_markers_in_signature(&function.sig, "free-function signature")?;
                reject_markers_in_block(&function.block, "local function item")?;
                if let Some(marker) = parse_single_marker(&function.attrs)? {
                    let ParsedMarker::Callable(parsed) = marker else {
                        return Err(format!(
                            "cpp_abi_alias is only valid on a type alias; `{}` is a function",
                            function.sig.ident
                        ));
                    };
                    validate_marker_companion_attrs(&function.attrs)?;
                    let key = CallableKey::Free {
                        module: module.clone(),
                        name: ident_key(&function.sig.ident),
                    };
                    let contract = validate_callable(
                        key.clone(),
                        &function.vis,
                        &function.sig,
                        &parsed,
                        module,
                        contracts,
                        &ordinary_alias_names,
                        &mut used_aliases,
                    )?;
                    insert_callable(contracts, key, contract)?;
                }
            }
            Item::Impl(implementation) => {
                reject_marker_attrs(&implementation.attrs, "impl block")?;
                reject_markers_in_impl_header(implementation)?;
                let owner = simple_impl_owner(implementation);
                for impl_item in &implementation.items {
                    if let syn::ImplItem::Fn(method) = impl_item {
                        reject_flat_import_marker_attrs(&method.attrs, "method")?;
                        reject_markers_in_signature(&method.sig, "method signature")?;
                        reject_markers_in_block(&method.block, "local method item")?;
                        if let Some(marker) = parse_single_marker(&method.attrs)? {
                            let owner = owner.as_ref().map_err(|_| {
                                "cpp_abi methods require an inherent impl of a simple, non-generic owner"
                                    .to_string()
                            })?.clone();
                            let ParsedMarker::Callable(parsed) = marker else {
                                return Err(format!(
                                    "cpp_abi_alias is only valid on a type alias; `{}::{}` is a method",
                                    owner, method.sig.ident
                                ));
                            };
                            validate_marker_companion_attrs(&method.attrs)?;
                            let key = CallableKey::InherentStatic {
                                module: module.clone(),
                                owner,
                                name: ident_key(&method.sig.ident),
                            };
                            let contract = validate_callable(
                                key.clone(),
                                &method.vis,
                                &method.sig,
                                &parsed,
                                module,
                                contracts,
                                &ordinary_alias_names,
                                &mut used_aliases,
                            )?;
                            insert_callable(contracts, key, contract)?;
                        }
                    } else {
                        reject_markers_in_impl_item(impl_item, "unsupported impl item")?;
                    }
                }
            }
            Item::Mod(item_mod) => {
                reject_marker_attrs(&item_mod.attrs, "module")?;
            }
            Item::Type(alias) => {
                // Parsed in the alias-first pass. Rejecting the wrong marker there
                // gives the more precise diagnostic.
                let _ = parse_single_marker(&alias.attrs)?;
            }
            Item::Use(item_use) => {
                if let Some(contract) = parse_flat_import_use(item_use, module)? {
                    if contracts
                        .flat_imports
                        .insert(contract.key.clone(), contract)
                        .is_some()
                    {
                        return Err(format!(
                            "duplicate cpp_import_namespace use `{}`",
                            item_use.to_token_stream()
                        ));
                    }
                } else {
                    reject_markers_in_item(item, "unmarked use item")?;
                }
            }
            other => reject_markers_in_item(other, "unsupported item")?,
        }
    }

    for ((alias_module, alias_name), _) in contracts.aliases.iter() {
        if alias_module == module && !used_aliases.contains(alias_name) {
            return Err(format!(
                "cpp_abi_alias `{}` is not consumed by a same-module const_ref adapter",
                alias_name
            ));
        }
    }

    for item in items {
        if let Item::Mod(item_mod) = item
            && let Some((_, nested)) = &item_mod.content
        {
            let mut nested_path = module.0.clone();
            nested_path.push(ident_key(&item_mod.ident));
            collect_module(nested, &ModulePath(nested_path), contracts)?;
        }
    }
    Ok(())
}

fn insert_callable(
    contracts: &mut CppAbiContracts,
    key: CallableKey,
    contract: CallableContract,
) -> Result<(), String> {
    if contracts.callables.insert(key.clone(), contract).is_some() {
        return Err(format!("duplicate cpp_abi callable `{key:?}`"));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_callable(
    key: CallableKey,
    vis: &syn::Visibility,
    sig: &syn::Signature,
    parsed: &ParsedCallable,
    module: &ModulePath,
    contracts: &CppAbiContracts,
    ordinary_alias_names: &BTreeSet<String>,
    used_aliases: &mut BTreeSet<String>,
) -> Result<CallableContract, String> {
    if !is_public(vis)
        || sig.constness.is_some()
        || sig.asyncness.is_some()
        || sig.unsafety.is_some()
        || sig.abi.is_some()
        || sig.variadic.is_some()
        || !sig.generics.params.is_empty()
        || sig.generics.where_clause.is_some()
    {
        return Err(format!(
            "cpp_abi callable `{}` must be pub, safe, non-const, non-async, non-extern, and non-generic",
            sig.ident
        ));
    }

    let mut parameter_types = BTreeMap::<String, &Type>::new();
    for arg in &sig.inputs {
        let FnArg::Typed(arg) = arg else {
            return Err(format!(
                "cpp_abi callable `{}` must be static and cannot have a receiver",
                sig.ident
            ));
        };
        if !arg.attrs.is_empty() {
            return Err(format!(
                "cpp_abi callable `{}` does not support parameter attributes",
                sig.ident
            ));
        }
        let syn::Pat::Ident(pattern) = arg.pat.as_ref() else {
            return Err(format!(
                "cpp_abi callable `{}` requires simple identifier parameters",
                sig.ident
            ));
        };
        if pattern.by_ref.is_some() || pattern.mutability.is_some() || pattern.subpat.is_some() {
            return Err(format!(
                "cpp_abi callable `{}` requires unmodified identifier parameters",
                sig.ident
            ));
        }
        parameter_types.insert(ident_key(&pattern.ident), arg.ty.as_ref());
    }

    let mut params = BTreeMap::new();
    for (name, adapter) in &parsed.params {
        let ty = parameter_types.get(name).ok_or_else(|| {
            format!(
                "cpp_abi param `{name}` does not name a parameter of `{}`",
                sig.ident
            )
        })?;
        let validated = match adapter {
            ParsedParamAdapter::StdStringBytes => {
                if !is_exact_vec_u8(ty) {
                    return Err(format!(
                        "cpp_abi param `{name}` with std_string_bytes must be exact by-value Vec<u8>"
                    ));
                }
                ParamAdapter::StdStringBytes
            }
            ParsedParamAdapter::ConstRef { alias } => {
                let slice_element = exact_immutable_slice_element(ty).ok_or_else(|| {
                    format!("cpp_abi param `{name}` with const_ref must have the exact type `&[T]`")
                })?;
                let alias_key = (module.clone(), alias.clone());
                let alias_contract = contracts.aliases.get(&alias_key).ok_or_else(|| {
                    if ordinary_alias_names.contains(alias) {
                        format!(
                            "cpp_abi const_ref alias `{alias}` must carry #[cfg_attr(any(), cpp_abi_alias(std_vector))]"
                        )
                    } else {
                        format!(
                            "cpp_abi const_ref alias `{alias}` must be a marked pub Vec<T> alias in the same module"
                        )
                    }
                })?;
                if type_text(slice_element) != type_text(&alias_contract.element) {
                    return Err(format!(
                        "cpp_abi const_ref alias `{alias}` element type does not match parameter `{name}`"
                    ));
                }
                used_aliases.insert(alias.clone());
                ParamAdapter::ConstRef {
                    alias: alias.clone(),
                    element: slice_element.clone(),
                }
            }
        };
        params.insert(name.clone(), validated);
    }

    if parsed.returns == Some(ReturnAdapter::StdStringBytes) {
        let syn::ReturnType::Type(_, ty) = &sig.output else {
            return Err(format!(
                "cpp_abi returns(std_string_bytes) on `{}` requires exact Vec<u8> return type",
                sig.ident
            ));
        };
        if !is_exact_vec_u8(ty) {
            return Err(format!(
                "cpp_abi returns(std_string_bytes) on `{}` requires exact Vec<u8> return type",
                sig.ident
            ));
        }
    }

    Ok(CallableContract {
        key,
        params,
        returns: parsed.returns.clone(),
    })
}

fn parse_single_marker(attrs: &[Attribute]) -> Result<Option<ParsedMarker>, String> {
    let mut result = None;
    for attr in attrs {
        let marker = parse_marker_attr(attr)?;
        if let Some(marker) = marker {
            if result.is_some() {
                return Err("duplicate cpp_abi/cpp_abi_alias marker attributes".to_string());
            }
            result = Some(marker);
        }
    }
    Ok(result)
}

fn parse_marker_attr(attr: &Attribute) -> Result<Option<ParsedMarker>, String> {
    let mentions_marker = attribute_mentions_marker(attr);
    if !mentions_marker {
        return Ok(None);
    }
    if path_mentions_marker(attr.path()) {
        return Err("cpp_abi markers must use the inert form #[cfg_attr(any(), ...)]".to_string());
    }
    if !is_simple_path(attr.path(), "cfg_attr") {
        return Err(
            "cpp_abi marker must be the sole payload of #[cfg_attr(any(), ...)]".to_string(),
        );
    }

    let Meta::List(cfg_attr) = &attr.meta else {
        return Err("cpp_abi marker requires list-form cfg_attr".to_string());
    };
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let metas = match parser.parse2(cfg_attr.tokens.clone()) {
        Ok(metas) => metas,
        Err(error) => {
            return Err(format!("malformed cpp_abi cfg_attr: {error}"));
        }
    };
    if metas.len() != 2 || meta_mentions_marker(&metas[0]) || !meta_mentions_marker(&metas[1]) {
        return Err(
            "cpp_abi marker must be the sole payload of #[cfg_attr(any(), ...)]".to_string(),
        );
    }
    let predicate = &metas[0];
    let Meta::List(predicate) = predicate else {
        return Err("cpp_abi cfg_attr predicate must be exactly any()".to_string());
    };
    if !is_simple_path(&predicate.path, "any") || !predicate.tokens.is_empty() {
        return Err("cpp_abi cfg_attr predicate must be exactly any()".to_string());
    }

    let Meta::List(marker) = &metas[1] else {
        return Err("cpp_abi marker requires a parenthesized adapter list".to_string());
    };
    if is_simple_path(&marker.path, "cpp_abi") {
        parse_callable_marker(&marker.tokens)
            .map(ParsedMarker::Callable)
            .map(Some)
    } else if is_simple_path(&marker.path, "cpp_abi_alias") {
        let parser = Punctuated::<syn::Path, Token![,]>::parse_terminated;
        let args = parser
            .parse2(marker.tokens.clone())
            .map_err(|error| format!("malformed cpp_abi_alias: {error}"))?;
        if args.len() != 1 || !is_simple_path(&args[0], "std_vector") {
            return Err("cpp_abi_alias accepts only `std_vector`".to_string());
        }
        Ok(Some(ParsedMarker::VectorAlias))
    } else {
        Err(
            "cpp_abi marker path must be the exact unqualified identifier `cpp_abi` or `cpp_abi_alias`"
                .to_string(),
        )
    }
}

fn parse_callable_marker(tokens: &proc_macro2::TokenStream) -> Result<ParsedCallable, String> {
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let clauses = parser
        .parse2(tokens.clone())
        .map_err(|error| format!("malformed cpp_abi adapter list: {error}"))?;
    if clauses.is_empty() {
        return Err("cpp_abi requires at least one adapter clause".to_string());
    }
    let mut parsed = ParsedCallable::default();
    for clause in clauses {
        let Meta::List(clause) = clause else {
            return Err("cpp_abi clauses must be parenthesized".to_string());
        };
        if clause.path.is_ident("param") {
            let parser = Punctuated::<Expr, Token![,]>::parse_terminated;
            let args = parser
                .parse2(clause.tokens)
                .map_err(|error| format!("malformed cpp_abi param clause: {error}"))?;
            if args.len() != 2 {
                return Err("cpp_abi param requires exactly a name and adapter".to_string());
            }
            let name = expr_simple_ident_key(&args[0]).ok_or_else(|| {
                "cpp_abi param name must be a single unqualified identifier".to_string()
            })?;
            let adapter = parse_param_adapter(&args[1])?;
            if parsed.params.insert(name.clone(), adapter).is_some() {
                return Err(format!("duplicate cpp_abi adapter for param `{name}`"));
            }
        } else if clause.path.is_ident("returns") {
            let parser = Punctuated::<Expr, Token![,]>::parse_terminated;
            let args = parser
                .parse2(clause.tokens)
                .map_err(|error| format!("malformed cpp_abi returns clause: {error}"))?;
            if args.len() != 1
                || expr_simple_ident_spelling(&args[0]).as_deref() != Some("std_string_bytes")
            {
                return Err("cpp_abi returns accepts only `std_string_bytes`".to_string());
            }
            if parsed
                .returns
                .replace(ReturnAdapter::StdStringBytes)
                .is_some()
            {
                return Err("duplicate cpp_abi returns clause".to_string());
            }
        } else {
            return Err(format!(
                "unknown cpp_abi clause `{}`",
                clause.path.to_token_stream()
            ));
        }
    }
    Ok(parsed)
}

fn parse_param_adapter(expr: &Expr) -> Result<ParsedParamAdapter, String> {
    if expr_simple_ident_spelling(expr).as_deref() == Some("std_string_bytes") {
        return Ok(ParsedParamAdapter::StdStringBytes);
    }
    if let Expr::Call(call) = expr
        && expr_simple_ident_spelling(&call.func).as_deref() == Some("const_ref")
        && call.args.len() == 1
        && let Some(alias) = expr_simple_ident_key(&call.args[0])
    {
        return Ok(ParsedParamAdapter::ConstRef { alias });
    }
    Err("cpp_abi param adapter must be `std_string_bytes` or `const_ref(Alias)`".to_string())
}

fn validate_marker_companion_attrs(attrs: &[Attribute]) -> Result<(), String> {
    for attr in attrs {
        if parse_marker_attr(attr)?.is_some() || attr.path().is_ident("doc") {
            continue;
        }
        return Err(format!(
            "cpp_abi items support only doc attributes in addition to the ABI marker; found `{}`",
            attr.path().to_token_stream()
        ));
    }
    Ok(())
}

fn reject_marker_attrs(attrs: &[Attribute], context: &str) -> Result<(), String> {
    for attr in attrs {
        if parse_marker_attr(attr)?.is_some() || parse_flat_import_marker_attr(attr)?.is_some() {
            return Err(format!("cpp_abi marker is not supported on {context}"));
        }
    }
    Ok(())
}

fn reject_flat_import_marker_attrs(
    attrs: &[Attribute],
    context: &str,
) -> Result<(), String> {
    for attr in attrs {
        if parse_flat_import_marker_attr(attr)?.is_some() {
            return Err(format!(
                "cpp_import_namespace marker is only supported on a private use item; found on {context}"
            ));
        }
    }
    Ok(())
}

#[derive(Default)]
struct AttributeCollector<'ast> {
    attrs: Vec<&'ast Attribute>,
    opaque_macro_mentions_marker: bool,
}

impl<'ast> Visit<'ast> for AttributeCollector<'ast> {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        self.attrs.push(attr);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if token_stream_mentions_reserved_marker(mac.tokens.clone()) {
            self.opaque_macro_mentions_marker = true;
        }
        syn::visit::visit_macro(self, mac);
    }
}

fn reject_collected_markers(
    collector: AttributeCollector<'_>,
    context: &str,
) -> Result<(), String> {
    for attr in collector.attrs {
        if parse_marker_attr(attr)?.is_some() || parse_flat_import_marker_attr(attr)?.is_some() {
            return Err(format!("cpp_abi marker is not supported on {context}"));
        }
    }
    if collector.opaque_macro_mentions_marker {
        return Err(format!(
            "reserved cpp_abi/cpp_abi_alias/cpp_import_namespace identifier inside opaque macro tokens on {context}"
        ));
    }
    Ok(())
}

fn reject_markers_in_block(block: &syn::Block, context: &str) -> Result<(), String> {
    let mut collector = AttributeCollector::default();
    collector.visit_block(block);
    reject_collected_markers(collector, context)
}

fn reject_markers_in_signature(sig: &syn::Signature, context: &str) -> Result<(), String> {
    let mut collector = AttributeCollector::default();
    collector.visit_signature(sig);
    reject_collected_markers(collector, context)
}

fn reject_markers_in_type_alias_descendants(alias: &syn::ItemType) -> Result<(), String> {
    let mut collector = AttributeCollector::default();
    collector.visit_generics(&alias.generics);
    collector.visit_type(&alias.ty);
    reject_collected_markers(collector, "type-alias descendants")
}

fn reject_markers_in_impl_header(implementation: &syn::ItemImpl) -> Result<(), String> {
    let mut collector = AttributeCollector::default();
    collector.visit_generics(&implementation.generics);
    if let Some((_, trait_path, _)) = &implementation.trait_ {
        collector.visit_path(trait_path);
    }
    collector.visit_type(&implementation.self_ty);
    reject_collected_markers(collector, "impl header")
}

fn reject_markers_in_impl_item(item: &syn::ImplItem, context: &str) -> Result<(), String> {
    let mut collector = AttributeCollector::default();
    collector.visit_impl_item(item);
    reject_collected_markers(collector, context)
}

fn reject_markers_in_item(item: &Item, context: &str) -> Result<(), String> {
    let mut collector = AttributeCollector::default();
    collector.visit_item(item);
    reject_collected_markers(collector, context)
}

fn is_public(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

fn marker_ident(ident: &proc_macro2::Ident) -> bool {
    let spelling = ident.to_string();
    let semantic = spelling.strip_prefix("r#").unwrap_or(&spelling);
    matches!(semantic, "cpp_abi" | "cpp_abi_alias")
}

fn flat_import_marker_ident(ident: &proc_macro2::Ident) -> bool {
    let spelling = ident.to_string();
    spelling.strip_prefix("r#").unwrap_or(&spelling) == "cpp_import_namespace"
}

fn reserved_marker_ident(ident: &proc_macro2::Ident) -> bool {
    marker_ident(ident) || flat_import_marker_ident(ident)
}

fn ident_key(ident: &proc_macro2::Ident) -> String {
    ident.unraw().to_string()
}

fn path_mentions_marker(path: &syn::Path) -> bool {
    path.segments
        .iter()
        .any(|segment| marker_ident(&segment.ident))
}

fn token_stream_mentions_marker(tokens: proc_macro2::TokenStream) -> bool {
    tokens.into_iter().any(|token| match token {
        proc_macro2::TokenTree::Ident(ident) => marker_ident(&ident),
        proc_macro2::TokenTree::Group(group) => token_stream_mentions_marker(group.stream()),
        proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => false,
    })
}

fn path_mentions_flat_import_marker(path: &syn::Path) -> bool {
    path.segments
        .iter()
        .any(|segment| flat_import_marker_ident(&segment.ident))
}

fn token_stream_mentions_flat_import_marker(tokens: proc_macro2::TokenStream) -> bool {
    tokens.into_iter().any(|token| match token {
        proc_macro2::TokenTree::Ident(ident) => flat_import_marker_ident(&ident),
        proc_macro2::TokenTree::Group(group) => {
            token_stream_mentions_flat_import_marker(group.stream())
        }
        proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => false,
    })
}

fn token_stream_mentions_reserved_marker(tokens: proc_macro2::TokenStream) -> bool {
    tokens.into_iter().any(|token| match token {
        proc_macro2::TokenTree::Ident(ident) => reserved_marker_ident(&ident),
        proc_macro2::TokenTree::Group(group) => {
            token_stream_mentions_reserved_marker(group.stream())
        }
        proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => false,
    })
}

pub(crate) fn source_mentions_reserved_marker(source: &str) -> bool {
    match syn::parse_file(source) {
        Ok(file) => match collect(&file) {
            Ok(contracts) => {
                !contracts.callables.is_empty()
                    || !contracts.aliases.is_empty()
                    || !contracts.flat_imports.is_empty()
            }
            // `collect` errors only after structurally finding a marker attempt
            // (including the documented opaque-macro reserved-identifier
            // policy), so an unsupported placement must still trip the target
            // layout guard.
            Err(_) => true,
        },
        // Invalid Rust will fail crate transpilation independently. Keep the
        // probe fail-closed only when a marker-shaped token is also present.
        Err(_) => source
            .parse::<proc_macro2::TokenStream>()
            .ok()
            .is_some_and(token_stream_mentions_reserved_marker),
    }
}

fn meta_mentions_marker(meta: &Meta) -> bool {
    path_mentions_marker(meta.path())
        || match meta {
            Meta::List(list) => {
                // Prefer structural recursion through nested meta items.  The
                // token-tree fallback is only for malformed list payloads so
                // an attempted marker cannot hide behind a parse error.  It
                // compares exact identifiers and never inspects literals.
                let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
                match parser.parse2(list.tokens.clone()) {
                    Ok(nested) => nested.iter().any(meta_mentions_marker),
                    Err(_) => token_stream_mentions_marker(list.tokens.clone()),
                }
            }
            Meta::Path(_) | Meta::NameValue(_) => false,
        }
}

fn attribute_mentions_marker(attr: &Attribute) -> bool {
    meta_mentions_marker(&attr.meta)
}

fn meta_mentions_flat_import_marker(meta: &Meta) -> bool {
    path_mentions_flat_import_marker(meta.path())
        || match meta {
            Meta::List(list) => {
                let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
                match parser.parse2(list.tokens.clone()) {
                    Ok(nested) => nested.iter().any(meta_mentions_flat_import_marker),
                    Err(_) => token_stream_mentions_flat_import_marker(list.tokens.clone()),
                }
            }
            Meta::Path(_) | Meta::NameValue(_) => false,
        }
}

fn attribute_mentions_flat_import_marker(attr: &Attribute) -> bool {
    meta_mentions_flat_import_marker(&attr.meta)
}

fn is_simple_path(path: &syn::Path, expected: &str) -> bool {
    path.leading_colon.is_none()
        && path.segments.len() == 1
        && path.segments[0].ident.to_string() == expected
}

fn expr_simple_ident_ref(expr: &Expr) -> Option<&proc_macro2::Ident> {
    let Expr::Path(path) = expr else {
        return None;
    };
    if path.qself.is_some()
        || path.path.leading_colon.is_some()
        || path.path.segments.len() != 1
        || !matches!(path.path.segments[0].arguments, PathArguments::None)
    {
        return None;
    }
    Some(&path.path.segments[0].ident)
}

fn expr_simple_ident_spelling(expr: &Expr) -> Option<String> {
    expr_simple_ident_ref(expr).map(ToString::to_string)
}

fn expr_simple_ident_key(expr: &Expr) -> Option<String> {
    expr_simple_ident_ref(expr).map(ident_key)
}

fn exact_vec_element(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    if path.qself.is_some() || path.path.leading_colon.is_some() || path.path.segments.len() != 1 {
        return None;
    }
    let segment = &path.path.segments[0];
    if segment.ident != "Vec" {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    if arguments.args.len() != 1 {
        return None;
    }
    match &arguments.args[0] {
        GenericArgument::Type(element) => Some(element),
        _ => None,
    }
}

fn is_exact_vec_u8(ty: &Type) -> bool {
    exact_vec_element(ty).is_some_and(|element| {
        matches!(element, Type::Path(path) if path.qself.is_none() && is_simple_path(&path.path, "u8"))
    })
}

fn exact_immutable_slice_element(ty: &Type) -> Option<&Type> {
    let Type::Reference(reference) = ty else {
        return None;
    };
    if reference.mutability.is_some() || reference.lifetime.is_some() {
        return None;
    }
    let Type::Slice(slice) = reference.elem.as_ref() else {
        return None;
    };
    Some(slice.elem.as_ref())
}

fn type_text(ty: &Type) -> String {
    ty.to_token_stream().to_string()
}

fn simple_impl_owner(implementation: &syn::ItemImpl) -> Result<String, String> {
    if implementation.trait_.is_some() || !implementation.generics.params.is_empty() {
        return Err(String::new());
    }
    let Type::Path(owner) = implementation.self_ty.as_ref() else {
        return Err(String::new());
    };
    if owner.qself.is_some()
        || owner.path.leading_colon.is_some()
        || owner.path.segments.len() != 1
        || !matches!(owner.path.segments[0].arguments, PathArguments::None)
    {
        return Err(String::new());
    }
    Ok(ident_key(&owner.path.segments[0].ident))
}

/// A single inline block after the carrier-wide ABI pass.  The emission plan
/// contains only facades physically provided by this block; `dependencies`
/// names earlier provider blocks whose semantic helpers this block calls.
#[derive(Clone, Debug)]
pub(crate) struct CppAbiInlineBlockPlan {
    pub(crate) lowered: syn::File,
    pub(crate) emission: CppAbiEmissionPlan,
    pub(crate) dependencies: BTreeSet<usize>,
}

#[derive(Clone, Debug)]
pub(crate) struct CppAbiInlineCarrierPlan {
    pub(crate) blocks: Vec<CppAbiInlineBlockPlan>,
    pub(crate) adapted_blocks: BTreeSet<usize>,
    pub(crate) flat_import_blocks: BTreeSet<usize>,
}

impl CppAbiInlineCarrierPlan {
    pub(crate) fn flat_import_requirements(
        &self,
        block: usize,
    ) -> Vec<(String, String, Vec<String>)> {
        self.blocks
            .get(block)
            .into_iter()
            .flat_map(|block| block.emission.flat_imports.values())
            .map(|contract| {
                (
                    contract.cpp_namespace.clone(),
                    contract.key.rust_child.clone(),
                    contract.key.leaves.clone(),
                )
            })
            .collect()
    }
}

fn joined_inline_file(files: &[syn::File]) -> syn::File {
    syn::File {
        shebang: None,
        attrs: Vec::new(),
        items: files
            .iter()
            .flat_map(|file| file.items.iter().cloned())
            .collect(),
    }
}

/// Collect the canonical callable/alias tails supplied by one carrier.  The
/// caller uses this cheap first pass to reserve names across all requested
/// carriers before any file is rendered or written.
pub(crate) fn inline_contract_names(files: &[syn::File]) -> Result<BTreeSet<String>, String> {
    let contracts = collect(&joined_inline_file(files))?;
    Ok(reserved_contract_names(&contracts))
}

pub(crate) fn inline_generated_helper_names(
    files: &[syn::File],
    inline_identity: &str,
) -> Result<BTreeSet<String>, String> {
    let contracts = collect(&joined_inline_file(files))?;
    Ok(contracts
        .callables
        .keys()
        .map(|key| inline_helper_stem(key, inline_identity))
        .collect())
}

pub(crate) fn inline_external_contract_indexes(
    carriers: &[Vec<syn::File>],
) -> Result<Vec<ExternalContractIndex>, String> {
    let mut global = GlobalContractIndex::default();
    for (index, files) in carriers.iter().enumerate() {
        global.add(
            index,
            &ModulePath(Vec::new()),
            &collect(&joined_inline_file(files))?,
        );
    }
    Ok((0..carriers.len())
        .map(|index| global.external_for(index))
        .collect())
}

/// Reject a public C++ spelling supplied by an adapter in one carrier when an
/// ordinary Rust item in another carrier projects to that same spelling.
/// Lexical bindings and items below a distinct Rust module are deliberately
/// outside this namespace/member census.
pub(crate) fn validate_inline_projected_cpp_name_collisions(
    sources: &[String],
    carriers: &[Vec<syn::File>],
) -> Result<(), String> {
    if sources.len() != carriers.len() {
        return Err("inline cpp_abi projected-name census input mismatch".to_string());
    }

    let mut census = ProjectedCppCensus::default();
    for (source, files) in sources.iter().zip(carriers) {
        let joined = joined_inline_file(files);
        let contracts = collect(&joined)?;
        collect_projected_cpp_names(
            &joined.items,
            &ModulePath(Vec::new()),
            &[],
            &contracts,
            source,
            None,
            None,
            &mut census,
        );
    }
    census.validate().map_err(|error| {
        format!("inline-rust cpp_abi projected public-name preflight failed: {error}")
    })
}

fn validate_inline_flat_import_block_items(
    items: &[Item],
    module: &ModulePath,
    block: usize,
    providers: &BTreeMap<(ModulePath, String), usize>,
    inherited_forbidden: &BTreeSet<String>,
) -> Result<(), String> {
    let current_leaves = providers
        .keys()
        .filter_map(|(owner, leaf)| (owner == module).then_some(leaf.clone()))
        .collect::<BTreeSet<_>>();
    let mut forbidden = inherited_forbidden.clone();
    forbidden.extend(providers.iter().filter_map(|((owner, leaf), provider)| {
        (owner == module && *provider != block).then_some(leaf.clone())
    }));

    for item in items {
        if let Item::Mod(item_mod) = item {
            if cpp_name_set_contains(&forbidden, &ident_key(&item_mod.ident))
                || item_mod.attrs.iter().any(|attr| {
                    token_stream_mentions_cpp_names(attr.meta.to_token_stream(), &forbidden)
                })
            {
                return Err(format!(
                    "cpp_import_namespace leaves [{}] may only be referenced or bound in their marked inline block and exact Rust module",
                    forbidden.iter().cloned().collect::<Vec<_>>().join(", ")
                ));
            }
            if let Some((_, nested)) = &item_mod.content {
                let mut nested_path = module.0.clone();
                nested_path.push(ident_key(&item_mod.ident));
                let mut nested_forbidden = inherited_forbidden.clone();
                nested_forbidden.extend(current_leaves.iter().cloned());
                validate_inline_flat_import_block_items(
                    nested,
                    &ModulePath(nested_path),
                    block,
                    providers,
                    &nested_forbidden,
                )?;
            }
        } else if token_stream_mentions_cpp_names(item.to_token_stream(), &forbidden) {
            return Err(format!(
                "cpp_import_namespace leaves [{}] may only be referenced or bound in their marked inline block and exact Rust module",
                forbidden.iter().cloned().collect::<Vec<_>>().join(", ")
            ));
        }
    }
    Ok(())
}

fn validate_inline_flat_import_block_locality(
    files: &[syn::File],
    locals: &[CppAbiContracts],
) -> Result<(), String> {
    let mut providers = BTreeMap::<(ModulePath, String), usize>::new();
    for (block, contracts) in locals.iter().enumerate() {
        for contract in contracts.flat_imports.values() {
            for leaf in &contract.key.leaves {
                if let Some(previous) =
                    providers.insert((contract.key.module.clone(), leaf.clone()), block)
                {
                    return Err(format!(
                        "cpp_import_namespace leaf `{leaf}` has marked inline providers in blocks {} and {}",
                        previous + 1,
                        block + 1
                    ));
                }
            }
        }
    }
    for (block, file) in files.iter().enumerate() {
        validate_inline_flat_import_block_items(
            &file.items,
            &ModulePath(Vec::new()),
            block,
            &providers,
            &BTreeSet::new(),
        )?;
    }
    Ok(())
}

struct ExternalInlineQualifiedPathAudit<'a> {
    names: &'a BTreeSet<String>,
    error: Option<String>,
}

impl ExternalInlineQualifiedPathAudit<'_> {
    fn audit(&mut self, path: &syn::Path, qself: bool) {
        let explicit_rust_root = path.segments.first().is_some_and(|segment| {
            matches!(ident_key(&segment.ident).as_str(), "crate" | "self" | "super")
        });
        if self.error.is_none()
            && (qself || path.leading_colon.is_some() || explicit_rust_root)
            && path
                .segments
                .last()
                .is_some_and(|segment| self.names.contains(&ident_key(&segment.ident)))
        {
            self.error = Some(format!(
                "qualified path `{}` has a cross-carrier cpp_abi name",
                path.to_token_stream()
            ));
        }
    }
}

impl<'ast> Visit<'ast> for ExternalInlineQualifiedPathAudit<'_> {
    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        self.audit(&path.path, path.qself.is_some());
        if self.error.is_none() {
            syn::visit::visit_expr_path(self, path);
        }
    }

    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        self.audit(&path.path, path.qself.is_some());
        if self.error.is_none() {
            syn::visit::visit_type_path(self, path);
        }
    }
}

/// Validate and lower all blocks in one physical C++ carrier as one ordered
/// Rust module.  Only same-block and earlier-block semantic helper calls are
/// resolvable.  Later providers remain reserved so a forward call fails rather
/// than silently binding to the public STL facade.
pub(crate) fn prepare_inline_carrier(
    files: &[syn::File],
    external_contracts: &ExternalContractIndex,
    inline_identity: &str,
) -> Result<CppAbiInlineCarrierPlan, String> {
    for (index, file) in files.iter().enumerate() {
        validate_cpp_abi_file_attrs(
            &file.attrs,
            &format!("inline block {} source", index + 1),
        )?;
    }
    let joined = joined_inline_file(files);
    let contracts = collect(&joined)?;
    validate_lowering_surface(&joined, &contracts)?;

    if !external_contracts.values.is_empty() || !external_contracts.types.is_empty() {
        let known_modules = BTreeSet::new();
        let mut audit =
            ScopedCrossFileAudit::new(external_contracts, &known_modules, Vec::new());
        audit.audit_module_items(&joined.items);
        if let Some(error) = audit.error {
            return Err(error.replace(
                "cpp_abi crate preflight found a sibling-file reference",
                "inline-rust cpp_abi preflight found a cross-carrier reference",
            ));
        }
        let external_names = external_contracts.all_names();
        let mut qualified = ExternalInlineQualifiedPathAudit {
            names: &external_names,
            error: None,
        };
        qualified.visit_file(&joined);
        if let Some(error) = qualified.error {
            return Err(format!(
                "inline-rust cpp_abi preflight found a cross-carrier reference: {error}"
            ));
        }
    }

    let helper_names = allocate_inline_helper_names(&joined, &contracts, inline_identity)?;
    let mut provider_by_key = BTreeMap::<CallableKey, usize>::new();
    let mut locals = Vec::with_capacity(files.len());
    let mut adapted_blocks = BTreeSet::new();
    let mut flat_import_blocks = BTreeSet::new();
    for (index, file) in files.iter().enumerate() {
        let local = collect(file)?;
        if !local.callables.is_empty() || !local.aliases.is_empty() {
            // This deliberately pins alias+const_ref and static-method owners
            // to one block for the first inline implementation slice.
            validate_lowering_surface(file, &local)?;
            adapted_blocks.insert(index);
        }
        if !local.flat_imports.is_empty() {
            flat_import_blocks.insert(index);
        }
        for key in local.callables.keys() {
            if provider_by_key.insert(key.clone(), index).is_some() {
                return Err(format!(
                    "duplicate inline cpp_abi provider for callable `{key:?}`"
                ));
            }
        }
        locals.push(local);
    }
    validate_inline_flat_import_block_locality(files, &locals)?;
    if provider_by_key.len() != contracts.callables.len() {
        return Err("inline cpp_abi provider census did not match carrier contracts".to_string());
    }

    let mut needs_string_support = contracts.callables.values().any(|contract| {
        contract.returns.is_some()
            || contract
                .params
                .values()
                .any(|adapter| matches!(adapter, ParamAdapter::StdStringBytes))
    });
    let mut needs_vector_support = !contracts.aliases.is_empty();
    let support_owner = adapted_blocks.iter().next().copied();

    let mut blocks = Vec::with_capacity(files.len());
    for (index, (file, local)) in files.iter().zip(locals.iter()).enumerate() {
        let available = provider_by_key
            .iter()
            .filter_map(|(key, provider)| (*provider <= index).then_some(key.clone()))
            .collect::<BTreeSet<_>>();
        let mut lowered = file.clone();
        let used = rewrite_semantic_calls_with_available(
            &mut lowered,
            &contracts,
            &helper_names,
            &available,
            &BTreeSet::new(),
        )?;
        if !used.is_empty() {
            adapted_blocks.insert(index);
        }
        let dependencies = used
            .iter()
            .filter_map(|key| provider_by_key.get(key).copied())
            .filter(|provider| *provider < index)
            .collect::<BTreeSet<_>>();

        let mut emission = CppAbiEmissionPlan {
            aliases: local.aliases.clone(),
            flat_imports: local.flat_imports.clone(),
            inline_identity: Some(inline_identity.to_string()),
            ..CppAbiEmissionPlan::default()
        };
        for (key, contract) in &local.callables {
            let helper_name = helper_names
                .get(key)
                .expect("validated inline cpp_abi helper name")
                .clone();
            emission.semantic_helpers.insert(
                (key_module(key).clone(), helper_name.clone()),
                key.clone(),
            );
            emission.facades.insert(
                key.clone(),
                CallableFacade {
                    contract: contract.clone(),
                    helper_name,
                },
            );
        }
        if support_owner == Some(index) {
            emission.emit_string_support = std::mem::take(&mut needs_string_support);
            emission.emit_vector_support = std::mem::take(&mut needs_vector_support);
        }
        lower_module_items(
            &mut lowered.items,
            &ModulePath(Vec::new()),
            local,
            &helper_names,
        )?;
        blocks.push(CppAbiInlineBlockPlan {
            lowered,
            emission,
            dependencies,
        });
    }

    Ok(CppAbiInlineCarrierPlan {
        blocks,
        adapted_blocks,
        flat_import_blocks,
    })
}

/// Validate and lower the source-owned ABI markers.  `None` is the exact
/// no-marker fast path: callers must pass the original parsed file to codegen
/// so an unannotated source cannot be perturbed by this feature.
pub(crate) fn lower(file: &syn::File) -> Result<Option<(syn::File, CppAbiEmissionPlan)>, String> {
    let contracts = collect(file)?;
    if contracts.callables.is_empty() && contracts.flat_imports.is_empty() {
        return Ok(None);
    }

    validate_lowering_surface(file, &contracts)?;
    let helper_names = allocate_helper_names(file, &contracts)?;
    let mut lowered = file.clone();
    rewrite_semantic_calls(&mut lowered, &contracts, &helper_names)?;

    let mut plan = CppAbiEmissionPlan {
        aliases: contracts.aliases.clone(),
        flat_imports: contracts.flat_imports.clone(),
        ..CppAbiEmissionPlan::default()
    };
    for (key, contract) in &contracts.callables {
        let helper_name = helper_names
            .get(key)
            .expect("validated cpp_abi helper name")
            .clone();
        plan.semantic_helpers
            .insert((key_module(key).clone(), helper_name.clone()), key.clone());
        plan.facades.insert(
            key.clone(),
            CallableFacade {
                contract: contract.clone(),
                helper_name,
            },
        );
    }
    plan.derive_support_requirements();
    lower_module_items(
        &mut lowered.items,
        &ModulePath(Vec::new()),
        &contracts,
        &helper_names,
    )?;
    Ok(Some((lowered, plan)))
}

#[derive(Clone, Copy, Debug)]
enum CrateModuleDeclKind {
    Inline,
    External,
}

#[derive(Clone, Debug)]
struct CrateModuleDecl {
    source: PathBuf,
    is_public: bool,
    unsupported_attrs: Vec<String>,
    kind: CrateModuleDeclKind,
}

fn conventional_file_module_path(path: &Path) -> Result<ModulePath, String> {
    let components = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    if components.first().map(String::as_str) != Some("src") {
        return Err(format!(
            "cpp_abi crate preflight supports only conventional paths below src/; found {}",
            path.display()
        ));
    }
    let relative = &components[1..];
    if relative.is_empty() || !relative.last().is_some_and(|name| name.ends_with(".rs")) {
        return Err(format!(
            "cpp_abi crate preflight expected a Rust source path; found {}",
            path.display()
        ));
    }
    if relative.len() == 1 && matches!(relative[0].as_str(), "lib.rs" | "main.rs") {
        return Ok(ModulePath(Vec::new()));
    }
    let mut module = relative[..relative.len() - 1].to_vec();
    let file = relative.last().expect("checked nonempty");
    if file != "mod.rs" {
        module.push(file.trim_end_matches(".rs").to_string());
    }
    Ok(ModulePath(module))
}

fn collect_crate_module_decls(
    items: &[Item],
    current: &ModulePath,
    source: &Path,
    out: &mut BTreeMap<ModulePath, Vec<CrateModuleDecl>>,
) {
    for item in items {
        let Item::Mod(item_mod) = item else {
            continue;
        };
        let mut path = current.0.clone();
        path.push(ident_key(&item_mod.ident));
        let module = ModulePath(path);
        let unsupported_attrs = item_mod
            .attrs
            .iter()
            .filter(|attr| !is_cpp_abi_doc_or_lint_attr(attr))
            .map(|attr| attr.path().to_token_stream().to_string())
            .collect();
        out.entry(module.clone())
            .or_default()
            .push(CrateModuleDecl {
                source: source.to_path_buf(),
                is_public: is_public(&item_mod.vis),
                unsupported_attrs,
                kind: if item_mod.content.is_some() {
                    CrateModuleDeclKind::Inline
                } else {
                    CrateModuleDeclKind::External
                },
            });
        if let Some((_, nested)) = &item_mod.content {
            collect_crate_module_decls(nested, &module, source, out);
        }
    }
}

fn global_provider_modules(base: &ModulePath, contracts: &CppAbiContracts) -> BTreeSet<ModulePath> {
    let mut modules = BTreeSet::new();
    for (module, _) in contracts.aliases.keys() {
        let mut global = base.0.clone();
        global.extend(module.0.iter().cloned());
        modules.insert(ModulePath(global));
    }
    for key in contracts.callables.keys() {
        let mut global = base.0.clone();
        global.extend(key_module(key).0.iter().cloned());
        modules.insert(ModulePath(global));
    }
    modules
}

fn validate_global_provider_ancestors(
    base: &ModulePath,
    providers: &BTreeSet<ModulePath>,
    declarations: &BTreeMap<ModulePath, Vec<CrateModuleDecl>>,
) -> Result<(), String> {
    for provider in providers {
        for depth in 1..=provider.0.len() {
            let prefix = ModulePath(provider.0[..depth].to_vec());
            let Some(found) = declarations.get(&prefix) else {
                return Err(format!(
                    "cpp_abi provider module `{}` is unattached: missing public module declaration `{}`",
                    provider.0.join("::"),
                    prefix.0.join("::")
                ));
            };
            if found.len() != 1 {
                return Err(format!(
                    "cpp_abi provider ancestor `{}` has {} declarations; exactly one conventional declaration is required",
                    prefix.0.join("::"),
                    found.len()
                ));
            }
            let declaration = &found[0];
            if !declaration.is_public {
                return Err(format!(
                    "cpp_abi provider ancestor `{}` in {} must use exact public visibility",
                    prefix.0.join("::"),
                    declaration.source.display()
                ));
            }
            if !declaration.unsupported_attrs.is_empty() {
                return Err(format!(
                    "cpp_abi provider ancestor `{}` in {} has unsupported attributes: {}",
                    prefix.0.join("::"),
                    declaration.source.display(),
                    declaration.unsupported_attrs.join(", ")
                ));
            }
            let crosses_file_boundary = depth <= base.0.len();
            if crosses_file_boundary && !matches!(declaration.kind, CrateModuleDeclKind::External) {
                return Err(format!(
                    "cpp_abi provider file boundary `{}` must be declared by `pub mod ...;`",
                    prefix.0.join("::")
                ));
            }
            if !crosses_file_boundary && !matches!(declaration.kind, CrateModuleDeclKind::Inline) {
                return Err(format!(
                    "cpp_abi local provider ancestor `{}` must be an inline public module",
                    prefix.0.join("::")
                ));
            }
        }
    }
    Ok(())
}

fn validate_complete_conventional_module_graph(
    physical_modules: &BTreeMap<ModulePath, Vec<&Path>>,
    declarations: &BTreeMap<ModulePath, Vec<CrateModuleDecl>>,
) -> Result<(), String> {
    for (module, found) in declarations {
        for declaration in found {
            if !declaration.unsupported_attrs.is_empty() {
                return Err(format!(
                    "cpp_abi module graph declaration `{}` in {} has unsupported presence/path attributes: {}",
                    module.0.join("::"),
                    declaration.source.display(),
                    declaration.unsupported_attrs.join(", ")
                ));
            }
            if matches!(declaration.kind, CrateModuleDeclKind::External) {
                let physical = physical_modules
                    .get(module)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                if physical.len() != 1 {
                    return Err(format!(
                        "cpp_abi external module declaration `{}` in {} requires exactly one conventional source file; found {}",
                        module.0.join("::"),
                        declaration.source.display(),
                        physical.len()
                    ));
                }
            }
        }
    }

    for (module, sources) in physical_modules {
        if module.0.is_empty() {
            continue;
        }
        let found = declarations.get(module).map(Vec::as_slice).unwrap_or(&[]);
        if found.len() != 1 || !matches!(found[0].kind, CrateModuleDeclKind::External) {
            return Err(format!(
                "cpp_abi physical source module `{}` ({}) must attach exactly once through an unconditional conventional `mod ...;` declaration",
                module.0.join("::"),
                sources
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    Ok(())
}

fn use_tree_contains_glob(tree: &syn::UseTree) -> bool {
    match tree {
        syn::UseTree::Path(path) => use_tree_contains_glob(&path.tree),
        syn::UseTree::Group(group) => group.items.iter().any(use_tree_contains_glob),
        syn::UseTree::Glob(_) => true,
        syn::UseTree::Name(_) | syn::UseTree::Rename(_) => false,
    }
}

fn use_tree_introduces_assert(tree: &syn::UseTree) -> bool {
    let mut leaves = Vec::new();
    collect_use_leaf_paths(tree, &mut Vec::new(), &mut leaves);
    leaves
        .into_iter()
        .any(|(_, introduced)| canonical_name(&introduced) == "assert")
}

fn use_tree_aliases_relative_module_root(tree: &syn::UseTree) -> bool {
    fn walk(tree: &syn::UseTree, prefix: &mut Vec<String>) -> bool {
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(ident_key(&path.ident));
                let found = walk(&path.tree, prefix);
                prefix.pop();
                found
            }
            syn::UseTree::Rename(rename) => {
                let semantic = ident_key(&rename.ident);
                let pushed = semantic != "self" || prefix.is_empty();
                if pushed {
                    prefix.push(semantic);
                }
                let aliases_root = !prefix.is_empty()
                    && prefix
                        .iter()
                        .all(|segment| matches!(segment.as_str(), "crate" | "self" | "super"));
                if pushed {
                    prefix.pop();
                }
                aliases_root
            }
            syn::UseTree::Group(group) => group.items.iter().any(|item| walk(item, prefix)),
            syn::UseTree::Name(_) | syn::UseTree::Glob(_) => false,
        }
    }

    walk(tree, &mut Vec::new())
}

fn is_macro_use_attribute(attr: &Attribute) -> bool {
    attr.path()
        .segments
        .last()
        .is_some_and(|segment| ident_key(&segment.ident) == "macro_use")
}

fn token_stream_declares_assert_macro(tokens: proc_macro2::TokenStream) -> bool {
    let trees = tokens.into_iter().collect::<Vec<_>>();
    for (index, tree) in trees.iter().enumerate() {
        if let proc_macro2::TokenTree::Group(group) = tree
            && token_stream_declares_assert_macro(group.stream())
        {
            return true;
        }
        let proc_macro2::TokenTree::Ident(keyword) = tree else {
            continue;
        };
        let keyword = ident_key(keyword);
        if keyword == "macro_rules"
            && matches!(trees.get(index + 1), Some(proc_macro2::TokenTree::Punct(punct)) if punct.as_char() == '!')
            && matches!(trees.get(index + 2), Some(proc_macro2::TokenTree::Ident(name)) if ident_key(name) == "assert")
        {
            return true;
        }
        if keyword == "macro"
            && matches!(trees.get(index + 1), Some(proc_macro2::TokenTree::Ident(name)) if ident_key(name) == "assert")
        {
            return true;
        }
    }
    false
}

fn token_stream_mentions_assert_identifier(tokens: proc_macro2::TokenStream) -> bool {
    tokens.into_iter().any(|tree| match tree {
        proc_macro2::TokenTree::Ident(ident) => ident_key(&ident) == "assert",
        proc_macro2::TokenTree::Group(group) => {
            token_stream_mentions_assert_identifier(group.stream())
        }
        proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => false,
    })
}

fn item_macro_introduces_assert(item: &syn::ItemMacro) -> bool {
    item.ident
        .as_ref()
        .is_some_and(|ident| ident_key(ident) == "assert")
        || token_stream_declares_assert_macro(item.mac.tokens.clone())
}

/// The only macro surface admitted while crate-mode ABI contracts are active.
/// Keep this structural and deliberately narrow: the built-in must be the
/// exact unqualified `assert!(EXPR)` or `assert!(EXPR, "literal")` spelling,
/// use parentheses, and contain no trailing argument. The diagnostic literal
/// must contain only printable ASCII and no format braces; it is otherwise
/// inert, and only `EXPR` is returned for recursive auditing. Because syntax
/// alone cannot distinguish a shadowing imported macro from the built-in,
/// `assert` is also a reserved macro-binding name everywhere in an adapter
/// crate.
fn parse_admitted_assert_expression(mac: &syn::Macro) -> Option<Expr> {
    if mac.path.leading_colon.is_some()
        || mac.path.segments.len() != 1
        || mac.path.segments[0].ident.to_string() != "assert"
        || !matches!(mac.path.segments[0].arguments, syn::PathArguments::None)
        || !matches!(mac.delimiter, syn::MacroDelimiter::Paren(_))
    {
        return None;
    }
    let parser = |input: syn::parse::ParseStream<'_>| {
        let expression: Expr = input.parse()?;
        if input.is_empty() {
            return Ok(expression);
        }
        input.parse::<Token![,]>()?;
        let message: syn::LitStr = input.parse()?;
        if !message
            .value()
            .chars()
            .all(|character| matches!(character, ' '..='~') && !matches!(character, '{' | '}'))
        {
            return Err(input
                .error("assert literal messages must be printable ASCII without format braces"));
        }
        if !input.is_empty() {
            return Err(input.error("assert literal message must be the final argument"));
        }
        Ok(expression)
    };
    parser.parse2(mac.tokens.clone()).ok()
}

#[derive(Default)]
struct CrateOpaqueSurfaceAudit {
    error: Option<String>,
    inside_assert_expression: bool,
}

impl<'ast> Visit<'ast> for CrateOpaqueSurfaceAudit {
    fn visit_item(&mut self, item: &'ast Item) {
        if let Item::Verbatim(tokens) = item
            && token_stream_declares_assert_macro(tokens.clone())
        {
            self.error = Some(
                "cpp_abi crate preflight reserves the macro definition name `assert`".to_string(),
            );
            return;
        }
        syn::visit::visit_item(self, item);
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if self.error.is_none() && item_macro_introduces_assert(item) {
            self.error = Some(
                "cpp_abi crate preflight reserves the macro definition name `assert`".to_string(),
            );
            return;
        }
        syn::visit::visit_item_macro(self, item);
    }

    fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
        for attr in &item.attrs {
            self.visit_attribute(attr);
        }
        if self.error.is_none() {
            self.error = Some(format!(
                "cpp_abi crate preflight rejects `extern crate` bindings while adapters are present: `{}`",
                item.to_token_stream()
            ));
        }
    }

    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        if self.error.is_none() && is_macro_use_attribute(attr) {
            self.error = Some(format!(
                "cpp_abi crate preflight rejects `#[macro_use]` while adapters are present: `{}`",
                attr.meta.to_token_stream()
            ));
        }
    }

    fn visit_item_use(&mut self, item_use: &'ast syn::ItemUse) {
        for attr in &item_use.attrs {
            self.visit_attribute(attr);
        }
        if self.error.is_some() {
            return;
        }
        if use_tree_contains_glob(&item_use.tree) {
            self.error = Some(format!(
                "cpp_abi crate preflight rejects glob imports: `{}`",
                item_use.to_token_stream()
            ));
        } else if use_tree_aliases_relative_module_root(&item_use.tree) {
            self.error = Some(format!(
                "cpp_abi crate preflight rejects aliases of `crate`, `self`, or `super` while adapters are present: `{}`",
                item_use.to_token_stream()
            ));
        } else if use_tree_introduces_assert(&item_use.tree) {
            self.error = Some(format!(
                "cpp_abi crate preflight reserves the imported macro binding `assert`: `{}`",
                item_use.to_token_stream()
            ));
        }
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if self.error.is_some() {
            return;
        }
        if !self.inside_assert_expression
            && let Some(expression) = parse_admitted_assert_expression(mac)
        {
            self.inside_assert_expression = true;
            self.visit_expr(&expression);
            self.inside_assert_expression = false;
            return;
        }
        self.error = Some(format!(
            "cpp_abi crate preflight rejects opaque macros while adapters are present: `{}`",
            mac.path.to_token_stream()
        ));
    }
}

#[derive(Default)]
struct GlobalContractIndex {
    values: BTreeMap<Vec<String>, usize>,
    types: BTreeMap<Vec<String>, usize>,
    provider_modules: BTreeMap<Vec<String>, usize>,
}

impl GlobalContractIndex {
    fn add(&mut self, unit: usize, base: &ModulePath, contracts: &CppAbiContracts) {
        for ((module, name), _) in &contracts.aliases {
            let mut path = base.0.clone();
            path.extend(module.0.iter().cloned());
            self.provider_modules.entry(path.clone()).or_insert(unit);
            path.push(name.clone());
            self.types.insert(path, unit);
        }
        for key in contracts.callables.keys() {
            match key {
                CallableKey::Free { module, name } => {
                    let mut path = base.0.clone();
                    path.extend(module.0.iter().cloned());
                    self.provider_modules.entry(path.clone()).or_insert(unit);
                    path.push(name.clone());
                    self.values.insert(path, unit);
                }
                CallableKey::InherentStatic {
                    module,
                    owner,
                    name,
                } => {
                    let mut module_path = base.0.clone();
                    module_path.extend(module.0.iter().cloned());
                    self.provider_modules
                        .entry(module_path.clone())
                        .or_insert(unit);
                    let mut owner_path = module_path.clone();
                    owner_path.push(owner.clone());
                    self.types.insert(owner_path.clone(), unit);
                    owner_path.push(name.clone());
                    self.values.insert(owner_path, unit);
                }
            }
        }
    }

    fn external_for(&self, unit: usize) -> ExternalContractIndex {
        ExternalContractIndex {
            values: self
                .values
                .iter()
                .filter_map(|(path, owner)| (*owner != unit).then_some(path.clone()))
                .collect(),
            types: self
                .types
                .iter()
                .filter_map(|(path, owner)| (*owner != unit).then_some(path.clone()))
                .collect(),
            provider_modules: self
                .provider_modules
                .iter()
                .filter_map(|(path, owner)| (*owner != unit).then_some(path.clone()))
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ExternalContractIndex {
    values: BTreeSet<Vec<String>>,
    types: BTreeSet<Vec<String>>,
    provider_modules: BTreeSet<Vec<String>>,
}

impl ExternalContractIndex {
    fn all_names(&self) -> BTreeSet<String> {
        self.values
            .iter()
            .chain(self.types.iter())
            .filter_map(|path| path.last().cloned())
            .collect()
    }
}

#[derive(Default)]
struct PatternBindings {
    names: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for PatternBindings {
    fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
        self.names.insert(ident_key(&pattern.ident));
        if let Some((_, subpattern)) = &pattern.subpat {
            self.visit_pat(subpattern);
        }
    }
}

fn pattern_bindings(pattern: &syn::Pat) -> BTreeSet<String> {
    let mut bindings = PatternBindings::default();
    bindings.visit_pat(pattern);
    bindings.names
}

fn collect_scope_item_bindings(
    items: impl Iterator<Item = Item>,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut values = BTreeSet::new();
    let mut types = BTreeSet::new();
    for item in items {
        match item {
            Item::Fn(item) => {
                values.insert(ident_key(&item.sig.ident));
            }
            Item::Const(item) => {
                values.insert(ident_key(&item.ident));
            }
            Item::Static(item) => {
                values.insert(ident_key(&item.ident));
            }
            Item::Struct(item) => {
                let name = ident_key(&item.ident);
                types.insert(name.clone());
                if matches!(item.fields, syn::Fields::Unit | syn::Fields::Unnamed(_)) {
                    values.insert(name);
                }
            }
            Item::Enum(item) => {
                types.insert(ident_key(&item.ident));
            }
            Item::Union(item) => {
                types.insert(ident_key(&item.ident));
            }
            Item::Type(item) => {
                types.insert(ident_key(&item.ident));
            }
            Item::Trait(item) => {
                types.insert(ident_key(&item.ident));
            }
            Item::TraitAlias(item) => {
                types.insert(ident_key(&item.ident));
            }
            Item::Mod(item) => {
                types.insert(ident_key(&item.ident));
            }
            Item::ExternCrate(item) => {
                types.insert(
                    item.rename
                        .as_ref()
                        .map(|(_, rename)| ident_key(rename))
                        .unwrap_or_else(|| ident_key(&item.ident)),
                );
            }
            Item::Use(item) => {
                let mut names = Vec::new();
                use_tree_bound_names(&item.tree, &mut names);
                values.extend(names.iter().map(|name| canonical_name(name)));
                types.extend(names.into_iter().map(|name| canonical_name(&name)));
            }
            Item::ForeignMod(item) => {
                for foreign in item.items {
                    match foreign {
                        syn::ForeignItem::Fn(item) => {
                            values.insert(ident_key(&item.sig.ident));
                        }
                        syn::ForeignItem::Static(item) => {
                            values.insert(ident_key(&item.ident));
                        }
                        syn::ForeignItem::Type(item) => {
                            types.insert(ident_key(&item.ident));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    (values, types)
}

fn collect_use_leaf_paths(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    out: &mut Vec<(Vec<String>, String)>,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(ident_key(&path.ident));
            collect_use_leaf_paths(&path.tree, prefix, out);
            prefix.pop();
        }
        syn::UseTree::Name(name) => {
            let semantic = ident_key(&name.ident);
            let mut source = prefix.clone();
            if semantic != "self" {
                source.push(semantic.clone());
            }
            let introduced = if semantic == "self" {
                prefix.last().cloned().unwrap_or(semantic)
            } else {
                semantic
            };
            out.push((source, introduced));
        }
        syn::UseTree::Rename(rename) => {
            let semantic = ident_key(&rename.ident);
            let mut source = prefix.clone();
            if semantic != "self" {
                source.push(semantic);
            }
            out.push((source, ident_key(&rename.rename)));
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_use_leaf_paths(item, prefix, out);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

struct ScopedCrossFileAudit<'a> {
    external: &'a ExternalContractIndex,
    known_modules: &'a BTreeSet<Vec<String>>,
    current_module: Vec<String>,
    module_values: Vec<BTreeSet<String>>,
    module_types: Vec<BTreeSet<String>>,
    lexical_values: Vec<BTreeSet<String>>,
    lexical_types: Vec<BTreeSet<String>>,
    inside_assert_expression: bool,
    error: Option<String>,
}

impl<'a> ScopedCrossFileAudit<'a> {
    fn new(
        external: &'a ExternalContractIndex,
        known_modules: &'a BTreeSet<Vec<String>>,
        current_module: Vec<String>,
    ) -> Self {
        Self {
            external,
            known_modules,
            current_module,
            module_values: Vec::new(),
            module_types: Vec::new(),
            lexical_values: Vec::new(),
            lexical_types: Vec::new(),
            inside_assert_expression: false,
            error: None,
        }
    }

    fn fail(&mut self, detail: impl Into<String>) {
        if self.error.is_none() {
            self.error = Some(format!(
                "cpp_abi crate preflight found a sibling-file reference: {}",
                detail.into()
            ));
        }
    }

    fn value_bound(&self, name: &str) -> bool {
        self.lexical_values
            .iter()
            .rev()
            .chain(self.module_values.iter().rev())
            .any(|scope| scope.contains(name))
    }

    fn type_bound(&self, name: &str) -> bool {
        self.lexical_types
            .iter()
            .rev()
            .chain(self.module_types.iter().rev())
            .any(|scope| scope.contains(name))
    }

    fn canonical_path(&self, path: &syn::Path) -> (Vec<String>, bool) {
        let segments = path
            .segments
            .iter()
            .map(|segment| ident_key(&segment.ident))
            .collect::<Vec<_>>();
        if segments.is_empty() {
            return (Vec::new(), false);
        }
        let mut explicit = path.leading_colon.is_some();
        let mut base = if explicit {
            Vec::new()
        } else {
            self.current_module.clone()
        };
        let mut index = 0;
        match segments[0].as_str() {
            "crate" => {
                explicit = true;
                base.clear();
                index = 1;
            }
            "self" => {
                explicit = true;
                index = 1;
            }
            "super" => {
                explicit = true;
                while index < segments.len() && segments[index] == "super" {
                    base.pop();
                    index += 1;
                }
            }
            _ => {}
        }
        base.extend(segments[index..].iter().cloned());
        (base, explicit)
    }

    fn audit_path(&mut self, path: &syn::Path, type_namespace: bool) {
        if self.error.is_some() || path.segments.is_empty() {
            return;
        }
        let semantic = path
            .segments
            .iter()
            .map(|segment| ident_key(&segment.ident))
            .collect::<Vec<_>>();
        if semantic.len() == 1 {
            let name = &semantic[0];
            let bound = if type_namespace {
                self.type_bound(name)
            } else {
                self.value_bound(name)
            };
            if bound {
                return;
            }
        }
        let (canonical, explicit) = self.canonical_path(path);
        let exact = if type_namespace {
            self.external.types.contains(&canonical)
        } else {
            self.external.values.contains(&canonical)
        };
        let external_type_prefix = !type_namespace
            && self
                .external
                .types
                .iter()
                .any(|candidate| canonical.starts_with(candidate));
        if exact || external_type_prefix {
            self.fail(format!(
                "path `{}` resolves to an adapted sibling declaration",
                path.to_token_stream()
            ));
            return;
        }

        let Some(first) = semantic.first() else {
            return;
        };
        let Some(last) = semantic.last() else {
            return;
        };
        let reserved_names = self.external.all_names();
        if semantic.len() == 1 {
            let bound = if type_namespace {
                self.type_bound(first)
            } else {
                self.value_bound(first)
            };
            if !bound && reserved_names.contains(first) {
                self.fail(format!(
                    "unbound path `{}` has an adapted sibling name",
                    path.to_token_stream()
                ));
            }
            return;
        }

        if explicit {
            return;
        }
        let root_bound = self.type_bound(first) || self.value_bound(first);
        let module_prefix = canonical[..canonical.len().saturating_sub(1)].to_vec();
        if !root_bound
            && !self.known_modules.contains(&module_prefix)
            && reserved_names.contains(last)
        {
            self.fail(format!(
                "unresolved qualified path `{}` has an adapted sibling tail",
                path.to_token_stream()
            ));
        }
    }

    fn bind_generics(&mut self, generics: &syn::Generics) {
        if self.lexical_values.is_empty() {
            self.lexical_values.push(BTreeSet::new());
        }
        if self.lexical_types.is_empty() {
            self.lexical_types.push(BTreeSet::new());
        }
        for parameter in &generics.params {
            match parameter {
                syn::GenericParam::Const(parameter) => {
                    self.lexical_values
                        .last_mut()
                        .expect("value scope")
                        .insert(ident_key(&parameter.ident));
                }
                syn::GenericParam::Type(parameter) => {
                    self.lexical_types
                        .last_mut()
                        .expect("type scope")
                        .insert(ident_key(&parameter.ident));
                }
                syn::GenericParam::Lifetime(_) => {}
            }
        }
    }

    fn audit_function(
        &mut self,
        attrs: &[Attribute],
        sig: &syn::Signature,
        block: Option<&syn::Block>,
    ) {
        for attr in attrs {
            self.visit_attribute(attr);
        }
        let saved_values = std::mem::take(&mut self.lexical_values);
        let saved_types = std::mem::take(&mut self.lexical_types);
        self.lexical_values.push(BTreeSet::new());
        self.lexical_types.push(BTreeSet::new());
        self.bind_generics(&sig.generics);
        syn::visit::visit_generics(self, &sig.generics);
        for input in &sig.inputs {
            match input {
                FnArg::Receiver(receiver) => {
                    self.lexical_values
                        .last_mut()
                        .expect("value scope")
                        .insert("self".to_string());
                    syn::visit::visit_receiver(self, receiver);
                }
                FnArg::Typed(input) => {
                    self.visit_type(&input.ty);
                    self.visit_pat(&input.pat);
                    self.lexical_values
                        .last_mut()
                        .expect("value scope")
                        .extend(pattern_bindings(&input.pat));
                }
            }
        }
        if let syn::ReturnType::Type(_, output) = &sig.output {
            self.visit_type(output);
        }
        if let Some(block) = block {
            self.visit_block(block);
        }
        self.lexical_values = saved_values;
        self.lexical_types = saved_types;
    }

    fn audit_module_items(&mut self, items: &[Item]) {
        let (values, types) = collect_scope_item_bindings(items.iter().cloned());
        self.module_values.push(values);
        self.module_types.push(types);
        for item in items {
            self.visit_item(item);
            if self.error.is_some() {
                break;
            }
        }
        self.module_values.pop();
        self.module_types.pop();
    }

    /// Audit an `if`/`while` condition from left to right and return the
    /// bindings introduced by let expressions that remain in scope after the
    /// condition succeeds. Rust let chains propagate bindings only across
    /// `&&`: each initializer is evaluated before its pattern binds, earlier
    /// bindings are visible to later operands, and no condition binding leaks
    /// into an `else` branch.
    fn audit_let_chain_condition(&mut self, expression: &syn::Expr) -> BTreeSet<String> {
        if self.error.is_some() {
            return BTreeSet::new();
        }
        match expression {
            syn::Expr::Let(let_) => {
                for attr in &let_.attrs {
                    self.visit_attribute(attr);
                }
                self.visit_expr(&let_.expr);
                self.visit_pat(&let_.pat);
                pattern_bindings(&let_.pat)
            }
            syn::Expr::Binary(binary) if matches!(binary.op, syn::BinOp::And(_)) => {
                for attr in &binary.attrs {
                    self.visit_attribute(attr);
                }
                let mut bindings = self.audit_let_chain_condition(&binary.left);
                self.lexical_values.push(bindings.clone());
                self.lexical_types.push(BTreeSet::new());
                let right = self.audit_let_chain_condition(&binary.right);
                self.lexical_values.pop();
                self.lexical_types.pop();
                bindings.extend(right);
                bindings
            }
            syn::Expr::Group(group) => {
                for attr in &group.attrs {
                    self.visit_attribute(attr);
                }
                self.audit_let_chain_condition(&group.expr)
            }
            syn::Expr::Paren(paren) => {
                for attr in &paren.attrs {
                    self.visit_attribute(attr);
                }
                self.audit_let_chain_condition(&paren.expr)
            }
            _ => {
                self.visit_expr(expression);
                BTreeSet::new()
            }
        }
    }
}

impl<'ast> Visit<'ast> for ScopedCrossFileAudit<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        if let Item::Verbatim(tokens) = item
            && token_stream_declares_assert_macro(tokens.clone())
        {
            self.fail("the macro definition name `assert` is reserved while adapters exist");
            return;
        }
        syn::visit::visit_item(self, item);
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if self.error.is_none() && item_macro_introduces_assert(item) {
            self.fail("the macro definition name `assert` is reserved while adapters exist");
            return;
        }
        syn::visit::visit_item_macro(self, item);
    }

    fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
        for attr in &item.attrs {
            self.visit_attribute(attr);
        }
        if self.error.is_none() {
            self.fail(format!(
                "`extern crate` bindings are unsupported while adapters exist: `{}`",
                item.to_token_stream()
            ));
        }
    }

    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        if self.error.is_none() && is_macro_use_attribute(attr) {
            self.fail(format!(
                "`#[macro_use]` is unsupported while adapters exist: `{}`",
                attr.meta.to_token_stream()
            ));
        } else if self.error.is_none()
            && !attr.path().is_ident("doc")
            && token_stream_mentions_names(attr.meta.to_token_stream(), &self.external.all_names())
        {
            self.fail(format!(
                "attribute metadata mentions an adapted sibling name: `{}`",
                attr.meta.to_token_stream()
            ));
        }
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if self.error.is_some() {
            return;
        }
        if !self.inside_assert_expression
            && let Some(expression) = parse_admitted_assert_expression(mac)
        {
            self.inside_assert_expression = true;
            self.visit_expr(&expression);
            self.inside_assert_expression = false;
            return;
        }
        self.fail(format!(
            "opaque macro `{}` is unsupported while sibling adapters exist",
            mac.path.to_token_stream()
        ));
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        for attr in &item.attrs {
            self.visit_attribute(attr);
        }
        if let Some((_, nested)) = &item.content {
            let previous = self.current_module.clone();
            self.current_module.push(ident_key(&item.ident));
            self.audit_module_items(nested);
            self.current_module = previous;
        }
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        for attr in &item.attrs {
            self.visit_attribute(attr);
        }
        if self.error.is_some() {
            return;
        }
        if use_tree_contains_glob(&item.tree) {
            self.fail(format!(
                "glob import `{}` is unsupported",
                item.to_token_stream()
            ));
            return;
        }
        if use_tree_aliases_relative_module_root(&item.tree) {
            self.fail(format!(
                "aliases of `crate`, `self`, or `super` are unsupported while adapters exist: `{}`",
                item.to_token_stream()
            ));
            return;
        }
        if use_tree_introduces_assert(&item.tree) {
            self.fail(format!(
                "the imported macro binding `assert` is reserved while adapters exist: `{}`",
                item.to_token_stream()
            ));
            return;
        }
        let names = self.external.all_names();
        if use_tree_mentions_names(&item.tree, &names) {
            self.fail(format!(
                "import/re-export mentions an adapted sibling name: `{}`",
                item.to_token_stream()
            ));
            return;
        }
        let mut leaves = Vec::new();
        collect_use_leaf_paths(&item.tree, &mut Vec::new(), &mut leaves);
        for (segments, _) in leaves {
            let path: syn::Path = match syn::parse_str(
                &segments
                    .iter()
                    .map(|segment| {
                        if matches!(segment.as_str(), "crate" | "self" | "super") {
                            segment.clone()
                        } else {
                            format!("r#{segment}")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("::"),
            ) {
                Ok(path) => path,
                Err(_) => continue,
            };
            let (canonical, _) = self.canonical_path(&path);
            if self
                .external
                .provider_modules
                .iter()
                .any(|provider| provider.starts_with(&canonical))
                || self.external.values.contains(&canonical)
                || self.external.types.contains(&canonical)
            {
                self.fail(format!(
                    "import/re-export targets an adapted sibling module or declaration: `{}`",
                    item.to_token_stream()
                ));
                return;
            }
        }
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.audit_function(&item.attrs, &item.sig, Some(&item.block));
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.audit_function(&item.attrs, &item.sig, Some(&item.block));
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        self.audit_function(&item.attrs, &item.sig, item.default.as_ref());
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        let local_items = block.stmts.iter().filter_map(|statement| match statement {
            syn::Stmt::Item(item) => Some(item.clone()),
            _ => None,
        });
        let (values, types) = collect_scope_item_bindings(local_items);
        self.lexical_values.push(values);
        self.lexical_types.push(types);
        for statement in &block.stmts {
            match statement {
                syn::Stmt::Local(local) => {
                    for attr in &local.attrs {
                        self.visit_attribute(attr);
                    }
                    self.visit_pat(&local.pat);
                    if let Some(initializer) = &local.init {
                        self.visit_expr(&initializer.expr);
                        if let Some((_, diverge)) = &initializer.diverge {
                            self.visit_expr(diverge);
                        }
                    }
                    self.lexical_values
                        .last_mut()
                        .expect("block value scope")
                        .extend(pattern_bindings(&local.pat));
                }
                syn::Stmt::Item(item) => self.visit_item(item),
                syn::Stmt::Expr(expr, _) => self.visit_expr(expr),
                syn::Stmt::Macro(statement) => self.visit_macro(&statement.mac),
            }
            if self.error.is_some() {
                break;
            }
        }
        self.lexical_values.pop();
        self.lexical_types.pop();
    }

    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        self.lexical_values.push(BTreeSet::new());
        self.lexical_types.push(BTreeSet::new());
        for input in &closure.inputs {
            self.visit_pat(input);
            self.lexical_values
                .last_mut()
                .expect("closure value scope")
                .extend(pattern_bindings(input));
        }
        self.visit_expr(&closure.body);
        self.lexical_values.pop();
        self.lexical_types.pop();
    }

    fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
        for attr in &expression.attrs {
            self.visit_attribute(attr);
        }
        let bindings = self.audit_let_chain_condition(&expression.cond);
        self.lexical_values.push(bindings);
        self.lexical_types.push(BTreeSet::new());
        self.visit_block(&expression.then_branch);
        self.lexical_values.pop();
        self.lexical_types.pop();
        if let Some((_, else_branch)) = &expression.else_branch {
            self.visit_expr(else_branch);
        }
    }

    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        for attr in &expression.attrs {
            self.visit_attribute(attr);
        }
        if let Some(label) = &expression.label {
            self.visit_label(label);
        }
        let bindings = self.audit_let_chain_condition(&expression.cond);
        self.lexical_values.push(bindings);
        self.lexical_types.push(BTreeSet::new());
        self.visit_block(&expression.body);
        self.lexical_values.pop();
        self.lexical_types.pop();
    }

    fn visit_expr_for_loop(&mut self, loop_: &'ast syn::ExprForLoop) {
        self.visit_expr(&loop_.expr);
        self.visit_pat(&loop_.pat);
        self.lexical_values.push(pattern_bindings(&loop_.pat));
        self.lexical_types.push(BTreeSet::new());
        self.visit_block(&loop_.body);
        self.lexical_values.pop();
        self.lexical_types.pop();
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        self.visit_pat(&arm.pat);
        self.lexical_values.push(pattern_bindings(&arm.pat));
        self.lexical_types.push(BTreeSet::new());
        if let Some((_, guard)) = &arm.guard {
            self.visit_expr(guard);
        }
        self.visit_expr(&arm.body);
        self.lexical_values.pop();
        self.lexical_types.pop();
    }

    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        self.audit_path(&path.path, false);
        if self.error.is_none() {
            syn::visit::visit_expr_path(self, path);
        }
    }

    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        self.audit_path(&path.path, true);
        if self.error.is_none() {
            syn::visit::visit_type_path(self, path);
        }
    }
}

/// Fail-closed crate-wide audit for ordinary per-file crate mode. The returned
/// boolean says whether any adapter exists; `false` is the exact legacy path.
/// Local lowering still runs in each owning unit after this global audit.
pub(crate) fn validate_source_contract_module_graph(
    inputs: &[(PathBuf, String)],
    contract_name: &str,
    owner_sources: &[PathBuf],
) -> Result<(), String> {
    let relabel = |error: String| error.replacen("cpp_abi", contract_name, 1);
    let mut parsed = Vec::with_capacity(inputs.len());
    for (path, source) in inputs {
        let base = conventional_file_module_path(path).map_err(&relabel)?;
        let file = syn::parse_file(source).map_err(|error| {
            format!(
                "{contract_name} crate preflight could not parse {}: {error}",
                path.display()
            )
        })?;
        validate_cpp_abi_file_attrs(
            &file.attrs,
            &format!("crate source file `{}`", path.display()),
        )
        .map_err(&relabel)?;
        parsed.push((path, base, file));
    }

    let mut physical_modules = BTreeMap::<ModulePath, Vec<&Path>>::new();
    for (path, base, _) in &parsed {
        physical_modules
            .entry(base.clone())
            .or_default()
            .push(path.as_path());
    }
    for (module, paths) in &physical_modules {
        if paths.len() != 1 {
            return Err(format!(
                "{contract_name} requires one conventional source file per Rust module `{}`; found {}",
                module.0.join("::"),
                paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    let mut declarations = BTreeMap::<ModulePath, Vec<CrateModuleDecl>>::new();
    for (path, base, file) in &parsed {
        collect_crate_module_decls(&file.items, base, path, &mut declarations);
    }
    validate_complete_conventional_module_graph(&physical_modules, &declarations)
        .map_err(&relabel)?;
    for (path, base, _) in &parsed {
        if !owner_sources.iter().any(|owner| owner == *path) {
            continue;
        }
        validate_global_provider_ancestors(
            base,
            &std::iter::once(base.clone()).collect(),
            &declarations,
        )
        .map_err(&relabel)?;
    }
    Ok(())
}

pub(crate) fn preflight_crate_sources(inputs: &[(PathBuf, String)]) -> Result<bool, String> {
    preflight_crate_sources_impl(inputs, false, None)
}

pub(crate) fn preflight_crate_sources_with_cxx_namespace(
    inputs: &[(PathBuf, String)],
    cxx_namespace: Option<&str>,
) -> Result<bool, String> {
    preflight_crate_sources_impl(inputs, true, cxx_namespace)
}

fn preflight_crate_sources_impl(
    inputs: &[(PathBuf, String)],
    validate_cxx_namespace: bool,
    cxx_namespace: Option<&str>,
) -> Result<bool, String> {
    struct Unit {
        path: PathBuf,
        base: ModulePath,
        file: syn::File,
        contracts: CppAbiContracts,
    }

    if inputs
        .iter()
        .all(|(_, source)| !source_mentions_reserved_marker(source))
    {
        return Ok(false);
    }

    let mut units = Vec::with_capacity(inputs.len());
    for (path, source) in inputs {
        let base = conventional_file_module_path(path)?;
        let file = syn::parse_file(source).map_err(|error| {
            format!(
                "cpp_abi crate preflight could not parse {}: {error}",
                path.display()
            )
        })?;
        let contracts = collect(&file)
            .map_err(|error| format!("cpp_abi crate preflight {}: {error}", path.display()))?;
        units.push(Unit {
            path: path.clone(),
            base,
            file,
            contracts,
        });
    }
    if units
        .iter()
        .all(|unit| {
            unit.contracts.callables.is_empty()
                && unit.contracts.aliases.is_empty()
                && unit.contracts.flat_imports.is_empty()
        })
    {
        return Ok(false);
    }

    if validate_cxx_namespace {
        for unit in &units {
            validate_flat_import_namespaces(
                &unit.contracts.flat_imports,
                cxx_namespace,
                &format!("crate source `{}`", unit.path.display()),
            )?;
        }
    }

    for unit in &units {
        validate_cpp_abi_file_attrs(
            &unit.file.attrs,
            &format!("crate source file `{}`", unit.path.display()),
        )?;
    }

    let mut physical_modules = BTreeMap::<ModulePath, Vec<&Path>>::new();
    for unit in &units {
        physical_modules
            .entry(unit.base.clone())
            .or_default()
            .push(&unit.path);
    }
    for (module, paths) in &physical_modules {
        if paths.len() != 1 {
            return Err(format!(
                "cpp_abi requires one conventional source file per Rust module `{}`; found {}",
                module.0.join("::"),
                paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    let mut declarations = BTreeMap::<ModulePath, Vec<CrateModuleDecl>>::new();
    for unit in &units {
        collect_crate_module_decls(&unit.file.items, &unit.base, &unit.path, &mut declarations);
    }
    validate_complete_conventional_module_graph(&physical_modules, &declarations)?;

    // A crate-mode flat import is deliberately narrower than ordinary Rust
    // name resolution.  `crate::<child>::Name` must come from the generated
    // interface for one exact physical root child; inline modules and
    // re-exports do not have an independently importable C++ named module.
    // The leaf itself must be the direct public, ordinary, non-generic free
    // function that this first contract slice is designed for.
    for consumer in &units {
        for contract in consumer.contracts.flat_imports.values() {
            let provider_module = ModulePath(vec![contract.key.rust_child.clone()]);
            let provider_paths = physical_modules
                .get(&provider_module)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            if provider_paths.len() != 1 {
                return Err(format!(
                    "cpp_import_namespace crate child `{}` must be exactly one physical generated root module; found {}",
                    contract.key.rust_child,
                    provider_paths.len()
                ));
            }
            let provider = units
                .iter()
                .find(|unit| unit.base == provider_module)
                .expect("physical flat-import provider has a parsed unit");
            for leaf in &contract.key.leaves {
                let direct = provider
                    .file
                    .items
                    .iter()
                    .filter_map(|item| match item {
                        Item::Fn(item_fn) if ident_key(&item_fn.sig.ident) == *leaf => {
                            Some(item_fn)
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if direct.len() != 1 {
                    return Err(format!(
                        "cpp_import_namespace leaf `crate::{}::{leaf}` must be exactly one direct root-level free function in {}; found {}",
                        contract.key.rust_child,
                        provider.path.display(),
                        direct.len()
                    ));
                }
                let function = direct[0];
                if !matches!(function.vis, syn::Visibility::Public(_)) {
                    return Err(format!(
                        "cpp_import_namespace leaf `crate::{}::{leaf}` must be an exact public free function",
                        contract.key.rust_child
                    ));
                }
                let callable_key = CallableKey::Free {
                    module: ModulePath(Vec::new()),
                    name: leaf.clone(),
                };
                if provider.contracts.callables.contains_key(&callable_key) {
                    return Err(format!(
                        "cpp_import_namespace leaf `crate::{}::{leaf}` must be unadapted",
                        contract.key.rust_child
                    ));
                }
                let unsupported_attrs = function
                    .attrs
                    .iter()
                    .filter(|attr| !is_cpp_abi_doc_or_lint_attr(attr))
                    .map(|attr| attr.path().to_token_stream().to_string())
                    .collect::<Vec<_>>();
                let ordinary = function.sig.generics.params.is_empty()
                    && function.sig.generics.where_clause.is_none()
                    && function.sig.constness.is_none()
                    && function.sig.asyncness.is_none()
                    && function.sig.unsafety.is_none()
                    && function.sig.abi.is_none()
                    && function.sig.variadic.is_none();
                if !ordinary || !unsupported_attrs.is_empty() {
                    return Err(format!(
                        "cpp_import_namespace leaf `crate::{}::{leaf}` must be an unconditional, ordinary, non-generic free function; unsupported attributes: {}",
                        contract.key.rust_child,
                        if unsupported_attrs.is_empty() {
                            "none".to_string()
                        } else {
                            unsupported_attrs.join(", ")
                        }
                    ));
                }
            }
        }
    }

    let global_providers = units
        .iter()
        .flat_map(|unit| global_provider_modules(&unit.base, &unit.contracts))
        .collect::<BTreeSet<_>>();
    let mut projected = ProjectedCppCensus::default();
    for unit in &units {
        collect_projected_cpp_names(
            &unit.file.items,
            &ModulePath(Vec::new()),
            &[],
            &unit.contracts,
            &unit.path.display().to_string(),
            Some(&unit.base),
            Some(&global_providers),
            &mut projected,
        );
    }
    projected.validate()?;

    let mut global_contracts = GlobalContractIndex::default();
    for (index, unit) in units.iter().enumerate() {
        global_contracts.add(index, &unit.base, &unit.contracts);
    }
    let known_modules = physical_modules
        .keys()
        .chain(declarations.keys())
        .map(|module| module.0.clone())
        .collect::<BTreeSet<_>>();
    let flat_import_rules = collect_flat_import_crate_rules(
        units
            .iter()
            .map(|unit| (&unit.base, &unit.contracts)),
    );
    let has_adapter_contracts = units.iter().any(|unit| {
        !unit.contracts.callables.is_empty() || !unit.contracts.aliases.is_empty()
    });

    let mut name_owners = BTreeMap::<String, usize>::new();
    for (index, unit) in units.iter().enumerate() {
        let providers = global_provider_modules(&unit.base, &unit.contracts);
        validate_global_provider_ancestors(&unit.base, &providers, &declarations)?;
        if !unit.contracts.callables.is_empty()
            || !unit.contracts.aliases.is_empty()
            || !unit.contracts.flat_imports.is_empty()
        {
            lower(&unit.file).map_err(|error| {
                format!("cpp_abi crate preflight {}: {error}", unit.path.display())
            })?;
        }
        for name in reserved_contract_names(&unit.contracts) {
            if let Some(previous) = name_owners.insert(name.clone(), index)
                && previous != index
            {
                return Err(format!(
                    "cpp_abi crate preflight requires globally unique adapter names; `{name}` occurs in {} and {}",
                    units[previous].path.display(),
                    unit.path.display()
                ));
            }
        }
    }

    for (index, unit) in units.iter().enumerate() {
        validate_flat_import_crate_references(
            &unit.file,
            &unit.base,
            &flat_import_rules,
        )
        .map_err(|error| format!("{error} in {}", unit.path.display()))?;
        let external = global_contracts.external_for(index);
        if !external.values.is_empty() || !external.types.is_empty() {
            let mut audit =
                ScopedCrossFileAudit::new(&external, &known_modules, unit.base.0.clone());
            audit.audit_module_items(&unit.file.items);
            if let Some(error) = audit.error {
                return Err(format!("{error} in {}", unit.path.display()));
            }
        }
        if has_adapter_contracts {
            let mut opaque = CrateOpaqueSurfaceAudit::default();
            opaque.visit_file(&unit.file);
            if let Some(error) = opaque.error {
                return Err(format!("{} in {}", error, unit.path.display()));
            }
        }
    }
    Ok(true)
}

fn key_module(key: &CallableKey) -> &ModulePath {
    match key {
        CallableKey::Free { module, .. } | CallableKey::InherentStatic { module, .. } => module,
    }
}

fn helper_stem(key: &CallableKey) -> String {
    match key {
        CallableKey::Free { name, .. } => format!("rusty_cpp_abi_sem_{name}"),
        CallableKey::InherentStatic { owner, name, .. } => {
            format!("rusty_cpp_abi_sem_{owner}_{name}")
        }
    }
}

fn inline_helper_stem(key: &CallableKey, identity: &str) -> String {
    helper_stem(key).replacen(
        "rusty_cpp_abi_sem_",
        &format!("rusty_cpp_abi_sem_{identity}_"),
        1,
    )
}

fn allocate_inline_helper_names(
    file: &syn::File,
    contracts: &CppAbiContracts,
    identity: &str,
) -> Result<BTreeMap<CallableKey, String>, String> {
    if identity.is_empty()
        || !identity
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err("inline cpp_abi identity must be a nonempty C++ identifier fragment".to_string());
    }
    let detail = format!("rusty_cpp_abi_detail_{identity}");
    let mut names_by_module = BTreeMap::<ModulePath, BTreeSet<String>>::new();
    collect_namespace_item_names(&file.items, &ModulePath(Vec::new()), &mut names_by_module);
    if names_by_module
        .values()
        .any(|names| names.contains(&detail))
    {
        return Err(format!(
            "source item `{detail}` collides with the reserved inline cpp_abi conversion namespace"
        ));
    }
    let mut result = BTreeMap::new();
    for key in contracts.callables.keys() {
        let module = key_module(key);
        let helper = inline_helper_stem(key, identity);
        let occupied = names_by_module.entry(module.clone()).or_default();
        if !occupied.insert(helper.clone()) {
            return Err(format!(
                "inline cpp_abi semantic helper name `{helper}` collides in module `{}`",
                module.0.join("::")
            ));
        }
        result.insert(key.clone(), helper);
    }
    Ok(result)
}

fn allocate_helper_names(
    file: &syn::File,
    contracts: &CppAbiContracts,
) -> Result<BTreeMap<CallableKey, String>, String> {
    let mut names_by_module = BTreeMap::<ModulePath, BTreeSet<String>>::new();
    collect_namespace_item_names(&file.items, &ModulePath(Vec::new()), &mut names_by_module);
    if names_by_module
        .values()
        .any(|names| names.contains("rusty_cpp_abi_detail"))
    {
        return Err(
            "source item `rusty_cpp_abi_detail` collides with the reserved cpp_abi conversion namespace"
                .to_string(),
        );
    }
    let mut result = BTreeMap::new();
    for key in contracts.callables.keys() {
        let module = key_module(key);
        let helper = helper_stem(key);
        let occupied = names_by_module.entry(module.clone()).or_default();
        if !occupied.insert(helper.clone()) {
            return Err(format!(
                "cpp_abi semantic helper name `{helper}` collides in module `{}`",
                module.0.join("::")
            ));
        }
        result.insert(key.clone(), helper);
    }
    Ok(result)
}

fn collect_namespace_item_names(
    items: &[Item],
    module: &ModulePath,
    out: &mut BTreeMap<ModulePath, BTreeSet<String>>,
) {
    let names = out.entry(module.clone()).or_default();
    for item in items {
        let ident = match item {
            Item::Const(v) => Some(&v.ident),
            Item::Enum(v) => Some(&v.ident),
            Item::ExternCrate(v) => Some(&v.ident),
            Item::Fn(v) => Some(&v.sig.ident),
            Item::Macro(v) => v.ident.as_ref(),
            Item::Mod(v) => Some(&v.ident),
            Item::Static(v) => Some(&v.ident),
            Item::Struct(v) => Some(&v.ident),
            Item::Trait(v) => Some(&v.ident),
            Item::TraitAlias(v) => Some(&v.ident),
            Item::Type(v) => Some(&v.ident),
            Item::Union(v) => Some(&v.ident),
            _ => None,
        };
        if let Some(ident) = ident {
            names.insert(ident_key(ident));
        }
    }
    for item in items {
        if let Item::Mod(item_mod) = item
            && let Some((_, nested)) = &item_mod.content
        {
            let mut path = module.0.clone();
            path.push(ident_key(&item_mod.ident));
            collect_namespace_item_names(nested, &ModulePath(path), out);
        }
    }
}

fn flat_import_leaves_by_module(
    contracts: &CppAbiContracts,
) -> Result<BTreeMap<ModulePath, BTreeSet<String>>, String> {
    let mut leaves_by_module = BTreeMap::<ModulePath, BTreeSet<String>>::new();
    for contract in contracts.flat_imports.values() {
        let leaves = leaves_by_module
            .entry(contract.key.module.clone())
            .or_default();
        for leaf in &contract.key.leaves {
            if !leaves.insert(leaf.clone()) {
                return Err(format!(
                    "cpp_import_namespace leaf `{leaf}` is imported more than once in Rust module `{}`",
                    contract.key.module.0.join("::")
                ));
            }
        }
    }
    Ok(leaves_by_module)
}

type FlatImportCrateRules =
    BTreeMap<(Vec<String>, String), BTreeSet<Vec<String>>>;

fn collect_flat_import_crate_rules<'a>(
    units: impl Iterator<Item = (&'a ModulePath, &'a CppAbiContracts)>,
) -> FlatImportCrateRules {
    let mut rules = FlatImportCrateRules::new();
    for (base, contracts) in units {
        for contract in contracts.flat_imports.values() {
            let provider = vec![contract.key.rust_child.clone()];
            let mut consumer = base.0.clone();
            consumer.extend(contract.key.module.0.iter().cloned());
            for leaf in &contract.key.leaves {
                rules
                    .entry((provider.clone(), leaf.clone()))
                    .or_default()
                    .insert(consumer.clone());
            }
        }
    }
    rules
}

fn canonical_crate_path_segments(
    current_module: &[String],
    segments: &[String],
    leading_colon: bool,
) -> Vec<String> {
    if segments.is_empty() {
        return Vec::new();
    }
    let mut base = if leading_colon {
        Vec::new()
    } else {
        current_module.to_vec()
    };
    let mut index = 0usize;
    match segments[0].as_str() {
        "crate" => {
            base.clear();
            index = 1;
        }
        "self" => index = 1,
        "super" => {
            while index < segments.len() && segments[index] == "super" {
                base.pop();
                index += 1;
            }
        }
        _ => {}
    }
    base.extend(segments[index..].iter().cloned());
    base
}

fn collect_use_source_paths(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    out: &mut Vec<Vec<String>>,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(ident_key(&path.ident));
            collect_use_source_paths(&path.tree, prefix, out);
            prefix.pop();
        }
        syn::UseTree::Name(name) => {
            let semantic = ident_key(&name.ident);
            let mut source = prefix.clone();
            if semantic != "self" {
                source.push(semantic);
            }
            out.push(source);
        }
        syn::UseTree::Rename(rename) => {
            let semantic = ident_key(&rename.ident);
            let mut source = prefix.clone();
            if semantic != "self" {
                source.push(semantic);
            }
            out.push(source);
        }
        syn::UseTree::Glob(_) => out.push(prefix.clone()),
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_use_source_paths(item, prefix, out);
            }
        }
    }
}

struct FlatImportCrateReferenceAudit<'a> {
    rules: &'a FlatImportCrateRules,
    current_module: Vec<String>,
    namespace_depth: usize,
    block_depth: usize,
    error: Option<String>,
}

impl FlatImportCrateReferenceAudit<'_> {
    fn provider_scope(provider: &[String], module: &[String]) -> bool {
        module.starts_with(provider)
    }

    fn fail(&mut self, detail: impl Into<String>) {
        if self.error.is_none() {
            self.error = Some(format!(
                "cpp_import_namespace crate preflight rejects {detail}",
                detail = detail.into()
            ));
        }
    }

    fn marked_use_is_authorized(&self, item: &syn::ItemUse) -> bool {
        let Some(contract) = parse_flat_import_use(item, &ModulePath(Vec::new()))
            .ok()
            .flatten()
        else {
            return false;
        };
        contract.key.leaves.iter().all(|leaf| {
            self.rules
                .get(&(vec![contract.key.rust_child.clone()], leaf.clone()))
                .is_some_and(|consumers| consumers.contains(&self.current_module))
        })
    }

    fn path_is_forbidden(
        &self,
        canonical: &[String],
        unqualified: bool,
    ) -> bool {
        self.rules.iter().any(|((provider, leaf), consumers)| {
            if Self::provider_scope(provider, &self.current_module) {
                return false;
            }
            let mut provider_item = provider.clone();
            provider_item.push(leaf.clone());
            if canonical == provider_item {
                return true;
            }
            consumers.iter().any(|consumer| {
                let mut imported_binding = consumer.clone();
                imported_binding.push(leaf.clone());
                canonical == imported_binding
                    && !(unqualified && consumer == &self.current_module)
            })
        })
    }

    fn use_source_is_forbidden(&self, canonical: &[String]) -> bool {
        self.rules.iter().any(|((provider, leaf), consumers)| {
            if Self::provider_scope(provider, &self.current_module) {
                return false;
            }
            let mut provider_item = provider.clone();
            provider_item.push(leaf.clone());
            canonical == provider
                || canonical == provider_item
                || consumers.iter().any(|consumer| {
                    let mut imported_binding = consumer.clone();
                    imported_binding.push(leaf.clone());
                    canonical == imported_binding
                        || (!canonical.is_empty() && consumer.starts_with(canonical))
                        || (canonical.is_empty() && consumer.is_empty())
                })
        })
    }

    fn opaque_leaf_names(&self) -> BTreeSet<String> {
        self.rules
            .keys()
            .filter_map(|(provider, leaf)| {
                (!Self::provider_scope(provider, &self.current_module))
                    .then_some(leaf.clone())
            })
            .collect()
    }

    fn colliding_leaf_names(&self) -> BTreeSet<String> {
        self.rules
            .keys()
            .map(|(_, leaf)| leaf.clone())
            .collect()
    }

    fn is_exact_provider_leaf_function(&self, item: &Item) -> bool {
        let Item::Fn(function) = item else {
            return false;
        };
        let name = ident_key(&function.sig.ident);
        self.rules
            .keys()
            .any(|(provider, leaf)| provider == &self.current_module && leaf == &name)
    }

    fn local_impl_target_name(item: &syn::ItemImpl) -> Option<String> {
        fn path(ty: &Type) -> Option<&syn::TypePath> {
            match ty {
                Type::Path(path) if path.qself.is_none() => Some(path),
                Type::Reference(reference) => path(&reference.elem),
                Type::Paren(paren) => path(&paren.elem),
                Type::Group(group) => path(&group.elem),
                _ => None,
            }
        }
        path(&item.self_ty)?
            .path
            .segments
            .last()
            .map(|segment| ident_key(&segment.ident))
    }

    fn local_impl_forces_namespace_hoist(item: &syn::ItemImpl) -> bool {
        let impl_is_generic = item.generics.params.iter().any(|parameter| {
            matches!(
                parameter,
                syn::GenericParam::Type(_) | syn::GenericParam::Const(_)
            )
        });
        let member_template = item.items.iter().any(|member| {
            matches!(
                member,
                syn::ImplItem::Fn(method)
                    if impl_is_generic || !method.sig.generics.params.is_empty()
            )
        });
        let namespace_trait = item
            .trait_
            .as_ref()
            .and_then(|(_, path, _)| path.segments.last())
            .is_some_and(|segment| {
                matches!(
                    ident_key(&segment.ident).as_str(),
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
        member_template || namespace_trait
    }

    fn reject_hoisted_local_collisions(&mut self, block: &syn::Block) {
        if self.error.is_some() {
            return;
        }
        let leaves = self.colliding_leaf_names();
        if leaves.is_empty() {
            return;
        }

        struct TypeTailCollector {
            names: BTreeSet<String>,
        }
        impl<'ast> Visit<'ast> for TypeTailCollector {
            fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
                if path.qself.is_none()
                    && let Some(segment) = path.path.segments.last()
                {
                    self.names.insert(ident_key(&segment.ident));
                }
                syn::visit::visit_type_path(self, path);
            }
        }

        let mut impl_hoisted_names = BTreeSet::new();
        for statement in &block.stmts {
            let syn::Stmt::Item(Item::Impl(item_impl)) = statement else {
                continue;
            };
            if !Self::local_impl_forces_namespace_hoist(item_impl) {
                continue;
            }
            if let Some(target) = Self::local_impl_target_name(item_impl) {
                impl_hoisted_names.insert(target);
            }
            if item_impl.trait_.is_some() {
                let mut collector = TypeTailCollector {
                    names: BTreeSet::new(),
                };
                for member in &item_impl.items {
                    match member {
                        syn::ImplItem::Type(associated) => {
                            collector.visit_type(&associated.ty)
                        }
                        syn::ImplItem::Fn(method) => {
                            collector.visit_signature(&method.sig)
                        }
                        _ => {}
                    }
                }
                impl_hoisted_names.extend(collector.names);
            }
        }

        let mut hoisted_names = BTreeSet::new();
        for statement in &block.stmts {
            let syn::Stmt::Item(item) = statement else {
                continue;
            };
            let name = match item {
                Item::Struct(item_struct) => {
                    let generic = item_struct.generics.params.iter().any(|parameter| {
                        matches!(
                            parameter,
                            syn::GenericParam::Type(_) | syn::GenericParam::Const(_)
                        )
                    });
                    (generic || impl_hoisted_names.contains(&ident_key(&item_struct.ident)))
                        .then(|| ident_key(&item_struct.ident))
                }
                Item::Enum(item_enum)
                    if impl_hoisted_names.contains(&ident_key(&item_enum.ident)) =>
                {
                    Some(ident_key(&item_enum.ident))
                }
                Item::Type(item_type) => item_type
                    .generics
                    .params
                    .iter()
                    .any(|parameter| {
                        matches!(
                            parameter,
                            syn::GenericParam::Type(_) | syn::GenericParam::Const(_)
                        )
                    })
                    .then(|| ident_key(&item_type.ident)),
                _ => None,
            };
            if let Some(name) = name {
                if cpp_name_set_contains(&leaves, &name) {
                    self.fail(format!(
                        "a block-local type `{name}` that code generation namespace-hoists or otherwise cannot safely lower at block scope and whose C++ spelling collides with a flat sibling leaf"
                    ));
                    return;
                }
                hoisted_names.insert(name);
            }
        }

        if hoisted_names.is_empty() {
            return;
        }
        struct PathCollector {
            names: BTreeSet<String>,
        }
        impl<'ast> Visit<'ast> for PathCollector {
            fn visit_path(&mut self, path: &'ast syn::Path) {
                for segment in &path.segments {
                    self.names.insert(ident_key(&segment.ident));
                }
                syn::visit::visit_path(self, path);
            }
        }
        let mut referenced = PathCollector {
            names: BTreeSet::new(),
        };
        for statement in &block.stmts {
            let syn::Stmt::Item(item) = statement else {
                continue;
            };
            let belongs_to_hoisted_type = match item {
                Item::Struct(item_struct) => {
                    hoisted_names.contains(&ident_key(&item_struct.ident))
                }
                Item::Enum(item_enum) => hoisted_names.contains(&ident_key(&item_enum.ident)),
                Item::Type(item_type) => hoisted_names.contains(&ident_key(&item_type.ident)),
                Item::Impl(item_impl) => Self::local_impl_target_name(item_impl)
                    .is_some_and(|name| hoisted_names.contains(&name)),
                _ => false,
            };
            if belongs_to_hoisted_type {
                referenced.visit_item(item);
            }
        }
        for statement in &block.stmts {
            let syn::Stmt::Item(Item::Static(item_static)) = statement else {
                continue;
            };
            let name = ident_key(&item_static.ident);
            if referenced.names.contains(&name) && cpp_name_set_contains(&leaves, &name) {
                self.fail(format!(
                    "a block-local static `{name}` that accompanies a namespace-hoisted local type and whose C++ spelling collides with a flat sibling leaf"
                ));
                return;
            }
        }
    }
}

impl<'ast> Visit<'ast> for FlatImportCrateReferenceAudit<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        if self.error.is_some() {
            return;
        }
        if self.namespace_depth == 0
            && self.block_depth == 0
            && let Some((ident, kind)) = item_namespace_name(item)
            && self.colliding_leaf_names().contains(
                &crate::codegen::escape_cpp_keyword(&ident_key(ident)),
            )
            && !self.is_exact_provider_leaf_function(item)
        {
            self.fail(format!(
                "a namespace-emitted {kind} whose name collides with a flat sibling leaf: `{}`",
                item.to_token_stream()
            ));
            return;
        }
        syn::visit::visit_item(self, item);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        for attr in &item.attrs {
            self.visit_attribute(attr);
        }
        if self.error.is_some() {
            return;
        }
        if let Some((_, nested)) = &item.content {
            self.current_module.push(ident_key(&item.ident));
            self.namespace_depth += 1;
            for item in nested {
                self.visit_item(item);
                if self.error.is_some() {
                    break;
                }
            }
            self.namespace_depth -= 1;
            self.current_module.pop();
        }
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if self.marked_use_is_authorized(item) {
            return;
        }
        for attr in &item.attrs {
            self.visit_attribute(attr);
        }
        if self.error.is_some() {
            return;
        }
        if use_tree_aliases_relative_module_root(&item.tree) {
            self.fail(format!(
                "a crate/self/super namespace-root alias while flat sibling imports exist: `{}`",
                item.to_token_stream()
            ));
            return;
        }
        if self.namespace_depth == 0 && self.block_depth == 0 {
            let leaves = self.colliding_leaf_names();
            if !leaves.is_empty() && use_tree_contains_glob(&item.tree) {
                self.fail(format!(
                    "a namespace-emitted glob import whose bindings cannot be proven disjoint from flat sibling leaves: `{}`",
                    item.to_token_stream()
                ));
                return;
            }
            let mut bindings = Vec::new();
            collect_use_leaf_paths(&item.tree, &mut Vec::new(), &mut bindings);
            if let Some((_, binding)) = bindings
                .into_iter()
                .find(|(_, binding)| {
                    leaves.contains(&crate::codegen::escape_cpp_keyword(binding))
                })
            {
                self.fail(format!(
                    "a namespace-emitted use binding `{binding}` that collides with a flat sibling leaf: `{}`",
                    item.to_token_stream()
                ));
                return;
            }
        }
        let mut sources = Vec::new();
        collect_use_source_paths(&item.tree, &mut Vec::new(), &mut sources);
        for source in sources {
            let canonical = canonical_crate_path_segments(
                &self.current_module,
                &source,
                item.leading_colon.is_some(),
            );
            if self.use_source_is_forbidden(&canonical) {
                self.fail(format!(
                    "an unmarked import of a flat sibling provider, marked consumer ancestor, or leaf: `{}`",
                    item.to_token_stream()
                ));
                return;
            }
        }
    }

    fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
        if ident_key(&item.ident) == "self" {
            self.fail(format!(
                "an `extern crate self` namespace-root alias while flat sibling imports exist: `{}`",
                item.to_token_stream()
            ));
            return;
        }
        syn::visit::visit_item_extern_crate(self, item);
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if self.namespace_depth == 0 && self.block_depth == 0 {
            self.reject_hoisted_local_collisions(&item.block);
        }
        if self.error.is_none() {
            syn::visit::visit_item_fn(self, item);
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if self.namespace_depth == 0 && self.block_depth == 0 {
            self.reject_hoisted_local_collisions(&item.block);
        }
        if self.error.is_none() {
            syn::visit::visit_impl_item_fn(self, item);
        }
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if self.namespace_depth == 0
            && self.block_depth == 0
            && let Some(block) = &item.default
        {
            self.reject_hoisted_local_collisions(block);
        }
        if self.error.is_none() {
            syn::visit::visit_trait_item_fn(self, item);
        }
    }

    fn visit_foreign_item_fn(&mut self, item: &'ast syn::ForeignItemFn) {
        if self.namespace_depth == 0
            && self.block_depth == 0
            && self
                .colliding_leaf_names()
                .contains(&crate::codegen::escape_cpp_keyword(
                    &ident_key(&item.sig.ident),
                ))
        {
            self.fail(format!(
                "a namespace-emitted foreign function whose name collides with a flat sibling leaf: `{}`",
                item.to_token_stream()
            ));
            return;
        }
        syn::visit::visit_foreign_item_fn(self, item);
    }

    fn visit_foreign_item_static(&mut self, item: &'ast syn::ForeignItemStatic) {
        if self.namespace_depth == 0
            && self.block_depth == 0
            && self
                .colliding_leaf_names()
                .contains(&crate::codegen::escape_cpp_keyword(
                    &ident_key(&item.ident),
                ))
        {
            self.fail(format!(
                "a namespace-emitted foreign static whose name collides with a flat sibling leaf: `{}`",
                item.to_token_stream()
            ));
            return;
        }
        syn::visit::visit_foreign_item_static(self, item);
    }

    fn visit_foreign_item_type(&mut self, item: &'ast syn::ForeignItemType) {
        if self.namespace_depth == 0
            && self.block_depth == 0
            && self
                .colliding_leaf_names()
                .contains(&crate::codegen::escape_cpp_keyword(
                    &ident_key(&item.ident),
                ))
        {
            self.fail(format!(
                "a namespace-emitted foreign type whose name collides with a flat sibling leaf: `{}`",
                item.to_token_stream()
            ));
            return;
        }
        syn::visit::visit_foreign_item_type(self, item);
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        if self.error.is_some() || path.segments.is_empty() {
            return;
        }
        let semantic = path
            .segments
            .iter()
            .map(|segment| ident_key(&segment.ident))
            .collect::<Vec<_>>();
        let unqualified = path.leading_colon.is_none() && semantic.len() == 1;
        let canonical = canonical_crate_path_segments(
            &self.current_module,
            &semantic,
            path.leading_colon.is_some(),
        );
        if self.path_is_forbidden(&canonical, unqualified) {
            self.fail(format!(
                "a qualified reference to a flat sibling provider leaf: `{}`",
                path.to_token_stream()
            ));
            return;
        }
        syn::visit::visit_path(self, path);
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.block_depth += 1;
        syn::visit::visit_block(self, block);
        self.block_depth -= 1;
    }

    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        if self.error.is_some()
            || attr.path().is_ident("doc")
            || attribute_mentions_flat_import_marker(attr)
        {
            return;
        }
        let leaves = self.opaque_leaf_names();
        if token_stream_mentions_cpp_names(attr.meta.to_token_stream(), &leaves) {
            self.fail(format!(
                "opaque attribute metadata that mentions a flat sibling leaf: `{}`",
                attr.meta.to_token_stream()
            ));
        }
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if self.error.is_some() {
            return;
        }
        let leaves = self.opaque_leaf_names();
        if mac
            .path
            .segments
            .iter()
            .any(|segment| cpp_name_set_contains(&leaves, &ident_key(&segment.ident)))
            || token_stream_mentions_cpp_names(mac.tokens.clone(), &leaves)
        {
            self.fail(format!(
                "opaque macro syntax that mentions a flat sibling leaf: `{}`",
                mac.to_token_stream()
            ));
            return;
        }
        syn::visit::visit_macro(self, mac);
    }
}

fn validate_flat_import_crate_references(
    file: &syn::File,
    base: &ModulePath,
    rules: &FlatImportCrateRules,
) -> Result<(), String> {
    if rules.is_empty() {
        return Ok(());
    }
    let mut audit = FlatImportCrateReferenceAudit {
        rules,
        current_module: base.0.clone(),
        namespace_depth: 0,
        block_depth: 0,
        error: None,
    };
    audit.visit_file(file);
    match audit.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

struct FlatImportBindingAudit<'a> {
    leaves: &'a BTreeSet<String>,
    module: &'a ModulePath,
    error: Option<String>,
}

impl FlatImportBindingAudit<'_> {
    fn reject(&mut self, name: &str, kind: &str) {
        let cpp_name = crate::codegen::escape_cpp_keyword(name);
        if self.error.is_none() && self.leaves.contains(&cpp_name) {
            self.error = Some(format!(
                "cpp_import_namespace leaf `{cpp_name}` may not be shadowed by {kind} with the same C++ spelling in Rust module `{}`",
                self.module.0.join("::")
            ));
        }
    }
}

impl<'ast> Visit<'ast> for FlatImportBindingAudit<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        if self.error.is_some() {
            return;
        }
        if let Item::Use(item_use) = item {
            if parse_flat_import_use(item_use, self.module)
                .ok()
                .flatten()
                .is_some()
            {
                return;
            }
            if use_tree_contains_glob(&item_use.tree) {
                self.error = Some(format!(
                    "cpp_import_namespace rejects an ordinary glob import in Rust module `{}` because it could shadow an imported leaf",
                    self.module.0.join("::")
                ));
                return;
            }
            let mut bindings = Vec::new();
            collect_use_leaf_paths(&item_use.tree, &mut Vec::new(), &mut bindings);
            for (_, binding) in bindings {
                self.reject(&binding, "another use binding");
            }
            return;
        }
        if let Some((ident, kind)) = item_namespace_name(item) {
            self.reject(&ident_key(ident), kind);
        } else if let Item::Macro(item_macro) = item
            && let Some(ident) = &item_macro.ident
        {
            self.reject(&ident_key(ident), "a macro item");
        }
        if self.error.is_some() || matches!(item, Item::Mod(_)) {
            return;
        }
        syn::visit::visit_item(self, item);
    }

    fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
        self.reject(&ident_key(&pattern.ident), "a lexical binding");
        if self.error.is_none() {
            syn::visit::visit_pat_ident(self, pattern);
        }
    }

    fn visit_generic_param(&mut self, parameter: &'ast syn::GenericParam) {
        match parameter {
            syn::GenericParam::Const(parameter) => {
                self.reject(&ident_key(&parameter.ident), "a const generic")
            }
            syn::GenericParam::Type(parameter) => {
                self.reject(&ident_key(&parameter.ident), "a type generic")
            }
            syn::GenericParam::Lifetime(_) => {}
        }
        if self.error.is_none() {
            syn::visit::visit_generic_param(self, parameter);
        }
    }

    fn visit_foreign_item_fn(&mut self, item: &'ast syn::ForeignItemFn) {
        self.reject(&ident_key(&item.sig.ident), "a foreign function");
        if self.error.is_none() {
            syn::visit::visit_foreign_item_fn(self, item);
        }
    }

    fn visit_foreign_item_static(&mut self, item: &'ast syn::ForeignItemStatic) {
        self.reject(&ident_key(&item.ident), "a foreign static");
        if self.error.is_none() {
            syn::visit::visit_foreign_item_static(self, item);
        }
    }

    fn visit_foreign_item_type(&mut self, item: &'ast syn::ForeignItemType) {
        self.reject(&ident_key(&item.ident), "a foreign type");
        if self.error.is_none() {
            syn::visit::visit_foreign_item_type(self, item);
        }
    }
}

fn validate_flat_import_module_bindings(
    items: &[Item],
    module: &ModulePath,
    leaves_by_module: &BTreeMap<ModulePath, BTreeSet<String>>,
) -> Result<(), String> {
    if let Some(leaves) = leaves_by_module.get(module) {
        let mut audit = FlatImportBindingAudit {
            leaves,
            module,
            error: None,
        };
        for item in items {
            audit.visit_item(item);
            if let Some(error) = audit.error {
                return Err(error);
            }
        }
    }
    for item in items {
        if let Item::Mod(item_mod) = item
            && let Some((_, nested)) = &item_mod.content
        {
            let mut nested_path = module.0.clone();
            nested_path.push(ident_key(&item_mod.ident));
            validate_flat_import_module_bindings(
                nested,
                &ModulePath(nested_path),
                leaves_by_module,
            )?;
        }
    }
    Ok(())
}

fn validate_flat_import_binding_surface(
    file: &syn::File,
    contracts: &CppAbiContracts,
) -> Result<(), String> {
    let leaves_by_module = flat_import_leaves_by_module(contracts)?;
    validate_flat_import_module_bindings(
        &file.items,
        &ModulePath(Vec::new()),
        &leaves_by_module,
    )
}

struct FlatImportOpaqueAudit<'a> {
    leaves: &'a BTreeSet<String>,
    error: Option<String>,
}

impl<'ast> Visit<'ast> for FlatImportOpaqueAudit<'_> {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        if self.error.is_some()
            || attr.path().is_ident("doc")
            || attribute_mentions_flat_import_marker(attr)
        {
            return;
        }
        if token_stream_mentions_cpp_names(attr.meta.to_token_stream(), self.leaves) {
            self.error = Some(format!(
                "opaque attribute metadata cannot mention a cpp_import_namespace leaf; found `{}`",
                attr.meta.to_token_stream()
            ));
        }
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if self.error.is_some() {
            return;
        }
        if mac
            .path
            .segments
            .iter()
            .any(|segment| cpp_name_set_contains(self.leaves, &ident_key(&segment.ident)))
            || token_stream_mentions_cpp_names(mac.tokens.clone(), self.leaves)
        {
            self.error = Some(format!(
                "opaque macro syntax cannot mention a cpp_import_namespace leaf; found `{}`",
                mac.to_token_stream()
            ));
        }
    }
}

fn validate_flat_import_opaque_surface(
    file: &syn::File,
    contracts: &CppAbiContracts,
) -> Result<(), String> {
    let leaves = contracts
        .flat_imports
        .values()
        .flat_map(|contract| contract.key.leaves.iter().cloned())
        .collect::<BTreeSet<_>>();
    if leaves.is_empty() {
        return Ok(());
    }
    let mut audit = FlatImportOpaqueAudit {
        leaves: &leaves,
        error: None,
    };
    audit.visit_file(file);
    match audit.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn validate_lowering_surface(file: &syn::File, contracts: &CppAbiContracts) -> Result<(), String> {
    validate_cpp_abi_file_attrs(&file.attrs, "source file")?;
    for alias in contracts.aliases.values() {
        if !is_exact_simple_type(&alias.element, "f64") {
            return Err(format!(
                "cpp_abi_alias `{}` currently supports only exact Vec<f64>",
                alias.name
            ));
        }
    }
    validate_flat_import_binding_surface(file, contracts)?;
    validate_flat_import_opaque_surface(file, contracts)?;
    validate_reserved_imports_and_macros(file, contracts)?;
    validate_local_callable_shadowing(&file.items, &ModulePath(Vec::new()), contracts)?;
    validate_facade_parameter_escaping(&file.items, &ModulePath(Vec::new()), contracts)?;
    validate_projected_cpp_name_collisions(file, contracts)?;
    validate_callable_use_contexts(&file.items, &ModulePath(Vec::new()), contracts)?;
    validate_alias_semantic_isolation(file, contracts)?;
    validate_module_lowering_surface(&file.items, &ModulePath(Vec::new()), contracts)
}

fn reserved_contract_names(contracts: &CppAbiContracts) -> BTreeSet<String> {
    let mut names = contracts
        .aliases
        .keys()
        .map(|(_, name)| name.clone())
        .collect::<BTreeSet<_>>();
    for key in contracts.callables.keys() {
        match key {
            CallableKey::Free { name, .. } | CallableKey::InherentStatic { name, .. } => {
                names.insert(name.clone());
            }
        }
    }
    names
}

fn token_stream_mentions_names(tokens: proc_macro2::TokenStream, names: &BTreeSet<String>) -> bool {
    tokens.into_iter().any(|token| match token {
        proc_macro2::TokenTree::Ident(ident) => names.contains(&ident_key(&ident)),
        proc_macro2::TokenTree::Group(group) => token_stream_mentions_names(group.stream(), names),
        proc_macro2::TokenTree::Literal(_) | proc_macro2::TokenTree::Punct(_) => false,
    })
}

fn cpp_name_set_contains(names: &BTreeSet<String>, rust_name: &str) -> bool {
    names.contains(&crate::codegen::escape_cpp_keyword(rust_name))
}

fn token_stream_mentions_cpp_names(
    tokens: proc_macro2::TokenStream,
    names: &BTreeSet<String>,
) -> bool {
    tokens.into_iter().any(|token| match token {
        proc_macro2::TokenTree::Ident(ident) => {
            cpp_name_set_contains(names, &ident_key(&ident))
        }
        proc_macro2::TokenTree::Group(group) => {
            token_stream_mentions_cpp_names(group.stream(), names)
        }
        proc_macro2::TokenTree::Literal(_) | proc_macro2::TokenTree::Punct(_) => false,
    })
}

fn use_tree_mentions_names(tree: &syn::UseTree, names: &BTreeSet<String>) -> bool {
    match tree {
        syn::UseTree::Path(path) => {
            names.contains(&ident_key(&path.ident)) || use_tree_mentions_names(&path.tree, names)
        }
        syn::UseTree::Name(name) => names.contains(&ident_key(&name.ident)),
        syn::UseTree::Rename(rename) => {
            names.contains(&ident_key(&rename.ident)) || names.contains(&ident_key(&rename.rename))
        }
        syn::UseTree::Glob(_) => true,
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .any(|item| use_tree_mentions_names(item, names)),
    }
}

fn local_contract_names(
    module: &ModulePath,
    contracts: &CppAbiContracts,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut free = BTreeSet::new();
    let mut owners = BTreeSet::new();
    for key in contracts.callables.keys() {
        match key {
            CallableKey::Free {
                module: owner_module,
                name,
            } if owner_module == module => {
                free.insert(name.clone());
            }
            CallableKey::InherentStatic {
                module: owner_module,
                owner,
                ..
            } if owner_module == module => {
                owners.insert(owner.clone());
            }
            _ => {}
        }
    }
    (free, owners)
}

fn validate_local_callable_shadowing(
    items: &[Item],
    module: &ModulePath,
    contracts: &CppAbiContracts,
) -> Result<(), String> {
    let (local_free, local_owners) = local_contract_names(module, contracts);
    if !local_free.is_empty() || !local_owners.is_empty() {
        for item in items {
            match item {
                Item::Fn(function) => {
                    let mut audit = LocalCallableShadowAudit::new(&local_free, &local_owners);
                    audit.visit_signature(&function.sig);
                    audit.visit_block(&function.block);
                    audit.finish()?;
                }
                Item::Impl(implementation) => {
                    for impl_item in &implementation.items {
                        if let syn::ImplItem::Fn(method) = impl_item {
                            let mut audit =
                                LocalCallableShadowAudit::new(&local_free, &local_owners);
                            audit.visit_signature(&method.sig);
                            audit.visit_block(&method.block);
                            audit.finish()?;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    for item in items {
        if let Item::Mod(item_mod) = item
            && let Some((_, nested)) = &item_mod.content
        {
            let mut path = module.0.clone();
            path.push(ident_key(&item_mod.ident));
            validate_local_callable_shadowing(nested, &ModulePath(path), contracts)?;
        }
    }
    Ok(())
}

struct LocalCallableShadowAudit<'a> {
    local_free: &'a BTreeSet<String>,
    local_owners: &'a BTreeSet<String>,
    error: Option<String>,
}

impl<'a> LocalCallableShadowAudit<'a> {
    fn new(local_free: &'a BTreeSet<String>, local_owners: &'a BTreeSet<String>) -> Self {
        Self {
            local_free,
            local_owners,
            error: None,
        }
    }

    fn reject_value(&mut self, name: &str, kind: &str) {
        if self.error.is_none() && self.local_free.contains(name) {
            self.error = Some(format!(
                "local {kind} `{name}` shadows a same-module cpp_abi callable"
            ));
        }
    }

    fn reject_owner(&mut self, name: &str, kind: &str) {
        if self.error.is_none() && self.local_owners.contains(name) {
            self.error = Some(format!(
                "local {kind} `{name}` shadows a same-module cpp_abi method owner"
            ));
        }
    }

    fn finish(self) -> Result<(), String> {
        match self.error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl<'ast> Visit<'ast> for LocalCallableShadowAudit<'_> {
    fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
        self.reject_value(&ident_key(&pattern.ident), "binding");
        if self.error.is_none() {
            syn::visit::visit_pat_ident(self, pattern);
        }
    }

    fn visit_generic_param(&mut self, parameter: &'ast syn::GenericParam) {
        match parameter {
            syn::GenericParam::Const(parameter) => {
                self.reject_value(&ident_key(&parameter.ident), "const generic parameter")
            }
            syn::GenericParam::Type(parameter) => {
                self.reject_owner(&ident_key(&parameter.ident), "type parameter")
            }
            syn::GenericParam::Lifetime(_) => {}
        }
        if self.error.is_none() {
            syn::visit::visit_generic_param(self, parameter);
        }
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.reject_value(&ident_key(&item.sig.ident), "function item");
        if self.error.is_none() {
            syn::visit::visit_item_fn(self, item);
        }
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        self.reject_value(&ident_key(&item.ident), "const item");
        if self.error.is_none() {
            syn::visit::visit_item_const(self, item);
        }
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        self.reject_value(&ident_key(&item.ident), "static item");
        if self.error.is_none() {
            syn::visit::visit_item_static(self, item);
        }
    }

    fn visit_foreign_item_fn(&mut self, item: &'ast syn::ForeignItemFn) {
        self.reject_value(&ident_key(&item.sig.ident), "foreign function");
        if self.error.is_none() {
            syn::visit::visit_foreign_item_fn(self, item);
        }
    }

    fn visit_foreign_item_static(&mut self, item: &'ast syn::ForeignItemStatic) {
        self.reject_value(&ident_key(&item.ident), "foreign static");
        if self.error.is_none() {
            syn::visit::visit_foreign_item_static(self, item);
        }
    }

    fn visit_foreign_item_type(&mut self, item: &'ast syn::ForeignItemType) {
        self.reject_owner(&ident_key(&item.ident), "foreign type");
        if self.error.is_none() {
            syn::visit::visit_foreign_item_type(self, item);
        }
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        let name = ident_key(&item.ident);
        self.reject_owner(&name, "struct");
        if matches!(item.fields, syn::Fields::Unit | syn::Fields::Unnamed(_)) {
            self.reject_value(&name, "tuple/unit struct constructor");
        }
        if self.error.is_none() {
            syn::visit::visit_item_struct(self, item);
        }
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        self.reject_owner(&ident_key(&item.ident), "enum");
        if self.error.is_none() {
            syn::visit::visit_item_enum(self, item);
        }
    }

    fn visit_item_union(&mut self, item: &'ast syn::ItemUnion) {
        self.reject_owner(&ident_key(&item.ident), "union");
        if self.error.is_none() {
            syn::visit::visit_item_union(self, item);
        }
    }

    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        self.reject_owner(&ident_key(&item.ident), "type alias");
        if self.error.is_none() {
            syn::visit::visit_item_type(self, item);
        }
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.reject_owner(&ident_key(&item.ident), "trait");
        if self.error.is_none() {
            syn::visit::visit_item_trait(self, item);
        }
    }

    fn visit_item_trait_alias(&mut self, item: &'ast syn::ItemTraitAlias) {
        self.reject_owner(&ident_key(&item.ident), "trait alias");
        if self.error.is_none() {
            syn::visit::visit_item_trait_alias(self, item);
        }
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        self.reject_owner(&ident_key(&item.ident), "module");
        if self.error.is_none() {
            syn::visit::visit_item_mod(self, item);
        }
    }

    fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
        let name = item
            .rename
            .as_ref()
            .map(|(_, rename)| ident_key(rename))
            .unwrap_or_else(|| ident_key(&item.ident));
        self.reject_owner(&name, "extern crate binding");
        if self.error.is_none() {
            syn::visit::visit_item_extern_crate(self, item);
        }
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        let mut names = self.local_free.clone();
        names.extend(self.local_owners.iter().cloned());
        if self.error.is_none() && use_tree_mentions_names(&item.tree, &names) {
            self.error = Some(format!(
                "local use item can shadow a cpp_abi callable or method owner: `{}`",
                item.to_token_stream()
            ));
        }
    }
}

#[derive(Clone, Debug)]
struct ProjectedCppDecl {
    origin: String,
    adapter_related: bool,
}

type ProjectedNamespaceKey = (Vec<String>, String);
type ProjectedMemberKey = (Vec<String>, String, String);

#[derive(Default)]
struct ProjectedCppCensus {
    namespace: BTreeMap<ProjectedNamespaceKey, Vec<ProjectedCppDecl>>,
    members: BTreeMap<ProjectedMemberKey, Vec<ProjectedCppDecl>>,
}

impl ProjectedCppCensus {
    fn namespace_decl(
        &mut self,
        scope: &[String],
        raw_name: &str,
        source: &str,
        kind: &str,
        adapter_related: bool,
    ) {
        self.namespace
            .entry((scope.to_vec(), crate::codegen::escape_cpp_keyword(raw_name)))
            .or_default()
            .push(ProjectedCppDecl {
                origin: format!("{source}: {kind} `{raw_name}`"),
                adapter_related,
            });
    }

    fn member_decl(
        &mut self,
        scope: &[String],
        raw_owner: &str,
        raw_name: &str,
        source: &str,
        kind: &str,
        adapter_related: bool,
        method_escape: bool,
    ) {
        let owner = crate::codegen::escape_cpp_keyword(raw_owner);
        let name = if method_escape {
            crate::codegen::CodeGen::escape_cpp_method_name(raw_name)
        } else {
            crate::codegen::escape_cpp_keyword(raw_name)
        };
        self.members
            .entry((scope.to_vec(), owner, name))
            .or_default()
            .push(ProjectedCppDecl {
                origin: format!("{source}: {kind} `{raw_owner}::{raw_name}`"),
                adapter_related,
            });
    }

    fn validate(self) -> Result<(), String> {
        for ((scope, name), declarations) in self.namespace {
            reject_projected_collision(&scope, None, &name, declarations)?;
        }
        for ((scope, owner, name), declarations) in self.members {
            reject_projected_collision(&scope, Some(&owner), &name, declarations)?;
        }
        Ok(())
    }
}

fn reject_projected_collision(
    scope: &[String],
    owner: Option<&str>,
    name: &str,
    declarations: Vec<ProjectedCppDecl>,
) -> Result<(), String> {
    if !declarations.iter().any(|decl| decl.adapter_related) {
        return Ok(());
    }
    let origins = declarations
        .into_iter()
        .map(|decl| decl.origin)
        .collect::<BTreeSet<_>>();
    if origins.len() <= 1 {
        return Ok(());
    }
    let mut projected = scope.to_vec();
    if let Some(owner) = owner {
        projected.push(owner.to_string());
    }
    projected.push(name.to_string());
    Err(format!(
        "cpp_abi declarations collide after exact C++ identifier escaping at `{}`: {}",
        projected.join("::"),
        origins.into_iter().collect::<Vec<_>>().join("; ")
    ))
}

fn use_tree_bound_names(tree: &syn::UseTree, out: &mut Vec<String>) {
    match tree {
        syn::UseTree::Path(path) => use_tree_bound_names(&path.tree, out),
        syn::UseTree::Name(name) => out.push(name.ident.to_string()),
        syn::UseTree::Rename(rename) => out.push(rename.rename.to_string()),
        syn::UseTree::Group(group) => {
            for item in &group.items {
                use_tree_bound_names(item, out);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

fn item_namespace_name(item: &Item) -> Option<(&proc_macro2::Ident, &'static str)> {
    match item {
        Item::Const(item) => Some((&item.ident, "const")),
        Item::Enum(item) => Some((&item.ident, "enum")),
        Item::ExternCrate(item) => Some((
            item.rename
                .as_ref()
                .map(|(_, rename)| rename)
                .unwrap_or(&item.ident),
            "extern crate",
        )),
        Item::Fn(item) => Some((&item.sig.ident, "free function")),
        Item::Mod(item) => Some((&item.ident, "module")),
        Item::Static(item) => Some((&item.ident, "static")),
        Item::Struct(item) => Some((&item.ident, "struct")),
        Item::Trait(item) => Some((&item.ident, "trait")),
        Item::TraitAlias(item) => Some((&item.ident, "trait alias")),
        Item::Type(item) => Some((&item.ident, "type alias")),
        Item::Union(item) => Some((&item.ident, "union")),
        _ => None,
    }
}

fn collect_projected_cpp_names(
    items: &[Item],
    rust_module: &ModulePath,
    cpp_scope: &[String],
    contracts: &CppAbiContracts,
    source: &str,
    crate_base: Option<&ModulePath>,
    global_providers: Option<&BTreeSet<ModulePath>>,
    census: &mut ProjectedCppCensus,
) {
    let adapted_owners = contracts
        .callables
        .keys()
        .filter_map(|key| match key {
            CallableKey::InherentStatic { module, owner, .. } if module == rust_module => {
                Some(owner.clone())
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();

    for item in items {
        if let Some((ident, kind)) = item_namespace_name(item) {
            let canonical = ident_key(ident);
            let adapter_related = match item {
                Item::Fn(_) => contracts.callables.contains_key(&CallableKey::Free {
                    module: rust_module.clone(),
                    name: canonical.clone(),
                }),
                Item::Type(_) => contracts
                    .aliases
                    .contains_key(&(rust_module.clone(), canonical.clone())),
                Item::Struct(_) => adapted_owners.contains(&canonical),
                Item::Mod(_) => {
                    let mut nested = rust_module.0.clone();
                    nested.push(canonical.clone());
                    let nested = ModulePath(nested);
                    let local_provider = module_contains_contract(&nested, contracts);
                    let global_provider =
                        crate_base
                            .zip(global_providers)
                            .is_some_and(|(base, providers)| {
                                let mut global = base.0.clone();
                                global.extend(nested.0.iter().cloned());
                                providers
                                    .iter()
                                    .any(|provider| provider.0.starts_with(&global))
                            });
                    local_provider || global_provider
                }
                _ => false,
            };
            census.namespace_decl(cpp_scope, &ident.to_string(), source, kind, adapter_related);
        }

        match item {
            Item::Use(item_use) => {
                let mut names = Vec::new();
                use_tree_bound_names(&item_use.tree, &mut names);
                for name in names {
                    census.namespace_decl(cpp_scope, &name, source, "use binding", false);
                }
            }
            Item::ForeignMod(foreign) => {
                for item in &foreign.items {
                    let (ident, kind) = match item {
                        syn::ForeignItem::Fn(item) => (&item.sig.ident, "foreign function"),
                        syn::ForeignItem::Static(item) => (&item.ident, "foreign static"),
                        syn::ForeignItem::Type(item) => (&item.ident, "foreign type"),
                        _ => continue,
                    };
                    census.namespace_decl(cpp_scope, &ident.to_string(), source, kind, false);
                }
            }
            Item::Struct(item_struct) => {
                let owner = item_struct.ident.to_string();
                for field in &item_struct.fields {
                    if let Some(ident) = &field.ident {
                        census.member_decl(
                            cpp_scope,
                            &owner,
                            &ident.to_string(),
                            source,
                            "field",
                            false,
                            false,
                        );
                    }
                }
            }
            Item::Impl(implementation) => {
                let Ok(owner) = simple_impl_owner(implementation) else {
                    continue;
                };
                for item in &implementation.items {
                    let (name, kind, method_escape) = match item {
                        syn::ImplItem::Fn(item) => (&item.sig.ident, "inherent method", true),
                        syn::ImplItem::Const(item) => (&item.ident, "associated const", false),
                        syn::ImplItem::Type(item) => (&item.ident, "associated type", false),
                        _ => continue,
                    };
                    let key = CallableKey::InherentStatic {
                        module: rust_module.clone(),
                        owner: owner.clone(),
                        name: ident_key(name),
                    };
                    census.member_decl(
                        cpp_scope,
                        &owner,
                        &name.to_string(),
                        source,
                        kind,
                        contracts.callables.contains_key(&key),
                        method_escape,
                    );
                }
            }
            Item::Mod(item_mod) => {
                if let Some((_, nested)) = &item_mod.content {
                    let mut nested_rust = rust_module.0.clone();
                    nested_rust.push(ident_key(&item_mod.ident));
                    let mut nested_cpp = cpp_scope.to_vec();
                    nested_cpp.push(crate::codegen::escape_cpp_keyword(
                        &item_mod.ident.to_string(),
                    ));
                    collect_projected_cpp_names(
                        nested,
                        &ModulePath(nested_rust),
                        &nested_cpp,
                        contracts,
                        source,
                        crate_base,
                        global_providers,
                        census,
                    );
                }
            }
            _ => {}
        }
    }
}

fn validate_projected_cpp_name_collisions(
    file: &syn::File,
    contracts: &CppAbiContracts,
) -> Result<(), String> {
    let mut census = ProjectedCppCensus::default();
    collect_projected_cpp_names(
        &file.items,
        &ModulePath(Vec::new()),
        &[],
        contracts,
        "source file",
        None,
        None,
        &mut census,
    );
    census.validate()
}

fn validate_signature_parameter_escaping(
    signature: &syn::Signature,
    label: &str,
) -> Result<(), String> {
    let mut projected = BTreeMap::<String, String>::new();
    for input in &signature.inputs {
        let FnArg::Typed(input) = input else {
            continue;
        };
        let syn::Pat::Ident(pattern) = input.pat.as_ref() else {
            continue;
        };
        let raw = pattern.ident.to_string();
        let cpp = crate::codegen::escape_cpp_keyword(&raw);
        if let Some(previous) = projected.insert(cpp.clone(), raw.clone())
            && previous != raw
        {
            return Err(format!(
                "cpp_abi {label} parameters `{previous}` and `{raw}` both project to C++ identifier `{cpp}`"
            ));
        }
    }
    Ok(())
}

fn validate_facade_parameter_escaping(
    items: &[Item],
    module: &ModulePath,
    contracts: &CppAbiContracts,
) -> Result<(), String> {
    for item in items {
        match item {
            Item::Fn(function) => {
                let key = CallableKey::Free {
                    module: module.clone(),
                    name: ident_key(&function.sig.ident),
                };
                if contracts.callables.contains_key(&key) {
                    validate_signature_parameter_escaping(
                        &function.sig,
                        &format!("facade `{}`", function.sig.ident),
                    )?;
                }
            }
            Item::Impl(implementation) => {
                if let Ok(owner) = simple_impl_owner(implementation) {
                    for item in &implementation.items {
                        let syn::ImplItem::Fn(method) = item else {
                            continue;
                        };
                        let key = CallableKey::InherentStatic {
                            module: module.clone(),
                            owner: owner.clone(),
                            name: ident_key(&method.sig.ident),
                        };
                        if contracts.callables.contains_key(&key) {
                            validate_signature_parameter_escaping(
                                &method.sig,
                                &format!("facade `{}::{}`", owner, method.sig.ident),
                            )?;
                        }
                    }
                }
            }
            Item::Mod(item_mod) => {
                if let Some((_, nested)) = &item_mod.content {
                    let mut path = module.0.clone();
                    path.push(ident_key(&item_mod.ident));
                    validate_facade_parameter_escaping(nested, &ModulePath(path), contracts)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_reserved_imports_and_macros(
    file: &syn::File,
    contracts: &CppAbiContracts,
) -> Result<(), String> {
    let names = reserved_contract_names(contracts);
    let mut audit = ReservedImportMacroAudit {
        names: &names,
        inside_assert_expression: false,
        error: None,
    };
    audit.visit_file(file);
    match audit.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

struct ReservedImportMacroAudit<'a> {
    names: &'a BTreeSet<String>,
    inside_assert_expression: bool,
    error: Option<String>,
}

impl<'ast> Visit<'ast> for ReservedImportMacroAudit<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        if let Item::Verbatim(tokens) = item
            && token_stream_declares_assert_macro(tokens.clone())
        {
            self.error = Some(
                "the macro definition name `assert` is reserved while cpp_abi contracts are present"
                    .to_string(),
            );
            return;
        }
        syn::visit::visit_item(self, item);
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if self.error.is_none() && item_macro_introduces_assert(item) {
            self.error = Some(
                "the macro definition name `assert` is reserved while cpp_abi contracts are present"
                    .to_string(),
            );
            return;
        }
        syn::visit::visit_item_macro(self, item);
    }

    fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
        for attr in &item.attrs {
            self.visit_attribute(attr);
        }
        if self.error.is_none() {
            self.error = Some(format!(
                "`extern crate` bindings are unsupported while cpp_abi contracts are present; found `{}`",
                item.to_token_stream()
            ));
        }
    }

    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        if self.error.is_none() && is_macro_use_attribute(attr) {
            self.error = Some(format!(
                "`#[macro_use]` is unsupported while cpp_abi contracts are present; found `{}`",
                attr.meta.to_token_stream()
            ));
        } else if self.error.is_none()
            && !attr.path().is_ident("doc")
            && !attribute_mentions_marker(attr)
            && token_stream_mentions_names(attr.meta.to_token_stream(), self.names)
        {
            self.error = Some(format!(
                "non-marker attributes cannot mention a cpp_abi callable or alias; found `{}`",
                attr.meta.to_token_stream()
            ));
        }
    }

    fn visit_item_use(&mut self, item_use: &'ast syn::ItemUse) {
        for attr in &item_use.attrs {
            self.visit_attribute(attr);
        }
        if self.error.is_some() {
            return;
        }
        if use_tree_introduces_assert(&item_use.tree) {
            self.error = Some(format!(
                "the imported macro binding `assert` is reserved while cpp_abi contracts are present; found `{}`",
                item_use.to_token_stream()
            ));
        } else if use_tree_aliases_relative_module_root(&item_use.tree) {
            self.error = Some(format!(
                "aliases of `crate`, `self`, or `super` are unsupported while cpp_abi contracts are present; found `{}`",
                item_use.to_token_stream()
            ));
        } else if use_tree_mentions_names(&item_use.tree, self.names) {
            self.error = Some(format!(
                "imports, re-exports, aliases, and glob imports cannot involve cpp_abi names; found `{}`",
                item_use.to_token_stream()
            ));
        }
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if self.error.is_some() {
            return;
        }
        if !self.inside_assert_expression
            && let Some(expression) = parse_admitted_assert_expression(mac)
        {
            if token_stream_mentions_names(mac.tokens.clone(), self.names) {
                self.error = Some(format!(
                    "the admitted `assert!(EXPR[, \"literal\"])` cannot mention a cpp_abi callable or alias; found `{}`",
                    mac.tokens
                ));
                return;
            }

            self.inside_assert_expression = true;
            self.visit_expr(&expression);
            self.inside_assert_expression = false;
            return;
        }
        if self.inside_assert_expression {
            self.error = Some(format!(
                "nested opaque macro `{}` is unsupported inside the admitted `assert!(EXPR[, \"literal\"])`",
                mac.path.to_token_stream()
            ));
            return;
        }
        if token_stream_declares_assert_macro(mac.tokens.clone())
            || token_stream_mentions_assert_identifier(mac.tokens.clone())
        {
            self.error = Some(format!(
                "opaque macro tokens cannot introduce or forward the reserved `assert` binding; found `{}`",
                mac.path.to_token_stream()
            ));
        } else if token_stream_mentions_names(mac.tokens.clone(), self.names) {
            self.error = Some(format!(
                "opaque macro tokens cannot mention a cpp_abi callable or alias; found `{}`",
                mac.path.to_token_stream()
            ));
        }
    }
}

fn validate_callable_use_contexts(
    items: &[Item],
    module: &ModulePath,
    contracts: &CppAbiContracts,
) -> Result<(), String> {
    let callable_names = contracts
        .callables
        .keys()
        .map(|key| match key {
            CallableKey::Free { name, .. } | CallableKey::InherentStatic { name, .. } => {
                name.clone()
            }
        })
        .collect::<BTreeSet<_>>();
    let local_free = contracts
        .callables
        .keys()
        .filter_map(|key| match key {
            CallableKey::Free {
                module: owner_module,
                name,
            } if owner_module == module => Some(name.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();

    for item in items {
        match item {
            Item::Fn(function) => {
                reject_callable_uses_in_signature(&function.sig, &callable_names)?;
                reject_callable_parameter_shadow(&function.sig, &local_free)?;
            }
            Item::Impl(implementation) => {
                let mut header_audit = CallableExprUseAudit {
                    names: &callable_names,
                    error: None,
                };
                header_audit.visit_generics(&implementation.generics);
                header_audit.visit_type(&implementation.self_ty);
                if let Some(error) = header_audit.error {
                    return Err(error);
                }
                for impl_item in &implementation.items {
                    if let syn::ImplItem::Fn(method) = impl_item {
                        reject_callable_uses_in_signature(&method.sig, &callable_names)?;
                        reject_callable_parameter_shadow(&method.sig, &local_free)?;
                    } else {
                        let mut audit = CallableExprUseAudit {
                            names: &callable_names,
                            error: None,
                        };
                        audit.visit_impl_item(impl_item);
                        if let Some(error) = audit.error {
                            return Err(error);
                        }
                    }
                }
            }
            Item::Mod(item_mod) => {
                if let Some((_, nested)) = &item_mod.content {
                    let mut path = module.0.clone();
                    path.push(ident_key(&item_mod.ident));
                    validate_callable_use_contexts(nested, &ModulePath(path), contracts)?;
                }
            }
            _ => {
                let mut audit = CallableExprUseAudit {
                    names: &callable_names,
                    error: None,
                };
                audit.visit_item(item);
                if let Some(error) = audit.error {
                    return Err(error);
                }
            }
        }
    }
    Ok(())
}

fn reject_callable_uses_in_signature(
    signature: &syn::Signature,
    callable_names: &BTreeSet<String>,
) -> Result<(), String> {
    let mut audit = CallableExprUseAudit {
        names: callable_names,
        error: None,
    };
    audit.visit_signature(signature);
    match audit.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn reject_callable_parameter_shadow(
    signature: &syn::Signature,
    local_free: &BTreeSet<String>,
) -> Result<(), String> {
    for arg in &signature.inputs {
        if let FnArg::Typed(arg) = arg {
            let mut audit = ParameterShadowAudit {
                local_free,
                error: None,
            };
            audit.visit_pat(&arg.pat);
            if let Some(error) = audit.error {
                return Err(error);
            }
        }
    }
    Ok(())
}

struct ParameterShadowAudit<'a> {
    local_free: &'a BTreeSet<String>,
    error: Option<String>,
}

impl<'ast> Visit<'ast> for ParameterShadowAudit<'_> {
    fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
        let name = ident_key(&pattern.ident);
        if self.error.is_none() && self.local_free.contains(&name) {
            self.error = Some(format!(
                "parameter `{name}` shadows a cpp_abi callable and prevents sound call resolution"
            ));
        }
    }
}

struct CallableExprUseAudit<'a> {
    names: &'a BTreeSet<String>,
    error: Option<String>,
}

impl<'ast> Visit<'ast> for CallableExprUseAudit<'_> {
    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        if self.error.is_none()
            && path
                .path
                .segments
                .last()
                .is_some_and(|segment| self.names.contains(&ident_key(&segment.ident)))
        {
            self.error = Some(format!(
                "cpp_abi callable `{}` is not supported in this semantic context",
                path.to_token_stream()
            ));
            return;
        }
        syn::visit::visit_expr_path(self, path);
    }
}

/// A marked alias names only the legacy C++ facade type. It must never leak
/// into the Rust semantic program: the helper continues to use the canonical
/// slice type recorded on the adapted parameter. Rejecting every type or
/// expression path containing a marked alias name is deliberately conservative;
/// it also closes associated constructors and qualified cross-module uses
/// without pretending to resolve Rust names before expansion.
fn validate_alias_semantic_isolation(
    file: &syn::File,
    contracts: &CppAbiContracts,
) -> Result<(), String> {
    let alias_names = contracts
        .aliases
        .keys()
        .map(|(_, name)| name.clone())
        .collect::<BTreeSet<_>>();
    if alias_names.is_empty() {
        return Ok(());
    }
    let alias_keys = contracts.aliases.keys().cloned().collect::<BTreeSet<_>>();
    let mut audit = AliasSemanticUseAudit {
        module: ModulePath(Vec::new()),
        alias_keys: &alias_keys,
        alias_names: &alias_names,
        error: None,
    };
    audit.visit_file(file);
    match audit.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

struct AliasSemanticUseAudit<'a> {
    module: ModulePath,
    alias_keys: &'a BTreeSet<(ModulePath, String)>,
    alias_names: &'a BTreeSet<String>,
    error: Option<String>,
}

impl AliasSemanticUseAudit<'_> {
    fn reject_path(&mut self, path: &syn::Path) {
        if self.error.is_none()
            && path
                .segments
                .iter()
                .any(|segment| self.alias_names.contains(&ident_key(&segment.ident)))
        {
            self.error = Some(format!(
                "marked cpp_abi alias `{}` may appear only in adapter metadata, not in semantic Rust types or expressions",
                path.to_token_stream()
            ));
        }
    }
}

impl<'ast> Visit<'ast> for AliasSemanticUseAudit<'_> {
    fn visit_item_mod(&mut self, item_mod: &'ast syn::ItemMod) {
        let Some((_, items)) = &item_mod.content else {
            return;
        };
        let previous = self.module.clone();
        self.module.0.push(ident_key(&item_mod.ident));
        for item in items {
            self.visit_item(item);
            if self.error.is_some() {
                break;
            }
        }
        self.module = previous;
    }

    fn visit_item_type(&mut self, item_type: &'ast syn::ItemType) {
        let key = (self.module.clone(), ident_key(&item_type.ident));
        if !self.alias_keys.contains(&key) {
            syn::visit::visit_item_type(self, item_type);
        }
    }

    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        self.reject_path(&path.path);
        if self.error.is_none() {
            syn::visit::visit_type_path(self, path);
        }
    }

    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        self.reject_path(&path.path);
        if self.error.is_none() {
            syn::visit::visit_expr_path(self, path);
        }
    }
}

fn is_cpp_abi_doc_or_lint_attr(attr: &Attribute) -> bool {
    ["doc", "allow", "warn", "deny", "forbid", "expect"]
        .iter()
        .any(|name| attr.path().is_ident(name))
}

fn validate_cpp_abi_file_attrs(attrs: &[Attribute], context: &str) -> Result<(), String> {
    for attr in attrs {
        if !is_cpp_abi_doc_or_lint_attr(attr) {
            return Err(format!(
                "cpp_abi {context} has an unsupported presence- or environment-changing inner attribute `{}`; only doc and lint-level attributes are supported",
                attr.path().to_token_stream()
            ));
        }
    }
    Ok(())
}

fn validate_cpp_abi_ancestor_attrs(attrs: &[Attribute], context: &str) -> Result<(), String> {
    for attr in attrs {
        if !is_cpp_abi_doc_or_lint_attr(attr) {
            return Err(format!(
                "cpp_abi {context} supports only doc and lint-level attributes; found `{}`",
                attr.path().to_token_stream()
            ));
        }
    }
    Ok(())
}

fn module_contains_contract(module: &ModulePath, contracts: &CppAbiContracts) -> bool {
    contracts
        .aliases
        .keys()
        .any(|(provider, _)| provider.0.starts_with(&module.0))
        || contracts
            .callables
            .keys()
            .any(|key| key_module(key).0.starts_with(&module.0))
}

fn validate_method_provider(
    owner: &str,
    owner_item: Option<&syn::ItemStruct>,
    implementation: &syn::ItemImpl,
    method: &syn::ImplItemFn,
) -> Result<(), String> {
    let owner_item = owner_item.ok_or_else(|| {
        format!(
            "cpp_abi method `{}::{}` requires its struct owner in the same module",
            owner, method.sig.ident
        )
    })?;
    if !is_public(&owner_item.vis)
        || !owner_item.generics.params.is_empty()
        || owner_item.generics.where_clause.is_some()
    {
        return Err(format!(
            "cpp_abi method `{}::{}` requires a public non-generic struct owner",
            owner, method.sig.ident
        ));
    }
    validate_cpp_abi_ancestor_attrs(&owner_item.attrs, &format!("owner `{owner}`"))?;
    if matches!(owner_item.fields, syn::Fields::Unnamed(_)) {
        return Err(format!(
            "cpp_abi method `{}::{}` owner `{owner}` is not supported by out-of-line method scheduling; tuple structs are unsupported",
            owner, method.sig.ident
        ));
    }
    validate_cpp_abi_ancestor_attrs(&implementation.attrs, &format!("impl for `{owner}`"))?;
    if implementation.unsafety.is_some() || implementation.defaultness.is_some() {
        return Err(format!(
            "cpp_abi method `{}::{}` requires an ordinary safe inherent impl",
            owner, method.sig.ident
        ));
    }
    Ok(())
}

fn validate_module_lowering_surface(
    items: &[Item],
    module: &ModulePath,
    contracts: &CppAbiContracts,
) -> Result<(), String> {
    let local_structs: BTreeMap<String, &syn::ItemStruct> = items
        .iter()
        .filter_map(|item| match item {
            Item::Struct(item) => Some((ident_key(&item.ident), item)),
            _ => None,
        })
        .collect();
    for item in items {
        match item {
            Item::Fn(function) => {
                let key = CallableKey::Free {
                    module: module.clone(),
                    name: ident_key(&function.sig.ident),
                };
                if let Some(contract) = contracts.callables.get(&key) {
                    validate_unadapted_signature(&function.sig, contract)?;
                }
            }
            Item::Impl(implementation) => {
                let Ok(owner) = simple_impl_owner(implementation) else {
                    continue;
                };
                let associated_names: BTreeSet<String> = implementation
                    .items
                    .iter()
                    .filter_map(|item| match item {
                        syn::ImplItem::Const(v) => Some(ident_key(&v.ident)),
                        syn::ImplItem::Fn(v) => Some(ident_key(&v.sig.ident)),
                        syn::ImplItem::Type(v) => Some(ident_key(&v.ident)),
                        _ => None,
                    })
                    .collect();
                for impl_item in &implementation.items {
                    let syn::ImplItem::Fn(method) = impl_item else {
                        continue;
                    };
                    let key = CallableKey::InherentStatic {
                        module: module.clone(),
                        owner: owner.clone(),
                        name: ident_key(&method.sig.ident),
                    };
                    if let Some(contract) = contracts.callables.get(&key) {
                        validate_method_provider(
                            &owner,
                            local_structs.get(&owner).copied(),
                            implementation,
                            method,
                        )?;
                        validate_unadapted_signature(&method.sig, contract)?;
                        let mut audit = ImplContextAudit {
                            associated_names: &associated_names,
                            error: None,
                        };
                        audit.visit_block(&method.block);
                        if let Some(error) = audit.error {
                            return Err(format!(
                                "cpp_abi method `{}::{}` {error}",
                                owner, method.sig.ident
                            ));
                        }
                    }
                }
            }
            Item::Mod(item_mod) => {
                if let Some((_, nested)) = &item_mod.content {
                    let mut path = module.0.clone();
                    path.push(ident_key(&item_mod.ident));
                    let nested_module = ModulePath(path);
                    if module_contains_contract(&nested_module, contracts) {
                        if !is_public(&item_mod.vis) {
                            return Err(format!(
                                "cpp_abi provider module `{}` and every ancestor module must be public",
                                nested_module.0.join("::")
                            ));
                        }
                        validate_cpp_abi_ancestor_attrs(
                            &item_mod.attrs,
                            &format!("provider module `{}`", nested_module.0.join("::")),
                        )?;
                    }
                    validate_module_lowering_surface(nested, &nested_module, contracts)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_unadapted_signature(
    sig: &syn::Signature,
    contract: &CallableContract,
) -> Result<(), String> {
    for arg in &sig.inputs {
        let FnArg::Typed(arg) = arg else {
            continue;
        };
        let syn::Pat::Ident(pattern) = arg.pat.as_ref() else {
            continue;
        };
        let name = ident_key(&pattern.ident);
        if name.starts_with("rusty_cpp_abi_") {
            return Err(format!(
                "cpp_abi callable `{}` parameter `{name}` collides with reserved generated local names",
                sig.ident
            ));
        }
        if !contract.params.contains_key(&name) && !is_supported_scalar(&arg.ty) {
            return Err(format!(
                "cpp_abi callable `{}` has unsupported unadapted parameter `{name}`; only scalar parameters are supported",
                sig.ident
            ));
        }
    }
    if contract.returns.is_none() {
        match &sig.output {
            syn::ReturnType::Default => {}
            syn::ReturnType::Type(_, ty)
                if matches!(ty.as_ref(), Type::Tuple(tuple) if tuple.elems.is_empty())
                    || is_supported_scalar(ty) => {}
            _ => {
                return Err(format!(
                    "cpp_abi callable `{}` has unsupported unadapted return type; only unit and scalar returns are supported",
                    sig.ident
                ));
            }
        }
    }
    Ok(())
}

fn is_supported_scalar(ty: &Type) -> bool {
    [
        "bool", "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128",
        "isize", "f32", "f64", "char",
    ]
    .iter()
    .any(|name| is_exact_simple_type(ty, name))
}

fn is_exact_simple_type(ty: &Type, name: &str) -> bool {
    matches!(ty, Type::Path(path) if path.qself.is_none() && is_simple_path(&path.path, name))
}

struct ImplContextAudit<'a> {
    associated_names: &'a BTreeSet<String>,
    error: Option<String>,
}

impl<'ast> Visit<'ast> for ImplContextAudit<'_> {
    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        if self.error.is_none() && path.qself.is_none() {
            let segments: Vec<String> = path
                .path
                .segments
                .iter()
                .map(|s| ident_key(&s.ident))
                .collect();
            if segments.first().is_some_and(|name| name == "Self") {
                self.error = Some("cannot use `Self` in its semantic helper body".to_string());
                return;
            }
            if segments.len() == 1 && self.associated_names.contains(&segments[0]) {
                self.error = Some(format!(
                    "cannot use unqualified associated item `{}` in its semantic helper body",
                    segments[0]
                ));
                return;
            }
        }
        syn::visit::visit_expr_path(self, path);
    }

    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        if self.error.is_none()
            && path.qself.is_none()
            && path
                .path
                .segments
                .first()
                .is_some_and(|segment| ident_key(&segment.ident) == "Self")
        {
            self.error = Some("cannot use `Self` in its semantic helper body".to_string());
            return;
        }
        syn::visit::visit_type_path(self, path);
    }
}

fn rewrite_semantic_calls(
    file: &mut syn::File,
    contracts: &CppAbiContracts,
    helper_names: &BTreeMap<CallableKey, String>,
) -> Result<(), String> {
    let available = helper_names.keys().cloned().collect::<BTreeSet<_>>();
    rewrite_semantic_calls_with_available(
        file,
        contracts,
        helper_names,
        &available,
        &BTreeSet::new(),
    )
    .map(|_| ())
}

fn rewrite_semantic_calls_with_available(
    file: &mut syn::File,
    contracts: &CppAbiContracts,
    helper_names: &BTreeMap<CallableKey, String>,
    available: &BTreeSet<CallableKey>,
    extra_reserved_tails: &BTreeSet<String>,
) -> Result<BTreeSet<CallableKey>, String> {
    let mut used = BTreeSet::new();
    rewrite_module_calls(
        &mut file.items,
        &ModulePath(Vec::new()),
        contracts,
        helper_names,
        available,
        extra_reserved_tails,
        &mut used,
    )?;
    Ok(used)
}

fn rewrite_module_calls(
    items: &mut [Item],
    module: &ModulePath,
    contracts: &CppAbiContracts,
    helper_names: &BTreeMap<CallableKey, String>,
    available: &BTreeSet<CallableKey>,
    extra_reserved_tails: &BTreeSet<String>,
    used: &mut BTreeSet<CallableKey>,
) -> Result<(), String> {
    let mut local_free = BTreeMap::<String, (CallableKey, String)>::new();
    let mut local_methods = BTreeMap::<(String, String), (CallableKey, String)>::new();
    let mut reserved_tails = extra_reserved_tails.clone();
    for (key, helper) in helper_names {
        match key {
            CallableKey::Free {
                module: owner_module,
                name,
            } => {
                reserved_tails.insert(name.clone());
                if owner_module == module && available.contains(key) {
                    local_free.insert(name.clone(), (key.clone(), helper.clone()));
                }
            }
            CallableKey::InherentStatic {
                module: owner_module,
                owner,
                name,
            } => {
                reserved_tails.insert(name.clone());
                if owner_module == module && available.contains(key) {
                    local_methods.insert(
                        (owner.clone(), name.clone()),
                        (key.clone(), helper.clone()),
                    );
                }
            }
        }
    }

    for item in items.iter_mut() {
        match item {
            Item::Fn(function) => {
                let mut rewrite = CallRewrite::new(&local_free, &local_methods, &reserved_tails);
                rewrite.visit_block_mut(&mut function.block);
                used.extend(rewrite.finish()?);
            }
            Item::Impl(implementation) => {
                for impl_item in &mut implementation.items {
                    if let syn::ImplItem::Fn(method) = impl_item {
                        let mut rewrite =
                            CallRewrite::new(&local_free, &local_methods, &reserved_tails);
                        rewrite.visit_block_mut(&mut method.block);
                        used.extend(rewrite.finish()?);
                    }
                }
            }
            Item::Mod(item_mod) => {
                if let Some((_, nested)) = &mut item_mod.content {
                    let mut path = module.0.clone();
                    path.push(ident_key(&item_mod.ident));
                    rewrite_module_calls(
                        nested,
                        &ModulePath(path),
                        contracts,
                        helper_names,
                        available,
                        extra_reserved_tails,
                        used,
                    )?;
                }
            }
            _ => {}
        }
    }
    let _ = contracts;
    Ok(())
}

struct CallRewrite<'a> {
    local_free: &'a BTreeMap<String, (CallableKey, String)>,
    local_methods: &'a BTreeMap<(String, String), (CallableKey, String)>,
    reserved_tails: &'a BTreeSet<String>,
    errors: Vec<String>,
    used: BTreeSet<CallableKey>,
    direct_callee_depth: usize,
}

impl<'a> CallRewrite<'a> {
    fn new(
        local_free: &'a BTreeMap<String, (CallableKey, String)>,
        local_methods: &'a BTreeMap<(String, String), (CallableKey, String)>,
        reserved_tails: &'a BTreeSet<String>,
    ) -> Self {
        Self {
            local_free,
            local_methods,
            reserved_tails,
            errors: Vec::new(),
            used: BTreeSet::new(),
            direct_callee_depth: 0,
        }
    }

    fn finish(self) -> Result<BTreeSet<CallableKey>, String> {
        if self.errors.is_empty() {
            Ok(self.used)
        } else {
            Err(self.errors.join("; "))
        }
    }

    fn resolve_direct(
        &self,
        path: &syn::ExprPath,
    ) -> Result<Option<(CallableKey, String)>, String> {
        if path.qself.is_some() || path.path.leading_colon.is_some() {
            return self.reject_if_reserved_tail(path);
        }
        let segments: Vec<String> = path
            .path
            .segments
            .iter()
            .map(|s| ident_key(&s.ident))
            .collect();
        if segments.len() == 1 {
            if let Some(helper) = self.local_free.get(&segments[0]) {
                return Ok(Some(helper.clone()));
            }
        } else if segments.len() == 2 {
            if let Some(helper) = self
                .local_methods
                .get(&(segments[0].clone(), segments[1].clone()))
            {
                return Ok(Some(helper.clone()));
            }
        }
        self.reject_if_reserved_tail(path)
    }

    fn reject_if_reserved_tail(
        &self,
        path: &syn::ExprPath,
    ) -> Result<Option<(CallableKey, String)>, String> {
        let tail = path.path.segments.last().map(|s| ident_key(&s.ident));
        if tail
            .as_ref()
            .is_some_and(|tail| self.reserved_tails.contains(tail))
        {
            Err(format!(
                "cpp_abi callable `{}` may only be used as a resolved same-module direct call",
                path.to_token_stream()
            ))
        } else {
            Ok(None)
        }
    }
}

impl VisitMut for CallRewrite<'_> {
    fn visit_expr_call_mut(&mut self, call: &mut syn::ExprCall) {
        if let Expr::Path(path) = call.func.as_mut() {
            match self.resolve_direct(path) {
                Ok(Some((key, helper))) => {
                    self.used.insert(key);
                    path.path = syn::Path::from(proc_macro2::Ident::new(
                        &helper,
                        path.path
                            .segments
                            .last()
                            .expect("nonempty callable path")
                            .ident
                            .span(),
                    ));
                }
                Ok(None) => {
                    self.direct_callee_depth += 1;
                    syn::visit_mut::visit_expr_path_mut(self, path);
                    self.direct_callee_depth -= 1;
                }
                Err(error) => self.errors.push(error),
            }
        } else {
            self.visit_expr_mut(&mut call.func);
        }
        for arg in &mut call.args {
            self.visit_expr_mut(arg);
        }
    }

    fn visit_expr_path_mut(&mut self, path: &mut syn::ExprPath) {
        if self.direct_callee_depth == 0 {
            let tail = path.path.segments.last().map(|s| ident_key(&s.ident));
            if tail
                .as_ref()
                .is_some_and(|tail| self.reserved_tails.contains(tail))
            {
                self.errors.push(format!(
                    "cpp_abi callable `{}` cannot be used as a function value or non-call path",
                    path.to_token_stream()
                ));
                return;
            }
        }
        syn::visit_mut::visit_expr_path_mut(self, path);
    }

    fn visit_pat_ident_mut(&mut self, pattern: &mut syn::PatIdent) {
        let name = ident_key(&pattern.ident);
        if self.local_free.contains_key(&name) {
            self.errors.push(format!(
                "local binding `{name}` shadows a cpp_abi callable and prevents sound call resolution"
            ));
        }
        syn::visit_mut::visit_pat_ident_mut(self, pattern);
    }
}

fn lower_module_items(
    items: &mut Vec<Item>,
    module: &ModulePath,
    contracts: &CppAbiContracts,
    helper_names: &BTreeMap<CallableKey, String>,
) -> Result<(), String> {
    let mut lowered = Vec::with_capacity(items.len() + contracts.callables.len());
    for mut item in std::mem::take(items) {
        match &mut item {
            Item::Fn(function) => {
                let key = CallableKey::Free {
                    module: module.clone(),
                    name: ident_key(&function.sig.ident),
                };
                if contracts.callables.contains_key(&key) {
                    let helper_name = helper_names.get(&key).expect("allocated helper");
                    let mut helper = function.clone();
                    helper.attrs.clear();
                    helper.vis = syn::Visibility::Inherited;
                    helper.sig.ident =
                        proc_macro2::Ident::new(helper_name, function.sig.ident.span());
                    function.attrs.retain(|attr| attr.path().is_ident("doc"));
                    function.block = Box::new(syn::parse_quote!({ unreachable!() }));
                    lowered.push(Item::Fn(helper));
                }
                lowered.push(item);
            }
            Item::Impl(implementation) => {
                let owner = simple_impl_owner(implementation).ok();
                let mut helpers = Vec::new();
                if let Some(owner) = owner {
                    for impl_item in &mut implementation.items {
                        let syn::ImplItem::Fn(method) = impl_item else {
                            continue;
                        };
                        let key = CallableKey::InherentStatic {
                            module: module.clone(),
                            owner: owner.clone(),
                            name: ident_key(&method.sig.ident),
                        };
                        if contracts.callables.contains_key(&key) {
                            let helper_name = helper_names.get(&key).expect("allocated helper");
                            let mut signature = method.sig.clone();
                            signature.ident =
                                proc_macro2::Ident::new(helper_name, method.sig.ident.span());
                            helpers.push(Item::Fn(syn::ItemFn {
                                attrs: Vec::new(),
                                vis: syn::Visibility::Inherited,
                                sig: signature,
                                block: Box::new(method.block.clone()),
                            }));
                            method.attrs.retain(|attr| attr.path().is_ident("doc"));
                            method.block = syn::parse_quote!({ unreachable!() });
                        }
                    }
                }
                lowered.extend(helpers);
                lowered.push(item);
            }
            Item::Type(alias) => {
                let key = (module.clone(), ident_key(&alias.ident));
                if contracts.aliases.contains_key(&key) {
                    alias.attrs.retain(|attr| attr.path().is_ident("doc"));
                }
                lowered.push(item);
            }
            Item::Mod(item_mod) => {
                if let Some((_, nested)) = &mut item_mod.content {
                    let mut path = module.0.clone();
                    path.push(ident_key(&item_mod.ident));
                    lower_module_items(nested, &ModulePath(path), contracts, helper_names)?;
                }
                lowered.push(item);
            }
            _ => lowered.push(item),
        }
    }
    *items = lowered;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contracts(source: &str) -> Result<CppAbiContracts, String> {
        let file = syn::parse_str(source).expect("test source must parse as Rust");
        collect(&file)
    }

    fn assert_rustc_valid(source: &str, label: &str) {
        let temp = tempfile::tempdir().unwrap();
        let source_path = temp.path().join("lib.rs");
        let output_path = temp.path().join("libparser_repro.rlib");
        std::fs::write(&source_path, source).unwrap();
        let rustc = std::process::Command::new("rustc")
            .arg("--edition=2024")
            .arg("--crate-type=lib")
            .arg(&source_path)
            .arg("-o")
            .arg(&output_path)
            .output()
            .unwrap();
        assert!(
            rustc.status.success(),
            "{label} must remain valid Rust: {}",
            String::from_utf8_lossy(&rustc.stderr)
        );
    }

    fn assert_rustc_valid_but_contract_rejected(source: &str, label: &str) {
        assert_rustc_valid(source, label);
        assert!(contracts(source).is_err(), "accepted {label}");
    }

    #[test]
    fn parses_closed_string_and_vector_contracts() {
        let parsed = contracts(
            r#"
            #[cfg_attr(any(), cpp_abi_alias(std_vector))]
            pub type Weights = Vec<f64>;

            #[cfg_attr(any(), cpp_abi(
                param(bytes, std_string_bytes),
                returns(std_string_bytes)
            ))]
            pub fn pad(bytes: Vec<u8>, width: i32) -> Vec<u8> { bytes }

            pub struct Generator;
            impl Generator {
                #[cfg_attr(any(), cpp_abi(param(weights, const_ref(Weights))))]
                pub fn select(weights: &[f64]) -> u32 { weights.len() as u32 }
            }
            "#,
        )
        .unwrap();

        assert_eq!(parsed.aliases.len(), 1);
        assert_eq!(parsed.callables.len(), 2);
        assert!(parsed.callables.contains_key(&CallableKey::Free {
            module: ModulePath(vec![]),
            name: "pad".into(),
        }));
        assert!(parsed.callables.contains_key(&CallableKey::InherentStatic {
            module: ModulePath(vec![]),
            owner: "Generator".into(),
            name: "select".into(),
        }));
    }

    #[test]
    fn canonical_nested_modules_do_not_collide_on_same_tail() {
        let parsed = contracts(
            r#"
            pub mod left {
                #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                pub fn convert(v: Vec<u8>) {}
            }
            pub mod right {
                #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                pub fn convert(v: Vec<u8>) {}
            }
            "#,
        )
        .unwrap();
        assert_eq!(parsed.callables.len(), 2);
        assert!(parsed.callables.keys().any(|key| matches!(
            key,
            CallableKey::Free { module: ModulePath(path), name }
                if path == &["left".to_string()] && name == "convert"
        )));
        assert!(parsed.callables.keys().any(|key| matches!(
            key,
            CallableKey::Free { module: ModulePath(path), name }
                if path == &["right".to_string()] && name == "convert"
        )));
    }

    #[test]
    fn rejects_direct_or_potentially_active_markers() {
        for source in [
            "#[cpp_abi(param(v, std_string_bytes))] pub fn f(v: Vec<u8>) {}",
            "#[cfg_attr(unix, cpp_abi(param(v, std_string_bytes)))] pub fn f(v: Vec<u8>) {}",
            "#[cfg_attr(any(x), cpp_abi(param(v, std_string_bytes)))] pub fn f(v: Vec<u8>) {}",
            "#[cfg_attr(any(), allow(dead_code), cpp_abi(param(v, std_string_bytes)))] pub fn f(v: Vec<u8>) {}",
        ] {
            assert!(contracts(source).is_err(), "accepted: {source}");
        }
    }

    #[test]
    fn rejects_qualified_raw_and_recursively_nested_marker_attempts() {
        for source in [
            "#[cfg_attr(any(), crate::cpp_abi(param(v, std_string_bytes)))] pub fn f(v: Vec<u8>) {}",
            "#[cfg_attr(any(), r#cpp_abi(param(v, std_string_bytes)))] pub fn f(v: Vec<u8>) {}",
            "#[cfg_attr(any(), cfg_attr(any(), cpp_abi(param(v, std_string_bytes))))] pub fn f(v: Vec<u8>) {}",
        ] {
            assert!(contracts(source).is_err(), "accepted: {source}");
        }
    }

    #[test]
    fn rejects_crate_and_inline_module_inner_marker_attributes() {
        for (label, source) in [
            (
                "crate callable marker",
                r#"
                    #![cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                    pub fn ordinary() {}
                "#,
            ),
            (
                "crate alias marker",
                r#"
                    #![cfg_attr(any(), cpp_abi_alias(std_vector))]
                    pub fn ordinary() {}
                "#,
            ),
            (
                "inline module inner marker",
                r#"
                    pub mod nested {
                        #![cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                        pub fn ordinary() {}
                    }
                "#,
            ),
        ] {
            assert_rustc_valid_but_contract_rejected(source, label);
        }
    }

    #[test]
    fn rejects_markers_on_unmarked_parameters_and_local_items() {
        for source in [
            r#"
                pub fn f(
                    #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                    v: Vec<u8>,
                ) {}
            "#,
            r#"
                pub struct G;
                impl G {
                    pub fn f(
                        #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                        v: Vec<u8>,
                    ) {}
                }
            "#,
            r#"
                pub fn outer() {
                    #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                    pub fn local(v: Vec<u8>) {}
                }
            "#,
        ] {
            assert!(contracts(source).is_err(), "accepted: {source}");
        }
    }

    #[test]
    fn marker_discovery_ignores_literals_and_longer_unrelated_identifiers() {
        let parsed = contracts(
            r#"
                #[doc = "cpp_abi"]
                #[cfg_attr(any(), doc = "cpp_abi_alias")]
                #[allow(my_cpp_abi_helper)]
                pub fn ordinary(value: Vec<u8>) -> Vec<u8> { value }

                pub struct cpp_abi_metadata;
            "#,
        )
        .unwrap();
        assert!(parsed.aliases.is_empty());
        assert!(parsed.callables.is_empty());
    }

    #[test]
    fn target_root_probe_is_structural_and_fail_closed() {
        assert!(!source_mentions_reserved_marker(
            r#"
                pub fn ordinary() {
                    let cpp_abi = 1;
                    let cpp_abi_alias = cpp_abi;
                    let _ = cpp_abi_alias;
                }
            "#,
        ));
        assert!(source_mentions_reserved_marker(
            r#"
                #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                pub fn adapted(v: Vec<u8>) {}
            "#,
        ));
        assert!(source_mentions_reserved_marker(
            r#"
                pub fn misplaced(
                    #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                    v: Vec<u8>,
                ) {}
            "#,
        ));
    }

    #[test]
    fn raw_identifiers_are_canonicalized_in_every_ir_key() {
        let parsed = contracts(
            r#"
                pub mod r#type {
                    #[cfg_attr(any(), cpp_abi_alias(std_vector))]
                    pub type r#match = Vec<f64>;

                    #[cfg_attr(any(), cpp_abi(param(r#type, const_ref(r#match))))]
                    pub fn r#loop(r#type: &[f64]) -> u32 { 0 }

                    pub struct r#struct;
                    impl r#struct {
                        #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
                        pub fn r#move() -> Vec<u8> { vec![] }
                    }
                }
            "#,
        )
        .unwrap();

        let module = ModulePath(vec!["type".to_string()]);
        assert!(
            parsed
                .aliases
                .contains_key(&(module.clone(), "match".into()))
        );
        let free = parsed
            .callables
            .get(&CallableKey::Free {
                module: module.clone(),
                name: "loop".into(),
            })
            .unwrap();
        assert!(free.params.contains_key("type"));
        assert!(matches!(
            free.params.get("type"),
            Some(ParamAdapter::ConstRef { alias, .. }) if alias == "match"
        ));
        assert!(parsed.callables.contains_key(&CallableKey::InherentStatic {
            module,
            owner: "struct".into(),
            name: "move".into(),
        }));
    }

    #[test]
    fn mutually_exclusive_raw_and_plain_module_keys_collide() {
        let source = r#"
            #[cfg(any())]
            pub mod same {
                #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                pub fn convert(v: Vec<u8>) {}
            }

            #[cfg(not(any()))]
            pub mod r#same {
                #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                pub fn convert(v: Vec<u8>) {}
            }
        "#;

        assert_rustc_valid(source, "mutually exclusive raw/plain duplicate");
        assert!(contracts(source).is_err());
    }

    #[test]
    fn rejects_markers_on_all_free_function_generic_parameter_kinds() {
        for (label, source) in [
            (
                "free type generic attribute",
                r#"
                    pub fn f<
                        #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))] T
                    >() {}
                "#,
            ),
            (
                "free const generic attribute",
                r#"
                    pub fn f<
                        #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))] const N: usize
                    >() {}
                "#,
            ),
            (
                "free lifetime generic attribute",
                r#"
                    pub fn f<
                        #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))] 'a
                    >() {}
                "#,
            ),
        ] {
            assert_rustc_valid_but_contract_rejected(source, label);
        }
    }

    #[test]
    fn rejects_markers_on_method_generic_and_self_receiver() {
        for (label, source) in [
            (
                "method generic attribute",
                r#"
                    pub struct G;
                    impl G {
                        pub fn f<
                            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))] T
                        >() {}
                    }
                "#,
            ),
            (
                "method self receiver attribute",
                r#"
                    pub struct G;
                    impl G {
                        pub fn f(
                            #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))] &self
                        ) {}
                    }
                "#,
            ),
        ] {
            assert_rustc_valid_but_contract_rejected(source, label);
        }
    }

    #[test]
    fn rejects_markers_on_type_alias_and_impl_generics() {
        for (label, source) in [
            (
                "ordinary type-alias generic attribute",
                r#"
                    pub type Alias<
                        #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))] T
                    > = Vec<T>;
                "#,
            ),
            (
                "impl generic attribute",
                r#"
                    pub struct G<T>(T);
                    impl<
                        #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))] T
                    > G<T> {}
                "#,
            ),
        ] {
            assert_rustc_valid_but_contract_rejected(source, label);
        }
    }

    #[test]
    fn rejects_deep_impl_const_and_gat_markers() {
        for (label, source) in [
            (
                "local marked fn in impl const initializer",
                r#"
                    pub struct G;
                    impl G {
                        pub const VALUE: () = {
                            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                            fn local(v: Vec<u8>) {}
                        };
                    }
                "#,
            ),
            (
                "marked generic attribute in impl GAT",
                r#"
                    pub trait Tr { type Out<T>; }
                    pub struct G;
                    impl Tr for G {
                        type Out<
                            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))] T
                        > = T;
                    }
                "#,
            ),
        ] {
            assert_rustc_valid_but_contract_rejected(source, label);
        }
    }

    #[test]
    fn rejects_markers_hidden_in_impl_and_top_level_macro_tokens() {
        for (label, source) in [
            (
                "marked method in opaque impl macro tokens",
                r#"
                    macro_rules! passthrough { ($($item:item)*) => { $($item)* }; }
                    pub struct G;
                    impl G {
                        passthrough! {
                            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                            pub fn f(v: Vec<u8>) {}
                        }
                    }
                "#,
            ),
            (
                "marked fn in opaque top-level macro tokens",
                r#"
                    macro_rules! passthrough { ($($item:item)*) => { $($item)* }; }
                    passthrough! {
                        #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                        pub fn f(v: Vec<u8>) {}
                    }
                "#,
            ),
            (
                "inner marker attribute in opaque macro tokens",
                r#"
                    macro_rules! make_module {
                        () => {
                            pub mod generated {
                                #![cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
                            }
                        };
                    }
                    make_module!();
                "#,
            ),
        ] {
            assert_rustc_valid_but_contract_rejected(source, label);
        }
    }

    #[test]
    fn opaque_macro_scan_ignores_doc_literals_and_longer_identifiers() {
        let source = r#"
            macro_rules! my_cpp_abi_helper { ($($item:item)*) => { $($item)* }; }
            my_cpp_abi_helper! {
                #[doc = "cpp_abi and cpp_abi_alias are documentation words"]
                pub fn ordinary() {}
            }
        "#;
        assert_rustc_valid(source, "opaque macro doc literal nonmatch");
        let parsed = contracts(source).unwrap();
        assert!(parsed.aliases.is_empty());
        assert!(parsed.callables.is_empty());
    }

    #[test]
    fn opaque_macro_reserves_ordinary_exact_identifiers_by_policy() {
        for (label, source) in [
            (
                "macro body local named cpp_abi",
                r#"
                    macro_rules! m {
                        () => {{ let cpp_abi = 1; let _ = cpp_abi; }};
                    }
                    pub fn ordinary() { m!(); }
                "#,
            ),
            (
                "macro argument ident named cpp_abi",
                r#"
                    macro_rules! consume { ($name:ident) => {}; }
                    consume!(cpp_abi);
                "#,
            ),
            (
                "discarded expression calling cpp_abi",
                r#"
                    macro_rules! discard_expr { ($expr:expr) => {}; }
                    discard_expr!(cpp_abi(1));
                "#,
            ),
        ] {
            assert_rustc_valid(source, label);
            assert!(
                contracts(source).is_err(),
                "accepted reserved ident in {label}"
            );
        }
    }

    #[test]
    fn rejects_split_token_marker_assembly_for_callable_and_alias() {
        for (label, marker_name, marker_args) in [
            (
                "split callable marker",
                "cpp_abi",
                "(param(v, std_string_bytes))",
            ),
            ("split alias marker", "cpp_abi_alias", "(std_vector)"),
        ] {
            let source = format!(
                r#"
                    macro_rules! apply {{
                        ($name:ident, $args:tt, $item:item) => {{
                            #[cfg_attr(any(), $name$args)] $item
                        }};
                    }}
                    apply!(
                        {marker_name},
                        {marker_args},
                        pub fn hidden(v: Vec<u8>) {{}}
                    );
                "#
            );
            assert_rustc_valid_but_contract_rejected(&source, label);
        }
    }

    #[test]
    fn rejects_marker_hidden_in_forwarded_meta_macro_argument() {
        let source = r#"
            macro_rules! apply {
                ($attr:meta, $item:item) => { #[$attr] $item };
            }
            apply!(
                cfg_attr(any(), cpp_abi(param(v, std_string_bytes))),
                pub fn hidden(v: Vec<u8>) {}
            );
        "#;
        assert_rustc_valid_but_contract_rejected(source, "forwarded cpp_abi metadata");
    }

    #[test]
    fn rejects_direct_marker_call_forms_forwarded_as_meta() {
        for (label, marker) in [
            (
                "direct callable marker metadata",
                "cpp_abi(param(v, std_string_bytes))",
            ),
            (
                "qualified callable marker metadata",
                "crate::cpp_abi(param(v, std_string_bytes))",
            ),
            (
                "raw callable marker metadata",
                "r#cpp_abi(param(v, std_string_bytes))",
            ),
            ("direct alias marker metadata", "cpp_abi_alias(std_vector)"),
        ] {
            let source = format!(
                r#"
                    macro_rules! apply {{
                        ($attr:meta, $item:item) => {{
                            #[cfg_attr(any(), $attr)] $item
                        }};
                    }}
                    apply!(
                        {marker},
                        pub fn hidden(v: Vec<u8>) {{}}
                    );
                "#
            );
            assert_rustc_valid_but_contract_rejected(&source, label);
        }
    }

    #[test]
    fn forwarded_nonmarker_metadata_remains_accepted() {
        for (label, source) in [
            (
                "forwarded doc metadata containing marker word",
                r#"
                    macro_rules! apply {
                        ($attr:meta, $item:item) => { #[$attr] $item };
                    }
                    apply!(doc = "cpp_abi is only documentation", pub fn documented() {});
                "#,
            ),
            (
                "forwarded cfg_attr doc metadata containing marker word",
                r#"
                    macro_rules! apply {
                        ($attr:meta, $item:item) => { #[$attr] $item };
                    }
                    apply!(
                        cfg_attr(any(), doc = "cpp_abi_alias is only documentation"),
                        pub fn documented() {}
                    );
                "#,
            ),
        ] {
            assert_rustc_valid(source, label);
            let parsed = contracts(source).unwrap();
            assert!(parsed.aliases.is_empty(), "unexpected alias in {label}");
            assert!(
                parsed.callables.is_empty(),
                "unexpected callable in {label}"
            );
        }
    }

    #[test]
    fn opaque_nonmarker_call_form_remains_accepted() {
        let source = r#"
            macro_rules! consume_expr { ($expr:expr) => {}; }
            consume_expr!(cpp_abi_helper(1));
        "#;
        assert_rustc_valid(source, "ordinary longer-name call form");
        let parsed = contracts(source).unwrap();
        assert!(parsed.aliases.is_empty());
        assert!(parsed.callables.is_empty());
    }

    #[test]
    fn rejects_unknown_arbitrary_or_duplicate_syntax() {
        for source in [
            "#[cfg_attr(any(), cpp_abi(param(v, std::string)))] pub fn f(v: Vec<u8>) {}",
            "#[cfg_attr(any(), cpp_abi(param(v, custom(\"std::string\"))))] pub fn f(v: Vec<u8>) {}",
            "#[cfg_attr(any(), cpp_abi(unknown(v)))] pub fn f(v: Vec<u8>) {}",
            "#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes), param(v, std_string_bytes)))] pub fn f(v: Vec<u8>) {}",
            "#[cfg_attr(any(), cpp_abi(returns(std_string_bytes), returns(std_string_bytes)))] pub fn f() -> Vec<u8> { vec![] }",
            "#[cfg_attr(any(), cpp_abi())] pub fn f() {}",
            "#[cfg_attr(any(), cpp_abi_alias(std_deque))] pub type V = Vec<f64>;",
        ] {
            assert!(contracts(source).is_err(), "accepted: {source}");
        }
    }

    #[test]
    fn rejects_adapter_type_mismatches() {
        for source in [
            "#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))] pub fn f(v: String) {}",
            "#[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))] pub fn f() -> String { String::new() }",
            "#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))] pub fn f(v: &Vec<u8>) {}",
            "#[cfg_attr(any(), cpp_abi(param(missing, std_string_bytes)))] pub fn f(v: Vec<u8>) {}",
        ] {
            assert!(contracts(source).is_err(), "accepted: {source}");
        }
    }

    #[test]
    fn rejects_invalid_alias_contracts_and_cross_module_lookup() {
        for source in [
            "#[cfg_attr(any(), cpp_abi_alias(std_vector))] type V = Vec<f64>;",
            "#[cfg_attr(any(), cpp_abi_alias(std_vector))] pub type V = std::vec::Vec<f64>;",
            "#[cfg_attr(any(), cpp_abi_alias(std_vector))] pub type V<T> = Vec<T>;",
            r#"
                #[cfg_attr(any(), cpp_abi_alias(std_vector))]
                pub type V = Vec<f64>;
                #[cfg_attr(any(), cpp_abi(param(v, const_ref(V))))]
                pub fn f(v: &[u64]) {}
            "#,
            r#"
                pub type V = Vec<f64>;
                #[cfg_attr(any(), cpp_abi(param(v, const_ref(V))))]
                pub fn f(v: &[f64]) {}
            "#,
            r#"
                #[cfg_attr(any(), cpp_abi_alias(std_vector))]
                pub type V = Vec<f64>;
                mod nested {
                    #[cfg_attr(any(), cpp_abi(param(v, const_ref(V))))]
                    pub fn f(v: &[f64]) {}
                }
            "#,
        ] {
            assert!(contracts(source).is_err(), "accepted: {source}");
        }
    }

    #[test]
    fn rejects_unconsumed_alias_and_non_static_method() {
        assert!(
            contracts("#[cfg_attr(any(), cpp_abi_alias(std_vector))] pub type V = Vec<f64>;")
                .is_err()
        );
        assert!(
            contracts(
                r#"
            pub struct G;
            impl G {
                #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
                pub fn f(&self) -> Vec<u8> { vec![] }
            }
            "#
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_non_public_generic_unsafe_and_patterned_callables() {
        for source in [
            "#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))] fn f(v: Vec<u8>) {}",
            "#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))] pub fn f<T>(v: Vec<u8>) {}",
            "#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))] pub unsafe fn f(v: Vec<u8>) {}",
            "#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))] pub fn f((v,): (Vec<u8>,)) {}",
            "#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))] pub fn f(mut v: Vec<u8>) {}",
        ] {
            assert!(contracts(source).is_err(), "accepted: {source}");
        }
    }

    #[test]
    fn rejects_trait_method_and_duplicate_same_scope_callable() {
        assert!(
            contracts(
                r#"
            pub trait T {
                #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
                fn f() -> Vec<u8>;
            }
            "#
            )
            .is_err()
        );
        assert!(
            contracts(
                r#"
            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
            pub fn f(v: Vec<u8>) {}
            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
            pub fn f(v: Vec<u8>) {}
            "#
            )
            .is_err()
        );
    }

    #[test]
    fn accepts_no_markers_as_an_empty_contract() {
        let parsed = contracts("pub fn ordinary(value: Vec<u8>) -> Vec<u8> { value }").unwrap();
        assert!(parsed.aliases.is_empty());
        assert!(parsed.callables.is_empty());
        assert!(parsed.flat_imports.is_empty());
    }

    #[test]
    fn flat_import_marker_is_inert_narrow_and_activates_lowering() {
        let source = r#"
            pub mod rand {
                pub fn randgen_rand_max() -> f64 { 1.0 }
                pub fn randgen_rand_raw() -> u64 { 0 }
            }
            #[cfg_attr(any(), cpp_import_namespace(rrr))]
            use crate::rand::{randgen_rand_max, randgen_rand_raw};
            pub fn draw() -> f64 {
                randgen_rand_raw() as f64 / randgen_rand_max()
            }
        "#;
        assert_rustc_valid(source, "flat namespace import marker");
        assert!(source_mentions_reserved_marker(source));
        let parsed = contracts(source).unwrap();
        assert_eq!(parsed.flat_imports.len(), 1);
        assert!(parsed.callables.is_empty());

        let file = syn::parse_str(source).unwrap();
        let (lowered, plan) = lower(&file).unwrap().expect("marker-only lowering");
        assert!(plan.facades.is_empty());
        assert!(!plan.is_empty());
        assert!(lowered
            .to_token_stream()
            .to_string()
            .contains("cpp_import_namespace"));
        plan.validate_flat_import_namespace(Some("rrr"), "test")
            .unwrap();
        assert!(plan
            .validate_flat_import_namespace(Some("other"), "test")
            .is_err());
        assert!(plan.validate_flat_import_namespace(None, "test").is_err());
    }

    #[test]
    fn flat_import_marker_rejects_every_broader_use_shape() {
        let invalid = [
            (
                "public",
                "#[cfg_attr(any(), cpp_import_namespace(rrr))] pub use crate::rand::f;",
            ),
            (
                "rename",
                "#[cfg_attr(any(), cpp_import_namespace(rrr))] use crate::rand::f as g;",
            ),
            (
                "glob",
                "#[cfg_attr(any(), cpp_import_namespace(rrr))] use crate::rand::*;",
            ),
            (
                "self",
                "#[cfg_attr(any(), cpp_import_namespace(rrr))] use crate::rand::{self};",
            ),
            (
                "nested child",
                "#[cfg_attr(any(), cpp_import_namespace(rrr))] use crate::rpc::rand::f;",
            ),
            (
                "active marker",
                "#[cpp_import_namespace(rrr)] use crate::rand::f;",
            ),
            (
                "wrong item",
                "#[cfg_attr(any(), cpp_import_namespace(rrr))] pub fn f() {}",
            ),
            (
                "qualified marker",
                "#[cfg_attr(any(), crate::cpp_import_namespace(rrr))] use crate::rand::f;",
            ),
            (
                "raw marker",
                "#[cfg_attr(any(), r#cpp_import_namespace(rrr))] use crate::rand::f;",
            ),
            (
                "duplicate marker",
                "#[cfg_attr(any(), cpp_import_namespace(rrr))] #[cfg_attr(any(), cpp_import_namespace(rrr))] use crate::rand::f;",
            ),
            (
                "malformed marker",
                "#[cfg_attr(any(), cpp_import_namespace)] use crate::rand::f;",
            ),
            (
                "extra cfg payload",
                "#[cfg_attr(any(), cpp_import_namespace(rrr), allow(dead_code))] use crate::rand::f;",
            ),
            (
                "raw namespace",
                "#[cfg_attr(any(), cpp_import_namespace(r#rrr))] use crate::rand::f;",
            ),
            (
                "absolute namespace",
                "#[cfg_attr(any(), cpp_import_namespace(::rrr))] use crate::rand::f;",
            ),
            (
                "C++ keyword namespace",
                "#[cfg_attr(any(), cpp_import_namespace(co_await))] use crate::rand::f;",
            ),
            (
                "raw leaf",
                "#[cfg_attr(any(), cpp_import_namespace(rrr))] use crate::rand::r#type;",
            ),
            (
                "C++ keyword leaf",
                "#[cfg_attr(any(), cpp_import_namespace(rrr))] use crate::rand::co_await;",
            ),
        ];
        for (label, source) in invalid {
            let file = syn::parse_str(source).unwrap_or_else(|error| {
                panic!("{label} fixture must parse as Rust syntax: {error}")
            });
            assert!(collect(&file).is_err(), "accepted {label}: {source}");
            assert!(source_mentions_reserved_marker(source), "missed {label}");
        }
    }

    #[test]
    fn flat_import_marker_discovery_ignores_literals_but_reserves_macro_tokens() {
        let ordinary = r#"
            #[doc = "cpp_import_namespace"]
            pub fn documented() -> &'static str { "cpp_import_namespace" }
        "#;
        assert!(!source_mentions_reserved_marker(ordinary));
        assert!(contracts(ordinary).unwrap().flat_imports.is_empty());

        let macro_source = r#"
            macro_rules! assemble { () => { cpp_import_namespace } }
        "#;
        assert!(source_mentions_reserved_marker(macro_source));
        assert!(contracts(macro_source).is_err());
    }

    #[test]
    fn flat_import_marker_rejects_same_module_shadowing() {
        for (label, source) in [
            (
                "item declaration",
                r#"
                    #[cfg_attr(any(), cpp_import_namespace(rrr))]
                    use crate::rand::f;
                    fn f() {}
                "#,
            ),
            (
                "ordinary use binding",
                r#"
                    #[cfg_attr(any(), cpp_import_namespace(rrr))]
                    use crate::rand::f;
                    use crate::other::g as f;
                "#,
            ),
            (
                "lexical binding",
                r#"
                    #[cfg_attr(any(), cpp_import_namespace(rrr))]
                    use crate::rand::f;
                    fn call(f: fn()) { f(); }
                "#,
            ),
            (
                "ordinary glob",
                r#"
                    #[cfg_attr(any(), cpp_import_namespace(rrr))]
                    use crate::rand::f;
                    use crate::other::*;
                "#,
            ),
            (
                "opaque macro leaf",
                r#"
                    #[cfg_attr(any(), cpp_import_namespace(rrr))]
                    use crate::rand::f;
                    fn call() { opaque!(f); }
                "#,
            ),
            (
                "opaque attribute leaf",
                r#"
                    #[cfg_attr(any(), cpp_import_namespace(rrr))]
                    use crate::rand::f;
                    #[allow(f)]
                    fn call() {}
                "#,
            ),
        ] {
            let file = syn::parse_str(source).unwrap();
            let error = lower(&file).expect_err(label);
            assert!(
                error.contains("shadow")
                    || error.contains("glob import")
                    || error.contains("opaque"),
                "{label}: {error}"
            );
        }

        for (label, body) in [
            (
                "raw lexical binding with imported C++ spelling",
                r#"
                    pub fn call() {
                        let r#static = 1u64;
                        let _ = r#static;
                        static_();
                    }
                "#,
            ),
            (
                "raw use binding with imported C++ spelling",
                r#"
                    use crate::other::helper as r#static;
                    pub fn call() {
                        r#static();
                        static_();
                    }
                "#,
            ),
            (
                "raw local item with imported C++ spelling",
                r#"
                    pub fn call() {
                        fn r#static() {}
                        r#static();
                        static_();
                    }
                "#,
            ),
            (
                "raw foreign item with imported C++ spelling",
                r#"
                    unsafe extern "C" { fn r#static(); }
                    pub fn call() { static_(); }
                "#,
            ),
        ] {
            let source = format!(
                r#"
                    mod rand {{ pub fn static_() {{}} }}
                    mod other {{ pub fn helper() {{}} }}
                    #[cfg_attr(any(), cpp_import_namespace(rrr))]
                    use crate::rand::static_;
                    {body}
                "#
            );
            assert_rustc_valid(&source, label);
            let file = syn::parse_str(&source).unwrap();
            let error = lower(&file).expect_err(label);
            assert!(error.contains("same C++ spelling"), "{label}: {error}");
        }
    }

    #[test]
    fn flat_import_inline_references_must_stay_in_marked_block() {
        let marked: syn::File = syn::parse_str(
            r#"
                #[cfg_attr(any(), cpp_import_namespace(rrr))]
                use crate::rand::f;
                pub fn first() -> u64 { f() }
            "#,
        )
        .unwrap();
        let cross_block: syn::File =
            syn::parse_str("pub fn second() -> u64 { f() }").unwrap();
        let error = prepare_inline_carrier(
            &[marked.clone(), cross_block],
            &ExternalContractIndex::default(),
            "test",
        )
        .expect_err("cross-block flat import reference");
        assert!(error.contains("marked inline block"), "{error}");

        let unrelated: syn::File =
            syn::parse_str("pub fn second() -> u64 { 3 }").unwrap();
        let plan = prepare_inline_carrier(
            &[marked, unrelated],
            &ExternalContractIndex::default(),
            "test",
        )
        .expect("unrelated later block");
        assert_eq!(plan.flat_import_blocks, BTreeSet::from([0]));
        assert!(plan.blocks[1].dependencies.is_empty());
    }

    #[test]
    fn lowering_extracts_helpers_and_rewrites_internal_direct_calls() {
        let source = r#"
            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes), returns(std_string_bytes)))]
            pub fn bytes(v: Vec<u8>) -> Vec<u8> { bytes_inner(v) }
            fn bytes_inner(v: Vec<u8>) -> Vec<u8> { bytes(v) }
        "#;
        let file = syn::parse_str(source).unwrap();
        let (lowered, plan) = lower(&file).unwrap().expect("nonempty lowering");
        let text = lowered.to_token_stream().to_string();
        assert!(text.contains("fn rusty_cpp_abi_sem_bytes"));
        assert!(text.contains("rusty_cpp_abi_sem_bytes (v)"));
        assert!(!text.contains("cpp_abi ("));
        assert!(plan.is_semantic_helper(&[], "rusty_cpp_abi_sem_bytes"));
        assert!(plan.free_facade(&[], "bytes").is_some());
    }

    #[test]
    fn lowering_rewrites_recursion_and_mutual_marked_calls() {
        let source = r#"
            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes), returns(std_string_bytes)))]
            pub fn left(v: Vec<u8>) -> Vec<u8> {
                if v.is_empty() { v } else { right(v) }
            }
            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes), returns(std_string_bytes)))]
            pub fn right(v: Vec<u8>) -> Vec<u8> {
                if v.is_empty() { left(v) } else { v }
            }
        "#;
        let file = syn::parse_str(source).unwrap();
        let (lowered, _) = lower(&file).unwrap().expect("nonempty lowering");
        let text = lowered.to_token_stream().to_string();
        assert!(text.contains("rusty_cpp_abi_sem_right (v)"));
        assert!(text.contains("rusty_cpp_abi_sem_left (v)"));
    }

    #[test]
    fn lowering_emits_exact_core_facade_shapes() {
        let source = include_str!("../tests/fixtures/cpp_abi_core.rs");
        assert_rustc_valid(source, "cpp_abi core fixture");
        let cpp = crate::transpile::transpile(source, Some("cpp_abi_core")).unwrap();
        for expected in [
            "export using Weights = std::vector<double>;",
            "static rusty::Vec<uint8_t> rusty_cpp_abi_sem_roundtrip(rusty::Vec<uint8_t> bytes);",
            "export std::string roundtrip(std::string bytes);",
            "static std::string encode(uint8_t value);",
            "std::string Codec::encode(uint8_t value) {",
            "static uint32_t choose(const Weights& weights);",
            "uint32_t Picker::choose(const Weights& weights) {",
            "auto rusty_cpp_abi_arg_0 = rusty_cpp_abi_detail::bytes_from_std_string(bytes);",
            "auto rusty_cpp_abi_arg_0 = rusty_cpp_abi_detail::f64_span_from_std_vector(weights);",
        ] {
            assert!(cpp.contains(expected), "missing `{expected}`\n{cpp}");
        }
        assert_eq!(
            cpp.matches("std::string Codec::encode(uint8_t value) {")
                .count(),
            1
        );
        assert_eq!(
            cpp.matches("uint32_t Picker::choose(const Weights& weights) {")
                .count(),
            1
        );
        assert!(!cpp.contains("export static rusty_cpp_abi_sem_"));
    }

    #[test]
    fn lowering_rejects_function_values_and_cross_module_calls() {
        for source in [
            r#"
                #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                pub fn bytes(v: Vec<u8>) {}
                pub fn value() { let _f = bytes; }
            "#,
            r#"
                mod a {
                    #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                    pub fn bytes(v: Vec<u8>) {}
                }
                mod b { pub fn call(v: Vec<u8>) { super::a::bytes(v); } }
            "#,
        ] {
            let file = syn::parse_str(source).unwrap();
            assert!(lower(&file).is_err(), "accepted unsupported use: {source}");
        }
    }

    #[test]
    fn lowering_rejects_reviewer_shadow_import_and_whole_ast_repros() {
        for (label, source) in [
            (
                "shadowed callable parameter",
                include_str!("../tests/fixtures/cpp_abi_reject/shadow_param.rs"),
            ),
            (
                "imported callable alias",
                include_str!("../tests/fixtures/cpp_abi_reject/use_alias.rs"),
            ),
            (
                "const static associated and trait uses",
                include_str!("../tests/fixtures/cpp_abi_reject/top_level_uses.rs"),
            ),
            (
                "opaque macro callable use",
                include_str!("../tests/fixtures/cpp_abi_reject/macro_use.rs"),
            ),
            (
                "local function item shadow",
                include_str!("../tests/fixtures/cpp_abi_reject/local_item_fn_shadow.rs"),
            ),
            (
                "local const item shadow",
                include_str!("../tests/fixtures/cpp_abi_reject/local_const_shadow.rs"),
            ),
            (
                "local static item shadow",
                include_str!("../tests/fixtures/cpp_abi_reject/local_static_shadow.rs"),
            ),
            (
                "local foreign function shadow",
                include_str!("../tests/fixtures/cpp_abi_reject/local_foreign_shadow.rs"),
            ),
            (
                "local method owner shadow",
                include_str!("../tests/fixtures/cpp_abi_reject/local_owner_shadow.rs"),
            ),
        ] {
            assert_rustc_valid(source, label);
            let file = syn::parse_str(source).unwrap();
            assert!(lower(&file).is_err(), "accepted reviewer repro: {label}");
        }
    }

    #[test]
    fn lowering_rejects_all_local_value_and_owner_shadow_forms() {
        for (label, source) in [
            (
                "tuple struct constructor",
                r#"
                    #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                    pub fn adapted(v: Vec<u8>) {}
                    pub fn consumer() {
                        struct adapted(Vec<u8>);
                        let _ = adapted(Vec::new());
                    }
                "#,
            ),
            (
                "unit struct constructor",
                r#"
                    #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                    pub fn adapted(v: Vec<u8>) {}
                    pub fn consumer() {
                        struct adapted;
                        let _ = adapted;
                    }
                "#,
            ),
            (
                "local use alias",
                r#"
                    #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                    pub fn adapted(v: Vec<u8>) {}
                    mod alternate { pub fn call(_: Vec<u8>) {} }
                    pub fn consumer() {
                        use crate::alternate::call as adapted;
                        adapted(Vec::new());
                    }
                "#,
            ),
            (
                "local use glob",
                r#"
                    #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                    pub fn adapted(v: Vec<u8>) {}
                    mod alternate { pub fn unrelated() {} }
                    pub fn consumer() {
                        use crate::alternate::*;
                        unrelated();
                    }
                "#,
            ),
            (
                "const generic parameter",
                r#"
                    #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                    pub fn adapted(v: Vec<u8>) {}
                    pub fn consumer<const adapted: usize>() { let _ = adapted; }
                "#,
            ),
            (
                "type generic owner",
                r#"
                    pub struct Owner;
                    impl Owner {
                        #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                        pub fn adapted(v: Vec<u8>) {}
                    }
                    pub trait Alternative { fn adapted(_: Vec<u8>); }
                    pub fn consumer<Owner: Alternative>() {
                        Owner::adapted(Vec::new());
                    }
                "#,
            ),
            (
                "local type alias owner",
                r#"
                    pub struct Owner;
                    impl Owner {
                        #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                        pub fn adapted(v: Vec<u8>) {}
                    }
                    pub struct Alternative;
                    impl Alternative { fn adapted(_: Vec<u8>) {} }
                    pub fn consumer() {
                        type Owner = Alternative;
                        Owner::adapted(Vec::new());
                    }
                "#,
            ),
            (
                "nested block shadow",
                r#"
                    #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                    pub fn adapted(v: Vec<u8>) {}
                    pub fn consumer() {
                        {
                            fn adapted(_: Vec<u8>) {}
                            adapted(Vec::new());
                        }
                    }
                "#,
            ),
        ] {
            assert_rustc_valid(source, label);
            let file = syn::parse_str(source).unwrap();
            assert!(lower(&file).is_err(), "accepted local shadow: {label}");
        }
    }

    #[test]
    fn lowering_rejects_file_presence_attrs_and_exact_escaped_collisions() {
        for (label, source) in [
            (
                "source file cfg false",
                include_str!("../tests/fixtures/cpp_abi_reject/file_cfg_false.rs"),
            ),
            (
                "escaped free function collision",
                include_str!("../tests/fixtures/cpp_abi_reject/escaped_free_collision.rs"),
            ),
            (
                "escaped method collision",
                include_str!("../tests/fixtures/cpp_abi_reject/escaped_method_collision.rs"),
            ),
            (
                "escaped owner collision",
                include_str!("../tests/fixtures/cpp_abi_reject/escaped_owner_collision.rs"),
            ),
            (
                "escaped alias collision",
                include_str!("../tests/fixtures/cpp_abi_reject/escaped_alias_collision.rs"),
            ),
            (
                "escaped facade parameter collision",
                r#"
                    #[cfg_attr(any(), cpp_abi(param(r#class, std_string_bytes)))]
                    pub fn adapted(r#class: Vec<u8>, class_: u8) {}
                "#,
            ),
            (
                "free facade and type collision",
                r#"
                    #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
                    pub fn r#class() -> Vec<u8> { Vec::new() }
                    pub struct class_;
                "#,
            ),
            (
                "alias and ordinary free collision",
                r#"
                    #[cfg_attr(any(), cpp_abi_alias(std_vector))]
                    pub type r#class = Vec<f64>;
                    pub fn class_() {}
                    pub struct Picker;
                    impl Picker {
                        #[cfg_attr(any(), cpp_abi(param(values, const_ref(r#class))))]
                        pub fn choose(values: &[f64]) { let _ = values; }
                    }
                "#,
            ),
        ] {
            assert_rustc_valid(source, label);
            let file = syn::parse_str(source).unwrap();
            assert!(lower(&file).is_err(), "accepted collision: {label}");
        }
    }

    #[test]
    fn lowering_rejects_private_cfg_and_unschedulable_ancestors() {
        for (label, source) in [
            (
                "private provider module with use",
                include_str!("../tests/fixtures/cpp_abi_reject/private_module.rs"),
            ),
            (
                "private provider module",
                include_str!("../tests/fixtures/cpp_abi_reject/private_module_only.rs"),
            ),
            (
                "cfg provider ancestors",
                include_str!("../tests/fixtures/cpp_abi_reject/cfg_ancestor.rs"),
            ),
            (
                "tuple owner scheduling",
                include_str!("../tests/fixtures/cpp_abi_reject/tuple_owner.rs"),
            ),
            (
                "private owner",
                r#"
                    struct Codec;
                    impl Codec {
                        #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
                        pub fn encode() -> Vec<u8> { vec![] }
                    }
                "#,
            ),
            (
                "configured owner",
                r#"
                    #[cfg(target_os = "linux")]
                    pub struct Codec;
                    impl Codec {
                        #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
                        pub fn encode() -> Vec<u8> { vec![] }
                    }
                "#,
            ),
            (
                "configured impl",
                r#"
                    pub struct Codec;
                    #[cfg(target_os = "linux")]
                    impl Codec {
                        #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
                        pub fn encode() -> Vec<u8> { vec![] }
                    }
                "#,
            ),
        ] {
            assert_rustc_valid(source, label);
            let file = syn::parse_str(source).unwrap();
            assert!(
                lower(&file).is_err(),
                "accepted unsupported ancestor: {label}"
            );
        }
    }

    #[test]
    fn lowering_canonicalizes_raw_nested_module_owner_alias_and_helper_keys() {
        let source = r#"
            pub mod r#private {
                #[cfg_attr(any(), cpp_abi_alias(std_vector))]
                pub type r#class = Vec<f64>;

                #[cfg_attr(any(), cpp_abi(param(bytes, std_string_bytes), returns(std_string_bytes)))]
                pub fn r#static(bytes: Vec<u8>) -> Vec<u8> { bytes }

                pub struct r#type;
                impl r#type {
                    #[cfg_attr(any(), cpp_abi(param(values, const_ref(r#class))))]
                    pub fn pause(values: &[f64]) -> u32 { values.len() as u32 }
                }
            }
        "#;
        assert_rustc_valid(source, "raw nested ABI surface");
        let file = syn::parse_str(source).unwrap();
        let (_, plan) = lower(&file).unwrap().expect("nonempty lowering");
        assert!(
            plan.free_facade(&["r#private".into()], "r#static")
                .is_some()
        );
        assert!(
            plan.method_facade(&["r#private".into()], "r#type", "pause")
                .is_some()
        );
        assert!(plan.alias(&["r#private".into()], "r#class").is_some());
        assert!(plan.is_semantic_helper(&["r#private".into()], "rusty_cpp_abi_sem_static"));

        let cpp = crate::transpile::transpile(source, Some("raw_nested")).unwrap();
        for expected in [
            "namespace private_ {",
            "export using class_ = std::vector<double>;",
            "export std::string static_(std::string bytes);",
            "static uint32_t pause(const class_& values);",
            "uint32_t type::pause(const class_& values) {",
            "static rusty::Vec<uint8_t> rusty_cpp_abi_sem_static",
        ] {
            assert!(cpp.contains(expected), "missing `{expected}`\n{cpp}");
        }
        assert!(
            !cpp.contains("pause_("),
            "member-position name was renamed\n{cpp}"
        );
    }

    #[test]
    fn lowering_rejects_adapter_without_named_module_output() {
        let source = r#"
            #[cfg_attr(any(), cpp_abi(param(bytes, std_string_bytes), returns(std_string_bytes)))]
            pub fn adapted(bytes: Vec<u8>) -> Vec<u8> { bytes }
        "#;
        let error = crate::transpile::transpile(source, None).unwrap_err();
        assert!(error.contains("named C++ module output"), "{error}");
    }

    fn crate_units(entries: &[(&str, &str)]) -> Vec<(PathBuf, String)> {
        entries
            .iter()
            .map(|(path, source)| (PathBuf::from(path), (*source).to_string()))
            .collect()
    }

    #[test]
    fn crate_preflight_accepts_attached_public_allow_and_nested_providers() {
        let simple = crate_units(&[
            ("src/lib.rs", "#[allow(dead_code)] pub mod api;"),
            (
                "src/api.rs",
                r#"
                    #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                    pub fn adapted(v: Vec<u8>) {}
                "#,
            ),
        ]);
        assert_eq!(preflight_crate_sources(&simple).unwrap(), true);

        let nested = crate_units(&[
            ("src/lib.rs", "pub mod outer;"),
            ("src/outer.rs", "#[allow(dead_code)] pub mod inner;"),
            (
                "src/outer/inner.rs",
                r#"
                    #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                    pub fn adapted(v: Vec<u8>) {}
                "#,
            ),
        ]);
        assert_eq!(preflight_crate_sources(&nested).unwrap(), true);
    }

    #[test]
    fn crate_preflight_allows_only_unadapted_flat_sibling_leaves() {
        let provider = r#"
            pub fn randgen_rand_max() -> f64 { 1.0 }
            pub fn randgen_rand_raw() -> u64 { 0 }
            pub struct RandomGenerator;
            impl RandomGenerator {
                #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
                pub fn adapted() -> Vec<u8> { Vec::new() }
            }
        "#;
        let allowed = crate_units(&[
            ("src/lib.rs", "pub mod rand; pub mod consumer;"),
            ("src/rand.rs", provider),
            (
                "src/consumer.rs",
                r#"
                    #[cfg_attr(any(), cpp_import_namespace(rrr))]
                    use crate::rand::{randgen_rand_max, randgen_rand_raw};
                    pub fn draw() -> f64 {
                        randgen_rand_raw() as f64 / randgen_rand_max()
                    }
                "#,
            ),
        ]);
        assert_eq!(
            preflight_crate_sources_with_cxx_namespace(&allowed, Some("rrr")).unwrap(),
            true
        );
        assert!(preflight_crate_sources_with_cxx_namespace(&allowed, None).is_err());
        assert!(
            preflight_crate_sources_with_cxx_namespace(&allowed, Some("wrong")).is_err()
        );

        let adapted_owner = crate_units(&[
            ("src/lib.rs", "pub mod rand; pub mod consumer;"),
            ("src/rand.rs", provider),
            (
                "src/consumer.rs",
                r#"
                    #[cfg_attr(any(), cpp_import_namespace(rrr))]
                    use crate::rand::RandomGenerator;
                    pub fn call() -> Vec<u8> { RandomGenerator::adapted() }
                "#,
            ),
        ]);
        let error = preflight_crate_sources_with_cxx_namespace(
            &adapted_owner,
            Some("rrr"),
        )
        .unwrap_err();
        assert!(
            error.contains("sibling-file reference")
                || error.contains("collide")
                || error.contains("direct root-level free function"),
            "{error}"
        );
        assert!(error.contains("RandomGenerator"), "{error}");

        let leaf_consumer = r#"
            #[cfg_attr(any(), cpp_import_namespace(rrr))]
            use crate::rand::target;
            pub fn draw() -> u64 { 0 }
        "#;
        for (label, provider_source, expected, rustc_valid) in [
            (
                "missing provider leaf",
                "pub fn other() -> u64 { 0 }",
                "direct root-level free function",
                false,
            ),
            (
                "private provider leaf",
                "fn target() -> u64 { 0 }",
                "exact public free function",
                false,
            ),
            (
                "const provider leaf",
                "pub const target: u64 = 0;",
                "direct root-level free function",
                true,
            ),
            (
                "type provider leaf",
                "pub type target = u64;",
                "direct root-level free function",
                true,
            ),
            (
                "re-exported provider leaf",
                "mod inner { pub fn target() -> u64 { 0 } } pub use inner::target;",
                "direct root-level free function",
                true,
            ),
            (
                "adapted provider leaf",
                r#"
                    #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
                    pub fn target() -> Vec<u8> { Vec::new() }
                "#,
                "must be unadapted",
                true,
            ),
            (
                "generic provider leaf",
                "pub fn target<T>() -> u64 { 0 }",
                "ordinary, non-generic free function",
                true,
            ),
        ] {
            if rustc_valid {
                let monolith = format!(
                    "mod rand {{ {provider_source} }} mod consumer {{ {leaf_consumer} }}"
                );
                assert_rustc_valid(&monolith, label);
            }
            let units = crate_units(&[
                ("src/lib.rs", "pub mod rand; pub mod consumer;"),
                ("src/rand.rs", provider_source),
                ("src/consumer.rs", leaf_consumer),
            ]);
            let error = preflight_crate_sources_with_cxx_namespace(&units, Some("rrr"))
                .expect_err(label);
            assert!(error.contains(expected), "{label}: {error}");
        }
    }

    #[test]
    fn crate_preflight_confines_flat_leaf_references_to_the_marked_binding() {
        let provider = r#"
            pub fn randgen_rand_max() -> f64 { 1.0 }
            pub fn randgen_rand_raw() -> u64 { 0 }
        "#;
        let consumer = r#"
            #[cfg_attr(any(), cpp_import_namespace(rrr))]
            use crate::rand::{randgen_rand_max, randgen_rand_raw};
            pub fn draw() -> f64 {
                randgen_rand_raw() as f64 / randgen_rand_max()
            }
        "#;

        let unrelated_same_named_local = crate_units(&[
            (
                "src/lib.rs",
                "pub mod rand; pub mod consumer; pub mod other;",
            ),
            ("src/rand.rs", provider),
            ("src/consumer.rs", consumer),
            (
                "src/other.rs",
                r#"
                    mod local {
                        pub fn helper() -> u64 { 2 }
                        pub fn randgen_rand_raw() -> u64 { 3 }
                    }
                    use local::helper as other_helper;
                    macro_rules! unrelated { () => { 7u64 } }
                    pub fn local() -> u64 {
                        let randgen_rand_raw = unrelated!() + other_helper();
                        randgen_rand_raw
                    }
                "#,
            ),
        ]);
        assert_eq!(
            preflight_crate_sources_with_cxx_namespace(
                &unrelated_same_named_local,
                Some("rrr"),
            )
            .unwrap(),
            true
        );

        for (label, consumer_source, other_source) in [
            (
                "qualified sibling reference",
                consumer,
                "pub fn steal() -> u64 { crate::rand::randgen_rand_raw() }",
            ),
            (
                "ordinary sibling import",
                consumer,
                "use crate::rand::randgen_rand_raw; pub fn steal() -> u64 { randgen_rand_raw() }",
            ),
            (
                "qualified bypass in marked module",
                r#"
                    #[cfg_attr(any(), cpp_import_namespace(rrr))]
                    use crate::rand::randgen_rand_raw;
                    pub fn draw() -> u64 { crate::rand::randgen_rand_raw() }
                "#,
                "pub fn unrelated() -> u64 { 1 }",
            ),
            (
                "descendant access to marked binding",
                r#"
                    #[cfg_attr(any(), cpp_import_namespace(rrr))]
                    use crate::rand::randgen_rand_raw;
                    mod nested {
                        pub fn draw() -> u64 { super::randgen_rand_raw() }
                    }
                "#,
                "pub fn unrelated() -> u64 { 1 }",
            ),
        ] {
            let units = crate_units(&[
                (
                    "src/lib.rs",
                    "pub mod rand; pub mod consumer; pub mod other;",
                ),
                ("src/rand.rs", provider),
                ("src/consumer.rs", consumer_source),
                ("src/other.rs", other_source),
            ]);
            let error = preflight_crate_sources_with_cxx_namespace(&units, Some("rrr"))
                .expect_err(label);
            assert!(
                error.contains("cpp_import_namespace crate preflight rejects"),
                "{label}: {error}"
            );
        }

        let descendant_alias_syntax = r#"
            mod rand { pub fn randgen_rand_raw() -> u64 { 0 } }
            mod outer {
                pub mod consumer {
                    #[cfg_attr(any(), cpp_import_namespace(rrr))]
                    use crate::rand::randgen_rand_raw;
                    mod exact_alias {
                        use crate::outer::consumer as c;
                        pub fn call() -> u64 { c::randgen_rand_raw() }
                    }
                    mod direct_import {
                        use crate::outer::consumer;
                        pub fn call() -> u64 { consumer::randgen_rand_raw() }
                    }
                    mod grouped_alias {
                        use crate::outer::{consumer as c};
                        pub fn call() -> u64 { c::randgen_rand_raw() }
                    }
                    mod raw_alias {
                        use crate::outer::consumer as r#type;
                        pub fn call() -> u64 { r#type::randgen_rand_raw() }
                    }
                    mod ancestor_alias {
                        use crate::outer as o;
                        pub fn call() -> u64 { o::consumer::randgen_rand_raw() }
                    }
                }
            }
        "#;
        assert_rustc_valid(
            descendant_alias_syntax,
            "flat-import marked-consumer descendant alias matrix",
        );
        for (label, nested_source) in [
            (
                "descendant exact consumer alias",
                "use crate::outer::consumer as c; pub fn steal() -> u64 { c::randgen_rand_raw() }",
            ),
            (
                "descendant direct consumer import",
                "use crate::outer::consumer; pub fn steal() -> u64 { consumer::randgen_rand_raw() }",
            ),
            (
                "descendant grouped consumer alias",
                "use crate::outer::{consumer as c}; pub fn steal() -> u64 { c::randgen_rand_raw() }",
            ),
            (
                "descendant raw consumer alias",
                "use crate::outer::consumer as r#type; pub fn steal() -> u64 { r#type::randgen_rand_raw() }",
            ),
            (
                "descendant consumer-ancestor alias",
                "use crate::outer as o; pub fn steal() -> u64 { o::consumer::randgen_rand_raw() }",
            ),
        ] {
            let consumer_source = format!(
                r#"
                    #[cfg_attr(any(), cpp_import_namespace(rrr))]
                    use crate::rand::randgen_rand_raw;
                    pub mod nested {{ {nested_source} }}
                "#
            );
            let units = crate_units(&[
                ("src/lib.rs", "pub mod rand; pub mod outer;"),
                ("src/rand.rs", provider),
                ("src/outer.rs", "pub mod consumer;"),
                ("src/outer/consumer.rs", &consumer_source),
            ]);
            let error = preflight_crate_sources_with_cxx_namespace(&units, Some("rrr"))
                .expect_err(label);
            assert!(
                error.contains("marked consumer ancestor"),
                "{label}: {error}"
            );
        }

        for (label, root_import) in [
            ("root consumer descendant super glob", "use super::*;"),
            ("root consumer descendant crate glob", "use crate::*;"),
        ] {
            let root_source = format!(
                r#"
                    pub mod rand;
                    #[cfg_attr(any(), cpp_import_namespace(rrr))]
                    use crate::rand::randgen_rand_raw;
                    mod nested {{
                        {root_import}
                        pub fn steal() -> u64 {{ randgen_rand_raw() }}
                    }}
                "#
            );
            let rustc_source = root_source.replace("pub mod rand;", &format!(
                "pub mod rand {{ {provider} }}"
            ));
            assert_rustc_valid(&rustc_source, label);
            let units = crate_units(&[
                ("src/lib.rs", &root_source),
                ("src/rand.rs", provider),
            ]);
            let error = preflight_crate_sources_with_cxx_namespace(&units, Some("rrr"))
                .expect_err(label);
            assert!(
                error.contains("marked consumer ancestor")
                    || error.contains("cannot involve cpp_abi names"),
                "{label}: {error}"
            );
        }

        let alias_syntax = r#"
            pub mod rand { pub fn randgen_rand_raw() -> u64 { 0 } }
            mod direct {
                use crate as c;
                pub fn call() -> u64 { c::rand::randgen_rand_raw() }
            }
            mod grouped {
                use {crate as c};
                pub fn call() -> u64 { c::rand::randgen_rand_raw() }
            }
            mod grouped_self {
                use crate::{self as c};
                pub fn call() -> u64 { c::rand::randgen_rand_raw() }
            }
            mod parent {
                use super as c;
                pub fn call() -> u64 { c::rand::randgen_rand_raw() }
            }
            mod external {
                extern crate self as c;
                pub fn call() -> u64 { c::rand::randgen_rand_raw() }
            }
            mod local {
                pub fn randgen_rand_raw() -> u64 { 1 }
                use self as c;
                pub fn call() -> u64 { c::randgen_rand_raw() }
            }
        "#;
        assert_rustc_valid(alias_syntax, "flat-import namespace-root alias matrix");

        for (label, consumer_source, other_source) in [
            (
                "crate root alias",
                consumer,
                "use crate as c; pub fn steal() -> u64 { c::rand::randgen_rand_raw() }",
            ),
            (
                "grouped crate root alias",
                consumer,
                "use {crate as c}; pub fn steal() -> u64 { c::rand::randgen_rand_raw() }",
            ),
            (
                "grouped crate self alias",
                consumer,
                "use crate::{self as c}; pub fn steal() -> u64 { c::rand::randgen_rand_raw() }",
            ),
            (
                "parent root alias",
                consumer,
                "use super as c; pub fn steal() -> u64 { c::rand::randgen_rand_raw() }",
            ),
            (
                "extern crate self alias",
                consumer,
                "extern crate self as c; pub fn steal() -> u64 { c::rand::randgen_rand_raw() }",
            ),
            (
                "marked-module self alias",
                r#"
                    #[cfg_attr(any(), cpp_import_namespace(rrr))]
                    use crate::rand::randgen_rand_raw;
                    use self as c;
                    pub fn draw() -> u64 { c::randgen_rand_raw() }
                "#,
                "pub fn unrelated() -> u64 { 1 }",
            ),
        ] {
            let units = crate_units(&[
                (
                    "src/lib.rs",
                    "pub mod rand; pub mod consumer; pub mod other;",
                ),
                ("src/rand.rs", provider),
                ("src/consumer.rs", consumer_source),
                ("src/other.rs", other_source),
            ]);
            let error = preflight_crate_sources_with_cxx_namespace(&units, Some("rrr"))
                .expect_err(label);
            assert!(
                error.contains("namespace-root alias")
                    || error.contains("aliases of `crate`, `self`, or `super`"),
                "{label}: {error}"
            );
        }

        for (label, other_source) in [
            (
                "free function collision",
                "pub fn randgen_rand_raw() -> u64 { 1 }",
            ),
            (
                "type collision",
                "pub type randgen_rand_raw = u64;",
            ),
            (
                "const collision",
                "pub const randgen_rand_raw: u64 = 1;",
            ),
            (
                "static collision",
                "pub static randgen_rand_raw: u64 = 1;",
            ),
            ("module collision", "pub mod randgen_rand_raw {}"),
            (
                "ordinary alias binding collision",
                r#"
                    pub fn helper() -> u64 { 1 }
                    use self::helper as randgen_rand_raw;
                    pub fn call() -> u64 { randgen_rand_raw() }
                "#,
            ),
            (
                "root glob cannot prove disjointness",
                r#"
                    mod local { pub fn helper() -> u64 { 1 } }
                    use local::*;
                    pub fn call() -> u64 { helper() }
                "#,
            ),
            (
                "foreign function collision",
                r#"
                    unsafe extern "C" {
                        pub fn randgen_rand_raw() -> u64;
                    }
                "#,
            ),
        ] {
            assert_rustc_valid(other_source, label);
            let units = crate_units(&[
                (
                    "src/lib.rs",
                    "pub mod rand; pub mod consumer; pub mod other;",
                ),
                ("src/rand.rs", provider),
                ("src/consumer.rs", consumer),
                ("src/other.rs", other_source),
            ]);
            let error = preflight_crate_sources_with_cxx_namespace(&units, Some("rrr"))
                .expect_err(label);
            assert!(
                error.contains("collides with a flat sibling leaf")
                    || error.contains("cannot be proven disjoint"),
                "{label}: {error}"
            );
        }

        let escaped_provider = "pub fn static_() -> u64 { 0 }";
        let escaped_consumer = r#"
            #[cfg_attr(any(), cpp_import_namespace(rrr))]
            use crate::rand::static_;
            pub fn draw() -> u64 { static_() }
        "#;
        for (label, other_source) in [
            (
                "raw item exact C++ spelling collision",
                "pub fn r#static() -> u64 { 1 }",
            ),
            (
                "raw use binding exact C++ spelling collision",
                r#"
                    pub fn helper() -> u64 { 1 }
                    use self::helper as r#static;
                    pub fn call() -> u64 { r#static() }
                "#,
            ),
        ] {
            assert_rustc_valid(other_source, label);
            let units = crate_units(&[
                (
                    "src/lib.rs",
                    "pub mod rand; pub mod consumer; pub mod other;",
                ),
                ("src/rand.rs", escaped_provider),
                ("src/consumer.rs", escaped_consumer),
                ("src/other.rs", other_source),
            ]);
            let error = preflight_crate_sources_with_cxx_namespace(&units, Some("rrr"))
                .expect_err(label);
            assert!(
                error.contains("collides with a flat sibling leaf"),
                "{label}: {error}"
            );
        }

        for (label, provider_source) in [
            (
                "raw provider item exact C++ spelling collision",
                r#"
                    pub fn static_() -> u64 { 0 }
                    pub fn r#static() -> u64 { 1 }
                "#,
            ),
            (
                "raw provider use binding exact C++ spelling collision",
                r#"
                    pub fn static_() -> u64 { 0 }
                    mod helper { pub fn value() -> u64 { 1 } }
                    use helper::value as r#static;
                    pub fn other() -> u64 { r#static() }
                "#,
            ),
            (
                "provider block-local generic struct exact C++ spelling collision",
                r#"
                    pub fn static_() -> u64 { 0 }
                    pub fn other() {
                        struct r#static<T>(T);
                        let _ = r#static(1u8);
                    }
                "#,
            ),
            (
                "provider block-local generic type exact C++ spelling collision",
                r#"
                    pub fn static_() -> u64 { 0 }
                    pub fn other() {
                        type r#static<T> = Option<T>;
                        let _: r#static<u8> = Some(1);
                    }
                "#,
            ),
            (
                "provider block-local companion static exact C++ spelling collision",
                r#"
                    pub fn static_() -> u64 { 0 }
                    pub fn other() -> u64 {
                        static r#static: u64 = 4;
                        struct Guard<T>(T);
                        impl<T> Guard<T> {
                            fn get(&self) -> u64 { r#static }
                        }
                        Guard(1u8).get()
                    }
                "#,
            ),
        ] {
            assert_rustc_valid(provider_source, label);
            let units = crate_units(&[
                ("src/lib.rs", "pub mod rand; pub mod consumer;"),
                ("src/rand.rs", provider_source),
                ("src/consumer.rs", escaped_consumer),
            ]);
            let error = preflight_crate_sources_with_cxx_namespace(&units, Some("rrr"))
                .expect_err(label);
            assert!(
                error.contains("collides with a flat sibling leaf"),
                "{label}: {error}"
            );
        }

        for (label, other_source) in [
            (
                "generic block-local struct is namespace-hoisted or unsupported",
                r#"
                    pub fn local() {
                        struct r#static<T>(T);
                        let _ = r#static(1u8);
                    }
                "#,
            ),
            (
                "generic block-local type alias is namespace-hoisted or unsupported",
                r#"
                    pub fn local() {
                        type r#static<T> = Option<T>;
                        let _: r#static<u8> = Some(1);
                    }
                "#,
            ),
            (
                "impl-forced block-local enum is namespace-hoisted",
                r#"
                    pub fn local() {
                        enum r#static { Value }
                        impl r#static {
                            fn make<T>(_: T) -> Self { Self::Value }
                        }
                        let _ = r#static::make(1u8);
                    }
                "#,
            ),
            (
                "referenced block-local static accompanies hoisted type",
                r#"
                    pub fn local() -> u64 {
                        static r#static: u64 = 4;
                        struct Guard<T>(T);
                        impl<T> Guard<T> {
                            fn get(&self) -> u64 { r#static }
                        }
                        Guard(1u8).get()
                    }
                "#,
            ),
            (
                "method block-local generic struct is namespace-hoisted or unsupported",
                r#"
                    pub struct Holder;
                    impl Holder {
                        pub fn local() {
                            struct r#static<T>(T);
                            let _ = r#static(1u8);
                        }
                    }
                "#,
            ),
        ] {
            assert_rustc_valid(other_source, label);
            let units = crate_units(&[
                (
                    "src/lib.rs",
                    "pub mod rand; pub mod consumer; pub mod other;",
                ),
                ("src/rand.rs", escaped_provider),
                ("src/consumer.rs", escaped_consumer),
                ("src/other.rs", other_source),
            ]);
            let error = preflight_crate_sources_with_cxx_namespace(&units, Some("rrr"))
                .expect_err(label);
            assert!(
                error.contains("hoist") && error.contains("flat sibling leaf"),
                "{label}: {error}"
            );
        }

        let escaped_block_locals = r#"
            pub fn local_fn() -> u64 {
                fn r#static() -> u64 { 1 }
                r#static()
            }
            pub fn local_const() -> u64 {
                const r#static: u64 = 2;
                r#static
            }
            pub fn local_static() -> u64 {
                static r#static: u64 = 3;
                r#static
            }
            pub fn local_type() -> u64 {
                type r#static = u64;
                let value: r#static = 4;
                value
            }
            pub fn local_struct() -> u64 {
                struct r#static(u64);
                r#static(5).0
            }
            pub fn local_enum() -> u64 {
                enum r#static { Value(u64) }
                match r#static::Value(6) { r#static::Value(value) => value }
            }
            pub mod nested {
                pub fn local_generic() -> u64 {
                    struct r#static<T>(T);
                    r#static(7u64).0
                }
                pub fn local_generic_alias() -> u64 {
                    type r#static<T> = Option<T>;
                    let value: r#static<u64> = Some(8);
                    value.unwrap()
                }
            }
        "#;
        assert_rustc_valid(
            escaped_block_locals,
            "raw block-local items outside a flat consumer",
        );
        let escaped_local_units = crate_units(&[
            (
                "src/lib.rs",
                "pub mod rand; pub mod consumer; pub mod other;",
            ),
            ("src/rand.rs", escaped_provider),
            ("src/consumer.rs", escaped_consumer),
            ("src/other.rs", escaped_block_locals),
        ]);
        assert_eq!(
            preflight_crate_sources_with_cxx_namespace(
                &escaped_local_units,
                Some("rrr"),
            )
            .unwrap(),
            true
        );
    }

    #[test]
    fn crate_preflight_rejects_reviewer_sibling_call_and_value_before_output() {
        let units = crate_units(&[
            (
                "src/lib.rs",
                include_str!("../tests/fixtures/cpp_abi_sibling_crate/src/lib.rs"),
            ),
            (
                "src/api.rs",
                include_str!("../tests/fixtures/cpp_abi_sibling_crate/src/api.rs"),
            ),
            (
                "src/sibling.rs",
                include_str!("../tests/fixtures/cpp_abi_sibling_crate/src/sibling.rs"),
            ),
        ]);
        let error = preflight_crate_sources(&units).unwrap_err();
        assert!(error.contains("sibling-file reference"), "{error}");
        assert!(error.contains("src/sibling.rs"), "{error}");
    }

    #[test]
    fn crate_preflight_rejects_import_glob_alias_macro_and_attribute_surfaces() {
        let provider = r#"
            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
            pub fn adapted(v: Vec<u8>) {}
            pub struct Picker;
            impl Picker {
                #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
                pub fn choose() -> Vec<u8> { Vec::new() }
            }
        "#;
        for (label, sibling) in [
            ("import alias", "use crate::api::adapted as renamed;"),
            ("reexport", "pub use crate::api::adapted;"),
            ("glob", "use crate::api::*;"),
            (
                "macro",
                "macro_rules! invoke { () => { crate::api::adapted(Vec::new()) }; }",
            ),
            ("attribute", "#[allow(adapted)] pub fn ordinary() {}"),
        ] {
            let units = crate_units(&[
                ("src/lib.rs", "pub mod api; pub mod sibling;"),
                ("src/api.rs", provider),
                ("src/sibling.rs", sibling),
            ]);
            assert!(
                preflight_crate_sources(&units).is_err(),
                "accepted cross-unit {label}"
            );
        }

        let alias_provider = r#"
            #[cfg_attr(any(), cpp_abi_alias(std_vector))]
            pub type Weights = Vec<f64>;
            pub struct Picker;
            impl Picker {
                #[cfg_attr(any(), cpp_abi(param(v, const_ref(Weights))))]
                pub fn choose(v: &[f64]) {}
            }
        "#;
        let alias_units = crate_units(&[
            ("src/lib.rs", "pub mod api; pub mod sibling;"),
            ("src/api.rs", alias_provider),
            ("src/sibling.rs", "pub type Copy = crate::api::Weights;"),
        ]);
        assert!(preflight_crate_sources(&alias_units).is_err());
    }

    #[test]
    fn crate_preflight_allows_only_exact_unqualified_assert_forms() {
        let provider = r#"
            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
            pub fn adapted(v: Vec<u8>) -> bool { !v.is_empty() }
        "#;
        let valid_source = format!(
            "{provider}\npub fn checked(value: usize) -> usize {{ assert!(value < 8); value }}"
        );
        assert_rustc_valid(&valid_source, "single-expression assert with adapter");
        assert!(preflight_crate_sources(&crate_units(&[("src/lib.rs", &valid_source)])).unwrap());

        for (label, message, emitted) in [
            (
                "cooked literal-message assert with adapter",
                r#""cooked message""#,
                r#"throw std::logic_error("cooked message");"#,
            ),
            (
                "raw literal-message assert with adapter",
                r##"r#"raw message: adapted cpp_abi assert!(matches!())"#"##,
                r#"throw std::logic_error("raw message: adapted cpp_abi assert!(matches!())");"#,
            ),
            (
                "quoted and backslashed literal-message assert with adapter",
                r#""quote: \" and slash: \\ end""#,
                r#"throw std::logic_error("quote: \" and slash: \\ end");"#,
            ),
        ] {
            let literal_message = format!(
                "{provider}\npub fn checked(value: usize) -> usize {{ assert!(value < 8, {message}); value }}"
            );
            assert_rustc_valid(&literal_message, label);
            assert!(
                preflight_crate_sources(&crate_units(&[("src/lib.rs", &literal_message)]))
                    .unwrap()
            );
            let cpp = crate::transpile::transpile(&literal_message, Some("literal_assert"))
                .unwrap_or_else(|error| panic!("{label} failed to transpile: {error}"));
            assert!(
                cpp.contains(emitted),
                "{label} changed message bytes:\n{cpp}"
            );
        }

        for (label, invocation) in [
            ("arbitrary macro", "assert_eq!(1, 1);"),
            (
                "assert implicit capture",
                "let value = 1; assert!(true, \"message {value}\");",
            ),
            (
                "assert positional format",
                "let value = 1; assert!(true, \"message {}\", value);",
            ),
            (
                "assert named format",
                "let value = 1; assert!(true, \"message {shown}\", shown = value);",
            ),
            (
                "assert escaped braces",
                "assert!(true, \"message {{literal}}\");",
            ),
            (
                "assert raw-string braces",
                "let value = 1; assert!(true, r#\"message {value}\"#);",
            ),
            ("assert NUL", "assert!(true, \"message\\0tail\");"),
            (
                "assert control followed by hex digit",
                r#"assert!(true, "message \u{1}A");"#,
            ),
            ("assert newline", "assert!(true, \"line\\nbreak\");"),
            ("assert tab", "assert!(true, \"column\\ttwo\");"),
            ("assert Unicode", "assert!(true, \"café\");"),
            ("assert trailing comma", "assert!(true,);"),
            (
                "assert literal message trailing comma",
                "assert!(true, \"message\",);",
            ),
            ("qualified assert", "core::assert!(true, \"message\");"),
            ("brace-delimited assert", "assert! { true };"),
            ("bracket-delimited assert", "assert![true];"),
            (
                "nested opaque macro",
                "assert!(matches!(Some(1), Some(_)), \"message\");",
            ),
        ] {
            let source = format!("{provider}\npub fn checked() {{ {invocation} }}");
            assert_rustc_valid(&source, label);
            let error =
                preflight_crate_sources(&crate_units(&[("src/lib.rs", &source)])).expect_err(label);
            assert!(error.contains("opaque macro"), "{label}: {error}");
        }

        let external = crate_units(&[
            ("src/lib.rs", "pub mod api; pub mod sibling;"),
            ("src/api.rs", provider),
            (
                "src/sibling.rs",
                "pub fn bad() { assert!(crate::api::adapted(Vec::new()), \"message\"); }",
            ),
        ]);
        let error = preflight_crate_sources(&external).unwrap_err();
        assert!(error.contains("sibling-file reference"), "{error}");
    }

    #[test]
    fn admitted_assert_literal_preserves_quote_and_backslash_bytes_in_clang_runtime() {
        let compiler = ["clang++", "clang++-22", "clang++-21"]
            .into_iter()
            .find(|candidate| {
                std::process::Command::new(candidate)
                    .arg("--version")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .is_ok()
            });
        let Some(compiler) = compiler else {
            eprintln!("skipping assert literal runtime proof: no clang++ in PATH");
            return;
        };
        let source = r##"
            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
            pub fn adapted(v: Vec<u8>) { let _ = v; }

            pub fn checked() {
                assert!(false, r#"quote: " and slash: \ end"#);
            }
        "##;
        assert_rustc_valid(source, "quoted and backslashed assert literal");
        assert!(preflight_crate_sources(&crate_units(&[("src/lib.rs", source)])).unwrap());

        let emission_source = r##"
            pub fn checked() {
                assert!(false, r#"quote: " and slash: \ end"#);
            }
        "##;
        let mut cpp = crate::transpile::transpile(emission_source, None).unwrap();
        assert!(
            cpp.contains(r#"throw std::logic_error("quote: \" and slash: \\ end");"#),
            "assert message was not escaped exactly:\n{cpp}"
        );
        cpp.push_str(
            r#"
int main() {
    try {
        checked();
    } catch (const std::logic_error& error) {
        return std::string_view(error.what()) == R"(quote: " and slash: \ end)" ? 0 : 2;
    }
    return 3;
}
"#,
        );

        let temp = tempfile::tempdir().unwrap();
        let cpp_path = temp.path().join("assert_literal.cpp");
        let binary_path = temp.path().join("assert_literal");
        std::fs::write(&cpp_path, cpp).unwrap();
        let include_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("include");
        let compile = std::process::Command::new(compiler)
            .arg("-std=c++23")
            .arg("-I")
            .arg(include_dir)
            .arg(&cpp_path)
            .arg("-o")
            .arg(&binary_path)
            .output()
            .unwrap();
        assert!(
            compile.status.success(),
            "assert literal C++ compile failed:\n{}",
            String::from_utf8_lossy(&compile.stderr)
        );
        let run = std::process::Command::new(binary_path).output().unwrap();
        assert!(
            run.status.success(),
            "assert literal runtime byte check failed with {:?}:\n{}",
            run.status.code(),
            String::from_utf8_lossy(&run.stderr)
        );
    }

    #[test]
    fn standalone_assert_recursively_rejects_nested_macro_surfaces() {
        let provider = r#"
            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
            pub fn adapted(v: Vec<u8>) { let _ = v; }
        "#;
        let ordinary = format!(
            r#"
                {provider}
                pub fn checked(value: usize) {{
                    assert!({{ let doubled = value * 2; doubled < 16 && value != usize::MAX }});
                }}
            "#
        );
        assert_rustc_valid(&ordinary, "macro-free assert expression");
        let file = syn::parse_str(&ordinary).unwrap();
        assert!(lower(&file).unwrap().is_some());
        assert!(preflight_crate_sources(&crate_units(&[("src/lib.rs", &ordinary)])).unwrap());

        for (label, helpers, invocation) in [
            (
                "one nested macro",
                "macro_rules! id_bool { ($e:expr) => { $e }; }",
                "assert!(id_bool!(false));",
            ),
            (
                "nested nested macros",
                "macro_rules! id_bool { ($e:expr) => { $e }; }",
                "assert!(id_bool!(id_bool!(false)));",
            ),
            (
                "assert inside assert",
                "",
                "assert!({ assert!(true); true });",
            ),
        ] {
            let source = format!("{helpers}\n{provider}\npub fn checked() {{ {invocation} }}");
            assert_rustc_valid(&source, label);
            let file = syn::parse_str(&source).unwrap();
            let error = lower(&file).expect_err(label);
            assert!(
                error.contains("nested opaque macro")
                    && error.contains("assert!(EXPR[, \"literal\"])"),
                "{label}: {error}"
            );
            let error =
                preflight_crate_sources(&crate_units(&[("src/lib.rs", &source)])).expect_err(label);
            assert!(error.contains("opaque macro"), "{label}: {error}");
        }
    }

    #[test]
    fn standalone_and_crate_preflight_reserve_assert_macro_bindings() {
        let provider = r#"
            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
            pub fn adapted(v: Vec<u8>) {}
        "#;
        for (label, binding) in [
            ("name", "use core::assert;"),
            ("rename", "use core::stringify as assert;"),
            ("group", "use core::{stringify as assert};"),
            ("raw rename", "use core::stringify as r#assert;"),
            ("public rename", "pub use core::stringify as assert;"),
            (
                "block-local rename",
                "pub fn scoped() { use core::stringify as assert; }",
            ),
        ] {
            let source = format!("{provider}\n{binding}");
            assert_rustc_valid(&source, label);
            let file = syn::parse_str(&source).unwrap();
            let error = lower(&file).expect_err(label);
            assert!(error.contains("binding `assert`"), "{label}: {error}");
            let error =
                preflight_crate_sources(&crate_units(&[("src/lib.rs", &source)])).expect_err(label);
            assert!(error.contains("binding `assert`"), "{label}: {error}");
        }

        for (label, source) in [
            (
                "broad macro_use",
                format!("#[macro_use]\nextern crate core;\n{provider}"),
            ),
            (
                "selective macro_use",
                format!("#[macro_use(assert)]\nextern crate core;\n{provider}"),
            ),
        ] {
            assert_rustc_valid(&source, label);
            let file = syn::parse_str(&source).unwrap();
            let error = lower(&file).expect_err(label);
            assert!(error.contains("#[macro_use]"), "{label}: {error}");
            let error =
                preflight_crate_sources(&crate_units(&[("src/lib.rs", &source)])).expect_err(label);
            assert!(error.contains("#[macro_use]"), "{label}: {error}");
        }
    }

    #[test]
    fn standalone_and_crate_preflight_reject_assert_macro_declarations() {
        let provider = r#"
            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
            pub fn adapted(v: Vec<u8>) {}
        "#;
        for (label, declaration) in [
            (
                "root macro_rules",
                r#"
                    macro_rules! assert { ($e:expr) => {{ let _ = $e; }} }
                    pub fn checked() { assert!(false); }
                "#,
            ),
            (
                "raw root macro_rules",
                r#"
                    macro_rules! r#assert { ($e:expr) => {{ let _ = $e; }} }
                    pub fn checked() { r#assert!(false); }
                "#,
            ),
            (
                "nested module macro_rules",
                r#"
                    pub mod nested {
                        macro_rules! assert { ($e:expr) => {{ let _ = $e; }} }
                        pub fn checked() { assert!(false); }
                    }
                "#,
            ),
            (
                "block-local macro_rules",
                r#"
                    pub fn checked() {
                        macro_rules! assert { ($e:expr) => {{ let _ = $e; }} }
                        assert!(false);
                    }
                "#,
            ),
            (
                "macro token assembly",
                r#"
                    macro_rules! define {
                        ($name:ident) => {
                            macro_rules! $name { ($e:expr) => {{ let _ = $e; }} }
                        };
                    }
                    define!(assert);
                "#,
            ),
        ] {
            let source = format!("{declaration}\n{provider}");
            assert_rustc_valid(&source, label);
            let file = syn::parse_str(&source).unwrap();
            let error = lower(&file).expect_err(label);
            assert!(error.contains("macro"), "{label}: {error}");
            if label != "macro token assembly" {
                assert!(error.contains("assert"), "{label}: {error}");
            }
            let error =
                preflight_crate_sources(&crate_units(&[("src/lib.rs", &source)])).expect_err(label);
            assert!(error.contains("macro"), "{label}: {error}");
        }

        let sibling_units = crate_units(&[
            ("src/lib.rs", "pub mod api; pub mod sibling;"),
            ("src/api.rs", provider),
            (
                "src/sibling.rs",
                r#"
                    pub mod nested {
                        macro_rules! r#assert { ($e:expr) => {{ let _ = $e; }} }
                        pub fn checked() { r#assert!(false); }
                    }
                "#,
            ),
        ]);
        let error = preflight_crate_sources(&sibling_units).unwrap_err();
        assert!(
            error.contains("macro") && error.contains("assert"),
            "{error}"
        );

        // `decl_macro` remains unstable on the supported compiler, but syn
        // preserves it as Verbatim. Keep that parser-only surface fail closed
        // so stabilization cannot silently reintroduce an assert shadow.
        let macro_two = format!("pub macro assert($e:expr) {{ {{ let _ = $e; }} }}\n{provider}");
        let file = syn::parse_str(&macro_two).expect("syn accepts decl_macro as Verbatim");
        let error = lower(&file).unwrap_err();
        assert!(
            error.contains("macro") && error.contains("assert"),
            "{error}"
        );
    }

    #[test]
    fn standalone_and_crate_preflight_reject_unresolved_crate_namespace_aliases() {
        let provider = r#"
            #[cfg_attr(any(), cpp_abi(param(bytes, std_string_bytes), returns(std_string_bytes)))]
            pub fn adapted(bytes: Vec<u8>) -> Vec<u8> { bytes }
            #[cfg_attr(any(), cpp_abi_alias(std_vector))]
            pub type Weights = Vec<f64>;
            pub struct Picker;
            impl Picker {
                #[cfg_attr(any(), cpp_abi(param(weights, const_ref(Weights))))]
                pub fn choose(weights: &[f64]) -> u32 { weights.len() as u32 }
            }
        "#;
        for (label, binding) in [
            ("extern self alias", "extern crate self as this_crate;"),
            ("extern core binding", "extern crate core;"),
            ("crate alias", "use crate as this_crate;"),
            ("raw crate alias", "use crate as r#this_crate;"),
            ("grouped crate alias", "use crate::{self as this_crate};"),
        ] {
            let source = format!("{binding}\n{provider}");
            assert_rustc_valid(&source, label);
            let file = syn::parse_str(&source).unwrap();
            let error = lower(&file).expect_err(label);
            assert!(
                error.contains("unsupported") || error.contains("rejects"),
                "{label}: {error}"
            );
            let error =
                preflight_crate_sources(&crate_units(&[("src/lib.rs", &source)])).expect_err(label);
            assert!(
                error.contains("extern crate") || error.contains("aliases"),
                "{label}: {error}"
            );
        }

        let provider_file = r#"
            #[cfg_attr(any(), cpp_abi(param(bytes, std_string_bytes), returns(std_string_bytes)))]
            pub fn adapted(bytes: Vec<u8>) -> Vec<u8> { bytes }
            #[cfg_attr(any(), cpp_abi_alias(std_vector))]
            pub type Weights = Vec<f64>;
            pub struct Picker;
            impl Picker {
                #[cfg_attr(any(), cpp_abi(param(weights, const_ref(Weights))))]
                pub fn choose(weights: &[f64]) -> u32 { weights.len() as u32 }
            }
        "#;
        for (label, sibling) in [
            (
                "extern self path",
                "extern crate self as this_crate; pub fn call(v: Vec<u8>) -> Vec<u8> { this_crate::api::adapted(v) }",
            ),
            (
                "extern self function value",
                "extern crate self as this_crate; pub fn value() { let _f = this_crate::api::adapted; }",
            ),
            (
                "extern self type alias",
                "extern crate self as this_crate; pub type Copy = this_crate::api::Weights;",
            ),
            (
                "use crate path",
                "use crate as this_crate; pub fn call(v: Vec<u8>) -> Vec<u8> { this_crate::api::adapted(v) }",
            ),
            (
                "use crate function value",
                "use crate as this_crate; pub fn value() { let _f = this_crate::api::adapted; }",
            ),
            (
                "grouped crate path",
                "use crate::{self as grouped_alias}; pub fn call(v: Vec<u8>) -> Vec<u8> { grouped_alias::api::adapted(v) }",
            ),
            (
                "use super path",
                "use super as this_crate; pub fn call(v: Vec<u8>) -> Vec<u8> { this_crate::api::adapted(v) }",
            ),
            (
                "block raw crate alias",
                "pub fn call(v: Vec<u8>) -> Vec<u8> { use crate as r#this_crate; r#this_crate::api::adapted(v) }",
            ),
        ] {
            let units = crate_units(&[
                ("src/lib.rs", "pub mod api; pub mod sibling;"),
                ("src/api.rs", provider_file),
                ("src/sibling.rs", sibling),
            ]);
            let error = preflight_crate_sources(&units).expect_err(label);
            assert!(error.contains("cpp_abi"), "{label}: {error}");
        }

        let ancestor_alias = crate_units(&[
            ("src/lib.rs", "pub mod outer; pub mod sibling;"),
            ("src/outer/mod.rs", "pub mod api;"),
            ("src/outer/api.rs", provider_file),
            (
                "src/sibling.rs",
                "use crate::outer as namespace; pub fn call(v: Vec<u8>) -> Vec<u8> { namespace::api::adapted(v) }",
            ),
        ]);
        let error = preflight_crate_sources(&ancestor_alias).expect_err("provider ancestor alias");
        assert!(error.contains("adapted sibling"), "{error}");

        let root_alias = crate_units(&[
            (
                "src/lib.rs",
                "pub mod api; pub mod sibling; use self as root_alias;",
            ),
            ("src/api.rs", provider_file),
            (
                "src/sibling.rs",
                "pub fn call(v: Vec<u8>) -> Vec<u8> { crate::root_alias::api::adapted(v) }",
            ),
        ]);
        let error = preflight_crate_sources(&root_alias).expect_err("root alias re-export");
        assert!(error.contains("aliases"), "{error}");
    }

    #[test]
    fn crate_preflight_rejects_private_restricted_cfg_path_missing_and_duplicate_ancestors() {
        let provider = r#"
            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
            pub fn adapted(v: Vec<u8>) {}
        "#;
        for (label, root) in [
            ("private", "mod api;"),
            ("restricted", "pub(crate) mod api;"),
            ("cfg", "#[cfg(target_os = \"linux\")] pub mod api;"),
            ("path", "#[path = \"api.rs\"] pub mod api;"),
            ("missing", ""),
        ] {
            let units = crate_units(&[("src/lib.rs", root), ("src/api.rs", provider)]);
            assert!(
                preflight_crate_sources(&units).is_err(),
                "accepted {label} provider ancestor"
            );
        }

        let duplicate = crate_units(&[
            ("src/lib.rs", "pub mod api;"),
            ("src/api.rs", provider),
            ("src/api/mod.rs", "pub fn ordinary() {}"),
        ]);
        assert!(preflight_crate_sources(&duplicate).is_err());
    }

    #[test]
    fn crate_preflight_rejects_presence_attrs_on_every_source_and_incomplete_graphs() {
        let provider = r#"
            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
            pub fn adapted(v: Vec<u8>) {}
        "#;
        for (label, units) in [
            (
                "configured crate root",
                crate_units(&[
                    ("src/lib.rs", "#![cfg(any())] pub mod api;"),
                    ("src/api.rs", provider),
                ]),
            ),
            (
                "configured provider file",
                crate_units(&[
                    ("src/lib.rs", "pub mod api;"),
                    ("src/api.rs", &format!("#![cfg(any())] {provider}")),
                ]),
            ),
            (
                "configured ordinary sibling file",
                crate_units(&[
                    ("src/lib.rs", "pub mod api; pub mod sibling;"),
                    ("src/api.rs", provider),
                    ("src/sibling.rs", "#![cfg(any())] pub fn ordinary() {}"),
                ]),
            ),
            (
                "missing ordinary external module",
                crate_units(&[
                    ("src/lib.rs", "pub mod api; pub mod absent;"),
                    ("src/api.rs", provider),
                ]),
            ),
            (
                "unattached ordinary physical source",
                crate_units(&[
                    ("src/lib.rs", "pub mod api;"),
                    ("src/api.rs", provider),
                    ("src/orphan.rs", "pub fn ordinary() {}"),
                ]),
            ),
        ] {
            let error = preflight_crate_sources(&units).expect_err(&format!("accepted {label}"));
            assert!(error.contains("cpp_abi"), "{label}: {error}");
        }
    }

    #[test]
    fn crate_preflight_scopes_sibling_bindings_but_rejects_unbound_initializers() {
        let provider = r#"
            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
            pub fn adapted(v: Vec<u8>) {}
            pub struct Picker;
            impl Picker {
                #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
                pub fn choose() -> Vec<u8> { Vec::new() }
            }
        "#;
        let scoped = crate_units(&[
            ("src/lib.rs", "pub mod api; pub mod sibling;"),
            ("src/api.rs", provider),
            (
                "src/sibling.rs",
                r#"
                    pub fn parameter(adapted: usize) -> usize { adapted }
                    pub fn local() {
                        let adapted = 1usize;
                        let _ = adapted;
                    }
                    pub fn closure() {
                        let f = |adapted: usize| adapted + 1;
                        let _ = f(1);
                    }
                    pub fn matched(value: Option<usize>) {
                        match value {
                            Some(adapted) => { let _ = adapted; }
                            None => {}
                        }
                    }
                    pub fn looped(values: Vec<usize>) {
                        for adapted in values { let _ = adapted; }
                    }
                    pub struct Receiver;
                    impl Receiver { pub fn choose(&self) -> usize { 1 } }
                    pub struct Other;
                    impl Other { pub fn choose() -> usize { 2 } }
                    pub fn method_tail(receiver: &Receiver) -> usize {
                        receiver.choose()
                    }
                    pub fn local_associated() -> usize { Other::choose() }
                    pub fn if_bound(value: Option<usize>) -> usize {
                        if let Some(r#adapted) = value
                            && let Some(next) = Some(r#adapted + 1)
                            && next > 0
                        { assert!(r#adapted < next); next } else { 0 }
                    }
                    pub fn while_bound(mut values: Vec<Option<usize>>) -> usize {
                        let mut total = 0;
                        while let Some(Some(r#adapted)) = values.pop()
                            && r#adapted > 0
                        { total += r#adapted; }
                        total
                    }
                "#,
            ),
        ]);
        assert_eq!(preflight_crate_sources(&scoped).unwrap(), true);

        let unbound_initializer = crate_units(&[
            ("src/lib.rs", "pub mod api; pub mod sibling;"),
            ("src/api.rs", provider),
            (
                "src/sibling.rs",
                "pub fn bad() { let adapted = adapted; let _ = adapted; }",
            ),
        ]);
        let error = preflight_crate_sources(&unbound_initializer).unwrap_err();
        assert!(error.contains("unbound path"), "{error}");

        for (label, body) in [
            (
                "if-let initializer before binding",
                "pub fn bad() { if let Some(adapted) = adapted { let _ = adapted; } }",
            ),
            (
                "while-let initializer before binding",
                "pub fn bad() { while let Some(adapted) = adapted { let _ = adapted; } }",
            ),
            (
                "if-let binding does not leak into else",
                "pub fn bad(value: Option<usize>) { if let Some(adapted) = value { let _ = adapted; } else { adapted(Vec::new()); } }",
            ),
            (
                "external associated path",
                "pub fn bad() { let _ = crate::api::Picker::choose(); }",
            ),
        ] {
            let units = crate_units(&[
                ("src/lib.rs", "pub mod api; pub mod sibling;"),
                ("src/api.rs", provider),
                ("src/sibling.rs", body),
            ]);
            let error = preflight_crate_sources(&units).expect_err(label);
            assert!(error.contains("sibling-file reference"), "{label}: {error}");
        }
    }

    #[test]
    fn crate_preflight_rejects_shared_namespace_projected_collisions() {
        let free_collision = crate_units(&[
            ("src/lib.rs", "pub mod api; pub mod ordinary;"),
            (
                "src/api.rs",
                r#"
                    #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
                    pub fn r#class() -> Vec<u8> { Vec::new() }
                "#,
            ),
            ("src/ordinary.rs", "pub struct class_;"),
        ]);
        let error = preflight_crate_sources(&free_collision).unwrap_err();
        assert!(error.contains("identifier escaping"), "{error}");

        let module_collision = crate_units(&[
            ("src/lib.rs", "pub mod r#class; pub struct class_;"),
            (
                "src/class.rs",
                r#"
                    #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
                    pub fn adapted() -> Vec<u8> { Vec::new() }
                "#,
            ),
        ]);
        let error = preflight_crate_sources(&module_collision).unwrap_err();
        assert!(error.contains("identifier escaping"), "{error}");
    }

    #[test]
    fn crate_preflight_preserves_distinct_nested_cpp_scopes_and_member_carveouts() {
        let units = crate_units(&[
            ("src/lib.rs", "pub mod api;"),
            (
                "src/api.rs",
                r#"
                    pub mod left {
                        #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
                        pub fn r#class() -> Vec<u8> { Vec::new() }
                    }
                    pub mod right {
                        pub fn class_() {}
                    }
                    pub struct Owner;
                    impl Owner {
                        #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
                        pub fn pause() -> Vec<u8> { Vec::new() }
                        pub fn pause_() {}
                    }
                "#,
            ),
        ]);
        assert_eq!(preflight_crate_sources(&units).unwrap(), true);
    }

    #[test]
    fn crate_preflight_keeps_marker_free_macro_and_glob_on_exact_fast_path() {
        let units = crate_units(&[(
            "src/lib.rs",
            r#"
                    use ordinary::*;
                    macro_rules! ordinary { () => {} }
                    pub mod ordinary { pub fn value() {} }
                "#,
        )]);
        assert_eq!(preflight_crate_sources(&units).unwrap(), false);
    }

    #[test]
    fn crate_preflight_does_not_treat_an_alias_only_contract_as_marker_free() {
        let units = crate_units(&[(
            "src/lib.rs",
            r#"
                #[cfg_attr(any(), cpp_abi_alias(std_vector))]
                pub type Weights = Vec<f64>;
            "#,
        )]);
        let error = preflight_crate_sources(&units).unwrap_err();
        assert!(error.contains("Weights"), "{error}");
    }

    #[test]
    fn lowering_rejects_callable_names_in_non_marker_attributes() {
        let source = r#"
            #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
            pub fn adapted(v: Vec<u8>) {}
            #[allow(adapted)]
            pub fn ordinary() {}
        "#;
        assert_rustc_valid(source, "callable name in lint attribute");
        let file = syn::parse_str(source).unwrap();
        assert!(lower(&file).is_err());
    }

    #[test]
    fn lowering_rejects_unsupported_types_aliases_and_generated_name_collisions() {
        for source in [
            r#"
                #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                pub fn bytes(v: Vec<u8>, unsupported: String) {}
            "#,
            r#"
                #[cfg_attr(any(), cpp_abi_alias(std_vector))]
                pub type Values = Vec<f32>;
                pub struct Owner;
                impl Owner {
                    #[cfg_attr(any(), cpp_abi(param(v, const_ref(Values))))]
                    pub fn use_values(v: &[f32]) {}
                }
            "#,
            r#"
                pub fn rusty_cpp_abi_sem_bytes() {}
                #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                pub fn bytes(v: Vec<u8>) {}
            "#,
            r#"
                #[cfg_attr(any(), cpp_abi(param(rusty_cpp_abi_arg_0, std_string_bytes)))]
                pub fn bytes(rusty_cpp_abi_arg_0: Vec<u8>) {}
            "#,
            r#"
                pub mod rusty_cpp_abi_detail {}
                #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                pub fn bytes(v: Vec<u8>) {}
            "#,
        ] {
            let file = syn::parse_str(source).unwrap();
            assert!(
                lower(&file).is_err(),
                "accepted unsupported surface: {source}"
            );
        }
    }

    #[test]
    fn lowering_rejects_marked_aliases_in_semantic_types_and_expressions() {
        for source in [
            r#"
                #[cfg_attr(any(), cpp_abi_alias(std_vector))]
                pub type Weights = Vec<f64>;
                pub struct Picker;
                impl Picker {
                    #[cfg_attr(any(), cpp_abi(param(v, const_ref(Weights))))]
                    pub fn choose(v: &[f64]) {}
                }
                pub fn semantic(v: Weights) { let _ = v; }
            "#,
            r#"
                #[cfg_attr(any(), cpp_abi_alias(std_vector))]
                pub type Weights = Vec<f64>;
                pub struct Picker;
                impl Picker {
                    #[cfg_attr(any(), cpp_abi(param(v, const_ref(Weights))))]
                    pub fn choose(v: &[f64]) {}
                }
                pub fn semantic() { let _ = Weights::new(); }
            "#,
            r#"
                mod provider {
                    #[cfg_attr(any(), cpp_abi_alias(std_vector))]
                    pub type Weights = Vec<f64>;
                    pub struct Picker;
                    impl Picker {
                        #[cfg_attr(any(), cpp_abi(param(v, const_ref(Weights))))]
                        pub fn choose(v: &[f64]) {}
                    }
                }
                mod consumer {
                    pub fn semantic(v: super::provider::Weights) { let _ = v; }
                }
            "#,
        ] {
            let file = syn::parse_str(source).unwrap();
            assert!(
                lower(&file).is_err(),
                "accepted marked alias in semantic Rust: {source}"
            );
        }
    }

    #[test]
    fn lowering_rejects_impl_context_dependent_method_bodies() {
        for source in [
            r#"
                pub struct Owner;
                impl Owner {
                    #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
                    pub fn bytes() -> Vec<u8> { let _: Option<Self> = None; Vec::new() }
                }
            "#,
            r#"
                pub struct Owner;
                impl Owner {
                    const VALUE: u8 = 1;
                    #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
                    pub fn bytes() -> Vec<u8> { let _ = VALUE; Vec::new() }
                }
            "#,
        ] {
            let file = syn::parse_str(source).unwrap();
            assert!(
                lower(&file).is_err(),
                "accepted impl-dependent body: {source}"
            );
        }
    }

    #[test]
    fn inline_lowering_rewrites_ordered_calls_and_records_dependencies() {
        let provider = syn::parse_str::<syn::File>(
            r#"
                #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes), returns(std_string_bytes)))]
                pub fn zero_pad(v: Vec<u8>) -> Vec<u8> { v }
            "#,
        )
        .unwrap();
        let consumer = syn::parse_str::<syn::File>(
            "pub fn format(v: Vec<u8>) -> Vec<u8> { zero_pad(v) }",
        )
        .unwrap();
        let plan = prepare_inline_carrier(
            &[provider, consumer],
            &ExternalContractIndex::default(),
            "test",
        )
        .unwrap();
        assert_eq!(plan.adapted_blocks, BTreeSet::from([0, 1]));
        assert!(plan.blocks[0].dependencies.is_empty());
        assert_eq!(plan.blocks[1].dependencies, BTreeSet::from([0]));
        assert!(plan.blocks[0].emission.needs_string_adapter());
        assert!(!plan.blocks[1].emission.needs_string_adapter());
        let consumer = plan.blocks[1].lowered.to_token_stream().to_string();
        assert!(consumer.contains("rusty_cpp_abi_sem_test_zero_pad"), "{consumer}");
    }

    #[test]
    fn inline_lowering_rejects_backward_calls_and_function_values() {
        for consumer in [
            "pub fn format(v: Vec<u8>) -> Vec<u8> { zero_pad(v) }",
            "pub fn format() { let _f = zero_pad; }",
        ] {
            let first = syn::parse_str::<syn::File>(consumer).unwrap();
            let provider = syn::parse_str::<syn::File>(
                r#"
                    #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes), returns(std_string_bytes)))]
                    pub fn zero_pad(v: Vec<u8>) -> Vec<u8> { v }
                "#,
            )
            .unwrap();
            let error = prepare_inline_carrier(
                &[first, provider],
                &ExternalContractIndex::default(),
                "test",
            )
            .expect_err("backward/non-call use must fail");
            assert!(error.contains("zero_pad"), "{error}");
        }
    }

    #[test]
    fn inline_lowering_rejects_external_names_and_consumer_inner_cfg() {
        let external_use = syn::parse_str::<syn::File>(
            "pub fn use_external(v: Vec<u8>) -> Vec<u8> { external_adapter(v) }",
        )
        .unwrap();
        let error = prepare_inline_carrier(
            &[external_use],
            &ExternalContractIndex {
                values: BTreeSet::from([vec!["external_adapter".to_string()]]),
                ..ExternalContractIndex::default()
            },
            "test",
        )
        .expect_err("cross-carrier call must fail");
        assert!(
            error.contains("cross-carrier") || error.contains("adapted sibling"),
            "{error}"
        );

        let provider = syn::parse_str::<syn::File>(
            r#"
                #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                pub fn adapted(v: Vec<u8>) {}
            "#,
        )
        .unwrap();
        let consumer = syn::parse_str::<syn::File>(
            "#![cfg(any())]\npub fn ordinary() {}",
        )
        .unwrap();
        let error = prepare_inline_carrier(
            &[provider, consumer],
            &ExternalContractIndex::default(),
            "test",
        )
        .expect_err("consumer inner cfg must not disappear");
        assert!(error.contains("inline block 2 source"), "{error}");
    }

    #[test]
    fn inline_projected_census_rejects_cross_carrier_public_name_collisions() {
        let adapted_free = syn::parse_str::<syn::File>(
            r#"
                #[cfg_attr(any(), cpp_abi(param(v, std_string_bytes)))]
                pub fn foo(v: Vec<u8>) {}
            "#,
        )
        .unwrap();
        let ordinary_free = syn::parse_str::<syn::File>("pub fn foo(v: i32) -> i32 { v }")
            .unwrap();
        let error = validate_inline_projected_cpp_name_collisions(
            &["adapted.cpp".to_string(), "ordinary.cpp".to_string()],
            &[vec![adapted_free.clone()], vec![ordinary_free]],
        )
        .expect_err("return-only/overload public name must fail closed across modules");
        assert!(error.contains("foo") && error.contains("ordinary.cpp"), "{error}");

        let alias = syn::parse_str::<syn::File>(
            r#"
                #[cfg_attr(any(), cpp_abi_alias(std_vector))]
                pub type Weights = Vec<f64>;
                pub struct Picker;
                impl Picker {
                    #[cfg_attr(any(), cpp_abi(param(v, const_ref(Weights))))]
                    pub fn choose(v: &[f64]) {}
                }
            "#,
        )
        .unwrap();
        let ordinary_type = syn::parse_str::<syn::File>("pub struct Weights;").unwrap();
        let error = validate_inline_projected_cpp_name_collisions(
            &["alias.cpp".to_string(), "type.cpp".to_string()],
            &[vec![alias], vec![ordinary_type]],
        )
        .expect_err("adapted alias and ordinary type must not share a public C++ spelling");
        assert!(error.contains("Weights") && error.contains("type.cpp"), "{error}");

        let nested = syn::parse_str::<syn::File>(
            "pub mod local { pub fn foo(v: i32) -> i32 { v } }",
        )
        .unwrap();
        validate_inline_projected_cpp_name_collisions(
            &["adapted.cpp".to_string(), "nested.cpp".to_string()],
            &[vec![adapted_free], vec![nested]],
        )
        .expect("a distinct nested namespace does not collide with root facade foo");
    }

    #[test]
    fn unmarked_nested_source_stays_on_the_empty_fast_path() {
        let source = "mod nested { pub fn ordinary(v: Vec<u8>) -> Vec<u8> { v } }";
        let file = syn::parse_str(source).unwrap();
        assert!(lower(&file).unwrap().is_none());
        let cpp = crate::transpile::transpile(source, Some("ordinary")).unwrap();
        assert!(!cpp.contains("rusty_cpp_abi_"));
        assert!(!cpp.contains("#include <vector>"));
    }
}
